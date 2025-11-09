use crate::cli::Args;
use crate::curl_parser::{parse_curl_command, parse_curl_file, CurlCommand};
use crate::http_client::{ClientState, ConnectionPool};
use crate::stats::{create_shared_stats, RequestResult, SharedStats};
use crate::template::TemplateEngine;
use anyhow::Result;
use rand::Rng;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub async fn run_benchmark(args: Args) -> Result<()> {
    // Parse curl commands if provided
    let commands = if let Some(curl_cmd) = &args.parse_curl {
        vec![parse_curl_command(curl_cmd)?]
    } else if let Some(curl_file) = &args.parse_curl_file {
        parse_curl_file(curl_file)?
    } else if let Some(url) = &args.url {
        vec![create_command_from_args(&args, url.clone())]
    } else {
        anyhow::bail!("No URL or curl command provided");
    };

    // Setup template engine
    let mut template_engine = TemplateEngine::new();
    for var in &args.vars {
        if let Some(pos) = var.find('=') {
            let name = var[..pos].to_string();
            let definition = &var[pos + 1..];
            template_engine.add_variable(name, definition)?;
        }
    }
    let template_engine = Arc::new(template_engine);

    // Print test configuration
    let duration = args.parse_duration()?;
    let target_desc = if commands.len() == 1 { 
        commands[0].url.clone()
    } else { 
        format!("{} endpoints", commands.len()) 
    };
    println!("Running {}s test @ {}", duration.as_secs(), target_desc);
    println!("  {} threads and {} connections", args.threads, args.connections);

    // Create shared statistics
    let stats = create_shared_stats();

    // Run the benchmark（使用 block_in_place 因为 run_workers 现在是同步的）
    tokio::task::block_in_place(|| {
        run_workers(
            commands,
            args.connections,
            args.threads,
            duration,
            args.rate,
            args.parse_timeout()?,
            &args.load_strategy,
            stats.clone(),
            template_engine,
            args.http2,
        )
    })?;

    // Finish statistics collection
    {
        let mut stats_lock = stats.lock().unwrap();
        stats_lock.finish();
    }

    // Print results
    let stats_lock = stats.lock().unwrap();
    stats_lock.print_summary(args.latency);

    Ok(())
}

fn create_command_from_args(args: &Args, url: String) -> CurlCommand {
    let mut cmd = CurlCommand::new(url);
    cmd.method = args.method.clone();
    
    for header in &args.headers {
        if let Some(pos) = header.find(':') {
            let key = header[..pos].trim().to_string();
            let value = header[pos + 1..].trim().to_string();
            cmd.headers.insert(key, value);
        }
    }
    
    if let Some(content_type) = &args.content_type {
        cmd.headers.insert("Content-Type".to_string(), content_type.clone());
    }
    
    cmd.body = args.data.clone();
    
    cmd
}

fn run_workers(
    commands: Vec<CurlCommand>,
    connections: usize,
    threads: usize,
    duration: Duration,
    rate: u32,
    timeout: Duration,
    load_strategy: &str,
    stats: SharedStats,
    template_engine: Arc<TemplateEngine>,
    enable_http2: bool,
) -> Result<()> {
    let commands = Arc::new(commands);
    let load_strategy = load_strategy.to_string();
    let end_time = Instant::now() + duration;

    // 参考 oha：使用物理 CPU 核心数
    let num_physical_cpus = num_cpus::get_physical();
    let actual_threads = if threads == 0 {
        num_physical_cpus
    } else {
        threads.min(num_physical_cpus * 2)
    };

    // 计算每个线程的连接数
    let connections_per_thread = (connections / actual_threads).max(1);
    
    // 创建连接池
    let pool_size = actual_threads.min(20);
    let connections_per_client = (connections / pool_size).max(1);
    let pool = Arc::new(
        ConnectionPool::new(pool_size, timeout, connections_per_client, enable_http2)
            .expect("Failed to create connection pool")
    );

    // 使用 LocalSet 架构：每个物理线程独立运行
    let handles: Vec<_> = (0..actual_threads)
        .map(|_| {
            let commands = commands.clone();
            let stats = stats.clone();
            let load_strategy = load_strategy.clone();
            let template_engine = template_engine.clone();
            let pool = pool.clone();
            
            // 为每个线程创建独立的 tokio 运行时
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                
                let local = tokio::task::LocalSet::new();
                
                // 在 LocalSet 中创建多个任务（每个线程处理多个连接）
                for _ in 0..connections_per_thread {
                    let commands = commands.clone();
                    let stats = stats.clone();
                    let load_strategy = load_strategy.clone();
                    let template_engine = template_engine.clone();
                    let client = pool.get_client().clone();
                    
                    local.spawn_local(async move {
                        // 创建客户端状态用于连接复用
                        let mut client_state = ClientState::new();
                        let mut request_count = 0u64;

                        while Instant::now() < end_time {
                            // Select command based on load strategy
                            let cmd = match load_strategy.as_str() {
                                "round-robin" => {
                                    &commands[request_count as usize % commands.len()]
                                }
                                _ => {
                                    // random (default)
                                    let idx = rand::thread_rng().gen_range(0..commands.len());
                                    &commands[idx]
                                }
                            };

                            // Apply template processing
                            let url = template_engine.process(&cmd.url);
                            let body = cmd.body.as_ref().map(|b| template_engine.process(b));

                            // Make request
                            let start = Instant::now();
                            let result = client.request(&mut client_state, &cmd.method, &url, &cmd.headers, body.as_deref()).await;
                            let duration = start.elapsed();

                            // Record result
                            let request_result = RequestResult {
                                duration,
                                status_code: result.as_ref().ok().and_then(|r| Some(r.0)),
                                bytes_read: result.as_ref().ok().map(|r| r.1).unwrap_or(0),
                                error: result.err().map(|e| e.to_string()),
                                endpoint: if commands.len() > 1 { Some(cmd.url.clone()) } else { None },
                            };

                            stats.lock().unwrap().record(request_result);
                            request_count += 1;

                            // Rate limiting
                            if rate > 0 {
                                let delay = Duration::from_secs_f64(1.0 / rate as f64);
                                tokio::time::sleep(delay).await;
                            }
                        }
                    });
                }
                
                // 运行 LocalSet
                rt.block_on(local);
            })
        })
        .collect();

    // 等待所有线程完成
    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

// make_request 函数已被移除，现在直接使用 HttpClient::request 方法

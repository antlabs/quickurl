use crate::cli::Args;
use crate::curl_parser::{parse_curl_command, parse_curl_file, CurlCommand};
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

    // Run the benchmark
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
    ).await?;

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

async fn run_workers(
    commands: Vec<CurlCommand>,
    _connections: usize,
    threads: usize,
    duration: Duration,
    rate: u32,
    timeout: Duration,
    load_strategy: &str,
    stats: SharedStats,
    template_engine: Arc<TemplateEngine>,
) -> Result<()> {
    let commands = Arc::new(commands);
    let load_strategy = load_strategy.to_string();
    let end_time = Instant::now() + duration;

    let mut handles = Vec::new();

    for _ in 0..threads {
        let commands = commands.clone();
        let stats = stats.clone();
        let load_strategy = load_strategy.clone();
        let template_engine = template_engine.clone();
        
        let handle = tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap();

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
                let result = make_request(&client, &url, &cmd.method, &cmd.headers, body.as_deref()).await;
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
                    sleep(delay).await;
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

async fn make_request(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    headers: &std::collections::HashMap<String, String>,
    body: Option<&str>,
) -> Result<(u16, usize)> {
    let mut request = match method {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "HEAD" => client.head(url),
        "PATCH" => client.patch(url),
        _ => client.get(url),
    };

    // Add headers
    for (key, value) in headers {
        request = request.header(key, value);
    }

    // Add body
    if let Some(body_data) = body {
        request = request.body(body_data.to_string());
    }

    // Send request
    let response = request.send().await?;
    let status = response.status().as_u16();
    let bytes = response.bytes().await?;
    
    Ok((status, bytes.len()))
}

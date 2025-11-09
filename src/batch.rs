use crate::cli::Args;
use crate::curl_parser::parse_curl_command;
use crate::engine::run_benchmark;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
struct BatchConfig {
    version: String,
    tests: Vec<TestConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TestConfig {
    name: String,
    curl: String,
    #[serde(default = "default_connections")]
    connections: usize,
    #[serde(default = "default_duration")]
    duration: String,
    #[serde(default = "default_threads")]
    threads: usize,
    #[serde(default)]
    rate: u32,
    #[serde(default = "default_timeout")]
    timeout: String,
    #[serde(default)]
    verbose: bool,
    #[serde(default)]
    use_nethttp: bool,
}

fn default_connections() -> usize { 10 }
fn default_duration() -> String { "10s".to_string() }
fn default_threads() -> usize { 2 }
fn default_timeout() -> String { "30s".to_string() }

#[derive(Debug)]
struct TestResult {
    name: String,
    duration: std::time::Duration,
    success: bool,
    error: Option<String>,
}

pub async fn run_batch_tests(args: Args) -> Result<()> {
    let config_path = args.batch_config.as_ref().unwrap();
    let config = load_config(config_path)?;

    println!("=== Batch Test Configuration ===");
    println!("Total Tests: {}", config.tests.len());
    println!("Concurrency: {}", if args.batch_sequential { 1 } else { args.batch_concurrency });
    println!();

    let start_time = Instant::now();
    let mut results = Vec::new();

    if args.batch_sequential {
        // Run tests sequentially
        for test in &config.tests {
            println!("Running test: {}", test.name);
            let result = run_single_test(test.clone()).await;
            results.push(result);
        }
    } else {
        // Run tests concurrently with limited concurrency
        use futures::stream::{self, StreamExt};
        
        let test_results: Vec<TestResult> = stream::iter(config.tests.clone())
            .map(|test| async move {
                println!("Running test: {}", test.name);
                run_single_test(test).await
            })
            .buffer_unordered(args.batch_concurrency)
            .collect()
            .await;
        
        results = test_results;
    }

    let total_duration = start_time.elapsed();

    // Print report
    print_report(&results, total_duration, &args.batch_report)?;

    Ok(())
}

async fn run_single_test(test: TestConfig) -> TestResult {
    let start = Instant::now();
    
    // Parse curl command
    let curl_cmd = match parse_curl_command(&test.curl) {
        Ok(cmd) => cmd,
        Err(e) => {
            return TestResult {
                name: test.name,
                duration: start.elapsed(),
                success: false,
                error: Some(format!("Failed to parse curl command: {}", e)),
            };
        }
    };

    // Create args for this test
    let args = Args {
        url: Some(curl_cmd.url.clone()),
        connections: test.connections,
        duration: test.duration.clone(),
        threads: test.threads,
        rate: test.rate,
        timeout: test.timeout.clone(),
        method: curl_cmd.method.clone(),
        headers: curl_cmd.headers.iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect(),
        data: curl_cmd.body.clone(),
        verbose: test.verbose,
        use_nethttp: test.use_nethttp,
        http2: false,  // 默认使用 HTTP/1.1
        latency: false,
        live_ui: false,
        parse_curl: None,
        parse_curl_file: None,
        load_strategy: "random".to_string(),
        content_type: None,
        mock_server: false,
        mock_port: 8080,
        mock_delay: None,
        mock_response: None,
        mock_status: 200,
        mock_config: None,
        batch_config: None,
        batch_concurrency: 3,
        batch_sequential: false,
        batch_report: "text".to_string(),
        vars: Vec::new(),
        help_templates: false,
    };

    // Run the benchmark
    match run_benchmark(args).await {
        Ok(_) => TestResult {
            name: test.name,
            duration: start.elapsed(),
            success: true,
            error: None,
        },
        Err(e) => TestResult {
            name: test.name,
            duration: start.elapsed(),
            success: false,
            error: Some(e.to_string()),
        },
    }
}

fn load_config(path: &Path) -> Result<BatchConfig> {
    let content = std::fs::read_to_string(path)?;
    
    // Try to parse as YAML first, then JSON
    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(serde_yaml::from_str(&content)?)
    }
}

fn print_report(results: &[TestResult], total_duration: std::time::Duration, format: &str) -> Result<()> {
    match format {
        "json" => print_json_report(results, total_duration),
        "csv" => print_csv_report(results, total_duration),
        _ => print_text_report(results, total_duration),
    }
}

fn print_text_report(results: &[TestResult], total_duration: std::time::Duration) -> Result<()> {
    println!("\n=== Batch Test Report ===\n");
    
    let success_count = results.iter().filter(|r| r.success).count();
    let success_rate = (success_count as f64 / results.len() as f64) * 100.0;
    
    println!("Total Tests: {}", results.len());
    println!("Success Rate: {:.2}%", success_rate);
    println!("Total Time: {:.2}s", total_duration.as_secs_f64());
    
    println!("\n=== Test Results ===\n");
    
    for (i, result) in results.iter().enumerate() {
        println!("{}. {}", i + 1, result.name);
        println!("   Duration: {:.2}s", result.duration.as_secs_f64());
        println!("   Status: {}", if result.success { "SUCCESS" } else { "FAILED" });
        if let Some(error) = &result.error {
            println!("   Error: {}", error);
        }
        println!();
    }
    
    Ok(())
}

fn print_json_report(results: &[TestResult], total_duration: std::time::Duration) -> Result<()> {
    #[derive(Serialize)]
    struct JsonReport {
        total_tests: usize,
        success_count: usize,
        success_rate: f64,
        total_duration_secs: f64,
        results: Vec<JsonTestResult>,
    }
    
    #[derive(Serialize)]
    struct JsonTestResult {
        name: String,
        duration_secs: f64,
        success: bool,
        error: Option<String>,
    }
    
    let success_count = results.iter().filter(|r| r.success).count();
    let success_rate = (success_count as f64 / results.len() as f64) * 100.0;
    
    let report = JsonReport {
        total_tests: results.len(),
        success_count,
        success_rate,
        total_duration_secs: total_duration.as_secs_f64(),
        results: results.iter().map(|r| JsonTestResult {
            name: r.name.clone(),
            duration_secs: r.duration.as_secs_f64(),
            success: r.success,
            error: r.error.clone(),
        }).collect(),
    };
    
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn print_csv_report(results: &[TestResult], _total_duration: std::time::Duration) -> Result<()> {
    println!("Name,Duration(s),Success,Error");
    
    for result in results {
        println!("{},{},{},{}",
            result.name,
            result.duration.as_secs_f64(),
            result.success,
            result.error.as_deref().unwrap_or("")
        );
    }
    
    Ok(())
}

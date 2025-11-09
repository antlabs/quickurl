use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "quickurl")]
#[command(about = "A modern, high-performance HTTP benchmarking tool", long_about = None)]
#[command(version)]
pub struct Args {
    /// Target URL to benchmark
    #[arg(value_name = "URL")]
    pub url: Option<String>,

    /// Number of HTTP connections to keep open
    #[arg(short = 'c', long = "connections", default_value = "10")]
    pub connections: usize,

    /// Duration of test (e.g., 10s, 5m, 1h)
    #[arg(short = 'd', long = "duration", default_value = "10s")]
    pub duration: String,

    /// Number of threads to use
    #[arg(short = 't', long = "threads", default_value = "2")]
    pub threads: usize,

    /// Work rate (requests/sec) 0=unlimited
    #[arg(short = 'R', long = "rate", default_value = "0")]
    pub rate: u32,

    /// Socket/request timeout
    #[arg(long = "timeout", default_value = "30s")]
    pub timeout: String,

    /// Parse curl command and use it for benchmarking
    #[arg(long = "parse-curl")]
    pub parse_curl: Option<String>,

    /// Parse multiple curl commands from file (one per line)
    #[arg(long = "parse-curl-file")]
    pub parse_curl_file: Option<PathBuf>,

    /// Load distribution strategy: random, round-robin
    #[arg(long = "load-strategy", default_value = "random")]
    pub load_strategy: String,

    /// HTTP method
    #[arg(short = 'X', long = "method", default_value = "GET")]
    pub method: String,

    /// HTTP header to add to request (can be used multiple times)
    #[arg(short = 'H', long = "header")]
    pub headers: Vec<String>,

    /// HTTP request body
    #[arg(long = "data")]
    pub data: Option<String>,

    /// Content-Type header
    #[arg(long = "content-type")]
    pub content_type: Option<String>,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Print latency statistics
    #[arg(long = "latency")]
    pub latency: bool,

    /// Enable live terminal UI with real-time stats
    #[arg(long = "live-ui")]
    pub live_ui: bool,

    /// Force use standard library instead of optimized client
    #[arg(long = "use-nethttp")]
    pub use_nethttp: bool,

    /// Start mock HTTP server
    #[arg(long = "mock-server")]
    pub mock_server: bool,

    /// Mock server port
    #[arg(long = "mock-port", default_value = "8080")]
    pub mock_port: u16,

    /// Mock server delay
    #[arg(long = "mock-delay")]
    pub mock_delay: Option<String>,

    /// Mock server response
    #[arg(long = "mock-response")]
    pub mock_response: Option<String>,

    /// Mock server status code
    #[arg(long = "mock-status", default_value = "200")]
    pub mock_status: u16,

    /// Mock server configuration file
    #[arg(long = "mock-config")]
    pub mock_config: Option<PathBuf>,

    /// Path to batch configuration file (YAML/JSON)
    #[arg(long = "batch-config")]
    pub batch_config: Option<PathBuf>,

    /// Maximum concurrent batch tests
    #[arg(long = "batch-concurrency", default_value = "3")]
    pub batch_concurrency: usize,

    /// Run tests sequentially instead of concurrently
    #[arg(long = "batch-sequential")]
    pub batch_sequential: bool,

    /// Report format: text, csv, json
    #[arg(long = "batch-report", default_value = "text")]
    pub batch_report: String,

    /// Define custom template variables (e.g., --var user_id=random:1-1000)
    #[arg(long = "var")]
    pub vars: Vec<String>,

    /// Show help for template variables
    #[arg(long = "help-templates")]
    pub help_templates: bool,
}

impl Args {
    pub fn parse_duration(&self) -> anyhow::Result<std::time::Duration> {
        parse_duration_string(&self.duration)
    }

    pub fn parse_timeout(&self) -> anyhow::Result<std::time::Duration> {
        parse_duration_string(&self.timeout)
    }
}

fn parse_duration_string(s: &str) -> anyhow::Result<std::time::Duration> {
    let s = s.trim();
    
    if s.ends_with("ms") {
        let num: u64 = s[..s.len() - 2].parse()?;
        Ok(std::time::Duration::from_millis(num))
    } else if s.ends_with('s') {
        let num: u64 = s[..s.len() - 1].parse()?;
        Ok(std::time::Duration::from_secs(num))
    } else if s.ends_with('m') {
        let num: u64 = s[..s.len() - 1].parse()?;
        Ok(std::time::Duration::from_secs(num * 60))
    } else if s.ends_with('h') {
        let num: u64 = s[..s.len() - 1].parse()?;
        Ok(std::time::Duration::from_secs(num * 3600))
    } else {
        // Default to seconds
        let num: u64 = s.parse()?;
        Ok(std::time::Duration::from_secs(num))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration_string("10s").unwrap(), std::time::Duration::from_secs(10));
        assert_eq!(parse_duration_string("5m").unwrap(), std::time::Duration::from_secs(300));
        assert_eq!(parse_duration_string("1h").unwrap(), std::time::Duration::from_secs(3600));
        assert_eq!(parse_duration_string("100ms").unwrap(), std::time::Duration::from_millis(100));
    }
}

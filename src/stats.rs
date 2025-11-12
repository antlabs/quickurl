use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RequestResult {
    pub duration: Duration,
    pub status_code: Option<u16>,
    pub bytes_read: usize,
    pub error: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Debug)]
pub struct Statistics {
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_bytes: u64,
    pub latency_histogram: Histogram<u64>,
    pub status_codes: HashMap<u16, u64>,
    pub errors: HashMap<String, u64>,
    pub endpoint_stats: HashMap<String, EndpointStats>,
}

#[derive(Debug, Clone)]
pub struct EndpointStats {
    pub requests: u64,
    pub errors: u64,
    pub total_bytes: u64,
    pub latency_histogram: Histogram<u64>,
    pub status_codes: HashMap<u16, u64>,
}

impl EndpointStats {
    pub fn new() -> Self {
        Self {
            requests: 0,
            errors: 0,
            total_bytes: 0,
            latency_histogram: Histogram::<u64>::new(3).unwrap(),
            status_codes: HashMap::new(),
        }
    }

    pub fn record(&mut self, result: &RequestResult) {
        self.requests += 1;

        if let Some(status) = result.status_code {
            *self.status_codes.entry(status).or_insert(0) += 1;
        }

        if result.error.is_some() {
            self.errors += 1;
        }

        self.total_bytes += result.bytes_read as u64;

        let _ = self
            .latency_histogram
            .record(result.duration.as_micros() as u64);
    }

    pub fn avg_latency(&self) -> Duration {
        if self.requests == 0 {
            return Duration::from_secs(0);
        }
        Duration::from_micros(self.latency_histogram.mean() as u64)
    }

    pub fn min_latency(&self) -> Duration {
        Duration::from_micros(self.latency_histogram.min())
    }

    pub fn max_latency(&self) -> Duration {
        Duration::from_micros(self.latency_histogram.max())
    }
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            end_time: None,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_bytes: 0,
            latency_histogram: Histogram::<u64>::new(3).unwrap(),
            status_codes: HashMap::new(),
            errors: HashMap::new(),
            endpoint_stats: HashMap::new(),
        }
    }

    pub fn record(&mut self, result: RequestResult) {
        self.total_requests += 1;

        if result.error.is_some() {
            self.failed_requests += 1;
            let error_msg = result.error.as_ref().unwrap().clone();
            *self.errors.entry(error_msg).or_insert(0) += 1;
        } else {
            self.successful_requests += 1;
        }

        if let Some(status) = result.status_code {
            *self.status_codes.entry(status).or_insert(0) += 1;
        }

        self.total_bytes += result.bytes_read as u64;

        let _ = self
            .latency_histogram
            .record(result.duration.as_micros() as u64);

        // Record per-endpoint stats
        if let Some(endpoint) = &result.endpoint {
            let endpoint_stat = self
                .endpoint_stats
                .entry(endpoint.clone())
                .or_insert_with(EndpointStats::new);
            endpoint_stat.record(&result);
        }
    }

    pub fn finish(&mut self) {
        self.end_time = Some(Instant::now());
    }

    pub fn duration(&self) -> Duration {
        match self.end_time {
            Some(end) => end.duration_since(self.start_time),
            None => Instant::now().duration_since(self.start_time),
        }
    }

    pub fn requests_per_sec(&self) -> f64 {
        let duration = self.duration().as_secs_f64();
        if duration > 0.0 {
            self.total_requests as f64 / duration
        } else {
            0.0
        }
    }

    pub fn bytes_per_sec(&self) -> f64 {
        let duration = self.duration().as_secs_f64();
        if duration > 0.0 {
            self.total_bytes as f64 / duration
        } else {
            0.0
        }
    }

    pub fn avg_latency(&self) -> Duration {
        if self.total_requests == 0 {
            return Duration::from_secs(0);
        }
        Duration::from_micros(self.latency_histogram.mean() as u64)
    }

    pub fn percentile(&self, percentile: f64) -> Duration {
        Duration::from_micros(self.latency_histogram.value_at_percentile(percentile))
    }

    pub fn print_summary(&self, show_latency: bool) {
        let duration = self.duration();

        println!(
            "\n{} requests in {:.2}s, {:.2}MB read",
            self.total_requests,
            duration.as_secs_f64(),
            self.total_bytes as f64 / 1024.0 / 1024.0
        );

        if self.failed_requests > 0 {
            println!(
                "  {} errors ({:.2}%)",
                self.failed_requests,
                (self.failed_requests as f64 / self.total_requests as f64) * 100.0
            );
        }

        println!("Requests/sec:   {:.2}", self.requests_per_sec());
        println!(
            "Transfer/sec:   {:.2}MB",
            self.bytes_per_sec() / 1024.0 / 1024.0
        );

        // Print latency stats
        println!("\nLatency Stats:");
        println!(
            "  Avg:      {:.2}ms",
            self.avg_latency().as_secs_f64() * 1000.0
        );
        println!(
            "  Min:      {:.2}ms",
            Duration::from_micros(self.latency_histogram.min()).as_secs_f64() * 1000.0
        );
        println!(
            "  Max:      {:.2}ms",
            Duration::from_micros(self.latency_histogram.max()).as_secs_f64() * 1000.0
        );
        println!("  Stdev:    {:.2}ms", self.latency_histogram.stdev());

        if show_latency {
            println!("\nLatency Distribution:");
            println!(
                "  50%:  {:.2}ms",
                self.percentile(50.0).as_secs_f64() * 1000.0
            );
            println!(
                "  75%:  {:.2}ms",
                self.percentile(75.0).as_secs_f64() * 1000.0
            );
            println!(
                "  90%:  {:.2}ms",
                self.percentile(90.0).as_secs_f64() * 1000.0
            );
            println!(
                "  95%:  {:.2}ms",
                self.percentile(95.0).as_secs_f64() * 1000.0
            );
            println!(
                "  99%:  {:.2}ms",
                self.percentile(99.0).as_secs_f64() * 1000.0
            );
        }

        // Print status code distribution
        if !self.status_codes.is_empty() {
            println!("\nStatus Code Distribution:");
            let mut codes: Vec<_> = self.status_codes.iter().collect();
            codes.sort_by_key(|&(code, _)| code);
            for (code, count) in codes {
                let percentage = (*count as f64 / self.total_requests as f64) * 100.0;
                println!("  [{}] {} ({:.2}%)", code, count, percentage);
            }
        }

        // Print errors
        if !self.errors.is_empty() {
            println!("\nError Summary:");
            for (error, count) in &self.errors {
                println!("  {}: {}", error, count);
            }
        }

        // Print per-endpoint stats
        if self.endpoint_stats.len() > 1 {
            println!("\n=== Per-Endpoint Statistics ===");
            for (endpoint, stats) in &self.endpoint_stats {
                println!("\n[{}]", endpoint);
                println!("  Requests:     {}", stats.requests);
                if stats.errors > 0 {
                    println!(
                        "  Errors:       {} ({:.1}%)",
                        stats.errors,
                        (stats.errors as f64 / stats.requests as f64) * 100.0
                    );
                }
                println!(
                    "  Requests/sec: {:.2}",
                    stats.requests as f64 / duration.as_secs_f64()
                );
                println!(
                    "  Latency:      avg={:.2}ms, min={:.2}ms, max={:.2}ms",
                    stats.avg_latency().as_secs_f64() * 1000.0,
                    stats.min_latency().as_secs_f64() * 1000.0,
                    stats.max_latency().as_secs_f64() * 1000.0
                );

                if !stats.status_codes.is_empty() {
                    print!("  Status codes: ");
                    let mut codes: Vec<_> = stats.status_codes.iter().collect();
                    codes.sort_by_key(|&(code, _)| code);
                    let code_strs: Vec<String> = codes
                        .iter()
                        .map(|(code, count)| {
                            let pct = (**count as f64 / stats.requests as f64) * 100.0;
                            format!("[{}] {} ({:.1}%)", code, count, pct)
                        })
                        .collect();
                    println!("{}", code_strs.join(", "));
                }

                println!(
                    "  Data:         {:.2}KB total, {:.2}KB/sec",
                    stats.total_bytes as f64 / 1024.0,
                    (stats.total_bytes as f64 / duration.as_secs_f64()) / 1024.0
                );
            }
        }
    }
}

pub type SharedStats = Arc<Mutex<Statistics>>;

pub fn create_shared_stats() -> SharedStats {
    Arc::new(Mutex::new(Statistics::new()))
}

/// Snapshot of statistics for UI updates (cloneable)
#[derive(Clone, Debug)]
pub struct StatisticsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_bytes: u64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p75_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub status_codes: HashMap<u16, u64>,
    pub errors: HashMap<String, u64>,
    pub endpoint_stats: HashMap<String, EndpointStatsSnapshot>,
}

#[derive(Clone, Debug)]
pub struct EndpointStatsSnapshot {
    pub url: String,
    pub requests: u64,
    pub errors: u64,
    pub total_bytes: u64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub status_codes: HashMap<u16, u64>,
}

impl StatisticsSnapshot {
    /// Create an empty snapshot (for initial state)
    pub fn empty() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_bytes: 0,
            avg_latency_ms: 0.0,
            min_latency_ms: 0.0,
            max_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p75_latency_ms: 0.0,
            p90_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            status_codes: HashMap::new(),
            errors: HashMap::new(),
            endpoint_stats: HashMap::new(),
        }
    }

    /// Create a snapshot from Statistics
    pub fn from_statistics(stats: &Statistics) -> Self {
        Self {
            total_requests: stats.total_requests,
            successful_requests: stats.successful_requests,
            failed_requests: stats.failed_requests,
            total_bytes: stats.total_bytes,
            avg_latency_ms: stats.avg_latency().as_secs_f64() * 1000.0,
            min_latency_ms: Duration::from_micros(stats.latency_histogram.min()).as_secs_f64()
                * 1000.0,
            max_latency_ms: Duration::from_micros(stats.latency_histogram.max()).as_secs_f64()
                * 1000.0,
            p50_latency_ms: stats.percentile(50.0).as_secs_f64() * 1000.0,
            p75_latency_ms: stats.percentile(75.0).as_secs_f64() * 1000.0,
            p90_latency_ms: stats.percentile(90.0).as_secs_f64() * 1000.0,
            p95_latency_ms: stats.percentile(95.0).as_secs_f64() * 1000.0,
            p99_latency_ms: stats.percentile(99.0).as_secs_f64() * 1000.0,
            status_codes: stats.status_codes.clone(),
            errors: stats.errors.clone(),
            endpoint_stats: stats
                .endpoint_stats
                .iter()
                .map(|(url, ep_stats)| {
                    (
                        url.clone(),
                        EndpointStatsSnapshot {
                            url: url.clone(),
                            requests: ep_stats.requests,
                            errors: ep_stats.errors,
                            total_bytes: ep_stats.total_bytes,
                            avg_latency_ms: ep_stats.avg_latency().as_secs_f64() * 1000.0,
                            min_latency_ms: ep_stats.min_latency().as_secs_f64() * 1000.0,
                            max_latency_ms: ep_stats.max_latency().as_secs_f64() * 1000.0,
                            status_codes: ep_stats.status_codes.clone(),
                        },
                    )
                })
                .collect(),
        }
    }
}

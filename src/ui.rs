// Live-UI module for real-time terminal UI during HTTP performance testing
// Provides visual real-time statistics display with charts and progress indicators

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{BarChart, Block, Borders, Cell, Gauge, Paragraph, Row, Table};
use ratatui::{Frame, Terminal};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Theme configuration for Live-UI
#[derive(Clone, Copy, Debug)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    /// Auto-detect theme based on terminal background
    pub fn detect() -> Self {
        // Try to detect terminal background color
        // For now, default to dark theme
        // In the future, could check TERM_PROGRAM or other env vars
        if std::env::var("TERM_BG").as_deref() == Ok("light") {
            Self::Light
        } else {
            Self::Dark
        }
    }

    pub fn border_color(&self) -> Color {
        match self {
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::Rgb(0, 95, 215), // #005fd7
        }
    }

    pub fn title_color(&self) -> Color {
        match self {
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::Rgb(0, 95, 215), // #005fd7
        }
    }

    pub fn text_color(&self) -> Color {
        match self {
            Theme::Dark => Color::White,
            Theme::Light => Color::Rgb(68, 68, 68), // #444444
        }
    }

    pub fn highlight_color(&self) -> Color {
        match self {
            Theme::Dark => Color::Yellow,
            Theme::Light => Color::Rgb(223, 95, 0), // #df5f00
        }
    }

    pub fn success_color(&self) -> Color {
        match self {
            Theme::Dark => Color::Green,
            Theme::Light => Color::Green,
        }
    }

    pub fn error_color(&self) -> Color {
        match self {
            Theme::Dark => Color::Red,
            Theme::Light => Color::Red,
        }
    }

    pub fn warning_color(&self) -> Color {
        match self {
            Theme::Dark => Color::Yellow,
            Theme::Light => Color::Yellow,
        }
    }

    pub fn info_color(&self) -> Color {
        match self {
            Theme::Dark => Color::Blue,
            Theme::Light => Color::Blue,
        }
    }
}

// Import StatisticsSnapshot from stats module
use crate::stats::StatisticsSnapshot;

/// Real-time statistics snapshot for UI updates
#[derive(Clone, Debug)]
pub struct LiveStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub requests_per_sec: f64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p75_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub status_codes: HashMap<u16, u64>,
    pub error_rate: f64,
    pub elapsed_secs: f64,
    pub total_duration_secs: f64,
    pub progress: f64,
    pub requests_per_sec_history: VecDeque<f64>, // Last 10 seconds
    pub endpoint_stats: HashMap<String, EndpointLiveStats>,
}

#[derive(Clone, Debug)]
pub struct EndpointLiveStats {
    pub url: String,
    pub requests: u64,
    pub requests_per_sec: f64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub errors: u64,
    pub error_rate: f64,
    pub status_codes: HashMap<u16, u64>,
}

impl LiveStats {
    pub fn from_snapshot(
        snapshot: &StatisticsSnapshot,
        total_duration: Duration,
        start_time: Instant,
    ) -> Self {
        let elapsed = start_time.elapsed();
        let total_duration_secs = total_duration.as_secs_f64();
        let elapsed_secs = elapsed.as_secs_f64();
        let progress = if total_duration_secs > 0.0 {
            (elapsed_secs / total_duration_secs).min(1.0)
        } else {
            0.0
        };

        let error_rate = if snapshot.total_requests > 0 {
            (snapshot.failed_requests as f64 / snapshot.total_requests as f64) * 100.0
        } else {
            0.0
        };

        // Calculate requests per second
        let requests_per_sec = if elapsed_secs > 0.0 {
            snapshot.total_requests as f64 / elapsed_secs
        } else {
            0.0
        };

        // Build endpoint stats
        let mut endpoint_stats = HashMap::new();
        for (url, ep_snapshot) in &snapshot.endpoint_stats {
            let ep_rps = if elapsed_secs > 0.0 {
                ep_snapshot.requests as f64 / elapsed_secs
            } else {
                0.0
            };
            let ep_error_rate = if ep_snapshot.requests > 0 {
                (ep_snapshot.errors as f64 / ep_snapshot.requests as f64) * 100.0
            } else {
                0.0
            };

            endpoint_stats.insert(
                url.clone(),
                EndpointLiveStats {
                    url: ep_snapshot.url.clone(),
                    requests: ep_snapshot.requests,
                    requests_per_sec: ep_rps,
                    avg_latency_ms: ep_snapshot.avg_latency_ms,
                    min_latency_ms: ep_snapshot.min_latency_ms,
                    max_latency_ms: ep_snapshot.max_latency_ms,
                    errors: ep_snapshot.errors,
                    error_rate: ep_error_rate,
                    status_codes: ep_snapshot.status_codes.clone(),
                },
            );
        }

        Self {
            total_requests: snapshot.total_requests,
            successful_requests: snapshot.successful_requests,
            failed_requests: snapshot.failed_requests,
            requests_per_sec,
            avg_latency_ms: snapshot.avg_latency_ms,
            min_latency_ms: snapshot.min_latency_ms,
            max_latency_ms: snapshot.max_latency_ms,
            p50_latency_ms: snapshot.p50_latency_ms,
            p75_latency_ms: snapshot.p75_latency_ms,
            p90_latency_ms: snapshot.p90_latency_ms,
            p95_latency_ms: snapshot.p95_latency_ms,
            p99_latency_ms: snapshot.p99_latency_ms,
            status_codes: snapshot.status_codes.clone(),
            error_rate,
            elapsed_secs,
            total_duration_secs,
            progress,
            requests_per_sec_history: VecDeque::new(),
            endpoint_stats,
        }
    }
}

/// Live-UI controller
pub struct LiveUI {
    theme: Theme,
    stats_rx: mpsc::Receiver<StatisticsSnapshot>,
    start_time: Instant,
    total_duration: Duration,
    should_stop: bool,
}

impl LiveUI {
    pub fn new(stats_rx: mpsc::Receiver<StatisticsSnapshot>, total_duration: Duration) -> Self {
        Self {
            theme: Theme::detect(),
            stats_rx,
            start_time: Instant::now(),
            total_duration,
            should_stop: false,
        }
    }

    /// Run the Live-UI main loop
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
        terminal.clear()?;

        // Statistics update interval (1 second)
        let mut last_update = Instant::now();
        let update_interval = Duration::from_secs(1);

        // Request history for chart (last 10 seconds)
        let mut request_history: VecDeque<f64> = VecDeque::with_capacity(10);

        // Keep track of last valid snapshot
        let mut last_snapshot = StatisticsSnapshot::empty();

        // Track last request count for calculating instantaneous RPS
        let mut last_request_count = 0u64;
        let mut last_rps_update = Instant::now();

        loop {
            // Check if we should stop
            if self.should_stop {
                break;
            }

            // Check if test duration has elapsed
            if self.start_time.elapsed() >= self.total_duration {
                break;
            }

            // Try to receive new statistics
            let mut has_new_data = false;
            while let Ok(new_snapshot) = self.stats_rx.try_recv() {
                last_snapshot = new_snapshot;
                has_new_data = true;
            }

            // Update request history every second
            if has_new_data && last_update.elapsed() >= update_interval {
                // Calculate instantaneous RPS (change in requests over time interval)
                let time_since_last = last_rps_update.elapsed().as_secs_f64();
                let current_requests = last_snapshot.total_requests;

                let instantaneous_rps = if time_since_last > 0.0 {
                    let request_delta = current_requests.saturating_sub(last_request_count);
                    request_delta as f64 / time_since_last
                } else {
                    0.0
                };

                request_history.push_back(instantaneous_rps);
                if request_history.len() > 10 {
                    request_history.pop_front();
                }

                last_request_count = current_requests;
                last_rps_update = Instant::now();
                last_update = Instant::now();
            }

            // Use last valid snapshot
            let snapshot = last_snapshot.clone();

            // Handle input events
            if crossterm::event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                self.should_stop = true;
                                break;
                            }
                            KeyCode::Esc => {
                                self.should_stop = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Render UI
            let live_stats =
                LiveStats::from_snapshot(&snapshot, self.total_duration, self.start_time);
            let mut live_stats_with_history = live_stats.clone();
            live_stats_with_history.requests_per_sec_history = request_history.clone();

            terminal.draw(|f| self.render(f, &live_stats_with_history))?;
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        Ok(())
    }

    /// Render the UI
    fn render(&self, f: &mut Frame, stats: &LiveStats) {
        let size = f.size();

        // Main layout: vertical split
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Min(0),    // Main content
            ])
            .split(size);

        // Progress bar
        self.render_progress(f, chunks[0], stats);

        // Main content area
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Stats panel
                Constraint::Length(6), // Status codes
                Constraint::Min(0),    // Charts / Endpoint table
            ])
            .split(chunks[1]);

        // Stats panel
        self.render_stats_panel(f, main_chunks[0], stats);

        // Status codes
        self.render_status_codes(f, main_chunks[1], stats);

        // Charts or endpoint table
        if stats.endpoint_stats.len() > 1 {
            self.render_endpoint_table(f, main_chunks[2], stats);
        } else {
            self.render_charts(f, main_chunks[2], stats);
        }
    }

    /// Render progress bar
    fn render_progress(&self, f: &mut Frame, area: Rect, stats: &LiveStats) {
        let progress_text = format!(
            "Progress: {:.1}% | Elapsed: {:.1}s / {:.1}s",
            stats.progress * 100.0,
            stats.elapsed_secs,
            stats.total_duration_secs
        );

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.border_color()))
                    .title("Test Progress"),
            )
            .gauge_style(
                Style::default()
                    .fg(self.theme.highlight_color())
                    .bg(Color::DarkGray),
            )
            .ratio(stats.progress)
            .label(progress_text);

        f.render_widget(gauge, area);
    }

    /// Render statistics panel
    fn render_stats_panel(&self, f: &mut Frame, area: Rect, stats: &LiveStats) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Left: Requests & Rate
                Constraint::Percentage(50), // Right: Latency
            ])
            .split(area);

        // Left panel: Requests & Rate
        let requests_text = vec![
            Line::from(vec![
                Span::styled(
                    "Total Requests: ",
                    Style::default().fg(self.theme.text_color()),
                ),
                Span::styled(
                    format_number(stats.total_requests),
                    Style::default()
                        .fg(self.theme.highlight_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Successful: ", Style::default().fg(self.theme.text_color())),
                Span::styled(
                    format_number(stats.successful_requests),
                    Style::default().fg(self.theme.success_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled("Failed: ", Style::default().fg(self.theme.text_color())),
                Span::styled(
                    format_number(stats.failed_requests),
                    Style::default().fg(self.theme.error_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Requests/sec: ",
                    Style::default().fg(self.theme.text_color()),
                ),
                Span::styled(
                    format!("{:.2}", stats.requests_per_sec),
                    Style::default()
                        .fg(self.theme.highlight_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];

        let requests_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_color()))
            .title("Request Statistics");

        let requests_para = Paragraph::new(requests_text).block(requests_block);
        f.render_widget(requests_para, chunks[0]);

        // Right panel: Latency
        let latency_text = vec![
            Line::from(vec![
                Span::styled("Average: ", Style::default().fg(self.theme.text_color())),
                Span::styled(
                    format!("{:.2}ms", stats.avg_latency_ms),
                    Style::default().fg(self.theme.highlight_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled("Min: ", Style::default().fg(self.theme.text_color())),
                Span::styled(
                    format!("{:.2}ms", stats.min_latency_ms),
                    Style::default().fg(self.theme.success_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled("Max: ", Style::default().fg(self.theme.text_color())),
                Span::styled(
                    format!("{:.2}ms", stats.max_latency_ms),
                    Style::default().fg(self.theme.error_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled("Error Rate: ", Style::default().fg(self.theme.text_color())),
                Span::styled(
                    format!("{:.2}%", stats.error_rate),
                    Style::default().fg(if stats.error_rate > 5.0 {
                        self.theme.error_color()
                    } else {
                        self.theme.success_color()
                    }),
                ),
            ]),
        ];

        let latency_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_color()))
            .title("Latency Statistics");

        let latency_para = Paragraph::new(latency_text).block(latency_block);
        f.render_widget(latency_para, chunks[1]);
    }

    /// Render status code distribution
    fn render_status_codes(&self, f: &mut Frame, area: Rect, stats: &LiveStats) {
        let mut status_items = Vec::new();
        let mut codes: Vec<_> = stats.status_codes.iter().collect();
        codes.sort_by_key(|&(code, _)| code);

        for (code, count) in codes {
            let _color = match *code {
                200..=299 => self.theme.success_color(),
                300..=399 => self.theme.info_color(),
                400..=499 => self.theme.warning_color(),
                500..=599 => self.theme.error_color(),
                _ => self.theme.text_color(),
            };

            let percentage = if stats.total_requests > 0 {
                (*count as f64 / stats.total_requests as f64) * 100.0
            } else {
                0.0
            };

            status_items.push(format!(
                "[{}] {} ({:.1}%)",
                code,
                format_number(*count),
                percentage
            ));
        }

        let status_text = if status_items.is_empty() {
            vec![Line::from(Span::styled(
                "No status codes yet...",
                Style::default().fg(self.theme.text_color()),
            ))]
        } else {
            vec![Line::from(Span::styled(
                status_items.join("  "),
                Style::default().fg(self.theme.text_color()),
            ))]
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_color()))
            .title("Status Code Distribution");

        let para = Paragraph::new(status_text).block(block);
        f.render_widget(para, area);
    }

    /// Render charts (request rate and latency histogram)
    fn render_charts(&self, f: &mut Frame, area: Rect, stats: &LiveStats) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Request chart
                Constraint::Percentage(50), // Latency histogram
            ])
            .split(area);

        // Request rate chart
        self.render_request_chart(f, chunks[0], stats);

        // Latency histogram
        self.render_latency_histogram(f, chunks[1], stats);
    }

    /// Render request rate chart
    fn render_request_chart(&self, f: &mut Frame, area: Rect, stats: &LiveStats) {
        let history = &stats.requests_per_sec_history;
        if history.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.border_color()))
                .title("Request Rate (Last 10s)");
            let para = Paragraph::new("Collecting data...")
                .block(block)
                .alignment(Alignment::Center);
            f.render_widget(para, area);
            return;
        }

        let max_value = history.iter().copied().fold(0.0f64, f64::max).max(1.0);

        // Build title with max value indicator
        let formatted_max = if max_value >= 1_000_000.0 {
            format!("{:.1}M", max_value / 1_000_000.0)
        } else if max_value >= 1_000.0 {
            format!("{:.1}K", max_value / 1_000.0)
        } else {
            format!("{:.1}", max_value)
        };
        let title = format!("Request Rate (Last 10s) - Max: {} req/s", formatted_max);

        // Create bar chart data - use actual TPS values, not normalized percentages
        let bar_data: Vec<(String, u64)> = history
            .iter()
            .enumerate()
            .map(|(i, &value)| {
                // Format TPS value with K/M suffix
                let tps_str = if value >= 1_000_000.0 {
                    format!("{:.0}M", value / 1_000_000.0)
                } else if value >= 1_000.0 {
                    format!("{:.0}K", value / 1_000.0)
                } else {
                    format!("{:.0}", value)
                };

                // Label with time and formatted TPS: "1s\n92K"
                let label = format!("{}s\n{}", i + 1, tps_str);

                // Use actual TPS value (as integer) for bar height
                let tps_value = value as u64;
                (label, tps_value)
            })
            .collect();

        // Convert to references for BarChart
        let bar_data_refs: Vec<(&str, u64)> = bar_data
            .iter()
            .map(|(label, height)| (label.as_str(), *height))
            .collect();

        let bar_chart = BarChart::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.border_color()))
                    .title(title),
            )
            .data(&bar_data_refs)
            .bar_width(3)
            .bar_gap(1)
            .bar_style(Style::default().fg(self.theme.highlight_color()))
            .value_style(
                Style::default()
                    .fg(self.theme.text_color())
                    .add_modifier(Modifier::BOLD),
            )
            .label_style(Style::default().fg(self.theme.text_color()));

        f.render_widget(bar_chart, area);
    }

    /// Render latency histogram (percentiles)
    fn render_latency_histogram(&self, f: &mut Frame, area: Rect, stats: &LiveStats) {
        let max_latency = stats.max_latency_ms.max(1.0);

        let data = vec![
            ("P50", stats.p50_latency_ms / max_latency * 100.0),
            ("P75", stats.p75_latency_ms / max_latency * 100.0),
            ("P90", stats.p90_latency_ms / max_latency * 100.0),
            ("P95", stats.p95_latency_ms / max_latency * 100.0),
            ("P99", stats.p99_latency_ms / max_latency * 100.0),
        ];

        let bar_data: Vec<(&str, u64)> = data
            .iter()
            .map(|(label, value)| (*label, *value as u64))
            .collect();

        let bar_chart = BarChart::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.border_color()))
                    .title("Latency Percentiles"),
            )
            .data(&bar_data)
            .bar_width(5)
            .bar_gap(1)
            .bar_style(Style::default().fg(self.theme.info_color()))
            .value_style(Style::default().fg(self.theme.text_color()));

        f.render_widget(bar_chart, area);

        // Add text showing actual values
        let values_text: Vec<Line> = data
            .iter()
            .map(|(label, _)| {
                let value = match *label {
                    "P50" => stats.p50_latency_ms,
                    "P75" => stats.p75_latency_ms,
                    "P90" => stats.p90_latency_ms,
                    "P95" => stats.p95_latency_ms,
                    "P99" => stats.p99_latency_ms,
                    _ => 0.0,
                };
                Line::from(vec![
                    Span::styled(
                        format!("{}: ", label),
                        Style::default().fg(self.theme.text_color()),
                    ),
                    Span::styled(
                        format!("{:.2}ms", value),
                        Style::default().fg(self.theme.highlight_color()),
                    ),
                ])
            })
            .collect();

        let values_area = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(values_text.len() as u16 + 1),
            width: area.width.saturating_sub(2),
            height: values_text.len() as u16,
        };

        let values_para = Paragraph::new(values_text);
        f.render_widget(values_para, values_area);
    }

    /// Render endpoint statistics table (for multi-endpoint mode)
    fn render_endpoint_table(&self, f: &mut Frame, area: Rect, stats: &LiveStats) {
        let mut rows = Vec::new();

        // Header
        rows.push(Row::new(vec![
            Cell::from("URL").style(
                Style::default()
                    .fg(self.theme.title_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("TPS").style(
                Style::default()
                    .fg(self.theme.title_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Avg").style(
                Style::default()
                    .fg(self.theme.title_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Min").style(
                Style::default()
                    .fg(self.theme.title_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Max").style(
                Style::default()
                    .fg(self.theme.title_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Errors").style(
                Style::default()
                    .fg(self.theme.title_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Err%").style(
                Style::default()
                    .fg(self.theme.title_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Data rows
        let mut endpoints: Vec<_> = stats.endpoint_stats.iter().collect();
        endpoints.sort_by_key(|(url, _)| *url);

        for (_, ep_stats) in endpoints {
            let error_color = if ep_stats.error_rate > 5.0 {
                self.theme.error_color()
            } else {
                self.theme.success_color()
            };

            rows.push(Row::new(vec![
                Cell::from(truncate_string(&ep_stats.url, 30))
                    .style(Style::default().fg(self.theme.text_color())),
                Cell::from(format_rps(ep_stats.requests_per_sec))
                    .style(Style::default().fg(self.theme.highlight_color())),
                Cell::from(format!("{:.2}ms", ep_stats.avg_latency_ms))
                    .style(Style::default().fg(self.theme.text_color())),
                Cell::from(format!("{:.2}ms", ep_stats.min_latency_ms))
                    .style(Style::default().fg(self.theme.success_color())),
                Cell::from(format!("{:.2}ms", ep_stats.max_latency_ms))
                    .style(Style::default().fg(self.theme.error_color())),
                Cell::from(format_number(ep_stats.errors)).style(Style::default().fg(error_color)),
                Cell::from(format!("{:.1}%", ep_stats.error_rate))
                    .style(Style::default().fg(error_color)),
            ]));
        }

        let widths = [
            Constraint::Percentage(40), // URL
            Constraint::Percentage(10), // TPS
            Constraint::Percentage(10), // Avg
            Constraint::Percentage(10), // Min
            Constraint::Percentage(10), // Max
            Constraint::Percentage(10), // Errors
            Constraint::Percentage(10), // Err%
        ];

        let table = Table::new(rows, widths)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.border_color()))
                    .title("Per-Endpoint Statistics"),
            )
            .column_spacing(1);

        f.render_widget(table, area);
    }
}

/// Format large numbers with K/M suffix
fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Format RPS (requests per second) with K/M suffix
fn format_rps(rps: f64) -> String {
    if rps >= 1_000_000.0 {
        format!("{:.2}M", rps / 1_000_000.0)
    } else if rps >= 1_000.0 {
        format!("{:.2}K", rps / 1_000.0)
    } else {
        format!("{:.2}", rps)
    }
}

/// Truncate string to max length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

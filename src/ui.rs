// UI module for live terminal UI
// This is a placeholder for future implementation of live terminal UI using ratatui

use crate::stats::SharedStats;

pub struct LiveUI {
    stats: SharedStats,
}

impl LiveUI {
    pub fn new(stats: SharedStats) -> Self {
        Self { stats }
    }

    pub async fn run(&mut self) {
        // TODO: Implement live UI using ratatui
        // For now, this is a placeholder
        println!("Live UI not yet implemented");
    }
}

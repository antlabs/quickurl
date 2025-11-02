mod cli;
mod curl_parser;
mod engine;
mod stats;
mod template;
mod batch;
mod mock_server;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::Args;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    // Handle different modes
    if args.mock_server {
        mock_server::run(args).await?;
    } else if args.batch_config.is_some() {
        batch::run_batch_tests(args).await?;
    } else if args.help_templates {
        template::print_help();
    } else {
        engine::run_benchmark(args).await?;
    }

    Ok(())
}

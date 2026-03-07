mod cli;

use clap::Parser;
use cli::{Cli, Verbosity};
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let default_level = match cli.verbosity() {
        Verbosity::Quiet => "error",
        Verbosity::Normal => "warn",
        Verbosity::Verbose => "debug",
        Verbosity::Trace => "trace",
    };

    // RUST_LOG overrides the verbosity flag
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .init();

    cli.run()
}

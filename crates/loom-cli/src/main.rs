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

    // Auto-update check: silent, respects hourly throttle.
    // Checks env var first (fast, no disk I/O), then config file.
    if !loom_core::update::is_disabled_by_env() {
        // Check config-based opt-out (loads config from disk)
        let config_disabled = loom_core::config::Config::load()
            .map(|c| !c.update.enabled)
            .unwrap_or(false);

        if !config_disabled {
            match loom_core::update::check_and_update(false, false) {
                Ok(Some(v)) => {
                    eprintln!("Updated to v{v}. Please restart.");
                    std::process::exit(0);
                }
                Ok(None) => {} // up-to-date or throttled
                Err(e) => {
                    tracing::debug!("Auto-update check failed: {e}");
                }
            }
        }
    }

    cli.run()
}

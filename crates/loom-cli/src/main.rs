mod cli;
mod tree_select;

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

    // Startup version check: lightweight HTTP GET, no download or binary replacement.
    // If a newer version is available, notify the user and proceed with their command.
    if !loom_core::update::is_disabled_by_env() {
        let config_disabled = loom_core::config::Config::load()
            .map(|c| !c.update.enabled)
            .unwrap_or(false);

        if !config_disabled {
            match loom_core::update::check_version_throttled(false) {
                Ok(Some(latest)) => {
                    eprintln!("Update available: v{latest}. Run `loom update` to install.");
                }
                Ok(None) => {} // up-to-date or throttled
                Err(e) => {
                    tracing::debug!("Update check failed: {e}");
                }
            }
        }
    }

    cli.run()
}

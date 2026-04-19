use std::process::ExitCode;

use accelerator_visualiser::config::Config;
use clap::Parser;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "accelerator-visualiser", version, about)]
struct Cli {
    /// Path to the config.json written by launch-server.sh.
    #[arg(long = "config", value_name = "PATH")]
    config: std::path::PathBuf,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let cfg = match Config::from_path(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "failed to load config");
            return ExitCode::from(2);
        }
    };

    info!(
        plugin_version = %cfg.plugin_version,
        host = %cfg.host,
        owner_pid = cfg.owner_pid,
        doc_paths = cfg.doc_paths.len(),
        templates = cfg.templates.len(),
        "config loaded"
    );
    ExitCode::SUCCESS
}

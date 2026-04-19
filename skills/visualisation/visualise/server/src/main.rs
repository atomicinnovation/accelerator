use std::process::ExitCode;

use accelerator_visualiser::{config::Config, server};
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
    let info_path = cfg.tmp_path.join("server-info.json");
    info!(
        config = %cli.config.display(),
        info_path = %info_path.display(),
        "bootstrapping server"
    );

    if let Err(e) = server::run(cfg, &info_path).await {
        error!(error = %e, "server error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

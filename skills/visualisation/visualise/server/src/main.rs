use std::process::ExitCode;

use accelerator_visualiser::{config::Config, log, server};
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
    let cli = Cli::parse();
    let cfg = match Config::from_path(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e}");
            return ExitCode::from(2);
        }
    };

    let _log_guard = match log::init(&cfg.log_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("failed to init logging: {e}");
            return ExitCode::from(2);
        }
    };

    if let Err(e) = redirect_std_streams_to_devnull() {
        tracing::error!(error = %e, "failed to redirect std streams to /dev/null");
        return ExitCode::from(2);
    }

    let info_path = cfg.tmp_path.join("server-info.json");
    info!(
        config = %cli.config.display(),
        log_path = %cfg.log_path.display(),
        "bootstrapping server"
    );

    let result = server::run(cfg, &info_path).await;
    if let Err(ref e) = result {
        error!(error = %e, "server error");
    }
    drop(_log_guard);
    if result.is_err() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(unix)]
fn redirect_std_streams_to_devnull() -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let devnull = std::fs::OpenOptions::new().write(true).open("/dev/null")?;
    let fd = devnull.as_raw_fd();
    // SAFETY: fd is a valid file descriptor we just opened. dup2 targets
    // stdout (1) and stderr (2) which always exist in a unix process.
    let r1 = unsafe { libc::dup2(fd, 1) };
    let r2 = unsafe { libc::dup2(fd, 2) };
    if r1 == -1 || r2 == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
fn redirect_std_streams_to_devnull() -> std::io::Result<()> {
    Ok(())
}

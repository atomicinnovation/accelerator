#![cfg_attr(
    not(test),
    warn(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use std::path::PathBuf;
use std::process::ExitCode;

use accelerator_visualiser::{compose, log, orchestration, server};
use clap::{Parser, Subcommand};
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "accelerator-visualiser", version, about)]
struct Cli {
    /// Owner process pid whose exit triggers auto-shutdown; the invoker passes
    /// `$PPID`. Used by `start`/`serve`; ignored by `stop`/`status` (a global so
    /// one invocation form serves every subcommand).
    #[arg(long, global = true)]
    owner_pid: Option<i32>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the server daemon in the foreground (spawned detached by `start`).
    Serve {
        /// Owner process start-time, for the pid-identity cross-check.
        #[arg(long)]
        owner_start_time: Option<u64>,
    },
    /// Start the server detached and print its loopback URL.
    Start,
    /// Stop the running server.
    Stop,
    /// Report whether the server is running.
    Status,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve { owner_start_time } => {
            run_serve(cli.owner_pid.unwrap_or(0), owner_start_time)
        }
        Command::Start => orchestration::start_command(cli.owner_pid),
        Command::Stop => orchestration::stop_command(),
        Command::Status => orchestration::status_command(),
    }
}

/// The bind host: loopback in every shipped build. The `dev-frontend` (test)
/// binary additionally honours `E2E_SERVER_HOST` so the containerised
/// visual-regression flow can reach the host over its bridge gateway.
fn resolve_host() -> String {
    #[cfg(feature = "dev-frontend")]
    {
        if let Ok(host) = std::env::var("E2E_SERVER_HOST") {
            if !host.is_empty() {
                return host;
            }
        }
    }
    "127.0.0.1".to_string()
}

fn run_serve(owner_pid: i32, owner_start_time: Option<u64>) -> ExitCode {
    let Some(plugin_root) = std::env::var_os("ACCELERATOR_PLUGIN_ROOT") else {
        eprintln!("ACCELERATOR_PLUGIN_ROOT is not set");
        return ExitCode::from(2);
    };
    let plugin_root = PathBuf::from(plugin_root);
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("cannot determine working directory: {e}");
            return ExitCode::from(2);
        }
    };

    let cfg = match compose::load(compose::Params {
        cwd,
        plugin_root,
        owner_pid,
        owner_start_time,
        host: resolve_host(),
    }) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to compose config: {e}");
            return ExitCode::from(2);
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("failed to build async runtime: {e}");
            return ExitCode::from(2);
        }
    };
    runtime.block_on(serve(cfg))
}

async fn serve(cfg: accelerator_visualiser::config::Config) -> ExitCode {
    let log_guard = match log::init(&cfg.log_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("failed to init logging: {e}");
            return ExitCode::from(2);
        }
    };

    if let Err(e) = redirect_std_streams_to_devnull() {
        error!(error = %e, "failed to redirect std streams to /dev/null");
        return ExitCode::from(2);
    }

    let info_path = cfg.tmp_path.join("server-info.json");
    info!(
        project_root = %cfg.project_root.display(),
        log_path = %cfg.log_path.display(),
        "bootstrapping server"
    );

    let result = server::run(cfg, &info_path).await;
    if let Err(ref e) = result {
        error!(error = %e, "server error");
    }
    drop(log_guard);
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

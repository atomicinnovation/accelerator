//! `a9r` — Accelerator's single CLI binary.
//!
//! Subcommands:
//!   - `config-read-value <key> [default]` — port of `config-read-value.sh`.
//!   - `config-read-path <key> [default]` — port of `config-read-path.sh`.
//!   - `visualise --config <path>` — the visualiser server (the boot block
//!     lifted from the former `accelerator-visualiser` binary). A bare
//!     `a9r --config <path>` (no subcommand) is aliased to this form for
//!     transitional compatibility with the old launcher invocation — see
//!     `inject_visualise_alias`.
//!
//! All parsing/resolution logic lives in `a9r-core`; this binary owns
//! stream/exit policy. The config-read subcommands must not pull up the
//! tokio/axum machinery — only `visualise` builds a runtime.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use a9r_core::{ConfigError, ReadOutcome};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "a9r", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Read a single configuration value from the accelerator config files.
    ConfigReadValue {
        /// Dot-notation key, e.g. `agents.reviewer` or `enabled`.
        key: Option<String>,
        /// Value to echo when the key is absent.
        default: Option<String>,
    },
    /// Read a path configuration value (prepends `paths.`; applies the
    /// centralised defaults table).
    ConfigReadPath {
        key: Option<String>,
        default: Option<String>,
    },
    /// Run the meta-directory visualiser server.
    Visualise {
        /// Path to the config.json written by launch-server.sh.
        #[arg(long = "config", value_name = "PATH")]
        config: PathBuf,
    },
}

fn migration_mode() -> bool {
    matches!(
        std::env::var("ACCELERATOR_MIGRATION_MODE").as_deref(),
        Ok("1")
    )
}

/// Emit a `ReadOutcome`: warnings to stderr (in order), then the value to
/// stdout with exactly one trailing newline (matching the bash `echo`).
fn emit(outcome: &ReadOutcome) {
    for w in &outcome.warnings {
        eprintln!("{w}");
    }
    println!("{}", outcome.value);
}

fn run_config_read_value(
    start: &Path,
    key: Option<String>,
    default: Option<String>,
    mm: bool,
) -> Result<(), ConfigError> {
    // config-read-value asserts the layout BEFORE validating the key, so a
    // legacy layout + empty key surfaces the legacy message (not usage).
    a9r_core::assert_no_legacy_layout(start, mm)?;
    let key = key.unwrap_or_default();
    if key.is_empty() {
        return Err(ConfigError::Usage(a9r_core::VALUE_USAGE));
    }
    let default = default.unwrap_or_default();
    emit(&a9r_core::read_value(start, &key, &default, mm));
    Ok(())
}

fn run_config_read_path(
    start: &Path,
    key: Option<String>,
    default: Option<String>,
    mm: bool,
) -> Result<(), ConfigError> {
    // config-read-path validates the key FIRST, then resolves the default and
    // migration warnings, then defers the legacy assert + lookup to the value
    // step — preserving the bash ordering (path warnings → assert → value
    // warnings → stdout).
    let key = key.unwrap_or_default();
    if key.is_empty() {
        return Err(ConfigError::Usage(a9r_core::PATH_USAGE));
    }
    let resolution = a9r_core::resolve_path(start, &key, default.as_deref(), mm);
    for w in &resolution.warnings {
        eprintln!("{w}");
    }
    a9r_core::assert_no_legacy_layout(start, mm)?;
    emit(&a9r_core::read_value(
        start,
        &resolution.value_key,
        &resolution.default,
        mm,
    ));
    Ok(())
}

fn run_config_read(result: Result<(), ConfigError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", e.stderr());
            ExitCode::from(e.exit_code())
        }
    }
}

/// Transitional invocation alias: the pre-rename `accelerator-visualiser`
/// binary was launched as `<bin> --config <path>` (no subcommand). The renamed
/// single binary is launched as `a9r visualise --config <path>`, but an older
/// installed launcher — or the old `accelerator-visualiser` asset name, which
/// during the rename transition is a byte-identical copy of this binary — still
/// uses the bare `--config` form. clap's derive `Subcommand` has no native
/// default subcommand, so detect the "no subcommand + leading `--config`" shape
/// in argv and inject `visualise`. `--version`/`--help`/`-V`/`-h` are left
/// untouched so top-level info flags still work. Bounded shim: removed together
/// with the bash fallback / old asset name (see plan Phase 6 §1).
fn inject_visualise_alias(mut args: Vec<String>) -> Vec<String> {
    // args[0] is the program name. The first real argument decides.
    match args.get(1).map(String::as_str) {
        Some("--config") => args.insert(1, "visualise".to_string()),
        Some(a) if a.starts_with("--config=") => {
            args.insert(1, "visualise".to_string());
        }
        _ => {}
    }
    args
}

fn main() -> ExitCode {
    let cli = Cli::parse_from(inject_visualise_alias(std::env::args().collect()));
    let mm = migration_mode();
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    match cli.command {
        Command::ConfigReadValue { key, default } => {
            run_config_read(run_config_read_value(&start, key, default, mm))
        }
        Command::ConfigReadPath { key, default } => {
            run_config_read(run_config_read_path(&start, key, default, mm))
        }
        Command::Visualise { config } => run_visualise(&config),
    }
}

// ── visualise ───────────────────────────────────────────────────────────────

/// Boot the visualiser server. Builds a multi-thread tokio runtime on demand
/// so the config-read subcommands never pay for it. Mirrors the boot block of
/// the former `accelerator-visualiser` binary.
fn run_visualise(config: &Path) -> ExitCode {
    use tracing::{error, info};
    use visualiser::{config::Config, log, server};

    let cfg = match Config::from_path(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e}");
            return ExitCode::from(2);
        }
    };

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

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error!(error = %e, "failed to build tokio runtime");
            return ExitCode::from(2);
        }
    };

    let info_path = cfg.tmp_path.join("server-info.json");
    info!(
        config = %config.display(),
        log_path = %cfg.log_path.display(),
        "bootstrapping server"
    );

    let result = runtime.block_on(server::run(cfg, &info_path));
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

#[cfg(test)]
mod tests {
    use super::inject_visualise_alias;

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn bare_config_injects_visualise() {
        assert_eq!(
            inject_visualise_alias(argv(&["a9r", "--config", "/tmp/c.json"])),
            argv(&["a9r", "visualise", "--config", "/tmp/c.json"])
        );
    }

    #[test]
    fn bare_config_eq_form_injects_visualise() {
        assert_eq!(
            inject_visualise_alias(argv(&["a9r", "--config=/tmp/c.json"])),
            argv(&["a9r", "visualise", "--config=/tmp/c.json"])
        );
    }

    #[test]
    fn explicit_visualise_is_untouched() {
        let a = argv(&["a9r", "visualise", "--config", "/tmp/c.json"]);
        assert_eq!(inject_visualise_alias(a.clone()), a);
    }

    #[test]
    fn config_read_subcommand_is_untouched() {
        let a = argv(&["a9r", "config-read-path", "plans"]);
        assert_eq!(inject_visualise_alias(a.clone()), a);
    }

    #[test]
    fn version_and_help_flags_are_untouched() {
        for flag in ["--version", "-V", "--help", "-h"] {
            let a = argv(&["a9r", flag]);
            assert_eq!(inject_visualise_alias(a.clone()), a);
        }
    }
}

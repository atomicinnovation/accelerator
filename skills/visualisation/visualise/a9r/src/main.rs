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

use a9r_core::{CommandOutput, ConfigError, ReadOutcome, SkillSection};
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
    /// Print the project context (config-file markdown bodies) under a header.
    ConfigReadContext,
    /// Print the resolved agent-name override block.
    ConfigReadAgents,
    /// Resolve a template (config → templates dir → plugin default) and print
    /// it, fence-wrapped. The plugin root comes from `ACCELERATOR_PLUGIN_ROOT`.
    ConfigReadTemplate {
        /// The template name, e.g. `plan` or `pr-description`.
        key: Option<String>,
    },
    /// Print a skill's `context.md` wrapped in a section header, if present.
    ConfigReadSkillContext {
        /// The skill name (the per-skill customisation directory).
        skill: Option<String>,
    },
    /// Print a skill's `instructions.md` wrapped in a section header.
    ConfigReadSkillInstructions { skill: Option<String> },
    /// Print artifact metadata (UTC timestamp, VCS revision, repo name,
    /// filename timestamp) for stamping generated documents.
    ArtifactDeriveMetadata,
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

/// Emit a `CommandOutput`: stderr verbatim, then stdout verbatim. Unlike
/// [`emit`], nothing is added — the section commands own their trailing
/// newline and an empty stdout prints nothing.
fn emit_command(out: &CommandOutput) {
    use std::io::Write;
    if !out.stderr.is_empty() {
        eprint!("{}", out.stderr);
    }
    print!("{}", out.stdout);
    let _ = std::io::stdout().flush();
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

fn run_config_read_skill(
    start: &Path,
    skill: Option<String>,
    section: &SkillSection,
    usage: &'static str,
    mm: bool,
) -> Result<(), ConfigError> {
    // Both skill readers assert the layout BEFORE validating the argument,
    // so a legacy layout + missing skill surfaces the legacy message.
    a9r_core::assert_no_legacy_layout(start, mm)?;
    let skill = skill.unwrap_or_default();
    if skill.is_empty() {
        return Err(ConfigError::Usage(usage));
    }
    emit_command(&a9r_core::read_skill_section(start, &skill, section));
    Ok(())
}

fn run_config_read_template(
    start: &Path,
    key: Option<String>,
    mm: bool,
) -> Result<(), ConfigError> {
    // config-read-template asserts the layout, then validates the name, then
    // resolves — matching the bash ordering.
    a9r_core::assert_no_legacy_layout(start, mm)?;
    let key = key.unwrap_or_default();
    if key.is_empty() {
        return Err(ConfigError::Usage(a9r_core::template::TEMPLATE_USAGE));
    }
    // The plugin root (parent of scripts/) is injected by the shim and the
    // test harness via ACCELERATOR_PLUGIN_ROOT; the binary cannot derive it.
    let plugin_root = std::env::var("ACCELERATOR_PLUGIN_ROOT").unwrap_or_default();
    emit_command(&a9r_core::template::read_template(
        start,
        &key,
        Path::new(&plugin_root),
        mm,
    )?);
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
        Command::ConfigReadContext => {
            emit_command(&a9r_core::read_context(&start, mm));
            ExitCode::SUCCESS
        }
        Command::ConfigReadAgents => {
            emit_command(&a9r_core::agents::read_agents(&start, mm));
            ExitCode::SUCCESS
        }
        Command::ConfigReadTemplate { key } => {
            run_config_read(run_config_read_template(&start, key, mm))
        }
        Command::ConfigReadSkillContext { skill } => run_config_read(run_config_read_skill(
            &start,
            skill,
            &SkillSection::Context,
            a9r_core::SKILL_CONTEXT_USAGE,
            mm,
        )),
        Command::ConfigReadSkillInstructions { skill } => run_config_read(run_config_read_skill(
            &start,
            skill,
            &SkillSection::Instructions,
            a9r_core::SKILL_INSTRUCTIONS_USAGE,
            mm,
        )),
        Command::ArtifactDeriveMetadata => run_artifact_derive_metadata(),
        Command::Visualise { config } => run_visualise(&config),
    }
}

// ── artifact-derive-metadata ──────────────────────────────────────────────────

/// Print artifact metadata, a faithful port of `artifact-derive-metadata.sh`:
/// a UTC ISO timestamp, then (when in a repo) the VCS revision and repo name,
/// then a local filename timestamp. jj is preferred over git; neither present
/// → the revision/name lines are omitted. Byte-for-byte parity is not possible
/// (timestamps/revisions are live), so the gate is the output *shape*
/// (`test-metadata-helpers.sh`), which routes through the shim in a9r mode.
fn run_artifact_derive_metadata() -> ExitCode {
    use chrono::{Local, Utc};
    let datetime_utc = Utc::now().format("%Y-%m-%dT%H:%M:%S+00:00").to_string();
    let filename_ts = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let (revision, repo_name) = vcs_metadata();

    println!("Current Date/Time (UTC): {datetime_utc}");
    if !revision.is_empty() {
        println!("Current Revision: {revision}");
    }
    if !repo_name.is_empty() {
        println!("Repository Name: {repo_name}");
    }
    println!("Timestamp For Filename: {filename_ts}");
    ExitCode::SUCCESS
}

/// `(revision, repo_name)` from jj (preferred) or git, or two empty strings
/// when neither rooted VCS is available — mirroring the bash if/elif/else.
fn vcs_metadata() -> (String, String) {
    use std::process::Command;

    // jj: `jj root` must succeed (also covers `command -v jj`, since a missing
    // binary fails to spawn). Run it twice like the bash (guard + capture).
    if command_succeeds(Command::new("jj").arg("root")) {
        if let Some(root) = command_stdout(Command::new("jj").arg("root")) {
            let revision = command_stdout(Command::new("jj").args([
                "log",
                "-r",
                "@",
                "--no-graph",
                "--template",
                "commit_id",
            ]))
            .unwrap_or_default();
            return (revision, basename(&root));
        }
    }

    // git: inside a work tree.
    if command_succeeds(Command::new("git").args(["rev-parse", "--is-inside-work-tree"])) {
        if let Some(root) =
            command_stdout(Command::new("git").args(["rev-parse", "--show-toplevel"]))
        {
            let revision =
                command_stdout(Command::new("git").args(["rev-parse", "HEAD"])).unwrap_or_default();
            return (revision, basename(&root));
        }
    }

    (String::new(), String::new())
}

/// Run a command, discarding output; true iff it spawned and exited 0.
fn command_succeeds(cmd: &mut std::process::Command) -> bool {
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Run a command and return its trimmed stdout, or `None` on spawn failure or
/// a non-zero exit (matching `$(…)` which yields empty on failure under the
/// bash guards above).
fn command_stdout(cmd: &mut std::process::Command) -> Option<String> {
    let out = cmd.stderr(std::process::Stdio::null()).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    } else {
        None
    }
}

/// `basename` of a path string (the final component), or the string itself.
fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned())
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

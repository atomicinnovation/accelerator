//! The `accelerator visualiser start|stop|status` lifecycle, ported from the
//! retired shell orchestration. Each concern is a focused submodule: process
//! identity/termination ([`process`]), the exclusive launch lock ([`lock`]), and
//! the state directory + lifecycle files ([`state`]).

pub mod lock;
pub mod process;
pub mod state;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use config::{ConfigAccess, Key, Resolved};
use config_adapters::LegacyPolicy;

use state::StateDir;

/// How long `start` polls for the daemon's `server-info.json` before failing.
const READINESS_BUDGET: Duration = Duration::from_secs(5);
const READINESS_TICK: Duration = Duration::from_millis(100);
/// SIGTERM → grace → SIGKILL window used by `stop`.
const STOP_GRACE: Duration = Duration::from_secs(2);
const STOP_TICK: Duration = Duration::from_millis(100);

#[derive(Debug, thiserror::Error)]
enum OrchestrationError {
    #[error("{0}")]
    Config(#[from] config::ConfigError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// The project context a lifecycle command resolves before acting.
struct Prepared {
    project_root: PathBuf,
    tmp_rel: String,
    state_dir: StateDir,
    composed: config_adapters::Composed,
}

impl Prepared {
    fn discover(cwd: &Path) -> Result<Self, OrchestrationError> {
        let composed = config_adapters::compose(cwd, LegacyPolicy::Reject)?;
        let project_root = config_adapters::FileConfigStore::discover_root(cwd);
        let tmp_rel = composed
            .service
            .effective_nonempty(&Key::parse("paths.tmp")?, None)?
            .rendered();
        let state_dir =
            StateDir::new(project_root.join(&tmp_rel).join("visualiser"));
        Ok(Self {
            project_root,
            tmp_rel,
            state_dir,
            composed,
        })
    }

    /// The init sentinel `<project>/<paths.tmp>/.gitignore` written by
    /// `/accelerator:init`.
    fn init_sentinel(&self) -> PathBuf {
        self.project_root.join(&self.tmp_rel).join(".gitignore")
    }

    /// True when the project predates the tickets→work rename: a `paths.tickets`
    /// override with no `paths.work`.
    fn has_ticket_migration_debt(&self) -> Result<bool, OrchestrationError> {
        let tickets = self
            .composed
            .service
            .get(&Key::parse("paths.tickets")?, None)?;
        let work = self
            .composed
            .service
            .get(&Key::parse("paths.work")?, None)?;
        Ok(matches!(tickets, Resolved::Found(_))
            && matches!(work, Resolved::Absent))
    }
}

fn print_json(value: &serde_json::Value) {
    println!("{}", serde_json::to_string(value).unwrap_or_default());
}

/// Emit a failure result on stdout (the `!`-preprocessor line the skill relays)
/// and exit non-zero.
fn fail(value: serde_json::Value) -> ExitCode {
    print_json(&value);
    ExitCode::from(1)
}

fn is_loopback_url(url: &str) -> bool {
    // http://127.0.0.1:<port> with an optional trailing slash.
    url.strip_prefix("http://127.0.0.1:")
        .map(|rest| rest.trim_end_matches('/'))
        .is_some_and(|port| {
            !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
        })
}

/// Resolve the owner pid: the explicit flag (passed by the SKILL.md invocation
/// as `$PPID`) when positive, else the parent of this process — correct under
/// the launcher's exec-replace, where `start`'s parent is the invoking harness.
fn resolve_owner_pid(
    explicit: Option<i32>,
    fallback: impl FnOnce() -> i32,
) -> i32 {
    explicit.filter(|pid| *pid > 0).unwrap_or_else(fallback)
}

pub fn start_command(owner_pid_flag: Option<i32>) -> ExitCode {
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            return fail(serde_json::json!({
                "error": "cannot determine working directory",
                "detail": error.to_string(),
            }));
        }
    };
    match start_inner(&cwd, owner_pid_flag) {
        Ok(code) => code,
        Err(error) => fail(serde_json::json!({ "error": error.to_string() })),
    }
}

fn start_inner(
    cwd: &Path,
    owner_pid_flag: Option<i32>,
) -> Result<ExitCode, OrchestrationError> {
    let prepared = Prepared::discover(cwd)?;
    let state = &prepared.state_dir;

    // Reuse short-circuit runs before the sentinel check so a transient sentinel
    // deletion cannot kill an already-running server.
    if let Some(recorded) = state.read_server() {
        if process::identity_matches(recorded.pid, recorded.start_time) {
            if let Some(url) = recorded.url.filter(|u| is_loopback_url(u)) {
                println!("**Visualiser URL**: {url}");
                return Ok(ExitCode::SUCCESS);
            }
        }
        state.remove_lifecycle_files();
    }

    if !prepared.init_sentinel().exists() {
        return Ok(fail(serde_json::json!({
            "error": "accelerator not initialised",
            "hint": format!(
                "run /accelerator:init in {} before launching the visualiser",
                prepared.project_root.display()
            ),
            "project_root": prepared.project_root.display().to_string(),
        })));
    }
    if prepared.has_ticket_migration_debt()? {
        return Ok(fail(serde_json::json!({
            "error": "project predates the tickets→work-items rename",
            "hint": "run /accelerator:migrate to apply 0001-rename-tickets-to-work before launching the visualiser",
        })));
    }

    std::fs::create_dir_all(state.path())?;
    set_dir_private(state.path());

    let Some(_lock) = lock::LaunchLock::try_acquire(&state.lock_path())? else {
        return Ok(fail(serde_json::json!({
            "error": "another launcher is running",
            "hint": format!("wait for it to finish, or check {} for a stale lock", state.path().display()),
        })));
    };

    state.remove_stopped();
    state.clean_stale_temps();

    let owner_pid =
        resolve_owner_pid(owner_pid_flag, || nix::unistd::getppid().as_raw());
    let owner_start_time = if owner_pid > 0 {
        process::process_start_time(owner_pid)
    } else {
        None
    };

    spawn_daemon(&prepared, owner_pid, owner_start_time)?;

    if !wait_for_readiness(state) {
        return Ok(fail(serde_json::json!({
            "error": "server-info.json did not appear within 5s",
            "log": state.bootstrap_log_path().display().to_string(),
        })));
    }
    match state.read_server().and_then(|s| s.url) {
        Some(url) if is_loopback_url(&url) => {
            println!("**Visualiser URL**: {url}");
            Ok(ExitCode::SUCCESS)
        }
        other => Ok(fail(serde_json::json!({
            "error": "server-info.json contained an invalid url",
            "url": other,
        }))),
    }
}

fn spawn_daemon(
    prepared: &Prepared,
    owner_pid: i32,
    owner_start_time: Option<u64>,
) -> Result<(), OrchestrationError> {
    let exe = std::env::current_exe()?;
    let bootstrap = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(prepared.state_dir.bootstrap_log_path())?;
    let bootstrap_err = bootstrap.try_clone()?;

    let mut command = Command::new(exe);
    command
        .arg("serve")
        .arg("--owner-pid")
        .arg(owner_pid.to_string());
    if let Some(start_time) = owner_start_time {
        command
            .arg("--owner-start-time")
            .arg(start_time.to_string());
    }
    command
        .current_dir(&prepared.project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(bootstrap))
        .stderr(Stdio::from(bootstrap_err));
    // Detach into a new session so the daemon outlives the invoking shell.
    unsafe {
        use std::os::unix::process::CommandExt as _;
        command.pre_exec(|| {
            nix::unistd::setsid()
                .map(|_| ())
                .map_err(std::io::Error::from)
        });
    }
    command.spawn()?;
    Ok(())
}

fn wait_for_readiness(state: &StateDir) -> bool {
    let deadline = std::time::Instant::now() + READINESS_BUDGET;
    loop {
        if state.info_path().exists() {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return state.info_path().exists();
        }
        std::thread::sleep(READINESS_TICK);
    }
}

fn set_dir_private(dir: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    let _ =
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
}

pub fn stop_command() -> ExitCode {
    match std::env::current_dir()
        .map_err(OrchestrationError::from)
        .and_then(|cwd| Prepared::discover(&cwd))
    {
        Ok(prepared) => stop_inner(&prepared.state_dir),
        Err(error) => fail(serde_json::json!({ "error": error.to_string() })),
    }
}

fn stop_inner(state: &StateDir) -> ExitCode {
    let Some(recorded) = state.read_server() else {
        print_json(&serde_json::json!({ "status": "not_running" }));
        return ExitCode::SUCCESS;
    };

    if !process::is_alive(recorded.pid) {
        state.remove_lifecycle_files();
        print_json(&serde_json::json!({
            "status": "stopped",
            "note": "pid was already dead",
        }));
        return ExitCode::SUCCESS;
    }

    // Recycle guard: refuse to signal a pid whose start-time no longer matches
    // the recorded server identity (a reused pid belonging to another process).
    if let Some(expected) = recorded.start_time {
        let current = process::process_start_time(recorded.pid);
        if current != Some(expected) {
            state.remove_lifecycle_files();
            return fail(serde_json::json!({
                "status": "refused",
                "reason": "pid identity mismatch — not killing an unrelated process",
                "pid": recorded.pid,
                "expected_start_time": expected,
                "actual_start_time": current,
            }));
        }
    }

    match process::terminate(recorded.pid, STOP_GRACE, STOP_TICK) {
        process::Termination::Failed => fail(serde_json::json!({
            "status": "failed",
            "error": "process still running after SIGKILL",
            "pid": recorded.pid,
        })),
        outcome => {
            let forced = outcome == process::Termination::Forced;
            // Uphold the post-shutdown invariant: server-stopped.json must
            // exist. A graceful SIGTERM lets the server write its own; a forced
            // kill (or a fake that never wrote one) is synthesised here.
            if forced || !state.stopped_path().exists() {
                let _ = state.write_forced_stopped();
            }
            state.remove_lifecycle_files();
            let mut body = serde_json::json!({ "status": "stopped" });
            if forced {
                body["forced"] = serde_json::Value::Bool(true);
            }
            print_json(&body);
            ExitCode::SUCCESS
        }
    }
}

pub fn status_command() -> ExitCode {
    match std::env::current_dir()
        .map_err(OrchestrationError::from)
        .and_then(|cwd| Prepared::discover(&cwd))
    {
        Ok(prepared) => {
            print_json(&status_body(&prepared.state_dir));
            ExitCode::SUCCESS
        }
        Err(error) => fail(serde_json::json!({ "error": error.to_string() })),
    }
}

fn status_body(state: &StateDir) -> serde_json::Value {
    match state.read_server() {
        Some(recorded)
            if process::identity_matches(recorded.pid, recorded.start_time) =>
        {
            serde_json::json!({
                "status": "running",
                "url": recorded.url,
                "pid": recorded.pid,
            })
        }
        _ => serde_json::json!({ "status": "stopped" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_pid_prefers_explicit_flag() {
        assert_eq!(resolve_owner_pid(Some(4242), || 99), 4242);
    }

    #[test]
    fn owner_pid_falls_back_to_parent_under_exec_replace() {
        assert_eq!(resolve_owner_pid(None, || 99), 99);
        // A non-positive explicit flag is ignored in favour of the parent.
        assert_eq!(resolve_owner_pid(Some(0), || 99), 99);
    }

    #[test]
    fn loopback_url_recognises_genuine_loopback_only() {
        assert!(is_loopback_url("http://127.0.0.1:8080"));
        assert!(is_loopback_url("http://127.0.0.1:8080/"));
        assert!(!is_loopback_url("http://127.0.0.1"));
        assert!(!is_loopback_url("http://127.0.0.1:"));
        assert!(!is_loopback_url("http://evil.example:80"));
        assert!(!is_loopback_url("http://127.0.0.1:80x"));
    }
}

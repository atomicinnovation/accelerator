//! The `executor` command layer: probe, resolve, compose, launch.
//!
//! Nothing here decides anything. The availability ordering, the downgrade
//! vocabulary and the launcher's verdicts all live in the domain; this assembles
//! the ports — the platform probe, the `cache ensure` adapter, the browser hatch
//! and the sticky-failure marker — and maps the one outcome the domain cannot
//! express, a process exit, onto a process exit.

use std::cell::RefCell;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use design::executor::envelope::LauncherError;
use design::executor::forwardable;
use design::executor::launch::LaunchFailure;
use design::executor::launch::Launcher;
use design::executor::ports::PathResolution as _;
use design::runtime::availability;
use design::runtime::availability::BrowserOutcome;
use design::runtime::availability::Resolution;
use design::runtime::availability::Runtime;
use design::runtime::availability::RuntimeOutcome;
use design::runtime::ensure::classify_cause;
use design::runtime::marker;
use design::runtime::marker::Marker;
use design::runtime::platform;
use design::DowngradeReason;
use design_adapters::ensure::discover_launcher;
use design_adapters::ensure::ensure as run_ensure;
use design_adapters::ensure::EnsureOutcome;
use design_adapters::marker::current_session;
use design_adapters::marker::MarkerStore;
use design_adapters::paths::HostPaths;
use design_adapters::platform::observe;
use design_adapters::process::BootstrapLog;
use design_adapters::process::DaemonSpawner;
use design_adapters::process::ExecClient;
use design_adapters::FileLock;
use design_adapters::HostControl;
use design_adapters::HostProbe;
use design_adapters::MonotonicClock;
use design_adapters::StateDirectory;

/// The runner the launcher hands every command to.
const RUNNER: &str = "skills/design/inventory-design/scripts/playwright/run.js";
const DAEMON_COMMAND: &str = "daemon";

/// The driver tree's own Node binary, so the daemon never shells out to a
/// system `node` — the prerequisite the vendored runtime exists to remove.
const NODE_BASENAME: &str = "node";
/// The bundled browser's executable, beside the browser tree root.
const CHROMIUM_SHELL: &str = "chrome-headless-shell";

/// The two vendored trees, matching the launcher's compiled-in set.
const ARTIFACT_DRIVER: &str = "driver";
const ARTIFACT_BROWSER: &str = "browser";

/// Where the vendored modules and the launch browser are resolved from.
const NODE_PATH_VAR: &str = "NODE_PATH";
const STATE_DIR_VAR: &str = "ACCELERATOR_PLAYWRIGHT_STATE_DIR";
const NS_ROOT_VAR: &str = "ACCELERATOR_PLAYWRIGHT_NS_ROOT";
const BROWSER_EXECUTABLE_VAR: &str = "ACCELERATOR_DESIGN_BROWSER_EXECUTABLE";

/// A crawl is bounded at five minutes, so a marker of that order suppresses
/// within-crawl retries without stranding the next crawl.
const MARKER_TTL_SECONDS: u64 = 300;

/// Everything resolved from the repository before the runtime is.
struct Resolved {
    paths: HostPaths,
    state_dir: PathBuf,
    repository_root: PathBuf,
    cwd: PathBuf,
}

/// The runtime the daemon runs against, resolved together so program and
/// environment thread to both spawn sites as one value.
struct ResolvedRuntime {
    node: PathBuf,
    namespace_root: PathBuf,
    browser_executable: PathBuf,
}

/// The browser tree the ensure step materialised, shared from the runtime thunk
/// into the browser thunk without either owning the other.
#[derive(Default)]
struct Ensured {
    browser_tree: Option<PathBuf>,
}

/// Runs `accelerator design executor <command> [json-args]`.
///
/// Returns the process exit status. A host that cannot run the vendored runtime
/// is a downgrade the caller renders and decides on; every other failure the
/// caller can act on is a three-key envelope on stderr.
#[must_use]
pub fn run(command: &str, arguments: &[String]) -> ExitCode {
    // Validated before anything is resolved: a rejected command must not create
    // a state directory or touch a lock.
    if let Err(rejection) = forwardable::check(command) {
        eprintln!("error: {rejection}");
        return ExitCode::from(2);
    }

    let resolved = match resolve() {
        Ok(resolved) => resolved,
        Err(failure) => return report(failure),
    };

    let hatch = match crate::config::resolve_browser_hatch(
        &resolved.cwd,
        &resolved.repository_root,
    ) {
        Ok(hatch) => hatch,
        Err(failure) => return report(failure),
    };
    for warning in &hatch.warnings {
        eprintln!("warning: {warning}");
    }

    let markers = MarkerStore::in_state_dir(&resolved.state_dir);
    let session = current_session();
    let now = now_seconds();

    match resolve_runtime(&resolved, &hatch, &markers, &session, now) {
        Resolution::Downgrade(reason) => report_downgrade(reason),
        Resolution::Ready(runtime) => {
            let digest = runtime.driver.to_string_lossy().into_owned();
            // The expensive spawn is skipped when a prior host-condition failure
            // for this same tree is still on record.
            if let Some(reason) = markers.read().and_then(|recorded| {
                marker::suppresses(
                    &recorded,
                    &session,
                    now,
                    MARKER_TTL_SECONDS,
                    Some(&digest),
                )
            }) {
                return report_downgrade(reason);
            }
            act_on(
                &resolved, &runtime, &markers, &session, now, command,
                arguments,
            )
        }
    }
}

/// Resolve the runtime crawler's preconditions in order — platform,
/// then the runtime, then the browser — over lazily-evaluated thunks, so an
/// unsupported host reaches neither the fetch nor the browser resolution.
fn resolve_runtime(
    resolved: &Resolved,
    hatch: &design::runtime::browser_path::HatchDecision,
    markers: &MarkerStore,
    session: &str,
    now: u64,
) -> Resolution {
    let support = platform::classify(&observe());
    let plugin_root = resolved.paths.plugin_root().ok();
    let want_browser = hatch.browser.is_none();
    let trees: Vec<&str> = if want_browser {
        vec![ARTIFACT_DRIVER, ARTIFACT_BROWSER]
    } else {
        vec![ARTIFACT_DRIVER]
    };
    let ensured = RefCell::new(Ensured::default());

    availability::resolve(
        support,
        || {
            ensure_runtime(
                &ensured,
                markers,
                session,
                now,
                plugin_root.as_deref(),
                &trees,
                want_browser,
            )
        },
        || resolve_browser(hatch, &ensured),
    )
}

/// The runtime thunk: the warm launcher-exported trees when present, otherwise a
/// cold `cache ensure` guarded by the sticky-failure marker.
fn ensure_runtime(
    ensured: &RefCell<Ensured>,
    markers: &MarkerStore,
    session: &str,
    now: u64,
    plugin_root: Option<&Path>,
    trees: &[&str],
    want_browser: bool,
) -> RuntimeOutcome {
    if let Some(driver) = tree_from_env(ARTIFACT_DRIVER) {
        if !want_browser {
            return RuntimeOutcome::Ready(driver);
        }
        if let Some(browser) = tree_from_env(ARTIFACT_BROWSER) {
            ensured.borrow_mut().browser_tree = Some(browser);
            return RuntimeOutcome::Ready(driver);
        }
        // The driver is exported but the browser is not, so fall through and let
        // the cold path materialise the browser.
    }

    if let Some(reason) = markers.read().and_then(|recorded| {
        marker::suppresses(&recorded, session, now, MARKER_TTL_SECONDS, None)
    }) {
        return RuntimeOutcome::Downgrade(reason);
    }

    let Some(launcher) = discover_launcher(plugin_root) else {
        return RuntimeOutcome::Downgrade(DowngradeReason::ArtifactUnavailable);
    };

    match run_ensure(&launcher, trees) {
        EnsureOutcome::Ready(resolved) => {
            if markers.read().is_some_and(|recorded| {
                marker::cleared_by_successful_ensure(&recorded)
            }) {
                markers.clear();
            }
            let Some(driver) = resolved
                .iter()
                .find(|tree| tree.artifact == ARTIFACT_DRIVER)
                .map(|tree| tree.path.clone())
            else {
                return RuntimeOutcome::Downgrade(
                    DowngradeReason::ArtifactUnavailable,
                );
            };
            if let Some(browser) = resolved
                .iter()
                .find(|tree| tree.artifact == ARTIFACT_BROWSER)
            {
                ensured.borrow_mut().browser_tree = Some(browser.path.clone());
            }
            RuntimeOutcome::Ready(driver)
        }
        EnsureOutcome::Failed(cause) => {
            let verdict = classify_cause(&cause);
            if verdict.sticky {
                markers.write(&Marker {
                    reason: verdict.reason,
                    session: session.to_owned(),
                    recorded_at: now,
                    digest: None,
                });
            }
            RuntimeOutcome::Downgrade(verdict.reason)
        }
    }
}

/// The browser thunk: the hatch when set, otherwise the bundled shell beside the
/// materialised browser tree.
fn resolve_browser(
    hatch: &design::runtime::browser_path::HatchDecision,
    ensured: &RefCell<Ensured>,
) -> BrowserOutcome {
    if let Some(path) = &hatch.browser {
        return BrowserOutcome::Hatch(path.clone());
    }
    ensured.borrow().browser_tree.as_ref().map_or(
        BrowserOutcome::Downgrade(DowngradeReason::ArtifactUnavailable),
        |tree| BrowserOutcome::Bundled(tree.join(CHROMIUM_SHELL)),
    )
}

/// Launch against the resolved runtime, recording a host-condition downgrade so
/// the rest of the crawl skips the spawn.
fn act_on(
    resolved: &Resolved,
    runtime: &Runtime,
    markers: &MarkerStore,
    session: &str,
    now: u64,
    command: &str,
    arguments: &[String],
) -> ExitCode {
    let digest = runtime.driver.to_string_lossy().into_owned();
    match launch(resolved, runtime, command, arguments) {
        Ok(never) => match never {},
        Err(LaunchFailure::Downgrade(reason)) => {
            if marker::is_host_condition(reason) {
                markers.write(&Marker {
                    reason,
                    session: session.to_owned(),
                    recorded_at: now,
                    digest: Some(digest),
                });
            }
            report_downgrade(reason)
        }
        Err(failure) => report(failure),
    }
}

/// The state directory and the repository, from where the caller is.
fn resolve() -> Result<Resolved, LaunchFailure> {
    let cwd = std::env::current_dir().map_err(|error| {
        LaunchFailure::Failed(kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        )))
    })?;

    let Some(facts) = vcs_adapters::facts(&cwd) else {
        return Err(LaunchFailure::Envelope(LauncherError::NoRepo));
    };

    let tmp_relative = crate::config::resolve_tmp_dir(&cwd)?;
    let state_dir =
        HostPaths::state_dir_for(&facts.root, Path::new(&tmp_relative));
    create_state_dir(&state_dir)?;

    Ok(Resolved {
        paths: HostPaths::new(state_dir.clone()),
        state_dir,
        repository_root: facts.root,
        cwd,
    })
}

/// Mode 0700: the directory holds a daemon's URL, its request token and the
/// sticky-failure marker.
fn create_state_dir(state_dir: &Path) -> Result<(), LaunchFailure> {
    use std::os::unix::fs::DirBuilderExt as _;

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(state_dir)
        .map_err(|error| {
            LaunchFailure::Failed(kernel::Error::Failed(format!(
                "could not create the state directory {}: {error}",
                state_dir.display()
            )))
        })
}

fn launch(
    resolved: &Resolved,
    runtime: &Runtime,
    command: &str,
    arguments: &[String],
) -> Result<std::convert::Infallible, LaunchFailure> {
    let plugin_root = resolved
        .paths
        .plugin_root()
        .map_err(LaunchFailure::Failed)?;
    let runner = plugin_root.join(RUNNER);
    let bootstrap_log = resolved
        .paths
        .bootstrap_log()
        .map_err(LaunchFailure::Failed)?;

    let vendored = ResolvedRuntime {
        node: runtime.driver.join(NODE_BASENAME),
        namespace_root: runtime.driver.clone(),
        browser_executable: runtime.browser_executable.clone(),
    };
    let environment = vec![
        (
            STATE_DIR_VAR.to_owned(),
            resolved.state_dir.display().to_string(),
        ),
        (
            NODE_PATH_VAR.to_owned(),
            vendored
                .namespace_root
                .join("node_modules")
                .display()
                .to_string(),
        ),
        (
            NS_ROOT_VAR.to_owned(),
            vendored.namespace_root.display().to_string(),
        ),
        (
            BROWSER_EXECUTABLE_VAR.to_owned(),
            vendored.browser_executable.display().to_string(),
        ),
    ];

    let clock = MonotonicClock::default();
    let state = StateDirectory::new(resolved.state_dir.clone());
    let diagnostics = BootstrapLog {
        path: bootstrap_log.clone(),
    };
    let lock = FileLock::open(&resolved.state_dir.join("launcher.lock"))
        .map_err(LaunchFailure::Failed)?;
    let spawner = DaemonSpawner {
        program: vendored.node.clone(),
        arguments: vec![
            runner.display().to_string(),
            DAEMON_COMMAND.to_owned(),
            "--state-dir".to_owned(),
            resolved.state_dir.display().to_string(),
        ],
        bootstrap_log: bootstrap_log.clone(),
        environment: environment.clone(),
    };
    let client = ExecClient {
        program: vendored.node,
        leading_arguments: vec![runner.display().to_string()],
        environment,
    };

    let launcher = Launcher {
        clock: &clock,
        probe: &HostProbe,
        state: &state,
        lock: &lock,
        spawner: &spawner,
        control: &HostControl,
        diagnostics: &diagnostics,
        bootstrap_log: bootstrap_log.display().to_string(),
    };

    let mut forwarded = vec![command.to_owned()];
    forwarded.extend_from_slice(arguments);
    launcher.launch(&forwarded, Box::new(client))
}

/// The tree path the launcher exported for an artifact on the warm path.
fn tree_from_env(artifact: &str) -> Option<PathBuf> {
    let variable = format!("ACCELERATOR_TREE_{}", artifact.to_uppercase());
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// The reason token only; the caller renders the message through
/// `notify-downgrade` and decides — code-only for the default and hybrid
/// crawlers, a hard failure for an explicit `--crawler runtime`.
fn report_downgrade(reason: DowngradeReason) -> ExitCode {
    eprintln!(r#"{{"error":"downgrade","reason":"{}"}}"#, reason.key());
    ExitCode::from(3)
}

/// Launcher envelopes reach stderr with the exit code the envelope names;
/// anything else is an internal failure at exit 1.
///
/// Daemon-side errors never reach here at all: the client owns the process by
/// then, so its envelope goes to stdout at exit 0. The skill discriminates on
/// exactly that asymmetry.
fn report(failure: LaunchFailure) -> ExitCode {
    match failure {
        LaunchFailure::Envelope(envelope) => {
            eprintln!("{}", envelope.render());
            ExitCode::from(envelope.exit_code())
        }
        LaunchFailure::Downgrade(reason) => report_downgrade(reason),
        LaunchFailure::Failed(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use design::executor::forwardable;

    use super::DAEMON_COMMAND;

    /// The forwarding allowlist must reject the runner's own internal
    /// subcommand, because arguments are forwarded verbatim.
    #[test]
    fn the_daemon_command_is_not_forwardable() {
        assert!(forwardable::check(DAEMON_COMMAND).is_err());
    }
}

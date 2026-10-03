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
use design::executor::DaemonBrowser;
use design::runtime::availability;
use design::runtime::availability::BrowserOutcome;
use design::runtime::availability::Resolution;
use design::runtime::availability::Runtime;
use design::runtime::availability::RuntimeOutcome;
use design::runtime::ensure::classify_cause;
use design::runtime::marker;
use design::runtime::marker::Marker;
use design::runtime::platform;
use design::Allowances;
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

/// Where the caller is, before the browser is known.
struct Located {
    cwd: PathBuf,
    repository_root: PathBuf,
}

/// Everything resolved from the repository and the browser before the runtime
/// is.
struct Resolved {
    paths: HostPaths,
    state_dir: PathBuf,
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
pub fn run(
    command: &str,
    arguments: &[String],
    allowances: Allowances,
) -> ExitCode {
    // Validated before anything is resolved: a rejected command must not create
    // a state directory or touch a lock.
    if let Err(rejection) = forwardable::check(command) {
        eprintln!("error: {rejection}");
        return ExitCode::from(2);
    }

    let arguments = match merge_allowances(arguments, allowances) {
        Ok(merged) => merged,
        Err(rejection) => {
            eprintln!("error: {rejection}");
            return ExitCode::from(2);
        }
    };

    let located = match locate() {
        Ok(located) => located,
        Err(failure) => return report(failure),
    };

    let hatch = match crate::config::browser_hatch(&located.cwd) {
        Ok(hatch) => hatch,
        Err(failure) => return report(failure),
    };
    for warning in &hatch.warnings {
        eprintln!("warning: {warning}");
    }
    if let Some(notice) = &hatch.notice {
        eprintln!("{notice}");
    }

    let resolved = match resolve(&located, &hatch) {
        Ok(resolved) => resolved,
        Err(failure) => return report(failure),
    };

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
                &arguments,
            )
        }
    }
}

/// Injects the invocation's allowances into the request body the executor
/// forwards, so the daemon classifies each navigation under them. The body is
/// `arguments[0]` when present; a command that carries none (`ping`,
/// `daemon-stop`) gets a synthesised empty object.
///
/// # Errors
///
/// A message when the body is not a JSON object, or already carries either
/// allowance key — a page-influenced payload must not pre-set its own
/// allowance, mirroring the `command`/`protocol` guard in the Node client.
fn merge_allowances(
    arguments: &[String],
    allowances: Allowances,
) -> Result<Vec<String>, String> {
    let raw = arguments.first().map_or("{}", String::as_str);
    let mut body: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| {
            format!("executor request body is not valid JSON: {error}")
        })?;
    let object = body.as_object_mut().ok_or_else(|| {
        "executor request body must be a JSON object".to_owned()
    })?;

    for key in ["allow_internal", "allow_insecure_scheme"] {
        if object.contains_key(key) {
            return Err(format!(
                "executor request body must not carry its own `{key}` field"
            ));
        }
    }
    object.insert("allow_internal".to_owned(), allowances.internal.into());
    object.insert(
        "allow_insecure_scheme".to_owned(),
        allowances.insecure_scheme.into(),
    );

    let merged =
        serde_json::to_string(&body).map_err(|error| error.to_string())?;
    let mut result = vec![merged];
    result.extend(arguments.iter().skip(1).cloned());
    Ok(result)
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

fn locate() -> Result<Located, LaunchFailure> {
    let cwd = std::env::current_dir().map_err(|error| {
        LaunchFailure::Failed(kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        )))
    })?;

    let Some(facts) = vcs_adapters::facts(&cwd) else {
        return Err(LaunchFailure::Envelope(LauncherError::NoRepo));
    };

    Ok(Located {
        cwd,
        repository_root: facts.root,
    })
}

/// The state directory of the daemon running the browser the hatch chose.
fn resolve(
    located: &Located,
    hatch: &design::runtime::browser_path::HatchDecision,
) -> Result<Resolved, LaunchFailure> {
    let tmp_relative = crate::config::resolve_tmp_dir(&located.cwd)?;
    let state_dir = HostPaths::state_dir_for(
        &located.repository_root,
        Path::new(&tmp_relative),
        &DaemonBrowser::chosen_by(hatch),
    )?;
    create_state_dir(&state_dir)?;

    Ok(Resolved {
        paths: HostPaths::new(state_dir.clone()),
        state_dir,
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

    let environment = runtime_environment(resolved, runtime);
    let clock = MonotonicClock::default();
    let state = StateDirectory::new(resolved.state_dir.clone());
    let diagnostics = BootstrapLog {
        path: bootstrap_log.clone(),
    };
    let lock = FileLock::open(&resolved.state_dir.join("launcher.lock"))
        .map_err(LaunchFailure::Failed)?;
    let spawner = daemon_spawner(resolved, runtime, &runner, &bootstrap_log);
    let client = ExecClient {
        program: ResolvedRuntime::from(runtime).node,
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

impl From<&Runtime> for ResolvedRuntime {
    fn from(runtime: &Runtime) -> Self {
        Self {
            node: runtime.driver.join(NODE_BASENAME),
            namespace_root: runtime.driver.clone(),
            browser_executable: runtime.browser_executable.clone(),
        }
    }
}

/// What the daemon and the client both learn of the runtime and the state
/// directory.
fn runtime_environment(
    resolved: &Resolved,
    runtime: &Runtime,
) -> Vec<(String, String)> {
    let vendored = ResolvedRuntime::from(runtime);
    vec![
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
    ]
}

fn daemon_spawner(
    resolved: &Resolved,
    runtime: &Runtime,
    runner: &Path,
    bootstrap_log: &Path,
) -> DaemonSpawner {
    DaemonSpawner {
        program: ResolvedRuntime::from(runtime).node,
        arguments: vec![
            runner.display().to_string(),
            DAEMON_COMMAND.to_owned(),
            "--state-dir".to_owned(),
            resolved.state_dir.display().to_string(),
        ],
        bootstrap_log: bootstrap_log.to_path_buf(),
        environment: runtime_environment(resolved, runtime),
    }
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
    #![allow(clippy::unwrap_used)]

    use std::cell::RefCell;
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::path::PathBuf;

    use design::executor::forwardable;
    use design::runtime::availability::BrowserOutcome;
    use design::runtime::availability::Runtime;
    use design::runtime::browser_path::HatchDecision;
    use design::Allowances;
    use design_adapters::process::DaemonSpawner;

    use super::daemon_spawner;
    use super::merge_allowances;
    use super::resolve;
    use super::resolve_browser;
    use super::Ensured;
    use super::Located;
    use super::BROWSER_EXECUTABLE_VAR;
    use super::CHROMIUM_SHELL;
    use super::DAEMON_COMMAND;

    struct Project {
        work: tempfile::TempDir,
        repository_root: PathBuf,
    }

    impl Project {
        fn new() -> Self {
            let work = tempfile::tempdir().unwrap();
            let repository_root = work.path().join("repo");
            std::fs::create_dir_all(repository_root.join(".git")).unwrap();
            Self {
                work,
                repository_root,
            }
        }

        fn outside_symlink_to_chrome(&self) -> PathBuf {
            let chrome = self.work.path().join("outside/real/chrome");
            std::fs::create_dir_all(chrome.parent().unwrap()).unwrap();
            std::fs::write(&chrome, "").unwrap();
            let link = self.work.path().join("outside/chrome");
            symlink(&chrome, &link).unwrap();
            link
        }

        fn state_dir(&self, hatch: &HatchDecision) -> PathBuf {
            self.resolved(hatch).state_dir
        }

        fn resolved(&self, hatch: &HatchDecision) -> super::Resolved {
            resolve(
                &Located {
                    cwd: self.repository_root.clone(),
                    repository_root: self.repository_root.clone(),
                },
                hatch,
            )
            .unwrap_or_else(|_| unreachable!("the project resolves"))
        }

        fn spawner_for(&self, hatch: &HatchDecision) -> DaemonSpawner {
            let ensured = RefCell::new(Ensured {
                browser_tree: Some(self.work.path().join("browser-tree")),
            });
            let (BrowserOutcome::Hatch(browser_executable)
            | BrowserOutcome::Bundled(browser_executable)) =
                resolve_browser(hatch, &ensured)
            else {
                unreachable!("a browser tree is ensured");
            };
            let runtime = Runtime {
                driver: self.work.path().join("driver-tree"),
                browser_executable,
            };
            let resolved = self.resolved(hatch);
            let bootstrap_log = resolved.state_dir.join("server.bootstrap.log");
            daemon_spawner(
                &resolved,
                &runtime,
                Path::new("/plugin/run.js"),
                &bootstrap_log,
            )
        }
    }

    fn hatch(browser: Option<PathBuf>) -> HatchDecision {
        HatchDecision {
            browser,
            warnings: Vec::new(),
            notice: None,
        }
    }

    fn browser_executable(spawner: &DaemonSpawner) -> Option<&str> {
        spawner
            .environment
            .iter()
            .find(|(name, _)| name == BROWSER_EXECUTABLE_VAR)
            .map(|(_, value)| value.as_str())
    }

    fn slot(spawner: &DaemonSpawner) -> String {
        let state_dir = spawner
            .arguments
            .iter()
            .skip_while(|argument| *argument != "--state-dir")
            .nth(1)
            .unwrap();
        Path::new(state_dir)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn executors_with_different_hatches_resolve_different_state_directories() {
        let project = Project::new();
        let chrome =
            project.outside_symlink_to_chrome().canonicalize().unwrap();

        assert_ne!(
            project.state_dir(&hatch(None)),
            project.state_dir(&hatch(Some(chrome.clone())))
        );
        assert_eq!(
            project.state_dir(&hatch(Some(chrome.clone()))),
            project.state_dir(&hatch(Some(chrome)))
        );
        assert_eq!(
            project.state_dir(&hatch(None)),
            project.state_dir(&hatch(None))
        );
    }

    #[test]
    fn an_outside_symlink_hatch_spawns_its_canonical_target_in_a_custom_slot() {
        let project = Project::new();
        let target =
            project.outside_symlink_to_chrome().canonicalize().unwrap();

        let spawner = project.spawner_for(&hatch(Some(target.clone())));

        assert_eq!(browser_executable(&spawner), target.to_str());
        assert!(slot(&spawner).starts_with("custom-"), "{}", slot(&spawner));
    }

    #[test]
    fn a_refused_hatch_spawns_the_bundled_browser_in_the_bundled_slot() {
        let project = Project::new();

        let spawner = project.spawner_for(&hatch(None));

        assert_eq!(
            browser_executable(&spawner),
            project
                .work
                .path()
                .join("browser-tree")
                .join(CHROMIUM_SHELL)
                .to_str()
        );
        assert_eq!(slot(&spawner), "bundled");
    }

    /// The forwarding allowlist must reject the runner's own internal
    /// subcommand, because arguments are forwarded verbatim.
    #[test]
    fn the_daemon_command_is_not_forwardable() {
        assert!(forwardable::check(DAEMON_COMMAND).is_err());
    }

    #[test]
    fn the_merge_injects_both_allowance_keys_into_the_body(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let merged = merge_allowances(
            &[r#"{"url":"https://example.com"}"#.to_owned()],
            Allowances {
                internal: true,
                insecure_scheme: false,
            },
        )?;
        let body: serde_json::Value = serde_json::from_str(&merged[0])?;
        assert_eq!(body["url"], serde_json::json!("https://example.com"));
        assert_eq!(body["allow_internal"], serde_json::json!(true));
        assert_eq!(body["allow_insecure_scheme"], serde_json::json!(false));
        Ok(())
    }

    #[test]
    fn the_merge_synthesises_a_body_for_a_command_that_carries_none(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let merged = merge_allowances(&[], Allowances::default())?;
        assert_eq!(merged.len(), 1);
        let body: serde_json::Value = serde_json::from_str(&merged[0])?;
        assert_eq!(body["allow_internal"], serde_json::json!(false));
        assert_eq!(body["allow_insecure_scheme"], serde_json::json!(false));
        Ok(())
    }

    /// A page-influenced payload must not pre-set its own allowance.
    #[test]
    fn the_merge_refuses_a_payload_that_pre_sets_an_allowance() {
        for key in ["allow_internal", "allow_insecure_scheme"] {
            let body = format!(r#"{{"{key}":true}}"#);
            assert!(
                merge_allowances(&[body], Allowances::default()).is_err(),
                "{key}"
            );
        }
    }

    #[test]
    fn the_merge_refuses_a_body_that_is_not_a_json_object() {
        assert!(merge_allowances(
            &["[1,2,3]".to_owned()],
            Allowances::default()
        )
        .is_err());
    }
}

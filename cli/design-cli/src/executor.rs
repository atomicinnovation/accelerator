//! The `executor` command layer: resolve, compose, launch.
//!
//! Nothing here decides anything. The launcher's sequence, its verdicts and its
//! envelope taxonomy all live in the domain; this assembles the ports and maps
//! the one outcome the domain cannot express — a process exit — onto a process
//! exit.

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use design::executor::envelope::LauncherError;
use design::executor::forwardable;
use design::executor::launch::LaunchFailure;
use design::executor::launch::Launcher;
use design::executor::ports::PathResolution as _;
use design_adapters::paths::HostPaths;
use design_adapters::process::DaemonSpawner;
use design_adapters::process::ExecClient;
use design_adapters::FileLock;
use design_adapters::HostControl;
use design_adapters::HostProbe;
use design_adapters::MonotonicClock;
use design_adapters::StateDirectory;

/// The runner the launcher hands every command to.
const RUNNER: &str = "skills/design/inventory-design/scripts/playwright/run.js";
const NODE: &str = "node";
const DAEMON_COMMAND: &str = "daemon";

/// Where the Playwright runtime's modules are resolved from.
const NODE_PATH_VAR: &str = "NODE_PATH";
const STATE_DIR_VAR: &str = "ACCELERATOR_PLAYWRIGHT_STATE_DIR";
const NS_ROOT_VAR: &str = "ACCELERATOR_PLAYWRIGHT_NS_ROOT";

/// Everything resolved before the launcher runs.
struct Resolved {
    paths: HostPaths,
    state_dir: PathBuf,
    namespace_root: PathBuf,
}

/// Runs `accelerator design executor <command> [json-args]`.
///
/// Returns the process exit status. Every failure the caller can act on is a
/// three-key envelope on stderr; anything else is an internal failure.
#[must_use]
pub fn run(command: &str, arguments: &[String]) -> ExitCode {
    // Validated before anything is resolved: a rejected command must not
    // create a state directory or touch a lock.
    if let Err(rejection) = forwardable::check(command) {
        eprintln!("error: {rejection}");
        return ExitCode::from(2);
    }

    let resolved = match resolve() {
        Ok(resolved) => resolved,
        Err(failure) => return report(failure),
    };

    if !runtime_is_installed(&resolved.namespace_root) {
        return report(LaunchFailure::Envelope(
            LauncherError::PlaywrightNotInstalled {
                namespace_root: resolved.namespace_root.display().to_string(),
            },
        ));
    }

    match launch(&resolved, command, arguments) {
        Ok(never) => match never {},
        Err(failure) => report(failure),
    }
}

/// The state directory and the namespace, from the repository the caller is in.
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

    let paths = HostPaths::new(state_dir.clone());
    let namespace_root =
        paths.namespace_root().map_err(LaunchFailure::Failed)?;
    Ok(Resolved {
        paths,
        state_dir,
        namespace_root,
    })
}

/// Mode 0700, matching the shell: the directory holds a daemon's URL and its
/// request token.
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

fn runtime_is_installed(namespace_root: &Path) -> bool {
    namespace_root
        .join("node_modules/playwright/package.json")
        .is_file()
}

fn launch(
    resolved: &Resolved,
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

    let environment = vec![
        (
            STATE_DIR_VAR.to_owned(),
            resolved.state_dir.display().to_string(),
        ),
        (
            NODE_PATH_VAR.to_owned(),
            resolved
                .namespace_root
                .join("node_modules")
                .display()
                .to_string(),
        ),
        (
            NS_ROOT_VAR.to_owned(),
            resolved.namespace_root.display().to_string(),
        ),
    ];

    let clock = MonotonicClock::default();
    let state = StateDirectory::new(resolved.state_dir.clone());
    let lock = FileLock::open(&resolved.state_dir.join("launcher.lock"))
        .map_err(LaunchFailure::Failed)?;
    let spawner = DaemonSpawner {
        program: PathBuf::from(NODE),
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
        program: PathBuf::from(NODE),
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
        bootstrap_log: bootstrap_log.display().to_string(),
    };

    let mut forwarded = vec![command.to_owned()];
    forwarded.extend_from_slice(arguments);
    launcher.launch(&forwarded, Box::new(client))
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
        LaunchFailure::Failed(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use design::executor::forwardable;

    use super::runtime_is_installed;
    use super::DAEMON_COMMAND;

    type TestError = Box<dyn std::error::Error>;

    /// The forwarding allowlist must reject the runner's own internal
    /// subcommand, because arguments are forwarded verbatim.
    #[test]
    fn the_daemon_command_is_not_forwardable() {
        assert!(forwardable::check(DAEMON_COMMAND).is_err());
    }

    #[test]
    fn a_namespace_without_the_runtime_is_not_installed(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        assert!(!runtime_is_installed(work.path()));
        Ok(())
    }

    #[test]
    fn a_namespace_carrying_the_package_manifest_is_installed(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let manifest = work.path().join("node_modules/playwright");
        std::fs::create_dir_all(&manifest)?;
        std::fs::write(manifest.join("package.json"), "{}")?;
        assert!(runtime_is_installed(work.path()));
        Ok(())
    }
}

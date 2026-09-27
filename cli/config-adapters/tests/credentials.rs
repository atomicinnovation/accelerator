//! The credential ladder's production adapters against real files and a real
//! `bash`, and the project-rooted context they are assembled into.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use config::consent::{ConfigFileTracking, Tracking};
use config::credentials::{
    resolve_token, CommandPolicy, CredentialError, Environment, FileFacts,
    FileState, Provenance, ResolvedToken, TokenCommandFailure,
    TokenCommandRunner, TokenKeys,
};
use config::{ConfigError, Key, Level, Resolved, Scalar, Value};
use config_adapters::credentials::{
    project_credential_context, BashTokenCommandRunner, CredentialPorts,
    SystemFileFacts,
};
use tempfile::TempDir;

const SENTINEL: &str = "s3cr3t-sentinel-value";

struct FixedConfig {
    personal: BTreeMap<String, String>,
}

impl config::ConfigAccess for FixedConfig {
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError> {
        let personal = match level {
            Some(Level::Personal) => &self.personal,
            Some(Level::Team) => return Ok(Resolved::Absent),
            None => unreachable!("the ladder always names a level"),
        };
        Ok(personal
            .get(&key.to_string())
            .map_or(Resolved::Absent, |value| {
                Resolved::Found(Value::Scalar(Scalar::String(value.clone())))
            }))
    }

    fn set(
        &self,
        _key: &Key,
        _value: &str,
        _level: Level,
    ) -> Result<(), ConfigError> {
        unreachable!("the ladder never writes")
    }
}

struct FixedEnvironment(BTreeMap<String, String>);

impl Environment for FixedEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

struct Untracked;

impl ConfigFileTracking for Untracked {
    fn tracking(&self, _path: &Path) -> Tracking {
        Tracking::Untracked
    }
}

struct FixedProvenance(Vec<PathBuf>);

impl Provenance for FixedProvenance {
    fn is_tracked(&self, path: &Path) -> bool {
        self.0.iter().any(|tracked| tracked == path)
    }
}

fn keys() -> TokenKeys {
    TokenKeys {
        env: "ACCELERATOR_JIRA_TOKEN",
        env_command: "ACCELERATOR_JIRA_TOKEN_CMD",
        value: Key::parse("jira.token").expect("jira.token parses"),
        command: Key::parse("jira.token_cmd").expect("jira.token_cmd parses"),
    }
}

/// A scratch project with an `.accelerator/` directory.
struct Project {
    root: TempDir,
}

impl Project {
    fn new() -> Self {
        let root = TempDir::new().expect("a scratch project");
        std::fs::create_dir(root.path().join(".accelerator"))
            .expect("create .accelerator");
        Self { root }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn personal_config(&self) -> PathBuf {
        self.path(".accelerator/config.local.md")
    }

    fn marker(&self) -> PathBuf {
        self.path(".accelerator/allow-insecure-local")
    }

    fn write_personal_config(&self, mode: u32) -> PathBuf {
        let path = self.personal_config();
        std::fs::write(&path, "---\njira:\n  token: unused\n---\n")
            .expect("write the personal config");
        set_mode(&path, mode);
        path
    }

    fn resolve(
        &self,
        environment: &[(&str, &str)],
        personal: &[(&str, &str)],
        tracked: &[PathBuf],
        command_timeout: Duration,
    ) -> Result<ResolvedToken, CredentialError> {
        let ports = CredentialPorts {
            environment: Box::new(FixedEnvironment(
                environment
                    .iter()
                    .map(|(name, value)| {
                        ((*name).to_owned(), (*value).to_owned())
                    })
                    .collect(),
            )),
            files: Box::new(SystemFileFacts),
            commands: Box::new(BashTokenCommandRunner),
            provenance: Box::new(FixedProvenance(tracked.to_vec())),
            tracking: Box::new(Untracked),
        };
        let config = FixedConfig {
            personal: personal
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        };
        resolve_token(
            &project_credential_context(
                self.root.path(),
                &ports,
                &config,
                command_timeout,
            ),
            &keys(),
        )
    }

    /// Resolves over the real composed store, so the personal file's mode and
    /// shape decide whether it is read at all.
    fn resolve_composed(
        &self,
        environment: &[(&str, &str)],
    ) -> Result<ResolvedToken, CredentialError> {
        std::fs::create_dir_all(self.path(".git")).expect("mark the root");
        let composed = config_adapters::compose(
            self.root.path(),
            config_adapters::LegacyPolicy::Reject,
        )
        .expect("the project composes");
        let ports = CredentialPorts {
            environment: Box::new(FixedEnvironment(
                environment
                    .iter()
                    .map(|(name, value)| {
                        ((*name).to_owned(), (*value).to_owned())
                    })
                    .collect(),
            )),
            files: Box::new(SystemFileFacts),
            commands: Box::new(BashTokenCommandRunner),
            provenance: Box::new(FixedProvenance(vec![self.marker()])),
            tracking: Box::new(Untracked),
        };
        resolve_token(
            &project_credential_context(
                self.root.path(),
                &ports,
                &composed.service,
                Duration::from_secs(5),
            ),
            &keys(),
        )
    }
}

fn policy(project: &Project) -> CommandPolicy {
    CommandPolicy {
        timeout: Duration::from_secs(5),
        max_output_bytes: 1024,
        working_directory: project.root.path().to_path_buf(),
    }
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .expect("set the mode");
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) {}

#[test]
fn the_context_reads_the_projects_personal_config_and_marker() {
    let project = Project::new();
    let ports = CredentialPorts::system(
        Box::new(FixedProvenance(Vec::new())),
        Box::new(Untracked),
    );
    let config = FixedConfig {
        personal: BTreeMap::new(),
    };

    let context = project_credential_context(
        project.root.path(),
        &ports,
        &config,
        Duration::from_secs(12),
    );

    assert_eq!(context.personal_config, project.personal_config());
    assert_eq!(context.insecure_marker, project.marker());
    assert_eq!(context.command.timeout, Duration::from_secs(12));
    assert_eq!(context.command.working_directory, project.root.path());
    assert_eq!(
        context.command.max_output_bytes,
        CommandPolicy::rooted_at(PathBuf::new()).max_output_bytes
    );
}

#[test]
fn a_missing_path_is_absent() {
    let project = Project::new();

    let state = SystemFileFacts.inspect(&project.personal_config());

    assert_eq!(state, Ok(FileState::Absent));
}

#[cfg(unix)]
#[test]
fn a_regular_file_reports_its_mode() {
    let project = Project::new();
    let path = project.write_personal_config(0o640);

    let state = SystemFileFacts.inspect(&path);

    assert_eq!(state, Ok(FileState::File { mode: 0o640 }));
}

#[test]
fn a_directory_is_neither_absent_nor_a_file() {
    let project = Project::new();

    let state = SystemFileFacts.inspect(&project.path(".accelerator"));

    assert_eq!(state, Ok(FileState::Other));
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_file_is_a_symlink() {
    let project = Project::new();
    let target = project.path("config.local.md.real");
    std::fs::write(&target, "").expect("write the target");
    std::os::unix::fs::symlink(&target, project.personal_config())
        .expect("symlink the personal config");

    let state = SystemFileFacts.inspect(&project.personal_config());

    assert_eq!(state, Ok(FileState::Symlink));
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_is_absent() {
    let project = Project::new();
    std::os::unix::fs::symlink(
        project.path("nowhere"),
        project.personal_config(),
    )
    .expect("symlink the personal config");

    let state = SystemFileFacts.inspect(&project.personal_config());

    assert_eq!(state, Ok(FileState::Absent));
}

#[test]
fn an_insecure_personal_config_is_ignored_and_refused_when_nothing_remains() {
    let project = Project::new();
    project.write_personal_config(0o644);

    let error = project
        .resolve_composed(&[])
        .expect_err("an ignored credential file yields nothing usable");

    assert!(matches!(error, CredentialError::LocalPermsInsecure { .. }));
    assert!(error.to_string().contains("chmod 600"), "{error}");
}

#[test]
fn the_environment_token_resolves_beside_an_insecure_personal_config() {
    let project = Project::new();
    project.write_personal_config(0o644);

    let resolved = project
        .resolve_composed(&[("ACCELERATOR_JIRA_TOKEN", "from-env")])
        .expect("the environment is unaffected by an ignored file");

    assert_eq!(resolved.value.expose(), "from-env");
}

#[test]
fn the_insecure_override_does_not_unlock_an_ignored_personal_config() {
    let project = Project::new();
    project.write_personal_config(0o640);
    std::fs::write(project.marker(), "").expect("write the marker");

    let error = project
        .resolve_composed(&[("ACCELERATOR_ALLOW_INSECURE_LOCAL", "1")])
        .expect_err("the variable and a tracked marker unlock nothing");

    assert!(matches!(error, CredentialError::LocalPermsInsecure { .. }));
}

#[cfg(unix)]
#[test]
fn a_symlinked_personal_config_is_ignored() {
    let project = Project::new();
    let target = project.path("config.local.md.real");
    std::fs::write(&target, "---\njira:\n  token: from-file\n---\n")
        .expect("write the real personal config");
    set_mode(&target, 0o600);
    std::os::unix::fs::symlink(&target, project.personal_config())
        .expect("symlink the personal config");

    let error = project
        .resolve_composed(&[])
        .expect_err("a symlinked personal config is never read");

    assert!(matches!(error, CredentialError::LocalPermsInsecure { .. }));
}

#[test]
fn the_helpers_trailing_newline_is_trimmed() {
    let project = Project::new();

    let printed = BashTokenCommandRunner
        .run("printf 'token\\n'", &policy(&project))
        .expect("the helper runs");

    assert_eq!(printed, "token");
}

#[test]
fn a_failing_helper_leaks_nothing_it_printed() {
    let project = Project::new();
    let command = format!("printf '{SENTINEL}'; exit 3");

    let error = project
        .resolve(
            &[("ACCELERATOR_JIRA_TOKEN_CMD", &command)],
            &[],
            &[],
            Duration::from_secs(5),
        )
        .expect_err("a non-zero helper is a failure");

    assert!(matches!(error, CredentialError::TokenCmdFailed { .. }));
    assert!(!error.to_string().contains(SENTINEL), "{error}");
    assert!(!format!("{error:?}").contains(SENTINEL));
}

#[test]
fn a_hanging_helper_is_abandoned_at_the_timeout() {
    let project = Project::new();
    let mut policy = policy(&project);
    policy.timeout = Duration::from_millis(300);

    let started = Instant::now();
    let failure = BashTokenCommandRunner
        .run("sleep 120", &policy)
        .expect_err("a hanging helper does not stall the caller");

    assert_eq!(failure, TokenCommandFailure::TimedOut);
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "the call returned in {:?}",
        started.elapsed()
    );
}

#[test]
fn an_unbounded_helper_is_truncated_rather_than_buffered_without_limit() {
    let project = Project::new();
    let mut policy = policy(&project);
    policy.max_output_bytes = 64;

    let outcome = BashTokenCommandRunner
        .run("yes abcdefghijklmnopqrstuvwxyz | head -c 10000000", &policy);

    match outcome {
        Ok(printed) => assert!(
            printed.len() <= 64,
            "the captured output must respect the cap"
        ),
        Err(failure) => assert!(
            matches!(failure, TokenCommandFailure::Failed(_)),
            "{failure:?}"
        ),
    }
}

#[test]
fn the_helper_cannot_read_the_parent_process_environment() {
    let project = Project::new();
    let leaked = project.path("leaked");
    // SAFETY: single-threaded test setup; the assertion is that the child
    // cannot see this, which is the point of the scrub.
    std::env::set_var("ACCELERATOR_TEST_SENTINEL", SENTINEL);

    let printed = BashTokenCommandRunner.run(
        &format!(
            "printf '%s' \"${{ACCELERATOR_TEST_SENTINEL:-absent}}\" > {} \
             && printf 'token'",
            leaked.display()
        ),
        &policy(&project),
    );
    std::env::remove_var("ACCELERATOR_TEST_SENTINEL");

    assert_eq!(printed, Ok("token".to_owned()));
    assert_eq!(
        std::fs::read_to_string(&leaked).expect("the helper wrote its view"),
        "absent",
        "the parent's environment must not reach the helper"
    );
}

#[test]
fn the_helper_runs_in_the_configured_working_directory() {
    let project = Project::new();

    BashTokenCommandRunner
        .run("printf 'token' > seen; printf 'token'", &policy(&project))
        .expect("the helper runs");

    assert!(
        project.path("seen").exists(),
        "the helper ran somewhere other than its defined working directory"
    );
}

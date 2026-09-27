//! The credential ladder's production adapters against real files and a real
//! `bash`, and the project-rooted context they are assembled into.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use config::consent::{
    ConfigFileTracking, Refusal, RepositoryRoots, Runner, Tracking,
};
use config::credentials::{
    resolve_token, CredentialError, Environment, FileFacts, FileState,
    ResolvedToken, TokenKeys,
};
use config::{ConfigError, Key, Level, Resolved, Scalar, Value};
use config_adapters::credentials::{
    project_credential_context, BashCommandRunner, CredentialPorts,
    SystemEnvironment, SystemFileFacts,
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

fn keys() -> TokenKeys {
    TokenKeys::declared("jira.token", "jira.token_cmd")
        .expect("jira's token keys are declared")
}

const fn is_insecure_personal_file(error: &CredentialError) -> bool {
    matches!(
        error,
        CredentialError::Consent(rejection)
            if matches!(rejection.fatal, Refusal::InsecurePersonalFile { .. })
    )
}

/// A scratch project with an `.accelerator/` directory.
struct Project {
    root: TempDir,
}

impl Project {
    fn runner(&self) -> Runner {
        runner_rooted_at(self.root.path())
    }

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
            runner: self.runner(),
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
            runner: self.runner(),
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

fn runner_rooted_at(root: &Path) -> Runner {
    Runner::new(Box::new(BashCommandRunner::new(
        RepositoryRoots::complete(vec![root
            .canonicalize()
            .expect("the project root resolves")]),
        Box::new(SystemEnvironment),
        std::env::temp_dir(),
    )))
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
        Box::new(Untracked),
        runner_rooted_at(project.root.path()),
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

    assert_eq!(
        context.provenance.personal_config,
        project.personal_config()
    );
    assert_eq!(context.insecure_marker, project.marker());
    assert_eq!(context.execution.timeout, Duration::from_secs(12));
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

    assert!(is_insecure_personal_file(&error), "{error:?}");
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

    assert!(is_insecure_personal_file(&error), "{error:?}");
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

    assert!(is_insecure_personal_file(&error), "{error:?}");
}

#[test]
fn a_failing_helper_leaks_nothing_it_printed() {
    let project = Project::new();
    let command = format!("printf '{SENTINEL}'; exit 3");

    let error = project
        .resolve(
            &[("ACCELERATOR_JIRA_TOKEN_CMD", &command)],
            &[],
            Duration::from_secs(5),
        )
        .expect_err("a non-zero helper is a failure");

    assert_eq!(
        error.to_string(),
        "E_TOKEN_CMD_FAILED: jira.token_cmd exited with status 3"
    );
    assert!(!error.to_string().contains(SENTINEL), "{error}");
    assert!(!format!("{error:?}").contains(SENTINEL));
}

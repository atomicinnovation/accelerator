//! Every rung of the credential ladder and every refusal it makes, over
//! in-memory ports.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use config::credentials::{
    refuse_tracked_source, resolve_token, CommandPolicy, CredentialContext,
    CredentialError, Environment, FileFacts, FileState, Provenance,
    ResolvedToken, TokenCommandFailure, TokenCommandRunner, TokenKeys,
    TokenSource, INSECURE_MARKER_RELATIVE,
};
use config::{ConfigError, Key, Level, Resolved, Scalar, Value};

const SENTINEL: &str = "s3cr3t-sentinel-value";
const PERSONAL: &str = "/project/.accelerator/config.local.md";
const MARKER: &str = "/project/.accelerator/allow-insecure-local";

struct FixedConfig {
    personal: BTreeMap<String, String>,
    team: BTreeMap<String, String>,
}

impl config::ConfigAccess for FixedConfig {
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError> {
        let map = match level {
            Some(Level::Personal) => &self.personal,
            Some(Level::Team) => &self.team,
            None => unreachable!("the ladder always names a level"),
        };
        Ok(map.get(&key.to_string()).map_or(Resolved::Absent, |value| {
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

struct FixedProvenance(Vec<PathBuf>);

impl Provenance for FixedProvenance {
    fn is_tracked(&self, path: &Path) -> bool {
        self.0.iter().any(|tracked| tracked == path)
    }
}

struct RecordingFiles {
    states: BTreeMap<PathBuf, Result<FileState, String>>,
    inspected: RefCell<Vec<PathBuf>>,
}

impl FileFacts for RecordingFiles {
    fn inspect(&self, path: &Path) -> Result<FileState, String> {
        self.inspected.borrow_mut().push(path.to_path_buf());
        self.states
            .get(path)
            .cloned()
            .unwrap_or(Ok(FileState::Absent))
    }
}

struct ScriptedRunner {
    outcome: Result<String, TokenCommandFailure>,
    runs: RefCell<Vec<(String, CommandPolicy)>>,
}

impl TokenCommandRunner for ScriptedRunner {
    fn run(
        &self,
        command: &str,
        policy: &CommandPolicy,
    ) -> Result<String, TokenCommandFailure> {
        self.runs
            .borrow_mut()
            .push((command.to_owned(), policy.clone()));
        self.outcome.clone()
    }
}

/// One project's worth of ports, with no personal config and nothing tracked
/// until a test says otherwise.
struct Ladder {
    config: FixedConfig,
    environment: FixedEnvironment,
    provenance: FixedProvenance,
    files: RecordingFiles,
    runner: ScriptedRunner,
    command: CommandPolicy,
}

impl Ladder {
    fn new() -> Self {
        Self {
            config: FixedConfig {
                personal: BTreeMap::new(),
                team: BTreeMap::new(),
            },
            environment: FixedEnvironment(BTreeMap::new()),
            provenance: FixedProvenance(Vec::new()),
            files: RecordingFiles {
                states: BTreeMap::new(),
                inspected: RefCell::new(Vec::new()),
            },
            runner: ScriptedRunner {
                outcome: Ok("from-helper".to_owned()),
                runs: RefCell::new(Vec::new()),
            },
            command: CommandPolicy::rooted_at(PathBuf::from("/project")),
        }
    }

    fn env(mut self, name: &str, value: &str) -> Self {
        self.environment.0.insert(name.to_owned(), value.to_owned());
        self
    }

    fn personal(mut self, key: &str, value: &str) -> Self {
        self.config
            .personal
            .insert(key.to_owned(), value.to_owned());
        self
    }

    fn team(mut self, key: &str, value: &str) -> Self {
        self.config.team.insert(key.to_owned(), value.to_owned());
        self
    }

    fn file(mut self, path: &str, state: Result<FileState, String>) -> Self {
        self.files.states.insert(PathBuf::from(path), state);
        self
    }

    fn personal_file(self, mode: u32) -> Self {
        self.file(PERSONAL, Ok(FileState::File { mode }))
    }

    fn tracked(mut self, path: &str) -> Self {
        self.provenance.0.push(PathBuf::from(path));
        self
    }

    fn helper(mut self, outcome: Result<String, TokenCommandFailure>) -> Self {
        self.runner.outcome = outcome;
        self
    }

    fn resolve(&self) -> Result<ResolvedToken, CredentialError> {
        resolve_token(
            &CredentialContext {
                environment: &self.environment,
                config: &self.config,
                provenance: &self.provenance,
                files: &self.files,
                commands: &self.runner,
                personal_config: PathBuf::from(PERSONAL),
                insecure_marker: PathBuf::from(MARKER),
                command: self.command.clone(),
            },
            &keys(),
        )
    }

    fn helper_runs(&self) -> Vec<String> {
        self.runner
            .runs
            .borrow()
            .iter()
            .map(|(command, _)| command.clone())
            .collect()
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

#[test]
fn the_environment_token_wins_over_every_configured_source() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN", "from-env")
        .personal_file(0o600)
        .personal("jira.token", "from-file");

    let resolved = ladder.resolve().expect("the environment resolves");

    assert_eq!(resolved.value.expose(), "from-env");
    assert_eq!(resolved.source, TokenSource::Env);
}

#[test]
fn the_environment_command_is_a_second_environment_source() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .team("jira.token", "from-shared")
        .helper(Ok("from-env-cmd".to_owned()));

    let resolved = ladder.resolve().expect("the environment command resolves");

    assert_eq!(resolved.value.expose(), "from-env-cmd");
    assert_eq!(resolved.source, TokenSource::EnvCommand);
    assert_eq!(ladder.helper_runs(), ["print-env-token"]);
}

#[test]
fn the_helper_runs_under_the_contexts_command_policy() {
    let mut ladder =
        Ladder::new().env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token");
    ladder.command.timeout = Duration::from_secs(7);

    ladder.resolve().expect("the environment command resolves");

    let runs = ladder.runner.runs.borrow();
    assert_eq!(runs[0].1.timeout, Duration::from_secs(7));
    assert_eq!(runs[0].1.working_directory, PathBuf::from("/project"));
}

#[test]
fn the_personal_value_outranks_the_personal_command() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token", "from-personal")
        .personal("jira.token_cmd", "print-personal-token");

    let resolved = ladder.resolve().expect("the personal value resolves");

    assert_eq!(resolved.value.expose(), "from-personal");
    assert_eq!(resolved.source, TokenSource::Personal);
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn the_personal_command_outranks_the_shared_value() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token_cmd", "print-personal-token")
        .team("jira.token", "from-shared")
        .helper(Ok("from-personal-cmd".to_owned()));

    let resolved = ladder.resolve().expect("the personal command resolves");

    assert_eq!(resolved.value.expose(), "from-personal-cmd");
    assert_eq!(resolved.source, TokenSource::PersonalCommand);
}

#[test]
fn the_shared_value_resolves_only_when_the_personal_file_is_absent() {
    let ladder = Ladder::new().team("jira.token", "from-shared");

    let resolved = ladder.resolve().expect("the shared value resolves");

    assert_eq!(resolved.value.expose(), "from-shared");
    assert_eq!(resolved.source, TokenSource::Shared);
}

#[test]
fn a_present_personal_file_with_no_token_does_not_fall_through_to_shared() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .team("jira.token", "from-shared");

    let error = ladder.resolve().expect_err(
        "the shared file is consulted only when the personal is absent",
    );

    assert!(matches!(error, CredentialError::NoToken { .. }));
}

#[test]
fn a_shared_token_command_is_refused_rather_than_ignored() {
    let ladder = Ladder::new()
        .team("jira.token_cmd", "print-shared-token")
        .team("jira.token", "from-shared");

    let error = ladder.resolve().expect_err("a shared token_cmd is refused");

    assert!(matches!(
        error,
        CredentialError::TokenCmdFromSharedConfig { .. }
    ));
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn nothing_configured_is_a_refusal_naming_both_keys() {
    let error = Ladder::new().resolve().expect_err("nothing resolves");

    assert!(matches!(error, CredentialError::NoToken { .. }));
    assert!(
        error.to_string().starts_with(
            "E_NO_TOKEN: no token found; configure jira.token or \
             jira.token_cmd in .accelerator/config.local.md"
        ),
        "{error}"
    );
}

#[test]
fn a_personal_command_from_a_tracked_file_is_refused_before_it_runs() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token_cmd", "print-personal-token")
        .tracked(PERSONAL);

    let error = ladder.resolve().expect_err("a tracked file is refused");

    assert!(matches!(
        error,
        CredentialError::TokenCmdFromTrackedFile { ref key, .. }
            if key == "jira.token_cmd"
    ));
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn a_personal_value_from_a_tracked_file_is_refused() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token", "from-personal")
        .personal("jira.token_cmd", "print-personal-token")
        .tracked(PERSONAL);

    let error = ladder.resolve().expect_err("a tracked value is refused");

    assert!(matches!(
        error,
        CredentialError::TokenFromTrackedFile { ref key, .. }
            if key == "jira.token"
    ));
    assert!(
        error.to_string().starts_with(&format!(
            "E_TOKEN_FROM_TRACKED_FILE: jira.token in {PERSONAL} refused"
        )),
        "{error}"
    );
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn a_tracked_personal_file_supplying_neither_key_is_no_token() {
    let ladder = Ladder::new().personal_file(0o600).tracked(PERSONAL);

    let error = ladder.resolve().expect_err("nothing resolves");

    assert!(matches!(error, CredentialError::NoToken { .. }));
}

#[test]
fn an_environment_token_never_consults_a_tracked_personal_file() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN", "from-env")
        .personal_file(0o600)
        .personal("jira.token", "from-personal")
        .tracked(PERSONAL);

    let resolved = ladder.resolve().expect("the environment resolves");

    assert_eq!(resolved.source, TokenSource::Env);
    assert!(ladder.files.inspected.borrow().is_empty());
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn an_untracked_owner_only_personal_file_resolves() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token", "from-personal");

    let resolved = ladder.resolve().expect("the personal value resolves");

    assert_eq!(resolved.source, TokenSource::Personal);
}

#[test]
fn an_allowlist_value_from_a_tracked_file_is_held_to_the_same_rule() {
    let personal = PathBuf::from(PERSONAL);
    let tracked = FixedProvenance(vec![personal.clone()]);

    let error =
        refuse_tracked_source(&tracked, &personal, "jira.allowed_sites")
            .expect_err("an allowlist entry from a tracked file is refused");

    assert!(
        error.to_string().starts_with(&format!(
            "E_TOKEN_CMD_FROM_TRACKED_FILE: jira.allowed_sites comes from \
             {PERSONAL}"
        )),
        "{error}"
    );
    assert!(
        refuse_tracked_source(
            &FixedProvenance(Vec::new()),
            &personal,
            "jira.allowed_sites"
        )
        .is_ok(),
        "an untracked provenance file is accepted"
    );
}

#[test]
fn a_personal_config_looser_than_0600_is_refused() {
    let ladder = Ladder::new()
        .personal_file(0o644)
        .personal("jira.token", "from-file");

    let error = ladder.resolve().expect_err("a readable file is refused");

    assert!(matches!(error, CredentialError::LocalPermsInsecure { .. }));
    assert!(error.to_string().contains("chmod 600"), "{error}");
}

#[test]
fn the_insecure_override_needs_both_the_variable_and_a_tracked_marker() {
    let untracked = Ladder::new()
        .env("ACCELERATOR_ALLOW_INSECURE_LOCAL", "1")
        .personal_file(0o644)
        .file(MARKER, Ok(FileState::File { mode: 0o644 }))
        .personal("jira.token", "from-file");
    assert!(
        matches!(
            untracked.resolve(),
            Err(CredentialError::LocalPermsInsecure { .. })
        ),
        "an untracked marker does not unlock the override"
    );

    let tracked = untracked.tracked(MARKER);
    let resolved = tracked
        .resolve()
        .expect("a tracked marker plus the variable honours the override");
    assert_eq!(resolved.value.expose(), "from-file");
}

#[test]
fn a_symlinked_personal_config_is_refused_even_under_the_override() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_ALLOW_INSECURE_LOCAL", "1")
        .file(PERSONAL, Ok(FileState::Symlink))
        .file(MARKER, Ok(FileState::File { mode: 0o644 }))
        .tracked(MARKER)
        .personal("jira.token", "from-file");

    let error = ladder.resolve().expect_err("a symlink is refused");

    assert!(matches!(error, CredentialError::LocalPermsInsecure { .. }));
}

#[test]
fn a_personal_config_that_is_not_a_regular_file_is_refused() {
    let ladder = Ladder::new()
        .file(PERSONAL, Ok(FileState::Other))
        .personal("jira.token", "from-file");

    let error = ladder.resolve().expect_err("a directory is refused");

    assert!(matches!(error, CredentialError::LocalPermsInsecure { .. }));
}

#[test]
fn an_uninspectable_personal_config_is_unreadable() {
    let ladder = Ladder::new()
        .file(PERSONAL, Err("permission denied".to_owned()))
        .team("jira.token", "from-shared");

    let error = ladder.resolve().expect_err("an unreadable file is refused");

    assert!(matches!(error, CredentialError::ConfigUnreadable { .. }));
}

#[test]
fn a_symlinked_marker_does_not_unlock_the_override() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_ALLOW_INSECURE_LOCAL", "1")
        .personal_file(0o644)
        .file(MARKER, Ok(FileState::Symlink))
        .tracked(MARKER)
        .personal("jira.token", "from-file");

    let error = ladder.resolve().expect_err("a symlinked marker is refused");

    assert!(matches!(error, CredentialError::LocalPermsInsecure { .. }));
}

#[test]
fn the_marker_path_lives_under_accelerator() {
    assert_eq!(
        INSECURE_MARKER_RELATIVE,
        ".accelerator/allow-insecure-local"
    );
}

#[test]
fn a_token_carrying_a_control_character_is_refused() {
    let ladder = Ladder::new().env("ACCELERATOR_JIRA_TOKEN", "abc\r\ndef");

    let error = ladder.resolve().expect_err("a header-injecting token");

    assert!(matches!(error, CredentialError::MalformedToken { .. }));
}

#[test]
fn a_failing_helper_names_the_command_key_once() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .helper(Err(TokenCommandFailure::Failed(
            "exited with exit status: 3".to_owned(),
        )));

    let error = ladder.resolve().expect_err("a failing helper");

    assert!(
        error.to_string().starts_with(
            "E_TOKEN_CMD_FAILED: jira.token_cmd exited with exit status: 3"
        ),
        "{error}"
    );
}

#[test]
fn a_helper_that_cannot_run_names_the_command_key_once() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .helper(Err(TokenCommandFailure::CouldNotRun(
            "no such file".to_owned(),
        )));

    let error = ladder.resolve().expect_err("an unrunnable helper");

    assert!(
        error.to_string().starts_with(
            "E_TOKEN_CMD_FAILED: jira.token_cmd could not be run: no such file"
        ),
        "{error}"
    );
}

#[test]
fn a_timed_out_helper_names_the_command_key_once_and_the_policy_timeout() {
    let mut ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .helper(Err(TokenCommandFailure::TimedOut));
    ladder.command.timeout = Duration::from_secs(9);

    let error = ladder.resolve().expect_err("a hanging helper");

    assert!(matches!(
        error,
        CredentialError::TokenCmdTimedOut { after, .. }
            if after == Duration::from_secs(9)
    ));
    assert!(
        error.to_string().starts_with(
            "E_TOKEN_CMD_FAILED: jira.token_cmd did not finish within 9s"
        ),
        "{error}"
    );
}

#[test]
fn a_shared_token_command_refusal_names_the_command_key_once() {
    let ladder = Ladder::new().team("jira.token_cmd", "print-shared-token");

    let error = ladder.resolve().expect_err("a shared token_cmd is refused");

    assert!(
        error.to_string().starts_with(
            "E_TOKEN_CMD_FROM_SHARED_CONFIG: jira.token_cmd in config.md \
             refused"
        ),
        "{error}"
    );
}

#[test]
fn a_tracked_personal_command_refusal_names_the_command_key_once() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token_cmd", "print-personal-token")
        .tracked(PERSONAL);

    let error = ladder.resolve().expect_err("a tracked file is refused");

    assert!(
        error.to_string().starts_with(&format!(
            "E_TOKEN_CMD_FROM_TRACKED_FILE: jira.token_cmd comes from \
             {PERSONAL}"
        )),
        "{error}"
    );
}

#[test]
fn a_resolved_secret_never_renders_itself_under_debug() {
    let ladder = Ladder::new().env("ACCELERATOR_JIRA_TOKEN", SENTINEL);

    let resolved = ladder.resolve().expect("the environment resolves");

    assert!(!format!("{resolved:?}").contains(SENTINEL));
}

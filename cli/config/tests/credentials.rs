//! Every rung of the credential ladder and every refusal it makes, over
//! in-memory ports.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use config::catalogue::BASE_COMMAND_ENVIRONMENT;
use config::consent::{
    CommandExecution, CommandFailure, CommandPolicy, CommandRunner,
    ConfigFileTracking, FailureCause, ProvenanceContext, Refusal, Rejection,
    Runner, StartFailure, Tracking,
};
use config::credentials::{
    resolve_token, CredentialContext, CredentialError, Environment,
    ResolvedToken, TokenKeys, TokenSource,
};
use config::{ConfigError, Key, Level, PersonalFile, Resolved, Scalar, Value};

const SENTINEL: &str = "s3cr3t-sentinel-value";
const PERSONAL: &str = "/project/.accelerator/config.local.md";

struct FixedConfig {
    personal: BTreeMap<String, String>,
    team: BTreeMap<String, String>,
    personal_file: PersonalFile,
    personal_fails: bool,
}

impl config::ConfigAccess for FixedConfig {
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError> {
        let map = match level {
            Some(Level::Personal) if self.personal_fails => {
                return Err(ConfigError::Io {
                    path: PERSONAL.to_owned(),
                    detail: "boom".to_owned(),
                })
            }
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

    fn personal_file(&self) -> &PersonalFile {
        &self.personal_file
    }
}

struct FixedTracking(RefCell<BTreeMap<PathBuf, Tracking>>);

impl ConfigFileTracking for FixedTracking {
    fn tracking(&self, path: &Path) -> Tracking {
        self.0
            .borrow()
            .get(path)
            .copied()
            .unwrap_or(Tracking::Untracked)
    }
}

struct FixedEnvironment(BTreeMap<String, String>);

impl Environment for FixedEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

#[derive(Clone)]
struct ScriptedRunner {
    outcome: Rc<RefCell<Result<String, CommandFailure>>>,
    runs: Rc<RefCell<Vec<(String, CommandPolicy)>>>,
}

impl CommandRunner for ScriptedRunner {
    fn run(
        &self,
        command: &str,
        policy: &CommandPolicy,
    ) -> Result<String, CommandFailure> {
        self.runs
            .borrow_mut()
            .push((command.to_owned(), policy.clone()));
        self.outcome.borrow().clone()
    }
}

/// One project's worth of ports, with no personal config and nothing tracked
/// until a test says otherwise.
struct Ladder {
    config: FixedConfig,
    environment: FixedEnvironment,
    tracking: FixedTracking,
    script: ScriptedRunner,
    runner: Runner,
    timeout: Duration,
}

impl Ladder {
    fn new() -> Self {
        let script = ScriptedRunner {
            outcome: Rc::new(RefCell::new(Ok("from-helper".to_owned()))),
            runs: Rc::new(RefCell::new(Vec::new())),
        };
        Self {
            config: FixedConfig {
                personal: BTreeMap::new(),
                team: BTreeMap::new(),
                personal_file: PersonalFile::Absent,
                personal_fails: false,
            },
            environment: FixedEnvironment(BTreeMap::new()),
            tracking: FixedTracking(RefCell::new(BTreeMap::new())),
            script: script.clone(),
            runner: Runner::new(Box::new(script)),
            timeout: CommandPolicy::DEFAULT_TIMEOUT,
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

    fn personal_file(mut self, mode: u32) -> Self {
        self.config.personal_file = if mode.trailing_zeros() >= 6 {
            PersonalFile::Readable
        } else {
            PersonalFile::Ignored {
                path: PathBuf::from(PERSONAL),
                mode,
            }
        };
        self
    }

    fn tracked(self, path: &str) -> Self {
        self.tracking_of(path, Tracking::Tracked)
    }

    fn tracking_of(self, path: &str, answer: Tracking) -> Self {
        self.tracking
            .0
            .borrow_mut()
            .insert(PathBuf::from(path), answer);
        self
    }

    const fn failing_personal_read(mut self) -> Self {
        self.config.personal_fails = true;
        self
    }

    fn helper(self, outcome: Result<String, CommandFailure>) -> Self {
        *self.script.outcome.borrow_mut() = outcome;
        self
    }

    fn resolve(&self) -> Result<ResolvedToken, CredentialError> {
        resolve_token(
            &CredentialContext {
                provenance: ProvenanceContext {
                    config: &self.config,
                    tracking: &self.tracking,
                    environment: &self.environment,
                    personal_config: PathBuf::from(PERSONAL),
                },
                execution: CommandExecution {
                    runner: &self.runner,
                    timeout: self.timeout,
                },
            },
            &keys(),
        )
    }

    fn helper_runs(&self) -> Vec<String> {
        self.script
            .runs
            .borrow()
            .iter()
            .map(|(command, _)| command.clone())
            .collect()
    }
}

fn keys() -> TokenKeys {
    TokenKeys::declared("jira.token", "jira.token_cmd")
        .expect("jira's token keys are declared")
}

fn codes(refusals: &[Refusal]) -> Vec<String> {
    refusals
        .iter()
        .map(|refusal| {
            refusal
                .to_string()
                .split(':')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

fn rejection(error: CredentialError) -> Rejection {
    let CredentialError::Consent(rejection) = error else {
        panic!("expected a consent rejection, got {error:?}");
    };
    rejection
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
    assert!(resolved.notice.is_none());
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
    assert_eq!(
        resolved
            .notice
            .expect("an environment command notices")
            .to_string(),
        "notice: jira.token_cmd taken from ACCELERATOR_JIRA_TOKEN_CMD"
    );
}

#[test]
fn the_helper_runs_under_the_contexts_command_policy() {
    let mut ladder =
        Ladder::new().env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token");
    ladder.timeout = Duration::from_secs(7);

    ladder.resolve().expect("the environment command resolves");

    let runs = ladder.script.runs.borrow();
    assert_eq!(runs[0].1.timeout(), Duration::from_secs(7));
    assert_eq!(runs[0].1.admitted_environment(), BASE_COMMAND_ENVIRONMENT);
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
    assert!(resolved.refusals.is_empty());
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
fn a_team_command_beside_a_present_personal_file_and_nothing_else_is_fatal() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .team("jira.token_cmd", "print-shared-token");

    let rejection = rejection(ladder.resolve().expect_err("nothing usable"));

    assert!(
        rejection
            .fatal
            .to_string()
            .starts_with("E_CONSENT_KEY_TEAM_LEVEL: jira.token_cmd"),
        "{}",
        rejection.fatal
    );
    assert!(rejection.warnings.is_empty());
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn a_team_command_beside_a_personal_token_warns_and_resolves_the_token() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token", "from-personal")
        .team("jira.token_cmd", "print-shared-token");

    let resolved = ladder.resolve().expect("the personal token resolves");

    assert_eq!(resolved.value.expose(), "from-personal");
    assert_eq!(codes(&resolved.refusals), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn a_team_command_beside_a_team_token_and_no_personal_file_resolves_it() {
    let ladder = Ladder::new()
        .team("jira.token_cmd", "print-shared-token")
        .team("jira.token", "from-shared");

    let resolved = ladder.resolve().expect("the team token resolves");

    assert_eq!(resolved.value.expose(), "from-shared");
    assert_eq!(codes(&resolved.refusals), ["E_CONSENT_KEY_TEAM_LEVEL"]);
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

    let rejection = rejection(ladder.resolve().expect_err("tracked"));

    assert!(
        rejection.fatal.to_string().starts_with(&format!(
            "E_CONSENT_KEY_TRACKED: jira.token_cmd in {PERSONAL} is refused"
        )),
        "{}",
        rejection.fatal
    );
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn a_personal_command_whose_tracking_is_unknown_is_refused() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token_cmd", "print-personal-token")
        .tracking_of(PERSONAL, Tracking::Unknown);

    let rejection = rejection(ladder.resolve().expect_err("unknown"));

    assert_eq!(
        codes(&[rejection.fatal]),
        ["E_CONSENT_KEY_TRACKING_UNKNOWN"]
    );
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn a_personal_value_from_a_tracked_file_is_refused() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token", "from-personal")
        .tracked(PERSONAL);

    let rejection = rejection(ladder.resolve().expect_err("tracked"));

    assert!(
        rejection.fatal.to_string().starts_with(&format!(
            "E_TOKEN_FROM_TRACKED_FILE: jira.token in {PERSONAL} is refused"
        )),
        "{}",
        rejection.fatal
    );
    assert!(matches!(
        rejection.fatal,
        Refusal::PlaintextFromUntrustedFile { .. }
    ));
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn a_personal_value_whose_tracking_is_unknown_is_refused_with_the_hint() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token", "from-personal")
        .tracking_of(PERSONAL, Tracking::Unknown);

    let rejection = rejection(ladder.resolve().expect_err("unknown"));

    assert_eq!(
        codes(std::slice::from_ref(&rejection.fatal)),
        ["E_TOKEN_FROM_TRACKED_FILE"]
    );
    assert!(
        rejection
            .fatal
            .to_string()
            .ends_with("set ACCELERATOR_JIRA_TOKEN in the environment"),
        "{}",
        rejection.fatal
    );
}

#[test]
fn a_tracked_personal_token_beside_a_team_command_is_fatal_and_warns() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token", "from-personal")
        .team("jira.token_cmd", "print-shared-token")
        .tracked(PERSONAL);

    let rejection = rejection(ladder.resolve().expect_err("tracked"));

    assert_eq!(codes(&[rejection.fatal]), ["E_TOKEN_FROM_TRACKED_FILE"]);
    assert_eq!(codes(&rejection.warnings), ["E_CONSENT_KEY_TEAM_LEVEL"]);
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
        .failing_personal_read()
        .tracked(PERSONAL);

    let resolved = ladder.resolve().expect("the environment resolves");

    assert_eq!(resolved.source, TokenSource::Env);
    assert!(resolved.refusals.is_empty());
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn an_environment_command_skips_provenance_but_runs_under_the_policy() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .personal_file(0o600)
        .failing_personal_read()
        .tracked(PERSONAL)
        .helper(Ok("from-env-cmd".to_owned()));

    let resolved = ladder.resolve().expect("the environment command resolves");

    assert_eq!(resolved.value.expose(), "from-env-cmd");
    assert_eq!(
        ladder.script.runs.borrow()[0].1.admitted_environment(),
        BASE_COMMAND_ENVIRONMENT
    );
}

#[test]
fn a_failing_environment_command_falls_through_to_a_personal_token() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .personal_file(0o600)
        .personal("jira.token", "from-personal")
        .helper(Err(CommandFailure::Failed(FailureCause::Exited(1))));

    let resolved = ladder.resolve().expect("the personal token resolves");

    assert_eq!(resolved.value.expose(), "from-personal");
    assert_eq!(codes(&resolved.refusals), ["E_TOKEN_CMD_FAILED"]);
    assert!(resolved.notice.is_none());
}

#[test]
fn a_timed_out_personal_command_beside_a_team_command_is_fatal_and_warns() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .personal("jira.token_cmd", "print-personal-token")
        .team("jira.token_cmd", "print-shared-token")
        .helper(Err(CommandFailure::TimedOut));

    let rejection = rejection(ladder.resolve().expect_err("timed out"));

    assert_eq!(codes(&[rejection.fatal]), ["E_COMMAND_TIMED_OUT"]);
    assert_eq!(codes(&rejection.warnings), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    assert_eq!(ladder.helper_runs(), ["print-personal-token"]);
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
fn an_ignored_personal_config_with_no_environment_token_is_refused() {
    let ladder = Ladder::new()
        .personal_file(0o644)
        .personal("jira.token", "from-file");

    let rejection = rejection(ladder.resolve().expect_err("ignored"));

    assert!(matches!(
        rejection.fatal,
        Refusal::InsecurePersonalFile { .. }
    ));
    assert!(rejection.fatal.to_string().contains("chmod 600"));
}

#[test]
fn an_ignored_personal_config_never_lets_the_team_token_through() {
    let ladder = Ladder::new()
        .personal_file(0o644)
        .team("jira.token", "from-shared");

    let rejection = rejection(
        ladder
            .resolve()
            .expect_err("a team token is not used beside an ignored file"),
    );
    assert_eq!(codes(&[rejection.fatal]), ["E_LOCAL_PERMS_INSECURE"]);

    let resolved = ladder
        .env("ACCELERATOR_JIRA_TOKEN", "from-env")
        .resolve()
        .expect("the environment still resolves beside an ignored file");
    assert_eq!(resolved.value.expose(), "from-env");
    assert_eq!(resolved.source, TokenSource::Env);
}

#[test]
fn an_ignored_personal_config_is_recorded_once_beside_a_team_command() {
    let ladder = Ladder::new()
        .personal_file(0o644)
        .personal("jira.token_cmd", "print-personal-token")
        .team("jira.token_cmd", "print-shared-token");

    let rejection = rejection(ladder.resolve().expect_err("ignored"));

    assert_eq!(codes(&[rejection.fatal]), ["E_LOCAL_PERMS_INSECURE"]);
    assert_eq!(codes(&rejection.warnings), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    assert!(ladder.helper_runs().is_empty());
}

#[test]
fn a_token_carrying_a_control_character_is_refused() {
    let ladder = Ladder::new().env("ACCELERATOR_JIRA_TOKEN", "abc\r\ndef");

    let rejection = rejection(ladder.resolve().expect_err("malformed"));

    assert_eq!(
        rejection.fatal.to_string(),
        "E_TOKEN_MALFORMED: jira.token yielded a value carrying a control \
         character"
    );
}

#[test]
fn a_malformed_environment_token_falls_through_to_a_valid_personal_one() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN", "abc\u{1}def")
        .personal_file(0o600)
        .personal("jira.token", "from-personal");

    let resolved = ladder.resolve().expect("the personal token resolves");

    assert_eq!(resolved.value.expose(), "from-personal");
    assert_eq!(codes(&resolved.refusals), ["E_TOKEN_MALFORMED"]);
}

#[test]
fn a_helper_printing_a_control_character_is_malformed() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .helper(Ok("tok\u{1}".to_owned()));

    let rejection = rejection(ladder.resolve().expect_err("malformed"));

    assert_eq!(codes(&[rejection.fatal]), ["E_TOKEN_MALFORMED"]);
}

#[test]
fn a_failing_helper_is_refused_with_its_cause() {
    let key = config::catalogue::declared("jira.token_cmd").unwrap();
    for (failure, cause) in [
        (
            CommandFailure::Failed(FailureCause::Exited(3)),
            FailureCause::Exited(3),
        ),
        (
            CommandFailure::Failed(FailureCause::CouldNotStart(
                StartFailure::NoBashOnPath,
            )),
            FailureCause::CouldNotStart(StartFailure::NoBashOnPath),
        ),
    ] {
        let ladder = Ladder::new()
            .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
            .helper(Err(failure));

        let error = ladder.resolve().expect_err("a failing helper");

        assert_eq!(
            error,
            CredentialError::Consent(Rejection::alone(
                Refusal::CommandFailed { key, cause }
            ))
        );
    }
}

#[test]
fn a_timed_out_helper_names_the_command_key_and_the_timeout() {
    let mut ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .helper(Err(CommandFailure::TimedOut));
    ladder.timeout = Duration::from_secs(9);

    let error = ladder.resolve().expect_err("a hanging helper");

    assert_eq!(
        error.to_string(),
        "E_COMMAND_TIMED_OUT: jira.token_cmd did not finish within 9s"
    );
}

#[test]
fn an_oversized_helper_output_is_refused() {
    let ladder = Ladder::new()
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "print-env-token")
        .helper(Err(CommandFailure::OutputExceeded));

    let error = ladder.resolve().expect_err("an oversized helper");

    assert_eq!(
        error.to_string(),
        "E_COMMAND_OUTPUT_EXCEEDED: jira.token_cmd printed more than 65536 \
         bytes"
    );
}

#[test]
fn a_failed_personal_read_aborts_carrying_the_team_refusal() {
    let ladder = Ladder::new()
        .personal_file(0o600)
        .failing_personal_read()
        .team("jira.token_cmd", "print-shared-token");

    let error = ladder.resolve().expect_err("an unreadable personal level");

    let CredentialError::ConfigUnreadable(aborted) = &error else {
        panic!("expected an aborted resolution, got {error:?}");
    };
    assert!(matches!(aborted.error, ConfigError::Io { .. }));
    assert_eq!(codes(error.warnings()), ["E_CONSENT_KEY_TEAM_LEVEL"]);
}

fn failing_env_command(ladder: Ladder, failure: CommandFailure) -> Ladder {
    ladder
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "cmd")
        .helper(Err(failure))
}

/// Every refusal code, with and without a usable value left: with one it is
/// a warning on the resolved token, without one it is the fatal refusal. A
/// distrusted personal file refuses every later personal rung, so its codes
/// have no row in which a value remains.
#[test]
fn every_refusal_is_a_warning_beside_a_usable_value_and_fatal_without_one() {
    type Arrange = fn(Ladder) -> Ladder;
    let rows: [(&str, Arrange, Option<Arrange>); 9] = [
        (
            "E_CONSENT_KEY_TEAM_LEVEL",
            |ladder| ladder.team("jira.token_cmd", "shared"),
            Some(|ladder| {
                ladder
                    .team("jira.token_cmd", "shared")
                    .personal("jira.token", "usable")
            }),
        ),
        (
            "E_CONSENT_KEY_TRACKED",
            |ladder| {
                ladder.personal("jira.token_cmd", "mine").tracked(PERSONAL)
            },
            None,
        ),
        (
            "E_CONSENT_KEY_TRACKING_UNKNOWN",
            |ladder| {
                ladder
                    .personal("jira.token_cmd", "mine")
                    .tracking_of(PERSONAL, Tracking::Unknown)
            },
            None,
        ),
        (
            "E_TOKEN_FROM_TRACKED_FILE",
            |ladder| ladder.personal("jira.token", "mine").tracked(PERSONAL),
            None,
        ),
        (
            "E_TOKEN_CMD_FAILED",
            |ladder| {
                failing_env_command(
                    ladder,
                    CommandFailure::Failed(FailureCause::Exited(2)),
                )
            },
            Some(|ladder| {
                failing_env_command(
                    ladder,
                    CommandFailure::Failed(FailureCause::Exited(2)),
                )
                .personal("jira.token", "usable")
            }),
        ),
        (
            "E_COMMAND_TIMED_OUT",
            |ladder| failing_env_command(ladder, CommandFailure::TimedOut),
            Some(|ladder| {
                failing_env_command(ladder, CommandFailure::TimedOut)
                    .personal("jira.token", "usable")
            }),
        ),
        (
            "E_COMMAND_OUTPUT_EXCEEDED",
            |ladder| {
                failing_env_command(ladder, CommandFailure::OutputExceeded)
            },
            Some(|ladder| {
                failing_env_command(ladder, CommandFailure::OutputExceeded)
                    .personal("jira.token", "usable")
            }),
        ),
        (
            "E_TOKEN_MALFORMED",
            |ladder| ladder.env("ACCELERATOR_JIRA_TOKEN", "a\u{1}b"),
            Some(|ladder| {
                ladder
                    .env("ACCELERATOR_JIRA_TOKEN", "a\u{1}b")
                    .personal("jira.token", "usable")
            }),
        ),
        (
            "E_LOCAL_PERMS_INSECURE",
            |ladder| ladder.personal_file(0o644),
            None,
        ),
    ];
    for (code, refused, usable) in rows {
        let error = refused(Ladder::new().personal_file(0o600))
            .resolve()
            .expect_err(code);
        assert_eq!(codes(&[rejection(error).fatal]), [code]);

        if let Some(usable) = usable {
            let resolved = usable(Ladder::new().personal_file(0o600))
                .resolve()
                .expect(code);
            assert_eq!(resolved.value.expose(), "usable", "{code}");
            assert_eq!(codes(&resolved.refusals), [code]);
        }
    }
}

#[test]
fn a_resolved_secret_never_renders_itself_under_debug() {
    let ladder = Ladder::new().env("ACCELERATOR_JIRA_TOKEN", SENTINEL);

    let resolved = ladder.resolve().expect("the environment resolves");

    assert!(!format!("{resolved:?}").contains(SENTINEL));
}

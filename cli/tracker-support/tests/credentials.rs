//! Every rung of the credential ladder, and every hardening obligation the
//! resolver enforces.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use config::{ConfigError, Key, Level, Resolved, Scalar, Value};
use tempfile::TempDir;
use tracker_support::{
    refuse_tracked_source, resolve_token, CommandPolicy, CredentialContext,
    CredentialError, Environment, Provenance, TokenKeys, TokenSource,
};

const SENTINEL: &str = "s3cr3t-sentinel-value";

struct FixedConfig {
    personal: BTreeMap<String, String>,
    team: BTreeMap<String, String>,
}

impl FixedConfig {
    const fn new() -> Self {
        Self {
            personal: BTreeMap::new(),
            team: BTreeMap::new(),
        }
    }

    fn with_personal(mut self, key: &str, value: &str) -> Self {
        self.personal.insert(key.to_owned(), value.to_owned());
        self
    }

    fn with_team(mut self, key: &str, value: &str) -> Self {
        self.team.insert(key.to_owned(), value.to_owned());
        self
    }
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

impl FixedEnvironment {
    const fn empty() -> Self {
        Self(BTreeMap::new())
    }

    fn with(mut self, name: &str, value: &str) -> Self {
        self.0.insert(name.to_owned(), value.to_owned());
        self
    }
}

impl Environment for FixedEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

struct FixedProvenance {
    tracked: Vec<PathBuf>,
}

impl FixedProvenance {
    const fn nothing_tracked() -> Self {
        Self {
            tracked: Vec::new(),
        }
    }

    fn tracking(path: &Path) -> Self {
        Self {
            tracked: vec![path.to_path_buf()],
        }
    }
}

impl Provenance for FixedProvenance {
    fn is_tracked(&self, path: &Path) -> bool {
        self.tracked.iter().any(|tracked| tracked == path)
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

struct Workspace {
    root: TempDir,
}

impl Workspace {
    fn new() -> Self {
        Self {
            root: TempDir::new().expect("a scratch workspace"),
        }
    }

    fn personal_config(&self) -> PathBuf {
        self.root.path().join("config.local.md")
    }

    fn marker(&self) -> PathBuf {
        self.root.path().join("insecure-local-ok")
    }

    fn write_personal_config(&self, mode: u32) -> PathBuf {
        let path = self.personal_config();
        std::fs::write(&path, "---\njira:\n  token: unused\n---\n")
            .expect("write the personal config");
        set_mode(&path, mode);
        path
    }

    fn context<'a>(
        &self,
        environment: &'a dyn Environment,
        config: &'a dyn config::ConfigAccess,
        provenance: &'a dyn Provenance,
    ) -> CredentialContext<'a> {
        CredentialContext {
            environment,
            config,
            provenance,
            personal_config: self.personal_config(),
            insecure_marker: self.marker(),
            command: CommandPolicy {
                timeout: Duration::from_secs(5),
                max_output_bytes: 1024,
                working_directory: self.root.path().to_path_buf(),
            },
        }
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
fn the_environment_token_wins_over_every_configured_source() {
    let workspace = Workspace::new();
    workspace.write_personal_config(0o600);
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "from-env");
    let config = FixedConfig::new().with_personal("jira.token", "from-file");
    let provenance = FixedProvenance::nothing_tracked();

    let resolved = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect("the environment resolves");

    assert_eq!(resolved.value.expose(), "from-env");
    assert_eq!(resolved.source, TokenSource::Env);
}

#[test]
fn the_environment_command_is_a_second_environment_source() {
    let workspace = Workspace::new();
    let environment = FixedEnvironment::empty()
        .with("ACCELERATOR_JIRA_TOKEN_CMD", "printf 'from-env-cmd\\n'");
    let config = FixedConfig::new().with_team("jira.token", "from-shared");
    let provenance = FixedProvenance::nothing_tracked();

    let resolved = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect("the environment command resolves");

    assert_eq!(resolved.value.expose(), "from-env-cmd");
    assert_eq!(resolved.source, TokenSource::EnvCommand);
}

#[test]
fn the_personal_value_outranks_the_personal_command() {
    let workspace = Workspace::new();
    workspace.write_personal_config(0o600);
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new()
        .with_personal("jira.token", "from-personal")
        .with_personal("jira.token_cmd", "printf 'from-personal-cmd'");
    let provenance = FixedProvenance::nothing_tracked();

    let resolved = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect("the personal value resolves");

    assert_eq!(resolved.value.expose(), "from-personal");
    assert_eq!(resolved.source, TokenSource::Personal);
}

#[test]
fn the_personal_command_outranks_the_shared_value() {
    let workspace = Workspace::new();
    workspace.write_personal_config(0o600);
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new()
        .with_personal("jira.token_cmd", "printf 'from-personal-cmd\\n'")
        .with_team("jira.token", "from-shared");
    let provenance = FixedProvenance::nothing_tracked();

    let resolved = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect("the personal command resolves");

    assert_eq!(resolved.value.expose(), "from-personal-cmd");
    assert_eq!(resolved.source, TokenSource::PersonalCommand);
}

#[test]
fn the_shared_value_resolves_only_when_the_personal_file_is_absent() {
    let workspace = Workspace::new();
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new().with_team("jira.token", "from-shared");
    let provenance = FixedProvenance::nothing_tracked();

    let resolved = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect("the shared value resolves");

    assert_eq!(resolved.value.expose(), "from-shared");
    assert_eq!(resolved.source, TokenSource::Shared);
}

#[test]
fn a_present_personal_file_with_no_token_does_not_fall_through_to_shared() {
    let workspace = Workspace::new();
    workspace.write_personal_config(0o600);
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new().with_team("jira.token", "from-shared");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect_err(
        "the shared file is consulted only when the personal is absent",
    );

    assert!(matches!(error, CredentialError::NoToken { .. }));
}

#[test]
fn a_shared_token_command_is_refused_rather_than_ignored() {
    let workspace = Workspace::new();
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new()
        .with_team("jira.token_cmd", "printf 'from-shared-cmd'")
        .with_team("jira.token", "from-shared");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect_err("a shared token_cmd is refused");

    assert!(matches!(
        error,
        CredentialError::TokenCmdFromSharedConfig { .. }
    ));
    assert!(
        error.to_string().contains("move it to config.local.md"),
        "{error}"
    );
}

#[test]
fn nothing_configured_is_a_refusal_naming_the_key() {
    let workspace = Workspace::new();
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect_err("nothing resolves");

    assert!(matches!(error, CredentialError::NoToken { .. }));
    assert!(error.to_string().contains("jira.token"), "{error}");
}

#[test]
fn a_personal_command_from_a_tracked_file_is_refused_before_it_runs() {
    let workspace = Workspace::new();
    let personal = workspace.write_personal_config(0o600);
    let canary = workspace.root.path().join("canary");
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new().with_personal(
        "jira.token_cmd",
        &format!("touch {} && printf 'token'", canary.display()),
    );
    let provenance = FixedProvenance::tracking(&personal);

    let error = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect_err("a tracked provenance file is refused");

    assert!(matches!(
        error,
        CredentialError::TokenCmdFromTrackedFile { .. }
    ));
    assert!(
        !canary.exists(),
        "the refusal must happen before the helper runs"
    );
}

#[test]
fn an_allowlist_value_from_a_tracked_file_is_held_to_the_same_rule() {
    let workspace = Workspace::new();
    let personal = workspace.write_personal_config(0o600);
    let provenance = FixedProvenance::tracking(&personal);

    let error =
        refuse_tracked_source(&provenance, &personal, "jira.allowed_sites")
            .expect_err("an allowlist entry from a tracked file is refused");

    assert!(error.to_string().contains("jira.allowed_sites"), "{error}");
    assert!(
        refuse_tracked_source(
            &FixedProvenance::nothing_tracked(),
            &personal,
            "jira.allowed_sites"
        )
        .is_ok(),
        "an untracked provenance file is accepted"
    );
}

#[test]
fn a_personal_config_looser_than_0600_is_refused() {
    let workspace = Workspace::new();
    workspace.write_personal_config(0o644);
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new().with_personal("jira.token", "from-file");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect_err("a world-readable credential file is refused");

    assert!(matches!(error, CredentialError::LocalPermsInsecure { .. }));
    assert!(error.to_string().contains("chmod 600"), "{error}");
}

#[test]
fn the_insecure_override_needs_both_the_variable_and_a_tracked_marker() {
    let workspace = Workspace::new();
    workspace.write_personal_config(0o644);
    let marker = workspace.marker();
    std::fs::write(&marker, "").expect("write the marker");
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_ALLOW_INSECURE_LOCAL", "1");
    let config = FixedConfig::new().with_personal("jira.token", "from-file");

    let untracked = FixedProvenance::nothing_tracked();
    let refused = resolve_token(
        &workspace.context(&environment, &config, &untracked),
        &keys(),
    );
    assert!(
        matches!(refused, Err(CredentialError::LocalPermsInsecure { .. })),
        "an untracked marker does not unlock the override"
    );

    let tracked = FixedProvenance::tracking(&marker);
    let resolved = resolve_token(
        &workspace.context(&environment, &config, &tracked),
        &keys(),
    )
    .expect("a tracked marker plus the variable honours the override");
    assert_eq!(resolved.value.expose(), "from-file");
}

#[test]
fn a_token_carrying_a_control_character_is_refused() {
    let workspace = Workspace::new();
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "abc\r\ndef");
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect_err("a header-injecting token is refused");

    assert!(matches!(error, CredentialError::MalformedToken { .. }));
}

#[test]
fn a_failing_helper_leaks_nothing_it_printed() {
    let workspace = Workspace::new();
    let environment = FixedEnvironment::empty().with(
        "ACCELERATOR_JIRA_TOKEN_CMD",
        &format!("printf '{SENTINEL}'; exit 3"),
    );
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect_err("a non-zero helper is a failure");

    assert!(matches!(error, CredentialError::TokenCmdFailed { .. }));
    assert!(!error.to_string().contains(SENTINEL), "{error}");
    assert!(!format!("{error:?}").contains(SENTINEL));
}

#[test]
fn a_resolved_secret_never_renders_itself_under_debug() {
    let workspace = Workspace::new();
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", SENTINEL);
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    let resolved = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect("the environment resolves");

    assert!(!format!("{resolved:?}").contains(SENTINEL));
}

#[test]
fn a_hanging_helper_is_abandoned_at_the_timeout() {
    let workspace = Workspace::new();
    let environment = FixedEnvironment::empty()
        .with("ACCELERATOR_JIRA_TOKEN_CMD", "sleep 120");
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();
    let mut context = workspace.context(&environment, &config, &provenance);
    context.command.timeout = Duration::from_millis(300);

    let started = Instant::now();
    let error = resolve_token(&context, &keys())
        .expect_err("a hanging helper does not stall the sync");

    assert!(matches!(error, CredentialError::TokenCmdTimedOut { .. }));
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "the call returned in {:?}",
        started.elapsed()
    );
}

#[test]
fn an_unbounded_helper_is_truncated_rather_than_buffered_without_limit() {
    let workspace = Workspace::new();
    let environment = FixedEnvironment::empty().with(
        "ACCELERATOR_JIRA_TOKEN_CMD",
        "yes abcdefghijklmnopqrstuvwxyz | head -c 10000000",
    );
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();
    let mut context = workspace.context(&environment, &config, &provenance);
    context.command.max_output_bytes = 64;

    let resolved = resolve_token(&context, &keys());

    match resolved {
        Ok(token) => assert!(
            token.value.expose().len() <= 64,
            "the captured output must respect the cap"
        ),
        Err(error) => assert!(
            matches!(error, CredentialError::TokenCmdFailed { .. }),
            "{error}"
        ),
    }
}

#[test]
fn the_helper_cannot_read_the_parent_process_environment() {
    let workspace = Workspace::new();
    let leaked = workspace.root.path().join("leaked");
    // SAFETY: single-threaded test setup; the assertion is that the child
    // cannot see this, which is the point of the scrub.
    std::env::set_var("ACCELERATOR_TEST_SENTINEL", SENTINEL);
    let environment = FixedEnvironment::empty().with(
        "ACCELERATOR_JIRA_TOKEN_CMD",
        &format!(
            "printf '%s' \"${{ACCELERATOR_TEST_SENTINEL:-absent}}\" > {} \
             && printf 'token'",
            leaked.display()
        ),
    );
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    let resolved = resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect("the helper resolves");
    std::env::remove_var("ACCELERATOR_TEST_SENTINEL");

    assert_eq!(resolved.value.expose(), "token");
    assert_eq!(
        std::fs::read_to_string(&leaked).expect("the helper wrote its view"),
        "absent",
        "the parent's environment must not reach the helper"
    );
}

#[test]
fn the_helper_runs_in_the_configured_working_directory() {
    let workspace = Workspace::new();
    let environment = FixedEnvironment::empty().with(
        "ACCELERATOR_JIRA_TOKEN_CMD",
        "printf 'token' > seen; \
               printf 'token'",
    );
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    resolve_token(
        &workspace.context(&environment, &config, &provenance),
        &keys(),
    )
    .expect("the helper resolves");

    assert!(
        workspace.root.path().join("seen").exists(),
        "the helper ran somewhere other than its defined working directory"
    );
}

//! Resolves the `github.token` credential the `collaboration` subcommands
//! authenticate with.
//!
//! Kept CLI-local (following `work-cli/src/config.rs`'s precedent) rather
//! than in the domain/adapters crate: this is authentication plumbing, not
//! `collaboration`'s core PR-helper business logic.

use config::consent;
use config::consent::Refusal;
use config::credentials::resolve_token;
use config::credentials::CredentialContext;
use config::credentials::CredentialError;
use config::credentials::Secret;
use config::credentials::TokenKeys;

/// Resolves `github.token` through the credential ladder every tracker
/// climbs, printing the notice and warnings it met on stderr.
///
/// # Errors
///
/// `kernel::Error::Refusal` when nothing usable resolves, and
/// `kernel::Error::Failed` when a config level cannot be read.
pub fn resolve_github_token(
    context: &CredentialContext<'_>,
) -> Result<Secret, kernel::Error> {
    let keys = TokenKeys::declared("github.token", "github.token_cmd")
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    match resolve_token(context, &keys) {
        Ok(resolved) => {
            if let Some(notice) = &resolved.notice {
                eprintln!("{notice}");
            }
            report_warnings(&resolved.refusals);
            Ok(resolved.value)
        }
        Err(CredentialError::NoToken { .. }) => Err(kernel::Error::Refusal(
            "no github.token configured: set github.token or \
                 github.token_cmd in .accelerator/config.local.md, or export \
                 GH_TOKEN/GITHUB_TOKEN"
                .to_owned(),
        )),
        Err(CredentialError::Consent(rejection)) => {
            report_warnings(&rejection.warnings);
            Err(kernel::Error::Refusal(rejection.fatal.to_string()))
        }
        Err(error @ CredentialError::ConfigUnreadable(_)) => {
            report_warnings(error.warnings());
            Err(kernel::Error::Failed(error.to_string()))
        }
    }
}

fn report_warnings(warnings: &[Refusal]) {
    for warning in consent::reportable(warnings) {
        eprintln!("warning: {warning}");
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::path::PathBuf;
    use std::rc::Rc;

    use config::catalogue::BASE_COMMAND_ENVIRONMENT;
    use config::consent::CommandExecution;
    use config::consent::CommandFailure;
    use config::consent::CommandPolicy;
    use config::consent::CommandRunner;
    use config::consent::ConfigFileTracking;
    use config::consent::ProvenanceContext;
    use config::consent::Runner;
    use config::consent::Tracking;
    use config::credentials::CredentialContext;
    use config::credentials::Environment;
    use config::credentials::Secret;
    use config::{
        ConfigError, Key, Level, PersonalFile, Resolved, Scalar, Value,
    };

    use super::resolve_github_token;

    const PERSONAL: &str = "/project/.accelerator/config.local.md";

    struct FixedConfig {
        personal: BTreeMap<&'static str, &'static str>,
        team: BTreeMap<&'static str, &'static str>,
        personal_file: PersonalFile,
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
                None => {
                    unreachable!("the ladder always names a level")
                }
            };
            Ok(map.get(key.to_string().as_str()).map_or(
                Resolved::Absent,
                |value| {
                    Resolved::Found(Value::Scalar(Scalar::String(
                        (*value).to_owned(),
                    )))
                },
            ))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            unreachable!("resolve_github_token never writes")
        }

        fn personal_file(&self) -> &PersonalFile {
            &self.personal_file
        }
    }

    struct FixedEnvironment(BTreeMap<&'static str, &'static str>);

    impl Environment for FixedEnvironment {
        fn read(&self, name: &str) -> Option<String> {
            self.0.get(name).map(|value| (*value).to_owned())
        }
    }

    struct FixedTracking(Tracking);

    impl ConfigFileTracking for FixedTracking {
        fn tracking(&self, _path: &Path) -> Tracking {
            self.0
        }
    }

    #[derive(Clone)]
    struct RecordingRunner {
        runs: Rc<RefCell<Vec<(String, CommandPolicy)>>>,
    }

    impl CommandRunner for RecordingRunner {
        fn run(
            &self,
            command: &str,
            policy: &CommandPolicy,
        ) -> Result<String, CommandFailure> {
            self.runs
                .borrow_mut()
                .push((command.to_owned(), policy.clone()));
            Ok(format!("from {command}"))
        }
    }

    struct Github {
        config: FixedConfig,
        environment: FixedEnvironment,
        tracking: FixedTracking,
        recording: RecordingRunner,
        runner: Runner,
    }

    impl Github {
        fn new() -> Self {
            let recording = RecordingRunner {
                runs: Rc::new(RefCell::new(Vec::new())),
            };
            Self {
                config: FixedConfig {
                    personal: BTreeMap::new(),
                    team: BTreeMap::new(),
                    personal_file: PersonalFile::Absent,
                },
                environment: FixedEnvironment(BTreeMap::new()),
                tracking: FixedTracking(Tracking::Untracked),
                runner: Runner::new(Box::new(recording.clone())),
                recording,
            }
        }

        fn env(mut self, name: &'static str, value: &'static str) -> Self {
            self.environment.0.insert(name, value);
            self
        }

        fn personal(mut self, key: &'static str, value: &'static str) -> Self {
            self.config.personal.insert(key, value);
            self.config.personal_file = PersonalFile::Readable;
            self
        }

        fn present_personal_file(mut self) -> Self {
            self.config.personal_file = PersonalFile::Readable;
            self
        }

        fn ignored_personal_file(mut self) -> Self {
            self.config.personal_file = PersonalFile::Ignored {
                path: PathBuf::from(PERSONAL),
                mode: 0o644,
            };
            self
        }

        fn team(mut self, key: &'static str, value: &'static str) -> Self {
            self.config.team.insert(key, value);
            self
        }

        const fn tracked(mut self) -> Self {
            self.tracking = FixedTracking(Tracking::Tracked);
            self
        }

        fn resolve(&self) -> Result<Secret, kernel::Error> {
            resolve_github_token(&CredentialContext {
                provenance: ProvenanceContext {
                    config: &self.config,
                    tracking: &self.tracking,
                    environment: &self.environment,
                    personal_config: PathBuf::from(PERSONAL),
                },
                execution: CommandExecution {
                    runner: &self.runner,
                    timeout: CommandPolicy::DEFAULT_TIMEOUT,
                },
            })
        }

        fn commands(&self) -> Vec<String> {
            self.recording
                .runs
                .borrow()
                .iter()
                .map(|(command, _)| command.clone())
                .collect()
        }
    }

    fn token(resolved: Result<Secret, kernel::Error>) -> String {
        match resolved {
            Ok(secret) => secret.expose().to_owned(),
            Err(error) => panic!("expected a token, got {error}"),
        }
    }

    fn refusal(resolved: Result<Secret, kernel::Error>) -> String {
        match resolved {
            Err(kernel::Error::Refusal(message)) => message,
            Err(error) => panic!("expected a refusal, got {error}"),
            Ok(_) => panic!("expected a refusal, got a token"),
        }
    }

    #[test]
    fn gh_token_wins_over_github_token_and_every_configured_source() {
        let github = Github::new()
            .env("GH_TOKEN", "gh-env")
            .env("GITHUB_TOKEN", "github-env")
            .personal("github.token", "personal")
            .personal("github.token_cmd", "gh auth token");

        assert_eq!(token(github.resolve()), "gh-env");
        assert!(github.commands().is_empty());
    }

    #[test]
    fn github_token_resolves_when_gh_token_is_absent() {
        let github = Github::new().env("GITHUB_TOKEN", "github-env");

        assert_eq!(token(github.resolve()), "github-env");
    }

    #[test]
    fn a_personal_token_outranks_a_personal_command() {
        let github = Github::new()
            .personal("github.token", "personal")
            .personal("github.token_cmd", "gh auth token");

        assert_eq!(token(github.resolve()), "personal");
        assert!(github.commands().is_empty());
    }

    #[test]
    fn a_personal_command_runs_under_the_github_policy() {
        let github =
            Github::new().personal("github.token_cmd", "gh auth token");

        assert_eq!(token(github.resolve()), "from gh auth token");
        let runs = github.recording.runs.borrow();
        assert_eq!(runs[0].1.timeout(), CommandPolicy::DEFAULT_TIMEOUT);
        assert_eq!(
            runs[0].1.admitted_environment(),
            [BASE_COMMAND_ENVIRONMENT, &["GH_HOST", "GH_CONFIG_DIR"]].concat()
        );
    }

    #[test]
    fn a_personal_command_wins_over_a_team_token() {
        let github = Github::new()
            .personal("github.token_cmd", "gh auth token")
            .team("github.token", "team");

        assert_eq!(token(github.resolve()), "from gh auth token");
    }

    #[test]
    fn a_team_command_beside_a_team_token_and_no_personal_file_uses_the_token()
    {
        let github = Github::new()
            .team("github.token_cmd", "gh auth token")
            .team("github.token", "team");

        assert_eq!(token(github.resolve()), "team");
        assert!(github.commands().is_empty());
    }

    #[test]
    fn a_team_token_beside_a_present_personal_file_is_not_used() {
        let github = Github::new()
            .present_personal_file()
            .team("github.token", "team");

        let message = refusal(github.resolve());

        assert!(
            message.starts_with("no github.token configured"),
            "{message}"
        );
    }

    #[test]
    fn a_team_command_alone_is_refused_at_team_level() {
        let github = Github::new().team("github.token_cmd", "gh auth token");

        let message = refusal(github.resolve());

        assert!(
            message.starts_with("E_CONSENT_KEY_TEAM_LEVEL: github.token_cmd"),
            "{message}"
        );
        assert!(github.commands().is_empty());
    }

    #[test]
    fn a_tracked_personal_command_is_refused_before_it_runs() {
        let github = Github::new()
            .personal("github.token_cmd", "gh auth token")
            .tracked();

        let message = refusal(github.resolve());

        assert!(
            message.starts_with("E_CONSENT_KEY_TRACKED: github.token_cmd"),
            "{message}"
        );
        assert!(github.commands().is_empty());
    }

    #[test]
    fn a_tracked_personal_token_is_refused() {
        let github =
            Github::new().personal("github.token", "personal").tracked();

        let message = refusal(github.resolve());

        assert!(
            message.starts_with("E_TOKEN_FROM_TRACKED_FILE: github.token"),
            "{message}"
        );
    }

    #[test]
    fn an_ignored_personal_file_never_lets_a_team_token_through() {
        let github = Github::new()
            .team("github.token", "team")
            .ignored_personal_file();

        let message = refusal(github.resolve());

        assert!(message.starts_with("E_LOCAL_PERMS_INSECURE"), "{message}");
    }

    #[test]
    fn an_env_token_resolves_beside_an_ignored_personal_file() {
        let github = Github::new()
            .env("GH_TOKEN", "gh-env")
            .team("github.token", "team")
            .ignored_personal_file();

        assert_eq!(token(github.resolve()), "gh-env");
    }

    #[test]
    fn a_token_carrying_a_control_character_is_malformed() {
        let github = Github::new().env("GH_TOKEN", "a\u{1}b");

        let message = refusal(github.resolve());

        assert!(message.starts_with("E_TOKEN_MALFORMED"), "{message}");
    }

    #[test]
    fn nothing_configured_is_a_refusal_naming_the_routes() {
        let message = refusal(Github::new().resolve());

        assert!(message.contains("GH_TOKEN/GITHUB_TOKEN"), "{message}");
    }
}

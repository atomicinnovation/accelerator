//! Resolves the `github.token` credential the `collaboration` subcommands
//! authenticate with.
//!
//! Kept CLI-local (following `work-cli/src/config.rs`'s precedent) rather
//! than in the domain/adapters crate: this is authentication plumbing, not
//! `collaboration`'s core PR-helper business logic.

use std::process::Command;

use config::{ConfigAccess, Key, Level, Resolved};

pub enum TokenSource {
    GhTokenEnv,
    GithubTokenEnv,
    Config,
    ConfigCmd,
}

pub struct ResolvedToken {
    pub value: String,
    /// Which precedence step resolved the token — asserted per-branch by
    /// this module's own tests; not yet read by any caller.
    #[allow(dead_code)]
    pub source: TokenSource,
}

/// Resolves the `github.token` credential.
///
/// Precedence: `GH_TOKEN`, then `GITHUB_TOKEN`, then the `github.token`
/// config value, then `github.token_cmd` output (executed via `bash -c`) —
/// env-first, matching `jira-auth.sh`/`linear-auth.sh`'s own precedence
/// order, so an ambient env var reliably escapes a stale or over-broad
/// on-filesystem config value rather than being shadowed by it. A
/// `token_cmd` configured in the shared/team config file (rather than
/// local overrides) is rejected with a clear error — mirroring
/// `jira`/`linear`'s shared-config `token_cmd` ban — rather than silently
/// executed.
///
/// # Errors
///
/// `kernel::Error::Refusal` when nothing resolves, or when a shared-config
/// `token_cmd` is present.
pub fn resolve_github_token(
    config: &dyn ConfigAccess,
) -> Result<ResolvedToken, kernel::Error> {
    if let Some(value) = nonempty_env("GH_TOKEN") {
        return Ok(ResolvedToken {
            value,
            source: TokenSource::GhTokenEnv,
        });
    }

    if let Some(value) = nonempty_env("GITHUB_TOKEN") {
        return Ok(ResolvedToken {
            value,
            source: TokenSource::GithubTokenEnv,
        });
    }

    if let Some(value) = nonempty_config_value(config)? {
        return Ok(ResolvedToken {
            value,
            source: TokenSource::Config,
        });
    }

    if nonempty_level_value(config, Level::Team, "github.token_cmd")?.is_some()
    {
        return Err(kernel::Error::Refusal(
            "github.token_cmd must not be set in the shared \
             .accelerator/config.md — move it to .accelerator/config.local.md"
                .to_owned(),
        ));
    }

    if let Some(cmd) =
        nonempty_level_value(config, Level::Personal, "github.token_cmd")?
    {
        return Ok(ResolvedToken {
            value: run_token_cmd(&cmd)?,
            source: TokenSource::ConfigCmd,
        });
    }

    Err(kernel::Error::Refusal(
        "no github.token configured: set github.token or \
         github.token_cmd in .accelerator/config.local.md, or export \
         GH_TOKEN/GITHUB_TOKEN"
            .to_owned(),
    ))
}

fn nonempty_config_value(
    config: &dyn ConfigAccess,
) -> Result<Option<String>, kernel::Error> {
    let key = parse_key("github.token")?;
    let resolution = config
        .effective(&key, None)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(resolution
        .configured_value()
        .filter(|value| !value.is_empty()))
}

fn nonempty_level_value(
    config: &dyn ConfigAccess,
    level: Level,
    key: &str,
) -> Result<Option<String>, kernel::Error> {
    let parsed = parse_key(key)?;
    let resolved = config
        .get(&parsed, Some(level))
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(match resolved {
        Resolved::Found(value) => {
            let rendered = config::render_value(&value);
            (!rendered.is_empty()).then_some(rendered)
        }
        Resolved::Absent => None,
    })
}

fn nonempty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn parse_key(key: &str) -> Result<Key, kernel::Error> {
    Key::parse(key).map_err(|error| kernel::Error::Failed(error.to_string()))
}

/// Runs `cmd` via `bash -c`, returning its trimmed stdout.
///
/// # Errors
///
/// `kernel::Error::Refusal` when the command cannot be run or exits
/// non-zero.
fn run_token_cmd(cmd: &str) -> Result<String, kernel::Error> {
    let output =
        Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|error| {
                kernel::Error::Refusal(format!(
                    "github.token_cmd could not be run: {error}"
                ))
            })?;
    if !output.status.success() {
        return Err(kernel::Error::Refusal(format!(
            "github.token_cmd exited with {}",
            output.status
        )));
    }
    let token = String::from_utf8(output.stdout).map_err(|error| {
        kernel::Error::Refusal(format!(
            "github.token_cmd produced non-UTF-8 output: {error}"
        ))
    })?;
    Ok(token.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use config::{ConfigError, Key, Level, Resolved, Scalar, Value};

    use super::{resolve_github_token, TokenSource};

    struct FixedConfig {
        personal: BTreeMap<&'static str, &'static str>,
        team: BTreeMap<&'static str, &'static str>,
    }

    impl FixedConfig {
        fn new() -> Self {
            Self {
                personal: BTreeMap::new(),
                team: BTreeMap::new(),
            }
        }

        fn with_personal(
            mut self,
            key: &'static str,
            value: &'static str,
        ) -> Self {
            self.personal.insert(key, value);
            self
        }

        fn with_team(mut self, key: &'static str, value: &'static str) -> Self {
            self.team.insert(key, value);
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
                None => {
                    unreachable!("this fake always passes an explicit level")
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
    }

    // A process-wide lock: GH_TOKEN/GITHUB_TOKEN env mutation is process
    // state, so tests touching them must not interleave.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<T>(
        vars: &[(&str, Option<&str>)],
        body: impl FnOnce() -> T,
    ) -> T {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous: Vec<(&str, Option<String>)> = vars
            .iter()
            .map(|(name, _)| (*name, std::env::var(name).ok()))
            .collect();
        for (name, value) in vars {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        let result = body();
        for (name, value) in previous {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        result
    }

    #[test]
    fn gh_token_env_resolves_when_no_config_is_present(
    ) -> Result<(), kernel::Error> {
        with_env(
            &[("GH_TOKEN", Some("gh-env-token")), ("GITHUB_TOKEN", None)],
            || {
                let config = FixedConfig::new();
                let resolved = resolve_github_token(&config)?;
                assert_eq!(resolved.value, "gh-env-token");
                assert!(matches!(resolved.source, TokenSource::GhTokenEnv));
                Ok(())
            },
        )
    }

    #[test]
    fn github_token_env_resolves_when_gh_token_is_absent(
    ) -> Result<(), kernel::Error> {
        with_env(
            &[
                ("GH_TOKEN", None),
                ("GITHUB_TOKEN", Some("github-env-token")),
            ],
            || {
                let config = FixedConfig::new();
                let resolved = resolve_github_token(&config)?;
                assert_eq!(resolved.value, "github-env-token");
                assert!(matches!(resolved.source, TokenSource::GithubTokenEnv));
                Ok(())
            },
        )
    }

    #[test]
    fn an_env_token_takes_precedence_over_a_configured_token(
    ) -> Result<(), kernel::Error> {
        with_env(
            &[("GH_TOKEN", Some("gh-env-token")), ("GITHUB_TOKEN", None)],
            || {
                let config = FixedConfig::new()
                    .with_personal("github.token", "personal-token");
                let resolved = resolve_github_token(&config)?;
                assert_eq!(resolved.value, "gh-env-token");
                assert!(matches!(resolved.source, TokenSource::GhTokenEnv));
                Ok(())
            },
        )
    }

    #[test]
    fn an_env_token_takes_precedence_over_a_personal_token_cmd(
    ) -> Result<(), kernel::Error> {
        with_env(
            &[
                ("GH_TOKEN", Some("gh-env-token")),
                ("GITHUB_TOKEN", Some("github-env-token")),
            ],
            || {
                let config = FixedConfig::new()
                    .with_personal("github.token_cmd", "printf should-not-run");
                let resolved = resolve_github_token(&config)?;
                assert_eq!(resolved.value, "gh-env-token");
                assert!(matches!(resolved.source, TokenSource::GhTokenEnv));
                Ok(())
            },
        )
    }

    #[test]
    fn a_configured_token_resolves_when_no_env_var_is_set(
    ) -> Result<(), kernel::Error> {
        with_env(&[("GH_TOKEN", None), ("GITHUB_TOKEN", None)], || {
            let config = FixedConfig::new()
                .with_personal("github.token", "personal-token");
            let resolved = resolve_github_token(&config)?;
            assert_eq!(resolved.value, "personal-token");
            assert!(matches!(resolved.source, TokenSource::Config));
            Ok(())
        })
    }

    #[test]
    fn a_personal_token_cmd_resolves_when_no_token_is_configured(
    ) -> Result<(), kernel::Error> {
        with_env(&[("GH_TOKEN", None), ("GITHUB_TOKEN", None)], || {
            let config = FixedConfig::new()
                .with_personal("github.token_cmd", "printf cmd-token");
            let resolved = resolve_github_token(&config)?;
            assert_eq!(resolved.value, "cmd-token");
            assert!(matches!(resolved.source, TokenSource::ConfigCmd));
            Ok(())
        })
    }

    #[test]
    fn a_shared_token_cmd_is_banned_even_when_no_other_source_resolves(
    ) -> Result<(), kernel::Error> {
        with_env(&[("GH_TOKEN", None), ("GITHUB_TOKEN", None)], || {
            let config = FixedConfig::new()
                .with_team("github.token_cmd", "printf shared-token");
            let result = resolve_github_token(&config);
            assert!(matches!(result, Err(kernel::Error::Refusal(_))));
            Ok(())
        })
    }

    #[test]
    fn nothing_configured_is_a_refusal() -> Result<(), kernel::Error> {
        with_env(&[("GH_TOKEN", None), ("GITHUB_TOKEN", None)], || {
            let config = FixedConfig::new();
            let result = resolve_github_token(&config);
            assert!(matches!(result, Err(kernel::Error::Refusal(_))));
            Ok(())
        })
    }

    #[test]
    fn a_configured_token_takes_precedence_over_a_personal_token_cmd(
    ) -> Result<(), kernel::Error> {
        with_env(&[("GH_TOKEN", None), ("GITHUB_TOKEN", None)], || {
            let config = FixedConfig::new()
                .with_personal("github.token", "personal-token")
                .with_personal("github.token_cmd", "printf should-not-run");
            let resolved = resolve_github_token(&config)?;
            assert_eq!(resolved.value, "personal-token");
            Ok(())
        })
    }
}

//! The five-rung credential ladder both providers climb.
//!
//! | # | Source | Notes |
//! |---|---|---|
//! | 1 | `ACCELERATOR_<PROVIDER>_TOKEN` | |
//! | 2 | `ACCELERATOR_<PROVIDER>_TOKEN_CMD` | a second environment source |
//! | 3 | `config.local.md` `token` | behind the permissions gate |
//! | 4 | `config.local.md` `token_cmd` | behind the same gate |
//! | 5 | `config.md` `token` | only when `config.local.md` is absent |
//!
//! Two consequences a summary tends to get backwards: the personal
//! `token_cmd` outranks the shared `token` value, and the shared file is
//! consulted only when the personal one does not exist at all — not merely
//! when it carries no token.
//!
//! Four deliberate hardening choices, each made because the safer behaviour
//! is worth it rather than for convenience:
//!
//! - a `token_cmd` in the shared config is **refused** rather than warned
//!   about and skipped — a silently-ignored credential source is worse than
//!   a loud one
//! - a personal `token` or `token_cmd` whose provenance file is VCS-tracked
//!   is refused: a repository-relative `config.local.md` can simply be
//!   committed, and `.gitignore` does not apply to an already-tracked file, so
//!   a hostile repository could otherwise supply a command a fresh clone
//!   executes, and a committed value is a leaked credential
//! - the helper runs under a wall-clock timeout, an output cap, a scrubbed
//!   environment and a defined working directory, so the one deliberately
//!   executed foreign code path is no more privileged than it needs to be
//! - its output is never folded into an error, and [`Secret`] and
//!   [`CredentialError`] both redact under `Debug`
//!
//! The ladder is pure policy: the environment, the filesystem facts, the VCS
//! provenance, and the helper run all arrive through ports, whose production
//! adapters live in `config-adapters`.

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use crate::render::render_value;
use crate::service::ConfigAccess;
use crate::service::Resolved;
use crate::Key;
use crate::Level;

/// A credential value that never renders itself.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(redacted)")
    }
}

/// Which rung of the ladder produced a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    Env,
    EnvCommand,
    Personal,
    PersonalCommand,
    Shared,
}

/// A resolved token and the rung it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToken {
    pub value: Secret,
    pub source: TokenSource,
}

/// The provider-specific names the ladder reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenKeys {
    pub env: &'static str,
    pub env_command: &'static str,
    pub value: Key,
    pub command: Key,
}

/// Environment reads, injected so a test needs no process state.
pub trait Environment {
    fn read(&self, name: &str) -> Option<String>;
}

/// Whether a file is tracked by the repository's VCS — the property that
/// decides whether a command-valued or allowlist-valued key may be honoured.
pub trait Provenance {
    fn is_tracked(&self, path: &Path) -> bool;
}

/// The bounds the credential helper runs under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPolicy {
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub working_directory: PathBuf,
}

impl CommandPolicy {
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

    #[must_use]
    pub const fn rooted_at(working_directory: PathBuf) -> Self {
        Self {
            timeout: Self::DEFAULT_TIMEOUT,
            max_output_bytes: 64 * 1024,
            working_directory,
        }
    }
}

/// What a path is, as the permissions gate needs to know it.
///
/// A dangling symlink is [`FileState::Absent`]: the ladder treats a personal
/// config that cannot be opened as one that does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Absent,
    Symlink,
    File { mode: u32 },
    Other,
}

/// Filesystem facts, injected so the ladder names no filesystem.
pub trait FileFacts {
    /// # Errors
    ///
    /// A rendered reason when the path's facts cannot be read.
    fn inspect(&self, path: &Path) -> Result<FileState, String>;
}

/// Why a credential helper produced no token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenCommandFailure {
    CouldNotRun(String),
    Failed(String),
    TimedOut,
}

/// Runs a credential helper, injected so the ladder spawns no process.
///
/// The helper's stdout never reaches a failure: only the trailing newline is
/// trimmed from what it printed.
pub trait TokenCommandRunner {
    /// # Errors
    ///
    /// [`TokenCommandFailure`] when the helper cannot run, fails, or outlives
    /// the policy's timeout.
    fn run(
        &self,
        command: &str,
        policy: &CommandPolicy,
    ) -> Result<String, TokenCommandFailure>;
}

/// Repo-relative path of the insecure-local override marker.
pub const INSECURE_MARKER_RELATIVE: &str = ".accelerator/allow-insecure-local";

/// Everything the ladder reads beyond the keys themselves.
pub struct CredentialContext<'a> {
    pub environment: &'a dyn Environment,
    pub config: &'a dyn ConfigAccess,
    pub provenance: &'a dyn Provenance,
    pub files: &'a dyn FileFacts,
    pub commands: &'a dyn TokenCommandRunner,
    pub personal_config: PathBuf,
    pub insecure_marker: PathBuf,
    pub command: CommandPolicy,
}

/// Why no token could be resolved.
///
/// `Debug` is hand-written and redacting: a helper that prints a secret and
/// then fails must not leak it into a CI log through a `{:?}` of the error.
#[derive(Clone, PartialEq, Eq)]
pub enum CredentialError {
    NoToken { key: String },
    TokenCmdFailed { key: String, detail: String },
    TokenCmdTimedOut { key: String, after: Duration },
    TokenCmdFromSharedConfig { key: String },
    TokenCmdFromTrackedFile { key: String, path: PathBuf },
    TokenFromTrackedFile { key: String, path: PathBuf },
    LocalPermsInsecure { path: PathBuf, mode: u32 },
    MalformedToken { key: String },
    ConfigUnreadable { key: String, detail: String },
}

impl fmt::Display for CredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoToken { key } => write!(
                formatter,
                "E_NO_TOKEN: no token found; configure {key} or {key}_cmd \
                 in .accelerator/config.local.md"
            ),
            Self::TokenCmdFailed { key, detail } => {
                write!(formatter, "E_TOKEN_CMD_FAILED: {key} {detail}")
            }
            Self::TokenCmdTimedOut { key, after } => write!(
                formatter,
                "E_TOKEN_CMD_FAILED: {key} did not finish within {}s",
                after.as_secs()
            ),
            Self::TokenCmdFromSharedConfig { key } => write!(
                formatter,
                "E_TOKEN_CMD_FROM_SHARED_CONFIG: {key} in config.md \
                 refused — move it to config.local.md"
            ),
            Self::TokenCmdFromTrackedFile { key, path } => write!(
                formatter,
                "E_TOKEN_CMD_FROM_TRACKED_FILE: {key} comes from {}, \
                 which is tracked by version control — a command a clone \
                 would run is refused",
                path.display()
            ),
            Self::TokenFromTrackedFile { key, path } => write!(
                formatter,
                "E_TOKEN_FROM_TRACKED_FILE: {key} in {} refused — the file \
                 is tracked by version control, so every clone carries the \
                 credential; untrack it",
                path.display()
            ),
            Self::LocalPermsInsecure { path, mode } => write!(
                formatter,
                "E_LOCAL_PERMS_INSECURE: {} is mode {mode:04o}; chmod 600 to \
                 allow credential read",
                path.display()
            ),
            Self::MalformedToken { key } => write!(
                formatter,
                "E_TOKEN_MALFORMED: the {key} value carries a control \
                 character"
            ),
            Self::ConfigUnreadable { key, detail } => {
                write!(formatter, "{key} could not be read: {detail}")
            }
        }
    }
}

impl fmt::Debug for CredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (variant, key) = match self {
            Self::NoToken { key } => ("NoToken", key.as_str()),
            Self::TokenCmdFailed { key, .. } => {
                ("TokenCmdFailed", key.as_str())
            }
            Self::TokenCmdTimedOut { key, .. } => {
                ("TokenCmdTimedOut", key.as_str())
            }
            Self::TokenCmdFromSharedConfig { key } => {
                ("TokenCmdFromSharedConfig", key.as_str())
            }
            Self::TokenCmdFromTrackedFile { key, .. } => {
                ("TokenCmdFromTrackedFile", key.as_str())
            }
            Self::TokenFromTrackedFile { key, .. } => {
                ("TokenFromTrackedFile", key.as_str())
            }
            Self::LocalPermsInsecure { .. } => ("LocalPermsInsecure", ""),
            Self::MalformedToken { key } => ("MalformedToken", key.as_str()),
            Self::ConfigUnreadable { key, .. } => {
                ("ConfigUnreadable", key.as_str())
            }
        };
        write!(formatter, "{variant} {{ key: {key:?} }}")
    }
}

impl Error for CredentialError {}

/// Climbs the ladder for one provider's keys.
///
/// # Errors
///
/// [`CredentialError`] naming the rung that refused.
pub fn resolve_token(
    context: &CredentialContext<'_>,
    keys: &TokenKeys,
) -> Result<ResolvedToken, CredentialError> {
    if let Some(value) = nonempty(context.environment.read(keys.env)) {
        return accept(value, TokenSource::Env, &key_name(&keys.value));
    }

    if let Some(command) = nonempty(context.environment.read(keys.env_command))
    {
        let value =
            run_token_command(context, &command, &key_name(&keys.command))?;
        return accept(value, TokenSource::EnvCommand, &key_name(&keys.value));
    }

    if personal_config_exists(context)? {
        if let Some(value) =
            level_value(context.config, &keys.value, Level::Personal)?
        {
            refuse_tracked_value(context, &key_name(&keys.value))?;
            return accept(
                value,
                TokenSource::Personal,
                &key_name(&keys.value),
            );
        }

        if let Some(command) =
            level_value(context.config, &keys.command, Level::Personal)?
        {
            refuse_tracked_source(
                context.provenance,
                &context.personal_config,
                &key_name(&keys.command),
            )?;
            let value =
                run_token_command(context, &command, &key_name(&keys.command))?;
            return accept(
                value,
                TokenSource::PersonalCommand,
                &key_name(&keys.value),
            );
        }
    } else {
        if level_value(context.config, &keys.command, Level::Team)?.is_some() {
            return Err(CredentialError::TokenCmdFromSharedConfig {
                key: key_name(&keys.command),
            });
        }
        if let Some(value) =
            level_value(context.config, &keys.value, Level::Team)?
        {
            return accept(value, TokenSource::Shared, &key_name(&keys.value));
        }
    }

    Err(CredentialError::NoToken {
        key: key_name(&keys.value),
    })
}

/// Refuses a command-valued or allowlist-valued key whose provenance file is
/// tracked by version control.
///
/// `.accelerator/config.local.md` is repository-relative, so a hostile
/// repository can simply commit it — `.gitignore` does not apply to an
/// already-tracked file — and thereby supply a command a fresh clone would
/// run, or an allowlist entry blessing a host of its choosing. Every such key
/// goes through here, not only the token ones.
///
/// # Errors
///
/// [`CredentialError::TokenCmdFromTrackedFile`] when the file is tracked.
pub fn refuse_tracked_source(
    provenance: &dyn Provenance,
    path: &Path,
    key: &str,
) -> Result<(), CredentialError> {
    if provenance.is_tracked(path) {
        return Err(CredentialError::TokenCmdFromTrackedFile {
            key: key.to_owned(),
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn refuse_tracked_value(
    context: &CredentialContext<'_>,
    key: &str,
) -> Result<(), CredentialError> {
    if context.provenance.is_tracked(&context.personal_config) {
        return Err(CredentialError::TokenFromTrackedFile {
            key: key.to_owned(),
            path: context.personal_config.clone(),
        });
    }
    Ok(())
}

fn accept(
    value: String,
    source: TokenSource,
    key: &str,
) -> Result<ResolvedToken, CredentialError> {
    if value.chars().any(char::is_control) {
        return Err(CredentialError::MalformedToken {
            key: key.to_owned(),
        });
    }
    Ok(ResolvedToken {
        value: Secret::new(value),
        source,
    })
}

fn key_name(key: &Key) -> String {
    key.to_string()
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

fn level_value(
    config: &dyn ConfigAccess,
    key: &Key,
    level: Level,
) -> Result<Option<String>, CredentialError> {
    let resolved = config.get(key, Some(level)).map_err(|error| {
        CredentialError::ConfigUnreadable {
            key: key.to_string(),
            detail: error.to_string(),
        }
    })?;
    Ok(match resolved {
        Resolved::Found(value) => nonempty(Some(render_value(&value))),
        Resolved::Absent => None,
    })
}

/// Whether the personal config exists, behind the mode-0600 gate, with an
/// override: `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` counts only when
/// `.accelerator/allow-insecure-local` is a regular, non-symlink, VCS-tracked
/// file.
fn personal_config_exists(
    context: &CredentialContext<'_>,
) -> Result<bool, CredentialError> {
    let path = &context.personal_config;
    let state = context.files.inspect(path).map_err(|detail| {
        CredentialError::ConfigUnreadable {
            key: path.display().to_string(),
            detail,
        }
    })?;
    let mode = match state {
        FileState::Absent => return Ok(false),
        FileState::Symlink | FileState::Other => {
            return Err(CredentialError::LocalPermsInsecure {
                path: path.clone(),
                mode: 0,
            })
        }
        FileState::File { mode } => mode,
    };
    if mode.trailing_zeros() >= 6 || insecure_override_allowed(context) {
        return Ok(true);
    }
    Err(CredentialError::LocalPermsInsecure {
        path: path.clone(),
        mode,
    })
}

fn insecure_override_allowed(context: &CredentialContext<'_>) -> bool {
    if context.environment.read("ACCELERATOR_ALLOW_INSECURE_LOCAL")
        != Some("1".to_owned())
    {
        return false;
    }
    let marker = &context.insecure_marker;
    matches!(context.files.inspect(marker), Ok(FileState::File { .. }))
        && context.provenance.is_tracked(marker)
}

fn run_token_command(
    context: &CredentialContext<'_>,
    command: &str,
    key: &str,
) -> Result<String, CredentialError> {
    context
        .commands
        .run(command, &context.command)
        .map_err(|failure| match failure {
            TokenCommandFailure::CouldNotRun(detail) => {
                CredentialError::TokenCmdFailed {
                    key: key.to_owned(),
                    detail: format!("could not be run: {detail}"),
                }
            }
            TokenCommandFailure::Failed(detail) => {
                CredentialError::TokenCmdFailed {
                    key: key.to_owned(),
                    detail,
                }
            }
            TokenCommandFailure::TimedOut => {
                CredentialError::TokenCmdTimedOut {
                    key: key.to_owned(),
                    after: context.command.timeout,
                }
            }
        })
}

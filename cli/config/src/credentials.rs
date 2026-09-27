//! The five-rung credential ladder both providers climb.
//!
//! | # | Source | Notes |
//! |---|---|---|
//! | 1 | `ACCELERATOR_<PROVIDER>_TOKEN` | |
//! | 2 | `ACCELERATOR_<PROVIDER>_TOKEN_CMD` | a second environment source |
//! | 3 | `config.local.md` `token` | only when the file is readable |
//! | 4 | `config.local.md` `token_cmd` | only when the file is readable |
//! | 5 | `config.md` `token` | only when `config.local.md` is absent |
//!
//! Two consequences a summary tends to get backwards: the personal
//! `token_cmd` outranks the shared `token` value, and the shared file is
//! consulted only when the personal one does not exist at all — not merely
//! when it carries no token, and not when it is ignored as insecure.
//!
//! Four deliberate hardening choices, each made because the safer behaviour
//! is worth it rather than for convenience:
//!
//! - a `token_cmd` in the shared config is **refused** rather than warned
//!   about and skipped — a silently-ignored credential source is worse than
//!   a loud one
//! - a personal `token` or `token_cmd` whose provenance file is VCS-tracked
//!   is refused: a repository-relative `config.local.md` can be committed,
//!   and `.gitignore` does not apply to an already-tracked file, so
//!   a hostile repository could otherwise supply a command a fresh clone
//!   executes, and a committed value is a leaked credential
//! - the helper runs only through the consent policy's runner, under the
//!   command key's own policy, so the one deliberately executed foreign code
//!   path is no more privileged than it needs to be
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

use crate::consent::CommandExecution;
use crate::consent::CommandKey;
use crate::consent::ConfigFileTracking;
pub use crate::consent::Environment;
use crate::consent::ProvenanceContext;
use crate::consent::Rejection;
use crate::render::render_value;
use crate::service::ConfigAccess;
use crate::service::PersonalFile;
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
    pub command: CommandKey,
}

/// Whether a file is tracked by the repository's VCS — the property that
/// decides whether a command-valued or allowlist-valued key may be honoured.
pub trait Provenance {
    fn is_tracked(&self, path: &Path) -> bool;
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

/// Repo-relative path of the insecure-local override marker.
pub const INSECURE_MARKER_RELATIVE: &str = ".accelerator/allow-insecure-local";

/// Everything the ladder reads beyond the keys themselves.
pub struct CredentialContext<'a> {
    pub environment: &'a dyn Environment,
    pub config: &'a dyn ConfigAccess,
    pub provenance: &'a dyn Provenance,
    pub tracking: &'a dyn ConfigFileTracking,
    pub files: &'a dyn FileFacts,
    pub execution: CommandExecution<'a>,
    pub personal_config: PathBuf,
    pub insecure_marker: PathBuf,
}

impl CredentialContext<'_> {
    /// The consent policy's view of the same ports.
    #[must_use]
    pub fn provenance(&self) -> ProvenanceContext<'_> {
        ProvenanceContext {
            config: self.config,
            tracking: self.tracking,
            environment: self.environment,
            personal_config: self.personal_config.clone(),
        }
    }
}

/// Why no token could be resolved.
///
/// `Debug` is hand-written and redacting: a helper that prints a secret and
/// then fails must not leak it into a CI log through a `{:?}` of the error.
#[derive(Clone, PartialEq, Eq)]
pub enum CredentialError {
    NoToken { key: String },
    Consent(Rejection),
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
            Self::Consent(rejection) => rejection.fatal.fmt(formatter),
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
            Self::Consent(rejection) => {
                return formatter
                    .debug_tuple("Consent")
                    .field(rejection)
                    .finish();
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
        let value = run_token_command(context, keys, &command)?;
        return accept(value, TokenSource::EnvCommand, &key_name(&keys.value));
    }

    match context.config.personal_file() {
        PersonalFile::Ignored { path, mode } => {
            Err(CredentialError::LocalPermsInsecure {
                path: path.clone(),
                mode: *mode,
            })
        }
        PersonalFile::Readable => resolve_personal(context, keys),
        PersonalFile::Absent => resolve_shared(context, keys),
    }
}

fn resolve_personal(
    context: &CredentialContext<'_>,
    keys: &TokenKeys,
) -> Result<ResolvedToken, CredentialError> {
    if let Some(value) =
        level_value(context.config, &keys.value, Level::Personal)?
    {
        refuse_tracked_value(context, &key_name(&keys.value))?;
        return accept(value, TokenSource::Personal, &key_name(&keys.value));
    }

    if let Some(command) =
        level_value(context.config, &command_key(keys)?, Level::Personal)?
    {
        refuse_tracked_source(
            context.provenance,
            &context.personal_config,
            keys.command.descriptor().name,
        )?;
        let value = run_token_command(context, keys, &command)?;
        return accept(
            value,
            TokenSource::PersonalCommand,
            &key_name(&keys.value),
        );
    }

    Err(CredentialError::NoToken {
        key: key_name(&keys.value),
    })
}

fn resolve_shared(
    context: &CredentialContext<'_>,
    keys: &TokenKeys,
) -> Result<ResolvedToken, CredentialError> {
    if level_value(context.config, &command_key(keys)?, Level::Team)?.is_some()
    {
        return Err(CredentialError::TokenCmdFromSharedConfig {
            key: keys.command.descriptor().name.to_owned(),
        });
    }
    if let Some(value) = level_value(context.config, &keys.value, Level::Team)?
    {
        return accept(value, TokenSource::Shared, &key_name(&keys.value));
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

fn command_key(keys: &TokenKeys) -> Result<Key, CredentialError> {
    let name = keys.command.descriptor().name;
    Key::parse(name).map_err(|error| CredentialError::ConfigUnreadable {
        key: name.to_owned(),
        detail: error.to_string(),
    })
}

fn run_token_command(
    context: &CredentialContext<'_>,
    keys: &TokenKeys,
    command: &str,
) -> Result<String, CredentialError> {
    context
        .execution
        .run(keys.command, command)
        .map_err(|refusal| CredentialError::Consent(Rejection::alone(refusal)))
}

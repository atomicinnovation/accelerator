//! The five-rung credential ladder every credential consumer climbs.
//!
//! | # | Source | Notes |
//! |---|---|---|
//! | 1 | the plaintext key's environment overrides | |
//! | 2 | the command key's environment overrides | a second environment source |
//! | 3 | `config.local.md` plaintext value | only when the file is present |
//! | 4 | `config.local.md` command | only when the file is readable |
//! | 5 | `config.md` plaintext value | only when `config.local.md` is absent |
//!
//! The first rung that yields a usable value wins. A refused value or a failed
//! command falls through to the next rung, and every refusal met on the way
//! travels back through the consent policy's one ordered channel, so a
//! consumer decides nothing about severity itself.
//!
//! Two consequences a summary tends to get backwards: the personal command
//! outranks the shared plaintext value, and the shared file is consulted only
//! when the personal one does not exist at all — not merely when it carries no
//! token, and not when it is ignored as insecure.
//!
//! A command key is a consent key, so a team-level command is always refused
//! and a command runs only through the policy's runner. A personal plaintext
//! value from a tracked or undeterminable `config.local.md` is refused too:
//! `.gitignore` does not apply to an already-tracked file, so a committed
//! value is a leaked credential.

use std::error::Error;
use std::fmt;

use crate::catalogue;
use crate::catalogue::ExtraKey;
use crate::consent;
use crate::consent::Aborted;
use crate::consent::CommandExecution;
use crate::consent::CommandKey;
pub use crate::consent::Environment;
use crate::consent::Ladder;
use crate::consent::Notice;
use crate::consent::ProvenanceContext;
use crate::consent::Refusal;
use crate::consent::Rejection;
use crate::consent::Rung;
use crate::consent::Usable;
use crate::error::ConfigError;
use crate::service::PersonalFile;
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

/// A resolved token, the rung it came from, and every refusal and notice met
/// on the way for the caller to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToken {
    pub value: Secret,
    pub source: TokenSource,
    pub refusals: Vec<Refusal>,
    pub notice: Option<Notice>,
}

/// The two keys one provider's ladder climbs: a plaintext credential and the
/// command that prints one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenKeys {
    pub plaintext: &'static ExtraKey,
    pub command: CommandKey,
}

impl TokenKeys {
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] unless the catalogue declares `plaintext` and
    /// declares `command` as a command-valued consent key.
    pub fn declared(
        plaintext: &str,
        command: &str,
    ) -> Result<Self, ConfigError> {
        let declared = catalogue::declared(plaintext).ok_or_else(|| {
            ConfigError::Invalid {
                detail: format!("{plaintext} is not a declared key"),
            }
        })?;
        Ok(Self {
            plaintext: declared,
            command: CommandKey::declared(command)?,
        })
    }
}

/// Everything the ladder reads beyond the keys themselves.
pub struct CredentialContext<'a> {
    pub provenance: ProvenanceContext<'a>,
    pub execution: CommandExecution<'a>,
}

/// Why no token could be resolved.
///
/// `Debug` is hand-written and redacting: a helper that prints a secret and
/// then fails must not leak it into a CI log through a `{:?}` of the error.
#[derive(Clone, PartialEq, Eq)]
pub enum CredentialError {
    NoToken { key: String },
    Consent(Rejection),
    ConfigUnreadable(Aborted),
}

impl CredentialError {
    /// Every refusal to report as a warning before this failure.
    #[must_use]
    pub fn warnings(&self) -> &[Refusal] {
        match self {
            Self::NoToken { .. } => &[],
            Self::Consent(rejection) => &rejection.warnings,
            Self::ConfigUnreadable(aborted) => &aborted.warnings,
        }
    }
}

impl From<Aborted> for CredentialError {
    fn from(aborted: Aborted) -> Self {
        Self::ConfigUnreadable(aborted)
    }
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
            Self::ConfigUnreadable(aborted) => aborted.error.fmt(formatter),
        }
    }
}

impl fmt::Debug for CredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoToken { key } => {
                formatter.debug_struct("NoToken").field("key", key).finish()
            }
            Self::Consent(rejection) => {
                formatter.debug_tuple("Consent").field(rejection).finish()
            }
            Self::ConfigUnreadable(aborted) => formatter
                .debug_struct("ConfigUnreadable")
                .field("warnings", &aborted.warnings)
                .finish_non_exhaustive(),
        }
    }
}

impl Error for CredentialError {}

/// Climbs the ladder for one provider's keys.
///
/// # Errors
///
/// [`CredentialError::Consent`] when refusals left nothing usable,
/// [`CredentialError::NoToken`] when nothing was configured, and
/// [`CredentialError::ConfigUnreadable`] when a level could not be read.
pub fn resolve_token(
    context: &CredentialContext<'_>,
    keys: &TokenKeys,
) -> Result<ResolvedToken, CredentialError> {
    let provenance = &context.provenance;
    let execution = &context.execution;
    let plaintext = keys.plaintext;
    let commands = consent::resolve_command(provenance, keys.command)?;
    let mut ladder = Ladder::new(commands.team_level_refusals().to_vec());

    ladder.offer(TokenSource::Env, plaintext, || {
        Ok(environment_rung(provenance.environment, plaintext))
    })?;
    ladder.attempt(
        TokenSource::EnvCommand,
        || Ok(commands.environment()),
        execution,
    )?;
    match provenance.config.personal_file() {
        PersonalFile::Readable => {
            ladder.offer(TokenSource::Personal, plaintext, || {
                consent::personal_plaintext(provenance, plaintext)
            })?;
            ladder.attempt(
                TokenSource::PersonalCommand,
                || commands.personal(),
                execution,
            )?;
        }
        PersonalFile::Ignored { .. } => {
            ladder.offer(TokenSource::Personal, plaintext, || {
                consent::personal_plaintext(provenance, plaintext)
            })?;
        }
        PersonalFile::Absent => {
            ladder.offer(TokenSource::Shared, plaintext, || {
                Ok(consent::raw_value(
                    provenance.config,
                    plaintext,
                    Level::Team,
                )?
                .map_or(Rung::Absent, Rung::Candidate))
            })?;
        }
    }

    match ladder.finish().or_fallback(None) {
        Usable::Value {
            value,
            warnings,
            notice,
        } => Ok(ResolvedToken {
            value: Secret::new(value.value),
            source: value.source,
            refusals: warnings,
            notice,
        }),
        Usable::Refused(rejection) => Err(CredentialError::Consent(rejection)),
        Usable::Absent => Err(CredentialError::NoToken {
            key: plaintext.name.to_owned(),
        }),
    }
}

fn environment_rung(
    environment: &dyn Environment,
    key: &ExtraKey,
) -> Rung<String> {
    consent::environment_candidate(environment, key)
        .map_or(Rung::Absent, |(_, value)| Rung::Candidate(value))
}

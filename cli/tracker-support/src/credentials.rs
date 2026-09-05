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
//! - a `token_cmd` whose provenance file is VCS-tracked is refused: a
//!   repository-relative `config.local.md` can simply be committed, and
//!   `.gitignore` does not apply to an already-tracked file, so a hostile
//!   repository could otherwise supply a command a fresh clone executes
//! - the helper runs under a wall-clock timeout, an output cap, a scrubbed
//!   environment and a defined working directory, so the one deliberately
//!   executed foreign code path is no more privileged than it needs to be
//! - its output is never folded into an error, and [`Secret`] and
//!   [`CredentialError`] both redact under `Debug`

use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::io::Read as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use config::ConfigAccess;
use config::Key;
use config::Level;
use config::Resolved;

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

/// The production environment.
pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
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
    #[must_use]
    pub const fn rooted_at(working_directory: PathBuf) -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_output_bytes: 64 * 1024,
            working_directory,
        }
    }
}

/// Repo-relative path of the insecure-local override marker.
pub const INSECURE_MARKER_RELATIVE: &str = ".accelerator/allow-insecure-local";

/// Everything the ladder reads beyond the keys themselves.
pub struct CredentialContext<'a> {
    pub environment: &'a dyn Environment,
    pub config: &'a dyn ConfigAccess,
    pub provenance: &'a dyn Provenance,
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
                write!(formatter, "E_TOKEN_CMD_FAILED: {key}_cmd {detail}")
            }
            Self::TokenCmdTimedOut { key, after } => write!(
                formatter,
                "E_TOKEN_CMD_FAILED: {key}_cmd did not finish within {}s",
                after.as_secs()
            ),
            Self::TokenCmdFromSharedConfig { key } => write!(
                formatter,
                "E_TOKEN_CMD_FROM_SHARED_CONFIG: {key}_cmd in config.md \
                 refused — move it to config.local.md"
            ),
            Self::TokenCmdFromTrackedFile { key, path } => write!(
                formatter,
                "E_TOKEN_CMD_FROM_TRACKED_FILE: {key}_cmd comes from {}, \
                 which is tracked by version control — a command a clone \
                 would run is refused",
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
        let value = run_token_command(
            &command,
            &context.command,
            &key_name(&keys.command),
        )?;
        return accept(value, TokenSource::EnvCommand, &key_name(&keys.value));
    }

    if context.personal_config.exists() {
        refuse_insecure_personal_config(context)?;

        if let Some(value) =
            level_value(context.config, &keys.value, Level::Personal)?
        {
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
            let value = run_token_command(
                &command,
                &context.command,
                &key_name(&keys.command),
            )?;
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
        Resolved::Found(value) => nonempty(Some(config::render_value(&value))),
        Resolved::Absent => None,
    })
}

/// The mode-0600 gate on the personal config file, with an override:
/// `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` counts only when
/// `.accelerator/allow-insecure-local` is a regular, non-symlink, VCS-tracked
/// file.
fn refuse_insecure_personal_config(
    context: &CredentialContext<'_>,
) -> Result<(), CredentialError> {
    let path = &context.personal_config;
    let facts = std::fs::symlink_metadata(path).map_err(|error| {
        CredentialError::ConfigUnreadable {
            key: path.display().to_string(),
            detail: error.to_string(),
        }
    })?;
    if facts.file_type().is_symlink() {
        return Err(CredentialError::LocalPermsInsecure {
            path: path.clone(),
            mode: 0,
        });
    }
    let mode = file_mode(&facts);
    if mode.trailing_zeros() >= 6 {
        return Ok(());
    }
    if insecure_override_allowed(context) {
        return Ok(());
    }
    Err(CredentialError::LocalPermsInsecure {
        path: path.clone(),
        mode,
    })
}

#[cfg(unix)]
fn file_mode(facts: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt as _;
    facts.mode() & 0o7777
}

#[cfg(not(unix))]
const fn file_mode(_facts: &std::fs::Metadata) -> u32 {
    0o600
}

fn insecure_override_allowed(context: &CredentialContext<'_>) -> bool {
    if context.environment.read("ACCELERATOR_ALLOW_INSECURE_LOCAL")
        != Some("1".to_owned())
    {
        return false;
    }
    let marker = &context.insecure_marker;
    std::fs::symlink_metadata(marker).is_ok_and(|facts| {
        facts.file_type().is_file() && context.provenance.is_tracked(marker)
    })
}

/// Runs a credential helper under [`CommandPolicy`].
///
/// The helper's stdout never reaches an error, its stderr is discarded, and
/// only the trailing newline is trimmed from what it printed.
fn run_token_command(
    command: &str,
    policy: &CommandPolicy,
    key: &str,
) -> Result<String, CredentialError> {
    let scrubbed: Vec<(String, OsString)> = ["PATH", "HOME", "TERM"]
        .iter()
        .filter_map(|name| {
            std::env::var_os(name).map(|value| ((*name).to_owned(), value))
        })
        .collect();

    let mut child = Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(&policy.working_directory)
        .env_clear()
        .envs(scrubbed)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| CredentialError::TokenCmdFailed {
            key: key.to_owned(),
            detail: format!("could not be run: {error}"),
        })?;

    let stdout = child.stdout.take();
    let cap = policy.max_output_bytes;
    let reader = std::thread::spawn(move || {
        let mut captured = Vec::new();
        if let Some(stream) = stdout {
            let mut bounded = stream.take(cap as u64);
            let _ = bounded.read_to_end(&mut captured);
        }
        captured
    });

    let deadline = Instant::now() + policy.timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(CredentialError::TokenCmdTimedOut {
                        key: key.to_owned(),
                        after: policy.timeout,
                    });
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => {
                break Err(CredentialError::TokenCmdFailed {
                    key: key.to_owned(),
                    detail: format!("could not be awaited: {error}"),
                })
            }
        }
    };

    let captured = reader.join().unwrap_or_default();
    let status = status?;

    if !status.success() {
        return Err(CredentialError::TokenCmdFailed {
            key: key.to_owned(),
            detail: format!("exited with {status}"),
        });
    }

    let printed = String::from_utf8(captured).map_err(|_| {
        CredentialError::TokenCmdFailed {
            key: key.to_owned(),
            detail: "produced non-UTF-8 output".to_owned(),
        }
    })?;
    Ok(printed
        .strip_suffix('\n')
        .map_or_else(|| printed.clone(), str::to_owned))
}

//! The trust barrier for consent keys, and the one ordered refusal channel
//! every credential rung reports through.
//!
//! A consent key holds a value only the user may supply. A team-level value is
//! always refused, and so is a value read from a `config.local.md` that is
//! VCS-tracked or whose tracking cannot be determined. An environment override
//! skips those provenance checks: whoever controls the environment is the
//! user.
//!
//! The highest-precedence source that passes every check wins, and every
//! refusal met on the way is reported, the team-level one last. Severity is
//! decided in one place, [`Consented::or_fallback`]: a refusal is fatal only
//! when no usable value remains.
//!
//! Plaintext credentials are not consent keys, but their refusals travel in
//! the same channel, so a consumer orders and renders every refusal alike.
//!
//! A path-valued consent key must also name an absolute executable that can
//! be shown to lie outside the repository.
//!
//! The policy is pure: the environment, the tracking answer, the config
//! levels and the filesystem facts all arrive through ports.

use std::fmt;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use crate::catalogue;
use crate::catalogue::ExtraKey;
use crate::catalogue::Trust;
use crate::error::ConfigError;
use crate::key::Key;
use crate::level::Level;
use crate::render::render_value;
use crate::service::ConfigAccess;
use crate::service::PersonalFile;
use crate::service::Resolved;

/// Environment reads, injected so a test needs no process state.
pub trait Environment {
    fn read(&self, name: &str) -> Option<String>;
}

/// Whether a file is tracked by the repository's VCS. `Unknown` means a VCS
/// was detected but could not answer, which the policy treats as tracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tracking {
    Untracked,
    Tracked,
    Unknown,
}

impl Tracking {
    #[must_use]
    pub const fn distrust(self) -> Option<Distrust> {
        match self {
            Self::Untracked => None,
            Self::Tracked => Some(Distrust::Tracked),
            Self::Unknown => Some(Distrust::Unknown),
        }
    }
}

/// Why a personal file must not be trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distrust {
    Tracked,
    Unknown,
}

impl fmt::Display for Distrust {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Tracked => "E_CONSENT_KEY_TRACKED",
            Self::Unknown => "E_CONSENT_KEY_TRACKING_UNKNOWN",
        })
    }
}

/// The warnings a consumer prints itself: an ignored personal file is left
/// out, because every composition root has already reported it once.
pub fn reportable(warnings: &[Refusal]) -> impl Iterator<Item = &Refusal> {
    warnings
        .iter()
        .filter(|warning| warning.reason() != RefusalReason::PersonalFile)
}

/// Whether the tracking question could be put at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingCheck {
    Known(Tracking),
    Unchecked,
}

/// Answers whether a config file is VCS-tracked.
pub trait ConfigFileTracking {
    fn tracking(&self, path: &Path) -> Tracking;

    /// As [`Self::tracking`], but able to report that the question could not
    /// be put at all. Only a whole-config [`audit`] asks this way, so it can
    /// tell an unreachable tracking service apart from an unknown answer.
    fn check(&self, path: &Path) -> TrackingCheck {
        TrackingCheck::Known(self.tracking(path))
    }
}

/// Why a value was not used.
#[derive(Clone, PartialEq, Eq)]
pub enum Refusal {
    TeamLevel {
        key: &'static ExtraKey,
    },
    UntrustedPersonalFile {
        key: &'static ExtraKey,
        path: PathBuf,
        distrust: Distrust,
    },
    InsecurePersonalFile {
        path: PathBuf,
        mode: u32,
    },
    CommandFailed {
        key: &'static ExtraKey,
        cause: FailureCause,
    },
    CommandTimedOut {
        key: &'static ExtraKey,
        after: Duration,
    },
    CommandOutputExceeded {
        key: &'static ExtraKey,
        limit: usize,
    },
    PlaintextFromUntrustedFile {
        key: &'static ExtraKey,
        path: PathBuf,
        distrust: Distrust,
    },
    MalformedToken {
        key: &'static ExtraKey,
    },
    PathRelative {
        key: &'static ExtraKey,
    },
    PathInsideRepository {
        key: &'static ExtraKey,
        path: PathBuf,
    },
}

/// The class of a refusal, which a consumer maps onto its own exit codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    Provenance,
    PersonalFile,
    Command,
    Malformed,
    Value,
}

/// Why a command produced no value. Only structured causes travel here, so a
/// command's output never reaches a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCause {
    CouldNotStart(StartFailure),
    /// The shell's convention: a leader killed by a signal reports
    /// `128 + signal`.
    Exited(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartFailure {
    NoBashOnPath,
    NoWorkingDirectoryOutsideTheRepository,
    SpawnFailed,
}

impl fmt::Display for FailureCause {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CouldNotStart(StartFailure::NoBashOnPath) => formatter
                .write_str("could not start: no bash on the filtered PATH"),
            Self::CouldNotStart(
                StartFailure::NoWorkingDirectoryOutsideTheRepository,
            ) => formatter.write_str(
                "could not start: no temporary directory outside the \
                 repository",
            ),
            Self::CouldNotStart(StartFailure::SpawnFailed) => {
                formatter.write_str("could not start")
            }
            Self::Exited(status) => {
                write!(formatter, "exited with status {status}")
            }
        }
    }
}

impl Refusal {
    /// The refusal a composition root reports once for an ignored personal
    /// file, or `None` when the file is absent or readable.
    #[must_use]
    pub fn for_personal_file(personal_file: &PersonalFile) -> Option<Self> {
        match personal_file {
            PersonalFile::Ignored { path, mode } => {
                Some(Self::InsecurePersonalFile {
                    path: path.clone(),
                    mode: *mode,
                })
            }
            PersonalFile::Absent | PersonalFile::Readable => None,
        }
    }

    #[must_use]
    pub const fn key(&self) -> Option<&'static ExtraKey> {
        match self {
            Self::TeamLevel { key }
            | Self::UntrustedPersonalFile { key, .. }
            | Self::CommandFailed { key, .. }
            | Self::CommandTimedOut { key, .. }
            | Self::CommandOutputExceeded { key, .. }
            | Self::PlaintextFromUntrustedFile { key, .. }
            | Self::MalformedToken { key }
            | Self::PathRelative { key }
            | Self::PathInsideRepository { key, .. } => Some(key),
            Self::InsecurePersonalFile { .. } => None,
        }
    }

    #[must_use]
    pub const fn reason(&self) -> RefusalReason {
        match self {
            Self::TeamLevel { .. }
            | Self::UntrustedPersonalFile { .. }
            | Self::PlaintextFromUntrustedFile { .. } => {
                RefusalReason::Provenance
            }
            Self::MalformedToken { .. } => RefusalReason::Malformed,
            Self::PathRelative { .. } | Self::PathInsideRepository { .. } => {
                RefusalReason::Value
            }
            Self::InsecurePersonalFile { .. } => RefusalReason::PersonalFile,
            Self::CommandFailed { .. }
            | Self::CommandTimedOut { .. }
            | Self::CommandOutputExceeded { .. } => RefusalReason::Command,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TeamLevel { key } => write!(
                formatter,
                "E_CONSENT_KEY_TEAM_LEVEL: {} in .accelerator/config.md is \
                 refused — only you may set it; move it to \
                 .accelerator/config.local.md",
                key.name
            ),
            Self::UntrustedPersonalFile {
                key,
                path,
                distrust,
            } => untrusted_file(
                formatter,
                &distrust.to_string(),
                key,
                path,
                *distrust,
                "the repository chose the value",
            ),
            Self::PlaintextFromUntrustedFile {
                key,
                path,
                distrust,
            } => untrusted_file(
                formatter,
                "E_TOKEN_FROM_TRACKED_FILE",
                key,
                path,
                *distrust,
                "every clone carries the credential",
            ),
            Self::MalformedToken { key } => write!(
                formatter,
                "E_TOKEN_MALFORMED: {} yielded a value carrying a control \
                 character",
                key.name
            ),
            Self::InsecurePersonalFile { path, mode } => write!(
                formatter,
                "E_LOCAL_PERMS_INSECURE: {} is mode {mode:04o}; ignored — run \
                 chmod 600 on it (it must not be a symlink), or where file \
                 modes cannot be honoured, keep team values in \
                 .accelerator/config.md and secrets in the ACCELERATOR_* \
                 overrides",
                escaped(&path.display().to_string())
            ),
            Self::CommandFailed { key, cause } => {
                write!(formatter, "E_TOKEN_CMD_FAILED: {} {cause}", key.name)
            }
            Self::CommandTimedOut { key, after } => write!(
                formatter,
                "E_COMMAND_TIMED_OUT: {} did not finish within {}",
                key.name,
                Seconds(*after)
            ),
            Self::CommandOutputExceeded { key, limit } => write!(
                formatter,
                "E_COMMAND_OUTPUT_EXCEEDED: {} printed more than {limit} bytes",
                key.name
            ),
            Self::PathRelative { key } => write!(
                formatter,
                "E_EXECUTABLE_PATH_RELATIVE: {} is a relative path and is \
                 refused — set it to an absolute path outside the repository",
                key.name
            ),
            Self::PathInsideRepository { key, path } => write!(
                formatter,
                "E_EXECUTABLE_PATH_INSIDE_REPOSITORY: {} ({}) is inside, or \
                 cannot be shown to be outside, the repository and is refused \
                 — set it to an absolute path outside the repository",
                key.name,
                escaped(&path.display().to_string())
            ),
        }
    }
}

struct Seconds(Duration);

impl fmt::Display for Seconds {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.subsec_nanos() == 0 {
            write!(formatter, "{}s", self.0.as_secs())
        } else {
            write!(formatter, "{:.1}s", self.0.as_secs_f64())
        }
    }
}

fn untrusted_file(
    formatter: &mut fmt::Formatter<'_>,
    code: &str,
    key: &ExtraKey,
    path: &Path,
    distrust: Distrust,
    consequence: &str,
) -> fmt::Result {
    write!(
        formatter,
        "{code}: {} in {} is refused — ",
        key.name,
        escaped(&path.display().to_string())
    )?;
    match distrust {
        Distrust::Tracked => write!(
            formatter,
            "the file is tracked by version control, so {consequence}; \
             untrack it"
        ),
        Distrust::Unknown => {
            formatter.write_str(
                "whether the file is tracked by version control could not \
                 be determined",
            )?;
            recovery_hint(formatter, key)
        }
    }
}

fn recovery_hint(
    formatter: &mut fmt::Formatter<'_>,
    key: &ExtraKey,
) -> fmt::Result {
    key.recovery_hint().map_or(Ok(()), |hint| {
        write!(formatter, "; set {hint} in the environment")
    })
}

/// Redacts every path and value: a refusal may reach a CI log through `{:?}`.
impl fmt::Debug for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TeamLevel { key } => formatter
                .debug_struct("TeamLevel")
                .field("key", &key.name)
                .finish(),
            Self::UntrustedPersonalFile { key, distrust, .. } => formatter
                .debug_struct("UntrustedPersonalFile")
                .field("key", &key.name)
                .field("distrust", distrust)
                .finish_non_exhaustive(),
            Self::InsecurePersonalFile { mode, .. } => formatter
                .debug_struct("InsecurePersonalFile")
                .field("mode", mode)
                .finish_non_exhaustive(),
            Self::CommandFailed { key, cause } => formatter
                .debug_struct("CommandFailed")
                .field("key", &key.name)
                .field("cause", cause)
                .finish(),
            Self::CommandTimedOut { key, after } => formatter
                .debug_struct("CommandTimedOut")
                .field("key", &key.name)
                .field("after", after)
                .finish(),
            Self::CommandOutputExceeded { key, limit } => formatter
                .debug_struct("CommandOutputExceeded")
                .field("key", &key.name)
                .field("limit", limit)
                .finish(),
            Self::PlaintextFromUntrustedFile { key, distrust, .. } => formatter
                .debug_struct("PlaintextFromUntrustedFile")
                .field("key", &key.name)
                .field("distrust", distrust)
                .finish_non_exhaustive(),
            Self::MalformedToken { key } => formatter
                .debug_struct("MalformedToken")
                .field("key", &key.name)
                .finish(),
            Self::PathRelative { key } => formatter
                .debug_struct("PathRelative")
                .field("key", &key.name)
                .finish(),
            Self::PathInsideRepository { key, .. } => formatter
                .debug_struct("PathInsideRepository")
                .field("key", &key.name)
                .finish_non_exhaustive(),
        }
    }
}

/// Escapes control characters, so a crafted path or value cannot forge or hide
/// an output line.
fn escaped(text: &str) -> String {
    let mut rendered = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_control() {
            rendered.extend(character.escape_default());
        } else {
            rendered.push(character);
        }
    }
    rendered
}

/// A key declared as [`Trust::Consent`]: a value with no checks beyond
/// provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsentKey(&'static ExtraKey);

/// A key declared as [`Trust::CommandConsent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandKey(ConsentKey);

/// A key declared as [`Trust::PathConsent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutablePathKey(ConsentKey);

impl ConsentKey {
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] unless the catalogue declares `name` as a
    /// plain consent key.
    pub fn declared(name: &str) -> Result<Self, ConfigError> {
        declared_as(name, "a consent key", |trust| trust == Trust::Consent)
            .map(Self)
    }

    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn for_test(name: &str) -> Self {
        Self(leaked(name, Trust::Consent))
    }

    #[must_use]
    pub const fn descriptor(&self) -> &'static ExtraKey {
        self.0
    }
}

impl CommandKey {
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] unless the catalogue declares `name` as a
    /// command-valued consent key.
    pub fn declared(name: &str) -> Result<Self, ConfigError> {
        declared_as(name, "a command-valued consent key", |trust| {
            matches!(trust, Trust::CommandConsent { .. })
        })
        .map(|key| Self(ConsentKey(key)))
    }

    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn for_test(
        name: &str,
        admitted_environment: &'static [&'static str],
    ) -> Self {
        Self(ConsentKey(leaked(
            name,
            Trust::CommandConsent {
                admitted_environment,
            },
        )))
    }

    #[must_use]
    pub const fn descriptor(&self) -> &'static ExtraKey {
        self.0 .0
    }
}

impl ExecutablePathKey {
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] unless the catalogue declares `name` as a
    /// path-valued consent key.
    pub fn declared(name: &str) -> Result<Self, ConfigError> {
        declared_as(name, "a path-valued consent key", |trust| {
            trust == Trust::PathConsent
        })
        .map(|key| Self(ConsentKey(key)))
    }

    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn for_test(name: &str) -> Self {
        Self(ConsentKey(leaked(name, Trust::PathConsent)))
    }

    #[must_use]
    pub const fn descriptor(&self) -> &'static ExtraKey {
        self.0 .0
    }
}

fn declared_as(
    name: &str,
    kind: &str,
    admits: impl Fn(Trust) -> bool,
) -> Result<&'static ExtraKey, ConfigError> {
    catalogue::declared(name)
        .filter(|key| admits(key.trust))
        .ok_or_else(|| ConfigError::Invalid {
            detail: format!("{name} is not declared as {kind}"),
        })
}

#[cfg(feature = "test-support")]
fn leaked(name: &str, trust: Trust) -> &'static ExtraKey {
    Box::leak(Box::new(ExtraKey {
        name: Box::leak(name.to_owned().into_boxed_str()),
        trust,
        overrides: &[],
        recovery: None,
    }))
}

/// What "inside the repository" means: the config root, and each root of
/// every repository enclosing the working directory.
///
/// Incomplete when a repository was detected but one of its roots could not
/// be determined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRoots {
    roots: Vec<PathBuf>,
    complete: bool,
}

impl RepositoryRoots {
    #[must_use]
    pub const fn complete(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            complete: true,
        }
    }

    #[must_use]
    pub const fn incomplete(known: Vec<PathBuf>) -> Self {
        Self {
            roots: known,
            complete: false,
        }
    }

    /// Whether `canonical`, already canonicalised by the caller, lies at or
    /// beneath any root.
    #[must_use]
    pub fn contains(&self, canonical: &Path) -> bool {
        self.roots.iter().any(|root| canonical.starts_with(root))
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }
}

/// The filesystem facts the path checks read, injected so the policy touches
/// no filesystem.
pub trait ExecutablePaths {
    /// The real location of an existing path, or `None` when it cannot be
    /// resolved.
    fn canonicalise(&self, existing: &Path) -> Option<PathBuf>;

    /// A symlink's target exactly as stored, relative or absolute, or `None`
    /// when `path` is not a symlink.
    fn link_target(&self, path: &Path) -> Option<PathBuf>;

    /// Whether an entry exists at `path` without following a final symlink,
    /// so a dangling symlink exists.
    fn exists(&self, path: &Path) -> bool;
}

/// Linux's `MAXSYMLINKS`.
pub const SYMLINK_HOP_LIMIT: usize = 40;

/// Vets a path-valued consent key's value: it must be absolute and resolve,
/// through every symlink, to a canonical path outside every repository root.
///
/// # Errors
///
/// [`Refusal::PathRelative`] for a relative value, and
/// [`Refusal::PathInsideRepository`] for every value that cannot be shown to
/// lie outside the repository: inside a root, incomplete roots, a symlink
/// chain past [`SYMLINK_HOP_LIMIT`], a `..` beneath a missing directory, or a
/// location that cannot be canonicalised.
pub fn vet_executable_path(
    key: &ExecutablePathKey,
    value: &str,
    roots: &RepositoryRoots,
    paths: &dyn ExecutablePaths,
) -> Result<PathBuf, Refusal> {
    let descriptor = key.descriptor();
    let supplied = Path::new(value);
    if !supplied.is_absolute() {
        return Err(Refusal::PathRelative { key: descriptor });
    }
    let inside = || Refusal::PathInsideRepository {
        key: descriptor,
        path: supplied.to_path_buf(),
    };
    if !roots.is_complete() {
        return Err(inside());
    }
    canonical_location(paths, supplied)
        .filter(|canonical| !roots.contains(canonical))
        .ok_or_else(inside)
}

fn canonical_location(
    paths: &dyn ExecutablePaths,
    supplied: &Path,
) -> Option<PathBuf> {
    let mut current = supplied.to_path_buf();
    for _ in 0..=SYMLINK_HOP_LIMIT {
        if let Some(target) = paths.link_target(&current) {
            current = beside_link(&current, target);
            continue;
        }
        let (ancestor, remainder) = nearest_existing_ancestor(paths, &current)?;
        if let Some(target) = paths.link_target(&ancestor) {
            current = beside_link(&ancestor, target).join(remainder);
            continue;
        }
        if remainder
            .components()
            .any(|component| component == Component::ParentDir)
        {
            return None;
        }
        return paths
            .canonicalise(&ancestor)
            .map(|real| real.join(remainder));
    }
    None
}

fn beside_link(link: &Path, target: PathBuf) -> PathBuf {
    if target.is_absolute() {
        return target;
    }
    match link.parent() {
        Some(parent) => parent.join(target),
        None => target,
    }
}

fn nearest_existing_ancestor(
    paths: &dyn ExecutablePaths,
    path: &Path,
) -> Option<(PathBuf, PathBuf)> {
    let mut ancestor = path.to_path_buf();
    let mut missing = Vec::new();
    while !paths.exists(&ancestor) {
        missing.push(ancestor.components().next_back()?.as_os_str().to_owned());
        ancestor = ancestor.parent()?.to_path_buf();
    }
    Some((ancestor, missing.iter().rev().collect()))
}

/// The bounds one command runs under. Only the policy builds one for a real
/// key, so the environment a command sees is always its descriptor's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPolicy {
    timeout: Duration,
    admitted_environment: Vec<&'static str>,
}

impl CommandPolicy {
    pub const OUTPUT_LIMIT: usize = 65_536;
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

    pub(crate) fn for_key(key: CommandKey, timeout: Duration) -> Self {
        let declared = match key.descriptor().trust {
            Trust::CommandConsent {
                admitted_environment,
            } => admitted_environment,
            Trust::Open | Trust::Consent | Trust::PathConsent => &[],
        };
        Self {
            timeout,
            admitted_environment: catalogue::BASE_COMMAND_ENVIRONMENT
                .iter()
                .chain(declared)
                .copied()
                .collect(),
        }
    }

    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn for_test(
        timeout: Duration,
        admitted_environment: &[&'static str],
    ) -> Self {
        Self {
            timeout,
            admitted_environment: admitted_environment.to_vec(),
        }
    }

    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    #[must_use]
    pub fn admitted_environment(&self) -> &[&'static str] {
        &self.admitted_environment
    }
}

/// Why a command run produced no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandFailure {
    Failed(FailureCause),
    TimedOut,
    OutputExceeded,
}

/// Runs a consented command, injected so the policy spawns no process.
pub trait CommandRunner {
    /// Returns the command's stdout trimmed of surrounding whitespace.
    ///
    /// # Errors
    ///
    /// [`CommandFailure`] when the command cannot start, exits non-zero,
    /// outlives the policy's timeout, or prints past
    /// [`CommandPolicy::OUTPUT_LIMIT`].
    fn run(
        &self,
        command: &str,
        policy: &CommandPolicy,
    ) -> Result<String, CommandFailure>;
}

/// A command runner a consumer can hold and hand on, but never run: only the
/// policy runs a command, and only one it has resolved.
///
/// ```compile_fail
/// # use config::consent::{CommandPolicy, Runner};
/// fn attempt(runner: &Runner, policy: &CommandPolicy) {
///     let _ = runner.run("printf token", policy);
/// }
/// ```
///
/// ```compile_fail
/// # use config::consent::{CommandKey, CommandPolicy};
/// let key = CommandKey::declared("jira.token_cmd").unwrap();
/// let _ = CommandPolicy::for_key(key, CommandPolicy::DEFAULT_TIMEOUT);
/// ```
pub struct Runner(Box<dyn CommandRunner>);

impl Runner {
    #[must_use]
    pub fn new(runner: Box<dyn CommandRunner>) -> Self {
        Self(runner)
    }

    pub(crate) fn run(
        &self,
        command: &str,
        policy: &CommandPolicy,
    ) -> Result<String, CommandFailure> {
        self.0.run(command, policy)
    }
}

/// How a consented command is run: through which runner, and for how long.
pub struct CommandExecution<'a> {
    pub runner: &'a Runner,
    pub timeout: Duration,
}

impl CommandExecution<'_> {
    /// Runs `command` for `key` under its descriptor's policy, refusing on any
    /// failure.
    pub(crate) fn run(
        &self,
        key: CommandKey,
        command: &str,
    ) -> Result<String, Refusal> {
        let descriptor = key.descriptor();
        self.runner
            .run(command, &CommandPolicy::for_key(key, self.timeout))
            .map_err(|failure| match failure {
                CommandFailure::Failed(cause) => Refusal::CommandFailed {
                    key: descriptor,
                    cause,
                },
                CommandFailure::TimedOut => Refusal::CommandTimedOut {
                    key: descriptor,
                    after: self.timeout,
                },
                CommandFailure::OutputExceeded => {
                    Refusal::CommandOutputExceeded {
                        key: descriptor,
                        limit: CommandPolicy::OUTPUT_LIMIT,
                    }
                }
            })
    }
}

/// That a consent key was taken from the environment, which skips the
/// provenance checks, so the user can see which variable decided it.
#[derive(Clone, PartialEq, Eq)]
pub struct Notice {
    key: &'static ExtraKey,
    variable: &'static str,
    value: Option<String>,
}

impl Notice {
    /// A command is never shown: it may carry a secret inline.
    #[must_use]
    pub fn new(
        key: &'static ExtraKey,
        variable: &'static str,
        value: &str,
    ) -> Self {
        let shown = !matches!(key.trust, Trust::CommandConsent { .. });
        Self {
            key,
            variable,
            value: shown.then(|| value.to_owned()),
        }
    }
}

impl fmt::Display for Notice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "notice: {} taken from {}",
            self.key.name, self.variable
        )?;
        self.value
            .as_ref()
            .map_or(Ok(()), |value| write!(formatter, ": {}", escaped(value)))
    }
}

impl fmt::Debug for Notice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Notice")
            .field("key", &self.key.name)
            .field("variable", &self.variable)
            .finish_non_exhaustive()
    }
}

/// What the policy decided: the admitted value, if any, and every refusal met
/// on the way, in precedence order with the team-level refusal last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consented<T> {
    pub admitted: Option<T>,
    pub refusals: Vec<Refusal>,
    pub notice: Option<Notice>,
}

/// A consent failure: the refusal that left nothing usable, and every other
/// refusal as a warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    pub fatal: Refusal,
    pub warnings: Vec<Refusal>,
}

impl Rejection {
    #[must_use]
    pub const fn alone(fatal: Refusal) -> Self {
        Self {
            fatal,
            warnings: Vec::new(),
        }
    }
}

/// A consumer's verdict once its own fallback is weighed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Usable<T> {
    Value {
        value: T,
        warnings: Vec<Refusal>,
        notice: Option<Notice>,
    },
    Refused(Rejection),
    Absent,
}

impl<T> Consented<T> {
    /// The one owner of refusal ordering: the refusals of the candidates
    /// tried, in precedence order, then the team-level refusals.
    pub(crate) fn from_candidates(
        admitted: Option<T>,
        candidate_refusals: Vec<Refusal>,
        team_refusals: Vec<Refusal>,
    ) -> Self {
        let (mut refusals, mut team_level): (Vec<_>, Vec<_>) =
            candidate_refusals
                .into_iter()
                .chain(team_refusals)
                .partition(|refusal| {
                    !matches!(refusal, Refusal::TeamLevel { .. })
                });
        refusals.append(&mut team_level);
        Self {
            admitted,
            refusals,
            notice: None,
        }
    }

    fn noticed(mut self, notice: Option<Notice>) -> Self {
        self.notice = notice;
        self
    }

    #[must_use]
    pub fn map<U>(self, transform: impl FnOnce(T) -> U) -> Consented<U> {
        Consented {
            admitted: self.admitted.map(transform),
            refusals: self.refusals,
            notice: self.notice,
        }
    }

    /// Decides severity: with a usable value, admitted or `fallback`, every
    /// refusal is a warning; with none, the highest-precedence refusal is
    /// fatal and the rest are warnings.
    #[must_use]
    pub fn or_fallback(self, fallback: Option<T>) -> Usable<T> {
        if let Some(value) = self.admitted {
            return Usable::Value {
                value,
                warnings: self.refusals,
                notice: self.notice,
            };
        }
        if let Some(value) = fallback {
            return Usable::Value {
                value,
                warnings: self.refusals,
                notice: None,
            };
        }
        let mut refusals = self.refusals.into_iter();
        refusals.next().map_or(Usable::Absent, |fatal| {
            Usable::Refused(Rejection {
                fatal,
                warnings: refusals.collect(),
            })
        })
    }
}

/// A resolution that could not finish reading config, carrying every refusal
/// gathered so far so a team-level value is still reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aborted {
    pub error: ConfigError,
    pub warnings: Vec<Refusal>,
}

/// One whole-config consent finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditFinding {
    Key(Refusal),
    PersonalFile(Distrust),
    PersonalFileUnchecked,
    PersonalFileIgnored(Refusal),
}

/// Everything the provenance checks read.
pub struct ProvenanceContext<'a> {
    pub config: &'a dyn ConfigAccess,
    pub tracking: &'a dyn ConfigFileTracking,
    pub environment: &'a dyn Environment,
    pub personal_config: PathBuf,
}

/// What one source of a value yielded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rung<T> {
    Absent,
    Candidate(T),
    Refused(Refusal),
}

impl<T> Rung<T> {
    fn and_then<U>(
        self,
        check: impl FnOnce(T) -> Result<U, Refusal>,
    ) -> Rung<U> {
        match self {
            Self::Absent => Rung::Absent,
            Self::Refused(refusal) => Rung::Refused(refusal),
            Self::Candidate(value) => match check(value) {
                Ok(checked) => Rung::Candidate(checked),
                Err(refusal) => Rung::Refused(refusal),
            },
        }
    }
}

/// Resolves a plain consent key.
///
/// # Errors
///
/// [`Aborted`] when a config level cannot be read.
pub fn resolve(
    context: &ProvenanceContext<'_>,
    key: &ConsentKey,
) -> Result<Consented<String>, Aborted> {
    resolve_checked(context, key.descriptor(), |value| Ok(value.to_owned()))
}

/// Resolves a path-valued consent key to its canonical path.
///
/// The first candidate, in precedence order, whose value passes
/// [`vet_executable_path`] is admitted, and a refused candidate falls through
/// to the next.
///
/// # Errors
///
/// [`Aborted`] when a config level cannot be read.
pub fn resolve_executable_path(
    context: &ProvenanceContext<'_>,
    key: &ExecutablePathKey,
    roots: &RepositoryRoots,
    paths: &dyn ExecutablePaths,
) -> Result<Consented<PathBuf>, Aborted> {
    resolve_checked(context, key.descriptor(), |value| {
        vet_executable_path(key, value, roots, paths)
    })
}

fn resolve_checked<T>(
    context: &ProvenanceContext<'_>,
    descriptor: &'static ExtraKey,
    check: impl Fn(&str) -> Result<T, Refusal>,
) -> Result<Consented<T>, Aborted> {
    let team_refusals = team_level_refusals(context.config, descriptor)
        .map_err(|error| Aborted {
            error,
            warnings: Vec::new(),
        })?;
    let mut refusals = Vec::new();

    if let Some((variable, value)) =
        environment_candidate(context.environment, descriptor)
    {
        match check(&value) {
            Ok(admitted) => {
                let notice = Notice::new(descriptor, variable, &value);
                return Ok(Consented::from_candidates(
                    Some(admitted),
                    refusals,
                    team_refusals,
                )
                .noticed(Some(notice)));
            }
            Err(refusal) => refusals.push(refusal),
        }
    }

    let personal = match personal_candidate(context, descriptor) {
        Ok(rung) => rung,
        Err(error) => {
            return Err(Aborted {
                error,
                warnings: Consented::<()>::from_candidates(
                    None,
                    refusals,
                    team_refusals,
                )
                .refusals,
            })
        }
    };
    let admitted = match personal.and_then(|value| check(&value)) {
        Rung::Absent => None,
        Rung::Candidate(admitted) => Some(admitted),
        Rung::Refused(refusal) => {
            refusals.push(refusal);
            None
        }
    };
    Ok(Consented::from_candidates(
        admitted,
        refusals,
        team_refusals,
    ))
}

/// Every consent problem in the whole config: each team-level consent key,
/// and a `config.local.md` that is distrusted or ignored whatever it sets.
///
/// # Errors
///
/// A [`ConfigError`] when the team level cannot be read.
pub fn audit(
    context: &ProvenanceContext<'_>,
) -> Result<Vec<AuditFinding>, ConfigError> {
    let mut findings = Vec::new();
    for key in catalogue::consent_keys() {
        findings.extend(
            team_level_refusals(context.config, key)?
                .into_iter()
                .map(AuditFinding::Key),
        );
    }
    let path = match context.config.personal_file() {
        PersonalFile::Absent => return Ok(findings),
        PersonalFile::Readable => &context.personal_config,
        PersonalFile::Ignored { path, mode } => {
            findings.push(AuditFinding::PersonalFileIgnored(
                Refusal::InsecurePersonalFile {
                    path: path.clone(),
                    mode: *mode,
                },
            ));
            path
        }
    };
    match context.tracking.check(path) {
        TrackingCheck::Unchecked => {
            findings.push(AuditFinding::PersonalFileUnchecked);
        }
        TrackingCheck::Known(tracking) => {
            findings
                .extend(tracking.distrust().map(AuditFinding::PersonalFile));
        }
    }
    Ok(findings)
}

/// A command the policy has resolved for a command key, which only
/// [`Self::run`] can execute.
pub struct ConsentedCommand {
    key: CommandKey,
    command: String,
    variable: Option<&'static str>,
}

impl ConsentedCommand {
    /// Runs the command under its key's policy.
    ///
    /// # Errors
    ///
    /// The command refusal for a command that could not start, failed,
    /// timed out, or printed too much.
    pub fn run(
        &self,
        execution: &CommandExecution<'_>,
    ) -> Result<String, Refusal> {
        execution.run(self.key, &self.command)
    }

    fn notice(&self) -> Option<Notice> {
        self.variable.map(|variable| {
            Notice::new(self.key.descriptor(), variable, &self.command)
        })
    }
}

/// The candidates a command key offers: the environment's and the personal
/// level's, read only when asked for, beside the team-level refusal.
pub struct CommandCandidates<'a> {
    context: &'a ProvenanceContext<'a>,
    key: CommandKey,
    team_level: Vec<Refusal>,
}

impl CommandCandidates<'_> {
    /// The environment override, which skips the provenance checks.
    #[must_use]
    pub fn environment(&self) -> Rung<ConsentedCommand> {
        environment_candidate(self.context.environment, self.key.descriptor())
            .map_or(Rung::Absent, |(variable, command)| {
                Rung::Candidate(ConsentedCommand {
                    key: self.key,
                    command,
                    variable: Some(variable),
                })
            })
    }

    /// The personal level's command, refused when the file is distrusted or
    /// ignored.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when the personal level cannot be read.
    pub fn personal(&self) -> Result<Rung<ConsentedCommand>, ConfigError> {
        Ok(
            match personal_candidate(self.context, self.key.descriptor())? {
                Rung::Absent => Rung::Absent,
                Rung::Refused(refusal) => Rung::Refused(refusal),
                Rung::Candidate(command) => Rung::Candidate(ConsentedCommand {
                    key: self.key,
                    command,
                    variable: None,
                }),
            },
        )
    }

    #[must_use]
    pub fn team_level_refusals(&self) -> &[Refusal] {
        &self.team_level
    }
}

/// Resolves a command key's candidates, reading the team level at once so
/// its refusal is reported whichever source later wins.
///
/// # Errors
///
/// [`Aborted`] when the team level cannot be read.
pub fn resolve_command<'a>(
    context: &'a ProvenanceContext<'a>,
    key: CommandKey,
) -> Result<CommandCandidates<'a>, Aborted> {
    let team_level = team_level_refusals(context.config, key.descriptor())
        .map_err(|error| Aborted {
            error,
            warnings: Vec::new(),
        })?;
    Ok(CommandCandidates {
        context,
        key,
        team_level,
    })
}

/// The value a [`Ladder`] admitted, and which of its rungs yielded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted<S> {
    pub source: S,
    pub value: String,
}

/// Climbs rungs in precedence order: the first to yield a usable value is
/// admitted, and every refusal met on the way is kept.
///
/// Once a value is admitted, later rungs are never consulted, so no further
/// level is read and no further command runs. A value carrying a control
/// character is refused as malformed and the climb continues.
pub struct Ladder<S> {
    admitted: Option<Admitted<S>>,
    refusals: Vec<Refusal>,
    team_level: Vec<Refusal>,
    notice: Option<Notice>,
}

impl<S> Ladder<S> {
    #[must_use]
    pub const fn new(team_level: Vec<Refusal>) -> Self {
        Self {
            admitted: None,
            refusals: Vec::new(),
            team_level,
            notice: None,
        }
    }

    /// Offers a plain value for `key` from `source`.
    ///
    /// # Errors
    ///
    /// [`Aborted`] when the rung cannot read its level.
    pub fn offer(
        &mut self,
        source: S,
        key: &'static ExtraKey,
        rung: impl FnOnce() -> Result<Rung<String>, ConfigError>,
    ) -> Result<(), Aborted> {
        if self.admitted.is_some() {
            return Ok(());
        }
        match rung().map_err(|error| self.aborted(error))? {
            Rung::Absent => {}
            Rung::Refused(refusal) => self.refusals.push(refusal),
            Rung::Candidate(value) => self.admit(source, key, value, None),
        }
        Ok(())
    }

    /// Runs a consented command from `source`, a failure falling through.
    ///
    /// # Errors
    ///
    /// [`Aborted`] when the rung cannot read its level.
    pub fn attempt(
        &mut self,
        source: S,
        rung: impl FnOnce() -> Result<Rung<ConsentedCommand>, ConfigError>,
        execution: &CommandExecution<'_>,
    ) -> Result<(), Aborted> {
        if self.admitted.is_some() {
            return Ok(());
        }
        match rung().map_err(|error| self.aborted(error))? {
            Rung::Absent => {}
            Rung::Refused(refusal) => self.refusals.push(refusal),
            Rung::Candidate(command) => match command.run(execution) {
                Ok(value) => self.admit(
                    source,
                    command.key.descriptor(),
                    value,
                    command.notice(),
                ),
                Err(refusal) => self.refusals.push(refusal),
            },
        }
        Ok(())
    }

    #[must_use]
    pub fn finish(self) -> Consented<Admitted<S>> {
        Consented::from_candidates(
            self.admitted,
            self.refusals,
            self.team_level,
        )
        .noticed(self.notice)
    }

    fn admit(
        &mut self,
        source: S,
        key: &'static ExtraKey,
        value: String,
        notice: Option<Notice>,
    ) {
        if value.chars().any(char::is_control) {
            self.refusals.push(Refusal::MalformedToken { key });
            return;
        }
        self.admitted = Some(Admitted { source, value });
        self.notice = notice;
    }

    fn aborted(&self, error: ConfigError) -> Aborted {
        Aborted {
            error,
            warnings: Consented::<()>::from_candidates(
                None,
                self.refusals.clone(),
                self.team_level.clone(),
            )
            .refusals,
        }
    }
}

fn team_level_refusals(
    config: &dyn ConfigAccess,
    key: &'static ExtraKey,
) -> Result<Vec<Refusal>, ConfigError> {
    Ok(raw_value(config, key, Level::Team)?
        .map(|_| Refusal::TeamLevel { key })
        .into_iter()
        .collect())
}

pub(crate) fn environment_candidate(
    environment: &dyn Environment,
    key: &ExtraKey,
) -> Option<(&'static str, String)> {
    key.overrides.iter().find_map(|variable| {
        environment
            .read(variable)
            .and_then(|value| non_blank(&value))
            .map(|value| (*variable, value))
    })
}

fn personal_candidate(
    context: &ProvenanceContext<'_>,
    key: &'static ExtraKey,
) -> Result<Rung<String>, ConfigError> {
    personal_value(context, key, |key, path, distrust| {
        Refusal::UntrustedPersonalFile {
            key,
            path,
            distrust,
        }
    })
}

/// A plaintext credential's personal value, refused like a consent key's when
/// the file is distrusted, though a plaintext key is not a consent key.
pub(crate) fn personal_plaintext(
    context: &ProvenanceContext<'_>,
    key: &'static ExtraKey,
) -> Result<Rung<String>, ConfigError> {
    personal_value(context, key, |key, path, distrust| {
        Refusal::PlaintextFromUntrustedFile {
            key,
            path,
            distrust,
        }
    })
}

fn personal_value(
    context: &ProvenanceContext<'_>,
    key: &'static ExtraKey,
    distrusted: fn(&'static ExtraKey, PathBuf, Distrust) -> Refusal,
) -> Result<Rung<String>, ConfigError> {
    match context.config.personal_file() {
        PersonalFile::Absent => Ok(Rung::Absent),
        PersonalFile::Ignored { path, mode } => {
            Ok(Rung::Refused(Refusal::InsecurePersonalFile {
                path: path.clone(),
                mode: *mode,
            }))
        }
        PersonalFile::Readable => {
            let Some(value) = raw_value(context.config, key, Level::Personal)?
            else {
                return Ok(Rung::Absent);
            };
            let distrust = context
                .tracking
                .tracking(&context.personal_config)
                .distrust();
            Ok(distrust.map_or(Rung::Candidate(value), |distrust| {
                Rung::Refused(distrusted(
                    key,
                    context.personal_config.clone(),
                    distrust,
                ))
            }))
        }
    }
}

pub(crate) fn raw_value(
    config: &dyn ConfigAccess,
    key: &ExtraKey,
    level: Level,
) -> Result<Option<String>, ConfigError> {
    let parsed = Key::parse(key.name)?;
    Ok(match config.get(&parsed, Some(level))? {
        Resolved::Found(value) => non_blank(&render_value(&value)),
        Resolved::Absent => None,
    })
}

fn non_blank(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{Consented, Distrust, Refusal};
    use crate::catalogue;

    #[test]
    fn candidate_refusals_come_before_team_level_ones_whatever_the_order_given()
    {
        let key = catalogue::declared("jira.allowed_sites")
            .unwrap_or_else(|| unreachable!("the allowlist is declared"));
        let team = Refusal::TeamLevel { key };
        let tracked = Refusal::UntrustedPersonalFile {
            key,
            path: "/p".into(),
            distrust: Distrust::Tracked,
        };

        let consented: Consented<String> = Consented::from_candidates(
            None,
            vec![team.clone(), tracked.clone()],
            Vec::new(),
        );

        assert_eq!(consented.refusals, [tracked, team]);
    }
}

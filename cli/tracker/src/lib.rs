//! The port a remote issue tracker satisfies, and the vocabulary it speaks.
//!
//! The provider clients that implement it and the sync engine that calls it
//! both live elsewhere; this crate is the seam between them and holds no
//! logic. It deliberately has no `-adapters` sibling.

use std::fmt::Display;
use std::fmt::Formatter;

/// The identifier a remote tracker gave an issue.
///
/// The same value the local work item carries in its `external_id`
/// frontmatter field, taken as opaque: the port does not parse, validate or
/// interpret the string.
///
/// Opaque to the port is not opaque to the client. The value is written
/// unquoted into a work item's YAML frontmatter, so an implementation must
/// reject an identifier it cannot safely persist — control characters, a
/// newline, a leading `---` or `#` — rather than returning it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalId(String);

impl ExternalId {
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ExternalId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A tracker's own last-modified stamp, held verbatim.
///
/// A cache key, not a clock: an unequal stamp means the body must be
/// re-hashed, never that the remote is newer. Hence no `PartialOrd` or `Ord`,
/// and no conversion surface beyond construction and read-back.
///
/// The bytes must survive unchanged — providers emit mutually incompatible
/// formats (see `tests/fixtures/remote-updated-at.txt` for the committed set),
/// and a date-library round-trip would rewrite a numeric offset, reclassifying
/// every item whose baseline the bash sync path already wrote.
///
/// The empty string is a legal value with two sources: a tracker that reports
/// no timestamp for an issue, and a post-push read that failed. `new`
/// therefore validates nothing. Both mean *unknown*.
///
/// Beware the consequence: `==` reports two empty stamps as equal, and that
/// must not be read as "unchanged". Check for emptiness before comparing, as
/// the sync classifier does — comparing two unknowns and concluding a match
/// classifies an item whose baseline was never written as already synced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTimestamp(String);

impl RemoteTimestamp {
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a tracker reports about one issue, in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteIssue {
    /// The tracker's own last-modified stamp, stored as `remote_updated_at`
    /// in the sync baseline. The two names refer to one value.
    pub updated: RemoteTimestamp,
    /// The already-projected domain body: the issue's title line, then its
    /// description, with no blank line between them and a trailing newline.
    /// A structured description is canonicalised first — key-sorted and
    /// compact — so equal content hashes equally; a Markdown one is carried
    /// verbatim.
    ///
    /// An absent description is where the two providers diverge and where a
    /// client is most likely to guess wrong: a structured one projects as the
    /// literal token `null`, a Markdown one as an empty line. Neither is
    /// inferable from a JSON deserialiser's natural output, and either wrong
    /// choice reclassifies every such item.
    ///
    /// The value is the *un-normalised* projection. The caller normalises
    /// before hashing.
    ///
    /// This is **not** the body a caller supplies when pushing: it carries the
    /// title line as well, so a push followed by a read is not the identity.
    ///
    /// Projection sits behind the port, so reproducing the recipe exactly is
    /// the implementing client's obligation. A body differing by so much as
    /// whitespace reclassifies every synced item as remotely modified, and an
    /// interior blank line survives normalisation. The bash recipe
    /// (`work-item-project-remote.sh`) is the current reference
    /// implementation; the contract above outlives it.
    pub body: String,
}

/// A failure reported by a remote tracker.
///
/// Two classes, closed deliberately: `#[non_exhaustive]` is absent so that
/// adding a third is a compile-breaking change for every consumer, which is
/// the property both consumers want.
///
/// The classes divide on one question: **could a remote change have
/// happened?** That makes classification operation-scoped, not a property of
/// the wire condition — the same provider status falls either way depending on
/// what was attempted, so a client must classify per call rather than from one
/// status table. A read cannot mutate, so a read never produces `Terminal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerError {
    /// No remote change occurred, provably.
    ///
    /// For a mutating call the test is the *mutation*, not the transmission:
    /// a request that never left the machine qualifies, and so does one the
    /// tracker received and rejected before applying anything.
    ///
    /// The test is provability, not a list of statuses. A rejection qualifies
    /// only where the provider's protocol makes it provable, and that varies
    /// by operation as well as by provider: the same wire condition can be
    /// provable on `create` and unprovable on `update` against one tracker.
    /// A single status-to-class table is therefore wrong — classify per
    /// operation, and when in doubt use `Terminal`.
    ///
    /// For a read it is the only class, because there was nothing to mutate;
    /// the caller degrades rather than repeating blindly.
    Retryable {
        /// What failed, for a human reading a sync report. State the
        /// provider, the operation, the external id where one is known, and
        /// the underlying status or exit code.
        detail: String,
    },
    /// A remote change may have happened, and which is unknowable.
    ///
    /// The conservative default **for mutating calls**: a failure belongs in
    /// `Retryable` only when the absence of a remote change is *provable*, so
    /// a lost or unparseable response, a 5xx, or a connection dropped after
    /// the request went out all belong here — the tracker may have applied it.
    /// Reads never produce this class.
    Terminal {
        /// What failed, in the same shape `Retryable` asks for, and
        /// additionally whether a remote mutation may have applied — that is
        /// what the reader has to act on.
        detail: String,
    },
}

impl Display for TrackerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retryable { detail } => write!(
                formatter,
                "tracker call failed with no remote change: {detail}"
            ),
            Self::Terminal { detail } => write!(
                formatter,
                "tracker call failed and a remote change may have applied, so \
                 the remote state is unknown: {detail}"
            ),
        }
    }
}

impl std::error::Error for TrackerError {}

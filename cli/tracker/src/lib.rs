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

/// What a bulk retrieval could establish about each requested issue.
///
/// The partition is total over the requested ids: every distinct id appears in
/// exactly one of the three vectors. Duplicates in `ids` are ignored — the
/// request is a set, as both bash adapters treat it — an empty request yields
/// an empty outcome and makes no remote call, and the three vectors are
/// unordered, so a caller indexes rather than zips.
///
/// `absent` carries the weight. An id belongs there only when the retrieval
/// was provably complete — a truncated page, an exhausted rate limit or a
/// partial failure puts its unseen ids in `indeterminate` instead. An
/// implementation that cannot distinguish the two must report every unseen id
/// as indeterminate: inferring absence from a fetch that may have been cut
/// short is what makes a sync delete an issue that still exists.
///
/// Nothing here enforces totality — the type cannot, and this crate ships no
/// logic. It is an obligation on every implementation, held by the shared
/// contract test that lives with the sync engine. Until that exists,
/// `tracker/tests/port.rs::partitions_totally` is the check to copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchOutcome {
    /// The issues the retrieval accounted for, each paired with the id it was
    /// requested under — bulk payloads carry no other way to associate a
    /// record with a local work item.
    ///
    /// A stamp, not an issue: bulk retrieval establishes *whether* an issue
    /// changed, and `show` fetches the body for the minority that did. No
    /// provider's bulk query returns a projected body, so a `RemoteIssue` here
    /// could only ever be filled with a fabricated one.
    ///
    /// An issue the tracker returns without a timestamp still belongs here,
    /// paired with an empty `RemoteTimestamp`. Never drop it: an id missing
    /// from a complete retrieval reads as absence, so filtering out the
    /// null-stamped entries reports a live issue as deleted.
    pub found: Vec<(ExternalId, RemoteTimestamp)>,
    /// Provably gone from the tracker. Only ever drawn from a complete
    /// retrieval.
    pub absent: Vec<ExternalId>,
    /// Not accounted for, and the retrieval could not prove why.
    pub indeterminate: Vec<ExternalId>,
}

/// The operations a remote issue tracker exposes to the sync engine.
///
/// Synchronous and `&self`-taking so the trait stays dyn-compatible: the sync
/// engine selects its client at composition time from the `work.integration`
/// config key and holds it as `Box<dyn RemoteTracker>`. A native async fn in a
/// trait is not object-safe, and the crate's import rule forbids the
/// `async-trait` dependency that would restore it.
///
/// Every call blocks until it resolves. Timeouts, retries and backoff are
/// wholly the implementing client's responsibility — they live inside the bash
/// bridges today, i.e. already behind this seam — and a client must bound its
/// own calls, because a caller has no way to.
pub trait RemoteTracker {
    /// Creates a new remote issue and returns the identifier assigned to it.
    ///
    /// An identifier that cannot be safely written back to frontmatter is a
    /// [`TrackerError::Terminal`] failure, not an `Ok`: the issue exists
    /// remotely, so a repeat would duplicate it, and the caller must be told
    /// rather than handed a value that would corrupt the file.
    ///
    /// `title` and `body` are the local work item's, not a projected body.
    /// `kind` is the work item's `kind` value, taken opaquely: mapping it onto
    /// a Jira issue type or its Linear equivalent is the implementing client's
    /// business, and the empty string means "use the tracker's configured
    /// default", which is what the bash bridge does with an omitted `--kind`.
    ///
    /// # Errors
    ///
    /// [`TrackerError::Retryable`] only when it is provable that no issue was
    /// created — which includes a tracker-side rejection, where the provider's
    /// protocol makes the rejection provable. A remote create is not
    /// idempotent, so once the request may have been *applied* the failure is
    /// [`TrackerError::Terminal`]: a repeat would duplicate the issue.
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError>;

    /// Replaces the remote issue's whole content.
    ///
    /// Returns nothing: the caller that needs the resulting stamp for its sync
    /// baseline reads it back with `show`.
    ///
    /// # Errors
    ///
    /// [`TrackerError::Retryable`] only when it is provable that nothing was
    /// modified; otherwise [`TrackerError::Terminal`]. The operation is
    /// idempotent, so the hazard is not duplication but not knowing whether it
    /// landed.
    ///
    /// The two operations' provable sets are **not nested in either
    /// direction** for at least one provider — each is narrower than the other
    /// on some conditions — so derive each from that operation's own mapping
    /// rather than carrying a classification across.
    fn update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), TrackerError>;

    /// Reads one remote issue in full, including its projected body.
    ///
    /// Absence is not discoverable here — a `RemoteIssue` or an error are the
    /// only outcomes. Establish that an id is gone with `fetch_all`, whose
    /// partition distinguishes provable absence from an unproven miss, and do
    /// not build a retry loop around a `show` that may be reading a deleted
    /// issue.
    ///
    /// # Errors
    ///
    /// Always [`TrackerError::Retryable`]. A read mutates nothing, so the
    /// terminal class — which means "a mutation may have applied" — cannot
    /// arise; the bash read bridge collapses every read failure to the
    /// retryable code for the same reason.
    ///
    /// Read the class as "nothing changed remotely", not as "call again". A
    /// deleted issue fails here indefinitely, so the caller degrades to
    /// presence-only rather than looping.
    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError>;

    /// Reads the requested issues in bulk, partitioned by what the retrieval
    /// could establish.
    ///
    /// Returns stamps, not bodies. This is the cheap first tier of a two-tier
    /// read: compare each stamp against the sync baseline, then call `show`
    /// for the minority that moved.
    ///
    /// # Errors
    ///
    /// Always [`TrackerError::Retryable`], and only on a **pre-flight** failure
    /// — one that stops any request being constructed, such as unresolvable
    /// credentials or a requested id the client cannot safely embed in its
    /// query.
    ///
    /// Once a request has been attempted, every outcome is an `Ok`. A partial
    /// retrieval puts its unproven ids in `indeterminate`; so does a total
    /// transport failure, which is an `Ok` with every id indeterminate rather
    /// than an `Err`. Degrading per id beats failing a whole sync run, and the
    /// partition can already say it.
    ///
    /// This deliberately differs from the current Linear bridge, which returns
    /// a retryable exit for any bulk-search failure — a client porting that
    /// adapter must move transport failures into the partition rather than
    /// carrying the `Err` across.
    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError>;
}

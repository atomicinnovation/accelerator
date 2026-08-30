//! The port a remote issue tracker satisfies, and the vocabulary it speaks.
//!
//! The provider clients that implement it and the sync engine that calls it
//! both live elsewhere; this crate is the seam between them and holds no
//! logic.

use std::fmt::Display;
use std::fmt::Formatter;

/// The identifier a remote tracker gave an issue.
///
/// The same value the local work item carries in its `external_id`
/// frontmatter field. The port neither parses nor validates it.
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

/// A tracker's own last-modified stamp, or the reason there is none.
///
/// A cache key, not a clock: an unequal stamp means the body must be
/// re-hashed, never that the remote is newer. Hence no `PartialOrd` or `Ord`,
/// and no conversion surface beyond construction and read-back.
///
/// The three variants are exhaustive over what a sync run can know, and stay
/// distinct rather than collapsing into one absent value: a sync report tells a
/// user "the tracker has no stamp for this issue" and "the push landed but its
/// stamp could not be read back" differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteTimestamp {
    /// The stamp the tracker reported, held verbatim.
    ///
    /// The bytes must survive unchanged — providers emit mutually incompatible
    /// formats, and a date-library round-trip would rewrite a numeric offset,
    /// reclassifying every item whose baseline holds the original spelling.
    ///
    /// Nothing validates the bytes, so an empty string is constructible here
    /// and means nothing. A tracker that answers with a blank or null stamp is
    /// reporting no stamp: map it to `NotReported`.
    Reported(String),
    /// The tracker holds no stamp for the issue.
    ///
    /// A property of the remote record, established by a retrieval that
    /// succeeded — so a client returns this, and it is the only unknown a
    /// client can return.
    NotReported,
    /// A push landed, and the read that would have learned the new stamp
    /// failed.
    ///
    /// No port operation returns this: `show` and `fetch_all` either answer or
    /// fail. The sync engine writes it into a baseline when its post-push
    /// read-back does not come back, recording that the remote moved by an
    /// unknown amount. The next run re-reads rather than trusting the baseline.
    NotRead,
}

impl RemoteTimestamp {
    /// The bytes the tracker reported, if it reported any.
    #[must_use]
    pub fn reported(&self) -> Option<&str> {
        match self {
            Self::Reported(stamp) => Some(stamp),
            Self::NotReported | Self::NotRead => None,
        }
    }

    /// Whether this stamp, read against `baseline`, proves the issue has not
    /// changed — and so that its body need not be re-hashed.
    ///
    /// Only two reported stamps holding identical bytes can prove it. An
    /// unknown on either side proves nothing, whichever kind it is — so use
    /// this rather than `==`, which reports two identical unknowns as equal and
    /// would classify an item whose baseline was never written as already
    /// synced.
    #[must_use]
    pub fn proves_unchanged_since(&self, baseline: &Self) -> bool {
        matches!(
            (self, baseline),
            (Self::Reported(stamp), Self::Reported(known)) if stamp == known
        )
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
    /// An absent description projects as the literal token `null` when
    /// structured and as an empty line when Markdown. Neither is inferable
    /// from a JSON deserialiser's natural output, and either wrong choice
    /// reclassifies every such item.
    ///
    /// The value is the *un-normalised* projection. The caller normalises
    /// before hashing, and an interior blank line survives that.
    ///
    /// This is **not** the body a caller supplies when pushing: it carries the
    /// title line as well, so a push followed by a read is not the identity.
    ///
    /// Projection sits behind the port, so reproducing the recipe exactly is
    /// the implementing client's obligation. A body differing by so much as
    /// whitespace reclassifies every synced item as remotely modified.
    pub body: String,
}

/// A failure reported by a remote tracker.
///
/// Two classes, and closed: `#[non_exhaustive]` is absent so that adding a
/// third is a compile-breaking change for every consumer.
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
    /// The test is provability, not a list of statuses, and it varies by
    /// operation as well as by provider: the same wire condition can be
    /// provable on `create` and unprovable on `update` against one tracker. A
    /// single status-to-class table is therefore wrong — classify per
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

impl TrackerError {
    /// The inner `detail`, unwrapped from either class.
    ///
    /// The two variants carry the same field, so a caller wanting the message
    /// alone — a report line, not the `Display` wrapper's class prose — takes it
    /// through one exhaustive match here rather than destructuring both arms.
    #[must_use]
    pub fn into_detail(self) -> String {
        match self {
            Self::Retryable { detail } | Self::Terminal { detail } => detail,
        }
    }
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
/// exactly one of the three vectors. The request is a set, so duplicates in
/// `ids` are ignored; an empty request yields an empty outcome and makes no
/// remote call; and the three vectors are unordered, so a caller indexes rather
/// than zips.
///
/// `absent` carries the weight. An id belongs there only when the retrieval
/// was provably complete — a truncated page, an exhausted rate limit or a
/// partial failure puts its unseen ids in `indeterminate` instead. An
/// implementation that cannot distinguish the two must report every unseen id
/// as indeterminate: inferring absence from a fetch that may have been cut
/// short is what makes a sync delete an issue that still exists.
///
/// Nothing here enforces totality — the type cannot, and this crate ships no
/// logic. It is an obligation on every implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchOutcome {
    /// The issues the retrieval accounted for, each paired with the id it was
    /// requested under — bulk payloads carry no other way to associate a
    /// record with a local work item.
    ///
    /// A stamp, not an issue: no provider's bulk query returns a projected
    /// body, so a `RemoteIssue` here could only ever hold a fabricated one.
    /// `show` fetches the body for the minority whose stamp moved.
    ///
    /// An issue the tracker returns without a timestamp still belongs here,
    /// paired with [`RemoteTimestamp::NotReported`]. Never drop it: an id
    /// missing from a complete retrieval reads as absence, so filtering out the
    /// null-stamped entries reports a live issue as deleted.
    pub found: Vec<(ExternalId, RemoteTimestamp)>,
    /// Provably gone from the tracker. Only ever drawn from a complete
    /// retrieval.
    pub absent: Vec<ExternalId>,
    /// Not accounted for, and the retrieval could not prove why.
    pub indeterminate: Vec<ExternalId>,
}

/// Where an unkeyed discovery query looks.
///
/// A discovery has no requested id set — it asks the tracker which issues exist
/// within a scope — so it needs its own scoping vocabulary rather than the id
/// slice `fetch_all` takes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchScope {
    /// The single project or team to scope discovery to.
    ///
    /// When both this and `all_projects` are set, `project` wins: a named
    /// project always narrows. The precedence is stated here rather than left to
    /// the composer so a caller reading the type knows which field decides.
    pub project: Option<String>,
    /// Discover across every project the credentials can see.
    pub all_projects: bool,
    /// Extra provider-specific field filters, each a `(field, value)` pair.
    pub filters: Vec<(String, String)>,
}

/// What an unkeyed discovery established.
///
/// Distinct from [`FetchOutcome`]: a discovery has no requested id set to
/// partition over, so it carries a truncation flag rather than an `absent`
/// vector. `complete == false` means the query was cut short — a page cap, a
/// deadline, a rate limit — and the caller must treat the result as a lower
/// bound, never as the whole of what the tracker holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// The issues the query saw, each with the stamp the tracker reported.
    pub found: Vec<(ExternalId, RemoteTimestamp)>,
    /// Whether the query saw everything in scope. `false` on any truncation.
    pub complete: bool,
}

/// A discovery scope that names no valid search target for this tracker — a
/// missing or unresolvable key.
///
/// A configuration fault, distinct from a transient read failure. It carries
/// its own type rather than overloading [`TrackerError::Retryable`] so it can
/// never be routed through a caller's tracker-error path and mis-exit as a
/// transient failure: `resolve_scope` mutates nothing, so a scope fault is
/// always something the operator must fix before any request goes out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeError {
    /// What is wrong with the scope, for a human. Names the missing or
    /// unresolvable key and how to supply it.
    pub detail: String,
}

impl Display for ScopeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "the discovery scope names no valid target: {}",
            self.detail
        )
    }
}

impl std::error::Error for ScopeError {}

/// One create-preview field, resolved to one of three states.
///
/// The three stay distinct because the create skill renders each field's
/// *source*, and a two-state `Option` would collapse `Unset` and `Unresolvable`
/// into one `None` and lose the distinction the create-preview must flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldResolution {
    /// A configured value that the tracker confirmed exists.
    Resolved(String),
    /// Nothing configured — a benign default applies.
    Unset,
    /// A configured value the tracker does not hold. The state the
    /// create-preview exists to surface, carrying the offending value.
    Unresolvable(String),
}

/// The fields a create would resolve, previewed without creating anything.
///
/// Both the project and the issue type are three-state [`FieldResolution`]s so
/// the caller can distinguish a benign default from a configured value absent
/// remotely, and recover each field's source for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePreview {
    pub project: FieldResolution,
    pub issue_type: FieldResolution,
}

/// Whether a composed update payload would be accepted.
///
/// Distinguishes a valid payload from a rejected one carrying *why*, rather
/// than folding rejection into an error class — the reasons are shown to the
/// operator before any mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationOutcome {
    Valid,
    Rejected { reasons: Vec<String> },
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
/// wholly the implementing client's responsibility, and a client must bound its
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
    /// default".
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
    /// arise.
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
    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError>;

    /// Discovers the issues within `scope`, without a requested id set.
    ///
    /// An unkeyed remote read: it asks the tracker which issues exist, rather
    /// than reporting on ids the caller already holds. A cap-hit or a truncated
    /// page is not a failure — it returns [`Discovery`] with `complete == false`
    /// so the caller treats the result as a lower bound.
    ///
    /// # Errors
    ///
    /// Always [`TrackerError::Retryable`] and never [`TrackerError::Terminal`]:
    /// a read mutates nothing. A retrieval cut short reports
    /// `complete == false` rather than erroring.
    fn search(&self, scope: &SearchScope) -> Result<Discovery, TrackerError>;

    /// Resolves and validates a discovery `scope` for this tracker, without any
    /// network call, returning the scope ready for [`RemoteTracker::search`].
    ///
    /// The one deterministic, local step in the discovery path: it substitutes
    /// provider identity a search needs but a config only names — a Linear team
    /// *key* for its UUID — and validates that the scope names a target at all.
    /// A caller runs it before any mutation, so a scope fault refuses the run
    /// before a single request goes out.
    ///
    /// # Errors
    ///
    /// [`ScopeError`] when the scope names no valid target — a missing or
    /// unresolvable key. It resolves locally and mutates nothing, so it never
    /// reports a transient or terminal failure; that separation is why the error
    /// is a [`ScopeError`] and not a [`TrackerError`].
    fn resolve_scope(
        &self,
        scope: &SearchScope,
    ) -> Result<SearchScope, ScopeError>;

    /// Previews the fields a `create` of the given `kind` would resolve,
    /// without creating anything.
    ///
    /// Performs a remote existence check — a provider that resolves a project
    /// key against its live catalogue contacts the tracker — so it can fail on a
    /// transport failure. An unresolvable value is **not** a failure: it is a
    /// successful [`CreatePreview`] carrying [`FieldResolution::Unresolvable`],
    /// which is the state the caller must surface.
    ///
    /// `kind` is the work item's `kind`, taken opaquely as for
    /// [`RemoteTracker::create`]; the empty string means the tracker's
    /// configured default.
    ///
    /// # Errors
    ///
    /// Always [`TrackerError::Retryable`] and never [`TrackerError::Terminal`]:
    /// the check applies no mutation, so a read that provably changed nothing
    /// can only be retryable.
    fn preview_create(&self, kind: &str)
        -> Result<CreatePreview, TrackerError>;

    /// Validates the payload a `update` of this content would send, without
    /// sending it.
    ///
    /// A **local payload-composition check** for both providers: it composes the
    /// update payload the way [`RemoteTracker::update`] would and reports whether
    /// it is locally well-formed, making no remote call. It detects
    /// locally-checkable omissions — a required field the composed payload leaves
    /// empty — and reports them as [`ValidationOutcome::Rejected`]. It does not
    /// reproduce a live-tracker field check; a clean outcome does not guarantee
    /// the tracker will accept the push.
    ///
    /// Returns [`ValidationOutcome`] directly rather than a `Result`: the check
    /// makes no remote call and so cannot fail with a [`TrackerError`], and the
    /// type makes a mutation unrepresentable.
    fn validate_update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> ValidationOutcome;
}

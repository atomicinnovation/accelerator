//! The change-detection engine.

use tracker::ExternalId;
use tracker::RemoteTimestamp;

use crate::sync::baseline::BaselineFingerprints;
use crate::sync::baseline::LegacyHashes;
use crate::sync::baseline::LocalFingerprints;
use crate::sync::baseline::RemoteFingerprints;
use crate::sync::baseline::SplitFingerprints;
use crate::sync::pull_residue::PullResidue;
use crate::sync::state::SyncState;

/// A lazy source of the values `classify` may need to hash or stat, under
/// either digest recipe.
///
/// A port rather than pre-computed values so a caller pays for a syscall
/// only when `classify` actually asks: the mtime pre-filter short-circuits
/// without hashing, and an absent baseline hash skips the `stat` and the
/// hash entirely.
///
/// Every method is fallible. An infallible port has nowhere to put a read
/// failure but a panic or a fabricated hash, and a fabricated empty hash
/// would silently classify the item as locally changed with no log line to
/// diagnose it from.
pub trait ItemDigests {
    /// `Ok(None)` when no mtime is available, which forces the hash path.
    ///
    /// # Errors
    ///
    /// When the mtime read fails in a way other than "not present".
    fn mtime(&self) -> Result<Option<u64>, kernel::Error>;

    /// # Errors
    ///
    /// When the local content cannot be read or normalised.
    fn legacy_local(&self) -> Result<String, kernel::Error>;

    /// `Ok(None)` means no body was fetched, not that a read failed. A read
    /// that failed is `Err`; the two must not collapse.
    ///
    /// # Errors
    ///
    /// When a body was fetched but could not be normalised.
    fn legacy_remote(&self) -> Result<Option<String>, kernel::Error>;

    /// # Errors
    ///
    /// When the local content cannot be read or fingerprinted.
    fn local(&self) -> Result<LocalFingerprints, kernel::Error>;

    /// `Ok(None)` means no body was fetched.
    ///
    /// # Errors
    ///
    /// When a body was fetched but could not be fingerprinted.
    fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error>;
}

/// Whether the two sides' bodies differ in meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyEquivalence {
    Equivalent,
    Different,
    /// The remote holds content with no Markdown form, so the bodies cannot
    /// be compared.
    RemoteUnrenderable,
    /// The remote was not fetched.
    Unknown,
}

/// Whether the two sides' synced tracker fields match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldsComparison {
    Same,
    Different,
    /// Either side's fields could not be read.
    Unknown,
}

/// What an item's two sides say about each other.
pub trait ItemComparison {
    /// The raw tracker content an earlier pull left in the local file, when
    /// the remote was fetched and the file still holds some.
    fn pull_residue(&self) -> Option<PullResidue>;

    fn body_equivalence(&self) -> BodyEquivalence;

    fn fields_comparison(&self) -> FieldsComparison;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePresence {
    Present,
    Absent,
    Indeterminate,
}

/// An item's baseline entry; an item with none compares against empty
/// legacy hashes.
pub struct BaselineEntry<'a> {
    pub remote_updated_at: &'a RemoteTimestamp,
    pub fingerprints: BaselineFingerprints,
}

pub struct Subject<'a> {
    pub external_id: Option<&'a ExternalId>,
    pub presence: RemotePresence,
    pub remote_updated: &'a RemoteTimestamp,
    /// The item's change-detection watermark: the epoch its local file was
    /// last synced. The mtime pre-filter short-circuits when the file's mtime
    /// does not exceed this. Sourced per item from the baseline (the entry's
    /// own watermark, or the document-level timestamp for an item with no
    /// entry yet).
    pub baseline_timestamp: u64,
}

/// How a synced item's baseline stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncedReconciliation {
    /// The baseline already records the item under the current recipe.
    ByContent,
    /// The item is synced, but its baseline must be rewritten under the
    /// current recipe.
    RecordOnly,
}

/// Whether a send carries the local synced fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDelivery {
    Send,
    /// The local fields have not moved since the baseline, so sending them
    /// could only undo a remote change.
    Withhold,
}

/// How a locally modified item is reconciled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalReconciliation {
    /// The local body is sent.
    ByContent { fields: FieldDelivery },
    /// Nothing the tracker holds moved locally, so only the baseline is
    /// rewritten.
    RecordOnly,
    /// Only the local synced fields are sent.
    SendFields,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    Unsynced,
    Synced(SyncedReconciliation),
    LocallyModified(LocalReconciliation),
    RemotelyModified,
    Conflict,
    RemoteAbsent,
    Indeterminate,
}

impl Classification {
    /// The state a report names this classification by.
    #[must_use]
    pub const fn state(self) -> SyncState {
        match self {
            Self::Unsynced => SyncState::Unsynced,
            Self::Synced(_) => SyncState::Synced,
            Self::LocallyModified(_) => SyncState::LocallyModified,
            Self::RemotelyModified => SyncState::RemotelyModified,
            Self::Conflict => SyncState::Conflict,
            Self::RemoteAbsent => SyncState::RemoteAbsent,
            Self::Indeterminate => SyncState::Indeterminate,
        }
    }
}

/// Whether one fact moved since the baseline recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Unchanged,
    Changed,
    /// Nothing was recorded, or nothing was read, to compare with.
    Unknown,
}

impl Change {
    fn between<T: PartialEq>(recorded: &T, current: &T) -> Self {
        if recorded == current {
            Self::Unchanged
        } else {
            Self::Changed
        }
    }

    const fn is_changed(self) -> bool {
        matches!(self, Self::Changed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalChange {
    pub frontmatter: Change,
    pub fields: Change,
    pub body: Change,
}

impl LocalChange {
    /// One change for every fact, as a legacy hash over the whole file
    /// reports it.
    const fn whole(change: Change) -> Self {
        Self {
            frontmatter: change,
            fields: change,
            body: change,
        }
    }

    const fn facts(self) -> [Change; 3] {
        [self.frontmatter, self.fields, self.body]
    }

    /// Whether every fact is proven unchanged.
    #[must_use]
    pub fn is_unchanged(self) -> bool {
        self.facts().iter().all(|fact| *fact == Change::Unchanged)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteChange {
    pub body: Change,
    pub fields: Change,
}

impl RemoteChange {
    const fn whole(change: Change) -> Self {
        Self {
            body: change,
            fields: change,
        }
    }

    const fn facts(self) -> [Change; 2] {
        [self.body, self.fields]
    }

    fn is_changed(self) -> bool {
        self.facts().iter().any(|fact| fact.is_changed())
    }
}

/// How the local side moved since `baseline`, under the entry's own
/// recipe; `watermark` is the epoch the local file was last synced.
///
/// The mtime pre-filter proves the file unchanged without hashing it when it
/// was not touched after the watermark.
///
/// # Errors
///
/// Propagates any [`ItemDigests`] read failure.
pub fn local_change(
    baseline: &BaselineEntry<'_>,
    watermark: u64,
    digests: &dyn ItemDigests,
) -> Result<LocalChange, kernel::Error> {
    let recorded_anything = match &baseline.fingerprints {
        BaselineFingerprints::Legacy(legacy) => legacy.local.is_some(),
        BaselineFingerprints::Split(split) => split.local.is_some(),
    };
    if !recorded_anything {
        return Ok(LocalChange::whole(Change::Unknown));
    }
    let may_have_moved = digests.mtime()?.is_none_or(|mtime| mtime > watermark);
    if !may_have_moved {
        return Ok(LocalChange::whole(Change::Unchanged));
    }
    Ok(match &baseline.fingerprints {
        BaselineFingerprints::Legacy(LegacyHashes {
            local: Some(recorded),
            ..
        }) => LocalChange::whole(Change::between(
            recorded,
            &digests.legacy_local()?,
        )),
        BaselineFingerprints::Split(SplitFingerprints {
            local: Some(recorded),
            ..
        }) => {
            let current = digests.local()?;
            LocalChange {
                frontmatter: Change::between(
                    &recorded.frontmatter,
                    &current.frontmatter,
                ),
                fields: Change::between(&recorded.fields, &current.fields),
                body: Change::between(&recorded.body, &current.body),
            }
        }
        _ => LocalChange::whole(Change::Unknown),
    })
}

fn remote_change(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    digests: &dyn ItemDigests,
) -> Result<RemoteChange, kernel::Error> {
    let recorded_anything = match &baseline.fingerprints {
        BaselineFingerprints::Legacy(legacy) => legacy.remote.is_some(),
        BaselineFingerprints::Split(split) => split.remote.is_some(),
    };
    if !recorded_anything {
        return Ok(RemoteChange::whole(Change::Unknown));
    }
    if subject
        .remote_updated
        .proves_unchanged_since(baseline.remote_updated_at)
    {
        return Ok(RemoteChange::whole(Change::Unchanged));
    }
    Ok(match &baseline.fingerprints {
        BaselineFingerprints::Legacy(LegacyHashes {
            remote: Some(recorded),
            ..
        }) => RemoteChange::whole(
            digests.legacy_remote()?.map_or(Change::Unknown, |current| {
                Change::between(recorded, &current)
            }),
        ),
        BaselineFingerprints::Split(SplitFingerprints {
            remote: Some(recorded),
            ..
        }) => digests.remote()?.map_or(
            RemoteChange::whole(Change::Unknown),
            |current: RemoteFingerprints| RemoteChange {
                body: Change::between(&recorded.body, &current.body),
                fields: Change::between(&recorded.fields, &current.fields),
            },
        ),
        _ => RemoteChange::whole(Change::Unknown),
    })
}

/// Both sides' movement since the baseline.
#[derive(Clone, Copy)]
struct Movement {
    local: LocalChange,
    remote: RemoteChange,
}

impl Movement {
    fn any_unknown(self) -> bool {
        self.local
            .facts()
            .into_iter()
            .chain(self.remote.facts())
            .any(|fact| fact == Change::Unknown)
    }

    fn is_unchanged(self) -> bool {
        self.local.is_unchanged() && !self.remote.is_changed()
    }

    const fn any_body_changed(self) -> bool {
        self.local.body.is_changed() || self.remote.body.is_changed()
    }
}

/// An item with a fact nothing can be compared with is settled only when
/// its two sides already agree; a recovered item is never pushed or pulled.
fn recover(comparison: &dyn ItemComparison) -> Classification {
    if comparison.body_equivalence() == BodyEquivalence::Equivalent
        && comparison.fields_comparison() == FieldsComparison::Same
    {
        Classification::Synced(SyncedReconciliation::RecordOnly)
    } else {
        Classification::Conflict
    }
}

/// Reconciles an item with no body to send: only the local frontmatter and
/// synced fields decide, so an unrelated frontmatter edit never sends a
/// stale title.
fn reconcile_fields(
    movement: Movement,
    comparison: &dyn ItemComparison,
) -> Classification {
    if !movement.local.fields.is_changed() {
        return if movement.local.frontmatter.is_changed() {
            Classification::LocallyModified(LocalReconciliation::RecordOnly)
        } else {
            Classification::Synced(SyncedReconciliation::RecordOnly)
        };
    }
    match (movement.remote.fields, comparison.fields_comparison()) {
        (_, FieldsComparison::Same) => {
            Classification::LocallyModified(LocalReconciliation::RecordOnly)
        }
        (Change::Unchanged, FieldsComparison::Different) => {
            Classification::LocallyModified(LocalReconciliation::SendFields)
        }
        _ => Classification::Conflict,
    }
}

/// Reconciles an item whose bodies differ, or cannot be compared, after a
/// body moved on at least one side.
fn reconcile_bodies(
    movement: Movement,
    equivalence: BodyEquivalence,
) -> Classification {
    let Movement { local, remote } = movement;
    let remote_changed = remote.is_changed();
    let frontmatter_edit_stands =
        local.frontmatter.is_changed() && !remote.body.is_changed();
    let local_changed = local.body.is_changed()
        || local.fields.is_changed()
        || frontmatter_edit_stands;
    if (local.fields.is_changed() && remote_changed)
        || (remote.fields.is_changed() && local_changed)
    {
        return Classification::Conflict;
    }
    if equivalence == BodyEquivalence::RemoteUnrenderable {
        return if local_changed {
            Classification::Conflict
        } else {
            Classification::RemotelyModified
        };
    }
    let fields = if local.fields.is_changed() {
        FieldDelivery::Send
    } else {
        FieldDelivery::Withhold
    };
    match (local_changed, remote_changed) {
        (false, false) => {
            Classification::Synced(SyncedReconciliation::ByContent)
        }
        (true, false) => {
            Classification::LocallyModified(LocalReconciliation::ByContent {
                fields,
            })
        }
        (false, true) => Classification::RemotelyModified,
        (true, true) => Classification::Conflict,
    }
}

fn classify_split(
    movement: Movement,
    comparison: &dyn ItemComparison,
) -> Classification {
    if movement.any_unknown() {
        return recover(comparison);
    }
    if movement.is_unchanged() {
        return Classification::Synced(SyncedReconciliation::ByContent);
    }
    if !movement.any_body_changed() {
        return reconcile_fields(movement, comparison);
    }
    match comparison.body_equivalence() {
        BodyEquivalence::Equivalent => reconcile_fields(movement, comparison),
        equivalence => reconcile_bodies(movement, equivalence),
    }
}

/// A legacy entry's single hash per side cannot tell which fact moved, nor
/// a local edit from a file an earlier pull polluted, so a local change is
/// settled only when the bodies are equivalent and the title cannot have
/// moved apart.
///
/// Residue decides first: a file holding the current remote's own
/// document is synced when nothing local can be unsent, and any other
/// residue is a conflict, so polluted content reaches a person rather than a
/// baseline or a send.
fn classify_legacy(
    movement: Movement,
    remote_fetched: bool,
    comparison: &dyn ItemComparison,
) -> Classification {
    match comparison.pull_residue() {
        Some(PullResidue::VerbatimRemote) => {
            return match movement.local.body {
                Change::Unchanged => {
                    Classification::Synced(SyncedReconciliation::RecordOnly)
                }
                Change::Unknown
                    if comparison.fields_comparison()
                        == FieldsComparison::Same =>
                {
                    Classification::Synced(SyncedReconciliation::RecordOnly)
                }
                _ => Classification::Conflict,
            };
        }
        Some(PullResidue::TrackerWireDocument) => {
            return Classification::Conflict
        }
        None => {}
    }
    if movement.any_unknown() {
        return recover(comparison);
    }
    if movement.is_unchanged() {
        return Classification::Synced(if remote_fetched {
            SyncedReconciliation::RecordOnly
        } else {
            SyncedReconciliation::ByContent
        });
    }
    let local_changed = !movement.local.is_unchanged();
    match comparison.body_equivalence() {
        BodyEquivalence::Equivalent => {
            if local_changed
                && comparison.fields_comparison() == FieldsComparison::Different
            {
                Classification::Conflict
            } else {
                reconcile_fields(movement, comparison)
            }
        }
        _ if local_changed => Classification::Conflict,
        equivalence => reconcile_bodies(movement, equivalence),
    }
}

/// Classifies one item against its baseline, under the entry's own recipe.
///
/// Each fact is unchanged, changed or unknown; when any is not unchanged,
/// the bodies' equivalence decides, so a cosmetic rewrite on either side is
/// never a change. A fact nothing can be compared with settles only an
/// item whose sides already agree, and is otherwise a conflict a human
/// resolves rather than a silent overwrite.
///
/// # Errors
///
/// Propagates any [`ItemDigests`] read failure.
pub fn classify(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    digests: &dyn ItemDigests,
    comparison: &dyn ItemComparison,
) -> Result<Classification, kernel::Error> {
    if subject.external_id.is_none() {
        return Ok(Classification::Unsynced);
    }

    match subject.presence {
        RemotePresence::Indeterminate => {
            return Ok(Classification::Indeterminate)
        }
        RemotePresence::Absent => return Ok(Classification::RemoteAbsent),
        RemotePresence::Present => {}
    }

    let movement = Movement {
        local: local_change(baseline, subject.baseline_timestamp, digests)?,
        remote: remote_change(subject, baseline, digests)?,
    };
    Ok(match &baseline.fingerprints {
        BaselineFingerprints::Legacy(_) => classify_legacy(
            movement,
            digests.legacy_remote()?.is_some(),
            comparison,
        ),
        BaselineFingerprints::Split(_) => classify_split(movement, comparison),
    })
}

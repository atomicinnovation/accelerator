//! The whole-corpus sync run: plan-then-apply over the gathered facts.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use corpus::body::BodyParser;
use corpus::store::AtomicWrite;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracker::Ceiling;
use tracker::Completeness;
use tracker::ExternalId;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker::Unrenderable;
use work::draft_id::DraftId;
use work::section_diff::SectionDiff;
use work::sync::plan as compute_plan;
use work::sync::Action;
use work::sync::FieldDelivery;
use work::sync::PlannedAction;
use work::sync::Resolution;
use work::sync::RunClock;
use work::sync::SyncDirection;
use work::sync::SyncPlan;
use work::sync::SyncState;

use crate::create_request_fields;
use crate::promotion::PromotionRow;
use crate::sync::apply::ApplyError;
use crate::sync::apply::CreateFromLocalRequest;
use crate::sync::apply::ItemApplier;
use crate::sync::apply::PullRequest;
use crate::sync::apply::PushRequest;
use crate::sync::apply::RecordRequest;
use crate::sync::apply::RetitleRequest;
use crate::sync::baseline::Degradation;
use crate::sync::baseline::Entry;
use crate::sync::baseline_digester::BaselineDigester;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::body_comparison::BodyComparison;
use crate::sync::create::canonical_external_key;
use crate::sync::create::LocalAuthor;
use crate::sync::fetch;
use crate::sync::fetch::GatheredFacts;
use crate::sync::fetch::GatheredRemote;
use crate::sync::fetch::LocalItem;
use crate::sync::fetch::RetrievalStrategy;
use crate::sync::fetch::WorkingCopyStatus;
use crate::sync::identity_settlement::IdentityRow;
use crate::sync::item_snapshot::ItemSnapshot;
use crate::sync::ordering;
use crate::sync::scope;
use crate::sync::send_refusal::legacy_unrenderable;
use crate::sync::send_refusal::send_refusal;
use crate::sync::send_refusal::EntryShape;
use crate::sync::send_refusal::SendRefusal;
use crate::sync::synced_fields::remote_fields;

#[derive(Debug)]
pub enum RunError {
    /// The plan's writes exceed a bound. `pulls`/`pushes` are the totals per
    /// direction; `new_local_files`/`new_remote_issues` break out how many of
    /// each total are brand-new artefacts (untracked-remote imports and
    /// unsynced-local creates respectively), so the operator sees the creation
    /// blast within the dimension that tripped.
    Refused {
        pulls: usize,
        pushes: usize,
        max_pulls: Ceiling,
        max_pushes: Ceiling,
        new_local_files: usize,
        new_remote_issues: usize,
    },
    /// A discovery query was cut short. Refused rather than acted on: an
    /// incomplete untracked set is a lower bound. `completeness` distinguishes
    /// a `max_pages` cap-hit (raise the cap) from a transient cutoff (retry),
    /// so the caller's guidance names the right remedy.
    DiscoveryIncomplete {
        found: usize,
        completeness: Completeness,
    },
    /// The discovery scope names no valid search target — a missing or
    /// unresolvable key. Caught pre-flight, before the apply/push phase, so
    /// nothing was sent; the operator fixes the config and re-runs.
    DiscoveryUnconfigured {
        detail: String,
    },
    /// The bulk keyed reconcile read hit its `max_pages` cap, so the remote
    /// state of the un-read items is unknown. Aborted before any write,
    /// all-or-nothing across both directions: the read feeds pull and push
    /// planning alike, so proceeding could push against unknown remote state or
    /// import a stale view. The remedy — raise `<tracker>.pull.max_pages` (its
    /// `keyed_read` override), or set it to `unlimited` — is named by the
    /// caller, which resolves the cap and the config file it came from.
    KeyedReadCapped,
    /// A retirement failed and could not restore every path it had written,
    /// so the corpus is not known to be consistent. Nothing after it ran.
    RetirementIncomplete {
        message: String,
    },
    Read(TrackerError),
    Internal(kernel::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Preview,
    Apply,
}

pub struct SyncPorts<'a> {
    pub tracker: &'a dyn RemoteTracker,
    pub parser: &'a dyn BodyParser,
    pub status: &'a dyn WorkingCopyStatus,
    pub writer: &'a dyn AtomicWrite,
    pub clock: &'a dyn RunClock,
    /// Authors the local files the two create paths need: a work-item file
    /// for a discovered remote issue, and the `external_id` write-back for an
    /// unsynced local draft. Behind a port because both need config, an id
    /// scheme, and a frontmatter renderer that live in the binary layer.
    pub author: &'a dyn LocalAuthor,
}

/// Which of the discovered corpus a run reconciles.
///
/// `All` is payload-free so `SyncRequest::corpus` is the single source of the
/// reconciled set — a construction site cannot desync the two, and the
/// invariant `reconciled() ⊆ corpus` holds structurally. `Targeted` gates the
/// untracked-remote *search* off, so a narrowed set can never re-import the
/// items it left out, and carries a third set beyond `corpus` and the
/// reconciled `items`: `pull_ids`, confirmed remote-only ids the run imports
/// by id in place of a discovery search. An id already bound to a local file
/// must not appear in `pull_ids`; the engine enforces this at the injection
/// point (see the discovery branch), so a caller need not pre-filter.
pub enum ItemSelection<'a> {
    All,
    Targeted {
        items: &'a [LocalItem],
        pull_ids: &'a [ExternalId],
    },
    /// A whole-corpus run after the identity pass, reconciling only the
    /// items the pass gathered facts for. Discovery and the document
    /// watermark behave as under `All`.
    Settled {
        items: &'a [LocalItem],
    },
}

pub struct SyncRequest<'a> {
    /// The full discovered set, always. Whole-corpus reads — the double-binding
    /// guard and untracked-remote discovery — read this regardless of the
    /// selection, or a narrowed run would misjudge what the corpus already
    /// carries.
    pub corpus: &'a [LocalItem],
    /// The reconciliation scope. Reconciliation reads go through
    /// [`SyncRequest::reconciled`].
    pub selection: ItemSelection<'a>,
    pub direction: SyncDirection,
    pub strategy: RetrievalStrategy,
    pub resolutions: &'a BTreeMap<String, Resolution>,
    pub max_pulls: Ceiling,
    pub max_pushes: Ceiling,
    pub mode: RunMode,
    /// Where the pending-push markers for unsynced-local creates live.
    pub integrations_root: &'a Path,
    /// The active tracker's name, naming the marker directory alongside the
    /// baseline.
    pub integration: &'a str,
    /// The scope untracked-remote discovery searches. Team/project-scoped by
    /// default so the untracked set stays bounded on a shared workspace.
    pub scope: SearchScope,
    /// Whether a run under a tracker-owned pattern promotes its drafts.
    pub promote: bool,
}

impl<'a> SyncRequest<'a> {
    #[must_use]
    pub const fn reconciled(&self) -> &'a [LocalItem] {
        match self.selection {
            ItemSelection::All => self.corpus,
            ItemSelection::Targeted { items, .. }
            | ItemSelection::Settled { items } => items,
        }
    }

    /// Whether the untracked-remote *search* is suppressed. A targeted run
    /// reaches remote-only items by id through `pull_ids`, not through the
    /// unkeyed discovery search, so the search is always off under `Targeted`.
    #[must_use]
    pub const fn discovery_search_suppressed(&self) -> bool {
        matches!(self.selection, ItemSelection::Targeted { .. })
    }
}

pub enum ItemOutcome {
    Applied,
    NotApplied,
    /// Nothing was read or written: the item's baseline entry was recorded
    /// under a digest recipe newer than this binary knows.
    UnsupportedRecipe,
    Failed(ApplyError),
}

pub struct ReportedItem {
    pub planned: PlannedAction,
    pub outcome: ItemOutcome,
    /// The local payload-composition check attached to a `Push` entry during
    /// a preview. `None` outside preview, and for every non-`Push` action.
    ///
    /// A `Rejected` outcome surfaces a locally-detectable missing required
    /// field before any mutation. It does not reproduce a live-tracker field
    /// check — a `Valid` preview does not guarantee the tracker accepts the
    /// push.
    pub validation: Option<tracker::ValidationOutcome>,
}

/// What untracked-remote discovery did on a run that got past the pre-flight
/// scope check, so a skip is never mistaken for a search that found nothing.
///
/// A config fault never reaches here — it is refused pre-flight as
/// [`RunError::DiscoveryUnconfigured`] before a report exists — so there is no
/// unconfigured variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryStatus {
    /// The search ran and completed, seeing `found` untracked issues. `found ==
    /// 0` is a real, distinguishable "searched and found nothing".
    Ran { found: usize },
    /// The run was push-only, so discovery was deliberately not run.
    SkippedPushOnly,
    /// The run named explicit targets, so the untracked-remote search was
    /// deliberately not run and no remote-only target was pulled by id.
    SkippedTargeted,
    /// The run named explicit targets and pulled `attempted` confirmed
    /// remote-only ids by id, in place of an untracked search. `attempted` is
    /// the count gated into the run, not the count that applied — a per-item
    /// `CreateFromRemote` failure still counts here.
    TargetedPull { attempted: usize },
    /// The search failed transiently (network). Maps to `RETRYABLE` (70), the
    /// fate a failed read carries.
    Failed { detail: String },
}

/// What the identity pass settled before the engine planned, so the engine
/// treats a key it saw move as tracked even when the move was not applied.
///
/// Obtained only from a settlement; a run with nothing to settle uses the
/// empty view.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettledView {
    followed_keys: BTreeSet<String>,
    promoted_keys: BTreeSet<String>,
}

impl SettledView {
    pub(crate) fn following(
        keys: impl IntoIterator<Item = ExternalId>,
    ) -> Self {
        Self {
            followed_keys: keys
                .into_iter()
                .map(|key| canonical_external_key(&key))
                .collect(),
            promoted_keys: BTreeSet::new(),
        }
    }

    /// The same view, also knowing the keys this run promoted drafts onto:
    /// the engine leaves those items for the next run.
    pub(crate) fn promoting(
        self,
        keys: impl IntoIterator<Item = ExternalId>,
    ) -> Self {
        Self {
            promoted_keys: keys
                .into_iter()
                .map(|key| canonical_external_key(&key))
                .collect(),
            ..self
        }
    }

    pub(crate) fn promoted(&self, id: &str) -> bool {
        self.promoted_keys
            .contains(&canonical_external_key(&ExternalId::new(id.to_owned())))
    }

    fn follows(&self, key: &ExternalId) -> bool {
        self.followed_keys.contains(&canonical_external_key(key))
    }
}

pub struct RunReport {
    /// The identity pass's rows, each keyed by the item's id when the run
    /// started. The engine's row for the same item is not rendered.
    pub identity: Vec<IdentityRow>,
    /// Every draft the run set out to promote, in id order.
    pub promotions: Vec<PromotionRow>,
    /// Items that appeared after the identity pass read the remote, left
    /// for the next run.
    pub deferred: usize,
    pub reported: Vec<ReportedItem>,
    pub read_failure: Option<TrackerError>,
    pub baseline_degradation: Degradation,
    pub finalised: bool,
    pub dossiers: Vec<ConflictDossier>,
    pub discovery: DiscoveryStatus,
    /// The bulk keyed read was cut short by a transient condition (a deadline,
    /// rate limit, or wire failure), so the affected items degraded to
    /// indeterminate and the run proceeded. A soft signal, not the fatal
    /// [`RunError::KeyedReadCapped`]: the operator sees the read was
    /// budget-limited rather than the incompleteness passing silently.
    pub keyed_read_budget_limited: bool,
}

impl RunReport {
    /// Items this run left for a human. Derived, never stored: a stored
    /// field could disagree with the report, yielding a run that prints
    /// conflicts and exits 0.
    pub fn awaiting_human(&self) -> impl Iterator<Item = &ReportedItem> {
        self.reported.iter().filter(|item| {
            matches!(
                item.planned.action,
                Action::Prompt | Action::SkipConflict | Action::SkipDirty
            ) || matches!(
                item.planned.state,
                SyncState::RemoteAbsent | SyncState::Indeterminate
            )
        })
    }
}

/// Whether an item was reconciled to a definite, up-to-date state this run, so
/// its change-detection watermark may advance. An item left for a human
/// (conflict, skipped-dirty, remote-absent), one whose remote read was
/// indeterminate, or one whose apply failed keeps the watermark of the last run
/// that did reconcile it, so a local edit predating this run is not buried.
/// Writes the run's baseline bookkeeping: blanks each unreconciled conflict's
/// local hash, advances the reconciled items' watermarks, and — only for a
/// whole-corpus run — advances the document-level fallback watermark. Returns
/// whether the write succeeded.
fn finalise_baseline(
    baseline: &mut BaselineStore<'_>,
    reported: &[ReportedItem],
    blank_local_hash: &[String],
    run_start_epoch: u64,
    selection: &ItemSelection<'_>,
) -> bool {
    let blank_refs: Vec<&str> =
        blank_local_hash.iter().map(String::as_str).collect();
    let advance_ids: Vec<&str> = reported
        .iter()
        .filter(|item| definitively_reconciled(item))
        .map(|item| item.planned.id.as_str())
        .collect();
    let advance_document = matches!(
        selection,
        ItemSelection::All | ItemSelection::Settled { .. }
    );
    baseline
        .finalise_run(
            &blank_refs,
            &advance_ids,
            run_start_epoch,
            advance_document,
        )
        .is_ok()
}

/// Whether a run settled `item`, so its watermark may move. A change the run
/// leaves pending, such as a local edit under `--pull-only`, keeps the old
/// watermark, or the next run's mtime pre-filter would hide it.
const fn definitively_reconciled(item: &ReportedItem) -> bool {
    let deferred = matches!(item.planned.action, Action::Noop)
        && matches!(
            item.planned.state,
            SyncState::LocallyModified | SyncState::RemotelyModified
        );
    !deferred
        && !matches!(item.outcome, ItemOutcome::Failed(_))
        && !matches!(
            item.planned.action,
            Action::Prompt | Action::SkipConflict | Action::SkipDirty
        )
        && !matches!(
            item.planned.state,
            SyncState::RemoteAbsent | SyncState::Indeterminate
        )
}

/// A local work item whose frontmatter parses, so its title and body can be
/// sent and its body replaced by a pull.
struct LocalWorkItem {
    frontmatter: String,
    title: String,
    body: String,
}

impl LocalWorkItem {
    fn parse(content: &str) -> Result<Self, String> {
        let (frontmatter, body) =
            crate::sync::digest::split_frontmatter_and_body(content)
                .map_err(|error| error.to_string())?;
        let title = create_request_fields::read(content)
            .map_err(|error| error.to_string())?
            .title;
        Ok(Self {
            frontmatter,
            title,
            body,
        })
    }

    fn with_body(&self, body: &str) -> String {
        format!("---\n{}\n---\n\n{body}", self.frontmatter)
    }
}

fn unparseable_local(item_id: &str, detail: String) -> ApplyError {
    ApplyError::Io {
        item_id: item_id.to_owned(),
        operation: "parse-local",
        detail,
    }
}

/// The six fields a user needs to choose a side of one conflict, gathered
/// from the run's own facts. Structured: rendering the sections to text is a
/// separate, injectable step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictDossier {
    pub id: String,
    pub title: String,
    /// The remote's title, when it was read.
    pub remote_title: Option<String>,
    pub local_modified: Option<u64>,
    pub remote_updated: RemoteTimestamp,
    pub sections: Vec<SectionDiff>,
    pub unrenderable: Option<UnrenderableReason>,
}

/// Why a dossier's sections cannot be shown. A remote-side reason carries
/// the content the remote holds that has no Markdown form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnrenderableReason {
    LocalUnreadable,
    RemoteUnrenderable(Unrenderable),
    LocalPullResidue(Unrenderable),
    /// The remote has no Markdown form and the item's legacy entry cannot
    /// show which local change is unsent, so no local overwrite is safe.
    LegacyUnrenderable(Unrenderable),
}

impl UnrenderableReason {
    const fn keyword(&self) -> &'static str {
        match self {
            Self::LocalUnreadable => "local-unreadable",
            Self::RemoteUnrenderable(_) => "remote-unrenderable",
            Self::LocalPullResidue(_) => "local-pull-residue",
            Self::LegacyUnrenderable(_) => "legacy-unrenderable",
        }
    }

    const fn cause(&self) -> Option<&Unrenderable> {
        match self {
            Self::LocalUnreadable => None,
            Self::RemoteUnrenderable(cause)
            | Self::LocalPullResidue(cause)
            | Self::LegacyUnrenderable(cause) => Some(cause),
        }
    }

    fn remedy(&self, id: &str) -> String {
        match self {
            Self::LocalUnreadable => format!(
                "This conflict could not be rendered: the local file could \
                 not be read. Item {id} was left unresolved. Fix the file and \
                 re-run the sync, or edit the work item by hand."
            ),
            Self::RemoteUnrenderable(_) => format!(
                "This conflict could not be rendered: the remote description \
                 holds content with no Markdown form. Item {id} was left \
                 unresolved. Either edit the description in Jira until it \
                 renders and re-run the sync, or resolve with --resolve \
                 {id}=local, which replaces the remote description with the \
                 local body."
            ),
            Self::LocalPullResidue(_) => format!(
                "This conflict could not be rendered: the local file still \
                 holds the raw tracker document an earlier pull wrote, and \
                 the remote description holds content with no Markdown form. \
                 Item {id} was left unresolved. If the file holds the \
                 remote's current document and only its frontmatter changed, \
                 revert that edit, sync once so the item's baseline is \
                 rewritten, then reapply it. Otherwise edit the description \
                 in Jira until it renders, then resolve with --resolve \
                 {id}=remote."
            ),
            Self::LegacyUnrenderable(_) => format!(
                "This conflict could not be rendered: the remote description \
                 holds content with no Markdown form, and the item's baseline \
                 predates the current digest recipe, so it cannot show which \
                 local change is unsent. Item {id} was left unresolved, and \
                 --resolve {id}=local is refused. If only the frontmatter \
                 changed and the remote description has not, revert that \
                 edit, sync once so the item's baseline is rewritten, then \
                 reapply it. Otherwise edit the description in Jira until it \
                 renders, then resolve with --resolve {id}=remote."
            ),
        }
    }
}

/// A dossier rendered to text, tagged by whether its sections could be
/// shown.
pub enum DossierRender {
    Renderable(String),
    Unrenderable(String),
}

fn format_epoch_utc(epoch: u64) -> String {
    i64::try_from(epoch)
        .ok()
        .and_then(|secs| OffsetDateTime::from_unix_timestamp(secs).ok())
        .and_then(|instant| instant.format(&Rfc3339).ok())
        .unwrap_or_else(|| "(unavailable)".to_owned())
}

fn render_stamp(stamp: &RemoteTimestamp) -> String {
    stamp
        .reported()
        .map_or_else(|| "(unavailable)".to_owned(), str::to_owned)
}

fn render_local_modified(local_modified: Option<u64>) -> String {
    local_modified.map_or_else(|| "(unavailable)".to_owned(), format_epoch_utc)
}

fn render_titles(dossier: &ConflictDossier) -> String {
    match &dossier.remote_title {
        Some(remote) if *remote != dossier.title => {
            format!("{} → {remote}", dossier.title)
        }
        _ => dossier.title.clone(),
    }
}

fn dossier_header(dossier: &ConflictDossier, status_lines: &str) -> String {
    format!(
        "# Conflict: {}\n{status_lines}title: {}\nlocal modified: {}\nremote \
         updated: {}\n\n",
        dossier.id,
        render_titles(dossier),
        render_local_modified(dossier.local_modified),
        render_stamp(&dossier.remote_updated),
    )
}

fn renderable_header(dossier: &ConflictDossier) -> String {
    dossier_header(dossier, "status: renderable\n")
}

fn unrenderable_header(
    dossier: &ConflictDossier,
    reason: &UnrenderableReason,
) -> String {
    let cause_line = reason
        .cause()
        .map(|cause| format!("cause: {}\n", cause.detail))
        .unwrap_or_default();
    let status_lines = format!(
        "status: unrenderable\nreason: {}\n{cause_line}",
        reason.keyword()
    );
    format!(
        "{}{}\n\n",
        dossier_header(dossier, &status_lines),
        reason.remedy(&dossier.id),
    )
}

/// Renders a dossier to text.
///
/// An infallible header, then each section through the injected `render`.
/// A dossier with an [`UnrenderableReason`] downgrades to
/// [`DossierRender::Unrenderable`] without rendering any section.
pub fn render_dossier(
    dossier: &ConflictDossier,
    render: &dyn Fn(&SectionDiff) -> String,
) -> DossierRender {
    if let Some(reason) = &dossier.unrenderable {
        return DossierRender::Unrenderable(unrenderable_header(
            dossier, reason,
        ));
    }
    let mut sections = String::new();
    for section in &dossier.sections {
        sections.push_str(&render(section));
    }
    DossierRender::Renderable(renderable_header(dossier) + &sections)
}

fn file_mtime_secs(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| {
            modified.duration_since(std::time::UNIX_EPOCH).ok()
        })
        .map(|duration| duration.as_secs())
}

/// Each reconciled item's baseline entry shape, read once per run.
struct EntryShapes(BTreeMap<String, EntryShape>);

impl EntryShapes {
    fn read(
        items: &[LocalItem],
        baseline: &crate::sync::baseline::Baseline,
    ) -> Self {
        Self(
            items
                .iter()
                .map(|item| {
                    (item.id.clone(), EntryShape::of(baseline.get(&item.id)))
                })
                .collect(),
        )
    }

    fn of(&self, id: &str) -> EntryShape {
        self.0.get(id).copied().unwrap_or(EntryShape::Legacy)
    }
}

/// Whether `item` is a conflict that keeps its local hash: one with no safe
/// local overwrite, whose remedy is to revert a metadata edit so the entry
/// migrates, which needs the hash it was recorded with.
fn keeps_local_hash(item: &LocalItem, knowledge: &RunKnowledge<'_>) -> bool {
    let Some(projected) = knowledge.projection(&item.id) else {
        return false;
    };
    knowledge.local_content(item).is_ok_and(|content| {
        LocalWorkItem::parse(content).is_ok_and(|local| {
            legacy_unrenderable(
                knowledge.comparison,
                knowledge.shape(&item.id),
                &local.body,
                projected,
            )
        })
    })
}

/// What this run knows about each reconciled item: its local file as the
/// run read it, the remote it was compared with, and its baseline entry's
/// shape. Each item and its snapshot are paired once, here.
struct RunKnowledge<'a> {
    facts: GatheredFacts,
    comparison: &'a BodyComparison<'a>,
    items: &'a [LocalItem],
    snapshots: Vec<ItemSnapshot<'a>>,
    shapes: EntryShapes,
    by_id: BTreeMap<&'a str, usize>,
}

impl<'a> RunKnowledge<'a> {
    fn gather(
        facts: GatheredFacts,
        items: &'a [LocalItem],
        baseline: &crate::sync::baseline::Baseline,
        digester: &'a BaselineDigester<'a>,
    ) -> Self {
        let snapshots = items
            .iter()
            .map(|item| {
                let remote_content = facts
                    .per_id
                    .get(&item.id)
                    .and_then(|(remote, _)| remote.body.clone());
                ItemSnapshot::new(&item.path, remote_content, digester)
            })
            .collect();
        let by_id = items
            .iter()
            .enumerate()
            .map(|(position, item)| (item.id.as_str(), position))
            .collect();
        Self {
            facts,
            comparison: digester.comparison(),
            items,
            snapshots,
            shapes: EntryShapes::read(items, baseline),
            by_id,
        }
    }

    fn plan(
        &self,
        baseline: &crate::sync::baseline::Baseline,
        request: &SyncRequest<'_>,
    ) -> Result<SyncPlan, RunError> {
        let plan_inputs =
            self.facts
                .plan_inputs(self.items, &self.snapshots, baseline);
        compute_plan(&plan_inputs, request.direction, request.resolutions)
            .map_err(RunError::Internal)
    }

    fn item(&self, id: &str) -> Option<&'a LocalItem> {
        self.by_id
            .get(id)
            .and_then(|&position| self.items.get(position))
    }

    fn snapshot(&self, id: &str) -> Option<&ItemSnapshot<'a>> {
        self.by_id
            .get(id)
            .and_then(|&position| self.snapshots.get(position))
    }

    fn shape(&self, id: &str) -> EntryShape {
        self.shapes.of(id)
    }

    fn remote(&self, id: &str) -> Option<&GatheredRemote> {
        self.facts.per_id.get(id).map(|(remote, _)| remote)
    }

    fn projection(&self, id: &str) -> Option<&str> {
        self.remote(id).and_then(|remote| remote.body.as_deref())
    }

    /// `item`'s file as the run read it, before classifying it.
    fn local_content(&self, item: &LocalItem) -> Result<&str, ApplyError> {
        let snapshot =
            self.snapshot(&item.id).ok_or_else(|| ApplyError::Io {
                item_id: item.id.clone(),
                operation: "read-local",
                detail: "the run took no snapshot of the file".to_owned(),
            })?;
        snapshot.content().map_err(|error| ApplyError::Io {
            item_id: item.id.clone(),
            operation: "read-local",
            detail: error.to_string(),
        })
    }

    fn remote_updated(&self, id: &str) -> RemoteTimestamp {
        self.remote(id).map_or(RemoteTimestamp::NotRead, |remote| {
            remote.remote_updated.clone()
        })
    }
}

fn build_dossiers(
    plan: &SyncPlan,
    knowledge: &RunKnowledge<'_>,
) -> Vec<ConflictDossier> {
    plan.actions
        .iter()
        .filter(|planned| planned.action == Action::Prompt)
        .filter_map(|planned| {
            let item = knowledge.item(&planned.id)?;
            Some(dossier_for(&planned.id, item, knowledge))
        })
        .collect()
}

fn dossier_for(
    id: &str,
    item: &LocalItem,
    knowledge: &RunKnowledge<'_>,
) -> ConflictDossier {
    let local = knowledge.local_content(item).ok().and_then(|content| {
        Some((content, LocalWorkItem::parse(content).ok()?))
    });
    match local {
        Some((content, local)) => {
            readable_dossier(id, item, knowledge, content, &local)
        }
        None => unreadable_dossier(id, knowledge.remote_updated(id)),
    }
}

fn unreadable_dossier(
    id: &str,
    remote_updated: RemoteTimestamp,
) -> ConflictDossier {
    ConflictDossier {
        id: id.to_owned(),
        title: String::new(),
        remote_title: None,
        local_modified: None,
        remote_updated,
        sections: Vec::new(),
        unrenderable: Some(UnrenderableReason::LocalUnreadable),
    }
}

fn readable_dossier(
    id: &str,
    item: &LocalItem,
    knowledge: &RunKnowledge<'_>,
    local_content: &str,
    local: &LocalWorkItem,
) -> ConflictDossier {
    let comparison = knowledge.comparison;
    let remote_projection = knowledge.projection(id).unwrap_or("");
    let unrenderable_reason = |cause| {
        if comparison
            .pull_residue(&local.body, remote_projection)
            .is_some()
        {
            UnrenderableReason::LocalPullResidue(cause)
        } else if legacy_unrenderable(
            comparison,
            knowledge.shape(id),
            &local.body,
            remote_projection,
        ) {
            UnrenderableReason::LegacyUnrenderable(cause)
        } else {
            UnrenderableReason::RemoteUnrenderable(cause)
        }
    };
    let (sections, unrenderable) =
        comparison.local_body(remote_projection).map_or_else(
            |cause| (Vec::new(), Some(unrenderable_reason(cause))),
            |remote_body| {
                let reconstructed = local.with_body(&remote_body);
                (
                    comparison
                        .differing_sections(local_content, &reconstructed),
                    None,
                )
            },
        );
    ConflictDossier {
        id: id.to_owned(),
        title: local.title.clone(),
        remote_title: knowledge
            .projection(id)
            .map(|projection| remote_fields(projection).title),
        local_modified: file_mtime_secs(&item.path),
        remote_updated: knowledge.remote_updated(id),
        sections,
        unrenderable,
    }
}

/// Reports the whole plan for a preview, attaching to each send either the
/// refusal the run would meet, as a failure, or a local payload-validation
/// outcome.
///
/// Every other planned action is reported `NotApplied`, so the report never
/// shrinks below the plan. The `validate_update` call is a local composition
/// check — it makes no remote call — so a `Rejected` outcome names a
/// locally-detectable missing field without reproducing the tracker's own
/// field validation.
fn validate_pushes(
    plan: &work::sync::SyncPlan,
    tracker: &dyn RemoteTracker,
    knowledge: &RunKnowledge<'_>,
) -> Vec<ReportedItem> {
    plan.actions
        .iter()
        .map(|planned| {
            let checked = if planned.action == Action::Pull {
                check_pull(planned, knowledge)
            } else {
                planned
                    .action
                    .sends_to_tracker()
                    .then(|| check_send(planned, tracker, knowledge))
                    .flatten()
            };
            match checked {
                Some(Err(refused)) => ReportedItem {
                    planned: planned.clone(),
                    outcome: ItemOutcome::Failed(refused),
                    validation: None,
                },
                validated => ReportedItem {
                    planned: planned.clone(),
                    outcome: ItemOutcome::NotApplied,
                    validation: validated.and_then(Result::ok),
                },
            }
        })
        .collect()
}

/// Why a pull would fail before writing anything, as the run would report it.
fn check_pull(
    planned: &PlannedAction,
    knowledge: &RunKnowledge<'_>,
) -> Option<Result<tracker::ValidationOutcome, ApplyError>> {
    let item = knowledge.item(&planned.id)?;
    pulled_file(item, knowledge).err().map(Err)
}

/// What a send carries: the local title and body, either withheld.
struct Payload<'a> {
    title: Option<&'a str>,
    body: Option<&'a str>,
}

impl<'a> Payload<'a> {
    /// `None` for an action that sends nothing.
    const fn of(action: Action, title: &'a str, body: &'a str) -> Option<Self> {
        match action {
            Action::Push(FieldDelivery::Send) | Action::Overwrite => {
                Some(Self {
                    title: Some(title),
                    body: Some(body),
                })
            }
            Action::Push(FieldDelivery::Withhold) => Some(Self {
                title: None,
                body: Some(body),
            }),
            Action::Retitle => Some(Self {
                title: Some(title),
                body: None,
            }),
            _ => None,
        }
    }
}

/// A send that passed every refusal, ready to validate or to send, with the
/// local file it was composed from.
struct PreparedSend<'k> {
    action: Action,
    content: &'k str,
    local: LocalWorkItem,
}

impl PreparedSend<'_> {
    fn payload(&self) -> Option<Payload<'_>> {
        Payload::of(self.action, &self.local.title, &self.local.body)
    }
}

/// The one gate both a preview and a run put a send through, so each
/// reports the same failure.
fn prepare_send<'k>(
    planned: &PlannedAction,
    item: &LocalItem,
    knowledge: &'k RunKnowledge<'_>,
) -> Result<PreparedSend<'k>, ApplyError> {
    let content = knowledge.local_content(item)?;
    let local = LocalWorkItem::parse(content)
        .map_err(|detail| unparseable_local(&item.id, detail))?;
    if let Some(refusal) = send_refusal(
        knowledge.comparison,
        planned.action,
        knowledge.shape(&item.id),
        &local.body,
        knowledge.projection(&item.id),
    ) {
        return Err(refused_send(&item.id, refusal));
    }
    Ok(PreparedSend {
        action: planned.action,
        content,
        local,
    })
}

/// The refusal a send would meet, or else the tracker's payload check of
/// exactly what the action sends.
fn check_send(
    planned: &PlannedAction,
    tracker: &dyn RemoteTracker,
    knowledge: &RunKnowledge<'_>,
) -> Option<Result<tracker::ValidationOutcome, ApplyError>> {
    let item = knowledge.item(&planned.id)?;
    let external_id = item.external_id.as_ref()?;
    let prepared = match prepare_send(planned, item, knowledge) {
        Ok(prepared) => prepared,
        Err(refused) => return Some(Err(refused)),
    };
    let payload = prepared.payload()?;
    Some(Ok(tracker.validate_update(
        external_id,
        payload.title,
        payload.body,
    )))
}

fn refused_send(item_id: &str, refusal: SendRefusal) -> ApplyError {
    let item_id = item_id.to_owned();
    match refusal {
        SendRefusal::PullResidue => ApplyError::LocalPullResidue { item_id },
        SendRefusal::RemoteUnrenderable(cause) => {
            ApplyError::RemoteUnrenderable { item_id, cause }
        }
        SendRefusal::RemoteUnfetched => {
            ApplyError::RemoteBodyUnavailable { item_id }
        }
    }
}

struct Discovered {
    ids: Vec<ExternalId>,
    completeness: Completeness,
}

/// Whether any local item in `corpus` already carries `candidate` as its
/// `external_id`, compared by canonical key so a cosmetic spelling difference
/// folds equal — the same folding untracked discovery applies. Shared by the
/// targeted-pull filter (so a `pull_id` already bound locally is never
/// re-imported) and the create-from-local double-binding guard.
fn corpus_carries(
    corpus: &[LocalItem],
    settled: &SettledView,
    candidate: &ExternalId,
) -> bool {
    let key = canonical_external_key(candidate);
    settled.follows(candidate)
        || corpus
            .iter()
            .filter_map(|item| item.external_id.as_ref())
            .any(|external| canonical_external_key(external) == key)
}

/// The untracked remote issues: a `search` over `scope` minus the
/// canonicalised set of local `external_id`s. A stored id differing from a
/// search result only cosmetically folds equal and is excluded.
fn discover_untracked(
    tracker: &dyn RemoteTracker,
    scope: &SearchScope,
    items: &[LocalItem],
    settled: &SettledView,
) -> Result<Discovered, TrackerError> {
    let discovery = tracker.search(scope)?;
    let local: std::collections::BTreeSet<String> = items
        .iter()
        .filter_map(|item| item.external_id.as_ref())
        .map(canonical_external_key)
        .collect();
    // Dedup discovered-vs-discovered by canonical key — one issue reached via
    // several scopes appears once — before dropping those already local, then
    // reconcile in a total, deterministic order.
    let mut seen = std::collections::BTreeSet::new();
    let mut ids: Vec<ExternalId> = discovery
        .found
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| seen.insert(canonical_external_key(id)))
        .filter(|id| !local.contains(&canonical_external_key(id)))
        .filter(|id| !settled.follows(id))
        .collect();
    ids.sort_by(ordering::discovered_order);
    Ok(Discovered {
        ids,
        completeness: discovery.completeness,
    })
}

/// The unsynced local items eligible for create-from-local: state `Unsynced`
/// (no `external_id`), under a push-capable direction. A draft is excluded:
/// it reaches the tracker only through promotion, which retires its ID.
fn unsynced_creates<'a>(
    plan: &work::sync::SyncPlan,
    items: &'a [LocalItem],
    direction: SyncDirection,
) -> Vec<&'a LocalItem> {
    if matches!(direction, SyncDirection::PullOnly) {
        return Vec::new();
    }
    plan.actions
        .iter()
        .filter(|planned| planned.state == SyncState::Unsynced)
        .filter_map(|planned| items.iter().find(|item| item.id == planned.id))
        .filter(|item| DraftId::parse(&item.id).is_none())
        .collect()
}

/// The `(title, body, kind)` a create-from-local needs, read from the draft's
/// own frontmatter and body. `None` when the file is unreadable or malformed.
fn create_inputs(path: &Path) -> Option<(String, String, String)> {
    let content = std::fs::read_to_string(path).ok()?;
    let (_, body) =
        crate::sync::digest::split_frontmatter_and_body(&content).ok()?;
    let fields = create_request_fields::read(&content).ok()?;
    Some((fields.title, body, fields.kind))
}

/// A create report line, always `Unsynced`-stated so it renders as an action
/// row rather than folding into the synced count, and never awaiting-human.
const fn create_report(
    id: String,
    action: Action,
    outcome: ItemOutcome,
) -> ReportedItem {
    ReportedItem {
        planned: PlannedAction {
            id,
            state: SyncState::Unsynced,
            action,
        },
        outcome,
        validation: None,
    }
}

fn send_outcome(
    applier: &mut ItemApplier<'_, '_>,
    planned: &PlannedAction,
    item: &LocalItem,
    external_id: &ExternalId,
    knowledge: &RunKnowledge<'_>,
) -> ItemOutcome {
    let prepared = match prepare_send(planned, item, knowledge) {
        Ok(prepared) => prepared,
        Err(refused) => return ItemOutcome::Failed(refused),
    };
    let sent = match prepared.payload() {
        Some(Payload {
            title: Some(title),
            body: None,
        }) => applier.retitle(&RetitleRequest {
            id: &item.id,
            external_id,
            title,
            sent_content: prepared.content,
        }),
        Some(Payload {
            title,
            body: Some(body),
        }) => applier.push(&PushRequest {
            id: &item.id,
            external_id,
            title,
            body,
            sent_content: prepared.content,
        }),
        _ => return ItemOutcome::NotApplied,
    };
    match sent {
        Ok(()) => ItemOutcome::Applied,
        Err(error) => ItemOutcome::Failed(error),
    }
}

/// The file a pull of `item` would write over the one it was classified
/// against, and the remote it came from.
struct PulledFile<'a> {
    classified_content: &'a str,
    content: String,
    projected_body: &'a str,
    remote_updated: RemoteTimestamp,
}

fn pulled_file<'a>(
    item: &LocalItem,
    knowledge: &'a RunKnowledge<'_>,
) -> Result<PulledFile<'a>, ApplyError> {
    let projected_body = knowledge.projection(&item.id).ok_or_else(|| {
        ApplyError::RemoteBodyUnavailable {
            item_id: item.id.clone(),
        }
    })?;
    let remote_body =
        knowledge
            .comparison
            .local_body(projected_body)
            .map_err(|cause| ApplyError::RemoteUnrenderable {
                item_id: item.id.clone(),
                cause,
            })?;
    let classified_content = knowledge.local_content(item)?;
    let local = LocalWorkItem::parse(classified_content)
        .map_err(|detail| unparseable_local(&item.id, detail))?;
    Ok(PulledFile {
        classified_content,
        content: local.with_body(&remote_body),
        projected_body,
        remote_updated: knowledge.remote_updated(&item.id),
    })
}

fn pull_outcome(
    applier: &mut ItemApplier<'_, '_>,
    item: &LocalItem,
    knowledge: &RunKnowledge<'_>,
) -> ItemOutcome {
    let pulled = match pulled_file(item, knowledge) {
        Ok(pulled) => pulled,
        Err(error) => return ItemOutcome::Failed(error),
    };
    match applier.pull(&PullRequest {
        id: &item.id,
        file_path: &item.path,
        classified_content: pulled.classified_content,
        content: &pulled.content,
        projected_body: pulled.projected_body,
        remote_updated: pulled.remote_updated,
    }) {
        Ok(()) => ItemOutcome::Applied,
        Err(error) => ItemOutcome::Failed(error),
    }
}

fn record_outcome(
    applier: &ItemApplier<'_, '_>,
    item: &LocalItem,
    snapshot: &ItemSnapshot<'_>,
    knowledge: &RunKnowledge<'_>,
) -> ItemOutcome {
    let fingerprints = match snapshot.recorded() {
        Ok(fingerprints) => fingerprints,
        Err(error) => {
            return ItemOutcome::Failed(ApplyError::Io {
                item_id: item.id.clone(),
                operation: "hash-local",
                detail: error.to_string(),
            })
        }
    };
    match applier.record_baseline(&RecordRequest {
        id: &item.id,
        file_path: &item.path,
        fingerprints,
        remote_updated: knowledge.remote_updated(&item.id),
    }) {
        Ok(()) => ItemOutcome::Applied,
        Err(error) => ItemOutcome::Failed(error),
    }
}

/// `item` reported as untouched for want of a known digest recipe, when its
/// entry was recorded under one this binary does not know.
fn noting_unsupported_recipe(
    item: ReportedItem,
    unsupported_recipe: &BTreeSet<String>,
) -> ReportedItem {
    if unsupported_recipe.contains(&item.planned.id)
        && matches!(item.outcome, ItemOutcome::NotApplied)
    {
        ReportedItem {
            outcome: ItemOutcome::UnsupportedRecipe,
            ..item
        }
    } else {
        item
    }
}

const fn not_applied(planned: PlannedAction) -> ReportedItem {
    ReportedItem {
        planned,
        outcome: ItemOutcome::NotApplied,
        validation: None,
    }
}

/// Applies one id-keyed planned action, returning its report line. A `Conflict`
/// under any skip action records its id for baseline blanking, unless it
/// [keeps its local hash](keeps_local_hash). The two create actions are
/// unreachable: both create paths run out-of-band of this loop.
fn apply_planned_action(
    applier: &mut ItemApplier<'_, '_>,
    planned: PlannedAction,
    knowledge: &RunKnowledge<'_>,
    blank_local_hash: &mut Vec<String>,
) -> ReportedItem {
    let item = knowledge.item(&planned.id);
    let outcome = match planned.action {
        Action::RecordBaseline => {
            match item.zip(knowledge.snapshot(&planned.id)) {
                Some((item, snapshot)) => {
                    record_outcome(applier, item, snapshot, knowledge)
                }
                None => return not_applied(planned),
            }
        }
        Action::Push(_) | Action::Retitle | Action::Overwrite => match item
            .and_then(|item| item.external_id.as_ref().map(|id| (item, id)))
        {
            Some((item, external_id)) => {
                send_outcome(applier, &planned, item, external_id, knowledge)
            }
            None => return not_applied(planned),
        },
        Action::Pull => match item {
            Some(item) => pull_outcome(applier, item, knowledge),
            None => return not_applied(planned),
        },
        Action::Prompt | Action::SkipConflict | Action::SkipDirty => {
            let keeps_hash =
                item.is_some_and(|item| keeps_local_hash(item, knowledge));
            if matches!(planned.state, SyncState::Conflict) && !keeps_hash {
                blank_local_hash.push(planned.id.clone());
            }
            ItemOutcome::NotApplied
        }
        Action::Noop => ItemOutcome::NotApplied,
        Action::CreateFromRemote | Action::CreateFromLocal => unreachable!(
            "the two create actions are applied out-of-band by the create \
             paths, never through the id-keyed plan loop, and decide() never \
             produces them"
        ),
    };
    ReportedItem {
        planned,
        outcome,
        validation: None,
    }
}

fn apply_local_create(
    applier: &mut ItemApplier<'_, '_>,
    item: &LocalItem,
    request: &SyncRequest<'_>,
    author: &dyn LocalAuthor,
    corpus_carries: &dyn Fn(&ExternalId) -> bool,
    attempted_at: u64,
) -> ReportedItem {
    let Some((title, body, kind)) = create_inputs(&item.path) else {
        return create_report(
            item.id.clone(),
            Action::CreateFromLocal,
            ItemOutcome::Failed(ApplyError::Io {
                item_id: item.id.clone(),
                operation: "read-local",
                detail: "unreadable or malformed frontmatter".to_owned(),
            }),
        );
    };
    let marker_path = crate::sync::pending_push::path(
        request.integrations_root,
        request.integration,
        &item.id,
    );
    let outcome = match applier.create_from_local(&CreateFromLocalRequest {
        item_id: &item.id,
        file_path: &item.path,
        title: &title,
        body: &body,
        kind: &kind,
        marker_path: &marker_path,
        author,
        corpus_carries,
        attempted_at,
    }) {
        Ok(()) => ItemOutcome::Applied,
        Err(error) => ItemOutcome::Failed(error),
    };
    create_report(item.id.clone(), Action::CreateFromLocal, outcome)
}

/// The gathered facts and bounded plan a run acts on, shared verbatim by the
/// preview and apply paths so both report the same blast radius.
struct PreparedRun<'a> {
    run_start_epoch: u64,
    unsupported_recipe: BTreeSet<String>,
    degradation: Degradation,
    read_failure: Option<TrackerError>,
    keyed_read_budget_limited: bool,
    knowledge: RunKnowledge<'a>,
    plan: SyncPlan,
    untracked: Vec<ExternalId>,
    discovery: DiscoveryStatus,
    creates_from_local: Vec<&'a LocalItem>,
    dossiers: Vec<ConflictDossier>,
}

/// Maps the bulk keyed read's completeness to the run's fate. A cap-hit is the
/// fail-loud, zero-write abort; a transient cutoff is a soft signal the run
/// carries and proceeds; a complete read is silent. The one place the
/// completeness signal is turned into a decision, so a new completeness cause
/// routes here rather than at a scattered inline check.
const fn classify_keyed_read(
    completeness: Completeness,
) -> Result<bool, RunError> {
    match completeness {
        Completeness::Complete => Ok(false),
        Completeness::Transient => Ok(true),
        Completeness::CapHit => Err(RunError::KeyedReadCapped),
    }
}

/// The remote-side ids a run imports and the discovery line that describes how
/// they were found. A `Targeted` run pulls its confirmed `pull_ids` by id,
/// dropping any already bound in the corpus; a full run resolves its scope and
/// searches for untracked issues; a push-only full run does neither.
///
/// Computed from reads only, so the combined gate in [`prepare_run`] can bound
/// every write — planned pulls, planned pushes, and both create paths — before
/// a single one runs.
///
/// # Errors
///
/// [`RunError::DiscoveryUnconfigured`] when a full run's scope names no valid
/// target; [`RunError::DiscoveryIncomplete`] when the untracked search is cut
/// short.
fn untracked_to_import(
    ports: &SyncPorts<'_>,
    request: &SyncRequest<'_>,
    settled: &SettledView,
) -> Result<(Vec<ExternalId>, DiscoveryStatus), RunError> {
    Ok(match &request.selection {
        ItemSelection::Targeted { pull_ids, .. } => {
            let confirmed: Vec<ExternalId> = pull_ids
                .iter()
                .filter(|id| !corpus_carries(request.corpus, settled, id))
                .cloned()
                .collect();
            if confirmed.is_empty() {
                (Vec::new(), DiscoveryStatus::SkippedTargeted)
            } else {
                let attempted = confirmed.len();
                (confirmed, DiscoveryStatus::TargetedPull { attempted })
            }
        }
        ItemSelection::All | ItemSelection::Settled { .. }
            if matches!(request.direction, SyncDirection::PushOnly) =>
        {
            (Vec::new(), DiscoveryStatus::SkippedPushOnly)
        }
        ItemSelection::All | ItemSelection::Settled { .. } => {
            let prepared = resolve_discovery_scope(ports, request)?;
            let resolved = if scope::is_broadened(&request.scope) {
                match scope::resolve_entities(ports.tracker, &prepared) {
                    Ok(resolved) => resolved,
                    Err(scope::EntityResolution::Unconfigured(error)) => {
                        return Err(RunError::DiscoveryUnconfigured {
                            detail: error.detail,
                        });
                    }
                    Err(scope::EntityResolution::Transient(error)) => {
                        return Ok((
                            Vec::new(),
                            DiscoveryStatus::Failed {
                                detail: error.into_detail(),
                            },
                        ));
                    }
                }
            } else {
                prepared
            };
            match discover_untracked(
                ports.tracker,
                &resolved,
                request.corpus,
                settled,
            ) {
                Ok(discovered) if !discovered.completeness.is_complete() => {
                    return Err(RunError::DiscoveryIncomplete {
                        found: discovered.ids.len(),
                        completeness: discovered.completeness,
                    });
                }
                Ok(discovered) => {
                    let found = discovered.ids.len();
                    (discovered.ids, DiscoveryStatus::Ran { found })
                }
                Err(TrackerError::Unconfigured { detail }) => {
                    return Err(RunError::DiscoveryUnconfigured { detail });
                }
                Err(error) => (
                    Vec::new(),
                    DiscoveryStatus::Failed {
                        detail: error.into_detail(),
                    },
                ),
            }
        }
    })
}

/// The configuration-only half of discovery's pre-flight: the scope must name
/// a valid target before anything is read or written.
fn resolve_discovery_scope(
    ports: &SyncPorts<'_>,
    request: &SyncRequest<'_>,
) -> Result<SearchScope, RunError> {
    ports
        .tracker
        .resolve_scope(&request.scope)
        .map_err(|error| RunError::DiscoveryUnconfigured {
            detail: error.detail,
        })
}

/// Refuses a run whose discovery is misconfigured, before the identity pass
/// changes anything.
///
/// # Errors
///
/// [`RunError::DiscoveryUnconfigured`] when a run that would search names no
/// valid target.
pub fn preflight(
    ports: &SyncPorts<'_>,
    request: &SyncRequest<'_>,
) -> Result<(), RunError> {
    if request.discovery_search_suppressed()
        || matches!(request.direction, SyncDirection::PushOnly)
    {
        return Ok(());
    }
    resolve_discovery_scope(ports, request).map(|_| ())
}

/// Gathers facts, plans, discovers both create sets, and refuses before any
/// write when the plan's pull or push count exceeds its bound.
///
/// # Errors
///
/// [`RunError::Internal`] for a clock, baseline-store or planning failure;
/// [`RunError::DiscoveryUnconfigured`] when the discovery scope names no valid
/// target; [`RunError::DiscoveryIncomplete`] when the untracked search is cut
/// short; [`RunError::Refused`] when the plan would exceed the write bounds.
fn prepare_run<'a>(
    ports: &SyncPorts<'_>,
    digester: &'a BaselineDigester<'a>,
    baseline: &BaselineStore<'_>,
    request: &SyncRequest<'a>,
    facts: Option<GatheredFacts>,
    settled: &SettledView,
) -> Result<PreparedRun<'a>, RunError> {
    let run_start_epoch =
        ports.clock.run_start_epoch().map_err(RunError::Internal)?;

    let (loaded_baseline, degradation) = baseline
        .load()
        .map_err(|error| RunError::Internal(error.into()))?;

    let facts = facts.unwrap_or_else(|| {
        fetch::gather(
            request.reconciled(),
            &loaded_baseline,
            ports.tracker,
            ports.status,
            request.strategy,
            digester,
        )
    });

    // A capped keyed read leaves the un-read items' remote state unknown for
    // both pull and push, so abort here — before any planning or write.
    let keyed_read_budget_limited = classify_keyed_read(facts.keyed_read)?;

    let knowledge = RunKnowledge::gather(
        facts,
        request.reconciled(),
        &loaded_baseline,
        digester,
    );
    let plan = knowledge.plan(&loaded_baseline, request)?;

    let read_failure = knowledge.facts.read_failure.clone();
    let unsupported_recipe = request
        .reconciled()
        .iter()
        .filter(|item| {
            matches!(
                loaded_baseline.get(&item.id),
                Some(Entry::Unsupported { .. })
            )
        })
        .map(|item| item.id.clone())
        .collect();

    let (untracked, discovery) = untracked_to_import(ports, request, settled)?;
    let creates_from_local =
        unsynced_creates(&plan, request.reconciled(), request.direction);

    // A create-from-remote authors a new local file (pull-direction); a
    // create-from-local issues a new remote issue (push-direction). Each folds
    // into the existing directional bound rather than a third knob.
    let pulls = plan.pull_count() + untracked.len();
    let pushes = plan.push_count() + creates_from_local.len();
    if request.max_pulls.exceeds(pulls) || request.max_pushes.exceeds(pushes) {
        return Err(RunError::Refused {
            pulls,
            pushes,
            max_pulls: request.max_pulls,
            max_pushes: request.max_pushes,
            new_local_files: untracked.len(),
            new_remote_issues: creates_from_local.len(),
        });
    }

    let dossiers = build_dossiers(&plan, &knowledge);

    Ok(PreparedRun {
        run_start_epoch,
        unsupported_recipe,
        degradation,
        read_failure,
        keyed_read_budget_limited,
        knowledge,
        plan,
        untracked,
        discovery,
        creates_from_local,
        dossiers,
    })
}

/// Runs a complete sync: gather, plan, and — under [`RunMode::Apply`] —
/// execute.
///
/// Refuses before any write, in both modes, when the plan's pull or push
/// count exceeds its bound: a preview that reported an over-threshold plan
/// and exited 0 would mispredict exactly the run with the largest blast
/// radius.
///
/// # Errors
///
/// [`RunError::Internal`] for a clock, baseline-store or planning failure;
/// [`RunError::Refused`] when the plan would exceed the write bounds.
pub fn run<'a>(
    ports: &SyncPorts<'a>,
    baseline: &mut BaselineStore<'a>,
    request: &SyncRequest<'_>,
) -> Result<RunReport, RunError> {
    execute(ports, baseline, request, None, &SettledView::default())
}

/// Runs the engine over facts an identity pass already gathered, so the
/// remote is read once per run. The facts carry each item's dirtiness, so
/// `ports.status` is not consulted.
///
/// # Errors
///
/// As [`run`].
pub fn run_with<'a>(
    ports: &SyncPorts<'a>,
    baseline: &mut BaselineStore<'a>,
    request: &SyncRequest<'_>,
    facts: GatheredFacts,
    settled: &SettledView,
) -> Result<RunReport, RunError> {
    execute(ports, baseline, request, Some(facts), settled)
}

#[allow(clippy::too_many_lines)]
fn execute<'a>(
    ports: &SyncPorts<'a>,
    baseline: &mut BaselineStore<'a>,
    request: &SyncRequest<'_>,
    facts: Option<GatheredFacts>,
    settled: &SettledView,
) -> Result<RunReport, RunError> {
    let digester = BaselineDigester::new(ports.tracker, ports.parser);
    let PreparedRun {
        run_start_epoch,
        unsupported_recipe,
        degradation,
        read_failure,
        keyed_read_budget_limited,
        knowledge,
        plan,
        untracked,
        discovery,
        creates_from_local,
        dossiers,
    } = prepare_run(ports, &digester, baseline, request, facts, settled)?;

    let reported_as = |item: ReportedItem| {
        noting_unsupported_recipe(item, &unsupported_recipe)
    };
    if matches!(request.mode, RunMode::Preview) {
        let mut reported: Vec<ReportedItem> =
            validate_pushes(&plan, ports.tracker, &knowledge)
                .into_iter()
                .map(reported_as)
                .collect();
        for external_id in &untracked {
            reported.push(create_report(
                external_id.as_str().to_owned(),
                Action::CreateFromRemote,
                ItemOutcome::NotApplied,
            ));
        }
        for item in &creates_from_local {
            reported.push(create_report(
                item.id.clone(),
                Action::CreateFromLocal,
                ItemOutcome::NotApplied,
            ));
        }
        return Ok(RunReport {
            identity: Vec::new(),
            promotions: Vec::new(),
            deferred: 0,
            reported,
            read_failure,
            baseline_degradation: degradation,
            finalised: false,
            dossiers,
            discovery,
            keyed_read_budget_limited,
        });
    }

    let mut reported = Vec::with_capacity(
        plan.actions.len() + untracked.len() + creates_from_local.len(),
    );
    let mut blank_local_hash: Vec<String> = Vec::new();
    let carries = |candidate: &ExternalId| {
        corpus_carries(request.corpus, settled, candidate)
    };

    {
        let mut applier = ItemApplier::new(
            ports.tracker,
            ports.parser,
            ports.writer,
            baseline,
            run_start_epoch,
        );

        for external_id in &untracked {
            let outcome =
                match applier.create_from_remote(external_id, ports.author) {
                    Ok(_) => ItemOutcome::Applied,
                    Err(error) => ItemOutcome::Failed(error),
                };
            reported.push(create_report(
                external_id.as_str().to_owned(),
                Action::CreateFromRemote,
                outcome,
            ));
        }

        for planned in plan.actions {
            reported.push(reported_as(apply_planned_action(
                &mut applier,
                planned,
                &knowledge,
                &mut blank_local_hash,
            )));
        }

        for item in &creates_from_local {
            reported.push(apply_local_create(
                &mut applier,
                item,
                request,
                ports.author,
                &carries,
                run_start_epoch,
            ));
        }
    }

    let finalised = finalise_baseline(
        baseline,
        &reported,
        &blank_local_hash,
        run_start_epoch,
        &request.selection,
    );

    Ok(RunReport {
        identity: Vec::new(),
        promotions: Vec::new(),
        deferred: 0,
        reported,
        read_failure,
        baseline_degradation: degradation,
        finalised,
        dossiers,
        discovery,
        keyed_read_budget_limited,
    })
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unnecessary_wraps
)]
mod tests {
    use tracker::RemoteTimestamp;
    use work::section_diff::SectionDiff;

    use super::render_dossier;
    use super::ConflictDossier;
    use super::DossierRender;

    fn section(name: &str, local: &str, remote: &str) -> SectionDiff {
        SectionDiff {
            name: name.to_owned(),
            local: local.to_owned(),
            remote: remote.to_owned(),
        }
    }

    fn dossier(
        local_modified: Option<u64>,
        remote_updated: RemoteTimestamp,
        sections: Vec<SectionDiff>,
    ) -> ConflictDossier {
        ConflictDossier {
            id: "0009".to_owned(),
            title: "The item's title".to_owned(),
            remote_title: None,
            local_modified,
            remote_updated,
            sections,
            unrenderable: None,
        }
    }

    fn ok_renderer(section: &SectionDiff) -> String {
        format!("=== {} (- LOCAL / + REMOTE) ===\nbody\n\n", section.name)
    }

    #[test]
    fn classify_keyed_read_routes_each_completeness() {
        use tracker::Completeness;

        assert!(matches!(
            super::classify_keyed_read(Completeness::Complete),
            Ok(false)
        ));
        assert!(
            matches!(
                super::classify_keyed_read(Completeness::Transient),
                Ok(true)
            ),
            "a transient cutoff is a soft signal, not an abort"
        );
        assert!(
            matches!(
                super::classify_keyed_read(Completeness::CapHit),
                Err(super::RunError::KeyedReadCapped)
            ),
            "a cap-hit is the fail-loud abort"
        );
    }

    #[test]
    fn a_known_epoch_renders_as_iso_8601_utc() {
        assert_eq!(
            super::format_epoch_utc(1_700_000_000),
            "2023-11-14T22:13:20Z"
        );
    }

    #[test]
    fn both_absent_stamp_variants_and_absent_mtime_render_as_unavailable() {
        for stamp in [RemoteTimestamp::NotReported, RemoteTimestamp::NotRead] {
            let rendered =
                render_dossier(&dossier(None, stamp, Vec::new()), &ok_renderer);
            let DossierRender::Renderable(text) = rendered else {
                panic!("a section-free dossier renders");
            };
            assert!(text.contains("local modified: (unavailable)"), "{text}");
            assert!(text.contains("remote updated: (unavailable)"), "{text}");
            assert!(text.contains("status: renderable"), "{text}");
        }
    }

    #[test]
    fn a_reported_stamp_and_known_mtime_render_as_timestamps() {
        let rendered = render_dossier(
            &dossier(
                Some(1_700_000_000),
                RemoteTimestamp::Reported("2026-07-01T00:00:00Z".to_owned()),
                vec![section("(preamble)", "local body", "remote body")],
            ),
            &ok_renderer,
        );
        let DossierRender::Renderable(text) = rendered else {
            panic!("a section-free dossier renders");
        };
        assert!(
            text.contains("local modified: 2023-11-14T22:13:20Z"),
            "{text}"
        );
        assert!(
            text.contains("remote updated: 2026-07-01T00:00:00Z"),
            "{text}"
        );
        let (header, body) = text
            .split_once("=== (preamble) (- LOCAL / + REMOTE) ===")
            .expect("the section body lands under the header");
        assert!(header.contains("status: renderable"), "{text}");
        assert!(body.contains("body"), "{text}");
    }

    #[test]
    fn a_local_unreadable_dossier_is_unrenderable_without_rendering() {
        let must_not_render = |_: &SectionDiff| -> String {
            panic!("the renderer must not run for an unreadable local file")
        };
        let mut unreadable =
            dossier(None, RemoteTimestamp::NotRead, Vec::new());
        unreadable.unrenderable =
            Some(super::UnrenderableReason::LocalUnreadable);

        let rendered = render_dossier(&unreadable, &must_not_render);
        let DossierRender::Unrenderable(text) = rendered else {
            panic!("an unreadable local file downgrades to unrenderable");
        };
        assert!(text.contains("status: unrenderable"), "{text}");
        assert!(text.contains("reason: local-unreadable"), "{text}");
        assert!(text.contains("left unresolved"), "{text}");
    }

    #[test]
    fn a_send_from_an_unreadable_file_fails_as_the_read_it_needs() {
        let missing = tempfile::tempdir().expect("a temporary directory");
        let item = super::LocalItem {
            id: "0009".to_owned(),
            path: missing.path().join("0009-gone.md"),
            external_id: None,
        };
        let planned = work::sync::PlannedAction {
            id: "0009".to_owned(),
            state: work::sync::SyncState::LocallyModified,
            action: work::sync::Action::Push(work::sync::FieldDelivery::Send),
        };
        let facts = super::GatheredFacts {
            per_id: std::collections::BTreeMap::new(),
            identity: std::collections::BTreeMap::new(),
            read_failure: None,
            keyed_read: tracker::Completeness::Complete,
        };

        let digester = super::BaselineDigester::new(
            &tracker::PlainDialect,
            &corpus_adapters::CommonMarkBody,
        );
        let knowledge = super::RunKnowledge::gather(
            facts,
            std::slice::from_ref(&item),
            &crate::sync::baseline::Baseline::read(None).0,
            &digester,
        );

        let prepared = super::prepare_send(&planned, &item, &knowledge);

        assert!(
            matches!(
                prepared,
                Err(super::ApplyError::Io {
                    operation: "read-local",
                    ..
                })
            ),
            "the send fails as the run's own read does"
        );
    }

    #[test]
    fn an_unrenderable_remote_dossier_names_its_cause_in_the_header() {
        let mut unrenderable =
            dossier(None, RemoteTimestamp::NotRead, Vec::new());
        unrenderable.unrenderable =
            Some(super::UnrenderableReason::RemoteUnrenderable(
                tracker::Unrenderable {
                    detail: "a panel has no Markdown form".to_owned(),
                },
            ));

        let DossierRender::Unrenderable(text) =
            render_dossier(&unrenderable, &ok_renderer)
        else {
            panic!("an unrenderable remote downgrades to unrenderable");
        };
        assert!(
            text.contains(
                "reason: remote-unrenderable\n\
                 cause: a panel has no Markdown form\n"
            ),
            "{text}"
        );
    }
}

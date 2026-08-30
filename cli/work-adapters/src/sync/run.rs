//! The whole-corpus sync run: plan-then-apply over the gathered facts.

use std::collections::BTreeMap;
use std::path::Path;

use corpus::store::AtomicWrite;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracker::ExternalId;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker::TrackerError;
use work::section_diff::differing_sections;
use work::section_diff::SectionDiff;
use work::sync::plan as compute_plan;
use work::sync::Action;
use work::sync::PlannedAction;
use work::sync::Resolution;
use work::sync::RunClock;
use work::sync::SyncDirection;
use work::sync::SyncPlan;
use work::sync::SyncState;

use crate::sync::apply::ApplyError;
use crate::sync::apply::CreateFromLocalRequest;
use crate::sync::apply::ItemApplier;
use crate::sync::apply::PullRequest;
use crate::sync::apply::PushRequest;
use crate::sync::baseline::Degradation;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::create::canonical_external_key;
use crate::sync::create::LocalAuthor;
use crate::sync::digest::LazyItemDigests;
use crate::sync::fetch;
use crate::sync::fetch::GatheredFacts;
use crate::sync::fetch::GatheredRemote;
use crate::sync::fetch::LocalItem;
use crate::sync::fetch::RetrievalStrategy;
use crate::sync::fetch::WorkingCopyStatus;

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
        max_pulls: usize,
        max_pushes: usize,
        new_local_files: usize,
        new_remote_issues: usize,
    },
    /// A discovery query was cut short (`complete == false`). Refused rather
    /// than acted on: an incomplete untracked set is a lower bound, and the
    /// remedy is to scope the search, not to raise a limit.
    DiscoveryIncomplete {
        found: usize,
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
    pub status: &'a dyn WorkingCopyStatus,
    pub writer: &'a dyn AtomicWrite,
    pub clock: &'a dyn RunClock,
    /// Authors the local files the two create paths need: a work-item file
    /// for a discovered remote issue, and the `external_id` write-back for an
    /// unsynced local draft. Behind a port because both need config, an id
    /// scheme, and a frontmatter renderer that live in the binary layer.
    pub author: &'a dyn LocalAuthor,
}

pub struct SyncRequest<'a> {
    pub items: &'a [LocalItem],
    pub direction: SyncDirection,
    pub strategy: RetrievalStrategy,
    pub resolutions: &'a BTreeMap<String, Resolution>,
    pub max_pulls: usize,
    pub max_pushes: usize,
    pub mode: RunMode,
    /// Where the pending-push markers for unsynced-local creates live.
    pub integrations_root: &'a Path,
    /// The active tracker's name, naming the marker directory alongside the
    /// baseline.
    pub integration: &'a str,
    /// The scope untracked-remote discovery searches. Team/project-scoped by
    /// default so the untracked set stays bounded on a shared workspace.
    pub scope: SearchScope,
}

pub enum ItemOutcome {
    Applied,
    NotApplied,
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

pub struct RunReport {
    pub reported: Vec<ReportedItem>,
    pub read_failure: Option<TrackerError>,
    pub baseline_degradation: Degradation,
    pub finalised: bool,
    pub dossiers: Vec<ConflictDossier>,
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

fn local_title_and_body(content: &str) -> (String, String) {
    let (frontmatter, body) =
        crate::sync::digest::split_frontmatter_and_body(content)
            .unwrap_or_default();
    let title =
        work::show::read_field_raw(&frontmatter, "title").unwrap_or_default();
    (title, body)
}

fn reconstruct_pulled_content(
    local_content: &str,
    remote_body: &str,
) -> String {
    let (frontmatter, _) =
        crate::sync::digest::split_frontmatter_and_body(local_content)
            .unwrap_or_default();
    format!("---\n{frontmatter}\n---\n\n{remote_body}")
}

/// The six fields a user needs to choose a side of one conflict, gathered
/// from the run's own facts. Structured: rendering the sections to text is a
/// separate, injectable step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictDossier {
    pub id: String,
    pub title: String,
    pub local_modified: Option<u64>,
    pub remote_updated: RemoteTimestamp,
    pub sections: Vec<SectionDiff>,
    pub local_unreadable: bool,
}

/// A dossier rendered to text, tagged by whether the local file could be
/// read and its sections shown.
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

fn dossier_header(dossier: &ConflictDossier, status: &str) -> String {
    format!(
        "# Conflict: {}\nstatus: {}\ntitle: {}\nlocal modified: {}\nremote \
         updated: {}\n\n",
        dossier.id,
        status,
        dossier.title,
        render_local_modified(dossier.local_modified),
        render_stamp(&dossier.remote_updated),
    )
}

fn renderable_header(dossier: &ConflictDossier) -> String {
    dossier_header(dossier, "renderable")
}

fn unrenderable_header(dossier: &ConflictDossier) -> String {
    format!(
        "{}This conflict could not be rendered: the local file could not be \
         read. Item {} was left unresolved. Fix the file and re-run the sync, \
         or edit the work item by hand.\n\n",
        dossier_header(dossier, "unrenderable"),
        dossier.id,
    )
}

/// Renders a dossier to text.
///
/// An infallible header, then each section through the injected `render`. An
/// unreadable local file is the only downgrade to
/// [`DossierRender::Unrenderable`].
pub fn render_dossier(
    dossier: &ConflictDossier,
    render: &dyn Fn(&SectionDiff) -> String,
) -> DossierRender {
    if dossier.local_unreadable {
        return DossierRender::Unrenderable(unrenderable_header(dossier));
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

/// An id-keyed view over the run's local items, built once and shared by the
/// dossier pass and the apply loop.
struct ItemIndex<'a> {
    by_id: BTreeMap<&'a str, &'a LocalItem>,
}

impl<'a> ItemIndex<'a> {
    fn build(items: &'a [LocalItem]) -> Self {
        Self {
            by_id: items.iter().map(|item| (item.id.as_str(), item)).collect(),
        }
    }

    fn get(&self, id: &str) -> Option<&'a LocalItem> {
        self.by_id.get(id).copied()
    }
}

fn build_dossiers(
    plan: &SyncPlan,
    items: &ItemIndex<'_>,
    facts: &GatheredFacts,
) -> Vec<ConflictDossier> {
    plan.actions
        .iter()
        .filter(|planned| planned.action == Action::Prompt)
        .filter_map(|planned| {
            let item = items.get(&planned.id)?;
            Some(dossier_for(&planned.id, item, facts))
        })
        .collect()
}

fn dossier_for(
    id: &str,
    item: &LocalItem,
    facts: &GatheredFacts,
) -> ConflictDossier {
    let remote = facts.per_id.get(id).map(|(remote, _)| remote);
    let remote_updated =
        remote.map_or(RemoteTimestamp::NotRead, |r| r.remote_updated.clone());

    match std::fs::read_to_string(&item.path) {
        Err(_) => unreadable_dossier(id, remote_updated),
        Ok(local_content) => {
            readable_dossier(id, item, remote, remote_updated, &local_content)
        }
    }
}

fn unreadable_dossier(
    id: &str,
    remote_updated: RemoteTimestamp,
) -> ConflictDossier {
    ConflictDossier {
        id: id.to_owned(),
        title: String::new(),
        local_modified: None,
        remote_updated,
        sections: Vec::new(),
        local_unreadable: true,
    }
}

fn readable_dossier(
    id: &str,
    item: &LocalItem,
    remote: Option<&GatheredRemote>,
    remote_updated: RemoteTimestamp,
    local_content: &str,
) -> ConflictDossier {
    let (title, _) = local_title_and_body(local_content);
    let remote_body = remote.and_then(|r| r.body.as_deref()).unwrap_or("");
    let reconstructed = reconstruct_pulled_content(local_content, remote_body);
    ConflictDossier {
        id: id.to_owned(),
        title,
        local_modified: file_mtime_secs(&item.path),
        remote_updated,
        sections: differing_sections(local_content, &reconstructed),
        local_unreadable: false,
    }
}

/// Reports the whole plan for a preview, attaching a local payload-validation
/// outcome to each `Push` entry.
///
/// Every planned action is reported `NotApplied`, so the report never shrinks
/// below the plan. The `validate_update` call is a local composition check —
/// it makes no remote call — so a `Rejected` outcome names a locally-detectable
/// missing field without reproducing the tracker's own field validation.
fn validate_pushes(
    plan: &work::sync::SyncPlan,
    items: &[LocalItem],
    tracker: &dyn RemoteTracker,
) -> Vec<ReportedItem> {
    plan.actions
        .iter()
        .map(|planned| {
            let validation = (planned.action == Action::Push)
                .then(|| validate_push(planned, items, tracker))
                .flatten();
            ReportedItem {
                planned: planned.clone(),
                outcome: ItemOutcome::NotApplied,
                validation,
            }
        })
        .collect()
}

fn validate_push(
    planned: &PlannedAction,
    items: &[LocalItem],
    tracker: &dyn RemoteTracker,
) -> Option<tracker::ValidationOutcome> {
    let item = items.iter().find(|candidate| candidate.id == planned.id)?;
    let external_id = item.external_id.as_ref()?;
    let content = std::fs::read_to_string(&item.path).ok()?;
    let (title, body) = local_title_and_body(&content);
    Some(tracker.validate_update(external_id, &title, &body))
}

struct Discovered {
    ids: Vec<ExternalId>,
    complete: bool,
}

/// The untracked remote issues: a `search` over `scope` minus the
/// canonicalised set of local `external_id`s. A stored id differing from a
/// search result only cosmetically folds equal and is excluded.
fn discover_untracked(
    tracker: &dyn RemoteTracker,
    scope: &SearchScope,
    items: &[LocalItem],
) -> Result<Discovered, TrackerError> {
    let discovery = tracker.search(scope)?;
    let local: std::collections::BTreeSet<String> = items
        .iter()
        .filter_map(|item| item.external_id.as_ref())
        .map(canonical_external_key)
        .collect();
    let ids = discovery
        .found
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| !local.contains(&canonical_external_key(id)))
        .collect();
    Ok(Discovered {
        ids,
        complete: discovery.complete,
    })
}

/// The unsynced local drafts eligible for create-from-local: state `Unsynced`
/// (no `external_id`), under a push-capable direction.
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
        .collect()
}

/// The `(title, body, kind)` a create-from-local needs, read from the draft's
/// own frontmatter and body. `None` when the file is unreadable or malformed.
fn create_inputs(path: &Path) -> Option<(String, String, String)> {
    let content = std::fs::read_to_string(path).ok()?;
    let (frontmatter, body) =
        crate::sync::digest::split_frontmatter_and_body(&content).ok()?;
    let title =
        work::show::read_field_raw(&frontmatter, "title").unwrap_or_default();
    let kind =
        work::show::read_field_raw(&frontmatter, "kind").unwrap_or_default();
    Some((title, body, kind))
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

fn push_outcome(
    applier: &mut ItemApplier<'_, '_>,
    item: &LocalItem,
    external_id: &ExternalId,
) -> ItemOutcome {
    let content = match std::fs::read_to_string(&item.path) {
        Ok(content) => content,
        Err(error) => {
            return ItemOutcome::Failed(ApplyError::Io {
                item_id: item.id.clone(),
                operation: "read-local",
                detail: error.to_string(),
            });
        }
    };
    let (title, body) = local_title_and_body(&content);
    match applier.push(&PushRequest {
        id: &item.id,
        external_id,
        title: &title,
        body: &body,
        file_path: &item.path,
    }) {
        Ok(()) => ItemOutcome::Applied,
        Err(error) => ItemOutcome::Failed(error),
    }
}

fn pull_outcome(
    applier: &mut ItemApplier<'_, '_>,
    item: &LocalItem,
    facts: &GatheredFacts,
) -> ItemOutcome {
    let remote = facts.per_id.get(&item.id).map(|(remote, _)| remote);
    let projected_body = remote.and_then(|r| r.body.as_deref()).unwrap_or("");
    let local_content = std::fs::read_to_string(&item.path).unwrap_or_default();
    let content = reconstruct_pulled_content(&local_content, projected_body);
    let remote_updated =
        remote.map_or(RemoteTimestamp::NotRead, |r| r.remote_updated.clone());
    match applier.pull(&PullRequest {
        id: &item.id,
        file_path: &item.path,
        content: &content,
        projected_body,
        remote_updated,
    }) {
        Ok(()) => ItemOutcome::Applied,
        Err(error) => ItemOutcome::Failed(error),
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
/// under any skip action records its id for baseline blanking. The two create
/// actions are unreachable: both create paths run out-of-band of this loop.
fn apply_planned_action(
    applier: &mut ItemApplier<'_, '_>,
    planned: PlannedAction,
    index: &ItemIndex<'_>,
    facts: &GatheredFacts,
    blank_local_hash: &mut Vec<String>,
) -> ReportedItem {
    let item = index.get(&planned.id);
    let outcome = match planned.action {
        Action::Push => {
            match item
                .and_then(|item| item.external_id.as_ref().map(|id| (item, id)))
            {
                Some((item, external_id)) => {
                    push_outcome(applier, item, external_id)
                }
                None => return not_applied(planned),
            }
        }
        Action::Pull => match item {
            Some(item) => pull_outcome(applier, item, facts),
            None => return not_applied(planned),
        },
        Action::Prompt | Action::SkipConflict | Action::SkipDirty => {
            if matches!(planned.state, SyncState::Conflict) {
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
    degradation: Degradation,
    read_failure: Option<TrackerError>,
    facts: GatheredFacts,
    plan: SyncPlan,
    untracked: Vec<ExternalId>,
    creates_from_local: Vec<&'a LocalItem>,
    index: ItemIndex<'a>,
    dossiers: Vec<ConflictDossier>,
}

/// Gathers facts, plans, discovers both create sets, and refuses before any
/// write when the plan's pull or push count exceeds its bound.
///
/// # Errors
///
/// [`RunError::Internal`] for a clock, baseline-store or planning failure;
/// [`RunError::DiscoveryIncomplete`] when the untracked search is cut short;
/// [`RunError::Refused`] when the plan would exceed the write bounds.
fn prepare_run<'a>(
    ports: &SyncPorts<'_>,
    baseline: &BaselineStore<'_>,
    request: &SyncRequest<'a>,
) -> Result<PreparedRun<'a>, RunError> {
    let run_start_epoch =
        ports.clock.run_start_epoch().map_err(RunError::Internal)?;

    let (loaded_baseline, degradation) = baseline
        .load()
        .map_err(|error| RunError::Internal(error.into()))?;

    let facts = fetch::gather(
        request.items,
        &loaded_baseline,
        ports.tracker,
        ports.status,
        request.strategy,
    );

    let digests: Vec<LazyItemDigests<'_>> = request
        .items
        .iter()
        .map(|item| {
            let remote_content = facts
                .per_id
                .get(&item.id)
                .and_then(|(remote, _)| remote.body.clone());
            LazyItemDigests::new(&item.path, remote_content)
        })
        .collect();

    let plan_inputs = facts.plan_inputs(
        request.items,
        &digests,
        &loaded_baseline,
        loaded_baseline.timestamp(),
    );

    let plan =
        compute_plan(&plan_inputs, request.direction, request.resolutions)
            .map_err(RunError::Internal)?;

    let mut read_failure = facts.read_failure.clone();

    // Untracked-remote discovery and unsynced-local creates are computed from
    // reads only, so the combined gate below can bound every write — planned
    // pulls, planned pushes, and both create paths — before a single one runs.
    let discovery_enabled =
        !matches!(request.direction, SyncDirection::PushOnly)
            && (request.scope.project.is_some() || request.scope.all_projects);
    let untracked = if discovery_enabled {
        match discover_untracked(ports.tracker, &request.scope, request.items) {
            Ok(discovered) if !discovered.complete => {
                return Err(RunError::DiscoveryIncomplete {
                    found: discovered.ids.len(),
                });
            }
            Ok(discovered) => discovered.ids,
            Err(error) => {
                read_failure = read_failure.or(Some(error));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    let creates_from_local =
        unsynced_creates(&plan, request.items, request.direction);

    // A create-from-remote authors a new local file (pull-direction); a
    // create-from-local issues a new remote issue (push-direction). Each folds
    // into the existing directional bound rather than a third knob.
    let pulls = plan.pull_count() + untracked.len();
    let pushes = plan.push_count() + creates_from_local.len();
    if pulls > request.max_pulls || pushes > request.max_pushes {
        return Err(RunError::Refused {
            pulls,
            pushes,
            max_pulls: request.max_pulls,
            max_pushes: request.max_pushes,
            new_local_files: untracked.len(),
            new_remote_issues: creates_from_local.len(),
        });
    }

    let index = ItemIndex::build(request.items);
    let dossiers = build_dossiers(&plan, &index, &facts);

    Ok(PreparedRun {
        run_start_epoch,
        degradation,
        read_failure,
        facts,
        plan,
        untracked,
        creates_from_local,
        index,
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
    let PreparedRun {
        run_start_epoch,
        degradation,
        read_failure,
        facts,
        plan,
        untracked,
        creates_from_local,
        index,
        dossiers,
    } = prepare_run(ports, baseline, request)?;

    if matches!(request.mode, RunMode::Preview) {
        let mut reported = validate_pushes(&plan, request.items, ports.tracker);
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
            reported,
            read_failure,
            baseline_degradation: degradation,
            finalised: false,
            dossiers,
        });
    }

    let mut reported = Vec::with_capacity(
        plan.actions.len() + untracked.len() + creates_from_local.len(),
    );
    let mut blank_local_hash: Vec<String> = Vec::new();
    let corpus_carries = |candidate: &ExternalId| {
        request
            .items
            .iter()
            .any(|item| item.external_id.as_ref() == Some(candidate))
    };

    {
        let mut applier =
            ItemApplier::new(ports.tracker, ports.writer, baseline);

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
            reported.push(apply_planned_action(
                &mut applier,
                planned,
                &index,
                &facts,
                &mut blank_local_hash,
            ));
        }

        for item in &creates_from_local {
            reported.push(apply_local_create(
                &mut applier,
                item,
                request,
                ports.author,
                &corpus_carries,
                run_start_epoch,
            ));
        }
    }

    let blank_refs: Vec<&str> =
        blank_local_hash.iter().map(String::as_str).collect();
    let finalised = baseline.finalise_run(&blank_refs, run_start_epoch).is_ok();

    Ok(RunReport {
        reported,
        read_failure,
        baseline_degradation: degradation,
        finalised,
        dossiers,
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
            local_modified,
            remote_updated,
            sections,
            local_unreadable: false,
        }
    }

    fn ok_renderer(section: &SectionDiff) -> String {
        format!("=== {} (- LOCAL / + REMOTE) ===\nbody\n\n", section.name)
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
        unreadable.local_unreadable = true;

        let rendered = render_dossier(&unreadable, &must_not_render);
        let DossierRender::Unrenderable(text) = rendered else {
            panic!("an unreadable local file downgrades to unrenderable");
        };
        assert!(text.contains("status: unrenderable"), "{text}");
        assert!(text.contains("left unresolved"), "{text}");
    }
}

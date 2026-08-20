//! The whole-corpus sync run: plan-then-apply over the gathered facts.

use std::collections::BTreeMap;

use corpus::store::AtomicWrite;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
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

use crate::diff_shellout::DiffUnavailable;
use crate::sync::apply::ApplyError;
use crate::sync::apply::ItemApplier;
use crate::sync::apply::PullRequest;
use crate::sync::apply::PushRequest;
use crate::sync::baseline::Degradation;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::digest::LazyItemDigests;
use crate::sync::fetch;
use crate::sync::fetch::GatheredFacts;
use crate::sync::fetch::GatheredRemote;
use crate::sync::fetch::LocalItem;
use crate::sync::fetch::RetrievalStrategy;
use crate::sync::fetch::WorkingCopyStatus;

pub enum RunError {
    Refused {
        pulls: usize,
        pushes: usize,
        max_pulls: usize,
        max_pushes: usize,
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
}

pub struct SyncRequest<'a> {
    pub items: &'a [LocalItem],
    pub direction: SyncDirection,
    pub strategy: RetrievalStrategy,
    pub resolutions: &'a BTreeMap<String, Resolution>,
    pub max_pulls: usize,
    pub max_pushes: usize,
    pub mode: RunMode,
}

pub enum ItemOutcome {
    Applied,
    NotApplied,
    Failed(ApplyError),
}

pub struct ReportedItem {
    pub planned: PlannedAction,
    pub outcome: ItemOutcome,
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
/// from the run's own facts. Structured, subprocess-free: rendering to text
/// through `diff -u` is a separate, injectable step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictDossier {
    pub id: String,
    pub title: String,
    pub local_modified: Option<u64>,
    pub remote_updated: RemoteTimestamp,
    pub sections: Vec<SectionDiff>,
    pub local_unreadable: bool,
}

/// A dossier rendered to text, tagged by whether every section could be
/// shown through `diff -u`.
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
        "{}This conflict could not be rendered (the `diff` tool was \
         unavailable). Item {} was left unresolved. Install `diff` and re-run \
         the sync, or edit the work item by hand.\n\n",
        dossier_header(dossier, "unrenderable"),
        dossier.id,
    )
}

fn raw_sections(dossier: &ConflictDossier) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    for section in &dossier.sections {
        let _ = writeln!(out, "=== {} (- LOCAL / + REMOTE) ===", section.name);
        for line in section.local.lines() {
            out.push_str("- ");
            out.push_str(line);
            out.push('\n');
        }
        for line in section.remote.lines() {
            out.push_str("+ ");
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// Renders a dossier to text.
///
/// An infallible header, then each section through the injected `render`. Any
/// failing section downgrades the whole item to
/// [`DossierRender::Unrenderable`], whose text still lists the section names
/// and raw values so the conflict is never announced with nothing to see.
pub fn render_dossier(
    dossier: &ConflictDossier,
    render: &dyn Fn(&SectionDiff) -> Result<String, DiffUnavailable>,
) -> DossierRender {
    if dossier.local_unreadable {
        return DossierRender::Unrenderable(unrenderable_header(dossier));
    }
    let mut sections = String::new();
    for section in &dossier.sections {
        match render(section) {
            Ok(body) => sections.push_str(&body),
            Err(DiffUnavailable) => {
                return DossierRender::Unrenderable(
                    unrenderable_header(dossier) + &raw_sections(dossier),
                );
            }
        }
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
#[allow(clippy::too_many_lines)]
pub fn run<'a>(
    ports: &SyncPorts<'a>,
    baseline: &mut BaselineStore<'a>,
    request: &SyncRequest<'_>,
) -> Result<RunReport, RunError> {
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

    let pulls = plan.pull_count();
    let pushes = plan.push_count();
    if pulls > request.max_pulls || pushes > request.max_pushes {
        return Err(RunError::Refused {
            pulls,
            pushes,
            max_pulls: request.max_pulls,
            max_pushes: request.max_pushes,
        });
    }

    let index = ItemIndex::build(request.items);
    let dossiers = build_dossiers(&plan, &index, &facts);

    if matches!(request.mode, RunMode::Preview) {
        return Ok(RunReport {
            reported: plan
                .actions
                .into_iter()
                .map(|planned| ReportedItem {
                    planned,
                    outcome: ItemOutcome::NotApplied,
                })
                .collect(),
            read_failure: facts.read_failure,
            baseline_degradation: degradation,
            finalised: false,
            dossiers,
        });
    }

    let mut reported = Vec::with_capacity(plan.actions.len());
    let mut blank_local_hash: Vec<String> = Vec::new();

    {
        let mut applier =
            ItemApplier::new(ports.tracker, ports.writer, baseline);
        for planned in plan.actions {
            let item = index.get(&planned.id);
            let outcome = match planned.action {
                Action::Push => {
                    let Some(item) = item else {
                        reported.push(ReportedItem {
                            planned,
                            outcome: ItemOutcome::NotApplied,
                        });
                        continue;
                    };
                    let Some(external_id) = &item.external_id else {
                        reported.push(ReportedItem {
                            planned,
                            outcome: ItemOutcome::NotApplied,
                        });
                        continue;
                    };
                    match std::fs::read_to_string(&item.path) {
                        Ok(content) => {
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
                        Err(error) => ItemOutcome::Failed(ApplyError::Io {
                            item_id: item.id.clone(),
                            operation: "read-local",
                            detail: error.to_string(),
                        }),
                    }
                }
                Action::Pull => {
                    let Some(item) = item else {
                        reported.push(ReportedItem {
                            planned,
                            outcome: ItemOutcome::NotApplied,
                        });
                        continue;
                    };
                    let remote = facts.per_id.get(&item.id).map(|(r, _)| r);
                    let projected_body =
                        remote.and_then(|r| r.body.as_deref()).unwrap_or("");
                    let local_content =
                        std::fs::read_to_string(&item.path).unwrap_or_default();
                    let content = reconstruct_pulled_content(
                        &local_content,
                        projected_body,
                    );
                    let remote_updated = remote
                        .map_or(tracker::RemoteTimestamp::NotRead, |r| {
                            r.remote_updated.clone()
                        });
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
                Action::Prompt | Action::SkipConflict | Action::SkipDirty => {
                    if matches!(planned.state, SyncState::Conflict) {
                        blank_local_hash.push(planned.id.clone());
                    }
                    ItemOutcome::NotApplied
                }
                Action::Noop => ItemOutcome::NotApplied,
            };
            reported.push(ReportedItem { planned, outcome });
        }
    }

    let blank_refs: Vec<&str> =
        blank_local_hash.iter().map(String::as_str).collect();
    let finalised = baseline.finalise_run(&blank_refs, run_start_epoch).is_ok();

    Ok(RunReport {
        reported,
        read_failure: facts.read_failure,
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
    use super::DiffUnavailable;
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

    fn ok_renderer(section: &SectionDiff) -> Result<String, DiffUnavailable> {
        Ok(format!(
            "=== {} (- LOCAL / + REMOTE) ===\nbody\n\n",
            section.name
        ))
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
                Vec::new(),
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
    }

    #[test]
    fn a_failing_renderer_downgrades_to_unrenderable_with_raw_values() {
        let failing = |_: &SectionDiff| -> Result<String, DiffUnavailable> {
            Err(DiffUnavailable)
        };
        let rendered = render_dossier(
            &dossier(
                None,
                RemoteTimestamp::NotRead,
                vec![section("(preamble)", "local body", "remote body")],
            ),
            &failing,
        );
        let DossierRender::Unrenderable(text) = rendered else {
            panic!("a failing renderer downgrades to unrenderable");
        };
        assert!(text.contains("status: unrenderable"), "{text}");
        assert!(
            text.contains("=== (preamble) (- LOCAL / + REMOTE) ==="),
            "{text}"
        );
        assert!(text.contains("- local body"), "{text}");
        assert!(text.contains("+ remote body"), "{text}");
    }

    #[test]
    fn a_local_unreadable_dossier_is_unrenderable_without_rendering() {
        let must_not_render =
            |_: &SectionDiff| -> Result<String, DiffUnavailable> {
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

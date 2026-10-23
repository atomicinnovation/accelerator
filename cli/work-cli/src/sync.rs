//! `accelerator work sync`: drives the remote sync engine end to end.

mod console;
mod recovery_labels;
mod stores;
mod work_files;

use stores::StoreFactories;
use stores::SyncStores;
use work_files::discover_corpus;
use work_files::ConfiguredNames;
use work_files::VcsConflicts;

use corpus_adapters::CommonMarkBody;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use work::sync::PendingPush;
use work_adapters::sync::pending_push::Marker;

use ::config::catalogue::TRACKERS;
use ::config::consent::ProvenanceContext;
use ::config::provenance::path_source;
use ::config::tracker_block::read_block;
use ::config::tracker_block::BlockName;
use ::config::ConfigAccess;
use ::config::Key;
use config_adapters::credentials::SystemEnvironment;
use config_adapters::credentials::PERSONAL_CONFIG_RELATIVE;
use config_adapters::TrackedConfigFile;
use corpus::WorkItemIdScheme;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::FileRecoveryCopies;
use tracker::ExternalId;
use vcs_adapters::library::InProcessProbe;
use vcs_adapters::InProcessTracking;
use work::draft_id::is_token_safe;
use work::identity::IdentityField;
use work::identity_run::IdentityRunGuard;
use work::retirement::RECOVERY_PARENT;
use work::retirement::RETIREMENT_INCOMPLETE;
use work::section_diff::SectionDiff;
use work::sync::NotATrackerKey;
use work::sync::PendingPushOrder;
use work::sync::PlannedAction;
use work::sync::Resolution;
use work::sync::RunClock;
use work::sync::SyncDirection;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::promotion_records::PromotionRecords;
use work_adapters::retirement::sweep_recoveries;
use work_adapters::retirement::RecoveryNotice;
use work_adapters::sync::baseline;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::create::canonical_external_key;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::files::SyncFiles;
use work_adapters::sync::files::SyncStore;
use work_adapters::sync::identity_settlement::IdentityOutcome;
use work_adapters::sync::identity_settlement::IdentityRow;
use work_adapters::sync::identity_settlement::SettlementPorts;
use work_adapters::sync::kept_edits::KeptCopy;
use work_adapters::sync::kept_edits::KeptEdits;
use work_adapters::sync::kept_edits::KeptEditsRetention;
use work_adapters::sync::kept_edits::KeptEditsReview;
use work_adapters::sync::kept_edits::Provenance;
use work_adapters::sync::report::CorpusScope;
use work_adapters::sync::report::CreateNotice;
use work_adapters::sync::report::FailureSeverity;
use work_adapters::sync::run::render_dossier;
use work_adapters::sync::run::ConflictDossier;
use work_adapters::sync::run::DiscoveryStatus;
use work_adapters::sync::run::DossierRender;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::ItemSelection;
use work_adapters::sync::run::RunError;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;
use work_adapters::sync::settled_run::run_settled;
use work_adapters::sync::settled_run::CorpusDiscovery;
use work_adapters::sync::settled_run::DiscoveredCorpus;
use work_adapters::sync::settled_run::SettledRunFailure;
use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

use crate::bounded_roots::configure_failure;
use crate::bounded_roots::ConfiguredRoot;
use crate::bounded_roots::RootKey;
use crate::bounded_roots::ValidatedRoots;
use crate::cli::SyncArgs;
use crate::exit_codes;
use crate::finaliser::FinishedRun;
use crate::finaliser::RunFinaliser;
use crate::identity_workspace::IdentityWorkspace;
use crate::promotion_report::detail_lines;
use crate::promotion_report::promotion_exit;
use crate::promotion_report::promotion_line;
use crate::report_records::failure_line;
use crate::report_records::single_line;
use crate::report_records::TsvRecord;
use crate::resolve::IdentityCandidate;
use crate::resolve::RunOutcome;
use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;
pub use console::Console;

struct SystemClock;

impl RunClock for SystemClock {
    fn run_start_epoch(&self) -> Result<u64, kernel::Error> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .map_err(|error| kernel::Error::Failed(error.to_string()))
    }
}

pub fn integrations_dir(
    config: &dyn ConfigAccess,
    root: &Path,
) -> Result<PathBuf, kernel::Error> {
    let relative =
        crate::config::effective_nonempty(config, "paths.integrations")?;
    let path = Path::new(&relative);
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    })
}

/// The work and integrations directories as configured, each with who
/// configured it, which decides whether a root outside the repository may
/// be written unattended.
fn configured_roots(
    config: &dyn ConfigAccess,
    repo_root: &Path,
) -> Result<(ConfiguredRoot, ConfiguredRoot), String> {
    let provenance = ProvenanceContext {
        config,
        tracking: &TrackedConfigFile(InProcessTracking),
        environment: &SystemEnvironment,
        personal_config: repo_root.join(PERSONAL_CONFIG_RELATIVE),
    };
    let configured = |key: RootKey, path: PathBuf| {
        Key::parse(key.config_key())
            .and_then(|parsed| path_source(&provenance, &parsed))
            .map(|source| ConfiguredRoot { key, path, source })
            .map_err(|error| error.to_string())
    };
    let work = crate::config::resolve_work_dir(config, repo_root)
        .map_err(|error| error.to_string())?;
    let integrations = integrations_dir(config, repo_root)
        .map_err(|error| error.to_string())?;
    Ok((
        configured(RootKey::Work, work)?,
        configured(RootKey::Integrations, integrations)?,
    ))
}

fn warn_outstanding_pushes(
    console: &mut Console<'_>,
    integrations_root: &Path,
    integration: &str,
) {
    let Ok(markers) = work_adapters::sync::pending_push::outstanding(
        integrations_root,
        integration,
    ) else {
        return;
    };
    for entry in markers {
        let (path, marker) = match entry {
            Ok(readable) => readable,
            Err(unreadable) => {
                console.warn(format_args!(
                    "warning: {} could not be read ({}); a create or \
                     promotion it recorded may have partially applied",
                    single_line(&unreadable.path.display().to_string()),
                    single_line(&unreadable.detail)
                ));
                continue;
            }
        };
        let (request, external_id) = match &marker {
            Marker::Legacy(PendingPush::Attempted { request }) => {
                (request, None)
            }
            Marker::Legacy(PendingPush::Created {
                request,
                external_id,
            }) => (request, Some(external_id.as_str())),
            Marker::Promotion(record) => (
                &record.request,
                record
                    .stage
                    .key()
                    .map(work::tracker_key::TrackerKey::as_str),
            ),
        };
        console.warn(format_args!(
            "warning: {} names a pending push for '{}' attempted at {}{}{}",
            single_line(&path.display().to_string()),
            single_line(&request.title),
            single_line(&request.attempted_at.to_string()),
            request
                .failure
                .as_deref()
                .map(|detail| format!(", failure: {}", single_line(detail)))
                .unwrap_or_default(),
            external_id
                .map(|id| format!(", external_id: {}", single_line(id)))
                .unwrap_or_default()
        ));
    }
}

/// Where retirement records and recovery copies live, relative to the
/// repository root.
pub const STATE_DIR: &str = ".accelerator/state";

/// Every configured document directory, so a retirement rewrites references
/// across the same tree `corpus frontmatter validate` walks.
pub fn corpus_roots(
    config: &dyn ConfigAccess,
    repo_root: &Path,
) -> Result<Vec<PathBuf>, ::config::ConfigError> {
    Ok(::config::paths::doc_type_dirs(config)?
        .into_iter()
        .map(|resolved| repo_root.join(resolved.dir))
        .collect())
}

struct WorkDirDiscovery<'a> {
    work_dir: &'a Path,
    root: &'a Path,
    names: &'a ConfiguredNames,
    conflicts: &'a VcsConflicts,
}

impl CorpusDiscovery for WorkDirDiscovery<'_> {
    fn discover(&self) -> Result<DiscoveredCorpus, RunError> {
        let (items, inventory) =
            discover_corpus(self.work_dir, self.names, self.conflicts)
                .map_err(RunError::Internal)?;
        Ok(DiscoveredCorpus {
            items,
            inventory,
            status: Box::new(VcsWorkingCopyStatus::probed_from(
                self.root,
                &InProcessProbe,
            )),
        })
    }
}

/// Clears recovery copies nobody needs any more and warns about every
/// restore still waiting for a person.
fn sweep_kept_recoveries(
    console: &mut Console<'_>,
    recovery: &FileRecoveryCopies,
) {
    use corpus::store::RecoveryCopies as _;

    let notices = match sweep_recoveries(recovery) {
        Ok(notices) => notices,
        Err(error) => {
            console.warn(format_args!(
                "warning: recovery copies could not be checked: {}",
                single_line(&error.to_string())
            ));
            return;
        }
    };
    for notice in notices {
        match notice {
            RecoveryNotice::StillPending { dir, unrestored } => {
                let paths =
                    unrestored.iter().fold(String::new(), |listing, path| {
                        listing
                            + "\n  "
                            + &single_line(&path.display().to_string())
                    });
                console.warn(format_args!(
                    "warning: {RETIREMENT_INCOMPLETE}: these paths still differ \
                     from their recovery copies in {}:{paths}\ncompare each path \
                     with its recovery copy and merge, then delete the copy",
                    single_line(&recovery.location(&dir).display().to_string())
                ));
            }
            RecoveryNotice::Cleared { dir } => console.warn(format_args!(
                "note: every path an incomplete restore left has been dealt \
                 with; removed {}",
                single_line(&recovery.location(&dir).display().to_string())
            )),
        }
    }
}

/// A warning naming each promotion record that cannot be read. Every guard
/// already treats such a record as a promotion in flight; the warning tells
/// a person which file to inspect.
pub fn unreadable_promotion_record_warnings(
    records: &dyn PromotionRecords,
) -> Vec<String> {
    match records.outstanding() {
        Ok(entries) => entries
            .into_iter()
            .filter_map(Result::err)
            .map(|unreadable| {
                format!(
                    "warning: the promotion record at {} could not be read \
                     ({}); its draft is treated as mid-promotion until the \
                     file is fixed or removed",
                    single_line(&unreadable.path.display().to_string()),
                    single_line(&unreadable.detail)
                )
            })
            .collect(),
        Err(error) => vec![format!(
            "warning: the promotion records could not be listed: {}",
            single_line(&error.to_string())
        )],
    }
}

/// Notes each retirement that completed after an incomplete restore; its
/// copies stay for reference until the next run removes them.
fn report_kept_recoveries(
    console: &mut Console<'_>,
    recovery: &FileRecoveryCopies,
) {
    use corpus::store::RecoveryCopies as _;

    let Ok(kept) = recovery.kept(Path::new(RECOVERY_PARENT)) else {
        return;
    };
    for kept in kept {
        if kept.state == corpus::store::KeptState::Completed {
            console.warn(format_args!(
                "note: the retirement whose restore was incomplete has since \
                 completed; its recovery copies remain in {} for reference \
                 and are removed on the next sync",
                single_line(
                    &recovery.location(&kept.dir).display().to_string()
                )
            ));
        }
    }
}

/// The orders `--resolve` gives, one per id: a conflict's resolution or a
/// refused pending push's settlement.
struct Orders {
    resolutions: BTreeMap<String, Resolution>,
    pending_push: BTreeMap<String, PendingPushOrder>,
}

/// Once orders can link or create issues, a typo must stop the run rather
/// than quietly do less, so every malformed order is refused here.
fn parse_orders(
    raw: &[(String, String)],
    integration: &str,
    scheme: &WorkItemIdScheme,
) -> Result<Orders, String> {
    let mut orders = Orders {
        resolutions: BTreeMap::new(),
        pending_push: BTreeMap::new(),
    };
    for (id, token) in raw {
        let (shown_id, shown_token) = (single_line(id), single_line(token));
        if orders.resolutions.contains_key(id)
            || orders.pending_push.contains_key(id)
        {
            return Err(format!("--resolve names '{shown_id}' more than once"));
        }
        if let Some(resolution) = work::sync::resolve_conflict_token(token) {
            orders.resolutions.insert(id.clone(), resolution);
            continue;
        }
        let order = match work::sync::resolve_pending_push_token(token) {
            Some(Ok(order)) => order,
            Some(Err(NotATrackerKey(key))) => {
                return Err(format!(
                    "--resolve {shown_id}={shown_token}: '{}' is not a \
                     {integration} key (for example PP-12)",
                    single_line(&key)
                ));
            }
            None => {
                return Err(format!(
                    "--resolve {shown_id}={shown_token}: '{shown_token}' is \
                     not an order (accepted: remote, local, skip, \
                     adopt:<KEY>, create)"
                ));
            }
        };
        if !is_token_safe(scheme, id) {
            return Err(format!(
                "--resolve: '{shown_id}' is not a work-item id"
            ));
        }
        orders.pending_push.insert(id.clone(), order);
    }
    Ok(orders)
}

/// A local create settled by an adopt order links an issue rather than
/// creating one, so its row says so.
const fn action_keyword(planned: &PlannedAction) -> &'static str {
    match planned.action() {
        work::sync::Action::CreateFromLocal
            if matches!(planned.order(), Some(PendingPushOrder::Adopt(_))) =>
        {
            "adopt"
        }
        action => plain_action_keyword(action),
    }
}

const fn plain_action_keyword(action: work::sync::Action) -> &'static str {
    match action {
        work::sync::Action::Push(_)
        | work::sync::Action::Retitle
        | work::sync::Action::Overwrite => "push",
        work::sync::Action::Pull => "pull",
        work::sync::Action::SkipConflict => "skip-conflict",
        work::sync::Action::SkipDirty => "skip-dirty",
        work::sync::Action::Prompt => "unresolved",
        work::sync::Action::Noop => "noop",
        work::sync::Action::RecordBaseline => "record-baseline",
        work::sync::Action::CreateFromRemote => "create-from-remote",
        work::sync::Action::CreateFromLocal => "create-from-local",
        work::sync::Action::Relink => "relink",
    }
}

fn discovery_line(discovery: &DiscoveryStatus) -> String {
    match discovery {
        DiscoveryStatus::Ran { found } => {
            format!("#\tdiscovery\tran\tfound={found}")
        }
        DiscoveryStatus::SkippedPushOnly => {
            "#\tdiscovery\tskipped\tpush-only".to_owned()
        }
        DiscoveryStatus::SkippedTargeted => {
            "#\tdiscovery\tskipped\ttargeted".to_owned()
        }
        DiscoveryStatus::TargetedPull { attempted } => {
            format!("#\tdiscovery\ttargeted-pull\t{attempted}")
        }
        DiscoveryStatus::Failed { detail } => {
            format!("#\tdiscovery\tfailed\t{}", single_line(detail))
        }
        DiscoveryStatus::SkippedIncompleteCorpus => {
            "#\tdiscovery\tskipped\tincomplete-corpus".to_owned()
        }
        DiscoveryStatus::SkippedPendingPush => {
            "#\tdiscovery\tskipped\tpending-push".to_owned()
        }
    }
}

/// The state an identity row shows: the engine's state for the item under
/// its settled id, or the state the identity change itself implies.
fn identity_state(row: &IdentityRow, report: &RunReport) -> String {
    report
        .reported
        .iter()
        .find(|item| item.planned.id() == row.settled_id)
        .map_or_else(
            || match row.action {
                work::sync::IdentityAction::NotFound => {
                    work::sync::SyncState::RemoteAbsent.to_string()
                }
                work::sync::IdentityAction::KeyChanged
                | work::sync::IdentityAction::Resumed => "-".to_owned(),
            },
            |item| item.planned.state().to_string(),
        )
}

fn identity_line(row: &IdentityRow, report: &RunReport) -> String {
    identity_line_in_state(row, &identity_state(row, report))
}

/// An identity row reported with no engine run beside it to take a state
/// from, as `work promote` reports the retirements it finishes first.
pub fn standalone_identity_line(row: &IdentityRow) -> String {
    identity_line_in_state(row, "-")
}

fn identity_line_in_state(row: &IdentityRow, state: &str) -> String {
    match &row.outcome {
        IdentityOutcome::Refused(reason) | IdentityOutcome::Failed(reason) => {
            TsvRecord::of([row.id.as_str(), "failed", state, reason])
                .to_string()
        }
        IdentityOutcome::Applied | IdentityOutcome::NotApplied => {
            TsvRecord::of([
                row.id.clone(),
                row.action.to_string(),
                state.to_owned(),
                row.detail.clone(),
            ])
            .to_string()
        }
    }
}

/// What a run that stopped before its report still prints: every identity
/// change the pass reported, then how many had landed.
fn stopped_run_lines(failure: &SettledRunFailure) -> Vec<String> {
    let mut lines: Vec<String> = failure
        .identity
        .iter()
        .map(standalone_identity_line)
        .chain(failure.promotions.iter().map(promotion_line))
        .collect();
    if failure.identity_applied > 0 {
        lines.push(format!(
            "#\tnote\tidentity-applied-before-refusal\t{}",
            failure.identity_applied
        ));
    }
    lines
}

fn render_report(finished: &FinishedRun<'_>) -> String {
    let report = finished.report;
    let settled_ids: BTreeSet<&str> = report
        .identity
        .iter()
        .map(|row| row.settled_id.as_str())
        .collect();
    let mut lines: Vec<String> = report
        .identity
        .iter()
        .map(|row| identity_line(row, report))
        .chain(report.promotions.iter().map(promotion_line))
        .collect();
    let mut synced_count = 0usize;
    for item in &report.reported {
        let failed = matches!(item.outcome, ItemOutcome::Failed(_));
        if settled_ids.contains(item.planned.id()) && !failed {
            continue;
        }
        if matches!(item.planned.state(), work::sync::SyncState::Synced)
            && !matches!(item.outcome, ItemOutcome::Failed(_))
        {
            synced_count += 1;
            continue;
        }
        let (action_field, detail) = match &item.outcome {
            ItemOutcome::Failed(error) => ("failed", error.class().keyword()),
            ItemOutcome::UnsupportedRecipe => {
                (action_keyword(&item.planned), "unsupported-recipe")
            }
            ItemOutcome::Applied | ItemOutcome::NotApplied => {
                (action_keyword(&item.planned), "-")
            }
        };
        lines.push(
            TsvRecord::of([
                item.planned.id().to_owned(),
                action_field.to_owned(),
                item.planned.state().to_string(),
                detail.to_owned(),
            ])
            .to_string(),
        );
    }
    lines.extend(report.unclassifiable.iter().map(|item| {
        TsvRecord::of([item.id.as_str(), "failed", "-", "retryable"])
            .to_string()
    }));
    lines.extend(report.refused_orders.iter().map(|error| {
        TsvRecord::of([error.item_id(), "failed", "-", error.class().keyword()])
            .to_string()
    }));
    // Capture before appending the always-present discovery line, which would
    // otherwise make `is_empty()` false and drop the empty-run summary.
    let summary_needed = synced_count > 0 || lines.is_empty();
    lines.sort();
    lines.push(discovery_line(&report.discovery));
    lines.extend(report.inventory.unidentified.iter().map(|entry| {
        TsvRecord::of([
            "#".to_owned(),
            "unidentified".to_owned(),
            entry.path.display().to_string(),
            entry.reason.keyword().to_owned(),
        ])
        .to_string()
    }));
    let mut promotions: Vec<_> = report.promotions.iter().collect();
    promotions.sort_by(|left, right| left.draft.cmp(&right.draft));
    lines.extend(promotions.into_iter().flat_map(detail_lines));
    if report.deferred > 0 {
        lines.push(format!(
            "#\tnote\tdeferred-to-next-run\t{}",
            report.deferred
        ));
    }
    lines.extend(kept_edits_records(&finished.kept_edits));
    if summary_needed {
        lines.push(format!("#\tsummary\tsynced\t{synced_count}"));
    }
    lines.join("\n")
}

/// One `#\tkept-edits` record per copy a person may need: kept this run,
/// kept earlier, or in a directory this tool does not trust.
fn kept_edits_records(review: &KeptEditsReview) -> Vec<String> {
    review
        .entries
        .iter()
        .filter_map(kept_edits_record)
        .collect()
}

fn kept_edits_record(entry: &KeptEdits) -> Option<String> {
    let copy_record = |kept: &KeptCopy, when: &str| {
        TsvRecord::of([
            "#".to_owned(),
            "kept-edits".to_owned(),
            kept.id.clone(),
            when.to_owned(),
            kept.item.display().to_string(),
            kept.copy.display().to_string(),
        ])
        .to_string()
    };
    match entry {
        KeptEdits::KeptThisRun(kept) => Some(copy_record(kept, "this-run")),
        KeptEdits::KeptEarlier(kept) | KeptEdits::NotRemoved { kept, .. } => {
            Some(copy_record(kept, "earlier"))
        }
        KeptEdits::Unrecorded { id, directory, .. } => Some(
            TsvRecord::of([
                "#".to_owned(),
                "kept-edits".to_owned(),
                id.clone().unwrap_or_else(|| "-".to_owned()),
                "unrecorded".to_owned(),
                "-".to_owned(),
                directory.display().to_string(),
            ])
            .to_string(),
        ),
        KeptEdits::Discarding { entry, .. }
        | KeptEdits::NotDiscarded { entry, .. } => kept_edits_record(entry),
        KeptEdits::Settled { .. }
        | KeptEdits::Discarded { .. }
        | KeptEdits::NothingToDiscard { .. } => None,
    }
}

/// The stderr notes for kept copies: one per copy kept this run, one
/// summary of every earlier copy, and one per directory that needs a
/// person's attention.
fn kept_edits_notes(review: &KeptEditsReview) -> Vec<String> {
    let mut notes = Vec::new();
    let mut earlier: Vec<&KeptCopy> = Vec::new();
    for entry in &review.entries {
        match entry {
            KeptEdits::KeptThisRun(kept) => notes.push(format!(
                "note: {id}: kept-edits: local edits kept at {copy}; merge \
                 them, then run accelerator work sync --target {id} \
                 --discard-kept-edits {id}",
                id = single_line(&kept.id),
                copy = single_line(&kept.copy.display().to_string()),
            )),
            KeptEdits::KeptEarlier(kept) => earlier.push(kept),
            KeptEdits::NotRemoved { kept, reason } => {
                earlier.push(kept);
                notes.push(format!(
                    "note: {}: kept-edits: settled copy at {} could not be \
                     removed: {}",
                    single_line(&kept.id),
                    single_line(&kept.directory.display().to_string()),
                    single_line(reason)
                ));
            }
            KeptEdits::Unrecorded {
                id,
                directory,
                provenance,
            } => notes.extend(unrecorded_note(
                id.as_deref(),
                directory,
                *provenance,
            )),
            KeptEdits::Settled { .. } => {}
            KeptEdits::Discarding { .. }
            | KeptEdits::Discarded { .. }
            | KeptEdits::NotDiscarded { .. }
            | KeptEdits::NothingToDiscard { .. } => {
                notes.push(discard_note(entry));
            }
        }
    }
    if !earlier.is_empty() {
        notes.push(earlier_copies_note(&earlier));
    }
    notes
}

fn discard_note(entry: &KeptEdits) -> String {
    let id = single_line(entry.id().unwrap_or("-"));
    let directory = entry
        .directory()
        .map(|directory| single_line(&directory.display().to_string()))
        .unwrap_or_default();
    match entry {
        KeptEdits::Discarding { .. } => {
            format!("note: {id}: kept-edits: {directory} would be discarded")
        }
        KeptEdits::Discarded { .. } => {
            format!("note: {id}: kept-edits: discarded {directory}")
        }
        KeptEdits::NotDiscarded { reason, .. } => format!(
            "note: {id}: kept-edits: {directory} could not be discarded: {}",
            single_line(reason)
        ),
        _ => format!("note: {id}: kept-edits: no kept copy to discard"),
    }
}

fn unrecorded_note(
    id: Option<&str>,
    directory: &Path,
    provenance: Provenance,
) -> Option<String> {
    let id = single_line(id.unwrap_or("-"));
    let directory = single_line(&directory.display().to_string());
    match provenance {
        Provenance::Unknown => None,
        Provenance::Tracked => Some(format!(
            "note: {id}: kept-edits: {directory} is tracked by version \
             control and was not made by this tool; untrack it (git rm \
             --cached -r {directory} or jj file untrack {directory}), then \
             sync"
        )),
        Provenance::TrackingUnknown => Some(format!(
            "note: {id}: kept-edits: could not confirm {directory} is \
             untracked; it is kept but not compared"
        )),
    }
}

fn earlier_copies_note(earlier: &[&KeptCopy]) -> String {
    let oldest = earlier
        .iter()
        .filter_map(|kept| kept.kept_since)
        .min()
        .and_then(utc_date)
        .unwrap_or_else(|| "an unknown date".to_owned());
    let mut ids: Vec<String> =
        earlier.iter().map(|kept| single_line(&kept.id)).collect();
    ids.dedup();
    format!(
        "note: -: kept-edits-earlier: {} earlier copies still kept (oldest \
         since {oldest}) for {}; discard each with accelerator work sync \
         --target <id> --discard-kept-edits <id>",
        earlier.len(),
        ids.join(", ")
    )
}

/// `epoch` as an RFC 3339 instant in UTC.
fn utc_date(epoch: u64) -> Option<String> {
    let seconds = i64::try_from(epoch).ok()?;
    let instant = time::OffsetDateTime::from_unix_timestamp(seconds).ok()?;
    instant
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
}

/// Where one run yields several outcomes, the most severe wins:
/// `71 > 1 > 4 > 75 > 74 > 70 > 0`.
pub const fn severity(code: u8) -> u8 {
    match code {
        exit_codes::TERMINAL => 6,
        exit_codes::ERROR => 5,
        exit_codes::UNRESOLVED => 4,
        exit_codes::REJECTED => 3,
        exit_codes::UNCONFIGURED => 2,
        exit_codes::RETRYABLE => 1,
        _ => 0,
    }
}

const fn failure_code(class: work_adapters::sync::apply::FailureClass) -> u8 {
    match class {
        work_adapters::sync::apply::FailureClass::Terminal => {
            exit_codes::TERMINAL
        }
        work_adapters::sync::apply::FailureClass::Retryable => {
            exit_codes::RETRYABLE
        }
        work_adapters::sync::apply::FailureClass::Unconfigured => {
            exit_codes::UNCONFIGURED
        }
        work_adapters::sync::apply::FailureClass::Rejected
        | work_adapters::sync::apply::FailureClass::Unrenderable
        | work_adapters::sync::apply::FailureClass::PullResidue
        | work_adapters::sync::apply::FailureClass::Unparseable
        | work_adapters::sync::apply::FailureClass::WriteRefused
        | work_adapters::sync::apply::FailureClass::OrderRefused => {
            exit_codes::REJECTED
        }
    }
}

/// The stderr line naming the items left untouched for want of a known
/// digest recipe, since their report row alone does not say how to fix it.
pub fn unsupported_recipe_warning(report: &RunReport) -> Option<String> {
    let untouched: Vec<&str> = report
        .reported
        .iter()
        .filter(|item| matches!(item.outcome, ItemOutcome::UnsupportedRecipe))
        .map(|item| item.planned.id())
        .collect();
    (!untouched.is_empty()).then(|| {
        format!(
            "warning: {} item(s) carry a sync baseline written by a newer \
             plugin release, so they were left untouched ({}); upgrade the \
             plugin to sync them",
            untouched.len(),
            untouched.join(", ")
        )
    })
}

/// One stderr line per failure the run reports, built from its parts so a
/// cause can never forge a second line.
fn failure_lines(report: &RunReport) -> Vec<String> {
    let failed = match report.mode {
        RunMode::Preview => "would fail",
        RunMode::Apply => "failed",
    };
    report
        .failures()
        .map(|failure| failure_line(&failure, failed))
        .collect()
}

/// A linked item never creates again, so its leftover marker is harmless;
/// it is named so a person can remove it.
fn leftover_marker_warnings(report: &RunReport) -> Vec<String> {
    report
        .reported
        .iter()
        .filter_map(|item| {
            item.leftover_marker.as_ref().map(|leftover| {
                format!(
                    "warning: {}: could not clear marker {}: {}; the item is \
                     linked, so remove the marker by hand",
                    single_line(item.planned.id()),
                    single_line(&leftover.path.display().to_string()),
                    single_line(&leftover.reason)
                )
            })
        })
        .collect()
}

/// What a person should know of a local create an order settled.
fn create_notices(report: &RunReport) -> Vec<String> {
    report
        .reported
        .iter()
        .filter_map(|item| {
            let id = single_line(item.planned.id());
            item.notice.as_ref().map(|notice| match notice {
                CreateNotice::DifferingAdoption(key) => {
                    let key = single_line(key.as_str());
                    format!(
                        "note: {id}: adopt: linked {key}, whose title or body \
                         differ from the item; the next sync reports a \
                         conflict. If {key} is the wrong issue, run --resolve \
                         {id}=adopt:<RIGHT-KEY>, or --resolve {id}=create if \
                         none exists"
                    )
                }
                CreateNotice::NothingToSettle => format!(
                    "warning: {id}: --resolve {id}=create had no pending push \
                     to settle"
                ),
            })
        })
        .collect()
}

fn exit_code_for_report(report: &RunReport) -> u8 {
    let failures = report
        .failures()
        .filter(|failure| failure.severity == FailureSeverity::Error)
        .map(|failure| failure_code(failure.class));
    let awaiting_human = (report.awaiting_human().next().is_some()
        || report.identity.iter().any(IdentityRow::awaits_human))
    .then_some(exit_codes::UNRESOLVED);
    let read_failed = (report.read_failure.is_some()
        || matches!(report.discovery, DiscoveryStatus::Failed { .. }))
    .then_some(exit_codes::RETRYABLE);
    failures
        .chain(awaiting_human)
        .chain(read_failed)
        .chain(report.promotions.iter().map(promotion_exit))
        .max_by_key(|&code| severity(code))
        .unwrap_or(exit_codes::CLEAN)
}

fn clear_stale_dossiers(
    store: &dyn SyncStore,
    dir: &Path,
    scheme: &WorkItemIdScheme,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(std::ffi::OsStr::to_str)
        else {
            continue;
        };
        let is_stale_dossier = name
            .strip_suffix(".md")
            .is_some_and(|stem| is_token_safe(scheme, stem));
        let is_write_artefact = name.starts_with(kernel::TEMP_PREFIX);
        if is_stale_dossier || is_write_artefact {
            let _ = store.remove(&path);
        }
    }
}

/// Fail-closed: creates the directory and writes a directory-local
/// `.gitignore` of `*`, verifying it before clearing anything, so a run that
/// cannot prove the dossiers ignored never writes one and never destroys the
/// prior run's. The stale-clear removes only canonical-id `<id>.md` dossiers
/// and this surface's own `.tmp-*` write artefacts, so anything else a user
/// placed under `conflicts/` survives.
fn prepare_conflicts_dir(
    store: &dyn SyncStore,
    dir: &Path,
    scheme: &WorkItemIdScheme,
) -> Result<(), String> {
    let gitignore = dir.join(".gitignore");
    store
        .write(&gitignore, b"*\n")
        .map_err(|error| error.to_string())?;
    let verified = std::fs::read_to_string(&gitignore)
        .map_err(|error| error.to_string())?
        .contains('*');
    if !verified {
        return Err("the conflicts .gitignore could not be verified".to_owned());
    }
    clear_stale_dossiers(store, dir, scheme);
    Ok(())
}

fn persist_dossiers(
    console: &mut Console<'_>,
    store: &dyn SyncStore,
    dossiers: &[ConflictDossier],
    dir: &Path,
    scheme: &WorkItemIdScheme,
    render: &dyn Fn(&SectionDiff) -> String,
) {
    for dossier in dossiers {
        if !is_token_safe(scheme, &dossier.id) {
            console.warn(format_args!(
                "warning: skipping dossier for unsafe id {:?}",
                dossier.id
            ));
            continue;
        }
        let body = match render_dossier(dossier, render) {
            DossierRender::Renderable(text)
            | DossierRender::Unrenderable(text) => text,
        };
        let path = dir.join(format!("{}.md", dossier.id));
        if let Err(error) = store.write(&path, body.as_bytes()) {
            console.warn(format_args!(
                "warning: could not write conflict dossier {}: {}",
                single_line(&path.display().to_string()),
                single_line(&error.to_string())
            ));
        }
    }
}

/// The reset-then-write over the shared `conflicts/` directory is not
/// lock-guarded here: `/sync-work-items` issues one `work sync` at a time and
/// the two-invocation shape is sequential, so a stale file from a racing run
/// is overwritten before it is read.
fn persist_conflict_dossiers(
    console: &mut Console<'_>,
    store: &dyn SyncStore,
    dir: &Path,
    dossiers: &[ConflictDossier],
    scheme: &WorkItemIdScheme,
    render: &dyn Fn(&SectionDiff) -> String,
) {
    match prepare_conflicts_dir(store, dir, scheme) {
        Ok(()) => {
            persist_dossiers(console, store, dossiers, dir, scheme, render);
        }
        Err(error) => console.warn(format_args!(
            "warning: conflict dossiers not written — could not guarantee an \
             ignored {} ({})",
            single_line(&dir.display().to_string()),
            single_line(&error)
        )),
    }
}

#[derive(Debug)]
struct ResolvedTargets {
    items: Vec<LocalItem>,
    /// Tokens with no local match, de-duplicated by canonical key. Each is a
    /// candidate for a remote-only pull, confirmed later by one `fetch_all`.
    remote_candidates: Vec<ExternalId>,
}

/// Why one `--target` token could not be resolved. Each carries a
/// user-facing message naming the offender; [`TargetResolutionFailure::exit_code`]
/// is the single source of the code mapping.
///
/// The variants span two sequenced moments. The local and push-only variants
/// are decided before the tracker is contacted, so a run carrying one aborts
/// credential-free; [`Self::Absent`] and [`Self::Indeterminate`] are the
/// outcomes of the post-credential `fetch_all` gate. Both fold into the same
/// [`Self::exit_code`] and rank, so precedence is single-sourced across both.
#[derive(Debug)]
enum TargetResolutionFailure {
    Malformed(String),
    Unmanaged(String),
    OutsideWorkDir(String),
    AmbiguousLocal(String),
    AmbiguousExternal(String),
    LocalCollision(String),
    RetiredAlias(String),
    PushOnlyRemoteOnly(String),
    Absent(String),
    Indeterminate(String),
    Unlistable(String),
}

impl TargetResolutionFailure {
    fn message(&self) -> &str {
        match self {
            Self::Malformed(message)
            | Self::Unmanaged(message)
            | Self::OutsideWorkDir(message)
            | Self::AmbiguousLocal(message)
            | Self::AmbiguousExternal(message)
            | Self::LocalCollision(message)
            | Self::RetiredAlias(message)
            | Self::PushOnlyRemoteOnly(message)
            | Self::Absent(message)
            | Self::Indeterminate(message)
            | Self::Unlistable(message) => message,
        }
    }

    const fn exit_code(&self) -> u8 {
        match self {
            Self::Malformed(_)
            | Self::AmbiguousLocal(_)
            | Self::AmbiguousExternal(_)
            | Self::LocalCollision(_)
            | Self::RetiredAlias(_)
            | Self::PushOnlyRemoteOnly(_) => exit_codes::USAGE,
            Self::Unmanaged(_) | Self::Absent(_) => {
                exit_codes::RESOLVE_NOT_FOUND
            }
            Self::OutsideWorkDir(_) => exit_codes::RESOLVE_OUTSIDE_WORKDIR,
            Self::Indeterminate(_) => exit_codes::RETRYABLE,
            Self::Unlistable(_) => exit_codes::ERROR,
        }
    }
}

/// The failure for a token that more than one item's identity names, told
/// apart by the fields that name it.
fn identity_conflict(
    token: &str,
    candidates: &[IdentityCandidate],
) -> TargetResolutionFailure {
    let named_by = |field: IdentityField| {
        candidates
            .iter()
            .filter(move |candidate| candidate.field == field)
    };
    let local = named_by(IdentityField::Id).next();
    let linked = local.and_then(|local| {
        named_by(IdentityField::ExternalId)
            .find(|candidate| candidate.path != local.path)
    });
    if let (Some(local), Some(linked)) = (local, linked) {
        return TargetResolutionFailure::LocalCollision(format!(
            "'{token}' is the local id of {} and also the external_id \
             recorded by {}; re-run with the path of the file you intended",
            local.path.display(),
            linked.path.display()
        ));
    }
    if let Some(retiring) = named_by(IdentityField::Alias).next() {
        return TargetResolutionFailure::RetiredAlias(format!(
            "'{token}' is an id retired by {} and is also claimed by another \
             item; re-run with the path of the file you intended",
            retiring.path.display()
        ));
    }
    if candidates
        .iter()
        .all(|candidate| candidate.field == IdentityField::ExternalId)
    {
        return TargetResolutionFailure::AmbiguousExternal(format!(
            "'{token}' matches more than one item's external_id; re-run \
             with a local id or a path"
        ));
    }
    ambiguous_local(token)
}

fn ambiguous_local(token: &str) -> TargetResolutionFailure {
    TargetResolutionFailure::AmbiguousLocal(format!(
        "'{token}' is an ambiguous local id; re-run with a full id or a path"
    ))
}

/// Resolves each `--target` token to a local item or a remote candidate,
/// accumulating every failure so one run names all offenders. A token that
/// no item's identity or filename names becomes a remote candidate rather
/// than an abort. An ambiguous, conflicting or out-of-directory local
/// outcome still fails.
fn resolve_targets(
    corpus: &[LocalItem],
    targets: &[String],
    resolver: &dyn Fn(&str) -> RunOutcome,
) -> Result<ResolvedTargets, Vec<TargetResolutionFailure>> {
    let mut matched: Vec<&LocalItem> = Vec::new();
    let mut remote_candidates: Vec<ExternalId> = Vec::new();
    let mut candidate_keys = std::collections::BTreeSet::new();
    let mut failures: Vec<TargetResolutionFailure> = Vec::new();

    for token in targets {
        if token.trim().is_empty() {
            failures.push(TargetResolutionFailure::Malformed(
                "a --target token is empty or whitespace-only".to_owned(),
            ));
            continue;
        }
        match resolver(token) {
            RunOutcome::Resolved(path) => {
                let local_match = corpus.iter().find(|candidate| {
                    candidate
                        .path
                        .canonicalize()
                        .map(|resolved| resolved == path)
                        .unwrap_or(false)
                });
                match local_match {
                    Some(item) => matched.push(item),
                    None => failures.push(TargetResolutionFailure::Unmanaged(
                        format!(
                            "'{token}' resolves to a file that is not a \
                             managed work item"
                        ),
                    )),
                }
            }
            RunOutcome::Ambiguous(_) => failures.push(ambiguous_local(token)),
            RunOutcome::Conflicting(candidates) => {
                failures.push(identity_conflict(token, &candidates));
            }
            RunOutcome::OutsideWorkDir(message) => {
                failures.push(TargetResolutionFailure::OutsideWorkDir(message));
            }
            RunOutcome::Unlistable(message) => {
                failures.push(TargetResolutionFailure::Unlistable(message));
            }
            RunOutcome::NotFound(_) | RunOutcome::Invalid(_) => {
                let key =
                    canonical_external_key(&ExternalId::new(token.to_owned()));
                if candidate_keys.insert(key) {
                    remote_candidates.push(ExternalId::new(token.clone()));
                }
            }
        }
    }

    if !failures.is_empty() {
        return Err(failures);
    }

    let mut seen = std::collections::BTreeSet::new();
    let items = matched
        .into_iter()
        .filter(|item| seen.insert(item.id.clone()))
        .cloned()
        .collect();
    Ok(ResolvedTargets {
        items,
        remote_candidates,
    })
}

struct SelectedTargets {
    scope: CorpusScope,
    items: Vec<LocalItem>,
    remote_candidates: Vec<ExternalId>,
}

/// A usage or ambiguity error is the most fundamental thing to fix, so it
/// dominates the summary code, with each successive class the next-most
/// actionable: `USAGE` (2) > `RESOLVE_OUTSIDE_WORKDIR` (6) > `RESOLVE_NOT_FOUND`
/// (3) > `RETRYABLE` (70). A batch carrying both an absent and an indeterminate
/// token exits 3 by this rank, not by iteration order. Every offender is still
/// named on stderr.
fn highest_precedence_code(failures: &[TargetResolutionFailure]) -> u8 {
    let rank = |code: u8| match code {
        exit_codes::USAGE => 4,
        exit_codes::RESOLVE_OUTSIDE_WORKDIR => 3,
        exit_codes::RESOLVE_NOT_FOUND => 2,
        _ => 1,
    };
    failures
        .iter()
        .map(TargetResolutionFailure::exit_code)
        .max_by_key(|&code| rank(code))
        .unwrap_or(exit_codes::USAGE)
}

/// Builds the reconciliation scope: `All` when no `--target` is given, else the
/// resolved `Targeted` slice plus its remote candidates. On failure prints
/// every offender and returns the highest-precedence exit code. A remote-only
/// candidate under `--push-only` is contradictory and fails here, before any
/// tracker contact, so the abort stays credential-free.
fn build_selection(
    console: &mut Console<'_>,
    corpus: &[LocalItem],
    targets: &[String],
    direction: SyncDirection,
    resolver: &dyn Fn(&str) -> RunOutcome,
) -> Result<SelectedTargets, ExitCode> {
    if targets.is_empty() {
        return Ok(SelectedTargets {
            scope: CorpusScope::WholeCorpus,
            items: Vec::new(),
            remote_candidates: Vec::new(),
        });
    }
    match resolve_targets(corpus, targets, resolver) {
        Ok(found) => {
            if matches!(direction, SyncDirection::PushOnly)
                && !found.remote_candidates.is_empty()
            {
                for candidate in &found.remote_candidates {
                    console.warn(push_only_remote_only(candidate).message());
                }
                return Err(ExitCode::from(exit_codes::USAGE));
            }
            Ok(SelectedTargets {
                scope: CorpusScope::Targeted,
                items: found.items,
                remote_candidates: found.remote_candidates,
            })
        }
        Err(failures) => {
            for failure in &failures {
                console.warn(failure.message());
            }
            Err(ExitCode::from(highest_precedence_code(&failures)))
        }
    }
}

fn push_only_remote_only(candidate: &ExternalId) -> TargetResolutionFailure {
    TargetResolutionFailure::PushOnlyRemoteOnly(format!(
        "'{}' has no local file and --push-only cannot pull a remote-only \
         item; drop --push-only to import it",
        candidate.as_str()
    ))
}

/// Partitions a bulk read of the remote candidates: `found` become the
/// confirmed pull ids (their stamps discarded — `create_from_remote` re-reads
/// each body via `show`), while `absent` and `indeterminate` fold into the
/// failure accumulator so the run aborts before any write. Any failure wins
/// over the confirmed ids, holding the all-or-nothing zero-write property.
fn partition_candidates(
    outcome: tracker::FetchOutcome,
) -> Result<Vec<ExternalId>, Vec<TargetResolutionFailure>> {
    let absent = outcome.absent.into_iter().map(|id| {
        TargetResolutionFailure::Absent(format!(
            "no local file nor remote issue for '{}'",
            id.as_str()
        ))
    });
    let indeterminate = outcome.indeterminate.into_iter().map(|id| {
        TargetResolutionFailure::Indeterminate(format!(
            "remote read for '{}' was indeterminate; re-run when the tracker \
             is reachable",
            id.as_str()
        ))
    });
    let failures: Vec<_> = absent.chain(indeterminate).collect();
    if failures.is_empty() {
        Ok(outcome.found.into_iter().map(|(id, _)| id).collect())
    } else {
        Err(failures)
    }
}

/// Confirms the remote candidates through one `fetch_all`. A whole-call `Err`
/// is a pre-flight fault the port cannot classify further — a credential fault
/// is already caught upstream at `registry.resolve` (exit 74), so the remaining
/// pre-flight fault is retryable — and maps to a single `Indeterminate` (exit
/// 70), never a silent success.
fn confirm_remote_candidates(
    tracker: &dyn tracker::RemoteTracker,
    candidates: &[ExternalId],
) -> Result<Vec<ExternalId>, Vec<TargetResolutionFailure>> {
    match tracker.fetch_all(candidates) {
        Ok(outcome) => partition_candidates(outcome),
        Err(error) => {
            Err(vec![TargetResolutionFailure::Indeterminate(format!(
                "the remote lookup for the named target(s) could not be \
                 completed ({}); re-run when the tracker is reachable",
                error.into_detail()
            ))])
        }
    }
}

/// Resolves the active tracker's scope key — the base creation and discovery
/// scope — dispatching on `work.integration`.
///
/// Jira resolves by identity via `jira.project_key`; Linear resolves the team
/// key (config, deprecated alias, then catalogue) so a catalogue-only repo
/// resolves the same scope here as in the client. A tracker with no scope-key
/// dependency (`trello`, `github-issues`, or an unset integration) preserves
/// the legacy `work.default_project_code` read unchanged.
fn resolve_active_scope_key(
    config: &dyn ConfigAccess,
    integration: &str,
    integrations_root: &Path,
) -> Option<String> {
    match integration {
        "jira" => jira_client::auth::project_code(config).ok(),
        "linear" => linear_client::auth::team_key(config, integrations_root)
            .ok()
            .flatten(),
        _ => {
            let legacy = crate::config::effective_nonempty(
                config,
                "work.default_project_code",
            )
            .unwrap_or_default();
            (!legacy.is_empty()).then_some(legacy)
        }
    }
}

/// Validates the active tracker's `<tracker>.pull` block before discovery, so a
/// malformed block fails loud rather than reaching lowering — the sync-path
/// mirror of the `configure`-time refusal. A tracker with no pull surface, an
/// absent block, or an empty block is a no-op. On failure returns the operator
/// message (naming the resolving config file).
fn validate_pull_config(
    config: &dyn ConfigAccess,
    integration: &str,
) -> Result<(), String> {
    match read_tracker_block(config, integration, BlockName::Pull)? {
        Some((value, level)) => {
            tracker_support::pull::validate(integration, &value)
                .map_err(|error| error.detail(level))
        }
        None => Ok(()),
    }
}

/// Validates the active tracker's `<tracker>.push` block before the run, so a
/// malformed block fails loud rather than silently defaulting — the sync-path
/// mirror of the `configure`-time refusal. A tracker with no push surface, an
/// absent block, or an empty block is a no-op. On failure returns the operator
/// message (naming the resolving config file).
fn validate_push_config(
    config: &dyn ConfigAccess,
    integration: &str,
) -> Result<(), String> {
    match read_tracker_block(config, integration, BlockName::Push)? {
        Some((value, level)) => {
            tracker_support::push::validate(integration, &value)
                .map_err(|error| error.detail(level))
        }
        None => Ok(()),
    }
}

fn read_tracker_block(
    config: &dyn ConfigAccess,
    integration: &str,
    name: BlockName,
) -> Result<Option<(::config::Value, ::config::Level)>, String> {
    if TRACKERS.block(integration, name).is_none() {
        return Ok(None);
    }
    read_block(config, integration, name).map_err(|error| error.to_string())
}

/// The resolved pull-direction write bound and the config level it resolved
/// from — the personal file when a personal block shadows, `None` for the
/// built-in default. Read after [`validate_pull_config`] has passed, so a
/// malformed block cannot reach here; a defensive fault falls back to the
/// built-in default.
fn resolve_max_items(
    config: &dyn ConfigAccess,
    integration: &str,
) -> (tracker::Ceiling, Option<::config::Level>) {
    match tracker_support::pull::read(config, integration) {
        Ok(Some((pull, level))) => {
            let max_items = pull
                .ceilings()
                .map_or(tracker::DEFAULT_MAX_ITEMS, |ceilings| {
                    ceilings.max_items
                });
            (max_items, Some(level))
        }
        _ => (tracker::DEFAULT_MAX_ITEMS, None),
    }
}

/// The resolved push-direction write bound and the config level it resolved
/// from — the personal file when a personal block shadows, `None` for the
/// built-in default. Read after [`validate_push_config`] has passed, so a
/// malformed block cannot reach here; a defensive fault falls back to the
/// built-in default.
fn resolve_push_max_items(
    config: &dyn ConfigAccess,
    integration: &str,
) -> (tracker::Ceiling, Option<::config::Level>) {
    match tracker_support::push::read(config, integration) {
        Ok(Some((push, level))) => {
            let max_items =
                push.max_items().unwrap_or(tracker::DEFAULT_MAX_ITEMS);
            (max_items, Some(level))
        }
        _ => (tracker::DEFAULT_MAX_ITEMS, None),
    }
}

/// The effective pull bound: a present `--max-pulls` flag overrides the
/// configured `<tracker>.pull.max_items`; an unset flag defers to it.
fn effective_max_pulls(
    flag: Option<tracker::Ceiling>,
    configured: tracker::Ceiling,
) -> tracker::Ceiling {
    flag.unwrap_or(configured)
}

/// The effective push bound: a present `--max-pushes` flag overrides the
/// configured `<tracker>.push.max_items`; an unset flag defers to it.
fn effective_max_pushes(
    flag: Option<tracker::Ceiling>,
    configured: tracker::Ceiling,
) -> tracker::Ceiling {
    flag.unwrap_or(configured)
}

/// The resolved discovery page cap and the config level it resolved from — the
/// built-in default when no block set it. Read after [`validate_pull_config`]
/// has passed; a defensive fault falls back to the built-in default.
fn resolve_discovery_pages(
    config: &dyn ConfigAccess,
    integration: &str,
) -> (tracker::Ceiling, Option<::config::Level>) {
    match tracker_support::pull::read(config, integration) {
        Ok(Some((pull, level))) => {
            let cap = pull
                .ceilings()
                .map_or(tracker::DEFAULT_MAX_PAGES, |ceilings| {
                    ceilings.discovery_pages
                });
            (cap, Some(level))
        }
        _ => (tracker::DEFAULT_MAX_PAGES, None),
    }
}

/// The resolved keyed-read page cap and the config level it resolved from — the
/// built-in default when no block set it. Read after [`validate_pull_config`]
/// has passed; a defensive fault falls back to the built-in default.
fn resolve_keyed_read_pages(
    config: &dyn ConfigAccess,
    integration: &str,
) -> (tracker::Ceiling, Option<::config::Level>) {
    match tracker_support::pull::read(config, integration) {
        Ok(Some((pull, level))) => {
            let cap = pull
                .ceilings()
                .map_or(tracker::DEFAULT_MAX_PAGES, |ceilings| {
                    ceilings.keyed_read_pages
                });
            (cap, Some(level))
        }
        _ => (tracker::DEFAULT_MAX_PAGES, None),
    }
}

/// The configured additional discovery entities, normalised from
/// `<tracker>.pull.additional_projects` / `additional_teams`. Read after
/// [`validate_pull_config`] has passed; an absent block or a defensive fault is
/// no broadening.
fn resolve_additional_entities(
    config: &dyn ConfigAccess,
    integration: &str,
) -> Vec<String> {
    match tracker_support::pull::read(config, integration) {
        Ok(Some((pull, _))) => pull.additional_entities,
        _ => Vec::new(),
    }
}

/// Whether `<tracker>.pull.all_projects` / `all_teams` is set, so discovery
/// covers the whole visible workspace. Read after [`validate_pull_config`] has
/// passed; an absent block or a defensive fault is base-scoped.
fn resolve_all_entities(config: &dyn ConfigAccess, integration: &str) -> bool {
    matches!(tracker_support::pull::read(config, integration), Ok(Some((pull, _))) if pull.all_entities)
}

/// Whether the unbounded-broadened-pull gate fires: an `unlimited` pull bound
/// over a broadened scope, not acknowledged with `--allow-unbounded`. A bounded
/// bound, a base-only scope, or the acknowledgement each keeps it closed.
const fn unbounded_gate_fires(
    max_pulls: tracker::Ceiling,
    broadened: bool,
    allow_unbounded: bool,
) -> bool {
    matches!(max_pulls, tracker::Ceiling::Unlimited)
        && broadened
        && !allow_unbounded
}

/// The unbounded-broadened-pull gate refusal. Names the hazard — `unlimited`
/// `max_items` over a broadened scope — the `--allow-unbounded`
/// acknowledgement, and the finite-`max_items` alternative.
fn unbounded_gate_message(integration: &str) -> String {
    format!(
        "Refusing an unbounded broadened pull: {integration}.pull.max_items is \
         `unlimited` and the pull scope is broadened (all_* or additional_*), \
         so the whole discovered set would be created with no write bound. \
         Re-run with --allow-unbounded to acknowledge this, or set a finite \
         {integration}.pull.max_items."
    )
}

/// The configured discovery filters, flattened from `<tracker>.pull.filters`
/// into the port's flat `(key, value)` bag — one entry per value, so an adapter
/// groups same-key values into one `IN` (values OR'd). Read after
/// [`validate_pull_config`] has passed; an absent block or a defensive fault is
/// no filters.
fn resolve_pull_filters(
    config: &dyn ConfigAccess,
    integration: &str,
) -> Vec<(String, String)> {
    match tracker_support::pull::read(config, integration) {
        Ok(Some((pull, _))) => pull
            .filters
            .into_iter()
            .flat_map(|(key, values)| {
                values.into_iter().map(move |value| (key.clone(), value))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The keyed-read cap-hit abort message. Names the keyed-read
/// `<tracker>.pull.max_pages` cap, the file it resolved from, the higher-value
/// / `unlimited` valve, and that the abort is run-wide — `--push-only` shares
/// the same reconcile read, so it is no bypass.
fn keyed_read_capped_message(
    config: &dyn ConfigAccess,
    integration: &str,
) -> String {
    let (cap, source) = resolve_keyed_read_pages(config, integration);
    let source =
        source.map_or("the built-in default", ::config::Level::filename);
    format!(
        "refused: the keyed reconcile read hit its page cap {cap} before \
         accounting for every tracked item, so the remote state of the un-read \
         items is unknown. Nothing was written. The cap resolves from \
         {integration}.pull.max_pages, or its keyed_read override ({source}); \
         raise it or set it to `unlimited` to lift the cap, then re-run. The \
         read feeds both pull and push planning, so --push-only does not bypass \
         this."
    )
}

/// The incomplete-discovery refusal message. A cap-hit names the discovery
/// `<tracker>.pull.max_pages` cap, the file it resolved from, and the
/// `unlimited` valve; a transient cutoff reports the read was budget-limited
/// and steers to a retry rather than mis-blaming the cap.
fn discovery_incomplete_message(
    config: &dyn ConfigAccess,
    integration: &str,
    found: usize,
    completeness: tracker::Completeness,
) -> String {
    let seen = format!(
        "refused: untracked-remote discovery was cut short after seeing \
         {found} issue(s) and cannot be trusted as complete."
    );
    match completeness {
        tracker::Completeness::CapHit => {
            let (cap, source) = resolve_discovery_pages(config, integration);
            let source = source
                .map_or("the built-in default", ::config::Level::filename);
            format!(
                "{seen} It hit the discovery page cap {cap}, which resolves \
                 from {integration}.pull.max_pages ({source}); raise it or set \
                 it to `unlimited` to lift the cap, or scope the search to a \
                 single project or team."
            )
        }
        tracker::Completeness::Complete | tracker::Completeness::Transient => {
            format!(
                "{seen} The read was cut short transiently (a deadline, rate \
                 limit, or wire failure), not by a page cap; retry, and scope \
                 the search to a single project or team if it recurs."
            )
        }
    }
}

/// The write-bounds refusal message, naming the effective limits and the
/// `<tracker>.pull.max_items` / `<tracker>.push.max_items` keys with the config
/// file each resolved from (the built-in default when no block set it).
#[allow(clippy::too_many_arguments)]
fn refusal_message(
    integration: &str,
    pulls: usize,
    pushes: usize,
    max_pulls: tracker::Ceiling,
    max_pushes: tracker::Ceiling,
    new_local_files: usize,
    new_remote_issues: usize,
    pulls_source: Option<::config::Level>,
    pushes_source: Option<::config::Level>,
) -> String {
    let pulls_source =
        pulls_source.map_or("the built-in default", ::config::Level::filename);
    let pushes_source =
        pushes_source.map_or("the built-in default", ::config::Level::filename);
    format!(
        "refused: this run would pull {pulls} item(s) ({new_local_files} of \
         them new local files, limit {max_pulls}) and push {pushes} item(s) \
         ({new_remote_issues} of them new remote issues, limit {max_pushes}). \
         The pull limit resolves from {integration}.pull.max_items \
         ({pulls_source}) and the push limit from {integration}.push.max_items \
         ({pushes_source}); raise the binding limit or set it to `unlimited` to \
         lift the bound, override with --max-pulls/--max-pushes, or inspect the \
         plan first with --preview."
    )
}

/// What a failed run's message needs to name the limits and caps it hit.
struct RunErrorContext<'a> {
    config: &'a dyn ConfigAccess,
    integration: &'a str,
    max_pulls_source: Option<::config::Level>,
    max_pushes_source: Option<::config::Level>,
}

/// The message and exit code for a run that stopped before reporting.
fn run_error_outcome(
    error: &RunError,
    context: &RunErrorContext<'_>,
) -> (String, u8) {
    match error {
        RunError::Refused {
            pulls,
            pushes,
            max_pulls,
            max_pushes,
            new_local_files,
            new_remote_issues,
        } => (
            refusal_message(
                context.integration,
                *pulls,
                *pushes,
                *max_pulls,
                *max_pushes,
                *new_local_files,
                *new_remote_issues,
                context.max_pulls_source,
                context.max_pushes_source,
            ),
            exit_codes::REFUSED_BULK_OVERWRITE,
        ),
        RunError::DiscoveryIncomplete {
            found,
            completeness,
        } => (
            discovery_incomplete_message(
                context.config,
                context.integration,
                *found,
                *completeness,
            ),
            exit_codes::REFUSED_BULK_OVERWRITE,
        ),
        RunError::DiscoveryUnconfigured { detail } => (
            format!("refused: discovery is unconfigured — {detail}"),
            exit_codes::UNCONFIGURED,
        ),
        RunError::KeyedReadCapped => (
            keyed_read_capped_message(context.config, context.integration),
            exit_codes::KEYED_READ_CAPPED,
        ),
        RunError::RetirementIncomplete { message } => {
            (message.clone(), exit_codes::TERMINAL)
        }
        RunError::Read(error) => (error.to_string(), exit_codes::RETRYABLE),
        RunError::Internal(error) => (error.to_string(), exit_codes::ERROR),
    }
}

/// What `run_sync_with` takes from its caller beyond the request: the
/// tracker, what runs once the report is out, and where the run prints.
pub struct SyncSeams<'a> {
    pub registry: &'a dyn TrackerRegistry,
    pub finaliser: &'a dyn RunFinaliser,
    pub console: Console<'a>,
    pub stores: StoreFactories<'a>,
}

/// # Errors
///
/// Never returns `Err`; every failure is reported through the exit code.
#[must_use]
pub fn run_sync(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &SyncArgs,
    registry: &dyn TrackerRegistry,
    finaliser: &dyn RunFinaliser,
) -> ExitCode {
    let (mut out, mut err) =
        (std::io::stdout().lock(), std::io::stderr().lock());
    run_sync_with(
        start,
        config,
        args,
        &mut SyncSeams {
            registry,
            finaliser,
            console: Console::new(&mut out, &mut err),
            stores: StoreFactories::production(),
        },
    )
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn run_sync_with(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &SyncArgs,
    seams: &mut SyncSeams<'_>,
) -> ExitCode {
    let SyncSeams {
        registry,
        finaliser,
        console,
        stores,
    } = seams;
    let direction = if args.push_only {
        SyncDirection::PushOnly
    } else if args.pull_only {
        SyncDirection::PullOnly
    } else {
        SyncDirection::Bidirectional
    };

    let integration =
        match crate::config::effective_nonempty(config, "work.integration") {
            Ok(value) => value,
            Err(error) => {
                console.warn(error);
                return ExitCode::from(exit_codes::ERROR);
            }
        };

    if let Err(message) = validate_pull_config(config, &integration) {
        console.warn(message);
        return ExitCode::from(exit_codes::ERROR);
    }

    if let Err(message) = validate_push_config(config, &integration) {
        console.warn(message);
        return ExitCode::from(exit_codes::ERROR);
    }

    // The directory resolution and target validation run before the tracker's
    // credential check, so a target-resolution abort is credential-independent.
    let repo_root = config_adapters::FileConfigStore::discover_root(start);
    let scheme = match crate::config::resolve_scheme(config) {
        Ok(scheme) => scheme,
        Err(error) => {
            console.warn(error);
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let discarding = match ids_to_discard(&scheme, &args.discard_kept_edits) {
        Ok(ids) => ids,
        Err(message) => {
            console.warn(message);
            return ExitCode::from(exit_codes::USAGE);
        }
    };
    let (work, integrations) = match configured_roots(config, &repo_root) {
        Ok(roots) => roots,
        Err(error) => {
            console.warn(error);
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let roots = match ValidatedRoots::check(&work, &integrations, &repo_root) {
        Ok(roots) => roots,
        Err(refusal) => {
            console.warn(configure_failure(&refusal));
            return ExitCode::from(exit_codes::UNCONFIGURED);
        }
    };
    let identity_run = match crate::identity_run::begin(&repo_root, "sync") {
        Ok(guard) => guard,
        Err(refusal) => {
            let (message, code) =
                crate::identity_run::refused(refusal, &repo_root);
            console.warn(message);
            return ExitCode::from(code);
        }
    };
    let work_dir = roots.work_items.root().to_path_buf();
    let integrations_root = roots.integrations.root().to_path_buf();

    let orders = match parse_orders(&args.resolutions, &integration, &scheme) {
        Ok(orders) => orders,
        Err(message) => {
            console.warn(message);
            return ExitCode::from(exit_codes::USAGE);
        }
    };
    let names = match ConfiguredNames::for_scheme(&scheme) {
        Ok(names) => names,
        Err(error) => {
            console.warn(error);
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let (conflicts, conflicts_unread) =
        VcsConflicts::read(&repo_root, &InProcessProbe, &InProcessProbe);
    if let Some(warning) = conflicts_unread {
        console.warn(warning);
    }
    let (items, inventory) =
        match discover_corpus(&work_dir, &names, &conflicts) {
            Ok(corpus) => corpus,
            Err(error) => {
                console.warn(error);
                return ExitCode::from(exit_codes::ERROR);
            }
        };

    let canonical_work_dir = match work_dir.canonicalize() {
        Ok(dir) => dir,
        Err(error) => {
            console.warn(format_args!(
                "could not resolve the work directory {}: {}",
                single_line(&work_dir.display().to_string()),
                single_line(&error.to_string())
            ));
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let resolver = |token: &str| {
        crate::resolve::resolve_with(&scheme, &canonical_work_dir, start, token)
    };
    let selected = match build_selection(
        console,
        &items,
        &args.targets,
        direction,
        &resolver,
    ) {
        Ok(selected) => selected,
        Err(code) => return code,
    };

    let tracker = match registry.resolve(&integration) {
        Ok(tracker) => tracker,
        Err(
            error @ (SelectionError::Unset
            | SelectionError::Unrecognised { .. }),
        ) => {
            console.warn(error.message());
            return ExitCode::from(exit_codes::UNRECOGNISED);
        }
        Err(error @ SelectionError::NotAvailable { .. }) => {
            console.warn(error.message());
            return ExitCode::from(exit_codes::NOT_AVAILABLE);
        }
        Err(error @ SelectionError::Unconfigured { .. }) => {
            console.warn(error.message());
            return ExitCode::from(exit_codes::UNCONFIGURED);
        }
    };

    let pull_ids = if selected.remote_candidates.is_empty() {
        Vec::new()
    } else {
        match confirm_remote_candidates(
            tracker.as_ref(),
            &selected.remote_candidates,
        ) {
            Ok(found) => found,
            Err(failures) => {
                for failure in &failures {
                    console.warn(failure.message());
                }
                return ExitCode::from(highest_precedence_code(&failures));
            }
        }
    };

    let workspace = match IdentityWorkspace::for_sync(
        config,
        &repo_root,
        &roots,
        &integration,
    ) {
        Ok(workspace) => workspace,
        Err(message) => {
            console.warn(message);
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let recovery = workspace.recovery();
    let discovery = WorkDirDiscovery {
        work_dir: &work_dir,
        root: &repo_root,
        names: &names,
        conflicts: &conflicts,
    };
    let mode = if args.preview {
        RunMode::Preview
    } else {
        RunMode::Apply
    };
    let strategy = if args.per_item_reads {
        RetrievalStrategy::PerItem
    } else {
        RetrievalStrategy::Bulk
    };
    let entities = if resolve_all_entities(config, &integration) {
        tracker::EntityScope::WholeWorkspace
    } else {
        tracker::EntityScope::Keyed {
            base: resolve_active_scope_key(
                config,
                &integration,
                &integrations_root,
            ),
            additional: resolve_additional_entities(config, &integration),
        }
    };
    let scope = tracker::SearchScope {
        entities,
        filters: resolve_pull_filters(config, &integration),
    };
    let selection = match selected.scope {
        CorpusScope::WholeCorpus => ItemSelection::All,
        CorpusScope::Targeted => ItemSelection::Targeted {
            items: &selected.items,
            pull_ids: &pull_ids,
        },
    };
    let (config_max_pulls, max_pulls_source) =
        resolve_max_items(config, &integration);
    let (config_max_pushes, max_pushes_source) =
        resolve_push_max_items(config, &integration);
    let max_pulls = effective_max_pulls(args.max_pulls, config_max_pulls);
    let max_pushes = effective_max_pushes(args.max_pushes, config_max_pushes);
    if unbounded_gate_fires(
        max_pulls,
        work_adapters::sync::scope::is_broadened(&scope),
        args.allow_unbounded,
    ) {
        console.warn(unbounded_gate_message(&integration));
        return ExitCode::from(exit_codes::REFUSED_UNBOUNDED);
    }
    let request = SyncRequest {
        corpus: &items,
        inventory: &inventory,
        selection,
        direction,
        strategy,
        resolutions: &orders.resolutions,
        pending_push_orders: &orders.pending_push,
        max_pulls,
        max_pushes,
        mode,
        integrations_root: &integrations_root,
        integration: &integration,
        scope,
        promote: !args.no_promote,
    };

    sweep_kept_recoveries(console, recovery);
    let stores = SyncStores::bounded(
        &roots.work_items,
        &roots.integration_state(&integration),
        stores,
    );
    let outcome = assemble_run(
        &RunAssembly {
            stores: &stores,
            config,
            workspace: &workspace,
            repo_root: &repo_root,
            work_dir: &work_dir,
            integrations_root: &integrations_root,
            tracker: tracker.as_ref(),
            identity_run: &identity_run,
            scheme: &scheme,
        },
        |ports, settlement, baseline| {
            for warning in
                unreadable_promotion_record_warnings(settlement.promotions)
            {
                console.warn(warning);
            }
            run_settled(&request, ports, settlement, baseline, &discovery)
        },
    );
    match outcome {
        Ok(report) => report_finished_run(
            console,
            &FinishedRunContext {
                config,
                integration: &integration,
                integrations_root: &integrations_root,
                repo_root: &repo_root,
                corpus: &items,
                workspace: &workspace,
                work_dir: &work_dir,
                scheme: &scheme,
                discarding: &discarding,
                finaliser: *finaliser,
                dossier_store: stores.integration_state.as_ref(),
            },
            &report,
        ),
        Err(failure) => report_stopped_run(
            console,
            &failure,
            &RunErrorContext {
                config,
                integration: &integration,
                max_pulls_source,
                max_pushes_source,
            },
            recovery,
        ),
    }
}

/// The items whose kept edits `--discard-kept-edits` names, each one an id
/// that can name a directory.
fn ids_to_discard(
    scheme: &WorkItemIdScheme,
    named: &[String],
) -> Result<BTreeSet<String>, String> {
    named
        .iter()
        .map(|id| {
            if is_token_safe(scheme, id) {
                Ok(id.clone())
            } else {
                Err(format!(
                    "--discard-kept-edits '{}' is not a work-item id",
                    single_line(id)
                ))
            }
        })
        .collect()
}

/// What a run's stores and ports are built from.
struct RunAssembly<'a> {
    stores: &'a SyncStores,
    config: &'a dyn ConfigAccess,
    workspace: &'a IdentityWorkspace,
    repo_root: &'a Path,
    work_dir: &'a Path,
    integrations_root: &'a Path,
    tracker: &'a dyn tracker::RemoteTracker,
    identity_run: &'a IdentityRunGuard,
    scheme: &'a WorkItemIdScheme,
}

/// Builds the stores and ports a run writes through and hands them to
/// `run`. They borrow one another, so they live only as long as it does.
fn assemble_run<T>(
    assembly: &RunAssembly<'_>,
    run: impl for<'p> FnOnce(
        &SyncPorts<'p>,
        &SettlementPorts<'_>,
        &mut BaselineStore<'p>,
    ) -> T,
) -> T {
    let workspace = assembly.workspace;
    let stores = assembly.stores;
    let project_store = FileCorpusStore::new(assembly.repo_root);
    let state_store = FileCorpusStore::new(workspace.state_dir());
    let integrations_store = FileCorpusStore::new(assembly.integrations_root);
    let mut baseline_store =
        workspace.baseline(stores.integration_state.as_ref());
    let retirement_baseline =
        workspace.baseline(stores.integration_state.as_ref());
    let retirement =
        workspace.retirement_ports(&project_store, &retirement_baseline);
    let records = workspace.retirement_records(&state_store);
    let promotions = workspace.promotion_records(&integrations_store);
    let probe_status = || -> Box<dyn WorkingCopyStatus> {
        Box::new(VcsWorkingCopyStatus::probed_from(
            assembly.repo_root,
            &InProcessProbe,
        ))
    };
    let settlement = SettlementPorts {
        retirement: &retirement,
        records: &records,
        promotions: &promotions,
        ownership: assembly.scheme.ownership(),
        state_dir: workspace.state_dir(),
        probe_status: &probe_status,
        remote: work_adapters::promotion::RemoteReading {
            tracker: assembly.tracker,
            parser: &CommonMarkBody,
        },
        identity_run: assembly.identity_run,
    };
    let clock = SystemClock;
    let author = crate::sync_author::ConfiguredLocalAuthor::new(
        assembly.config,
        assembly.repo_root.to_path_buf(),
        assembly.work_dir.to_path_buf(),
        assembly.identity_run,
    );
    let ports = SyncPorts {
        tracker: assembly.tracker,
        parser: &CommonMarkBody,
        status: workspace.status(),
        files: SyncFiles {
            work_items: stores.work_items.as_ref(),
            integration_state: stores.integration_state.as_ref(),
            file_locks: workspace.locks(),
            recovery: workspace.recovery(),
            tracking: workspace.tracking(),
        },
        clock: &clock,
        author: &author,
        recovery_labels: &recovery_labels::DrawnRecoveryLabels::new(
            assembly.scheme,
            RandomSuffixDraws,
        ),
    };
    run(&ports, &settlement, &mut baseline_store)
}

/// What reporting a finished run needs beside its report.
struct FinishedRunContext<'a> {
    config: &'a dyn ConfigAccess,
    integration: &'a str,
    integrations_root: &'a Path,
    repo_root: &'a Path,
    corpus: &'a [LocalItem],
    workspace: &'a IdentityWorkspace,
    work_dir: &'a Path,
    scheme: &'a WorkItemIdScheme,
    discarding: &'a BTreeSet<String>,
    finaliser: &'a dyn RunFinaliser,
    dossier_store: &'a dyn SyncStore,
}

impl FinishedRunContext<'_> {
    fn kept_edits_retention(&self) -> KeptEditsRetention<'_> {
        KeptEditsRetention {
            recovery: self.workspace.recovery(),
            tracking: self.workspace.tracking(),
            locks: self.workspace.locks(),
            work_root: self.work_dir.to_path_buf(),
            scheme: self.scheme,
            corpus: self.corpus,
            discarding: self.discarding,
        }
    }
}

/// The review of every kept copy, and on an apply run the sweep of those
/// that settled; a review that cannot list the copies is warned about and
/// reports none.
fn review_kept_edits(
    console: &mut Console<'_>,
    context: &FinishedRunContext<'_>,
    report: &RunReport,
) -> KeptEditsReview {
    let retention = context.kept_edits_retention();
    let this_run: BTreeSet<PathBuf> = report
        .reported
        .iter()
        .filter_map(|item| item.kept_edits.clone())
        .collect();
    match retention.review(&this_run) {
        Ok(review) if report.mode == RunMode::Apply => retention.sweep(review),
        Ok(review) => review,
        Err(error) => {
            console.warn(format_args!(
                "warning: kept edits could not be checked: {}",
                single_line(&error.to_string())
            ));
            KeptEditsReview {
                entries: Vec::new(),
            }
        }
    }
}

fn report_finished_run(
    console: &mut Console<'_>,
    context: &FinishedRunContext<'_>,
    report: &RunReport,
) -> ExitCode {
    match &report.baseline_degradation {
        baseline::Degradation::Unparseable { detail } => {
            console.warn(format_args!(
                "warning: {} could not be parsed ({}); treating as empty",
                single_line(
                    &baseline::path(
                        context.integrations_root,
                        context.integration
                    )
                    .display()
                    .to_string()
                ),
                single_line(detail)
            ));
        }
        baseline::Degradation::EntriesDiscarded { ids } => {
            console.warn(format_args!(
                "warning: baseline entries discarded (malformed): {}",
                single_line(&ids.join(", "))
            ));
        }
        baseline::Degradation::None => {}
    }
    if report.keyed_read_budget_limited {
        console.warn(
            "warning: the keyed reconcile read was cut short by a transient \
             budget limit (a deadline or wire failure); the affected items \
             are indeterminate this run and will reconcile on a retry.",
        );
    }
    let conflicts_dir = context
        .integrations_root
        .join(context.integration)
        .join("conflicts");
    match crate::config::resolve_scheme(context.config) {
        Ok(scheme) => persist_conflict_dossiers(
            console,
            context.dossier_store,
            &conflicts_dir,
            &report.dossiers,
            &scheme,
            &work_adapters::diff::render,
        ),
        Err(error) => console.warn(format_args!(
            "warning: conflict dossiers not written — could not resolve the \
             work-item id scheme ({})",
            single_line(&error.to_string())
        )),
    }
    let finished = FinishedRun {
        report,
        corpus_external_ids: context
            .corpus
            .iter()
            .filter_map(|item| item.external_id.clone())
            .collect(),
        integrations_root: context.integrations_root,
        repo_root: context.repo_root,
        kept_edits: review_kept_edits(console, context, report),
    };
    console.report(render_report(&finished));
    for failure in failure_lines(report) {
        console.warn(failure);
    }
    for warning in leftover_marker_warnings(report) {
        console.warn(warning);
    }
    for notice in create_notices(report) {
        console.warn(notice);
    }
    for note in kept_edits_notes(&finished.kept_edits) {
        console.warn(note);
    }
    if let Some(warning) = unsupported_recipe_warning(report) {
        console.warn(warning);
    }
    context
        .finaliser
        .finalise(&finished, &mut console.warnings());
    warn_outstanding_pushes(
        console,
        context.integrations_root,
        context.integration,
    );
    report_kept_recoveries(console, context.workspace.recovery());
    ExitCode::from(exit_code_for_report(report))
}

fn report_stopped_run(
    console: &mut Console<'_>,
    failure: &SettledRunFailure,
    context: &RunErrorContext<'_>,
    recovery: &FileRecoveryCopies,
) -> ExitCode {
    for line in stopped_run_lines(failure) {
        console.report(line);
    }
    let (message, code) = run_error_outcome(&failure.error, context);
    console.warn(message);
    report_kept_recoveries(console, recovery);
    ExitCode::from(code)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unnecessary_wraps,
    clippy::unimplemented,
    clippy::panic
)]
mod tests {
    use corpus::WorkItemIdScheme;
    use tracker::RemoteTimestamp;
    use tracker::TrackerError;
    use work::section_diff::SectionDiff;
    use work::sync::Action;
    use work::sync::FieldDelivery;
    use work::sync::PendingPushOrder;
    use work::sync::PlannedAction;
    use work::sync::SyncState;
    use work_adapters::sync::apply::ApplyError;
    use work_adapters::sync::apply::ItemProgress;
    use work_adapters::sync::apply::LeftoverMarker;
    use work_adapters::sync::operation::SyncOperation;
    use work_adapters::sync::report::FailureSeverity;
    use work_adapters::sync::report::Finalisation;
    use work_adapters::sync::run::ConflictDossier;
    use work_adapters::sync::run::DiscoveryStatus;
    use work_adapters::sync::run::ItemOutcome;
    use work_adapters::sync::run::ReportedItem;
    use work_adapters::sync::run::RunMode;
    use work_adapters::sync::run::RunReport;
    use work_adapters::sync::run::UnrenderableReason;

    use std::path::Path;
    use std::path::PathBuf;

    use tracker::ExternalId;
    use work_adapters::sync::fetch::LocalItem;

    use work::sync::SyncDirection;

    use super::build_selection;
    use super::highest_precedence_code;
    use super::leftover_marker_warnings;
    use super::partition_candidates;
    use super::render_report;
    use super::resolve_targets;
    use super::unbounded_gate_fires;
    use super::TargetResolutionFailure;
    use crate::exit_codes;
    use crate::resolve::RunOutcome;
    use work::inventory::UnidentifiedReason;
    use work_adapters::sync::report::CorpusScope;

    #[test]
    fn the_unbounded_gate_fires_only_on_unlimited_over_a_broadened_scope() {
        use tracker::Ceiling;
        // The hazard: unlimited writes over a broadened scope, unacknowledged.
        assert!(unbounded_gate_fires(Ceiling::Unlimited, true, false));
        // Acknowledged, so it proceeds.
        assert!(!unbounded_gate_fires(Ceiling::Unlimited, true, true));
        // A finite bound protects even a broadened scope.
        assert!(!unbounded_gate_fires(Ceiling::Bounded(25), true, false));
        // An unbounded base-only pull is unaffected.
        assert!(!unbounded_gate_fires(Ceiling::Unlimited, false, false));
    }

    fn scheme() -> WorkItemIdScheme {
        WorkItemIdScheme::numeric()
    }

    fn target_item(dir: &Path, id: &str, external: Option<&str>) -> LocalItem {
        let path = dir.join(format!("{id}.md"));
        std::fs::write(&path, "body").expect("write item");
        LocalItem {
            id: id.to_owned(),
            path,
            external_id: external.map(|raw| ExternalId::new(raw.to_owned())),
        }
    }

    fn canonical(item: &LocalItem) -> PathBuf {
        item.path.canonicalize().expect("canonicalise item path")
    }

    struct RealCorpus {
        _dir: tempfile::TempDir,
        work_dir: PathBuf,
    }

    impl RealCorpus {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("tempdir");
            let work_dir =
                dir.path().canonicalize().expect("canonical work dir");
            std::fs::create_dir_all(work_dir.join("drafts"))
                .expect("drafts dir");
            Self {
                _dir: dir,
                work_dir,
            }
        }

        fn item(&self, relative: &str, frontmatter: &str) -> &Self {
            std::fs::write(
                self.work_dir.join(relative),
                format!("---\n{frontmatter}---\n\n# Title\n"),
            )
            .expect("write item");
            self
        }

        fn items(&self) -> Vec<LocalItem> {
            let (conflicts, _) = super::work_files::VcsConflicts::read(
                &self.work_dir,
                &vcs_adapters::library::InProcessProbe,
                &vcs_adapters::library::InProcessProbe,
            );
            super::work_files::discover_corpus(
                &self.work_dir,
                &super::work_files::ConfiguredNames::for_scheme(&scheme())
                    .expect("the scheme compiles"),
                &conflicts,
            )
            .expect("discover items")
            .0
        }

        fn resolve(&self, targets: &[&str]) -> ResolveResult {
            let resolver = |token: &str| {
                crate::resolve::resolve_with(
                    &scheme(),
                    &self.work_dir,
                    &self.work_dir,
                    token,
                )
            };
            let targets: Vec<String> =
                targets.iter().map(|&target| target.to_owned()).collect();
            resolve_targets(&self.items(), &targets, &resolver)
        }
    }

    type ResolveResult =
        Result<super::ResolvedTargets, Vec<TargetResolutionFailure>>;

    fn only_failure(result: ResolveResult) -> TargetResolutionFailure {
        let Err(mut failures) = result else {
            unreachable!("the target must fail")
        };
        assert_eq!(failures.len(), 1, "{failures:?}");
        failures.remove(0)
    }

    fn matched_ids(result: ResolveResult) -> Vec<String> {
        result
            .expect("the targets resolve")
            .items
            .into_iter()
            .map(|item| item.id)
            .collect()
    }

    #[test]
    fn a_target_that_is_one_items_id_and_anothers_external_id_is_a_local_collision(
    ) {
        let corpus = RealCorpus::new();
        corpus
            .item("0001-a.md", "id: \"0001\"\n")
            .item("0002-b.md", "id: \"0002\"\nexternal_id: \"0001\"\n");

        let failure = only_failure(corpus.resolve(&["0001"]));

        assert!(
            matches!(failure, TargetResolutionFailure::LocalCollision(_)),
            "{failure:?}"
        );
        assert_eq!(failure.exit_code(), exit_codes::USAGE);
        let message = failure.message();
        assert!(message.contains("0001-a.md"), "names file A: {message}");
        assert!(message.contains("0002-b.md"), "names file B: {message}");
    }

    #[test]
    fn a_target_that_is_several_items_external_id_is_ambiguous_external() {
        let corpus = RealCorpus::new();
        corpus
            .item("0001-a.md", "id: \"0001\"\nexternal_id: \"PP-1\"\n")
            .item("0002-b.md", "id: \"0002\"\nexternal_id: \"PP-1\"\n");

        let failure = only_failure(corpus.resolve(&["PP-1"]));

        assert!(
            matches!(failure, TargetResolutionFailure::AmbiguousExternal(_)),
            "{failure:?}"
        );
        assert_eq!(failure.exit_code(), exit_codes::USAGE);
    }

    #[test]
    fn a_target_matching_an_alias_names_the_retiring_item() {
        let corpus = RealCorpus::new();
        corpus
            .item("0001-a.md", "id: \"0001\"\nexternal_id: \"ENG-42\"\n")
            .item("ENG-7-b.md", "id: \"ENG-7\"\naliases: [\"ENG-42\"]\n");

        let failure = only_failure(corpus.resolve(&["ENG-42"]));

        assert!(
            matches!(failure, TargetResolutionFailure::RetiredAlias(_)),
            "{failure:?}"
        );
        assert_eq!(failure.exit_code(), exit_codes::USAGE);
        let message = failure.message();
        assert!(message.contains("ENG-42"), "names the alias: {message}");
        assert!(
            message.contains("ENG-7-b.md"),
            "names the retiring item: {message}"
        );
    }

    #[test]
    fn a_lowercase_key_target_resolves_its_item() {
        let corpus = RealCorpus::new();
        corpus.item("0002-b.md", "id: \"0002\"\nexternal_id: \"PP-787\"\n");

        assert_eq!(matched_ids(corpus.resolve(&["pp-787"])), vec!["0002"]);
    }

    #[test]
    fn a_remote_id_token_matches_through_the_items_external_id() {
        let corpus = RealCorpus::new();
        corpus.item("0002-b.md", "id: \"0002\"\nexternal_id: \"PP-787\"\n");

        assert_eq!(matched_ids(corpus.resolve(&["PP-787"])), vec!["0002"]);
    }

    #[test]
    fn two_tokens_naming_one_item_de_duplicate_to_a_single_entry() {
        let corpus = RealCorpus::new();
        corpus.item("0002-b.md", "id: \"0002\"\nexternal_id: \"PP-787\"\n");

        assert_eq!(
            matched_ids(corpus.resolve(&["0002", "PP-787"])),
            vec!["0002"],
            "the same item named twice collapses to a single slice entry"
        );
    }

    #[test]
    fn sync_discovery_includes_drafts() {
        let corpus = RealCorpus::new();
        corpus
            .item("0001-a.md", "id: \"0001\"\n")
            .item("drafts/draft-k7mq3x-b.md", "id: \"draft-k7mq3x\"\n");

        let ids: Vec<String> =
            corpus.items().into_iter().map(|item| item.id).collect();

        assert_eq!(ids, vec!["0001", "draft-k7mq3x"]);
    }

    #[test]
    fn an_own_external_id_token_reconciles_with_no_collision() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("0002"))];
        let local_path = canonical(&corpus[0]);
        let resolver = |_token: &str| RunOutcome::Resolved(local_path.clone());

        let matched = resolve_targets(&corpus, &["0002".to_owned()], &resolver)
            .expect("a token equal to its own file's external_id resolves");

        assert_eq!(matched.items.len(), 1);
        assert_eq!(matched.items[0].id, "0002");
    }

    #[test]
    fn an_ambiguous_local_id_fails_without_cascading_to_the_remote() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("0001"))];
        let resolver = |_token: &str| RunOutcome::Ambiguous(Vec::new());

        let failures =
            resolve_targets(&corpus, &["0001".to_owned()], &resolver)
                .expect_err("an ambiguous local id must fail");

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].exit_code(), exit_codes::USAGE);
    }

    #[test]
    fn an_out_of_directory_path_fails_without_cascading() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("0001"))];
        let resolver =
            |_token: &str| RunOutcome::OutsideWorkDir("outside".to_owned());

        let failures =
            resolve_targets(&corpus, &["0001".to_owned()], &resolver)
                .expect_err("an out-of-directory path must fail");

        assert_eq!(failures.len(), 1);
        assert_eq!(
            failures[0].exit_code(),
            exit_codes::RESOLVE_OUTSIDE_WORKDIR
        );
    }

    #[test]
    fn a_token_resolved_over_an_unlistable_corpus_fails_as_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("PP-787"))];
        let resolver =
            |_token: &str| RunOutcome::Unlistable("unreadable".to_owned());

        let failures =
            resolve_targets(&corpus, &["PP-787".to_owned()], &resolver)
                .expect_err("an unlistable corpus must fail the target");

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].exit_code(), exit_codes::ERROR);
    }

    #[test]
    fn a_no_local_match_token_becomes_a_remote_candidate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("PP-787"))];
        let resolver = |_token: &str| RunOutcome::NotFound("nope".to_owned());

        let outcome = resolve_targets(&corpus, &["9999".to_owned()], &resolver)
            .expect(
                "a no-local-match token is a remote candidate, not a failure",
            );

        assert!(outcome.items.is_empty());
        assert_eq!(outcome.remote_candidates.len(), 1);
        assert_eq!(outcome.remote_candidates[0].as_str(), "9999");
    }

    #[test]
    fn two_spellings_of_one_remote_only_token_collapse_to_one_candidate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("PP-787"))];
        let resolver = |_token: &str| RunOutcome::NotFound("nope".to_owned());

        let outcome = resolve_targets(
            &corpus,
            &["PP-999".to_owned(), "pp-999".to_owned()],
            &resolver,
        )
        .expect("both spellings are remote candidates");

        assert_eq!(
            outcome.remote_candidates.len(),
            1,
            "two spellings of one issue fold to a single candidate"
        );
    }

    #[test]
    fn every_failure_is_accumulated_with_usage_dominating_the_code() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("PP-787"))];
        let resolver =
            |_token: &str| RunOutcome::OutsideWorkDir("outside".to_owned());

        let failures = resolve_targets(
            &corpus,
            &["   ".to_owned(), "meta/outside.md".to_owned()],
            &resolver,
        )
        .expect_err("both tokens fail");

        assert_eq!(failures.len(), 2, "collect-all names every offender");
        assert_eq!(
            highest_precedence_code(&failures),
            exit_codes::USAGE,
            "a malformed token dominates an out-of-directory path"
        );
    }

    #[test]
    fn precedence_orders_usage_over_outside_over_not_found_over_retryable() {
        let failures = vec![
            TargetResolutionFailure::Indeterminate("retry".to_owned()),
            TargetResolutionFailure::Absent("gone".to_owned()),
        ];
        assert_eq!(
            highest_precedence_code(&failures),
            exit_codes::RESOLVE_NOT_FOUND,
            "an absent token dominates an indeterminate one by rank"
        );

        let with_outside = vec![
            TargetResolutionFailure::Absent("gone".to_owned()),
            TargetResolutionFailure::OutsideWorkDir("outside".to_owned()),
            TargetResolutionFailure::Malformed("blank".to_owned()),
        ];
        assert_eq!(
            highest_precedence_code(&with_outside),
            exit_codes::USAGE,
            "usage dominates outside-work-dir and not-found"
        );
    }

    fn found_pair(id: &str) -> (ExternalId, tracker::RemoteTimestamp) {
        (
            ExternalId::new(id.to_owned()),
            tracker::RemoteTimestamp::NotReported,
        )
    }

    #[test]
    fn partition_candidates_projects_found_ids_when_all_resolve() {
        let outcome = tracker::FetchOutcome {
            found: vec![found_pair("PP-1"), found_pair("PP-2")],
            absent: Vec::new(),
            indeterminate: Vec::new(),
            completeness: tracker::Completeness::Complete,
        };

        let found = partition_candidates(outcome).expect("an all-found batch");

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].as_str(), "PP-1");
    }

    #[test]
    fn partition_candidates_folds_absent_to_exit_three() {
        let outcome = tracker::FetchOutcome {
            found: vec![found_pair("PP-1")],
            absent: vec![ExternalId::new("PP-9".to_owned())],
            indeterminate: Vec::new(),
            completeness: tracker::Completeness::Complete,
        };

        let failures =
            partition_candidates(outcome).expect_err("an absent token fails");

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].exit_code(), exit_codes::RESOLVE_NOT_FOUND);
        assert!(failures[0].message().contains("PP-9"));
    }

    #[test]
    fn partition_candidates_folds_indeterminate_to_exit_seventy() {
        let outcome = tracker::FetchOutcome {
            found: Vec::new(),
            absent: Vec::new(),
            indeterminate: vec![ExternalId::new("PP-7".to_owned())],
            completeness: tracker::Completeness::Complete,
        };

        let failures = partition_candidates(outcome)
            .expect_err("an indeterminate token fails");

        assert_eq!(failures[0].exit_code(), exit_codes::RETRYABLE);
    }

    #[test]
    fn a_mixed_absent_and_indeterminate_batch_exits_three_by_rank() {
        let outcome = tracker::FetchOutcome {
            found: Vec::new(),
            absent: vec![ExternalId::new("PP-9".to_owned())],
            indeterminate: vec![ExternalId::new("PP-7".to_owned())],
            completeness: tracker::Completeness::Complete,
        };

        let failures =
            partition_candidates(outcome).expect_err("the mixed batch fails");

        assert_eq!(
            highest_precedence_code(&failures),
            exit_codes::RESOLVE_NOT_FOUND,
            "absent (3) dominates indeterminate (70) by rank, not order"
        );
    }

    #[test]
    fn a_whole_call_fetch_all_error_folds_to_indeterminate_exit_seventy() {
        let failures = super::confirm_remote_candidates(
            &FetchFailingTracker,
            &[ExternalId::new("PP-9".to_owned())],
        )
        .expect_err("a pre-flight fetch_all failure aborts");

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].exit_code(), exit_codes::RETRYABLE);
    }

    #[test]
    fn a_push_only_remote_only_candidate_aborts_before_the_tracker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("PP-787"))];
        let resolver = |_token: &str| RunOutcome::NotFound("nope".to_owned());

        let (mut out, mut err) = (std::io::sink(), std::io::sink());

        let outcome = build_selection(
            &mut super::Console::new(&mut out, &mut err),
            &corpus,
            &["PP-999".to_owned()],
            SyncDirection::PushOnly,
            &resolver,
        );

        assert!(
            outcome.is_err(),
            "a remote-only target under --push-only aborts before any tracker \
             contact"
        );
    }

    #[test]
    fn an_empty_target_token_is_a_usage_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("PP-787"))];
        let resolver = |_token: &str| RunOutcome::Invalid("unused".to_owned());

        let failures = resolve_targets(&corpus, &[String::new()], &resolver)
            .expect_err("an empty token must fail");

        assert!(matches!(failures[0], TargetResolutionFailure::Malformed(_)));
        assert_eq!(failures[0].exit_code(), exit_codes::USAGE);
    }

    #[test]
    fn no_targets_selects_the_whole_corpus() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", None)];
        let resolver = |_token: &str| RunOutcome::Invalid("unused".to_owned());

        let (mut out, mut err) = (std::io::sink(), std::io::sink());

        let selected = build_selection(
            &mut super::Console::new(&mut out, &mut err),
            &corpus,
            &[],
            SyncDirection::Bidirectional,
            &resolver,
        )
        .expect("no targets is a whole-corpus run");

        assert!(matches!(selected.scope, CorpusScope::WholeCorpus));
        assert!(selected.items.is_empty());
    }

    fn ok_render(section: &SectionDiff) -> String {
        format!("=== {} (- LOCAL / + REMOTE) ===\nbody\n\n", section.name)
    }

    fn conflict_dossier(id: &str, local_unreadable: bool) -> ConflictDossier {
        ConflictDossier {
            id: id.to_owned(),
            title: "Title".to_owned(),
            remote_title: None,
            local_modified: Some(1_700_000_000),
            remote_updated: RemoteTimestamp::Reported(
                "2026-07-01T00:00:00Z".to_owned(),
            ),
            sections: vec![SectionDiff {
                name: "(preamble)".to_owned(),
                local: "local".to_owned(),
                remote: "remote".to_owned(),
            }],
            unrenderable: local_unreadable
                .then_some(UnrenderableReason::LocalUnreadable),
        }
    }

    fn reported(id: &str, state: SyncState, action: Action) -> ReportedItem {
        ReportedItem {
            planned: PlannedAction::new(id.to_owned(), state, action),
            outcome: ItemOutcome::NotApplied,
            validation: None,
            kept_edits: None,
            leftover_marker: None,
            notice: None,
        }
    }

    fn failed(
        id: &str,
        state: SyncState,
        source: TrackerError,
    ) -> ReportedItem {
        ReportedItem {
            planned: PlannedAction::new(id.to_owned(), state, Action::Noop),
            outcome: ItemOutcome::Failed(ApplyError::Tracker {
                item_id: id.to_owned(),
                operation:
                    work_adapters::sync::operation::SyncOperation::Update,
                source,
            }),
            validation: None,
            kept_edits: None,
            leftover_marker: None,
            notice: None,
        }
    }

    #[test]
    fn a_linked_items_leftover_marker_is_a_warning_naming_it() {
        let report = report_of(vec![ReportedItem {
            leftover_marker: Some(LeftoverMarker {
                path: PathBuf::from("/s/pending-push/0001.json"),
                reason: "not writable".to_owned(),
            }),
            ..reported("0001", SyncState::Unsynced, Action::CreateFromLocal)
        }]);

        assert_eq!(
            leftover_marker_warnings(&report),
            vec!["warning: 0001: could not clear marker \
                 /s/pending-push/0001.json: not writable; the item is \
                 linked, so remove the marker by hand"
                .to_owned()]
        );
    }

    #[test]
    fn render_report_sorts_fixed_width_ids_numerically() {
        let report = RunReport {
            reported: vec![
                reported(
                    "0001",
                    SyncState::LocallyModified,
                    Action::Push(FieldDelivery::Send),
                ),
                reported("0002", SyncState::RemotelyModified, Action::Pull),
                reported("0003", SyncState::Conflict, Action::Prompt),
                reported("0004", SyncState::Conflict, Action::SkipConflict),
                reported("0005", SyncState::LocallyModified, Action::SkipDirty),
                failed(
                    "0006",
                    SyncState::LocallyModified,
                    TrackerError::Retryable {
                        detail: "rate limited".to_owned(),
                    },
                ),
                failed(
                    "0007",
                    SyncState::LocallyModified,
                    TrackerError::Terminal {
                        detail: "unsafe identifier".to_owned(),
                    },
                ),
                reported("0008", SyncState::Synced, Action::Noop),
                reported("0009", SyncState::RemoteAbsent, Action::Noop),
                reported("0010", SyncState::Indeterminate, Action::Noop),
                failed_with(
                    "0011",
                    ApplyError::RemoteUnrenderable {
                        item_id: "0011".to_owned(),
                        operation: SyncOperation::Update,
                        cause: tracker::Unrenderable {
                            detail: "panel".to_owned(),
                        },
                    },
                ),
                failed_with(
                    "0012",
                    ApplyError::LocalPullResidue {
                        item_id: "0012".to_owned(),
                        operation: SyncOperation::Update,
                        external_id: tracker::ExternalId::new(
                            "ENG-12".to_owned(),
                        ),
                    },
                ),
                ReportedItem {
                    outcome: ItemOutcome::UnsupportedRecipe,
                    ..reported("0013", SyncState::Indeterminate, Action::Noop)
                },
                record(
                    "0014",
                    ItemOutcome::Failed(
                        ApplyError::LocalChangedSinceClassified {
                            item_id: "0014".to_owned(),
                        },
                    ),
                ),
                reported(
                    "0015",
                    SyncState::LocallyModified,
                    Action::RecordBaseline,
                ),
            ],
            ..crate::test_support::run_report()
        };

        let golden = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/sync-report.golden"
        ))
        .expect("golden readable");

        assert_eq!(
            render_report(&crate::finished_run::FinishedRun::for_report(
                &report
            )),
            golden
        );
    }

    #[test]
    fn render_report_emits_the_summary_row_for_an_empty_corpus() {
        let report = RunReport {
            discovery: DiscoveryStatus::SkippedPushOnly,
            ..crate::test_support::run_report()
        };

        assert_eq!(
            render_report(&crate::finished_run::FinishedRun::for_report(
                &report
            )),
            "#\tdiscovery\tskipped\tpush-only\n#\tsummary\tsynced\t0"
        );
    }

    fn identity_row(
        id: &str,
        settled_id: &str,
        action: work::sync::IdentityAction,
        detail: &str,
        outcome: work_adapters::sync::identity_settlement::IdentityOutcome,
    ) -> work_adapters::sync::identity_settlement::IdentityRow {
        work_adapters::sync::identity_settlement::IdentityRow {
            id: id.to_owned(),
            settled_id: settled_id.to_owned(),
            action,
            detail: detail.to_owned(),
            outcome,
        }
    }

    #[test]
    fn a_stopped_run_prints_the_identity_changes_that_landed_before_its_note() {
        use work::promotion::Promotion;
        use work_adapters::promotion::PromotionOutcome;
        use work_adapters::promotion::PromotionRow;
        use work_adapters::sync::identity_settlement::IdentityOutcome;
        use work_adapters::sync::settled_run::SettledRunFailure;

        let promoted = PromotionRow {
            draft: "draft-k7mq3x".to_owned(),
            path: PathBuf::from("meta/work/drafts/draft-k7mq3x-title.md"),
            outcome: PromotionOutcome::Promoted(Promotion::Completed(
                ExternalId::new("ENG-43".to_owned()),
                SyncState::Synced,
            )),
            details: Vec::new(),
        };
        let failure = SettledRunFailure {
            error: work_adapters::sync::run::RunError::KeyedReadCapped,
            identity_applied: 2,
            identity: vec![identity_row(
                "PP-760",
                "ENG-42",
                work::sync::IdentityAction::KeyChanged,
                "PP-760->ENG-42",
                IdentityOutcome::Applied,
            )],
            promotions: vec![promoted.clone()],
        };

        assert_eq!(
            super::stopped_run_lines(&failure),
            vec![
                "PP-760\tkey-changed\t-\tPP-760->ENG-42".to_owned(),
                crate::promotion_report::promotion_line(&promoted),
                "#\tnote\tidentity-applied-before-refusal\t2".to_owned(),
            ]
        );
    }

    fn settled_report(
        identity: Vec<work_adapters::sync::identity_settlement::IdentityRow>,
        reported: Vec<ReportedItem>,
    ) -> RunReport {
        RunReport {
            identity,
            reported,
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        }
    }

    #[test]
    fn a_key_change_row_renders_old_and_new_keys() {
        use work::sync::IdentityAction;
        use work_adapters::sync::identity_settlement::IdentityOutcome;

        let report = settled_report(
            vec![identity_row(
                "PP-760",
                "ENG-42",
                IdentityAction::KeyChanged,
                "PP-760->ENG-42",
                IdentityOutcome::Applied,
            )],
            vec![reported(
                "ENG-42",
                SyncState::LocallyModified,
                Action::Push(FieldDelivery::Send),
            )],
        );

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        assert!(
            rendered.lines().any(|line| line
                == "PP-760\tkey-changed\tlocally-modified\tPP-760->ENG-42"),
            "{rendered}"
        );
    }

    #[test]
    fn a_not_found_row_is_a_four_column_record() {
        use work::sync::IdentityAction;
        use work_adapters::sync::identity_settlement::IdentityOutcome;

        let report = settled_report(
            vec![identity_row(
                "PP-76",
                "PP-76",
                IdentityAction::NotFound,
                "PP-76",
                IdentityOutcome::NotApplied,
            )],
            vec![reported("PP-76", SyncState::RemoteAbsent, Action::Noop)],
        );

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        let line = rendered
            .lines()
            .find(|line| line.starts_with("PP-76\t"))
            .expect("a row for the item");
        assert_eq!(line, "PP-76\tnot-found\tremote-absent\tPP-76");
        assert_eq!(line.split('\t').count(), 4);
    }

    #[test]
    fn each_id_appears_in_one_report_row() {
        use work::sync::IdentityAction;
        use work_adapters::sync::identity_settlement::IdentityOutcome;

        let report = settled_report(
            vec![identity_row(
                "0230",
                "0230",
                IdentityAction::KeyChanged,
                "PP-760->ENG-42",
                IdentityOutcome::Applied,
            )],
            vec![reported(
                "0230",
                SyncState::LocallyModified,
                Action::Push(FieldDelivery::Send),
            )],
        );

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        assert_eq!(
            rendered
                .lines()
                .filter(|line| line.starts_with("0230\t"))
                .count(),
            1,
            "{rendered}"
        );
    }

    #[test]
    fn key_changed_rows_survive_synced_row_suppression() {
        use work::sync::IdentityAction;
        use work_adapters::sync::identity_settlement::IdentityOutcome;

        let report = settled_report(
            vec![identity_row(
                "0230",
                "0230",
                IdentityAction::KeyChanged,
                "PP-760->ENG-42",
                IdentityOutcome::Applied,
            )],
            vec![reported("0230", SyncState::Synced, Action::Noop)],
        );

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        assert!(
            rendered.contains("0230\tkey-changed\tsynced\tPP-760->ENG-42"),
            "{rendered}"
        );
        assert!(!rendered.contains("#\tsummary\tsynced\t1"), "{rendered}");
    }

    #[test]
    fn a_refused_identity_row_renders_as_failed_with_its_reason() {
        use work::sync::IdentityAction;
        use work_adapters::sync::identity_settlement::IdentityOutcome;

        let report = settled_report(
            vec![identity_row(
                "PP-760",
                "PP-760",
                IdentityAction::KeyChanged,
                "PP-760->ENG-42",
                IdentityOutcome::Refused(
                    "cannot retire PP-760 for ENG-42: ENG-7-other.md\nholds it"
                        .to_owned(),
                ),
            )],
            Vec::new(),
        );

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        assert!(
            rendered.contains(
                "PP-760\tfailed\t-\tcannot retire PP-760 for ENG-42: \
                 ENG-7-other.md holds it"
            ),
            "{rendered}"
        );
    }

    #[test]
    fn a_key_change_collision_exits_unresolved() {
        use work::sync::IdentityAction;
        use work_adapters::sync::identity_settlement::IdentityOutcome;

        let report = settled_report(
            vec![identity_row(
                "PP-760",
                "PP-760",
                IdentityAction::KeyChanged,
                "PP-760->ENG-42",
                IdentityOutcome::Refused("collision".to_owned()),
            )],
            Vec::new(),
        );

        assert_eq!(
            super::exit_code_for_report(&report),
            exit_codes::UNRESOLVED
        );
    }

    #[test]
    fn deferred_items_are_noted() {
        let report = RunReport {
            deferred: 2,
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        };

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("#\tnote\tdeferred-to-next-run\t2"),);
    }

    #[test]
    fn an_incomplete_restore_names_both_ids_every_unrestored_path_the_recovery_directory_and_the_remedy(
    ) {
        use work::retirement::Retirement;
        use work::retirement::RetirementCause;
        use work::retirement::RetirementCauseKind;
        use work::retirement::RetirementFailure;
        use work_adapters::sync::run::RunError;

        struct NoConfig;

        impl ::config::ConfigAccess for NoConfig {
            fn get(
                &self,
                _key: &::config::Key,
                _level: Option<::config::Level>,
            ) -> Result<::config::Resolved, ::config::ConfigError> {
                Ok(::config::Resolved::Absent)
            }

            fn set(
                &self,
                _key: &::config::Key,
                _value: &str,
                _level: ::config::Level,
            ) -> Result<(), ::config::ConfigError> {
                unreachable!("reporting a failure never writes config")
            }
        }

        let retirement = Retirement {
            old_id: "PP-760",
            new_id: "ENG-42",
            new_external_id: Some("ENG-42"),
        };
        let recovery = Path::new(
            "/repo/.accelerator/state/retirement-recovery/PP-760--ENG-42",
        );
        let failure = RetirementFailure::RestoreIncomplete {
            cause: RetirementCause {
                path: PathBuf::from("/repo/meta/work/ENG-42-a.md"),
                kind: RetirementCauseKind::ChangedSinceSnapshot,
            },
            unrestored: vec![
                PathBuf::from("/repo/meta/plans/plan.md"),
                PathBuf::from("/repo/meta/work/0001-child.md"),
            ],
        };
        let error = RunError::RetirementIncomplete {
            message: failure.message(&retirement, recovery),
        };

        let (message, code) = super::run_error_outcome(
            &error,
            &super::RunErrorContext {
                config: &NoConfig,
                integration: "linear",
                max_pulls_source: None,
                max_pushes_source: None,
            },
        );

        assert_eq!(code, exit_codes::TERMINAL);
        for expected in [
            "retirement-incomplete",
            "PP-760",
            "ENG-42",
            "/repo/meta/plans/plan.md",
            "/repo/meta/work/0001-child.md",
            "/repo/.accelerator/state/retirement-recovery/PP-760--ENG-42",
            "restore these paths from version control",
        ] {
            assert!(message.contains(expected), "{expected}: {message}");
        }
    }

    #[test]
    fn an_unclassifiable_item_renders_a_class_keyword() {
        use work_adapters::sync::identity_settlement::IdentityOutcome;

        let report = RunReport {
            identity: vec![identity_row(
                "0001",
                "0001",
                work::sync::IdentityAction::KeyChanged,
                "ENG-1->ENG-2",
                IdentityOutcome::Applied,
            )],
            unclassifiable: vec![work::sync::Unclassifiable {
                id: "0001".to_owned(),
                reason: "could not\tread it".to_owned(),
            }],
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        };

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        assert!(
            rendered
                .lines()
                .any(|line| line == "0001\tfailed\t-\tretryable"),
            "{rendered}"
        );
        assert!(
            rendered
                .lines()
                .any(|line| line.starts_with("0001\tkey-changed")),
            "{rendered}"
        );
        assert!(!rendered.contains("could not"), "{rendered}");
        assert_eq!(
            super::failure_lines(&report),
            vec!["error: 0001: classify-local failed: could not read it"]
        );
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::RETRYABLE
        );
    }

    fn kept(id: &str, kept_since: Option<u64>) -> super::KeptCopy {
        super::KeptCopy {
            id: id.to_owned(),
            item: PathBuf::from(format!("/repo/meta/work/{id}.md")),
            copy: PathBuf::from(format!(
                "/repo/.accelerator/state/sync-recovery/1-k7mq3x-{id}/\
                 meta/work/{id}.md"
            )),
            directory: PathBuf::from(format!(
                "/repo/.accelerator/state/sync-recovery/1-k7mq3x-{id}"
            )),
            kept_since,
        }
    }

    fn review_of(entries: Vec<super::KeptEdits>) -> super::KeptEditsReview {
        super::KeptEditsReview { entries }
    }

    #[test]
    fn every_kept_copy_but_a_settled_one_gets_a_record_before_the_summary() {
        use super::KeptEdits;
        use super::Provenance;

        let report = report_with(DiscoveryStatus::Ran { found: 0 });
        let finished = crate::finished_run::FinishedRun {
            kept_edits: review_of(vec![
                KeptEdits::KeptThisRun(kept("0001", Some(1))),
                KeptEdits::KeptEarlier(kept("0002", Some(1))),
                KeptEdits::Settled {
                    kept: kept("0003", Some(1)),
                    recorded: corpus::store::RecordedCopy {
                        dir: PathBuf::from("sync-recovery/1-k7mq3x-0003"),
                        original: PathBuf::from("/repo/meta/work/0003.md"),
                        copy: PathBuf::from("meta/work/0003.md"),
                    },
                },
                KeptEdits::Unrecorded {
                    id: Some("0004".to_owned()),
                    directory: PathBuf::from("/state/sync-recovery/x-0004"),
                    provenance: Provenance::Unknown,
                },
                KeptEdits::Unrecorded {
                    id: None,
                    directory: PathBuf::from("/state/sync-recovery/stray"),
                    provenance: Provenance::Unknown,
                },
            ]),
            ..crate::finished_run::FinishedRun::for_report(&report)
        };

        let rendered = render_report(&finished);

        let kept_lines: Vec<&str> = rendered
            .lines()
            .filter(|line| line.starts_with("#\tkept-edits"))
            .collect();
        assert_eq!(
            kept_lines,
            vec![
                "#\tkept-edits\t0001\tthis-run\t/repo/meta/work/0001.md\t\
                 /repo/.accelerator/state/sync-recovery/1-k7mq3x-0001/meta/\
                 work/0001.md",
                "#\tkept-edits\t0002\tearlier\t/repo/meta/work/0002.md\t\
                 /repo/.accelerator/state/sync-recovery/1-k7mq3x-0002/meta/\
                 work/0002.md",
                "#\tkept-edits\t0004\tunrecorded\t-\t\
                 /state/sync-recovery/x-0004",
                "#\tkept-edits\t-\tunrecorded\t-\t/state/sync-recovery/stray",
            ]
        );
        assert!(
            rendered.ends_with("#\tsummary\tsynced\t0"),
            "the records precede the summary: {rendered}"
        );
    }

    #[test]
    fn each_kept_copy_note_names_what_to_do() {
        use super::KeptEdits;
        use super::Provenance;

        let notes = super::kept_edits_notes(&review_of(vec![
            KeptEdits::KeptThisRun(kept("0001", Some(1_700_000_000))),
            KeptEdits::KeptEarlier(kept("0002", Some(1_700_000_000))),
            KeptEdits::NotRemoved {
                kept: kept("0003", Some(1_600_000_000)),
                reason: "Permission denied".to_owned(),
            },
            KeptEdits::Unrecorded {
                id: Some("0004".to_owned()),
                directory: PathBuf::from("/state/sync-recovery/d4"),
                provenance: Provenance::Tracked,
            },
            KeptEdits::Unrecorded {
                id: Some("0005".to_owned()),
                directory: PathBuf::from("/state/sync-recovery/d5"),
                provenance: Provenance::TrackingUnknown,
            },
        ]));

        assert_eq!(
            notes,
            vec![
                "note: 0001: kept-edits: local edits kept at \
                 /repo/.accelerator/state/sync-recovery/1-k7mq3x-0001/meta/\
                 work/0001.md; merge them, then run accelerator work sync \
                 --target 0001 --discard-kept-edits 0001",
                "note: 0003: kept-edits: settled copy at \
                 /repo/.accelerator/state/sync-recovery/1-k7mq3x-0003 could \
                 not be removed: Permission denied",
                "note: 0004: kept-edits: /state/sync-recovery/d4 is tracked by \
                 version control and was not made by this tool; untrack it \
                 (git rm --cached -r /state/sync-recovery/d4 or jj file \
                 untrack /state/sync-recovery/d4), then sync",
                "note: 0005: kept-edits: could not confirm \
                 /state/sync-recovery/d5 is untracked; it is kept but not \
                 compared",
                "note: -: kept-edits-earlier: 2 earlier copies still kept \
                 (oldest since 2020-09-13T12:26:40Z) for 0002, 0003; discard \
                 each with accelerator work sync --target <id> \
                 --discard-kept-edits <id>",
            ]
        );
    }

    #[test]
    fn each_discard_says_what_became_of_it() {
        use super::KeptEdits;

        let earlier = || KeptEdits::KeptEarlier(kept("0001", Some(1)));
        let directory = "/repo/.accelerator/state/sync-recovery/1-k7mq3x-0001";
        let review = review_of(vec![
            KeptEdits::Discarding {
                entry: Box::new(earlier()),
                dir: PathBuf::from("sync-recovery/1-k7mq3x-0001"),
                item: None,
            },
            KeptEdits::Discarded {
                id: "0001".to_owned(),
                directory: PathBuf::from(directory),
            },
            KeptEdits::NotDiscarded {
                entry: Box::new(earlier()),
                reason: "Permission denied".to_owned(),
            },
            KeptEdits::NothingToDiscard {
                id: "0009".to_owned(),
            },
        ]);

        assert_eq!(
            super::kept_edits_notes(&review),
            vec![
                format!(
                    "note: 0001: kept-edits: {directory} would be discarded"
                ),
                format!("note: 0001: kept-edits: discarded {directory}"),
                format!(
                    "note: 0001: kept-edits: {directory} could not be \
                     discarded: Permission denied"
                ),
                "note: 0009: kept-edits: no kept copy to discard".to_owned(),
            ]
        );
        let report = report_with(DiscoveryStatus::Ran { found: 0 });
        let rendered = render_report(&crate::finished_run::FinishedRun {
            kept_edits: review,
            ..crate::finished_run::FinishedRun::for_report(&report)
        });
        assert_eq!(
            rendered
                .lines()
                .filter(|line| line.starts_with("#\tkept-edits\t0001\tearlier"))
                .count(),
            2,
            "a copy still kept is still recorded: {rendered}"
        );
    }

    #[test]
    fn earlier_copies_of_unknown_age_are_kept_since_an_unknown_date() {
        let notes = super::kept_edits_notes(&review_of(vec![
            super::KeptEdits::KeptEarlier(kept("0002", None)),
        ]));

        assert!(
            notes[0].contains("(oldest since an unknown date) for 0002"),
            "{notes:?}"
        );
    }

    fn report_with(discovery: DiscoveryStatus) -> RunReport {
        RunReport {
            discovery,
            ..crate::test_support::run_report()
        }
    }

    mod promotion_rows {
        use std::path::PathBuf;

        use tracker::ExternalId;
        use work::identity::IdentityField;
        use work::promotion::NotPromoted;
        use work::promotion::Promotion;
        use work::retirement::RetirementCause;
        use work::retirement::RetirementCauseKind;
        use work::retirement::RetirementFailure;
        use work::retirement::RetirementRefusal;
        use work::sync::Action;
        use work::sync::FieldDelivery;
        use work::sync::SyncState;
        use work_adapters::promotion::Detail;
        use work_adapters::promotion::DetailSource;
        use work_adapters::promotion::PromotionOutcome;
        use work_adapters::promotion::PromotionRow;
        use work_adapters::sync::run::DiscoveryStatus;
        use work_adapters::sync::run::RunReport;

        use super::super::exit_code_for_report;
        use super::super::render_report;
        use super::report_with;
        use super::reported;
        use crate::exit_codes;

        fn key() -> ExternalId {
            ExternalId::new("PP-900".to_owned())
        }

        fn draft_path(draft: &str) -> PathBuf {
            PathBuf::from(format!("meta/work/drafts/{draft}-title.md"))
        }

        fn row(draft: &str, outcome: PromotionOutcome) -> PromotionRow {
            PromotionRow {
                draft: draft.to_owned(),
                path: draft_path(draft),
                outcome,
                details: Vec::new(),
            }
        }

        fn not_promoted(draft: &str, reason: NotPromoted) -> PromotionRow {
            PromotionRow {
                details: vec![Detail {
                    source: DetailSource::Item,
                    path: draft_path(draft),
                }],
                ..row(
                    draft,
                    PromotionOutcome::NotPromoted {
                        reason,
                        held_key: None,
                    },
                )
            }
        }

        fn promoting(rows: Vec<PromotionRow>) -> RunReport {
            RunReport {
                promotions: rows,
                ..report_with(DiscoveryStatus::Ran { found: 0 })
            }
        }

        fn lines(report: &RunReport) -> Vec<String> {
            render_report(&crate::finished_run::FinishedRun::for_report(report))
                .lines()
                .map(str::to_owned)
                .collect()
        }

        #[test]
        fn a_not_promoted_draft_is_reported_with_its_reason() {
            let report = promoting(vec![not_promoted(
                "draft-aaaaaa",
                NotPromoted::TrackerUnreachable,
            )]);

            assert!(lines(&report).contains(
                &"draft-aaaaaa\tnot-promoted\tunsynced\ttracker-unreachable"
                    .to_owned()
            ));
        }

        #[test]
        fn promotion_rows_are_four_column_records() {
            let report = promoting(vec![
                row("draft-aaaaaa", PromotionOutcome::Previewed),
                row(
                    "draft-bbbbbb",
                    PromotionOutcome::Promoted(Promotion::Completed(
                        key(),
                        SyncState::Synced,
                    )),
                ),
                row(
                    "draft-cccccc",
                    PromotionOutcome::Promoted(Promotion::Completed(
                        ExternalId::new("PP-901".to_owned()),
                        SyncState::LocallyModified,
                    )),
                ),
                row(
                    "draft-dddddd",
                    PromotionOutcome::Promoted(Promotion::AlreadyDone(
                        ExternalId::new("PP-902".to_owned()),
                    )),
                ),
            ]);

            let rendered = lines(&report);

            for expected in [
                "draft-aaaaaa\tpromote\tunsynced\t-",
                "draft-bbbbbb\tpromoted\tsynced\tPP-900",
                "draft-cccccc\tpromoted\tlocally-modified\tPP-901",
                "draft-dddddd\talready-promoted\tsynced\tPP-902",
            ] {
                assert!(
                    rendered.contains(&expected.to_owned()),
                    "{expected} in {rendered:?}"
                );
            }
        }

        #[test]
        fn detail_lines_trail_the_report_and_carry_their_row_id() {
            let mut report = promoting(vec![
                not_promoted("draft-bbbbbb", NotPromoted::TrackerUnreachable),
                not_promoted(
                    "draft-aaaaaa",
                    NotPromoted::Refused(RetirementRefusal::KeyLinked {
                        holder: PathBuf::from("meta/work/0002-legacy.md"),
                    }),
                ),
            ]);
            report.promotions[1].details = vec![Detail {
                source: DetailSource::Holder,
                path: PathBuf::from("meta/work/0002-legacy.md"),
            }];
            report.reported = vec![reported(
                "0001",
                SyncState::LocallyModified,
                Action::Push(FieldDelivery::Send),
            )];

            let rendered = lines(&report);

            let first_hash = rendered
                .iter()
                .position(|line| line.starts_with('#'))
                .unwrap();
            assert!(
                rendered[first_hash..]
                    .iter()
                    .all(|line| line.starts_with('#')),
                "{rendered:?}"
            );
            let details: Vec<&String> = rendered
                .iter()
                .filter(|line| line.starts_with("#\tdetail\t"))
                .collect();
            assert_eq!(
                details,
                vec![
                    "#\tdetail\tdraft-aaaaaa\tholder\tmeta/work/0002-legacy.md",
                    "#\tdetail\tdraft-bbbbbb\titem\t\
                     meta/work/drafts/draft-bbbbbb-title.md",
                ]
            );
        }

        #[test]
        fn an_adopt_that_will_conflict_reports_promoted_conflict_with_a_detail_line(
        ) {
            let report = promoting(vec![PromotionRow {
                details: vec![Detail {
                    source: DetailSource::Item,
                    path: PathBuf::from("meta/work/PP-900-title.md"),
                }],
                ..row(
                    "draft-aaaaaa",
                    PromotionOutcome::Promoted(Promotion::Completed(
                        key(),
                        SyncState::Conflict,
                    )),
                )
            }]);

            let rendered = lines(&report);

            assert!(rendered.contains(
                &"draft-aaaaaa\tpromoted\tconflict\tPP-900".to_owned()
            ));
            assert!(rendered.contains(
                &"#\tdetail\tdraft-aaaaaa\titem\tmeta/work/PP-900-title.md"
                    .to_owned()
            ));
        }

        #[test]
        fn a_path_to_restore_names_vcs_or_its_recovery_directory() {
            let mut report = promoting(vec![not_promoted(
                "draft-aaaaaa",
                NotPromoted::TrackerUnreachable,
            )]);
            report.promotions[0].details = vec![
                Detail {
                    source: DetailSource::Vcs,
                    path: PathBuf::from("meta/plans/a.md"),
                },
                Detail {
                    source: DetailSource::Recovery {
                        location: PathBuf::from(".accelerator/state/r"),
                    },
                    path: PathBuf::from("meta/plans/b.md"),
                },
            ];

            let rendered = lines(&report);

            assert!(rendered.contains(
                &"#\tdetail\tdraft-aaaaaa\tvcs\tmeta/plans/a.md".to_owned()
            ));
            assert!(rendered.contains(
                &"#\tdetail\tdraft-aaaaaa\trecovery\tmeta/plans/b.md\t\
                  .accelerator/state/r"
                    .to_owned()
            ));
        }

        fn cause() -> RetirementCause {
            RetirementCause {
                path: PathBuf::from("meta/work/x.md"),
                kind: RetirementCauseKind::ChangedSinceSnapshot,
            }
        }

        fn every_reason() -> Vec<NotPromoted> {
            let holder = PathBuf::from("meta/work/0001-holder.md");
            vec![
                NotPromoted::TrackerUnreachable,
                NotPromoted::CreateOutcomeUnknown,
                NotPromoted::RequestRejected {
                    detail: String::new(),
                },
                NotPromoted::EarlierAttemptUnconfirmed,
                NotPromoted::Refused(RetirementRefusal::IdTaken {
                    holder: holder.clone(),
                    field: IdentityField::Id,
                }),
                NotPromoted::Refused(RetirementRefusal::KeyLinked {
                    holder: holder.clone(),
                }),
                NotPromoted::Refused(RetirementRefusal::TargetExists(holder)),
                NotPromoted::Refused(RetirementRefusal::ItemNotFound(
                    "draft-aaaaaa".to_owned(),
                )),
                NotPromoted::AdoptedIssueMissing(key()),
                NotPromoted::AdoptConflictsWithRecordedKey { recorded: key() },
                NotPromoted::RetirementFailed(RetirementFailure::RolledBack {
                    cause: cause(),
                }),
                NotPromoted::RetirementFailed(
                    RetirementFailure::RestoreIncomplete {
                        cause: cause(),
                        unrestored: Vec::new(),
                    },
                ),
                NotPromoted::ReadBackFailed(key()),
                NotPromoted::RecordUnwritable {
                    key: Some(key()),
                    detail: String::new(),
                },
                NotPromoted::RecordUnwritable {
                    key: None,
                    detail: String::new(),
                },
            ]
        }

        #[test]
        fn every_not_promoted_reason_is_reported_under_its_keyword() {
            for reason in every_reason() {
                let keyword = reason.keyword();
                let report =
                    promoting(vec![not_promoted("draft-aaaaaa", reason)]);

                assert!(
                    lines(&report).contains(&format!(
                        "draft-aaaaaa\tnot-promoted\tunsynced\t{keyword}"
                    )),
                    "{keyword}"
                );
            }
        }

        fn exit_for(reasons: Vec<NotPromoted>) -> u8 {
            exit_code_for_report(&promoting(
                reasons
                    .into_iter()
                    .enumerate()
                    .map(|(index, reason)| {
                        not_promoted(&format!("draft-{index:06}"), reason)
                    })
                    .collect(),
            ))
        }

        #[test]
        fn exit_code_reflects_not_promoted_drafts() {
            let unreachable = || NotPromoted::TrackerUnreachable;
            let duplicate = || NotPromoted::EarlierAttemptUnconfirmed;
            let rejected = || NotPromoted::RequestRejected {
                detail: String::new(),
            };
            let incomplete = || {
                NotPromoted::RetirementFailed(
                    RetirementFailure::RestoreIncomplete {
                        cause: cause(),
                        unrestored: Vec::new(),
                    },
                )
            };
            for (reasons, expected) in [
                (vec![unreachable()], exit_codes::RETRYABLE),
                (vec![rejected(), unreachable()], exit_codes::REJECTED),
                (
                    vec![duplicate(), rejected(), unreachable()],
                    exit_codes::UNRESOLVED,
                ),
                (
                    vec![incomplete(), duplicate(), rejected()],
                    exit_codes::TERMINAL,
                ),
                (
                    vec![NotPromoted::AdoptedIssueMissing(key())],
                    exit_codes::UNRESOLVED,
                ),
            ] {
                assert_eq!(exit_for(reasons.clone()), expected, "{reasons:?}");
            }
        }

        #[test]
        fn a_promoted_draft_leaves_the_exit_clean() {
            let report = promoting(vec![row(
                "draft-aaaaaa",
                PromotionOutcome::Promoted(Promotion::Completed(
                    key(),
                    SyncState::Conflict,
                )),
            )]);

            assert_eq!(exit_code_for_report(&report), exit_codes::CLEAN);
        }

        /// `create` reports an unreachable tracker, an earlier unconfirmed
        /// create and an unwritable first record as saving a draft or a loud
        /// terminal; `exit_codes` documents each difference.
        const DOCUMENTED_DIFFERENCES: &[&str] = &[
            "tracker-unreachable",
            "possible-duplicate",
            "record-unwritable",
        ];

        #[test]
        fn each_outcome_exits_alike_from_every_command_or_is_documented() {
            for reason in every_reason() {
                let from_create = reason.create_outcome().exit_code();
                let from_sync = reason.exit_code(true);
                let documented = DOCUMENTED_DIFFERENCES
                    .contains(&reason.keyword())
                    && !matches!(
                        reason,
                        NotPromoted::RecordUnwritable { key: Some(_), .. }
                    );
                assert!(
                    from_create == from_sync || documented,
                    "{} exits {from_create} from create and {from_sync} from \
                     sync",
                    reason.keyword()
                );
            }
        }

        #[test]
        fn drafts_skipped_by_no_promote_do_not_affect_the_exit_code() {
            let report = RunReport {
                reported: vec![reported(
                    "draft-aaaaaa",
                    SyncState::Unsynced,
                    Action::Noop,
                )],
                ..report_with(DiscoveryStatus::Ran { found: 0 })
            };

            assert_eq!(exit_code_for_report(&report), exit_codes::CLEAN);
        }
    }

    #[test]
    fn render_report_emits_each_discovery_status_line() {
        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report_with(DiscoveryStatus::Ran { found: 3 })
        ))
        .contains("#\tdiscovery\tran\tfound=3"));
        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report_with(DiscoveryStatus::SkippedPushOnly)
        ))
        .contains("#\tdiscovery\tskipped\tpush-only"));
        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report_with(DiscoveryStatus::SkippedTargeted)
        ))
        .contains("#\tdiscovery\tskipped\ttargeted"));
        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report_with(DiscoveryStatus::TargetedPull { attempted: 2 })
        ))
        .contains("#\tdiscovery\ttargeted-pull\t2"));
        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report_with(DiscoveryStatus::SkippedPendingPush)
        ))
        .contains("#\tdiscovery\tskipped\tpending-push"));
        let failed =
            render_report(&crate::finished_run::FinishedRun::for_report(
                &report_with(DiscoveryStatus::Failed {
                    detail: "connection refused".to_owned(),
                }),
            ));
        assert!(
            failed.contains("#\tdiscovery\tfailed\tconnection refused"),
            "{failed}"
        );
    }

    #[test]
    fn a_failed_discovery_detail_is_flattened_to_one_record() {
        let rendered =
            render_report(&crate::finished_run::FinishedRun::for_report(
                &report_with(DiscoveryStatus::Failed {
                    detail: "line one\tsplit\nline two\r".to_owned(),
                }),
            ));
        let discovery = rendered
            .lines()
            .find(|line| line.starts_with("#\tdiscovery\tfailed"))
            .expect("a failed discovery line");
        assert_eq!(
            discovery, "#\tdiscovery\tfailed\tline one split line two ",
            "tabs, newlines and carriage returns are collapsed to spaces"
        );
    }

    #[test]
    fn single_line_strips_record_breaking_whitespace() {
        assert_eq!(super::single_line("a\tb\nc\rd"), "a b c d");
    }

    fn unconfigured(detail: &str) -> TrackerError {
        TrackerError::Unconfigured {
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn render_report_renders_an_unconfigured_failure() {
        let report = RunReport {
            reported: vec![failed(
                "0001",
                SyncState::LocallyModified,
                unconfigured("no team in scope"),
            )],
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        };

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tlocally-modified\tunconfigured"));
    }

    fn rejected(detail: &str) -> TrackerError {
        TrackerError::Rejected {
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn a_rejected_create_from_local_renders_a_rejected_failed_row() {
        let report = RunReport {
            reported: vec![failed(
                "0001",
                SyncState::Unsynced,
                rejected("jira create: the body has a table"),
            )],
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        };

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tunsynced\trejected"));
    }

    #[test]
    fn sync_exit_code_ranks_rejected_between_awaiting_human_and_unconfigured() {
        let awaiting = || reported("0001", SyncState::Conflict, Action::Prompt);
        let rejection =
            || failed("0002", SyncState::Unsynced, rejected("bad body"));
        let refused =
            || failed("0003", SyncState::LocallyModified, unconfigured(""));
        let exit_for = |items: Vec<ReportedItem>| {
            super::exit_code_for_report(&RunReport {
                reported: items,
                ..report_with(DiscoveryStatus::Ran { found: 0 })
            })
        };

        assert_eq!(
            exit_for(vec![awaiting(), rejection(), refused()]),
            super::exit_codes::UNRESOLVED
        );
        assert_eq!(
            exit_for(vec![rejection(), refused()]),
            super::exit_codes::REJECTED
        );
    }

    fn refused_order(id: &str) -> ApplyError {
        ApplyError::OrderRefused {
            item_id: id.to_owned(),
            order: PendingPushOrder::Create,
            refusal: Box::new(
                work_adapters::sync::apply::OrderRefusal::NoSuchItem {
                    id: id.to_owned(),
                },
            ),
        }
    }

    #[test]
    fn a_refused_order_renders_a_failed_row_and_exits_75() {
        let report = RunReport {
            refused_orders: vec![refused_order("0009")],
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        };

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        assert!(
            rendered
                .lines()
                .any(|line| line == "0009\tfailed\t-\torder-refused"),
            "{rendered}"
        );
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::REJECTED
        );
        assert_eq!(
            super::failure_lines(&report),
            ["error: 0009: create failed: no work item 0009 in this run"]
        );
    }

    #[test]
    fn exit_code_for_report_ranks_outcomes() {
        let terminal = || {
            failed(
                "0001",
                SyncState::LocallyModified,
                TrackerError::Terminal {
                    detail: String::new(),
                },
            )
        };
        let awaiting = || reported("0002", SyncState::Conflict, Action::Prompt);
        let refused =
            || failed("0003", SyncState::LocallyModified, unconfigured(""));
        let retryable = || {
            failed(
                "0004",
                SyncState::LocallyModified,
                TrackerError::Retryable {
                    detail: String::new(),
                },
            )
        };
        let exit_for = |items: Vec<ReportedItem>| {
            super::exit_code_for_report(&RunReport {
                reported: items,
                ..report_with(DiscoveryStatus::Ran { found: 0 })
            })
        };

        for (items, expected) in [
            (
                vec![terminal(), awaiting(), refused(), retryable()],
                super::exit_codes::TERMINAL,
            ),
            (
                vec![awaiting(), refused(), retryable()],
                super::exit_codes::UNRESOLVED,
            ),
            (
                vec![refused(), retryable()],
                super::exit_codes::UNCONFIGURED,
            ),
            (vec![retryable()], super::exit_codes::RETRYABLE),
        ] {
            assert_eq!(exit_for(items), expected);
        }
    }

    #[test]
    fn exit_code_for_report_ranks_local_failures_by_landing() {
        let conflict = || reported("0002", SyncState::Conflict, Action::Prompt);
        let exit_for = |items: Vec<ReportedItem>| {
            super::exit_code_for_report(&report_of(items))
        };

        for (items, expected) in [
            (
                vec![io_failed("0001", ItemProgress::NothingLanded)],
                super::exit_codes::RETRYABLE,
            ),
            (
                vec![io_failed("0001", ItemProgress::MayHaveLanded)],
                super::exit_codes::TERMINAL,
            ),
            (
                vec![
                    conflict(),
                    io_failed("0001", ItemProgress::MayHaveLanded),
                ],
                super::exit_codes::TERMINAL,
            ),
            (
                vec![
                    conflict(),
                    io_failed("0001", ItemProgress::NothingLanded),
                ],
                super::exit_codes::UNRESOLVED,
            ),
        ] {
            assert_eq!(exit_for(items), expected);
        }
    }

    fn unclassifiable(id: &str) -> work::sync::Unclassifiable {
        work::sync::Unclassifiable {
            id: id.to_owned(),
            reason: "could not read it".to_owned(),
        }
    }

    #[test]
    fn an_unclassifiable_item_fails_a_targeted_run_and_yields_to_a_conflict() {
        let targeted = RunReport {
            scope: CorpusScope::Targeted,
            unclassifiable: vec![unclassifiable("0001")],
            ..crate::test_support::run_report()
        };
        let beside_a_conflict = RunReport {
            reported: vec![reported(
                "0002",
                SyncState::Conflict,
                Action::Prompt,
            )],
            unclassifiable: vec![unclassifiable("0001")],
            ..crate::test_support::run_report()
        };

        let failures: Vec<_> = targeted.failures().collect();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].severity, FailureSeverity::Error);
        assert_eq!(
            super::exit_code_for_report(&targeted),
            super::exit_codes::RETRYABLE
        );
        assert_eq!(
            super::exit_code_for_report(&beside_a_conflict),
            super::exit_codes::UNRESOLVED
        );
    }

    #[test]
    fn a_failed_finalise_is_a_retryable_run_level_failure() {
        let failed = RunReport {
            finalisation: Finalisation::Failed("disk full".to_owned()),
            ..crate::test_support::run_report()
        };
        let preview = RunReport {
            mode: RunMode::Preview,
            finalisation: Finalisation::NotAttempted,
            ..crate::test_support::run_report()
        };

        assert_eq!(
            super::failure_lines(&failed),
            vec!["error: -: baseline-set failed: disk full"]
        );
        assert_eq!(
            super::exit_code_for_report(&failed),
            super::exit_codes::RETRYABLE
        );
        assert!(super::failure_lines(&preview).is_empty());
        assert_eq!(
            super::exit_code_for_report(&preview),
            super::exit_codes::CLEAN
        );
    }

    #[test]
    fn every_failed_row_gets_an_error_line_naming_its_operation() {
        let report = report_of(vec![
            io_failed("0001", ItemProgress::NothingLanded),
            failed_with(
                "0002",
                ApplyError::RemoteBodyUnavailable {
                    item_id: "0002".to_owned(),
                    operation: SyncOperation::Pull,
                },
            ),
            failed(
                "0003",
                SyncState::LocallyModified,
                TrackerError::Retryable {
                    detail: "offline".to_owned(),
                },
            ),
            ReportedItem {
                outcome: ItemOutcome::UnsupportedRecipe,
                ..reported("0004", SyncState::Synced, Action::Noop)
            },
        ]);

        let lines = super::failure_lines(&report);

        assert_eq!(lines.len(), 3, "{lines:?}");
        assert_eq!(lines[0], "error: 0001: write-local failed: disk full");
        assert!(
            lines[1].starts_with(
                "error: 0002: pull failed: the remote description was not \
                 fetched"
            ),
            "{lines:?}"
        );
        assert!(
            lines[2].starts_with("error: 0003: update failed: "),
            "{lines:?}"
        );
    }

    #[test]
    fn a_preview_says_would_fail() {
        let report = RunReport {
            mode: RunMode::Preview,
            finalisation: Finalisation::NotAttempted,
            ..report_of(vec![failed_with(
                "0002",
                ApplyError::RemoteBodyUnavailable {
                    item_id: "0002".to_owned(),
                    operation: SyncOperation::Pull,
                },
            )])
        };

        let lines = super::failure_lines(&report);

        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].starts_with("error: 0002: pull would fail: "),
            "{lines:?}"
        );
    }

    #[test]
    fn a_newline_in_a_tracker_cause_cannot_forge_an_error_line() {
        let report = report_of(vec![failed(
            "0001",
            SyncState::LocallyModified,
            TrackerError::Retryable {
                detail: "offline\nerror: -: baseline-set failed:\rforged"
                    .to_owned(),
            },
        )]);

        let lines = super::failure_lines(&report);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].lines().count(), 1, "{lines:?}");
        assert!(!lines[0].contains('\r'), "{lines:?}");
    }

    #[test]
    fn a_failed_item_with_an_identity_row_keeps_its_failed_row() {
        use work_adapters::sync::identity_settlement::IdentityOutcome;

        let report = RunReport {
            identity: vec![identity_row(
                "0001",
                "0001",
                work::sync::IdentityAction::KeyChanged,
                "ENG-1->ENG-2",
                IdentityOutcome::Applied,
            )],
            reported: vec![io_failed("0001", ItemProgress::NothingLanded)],
            ..crate::test_support::run_report()
        };

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        assert!(
            rendered
                .lines()
                .any(|line| line.starts_with("0001\tkey-changed")),
            "{rendered}"
        );
        assert!(
            rendered.lines().any(
                |line| line == "0001\tfailed\tremotely-modified\tretryable"
            ),
            "{rendered}"
        );
    }

    fn inventory_holding(
        reasons: Vec<UnidentifiedReason>,
    ) -> work::inventory::CorpusInventory {
        work::inventory::CorpusInventory {
            ids: std::iter::once("0001".to_owned()).collect(),
            unidentified: reasons
                .into_iter()
                .enumerate()
                .map(|(index, reason)| work::inventory::Unidentified {
                    path: PathBuf::from(format!(
                        "meta/work/000{}-x.md",
                        index + 2
                    )),
                    reason,
                })
                .collect(),
        }
    }

    fn reporting(
        scope: CorpusScope,
        reasons: Vec<UnidentifiedReason>,
    ) -> RunReport {
        RunReport {
            scope,
            inventory: inventory_holding(reasons),
            ..crate::test_support::run_report()
        }
    }

    fn unreadable() -> UnidentifiedReason {
        UnidentifiedReason::Unreadable("Permission denied".to_owned())
    }

    fn malformed() -> UnidentifiedReason {
        UnidentifiedReason::Malformed(
            work::inventory::MalformedCause::UnclosedFence,
        )
    }

    #[test]
    fn every_unidentified_file_gets_a_record() {
        let report = reporting(
            CorpusScope::WholeCorpus,
            vec![
                unreadable(),
                UnidentifiedReason::Undecodable,
                malformed(),
                UnidentifiedReason::NoFrontmatter,
                UnidentifiedReason::NoId,
            ],
        );

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        for (index, kind) in [
            "unreadable",
            "undecodable",
            "malformed",
            "no-frontmatter",
            "no-id",
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                rendered.lines().any(|line| line
                    == format!(
                        "#\tunidentified\tmeta/work/000{}-x.md\t{kind}",
                        index + 2
                    )),
                "{rendered}"
            );
        }
    }

    #[test]
    fn unusable_files_fail_a_whole_corpus_run_by_what_is_wrong() {
        let report = reporting(
            CorpusScope::WholeCorpus,
            vec![
                unreadable(),
                UnidentifiedReason::Undecodable,
                malformed(),
                UnidentifiedReason::NoId,
            ],
        );

        assert_eq!(
            super::failure_lines(&report),
            vec![
                "error: meta/work/0002-x.md: read-local failed: Permission denied",
                "error: meta/work/0003-x.md: read-local failed: not UTF-8",
                "error: meta/work/0004-x.md: identify-local failed: frontmatter \
                 opened but never closed",
            ]
        );
    }

    #[test]
    fn exit_code_for_report_ranks_unusable_files() {
        for (scope, reasons, expected) in [
            (
                CorpusScope::WholeCorpus,
                vec![UnidentifiedReason::NoId],
                super::exit_codes::CLEAN,
            ),
            (
                CorpusScope::WholeCorpus,
                vec![unreadable()],
                super::exit_codes::RETRYABLE,
            ),
            (
                CorpusScope::WholeCorpus,
                vec![UnidentifiedReason::Undecodable],
                super::exit_codes::REJECTED,
            ),
            (
                CorpusScope::WholeCorpus,
                vec![malformed()],
                super::exit_codes::REJECTED,
            ),
            (
                CorpusScope::Targeted,
                vec![unreadable(), malformed()],
                super::exit_codes::CLEAN,
            ),
        ] {
            assert_eq!(
                super::exit_code_for_report(&reporting(scope, reasons.clone())),
                expected,
                "{scope:?} {reasons:?}"
            );
        }
    }

    #[test]
    fn a_targeted_run_only_warns_about_unusable_files() {
        let report = reporting(CorpusScope::Targeted, vec![unreadable()]);

        assert_eq!(
            super::failure_lines(&report),
            vec!["warning: meta/work/0002-x.md: read-local failed: Permission denied"]
        );
    }

    fn failed_with(id: &str, error: ApplyError) -> ReportedItem {
        ReportedItem {
            planned: PlannedAction::new(
                id.to_owned(),
                SyncState::RemotelyModified,
                Action::Pull,
            ),
            outcome: ItemOutcome::Failed(error),
            validation: None,
            kept_edits: None,
            leftover_marker: None,
            notice: None,
        }
    }

    fn report_of(items: Vec<ReportedItem>) -> RunReport {
        RunReport {
            reported: items,
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        }
    }

    fn record(id: &str, outcome: ItemOutcome) -> ReportedItem {
        ReportedItem {
            planned: PlannedAction::new(
                id.to_owned(),
                SyncState::Synced,
                Action::RecordBaseline,
            ),
            outcome,
            validation: None,
            kept_edits: None,
            leftover_marker: None,
            notice: None,
        }
    }

    #[test]
    fn a_failed_baseline_record_reports_a_retryable_row_and_exits_70() {
        let report = report_of(vec![record(
            "0001",
            ItemOutcome::Failed(ApplyError::LocalChangedSinceClassified {
                item_id: "0001".to_owned(),
            }),
        )]);

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );
        assert!(
            rendered.contains("0001\tfailed\tsynced\tretryable"),
            "{rendered}"
        );
        assert!(!rendered.contains("#\tsummary\tsynced\t1"), "{rendered}");
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::RETRYABLE
        );
    }

    #[test]
    fn a_recorded_locally_modified_item_reports_a_record_baseline_row() {
        let report = report_of(vec![ReportedItem {
            planned: PlannedAction::new(
                "0001".to_owned(),
                SyncState::LocallyModified,
                Action::RecordBaseline,
            ),
            outcome: ItemOutcome::Applied,
            validation: None,
            kept_edits: None,
            leftover_marker: None,
            notice: None,
        }]);

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );

        assert!(
            rendered.contains("0001\trecord-baseline\tlocally-modified\t-"),
            "{rendered}"
        );
        assert!(!rendered.contains("#\tsummary"), "{rendered}");
    }

    #[test]
    fn a_recorded_baseline_counts_as_synced_with_no_row() {
        let report = report_of(vec![record("0001", ItemOutcome::Applied)]);

        let rendered = render_report(
            &crate::finished_run::FinishedRun::for_report(&report),
        );
        assert!(!rendered.contains("0001\t"), "{rendered}");
        assert!(rendered.contains("#\tsummary\tsynced\t1"), "{rendered}");
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::CLEAN
        );
    }

    fn io_failed(id: &str, progress: ItemProgress) -> ReportedItem {
        failed_with(
            id,
            ApplyError::Io {
                item_id: id.to_owned(),
                operation: SyncOperation::WriteLocal,
                progress,
                detail: "disk full".to_owned(),
            },
        )
    }

    #[test]
    fn a_local_io_failure_after_a_landing_reports_terminal_and_exits_71() {
        let report =
            report_of(vec![io_failed("0001", ItemProgress::MayHaveLanded)]);

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tremotely-modified\tterminal"));
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::TERMINAL
        );
    }

    #[test]
    fn a_local_io_failure_before_anything_landed_reports_retryable_and_exits_70(
    ) {
        let report =
            report_of(vec![io_failed("0001", ItemProgress::NothingLanded)]);

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tremotely-modified\tretryable"));
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::RETRYABLE
        );
    }

    #[test]
    fn a_remote_body_unavailable_failure_reports_retryable_and_exits_70() {
        let report = report_of(vec![failed_with(
            "0001",
            ApplyError::RemoteBodyUnavailable {
                item_id: "0001".to_owned(),
                operation: SyncOperation::Pull,
            },
        )]);

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tremotely-modified\tretryable"));
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::RETRYABLE
        );
    }

    #[test]
    fn a_refused_local_write_reports_its_row_and_exits_75() {
        let report = report_of(vec![failed_with(
            "0001",
            ApplyError::LocalWriteRefused {
                item_id: "0001".to_owned(),
                detail: "refusing to write through an unsafe path".to_owned(),
            },
        )]);

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tremotely-modified\tlocal-write-refused"));
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::REJECTED
        );
    }

    #[test]
    fn items_under_an_unknown_recipe_are_named_on_stderr() {
        let untouched = |id: &str| ReportedItem {
            outcome: ItemOutcome::UnsupportedRecipe,
            ..reported(id, SyncState::Indeterminate, Action::Noop)
        };

        assert_eq!(
            super::unsupported_recipe_warning(&report_of(vec![
                untouched("0012"),
                reported("0020", SyncState::Synced, Action::Noop),
                untouched("0031"),
            ])),
            Some(
                "warning: 2 item(s) carry a sync baseline written by a newer \
                 plugin release, so they were left untouched (0012, 0031); \
                 upgrade the plugin to sync them"
                    .to_owned()
            )
        );
        assert_eq!(super::unsupported_recipe_warning(&report_of(vec![])), None);
    }

    #[test]
    fn an_unparseable_item_reports_an_unparseable_row_and_exits_75() {
        let report = report_of(vec![failed_with(
            "0001",
            ApplyError::LocalUnparseable {
                item_id: "0001".to_owned(),
                operation: SyncOperation::ParseLocal,
                detail: "no frontmatter".to_owned(),
            },
        )]);

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tremotely-modified\tunparseable"));
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::REJECTED
        );
    }

    #[test]
    fn a_pull_residue_refusal_reports_pull_residue_and_exits_75() {
        let report = report_of(vec![failed_with(
            "0001",
            ApplyError::LocalPullResidue {
                item_id: "0001".to_owned(),
                operation: SyncOperation::Update,
                external_id: tracker::ExternalId::new("ENG-1".to_owned()),
            },
        )]);

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tremotely-modified\tpull-residue"));
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::REJECTED
        );
    }

    #[test]
    fn an_unrenderable_pull_exits_75() {
        let report = report_of(vec![failed_with(
            "0001",
            ApplyError::RemoteUnrenderable {
                item_id: "0001".to_owned(),
                operation: SyncOperation::Pull,
                cause: tracker::Unrenderable {
                    detail: "panel".to_owned(),
                },
            },
        )]);

        assert!(render_report(&crate::finished_run::FinishedRun::for_report(
            &report
        ))
        .contains("0001\tfailed\tremotely-modified\tunrenderable"));
        assert_eq!(
            super::exit_code_for_report(&report),
            super::exit_codes::REJECTED
        );
        assert_eq!(super::exit_codes::REJECTED, 75);
    }

    #[test]
    fn a_failed_discovery_exits_retryable_and_the_others_are_clean() {
        assert_eq!(
            super::exit_code_for_report(&report_with(
                DiscoveryStatus::Failed {
                    detail: "boom".to_owned(),
                }
            )),
            super::exit_codes::RETRYABLE
        );
        assert_eq!(
            super::exit_code_for_report(&report_with(DiscoveryStatus::Ran {
                found: 0
            })),
            super::exit_codes::CLEAN
        );
        assert_eq!(
            super::exit_code_for_report(&report_with(
                DiscoveryStatus::SkippedPushOnly
            )),
            super::exit_codes::CLEAN
        );
    }

    fn tracker_scheme() -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: corpus::TRACKER_TOKEN.to_owned(),
            key: None,
        }
    }

    #[test]
    fn a_conflict_dossier_is_written_for_a_tracker_keyed_item() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dossiers = vec![
            conflict_dossier("ENG-42", false),
            conflict_dossier("MY_PROJ-7", false),
            conflict_dossier("0230", false),
            conflict_dossier("../ENG-1", false),
        ];

        let (mut out, mut err) = (std::io::sink(), std::io::sink());

        super::persist_dossiers(
            &mut super::Console::new(&mut out, &mut err),
            &corpus_adapters::FileCorpusStore::new(dir.path()),
            &dossiers,
            dir.path(),
            &tracker_scheme(),
            &ok_render,
        );

        assert_eq!(
            md_files(dir.path()),
            vec!["0230.md", "ENG-42.md", "MY_PROJ-7.md"]
        );
    }

    #[test]
    fn a_conflict_dossier_is_written_for_a_draft() {
        let dir = tempfile::tempdir().expect("tempdir");

        let (mut out, mut err) = (std::io::sink(), std::io::sink());

        super::persist_dossiers(
            &mut super::Console::new(&mut out, &mut err),
            &corpus_adapters::FileCorpusStore::new(dir.path()),
            &[conflict_dossier("draft-k7mq3x", false)],
            dir.path(),
            &tracker_scheme(),
            &ok_render,
        );

        assert_eq!(md_files(dir.path()), vec!["draft-k7mq3x.md"]);
    }

    fn md_files(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("dir readable")
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                (path.extension().and_then(std::ffi::OsStr::to_str)
                    == Some("md"))
                .then(|| entry.file_name().to_string_lossy().into_owned())
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn persist_dossiers_writes_safe_ids_and_skips_unsafe_ones() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dossiers = vec![
            conflict_dossier("0001", false),
            conflict_dossier("0002", true),
            conflict_dossier("../evil", false),
        ];

        let (mut out, mut err) = (std::io::sink(), std::io::sink());

        super::persist_dossiers(
            &mut super::Console::new(&mut out, &mut err),
            &corpus_adapters::FileCorpusStore::new(dir.path()),
            &dossiers,
            dir.path(),
            &scheme(),
            &ok_render,
        );

        assert_eq!(md_files(dir.path()), vec!["0001.md", "0002.md"]);
        let renderable =
            std::fs::read_to_string(dir.path().join("0001.md")).unwrap();
        assert!(renderable.contains("status: renderable"), "{renderable}");
        let unrenderable =
            std::fs::read_to_string(dir.path().join("0002.md")).unwrap();
        assert!(
            unrenderable.contains("status: unrenderable"),
            "{unrenderable}"
        );
    }

    #[test]
    fn prepare_conflicts_dir_writes_a_config_independent_ignore() {
        let dir = tempfile::tempdir().expect("tempdir");
        let conflicts = dir.path().join("somewhere").join("conflicts");

        let (mut out, mut err) = (std::io::sink(), std::io::sink());

        super::persist_conflict_dossiers(
            &mut super::Console::new(&mut out, &mut err),
            &corpus_adapters::FileCorpusStore::new(dir.path()),
            &conflicts,
            &[conflict_dossier("0001", false)],
            &scheme(),
            &ok_render,
        );

        let ignore = std::fs::read_to_string(conflicts.join(".gitignore"))
            .expect("the directory-local .gitignore is written");
        assert!(ignore.contains('*'), "{ignore}");
        assert!(conflicts.join("0001.md").exists());
    }

    #[test]
    fn the_stale_clear_removes_only_canonical_dossiers_and_artefacts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let conflicts = dir.path().join("conflicts");
        std::fs::create_dir_all(&conflicts).expect("mkdir");
        std::fs::write(conflicts.join("0001.md"), "stale").expect("seed");
        std::fs::write(conflicts.join("notes.md"), "mine").expect("seed");
        std::fs::write(
            conflicts.join(format!("{}sweep", kernel::TEMP_PREFIX)),
            "artefact",
        )
        .expect("seed");

        let (mut out, mut err) = (std::io::sink(), std::io::sink());

        super::persist_conflict_dossiers(
            &mut super::Console::new(&mut out, &mut err),
            &corpus_adapters::FileCorpusStore::new(dir.path()),
            &conflicts,
            &[conflict_dossier("0002", false)],
            &scheme(),
            &ok_render,
        );

        assert!(
            conflicts.join("notes.md").exists(),
            "a user's own notes.md must survive the stale-clear"
        );
        assert!(
            !conflicts.join("0001.md").exists(),
            "a resolved conflict's dossier is cleared"
        );
        assert!(
            !conflicts
                .join(format!("{}sweep", kernel::TEMP_PREFIX))
                .exists(),
            "a stray write artefact is swept"
        );
        assert!(conflicts.join("0002.md").exists());
    }

    #[test]
    fn each_unreadable_promotion_record_is_named_in_a_warning() {
        let dir = tempfile::tempdir().expect("tempdir");
        let record = dir.path().join("linear/pending-push/draft-k7mq3x.json");
        std::fs::create_dir_all(record.parent().expect("parent"))
            .expect("mkdir");
        std::fs::write(&record, "{").expect("write");
        let store = corpus_adapters::FileCorpusStore::new(dir.path());
        let records =
            work_adapters::promotion_records::FilePromotionRecords::new(
                dir.path(),
                "linear",
                &store,
            );

        let warnings = super::unreadable_promotion_record_warnings(&records);

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].contains(&record.display().to_string()),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_fail_closed_prepare_writes_nothing_and_clears_nothing() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().expect("tempdir");
        let conflicts = dir.path().join("conflicts");
        std::fs::create_dir_all(&conflicts).expect("mkdir");
        std::fs::write(conflicts.join("0001.md"), "prior").expect("seed");
        // A read-only directory with no `.gitignore` cannot have one created,
        // so `prepare_conflicts_dir` fails before it clears anything.
        std::fs::set_permissions(
            &conflicts,
            std::fs::Permissions::from_mode(0o555),
        )
        .expect("chmod");

        let (mut out, mut err) = (std::io::sink(), std::io::sink());

        super::persist_conflict_dossiers(
            &mut super::Console::new(&mut out, &mut err),
            &corpus_adapters::FileCorpusStore::new(dir.path()),
            &conflicts,
            &[conflict_dossier("0002", false)],
            &scheme(),
            &ok_render,
        );

        let readable = std::fs::set_permissions(
            &conflicts,
            std::fs::Permissions::from_mode(0o755),
        );
        assert!(readable.is_ok());

        assert!(
            conflicts.join("0001.md").exists(),
            "a fail-closed prepare must not destroy the prior run's dossiers"
        );
        assert!(
            !conflicts.join("0002.md").exists(),
            "no dossier is written when the ignore cannot be guaranteed"
        );
    }

    // --- run_sync end-to-end over a stub tracker registry -------------------
    //
    // `accelerator-work` is bin-only, so these in-crate tests are the only seam
    // that can exercise the post-credential `fetch_all` gate without live
    // Linear/Jira. `run_sync_with` returns an opaque `ExitCode`, so they assert
    // the observable effects — files written, tracker calls made, and what the
    // captured console printed — while the exit-code chain is proven by the
    // pure-logic tests above.

    use std::cell::RefCell;
    use std::process::ExitCode;
    use std::rc::Rc;

    use tracker::RemoteIssue;
    use tracker::RemoteTracker;
    use tracker_test_support::Call;
    use tracker_test_support::RecordingTracker;

    use corpus_adapters::CommonMarkBody;
    use corpus_adapters::FileCorpusStore;
    use work_adapters::sync::baseline_store::BaselineStore;
    use work_adapters::sync::files::SyncStore;

    use crate::cli::SyncArgs;
    use crate::finaliser::FinishedRun;
    use crate::finaliser::NoFinaliser;
    use crate::finaliser::RunFinaliser;
    use crate::test_support::StubRegistry;
    use crate::tracker_registry::SelectionError;
    use crate::tracker_registry::TrackerRegistry;

    /// A tracker whose `fetch_all` fails pre-flight — the whole-call `Err` path
    /// `RecordingTracker` cannot produce. Only `fetch_all` is ever called on it.
    struct FetchFailingTracker;

    impl tracker::BodyDialect for FetchFailingTracker {
        fn is_wire_document(&self, _body: &str) -> bool {
            false
        }
    }

    impl RemoteTracker for FetchFailingTracker {
        fn fetch_all(
            &self,
            _ids: &[ExternalId],
        ) -> Result<tracker::FetchOutcome, TrackerError> {
            Err(TrackerError::Retryable {
                detail: "unresolvable pre-flight".to_owned(),
            })
        }

        fn create(
            &self,
            _title: &str,
            _body: &str,
            _kind: &str,
        ) -> Result<ExternalId, TrackerError> {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn update(
            &self,
            _id: &ExternalId,
            _title: Option<&str>,
            _body: &str,
        ) -> Result<(), TrackerError> {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn retitle(
            &self,
            _id: &ExternalId,
            _title: &str,
        ) -> Result<(), TrackerError> {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn show(&self, _id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn locate(
            &self,
            _id: &ExternalId,
        ) -> Result<tracker::Located, TrackerError> {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn search(
            &self,
            _scope: &tracker::SearchScope,
        ) -> Result<tracker::Discovery, TrackerError> {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn resolve_scope(
            &self,
            _scope: &tracker::SearchScope,
        ) -> Result<tracker::SearchScope, tracker::ScopeError> {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn enumerate_visible_entities(
            &self,
        ) -> Result<Vec<tracker::VisibleEntity>, tracker::TrackerError>
        {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn preview_create(
            &self,
            _kind: &str,
        ) -> Result<tracker::CreatePreview, TrackerError> {
            unimplemented!("not exercised by the fetch_all failure path")
        }

        fn validate_update(
            &self,
            _id: &ExternalId,
            _title: Option<&str>,
            _body: Option<&str>,
        ) -> tracker::ValidationOutcome {
            unimplemented!("not exercised by the fetch_all failure path")
        }
    }

    /// Delegates to a recording tracker, noting at each `fetch_all`
    /// whether the identity-run lockdir exists.
    struct LockWatchingTracker {
        inner: tracker_test_support::SharedTracker,
        lockdir: PathBuf,
        held_at_fetch: Rc<RefCell<Vec<bool>>>,
    }

    impl tracker::BodyDialect for LockWatchingTracker {
        fn local_body(
            &self,
            description: &str,
        ) -> Result<String, tracker::Unrenderable> {
            self.inner.local_body(description)
        }

        fn as_pushed<'a>(&self, body: &'a str) -> std::borrow::Cow<'a, str> {
            self.inner.as_pushed(body)
        }

        fn rejects(&self, body: &str) -> bool {
            self.inner.rejects(body)
        }

        fn is_wire_document(&self, body: &str) -> bool {
            self.inner.is_wire_document(body)
        }
    }

    impl RemoteTracker for LockWatchingTracker {
        fn fetch_all(
            &self,
            ids: &[ExternalId],
        ) -> Result<tracker::FetchOutcome, TrackerError> {
            self.held_at_fetch.borrow_mut().push(self.lockdir.is_dir());
            self.inner.fetch_all(ids)
        }

        fn create(
            &self,
            title: &str,
            body: &str,
            kind: &str,
        ) -> Result<ExternalId, TrackerError> {
            self.inner.create(title, body, kind)
        }

        fn update(
            &self,
            id: &ExternalId,
            title: Option<&str>,
            body: &str,
        ) -> Result<(), TrackerError> {
            self.inner.update(id, title, body)
        }

        fn retitle(
            &self,
            id: &ExternalId,
            title: &str,
        ) -> Result<(), TrackerError> {
            self.inner.retitle(id, title)
        }

        fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
            self.inner.show(id)
        }

        fn locate(
            &self,
            id: &ExternalId,
        ) -> Result<tracker::Located, TrackerError> {
            self.inner.locate(id)
        }

        fn search(
            &self,
            scope: &tracker::SearchScope,
        ) -> Result<tracker::Discovery, TrackerError> {
            self.inner.search(scope)
        }

        fn resolve_scope(
            &self,
            scope: &tracker::SearchScope,
        ) -> Result<tracker::SearchScope, tracker::ScopeError> {
            self.inner.resolve_scope(scope)
        }

        fn enumerate_visible_entities(
            &self,
        ) -> Result<Vec<tracker::VisibleEntity>, tracker::TrackerError>
        {
            self.inner.enumerate_visible_entities()
        }

        fn preview_create(
            &self,
            kind: &str,
        ) -> Result<tracker::CreatePreview, TrackerError> {
            self.inner.preview_create(kind)
        }

        fn validate_update(
            &self,
            id: &ExternalId,
            title: Option<&str>,
            body: Option<&str>,
        ) -> tracker::ValidationOutcome {
            self.inner.validate_update(id, title, body)
        }
    }

    struct LockWatchingRegistry {
        tracker: Rc<RecordingTracker>,
        lockdir: PathBuf,
        held_at_fetch: Rc<RefCell<Vec<bool>>>,
    }

    impl TrackerRegistry for LockWatchingRegistry {
        fn resolve(
            &self,
            _name: &str,
        ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
            Ok(Box::new(LockWatchingTracker {
                inner: tracker_test_support::SharedTracker(Rc::clone(
                    &self.tracker,
                )),
                lockdir: self.lockdir.clone(),
                held_at_fetch: Rc::clone(&self.held_at_fetch),
            }))
        }
    }

    #[test]
    fn sync_refuses_while_a_rekey_is_in_progress_and_says_run_work_rekey() {
        let dir = sync_repo();
        work_file(dir.path(), "0001", Some("ENG-1"));
        crate::test_support::rekey_in_progress(dir.path());
        let before = crate::test_support::meta_tree(dir.path());
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let code = drive_sync(dir.path(), &tracker, &sync_args(Vec::new()));

        assert_eq!(code, ExitCode::from(exit_codes::UNRESOLVED));
        assert!(tracker.calls().is_empty());
        assert_eq!(crate::test_support::meta_tree(dir.path()), before);
    }

    #[test]
    fn a_proposed_rekey_plan_does_not_block_sync() {
        let dir = sync_repo();
        crate::test_support::rekey_proposed(dir.path());
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let code = drive_sync(dir.path(), &tracker, &sync_args(Vec::new()));

        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn sync_refuses_while_a_run_plan_is_unreadable() {
        let dir = sync_repo();
        work_file(dir.path(), "0001", Some("ENG-1"));
        crate::test_support::rekey_plan_unreadable(dir.path());
        let before = crate::test_support::meta_tree(dir.path());
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let code = drive_sync(dir.path(), &tracker, &sync_args(Vec::new()));

        assert_eq!(code, ExitCode::from(exit_codes::UNRESOLVED));
        assert!(tracker.calls().is_empty());
        assert_eq!(crate::test_support::meta_tree(dir.path()), before);
    }

    #[test]
    fn sync_holds_the_identity_run_lock_across_its_pulls() {
        let dir = sync_repo();
        work_file(dir.path(), "0001", Some("ENG-1"));
        let held_at_fetch = Rc::new(RefCell::new(Vec::new()));
        let registry = LockWatchingRegistry {
            tracker: Rc::new(RecordingTracker::holding(Vec::new())),
            lockdir: dir
                .path()
                .join(".accelerator/state/locks/identity-run.lockdir"),
            held_at_fetch: Rc::clone(&held_at_fetch),
        };
        let composed = config_adapters::compose(
            dir.path(),
            config_adapters::LegacyPolicy::Reject,
        )
        .expect("compose the test config");

        let _ = super::run_sync(
            dir.path(),
            &composed.service,
            &sync_args(Vec::new()),
            &registry,
            &NoFinaliser,
        );

        let held = held_at_fetch.borrow();
        assert!(!held.is_empty(), "the run never read the tracker");
        assert!(held.iter().all(|held| *held), "{held:?}");
    }

    fn sync_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        init_sync_repo(dir.path());
        dir
    }

    fn init_sync_repo(root: &Path) {
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .expect("git invocation");
        };
        git(&["init", "-q"]);
        git(&["config", "user.name", "Test User"]);
        git(&["config", "user.email", "test@example.com"]);
        std::fs::create_dir_all(root.join("meta/work")).expect("mkdir");
        std::fs::create_dir_all(root.join(".accelerator")).expect("mkdir");
        std::fs::write(
            root.join(".accelerator/config.md"),
            "---\nwork:\n  integration: jira\n---\n",
        )
        .expect("write config");
    }

    /// Whether the run persisted a baseline for the integration — proof the
    /// create-from-remote sequence completed through its final baseline write,
    /// not just that a file was authored.
    fn baseline_written(dir: &Path) -> bool {
        std::fs::read_to_string(
            dir.join(".accelerator/state/integrations/jira/last-sync.json"),
        )
        .is_ok()
    }

    fn work_file(dir: &Path, id: &str, external: Option<&str>) {
        let external_line = external
            .map(|value| format!("external_id: \"{value}\"\n"))
            .unwrap_or_default();
        std::fs::write(
            dir.join(format!("meta/work/{id}-title.md")),
            format!("---\nid: \"{id}\"\n{external_line}---\n\nBody\n"),
        )
        .expect("write work item");
    }

    fn sync_args(targets: Vec<String>) -> SyncArgs {
        SyncArgs {
            push_only: false,
            pull_only: false,
            preview: false,
            resolutions: Vec::new(),
            per_item_reads: false,
            max_pulls: None,
            max_pushes: None,
            allow_unbounded: false,
            no_promote: false,
            targets,
            discard_kept_edits: Vec::new(),
        }
    }

    fn drive_sync(
        dir: &Path,
        tracker: &Rc<RecordingTracker>,
        args: &SyncArgs,
    ) -> ExitCode {
        drive_sync_finalising(dir, tracker, args, &NoFinaliser)
    }

    fn drive_sync_finalising(
        dir: &Path,
        tracker: &Rc<RecordingTracker>,
        args: &SyncArgs,
        finaliser: &dyn RunFinaliser,
    ) -> ExitCode {
        capture_sync_finalising(dir, tracker, args, finaliser).code
    }

    struct CapturedRun {
        out: String,
        err: String,
        code: ExitCode,
    }

    fn drive_sync_captured(
        dir: &Path,
        tracker: &Rc<RecordingTracker>,
        args: &SyncArgs,
    ) -> CapturedRun {
        capture_sync_finalising(dir, tracker, args, &NoFinaliser)
    }

    fn capture_sync_finalising(
        dir: &Path,
        tracker: &Rc<RecordingTracker>,
        args: &SyncArgs,
        finaliser: &dyn RunFinaliser,
    ) -> CapturedRun {
        capture_sync_over(
            dir,
            tracker,
            args,
            finaliser,
            super::stores::StoreFactories::production(),
        )
    }

    fn capture_sync_over(
        dir: &Path,
        tracker: &Rc<RecordingTracker>,
        args: &SyncArgs,
        finaliser: &dyn RunFinaliser,
        stores: super::stores::StoreFactories<'_>,
    ) -> CapturedRun {
        let composed = config_adapters::compose(
            dir,
            config_adapters::LegacyPolicy::Reject,
        )
        .expect("compose the test config");
        let registry = StubRegistry(Rc::clone(tracker));
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = super::run_sync_with(
            dir,
            &composed.service,
            args,
            &mut super::SyncSeams {
                registry: &registry,
                finaliser,
                console: super::Console::new(&mut out, &mut err),
                stores,
            },
        );
        CapturedRun {
            out: String::from_utf8(out).expect("stdout is UTF-8"),
            err: String::from_utf8(err).expect("stderr is UTF-8"),
            code,
        }
    }

    fn tracker_sync_repo_with_a_draft() -> tempfile::TempDir {
        let dir = sync_repo();
        std::fs::write(
            dir.path().join(".accelerator/config.md"),
            "---\nwork:\n  integration: jira\n  id_pattern: \"{tracker}\"\n---\n",
        )
        .expect("write config");
        std::fs::create_dir_all(dir.path().join("meta/work/drafts"))
            .expect("mkdir");
        std::fs::write(
            dir.path().join("meta/work/drafts/draft-aaaaaa-title.md"),
            "---\nid: \"draft-aaaaaa\"\ntitle: \"Title\"\nkind: \"task\"\n\
             ---\n\n# draft-aaaaaa: Title\n",
        )
        .expect("write draft");
        dir
    }

    fn creates(tracker: &RecordingTracker) -> usize {
        tracker
            .calls()
            .iter()
            .filter(|call| matches!(call, Call::Create { .. }))
            .count()
    }

    #[test]
    fn no_promote_maps_onto_the_sync_request() {
        for (no_promote, expected_creates) in [(false, 1), (true, 0)] {
            let dir = tracker_sync_repo_with_a_draft();
            let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

            drive_sync(
                dir.path(),
                &tracker,
                &SyncArgs {
                    no_promote,
                    ..sync_args(Vec::new())
                },
            );

            assert_eq!(creates(&tracker), expected_creates, "{no_promote}");
        }
    }

    #[test]
    fn no_promote_exits_zero_with_drafts_pending() {
        let dir = tracker_sync_repo_with_a_draft();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let code = drive_sync(
            dir.path(),
            &tracker,
            &SyncArgs {
                no_promote: true,
                ..sync_args(Vec::new())
            },
        );

        assert_eq!(code, ExitCode::SUCCESS);
        assert!(dir
            .path()
            .join("meta/work/drafts/draft-aaaaaa-title.md")
            .exists());
    }

    #[test]
    fn an_unreachable_tracker_leaves_the_draft_and_exits_retryable() {
        let dir = tracker_sync_repo_with_a_draft();
        let tracker =
            Rc::new(RecordingTracker::holding(Vec::new()).failing_create(
                TrackerError::Retryable {
                    detail: "connection refused".to_owned(),
                },
            ));

        let code = drive_sync(dir.path(), &tracker, &sync_args(Vec::new()));

        assert_eq!(code, ExitCode::from(exit_codes::RETRYABLE));
        assert!(dir
            .path()
            .join("meta/work/drafts/draft-aaaaaa-title.md")
            .exists());
    }

    fn create_pushed(
        dir: &Path,
        tracker: &Rc<RecordingTracker>,
    ) -> crate::create::RunOutcome {
        let composed = config_adapters::compose(
            dir,
            config_adapters::LegacyPolicy::Reject,
        )
        .expect("compose the test config");
        let templates = composed.store.with_plugin_root(Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        ));
        crate::create::run(
            dir,
            &composed.service,
            &templates,
            &crate::create::CreateArgs {
                title: "Round trip".to_owned(),
                kind: "task".to_owned(),
                priority: "low".to_owned(),
                status: "ready".to_owned(),
                parent: None,
                tags: Vec::new(),
                blocks: Vec::new(),
                blocked_by: Vec::new(),
                derived_from: Vec::new(),
                relates_to: Vec::new(),
                source: None,
                project: None,
                author: Some("A Tester".to_owned()),
                producer: "create-work-item".to_owned(),
                body_file: None,
                push: true,
                dry_run: false,
            },
            &StubRegistry(Rc::clone(tracker)),
        )
    }

    fn recorded_remote_hash(dir: &Path) -> Option<String> {
        let baseline: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                dir.join(".accelerator/state/integrations/jira/last-sync.json"),
            )
            .expect("the create recorded a baseline"),
        )
        .expect("the baseline is JSON");
        baseline["items"]
            .as_object()
            .and_then(|items| items.values().next())
            .and_then(|entry| entry["remote_hash"].as_str())
            .filter(|hash| !hash.is_empty())
            .map(str::to_owned)
    }

    #[test]
    fn a_create_then_sync_reports_synced() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        assert!(matches!(
            create_pushed(dir.path(), &tracker),
            crate::create::RunOutcome::Created { path: Some(_), .. }
        ));
        let calls_after_create = tracker.calls().len();

        let code = drive_sync(dir.path(), &tracker, &sync_args(Vec::new()));

        assert_eq!(code, ExitCode::SUCCESS);
        let mutated =
            tracker.calls()[calls_after_create..].iter().any(|call| {
                matches!(call, Call::Create { .. } | Call::Update { .. })
            });
        assert!(
            !mutated,
            "a synced item pushes nothing: {:?}",
            tracker.calls()
        );
    }

    #[test]
    fn a_create_whose_read_back_failed_syncs_as_synced_and_gains_a_remote_hash()
    {
        let dir = sync_repo();
        let creating =
            Rc::new(RecordingTracker::holding(Vec::new()).failing_show(
                ExternalId::new("REC-1".to_owned()),
                TrackerError::Retryable {
                    detail: "the read-back was lost".to_owned(),
                },
            ));
        let path = match create_pushed(dir.path(), &creating) {
            crate::create::RunOutcome::Created { path, .. } => path,
            _ => None,
        }
        .expect("the create writes the local item");
        assert_eq!(recorded_remote_hash(dir.path()), None);
        let (title, body, kind) = creating
            .calls()
            .into_iter()
            .find_map(|call| match call {
                Call::Create { title, body, kind } => Some((title, body, kind)),
                _ => None,
            })
            .expect("the create reached the tracker");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        tracker
            .create(&title, &body, &kind)
            .expect("the tracker holds the same issue");
        let calls_before_sync = tracker.calls().len();
        let local_before = std::fs::read(&path).expect("the item exists");

        let code = drive_sync(dir.path(), &tracker, &sync_args(Vec::new()));

        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            !tracker.calls()[calls_before_sync..].iter().any(|call| {
                matches!(call, Call::Create { .. } | Call::Update { .. })
            }),
            "{:?}",
            tracker.calls()
        );
        assert_eq!(
            std::fs::read(&path).expect("the item exists"),
            local_before
        );
        assert!(recorded_remote_hash(dir.path()).is_some());
    }

    /// Keeps what the run handed it, and can report a failure of its own.
    #[derive(Default)]
    struct RecordingFinaliser {
        corpus: RefCell<Vec<String>>,
        imports: RefCell<Vec<String>>,
        complaint: Option<&'static str>,
    }

    impl RunFinaliser for RecordingFinaliser {
        fn finalise(
            &self,
            run: &FinishedRun<'_>,
            diagnostics: &mut dyn std::io::Write,
        ) {
            self.corpus.borrow_mut().extend(
                run.corpus_external_ids
                    .iter()
                    .map(|id| id.as_str().to_owned()),
            );
            self.imports.borrow_mut().extend(
                run.applied_imports()
                    .iter()
                    .map(|id| id.as_str().to_owned()),
            );
            if let Some(complaint) = self.complaint {
                let _ = writeln!(diagnostics, "{complaint}");
            }
        }
    }

    #[test]
    fn the_finaliser_receives_the_corpus_external_ids() {
        let dir = sync_repo();
        work_file(dir.path(), "0292", Some("PP-869"));
        work_file(dir.path(), "PROJ-0042", Some("ENG-12"));
        work_file(dir.path(), "0001", None);
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let finaliser = RecordingFinaliser::default();

        drive_sync_finalising(
            dir.path(),
            &tracker,
            &sync_args(Vec::new()),
            &finaliser,
        );

        let mut corpus = finaliser.corpus.borrow().clone();
        corpus.sort();
        assert_eq!(corpus, vec!["ENG-12", "PP-869"], "no local id");
    }

    #[test]
    fn an_applied_import_reaches_the_finaliser() {
        let dir = sync_repo();
        let tracker = Rc::new(
            RecordingTracker::holding(vec![(
                ExternalId::new("ENG-7".to_owned()),
                issue("Imported\nRemote body"),
            )])
            .discovering(
                vec![(
                    ExternalId::new("ENG-7".to_owned()),
                    tracker::RemoteTimestamp::Reported(
                        "2026-01-01T00:00:00Z".to_owned(),
                    ),
                )],
                true,
            ),
        );
        let finaliser = RecordingFinaliser::default();

        drive_sync_finalising(
            dir.path(),
            &tracker,
            &sync_args(Vec::new()),
            &finaliser,
        );

        assert_eq!(*finaliser.imports.borrow(), vec!["ENG-7"]);
    }

    #[test]
    fn a_failed_import_adds_no_synced_team() {
        let dir = sync_repo();
        let tracker = Rc::new(
            RecordingTracker::holding(Vec::new())
                .failing_show(
                    ExternalId::new("ENG-7".to_owned()),
                    TrackerError::Retryable {
                        detail: "boom".to_owned(),
                    },
                )
                .discovering(
                    vec![(
                        ExternalId::new("ENG-7".to_owned()),
                        tracker::RemoteTimestamp::NotReported,
                    )],
                    true,
                ),
        );
        let finaliser = RecordingFinaliser::default();

        drive_sync_finalising(
            dir.path(),
            &tracker,
            &sync_args(Vec::new()),
            &finaliser,
        );

        assert!(finaliser.imports.borrow().is_empty());
    }

    #[test]
    fn a_failing_finaliser_leaves_the_exit_code_unchanged() {
        let quiet = sync_repo();
        let complaining = sync_repo();
        let tracker = || Rc::new(RecordingTracker::holding(Vec::new()));

        let expected =
            drive_sync(quiet.path(), &tracker(), &sync_args(Vec::new()));
        let observed = drive_sync_finalising(
            complaining.path(),
            &tracker(),
            &sync_args(Vec::new()),
            &RecordingFinaliser {
                complaint: Some("warning: finaliser failed"),
                ..RecordingFinaliser::default()
            },
        );

        assert_eq!(observed, expected);
    }

    fn unparseable_baseline(root: &Path) {
        let baseline =
            root.join(".accelerator/state/integrations/jira/last-sync.json");
        std::fs::create_dir_all(baseline.parent().expect("parent"))
            .expect("mkdir");
        std::fs::write(baseline, "not json").expect("seed the baseline");
    }

    const CLEAN_ITEM: &str = "---\nid: \"0001\"\ntitle: \"Title\"\n\
                              external_id: \"PP-1\"\n---\n\nOld body\n";

    fn item_path(dir: &Path) -> PathBuf {
        dir.join("meta/work/0001-title.md")
    }

    fn baseline_path_in(dir: &Path) -> PathBuf {
        baseline_under(&dir.join(".accelerator/state/integrations"))
    }

    fn baseline_under(integrations: &Path) -> PathBuf {
        integrations.join("jira/last-sync.json")
    }

    fn git_in(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git invocation");
        assert!(status.success(), "git {args:?}");
    }

    fn commit_all(dir: &Path) {
        git_in(dir, &["add", "-A"]);
        git_in(dir, &["commit", "-q", "-m", "seed"]);
    }

    /// Item `0001` tracked as `PP-1`, holding `local`, with a recipe-1
    /// baseline entry recording `local` against `remote_body` as of the
    /// start of 2026.
    fn synced_item(
        dir: &Path,
        tracker: &RecordingTracker,
        local: &str,
        remote_body: &str,
    ) {
        synced_item_recorded_at(
            &baseline_path_in(dir),
            dir,
            tracker,
            local,
            remote_body,
        );
    }

    fn synced_item_recorded_at(
        baseline_path: &Path,
        dir: &Path,
        tracker: &RecordingTracker,
        local: &str,
        remote_body: &str,
    ) {
        std::fs::write(item_path(dir), local).expect("write the item");
        let baseline_path = baseline_path.to_path_buf();
        std::fs::create_dir_all(baseline_path.parent().expect("parent"))
            .expect("mkdir");
        let writer = FileCorpusStore::new(
            baseline_path.parent().expect("the baseline's directory"),
        );
        let baseline = BaselineStore::new(
            baseline_path,
            &corpus_adapters::RealFs,
            &writer,
        );
        let fingerprints =
            work_adapters::sync::baseline_digester::BaselineDigester::new(
                tracker,
                &CommonMarkBody,
            )
            .digests(local, Some(remote_body))
            .expect("the item is well formed");
        baseline
            .set(
                "0001",
                work_adapters::sync::baseline::Entry::recorded(
                    work_adapters::sync::baseline::SyncStamps {
                        remote_updated_at: tracker::RemoteTimestamp::Reported(
                            "2026-01-01T00:00:00Z".to_owned(),
                        ),
                        local_synced_at: 0,
                    },
                    fingerprints,
                ),
            )
            .expect("seed the baseline");
    }

    fn edited_remotely(body: &str) -> Rc<RecordingTracker> {
        Rc::new(RecordingTracker::holding(vec![(
            ExternalId::new("PP-1".to_owned()),
            RemoteIssue {
                key: ExternalId::new("PP-1".to_owned()),
                updated: tracker::RemoteTimestamp::Reported(
                    "2026-02-01T00:00:00Z".to_owned(),
                ),
                body: body.to_owned(),
            },
        )]))
    }

    fn frontmatter_of(content: &str) -> &str {
        let end = content[4..].find("\n---\n").expect("a closing fence") + 4;
        &content[..end]
    }

    fn assert_recorded_as_pulled(
        dir: &Path,
        tracker: &RecordingTracker,
        remote_body: &str,
    ) {
        assert_recorded_as_pulled_at(
            &baseline_path_in(dir),
            dir,
            tracker,
            remote_body,
        );
    }

    fn assert_recorded_as_pulled_at(
        baseline_path: &Path,
        dir: &Path,
        tracker: &RecordingTracker,
        remote_body: &str,
    ) {
        let content =
            std::fs::read_to_string(item_path(dir)).expect("read the item");
        let unused_writer = FileCorpusStore::new(dir);
        let baseline = BaselineStore::new(
            baseline_path.to_path_buf(),
            &corpus_adapters::RealFs,
            &unused_writer,
        );
        let (loaded, _) = baseline.load().expect("the baseline loads");
        let expected =
            work_adapters::sync::baseline_digester::BaselineDigester::new(
                tracker,
                &CommonMarkBody,
            )
            .digests(&content, Some(remote_body))
            .expect("the pulled item is well formed");
        let Some(work_adapters::sync::baseline::Entry::Split {
            legacy,
            split,
            ..
        }) = loaded.get("0001")
        else {
            panic!("expected a recipe-1 entry for 0001");
        };
        assert_eq!(split, &expected.split, "the split fingerprints");
        assert_eq!(legacy, &expected.legacy, "the legacy hashes");
    }

    #[test]
    fn a_remote_edit_to_a_clean_tracked_item_is_pulled() {
        let dir = sync_repo();
        let remote_body = "Title\nPulled body\n";
        let tracker = edited_remotely(remote_body);
        synced_item(dir.path(), &tracker, CLEAN_ITEM, "Title\nOld body\n");
        commit_all(dir.path());

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        let content = std::fs::read_to_string(item_path(dir.path()))
            .expect("read the item");
        assert_eq!(frontmatter_of(&content), frontmatter_of(CLEAN_ITEM));
        assert_eq!(
            &content[frontmatter_of(&content).len()..],
            "\n---\n\nPulled body\n"
        );
        assert_recorded_as_pulled(dir.path(), &tracker, remote_body);
        assert!(
            run.out.contains("0001\tpull\tremotely-modified\t-"),
            "{}",
            run.out
        );
        assert!(!run.err.contains("note:"), "{}", run.err);
        assert!(!run.err.contains("error:"), "{}", run.err);
        assert_eq!(run.code, ExitCode::SUCCESS);

        let preview = drive_sync_captured(
            dir.path(),
            &tracker,
            &SyncArgs {
                preview: true,
                ..sync_args(vec![])
            },
        );
        assert!(!preview.out.contains("0001\t"), "{}", preview.out);
        assert!(
            preview.out.contains("#\tsummary\tsynced\t1"),
            "{}",
            preview.out
        );
        assert_eq!(preview.code, ExitCode::SUCCESS);
    }

    fn kept_edits_record<'a>(out: &'a str, id: &str) -> Option<Vec<&'a str>> {
        out.lines()
            .map(|line| line.split('\t').collect::<Vec<_>>())
            .find(|fields| {
                fields.get(1) == Some(&"kept-edits")
                    && fields.get(2) == Some(&id)
            })
    }

    #[test]
    fn a_resolve_to_remote_pull_lands_and_keeps_the_local_edits() {
        let dir = sync_repo();
        let remote_body = "Title\nPulled body\n";
        let tracker = edited_remotely(remote_body);
        synced_item(dir.path(), &tracker, CLEAN_ITEM, "Title\nOld body\n");
        commit_all(dir.path());
        let edited = CLEAN_ITEM.replace("Old body", "Local edit");
        std::fs::write(item_path(dir.path()), &edited).expect("edit");

        let run = drive_sync_captured(
            dir.path(),
            &tracker,
            &SyncArgs {
                resolutions: vec![("0001".to_owned(), "remote".to_owned())],
                ..sync_args(vec![])
            },
        );

        let content = std::fs::read_to_string(item_path(dir.path()))
            .expect("read the item");
        assert!(content.ends_with("\n---\n\nPulled body\n"), "{content}");
        assert_recorded_as_pulled(dir.path(), &tracker, remote_body);
        assert!(run.out.contains("0001\tpull\tconflict\t-"), "{}", run.out);
        let record = kept_edits_record(&run.out, "0001")
            .unwrap_or_else(|| panic!("no kept-edits record: {}", run.out));
        assert_eq!(record[3], "this-run", "{record:?}");
        assert_eq!(
            Path::new(record[4]).canonicalize().expect("the item"),
            item_path(dir.path()).canonicalize().expect("the item")
        );
        let copy = record[5];
        assert_eq!(std::fs::read_to_string(copy).expect("the copy"), edited);
        assert!(
            run.err.contains(&format!(
                "note: 0001: kept-edits: local edits kept at {copy}; merge \
                 them, then run accelerator work sync --target 0001 \
                 --discard-kept-edits 0001"
            )),
            "{}",
            run.err
        );
    }

    fn kept_copy_settles_over_three_runs(dir: &Path) {
        let remote_body = "Title\nPulled body\n";
        let tracker = edited_remotely(remote_body);
        synced_item(dir, &tracker, CLEAN_ITEM, "Title\nOld body\n");
        let edited = CLEAN_ITEM.replace("Old body", "Local edit");
        std::fs::write(item_path(dir), &edited).expect("edit");
        let resolving = SyncArgs {
            resolutions: vec![("0001".to_owned(), "remote".to_owned())],
            ..sync_args(vec![])
        };

        let first = drive_sync_captured(dir, &tracker, &resolving);
        let kept = kept_edits_record(&first.out, "0001")
            .unwrap_or_else(|| panic!("no kept copy: {}", first.out));
        assert_eq!(kept[3], "this-run");
        let copy = PathBuf::from(kept[5]);

        let second = drive_sync_captured(dir, &tracker, &sync_args(vec![]));
        let carried = kept_edits_record(&second.out, "0001")
            .unwrap_or_else(|| panic!("no carried copy: {}", second.out));
        assert_eq!(carried[3], "earlier", "{}", second.out);
        assert!(
            !second.err.contains("tracked by version control"),
            "{}",
            second.err
        );

        std::fs::write(item_path(dir), &edited).expect("restore the edit");
        let third = drive_sync_captured(dir, &tracker, &sync_args(vec![]));
        assert!(
            kept_edits_record(&third.out, "0001").is_none(),
            "{}",
            third.out
        );
        assert!(!copy.exists(), "the settled copy is swept");
    }

    #[test]
    fn a_kept_copy_is_read_as_untracked_and_settles_over_three_runs_under_git()
    {
        let dir = sync_repo();
        commit_all(dir.path());

        kept_copy_settles_over_three_runs(dir.path());
    }

    #[test]
    fn a_kept_copy_is_read_as_untracked_and_settles_over_three_runs_under_jj() {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")
            .expect("the pinned jj");
        let dir = tempfile::tempdir().expect("tempdir");
        let config = tempfile::tempdir().expect("tempdir");
        let env =
            vcs_test_support::hermetic::Hermetic::rooted_at(config.path())
                .expect("an isolated VCS config");
        env.jj(&["git", "init", "--no-colocate"], dir.path())
            .expect("jj init");
        std::fs::create_dir_all(dir.path().join("meta/work")).expect("mkdir");
        std::fs::create_dir_all(dir.path().join(".accelerator"))
            .expect("mkdir");
        team_config(dir.path(), "");

        kept_copy_settles_over_three_runs(dir.path());
    }

    /// Fails the first baseline write, then writes through `inner`.
    struct FailingFirstBaselineWrite {
        inner: Box<dyn SyncStore>,
        failed: std::cell::Cell<bool>,
    }

    impl corpus::store::AtomicWrite for FailingFirstBaselineWrite {
        fn write(
            &self,
            path: &Path,
            bytes: &[u8],
        ) -> Result<(), corpus::store::StoreError> {
            if path.ends_with("last-sync.json") && !self.failed.replace(true) {
                return Err(corpus::store::StoreError::Io {
                    path: path.display().to_string(),
                    detail: "injected baseline failure".to_owned(),
                });
            }
            self.inner.write(path, bytes)
        }
    }

    impl corpus::store::FileRemove for FailingFirstBaselineWrite {
        fn remove(&self, path: &Path) -> Result<(), corpus::store::StoreError> {
            self.inner.remove(path)
        }
    }

    /// Writes through `inner`, logging every path written.
    struct RecordingWrites {
        inner: Box<dyn SyncStore>,
        written: Rc<RefCell<Vec<PathBuf>>>,
    }

    impl corpus::store::AtomicWrite for RecordingWrites {
        fn write(
            &self,
            path: &Path,
            bytes: &[u8],
        ) -> Result<(), corpus::store::StoreError> {
            self.written.borrow_mut().push(path.to_path_buf());
            self.inner.write(path, bytes)
        }
    }

    impl corpus::store::FileRemove for RecordingWrites {
        fn remove(&self, path: &Path) -> Result<(), corpus::store::StoreError> {
            self.inner.remove(path)
        }
    }

    fn mutations(tracker: &RecordingTracker) -> Vec<Call> {
        tracker
            .calls()
            .into_iter()
            .filter(|call| {
                matches!(
                    call,
                    Call::Update { .. }
                        | Call::Retitle { .. }
                        | Call::Create { .. }
                )
            })
            .collect()
    }

    #[test]
    fn a_terminal_baseline_set_after_a_pull_records_the_baseline_on_the_next_run(
    ) {
        use super::stores::rooted_store;
        use super::stores::StoreFactories;

        let dir = sync_repo();
        let remote_body = "Title\nPulled body\n";
        let tracker = edited_remotely(remote_body);
        synced_item(dir.path(), &tracker, CLEAN_ITEM, "Title\nOld body\n");
        commit_all(dir.path());
        let failing_baseline = |root: &crate::bounded_roots::BoundedRoot| {
            Box::new(FailingFirstBaselineWrite {
                inner: rooted_store(root),
                failed: std::cell::Cell::new(false),
            }) as Box<dyn SyncStore>
        };
        let first = capture_sync_over(
            dir.path(),
            &tracker,
            &sync_args(vec![]),
            &NoFinaliser,
            StoreFactories {
                work_items: &rooted_store,
                integration_state: &failing_baseline,
            },
        );
        assert!(
            first
                .out
                .contains("0001\tfailed\tremotely-modified\tterminal"),
            "{}",
            first.out
        );
        let pulled = std::fs::read_to_string(item_path(dir.path()))
            .expect("read the item");
        assert!(pulled.ends_with("\n---\n\nPulled body\n"), "{pulled}");
        let mutations_before = mutations(&tracker).len();

        let written = Rc::new(RefCell::new(Vec::new()));
        let recording_items = |root: &crate::bounded_roots::BoundedRoot| {
            Box::new(RecordingWrites {
                inner: rooted_store(root),
                written: Rc::clone(&written),
            }) as Box<dyn SyncStore>
        };
        let second = capture_sync_over(
            dir.path(),
            &tracker,
            &sync_args(vec![]),
            &NoFinaliser,
            StoreFactories {
                work_items: &recording_items,
                integration_state: &rooted_store,
            },
        );

        assert!(!second.out.contains("0001\t"), "{}", second.out);
        assert!(
            second.out.contains("#\tsummary\tsynced\t1"),
            "{}",
            second.out
        );
        assert!(
            !written
                .borrow()
                .iter()
                .any(|path| path.ends_with("0001-title.md")),
            "{:?}",
            written.borrow()
        );
        assert_eq!(mutations(&tracker).len(), mutations_before);
        assert_recorded_as_pulled(dir.path(), &tracker, remote_body);

        let third = drive_sync_captured(
            dir.path(),
            &tracker,
            &SyncArgs {
                preview: true,
                ..sync_args(vec![])
            },
        );
        assert!(!third.out.contains("0001\t"), "{}", third.out);
        assert_eq!(third.code, ExitCode::SUCCESS);
    }

    fn numbered_item(id: &str, key: &str, body: &str) -> String {
        format!(
            "---\nid: \"{id}\"\ntitle: \"Title\"\nexternal_id: \"{key}\"\n\
             ---\n\n{body}"
        )
    }

    fn numbered_path(dir: &Path, id: &str) -> PathBuf {
        dir.join(format!("meta/work/{id}-title.md"))
    }

    /// Items each tracked under their key, recorded in sync with the
    /// remote's old body, whose remote side now holds a new body.
    fn edited_remotely_numbered(
        dir: &Path,
        items: &[(&str, &str)],
    ) -> Rc<RecordingTracker> {
        let tracker = Rc::new(RecordingTracker::holding(
            items
                .iter()
                .map(|(_, key)| {
                    (
                        ExternalId::new((*key).to_owned()),
                        RemoteIssue {
                            key: ExternalId::new((*key).to_owned()),
                            updated: tracker::RemoteTimestamp::Reported(
                                "2026-02-01T00:00:00Z".to_owned(),
                            ),
                            body: "Title\nPulled body\n".to_owned(),
                        },
                    )
                })
                .collect(),
        ));
        let writer = FileCorpusStore::new(
            baseline_path_in(dir)
                .parent()
                .expect("a baseline directory"),
        );
        std::fs::create_dir_all(
            baseline_path_in(dir)
                .parent()
                .expect("a baseline directory"),
        )
        .expect("mkdir");
        let baseline = BaselineStore::new(
            baseline_path_in(dir),
            &corpus_adapters::RealFs,
            &writer,
        );
        for (id, key) in items {
            let local = numbered_item(id, key, "Old body\n");
            std::fs::write(numbered_path(dir, id), &local)
                .expect("write the item");
            let fingerprints =
                work_adapters::sync::baseline_digester::BaselineDigester::new(
                    tracker.as_ref(),
                    &CommonMarkBody,
                )
                .digests(&local, Some("Title\nOld body\n"))
                .expect("the item is well formed");
            baseline
                .set(
                    id,
                    work_adapters::sync::baseline::Entry::recorded(
                        work_adapters::sync::baseline::SyncStamps {
                            remote_updated_at:
                                tracker::RemoteTimestamp::Reported(
                                    "2026-01-01T00:00:00Z".to_owned(),
                                ),
                            local_synced_at: 0,
                        },
                        fingerprints,
                    ),
                )
                .expect("seed the baseline");
        }
        commit_all(dir);
        tracker
    }

    fn recorded_entry(
        dir: &Path,
        id: &str,
    ) -> Option<work_adapters::sync::baseline::Entry> {
        let unused_writer = FileCorpusStore::new(dir);
        let baseline = BaselineStore::new(
            baseline_path_in(dir),
            &corpus_adapters::RealFs,
            &unused_writer,
        );
        let (loaded, _) = baseline.load().expect("the baseline loads");
        loaded.get(id).cloned()
    }

    type StoreFactory =
        dyn Fn(&crate::bounded_roots::BoundedRoot) -> Box<dyn SyncStore>;

    fn failing_work_items(
        applies: fn(&Path) -> bool,
        error: fn(&Path) -> corpus::store::StoreError,
    ) -> impl Fn(&crate::bounded_roots::BoundedRoot) -> Box<dyn SyncStore> {
        move |root| {
            Box::new(crate::test_support::Faults {
                inner: super::stores::rooted_store(root),
                applies,
                left: std::cell::Cell::new(1),
                error,
            }) as Box<dyn SyncStore>
        }
    }

    fn not_writable(path: &Path) -> corpus::store::StoreError {
        corpus::store::StoreError::NotWritable {
            path: path.display().to_string(),
            detail: "Permission denied (os error 13)".to_owned(),
        }
    }

    fn capture_with_stores(
        dir: &Path,
        tracker: &Rc<RecordingTracker>,
        work_items: &StoreFactory,
        integration_state: &StoreFactory,
    ) -> CapturedRun {
        capture_sync_over(
            dir,
            tracker,
            &sync_args(vec![]),
            &NoFinaliser,
            super::stores::StoreFactories {
                work_items,
                integration_state,
            },
        )
    }

    fn error_lines(err: &str) -> Vec<&str> {
        err.lines()
            .filter(|line| line.starts_with("error:"))
            .collect()
    }

    #[test]
    fn a_write_refused_for_permission_after_the_lock_is_retryable() {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);

        let run = capture_with_stores(
            dir.path(),
            &tracker,
            &failing_work_items(
                |path| path.ends_with("0001-title.md"),
                not_writable,
            ),
            &super::stores::rooted_store,
        );

        assert!(
            run.out
                .contains("0001\tfailed\tremotely-modified\tretryable"),
            "{}",
            run.out
        );
        let errors = error_lines(&run.err);
        assert_eq!(errors.len(), 1, "{}", run.err);
        assert!(errors[0].starts_with("error: 0001: write-local failed: "));
        assert!(errors[0].contains("Permission denied"), "{}", run.err);
        assert_eq!(run.code, ExitCode::from(super::exit_codes::RETRYABLE));
    }

    #[test]
    fn a_failed_work_item_write_is_retryable() {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);

        let run = capture_with_stores(
            dir.path(),
            &tracker,
            &failing_work_items(
                |path| path.ends_with("0001-title.md"),
                crate::test_support::injected,
            ),
            &super::stores::rooted_store,
        );

        assert!(
            run.out
                .contains("0001\tfailed\tremotely-modified\tretryable"),
            "{}",
            run.out
        );
        let errors = error_lines(&run.err);
        assert_eq!(errors.len(), 1, "{}", run.err);
        assert!(errors[0].contains("write-local"), "{}", run.err);
        assert!(errors[0].contains("injected"), "{}", run.err);
        assert_eq!(run.code, ExitCode::from(super::exit_codes::RETRYABLE));
    }

    #[test]
    fn a_failed_baseline_write_after_a_pull_is_terminal() {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);
        let before = recorded_entry(dir.path(), "0001");

        let run = capture_with_stores(
            dir.path(),
            &tracker,
            &super::stores::rooted_store,
            &failing_work_items(
                |path| path.ends_with("last-sync.json"),
                crate::test_support::injected,
            ),
        );

        let content =
            std::fs::read_to_string(numbered_path(dir.path(), "0001"))
                .expect("read the item");
        assert!(content.ends_with("\n---\n\nPulled body\n"), "{content}");
        assert_eq!(recorded_entry(dir.path(), "0001"), before);
        assert!(
            run.out
                .contains("0001\tfailed\tremotely-modified\tterminal"),
            "{}",
            run.out
        );
        let errors = error_lines(&run.err);
        assert!(
            errors.iter().any(|line| line
                .starts_with("error: 0001: baseline-set failed: after pull: ")
                && line.contains("injected")),
            "{}",
            run.err
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::TERMINAL));
    }

    #[test]
    fn another_items_successful_pull_does_not_make_a_failure_terminal() {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(
            dir.path(),
            &[("0001", "PP-1"), ("0002", "PP-2")],
        );

        let run = capture_with_stores(
            dir.path(),
            &tracker,
            &failing_work_items(
                |path| path.ends_with("0002-title.md"),
                crate::test_support::injected,
            ),
            &super::stores::rooted_store,
        );

        assert!(
            run.out.contains("0001\tpull\tremotely-modified\t-"),
            "{}",
            run.out
        );
        assert!(
            run.out
                .contains("0002\tfailed\tremotely-modified\tretryable"),
            "{}",
            run.out
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::RETRYABLE));
    }

    #[test]
    fn a_conflict_beside_a_local_failure_shows_both_rows() {
        for (applies, row, code) in [
            (
                (|path: &Path| path.ends_with("0002-title.md"))
                    as fn(&Path) -> bool,
                "0002\tfailed\tremotely-modified\tretryable",
                super::exit_codes::UNRESOLVED,
            ),
            (
                |path: &Path| path.ends_with("last-sync.json"),
                "0002\tfailed\tremotely-modified\tterminal",
                super::exit_codes::TERMINAL,
            ),
        ] {
            let dir = sync_repo();
            let tracker = edited_remotely_numbered(
                dir.path(),
                &[("0001", "PP-1"), ("0002", "PP-2")],
            );
            std::fs::write(
                numbered_path(dir.path(), "0001"),
                numbered_item("0001", "PP-1", "Local edit\n"),
            )
            .expect("edit the item");
            let faulty =
                failing_work_items(applies, crate::test_support::injected);
            let (work_items, integration_state): (
                &StoreFactory,
                &StoreFactory,
            ) = if code == super::exit_codes::UNRESOLVED {
                (&faulty, &super::stores::rooted_store)
            } else {
                (&super::stores::rooted_store, &faulty)
            };

            let run = capture_with_stores(
                dir.path(),
                &tracker,
                work_items,
                integration_state,
            );

            assert!(
                run.out.contains("0001\tunresolved\tconflict\t-"),
                "{}",
                run.out
            );
            assert!(run.out.contains(row), "{}", run.out);
            assert!(
                error_lines(&run.err)
                    .iter()
                    .any(|line| line.starts_with("error: 0002: ")),
                "{}",
                run.err
            );
            assert_eq!(run.code, ExitCode::from(code));
        }
    }

    #[test]
    fn a_read_only_work_directory_fails_the_pull_as_retryable_then_pulls() {
        use std::os::unix::fs::PermissionsExt;

        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);
        let work = dir.path().join("meta/work");
        let item_before = std::fs::read(numbered_path(dir.path(), "0001"))
            .expect("read the item");
        let baseline_before = recorded_entry(dir.path(), "0001");
        std::fs::set_permissions(&work, std::fs::Permissions::from_mode(0o555))
            .expect("make the work directory read-only");
        let probe = work.join(".probe");
        let enforced = std::fs::write(&probe, "").is_err();
        let _ = std::fs::remove_file(&probe);

        let refused =
            drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));
        std::fs::set_permissions(&work, std::fs::Permissions::from_mode(0o755))
            .expect("restore the work directory");
        if !enforced {
            return;
        }

        assert!(
            refused
                .out
                .contains("0001\tfailed\tremotely-modified\tretryable"),
            "{}",
            refused.out
        );
        let errors = error_lines(&refused.err);
        assert_eq!(errors.len(), 1, "{}", refused.err);
        assert!(errors[0].contains("0001"), "{}", refused.err);
        assert!(errors[0].contains("lock-local"), "{}", refused.err);
        assert!(errors[0].contains("Permission denied"), "{}", refused.err);
        assert_eq!(
            std::fs::read(numbered_path(dir.path(), "0001")).expect("read"),
            item_before
        );
        assert_eq!(recorded_entry(dir.path(), "0001"), baseline_before);
        assert_eq!(refused.code, ExitCode::from(super::exit_codes::RETRYABLE));

        let pulled =
            drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert!(
            pulled.out.contains("0001\tpull\tremotely-modified\t-"),
            "{}",
            pulled.out
        );
        assert_ne!(recorded_entry(dir.path(), "0001"), baseline_before);
    }

    #[test]
    fn each_tracker_failure_kind_reports_its_class_detail_and_exit() {
        for (error, detail, code) in [
            (
                TrackerError::Retryable {
                    detail: "offline".to_owned(),
                },
                "retryable",
                super::exit_codes::RETRYABLE,
            ),
            (
                TrackerError::Terminal {
                    detail: "timed out after sending".to_owned(),
                },
                "terminal",
                super::exit_codes::TERMINAL,
            ),
            (
                TrackerError::Unconfigured {
                    detail: "no such field".to_owned(),
                },
                "unconfigured",
                super::exit_codes::UNCONFIGURED,
            ),
            (
                TrackerError::Rejected {
                    detail: "the body has a table".to_owned(),
                },
                "rejected",
                super::exit_codes::REJECTED,
            ),
        ] {
            let dir = sync_repo();
            let local = numbered_item("0001", "PP-1", "Old body\n");
            let tracker = Rc::new(
                RecordingTracker::holding(vec![(
                    ExternalId::new("PP-1".to_owned()),
                    RemoteIssue {
                        key: ExternalId::new("PP-1".to_owned()),
                        updated: tracker::RemoteTimestamp::Reported(
                            "2026-01-01T00:00:00Z".to_owned(),
                        ),
                        body: "Title\nOld body\n".to_owned(),
                    },
                )])
                .failing_update(
                    ExternalId::new("PP-1".to_owned()),
                    error.clone(),
                ),
            );
            synced_item(dir.path(), &tracker, &local, "Title\nOld body\n");
            std::fs::write(
                item_path(dir.path()),
                numbered_item("0001", "PP-1", "Local edit\n"),
            )
            .expect("edit the item");
            commit_all(dir.path());

            let run =
                drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

            assert!(
                run.out.contains(&format!(
                    "0001\tfailed\tlocally-modified\t{detail}"
                )),
                "{detail}: {}",
                run.out
            );
            let errors = error_lines(&run.err);
            assert!(
                errors.iter().any(|line| {
                    line.starts_with("error: 0001: update failed: ")
                        && line.contains(&error.to_string())
                }),
                "{detail}: {}",
                run.err
            );
            assert_eq!(run.code, ExitCode::from(code), "{detail}");
        }
    }

    #[test]
    fn a_pending_push_refusal_reports_terminal_and_names_its_reason() {
        let dir = sync_repo();
        std::fs::write(
            dir.path().join("meta/work/0001-title.md"),
            "---\nid: \"0001\"\ntitle: \"Title\"\nkind: task\n---\n\nBody\n",
        )
        .expect("write the draft");
        let marker = dir.path().join(
            ".accelerator/state/integrations/jira/pending-push/0001.json",
        );
        std::fs::create_dir_all(marker.parent().expect("a parent"))
            .expect("mkdir");
        std::fs::write(&marker, "{ not a marker").expect("seed the marker");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert!(
            run.out.contains("0001\tfailed\tunsynced\tterminal"),
            "{}",
            run.out
        );
        assert!(
            error_lines(&run.err).iter().any(|line| line.starts_with(
                "error: 0001: create failed: pending push refused \
                 (marker-unreadable): "
            )),
            "{}",
            run.err
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::TERMINAL));
    }

    fn unidentified_record(out: &str, path: &Path, kind: &str) -> bool {
        out.lines().any(|line| {
            line == format!("#\tunidentified\t{}\t{kind}", path.display())
        })
    }

    #[test]
    fn an_undecodable_work_file_is_reported_by_path() {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);
        let garbled = dir.path().join("meta/work/0002-garbled.md");
        std::fs::write(&garbled, [0xff, 0xfe, b'\n']).expect("write the file");

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert!(
            unidentified_record(&run.out, &garbled, "undecodable"),
            "{}",
            run.out
        );
        assert!(
            run.out.contains("0001\tpull\tremotely-modified\t-"),
            "{}",
            run.out
        );
        assert_eq!(
            error_lines(&run.err),
            vec![format!(
                "error: {}: read-local failed: not UTF-8",
                garbled.display()
            )]
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::REJECTED));
    }

    #[test]
    fn an_unreadable_work_file_is_reported_by_path() {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);
        let sealed = dir.path().join("meta/work/0002-sealed.md");
        std::fs::write(&sealed, "---\nid: \"0002\"\n---\n").expect("write");
        let Some(_guard) =
            store::test_support::UnreadableFile::new(&sealed).expect("seal")
        else {
            return;
        };

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert!(
            unidentified_record(&run.out, &sealed, "unreadable"),
            "{}",
            run.out
        );
        let errors = error_lines(&run.err);
        assert_eq!(errors.len(), 1, "{}", run.err);
        assert!(
            errors[0].starts_with(&format!(
                "error: {}: read-local failed: ",
                sealed.display()
            )),
            "{}",
            run.err
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::RETRYABLE));
    }

    #[test]
    fn a_readme_in_the_work_directory_does_not_fail_sync() {
        for (content, kind) in [
            ("---\ntitle: About\n---\n\n# About\n", "no-id"),
            ("# About\n", "no-frontmatter"),
        ] {
            let dir = sync_repo();
            let tracker =
                edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);
            let readme = dir.path().join("meta/work/README.md");
            std::fs::write(&readme, content).expect("write the readme");

            let run =
                drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

            assert!(
                unidentified_record(&run.out, &readme, kind),
                "{}",
                run.out
            );
            assert!(!run.err.contains("README"), "{}", run.err);
            assert_eq!(run.code, ExitCode::SUCCESS, "{kind}: {}", run.err);
        }
    }

    #[test]
    fn a_conflicted_work_item_file_fails_the_run() {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);
        let conflicted = dir.path().join("meta/work/0002-two.md");
        std::fs::write(
            &conflicted,
            "<<<<<<< Conflict 1 of 1\n%%%%%%% Changes from base to side #1\n\
             -id: \"0002\"\n+id: \"0002\"\n+++++++ Contents of side #2\n\
             id: \"0002\"\n>>>>>>> Conflict 1 of 1 ends\n---\n\nBody\n",
        )
        .expect("write the file");

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert!(
            unidentified_record(&run.out, &conflicted, "malformed"),
            "{}",
            run.out
        );
        assert!(
            error_lines(&run.err).iter().any(|line| line.starts_with(
                &format!(
                    "error: {}: identify-local failed: ",
                    conflicted.display()
                )
            )),
            "{}",
            run.err
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::REJECTED));
    }

    #[test]
    fn a_targeted_run_reports_but_does_not_fail_on_an_unrelated_unreadable_file(
    ) {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);
        let garbled = dir.path().join("meta/work/0002-garbled.md");
        std::fs::write(&garbled, [0xff, 0xfe, b'\n']).expect("write the file");

        let run = drive_sync_captured(
            dir.path(),
            &tracker,
            &sync_args(vec!["0001".to_owned()]),
        );

        assert!(
            unidentified_record(&run.out, &garbled, "undecodable"),
            "{}",
            run.out
        );
        assert!(error_lines(&run.err).is_empty(), "{}", run.err);
        assert!(
            run.err.contains(&format!(
                "warning: {}: read-local failed: not UTF-8",
                garbled.display()
            )),
            "{}",
            run.err
        );
        assert_eq!(run.code, ExitCode::SUCCESS);
    }

    #[test]
    fn a_newline_in_a_filename_cannot_forge_an_error_line() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        std::fs::write(
            dir.path()
                .join("meta/work/0002-\nerror: -: baseline-set failed:.md"),
            "# no frontmatter\n",
        )
        .expect("write the file");

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_eq!(error_lines(&run.err).len(), 1, "{}", run.err);
        assert_eq!(run.code, ExitCode::from(super::exit_codes::REJECTED));
    }

    #[test]
    fn a_body_with_conflict_markers_is_neither_pushed_nor_pulled() {
        let dir = sync_repo();
        let tracker = edited_remotely_numbered(dir.path(), &[("0001", "PP-1")]);
        let conflicted = numbered_item(
            "0001",
            "PP-1",
            "<<<<<<< HEAD\nOld body\n=======\nOther body\n>>>>>>> side\n",
        );
        std::fs::write(numbered_path(dir.path(), "0001"), &conflicted)
            .expect("write the item");

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_eq!(
            std::fs::read_to_string(numbered_path(dir.path(), "0001"))
                .expect("read the item"),
            conflicted
        );
        assert!(mutations(&tracker).is_empty());
        assert!(!run.out.contains("0001\t"), "{}", run.out);
        assert_eq!(run.code, ExitCode::from(super::exit_codes::REJECTED));
    }

    fn discovering_remote(keys: &[&str]) -> Rc<RecordingTracker> {
        Rc::new(
            RecordingTracker::holding(
                keys.iter()
                    .map(|key| {
                        (
                            ExternalId::new((*key).to_owned()),
                            issue("Imported\nRemote body"),
                        )
                    })
                    .collect(),
            )
            .discovering(
                keys.iter()
                    .map(|key| {
                        (
                            ExternalId::new((*key).to_owned()),
                            tracker::RemoteTimestamp::Reported(
                                "2026-01-01T00:00:00Z".to_owned(),
                            ),
                        )
                    })
                    .collect(),
                true,
            ),
        )
    }

    const CONFLICTED_ITEM: &str = "<<<<<<< Conflict 1 of 1\n%%%%%%% Changes \
                                   from base to side #1\n-title: a\n+title: \
                                   b\n+++++++ Contents of side #2\ntitle: \
                                   c\n>>>>>>> Conflict 1 of 1 ends\n---\nid: \
                                   \"0042\"\nexternal_id: \"PP-42\"\n---\n";

    #[test]
    fn a_conflicted_tracked_item_is_not_re_imported() {
        let dir = sync_repo();
        let conflicted = dir.path().join("meta/work/0042-tracked.md");
        std::fs::write(&conflicted, CONFLICTED_ITEM).expect("write the item");
        let tracker = discovering_remote(&["PP-42"]);

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_eq!(new_work_files(dir.path()), vec!["0042-tracked.md"]);
        assert!(
            run.out
                .lines()
                .any(|line| line == "#\tdiscovery\tskipped\tincomplete-corpus"),
            "{}",
            run.out
        );
        assert!(
            unidentified_record(&run.out, &conflicted, "malformed"),
            "{}",
            run.out
        );
        assert!(
            error_lines(&run.err)
                .iter()
                .any(|line| line.contains("identify-local failed")),
            "{}",
            run.err
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::REJECTED));
    }

    #[test]
    fn a_targeted_pull_is_deferred_while_the_corpus_is_incomplete() {
        let dir = sync_repo();
        std::fs::write(
            dir.path().join("meta/work/0002-broken.md"),
            "---\nid: \"0002\"\n",
        )
        .expect("write the broken item");
        let tracker = Rc::new(RecordingTracker::holding(vec![(
            ExternalId::new("PP-9".to_owned()),
            issue("Imported\nRemote body"),
        )]));

        let run = drive_sync_captured(
            dir.path(),
            &tracker,
            &sync_args(vec!["PP-9".to_owned()]),
        );

        assert_eq!(new_work_files(dir.path()), vec!["0002-broken.md"]);
        assert_eq!(
            error_lines(&run.err),
            vec![
                "error: -: discover failed: the work directory holds \
                 unreadable or malformed files, so PP-9 cannot be shown to be \
                 untracked"
            ]
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::REJECTED));
    }

    #[test]
    fn a_targeted_pull_deferred_by_an_unreadable_file_is_retryable() {
        let dir = sync_repo();
        let sealed = dir.path().join("meta/work/0002-sealed.md");
        std::fs::write(&sealed, "---\nid: \"0002\"\n---\n").expect("write");
        let Some(_guard) =
            store::test_support::UnreadableFile::new(&sealed).expect("seal")
        else {
            return;
        };
        let tracker = Rc::new(RecordingTracker::holding(vec![(
            ExternalId::new("PP-9".to_owned()),
            issue("Imported\nRemote body"),
        )]));

        let run = drive_sync_captured(
            dir.path(),
            &tracker,
            &sync_args(vec!["PP-9".to_owned()]),
        );

        assert_eq!(new_work_files(dir.path()), vec!["0002-sealed.md"]);
        assert!(
            error_lines(&run.err)
                .iter()
                .any(|line| line.starts_with("error: -: discover failed: ")),
            "{}",
            run.err
        );
        assert_eq!(run.code, ExitCode::from(super::exit_codes::RETRYABLE));
    }

    #[test]
    fn an_empty_healthy_corpus_imports_remote_issues() {
        let full = sync_repo();
        let targeted = sync_repo();

        drive_sync(
            full.path(),
            &discovering_remote(&["PP-7"]),
            &sync_args(vec![]),
        );
        drive_sync(
            targeted.path(),
            &discovering_remote(&["PP-7"]),
            &sync_args(vec!["PP-7".to_owned()]),
        );

        assert_eq!(new_work_files(full.path()).len(), 1);
        assert_eq!(new_work_files(targeted.path()).len(), 1);
    }

    fn team_config(dir: &Path, extra: &str) {
        std::fs::write(
            dir.join(".accelerator/config.md"),
            format!("---\nwork:\n  integration: jira\n{extra}---\n"),
        )
        .expect("write the team config");
    }

    fn personal_config(dir: &Path, yaml: &str) {
        use std::os::unix::fs::PermissionsExt as _;
        let path = dir.join(".accelerator/config.local.md");
        std::fs::write(&path, format!("---\n{yaml}---\n"))
            .expect("write the personal config");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("chmod");
    }

    fn integrations_at(path: &Path) -> String {
        format!("paths:\n  integrations: \"{}\"\n", path.display())
    }

    fn assert_refused_before_any_tracker_call(
        run: &CapturedRun,
        tracker: &RecordingTracker,
        cause: &str,
    ) {
        assert_eq!(run.code, ExitCode::from(exit_codes::UNCONFIGURED));
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
        assert!(
            run.err
                .contains(&format!("error: -: configure failed: {cause}")),
            "{}",
            run.err
        );
    }

    fn is_empty_dir(path: &Path) -> bool {
        std::fs::read_dir(path)
            .expect("read the directory")
            .next()
            .is_none()
    }

    #[test]
    fn a_symlinked_work_root_refuses_the_run_before_any_tracker_call() {
        let dir = sync_repo();
        let elsewhere = tempfile::tempdir().expect("tempdir");
        let work = dir.path().join("meta/work");
        std::fs::remove_dir(&work).expect("rmdir");
        std::os::unix::fs::symlink(elsewhere.path(), &work).expect("symlink");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_refused_before_any_tracker_call(
            &run,
            &tracker,
            &format!(
                "work directory {} is a symlink outside the repository; move \
                 it inside the repository",
                work.display()
            ),
        );
        assert!(is_empty_dir(elsewhere.path()));
    }

    #[test]
    fn a_symlinked_integrations_root_refuses_the_run_before_any_tracker_call() {
        let dir = sync_repo();
        let elsewhere = tempfile::tempdir().expect("tempdir");
        let state = dir.path().join(".accelerator/state");
        std::fs::create_dir_all(&state).expect("mkdir");
        std::os::unix::fs::symlink(
            elsewhere.path(),
            state.join("integrations"),
        )
        .expect("symlink");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_refused_before_any_tracker_call(
            &run,
            &tracker,
            "integrations directory ",
        );
        assert!(
            run.err.contains(
                "is a symlink outside the repository; set paths.integrations \
                 to its absolute target in .accelerator/config.local.md to \
                 sync it"
            ),
            "{}",
            run.err
        );
        assert!(is_empty_dir(elsewhere.path()));
    }

    #[test]
    fn a_symlinked_state_root_refuses_the_run() {
        let dir = sync_repo();
        let elsewhere = tempfile::tempdir().expect("tempdir");
        std::os::unix::fs::symlink(
            elsewhere.path(),
            dir.path().join(".accelerator/state"),
        )
        .expect("symlink");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_refused_before_any_tracker_call(
            &run,
            &tracker,
            ".accelerator/state is a symlink outside the repository; move it \
             back inside the repository",
        );
        assert!(is_empty_dir(elsewhere.path()));
    }

    #[test]
    fn a_symlinked_recovery_parent_refuses_the_run() {
        for parent in ["sync-recovery", "retirement-recovery"] {
            let dir = sync_repo();
            let elsewhere = tempfile::tempdir().expect("tempdir");
            let state = dir.path().join(".accelerator/state");
            std::fs::create_dir_all(&state).expect("mkdir");
            std::os::unix::fs::symlink(elsewhere.path(), state.join(parent))
                .expect("symlink");
            let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

            let run =
                drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

            assert_refused_before_any_tracker_call(
                &run,
                &tracker,
                &format!(
                    ".accelerator/state/{parent} is a symlink to {}; move its \
                     contents back into .accelerator/state/{parent}/, then \
                     remove the link",
                    elsewhere.path().display()
                ),
            );
            assert!(is_empty_dir(elsewhere.path()), "{parent}");
        }
    }

    #[test]
    fn a_nested_integrations_root_inside_the_work_root_refuses_the_run() {
        let dir = sync_repo();
        team_config(dir.path(), "paths:\n  integrations: meta/work/state\n");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_refused_before_any_tracker_call(
            &run,
            &tracker,
            "paths.work and paths.integrations overlap; configure them as \
             separate directories",
        );
    }

    #[test]
    fn overlapping_work_and_integrations_roots_refuse_the_run() {
        let dir = sync_repo();
        team_config(dir.path(), "paths:\n  integrations: meta/work\n");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_refused_before_any_tracker_call(
            &run,
            &tracker,
            "paths.work and paths.integrations overlap",
        );
    }

    #[test]
    fn a_work_root_containing_the_state_root_refuses_the_run() {
        let dir = sync_repo();
        team_config(
            dir.path(),
            "paths:\n  work: .accelerator\n  integrations: integrations\n",
        );
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_refused_before_any_tracker_call(
            &run,
            &tracker,
            "paths.work contains .accelerator/state; configure it as a \
             directory beside it",
        );
    }

    #[test]
    fn a_team_config_root_outside_the_repository_refuses_the_run() {
        let dir = sync_repo();
        let elsewhere = tempfile::tempdir().expect("tempdir");
        team_config(dir.path(), &integrations_at(elsewhere.path()));
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_refused_before_any_tracker_call(
            &run,
            &tracker,
            "paths.integrations points outside the repository from the team \
             config; set it in .accelerator/config.local.md to sync there",
        );
        assert!(is_empty_dir(elsewhere.path()));
    }

    #[test]
    fn an_outside_root_from_a_tracked_personal_config_refuses_the_run() {
        let dir = sync_repo();
        let elsewhere = tempfile::tempdir().expect("tempdir");
        personal_config(dir.path(), &integrations_at(elsewhere.path()));
        git_in(dir.path(), &["add", "-f", ".accelerator/config.local.md"]);
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_refused_before_any_tracker_call(
            &run,
            &tracker,
            "paths.integrations points outside the repository from a \
             config.local.md tracked by version control; untrack it",
        );
        assert!(is_empty_dir(elsewhere.path()));
    }

    fn assert_pulled(run: &CapturedRun) {
        assert!(
            run.out.contains("0001\tpull\tremotely-modified\t-"),
            "{}\n{}",
            run.out,
            run.err
        );
        assert_eq!(run.code, ExitCode::SUCCESS, "{}", run.err);
    }

    #[test]
    fn an_absolute_integrations_root_outside_the_repository_syncs() {
        let dir = sync_repo();
        let elsewhere = tempfile::tempdir().expect("tempdir");
        let integrations = elsewhere.path().join("integrations");
        let remote_body = "Title\nPulled body\n";
        let tracker = edited_remotely(remote_body);
        synced_item_recorded_at(
            &baseline_under(&integrations),
            dir.path(),
            &tracker,
            CLEAN_ITEM,
            "Title\nOld body\n",
        );
        commit_all(dir.path());
        personal_config(dir.path(), &integrations_at(&integrations));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_pulled(&run);
        assert_recorded_as_pulled_at(
            &baseline_under(&integrations),
            dir.path(),
            &tracker,
            remote_body,
        );
        assert!(!baseline_path_in(dir.path()).exists());
    }

    #[test]
    fn a_dotdot_configured_integrations_root_pulls() {
        let dir = sync_repo();
        team_config(
            dir.path(),
            "paths:\n  integrations: \
             .accelerator/../.accelerator/state/integrations\n",
        );
        let remote_body = "Title\nPulled body\n";
        let tracker = edited_remotely(remote_body);
        synced_item(dir.path(), &tracker, CLEAN_ITEM, "Title\nOld body\n");
        commit_all(dir.path());

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_pulled(&run);
        assert_recorded_as_pulled(dir.path(), &tracker, remote_body);
    }

    #[test]
    fn a_team_in_repo_paths_work_raises_no_consent_warning() {
        let dir = sync_repo();
        team_config(dir.path(), "paths:\n  work: meta/work\n");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert_eq!(run.code, ExitCode::SUCCESS, "{}", run.err);
        assert!(!run.err.contains("consent"), "{}", run.err);
        assert!(!run.err.contains("E_CONSENT"), "{}", run.err);
    }

    #[test]
    fn a_captured_run_prints_its_report_to_the_console() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert!(run.out.contains("#\tsummary\tsynced\t0"), "{}", run.out);
        assert_eq!(run.code, ExitCode::SUCCESS);
    }

    #[test]
    fn an_unsafe_discard_id_exits_2_before_any_write() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let args = SyncArgs {
            discard_kept_edits: vec!["../0001".to_owned()],
            ..sync_args(vec![])
        };

        let run = drive_sync_captured(dir.path(), &tracker, &args);

        assert_eq!(run.code, ExitCode::from(exit_codes::USAGE));
        assert!(run.err.contains("../0001"), "{}", run.err);
        assert!(tracker.calls().is_empty());
        assert!(!dir.path().join(".accelerator/state").exists());
    }

    #[test]
    fn a_warning_reaches_err_and_not_out() {
        let dir = sync_repo();
        unparseable_baseline(dir.path());
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &sync_args(vec![]));

        assert!(run.err.contains("could not be parsed"), "{}", run.err);
        assert!(!run.out.contains("could not be parsed"), "{}", run.out);
    }

    #[test]
    fn a_warning_with_control_characters_is_printed_safely() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(
            dir.path(),
            &tracker,
            &SyncArgs {
                resolutions: vec![(
                    "0001".to_owned(),
                    "\x1b[2J\rremote".to_owned(),
                )],
                ..sync_args(vec![])
            },
        );

        assert!(run.err.contains("is not an order"), "{}", run.err);
        assert!(!run.err.contains(['\x1b', '\r']), "{:?}", run.err);
    }

    fn resolving(orders: &[(&str, &str)]) -> SyncArgs {
        SyncArgs {
            resolutions: orders
                .iter()
                .map(|(id, order)| ((*id).to_owned(), (*order).to_owned()))
                .collect(),
            ..sync_args(vec![])
        }
    }

    fn assert_refused_before_running(orders: &[(&str, &str)], expected: &str) {
        let dir = sync_repo();
        work_file(dir.path(), "0001", None);
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(dir.path(), &tracker, &resolving(orders));

        assert_eq!(run.code, ExitCode::from(exit_codes::USAGE), "{}", run.err);
        assert!(run.err.contains(expected), "{}", run.err);
        assert!(run.out.is_empty(), "{}", run.out);
        assert!(!baseline_written(dir.path()));
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
    }

    #[test]
    fn an_unrecognised_resolve_order_exits_2() {
        assert_refused_before_running(
            &[("0001", "remot")],
            "--resolve 0001=remot: 'remot' is not an order (accepted: \
             remote, local, skip, adopt:<KEY>, create)",
        );
    }

    #[test]
    fn two_orders_for_one_id_exit_2() {
        assert_refused_before_running(
            &[("0001", "create"), ("0001", "adopt:PP-12")],
            "--resolve names '0001' more than once",
        );
    }

    #[test]
    fn a_conflict_order_and_a_pending_push_order_for_one_id_exit_2() {
        assert_refused_before_running(
            &[("0001", "remote"), ("0001", "create")],
            "--resolve names '0001' more than once",
        );
    }

    #[test]
    fn a_malformed_adopt_key_exits_2_before_any_write() {
        assert_refused_before_running(
            &[("0001", "adopt:../last-sync")],
            "--resolve 0001=adopt:../last-sync: '../last-sync' is not a jira \
             key (for example PP-12)",
        );
    }

    #[test]
    fn a_pending_push_order_for_an_id_that_names_no_file_exits_2() {
        assert_refused_before_running(
            &[("../0001", "create")],
            "--resolve: '../0001' is not a work-item id",
        );
    }

    #[test]
    fn a_newline_in_a_warning_path_is_collapsed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("forged\nwarning: injected");
        std::fs::create_dir_all(&root).expect("mkdir");
        init_sync_repo(&root);
        unparseable_baseline(&root);
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(&root, &tracker, &sync_args(vec![]));

        assert!(
            run.err.contains(
                "forged warning: injected/.accelerator/state/integrations/\
                 jira/last-sync.json could not be parsed"
            ),
            "{}",
            run.err
        );
        assert!(
            !run.err
                .lines()
                .any(|line| line.starts_with("warning: injected")),
            "{}",
            run.err
        );
    }

    #[test]
    fn a_still_pending_restore_warning_keeps_its_layout() {
        use corpus::store::RecoveryCopies as _;

        let dir = sync_repo();
        let root = config_adapters::FileConfigStore::discover_root(dir.path());
        let recovery = corpus_adapters::FileRecoveryCopies::new(
            root.join(super::STATE_DIR),
            &root,
        );
        let kept = Path::new(work::retirement::RECOVERY_PARENT).join("0001");
        let child = root.join("meta/notes/child.md");
        std::fs::create_dir_all(child.parent().expect("parent"))
            .expect("mkdir");
        std::fs::write(&child, "edited after the retirement").expect("write");
        recovery.prepare(&kept).expect("prepare");
        recovery
            .write_once(&kept, &child, b"as the retirement left it")
            .expect("copy");
        recovery
            .mark_restore_pending(&kept, std::slice::from_ref(&child))
            .expect("mark");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = drive_sync_captured(&root, &tracker, &sync_args(vec![]));

        let expected = format!(
            "warning: retirement-incomplete: these paths still differ from \
             their recovery copies in {}:\n  {}\ncompare each path with its \
             recovery copy and merge, then delete the copy\n",
            recovery.location(&kept).display(),
            child.display()
        );
        assert!(run.err.contains(&expected), "{}", run.err);
    }

    #[test]
    fn a_finaliser_warning_is_printed_safely() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run = capture_sync_finalising(
            dir.path(),
            &tracker,
            &sync_args(vec![]),
            &RecordingFinaliser {
                complaint: Some("warning: \x1b[31mred\rover"),
                ..RecordingFinaliser::default()
            },
        );

        assert!(run.err.contains("red"), "{}", run.err);
        assert!(!run.err.contains(['\x1b', '\r']), "{:?}", run.err);
    }

    fn issue(body: &str) -> RemoteIssue {
        RemoteIssue {
            key: ExternalId::new("ENG-1".to_owned()),
            updated: tracker::RemoteTimestamp::Reported(
                "2026-01-01T00:00:00Z".to_owned(),
            ),
            body: body.to_owned(),
        }
    }

    fn keyed_issue(key: &str, body: &str) -> (ExternalId, RemoteIssue) {
        let key = ExternalId::new(key.to_owned());
        (key.clone(), RemoteIssue { key, ..issue(body) })
    }

    fn unsynced_draft(dir: &Path) {
        std::fs::write(
            dir.join("meta/work/0001-draft-one.md"),
            "---\nid: \"0001\"\ntitle: \"Draft one\"\nkind: task\n---\n\n\
             Body\n",
        )
        .expect("write work item");
    }

    fn ordering(order: &str) -> SyncArgs {
        resolving(&[("0001", order)])
    }

    #[test]
    fn an_adopt_row_reads_adopt() {
        let dir = sync_repo_integrated_with("linear");
        unsynced_draft(dir.path());
        let tracker = Rc::new(RecordingTracker::holding(vec![keyed_issue(
            "ENG-7",
            "Draft one\nBody\n",
        )]));

        let run =
            drive_sync_captured(dir.path(), &tracker, &ordering("adopt:ENG-7"));

        assert!(
            run.out
                .lines()
                .any(|line| line == "0001\tadopt\tunsynced\t-"),
            "{}",
            run.out
        );
        assert_eq!(run.code, ExitCode::SUCCESS, "{}", run.err);
    }

    #[test]
    fn a_differing_adopt_says_a_conflict_follows() {
        let dir = sync_repo_integrated_with("linear");
        unsynced_draft(dir.path());
        let tracker = Rc::new(RecordingTracker::holding(vec![keyed_issue(
            "ENG-7",
            "Someone else's issue\nBody\n",
        )]));

        let run =
            drive_sync_captured(dir.path(), &tracker, &ordering("adopt:ENG-7"));

        assert!(
            run.err.contains(
                "note: 0001: adopt: linked ENG-7, whose title or body differ \
                 from the item; the next sync reports a conflict. If ENG-7 is \
                 the wrong issue, run --resolve 0001=adopt:<RIGHT-KEY>, or \
                 --resolve 0001=create if none exists"
            ),
            "{}",
            run.err
        );
        assert_eq!(run.code, ExitCode::SUCCESS, "{}", run.err);
    }

    #[test]
    fn a_create_order_with_no_marker_warns() {
        let dir = sync_repo_integrated_with("linear");
        unsynced_draft(dir.path());
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let run =
            drive_sync_captured(dir.path(), &tracker, &ordering("create"));

        assert!(
            run.err.contains(
                "warning: 0001: --resolve 0001=create had no pending push to \
                 settle"
            ),
            "{}",
            run.err
        );
        assert!(
            run.out
                .lines()
                .any(|line| line == "0001\tcreate-from-local\tunsynced\t-"),
            "{}",
            run.out
        );
    }

    fn new_work_files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir.join("meta/work"))
            .expect("read work dir")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(std::ffi::OsStr::to_str) == Some("md")
            })
            .map(|path| {
                path.file_name().unwrap().to_string_lossy().into_owned()
            })
            .collect();
        names.sort();
        names
    }

    fn only_created_author(dir: &Path) -> String {
        let files = new_work_files(dir);
        let name = files.first().expect("one created file");
        let content = std::fs::read_to_string(dir.join("meta/work").join(name))
            .expect("read created file");
        content
            .lines()
            .find_map(|line| line.strip_prefix("author: "))
            .map(|value| value.trim_matches('"').to_owned())
            .expect("author frontmatter")
    }

    /// Whether any `fetch_all` the run made named `id`. The candidate-
    /// confirmation gate reads the raw `--target` token, so a bare local id
    /// appearing here means the gate ran; the engine's reconcile read only ever
    /// names an item's `external_id`, never its local id.
    fn fetch_all_contains(tracker: &RecordingTracker, id: &str) -> bool {
        tracker.calls().iter().any(|call| {
            matches!(call, Call::FetchAll { ids }
                if ids.iter().any(|candidate| candidate.as_str() == id))
        })
    }

    fn recorded_search_scope(
        tracker: &RecordingTracker,
    ) -> Option<tracker::SearchScope> {
        tracker.calls().iter().find_map(|call| match call {
            Call::Search { scope } => Some(scope.clone()),
            _ => None,
        })
    }

    #[test]
    fn a_configured_filters_bag_reaches_the_search_scope() {
        let dir = sync_repo();
        std::fs::write(
            dir.path().join(".accelerator/config.md"),
            "---\nwork:\n  integration: jira\njira:\n  pull:\n    filters:\n      \
             label:\n        - a\n        - b\n      state:\n        - open\n---\n",
        )
        .expect("write config");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        drive_sync(dir.path(), &tracker, &sync_args(Vec::new()));

        let scope = recorded_search_scope(&tracker)
            .expect("a whole-corpus run issues a discovery search");
        assert_eq!(
            scope.filters,
            vec![
                ("label".to_owned(), "a".to_owned()),
                ("label".to_owned(), "b".to_owned()),
                ("state".to_owned(), "open".to_owned()),
            ],
            "the configured filters flatten one-per-value into the search scope"
        );
    }

    #[test]
    fn a_remote_only_target_is_pulled_into_a_new_local_file() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(vec![(
            ExternalId::new("PP-999".to_owned()),
            issue("Imported\nRemote body"),
        )]));

        drive_sync(dir.path(), &tracker, &sync_args(vec!["PP-999".to_owned()]));

        assert_eq!(
            new_work_files(dir.path()).len(),
            1,
            "a remote-only target creates one new local file"
        );
        assert!(
            baseline_written(dir.path()),
            "the create-from-remote completes through its baseline write, even \
             on a never-synced integration whose state dir does not yet exist"
        );
        assert_eq!(
            only_created_author(dir.path()),
            "Test User",
            "the imported file is authored with the synced repository's own \
             identity, not the ambient process identity"
        );
        assert!(
            fetch_all_contains(&tracker, "PP-999"),
            "the candidate is confirmed through fetch_all"
        );
    }

    #[test]
    fn a_locally_resolvable_target_makes_no_fetch_all_call() {
        let dir = sync_repo();
        work_file(dir.path(), "0001", Some("PP-1"));
        let tracker = Rc::new(RecordingTracker::holding(vec![(
            ExternalId::new("PP-1".to_owned()),
            issue("Local\nRemote body"),
        )]));

        drive_sync(dir.path(), &tracker, &sync_args(vec!["0001".to_owned()]));

        assert!(
            !fetch_all_contains(&tracker, "0001"),
            "a locally-resolvable target names no remote candidate, so the \
             confirmation gate issues no fetch_all for its token"
        );
    }

    #[test]
    fn a_mixed_found_and_absent_batch_writes_no_local_file() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(vec![(
            ExternalId::new("PP-999".to_owned()),
            issue("Found\nRemote body"),
        )]));

        drive_sync(
            dir.path(),
            &tracker,
            &sync_args(vec!["PP-999".to_owned(), "PP-404".to_owned()]),
        );

        assert!(
            new_work_files(dir.path()).is_empty(),
            "an absent token aborts the whole run before any write, so the \
             found target is not pulled either"
        );
    }

    fn sync_repo_integrated_with(integration: &str) -> tempfile::TempDir {
        let dir = sync_repo();
        std::fs::write(
            dir.path().join(".accelerator/config.md"),
            format!("---\nwork:\n  integration: {integration}\n---\n"),
        )
        .expect("write config");
        dir
    }

    #[test]
    fn a_targeted_pull_create_writes_no_projected_title_line() {
        for (integration, description) in [
            ("linear", "Remote body"),
            (
                "jira",
                "{\"content\":[{\"content\":[{\"text\":\"Remote body\",\
                 \"type\":\"text\"}],\"type\":\"paragraph\"}],\
                 \"type\":\"doc\",\"version\":1}",
            ),
        ] {
            let dir = sync_repo_integrated_with(integration);
            let tracker = Rc::new(RecordingTracker::holding(vec![(
                ExternalId::new("PP-999".to_owned()),
                issue(&format!("Imported\n{description}")),
            )]));

            drive_sync(
                dir.path(),
                &tracker,
                &sync_args(vec!["PP-999".to_owned()]),
            );

            let files = new_work_files(dir.path());
            let content = std::fs::read_to_string(
                dir.path().join("meta/work").join(&files[0]),
            )
            .expect("read created file");
            assert!(content.contains(description), "{integration}: {content}");
            assert!(
                !content.lines().any(|line| line == "Imported"),
                "{integration}: the bare title line leaked: {content}"
            );
        }
    }

    #[test]
    fn a_previewed_remote_only_target_writes_nothing_yet_confirms_it() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(vec![(
            ExternalId::new("PP-999".to_owned()),
            issue("Preview\nRemote body"),
        )]));
        let mut args = sync_args(vec!["PP-999".to_owned()]);
        args.preview = true;

        drive_sync(dir.path(), &tracker, &args);

        assert!(
            new_work_files(dir.path()).is_empty(),
            "a previewed pull writes no file"
        );
        assert!(
            fetch_all_contains(&tracker, "PP-999"),
            "preview still confirms the candidate through fetch_all"
        );
    }

    #[test]
    fn a_mixed_local_and_remote_only_run_excludes_the_non_targeted_item() {
        let dir = sync_repo();
        work_file(dir.path(), "0001", Some("PP-1"));
        work_file(dir.path(), "0002", Some("PP-2"));
        let non_targeted = dir.path().join("meta/work/0002-title.md");
        let before = std::fs::read_to_string(&non_targeted).expect("read 0002");
        let tracker = Rc::new(RecordingTracker::holding(vec![
            (
                ExternalId::new("PP-1".to_owned()),
                issue("One\nRemote body"),
            ),
            (
                ExternalId::new("PP-999".to_owned()),
                issue("New\nRemote body"),
            ),
        ]));

        drive_sync(
            dir.path(),
            &tracker,
            &sync_args(vec!["0001".to_owned(), "PP-999".to_owned()]),
        );

        assert_eq!(
            new_work_files(dir.path()).len(),
            3,
            "the two seeded files plus one pulled file for PP-999"
        );
        assert_eq!(
            std::fs::read_to_string(&non_targeted).expect("read 0002"),
            before,
            "the non-targeted item is left byte-identical"
        );
        assert!(
            !fetch_all_contains(&tracker, "PP-2"),
            "the non-targeted item's remote is never read"
        );
    }

    mod scope_dispatch {
        use std::collections::HashMap;

        use ::config::{
            ConfigAccess, ConfigError, Key, Level, Resolved, Scalar, Value,
        };

        use super::super::resolve_active_scope_key;

        struct FakeConfig(HashMap<String, String>);

        impl FakeConfig {
            fn with(pairs: &[(&str, &str)]) -> Self {
                Self(
                    pairs
                        .iter()
                        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                        .collect(),
                )
            }
        }

        impl ConfigAccess for FakeConfig {
            fn get(
                &self,
                key: &Key,
                _level: Option<Level>,
            ) -> Result<Resolved, ConfigError> {
                Ok(self.0.get(&key.to_string()).map_or(
                    Resolved::Absent,
                    |value| {
                        Resolved::Found(Value::Scalar(Scalar::String(
                            value.clone(),
                        )))
                    },
                ))
            }

            fn set(
                &self,
                _key: &Key,
                _value: &str,
                _level: Level,
            ) -> Result<(), ConfigError> {
                unreachable!("the scope resolver never writes config")
            }
        }

        fn no_integrations() -> std::path::PathBuf {
            std::path::PathBuf::from("/nonexistent-integrations-root")
        }

        #[test]
        fn jira_scopes_from_jira_project_key() {
            let config = FakeConfig::with(&[("jira.project_key", "OPS")]);
            assert_eq!(
                resolve_active_scope_key(&config, "jira", &no_integrations()),
                Some("OPS".to_owned())
            );
        }

        #[test]
        fn linear_scopes_from_linear_team_key() {
            let config = FakeConfig::with(&[("linear.team_key", "ENG")]);
            assert_eq!(
                resolve_active_scope_key(&config, "linear", &no_integrations()),
                Some("ENG".to_owned())
            );
        }

        #[test]
        fn a_scope_key_less_integration_reads_the_legacy_key_unchanged() {
            let config =
                FakeConfig::with(&[("work.default_project_code", "TR")]);
            assert_eq!(
                resolve_active_scope_key(&config, "trello", &no_integrations()),
                Some("TR".to_owned())
            );
        }

        #[test]
        fn an_unset_scope_yields_none() {
            let config = FakeConfig::with(&[]);
            assert_eq!(
                resolve_active_scope_key(&config, "jira", &no_integrations()),
                None
            );
        }

        #[test]
        fn a_divergent_work_key_does_not_affect_the_scope() {
            let config = FakeConfig::with(&[
                ("jira.project_key", "PROJ"),
                ("work.key", "PP"),
            ]);
            assert_eq!(
                resolve_active_scope_key(&config, "jira", &no_integrations()),
                Some("PROJ".to_owned()),
                "discovery scopes from the scope key, not work.key"
            );
        }
    }

    mod write_bounds {
        use super::super::{
            effective_max_pulls, effective_max_pushes, refusal_message,
            resolve_max_items, resolve_push_max_items,
        };
        use tracker::Ceiling;

        #[test]
        fn a_present_flag_overrides_the_configured_ceiling() {
            assert_eq!(
                effective_max_pulls(
                    Some(Ceiling::Bounded(3)),
                    Ceiling::Bounded(9)
                ),
                Ceiling::Bounded(3)
            );
            assert_eq!(
                effective_max_pushes(
                    Some(Ceiling::Bounded(4)),
                    Ceiling::Bounded(25)
                ),
                Ceiling::Bounded(4)
            );
        }

        #[test]
        fn an_unset_flag_defers_to_the_configured_ceiling() {
            assert_eq!(
                effective_max_pulls(None, Ceiling::Unlimited),
                Ceiling::Unlimited
            );
            assert_eq!(
                effective_max_pushes(None, Ceiling::Bounded(25)),
                Ceiling::Bounded(25)
            );
        }

        #[test]
        fn a_present_unlimited_flag_lifts_a_bounded_config() {
            assert_eq!(
                effective_max_pulls(
                    Some(Ceiling::Unlimited),
                    Ceiling::Bounded(9)
                ),
                Ceiling::Unlimited
            );
            assert_eq!(
                effective_max_pushes(
                    Some(Ceiling::Unlimited),
                    Ceiling::Bounded(25)
                ),
                Ceiling::Unlimited
            );
        }

        #[test]
        fn the_refusal_message_names_both_keys_their_files_and_unlimited() {
            let message = refusal_message(
                "linear",
                5,
                0,
                Ceiling::Bounded(3),
                Ceiling::Bounded(25),
                5,
                0,
                Some(::config::Level::Personal),
                None,
            );
            assert!(message.contains("linear.pull.max_items"), "{message}");
            assert!(message.contains("linear.push.max_items"), "{message}");
            assert!(
                message.contains(".accelerator/config.local.md"),
                "{message}"
            );
            assert!(message.contains("the built-in default"), "{message}");
            assert!(message.contains("unlimited"), "{message}");
        }

        fn resolve_from(
            team: &str,
            personal: Option<&str>,
            resolve: impl Fn(
                &dyn ::config::ConfigAccess,
                &str,
            ) -> (Ceiling, Option<::config::Level>),
        ) -> (Ceiling, Option<::config::Level>) {
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir_all(dir.path().join(".git")).expect("git");
            std::fs::create_dir_all(dir.path().join(".accelerator"))
                .expect("mkdir");
            std::fs::write(dir.path().join(".accelerator/config.md"), team)
                .expect("team config");
            if let Some(personal) = personal {
                use std::os::unix::fs::PermissionsExt as _;
                let path = dir.path().join(".accelerator/config.local.md");
                std::fs::write(&path, personal).expect("personal config");
                std::fs::set_permissions(
                    &path,
                    std::fs::Permissions::from_mode(0o600),
                )
                .expect("personal config must be private");
            }
            let composed = config_adapters::compose(
                dir.path(),
                config_adapters::LegacyPolicy::Reject,
            )
            .expect("compose");
            resolve(&composed.service, "jira")
        }

        #[test]
        fn an_unconfigured_max_items_is_the_built_in_default() {
            let team = "---\nwork:\n  integration: jira\n---\n";
            assert_eq!(
                resolve_from(team, None, resolve_max_items),
                (Ceiling::Bounded(25), None)
            );
            assert_eq!(
                resolve_from(team, None, resolve_push_max_items),
                (Ceiling::Bounded(25), None)
            );
        }

        #[test]
        fn a_team_pull_max_items_resolves_from_the_team_file() {
            assert_eq!(
                resolve_from(
                    "---\nwork:\n  integration: jira\njira:\n  pull:\n    \
                     max_items: 3\n---\n",
                    None,
                    resolve_max_items,
                ),
                (Ceiling::Bounded(3), Some(::config::Level::Team))
            );
        }

        #[test]
        fn a_team_push_max_items_resolves_from_the_team_file() {
            assert_eq!(
                resolve_from(
                    "---\nwork:\n  integration: jira\njira:\n  push:\n    \
                     max_items: 4\n---\n",
                    None,
                    resolve_push_max_items,
                ),
                (Ceiling::Bounded(4), Some(::config::Level::Team))
            );
        }

        #[test]
        fn a_personal_pull_block_shadows_the_team_block() {
            let (ceiling, level) = resolve_from(
                "---\nwork:\n  integration: jira\njira:\n  pull:\n    \
                 max_items: 3\n---\n",
                Some("---\njira:\n  pull:\n    max_items: unlimited\n---\n"),
                resolve_max_items,
            );
            assert_eq!(ceiling, Ceiling::Unlimited);
            assert_eq!(level, Some(::config::Level::Personal));
        }

        #[test]
        fn a_personal_push_block_shadows_the_team_block() {
            let (ceiling, level) = resolve_from(
                "---\nwork:\n  integration: jira\njira:\n  push:\n    \
                 max_items: 4\n---\n",
                Some("---\njira:\n  push:\n    max_items: unlimited\n---\n"),
                resolve_push_max_items,
            );
            assert_eq!(ceiling, Ceiling::Unlimited);
            assert_eq!(level, Some(::config::Level::Personal));
        }
    }
}

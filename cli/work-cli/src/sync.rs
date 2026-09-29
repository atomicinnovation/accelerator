//! `accelerator work sync`: drives the remote sync engine end to end.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use work::sync::PendingPush;
use work_adapters::sync::pending_push::Marker;

use ::config::ConfigAccess;
use corpus::store::AtomicWrite;
use corpus::IdOwnership;
use corpus::WorkItemIdScheme;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::FileRecoveryCopies;
use tracker::ExternalId;
use work::draft_id::DraftId;
use work::identity::IdentityField;
use work::retirement::RECOVERY_PARENT;
use work::retirement::RETIREMENT_INCOMPLETE;
use work::section_diff::SectionDiff;
use work::sync::Resolution;
use work::sync::RunClock;
use work::sync::SyncDirection;
use work::work_item_files::identity_of;
use work::work_item_files::WorkItemFiles;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::promotion_records::PromotionRecords;
use work_adapters::retirement::sweep_recoveries;
use work_adapters::retirement::RecoveryNotice;
use work_adapters::sync::baseline;
use work_adapters::sync::create::canonical_external_key;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::identity_settlement::IdentityOutcome;
use work_adapters::sync::identity_settlement::IdentityRow;
use work_adapters::sync::identity_settlement::SettlementPorts;
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

use crate::cli::SyncArgs;
use crate::exit_codes;
use crate::finaliser::FinishedRun;
use crate::finaliser::RunFinaliser;
use crate::identity_workspace::IdentityWorkspace;
use crate::promotion_report::detail_lines;
use crate::promotion_report::promotion_exit;
use crate::promotion_report::promotion_line;
use crate::resolve::IdentityCandidate;
use crate::resolve::RunOutcome;
use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;

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

fn warn_outstanding_pushes(integrations_root: &Path, integration: &str) {
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
                eprintln!(
                    "warning: {} could not be read ({}); a create or \
                     promotion it recorded may have partially applied",
                    unreadable.path.display(),
                    unreadable.detail
                );
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
                record.stage.key().map(tracker::ExternalId::as_str),
            ),
        };
        eprintln!(
            "warning: {} names a pending push for '{}' attempted at {}{}{}",
            path.display(),
            request.title,
            request.attempted_at,
            request
                .failure
                .as_deref()
                .map(|detail| format!(", failure: {detail}"))
                .unwrap_or_default(),
            external_id
                .map(|id| format!(", external_id: {id}"))
                .unwrap_or_default()
        );
    }
}

fn discover_items(work_dir: &Path) -> Result<Vec<LocalItem>, kernel::Error> {
    let files = FilesystemWorkItemFiles::new(work_dir).files()?;
    let mut items: Vec<LocalItem> = files
        .iter()
        .filter_map(identity_of)
        .map(|identity| LocalItem {
            id: identity.id,
            path: identity.path,
            external_id: identity.external_id.map(ExternalId::new),
        })
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(items)
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
}

impl CorpusDiscovery for WorkDirDiscovery<'_> {
    fn discover(&self) -> Result<DiscoveredCorpus, RunError> {
        Ok(DiscoveredCorpus {
            items: discover_items(self.work_dir).map_err(RunError::Internal)?,
            status: Box::new(VcsWorkingCopyStatus::probed_from(self.root)),
        })
    }
}

/// Clears recovery copies nobody needs any more and warns about every
/// restore still waiting for a person.
fn sweep_kept_recoveries(recovery: &FileRecoveryCopies) {
    let notices = match sweep_recoveries(recovery) {
        Ok(notices) => notices,
        Err(error) => {
            eprintln!("warning: recovery copies could not be checked: {error}");
            return;
        }
    };
    for notice in notices {
        match notice {
            RecoveryNotice::StillPending { dir, unrestored } => {
                let paths =
                    unrestored.iter().fold(String::new(), |listing, path| {
                        listing + "\n  " + &path.display().to_string()
                    });
                eprintln!(
                    "warning: {RETIREMENT_INCOMPLETE}: these paths still differ \
                     from their recovery copies in {}:{paths}\ncompare each path \
                     with its recovery copy and merge, then delete the copy",
                    recovery.location(&dir).display()
                );
            }
            RecoveryNotice::Cleared { dir } => eprintln!(
                "note: every path an incomplete restore left has been dealt \
                 with; removed {}",
                recovery.location(&dir).display()
            ),
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
                    unreadable.path.display(),
                    unreadable.detail
                )
            })
            .collect(),
        Err(error) => vec![format!(
            "warning: the promotion records could not be listed: {error}"
        )],
    }
}

/// Notes each retirement that completed after an incomplete restore; its
/// copies stay for reference until the next run removes them.
fn report_kept_recoveries(recovery: &FileRecoveryCopies) {
    use corpus::store::RecoveryCopies as _;

    let Ok(kept) = recovery.kept(Path::new(RECOVERY_PARENT)) else {
        return;
    };
    for kept in kept {
        if kept.state == corpus::store::KeptState::Completed {
            eprintln!(
                "note: the retirement whose restore was incomplete has since \
                 completed; its recovery copies remain in {} for reference \
                 and are removed on the next sync",
                recovery.location(&kept.dir).display()
            );
        }
    }
}

fn parse_resolutions(
    raw: &[(String, String)],
) -> Result<BTreeMap<String, Resolution>, String> {
    let mut resolutions = BTreeMap::new();
    for (id, token) in raw {
        if resolutions.contains_key(id) {
            return Err(format!(
                "--resolve names '{id}' more than once with contradictory \
                 orders"
            ));
        }
        let resolution = work::sync::resolve_conflict_token(token)
            .unwrap_or_else(|| {
                eprintln!(
                    "warning: --resolve {id}={token} — '{token}' is not a \
                     recognised order (accepted: remote, local, skip); \
                     treating as skip"
                );
                Resolution::Skip
            });
        resolutions.insert(id.clone(), resolution);
    }
    Ok(resolutions)
}

const fn action_keyword(action: work::sync::Action) -> &'static str {
    match action {
        work::sync::Action::Push => "push",
        work::sync::Action::Pull => "pull",
        work::sync::Action::SkipConflict => "skip-conflict",
        work::sync::Action::SkipDirty => "skip-dirty",
        work::sync::Action::Prompt => "unresolved",
        work::sync::Action::Noop => "noop",
        work::sync::Action::CreateFromRemote => "create-from-remote",
        work::sync::Action::CreateFromLocal => "create-from-local",
    }
}

/// Collapses tab, newline and carriage return to a space, so a transport
/// `detail` cannot break the single-record TSV format.
fn single_line(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
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
    }
}

/// The state an identity row shows: the engine's state for the item under
/// its settled id, or the state the identity change itself implies.
fn identity_state(row: &IdentityRow, report: &RunReport) -> String {
    report
        .reported
        .iter()
        .find(|item| item.planned.id == row.settled_id)
        .map_or_else(
            || match row.action {
                work::sync::IdentityAction::NotFound => {
                    work::sync::SyncState::RemoteAbsent.to_string()
                }
                work::sync::IdentityAction::KeyChanged
                | work::sync::IdentityAction::Resumed => "-".to_owned(),
            },
            |item| item.planned.state.to_string(),
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
            format!("{}\tfailed\t{state}\t{}", row.id, single_line(reason))
        }
        IdentityOutcome::Applied | IdentityOutcome::NotApplied => {
            format!("{}\t{}\t{state}\t{}", row.id, row.action, row.detail)
        }
    }
}

fn render_report(report: &RunReport) -> String {
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
        if settled_ids.contains(item.planned.id.as_str()) {
            continue;
        }
        if matches!(item.planned.state, work::sync::SyncState::Synced) {
            synced_count += 1;
            continue;
        }
        let (action_field, detail) = match &item.outcome {
            ItemOutcome::Failed(error) => (
                "failed",
                match error.class() {
                    Some(
                        work_adapters::sync::apply::FailureClass::Retryable,
                    ) => "retryable",
                    Some(
                        work_adapters::sync::apply::FailureClass::Terminal,
                    ) => "terminal",
                    Some(
                        work_adapters::sync::apply::FailureClass::Unconfigured,
                    ) => "unconfigured",
                    Some(
                        work_adapters::sync::apply::FailureClass::Rejected,
                    ) => "rejected",
                    None => "-",
                },
            ),
            _ => (action_keyword(item.planned.action), "-"),
        };
        lines.push(format!(
            "{}\t{}\t{}\t{}",
            item.planned.id, action_field, item.planned.state, detail
        ));
    }
    // Capture before appending the always-present discovery line, which would
    // otherwise make `is_empty()` false and drop the empty-run summary.
    let summary_needed = synced_count > 0 || lines.is_empty();
    lines.sort();
    lines.push(discovery_line(&report.discovery));
    let mut promotions: Vec<_> = report.promotions.iter().collect();
    promotions.sort_by(|left, right| left.draft.cmp(&right.draft));
    lines.extend(promotions.into_iter().flat_map(detail_lines));
    if report.deferred > 0 {
        lines.push(format!(
            "#\tnote\tdeferred-to-next-run\t{}",
            report.deferred
        ));
    }
    if summary_needed {
        lines.push(format!("#\tsummary\tsynced\t{synced_count}"));
    }
    lines.join("\n")
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
        work_adapters::sync::apply::FailureClass::Rejected => {
            exit_codes::REJECTED
        }
    }
}

fn exit_code_for_report(report: &RunReport) -> u8 {
    let failures =
        report
            .reported
            .iter()
            .filter_map(|item| match &item.outcome {
                ItemOutcome::Failed(error) => error.class().map(failure_code),
                ItemOutcome::Applied | ItemOutcome::NotApplied => None,
            });
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

fn id_is_token_safe(scheme: &WorkItemIdScheme, id: &str) -> bool {
    scheme.is_canonical_id_token(id)
        || (scheme.ownership() == IdOwnership::Tracker
            && (corpus::is_tracker_key(id) || DraftId::parse(id).is_some()))
}

fn clear_stale_dossiers(dir: &Path, scheme: &WorkItemIdScheme) {
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
            .is_some_and(|stem| id_is_token_safe(scheme, stem));
        let is_write_artefact = name.starts_with(store::TEMP_PREFIX);
        if is_stale_dossier || is_write_artefact {
            let _ = std::fs::remove_file(&path);
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
    dir: &Path,
    scheme: &WorkItemIdScheme,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let gitignore = dir.join(".gitignore");
    std::fs::write(&gitignore, "*\n")?;
    if !std::fs::read_to_string(&gitignore)?.contains('*') {
        return Err(std::io::Error::other(
            "the conflicts .gitignore could not be verified",
        ));
    }
    clear_stale_dossiers(dir, scheme);
    Ok(())
}

fn persist_dossiers(
    dossiers: &[ConflictDossier],
    dir: &Path,
    scheme: &WorkItemIdScheme,
    render: &dyn Fn(&SectionDiff) -> String,
) {
    let store = FileCorpusStore::new(dir);
    for dossier in dossiers {
        if !id_is_token_safe(scheme, &dossier.id) {
            eprintln!(
                "warning: skipping dossier for unsafe id {:?}",
                dossier.id
            );
            continue;
        }
        let body = match render_dossier(dossier, render) {
            DossierRender::Renderable(text)
            | DossierRender::Unrenderable(text) => text,
        };
        let path = dir.join(format!("{}.md", dossier.id));
        if let Err(error) = store.write(&path, body.as_bytes()) {
            eprintln!(
                "warning: could not write conflict dossier {}: {error}",
                path.display()
            );
        }
    }
}

/// The reset-then-write over the shared `conflicts/` directory is not
/// lock-guarded here: `/sync-work-items` issues one `work sync` at a time and
/// the two-invocation shape is sequential, so a stale file from a racing run
/// is overwritten before it is read.
fn persist_conflict_dossiers(
    dir: &Path,
    dossiers: &[ConflictDossier],
    scheme: &WorkItemIdScheme,
    render: &dyn Fn(&SectionDiff) -> String,
) {
    match prepare_conflicts_dir(dir, scheme) {
        Ok(()) => persist_dossiers(dossiers, dir, scheme, render),
        Err(error) => eprintln!(
            "warning: conflict dossiers not written — could not guarantee an \
             ignored {} ({error})",
            dir.display()
        ),
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

enum Scope {
    All,
    Targeted,
}

struct SelectedTargets {
    scope: Scope,
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
    corpus: &[LocalItem],
    targets: &[String],
    direction: SyncDirection,
    resolver: &dyn Fn(&str) -> RunOutcome,
) -> Result<SelectedTargets, ExitCode> {
    if targets.is_empty() {
        return Ok(SelectedTargets {
            scope: Scope::All,
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
                    eprintln!("{}", push_only_remote_only(candidate).message());
                }
                return Err(ExitCode::from(exit_codes::USAGE));
            }
            Ok(SelectedTargets {
                scope: Scope::Targeted,
                items: found.items,
                remote_candidates: found.remote_candidates,
            })
        }
        Err(failures) => {
            for failure in &failures {
                eprintln!("{}", failure.message());
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
    let Some(tracker) =
        tracker_support::pull::Tracker::from_integration(integration)
    else {
        return Ok(());
    };
    let key = ::config::Key::parse(&format!("{integration}.pull"))
        .map_err(|error| error.to_string())?;
    let ::config::Resolved::Found(value) =
        config.get(&key, None).map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    if matches!(&value, ::config::Value::Mapping(entries) if entries.is_empty())
    {
        return Ok(());
    }
    let level = match config
        .effective(&key, None)
        .map_err(|error| error.to_string())?
        .source()
    {
        ::config::Source::Personal => ::config::Level::Personal,
        _ => ::config::Level::Team,
    };
    let parsed = tracker_support::pull::parse(&value)
        .map_err(|error| error.detail(level))?;
    tracker_support::pull::validate(&parsed, tracker)
        .map_err(|error| error.detail(level))
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
    match tracker_support::push::read(config, integration)? {
        Some((push, level)) => tracker_support::push::validate(&push)
            .map_err(|error| error.detail(level)),
        None => Ok(()),
    }
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

/// # Errors
///
/// Never returns `Err`; every failure is reported through the exit code.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn run_sync(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &SyncArgs,
    registry: &dyn TrackerRegistry,
    finaliser: &dyn RunFinaliser,
) -> ExitCode {
    let direction = if args.push_only {
        SyncDirection::PushOnly
    } else if args.pull_only {
        SyncDirection::PullOnly
    } else {
        SyncDirection::Bidirectional
    };

    let resolutions = match parse_resolutions(&args.resolutions) {
        Ok(resolutions) => resolutions,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(exit_codes::USAGE);
        }
    };

    let integration =
        match crate::config::effective_nonempty(config, "work.integration") {
            Ok(value) => value,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(exit_codes::ERROR);
            }
        };

    if let Err(message) = validate_pull_config(config, &integration) {
        eprintln!("{message}");
        return ExitCode::from(exit_codes::ERROR);
    }

    if let Err(message) = validate_push_config(config, &integration) {
        eprintln!("{message}");
        return ExitCode::from(exit_codes::ERROR);
    }

    // The directory resolution and target validation run before the tracker's
    // credential check, so a target-resolution abort is credential-independent.
    let root = config_adapters::FileConfigStore::discover_root(start);
    let repo_root = root.clone();
    let work_dir = match crate::config::resolve_work_dir(config, &root) {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let integrations_root = match integrations_dir(config, &root) {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(exit_codes::ERROR);
        }
    };

    let items = match discover_items(&work_dir) {
        Ok(items) => items,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(exit_codes::ERROR);
        }
    };

    let scheme = match crate::config::resolve_scheme(config) {
        Ok(scheme) => scheme,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let canonical_work_dir = match work_dir.canonicalize() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!(
                "could not resolve the work directory {}: {error}",
                work_dir.display()
            );
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let resolver = |token: &str| {
        crate::resolve::resolve_with(&scheme, &canonical_work_dir, start, token)
    };
    let selected =
        match build_selection(&items, &args.targets, direction, &resolver) {
            Ok(selected) => selected,
            Err(code) => return code,
        };

    let tracker = match registry.resolve(&integration) {
        Ok(tracker) => tracker,
        Err(
            error @ (SelectionError::Unset
            | SelectionError::Unrecognised { .. }),
        ) => {
            eprintln!("{}", error.message());
            return ExitCode::from(exit_codes::UNRECOGNISED);
        }
        Err(error @ SelectionError::NotAvailable { .. }) => {
            eprintln!("{}", error.message());
            return ExitCode::from(exit_codes::NOT_AVAILABLE);
        }
        Err(error @ SelectionError::Unconfigured { .. }) => {
            eprintln!("{}", error.message());
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
                    eprintln!("{}", failure.message());
                }
                return ExitCode::from(highest_precedence_code(&failures));
            }
        }
    };

    let workspace = match IdentityWorkspace::open(
        config,
        &repo_root,
        &work_dir,
        &integrations_root,
        &integration,
    ) {
        Ok(workspace) => workspace,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    let baseline_path = workspace.baseline_path();
    let baseline_dir = baseline_path.parent().unwrap_or(&integrations_root);
    let corpus_store = FileCorpusStore::new(baseline_dir);
    let project_store = FileCorpusStore::new(&repo_root);
    let state_store = FileCorpusStore::new(workspace.state_dir());
    let integrations_store = FileCorpusStore::new(&integrations_root);
    let mut baseline_store = workspace.baseline(&corpus_store);
    let retirement_baseline = workspace.baseline(&corpus_store);
    let retirement =
        workspace.retirement_ports(&project_store, &retirement_baseline);
    let records = workspace.retirement_records(&state_store);
    let promotions = workspace.promotion_records(&integrations_store);
    let probe_status = || -> Box<dyn WorkingCopyStatus> {
        Box::new(VcsWorkingCopyStatus::probed_from(&repo_root))
    };
    let settlement = SettlementPorts {
        retirement: &retirement,
        records: &records,
        promotions: &promotions,
        ownership: scheme.ownership(),
        state_dir: workspace.state_dir(),
        probe_status: &probe_status,
    };
    let recovery = workspace.recovery();
    let discovery = WorkDirDiscovery {
        work_dir: &work_dir,
        root: &repo_root,
    };
    let clock = SystemClock;
    let author = crate::sync_author::ConfiguredLocalAuthor::new(
        config,
        root,
        work_dir.clone(),
    );

    let ports = SyncPorts {
        tracker: tracker.as_ref(),
        status: workspace.status(),
        writer: &corpus_store,
        clock: &clock,
        author: &author,
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
        Scope::All => ItemSelection::All,
        Scope::Targeted => ItemSelection::Targeted {
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
        eprintln!("{}", unbounded_gate_message(&integration));
        return ExitCode::from(exit_codes::REFUSED_UNBOUNDED);
    }
    let request = SyncRequest {
        corpus: &items,
        selection,
        direction,
        strategy,
        resolutions: &resolutions,
        max_pulls,
        max_pushes,
        mode,
        integrations_root: &integrations_root,
        integration: &integration,
        scope,
        promote: !args.no_promote,
    };

    sweep_kept_recoveries(recovery);
    for warning in unreadable_promotion_record_warnings(&promotions) {
        eprintln!("{warning}");
    }
    match run_settled(
        &request,
        &ports,
        &settlement,
        &mut baseline_store,
        &discovery,
    ) {
        Ok(report) => {
            if let work_adapters::sync::baseline::Degradation::Unparseable {
                detail,
            } = &report.baseline_degradation
            {
                eprintln!(
                    "warning: {} could not be parsed ({detail}); treating \
                     as empty",
                    baseline::path(&integrations_root, &integration).display()
                );
            }
            if let work_adapters::sync::baseline::Degradation::EntriesDiscarded {
                ids,
            } = &report.baseline_degradation
            {
                eprintln!(
                    "warning: baseline entries discarded (malformed): {}",
                    ids.join(", ")
                );
            }
            if report.keyed_read_budget_limited {
                eprintln!(
                    "warning: the keyed reconcile read was cut short by a \
                     transient budget limit (a deadline or wire failure); the \
                     affected items are indeterminate this run and will \
                     reconcile on a retry."
                );
            }
            let conflicts_dir =
                integrations_root.join(&integration).join("conflicts");
            match crate::config::resolve_scheme(config) {
                Ok(scheme) => persist_conflict_dossiers(
                    &conflicts_dir,
                    &report.dossiers,
                    &scheme,
                    &work_adapters::diff::render,
                ),
                Err(error) => eprintln!(
                    "warning: conflict dossiers not written — could not \
                     resolve the work-item id scheme ({error})"
                ),
            }
            println!("{}", render_report(&report));
            finaliser.finalise(
                &FinishedRun {
                    report: &report,
                    mode,
                    corpus_external_ids: items
                        .iter()
                        .filter_map(|item| item.external_id.clone())
                        .collect(),
                    integrations_root: &integrations_root,
                    repo_root: &repo_root,
                },
                &mut std::io::stderr(),
            );
            warn_outstanding_pushes(&integrations_root, &integration);
            report_kept_recoveries(recovery);
            ExitCode::from(exit_code_for_report(&report))
        }
        Err(SettledRunFailure {
            error,
            identity_applied,
        }) => {
            if identity_applied > 0 {
                println!(
                    "#\tnote\tidentity-applied-before-refusal\t\
                     {identity_applied}"
                );
            }
            let (message, code) = run_error_outcome(
                &error,
                &RunErrorContext {
                    config,
                    integration: &integration,
                    max_pulls_source,
                    max_pushes_source,
                },
            );
            eprintln!("{message}");
            report_kept_recoveries(recovery);
            ExitCode::from(code)
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unnecessary_wraps,
    clippy::unimplemented
)]
mod tests {
    use corpus::WorkItemIdScheme;
    use tracker::RemoteTimestamp;
    use tracker::TrackerError;
    use work::section_diff::SectionDiff;
    use work::sync::Action;
    use work::sync::PlannedAction;
    use work::sync::SyncState;
    use work_adapters::sync::apply::ApplyError;
    use work_adapters::sync::baseline::Degradation;
    use work_adapters::sync::run::ConflictDossier;
    use work_adapters::sync::run::DiscoveryStatus;
    use work_adapters::sync::run::ItemOutcome;
    use work_adapters::sync::run::ReportedItem;
    use work_adapters::sync::run::RunReport;

    use std::path::Path;
    use std::path::PathBuf;

    use tracker::ExternalId;
    use work_adapters::sync::fetch::LocalItem;

    use work::sync::SyncDirection;

    use super::build_selection;
    use super::highest_precedence_code;
    use super::partition_candidates;
    use super::render_report;
    use super::resolve_targets;
    use super::unbounded_gate_fires;
    use super::Scope;
    use super::TargetResolutionFailure;
    use crate::exit_codes;
    use crate::resolve::RunOutcome;

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
            super::discover_items(&self.work_dir).expect("discover items")
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

        let outcome = build_selection(
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

        let selected = build_selection(
            &corpus,
            &[],
            SyncDirection::Bidirectional,
            &resolver,
        )
        .expect("no targets is a whole-corpus run");

        assert!(matches!(selected.scope, Scope::All));
        assert!(selected.items.is_empty());
    }

    fn ok_render(section: &SectionDiff) -> String {
        format!("=== {} (- LOCAL / + REMOTE) ===\nbody\n\n", section.name)
    }

    fn conflict_dossier(id: &str, local_unreadable: bool) -> ConflictDossier {
        ConflictDossier {
            id: id.to_owned(),
            title: "Title".to_owned(),
            local_modified: Some(1_700_000_000),
            remote_updated: RemoteTimestamp::Reported(
                "2026-07-01T00:00:00Z".to_owned(),
            ),
            sections: vec![SectionDiff {
                name: "(preamble)".to_owned(),
                local: "local".to_owned(),
                remote: "remote".to_owned(),
            }],
            local_unreadable,
        }
    }

    fn reported(id: &str, state: SyncState, action: Action) -> ReportedItem {
        ReportedItem {
            planned: PlannedAction {
                id: id.to_owned(),
                state,
                action,
            },
            outcome: ItemOutcome::NotApplied,
            validation: None,
        }
    }

    fn failed(
        id: &str,
        state: SyncState,
        source: TrackerError,
    ) -> ReportedItem {
        ReportedItem {
            planned: PlannedAction {
                id: id.to_owned(),
                state,
                action: Action::Noop,
            },
            outcome: ItemOutcome::Failed(ApplyError::Tracker {
                item_id: id.to_owned(),
                operation: "update",
                source,
            }),
            validation: None,
        }
    }

    #[test]
    fn render_report_sorts_fixed_width_ids_numerically() {
        let report = RunReport {
            identity: Vec::new(),
            promotions: Vec::new(),
            deferred: 0,
            reported: vec![
                reported("0001", SyncState::LocallyModified, Action::Push),
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
            ],
            read_failure: None,
            baseline_degradation: Degradation::None,
            finalised: true,
            dossiers: Vec::new(),
            discovery: DiscoveryStatus::Ran { found: 0 },
            keyed_read_budget_limited: false,
        };

        let golden = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/sync-report.golden"
        ))
        .expect("golden readable");

        assert_eq!(render_report(&report), golden);
    }

    #[test]
    fn render_report_emits_the_summary_row_for_an_empty_corpus() {
        let report = RunReport {
            identity: Vec::new(),
            promotions: Vec::new(),
            deferred: 0,
            reported: Vec::new(),
            read_failure: None,
            baseline_degradation: Degradation::None,
            finalised: true,
            dossiers: Vec::new(),
            discovery: DiscoveryStatus::SkippedPushOnly,
            keyed_read_budget_limited: false,
        };

        assert_eq!(
            render_report(&report),
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
            vec![reported("ENG-42", SyncState::LocallyModified, Action::Push)],
        );

        let rendered = render_report(&report);

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

        let rendered = render_report(&report);

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
            vec![reported("0230", SyncState::LocallyModified, Action::Push)],
        );

        let rendered = render_report(&report);

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

        let rendered = render_report(&report);

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

        let rendered = render_report(&report);

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

        assert!(
            render_report(&report).contains("#\tnote\tdeferred-to-next-run\t2"),
        );
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

    fn report_with(discovery: DiscoveryStatus) -> RunReport {
        RunReport {
            identity: Vec::new(),
            promotions: Vec::new(),
            deferred: 0,
            reported: Vec::new(),
            read_failure: None,
            baseline_degradation: Degradation::None,
            finalised: true,
            dossiers: Vec::new(),
            discovery,
            keyed_read_budget_limited: false,
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
            render_report(report).lines().map(str::to_owned).collect()
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
                Action::Push,
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
                let from_create =
                    crate::create::create_outcome_of(&reason).exit_code();
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
        assert!(
            render_report(&report_with(DiscoveryStatus::Ran { found: 3 }))
                .contains("#\tdiscovery\tran\tfound=3")
        );
        assert!(
            render_report(&report_with(DiscoveryStatus::SkippedPushOnly))
                .contains("#\tdiscovery\tskipped\tpush-only")
        );
        assert!(
            render_report(&report_with(DiscoveryStatus::SkippedTargeted))
                .contains("#\tdiscovery\tskipped\ttargeted")
        );
        assert!(render_report(&report_with(DiscoveryStatus::TargetedPull {
            attempted: 2
        }))
        .contains("#\tdiscovery\ttargeted-pull\t2"));
        let failed = render_report(&report_with(DiscoveryStatus::Failed {
            detail: "connection refused".to_owned(),
        }));
        assert!(
            failed.contains("#\tdiscovery\tfailed\tconnection refused"),
            "{failed}"
        );
    }

    #[test]
    fn a_failed_discovery_detail_is_flattened_to_one_record() {
        let rendered = render_report(&report_with(DiscoveryStatus::Failed {
            detail: "line one\tsplit\nline two\r".to_owned(),
        }));
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
            identity: Vec::new(),
            promotions: Vec::new(),
            deferred: 0,
            reported: vec![failed(
                "0001",
                SyncState::LocallyModified,
                unconfigured("no team in scope"),
            )],
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        };

        assert!(render_report(&report)
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
            identity: Vec::new(),
            promotions: Vec::new(),
            deferred: 0,
            reported: vec![failed(
                "0001",
                SyncState::Unsynced,
                rejected("jira create: the body has a table"),
            )],
            ..report_with(DiscoveryStatus::Ran { found: 0 })
        };

        assert!(
            render_report(&report).contains("0001\tfailed\tunsynced\trejected")
        );
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
                identity: Vec::new(),
                promotions: Vec::new(),
                deferred: 0,
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
                identity: Vec::new(),
                promotions: Vec::new(),
                deferred: 0,
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

    #[test]
    fn id_is_token_safe_admits_only_canonical_ids() {
        let scheme = scheme();
        assert!(super::id_is_token_safe(&scheme, "0001"));
        assert!(!super::id_is_token_safe(&scheme, "../foo"));
        assert!(!super::id_is_token_safe(&scheme, "a/b"));
        assert!(!super::id_is_token_safe(&scheme, "0001; rm -rf ~"));
        assert!(!super::id_is_token_safe(&scheme, "1"));
        assert!(!super::id_is_token_safe(&scheme, ""));
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

        super::persist_dossiers(
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

        super::persist_dossiers(
            &[conflict_dossier("draft-k7mq3x", false)],
            dir.path(),
            &tracker_scheme(),
            &ok_render,
        );

        assert_eq!(md_files(dir.path()), vec!["draft-k7mq3x.md"]);
    }

    #[test]
    fn a_tracker_key_is_not_token_safe_under_a_local_pattern() {
        assert!(!super::id_is_token_safe(&scheme(), "ENG-42"));
        assert!(!super::id_is_token_safe(&scheme(), "draft-k7mq3x"));
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

        super::persist_dossiers(&dossiers, dir.path(), &scheme(), &ok_render);

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

        super::persist_conflict_dossiers(
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
            conflicts.join(format!("{}sweep", store::TEMP_PREFIX)),
            "artefact",
        )
        .expect("seed");

        super::persist_conflict_dossiers(
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
                .join(format!("{}sweep", store::TEMP_PREFIX))
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

        super::persist_conflict_dossiers(
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
    // Linear/Jira. `run_sync` returns an opaque `ExitCode`, so they assert the
    // observable effects — files written or not, the fetch_all call made or not
    // — while the exit-code chain is proven by the pure-logic tests above.

    use std::cell::RefCell;
    use std::process::ExitCode;
    use std::rc::Rc;

    use tracker::RemoteIssue;
    use tracker::RemoteTracker;
    use tracker_test_support::Call;
    use tracker_test_support::RecordingTracker;

    use crate::cli::SyncArgs;
    use crate::finaliser::FinishedRun;
    use crate::finaliser::NoFinaliser;
    use crate::finaliser::RunFinaliser;
    use crate::test_support::StubRegistry;

    /// A tracker whose `fetch_all` fails pre-flight — the whole-call `Err` path
    /// `RecordingTracker` cannot produce. Only `fetch_all` is ever called on it.
    struct FetchFailingTracker;

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
            _title: &str,
            _body: &str,
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
            _title: &str,
            _body: &str,
        ) -> tracker::ValidationOutcome {
            unimplemented!("not exercised by the fetch_all failure path")
        }
    }

    fn sync_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .status()
                .expect("git invocation");
        };
        git(&["init", "-q"]);
        git(&["config", "user.name", "Test User"]);
        git(&["config", "user.email", "test@example.com"]);
        std::fs::create_dir_all(dir.path().join("meta/work")).expect("mkdir");
        std::fs::create_dir_all(dir.path().join(".accelerator"))
            .expect("mkdir");
        std::fs::write(
            dir.path().join(".accelerator/config.md"),
            "---\nwork:\n  integration: jira\n---\n",
        )
        .expect("write config");
        dir
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
        let composed = config_adapters::compose(
            dir,
            config_adapters::LegacyPolicy::Reject,
        )
        .expect("compose the test config");
        let registry = StubRegistry(Rc::clone(tracker));
        super::run_sync(dir, &composed.service, args, &registry, finaliser)
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

    #[test]
    fn a_create_then_sync_reports_synced() {
        let dir = sync_repo();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let composed = config_adapters::compose(
            dir.path(),
            config_adapters::LegacyPolicy::Reject,
        )
        .expect("compose the test config");
        let templates = composed.store.with_plugin_root(Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        ));
        let created = crate::create::run(
            dir.path(),
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
            &StubRegistry(Rc::clone(&tracker)),
        );
        assert!(matches!(
            created,
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

    /// Keeps what the run handed it, and can report a failure of its own.
    #[derive(Default)]
    struct RecordingFinaliser {
        corpus: RefCell<Vec<String>>,
        imports: RefCell<Vec<String>>,
        complains: bool,
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
            if self.complains {
                let _ = writeln!(diagnostics, "warning: finaliser failed");
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
                complains: true,
                ..RecordingFinaliser::default()
            },
        );

        assert_eq!(observed, expected);
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

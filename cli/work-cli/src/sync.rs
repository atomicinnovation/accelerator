//! `accelerator work sync`: drives the remote sync engine end to end.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use ::config::ConfigAccess;
use corpus::store::AtomicWrite;
use corpus::WorkItemIdScheme;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::RealFs;
use tracker::ExternalId;
use work::section_diff::SectionDiff;
use work::sync::Resolution;
use work::sync::RunClock;
use work::sync::SyncDirection;
use work_adapters::sync::baseline;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::create::canonical_external_key;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
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
use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

use crate::cli::SyncArgs;
use crate::exit_codes;
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
    for (path, marker) in markers {
        let (request, external_id) = match &marker {
            work::sync::PendingPush::Attempted { request } => (request, None),
            work::sync::PendingPush::Created {
                request,
                external_id,
            } => (request, Some(external_id.as_str())),
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

fn discover_items(work_dir: &Path) -> Vec<LocalItem> {
    let Ok(entries) = std::fs::read_dir(work_dir) else {
        return Vec::new();
    };
    let mut items: Vec<LocalItem> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(std::ffi::OsStr::to_str) == Some("md")
        })
        .filter_map(|path| {
            let content = std::fs::read_to_string(&path).ok()?;
            let (frontmatter, _) =
                work_adapters::sync::digest::split_frontmatter_and_body(
                    &content,
                )
                .ok()?;
            let id = work::show::read_field_raw(&frontmatter, "id")?;
            let external_id =
                work::show::read_field_raw(&frontmatter, "external_id")
                    .filter(|raw| {
                        !raw.trim_matches(|c: char| {
                            c.is_ascii_whitespace() || c == '"' || c == '\''
                        })
                        .is_empty()
                    })
                    .map(ExternalId::new);
            Some(LocalItem {
                id,
                path,
                external_id,
            })
        })
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    items
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

fn render_report(report: &RunReport) -> String {
    let mut lines = Vec::new();
    let mut synced_count = 0usize;
    for item in &report.reported {
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
    if summary_needed {
        lines.push(format!("#\tsummary\tsynced\t{synced_count}"));
    }
    lines.join("\n")
}

fn exit_code_for_report(report: &RunReport) -> u8 {
    let any_terminal = report.reported.iter().any(|item| {
        matches!(
            item.outcome,
            ItemOutcome::Failed(ref error)
                if error.class() == Some(work_adapters::sync::apply::FailureClass::Terminal)
        )
    });
    let any_retryable = report.reported.iter().any(|item| {
        matches!(
            item.outcome,
            ItemOutcome::Failed(ref error)
                if error.class() == Some(work_adapters::sync::apply::FailureClass::Retryable)
        )
    });
    let awaiting_human = report.awaiting_human().next().is_some();

    if any_terminal {
        exit_codes::TERMINAL
    } else if awaiting_human {
        exit_codes::UNRESOLVED
    } else if any_retryable
        || report.read_failure.is_some()
        || matches!(report.discovery, DiscoveryStatus::Failed { .. })
    {
        exit_codes::RETRYABLE
    } else {
        exit_codes::CLEAN
    }
}

fn id_is_token_safe(scheme: &WorkItemIdScheme, id: &str) -> bool {
    scheme.is_canonical_id_token(id)
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
    PushOnlyRemoteOnly(String),
    Absent(String),
    Indeterminate(String),
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
            | Self::PushOnlyRemoteOnly(message)
            | Self::Absent(message)
            | Self::Indeterminate(message) => message,
        }
    }

    const fn exit_code(&self) -> u8 {
        match self {
            Self::Malformed(_)
            | Self::AmbiguousLocal(_)
            | Self::AmbiguousExternal(_)
            | Self::LocalCollision(_)
            | Self::PushOnlyRemoteOnly(_) => exit_codes::USAGE,
            Self::Unmanaged(_) | Self::Absent(_) => {
                exit_codes::RESOLVE_NOT_FOUND
            }
            Self::OutsideWorkDir(_) => exit_codes::RESOLVE_OUTSIDE_WORKDIR,
            Self::Indeterminate(_) => exit_codes::RETRYABLE,
        }
    }
}

fn external_id_index(
    corpus: &[LocalItem],
) -> BTreeMap<String, Vec<&LocalItem>> {
    let mut index: BTreeMap<String, Vec<&LocalItem>> = BTreeMap::new();
    for item in corpus {
        if let Some(external) = &item.external_id {
            index
                .entry(canonical_external_key(external))
                .or_default()
                .push(item);
        }
    }
    index
}

/// The other local file a dual-shape token collides with: the token is
/// `local_match`'s local id and simultaneously a *different* file's
/// `external_id`. A genuine local/local collision, decidable entirely from the
/// corpus, so the caller can name both files without any remote call.
fn colliding_file<'a>(
    index: &BTreeMap<String, Vec<&'a LocalItem>>,
    token: &str,
    local_match: &LocalItem,
) -> Option<&'a LocalItem> {
    let key = canonical_external_key(&ExternalId::new(token.to_owned()));
    index
        .get(&key)?
        .iter()
        .find(|item| item.id != local_match.id)
        .copied()
}

/// Resolves each `--target` token to a local item or a remote candidate,
/// accumulating every failure so one run names all offenders. Local resolution
/// wins; a token that resolves locally to `NotFound`/`Invalid` cascades to the
/// `external_id` index, and a token that matches nothing locally becomes a
/// remote candidate rather than an abort. An ambiguous or out-of-directory
/// local outcome still fails.
fn resolve_targets(
    corpus: &[LocalItem],
    targets: &[String],
    resolver: &dyn Fn(&str) -> RunOutcome,
) -> Result<ResolvedTargets, Vec<TargetResolutionFailure>> {
    let index = external_id_index(corpus);
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
                    Some(item) => match colliding_file(&index, token, item) {
                        Some(other) => failures.push(
                            TargetResolutionFailure::LocalCollision(format!(
                                "'{token}' is the local id of {} and also \
                                 the external_id recorded by {}; re-run with \
                                 the path of the file you intended",
                                item.path.display(),
                                other.path.display()
                            )),
                        ),
                        None => matched.push(item),
                    },
                    None => failures.push(TargetResolutionFailure::Unmanaged(
                        format!(
                            "'{token}' resolves to a file that is not a \
                             managed work item"
                        ),
                    )),
                }
            }
            RunOutcome::Ambiguous(_) => {
                failures.push(TargetResolutionFailure::AmbiguousLocal(
                    format!(
                    "'{token}' is an ambiguous local id; re-run with a full \
                     id or a path"
                ),
                ));
            }
            RunOutcome::OutsideWorkDir(message) => {
                failures.push(TargetResolutionFailure::OutsideWorkDir(message));
            }
            RunOutcome::NotFound(_) | RunOutcome::Invalid(_) => {
                let key =
                    canonical_external_key(&ExternalId::new(token.to_owned()));
                match index.get(&key).map(Vec::as_slice) {
                    Some([one]) => matched.push(one),
                    None | Some([]) => {
                        if candidate_keys.insert(key) {
                            remote_candidates
                                .push(ExternalId::new(token.clone()));
                        }
                    }
                    Some(_) => failures.push(
                        TargetResolutionFailure::AmbiguousExternal(format!(
                            "'{token}' matches more than one item's \
                             external_id; re-run with a local id or a path"
                        )),
                    ),
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

    // The directory resolution and target validation run before the tracker's
    // credential check, so a target-resolution abort is credential-independent.
    let root = config_adapters::FileConfigStore::discover_root(start);
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

    let items = discover_items(&work_dir);

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

    let baseline_path = baseline::path(&integrations_root, &integration);
    let file_reader = RealFs;
    let corpus_store = FileCorpusStore::new(
        baseline_path.parent().unwrap_or(&integrations_root),
    );
    let mut baseline_store =
        BaselineStore::new(baseline_path, &file_reader, &corpus_store);
    let status = VcsWorkingCopyStatus::probed_from(&root);
    let clock = SystemClock;
    let author =
        crate::sync_author::ConfiguredLocalAuthor::new(config, root, work_dir);

    let ports = SyncPorts {
        tracker: tracker.as_ref(),
        status: &status,
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
    let default_project =
        crate::config::effective_nonempty(config, "work.default_project_code")
            .unwrap_or_default();
    let scope = tracker::SearchScope {
        project: (!default_project.is_empty()).then_some(default_project),
        all_projects: false,
        filters: Vec::new(),
    };
    let selection = match selected.scope {
        Scope::All => ItemSelection::All,
        Scope::Targeted => ItemSelection::Targeted {
            items: &selected.items,
            pull_ids: &pull_ids,
        },
    };
    let request = SyncRequest {
        corpus: &items,
        selection,
        direction,
        strategy,
        resolutions: &resolutions,
        max_pulls: args.max_pulls,
        max_pushes: args.max_pushes,
        mode,
        integrations_root: &integrations_root,
        integration: &integration,
        scope,
    };

    match work_adapters::sync::run::run(&ports, &mut baseline_store, &request) {
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
            warn_outstanding_pushes(&integrations_root, &integration);
            ExitCode::from(exit_code_for_report(&report))
        }
        Err(RunError::Refused {
            pulls,
            pushes,
            max_pulls,
            max_pushes,
            new_local_files,
            new_remote_issues,
        }) => {
            eprintln!(
                "refused: this run would pull {pulls} item(s) ({new_local_files} \
                 of them new local files, limit {max_pulls}) and push {pushes} \
                 item(s) ({new_remote_issues} of them new remote issues, limit \
                 {max_pushes}). Scope the search or raise the limit with \
                 --max-pulls/--max-pushes, or inspect the plan first with \
                 --preview."
            );
            ExitCode::from(exit_codes::REFUSED_BULK_OVERWRITE)
        }
        Err(RunError::DiscoveryIncomplete { found }) => {
            eprintln!(
                "refused: untracked-remote discovery was cut short after \
                 seeing {found} issue(s) and cannot be trusted as complete. \
                 Scope the search to a single project or team before pulling \
                 untracked issues."
            );
            ExitCode::from(exit_codes::REFUSED_BULK_OVERWRITE)
        }
        Err(RunError::DiscoveryUnconfigured { detail }) => {
            eprintln!("refused: discovery is unconfigured — {detail}");
            ExitCode::from(exit_codes::UNCONFIGURED)
        }
        Err(RunError::Read(error)) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::RETRYABLE)
        }
        Err(RunError::Internal(error)) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::ERROR)
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
    use super::Scope;
    use super::TargetResolutionFailure;
    use crate::exit_codes;
    use crate::resolve::RunOutcome;

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

    #[test]
    fn a_local_local_collision_is_a_usage_error_naming_both_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let local = target_item(dir.path(), "0001", None);
        let remote_holder = target_item(dir.path(), "0002", Some("0001"));
        let corpus = vec![local, remote_holder];
        let local_path = canonical(&corpus[0]);
        let resolver = |_token: &str| RunOutcome::Resolved(local_path.clone());

        let failures =
            resolve_targets(&corpus, &["0001".to_owned()], &resolver)
                .expect_err("a local/local collision must abort");

        assert_eq!(failures.len(), 1);
        assert!(matches!(
            failures[0],
            TargetResolutionFailure::LocalCollision(_)
        ));
        assert_eq!(failures[0].exit_code(), exit_codes::USAGE);
        let message = failures[0].message();
        assert!(message.contains("0001.md"), "names file A: {message}");
        assert!(message.contains("0002.md"), "names file B: {message}");
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
    fn a_remote_id_token_matches_through_the_external_id_index() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("PP-787"))];
        let resolver =
            |_token: &str| RunOutcome::Invalid("not local".to_owned());

        let matched =
            resolve_targets(&corpus, &["PP-787".to_owned()], &resolver)
                .expect("the remote id resolves through the index");

        assert_eq!(matched.items.len(), 1);
        assert_eq!(matched.items[0].id, "0002");
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
    fn two_tokens_naming_one_item_de_duplicate_to_a_single_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus = vec![target_item(dir.path(), "0002", Some("PP-787"))];
        let item_path = canonical(&corpus[0]);
        let resolver = move |token: &str| {
            if token == "0002" {
                RunOutcome::Resolved(item_path.clone())
            } else {
                RunOutcome::Invalid("not local".to_owned())
            }
        };

        let matched = resolve_targets(
            &corpus,
            &["0002".to_owned(), "PP-787".to_owned()],
            &resolver,
        )
        .expect("both tokens resolve to the one item");

        assert_eq!(
            matched.items.len(),
            1,
            "the same item named twice collapses to a single slice entry"
        );
        assert_eq!(matched.items[0].id, "0002");
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
            reported: Vec::new(),
            read_failure: None,
            baseline_degradation: Degradation::None,
            finalised: true,
            dossiers: Vec::new(),
            discovery: DiscoveryStatus::SkippedPushOnly,
        };

        assert_eq!(
            render_report(&report),
            "#\tdiscovery\tskipped\tpush-only\n#\tsummary\tsynced\t0"
        );
    }

    fn report_with(discovery: DiscoveryStatus) -> RunReport {
        RunReport {
            reported: Vec::new(),
            read_failure: None,
            baseline_degradation: Degradation::None,
            finalised: true,
            dossiers: Vec::new(),
            discovery,
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

    use std::rc::Rc;

    use tracker::RemoteIssue;
    use tracker::RemoteTracker;
    use tracker_test_support::Call;
    use tracker_test_support::RecordingTracker;

    use crate::cli::SyncArgs;
    use crate::tracker_registry::SelectionError;
    use crate::tracker_registry::TrackerRegistry;

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

    /// Shares one `RecordingTracker` between the registry (which hands the
    /// engine a `Box<dyn RemoteTracker>`) and the test (which inspects the call
    /// log afterwards), since the box is moved into `run_sync`.
    struct SharedTracker(Rc<RecordingTracker>);

    impl RemoteTracker for SharedTracker {
        fn create(
            &self,
            title: &str,
            body: &str,
            kind: &str,
        ) -> Result<ExternalId, TrackerError> {
            self.0.create(title, body, kind)
        }

        fn update(
            &self,
            id: &ExternalId,
            title: &str,
            body: &str,
        ) -> Result<(), TrackerError> {
            self.0.update(id, title, body)
        }

        fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
            self.0.show(id)
        }

        fn fetch_all(
            &self,
            ids: &[ExternalId],
        ) -> Result<tracker::FetchOutcome, TrackerError> {
            self.0.fetch_all(ids)
        }

        fn search(
            &self,
            scope: &tracker::SearchScope,
        ) -> Result<tracker::Discovery, TrackerError> {
            self.0.search(scope)
        }

        fn resolve_scope(
            &self,
            scope: &tracker::SearchScope,
        ) -> Result<tracker::SearchScope, tracker::ScopeError> {
            self.0.resolve_scope(scope)
        }

        fn preview_create(
            &self,
            kind: &str,
        ) -> Result<tracker::CreatePreview, TrackerError> {
            self.0.preview_create(kind)
        }

        fn validate_update(
            &self,
            id: &ExternalId,
            title: &str,
            body: &str,
        ) -> tracker::ValidationOutcome {
            self.0.validate_update(id, title, body)
        }
    }

    struct StubRegistry(Rc<RecordingTracker>);

    impl TrackerRegistry for StubRegistry {
        fn resolve(
            &self,
            _name: &str,
        ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
            Ok(Box::new(SharedTracker(Rc::clone(&self.0))))
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
        // A configured integration's state directory already exists; the
        // baseline write into it (unchanged by this feature) needs its parent
        // present, exactly as a full-sync discovery import does.
        std::fs::create_dir_all(
            dir.path().join(".accelerator/state/integrations/jira"),
        )
        .expect("mkdir integration state");
        std::fs::write(
            dir.path().join(".accelerator/config.md"),
            "---\nwork:\n  integration: jira\n---\n",
        )
        .expect("write config");
        dir
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
            max_pulls: 25,
            max_pushes: 25,
            targets,
        }
    }

    fn drive_sync(dir: &Path, tracker: &Rc<RecordingTracker>, args: &SyncArgs) {
        let composed = config_adapters::compose(
            dir,
            config_adapters::LegacyPolicy::Reject,
        )
        .expect("compose the test config");
        let registry = StubRegistry(Rc::clone(tracker));
        let _ = super::run_sync(dir, &composed.service, args, &registry);
    }

    fn issue(body: &str) -> RemoteIssue {
        RemoteIssue {
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
}

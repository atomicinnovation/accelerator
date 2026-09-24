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
use crate::finaliser::FinishedRun;
use crate::finaliser::RunFinaliser;
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
                    Some(
                        work_adapters::sync::apply::FailureClass::Unconfigured,
                    ) => "unconfigured",
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
    let any_unconfigured = report.reported.iter().any(|item| {
        matches!(
            item.outcome,
            ItemOutcome::Failed(ref error)
                if error.class() == Some(work_adapters::sync::apply::FailureClass::Unconfigured)
        )
    });
    let awaiting_human = report.awaiting_human().next().is_some();

    if any_terminal {
        exit_codes::TERMINAL
    } else if awaiting_human {
        exit_codes::UNRESOLVED
    } else if any_unconfigured {
        exit_codes::UNCONFIGURED
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
    let baseline_dir = baseline_path.parent().unwrap_or(&integrations_root);
    // The integration's state directory holds the baseline, the conflict
    // dossiers, and the pending-push markers. On a never-synced integration it
    // does not exist yet, and the atomic-write containment check canonicalises
    // this directory as its trusted root, so the first baseline write fails
    // unless it is present. Create it up-front rather than relying on a later
    // write to author it.
    if let Err(error) = std::fs::create_dir_all(baseline_dir) {
        eprintln!(
            "could not create the integration state directory {}: {error}",
            baseline_dir.display()
        );
        return ExitCode::from(exit_codes::ERROR);
    }
    let file_reader = RealFs;
    let corpus_store = FileCorpusStore::new(baseline_dir);
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
                "{}",
                refusal_message(
                    &integration,
                    pulls,
                    pushes,
                    max_pulls,
                    max_pushes,
                    new_local_files,
                    new_remote_issues,
                    max_pulls_source,
                    max_pushes_source,
                )
            );
            ExitCode::from(exit_codes::REFUSED_BULK_OVERWRITE)
        }
        Err(RunError::DiscoveryIncomplete {
            found,
            completeness,
        }) => {
            eprintln!(
                "{}",
                discovery_incomplete_message(
                    config,
                    &integration,
                    found,
                    completeness,
                )
            );
            ExitCode::from(exit_codes::REFUSED_BULK_OVERWRITE)
        }
        Err(RunError::DiscoveryUnconfigured { detail }) => {
            eprintln!("refused: discovery is unconfigured — {detail}");
            ExitCode::from(exit_codes::UNCONFIGURED)
        }
        Err(RunError::KeyedReadCapped) => {
            eprintln!("{}", keyed_read_capped_message(config, &integration));
            ExitCode::from(exit_codes::KEYED_READ_CAPPED)
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

    fn report_with(discovery: DiscoveryStatus) -> RunReport {
        RunReport {
            reported: Vec::new(),
            read_failure: None,
            baseline_degradation: Degradation::None,
            finalised: true,
            dossiers: Vec::new(),
            discovery,
            keyed_read_budget_limited: false,
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

        fn enumerate_visible_entities(
            &self,
        ) -> Result<Vec<tracker::VisibleEntity>, tracker::TrackerError>
        {
            self.0.enumerate_visible_entities()
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

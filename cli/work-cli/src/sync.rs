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
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::run::render_dossier;
use work_adapters::sync::run::ConflictDossier;
use work_adapters::sync::run::DossierRender;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::RunError;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;
use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

use crate::cli::SyncArgs;
use crate::exit_codes;
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
    lines.sort();
    if synced_count > 0 || lines.is_empty() {
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
    } else if any_retryable || report.read_failure.is_some() {
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
    let request = SyncRequest {
        items: &items,
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
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::unnecessary_wraps)]
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
    use work_adapters::sync::run::ItemOutcome;
    use work_adapters::sync::run::ReportedItem;
    use work_adapters::sync::run::RunReport;

    use super::render_report;

    fn scheme() -> WorkItemIdScheme {
        WorkItemIdScheme::numeric()
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
        };

        assert_eq!(render_report(&report), "#\tsummary\tsynced\t0");
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
}

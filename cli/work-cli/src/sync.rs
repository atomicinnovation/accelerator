//! `accelerator work sync`: drives the remote sync engine end to end.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use ::config::ConfigAccess;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::RealFs;
use tracker::ExternalId;
use work::sync::Dirtiness;
use work::sync::Resolution;
use work::sync::RunClock;
use work::sync::SyncDirection;
use work_adapters::sync::baseline;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::RunError;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;

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

/// Shells the real VCS to answer dirtiness, per the mode `work::file_dirty`
/// expects: one whole-tree `jj diff --name-only` probed once and reused,
/// or a per-path `git status --porcelain -- <path>` — a failed probe
/// (neither `.jj` nor `.git` present, or the shellout itself fails) yields
/// [`Dirtiness::Unknown`].
struct ShelledWorkingCopyStatus {
    repo_root: PathBuf,
    jj_status_text: Option<String>,
}

impl ShelledWorkingCopyStatus {
    fn new(repo_root: PathBuf) -> Self {
        let jj_status_text = if repo_root.join(".jj").is_dir() {
            std::process::Command::new("jj")
                .arg("diff")
                .arg("--name-only")
                .current_dir(&repo_root)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
        } else {
            None
        };
        Self {
            repo_root,
            jj_status_text,
        }
    }
}

impl WorkingCopyStatus for ShelledWorkingCopyStatus {
    fn is_dirty(&self, path: &Path) -> Dirtiness {
        let Ok(relative) = path.strip_prefix(&self.repo_root) else {
            return Dirtiness::Unknown;
        };
        let relative_str = relative.to_string_lossy();

        if let Some(status_text) = &self.jj_status_text {
            return if work::file_dirty::is_dirty(
                work::file_dirty::VcsMode::Jj,
                &relative_str,
                status_text,
            ) {
                Dirtiness::Dirty
            } else {
                Dirtiness::Clean
            };
        }

        if self.repo_root.join(".git").exists() {
            return std::process::Command::new("git")
                .arg("status")
                .arg("--porcelain")
                .arg("--")
                .arg(path)
                .current_dir(&self.repo_root)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map_or(Dirtiness::Unknown, |output| {
                    let text = String::from_utf8_lossy(&output.stdout);
                    if work::file_dirty::is_dirty(
                        work::file_dirty::VcsMode::Git,
                        &relative_str,
                        &text,
                    ) {
                        Dirtiness::Dirty
                    } else {
                        Dirtiness::Clean
                    }
                });
        }

        Dirtiness::Unknown
    }
}

fn integrations_dir(
    config: &dyn ConfigAccess,
    root: &Path,
) -> Result<PathBuf, kernel::Error> {
    let configured =
        crate::config::effective_nonempty(config, "paths.integrations")?;
    let relative = if configured.is_empty() {
        ".accelerator/state/integrations".to_owned()
    } else {
        configured
    };
    let path = Path::new(&relative);
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    })
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
    let status = ShelledWorkingCopyStatus::new(root);
    let clock = SystemClock;

    let ports = SyncPorts {
        tracker: tracker.as_ref(),
        status: &status,
        writer: &corpus_store,
        clock: &clock,
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
    let request = SyncRequest {
        items: &items,
        direction,
        strategy,
        resolutions: &resolutions,
        max_pulls: args.max_pulls,
        max_pushes: args.max_pushes,
        mode,
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
            println!("{}", render_report(&report));
            ExitCode::from(exit_code_for_report(&report))
        }
        Err(RunError::Refused {
            pulls,
            pushes,
            max_pulls,
            max_pushes,
        }) => {
            eprintln!(
                "refused: this run would pull {pulls} item(s) (limit \
                 {max_pulls}) and push {pushes} item(s) (limit \
                 {max_pushes}). Raise the limit with --max-pulls/\
                 --max-pushes, or inspect the plan first with --preview."
            );
            ExitCode::from(exit_codes::REFUSED_BULK_OVERWRITE)
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

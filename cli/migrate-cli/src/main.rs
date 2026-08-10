//! `accelerator-migrate` — applies pending meta-directory schema migrations,
//! dispatched by the `accelerator` launcher.

mod cli;
mod discoverability;
mod render;

use std::io::IsTerminal as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser as _;
use config_adapters::FileConfigStore;
use migrate::ledger;
use migrate::ports::DecisionSource;
use migrate::ports::LedgerStore as _;
use migrate::ports::ManifestStore as _;
use migrate::ports::MigrationContext as _;
use migrate::ports::NoInputDecisionSource;
use migrate::ports::RunLock as _;
use migrate::preflight::Preflight;
use migrate::preflight::PreflightError;
use migrate::preflight::PreflightOutcome;
use migrate_adapters::context::FileMigrationContext;
use migrate_adapters::decisions_file_decision_source::DecisionsFileDecisionSource;
use migrate_adapters::dirty_path_scanner::VcsDirtyPathScanner;
use migrate_adapters::ledger_store::FileLedgerStore;
use migrate_adapters::manifest_store::FileManifestStore;
use migrate_adapters::run_lock::FileRunLock;
use migrate_adapters::session_log_factory::FileSessionLogFactory;
use migrate_adapters::tty_decision_source::TtyDecisionSource;
use vcs::VcsKind;

use crate::cli::Cli;
use crate::render::StdoutReporter;

const RUNNER_APPLIED: &str = ".accelerator/state/migrations-applied";
const RUNNER_SKIPPED: &str = ".accelerator/state/migrations-skipped";
const RUNNER_RUN_PATHS: &str = ".accelerator/state/migrations-run-paths.txt";
const RUNNER_RUN_ID: &str = ".accelerator/state/migrations-run.id";
const DECISION_TIMEOUT: Duration = Duration::from_secs(30);

fn project_root() -> Result<PathBuf, kernel::Error> {
    let cwd = std::env::current_dir().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        ))
    })?;
    Ok(FileConfigStore::discover_root(&cwd))
}

fn vcs_kind(root: &Path) -> VcsKind {
    if root.join(".jj").is_dir() {
        VcsKind::Jj
    } else if root.join(".git").exists() {
        VcsKind::Git
    } else {
        VcsKind::None
    }
}

fn force_requested() -> bool {
    std::env::var("ACCELERATOR_MIGRATE_FORCE")
        .is_ok_and(|value| !value.is_empty())
}

fn session_log_decision_count(root: &Path) -> impl Fn(&str) -> usize + '_ {
    move |path| {
        std::fs::read_to_string(root.join(path))
            .map(|content| content.lines().count())
            .unwrap_or(0)
    }
}

fn resolve_decisions_file(cli: &Cli) -> Option<PathBuf> {
    cli.decisions_file.clone().or_else(|| {
        std::env::var("ACCELERATOR_MIGRATE_DECISIONS_FILE")
            .ok()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    })
}

/// Three decisions-file existence checks, run in a fixed order,
/// unconditionally whenever the var is set — including under `--list`,
/// which never ends up reading the file.
///
/// # Errors
/// [`kernel::Error::Failed`] naming which check failed.
fn validate_decisions_file_exists(path: &Path) -> Result<(), kernel::Error> {
    if path.is_dir() {
        return Err(kernel::Error::Failed(format!(
            "Error: ACCELERATOR_MIGRATE_DECISIONS_FILE is a directory: {}",
            path.display()
        )));
    }
    if !path.exists() {
        return Err(kernel::Error::Failed(format!(
            "Error: ACCELERATOR_MIGRATE_DECISIONS_FILE does not exist: {}",
            path.display()
        )));
    }
    if std::fs::File::open(path).is_err() {
        return Err(kernel::Error::Failed(format!(
            "Error: ACCELERATOR_MIGRATE_DECISIONS_FILE is not readable: {}",
            path.display()
        )));
    }
    Ok(())
}

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    if cli.discoverability_hook {
        let root = project_root()?;
        let entries = migrate::registry::registry();
        if let Some(output) = discoverability::run(&root, &entries) {
            println!("{output}");
        }
        return Ok(());
    }

    let root = project_root()?;
    let ledger_store = FileLedgerStore::new(&root);
    let run_lock = FileRunLock::new(&root);

    if let Some(id) = &cli.skip {
        let _guard = run_lock.acquire()?;
        ledger::skip(&ledger_store, id)?;
        println!("Skipped migration: {id}");
        return Ok(());
    }
    if let Some(id) = &cli.unskip {
        let _guard = run_lock.acquire()?;
        ledger::unskip(&ledger_store, id)?;
        println!("Unskipped migration: {id}");
        return Ok(());
    }
    if let Some(id) = &cli.unapply {
        let _guard = run_lock.acquire()?;
        ledger::unapply(&ledger_store, id)?;
        println!("Unapplied migration: {id}");
        return Ok(());
    }

    let decisions_file = resolve_decisions_file(cli);
    if let Some(path) = &decisions_file {
        validate_decisions_file_exists(path)?;
    }

    if cli.list {
        return run_list(&root, &ledger_store);
    }

    run_default(&root, &run_lock, &ledger_store, decisions_file.as_deref())
}

fn run_list(
    root: &Path,
    ledger_store: &FileLedgerStore,
) -> Result<(), kernel::Error> {
    let ctx = FileMigrationContext::new(root);
    let session_logs = FileSessionLogFactory::new(root);
    let entries = migrate::registry::registry();
    let applied = ledger_store.applied()?;
    let skipped = ledger_store.skipped()?;
    let reporter = StdoutReporter {
        root: root.to_path_buf(),
    };
    let groups = migrate::list::list_pending(
        &entries,
        &applied,
        &skipped,
        &ctx,
        &session_logs,
        &reporter,
    )?;
    render::render_list(root, &groups);
    Ok(())
}

/// The session log path a real TTY prompt banner names — the compiled
/// registry never holds more than one `Interactive` entry, so there is
/// only ever one path to find. A placeholder when none exists is harmless:
/// the banner only ever prints once a `Prompt`-outcome transformation is
/// reached, which only an `Interactive` entry can produce.
fn interactive_session_log_path(
    root: &Path,
    entries: &[migrate::registry::MigrationEntry],
) -> PathBuf {
    entries
        .iter()
        .find_map(|entry| match entry {
            migrate::registry::MigrationEntry::Interactive(migration) => {
                Some(migrate_adapters::session_log::session_log_path(
                    root,
                    migration.id(),
                ))
            }
            migrate::registry::MigrationEntry::Mechanical(_) => None,
        })
        .unwrap_or_else(|| {
            root.join(".accelerator/state/migrations-session.jsonl")
        })
}

fn run_default(
    root: &Path,
    run_lock: &FileRunLock,
    ledger_store: &FileLedgerStore,
    decisions_file: Option<&Path>,
) -> Result<(), kernel::Error> {
    let decisions_file_content = decisions_file
        .map(std::fs::read_to_string)
        .transpose()
        .map_err(|error| {
            kernel::Error::Failed(format!(
                "could not read the decisions file: {error}"
            ))
        })?;

    let ctx = FileMigrationContext::new(root);
    let manifest_store = FileManifestStore::new(root);
    let scanner = VcsDirtyPathScanner::new(root, vcs_kind(root));
    let session_log_decisions = session_log_decision_count(root);
    let preflight = Preflight {
        lock: run_lock,
        scanner: &scanner,
        manifest: &manifest_store,
        runner: migrate::manifest::RunnerPaths {
            applied: RUNNER_APPLIED,
            skipped: RUNNER_SKIPPED,
            run_paths: RUNNER_RUN_PATHS,
            run_id: RUNNER_RUN_ID,
        },
        revision: ctx.revision(),
        force: force_requested(),
        session_log_decision_count: &session_log_decisions,
    };

    let _guard = match preflight.run() {
        Ok((guard, PreflightOutcome::Clean)) => guard,
        Ok((guard, PreflightOutcome::Resumed { affordance })) => {
            render::resume_affordance(root, &affordance);
            guard
        }
        Err(PreflightError::ForeignDirt) => {
            eprintln!("{}", render::DIRTY_TREE_REFUSAL);
            return Err(kernel::Error::Failed(String::new()));
        }
        Err(PreflightError::Failed(error)) => return Err(error.into()),
    };

    let reporter = StdoutReporter {
        root: root.to_path_buf(),
    };
    let entries = migrate::registry::registry();
    let session_logs = FileSessionLogFactory::new(root);
    let decisions_file_source = decisions_file_content
        .as_deref()
        .map(DecisionsFileDecisionSource::new);
    let tty_decisions =
        TtyDecisionSource::new(interactive_session_log_path(root, &entries));
    let no_input = NoInputDecisionSource;
    let decisions: &dyn DecisionSource =
        if let Some(source) = &decisions_file_source {
            source
        } else if std::io::stdin().is_terminal() {
            &tty_decisions
        } else {
            &no_input
        };
    let result = migrate::lifecycle::run_pending(
        &entries,
        &ctx,
        ledger_store,
        &reporter,
        decisions,
        &session_logs,
        DECISION_TIMEOUT,
        decisions_file_content.as_deref(),
    );
    if result.is_ok() {
        manifest_store.clear()?;
    }
    result?;
    Ok(())
}

fn report(error: &kernel::Error) -> ExitCode {
    let message = error.to_string();
    if !message.is_empty() {
        eprintln!("{message}");
    }
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error),
    }
}

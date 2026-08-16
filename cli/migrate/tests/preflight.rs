//! `Preflight::run` against in-memory test doubles: clean, resumed-owned,
//! foreign-refused, `FORCE`, and staleness.

use std::cell::RefCell;

use migrate::manifest::RunnerPaths;
use migrate::ports::DirtyPathScanner;
use migrate::ports::ManifestStore;
use migrate::ports::MigrationError;
use migrate::ports::RunLock;
use migrate::ports::RunLockGuard;
use migrate::preflight::Preflight;
use migrate::preflight::PreflightError;
use migrate::preflight::PreflightOutcome;

type TestError = Box<dyn std::error::Error>;

struct AlwaysLock;

impl RunLock for AlwaysLock {
    fn acquire(&self) -> Result<RunLockGuard, MigrationError> {
        Ok(RunLockGuard::new(()))
    }
}

struct NeverLock;

impl RunLock for NeverLock {
    fn acquire(&self) -> Result<RunLockGuard, MigrationError> {
        Err(MigrationError::new(
            "Another accelerator migrate run is already in progress (pid 1).",
        ))
    }
}

struct StubScanner(Vec<String>);

impl DirtyPathScanner for StubScanner {
    fn dirty_paths(
        &self,
        roots: &[&str],
    ) -> Result<Vec<String>, MigrationError> {
        Ok(self
            .0
            .iter()
            .filter(|path| roots.iter().any(|scope| path.starts_with(scope)))
            .cloned()
            .collect())
    }
}

#[derive(Default)]
struct InMemoryManifestStore {
    manifest: RefCell<Option<Vec<String>>>,
    run_id: RefCell<Option<String>>,
}

impl InMemoryManifestStore {
    fn seeded(manifest: Vec<String>, run_id: Option<&str>) -> Self {
        Self {
            manifest: RefCell::new(Some(manifest)),
            run_id: RefCell::new(run_id.map(str::to_owned)),
        }
    }
}

impl ManifestStore for InMemoryManifestStore {
    fn manifest(&self) -> Result<Option<Vec<String>>, MigrationError> {
        Ok(self.manifest.borrow().clone())
    }

    fn write_manifest(&self, paths: &[String]) -> Result<(), MigrationError> {
        *self.manifest.borrow_mut() = Some(paths.to_vec());
        Ok(())
    }

    fn append_manifest_path(&self, path: &str) -> Result<(), MigrationError> {
        let mut manifest = self.manifest.borrow_mut();
        let entries = manifest.get_or_insert_with(Vec::new);
        if !entries.iter().any(|existing| existing == path) {
            entries.push(path.to_owned());
        }
        Ok(())
    }

    fn run_id(&self) -> Result<Option<String>, MigrationError> {
        Ok(self.run_id.borrow().clone())
    }

    fn write_run_id(
        &self,
        revision: Option<&str>,
    ) -> Result<(), MigrationError> {
        *self.run_id.borrow_mut() = revision.map(str::to_owned);
        Ok(())
    }

    fn clear(&self) -> Result<(), MigrationError> {
        *self.manifest.borrow_mut() = None;
        *self.run_id.borrow_mut() = None;
        Ok(())
    }
}

const fn runner() -> RunnerPaths<'static> {
    RunnerPaths {
        applied: ".accelerator/state/migrations-applied",
        skipped: ".accelerator/state/migrations-skipped",
        run_paths: ".accelerator/state/migrations-run-paths.txt",
        run_id: ".accelerator/state/migrations-run.id",
        lock_dir: ".accelerator/state/migrate-run.lockdir",
    }
}

/// The run lock is acquired before the scan and released after it, so its
/// sentinel is present for every scan a run ever performs. Its filename
/// carries a fresh nonce, so ownership here is by prefix, not by equality.
#[test]
fn the_held_run_locks_sentinel_is_never_dirt() -> Result<(), TestError> {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec![
        ".accelerator/state/migrate-run.lockdir/owner.e218f68e83638d9b"
            .to_owned(),
    ]);
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    Ok(())
}

#[test]
fn a_clean_tree_mints_a_fresh_manifest_and_run_id() -> Result<(), TestError> {
    let lock = AlwaysLock;
    let scanner = StubScanner(Vec::new());
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.manifest()?, Some(Vec::new()));
    assert_eq!(manifest.run_id()?, Some("rev-1".to_owned()));
    Ok(())
}

/// The runner's own ledger is owned by pattern, not by manifest, so a run
/// that has nothing else dirty is clean even when no prior manifest exists.
/// An uncommitted ledger would otherwise refuse every run but the first.
#[test]
fn a_tree_dirty_only_in_the_runners_own_bookkeeping_is_clean(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec![
        ".accelerator/state/migrations-applied".to_owned(),
        ".accelerator/state/migrations-skipped".to_owned(),
        ".accelerator/state/migrations-run-paths.txt".to_owned(),
        ".accelerator/state/migrations-run.id".to_owned(),
    ]);
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.manifest()?, Some(Vec::new()));
    assert_eq!(manifest.run_id()?, Some("rev-1".to_owned()));
    Ok(())
}

/// Filtering the ledger out must not soften the gate around it: a foreign
/// document alongside the ledger still refuses.
#[test]
fn foreign_dirt_beside_the_runners_bookkeeping_still_refuses() {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec![
        ".accelerator/state/migrations-applied".to_owned(),
        "meta/work/0001-foo.md".to_owned(),
    ]);
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    assert!(matches!(preflight.run(), Err(PreflightError::ForeignDirt)));
}

/// A resumed run's affordance lists what the user must review. The ledger is
/// the runner's own append-only bookkeeping, not a document under review.
#[test]
fn the_runners_bookkeeping_is_absent_from_the_resume_affordance(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec![
        ".accelerator/state/migrations-applied".to_owned(),
        "meta/work/0001-foo.md".to_owned(),
    ]);
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/0001-foo.md".to_owned()],
        Some("rev-1"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    let PreflightOutcome::Resumed { affordance } = outcome else {
        return Err("expected a resumed outcome".into());
    };
    assert_eq!(affordance.len(), 1);
    assert_eq!(affordance[0].path, "meta/work/0001-foo.md");
    Ok(())
}

#[test]
fn every_dirty_path_manifested_at_a_matching_revision_resumes(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec!["meta/work/0001-foo.md".to_owned()]);
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/0001-foo.md".to_owned()],
        Some("rev-1"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    let PreflightOutcome::Resumed { affordance } = outcome else {
        return Err("expected a resumed outcome".into());
    };
    assert_eq!(affordance.len(), 1);
    assert_eq!(affordance[0].path, "meta/work/0001-foo.md");
    assert_eq!(affordance[0].session_log_decision_count, None);
    Ok(())
}

#[test]
fn a_session_log_in_the_affordance_reports_its_decision_count(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let session_log = ".accelerator/state/migrations-0099-session.jsonl";
    let scanner = StubScanner(vec![session_log.to_owned()]);
    let manifest = InMemoryManifestStore::seeded(Vec::new(), Some("rev-1"));
    let counter = |path: &str| if path == session_log { 3 } else { 0 };
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &counter,
    };

    let (_guard, outcome) = preflight.run()?;

    let PreflightOutcome::Resumed { affordance } = outcome else {
        return Err("expected a resumed outcome".into());
    };
    assert_eq!(affordance[0].session_log_decision_count, Some(3));
    Ok(())
}

#[test]
fn a_foreign_dirty_path_refuses() {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec!["meta/unrelated.md".to_owned()]);
    let manifest = InMemoryManifestStore::seeded(Vec::new(), Some("rev-1"));
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let outcome = preflight.run();

    assert!(matches!(outcome, Err(PreflightError::ForeignDirt)));
}

/// The fail-closed usability gate, kept for every class ownership cannot
/// settle by pattern alone. A session artefact needs a usable manifest at a
/// matching revision; the runner's own bookkeeping does not, and is filtered
/// out before this gate is reached.
#[test]
fn an_absent_manifest_or_run_id_refuses_even_a_session_artefact() {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec![
        ".accelerator/state/migrations-0099-session.jsonl".to_owned(),
    ]);
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let outcome = preflight.run();

    assert!(matches!(outcome, Err(PreflightError::ForeignDirt)));
}

#[test]
fn a_recorded_revision_of_none_never_matches_even_a_current_none() {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec!["meta/work/0001-foo.md".to_owned()]);
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/0001-foo.md".to_owned()],
        None,
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: None,
        force: false,
        session_log_decision_count: &no_op,
    };

    let outcome = preflight.run();

    assert!(matches!(outcome, Err(PreflightError::ForeignDirt)));
}

#[test]
fn a_stale_leftover_manifest_on_a_clean_tree_is_truncated_and_reminted(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let scanner = StubScanner(Vec::new());
    let manifest = InMemoryManifestStore::seeded(
        vec!["stale/path.md".to_owned()],
        Some("old-rev"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-2".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.manifest()?, Some(Vec::new()));
    assert_eq!(manifest.run_id()?, Some("rev-2".to_owned()));
    Ok(())
}

#[test]
fn two_distinct_non_none_revisions_are_a_stale_base_and_refuse() {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec!["meta/work/owned.md".to_owned()]);
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/owned.md".to_owned()],
        Some("rev-1"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-2".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let outcome = preflight.run();

    assert!(matches!(outcome, Err(PreflightError::ForeignDirt)));
}

#[test]
fn force_bypasses_the_dirty_check_and_mints_a_fresh_manifest(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let scanner = StubScanner(vec!["meta/unrelated.md".to_owned()]);
    let manifest = InMemoryManifestStore::seeded(
        vec!["stale".to_owned()],
        Some("stale-rev"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-2".to_owned()),
        force: true,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.manifest()?, Some(Vec::new()));
    assert_eq!(manifest.run_id()?, Some("rev-2".to_owned()));
    Ok(())
}

#[test]
fn lock_contention_surfaces_as_a_failed_preflight() {
    let lock = NeverLock;
    let scanner = StubScanner(Vec::new());
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        scanner: &scanner,
        manifest: &manifest,
        runner: runner(),
        revision: Some("rev-1".to_owned()),
        force: false,
        session_log_decision_count: &no_op,
    };

    let outcome = preflight.run();

    assert!(matches!(outcome, Err(PreflightError::Failed(_))));
}

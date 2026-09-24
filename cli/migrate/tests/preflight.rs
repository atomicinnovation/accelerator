//! `Preflight::run` against in-memory test doubles: clean, resumed-owned,
//! unowned-refused, `FORCE`, and staleness.

use std::cell::RefCell;

use migrate::manifest::RunnerPaths;
use migrate::ports::ManifestStore;
use migrate::ports::MigrationError;
use migrate::ports::RunLock;
use migrate::ports::RunLockGuard;
use migrate::ports::WorkingCopy;
use migrate::ports::WorkingCopyObservation;
use migrate::preflight::Preflight;
use migrate::preflight::PreflightError;
use migrate::preflight::PreflightOutcome;
use migrate::preflight::UnownedChanges;
use migrate::run_base::RunBase;

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

struct StubWorkingCopy {
    run_base: Option<RunBase>,
    dirty_paths: Vec<String>,
}

impl StubWorkingCopy {
    fn based_on(run_base: Option<&str>, dirty_paths: Vec<String>) -> Self {
        Self {
            run_base: run_base.and_then(RunBase::recorded),
            dirty_paths,
        }
    }
}

impl WorkingCopy for StubWorkingCopy {
    fn observe(
        &self,
        roots: &[&str],
    ) -> Result<WorkingCopyObservation, MigrationError> {
        Ok(WorkingCopyObservation {
            run_base: self.run_base.clone(),
            dirty_paths: self
                .dirty_paths
                .iter()
                .filter(|path| {
                    roots.iter().any(|scope| path.starts_with(scope))
                })
                .cloned()
                .collect(),
        })
    }
}

#[derive(Default)]
struct InMemoryManifestStore {
    manifest: RefCell<Option<Vec<String>>>,
    recorded_run_base: RefCell<Option<RunBase>>,
}

impl InMemoryManifestStore {
    fn seeded(manifest: Vec<String>, run_base: Option<&str>) -> Self {
        Self {
            manifest: RefCell::new(Some(manifest)),
            recorded_run_base: RefCell::new(
                run_base.and_then(RunBase::recorded),
            ),
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

    fn recorded_run_base(&self) -> Result<Option<RunBase>, MigrationError> {
        Ok(self.recorded_run_base.borrow().clone())
    }

    fn record_run_base(
        &self,
        run_base: Option<&RunBase>,
    ) -> Result<(), MigrationError> {
        *self.recorded_run_base.borrow_mut() = run_base.cloned();
        Ok(())
    }

    fn clear(&self) -> Result<(), MigrationError> {
        *self.manifest.borrow_mut() = None;
        *self.recorded_run_base.borrow_mut() = None;
        Ok(())
    }
}

const fn runner() -> RunnerPaths<'static> {
    RunnerPaths {
        applied: ".accelerator/state/migrations-applied",
        skipped: ".accelerator/state/migrations-skipped",
        run_paths: ".accelerator/state/migrations-run-paths.txt",
        recorded_run_base: ".accelerator/state/migrations-run.id",
        lock_dir: ".accelerator/state/migrate-run.lockdir",
    }
}

fn refusal_over(
    dirty_paths: &[&str],
    manifest: &InMemoryManifestStore,
    revision: Option<&str>,
) -> Result<UnownedChanges, TestError> {
    let lock = AlwaysLock;
    let working_copy = StubWorkingCopy::based_on(
        revision,
        dirty_paths.iter().map(|path| (*path).to_owned()).collect(),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest,
        runner: runner(),
        force: false,
        session_log_decision_count: &no_op,
    };
    match preflight.run() {
        Err(PreflightError::UnownedChanges(unowned)) => Ok(unowned),
        Err(PreflightError::Failed(error)) => Err(error.into()),
        Ok(_) => Err("expected a refusal".into()),
    }
}

fn unowned(paths: &[&str], stale_run: bool) -> UnownedChanges {
    UnownedChanges {
        paths: paths.iter().map(|path| (*path).to_owned()).collect(),
        stale_run,
    }
}

/// The run lock is acquired before the scan and released after it, so its
/// sentinel is present for every scan a run ever performs. Its filename
/// carries a fresh nonce, so ownership here is by prefix, not by equality.
#[test]
fn the_held_run_locks_sentinel_is_never_dirt() -> Result<(), TestError> {
    let lock = AlwaysLock;
    let working_copy = StubWorkingCopy::based_on(
        Some("rev-1"),
        vec![
            ".accelerator/state/migrate-run.lockdir/owner.e218f68e83638d9b"
                .to_owned(),
        ],
    );
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
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
    let working_copy = StubWorkingCopy::based_on(Some("rev-1"), Vec::new());
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.manifest()?, Some(Vec::new()));
    assert_eq!(manifest.recorded_run_base()?, RunBase::recorded("rev-1"));
    Ok(())
}

/// The runner's own ledger is owned by pattern, not by manifest, so a run
/// that has nothing else dirty is clean even when no prior manifest exists.
/// An uncommitted ledger would otherwise refuse every run but the first.
#[test]
fn a_tree_dirty_only_in_the_runners_own_bookkeeping_is_clean(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let working_copy = StubWorkingCopy::based_on(
        Some("rev-1"),
        vec![
            ".accelerator/state/migrations-applied".to_owned(),
            ".accelerator/state/migrations-skipped".to_owned(),
            ".accelerator/state/migrations-run-paths.txt".to_owned(),
            ".accelerator/state/migrations-run.id".to_owned(),
        ],
    );
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.manifest()?, Some(Vec::new()));
    assert_eq!(manifest.recorded_run_base()?, RunBase::recorded("rev-1"));
    Ok(())
}

/// Filtering the ledger out must not soften the gate around it: an unowned
/// document alongside the ledger still refuses.
#[test]
fn unowned_changes_beside_the_runners_bookkeeping_still_refuse(
) -> Result<(), TestError> {
    let manifest = InMemoryManifestStore::default();

    let refusal = refusal_over(
        &[
            ".accelerator/state/migrations-applied",
            "meta/work/0001-foo.md",
        ],
        &manifest,
        Some("rev-1"),
    )?;

    assert_eq!(refusal, unowned(&["meta/work/0001-foo.md"], false));
    Ok(())
}

/// A resumed run's affordance lists what the user must review. The ledger is
/// the runner's own append-only bookkeeping, not a document under review.
#[test]
fn the_runners_bookkeeping_is_absent_from_the_resume_affordance(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let working_copy = StubWorkingCopy::based_on(
        Some("rev-1"),
        vec![
            ".accelerator/state/migrations-applied".to_owned(),
            "meta/work/0001-foo.md".to_owned(),
        ],
    );
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/0001-foo.md".to_owned()],
        Some("rev-1"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
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
    let working_copy = StubWorkingCopy::based_on(
        Some("rev-1"),
        vec!["meta/work/0001-foo.md".to_owned()],
    );
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/0001-foo.md".to_owned()],
        Some("rev-1"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
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
    let working_copy =
        StubWorkingCopy::based_on(Some("rev-1"), vec![session_log.to_owned()]);
    let manifest = InMemoryManifestStore::seeded(Vec::new(), Some("rev-1"));
    let counter = |path: &str| if path == session_log { 3 } else { 0 };
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
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
fn an_unowned_change_refuses() -> Result<(), TestError> {
    let manifest = InMemoryManifestStore::seeded(Vec::new(), Some("rev-1"));

    let refusal =
        refusal_over(&["meta/unrelated.md"], &manifest, Some("rev-1"))?;

    assert_eq!(refusal, unowned(&["meta/unrelated.md"], false));
    Ok(())
}

#[test]
fn a_current_run_lists_only_its_unowned_changes_byte_sorted(
) -> Result<(), TestError> {
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/owned.md".to_owned()],
        Some("rev-1"),
    );

    let refusal = refusal_over(
        &["meta/a.md", "meta/work/owned.md", ".accelerator/b"],
        &manifest,
        Some("rev-1"),
    )?;

    assert_eq!(refusal, unowned(&[".accelerator/b", "meta/a.md"], false));
    Ok(())
}

#[test]
fn without_a_manifest_every_change_is_listed() -> Result<(), TestError> {
    let manifest = InMemoryManifestStore::default();

    let refusal = refusal_over(
        &["meta/a.md", ".accelerator/b"],
        &manifest,
        Some("rev-1"),
    )?;

    assert_eq!(refusal, unowned(&[".accelerator/b", "meta/a.md"], false));
    Ok(())
}

#[test]
fn a_stale_run_lists_its_own_output_and_says_it_is_stale(
) -> Result<(), TestError> {
    let session_log = ".accelerator/state/migrations-0099-session.jsonl";
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/owned.md".to_owned()],
        Some("rev-1"),
    );

    let refusal = refusal_over(
        &["meta/work/owned.md", session_log, "meta/a.md"],
        &manifest,
        Some("rev-2"),
    )?;

    assert_eq!(
        refusal,
        unowned(&[session_log, "meta/a.md", "meta/work/owned.md"], true)
    );
    Ok(())
}

#[test]
fn a_recorded_run_base_without_a_manifest_is_not_a_stale_run(
) -> Result<(), TestError> {
    let manifest = InMemoryManifestStore {
        manifest: RefCell::new(None),
        recorded_run_base: RefCell::new(RunBase::recorded("rev-1")),
    };

    let refusal = refusal_over(&["meta/a.md"], &manifest, Some("rev-2"))?;

    assert_eq!(refusal, unowned(&["meta/a.md"], false));
    Ok(())
}

#[test]
fn a_manifest_without_a_recorded_run_base_lists_every_change(
) -> Result<(), TestError> {
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/owned.md".to_owned()],
        None,
    );

    let refusal = refusal_over(
        &["meta/work/owned.md", "meta/a.md"],
        &manifest,
        Some("rev-1"),
    )?;

    assert_eq!(
        refusal,
        unowned(&["meta/a.md", "meta/work/owned.md"], false)
    );
    Ok(())
}

#[test]
fn runner_managed_paths_are_never_listed() -> Result<(), TestError> {
    let manifest = InMemoryManifestStore::seeded(Vec::new(), Some("rev-1"));

    let refusal = refusal_over(
        &[
            ".accelerator/state/migrations-run.id",
            ".accelerator/state/migrate-run.lockdir/owner.1",
            "meta/a.md",
        ],
        &manifest,
        Some("rev-2"),
    )?;

    assert_eq!(refusal.paths, vec!["meta/a.md".to_owned()]);
    Ok(())
}

/// The fail-closed usability gate, kept for every class ownership cannot
/// settle by pattern alone. A session artefact needs a usable manifest at a
/// matching run base; the runner's own bookkeeping does not, and is filtered
/// out before this gate is reached.
#[test]
fn an_absent_manifest_or_run_base_refuses_even_a_session_artefact(
) -> Result<(), TestError> {
    let session_log = ".accelerator/state/migrations-0099-session.jsonl";
    let manifest = InMemoryManifestStore::default();

    let refusal = refusal_over(&[session_log], &manifest, Some("rev-1"))?;

    assert_eq!(refusal, unowned(&[session_log], false));
    Ok(())
}

#[test]
fn a_recorded_revision_of_none_never_matches_even_a_current_none(
) -> Result<(), TestError> {
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/0001-foo.md".to_owned()],
        None,
    );

    let refusal = refusal_over(&["meta/work/0001-foo.md"], &manifest, None)?;

    assert_eq!(refusal, unowned(&["meta/work/0001-foo.md"], false));
    Ok(())
}

#[test]
fn a_stale_leftover_manifest_on_a_clean_tree_is_truncated_and_reminted(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let working_copy = StubWorkingCopy::based_on(Some("rev-2"), Vec::new());
    let manifest = InMemoryManifestStore::seeded(
        vec!["stale/path.md".to_owned()],
        Some("old-rev"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
        force: false,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.manifest()?, Some(Vec::new()));
    assert_eq!(manifest.recorded_run_base()?, RunBase::recorded("rev-2"));
    Ok(())
}

#[test]
fn two_distinct_non_none_revisions_are_a_stale_base_and_refuse(
) -> Result<(), TestError> {
    let manifest = InMemoryManifestStore::seeded(
        vec!["meta/work/owned.md".to_owned()],
        Some("rev-1"),
    );

    let refusal =
        refusal_over(&["meta/work/owned.md"], &manifest, Some("rev-2"))?;

    assert_eq!(refusal, unowned(&["meta/work/owned.md"], true));
    Ok(())
}

#[test]
fn force_bypasses_the_dirty_check_and_mints_a_fresh_manifest(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let working_copy = StubWorkingCopy::based_on(
        Some("rev-2"),
        vec!["meta/unrelated.md".to_owned()],
    );
    let manifest = InMemoryManifestStore::seeded(
        vec!["stale".to_owned()],
        Some("stale-rev"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
        force: true,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.manifest()?, Some(Vec::new()));
    assert_eq!(manifest.recorded_run_base()?, RunBase::recorded("rev-2"));
    Ok(())
}

#[test]
fn lock_contention_surfaces_as_a_failed_preflight() {
    let lock = NeverLock;
    let working_copy = StubWorkingCopy::based_on(Some("rev-1"), Vec::new());
    let manifest = InMemoryManifestStore::default();
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
        force: false,
        session_log_decision_count: &no_op,
    };

    let outcome = preflight.run();

    assert!(matches!(outcome, Err(PreflightError::Failed(_))));
}

#[test]
fn force_over_an_unreadable_working_copy_records_no_run_base(
) -> Result<(), TestError> {
    let lock = AlwaysLock;
    let working_copy = StubWorkingCopy::based_on(None, Vec::new());
    let manifest = InMemoryManifestStore::seeded(
        vec!["stale".to_owned()],
        Some("stale-rev"),
    );
    let no_op = |_: &str| 0;
    let preflight = Preflight {
        lock: &lock,
        working_copy: &working_copy,
        manifest: &manifest,
        runner: runner(),
        force: true,
        session_log_decision_count: &no_op,
    };

    let (_guard, outcome) = preflight.run()?;

    assert!(matches!(outcome, PreflightOutcome::Clean));
    assert_eq!(manifest.recorded_run_base()?, None);
    Ok(())
}

struct RecordingLock<'a>(&'a RefCell<Vec<&'static str>>);

impl RunLock for RecordingLock<'_> {
    fn acquire(&self) -> Result<RunLockGuard, MigrationError> {
        self.0.borrow_mut().push("lock");
        Ok(RunLockGuard::new(()))
    }
}

struct RecordingWorkingCopy<'a>(&'a RefCell<Vec<&'static str>>);

impl WorkingCopy for RecordingWorkingCopy<'_> {
    fn observe(
        &self,
        _roots: &[&str],
    ) -> Result<WorkingCopyObservation, MigrationError> {
        self.0.borrow_mut().push("observe");
        Ok(WorkingCopyObservation {
            run_base: RunBase::recorded("rev-1"),
            dirty_paths: Vec::new(),
        })
    }
}

#[test]
fn the_working_copy_is_observed_once_the_run_lock_is_held(
) -> Result<(), TestError> {
    for force in [false, true] {
        let calls = RefCell::new(Vec::new());
        let lock = RecordingLock(&calls);
        let working_copy = RecordingWorkingCopy(&calls);
        let manifest = InMemoryManifestStore::default();
        let no_op = |_: &str| 0;
        let preflight = Preflight {
            lock: &lock,
            working_copy: &working_copy,
            manifest: &manifest,
            runner: runner(),
            force,
            session_log_decision_count: &no_op,
        };

        preflight.run()?;

        assert_eq!(*calls.borrow(), vec!["lock", "observe"], "force: {force}");
    }
    Ok(())
}

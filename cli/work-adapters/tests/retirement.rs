//! ID retirement applied to a real corpus on disk: every step lands, or the
//! corpus, the baseline and the recovery copies are as they were.

use std::cell::Cell;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use corpus::lock::ExclusiveLock as _;
use corpus::lock::LockName;
use corpus::store::AtomicWrite;
use corpus::store::ExclusiveCreate;
use corpus::store::RemoveFile;
use corpus::StoreError;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::FileRecoveryCopies;
use corpus_adapters::LockdirLock;
use corpus_adapters::RealFs;
use store::lock::LockOptions;
use tracker::RemoteTimestamp;
use work::dirtiness::Dirtiness;
use work::identity::resolve_identity;
use work::identity::IdentityResolution;
use work::retirement::plan_retirement;
use work::retirement::Retirement;
use work::retirement::RetirementCauseKind;
use work::retirement::RetirementFailure;
use work::retirement::RetirementPlan;
use work::retirement::RetirementRefusal;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles as _;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::retirement::acquire_retirement_lock;
use work_adapters::retirement::apply_retirement;
use work_adapters::retirement::corpus_files;
use work_adapters::retirement::finish_retirement;
use work_adapters::retirement::CorpusLayout;
use work_adapters::retirement::FinishFailure;
use work_adapters::retirement::RetirementFiles;
use work_adapters::retirement::RetirementPorts;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::fetch::WorkingCopyStatus;

type TestError = Box<dyn std::error::Error>;

const DRAFT: &str = "meta/work/drafts/draft-k7mq3x-add-search.md";
const TARGET: &str = "meta/work/PP-900-add-search.md";
const CHILD: &str = "meta/work/0001-child.md";
const PLAN: &str = "meta/plans/plan.md";
const NOTE: &str = "meta/notes/unrelated.md";
const OUTSIDE: &str = "README.md";
const BASELINE: &str = ".accelerator/state/integrations/linear/last-sync.json";
const RECOVERY: &str =
    ".accelerator/state/retirement-recovery/draft-k7mq3x--PP-900";

const DRAFT_CONTENT: &str =
    "---\nid: \"draft-k7mq3x\"\ntitle: \"Add search\"\n\
                             ---\n\n# draft-k7mq3x: Add search\n";
const CHILD_CONTENT: &str = "---\nid: \"0001\"\nparent: \
                             \"work-item:draft-k7mq3x\"\n---\n\n# 0001: Child\n";
const PLAN_CONTENT: &str =
    "See draft-k7mq3x in [it](../work/drafts/draft-k7mq3x-add-search.md).\n";
const NOTE_CONTENT: &str = "Nothing to see.\n";
const OUTSIDE_CONTENT: &str = "Mentions draft-k7mq3x.\n";

const FAST: LockOptions = LockOptions {
    ceiling_ms: 1,
    base_ms: 1,
    cap_ms: 1,
};

const fn promotion() -> Retirement<'static> {
    Retirement {
        old_id: "draft-k7mq3x",
        new_id: "PP-900",
        new_external_id: Some("PP-900"),
    }
}

struct EveryPath(Dirtiness);

impl WorkingCopyStatus for EveryPath {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        self.0
    }
}

/// Delegates to a real store, failing the `fail_at`th mutating operation
/// counted from zero across rewrites, creation, removal and the baseline
/// write, and every later one when `keep_failing`; `before_failing` runs
/// just before the first failure is returned.
struct Faulty {
    inner: FileCorpusStore,
    count: Cell<usize>,
    fail_at: Option<usize>,
    before_failing: RefCell<Option<Box<dyn Fn()>>>,
    keep_failing: bool,
}

impl Faulty {
    fn new(root: &Path, fail_at: Option<usize>) -> Self {
        Self {
            inner: FileCorpusStore::new(root),
            count: Cell::new(0),
            fail_at,
            before_failing: RefCell::new(None),
            keep_failing: false,
        }
    }

    fn check(&self, path: &Path) -> Result<(), StoreError> {
        let index = self.count.get();
        self.count.set(index + 1);
        let failing = self
            .fail_at
            .is_some_and(|at| index == at || (self.keep_failing && index > at));
        if failing {
            if let Some(hook) = self.before_failing.borrow_mut().take() {
                hook();
            }
            return Err(StoreError::Io {
                path: path.display().to_string(),
                detail: format!("injected failure at operation {index}"),
            });
        }
        Ok(())
    }
}

impl AtomicWrite for Faulty {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.check(path)?;
        self.inner.write(path, bytes)
    }
}

impl ExclusiveCreate for Faulty {
    fn create_new(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.check(path)?;
        self.inner.create_new(path, bytes)
    }
}

impl RemoveFile for Faulty {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        self.check(path)?;
        self.inner.remove(path)
    }
}

struct Repo {
    dir: tempfile::TempDir,
}

fn entry(local_hash: &str) -> Entry {
    Entry {
        remote_updated_at: RemoteTimestamp::NotRead,
        remote_hash: String::new(),
        local_hash: local_hash.to_owned(),
        local_synced_at: 0,
    }
}

impl Repo {
    fn new() -> Result<Self, TestError> {
        let repo = Self {
            dir: tempfile::tempdir()?,
        };
        for (path, content) in [
            (DRAFT, DRAFT_CONTENT),
            (CHILD, CHILD_CONTENT),
            (PLAN, PLAN_CONTENT),
            (NOTE, NOTE_CONTENT),
            (OUTSIDE, OUTSIDE_CONTENT),
        ] {
            repo.write(path, content)?;
        }
        let store = FileCorpusStore::new(repo.root());
        fs::create_dir_all(repo.path(BASELINE).parent().ok_or("no parent")?)?;
        let baseline = BaselineStore::new(repo.path(BASELINE), &RealFs, &store);
        baseline.set("draft-k7mq3x", entry("draft"))?;
        baseline.set("0001", entry("child"))?;
        Ok(repo)
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    fn write(&self, relative: &str, content: &str) -> Result<(), TestError> {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn read(&self, relative: &str) -> Option<String> {
        fs::read_to_string(self.path(relative)).ok()
    }

    fn snapshot(&self) -> Vec<(&'static str, Option<String>)> {
        [DRAFT, TARGET, CHILD, PLAN, NOTE, OUTSIDE, BASELINE]
            .into_iter()
            .map(|path| (path, self.read(path)))
            .collect()
    }

    fn work_dir(&self) -> PathBuf {
        self.path("meta/work")
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![self.path("meta")]
    }

    fn state_dir(&self) -> PathBuf {
        self.path(".accelerator/state")
    }

    fn baseline_entry(&self, id: &str) -> Result<Option<Entry>, TestError> {
        let store = FileCorpusStore::new(self.root());
        let baseline = BaselineStore::new(self.path(BASELINE), &RealFs, &store);
        let (document, _) = baseline.load()?;
        Ok(document.get(id).cloned())
    }
}

struct Harness<'a> {
    repo: &'a Repo,
    store: Faulty,
    locks: LockdirLock,
    recovery: FileRecoveryCopies,
    status: Box<dyn WorkingCopyStatus>,
    roots: Vec<PathBuf>,
    work_dir: PathBuf,
}

impl<'a> Harness<'a> {
    fn new(repo: &'a Repo) -> Self {
        Self::failing_at(repo, None)
    }

    fn failing_at(repo: &'a Repo, fail_at: Option<usize>) -> Self {
        Self {
            repo,
            store: Faulty::new(repo.root(), fail_at),
            locks: LockdirLock::with_options(repo.work_dir(), FAST),
            recovery: FileRecoveryCopies::new(
                repo.state_dir(),
                repo.path("meta"),
            ),
            status: Box::new(EveryPath(Dirtiness::Clean)),
            roots: repo.roots(),
            work_dir: repo.work_dir(),
        }
    }

    fn files(&self) -> RetirementFiles<'_> {
        RetirementFiles {
            reader: &RealFs,
            writer: &self.store,
            creator: &self.store,
            remover: &self.store,
            file_locks: &self.locks,
            recovery: &self.recovery,
        }
    }

    fn baseline(&self) -> BaselineStore<'_> {
        BaselineStore::new(self.repo.path(BASELINE), &RealFs, &self.store)
    }

    fn plan(&self) -> Result<RetirementPlan, TestError> {
        let corpus =
            corpus_files(&RealFs, &RealFs, self.status.as_ref(), &self.roots)?;
        let items =
            identities(&FilesystemWorkItemFiles::new(&self.work_dir).files()?);
        plan_retirement(&promotion(), &self.work_dir, &items, &corpus)
            .map_err(|refusal| format!("{refusal:?}").into())
    }

    fn apply(&self, plan: &RetirementPlan) -> Result<(), RetirementFailure> {
        let Ok(lock) = acquire_retirement_lock(&self.locks) else {
            unreachable!("the retirement lock is free");
        };
        apply_retirement(plan, &self.files(), &self.baseline(), &lock)
    }

    fn finish(&self) -> Result<(), FinishFailure> {
        let baseline = self.baseline();
        let ports = RetirementPorts {
            files: self.files(),
            layout: CorpusLayout {
                roots: &self.roots,
                work_dir: &self.work_dir,
            },
            walker: &RealFs,
            status: self.status.as_ref(),
            lock: &self.locks,
            baseline: &baseline,
        };
        let Ok(lock) = acquire_retirement_lock(&self.locks) else {
            unreachable!("the retirement lock is free");
        };
        finish_retirement(&promotion(), &ports, &lock)
    }
}

const OPERATIONS: usize = 5;

#[test]
fn applying_a_plan_writes_every_rewrite_and_moves_the_file(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);

    harness
        .apply(&harness.plan()?)
        .map_err(|f| format!("{f:?}"))?;

    assert_eq!(repo.read(DRAFT), None);
    assert_eq!(
        repo.read(TARGET).as_deref(),
        Some(
            "---\nid: \"PP-900\"\nexternal_id: \"PP-900\"\n\
             aliases: [\"draft-k7mq3x\"]\ntitle: \"Add search\"\n---\n\n\
             # PP-900: Add search\n"
        )
    );
    assert_eq!(
        repo.read(CHILD).as_deref(),
        Some("---\nid: \"0001\"\nparent: \"work-item:PP-900\"\n---\n\n# 0001: Child\n")
    );
    assert_eq!(
        repo.read(PLAN).as_deref(),
        Some("See PP-900 in [it](../work/PP-900-add-search.md).\n")
    );
    assert_eq!(harness.store.count.get(), OPERATIONS);
    Ok(())
}

#[test]
fn applying_a_plan_moves_the_baseline_entry() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);

    harness
        .apply(&harness.plan()?)
        .map_err(|f| format!("{f:?}"))?;

    assert_eq!(repo.baseline_entry("draft-k7mq3x")?, None);
    assert_eq!(repo.baseline_entry("PP-900")?, Some(entry("draft")));
    assert_eq!(repo.baseline_entry("0001")?, Some(entry("child")));
    Ok(())
}

#[test]
fn a_failure_at_any_step_restores_everything() -> Result<(), TestError> {
    for fail_at in 0..OPERATIONS {
        let repo = Repo::new()?;
        let before = repo.snapshot();
        let harness = Harness::failing_at(&repo, Some(fail_at));

        let result = harness.apply(&harness.plan()?);

        assert!(
            matches!(result, Err(RetirementFailure::RolledBack { .. })),
            "operation {fail_at}: {result:?}"
        );
        assert_eq!(repo.snapshot(), before, "operation {fail_at}");
    }
    Ok(())
}

#[test]
fn a_failure_while_restoring_reports_both_failures() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::failing_at(&repo, Some(2));
    harness.store.keep_failing = true;

    let result = harness.apply(&harness.plan()?);

    let Err(RetirementFailure::RestoreIncomplete { cause, unrestored }) =
        result
    else {
        return Err(
            format!("expected an incomplete restore: {result:?}").into()
        );
    };
    assert_eq!(cause.path, repo.path(TARGET));
    assert_eq!(unrestored, vec![repo.path(CHILD), repo.path(PLAN)]);
    Ok(())
}

#[test]
fn an_uncommitted_file_touched_by_a_retirement_is_saved_for_recovery_before_the_first_write(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::failing_at(&repo, Some(0));
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));
    let recovery = repo.path(RECOVERY);
    let copy_existed = Rc::new(Cell::new(false));
    let observed = Rc::clone(&copy_existed);
    let copy = recovery.join("plans/plan.md");
    *harness.store.before_failing.borrow_mut() =
        Some(Box::new(move || observed.set(copy.is_file())));

    let _ = harness.apply(&harness.plan()?);

    assert!(copy_existed.get(), "the copy precedes the first write");
    Ok(())
}

#[test]
fn the_recovery_directory_is_removed_after_success_and_after_a_clean_first_rollback(
) -> Result<(), TestError> {
    let succeeding = Repo::new()?;
    let mut harness = Harness::new(&succeeding);
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));
    harness.finish().map_err(|f| format!("{f:?}"))?;
    assert!(!succeeding.path(RECOVERY).exists());

    let rolling_back = Repo::new()?;
    let mut harness = Harness::failing_at(&rolling_back, Some(3));
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));
    let result = harness.apply(&harness.plan()?);
    assert!(matches!(result, Err(RetirementFailure::RolledBack { .. })));
    assert!(!rolling_back.path(RECOVERY).exists());
    Ok(())
}

#[test]
fn finish_retirement_removes_the_recovery_directory_on_success(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::new(&repo);
    harness.status = Box::new(EveryPath(Dirtiness::Unknown));

    harness.finish().map_err(|f| format!("{f:?}"))?;

    assert!(!repo.path(RECOVERY).exists());
    assert_eq!(repo.read(DRAFT), None);
    Ok(())
}

#[test]
fn an_incomplete_restore_keeps_the_recovery_copies_of_unrestored_files(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::failing_at(&repo, Some(2));
    harness.store.keep_failing = true;
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));

    let _ = harness.apply(&harness.plan()?);

    let recovery = repo.path(RECOVERY);
    assert_eq!(
        fs::read_to_string(recovery.join("work/0001-child.md"))?,
        CHILD_CONTENT
    );
    assert_eq!(
        fs::read_to_string(recovery.join("plans/plan.md"))?,
        PLAN_CONTENT
    );
    Ok(())
}

#[test]
fn an_incomplete_restore_marks_the_recovery_directory_restore_pending(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::failing_at(&repo, Some(2));
    harness.store.keep_failing = true;

    let _ = harness.apply(&harness.plan()?);

    let marker =
        fs::read_to_string(repo.path(RECOVERY).join("RESTORE-PENDING"))?;
    assert!(marker.contains("0001-child.md"), "{marker}");
    assert!(marker.contains("plan.md"), "{marker}");
    Ok(())
}

#[test]
fn a_restore_pending_directory_survives_a_later_successful_retirement(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);
    harness.recovery.prepare_for_test(&repo)?;

    harness.finish().map_err(|f| format!("{f:?}"))?;

    assert!(repo.path(RECOVERY).is_dir());
    Ok(())
}

#[test]
fn a_restore_pending_marker_is_downgraded_when_the_retirement_completes(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);
    harness.recovery.prepare_for_test(&repo)?;

    harness.finish().map_err(|f| format!("{f:?}"))?;

    assert!(!repo.path(RECOVERY).join("RESTORE-PENDING").exists());
    assert!(repo.path(RECOVERY).join("COMPLETED").is_file());
    Ok(())
}

trait PendingRestore {
    fn prepare_for_test(&self, repo: &Repo) -> Result<(), TestError>;
}

impl PendingRestore for FileRecoveryCopies {
    fn prepare_for_test(&self, repo: &Repo) -> Result<(), TestError> {
        use corpus::store::RecoveryCopies as _;
        let dir = promotion().recovery_dir();
        self.prepare(&dir)?;
        self.mark_restore_pending(&dir, &[repo.path(CHILD)])?;
        Ok(())
    }
}

#[test]
fn the_recovery_ignore_rule_is_written_before_the_first_copy(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::new(&repo);
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));
    fs::create_dir_all(repo.path(RECOVERY).join(".gitignore"))?;
    let before = repo.snapshot();

    let result = harness.apply(&harness.plan()?);

    assert!(matches!(result, Err(RetirementFailure::RolledBack { .. })));
    assert!(!repo.path(RECOVERY).join("plans").exists());
    assert_eq!(repo.snapshot(), before);
    Ok(())
}

#[test]
fn an_existing_recovery_copy_is_never_overwritten() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::new(&repo);
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));
    repo.write(&format!("{RECOVERY}/plans/plan.md"), "before the crash\n")?;

    harness
        .apply(&harness.plan()?)
        .map_err(|f| format!("{f:?}"))?;

    assert_eq!(
        repo.read(&format!("{RECOVERY}/plans/plan.md")).as_deref(),
        Some("before the crash\n")
    );
    Ok(())
}

#[test]
fn a_crash_after_rewrites_before_to_is_planned_as_resumed(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::failing_at(&repo, Some(2));
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));
    repo.write(&format!("{RECOVERY}/plans/plan.md"), PLAN_CONTENT)?;

    let result = harness.apply(&harness.plan()?);

    assert!(matches!(result, Err(RetirementFailure::RolledBack { .. })));
    assert!(repo.path(RECOVERY).is_dir());
    Ok(())
}

#[test]
fn a_rollback_of_a_resumed_retirement_keeps_the_recovery_directory(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    repo.write(
        TARGET,
        "---\nid: \"PP-900\"\nexternal_id: \"PP-900\"\n\
         aliases: [\"draft-k7mq3x\"]\n---\n\n# PP-900: Add search\n",
    )?;
    let mut harness = Harness::failing_at(&repo, Some(0));
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));

    let plan = harness.plan()?;
    assert!(plan.resumes);
    let result = harness.apply(&plan);

    assert!(matches!(result, Err(RetirementFailure::RolledBack { .. })));
    assert!(repo.path(RECOVERY).is_dir());
    Ok(())
}

#[test]
fn retirement_outside_any_vcs_succeeds() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::new(&repo);
    harness.status = Box::new(EveryPath(Dirtiness::Unknown));

    harness.finish().map_err(|f| format!("{f:?}"))?;

    assert!(repo.read(TARGET).is_some());
    Ok(())
}

#[test]
fn files_outside_the_corpus_root_are_byte_identical() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);

    harness.finish().map_err(|f| format!("{f:?}"))?;

    assert_eq!(repo.read(OUTSIDE).as_deref(), Some(OUTSIDE_CONTENT));
    assert_eq!(repo.read(NOTE).as_deref(), Some(NOTE_CONTENT));
    Ok(())
}

#[test]
fn a_retired_id_resolves_through_its_alias_after_rereading_the_corpus(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);

    harness.finish().map_err(|f| format!("{f:?}"))?;

    let items =
        identities(&FilesystemWorkItemFiles::new(&repo.work_dir()).files()?);
    let IdentityResolution::Unique(item) =
        resolve_identity("draft-k7mq3x", &items)
    else {
        return Err("the alias does not resolve".into());
    };
    assert_eq!(item.path, repo.path(TARGET));
    Ok(())
}

#[test]
fn a_file_changed_after_the_snapshot_aborts_and_rolls_back(
) -> Result<(), TestError> {
    for edited in [PLAN, DRAFT] {
        let repo = Repo::new()?;
        let harness = Harness::new(&repo);
        let plan = harness.plan()?;
        repo.write(edited, "edited meanwhile\n")?;
        let before = repo.snapshot();

        let result = harness.apply(&plan);

        let Err(RetirementFailure::RolledBack { cause }) = result else {
            return Err(format!("{edited}: {result:?}").into());
        };
        assert_eq!(cause.kind, RetirementCauseKind::ChangedSinceSnapshot);
        assert_eq!(cause.path, repo.path(edited));
        assert_eq!(repo.snapshot(), before, "{edited}");
    }
    Ok(())
}

#[test]
fn a_file_changed_after_the_snapshot_leaves_no_recovery_directory(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::new(&repo);
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));
    let plan = harness.plan()?;
    repo.write(PLAN, "edited meanwhile\n")?;

    let result = harness.apply(&plan);

    assert!(matches!(result, Err(RetirementFailure::RolledBack { .. })));
    assert!(!repo.path(RECOVERY).exists());
    Ok(())
}

#[test]
fn an_edit_between_planning_and_applying_is_detected() -> Result<(), TestError>
{
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);
    let plan = harness.plan()?;
    repo.write(CHILD, &CHILD_CONTENT.replace("Child", "Renamed"))?;

    let result = harness.apply(&plan);

    assert!(matches!(
        result,
        Err(RetirementFailure::RolledBack { cause })
            if cause.kind == RetirementCauseKind::ChangedSinceSnapshot
    ));
    Ok(())
}

#[test]
fn finish_retirement_replans_once_on_a_changed_file() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::failing_at(&repo, Some(0));
    let edit_path = repo.path(CHILD);
    *harness.store.before_failing.borrow_mut() = Some(Box::new(move || {
        let _ =
            fs::write(&edit_path, CHILD_CONTENT.replace("Child", "Renamed"));
    }));

    harness.finish().map_err(|f| format!("{f:?}"))?;

    assert_eq!(
        repo.read(CHILD).as_deref(),
        Some("---\nid: \"0001\"\nparent: \"work-item:PP-900\"\n---\n\n# 0001: Renamed\n")
    );
    Ok(())
}

#[test]
fn finish_retirement_retries_a_store_failure_once_then_succeeds(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::failing_at(&repo, Some(3));

    harness.finish().map_err(|f| format!("{f:?}"))?;

    assert_eq!(repo.read(DRAFT), None);
    assert!(repo.read(TARGET).is_some());
    Ok(())
}

#[test]
fn finish_retirement_gives_up_after_a_second_failure() -> Result<(), TestError>
{
    let repo = Repo::new()?;
    let mut harness = Harness::failing_at(&repo, Some(0));
    harness.store.keep_failing = true;
    let before = repo.snapshot();

    let result = harness.finish();

    assert!(matches!(
        result,
        Err(FinishFailure::Failed(RetirementFailure::RolledBack { .. }))
    ));
    assert_eq!(repo.snapshot(), before);
    Ok(())
}

#[test]
fn finish_retirement_returns_a_planning_refusal() -> Result<(), TestError> {
    let repo = Repo::new()?;
    repo.write(
        "meta/work/PP-900-other.md",
        "---\nid: \"PP-900\"\n---\n\n# PP-900: Other\n",
    )?;
    let harness = Harness::new(&repo);

    let result = harness.finish();

    assert!(matches!(
        result,
        Err(FinishFailure::Refused(RetirementRefusal::IdTaken { .. }))
    ));
    Ok(())
}

#[test]
fn a_held_update_lock_on_a_touched_file_blocks_the_retirement_write(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);
    let plan = harness.plan()?;
    let before = repo.snapshot();
    let _held = harness
        .locks
        .acquire(&LockName::ForFile(repo.path(CHILD)))?;

    let result = harness.apply(&plan);

    assert!(matches!(
        result,
        Err(RetirementFailure::RolledBack { cause })
            if matches!(cause.kind, RetirementCauseKind::Store(StoreError::LockTimeout { .. }))
    ));
    assert_eq!(repo.snapshot(), before);
    Ok(())
}

#[test]
fn a_file_edited_after_the_retirement_wrote_it_is_not_rolled_back_over(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mut harness = Harness::failing_at(&repo, Some(2));
    harness.status = Box::new(EveryPath(Dirtiness::Dirty));
    let edit_path = repo.path(CHILD);
    *harness.store.before_failing.borrow_mut() = Some(Box::new(move || {
        let _ = fs::write(&edit_path, "someone else's edit\n");
    }));

    let result = harness.apply(&harness.plan()?);

    let Err(RetirementFailure::RestoreIncomplete { unrestored, .. }) = result
    else {
        return Err(
            format!("expected an incomplete restore: {result:?}").into()
        );
    };
    assert_eq!(unrestored, vec![repo.path(CHILD)]);
    assert_eq!(repo.read(CHILD).as_deref(), Some("someone else's edit\n"));
    assert_eq!(repo.read(PLAN).as_deref(), Some(PLAN_CONTENT));
    assert!(repo.path(RECOVERY).join("work/0001-child.md").is_file());
    Ok(())
}

#[test]
fn an_existing_target_created_after_planning_is_not_overwritten(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);
    let plan = harness.plan()?;
    repo.write(TARGET, "arrived meanwhile\n")?;

    let result = harness.apply(&plan);

    assert!(matches!(
        result,
        Err(RetirementFailure::RolledBack { cause })
            if cause.kind == RetirementCauseKind::TargetAppeared
    ));
    assert_eq!(repo.read(TARGET).as_deref(), Some("arrived meanwhile\n"));
    Ok(())
}

#[test]
fn a_held_retirement_lock_blocks_a_second_retirement() -> Result<(), TestError>
{
    let repo = Repo::new()?;
    let harness = Harness::new(&repo);
    let _first = acquire_retirement_lock(&harness.locks)?;

    let second = acquire_retirement_lock(&harness.locks);

    assert!(matches!(second, Err(StoreError::LockTimeout { .. })));
    Ok(())
}

#[cfg(feature = "bash-parity")]
mod in_a_real_repository {
    use std::fs;

    use vcs_adapters::library::InProcessProbe;
    use vcs_test_support::hermetic::Hermetic;
    use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

    use super::Harness;
    use super::Repo;
    use super::TestError;
    use super::DRAFT;
    use super::TARGET;

    fn retire_the_uncommitted_draft(
        repo: &Repo,
        status: VcsWorkingCopyStatus,
    ) -> Result<(), TestError> {
        let mut harness = Harness::new(repo);
        harness.status = Box::new(status);
        let plan = harness.plan()?;
        assert!(plan.recovery.contains(&repo.path(DRAFT)));

        harness.finish().map_err(|f| format!("{f:?}"))?;

        assert_eq!(repo.read(DRAFT), None);
        assert!(repo.read(TARGET).is_some());
        Ok(())
    }

    #[test]
    fn a_freshly_written_untracked_draft_is_retired_in_a_git_repo(
    ) -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let repo = Repo::new()?;
        let env = Hermetic::rooted_at(repo.root())?;
        let draft = fs::read_to_string(repo.path(DRAFT))?;
        fs::remove_file(repo.path(DRAFT))?;
        env.git(&["init", "--quiet"], repo.root())?;
        env.git(&["add", "meta"], repo.root())?;
        env.git(&["commit", "--quiet", "-m", "init"], repo.root())?;
        repo.write(DRAFT, &draft)?;

        retire_the_uncommitted_draft(
            &repo,
            VcsWorkingCopyStatus::probed_from(repo.root(), &InProcessProbe),
        )
    }

    #[test]
    fn a_freshly_written_untracked_draft_is_retired_in_a_jj_repo(
    ) -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let repo = Repo::new()?;
        let env = Hermetic::rooted_at(repo.root())?;
        let draft = fs::read_to_string(repo.path(DRAFT))?;
        fs::remove_file(repo.path(DRAFT))?;
        env.jj(&["git", "init", "--no-colocate"], repo.root())?;
        env.jj(&["commit", "-m", "init"], repo.root())?;
        repo.write(DRAFT, &draft)?;

        retire_the_uncommitted_draft(
            &repo,
            VcsWorkingCopyStatus::probed_from(repo.root(), &InProcessProbe),
        )
    }
}

#[test]
fn a_restore_pending_directory_is_reported_until_every_path_matches_its_copy(
) -> Result<(), TestError> {
    use corpus::store::RecoveryCopies as _;
    use work_adapters::retirement::sweep_recoveries;
    use work_adapters::retirement::RecoveryNotice;

    let repo = Repo::new()?;
    let recovery = FileRecoveryCopies::new(repo.state_dir(), repo.path("meta"));
    let dir = promotion().recovery_dir();
    let child = repo.path(CHILD);
    recovery.prepare(&dir)?;
    recovery.write_once(&dir, &child, CHILD_CONTENT.as_bytes())?;
    recovery.mark_restore_pending(&dir, std::slice::from_ref(&child))?;
    repo.write(CHILD, "edited after the retirement wrote it")?;

    assert_eq!(
        sweep_recoveries(&recovery)?,
        vec![RecoveryNotice::StillPending {
            dir: dir.clone(),
            unrestored: vec![child],
        }]
    );
    repo.write(CHILD, CHILD_CONTENT)?;
    assert_eq!(
        sweep_recoveries(&recovery)?,
        vec![RecoveryNotice::Cleared { dir }]
    );
    assert!(!repo.path(RECOVERY).exists());
    Ok(())
}

#[test]
fn a_completed_directory_is_removed_by_the_next_sweep() -> Result<(), TestError>
{
    use corpus::store::RecoveryCopies as _;
    use work_adapters::retirement::sweep_recoveries;

    let repo = Repo::new()?;
    let recovery = FileRecoveryCopies::new(repo.state_dir(), repo.path("meta"));
    let dir = promotion().recovery_dir();
    recovery.prepare(&dir)?;
    recovery.mark_restore_pending(&dir, &[])?;
    recovery.mark_completed(&dir)?;

    assert!(sweep_recoveries(&recovery)?.is_empty());
    assert!(!repo.path(RECOVERY).exists());
    Ok(())
}

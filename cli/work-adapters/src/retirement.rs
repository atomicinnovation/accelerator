//! Applying a [`RetirementPlan`] all-or-nothing across the corpus, and the
//! one path that plans, applies and cleans up a retirement.

use std::path::Path;
use std::path::PathBuf;

use corpus::lock::ExclusiveLock;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::scan::CorpusWalker;
use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::ExclusiveCreate;
use corpus::store::FileRemove;
use corpus::store::KeptState;
use corpus::store::RecoveryCopies;
use corpus::StoreError;
use work::identity::ItemIdentity;
use work::retirement::plan_retirement;
use work::retirement::CorpusFile;
use work::retirement::Retirement;
use work::retirement::RetirementCause;
use work::retirement::RetirementCauseKind;
use work::retirement::RetirementFailure;
use work::retirement::RetirementPlan;
use work::retirement::RetirementRefusal;
use work::retirement::RECOVERY_PARENT;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFile;
use work::work_item_files::DRAFTS_DIRECTORY;

use crate::sync::baseline_store::BaselineStore;
use crate::sync::fetch::WorkingCopyStatus;

#[derive(Clone, Copy)]
pub struct RetirementFiles<'a> {
    pub reader: &'a dyn FileReader,
    pub writer: &'a dyn AtomicWrite,
    pub creator: &'a dyn ExclusiveCreate,
    pub remover: &'a dyn FileRemove,
    pub file_locks: &'a dyn ExclusiveLock,
    pub recovery: &'a dyn RecoveryCopies,
}

/// Where the corpus lives: every root it is walked from, and the work
/// directory inside it.
#[derive(Clone, Copy)]
pub struct CorpusLayout<'a> {
    pub roots: &'a [PathBuf],
    pub work_dir: &'a Path,
}

#[derive(Clone, Copy)]
pub struct RetirementPorts<'a> {
    pub files: RetirementFiles<'a>,
    pub layout: CorpusLayout<'a>,
    pub walker: &'a dyn CorpusWalker,
    pub status: &'a dyn WorkingCopyStatus,
    pub lock: &'a dyn ExclusiveLock,
    pub baseline: &'a BaselineStore<'a>,
}

/// Proof that the caller holds the retirement lock, so no retirement can
/// apply unlocked.
pub struct RetirementLockGuard {
    _held: HeldLock,
}

/// # Errors
///
/// [`StoreError`] when the lock cannot be taken.
pub fn acquire_retirement_lock(
    lock: &dyn ExclusiveLock,
) -> Result<RetirementLockGuard, StoreError> {
    lock.acquire(&LockName::Retirement)
        .map(|held| RetirementLockGuard { _held: held })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishFailure {
    Refused(RetirementRefusal),
    Failed(RetirementFailure),
}

/// Every markdown file under the corpus roots with its content and
/// dirtiness.
///
/// # Errors
///
/// A [`kernel::Error`] when a root or a file cannot be read.
pub fn corpus_files(
    walker: &dyn CorpusWalker,
    reader: &dyn FileReader,
    status: &dyn WorkingCopyStatus,
    roots: &[PathBuf],
) -> Result<Vec<CorpusFile>, kernel::Error> {
    let mut paths = walker.walk_markdown(roots)?;
    paths.sort();
    paths.dedup();
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        if let Some(content) = reader.read(&path)? {
            let dirtiness = status.is_dirty(&path);
            files.push(CorpusFile {
                path,
                content,
                dirtiness,
            });
        }
    }
    Ok(files)
}

fn is_work_item_file(path: &Path, work_dir: &Path) -> bool {
    let parent = path.parent();
    parent == Some(work_dir)
        || parent == Some(work_dir.join(DRAFTS_DIRECTORY).as_path())
}

fn work_items(corpus: &[CorpusFile], work_dir: &Path) -> Vec<WorkItemFile> {
    corpus
        .iter()
        .filter(|file| is_work_item_file(&file.path, work_dir))
        .map(|file| WorkItemFile {
            path: file.path.clone(),
            content: file.content.clone(),
        })
        .collect()
}

fn cause(path: &Path, kind: RetirementCauseKind) -> RetirementCause {
    RetirementCause {
        path: path.to_path_buf(),
        kind,
    }
}

fn store_cause(path: &Path, error: StoreError) -> RetirementCause {
    let kind = match error {
        StoreError::AlreadyExists { .. } => RetirementCauseKind::TargetAppeared,
        other => RetirementCauseKind::Store(other),
    };
    cause(path, kind)
}

enum Step<'a> {
    Rewrite {
        path: &'a Path,
        original: &'a str,
        content: &'a str,
    },
    Create {
        path: &'a Path,
        content: &'a str,
    },
    Remove {
        path: &'a Path,
        original: &'a str,
    },
    RenameBaseline,
}

fn steps(plan: &RetirementPlan) -> Vec<Step<'_>> {
    let mut steps: Vec<Step<'_>> = plan
        .rewrites
        .iter()
        .map(|rewrite| Step::Rewrite {
            path: &rewrite.path,
            original: &rewrite.original,
            content: &rewrite.content,
        })
        .collect();
    if let Some(content) = &plan.retired_item {
        steps.push(Step::Create {
            path: &plan.to,
            content,
        });
    }
    if let Some(original) = &plan.from_original {
        steps.push(Step::Remove {
            path: &plan.from,
            original,
        });
    }
    steps.push(Step::RenameBaseline);
    steps
}

fn touched_paths(plan: &RetirementPlan) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = plan
        .rewrites
        .iter()
        .map(|rewrite| rewrite.path.clone())
        .collect();
    if plan.from_original.is_some() {
        paths.push(plan.from.clone());
    }
    if plan.retired_item.is_some() {
        paths.push(plan.to.clone());
    }
    paths.sort();
    paths.dedup();
    paths
}

fn snapshot_of<'a>(plan: &'a RetirementPlan, path: &Path) -> Option<&'a str> {
    plan.rewrites
        .iter()
        .find(|rewrite| rewrite.path == path)
        .map(|rewrite| rewrite.original.as_str())
        .or_else(|| {
            (plan.from == path)
                .then_some(plan.from_original.as_deref())
                .flatten()
        })
}

fn current(
    reader: &dyn FileReader,
    path: &Path,
) -> Result<Option<String>, RetirementCause> {
    reader.read(path).map_err(|error| {
        cause(
            path,
            RetirementCauseKind::Store(StoreError::Io {
                path: path.display().to_string(),
                detail: error.to_string(),
            }),
        )
    })
}

fn save_recovery_copies(
    plan: &RetirementPlan,
    files: &RetirementFiles<'_>,
) -> Result<(), RetirementCause> {
    if plan.recovery.is_empty() {
        return Ok(());
    }
    files
        .recovery
        .prepare(&plan.recovery_dir)
        .map_err(|error| store_cause(&plan.recovery_dir, error))?;
    for path in &plan.recovery {
        let Some(bytes) = snapshot_of(plan, path) else {
            continue;
        };
        match files.recovery.write_once(
            &plan.recovery_dir,
            path,
            bytes.as_bytes(),
        ) {
            Ok(()) | Err(StoreError::AlreadyExists { .. }) => {}
            Err(error) => return Err(store_cause(path, error)),
        }
    }
    Ok(())
}

fn lock_touched_files(
    plan: &RetirementPlan,
    locks: &dyn ExclusiveLock,
) -> Result<Vec<HeldLock>, RetirementCause> {
    touched_paths(plan)
        .into_iter()
        .map(|path| {
            locks
                .acquire(&LockName::ForFile(path.clone()))
                .map_err(|error| store_cause(&path, error))
        })
        .collect()
}

fn verify_unchanged(
    plan: &RetirementPlan,
    reader: &dyn FileReader,
) -> Result<(), RetirementCause> {
    let changed =
        |path: &Path| cause(path, RetirementCauseKind::ChangedSinceSnapshot);
    for rewrite in &plan.rewrites {
        if current(reader, &rewrite.path)?.as_deref()
            != Some(rewrite.original.as_str())
        {
            return Err(changed(&rewrite.path));
        }
    }
    if let Some(original) = &plan.from_original {
        if current(reader, &plan.from)?.as_deref() != Some(original.as_str()) {
            return Err(changed(&plan.from));
        }
    }
    if plan.retired_item.is_some() && current(reader, &plan.to)?.is_some() {
        return Err(cause(&plan.to, RetirementCauseKind::TargetAppeared));
    }
    Ok(())
}

fn take(
    step: &Step<'_>,
    plan: &RetirementPlan,
    files: &RetirementFiles<'_>,
    baseline: &BaselineStore<'_>,
) -> Result<(), RetirementCause> {
    match step {
        Step::Rewrite { path, content, .. } => files
            .writer
            .write(path, content.as_bytes())
            .map_err(|error| store_cause(path, error)),
        Step::Create { path, content } => files
            .creator
            .create_new(path, content.as_bytes())
            .map_err(|error| store_cause(path, error)),
        Step::Remove { path, .. } => files
            .remover
            .remove(path)
            .map_err(|error| store_cause(path, error)),
        Step::RenameBaseline => baseline
            .rename(&plan.baseline_rename.from, &plan.baseline_rename.to)
            .map_err(|error| store_cause(baseline.path(), error)),
    }
}

/// Reverses one taken step, unless the path has changed since the
/// retirement wrote it: those edits are someone else's, so the path is left
/// for a person to merge and reported.
fn undo(step: &Step<'_>, files: &RetirementFiles<'_>) -> Result<(), PathBuf> {
    let unrestored = |path: &Path| path.to_path_buf();
    match step {
        Step::Rewrite {
            path,
            original,
            content,
        } => {
            let still_ours = current(files.reader, path)
                .ok()
                .flatten()
                .is_some_and(|now| now == *content);
            if !still_ours {
                return Err(unrestored(path));
            }
            files
                .writer
                .write(path, original.as_bytes())
                .map_err(|_| unrestored(path))
        }
        Step::Create { path, content } => {
            let still_ours = current(files.reader, path)
                .ok()
                .flatten()
                .is_some_and(|now| now == *content);
            if !still_ours {
                return Err(unrestored(path));
            }
            files.remover.remove(path).map_err(|_| unrestored(path))
        }
        Step::Remove { path, original } => files
            .creator
            .create_new(path, original.as_bytes())
            .map_err(|_| unrestored(path)),
        Step::RenameBaseline => Ok(()),
    }
}

fn roll_back(
    taken: &[Step<'_>],
    first_failure: RetirementCause,
    plan: &RetirementPlan,
    files: &RetirementFiles<'_>,
    resumes: bool,
) -> RetirementFailure {
    let unrestored: Vec<PathBuf> = taken
        .iter()
        .rev()
        .filter_map(|step| undo(step, files).err())
        .collect();
    if unrestored.is_empty() {
        if !resumes && !files.recovery.is_restore_pending(&plan.recovery_dir) {
            let _ = files.recovery.remove_dir(&plan.recovery_dir);
        }
        return RetirementFailure::RolledBack {
            cause: first_failure,
        };
    }
    let _ = files.recovery.prepare(&plan.recovery_dir).and_then(|()| {
        files
            .recovery
            .mark_restore_pending(&plan.recovery_dir, &unrestored)
    });
    RetirementFailure::RestoreIncomplete {
        cause: first_failure,
        unrestored,
    }
}

/// Applies every step of `plan` or, on the first failure, restores every
/// path it had changed.
///
/// # Errors
///
/// [`RetirementFailure::RolledBack`] when the corpus is back as it was, and
/// [`RetirementFailure::RestoreIncomplete`] naming the paths it could not
/// restore.
pub fn apply_retirement(
    plan: &RetirementPlan,
    files: &RetirementFiles<'_>,
    baseline: &BaselineStore<'_>,
    _lock: &RetirementLockGuard,
) -> Result<(), RetirementFailure> {
    let resumes = plan.resumes || files.recovery.exists(&plan.recovery_dir);
    let rolled_back = |cause| RetirementFailure::RolledBack { cause };
    let _file_locks =
        lock_touched_files(plan, files.file_locks).map_err(rolled_back)?;
    verify_unchanged(plan, files.reader).map_err(rolled_back)?;
    if let Err(cause) = save_recovery_copies(plan, files) {
        if !resumes {
            let _ = files.recovery.remove_dir(&plan.recovery_dir);
        }
        return Err(rolled_back(cause));
    }
    let steps = steps(plan);
    for (index, step) in steps.iter().enumerate() {
        if let Err(failure) = take(step, plan, files, baseline) {
            return Err(roll_back(
                &steps[..index],
                failure,
                plan,
                files,
                resumes,
            ));
        }
    }
    Ok(())
}

fn walked_corpus(
    ports: &RetirementPorts<'_>,
) -> Result<Vec<CorpusFile>, FinishFailure> {
    corpus_files(
        ports.walker,
        ports.files.reader,
        ports.status,
        ports.layout.roots,
    )
    .map_err(|error| {
        FinishFailure::Failed(RetirementFailure::RolledBack {
            cause: cause(
                ports.layout.work_dir,
                RetirementCauseKind::Store(StoreError::Io {
                    path: ports.layout.work_dir.display().to_string(),
                    detail: error.to_string(),
                }),
            ),
        })
    })
}

/// Plans `retirement` over the corpus as it now stands.
///
/// # Errors
///
/// [`FinishFailure::Refused`] when planning refuses, and
/// [`FinishFailure::Failed`] when the corpus cannot be read.
pub fn plan_from_corpus(
    retirement: &Retirement<'_>,
    ports: &RetirementPorts<'_>,
) -> Result<RetirementPlan, FinishFailure> {
    let corpus = walked_corpus(ports)?;
    let items = identities(&work_items(&corpus, ports.layout.work_dir));
    plan_retirement(retirement, ports.layout.work_dir, &items, &corpus)
        .map_err(FinishFailure::Refused)
}

/// The identities of every work item, drafts included, as the corpus now
/// stands.
///
/// # Errors
///
/// [`FinishFailure::Failed`] when the corpus cannot be read.
pub fn corpus_identities(
    ports: &RetirementPorts<'_>,
) -> Result<Vec<ItemIdentity>, FinishFailure> {
    let corpus = walked_corpus(ports)?;
    Ok(identities(&work_items(&corpus, ports.layout.work_dir)))
}

/// Whether `retirement` still has a step to take: a file to write, create
/// or remove, or a baseline entry still under the old ID.
///
/// # Errors
///
/// [`FinishFailure`] when planning refuses or the corpus cannot be read.
pub fn retirement_outstanding(
    retirement: &Retirement<'_>,
    ports: &RetirementPorts<'_>,
) -> Result<bool, FinishFailure> {
    let plan = plan_from_corpus(retirement, ports)?;
    let baseline_still_old = ports
        .baseline
        .load()
        .map(|(baseline, _)| baseline.get(retirement.old_id).is_some())
        .unwrap_or(true);
    Ok(!plan.leaves_corpus_unchanged() || baseline_still_old)
}

/// Plans and applies a retirement, whether fresh or interrupted.
///
/// Plans from the corpus as it now stands, and re-plans once if the first
/// attempt rolls back. On success the recovery copies are removed, unless
/// an earlier incomplete restore still needs them.
///
/// # Errors
///
/// [`FinishFailure::Refused`] when planning refuses, and
/// [`FinishFailure::Failed`] when the second attempt also fails or a
/// restore was incomplete.
pub fn finish_retirement(
    retirement: &Retirement<'_>,
    ports: &RetirementPorts<'_>,
    lock: &RetirementLockGuard,
) -> Result<(), FinishFailure> {
    let mut attempts_left = 2;
    loop {
        attempts_left -= 1;
        let plan = plan_from_corpus(retirement, ports)?;
        match apply_retirement(&plan, &ports.files, ports.baseline, lock) {
            Ok(()) => break,
            Err(RetirementFailure::RolledBack { .. }) if attempts_left > 0 => {}
            Err(failure) => return Err(FinishFailure::Failed(failure)),
        }
    }
    let recovery_dir = retirement.recovery_dir();
    let recovery = ports.files.recovery;
    if recovery.is_restore_pending(&recovery_dir) {
        let _ = recovery.mark_completed(&recovery_dir);
    } else {
        let _ = recovery.remove_dir(&recovery_dir);
    }
    Ok(())
}

/// A recovery directory a sweep found still needing a person, or found
/// dealt with and removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryNotice {
    StillPending {
        dir: PathBuf,
        unrestored: Vec<PathBuf>,
    },
    Cleared {
        dir: PathBuf,
    },
}

/// Removes the recovery directories nobody needs any more.
///
/// That is a completed retirement's, whose notice an earlier run gave, and
/// an incomplete restore's once every unrestored path matches its copy
/// again or the copy has been deleted. Reports every restore still pending.
///
/// # Errors
///
/// [`StoreError`] when the directories cannot be listed or one cannot be
/// removed.
pub fn sweep_recoveries(
    recovery: &dyn RecoveryCopies,
) -> Result<Vec<RecoveryNotice>, StoreError> {
    let mut notices = Vec::new();
    for kept in recovery.kept(Path::new(RECOVERY_PARENT))? {
        match kept.state {
            KeptState::Completed => recovery.remove_dir(&kept.dir)?,
            KeptState::RestorePending(unrestored) => {
                let settled = unrestored
                    .iter()
                    .all(|path| recovery.copy_settled(&kept.dir, path));
                if settled {
                    recovery.remove_dir(&kept.dir)?;
                    notices.push(RecoveryNotice::Cleared { dir: kept.dir });
                } else {
                    notices.push(RecoveryNotice::StillPending {
                        dir: kept.dir,
                        unrestored,
                    });
                }
            }
        }
    }
    Ok(notices)
}

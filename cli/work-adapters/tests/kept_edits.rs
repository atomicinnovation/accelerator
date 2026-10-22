//! Which kept pull copies a run reports and which it sweeps: a copy stays
//! until it settles or a person deletes it, whatever its age.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unimplemented
)]

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus::lock::ExclusiveLock;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::store::RecoveryCopies as _;
use corpus::store::StoreError;
use corpus::WorkItemIdScheme;
use corpus_adapters::FileRecoveryCopies;
use tempfile::TempDir;
use tracker::ExternalId;
use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::kept_edits::KeptCopy;
use work_adapters::sync::kept_edits::KeptEdits;
use work_adapters::sync::kept_edits::KeptEditsRetention;
use work_adapters::sync::kept_edits::Provenance;

const SYNCED: &str = "---\nstatus: ready\n---\n\nAs pulled\n";
const EDITED: &str = "---\nstatus: ready\n---\n\nEdited locally\n";

#[derive(Default)]
struct Tracking {
    answers: RefCell<Vec<(PathBuf, FileTracking)>>,
}

impl Tracking {
    fn answer(&self, path: PathBuf, tracking: FileTracking) {
        self.answers.borrow_mut().push((path, tracking));
    }
}

impl RepositoryTracking for Tracking {
    fn file_tracking(&self, path: &Path) -> FileTracking {
        self.answers
            .borrow()
            .iter()
            .find(|(answered, _)| path.starts_with(answered))
            .map_or(FileTracking::Untracked, |(_, tracking)| *tracking)
    }

    fn repository_roots(&self, _directory: &Path) -> RootsAnswer {
        unimplemented!("the review asks only whether a file is tracked")
    }
}

#[derive(Default)]
struct Locks {
    refused: RefCell<bool>,
    before_granting: RefCell<Option<(PathBuf, String)>>,
}

impl ExclusiveLock for Locks {
    fn acquire(&self, name: &LockName) -> Result<HeldLock, StoreError> {
        if *self.refused.borrow() {
            return Err(StoreError::LockTimeout {
                path: format!("{name:?}"),
            });
        }
        if let Some((path, content)) = self.before_granting.borrow_mut().take()
        {
            fs::write(path, content).expect("the item is edited");
        }
        Ok(HeldLock::new(()))
    }

    fn clear_abandoned(&self, _name: &LockName) {}
}

struct Repo {
    root: TempDir,
    recovery: FileRecoveryCopies,
    tracking: Tracking,
    locks: Locks,
    items: Vec<LocalItem>,
    scheme: WorkItemIdScheme,
}

impl Repo {
    fn new() -> Self {
        let root = TempDir::new().expect("a repository");
        let state = root.path().join(".accelerator/state");
        fs::create_dir_all(&state).expect("the state root");
        fs::create_dir_all(root.path().join("meta/work")).expect("work");
        Self {
            recovery: FileRecoveryCopies::new(&state, root.path()),
            root,
            tracking: Tracking::default(),
            locks: Locks::default(),
            items: Vec::new(),
            scheme: WorkItemIdScheme::numeric(),
        }
    }

    fn work_root(&self) -> PathBuf {
        self.root.path().join("meta/work")
    }

    fn item(&mut self, id: &str, content: &str) -> PathBuf {
        let path = self.work_root().join(format!("{id}.md"));
        fs::write(&path, content).expect("the item");
        self.items.push(LocalItem {
            id: id.to_owned(),
            path: path.clone(),
            external_id: Some(ExternalId::new(format!("ENG-{id}"))),
        });
        path
    }

    /// Keeps `copy` of `item` as a pull keeps it, in the directory named
    /// `name` under `sync-recovery`.
    fn keep(&self, name: &str, item: &Path, copy: &str) -> PathBuf {
        let dir = Path::new("sync-recovery").join(name);
        self.recovery.prepare(&dir).expect("prepare");
        self.recovery
            .write_once(&dir, item, copy.as_bytes())
            .expect("write the copy");
        self.recovery
            .mark_copy_pending(&self.recovery.mirrored_copy(&dir, item))
            .expect("record the copy");
        dir
    }

    fn location(&self, dir: &Path) -> PathBuf {
        self.recovery.location(dir)
    }

    fn copy_location(&self, dir: &Path, item: &Path) -> PathBuf {
        self.location(&self.recovery.mirrored_copy(dir, item).copy_path())
    }

    fn retention(&self) -> KeptEditsRetention<'_> {
        KeptEditsRetention {
            recovery: &self.recovery,
            tracking: &self.tracking,
            locks: &self.locks,
            work_root: self.root.path().join("meta/work"),
            scheme: &self.scheme,
            corpus: &self.items,
        }
    }

    fn review(&self) -> Vec<KeptEdits> {
        self.review_keeping(&BTreeSet::new())
    }

    fn review_keeping(&self, this_run: &BTreeSet<PathBuf>) -> Vec<KeptEdits> {
        self.retention()
            .review(this_run)
            .expect("the review")
            .entries
    }

    fn sweep(&self) -> Vec<KeptEdits> {
        let retention = self.retention();
        let review = retention.review(&BTreeSet::new()).expect("the review");
        retention.sweep(review).entries
    }
}

fn kept_copy(repo: &Repo, id: &str, dir: &Path, item: &Path) -> KeptCopy {
    KeptCopy {
        id: id.to_owned(),
        item: item.to_path_buf(),
        copy: repo.copy_location(dir, item),
        directory: repo.location(dir),
        kept_since: Some(1_700_000_000),
    }
}

#[test]
fn a_kept_pull_copy_survives_the_next_sweep() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, EDITED);

    let swept = repo.sweep();

    assert_eq!(
        swept,
        vec![KeptEdits::KeptEarlier(kept_copy(
            &repo, "0001", &dir, &item
        ))]
    );
    assert!(repo.location(&dir).is_dir());
}

#[test]
fn an_old_unsettled_copy_is_never_swept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("0-k7mq3x-0001", &item, EDITED);

    repo.sweep();

    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_settled_copy_is_swept_whatever_its_age() {
    for epoch in ["0", "1700000000"] {
        let mut repo = Repo::new();
        let item = repo.item("0001", SYNCED);
        let dir = repo.keep(&format!("{epoch}-k7mq3x-0001"), &item, SYNCED);

        let swept = repo.sweep();

        assert_eq!(swept, Vec::new(), "{epoch}");
        assert!(!repo.location(&dir).exists(), "{epoch}");
    }
}

#[test]
fn a_copy_that_changed_after_review_is_kept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, SYNCED);
    *repo.locks.before_granting.borrow_mut() =
        Some((item.clone(), EDITED.to_owned()));

    let swept = repo.sweep();

    assert_eq!(
        swept,
        vec![KeptEdits::KeptEarlier(kept_copy(
            &repo, "0001", &dir, &item
        ))]
    );
    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_copy_whose_item_lock_cannot_be_taken_is_kept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, SYNCED);
    *repo.locks.refused.borrow_mut() = true;

    repo.sweep();

    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_copy_whose_item_file_is_missing_is_kept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, EDITED);
    fs::remove_file(&item).expect("the item is deleted");

    let swept = repo.sweep();

    assert_eq!(
        swept,
        vec![KeptEdits::KeptEarlier(kept_copy(
            &repo, "0001", &dir, &item
        ))]
    );
}

#[test]
fn a_copy_that_cannot_be_read_is_kept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, SYNCED);
    let copy = repo.copy_location(&dir, &item);
    fs::remove_file(&copy).expect("the copy is replaced");
    fs::create_dir(&copy).expect("by a directory");

    repo.sweep();

    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_directory_without_a_record_is_never_swept() {
    let repo = Repo::new();
    let dir = Path::new("sync-recovery/1700000000-k7mq3x-0001");
    repo.recovery.prepare(dir).expect("prepare");

    let swept = repo.sweep();

    assert_eq!(
        swept,
        vec![KeptEdits::Unrecorded {
            id: Some("0001".to_owned()),
            directory: repo.location(dir),
            provenance: Provenance::Unknown,
        }]
    );
    assert!(repo.location(dir).is_dir());
}

#[test]
fn an_unparsable_epoch_is_kept_since_an_unknown_date() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("soon-k7mq3x-0001", &item, EDITED);

    assert_eq!(
        repo.review(),
        vec![KeptEdits::KeptEarlier(KeptCopy {
            kept_since: None,
            ..kept_copy(&repo, "0001", &dir, &item)
        })]
    );
}

#[test]
fn this_runs_directories_are_never_swept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, SYNCED);
    let retention = repo.retention();
    let this_run = BTreeSet::from([dir.clone()]);

    let review = retention.review(&this_run).expect("the review");
    assert_eq!(
        review.entries,
        vec![KeptEdits::KeptThisRun(kept_copy(
            &repo, "0001", &dir, &item
        ))]
    );
    let _ = retention.sweep(review);

    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_preview_sweeps_nothing_but_reports_carried_over_copies() {
    let mut repo = Repo::new();
    let settled = repo.item("0001", SYNCED);
    let kept = repo.item("0002", SYNCED);
    let settled_dir = repo.keep("1700000000-k7mq3x-0001", &settled, SYNCED);
    let kept_dir = repo.keep("1700000000-k7mq3x-0002", &kept, EDITED);

    let review = repo.review();

    assert_eq!(review.len(), 2);
    assert!(matches!(review[0], KeptEdits::Settled { .. }));
    assert_eq!(
        review[1],
        KeptEdits::KeptEarlier(kept_copy(&repo, "0002", &kept_dir, &kept))
    );
    assert!(repo.location(&settled_dir).is_dir());
}

#[test]
fn two_dirty_pulls_in_one_run_each_keep_their_copy() {
    let mut repo = Repo::new();
    let first = repo.item("0001", SYNCED);
    let second = repo.item("0002", SYNCED);
    let first_dir = repo.keep("1700000000-k7mq3x-0001", &first, EDITED);
    let second_dir = repo.keep("1700000000-p2b3c4-0002", &second, EDITED);

    let review = repo.review_keeping(&BTreeSet::from([
        first_dir.clone(),
        second_dir.clone(),
    ]));

    assert_eq!(
        review,
        vec![
            KeptEdits::KeptThisRun(kept_copy(
                &repo, "0001", &first_dir, &first
            )),
            KeptEdits::KeptThisRun(kept_copy(
                &repo,
                "0002",
                &second_dir,
                &second
            )),
        ]
    );
}

#[test]
fn a_hyphenated_id_is_recovered_from_its_directory_name() {
    let mut repo = Repo::new();
    repo.scheme = WorkItemIdScheme {
        id_pattern: corpus::TRACKER_TOKEN.to_owned(),
        key: None,
    };
    let item = repo.item("ENG-42", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-ENG-42", &item, EDITED);

    assert_eq!(
        repo.review(),
        vec![KeptEdits::KeptEarlier(kept_copy(
            &repo, "ENG-42", &dir, &item
        ))]
    );
}

#[test]
fn retirement_recoveries_are_untouched_by_the_pull_sweep() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = Path::new("retirement-recovery/PP-1--ENG-1");
    repo.recovery.prepare(dir).expect("prepare");
    repo.recovery
        .write_once(dir, &item, SYNCED.as_bytes())
        .expect("copy");
    repo.recovery
        .mark_restore_pending(dir, std::slice::from_ref(&item))
        .expect("record");

    assert_eq!(repo.sweep(), Vec::new());
    assert!(repo.location(dir).is_dir());
}

#[test]
fn an_unparseable_directory_with_an_equal_copy_is_never_swept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("not-a-label", &item, SYNCED);

    let swept = repo.sweep();

    assert_eq!(
        swept,
        vec![KeptEdits::Unrecorded {
            id: None,
            directory: repo.location(&dir),
            provenance: Provenance::Unknown,
        }]
    );
    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_directory_whose_copy_was_deleted_is_swept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, EDITED);
    fs::remove_dir_all(repo.location(&dir).join("meta"))
        .expect("the copy is deleted");

    assert_eq!(repo.sweep(), Vec::new());
    assert!(!repo.location(&dir).exists());
}

#[test]
fn a_copy_missing_directory_with_another_entry_is_kept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, EDITED);
    fs::remove_file(repo.copy_location(&dir, &item)).expect("deleted");
    fs::write(repo.location(&dir).join("notes.md"), "kept by hand")
        .expect("another entry");

    repo.sweep();

    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_copy_at_a_different_mirror_path_is_kept() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = Path::new("sync-recovery/1700000000-k7mq3x-0001");
    repo.recovery.prepare(dir).expect("prepare");
    let elsewhere = repo.location(dir).join("spelled/differently.md");
    fs::create_dir_all(elsewhere.parent().unwrap()).expect("dirs");
    fs::write(&elsewhere, EDITED).expect("the copy");
    repo.recovery
        .mark_copy_pending(&corpus::store::RecordedCopy {
            dir: dir.to_path_buf(),
            original: item.clone(),
            copy: PathBuf::from("spelled/differently.md"),
        })
        .expect("record");

    let swept = repo.sweep();

    assert_eq!(
        swept,
        vec![KeptEdits::KeptEarlier(KeptCopy {
            copy: elsewhere,
            ..kept_copy(&repo, "0001", dir, &item)
        })]
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_copy_is_unrecorded_and_never_read() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, SYNCED);
    let copy = repo.copy_location(&dir, &item);
    fs::remove_file(&copy).expect("the copy");
    std::os::unix::fs::symlink(&item, &copy).expect("a link");

    let swept = repo.sweep();

    assert_eq!(
        swept,
        vec![KeptEdits::Unrecorded {
            id: Some("0001".to_owned()),
            directory: repo.location(&dir),
            provenance: Provenance::Unknown,
        }]
    );
    assert!(repo.location(&dir).is_dir());
}

#[cfg(unix)]
#[test]
fn a_symlinked_recovery_directory_is_unrecorded() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let outside = TempDir::new().expect("outside");
    let dir = Path::new("sync-recovery/1700000000-k7mq3x-0001");
    fs::create_dir_all(repo.location(Path::new("sync-recovery")))
        .expect("parent");
    std::os::unix::fs::symlink(outside.path(), repo.location(dir))
        .expect("a link");
    drop(item);

    assert_eq!(
        repo.sweep(),
        vec![KeptEdits::Unrecorded {
            id: Some("0001".to_owned()),
            directory: repo.location(dir),
            provenance: Provenance::Unknown,
        }]
    );
    assert!(outside.path().is_dir());
}

#[cfg(unix)]
#[test]
fn a_symlinked_record_is_unrecorded() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = repo.keep("1700000000-k7mq3x-0001", &item, SYNCED);
    let record = repo.location(&dir).join("RESTORE-PENDING");
    let elsewhere = repo.root.path().join("record");
    fs::rename(&record, &elsewhere).expect("moved");
    std::os::unix::fs::symlink(&elsewhere, &record).expect("a link");

    assert!(matches!(
        repo.sweep().as_slice(),
        [KeptEdits::Unrecorded { .. }]
    ));
    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_committed_recovery_directory_is_never_offered_for_restore() {
    for (tracking, provenance) in [
        (FileTracking::Tracked, Provenance::Tracked),
        (FileTracking::Unknown, Provenance::TrackingUnknown),
    ] {
        let mut repo = Repo::new();
        let item = repo.item("0001", SYNCED);
        let dir = repo.keep("1700000000-k7mq3x-0001", &item, SYNCED);
        repo.tracking.answer(repo.location(&dir), tracking);

        let swept = repo.sweep();

        assert_eq!(
            swept,
            vec![KeptEdits::Unrecorded {
                id: Some("0001".to_owned()),
                directory: repo.location(&dir),
                provenance,
            }]
        );
        assert!(repo.location(&dir).is_dir());
    }
}

#[test]
fn a_record_naming_a_path_outside_the_work_directory_is_unrecorded() {
    let mut repo = Repo::new();
    repo.item("0001", SYNCED);
    let outside = repo.root.path().join("elsewhere.md");
    fs::write(&outside, SYNCED).expect("outside");
    let dir = repo.keep("1700000000-k7mq3x-0001", &outside, SYNCED);

    assert_eq!(
        repo.sweep(),
        vec![KeptEdits::Unrecorded {
            id: Some("0001".to_owned()),
            directory: repo.location(&dir),
            provenance: Provenance::Unknown,
        }]
    );
    assert!(repo.location(&dir).is_dir());
}

#[test]
fn a_copy_without_a_record_is_matched_to_its_item_by_its_mirrored_path() {
    let mut repo = Repo::new();
    let item = repo.item("0001", SYNCED);
    let dir = Path::new("sync-recovery/1700000000-k7mq3x-0001");
    repo.recovery.prepare(dir).expect("prepare");
    repo.recovery
        .write_once(dir, &item, SYNCED.as_bytes())
        .expect("copy");

    assert_eq!(
        repo.sweep(),
        vec![KeptEdits::KeptEarlier(kept_copy(&repo, "0001", dir, &item))]
    );
    assert!(repo.location(dir).is_dir());
}

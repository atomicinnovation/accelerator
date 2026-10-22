//! Apply ordering, the induced-crash re-run, and post-overwrite hashing.
//!
//! One double plays tracker, reader and writer at once, so every call lands
//! in a single ordered log, and retains written bytes so a second run can
//! read back what the first wrote.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unimplemented,
    clippy::panic
)]

mod support;

use corpus_adapters::CommonMarkBody;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use corpus::lock::ExclusiveLock;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::CopyComparison;
use corpus::store::FileRemove;
use corpus::store::RecordedCopy;
use corpus::store::RecoveryCopies;
use corpus::store::RecoveryEntry;
use corpus::store::StoreError;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::PlainDialect;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;
use work::sync::Dirtiness;
use work_adapters::sync::apply::ApplierPorts;
use work_adapters::sync::apply::ApplyError;
use work_adapters::sync::apply::FailureClass;
use work_adapters::sync::apply::ItemApplier;
use work_adapters::sync::apply::PullAttempt;
use work_adapters::sync::apply::PullRequest;
use work_adapters::sync::apply::PushRequest;
use work_adapters::sync::apply::RecordRequest;
use work_adapters::sync::apply::RetitleRequest;
use work_adapters::sync::apply::UnchangedSinceSync;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::files::SyncFiles;
use work_adapters::sync::recovery_label::RecoveryLabel;

use crate::support::assert_recipe_one_entry;

type TestError = Box<dyn std::error::Error>;

#[derive(Default)]
struct Fake {
    calls: Arc<Mutex<Vec<String>>>,
    refuse_lock: RefCell<bool>,
    dirtiness: RefCell<Option<Dirtiness>>,
    tracking: RefCell<Option<FileTracking>>,
    fail_recovery_at: RefCell<Option<&'static str>>,
    rewrite_on_copy_recorded: RefCell<Option<(PathBuf, String)>>,
    files: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
    fail_write_to: RefCell<Option<PathBuf>>,
    refuse_write_to: RefCell<Option<PathBuf>>,
    show_result: RefCell<Option<RemoteIssue>>,
    show_fails: RefCell<bool>,
    update_fails: RefCell<Option<TrackerError>>,
}

impl Fake {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("the call log").clone()
    }

    fn record(&self, call: String) {
        self.calls.lock().expect("the call log").push(call);
    }

    fn refuse_locks(&self) {
        *self.refuse_lock.borrow_mut() = true;
    }

    fn report_status(&self, dirtiness: Dirtiness) {
        *self.dirtiness.borrow_mut() = Some(dirtiness);
    }

    fn report_tracking(&self, tracking: FileTracking) {
        *self.tracking.borrow_mut() = Some(tracking);
    }

    fn fail_recovery_at(&self, step: &'static str) {
        *self.fail_recovery_at.borrow_mut() = Some(step);
    }

    fn rewrite_on_copy_recorded(&self, path: &Path, content: &str) {
        *self.rewrite_on_copy_recorded.borrow_mut() =
            Some((path.to_path_buf(), content.to_owned()));
    }

    fn recovery_calls(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|call| {
                ["prepare:", "write_once:", "mark_copy_pending:"]
                    .iter()
                    .any(|step| call.starts_with(step))
            })
            .collect()
    }

    fn recovery_step(&self, step: &'static str) -> Result<(), StoreError> {
        if *self.fail_recovery_at.borrow() == Some(step) {
            return Err(StoreError::Io {
                path: "the recovery directory".to_owned(),
                detail: "injected recovery failure".to_owned(),
            });
        }
        Ok(())
    }

    fn content(&self, path: &Path) -> Option<String> {
        self.files
            .borrow()
            .get(path)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    fn refuse_next_write_to(&self, path: &Path) {
        *self.refuse_write_to.borrow_mut() = Some(path.to_path_buf());
    }

    fn fail_next_write_to(&self, path: &Path) {
        *self.fail_write_to.borrow_mut() = Some(path.to_path_buf());
    }

    fn set_show_result(&self, issue: RemoteIssue) {
        *self.show_result.borrow_mut() = Some(issue);
    }

    fn fail_show(&self) {
        *self.show_fails.borrow_mut() = true;
    }

    fn fail_update(&self, error: TrackerError) {
        *self.update_fails.borrow_mut() = Some(error);
    }
}

impl FileReader for Fake {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        self.record(format!("read:{}", path.display()));
        Ok(self.content(path))
    }
}

impl AtomicWrite for Fake {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.record(format!("write:{}", path.display()));
        if self.refuse_write_to.borrow().as_deref() == Some(path) {
            *self.refuse_write_to.borrow_mut() = None;
            return Err(StoreError::UnsafePath {
                path: path.display().to_string(),
            });
        }
        if self.fail_write_to.borrow().as_deref() == Some(path) {
            *self.fail_write_to.borrow_mut() = None;
            return Err(StoreError::Io {
                path: path.display().to_string(),
                detail: "injected failure".to_owned(),
            });
        }
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }
}

impl FileRemove for Fake {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        self.record(format!("remove:{}", path.display()));
        self.files.borrow_mut().remove(path);
        Ok(())
    }
}

struct Release {
    calls: Arc<Mutex<Vec<String>>>,
    name: String,
}

impl Drop for Release {
    fn drop(&mut self) {
        self.calls
            .lock()
            .expect("the call log")
            .push(format!("released:{}", self.name));
    }
}

impl ExclusiveLock for Fake {
    fn acquire(&self, name: &LockName) -> Result<HeldLock, StoreError> {
        let LockName::ForFile(path) = name else {
            unimplemented!("a pull takes only its file's lock")
        };
        if *self.refuse_lock.borrow() {
            return Err(StoreError::LockTimeout {
                path: format!("{}.lockdir", path.display()),
            });
        }
        let name = path.display().to_string();
        self.record(format!("acquired:{name}"));
        Ok(HeldLock::new(Release {
            calls: Arc::clone(&self.calls),
            name,
        }))
    }

    fn clear_abandoned(&self, _name: &LockName) {}
}

impl WorkingCopyStatus for Fake {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        self.dirtiness.borrow().unwrap_or(Dirtiness::Clean)
    }
}

impl RepositoryTracking for Fake {
    fn file_tracking(&self, _path: &Path) -> FileTracking {
        self.tracking.borrow().unwrap_or(FileTracking::Tracked)
    }

    fn repository_roots(&self, _directory: &Path) -> RootsAnswer {
        unimplemented!("a pull asks only whether its file is tracked")
    }
}

impl RecoveryCopies for Fake {
    fn exists(&self, _dir: &Path) -> bool {
        unimplemented!("a pull never asks whether a copy exists")
    }

    fn prepare(&self, dir: &Path) -> Result<(), StoreError> {
        self.record(format!("prepare:{}", dir.display()));
        self.recovery_step("prepare")
    }

    fn write_once(
        &self,
        dir: &Path,
        original: &Path,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        self.record(format!(
            "write_once:{}:{}:{}",
            dir.display(),
            original.display(),
            String::from_utf8_lossy(bytes)
        ));
        self.recovery_step("write_once")
    }

    fn mark_restore_pending(
        &self,
        _dir: &Path,
        _unrestored: &[PathBuf],
    ) -> Result<(), StoreError> {
        unimplemented!("a pull records its copy, not a retirement's paths")
    }

    fn is_restore_pending(&self, _dir: &Path) -> bool {
        unimplemented!("a pull never reads a restore-pending record")
    }

    fn mark_completed(&self, _dir: &Path) -> Result<(), StoreError> {
        unimplemented!("a pull never completes a recovery")
    }

    fn remove_dir(&self, _dir: &Path) -> Result<(), StoreError> {
        unimplemented!("a pull never removes a recovery directory")
    }

    fn kept(
        &self,
        _parent: &Path,
    ) -> Result<Vec<corpus::store::KeptRecovery>, StoreError> {
        unimplemented!("a pull never lists kept copies")
    }

    fn all_entries(
        &self,
        _parent: &Path,
    ) -> Result<Vec<RecoveryEntry>, StoreError> {
        unimplemented!("a pull never lists recovery entries")
    }

    fn location(&self, _path: &Path) -> PathBuf {
        unimplemented!("a pull never locates a recovery entry")
    }

    fn mirrored_copy(&self, dir: &Path, original: &Path) -> RecordedCopy {
        RecordedCopy {
            dir: dir.to_path_buf(),
            original: original.to_path_buf(),
            copy: PathBuf::from("0001.md"),
        }
    }

    fn mark_copy_pending(&self, copy: &RecordedCopy) -> Result<(), StoreError> {
        self.record(format!(
            "mark_copy_pending:{}:{}",
            copy.dir.display(),
            copy.copy.display()
        ));
        if let Some((path, content)) =
            self.rewrite_on_copy_recorded.borrow().as_ref()
        {
            std::fs::write(path, content).expect("rewrite the item file");
        }
        self.recovery_step("mark_copy_pending")
    }

    fn recorded_copy(&self, _dir: &Path) -> Option<RecordedCopy> {
        unimplemented!("a pull never reads a copy's record")
    }

    fn compare_copy(&self, _copy: &RecordedCopy) -> CopyComparison {
        unimplemented!("a pull never compares a copy")
    }
}

impl tracker::BodyDialect for Fake {
    fn is_wire_document(&self, _body: &str) -> bool {
        false
    }
}

impl RemoteTracker for Fake {
    fn create(
        &self,
        _title: &str,
        _body: &str,
        _kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        unimplemented!("not exercised by the apply suite")
    }

    fn update(
        &self,
        id: &ExternalId,
        _title: Option<&str>,
        _body: &str,
    ) -> Result<(), TrackerError> {
        self.record(format!("update:{id}"));
        self.update_fails.borrow().clone().map_or(Ok(()), Err)
    }

    fn retitle(
        &self,
        id: &ExternalId,
        title: &str,
    ) -> Result<(), TrackerError> {
        self.record(format!("retitle:{id}:{title}"));
        self.update_fails.borrow().clone().map_or(Ok(()), Err)
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        self.record(format!("show:{id}"));
        if *self.show_fails.borrow() {
            return Err(TrackerError::Retryable {
                detail: "injected show failure".to_owned(),
            });
        }
        self.show_result.borrow().clone().ok_or_else(|| {
            TrackerError::Retryable {
                detail: "no show result configured".to_owned(),
            }
        })
    }

    fn locate(
        &self,
        id: &ExternalId,
    ) -> Result<tracker::Located, TrackerError> {
        self.show(id).map(tracker::Located::Found)
    }

    fn fetch_all(
        &self,
        _ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError> {
        unimplemented!("not exercised by the apply suite")
    }

    fn search(
        &self,
        _scope: &tracker::SearchScope,
    ) -> Result<tracker::Discovery, TrackerError> {
        unimplemented!("not exercised by the apply suite")
    }

    fn resolve_scope(
        &self,
        scope: &tracker::SearchScope,
    ) -> Result<tracker::SearchScope, tracker::ScopeError> {
        Ok(scope.clone())
    }

    fn enumerate_visible_entities(
        &self,
    ) -> Result<Vec<tracker::VisibleEntity>, tracker::TrackerError> {
        Ok(Vec::new())
    }

    fn preview_create(
        &self,
        _kind: &str,
    ) -> Result<tracker::CreatePreview, TrackerError> {
        unimplemented!("not exercised by the apply suite")
    }

    fn validate_update(
        &self,
        _id: &ExternalId,
        _title: Option<&str>,
        _body: Option<&str>,
    ) -> tracker::ValidationOutcome {
        unimplemented!("not exercised by the apply suite")
    }
}

fn ports(fake: &Fake) -> ApplierPorts<'_> {
    ApplierPorts {
        tracker: fake,
        parser: &CommonMarkBody,
        status: fake,
        files: SyncFiles {
            file_locks: fake,
            recovery: fake,
            tracking: fake,
            ..support::sync_files(fake)
        },
    }
}

const BASELINE_PATH: &str = "/baseline/last-sync.json";
const CLASSIFIED: &str = "---\nstatus: draft\n---\n\nOld body\n";

const fn item_content() -> &'static str {
    "---\nstatus: ready\nexternal_id: \"ENG-1\"\n---\n\nBody text\n"
}

/// `pull`'s guard and `record_baseline` read the local file through real
/// `std::fs`, unlike every write and baseline access, so their tests need a
/// file on disk.
fn real_item_file(dir: &tempfile::TempDir, content: &str) -> PathBuf {
    let path = dir.path().join("0001.md");
    std::fs::write(&path, content).expect("seed the real item file");
    path
}

#[test]
fn push_writes_the_baseline_strictly_after_the_tracker_update(
) -> Result<(), TestError> {
    let fake = Fake::default();
    fake.set_show_result(RemoteIssue {
        key: ExternalId::new("ENG-1".to_owned()),
        updated: RemoteTimestamp::Reported("2026-06-01T00:00:00Z".to_owned()),
        body: "Title\nProjected body\n".to_owned(),
    });

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let external_id = ExternalId::new("ENG-1".to_owned());
    {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier.push(&PushRequest {
            id: "0001",
            external_id: &external_id,
            title: Some("Title"),
            body: "Body text\n",
            sent_content: item_content(),
        })?;
    }

    let calls = fake.calls();
    let update_index = calls
        .iter()
        .position(|call| call.starts_with("update:"))
        .expect("update was called");
    let baseline_write_index = calls
        .iter()
        .position(|call| call == &format!("write:{BASELINE_PATH}"))
        .expect("baseline was written");
    assert!(update_index < baseline_write_index);

    let (baseline, _) = store.load()?;
    let entry = baseline.get("0001").expect("entry recorded");
    assert_eq!(
        entry.stamps().unwrap().remote_updated_at,
        RemoteTimestamp::Reported("2026-06-01T00:00:00Z".to_owned())
    );
    assert_recipe_one_entry(
        Some(entry),
        &PlainDialect,
        item_content(),
        Some("Title\nProjected body\n"),
    );
    Ok(())
}

#[test]
fn a_retitle_sends_the_title_alone_and_records_the_read_back(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, item_content());
    let fake = Fake::default();
    fake.set_show_result(RemoteIssue {
        key: ExternalId::new("ENG-1".to_owned()),
        updated: RemoteTimestamp::Reported("2026-06-01T00:00:00Z".to_owned()),
        body: "Retitled\nBody text\n".to_owned(),
    });

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let external_id = ExternalId::new("ENG-1".to_owned());
    {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier.retitle(&RetitleRequest {
            id: "0001",
            external_id: &external_id,
            title: "Retitled",
            sent_content: item_content(),
        })?;
    }

    let calls = fake.calls();
    assert!(calls.contains(&"retitle:ENG-1:Retitled".to_owned()));
    assert!(!calls.iter().any(|call| call.starts_with("update:")));
    assert_eq!(std::fs::read_to_string(&item_path)?, item_content());
    let (baseline, _) = store.load()?;
    assert_recipe_one_entry(
        baseline.get("0001"),
        &PlainDialect,
        item_content(),
        Some("Retitled\nBody text\n"),
    );
    Ok(())
}

#[test]
fn a_failed_update_leaves_the_baseline_entry_unset() -> Result<(), TestError> {
    let fake = Fake::default();
    fake.fail_update(TrackerError::Retryable {
        detail: "rejected".to_owned(),
    });

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let external_id = ExternalId::new("ENG-1".to_owned());
    let result = {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier.push(&PushRequest {
            id: "0001",
            external_id: &external_id,
            title: Some("Title"),
            body: "Body text\n",
            sent_content: item_content(),
        })
    };

    assert!(result.is_err());
    let (baseline, _) = store.load()?;
    assert!(baseline.get("0001").is_none());
    assert!(!fake
        .calls()
        .iter()
        .any(|call| call == &format!("write:{BASELINE_PATH}")));
    Ok(())
}

#[test]
fn an_unconfigured_error_is_classed_unconfigured() -> Result<(), TestError> {
    let fake = Fake::default();
    fake.fail_update(TrackerError::Unconfigured {
        detail: "no team in scope carries label \"typo\"".to_owned(),
    });

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let external_id = ExternalId::new("ENG-1".to_owned());
    let error = {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier
            .push(&PushRequest {
                id: "0001",
                external_id: &external_id,
                title: Some("Title"),
                body: "Body text\n",
                sent_content: item_content(),
            })
            .expect_err("the tracker refused the push")
    };

    assert_eq!(error.class(), FailureClass::Unconfigured);
    assert!(error.to_string().contains("no team in scope carries label"));
    let (baseline, _) = store.load()?;
    assert!(baseline.get("0001").is_none());
    Ok(())
}

#[test]
fn a_failed_post_push_show_still_writes_the_entry_with_not_read(
) -> Result<(), TestError> {
    let fake = Fake::default();
    fake.fail_show();

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let external_id = ExternalId::new("ENG-1".to_owned());
    {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier.push(&PushRequest {
            id: "0001",
            external_id: &external_id,
            title: Some("Title"),
            body: "Body text\n",
            sent_content: item_content(),
        })?;
    }

    let (baseline, _) = store.load()?;
    let entry = baseline
        .get("0001")
        .expect("entry recorded despite the failed show");
    assert_eq!(
        entry.stamps().unwrap().remote_updated_at,
        RemoteTimestamp::NotRead
    );
    assert_recipe_one_entry(Some(entry), &PlainDialect, item_content(), None);
    Ok(())
}

#[test]
fn pull_writes_the_baseline_strictly_after_the_local_write(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier
            .pull(&PullRequest {
                projected_body: "Title\nProjected body\n",
                ..pull_request(&item_path)
            })
            .result?;
    }

    let calls = fake.calls();
    let item_write_index = calls
        .iter()
        .position(|call| call == &format!("write:{}", item_path.display()))
        .expect("item file was written");
    let baseline_write_index = calls
        .iter()
        .position(|call| call == &format!("write:{BASELINE_PATH}"))
        .expect("baseline was written");
    assert!(item_write_index < baseline_write_index);

    assert_eq!(fake.content(&item_path), Some(item_content().to_owned()));
    Ok(())
}

#[test]
fn pull_derives_both_hashes_from_post_overwrite_state() -> Result<(), TestError>
{
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier
            .pull(&PullRequest {
                projected_body: "Title\nNew projected body\n",
                ..pull_request(&item_path)
            })
            .result?;
    }

    let (baseline, _) = store.load()?;
    assert_recipe_one_entry(
        baseline.get("0001"),
        &PlainDialect,
        item_content(),
        Some("Title\nNew projected body\n"),
    );
    Ok(())
}

#[test]
fn a_crash_between_side_effect_and_baseline_set_is_recoverable_on_re_run(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.fail_next_write_to(Path::new(BASELINE_PATH));

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let request = PullRequest {
        projected_body: "Title\nProjected body\n",
        ..pull_request(&item_path)
    };

    let first_attempt = {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier.pull(&request).result
    };
    assert!(first_attempt.is_err(), "the injected failure must surface");
    assert_eq!(
        fake.content(&item_path),
        Some(item_content().to_owned()),
        "the side effect must have landed despite the interrupted baseline write"
    );
    assert!(
        store.load()?.0.get("0001").is_none(),
        "the baseline entry must not exist after the interrupted write"
    );

    let second_attempt = {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier.pull(&request).result
    };
    assert!(second_attempt.is_ok(), "the re-run must succeed");
    let (baseline, _) = store.load()?;
    assert!(baseline.get("0001").is_some());
    Ok(())
}

#[test]
fn finalise_advances_the_global_timestamp() -> Result<(), TestError> {
    let fake = Fake::default();
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier.finalise(1_700_000_000)?;
    }
    let (baseline, _) = store.load()?;
    assert_eq!(baseline.timestamp(), 1_700_000_000);
    Ok(())
}

#[test]
fn a_local_edit_between_classification_and_apply_is_not_absorbed_into_the_baseline(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, item_content());
    let classified =
        work_adapters::sync::baseline_digester::BaselineDigester::new(
            &PlainDialect,
            &CommonMarkBody,
        )
        .digests(item_content(), Some("Title\nBody text\n"))?;
    std::fs::write(&item_path, item_content().replace("Body", "Edited"))?;
    let fake = Fake::default();
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);

    let error = ItemApplier::new(ports(&fake), &mut store, 1_700_000_000)
        .record_baseline(&RecordRequest {
            id: "0001",
            file_path: &item_path,
            fingerprints: classified,
            remote_updated: RemoteTimestamp::Reported(
                "2026-06-01T00:00:00Z".to_owned(),
            ),
        })
        .expect_err("the edit is never recorded as synced");

    assert_eq!(error.class(), FailureClass::Retryable);
    assert!(!fake
        .calls()
        .iter()
        .any(|call| call == &format!("write:{BASELINE_PATH}")));
    Ok(())
}

#[test]
fn an_unfetched_remote_is_reported_without_naming_a_direction() {
    assert_eq!(
        ApplyError::RemoteBodyUnavailable {
            item_id: "0001".to_owned()
        }
        .to_string(),
        "0001: the remote description was not fetched this run, so nothing \
         was sent or written; re-run the sync"
    );
}

#[test]
fn a_pull_residue_refusal_names_the_command_for_its_own_key() {
    let shown = ApplyError::LocalPullResidue {
        item_id: "0001".to_owned(),
        external_id: ExternalId::new("ENG-12".to_owned()),
    }
    .to_string();

    assert!(shown.contains("`jira show ENG-12 --render-adf`"), "{shown}");
}

fn pull_through(fake: &Fake, item_path: &Path) -> ApplyError {
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), fake, fake);
    ItemApplier::new(ports(fake), &mut store, 1_700_000_000)
        .pull(&PullRequest {
            projected_body: "Title\nProjected body\n",
            ..pull_request(item_path)
        })
        .result
        .expect_err("the write fails")
}

#[test]
fn a_pull_the_store_refuses_before_writing_is_known_unapplied(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.refuse_next_write_to(&item_path);

    let error = pull_through(&fake, &item_path);

    assert_eq!(error.class(), FailureClass::WriteRefused);
    Ok(())
}

#[test]
fn a_pull_whose_write_fails_in_flight_stays_terminal() -> Result<(), TestError>
{
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.fail_next_write_to(&item_path);

    let error = pull_through(&fake, &item_path);

    assert_eq!(error.class(), FailureClass::Terminal);
    Ok(())
}

#[test]
fn a_pull_holds_its_lock_while_it_writes() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);

    ItemApplier::new(ports(&fake), &mut store, 1_700_000_000)
        .pull(&pull_request(&item_path))
        .result?;

    let item = item_path.display();
    let calls = fake.calls();
    let around_the_write: Vec<&String> = calls
        .iter()
        .filter(|call| call.ends_with(&item.to_string()))
        .collect();
    assert_eq!(
        around_the_write,
        [
            &format!("acquired:{item}"),
            &format!("write:{item}"),
            &format!("released:{item}"),
        ]
    );
    Ok(())
}

#[test]
fn a_pull_whose_lock_cannot_be_taken_writes_nothing() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.refuse_locks();
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);

    let error = ItemApplier::new(ports(&fake), &mut store, 1_700_000_000)
        .pull(&pull_request(&item_path))
        .result
        .expect_err("the lock is refused");

    let ApplyError::Io {
        operation, detail, ..
    } = &error
    else {
        panic!("a refused lock is a local failure: {error}");
    };
    assert_eq!(*operation, "lock-local");
    assert!(
        detail.contains(&format!("{}.lockdir", item_path.display())),
        "{detail}"
    );
    assert!(fake.calls().iter().all(|call| !call.starts_with("write:")));
    assert_eq!(std::fs::read_to_string(&item_path)?, CLASSIFIED);
    Ok(())
}

fn pull_request(item_path: &Path) -> PullRequest<'_> {
    PullRequest {
        id: "0001",
        file_path: item_path,
        classified_content: CLASSIFIED,
        content: item_content(),
        projected_body: "Title\nProjected body\n",
        remote_updated: RemoteTimestamp::Reported(
            "2026-06-01T00:00:00Z".to_owned(),
        ),
        unchanged_since_sync: UnchangedSinceSync::RecipeOne,
        recovery_label: Some(RecoveryLabel::new(
            1_700_000_000,
            "k7mq3x",
            "0001",
        )),
    }
}

#[test]
fn a_pull_hashes_before_it_writes() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);

    let error = ItemApplier::new(ports(&fake), &mut store, 1_700_000_000)
        .pull(&PullRequest {
            content: "---\nstatus: ready\nno closing fence\n",
            ..pull_request(&item_path)
        })
        .result
        .expect_err("the pulled content cannot be fingerprinted");

    assert!(
        matches!(
            error,
            ApplyError::Io {
                operation: "hash-local",
                ..
            }
        ),
        "{error}"
    );
    assert!(fake.calls().iter().all(|call| !call.starts_with("write:")));
    assert_eq!(std::fs::read_to_string(&item_path)?, CLASSIFIED);
    Ok(())
}

const KEPT_DIR: &str = "sync-recovery/1700000000-k7mq3x-0001";

fn pull_with(fake: &Fake, request: &PullRequest<'_>) -> PullAttempt {
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), fake, fake);
    ItemApplier::new(ports(fake), &mut store, 1_700_000_000).pull(request)
}

fn kept_a_copy_of(item_path: &Path, content: &str) -> Vec<String> {
    vec![
        format!("prepare:{KEPT_DIR}"),
        format!("write_once:{KEPT_DIR}:{}:{content}", item_path.display()),
        format!("mark_copy_pending:{KEPT_DIR}:0001.md"),
    ]
}

#[test]
fn pull_writes_through_the_work_item_store() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let (fake, work_items) = (Fake::default(), Fake::default());
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let ports = ApplierPorts {
        files: SyncFiles {
            work_items: &work_items,
            ..ports(&fake).files
        },
        ..ports(&fake)
    };

    ItemApplier::new(ports, &mut store, 1_700_000_000)
        .pull(&pull_request(&item_path))
        .result?;

    assert_eq!(
        work_items.content(&item_path).as_deref(),
        Some(item_content())
    );
    assert_eq!(fake.content(&item_path), None);
    Ok(())
}

#[test]
fn a_pull_takes_its_lock_then_keeps_a_copy_then_writes() -> Result<(), TestError>
{
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_status(Dirtiness::Dirty);

    let attempt = pull_with(&fake, &pull_request(&item_path));

    attempt.result?;
    let item = item_path.display();
    let calls: Vec<String> = fake
        .calls()
        .into_iter()
        .filter(|call| !call.contains(BASELINE_PATH))
        .collect();
    assert_eq!(
        calls,
        [
            vec![format!("acquired:{item}")],
            kept_a_copy_of(&item_path, CLASSIFIED),
            vec![format!("write:{item}"), format!("released:{item}")],
        ]
        .concat()
    );
    assert_eq!(attempt.kept_edits, Some(PathBuf::from(KEPT_DIR)));
    Ok(())
}

#[test]
fn a_pull_over_a_file_changed_since_classification_keeps_no_copy(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, "---\nstatus: draft\n---\n\nNew\n");
    let fake = Fake::default();
    fake.report_status(Dirtiness::Dirty);

    let attempt = pull_with(&fake, &pull_request(&item_path));

    assert!(matches!(
        attempt.result,
        Err(ApplyError::LocalChangedSinceClassified { .. })
    ));
    assert_eq!(fake.recovery_calls(), Vec::<String>::new());
    assert_eq!(attempt.kept_edits, None);
    Ok(())
}

#[test]
fn a_pull_over_a_file_of_unknown_status_keeps_a_recovery_copy(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_status(Dirtiness::Unknown);

    let attempt = pull_with(&fake, &pull_request(&item_path));

    attempt.result?;
    assert_eq!(
        fake.recovery_calls(),
        kept_a_copy_of(&item_path, CLASSIFIED)
    );
    Ok(())
}

#[test]
fn a_pull_over_a_vcs_clean_file_edited_since_the_last_sync_keeps_a_copy(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();

    let attempt = pull_with(
        &fake,
        &PullRequest {
            unchanged_since_sync: UnchangedSinceSync::No,
            ..pull_request(&item_path)
        },
    );

    attempt.result?;
    assert_eq!(
        fake.recovery_calls(),
        kept_a_copy_of(&item_path, CLASSIFIED)
    );
    Ok(())
}

#[test]
fn a_pull_over_a_clean_file_unchanged_since_the_last_sync_keeps_no_copy(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    for unchanged in [UnchangedSinceSync::RecipeOne, UnchangedSinceSync::Legacy]
    {
        let fake = Fake::default();

        let attempt = pull_with(
            &fake,
            &PullRequest {
                unchanged_since_sync: unchanged,
                ..pull_request(&item_path)
            },
        );

        attempt.result?;
        assert_eq!(fake.recovery_calls(), Vec::<String>::new());
        assert_eq!(attempt.kept_edits, None);
        std::fs::write(&item_path, CLASSIFIED)?;
    }
    Ok(())
}

#[test]
fn a_re_run_over_a_file_already_holding_the_target_keeps_no_copy(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, item_content());
    let fake = Fake::default();
    fake.report_status(Dirtiness::Dirty);

    let attempt = pull_with(&fake, &pull_request(&item_path));

    attempt.result?;
    assert_eq!(fake.recovery_calls(), Vec::<String>::new());
    assert!(fake
        .calls()
        .contains(&format!("write:{}", item_path.display())));
    assert!(fake.calls().contains(&format!("write:{BASELINE_PATH}")));
    Ok(())
}

#[test]
fn a_failed_recovery_copy_stops_the_pull() -> Result<(), TestError> {
    for step in ["prepare", "write_once", "mark_copy_pending"] {
        let dir = tempfile::tempdir()?;
        let item_path = real_item_file(&dir, CLASSIFIED);
        let fake = Fake::default();
        fake.report_status(Dirtiness::Dirty);
        fake.fail_recovery_at(step);

        let attempt = pull_with(&fake, &pull_request(&item_path));

        let Err(ApplyError::Io {
            operation, detail, ..
        }) = &attempt.result
        else {
            panic!("{step}: a failed copy is a local failure");
        };
        assert_eq!(*operation, "keep-edits", "{step}");
        assert!(detail.contains(KEPT_DIR), "{step}: {detail}");
        assert!(detail.contains("injected recovery failure"), "{detail}");
        assert!(
            fake.calls().iter().all(|call| !call.starts_with("write:")),
            "{step}"
        );
        assert_eq!(attempt.kept_edits, None, "{step}");
    }
    Ok(())
}

#[test]
fn an_id_unsafe_for_a_recovery_directory_stops_the_pull(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_status(Dirtiness::Dirty);

    let attempt = pull_with(
        &fake,
        &PullRequest {
            recovery_label: None,
            ..pull_request(&item_path)
        },
    );

    assert!(
        matches!(
            attempt.result,
            Err(ApplyError::Io {
                operation: "keep-edits",
                ..
            })
        ),
        "{:?}",
        attempt.result
    );
    assert!(fake.calls().iter().all(|call| !call.starts_with("write:")));
    Ok(())
}

#[test]
fn a_pull_over_a_file_of_unknown_tracking_keeps_a_copy() -> Result<(), TestError>
{
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_tracking(FileTracking::Unknown);

    let attempt = pull_with(&fake, &pull_request(&item_path));

    attempt.result?;
    assert_eq!(
        fake.recovery_calls(),
        kept_a_copy_of(&item_path, CLASSIFIED)
    );
    Ok(())
}

#[test]
fn a_pull_over_an_ignored_file_edited_since_the_last_sync_keeps_a_copy(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_tracking(FileTracking::Untracked);

    let attempt = pull_with(
        &fake,
        &PullRequest {
            unchanged_since_sync: UnchangedSinceSync::No,
            ..pull_request(&item_path)
        },
    );

    attempt.result?;
    assert_eq!(
        fake.recovery_calls(),
        kept_a_copy_of(&item_path, CLASSIFIED)
    );
    Ok(())
}

#[test]
fn an_untracked_file_unchanged_since_the_last_sync_keeps_no_copy(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_tracking(FileTracking::Untracked);
    fake.report_status(Dirtiness::Dirty);

    let attempt = pull_with(&fake, &pull_request(&item_path));

    attempt.result?;
    assert_eq!(fake.recovery_calls(), Vec::<String>::new());
    Ok(())
}

#[test]
fn an_untracked_file_under_a_legacy_entry_keeps_a_copy() -> Result<(), TestError>
{
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_tracking(FileTracking::Untracked);

    let attempt = pull_with(
        &fake,
        &PullRequest {
            unchanged_since_sync: UnchangedSinceSync::Legacy,
            ..pull_request(&item_path)
        },
    );

    attempt.result?;
    assert_eq!(
        fake.recovery_calls(),
        kept_a_copy_of(&item_path, CLASSIFIED)
    );
    Ok(())
}

#[test]
fn a_file_edited_while_its_copy_was_kept_is_not_pulled_over(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_status(Dirtiness::Dirty);
    let concurrent = "---\nstatus: draft\n---\n\nConcurrent edit\n";
    fake.rewrite_on_copy_recorded(&item_path, concurrent);

    let attempt = pull_with(&fake, &pull_request(&item_path));

    assert!(matches!(
        attempt.result,
        Err(ApplyError::LocalChangedSinceClassified { .. })
    ));
    assert_eq!(std::fs::read_to_string(&item_path)?, concurrent);
    assert!(fake.calls().iter().all(|call| !call.starts_with("write:")));
    assert_eq!(attempt.kept_edits, Some(PathBuf::from(KEPT_DIR)));
    Ok(())
}

#[test]
fn a_pull_failing_after_its_copy_still_reports_kept_edits(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, CLASSIFIED);
    let fake = Fake::default();
    fake.report_status(Dirtiness::Dirty);
    fake.fail_next_write_to(&item_path);

    let attempt = pull_with(&fake, &pull_request(&item_path));

    assert!(attempt.result.is_err());
    assert_eq!(attempt.kept_edits, Some(PathBuf::from(KEPT_DIR)));
    Ok(())
}

#[test]
fn tracker_failures_keep_their_named_class() {
    let detail = || "injected".to_owned();
    let kinds = [
        (
            TrackerError::Retryable { detail: detail() },
            FailureClass::Retryable,
        ),
        (
            TrackerError::Terminal { detail: detail() },
            FailureClass::Terminal,
        ),
        (
            TrackerError::Unconfigured { detail: detail() },
            FailureClass::Unconfigured,
        ),
        (
            TrackerError::Rejected { detail: detail() },
            FailureClass::Rejected,
        ),
    ];
    for operation in ["show", "update", "retitle", "create"] {
        for (source, class) in kinds.clone() {
            let error = ApplyError::Tracker {
                item_id: "0001".to_owned(),
                operation,
                source,
            };
            assert_eq!(error.class(), class, "{operation}");
        }
    }
}

#[test]
fn each_failure_class_has_its_report_keyword() {
    let keywords = [
        (FailureClass::Retryable, "retryable"),
        (FailureClass::Terminal, "terminal"),
        (FailureClass::Unconfigured, "unconfigured"),
        (FailureClass::Rejected, "rejected"),
        (FailureClass::Unrenderable, "unrenderable"),
        (FailureClass::PullResidue, "pull-residue"),
        (FailureClass::Unparseable, "unparseable"),
        (FailureClass::WriteRefused, "local-write-refused"),
    ];
    for (class, keyword) in keywords {
        assert_eq!(class.keyword(), keyword);
    }
}

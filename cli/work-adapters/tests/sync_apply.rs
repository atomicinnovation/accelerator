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
use corpus::store::StoreError;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::PlainDialect;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use work_adapters::sync::apply::ApplierPorts;
use work_adapters::sync::apply::ApplyError;
use work_adapters::sync::apply::FailureClass;
use work_adapters::sync::apply::ItemApplier;
use work_adapters::sync::apply::PullRequest;
use work_adapters::sync::apply::PushRequest;
use work_adapters::sync::apply::RecordRequest;
use work_adapters::sync::apply::RetitleRequest;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::files::SyncFiles;

use crate::support::assert_recipe_one_entry;

type TestError = Box<dyn std::error::Error>;

#[derive(Default)]
struct Fake {
    calls: Arc<Mutex<Vec<String>>>,
    refuse_lock: RefCell<bool>,
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
        files: SyncFiles {
            file_locks: fake,
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
        applier.pull(&PullRequest {
            id: "0001",
            file_path: &item_path,
            classified_content: CLASSIFIED,
            content: item_content(),
            projected_body: "Title\nProjected body\n",
            remote_updated: RemoteTimestamp::Reported(
                "2026-06-01T00:00:00Z".to_owned(),
            ),
        })?;
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
        applier.pull(&PullRequest {
            id: "0001",
            file_path: &item_path,
            classified_content: CLASSIFIED,
            content: item_content(),
            projected_body: "Title\nNew projected body\n",
            remote_updated: RemoteTimestamp::Reported(
                "2026-06-01T00:00:00Z".to_owned(),
            ),
        })?;
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
        id: "0001",
        file_path: &item_path,
        classified_content: CLASSIFIED,
        content: item_content(),
        projected_body: "Title\nProjected body\n",
        remote_updated: RemoteTimestamp::Reported(
            "2026-06-01T00:00:00Z".to_owned(),
        ),
    };

    let first_attempt = {
        let mut applier =
            ItemApplier::new(ports(&fake), &mut store, 1_700_000_000);
        applier.pull(&request)
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
        applier.pull(&request)
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
            id: "0001",
            file_path: item_path,
            classified_content: CLASSIFIED,
            content: item_content(),
            projected_body: "Title\nProjected body\n",
            remote_updated: RemoteTimestamp::Reported(
                "2026-06-01T00:00:00Z".to_owned(),
            ),
        })
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
        .pull(&pull_request(&item_path))?;

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

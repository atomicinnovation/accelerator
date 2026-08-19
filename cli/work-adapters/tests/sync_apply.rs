//! Apply ordering, the induced-crash re-run, and post-overwrite hashing.
//!
//! One double plays tracker, reader and writer at once, so every call lands
//! in a single ordered log, and retains written bytes so a second run can
//! read back what the first wrote.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::unimplemented)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use work_adapters::sync::apply::ItemApplier;
use work_adapters::sync::apply::PullRequest;
use work_adapters::sync::apply::PushRequest;
use work_adapters::sync::baseline_store::BaselineStore;

type TestError = Box<dyn std::error::Error>;

#[derive(Default)]
struct Fake {
    calls: RefCell<Vec<String>>,
    files: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
    fail_write_to: RefCell<Option<PathBuf>>,
    show_result: RefCell<Option<RemoteIssue>>,
    show_fails: RefCell<bool>,
    update_fails: RefCell<Option<TrackerError>>,
}

impl Fake {
    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    fn seed_file(&self, path: &Path, content: &str) {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), content.as_bytes().to_vec());
    }

    fn content(&self, path: &Path) -> Option<String> {
        self.files
            .borrow()
            .get(path)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
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
        self.calls
            .borrow_mut()
            .push(format!("read:{}", path.display()));
        Ok(self.content(path))
    }
}

impl AtomicWrite for Fake {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.calls
            .borrow_mut()
            .push(format!("write:{}", path.display()));
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
        _title: &str,
        _body: &str,
    ) -> Result<(), TrackerError> {
        self.calls.borrow_mut().push(format!("update:{id}"));
        self.update_fails.borrow().clone().map_or(Ok(()), Err)
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        self.calls.borrow_mut().push(format!("show:{id}"));
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

    fn preview_create(
        &self,
        _kind: &str,
    ) -> Result<tracker::CreatePreview, TrackerError> {
        unimplemented!("not exercised by the apply suite")
    }

    fn validate_update(
        &self,
        _id: &ExternalId,
        _title: &str,
        _body: &str,
    ) -> tracker::ValidationOutcome {
        unimplemented!("not exercised by the apply suite")
    }
}

const BASELINE_PATH: &str = "/baseline/last-sync.json";
const ITEM_PATH: &str = "/items/0001.md";

const fn item_content() -> &'static str {
    "---\nstatus: ready\nexternal_id: \"ENG-1\"\n---\n\nBody text\n"
}

/// `push`'s local-file read goes through real `std::fs`, unlike `pull`'s
/// write and every baseline access, so push tests need a file on disk.
fn real_item_file(dir: &tempfile::TempDir, content: &str) -> PathBuf {
    let path = dir.path().join("0001.md");
    std::fs::write(&path, content).expect("seed the real item file");
    path
}

#[test]
fn push_writes_the_baseline_strictly_after_the_tracker_update(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, item_content());
    let fake = Fake::default();
    fake.set_show_result(RemoteIssue {
        updated: RemoteTimestamp::Reported("2026-06-01T00:00:00Z".to_owned()),
        body: "Title\nProjected body\n".to_owned(),
    });

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let external_id = ExternalId::new("ENG-1".to_owned());
    {
        let mut applier = ItemApplier::new(&fake, &fake, &mut store);
        applier.push(&PushRequest {
            id: "0001",
            external_id: &external_id,
            title: "Title",
            body: "Body text\n",
            file_path: &item_path,
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
        entry.remote_updated_at,
        RemoteTimestamp::Reported("2026-06-01T00:00:00Z".to_owned())
    );
    assert!(!entry.remote_hash.is_empty());
    assert!(!entry.local_hash.is_empty());
    Ok(())
}

#[test]
fn a_failed_update_leaves_the_baseline_entry_unset() -> Result<(), TestError> {
    let fake = Fake::default();
    fake.seed_file(Path::new(ITEM_PATH), item_content());
    fake.fail_update(TrackerError::Retryable {
        detail: "rejected".to_owned(),
    });

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let external_id = ExternalId::new("ENG-1".to_owned());
    let result = {
        let mut applier = ItemApplier::new(&fake, &fake, &mut store);
        applier.push(&PushRequest {
            id: "0001",
            external_id: &external_id,
            title: "Title",
            body: "Body text\n",
            file_path: Path::new(ITEM_PATH),
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
fn a_failed_post_push_show_still_writes_the_entry_with_not_read(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let item_path = real_item_file(&dir, item_content());
    let fake = Fake::default();
    fake.fail_show();

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let external_id = ExternalId::new("ENG-1".to_owned());
    {
        let mut applier = ItemApplier::new(&fake, &fake, &mut store);
        applier.push(&PushRequest {
            id: "0001",
            external_id: &external_id,
            title: "Title",
            body: "Body text\n",
            file_path: &item_path,
        })?;
    }

    let (baseline, _) = store.load()?;
    let entry = baseline
        .get("0001")
        .expect("entry recorded despite the failed show");
    assert_eq!(entry.remote_updated_at, RemoteTimestamp::NotRead);
    assert_eq!(entry.remote_hash, "");
    assert!(!entry.local_hash.is_empty());
    Ok(())
}

#[test]
fn pull_writes_the_baseline_strictly_after_the_local_write(
) -> Result<(), TestError> {
    let fake = Fake::default();
    fake.seed_file(
        Path::new(ITEM_PATH),
        "---\nstatus: draft\n---\n\nOld body\n",
    );

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    {
        let mut applier = ItemApplier::new(&fake, &fake, &mut store);
        applier.pull(&PullRequest {
            id: "0001",
            file_path: Path::new(ITEM_PATH),
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
        .position(|call| call == &format!("write:{ITEM_PATH}"))
        .expect("item file was written");
    let baseline_write_index = calls
        .iter()
        .position(|call| call == &format!("write:{BASELINE_PATH}"))
        .expect("baseline was written");
    assert!(item_write_index < baseline_write_index);

    assert_eq!(
        fake.content(Path::new(ITEM_PATH)),
        Some(item_content().to_owned())
    );
    Ok(())
}

#[test]
fn pull_derives_both_hashes_from_post_overwrite_state() -> Result<(), TestError>
{
    let fake = Fake::default();
    fake.seed_file(
        Path::new(ITEM_PATH),
        "---\nstatus: draft\n---\n\nOld body\n",
    );

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    {
        let mut applier = ItemApplier::new(&fake, &fake, &mut store);
        applier.pull(&PullRequest {
            id: "0001",
            file_path: Path::new(ITEM_PATH),
            content: item_content(),
            projected_body: "Title\nNew projected body\n",
            remote_updated: RemoteTimestamp::Reported(
                "2026-06-01T00:00:00Z".to_owned(),
            ),
        })?;
    }

    let (baseline, _) = store.load()?;
    let entry = baseline.get("0001").expect("entry recorded");
    let expected_local = work_adapters::sync::digest::local(item_content())?;
    let expected_remote =
        work_adapters::sync::digest::remote_body("Title\nNew projected body\n");
    assert_eq!(entry.local_hash, expected_local);
    assert_eq!(entry.remote_hash, expected_remote);
    Ok(())
}

#[test]
fn a_crash_between_side_effect_and_baseline_set_is_recoverable_on_re_run(
) -> Result<(), TestError> {
    let fake = Fake::default();
    fake.seed_file(
        Path::new(ITEM_PATH),
        "---\nstatus: draft\n---\n\nOld body\n",
    );
    fake.fail_next_write_to(Path::new(BASELINE_PATH));

    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &fake, &fake);
    let request = PullRequest {
        id: "0001",
        file_path: Path::new(ITEM_PATH),
        content: item_content(),
        projected_body: "Title\nProjected body\n",
        remote_updated: RemoteTimestamp::Reported(
            "2026-06-01T00:00:00Z".to_owned(),
        ),
    };

    let first_attempt = {
        let mut applier = ItemApplier::new(&fake, &fake, &mut store);
        applier.pull(&request)
    };
    assert!(first_attempt.is_err(), "the injected failure must surface");
    assert_eq!(
        fake.content(Path::new(ITEM_PATH)),
        Some(item_content().to_owned()),
        "the side effect must have landed despite the interrupted baseline write"
    );
    assert!(
        store.load()?.0.get("0001").is_none(),
        "the baseline entry must not exist after the interrupted write"
    );

    let second_attempt = {
        let mut applier = ItemApplier::new(&fake, &fake, &mut store);
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
        let mut applier = ItemApplier::new(&fake, &fake, &mut store);
        applier.finalise(1_700_000_000)?;
    }
    let (baseline, _) = store.load()?;
    assert_eq!(baseline.timestamp(), 1_700_000_000);
    Ok(())
}

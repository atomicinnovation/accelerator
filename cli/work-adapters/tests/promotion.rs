//! Promoting a draft over a real corpus on disk and a recording tracker:
//! the remote issue is created once, whatever stage an earlier promotion
//! stopped at, and the draft is retired to its key.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::Cell;
use std::fs;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use corpus::store::ExclusiveCreate;
use corpus::store::RemoveFile;
use corpus::StoreError;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::FileRecoveryCopies;
use corpus_adapters::LockdirLock;
use corpus_adapters::RealFs;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use store::lock::LockOptions;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker as _;
use tracker::TrackerError;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use work::dirtiness::Dirtiness;
use work::draft_id::DraftId;
use work::promotion::NotPromoted;
use work::promotion::Promotion;
use work::promotion::PromotionMode;
use work::promotion::PromotionRecord;
use work::promotion::PromotionStage;
use work::promotion::RemoteHash;
use work::retirement::RetirementFailure;
use work::retirement::RetirementRefusal;
use work::sync::RequestFingerprint;
use work::sync::SyncState;
use work_adapters::promotion::promote;
use work_adapters::promotion::Detail;
use work_adapters::promotion::DetailSource;
use work_adapters::promotion::PromotionPorts;
use work_adapters::promotion::PromotionRow;
use work_adapters::promotion_records::FilePromotionRecords;
use work_adapters::promotion_records::PromotionRecords as _;
use work_adapters::promotion_records::StoredRecord;
use work_adapters::retirement::CorpusLayout;
use work_adapters::retirement::RetirementFiles;
use work_adapters::retirement::RetirementPorts;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::digest;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::pending_push;

const DRAFT_ID: &str = "draft-k7mq3x";
const DRAFT: &str = "meta/work/drafts/draft-k7mq3x-add-search.md";
const KEY: &str = "REC-1";
const TARGET: &str = "meta/work/REC-1-add-search.md";
const CHILD: &str = "meta/work/0001-child.md";
const PLAN: &str = "meta/plans/plan.md";
const OUTSIDE: &str = "README.md";
const INTEGRATIONS: &str = ".accelerator/state/integrations";
const BASELINE: &str = ".accelerator/state/integrations/linear/last-sync.json";

const DRAFT_CONTENT: &str = "---\nid: \"draft-k7mq3x\"\ntitle: \"Add search\"\n\
                             kind: \"story\"\n---\n\n# draft-k7mq3x: Add search\n\n\
                             Supersedes nothing; see draft-k7mq3x.\n";
const CHILD_CONTENT: &str = "---\nid: \"0001\"\nparent: \
                             \"work-item:draft-k7mq3x\"\n---\n\n# 0001: Child\n";
const PLAN_CONTENT: &str = "See draft-k7mq3x.\n";
const OUTSIDE_CONTENT: &str = "Mentions draft-k7mq3x.\n";

const FAST: LockOptions = LockOptions {
    ceiling_ms: 1,
    base_ms: 1,
    cap_ms: 1,
};

type TestError = Box<dyn std::error::Error>;

struct Clean;

impl WorkingCopyStatus for Clean {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        Dirtiness::Clean
    }
}

/// A real store that simulates the process dying at its `die_at`th
/// mutating operation, counted from zero across record writes and every
/// retirement step, by panicking before the operation lands; or fails each
/// operation whose index is in `fail_at`.
struct Store {
    inner: FileCorpusStore,
    count: Cell<usize>,
    die_at: Option<usize>,
    fail_at: Vec<usize>,
}

impl Store {
    fn new(root: &Path) -> Self {
        Self {
            inner: FileCorpusStore::new(root),
            count: Cell::new(0),
            die_at: None,
            fail_at: Vec::new(),
        }
    }

    fn check(&self, path: &Path) -> Result<(), StoreError> {
        let index = self.count.get();
        self.count.set(index + 1);
        assert_ne!(self.die_at, Some(index), "killed at operation {index}");
        if self.fail_at.contains(&index) {
            return Err(StoreError::Io {
                path: path.display().to_string(),
                detail: format!("injected failure at operation {index}"),
            });
        }
        Ok(())
    }
}

impl AtomicWrite for Store {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.check(path)?;
        self.inner.write(path, bytes)
    }
}

impl ExclusiveCreate for Store {
    fn create_new(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.check(path)?;
        self.inner.create_new(path, bytes)
    }
}

impl RemoveFile for Store {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        self.check(path)?;
        self.inner.remove(path)
    }
}

struct Repo {
    dir: tempfile::TempDir,
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
            (OUTSIDE, OUTSIDE_CONTENT),
        ] {
            repo.write(path, content)?;
        }
        fs::create_dir_all(repo.path(BASELINE).parent().ok_or("no parent")?)?;
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

    fn baseline_entry(&self, id: &str) -> Option<Entry> {
        let store = FileCorpusStore::new(self.root());
        let baseline = BaselineStore::new(self.path(BASELINE), &RealFs, &store);
        let (document, _) = baseline.load().expect("baseline loads");
        document.get(id).cloned()
    }

    fn records<'a>(
        &self,
        writer: &'a dyn AtomicWrite,
    ) -> FilePromotionRecords<'a> {
        FilePromotionRecords::new(&self.path(INTEGRATIONS), "linear", writer)
    }

    fn stored(&self) -> StoredRecord {
        let store = FileCorpusStore::new(self.root());
        self.records(&store).read(&draft())
    }

    fn save(&self, record: &PromotionRecord) {
        let store = FileCorpusStore::new(self.root());
        self.records(&store).save(record).expect("record saved");
    }

    fn record_path(&self) -> PathBuf {
        pending_push::record_path(&self.path(INTEGRATIONS), "linear", &draft())
    }
}

fn draft() -> DraftId {
    DraftId::parse(DRAFT_ID).expect("a draft id")
}

fn key() -> ExternalId {
    ExternalId::new(KEY.to_owned())
}

/// Promotes the draft once over `store`, as one process would.
fn promote_with(
    repo: &Repo,
    tracker: &RecordingTracker,
    store: &Store,
) -> Result<Promotion, NotPromoted> {
    promote_in(repo, tracker, store, &PromotionMode::Standard)
}

fn promote_in(
    repo: &Repo,
    tracker: &RecordingTracker,
    store: &Store,
    mode: &PromotionMode,
) -> Result<Promotion, NotPromoted> {
    let roots = vec![repo.path("meta")];
    let work_dir = repo.path("meta/work");
    let locks = LockdirLock::with_options(&work_dir, FAST);
    let recovery = FileRecoveryCopies::new(
        repo.path(".accelerator/state"),
        repo.path("meta"),
    );
    let baseline = BaselineStore::new(repo.path(BASELINE), &RealFs, store);
    let retirement = RetirementPorts {
        files: RetirementFiles {
            reader: &RealFs,
            writer: store,
            creator: store,
            remover: store,
            file_locks: &locks,
            recovery: &recovery,
        },
        layout: CorpusLayout {
            roots: &roots,
            work_dir: &work_dir,
        },
        walker: &RealFs,
        status: &Clean,
        lock: &locks,
        baseline: &baseline,
    };
    let records = repo.records(store);
    promote(
        &draft(),
        mode,
        &PromotionPorts {
            tracker,
            retirement: &retirement,
            records: &records,
        },
    )
}

/// Promotes the draft and reports it as a sync or `work promote` would.
fn reported(repo: &Repo, tracker: &RecordingTracker) -> PromotionRow {
    let store = Store::new(repo.root());
    let roots = vec![repo.path("meta")];
    let work_dir = repo.path("meta/work");
    let locks = LockdirLock::with_options(&work_dir, FAST);
    let state = repo.path(".accelerator/state");
    let recovery = FileRecoveryCopies::new(&state, repo.path("meta"));
    let baseline = BaselineStore::new(repo.path(BASELINE), &RealFs, &store);
    let retirement = RetirementPorts {
        files: RetirementFiles {
            reader: &RealFs,
            writer: &store,
            creator: &store,
            remover: &store,
            file_locks: &locks,
            recovery: &recovery,
        },
        layout: CorpusLayout {
            roots: &roots,
            work_dir: &work_dir,
        },
        walker: &RealFs,
        status: &Clean,
        lock: &locks,
        baseline: &baseline,
    };
    let records = repo.records(&store);
    let ports = PromotionPorts {
        tracker,
        retirement: &retirement,
        records: &records,
    };
    let result = promote(&draft(), &PromotionMode::Standard, &ports);
    PromotionRow::of(&draft(), repo.path(DRAFT), result, &ports, &state)
}

fn adopted(
    repo: &Repo,
    tracker: &RecordingTracker,
) -> Result<Promotion, NotPromoted> {
    promote_in(
        repo,
        tracker,
        &Store::new(repo.root()),
        &PromotionMode::Adopt(key()),
    )
}

fn promoted(
    repo: &Repo,
    tracker: &RecordingTracker,
) -> Result<Promotion, NotPromoted> {
    promote_with(repo, tracker, &Store::new(repo.root()))
}

fn creates(tracker: &RecordingTracker) -> usize {
    tracker
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::Create { .. }))
        .count()
}

fn updates(tracker: &RecordingTracker) -> Vec<(String, String)> {
    tracker
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Update { title, body, .. } => Some((title, body)),
            _ => None,
        })
        .collect()
}

const fn fresh_tracker() -> RecordingTracker {
    RecordingTracker::holding(Vec::new())
}

/// A tracker already holding the draft's issue under `KEY` with `body`,
/// as a create an earlier run sent left it.
fn tracker_holding(body: &str) -> RecordingTracker {
    RecordingTracker::holding(vec![(
        key(),
        RemoteIssue {
            key: key(),
            updated: RemoteTimestamp::Reported("t0".to_owned()),
            body: body.to_owned(),
        },
    )])
}

fn draft_projection() -> String {
    let (_, body) = digest::split_frontmatter_and_body(DRAFT_CONTENT).unwrap();
    format!("Add search\n{body}")
}

fn record_at(stage: PromotionStage) -> PromotionRecord {
    let (_, body) = digest::split_frontmatter_and_body(DRAFT_CONTENT).unwrap();
    PromotionRecord {
        draft_id: draft(),
        request: RequestFingerprint {
            title: "Add search".to_owned(),
            digest: pending_push::request_digest("Add search", &body, "story"),
            attempted_at: 1,
            failure: None,
        },
        content_digest: pending_push::content_digest(
            "Add search",
            &body,
            "story",
            DRAFT_ID,
        ),
        stage,
    }
}

fn assert_promoted(repo: &Repo) {
    assert_eq!(repo.read(DRAFT), None, "the draft is retired");
    let target = repo.read(TARGET).expect("the item is at the key path");
    assert!(target.contains("id: \"REC-1\""), "{target}");
    assert!(target.contains("external_id: \"REC-1\""), "{target}");
    assert!(target.contains("aliases: [\"draft-k7mq3x\"]"), "{target}");
    assert!(target.contains("\n# REC-1: Add search\n"), "{target}");
    assert!(!repo.record_path().exists(), "no record is left");
}

#[test]
fn promotion_creates_the_issue_moves_the_file_and_rewrites_meta() {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker();

    assert_eq!(
        promoted(&repo, &tracker),
        Ok(Promotion::Completed(key(), SyncState::Synced))
    );

    assert_eq!(creates(&tracker), 1);
    assert_promoted(&repo);
    let child = repo.read(CHILD).unwrap();
    assert!(child.contains("parent: \"work-item:REC-1\""), "{child}");
    assert_eq!(repo.read(PLAN).unwrap(), "See REC-1.\n");
    let target = repo.read(TARGET).unwrap();
    assert!(!target.contains("see draft-k7mq3x."), "{target}");
}

#[test]
fn a_title_the_frontmatter_escapes_reaches_the_tracker_as_written() {
    let repo = Repo::new().unwrap();
    let title = r#"Say "hi" to C:\temp"#;
    let mut frontmatter = Mapping::new();
    for (field, value) in
        [("id", DRAFT_ID), ("title", title), ("kind", "story")]
    {
        frontmatter
            .push(field.to_owned(), Yaml::Scalar(Scalar::String(value.into())));
    }
    let draft = document::render(None, &Yaml::Mapping(frontmatter)).unwrap()
        + "\n# draft-k7mq3x: Say hi\n";
    repo.write(DRAFT, &draft).unwrap();
    let tracker = fresh_tracker();

    promoted(&repo, &tracker).unwrap();

    let sent: Vec<String> = tracker
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Create { title, .. } => Some(title),
            _ => None,
        })
        .collect();
    assert_eq!(sent, [title]);
}

#[test]
fn a_draft_id_outside_meta_is_untouched() {
    let repo = Repo::new().unwrap();

    promoted(&repo, &fresh_tracker()).unwrap();

    assert_eq!(repo.read(OUTSIDE).unwrap(), OUTSIDE_CONTENT);
}

#[test]
fn the_remote_update_carries_rewritten_self_references() {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker();

    promoted(&repo, &tracker).unwrap();

    let sent = updates(&tracker);
    assert_eq!(sent.len(), 1);
    let (title, body) = &sent[0];
    assert_eq!(title, "Add search");
    assert!(body.contains("# REC-1: Add search"), "{body}");
    assert!(body.contains("see REC-1."), "{body}");
    assert!(!body.contains(DRAFT_ID), "{body}");
}

#[test]
fn promotion_writes_a_baseline_under_the_tracker_key() {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker();

    promoted(&repo, &tracker).unwrap();

    let entry = repo.baseline_entry(KEY).expect("a baseline under the key");
    let promoted_file = repo.read(TARGET).unwrap();
    assert_eq!(entry.local_hash, digest::local(&promoted_file).unwrap());
    let remote = tracker.show(&key()).unwrap();
    assert_eq!(entry.remote_hash, digest::remote_body(&remote.body));
    assert_eq!(repo.baseline_entry(DRAFT_ID), None);
}

#[test]
fn a_local_save_leaves_the_draft_unchanged_with_no_marker() {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker().failing_create(TrackerError::Retryable {
        detail: "connection refused".to_owned(),
    });

    assert_eq!(
        promoted(&repo, &tracker),
        Err(NotPromoted::TrackerUnreachable)
    );

    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
    assert_eq!(repo.stored(), StoredRecord::Absent);
}

#[test]
fn an_unknown_create_outcome_stays_unknown_when_its_failure_cannot_be_noted() {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker().failing_create(TrackerError::Terminal {
        detail: "response lost".to_owned(),
    });
    let attempted_is_noted_again = 1;
    let mut store = Store::new(repo.root());
    store.fail_at = vec![attempted_is_noted_again];

    assert_eq!(
        promote_with(&repo, &tracker, &store),
        Err(NotPromoted::CreateOutcomeUnknown)
    );

    let StoredRecord::Present(record) = repo.stored() else {
        panic!("the attempted record written before sending stands");
    };
    assert_eq!(record.stage, PromotionStage::Attempted);
}

#[test]
fn a_tracker_error_leaves_the_draft_unchanged_with_an_attempted_marker() {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker().failing_create(TrackerError::Terminal {
        detail: "response lost".to_owned(),
    });

    assert_eq!(
        promoted(&repo, &tracker),
        Err(NotPromoted::CreateOutcomeUnknown)
    );

    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
    let StoredRecord::Present(record) = repo.stored() else {
        panic!("an attempted record names the draft");
    };
    assert_eq!(record.stage, PromotionStage::Attempted);
    assert_eq!(record.request.failure.as_deref(), Some("response lost"));
}

#[test]
fn a_rejected_request_leaves_the_draft_unchanged_with_no_marker_and_its_cause()
{
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker().failing_create(TrackerError::Rejected {
        detail: "a table".to_owned(),
    });

    assert_eq!(
        promoted(&repo, &tracker),
        Err(NotPromoted::RequestRejected {
            detail: "a table".to_owned()
        })
    );

    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
    assert_eq!(repo.stored(), StoredRecord::Absent);
}

#[test]
fn an_attempted_marker_stops_promotion_naming_the_possible_duplicate() {
    let repo = Repo::new().unwrap();
    repo.save(&record_at(PromotionStage::Attempted));
    let tracker = fresh_tracker();

    assert_eq!(
        promoted(&repo, &tracker),
        Err(NotPromoted::EarlierAttemptUnconfirmed)
    );
    assert_eq!(creates(&tracker), 0);
    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
}

#[test]
fn an_attempted_marker_survives_retitling_and_does_not_block_a_same_titled_draft(
) {
    let repo = Repo::new().unwrap();
    repo.save(&record_at(PromotionStage::Attempted));
    repo.write(DRAFT, &DRAFT_CONTENT.replace("Add search", "Find things"))
        .unwrap();
    let twin = "meta/work/drafts/draft-p2r9zz-add-search.md";
    repo.write(twin, &DRAFT_CONTENT.replace(DRAFT_ID, "draft-p2r9zz"))
        .unwrap();
    let tracker = fresh_tracker();

    assert_eq!(
        promoted(&repo, &tracker),
        Err(NotPromoted::EarlierAttemptUnconfirmed),
        "the retitled draft is still stopped"
    );
    assert_eq!(creates(&tracker), 0);

    let store = Store::new(repo.root());
    let roots = vec![repo.path("meta")];
    let work_dir = repo.path("meta/work");
    let locks = LockdirLock::with_options(&work_dir, FAST);
    let recovery = FileRecoveryCopies::new(
        repo.path(".accelerator/state"),
        repo.path("meta"),
    );
    let baseline = BaselineStore::new(repo.path(BASELINE), &RealFs, &store);
    let retirement = RetirementPorts {
        files: RetirementFiles {
            reader: &RealFs,
            writer: &store,
            creator: &store,
            remover: &store,
            file_locks: &locks,
            recovery: &recovery,
        },
        layout: CorpusLayout {
            roots: &roots,
            work_dir: &work_dir,
        },
        walker: &RealFs,
        status: &Clean,
        lock: &locks,
        baseline: &baseline,
    };
    let records = repo.records(&store);
    let twin_promoted = promote(
        &DraftId::parse("draft-p2r9zz").unwrap(),
        &PromotionMode::Standard,
        &PromotionPorts {
            tracker: &tracker,
            retirement: &retirement,
            records: &records,
        },
    );

    assert_eq!(
        twin_promoted,
        Ok(Promotion::Completed(key(), SyncState::Synced))
    );
}

#[test]
fn an_unreadable_record_blocks_a_matching_tracker_create() {
    let repo = Repo::new().unwrap();
    fs::create_dir_all(repo.record_path().parent().unwrap()).unwrap();
    fs::write(repo.record_path(), "{\"kind\":").unwrap();
    let tracker = fresh_tracker();

    assert_eq!(
        promoted(&repo, &tracker),
        Err(NotPromoted::CreateOutcomeUnknown)
    );
    assert_eq!(creates(&tracker), 0);
}

#[test]
fn a_created_marker_is_adopted_without_a_new_issue_even_after_edits() {
    let repo = Repo::new().unwrap();
    let tracker = tracker_holding(&draft_projection());
    repo.save(&record_at(PromotionStage::Created {
        key: key(),
        created_remote_hash: Some(digest::remote_body(&draft_projection())),
    }));
    let edited = DRAFT_CONTENT.replace("Supersedes nothing", "Supersedes all");
    repo.write(DRAFT, &edited).unwrap();

    assert_eq!(
        promoted(&repo, &tracker),
        Ok(Promotion::Completed(key(), SyncState::Synced))
    );

    assert_eq!(creates(&tracker), 0);
    assert_promoted(&repo);
    assert!(repo.read(TARGET).unwrap().contains("Supersedes all"));
}

#[test]
fn an_untouched_recorded_issue_gets_the_tracker_key_h1_on_adoption() {
    let repo = Repo::new().unwrap();
    let tracker = tracker_holding(&draft_projection());
    repo.save(&record_at(PromotionStage::Created {
        key: key(),
        created_remote_hash: Some(digest::remote_body(&draft_projection())),
    }));

    promoted(&repo, &tracker).unwrap();

    let remote = tracker.show(&key()).unwrap();
    assert!(
        remote.body.contains("# REC-1: Add search"),
        "{}",
        remote.body
    );
}

#[test]
fn an_edited_recorded_issue_is_not_rewritten_and_the_next_sync_raises_a_conflict(
) {
    let repo = Repo::new().unwrap();
    let tracker = tracker_holding("Add search\nSomeone rewrote this.\n");
    repo.save(&record_at(PromotionStage::Created {
        key: key(),
        created_remote_hash: Some(digest::remote_body(&draft_projection())),
    }));

    promoted(&repo, &tracker).unwrap();

    assert!(
        updates(&tracker).is_empty(),
        "the edit is never overwritten"
    );
    let entry = repo.baseline_entry(KEY).unwrap();
    assert_eq!(entry.remote_hash, "", "the remote reads as changed");
    assert_ne!(
        entry.local_hash,
        digest::local(&repo.read(TARGET).unwrap()).unwrap(),
        "the local reads as changed, so the two conflict"
    );
}

#[test]
fn a_kill_between_the_h1_update_and_its_record_resumes_as_retitled_not_edited()
{
    let repo = Repo::new().unwrap();
    let (_, body) = digest::split_frontmatter_and_body(DRAFT_CONTENT).unwrap();
    let retitled = format!("Add search\n{}", body.replace(DRAFT_ID, KEY));
    let tracker = tracker_holding(&retitled);
    repo.save(&record_at(PromotionStage::Created {
        key: key(),
        created_remote_hash: Some(digest::remote_body(&draft_projection())),
    }));

    promoted(&repo, &tracker).unwrap();

    let entry = repo.baseline_entry(KEY).unwrap();
    assert_eq!(
        entry.local_hash,
        digest::local(&repo.read(TARGET).unwrap()).unwrap(),
        "the tool's own update is recognised, so the item syncs as synced"
    );
    assert_eq!(entry.remote_hash, digest::remote_body(&retitled));
}

#[test]
fn a_failed_h1_update_records_a_baseline_that_makes_the_next_sync_push() {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker().failing_update(
        ExternalId::new("REC-1".to_owned()),
        TrackerError::Retryable {
            detail: "refused".to_owned(),
        },
    );

    assert_eq!(
        promoted(&repo, &tracker),
        Ok(Promotion::Completed(key(), SyncState::LocallyModified))
    );

    let entry = repo.baseline_entry(KEY).unwrap();
    let remote = tracker.show(&key()).unwrap();
    assert_eq!(entry.remote_hash, digest::remote_body(&remote.body));
    assert_eq!(entry.local_hash, digest::local(DRAFT_CONTENT).unwrap());
}

#[test]
fn a_failed_read_back_leaves_the_record_at_created_and_reports_created_unwritten(
) {
    let repo = Repo::new().unwrap();
    let tracker = RecordingTracker::holding(Vec::new()).failing_show(
        key(),
        TrackerError::Retryable {
            detail: "refused".to_owned(),
        },
    );

    assert_eq!(
        promoted(&repo, &tracker),
        Err(NotPromoted::ReadBackFailed(key()))
    );

    let StoredRecord::Present(record) = repo.stored() else {
        panic!("the record holds the key");
    };
    assert_eq!(
        record.stage,
        PromotionStage::Created {
            key: key(),
            created_remote_hash: None
        }
    );
    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
}

#[test]
fn a_retirement_failure_rolls_back_and_leaves_a_promotion_record_holding_the_key(
) {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker();
    let mut store = Store::new(repo.root());
    store.fail_at = vec![4, 5];

    let result = promote_with(&repo, &tracker, &store);

    assert!(
        matches!(
            result,
            Err(NotPromoted::RetirementFailed(
                RetirementFailure::RolledBack { .. }
            ))
        ),
        "{result:?}"
    );
    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
    assert_eq!(repo.read(CHILD).unwrap(), CHILD_CONTENT);
    let StoredRecord::Present(record) = repo.stored() else {
        panic!("the record holds the key");
    };
    assert_eq!(record.stage.key(), Some(&key()));
    assert!(
        !matches!(record.stage, PromotionStage::Retiring { .. }),
        "rewound: {:?}",
        record.stage
    );

    assert_eq!(
        promoted(&repo, &tracker),
        Ok(Promotion::Completed(key(), SyncState::Synced)),
        "the_next_promotion_adopts_it_without_a_new_issue"
    );
    assert_eq!(creates(&tracker), 1);
    assert_promoted(&repo);
}

#[test]
fn a_refused_retirement_rewinds_the_record_to_before() {
    let repo = Repo::new().unwrap();
    repo.write(TARGET, "stray\n").unwrap();
    let tracker = fresh_tracker();

    let result = promoted(&repo, &tracker);

    assert!(
        matches!(
            result,
            Err(NotPromoted::Refused(RetirementRefusal::TargetExists(_)))
        ),
        "{result:?}"
    );
    let StoredRecord::Present(record) = repo.stored() else {
        panic!("the record holds the key");
    };
    assert!(matches!(
        record.stage,
        PromotionStage::RemoteRetitled { .. }
    ));
    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
}

#[test]
fn a_collision_keeps_the_marker_and_reports_it() {
    let repo = Repo::new().unwrap();
    repo.write(
        "meta/work/0002-holder.md",
        "---\nid: \"0002\"\naliases: [\"REC-1\"]\n---\n\n# 0002: Holder\n",
    )
    .unwrap();

    let result = promoted(&repo, &fresh_tracker());

    assert!(
        matches!(
            &result,
            Err(NotPromoted::Refused(RetirementRefusal::IdTaken { holder, .. }))
                if holder.ends_with("0002-holder.md")
        ),
        "{result:?}"
    );
    assert!(matches!(repo.stored(), StoredRecord::Present(_)));
}

#[test]
fn a_created_key_already_linked_by_a_legacy_item_is_refused_naming_both() {
    let repo = Repo::new().unwrap();
    repo.write(
        "meta/work/0002-legacy.md",
        "---\nid: \"0002\"\nexternal_id: \"REC-1\"\n---\n\n# 0002: Legacy\n",
    )
    .unwrap();

    let result = promoted(&repo, &fresh_tracker());

    assert!(
        matches!(
            &result,
            Err(NotPromoted::Refused(RetirementRefusal::KeyLinked { holder }))
                if holder.ends_with("0002-legacy.md")
        ),
        "{result:?}"
    );
}

#[test]
fn a_retiring_record_with_nothing_applied_is_retired_not_discarded() {
    let repo = Repo::new().unwrap();
    let tracker = tracker_holding(&draft_projection());
    let before = PromotionStage::RemoteKept {
        key: key(),
        reason: work::promotion::RemoteKeptReason::NoHash {
            read_back: work::promotion::ReadBack {
                hash: digest::remote_body(&draft_projection()),
                updated: RemoteTimestamp::Reported("t0".to_owned()),
            },
        },
    };
    repo.save(&record_at(PromotionStage::Retiring {
        key: key(),
        baseline: work::promotion::IntendedBaseline {
            remote_hash: RemoteHash::Unknown,
            local_hash: "draft".to_owned(),
        },
        recovery_dir: PathBuf::from("retirement-recovery/draft-k7mq3x--REC-1"),
        before: Box::new(before),
    }));

    assert_eq!(
        promoted(&repo, &tracker),
        Ok(Promotion::Completed(key(), SyncState::Conflict))
    );

    assert_eq!(creates(&tracker), 0);
    assert_promoted(&repo);
    assert_eq!(repo.baseline_entry(KEY).unwrap().local_hash, "draft");
}

#[test]
fn an_already_promoted_draft_finishes_its_outstanding_record_first() {
    let repo = Repo::new().unwrap();
    let tracker = fresh_tracker();
    let mut store = Store::new(repo.root());
    store.die_at = Some(8);
    let killed = std::panic::catch_unwind(AssertUnwindSafe(|| {
        promote_with(&repo, &tracker, &store)
    }));
    assert!(killed.is_err(), "killed while recording the baseline");
    assert_eq!(repo.read(DRAFT), None, "the retirement had landed");

    assert_eq!(promoted(&repo, &tracker), Ok(Promotion::AlreadyDone(key())));
    assert!(
        repo.baseline_entry(KEY).is_some(),
        "the baseline is recorded"
    );
    assert!(!repo.record_path().exists());
}

#[test]
fn a_promotion_killed_at_each_stage_boundary_finishes_on_the_next_promote() {
    for die_at in 0..=10 {
        let repo = Repo::new().unwrap();
        let tracker = fresh_tracker();
        let mut store = Store::new(repo.root());
        store.die_at = Some(die_at);
        let first = std::panic::catch_unwind(AssertUnwindSafe(|| {
            promote_with(&repo, &tracker, &store)
        }));

        let resumed = promoted(&repo, &tracker);

        if die_at == 1 {
            assert_eq!(
                resumed,
                Err(NotPromoted::EarlierAttemptUnconfirmed),
                "killed between sending the create and recording its key"
            );
            continue;
        }
        assert!(
            matches!(
                resumed,
                Ok(Promotion::Completed(..) | Promotion::AlreadyDone(_))
            ),
            "killed at {die_at} ({first:?}): {resumed:?}"
        );
        assert_eq!(creates(&tracker), 1, "killed at {die_at}");
        assert_promoted(&repo);
        assert!(repo.baseline_entry(KEY).is_some(), "killed at {die_at}");
    }
}

#[test]
fn a_draft_in_the_canonical_directory_is_promoted_as_a_draft() {
    let repo = Repo::new().unwrap();
    fs::remove_file(repo.path(DRAFT)).unwrap();
    repo.write("meta/work/draft-k7mq3x-add-search.md", DRAFT_CONTENT)
        .unwrap();

    assert_eq!(
        promoted(&repo, &fresh_tracker()),
        Ok(Promotion::Completed(key(), SyncState::Synced))
    );
    assert_eq!(repo.read("meta/work/draft-k7mq3x-add-search.md"), None);
    assert_promoted(&repo);
}

#[test]
fn a_draft_edited_during_promotion_is_replanned_and_keeps_the_edit() {
    let repo = Repo::new().unwrap();
    let mut tracker = fresh_tracker();
    let park = tracker.parking_create();

    std::thread::scope(|scope| {
        let repo = &repo;
        let promoting = scope.spawn(move || promoted(repo, &tracker));
        park.wait_parked();
        repo.write(DRAFT, &format!("{DRAFT_CONTENT}\nAn afterthought.\n"))
            .unwrap();
        park.release();
        assert_eq!(
            promoting.join().unwrap(),
            Ok(Promotion::Completed(key(), SyncState::Synced))
        );
    });

    assert!(repo.read(TARGET).unwrap().contains("An afterthought."));
}

#[test]
fn two_promoters_of_one_draft_create_one_issue() {
    let repo = Repo::new().unwrap();
    let mut first_tracker = fresh_tracker();
    let park = first_tracker.parking_create();
    let second_tracker = fresh_tracker();

    let created = std::thread::scope(|scope| {
        let repo = &repo;
        let first = scope.spawn(move || {
            let result = promoted(repo, &first_tracker);
            (result, creates(&first_tracker))
        });
        park.wait_parked();
        let second = scope.spawn(move || {
            let result = promoted_waiting(repo, &second_tracker);
            (result, creates(&second_tracker))
        });
        std::thread::sleep(std::time::Duration::from_millis(100));
        park.release();

        let (first, first_creates) = first.join().unwrap();
        let (second, second_creates) = second.join().unwrap();
        assert_eq!(first, Ok(Promotion::Completed(key(), SyncState::Synced)));
        assert_eq!(second, Ok(Promotion::AlreadyDone(key())));
        first_creates + second_creates
    });

    assert_eq!(created, 1);
    assert_promoted(&repo);
}

/// Promotes with the production lock timeout, so a promoter waits for
/// another holding the retirement lock rather than timing out.
fn promoted_waiting(
    repo: &Repo,
    tracker: &RecordingTracker,
) -> Result<Promotion, NotPromoted> {
    let store = Store::new(repo.root());
    let roots = vec![repo.path("meta")];
    let work_dir = repo.path("meta/work");
    let locks = LockdirLock::new(&work_dir);
    let recovery = FileRecoveryCopies::new(
        repo.path(".accelerator/state"),
        repo.path("meta"),
    );
    let baseline = BaselineStore::new(repo.path(BASELINE), &RealFs, &store);
    let retirement = RetirementPorts {
        files: RetirementFiles {
            reader: &RealFs,
            writer: &store,
            creator: &store,
            remover: &store,
            file_locks: &locks,
            recovery: &recovery,
        },
        layout: CorpusLayout {
            roots: &roots,
            work_dir: &work_dir,
        },
        walker: &RealFs,
        status: &Clean,
        lock: &locks,
        baseline: &baseline,
    };
    let records = repo.records(&store);
    promote(
        &draft(),
        &PromotionMode::Standard,
        &PromotionPorts {
            tracker,
            retirement: &retirement,
            records: &records,
        },
    )
}

#[test]
fn adopting_a_named_key_verifies_it_then_retires_the_draft() {
    let repo = Repo::new().unwrap();
    repo.save(&record_at(PromotionStage::Attempted));
    let tracker = tracker_holding(&draft_projection());

    let result = adopted(&repo, &tracker);

    assert!(
        matches!(&result, Ok(Promotion::Completed(adopted, _)) if *adopted == key()),
        "{result:?}"
    );
    assert_eq!(creates(&tracker), 0);
    assert!(
        tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Locate { id } if *id == key())),
        "the named key is verified: {:?}",
        tracker.calls()
    );
    assert_promoted(&repo);
}

#[test]
fn a_user_named_adopt_never_rewrites_and_the_next_sync_raises_a_conflict_when_bodies_differ(
) {
    let repo = Repo::new().unwrap();
    let tracker = tracker_holding("Add search\nWritten in the tracker.\n");

    assert_eq!(
        adopted(&repo, &tracker),
        Ok(Promotion::Completed(key(), SyncState::Conflict))
    );

    assert!(updates(&tracker).is_empty(), "the issue is never rewritten");
    let entry = repo.baseline_entry(KEY).unwrap();
    assert_eq!(entry.remote_hash, "", "the remote reads as changed");
    assert_ne!(
        entry.local_hash,
        digest::local(&repo.read(TARGET).unwrap()).unwrap(),
        "the local reads as changed, so the two conflict"
    );
}

#[test]
fn a_user_named_adopt_differing_only_by_id_reports_local_changed_and_the_next_sync_pushes_the_key_h1(
) {
    let repo = Repo::new().unwrap();
    let tracker = tracker_holding(&draft_projection());

    assert_eq!(
        adopted(&repo, &tracker),
        Ok(Promotion::Completed(key(), SyncState::LocallyModified))
    );

    assert!(updates(&tracker).is_empty(), "the issue is never rewritten");
    let entry = repo.baseline_entry(KEY).unwrap();
    let remote = tracker.show(&key()).unwrap();
    assert_eq!(
        entry.remote_hash,
        digest::remote_body(&remote.body),
        "the remote reads as unchanged"
    );
    assert_ne!(
        entry.local_hash,
        digest::local(&repo.read(TARGET).unwrap()).unwrap(),
        "the local reads as changed, so the next sync pushes the key H1"
    );
}

#[test]
fn adopting_a_key_linked_by_a_legacy_item_is_refused_naming_both() {
    let repo = Repo::new().unwrap();
    repo.write(
        "meta/work/0002-legacy.md",
        "---\nid: \"0002\"\nexternal_id: \"REC-1\"\n---\n\n# 0002: Legacy\n",
    )
    .unwrap();
    let tracker = tracker_holding(&draft_projection());

    let result = adopted(&repo, &tracker);

    assert!(
        matches!(
            &result,
            Err(NotPromoted::Refused(RetirementRefusal::KeyLinked { holder }))
                if holder.ends_with("0002-legacy.md")
        ),
        "{result:?}"
    );
    assert_eq!(repo.stored(), StoredRecord::Absent, "nothing is recorded");
    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
}

#[test]
fn adopting_a_missing_issue_changes_nothing() {
    let repo = Repo::new().unwrap();
    repo.save(&record_at(PromotionStage::Attempted));

    assert_eq!(
        adopted(&repo, &fresh_tracker().not_found(&key())),
        Err(NotPromoted::AdoptedIssueMissing(key()))
    );

    assert!(matches!(
        repo.stored(),
        StoredRecord::Present(record) if record.stage == PromotionStage::Attempted
    ));
    assert_eq!(repo.read(DRAFT).unwrap(), DRAFT_CONTENT);
}

#[test]
fn adopting_replaces_an_unreadable_record() {
    let repo = Repo::new().unwrap();
    fs::create_dir_all(repo.record_path().parent().unwrap()).unwrap();
    fs::write(repo.record_path(), "{\"kind\":").unwrap();

    let result = adopted(&repo, &tracker_holding(&draft_projection()));

    assert!(matches!(result, Ok(Promotion::Completed(..))), "{result:?}");
    assert_promoted(&repo);
}

#[test]
fn creating_accepts_the_duplicate_risk_over_an_attempted_record() {
    let repo = Repo::new().unwrap();
    repo.save(&record_at(PromotionStage::Attempted));
    let tracker = fresh_tracker();

    let result = promote_in(
        &repo,
        &tracker,
        &Store::new(repo.root()),
        &PromotionMode::CreateAcceptingDuplicate,
    );

    assert_eq!(result, Ok(Promotion::Completed(key(), SyncState::Synced)));
    assert_eq!(creates(&tracker), 1);
    assert_promoted(&repo);
}

#[test]
fn an_adopt_naming_another_key_than_the_record_holds_stops() {
    let repo = Repo::new().unwrap();
    repo.save(&record_at(PromotionStage::Created {
        key: ExternalId::new("REC-7".to_owned()),
        created_remote_hash: None,
    }));

    assert_eq!(
        adopted(&repo, &tracker_holding(&draft_projection())),
        Err(NotPromoted::AdoptConflictsWithRecordedKey {
            recorded: ExternalId::new("REC-7".to_owned())
        })
    );
}

#[test]
fn each_not_promoted_row_names_its_subject() {
    let repo = Repo::new().unwrap();
    let unreachable = fresh_tracker().failing_create(TrackerError::Retryable {
        detail: "connection refused".to_owned(),
    });

    let row = reported(&repo, &unreachable);

    assert_eq!(
        row.details,
        vec![Detail {
            source: DetailSource::Item,
            path: repo.path(DRAFT),
        }]
    );
}

#[test]
fn a_collision_names_its_holder() {
    let repo = Repo::new().unwrap();
    repo.write(
        "meta/work/0002-legacy.md",
        "---\nid: \"0002\"\nexternal_id: \"REC-1\"\n---\n\n# 0002: Legacy\n",
    )
    .unwrap();

    let row = reported(&repo, &fresh_tracker());

    assert_eq!(
        row.details,
        vec![Detail {
            source: DetailSource::Holder,
            path: repo.path("meta/work/0002-legacy.md"),
        }]
    );
}

#[test]
fn a_promotion_that_will_conflict_names_the_promoted_item() {
    let repo = Repo::new().unwrap();
    let tracker = tracker_holding("Add search\nSomeone rewrote this.\n");
    repo.save(&record_at(PromotionStage::Created {
        key: key(),
        created_remote_hash: Some(digest::remote_body(&draft_projection())),
    }));

    let row = reported(&repo, &tracker);

    assert_eq!(
        row.details,
        vec![Detail {
            source: DetailSource::Item,
            path: repo.path(TARGET),
        }]
    );
}

#[test]
fn a_clean_promotion_names_nothing() {
    let repo = Repo::new().unwrap();

    assert!(reported(&repo, &fresh_tracker()).details.is_empty());
}

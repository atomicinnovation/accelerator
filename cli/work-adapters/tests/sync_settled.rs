//! A whole sync run over a real corpus on disk: the identity pass settles
//! key changes and interrupted retirements before the engine plans.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use corpus::lock::ExclusiveLock;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::store::AtomicWrite;
use corpus::store::ExclusiveCreate;
use corpus::store::RemoveFile;
use corpus::IdOwnership;
use corpus::StoreError;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::FileRecoveryCopies;
use corpus_adapters::LockdirLock;
use corpus_adapters::RealFs;
use store::lock::LockOptions;
use tracker::Ceiling;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::TrackerError;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use work::dirtiness::Dirtiness;
use work::identity::resolve_identity;
use work::identity::IdentityResolution;
use work::retirement::Retirement;
use work::retirement::RetirementRecord;
use work::sync::IdentityAction;
use work::sync::SyncDirection;
use work::sync::SyncState;
use work::work_item_files::identities;
use work::work_item_files::identity_of;
use work::work_item_files::WorkItemFiles as _;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::retirement::CorpusLayout;
use work_adapters::retirement::RetirementFiles;
use work_adapters::retirement::RetirementPorts;
use work_adapters::retirement_records::FileRetirementRecords;
use work_adapters::retirement_records::RetirementRecords;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::create::AuthoredLocal;
use work_adapters::sync::create::DiscoveredIssue;
use work_adapters::sync::create::LocalAuthor;
use work_adapters::sync::digest;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::identity_settlement::IdentityOutcome;
use work_adapters::sync::identity_settlement::IdentityRow;
use work_adapters::sync::identity_settlement::SettlementPorts;
use work_adapters::sync::run::ItemSelection;
use work_adapters::sync::run::RunError;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;
use work_adapters::sync::settled_run::run_settled;
use work_adapters::sync::settled_run::CorpusDiscovery;
use work_adapters::sync::settled_run::DiscoveredCorpus;
use work_adapters::sync::settled_run::SettledRunFailure;

const BASELINE: &str = ".accelerator/state/integrations/linear/last-sync.json";
const STATE: &str = ".accelerator/state";
const STAMP: &str = "2026-06-01T00:00:00Z";
const BODY: &str = "Title\nBody\n";

const FAST: LockOptions = LockOptions {
    ceiling_ms: 1,
    base_ms: 1,
    cap_ms: 1,
};

fn key(raw: &str) -> ExternalId {
    ExternalId::new(raw.to_owned())
}

fn held(raw: &str) -> (ExternalId, RemoteIssue) {
    (
        key(raw),
        RemoteIssue {
            key: key(raw),
            updated: RemoteTimestamp::Reported(STAMP.to_owned()),
            body: BODY.to_owned(),
        },
    )
}

struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fs::create_dir_all(repo.work_dir().join("drafts")).unwrap();
        fs::create_dir_all(repo.path(BASELINE).parent().unwrap()).unwrap();
        repo
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    fn work_dir(&self) -> PathBuf {
        self.path("meta/work")
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn read(&self, relative: &str) -> Option<String> {
        fs::read_to_string(self.path(relative)).ok()
    }

    /// A synced work item: its baseline entry matches both sides.
    fn item(&self, file: &str, id: &str, external: Option<&str>) {
        let external_line = external
            .map(|raw| format!("external_id: \"{raw}\"\n"))
            .unwrap_or_default();
        let content = format!(
            "---\nid: \"{id}\"\ntitle: \"Title\"\n{external_line}---\n\n\
             # {id}: Title\n"
        );
        self.write(&format!("meta/work/{file}"), &content);
        if external.is_some() {
            self.baseline_set(
                id,
                Entry {
                    remote_updated_at: RemoteTimestamp::Reported(
                        STAMP.to_owned(),
                    ),
                    remote_hash: digest::remote_body(BODY),
                    local_hash: digest::local(&content).unwrap(),
                    local_synced_at: 0,
                },
            );
        }
    }

    fn baseline_set(&self, id: &str, entry: Entry) {
        let store = FileCorpusStore::new(self.root());
        let mut baseline =
            BaselineStore::new(self.path(BASELINE), &RealFs, &store);
        baseline.set(id, entry).unwrap();
    }

    fn baseline_entry(&self, id: &str) -> Option<Entry> {
        let store = FileCorpusStore::new(self.root());
        let baseline = BaselineStore::new(self.path(BASELINE), &RealFs, &store);
        baseline.load().unwrap().0.get(id).cloned()
    }

    fn items(&self) -> Vec<LocalItem> {
        let files = FilesystemWorkItemFiles::new(&self.work_dir())
            .files()
            .unwrap();
        let mut items: Vec<LocalItem> = files
            .iter()
            .filter_map(identity_of)
            .map(|identity| LocalItem {
                id: identity.id,
                path: identity.path,
                external_id: identity.external_id.map(ExternalId::new),
            })
            .collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        items
    }

    fn records(&self) -> Vec<RetirementRecord> {
        let store = FileCorpusStore::new(self.root());
        FileRetirementRecords::new(&self.path(STATE), &store)
            .outstanding()
            .unwrap()
    }

    fn save_record(&self, retirement: &Retirement<'_>) {
        let store = FileCorpusStore::new(self.root());
        FileRetirementRecords::new(&self.path(STATE), &store)
            .save(&RetirementRecord::of(retirement))
            .unwrap();
    }

    fn snapshot(&self) -> BTreeMap<PathBuf, String> {
        fn walk(dir: &Path, into: &mut BTreeMap<PathBuf, String>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, into);
                } else if let Ok(content) = fs::read_to_string(&path) {
                    into.insert(path, content);
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(self.root(), &mut files);
        files
    }
}

struct EveryPath(Dirtiness);

impl WorkingCopyStatus for EveryPath {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        self.0
    }
}

struct FixedClock;

impl work::sync::RunClock for FixedClock {
    fn run_start_epoch(&self) -> Result<u64, kernel::Error> {
        Ok(1_700_000_000)
    }
}

/// Links by rewriting the `external_id` line, and records every remote
/// issue it is asked to import.
#[derive(Default)]
struct FileAuthor {
    imported: RefCell<Vec<String>>,
}

impl LocalAuthor for FileAuthor {
    fn author_from_remote(
        &self,
        issue: &DiscoveredIssue,
    ) -> Result<AuthoredLocal, kernel::Error> {
        self.imported
            .borrow_mut()
            .push(issue.external_id.to_string());
        Err(kernel::Error::Failed(
            "imports are recorded, not written".into(),
        ))
    }

    fn link_external_id(
        &self,
        path: &Path,
        external_id: &ExternalId,
    ) -> Result<(), kernel::Error> {
        let content = fs::read_to_string(path)
            .map_err(|error| kernel::Error::Failed(error.to_string()))?;
        let linked: String = content
            .lines()
            .map(|line| {
                if line.starts_with("external_id:") {
                    format!("external_id: \"{external_id}\"\n")
                } else {
                    format!("{line}\n")
                }
            })
            .collect();
        fs::write(path, linked)
            .map_err(|error| kernel::Error::Failed(error.to_string()))
    }
}

/// Delegates to a real store, failing from the `fail_at`th mutating
/// operation onwards.
struct Faulty {
    inner: FileCorpusStore,
    count: Cell<usize>,
    fail_at: Option<usize>,
    keep_failing: bool,
}

impl Faulty {
    fn check(&self, path: &Path) -> Result<(), StoreError> {
        let index = self.count.get();
        self.count.set(index + 1);
        let failing = self
            .fail_at
            .is_some_and(|at| index == at || (self.keep_failing && index > at));
        if failing {
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

type Log = Rc<RefCell<Vec<&'static str>>>;

struct LoggingLock {
    inner: LockdirLock,
    log: Log,
}

impl ExclusiveLock for LoggingLock {
    fn acquire(&self, name: &LockName) -> Result<HeldLock, StoreError> {
        if matches!(name, LockName::Retirement) {
            self.log.borrow_mut().push("lock");
        }
        self.inner.acquire(name)
    }
}

struct LoggingRecords<'a> {
    inner: FileRetirementRecords<'a>,
    log: Log,
}

impl RetirementRecords for LoggingRecords<'_> {
    fn save(&self, record: &RetirementRecord) -> Result<(), StoreError> {
        self.inner.save(record)
    }

    fn outstanding(&self) -> Result<Vec<RetirementRecord>, StoreError> {
        self.log.borrow_mut().push("read-records");
        self.inner.outstanding()
    }

    fn remove(&self, record: &RetirementRecord) -> Result<(), StoreError> {
        self.inner.remove(record)
    }
}

struct FileDiscovery<'a> {
    repo: &'a Repo,
    dirtiness: Dirtiness,
    before: Option<Box<dyn Fn()>>,
}

impl CorpusDiscovery for FileDiscovery<'_> {
    fn discover(&self) -> Result<DiscoveredCorpus, RunError> {
        if let Some(before) = &self.before {
            before();
        }
        Ok(DiscoveredCorpus {
            items: self.repo.items(),
            status: Box::new(EveryPath(self.dirtiness)),
        })
    }
}

struct Options {
    ownership: IdOwnership,
    mode: RunMode,
    strategy: RetrievalStrategy,
    max_pulls: Ceiling,
    targets: Option<Vec<&'static str>>,
    fail_at: Option<usize>,
    keep_failing: bool,
    dirtiness_after: Dirtiness,
    before_rediscovery: Option<Box<dyn Fn()>>,
    log: Option<Log>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            ownership: IdOwnership::Tracker,
            mode: RunMode::Apply,
            strategy: RetrievalStrategy::Bulk,
            max_pulls: Ceiling::Bounded(25),
            targets: None,
            fail_at: None,
            keep_failing: false,
            dirtiness_after: Dirtiness::Clean,
            before_rediscovery: None,
            log: None,
        }
    }
}

struct Outcome {
    result: Result<RunReport, SettledRunFailure>,
    imported: Vec<String>,
}

impl Outcome {
    fn report(&self) -> &RunReport {
        match &self.result {
            Ok(report) => report,
            Err(failure) => panic!("the run failed: {:?}", failure.error),
        }
    }

    fn row(&self, id: &str) -> &IdentityRow {
        self.report()
            .identity
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| {
                panic!("no identity row for {id}: {:?}", self.report().identity)
            })
    }

    fn engine_state(&self, id: &str) -> Option<SyncState> {
        self.report()
            .reported
            .iter()
            .find(|reported| reported.planned.id == id)
            .map(|reported| reported.planned.state)
    }

    fn error(&self) -> &RunError {
        match &self.result {
            Ok(_) => panic!("the run succeeded"),
            Err(failure) => &failure.error,
        }
    }
}

fn targeted(items: &[LocalItem], targets: Option<&[&str]>) -> Vec<LocalItem> {
    targets.map_or_else(Vec::new, |ids| {
        items
            .iter()
            .filter(|item| ids.contains(&item.id.as_str()))
            .cloned()
            .collect()
    })
}

fn sync(repo: &Repo, tracker: &RecordingTracker, options: Options) -> Outcome {
    let root = repo.root();
    let work_dir = repo.work_dir();
    let state = repo.path(STATE);
    let roots = vec![repo.path("meta")];
    let plain = FileCorpusStore::new(root);
    let faulty = Faulty {
        inner: FileCorpusStore::new(root),
        count: Cell::new(0),
        fail_at: options.fail_at,
        keep_failing: options.keep_failing,
    };
    let log = options.log.clone().unwrap_or_default();
    let locks = LoggingLock {
        inner: LockdirLock::with_options(&work_dir, FAST),
        log: Rc::clone(&log),
    };
    let recovery = FileRecoveryCopies::new(&state, repo.path("meta"));
    let status = EveryPath(Dirtiness::Clean);
    let retirement_baseline =
        BaselineStore::new(repo.path(BASELINE), &RealFs, &plain);
    let retirement = RetirementPorts {
        files: RetirementFiles {
            reader: &RealFs,
            writer: &faulty,
            creator: &faulty,
            remover: &faulty,
            file_locks: &locks,
            recovery: &recovery,
        },
        layout: CorpusLayout {
            roots: &roots,
            work_dir: &work_dir,
        },
        walker: &RealFs,
        status: &status,
        lock: &locks,
        baseline: &retirement_baseline,
    };
    let records = LoggingRecords {
        inner: FileRetirementRecords::new(&state, &plain),
        log,
    };
    let settlement = SettlementPorts {
        retirement: &retirement,
        records: &records,
        ownership: options.ownership,
        state_dir: &state,
    };
    let author = FileAuthor::default();
    let ports = SyncPorts {
        tracker,
        status: &status,
        writer: &plain,
        clock: &FixedClock,
        author: &author,
    };
    let mut baseline = BaselineStore::new(repo.path(BASELINE), &RealFs, &plain);
    let items = repo.items();
    let targeted = targeted(&items, options.targets.as_deref());
    let selection = if options.targets.is_some() {
        ItemSelection::Targeted {
            items: &targeted,
            pull_ids: &[],
        }
    } else {
        ItemSelection::All
    };
    let resolutions = BTreeMap::new();
    let integrations_root = repo.path(".accelerator/state/integrations");
    let request = SyncRequest {
        corpus: &items,
        selection,
        direction: SyncDirection::Bidirectional,
        strategy: options.strategy,
        resolutions: &resolutions,
        max_pulls: options.max_pulls,
        max_pushes: Ceiling::Bounded(25),
        mode: options.mode,
        integrations_root: &integrations_root,
        integration: "linear",
        scope: tracker::SearchScope::default(),
    };
    let discovery = FileDiscovery {
        repo,
        dirtiness: options.dirtiness_after,
        before: options.before_rediscovery,
    };
    let result =
        run_settled(&request, &ports, &settlement, &mut baseline, &discovery);
    let imported = author.imported.borrow().clone();
    Outcome { result, imported }
}

fn moved(from: &str, to: &str) -> RecordingTracker {
    RecordingTracker::holding(vec![held(from)]).moving(&key(from), &key(to))
}

/// `PP-760` moved to `ENG-42`, with a plan linking to it by typed link and
/// by prose.
fn moved_tracker_owned_item(repo: &Repo) {
    repo.item("PP-760-add-search.md", "PP-760", Some("PP-760"));
    repo.write(
        "meta/plans/plan.md",
        "---\nwork_item_id: \"work-item:PP-760\"\n---\n\nImplements PP-760.\n",
    );
}

fn fetch_all_calls(tracker: &RecordingTracker) -> usize {
    tracker
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::FetchAll { .. }))
        .count()
}

#[test]
fn a_moved_issue_updates_external_id_under_a_local_pattern() {
    let repo = Repo::new();
    repo.item("0230-add-search.md", "0230", Some("PP-760"));

    let outcome = sync(
        &repo,
        &moved("PP-760", "ENG-42"),
        Options {
            ownership: IdOwnership::Local,
            ..Options::default()
        },
    );

    let content = repo.read("meta/work/0230-add-search.md").unwrap();
    assert!(content.contains("external_id: \"ENG-42\""), "{content}");
    assert!(content.contains("id: \"0230\""), "{content}");
    let row = outcome.row("0230");
    assert_eq!(row.action, IdentityAction::KeyChanged);
    assert_eq!(row.detail, "PP-760->ENG-42");
    assert_eq!(row.outcome, IdentityOutcome::Applied);
}

#[test]
fn a_moved_issue_of_a_legacy_item_keeps_its_id_under_tracker_ownership() {
    let repo = Repo::new();
    repo.item("0230-add-search.md", "0230", Some("PP-760"));

    sync(&repo, &moved("PP-760", "ENG-42"), Options::default());

    let content = repo.read("meta/work/0230-add-search.md").unwrap();
    assert!(content.contains("id: \"0230\""), "{content}");
    assert!(content.contains("external_id: \"ENG-42\""), "{content}");
    assert!(!content.contains("aliases"), "{content}");
}

#[test]
fn a_moved_issue_retires_the_old_key_when_id_equals_external_id() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);

    let outcome = sync(&repo, &moved("PP-760", "ENG-42"), Options::default());

    assert_eq!(repo.read("meta/work/PP-760-add-search.md"), None);
    let retired = repo.read("meta/work/ENG-42-add-search.md").unwrap();
    assert!(retired.contains("id: \"ENG-42\""), "{retired}");
    assert!(retired.contains("external_id: \"ENG-42\""), "{retired}");
    assert!(retired.contains("# ENG-42: Title"), "{retired}");
    assert!(retired.contains("aliases: [\"PP-760\"]"), "{retired}");
    let plan = repo.read("meta/plans/plan.md").unwrap();
    assert!(plan.contains("work-item:ENG-42"), "{plan}");
    assert!(plan.contains("Implements ENG-42."), "{plan}");
    let row = outcome.row("PP-760");
    assert_eq!(row.settled_id, "ENG-42");
    assert_eq!(row.outcome, IdentityOutcome::Applied);
}

#[test]
fn a_jira_project_move_retires_the_key() {
    let repo = Repo::new();
    repo.item("PP-76-fix-login.md", "PP-76", Some("PP-76"));

    let outcome = sync(&repo, &moved("PP-76", "OPS-5"), Options::default());

    assert!(repo.read("meta/work/OPS-5-fix-login.md").is_some());
    assert_eq!(outcome.row("PP-76").detail, "PP-76->OPS-5");
}

#[test]
fn the_baseline_entry_follows_a_retired_key() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);

    sync(&repo, &moved("PP-760", "ENG-42"), Options::default());

    assert!(repo.baseline_entry("PP-760").is_none());
    assert!(repo.baseline_entry("ENG-42").is_some());
}

#[test]
fn work_resolve_pp_760_returns_the_moved_path() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);

    sync(&repo, &moved("PP-760", "ENG-42"), Options::default());

    let items = identities(
        &FilesystemWorkItemFiles::new(&repo.work_dir())
            .files()
            .unwrap(),
    );
    let IdentityResolution::Unique(item) = resolve_identity("PP-760", &items)
    else {
        panic!("the retired key resolves through its alias");
    };
    assert_eq!(item.path, repo.path("meta/work/ENG-42-add-search.md"));
}

#[test]
fn a_moved_key_is_not_imported_again_by_discovery() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    let tracker = moved("PP-760", "ENG-42").discovering(
        vec![(key("ENG-42"), RemoteTimestamp::Reported(STAMP.to_owned()))],
        true,
    );

    let outcome = sync(&repo, &tracker, Options::default());

    assert!(outcome.imported.is_empty(), "{:?}", outcome.imported);
}

#[test]
fn a_moved_issue_is_followed_under_per_item_reads() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);

    let outcome = sync(
        &repo,
        &moved("PP-760", "ENG-42"),
        Options {
            strategy: RetrievalStrategy::PerItem,
            ..Options::default()
        },
    );

    assert_eq!(outcome.row("PP-760").outcome, IdentityOutcome::Applied);
    assert!(repo.read("meta/work/ENG-42-add-search.md").is_some());
}

#[test]
fn a_show_returning_a_different_key_after_a_bulk_read_is_a_move() {
    let repo = Repo::new();
    repo.item("0230-add-search.md", "0230", Some("PP-760"));
    repo.baseline_set(
        "0230",
        Entry {
            remote_updated_at: RemoteTimestamp::Reported("stale".to_owned()),
            remote_hash: digest::remote_body(BODY),
            local_hash: String::new(),
            local_synced_at: 0,
        },
    );
    let tracker = RecordingTracker::holding(vec![held("PP-760")])
        .showing_under(&key("PP-760"), &key("ENG-42"));

    let outcome = sync(&repo, &tracker, Options::default());

    assert_eq!(outcome.row("0230").detail, "PP-760->ENG-42");
}

#[test]
fn a_show_returning_the_requested_key_in_another_case_is_not_a_move() {
    let repo = Repo::new();
    repo.item("0230-add-search.md", "0230", Some("PP-760"));
    repo.baseline_set(
        "0230",
        Entry {
            remote_updated_at: RemoteTimestamp::Reported("stale".to_owned()),
            remote_hash: digest::remote_body(BODY),
            local_hash: String::new(),
            local_synced_at: 0,
        },
    );
    let tracker = RecordingTracker::holding(vec![held("PP-760")])
        .showing_under(&key("PP-760"), &key("pp-760"));

    let outcome = sync(&repo, &tracker, Options::default());

    assert!(outcome.report().identity.is_empty());
}

#[test]
fn a_moved_key_colliding_with_another_items_alias_leaves_the_item_unchanged() {
    let repo = Repo::new();
    repo.item("PP-760-add-search.md", "PP-760", Some("PP-760"));
    repo.write(
        "meta/work/ENG-7-other.md",
        "---\nid: \"ENG-7\"\naliases: [\"ENG-42\"]\n---\n\n# ENG-7: Other\n",
    );
    let before = repo.read("meta/work/PP-760-add-search.md");

    let outcome = sync(&repo, &moved("PP-760", "ENG-42"), Options::default());

    assert_eq!(repo.read("meta/work/PP-760-add-search.md"), before);
    let row = outcome.row("PP-760");
    let IdentityOutcome::Refused(message) = &row.outcome else {
        panic!("the collision is refused: {row:?}");
    };
    assert!(message.contains("PP-760"), "{message}");
    assert!(message.contains("ENG-7-other.md"), "{message}");
    assert!(row.awaits_human());
}

#[test]
fn a_moved_key_already_linked_by_another_item_leaves_both_unchanged() {
    for ownership in [IdOwnership::Local, IdOwnership::Tracker] {
        let repo = Repo::new();
        repo.item("0230-add-search.md", "0230", Some("PP-760"));
        repo.item("0231-other.md", "0231", Some("ENG-42"));
        let tracker =
            RecordingTracker::holding(vec![held("PP-760"), held("ENG-42")])
                .moving(&key("PP-760"), &key("ENG-42"));
        let before = repo.snapshot();

        let outcome = sync(
            &repo,
            &tracker,
            Options {
                ownership,
                ..Options::default()
            },
        );

        let after = repo.snapshot();
        for file in ["meta/work/0230-add-search.md", "meta/work/0231-other.md"]
        {
            assert_eq!(
                after.get(&repo.path(file)),
                before.get(&repo.path(file)),
                "{ownership:?} {file}"
            );
        }
        assert!(
            matches!(outcome.row("0230").outcome, IdentityOutcome::Refused(_)),
            "{ownership:?}"
        );
    }
}

#[test]
fn an_issue_missing_under_its_stored_key_is_unchanged_and_warned() {
    let repo = Repo::new();
    repo.item("PP-76-fix-login.md", "PP-76", Some("PP-76"));
    let before = repo.snapshot();
    let tracker =
        RecordingTracker::holding(Vec::new()).not_found(&key("PP-76"));

    let outcome = sync(&repo, &tracker, Options::default());

    let row = outcome.row("PP-76");
    assert_eq!(row.action, IdentityAction::NotFound);
    assert_eq!(row.detail, "PP-76");
    assert!(row.awaits_human());
    assert_eq!(outcome.engine_state("PP-76"), Some(SyncState::RemoteAbsent));
    let after = repo.snapshot();
    assert_eq!(
        after.get(&repo.path("meta/work/PP-76-fix-login.md")),
        before.get(&repo.path("meta/work/PP-76-fix-login.md"))
    );
}

#[test]
fn a_failed_locate_leaves_the_item_and_corpus_unchanged() {
    let repo = Repo::new();
    repo.item("PP-76-fix-login.md", "PP-76", Some("PP-76"));
    let before = repo.read("meta/work/PP-76-fix-login.md");
    let tracker = RecordingTracker::holding(Vec::new()).failing_locate(
        &key("PP-76"),
        TrackerError::Retryable {
            detail: "down".to_owned(),
        },
    );

    let outcome = sync(&repo, &tracker, Options::default());

    assert!(outcome.report().identity.is_empty());
    assert_eq!(repo.read("meta/work/PP-76-fix-login.md"), before);
}

#[test]
fn an_indeterminate_id_is_located_and_followed() {
    let repo = Repo::new();
    repo.item("0230-add-search.md", "0230", Some("PP-760"));
    let tracker =
        RecordingTracker::truncating(vec![held("PP-760")], vec![key("PP-760")])
            .moving(&key("PP-760"), &key("ENG-42"));

    let outcome = sync(
        &repo,
        &tracker,
        Options {
            ownership: IdOwnership::Local,
            ..Options::default()
        },
    );

    assert_eq!(outcome.row("0230").outcome, IdentityOutcome::Applied);
}

#[test]
fn preview_reports_key_changes_without_writing() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    let before = repo.snapshot();

    let outcome = sync(
        &repo,
        &moved("PP-760", "ENG-42"),
        Options {
            mode: RunMode::Preview,
            ..Options::default()
        },
    );

    assert_eq!(repo.snapshot(), before);
    let row = outcome.row("PP-760");
    assert_eq!(row.detail, "PP-760->ENG-42");
    assert_eq!(row.outcome, IdentityOutcome::NotApplied);
}

#[test]
fn preview_does_not_offer_to_import_a_moved_key() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    let tracker = moved("PP-760", "ENG-42").discovering(
        vec![(key("ENG-42"), RemoteTimestamp::Reported(STAMP.to_owned()))],
        true,
    );

    let outcome = sync(
        &repo,
        &tracker,
        Options {
            mode: RunMode::Preview,
            ..Options::default()
        },
    );

    assert!(
        outcome
            .report()
            .reported
            .iter()
            .all(|reported| reported.planned.id != "ENG-42"),
        "the moved key is not offered for import"
    );
}

/// The corpus as a key-change retirement left it after writing the target
/// and before removing the old file.
fn interrupted_after_writing_to(repo: &Repo) {
    repo.item("PP-760-add-search.md", "PP-760", Some("PP-760"));
    repo.write(
        "meta/work/ENG-42-add-search.md",
        "---\nid: \"ENG-42\"\ntitle: \"Title\"\nexternal_id: \"ENG-42\"\n\
         aliases: [\"PP-760\"]\n---\n\n# ENG-42: Title\n",
    );
    repo.save_record(&Retirement {
        old_id: "PP-760",
        new_id: "ENG-42",
        new_external_id: Some("ENG-42"),
    });
}

#[test]
fn a_key_change_retirement_killed_after_writing_to_resumes() {
    let repo = Repo::new();
    interrupted_after_writing_to(&repo);

    let outcome = sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-42")]),
        Options::default(),
    );

    assert_eq!(repo.read("meta/work/PP-760-add-search.md"), None);
    assert!(repo.baseline_entry("ENG-42").is_some());
    assert!(repo.records().is_empty());
    let row = outcome.row("PP-760");
    assert_eq!(row.action, IdentityAction::Resumed);
    assert_eq!(row.detail, "PP-760->ENG-42");
}

#[test]
fn a_retirement_killed_after_removing_from_is_reconciled_on_the_next_sync() {
    let repo = Repo::new();
    interrupted_after_writing_to(&repo);
    fs::remove_file(repo.path("meta/work/PP-760-add-search.md")).unwrap();

    let outcome = sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-42")]),
        Options::default(),
    );

    assert!(repo.baseline_entry("PP-760").is_none());
    assert!(repo.baseline_entry("ENG-42").is_some());
    assert!(repo.records().is_empty());
    assert_eq!(outcome.row("PP-760").action, IdentityAction::Resumed);
}

#[test]
fn a_completed_retirement_is_not_reported_resumed() {
    let repo = Repo::new();
    repo.item("ENG-42-add-search.md", "ENG-42", Some("ENG-42"));
    repo.save_record(&Retirement {
        old_id: "PP-760",
        new_id: "ENG-42",
        new_external_id: Some("ENG-42"),
    });
    let content = repo.read("meta/work/ENG-42-add-search.md").unwrap();
    repo.write(
        "meta/work/ENG-42-add-search.md",
        &content.replace("---\n\n", "aliases: [\"PP-760\"]\n---\n\n"),
    );

    let outcome = sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-42")]),
        Options::default(),
    );

    assert!(
        outcome.report().identity.is_empty(),
        "{:?}",
        outcome.report().identity
    );
    assert!(repo.records().is_empty());
}

#[test]
fn reconciliation_waits_for_a_retirement_holding_the_lock() {
    let repo = Repo::new();
    interrupted_after_writing_to(&repo);
    let log = Log::default();

    sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-42")]),
        Options {
            log: Some(Rc::clone(&log)),
            ..Options::default()
        },
    );

    let order = log.borrow();
    assert_eq!(
        order.get(..3),
        Some(&["read-records", "lock", "read-records"][..]),
        "the record is re-read under the lock: {order:?}"
    );
}

#[test]
fn preview_reports_reconciliation_without_writing() {
    let repo = Repo::new();
    interrupted_after_writing_to(&repo);
    let before = repo.snapshot();

    let outcome = sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-42")]),
        Options {
            mode: RunMode::Preview,
            ..Options::default()
        },
    );

    assert_eq!(repo.snapshot(), before);
    let row = outcome.row("PP-760");
    assert_eq!(row.action, IdentityAction::Resumed);
    assert_eq!(row.outcome, IdentityOutcome::NotApplied);
}

#[test]
fn a_targeted_sync_still_reconciles_an_unrelated_interrupted_retirement() {
    let repo = Repo::new();
    interrupted_after_writing_to(&repo);
    repo.item("0001-other.md", "0001", Some("ENG-1"));

    let outcome = sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-42"), held("ENG-1")]),
        Options {
            targets: Some(vec!["0001"]),
            ..Options::default()
        },
    );

    assert_eq!(outcome.row("PP-760").action, IdentityAction::Resumed);
    assert!(repo.records().is_empty());
}

#[test]
fn reconciliation_does_not_consume_the_pull_budget() {
    let repo = Repo::new();
    interrupted_after_writing_to(&repo);

    let outcome = sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-42")]),
        Options {
            max_pulls: Ceiling::Bounded(0),
            ..Options::default()
        },
    );

    assert_eq!(outcome.row("PP-760").outcome, IdentityOutcome::Applied);
}

#[test]
fn a_rolled_back_key_change_leaves_no_retirement_record() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    let before = repo.snapshot();

    let outcome = sync(
        &repo,
        &moved("PP-760", "ENG-42"),
        Options {
            fail_at: Some(0),
            keep_failing: true,
            ..Options::default()
        },
    );

    assert!(repo.records().is_empty());
    assert!(matches!(
        outcome.row("PP-760").outcome,
        IdentityOutcome::Failed(_)
    ));
    let after = repo.snapshot();
    for file in ["meta/work/PP-760-add-search.md", "meta/plans/plan.md"] {
        assert_eq!(after.get(&repo.path(file)), before.get(&repo.path(file)));
    }
}

#[test]
fn a_refused_key_change_leaves_no_retirement_record() {
    let repo = Repo::new();
    repo.item("PP-760-add-search.md", "PP-760", Some("PP-760"));
    repo.write(
        "meta/work/ENG-7-other.md",
        "---\nid: \"ENG-7\"\naliases: [\"ENG-42\"]\n---\n\n# ENG-7: Other\n",
    );

    sync(&repo, &moved("PP-760", "ENG-42"), Options::default());

    assert!(repo.records().is_empty());
}

#[test]
fn a_refused_key_change_yields_one_row_and_no_remote_absent() {
    let repo = Repo::new();
    repo.item("PP-760-add-search.md", "PP-760", Some("PP-760"));
    repo.write(
        "meta/work/ENG-7-other.md",
        "---\nid: \"ENG-7\"\naliases: [\"ENG-42\"]\n---\n\n# ENG-7: Other\n",
    );

    let outcome = sync(&repo, &moved("PP-760", "ENG-42"), Options::default());

    assert_eq!(
        outcome
            .report()
            .identity
            .iter()
            .filter(|row| row.id == "PP-760")
            .count(),
        1
    );
    assert_eq!(outcome.engine_state("PP-760"), None);
}

#[test]
fn a_refused_key_change_is_not_imported_as_untracked() {
    let repo = Repo::new();
    repo.item("PP-760-add-search.md", "PP-760", Some("PP-760"));
    repo.write(
        "meta/work/ENG-7-other.md",
        "---\nid: \"ENG-7\"\naliases: [\"ENG-42\"]\n---\n\n# ENG-7: Other\n",
    );
    let tracker = moved("PP-760", "ENG-42").discovering(
        vec![(key("ENG-42"), RemoteTimestamp::Reported(STAMP.to_owned()))],
        true,
    );

    let outcome = sync(&repo, &tracker, Options::default());

    assert!(outcome.imported.is_empty(), "{:?}", outcome.imported);
}

#[test]
fn a_key_change_whose_retirement_rolled_back_is_not_imported_as_untracked() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    let tracker = moved("PP-760", "ENG-42").discovering(
        vec![(key("ENG-42"), RemoteTimestamp::Reported(STAMP.to_owned()))],
        true,
    );

    let outcome = sync(
        &repo,
        &tracker,
        Options {
            fail_at: Some(0),
            keep_failing: true,
            ..Options::default()
        },
    );

    assert!(outcome.imported.is_empty(), "{:?}", outcome.imported);
}

#[test]
fn retirement_incomplete_stops_the_pass_and_the_engine() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    let tracker = moved("PP-760", "ENG-42");

    let outcome = sync(
        &repo,
        &tracker,
        Options {
            fail_at: Some(1),
            keep_failing: true,
            ..Options::default()
        },
    );

    let RunError::RetirementIncomplete { message } = outcome.error() else {
        panic!("expected an incomplete restore: {:?}", outcome.error());
    };
    assert!(message.contains("retirement-incomplete"), "{message}");
    assert!(message.contains("PP-760"), "{message}");
    assert!(message.contains("ENG-42"), "{message}");
    assert!(
        !tracker.calls().iter().any(|call| matches!(
            call,
            Call::Update { .. } | Call::Search { .. }
        )),
        "the engine did not run: {:?}",
        tracker.calls()
    );
}

#[test]
fn key_changes_beyond_max_pulls_refuse_the_run_before_any_write() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    let before = repo.snapshot();

    let outcome = sync(
        &repo,
        &moved("PP-760", "ENG-42"),
        Options {
            max_pulls: Ceiling::Bounded(0),
            ..Options::default()
        },
    );

    assert!(matches!(
        outcome.error(),
        RunError::Refused { pulls: 1, .. }
    ));
    assert_eq!(repo.snapshot(), before);
}

#[test]
fn the_engine_gets_the_pull_budget_the_identity_pass_left() {
    let repo = Repo::new();
    repo.item("0230-add-search.md", "0230", Some("PP-760"));
    repo.item("0001-pulled.md", "0001", Some("ENG-1"));
    repo.baseline_set(
        "0001",
        Entry {
            remote_updated_at: RemoteTimestamp::Reported("stale".to_owned()),
            remote_hash: "stale".to_owned(),
            local_hash: digest::local(
                &repo.read("meta/work/0001-pulled.md").unwrap(),
            )
            .unwrap(),
            local_synced_at: 0,
        },
    );
    let tracker =
        RecordingTracker::holding(vec![held("PP-760"), held("ENG-1")])
            .moving(&key("PP-760"), &key("ENG-42"));

    let outcome = sync(
        &repo,
        &tracker,
        Options {
            ownership: IdOwnership::Local,
            max_pulls: Ceiling::Bounded(1),
            ..Options::default()
        },
    );

    assert!(
        matches!(
            outcome.error(),
            RunError::Refused {
                pulls: 1,
                max_pulls: Ceiling::Bounded(0),
                ..
            }
        ),
        "{:?}",
        outcome.error()
    );
    assert_eq!(
        outcome
            .result
            .as_ref()
            .err()
            .map(|failure| failure.identity_applied),
        Some(1)
    );
}

#[test]
fn an_engine_refusal_after_applied_key_changes_notes_them() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    repo.item("0001-pulled.md", "0001", Some("ENG-1"));
    repo.baseline_set(
        "0001",
        Entry {
            remote_updated_at: RemoteTimestamp::Reported("stale".to_owned()),
            remote_hash: "stale".to_owned(),
            local_hash: digest::local(
                &repo.read("meta/work/0001-pulled.md").unwrap(),
            )
            .unwrap(),
            local_synced_at: 0,
        },
    );
    let tracker =
        RecordingTracker::holding(vec![held("PP-760"), held("ENG-1")])
            .moving(&key("PP-760"), &key("ENG-42"));

    let outcome = sync(
        &repo,
        &tracker,
        Options {
            max_pulls: Ceiling::Bounded(1),
            ..Options::default()
        },
    );

    let failure = outcome.result.as_ref().err().expect("the engine refuses");
    assert_eq!(failure.identity_applied, 1);
    assert!(repo.read("meta/work/ENG-42-add-search.md").is_some());
}

#[test]
fn a_settled_run_reads_the_remote_once() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    repo.item("0001-other.md", "0001", Some("ENG-1"));
    let tracker =
        RecordingTracker::holding(vec![held("PP-760"), held("ENG-1")])
            .moving(&key("PP-760"), &key("ENG-42"));

    sync(&repo, &tracker, Options::default());

    assert_eq!(fetch_all_calls(&tracker), 1);
}

#[test]
fn a_targeted_sync_follows_key_changes_only_for_targets() {
    let repo = Repo::new();
    repo.item("0230-add-search.md", "0230", Some("PP-760"));
    repo.item("0231-other.md", "0231", Some("PP-761"));
    let tracker =
        RecordingTracker::holding(vec![held("PP-760"), held("PP-761")])
            .moving(&key("PP-760"), &key("ENG-42"))
            .moving(&key("PP-761"), &key("ENG-43"));

    let outcome = sync(
        &repo,
        &tracker,
        Options {
            ownership: IdOwnership::Local,
            targets: Some(vec!["0230"]),
            ..Options::default()
        },
    );

    assert_eq!(outcome.row("0230").outcome, IdentityOutcome::Applied);
    assert!(outcome.report().identity.iter().all(|row| row.id != "0231"));
    assert!(repo
        .read("meta/work/0231-other.md")
        .unwrap()
        .contains("PP-761"));
}

#[test]
fn a_misconfigured_discovery_refuses_before_any_identity_change() {
    let repo = Repo::new();
    moved_tracker_owned_item(&repo);
    let before = repo.snapshot();
    let tracker =
        moved("PP-760", "ENG-42").refusing_scope(tracker::ScopeError {
            detail: "no team".to_owned(),
        });

    let outcome = sync(&repo, &tracker, Options::default());

    assert!(matches!(
        outcome.error(),
        RunError::DiscoveryUnconfigured { .. }
    ));
    assert_eq!(repo.snapshot(), before);
    assert_eq!(fetch_all_calls(&tracker), 0);
}

#[test]
fn an_item_written_between_settlement_and_rediscovery_is_deferred_not_planned()
{
    let repo = Repo::new();
    repo.item("0001-other.md", "0001", Some("ENG-1"));
    let late = repo.work_dir().join("0002-late.md");

    let outcome = sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-1"), held("ENG-2")]),
        Options {
            before_rediscovery: Some(Box::new(move || {
                fs::write(
                    &late,
                    "---\nid: \"0002\"\nexternal_id: \"ENG-2\"\n---\n\n# 0002: Late\n",
                )
                .unwrap();
            })),
            ..Options::default()
        },
    );

    assert_eq!(outcome.report().deferred, 1);
    assert_eq!(outcome.engine_state("0002"), None);
}

#[test]
fn the_engine_sees_dirtiness_reprobed_after_the_pass() {
    let repo = Repo::new();
    repo.item("0001-pulled.md", "0001", Some("ENG-1"));
    repo.baseline_set(
        "0001",
        Entry {
            remote_updated_at: RemoteTimestamp::Reported("stale".to_owned()),
            remote_hash: "stale".to_owned(),
            local_hash: digest::local(
                &repo.read("meta/work/0001-pulled.md").unwrap(),
            )
            .unwrap(),
            local_synced_at: 0,
        },
    );

    let outcome = sync(
        &repo,
        &RecordingTracker::holding(vec![held("ENG-1")]),
        Options {
            dirtiness_after: Dirtiness::Dirty,
            ..Options::default()
        },
    );

    let reported = outcome
        .report()
        .reported
        .iter()
        .find(|reported| reported.planned.id == "0001")
        .expect("the item is planned");
    assert_eq!(
        reported.planned.action,
        work::sync::Action::Prompt,
        "a dirty, remotely modified item is not pulled over"
    );
}

#[test]
fn a_key_change_and_a_planned_pull_of_a_referencing_item_in_one_run_keep_the_rewrite(
) {
    let repo = Repo::new();
    repo.item("PP-760-add-search.md", "PP-760", Some("PP-760"));
    repo.write(
        "meta/work/0001-child.md",
        "---\nid: \"0001\"\ntitle: \"Title\"\nexternal_id: \"ENG-1\"\n\
         parent: \"work-item:PP-760\"\n---\n\n# 0001: Title\n",
    );
    repo.baseline_set(
        "0001",
        Entry {
            remote_updated_at: RemoteTimestamp::Reported("stale".to_owned()),
            remote_hash: "stale".to_owned(),
            local_hash: digest::local(
                &repo.read("meta/work/0001-child.md").unwrap(),
            )
            .unwrap(),
            local_synced_at: 0,
        },
    );
    let tracker =
        RecordingTracker::holding(vec![held("PP-760"), held("ENG-1")])
            .moving(&key("PP-760"), &key("ENG-42"));

    sync(&repo, &tracker, Options::default());

    let child = repo.read("meta/work/0001-child.md").unwrap();
    assert!(child.contains("work-item:ENG-42"), "{child}");
    assert!(!child.contains("PP-760"), "{child}");
    assert!(repo.baseline_entry("ENG-42").is_some());
}

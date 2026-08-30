//! The two out-of-band create paths driven through `work_adapters::sync::run`:
//! untracked-remote pull (create-from-remote) and unsynced-local create
//! (create-from-local), with the combined gate and the pending-push recovery.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::ScopeError;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use work::sync::Action;
use work::sync::Dirtiness;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::create::AuthoredLocal;
use work_adapters::sync::create::DiscoveredIssue;
use work_adapters::sync::create::LocalAuthor;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::run::run;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::RunError;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;

type TestError = Box<dyn std::error::Error>;

const BASELINE_PATH: &str = "/baseline/last-sync.json";

/// A baseline-store backing on real memory plus a call log, so a refusal can be
/// shown to precede every write.
#[derive(Default)]
struct Spy {
    files: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
    writes: RefCell<Vec<PathBuf>>,
}

impl Spy {
    fn seed(&self, path: &str, content: &str) {
        self.files
            .borrow_mut()
            .insert(PathBuf::from(path), content.as_bytes().to_vec());
    }

    fn write_count(&self) -> usize {
        self.writes.borrow().len()
    }
}

impl FileReader for Spy {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        Ok(self
            .files
            .borrow()
            .get(path)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned()))
    }
}

impl AtomicWrite for Spy {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.writes.borrow_mut().push(path.to_path_buf());
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }
}

struct AlwaysClean;

impl WorkingCopyStatus for AlwaysClean {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        Dirtiness::Clean
    }
}

struct FixedClock(u64);

impl work::sync::RunClock for FixedClock {
    fn run_start_epoch(&self) -> Result<u64, kernel::Error> {
        Ok(self.0)
    }
}

/// A `LocalAuthor` that authors a minimal valid file for a discovery and
/// records every call, so a test can assert exactly which issues were authored
/// and linked.
struct RecordingAuthor {
    dir: PathBuf,
    authored: RefCell<Vec<ExternalId>>,
    linked: RefCell<Vec<(PathBuf, ExternalId)>>,
    next: RefCell<u32>,
}

impl RecordingAuthor {
    fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            authored: RefCell::new(Vec::new()),
            linked: RefCell::new(Vec::new()),
            next: RefCell::new(9000),
        }
    }
}

impl LocalAuthor for RecordingAuthor {
    fn author_from_remote(
        &self,
        issue: &DiscoveredIssue,
    ) -> Result<AuthoredLocal, kernel::Error> {
        self.authored.borrow_mut().push(issue.external_id.clone());
        let mut next = self.next.borrow_mut();
        let id = format!("{next}");
        *next += 1;
        let path = self.dir.join(format!("{id}.md"));
        std::fs::write(
            &path,
            format!(
                "---\nid: \"{id}\"\nexternal_id: \"{}\"\n---\n\n{}\n",
                issue.external_id.as_str(),
                issue.issue.body
            ),
        )
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
        Ok(AuthoredLocal { id, path })
    }

    fn link_external_id(
        &self,
        path: &Path,
        external_id: &ExternalId,
    ) -> Result<(), kernel::Error> {
        self.linked
            .borrow_mut()
            .push((path.to_path_buf(), external_id.clone()));
        Ok(())
    }
}

fn baseline_document(entries: &[String]) -> String {
    format!("{{\"timestamp\":0,\"items\":{{{}}}}}\n", entries.join(","))
}

fn issue(body: &str) -> RemoteIssue {
    RemoteIssue {
        updated: RemoteTimestamp::Reported("2026-06-01T00:00:00Z".to_owned()),
        body: body.to_owned(),
    }
}

struct Fixture {
    dir: tempfile::TempDir,
    spy: Spy,
}

impl Fixture {
    fn new() -> Result<Self, TestError> {
        let dir = tempfile::tempdir()?;
        let spy = Spy::default();
        spy.seed(BASELINE_PATH, &baseline_document(&[]));
        Ok(Self { dir, spy })
    }

    /// A local draft with no `external_id`, so it classifies `Unsynced`.
    fn unsynced_item(
        &self,
        id: &str,
        title: &str,
    ) -> Result<LocalItem, TestError> {
        let path = self.dir.path().join(format!("{id}.md"));
        std::fs::write(
            &path,
            format!("---\nid: \"{id}\"\ntitle: \"{title}\"\nkind: task\n---\n\nBody\n"),
        )?;
        Ok(LocalItem {
            id: id.to_owned(),
            path,
            external_id: None,
        })
    }
}

struct Ports<'a> {
    tracker: &'a RecordingTracker,
    author: &'a RecordingAuthor,
    spy: &'a Spy,
}

#[allow(clippy::too_many_arguments)]
fn run_sync(
    ports: &Ports<'_>,
    items: &[LocalItem],
    integrations_root: &Path,
    direction: SyncDirection,
    scope: SearchScope,
    max_pulls: usize,
    max_pushes: usize,
    mode: RunMode,
) -> Result<RunReport, RunError> {
    let clock = FixedClock(1_700_000_000);
    let status = AlwaysClean;
    let sync_ports = SyncPorts {
        tracker: ports.tracker,
        status: &status,
        writer: ports.spy,
        clock: &clock,
        author: ports.author,
    };
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), ports.spy, ports.spy);
    let resolutions: BTreeMap<String, Resolution> = BTreeMap::new();
    let request = SyncRequest {
        items,
        direction,
        strategy: RetrievalStrategy::Bulk,
        resolutions: &resolutions,
        max_pulls,
        max_pushes,
        mode,
        integrations_root,
        integration: "jira",
        scope,
    };
    run(&sync_ports, &mut store, &request)
}

fn scoped() -> SearchScope {
    SearchScope {
        project: Some("ENG".to_owned()),
        all_projects: false,
        filters: Vec::new(),
    }
}

// --- Gap A: untracked-remote pull -------------------------------------------

#[test]
fn discovery_creates_only_untracked_issues_folding_cosmetic_ids(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    // One local item carries ENG-1 in a cosmetically different spelling.
    let tracked_path = fixture.dir.path().join("0001.md");
    std::fs::write(
        &tracked_path,
        "---\nid: \"0001\"\nexternal_id: \"eng-1\"\n---\n\nBody\n",
    )?;
    let items = vec![LocalItem {
        id: "0001".to_owned(),
        path: tracked_path,
        external_id: Some(ExternalId::new("eng-1".to_owned())),
    }];

    let tracker = RecordingTracker::holding(vec![
        (ExternalId::new("ENG-1".to_owned()), issue("One\nbody")),
        (ExternalId::new("ENG-2".to_owned()), issue("Two\nbody")),
    ])
    .discovering(
        vec![
            (
                ExternalId::new("ENG-1".to_owned()),
                RemoteTimestamp::NotReported,
            ),
            (
                ExternalId::new("ENG-2".to_owned()),
                RemoteTimestamp::NotReported,
            ),
        ],
        true,
    );
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let report = run_sync(
        &ports,
        &items,
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        scoped(),
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "the run must not refuse")?;

    assert_eq!(
        *author.authored.borrow(),
        vec![ExternalId::new("ENG-2".to_owned())],
        "only the untracked ENG-2 is authored; ENG-1 folds equal to eng-1"
    );
    let creates = report
        .reported
        .iter()
        .filter(|item| item.planned.action == Action::CreateFromRemote)
        .count();
    assert_eq!(creates, 1);
    Ok(())
}

#[test]
fn an_over_threshold_untracked_set_aborts_with_no_creations_or_shows(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let tracker = RecordingTracker::holding(vec![
        (ExternalId::new("ENG-1".to_owned()), issue("One\nbody")),
        (ExternalId::new("ENG-2".to_owned()), issue("Two\nbody")),
        (ExternalId::new("ENG-3".to_owned()), issue("Three\nbody")),
    ])
    .discovering(
        vec![
            (
                ExternalId::new("ENG-1".to_owned()),
                RemoteTimestamp::NotReported,
            ),
            (
                ExternalId::new("ENG-2".to_owned()),
                RemoteTimestamp::NotReported,
            ),
            (
                ExternalId::new("ENG-3".to_owned()),
                RemoteTimestamp::NotReported,
            ),
        ],
        true,
    );
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let error = run_sync(
        &ports,
        &[],
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        scoped(),
        2,
        25,
        RunMode::Apply,
    )
    .err()
    .expect("three untracked against a bound of two must refuse");

    match error {
        RunError::Refused {
            pulls,
            new_local_files,
            ..
        } => {
            assert_eq!(pulls, 3);
            assert_eq!(new_local_files, 3);
        }
        other => panic!("expected Refused, got a different error {other:?}"),
    }
    assert!(
        author.authored.borrow().is_empty(),
        "no file may be authored"
    );
    assert!(
        !tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Show { .. })),
        "no per-issue show fan-out before the gate"
    );
    assert_eq!(fixture.spy.write_count(), 0);
    Ok(())
}

#[test]
fn an_unresolvable_scope_refuses_pre_flight_and_sends_nothing(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    // An unsynced draft that would create-from-local were the run to proceed.
    let item = fixture.unsynced_item("0001", "Draft")?;
    let tracker =
        RecordingTracker::holding(Vec::new()).refusing_scope(ScopeError {
            detail: "E_SEARCH_NO_TEAM: discovery needs a team key".to_owned(),
        });
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let error = run_sync(
        &ports,
        &[item],
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        scoped(),
        25,
        25,
        RunMode::Apply,
    )
    .err()
    .expect("an unresolvable scope must refuse the run");

    assert!(
        matches!(error, RunError::DiscoveryUnconfigured { .. }),
        "an unresolvable scope refuses as DiscoveryUnconfigured, got: {error:?}"
    );
    assert!(
        !tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Search { .. })),
        "a refused scope must not reach the search"
    );
    assert!(
        !tracker.calls().iter().any(|call| matches!(
            call,
            Call::Create { .. } | Call::Update { .. }
        )),
        "a pre-flight refusal must send no push"
    );
    assert_eq!(
        fixture.spy.write_count(),
        0,
        "a pre-flight refusal must not write"
    );
    Ok(())
}

#[test]
fn a_push_only_run_authors_no_untracked_local_files() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let tracker = RecordingTracker::holding(vec![(
        ExternalId::new("ENG-2".to_owned()),
        issue("Two\nbody"),
    )])
    .discovering(
        vec![(
            ExternalId::new("ENG-2".to_owned()),
            RemoteTimestamp::NotReported,
        )],
        true,
    );
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let _ = run_sync(
        &ports,
        &[],
        fixture.dir.path(),
        SyncDirection::PushOnly,
        scoped(),
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "push-only with no items must not refuse")?;

    assert!(
        author.authored.borrow().is_empty(),
        "push-only must not author untracked local files"
    );
    assert!(
        !tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Search { .. })),
        "push-only must not even search"
    );
    Ok(())
}

#[test]
fn the_happy_path_authors_exactly_one_file_per_untracked_issue(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let tracker = RecordingTracker::holding(vec![(
        ExternalId::new("ENG-7".to_owned()),
        issue("Seven\nremote body"),
    )])
    .discovering(
        vec![(
            ExternalId::new("ENG-7".to_owned()),
            RemoteTimestamp::NotReported,
        )],
        true,
    );
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let report = run_sync(
        &ports,
        &[],
        fixture.dir.path(),
        SyncDirection::PullOnly,
        scoped(),
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "one untracked pull must proceed")?;

    assert_eq!(author.authored.borrow().len(), 1);
    let create = report
        .reported
        .iter()
        .find(|item| item.planned.action == Action::CreateFromRemote)
        .expect("one create-from-remote row");
    assert!(matches!(create.outcome, ItemOutcome::Applied));
    Ok(())
}

#[test]
fn an_incomplete_discovery_is_refused_with_guidance() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let tracker = RecordingTracker::holding(Vec::new()).discovering(
        vec![(
            ExternalId::new("ENG-9".to_owned()),
            RemoteTimestamp::NotReported,
        )],
        false,
    );
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let error = run_sync(
        &ports,
        &[],
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        scoped(),
        25,
        25,
        RunMode::Apply,
    )
    .err()
    .expect("a truncated discovery must refuse");

    assert!(matches!(error, RunError::DiscoveryIncomplete { .. }));
    assert!(author.authored.borrow().is_empty());
    Ok(())
}

#[test]
fn preview_reports_untracked_as_create_rows_without_authoring(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let tracker = RecordingTracker::holding(vec![(
        ExternalId::new("ENG-5".to_owned()),
        issue("Five\nbody"),
    )])
    .discovering(
        vec![(
            ExternalId::new("ENG-5".to_owned()),
            RemoteTimestamp::NotReported,
        )],
        true,
    );
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let report = run_sync(
        &ports,
        &[],
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        scoped(),
        25,
        25,
        RunMode::Preview,
    )
    .map_err(|_| "preview within bounds must not refuse")?;

    assert!(
        author.authored.borrow().is_empty(),
        "preview authors nothing"
    );
    assert_eq!(fixture.spy.write_count(), 0, "preview writes nothing");
    assert!(
        !tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Show { .. })),
        "preview issues no show fan-out"
    );
    assert_eq!(
        report
            .reported
            .iter()
            .filter(|item| item.planned.action == Action::CreateFromRemote)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn a_show_failure_mid_batch_fails_that_issue_and_continues(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let tracker = RecordingTracker::holding(vec![
        (ExternalId::new("ENG-1".to_owned()), issue("One\nbody")),
        (ExternalId::new("ENG-3".to_owned()), issue("Three\nbody")),
    ])
    .discovering(
        vec![
            (
                ExternalId::new("ENG-1".to_owned()),
                RemoteTimestamp::NotReported,
            ),
            (
                ExternalId::new("ENG-2".to_owned()),
                RemoteTimestamp::NotReported,
            ),
            (
                ExternalId::new("ENG-3".to_owned()),
                RemoteTimestamp::NotReported,
            ),
        ],
        true,
    )
    .failing_show(
        ExternalId::new("ENG-2".to_owned()),
        TrackerError::Retryable {
            detail: "connection reset".to_owned(),
        },
    );
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let report = run_sync(
        &ports,
        &[],
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        scoped(),
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "a per-issue show failure must not abort the batch")?;

    assert_eq!(
        *author.authored.borrow(),
        vec![
            ExternalId::new("ENG-1".to_owned()),
            ExternalId::new("ENG-3".to_owned()),
        ],
        "the readable issues are authored around the failed one"
    );
    let failed = report
        .reported
        .iter()
        .filter(|item| {
            item.planned.action == Action::CreateFromRemote
                && matches!(item.outcome, ItemOutcome::Failed(_))
        })
        .count();
    assert_eq!(failed, 1, "exactly the unreadable issue is reported Failed");
    Ok(())
}

#[test]
fn planned_writes_over_bound_refuse_before_any_create_from_remote(
) -> Result<(), TestError> {
    // Two planned pulls against a bound of one, plus a single small
    // discovery: the combined gate must refuse before any create is authored.
    let fixture = Fixture::new()?;
    let stamp = RemoteTimestamp::Reported("2026-07-01T00:00:00Z".to_owned());
    let mut items = Vec::new();
    let mut entries = Vec::new();
    let mut holding = Vec::new();
    for index in 1..=2 {
        let id = format!("{index:04}");
        let external = ExternalId::new(format!("TRK-{index}"));
        let path = fixture.dir.path().join(format!("{id}.md"));
        let content = format!(
            "---\nstatus: ready\nexternal_id: \"{}\"\n---\n\nBody text\n",
            external.as_str()
        );
        std::fs::write(&path, &content)?;
        let local_hash = work_adapters::sync::digest::local(&content)?;
        entries.push(format!(
            "\"{id}\":{{\"remote_updated_at\":\"2026-06-01T00:00:00Z\",\
             \"remote_hash\":\"stale\",\"local_hash\":\"{local_hash}\"}}"
        ));
        holding.push((
            external.clone(),
            RemoteIssue {
                updated: stamp.clone(),
                body: "Title\nRemote body\n".to_owned(),
            },
        ));
        items.push(LocalItem {
            id,
            path,
            external_id: Some(external),
        });
    }
    fixture
        .spy
        .seed(BASELINE_PATH, &baseline_document(&entries));

    let tracker = RecordingTracker::holding(holding).discovering(
        vec![(
            ExternalId::new("ENG-9".to_owned()),
            RemoteTimestamp::NotReported,
        )],
        true,
    );
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let error = run_sync(
        &ports,
        &items,
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        scoped(),
        1,
        25,
        RunMode::Apply,
    )
    .err()
    .expect("three pulls (2 planned + 1 discovered) against a bound of 1");

    assert!(matches!(error, RunError::Refused { pulls: 3, .. }));
    assert!(
        author.authored.borrow().is_empty(),
        "the gate must refuse before any create-from-remote write"
    );
    Ok(())
}

// --- Gap B: unsynced-local create -------------------------------------------

#[test]
fn an_unsynced_item_issues_exactly_one_create_and_links_it(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let item = fixture.unsynced_item("0001", "Draft one")?;
    let tracker = RecordingTracker::holding(Vec::new());
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let report = run_sync(
        &ports,
        std::slice::from_ref(&item),
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        SearchScope::default(),
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "one create-from-local must proceed")?;

    let creates = tracker
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::Create { .. }))
        .count();
    assert_eq!(creates, 1, "exactly one remote create");
    let linked = author.linked.borrow();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].1, ExternalId::new("REC-1".to_owned()));
    assert!(report.reported.iter().any(|item| {
        item.planned.action == Action::CreateFromLocal
            && matches!(item.outcome, ItemOutcome::Applied)
    }));
    Ok(())
}

#[test]
fn create_from_local_writes_the_marker_before_the_create(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let item = fixture.unsynced_item("0001", "Draft one")?;

    // The marker path the engine will use, so the fake create can observe it.
    let marker_path = work_adapters::sync::pending_push::path(
        fixture.dir.path(),
        "jira",
        "0001",
    );
    let observed = std::rc::Rc::new(RefCell::new(false));
    let tracker = MarkerObservingTracker {
        marker_path,
        marker_present_at_create: std::rc::Rc::clone(&observed),
        calls: RefCell::new(Vec::new()),
    };
    let author = RecordingAuthor::new(fixture.dir.path());
    // The marker writer must be a real store so the fake create can stat it.
    let real = RealWrite;
    let clock = FixedClock(1_700_000_000);
    let status = AlwaysClean;
    let sync_ports = SyncPorts {
        tracker: &tracker,
        status: &status,
        writer: &real,
        clock: &clock,
        author: &author,
    };
    let baseline_path = fixture.dir.path().join("last-sync.json");
    std::fs::write(&baseline_path, baseline_document(&[]))?;
    let reader = RealRead;
    let mut store = BaselineStore::new(baseline_path, &reader, &real);
    let resolutions: BTreeMap<String, Resolution> = BTreeMap::new();
    let request = SyncRequest {
        items: std::slice::from_ref(&item),
        direction: SyncDirection::Bidirectional,
        strategy: RetrievalStrategy::Bulk,
        resolutions: &resolutions,
        max_pulls: 25,
        max_pushes: 25,
        mode: RunMode::Apply,
        integrations_root: fixture.dir.path(),
        integration: "jira",
        scope: SearchScope::default(),
    };
    let _ = run(&sync_ports, &mut store, &request);

    assert!(
        *observed.borrow(),
        "the pending-push marker must exist on disk when create is called"
    );
    Ok(())
}

#[test]
fn a_seeded_created_marker_reuses_the_id_without_a_second_create(
) -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let item = fixture.unsynced_item("0001", "Draft one")?;

    // Seed a Created marker matching the request fingerprint. The engine
    // derives (title, body, kind) from the draft's own frontmatter and split
    // body, so the digest must be computed from exactly that.
    let content = std::fs::read_to_string(&item.path)?;
    let (frontmatter, body) =
        work_adapters::sync::digest::split_frontmatter_and_body(&content)?;
    let title = work::show::read_field_raw(&frontmatter, "title").unwrap();
    let kind = work::show::read_field_raw(&frontmatter, "kind").unwrap();
    let digest =
        work_adapters::sync::pending_push::request_digest(&title, &body, &kind);
    let marker = work::sync::PendingPush::Created {
        request: work::sync::RequestFingerprint {
            title: "Draft one".to_owned(),
            digest,
            attempted_at: 1,
            failure: None,
        },
        external_id: ExternalId::new("ENG-42".to_owned()),
    };
    let marker_path = work_adapters::sync::pending_push::path(
        fixture.dir.path(),
        "jira",
        "0001",
    );
    std::fs::create_dir_all(marker_path.parent().unwrap())?;
    std::fs::write(
        &marker_path,
        work_adapters::sync::pending_push::render(&marker),
    )?;

    let tracker = RecordingTracker::holding(vec![(
        ExternalId::new("ENG-42".to_owned()),
        issue("Draft one\nBody"),
    )]);
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let _ = run_sync(
        &ports,
        std::slice::from_ref(&item),
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        SearchScope::default(),
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "reuse path must proceed")?;

    assert!(
        !tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Create { .. })),
        "a Created marker must be reused, not re-created"
    );
    let linked = author.linked.borrow();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].1, ExternalId::new("ENG-42".to_owned()));
    Ok(())
}

#[test]
fn create_from_local_counts_against_max_pushes() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let item = fixture.unsynced_item("0001", "Draft one")?;
    let tracker = RecordingTracker::holding(Vec::new());
    let author = RecordingAuthor::new(fixture.dir.path());
    let ports = Ports {
        tracker: &tracker,
        author: &author,
        spy: &fixture.spy,
    };

    let error = run_sync(
        &ports,
        std::slice::from_ref(&item),
        fixture.dir.path(),
        SyncDirection::Bidirectional,
        SearchScope::default(),
        25,
        0,
        RunMode::Apply,
    )
    .err()
    .expect("a --max-pushes 0 run must refuse the create-from-local");

    match error {
        RunError::Refused {
            pushes,
            new_remote_issues,
            ..
        } => {
            assert_eq!(pushes, 1);
            assert_eq!(new_remote_issues, 1);
        }
        other => panic!("expected Refused, got {other:?}"),
    }
    assert!(
        !tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Create { .. })),
        "a refused run creates no remote issue"
    );
    Ok(())
}

// --- Small real-fs doubles for the marker-ordering test ---------------------

struct RealWrite;

impl AtomicWrite for RealWrite {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                StoreError::Io {
                    path: parent.display().to_string(),
                    detail: error.to_string(),
                }
            })?;
        }
        std::fs::write(path, bytes).map_err(|error| StoreError::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        })
    }
}

struct RealRead;

impl FileReader for RealRead {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(None)
            }
            Err(error) => Err(kernel::Error::Failed(error.to_string())),
        }
    }
}

/// A tracker whose `create` records whether the pending-push marker is present
/// on disk at the moment it is called.
struct MarkerObservingTracker {
    marker_path: PathBuf,
    marker_present_at_create: std::rc::Rc<RefCell<bool>>,
    calls: RefCell<Vec<Call>>,
}

impl tracker::RemoteTracker for MarkerObservingTracker {
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        *self.marker_present_at_create.borrow_mut() = self.marker_path.exists();
        self.calls.borrow_mut().push(Call::Create {
            title: title.to_owned(),
            body: body.to_owned(),
            kind: kind.to_owned(),
        });
        Ok(ExternalId::new("ENG-1".to_owned()))
    }

    fn update(
        &self,
        _id: &ExternalId,
        _title: &str,
        _body: &str,
    ) -> Result<(), TrackerError> {
        Ok(())
    }

    fn show(&self, _id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        Ok(issue("Draft one\nBody"))
    }

    fn fetch_all(
        &self,
        _ids: &[ExternalId],
    ) -> Result<tracker::FetchOutcome, TrackerError> {
        Ok(tracker::FetchOutcome {
            found: Vec::new(),
            absent: Vec::new(),
            indeterminate: Vec::new(),
        })
    }

    fn search(
        &self,
        _scope: &SearchScope,
    ) -> Result<tracker::Discovery, TrackerError> {
        Ok(tracker::Discovery {
            found: Vec::new(),
            complete: true,
        })
    }

    fn resolve_scope(
        &self,
        scope: &SearchScope,
    ) -> Result<SearchScope, tracker::ScopeError> {
        Ok(scope.clone())
    }

    fn preview_create(
        &self,
        _kind: &str,
    ) -> Result<tracker::CreatePreview, TrackerError> {
        Ok(tracker::CreatePreview {
            project: tracker::FieldResolution::Unset,
            issue_type: tracker::FieldResolution::Unset,
        })
    }

    fn validate_update(
        &self,
        _id: &ExternalId,
        _title: &str,
        _body: &str,
    ) -> tracker::ValidationOutcome {
        tracker::ValidationOutcome::Valid
    }
}

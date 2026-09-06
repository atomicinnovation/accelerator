//! The whole-corpus run: the write bounds, preview's no-write guarantee,
//! and the finalise bookkeeping.
//!
//! Items live on real files because `run` and `LazyItemDigests` read them
//! through `std::fs`; only the baseline goes through the injected reader
//! and writer, which is what lets a spy count writes.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use work::sync::Dirtiness;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work::sync::SyncState;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::digest;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::run::run;
use work_adapters::sync::run::DiscoveryStatus;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::ItemSelection;
use work_adapters::sync::run::RunError;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;

type TestError = Box<dyn std::error::Error>;

const BASELINE_PATH: &str = "/baseline/last-sync.json";
const STAMP: &str = "2026-06-01T00:00:00Z";
const MOVED_STAMP: &str = "2026-07-01T00:00:00Z";

/// Baseline store backing plus a write counter, so a refusal can be shown
/// to happen before any write rather than merely to return an error.
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

    fn content(&self, path: &str) -> Option<String> {
        self.content_of(Path::new(path))
    }

    fn content_of(&self, path: &Path) -> Option<String> {
        self.files
            .borrow()
            .get(path)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
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

struct AlwaysDirty;

impl WorkingCopyStatus for AlwaysDirty {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        Dirtiness::Dirty
    }
}

/// A `LocalAuthor` the create-free scenarios never invoke: an empty discovery
/// and no unsynced items mean neither create path runs.
struct UnusedAuthor;

impl work_adapters::sync::create::LocalAuthor for UnusedAuthor {
    fn author_from_remote(
        &self,
        _issue: &work_adapters::sync::create::DiscoveredIssue,
    ) -> Result<work_adapters::sync::create::AuthoredLocal, kernel::Error> {
        panic!("no scenario in this suite authors from a discovered issue")
    }

    fn link_external_id(
        &self,
        _path: &Path,
        _external_id: &ExternalId,
    ) -> Result<(), kernel::Error> {
        panic!("no scenario in this suite links an external id")
    }
}

struct FixedClock(u64);

impl work::sync::RunClock for FixedClock {
    fn run_start_epoch(&self) -> Result<u64, kernel::Error> {
        Ok(self.0)
    }
}

fn item_content(id: &str) -> String {
    format!("---\nstatus: ready\nexternal_id: \"{id}\"\n---\n\nBody text\n")
}

const fn projected_body() -> &'static str {
    "Title\nRemote body\n"
}

/// One item whose remote side has moved and whose local side has not, so it
/// classifies `remotely-modified` and — clean, bidirectional — decides
/// `Pull`.
fn pullable(
    dir: &Path,
    index: usize,
) -> Result<(LocalItem, (ExternalId, RemoteIssue), String), TestError> {
    let id = format!("{index:04}");
    let external = ExternalId::new(format!("ENG-{index}"));
    let path = dir.join(format!("{id}.md"));
    std::fs::write(&path, item_content(external.as_str()))?;

    let local_hash = digest::local(&item_content(external.as_str()))?;
    let entry = format!(
        "\"{id}\":{{\"remote_updated_at\":\"{STAMP}\",\"remote_hash\":\"stale\",\"local_hash\":\"{local_hash}\"}}"
    );
    let issue = RemoteIssue {
        updated: RemoteTimestamp::Reported(MOVED_STAMP.to_owned()),
        body: projected_body().to_owned(),
    };
    Ok((
        LocalItem {
            id,
            path,
            external_id: Some(external.clone()),
        },
        (external, issue),
        entry,
    ))
}

/// One item whose local side has moved and whose remote side is proven
/// unchanged, so it classifies `locally-modified` and decides `Push`.
fn pushable(
    dir: &Path,
    index: usize,
) -> Result<(LocalItem, (ExternalId, RemoteIssue), String), TestError> {
    let id = format!("{index:04}");
    let external = ExternalId::new(format!("ENG-{index}"));
    let path = dir.join(format!("{id}.md"));
    std::fs::write(&path, item_content(external.as_str()))?;

    let remote_hash = digest::remote_body(projected_body());
    let entry = format!(
        "\"{id}\":{{\"remote_updated_at\":\"{STAMP}\",\"remote_hash\":\"{remote_hash}\",\"local_hash\":\"stale\"}}"
    );
    let issue = RemoteIssue {
        updated: RemoteTimestamp::Reported(STAMP.to_owned()),
        body: projected_body().to_owned(),
    };
    Ok((
        LocalItem {
            id,
            path,
            external_id: Some(external.clone()),
        },
        (external, issue),
        entry,
    ))
}

fn baseline_document(entries: &[String]) -> String {
    format!("{{\"timestamp\":0,\"items\":{{{}}}}}\n", entries.join(","))
}

struct Scenario {
    items: Vec<LocalItem>,
    tracker: RecordingTracker,
    spy: Spy,
    dir: tempfile::TempDir,
}

fn scenario(pulls: usize, pushes: usize) -> Result<Scenario, TestError> {
    let dir = tempfile::tempdir()?;
    let mut items = Vec::new();
    let mut issues = Vec::new();
    let mut entries = Vec::new();

    for index in 0..pulls {
        let (item, issue, entry) = pullable(dir.path(), index + 1)?;
        items.push(item);
        issues.push(issue);
        entries.push(entry);
    }
    for index in 0..pushes {
        let (item, issue, entry) = pushable(dir.path(), 100 + index)?;
        items.push(item);
        issues.push(issue);
        entries.push(entry);
    }

    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(&entries));

    Ok(Scenario {
        items,
        tracker: RecordingTracker::holding(issues),
        spy,
        dir,
    })
}

fn request<'a>(
    corpus: &'a [LocalItem],
    selection: ItemSelection<'a>,
    resolutions: &'a BTreeMap<String, Resolution>,
    integrations_root: &'a Path,
    max_pulls: usize,
    max_pushes: usize,
    mode: RunMode,
) -> SyncRequest<'a> {
    SyncRequest {
        corpus,
        selection,
        direction: SyncDirection::Bidirectional,
        strategy: RetrievalStrategy::Bulk,
        resolutions,
        max_pulls,
        max_pushes,
        mode,
        integrations_root,
        integration: "jira",
        scope: tracker::SearchScope::default(),
    }
}

fn execute(
    scenario: &Scenario,
    max_pulls: usize,
    max_pushes: usize,
    mode: RunMode,
) -> Result<work_adapters::sync::run::RunReport, RunError> {
    let clock = FixedClock(1_700_000_000);
    let status = AlwaysClean;
    let author = UnusedAuthor;
    let ports = SyncPorts {
        tracker: &scenario.tracker,
        status: &status,
        writer: &scenario.spy,
        clock: &clock,
        author: &author,
    };
    let mut store = BaselineStore::new(
        PathBuf::from(BASELINE_PATH),
        &scenario.spy,
        &scenario.spy,
    );
    let resolutions = BTreeMap::new();
    run(
        &ports,
        &mut store,
        &request(
            &scenario.items,
            ItemSelection::All,
            &resolutions,
            scenario.dir.path(),
            max_pulls,
            max_pushes,
            mode,
        ),
    )
}

fn execute_targeted(
    scenario: &Scenario,
    targeted: &[LocalItem],
    direction: SyncDirection,
    max_pulls: usize,
    max_pushes: usize,
    mode: RunMode,
) -> Result<work_adapters::sync::run::RunReport, RunError> {
    let clock = FixedClock(1_700_000_000);
    let status = AlwaysClean;
    let author = UnusedAuthor;
    let ports = SyncPorts {
        tracker: &scenario.tracker,
        status: &status,
        writer: &scenario.spy,
        clock: &clock,
        author: &author,
    };
    let mut store = BaselineStore::new(
        PathBuf::from(BASELINE_PATH),
        &scenario.spy,
        &scenario.spy,
    );
    let resolutions = BTreeMap::new();
    let mut req = request(
        &scenario.items,
        ItemSelection::Targeted(targeted),
        &resolutions,
        scenario.dir.path(),
        max_pulls,
        max_pushes,
        mode,
    );
    req.direction = direction;
    run(&ports, &mut store, &req)
}

/// A per-item projection stable enough to compare a targeted run's outcome
/// for one item against the same item's outcome in a full sync. `ReportedItem`
/// derives no `PartialEq`, so this reduces it to comparable fragments.
fn project_item(report: &RunReport, id: &str) -> Option<(String, String)> {
    report
        .reported
        .iter()
        .find(|item| item.planned.id == id)
        .map(|item| {
            let outcome = match &item.outcome {
                ItemOutcome::Applied => "applied".to_owned(),
                ItemOutcome::NotApplied => "not-applied".to_owned(),
                ItemOutcome::Failed(error) => format!("failed:{error:?}"),
            };
            (
                format!("{:?}/{:?}", item.planned.action, item.planned.state),
                outcome,
            )
        })
}

/// A run at a chosen epoch over a persistent spy, so a sequence of runs shares
/// one baseline document — the watermark scenarios need "run at E1, then E2,
/// then E3 against one baseline".
fn run_with<'a>(
    spy: &Spy,
    tracker: &RecordingTracker,
    dir: &Path,
    corpus: &'a [LocalItem],
    selection: ItemSelection<'a>,
    epoch: u64,
) -> Result<RunReport, RunError> {
    let clock = FixedClock(epoch);
    let status = AlwaysClean;
    let author = UnusedAuthor;
    let ports = SyncPorts {
        tracker,
        status: &status,
        writer: spy,
        clock: &clock,
        author: &author,
    };
    let mut store = BaselineStore::new(PathBuf::from(BASELINE_PATH), spy, spy);
    let resolutions = BTreeMap::new();
    let request = SyncRequest {
        corpus,
        selection,
        direction: SyncDirection::Bidirectional,
        strategy: RetrievalStrategy::Bulk,
        resolutions: &resolutions,
        max_pulls: 25,
        max_pushes: 25,
        mode: RunMode::Apply,
        integrations_root: dir,
        integration: "jira",
        scope: tracker::SearchScope::default(),
    };
    run(&ports, &mut store, &request)
}

fn mtime_secs(path: &Path) -> Result<u64, TestError> {
    Ok(path
        .metadata()?
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}

#[test]
fn a_targeted_run_does_not_bury_a_non_targeted_local_edit(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;

    // A: a synced item — local matches baseline, remote proven unchanged.
    let a_external = ExternalId::new("ENG-1".to_owned());
    let a_path = dir.path().join("0001.md");
    let a_content = item_content(a_external.as_str());
    std::fs::write(&a_path, &a_content)?;
    let a_hash = digest::local(&a_content)?;
    let a_entry = format!(
        "\"0001\":{{\"remote_updated_at\":\"{STAMP}\",\"remote_hash\":\"h\",\"local_hash\":\"{a_hash}\",\"local_synced_at\":0}}"
    );

    // B: a pullable item, the middle run's sole target.
    let (b_item, b_issue, b_entry) = pullable(dir.path(), 2)?;

    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(&[a_entry, b_entry]));
    let tracker = RecordingTracker::holding(vec![
        (
            a_external.clone(),
            RemoteIssue {
                updated: RemoteTimestamp::Reported(STAMP.to_owned()),
                body: projected_body().to_owned(),
            },
        ),
        b_issue,
    ]);

    let corpus = vec![
        LocalItem {
            id: "0001".to_owned(),
            path: a_path.clone(),
            external_id: Some(a_external),
        },
        b_item,
    ];

    let m0 = mtime_secs(&a_path)?;
    run_with(
        &spy,
        &tracker,
        dir.path(),
        &corpus,
        ItemSelection::All,
        m0 - 100,
    )
    .map_err(|_| "the first full sync must proceed")?;

    // A local edit to the non-targeted A's body, then a targeted sync of B.
    std::fs::write(
        &a_path,
        "---\nstatus: ready\nexternal_id: \"ENG-1\"\n---\n\nEdited body\n",
    )?;
    let m1 = mtime_secs(&a_path)?;
    run_with(
        &spy,
        &tracker,
        dir.path(),
        &corpus,
        ItemSelection::Targeted(std::slice::from_ref(&corpus[1])),
        m1 + 100,
    )
    .map_err(|_| "the targeted sync must proceed")?;

    let report = run_with(
        &spy,
        &tracker,
        dir.path(),
        &corpus,
        ItemSelection::All,
        m1 + 200,
    )
    .map_err(|_| "the final full sync must proceed")?;

    let a = report
        .reported
        .iter()
        .find(|item| item.planned.id == "0001")
        .expect("A is reconciled by the final full sync");
    assert_eq!(
        a.planned.state,
        SyncState::LocallyModified,
        "the targeted run must not advance A's watermark past its edit, so \
         the later full sync still detects the local change"
    );
    Ok(())
}

#[test]
fn an_indeterminate_items_watermark_is_left_unadvanced() -> Result<(), TestError>
{
    let dir = tempfile::tempdir()?;
    let external = ExternalId::new("ENG-1".to_owned());
    let path = dir.path().join("0001.md");
    std::fs::write(&path, item_content(external.as_str()))?;

    let entry = "\"0001\":{\"remote_updated_at\":\"2026-06-01T00:00:00Z\",\"remote_hash\":\"h\",\"local_hash\":\"stale\",\"local_synced_at\":500}";
    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(&[entry.to_owned()]));
    let scenario = Scenario {
        items: vec![LocalItem {
            id: "0001".to_owned(),
            path,
            external_id: Some(external.clone()),
        }],
        tracker: RecordingTracker::truncating(Vec::new(), vec![external]),
        spy,
        dir,
    };

    let report = execute(&scenario, 25, 25, RunMode::Apply).map_err(|_| {
        "an indeterminate item is not a write, so bounds cannot refuse it"
    })?;
    assert_eq!(report.reported[0].planned.state, SyncState::Indeterminate);

    let written = scenario
        .spy
        .content(BASELINE_PATH)
        .expect("the run writes the baseline");
    let (baseline, _) = Baseline::read(Some(&written));
    assert_eq!(
        baseline.get("0001").expect("entry present").local_synced_at,
        500,
        "an indeterminate item keeps its watermark, so a later run still \
         detects a pre-existing local edit"
    );
    Ok(())
}

#[test]
fn a_targeted_run_reconciles_only_the_named_item() -> Result<(), TestError> {
    let scenario = scenario(3, 0)?;
    let seed = scenario
        .spy
        .content(BASELINE_PATH)
        .expect("the scenario seeds a baseline");
    let targeted = std::slice::from_ref(&scenario.items[1]);

    let report = execute_targeted(
        &scenario,
        targeted,
        SyncDirection::Bidirectional,
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "a single targeted pull within bounds must proceed")?;

    assert_eq!(report.reported.len(), 1, "only the named item is reported");
    assert_eq!(report.reported[0].planned.id, "0002");
    assert!(
        scenario.spy.content_of(&scenario.items[0].path).is_none(),
        "a non-targeted item must not be written"
    );
    assert!(
        scenario.spy.content_of(&scenario.items[2].path).is_none(),
        "a non-targeted item must not be written"
    );

    let written = scenario
        .spy
        .content(BASELINE_PATH)
        .expect("the run writes the baseline");
    let (seed_baseline, _) = Baseline::read(Some(&seed));
    let (written_baseline, _) = Baseline::read(Some(&written));
    assert_eq!(
        written_baseline.get("0001"),
        seed_baseline.get("0001"),
        "a non-targeted entry is left exactly as the last run set it"
    );
    assert_eq!(
        written_baseline.get("0003"),
        seed_baseline.get("0003"),
        "a non-targeted entry is left exactly as the last run set it"
    );
    Ok(())
}

#[test]
fn a_targeted_run_suppresses_discovery_and_never_searches(
) -> Result<(), TestError> {
    let scenario = scenario(2, 0)?;
    let targeted = std::slice::from_ref(&scenario.items[0]);

    let report = execute_targeted(
        &scenario,
        targeted,
        SyncDirection::Bidirectional,
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "a targeted run within bounds must proceed")?;

    assert_eq!(report.discovery, DiscoveryStatus::SkippedTargeted);
    assert!(
        !scenario
            .tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Search { .. })),
        "a targeted run must not search for untracked issues"
    );
    Ok(())
}

#[test]
fn a_targeted_push_only_run_reports_targeted_not_push_only(
) -> Result<(), TestError> {
    let scenario = scenario(0, 1)?;
    let targeted = std::slice::from_ref(&scenario.items[0]);

    let report = execute_targeted(
        &scenario,
        targeted,
        SyncDirection::PushOnly,
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "a targeted push-only run within bounds must proceed")?;

    assert_eq!(
        report.discovery,
        DiscoveryStatus::SkippedTargeted,
        "targeting takes precedence over the push-only skip"
    );
    Ok(())
}

#[test]
fn a_targeted_item_reconciles_exactly_as_it_would_in_a_full_sync(
) -> Result<(), TestError> {
    let full_scenario = scenario(2, 1)?;
    let full = execute(&full_scenario, 25, 25, RunMode::Apply)
        .map_err(|_| "the full sync within bounds must proceed")?;

    let targeted_scenario = scenario(2, 1)?;
    let targeted = std::slice::from_ref(&targeted_scenario.items[1]);
    let scoped = execute_targeted(
        &targeted_scenario,
        targeted,
        SyncDirection::Bidirectional,
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "the targeted sync within bounds must proceed")?;

    assert_eq!(
        project_item(&scoped, "0002"),
        project_item(&full, "0002"),
        "the named item reconciles identically whether targeted or in a \
         full sync"
    );
    Ok(())
}

#[test]
fn a_targeted_run_does_not_create_a_non_targeted_unsynced_draft(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let draft_path = dir.path().join("0001.md");
    std::fs::write(&draft_path, "---\nstatus: ready\n---\n\nDraft\n")?;
    let draft = LocalItem {
        id: "0001".to_owned(),
        path: draft_path,
        external_id: None,
    };
    let (pushable_item, issue, entry) = pushable(dir.path(), 100)?;

    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(&[entry]));
    let scenario = Scenario {
        items: vec![draft, pushable_item],
        tracker: RecordingTracker::holding(vec![issue]),
        spy,
        dir,
    };
    let targeted = std::slice::from_ref(&scenario.items[1]);

    execute_targeted(
        &scenario,
        targeted,
        SyncDirection::Bidirectional,
        25,
        25,
        RunMode::Apply,
    )
    .map_err(|_| "a targeted push within bounds must proceed")?;

    assert!(
        !scenario
            .tracker
            .calls()
            .iter()
            .any(|call| matches!(call, Call::Create { .. })),
        "a targeted run must not issue a non-targeted unsynced draft"
    );
    Ok(())
}

#[test]
fn the_fixtures_classify_as_intended() -> Result<(), TestError> {
    let scenario = scenario(2, 1)?;
    let report = execute(&scenario, 25, 25, RunMode::Preview)
        .map_err(|_| "preview must not refuse within the bounds")?;

    let pulls = report
        .reported
        .iter()
        .filter(|item| item.planned.action == work::sync::Action::Pull)
        .count();
    let pushes = report
        .reported
        .iter()
        .filter(|item| item.planned.action == work::sync::Action::Push)
        .count();

    assert_eq!(pulls, 2, "two items must decide Pull");
    assert_eq!(pushes, 1, "one item must decide Push");
    Ok(())
}

#[test]
fn a_full_sync_advances_every_present_entry_watermark_to_the_run_epoch(
) -> Result<(), TestError> {
    let scenario = scenario(2, 1)?;
    execute(&scenario, 25, 25, RunMode::Apply)
        .map_err(|_| "apply must not refuse within the bounds")?;

    let written = scenario
        .spy
        .content(BASELINE_PATH)
        .expect("the run writes the baseline");
    let occurrences = written.matches("\"local_synced_at\":1700000000").count();
    assert_eq!(
        occurrences, 3,
        "a full sync advances every present entry's watermark together, \
         exactly as the old global timestamp did"
    );
    Ok(())
}

#[test]
fn preview_reports_the_plan_without_writing_anything() -> Result<(), TestError>
{
    let scenario = scenario(2, 1)?;
    let before = scenario.spy.content(BASELINE_PATH);

    let report = execute(&scenario, 25, 25, RunMode::Preview)
        .map_err(|_| "preview must not refuse within the bounds")?;

    assert_eq!(report.reported.len(), 3);
    assert_eq!(
        scenario.spy.write_count(),
        0,
        "preview must perform no write at all, baseline included"
    );
    assert!(
        !report.finalised,
        "preview must not advance the baseline timestamp"
    );
    assert_eq!(
        scenario.spy.content(BASELINE_PATH),
        before,
        "preview must leave the baseline document byte-identical"
    );
    Ok(())
}

#[test]
fn a_plan_exactly_on_the_pull_bound_proceeds() -> Result<(), TestError> {
    let scenario = scenario(3, 0)?;

    let report = execute(&scenario, 3, 25, RunMode::Apply)
        .map_err(|_| "three pulls against a bound of three must proceed")?;

    assert_eq!(report.reported.len(), 3);
    assert!(report.finalised);
    Ok(())
}

#[test]
fn a_plan_one_over_the_pull_bound_is_refused() -> Result<(), TestError> {
    let scenario = scenario(3, 0)?;

    let error = execute(&scenario, 2, 25, RunMode::Apply)
        .err()
        .expect("four-over-bound must refuse");

    match error {
        RunError::Refused {
            pulls,
            pushes,
            max_pulls,
            max_pushes,
            ..
        } => {
            assert_eq!((pulls, max_pulls), (3, 2));
            assert_eq!((pushes, max_pushes), (0, 25));
        }
        RunError::Read(_)
        | RunError::Internal(_)
        | RunError::DiscoveryIncomplete { .. }
        | RunError::DiscoveryUnconfigured { .. } => {
            panic!("expected Refused, got a read or internal failure")
        }
    }
    Ok(())
}

#[test]
fn a_refusal_happens_before_any_write() -> Result<(), TestError> {
    let scenario = scenario(3, 0)?;
    let before = scenario.spy.content(BASELINE_PATH);

    let _ = execute(&scenario, 2, 25, RunMode::Apply)
        .err()
        .expect("must refuse");

    assert_eq!(
        scenario.spy.write_count(),
        0,
        "a refused run must not write, not even the baseline timestamp"
    );
    assert_eq!(scenario.spy.content(BASELINE_PATH), before);
    Ok(())
}

#[test]
fn a_zero_pull_bound_refuses_every_pull() -> Result<(), TestError> {
    let scenario = scenario(1, 0)?;

    let error = execute(&scenario, 0, 25, RunMode::Apply)
        .err()
        .expect("a zero bound must refuse a single pull");

    assert!(matches!(error, RunError::Refused { pulls: 1, .. }));
    assert_eq!(scenario.spy.write_count(), 0);
    Ok(())
}

#[test]
fn an_over_bound_push_count_is_refused() -> Result<(), TestError> {
    let scenario = scenario(0, 2)?;

    let error = execute(&scenario, 25, 1, RunMode::Apply)
        .err()
        .expect("two pushes against a bound of one must refuse");

    match error {
        RunError::Refused {
            pushes, max_pushes, ..
        } => assert_eq!((pushes, max_pushes), (2, 1)),
        RunError::Read(_)
        | RunError::Internal(_)
        | RunError::DiscoveryIncomplete { .. }
        | RunError::DiscoveryUnconfigured { .. } => {
            panic!("expected Refused, got a read or internal failure")
        }
    }
    assert_eq!(scenario.spy.write_count(), 0);
    Ok(())
}

#[test]
fn preview_refuses_an_over_bound_plan_rather_than_reporting_it(
) -> Result<(), TestError> {
    let scenario = scenario(3, 0)?;

    let error = execute(&scenario, 2, 25, RunMode::Preview)
        .err()
        .expect("preview must refuse an over-bound plan, not report it");

    assert!(matches!(error, RunError::Refused { pulls: 3, .. }));
    Ok(())
}

#[test]
fn an_applied_pull_writes_the_item_and_advances_the_baseline(
) -> Result<(), TestError> {
    let scenario = scenario(1, 0)?;
    let item_path = scenario.items[0].path.clone();

    let report = execute(&scenario, 25, 25, RunMode::Apply)
        .map_err(|_| "a single pull within bounds must proceed")?;

    assert!(report.finalised, "the run must finalise");
    assert!(
        scenario.spy.write_count() >= 1,
        "an applied pull must write the baseline"
    );

    // `pull` writes through the injected `AtomicWrite`, not `std::fs`, so
    // the written content lands in the spy rather than on disk.
    let written = scenario
        .spy
        .content_of(&item_path)
        .expect("the pulled item must have been written through the writer");
    assert!(
        written.contains("Remote body"),
        "the pulled file must carry the remote's projected body, got: {written}"
    );

    let baseline = scenario
        .spy
        .content(BASELINE_PATH)
        .expect("baseline must still exist");
    assert!(
        baseline.contains("\"timestamp\":1700000000"),
        "finalise must advance the baseline timestamp, got: {baseline}"
    );
    Ok(())
}

#[test]
fn an_unresolved_conflict_blanks_its_local_hash_and_still_finalises(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let external = ExternalId::new("ENG-9".to_owned());
    let path = dir.path().join("0009.md");
    std::fs::write(&path, item_content(external.as_str()))?;

    // Both sides changed: a stale local hash and a stale remote hash against
    // a stamp that has moved, so the 2x2 verdict is `conflict`, which
    // bidirectional decides `Prompt`.
    let entry = "\"0009\":{\"remote_updated_at\":\"2026-06-01T00:00:00Z\",\"remote_hash\":\"stale\",\"local_hash\":\"stale\"}";
    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(&[entry.to_owned()]));

    let scenario = Scenario {
        items: vec![LocalItem {
            id: "0009".to_owned(),
            path,
            external_id: Some(external.clone()),
        }],
        tracker: RecordingTracker::holding(vec![(
            external,
            RemoteIssue {
                updated: RemoteTimestamp::Reported(MOVED_STAMP.to_owned()),
                body: projected_body().to_owned(),
            },
        )]),
        spy,
        dir,
    };

    let report = execute(&scenario, 25, 25, RunMode::Apply)
        .map_err(|_| "a conflict is not a write, so bounds cannot refuse it")?;

    assert_eq!(report.reported.len(), 1);
    assert_eq!(
        report.reported[0].planned.action,
        work::sync::Action::Prompt
    );
    assert_eq!(
        report.awaiting_human().count(),
        1,
        "an unresolved conflict must be reported as awaiting a human"
    );
    assert!(report.finalised);

    let baseline = scenario
        .spy
        .content(BASELINE_PATH)
        .expect("baseline must still exist");
    assert!(
        baseline.contains("\"local_hash\":\"\""),
        "an unreconciled conflict must have its local_hash blanked, got: {baseline}"
    );
    assert!(
        baseline.contains("\"timestamp\":1700000000"),
        "the timestamp must still advance alongside the blank"
    );
    Ok(())
}

fn conflict_content(index: usize, title: &str, body: &str) -> String {
    format!(
        "---\nstatus: ready\ntitle: {title}\nexternal_id: \"ENG-{index}\"\n---\n\n{body}"
    )
}

/// A conflicting item: both sides changed against a moved stamp, so it
/// classifies `conflict`, which bidirectional decides `Prompt`. The local
/// file carries `local_body`; the remote issue carries `remote_body`.
fn conflicting(
    dir: &Path,
    index: usize,
    title: &str,
    local_body: &str,
    remote_body: &str,
) -> Result<(LocalItem, (ExternalId, RemoteIssue), String), TestError> {
    let id = format!("{index:04}");
    let external = ExternalId::new(format!("ENG-{index}"));
    let path = dir.join(format!("{id}.md"));
    std::fs::write(&path, conflict_content(index, title, local_body))?;

    let entry = format!(
        "\"{id}\":{{\"remote_updated_at\":\"{STAMP}\",\"remote_hash\":\"stale\",\"local_hash\":\"stale\"}}"
    );
    let issue = RemoteIssue {
        updated: RemoteTimestamp::Reported(MOVED_STAMP.to_owned()),
        body: remote_body.to_owned(),
    };
    Ok((
        LocalItem {
            id,
            path,
            external_id: Some(external.clone()),
        },
        (external, issue),
        entry,
    ))
}

#[test]
fn a_two_conflict_corpus_builds_a_dossier_per_item_with_bound_values(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let (item_a, issue_a, entry_a) = conflicting(
        dir.path(),
        1,
        "Item one",
        "## Summary\nlocal summary\n## Requirements\nlocal reqs\n",
        "## Summary\nremote summary\n## Requirements\nremote reqs\n",
    )?;
    let (item_b, issue_b, entry_b) = conflicting(
        dir.path(),
        2,
        "Item two",
        "## Summary\nlocal only\n",
        "## Summary\nremote only\n",
    )?;

    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(&[entry_a, entry_b]));
    let scenario = Scenario {
        items: vec![item_a, item_b],
        tracker: RecordingTracker::holding(vec![issue_a, issue_b]),
        spy,
        dir,
    };

    let report = execute(&scenario, 25, 25, RunMode::Apply)
        .map_err(|_| "a conflict is not a write, so bounds cannot refuse it")?;

    assert_eq!(report.dossiers.len(), 2, "one dossier per Prompt item");
    let ids: BTreeSet<&str> =
        report.dossiers.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["0001", "0002"].into_iter().collect());

    let multi = report
        .dossiers
        .iter()
        .find(|d| d.id == "0001")
        .expect("the multi-section conflict has a dossier");
    assert!(!multi.local_unreadable);
    assert!(multi.local_modified.is_some(), "a real file has an mtime");
    assert!(
        matches!(multi.remote_updated, RemoteTimestamp::Reported(ref s) if s == MOVED_STAMP),
        "the remote stamp is carried through"
    );
    assert!(!multi.title.is_empty(), "the title is carried through");

    let names: Vec<&str> =
        multi.sections.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Summary"), "sections: {names:?}");
    assert!(names.contains(&"Requirements"), "sections: {names:?}");

    let summary = multi
        .sections
        .iter()
        .find(|s| s.name == "Summary")
        .expect("the Summary section differs");
    assert_eq!(
        summary.local, "local summary\n",
        "the local side is bound to the seeded local body"
    );
    assert_eq!(
        summary.remote, "remote summary\n",
        "the remote side is bound to the seeded remote body"
    );

    let single = report
        .dossiers
        .iter()
        .find(|d| d.id == "0002")
        .expect("the single-section conflict has a dossier");
    assert_eq!(single.sections.len(), 1);
    Ok(())
}

#[test]
fn a_prompt_item_with_an_unreadable_local_file_is_marked_local_unreadable(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let external = ExternalId::new("ENG-9".to_owned());
    // A directory cannot be read as a file, so the dossier read fails at the
    // boundary. A blank baseline `local_hash` lets `classify` decide
    // `conflict` without ever reading the local file, so the item still
    // reaches the `Prompt` arm.
    let path = dir.path().join("0009.md");
    std::fs::create_dir(&path)?;

    let entry = "\"0009\":{\"remote_updated_at\":\"2026-06-01T00:00:00Z\",\"remote_hash\":\"stale\",\"local_hash\":\"\"}";
    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(&[entry.to_owned()]));

    let scenario = Scenario {
        items: vec![LocalItem {
            id: "0009".to_owned(),
            path,
            external_id: Some(external.clone()),
        }],
        tracker: RecordingTracker::holding(vec![(
            external,
            RemoteIssue {
                updated: RemoteTimestamp::Reported(MOVED_STAMP.to_owned()),
                body: projected_body().to_owned(),
            },
        )]),
        spy,
        dir,
    };

    let report = execute(&scenario, 25, 25, RunMode::Apply)
        .map_err(|_| "a blank baseline local_hash needs no local read")?;

    assert_eq!(
        report.reported[0].planned.action,
        work::sync::Action::Prompt,
        "the item still awaits a human"
    );
    assert_eq!(report.dossiers.len(), 1);
    let dossier = &report.dossiers[0];
    assert!(
        dossier.local_unreadable,
        "the unreadable local file is flagged"
    );
    assert!(
        dossier.sections.is_empty(),
        "no sections are fabricated from an unreadable local side"
    );
    assert_eq!(dossier.local_modified, None);
    Ok(())
}

#[test]
fn preview_lists_every_action_and_validates_push_entries_only(
) -> Result<(), TestError> {
    let scenario = scenario(2, 1)?;

    let report = execute(&scenario, 25, 25, RunMode::Preview)
        .map_err(|_| "preview must not refuse within the bounds")?;

    assert_eq!(
        report.reported.len(),
        3,
        "the preview report must not shrink below the whole plan"
    );
    for item in &report.reported {
        let is_push = item.planned.action == work::sync::Action::Push;
        assert_eq!(
            item.validation.is_some(),
            is_push,
            "only push entries carry a validation outcome: {:?}",
            item.planned
        );
    }
    Ok(())
}

#[test]
fn preview_validation_rejects_a_locally_missing_field_without_a_remote_call(
) -> Result<(), TestError> {
    // The pushable fixture's frontmatter carries no `title`, so the composed
    // update payload leaves the required field empty and the local check
    // rejects it.
    let scenario = scenario(0, 1)?;

    let report = execute(&scenario, 25, 25, RunMode::Preview)
        .map_err(|_| "preview must not refuse within the bounds")?;

    let push = report
        .reported
        .iter()
        .find(|item| item.planned.action == work::sync::Action::Push)
        .expect("one push entry");
    assert!(
        matches!(
            push.validation,
            Some(tracker::ValidationOutcome::Rejected { .. })
        ),
        "a missing required field must be rejected, got {:?}",
        push.validation
    );

    let mutated = scenario.tracker.calls().iter().any(|call| {
        matches!(
            call,
            tracker_test_support::Call::Update { .. }
                | tracker_test_support::Call::Create { .. }
        )
    });
    assert!(
        !mutated,
        "validate_update must make no mutating remote call"
    );
    Ok(())
}

#[test]
fn a_dirty_remotely_modified_item_is_not_pulled_and_its_file_is_untouched(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let (item, issue, entry) = pullable(dir.path(), 1)?;
    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(&[entry]));
    let tracker = RecordingTracker::holding(vec![issue]);
    let clock = FixedClock(1_700_000_000);
    let status = AlwaysDirty;
    let author = UnusedAuthor;
    let ports = SyncPorts {
        tracker: &tracker,
        status: &status,
        writer: &spy,
        clock: &clock,
        author: &author,
    };
    let mut store =
        BaselineStore::new(PathBuf::from(BASELINE_PATH), &spy, &spy);
    let resolutions = BTreeMap::new();
    let integrations_root = dir.path().to_path_buf();

    let report = run(
        &ports,
        &mut store,
        &request(
            std::slice::from_ref(&item),
            ItemSelection::All,
            &resolutions,
            &integrations_root,
            25,
            25,
            RunMode::Apply,
        ),
    )
    .map_err(|_| "a dirty conflict is not a write, so bounds cannot refuse")?;

    assert_eq!(
        report.reported[0].planned.action,
        work::sync::Action::Prompt,
        "a dirty remotely-modified item must decide Prompt, not Pull"
    );
    assert!(
        !spy.writes.borrow().iter().any(|path| path == &item.path),
        "the dirty guard must leave the item's file unwritten"
    );
    assert_eq!(report.awaiting_human().count(), 1);
    Ok(())
}

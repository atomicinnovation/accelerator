//! The whole-corpus run: the write bounds, preview's no-write guarantee,
//! and the finalise bookkeeping.
//!
//! Items live on real files because `run` and `LazyItemDigests` read them
//! through `std::fs`; only the baseline goes through the injected reader
//! and writer, which is what lets a spy count writes.
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
use tracker_test_support::RecordingTracker;
use work::sync::Dirtiness;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::digest;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::run::run;
use work_adapters::sync::run::RunError;
use work_adapters::sync::run::RunMode;
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
    _dir: tempfile::TempDir,
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
        _dir: dir,
    })
}

const fn request<'a>(
    items: &'a [LocalItem],
    resolutions: &'a BTreeMap<String, Resolution>,
    max_pulls: usize,
    max_pushes: usize,
    mode: RunMode,
) -> SyncRequest<'a> {
    SyncRequest {
        items,
        direction: SyncDirection::Bidirectional,
        strategy: RetrievalStrategy::Bulk,
        resolutions,
        max_pulls,
        max_pushes,
        mode,
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
    let ports = SyncPorts {
        tracker: &scenario.tracker,
        status: &status,
        writer: &scenario.spy,
        clock: &clock,
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
        &request(&scenario.items, &resolutions, max_pulls, max_pushes, mode),
    )
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
        } => {
            assert_eq!((pulls, max_pulls), (3, 2));
            assert_eq!((pushes, max_pushes), (0, 25));
        }
        RunError::Read(_) | RunError::Internal(_) => {
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
        RunError::Read(_) | RunError::Internal(_) => {
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
        _dir: dir,
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

//! A run moving baseline entries from the legacy digest recipe to recipe 1:
//! every synced legacy entry is rewritten without a file write or a tracker
//! mutation, an entry the run does not settle keeps its recipe, and a file
//! an earlier pull left raw tracker content in is a conflict, not a
//! migration.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;
use corpus_adapters::CommonMarkBody;
use jira_client::JiraBodyDialect;
use tracker::BodyDialect;
use tracker::ExternalId;
use tracker::PlainDialect;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::TrackerError;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use work::sync::classify;
use work::sync::Action;
use work::sync::BaselineEntry;
use work::sync::BaselineFingerprints;
use work::sync::BodyEquivalence;
use work::sync::Dirtiness;
use work::sync::FieldsComparison;
use work::sync::ItemComparison;
use work::sync::ItemDigests;
use work::sync::LegacyHashes;
use work::sync::LocalFingerprints;
use work::sync::PullResidue;
use work::sync::RemoteFingerprints;
use work::sync::RemotePresence;
use work::sync::Resolution;
use work::sync::Subject;
use work::sync::SyncDirection;
use work::sync::SyncState;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::digest;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::run::run;
use work_adapters::sync::run::ItemSelection;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;

use crate::support::assert_recipe_one_entry;

type TestError = Box<dyn std::error::Error>;

const BASELINE_PATH: &str = "/baseline/last-sync.json";
const STAMP: &str = "2026-06-01T00:00:00Z";
const MOVED_STAMP: &str = "2026-07-01T00:00:00Z";

const JIRA_CURRENT: &str = "{\"content\":[{\"content\":[{\"text\":\
    \"Current\",\"type\":\"text\"}],\"type\":\"paragraph\"}],\
    \"type\":\"doc\",\"version\":1}";
const JIRA_EARLIER: &str = "{\"content\":[{\"content\":[{\"text\":\
    \"Earlier\",\"type\":\"text\"}],\"type\":\"paragraph\"}],\
    \"type\":\"doc\",\"version\":1}";
const JIRA_MENTION: &str = "{\"content\":[{\"content\":[{\
    \"attrs\":{\"id\":\"u1\",\"text\":\"@someone\"},\"type\":\"mention\"}],\
    \"type\":\"paragraph\"}],\"type\":\"doc\",\"version\":1}";

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

    fn content(&self, path: &str) -> Option<String> {
        self.files
            .borrow()
            .get(Path::new(path))
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    fn writes(&self) -> Vec<PathBuf> {
        self.writes.borrow().clone()
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

struct Status(Dirtiness);

impl WorkingCopyStatus for Status {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        self.0
    }
}

struct UnusedAuthor;

impl work_adapters::sync::create::LocalAuthor for UnusedAuthor {
    fn author_from_remote(
        &self,
        _issue: &work_adapters::sync::create::DiscoveredIssue,
    ) -> Result<work_adapters::sync::create::AuthoredLocal, kernel::Error> {
        panic!("no scenario in this suite authors a discovered issue")
    }

    fn link_external_id(
        &self,
        _path: &Path,
        _external_id: &ExternalId,
    ) -> Result<(), kernel::Error> {
        panic!("no scenario in this suite links an external id")
    }
}

struct FixedClock;

impl work::sync::RunClock for FixedClock {
    fn run_start_epoch(&self) -> Result<u64, kernel::Error> {
        Ok(1_700_000_000)
    }
}

/// One item: its local file, the remote the tracker holds, and the
/// baseline entry seeded for it.
struct Seed {
    index: usize,
    title: &'static str,
    local_body: String,
    remote: Option<(String, &'static str)>,
    entry: SeedEntry,
}

enum SeedEntry {
    Raw(&'static str),
    Legacy {
        local_content: String,
        projection: String,
    },
    Split {
        local_content: String,
        projection: String,
    },
}

fn content(index: usize, title: &str, body: &str) -> String {
    format!(
        "---\nstatus: ready\ntitle: {title}\nexternal_id: \"ENG-{index}\"\n\
         ---\n\n{body}"
    )
}

impl Seed {
    fn id(&self) -> String {
        format!("{:04}", self.index)
    }

    fn external(&self) -> ExternalId {
        ExternalId::new(format!("ENG-{}", self.index))
    }

    fn content(&self) -> String {
        content(self.index, self.title, &self.local_body)
    }

    /// An item whose legacy entry recorded exactly the file and remote it
    /// has now.
    fn synced_legacy(
        index: usize,
        title: &'static str,
        local_body: &str,
        projection: &str,
    ) -> Self {
        Self {
            index,
            title,
            local_body: local_body.to_owned(),
            remote: Some((projection.to_owned(), STAMP)),
            entry: SeedEntry::Legacy {
                local_content: content(index, title, local_body),
                projection: projection.to_owned(),
            },
        }
    }
}

struct Harness {
    dir: tempfile::TempDir,
    items: Vec<LocalItem>,
    tracker: RecordingTracker,
    spy: Spy,
    seeded: String,
}

fn legacy_fragment(id: &str, local_content: &str, projection: &str) -> String {
    support::entry_fragment(
        id,
        Entry::persisted_legacy(
            RemoteTimestamp::Reported(STAMP.to_owned()),
            digest::remote_body(projection),
            digest::local(local_content).expect("a work item"),
            0,
        ),
    )
}

fn harness(
    dialect: &dyn BodyDialect,
    jira: bool,
    seeds: &[Seed],
    configure: impl FnOnce(RecordingTracker) -> RecordingTracker,
) -> Result<Harness, TestError> {
    let dir = tempfile::tempdir()?;
    let mut items = Vec::new();
    let mut issues = Vec::new();
    let mut entries = Vec::new();
    for seed in seeds {
        let path = dir.path().join(format!("{}.md", seed.id()));
        std::fs::write(&path, seed.content())?;
        items.push(LocalItem {
            id: seed.id(),
            path,
            external_id: Some(seed.external()),
        });
        if let Some((projection, stamp)) = &seed.remote {
            issues.push((
                seed.external(),
                RemoteIssue {
                    key: seed.external(),
                    updated: RemoteTimestamp::Reported((*stamp).to_owned()),
                    body: projection.clone(),
                },
            ));
        }
        match &seed.entry {
            SeedEntry::Raw(entry) => {
                entries.push(format!("\"{}\":{entry}", seed.id()));
            }
            SeedEntry::Legacy {
                local_content,
                projection,
            } => entries.push(legacy_fragment(
                &seed.id(),
                local_content,
                projection,
            )),
            SeedEntry::Split {
                local_content,
                projection,
            } => entries.push(support::split_entry(
                &seed.id(),
                dialect,
                local_content,
                projection,
                STAMP,
            )),
        }
    }
    let seeded =
        format!("{{\"timestamp\":0,\"items\":{{{}}}}}\n", entries.join(","));
    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &seeded);
    let tracker = if jira {
        support::jira_recording_tracker(issues)
    } else {
        RecordingTracker::holding(issues)
    };
    Ok(Harness {
        dir,
        items,
        tracker: configure(tracker),
        spy,
        seeded,
    })
}

struct Run<'a> {
    direction: SyncDirection,
    mode: RunMode,
    dirty: Dirtiness,
    resolutions: &'a BTreeMap<String, Resolution>,
}

const fn applying(resolutions: &BTreeMap<String, Resolution>) -> Run<'_> {
    Run {
        direction: SyncDirection::Bidirectional,
        mode: RunMode::Apply,
        dirty: Dirtiness::Clean,
        resolutions,
    }
}

fn sync(harness: &Harness, settings: &Run<'_>) -> RunReport {
    let status = Status(settings.dirty);
    let author = UnusedAuthor;
    let ports = SyncPorts {
        tracker: &harness.tracker,
        parser: &CommonMarkBody,
        status: &status,
        files: support::sync_files(&harness.spy),
        clock: &FixedClock,
        author: &author,
    };
    let mut store = BaselineStore::new(
        PathBuf::from(BASELINE_PATH),
        &harness.spy,
        &harness.spy,
    );
    run(
        &ports,
        &mut store,
        &SyncRequest {
            corpus: &harness.items,
            selection: ItemSelection::All,
            direction: settings.direction,
            strategy: RetrievalStrategy::Bulk,
            resolutions: settings.resolutions,
            max_pulls: tracker::Ceiling::Bounded(25),
            max_pushes: tracker::Ceiling::Bounded(25),
            mode: settings.mode,
            integrations_root: harness.dir.path(),
            integration: "jira",
            promote: true,
            scope: tracker::SearchScope::default(),
        },
    )
    .unwrap_or_else(|_| panic!("the run proceeds"))
}

fn sync_once(harness: &Harness) -> RunReport {
    sync(harness, &applying(&BTreeMap::new()))
}

fn entry(harness: &Harness, id: &str) -> Option<Entry> {
    Baseline::read(harness.spy.content(BASELINE_PATH).as_deref())
        .0
        .get(id)
        .cloned()
}

fn seeded_entry(harness: &Harness, id: &str) -> Option<Entry> {
    Baseline::read(Some(&harness.seeded)).0.get(id).cloned()
}

fn row<'a>(
    report: &'a RunReport,
    id: &str,
) -> &'a work_adapters::sync::run::ReportedItem {
    report
        .reported
        .iter()
        .find(|item| item.planned.id == id)
        .unwrap_or_else(|| panic!("{id} is reported"))
}

fn mutated(tracker: &RecordingTracker) -> bool {
    tracker.calls().iter().any(|call| {
        matches!(
            call,
            Call::Update { .. } | Call::Retitle { .. } | Call::Create { .. }
        )
    })
}

fn wrote_only_the_baseline(harness: &Harness) -> bool {
    harness
        .spy
        .writes()
        .iter()
        .all(|path| path == Path::new(BASELINE_PATH))
}

fn files_unchanged(harness: &Harness, seeds: &[Seed]) -> bool {
    harness.items.iter().zip(seeds).all(|(item, seed)| {
        std::fs::read_to_string(&item.path).ok() == Some(seed.content())
    })
}

fn two_synced_linear_items() -> Vec<Seed> {
    vec![
        Seed::synced_legacy(
            1,
            "One",
            "# One\n\nBody one\n",
            "One\n# One\n\nBody one\n",
        ),
        Seed::synced_legacy(
            2,
            "Two",
            "# Two\n\n* item\n",
            "Two\n# Two\n\n- item\n",
        ),
    ]
}

#[test]
fn a_run_rewrites_every_classified_legacy_entry_under_recipe_one(
) -> Result<(), TestError> {
    let seeds = two_synced_linear_items();
    let harness = harness(&PlainDialect, false, &seeds, |tracker| tracker)?;

    let report = sync_once(&harness);

    for seed in &seeds {
        let reported = row(&report, &seed.id());
        assert_eq!(reported.planned.state, SyncState::Synced);
        assert_eq!(reported.planned.action, Action::RecordBaseline);
        assert_recipe_one_entry(
            entry(&harness, &seed.id()).as_ref(),
            &PlainDialect,
            &seed.content(),
            seed.remote
                .as_ref()
                .map(|(projection, _)| projection.as_str()),
        );
    }
    Ok(())
}

#[test]
fn rewriting_a_synced_legacy_entry_writes_no_file_and_calls_no_mutation(
) -> Result<(), TestError> {
    let seeds = two_synced_linear_items();
    let harness = harness(&PlainDialect, false, &seeds, |tracker| tracker)?;

    sync_once(&harness);

    assert!(
        wrote_only_the_baseline(&harness),
        "{:?}",
        harness.spy.writes()
    );
    assert!(files_unchanged(&harness, &seeds));
    assert!(!mutated(&harness.tracker));
    Ok(())
}

#[test]
fn a_preview_leaves_last_sync_json_byte_identical() -> Result<(), TestError> {
    let seeds = two_synced_linear_items();
    let harness = harness(&PlainDialect, false, &seeds, |tracker| tracker)?;

    sync(
        &harness,
        &Run {
            mode: RunMode::Preview,
            ..applying(&BTreeMap::new())
        },
    );

    assert!(harness.spy.writes().is_empty());
    assert_eq!(
        harness.spy.content(BASELINE_PATH).as_deref(),
        Some(harness.seeded.as_str())
    );
    Ok(())
}

fn assert_legacy(harness: &Harness, id: &str) {
    let entry = entry(harness, id);
    assert!(
        matches!(entry, Some(Entry::Legacy { .. })),
        "{id} keeps the legacy recipe: {entry:?}"
    );
}

#[test]
fn entries_this_run_does_not_settle_keep_the_legacy_recipe(
) -> Result<(), TestError> {
    let unresolved = |index| Seed {
        index,
        title: "Conflict",
        local_body: "Edited locally\n".to_owned(),
        remote: Some(("Conflict\nEdited remotely\n".to_owned(), MOVED_STAMP)),
        entry: SeedEntry::Legacy {
            local_content: content(index, "Conflict", "Recorded\n"),
            projection: "Conflict\nRecorded\n".to_owned(),
        },
    };
    let locally_modified = |index| Seed {
        index,
        title: "Local",
        local_body: "Edited locally\n".to_owned(),
        remote: Some(("Local\nRecorded\n".to_owned(), STAMP)),
        entry: SeedEntry::Legacy {
            local_content: content(index, "Local", "Recorded\n"),
            projection: "Local\nRecorded\n".to_owned(),
        },
    };
    let remotely_modified = |index| Seed {
        index,
        title: "Remote",
        local_body: "Recorded\n".to_owned(),
        remote: Some(("Remote\nEdited remotely\n".to_owned(), MOVED_STAMP)),
        entry: SeedEntry::Legacy {
            local_content: content(index, "Remote", "Recorded\n"),
            projection: "Remote\nRecorded\n".to_owned(),
        },
    };
    let absent = Seed {
        index: 1,
        title: "Gone",
        local_body: "Body\n".to_owned(),
        remote: None,
        entry: SeedEntry::Legacy {
            local_content: content(1, "Gone", "Body\n"),
            projection: "Gone\nBody\n".to_owned(),
        },
    };
    let indeterminate =
        Seed::synced_legacy(2, "Unread", "Body\n", "Unread\nBody\n");
    let bidirectional = harness(
        &PlainDialect,
        false,
        &[absent, indeterminate, unresolved(3)],
        |tracker| {
            tracker.failing_show(
                ExternalId::new("ENG-2".to_owned()),
                TrackerError::Retryable {
                    detail: "timed out".to_owned(),
                },
            )
        },
    )?;
    let pull_only =
        harness(&PlainDialect, false, &[locally_modified(4)], |tracker| {
            tracker
        })?;
    let push_only =
        harness(&PlainDialect, false, &[remotely_modified(5)], |tracker| {
            tracker
        })?;
    let dirty =
        harness(&PlainDialect, false, &[remotely_modified(6)], |tracker| {
            tracker
        })?;
    let none = BTreeMap::new();

    let report = sync_once(&bidirectional);
    sync(
        &pull_only,
        &Run {
            direction: SyncDirection::PullOnly,
            ..applying(&none)
        },
    );
    sync(
        &push_only,
        &Run {
            direction: SyncDirection::PushOnly,
            ..applying(&none)
        },
    );
    sync(
        &dirty,
        &Run {
            dirty: Dirtiness::Dirty,
            ..applying(&none)
        },
    );

    assert_eq!(row(&report, "0001").planned.state, SyncState::RemoteAbsent);
    assert_eq!(row(&report, "0002").planned.state, SyncState::Indeterminate);
    assert_eq!(row(&report, "0003").planned.state, SyncState::Conflict);
    for id in ["0001", "0002", "0003"] {
        assert_legacy(&bidirectional, id);
    }
    assert_legacy(&pull_only, "0004");
    assert_legacy(&push_only, "0005");
    assert_legacy(&dirty, "0006");
    Ok(())
}

#[test]
fn a_legacy_entry_whose_forced_show_fails_keeps_its_bytes(
) -> Result<(), TestError> {
    let seeds = [Seed::synced_legacy(2, "Unread", "Body\n", "Unread\nBody\n")];
    let harness = harness(&PlainDialect, false, &seeds, |tracker| {
        tracker.failing_show(
            ExternalId::new("ENG-2".to_owned()),
            TrackerError::Retryable {
                detail: "timed out".to_owned(),
            },
        )
    })?;

    sync_once(&harness);

    assert_eq!(entry(&harness, "0002"), seeded_entry(&harness, "0002"));
    Ok(())
}

#[test]
fn a_remote_updated_at_change_alone_leaves_the_hashes_unchanged(
) -> Result<(), TestError> {
    let body = "# Item\n\nBody\n";
    let projection = "Item\n# Item\n\nBody\n";
    let seeds = [Seed {
        index: 1,
        title: "Item",
        local_body: body.to_owned(),
        remote: Some((projection.to_owned(), MOVED_STAMP)),
        entry: SeedEntry::Split {
            local_content: content(1, "Item", body),
            projection: projection.to_owned(),
        },
    }];
    let harness = harness(&PlainDialect, false, &seeds, |tracker| tracker)?;

    let report = sync_once(&harness);

    assert_eq!(row(&report, "0001").planned.state, SyncState::Synced);
    assert_eq!(row(&report, "0001").planned.action, Action::Noop);
    let (before, after) =
        (seeded_entry(&harness, "0001"), entry(&harness, "0001"));
    assert_eq!(
        after.as_ref().and_then(Entry::split),
        before.as_ref().and_then(Entry::split)
    );
    assert_eq!(
        after.as_ref().and_then(Entry::legacy),
        before.as_ref().and_then(Entry::legacy)
    );
    Ok(())
}

/// The digests a binary that knows only the legacy recipe computes.
struct LegacyReader {
    local: String,
    remote: String,
}

impl ItemDigests for LegacyReader {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
        Ok(None)
    }

    fn legacy_local(&self) -> Result<String, kernel::Error> {
        digest::local(&self.local)
    }

    fn legacy_remote(&self) -> Result<Option<String>, kernel::Error> {
        Ok(Some(digest::remote_body(&self.remote)))
    }

    fn local(&self) -> Result<LocalFingerprints, kernel::Error> {
        panic!("a legacy reader has no split fingerprints")
    }

    fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error> {
        panic!("a legacy reader has no split fingerprints")
    }
}

impl ItemComparison for LegacyReader {
    fn pull_residue(&self) -> Option<PullResidue> {
        None
    }

    fn body_equivalence(&self) -> BodyEquivalence {
        BodyEquivalence::Unknown
    }

    fn fields_comparison(&self) -> FieldsComparison {
        FieldsComparison::Unknown
    }
}

#[test]
fn a_recipe_one_entry_classifies_as_synced_under_the_legacy_reader(
) -> Result<(), TestError> {
    let local = content(1, "Item", "# Item\n\n* one\n");
    let remote = "Item\n# Item\n\n- one\n".to_owned();
    let fragment =
        support::split_entry("0001", &PlainDialect, &local, &remote, STAMP);
    let rendered: serde_json::Value =
        serde_json::from_str(&format!("{{{fragment}}}"))?;
    let legacy_view = &rendered["0001"];
    let stamp = RemoteTimestamp::Reported(
        legacy_view["remote_updated_at"]
            .as_str()
            .ok_or("a stamp")?
            .to_owned(),
    );
    let text = |key: &str| legacy_view[key].as_str().map(str::to_owned);
    let external = ExternalId::new("ENG-1".to_owned());
    let reader = LegacyReader { local, remote };

    let classification = classify(
        &Subject {
            external_id: Some(&external),
            presence: RemotePresence::Present,
            remote_updated: &RemoteTimestamp::Reported(MOVED_STAMP.to_owned()),
            baseline_timestamp: 0,
        },
        &BaselineEntry {
            remote_updated_at: &stamp,
            fingerprints: BaselineFingerprints::Legacy(LegacyHashes {
                local: text("local_hash"),
                remote: text("remote_hash"),
            }),
        },
        &reader,
        &reader,
    )?;

    assert_eq!(classification.state(), SyncState::Synced);
    Ok(())
}

#[test]
fn a_legacy_linear_file_carrying_a_title_line_migrates() -> Result<(), TestError>
{
    let projection = "Item\n# Item\n\nBody\n";
    let seeds = [Seed::synced_legacy(1, "Item", projection, projection)];
    let harness = harness(&PlainDialect, false, &seeds, |tracker| tracker)?;

    let report = sync_once(&harness);

    assert_eq!(row(&report, "0001").planned.action, Action::RecordBaseline);
    assert!(files_unchanged(&harness, &seeds));
    assert_recipe_one_entry(
        entry(&harness, "0001").as_ref(),
        &PlainDialect,
        &seeds[0].content(),
        Some(projection),
    );
    Ok(())
}

fn jira_projection(description: &str) -> String {
    format!("Title\n{description}\n")
}

#[test]
fn a_legacy_jira_file_holding_adf_json_is_a_conflict_not_a_migration(
) -> Result<(), TestError> {
    for local_body in [
        format!("Title\n{JIRA_EARLIER}\n"),
        format!("{JIRA_EARLIER}\n"),
    ] {
        let seeds = [Seed::synced_legacy(
            1,
            "Title",
            &local_body,
            &jira_projection(JIRA_CURRENT),
        )];
        let harness =
            harness(&JiraBodyDialect, true, &seeds, |tracker| tracker)?;

        let report = sync_once(&harness);

        assert_eq!(
            row(&report, "0001").planned.state,
            SyncState::Conflict,
            "{local_body}"
        );
        assert_legacy(&harness, "0001");
        assert!(files_unchanged(&harness, &seeds));
        assert!(!mutated(&harness.tracker));
    }
    Ok(())
}

#[test]
fn an_untouched_verbatim_jira_pull_migrates_without_a_write(
) -> Result<(), TestError> {
    let projection = jira_projection(JIRA_MENTION);
    let seeds = [Seed::synced_legacy(1, "Title", &projection, &projection)];
    let harness = harness(&JiraBodyDialect, true, &seeds, |tracker| tracker)?;

    let first = sync_once(&harness);
    let second = sync_once(&harness);

    assert_eq!(row(&first, "0001").planned.action, Action::RecordBaseline);
    assert_recipe_one_entry(
        entry(&harness, "0001").as_ref(),
        &JiraBodyDialect,
        &seeds[0].content(),
        Some(&projection),
    );
    assert_eq!(row(&second, "0001").planned.state, SyncState::Synced);
    assert_eq!(row(&second, "0001").planned.action, Action::Noop);
    assert!(files_unchanged(&harness, &seeds));
    assert!(!mutated(&harness.tracker));
    Ok(())
}

#[test]
fn a_skipped_residue_conflict_is_still_a_conflict_on_the_next_run(
) -> Result<(), TestError> {
    let seeds = [Seed::synced_legacy(
        1,
        "Title",
        &format!("Title\n{JIRA_EARLIER}\n"),
        &jira_projection(JIRA_CURRENT),
    )];
    let harness = harness(&JiraBodyDialect, true, &seeds, |tracker| tracker)?;

    let first = sync_once(&harness);
    let second = sync_once(&harness);

    for report in [&first, &second] {
        assert_eq!(row(report, "0001").planned.state, SyncState::Conflict);
    }
    assert!(!mutated(&harness.tracker), "the second run sends nothing");
    assert_legacy(&harness, "0001");
    Ok(())
}

#[test]
fn a_skipped_verbatim_conflict_with_a_title_edit_is_still_a_conflict(
) -> Result<(), TestError> {
    let projection = jira_projection(JIRA_MENTION);
    let seeds = [Seed {
        index: 1,
        title: "Retitled locally",
        local_body: projection.clone(),
        remote: Some((projection.clone(), STAMP)),
        entry: SeedEntry::Legacy {
            local_content: content(1, "Title", &projection),
            projection: projection.clone(),
        },
    }];
    let harness = harness(&JiraBodyDialect, true, &seeds, |tracker| tracker)?;

    let first = sync_once(&harness);
    let second = sync_once(&harness);

    for report in [&first, &second] {
        assert_eq!(row(report, "0001").planned.state, SyncState::Conflict);
    }
    assert!(!mutated(&harness.tracker), "the title is never sent");
    assert_legacy(&harness, "0001");
    Ok(())
}

#[test]
fn an_item_with_an_unknown_recipe_entry_is_indeterminate_and_untouched(
) -> Result<(), TestError> {
    let raw = "{\"digest_recipe\":2,\"local_hash\":\"l\",\
               \"local_synced_at\":5,\"remote_updated_at\":\"t\"}";
    let seeds = [Seed {
        index: 1,
        title: "Newer",
        local_body: "Edited locally\n".to_owned(),
        remote: Some(("Newer\nEdited remotely\n".to_owned(), MOVED_STAMP)),
        entry: SeedEntry::Raw(raw),
    }];
    let harness = harness(&PlainDialect, false, &seeds, |tracker| tracker)?;

    let report = sync_once(&harness);

    let reported = row(&report, "0001");
    assert_eq!(reported.planned.state, SyncState::Indeterminate);
    assert_eq!(reported.planned.action, Action::Noop);
    assert!(matches!(
        reported.outcome,
        work_adapters::sync::run::ItemOutcome::UnsupportedRecipe
    ));
    assert!(!harness
        .tracker
        .calls()
        .iter()
        .any(|call| matches!(call, Call::Show { .. })));
    assert!(wrote_only_the_baseline(&harness));
    assert!(files_unchanged(&harness, &seeds));
    assert!(!mutated(&harness.tracker));
    let written = harness.spy.content(BASELINE_PATH).unwrap_or_default();
    assert!(written.contains(&format!("\"0001\":{raw}")), "{written}");
    Ok(())
}

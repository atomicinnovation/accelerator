//! A work-item repository on disk, synced run after run against a recording
//! tracker, so a test can follow an item through several runs.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;
use corpus_adapters::CommonMarkBody;
use tracker::BodyDialect;
use tracker::ExternalId;
use tracker::PlainDialect;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use tracker_test_support::Rewrite;
use work::sync::Action;
use work::sync::Dirtiness;
use work::sync::LegacyHashes;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work::sync::SyncState;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::baseline::SyncStamps;
use work_adapters::sync::baseline_digester::BaselineDigester;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::digest;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::run::run;
use work_adapters::sync::run::ItemSelection;
use work_adapters::sync::run::ReportedItem;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;

pub const BASELINE_PATH: &str = "/baseline/last-sync.json";
pub const STAMP: &str = "2026-06-01T00:00:00Z";
pub const MOVED_STAMP: &str = "2026-07-01T00:00:00Z";

/// Which tracker's body rules the repository syncs under.
#[derive(Clone, Copy)]
pub enum Tracker {
    Linear,
    Jira,
}

impl Tracker {
    fn dialect(self) -> Box<dyn BodyDialect + Send> {
        match self {
            Self::Linear => Box::new(PlainDialect),
            Self::Jira => Box::new(jira_client::JiraBodyDialect),
        }
    }
}

/// The baseline in memory and the work items on disk, recording every write.
#[derive(Default)]
struct Store {
    baseline: RefCell<Option<String>>,
    writes: RefCell<Vec<PathBuf>>,
}

impl FileReader for Store {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        if path == Path::new(BASELINE_PATH) {
            return Ok(self.baseline.borrow().clone());
        }
        Ok(std::fs::read_to_string(path).ok())
    }
}

impl AtomicWrite for Store {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.writes.borrow_mut().push(path.to_path_buf());
        if path == Path::new(BASELINE_PATH) {
            *self.baseline.borrow_mut() =
                Some(String::from_utf8_lossy(bytes).into_owned());
            return Ok(());
        }
        std::fs::write(path, bytes).map_err(|error| StoreError::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        })
    }
}

struct Clean;

impl WorkingCopyStatus for Clean {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        Dirtiness::Clean
    }
}

struct UnusedAuthor;

impl work_adapters::sync::create::LocalAuthor for UnusedAuthor {
    #[allow(clippy::panic)]
    fn author_from_remote(
        &self,
        _issue: &work_adapters::sync::create::DiscoveredIssue,
    ) -> Result<work_adapters::sync::create::AuthoredLocal, kernel::Error> {
        panic!("no repository here authors a discovered issue")
    }

    #[allow(clippy::panic)]
    fn link_external_id(
        &self,
        _path: &Path,
        _external_id: &ExternalId,
    ) -> Result<(), kernel::Error> {
        panic!("no repository here links an external id")
    }
}

struct FixedClock;

impl work::sync::RunClock for FixedClock {
    fn run_start_epoch(&self) -> Result<u64, kernel::Error> {
        Ok(1_700_000_000)
    }
}

/// A work item's file content under `title`, with `extra` frontmatter lines.
pub fn work_item(index: usize, title: &str, extra: &str, body: &str) -> String {
    format!(
        "---\ntitle: {title}\nexternal_id: \"ENG-{index}\"\n{extra}---\n\n{body}"
    )
}

/// The issue the tracker holds for item `index`.
pub fn issue(index: usize, projection: &str, stamp: &str) -> RemoteIssue {
    RemoteIssue {
        key: external(index),
        updated: RemoteTimestamp::Reported(stamp.to_owned()),
        body: projection.to_owned(),
    }
}

pub fn external(index: usize) -> ExternalId {
    ExternalId::new(format!("ENG-{index}"))
}

pub fn id(index: usize) -> String {
    format!("{index:04}")
}

/// What item `index`'s baseline recorded of its two sides at [`STAMP`],
/// under either recipe; a side recorded as `None` is a blanked hash.
pub enum Recorded<'a> {
    Nothing,
    Legacy {
        local: Option<&'a str>,
        projection: Option<&'a str>,
    },
    Split {
        local: Option<&'a str>,
        projection: Option<&'a str>,
    },
}

impl<'a> Recorded<'a> {
    pub const fn legacy(local: &'a str, projection: &'a str) -> Self {
        Self::Legacy {
            local: Some(local),
            projection: Some(projection),
        }
    }

    pub const fn split(local: &'a str, projection: &'a str) -> Self {
        Self::Split {
            local: Some(local),
            projection: Some(projection),
        }
    }
}

pub struct Repo {
    dir: tempfile::TempDir,
    tracker_kind: Tracker,
    pub items: Vec<LocalItem>,
    pub tracker: RecordingTracker,
    store: Store,
}

#[allow(clippy::expect_used, clippy::panic)]
impl Repo {
    /// One item, `index`, holding `content` locally and `remote` in the
    /// tracker, with its baseline recording `recorded`.
    pub fn with_item(
        tracker_kind: Tracker,
        index: usize,
        content: &str,
        remote: RemoteIssue,
        recorded: &Recorded<'_>,
    ) -> Self {
        let tracker =
            RecordingTracker::holding(vec![(external(index), remote)])
                .with_dialect(tracker_kind.dialect());
        Self::over(tracker, tracker_kind, index, content, recorded)
    }

    /// As [`Self::with_item`] for Linear, whose stored descriptions undergo
    /// `rewrite`.
    pub fn with_rewritten_item(
        rewrite: Rewrite,
        index: usize,
        content: &str,
        remote: RemoteIssue,
        recorded: &Recorded<'_>,
    ) -> Self {
        let tracker = RecordingTracker::rewriting(
            vec![(external(index), remote)],
            rewrite,
        );
        Self::over(tracker, Tracker::Linear, index, content, recorded)
    }

    fn over(
        tracker: RecordingTracker,
        tracker_kind: Tracker,
        index: usize,
        content: &str,
        recorded: &Recorded<'_>,
    ) -> Self {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join(format!("{}.md", id(index)));
        std::fs::write(&path, content).expect("the item is written");
        let store = Store::default();
        let entries: Vec<String> = fragment(tracker_kind, index, recorded)
            .into_iter()
            .collect();
        *store.baseline.borrow_mut() = Some(format!(
            "{{\"timestamp\":0,\"items\":{{{}}}}}\n",
            entries.join(",")
        ));
        Self {
            dir,
            tracker_kind,
            items: vec![LocalItem {
                id: id(index),
                path,
                external_id: Some(external(index)),
            }],
            tracker,
            store,
        }
    }

    pub fn sync(
        &self,
        direction: SyncDirection,
        resolutions: &BTreeMap<String, Resolution>,
        mode: RunMode,
    ) -> RunReport {
        let author = UnusedAuthor;
        let ports = SyncPorts {
            tracker: &self.tracker,
            parser: &CommonMarkBody,
            status: &Clean,
            writer: &self.store,
            clock: &FixedClock,
            author: &author,
        };
        let mut store = BaselineStore::new(
            PathBuf::from(BASELINE_PATH),
            &self.store,
            &self.store,
        );
        run(
            &ports,
            &mut store,
            &SyncRequest {
                corpus: &self.items,
                selection: ItemSelection::All,
                direction,
                strategy: RetrievalStrategy::Bulk,
                resolutions,
                max_pulls: tracker::Ceiling::Bounded(25),
                max_pushes: tracker::Ceiling::Bounded(25),
                mode,
                integrations_root: self.dir.path(),
                integration: "jira",
                promote: true,
                scope: tracker::SearchScope::default(),
            },
        )
        .unwrap_or_else(|_| panic!("the run proceeds"))
    }

    pub fn sync_once(&self) -> RunReport {
        self.sync(
            SyncDirection::Bidirectional,
            &BTreeMap::new(),
            RunMode::Apply,
        )
    }

    pub fn preview(&self) -> RunReport {
        self.sync(
            SyncDirection::Bidirectional,
            &BTreeMap::new(),
            RunMode::Preview,
        )
    }

    pub fn resolving(&self, index: usize, resolution: Resolution) -> RunReport {
        self.sync(
            SyncDirection::Bidirectional,
            &BTreeMap::from([(id(index), resolution)]),
            RunMode::Apply,
        )
    }

    pub fn read(&self, index: usize) -> String {
        std::fs::read_to_string(self.path(index)).expect("the item is readable")
    }

    pub fn write(&self, index: usize, content: &str) {
        std::fs::write(self.path(index), content).expect("the item is written");
    }

    fn path(&self, index: usize) -> &Path {
        &self
            .items
            .iter()
            .find(|item| item.id == id(index))
            .expect("the item exists")
            .path
    }

    pub fn baseline_text(&self) -> Option<String> {
        self.store.baseline.borrow().clone()
    }

    pub fn entry(&self, index: usize) -> Option<Entry> {
        Baseline::read(self.baseline_text().as_deref())
            .0
            .get(&id(index))
            .cloned()
    }

    /// Whether the run wrote any work-item file.
    pub fn wrote_an_item(&self) -> bool {
        self.store
            .writes
            .borrow()
            .iter()
            .any(|path| path != Path::new(BASELINE_PATH))
    }

    pub fn forget_writes(&self) {
        self.store.writes.borrow_mut().clear();
    }

    pub fn dialect(&self) -> Box<dyn BodyDialect + Send> {
        self.tracker_kind.dialect()
    }

    /// The calls that change the remote, in order.
    pub fn mutations(&self) -> Vec<Call> {
        self.tracker
            .calls()
            .into_iter()
            .filter(|call| {
                matches!(
                    call,
                    Call::Update { .. }
                        | Call::Retitle { .. }
                        | Call::Create { .. }
                )
            })
            .collect()
    }

    /// The projection the tracker holds for item `index` now.
    pub fn remote(&self, index: usize) -> String {
        use tracker::RemoteTracker as _;
        self.tracker
            .show(&external(index))
            .expect("the issue exists")
            .body
    }
}

#[allow(clippy::expect_used)]
fn fragment(
    tracker_kind: Tracker,
    index: usize,
    recorded: &Recorded<'_>,
) -> Option<String> {
    let stamps = || SyncStamps {
        remote_updated_at: RemoteTimestamp::Reported(STAMP.to_owned()),
        local_synced_at: 0,
    };
    let entry = match recorded {
        Recorded::Nothing => return None,
        Recorded::Legacy { local, projection } => Entry::Legacy {
            stamps: stamps(),
            legacy: LegacyHashes {
                local: local
                    .map(|local| digest::local(local).expect("a work item")),
                remote: projection.map(digest::remote_body),
            },
        },
        Recorded::Split { local, projection } => {
            let placeholder = "---\ntitle: x\n---\n";
            let mut recorded = BaselineDigester::new(
                tracker_kind.dialect().as_ref(),
                &CommonMarkBody,
            )
            .digests(local.unwrap_or(placeholder), *projection)
            .expect("a work item");
            if local.is_none() {
                recorded.split.local = None;
                recorded.legacy.local = None;
            }
            Entry::recorded(stamps(), recorded)
        }
    };
    Some(super::entry_fragment(&id(index), entry))
}

/// The state and action `report` gives item `index`.
#[allow(clippy::panic)]
pub fn outcome_of(report: &RunReport, index: usize) -> (SyncState, Action) {
    let row = row(report, index);
    (row.planned.state, row.planned.action)
}

#[allow(clippy::panic)]
pub fn row(report: &RunReport, index: usize) -> &ReportedItem {
    report
        .reported
        .iter()
        .find(|item| item.planned.id == id(index))
        .unwrap_or_else(|| panic!("item {index} is reported"))
}

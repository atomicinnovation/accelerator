//! A work-item repository on disk, synced run after run against a recording
//! tracker, so a test can follow an item through several runs.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use corpus::lock::ExclusiveLock;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::CopyComparison;
use corpus::store::FileRemove;
use corpus::store::KeptRecovery;
use corpus::store::RecordedCopy;
use corpus::store::RecoveryCopies;
use corpus::store::RecoveryEntry;
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
use vcs::tracking::RepositoryTracking;
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
use work_adapters::sync::files::SyncFiles;
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

/// The work items on disk, recording every write.
#[derive(Default)]
struct WorkItems {
    writes: RefCell<Vec<PathBuf>>,
}

impl AtomicWrite for WorkItems {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.writes.borrow_mut().push(path.to_path_buf());
        std::fs::write(path, bytes).map_err(|error| io(path, &error))
    }
}

impl FileRemove for WorkItems {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        std::fs::remove_file(path).map_err(|error| io(path, &error))
    }
}

/// The integration state, of which a run here writes only the baseline,
/// held in memory.
#[derive(Default)]
struct IntegrationState {
    baseline: RefCell<Option<String>>,
}

impl IntegrationState {
    fn refuse_other_than_the_baseline(path: &Path) -> Result<(), StoreError> {
        if path == Path::new(BASELINE_PATH) {
            Ok(())
        } else {
            Err(StoreError::UnsafePath {
                path: path.display().to_string(),
            })
        }
    }
}

impl FileReader for IntegrationState {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        Self::refuse_other_than_the_baseline(path)
            .map_err(|error| kernel::Error::Failed(error.to_string()))?;
        Ok(self.baseline.borrow().clone())
    }
}

impl AtomicWrite for IntegrationState {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        Self::refuse_other_than_the_baseline(path)?;
        *self.baseline.borrow_mut() =
            Some(String::from_utf8_lossy(bytes).into_owned());
        Ok(())
    }
}

impl FileRemove for IntegrationState {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        Self::refuse_other_than_the_baseline(path)?;
        *self.baseline.borrow_mut() = None;
        Ok(())
    }
}

fn io(path: &Path, error: &std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    }
}

/// The lock and recovery ports of a run, recording each kept copy and able
/// to edit an item at the moment its lock is taken or its copy is kept, as
/// an editor saving mid-pull would.
#[derive(Default)]
struct FileEvents {
    edit_on_lock: RefCell<Option<(PathBuf, String)>>,
    edit_on_kept: RefCell<Option<(PathBuf, String)>>,
    kept: RefCell<Vec<PathBuf>>,
}

#[allow(clippy::expect_used)]
fn edit_once(edit: &RefCell<Option<(PathBuf, String)>>, path: &Path) {
    let due = edit
        .borrow()
        .as_ref()
        .is_some_and(|(edited, _)| edited == path);
    if due {
        let (edited, content) = edit.borrow_mut().take().expect("due");
        std::fs::write(edited, content).expect("the item is edited");
    }
}

impl ExclusiveLock for FileEvents {
    fn acquire(&self, name: &LockName) -> Result<HeldLock, StoreError> {
        if let LockName::ForFile(path) = name {
            edit_once(&self.edit_on_lock, path);
        }
        Ok(HeldLock::new(()))
    }

    fn clear_abandoned(&self, _name: &LockName) {}
}

impl RecoveryCopies for FileEvents {
    fn exists(&self, _dir: &Path) -> bool {
        false
    }

    fn prepare(&self, _dir: &Path) -> Result<(), StoreError> {
        Ok(())
    }

    fn write_once(
        &self,
        _dir: &Path,
        _original: &Path,
        _bytes: &[u8],
    ) -> Result<(), StoreError> {
        Ok(())
    }

    fn mark_restore_pending(
        &self,
        _dir: &Path,
        _unrestored: &[PathBuf],
    ) -> Result<(), StoreError> {
        Ok(())
    }

    fn is_restore_pending(&self, _dir: &Path) -> bool {
        false
    }

    fn mark_completed(&self, _dir: &Path) -> Result<(), StoreError> {
        Ok(())
    }

    fn remove_dir(&self, _dir: &Path) -> Result<(), StoreError> {
        Ok(())
    }

    fn kept(&self, _parent: &Path) -> Result<Vec<KeptRecovery>, StoreError> {
        Ok(Vec::new())
    }

    fn all_entries(
        &self,
        _parent: &Path,
    ) -> Result<Vec<RecoveryEntry>, StoreError> {
        Ok(Vec::new())
    }

    fn location(&self, path: &Path) -> PathBuf {
        path.to_path_buf()
    }

    fn mirrored_copy(&self, dir: &Path, original: &Path) -> RecordedCopy {
        RecordedCopy {
            dir: dir.to_path_buf(),
            original: original.to_path_buf(),
            copy: original.file_name().map(PathBuf::from).unwrap_or_default(),
        }
    }

    fn mark_copy_pending(&self, copy: &RecordedCopy) -> Result<(), StoreError> {
        self.kept.borrow_mut().push(copy.dir.clone());
        edit_once(&self.edit_on_kept, &copy.original);
        Ok(())
    }

    fn recorded_copy(&self, _dir: &Path) -> Option<RecordedCopy> {
        None
    }

    fn compare_copy(&self, _copy: &RecordedCopy) -> CopyComparison {
        CopyComparison::CopyMissing
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

/// When an item's baseline says its local file was last synced, against the
/// file's own mtime: after it, the mtime pre-filter proves the file unchanged
/// without reading it.
#[derive(Clone, Copy)]
pub enum LastSynced {
    BeforeItsLastEdit,
    AfterItsLastEdit,
}

impl LastSynced {
    const fn epoch(self) -> u64 {
        match self {
            Self::BeforeItsLastEdit => 0,
            Self::AfterItsLastEdit => 4_000_000_000,
        }
    }
}

/// One item a repository holds: `content` locally, `remote` in the tracker,
/// and a baseline recording `recorded` as last synced at `last_synced`.
pub struct Seed<'a> {
    pub index: usize,
    pub content: &'a str,
    pub remote: RemoteIssue,
    pub recorded: Recorded<'a>,
    pub last_synced: LastSynced,
}

pub struct Repo {
    dir: tempfile::TempDir,
    tracker_kind: Tracker,
    pub items: Vec<LocalItem>,
    pub tracker: RecordingTracker,
    work_items: WorkItems,
    integration_state: IntegrationState,
    events: FileEvents,
    tracking: Option<Box<dyn RepositoryTracking>>,
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

    /// Several items, synced in the order given.
    pub fn with_items(tracker_kind: Tracker, seeds: &[Seed<'_>]) -> Self {
        let tracker = RecordingTracker::holding(
            seeds
                .iter()
                .map(|seed| (external(seed.index), seed.remote.clone()))
                .collect(),
        )
        .with_dialect(tracker_kind.dialect());
        Self::seeded(tracker, tracker_kind, seeds)
    }

    /// Rewrites item `edited` to `content` once, when a write to item
    /// `written` lands in the tracker, as an editor saving mid-run would.
    #[must_use]
    pub fn editing_when_written(
        mut self,
        written: usize,
        edited: usize,
        content: &str,
    ) -> Self {
        let path = self.path(edited).to_path_buf();
        let content = content.to_owned();
        let trigger = external(written);
        let pending = std::sync::atomic::AtomicBool::new(true);
        let tracker = std::mem::replace(
            &mut self.tracker,
            RecordingTracker::holding(Vec::new()),
        );
        self.tracker = tracker.after_writing(move |landed| {
            if *landed == trigger
                && pending.swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                std::fs::write(&path, &content).expect("the item is edited");
            }
        });
        self
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
        Self::holding(
            dir,
            tracker,
            tracker_kind,
            vec![(index, path)],
            fragment(
                tracker_kind,
                index,
                recorded,
                LastSynced::BeforeItsLastEdit,
            )
            .as_slice(),
        )
    }

    fn seeded(
        tracker: RecordingTracker,
        tracker_kind: Tracker,
        seeds: &[Seed<'_>],
    ) -> Self {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut paths = Vec::with_capacity(seeds.len());
        let mut entries = Vec::with_capacity(seeds.len());
        for seed in seeds {
            let path = dir.path().join(format!("{}.md", id(seed.index)));
            std::fs::write(&path, seed.content).expect("the item is written");
            paths.push((seed.index, path));
            entries.extend(fragment(
                tracker_kind,
                seed.index,
                &seed.recorded,
                seed.last_synced,
            ));
        }
        Self::holding(dir, tracker, tracker_kind, paths, &entries)
    }

    fn holding(
        dir: tempfile::TempDir,
        tracker: RecordingTracker,
        tracker_kind: Tracker,
        paths: Vec<(usize, PathBuf)>,
        entries: &[String],
    ) -> Self {
        let integration_state = IntegrationState::default();
        *integration_state.baseline.borrow_mut() = Some(format!(
            "{{\"timestamp\":0,\"items\":{{{}}}}}\n",
            entries.join(",")
        ));
        Self {
            dir,
            tracker_kind,
            items: paths
                .into_iter()
                .map(|(index, path)| LocalItem {
                    id: id(index),
                    path,
                    external_id: Some(external(index)),
                })
                .collect(),
            tracker,
            work_items: WorkItems::default(),
            integration_state,
            events: FileEvents::default(),
            tracking: None,
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
            files: SyncFiles {
                work_items: &self.work_items,
                integration_state: &self.integration_state,
                file_locks: &self.events,
                recovery: &self.events,
                tracking: self
                    .tracking
                    .as_deref()
                    .unwrap_or(&super::AllTracked),
            },
            clock: &FixedClock,
            author: &author,
            recovery_labels: &super::LabelsEveryId,
        };
        let mut store = BaselineStore::new(
            PathBuf::from(BASELINE_PATH),
            &self.integration_state,
            &self.integration_state,
        );
        run(
            &ports,
            &mut store,
            &SyncRequest {
                corpus: &self.items,
                inventory: &work_adapters::sync::fetch::inventory_of(
                    &self.items,
                ),
                selection: ItemSelection::All,
                direction,
                strategy: RetrievalStrategy::Bulk,
                resolutions,
                pending_push_orders: &BTreeMap::new(),
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

    /// Asks the repositories enclosing the items whether each is tracked,
    /// rather than answering that every one is.
    #[must_use]
    pub fn asking_the_repository(mut self) -> Self {
        self.tracking = Some(Box::new(vcs_adapters::InProcessTracking));
        self
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// Rewrites item `index` to `content` once, when a pull takes its lock.
    #[must_use]
    pub fn editing_when_locked(self, index: usize, content: &str) -> Self {
        *self.events.edit_on_lock.borrow_mut() =
            Some((self.path(index).to_path_buf(), content.to_owned()));
        self
    }

    /// Rewrites item `index` to `content` once, when a pull has kept its
    /// edits.
    #[must_use]
    pub fn editing_when_kept(self, index: usize, content: &str) -> Self {
        *self.events.edit_on_kept.borrow_mut() =
            Some((self.path(index).to_path_buf(), content.to_owned()));
        self
    }

    /// Every directory a run kept local edits in, in the order kept.
    pub fn kept_edits(&self) -> Vec<PathBuf> {
        self.events.kept.borrow().clone()
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
        self.integration_state.baseline.borrow().clone()
    }

    pub fn entry(&self, index: usize) -> Option<Entry> {
        Baseline::read(self.baseline_text().as_deref())
            .0
            .get(&id(index))
            .cloned()
    }

    /// Whether the run wrote any work-item file.
    pub fn wrote_an_item(&self) -> bool {
        !self.work_items.writes.borrow().is_empty()
    }

    pub fn forget_writes(&self) {
        self.work_items.writes.borrow_mut().clear();
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
    last_synced: LastSynced,
) -> Option<String> {
    let stamps = || SyncStamps {
        remote_updated_at: RemoteTimestamp::Reported(STAMP.to_owned()),
        local_synced_at: last_synced.epoch(),
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
    (row.planned.state(), row.planned.action())
}

#[allow(clippy::panic)]
pub fn row(report: &RunReport, index: usize) -> &ReportedItem {
    report
        .reported
        .iter()
        .find(|item| item.planned.id() == id(index))
        .unwrap_or_else(|| panic!("item {index} is reported"))
}

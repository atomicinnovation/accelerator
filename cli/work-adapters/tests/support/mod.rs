//! A re-key corpus on disk, and ports that count every mutation so a test
//! can fail or halt a run at any one of them.

#![allow(dead_code)]

pub mod repo;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use corpus::store::ExclusiveCreate;
use corpus::store::FileRemove;
use corpus::store::KeptRecovery;
use corpus::store::RecoveryCopies;
use corpus::StoreError;
use corpus_adapters::CommonMarkBody;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::FileRecoveryCopies;
use corpus_adapters::LockdirLock;
use corpus_adapters::RealFs;
use jira_client::JiraBodyDialect;
use store::lock::LockOptions;
use tracker::BodyDialect;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker_test_support::RecordingTracker;
use work::dirtiness::Dirtiness;
use work::rekey::entry::RekeyEntry;
use work::rekey::entry::RekeyTarget;
use work::sync::LegacyHashes;
use work_adapters::rekey::execute::RekeyCorpus;
use work_adapters::rekey::run_plan_store::FileRunPlanStore;
use work_adapters::retirement::CorpusLayout;
use work_adapters::retirement::RetirementFiles;
use work_adapters::retirement::RetirementPorts;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::baseline::SyncStamps;
use work_adapters::sync::baseline_digester::BaselineDigester;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::fetch::WorkingCopyStatus;

pub type TestError = Box<dyn std::error::Error>;

pub const FIRST: &str = "meta/work/0041-first.md";
pub const SYNCED: &str = "meta/work/PP-760-sync.md";
pub const PLAN: &str = "meta/plans/plan.md";
pub const NOTE: &str = "meta/notes/unrelated.md";
pub const FIRST_TARGET: &str = "meta/work/ACC-0041-first.md";
pub const SYNCED_TARGET: &str = "meta/work/ACC-0042-sync.md";
pub const BASELINE: &str =
    ".accelerator/state/integrations/linear/last-sync.json";
pub const RUN_PLAN: &str = ".accelerator/state/rekey/run-plan.json";

pub const FIRST_CONTENT: &str =
    "---\nid: \"0041\"\ntitle: \"First\"\n---\n\n# 0041: First\n";
pub const SYNCED_CONTENT: &str = "---\nid: \"PP-760\"\nexternal_id: \
                                  \"PP-760\"\ntitle: \"Sync\"\n---\n\n# \
                                  PP-760: Sync\n";
pub const PLAN_CONTENT: &str =
    "Builds on work-item:PP-760 and [first](../work/0041-first.md).\n";
pub const NOTE_CONTENT: &str = "Nothing to see.\n";

const FAST: LockOptions = LockOptions {
    ceiling_ms: 1,
    base_ms: 1,
    cap_ms: 1,
};

pub fn renumbered(old: &str, new: &str) -> RekeyEntry {
    RekeyEntry {
        old: old.to_owned(),
        target: RekeyTarget::Renumbered(new.to_owned()),
    }
}

pub fn entries() -> Vec<RekeyEntry> {
    vec![
        renumbered("0041", "ACC-0041"),
        renumbered("PP-760", "ACC-0042"),
    ]
}

pub struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    pub fn new() -> Result<Self, TestError> {
        let repo = Self {
            dir: tempfile::tempdir()?,
        };
        for (path, content) in [
            (FIRST, FIRST_CONTENT),
            (SYNCED, SYNCED_CONTENT),
            (PLAN, PLAN_CONTENT),
            (NOTE, NOTE_CONTENT),
        ] {
            repo.write(path, content)?;
        }
        let store = FileCorpusStore::new(repo.root());
        fs::create_dir_all(repo.path(BASELINE).parent().ok_or("no parent")?)?;
        BaselineStore::new(repo.path(BASELINE), &RealFs, &store).set(
            "PP-760",
            Entry::Legacy {
                stamps: SyncStamps {
                    remote_updated_at: RemoteTimestamp::NotRead,
                    local_synced_at: 0,
                },
                legacy: LegacyHashes {
                    local: Some("synced".to_owned()),
                    remote: None,
                },
            },
        )?;
        Ok(repo)
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    pub fn write(
        &self,
        relative: &str,
        content: &str,
    ) -> Result<(), TestError> {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn read(&self, relative: &str) -> Option<String> {
        fs::read_to_string(self.path(relative)).ok()
    }

    pub fn work_dir(&self) -> PathBuf {
        self.path("meta/work")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.path(".accelerator/state")
    }

    /// Every file under `meta/` and the baseline, by repo-relative path;
    /// lockdirs are left out, since a real kill can leave one a reclaim
    /// later removes.
    pub fn tree(&self) -> BTreeMap<String, String> {
        let mut tree = BTreeMap::new();
        collect(self.root(), &self.path("meta"), &mut tree);
        if let Some(baseline) = self.read(BASELINE) {
            tree.insert(BASELINE.to_owned(), baseline);
        }
        tree
    }
}

fn collect(root: &Path, dir: &Path, tree: &mut BTreeMap<String, String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".lockdir") {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, tree);
        } else if let (Ok(relative), Ok(content)) =
            (path.strip_prefix(root), fs::read_to_string(&path))
        {
            tree.insert(relative.display().to_string(), content);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    Never,
    FailAt(usize),
    FailFrom(usize),
    HaltAt(usize),
}

/// The signal a halted run unwinds with, as a killed process stops.
#[derive(Debug)]
pub struct Halted(pub usize);

/// Every mutation a run makes, in order, failing or halting the one its
/// fault names.
pub struct Mutations {
    fault: Fault,
    log: RefCell<Vec<PathBuf>>,
    pub before_failing: RefCell<Option<Box<dyn Fn()>>>,
}

impl Mutations {
    pub fn new(fault: Fault) -> Self {
        Self {
            fault,
            log: RefCell::new(Vec::new()),
            before_failing: RefCell::new(None),
        }
    }

    pub fn count(&self) -> usize {
        self.log.borrow().len()
    }

    pub fn log(&self) -> Vec<PathBuf> {
        self.log.borrow().clone()
    }

    fn check(&self, path: &Path) -> Result<(), StoreError> {
        let index = self.count();
        self.log.borrow_mut().push(path.to_path_buf());
        let failing = match self.fault {
            Fault::HaltAt(at) if at == index => {
                std::panic::resume_unwind(Box::new(Halted(index)))
            }
            Fault::Never | Fault::HaltAt(_) => false,
            Fault::FailAt(at) => at == index,
            Fault::FailFrom(at) => index >= at,
        };
        if !failing {
            return Ok(());
        }
        if let Some(hook) = self.before_failing.borrow_mut().take() {
            hook();
        }
        Err(StoreError::Io {
            path: path.display().to_string(),
            detail: format!("injected failure at mutation {index}"),
        })
    }
}

pub struct CountingStore<'m> {
    inner: FileCorpusStore,
    mutations: &'m Mutations,
}

impl AtomicWrite for CountingStore<'_> {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.mutations.check(path)?;
        self.inner.write(path, bytes)
    }
}

impl ExclusiveCreate for CountingStore<'_> {
    fn create_new(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.mutations.check(path)?;
        self.inner.create_new(path, bytes)
    }
}

impl FileRemove for CountingStore<'_> {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        self.mutations.check(path)?;
        self.inner.remove(path)
    }
}

pub struct CountingRecovery<'m> {
    inner: FileRecoveryCopies,
    mutations: &'m Mutations,
}

impl RecoveryCopies for CountingRecovery<'_> {
    fn exists(&self, dir: &Path) -> bool {
        self.inner.exists(dir)
    }

    fn prepare(&self, dir: &Path) -> Result<(), StoreError> {
        self.mutations.check(dir)?;
        self.inner.prepare(dir)
    }

    fn write_once(
        &self,
        dir: &Path,
        original: &Path,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        self.mutations.check(original)?;
        self.inner.write_once(dir, original, bytes)
    }

    fn mark_restore_pending(
        &self,
        dir: &Path,
        unrestored: &[PathBuf],
    ) -> Result<(), StoreError> {
        self.mutations.check(dir)?;
        self.inner.mark_restore_pending(dir, unrestored)
    }

    fn is_restore_pending(&self, dir: &Path) -> bool {
        self.inner.is_restore_pending(dir)
    }

    fn mark_completed(&self, dir: &Path) -> Result<(), StoreError> {
        self.mutations.check(dir)?;
        self.inner.mark_completed(dir)
    }

    fn remove_dir(&self, dir: &Path) -> Result<(), StoreError> {
        if !self.inner.exists(dir) {
            return Ok(());
        }
        self.mutations.check(dir)?;
        self.inner.remove_dir(dir)
    }

    fn kept(&self, parent: &Path) -> Result<Vec<KeptRecovery>, StoreError> {
        self.inner.kept(parent)
    }

    fn copy_settled(&self, dir: &Path, original: &Path) -> bool {
        self.inner.copy_settled(dir, original)
    }
}

pub struct Clean;

impl WorkingCopyStatus for Clean {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        Dirtiness::Clean
    }
}

/// Everything a re-key writes through, over `repo`, counting into
/// `mutations`.
pub struct Ports<'r> {
    repo: &'r Repo,
    pub store: CountingStore<'r>,
    pub locks: LockdirLock,
    pub recovery: CountingRecovery<'r>,
    roots: Vec<PathBuf>,
    work_dir: PathBuf,
    state_dir: PathBuf,
    pub integrated: bool,
}

impl<'r> Ports<'r> {
    pub fn new(repo: &'r Repo, mutations: &'r Mutations) -> Self {
        Self {
            repo,
            store: CountingStore {
                inner: FileCorpusStore::new(repo.root()),
                mutations,
            },
            locks: LockdirLock::with_options(repo.work_dir(), FAST),
            recovery: CountingRecovery {
                inner: FileRecoveryCopies::new(repo.state_dir(), repo.root()),
                mutations,
            },
            roots: vec![repo.path("meta")],
            work_dir: repo.work_dir(),
            state_dir: repo.state_dir(),
            integrated: true,
        }
    }

    pub const fn without_an_integration(mut self) -> Self {
        self.integrated = false;
        self
    }

    pub fn baseline(&self) -> BaselineStore<'_> {
        BaselineStore::new(self.repo.path(BASELINE), &RealFs, &self.store)
    }

    pub fn plan_store(&self) -> FileRunPlanStore<'_> {
        FileRunPlanStore::new(
            &self.state_dir,
            self.repo.root(),
            &self.store,
            &self.store,
        )
    }

    pub fn corpus<'p>(
        &'p self,
        baseline: &'p BaselineStore<'p>,
    ) -> RekeyCorpus<'p> {
        RekeyCorpus {
            repo_root: self.repo.root(),
            state_dir: &self.state_dir,
            ports: RetirementPorts {
                files: RetirementFiles {
                    reader: &RealFs,
                    writer: &self.store,
                    creator: &self.store,
                    remover: &self.store,
                    file_locks: &self.locks,
                    recovery: &self.recovery,
                },
                layout: CorpusLayout {
                    roots: &self.roots,
                    work_dir: &self.work_dir,
                },
                walker: &RealFs,
                status: &Clean,
                lock: &self.locks,
                baseline: self.integrated.then_some(baseline),
            },
        }
    }
}

/// `Err` naming the mutation a [`Fault::HaltAt`] stopped `run` at.
pub fn halted_at<T>(run: impl FnOnce() -> T) -> Result<T, Option<usize>> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).map_err(
        |payload| payload.downcast_ref::<Halted>().map(|halted| halted.0),
    )
}

/// A recording tracker holding `issues` under Jira's body rules.
pub fn jira_recording_tracker(
    issues: Vec<(ExternalId, RemoteIssue)>,
) -> RecordingTracker {
    RecordingTracker::holding(issues).with_dialect(Box::new(JiraBodyDialect))
}

/// Asserts `entry` is a recipe-1 entry recording both recipes' digests of
/// `local_content` and `remote_projection`.
///
/// # Panics
///
/// When the entry is absent, not recipe 1, or records other digests.
#[allow(clippy::panic, clippy::expect_used)]
pub fn assert_recipe_one_entry(
    entry: Option<&Entry>,
    dialect: &dyn BodyDialect,
    local_content: &str,
    remote_projection: Option<&str>,
) {
    let recorded = BaselineDigester::new(dialect, &CommonMarkBody)
        .digests(local_content, remote_projection)
        .expect("the local content is well formed");
    let Some(Entry::Split { legacy, split, .. }) = entry else {
        panic!("expected a recipe-1 entry, found {entry:?}");
    };
    assert_eq!(split, &recorded.split, "the split fingerprints");
    assert_eq!(legacy, &recorded.legacy, "the legacy hashes");
}

/// `id`'s recipe-1 entry as a `last-sync.json` item fragment, recording
/// `local_content` and `remote_projection` under `dialect`'s body rules.
///
/// # Panics
///
/// When `local_content` is not a well-formed work item.
#[allow(clippy::expect_used)]
pub fn split_entry(
    id: &str,
    dialect: &dyn BodyDialect,
    local_content: &str,
    remote_projection: &str,
    remote_updated_at: &str,
) -> String {
    let fingerprints = BaselineDigester::new(dialect, &CommonMarkBody)
        .digests(local_content, Some(remote_projection))
        .expect("the local content is well formed");
    entry_fragment(
        id,
        Entry::recorded(
            SyncStamps {
                remote_updated_at: RemoteTimestamp::Reported(
                    remote_updated_at.to_owned(),
                ),
                local_synced_at: 0,
            },
            fingerprints,
        ),
    )
}

/// `entry` as `id`'s `last-sync.json` item fragment.
///
/// # Panics
///
/// Never: one rendered entry always has the sliced shape.
#[allow(clippy::expect_used)]
pub fn entry_fragment(id: &str, entry: Entry) -> String {
    let mut baseline = Baseline::read(None).0;
    baseline.set(id, entry);
    baseline
        .render()
        .trim_end()
        .strip_prefix("{\"timestamp\":0,\"items\":{")
        .and_then(|items| items.strip_suffix("}}"))
        .expect("one rendered entry")
        .to_owned()
}

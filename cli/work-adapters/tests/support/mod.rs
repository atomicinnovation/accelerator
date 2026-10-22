//! A re-key corpus on disk, and ports that count every mutation so a test
//! can fail or halt a run at any one of them.

#![allow(dead_code)]

pub mod repo;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus::lock::ExclusiveLock;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::store::AtomicWrite;
use corpus::store::CopyComparison;
use corpus::store::ExclusiveCreate;
use corpus::store::FileRemove;
use corpus::store::KeptRecovery;
use corpus::store::RecordedCopy;
use corpus::store::RecoveryCopies;
use corpus::store::RecoveryEntry;
use corpus::StoreError;
use corpus::WorkItemIdScheme;
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
use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;
use vcs::RepoFacts;
use vcs::RepositoryProbe;
use vcs::VcsKind;
use vcs::WorkingCopyState;
use vcs::WorkingCopyStateProbe;
use work::dirtiness::Dirtiness;
use work::identity_run::begin_rekey_run;
use work::identity_run::HeldIdentityRun;
use work::identity_run::IdentityRun;
use work::identity_run::IdentityRunAttempt;
use work::identity_run::IdentityRunGuard;
use work::identity_run::IdentityRunLockError;
use work::rekey::entry::RekeyEntry;
use work::rekey::entry::RekeyTarget;
use work::rekey::mapping::RefreshedKeys;
use work::rekey::outcome::Holder;
use work::rekey::outcome::NotRekeyed;
use work::sync::LegacyHashes;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::identity_run_lock::IdentityRunLock;
use work_adapters::rekey::execute::RekeyCorpus;
use work_adapters::rekey::guards::PendingPushDir;
use work_adapters::rekey::guards::SyncState;
use work_adapters::rekey::run::run;
use work_adapters::rekey::run::Forcing;
use work_adapters::rekey::run::KeyRefresh;
use work_adapters::rekey::run::RekeyAction;
use work_adapters::rekey::run::RekeyPorts;
use work_adapters::rekey::run::RekeyProgress;
use work_adapters::rekey::run::RekeyReport;
use work_adapters::rekey::run::RekeyRequest;
use work_adapters::rekey::run_plan_store::FileRunPlanStore;
use work_adapters::retirement::CorpusLayout;
use work_adapters::retirement::RetirementFiles;
use work_adapters::retirement::RetirementPorts;
use work_adapters::retirement_records::FileRetirementRecords;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::baseline::SyncStamps;
use work_adapters::sync::baseline_digester::BaselineDigester;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::files::SyncFiles;
use work_adapters::sync::recovery_label::RecoveryLabel;
use work_adapters::sync::recovery_label::RecoveryLabels;

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

    fn all_entries(
        &self,
        parent: &Path,
    ) -> Result<Vec<RecoveryEntry>, StoreError> {
        self.inner.all_entries(parent)
    }

    fn location(&self, path: &Path) -> PathBuf {
        self.inner.location(path)
    }

    fn mirrored_copy(&self, dir: &Path, original: &Path) -> RecordedCopy {
        self.inner.mirrored_copy(dir, original)
    }

    fn mark_copy_pending(&self, copy: &RecordedCopy) -> Result<(), StoreError> {
        self.mutations.check(&copy.dir)?;
        self.inner.mark_copy_pending(copy)
    }

    fn recorded_copy(&self, dir: &Path) -> Option<RecordedCopy> {
        self.inner.recorded_copy(dir)
    }

    fn compare_copy(&self, copy: &RecordedCopy) -> CopyComparison {
        self.inner.compare_copy(copy)
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

// "{key}" and "{number:04d}" are pattern tokens, not format args.
#[allow(clippy::literal_string_with_formatting_args)]
pub fn scheme() -> WorkItemIdScheme {
    WorkItemIdScheme {
        id_pattern: "{key}-{number:04d}".to_owned(),
        key: Some("ACC".to_owned()),
    }
}

/// A repository whose working copy is whatever the test says it is.
pub struct ScriptedRepository {
    root: PathBuf,
    pub base: RefCell<Vec<String>>,
    pub dirty: RefCell<Vec<String>>,
}

impl ScriptedRepository {
    pub fn clean_at(repo: &Repo, commit: &str) -> Self {
        Self {
            root: repo.root().to_path_buf(),
            base: RefCell::new(vec![commit.to_owned()]),
            dirty: RefCell::new(Vec::new()),
        }
    }
}

impl WorkingCopyStateProbe for ScriptedRepository {
    fn working_copy_state(
        &self,
        _root: &Path,
        _kind: VcsKind,
    ) -> Result<WorkingCopyState, kernel::Error> {
        Ok(WorkingCopyState {
            base_commits: self.base.borrow().clone(),
            dirty_paths: self.dirty.borrow().clone(),
        })
    }
}

impl RepositoryProbe for ScriptedRepository {
    fn facts_at(&self, _start: &Path) -> Option<RepoFacts> {
        Some(RepoFacts {
            root: self.root.clone(),
            name: "repo".to_owned(),
            kind: VcsKind::Git,
            revision: None,
        })
    }

    fn user_name_at(&self, _start: &Path) -> Option<String> {
        None
    }

    fn canonical(&self, path: &Path) -> PathBuf {
        path.to_path_buf()
    }
}

/// Every line a run shows a person while it proceeds.
#[derive(Default)]
pub struct Recorder {
    pub lines: RefCell<Vec<String>>,
    pub on_applied: RefCell<Option<Box<dyn Fn()>>>,
}

impl RekeyProgress for Recorder {
    fn waiting(&self, holder: Option<&Holder>) {
        self.lines.borrow_mut().push(format!("waiting {holder:?}"));
    }

    fn applied(&self, entry: &RekeyEntry, path: &str) {
        if let Some(hook) = self.on_applied.borrow().as_ref() {
            hook();
        }
        self.lines.borrow_mut().push(format!(
            "{} → {}\t{path}",
            entry.old,
            entry.target.id()
        ));
    }

    fn remaining(&self, lines: &[String]) {
        self.lines
            .borrow_mut()
            .extend(lines.iter().map(|line| format!("remaining {line}")));
    }

    fn forced_past(&self, dirty: &[String]) {
        self.lines
            .borrow_mut()
            .extend(dirty.iter().map(|path| format!("forced past {path}")));
    }

    fn partly_applied(&self, items: &[PathBuf], recoveries: &[PathBuf]) {
        let lines = items
            .iter()
            .map(|item| format!("item {}", item.display()))
            .chain(
                recoveries
                    .iter()
                    .map(|dir| format!("recovery {}", dir.display())),
            );
        self.lines.borrow_mut().extend(lines);
    }
}

/// A tracker no numeric re-key may reach.
pub struct NoTracker;

impl KeyRefresh for NoTracker {
    fn refresh(
        &self,
        _keys: &[ExternalId],
    ) -> Result<RefreshedKeys, NotRekeyed> {
        Err(NotRekeyed::TrackerUnreachable(
            "a numeric re-key reads no tracker".to_owned(),
        ))
    }
}

pub fn request(action: RekeyAction, forcing: Forcing) -> RekeyRequest {
    RekeyRequest {
        action,
        forcing,
        scheme: scheme(),
    }
}

impl Ports<'_> {
    /// Runs `request` over this repository, through `identity_run` when
    /// given and the real identity-run lock otherwise.
    pub fn rekey(
        &self,
        request: &RekeyRequest,
        repository: &dyn RepositoryProbe,
        progress: &dyn RekeyProgress,
        identity_run: Option<&dyn IdentityRun>,
    ) -> Result<RekeyReport, NotRekeyed> {
        let baseline = self.baseline();
        let lock = IdentityRunLock::new(&self.state_dir, &self.store);
        let records = FileRetirementRecords::new(&self.state_dir, &self.store);
        let items = FilesystemWorkItemFiles::new(&self.work_dir);
        let integrations = self.state_dir.join("integrations");
        let store = self.plan_store();
        let scopes: Vec<String> = vcs::guarded_working_copy::GUARDED_SCOPES
            .iter()
            .map(|scope| (*scope).to_owned())
            .collect();
        run(
            request,
            &RekeyPorts {
                identity_run: identity_run.unwrap_or(&lock),
                store: &store,
                corpus: self.corpus(&baseline),
                items: &items,
                sync: SyncState {
                    state_dir: &self.state_dir,
                    records: &records,
                    recovery: &self.recovery,
                    pending_push: self.integrated.then_some(PendingPushDir {
                        integrations_root: &integrations,
                        integration: "linear",
                    }),
                },
                repository,
                scopes: &scopes,
                progress,
                refresh: &NoTracker,
                draws: &RefCell::new(RandomSuffixDraws),
            },
        )
    }

    pub fn apply(
        &self,
        repository: &dyn RepositoryProbe,
    ) -> Result<RekeyReport, NotRekeyed> {
        self.rekey(
            &request(RekeyAction::Apply, Forcing::Unforced),
            repository,
            &Recorder::default(),
            None,
        )
    }
}

/// Puts every file back as `before` had it, deleting any it did not hold.
pub fn restore(
    repo: &Repo,
    before: &BTreeMap<String, String>,
) -> Result<(), TestError> {
    for path in repo.tree().keys() {
        if !before.contains_key(path) {
            fs::remove_file(repo.path(path))?;
        }
    }
    for (path, content) in before {
        repo.write(path, content)?;
    }
    Ok(())
}

/// An identity-run lock nobody else holds.
pub struct Unheld;

impl IdentityRun for Unheld {
    fn try_acquire(
        &self,
        _command: &str,
    ) -> Result<IdentityRunAttempt, StoreError> {
        Ok(IdentityRunAttempt::Acquired(HeldIdentityRun::new(())))
    }

    fn acquire_within(
        &self,
        _command: &str,
        _ceiling: std::time::Duration,
    ) -> Result<HeldIdentityRun, IdentityRunLockError> {
        Ok(HeldIdentityRun::new(()))
    }
}

/// A held identity run, for use cases that need proof of one.
pub fn identity_run() -> Result<IdentityRunGuard, NotRekeyed> {
    begin_rekey_run(&Unheld, &|_| {})
}

/// The file ports of a run whose every write goes through `writer`.
pub fn sync_files(writer: &dyn AtomicWrite) -> SyncFiles<'_> {
    SyncFiles {
        work_items: writer,
        integration_state: writer,
        file_locks: &Unlocked,
        recovery: &DiscardedCopies,
        tracking: &AllTracked,
    }
}

/// Accepts every recovery copy and keeps none, for runs that never read
/// one back.
pub struct DiscardedCopies;

impl RecoveryCopies for DiscardedCopies {
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

    fn mark_copy_pending(
        &self,
        _copy: &RecordedCopy,
    ) -> Result<(), StoreError> {
        Ok(())
    }

    fn recorded_copy(&self, _dir: &Path) -> Option<RecordedCopy> {
        None
    }

    fn compare_copy(&self, _copy: &RecordedCopy) -> CopyComparison {
        CopyComparison::CopyMissing
    }
}

/// Answers that every file is committed.
pub struct AllTracked;

impl RepositoryTracking for AllTracked {
    fn file_tracking(&self, _path: &Path) -> FileTracking {
        FileTracking::Tracked
    }

    fn repository_roots(&self, _directory: &Path) -> RootsAnswer {
        RootsAnswer {
            roots: Vec::new(),
            complete: true,
        }
    }
}

/// Names a recovery directory for every id.
pub struct LabelsEveryId;

impl RecoveryLabels for LabelsEveryId {
    fn label_for(&self, id: &str, epoch: u64) -> Option<RecoveryLabel> {
        Some(RecoveryLabel::new(epoch, "000000", id))
    }
}

/// Grants every lock at once, for runs that never contend.
pub struct Unlocked;

impl ExclusiveLock for Unlocked {
    fn acquire(&self, _name: &LockName) -> Result<HeldLock, StoreError> {
        Ok(HeldLock::new(()))
    }

    fn clear_abandoned(&self, _name: &LockName) {}
}

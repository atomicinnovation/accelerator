//! `accelerator cache prune` in a relocated cache root, driven through
//! `start_up` and `cache::run` as production drives them.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lease::lease_beside;
use tempfile::TempDir;

use std::ffi::OsString;

use accelerator::launch::cache;
use accelerator::launch::core::tree::{
    AcquiredTree, Clock, HeldLease, MaterialiseTree as _, SealedTree, TreeError,
};
use accelerator::launch::core::{
    acquire_trees, run_external, ExecBinary, ExternalCommand, ResolutionError,
};
use accelerator::launch::inbound::cli::CacheAction;
use accelerator::launch::outbound::resolve::cache::store;
use accelerator::launch::outbound::resolve::cache_root::CacheRootConfig;
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::filesystem::testing::ReportingFilesystem;
use accelerator::launch::outbound::resolve::filesystem::{
    CacheFilesystem, EntryMetadata, SystemFilesystem,
};
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::ownership::safe_root::{
    SafePruneRoot, TrustedOwners,
};
use accelerator::launch::outbound::resolve::ownership::{
    Ownership, PrimaryGroup, ProcessOwnership,
};
use accelerator::launch::outbound::resolve::prune_report::{
    NoPruneSteps, PruneStep, PruneSteps, UntrustedDirectory,
};
use accelerator::launch::outbound::resolve::retention::ports::CachePorts;
use accelerator::launch::outbound::resolve::retention::residue::BootstrapHolderLiveness;
use accelerator::launch::outbound::resolve::retention::root_lock::ROOT_LOCK;
use accelerator::launch::outbound::resolve::retention::version_claims::{
    self, VersionClaim, VERSION_CLAIMS_DIR,
};
use accelerator::launch::outbound::resolve::retention_windows::{
    AGE_BACKSTOP, CLAIM_WINDOW,
};
use accelerator::launch::outbound::resolve::single_flight::{
    LockGuard, LockPatience, LockProbe, SingleFlightLocks,
};
use accelerator::launch::outbound::resolve::tree::layout::{
    Artifact, TreePaths,
};
use accelerator::launch::outbound::resolve::tree::lease::{
    HeldLeases, Liveness, LivenessProbe,
};
use accelerator::launch::outbound::resolve::tree::locks::{
    FlockLocks, TreeLocks,
};
use accelerator::launch::outbound::resolve::tree::reap::{
    RemovalObserver, UnobservedRemovals,
};
use accelerator::launch::outbound::resolve::tree::{
    ExpectedDigests, NoSteps, TreePrune, TreePrunePorts, TreePruneRegime,
    TreePruneReport,
};
use accelerator::launch::outbound::resolve::HOST_PLATFORM;
use accelerator::launch::start_up::{
    start_up, DispatchConfig, StartUp, TreeConfig,
};
use common::recording_filesystem::{
    in_root, FilesystemCall, RecordedPath, RecordingFilesystem, ScriptedEvent,
    TimelineEvent,
};
use common::recording_locks::{LockCall, RecordingLocks};
use common::scripted_filesystem::ScriptedFilesystem;
use common::signed_release::{SignedRelease, BINARY};
use common::trees::{
    another_platform, digest, exists, on_host, set_mode, set_mtime,
    FailingLockAt, FixedClock, FixedLiveness, PlantedTree, Trees,
};

const OLD: &str = "1.0.0";
const RUNNING: &str = env!("CARGO_PKG_VERSION");
const DIGEST: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const OTHER_ID: &str = "fedcba9876543210";
const BOUND: Duration = Duration::from_secs(10);

#[derive(Default)]
struct LiveHolders(Vec<i32>);

impl BootstrapHolderLiveness for LiveHolders {
    fn is_alive(&self, pid: i32) -> bool {
        self.0.contains(&pid)
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs()
}

struct Ports {
    filesystem: Arc<dyn CacheFilesystem + Send + Sync>,
    ownership: Arc<dyn Ownership + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    locks: Arc<dyn SingleFlightLocks + Send + Sync>,
    steps: Arc<dyn PruneSteps + Send + Sync>,
    bootstrap_holders: Arc<dyn BootstrapHolderLiveness + Send + Sync>,
    lease_liveness: Arc<dyn LivenessProbe + Send + Sync>,
    pins: ExpectedDigests,
    removals: Arc<dyn RemovalObserver + Send + Sync>,
}

impl Ports {
    fn build(self) -> Arc<CachePorts> {
        Arc::new(CachePorts {
            filesystem: self.filesystem,
            ownership: self.ownership,
            clock: self.clock,
            locks: self.locks,
            steps: self.steps,
            bootstrap_holders: self.bootstrap_holders,
            lease_liveness: self.lease_liveness,
            pins: Arc::new(self.pins),
            removals: self.removals,
        })
    }
}

struct Verb {
    outcome: Result<(), kernel::Error>,
    out: String,
    err: String,
}

impl Verb {
    #[track_caller]
    fn succeeded(&self) -> &Self {
        assert!(self.outcome.is_ok(), "{:?}\n{}", self.outcome, self.err);
        self
    }

    #[track_caller]
    fn failed(&self) -> String {
        match &self.outcome {
            Err(error) => error.to_string(),
            Ok(()) => panic!("the prune succeeded: {}", self.out),
        }
    }
}

/// A canonical `0700` relocated root beside a plugin root, made with explicit
/// modes so the host's umask and temp-path ancestry decide nothing.
struct RootHarness {
    anchor: TempDir,
    root: PathBuf,
    plugin: PathBuf,
    now: u64,
}

impl RootHarness {
    fn new() -> Self {
        let anchor = tempfile::tempdir().expect("anchor");
        let canonical = anchor.path().canonicalize().expect("canonical");
        let root = canonical.join("cache");
        std::fs::create_dir(&root).expect("root");
        set_mode(&root, 0o700);
        let plugin = canonical.join("plugin");
        std::fs::create_dir_all(plugin.join("bin")).expect("plugin bin");
        assert!(
            SafePruneRoot::check(
                &root,
                &SystemFilesystem,
                &ProcessOwnership::default(),
                &TrustedOwners::none(),
            )
            .is_ok(),
            "the harness root must be safe to prune on this host"
        );
        Self {
            anchor,
            root,
            plugin,
            now: now(),
        }
    }

    fn join(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn has(&self, name: &str) -> bool {
        self.join(name).symlink_metadata().is_ok()
    }

    const fn aged(&self) -> u64 {
        self.now - AGE_BACKSTOP.as_secs()
    }

    const fn young(&self) -> u64 {
        self.now - 60
    }

    /// A clock far enough on that everything this test made is aged.
    fn later(&self) -> Arc<dyn Clock + Send + Sync> {
        Arc::new(FixedClock(self.now + 2 * AGE_BACKSTOP.as_secs()))
    }

    fn file(&self, name: &str, modified: u64) -> PathBuf {
        let path = self.join(name);
        std::fs::write(&path, b"x").expect("seeded file");
        set_mtime(&path, modified);
        path
    }

    fn entry(&self, version: &str, modified: u64) -> String {
        self.named_entry(&format!("vcs-{version}-{DIGEST}"), modified)
    }

    fn named_entry(&self, stem: &str, modified: u64) -> String {
        self.file(stem, modified);
        self.file(&format!("{stem}.minisig"), modified);
        stem.to_owned()
    }

    fn claims(&self) -> PathBuf {
        self.join(VERSION_CLAIMS_DIR)
    }

    fn claim(&self, version: &str, modified: u64) -> PathBuf {
        if !self.claims().exists() {
            std::fs::create_dir(self.claims()).expect("claims");
            set_mode(&self.claims(), 0o700);
        }
        let path = self.claims().join(format!("{version}.{OTHER_ID}"));
        std::fs::write(&path, b"").expect("claim");
        set_mtime(&path, modified);
        path
    }

    fn trees(&self) -> Trees {
        Trees::under(&self.root)
    }

    fn recorder(&self) -> RecordingFilesystem {
        RecordingFilesystem::new(&self.root, self.anchor.path())
    }

    fn ports(&self) -> Ports {
        Ports {
            filesystem: Arc::new(SystemFilesystem),
            ownership: Arc::new(ProcessOwnership::default()),
            clock: Arc::new(FixedClock(self.now)),
            locks: Arc::new(FlockLocks),
            steps: Arc::new(NoPruneSteps),
            bootstrap_holders: Arc::new(LiveHolders::default()),
            lease_liveness: Arc::new(FixedLiveness(Liveness::Free)),
            pins: ExpectedDigests::Compiled,
            removals: Arc::new(UnobservedRemovals),
        }
    }

    fn start_over(
        &self,
        cache_dir: &Path,
        trusted_owners: TrustedOwners,
        ports: Ports,
    ) -> StartUp {
        start_up(
            &CacheRootConfig {
                cache_dir_override: Some(cache_dir.to_path_buf()),
                plugin_root: Some(self.plugin.clone()),
            },
            trusted_owners,
            ports.build(),
        )
    }

    fn prune(&self, ports: Ports) -> Verb {
        self.prune_older_than(ports, None)
    }

    fn prune_older_than(&self, ports: Ports, older_than: Option<u64>) -> Verb {
        prune_through(
            &self.start_over(&self.root, TrustedOwners::none(), ports),
            older_than,
        )
    }
}

fn prune_through(start: &StartUp, older_than: Option<u64>) -> Verb {
    let keys = TrustedKeys::embedded().expect("embedded keys");
    let fetcher =
        Fetcher::with_backoff(Duration::from_millis(1)).expect("fetcher");
    let resolver = start
        .tree_resolver(TreeConfig {
            base_url: "http://127.0.0.1:9".to_owned(),
            keys: &keys,
            fetcher: &fetcher,
            expected_digests: ExpectedDigests::Compiled,
            waiter_bound: Duration::from_secs(1),
            steps: &NoSteps,
        })
        .expect("tree resolver");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let outcome = cache::run(
        &CacheAction::Prune { older_than },
        &resolver,
        start.retention(),
        &mut out,
        &mut err,
    );
    Verb {
        outcome,
        out: String::from_utf8(out).expect("utf-8"),
        err: String::from_utf8(err).expect("utf-8"),
    }
}

#[test]
fn foreign_names_survive_every_trigger() {
    for trigger in EVERY_TRIGGER {
        foreign_names_survive(trigger);
    }
}

fn foreign_names_survive(trigger: Trigger) {
    let harness = RootHarness::new();
    let foreign = [
        ".tmp-foreign".to_owned(),
        format!("x-y-{DIGEST}"),
        ".foreign-dotfile".to_owned(),
    ];
    for name in &foreign {
        harness.file(name, harness.aged());
    }
    harness.claim(RUNNING, harness.now);
    let foreign_claim = harness.claims().join("not-a-claim");
    std::fs::write(&foreign_claim, b"").expect("foreign claim");
    set_mtime(&foreign_claim, harness.now - 2 * CLAIM_WINDOW.as_secs());
    let unclaimed = harness.entry(OLD, harness.aged());
    let trees = harness.trees();
    let foreign_in_trees = [
        trees.root().join(".tmp-foreign"),
        trees.root().join("notes.txt"),
        trees.root().join("backup.ref"),
    ];
    for path in &foreign_in_trees {
        std::fs::write(path, b"x").expect("foreign file");
        set_mtime(path, harness.aged());
    }
    let foreign_directories = [
        trees.plant_generation("backup-2021-1-final", harness.aged()),
        trees.plant_generation(
            &format!("driver-plan9-x64-{DIGEST}-1-0011223344556677"),
            harness.aged(),
        ),
    ];
    let unclaimed_tree =
        trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());

    let fired = harness.fire(trigger, harness.ports());

    if let Fired::Verb(verb) = &fired {
        verb.succeeded();
        assert!(verb.out.contains("reclaimed 2 entries"), "{}", verb.out);
    }
    if !fired.ran() {
        return;
    }
    assert!(foreign.iter().all(|name| harness.has(name)));
    assert!(foreign_claim.exists());
    assert!(foreign_in_trees.iter().all(|path| exists(path)));
    assert!(foreign_directories.iter().all(|path| exists(path)));
    assert!(
        !harness.has(&unclaimed),
        "{trigger:?}: the positive control survived"
    );
    unclaimed_tree.assert_reclaimed();
}

#[test]
fn the_kept_names_survive_beside_an_unclaimed_version() {
    for trigger in EVERY_TRIGGER {
        kept_names_survive(trigger);
    }
}

fn kept_names_survive(trigger: Trigger) {
    let harness = RootHarness::new();
    let lock = harness.join(&format!(".accelerator-lock-{HOST_PLATFORM}"));
    std::fs::create_dir(&lock).expect("lock");
    set_mode(&lock, 0o755);
    std::fs::write(lock.join("pid"), "4242\n").expect("pid");
    set_mtime(&lock, harness.aged());
    let kept = [
        ".accelerator-probe-77".to_owned(),
        ".accelerator-unverified.log".to_owned(),
        "accelerator-launcher-1.0.0-plan9-x64".to_owned(),
        "accelerator-launcher-1.0.0-plan9-x64.minisig".to_owned(),
        format!("lone-1.0.0-{DIGEST}.minisig"),
    ];
    harness.file(&kept[0], harness.young());
    for name in &kept[1..] {
        harness.file(name, harness.aged());
    }
    std::fs::create_dir(harness.join("trees")).expect("trees");
    let unclaimed = harness.entry(OLD, harness.aged());

    let fired = harness.fire(
        trigger,
        Ports {
            bootstrap_holders: Arc::new(LiveHolders(vec![4242])),
            ..harness.ports()
        },
    );

    if let Fired::Verb(verb) = &fired {
        verb.succeeded();
    }
    if !fired.ran() {
        return;
    }

    assert!(lock.join("pid").exists());
    assert!(kept.iter().all(|name| harness.has(name)));
    assert!(harness.has("trees"));
    assert!(!harness.has(&unclaimed), "{trigger:?}");
}

#[test]
fn the_verb_names_unreadable_claims_and_exits_zero() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    std::fs::create_dir(harness.claims()).expect("claims");
    set_mode(&harness.claims(), 0o000);

    let verb = harness.prune(harness.ports());

    set_mode(&harness.claims(), 0o700);
    verb.succeeded();
    assert!(harness.has(&entry));
    assert!(
        verb.err.contains("claims unreadable (permission denied)"),
        "{}",
        verb.err
    );
}

#[test]
fn a_lock_directory_reclaimed_before_the_unlink_is_kept() {
    let harness = RootHarness::new();
    let lock = harness.join(&format!(".accelerator-lock-{HOST_PLATFORM}"));
    std::fs::create_dir(&lock).expect("lock");
    set_mode(&lock, 0o755);
    let pid = lock.join("pid");
    std::fs::write(&pid, "4343\n").expect("dead pid");
    set_mtime(&lock, harness.aged());
    let recorder = harness.recorder();
    let reclaimed_by = pid.clone();
    let filesystem = ScriptedFilesystem::over(recorder.clone()).before(
        FilesystemCall::ReadBounded(in_root("")).kind(),
        &pid,
        2,
        move || std::fs::write(&reclaimed_by, "4444\n").expect("reclaimed"),
    );

    harness
        .prune(Ports {
            filesystem: Arc::new(filesystem),
            ..harness.ports()
        })
        .succeeded();

    assert!(pid.exists(), "the reclaimed lock was swept");
    let read_pid = FilesystemCall::ReadBounded(recorder.recorded(&pid));
    let timeline = recorder.timeline();
    let ran = timeline
        .iter()
        .position(|event| {
            matches!(event, TimelineEvent::Scripted(ScriptedEvent::Ran { .. }))
        })
        .expect("the script ran");
    assert_eq!(timeline.get(ran + 1), Some(&TimelineEvent::Call(read_pid)));
}

/// Pauses the prune at `at` until released, so another thread can act in
/// the gap.
struct Pause {
    at: PruneStep,
    reached: Mutex<Option<mpsc::Sender<()>>>,
    resume: Mutex<mpsc::Receiver<()>>,
}

impl PruneSteps for Pause {
    fn reached(&self, step: PruneStep) {
        if step != self.at {
            return;
        }
        let announce = self.reached.lock().expect("reached").take();
        if let Some(announce) = announce {
            announce.send(()).expect("announced");
            self.resume
                .lock()
                .expect("resume")
                .recv_timeout(BOUND)
                .expect("resumed");
        }
    }
}

/// Runs a prune paused at `at`, stores an entry of `version` in the gap,
/// claiming it first when `claimed`, and reports whether the entry survived.
fn a_store_survives(at: PruneStep, claimed: bool) -> bool {
    let harness = Arc::new(RootHarness::new());
    let version = "5.0.0";
    let (announce, announced) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let pause = Arc::new(Pause {
        at,
        reached: Mutex::new(Some(announce)),
        resume: Mutex::new(released),
    });
    let clock_now = harness.now + AGE_BACKSTOP.as_secs() + 600;
    let pruner = {
        let harness = Arc::clone(&harness);
        std::thread::spawn(move || {
            harness
                .prune(Ports {
                    steps: pause,
                    clock: Arc::new(FixedClock(clock_now)),
                    ..harness.ports()
                })
                .outcome
                .is_ok()
        })
    };
    announced.recv_timeout(BOUND).expect("the prune paused");
    if claimed {
        version_claims::refresh(
            &harness.root,
            &VersionClaim {
                version,
                launcher_id: OTHER_ID,
            },
            &SystemFilesystem,
            &ProcessOwnership::default(),
            &FixedClock(harness.now),
        );
    }
    let stored = store(
        &SystemFilesystem,
        &harness.root,
        "vcs",
        version,
        DIGEST,
        b"binary",
        "signature",
    )
    .expect("stored");
    release.send(()).expect("resumed");
    assert!(pruner.join().expect("pruner"), "the prune failed");
    stored.path.exists()
}

#[test]
fn a_store_after_listing_survives_a_paused_prune() {
    assert!(a_store_survives(PruneStep::Listed, false));
}

#[test]
fn a_store_before_listing_survives_a_paused_prune() {
    assert!(a_store_survives(PruneStep::AboutToList, true));
}

#[test]
fn a_store_without_a_claim_behind_a_paused_prune_is_removed() {
    assert!(!a_store_survives(PruneStep::AboutToList, false));
}

#[test]
fn single_file_prune_lists_the_root_before_reading_claims() {
    let harness = RootHarness::new();
    harness.entry(OLD, harness.aged());
    let recorder = harness.recorder();
    let start = harness.start_over(
        &harness.root,
        TrustedOwners::none(),
        Ports {
            filesystem: Arc::new(recorder.clone()),
            ..harness.ports()
        },
    );
    let start_up_calls = recorder.calls().len();

    prune_through(&start, None).succeeded();

    let calls = recorder.calls().split_off(start_up_calls);
    let listed = calls
        .iter()
        .position(|call| call == &FilesystemCall::List(in_root("")))
        .expect("the root was listed");
    let first_claims_call = calls
        .iter()
        .position(|call| {
            call.paths().iter().any(|path| {
                matches!(path, RecordedPath::InRoot(relative)
                    if relative.starts_with(VERSION_CLAIMS_DIR))
            })
        })
        .expect("claims were read");
    assert!(listed < first_claims_call, "{calls:#?}");
}

#[test]
fn a_last_claim_goes_only_after_its_versions_entries() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    let claim = harness.claim(OLD, harness.now - CLAIM_WINDOW.as_secs());
    let shared = "2.0.0";
    let shared_entry = harness.entry(shared, harness.aged());
    let shared_stale =
        harness.claim(shared, harness.now - CLAIM_WINDOW.as_secs());
    let shared_fresh =
        harness.claims().join(format!("{shared}.0123456789abcdef"));
    std::fs::write(&shared_fresh, b"").expect("fresh claim");
    let recorder = harness.recorder();

    harness
        .prune(Ports {
            filesystem: Arc::new(recorder.clone()),
            ..harness.ports()
        })
        .succeeded();

    let removals: Vec<FilesystemCall> = recorder
        .calls()
        .into_iter()
        .filter(|call| matches!(call, FilesystemCall::RemoveFile(_)))
        .collect();
    let of_the_last_claimed_version: Vec<&FilesystemCall> = removals
        .iter()
        .filter(|call| {
            **call
                != FilesystemCall::RemoveFile(recorder.recorded(&shared_stale))
        })
        .collect();
    assert_eq!(
        of_the_last_claimed_version,
        [
            &FilesystemCall::RemoveFile(in_root(format!("{entry}.minisig"))),
            &FilesystemCall::RemoveFile(in_root(&entry)),
            &FilesystemCall::RemoveFile(recorder.recorded(&claim)),
        ]
    );
    assert_eq!(removals.len(), 4, "{removals:#?}");
    assert!(harness.has(&shared_entry));
    assert!(shared_fresh.exists());
}

#[test]
fn the_verb_waits_for_a_held_root_lock() {
    let harness = Arc::new(RootHarness::new());
    let entry = harness.entry(OLD, harness.aged());
    let held = FlockLocks.take(&harness.join(ROOT_LOCK)).expect("held");
    let (done, finished) = mpsc::channel();
    let pruner = {
        let harness = Arc::clone(&harness);
        std::thread::spawn(move || {
            let succeeded = harness.prune(harness.ports()).outcome.is_ok();
            done.send(()).expect("announced");
            succeeded
        })
    };

    assert!(
        finished.recv_timeout(Duration::from_millis(300)).is_err(),
        "the verb did not wait for the held lock"
    );
    assert!(harness.has(&entry));
    drop(held);

    finished.recv_timeout(BOUND).expect("the verb finished");
    assert!(pruner.join().expect("pruner"));
    assert!(!harness.has(&entry));
}

struct FailingLocks;

impl SingleFlightLocks for FailingLocks {
    fn take(&self, _path: &Path) -> Result<LockGuard, TreeError> {
        Err(TreeError::Lease {
            detail: "No locks available (os error 77)".to_owned(),
        })
    }

    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError> {
        self.take(path).map(Some)
    }

    fn wait_existing(
        &self,
        path: &Path,
    ) -> Result<Option<LockGuard>, TreeError> {
        self.take(path).map(Some)
    }

    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError> {
        self.take(path).map(LockProbe::Taken)
    }
}

#[test]
fn the_verb_reports_a_failed_root_lock_and_exits_non_zero() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());

    let verb = harness.prune(Ports {
        locks: Arc::new(FailingLocks),
        ..harness.ports()
    });

    verb.failed();
    assert!(harness.has(&entry));
    assert!(verb.err.contains(ROOT_LOCK), "{}", verb.err);
    assert!(verb.err.contains("No locks available"), "{}", verb.err);
}

#[test]
fn a_symlinked_or_foreign_root_lock_is_a_lock_failure() {
    let symlinked = RootHarness::new();
    let target = symlinked.file("target", symlinked.now);
    std::os::unix::fs::symlink(&target, symlinked.join(ROOT_LOCK))
        .expect("symlink");
    let entry = symlinked.entry(OLD, symlinked.aged());
    let verb = symlinked.prune(symlinked.ports());
    verb.failed();
    assert!(symlinked.has(&entry));
    assert_eq!(std::fs::read(&target).expect("target"), b"x");

    let foreign = RootHarness::new();
    let lock = foreign.file(ROOT_LOCK, foreign.now);
    let filesystem = ReportingFilesystem::over_system().reporting(
        &lock,
        EntryMetadata {
            owner: ProcessOwnership::default().effective_uid() + 1,
            ..SystemFilesystem.entry(&lock).expect("lock metadata")
        },
    );
    let entry = foreign.entry(OLD, foreign.aged());
    let verb = foreign.prune(Ports {
        filesystem: Arc::new(filesystem),
        ..foreign.ports()
    });
    verb.failed();
    assert!(foreign.has(&entry));
    assert!(lock.exists());
}

#[test]
fn a_relocated_root_verb_still_reclaims_an_unclaimed_superseded_pinned_tree() {
    let harness = RootHarness::new();
    let trees = harness.join("trees");
    std::fs::create_dir(&trees).expect("trees");
    set_mode(&trees, 0o700);
    let superseded = "d".repeat(64);
    let generation =
        format!("driver-{HOST_PLATFORM}-{superseded}-1-0011223344556677");
    std::fs::create_dir(trees.join(&generation)).expect("generation");
    let pointer =
        trees.join(format!("driver-{HOST_PLATFORM}-{superseded}.ref"));
    std::fs::write(&pointer, &generation).expect("pointer");
    set_mtime(
        &trees.join(&generation),
        harness.now - 2 * AGE_BACKSTOP.as_secs(),
    );

    harness
        .prune(Ports {
            pins: ExpectedDigests::Fixed(BTreeMap::from([(
                ("driver".to_owned(), HOST_PLATFORM.to_owned()),
                "a".repeat(64),
            )])),
            ..harness.ports()
        })
        .succeeded();

    assert!(!trees.join(&generation).exists());
    assert!(!pointer.exists());
}

fn shared_group_writable(path: &Path) -> EntryMetadata {
    EntryMetadata {
        mode: 0o040_770,
        group: ProcessOwnership::default().effective_gid() + 1,
        ..SystemFilesystem.entry(path).expect("metadata")
    }
}

#[test]
fn the_verb_refuses_an_unsafe_root_and_exits_non_zero() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    let filesystem = ReportingFilesystem::over_system()
        .reporting(&harness.root, shared_group_writable(&harness.root));

    let verb = harness.prune(Ports {
        filesystem: Arc::new(filesystem),
        ..harness.ports()
    });

    let message = verb.failed();
    assert!(harness.has(&entry));
    assert!(
        message.starts_with(&format!(
            "not pruning: {} is group-writable",
            harness.root.display()
        )),
        "{message}"
    );
    assert!(
        message.contains(&format!("chmod g-w {}", harness.root.display())),
        "{message}"
    );
}

#[test]
fn claims_are_still_written_in_an_unsafe_root() {
    let harness = RootHarness::new();
    let filesystem = ReportingFilesystem::over_system()
        .reporting(&harness.root, shared_group_writable(&harness.root));

    harness.prune(Ports {
        filesystem: Arc::new(filesystem),
        ..harness.ports()
    });

    let claims = std::fs::read_dir(harness.claims())
        .expect("claims written")
        .flatten()
        .filter(|claim| {
            claim
                .file_name()
                .to_string_lossy()
                .starts_with(&format!("{RUNNING}."))
        })
        .count();
    assert_eq!(claims, 1);
}

#[test]
fn a_root_reached_through_a_symlinked_ancestor_is_pruned() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    let link = harness.anchor.path().join("ancestor-link");
    std::os::unix::fs::symlink(harness.anchor.path(), &link)
        .expect("symlinked ancestor");

    let verb = prune_through(
        &harness.start_over(
            &link.join("cache"),
            TrustedOwners::none(),
            harness.ports(),
        ),
        None,
    );

    verb.succeeded();
    assert!(!harness.has(&entry));
}

#[test]
fn a_symlinked_root_is_pruned_through_its_target() {
    for spelling in ["root-link", "root-link/"] {
        let harness = RootHarness::new();
        let entry = harness.entry(OLD, harness.aged());
        std::os::unix::fs::symlink(
            &harness.root,
            harness.anchor.path().join("root-link"),
        )
        .expect("symlinked root");

        let verb = prune_through(
            &harness.start_over(
                &harness.anchor.path().join(spelling),
                TrustedOwners::none(),
                harness.ports(),
            ),
            None,
        );

        verb.succeeded();
        assert!(!harness.has(&entry), "{spelling}");
    }
}

#[test]
fn a_symlinked_root_with_an_unsafe_target_is_not_pruned() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    let link = harness.anchor.path().join("root-link");
    std::os::unix::fs::symlink(&harness.root, &link).expect("symlinked root");
    let filesystem = ReportingFilesystem::over_system()
        .reporting(&harness.root, shared_group_writable(&harness.root));

    let verb = prune_through(
        &harness.start_over(
            &link,
            TrustedOwners::none(),
            Ports {
                filesystem: Arc::new(filesystem),
                ..harness.ports()
            },
        ),
        None,
    );

    verb.failed();
    assert!(harness.has(&entry));
}

#[test]
fn a_listed_owner_reaches_the_relocated_prune() {
    let foreign_uid = ProcessOwnership::default().effective_uid() + 1;
    for (trusted, pruned) in [
        (
            TrustedOwners::parse(&foreign_uid.to_string()).expect("parses"),
            true,
        ),
        (TrustedOwners::none(), false),
    ] {
        let harness = RootHarness::new();
        let entry = harness.entry(OLD, harness.aged());
        let ancestor = harness.root.parent().expect("ancestor").to_path_buf();
        let filesystem = ReportingFilesystem::over_system().reporting(
            &ancestor,
            EntryMetadata {
                owner: foreign_uid,
                ..SystemFilesystem.entry(&ancestor).expect("metadata")
            },
        );

        let verb = prune_through(
            &harness.start_over(
                &harness.root,
                trusted,
                Ports {
                    filesystem: Arc::new(filesystem),
                    ..harness.ports()
                },
            ),
            None,
        );

        assert_eq!(verb.outcome.is_ok(), pruned, "{}", verb.err);
        assert_eq!(!harness.has(&entry), pruned);
    }
}

#[test]
fn the_verb_reports_a_failed_removal_and_exits_non_zero() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    let signature = harness.join(&format!("{entry}.minisig"));
    let filesystem = ScriptedFilesystem::over(harness.recorder()).failing(
        FilesystemCall::RemoveFile(in_root("")).kind(),
        &signature,
        1,
        std::io::ErrorKind::PermissionDenied,
    );

    let verb = harness.prune(Ports {
        filesystem: Arc::new(filesystem),
        ..harness.ports()
    });

    verb.failed();
    assert!(harness.has(&entry));
    assert!(
        verb.err.contains(&format!(
            "could not remove {}: permission denied",
            signature.display()
        )),
        "{}",
        verb.err
    );
}

#[test]
fn a_claim_refreshed_before_unlink_is_kept() {
    let harness = RootHarness::new();
    let claim = harness.claim(OLD, harness.now - CLAIM_WINDOW.as_secs());
    let recorder = harness.recorder();
    let refreshed = claim.clone();
    let now = harness.now;
    let filesystem =
        Arc::new(ScriptedFilesystem::over(recorder.clone()).before(
            FilesystemCall::Entry(in_root("")).kind(),
            &claim,
            2,
            move || set_mtime(&refreshed, now),
        ));

    harness
        .prune(Ports {
            filesystem: Arc::clone(&filesystem)
                as Arc<dyn CacheFilesystem + Send + Sync>,
            ..harness.ports()
        })
        .succeeded();

    assert!(claim.exists());
    assert_eq!(filesystem.fired().len(), 1, "the script never ran");
    let timeline = recorder.timeline();
    let ran = timeline
        .iter()
        .position(|event| matches!(event, TimelineEvent::Scripted(_)))
        .expect("scripted");
    assert_eq!(
        timeline.get(ran + 1),
        Some(&TimelineEvent::Call(FilesystemCall::Entry(
            recorder.recorded(&claim)
        )))
    );
}

#[test]
fn a_failed_removal_keeps_the_versions_claim() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    let claim = harness.claim(OLD, harness.now - CLAIM_WINDOW.as_secs());
    let filesystem = ScriptedFilesystem::over(harness.recorder()).failing(
        FilesystemCall::RemoveFile(in_root("")).kind(),
        &harness.join(&entry),
        1,
        std::io::ErrorKind::PermissionDenied,
    );

    harness
        .prune(Ports {
            filesystem: Arc::new(filesystem),
            ..harness.ports()
        })
        .failed();

    assert!(claim.exists());
}

#[test]
fn a_removal_failing_with_enoent_is_not_an_error() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    let filesystem = ScriptedFilesystem::over(harness.recorder()).failing(
        FilesystemCall::RemoveFile(in_root("")).kind(),
        &harness.join(&entry),
        1,
        std::io::ErrorKind::NotFound,
    );

    let verb = harness.prune(Ports {
        filesystem: Arc::new(filesystem),
        ..harness.ports()
    });

    verb.succeeded();
    assert!(verb.err.is_empty(), "{}", verb.err);
}

#[test]
fn older_than_narrows_the_version_claim_window_through_the_verb() {
    let day = 24 * 60 * 60;
    for (older_than, survives) in [(Some(day), false), (None, true)] {
        let harness = RootHarness::new();
        let entry = harness.entry(OLD, harness.aged());
        let claim = harness.claim(OLD, harness.now - 2 * day);

        harness
            .prune_older_than(harness.ports(), older_than)
            .succeeded();

        assert_eq!(harness.has(&entry), survives, "{older_than:?}");
        assert_eq!(exists(&claim), survives, "{older_than:?}");
    }
}

#[test]
fn the_reclaimed_count_counts_an_entry_and_its_signature_once() {
    let harness = RootHarness::new();
    harness.entry(OLD, harness.aged());

    let verb = harness.prune(harness.ports());

    assert!(
        verb.succeeded().out.contains("reclaimed 1 entries"),
        "{}",
        verb.out
    );
}

fn pinning(artifact: &str, digest: &str) -> ExpectedDigests {
    ExpectedDigests::Fixed(BTreeMap::from([(
        (artifact.to_owned(), HOST_PLATFORM.to_owned()),
        digest.to_owned(),
    )]))
}

#[test]
fn unclaimed_retired_and_foreign_platform_trees_go_and_claimed_ones_stay() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let retired =
        trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());
    let foreign_platform = trees.plant_tree(
        &Artifact::new("driver", another_platform()),
        &digest('b'),
        harness.aged(),
    );
    let claimed = trees.plant_tree(
        &Artifact::new("driver", another_platform()),
        &digest('c'),
        harness.aged(),
    );
    let claim = trees.plant_claim(&digest('c'), OTHER_ID, harness.now);

    harness.prune(harness.ports()).succeeded();

    retired.assert_reclaimed();
    foreign_platform.assert_reclaimed();
    claimed.assert_survives();
    assert!(claim.exists());
}

#[test]
fn only_aged_unknown_and_free_unpinned_trees_are_reclaimed() {
    for (liveness, aged, pinned, reclaimed) in [
        (Liveness::Held, true, false, false),
        (Liveness::Unknown, false, false, false),
        (Liveness::Unknown, true, false, true),
        (Liveness::Free, false, false, true),
        (Liveness::Free, false, true, false),
    ] {
        let harness = RootHarness::new();
        let modified = if aged {
            harness.aged()
        } else {
            harness.young()
        };
        let tree = harness.trees().plant_tree(
            &on_host("driver"),
            &digest('a'),
            modified,
        );
        let pins = if pinned {
            pinning("driver", &digest('a'))
        } else {
            ExpectedDigests::Fixed(BTreeMap::new())
        };

        harness
            .prune(Ports {
                lease_liveness: Arc::new(FixedLiveness(liveness)),
                pins,
                ..harness.ports()
            })
            .succeeded();

        assert_eq!(
            !exists(&tree.generation),
            reclaimed,
            "{liveness:?}, aged {aged}, pinned {pinned}"
        );
        assert_eq!(exists(&tree.pointer), exists(&tree.generation));
    }
}

#[test]
fn only_tree_claims_outside_the_window_are_removed() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let stale_seconds = harness.now - CLAIM_WINDOW.as_secs() - 1;
    let live = trees.plant_tree(&on_host("retired"), &digest('a'), harness.now);
    let stale = trees.plant_claim(&digest('a'), OTHER_ID, stale_seconds);
    let fresh =
        trees.plant_claim(&digest('a'), "0123456789abcdef", harness.now - 60);
    let unparsed_launcher =
        trees.plant_claim(&digest('c'), "test-install", stale_seconds);
    let foreign = trees.claims().join("not-a-claim");
    std::fs::write(&foreign, b"").expect("foreign");
    set_mtime(&foreign, stale_seconds);

    harness.prune(harness.ports()).succeeded();

    live.assert_survives();
    assert!(!stale.exists(), "a stale claim survived");
    assert!(fresh.exists());
    assert!(unparsed_launcher.exists());
    assert!(foreign.exists());
}

#[test]
fn a_tree_claim_exactly_at_the_window_is_stale() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let at_the_window = harness.now - CLAIM_WINDOW.as_secs();
    let tree =
        trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());
    trees.plant_claim(&digest('a'), OTHER_ID, at_the_window);
    let treeless = trees.plant_claim(&digest('b'), OTHER_ID, at_the_window);

    harness.prune(harness.ports()).succeeded();

    tree.assert_reclaimed();
    assert!(!treeless.exists());
}

#[test]
fn older_than_narrows_the_tree_claim_window() {
    let day = 24 * 60 * 60;
    for (older_than, survives) in [(Some(day), false), (None, true)] {
        let harness = RootHarness::new();
        let trees = harness.trees();
        let tree =
            trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());
        let claim =
            trees.plant_claim(&digest('a'), OTHER_ID, harness.now - 2 * day);

        harness
            .prune_older_than(harness.ports(), older_than)
            .succeeded();

        assert_eq!(exists(&tree.generation), survives, "{older_than:?}");
        assert_eq!(exists(&claim), survives, "{older_than:?}");
    }
}

#[test]
fn the_verb_reclaims_single_file_entries_and_an_unclaimed_tree() {
    let harness = RootHarness::new();
    let entry = harness.entry(OLD, harness.aged());
    let tree = harness.trees().plant_tree(
        &on_host("retired"),
        &digest('a'),
        harness.aged(),
    );

    let verb = harness.prune(harness.ports());

    assert!(
        verb.succeeded().out.contains("reclaimed 2 entries"),
        "{}",
        verb.out
    );
    assert!(!harness.has(&entry));
    tree.assert_reclaimed();
}

#[test]
fn the_pointer_goes_before_its_generation() {
    let harness = RootHarness::new();
    let tree = harness.trees().plant_tree(
        &on_host("retired"),
        &digest('a'),
        harness.aged(),
    );
    let recorder = harness.recorder();

    harness
        .prune(Ports {
            filesystem: Arc::new(recorder.clone()),
            ..harness.ports()
        })
        .succeeded();

    let calls = recorder.calls();
    let position = |wanted: FilesystemCall| {
        calls
            .iter()
            .position(|call| call == &wanted)
            .unwrap_or_else(|| panic!("no {wanted:?} in {calls:#?}"))
    };
    assert!(
        position(FilesystemCall::RemoveFile(recorder.recorded(&tree.pointer)))
            < position(FilesystemCall::RemoveTree(
                recorder.recorded(&tree.generation)
            ))
    );
}

#[test]
fn the_verb_waits_for_a_held_tree_lock() {
    let harness = Arc::new(RootHarness::new());
    let trees = harness.trees();
    let retired = on_host("retired");
    let tree = trees.plant_tree(&retired, &digest('a'), harness.aged());
    let held = FlockLocks
        .take(&trees.paths().single_flight_lock(&retired))
        .expect("held");
    let (done, finished) = mpsc::channel();
    let pruner = {
        let harness = Arc::clone(&harness);
        std::thread::spawn(move || {
            let succeeded = harness.prune(harness.ports()).outcome.is_ok();
            done.send(()).expect("announced");
            succeeded
        })
    };

    assert!(
        finished.recv_timeout(Duration::from_millis(300)).is_err(),
        "the verb did not wait for the held tree lock"
    );
    tree.assert_survives();
    drop(held);

    finished.recv_timeout(BOUND).expect("the verb finished");
    assert!(pruner.join().expect("pruner"));
    tree.assert_reclaimed();
}

#[test]
fn the_verb_reports_a_failed_tree_lock_and_exits_non_zero() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let unlockable = on_host("retired-x");
    let x = trees.plant_tree(&unlockable, &digest('a'), harness.aged());
    let y =
        trees.plant_tree(&on_host("retired-y"), &digest('b'), harness.aged());
    let lock = trees.paths().single_flight_lock(&unlockable);

    let verb = harness.prune(Ports {
        locks: Arc::new(FailingLockAt(lock.clone())),
        ..harness.ports()
    });

    verb.failed();
    x.assert_survives();
    y.assert_reclaimed();
    assert!(
        verb.err.contains(&lock.display().to_string()),
        "{}",
        verb.err
    );
    assert!(verb.err.contains("No locks available"), "{}", verb.err);
}

#[test]
fn old_orphans_sidecars_and_retired_locks_go_and_young_generations_stay() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let orphaned = Trees::generation_of(&on_host("retired-a"), &digest('a'));
    let sidecars = trees.plant_sidecars(&orphaned, harness.aged());
    let lock = trees.plant_lock(&on_host("retired-b"), harness.aged());
    let old = trees.plant_generation(
        &Trees::generation_of(&on_host("retired-c"), &digest('d')),
        harness.aged(),
    );
    let young = trees.plant_generation(
        &Trees::generation_of(&on_host("retired-c"), &digest('c')),
        harness.young(),
    );

    harness.prune(harness.ports()).succeeded();

    assert!(sidecars.iter().all(|sidecar| !exists(sidecar)));
    assert!(!exists(&lock), "the retired lock survived");
    assert!(!exists(&old), "an old unreferenced generation survived");
    assert!(exists(&young), "a young generation was reaped");
}

#[test]
fn only_old_unheld_orphan_sidecars_are_removed() {
    for (liveness, aged, removed) in [
        (Liveness::Held, true, false),
        (Liveness::Free, false, false),
        (Liveness::Free, true, true),
        (Liveness::Unknown, true, true),
    ] {
        let harness = RootHarness::new();
        let modified = if aged {
            harness.aged()
        } else {
            harness.young()
        };
        let sidecars = harness.trees().plant_sidecars(
            &Trees::generation_of(&on_host("retired"), &digest('a')),
            modified,
        );

        harness
            .prune(Ports {
                lease_liveness: Arc::new(FixedLiveness(liveness)),
                ..harness.ports()
            })
            .succeeded();

        for sidecar in &sidecars {
            assert_eq!(
                !exists(sidecar),
                removed,
                "{liveness:?}, aged {aged}: {}",
                sidecar.display()
            );
        }
    }
}

#[test]
fn orphan_tree_locks_go_only_once_aged_and_never_for_a_current_tree() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let aged = trees.plant_lock(&on_host("retired-a"), harness.aged());
    let young = trees.plant_lock(&on_host("retired-b"), harness.young());
    let current = on_host("retired-c");
    let tree = trees.plant_tree(&current, &digest('c'), harness.aged());
    trees.plant_claim(&digest('c'), OTHER_ID, harness.now);
    let current_lock = trees.plant_lock(&current, harness.aged());

    harness.prune(harness.ports()).succeeded();

    assert!(!exists(&aged), "an aged orphan lock survived");
    assert!(exists(&young), "a young lock was removed");
    assert!(exists(&current_lock), "a current tree's lock was removed");
    tree.assert_survives();
}

#[test]
fn a_tree_lock_is_kept_while_its_partial_archive_remains() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let pinned = on_host("driver");
    let archive =
        trees.plant_partial_archive(&pinned, &digest('a'), harness.aged());
    let lock = trees.plant_lock(&pinned, harness.aged());

    harness
        .prune(Ports {
            pins: pinning("driver", &digest('a')),
            ..harness.ports()
        })
        .succeeded();

    assert!(exists(&archive), "the resumable archive was reaped");
    assert!(exists(&lock), "the lock went while its archive remained");
    assert!(!exists(&trees.paths().pointer(
        "driver",
        HOST_PLATFORM,
        &digest('a')
    )));
}

#[test]
fn a_lockless_partial_archive_waits_for_its_own_lock() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let retired = on_host("retired");
    let archive =
        trees.plant_partial_archive(&retired, &digest('e'), harness.aged());
    let earlier =
        trees.plant_tree(&on_host("aardvark"), &digest('a'), harness.aged());
    let lock = trees.paths().single_flight_lock(&retired);

    harness.prune(harness.ports()).succeeded();
    earlier.assert_reclaimed();
    assert!(exists(&archive), "a lockless partial archive was reaped");
    assert!(exists(&lock), "the residue did not take its own lock");

    harness.prune(harness.ports()).succeeded();
    assert!(!exists(&archive), "the archive outlived its own lock");
    assert!(exists(&lock), "a young lock was removed");

    harness
        .prune(Ports {
            clock: harness.later(),
            ..harness.ports()
        })
        .succeeded();
    assert!(!exists(&lock), "the aged lock survived");

    let alone = RootHarness::new();
    let archive = alone.trees().plant_partial_archive(
        &retired,
        &digest('e'),
        alone.aged(),
    );
    alone.prune(alone.ports()).succeeded();
    assert!(!exists(&archive), "its own lock did not reap it");
}

#[test]
fn a_retired_partial_archive_goes_with_its_lock() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let retired = on_host("retired");
    let archive =
        trees.plant_partial_archive(&retired, &digest('e'), harness.aged());
    let lock = trees.plant_lock(&retired, harness.aged());

    harness.prune(harness.ports()).succeeded();

    assert!(!exists(&archive), "the archive survived");
    assert!(!exists(&lock), "the lock survived its archive");
}

#[test]
fn a_residue_only_artifact_is_locked_and_reaped() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let retired = on_host("retired");
    let residue =
        trees.plant_temp_generation(&retired, &digest('e'), harness.aged());
    let lock = trees.paths().single_flight_lock(&retired);

    harness.prune(harness.ports()).succeeded();
    assert!(!exists(&residue), "the orphan temp generation survived");
    assert!(exists(&lock), "a young lock was removed");

    harness
        .prune(Ports {
            clock: harness.later(),
            ..harness.ports()
        })
        .succeeded();
    assert!(!exists(&lock), "the aged lock survived");
}

fn prune_trees_directly(
    harness: &RootHarness,
    ports: &CachePorts,
    held_leases: &HeldLeases,
    patience: LockPatience,
) {
    let safe = SafePruneRoot::check(
        &harness.root,
        ports.filesystem.as_ref(),
        ports.ownership.as_ref(),
        &TrustedOwners::none(),
    )
    .expect("safe");
    TreePrune {
        regime: TreePruneRegime::WholeTree(&safe),
        claim_window: CLAIM_WINDOW,
        ports: TreePrunePorts {
            held_leases,
            filesystem: ports.filesystem.as_ref(),
            ownership: ports.ownership.as_ref(),
            clock: ports.clock.as_ref(),
            lease_liveness: ports.lease_liveness.as_ref(),
            pins: ports.pins.as_ref(),
            locks: TreeLocks::over(ports.locks.as_ref()),
            steps: ports.steps.as_ref(),
            removals: ports.removals.as_ref(),
        },
    }
    .run(patience);
}

#[test]
fn a_lock_only_artifact_reaches_the_orphan_lock_step() {
    for patience in [LockPatience::Wait, LockPatience::Try] {
        let harness = RootHarness::new();
        let lock = harness
            .trees()
            .plant_lock(&on_host("retired"), harness.aged());

        prune_trees_directly(
            &harness,
            &harness.ports().build(),
            &HeldLeases::none(),
            patience,
        );

        assert!(!exists(&lock), "{patience:?}");
    }
}

fn is_running_as_root() -> bool {
    ProcessOwnership::default().effective_uid() == 0
}

#[test]
fn a_symlinked_trees_or_tree_claims_directory_is_not_pruned_through() {
    let symlinked_trees = RootHarness::new();
    let elsewhere = symlinked_trees.anchor.path().join("elsewhere");
    let outside = Trees::under(&elsewhere_root(&elsewhere));
    let outside_tree = outside.plant_tree(
        &on_host("retired"),
        &digest('a'),
        symlinked_trees.aged(),
    );
    std::os::unix::fs::symlink(outside.root(), symlinked_trees.join("trees"))
        .expect("symlinked trees");
    let entry = symlinked_trees.entry(OLD, symlinked_trees.aged());

    let verb = symlinked_trees.prune(symlinked_trees.ports());

    verb.succeeded();
    outside_tree.assert_survives();
    assert!(
        !symlinked_trees.has(&entry),
        "the positive control survived"
    );
    assert!(
        verb.err.contains(&format!(
            "not pruning through {}",
            symlinked_trees.join("trees").display()
        )),
        "{}",
        verb.err
    );

    let symlinked_claims = RootHarness::new();
    let trees = symlinked_claims.trees();
    let tree = trees.plant_tree(
        &on_host("retired"),
        &digest('a'),
        symlinked_claims.aged(),
    );
    let outside_claims = symlinked_claims.anchor.path().join("claims");
    std::fs::create_dir(&outside_claims).expect("outside claims");
    let stale_claim =
        outside_claims.join(format!("{}.{OTHER_ID}", digest('b')));
    std::fs::write(&stale_claim, b"").expect("stale claim");
    set_mtime(
        &stale_claim,
        symlinked_claims.now - 2 * CLAIM_WINDOW.as_secs(),
    );
    std::os::unix::fs::symlink(&outside_claims, trees.claims())
        .expect("symlinked claims");
    let entry = symlinked_claims.entry(OLD, symlinked_claims.aged());

    let verb = symlinked_claims.prune(symlinked_claims.ports());

    verb.succeeded();
    tree.assert_survives();
    assert!(
        stale_claim.exists(),
        "a claim was swept through the symlink"
    );
    assert!(
        !symlinked_claims.has(&entry),
        "the positive control survived"
    );
    assert!(
        verb.err.contains(&format!(
            "not pruning through {}",
            trees.claims().display()
        )),
        "{}",
        verb.err
    );
}

fn elsewhere_root(elsewhere: &Path) -> PathBuf {
    std::fs::create_dir(elsewhere).expect("elsewhere");
    elsewhere.to_path_buf()
}

#[test]
fn an_unreadable_tree_claims_directory_reclaims_no_claimed_tree() {
    if is_running_as_root() {
        return;
    }
    let harness = RootHarness::new();
    let trees = harness.trees();
    let claimed =
        trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());
    trees.plant_claim(&digest('a'), OTHER_ID, harness.now);
    let residue = trees.plant_temp_generation(
        &on_host("retired"),
        &digest('b'),
        harness.aged(),
    );
    set_mode(&trees.claims(), 0o000);

    let verb = harness.prune(harness.ports());

    set_mode(&trees.claims(), 0o700);
    verb.succeeded();
    claimed.assert_survives();
    assert!(!exists(&residue), "the orphan residue was not reaped");
    assert!(
        verb.err.contains(&format!(
            "claims unreadable (permission denied) in {}",
            trees.claims().display()
        )),
        "{}",
        verb.err
    );
}

struct PrivatePrimaryGroup;

impl Ownership for PrivatePrimaryGroup {
    fn effective_uid(&self) -> u32 {
        ProcessOwnership::default().effective_uid()
    }

    fn effective_gid(&self) -> u32 {
        ProcessOwnership::default().effective_gid()
    }

    fn primary_group(&self) -> PrimaryGroup {
        PrimaryGroup::Private
    }
}

#[test]
fn a_user_private_group_root_prunes_its_group_writable_trees() {
    let gid = ProcessOwnership::default().effective_gid();
    for (group, reclaimed) in [(gid, true), (gid + 1, false)] {
        let harness = RootHarness::new();
        let trees = harness.trees();
        let tree =
            trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());
        let filesystem = ReportingFilesystem::over_system().reporting(
            trees.root(),
            EntryMetadata {
                mode: 0o040_770,
                group,
                ..SystemFilesystem.entry(trees.root()).expect("metadata")
            },
        );

        harness
            .prune(Ports {
                filesystem: Arc::new(filesystem),
                ownership: Arc::new(PrivatePrimaryGroup),
                ..harness.ports()
            })
            .succeeded();

        assert_eq!(!exists(&tree.generation), reclaimed, "group {group}");
    }
}

#[test]
fn a_tree_claimed_after_the_index_was_read_is_kept() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let tree =
        trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());
    trees.plant_claim(&digest('b'), OTHER_ID, harness.now);
    let recorder = harness.recorder();
    let claims = trees.claims();
    let late_claim = claims.join(format!("{}.{OTHER_ID}", digest('a')));
    let filesystem =
        Arc::new(ScriptedFilesystem::over(recorder.clone()).before(
            FilesystemCall::List(in_root("")).kind(),
            &claims,
            2,
            move || std::fs::write(&late_claim, b"").expect("late claim"),
        ));

    harness
        .prune(Ports {
            filesystem: Arc::clone(&filesystem)
                as Arc<dyn CacheFilesystem + Send + Sync>,
            lease_liveness: Arc::new(FixedLiveness(Liveness::Unknown)),
            ..harness.ports()
        })
        .succeeded();

    tree.assert_survives();
    assert_eq!(filesystem.fired().len(), 1, "the script never ran");
    let timeline = recorder.timeline();
    let ran = timeline
        .iter()
        .position(|event| matches!(event, TimelineEvent::Scripted(_)))
        .expect("scripted");
    assert_eq!(
        timeline.get(ran + 1),
        Some(&TimelineEvent::Call(FilesystemCall::List(
            recorder.recorded(&claims)
        )))
    );
}

#[test]
fn a_tree_claim_refreshed_before_unlink_is_kept() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let claim = trees.plant_claim(
        &digest('a'),
        OTHER_ID,
        harness.now - CLAIM_WINDOW.as_secs(),
    );
    let recorder = harness.recorder();
    let refreshed = claim.clone();
    let now = harness.now;
    let filesystem =
        Arc::new(ScriptedFilesystem::over(recorder.clone()).before(
            FilesystemCall::Entry(in_root("")).kind(),
            &claim,
            2,
            move || set_mtime(&refreshed, now),
        ));

    harness
        .prune(Ports {
            filesystem: Arc::clone(&filesystem)
                as Arc<dyn CacheFilesystem + Send + Sync>,
            ..harness.ports()
        })
        .succeeded();

    assert!(claim.exists());
    assert_eq!(filesystem.fired().len(), 1, "the script never ran");
    let timeline = recorder.timeline();
    let ran = timeline
        .iter()
        .position(|event| matches!(event, TimelineEvent::Scripted(_)))
        .expect("scripted");
    assert_eq!(
        timeline.get(ran + 1),
        Some(&TimelineEvent::Call(FilesystemCall::Entry(
            recorder.recorded(&claim)
        )))
    );
}

#[test]
fn a_tree_leased_during_removal_is_kept() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let tree =
        trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());
    let lease = trees
        .sidecars_of(&Trees::generation_of(&on_host("retired"), &digest('a')))
        [2]
    .clone();
    let held = Arc::new(Mutex::new(None));
    let leased = Arc::clone(&held);
    let filesystem = ScriptedFilesystem::over(harness.recorder()).before(
        FilesystemCall::RemoveFile(in_root("")).kind(),
        &tree.pointer,
        1,
        move || {
            *leased.lock().expect("leased") =
                Some(lease::hold_shared_lease(&lease).expect("lease"));
        },
    );

    harness
        .prune(Ports {
            filesystem: Arc::new(filesystem),
            ..harness.ports()
        })
        .succeeded();

    assert!(
        held.lock().expect("held").is_some(),
        "the lease was never held"
    );
    assert!(!exists(&tree.pointer), "the pointer survived");
    assert!(exists(&tree.generation), "a leased generation was removed");
}

#[test]
fn a_tree_removal_failure_is_reported() {
    let harness = RootHarness::new();
    let tree = harness.trees().plant_tree(
        &on_host("retired"),
        &digest('a'),
        harness.aged(),
    );
    let filesystem = ScriptedFilesystem::over(harness.recorder()).failing(
        FilesystemCall::RemoveTree(in_root("")).kind(),
        &tree.generation,
        1,
        std::io::ErrorKind::PermissionDenied,
    );

    let verb = harness.prune(Ports {
        filesystem: Arc::new(filesystem),
        ..harness.ports()
    });

    verb.failed();
    assert!(exists(&tree.generation));
    assert!(
        verb.err.contains(&format!(
            "could not remove {}: permission denied",
            tree.generation.display()
        )),
        "{}",
        verb.err
    );
}

#[test]
fn a_tree_whose_lease_this_process_holds_is_kept_unprobed() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let tree =
        trees.plant_tree(&on_host("retired"), &digest('a'), harness.aged());
    let liveness = Arc::new(ProbedLeases::default());

    prune_trees_directly(
        &harness,
        &Ports {
            lease_liveness: Arc::clone(&liveness)
                as Arc<dyn LivenessProbe + Send + Sync>,
            ..harness.ports()
        }
        .build(),
        &HeldLeases::holding([tree.generation.clone()]),
        LockPatience::Wait,
    );

    tree.assert_survives();
    assert!(
        liveness
            .of_generations(std::slice::from_ref(&tree.generation))
            .is_empty(),
        "a lease this process holds was probed"
    );
}

/// An unclaimed tree a prune must reclaim, showing it ran far enough to have
/// removed whatever a test expects it to keep.
fn reclaimable_control(harness: &RootHarness) -> common::trees::PlantedTree {
    harness.trees().plant_tree(
        &on_host("control"),
        &digest('c'),
        harness.aged(),
    )
}

#[test]
fn a_pointer_naming_an_absolute_path_deletes_nothing_outside_the_root() {
    let harness = RootHarness::new();
    let control = reclaimable_control(&harness);
    let victim = harness.anchor.path().join("victim");
    std::fs::create_dir(&victim).expect("victim");
    harness.trees().plant_pointer(
        &on_host("retired"),
        &digest('a'),
        victim.to_str().expect("utf-8"),
    );

    harness.prune(harness.ports()).succeeded();

    assert!(victim.exists(), "a directory outside the root was removed");
    control.assert_reclaimed();
}

#[test]
fn a_pointer_escaping_with_dot_dot_deletes_nothing() {
    let harness = RootHarness::new();
    let control = reclaimable_control(&harness);
    let victim = harness.join("victim");
    std::fs::create_dir(&victim).expect("victim");
    harness.trees().plant_pointer(
        &on_host("retired"),
        &digest('a'),
        "../victim",
    );

    harness.prune(harness.ports()).succeeded();

    assert!(victim.exists(), "a directory above trees/ was removed");
    control.assert_reclaimed();
}

#[test]
fn a_pointer_naming_another_artifacts_generation_is_kept() {
    let harness = RootHarness::new();
    let control = reclaimable_control(&harness);
    let trees = harness.trees();
    let generation = Trees::generation_of(&on_host("other"), &digest('e'));
    let victim = trees.plant_generation(&generation, harness.aged());
    let pointer =
        trees.plant_pointer(&on_host("retired"), &digest('e'), &generation);

    harness.prune(harness.ports()).succeeded();

    assert!(victim.exists(), "another artifact's generation was removed");
    assert!(pointer.exists(), "the mismatched pointer was removed");
    control.assert_reclaimed();
}

#[test]
fn a_malformed_pointer_is_kept() {
    let harness = RootHarness::new();
    let control = reclaimable_control(&harness);
    let trees = harness.trees();
    let named = format!("retired-{HOST_PLATFORM}-{}-1-short", digest('e'));
    let victim = trees.plant_generation(&named, harness.aged());
    let pointer =
        trees.plant_pointer(&on_host("retired"), &digest('e'), &named);

    harness.prune(harness.ports()).succeeded();

    assert!(victim.exists(), "a malformed pointer's target was removed");
    assert!(pointer.exists(), "the malformed pointer was removed");
    control.assert_reclaimed();
}

#[test]
fn a_symlinked_fifo_or_oversized_pointer_is_malformed_and_kept() {
    let harness = Arc::new(RootHarness::new());
    let control = reclaimable_control(&harness);
    let trees = harness.trees();
    let retired = on_host("retired");

    let target = harness.anchor.path().join("pointer-target");
    std::fs::write(&target, Trees::generation_of(&retired, &digest('1')))
        .expect("target");
    let linked =
        trees
            .paths()
            .pointer(retired.name(), retired.platform(), &digest('1'));
    std::os::unix::fs::symlink(&target, &linked).expect("symlink");

    let fifo =
        trees
            .paths()
            .pointer(retired.name(), retired.platform(), &digest('2'));
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo");
    assert!(made.success(), "mkfifo failed");

    let oversized = trees.plant_pointer(
        &retired,
        &digest('3'),
        format!(
            "{}{}",
            Trees::generation_of(&retired, &digest('3')),
            " ".repeat(300)
        ),
    );

    let pruning = Arc::clone(&harness);
    common::within(BOUND, "the prune", move || {
        pruning.prune(pruning.ports()).outcome.is_ok()
    });

    for pointer in [&linked, &fifo, &oversized] {
        assert!(exists(pointer), "{} was removed", pointer.display());
    }
    assert!(
        target.exists(),
        "the symlinked pointer's target was removed"
    );
    control.assert_reclaimed();
}

#[test]
fn another_layouts_tree_is_kept_and_this_layouts_twin_is_removed() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let retired = on_host("retired");
    let twin = trees.plant_tree(&retired, &digest('a'), harness.aged());
    let generation = Trees::generation_at_layout(&retired, &digest('e'), 2);
    let other_layout = trees.plant_generation(&generation, harness.aged());
    trees.plant_sidecars(&generation, harness.aged());
    let pointer = trees.plant_pointer(&retired, &digest('e'), &generation);
    set_mtime(&pointer, harness.aged());

    harness.prune(harness.ports()).succeeded();

    assert!(other_layout.exists(), "another layout's tree was removed");
    assert!(pointer.exists(), "another layout's pointer was removed");
    twin.assert_reclaimed();
}

#[test]
fn the_verb_creates_no_lock_for_an_artifact_it_never_materialised() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    trees.plant_claim(&digest('a'), OTHER_ID, harness.now);
    let aged_sidecars = trees.plant_sidecars(
        &Trees::generation_of(&on_host("retired-a"), &digest('a')),
        harness.aged(),
    );
    let young_sidecars = trees.plant_sidecars(
        &Trees::generation_of(&on_host("retired-b"), &digest('b')),
        harness.young(),
    );

    harness.prune(harness.ports()).succeeded();

    assert!(
        aged_sidecars.iter().all(|sidecar| !exists(sidecar)),
        "the prune never visited the sidecar-only artifact"
    );
    assert!(young_sidecars.iter().all(|sidecar| exists(sidecar)));
    let locks: Vec<_> = std::fs::read_dir(trees.root())
        .expect("trees")
        .flatten()
        .map(|entry| entry.file_name())
        .filter(|name| name.to_string_lossy().ends_with(".lock"))
        .collect();
    assert!(locks.is_empty(), "{locks:?}");
}

fn prune_held_tree(
    harness: &RootHarness,
    artifact: &Artifact,
) -> TreePruneReport {
    let ports = harness.ports().build();
    let safe = SafePruneRoot::check(
        &harness.root,
        ports.filesystem.as_ref(),
        ports.ownership.as_ref(),
        &TrustedOwners::none(),
    )
    .expect("safe");
    let locks = TreeLocks::over(ports.locks.as_ref());
    let held = locks
        .take_tree(&TreePaths::under(&harness.root), artifact)
        .expect("held");
    TreePrune {
        regime: TreePruneRegime::WholeTree(&safe),
        claim_window: CLAIM_WINDOW,
        ports: TreePrunePorts {
            held_leases: &HeldLeases::none(),
            filesystem: ports.filesystem.as_ref(),
            ownership: ports.ownership.as_ref(),
            clock: ports.clock.as_ref(),
            lease_liveness: ports.lease_liveness.as_ref(),
            pins: ports.pins.as_ref(),
            locks,
            steps: ports.steps.as_ref(),
            removals: ports.removals.as_ref(),
        },
    }
    .prune_held(&held)
}

#[test]
fn a_symlinked_trees_directory_is_not_pruned_by_prune_held() {
    let retired = on_host("retired");
    let harness = RootHarness::new();
    let outside =
        Trees::under(&elsewhere_root(&harness.anchor.path().join("elsewhere")));
    let tree = outside.plant_tree(&retired, &digest('a'), harness.aged());
    std::os::unix::fs::symlink(outside.root(), harness.join("trees"))
        .expect("symlinked trees");

    let report = prune_held_tree(&harness, &retired);

    tree.assert_survives();
    assert_eq!(
        report.untrusted_directories,
        [UntrustedDirectory {
            path: harness.join("trees")
        }]
    );

    let real = RootHarness::new();
    let tree = real.trees().plant_tree(&retired, &digest('a'), real.aged());
    let bystander =
        real.trees()
            .plant_tree(&on_host("other"), &digest('b'), real.aged());
    let orphaned = Trees::generation_of(&retired, &digest('c'));
    let sidecars = real.trees().plant_sidecars(&orphaned, real.aged());

    prune_held_tree(&real, &retired);

    tree.assert_reclaimed();
    bystander.assert_survives();
    assert!(sidecars.iter().all(|sidecar| !exists(sidecar)));
}

/// Records every entry a prune is about to remove, trying each tree lock
/// among them from a separate open file description, as another process
/// would.
#[derive(Default)]
struct ProbingRemovals(Mutex<Vec<(PathBuf, Option<bool>)>>);

impl ProbingRemovals {
    fn seen(&self) -> Vec<(PathBuf, Option<bool>)> {
        self.0.lock().expect("seen").clone()
    }
}

impl RemovalObserver for ProbingRemovals {
    fn removing(&self, entry: &Path) {
        let held = entry
            .extension()
            .is_some_and(|extension| extension == "lock")
            .then(|| matches!(FlockLocks.probe(entry), Ok(LockProbe::Held)));
        self.0
            .lock()
            .expect("seen")
            .push((entry.to_path_buf(), held));
    }
}

#[test]
fn no_production_path_unlinks_a_tree_lock_it_does_not_hold() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let lock_only = trees.plant_lock(&on_host("retired-a"), harness.aged());
    let retired = on_host("retired-b");
    trees.plant_partial_archive(&retired, &digest('b'), harness.aged());
    let with_archive = trees.plant_lock(&retired, harness.aged());
    let tree =
        trees.plant_tree(&on_host("retired-c"), &digest('c'), harness.aged());
    let claim = trees.plant_claim(
        &digest('d'),
        OTHER_ID,
        harness.now - 2 * CLAIM_WINDOW.as_secs(),
    );
    let observer = Arc::new(ProbingRemovals::default());

    harness
        .prune(Ports {
            removals: Arc::clone(&observer)
                as Arc<dyn RemovalObserver + Send + Sync>,
            ..harness.ports()
        })
        .succeeded();

    let seen = observer.seen();
    for expected in [
        &lock_only,
        &with_archive,
        &tree.pointer,
        &tree.generation,
        &claim,
    ] {
        assert!(
            seen.iter().any(|(entry, _)| entry == expected),
            "{} went unobserved: {seen:#?}",
            expected.display()
        );
    }
    let locks: Vec<_> = seen
        .iter()
        .filter_map(|(entry, held)| held.map(|held| (entry, held)))
        .collect();
    assert_eq!(locks.len(), 2, "{seen:#?}");
    assert!(locks.iter().all(|(_, held)| *held), "{locks:#?}");
}

#[test]
fn no_prune_path_probes_or_takes_a_lock_it_holds() {
    for trigger in EVERY_TRIGGER {
        no_lock_held_is_probed_or_taken(trigger);
    }
}

fn no_lock_held_is_probed_or_taken(trigger: Trigger) {
    let harness = RootHarness::new();
    let trees = harness.trees();
    trees.plant_tree(&on_host("aardvark"), &digest('a'), harness.aged());
    trees.plant_temp_generation(
        &on_host("aardvark"),
        &digest('f'),
        harness.aged(),
    );
    let retired = on_host("retired-b");
    trees.plant_partial_archive(&retired, &digest('b'), harness.aged());
    trees.plant_lock(&retired, harness.aged());
    trees.plant_temp_generation(
        &on_host("retired-c"),
        &digest('c'),
        harness.aged(),
    );
    trees.plant_lock(&on_host("retired-d"), harness.aged());
    trees.plant_sidecars(
        &Trees::generation_of(&on_host("retired-e"), &digest('e')),
        harness.aged(),
    );
    harness.entry(OLD, harness.aged());
    let locks = RecordingLocks::default();

    let fired = harness.fire(
        trigger,
        Ports {
            locks: Arc::new(locks.clone()),
            ..harness.ports()
        },
    );

    if let Fired::Verb(verb) = &fired {
        verb.succeeded();
    }
    if !fired.ran() {
        return;
    }
    let calls = locks.calls();
    assert!(
        calls.iter().any(|(call, _)| *call == LockCall::Probe),
        "{trigger:?}: the reap probed no other artifact's lock: {calls:#?}"
    );
    assert!(calls.len() >= 5, "{trigger:?}: {calls:#?}");
    assert_eq!(
        locks.calls_on_held_locks(),
        Vec::new(),
        "{trigger:?}: {calls:#?}"
    );
}

#[test]
fn each_artifact_state_and_patience_uses_the_tables_lock_call() {
    enum State {
        Work,
        LockOnly,
        SidecarsOnly,
    }
    for (state, patience, expected) in [
        (State::Work, LockPatience::Wait, vec![LockCall::Take]),
        (State::Work, LockPatience::Try, vec![LockCall::TryTake]),
        (
            State::LockOnly,
            LockPatience::Wait,
            vec![LockCall::WaitExisting],
        ),
        (State::LockOnly, LockPatience::Try, vec![LockCall::Probe]),
        (State::SidecarsOnly, LockPatience::Wait, vec![]),
        (State::SidecarsOnly, LockPatience::Try, vec![]),
    ] {
        let harness = RootHarness::new();
        let trees = harness.trees();
        let retired = on_host("retired");
        match state {
            State::Work => {
                trees.plant_tree(&retired, &digest('a'), harness.aged());
            }
            State::LockOnly => {
                trees.plant_lock(&retired, harness.aged());
            }
            State::SidecarsOnly => {
                trees.plant_sidecars(
                    &Trees::generation_of(&retired, &digest('a')),
                    harness.aged(),
                );
            }
        }
        let locks = RecordingLocks::default();

        prune_trees_directly(
            &harness,
            &Ports {
                locks: Arc::new(locks.clone()),
                ..harness.ports()
            }
            .build(),
            &HeldLeases::none(),
            patience,
        );

        assert_eq!(
            locks.calls_on(&trees.paths().single_flight_lock(&retired)),
            expected,
            "{patience:?}"
        );
    }
}

/// Everything a relocated root's prune would remove, seeded in the plugin
/// `bin` where none of it may go.
struct RelocatedPrunables {
    files: Vec<PathBuf>,
    trees: Vec<PlantedTree>,
}

impl RelocatedPrunables {
    fn plant(harness: &RootHarness, bin: &Path) -> Self {
        let mut files: Vec<PathBuf> = [
            format!("vcs-{OLD}-{DIGEST}"),
            format!("vcs-{OLD}-{DIGEST}.minisig"),
            format!("accelerator-launcher-{OLD}-{HOST_PLATFORM}"),
            format!("accelerator-launcher-{OLD}-{HOST_PLATFORM}.minisig"),
            format!("accelerator-verify-{HOST_PLATFORM}-{DIGEST}"),
            format!("accelerator-verify-{HOST_PLATFORM}-{DIGEST}.staging.4343"),
            format!(".tmp-vcs-{DIGEST}-1-2"),
            ".tmp-launcher-4343".to_owned(),
            ".accelerator-probe-77".to_owned(),
        ]
        .iter()
        .map(|name| {
            let path = bin.join(name);
            std::fs::write(&path, b"x").expect("seeded");
            set_mtime(&path, harness.aged());
            path
        })
        .collect();
        let lock = bin.join(format!(".accelerator-lock-{HOST_PLATFORM}"));
        std::fs::create_dir(&lock).expect("bootstrap lock");
        std::fs::write(lock.join("pid"), "4343\n").expect("pid");
        set_mtime(&lock, harness.aged());
        files.push(lock.join("pid"));
        let trees = Trees::under(bin);
        files.push(trees.plant_claim(
            &digest('a'),
            OTHER_ID,
            harness.now - 2 * CLAIM_WINDOW.as_secs(),
        ));
        files.push(trees.plant_lock(&on_host("retired-lock"), harness.aged()));
        files.extend(trees.plant_sidecars(
            &Trees::generation_of(&on_host("retired-sidecar"), &digest('e')),
            harness.aged(),
        ));
        Self {
            files,
            trees: vec![
                trees.plant_tree(
                    &on_host("driver"),
                    &digest('a'),
                    harness.aged(),
                ),
                trees.plant_tree(
                    &on_host("retired"),
                    &digest('b'),
                    harness.aged(),
                ),
                trees.plant_tree(
                    &Artifact::new("driver", another_platform()),
                    &digest('c'),
                    harness.aged(),
                ),
            ],
        }
    }

    #[track_caller]
    fn assert_survive(&self, spelling: &str) {
        for path in &self.files {
            assert!(exists(path), "{spelling}: {} was removed", path.display());
        }
        for tree in &self.trees {
            tree.assert_survives();
        }
    }
}

/// The cache directory override that names `bin` as `spelling` does, or none.
fn spelled(
    spelling: &str,
    harness: &RootHarness,
    bin: &Path,
) -> Option<PathBuf> {
    match spelling {
        "unset" => None,
        "named" => Some(bin.to_path_buf()),
        "symlinked" => {
            let link = harness.anchor.path().join("bin-link");
            std::os::unix::fs::symlink(bin, &link).expect("symlink");
            Some(link)
        }
        _ => Some(PathBuf::from(format!("{}/", bin.display()))),
    }
}

#[test]
fn the_plugin_bin_root_is_never_claimed_or_pruned() {
    for spelling in ["unset", "named", "symlinked", "slashed"] {
        let harness = RootHarness::new();
        let bin = harness.plugin.join("bin");
        let prunables = RelocatedPrunables::plant(&harness, &bin);
        let superseded = Trees::under(&bin).plant_tree(
            &on_host("driver"),
            &digest('d'),
            harness.aged(),
        );

        let start = start_up(
            &CacheRootConfig {
                cache_dir_override: spelled(spelling, &harness, &bin),
                plugin_root: Some(harness.plugin.clone()),
            },
            TrustedOwners::none(),
            Ports {
                pins: pinning("driver", &digest('a')),
                ..harness.ports()
            }
            .build(),
        );
        if let Some(release) = harness.release_with_trees(&[MATERIALISED]) {
            let executor = SnapshotAtExec::watching(bin.clone());
            let failure =
                cold_dispatch_through(&start, &release, &[], &executor);
            assert!(
                executor.program().is_some(),
                "{spelling}: the dispatch never reached exec: {failure}"
            );
            materialise_through(&start, &release, MATERIALISED)
                .expect("materialised");
            superseded.assert_survives();
        }
        let verb = prune_through(&start, None);

        verb.succeeded();
        assert!(!bin.join(VERSION_CLAIMS_DIR).exists(), "{spelling}");
        prunables.assert_survive(spelling);
        superseded.assert_reclaimed();
    }
}

struct SnapshotAtExec {
    watched: Vec<PathBuf>,
    present_at_exec: Mutex<Option<bool>>,
    program: Mutex<Option<PathBuf>>,
}

impl SnapshotAtExec {
    fn watching(watched: PathBuf) -> Self {
        Self::watching_any(vec![watched])
    }

    const fn watching_any(watched: Vec<PathBuf>) -> Self {
        Self {
            watched,
            present_at_exec: Mutex::new(None),
            program: Mutex::new(None),
        }
    }

    fn present_at_exec(&self) -> Option<bool> {
        *self.present_at_exec.lock().expect("snapshot")
    }

    fn program(&self) -> Option<PathBuf> {
        self.program.lock().expect("program").clone()
    }
}

impl ExecBinary for SnapshotAtExec {
    fn exec(&self, program: &Path, _args: &[OsString]) -> ResolutionError {
        *self.present_at_exec.lock().expect("snapshot") = Some(
            self.watched
                .iter()
                .any(|path| path.symlink_metadata().is_ok()),
        );
        *self.program.lock().expect("program") = Some(program.to_path_buf());
        ResolutionError::Exec {
            program: program.to_path_buf(),
            source: std::io::Error::other("executed"),
        }
    }
}

/// A dispatch whose sub-binary is not cached, holding `consumed` as `run`
/// holds the trees it acquired.
fn cold_dispatch_through(
    start: &StartUp,
    release: &SignedRelease,
    consumed: &[AcquiredTree],
    executor: &impl ExecBinary,
) -> ResolutionError {
    let resolver = start
        .dispatch_resolver(
            DispatchConfig {
                base_url: release.server.base_url(),
                keys: release.keys(),
                fetcher: Fetcher::with_backoff(Duration::from_millis(1))
                    .expect("fetcher"),
            },
            consumed,
        )
        .expect("dispatch resolver");
    run_external(&resolver, executor, &visualiser())
}

fn visualiser() -> ExternalCommand {
    ExternalCommand {
        name: OsString::from(BINARY),
        args: vec![],
    }
}

impl RootHarness {
    fn release(&self) -> Option<SignedRelease> {
        self.release_with_trees(&[])
    }

    fn release_with_trees(&self, artifacts: &[&str]) -> Option<SignedRelease> {
        SignedRelease::serve_or_skip_with_trees(self.anchor.path(), artifacts)
    }

    fn cold_dispatch(
        &self,
        release: &SignedRelease,
        ports: Ports,
        executor: &impl ExecBinary,
    ) -> ResolutionError {
        let start = self.start_over(&self.root, TrustedOwners::none(), ports);
        cold_dispatch_through(&start, release, &[], executor)
    }

    fn dispatched(&self, release: &SignedRelease, ports: Ports) {
        let executor = SnapshotAtExec::watching(self.root.clone());
        let failure = self.cold_dispatch(release, ports, &executor);
        assert!(
            executor.program().is_some(),
            "the dispatch never reached exec: {failure}"
        );
    }
}

#[test]
fn a_cold_dispatch_prunes_before_exec() {
    let harness = RootHarness::new();
    let Some(release) = harness.release() else {
        return;
    };
    let unclaimed = harness.entry(OLD, harness.aged());
    let executor = SnapshotAtExec::watching_any(vec![
        harness.join(&unclaimed),
        harness.join(&format!("{unclaimed}.minisig")),
    ]);

    harness.cold_dispatch(&release, harness.ports(), &executor);

    assert_eq!(executor.present_at_exec(), Some(false));
    let program = executor.program().expect("exec reached");
    assert_eq!(
        std::fs::read(program).expect("stored"),
        release.fixture_bytes
    );
}

#[test]
fn a_held_root_lock_does_not_delay_a_cold_dispatch() {
    let harness = Arc::new(RootHarness::new());
    let Some(release) = harness.release() else {
        return;
    };
    let unclaimed = harness.entry(OLD, harness.aged());
    let probe = harness.file(".accelerator-probe-77", harness.aged());
    let held = FlockLocks.take(&harness.join(ROOT_LOCK)).expect("held");
    let (done, finished) = mpsc::channel();
    let dispatcher = {
        let harness = Arc::clone(&harness);
        std::thread::spawn(move || {
            harness.dispatched(&release, harness.ports());
            done.send(()).expect("announced");
        })
    };

    let reached = finished.recv_timeout(BOUND);

    drop(held);
    dispatcher.join().expect("dispatcher");
    assert!(reached.is_ok(), "the dispatch waited for the root lock");
    assert!(harness.has(&unclaimed));
    assert!(
        probe.exists(),
        "the residue sweep ran without the root lock"
    );
}

#[test]
fn a_cold_dispatch_skips_a_held_tree_lock_and_prunes_the_rest() {
    let harness = RootHarness::new();
    let Some(release) = harness.release() else {
        return;
    };
    let trees = harness.trees();
    let held_artifact = on_host("retired-x");
    let kept = trees.plant_tree(&held_artifact, &digest('a'), harness.aged());
    let removed =
        trees.plant_tree(&on_host("retired-y"), &digest('b'), harness.aged());
    let held = FlockLocks
        .take(&trees.paths().single_flight_lock(&held_artifact))
        .expect("held");

    harness.dispatched(&release, harness.ports());

    drop(held);
    kept.assert_survives();
    removed.assert_reclaimed();
}

struct HeldByTest;

impl HeldLease for HeldByTest {}

/// Answers `Free` for every lease and remembers each it was asked about, so
/// a test can see whether a lease this process holds was probed.
#[derive(Default)]
struct ProbedLeases(Mutex<Vec<PathBuf>>);

impl ProbedLeases {
    fn of_generations(&self, generations: &[PathBuf]) -> Vec<PathBuf> {
        self.0
            .lock()
            .expect("probed")
            .iter()
            .filter(|lease| {
                generations
                    .iter()
                    .any(|generation| lease_beside(generation) == **lease)
            })
            .cloned()
            .collect()
    }
}

impl LivenessProbe for ProbedLeases {
    fn probe(&self, lease: &Path) -> Liveness {
        self.0.lock().expect("probed").push(lease.to_path_buf());
        Liveness::Free
    }
}

fn acquired(tree: &common::trees::PlantedTree) -> AcquiredTree {
    AcquiredTree {
        tree: SealedTree {
            artifact: "retired".to_owned(),
            path: tree.generation.clone(),
            lease_path: lease_beside(&tree.generation),
            digest: digest('a'),
        },
        lease: Box::new(HeldByTest),
    }
}

#[test]
fn a_cold_store_never_probes_its_own_held_leases() {
    for consumed in [true, false] {
        let harness = RootHarness::new();
        let Some(release) = harness.release() else {
            return;
        };
        let tree = harness.trees().plant_tree(
            &on_host("retired"),
            &digest('a'),
            harness.aged(),
        );
        let liveness = Arc::new(ProbedLeases::default());
        let start = harness.start_over(
            &harness.root,
            TrustedOwners::none(),
            Ports {
                lease_liveness: Arc::clone(&liveness)
                    as Arc<dyn LivenessProbe + Send + Sync>,
                ..harness.ports()
            },
        );
        let held: Vec<AcquiredTree> = if consumed {
            vec![acquired(&tree)]
        } else {
            Vec::new()
        };

        cold_dispatch_through(
            &start,
            &release,
            &held,
            &SnapshotAtExec::watching(harness.root.clone()),
        );

        let probed =
            liveness.of_generations(std::slice::from_ref(&tree.generation));
        if consumed {
            assert_eq!(probed, Vec::<PathBuf>::new());
            tree.assert_survives();
        } else {
            assert!(!probed.is_empty(), "the control never fired");
        }
    }
}

struct SettableClock(std::sync::atomic::AtomicU64);

impl SettableClock {
    fn set(&self, seconds: u64) {
        self.0.store(seconds, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Clock for SettableClock {
    fn now_seconds(&self) -> u64 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn sleep_poll_interval(&self) {}
}

fn tree_config<'a>(
    release: &SignedRelease,
    keys: &'a TrustedKeys,
    fetcher: &'a Fetcher,
) -> TreeConfig<'a> {
    TreeConfig {
        base_url: release.server.base_url(),
        keys,
        fetcher,
        expected_digests: ExpectedDigests::Fixed(release.tree_digests()),
        waiter_bound: Duration::from_secs(5),
        steps: &NoSteps,
    }
}

#[test]
fn acquired_trees_are_never_probed_by_the_cold_store_prune() {
    for consumed in [true, false] {
        let harness = RootHarness::new();
        let artifacts = ["driver", "browser"];
        let Some(release) = harness.release_with_trees(&artifacts) else {
            return;
        };
        let clock = Arc::new(SettableClock(harness.now.into()));
        let liveness = Arc::new(ProbedLeases::default());
        let start = harness.start_over(
            &harness.root,
            TrustedOwners::none(),
            Ports {
                clock: Arc::clone(&clock) as Arc<dyn Clock + Send + Sync>,
                lease_liveness: Arc::clone(&liveness)
                    as Arc<dyn LivenessProbe + Send + Sync>,
                pins: ExpectedDigests::Fixed(BTreeMap::new()),
                ..harness.ports()
            },
        );
        let keys = release.keys();
        let fetcher =
            Fetcher::with_backoff(Duration::from_millis(1)).expect("fetcher");
        let trees = start
            .tree_resolver(tree_config(&release, &keys, &fetcher))
            .expect("tree resolver");
        for artifact in artifacts {
            trees.materialise(artifact).expect("materialised");
        }
        let held = acquire_trees(&trees, &artifacts).expect("acquired");
        let generations: Vec<PathBuf> =
            held.iter().map(|tree| tree.tree.path.clone()).collect();
        assert_eq!(generations.len(), artifacts.len());
        clock.set(harness.now + 2 * CLAIM_WINDOW.as_secs());
        let consumed_trees: &[AcquiredTree] =
            if consumed { &held } else { &[] };

        cold_dispatch_through(
            &start,
            &release,
            consumed_trees,
            &SnapshotAtExec::watching(harness.root.clone()),
        );

        let probed = liveness.of_generations(&generations);
        if consumed {
            assert_eq!(probed, Vec::<PathBuf>::new());
            assert!(generations.iter().all(|generation| generation.exists()));
        } else {
            assert!(!probed.is_empty(), "the control never fired");
        }
    }
}

#[test]
fn a_cold_prune_behind_a_paused_prune_returns_without_touching_anything() {
    let harness = Arc::new(RootHarness::new());
    let Some(release) = harness.release() else {
        return;
    };
    let unclaimed = harness.entry(OLD, harness.aged());
    let (announce, announced) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    let pause = Arc::new(Pause {
        at: PruneStep::RootLockHeld,
        reached: Mutex::new(Some(announce)),
        resume: Mutex::new(resumed),
    });
    let first = {
        let harness = Arc::clone(&harness);
        std::thread::spawn(move || {
            harness
                .prune(Ports {
                    steps: pause,
                    ..harness.ports()
                })
                .outcome
                .is_ok()
        })
    };
    announced
        .recv_timeout(BOUND)
        .expect("the first prune paused");

    harness.dispatched(&release, harness.ports());

    assert!(harness.has(&unclaimed), "the cold prune did not skip");
    let (done, finished) = mpsc::channel();
    let second = {
        let harness = Arc::clone(&harness);
        std::thread::spawn(move || {
            let succeeded = harness.prune(harness.ports()).outcome.is_ok();
            done.send(()).expect("announced");
            succeeded
        })
    };
    assert!(
        finished.recv_timeout(Duration::from_millis(300)).is_err(),
        "the verb did not wait behind the paused prune"
    );
    resume.send(()).expect("resumed");
    assert!(first.join().expect("first prune"));
    finished.recv_timeout(BOUND).expect("the verb finished");
    assert!(second.join().expect("second prune"));
    assert!(!harness.has(&unclaimed));
}

#[test]
fn an_enolck_root_lock_skips_single_file_work_on_the_cold_path() {
    let harness = RootHarness::new();
    let Some(release) = harness.release() else {
        return;
    };
    let unclaimed = harness.entry(OLD, harness.aged());
    let probe = harness.file(".accelerator-probe-77", harness.aged());
    let tree = harness.trees().plant_tree(
        &on_host("retired"),
        &digest('a'),
        harness.aged(),
    );

    harness.dispatched(
        &release,
        Ports {
            locks: Arc::new(FailingLockAt(harness.join(ROOT_LOCK))),
            ..harness.ports()
        },
    );

    assert!(harness.has(&unclaimed));
    assert!(
        probe.exists(),
        "the residue sweep ran without the root lock"
    );
    tree.assert_reclaimed();
}

#[test]
fn an_enolck_tree_lock_skips_only_that_artifact_on_cold_paths() {
    for trigger in COLD_TRIGGERS {
        let harness = RootHarness::new();
        let trees = harness.trees();
        let failing = on_host("retired-x");
        let kept = trees.plant_tree(&failing, &digest('a'), harness.aged());
        let superseded = trees.plant_tree(
            &on_host(MATERIALISED),
            &digest('b'),
            harness.aged(),
        );

        let fired = harness.fire(
            trigger,
            Ports {
                locks: Arc::new(FailingLockAt(
                    trees.paths().single_flight_lock(&failing),
                )),
                ..harness.ports()
            },
        );

        if !fired.ran() {
            return;
        }
        kept.assert_survives();
        superseded.assert_reclaimed();
    }
}

#[test]
fn a_development_checkout_relocated_elsewhere_prunes_on_its_cold_path() {
    let harness = RootHarness::new();
    let Some(release) = harness.release() else {
        return;
    };
    let checkout = harness.anchor.path().join("checkout");
    std::fs::create_dir_all(checkout.join("bin")).expect("checkout bin");
    std::fs::write(checkout.join("bin/accelerator"), b"#!/bin/sh\n")
        .expect("checkout launcher");
    let unclaimed = harness.entry(OLD, harness.aged());
    let start = start_up(
        &CacheRootConfig {
            cache_dir_override: Some(harness.root.clone()),
            plugin_root: Some(checkout),
        },
        TrustedOwners::none(),
        harness.ports().build(),
    );

    cold_dispatch_through(
        &start,
        &release,
        &[],
        &SnapshotAtExec::watching(harness.root.clone()),
    );

    assert!(!harness.has(&unclaimed));
}

fn owned_by_another_uid(path: &Path) -> EntryMetadata {
    EntryMetadata {
        owner: ProcessOwnership::default().effective_uid() + 1,
        ..SystemFilesystem.entry(path).expect("metadata")
    }
}

#[test]
fn an_unsafe_root_is_not_pruned_from_any_trigger() {
    for unsafe_metadata in [shared_group_writable, owned_by_another_uid] {
        for trigger in COLD_TRIGGERS {
            let harness = RootHarness::new();
            let unclaimed = harness.entry(OLD, harness.aged());
            let tree = harness.trees().plant_tree(
                &on_host("retired"),
                &digest('a'),
                harness.aged(),
            );
            let filesystem = ReportingFilesystem::over_system()
                .reporting(&harness.root, unsafe_metadata(&harness.root));

            if !harness
                .fire(
                    trigger,
                    Ports {
                        filesystem: Arc::new(filesystem),
                        ..harness.ports()
                    },
                )
                .ran()
            {
                return;
            }

            assert!(harness.has(&unclaimed), "{trigger:?}");
            tree.assert_survives();
        }
    }
}

/// What can set a root prune off: the verb, or a cold path of a dispatch.
#[derive(Debug, Clone, Copy)]
enum Trigger {
    Verb,
    ColdStore,
    Materialise,
}

const COLD_TRIGGERS: [Trigger; 2] = [Trigger::ColdStore, Trigger::Materialise];

const EVERY_TRIGGER: [Trigger; 3] =
    [Trigger::Verb, Trigger::ColdStore, Trigger::Materialise];

/// The tree every materialise trigger fetches, sorting after the trees the
/// tests plant so a whole-tree prune reaches them first.
const MATERIALISED: &str = "zz-materialised";

enum Fired {
    Verb(Verb),
    Dispatch,
    /// This host cannot fire the trigger, as when `minisign` is absent
    /// outside CI.
    Skipped,
}

impl Fired {
    const fn ran(&self) -> bool {
        !matches!(self, Self::Skipped)
    }
}

impl RootHarness {
    fn fire(&self, trigger: Trigger, ports: Ports) -> Fired {
        match trigger {
            Trigger::Verb => Fired::Verb(self.prune(ports)),
            Trigger::ColdStore => {
                let Some(release) = self.release() else {
                    return Fired::Skipped;
                };
                self.dispatched(&release, ports);
                Fired::Dispatch
            }
            Trigger::Materialise => {
                let Some(release) = self.release_with_trees(&[MATERIALISED])
                else {
                    return Fired::Skipped;
                };
                let start =
                    self.start_over(&self.root, TrustedOwners::none(), ports);
                materialise_through(&start, &release, MATERIALISED)
                    .expect("materialised");
                Fired::Dispatch
            }
        }
    }
}

fn materialise_through(
    start: &StartUp,
    release: &SignedRelease,
    artifact: &str,
) -> Result<SealedTree, TreeError> {
    let keys = release.keys();
    let fetcher =
        Fetcher::with_backoff(Duration::from_millis(1)).expect("fetcher");
    start
        .tree_resolver(tree_config(release, &keys, &fetcher))
        .expect("tree resolver")
        .materialise(artifact)
}

fn repair_through(
    start: &StartUp,
    release: &SignedRelease,
    artifact: &str,
) -> SealedTree {
    let keys = release.keys();
    let fetcher =
        Fetcher::with_backoff(Duration::from_millis(1)).expect("fetcher");
    start
        .tree_resolver(tree_config(release, &keys, &fetcher))
        .expect("tree resolver")
        .repair(artifact, true)
        .expect("repaired")
}

#[test]
fn a_materialise_with_the_root_lock_held_still_prunes_other_trees() {
    let harness = Arc::new(RootHarness::new());
    let unclaimed = harness.entry(OLD, harness.aged());
    let probe = harness.file(".accelerator-probe-77", harness.aged());
    let tree = harness.trees().plant_tree(
        &on_host("retired"),
        &digest('a'),
        harness.aged(),
    );
    let held = FlockLocks.take(&harness.join(ROOT_LOCK)).expect("held");
    let (done, finished) = mpsc::channel();
    let materialiser = {
        let harness = Arc::clone(&harness);
        std::thread::spawn(move || {
            let ran = harness.fire(Trigger::Materialise, harness.ports()).ran();
            done.send(()).expect("announced");
            ran
        })
    };

    let reached = finished.recv_timeout(BOUND);

    drop(held);
    let ran = materialiser.join().expect("materialiser");
    assert!(reached.is_ok(), "the materialise waited for the root lock");
    if !ran {
        return;
    }
    assert!(harness.has(&unclaimed));
    assert!(
        probe.exists(),
        "the residue sweep ran without the root lock"
    );
    tree.assert_reclaimed();
}

#[test]
fn a_materialise_releases_its_lock_before_pruning_other_artifacts() {
    let harness = Arc::new(RootHarness::new());
    let Some(release) = harness.release_with_trees(&[MATERIALISED]) else {
        return;
    };
    harness.trees().plant_tree(
        &on_host("retired"),
        &digest('a'),
        harness.aged(),
    );
    let (announce, announced) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    let pause = Arc::new(Pause {
        at: PruneStep::TreeLockHeld,
        reached: Mutex::new(Some(announce)),
        resume: Mutex::new(resumed),
    });
    let materialiser = {
        let harness = Arc::clone(&harness);
        std::thread::spawn(move || {
            let start = harness.start_over(
                &harness.root,
                TrustedOwners::none(),
                Ports {
                    steps: pause,
                    ..harness.ports()
                },
            );
            materialise_through(&start, &release, MATERIALISED).is_ok()
        })
    };
    announced.recv_timeout(BOUND).expect("the prune paused");

    let taken = FlockLocks.try_take(
        &harness
            .trees()
            .paths()
            .single_flight_lock(&on_host(MATERIALISED)),
    );

    let taken = taken.expect("lock call").is_some();
    resume.send(()).expect("resumed");
    assert!(materialiser.join().expect("materialiser"));
    assert!(taken, "the materialise still held its lock");
}

#[test]
fn a_materialise_prunes_the_superseded_tree_single_files_and_probes() {
    let harness = RootHarness::new();
    let unclaimed = harness.entry(OLD, harness.aged());
    let probe = ".accelerator-probe-77";
    harness.file(probe, harness.aged());
    let superseded = harness.trees().plant_tree(
        &on_host(MATERIALISED),
        &digest('b'),
        harness.aged(),
    );
    let Some(release) = harness.release_with_trees(&[MATERIALISED]) else {
        return;
    };
    let start = harness.start_over(
        &harness.root,
        TrustedOwners::none(),
        harness.ports(),
    );

    let materialised = materialise_through(&start, &release, MATERIALISED)
        .expect("materialised");

    assert!(!harness.has(&unclaimed));
    assert!(!harness.has(probe));
    superseded.assert_reclaimed();
    assert!(materialised.path.exists(), "the new tree was removed");
}

struct ReapableResidue {
    temp: PathBuf,
    unreferenced: PathBuf,
}

impl ReapableResidue {
    fn plant(trees: &Trees, modified: u64) -> Self {
        let artifact = on_host(MATERIALISED);
        Self {
            temp: trees.plant_temp_generation(
                &artifact,
                &digest('c'),
                modified,
            ),
            unreferenced: trees.plant_generation(
                &Trees::generation_of(&artifact, &digest('d')),
                modified,
            ),
        }
    }

    #[track_caller]
    fn assert_survives(&self) {
        assert!(exists(&self.temp), "the temp generation was reaped");
        assert!(exists(&self.unreferenced), "the generation was reaped");
    }

    #[track_caller]
    fn assert_reaped(&self) {
        assert!(!exists(&self.temp), "the temp generation survived");
        assert!(!exists(&self.unreferenced), "the generation survived");
    }
}

#[test]
fn a_materialise_in_an_unsafe_root_reaps_nothing() {
    let harness = RootHarness::new();
    let residue =
        ReapableResidue::plant(&harness.trees(), harness.now - 86_400);
    let filesystem = ReportingFilesystem::over_system()
        .reporting(&harness.root, shared_group_writable(&harness.root));

    let fired = harness.fire(
        Trigger::Materialise,
        Ports {
            filesystem: Arc::new(filesystem),
            ..harness.ports()
        },
    );

    if !fired.ran() {
        return;
    }
    residue.assert_survives();
}

#[test]
fn a_materialise_with_an_untrusted_trees_directory_reaps_nothing() {
    for (relocated, trusted) in
        [(true, false), (true, true), (false, false), (false, true)]
    {
        let harness = RootHarness::new();
        let Some(release) = harness.release_with_trees(&[MATERIALISED]) else {
            return;
        };
        let root = if relocated {
            harness.root.clone()
        } else {
            harness.plugin.join("bin")
        };
        let trees = Trees::under(&root);
        let residue = ReapableResidue::plant(&trees, harness.now - 86_400);
        let reporting = ReportingFilesystem::over_system();
        let filesystem = if trusted {
            reporting
        } else {
            reporting
                .reporting(trees.root(), shared_group_writable(trees.root()))
        };
        let start = start_up(
            &CacheRootConfig {
                cache_dir_override: relocated.then(|| root.clone()),
                plugin_root: Some(harness.plugin.clone()),
            },
            TrustedOwners::none(),
            Ports {
                filesystem: Arc::new(filesystem),
                ..harness.ports()
            }
            .build(),
        );

        materialise_through(&start, &release, MATERIALISED)
            .expect("materialised");

        if trusted {
            residue.assert_reaped();
        } else {
            residue.assert_survives();
        }
    }
}

#[test]
fn a_forced_repair_in_an_unsafe_root_reaps_nothing() {
    for safe in [false, true] {
        let harness = RootHarness::new();
        let Some(release) = harness.release_with_trees(&[MATERIALISED]) else {
            return;
        };
        let filesystem = if safe {
            ReportingFilesystem::over_system()
        } else {
            ReportingFilesystem::over_system()
                .reporting(&harness.root, shared_group_writable(&harness.root))
        };
        let start = harness.start_over(
            &harness.root,
            TrustedOwners::none(),
            Ports {
                filesystem: Arc::new(filesystem),
                ..harness.ports()
            },
        );
        let superseded = materialise_through(&start, &release, MATERIALISED)
            .expect("materialised");

        let repaired = repair_through(&start, &release, MATERIALISED);

        assert_ne!(repaired.path, superseded.path);
        assert_eq!(exists(&superseded.path), !safe, "safe: {safe}");
    }
}

/// One sequence of the lock releases and removals a prune makes, so a test
/// can see which came first.
#[derive(Clone, Default)]
struct Sequence(Arc<Mutex<Vec<Sequenced>>>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Sequenced {
    Removing(PathBuf),
    Released(PathBuf),
}

impl Sequence {
    fn events(&self) -> Vec<Sequenced> {
        self.0.lock().expect("sequence").clone()
    }

    fn push(&self, event: Sequenced) {
        self.0.lock().expect("sequence").push(event);
    }
}

impl RemovalObserver for Sequence {
    fn removing(&self, entry: &Path) {
        self.push(Sequenced::Removing(entry.to_path_buf()));
    }
}

struct ReleasedInSequence {
    path: PathBuf,
    sequence: Sequence,
    _guard: LockGuard,
}

impl Drop for ReleasedInSequence {
    fn drop(&mut self) {
        self.sequence.push(Sequenced::Released(self.path.clone()));
    }
}

impl Sequence {
    fn guarding(&self, path: &Path, guard: LockGuard) -> LockGuard {
        LockGuard::holding(ReleasedInSequence {
            path: path.to_path_buf(),
            sequence: self.clone(),
            _guard: guard,
        })
    }
}

impl SingleFlightLocks for Sequence {
    fn take(&self, path: &Path) -> Result<LockGuard, TreeError> {
        FlockLocks
            .take(path)
            .map(|guard| self.guarding(path, guard))
    }

    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError> {
        Ok(FlockLocks
            .try_take(path)?
            .map(|guard| self.guarding(path, guard)))
    }

    fn wait_existing(
        &self,
        path: &Path,
    ) -> Result<Option<LockGuard>, TreeError> {
        Ok(FlockLocks
            .wait_existing(path)?
            .map(|guard| self.guarding(path, guard)))
    }

    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError> {
        Ok(match FlockLocks.probe(path)? {
            LockProbe::Taken(guard) => {
                LockProbe::Taken(self.guarding(path, guard))
            }
            probed => probed,
        })
    }
}

#[test]
fn prune_held_removes_the_superseded_tree_before_the_lock_is_released() {
    let harness = RootHarness::new();
    let trees = harness.trees();
    let artifact = on_host(MATERIALISED);
    let superseded = trees.plant_tree(&artifact, &digest('b'), harness.aged());
    let sequence = Sequence::default();

    let fired = harness.fire(
        Trigger::Materialise,
        Ports {
            locks: Arc::new(sequence.clone()),
            removals: Arc::new(sequence.clone()),
            ..harness.ports()
        },
    );

    if !fired.ran() {
        return;
    }
    let events = sequence.events();
    let removed = events
        .iter()
        .position(|event| {
            *event == Sequenced::Removing(superseded.generation.clone())
        })
        .expect("the superseded generation was removed");
    let released = events
        .iter()
        .position(|event| {
            *event
                == Sequenced::Released(
                    trees.paths().single_flight_lock(&artifact),
                )
        })
        .expect("the materialise's lock was released");
    assert!(removed < released, "{events:#?}");
}

#[test]
fn the_plugin_bin_orphan_reap_is_unchanged() {
    let harness = RootHarness::new();
    let Some(release) = harness.release_with_trees(&[MATERIALISED]) else {
        return;
    };
    let bin = harness.plugin.join("bin");
    let trees = Trees::under(&bin);
    let artifact = on_host(MATERIALISED);
    let old = trees.plant_generation(
        &Trees::generation_of(&artifact, &digest('c')),
        harness.aged(),
    );
    let young = trees.plant_generation(
        &Trees::generation_of(&artifact, &digest('d')),
        harness.young(),
    );
    let start = start_up(
        &CacheRootConfig {
            cache_dir_override: None,
            plugin_root: Some(harness.plugin.clone()),
        },
        TrustedOwners::none(),
        harness.ports().build(),
    );

    materialise_through(&start, &release, MATERIALISED).expect("materialised");

    assert!(!exists(&old));
    assert!(exists(&young));
}

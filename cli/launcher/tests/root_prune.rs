//! `accelerator cache prune` in a relocated cache root, driven through
//! `start_up` and `cache::run` as production drives them.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tempfile::TempDir;

use accelerator::launch::cache;
use accelerator::launch::core::tree::{Clock, TreeError};
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
    NoPruneSteps, PruneStep, PruneSteps,
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
use accelerator::launch::outbound::resolve::tree::layout::Artifact;
use accelerator::launch::outbound::resolve::tree::lease::{
    Liveness, LivenessProbe,
};
use accelerator::launch::outbound::resolve::tree::locks::{
    FlockLocks, TreeLocks,
};
use accelerator::launch::outbound::resolve::tree::{
    ExpectedDigests, NoSteps, TreePrune, TreePrunePorts, TreePruneRegime,
};
use accelerator::launch::outbound::resolve::HOST_PLATFORM;
use accelerator::launch::start_up::{start_up, StartUp, TreeConfig};
use common::recording_filesystem::{
    in_root, FilesystemCall, RecordedPath, RecordingFilesystem, ScriptedEvent,
    TimelineEvent,
};
use common::scripted_filesystem::ScriptedFilesystem;
use common::trees::{
    another_platform, digest, exists, on_host, set_mode, set_mtime, FixedClock,
    FixedLiveness, Trees,
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

    let verb = harness.prune(harness.ports());

    verb.succeeded();
    assert!(foreign.iter().all(|name| harness.has(name)));
    assert!(foreign_claim.exists());
    assert!(foreign_in_trees.iter().all(|path| exists(path)));
    assert!(foreign_directories.iter().all(|path| exists(path)));
    assert!(!harness.has(&unclaimed), "the positive control survived");
    unclaimed_tree.assert_reclaimed();
    assert!(verb.out.contains("reclaimed 2 entries"), "{}", verb.out);
}

#[test]
fn the_kept_names_survive_beside_an_unclaimed_version() {
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

    harness
        .prune(Ports {
            bootstrap_holders: Arc::new(LiveHolders(vec![4242])),
            ..harness.ports()
        })
        .succeeded();

    assert!(lock.join("pid").exists());
    assert!(kept.iter().all(|name| harness.has(name)));
    assert!(harness.has("trees"));
    assert!(!harness.has(&unclaimed));
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
    assert_eq!(
        removals,
        [
            FilesystemCall::RemoveFile(in_root(format!("{entry}.minisig"))),
            FilesystemCall::RemoveFile(in_root(&entry)),
            FilesystemCall::RemoveFile(recorder.recorded(&claim)),
        ]
    );
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
        harness.claim(OLD, harness.now - 2 * day);

        harness
            .prune_older_than(harness.ports(), older_than)
            .succeeded();

        assert_eq!(harness.has(&entry), survives, "{older_than:?}");
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
    let claimed =
        trees.plant_tree(&on_host("retired"), &digest('c'), harness.aged());
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
    let stale = trees.plant_claim(&digest('a'), OTHER_ID, stale_seconds);
    let fresh = trees.plant_claim(&digest('b'), OTHER_ID, harness.now - 60);
    let unparsed_launcher =
        trees.plant_claim(&digest('c'), "test-install", stale_seconds);
    let foreign = trees.claims().join("not-a-claim");
    std::fs::write(&foreign, b"").expect("foreign");
    set_mtime(&foreign, stale_seconds);

    harness.prune(harness.ports()).succeeded();

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
        trees.plant_claim(&digest('a'), OTHER_ID, harness.now - 2 * day);

        harness
            .prune_older_than(harness.ports(), older_than)
            .succeeded();

        assert_eq!(exists(&tree.generation), survives, "{older_than:?}");
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

/// Fails every lock call on one path and takes every other for real.
struct FailingLockAt(PathBuf);

impl FailingLockAt {
    fn fails(&self, path: &Path) -> Result<(), TreeError> {
        if path == self.0 {
            return Err(TreeError::Lease {
                detail: "No locks available (os error 77)".to_owned(),
            });
        }
        Ok(())
    }
}

impl SingleFlightLocks for FailingLockAt {
    fn take(&self, path: &Path) -> Result<LockGuard, TreeError> {
        self.fails(path)?;
        FlockLocks.take(path)
    }

    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError> {
        self.fails(path)?;
        FlockLocks.try_take(path)
    }

    fn wait_existing(
        &self,
        path: &Path,
    ) -> Result<Option<LockGuard>, TreeError> {
        self.fails(path)?;
        FlockLocks.wait_existing(path)
    }

    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError> {
        self.fails(path)?;
        FlockLocks.probe(path)
    }
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
    let young = trees.plant_generation(
        &Trees::generation_of(&on_host("retired-c"), &digest('c')),
        harness.young(),
    );

    harness.prune(harness.ports()).succeeded();

    assert!(sidecars.iter().all(|sidecar| !exists(sidecar)));
    assert!(!exists(&lock), "the retired lock survived");
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
            filesystem: ports.filesystem.as_ref(),
            ownership: ports.ownership.as_ref(),
            clock: ports.clock.as_ref(),
            lease_liveness: ports.lease_liveness.as_ref(),
            pins: ports.pins.as_ref(),
            locks: TreeLocks::over(ports.locks.as_ref()),
            steps: ports.steps.as_ref(),
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

        prune_trees_directly(&harness, &harness.ports().build(), patience);

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

/// The process's own identity, with its primary group taken to be private.
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

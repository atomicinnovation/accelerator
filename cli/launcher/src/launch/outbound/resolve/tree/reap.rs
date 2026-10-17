//! Reclaiming residue: partial archives, partial trees, and generations no
//! pointer references.
//!
//! Runs under the per-`(name, platform)` single-flight lock held by its caller,
//! so a reaper and a materialiser of a different artifact cannot disagree about
//! what `trees/` contains. It never runs from the hit path.
//!
//! Two signals gate a removal, and they compose. A generation whose lease is
//! held by a live process is spared regardless of age — the lease is the
//! liveness oracle. A generation nothing holds is reclaimed once it is older
//! than the backstop, which is the only thing that reclaims a generation whose
//! filesystem cannot answer `flock`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use kernel::TEMP_PREFIX;

use crate::launch::core::tree::{Clock, TreeError};

use super::layout::{Artifact, TreePaths, LAYOUT_VERSION};
use super::lease::{
    probe_single_flight, Liveness, LivenessProbe, SingleFlight,
    SingleFlightProbe,
};
use super::tree_lock::HeldTreeLock;

/// Generations younger than this with no live lease are spared, so a generation
/// briefly between its rename and its pointer write is never mistaken for
/// crash residue. Sized well beyond a fetch-plus-extract.
const AGE_BACKSTOP: Duration = Duration::from_secs(60 * 60);

/// Which digest each artifact pins, so a partial archive is judged against
/// its own artifact's pin rather than the reaping caller's.
pub trait PinnedDigests {
    fn pinned_digest(&self, artifact: &Artifact) -> Option<String>;
}

/// Sees each entry the reap is about to remove; a generation's sidecars go
/// with it, unseen.
pub trait RemovalObserver {
    fn removing(&self, entry: &Path);
}

pub struct UnobservedRemovals;

impl RemovalObserver for UnobservedRemovals {
    fn removing(&self, _entry: &Path) {}
}

enum OwnerLock {
    HeldByCaller,
    Taken { _guard: SingleFlight },
    Absent,
    HeldElsewhere,
    Unprobeable(TreeError),
}

const fn temp_residue_is_reclaimable(lock: &OwnerLock) -> bool {
    matches!(
        lock,
        OwnerLock::HeldByCaller | OwnerLock::Taken { .. } | OwnerLock::Absent
    )
}

/// Reclaims residue under the root its caller's tree lock was taken under.
pub struct Reaper<'a> {
    held: &'a HeldTreeLock,
    clock: &'a dyn Clock,
    liveness: &'a dyn LivenessProbe,
    pins: &'a dyn PinnedDigests,
    observer: &'a dyn RemovalObserver,
}

impl<'a> Reaper<'a> {
    #[must_use]
    pub const fn new(
        held: &'a HeldTreeLock,
        clock: &'a dyn Clock,
        liveness: &'a dyn LivenessProbe,
        pins: &'a dyn PinnedDigests,
    ) -> Self {
        Self {
            held,
            clock,
            liveness,
            pins,
            observer: &UnobservedRemovals,
        }
    }

    #[must_use]
    pub const fn observed_by(self, observer: &'a dyn RemovalObserver) -> Self {
        Self { observer, ..self }
    }

    /// Reclaim orphan temp residue and unreferenced generations under
    /// `trees/`.
    ///
    /// A partial archive whose digest is pinned for its own artifact is
    /// spared, so a resumable download is not reclaimed before the next run
    /// can continue it. An entry with no known owner is spared too: it is a
    /// newer launcher's, on a platform this one does not know. Another
    /// artifact's temp residue goes only under that artifact's tree lock,
    /// taken without blocking, or when it has no lock file; otherwise it is
    /// kept. A fully-materialised generation goes only when no
    /// pointer names it, no live lease is held on it, and it is older than
    /// the backstop.
    ///
    /// # Errors
    ///
    /// [`TreeError::Pointer`] if `trees/` cannot be read.
    pub fn reap_orphans(&self) -> Result<Reclaimed, TreeError> {
        let names = entry_names(self.paths().root())?;
        let referenced = referenced_generations(self.paths().root());
        let mut reclaimed = Reclaimed::default();
        for (owner, entries) in temp_entries_by_owner(&names) {
            self.reap_temp_entries_of(&owner, &entries, &mut reclaimed);
        }
        self.reap_generations(&names, &referenced, &mut reclaimed);
        Ok(reclaimed)
    }

    fn reap_temp_entries_of(
        &self,
        owner: &Artifact,
        entries: &[&str],
        reclaimed: &mut Reclaimed,
    ) {
        let lock = self.lock_of(owner);
        if let OwnerLock::Unprobeable(error) = &lock {
            tracing::warn!(
                artifact = owner.name(),
                platform = owner.platform(),
                %error,
                "cannot probe the tree lock; keeping its temp residue"
            );
        }
        if !temp_residue_is_reclaimable(&lock) {
            return;
        }
        for entry in entries {
            if !is_resumable(entry, owner, self.pins) {
                self.remove(&self.paths().root().join(entry), reclaimed);
            }
        }
    }

    fn lock_of(&self, owner: &Artifact) -> OwnerLock {
        if owner == self.held.artifact() {
            return OwnerLock::HeldByCaller;
        }
        match probe_single_flight(&self.paths().single_flight_lock_of(owner)) {
            Ok(SingleFlightProbe::Taken(guard)) => {
                OwnerLock::Taken { _guard: guard }
            }
            Ok(SingleFlightProbe::Absent) => OwnerLock::Absent,
            Ok(SingleFlightProbe::Held) => OwnerLock::HeldElsewhere,
            Err(error) => OwnerLock::Unprobeable(error),
        }
    }

    const fn paths(&self) -> &TreePaths {
        self.held.paths()
    }

    fn reap_generations(
        &self,
        names: &[String],
        referenced: &BTreeSet<String>,
        reclaimed: &mut Reclaimed,
    ) {
        for name in names.iter().filter(|name| is_generation(name)) {
            if is_reclaimable_generation(
                self.paths(),
                name,
                referenced,
                self.clock,
                self.liveness,
            ) {
                self.remove(&self.paths().generation(name), reclaimed);
                for sidecar in sidecars_of(self.paths(), name) {
                    let _ = std::fs::remove_file(sidecar);
                }
            }
        }
    }

    fn remove(&self, path: &Path, reclaimed: &mut Reclaimed) {
        self.observer.removing(path);
        let outcome = if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        };
        if outcome.is_ok() {
            reclaimed.entries += 1;
        }
    }
}

/// How much a reap reclaimed, for the caller to report.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reclaimed {
    pub entries: u64,
}

fn entry_names(root: &Path) -> Result<Vec<String>, TreeError> {
    match std::fs::read_dir(root) {
        Ok(entries) => Ok(entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(Vec::new())
        }
        Err(error) => Err(TreeError::Pointer {
            detail: format!("cannot read the trees directory: {error}"),
        }),
    }
}

/// Lock files and entries with no known owner parse to no owner, so they are
/// never grouped and never reaped.
fn temp_entries_by_owner(names: &[String]) -> BTreeMap<Artifact, Vec<&str>> {
    let mut by_owner: BTreeMap<Artifact, Vec<&str>> = BTreeMap::new();
    for name in names {
        if let Some(owner) = Artifact::owning(name) {
            by_owner.entry(owner).or_default().push(name);
        }
    }
    by_owner
}

fn is_resumable(
    entry: &str,
    owner: &Artifact,
    pins: &dyn PinnedDigests,
) -> bool {
    owner.partial_archive_digest(entry).is_some_and(|digest| {
        pins.pinned_digest(owner).as_deref() == Some(digest)
    })
}

/// A generation directory: the fixed grammar, no sidecar suffix, and never a
/// temp entry, which only the temp arm may judge.
fn is_generation(name: &str) -> bool {
    !name.starts_with(TEMP_PREFIX)
        && name.contains(&format!("-{LAYOUT_VERSION}-"))
        && !has_sidecar_suffix(name)
}

fn has_sidecar_suffix(name: &str) -> bool {
    [".ref", ".sealed", ".sealed.sig", lease::LEASE_SUFFIX]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// The generation directory names any pointer currently references.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn referenced_generations(root: &Path) -> BTreeSet<String> {
    let mut referenced = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return referenced;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.ends_with(".ref") {
            if let Ok(contents) = std::fs::read_to_string(entry.path()) {
                referenced.insert(contents.trim().to_owned());
            }
        }
    }
    referenced
}

fn is_reclaimable_generation(
    paths: &TreePaths,
    name: &str,
    referenced: &BTreeSet<String>,
    clock: &dyn Clock,
    liveness: &dyn LivenessProbe,
) -> bool {
    if referenced.contains(name) {
        return false;
    }
    orphan_is_reclaimable(
        liveness.probe(&paths.lease(name)),
        generation_is_aged(paths, name, clock),
    )
}

/// Whether `reclaim_unclaimed` may remove a pointed-at generation whose claim
/// has lapsed.
pub(super) const fn unclaimed_is_reclaimable(
    liveness: Liveness,
    aged: bool,
) -> bool {
    match liveness {
        Liveness::Held => false,
        Liveness::Free => true,
        Liveness::Unknown => aged,
    }
}

/// What `repair` does with the generation it has just superseded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SupersededDisposition {
    Reclaim,
    /// Left for the age-gated reaper, with its backstop restarted.
    Defer,
    Keep,
}

pub(super) const fn superseded_disposition(
    liveness: Liveness,
) -> SupersededDisposition {
    match liveness {
        Liveness::Free => SupersededDisposition::Reclaim,
        Liveness::Unknown => SupersededDisposition::Defer,
        Liveness::Held => SupersededDisposition::Keep,
    }
}

/// Unlike an unclaimed generation, an unreferenced one is reclaimed only once
/// aged even when free, since a young one may sit between its rename and its
/// pointer write.
const fn orphan_is_reclaimable(liveness: Liveness, aged: bool) -> bool {
    !matches!(liveness, Liveness::Held) && aged
}

pub(super) fn generation_is_aged(
    paths: &TreePaths,
    generation: &str,
    clock: &dyn Clock,
) -> bool {
    older_than_backstop(&paths.generation(generation), clock)
}

/// Sets the generation's mtime to the clock's now, so the backstop counts from
/// this moment rather than from extraction. Best-effort: where the filesystem
/// rejects it, the generation keeps its extraction mtime and a warning says so.
pub(super) fn restart_backstop(
    paths: &TreePaths,
    generation: &str,
    clock: &dyn Clock,
) {
    #[cfg(unix)]
    {
        use rustix::fs::{AtFlags, Timespec, Timestamps, CWD, UTIME_OMIT};

        let timestamps = Timestamps {
            last_access: Timespec {
                tv_sec: 0,
                tv_nsec: UTIME_OMIT,
            },
            last_modification: Timespec {
                tv_sec: i64::try_from(clock.now_seconds()).unwrap_or(i64::MAX),
                tv_nsec: 0,
            },
        };
        let path = paths.generation(generation);
        if let Err(error) =
            rustix::fs::utimensat(CWD, &path, &timestamps, AtFlags::empty())
        {
            tracing::warn!(
                generation = %path.display(),
                %error,
                "cannot restart the age backstop; the generation keeps its \
                 extraction age"
            );
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (paths, generation, clock);
    }
}

/// Age is measured against the injected clock rather than `SystemTime::now()`,
/// so a test advances time by advancing the clock rather than by back-dating an
/// mtime — which is the discipline the repository's lock-timing flake history
/// demands.
fn older_than_backstop(path: &Path, clock: &dyn Clock) -> bool {
    let Some(modified) = modified_unix_seconds(path) else {
        return false;
    };
    clock
        .now_seconds()
        .checked_sub(modified)
        .is_some_and(|age| age >= AGE_BACKSTOP.as_secs())
}

fn modified_unix_seconds(path: &Path) -> Option<u64> {
    let modified = std::fs::symlink_metadata(path).ok()?.modified().ok()?;
    modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

/// The sidecars belonging to a reclaimed generation, removed alongside it.
#[must_use]
pub fn sidecars_of(paths: &TreePaths, generation: &str) -> Vec<PathBuf> {
    vec![
        paths.attestation(generation),
        paths.attestation_signature(generation),
        paths.lease(generation),
    ]
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use kernel::TEMP_PREFIX;

    use crate::launch::core::tree::{Clock, TreeError};

    use super::super::layout::{
        generation_name, Artifact, TreePaths, LAYOUT_VERSION,
    };
    use super::super::lease::{
        probe_single_flight, take_single_flight, FlockProbe, Liveness,
        SingleFlight, SingleFlightProbe,
    };
    use super::super::testing::{logged, within, BLOCKING_BOUND};
    use super::super::tree_lock::HeldTreeLock;
    use super::{
        generation_is_aged, orphan_is_reclaimable, restart_backstop,
        sidecars_of, superseded_disposition, temp_residue_is_reclaimable,
        unclaimed_is_reclaimable, OwnerLock, PinnedDigests, Reaper, Reclaimed,
        RemovalObserver, SupersededDisposition, UnobservedRemovals,
        AGE_BACKSTOP,
    };

    /// A clock the test drives forward, so a generation ages because time
    /// advances rather than because its mtime was back-dated.
    struct AtClock(u64);
    impl Clock for AtClock {
        fn now_seconds(&self) -> u64 {
            self.0
        }
        fn sleep_poll_interval(&self) {}
    }

    /// Now — freshly written files carry an mtime near this.
    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after the epoch")
            .as_secs()
    }

    /// A clock reading well past the backstop, so real recent mtimes are old.
    fn well_aged() -> AtClock {
        AtClock(now() + AGE_BACKSTOP.as_secs() * 2)
    }

    /// A clock reading the present, so recent mtimes are young.
    fn present() -> AtClock {
        AtClock(now())
    }

    const DIGEST: &str =
        "abc0000000000000000000000000000000000000000000000000000000000123";

    fn trees(root: &Path) -> TreePaths {
        let paths = TreePaths::under(root);
        fs::create_dir_all(paths.root()).expect("trees dir");
        paths
    }

    const PLATFORM: &str = "linux-x64";
    const SUFFIX: &str = "0123456789abcdef";

    fn y() -> Artifact {
        Artifact::new("driver", PLATFORM)
    }

    fn gen_name() -> String {
        generation_name("browser", PLATFORM, DIGEST, LAYOUT_VERSION, SUFFIX)
    }

    #[derive(Default)]
    struct Pins(BTreeMap<Artifact, String>);

    impl PinnedDigests for Pins {
        fn pinned_digest(&self, artifact: &Artifact) -> Option<String> {
            self.0.get(artifact).cloned()
        }
    }

    impl Pins {
        fn pinning(pins: &[(Artifact, &str)]) -> Self {
            Self(
                pins.iter()
                    .map(|(artifact, digest)| {
                        (artifact.clone(), (*digest).to_owned())
                    })
                    .collect(),
            )
        }
    }

    /// Reaps `root` for Y while the test thread holds Y's tree lock, as a
    /// caller does, failing rather than hanging if the reap blocks and if
    /// any lock file is created or removed.
    fn reap_for_y_at(
        root: &Path,
        clock: impl Clock + Send + 'static,
        pins: Pins,
        observer: impl RemovalObserver + Send + 'static,
    ) -> Result<Reclaimed, TreeError> {
        let paths = TreePaths::under(root);
        let caller_holds = HeldTreeLock::take(&paths, y()).expect("Y's lock");
        let locks_before = lock_files(&paths);
        let outcome = within(BLOCKING_BOUND, "the reap", move || {
            Reaper::new(&caller_holds, &clock, &FlockProbe, &pins)
                .observed_by(&observer)
                .reap_orphans()
        });
        assert_eq!(lock_files(&paths), locks_before, "a lock file changed");
        outcome
    }

    fn reap_for_y(root: &Path, pins: Pins) -> Result<Reclaimed, TreeError> {
        reap_for_y_at(root, present(), pins, UnobservedRemovals)
    }

    #[derive(Clone, Default)]
    struct Recording(Arc<Mutex<Vec<PathBuf>>>);

    impl RemovalObserver for Recording {
        fn removing(&self, entry: &Path) {
            self.0.lock().expect("recording").push(entry.to_owned());
        }
    }

    impl Recording {
        fn sorted(&self) -> Vec<PathBuf> {
            let mut seen = self.0.lock().expect("recording").clone();
            seen.sort();
            seen
        }
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn lock_files(paths: &TreePaths) -> Vec<String> {
        let mut locks: Vec<String> = fs::read_dir(paths.root())
            .expect("trees dir")
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                name.starts_with(TEMP_PREFIX) && name.ends_with(".lock")
            })
            .collect();
        locks.sort();
        locks
    }

    struct Residue {
        tree: PathBuf,
        pointer: PathBuf,
        archive: PathBuf,
    }

    impl Residue {
        fn paths(&self) -> [&Path; 3] {
            [&self.tree, &self.pointer, &self.archive]
        }
    }

    fn plant_residue(paths: &TreePaths, owner: &Artifact) -> Residue {
        let tree = temp_tree_of(paths, owner);
        let generation = generation_name(
            owner.name(),
            owner.platform(),
            DIGEST,
            LAYOUT_VERSION,
            SUFFIX,
        );
        let pointer = paths.temp_pointer(&generation);
        fs::write(&pointer, &generation).expect("temp pointer");
        let archive = paths.temp_archive_of(owner, &"f".repeat(64));
        fs::write(&archive, b"partial").expect("partial archive");
        Residue {
            tree,
            pointer,
            archive,
        }
    }

    fn plant_archive(
        paths: &TreePaths,
        owner: &Artifact,
        digest: &str,
    ) -> PathBuf {
        let archive = paths.temp_archive_of(owner, digest);
        fs::write(&archive, b"resumable").expect("partial archive");
        archive
    }

    #[test]
    fn a_temp_archive_and_a_temp_tree_are_reclaimed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let archive = paths.temp_archive("browser", PLATFORM, &"f".repeat(64));
        fs::write(&archive, b"partial").expect("archive");
        let temp_tree = paths.temp_generation(&gen_name());
        fs::create_dir(&temp_tree).expect("temp tree");

        let reclaimed = reap_for_y(dir.path(), Pins::default()).expect("reap");
        assert_eq!(reclaimed.entries, 2);
        assert!(!archive.exists());
        assert!(!temp_tree.exists());
    }

    #[test]
    fn a_partial_archive_pinned_for_its_own_artifact_is_spared() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let archive = plant_archive(&paths, &y(), DIGEST);

        let reclaimed = reap_for_y(dir.path(), Pins::pinning(&[(y(), DIGEST)]))
            .expect("reap");
        assert_eq!(reclaimed.entries, 0);
        assert!(archive.exists(), "a resumable partial was reclaimed");
    }

    #[test]
    fn the_reaps_own_residue_goes_but_its_resumable_archive_stays() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let residue = plant_residue(&paths, &y());
        let resumable = plant_archive(&paths, &y(), DIGEST);

        reap_for_y(dir.path(), Pins::pinning(&[(y(), DIGEST)])).expect("reap");

        for entry in residue.paths() {
            assert!(!entry.exists(), "{} survived", entry.display());
        }
        assert!(resumable.exists(), "the resumable archive was reaped");
    }

    fn x() -> Artifact {
        Artifact::new("browser", PLATFORM)
    }

    const X_DIGEST: &str =
        "eee0000000000000000000000000000000000000000000000000000000000eee";

    fn create_free_lock(paths: &TreePaths, owner: &Artifact) {
        drop(
            take_single_flight(&paths.single_flight_lock_of(owner))
                .expect("lock file"),
        );
    }

    #[test]
    fn another_artifacts_pinned_archive_survives() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        create_free_lock(&paths, &x());
        let pinned = plant_archive(&paths, &x(), X_DIGEST);

        reap_for_y(
            dir.path(),
            Pins::pinning(&[(y(), DIGEST), (x(), X_DIGEST)]),
        )
        .expect("reap");

        assert!(pinned.exists(), "another artifact's pinned archive went");
    }

    #[test]
    fn an_archive_pinned_only_for_another_artifact_is_reclaimed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        create_free_lock(&paths, &x());
        let pinned_for_y = plant_archive(&paths, &x(), DIGEST);

        reap_for_y(dir.path(), Pins::pinning(&[(y(), DIGEST)])).expect("reap");

        assert!(
            !pinned_for_y.exists(),
            "an archive spared by another artifact's pin survived"
        );
    }

    fn a() -> Artifact {
        Artifact::new("aardvark", PLATFORM)
    }

    fn z() -> Artifact {
        Artifact::new("zebra", PLATFORM)
    }

    fn temp_tree_of(paths: &TreePaths, owner: &Artifact) -> PathBuf {
        let tree = paths.temp_generation(&generation_name(
            owner.name(),
            owner.platform(),
            DIGEST,
            LAYOUT_VERSION,
            SUFFIX,
        ));
        fs::create_dir(&tree).expect("temp tree");
        tree
    }

    /// Orphan temp trees for artifacts sorting either side of X, each with a
    /// free lock, so a reap that stops at X leaves one of them behind.
    fn plant_bystander_orphans(paths: &TreePaths) -> [PathBuf; 2] {
        [a(), z()].map(|owner| {
            create_free_lock(paths, &owner);
            temp_tree_of(paths, &owner)
        })
    }

    fn hold_lock_of(paths: &TreePaths, owner: &Artifact) -> SingleFlight {
        take_single_flight(&paths.single_flight_lock_of(owner))
            .expect("held elsewhere")
    }

    fn assert_survives(entries: &[&Path]) {
        for entry in entries {
            assert!(entry.exists(), "{} was reaped", entry.display());
        }
    }

    fn assert_gone(entries: &[&Path]) {
        for entry in entries {
            assert!(!entry.exists(), "{} survived", entry.display());
        }
    }

    #[test]
    fn another_artifacts_residue_survives_while_its_tree_lock_is_held() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let _x_held = hold_lock_of(&paths, &x());
        let residue = plant_residue(&paths, &x());
        let [aardvark, zebra] = plant_bystander_orphans(&paths);

        reap_for_y_at(
            dir.path(),
            well_aged(),
            Pins::default(),
            UnobservedRemovals,
        )
        .expect("reap");

        assert_survives(&residue.paths());
        assert_gone(&[&aardvark, &zebra]);
    }

    /// Probes X's lock as each of X's planted entries is removed.
    struct ProbingLockOf {
        watched: Vec<PathBuf>,
        lock: PathBuf,
        seen: Arc<Mutex<Vec<(PathBuf, bool)>>>,
    }

    impl RemovalObserver for ProbingLockOf {
        fn removing(&self, entry: &Path) {
            if self.watched.iter().any(|watched| watched == entry) {
                let held = matches!(
                    probe_single_flight(&self.lock),
                    Ok(SingleFlightProbe::Held)
                );
                self.seen
                    .lock()
                    .expect("seen")
                    .push((entry.to_owned(), held));
            }
        }
    }

    #[test]
    fn another_artifacts_residue_is_removed_under_its_tree_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        create_free_lock(&paths, &x());
        let residue = plant_residue(&paths, &x());
        let pinned = plant_archive(&paths, &x(), X_DIGEST);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let observer = ProbingLockOf {
            watched: residue.paths().map(Path::to_path_buf).into(),
            lock: paths.single_flight_lock_of(&x()),
            seen: Arc::clone(&seen),
        };

        reap_for_y_at(
            dir.path(),
            present(),
            Pins::pinning(&[(x(), X_DIGEST)]),
            observer,
        )
        .expect("reap");

        let mut seen = seen.lock().expect("seen").clone();
        seen.sort();
        let mut expected: Vec<(PathBuf, bool)> = residue
            .paths()
            .map(|entry| (entry.to_path_buf(), true))
            .into();
        expected.sort();
        assert_eq!(seen, expected, "X's residue went without X's lock");
        assert_survives(&[&pinned]);
        assert!(matches!(
            probe_single_flight(&paths.single_flight_lock_of(&x())),
            Ok(SingleFlightProbe::Taken(_))
        ));
    }

    #[test]
    fn a_retired_artifacts_residue_goes_without_creating_its_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let retired = Artifact::new("retired", PLATFORM);
        let residue = plant_residue(&paths, &retired);
        let pinned = plant_archive(&paths, &retired, X_DIGEST);

        reap_for_y(dir.path(), Pins::pinning(&[(retired, X_DIGEST)]))
            .expect("reap");

        assert_gone(&residue.paths());
        assert_survives(&[&pinned]);
    }

    fn plant_unprobeable_lock_of(paths: &TreePaths, owner: &Artifact) {
        fs::create_dir(paths.single_flight_lock_of(owner))
            .expect("a directory at the lock path");
    }

    #[test]
    fn another_artifacts_residue_survives_an_unprobeable_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        plant_unprobeable_lock_of(&paths, &x());
        let residue = plant_residue(&paths, &x());
        let [aardvark, zebra] = plant_bystander_orphans(&paths);

        reap_for_y(dir.path(), Pins::default()).expect("reap");

        assert_survives(&residue.paths());
        assert_gone(&[&aardvark, &zebra]);
    }

    #[test]
    fn an_unprobeable_lock_is_warned_with_its_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        plant_unprobeable_lock_of(&paths, &x());
        plant_residue(&paths, &x());

        let log = logged(|| {
            reap_for_y(dir.path(), Pins::default()).expect("reap");
        });

        assert!(log.contains("WARN"), "no warning: {log}");
        assert!(log.contains("browser"), "warning names no artifact: {log}");
        assert!(log.contains(PLATFORM), "warning names no platform: {log}");
    }

    fn y_on_another_platform() -> Artifact {
        Artifact::new(y().name(), "darwin-arm64")
    }

    #[test]
    fn own_name_on_another_platform_survives_while_its_lock_is_held() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let _held = hold_lock_of(&paths, &y_on_another_platform());
        let residue = plant_residue(&paths, &y_on_another_platform());

        reap_for_y(dir.path(), Pins::default()).expect("reap");

        assert_survives(&residue.paths());
    }

    #[test]
    fn own_name_on_another_platform_goes_while_its_lock_is_free() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        create_free_lock(&paths, &y_on_another_platform());
        let residue = plant_residue(&paths, &y_on_another_platform());

        reap_for_y(dir.path(), Pins::default()).expect("reap");

        assert_gone(&residue.paths());
    }

    #[test]
    fn an_artifact_extending_the_own_name_is_another_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let extending = Artifact::new("driver-extra", PLATFORM);
        let _held = hold_lock_of(&paths, &extending);
        let residue = plant_residue(&paths, &extending);

        reap_for_y(dir.path(), Pins::default()).expect("reap");

        assert_survives(&residue.paths());
    }

    #[test]
    fn entries_with_no_known_owner_survive() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let musl = Artifact::new("browser", "linux-x64-musl");
        let orphan = paths.root().join(".tmp-orphan-residue");
        fs::create_dir(&orphan).expect("orphan");
        let nameless =
            plant_archive(&paths, &Artifact::new("", PLATFORM), DIGEST);
        let musl_archive = plant_archive(&paths, &musl, DIGEST);
        let musl_tree = paths.temp_generation(&generation_name(
            musl.name(),
            musl.platform(),
            DIGEST,
            LAYOUT_VERSION,
            SUFFIX,
        ));
        fs::create_dir(&musl_tree).expect("musl temp tree");

        reap_for_y_at(
            dir.path(),
            well_aged(),
            Pins::default(),
            UnobservedRemovals,
        )
        .expect("reap");

        for entry in [orphan, nameless, musl_archive, musl_tree] {
            assert!(entry.exists(), "{} was reaped", entry.display());
        }
    }

    #[test]
    fn every_counted_removal_was_observed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let residue = plant_residue(&paths, &y());
        let generation = planted(&paths);
        let observer = Recording::default();

        let reclaimed = reap_for_y_at(
            dir.path(),
            well_aged(),
            Pins::default(),
            observer.clone(),
        )
        .expect("reap");

        let mut removed: Vec<PathBuf> =
            residue.paths().map(Path::to_path_buf).into();
        removed.push(paths.generation(&generation));
        removed.sort();
        assert_eq!(observer.sorted(), removed);
        assert_eq!(reclaimed.entries, 4);
    }

    #[test]
    fn a_temp_pointer_reaped_alongside_still_protects_its_generation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let generation = gen_name();
        fs::create_dir(paths.generation(&generation)).expect("generation");
        let pointer = paths.temp_pointer(&generation);
        fs::write(&pointer, &generation).expect("temp pointer");

        reap_for_y_at(
            dir.path(),
            well_aged(),
            Pins::default(),
            UnobservedRemovals,
        )
        .expect("reap");

        assert!(!pointer.exists(), "the temp pointer survived");
        assert!(
            paths.generation(&generation).exists(),
            "the generation the temp pointer named was reaped"
        );
    }

    #[test]
    fn an_unreferenced_aged_generation_is_reclaimed_and_a_referenced_one_spared(
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());

        let orphan = gen_name();
        let orphan_dir = paths.generation(&orphan);
        fs::create_dir(&orphan_dir).expect("orphan");

        let referenced = generation_name(
            "driver",
            "linux-x64",
            DIGEST,
            LAYOUT_VERSION,
            "fedcba9876543210",
        );
        let referenced_dir = paths.generation(&referenced);
        fs::create_dir(&referenced_dir).expect("referenced");
        fs::write(paths.pointer("driver", "linux-x64", DIGEST), &referenced)
            .expect("pointer");

        // The clock reads past the backstop, so both are old — only the
        // reference spares one of them.
        reap_for_y_at(
            dir.path(),
            well_aged(),
            Pins::default(),
            UnobservedRemovals,
        )
        .expect("reap");
        assert!(!orphan_dir.exists(), "the orphan survived");
        assert!(
            referenced_dir.exists(),
            "a referenced generation was reaped"
        );
    }

    #[test]
    fn a_young_unreferenced_generation_is_spared_by_the_backstop() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let fresh = paths.generation(&gen_name());
        fs::create_dir(&fresh).expect("fresh");
        let sidecars = sidecars_of(&paths, &gen_name());
        for sidecar in &sidecars {
            fs::write(sidecar, b"").expect("sidecar");
        }

        // The clock reads the present, so a just-renamed generation — between
        // its rename and its pointer write — is inside the backstop window.
        let reclaimed = reap_for_y(dir.path(), Pins::default()).expect("reap");
        assert_eq!(reclaimed.entries, 0);
        assert!(fresh.exists(), "a freshly renamed generation was reaped");
        for sidecar in &sidecars {
            assert!(sidecar.exists(), "{} was removed", sidecar.display());
        }
    }

    fn a_taken_lock() -> (tempfile::TempDir, SingleFlight) {
        let dir = tempfile::tempdir().expect("tempdir");
        let guard =
            take_single_flight(&dir.path().join("lock")).expect("taken");
        (dir, guard)
    }

    #[test]
    fn own_residue_is_reclaimable() {
        assert!(temp_residue_is_reclaimable(&OwnerLock::HeldByCaller));
    }

    #[test]
    fn residue_under_a_taken_lock_is_reclaimable() {
        let (_dir, guard) = a_taken_lock();
        assert!(temp_residue_is_reclaimable(&OwnerLock::Taken {
            _guard: guard
        }));
    }

    #[test]
    fn residue_with_no_lock_file_is_reclaimable() {
        assert!(temp_residue_is_reclaimable(&OwnerLock::Absent));
    }

    #[test]
    fn residue_under_a_lock_held_elsewhere_is_kept() {
        assert!(!temp_residue_is_reclaimable(&OwnerLock::HeldElsewhere));
    }

    #[test]
    fn residue_under_an_unprobeable_lock_is_kept() {
        assert!(!temp_residue_is_reclaimable(&OwnerLock::Unprobeable(
            TreeError::Lease {
                detail: "cannot open".to_owned()
            }
        )));
    }

    #[test]
    fn an_unclaimed_held_generation_is_kept_while_young() {
        assert!(!unclaimed_is_reclaimable(Liveness::Held, false));
    }

    #[test]
    fn an_unclaimed_held_generation_is_kept_once_aged() {
        assert!(!unclaimed_is_reclaimable(Liveness::Held, true));
    }

    #[test]
    fn an_unclaimed_free_generation_is_reclaimed_while_young() {
        assert!(unclaimed_is_reclaimable(Liveness::Free, false));
    }

    #[test]
    fn an_unclaimed_free_generation_is_reclaimed_once_aged() {
        assert!(unclaimed_is_reclaimable(Liveness::Free, true));
    }

    #[test]
    fn an_unclaimed_unknown_generation_is_kept_while_young() {
        assert!(!unclaimed_is_reclaimable(Liveness::Unknown, false));
    }

    #[test]
    fn an_unclaimed_unknown_generation_is_reclaimed_once_aged() {
        assert!(unclaimed_is_reclaimable(Liveness::Unknown, true));
    }

    #[test]
    fn a_free_superseded_generation_is_reclaimed() {
        assert_eq!(
            superseded_disposition(Liveness::Free),
            SupersededDisposition::Reclaim
        );
    }

    #[test]
    fn an_unknown_superseded_generation_is_deferred() {
        assert_eq!(
            superseded_disposition(Liveness::Unknown),
            SupersededDisposition::Defer
        );
    }

    #[test]
    fn a_held_superseded_generation_is_kept() {
        assert_eq!(
            superseded_disposition(Liveness::Held),
            SupersededDisposition::Keep
        );
    }

    #[test]
    fn a_held_orphan_is_kept_while_young() {
        assert!(!orphan_is_reclaimable(Liveness::Held, false));
    }

    #[test]
    fn a_held_orphan_is_kept_once_aged() {
        assert!(!orphan_is_reclaimable(Liveness::Held, true));
    }

    #[test]
    fn a_free_orphan_is_kept_while_young() {
        assert!(!orphan_is_reclaimable(Liveness::Free, false));
    }

    #[test]
    fn a_free_orphan_is_reclaimed_once_aged() {
        assert!(orphan_is_reclaimable(Liveness::Free, true));
    }

    #[test]
    fn an_unknown_orphan_is_kept_while_young() {
        assert!(!orphan_is_reclaimable(Liveness::Unknown, false));
    }

    #[test]
    fn an_unknown_orphan_is_reclaimed_once_aged() {
        assert!(orphan_is_reclaimable(Liveness::Unknown, true));
    }

    fn planted(paths: &TreePaths) -> String {
        let name = gen_name();
        fs::create_dir(paths.generation(&name)).expect("generation");
        name
    }

    fn mtime_of(path: &Path) -> u64 {
        fs::symlink_metadata(path)
            .expect("metadata")
            .modified()
            .expect("mtime")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after the epoch")
            .as_secs()
    }

    fn back_date(path: &Path, seconds: u64) {
        let then = std::time::SystemTime::now()
            - std::time::Duration::from_secs(seconds);
        fs::File::open(path)
            .expect("open")
            .set_modified(then)
            .expect("back-date");
    }

    #[test]
    fn a_generation_one_second_short_of_the_backstop_is_not_aged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let name = planted(&paths);
        let mtime = mtime_of(&paths.generation(&name));

        let clock = AtClock(mtime + AGE_BACKSTOP.as_secs() - 1);
        assert!(!generation_is_aged(&paths, &name, &clock));
    }

    #[test]
    fn a_generation_exactly_at_the_backstop_is_aged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let name = planted(&paths);
        let mtime = mtime_of(&paths.generation(&name));

        let clock = AtClock(mtime + AGE_BACKSTOP.as_secs());
        assert!(generation_is_aged(&paths, &name, &clock));
    }

    #[test]
    fn restarting_the_backstop_counts_from_the_injected_clock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let name = planted(&paths);
        back_date(&paths.generation(&name), 2 * 86_400);
        let later = now() + 86_400;

        restart_backstop(&paths, &name, &AtClock(later));

        assert!(!generation_is_aged(&paths, &name, &AtClock(later + 60)));
        assert!(generation_is_aged(
            &paths,
            &name,
            &AtClock(later + AGE_BACKSTOP.as_secs())
        ));
    }

    #[test]
    fn a_backstop_that_cannot_be_restarted_is_warned() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let absent = gen_name();

        let log = logged(|| restart_backstop(&paths, &absent, &present()));

        assert!(log.contains("WARN"), "no warning: {log}");
        assert!(log.contains(&absent), "warning names no generation: {log}");
    }

    #[test]
    fn a_restarted_backstop_is_not_warned() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let name = planted(&paths);

        let log = logged(|| restart_backstop(&paths, &name, &present()));

        assert!(log.is_empty(), "unexpected log: {log}");
    }

    #[test]
    fn an_aged_orphan_is_reclaimed_with_its_sidecars() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let name = planted(&paths);
        let sidecars = sidecars_of(&paths, &name);
        for sidecar in &sidecars {
            fs::write(sidecar, b"").expect("sidecar");
        }

        reap_for_y_at(
            dir.path(),
            well_aged(),
            Pins::default(),
            UnobservedRemovals,
        )
        .expect("reap");

        assert!(!paths.generation(&name).exists());
        for sidecar in &sidecars {
            assert!(!sidecar.exists(), "{} survived", sidecar.display());
        }
    }
}

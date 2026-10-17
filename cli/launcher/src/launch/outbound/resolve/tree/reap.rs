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
use super::lease::{Liveness, LivenessProbe};

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

/// Reclaims residue under `trees/` for a caller holding a tree lock.
pub struct Reaper<'a> {
    paths: &'a TreePaths,
    clock: &'a dyn Clock,
    liveness: &'a dyn LivenessProbe,
    pins: &'a dyn PinnedDigests,
    observer: &'a dyn RemovalObserver,
}

impl<'a> Reaper<'a> {
    #[must_use]
    pub const fn new(
        paths: &'a TreePaths,
        clock: &'a dyn Clock,
        liveness: &'a dyn LivenessProbe,
        pins: &'a dyn PinnedDigests,
    ) -> Self {
        Self {
            paths,
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
    /// newer launcher's, on a platform this one does not know. Every other
    /// temp residue goes; a fully-materialised generation goes only when no
    /// pointer names it, no live lease is held on it, and it is older than
    /// the backstop.
    ///
    /// # Errors
    ///
    /// [`TreeError::Pointer`] if `trees/` cannot be read.
    pub fn reap_orphans(&self) -> Result<Reclaimed, TreeError> {
        let names = entry_names(self.paths.root())?;
        let referenced = referenced_generations(self.paths.root());
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
        for entry in entries {
            if !is_resumable(entry, owner, self.pins) {
                self.remove(&self.paths.root().join(entry), reclaimed);
            }
        }
    }

    fn reap_generations(
        &self,
        names: &[String],
        referenced: &BTreeSet<String>,
        reclaimed: &mut Reclaimed,
    ) {
        for name in names.iter().filter(|name| is_generation(name)) {
            if is_reclaimable_generation(
                self.paths,
                name,
                referenced,
                self.clock,
                self.liveness,
            ) {
                self.remove(&self.paths.generation(name), reclaimed);
                for sidecar in sidecars_of(self.paths, name) {
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
    use super::super::lease::{take_single_flight, FlockProbe, Liveness};
    use super::super::testing::{logged, within, BLOCKING_BOUND};
    use super::{
        generation_is_aged, orphan_is_reclaimable, restart_backstop,
        sidecars_of, superseded_disposition, unclaimed_is_reclaimable,
        PinnedDigests, Reaper, Reclaimed, RemovalObserver,
        SupersededDisposition, UnobservedRemovals, AGE_BACKSTOP,
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
        let _caller_holds =
            take_single_flight(&paths.single_flight_lock_of(&y()))
                .expect("Y's lock");
        let locks_before = lock_files(&paths);
        let root = root.to_owned();
        let outcome = within(BLOCKING_BOUND, "the reap", move || {
            let paths = TreePaths::under(&root);
            Reaper::new(&paths, &clock, &FlockProbe, &pins)
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
        let generation = generation_name(
            owner.name(),
            owner.platform(),
            DIGEST,
            LAYOUT_VERSION,
            SUFFIX,
        );
        let tree = paths.temp_generation(&generation);
        fs::create_dir(&tree).expect("temp tree");
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

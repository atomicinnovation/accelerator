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

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::launch::core::tree::{Clock, TreeError};

use super::layout::{TreePaths, LAYOUT_VERSION};
use super::lease::{probe_liveness, Liveness};

/// Generations younger than this with no live lease are spared, so a generation
/// briefly between its rename and its pointer write is never mistaken for
/// crash residue. Sized well beyond a fetch-plus-extract.
const AGE_BACKSTOP: Duration = Duration::from_secs(60 * 60);

/// Reclaim orphan temp residue and unreferenced generations under `trees/`.
///
/// A temp archive for the launcher's current expected digest `keep_digest` is
/// spared, so a resumable download is not reclaimed before the next run can
/// continue it. Every other temp residue goes; a fully-materialised generation
/// goes only when no pointer names it, no live lease is held on it, and it is
/// older than the backstop.
///
/// # Errors
///
/// [`TreeError::Pointer`] if `trees/` cannot be read.
pub fn reap_orphans(
    paths: &TreePaths,
    clock: &dyn Clock,
    keep_digests: &BTreeSet<String>,
) -> Result<Reclaimed, TreeError> {
    let root = paths.root();
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Reclaimed::default())
        }
        Err(error) => {
            return Err(TreeError::Pointer {
                detail: format!("cannot read the trees directory: {error}"),
            })
        }
    };

    let referenced = referenced_generations(root);
    let mut reclaimed = Reclaimed::default();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let path = entry.path();
        if let Some(residue) = classify(name, keep_digests) {
            match residue {
                Residue::TempArchive | Residue::TempTree => {
                    remove(&path, &mut reclaimed);
                }
                Residue::Generation => {
                    if is_reclaimable_generation(
                        paths, name, &path, &referenced, clock,
                    ) {
                        remove(&path, &mut reclaimed);
                    }
                }
            }
        }
    }
    Ok(reclaimed)
}

/// How much a reap reclaimed, for the caller to report.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reclaimed {
    pub entries: u64,
}

enum Residue {
    TempArchive,
    TempTree,
    Generation,
}

fn classify(name: &str, keep_digests: &BTreeSet<String>) -> Option<Residue> {
    if let Some(rest) = name.strip_prefix(".tmp-") {
        if rest.ends_with(".archive") {
            // A partial archive for a digest the launcher still wants is a
            // resumable download, not an orphan.
            if keep_digests.iter().any(|digest| rest.contains(digest.as_str()))
            {
                return None;
            }
            return Some(Residue::TempArchive);
        }
        if rest.ends_with(".lock") {
            return None;
        }
        return Some(Residue::TempTree);
    }
    // A generation directory: the fixed grammar, no sidecar suffix.
    if name.contains(&format!("-{LAYOUT_VERSION}-")) && !has_sidecar_suffix(name)
    {
        return Some(Residue::Generation);
    }
    None
}

fn has_sidecar_suffix(name: &str) -> bool {
    [".ref", ".sealed", ".sealed.sig", ".lease"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// The generation directory names any pointer currently references.
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
    path: &Path,
    referenced: &BTreeSet<String>,
    clock: &dyn Clock,
) -> bool {
    if referenced.contains(name) {
        return false;
    }
    if probe_liveness(&paths.lease(name)) == Liveness::Held {
        return false;
    }
    older_than_backstop(path, clock)
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

fn remove(path: &Path, reclaimed: &mut Reclaimed) {
    let outcome = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    if outcome.is_ok() {
        reclaimed.entries += 1;
    }
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
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    use crate::launch::core::tree::Clock;

    use super::super::layout::{generation_name, TreePaths, LAYOUT_VERSION};
    use super::{reap_orphans, AGE_BACKSTOP};

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

    fn gen_name() -> String {
        generation_name(
            "browser",
            "linux-x64",
            DIGEST,
            LAYOUT_VERSION,
            "0123456789abcdef",
        )
    }

    #[test]
    fn a_temp_archive_and_a_temp_tree_are_reclaimed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let archive =
            paths.temp_archive("browser", "linux-x64", &"f".repeat(64));
        fs::write(&archive, b"partial").expect("archive");
        let temp_tree = paths.temp_generation(&gen_name());
        fs::create_dir(&temp_tree).expect("temp tree");

        let reclaimed =
            reap_orphans(&paths, &present(), &BTreeSet::new()).expect("reap");
        assert_eq!(reclaimed.entries, 2);
        assert!(!archive.exists());
        assert!(!temp_tree.exists());
    }

    #[test]
    fn a_partial_archive_for_the_wanted_digest_is_spared() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let archive = paths.temp_archive("browser", "linux-x64", DIGEST);
        fs::write(&archive, b"resumable").expect("archive");

        let mut keep = BTreeSet::new();
        keep.insert(DIGEST.to_owned());
        let reclaimed =
            reap_orphans(&paths, &present(), &keep).expect("reap");
        assert_eq!(reclaimed.entries, 0);
        assert!(archive.exists(), "a resumable partial was reclaimed");
    }

    #[test]
    fn an_unreferenced_aged_generation_is_reclaimed_and_a_referenced_one_spared()
    {
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
        fs::write(
            paths.pointer("driver", "linux-x64", DIGEST),
            &referenced,
        )
        .expect("pointer");

        // The clock reads past the backstop, so both are old — only the
        // reference spares one of them.
        reap_orphans(&paths, &well_aged(), &BTreeSet::new()).expect("reap");
        assert!(!orphan_dir.exists(), "the orphan survived");
        assert!(referenced_dir.exists(), "a referenced generation was reaped");
    }

    #[test]
    fn a_young_unreferenced_generation_is_spared_by_the_backstop() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let fresh = paths.generation(&gen_name());
        fs::create_dir(&fresh).expect("fresh");

        // The clock reads the present, so a just-renamed generation — between
        // its rename and its pointer write — is inside the backstop window.
        let reclaimed =
            reap_orphans(&paths, &present(), &BTreeSet::new()).expect("reap");
        assert_eq!(reclaimed.entries, 0);
        assert!(fresh.exists(), "a freshly renamed generation was reaped");
    }
}

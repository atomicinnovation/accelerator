//! Retention claims: how an install declares "I still use this digest".
//!
//! A compiled-in digest is private to the binary that carries it, so `prune`
//! cannot infer which digests a sibling install still wants by reading its
//! binary — the only ways to learn a sibling's value are executing an unverified
//! binary or scanning it for strings, and the failure mode of getting it wrong
//! is deleting a live install's ~294MB. So each launcher writes its claim down
//! instead: `trees/claims/<digest>.<launcher-id>`, refreshed on every resolve,
//! and `prune` reads only that directory.

use std::path::Path;
use std::time::Duration;

use crate::launch::core::tree::Clock;

use super::layout::TreePaths;

/// A digest with no claim refreshed inside this window is reclaimable. Sized to
/// a ~14-day orphan sweep, so a merely-idle install is not evicted.
pub const CLAIM_WINDOW: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// Below this fraction of the window, a fresh claim is not rewritten, so a crawl
/// of 100-200 invocations does at most one claim write rather than one each.
const REFRESH_FRACTION: u64 = 10;

/// Refresh this install's claim on `digest`, best-effort.
///
/// Never fails a resolution: a populated cache root may be read-only on a warm
/// start, so `EROFS`/`EACCES` are ignored. The write is skipped when a claim
/// already exists with a recent mtime, which is what turns per-invocation writes
/// into at most one per crawl.
pub fn refresh(
    paths: &TreePaths,
    digest: &str,
    launcher_id: &str,
    clock: &dyn Clock,
) {
    let claim = paths.claims().join(format!("{digest}.{launcher_id}"));
    if is_fresh_enough(&claim, clock) {
        return;
    }
    if std::fs::create_dir_all(paths.claims()).is_err() {
        return;
    }
    write_private(&claim);
}

/// Whether any valid claim on `digest` was refreshed within `window`.
///
/// A claim that is a symlink, or not owned by the effective uid, is ignored
/// rather than trusted — otherwise a planted claim would pin a generation
/// forever.
#[must_use]
pub fn claimed_within(
    paths: &TreePaths,
    digest: &str,
    window: Duration,
    clock: &dyn Clock,
) -> bool {
    let prefix = format!("{digest}.");
    let Ok(entries) = std::fs::read_dir(paths.claims()) else {
        return false;
    };
    let now = clock.now_seconds();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with(&prefix) {
            continue;
        }
        let path = entry.path();
        if !is_valid_claim(&path) {
            continue;
        }
        if let Some(modified) = modified_seconds(&path) {
            if now.saturating_sub(modified) < window.as_secs() {
                return true;
            }
        }
    }
    false
}

/// Remove every claim on `digest` — used when the digest's generation is
/// reclaimed, so a stale pointer's claims do not linger.
pub fn drop_all(paths: &TreePaths, digest: &str) {
    let prefix = format!("{digest}.");
    let Ok(entries) = std::fs::read_dir(paths.claims()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_str().is_some_and(|name| name.starts_with(&prefix)) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn is_fresh_enough(claim: &Path, clock: &dyn Clock) -> bool {
    let Some(modified) = modified_seconds(claim) else {
        return false;
    };
    clock.now_seconds().saturating_sub(modified)
        < CLAIM_WINDOW.as_secs() / REFRESH_FRACTION
}

#[cfg(unix)]
fn is_valid_claim(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    !metadata.file_type().is_symlink()
        && metadata.uid() == rustix::process::geteuid().as_raw()
}

#[cfg(not(unix))]
fn is_valid_claim(_path: &Path) -> bool {
    false
}

fn modified_seconds(path: &Path) -> Option<u64> {
    std::fs::symlink_metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

#[cfg(unix)]
fn write_private(path: &Path) {
    use std::os::unix::fs::OpenOptionsExt as _;
    let _ = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path);
}

#[cfg(not(unix))]
fn write_private(path: &Path) {
    let _ = std::fs::File::create(path);
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::time::Duration;

    use crate::launch::core::tree::Clock;

    use super::super::layout::TreePaths;
    use super::{claimed_within, refresh, CLAIM_WINDOW};

    struct AtClock(u64);
    impl Clock for AtClock {
        fn now_seconds(&self) -> u64 {
            self.0
        }
        fn sleep_poll_interval(&self) {}
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after epoch")
            .as_secs()
    }

    const DIGEST: &str =
        "abc0000000000000000000000000000000000000000000000000000000000123";

    fn trees(root: &std::path::Path) -> TreePaths {
        let paths = TreePaths::under(root);
        std::fs::create_dir_all(paths.root()).expect("trees");
        paths
    }

    #[test]
    fn a_fresh_claim_reads_as_recent_and_a_stale_one_does_not() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let present = AtClock(now());
        refresh(&paths, DIGEST, "install-a", &present);
        assert!(claimed_within(&paths, DIGEST, CLAIM_WINDOW, &present));

        // A clock a fortnight on sees the same claim as stale.
        let later = AtClock(now() + CLAIM_WINDOW.as_secs() + 1);
        assert!(!claimed_within(&paths, DIGEST, CLAIM_WINDOW, &later));
    }

    #[test]
    fn two_installs_write_distinct_claim_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let clock = AtClock(now());
        refresh(&paths, DIGEST, "install-a", &clock);
        refresh(&paths, DIGEST, "install-b", &clock);
        let count = std::fs::read_dir(paths.claims()).expect("claims").count();
        assert_eq!(count, 2, "each install writes its own claim");
    }

    #[cfg(unix)]
    #[test]
    fn a_fresh_claim_is_not_rewritten_but_a_stale_one_is() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let present = AtClock(now());
        refresh(&paths, DIGEST, "install-a", &present);

        // A sentinel survives only if the second refresh skips the rewrite;
        // `write_private` truncates, so a rewrite would empty it.
        let claim = paths.claims().join(format!("{DIGEST}.install-a"));
        std::fs::write(&claim, b"sentinel").expect("sentinel");

        refresh(&paths, DIGEST, "install-a", &present);
        assert_eq!(
            std::fs::read(&claim).expect("read"),
            b"sentinel",
            "a fresh claim must not be rewritten"
        );

        // A clock past the refresh fraction sees the claim as due, and rewrites.
        let stale = AtClock(
            now() + CLAIM_WINDOW.as_secs() / super::REFRESH_FRACTION + 1,
        );
        refresh(&paths, DIGEST, "install-a", &stale);
        assert!(
            std::fs::read(&claim).expect("read").is_empty(),
            "a claim past the refresh fraction must be rewritten"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_claim_is_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        std::fs::create_dir_all(paths.claims()).expect("claims dir");
        // A claim that is a symlink, not a real file, must not count.
        std::os::unix::fs::symlink(
            "/etc/hostname",
            paths.claims().join(format!("{DIGEST}.planted")),
        )
        .expect("symlink");
        let clock = AtClock(now());
        assert!(!claimed_within(
            &paths,
            DIGEST,
            Duration::from_secs(60),
            &clock
        ));
    }
}

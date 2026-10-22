//! Retention claims: how an install declares "I still use this digest".
//!
//! A compiled-in digest is private to the binary that carries it, so `prune`
//! cannot infer which digests a sibling install still wants by reading its
//! binary — the only ways to learn a sibling's value are executing an unverified
//! binary or scanning it for strings, and the failure mode of getting it wrong
//! is deleting a live install's ~294MB. So each launcher writes its claim down
//! instead: `trees/claims/<digest>.<launcher-id>`, refreshed on every resolve,
//! and `prune` reads only that directory.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::launch::core::tree::Clock;

use super::super::filesystem::{
    CacheFilesystem, EntryMetadata, FileKind, SystemFilesystem,
};
use super::super::ownership::{
    is_trusted_directory, is_trusted_directory_entry, Ownership,
    ProcessOwnership,
};
pub use super::super::retention_windows::CLAIM_WINDOW;
use super::super::retention_windows::REFRESH_FRACTION;
use super::layout::{is_wellformed_digest, TreePaths};

const LAUNCHER_ID_LENGTH: usize = 16;

/// Refresh this install's claim on `digest`, best-effort.
///
/// Never fails a resolution: a populated cache root may be read-only on a warm
/// start, so `EROFS`/`EACCES` are ignored. The write is skipped when a claim
/// already exists with a recent mtime, which is what turns per-invocation writes
/// into at most one per crawl. Nothing is written through a symlink, nor into a
/// claims directory another user could rename into.
pub fn refresh(
    paths: &TreePaths,
    digest: &str,
    launcher_id: &str,
    clock: &dyn Clock,
) {
    let claims = paths.claims();
    let claim = claims.join(format!("{digest}.{launcher_id}"));
    if is_fresh_enough(&claim, clock) {
        return;
    }
    match SystemFilesystem.create_private_dir(&claims) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return,
    }
    if !is_trusted_directory(
        &claims,
        &SystemFilesystem,
        &ProcessOwnership::default(),
    ) {
        return;
    }
    write_private(&claim, clock);
}

/// One tree claim whose name parses as `<digest>.<launcher-id>`.
#[derive(Debug, Clone)]
pub struct ListedTreeClaim {
    pub digest: String,
    pub path: PathBuf,
    /// Only a claim whose launcher id is the launcher's own sixteen-hex shape
    /// is ever swept; any other id still protects its digest.
    pub sweepable: bool,
    pub entry: EntryMetadata,
}

impl ListedTreeClaim {
    /// An unreadable or future mtime counts as fresh, so it can only protect.
    #[must_use]
    pub fn protects_within(
        &self,
        window: Duration,
        now: u64,
        uid: u32,
    ) -> bool {
        is_valid_claim_entry(&self.entry, uid)
            && self.entry.modified_seconds.is_none_or(|modified| {
                now.saturating_sub(modified) < window.as_secs()
            })
    }
}

/// Whether `entry` is a claim this prune may sweep as stale: a valid claim
/// last refreshed at least `window` ago.
#[must_use]
pub fn is_stale_claim_entry(
    entry: &EntryMetadata,
    window: Duration,
    now: u64,
    uid: u32,
) -> bool {
    is_valid_claim_entry(entry, uid)
        && entry.modified_seconds.is_some_and(|modified| {
            now.saturating_sub(modified) >= window.as_secs()
        })
}

const fn is_valid_claim_entry(entry: &EntryMetadata, uid: u32) -> bool {
    matches!(entry.kind, FileKind::File) && entry.owner == uid
}

/// The tree claims read once, so every keep decision shares one view.
#[derive(Debug, Default)]
pub struct TreeClaimIndex {
    claims: Vec<ListedTreeClaim>,
}

impl TreeClaimIndex {
    #[must_use]
    pub fn claims(&self) -> &[ListedTreeClaim] {
        &self.claims
    }

    pub fn on<'a>(
        &'a self,
        digest: &'a str,
    ) -> impl Iterator<Item = &'a ListedTreeClaim> {
        self.claims
            .iter()
            .filter(move |claim| claim.digest == digest)
    }

    #[must_use]
    pub fn protects(
        &self,
        digest: &str,
        window: Duration,
        now: u64,
        uid: u32,
    ) -> bool {
        self.on(digest)
            .any(|claim| claim.protects_within(window, now, uid))
    }
}

#[derive(Debug)]
pub enum TreeClaimsListing {
    Read(TreeClaimIndex),
    /// Distinct from an empty listing, so a prune never reads "no claims"
    /// into a directory it could not read.
    Unreadable(io::ErrorKind),
    /// A directory another user could rename claims into.
    Untrusted,
}

/// Every tree claim whose name parses; a missing `trees/claims/` is an empty
/// index.
pub fn list(
    paths: &TreePaths,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
) -> TreeClaimsListing {
    let directory = paths.claims();
    match filesystem.entry(&directory) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return TreeClaimsListing::Read(TreeClaimIndex::default());
        }
        Err(error) => return TreeClaimsListing::Unreadable(error.kind()),
        Ok(entry)
            if !is_trusted_directory_entry(
                &directory, &entry, filesystem, ownership,
            ) =>
        {
            return TreeClaimsListing::Untrusted;
        }
        Ok(_) => {}
    }
    let names = match filesystem.list(&directory) {
        Ok(names) => names,
        Err(error) => return TreeClaimsListing::Unreadable(error.kind()),
    };
    let claims = names
        .iter()
        .filter_map(|name| name.to_str())
        .filter_map(|name| {
            let (digest, launcher_id) = name.split_once('.')?;
            if !is_wellformed_digest(digest) || launcher_id.is_empty() {
                return None;
            }
            let path = directory.join(name);
            let entry = filesystem.entry(&path).ok()?;
            Some(ListedTreeClaim {
                digest: digest.to_owned(),
                sweepable: is_launcher_id(launcher_id),
                entry,
                path,
            })
        })
        .collect();
    TreeClaimsListing::Read(TreeClaimIndex { claims })
}

fn is_launcher_id(candidate: &str) -> bool {
    candidate.len() == LAUNCHER_ID_LENGTH
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_fresh_enough(claim: &Path, clock: &dyn Clock) -> bool {
    let Some(modified) = modified_seconds(claim) else {
        return false;
    };
    clock.now_seconds().saturating_sub(modified)
        < CLAIM_WINDOW.as_secs() / REFRESH_FRACTION
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

/// Sets the mtime explicitly rather than relying on `O_TRUNC` of an empty
/// claim to advance it.
#[cfg(unix)]
fn write_private(path: &Path, clock: &dyn Clock) {
    use std::os::unix::fs::OpenOptionsExt as _;
    let Ok(file) = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits().cast_signed())
        .open(path)
    else {
        return;
    };
    let _ = file.set_modified(
        std::time::UNIX_EPOCH + Duration::from_secs(clock.now_seconds()),
    );
}

#[cfg(not(unix))]
fn write_private(path: &Path, _clock: &dyn Clock) {
    let _ = std::fs::File::create(path);
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::time::Duration;

    use crate::launch::core::tree::Clock;

    use super::super::super::filesystem::SystemFilesystem;
    use super::super::super::ownership::{Ownership as _, ProcessOwnership};
    use super::super::layout::TreePaths;
    use super::{list, refresh, TreeClaimsListing, CLAIM_WINDOW};

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

    fn protected(paths: &TreePaths, window: Duration, clock: &AtClock) -> bool {
        let ownership = ProcessOwnership::default();
        let TreeClaimsListing::Read(claims) =
            list(paths, &SystemFilesystem, &ownership)
        else {
            panic!("the claims were not read");
        };
        claims.protects(
            DIGEST,
            window,
            clock.now_seconds(),
            ownership.effective_uid(),
        )
    }

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
        assert!(protected(&paths, CLAIM_WINDOW, &present));

        let later = AtClock(now() + CLAIM_WINDOW.as_secs() + 1);
        assert!(!protected(&paths, CLAIM_WINDOW, &later));
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
        assert!(!protected(&paths, Duration::from_secs(60), &AtClock(now())));
    }

    fn back_date(path: &std::path::Path, seconds: u64) {
        std::fs::File::options()
            .write(true)
            .open(path)
            .expect("open claim")
            .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(seconds))
            .expect("back-date claim");
    }

    fn modified(path: &std::path::Path) -> u64 {
        std::fs::metadata(path)
            .expect("claim metadata")
            .modified()
            .expect("mtime")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after epoch")
            .as_secs()
    }

    fn due() -> AtClock {
        AtClock(now() + CLAIM_WINDOW.as_secs() / super::REFRESH_FRACTION + 60)
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_tree_claim_leaves_its_target_unmodified() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        std::fs::create_dir_all(paths.claims()).expect("claims dir");
        let target = dir.path().join("target");
        std::fs::write(&target, b"untouched").expect("target");
        std::os::unix::fs::symlink(
            &target,
            paths.claims().join(format!("{DIGEST}.install-a")),
        )
        .expect("symlink");

        refresh(&paths, DIGEST, "install-a", &due());

        assert_eq!(std::fs::read(&target).expect("target"), b"untouched");
    }

    #[cfg(unix)]
    #[test]
    fn refreshing_an_empty_back_dated_tree_claim_moves_its_mtime_to_now() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        std::fs::create_dir_all(paths.claims()).expect("claims dir");
        let claim = paths.claims().join(format!("{DIGEST}.install-a"));
        std::fs::write(&claim, b"").expect("empty claim");
        back_date(&claim, now());
        let clock = due();

        refresh(&paths, DIGEST, "install-a", &clock);

        assert_eq!(modified(&claim), clock.now_seconds());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_tree_claims_directory_is_not_written_through() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees(dir.path());
        let elsewhere = dir.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).expect("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, paths.claims())
            .expect("symlink");

        refresh(&paths, DIGEST, "install-a", &AtClock(now()));

        assert_eq!(
            std::fs::read_dir(&elsewhere).expect("elsewhere").count(),
            0
        );
    }
}

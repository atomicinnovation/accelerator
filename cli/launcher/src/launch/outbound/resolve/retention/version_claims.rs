//! Version claims: how a launcher declares "my version still uses this root".
//!
//! Each launcher start in a relocated root refreshes
//! `claims/<version>.<launcher-id>`, so a prune can tell which versions'
//! entries are still wanted by reading only that directory.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::launch::core::tree::Clock;

use super::super::filesystem::{CacheFilesystem, EntryMetadata, FileKind};
use super::super::ownership::{
    is_trusted_directory, is_trusted_directory_entry, Ownership,
};
use super::super::retention_windows::{CLAIM_WINDOW, REFRESH_FRACTION};

pub const VERSION_CLAIMS_DIR: &str = "claims";

const LAUNCHER_ID_LENGTH: usize = 16;

pub struct VersionClaim<'a> {
    pub version: &'a str,
    pub launcher_id: &'a str,
}

impl VersionClaim<'_> {
    fn name(&self) -> String {
        format!("{}.{}", self.version, self.launcher_id)
    }
}

/// Refresh this launcher's claim on its version, best-effort.
///
/// A valid claim younger than a tenth of the window is left alone, so a crawl
/// writes at most once. A failed write is only logged at debug, because a
/// read-only root is a supported case.
pub fn refresh(
    root: &Path,
    claim: &VersionClaim<'_>,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
    clock: &dyn Clock,
) {
    let claims = root.join(VERSION_CLAIMS_DIR);
    let path = claims.join(claim.name());
    let now = clock.now_seconds();
    let young = filesystem.entry(&path).is_ok_and(|entry| {
        is_valid(&entry, ownership.effective_uid())
            && age(&entry, now).is_some_and(|age| {
                age < CLAIM_WINDOW.as_secs() / REFRESH_FRACTION
            })
    });
    if young {
        return;
    }
    if let Err(error) = write_claim(&claims, &path, filesystem, ownership, now)
    {
        tracing::debug!(
            path = %path.display(),
            %error,
            "could not refresh the version claim"
        );
    }
}

fn write_claim(
    claims: &Path,
    path: &Path,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
    now: u64,
) -> io::Result<()> {
    match filesystem.create_private_dir(claims) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => {
            return Err(error);
        }
        _ => {}
    }
    if !is_trusted_directory(claims, filesystem, ownership) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the claims directory is not this user's alone",
        ));
    }
    filesystem.write_private(path, b"")?;
    filesystem.set_modified(path, now)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedClaim {
    pub version: String,
    pub launcher_id: String,
    pub path: PathBuf,
    /// A regular file owned by the effective uid; nothing else is trusted to
    /// protect a version.
    pub valid: bool,
    pub modified_seconds: Option<u64>,
}

#[derive(Debug)]
pub enum ClaimsListing {
    Read(Vec<ListedClaim>),
    /// Distinct from an empty listing, so a prune never reads "no claims"
    /// into a directory it could not read.
    Unreadable(io::ErrorKind),
}

/// Every claim in the root whose name parses.
pub fn list(
    root: &Path,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
) -> ClaimsListing {
    let claims = root.join(VERSION_CLAIMS_DIR);
    match filesystem.entry(&claims) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return ClaimsListing::Read(Vec::new());
        }
        Err(error) => return ClaimsListing::Unreadable(error.kind()),
        Ok(entry)
            if !is_trusted_directory_entry(
                &claims, &entry, filesystem, ownership,
            ) =>
        {
            return ClaimsListing::Unreadable(io::ErrorKind::PermissionDenied);
        }
        Ok(_) => {}
    }
    let names = match filesystem.list(&claims) {
        Ok(names) => names,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return ClaimsListing::Unreadable(error.kind()),
    };
    let uid = ownership.effective_uid();
    let listed = names
        .iter()
        .filter_map(|name| name.to_str())
        .filter_map(|name| {
            let (version, launcher_id) = parse_claim_name(name)?;
            let path = claims.join(name);
            let entry = filesystem.entry(&path).ok()?;
            Some(ListedClaim {
                version: version.to_owned(),
                launcher_id: launcher_id.to_owned(),
                valid: is_valid(&entry, uid),
                modified_seconds: entry.modified_seconds,
                path,
            })
        })
        .collect();
    ClaimsListing::Read(listed)
}

/// The versions some valid claim protects.
#[derive(Debug, Default)]
pub struct FreshVersions {
    versions: BTreeSet<String>,
}

impl FreshVersions {
    /// An unreadable or future mtime counts as fresh, so it can only protect.
    #[must_use]
    pub fn within(
        claims: &[ListedClaim],
        window: Duration,
        clock: &dyn Clock,
    ) -> Self {
        let now = clock.now_seconds();
        let versions = claims
            .iter()
            .filter(|claim| claim.valid)
            .filter(|claim| {
                claim.modified_seconds.is_none_or(|modified| {
                    now.saturating_sub(modified) < window.as_secs()
                })
            })
            .map(|claim| claim.version.clone())
            .collect();
        Self { versions }
    }

    /// The same versions and `version` besides, as the running version is
    /// protected whatever its claims say.
    #[must_use]
    pub fn including(mut self, version: &str) -> Self {
        self.versions.insert(version.to_owned());
        self
    }

    #[must_use]
    pub fn contains(&self, version: &str) -> bool {
        self.versions.contains(version)
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.versions.iter().map(String::as_str)
    }
}

/// Split a claim name at its last `.` when exactly sixteen lowercase hex
/// characters follow, so a version's own dots survive.
#[must_use]
pub fn parse_claim_name(name: &str) -> Option<(&str, &str)> {
    let (version, launcher_id) = name.rsplit_once('.')?;
    let is_launcher_id = launcher_id.len() == LAUNCHER_ID_LENGTH
        && launcher_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    (is_launcher_id && !version.is_empty()).then_some((version, launcher_id))
}

const fn is_valid(entry: &EntryMetadata, uid: u32) -> bool {
    matches!(entry.kind, FileKind::File) && entry.owner == uid
}

/// A future mtime is age zero.
fn age(entry: &EntryMetadata, now: u64) -> Option<u64> {
    entry
        .modified_seconds
        .map(|modified| now.saturating_sub(modified))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use crate::launch::core::tree::Clock;

    use super::super::super::filesystem::testing::ReportingFilesystem;
    use super::super::super::filesystem::{
        CacheFilesystem, EntryMetadata, SystemFilesystem,
    };
    use super::super::super::ownership::{Ownership, ProcessOwnership};
    use super::super::super::retention_windows::{
        CLAIM_WINDOW, REFRESH_FRACTION,
    };
    use super::{
        list, parse_claim_name, refresh, ClaimsListing, FreshVersions,
        VersionClaim, VERSION_CLAIMS_DIR,
    };

    const VERSION: &str = "1.24.0-pre.83";
    const LAUNCHER_ID: &str = "0123456789abcdef";

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

    const REFRESH_AGE: u64 = CLAIM_WINDOW.as_secs() / REFRESH_FRACTION;

    struct Root {
        temp: TempDir,
    }

    impl Root {
        fn new() -> Self {
            Self {
                temp: tempfile::tempdir().expect("tempdir"),
            }
        }

        fn path(&self) -> &Path {
            self.temp.path()
        }

        fn claims(&self) -> PathBuf {
            self.path().join(VERSION_CLAIMS_DIR)
        }

        fn claim(&self) -> PathBuf {
            self.claims().join(format!("{VERSION}.{LAUNCHER_ID}"))
        }

        fn seed_claim(&self, bytes: &[u8], modified: u64) {
            std::fs::create_dir_all(self.claims()).expect("claims dir");
            std::fs::write(self.claim(), bytes).expect("claim");
            SystemFilesystem
                .set_modified(&self.claim(), modified)
                .expect("claim mtime");
        }

        fn refresh_with(
            &self,
            filesystem: &dyn CacheFilesystem,
            clock: &dyn Clock,
        ) {
            refresh(
                self.path(),
                &claim(),
                filesystem,
                &ProcessOwnership::default(),
                clock,
            );
        }

        fn refresh_at(&self, seconds: u64) {
            self.refresh_with(&SystemFilesystem, &AtClock(seconds));
        }
    }

    const fn claim() -> VersionClaim<'static> {
        VersionClaim {
            version: VERSION,
            launcher_id: LAUNCHER_ID,
        }
    }

    fn modified(path: &Path) -> Option<u64> {
        SystemFilesystem
            .entry(path)
            .expect("claim metadata")
            .modified_seconds
    }

    fn real(path: &Path) -> EntryMetadata {
        SystemFilesystem.entry(path).expect("metadata")
    }

    fn foreign(path: &Path) -> EntryMetadata {
        EntryMetadata {
            owner: ProcessOwnership::default().effective_uid() + 1,
            ..real(path)
        }
    }

    fn listed(
        root: &Root,
        filesystem: &dyn CacheFilesystem,
    ) -> Vec<super::ListedClaim> {
        match list(root.path(), filesystem, &ProcessOwnership::default()) {
            ClaimsListing::Read(claims) => claims,
            ClaimsListing::Unreadable(kind) => panic!("unreadable: {kind:?}"),
        }
    }

    #[test]
    fn a_missing_claim_is_written() {
        let root = Root::new();
        let at = now();

        root.refresh_at(at);

        assert_eq!(modified(&root.claim()), Some(at));
    }

    #[test]
    fn a_claim_older_than_a_tenth_of_the_window_is_rewritten() {
        let root = Root::new();
        root.seed_claim(b"sentinel", now());
        let at = now() + REFRESH_AGE + 60;

        root.refresh_at(at);

        assert_eq!(modified(&root.claim()), Some(at));
        assert!(std::fs::read(root.claim()).expect("claim").is_empty());
    }

    #[test]
    fn a_claim_exactly_a_tenth_of_the_window_old_is_rewritten() {
        let root = Root::new();
        let seeded = now();
        root.seed_claim(b"sentinel", seeded);

        root.refresh_at(seeded + REFRESH_AGE);

        assert_eq!(modified(&root.claim()), Some(seeded + REFRESH_AGE));
    }

    #[test]
    fn a_younger_claim_is_not_rewritten() {
        let root = Root::new();
        let seeded = now();
        root.seed_claim(b"sentinel", seeded);

        root.refresh_at(seeded + REFRESH_AGE - 1);

        assert_eq!(std::fs::read(root.claim()).expect("claim"), b"sentinel");
        assert_eq!(modified(&root.claim()), Some(seeded));
    }

    #[test]
    fn refreshing_an_empty_back_dated_claim_moves_its_mtime_to_now() {
        let root = Root::new();
        let at = now();
        root.seed_claim(b"", at - CLAIM_WINDOW.as_secs());

        root.refresh_at(at);

        assert_eq!(modified(&root.claim()), Some(at));
    }

    #[cfg(unix)]
    #[test]
    fn a_read_only_root_never_fails_the_refresh() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = Root::new();
        std::fs::set_permissions(
            root.path(),
            std::fs::Permissions::from_mode(0o555),
        )
        .expect("read-only");

        root.refresh_at(now());

        std::fs::set_permissions(
            root.path(),
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("writable");
        assert!(!root.claims().exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_or_foreign_owned_claim_is_not_valid() {
        let root = Root::new();
        std::fs::create_dir_all(root.claims()).expect("claims dir");
        let target = root.path().join("target");
        std::fs::write(&target, b"").expect("target");
        let symlinked =
            root.claims().join(format!("{VERSION}.{}", "a".repeat(16)));
        std::os::unix::fs::symlink(&target, &symlinked).expect("symlink");
        root.seed_claim(b"", now());
        let filesystem = ReportingFilesystem::over_system()
            .reporting(&root.claim(), foreign(&root.claim()));

        let claims = listed(&root, &filesystem);

        assert_eq!(claims.len(), 2);
        assert!(claims.iter().all(|claim| !claim.valid), "{claims:?}");
        assert!(
            !FreshVersions::within(&claims, CLAIM_WINDOW, &AtClock(now()))
                .contains(VERSION)
        );
    }

    #[test]
    fn a_young_foreign_owned_claim_does_not_suppress_the_refresh() {
        let root = Root::new();
        let seeded = now();
        root.seed_claim(b"sentinel", seeded);
        let filesystem = ReportingFilesystem::over_system()
            .reporting(&root.claim(), foreign(&root.claim()));

        root.refresh_with(&filesystem, &AtClock(seeded + 1));

        assert!(std::fs::read(root.claim()).expect("claim").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_or_foreign_version_claims_directory_is_not_written_through()
    {
        let symlinked = Root::new();
        let elsewhere = symlinked.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).expect("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, symlinked.claims())
            .expect("symlink");
        symlinked.refresh_at(now());
        assert_eq!(
            std::fs::read_dir(&elsewhere).expect("elsewhere").count(),
            0
        );

        let foreign_owned = Root::new();
        std::fs::create_dir(foreign_owned.claims()).expect("claims dir");
        let filesystem = ReportingFilesystem::over_system().reporting(
            &foreign_owned.claims(),
            foreign(&foreign_owned.claims()),
        );
        foreign_owned.refresh_with(&filesystem, &AtClock(now()));
        assert!(!foreign_owned.claim().exists());
    }

    #[test]
    fn an_unknown_or_future_mtime_never_removes_or_unprotects() {
        let root = Root::new();
        let at = now();
        root.seed_claim(b"", at + CLAIM_WINDOW.as_secs());
        let future = listed(&root, &SystemFilesystem);
        let filesystem = ReportingFilesystem::over_system().reporting(
            &root.claim(),
            EntryMetadata {
                modified_seconds: None,
                ..real(&root.claim())
            },
        );
        let unknown = listed(&root, &filesystem);

        for claims in [future, unknown] {
            assert!(
                FreshVersions::within(&claims, CLAIM_WINDOW, &AtClock(at))
                    .contains(VERSION),
                "{claims:?}"
            );
        }
        assert!(root.claim().exists());
    }

    #[test]
    fn a_claim_outside_the_window_does_not_make_its_version_fresh() {
        let root = Root::new();
        let at = now();
        root.seed_claim(b"", at - CLAIM_WINDOW.as_secs());

        let claims = listed(&root, &SystemFilesystem);

        assert!(!FreshVersions::within(&claims, CLAIM_WINDOW, &AtClock(at))
            .contains(VERSION));
    }

    #[test]
    fn claim_names_keep_the_versions_own_dots() {
        assert_eq!(
            parse_claim_name("1.24.0-pre.83.0123456789abcdef"),
            Some(("1.24.0-pre.83", "0123456789abcdef"))
        );
    }

    #[test]
    fn malformed_launcher_id_suffixes_do_not_parse() {
        for name in [
            "1.24.0.0123456789abcde",
            "1.24.0.0123456789ABCDEF",
            "0123456789abcdef",
            ".0123456789abcdef",
            "1.24.0",
        ] {
            assert_eq!(parse_claim_name(name), None, "{name}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_claims_directory_is_reported_as_unreadable() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = Root::new();
        std::fs::create_dir(root.claims()).expect("claims dir");
        std::fs::set_permissions(
            root.claims(),
            std::fs::Permissions::from_mode(0o000),
        )
        .expect("unreadable");

        let listing =
            list(root.path(), &SystemFilesystem, &ProcessOwnership::default());

        std::fs::set_permissions(
            root.claims(),
            std::fs::Permissions::from_mode(0o700),
        )
        .expect("readable");
        assert!(
            matches!(
                listing,
                ClaimsListing::Unreadable(std::io::ErrorKind::PermissionDenied)
            ),
            "{listing:?}"
        );
    }

    #[test]
    fn a_claims_directory_owned_by_another_uid_is_unreadable() {
        let root = Root::new();
        root.seed_claim(b"", now());
        let filesystem = ReportingFilesystem::over_system()
            .reporting(&root.claims(), foreign(&root.claims()));

        let listing =
            list(root.path(), &filesystem, &ProcessOwnership::default());

        assert!(
            matches!(
                listing,
                ClaimsListing::Unreadable(std::io::ErrorKind::PermissionDenied)
            ),
            "{listing:?}"
        );
    }

    #[test]
    fn a_missing_claims_directory_lists_no_claims() {
        let root = Root::new();

        assert!(listed(&root, &SystemFilesystem).is_empty());
    }
}

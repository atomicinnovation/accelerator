//! Whether a cache root is its owner's alone, so a prune may remove by path
//! inside it without another user redirecting the removal.

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use super::super::filesystem::{
    AccessAcl, CacheFilesystem, EntryMetadata, FileKind,
};
use super::{Ownership, PrimaryGroup};

pub const TRUSTED_OWNERS_VAR: &str = "ACCELERATOR_CACHE_TRUSTED_OWNERS";

/// The overflow uid stands for every unmapped host account, and uid 99 for
/// every local user on a volume with ownership ignored, so neither is one
/// account anyone could vouch for.
const UNTRUSTABLE_UIDS: [u32; 2] = [65_534, 99];

const GROUP_WRITABLE: u32 = 0o020;
const WORLD_WRITABLE: u32 = 0o002;
const STICKY: u32 = 0o1000;

/// Uids an operator lists as owning an ancestor of the cache root, such as a
/// CI runner's workspace or a container's bind mount.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustedOwners {
    uids: BTreeSet<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedTrustedOwners;

impl TrustedOwners {
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Comma-separated decimal uids and nothing else.
    ///
    /// # Errors
    ///
    /// [`MalformedTrustedOwners`] for anything but that.
    pub fn parse(value: &str) -> Result<Self, MalformedTrustedOwners> {
        value
            .split(',')
            .map(|uid| {
                if uid.is_empty()
                    || !uid.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(MalformedTrustedOwners);
                }
                uid.parse().map_err(|_| MalformedTrustedOwners)
            })
            .collect::<Result<_, _>>()
            .map(|uids| Self { uids })
    }

    /// The listing in [`TRUSTED_OWNERS_VAR`]; a malformed value is ignored
    /// with a warning, so it can only refuse.
    #[must_use]
    pub fn from_env() -> Self {
        let Some(value) = std::env::var_os(TRUSTED_OWNERS_VAR) else {
            return Self::none();
        };
        value
            .to_str()
            .ok_or(MalformedTrustedOwners)
            .and_then(Self::parse)
            .unwrap_or_else(|MalformedTrustedOwners| {
                tracing::warn!(
                    "ignoring {TRUSTED_OWNERS_VAR}: expected comma-separated \
                     decimal uids"
                );
                Self::none()
            })
    }

    fn contains(&self, uid: u32) -> bool {
        self.uids.contains(&uid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsafeReason {
    Unexaminable(io::ErrorKind),
    NotADirectory,
    RootOwnedByAnother { owner: u32 },
    AncestorOwnedByAnother { owner: u32 },
    WritableBySharedGroup { group: String },
    WritableByUnresolvedGroup { gid: u32 },
    WorldWritableRoot,
    WorldWritableWithoutStickyBit,
    AclMayGrantWrite,
    OwnershipIgnored,
}

/// The path that failed the rule and why, with its remedy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsafeRoot {
    pub path: PathBuf,
    pub reason: UnsafeReason,
}

const PER_USER_REMEDY: &str = "set ACCELERATOR_CACHE_DIR to a per-user \
                               directory on a native filesystem";

impl fmt::Display for UnsafeRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.path.display();
        match &self.reason {
            UnsafeReason::Unexaminable(kind) => {
                write!(formatter, "{path} could not be examined ({kind})")
            }
            UnsafeReason::NotADirectory => {
                write!(formatter, "{path} is not a directory")
            }
            UnsafeReason::RootOwnedByAnother { owner } => write!(
                formatter,
                "{path} is owned by uid {owner}, not this user; \
                 {PER_USER_REMEDY}"
            ),
            UnsafeReason::AncestorOwnedByAnother { owner }
                if UNTRUSTABLE_UIDS.contains(owner) =>
            {
                write!(
                    formatter,
                    "{path} is owned by uid {owner}, which stands for many \
                     accounts or none; {PER_USER_REMEDY}"
                )
            }
            UnsafeReason::AncestorOwnedByAnother { owner } => write!(
                formatter,
                "{path} is owned by uid {owner}, who could redirect a \
                 prune; if that account is trusted, set \
                 {TRUSTED_OWNERS_VAR}={owner}"
            ),
            UnsafeReason::WritableBySharedGroup { group } => write!(
                formatter,
                "{path} is group-writable by the shared group {group}; run \
                 chmod g-w {path}"
            ),
            UnsafeReason::WritableByUnresolvedGroup { gid } => write!(
                formatter,
                "{path} is group-writable and could not resolve group {gid}; \
                 run chmod g-w {path}"
            ),
            UnsafeReason::WorldWritableRoot => write!(
                formatter,
                "{path} is world-writable; run chmod 700 {path}, or \
                 {PER_USER_REMEDY}"
            ),
            UnsafeReason::WorldWritableWithoutStickyBit => write!(
                formatter,
                "{path} is world-writable without the sticky bit; \
                 {PER_USER_REMEDY}"
            ),
            UnsafeReason::AclMayGrantWrite => write!(
                formatter,
                "{path} has an access ACL that may let another user write to \
                 it; {PER_USER_REMEDY}"
            ),
            UnsafeReason::OwnershipIgnored => write!(
                formatter,
                "{path} is on a volume that ignores ownership; \
                 {PER_USER_REMEDY}"
            ),
        }
    }
}

/// A cache root checked to be its owner's alone, the only way to build a
/// prune.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafePruneRoot {
    root: PathBuf,
}

impl SafePruneRoot {
    /// Check the canonical `root` and every ancestor up to `/`, as sshd's
    /// `StrictModes` does.
    ///
    /// Each must be a directory no other user can create, rename or remove
    /// names in. The root must be the effective user's own; an ancestor may
    /// also be root's or a listed owner's. A sticky ancestor may be writable
    /// by others, but never the root: the sticky bit stops renames, not
    /// another user creating a predictable name first.
    ///
    /// # Errors
    ///
    /// [`UnsafeRoot`] naming the first path, from the root up, that fails.
    pub fn check(
        root: &Path,
        filesystem: &dyn CacheFilesystem,
        ownership: &dyn Ownership,
        trusted_owners: &TrustedOwners,
    ) -> Result<Self, UnsafeRoot> {
        let refuse = |path: &Path, reason| UnsafeRoot {
            path: path.to_path_buf(),
            reason,
        };
        if filesystem.ignores_ownership(root).unwrap_or(true) {
            return Err(refuse(root, UnsafeReason::OwnershipIgnored));
        }
        for (path, position) in root.ancestors().zip(positions()) {
            let entry = filesystem.entry(path).map_err(|error| {
                refuse(path, UnsafeReason::Unexaminable(error.kind()))
            })?;
            let acl = filesystem.access_acl(path).map_err(|error| {
                refuse(path, UnsafeReason::Unexaminable(error.kind()))
            })?;
            Directory {
                entry,
                acl,
                position,
            }
            .verdict(ownership, trusted_owners)
            .map_err(|reason| refuse(path, reason))?;
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Root,
    Ancestor,
}

fn positions() -> impl Iterator<Item = Position> {
    std::iter::once(Position::Root).chain(std::iter::repeat(Position::Ancestor))
}

struct Directory {
    entry: EntryMetadata,
    acl: AccessAcl,
    position: Position,
}

impl Directory {
    fn verdict(
        &self,
        ownership: &dyn Ownership,
        trusted_owners: &TrustedOwners,
    ) -> Result<(), UnsafeReason> {
        if self.entry.kind != FileKind::Directory {
            return Err(UnsafeReason::NotADirectory);
        }
        self.owner_verdict(ownership.effective_uid(), trusted_owners)?;
        if self.acl == AccessAcl::Other {
            return Err(UnsafeReason::AclMayGrantWrite);
        }
        self.world_verdict()?;
        self.group_verdict(ownership)
    }

    fn owner_verdict(
        &self,
        uid: u32,
        trusted_owners: &TrustedOwners,
    ) -> Result<(), UnsafeReason> {
        let owner = self.entry.owner;
        match self.position {
            Position::Root if owner != uid => {
                Err(UnsafeReason::RootOwnedByAnother { owner })
            }
            Position::Ancestor
                if owner != uid
                    && owner != 0
                    && (UNTRUSTABLE_UIDS.contains(&owner)
                        || !trusted_owners.contains(owner)) =>
            {
                Err(UnsafeReason::AncestorOwnedByAnother { owner })
            }
            Position::Root | Position::Ancestor => Ok(()),
        }
    }

    const fn world_verdict(&self) -> Result<(), UnsafeReason> {
        if self.entry.mode & WORLD_WRITABLE == 0 {
            return Ok(());
        }
        match self.position {
            Position::Root => Err(UnsafeReason::WorldWritableRoot),
            Position::Ancestor if self.is_sticky() => Ok(()),
            Position::Ancestor => {
                Err(UnsafeReason::WorldWritableWithoutStickyBit)
            }
        }
    }

    /// A directory with a POSIX ACL shows its mask in its group bits, so only
    /// the sticky bit can excuse those bits there.
    fn group_verdict(
        &self,
        ownership: &dyn Ownership,
    ) -> Result<(), UnsafeReason> {
        if self.entry.mode & GROUP_WRITABLE == 0
            || (self.position == Position::Ancestor && self.is_sticky())
        {
            return Ok(());
        }
        if self.acl == AccessAcl::Posix {
            return Err(UnsafeReason::AclMayGrantWrite);
        }
        let gid = ownership.effective_gid();
        if self.entry.group != gid {
            return Err(UnsafeReason::WritableBySharedGroup {
                group: format!("gid {}", self.entry.group),
            });
        }
        match ownership.primary_group() {
            PrimaryGroup::Private => Ok(()),
            PrimaryGroup::Shared { name } => {
                Err(UnsafeReason::WritableBySharedGroup { group: name })
            }
            PrimaryGroup::Unresolved => {
                Err(UnsafeReason::WritableByUnresolvedGroup { gid })
            }
        }
    }

    const fn is_sticky(&self) -> bool {
        self.entry.mode & STICKY != 0
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::super::super::filesystem::testing::ReportingFilesystem;
    use super::super::super::filesystem::{
        AccessAcl, EntryMetadata, FileKind, SystemFilesystem,
    };
    use super::super::{Ownership, PrimaryGroup, ProcessOwnership};
    use super::{
        SafePruneRoot, TrustedOwners, UnsafeReason, UnsafeRoot,
        TRUSTED_OWNERS_VAR,
    };

    const UID: u32 = 1000;
    const GID: u32 = 1000;
    const OTHER: u32 = 1001;
    const ROOT: &str = "/srv/cache/accelerator";

    struct Fixed(PrimaryGroup);

    impl Ownership for Fixed {
        fn effective_uid(&self) -> u32 {
            UID
        }

        fn effective_gid(&self) -> u32 {
            GID
        }

        fn primary_group(&self) -> PrimaryGroup {
            self.0.clone()
        }
    }

    fn private() -> Fixed {
        Fixed(PrimaryGroup::Private)
    }

    fn shared() -> Fixed {
        Fixed(PrimaryGroup::Shared {
            name: "staff".to_owned(),
        })
    }

    const fn directory(owner: u32, group: u32, mode: u32) -> EntryMetadata {
        EntryMetadata {
            kind: FileKind::Directory,
            owner,
            group,
            mode: 0o040_000 | mode,
            modified_seconds: Some(0),
        }
    }

    /// A synthetic `/srv/cache/accelerator`: `/` and `/srv` root-owned
    /// `0755`, `/srv/cache` and the root as given.
    struct Tree {
        cache: EntryMetadata,
        root: EntryMetadata,
        acls: Vec<(&'static str, AccessAcl)>,
        ownership_ignored: bool,
    }

    impl Tree {
        const fn new() -> Self {
            Self {
                cache: directory(0, 0, 0o755),
                root: directory(UID, GID, 0o700),
                acls: Vec::new(),
                ownership_ignored: false,
            }
        }

        const fn cache(mut self, entry: EntryMetadata) -> Self {
            self.cache = entry;
            self
        }

        const fn root(mut self, entry: EntryMetadata) -> Self {
            self.root = entry;
            self
        }

        fn acl(mut self, path: &'static str, acl: AccessAcl) -> Self {
            self.acls.push((path, acl));
            self
        }

        const fn ignoring_ownership(mut self) -> Self {
            self.ownership_ignored = true;
            self
        }

        fn filesystem(&self) -> ReportingFilesystem {
            let entries = [
                ("/", directory(0, 0, 0o755)),
                ("/srv", directory(0, 0, 0o755)),
                ("/srv/cache", self.cache),
                (ROOT, self.root),
            ];
            let mut filesystem = ReportingFilesystem::over_system()
                .reporting_ownership_ignored(
                    Path::new(ROOT),
                    self.ownership_ignored,
                );
            for (path, entry) in entries {
                let acl = self
                    .acls
                    .iter()
                    .find(|(acl_path, _)| *acl_path == path)
                    .map_or(AccessAcl::None, |(_, acl)| *acl);
                filesystem = filesystem
                    .reporting(Path::new(path), entry)
                    .reporting_acl(Path::new(path), acl);
            }
            filesystem
        }

        fn check_as(
            &self,
            ownership: &Fixed,
            trusted: &TrustedOwners,
        ) -> Result<SafePruneRoot, UnsafeRoot> {
            SafePruneRoot::check(
                Path::new(ROOT),
                &self.filesystem(),
                ownership,
                trusted,
            )
        }

        fn check(&self) -> Result<SafePruneRoot, UnsafeRoot> {
            self.check_as(&private(), &TrustedOwners::none())
        }
    }

    #[track_caller]
    fn refused(
        outcome: Result<SafePruneRoot, UnsafeRoot>,
        path: &str,
        reason: &UnsafeReason,
    ) -> UnsafeRoot {
        let unsafe_root = outcome.expect_err("the root was accepted");
        assert_eq!(unsafe_root.path, PathBuf::from(path));
        assert_eq!(&unsafe_root.reason, reason);
        unsafe_root
    }

    #[test]
    fn a_root_and_ancestors_only_its_owner_and_root_can_write_are_safe() {
        let safe = Tree::new().check().expect("safe");
        assert_eq!(safe.path(), Path::new(ROOT));
    }

    #[test]
    fn a_root_owned_ancestor_is_accepted() {
        assert!(Tree::new().cache(directory(0, 0, 0o755)).check().is_ok());
    }

    #[test]
    fn a_sticky_world_writable_ancestor_is_accepted() {
        assert!(Tree::new().cache(directory(0, 0, 0o1777)).check().is_ok());
    }

    #[test]
    fn a_root_under_a_writable_ancestor_is_not_pruned() {
        refused(
            Tree::new().cache(directory(0, 0, 0o777)).check(),
            "/srv/cache",
            &UnsafeReason::WorldWritableWithoutStickyBit,
        );
    }

    #[test]
    fn a_per_user_root_under_a_sticky_shared_parent_is_pruned() {
        assert!(Tree::new()
            .cache(directory(0, 0, 0o1777))
            .root(directory(UID, GID, 0o700))
            .check()
            .is_ok());
    }

    #[test]
    fn a_private_group_writable_ancestor_is_accepted() {
        assert!(Tree::new()
            .cache(directory(UID, GID, 0o775))
            .check()
            .is_ok());
    }

    #[test]
    fn a_shared_group_writable_ancestor_is_refused() {
        let unsafe_root = refused(
            Tree::new()
                .cache(directory(UID, GID, 0o775))
                .check_as(&shared(), &TrustedOwners::none()),
            "/srv/cache",
            &UnsafeReason::WritableBySharedGroup {
                group: "staff".to_owned(),
            },
        );
        assert!(unsafe_root.to_string().contains("chmod g-w /srv/cache"));
    }

    #[test]
    fn a_shared_group_writable_root_is_not_pruned_automatically() {
        refused(
            Tree::new()
                .root(directory(UID, GID, 0o770))
                .check_as(&shared(), &TrustedOwners::none()),
            ROOT,
            &UnsafeReason::WritableBySharedGroup {
                group: "staff".to_owned(),
            },
        );
    }

    #[test]
    fn a_user_private_group_writable_root_is_pruned() {
        assert!(Tree::new().root(directory(UID, GID, 0o775)).check().is_ok());
    }

    #[test]
    fn a_root_writable_by_another_private_looking_group_is_refused() {
        refused(
            Tree::new().root(directory(UID, GID + 5, 0o775)).check(),
            ROOT,
            &UnsafeReason::WritableBySharedGroup {
                group: format!("gid {}", GID + 5),
            },
        );
    }

    #[test]
    fn an_unresolvable_account_refuses_with_the_cause_and_remedy() {
        let unsafe_root = refused(
            Tree::new().root(directory(UID, GID, 0o775)).check_as(
                &Fixed(PrimaryGroup::Unresolved),
                &TrustedOwners::none(),
            ),
            ROOT,
            &UnsafeReason::WritableByUnresolvedGroup { gid: GID },
        );
        let message = unsafe_root.to_string();
        assert!(
            message.contains(&format!("could not resolve group {GID}")),
            "{message}"
        );
        assert!(message.contains(&format!("chmod g-w {ROOT}")), "{message}");
    }

    #[test]
    fn a_user_owned_world_writable_root_is_not_pruned() {
        let unsafe_root = refused(
            Tree::new().root(directory(UID, GID, 0o1777)).check(),
            ROOT,
            &UnsafeReason::WorldWritableRoot,
        );
        let message = unsafe_root.to_string();
        assert!(message.contains(&format!("chmod 700 {ROOT}")), "{message}");
        assert!(message.contains("per-user"), "{message}");
    }

    #[test]
    fn a_root_owned_sticky_root_is_not_pruned() {
        let unsafe_root = refused(
            Tree::new().root(directory(0, 0, 0o1777)).check(),
            ROOT,
            &UnsafeReason::RootOwnedByAnother { owner: 0 },
        );
        assert!(unsafe_root.to_string().contains("per-user"));
    }

    #[test]
    fn a_root_owned_0755_root_is_not_pruned() {
        refused(
            Tree::new().root(directory(0, 0, 0o755)).check(),
            ROOT,
            &UnsafeReason::RootOwnedByAnother { owner: 0 },
        );
    }

    #[test]
    fn a_root_owned_by_another_uid_is_not_pruned() {
        refused(
            Tree::new().root(directory(OTHER, OTHER, 0o700)).check(),
            ROOT,
            &UnsafeReason::RootOwnedByAnother { owner: OTHER },
        );
    }

    #[test]
    fn an_ancestor_owned_by_an_untrusted_owner_names_the_variable() {
        let unsafe_root = refused(
            Tree::new().cache(directory(OTHER, OTHER, 0o755)).check(),
            "/srv/cache",
            &UnsafeReason::AncestorOwnedByAnother { owner: OTHER },
        );
        assert!(unsafe_root
            .to_string()
            .contains(&format!("{TRUSTED_OWNERS_VAR}={OTHER}")));
    }

    fn trusting(uid: u32) -> TrustedOwners {
        TrustedOwners::parse(&uid.to_string()).expect("parses")
    }

    #[test]
    fn an_ancestor_owned_by_a_trusted_owner_is_accepted() {
        assert!(Tree::new()
            .cache(directory(OTHER, OTHER, 0o755))
            .check_as(&private(), &trusting(OTHER))
            .is_ok());
    }

    #[test]
    fn a_trusted_owner_does_not_admit_a_writable_ancestor() {
        refused(
            Tree::new()
                .cache(directory(OTHER, OTHER, 0o777))
                .check_as(&private(), &trusting(OTHER)),
            "/srv/cache",
            &UnsafeReason::WorldWritableWithoutStickyBit,
        );
    }

    #[test]
    fn a_trusted_owner_does_not_admit_a_foreign_root() {
        refused(
            Tree::new()
                .root(directory(OTHER, OTHER, 0o700))
                .check_as(&private(), &trusting(OTHER)),
            ROOT,
            &UnsafeReason::RootOwnedByAnother { owner: OTHER },
        );
    }

    #[test]
    fn the_overflow_uid_and_uid_99_are_refused_even_when_listed() {
        for uid in [65_534, 99] {
            let unsafe_root = refused(
                Tree::new()
                    .cache(directory(uid, uid, 0o755))
                    .check_as(&private(), &trusting(uid)),
                "/srv/cache",
                &UnsafeReason::AncestorOwnedByAnother { owner: uid },
            );
            assert!(
                !unsafe_root.to_string().contains(TRUSTED_OWNERS_VAR),
                "the refusal offers uid {uid}"
            );
        }
    }

    #[test]
    fn an_acl_granting_another_user_write_refuses_the_directory() {
        refused(
            Tree::new().acl("/srv/cache", AccessAcl::Other).check(),
            "/srv/cache",
            &UnsafeReason::AclMayGrantWrite,
        );
        assert!(Tree::new()
            .acl("/srv/cache", AccessAcl::None)
            .check()
            .is_ok());
    }

    #[test]
    fn a_posix_acl_denies_the_private_group_exception() {
        refused(
            Tree::new()
                .root(directory(UID, GID, 0o775))
                .acl(ROOT, AccessAcl::Posix)
                .check(),
            ROOT,
            &UnsafeReason::AclMayGrantWrite,
        );
        assert!(Tree::new().acl(ROOT, AccessAcl::Posix).check().is_ok());
    }

    #[test]
    fn a_root_on_a_volume_ignoring_ownership_is_refused() {
        refused(
            Tree::new().ignoring_ownership().check(),
            ROOT,
            &UnsafeReason::OwnershipIgnored,
        );
    }

    #[test]
    fn a_symlink_in_the_walk_is_refused() {
        refused(
            Tree::new()
                .root(EntryMetadata {
                    kind: FileKind::Symlink,
                    ..directory(UID, GID, 0o700)
                })
                .check(),
            ROOT,
            &UnsafeReason::NotADirectory,
        );
    }

    #[test]
    fn the_unsafe_root_message_names_the_condition_and_remedy() {
        let unsafe_root = UnsafeRoot {
            path: PathBuf::from("/Users/shared/cache"),
            reason: UnsafeReason::WritableBySharedGroup {
                group: "staff".to_owned(),
            },
        };
        assert_eq!(
            unsafe_root.to_string(),
            "/Users/shared/cache is group-writable by the shared group \
             staff; run chmod g-w /Users/shared/cache"
        );
    }

    #[test]
    fn a_real_temporary_root_is_safe() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().canonicalize().expect("canonical");
        assert!(SafePruneRoot::check(
            &root,
            &SystemFilesystem,
            &ProcessOwnership::default(),
            &TrustedOwners::none(),
        )
        .is_ok());
    }

    #[test]
    fn trusted_owners_parse_only_comma_separated_decimal_uids() {
        assert_eq!(
            TrustedOwners::parse("1001,1000"),
            Ok(TrustedOwners {
                uids: [1000, 1001].into()
            })
        );
        assert!(TrustedOwners::parse("1001").expect("one").contains(1001));
        for malformed in [
            "",
            " 1001",
            "1001 ",
            "1001,",
            ",1001",
            "1001,,1000",
            "-1",
            "+5",
            "0x3e9",
            "4294967296",
            "abc",
        ] {
            assert!(
                TrustedOwners::parse(malformed).is_err(),
                "{malformed:?} parsed"
            );
        }
    }
}

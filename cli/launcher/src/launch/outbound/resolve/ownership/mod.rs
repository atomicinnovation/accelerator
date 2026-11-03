//! Which cache bookkeeping directories this process may trust not to be
//! renamed into by another user.

pub mod accounts;
pub mod safe_root;

use std::path::Path;
use std::sync::OnceLock;

use self::accounts::{AccountLookup, LibcLookup};
use super::filesystem::{AccessAcl, CacheFilesystem, EntryMetadata, FileKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimaryGroup {
    /// The user-private group of Ubuntu, Debian, Fedora and RHEL: named as
    /// the user's login, listing no one else.
    Private,
    Shared {
        name: String,
    },
    Unresolved,
}

pub trait Ownership {
    fn effective_uid(&self) -> u32;

    fn effective_gid(&self) -> u32;

    fn primary_group(&self) -> PrimaryGroup;

    fn primary_group_is_private(&self) -> bool {
        self.primary_group() == PrimaryGroup::Private
    }
}

pub struct ProcessOwnership {
    lookups: Vec<Box<dyn AccountLookup + Send + Sync>>,
    primary_group: OnceLock<PrimaryGroup>,
}

impl Default for ProcessOwnership {
    fn default() -> Self {
        Self::with_lookups(vec![
            Box::new(LibcLookup),
            #[cfg(target_os = "linux")]
            Box::new(accounts::GetentLookup::production()),
        ])
    }
}

impl ProcessOwnership {
    #[must_use]
    pub fn with_lookups(
        lookups: Vec<Box<dyn AccountLookup + Send + Sync>>,
    ) -> Self {
        Self {
            lookups,
            primary_group: OnceLock::new(),
        }
    }
}

impl Ownership for ProcessOwnership {
    #[cfg(unix)]
    fn effective_uid(&self) -> u32 {
        rustix::process::geteuid().as_raw()
    }

    #[cfg(not(unix))]
    fn effective_uid(&self) -> u32 {
        u32::MAX
    }

    #[cfg(unix)]
    fn effective_gid(&self) -> u32 {
        rustix::process::getegid().as_raw()
    }

    #[cfg(not(unix))]
    fn effective_gid(&self) -> u32 {
        u32::MAX
    }

    fn primary_group(&self) -> PrimaryGroup {
        self.primary_group
            .get_or_init(|| {
                let (uid, gid) = (self.effective_uid(), self.effective_gid());
                self.lookups
                    .iter()
                    .find_map(|lookup| lookup.accounts(uid, gid))
                    .map_or(PrimaryGroup::Unresolved, |accounts| {
                        let members: Vec<&str> = accounts
                            .members
                            .iter()
                            .map(String::as_str)
                            .collect();
                        if is_private_group(
                            &accounts.login,
                            &accounts.group_name,
                            &members,
                        ) {
                            PrimaryGroup::Private
                        } else {
                            PrimaryGroup::Shared {
                                name: accounts.group_name,
                            }
                        }
                    })
            })
            .clone()
    }
}

#[must_use]
pub fn is_private_group(
    login: &str,
    group_name: &str,
    members: &[&str],
) -> bool {
    group_name == login && members.iter().all(|member| *member == login)
}

#[must_use]
pub const fn is_trusted_entry(
    entry: &EntryMetadata,
    uid: u32,
    private_gid: Option<u32>,
) -> bool {
    let writable_only_by_its_owner = entry.mode & 0o022 == 0;
    let writable_only_through_the_private_group = entry.mode & 0o002 == 0
        && matches!(private_gid, Some(gid) if gid == entry.group);
    matches!(entry.kind, FileKind::Directory)
        && entry.owner == uid
        && (writable_only_by_its_owner
            || writable_only_through_the_private_group)
}

pub fn is_trusted_directory(
    path: &Path,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
) -> bool {
    filesystem.entry(path).is_ok_and(|entry| {
        is_trusted_directory_entry(path, &entry, filesystem, ownership)
    })
}

/// A private group is admitted only with no access ACL: on Linux the group
/// mode bits of a directory with a POSIX ACL are its mask, so another user's
/// entry would read as group-writable.
pub fn is_trusted_directory_entry(
    path: &Path,
    entry: &EntryMetadata,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
) -> bool {
    let uid = ownership.effective_uid();
    if is_trusted_entry(entry, uid, None) {
        return true;
    }
    let gid = ownership.effective_gid();
    entry.group == gid
        && filesystem
            .access_acl(path)
            .is_ok_and(|acl| acl == AccessAcl::None)
        && ownership.primary_group_is_private()
        && is_trusted_entry(entry, uid, Some(gid))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::super::filesystem::testing::ReportingFilesystem;
    use super::super::filesystem::{AccessAcl, EntryMetadata, FileKind};
    use super::accounts::{AccountLookup, Accounts};
    use super::{
        is_private_group, is_trusted_directory, is_trusted_entry, Ownership,
        PrimaryGroup, ProcessOwnership,
    };

    const UID: u32 = 501;
    const GID: u32 = 501;

    fn directory(mode: u32) -> EntryMetadata {
        EntryMetadata {
            kind: FileKind::Directory,
            owner: UID,
            group: 20,
            mode,
            modified_seconds: Some(0),
        }
    }

    #[test]
    fn only_an_owned_directory_nobody_else_can_write_is_trusted() {
        let cases = [
            (directory(0o040_700), true),
            (directory(0o040_755), true),
            (directory(0o040_775), false),
            (directory(0o040_777), false),
            (
                EntryMetadata {
                    owner: UID + 1,
                    ..directory(0o040_700)
                },
                false,
            ),
            (
                EntryMetadata {
                    kind: FileKind::Symlink,
                    ..directory(0o120_700)
                },
                false,
            ),
            (
                EntryMetadata {
                    kind: FileKind::File,
                    ..directory(0o100_600)
                },
                false,
            ),
        ];
        for (entry, trusted) in cases {
            assert_eq!(
                is_trusted_entry(&entry, UID, None),
                trusted,
                "{entry:?}"
            );
        }
    }

    #[test]
    fn a_directory_writable_only_through_the_private_group_is_trusted() {
        let private = EntryMetadata {
            group: GID,
            ..directory(0o040_775)
        };
        assert!(is_trusted_entry(&private, UID, Some(GID)));
        assert!(!is_trusted_entry(&private, UID, Some(GID + 1)));
        assert!(!is_trusted_entry(&private, UID, None));
        assert!(!is_trusted_entry(
            &EntryMetadata {
                group: GID,
                ..directory(0o040_777)
            },
            UID,
            Some(GID)
        ));
    }

    #[test]
    fn a_private_group_is_named_for_its_user_and_lists_no_one_else() {
        assert!(is_private_group("toby", "toby", &[]));
        assert!(is_private_group("toby", "toby", &["toby"]));
        assert!(!is_private_group("toby", "toby", &["toby", "alex"]));
        assert!(!is_private_group("toby", "toby", &["alex"]));
        assert!(!is_private_group("toby", "staff", &[]));
    }

    struct Answering {
        accounts: Option<Accounts>,
        asked: Arc<AtomicUsize>,
    }

    impl AccountLookup for Answering {
        fn accounts(&self, _uid: u32, _gid: u32) -> Option<Accounts> {
            self.asked.fetch_add(1, Ordering::SeqCst);
            self.accounts.clone()
        }
    }

    fn private_accounts() -> Accounts {
        Accounts {
            login: "toby".to_owned(),
            group_name: "toby".to_owned(),
            members: vec![],
        }
    }

    fn answering(
        accounts: Option<Accounts>,
    ) -> (Box<Answering>, Arc<AtomicUsize>) {
        let asked = Arc::new(AtomicUsize::new(0));
        let lookup = Box::new(Answering {
            accounts,
            asked: Arc::clone(&asked),
        });
        (lookup, asked)
    }

    #[test]
    fn the_getent_fallback_is_used_only_when_libc_has_no_entry() {
        let (libc, _) = answering(Some(private_accounts()));
        let (fallback, fallback_asked) = answering(None);
        let answered = ProcessOwnership::with_lookups(vec![libc, fallback]);
        assert_eq!(answered.primary_group(), PrimaryGroup::Private);
        assert_eq!(fallback_asked.load(Ordering::SeqCst), 0);

        let (libc, _) = answering(None);
        let (fallback, fallback_asked) = answering(Some(Accounts {
            group_name: "staff".to_owned(),
            ..private_accounts()
        }));
        let fell_back = ProcessOwnership::with_lookups(vec![libc, fallback]);
        assert_eq!(
            fell_back.primary_group(),
            PrimaryGroup::Shared {
                name: "staff".to_owned()
            }
        );
        assert_eq!(fallback_asked.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn no_answer_leaves_the_primary_group_unresolved_and_asks_once() {
        let (none, asked) = answering(None);
        let ownership = ProcessOwnership::with_lookups(vec![none]);

        assert_eq!(ownership.primary_group(), PrimaryGroup::Unresolved);
        assert_eq!(ownership.primary_group(), PrimaryGroup::Unresolved);
        assert_eq!(asked.load(Ordering::SeqCst), 1);
    }

    fn oracle(command: &str, args: &[&str]) -> String {
        let output = Command::new(command)
            .args(args)
            .output()
            .expect("oracle runs");
        assert!(output.status.success(), "{command} {args:?}");
        String::from_utf8(output.stdout)
            .expect("utf-8")
            .trim()
            .to_owned()
    }

    #[cfg(target_os = "macos")]
    fn listed_members(gid: u32) -> Vec<String> {
        oracle(
            "dscacheutil",
            &["-q", "group", "-a", "gid", &gid.to_string()],
        )
        .lines()
        .find_map(|line| line.strip_prefix("users:"))
        .map(|users| users.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
    }

    #[cfg(target_os = "linux")]
    fn listed_members(gid: u32) -> Vec<String> {
        oracle("getent", &["group", &gid.to_string()])
            .rsplit(':')
            .next()
            .map(|members| {
                members
                    .split(',')
                    .filter(|member| !member.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(unix)]
    #[test]
    fn the_process_ownership_answers_for_the_current_user() {
        let ownership = ProcessOwnership::default();
        let login = oracle("id", &["-un"]);
        let group_name = oracle("id", &["-gn"]);
        let members = listed_members(ownership.effective_gid());
        let members: Vec<&str> = members.iter().map(String::as_str).collect();

        let expected = if is_private_group(&login, &group_name, &members) {
            PrimaryGroup::Private
        } else {
            PrimaryGroup::Shared { name: group_name }
        };

        assert_eq!(ownership.primary_group(), expected);
    }

    struct Fixed {
        primary_group: PrimaryGroup,
        asked: AtomicUsize,
    }

    impl Fixed {
        fn new(primary_group: PrimaryGroup) -> Self {
            Self {
                primary_group,
                asked: AtomicUsize::new(0),
            }
        }
    }

    impl Ownership for Fixed {
        fn effective_uid(&self) -> u32 {
            UID
        }

        fn effective_gid(&self) -> u32 {
            GID
        }

        fn primary_group(&self) -> PrimaryGroup {
            self.asked.fetch_add(1, Ordering::SeqCst);
            self.primary_group.clone()
        }
    }

    fn reported(
        metadata: EntryMetadata,
        acl: AccessAcl,
    ) -> ReportingFilesystem {
        ReportingFilesystem::over_system()
            .reporting(Path::new("/cache/dir"), metadata)
            .reporting_acl(Path::new("/cache/dir"), acl)
    }

    fn trusted(filesystem: &ReportingFilesystem, ownership: &Fixed) -> bool {
        is_trusted_directory(Path::new("/cache/dir"), filesystem, ownership)
    }

    fn private_group_writable() -> EntryMetadata {
        EntryMetadata {
            group: GID,
            ..directory(0o040_775)
        }
    }

    #[test]
    fn a_private_group_writable_directory_without_an_acl_is_trusted() {
        let filesystem = reported(private_group_writable(), AccessAcl::None);
        assert!(trusted(&filesystem, &Fixed::new(PrimaryGroup::Private)));
        assert!(!trusted(
            &filesystem,
            &Fixed::new(PrimaryGroup::Shared {
                name: "staff".to_owned()
            })
        ));
        assert!(!trusted(&filesystem, &Fixed::new(PrimaryGroup::Unresolved)));
    }

    #[test]
    fn an_acl_granting_another_user_write_is_not_private() {
        for acl in [AccessAcl::Posix, AccessAcl::Other] {
            let filesystem = reported(private_group_writable(), acl);
            assert!(
                !trusted(&filesystem, &Fixed::new(PrimaryGroup::Private)),
                "{acl:?}"
            );
        }
    }

    #[test]
    fn a_directory_writable_by_another_group_is_never_trusted() {
        let filesystem = reported(
            EntryMetadata {
                group: GID + 1,
                ..directory(0o040_775)
            },
            AccessAcl::None,
        );
        assert!(!trusted(&filesystem, &Fixed::new(PrimaryGroup::Private)));
    }

    #[test]
    fn a_directory_nobody_else_can_write_never_resolves_the_primary_group() {
        let ownership = Fixed::new(PrimaryGroup::Private);
        let filesystem = reported(directory(0o040_700), AccessAcl::None);

        assert!(trusted(&filesystem, &ownership));
        assert_eq!(ownership.asked.load(Ordering::SeqCst), 0);
    }
}

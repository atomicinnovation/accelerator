//! Who owns the cache's bookkeeping directories, and which of them this
//! process may trust not to be renamed into by another user.

use std::path::Path;

use super::filesystem::{CacheFilesystem, EntryMetadata, FileKind};

pub trait Ownership {
    fn effective_uid(&self) -> u32;
}

pub struct ProcessOwnership;

impl Ownership for ProcessOwnership {
    #[cfg(unix)]
    fn effective_uid(&self) -> u32 {
        rustix::process::geteuid().as_raw()
    }

    #[cfg(not(unix))]
    fn effective_uid(&self) -> u32 {
        u32::MAX
    }
}

/// A real directory owned by `uid` that neither its group nor anyone else can
/// write to, so no other user can rename entries into or out of it.
#[must_use]
pub const fn is_trusted_entry(entry: &EntryMetadata, uid: u32) -> bool {
    matches!(entry.kind, FileKind::Directory)
        && entry.owner == uid
        && entry.mode & 0o022 == 0
}

/// [`is_trusted_entry`] for the directory at `path`, read without following a
/// symlink there; an unreadable path is never trusted.
pub fn is_trusted_directory(
    path: &Path,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
) -> bool {
    filesystem
        .entry(path)
        .is_ok_and(|entry| is_trusted_entry(&entry, ownership.effective_uid()))
}

#[cfg(test)]
mod tests {
    use super::super::filesystem::{EntryMetadata, FileKind};
    use super::is_trusted_entry;

    const UID: u32 = 501;

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
            assert_eq!(is_trusted_entry(&entry, UID), trusted, "{entry:?}");
        }
    }
}

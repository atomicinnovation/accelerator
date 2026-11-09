//! A claims directory, read the same way for version and tree claims, so a
//! prune never reads "no claims" into one it could not read or trust.

use std::io;
use std::path::{Path, PathBuf};

use super::filesystem::{CacheFilesystem, EntryMetadata, FileKind};
use super::ownership::{is_trusted_directory_entry, Ownership};

const LAUNCHER_ID_LENGTH: usize = 16;

/// The sixteen lowercase hex characters a launcher names its claims with.
#[must_use]
pub fn is_launcher_id(candidate: &str) -> bool {
    candidate.len() == LAUNCHER_ID_LENGTH
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Only a regular file owned by the effective uid is trusted to protect
/// anything.
#[must_use]
pub const fn is_valid_claim(entry: &EntryMetadata, uid: u32) -> bool {
    matches!(entry.kind, FileKind::File) && entry.owner == uid
}

#[derive(Debug)]
pub enum ClaimsDirectoryListing<T> {
    Read(T),
    /// Distinct from an empty listing, so a prune never reads "no claims"
    /// into a directory it could not read.
    Unreadable(io::ErrorKind),
    /// A directory another user could rename claims into.
    Untrusted,
}

impl<T> ClaimsDirectoryListing<T> {
    pub fn map<U>(
        self,
        read: impl FnOnce(T) -> U,
    ) -> ClaimsDirectoryListing<U> {
        match self {
            Self::Read(claims) => ClaimsDirectoryListing::Read(read(claims)),
            Self::Unreadable(kind) => ClaimsDirectoryListing::Unreadable(kind),
            Self::Untrusted => ClaimsDirectoryListing::Untrusted,
        }
    }
}

/// A name the claim kind parsed, with what its `lstat` found.
pub struct ListedEntry<N> {
    pub name: N,
    pub path: PathBuf,
    pub entry: EntryMetadata,
}

/// A claim whose `lstat` fails for any reason but its being gone makes the
/// whole listing unreadable, since it might have protected anything.
pub fn list<N>(
    directory: &Path,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
    parse: impl Fn(&str) -> Option<N>,
) -> ClaimsDirectoryListing<Vec<ListedEntry<N>>> {
    match filesystem.entry(directory) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return ClaimsDirectoryListing::Read(Vec::new());
        }
        Err(error) => return ClaimsDirectoryListing::Unreadable(error.kind()),
        Ok(entry)
            if !is_trusted_directory_entry(
                directory, &entry, filesystem, ownership,
            ) =>
        {
            return ClaimsDirectoryListing::Untrusted;
        }
        Ok(_) => {}
    }
    let names = match filesystem.list(directory) {
        Ok(names) => names,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return ClaimsDirectoryListing::Unreadable(error.kind()),
    };
    let mut listed = Vec::new();
    for raw in names.iter().filter_map(|name| name.to_str()) {
        let Some(name) = parse(raw) else {
            continue;
        };
        let path = directory.join(raw);
        let entry = match filesystem.entry(&path) {
            Ok(entry) => entry,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                return ClaimsDirectoryListing::Unreadable(error.kind());
            }
        };
        listed.push(ListedEntry { name, path, entry });
    }
    ClaimsDirectoryListing::Read(listed)
}

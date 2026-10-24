//! The filesystem calls the single-file cache makes, behind a port so a test
//! can record a dispatch's exact sequence.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// What a path is, read without following a symlink at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Directory,
    Symlink,
    Other,
}

/// A path's own metadata, never its symlink target's.
///
/// An unreadable or pre-epoch mtime is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryMetadata {
    pub kind: FileKind,
    pub owner: u32,
    pub group: u32,
    pub mode: u32,
    pub modified_seconds: Option<u64>,
}

/// Two paths name the same directory exactly when their identities are equal,
/// even where their canonical spellings differ, as across a bind mount or a
/// macOS firmlink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectoryIdentity {
    pub device: u64,
    pub inode: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDirectory {
    pub canonical: PathBuf,
    pub identity: DirectoryIdentity,
}

/// What a directory's access ACL adds to its mode bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessAcl {
    None,
    /// A POSIX ACL, whose mask the group mode bits then show.
    Posix,
    /// Any other ACL, which mode bits only approximate.
    Other,
}

pub trait CacheFilesystem {
    /// An entry the directory stream fails on is skipped.
    ///
    /// # Errors
    ///
    /// If `dir` cannot be opened for listing.
    fn list(&self, dir: &Path) -> io::Result<Vec<OsString>>;

    fn exists(&self, path: &Path) -> bool;

    /// # Errors
    ///
    /// If `path` cannot be read.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// # Errors
    ///
    /// If `path` cannot be read or is not UTF-8.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Read a regular file of at most `limit` bytes, never following a symlink
    /// at `path` and never blocking on a FIFO or device.
    ///
    /// # Errors
    ///
    /// If `path` is not a regular file, cannot be read, or holds more than
    /// `limit` bytes.
    fn read_bounded(&self, path: &Path, limit: u64) -> io::Result<Vec<u8>>;

    /// # Errors
    ///
    /// If any missing component cannot be created.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Write `bytes` to a file readable only by its owner, never following a
    /// symlink at `path`.
    ///
    /// # Errors
    ///
    /// If the file cannot be opened or written.
    fn write_private(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;

    /// # Errors
    ///
    /// If the permissions cannot be set.
    fn make_executable(&self, path: &Path) -> io::Result<()>;

    /// # Errors
    ///
    /// If the rename fails.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// # Errors
    ///
    /// If the file cannot be removed.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// # Errors
    ///
    /// If the directory cannot be removed.
    fn remove_dir(&self, path: &Path) -> io::Result<()>;

    /// Never follows a symlink.
    ///
    /// # Errors
    ///
    /// If anything in the tree cannot be removed.
    fn remove_tree(&self, path: &Path) -> io::Result<()>;

    /// # Errors
    ///
    /// If `path` cannot be examined.
    fn entry(&self, path: &Path) -> io::Result<EntryMetadata>;

    /// # Errors
    ///
    /// If `path` does not exist or cannot be resolved.
    fn resolve(&self, path: &Path) -> io::Result<ResolvedDirectory>;

    /// Create one directory accessible only to its owner.
    ///
    /// # Errors
    ///
    /// `AlreadyExists` when `path` exists, whatever it is, or any other error
    /// the creation meets.
    fn create_private_dir(&self, path: &Path) -> io::Result<()>;

    /// Never follows a symlink at `path`.
    ///
    /// # Errors
    ///
    /// If the file cannot be opened or its time set.
    fn set_modified(&self, path: &Path, seconds: u64) -> io::Result<()>;

    /// Never follows a symlink at `path`.
    ///
    /// # Errors
    ///
    /// If the ACL cannot be read for a reason other than there being none.
    fn access_acl(&self, path: &Path) -> io::Result<AccessAcl>;

    /// Whether the volume holding `path` ignores file ownership, so its
    /// permission bits are not enforced.
    ///
    /// # Errors
    ///
    /// If the volume cannot be examined.
    fn ignores_ownership(&self, path: &Path) -> io::Result<bool>;
}

pub struct SystemFilesystem;

impl CacheFilesystem for SystemFilesystem {
    fn list(&self, dir: &Path) -> io::Result<Vec<OsString>> {
        Ok(std::fs::read_dir(dir)?
            .flatten()
            .map(|entry| entry.file_name())
            .collect())
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn read_bounded(&self, path: &Path, limit: u64) -> io::Result<Vec<u8>> {
        use std::io::Read as _;
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        let file = not_following_links_nor_blocking(&mut options).open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        let mut bytes = Vec::new();
        file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("longer than {limit} bytes"),
            ));
        }
        Ok(bytes)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn write_private(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        use std::io::Write as _;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        private_and_not_following_links(&mut options)
            .open(path)?
            .write_all(bytes)
    }

    fn make_executable(&self, path: &Path) -> io::Result<()> {
        set_executable(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir(path)
    }

    fn remove_tree(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    fn entry(&self, path: &Path) -> io::Result<EntryMetadata> {
        Ok(entry_metadata(&std::fs::symlink_metadata(path)?))
    }

    fn resolve(&self, path: &Path) -> io::Result<ResolvedDirectory> {
        let canonical = std::fs::canonicalize(path)?;
        let identity = identity_of(&std::fs::metadata(&canonical)?);
        Ok(ResolvedDirectory {
            canonical,
            identity,
        })
    }

    fn create_private_dir(&self, path: &Path) -> io::Result<()> {
        private_dir_builder().create(path)
    }

    fn set_modified(&self, path: &Path, seconds: u64) -> io::Result<()> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true);
        not_following_links_nor_blocking(&mut options)
            .open(path)?
            .set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds),
            )
    }

    fn access_acl(&self, path: &Path) -> io::Result<AccessAcl> {
        access_acl_of(path)
    }

    fn ignores_ownership(&self, path: &Path) -> io::Result<bool> {
        ignores_ownership_of(path)
    }
}

#[cfg(target_os = "macos")]
fn ignores_ownership_of(path: &Path) -> io::Result<bool> {
    let flags = rustix::fs::statfs(path)?.f_flags;
    Ok(flags & libc::MNT_IGNORE_OWNERSHIP.cast_unsigned() != 0)
}

#[cfg(not(target_os = "macos"))]
const fn ignores_ownership_of(_path: &Path) -> io::Result<bool> {
    Ok(false)
}

/// An allow entry granting any of these lets its principal create, rename or
/// remove names in the directory. Owner entries are counted too: matching the
/// owner would need a uid lookup, and refusing one only refuses more.
#[cfg(target_os = "macos")]
fn access_acl_of(path: &Path) -> io::Result<AccessAcl> {
    use exacl::{AclOption, Perm};
    let writes = Perm::WRITE | Perm::APPEND | Perm::DELETE_CHILD;
    let entries = exacl::getfacl(path, AclOption::SYMLINK_ACL)?;
    Ok(
        if entries
            .iter()
            .any(|entry| entry.allow && entry.perms.intersects(writes))
        {
            AccessAcl::Other
        } else {
            AccessAcl::None
        },
    )
}

#[cfg(target_os = "linux")]
const OTHER_ACL_ATTRIBUTES: [&str; 3] =
    ["system.nfs4_acl", "system.nfs4_acl_xdr", "system.richacl"];

#[cfg(target_os = "linux")]
fn access_acl_of(path: &Path) -> io::Result<AccessAcl> {
    if has_attribute(path, "system.posix_acl_access")? {
        return Ok(AccessAcl::Posix);
    }
    for name in OTHER_ACL_ATTRIBUTES {
        if has_attribute(path, name)? {
            return Ok(AccessAcl::Other);
        }
    }
    Ok(AccessAcl::None)
}

#[cfg(target_os = "linux")]
fn has_attribute(path: &Path, name: &str) -> io::Result<bool> {
    use rustix::io::Errno;
    match rustix::fs::lgetxattr(path, name, &mut [0_u8; 0][..]) {
        Ok(_) => Ok(true),
        Err(Errno::NODATA | Errno::NOTSUP) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const fn access_acl_of(_path: &Path) -> io::Result<AccessAcl> {
    Ok(AccessAcl::Other)
}

#[cfg(unix)]
fn entry_metadata(metadata: &std::fs::Metadata) -> EntryMetadata {
    use std::os::unix::fs::MetadataExt as _;
    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        FileKind::Symlink
    } else if file_type.is_dir() {
        FileKind::Directory
    } else if file_type.is_file() {
        FileKind::File
    } else {
        FileKind::Other
    };
    EntryMetadata {
        kind,
        owner: metadata.uid(),
        group: metadata.gid(),
        mode: metadata.mode(),
        modified_seconds: metadata
            .modified()
            .ok()
            .and_then(|modified| {
                modified.duration_since(std::time::UNIX_EPOCH).ok()
            })
            .map(|since| since.as_secs()),
    }
}

#[cfg(not(unix))]
fn entry_metadata(_metadata: &std::fs::Metadata) -> EntryMetadata {
    EntryMetadata {
        kind: FileKind::Other,
        owner: u32::MAX,
        group: u32::MAX,
        mode: 0,
        modified_seconds: None,
    }
}

#[cfg(unix)]
fn identity_of(metadata: &std::fs::Metadata) -> DirectoryIdentity {
    use std::os::unix::fs::MetadataExt as _;
    DirectoryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(not(unix))]
const fn identity_of(_metadata: &std::fs::Metadata) -> DirectoryIdentity {
    DirectoryIdentity {
        device: 0,
        inode: 0,
    }
}

#[cfg(unix)]
fn private_dir_builder() -> std::fs::DirBuilder {
    use std::os::unix::fs::DirBuilderExt as _;
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    builder
}

#[cfg(not(unix))]
fn private_dir_builder() -> std::fs::DirBuilder {
    std::fs::DirBuilder::new()
}

#[cfg(unix)]
fn private_and_not_following_links(
    options: &mut std::fs::OpenOptions,
) -> &mut std::fs::OpenOptions {
    use std::os::unix::fs::OpenOptionsExt as _;
    options
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits().cast_signed())
}

#[cfg(unix)]
fn not_following_links_nor_blocking(
    options: &mut std::fs::OpenOptions,
) -> &mut std::fs::OpenOptions {
    use std::os::unix::fs::OpenOptionsExt as _;
    options.custom_flags(
        (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK)
            .bits()
            .cast_signed(),
    )
}

#[cfg(not(unix))]
const fn not_following_links_nor_blocking(
    options: &mut std::fs::OpenOptions,
) -> &mut std::fs::OpenOptions {
    options
}

#[cfg(not(unix))]
const fn private_and_not_following_links(
    options: &mut std::fs::OpenOptions,
) -> &mut std::fs::OpenOptions {
    options
}

#[cfg(unix)]
fn set_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
const fn set_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Compiled always because the launcher's integration tests link the library
/// without `cfg(test)`.
#[doc(hidden)]
pub mod testing {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, MutexGuard};

    use super::{
        AccessAcl, CacheFilesystem, EntryMetadata, ResolvedDirectory,
        SystemFilesystem,
    };

    pub struct ReportingFilesystem<F = SystemFilesystem> {
        inner: F,
        overlay: Mutex<Overlay>,
    }

    #[derive(Default)]
    struct Overlay {
        entries: BTreeMap<PathBuf, EntryMetadata>,
        resolutions: BTreeMap<PathBuf, ResolvedDirectory>,
        acls: BTreeMap<PathBuf, AccessAcl>,
        ownership_ignored: BTreeMap<PathBuf, bool>,
        resolved: Vec<PathBuf>,
    }

    impl ReportingFilesystem {
        #[must_use]
        pub fn over_system() -> Self {
            Self::over(SystemFilesystem)
        }
    }

    impl<F> ReportingFilesystem<F> {
        pub fn over(inner: F) -> Self {
            Self {
                inner,
                overlay: Mutex::default(),
            }
        }

        #[must_use]
        pub fn reporting(self, path: &Path, metadata: EntryMetadata) -> Self {
            self.overlay().entries.insert(path.to_path_buf(), metadata);
            self
        }

        #[must_use]
        pub fn resolving(
            self,
            path: &Path,
            resolved: ResolvedDirectory,
        ) -> Self {
            self.overlay()
                .resolutions
                .insert(path.to_path_buf(), resolved);
            self
        }

        #[must_use]
        pub fn reporting_acl(self, path: &Path, acl: AccessAcl) -> Self {
            self.overlay().acls.insert(path.to_path_buf(), acl);
            self
        }

        #[must_use]
        pub fn reporting_ownership_ignored(
            self,
            path: &Path,
            ignored: bool,
        ) -> Self {
            self.overlay()
                .ownership_ignored
                .insert(path.to_path_buf(), ignored);
            self
        }

        pub fn resolved_paths(&self) -> Vec<PathBuf> {
            self.overlay().resolved.clone()
        }

        fn overlay(&self) -> MutexGuard<'_, Overlay> {
            self.overlay
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        }
    }

    impl<F: CacheFilesystem> CacheFilesystem for ReportingFilesystem<F> {
        fn list(&self, dir: &Path) -> io::Result<Vec<OsString>> {
            self.inner.list(dir)
        }

        fn exists(&self, path: &Path) -> bool {
            self.inner.exists(path)
        }

        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            self.inner.read(path)
        }

        fn read_to_string(&self, path: &Path) -> io::Result<String> {
            self.inner.read_to_string(path)
        }

        fn read_bounded(&self, path: &Path, limit: u64) -> io::Result<Vec<u8>> {
            self.inner.read_bounded(path, limit)
        }

        fn create_dir_all(&self, path: &Path) -> io::Result<()> {
            self.inner.create_dir_all(path)
        }

        fn write_private(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
            self.inner.write_private(path, bytes)
        }

        fn make_executable(&self, path: &Path) -> io::Result<()> {
            self.inner.make_executable(path)
        }

        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
            self.inner.rename(from, to)
        }

        fn remove_file(&self, path: &Path) -> io::Result<()> {
            self.inner.remove_file(path)
        }

        fn remove_dir(&self, path: &Path) -> io::Result<()> {
            self.inner.remove_dir(path)
        }

        fn remove_tree(&self, path: &Path) -> io::Result<()> {
            self.inner.remove_tree(path)
        }

        fn entry(&self, path: &Path) -> io::Result<EntryMetadata> {
            let reported = self.overlay().entries.get(path).copied();
            reported.map_or_else(|| self.inner.entry(path), Ok)
        }

        fn resolve(&self, path: &Path) -> io::Result<ResolvedDirectory> {
            let reported = {
                let mut overlay = self.overlay();
                overlay.resolved.push(path.to_path_buf());
                overlay.resolutions.get(path).cloned()
            };
            reported.map_or_else(|| self.inner.resolve(path), Ok)
        }

        fn create_private_dir(&self, path: &Path) -> io::Result<()> {
            self.inner.create_private_dir(path)
        }

        fn set_modified(&self, path: &Path, seconds: u64) -> io::Result<()> {
            self.inner.set_modified(path, seconds)
        }

        fn access_acl(&self, path: &Path) -> io::Result<AccessAcl> {
            let reported = self.overlay().acls.get(path).copied();
            reported.map_or_else(|| self.inner.access_acl(path), Ok)
        }

        fn ignores_ownership(&self, path: &Path) -> io::Result<bool> {
            let reported = self.overlay().ownership_ignored.get(path).copied();
            reported.map_or_else(|| self.inner.ignores_ownership(path), Ok)
        }
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    use super::{AccessAcl, CacheFilesystem as _, SystemFilesystem};

    fn acl_of(path: &Path) -> AccessAcl {
        SystemFilesystem.access_acl(path).expect("access ACL")
    }

    #[cfg(target_os = "macos")]
    fn add_ace(path: &Path, ace: &str) {
        let status = Command::new("/bin/chmod")
            .args(["+a", ace])
            .arg(path)
            .status()
            .expect("chmod +a");
        assert!(status.success(), "chmod +a {ace}");
    }

    #[test]
    fn a_local_temporary_volume_enforces_ownership() {
        let fresh = tempfile::tempdir().expect("tempdir");
        assert!(!SystemFilesystem
            .ignores_ownership(fresh.path())
            .expect("statfs"));
    }

    #[test]
    fn the_system_filesystem_reports_acls() {
        let fresh = tempfile::tempdir().expect("tempdir");
        assert_eq!(acl_of(fresh.path()), AccessAcl::None);

        #[cfg(target_os = "macos")]
        {
            let denied = tempfile::tempdir().expect("tempdir");
            add_ace(denied.path(), "everyone deny delete");
            assert_eq!(acl_of(denied.path()), AccessAcl::None);

            let allowed = tempfile::tempdir().expect("tempdir");
            add_ace(allowed.path(), "everyone allow add_file");
            assert_eq!(acl_of(allowed.path()), AccessAcl::Other);
        }

        #[cfg(target_os = "linux")]
        {
            let granted = tempfile::tempdir().expect("tempdir");
            match Command::new("setfacl")
                .args(["-m", "u:nobody:rwx"])
                .arg(granted.path())
                .status()
            {
                Ok(status) => {
                    assert!(status.success(), "setfacl");
                    assert_eq!(acl_of(granted.path()), AccessAcl::Posix);
                }
                Err(_) if std::env::var_os("CI").is_some() => {
                    panic!("setfacl is required under CI and was not on PATH")
                }
                Err(_) => eprintln!("skipping: setfacl not on PATH"),
            }
        }
    }
}

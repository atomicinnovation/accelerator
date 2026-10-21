//! The filesystem calls the single-file cache makes, behind a port so a test
//! can record a dispatch's exact sequence.

use std::ffi::OsString;
use std::io;
use std::path::Path;

pub trait CacheFilesystem {
    /// The names in `dir`; an entry the directory stream fails on is skipped.
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

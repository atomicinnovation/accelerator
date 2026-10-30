//! Filesystem guards for tests that need a permission refusal.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

const SKIP_VARIABLE: &str = "ACCELERATOR_TEST_SKIP_PERMISSION_TESTS";

/// A directory sealed at `0o555` until the guard drops, when it is restored to
/// `0o755` so a panicking test still leaves a removable directory.
#[derive(Debug)]
pub struct ReadOnlyDir {
    path: PathBuf,
}

impl ReadOnlyDir {
    /// Seals `path`, or returns `Ok(None)` when the process ignores
    /// permissions and the skip variable is set.
    ///
    /// A process running as root can still write under a sealed directory, so
    /// a permission test would pass without exercising anything; that is a
    /// failure unless the skip variable opts out.
    ///
    /// # Errors
    /// When the mode cannot be changed, or the sealed directory is still
    /// writable and the skip variable is unset.
    pub fn new(path: &Path) -> io::Result<Option<Self>> {
        fs::set_permissions(path, fs::Permissions::from_mode(0o555))?;
        let sealed = Self {
            path: path.to_path_buf(),
        };
        if permissions_are_enforced(path) {
            return Ok(Some(sealed));
        }
        drop(sealed);
        if std::env::var_os(SKIP_VARIABLE).is_some() {
            return Ok(None);
        }
        Err(io::Error::other(format!(
            "permission tests cannot run as root; set {SKIP_VARIABLE}=1 to skip"
        )))
    }
}

impl Drop for ReadOnlyDir {
    fn drop(&mut self) {
        let _ =
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755));
    }
}

/// A file made unreadable (`0o000`) until the guard drops, when it is
/// restored to `0o644`.
#[derive(Debug)]
pub struct UnreadableFile {
    path: PathBuf,
}

impl UnreadableFile {
    /// Seals `path`, or returns `Ok(None)` when the process ignores
    /// permissions and the skip variable is set.
    ///
    /// # Errors
    /// When the mode cannot be changed, or the sealed file is still readable
    /// and the skip variable is unset.
    pub fn new(path: &Path) -> io::Result<Option<Self>> {
        fs::set_permissions(path, fs::Permissions::from_mode(0o000))?;
        let sealed = Self {
            path: path.to_path_buf(),
        };
        if fs::read(path).is_err() {
            return Ok(Some(sealed));
        }
        drop(sealed);
        if std::env::var_os(SKIP_VARIABLE).is_some() {
            return Ok(None);
        }
        Err(io::Error::other(format!(
            "permission tests cannot run as root; set {SKIP_VARIABLE}=1 to skip"
        )))
    }
}

impl Drop for UnreadableFile {
    fn drop(&mut self) {
        let _ =
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o644));
    }
}

fn permissions_are_enforced(sealed: &Path) -> bool {
    let probe = sealed.join(".permission-probe");
    match fs::File::create(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            false
        }
        Err(_) => true,
    }
}

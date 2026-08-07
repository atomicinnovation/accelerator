//! Filesystem read ports the `adr` and `frontmatter_validation` conventions
//! need, injected rather than reached for directly — the same shape as
//! `vcs::classify::CheckoutProbe`/`vcs::mode::ModeProbe`.

use std::path::Path;

/// Reads a directory's immediate entries.
pub trait DirReader {
    /// Immediate entries' file names, or `None` if `dir` doesn't exist.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when `dir` exists but cannot be read.
    fn list(&self, dir: &Path) -> Result<Option<Vec<String>>, kernel::Error>;
}

/// Reads a single file's contents.
pub trait FileReader {
    /// A file's contents, or `None` if it doesn't exist or isn't a regular
    /// file.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when `path` exists but cannot be read.
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error>;
}

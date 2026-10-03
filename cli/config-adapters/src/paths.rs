//! The path checks' filesystem port over the real filesystem.

use std::path::Path;
use std::path::PathBuf;

use config::consent::ExecutablePaths;

pub struct SystemExecutablePaths;

impl ExecutablePaths for SystemExecutablePaths {
    fn canonicalise(&self, existing: &Path) -> Option<PathBuf> {
        dunce::canonicalize(existing).ok()
    }

    fn link_target(&self, path: &Path) -> Option<PathBuf> {
        std::fs::symlink_metadata(path)
            .is_ok_and(|facts| facts.file_type().is_symlink())
            .then(|| std::fs::read_link(path).ok())
            .flatten()
    }

    fn exists(&self, path: &Path) -> bool {
        std::fs::symlink_metadata(path).is_ok()
    }
}

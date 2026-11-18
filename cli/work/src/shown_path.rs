//! Paths as a refusal shows them: relative to the repository root.

use std::path::Path;
use std::path::PathBuf;

/// `path` relative to `root` when it lies under it, else as it is.
pub fn relative_to(path: PathBuf, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or(path)
}

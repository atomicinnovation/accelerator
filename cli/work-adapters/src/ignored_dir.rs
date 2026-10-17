//! State directories version control never sees.

use std::path::Path;

use corpus::store::AtomicWrite;
use corpus::StoreError;

const IGNORE_EVERYTHING: &str = "*\n";

/// Creates `dir` with a `.gitignore` that ignores everything in it, verified
/// before anything else lands there.
///
/// An existing rule is left alone, and a missing one is written atomically,
/// so two commands preparing together never read a truncated rule.
///
/// # Errors
///
/// [`StoreError`] when the directory or rule cannot be written, or the rule
/// reads back as anything else.
pub fn prepare_ignored_dir(
    dir: &Path,
    writer: &dyn AtomicWrite,
) -> Result<(), StoreError> {
    let io = |path: &Path, error: &std::io::Error| StoreError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    };
    std::fs::create_dir_all(dir).map_err(|error| io(dir, &error))?;
    let ignore = dir.join(".gitignore");
    if holds_the_rule(&ignore) {
        return Ok(());
    }
    writer.write(&ignore, IGNORE_EVERYTHING.as_bytes())?;
    if holds_the_rule(&ignore) {
        Ok(())
    } else {
        Err(StoreError::Io {
            path: ignore.display().to_string(),
            detail: "the ignore rule could not be verified".to_owned(),
        })
    }
}

fn holds_the_rule(ignore: &Path) -> bool {
    std::fs::read_to_string(ignore)
        .is_ok_and(|content| content == IGNORE_EVERYTHING)
}

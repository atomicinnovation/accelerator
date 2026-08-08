//! `merge_move`: relocate a file or directory onto a destination, merging
//! directories recursively — the Rust equivalent of `scripts/fs-common.sh`'s
//! bash function of the same name.
//!
//! NON-ATOMIC by design, mirroring the bash original: a per-entry
//! rename/remove sequence, so a mid-merge failure can leave a partially
//! merged tree. A re-run converges (each migration's own idempotency
//! self-check is the recovery net, not this function).

use std::fs;
use std::path::Path;

use migrate::ports::MigrationError;
use store::ensure_contained;
use store::WriteBounds;

/// # Errors
/// [`MigrationError`] when `dst` escapes `root` or the underlying filesystem
/// operation fails.
pub fn merge_move(
    src: &Path,
    dst: &Path,
    root: &Path,
) -> Result<(), MigrationError> {
    if !src.exists() {
        return Ok(());
    }
    let bounds = WriteBounds {
        permitted_root: root,
        project_root: root,
    };
    ensure_contained(dst, &bounds)
        .map_err(|error| MigrationError::new(error.to_string()))?;

    if !dst.exists() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| io_error(parent, &error))?;
        }
        fs::rename(src, dst).map_err(|error| io_error(src, &error))?;
        return Ok(());
    }

    let src_is_dir = src.is_dir();
    let dst_is_dir = dst.is_dir();
    if !src_is_dir || !dst_is_dir {
        fs::remove_dir_all(dst)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    fs::remove_file(dst)
                }
            })
            .map_err(|error| io_error(dst, &error))?;
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| io_error(parent, &error))?;
        }
        fs::rename(src, dst).map_err(|error| io_error(src, &error))?;
        return Ok(());
    }

    let entries = fs::read_dir(src).map_err(|error| io_error(src, &error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(src, &error))?;
        let name = entry.file_name();
        merge_move(&entry.path(), &dst.join(&name), root)?;
    }
    // A non-empty source after merge (`Err(_)`, not-found aside) signals a
    // non-converging merge — left in place, matching bash's own
    // diagnostic-and-continue behaviour rather than failing the migration.
    let _ = fs::remove_dir(src);
    Ok(())
}

fn io_error(path: &Path, error: &std::io::Error) -> MigrationError {
    MigrationError::new(format!("{}: {error}", path.display()))
}

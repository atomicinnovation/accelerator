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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::merge_move;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn a_missing_source_is_a_no_op() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let root = dir.path();

        merge_move(&root.join("absent"), &root.join("dst"), root)?;

        assert!(!root.join("dst").exists());
        Ok(())
    }

    #[test]
    fn an_absent_destination_file_is_a_plain_move() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let root = dir.path();
        fs::write(root.join("src.md"), "content")?;

        merge_move(&root.join("src.md"), &root.join("nested/dst.md"), root)?;

        assert!(!root.join("src.md").exists());
        assert_eq!(fs::read_to_string(root.join("nested/dst.md"))?, "content");
        Ok(())
    }

    #[test]
    fn an_absent_destination_directory_is_a_plain_move() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        let root = dir.path();
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("src/a.md"), "a")?;

        merge_move(&root.join("src"), &root.join("dst"), root)?;

        assert!(!root.join("src").exists());
        assert_eq!(fs::read_to_string(root.join("dst/a.md"))?, "a");
        Ok(())
    }

    #[test]
    fn a_colliding_leaf_file_is_source_wins() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let root = dir.path();
        fs::write(root.join("src.md"), "SRC")?;
        fs::write(root.join("dst.md"), "DST")?;

        merge_move(&root.join("src.md"), &root.join("dst.md"), root)?;

        assert!(!root.join("src.md").exists());
        assert_eq!(fs::read_to_string(root.join("dst.md"))?, "SRC");
        Ok(())
    }

    #[test]
    fn a_source_file_replaces_a_destination_directory() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        let root = dir.path();
        fs::write(root.join("src.md"), "SRC")?;
        fs::create_dir_all(root.join("dst.md/nested"))?;
        fs::write(root.join("dst.md/nested/x.md"), "x")?;

        merge_move(&root.join("src.md"), &root.join("dst.md"), root)?;

        assert!(root.join("dst.md").is_file());
        assert_eq!(fs::read_to_string(root.join("dst.md"))?, "SRC");
        Ok(())
    }

    #[test]
    fn two_directories_merge_source_wins_on_collision_dest_only_survives(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let root = dir.path();
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("src/shared.md"), "SRC")?;
        fs::write(root.join("src/only-in-src.md"), "new")?;
        fs::create_dir_all(root.join("dst"))?;
        fs::write(root.join("dst/shared.md"), "DST")?;
        fs::write(root.join("dst/only-in-dst.md"), "kept")?;

        merge_move(&root.join("src"), &root.join("dst"), root)?;

        assert!(!root.join("src").exists());
        assert_eq!(fs::read_to_string(root.join("dst/shared.md"))?, "SRC");
        assert_eq!(fs::read_to_string(root.join("dst/only-in-src.md"))?, "new");
        assert_eq!(
            fs::read_to_string(root.join("dst/only-in-dst.md"))?,
            "kept"
        );
        Ok(())
    }

    #[test]
    fn a_destination_escaping_root_refuses() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let root = dir.path().join("root");
        fs::create_dir_all(&root)?;
        fs::write(root.join("src.md"), "content")?;

        let result =
            merge_move(&root.join("src.md"), &root.join("../escape.md"), &root);

        assert!(result.is_err());
        assert!(root.join("src.md").exists());
        Ok(())
    }
}

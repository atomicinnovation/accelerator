//! Sealing: making a materialised tree read-only so a later hit can trust it
//! without re-verifying.
//!
//! The seal is a deterministic function of the executable bit — `0555` for a
//! file the archive marked executable, `0444` otherwise, directories left
//! owner-writable so the tree stays removable — so verification computes the
//! expected sealed mode rather than recording it a second time. It is not an
//! integrity discriminator: `tar` preserves read-only modes exactly, so a plain
//! `tar xzf` reproduces it. The check is retained only because the `stat` it
//! needs already happens.

#[cfg(unix)]
pub use unix::{seal_tree, sealed_mode};

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::Path;

    use crate::launch::core::tree::TreeError;

    /// The mode a sealed entry must carry, given the mode it was extracted with.
    #[must_use]
    pub const fn sealed_mode(is_dir: bool, extracted_mode: u32) -> u32 {
        if is_dir {
            0o755
        } else if extracted_mode & 0o111 != 0 {
            0o555
        } else {
            0o444
        }
    }

    /// Seal every file and directory under `root`, bottom-up.
    ///
    /// Bottom-up so a directory is not made read-only before its children are
    /// sealed. Symlinks are walked but never re-moded: `set_permissions`
    /// follows a link and would re-mode its target.
    ///
    /// # Errors
    ///
    /// [`TreeError::Seal`] if any entry cannot be walked or re-moded.
    pub fn seal_tree(root: &Path) -> Result<(), TreeError> {
        seal_dir(root)
    }

    fn seal_dir(dir: &Path) -> Result<(), TreeError> {
        let entries = fs::read_dir(dir)
            .map_err(|error| seal(dir, &format!("cannot read: {error}")))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| seal(dir, &format!("cannot read: {error}")))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|error| {
                seal(&path, &format!("cannot stat: {error}"))
            })?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                seal_dir(&path)?;
            } else {
                set_mode(&path, sealed_mode(false, current_mode(&path)?))?;
            }
        }
        // The directory itself is sealed after its contents.
        set_mode(dir, sealed_mode(true, 0))
    }

    fn current_mode(path: &Path) -> Result<u32, TreeError> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| seal(path, &format!("cannot stat: {error}")))?;
        Ok(metadata.permissions().mode())
    }

    fn set_mode(path: &Path, mode: u32) -> Result<(), TreeError> {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|error| seal(path, &format!("cannot chmod: {error}")))
    }

    fn seal(path: &Path, detail: &str) -> TreeError {
        TreeError::Seal {
            detail: format!("{}: {detail}", path.display()),
        }
    }

    #[cfg(test)]
    #[allow(clippy::expect_used)]
    mod tests {
        use std::fs;
        use std::os::unix::fs::PermissionsExt as _;

        use super::{seal_tree, sealed_mode};

        #[test]
        fn the_sealed_mode_is_a_function_of_the_executable_bit() {
            assert_eq!(sealed_mode(true, 0o700), 0o755);
            assert_eq!(sealed_mode(false, 0o755), 0o555);
            assert_eq!(sealed_mode(false, 0o644), 0o444);
        }

        #[test]
        fn a_sealed_tree_is_read_only_yet_still_removable() {
            let dir = tempfile::tempdir().expect("tempdir");
            let root = dir.path().join("gen");
            fs::create_dir(&root).expect("root");
            fs::create_dir(root.join("lib")).expect("lib");
            fs::write(root.join("lib/data.pak"), b"x").expect("data");
            let shell = root.join("shell");
            fs::write(&shell, b"#!/bin/sh\n").expect("shell");
            fs::set_permissions(&shell, fs::Permissions::from_mode(0o755))
                .expect("make executable");

            seal_tree(&root).expect("seal");

            let data_mode = fs::metadata(root.join("lib/data.pak"))
                .expect("data")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(data_mode, 0o444);
            let shell_mode =
                fs::metadata(&shell).expect("shell").permissions().mode()
                    & 0o777;
            assert_eq!(shell_mode, 0o555, "the executable bit is retained");

            // Writing without an intervening chmod is refused.
            assert!(fs::write(root.join("lib/data.pak"), b"y").is_err());
            // And the whole tree is still removable.
            fs::remove_dir_all(&root).expect("a sealed tree is removable");
        }

        #[test]
        fn a_symlinks_target_is_not_re_moded_by_the_seal() {
            let dir = tempfile::tempdir().expect("tempdir");
            let root = dir.path().join("gen");
            fs::create_dir(&root).expect("root");
            let target = root.join("target");
            fs::write(&target, b"payload").expect("target");
            fs::set_permissions(&target, fs::Permissions::from_mode(0o644))
                .expect("target mode");
            std::os::unix::fs::symlink("target", root.join("link"))
                .expect("symlink");

            seal_tree(&root).expect("seal");

            // The target keeps the sealed 0444 it earned as a real file, not a
            // mode applied through the link.
            let mode = fs::symlink_metadata(&target)
                .expect("target")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o444);
        }
    }
}

#[cfg(not(unix))]
pub fn seal_tree(
    _root: &std::path::Path,
) -> Result<(), crate::launch::core::tree::TreeError> {
    unimplemented!("sealing is a Unix-only path")
}

#[cfg(not(unix))]
#[must_use]
pub const fn sealed_mode(_is_dir: bool, _extracted_mode: u32) -> u32 {
    0
}

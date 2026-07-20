//! `atomic_write`: whole-file atomic replacement with a permitted-root symlink
//! refusal, shared by the config and corpus writers. A reader never observes a
//! partial file, and no component of the target may resolve outside the caller's
//! permitted root — a symlinked component is refused, never followed.
//!
//! Infrastructure only: depends on std, `tempfile` and `rustix`. Both consumers
//! translate [`WriteError`] into their own taxonomy, so this crate does not
//! depend on `kernel`.

use std::fs;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use tempfile::{Builder, NamedTempFile};

/// The staged temp-file name prefix. Pinned so a `tempfile` default change
/// cannot silently stop the `.accelerator/.gitignore` rule matching; a caller
/// writing that rule must use the same literal.
pub const TEMP_PREFIX: &str = ".tmp-";

/// An atomic-write failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteError {
    NotWritable { path: String },
    CrossFilesystem { path: String },
    UnsafePath { path: String },
    Io { path: String, detail: String },
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWritable { path } => {
                write!(formatter, "cannot write '{path}': not writable")
            }
            Self::CrossFilesystem { path } => write!(
                formatter,
                "atomic rename to '{path}' crossed a filesystem boundary"
            ),
            Self::UnsafePath { path } => write!(
                formatter,
                "refusing to write through an unsafe path '{path}'"
            ),
            Self::Io { path, detail } => {
                write!(formatter, "I/O error on '{path}': {detail}")
            }
        }
    }
}

impl std::error::Error for WriteError {}

/// How the persisted file's mode is chosen. The caller supplies any concrete
/// mode value, so `store` never reads the process-global umask.
#[derive(Debug, Clone, Copy)]
pub enum NewFileMode {
    /// Force this mode whether or not the target exists.
    Set(u32),
    /// Preserve an existing target's mode; create a fresh file at this mode.
    PreserveOr(u32),
}

/// The trusted roots bounding a write. `permitted_root` bounds the target;
/// `project_root` is the independently-discovered root a symlinked
/// `permitted_root` must itself resolve inside.
pub struct WriteBounds<'a> {
    pub permitted_root: &'a Path,
    pub project_root: &'a Path,
}

/// Whole-file atomic replacement: a reader never observes a partial file.
///
/// The mode is applied to the staged temp before the rename, so the target is
/// never briefly visible at the temp's default mode.
///
/// # Errors
/// Returns [`WriteError`] on a containment refusal, cross-filesystem staging, an
/// unwritable target, or any underlying I/O failure.
pub fn atomic_write(
    path: &Path,
    bytes: &[u8],
    bounds: &WriteBounds<'_>,
    mode: NewFileMode,
) -> Result<(), WriteError> {
    let parent = ensure_contained(path, bounds)?;
    fs::create_dir_all(&parent).map_err(|error| io(&parent, &error))?;
    let file_name = path.file_name().ok_or_else(|| unsafe_path(path))?;
    let target = parent.join(file_name);
    let staged = stage(&parent, &target, bytes, mode)?;
    persist(staged, &target)
}

/// Resolves `path` against `bounds` and returns the canonical directory a temp
/// should be staged in.
///
/// Refuses any component that escapes the permitted root through a symlink.
/// Read-only, so the write path and the read path share it and one file has a
/// single containment contract.
///
/// The nearest *existing* ancestor is canonicalised and checked before any
/// directory is created, and a `permitted_root` that is itself a symlink out of
/// `project_root` is refused — a cloned repository can ship `.accelerator` as
/// one, and canonicalising it as trusted would follow it.
///
/// # Errors
/// Returns [`WriteError::UnsafePath`] when a component escapes, or
/// [`WriteError::Io`] when a path cannot be resolved.
pub fn ensure_contained(
    path: &Path,
    bounds: &WriteBounds<'_>,
) -> Result<PathBuf, WriteError> {
    let project = fs::canonicalize(bounds.project_root)
        .map_err(|error| io(bounds.project_root, &error))?;
    let permitted = canonicalize_allowing_missing(bounds.permitted_root)?;
    if !permitted.starts_with(&project) {
        return Err(unsafe_path(bounds.permitted_root));
    }
    let parent = path.parent().unwrap_or(path);
    let parent_canonical = canonicalize_allowing_missing(parent)?;
    if !parent_canonical.starts_with(&permitted) {
        return Err(unsafe_path(path));
    }
    if is_symlink(path)? {
        let leaf = fs::canonicalize(path).map_err(|error| io(path, &error))?;
        if !leaf.starts_with(&permitted) {
            return Err(unsafe_path(path));
        }
    }
    Ok(parent_canonical)
}

/// Canonicalises `path` by resolving its deepest existing ancestor and
/// re-appending the not-yet-existing tail verbatim, so a target under a
/// not-yet-created permitted root still resolves for the containment check.
fn canonicalize_allowing_missing(path: &Path) -> Result<PathBuf, WriteError> {
    for ancestor in path.ancestors() {
        match fs::canonicalize(ancestor) {
            Ok(canonical) => {
                let tail = path.strip_prefix(ancestor).map_err(|_| {
                    WriteError::Io {
                        path: show(path),
                        detail: "path is not under its ancestor".to_owned(),
                    }
                })?;
                return Ok(canonical.join(tail));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(io(ancestor, &error)),
        }
    }
    Err(WriteError::Io {
        path: show(path),
        detail: "no existing ancestor".to_owned(),
    })
}

fn is_symlink(path: &Path) -> Result<bool, WriteError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io(path, &error)),
    }
}

fn stage(
    dir: &Path,
    target: &Path,
    bytes: &[u8],
    mode: NewFileMode,
) -> Result<NamedTempFile, WriteError> {
    let mut temp = Builder::new()
        .prefix(TEMP_PREFIX)
        .tempfile_in(dir)
        .map_err(|error| {
            if error.kind() == ErrorKind::PermissionDenied {
                WriteError::NotWritable { path: show(dir) }
            } else {
                io(dir, &error)
            }
        })?;
    if let Some(bits) = resolve_mode(target, mode)? {
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(bits))
            .map_err(|error| io(temp.path(), &error))?;
    }
    temp.write_all(bytes).map_err(|error| io(dir, &error))?;
    Ok(temp)
}

fn resolve_mode(
    target: &Path,
    mode: NewFileMode,
) -> Result<Option<u32>, WriteError> {
    match mode {
        NewFileMode::Set(bits) => Ok(Some(bits)),
        NewFileMode::PreserveOr(fresh) => match fs::metadata(target) {
            Ok(metadata) => Ok(Some(metadata.permissions().mode() & 0o777)),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Ok(Some(fresh))
            }
            Err(error) => Err(io(target, &error)),
        },
    }
}

fn persist(temp: NamedTempFile, target: &Path) -> Result<(), WriteError> {
    temp.persist(target)
        .map(|_| ())
        .map_err(|error| classify_persist_error(target, &error.error))
}

fn classify_persist_error(target: &Path, error: &IoError) -> WriteError {
    if rustix::io::Errno::from_io_error(error) == Some(rustix::io::Errno::XDEV)
    {
        WriteError::CrossFilesystem { path: show(target) }
    } else {
        io(target, error)
    }
}

fn show(path: &Path) -> String {
    path.display().to_string()
}

fn io(path: &Path, error: &IoError) -> WriteError {
    WriteError::Io {
        path: show(path),
        detail: error.to_string(),
    }
}

fn unsafe_path(path: &Path) -> WriteError {
    WriteError::UnsafePath { path: show(path) }
}

/// The process file-creation mask, read without leaving it changed.
///
/// `umask(2)` has no read-only form, so the value is set and immediately
/// restored; rustix wraps the syscall, so no `unsafe` is needed. Callers derive
/// a fresh file's mode from it (e.g. `0o666 & !current_umask()`) and hand that
/// to [`NewFileMode`], keeping [`atomic_write`] free of any ambient-state read.
#[must_use]
#[allow(clippy::useless_conversion)]
pub fn current_umask() -> u32 {
    let previous =
        rustix::process::umask(rustix::fs::Mode::from_bits_truncate(0o022));
    rustix::process::umask(previous);
    u32::from(previous.bits())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::Path;

    use tempfile::TempDir;

    use super::{
        atomic_write, classify_persist_error, ensure_contained, stage,
        NewFileMode, WriteBounds, WriteError,
    };

    type TestError = Box<dyn std::error::Error>;

    fn bounds<'a>(permitted: &'a Path, project: &'a Path) -> WriteBounds<'a> {
        WriteBounds {
            permitted_root: permitted,
            project_root: project,
        }
    }

    fn temp_names(dir: &Path) -> Result<Vec<String>, TestError> {
        let mut names = Vec::new();
        for entry in fs::read_dir(dir)? {
            names.push(entry?.file_name().to_string_lossy().into_owned());
        }
        names.sort();
        Ok(names)
    }

    fn mode_of(path: &Path) -> Result<u32, TestError> {
        Ok(fs::metadata(path)?.permissions().mode() & 0o777)
    }

    #[test]
    fn a_successful_write_leaves_the_bytes_and_no_stray_temp(
    ) -> Result<(), TestError> {
        let project = TempDir::new()?;
        let permitted = project.path().join("permitted");
        fs::create_dir_all(&permitted)?;
        let target = permitted.join("file.md");
        atomic_write(
            &target,
            b"hello",
            &bounds(&permitted, project.path()),
            NewFileMode::Set(0o600),
        )?;
        assert_eq!(fs::read(&target)?, b"hello");
        assert_eq!(temp_names(&permitted)?, vec!["file.md".to_owned()]);
        Ok(())
    }

    #[test]
    fn a_write_creates_a_not_yet_existing_permitted_root(
    ) -> Result<(), TestError> {
        let project = TempDir::new()?;
        let permitted = project.path().join(".accelerator");
        let target = permitted.join("config.md");
        atomic_write(
            &target,
            b"x",
            &bounds(&permitted, project.path()),
            NewFileMode::PreserveOr(0o644),
        )?;
        assert_eq!(fs::read(&target)?, b"x");
        Ok(())
    }

    #[test]
    fn an_abandoned_stage_leaves_existing_content_intact(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("file.md");
        fs::write(&target, b"seeded")?;
        {
            let _staged = stage(
                dir.path(),
                &target,
                b"never persisted",
                NewFileMode::Set(0o600),
            )?;
        }
        assert_eq!(fs::read(&target)?, b"seeded");
        assert_eq!(temp_names(dir.path())?, vec!["file.md".to_owned()]);
        Ok(())
    }

    #[test]
    fn an_abandoned_stage_leaves_a_fresh_path_absent() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        let target = dir.path().join("file.md");
        {
            let _staged =
                stage(dir.path(), &target, b"x", NewFileMode::Set(0o600))?;
        }
        assert!(!target.exists());
        assert!(temp_names(dir.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn cross_filesystem_errno_classifies_as_cross_filesystem() {
        let error = std::io::Error::from_raw_os_error(
            rustix::io::Errno::XDEV.raw_os_error(),
        );
        assert_eq!(
            classify_persist_error(Path::new("/x/log"), &error),
            WriteError::CrossFilesystem {
                path: "/x/log".to_owned()
            }
        );
    }

    #[test]
    fn any_other_errno_classifies_as_io() {
        let error = std::io::Error::from_raw_os_error(
            rustix::io::Errno::NOENT.raw_os_error(),
        );
        assert!(matches!(
            classify_persist_error(Path::new("/x/log"), &error),
            WriteError::Io { .. }
        ));
    }

    #[test]
    fn current_umask_reads_without_leaving_it_changed() {
        let first = super::current_umask();
        let second = super::current_umask();
        assert_eq!(first, second, "reading the umask must not change it");
        assert_eq!(first & !0o777, 0, "a umask carries only permission bits");
    }

    #[test]
    fn set_forces_the_mode_on_a_fresh_file() -> Result<(), TestError> {
        let project = TempDir::new()?;
        let permitted = project.path().join("p");
        fs::create_dir_all(&permitted)?;
        let target = permitted.join("secret");
        atomic_write(
            &target,
            b"s",
            &bounds(&permitted, project.path()),
            NewFileMode::Set(0o600),
        )?;
        assert_eq!(mode_of(&target)?, 0o600);
        Ok(())
    }

    #[test]
    fn set_clamps_a_preexisting_wider_mode() -> Result<(), TestError> {
        let project = TempDir::new()?;
        let permitted = project.path().join("p");
        fs::create_dir_all(&permitted)?;
        let target = permitted.join("secret");
        fs::write(&target, b"old")?;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644))?;
        atomic_write(
            &target,
            b"new",
            &bounds(&permitted, project.path()),
            NewFileMode::Set(0o600),
        )?;
        assert_eq!(mode_of(&target)?, 0o600);
        Ok(())
    }

    #[test]
    fn preserve_or_keeps_an_existing_mode() -> Result<(), TestError> {
        let project = TempDir::new()?;
        let permitted = project.path().join("p");
        fs::create_dir_all(&permitted)?;
        let target = permitted.join("shared");
        fs::write(&target, b"old")?;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o664))?;
        atomic_write(
            &target,
            b"new",
            &bounds(&permitted, project.path()),
            NewFileMode::PreserveOr(0o600),
        )?;
        assert_eq!(mode_of(&target)?, 0o664);
        Ok(())
    }

    #[test]
    fn preserve_or_uses_the_fresh_mode_for_a_new_file() -> Result<(), TestError>
    {
        let project = TempDir::new()?;
        let permitted = project.path().join("p");
        fs::create_dir_all(&permitted)?;
        let target = permitted.join("shared");
        atomic_write(
            &target,
            b"x",
            &bounds(&permitted, project.path()),
            NewFileMode::PreserveOr(0o640),
        )?;
        assert_eq!(mode_of(&target)?, 0o640);
        Ok(())
    }

    #[test]
    fn a_parent_symlink_escaping_the_permitted_root_is_refused(
    ) -> Result<(), TestError> {
        let project = TempDir::new()?;
        let permitted = project.path().join("permitted");
        fs::create_dir_all(&permitted)?;
        let outside = project.path().join("outside");
        fs::create_dir_all(&outside)?;
        std::os::unix::fs::symlink(&outside, permitted.join("link"))?;
        let target = permitted.join("link/file.md");
        assert!(matches!(
            ensure_contained(&target, &bounds(&permitted, project.path())),
            Err(WriteError::UnsafePath { .. })
        ));
        Ok(())
    }

    #[test]
    fn a_leaf_symlink_escaping_the_permitted_root_is_refused(
    ) -> Result<(), TestError> {
        let project = TempDir::new()?;
        let permitted = project.path().join("permitted");
        fs::create_dir_all(&permitted)?;
        let outside = project.path().join("outside.md");
        fs::write(&outside, b"secret")?;
        let target = permitted.join("config.local.md");
        std::os::unix::fs::symlink(&outside, &target)?;
        assert!(matches!(
            ensure_contained(&target, &bounds(&permitted, project.path())),
            Err(WriteError::UnsafePath { .. })
        ));
        Ok(())
    }

    #[test]
    fn an_absent_root_reached_through_a_symlinked_ancestor_is_refused(
    ) -> Result<(), TestError> {
        let project = TempDir::new()?;
        let elsewhere = TempDir::new()?;
        // `permitted` does not exist yet and is reached via a symlink that
        // leaves the project.
        let link = project.path().join("link");
        std::os::unix::fs::symlink(elsewhere.path(), &link)?;
        let permitted = link.join(".accelerator");
        let target = permitted.join("config.md");
        assert!(matches!(
            ensure_contained(&target, &bounds(&permitted, project.path())),
            Err(WriteError::UnsafePath { .. })
        ));
        Ok(())
    }

    #[test]
    fn a_permitted_root_that_is_a_symlink_out_is_refused(
    ) -> Result<(), TestError> {
        let project = TempDir::new()?;
        let elsewhere = TempDir::new()?;
        let permitted = project.path().join(".accelerator");
        std::os::unix::fs::symlink(elsewhere.path(), &permitted)?;
        let target = permitted.join("config.md");
        assert!(matches!(
            ensure_contained(&target, &bounds(&permitted, project.path())),
            Err(WriteError::UnsafePath { .. })
        ));
        Ok(())
    }

    #[test]
    fn a_permitted_root_under_a_symlinked_project_is_allowed(
    ) -> Result<(), TestError> {
        // macOS /tmp -> /private/tmp: canonicalising both sides must not
        // falsely refuse a legitimate file under a symlinked root.
        let base = TempDir::new()?;
        let real = base.path().join("real");
        fs::create_dir_all(&real)?;
        let linked = base.path().join("linked");
        std::os::unix::fs::symlink(&real, &linked)?;
        let permitted = linked.join(".accelerator");
        let target = permitted.join("config.md");
        atomic_write(
            &target,
            b"x",
            &bounds(&permitted, &linked),
            NewFileMode::PreserveOr(0o644),
        )?;
        assert_eq!(fs::read(real.join(".accelerator/config.md"))?, b"x");
        Ok(())
    }
}

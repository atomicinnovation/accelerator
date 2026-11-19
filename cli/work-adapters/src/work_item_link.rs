//! Links a work item to its remote issue by rewriting `external_id` in the
//! file's frontmatter, under the file's lock, as one atomic write.

use std::path::Path;

use corpus::lock::ExclusiveLock;
use corpus::lock::LockName;
use corpus::AtomicWrite;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use tracker::ExternalId;

use crate::sync::create::AuthorError;

/// Upserts `external_id`, preserving every other field and the body.
///
/// # Errors
///
/// [`AuthorError`] when the lock cannot be taken, the file cannot be read or
/// parsed, or the write fails.
pub fn set_external_id(
    lock: &dyn ExclusiveLock,
    store: &dyn AtomicWrite,
    path: &Path,
    external_id: &ExternalId,
) -> Result<(), AuthorError> {
    rewrite_frontmatter(lock, store, path, |mapping| {
        mapping.set(
            "external_id".to_owned(),
            Yaml::Scalar(Scalar::String(external_id.as_str().to_owned())),
        );
    })
}

/// Removes `external_id`, so the item can be created anew.
///
/// # Errors
///
/// [`AuthorError`] when the lock cannot be taken, the file cannot be read or
/// parsed, or the write fails.
pub fn clear_external_id(
    lock: &dyn ExclusiveLock,
    store: &dyn AtomicWrite,
    path: &Path,
) -> Result<(), AuthorError> {
    rewrite_frontmatter(lock, store, path, |mapping| {
        *mapping = std::mem::take(mapping)
            .into_iter()
            .filter(|(key, _)| key != "external_id")
            .collect();
    })
}

fn rewrite_frontmatter(
    lock: &dyn ExclusiveLock,
    store: &dyn AtomicWrite,
    path: &Path,
    change: impl FnOnce(&mut Mapping),
) -> Result<(), AuthorError> {
    let _held = lock
        .acquire(&LockName::ForFile(path.to_path_buf()))
        .map_err(failed)?;
    let content = std::fs::read_to_string(path).map_err(failed)?;
    let mut yaml = document::parse(&content).map_err(failed)?;
    let Yaml::Mapping(mapping) = &mut yaml else {
        return Err(failed("frontmatter is not a mapping").into());
    };
    change(mapping);
    let rendered = document::render(Some(&content), &yaml).map_err(failed)?;
    store.write(path, rendered.as_bytes())?;
    Ok(())
}

fn failed(message: impl std::fmt::Display) -> kernel::Error {
    kernel::Error::Failed(message.to_string())
}

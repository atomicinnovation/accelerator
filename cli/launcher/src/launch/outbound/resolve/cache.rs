//! The on-disk binary cache, keyed by `{name}-{version}-{sha256}` with a
//! `.minisig` sibling; the checksum in the name lets a hit resolve offline.
//!
//! Writes are atomic (temp-in-dir then rename), and a replacement renames a
//! fresh inode over any corrupt entry so a process mid-`exec` never hits
//! `ETXTBSY`. The version-scoped cache needs no eviction.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use store::TEMP_PREFIX;

use crate::launch::core::ResolutionError;

// Per-process temp uniqueness; the PID alone collides across threads.
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// A located cache entry: the binary and its detached signature.
pub struct CachedBinary {
    pub path: PathBuf,
    pub sha256: String,
    pub signature_path: PathBuf,
}

fn is_sha256_hex(candidate: &str) -> bool {
    candidate.len() == 64 && candidate.bytes().all(|b| b.is_ascii_hexdigit())
}

fn stem(name: &str, version: &str, sha256: &str) -> String {
    format!("{name}-{version}-{sha256}")
}

fn signature_name(stem: &str) -> String {
    format!("{stem}.minisig")
}

fn temp_name(stem: &str, unique: &str, suffix: &str) -> String {
    format!("{TEMP_PREFIX}{stem}-{unique}{suffix}")
}

fn cache_error(path: &Path, error: &std::io::Error) -> ResolutionError {
    ResolutionError::Cache {
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}

/// Find a cached binary for `name`+`version` by prefix scan. Returns the entry
/// only if its signature sidecar is also present.
#[must_use]
pub fn find(root: &Path, name: &str, version: &str) -> Option<CachedBinary> {
    let prefix = format!("{name}-{version}-");
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file = file_name.to_str()?;
        let Some(sha) = file.strip_prefix(&prefix) else {
            continue;
        };
        if !is_sha256_hex(sha) {
            continue;
        }
        let signature_path = root.join(signature_name(file));
        if signature_path.exists() {
            return Some(CachedBinary {
                path: entry.path(),
                sha256: sha.to_owned(),
                signature_path,
            });
        }
    }
    None
}

/// Atomically store an already-verified binary + its signature. The caller must
/// have verified `bytes` before calling — only verified bytes reach the cache.
///
/// # Errors
///
/// [`ResolutionError::Cache`] on any IO failure.
pub fn store(
    root: &Path,
    name: &str,
    version: &str,
    sha256: &str,
    bytes: &[u8],
    signature: &str,
) -> Result<CachedBinary, ResolutionError> {
    std::fs::create_dir_all(root).map_err(|e| cache_error(root, &e))?;
    let stem = stem(name, version, sha256);
    let final_path = root.join(&stem);
    let signature_path = root.join(signature_name(&stem));

    // Temp inside the cache dir so the rename is intra-filesystem (no EXDEV).
    let unique = format!(
        "{}-{}",
        std::process::id(),
        TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let temp_binary = root.join(temp_name(&stem, &unique, ""));
    let temp_signature = root.join(temp_name(&stem, &unique, ".minisig"));

    write_then_rename(&temp_binary, &final_path, bytes, true)?;
    write_then_rename(
        &temp_signature,
        &signature_path,
        signature.as_bytes(),
        false,
    )?;

    Ok(CachedBinary {
        path: final_path,
        sha256: sha256.to_owned(),
        signature_path,
    })
}

fn write_then_rename(
    temp: &Path,
    final_path: &Path,
    bytes: &[u8],
    executable: bool,
) -> Result<(), ResolutionError> {
    // 0600 so bytes are never other-readable before the entry is published.
    write_private(temp, bytes)?;
    if executable {
        set_executable(temp)?;
    }
    std::fs::rename(temp, final_path).map_err(|e| {
        let _ = std::fs::remove_file(temp);
        cache_error(final_path, &e)
    })
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), ResolutionError> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| cache_error(path, &e))?;
    file.write_all(bytes).map_err(|e| cache_error(path, &e))
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), ResolutionError> {
    std::fs::write(path, bytes).map_err(|e| cache_error(path, &e))
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), ResolutionError> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| cache_error(path, &e))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), ResolutionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{find, store, temp_name, TEMP_PREFIX};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn temp_names_begin_with_the_store_temp_prefix() {
        assert!(temp_name("foo-1.0.0-abc", "9-0", "").starts_with(TEMP_PREFIX));
        assert!(temp_name("foo-1.0.0-abc", "9-0", ".minisig")
            .starts_with(TEMP_PREFIX));
    }

    fn tempdir() -> Result<PathBuf, Box<dyn Error>> {
        let dir = std::env::temp_dir().join(format!(
            "acc-cache-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    const SHA: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";

    #[test]
    fn store_then_find_round_trips() -> Result<(), Box<dyn Error>> {
        let root = tempdir()?;
        store(&root, "foo", "1.0.0", SHA, b"binary", "sig")?;
        let found = find(&root, "foo", "1.0.0").ok_or("not found")?;
        assert_eq!(found.sha256, SHA);
        assert_eq!(std::fs::read(&found.path)?, b"binary");
        assert!(found.signature_path.exists());
        Ok(())
    }

    #[test]
    fn find_ignores_an_entry_missing_its_signature(
    ) -> Result<(), Box<dyn Error>> {
        let root = tempdir()?;
        std::fs::write(root.join(format!("foo-1.0.0-{SHA}")), b"x")?;
        assert!(find(&root, "foo", "1.0.0").is_none());
        Ok(())
    }

    #[test]
    fn a_re_store_replaces_the_entry_in_place() -> Result<(), Box<dyn Error>> {
        let root = tempdir()?;
        store(&root, "foo", "1.0.0", SHA, b"binary", "sig")?;
        let path = find(&root, "foo", "1.0.0").ok_or("not found")?.path;
        std::fs::write(&path, b"poisoned")?;
        store(&root, "foo", "1.0.0", SHA, b"binary", "sig")?;
        assert_eq!(std::fs::read(&path)?, b"binary");
        Ok(())
    }
}

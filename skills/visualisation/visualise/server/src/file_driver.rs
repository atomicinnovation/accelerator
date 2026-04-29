use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::docs::DocTypeKey;
use crate::patcher::{self, FrontmatterPatch, PatchError};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub bytes: Vec<u8>,
    pub etag: String,
    pub mtime_ms: i64,
    pub size: u64,
}

const MAX_DOC_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum FileDriverError {
    #[error("configured doc-type path missing: {kind:?}")]
    TypeNotConfigured { kind: DocTypeKey },
    #[error("path escapes the configured root: {path}")]
    PathEscape { path: PathBuf },
    #[error("not found: {path}")]
    NotFound { path: PathBuf },
    #[error("file too large: {path} is {size} bytes (limit {limit})")]
    TooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("etag mismatch: current etag is {current}")]
    EtagMismatch { current: String },
    #[error("patch failed: {0}")]
    Patch(#[source] PatchError),
    #[error("path is not in a writable root: {path}")]
    PathNotWritable { path: PathBuf },
    #[error("cross-filesystem rename not supported for {path}")]
    CrossFilesystem { path: PathBuf },
}

pub trait FileDriver: Send + Sync {
    fn list(
        &self,
        kind: DocTypeKey,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<PathBuf>, FileDriverError>> + Send + '_>>;

    fn read(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<FileContent, FileDriverError>> + Send + '_>>;

    fn write_frontmatter(
        &self,
        path: &Path,
        patch: FrontmatterPatch,
        if_match: &str,
        on_committed: Box<dyn FnOnce(&Path) + Send>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<FileContent, FileDriverError>> + Send + '_>>;

    fn kind_for_canonical_path(&self, path: &Path) -> Option<DocTypeKey>;
}

pub struct LocalFileDriver {
    roots: HashMap<DocTypeKey, PathBuf>,
    extra_roots: Vec<PathBuf>,
    writable_roots: Vec<PathBuf>,
    path_locks: Arc<std::sync::Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>>,
}

impl LocalFileDriver {
    pub fn new(
        doc_paths: &HashMap<String, PathBuf>,
        extra_roots: Vec<PathBuf>,
        writable_roots: Vec<PathBuf>,
    ) -> Self {
        let mut roots = HashMap::new();
        for kind in DocTypeKey::all() {
            let Some(cfg_key) = kind.config_path_key() else {
                continue;
            };
            let Some(raw) = doc_paths.get(cfg_key) else {
                continue;
            };
            let canonical = std::fs::canonicalize(raw).unwrap_or_else(|_| raw.clone());
            roots.insert(kind, canonical);
        }
        let extra_roots = extra_roots
            .into_iter()
            .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
            .collect();
        let writable_roots = writable_roots
            .into_iter()
            .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
            .collect();
        Self {
            roots,
            extra_roots,
            writable_roots,
            path_locks: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    fn root_for(&self, kind: DocTypeKey) -> Result<&Path, FileDriverError> {
        self.roots
            .get(&kind)
            .map(|p| p.as_path())
            .ok_or(FileDriverError::TypeNotConfigured { kind })
    }

    fn path_is_allowed(&self, canonical_path: &Path) -> bool {
        self.roots
            .values()
            .any(|root| canonical_path.starts_with(root))
            || self
                .extra_roots
                .iter()
                .any(|r| canonical_path.starts_with(r))
    }

    async fn acquire_write_lock(&self, canonical: &Path) -> tokio::sync::OwnedMutexGuard<()> {
        let per_path = {
            let mut map = self.path_locks.lock().unwrap();
            map.entry(canonical.to_path_buf())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        per_path.lock_owned().await
    }

    async fn read_and_check_etag(
        &self,
        canonical: &Path,
        if_match: &str,
    ) -> Result<Vec<u8>, FileDriverError> {
        let bytes = tokio::fs::read(canonical)
            .await
            .map_err(|source| FileDriverError::Io {
                path: canonical.to_path_buf(),
                source,
            })?;
        let current = etag_of(&bytes);
        let stripped = if_match.trim_matches('"');
        if stripped != current {
            return Err(FileDriverError::EtagMismatch { current });
        }
        Ok(bytes)
    }

    async fn atomic_write_preserving_perms(
        canonical: PathBuf,
        new_bytes: Vec<u8>,
        original_perms: std::fs::Permissions,
    ) -> Result<(), FileDriverError> {
        use std::io::Write as _;

        let canonical_clone = canonical.clone();

        tokio::task::spawn_blocking(move || -> Result<(), FileDriverError> {
            let parent = canonical_clone
                .parent()
                .unwrap_or(&canonical_clone)
                .to_path_buf();

            let mut tmp =
                tempfile::NamedTempFile::new_in(&parent).map_err(|source| FileDriverError::Io {
                    path: parent.clone(),
                    source,
                })?;

            tmp.write_all(&new_bytes)
                .map_err(|source| FileDriverError::Io {
                    path: parent.clone(),
                    source,
                })?;

            tmp.as_file()
                .sync_all()
                .map_err(|source| FileDriverError::Io {
                    path: parent.clone(),
                    source,
                })?;

            std::fs::set_permissions(tmp.path(), original_perms).map_err(|source| {
                FileDriverError::Io {
                    path: tmp.path().to_path_buf(),
                    source,
                }
            })?;

            tmp.persist(&canonical_clone).map_err(|e| {
                if e.error.raw_os_error() == Some(libc::EXDEV) {
                    FileDriverError::CrossFilesystem {
                        path: canonical_clone.clone(),
                    }
                } else {
                    FileDriverError::Io {
                        path: canonical_clone.clone(),
                        source: e.error,
                    }
                }
            })?;

            let dir = std::fs::File::open(&parent).map_err(|source| FileDriverError::Io {
                path: parent.clone(),
                source,
            })?;
            dir.sync_all().map_err(|source| FileDriverError::Io {
                path: parent.clone(),
                source,
            })?;

            Ok(())
        })
        .await
        .map_err(|e| FileDriverError::Io {
            path: canonical,
            source: std::io::Error::other(e),
        })?
    }
}

impl FileDriver for LocalFileDriver {
    fn list(
        &self,
        kind: DocTypeKey,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<PathBuf>, FileDriverError>> + Send + '_>>
    {
        let root = match self.root_for(kind) {
            Ok(r) => r.to_path_buf(),
            Err(e) => return Box::pin(std::future::ready(Err(e))),
        };
        Box::pin(async move {
            let read = match tokio::fs::read_dir(&root).await {
                Ok(r) => r,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(vec![]);
                }
                Err(source) => return Err(FileDriverError::Io { path: root, source }),
            };
            let mut entries = Vec::new();
            let mut stream = read;
            loop {
                let entry = match stream.next_entry().await {
                    Ok(Some(e)) => e,
                    Ok(None) => break,
                    Err(source) => {
                        return Err(FileDriverError::Io {
                            path: root.clone(),
                            source,
                        });
                    }
                };
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("md") {
                    continue;
                }
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(source) => {
                        return Err(FileDriverError::Io { path, source });
                    }
                };
                if !file_type.is_file() {
                    continue;
                }
                entries.push(path);
            }
            Ok(entries)
        })
    }

    fn read(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<FileContent, FileDriverError>> + Send + '_>>
    {
        let path = path.to_path_buf();
        Box::pin(async move {
            let canonical = tokio::fs::canonicalize(&path).await.map_err(|source| {
                if source.kind() == std::io::ErrorKind::NotFound {
                    FileDriverError::NotFound { path: path.clone() }
                } else {
                    FileDriverError::Io {
                        path: path.clone(),
                        source,
                    }
                }
            })?;
            if !self.path_is_allowed(&canonical) {
                return Err(FileDriverError::PathEscape { path });
            }
            let meta =
                tokio::fs::metadata(&canonical)
                    .await
                    .map_err(|source| FileDriverError::Io {
                        path: canonical.clone(),
                        source,
                    })?;
            if meta.len() > MAX_DOC_BYTES {
                return Err(FileDriverError::TooLarge {
                    path,
                    size: meta.len(),
                    limit: MAX_DOC_BYTES,
                });
            }
            let bytes = tokio::fs::read(&canonical).await.map_err(|source| {
                if source.kind() == std::io::ErrorKind::NotFound {
                    FileDriverError::NotFound {
                        path: canonical.clone(),
                    }
                } else {
                    FileDriverError::Io {
                        path: canonical.clone(),
                        source,
                    }
                }
            })?;
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            let etag = etag_of(&bytes);
            Ok(FileContent {
                bytes,
                etag,
                mtime_ms,
                size: meta.len(),
            })
        })
    }

    fn write_frontmatter(
        &self,
        path: &Path,
        patch: FrontmatterPatch,
        if_match: &str,
        on_committed: Box<dyn FnOnce(&Path) + Send>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<FileContent, FileDriverError>> + Send + '_>>
    {
        let path = path.to_path_buf();
        let if_match = if_match.to_string();
        Box::pin(async move {
            // Canonicalize
            let canonical = tokio::fs::canonicalize(&path).await.map_err(|source| {
                if source.kind() == std::io::ErrorKind::NotFound {
                    FileDriverError::NotFound { path: path.clone() }
                } else {
                    FileDriverError::Io {
                        path: path.clone(),
                        source,
                    }
                }
            })?;

            // Writable-root check
            let writable = self.writable_roots.iter().any(|r| canonical.starts_with(r));
            if !writable {
                return Err(FileDriverError::PathNotWritable { path: canonical });
            }

            // Acquire per-path mutex (TOCTOU safety)
            let _guard = self.acquire_write_lock(&canonical).await;

            // Read original permissions before any mutation
            let original_perms = tokio::fs::metadata(&canonical)
                .await
                .map_err(|source| FileDriverError::Io {
                    path: canonical.clone(),
                    source,
                })?
                .permissions();

            // Read bytes and verify etag
            let bytes = self.read_and_check_etag(&canonical, &if_match).await?;

            // Apply the patch
            let new_bytes = patcher::apply(&bytes, patch).map_err(FileDriverError::Patch)?;

            // Idempotent short-circuit: no write needed
            if new_bytes == bytes {
                let mtime_ms = tokio::fs::metadata(&canonical)
                    .await
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                let size = bytes.len() as u64;
                return Ok(FileContent {
                    bytes,
                    etag: etag_of(&new_bytes),
                    mtime_ms,
                    size,
                });
            }

            // Atomic write with permission preservation
            Self::atomic_write_preserving_perms(
                canonical.clone(),
                new_bytes.clone(),
                original_perms,
            )
            .await?;

            // Invoke callback while still holding the per-path lock
            on_committed(&canonical);

            let meta =
                tokio::fs::metadata(&canonical)
                    .await
                    .map_err(|source| FileDriverError::Io {
                        path: canonical.clone(),
                        source,
                    })?;
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            let size = new_bytes.len() as u64;
            let new_etag = etag_of(&new_bytes);

            Ok(FileContent {
                bytes: new_bytes,
                etag: new_etag,
                mtime_ms,
                size,
            })
            // _guard dropped here: per-path lock released
        })
    }

    fn kind_for_canonical_path(&self, path: &Path) -> Option<DocTypeKey> {
        self.roots
            .iter()
            .find(|(_, root)| path.starts_with(*root))
            .map(|(kind, _)| *kind)
    }
}

pub fn etag_of(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("sha256-{}", hex::encode(h.finalize()))
}

pub fn template_extra_roots(
    templates: &HashMap<String, crate::config::TemplateTiers>,
) -> Vec<PathBuf> {
    let mut dirs = std::collections::HashSet::new();
    for tiers in templates.values() {
        if let Some(co) = &tiers.config_override {
            if let Some(p) = co.parent() {
                dirs.insert(p.to_path_buf());
            }
        }
        if let Some(p) = tiers.user_override.parent() {
            dirs.insert(p.to_path_buf());
        }
        if let Some(p) = tiers.plugin_default.parent() {
            dirs.insert(p.to_path_buf());
        }
    }
    dirs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn seeded_driver(tmp: &Path) -> LocalFileDriver {
        let dec = tmp.join("decisions");
        std::fs::create_dir_all(&dec).unwrap();
        std::fs::write(dec.join("ADR-0001-foo.md"), "# Foo\n").unwrap();
        std::fs::write(dec.join("ADR-0002-bar.md"), "# Bar\n").unwrap();
        std::fs::write(dec.join(".gitkeep"), "").unwrap();
        let plans = tmp.join("plans");

        let mut map = HashMap::new();
        map.insert("decisions".into(), dec.clone());
        map.insert("plans".into(), plans);
        LocalFileDriver::new(&map, vec![], vec![])
    }

    #[tokio::test]
    async fn list_returns_only_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let mut got = d.list(DocTypeKey::Decisions).await.unwrap();
        got.sort();
        assert_eq!(got.len(), 2);
        for p in &got {
            assert!(p.to_string_lossy().ends_with(".md"));
        }
    }

    #[tokio::test]
    async fn list_missing_directory_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let got = d.list(DocTypeKey::Plans).await.unwrap();
        assert!(got.is_empty(), "missing dir must not be an error");
    }

    #[tokio::test]
    async fn list_unconfigured_type_is_not_configured_error() {
        let d = LocalFileDriver::new(&HashMap::new(), vec![], vec![]);
        let err = d.list(DocTypeKey::Notes).await.unwrap_err();
        assert!(matches!(err, FileDriverError::TypeNotConfigured { .. }));
    }

    #[tokio::test]
    async fn read_returns_bytes_and_etag() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let p = tmp.path().join("decisions").join("ADR-0001-foo.md");
        let content = d.read(&p).await.unwrap();
        assert_eq!(content.bytes, b"# Foo\n");
        assert_eq!(content.size, 6);
        assert!(content.etag.starts_with("sha256-"));
        assert_eq!(content.etag.len(), "sha256-".len() + 64);
    }

    #[tokio::test]
    async fn read_rejects_path_outside_any_configured_root() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let outside = tmp.path().join("outside.md");
        std::fs::write(&outside, "x").unwrap();
        let err = d.read(&outside).await.unwrap_err();
        assert!(
            matches!(err, FileDriverError::PathEscape { .. }),
            "got {err:?}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn read_rejects_symlink_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let outside = tmp.path().join("secret.txt");
        std::fs::write(&outside, "s3cret").unwrap();
        let dec = tmp.path().join("decisions");
        let link = dec.join("escape.md");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let err = d.read(&link).await.unwrap_err();
        assert!(
            matches!(err, FileDriverError::PathEscape { .. }),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn read_missing_file_is_notfound() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let err = d
            .read(&tmp.path().join("decisions").join("nope.md"))
            .await
            .unwrap_err();
        assert!(matches!(err, FileDriverError::NotFound { .. }));
    }

    #[test]
    fn etag_is_stable_hex_sha256() {
        let e = etag_of(b"hello world");
        assert_eq!(
            e,
            "sha256-b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}

#[cfg(test)]
mod write_tests {
    use super::*;
    use crate::patcher::{FrontmatterPatch, TicketStatus};

    fn seeded_write_driver(tmp: &Path) -> LocalFileDriver {
        let tickets = tmp.join("tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(
            tickets.join("0001-foo.md"),
            "---\ntitle: Foo\nstatus: todo\n---\n# body\n",
        )
        .unwrap();
        let plans = tmp.join("plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::write(
            plans.join("2026-01-01-plan.md"),
            "---\ntitle: Plan\nstatus: draft\n---\n# body\n",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert("tickets".into(), tickets.clone());
        map.insert("plans".into(), plans);
        LocalFileDriver::new(&map, vec![], vec![tickets])
    }

    // ── Step 2.1 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn writes_status_to_disk_atomically() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let path = tmp.path().join("tickets").join("0001-foo.md");
        let content = d.read(&path).await.unwrap();

        d.write_frontmatter(
            &path,
            FrontmatterPatch::Status(TicketStatus::InProgress),
            &content.etag,
            Box::new(|_| {}),
        )
        .await
        .unwrap();

        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains("status: in-progress"),
            "expected status: in-progress in:\n{on_disk}"
        );
    }

    // ── Step 2.2 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn returns_new_etag_and_mtime_in_filecontent() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let path = tmp.path().join("tickets").join("0001-foo.md");
        let before = d.read(&path).await.unwrap();

        let after = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status(TicketStatus::InProgress),
                &before.etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap();

        assert_ne!(after.etag, before.etag, "etag must change after write");
        let new_bytes = std::fs::read(&path).unwrap();
        assert_eq!(after.etag, etag_of(&new_bytes));
    }

    // ── Step 2.3 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn preserves_other_frontmatter_and_body() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let path = tmp.path().join("tickets").join("0001-foo.md");
        let before = d.read(&path).await.unwrap();

        d.write_frontmatter(
            &path,
            FrontmatterPatch::Status(TicketStatus::Done),
            &before.etag,
            Box::new(|_| {}),
        )
        .await
        .unwrap();

        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("title: Foo"), "title must be preserved");
        assert!(on_disk.contains("# body"), "body must be preserved");
        assert!(on_disk.contains("status: done"), "status must be updated");
    }

    // ── Step 2.4 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn rejects_etag_mismatch_with_current_etag() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let path = tmp.path().join("tickets").join("0001-foo.md");
        let original = d.read(&path).await.unwrap();

        // Out-of-band edit
        std::fs::write(&path, "---\ntitle: Foo\nstatus: done\n---\n# body\n").unwrap();

        let err = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status(TicketStatus::InProgress),
                &original.etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap_err();

        match &err {
            FileDriverError::EtagMismatch { current } => {
                let new_bytes = std::fs::read(&path).unwrap();
                assert_eq!(*current, etag_of(&new_bytes));
            }
            other => panic!("expected EtagMismatch, got {other:?}"),
        }

        // On-disk content unchanged (still the out-of-band edit)
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("status: done"));
    }

    // ── Step 2.5 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn rejects_path_outside_tickets_root() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let plans_path = tmp.path().join("plans").join("2026-01-01-plan.md");
        let content = d.read(&plans_path).await.unwrap();

        let err = d
            .write_frontmatter(
                &plans_path,
                FrontmatterPatch::Status(TicketStatus::InProgress),
                &content.etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(err, FileDriverError::PathNotWritable { .. }),
            "expected PathNotWritable, got {err:?}"
        );

        // Plans file unchanged
        let on_disk = std::fs::read_to_string(&plans_path).unwrap();
        assert!(on_disk.contains("status: draft"));
    }

    // ── Step 2.6 ─────────────────────────────────────────────────────────────
    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_path_escape_via_symlink() {
        // Create a symlink inside tickets/ pointing at a sibling plans/ file.
        // Canonicalisation resolves the symlink; the canonical path is outside
        // writable_roots, so PathNotWritable must be returned.
        // (Windows symlinks require elevated privileges, so this test is Unix-only.)
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());

        let plans_path = tmp.path().join("plans").join("2026-01-01-plan.md");
        let link_path = tmp.path().join("tickets").join("sneaky.md");
        std::os::unix::fs::symlink(&plans_path, &link_path).unwrap();

        // Read the symlink to get an etag (goes through read which allows extra_roots)
        // Actually we need to bypass the read check — just get the raw etag
        let raw = std::fs::read(&plans_path).unwrap();
        let etag = etag_of(&raw);

        let err = d
            .write_frontmatter(
                &link_path,
                FrontmatterPatch::Status(TicketStatus::Done),
                &etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(err, FileDriverError::PathNotWritable { .. }),
            "expected PathNotWritable for symlink escape, got {err:?}"
        );
    }

    // ── Step 2.7 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn propagates_patcher_error_when_frontmatter_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let tickets = tmp.path().join("tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(tickets.join("0001-no-fm.md"), "# Heading\nno frontmatter\n").unwrap();

        let mut map = HashMap::new();
        map.insert("tickets".into(), tickets.clone());
        let d = LocalFileDriver::new(&map, vec![], vec![tickets.clone()]);

        let path = tickets.join("0001-no-fm.md");
        let raw = std::fs::read(&path).unwrap();
        let etag = etag_of(&raw);

        let err = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status(TicketStatus::Done),
                &etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(
                err,
                FileDriverError::Patch(crate::patcher::PatchError::FrontmatterAbsent)
            ),
            "expected Patch(FrontmatterAbsent), got {err:?}"
        );
    }

    // ── Step 2.8 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn propagates_patcher_error_for_missing_status_key() {
        let tmp = tempfile::tempdir().unwrap();
        let tickets = tmp.path().join("tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(
            tickets.join("0001-no-status.md"),
            "---\ntitle: Foo\n---\n# body\n",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert("tickets".into(), tickets.clone());
        let d = LocalFileDriver::new(&map, vec![], vec![tickets.clone()]);

        let path = tickets.join("0001-no-status.md");
        let raw = std::fs::read(&path).unwrap();
        let etag = etag_of(&raw);

        let err = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status(TicketStatus::Done),
                &etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(
                err,
                FileDriverError::Patch(crate::patcher::PatchError::KeyNotFound)
            ),
            "expected Patch(KeyNotFound), got {err:?}"
        );
    }

    // ── Step 2.9 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn concurrent_writes_with_same_if_match_one_returns_etag_mismatch() {
        use std::sync::Arc as StdArc;
        use tokio::sync::Barrier;

        let tmp = tempfile::tempdir().unwrap();
        let tickets = tmp.path().join("tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(
            tickets.join("0001-foo.md"),
            "---\ntitle: Foo\nstatus: todo\n---\n# body\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("tickets".into(), tickets.clone());
        let d = StdArc::new(LocalFileDriver::new(&map, vec![], vec![tickets.clone()]));

        let path = tickets.join("0001-foo.md");
        let etag = {
            let raw = std::fs::read(&path).unwrap();
            etag_of(&raw)
        };

        let barrier = StdArc::new(Barrier::new(2));
        let mut handles = vec![];

        for status in [TicketStatus::InProgress, TicketStatus::Done] {
            let d2 = d.clone();
            let p2 = path.clone();
            let e2 = etag.clone();
            let b2 = barrier.clone();
            handles.push(tokio::spawn(async move {
                b2.wait().await;
                d2.write_frontmatter(&p2, FrontmatterPatch::Status(status), &e2, Box::new(|_| {}))
                    .await
            }));
        }

        let results: Vec<_> = futures_collect(handles).await;
        let ok_count = results.iter().filter(|r| r.is_ok()).count();
        let mismatch_count = results
            .iter()
            .filter(|r| matches!(r, Err(FileDriverError::EtagMismatch { .. })))
            .count();

        assert_eq!(ok_count, 1, "exactly one write should succeed");
        assert_eq!(
            mismatch_count, 1,
            "exactly one write should get EtagMismatch"
        );
    }

    // ── Step 2.10 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn tolerates_quoted_etag_in_if_match() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let path = tmp.path().join("tickets").join("0001-foo.md");
        let raw = std::fs::read(&path).unwrap();
        let bare_etag = etag_of(&raw);

        // Both bare and quoted forms should succeed
        let r1 = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status(TicketStatus::InProgress),
                &bare_etag,
                Box::new(|_| {}),
            )
            .await;
        assert!(r1.is_ok(), "bare etag should be accepted: {r1:?}");

        // Re-read and use quoted form
        let raw2 = std::fs::read(&path).unwrap();
        let etag2 = etag_of(&raw2);
        let quoted2 = format!("\"{etag2}\"");
        let r2 = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status(TicketStatus::Done),
                &quoted2,
                Box::new(|_| {}),
            )
            .await;
        assert!(r2.is_ok(), "quoted etag should be accepted: {r2:?}");
    }

    // ── Step 2.11 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn idempotent_same_value_short_circuits() {
        let tmp = tempfile::tempdir().unwrap();
        let tickets = tmp.path().join("tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(
            tickets.join("0001-foo.md"),
            "---\ntitle: Foo\nstatus: in-progress\n---\n# body\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("tickets".into(), tickets.clone());
        let d = LocalFileDriver::new(&map, vec![], vec![tickets.clone()]);

        let path = tickets.join("0001-foo.md");
        let raw = std::fs::read(&path).unwrap();
        let etag = etag_of(&raw);
        let mtime_before = std::fs::metadata(&path).unwrap().modified().unwrap();

        let result = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status(TicketStatus::InProgress),
                &etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap();

        // Etag must be unchanged (same content)
        assert_eq!(result.etag, etag);

        // mtime must be unchanged (no rename happened)
        let mtime_after = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(
            mtime_before, mtime_after,
            "mtime must not change for idempotent write"
        );
    }

    // ── Step 2.12 ────────────────────────────────────────────────────────────
    #[cfg(unix)]
    #[tokio::test]
    async fn preserves_unix_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let path = tmp.path().join("tickets").join("0001-foo.md");

        // Set mode to 0o644
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let raw = std::fs::read(&path).unwrap();
        let etag = etag_of(&raw);

        d.write_frontmatter(
            &path,
            FrontmatterPatch::Status(TicketStatus::Done),
            &etag,
            Box::new(|_| {}),
        )
        .await
        .unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644, "file mode must be preserved after write");
    }

    // ── Step 2.13 ────────────────────────────────────────────────────────────
    // Cross-filesystem persist is not easily simulated on macOS/CI;
    // marked #[ignore] and #[cfg(target_os = "linux")]. Opt-in when
    // a tmpfs mount is available at the path below.
    #[cfg(target_os = "linux")]
    #[ignore]
    #[tokio::test]
    async fn cross_filesystem_persist_returns_cross_filesystem_error() {
        // This test requires a tmpfs mounted separately; it is ignored by
        // default and must be run explicitly in environments where the
        // setup is available.
        panic!("set up a separate tmpfs mount and implement this test");
    }

    // Helper: collect JoinHandle results
    async fn futures_collect<T>(handles: Vec<tokio::task::JoinHandle<T>>) -> Vec<T> {
        let mut out = Vec::with_capacity(handles.len());
        for h in handles {
            out.push(h.await.expect("task panicked"));
        }
        out
    }
}

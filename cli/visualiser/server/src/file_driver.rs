use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use serde::Serialize;
use sha2::{Digest, Sha256};

use corpus_adapters::{patch_status, FileCorpusStore, PatchError};

use corpus::doc_type::{FilenameFilter, Listing};
use corpus::set::{MemberKind, SetMatcher, SetPlace};
use corpus::{AtomicWrite, DocTypeCatalogue, DocTypeKey, StoreError};

/// A frontmatter mutation applied through the file driver.
#[derive(Debug, Clone)]
pub enum FrontmatterPatch {
    Status(String),
}

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
    // PatchError is corpus_adapters::PatchError.
    #[error("path is not in a writable root: {path}")]
    PathNotWritable { path: PathBuf },
    #[error("cross-filesystem rename not supported for {path}")]
    CrossFilesystem { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetListing {
    pub directory: PathBuf,
    pub root: PathBuf,
    pub documents: Vec<String>,
    pub assets: Vec<AssetListing>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetListing {
    pub set_relative_path: String,
    pub size: u64,
    pub mtime_ms: i64,
}

pub trait FileDriver: Send + Sync {
    fn list(
        &self,
        kind: DocTypeKey,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<PathBuf>, FileDriverError>,
                > + Send
                + '_,
        >,
    >;

    fn list_sets(
        &self,
        kind: DocTypeKey,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<SetListing>, FileDriverError>,
                > + Send
                + '_,
        >,
    >;

    fn read(
        &self,
        path: &Path,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<FileContent, FileDriverError>,
                > + Send
                + '_,
        >,
    >;

    fn write_frontmatter(
        &self,
        path: &Path,
        patch: FrontmatterPatch,
        if_match: &str,
        on_committed: Box<dyn FnOnce(&Path) + Send>,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<FileContent, FileDriverError>,
                > + Send
                + '_,
        >,
    >;

    fn kind_for_canonical_path(&self, path: &Path) -> Option<DocTypeKey>;
}

pub struct LocalFileDriver {
    doc_types: Arc<DocTypeCatalogue>,
    roots: HashMap<DocTypeKey, PathBuf>,
    extra_roots: Vec<PathBuf>,
    writable_roots: Vec<PathBuf>,
    path_locks:
        Arc<std::sync::Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>>,
}

/// A doc root as the filesystem resolves it: canonical when it exists, else
/// its nearest existing ancestor canonicalised with the rest appended, so a
/// root that does not exist yet still resolves through any symlink above it.
#[must_use]
pub fn resolved_root(raw: &Path) -> PathBuf {
    raw.ancestors()
        .find_map(|ancestor| {
            let canonical = std::fs::canonicalize(ancestor).ok()?;
            let rest = raw.strip_prefix(ancestor).ok()?;
            Some(canonical.join(rest))
        })
        .unwrap_or_else(|| raw.to_path_buf())
}

/// Refuses doc roots of which one equals or contains another once each is
/// resolved through the filesystem, so a symlink cannot alias one root into
/// another.
///
/// # Errors
///
/// [`config::ConfigError::OverlappingDocRoots`] naming the first overlapping
/// pair.
pub fn refuse_overlapping_doc_roots(
    doc_types: &DocTypeCatalogue,
    doc_paths: &HashMap<String, PathBuf>,
) -> Result<(), config::ConfigError> {
    let roots: Vec<config::paths::DocTypeDir> = doc_types
        .doc_types()
        .iter()
        .filter_map(|&kind| {
            let path_key = kind.config_path_key()?;
            let raw = doc_paths.get(path_key)?;
            Some(config::paths::DocTypeDir {
                doc_type: kind.linkage_type_name().unwrap_or(kind.wire_token()),
                path_key,
                dir: resolved_root(raw).display().to_string(),
                blank_fallback: false,
            })
        })
        .collect();
    let paths: Vec<&Path> =
        roots.iter().map(|root| Path::new(&root.dir)).collect();
    config::paths::first_overlap(&paths).map_or(Ok(()), |overlap| {
        Err(config::ConfigError::OverlappingDocRoots {
            container: Box::new(roots[overlap.container].clone()),
            contained: Box::new(roots[overlap.contained].clone()),
        })
    })
}

impl LocalFileDriver {
    pub fn new(
        doc_types: &Arc<DocTypeCatalogue>,
        doc_paths: &HashMap<String, PathBuf>,
        extra_roots: Vec<PathBuf>,
        writable_roots: Vec<PathBuf>,
    ) -> Self {
        let mut roots = HashMap::new();
        for &kind in doc_types.doc_types() {
            let Some(cfg_key) = kind.config_path_key() else {
                continue;
            };
            let Some(raw) = doc_paths.get(cfg_key) else {
                continue;
            };
            roots.insert(kind, resolved_root(raw));
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
            doc_types: Arc::clone(doc_types),
            roots,
            extra_roots,
            writable_roots,
            path_locks: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    fn root_for(&self, kind: DocTypeKey) -> Result<&Path, FileDriverError> {
        self.roots
            .get(&kind)
            .map(std::path::PathBuf::as_path)
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

    async fn acquire_write_lock(
        &self,
        canonical: &Path,
    ) -> tokio::sync::OwnedMutexGuard<()> {
        let per_path = {
            // Poisoned only by a panic while held; nothing panics under this lock.
            #[allow(clippy::unwrap_used)]
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
        let bytes = tokio::fs::read(canonical).await.map_err(|source| {
            FileDriverError::Io {
                path: canonical.to_path_buf(),
                source,
            }
        })?;
        let current = etag_of(&bytes);
        let stripped = if_match.trim_matches('"');
        if stripped != current {
            return Err(FileDriverError::EtagMismatch { current });
        }
        Ok(bytes)
    }

    /// Commits `new_bytes` to `canonical` through the shared corpus store.
    ///
    /// An async façade over the synchronous [`FileCorpusStore`]/[`AtomicWrite`]
    /// port: the temp-write, mode preservation, containment check, atomic
    /// rename, and fsync of both the file and its parent directory all run on a
    /// `spawn_blocking` thread. `store_root` is the writable root the target was
    /// verified to sit under, and bounds the store's containment guard.
    async fn commit_write(
        store_root: PathBuf,
        canonical: PathBuf,
        new_bytes: Vec<u8>,
    ) -> Result<(), FileDriverError> {
        let target = canonical.clone();
        tokio::task::spawn_blocking(move || {
            FileCorpusStore::new(store_root)
                .write(&canonical, &new_bytes)
                .map_err(|error| store_error_to_fd(error, &canonical))
        })
        .await
        .map_err(|e| FileDriverError::Io {
            path: target,
            source: std::io::Error::other(e),
        })?
    }
}

impl LocalFileDriver {
    async fn set_listing(
        &self,
        matcher: &SetMatcher,
        directory: PathBuf,
    ) -> Result<SetListing, FileDriverError> {
        let mut listing = SetListing {
            root: directory.join(matcher.root()),
            directory,
            documents: Vec::new(),
            assets: Vec::new(),
        };
        let mut pending = vec![String::new()];
        while let Some(set_relative_dir) = pending.pop() {
            let dir = listing.directory.join(&set_relative_dir);
            for (name, metadata) in undotted_entries_of(&dir).await? {
                let set_relative = if set_relative_dir.is_empty() {
                    name
                } else {
                    format!("{set_relative_dir}/{name}")
                };
                if metadata.is_dir() {
                    if matcher.may_contain_members(&set_relative) {
                        pending.push(set_relative);
                    }
                    continue;
                }
                let Some(SetPlace::Member(kind)) =
                    matcher.classify(&set_relative)
                else {
                    continue;
                };
                let path = listing.directory.join(&set_relative);
                let Ok(canonical) = tokio::fs::canonicalize(&path).await else {
                    continue;
                };
                if !self.path_is_allowed(&canonical) {
                    continue;
                }
                let metadata = if metadata.is_symlink() {
                    match tokio::fs::metadata(&canonical).await {
                        Ok(target) if target.is_file() => target,
                        _ => continue,
                    }
                } else {
                    metadata
                };
                match kind {
                    MemberKind::Document => {
                        listing.documents.push(set_relative);
                    }
                    MemberKind::Asset => listing.assets.push(AssetListing {
                        set_relative_path: set_relative,
                        size: metadata.len(),
                        mtime_ms: mtime_ms_of(&metadata),
                    }),
                }
            }
        }
        listing.documents.sort();
        listing
            .assets
            .sort_by(|a, b| a.set_relative_path.cmp(&b.set_relative_path));
        Ok(listing)
    }
}

/// The non-dot directories directly in `root` that hold `set_root` as a
/// file; none when `root` does not exist.
async fn set_directories_in(
    root: &Path,
    set_root: &str,
) -> Result<Vec<PathBuf>, FileDriverError> {
    let mut directories = Vec::new();
    for (name, metadata) in undotted_entries_of(root).await? {
        if !metadata.is_dir() {
            continue;
        }
        let directory = root.join(name);
        match tokio::fs::metadata(directory.join(set_root)).await {
            Ok(meta) if meta.is_file() => directories.push(directory),
            _ => {}
        }
    }
    Ok(directories)
}

/// The entries of `dir` whose names do not start with a dot, each with the
/// metadata of the entry itself, so a symlink is never taken for a
/// directory; none when `dir` does not exist.
async fn undotted_entries_of(
    dir: &Path,
) -> Result<Vec<(String, std::fs::Metadata)>, FileDriverError> {
    let io_error = |source| FileDriverError::Io {
        path: dir.to_path_buf(),
        source,
    };
    let mut stream = match tokio::fs::read_dir(dir).await {
        Ok(read) => read,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(vec![]);
        }
        Err(source) => return Err(io_error(source)),
    };
    let mut entries = Vec::new();
    while let Some(entry) = stream.next_entry().await.map_err(io_error)? {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let metadata = tokio::fs::symlink_metadata(entry.path())
            .await
            .map_err(io_error)?;
        entries.push((name, metadata));
    }
    Ok(entries)
}

fn mtime_ms_of(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64)
}

fn store_error_to_fd(error: StoreError, path: &Path) -> FileDriverError {
    match error {
        StoreError::CrossFilesystem { .. } => {
            FileDriverError::CrossFilesystem {
                path: path.to_path_buf(),
            }
        }
        StoreError::NotWritable { .. } | StoreError::UnsafePath { .. } => {
            FileDriverError::PathNotWritable {
                path: path.to_path_buf(),
            }
        }
        other => FileDriverError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::other(other.to_string()),
        },
    }
}

impl FileDriver for LocalFileDriver {
    fn list(
        &self,
        kind: DocTypeKey,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<PathBuf>, FileDriverError>,
                > + Send
                + '_,
        >,
    > {
        let root = match self.root_for(kind) {
            Ok(r) => r.to_path_buf(),
            Err(e) => return Box::pin(std::future::ready(Err(e))),
        };
        let set_root = match kind.listing() {
            Listing::Flat {
                drafts_subdirectory,
                filename_filter,
            } => {
                let is_document = match filename_filter {
                    FilenameFilter::Markdown => has_md_extension,
                    FilenameFilter::WorkItem => is_work_item_file,
                };
                return Box::pin(async move {
                    let mut entries =
                        markdown_files_in(&root, is_document).await?;
                    if let Some(drafts) = drafts_subdirectory {
                        entries.extend(
                            markdown_files_in(&root.join(drafts), is_document)
                                .await?,
                        );
                    }
                    Ok(entries)
                });
            }
            Listing::Set => {
                self.doc_types.set_matcher(kind).map(SetMatcher::root)
            }
        };
        let Some(set_root) = set_root else {
            return Box::pin(std::future::ready(Ok(Vec::new())));
        };
        Box::pin(async move {
            let read = match tokio::fs::read_dir(&root).await {
                Ok(r) => r,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(vec![]);
                }
                Err(source) => {
                    return Err(FileDriverError::Io { path: root, source })
                }
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
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(source) => {
                        return Err(FileDriverError::Io { path, source });
                    }
                };

                // Each set lives at `<root>/<set-dir>/<set root>`.
                if !file_type.is_dir() {
                    continue;
                }
                // Skip dot-prefixed in-flight directories (e.g.
                // `.YYYY-MM-DD-HHMMSS-{source-id}.tmp/`).
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'))
                {
                    continue;
                }
                let candidate = path.join(set_root);
                match tokio::fs::metadata(&candidate).await {
                    Ok(meta) if meta.is_file() => entries.push(candidate),
                    _ => {}
                }
            }
            Ok(entries)
        })
    }

    fn list_sets(
        &self,
        kind: DocTypeKey,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<SetListing>, FileDriverError>,
                > + Send
                + '_,
        >,
    > {
        let root = match self.root_for(kind) {
            Ok(r) => r.to_path_buf(),
            Err(e) => return Box::pin(std::future::ready(Err(e))),
        };
        let Some(matcher) = self.doc_types.set_matcher(kind) else {
            return Box::pin(std::future::ready(Ok(Vec::new())));
        };
        Box::pin(async move {
            let mut sets = Vec::new();
            for directory in set_directories_in(&root, matcher.root()).await? {
                sets.push(self.set_listing(matcher, directory).await?);
            }
            sets.sort_by(|a, b| a.directory.cmp(&b.directory));
            Ok(sets)
        })
    }

    fn read(
        &self,
        path: &Path,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<FileContent, FileDriverError>,
                > + Send
                + '_,
        >,
    > {
        let path = path.to_path_buf();
        Box::pin(async move {
            let canonical =
                tokio::fs::canonicalize(&path).await.map_err(|source| {
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
                tokio::fs::metadata(&canonical).await.map_err(|source| {
                    FileDriverError::Io {
                        path: canonical.clone(),
                        source,
                    }
                })?;
            if meta.len() > MAX_DOC_BYTES {
                return Err(FileDriverError::TooLarge {
                    path,
                    size: meta.len(),
                    limit: MAX_DOC_BYTES,
                });
            }
            let bytes =
                tokio::fs::read(&canonical).await.map_err(|source| {
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
                .map_or(0, |d| d.as_millis() as i64);
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
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<FileContent, FileDriverError>,
                > + Send
                + '_,
        >,
    > {
        let path = path.to_path_buf();
        let if_match = if_match.to_string();
        Box::pin(async move {
            // Canonicalize
            let canonical =
                tokio::fs::canonicalize(&path).await.map_err(|source| {
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
            let store_root = self
                .writable_roots
                .iter()
                .find(|r| canonical.starts_with(r))
                .cloned();
            let Some(store_root) = store_root else {
                return Err(FileDriverError::PathNotWritable {
                    path: canonical,
                });
            };

            // Acquire per-path mutex (TOCTOU safety)
            let _guard = self.acquire_write_lock(&canonical).await;

            // Read bytes and verify etag
            let bytes = self.read_and_check_etag(&canonical, &if_match).await?;

            // Apply the patch
            let FrontmatterPatch::Status(new_status) = &patch;
            let new_bytes = patch_status(&bytes, new_status)
                .map_err(FileDriverError::Patch)?;

            // Idempotent short-circuit: no write needed
            if new_bytes == bytes {
                let mtime_ms = tokio::fs::metadata(&canonical)
                    .await
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_millis() as i64);
                let size = bytes.len() as u64;
                return Ok(FileContent {
                    bytes,
                    etag: etag_of(&new_bytes),
                    mtime_ms,
                    size,
                });
            }

            // Atomic write through the shared corpus store (preserves the
            // target's mode; fsyncs the file and its parent directory).
            Self::commit_write(
                store_root,
                canonical.clone(),
                new_bytes.clone(),
            )
            .await?;

            // Invoke callback while still holding the per-path lock
            on_committed(&canonical);

            let meta =
                tokio::fs::metadata(&canonical).await.map_err(|source| {
                    FileDriverError::Io {
                        path: canonical.clone(),
                        source,
                    }
                })?;
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis() as i64);
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

/// Where a work item no tracker has confirmed yet waits, beside the rest.
fn has_md_extension(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("md")
}

fn is_work_item_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(work::work_item_files::is_md_filename)
}

/// The markdown files directly in `dir`; none when it does not exist.
async fn markdown_files_in(
    dir: &Path,
    is_markdown: fn(&Path) -> bool,
) -> Result<Vec<PathBuf>, FileDriverError> {
    let mut stream = match tokio::fs::read_dir(dir).await {
        Ok(read) => read,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(vec![]);
        }
        Err(source) => {
            return Err(FileDriverError::Io {
                path: dir.to_path_buf(),
                source,
            })
        }
    };
    let mut entries = Vec::new();
    while let Some(entry) =
        stream
            .next_entry()
            .await
            .map_err(|source| FileDriverError::Io {
                path: dir.to_path_buf(),
                source,
            })?
    {
        let path = entry.path();
        if !is_markdown(&path) {
            continue;
        }
        let file_type =
            entry
                .file_type()
                .await
                .map_err(|source| FileDriverError::Io {
                    path: path.clone(),
                    source,
                })?;
        if file_type.is_file() {
            entries.push(path);
        }
    }
    Ok(entries)
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
        LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![],
        )
    }

    #[tokio::test]
    async fn list_returns_only_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let mut got = d.list(DocTypeKey::DECISIONS).await.unwrap();
        got.sort();
        assert_eq!(got.len(), 2);
        for p in &got {
            assert!(p.to_string_lossy().ends_with(".md"));
        }
    }

    #[tokio::test]
    async fn work_items_are_listed_with_the_drafts_beside_them() {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("work");
        let drafts = work.join("drafts");
        std::fs::create_dir_all(drafts.join("nested")).unwrap();
        std::fs::write(work.join("PP-1-item.md"), "# Item\n").unwrap();
        std::fs::write(drafts.join("draft-k7mq3x-idea.md"), "# Idea\n")
            .unwrap();
        std::fs::write(drafts.join("nested/deeper.md"), "# No\n").unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work);
        let d = LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![],
        );

        let mut got: Vec<String> = d
            .list(DocTypeKey::WORK_ITEMS)
            .await
            .unwrap()
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        got.sort();

        assert_eq!(got, vec!["PP-1-item.md", "draft-k7mq3x-idea.md"]);
    }

    fn seed_every_listing_shape(root: &Path) {
        for directory in ["", "drafts", "set", ".hidden-set", "rootless"] {
            std::fs::create_dir_all(root.join(directory)).unwrap();
        }
        for file in [
            "0001-item.md",
            "a-note.md",
            "notes.txt",
            "UPPER.MD",
            "drafts/UPPER.MD",
            "drafts/0002-draft.md",
            "drafts/a-draft.md",
            "set/manifest.md",
            "set/inventory.md",
            ".hidden-set/manifest.md",
            ".hidden-set/inventory.md",
            "rootless/other.md",
        ] {
            std::fs::write(root.join(file), "# x\n").unwrap();
        }
    }

    #[tokio::test]
    async fn every_built_in_doc_type_lists_by_its_declared_rule() {
        let expected: [(&str, &[&str]); 14] = [
            ("decisions", &["0001-item.md", "a-note.md"]),
            (
                "work-items",
                &[
                    "0001-item.md",
                    "UPPER.MD",
                    "a-note.md",
                    "drafts/0002-draft.md",
                    "drafts/UPPER.MD",
                    "drafts/a-draft.md",
                ],
            ),
            ("plans", &["0001-item.md", "a-note.md"]),
            ("codebase-research", &["0001-item.md", "a-note.md"]),
            ("plan-reviews", &["0001-item.md", "a-note.md"]),
            ("pr-reviews", &["0001-item.md", "a-note.md"]),
            ("work-item-reviews", &["0001-item.md", "a-note.md"]),
            ("validations", &["0001-item.md", "a-note.md"]),
            ("notes", &["0001-item.md", "a-note.md"]),
            ("pr-descriptions", &["0001-item.md", "a-note.md"]),
            ("design-gaps", &["0001-item.md", "a-note.md"]),
            ("design-inventories", &["set/inventory.md"]),
            ("topic-research", &["set/manifest.md"]),
            ("root-cause-analyses", &["0001-item.md", "a-note.md"]),
        ];
        let catalogue = crate::test_support::assembled_catalogue();
        let listed: Vec<&str> =
            expected.iter().map(|(token, _)| *token).collect();
        let configured: Vec<&str> = catalogue
            .doc_types()
            .iter()
            .filter(|kind| kind.config_path_key().is_some())
            .map(|kind| kind.wire_token())
            .collect();
        assert_eq!(listed, configured);
        for (token, files) in expected {
            let kind = catalogue.from_wire_token(token).unwrap();
            let tmp = tempfile::tempdir().unwrap();
            let root = std::fs::canonicalize(tmp.path()).unwrap();
            seed_every_listing_shape(&root);
            let mut map = HashMap::new();
            map.insert(
                kind.config_path_key().unwrap().to_owned(),
                root.clone(),
            );
            let driver = LocalFileDriver::new(
                &crate::test_support::assembled_catalogue(),
                &map,
                vec![],
                vec![],
            );
            let mut got: Vec<String> = driver
                .list(kind)
                .await
                .unwrap()
                .iter()
                .map(|path| {
                    path.strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            got.sort();
            assert_eq!(got, files, "{token}");
        }
    }

    fn set_driver(path_key: &str, root: &Path) -> LocalFileDriver {
        let mut map = HashMap::new();
        map.insert(path_key.to_owned(), root.to_path_buf());
        LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![],
        )
    }

    fn write_files(root: &Path, files: &[&str]) {
        for file in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "# x\n").unwrap();
        }
    }

    #[tokio::test]
    async fn list_sets_lists_each_set_with_the_members_its_shape_holds() {
        let tmp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        write_files(
            &root,
            &[
                "s/manifest.md",
                "s/brief.md",
                "s/findings/01-a.md",
                "s/findings/01-a.levels/2-1.md",
                "s/reports/r.md",
                "s/notes.txt",
                "s/findings/sub/x.md",
                "s/findings/.01-x-web.md.invalid",
                "s/.conduct-run.json",
                "s/.scratch/x.md",
                ".draft.tmp/manifest.md",
                "rootless/brief.md",
                "stray.md",
            ],
        );
        let driver = set_driver("research_topics", &root);

        let sets = driver.list_sets(DocTypeKey::TOPIC_RESEARCH).await.unwrap();

        assert_eq!(sets.len(), 1, "{sets:?}");
        assert_eq!(sets[0].directory, root.join("s"));
        assert_eq!(sets[0].root, root.join("s/manifest.md"));
        assert_eq!(
            sets[0].documents,
            vec![
                "brief.md",
                "findings/01-a.levels/2-1.md",
                "findings/01-a.md",
                "reports/r.md",
            ]
        );
        assert!(sets[0].assets.is_empty(), "{:?}", sets[0].assets);
    }

    #[tokio::test]
    async fn list_sets_carries_each_asset_with_its_size_and_mtime() {
        let tmp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        write_files(
            &root,
            &[
                "i/inventory.md",
                "i/prototype-full/x.html",
                "i/assets/y.svg",
                "i/notes.txt",
            ],
        );
        let screenshots = root.join("i/screenshots");
        std::fs::create_dir_all(&screenshots).unwrap();
        for (name, bytes, mtime_ms) in [
            ("a.png", "abc", 1_700_000_000_000_u64),
            ("Some File (v2).png", "abcdefg", 1_700_000_001_000),
        ] {
            let path = screenshots.join(name);
            std::fs::write(&path, bytes).unwrap();
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(
                    std::time::UNIX_EPOCH
                        + std::time::Duration::from_millis(mtime_ms),
                )
                .unwrap();
        }
        let driver = set_driver("research_design_inventories", &root);

        let sets = driver
            .list_sets(DocTypeKey::DESIGN_INVENTORIES)
            .await
            .unwrap();

        assert_eq!(sets.len(), 1, "{sets:?}");
        assert!(sets[0].documents.is_empty(), "{:?}", sets[0].documents);
        assert_eq!(
            sets[0].assets,
            vec![
                AssetListing {
                    set_relative_path: "screenshots/Some File (v2).png".into(),
                    size: 7,
                    mtime_ms: 1_700_000_001_000,
                },
                AssetListing {
                    set_relative_path: "screenshots/a.png".into(),
                    size: 3,
                    mtime_ms: 1_700_000_000_000,
                },
            ]
        );
    }

    #[tokio::test]
    async fn list_sets_of_a_missing_doc_root_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = set_driver("research_topics", &tmp.path().join("missing"));

        let sets = driver.list_sets(DocTypeKey::TOPIC_RESEARCH).await.unwrap();

        assert!(sets.is_empty(), "{sets:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_slotted_symlink_out_of_the_doc_roots_is_not_listed() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp = std::fs::canonicalize(tmp.path()).unwrap();
        let root = tmp.join("inventories");
        write_files(&root, &["i/inventory.md", "i/screenshots/kept.png"]);
        std::fs::write(tmp.join("secret.png"), "s3cret").unwrap();
        std::os::unix::fs::symlink(
            tmp.join("secret.png"),
            root.join("i/screenshots/escape.png"),
        )
        .unwrap();
        let driver = set_driver("research_design_inventories", &root);

        let sets = driver
            .list_sets(DocTypeKey::DESIGN_INVENTORIES)
            .await
            .unwrap();

        let assets: Vec<&str> = sets[0]
            .assets
            .iter()
            .map(|asset| asset.set_relative_path.as_str())
            .collect();
        assert_eq!(assets, vec!["screenshots/kept.png"]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_symlinked_slot_directory_is_not_walked() {
        let tmp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        write_files(&root, &["s/manifest.md", "elsewhere/findings/01-a.md"]);
        std::os::unix::fs::symlink(
            root.join("elsewhere/findings"),
            root.join("s/findings"),
        )
        .unwrap();
        let driver = set_driver("research_topics", &root);

        let sets = driver.list_sets(DocTypeKey::TOPIC_RESEARCH).await.unwrap();

        let set = sets
            .iter()
            .find(|set| set.directory == root.join("s"))
            .unwrap();
        assert!(set.documents.is_empty(), "{:?}", set.documents);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_symlinked_directory_named_like_a_member_is_not_listed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        write_files(&root, &["s/manifest.md", "s/findings/01-a.md"]);
        std::os::unix::fs::symlink(
            root.join("s/findings"),
            root.join("s/findings/02-b.md"),
        )
        .unwrap();
        let driver = set_driver("research_topics", &root);

        let sets = driver.list_sets(DocTypeKey::TOPIC_RESEARCH).await.unwrap();

        assert_eq!(sets[0].documents, vec!["findings/01-a.md"]);
    }

    #[tokio::test]
    async fn list_missing_directory_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let got = d.list(DocTypeKey::PLANS).await.unwrap();
        assert!(got.is_empty(), "missing dir must not be an error");
    }

    #[tokio::test]
    async fn list_unconfigured_type_is_not_configured_error() {
        let d = LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &HashMap::new(),
            vec![],
            vec![],
        );
        let err = d.list(DocTypeKey::NOTES).await.unwrap_err();
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

    fn seeded_write_driver(tmp: &Path) -> LocalFileDriver {
        let work = tmp.join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join("0001-foo.md"),
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
        map.insert("work".into(), work.clone());
        map.insert("plans".into(), plans);
        LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![work],
        )
    }

    // ── Step 2.1 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn writes_status_to_disk_atomically() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let path = tmp.path().join("work").join("0001-foo.md");
        let content = d.read(&path).await.unwrap();

        d.write_frontmatter(
            &path,
            FrontmatterPatch::Status("in-progress".to_string()),
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
        let path = tmp.path().join("work").join("0001-foo.md");
        let before = d.read(&path).await.unwrap();

        let after = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status("in-progress".to_string()),
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
        let path = tmp.path().join("work").join("0001-foo.md");
        let before = d.read(&path).await.unwrap();

        d.write_frontmatter(
            &path,
            FrontmatterPatch::Status("done".to_string()),
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
        let path = tmp.path().join("work").join("0001-foo.md");
        let original = d.read(&path).await.unwrap();

        // Out-of-band edit
        std::fs::write(&path, "---\ntitle: Foo\nstatus: done\n---\n# body\n")
            .unwrap();

        let err = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status("in-progress".to_string()),
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
    async fn rejects_path_outside_work_root() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());
        let plans_path = tmp.path().join("plans").join("2026-01-01-plan.md");
        let content = d.read(&plans_path).await.unwrap();

        let err = d
            .write_frontmatter(
                &plans_path,
                FrontmatterPatch::Status("in-progress".to_string()),
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
        // Create a symlink inside work/ pointing at a sibling plans/ file.
        // Canonicalisation resolves the symlink; the canonical path is outside
        // writable_roots, so PathNotWritable must be returned.
        // (Windows symlinks require elevated privileges, so this test is Unix-only.)
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_write_driver(tmp.path());

        let plans_path = tmp.path().join("plans").join("2026-01-01-plan.md");
        let link_path = tmp.path().join("work").join("sneaky.md");
        std::os::unix::fs::symlink(&plans_path, &link_path).unwrap();

        // Read the symlink to get an etag (goes through read which allows extra_roots)
        // Actually we need to bypass the read check — just get the raw etag
        let raw = std::fs::read(&plans_path).unwrap();
        let etag = etag_of(&raw);

        let err = d
            .write_frontmatter(
                &link_path,
                FrontmatterPatch::Status("done".to_string()),
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
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join("0001-no-fm.md"),
            "# Heading\nno frontmatter\n",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert("work".into(), work.clone());
        let d = LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![work.clone()],
        );

        let path = work.join("0001-no-fm.md");
        let raw = std::fs::read(&path).unwrap();
        let etag = etag_of(&raw);

        let err = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status("done".to_string()),
                &etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(
                err,
                FileDriverError::Patch(
                    corpus_adapters::PatchError::FrontmatterAbsent
                )
            ),
            "expected Patch(FrontmatterAbsent), got {err:?}"
        );
    }

    // ── Step 2.8 ─────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn propagates_patcher_error_for_missing_status_key() {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join("0001-no-status.md"),
            "---\ntitle: Foo\n---\n# body\n",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert("work".into(), work.clone());
        let d = LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![work.clone()],
        );

        let path = work.join("0001-no-status.md");
        let raw = std::fs::read(&path).unwrap();
        let etag = etag_of(&raw);

        let err = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status("done".to_string()),
                &etag,
                Box::new(|_| {}),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(
                err,
                FileDriverError::Patch(
                    corpus_adapters::PatchError::KeyNotFound
                )
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
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join("0001-foo.md"),
            "---\ntitle: Foo\nstatus: todo\n---\n# body\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work.clone());
        let d = StdArc::new(LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![work.clone()],
        ));

        let path = work.join("0001-foo.md");
        let etag = {
            let raw = std::fs::read(&path).unwrap();
            etag_of(&raw)
        };

        let barrier = StdArc::new(Barrier::new(2));
        let mut handles = vec![];

        for status in ["in-progress".to_string(), "done".to_string()] {
            let d2 = d.clone();
            let p2 = path.clone();
            let e2 = etag.clone();
            let b2 = barrier.clone();
            handles.push(tokio::spawn(async move {
                b2.wait().await;
                d2.write_frontmatter(
                    &p2,
                    FrontmatterPatch::Status(status),
                    &e2,
                    Box::new(|_| {}),
                )
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
        let path = tmp.path().join("work").join("0001-foo.md");
        let raw = std::fs::read(&path).unwrap();
        let bare_etag = etag_of(&raw);

        // Both bare and quoted forms should succeed
        let r1 = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status("in-progress".to_string()),
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
                FrontmatterPatch::Status("done".to_string()),
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
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join("0001-foo.md"),
            "---\ntitle: Foo\nstatus: in-progress\n---\n# body\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work.clone());
        let d = LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![work.clone()],
        );

        let path = work.join("0001-foo.md");
        let raw = std::fs::read(&path).unwrap();
        let etag = etag_of(&raw);
        let mtime_before =
            std::fs::metadata(&path).unwrap().modified().unwrap();

        let result = d
            .write_frontmatter(
                &path,
                FrontmatterPatch::Status("in-progress".to_string()),
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
        let path = tmp.path().join("work").join("0001-foo.md");

        // Set mode to 0o644
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .unwrap();

        let raw = std::fs::read(&path).unwrap();
        let etag = etag_of(&raw);

        d.write_frontmatter(
            &path,
            FrontmatterPatch::Status("done".to_string()),
            &etag,
            Box::new(|_| {}),
        )
        .await
        .unwrap();

        let mode =
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644, "file mode must be preserved after write");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn preserves_crlf_line_endings_and_non_default_mode() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let path = work.join("0001-foo.md");
        std::fs::write(
            &path,
            "---\r\ntitle: Foo\r\nstatus: todo\r\n---\r\n# body\r\n",
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))
            .unwrap();

        let mut map = HashMap::new();
        map.insert("work".into(), work.clone());
        let d = LocalFileDriver::new(
            &crate::test_support::assembled_catalogue(),
            &map,
            vec![],
            vec![work.clone()],
        );

        let etag = etag_of(&std::fs::read(&path).unwrap());
        d.write_frontmatter(
            &path,
            FrontmatterPatch::Status("done".to_string()),
            &etag,
            Box::new(|_| {}),
        )
        .await
        .unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(
            written.contains("status: done"),
            "status must be patched: {written:?}"
        );
        assert!(
            written.contains("\r\n") && !written.contains("\n\n"),
            "CRLF line endings must be preserved: {written:?}"
        );
        assert!(
            !written.replace("\r\n", "").contains('\n'),
            "no bare LF may be introduced: {written:?}"
        );
        let mode =
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "non-default file mode must be preserved");
    }

    // ── Step 2.13 ────────────────────────────────────────────────────────────
    // Cross-filesystem persist is not easily simulated on macOS/CI;
    // marked #[ignore] and #[cfg(target_os = "linux")]. Opt-in when
    // a tmpfs mount is available at the path below.
    #[cfg(target_os = "linux")]
    #[ignore = "requires a separate tmpfs mount; run explicitly where available"]
    #[tokio::test]
    async fn cross_filesystem_persist_returns_cross_filesystem_error() {
        // This test requires a tmpfs mounted separately; it is ignored by
        // default and must be run explicitly in environments where the
        // setup is available.
        panic!("set up a separate tmpfs mount and implement this test");
    }

    // Helper: collect JoinHandle results
    async fn futures_collect<T>(
        handles: Vec<tokio::task::JoinHandle<T>>,
    ) -> Vec<T> {
        let mut out = Vec::with_capacity(handles.len());
        for h in handles {
            out.push(h.await.expect("task panicked"));
        }
        out
    }
}

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::docs::DocTypeKey;

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
}

pub struct LocalFileDriver {
    roots: HashMap<DocTypeKey, PathBuf>,
    extra_roots: Vec<PathBuf>,
}

impl LocalFileDriver {
    pub fn new(doc_paths: &HashMap<String, PathBuf>, extra_roots: Vec<PathBuf>) -> Self {
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
        Self { roots, extra_roots }
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
        LocalFileDriver::new(&map, vec![])
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
        let d = LocalFileDriver::new(&HashMap::new(), vec![]);
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

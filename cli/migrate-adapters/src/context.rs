//! The composed `MigrationContext`: config access for doc-type directories,
//! the bounded atomic write every migration's mutation routes through, and
//! the injected corpus and sync-baseline capabilities.

use std::cell::OnceCell;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use config::Key;
use corpus::doc_type::DocTypeKey;
use migrate::ports::CorpusIndex;
use migrate::ports::DocTypeDir;
use migrate::ports::ManifestStore as _;
use migrate::ports::MigrationContext;
use migrate::ports::MigrationError;
use store::NewFileMode;
use store::WriteBounds;

use crate::corpus_index::FileCorpusIndex;
use crate::manifest_store::FileManifestStore;
use crate::merge_move::merge_move;

/// What a composition root lends every migration.
pub struct Capabilities<'a> {
    pub config: &'a dyn config::ConfigAccess,
    pub walker: &'a dyn corpus::scan::CorpusWalker,
    pub reader: &'a dyn corpus::scan::FileReader,
    pub frontmatter: &'a dyn corpus::frontmatter::FrontmatterParser,
    pub canonicaliser: &'a dyn corpus::work_item_id::WorkItemIdCanonicaliser,
    pub sync_baselines: &'a dyn migrate::ports::SyncBaselines,
}

pub struct FileMigrationContext<'a> {
    root: PathBuf,
    capabilities: Capabilities<'a>,
    fresh_mode: u32,
    index: OnceCell<FileCorpusIndex>,
    manifest: FileManifestStore,
}

impl<'a> FileMigrationContext<'a> {
    #[must_use]
    pub fn new(
        root: impl Into<PathBuf>,
        capabilities: Capabilities<'a>,
    ) -> Self {
        let root = root.into();
        Self {
            capabilities,
            fresh_mode: 0o666 & !store::current_umask(),
            index: OnceCell::new(),
            manifest: FileManifestStore::new(&root),
            root,
        }
    }

    fn bounds(&self) -> WriteBounds<'_> {
        WriteBounds {
            permitted_root: &self.root,
            project_root: &self.root,
        }
    }

    /// The `corpus::linkage`-shaped doc-type table: every configured
    /// doc-type directory, keyed by its [`DocTypeKey`] rather than its bare
    /// linkage-type-name string.
    fn linkage_table(&self) -> Vec<(DocTypeKey, PathBuf)> {
        MigrationContext::doc_type_dirs(self)
            .into_iter()
            .filter_map(|dir| {
                DocTypeKey::from_linkage_type_name(&dir.doc_type)
                    .map(|key| (key, dir.dir))
            })
            .collect()
    }

    fn resolve(&self, key: &str) -> Result<config::Resolution, MigrationError> {
        let key = Key::parse(key)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        self.capabilities
            .config
            .effective(&key, None)
            .map_err(|error| MigrationError::new(error.to_string()))
    }
}

impl MigrationContext for FileMigrationContext<'_> {
    fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
        config::paths::doc_type_dirs(self.capabilities.config)
            .map(|dirs| {
                dirs.into_iter()
                    .map(|dir| DocTypeDir {
                        doc_type: dir.doc_type.to_owned(),
                        dir: self.root.join(dir.dir),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn corpus_index(&self) -> &dyn CorpusIndex {
        self.index.get_or_init(|| {
            FileCorpusIndex::build(
                &self.linkage_table(),
                self.capabilities.walker,
            )
        })
    }

    fn write(&self, path: &Path, content: &str) -> Result<(), MigrationError> {
        store::atomic_write(
            path,
            content.as_bytes(),
            &self.bounds(),
            NewFileMode::PreserveOr(self.fresh_mode),
        )
        .map_err(|error| MigrationError::new(error.to_string()))?;
        if let Ok(relative) = path.strip_prefix(&self.root) {
            self.manifest
                .append_manifest_path(&relative.to_string_lossy())?;
        }
        Ok(())
    }

    fn write_private(
        &self,
        path: &Path,
        content: &str,
    ) -> Result<(), MigrationError> {
        store::atomic_write(
            path,
            content.as_bytes(),
            &self.bounds(),
            NewFileMode::Set(0o600),
        )
        .map_err(|error| MigrationError::new(error.to_string()))?;
        if let Ok(relative) = path.strip_prefix(&self.root) {
            self.manifest
                .append_manifest_path(&relative.to_string_lossy())?;
        }
        Ok(())
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn config_value(
        &self,
        key: &str,
    ) -> Result<Option<String>, MigrationError> {
        Ok(Some(self.resolve(key)?.rendered()))
    }

    fn configured_path_override(
        &self,
        key: &str,
    ) -> Result<Option<String>, MigrationError> {
        Ok(self.resolve(key)?.configured_value())
    }

    fn read(&self, path: &Path) -> Result<Option<String>, MigrationError> {
        let bytes = store::read_within(path, &self.bounds())
            .map_err(|error| MigrationError::new(error.to_string()))?;
        Ok(bytes.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn dir_exists(&self, path: &Path) -> bool {
        fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
    }

    fn remove_file(&self, path: &Path) -> Result<(), MigrationError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(())
            }
            Err(error) => Err(MigrationError::new(error.to_string())),
        }
    }

    fn remove_dir_if_empty(&self, path: &Path) -> Result<bool, MigrationError> {
        match fs::remove_dir(path) {
            Ok(()) => Ok(true),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::DirectoryNotEmpty
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(MigrationError::new(error.to_string())),
        }
    }

    fn list_md_files(
        &self,
        dir: &Path,
    ) -> Result<Vec<PathBuf>, MigrationError> {
        let mut files = Vec::new();
        walk(dir, &mut files, true)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        files.sort();
        Ok(files)
    }

    fn list_all_under(
        &self,
        dir: &Path,
    ) -> Result<Vec<PathBuf>, MigrationError> {
        let mut entries = Vec::new();
        walk(dir, &mut entries, false)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        entries.sort();
        Ok(entries)
    }

    fn merge_move(&self, src: &Path, dst: &Path) -> Result<(), MigrationError> {
        merge_move(src, dst, &self.root)
    }

    fn canonicalise_work_item_id(
        &self,
        bare_number: &str,
    ) -> Result<String, MigrationError> {
        let pattern = MigrationContext::config_value(self, "work.id_pattern")?
            .unwrap_or_else(|| "{number:04d}".to_owned());
        let project =
            MigrationContext::config_value(self, "work.default_project_code")?
                .unwrap_or_default();
        self.capabilities
            .canonicaliser
            .canonicalise(bare_number, &pattern, &project)
            .map_err(|error| MigrationError::new(error.to_string()))
    }

    fn validate_frontmatter(
        &self,
        files: &[PathBuf],
    ) -> Result<(), MigrationError> {
        let table = self.linkage_table();
        let Capabilities {
            walker,
            reader,
            frontmatter,
            ..
        } = self.capabilities;
        let target = if files.is_empty() {
            corpus::frontmatter_validation::pipeline::corpus_files(
                &table, walker,
            )
            .map_err(|error| MigrationError::new(error.to_string()))?
        } else {
            files.to_vec()
        };
        let index = corpus::frontmatter_validation::pipeline::build_index(
            &table,
            walker,
            reader,
            frontmatter,
        )
        .map_err(|error| MigrationError::new(error.to_string()))?;
        let checks = corpus::frontmatter_validation::pipeline::Checks {
            structure: true,
            references: true,
            canonical: false,
        };
        let results =
            corpus::frontmatter_validation::pipeline::validate_targets(
                &target,
                &table,
                &index,
                checks,
                reader,
                frontmatter,
            )
            .map_err(|error| MigrationError::new(error.to_string()))?;

        let mut messages = Vec::new();
        for (path, outcome) in &results {
            if let corpus::frontmatter_validation::pipeline::TargetOutcome::Violations(violations) = outcome {
                for violation in violations {
                    messages.push(format!("{}: {violation}", path.display()));
                }
            }
        }
        if messages.is_empty() {
            Ok(())
        } else {
            Err(MigrationError::new(messages.join("\n")))
        }
    }

    fn realign_sync_baseline(
        &self,
        pre_migration: &[(PathBuf, String)],
    ) -> Result<usize, MigrationError> {
        let Some(relative) =
            MigrationContext::config_value(self, "paths.integrations")?
                .filter(|value| !value.is_empty())
        else {
            return Ok(0);
        };
        let integrations_root = {
            let path = Path::new(&relative);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.root.join(path)
            }
        };
        if !integrations_root.is_dir() {
            return Ok(0);
        }

        self.capabilities
            .sync_baselines
            .realign(&integrations_root, pre_migration)
    }

    fn parse_frontmatter(
        &self,
        content: &str,
    ) -> Result<corpus::FrontmatterValue, MigrationError> {
        self.capabilities
            .frontmatter
            .parse_value(content)
            .map_err(|error| MigrationError::new(error.to_string()))
    }

    fn frontmatter_text(
        &self,
        content: &str,
    ) -> Result<String, MigrationError> {
        self.capabilities
            .frontmatter
            .split_frontmatter(content)
            .map_err(|error| MigrationError::new(error.to_string()))
    }

    fn render_canonical(
        &self,
        content: &str,
    ) -> Result<String, MigrationError> {
        let frontmatter = document::parse(content)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        document::render(Some(content), &frontmatter)
            .map_err(|error| MigrationError::new(error.to_string()))
    }
}

/// Recursively collects entries under `dir`. `md_only` selects between
/// [`MigrationContext::list_md_files`]'s `.md`-file-only listing and
/// [`MigrationContext::list_all_under`]'s unfiltered file-and-directory
/// listing.
fn walk(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    md_only: bool,
) -> std::io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(())
        }
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !md_only {
                out.push(path.clone());
            }
            walk(&path, out, md_only)?;
        } else if file_type.is_file()
            && (!md_only || path.extension().is_some_and(|ext| ext == "md"))
        {
            out.push(path);
        }
    }
    Ok(())
}

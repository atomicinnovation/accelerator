//! The filesystem store: it roots the two config files at a discovered project
//! directory and implements the core's read/write ports over `std::fs`.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use config::{ConfigError, Level, Node, ReadConfigLevel, WriteConfigLevel};

use crate::document;

static WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A config store rooted at a project directory. Holds only its root path, so
/// it is a cheap `Clone` backing both read and write ports.
#[derive(Clone)]
pub struct FileConfigStore {
    root: PathBuf,
}

impl FileConfigStore {
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Roots at the nearest ancestor of `start` holding a `.accelerator/`
    /// directory, a `.git` entry, or a `.jj` entry, else at `start`. `.jj`
    /// matches bash `find_repo_root` so a jj-only workspace checkout roots the
    /// same way; `.accelerator/` is an additional Rust-only stop marker.
    #[must_use]
    pub fn discover_root(start: &Path) -> PathBuf {
        let mut ancestor = Some(start);
        while let Some(dir) = ancestor {
            if dir.join(".accelerator").is_dir()
                || dir.join(".git").exists()
                || dir.join(".jj").exists()
            {
                return dir.to_path_buf();
            }
            ancestor = dir.parent();
        }
        start.to_path_buf()
    }

    fn config_dir(&self) -> PathBuf {
        self.root.join(".accelerator")
    }

    fn level_path(&self, level: Level) -> PathBuf {
        self.config_dir().join(match level {
            Level::Team => "config.md",
            Level::Personal => "config.local.md",
        })
    }

    fn atomic_write(
        &self,
        target: &Path,
        contents: &str,
    ) -> Result<(), ConfigError> {
        let temp_dir = self.config_dir().join("tmp");
        fs::create_dir_all(&temp_dir)
            .map_err(|error| io_error(&temp_dir, &error))?;
        let temp = temp_dir.join(format!(
            "config-{}-{}.tmp",
            process::id(),
            WRITE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        if let Err(error) = fs::write(&temp, contents) {
            let _ = fs::remove_file(&temp);
            return Err(io_error(&temp, &error));
        }
        if let Err(error) = fs::rename(&temp, target) {
            let _ = fs::remove_file(&temp);
            return Err(io_error(target, &error));
        }
        Ok(())
    }
}

impl ReadConfigLevel for FileConfigStore {
    fn read(&self, level: Level) -> Result<Option<Node>, ConfigError> {
        let path = self.level_path(level);
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(None)
            }
            Err(error) => return Err(io_error(&path, &error)),
        };
        let node = document::parse(&content).map_err(|detail| {
            ConfigError::MalformedFrontmatter {
                path: display(&path),
                detail,
            }
        })?;
        Ok(Some(node))
    }
}

impl WriteConfigLevel for FileConfigStore {
    fn write(&self, level: Level, document: &Node) -> Result<(), ConfigError> {
        let path = self.level_path(level);
        let existing = match fs::read_to_string(&path) {
            Ok(content) => Some(content),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => return Err(io_error(&path, &error)),
        };
        let rendered = document::render(existing.as_deref(), document)
            .map_err(|detail| ConfigError::MalformedFrontmatter {
                path: display(&path),
                detail,
            })?;
        self.atomic_write(&path, &rendered)
    }
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

fn io_error(path: &Path, error: &std::io::Error) -> ConfigError {
    ConfigError::Io {
        path: display(path),
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use config::{
        ConfigAccess, ConfigError, ConfigService, Key, Level, Node,
        ReadConfigLevel, Scalar, WriteConfigLevel,
    };

    use super::FileConfigStore;

    type TestError = Box<dyn std::error::Error>;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tempdir() -> Result<PathBuf, TestError> {
        let dir = std::env::temp_dir().join(format!(
            "cfg-adapters-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn seed(root: &Path, name: &str, content: &str) -> Result<(), TestError> {
        fs::create_dir_all(root.join(".accelerator"))?;
        fs::write(root.join(".accelerator").join(name), content)?;
        Ok(())
    }

    fn service(
        store: &FileConfigStore,
    ) -> ConfigService<FileConfigStore, FileConfigStore> {
        ConfigService::new(store.clone(), store.clone())
    }

    fn scalar_at<'a>(node: &'a Node, path: &[&str]) -> Option<&'a Scalar> {
        let mut current = node;
        for segment in path {
            let Node::Mapping(mapping) = current else {
                return None;
            };
            current = mapping.get(segment)?;
        }
        match current {
            Node::Scalar(scalar) => Some(scalar),
            _ => None,
        }
    }

    fn single_mapping(key: &str, value: &str) -> Node {
        Node::Mapping(
            vec![(
                key.to_owned(),
                Node::Scalar(Scalar::String(value.to_owned())),
            )]
            .into_iter()
            .collect(),
        )
    }

    #[test]
    fn an_absent_file_reads_as_an_empty_level() -> Result<(), TestError> {
        let store = FileConfigStore::at(tempdir()?);
        assert!(store.read(Level::Personal)?.is_none());
        Ok(())
    }

    #[test]
    fn a_write_creates_the_dir_and_round_trips() -> Result<(), TestError> {
        let root = tempdir()?;
        let store = FileConfigStore::at(&root);
        service(&store).set(
            &Key::parse("core.example")?,
            "value",
            Level::Team,
        )?;

        assert!(root.join(".accelerator/config.md").is_file());
        let read = store.read(Level::Team)?.ok_or("expected a document")?;
        assert_eq!(
            scalar_at(&read, &["core", "example"]),
            Some(&Scalar::String("value".to_owned()))
        );
        Ok(())
    }

    #[test]
    fn a_write_preserves_the_body() -> Result<(), TestError> {
        let root = tempdir()?;
        seed(
            &root,
            "config.md",
            "---\ncore:\n  example: old\n---\nbody\n",
        )?;
        let store = FileConfigStore::at(&root);
        service(&store).set(
            &Key::parse("core.example")?,
            "new",
            Level::Team,
        )?;

        let content = fs::read_to_string(root.join(".accelerator/config.md"))?;
        assert!(content.ends_with("body\n"), "body lost: {content:?}");
        Ok(())
    }

    #[test]
    fn typed_scalars_and_a_sequence_parse() -> Result<(), TestError> {
        let root = tempdir()?;
        seed(
            &root,
            "config.md",
            "---\nflag: true\ncount: 7\nratio: 1.5\nempty:\n\
             items:\n  - a\n  - b\nbig: 10000000000000000000\n---\n",
        )?;
        let store = FileConfigStore::at(&root);
        let read = store.read(Level::Team)?.ok_or("expected a document")?;

        assert_eq!(scalar_at(&read, &["flag"]), Some(&Scalar::Bool(true)));
        assert_eq!(scalar_at(&read, &["count"]), Some(&Scalar::Int(7)));
        assert_eq!(scalar_at(&read, &["ratio"]), Some(&Scalar::Float(1.5)));
        assert_eq!(scalar_at(&read, &["empty"]), Some(&Scalar::Null));
        assert_eq!(
            scalar_at(&read, &["big"]),
            Some(&Scalar::String("10000000000000000000".to_owned()))
        );
        let Node::Mapping(map) = &read else {
            return Err("root was not a mapping".into());
        };
        assert!(matches!(map.get("items"), Some(Node::Sequence(_))));
        Ok(())
    }

    #[test]
    fn malformed_frontmatter_reads_as_malformed() -> Result<(), TestError> {
        let root = tempdir()?;
        seed(&root, "config.md", "---\nkey: value\n")?;
        let store = FileConfigStore::at(&root);
        assert!(matches!(
            store.read(Level::Team),
            Err(ConfigError::MalformedFrontmatter { .. })
        ));
        Ok(())
    }

    #[test]
    fn a_write_against_a_malformed_file_fails_closed() -> Result<(), TestError>
    {
        let root = tempdir()?;
        let malformed = "---\nkey: value\n";
        seed(&root, "config.md", malformed)?;
        let store = FileConfigStore::at(&root);
        let document = single_mapping("core", "v");

        assert!(matches!(
            store.write(Level::Team, &document),
            Err(ConfigError::MalformedFrontmatter { .. })
        ));
        assert_eq!(
            fs::read_to_string(root.join(".accelerator/config.md"))?,
            malformed
        );
        Ok(())
    }

    #[test]
    fn a_write_against_a_fence_valid_but_invalid_yaml_file_fails_closed(
    ) -> Result<(), TestError> {
        let root = tempdir()?;
        let malformed = "---\nkey: : :\n  - broken\n---\nbody\n";
        seed(&root, "config.md", malformed)?;
        let store = FileConfigStore::at(&root);
        let document = single_mapping("core", "v");

        assert!(matches!(
            store.write(Level::Team, &document),
            Err(ConfigError::MalformedFrontmatter { .. })
        ));
        assert_eq!(
            fs::read_to_string(root.join(".accelerator/config.md"))?,
            malformed
        );
        Ok(())
    }

    #[test]
    fn an_over_cap_frontmatter_reads_as_malformed() -> Result<(), TestError> {
        let root = tempdir()?;
        let mut content = String::from("---\n");
        content.push_str(&"filler: line\n".repeat(120_000));
        content.push_str("---\nbody\n");
        seed(&root, "config.md", &content)?;
        let store = FileConfigStore::at(&root);
        assert!(matches!(
            store.read(Level::Team),
            Err(ConfigError::MalformedFrontmatter { .. })
        ));
        Ok(())
    }

    #[test]
    fn a_successful_write_leaves_no_stray_temp() -> Result<(), TestError> {
        let root = tempdir()?;
        let store = FileConfigStore::at(&root);
        service(&store).set(&Key::parse("core.example")?, "v", Level::Team)?;

        let temp_entries: Vec<_> = fs::read_dir(root.join(".accelerator/tmp"))?
            .flatten()
            .collect();
        assert!(temp_entries.is_empty(), "stray temp: {temp_entries:?}");
        Ok(())
    }

    #[test]
    fn discover_prefers_the_nearest_accelerator_under_an_ancestor_git(
    ) -> Result<(), TestError> {
        let root = tempdir()?;
        fs::create_dir_all(root.join(".git"))?;
        let project = root.join("project");
        fs::create_dir_all(project.join(".accelerator"))?;
        let start = project.join("a/b");
        fs::create_dir_all(&start)?;
        assert_eq!(FileConfigStore::discover_root(&start), project);
        Ok(())
    }

    #[test]
    fn discover_roots_at_a_jj_only_checkout() -> Result<(), TestError> {
        let root = tempdir()?;
        fs::create_dir_all(root.join(".jj"))?;
        let start = root.join("sub");
        fs::create_dir_all(&start)?;
        assert_eq!(FileConfigStore::discover_root(&start), root);
        Ok(())
    }

    #[test]
    fn discover_with_no_marker_roots_at_the_start_dir() -> Result<(), TestError>
    {
        let start = tempdir()?.join("isolated/leaf");
        fs::create_dir_all(&start)?;
        assert_eq!(FileConfigStore::discover_root(&start), start);
        Ok(())
    }
}

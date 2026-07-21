//! The filesystem store: it roots the two config files at a discovered project
//! directory and implements the core's read/write ports over `std::fs`.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use config::{
    ConfigError, CustomLens, LensFields, Level, Node, ReadConfigLevel,
    ReadContent, ReadLensCatalogue, WriteConfigLevel,
};
use store::{NewFileMode, WriteBounds, WriteError};

use crate::document;

/// Whether the reader honours the legacy `.claude/accelerator.md` layout.
///
/// `Allow` carries both halves the bash `ACCELERATOR_MIGRATION_MODE=1` did: it
/// suppresses the uniform legacy-layout refusal, and — when the current-layout
/// pair is absent — falls back to reading the legacy `.claude/accelerator.md`
/// and `.claude/accelerator.local.md` pair.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LegacyPolicy {
    Reject,
    Allow,
}

/// A config store rooted at a project directory. Holds its root path and the
/// legacy policy, so it is a cheap `Clone` backing both read and write ports.
#[derive(Clone)]
pub struct FileConfigStore {
    root: PathBuf,
    policy: LegacyPolicy,
}

impl FileConfigStore {
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            policy: LegacyPolicy::Reject,
        }
    }

    #[must_use]
    pub const fn with_legacy_policy(mut self, policy: LegacyPolicy) -> Self {
        self.policy = policy;
        self
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

    fn legacy_dir(&self) -> PathBuf {
        self.root.join(".claude")
    }

    /// Whether the legacy source fallback is engaged: the policy allows it and
    /// neither current-layout file exists, matching bash `config_find_files`.
    fn legacy_fallback_active(&self) -> bool {
        self.policy == LegacyPolicy::Allow
            && !self.config_dir().join("config.md").exists()
            && !self.config_dir().join("config.local.md").exists()
    }

    fn level_path(&self, level: Level) -> PathBuf {
        if self.legacy_fallback_active() {
            return self.legacy_dir().join(match level {
                Level::Team => "accelerator.md",
                Level::Personal => "accelerator.local.md",
            });
        }
        self.config_dir().join(match level {
            Level::Team => "config.md",
            Level::Personal => "config.local.md",
        })
    }

    /// The directory a level's file legitimately lives in, which the legacy
    /// fallback moves from `.accelerator/` to `.claude/`.
    fn permitted_root(&self) -> PathBuf {
        if self.legacy_fallback_active() {
            self.legacy_dir()
        } else {
            self.config_dir()
        }
    }

    fn bounds<'a>(&'a self, permitted_root: &'a Path) -> WriteBounds<'a> {
        WriteBounds {
            permitted_root,
            project_root: &self.root,
        }
    }
}

impl ReadConfigLevel for FileConfigStore {
    fn read(&self, level: Level) -> Result<Option<Node>, ConfigError> {
        let path = self.level_path(level);
        let permitted_root = self.permitted_root();
        store::ensure_contained(&path, &self.bounds(&permitted_root))
            .map_err(to_config_error)?;
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
        let config_dir = self.config_dir();
        let bounds = self.bounds(&config_dir);
        store::ensure_contained(&path, &bounds).map_err(to_config_error)?;
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
        ensure_inner_gitignore(&config_dir)?;
        let mode = match level {
            Level::Personal => NewFileMode::Set(0o600),
            Level::Team => {
                NewFileMode::PreserveOr(0o666 & !store::current_umask())
            }
        };
        store::atomic_write(&path, rendered.as_bytes(), &bounds, mode)
            .map_err(to_config_error)
    }
}

impl ReadContent for FileConfigStore {
    fn config_body(&self, level: Level) -> Result<Option<String>, ConfigError> {
        let path = self.level_path(level);
        let permitted_root = self.permitted_root();
        Ok(read_within(&path, &self.bounds(&permitted_root))?
            .map(|content| extract_body(&content)))
    }

    fn skill_context(
        &self,
        skill: &str,
    ) -> Result<Option<String>, ConfigError> {
        self.read_skill_file(skill, "context.md")
    }

    fn skill_instructions(
        &self,
        skill: &str,
    ) -> Result<Option<String>, ConfigError> {
        self.read_skill_file(skill, "instructions.md")
    }
}

impl FileConfigStore {
    fn read_skill_file(
        &self,
        skill: &str,
        file: &str,
    ) -> Result<Option<String>, ConfigError> {
        let config_dir = self.config_dir();
        let path = config_dir.join("skills").join(skill).join(file);
        read_within(&path, &self.bounds(&config_dir))
    }
}

impl ReadLensCatalogue for FileConfigStore {
    fn custom_lenses(&self) -> Result<Vec<CustomLens>, ConfigError> {
        let lenses_dir = self.config_dir().join("lenses");
        let entries = match fs::read_dir(&lenses_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Vec::new())
            }
            Err(error) => return Err(io_error(&lenses_dir, &error)),
        };
        let mut dirs: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        dirs.sort();
        let mut lenses = Vec::new();
        for dir in dirs {
            let skill_file = dir.join("SKILL.md");
            if !skill_file.is_file() {
                continue;
            }
            lenses.push(read_lens(&dir, &skill_file)?);
        }
        Ok(lenses)
    }

    fn skill_names(&self) -> Result<Vec<String>, ConfigError> {
        let skills_dir = self.config_dir().join("skills");
        let entries = match fs::read_dir(&skills_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Vec::new())
            }
            Err(error) => return Err(io_error(&skills_dir, &error)),
        };
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        Ok(names)
    }

    fn init_sentinel_present(
        &self,
        tmp_relative: &str,
    ) -> Result<bool, ConfigError> {
        Ok(self.root.join(tmp_relative).join(".gitignore").is_file())
    }
}

fn read_lens(dir: &Path, skill_file: &Path) -> Result<CustomLens, ConfigError> {
    let content =
        fs::read_to_string(skill_file).map_err(|e| io_error(skill_file, &e))?;
    let fields = document::parse(&content).ok().map(|node| LensFields {
        name: lens_field(&node, "name"),
        auto_detect: lens_field(&node, "auto_detect"),
        applies_to: lens_field(&node, "applies_to"),
    });
    Ok(CustomLens {
        dir: display(dir),
        path: display(skill_file),
        fields,
    })
}

fn lens_field(node: &Node, key: &str) -> Option<String> {
    let Node::Mapping(mapping) = node else {
        return None;
    };
    mapping.get(key).map(render_node)
}

fn render_node(node: &Node) -> String {
    match node {
        Node::Scalar(scalar) => render_scalar(scalar),
        Node::Sequence(items) => {
            let rendered: Vec<String> = items
                .iter()
                .map(|item| match item {
                    Node::Scalar(scalar) => render_scalar(scalar),
                    _ => String::new(),
                })
                .collect();
            format!("[{}]", rendered.join(", "))
        }
        Node::Mapping(_) => String::new(),
    }
}

fn render_scalar(scalar: &config::Scalar) -> String {
    use config::Scalar;
    match scalar {
        Scalar::String(value) => value.clone(),
        Scalar::Bool(value) => value.to_string(),
        Scalar::Int(value) => value.to_string(),
        Scalar::Float(value) => value.to_string(),
        Scalar::Null => String::new(),
    }
}

/// Reads a file within `bounds`, returning `None` when it is absent. Refuses a
/// path escaping the permitted root before touching the filesystem.
fn read_within(
    path: &Path,
    bounds: &WriteBounds<'_>,
) -> Result<Option<String>, ConfigError> {
    store::ensure_contained(path, bounds).map_err(to_config_error)?;
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, &error)),
    }
}

/// The markdown body: everything after a leading `---`-fenced frontmatter, or
/// the whole file when there is none. An unterminated fence yields no body,
/// matching bash `config_extract_body`.
fn extract_body(content: &str) -> String {
    let mut lines = content.lines();
    match lines.next() {
        None => String::new(),
        Some(first) if !is_fence(first) => content.to_owned(),
        Some(_) => {
            let mut closed = false;
            let mut body = Vec::new();
            for line in lines {
                if closed {
                    body.push(line);
                } else if is_fence(line) {
                    closed = true;
                }
            }
            if closed {
                body.join("\n")
            } else {
                String::new()
            }
        }
    }
}

fn is_fence(line: &str) -> bool {
    line.strip_prefix("---")
        .is_some_and(|rest| rest.trim().is_empty())
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

fn to_config_error(error: WriteError) -> ConfigError {
    match error {
        WriteError::UnsafePath { path } => ConfigError::UnsafePath { path },
        WriteError::NotWritable { path } => ConfigError::Io {
            path,
            detail: "not writable".to_owned(),
        },
        WriteError::CrossFilesystem { path } => ConfigError::Io {
            path,
            detail: "atomic rename crossed a filesystem boundary".to_owned(),
        },
        WriteError::Io { path, detail } => ConfigError::Io { path, detail },
        other => ConfigError::Io {
            path: String::new(),
            detail: other.to_string(),
        },
    }
}

/// Ensures `.accelerator/.gitignore` ignores the personal config file and the
/// staged temp prefix, appending only rules that are absent so a hand-edited
/// file keeps its other entries. Fails closed so a write never orphans an
/// un-ignored temp under jj's working-copy auto-snapshot.
fn ensure_inner_gitignore(config_dir: &Path) -> Result<(), ConfigError> {
    let path = config_dir.join(".gitignore");
    let existing = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => return Err(io_error(&path, &error)),
    };
    let temp_rule = format!("{}*", store::TEMP_PREFIX);
    let mut additions = String::new();
    for rule in ["config.local.md", temp_rule.as_str()] {
        if !existing.lines().any(|line| line == rule) {
            additions.push_str(rule);
            additions.push('\n');
        }
    }
    if additions.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(config_dir)
        .map_err(|error| io_error(config_dir, &error))?;
    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&additions);
    fs::write(&path, content).map_err(|error| io_error(&path, &error))
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

        let temps: Vec<_> = fs::read_dir(root.join(".accelerator"))?
            .flatten()
            .filter(|entry| {
                entry.file_name().to_string_lossy().starts_with(".tmp-")
            })
            .collect();
        assert!(temps.is_empty(), "stray temp: {temps:?}");
        Ok(())
    }

    fn mode_of(path: &Path) -> Result<u32, TestError> {
        use std::os::unix::fs::PermissionsExt as _;
        Ok(fs::metadata(path)?.permissions().mode() & 0o777)
    }

    #[test]
    fn a_personal_write_lands_at_0600() -> Result<(), TestError> {
        let root = tempdir()?;
        let store = FileConfigStore::at(&root);
        service(&store).set(
            &Key::parse("jira.token")?,
            "secret",
            Level::Personal,
        )?;
        assert_eq!(mode_of(&root.join(".accelerator/config.local.md"))?, 0o600);
        Ok(())
    }

    #[test]
    fn a_personal_write_clamps_a_preexisting_wider_mode(
    ) -> Result<(), TestError> {
        use std::os::unix::fs::PermissionsExt as _;
        let root = tempdir()?;
        seed(&root, "config.local.md", "---\njira:\n  token: old\n---\n")?;
        fs::set_permissions(
            root.join(".accelerator/config.local.md"),
            fs::Permissions::from_mode(0o644),
        )?;
        let store = FileConfigStore::at(&root);
        service(&store).set(
            &Key::parse("jira.token")?,
            "new",
            Level::Personal,
        )?;
        assert_eq!(
            mode_of(&root.join(".accelerator/config.local.md"))?,
            0o600,
            "a personal write must clamp a world-readable file to 0600"
        );
        Ok(())
    }

    #[test]
    fn a_team_write_preserves_an_existing_mode() -> Result<(), TestError> {
        use std::os::unix::fs::PermissionsExt as _;
        let root = tempdir()?;
        seed(&root, "config.md", "---\ncore:\n  example: old\n---\n")?;
        fs::set_permissions(
            root.join(".accelerator/config.md"),
            fs::Permissions::from_mode(0o664),
        )?;
        let store = FileConfigStore::at(&root);
        service(&store).set(
            &Key::parse("core.example")?,
            "new",
            Level::Team,
        )?;
        assert_eq!(
            mode_of(&root.join(".accelerator/config.md"))?,
            0o664,
            "a team write must preserve the shared mode"
        );
        Ok(())
    }

    #[test]
    fn a_personal_write_ensures_the_inner_gitignore() -> Result<(), TestError> {
        let root = tempdir()?;
        let store = FileConfigStore::at(&root);
        service(&store).set(
            &Key::parse("jira.token")?,
            "secret",
            Level::Personal,
        )?;
        let gitignore =
            fs::read_to_string(root.join(".accelerator/.gitignore"))?;
        assert!(gitignore.lines().any(|line| line == "config.local.md"));
        assert!(gitignore.lines().any(|line| line == ".tmp-*"));
        Ok(())
    }

    #[test]
    fn a_symlinked_config_file_escaping_is_refused_on_read(
    ) -> Result<(), TestError> {
        let root = tempdir()?;
        let outside = root.join("outside.md");
        fs::write(&outside, "---\njira:\n  token: stolen\n---\n")?;
        fs::create_dir_all(root.join(".accelerator"))?;
        std::os::unix::fs::symlink(
            &outside,
            root.join(".accelerator/config.local.md"),
        )?;
        let store = FileConfigStore::at(&root);
        assert!(matches!(
            store.read(Level::Personal),
            Err(ConfigError::UnsafePath { .. })
        ));
        Ok(())
    }

    #[test]
    fn a_symlinked_config_file_escaping_is_refused_on_write(
    ) -> Result<(), TestError> {
        let root = tempdir()?;
        let outside = root.join("outside.md");
        fs::write(&outside, "---\ncore:\n  example: original\n---\n")?;
        fs::create_dir_all(root.join(".accelerator"))?;
        std::os::unix::fs::symlink(
            &outside,
            root.join(".accelerator/config.md"),
        )?;
        let store = FileConfigStore::at(&root);
        assert!(matches!(
            store.write(Level::Team, &single_mapping("core", "v")),
            Err(ConfigError::UnsafePath { .. })
        ));
        assert_eq!(
            fs::read_to_string(&outside)?,
            "---\ncore:\n  example: original\n---\n",
            "the symlink target must not be clobbered"
        );
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

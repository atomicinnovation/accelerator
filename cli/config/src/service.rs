//! The driven and driving ports and the application service that performs
//! precedence resolution and the nested-path walk and insert.

use crate::catalogue;
use crate::error::ConfigError;
use crate::error::Existing;
use crate::key::Key;
use crate::level::Level;
use crate::node::Mapping;
use crate::node::Node;
use crate::node::Scalar;
use crate::render::render_value;

/// A resolved configuration value: a scalar leaf or a sequence of scalars.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Scalar(Scalar),
    Sequence(Vec<Scalar>),
}

/// The outcome of resolving a key. Presence is decided here; a present empty
/// string, null, or non-addressable node still resolves to [`Resolved::Found`].
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved {
    Found(Value),
    Absent,
}

/// Which side supplied a resolved value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Personal,
    Team,
    Catalogue,
    Unset,
}

/// A resolved value with its winning source, the catalogue default already
/// folded in on absence.
///
/// Where [`Resolved`] reports raw presence in a level, `Resolution` is
/// presence-plus-default-plus-source. `render_value(Scalar::Null)` is the empty
/// string, so an [`Source::Unset`] resolution renders identically to a
/// config-present empty string; absence is authoritative only via [`source`]
/// and [`is_from_config`], never `rendered().is_empty()`.
///
/// [`source`]: Resolution::source
/// [`is_from_config`]: Resolution::is_from_config
#[derive(Debug, Clone)]
pub struct Resolution {
    value: Value,
    source: Source,
}

impl Resolution {
    #[must_use]
    pub fn rendered(&self) -> String {
        render_value(&self.value)
    }

    #[must_use]
    pub const fn source(&self) -> Source {
        self.source
    }

    #[must_use]
    pub const fn is_from_config(&self) -> bool {
        matches!(self.source, Source::Personal | Source::Team)
    }

    /// The rendered value only when it came from config (`Personal` or `Team`);
    /// `None` for a catalogue default or an unset key, so the empty-vs-absent
    /// distinction cannot be lost to a naive `.is_empty()` check.
    #[must_use]
    pub fn configured_value(&self) -> Option<String> {
        self.is_from_config().then(|| self.rendered())
    }
}

/// Reads a single level's document — a driven port.
pub trait ReadConfigLevel {
    /// Returns the parsed document, or `None` when the level's file is absent.
    ///
    /// # Errors
    ///
    /// [`ConfigError::MalformedFrontmatter`] or [`ConfigError::Io`] when a
    /// present file cannot be parsed or read — never silently skipped.
    fn read(&self, level: Level) -> Result<Option<Node>, ConfigError>;
}

/// Writes a whole document to a single level — a driven port.
pub trait WriteConfigLevel {
    /// # Errors
    ///
    /// [`ConfigError::Io`] when the document cannot be persisted.
    fn write(&self, level: Level, document: &Node) -> Result<(), ConfigError>;
}

/// Reads the injection *content* the block subcommands render — a driven port.
///
/// That content is the markdown body of a config level's file (project
/// context) and a per-skill customisation file. Distinct from
/// [`ReadConfigLevel`], which parses only the frontmatter.
pub trait ReadContent {
    /// The raw markdown body of the config level's file (everything after the
    /// frontmatter), or `None` when the file is absent.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Io`] or [`ConfigError::UnsafePath`] when a present file
    /// cannot be read.
    fn config_body(&self, level: Level) -> Result<Option<String>, ConfigError>;

    /// The raw content of `.accelerator/skills/<skill>/context.md`, or `None`
    /// when it is absent. `skill` is a validated identifier.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Io`] or [`ConfigError::UnsafePath`] when a present file
    /// cannot be read.
    fn skill_context(&self, skill: &str)
        -> Result<Option<String>, ConfigError>;

    /// The raw content of `.accelerator/skills/<skill>/instructions.md`, or
    /// `None` when it is absent. `skill` is a validated identifier.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Io`] or [`ConfigError::UnsafePath`] when a present file
    /// cannot be read.
    fn skill_instructions(
        &self,
        skill: &str,
    ) -> Result<Option<String>, ConfigError>;
}

/// A custom lens directory's parsed frontmatter fields. `None` on a field means
/// the field was absent; `Some(String::new())` means it was present but empty.
pub struct LensFields {
    pub name: Option<String>,
    pub auto_detect: Option<String>,
    pub applies_to: Option<String>,
}

/// One custom lens directory carrying a `SKILL.md`. `fields` is `None` when that
/// file's frontmatter is malformed.
pub struct CustomLens {
    pub dir: String,
    pub path: String,
    pub fields: Option<LensFields>,
}

/// Enumerates the project's customisation directories — a driven port.
///
/// Covers the custom lenses under `.accelerator/lenses/`, the per-skill
/// customisation directory names under `.accelerator/skills/`, and the init
/// sentinel. Domain validation (lens name collision, `applies_to` filtering)
/// is the review view's, not the adapter's.
pub trait ReadLensCatalogue {
    /// # Errors
    ///
    /// A [`ConfigError`] when the lenses directory cannot be enumerated.
    fn custom_lenses(&self) -> Result<Vec<CustomLens>, ConfigError>;

    /// The directory names directly under `.accelerator/skills/`, sorted.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when the skills directory cannot be enumerated.
    fn skill_names(&self) -> Result<Vec<String>, ConfigError>;

    /// The plugin's own skill names (excluding `configure`), used to flag a
    /// customisation directory that matches no real skill. Empty when the
    /// plugin root is unknown, so the caller cannot validate and stays silent.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when a plugin skill directory cannot be enumerated.
    fn known_skill_names(&self) -> Result<Vec<String>, ConfigError>;

    /// Whether the init sentinel `<tmp>/.gitignore` exists, `tmp` resolved
    /// relative to the project root.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when the check cannot be performed.
    fn init_sentinel_present(
        &self,
        tmp_relative: &str,
    ) -> Result<bool, ConfigError>;
}

/// Where a resolved template came from, in three-tier precedence order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TemplateSource {
    ConfigPath,
    UserOverride,
    PluginDefault,
}

impl TemplateSource {
    /// The bash label for the source.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ConfigPath => "config path",
            Self::UserOverride => "user override",
            Self::PluginDefault => "plugin default",
        }
    }
}

/// A resolved template.
///
/// Carries its source, its absolute and display-shortened paths, and its raw
/// content; `warning` holds the tier-1 fallback note when a configured path was
/// set but absent.
pub struct ResolvedTemplate {
    pub source: TemplateSource,
    pub abs_path: String,
    pub display_path: String,
    pub content: String,
    pub warning: Option<String>,
}

/// What ejecting one template did (or would do), for the exit code and message.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EjectOutcome {
    Ejected,
    Overwritten,
    /// The target already exists and `--force` was not given — exit 2.
    Exists,
    WouldEject,
    WouldOverwrite,
    WouldSkip,
    /// No plugin default template for the name — exit 1.
    NoDefault,
}

/// The result of an eject: its outcome, the key, and the display target path.
pub struct EjectResult {
    pub outcome: EjectOutcome,
    pub key: String,
    pub display: String,
}

/// Mutates the user template overrides — a driven port. Reads live on
/// [`ReadTemplate`]; this port owns the eject copy and the reset delete.
pub trait TemplateOverride {
    /// Ejects (copies) the plugin default for `name` into `templates_dir`.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when the copy or the directory creation fails.
    fn eject(
        &self,
        name: &str,
        templates_dir: &str,
        force: bool,
        dry_run: bool,
    ) -> Result<EjectResult, ConfigError>;

    /// Deletes a resolved override file (reset `--confirm`).
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when the delete fails.
    fn delete(&self, abs_path: &str) -> Result<(), ConfigError>;

    /// Whether an absolute path lies inside the resolution project root.
    fn within_project(&self, abs_path: &str) -> bool;
}

/// Creates the project scaffold — a driven port. The core resolves which
/// content directories and tmp directory to create; the adapter performs the
/// idempotent filesystem work.
pub trait Scaffold {
    /// Creates each content directory with a `.gitkeep`, the `.accelerator/`
    /// core tree and its ignore rules, the tmp directory with its ignore file,
    /// and the anchored root ignore rule. Idempotent.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when a directory or file cannot be created.
    fn init(
        &self,
        content_dirs: &[String],
        tmp_dir: &str,
    ) -> Result<(), ConfigError>;
}

/// Resolves and enumerates template files across the project and plugin.
///
/// A driven port: the caller supplies the config-derived inputs
/// (`templates.<key>` and the templates directory); the port performs the
/// filesystem tiers.
pub trait ReadTemplate {
    /// Resolves `name` through the three tiers, or `None` when not found.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when a candidate file cannot be read.
    fn resolve_template(
        &self,
        name: &str,
        config_path: Option<&str>,
        templates_dir: &str,
    ) -> Result<Option<ResolvedTemplate>, ConfigError>;

    /// The template names available from the plugin templates directory,
    /// sorted.
    fn template_names(&self) -> Vec<String>;

    /// The plugin default template for `name`, or `None` when the plugin ships
    /// no default for it.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when a present default cannot be read.
    fn plugin_default(
        &self,
        name: &str,
    ) -> Result<Option<ResolvedTemplate>, ConfigError>;
}

/// The operations the core offers callers — the driving port.
pub trait ConfigAccess {
    /// Resolves a key, full-stack (personal over team) when `level` is `None`,
    /// or against a single level when `Some`.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when a level being read fails; a full-stack read fails
    /// if either level is malformed.
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError>;

    /// Writes a string value at a key in a single level, creating intermediate
    /// mappings as needed.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when the level cannot be read, the path conflicts with
    /// an existing shape, or the write fails.
    fn set(
        &self,
        key: &Key,
        value: &str,
        level: Level,
    ) -> Result<(), ConfigError>;

    /// Resolves a key with its catalogue default folded in on absence, reporting
    /// the winning [`Source`]. A config-present value keeps its level source even
    /// when it renders empty; the catalogue default applies only when neither
    /// level supplies the key. Reads both levels eagerly when `level` is `None`,
    /// so a malformed non-winning level still fails loud.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when a level being read fails; a full-stack read fails
    /// if either level is malformed.
    fn effective(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolution, ConfigError> {
        let found = if let Some(one) = level {
            match self.get(key, Some(one))? {
                Resolved::Found(value) => Some((source_of(one), value)),
                Resolved::Absent => None,
            }
        } else {
            let personal = self.get(key, Some(Level::Personal))?;
            let team = self.get(key, Some(Level::Team))?;
            match (personal, team) {
                (Resolved::Found(value), _) => Some((Source::Personal, value)),
                (Resolved::Absent, Resolved::Found(value)) => {
                    Some((Source::Team, value))
                }
                (Resolved::Absent, Resolved::Absent) => None,
            }
        };
        Ok(match found {
            Some((source, value)) => Resolution { value, source },
            None => default_resolution(key),
        })
    }

    /// As [`effective`], but a config-present value that renders empty is treated
    /// as absent and replaced by the catalogue default — the empty-collapse the
    /// agent and template paths need.
    ///
    /// [`effective`]: ConfigAccess::effective
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] when a level being read fails; a full-stack read fails
    /// if either level is malformed.
    fn effective_nonempty(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolution, ConfigError> {
        let resolution = self.effective(key, level)?;
        Ok(
            if resolution.is_from_config() && resolution.rendered().is_empty() {
                default_resolution(key)
            } else {
                resolution
            },
        )
    }
}

const fn source_of(level: Level) -> Source {
    match level {
        Level::Team => Source::Team,
        Level::Personal => Source::Personal,
    }
}

fn default_resolution(key: &Key) -> Resolution {
    catalogue::default_for(&key.to_string()).map_or(
        Resolution {
            value: Value::Scalar(Scalar::Null),
            source: Source::Unset,
        },
        |value| Resolution {
            value,
            source: Source::Catalogue,
        },
    )
}

/// The application service. Depends only on the two driven ports.
pub struct ConfigService<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> ConfigService<R, W> {
    pub const fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

impl<R: ReadConfigLevel, W: WriteConfigLevel> ConfigAccess
    for ConfigService<R, W>
{
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError> {
        if let Some(level) = level {
            return Ok(resolve(self.reader.read(level)?.as_ref(), key));
        }
        let personal = self.reader.read(Level::Personal)?;
        let team = self.reader.read(Level::Team)?;
        let resolved = resolve(personal.as_ref(), key);
        Ok(if matches!(resolved, Resolved::Found(_)) {
            resolved
        } else {
            resolve(team.as_ref(), key)
        })
    }

    fn set(
        &self,
        key: &Key,
        value: &str,
        level: Level,
    ) -> Result<(), ConfigError> {
        let mut root = match self.reader.read(level)? {
            Some(Node::Mapping(mapping)) => mapping,
            None => Mapping::new(),
            Some(_) => {
                return Err(ConfigError::Invalid {
                    detail:
                        "refusing to write: the config frontmatter root is \
                         not a mapping"
                            .to_owned(),
                })
            }
        };
        insert(&mut root, key.segments(), value, key)?;
        self.writer.write(level, &Node::Mapping(root))
    }
}

fn resolve(document: Option<&Node>, key: &Key) -> Resolved {
    let Some(mut current) = document else {
        return Resolved::Absent;
    };
    for segment in key.segments() {
        let Node::Mapping(mapping) = current else {
            return Resolved::Absent;
        };
        match mapping.get(segment) {
            Some(child) => current = child,
            None => return Resolved::Absent,
        }
    }
    Resolved::Found(project(current))
}

fn project(node: &Node) -> Value {
    match node {
        Node::Scalar(scalar) => Value::Scalar(scalar.clone()),
        Node::Sequence(items) => scalar_elements(items)
            .map_or(Value::Scalar(Scalar::Null), Value::Sequence),
        Node::Mapping(_) => Value::Scalar(Scalar::Null),
    }
}

fn scalar_elements(items: &[Node]) -> Option<Vec<Scalar>> {
    items
        .iter()
        .map(|item| match item {
            Node::Scalar(scalar) => Some(scalar.clone()),
            _ => None,
        })
        .collect()
}

fn insert(
    mapping: &mut Mapping,
    segments: &[String],
    value: &str,
    key: &Key,
) -> Result<(), ConfigError> {
    let Some((head, rest)) = segments.split_first() else {
        return Ok(());
    };
    if rest.is_empty() {
        return set_leaf(mapping, head, value, key);
    }
    let child = descend_for_insert(mapping, head, key)?;
    insert(child, rest, value, key)
}

fn set_leaf(
    mapping: &mut Mapping,
    head: &str,
    value: &str,
    key: &Key,
) -> Result<(), ConfigError> {
    if matches!(
        mapping.get(head),
        Some(Node::Mapping(_) | Node::Sequence(_))
    ) {
        return Err(ConfigError::PathConflict {
            key: key.clone(),
            at: head.to_owned(),
            existing: Existing::Section,
        });
    }
    mapping.upsert(head, Node::Scalar(Scalar::String(value.to_owned())));
    Ok(())
}

fn descend_for_insert<'m>(
    parent: &'m mut Mapping,
    head: &str,
    key: &Key,
) -> Result<&'m mut Mapping, ConfigError> {
    if parent.get(head).is_none() {
        parent.push(head.to_owned(), Node::Mapping(Mapping::new()));
    }
    match parent.get_mut(head) {
        Some(Node::Mapping(child)) => Ok(child),
        _ => Err(ConfigError::PathConflict {
            key: key.clone(),
            at: head.to_owned(),
            existing: Existing::Value,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::{
        ConfigAccess, ConfigService, ReadConfigLevel, Resolved, Source, Value,
        WriteConfigLevel,
    };
    use crate::error::{ConfigError, Existing};
    use crate::key::Key;
    use crate::level::Level;
    use crate::node::{Node, Scalar};

    fn text(value: &str) -> Node {
        Node::Scalar(Scalar::String(value.to_owned()))
    }

    fn found_text(value: &str) -> Resolved {
        Resolved::Found(Value::Scalar(Scalar::String(value.to_owned())))
    }

    fn mapping(entries: Vec<(&str, Node)>) -> Node {
        Node::Mapping(
            entries
                .into_iter()
                .map(|(name, node)| (name.to_owned(), node))
                .collect(),
        )
    }

    fn sequence(values: &[&str]) -> Node {
        Node::Sequence(values.iter().map(|value| text(value)).collect())
    }

    enum LevelState {
        Missing,
        Present(Node),
        Failing,
    }

    struct FakeReader {
        team: LevelState,
        personal: LevelState,
    }

    impl FakeReader {
        fn new(team: LevelState, personal: LevelState) -> Self {
            Self { team, personal }
        }
    }

    impl ReadConfigLevel for FakeReader {
        fn read(&self, level: Level) -> Result<Option<Node>, ConfigError> {
            let state = match level {
                Level::Team => &self.team,
                Level::Personal => &self.personal,
            };
            match state {
                LevelState::Missing => Ok(None),
                LevelState::Present(node) => Ok(Some(node.clone())),
                LevelState::Failing => Err(ConfigError::Io {
                    path: "fake".to_owned(),
                    detail: "boom".to_owned(),
                }),
            }
        }
    }

    #[derive(Clone, Default)]
    struct FakeWriter {
        captured: Rc<RefCell<Vec<(Level, Node)>>>,
    }

    impl WriteConfigLevel for FakeWriter {
        fn write(
            &self,
            level: Level,
            document: &Node,
        ) -> Result<(), ConfigError> {
            self.captured.borrow_mut().push((level, document.clone()));
            Ok(())
        }
    }

    fn service(reader: FakeReader) -> ConfigService<FakeReader, FakeWriter> {
        ConfigService::new(reader, FakeWriter::default())
    }

    #[test]
    fn personal_overrides_team() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("team"))]),
            )])),
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("personal"))]),
            )])),
        );
        let resolved =
            service(reader).get(&Key::parse("core.example")?, None)?;
        assert_eq!(resolved, found_text("personal"));
        Ok(())
    }

    #[test]
    fn team_only_falls_through() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("team"))]),
            )])),
            LevelState::Missing,
        );
        let resolved =
            service(reader).get(&Key::parse("core.example")?, None)?;
        assert_eq!(resolved, found_text("team"));
        Ok(())
    }

    #[test]
    fn reads_only_the_named_level() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Present(mapping(vec![("k", text("team"))])),
            LevelState::Present(mapping(vec![("k", text("personal"))])),
        );
        let service = service(reader);
        let key = Key::parse("k")?;
        assert_eq!(service.get(&key, Some(Level::Team))?, found_text("team"));
        assert_eq!(
            service.get(&key, Some(Level::Personal))?,
            found_text("personal")
        );
        Ok(())
    }

    #[test]
    fn present_null_resolves_to_found() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![(
                "example",
                Node::Scalar(Scalar::Null),
            )])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("example")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::Null))
        );
        Ok(())
    }

    #[test]
    fn present_empty_string_resolves_to_found() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![("example", text(""))])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("example")?, None)?,
            found_text("")
        );
        Ok(())
    }

    #[test]
    fn absent_from_all_levels_resolves_to_absent() -> Result<(), ConfigError> {
        let reader = FakeReader::new(LevelState::Missing, LevelState::Missing);
        assert_eq!(
            service(reader).get(&Key::parse("core.example")?, None)?,
            Resolved::Absent
        );
        Ok(())
    }

    #[test]
    fn resolves_to_the_matching_typed_scalar() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![
                    ("flag", Node::Scalar(Scalar::Bool(true))),
                    ("count", Node::Scalar(Scalar::Int(42))),
                ]),
            )])),
        );
        let service = service(reader);
        assert_eq!(
            service.get(&Key::parse("core.flag")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::Bool(true)))
        );
        assert_eq!(
            service.get(&Key::parse("core.count")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::Int(42)))
        );
        Ok(())
    }

    #[test]
    fn resolves_a_scalar_sequence_to_a_typed_list() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![(
                "review",
                mapping(vec![("core_lenses", sequence(&["a", "b"]))]),
            )])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("review.core_lenses")?, None)?,
            Resolved::Found(Value::Sequence(vec![
                Scalar::String("a".to_owned()),
                Scalar::String("b".to_owned()),
            ]))
        );
        Ok(())
    }

    #[test]
    fn a_personal_sequence_shadows_a_team_sequence() -> Result<(), ConfigError>
    {
        let reader = FakeReader::new(
            LevelState::Present(mapping(vec![(
                "review",
                mapping(vec![("core_lenses", sequence(&["team"]))]),
            )])),
            LevelState::Present(mapping(vec![(
                "review",
                mapping(vec![("core_lenses", sequence(&["personal"]))]),
            )])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("review.core_lenses")?, None)?,
            Resolved::Found(Value::Sequence(vec![Scalar::String(
                "personal".to_owned()
            )]))
        );
        Ok(())
    }

    #[test]
    fn a_sequence_with_a_non_scalar_element_is_found_empty(
    ) -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![(
                "review",
                mapping(vec![(
                    "core_lenses",
                    Node::Sequence(vec![mapping(vec![("k", text("v"))])]),
                )]),
            )])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("review.core_lenses")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::Null))
        );
        Ok(())
    }

    #[test]
    fn descending_through_a_non_mapping_is_absent() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("leaf"))]),
            )])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("core.example.deeper")?, None)?,
            Resolved::Absent
        );
        Ok(())
    }

    #[test]
    fn a_path_ending_on_a_mapping_is_found_empty() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("leaf"))]),
            )])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("core")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::Null))
        );
        Ok(())
    }

    #[test]
    fn a_personal_mapping_node_shadows_a_team_scalar_as_found_empty(
    ) -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("team"))]),
            )])),
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![(
                    "example",
                    mapping(vec![("nested", text("x"))]),
                )]),
            )])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("core.example")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::Null))
        );
        Ok(())
    }

    #[test]
    fn walks_a_nested_path() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![(
                "a",
                mapping(vec![("b", mapping(vec![("c", text("deep"))]))]),
            )])),
        );
        assert_eq!(
            service(reader).get(&Key::parse("a.b.c")?, None)?,
            found_text("deep")
        );
        Ok(())
    }

    #[test]
    fn set_creates_an_absent_nested_block() -> Result<(), ConfigError> {
        let writer = FakeWriter::default();
        let captured = writer.captured.clone();
        let service = ConfigService::new(
            FakeReader::new(LevelState::Missing, LevelState::Missing),
            writer,
        );
        service.set(&Key::parse("core.example")?, "value", Level::Team)?;
        let captured = captured.borrow();
        assert_eq!(
            captured.as_slice(),
            [(
                Level::Team,
                mapping(vec![(
                    "core",
                    mapping(vec![("example", text("value"))]),
                )])
            )]
        );
        Ok(())
    }

    #[test]
    fn set_creates_every_intermediate_on_a_deep_path() -> Result<(), ConfigError>
    {
        let writer = FakeWriter::default();
        let captured = writer.captured.clone();
        let service = ConfigService::new(
            FakeReader::new(LevelState::Missing, LevelState::Missing),
            writer,
        );
        service.set(&Key::parse("a.b.c.d")?, "deep", Level::Personal)?;
        let expected = mapping(vec![(
            "a",
            mapping(vec![(
                "b",
                mapping(vec![("c", mapping(vec![("d", text("deep"))]))]),
            )]),
        )]);
        assert_eq!(captured.borrow().as_slice(), [(Level::Personal, expected)]);
        Ok(())
    }

    #[test]
    fn set_replacing_a_scalar_leaf_is_a_normal_update(
    ) -> Result<(), ConfigError> {
        let writer = FakeWriter::default();
        let captured = writer.captured.clone();
        let service = ConfigService::new(
            FakeReader::new(
                LevelState::Missing,
                LevelState::Present(mapping(vec![(
                    "core",
                    mapping(vec![("example", text("old"))]),
                )])),
            ),
            writer,
        );
        service.set(&Key::parse("core.example")?, "new", Level::Personal)?;
        assert_eq!(
            captured.borrow().as_slice(),
            [(
                Level::Personal,
                mapping(vec![(
                    "core",
                    mapping(vec![("example", text("new"))]),
                )])
            )]
        );
        Ok(())
    }

    #[test]
    fn set_refuses_a_present_non_mapping_root() -> Result<(), ConfigError> {
        let writer = FakeWriter::default();
        let captured = writer.captured.clone();
        let service = ConfigService::new(
            FakeReader::new(
                LevelState::Missing,
                LevelState::Present(sequence(&["a", "b"])),
            ),
            writer,
        );
        assert!(matches!(
            service.set(&Key::parse("core.example")?, "v", Level::Personal),
            Err(ConfigError::Invalid { .. })
        ));
        assert!(captured.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn set_conflicts_descending_through_a_scalar() -> Result<(), ConfigError> {
        let writer = FakeWriter::default();
        let captured = writer.captured.clone();
        let service = ConfigService::new(
            FakeReader::new(
                LevelState::Missing,
                LevelState::Present(mapping(vec![("core", text("scalar"))])),
            ),
            writer,
        );
        let result =
            service.set(&Key::parse("core.example")?, "value", Level::Personal);
        assert_eq!(
            result,
            Err(ConfigError::PathConflict {
                key: Key::parse("core.example")?,
                at: "core".to_owned(),
                existing: Existing::Value,
            })
        );
        assert!(captured.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn set_conflicts_replacing_a_container_leaf() -> Result<(), ConfigError> {
        let writer = FakeWriter::default();
        let captured = writer.captured.clone();
        let service = ConfigService::new(
            FakeReader::new(
                LevelState::Missing,
                LevelState::Present(mapping(vec![(
                    "core",
                    mapping(vec![("example", text("x"))]),
                )])),
            ),
            writer,
        );
        let result =
            service.set(&Key::parse("core")?, "value", Level::Personal);
        assert_eq!(
            result,
            Err(ConfigError::PathConflict {
                key: Key::parse("core")?,
                at: "core".to_owned(),
                existing: Existing::Section,
            })
        );
        assert!(captured.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn full_stack_get_fails_loud_on_a_personal_read_error(
    ) -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Present(mapping(vec![("k", text("team"))])),
            LevelState::Failing,
        );
        assert!(service(reader).get(&Key::parse("k")?, None).is_err());
        Ok(())
    }

    #[test]
    fn full_stack_get_fails_loud_on_a_team_read_error(
    ) -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Failing,
            LevelState::Present(mapping(vec![("k", text("personal"))])),
        );
        assert!(service(reader).get(&Key::parse("k")?, None).is_err());
        Ok(())
    }

    #[test]
    fn set_fails_closed_and_never_writes_on_a_read_error(
    ) -> Result<(), ConfigError> {
        let writer = FakeWriter::default();
        let captured = writer.captured.clone();
        let service = ConfigService::new(
            FakeReader::new(LevelState::Missing, LevelState::Failing),
            writer,
        );
        assert!(service
            .set(&Key::parse("k")?, "value", Level::Personal)
            .is_err());
        assert!(captured.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn set_preserves_sibling_type_and_key_order() -> Result<(), ConfigError> {
        let writer = FakeWriter::default();
        let captured = writer.captured.clone();
        let service = ConfigService::new(
            FakeReader::new(
                LevelState::Missing,
                LevelState::Present(mapping(vec![
                    ("enabled", Node::Scalar(Scalar::Bool(true))),
                    ("core", mapping(vec![("example", text("old"))])),
                ])),
            ),
            writer,
        );
        service.set(&Key::parse("core.example")?, "new", Level::Personal)?;
        let captured = captured.borrow();
        let (_, Node::Mapping(root)) = &captured[0] else {
            return Err(ConfigError::Io {
                path: "test".to_owned(),
                detail: "expected a mapping document".to_owned(),
            });
        };
        let entries = root.entries();
        assert_eq!(entries[0].0, "enabled");
        assert_eq!(entries[0].1, Node::Scalar(Scalar::Bool(true)));
        assert_eq!(entries[1].0, "core");
        Ok(())
    }

    fn agents_reviewer(value: &str) -> FakeReader {
        FakeReader::new(
            LevelState::Missing,
            LevelState::Present(mapping(vec![(
                "agents",
                mapping(vec![("reviewer", text(value))]),
            )])),
        )
    }

    #[test]
    fn effective_keeps_the_config_source_for_an_explicit_empty_value(
    ) -> Result<(), ConfigError> {
        let resolution = service(agents_reviewer(""))
            .effective(&Key::parse("agents.reviewer")?, None)?;
        assert_eq!(resolution.source(), Source::Personal);
        assert!(resolution.is_from_config());
        assert_eq!(resolution.rendered(), "");
        assert_eq!(resolution.configured_value(), Some(String::new()));
        Ok(())
    }

    #[test]
    fn effective_nonempty_collapses_an_explicit_empty_to_the_catalogue(
    ) -> Result<(), ConfigError> {
        let resolution = service(agents_reviewer(""))
            .effective_nonempty(&Key::parse("agents.reviewer")?, None)?;
        assert_eq!(resolution.source(), Source::Catalogue);
        assert!(!resolution.is_from_config());
        assert_eq!(resolution.rendered(), "accelerator:reviewer");
        assert_eq!(resolution.configured_value(), None);
        Ok(())
    }

    #[test]
    fn effective_source_is_authoritative_independent_of_the_rendered_value(
    ) -> Result<(), ConfigError> {
        let configured_empty = service(agents_reviewer(""))
            .effective(&Key::parse("agents.reviewer")?, None)?;
        assert_eq!(configured_empty.source(), Source::Personal);
        assert_eq!(configured_empty.rendered(), "");

        let absent =
            service(FakeReader::new(LevelState::Missing, LevelState::Missing))
                .effective(&Key::parse("agents.reviewer")?, None)?;
        assert_eq!(absent.source(), Source::Catalogue);
        Ok(())
    }

    #[test]
    fn effective_personal_wins_over_team() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("team"))]),
            )])),
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("personal"))]),
            )])),
        );
        let resolution =
            service(reader).effective(&Key::parse("core.example")?, None)?;
        assert_eq!(resolution.source(), Source::Personal);
        assert_eq!(resolution.rendered(), "personal");
        Ok(())
    }

    #[test]
    fn effective_team_wins_on_personal_absent() -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Present(mapping(vec![(
                "core",
                mapping(vec![("example", text("team"))]),
            )])),
            LevelState::Missing,
        );
        let resolution =
            service(reader).effective(&Key::parse("core.example")?, None)?;
        assert_eq!(resolution.source(), Source::Team);
        assert_eq!(resolution.rendered(), "team");
        Ok(())
    }

    #[test]
    fn effective_catalogue_on_both_absent() -> Result<(), ConfigError> {
        let reader = FakeReader::new(LevelState::Missing, LevelState::Missing);
        let resolution =
            service(reader).effective(&Key::parse("paths.work")?, None)?;
        assert_eq!(resolution.source(), Source::Catalogue);
        assert_eq!(resolution.rendered(), "meta/work");
        Ok(())
    }

    #[test]
    fn effective_is_unset_on_an_unknown_key() -> Result<(), ConfigError> {
        let reader = FakeReader::new(LevelState::Missing, LevelState::Missing);
        let resolution =
            service(reader).effective(&Key::parse("no.such.key")?, None)?;
        assert_eq!(resolution.source(), Source::Unset);
        assert_eq!(resolution.rendered(), "");
        assert_eq!(resolution.configured_value(), None);
        Ok(())
    }

    #[test]
    fn effective_fails_loud_when_the_non_winning_level_is_malformed(
    ) -> Result<(), ConfigError> {
        let reader = FakeReader::new(
            LevelState::Failing,
            LevelState::Present(mapping(vec![("k", text("personal"))])),
        );
        assert!(service(reader).effective(&Key::parse("k")?, None).is_err());
        Ok(())
    }
}

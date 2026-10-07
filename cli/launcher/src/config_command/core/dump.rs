//! The `dump` view: every catalogue key with its effective value and source
//! attribution, plus the ad-hoc integration keys.
//!
//! Source is decided by *presence* at a level, not by value — a key set to its
//! default string still attributes to the level that set it. Credential keys
//! render as `*(set — hidden)*`.

use config::tracker_block::{
    read_block, BlockName, TrackerBlock, TrackerCatalogue,
};
use config::{
    catalogue, ConfigAccess, ConfigError, Key, Level, ReadConfigLevel,
    Resolved, Value,
};

/// The value cell of a dump row.
pub enum Cell {
    /// A concrete value, rendered in backticks.
    Value(String),
    /// A `work.integration` value outside the allow-set; the raw value only,
    /// with the "invalid: must be …" annotation added by the renderer.
    Invalid(String),
    /// A set credential, rendered as `*(set — hidden)*`.
    Hidden,
    /// An unset key, rendered as `*(not set)*`.
    NotSet,
}

/// Where a key's value came from.
#[derive(Clone, Copy)]
pub enum Source {
    Team,
    Local,
    Default,
}

pub struct Row {
    pub key: String,
    pub cell: Cell,
    pub source: Source,
}

/// The dump rows, or `None` when no config file exists at all (the reader emits
/// nothing then).
///
/// # Errors
///
/// A [`ConfigError`] when a config level cannot be read.
pub fn assemble(
    config: &dyn ConfigAccess,
    levels: &dyn ReadConfigLevel,
    trackers: TrackerCatalogue,
) -> Result<Option<Vec<Row>>, ConfigError> {
    let has_config = levels.read(Level::Team)?.is_some()
        || levels.read(Level::Personal)?.is_some();
    if !has_config {
        return Ok(None);
    }
    let mut rows = Vec::new();
    for (key, _) in catalogue::REVIEW_KEYS {
        rows.push(defaulted_row(config, key)?);
    }
    for (key, _) in catalogue::RESEARCH_KEYS {
        rows.push(defaulted_row(config, key)?);
    }
    for name in catalogue::AGENT_KEYS {
        rows.push(defaulted_row(config, &format!("agents.{name}"))?);
    }
    for (key, _) in catalogue::PATH_KEYS {
        rows.push(defaulted_row(config, key)?);
    }
    for key in catalogue::TEMPLATE_KEYS {
        rows.push(optional_row(config, key)?);
    }
    for (key, _) in catalogue::WORK_KEYS {
        rows.push(work_row(config, key)?);
    }
    for (key, _) in catalogue::VISUALISER_KEYS {
        rows.push(defaulted_row(config, key)?);
    }
    for key in catalogue::EXTRA_KEYS {
        rows.push(extra_row(config, key.name)?);
    }
    for name in [BlockName::Pull, BlockName::Push] {
        rows.extend(tracker_block_rows(config, trackers, name)?);
    }
    Ok(Some(rows))
}

/// Read-only rows for the active tracker's `<tracker>.<name>` block: the
/// resolved block flattened to `<tracker>.<name>.<field>` rows when present,
/// else an unset-but-available placeholder per catalogued field. Whole-block
/// replacement shows through the per-row source when a personal block wins.
///
/// A structurally-invalid block is a fail-closed [`ConfigError::Invalid`]
/// refusal here, not a rendered row — the same fail-loud contract `configure`
/// applies to `work.integration`.
fn tracker_block_rows(
    config: &dyn ConfigAccess,
    trackers: TrackerCatalogue,
    name: BlockName,
) -> Result<Vec<Row>, ConfigError> {
    let integration = config
        .effective(&Key::parse("work.integration")?, None)?
        .rendered();
    let Some(block) = trackers.block(&integration, name) else {
        return Ok(Vec::new());
    };
    let prefix = format!("{integration}.{name}");
    let Some((value, level)) = read_block(config, &integration, name)? else {
        return Ok(placeholder_rows(&prefix, block));
    };
    trackers
        .validate(&integration, name, &value)
        .map_err(|error| ConfigError::Invalid {
            detail: error.detail(level),
        })?;
    let source = source_of(config, &prefix)?;
    Ok(block_leaf_rows(&value, &prefix, source))
}

/// Flattens a resolved block to its `<prefix>.<field>` value rows, all
/// attributed to the one source the block resolved from.
fn block_leaf_rows(value: &Value, prefix: &str, source: Source) -> Vec<Row> {
    let mut leaves = Vec::new();
    if let Value::Mapping(entries) = value {
        for (field, child) in entries {
            flatten_block(&format!("{prefix}.{field}"), child, &mut leaves);
        }
    }
    leaves
        .into_iter()
        .map(|(key, value)| Row {
            key,
            cell: Cell::Value(value),
            source,
        })
        .collect()
}

/// The unset-but-available placeholder rows for a block's catalogued fields.
fn placeholder_rows(prefix: &str, block: &TrackerBlock) -> Vec<Row> {
    block
        .fields
        .iter()
        .map(|field| Row {
            key: format!("{prefix}.{}", field.name),
            cell: Cell::NotSet,
            source: Source::Default,
        })
        .collect()
}

/// Flattens a resolved block to leaf `(dotted-key, rendered-value)` pairs,
/// recursing into nested mappings (`filters`, a `max_pages` block) so each
/// scalar or sequence surfaces as its own dotted row.
fn flatten_block(prefix: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Mapping(entries) if !entries.is_empty() => {
            for (key, child) in entries {
                flatten_block(&format!("{prefix}.{key}"), child, out);
            }
        }
        leaf => out.push((prefix.to_owned(), config::render_value(leaf))),
    }
}

fn config_get(
    config: &dyn ConfigAccess,
    key: &str,
    level: Option<Level>,
) -> Result<Option<String>, ConfigError> {
    let parsed = Key::parse(key)?;
    Ok(match config.get(&parsed, level)? {
        Resolved::Found(value) => Some(config::render_value(&value)),
        Resolved::Absent => None,
    })
}

fn source_of(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Source, ConfigError> {
    let parsed = Key::parse(key)?;
    Ok(match config.effective(&parsed, None)?.source() {
        config::Source::Personal => Source::Local,
        config::Source::Team => Source::Team,
        config::Source::Catalogue | config::Source::Unset => Source::Default,
    })
}

fn defaulted_row(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Row, ConfigError> {
    let value = config.effective(&Key::parse(key)?, None)?.rendered();
    Ok(Row {
        key: key.to_owned(),
        cell: Cell::Value(value),
        source: source_of(config, key)?,
    })
}

fn optional_row(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Row, ConfigError> {
    match config_get(config, key, None)? {
        Some(value) if !value.is_empty() => Ok(Row {
            key: key.to_owned(),
            cell: Cell::Value(value),
            source: source_of(config, key)?,
        }),
        _ => Ok(Row {
            key: key.to_owned(),
            cell: Cell::NotSet,
            source: Source::Default,
        }),
    }
}

fn work_row(config: &dyn ConfigAccess, key: &str) -> Result<Row, ConfigError> {
    let value = config.effective(&Key::parse(key)?, None)?.rendered();
    if value.is_empty() {
        return Ok(Row {
            key: key.to_owned(),
            cell: Cell::NotSet,
            source: source_of(config, key)?,
        });
    }
    let cell = if key == "work.integration"
        && !catalogue::is_valid_work_integration(&value)
    {
        Cell::Invalid(value)
    } else {
        Cell::Value(value)
    };
    Ok(Row {
        key: key.to_owned(),
        cell,
        source: source_of(config, key)?,
    })
}

const CREDENTIAL_LEAVES: [&str; 4] =
    ["token", "token_cmd", "api_key", "api_key_cmd"];

fn extra_row(config: &dyn ConfigAccess, key: &str) -> Result<Row, ConfigError> {
    let Some(value) = config_get(config, key, None)?.filter(|v| !v.is_empty())
    else {
        return Ok(Row {
            key: key.to_owned(),
            cell: Cell::NotSet,
            source: Source::Default,
        });
    };
    let leaf = key.rsplit('.').next().unwrap_or(key);
    let cell = if CREDENTIAL_LEAVES.contains(&leaf) {
        Cell::Hidden
    } else {
        Cell::Value(value)
    };
    Ok(Row {
        key: key.to_owned(),
        cell,
        source: source_of(config, key)?,
    })
}

#[cfg(test)]
mod tests {
    use config::tracker_block::{
        BlockName, Field, FieldKind, TrackerBlock, TrackerCatalogue,
    };
    use config::{
        ConfigError, ConfigService, Level, Node, ReadConfigLevel, Scalar,
        WriteConfigLevel,
    };

    use super::{assemble, Cell};

    #[derive(Clone)]
    struct TeamOnly(Node);

    impl ReadConfigLevel for TeamOnly {
        fn read(&self, level: Level) -> Result<Option<Node>, ConfigError> {
            Ok((level == Level::Team).then(|| self.0.clone()))
        }
    }

    struct NoWrites;

    impl WriteConfigLevel for NoWrites {
        fn write(&self, _: Level, _: &Node) -> Result<(), ConfigError> {
            Ok(())
        }
    }

    fn text(value: &str) -> Node {
        Node::Scalar(Scalar::String(value.to_owned()))
    }

    fn int(value: i64) -> Node {
        Node::Scalar(Scalar::Int(value))
    }

    fn mapping(entries: Vec<(&str, Node)>) -> Node {
        Node::Mapping(
            entries
                .into_iter()
                .map(|(key, node)| (key.to_owned(), node))
                .collect(),
        )
    }

    static EXTENDED: &[TrackerBlock] = &[
        TrackerBlock {
            scope: "acme",
            label: "Acme",
            name: BlockName::Pull,
            fields: &[
                Field {
                    name: "extra_boards",
                    kind: FieldKind::EntityList,
                },
                Field {
                    name: "every_board",
                    kind: FieldKind::ScopeFlag,
                },
                Field {
                    name: "labels_by_board",
                    kind: FieldKind::Filters {
                        accepted: &["label"],
                        reserved: &[],
                    },
                },
            ],
            mutually_exclusive: &[],
        },
        TrackerBlock {
            scope: "acme",
            label: "Acme",
            name: BlockName::Push,
            fields: &[Field {
                name: "max_comments",
                kind: FieldKind::Ceiling { allow_zero: false },
            }],
            mutually_exclusive: &[],
        },
    ];

    const CATALOGUE: TrackerCatalogue = TrackerCatalogue(EXTENDED);

    fn acme_rows(
        blocks: Vec<(&str, Node)>,
    ) -> Result<Vec<(String, String)>, ConfigError> {
        let mut team =
            vec![("work", mapping(vec![("integration", text("acme"))]))];
        if !blocks.is_empty() {
            team.push(("acme", mapping(blocks)));
        }
        let reader = TeamOnly(mapping(team));
        let config = ConfigService::new(reader.clone(), NoWrites);
        let rows = assemble(&config, &reader, CATALOGUE)?.unwrap_or_default();
        Ok(rows
            .into_iter()
            .filter(|row| row.key.starts_with("acme."))
            .map(|row| {
                let cell = match row.cell {
                    Cell::Value(value) => value,
                    Cell::NotSet => "(not set)".to_owned(),
                    Cell::Invalid(_) | Cell::Hidden => "(other)".to_owned(),
                };
                (row.key, cell)
            })
            .collect())
    }

    fn owned(rows: &[(&str, &str)]) -> Vec<(String, String)> {
        rows.iter()
            .map(|(key, cell)| ((*key).to_owned(), (*cell).to_owned()))
            .collect()
    }

    #[test]
    fn unset_blocks_list_the_catalogues_fields() -> Result<(), ConfigError> {
        assert_eq!(
            acme_rows(Vec::new())?,
            owned(&[
                ("acme.pull.extra_boards", "(not set)"),
                ("acme.pull.every_board", "(not set)"),
                ("acme.pull.labels_by_board", "(not set)"),
                ("acme.push.max_comments", "(not set)"),
            ])
        );
        Ok(())
    }

    #[test]
    fn an_extension_field_set_well_is_printed() -> Result<(), ConfigError> {
        assert_eq!(
            acme_rows(vec![
                (
                    "pull",
                    mapping(vec![(
                        "labels_by_board",
                        mapping(vec![("label", text("bug"))]),
                    )]),
                ),
                ("push", mapping(vec![("max_comments", int(3))])),
            ])?,
            owned(&[
                ("acme.pull.labels_by_board.label", "bug"),
                ("acme.push.max_comments", "3"),
            ])
        );
        Ok(())
    }

    #[test]
    fn an_extension_field_set_badly_is_structurally_invalid() {
        for (block, extension) in [
            ("pull", mapping(vec![("labels_by_board", text("oops"))])),
            ("push", mapping(vec![("max_comments", int(0))])),
        ] {
            let result = acme_rows(vec![(block, extension)]);
            assert!(
                matches!(
                    &result,
                    Err(ConfigError::Invalid { detail })
                        if detail.contains(&format!("Fix the `{block}` block"))
                ),
                "{block}: {result:?}"
            );
        }
    }
}

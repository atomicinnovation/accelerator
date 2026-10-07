//! The structural syntax of the `<tracker>.pull` and `<tracker>.push` blocks:
//! which fields each block accepts, the shape each field takes, and the
//! validator that runs off that description.
//!
//! The meaning of a well-formed block — ceiling interpretation, defaults,
//! folding into a typed config — belongs to the tracker context, not here.

use std::fmt::Display;
use std::fmt::Formatter;

use crate::error::ConfigError;
use crate::key::Key;
use crate::level::Level;
use crate::render::render_value;
use crate::service::ConfigAccess;
use crate::service::Resolved;
use crate::service::Source;
use crate::service::Value;

/// The written ceiling meaning "no bound".
pub const UNLIMITED: &str = "unlimited";

/// The shape a block field's value takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// The tracker's own list of further entities to broaden discovery onto.
    EntityList,
    /// The tracker's own switch to broaden discovery onto every entity.
    ScopeFlag,
    /// A count bound: an integer or [`UNLIMITED`].
    Ceiling { allow_zero: bool },
    /// Positive page bounds: a scalar sets the `default` cap, a block sets
    /// the `default` cap and any of the per-operation `overrides`.
    PageCaps {
        default: &'static str,
        overrides: &'static [&'static str],
    },
    /// A block of filter fields, each drawn from `accepted`; a `reserved`
    /// field is refused as an unsupported nesting.
    Filters {
        accepted: &'static [&'static str],
        reserved: &'static [&'static str],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub kind: FieldKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockName {
    Pull,
    Push,
}

impl BlockName {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pull => "pull",
            Self::Push => "push",
        }
    }
}

impl Display for BlockName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One tracker's `<scope>.<name>` block. Field order is the order the block's
/// fields are listed to the operator.
#[derive(Debug, PartialEq, Eq)]
pub struct TrackerBlock {
    pub scope: &'static str,
    /// The tracker's display name.
    pub label: &'static str,
    pub name: BlockName,
    pub fields: &'static [Field],
    pub mutually_exclusive: &'static [(&'static str, &'static str)],
}

impl TrackerBlock {
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&'static Field> {
        self.fields.iter().find(|field| field.name == name)
    }

    fn field_of_kind(&self, kind: FieldKind) -> Option<&'static Field> {
        self.fields.iter().find(|field| field.kind == kind)
    }

    fn accepted(&self) -> Vec<String> {
        self.fields
            .iter()
            .map(|field| field.name.to_owned())
            .collect()
    }

    const fn error(&self, fault: BlockFault) -> BlockError {
        BlockError {
            block: self.name,
            fault,
        }
    }

    /// The block's entries once its shape holds: the block is a mapping, and
    /// so is every filters field it sets.
    ///
    /// # Errors
    ///
    /// [`BlockFault::NotAMapping`] or [`BlockFault::SubBlockNotAMapping`].
    pub fn entries<'value>(
        &self,
        value: &'value Value,
    ) -> Result<&'value [(String, Value)], BlockError> {
        let Value::Mapping(entries) = value else {
            return Err(self.error(BlockFault::NotAMapping));
        };
        for (key, value) in entries {
            let is_filters = matches!(
                self.field(key),
                Some(Field {
                    kind: FieldKind::Filters { .. },
                    ..
                })
            );
            if is_filters && !matches!(value, Value::Mapping(_)) {
                return Err(self.error(BlockFault::SubBlockNotAMapping {
                    key: key.clone(),
                }));
            }
        }
        Ok(entries)
    }

    fn first_unrecognised(
        &self,
        entries: &[(String, Value)],
    ) -> Option<String> {
        entries
            .iter()
            .find_map(|(key, value)| match self.field(key) {
                None => Some(key.clone()),
                Some(Field {
                    kind: FieldKind::PageCaps { default, overrides },
                    ..
                }) => match value {
                    Value::Mapping(caps) => caps
                        .iter()
                        .map(|(cap, _)| cap)
                        .find(|cap| {
                            *cap != default
                                && !overrides.contains(&cap.as_str())
                        })
                        .map(|cap| format!("{key}.{cap}")),
                    _ => None,
                },
                Some(_) => None,
            })
    }

    fn exclusivity_fault(
        &self,
        entries: &[(String, Value)],
    ) -> Option<BlockFault> {
        self.mutually_exclusive
            .iter()
            .find(|(first, second)| {
                self.engages(entries, first) && self.engages(entries, second)
            })
            .map(|(first, second)| BlockFault::MutuallyExclusive {
                first: (*first).to_owned(),
                second: (*second).to_owned(),
            })
    }

    fn engages(&self, entries: &[(String, Value)], name: &str) -> bool {
        let Some(value) = setting(entries, name) else {
            return false;
        };
        match self.field(name).map(|field| field.kind) {
            Some(FieldKind::ScopeFlag) => render_value(value) == "true",
            Some(FieldKind::EntityList) => {
                !value.as_string_sequence().is_empty()
            }
            _ => true,
        }
    }

    fn filter_fault(&self, entries: &[(String, Value)]) -> Option<BlockFault> {
        self.fields.iter().find_map(|field| {
            let FieldKind::Filters { accepted, reserved } = field.kind else {
                return None;
            };
            let Some(Value::Mapping(filters)) = setting(entries, field.name)
            else {
                return None;
            };
            filters.iter().find_map(|(key, _)| {
                if reserved.contains(&key.as_str()) {
                    Some(BlockFault::NestedFiltersUnsupported {
                        key: key.clone(),
                    })
                } else if accepted.contains(&key.as_str()) {
                    None
                } else {
                    Some(BlockFault::UnsupportedFilterKey {
                        key: key.clone(),
                        accepted: accepted
                            .iter()
                            .map(|name| (*name).to_owned())
                            .collect(),
                    })
                }
            })
        })
    }

    fn ceiling_fault(&self, entries: &[(String, Value)]) -> Option<BlockFault> {
        self.fields.iter().find_map(|field| {
            let value = setting(entries, field.name)?;
            match field.kind {
                FieldKind::Ceiling { allow_zero } => {
                    bad_ceiling(field.name.to_owned(), value, allow_zero)
                }
                FieldKind::PageCaps { default, overrides } => {
                    page_cap_fault(field.name, value, default, overrides)
                }
                _ => None,
            }
        })
    }
}

fn setting<'value>(
    entries: &'value [(String, Value)],
    name: &str,
) -> Option<&'value Value> {
    entries
        .iter()
        .rev()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn page_cap_fault(
    field: &str,
    value: &Value,
    default: &str,
    overrides: &[&str],
) -> Option<BlockFault> {
    let Value::Mapping(caps) = value else {
        return bad_ceiling(field.to_owned(), value, false);
    };
    std::iter::once(default)
        .chain(overrides.iter().copied())
        .find_map(|cap| {
            let key = if cap == default {
                field.to_owned()
            } else {
                format!("{field}.{cap}")
            };
            bad_ceiling(key, setting(caps, cap)?, false)
        })
}

fn bad_ceiling(
    key: String,
    value: &Value,
    allow_zero: bool,
) -> Option<BlockFault> {
    let token = render_value(value);
    (!is_valid_ceiling(&token, allow_zero)).then_some(BlockFault::BadCeiling {
        key,
        value: token,
        allow_zero,
    })
}

/// Whether a written ceiling is a valid bound: [`UNLIMITED`], or a
/// non-negative integer that is positive unless `allow_zero`.
#[must_use]
pub fn is_valid_ceiling(token: &str, allow_zero: bool) -> bool {
    token == UNLIMITED
        || token
            .parse::<usize>()
            .is_ok_and(|bound| allow_zero || bound > 0)
}

/// The tracker blocks the platform accepts.
#[derive(Debug, Clone, Copy)]
pub struct TrackerCatalogue(pub &'static [TrackerBlock]);

impl TrackerCatalogue {
    #[must_use]
    pub fn block(
        self,
        scope: &str,
        name: BlockName,
    ) -> Option<&'static TrackerBlock> {
        self.0
            .iter()
            .find(|block| block.scope == scope && block.name == name)
    }

    /// Validates a `<scope>.<name>` block's structure, reporting the first
    /// fault in stage order: shape, a wrong-tracker key, an unrecognised key,
    /// exclusivity, filters, then ceilings. A scope with no such block has
    /// no structure to violate.
    ///
    /// # Errors
    ///
    /// The first [`BlockError`] found.
    pub fn validate(
        self,
        scope: &str,
        name: BlockName,
        value: &Value,
    ) -> Result<(), BlockError> {
        let Some(block) = self.block(scope, name) else {
            return Ok(());
        };
        let entries = block.entries(value)?;
        let fault = self
            .wrong_tracker_key(block, entries)
            .or_else(|| {
                block.first_unrecognised(entries).map(|key| {
                    BlockFault::UnrecognisedKey {
                        key,
                        accepted: block.accepted(),
                        hint: None,
                    }
                })
            })
            .or_else(|| block.exclusivity_fault(entries))
            .or_else(|| block.filter_fault(entries))
            .or_else(|| block.ceiling_fault(entries));
        fault.map_or(Ok(()), |fault| Err(block.error(fault)))
    }

    fn wrong_tracker_key(
        self,
        block: &TrackerBlock,
        entries: &[(String, Value)],
    ) -> Option<BlockFault> {
        entries
            .iter()
            .filter(|(key, _)| block.field(key).is_none())
            .find_map(|(key, _)| {
                let hint = self.wrong_tracker_hint(block, key)?;
                Some(BlockFault::UnrecognisedKey {
                    key: key.clone(),
                    accepted: block.accepted(),
                    hint: Some(hint),
                })
            })
    }

    fn wrong_tracker_hint(
        self,
        block: &TrackerBlock,
        key: &str,
    ) -> Option<String> {
        self.0
            .iter()
            .filter(|other| {
                other.name == block.name && other.scope != block.scope
            })
            .find_map(|other| {
                let foreign = other.field(key)?;
                if !matches!(
                    foreign.kind,
                    FieldKind::EntityList | FieldKind::ScopeFlag
                ) {
                    return None;
                }
                let own = block.field_of_kind(foreign.kind)?;
                Some(format!(
                    " `{key}` is a {} key; this integration is {} — did you \
                     mean `{}`?",
                    other.label, block.label, own.name
                ))
            })
    }
}

/// Why a tracker block is structurally invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockError {
    pub block: BlockName,
    pub fault: BlockFault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockFault {
    /// A scalar or sequence where a block was expected. Never silently an
    /// empty block, so a malformed personal block cannot shadow a valid team
    /// block under whole-block replacement.
    NotAMapping,
    SubBlockNotAMapping {
        key: String,
    },
    /// A key outside the block's fields; `hint` names the intended field
    /// when the key is another tracker's.
    UnrecognisedKey {
        key: String,
        accepted: Vec<String>,
        hint: Option<String>,
    },
    MutuallyExclusive {
        first: String,
        second: String,
    },
    NestedFiltersUnsupported {
        key: String,
    },
    UnsupportedFilterKey {
        key: String,
        accepted: Vec<String>,
    },
    BadCeiling {
        key: String,
        value: String,
        allow_zero: bool,
    },
}

impl BlockError {
    /// The operator message, naming the config file the effective block
    /// resolved from.
    #[must_use]
    pub fn detail(&self, level: Level) -> String {
        let block = self.block;
        let fix = format!("Fix the `{block}` block in {}.", level.filename());
        match &self.fault {
            BlockFault::NotAMapping => format!(
                "the `{block}` value must be a block of settings, not a \
                 scalar or list. {fix}"
            ),
            BlockFault::SubBlockNotAMapping { key } => format!(
                "the {block} `{key}` value must be a block of settings. {fix}"
            ),
            BlockFault::UnsupportedFilterKey { key, accepted } => format!(
                "the {block} filter key `{key}` is not supported (accepted: \
                 {}). {fix}",
                accepted.join(", ")
            ),
            BlockFault::NestedFiltersUnsupported { key } => format!(
                "the {block} filter key `{key}` is reserved: nested filters \
                 not yet supported. {fix}"
            ),
            BlockFault::BadCeiling {
                key,
                value,
                allow_zero: true,
            } => format!(
                "{block} `{key}` must be a non-negative integer (0 refuses \
                 all) or `{UNLIMITED}` (got `{value}`). {fix}"
            ),
            BlockFault::BadCeiling {
                key,
                value,
                allow_zero: false,
            } => format!(
                "{block} `{key}` must be a positive integer or `{UNLIMITED}` \
                 (got `{value}`). {fix}"
            ),
            BlockFault::MutuallyExclusive { first, second } => {
                let (first, second) = (family(first), family(second));
                format!(
                    "a {block} block sets both `{first}` and `{second}` — \
                     remove one of `{first}`/`{second}`. {fix}"
                )
            }
            BlockFault::UnrecognisedKey {
                key,
                accepted,
                hint,
            } => format!(
                "the {block} block key `{key}` is not recognised (accepted: \
                 {}).{} {fix}",
                accepted.join(", "),
                hint.as_deref().unwrap_or("")
            ),
        }
    }
}

/// The wildcard naming every tracker's spelling of a field: `all_teams` and
/// `all_projects` are both `all_*`.
fn family(field: &str) -> String {
    let stem = field.split_once('_').map_or(field, |(stem, _)| stem);
    format!("{stem}_*")
}

/// The configured `<scope>.<name>` block and the level it resolved from, or
/// `None` when the block is unset or an empty mapping — each meaning the
/// built-in defaults apply.
///
/// # Errors
///
/// A [`ConfigError`] when a config level cannot be read.
pub fn read_block(
    config: &dyn ConfigAccess,
    scope: &str,
    name: BlockName,
) -> Result<Option<(Value, Level)>, ConfigError> {
    let key = Key::parse(&format!("{scope}.{name}"))?;
    let Resolved::Found(value) = config.get(&key, None)? else {
        return Ok(None);
    };
    if matches!(&value, Value::Mapping(entries) if entries.is_empty()) {
        return Ok(None);
    }
    let level = match config.effective(&key, None)?.source() {
        Source::Personal => Level::Personal,
        _ => Level::Team,
    };
    Ok(Some((value, level)))
}

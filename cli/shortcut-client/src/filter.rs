//! Lowering `shortcut.pull` filters into v4 search queries.
//!
//! The filters keep the meaning Jira and Linear give them: values for one
//! field are OR'd, fields are AND'd, and a `~` value is excluded. Shortcut's
//! search ANDs every operator, so each combination of positive values becomes
//! its own query and the caller unions the results. `filter.rs` is the one
//! place a value is checked before it enters a query; `"` and `\` are refused
//! outright, because Shortcut's handling of an escaped quote is unknown.

use std::fmt;

use crate::new_entity::story_type_for_kind;
use crate::new_entity::StoryType;

const MAX_QUERIES: usize = 10;
const ACCEPTED_TYPES: &str =
    "feature, bug and chore, or the local kinds story, bug, task and spike";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Field {
    Label,
    State,
    Owner,
    Type,
}

impl Field {
    fn named(name: &str) -> Option<Self> {
        match name {
            "label" => Some(Self::Label),
            "state" => Some(Self::State),
            "owner" => Some(Self::Owner),
            "type" => Some(Self::Type),
            _ => None,
        }
    }

    fn operator(self, value: &str) -> String {
        match self {
            Self::Label => format!("label:\"{value}\""),
            Self::State => format!("state:\"{value}\""),
            Self::Owner => format!("owner:{value}"),
            Self::Type => format!("type:{value}"),
        }
    }
}

/// Why a filter cannot be lowered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterRefusal(pub String);

impl fmt::Display for FilterRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for FilterRefusal {}

/// The validated filters of one `shortcut.pull` block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchFilters {
    positive: Vec<(Field, Vec<String>)>,
    negative: Vec<(Field, String)>,
}

impl SearchFilters {
    /// # Errors
    ///
    /// [`FilterRefusal`] for an unknown field or a value search cannot take.
    pub fn parse(pairs: &[(String, String)]) -> Result<Self, FilterRefusal> {
        let mut filters = Self::default();
        for (name, raw) in pairs {
            let field = Field::named(name).ok_or_else(|| {
                FilterRefusal(format!(
                    "the Shortcut pull filter {name:?} is not supported; use \
                     label, state, owner or type"
                ))
            })?;
            let (negated, value) = raw
                .strip_prefix('~')
                .map_or((false, raw.as_str()), |bare| (true, bare));
            let value = lowered_value(field, value)?;
            if negated {
                filters.negative.push((field, value));
            } else if let Some((_, values)) = filters
                .positive
                .iter_mut()
                .find(|(known, _)| *known == field)
            {
                values.push(value);
            } else {
                filters.positive.push((field, vec![value]));
            }
        }
        Ok(filters)
    }

    /// Every state a filter names, positive or negated, for the caller to
    /// check against the workflows in scope.
    #[must_use]
    pub fn states(&self) -> Vec<&str> {
        let positive = self
            .positive
            .iter()
            .filter(|(field, _)| *field == Field::State)
            .flat_map(|(_, values)| values.iter().map(String::as_str));
        let negative = self
            .negative
            .iter()
            .filter(|(field, _)| *field == Field::State)
            .map(|(_, value)| value.as_str());
        positive.chain(negative).collect()
    }

    /// One query per combination of positive values, each scoped to `team`.
    ///
    /// # Errors
    ///
    /// [`FilterRefusal`] for a team name search cannot quote, or a filter
    /// needing more than ten queries.
    pub fn queries(&self, team: &str) -> Result<Vec<String>, FilterRefusal> {
        if holds_unquotable(team) {
            return Err(FilterRefusal(format!(
                "the Shortcut team {team:?} has a name holding \" or \\, which \
                 a search cannot quote; rename it in Shortcut, or scope to \
                 another team"
            )));
        }
        let count: usize = self
            .positive
            .iter()
            .map(|(_, values)| values.len())
            .product();
        if count > MAX_QUERIES {
            return Err(FilterRefusal(format!(
                "the Shortcut pull filters need {count} searches per team, \
                 more than {MAX_QUERIES}; narrow the filters"
            )));
        }
        let mut combinations: Vec<Vec<String>> = vec![Vec::new()];
        for (field, values) in &self.positive {
            combinations = combinations
                .into_iter()
                .flat_map(|chosen| {
                    values.iter().map(move |value| {
                        let mut next = chosen.clone();
                        next.push(field.operator(value));
                        next
                    })
                })
                .collect();
        }
        let negated: Vec<String> = self
            .negative
            .iter()
            .map(|(field, value)| format!("-{}", field.operator(value)))
            .collect();
        Ok(combinations
            .into_iter()
            .map(|chosen| {
                std::iter::once(format!("team:\"{team}\""))
                    .chain(chosen)
                    .chain(negated.iter().cloned())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect())
    }
}

fn holds_unquotable(value: &str) -> bool {
    value.contains('"') || value.contains('\\')
}

fn lowered_value(field: Field, value: &str) -> Result<String, FilterRefusal> {
    if holds_unquotable(value) {
        return Err(FilterRefusal(format!(
            "the Shortcut pull filter value {value:?} holds \" or \\, which a \
             search cannot quote"
        )));
    }
    match field {
        Field::Label | Field::State => Ok(value.to_owned()),
        Field::Owner => {
            let mention = !value.is_empty()
                && value.chars().all(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')
                });
            if mention {
                Ok(value.to_owned())
            } else {
                Err(FilterRefusal(format!(
                    "the Shortcut owner filter {value:?} is not a mention name"
                )))
            }
        }
        Field::Type => StoryType::named(value)
            .or_else(|| story_type_for_kind(value))
            .map(|story_type| story_type.as_str().to_owned())
            .ok_or_else(|| {
                FilterRefusal(format!(
                    "the Shortcut type filter {value:?} is not a story type; \
                     use {ACCEPTED_TYPES}"
                ))
            }),
    }
}

//! The one home of the rule that names a team.
//!
//! A team's **team key** is its mention name, or its display name when it has
//! none. The uuid is the team's identity. The catalogue's team record and the
//! live team model both derive keys here, so the key the engine matches and
//! the key the catalogue resolves cannot drift apart.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TeamKey(String);

impl TeamKey {
    #[must_use]
    pub fn of(mention_name: Option<&str>, name: &str) -> Self {
        let key = mention_name
            .filter(|mention| !mention.is_empty())
            .unwrap_or(name);
        Self(key.to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The parts of a team that decide its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamIdentity {
    pub uuid: String,
    pub mention_name: Option<String>,
    pub name: String,
}

/// Why a team entity from the API names no usable team.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamFieldError {
    Missing(&'static str),
    NotText(&'static str),
}

impl fmt::Display for TeamFieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(field) => write!(formatter, "a team had no {field}"),
            Self::NotText(field) => {
                write!(formatter, "a team's {field} was not text")
            }
        }
    }
}

impl std::error::Error for TeamFieldError {}

impl TeamIdentity {
    /// Reads a team entity from the API. A team needs a text id and a text
    /// name; its mention name is optional.
    ///
    /// # Errors
    ///
    /// [`TeamFieldError`] naming the first field that is missing or not text.
    pub fn from_entity(entity: &Value) -> Result<Self, TeamFieldError> {
        let text = |field: &'static str| {
            entity
                .get(field)
                .ok_or(TeamFieldError::Missing(field))?
                .as_str()
                .map(str::to_owned)
                .ok_or(TeamFieldError::NotText(field))
        };
        Ok(Self {
            uuid: text("id")?,
            name: text("name")?,
            mention_name: entity
                .get("mention_name")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })
    }

    #[must_use]
    pub fn key(&self) -> TeamKey {
        TeamKey::of(self.mention_name.as_deref(), &self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyLookup<'a> {
    Found(&'a str),
    Colliding,
    Unknown,
}

/// Every team's key, and the keys two or more teams share.
#[derive(Debug, Clone, Default)]
pub struct TeamKeys {
    uuids_by_key: BTreeMap<TeamKey, Vec<String>>,
    keys_by_uuid: BTreeMap<String, TeamKey>,
}

impl TeamKeys {
    #[must_use]
    pub fn over(teams: &[TeamIdentity]) -> Self {
        let mut keys = Self::default();
        for team in teams {
            let key = team.key();
            keys.uuids_by_key
                .entry(key.clone())
                .or_default()
                .push(team.uuid.clone());
            keys.keys_by_uuid.insert(team.uuid.clone(), key);
        }
        keys
    }

    #[must_use]
    pub fn lookup(&self, key: &str) -> KeyLookup<'_> {
        match self
            .uuids_by_key
            .get(&TeamKey(key.to_owned()))
            .map(Vec::as_slice)
        {
            Some([uuid]) => KeyLookup::Found(uuid),
            Some([]) | None => KeyLookup::Unknown,
            Some(_) => KeyLookup::Colliding,
        }
    }

    /// The key a team is visible under: its team key, or its uuid when the
    /// key collides, so no two visible teams share one.
    #[must_use]
    pub fn visible_key(&self, uuid: &str) -> Option<String> {
        let key = self.keys_by_uuid.get(uuid)?;
        Some(match self.lookup(key.as_str()) {
            KeyLookup::Found(_) => key.as_str().to_owned(),
            KeyLookup::Colliding | KeyLookup::Unknown => uuid.to_owned(),
        })
    }

    /// Every key that resolves to exactly one team, sorted.
    #[must_use]
    pub fn known(&self) -> Vec<&str> {
        self.uuids_by_key
            .iter()
            .filter(|(_, uuids)| uuids.len() == 1)
            .map(|(key, _)| key.as_str())
            .collect()
    }
}

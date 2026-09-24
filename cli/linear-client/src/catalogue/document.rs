//! The shape of `catalogue.json`: a `baseTeam` pointer into a `teams` array of
//! per-team entries, plus the workspace `labels` no team owns.
//!
//! The legacy `team` and `workflowStates` keys are never stored: they are
//! projected from the base entry on every write, so binaries that predate the
//! per-team shape keep reading the file.

use std::collections::BTreeSet;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use thiserror::Error;

use super::section::CatalogueSection;

/// One team's identity and whichever of its sections the catalogue holds. A
/// section that is `None` has never been fetched, which is distinct from a
/// fetched section with no records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamEntry {
    pub id: String,
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub states: Option<Vec<CataloguedState>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<CataloguedLabel>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<CataloguedMember>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<CataloguedProject>>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl TeamEntry {
    /// A team known only by its identity, none of its sections yet fetched.
    #[must_use]
    pub fn identified(id: &str, key: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            key: key.to_owned(),
            name: name.to_owned(),
            states: None,
            labels: None,
            members: None,
            projects: None,
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CataloguedState {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Linear types this as a `Float` while today's files hold integers, and
    /// only a `Number` round-trips both without rewriting `1002` as `1002.0`.
    pub position: Number,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CataloguedLabel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CataloguedMember {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub email: String,
    pub active: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CataloguedProject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

trait CatalogueRecord {
    fn id(&self) -> &str;
    fn extra_mut(&mut self) -> &mut Map<String, Value>;
}

macro_rules! catalogue_record {
    ($($record:ty),*) => {$(
        impl CatalogueRecord for $record {
            fn id(&self) -> &str {
                &self.id
            }

            fn extra_mut(&mut self) -> &mut Map<String, Value> {
                &mut self.extra
            }
        }
    )*};
}

catalogue_record!(
    CataloguedState,
    CataloguedLabel,
    CataloguedMember,
    CataloguedProject
);

/// How a damaged `teams` entry is treated. A writer must refuse it, since
/// rewriting the file would lose it; a reader skips it and keeps the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    Forgiving,
    Strict,
}

#[derive(Debug, Error)]
pub enum CatalogueParseError {
    #[error("it is not valid JSON ({reason})")]
    NotJson { reason: String },
    #[error("it is not a JSON object")]
    NotAnObject,
    #[error("its `teams` is not an array")]
    TeamsNotAnArray,
    #[error("its `teams[{index}]`{} {reason}", describe_id(id.as_deref()))]
    Entry {
        index: usize,
        id: Option<String>,
        reason: String,
    },
    #[error("its workspace `labels` {reason}")]
    WorkspaceLabels { reason: String },
}

fn describe_id(id: Option<&str>) -> String {
    id.map(|id| format!(" (id {id:?})")).unwrap_or_default()
}

/// What one write records: entries to merge section by section, and
/// optionally a new base team and a fresh set of workspace labels.
#[derive(Debug, Clone, Default)]
pub struct CatalogueUpdate {
    pub base_team: Option<String>,
    pub entries: Vec<TeamEntry>,
    pub workspace_labels: Option<Vec<CataloguedLabel>>,
}

/// What a forgiving read could not parse. A strict read refuses instead, so
/// only a reader's document carries any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CatalogueDamage {
    teams_unreadable: bool,
    sections: BTreeSet<(String, CatalogueSection)>,
    workspace_labels: bool,
}

impl CatalogueDamage {
    pub(super) const fn teams_unreadable(&self) -> bool {
        self.teams_unreadable
    }

    pub(super) fn section_damaged(
        &self,
        team_id: &str,
        section: CatalogueSection,
    ) -> bool {
        self.sections.contains(&(team_id.to_owned(), section))
    }

    pub(super) const fn workspace_labels_damaged(&self) -> bool {
        self.workspace_labels
    }

    pub(super) fn repair(&mut self, team_id: &str, section: CatalogueSection) {
        self.sections.remove(&(team_id.to_owned(), section));
    }

    pub(super) const fn repair_workspace_labels(&mut self) {
        self.workspace_labels = false;
    }

    fn damage_entry(&mut self, team_id: &str) {
        for section in CatalogueSection::PER_TEAM {
            self.sections.insert((team_id.to_owned(), section));
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogueDocument {
    base_team: Option<String>,
    labels: Option<Vec<CataloguedLabel>>,
    teams: Vec<TeamEntry>,
    predates_synced_teams: bool,
    damage: CatalogueDamage,
    extra: Map<String, Value>,
}

const BASE_TEAM: &str = "baseTeam";
const WORKSPACE_LABELS: &str = "labels";
const LEGACY_TEAM: &str = "team";
const TEAMS: &str = "teams";
const LEGACY_STATES: &str = "workflowStates";

impl CatalogueDocument {
    /// Reads a catalogue of any vintage. A file with no `baseTeam` takes its
    /// base team from the legacy `team`, and legacy `workflowStates` fill the
    /// base entry's states only where it has none.
    ///
    /// # Errors
    ///
    /// [`CatalogueParseError`] when the text is not a JSON object, and, under
    /// [`Strictness::Strict`], when `teams` is not an array or one of its
    /// entries or the workspace labels fail typed parsing.
    pub fn parse(
        text: &str,
        strictness: Strictness,
    ) -> Result<Self, CatalogueParseError> {
        let value: Value = serde_json::from_str(text).map_err(|error| {
            CatalogueParseError::NotJson {
                reason: error.to_string(),
            }
        })?;
        let Value::Object(mut object) = value else {
            return Err(CatalogueParseError::NotAnObject);
        };
        let mut damage = CatalogueDamage::default();
        let base_team =
            object.remove(BASE_TEAM).as_ref().and_then(non_blank_string);
        let labels = parse_workspace_labels(
            object.remove(WORKSPACE_LABELS),
            strictness,
            &mut damage,
        )?;
        let legacy_team = object.remove(LEGACY_TEAM);
        let legacy_states = object.remove(LEGACY_STATES);
        let teams_value = object.remove(TEAMS);
        let predates_synced_teams =
            legacy_team.is_some() && teams_value.is_none();
        let teams = match strictness {
            Strictness::Strict => parse_teams_strictly(teams_value)?,
            Strictness::Forgiving => {
                parse_teams_forgivingly(teams_value, &mut damage)
            }
        };

        let mut document = Self {
            base_team,
            labels,
            teams: Vec::new(),
            predates_synced_teams,
            damage,
            extra: object,
        };
        document.merge_entries(teams);
        if document.base_team.is_none() {
            document.adopt_legacy_base(
                legacy_team.as_ref(),
                legacy_states.as_ref(),
            );
        }
        Ok(document)
    }

    /// A document standing in for a file that could not be read at all, so
    /// every question about its teams answers that the catalogue is damaged.
    #[must_use]
    pub(super) fn unreadable() -> Self {
        Self {
            damage: CatalogueDamage {
                teams_unreadable: true,
                workspace_labels: true,
                ..CatalogueDamage::default()
            },
            ..Self::default()
        }
    }

    pub(super) const fn damage(&self) -> &CatalogueDamage {
        &self.damage
    }

    pub(super) const fn damage_mut(&mut self) -> &mut CatalogueDamage {
        &mut self.damage
    }

    pub fn base_team(&self) -> Option<&str> {
        self.base_team.as_deref()
    }

    pub fn base_entry(&self) -> Option<&TeamEntry> {
        let base = self.base_team.as_deref()?;
        self.teams.iter().find(|team| team.id == base)
    }

    /// Every catalogued team, in id order, with same-id entries merged.
    pub fn teams(&self) -> &[TeamEntry] {
        &self.teams
    }

    pub fn workspace_labels(&self) -> Option<&[CataloguedLabel]> {
        self.labels.as_deref()
    }

    /// Whether the file carried the legacy `team` but no `teams` array — the
    /// shape an older binary leaves after overwriting a catalogue that had
    /// synced teams.
    pub const fn predates_synced_teams(&self) -> bool {
        self.predates_synced_teams
    }

    /// Merges `update` in. A section an entry carries replaces the stored one
    /// and a section it omits is kept, so a write never drops data it does
    /// not own. Returns the keys of the teams catalogued for the first time.
    pub fn record(&mut self, update: CatalogueUpdate) -> Vec<String> {
        let mut incoming = Self::default();
        incoming.merge_entries(update.entries);
        let newly_catalogued = incoming
            .teams
            .iter()
            .filter(|entry| !self.teams.iter().any(|team| team.id == entry.id))
            .map(|entry| entry.key.clone())
            .collect();
        self.merge_entries(incoming.teams);
        if let Some(base_team) = update.base_team {
            self.base_team = Some(base_team);
        }
        if let Some(labels) = update.workspace_labels {
            self.labels = Some(replace_records(self.labels.take(), labels));
        }
        newly_catalogued
    }

    /// The file's text: every key in alphabetical order, entries and records
    /// in id order, and the legacy projections derived from the base entry.
    ///
    /// # Errors
    ///
    /// [`serde_json::Error`] if a record cannot be serialised.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut object = self.extra.clone();
        if let Some(base_team) = &self.base_team {
            object.insert(BASE_TEAM.to_owned(), Value::from(base_team.clone()));
        }
        if let Some(labels) = &self.labels {
            object.insert(
                WORKSPACE_LABELS.to_owned(),
                serde_json::to_value(labels)?,
            );
        }
        object.insert(TEAMS.to_owned(), serde_json::to_value(&self.teams)?);
        if let Some(base) = self.base_entry() {
            object.insert(LEGACY_TEAM.to_owned(), legacy_team(base));
            if let Some(states) = &base.states {
                object.insert(
                    LEGACY_STATES.to_owned(),
                    legacy_workflow_states(states),
                );
            }
        }
        let mut text = serde_json::to_string_pretty(&Value::Object(object))?;
        text.push('\n');
        Ok(text)
    }

    fn merge_entries(&mut self, entries: Vec<TeamEntry>) {
        for entry in entries {
            match self.teams.iter_mut().find(|team| team.id == entry.id) {
                Some(stored) => merge_entry(stored, entry),
                None => self.teams.push(sorted_sections(entry)),
            }
        }
        self.teams.sort_by(|left, right| left.id.cmp(&right.id));
    }

    fn adopt_legacy_base(
        &mut self,
        legacy_team: Option<&Value>,
        legacy_states: Option<&Value>,
    ) {
        let Some(id) = legacy_team.and_then(|team| team.get("id")) else {
            return;
        };
        let Some(id) = non_blank_string(id) else {
            return;
        };
        let states = legacy_states.map(parse_legacy_states);
        if let Some(entry) = self.teams.iter_mut().find(|team| team.id == id) {
            if entry.states.is_none() {
                entry.states = states;
            }
        } else {
            let field = |name: &str| {
                legacy_team
                    .and_then(|team| team.get(name))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            self.merge_entries(vec![TeamEntry {
                states,
                ..TeamEntry::identified(&id, &field("key"), &field("name"))
            }]);
        }
        self.base_team = Some(id);
    }
}

fn non_blank_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
}

fn parse_teams_strictly(
    value: Option<Value>,
) -> Result<Vec<TeamEntry>, CatalogueParseError> {
    let entries = match value {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(entries)) => entries,
        Some(_) => return Err(CatalogueParseError::TeamsNotAnArray),
    };
    let mut teams = Vec::with_capacity(entries.len());
    for (index, raw) in entries.into_iter().enumerate() {
        let id = raw.get("id").and_then(Value::as_str).map(str::to_owned);
        let entry = serde_json::from_value::<TeamEntry>(raw)
            .map_err(|error| error.to_string())
            .and_then(|entry| {
                if entry.id.trim().is_empty() {
                    Err("has a blank id".to_owned())
                } else {
                    Ok(entry)
                }
            })
            .map_err(|reason| CatalogueParseError::Entry {
                index,
                id,
                reason,
            })?;
        teams.push(entry);
    }
    Ok(teams)
}

/// Every entry with a usable id, each section parsed on its own so one
/// damaged section never hides the rest of its entry.
fn parse_teams_forgivingly(
    value: Option<Value>,
    damage: &mut CatalogueDamage,
) -> Vec<TeamEntry> {
    match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(entries)) => entries
            .into_iter()
            .filter_map(|raw| parse_entry_forgivingly(raw, damage))
            .collect(),
        Some(_) => {
            damage.teams_unreadable = true;
            Vec::new()
        }
    }
}

fn parse_entry_forgivingly(
    raw: Value,
    damage: &mut CatalogueDamage,
) -> Option<TeamEntry> {
    let Value::Object(mut fields) = raw else {
        return None;
    };
    let id = fields.remove("id").as_ref().and_then(non_blank_string)?;
    let key = take_string(&mut fields, "key");
    let name = take_string(&mut fields, "name");
    let states = take_section(
        &mut fields,
        "states",
        &id,
        CatalogueSection::States,
        damage,
    );
    let labels = take_section(
        &mut fields,
        "labels",
        &id,
        CatalogueSection::Labels,
        damage,
    );
    let members = take_section(
        &mut fields,
        "members",
        &id,
        CatalogueSection::Members,
        damage,
    );
    let projects = take_section(
        &mut fields,
        "projects",
        &id,
        CatalogueSection::Projects,
        damage,
    );
    let (key, name) = match (key, name) {
        (Some(key), Some(name)) => (key, name),
        (key, _) => {
            damage.damage_entry(&id);
            return Some(TeamEntry::identified(
                &id,
                &key.unwrap_or_default(),
                "",
            ));
        }
    };
    Some(TeamEntry {
        id,
        key,
        name,
        states,
        labels,
        members,
        projects,
        extra: fields,
    })
}

fn take_string(fields: &mut Map<String, Value>, name: &str) -> Option<String> {
    match fields.remove(name) {
        Some(Value::String(text)) => Some(text),
        _ => None,
    }
}

fn take_section<R: DeserializeOwned>(
    fields: &mut Map<String, Value>,
    name: &str,
    team_id: &str,
    section: CatalogueSection,
    damage: &mut CatalogueDamage,
) -> Option<Vec<R>> {
    let value = fields.remove(name).filter(|value| !value.is_null())?;
    serde_json::from_value(value)
        .inspect_err(|_| {
            damage.sections.insert((team_id.to_owned(), section));
        })
        .ok()
}

fn parse_workspace_labels(
    value: Option<Value>,
    strictness: Strictness,
    damage: &mut CatalogueDamage,
) -> Result<Option<Vec<CataloguedLabel>>, CatalogueParseError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    match (serde_json::from_value(value), strictness) {
        (Ok(labels), _) => Ok(Some(sorted_records(labels))),
        (Err(_), Strictness::Forgiving) => {
            damage.workspace_labels = true;
            Ok(None)
        }
        (Err(error), Strictness::Strict) => {
            Err(CatalogueParseError::WorkspaceLabels {
                reason: error.to_string(),
            })
        }
    }
}

/// Legacy states the base entry adopts. A state missing a field today's
/// discovery always writes is skipped: the projection is regenerated from the
/// base entry, so it cannot be carried forward faithfully anyway.
fn parse_legacy_states(value: &Value) -> Vec<CataloguedState> {
    let states = value
        .as_array()
        .map(|states| {
            states
                .iter()
                .filter_map(|state| serde_json::from_value(state.clone()).ok())
                .collect()
        })
        .unwrap_or_default();
    sorted_records(states)
}

fn merge_entry(stored: &mut TeamEntry, update: TeamEntry) {
    stored.key = update.key;
    stored.name = update.name;
    stored.extra.extend(update.extra);
    replace_section(&mut stored.states, update.states);
    replace_section(&mut stored.labels, update.labels);
    replace_section(&mut stored.members, update.members);
    replace_section(&mut stored.projects, update.projects);
}

fn replace_section<R: CatalogueRecord>(
    stored: &mut Option<Vec<R>>,
    update: Option<Vec<R>>,
) {
    if let Some(records) = update {
        *stored = Some(replace_records(stored.take(), records));
    }
}

/// The fetched records, each keeping any fields a later version stored on
/// the record with the same id, under the fetched record's own values.
fn replace_records<R: CatalogueRecord>(
    stored: Option<Vec<R>>,
    fetched: Vec<R>,
) -> Vec<R> {
    let mut stored = stored.unwrap_or_default();
    let records = fetched
        .into_iter()
        .map(|mut record| {
            if let Some(previous) = stored
                .iter_mut()
                .find(|previous| previous.id() == record.id())
            {
                let mut extra = std::mem::take(previous.extra_mut());
                extra.extend(std::mem::take(record.extra_mut()));
                *record.extra_mut() = extra;
            }
            record
        })
        .collect();
    sorted_records(records)
}

fn sorted_sections(mut entry: TeamEntry) -> TeamEntry {
    entry.states = entry.states.map(sorted_records);
    entry.labels = entry.labels.map(sorted_records);
    entry.members = entry.members.map(sorted_records);
    entry.projects = entry.projects.map(sorted_records);
    entry
}

fn sorted_records<R: CatalogueRecord>(mut records: Vec<R>) -> Vec<R> {
    records.sort_by(|left, right| left.id().cmp(right.id()));
    records
}

fn legacy_team(base: &TeamEntry) -> Value {
    let mut team = Map::new();
    team.insert("id".to_owned(), Value::from(base.id.clone()));
    team.insert("key".to_owned(), Value::from(base.key.clone()));
    team.insert("name".to_owned(), Value::from(base.name.clone()));
    Value::Object(team)
}

fn legacy_workflow_states(states: &[CataloguedState]) -> Value {
    states
        .iter()
        .filter(|state| state.archived_at.is_none())
        .map(|state| {
            let mut projected = Map::new();
            projected.insert("id".to_owned(), Value::from(state.id.clone()));
            projected
                .insert("name".to_owned(), Value::from(state.name.clone()));
            projected
                .insert("type".to_owned(), Value::from(state.kind.clone()));
            projected.insert(
                "position".to_owned(),
                Value::Number(state.position.clone()),
            );
            Value::Object(projected)
        })
        .collect()
}

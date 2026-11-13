//! The committed, team-shared `catalogue.json`: the workspace, its teams and
//! every workflow with its states.
//!
//! Teammates on different plugin versions share this file, so it changes
//! additively. Every record keeps the fields this client does not know, and a
//! rewrite merges them back; `schema_version` changes only for a breaking
//! change.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::paths::WorkspaceSlug;
use crate::team_key::KeyLookup;
use crate::team_key::TeamIdentity;
use crate::team_key::TeamKeys;

pub const SCHEMA_VERSION: u64 = 1;

type Extra = Map<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub slug: String,
    pub name: String,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub mention_name: Option<String>,
    #[serde(default)]
    pub default_workflow_id: Option<i64>,
    #[serde(default)]
    pub workflow_ids: Vec<i64>,
    #[serde(flatten)]
    pub extra: Extra,
}

impl Team {
    #[must_use]
    pub fn identity(&self) -> TeamIdentity {
        TeamIdentity {
            uuid: self.id.clone(),
            mention_name: self.mention_name.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub states: Vec<WorkflowState>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowState {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: StateType,
    #[serde(default)]
    pub position: i64,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A workflow state's type. Shortcut documents four; an unseen one is kept
/// rather than refused, because v4 is a beta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum StateType {
    Backlog,
    Unstarted,
    Started,
    Done,
    Other(String),
}

impl From<String> for StateType {
    fn from(text: String) -> Self {
        match text.as_str() {
            "backlog" => Self::Backlog,
            "unstarted" => Self::Unstarted,
            "started" => Self::Started,
            "done" => Self::Done,
            _ => Self::Other(text),
        }
    }
}

impl From<StateType> for String {
    fn from(kind: StateType) -> Self {
        match kind {
            StateType::Backlog => "backlog".to_owned(),
            StateType::Unstarted => "unstarted".to_owned(),
            StateType::Started => "started".to_owned(),
            StateType::Done => "done".to_owned(),
            StateType::Other(text) => text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Document {
    schema_version: u64,
    workspace: Workspace,
    #[serde(default)]
    teams: Vec<Team>,
    #[serde(default)]
    workflows: Vec<Workflow>,
    #[serde(flatten)]
    extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogueError {
    Unparseable { detail: String },
    NewerSchema { found: u64 },
    OlderSchema { found: u64 },
    Invalid { field: String, detail: String },
}

impl fmt::Display for CatalogueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unparseable { detail } => write!(
                formatter,
                "E_CATALOGUE_UNPARSEABLE: catalogue.json could not be read: \
                 {detail}. Restore it from version control, or run \
                 `accelerator shortcut init discover`"
            ),
            Self::NewerSchema { found } => write!(
                formatter,
                "E_CATALOGUE_NEWER: catalogue.json has schema_version {found}, \
                 newer than this plugin's {SCHEMA_VERSION}. A teammate wrote \
                 it with a newer plugin; upgrade the plugin to read it"
            ),
            Self::OlderSchema { found } => write!(
                formatter,
                "E_CATALOGUE_OLDER: catalogue.json has schema_version {found}, \
                 older than this plugin's {SCHEMA_VERSION}. Run \
                 `accelerator shortcut init discover` to rewrite it"
            ),
            Self::Invalid { field, detail } => write!(
                formatter,
                "E_CATALOGUE_INVALID: catalogue.json field {field} {detail}. \
                 Run `accelerator shortcut init discover` to rewrite it"
            ),
        }
    }
}

impl std::error::Error for CatalogueError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamLookupError {
    Unknown { key: String, known: Vec<String> },
    Colliding { key: String },
}

impl fmt::Display for TeamLookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown { key, known } => write!(
                formatter,
                "E_TEAM_UNKNOWN: no Shortcut team has the key {key:?}. Known \
                 team keys: {}. Run `accelerator shortcut init list-teams` to \
                 see every team",
                if known.is_empty() {
                    "none".to_owned()
                } else {
                    known.join(", ")
                }
            ),
            Self::Colliding { key } => write!(
                formatter,
                "E_TEAM_AMBIGUOUS: two Shortcut teams have the key {key:?}. \
                 1. In Shortcut, give one team a mention name, or rename it. \
                 2. Run `accelerator shortcut init discover`. 3. Set \
                 shortcut.team_key or shortcut.pull.additional_teams to the \
                 team's new key. 4. Then commit catalogue.json"
            ),
        }
    }
}

impl std::error::Error for TeamLookupError {}

/// A catalogue whose every request-bound value has been validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    document: Document,
    slug: WorkspaceSlug,
}

impl Catalogue {
    /// # Errors
    ///
    /// [`CatalogueError::Invalid`] naming the first value that cannot be put
    /// in a request.
    pub fn new(
        workspace: Workspace,
        teams: Vec<Team>,
        workflows: Vec<Workflow>,
    ) -> Result<Self, CatalogueError> {
        Self::validated(Document {
            schema_version: SCHEMA_VERSION,
            workspace,
            teams,
            workflows,
            extra: Extra::new(),
        })
    }

    /// # Errors
    ///
    /// [`CatalogueError`] for text that is not a catalogue, a schema version
    /// this client does not read, or a value that fails validation.
    pub fn parse(text: &str) -> Result<Self, CatalogueError> {
        let unparseable =
            |detail: String| CatalogueError::Unparseable { detail };
        let value: Value = serde_json::from_str(text)
            .map_err(|error| unparseable(error.to_string()))?;
        let found = value
            .get("schema_version")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                unparseable("it has no schema_version".to_owned())
            })?;
        if found > SCHEMA_VERSION {
            return Err(CatalogueError::NewerSchema { found });
        }
        if found < SCHEMA_VERSION {
            return Err(CatalogueError::OlderSchema { found });
        }
        let document: Document = serde_json::from_value(value)
            .map_err(|error| unparseable(error.to_string()))?;
        Self::validated(document)
    }

    /// # Errors
    ///
    /// [`CatalogueError::Unparseable`] if the catalogue cannot be serialised.
    pub fn to_json(&self) -> Result<String, CatalogueError> {
        let mut text =
            serde_json::to_string_pretty(&self.document).map_err(|error| {
                CatalogueError::Unparseable {
                    detail: error.to_string(),
                }
            })?;
        text.push('\n');
        Ok(text)
    }

    #[must_use]
    pub const fn schema_version(&self) -> u64 {
        self.document.schema_version
    }

    #[must_use]
    pub const fn slug(&self) -> &WorkspaceSlug {
        &self.slug
    }

    #[must_use]
    pub const fn workspace(&self) -> &Workspace {
        &self.document.workspace
    }

    #[must_use]
    pub fn teams(&self) -> &[Team] {
        &self.document.teams
    }

    #[must_use]
    pub fn workflows(&self) -> &[Workflow] {
        &self.document.workflows
    }

    #[must_use]
    pub fn workflow(&self, id: i64) -> Option<&Workflow> {
        self.document
            .workflows
            .iter()
            .find(|workflow| workflow.id == id)
    }

    #[must_use]
    pub fn team_keys(&self) -> TeamKeys {
        let identities: Vec<TeamIdentity> =
            self.document.teams.iter().map(Team::identity).collect();
        TeamKeys::over(&identities)
    }

    /// # Errors
    ///
    /// [`TeamLookupError`] for a key no team has, or one two teams share.
    pub fn team_for_key(&self, key: &str) -> Result<&Team, TeamLookupError> {
        let keys = self.team_keys();
        match keys.lookup(key) {
            KeyLookup::Found(uuid) => self
                .document
                .teams
                .iter()
                .find(|team| team.id == uuid)
                .ok_or_else(|| unknown(key, &keys)),
            KeyLookup::Colliding => Err(TeamLookupError::Colliding {
                key: key.to_owned(),
            }),
            KeyLookup::Unknown => Err(unknown(key, &keys)),
        }
    }

    /// This catalogue, with every field `previous` holds and this one does not
    /// merged back in, matched record by record.
    #[must_use]
    pub fn merged_over(mut self, previous: &Self) -> Self {
        let old = &previous.document;
        let new = &mut self.document;
        keep_unknown(&mut new.extra, &old.extra);
        keep_unknown(&mut new.workspace.extra, &old.workspace.extra);
        for team in &mut new.teams {
            if let Some(before) =
                old.teams.iter().find(|before| before.id == team.id)
            {
                keep_unknown(&mut team.extra, &before.extra);
            }
        }
        for workflow in &mut new.workflows {
            let Some(before) =
                old.workflows.iter().find(|before| before.id == workflow.id)
            else {
                continue;
            };
            keep_unknown(&mut workflow.extra, &before.extra);
            for state in &mut workflow.states {
                if let Some(earlier) =
                    before.states.iter().find(|earlier| earlier.id == state.id)
                {
                    keep_unknown(&mut state.extra, &earlier.extra);
                }
            }
        }
        self
    }

    fn validated(document: Document) -> Result<Self, CatalogueError> {
        let invalid = |field: String, detail: &str| CatalogueError::Invalid {
            field,
            detail: detail.to_owned(),
        };
        let slug =
            WorkspaceSlug::parse(&document.workspace.slug).map_err(|_| {
                invalid(
                    "workspace.slug".to_owned(),
                    "is not lowercase letters, digits and hyphens",
                )
            })?;
        for (index, team) in document.teams.iter().enumerate() {
            if !is_uuid(&team.id) {
                return Err(invalid(
                    format!("teams[{index}].id"),
                    "is not a uuid",
                ));
            }
            if team
                .workflow_ids
                .iter()
                .chain(&team.default_workflow_id)
                .any(|id| *id <= 0)
            {
                return Err(invalid(
                    format!("teams[{index}].workflow_ids"),
                    "holds an id that is not a positive integer",
                ));
            }
        }
        for (index, workflow) in document.workflows.iter().enumerate() {
            if workflow.id <= 0 {
                return Err(invalid(
                    format!("workflows[{index}].id"),
                    "is not a positive integer",
                ));
            }
            for (position, state) in workflow.states.iter().enumerate() {
                if state.id <= 0 {
                    return Err(invalid(
                        format!("workflows[{index}].states[{position}].id"),
                        "is not a positive integer",
                    ));
                }
            }
        }
        Ok(Self { document, slug })
    }
}

fn unknown(key: &str, keys: &TeamKeys) -> TeamLookupError {
    TeamLookupError::Unknown {
        key: key.to_owned(),
        known: keys.known().into_iter().map(str::to_owned).collect(),
    }
}

fn keep_unknown(fresh: &mut Extra, previous: &Extra) {
    for (field, value) in previous {
        fresh.entry(field.clone()).or_insert_with(|| value.clone());
    }
}

fn is_uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 5
        && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, length)| {
            group.len() == length
                && group.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

//! Discovering the workspace's teams and every workflow, for `init discover`.
//!
//! Each team's workflows and each workflow's states come from their own
//! listing endpoints rather than the nested lists an entity carries, which v4
//! may cut short. Discovery has no page cap — only its deadline stops it — and
//! it writes nothing.

use std::fmt;

use reqwest::Url;
use serde_json::Map;
use serde_json::Value;

use crate::cache::Listing;
use crate::catalogue::Catalogue;
use crate::catalogue::Team;
use crate::catalogue::Workflow;
use crate::catalogue::WorkflowState;
use crate::catalogue::Workspace;
use crate::classify::explain;
use crate::error::ClientError;
use crate::paging::collect;
use crate::paging::PageError;
use crate::paging::Pages;
use crate::paths::Paths;
use crate::transport::Deadline;
use crate::transport::Received;
use crate::transport::Transport;
use crate::workspace::whoami;
use crate::workspace::WhoAmI;

#[derive(Debug)]
pub enum DiscoveryError {
    Refused { status: u16, detail: String },
    Unusable { what: String },
    Client(ClientError),
    RepeatedPage { url: String },
}

impl DiscoveryError {
    #[must_use]
    pub fn refused(received: &Received, scopes: &[String]) -> Self {
        Self::Refused {
            status: received.status,
            detail: explain(received, scopes),
        }
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused { detail, .. } => write!(formatter, "E_DISCOVER_REFUSED: {detail}"),
            Self::Unusable { what } => write!(formatter, "E_DISCOVER_UNUSABLE: {what}"),
            Self::Client(error) => error.fmt(formatter),
            Self::RepeatedPage { url } => write!(
                formatter,
                "E_DISCOVER_LOOP: Shortcut returned the same next page twice ({url})"
            ),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// A catalogue built from one discovery, and whether every listing finished.
#[derive(Debug)]
pub struct Discovered {
    pub catalogue: Catalogue,
    pub listing: Listing,
}

/// # Errors
///
/// [`DiscoveryError`] for a failed or unusable answer.
pub fn discover(
    transport: &Transport,
    base: &Url,
    deadline: &Deadline,
) -> Result<(WhoAmI, Discovered), DiscoveryError> {
    let me = whoami(transport, base, deadline)?;
    let paths = Paths::new(base.clone(), me.slug.clone());
    let mut listing = Listing::Complete;
    let mut list =
        |what: &str, url: &Url| -> Result<Vec<Value>, DiscoveryError> {
            let pages = collect(transport, &paths, url, None, deadline)
                .map_err(|error| page_failure(error, &me.scopes))?;
            record_cut(&mut listing, what, &pages);
            Ok(pages.entities)
        };

    let mut teams = Vec::new();
    for entity in list("teams", &paths.teams())? {
        let mut team = team(&entity)?;
        team.workflow_ids =
            list("team workflows", &paths.team_workflows(&team.id))?
                .iter()
                .filter_map(|workflow| {
                    workflow.get("id").and_then(Value::as_i64)
                })
                .collect();
        teams.push(team);
    }
    let mut workflows = Vec::new();
    for entity in list("workflows", &paths.workflows())? {
        let mut workflow = workflow(&entity)?;
        workflow.states =
            list("workflow states", &paths.workflow_states(workflow.id))?
                .iter()
                .map(state)
                .collect::<Result<_, _>>()?;
        workflows.push(workflow);
    }

    let workspace = Workspace {
        slug: me.slug.as_str().to_owned(),
        name: me.workspace_name.clone(),
        extra: Map::new(),
    };
    let catalogue =
        Catalogue::new(workspace, teams, workflows).map_err(|error| {
            DiscoveryError::Unusable {
                what: error.to_string(),
            }
        })?;
    Ok((me, Discovered { catalogue, listing }))
}

fn record_cut(listing: &mut Listing, what: &str, pages: &Pages) {
    if let (Listing::Complete, Some(cut)) = (&*listing, pages.cut) {
        *listing = Listing::Incomplete {
            listing: what.to_owned(),
            cause: cut.cause().to_owned(),
        };
    }
}

fn page_failure(error: PageError, scopes: &[String]) -> DiscoveryError {
    match error {
        PageError::Refused(received) => {
            DiscoveryError::refused(&received, scopes)
        }
        PageError::Unusable(what) => DiscoveryError::Unusable { what },
        PageError::Client(error) => DiscoveryError::Client(error),
        PageError::Repeated(url) => DiscoveryError::RepeatedPage { url },
    }
}

fn field<'a>(
    entity: &'a Value,
    name: &str,
    of: &str,
) -> Result<&'a Value, DiscoveryError> {
    entity.get(name).ok_or_else(|| DiscoveryError::Unusable {
        what: format!("a {of} had no {name}"),
    })
}

fn string(
    entity: &Value,
    name: &str,
    of: &str,
) -> Result<String, DiscoveryError> {
    field(entity, name, of)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| DiscoveryError::Unusable {
            what: format!("a {of}'s {name} was not text"),
        })
}

fn integer(
    entity: &Value,
    name: &str,
    of: &str,
) -> Result<i64, DiscoveryError> {
    field(entity, name, of)?
        .as_i64()
        .ok_or_else(|| DiscoveryError::Unusable {
            what: format!("a {of}'s {name} was not an integer"),
        })
}

fn team(entity: &Value) -> Result<Team, DiscoveryError> {
    Ok(Team {
        id: string(entity, "id", "team")?,
        name: string(entity, "name", "team")?,
        mention_name: entity
            .get("mention_name")
            .and_then(Value::as_str)
            .map(str::to_owned),
        default_workflow_id: entity
            .pointer("/default_workflow/id")
            .and_then(Value::as_i64),
        workflow_ids: Vec::new(),
        extra: Map::new(),
    })
}

fn workflow(entity: &Value) -> Result<Workflow, DiscoveryError> {
    Ok(Workflow {
        id: integer(entity, "id", "workflow")?,
        name: string(entity, "name", "workflow")?,
        states: Vec::new(),
        extra: Map::new(),
    })
}

fn state(entity: &Value) -> Result<WorkflowState, DiscoveryError> {
    Ok(WorkflowState {
        id: integer(entity, "id", "workflow state")?,
        name: string(entity, "name", "workflow state")?,
        kind: string(entity, "type", "workflow state")?.into(),
        position: entity
            .get("position")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        extra: Map::new(),
    })
}

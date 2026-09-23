//! Init discovery.
//!
//! The three queries that populate the on-disk caches, returned as the cache
//! shapes rather than written. Writing, locking and scaffold upkeep are
//! composition concerns and live in [`crate::cache`].
//!
//! The interactive team selection is not ported — a client must not prompt —
//! so [`LinearClient::list_teams`] exposes the choices for the skill to
//! render.

use serde_json::json;
use serde_json::Value;

use crate::client::LinearClient;
use crate::surface::interpret;
use crate::surface::SurfaceError;

const VIEWER: &str = "query { viewer { id name } }";
const TEAMS: &str = "query($cursor: String) {
    teams(first: 250, after: $cursor) {
      nodes { id name key }
      pageInfo { hasNextPage endCursor }
    }
  }";
const TEAM_STATES: &str = "query($id: String!) {
    team(id: $id) {
      id name key
      states { nodes { id name type position } }
    }
  }";

impl LinearClient {
    /// Verifies credentials against `viewer` and returns the `viewer.json`
    /// shape `{id, name}`.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure, an `errors[]` response, or a
    /// response carrying no viewer id.
    pub fn discover_viewer(&self) -> Result<Value, SurfaceError> {
        let received = self.transport().send(VIEWER, &json!({}))?;
        let body = interpret(&received, "discover viewer")?;
        let id = string_at(&body, "/data/viewer/id").ok_or_else(|| {
            SurfaceError::BadResponse {
                operation: "discover viewer",
                reason: "the response carried no viewer id".to_owned(),
            }
        })?;
        let name = string_at(&body, "/data/viewer/name").unwrap_or_default();
        Ok(json!({ "id": id, "name": name }))
    }

    /// Lists the teams the token can see, as the array the skill renders for
    /// selection. Paginated to exhaustion so a workspace exceeding one page is
    /// fully enumerated.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure or an `errors[]` response.
    pub fn list_teams(&self) -> Result<Value, SurfaceError> {
        Ok(Value::Array(self.paginate_teams()?))
    }

    /// Every visible team node, following the Relay cursor to exhaustion.
    ///
    /// Fails loud on any page failure rather than returning a subset: a
    /// truncated enumeration that dropped a visible team would falsely abort a
    /// valid `additional_teams` or under-scope `all_teams`.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure or an `errors[]` response on
    /// any page.
    pub(crate) fn paginate_teams(&self) -> Result<Vec<Value>, SurfaceError> {
        let mut nodes = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let received =
                self.transport().send(TEAMS, &json!({ "cursor": cursor }))?;
            let body = interpret(&received, "list teams")?;
            let teams = body.pointer("/data/teams");
            if let Some(page_nodes) = teams
                .and_then(|teams| teams.get("nodes"))
                .and_then(Value::as_array)
            {
                nodes.extend(page_nodes.iter().cloned());
            }
            let page_info = teams.and_then(|teams| teams.get("pageInfo"));
            let has_next = page_info
                .and_then(|info| info.get("hasNextPage"))
                .and_then(Value::as_bool)
                == Some(true);
            cursor = page_info
                .and_then(|info| info.get("endCursor"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            if !has_next || cursor.is_none() {
                break;
            }
        }
        Ok(nodes)
    }

    /// Discovers a team's workflow states and returns the `catalogue.json`
    /// shape `{team:{id,key,name}, workflowStates:[{id,name,type,position}]}`.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure, an `errors[]` response, or a
    /// team that is not found or has no workflow states.
    pub fn discover_team(&self, team_id: &str) -> Result<Value, SurfaceError> {
        let received = self
            .transport()
            .send(TEAM_STATES, &json!({ "id": team_id }))?;
        let body = interpret(&received, "discover team")?;

        let team = body.pointer("/data/team");
        let resolved_id = team
            .and_then(|team| team.get("id"))
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty());
        let nodes = team
            .and_then(|team| team.pointer("/states/nodes"))
            .and_then(Value::as_array);
        let (Some(_), Some(nodes)) = (resolved_id, nodes) else {
            return Err(SurfaceError::BadResponse {
                operation: "discover team",
                reason: format!(
                    "team {team_id} was not found or has no workflow states"
                ),
            });
        };
        if nodes.is_empty() {
            return Err(SurfaceError::BadResponse {
                operation: "discover team",
                reason: format!("team {team_id} has no workflow states"),
            });
        }

        let states: Vec<Value> = nodes.iter().map(state_entry).collect();
        Ok(json!({
            "team": {
                "id": team.and_then(|team| team.get("id")),
                "key": team.and_then(|team| team.get("key")),
                "name": team.and_then(|team| team.get("name")),
            },
            "workflowStates": states,
        }))
    }
}

fn state_entry(state: &Value) -> Value {
    json!({
        "id": state.get("id"),
        "name": state.get("name"),
        "type": state.get("type"),
        "position": state.get("position"),
    })
}

fn string_at(body: &Value, pointer: &str) -> Option<String> {
    body.pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

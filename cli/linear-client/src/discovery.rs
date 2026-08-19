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
const TEAMS: &str = "query { teams { nodes { id name key } } }";
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
    /// selection.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure or an `errors[]` response.
    pub fn list_teams(&self) -> Result<Value, SurfaceError> {
        let received = self.transport().send(TEAMS, &json!({}))?;
        let body = interpret(&received, "list teams")?;
        Ok(body
            .pointer("/data/teams/nodes")
            .cloned()
            .unwrap_or_else(|| json!([])))
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

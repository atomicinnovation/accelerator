//! Workflow transitions.
//!
//! Unlike Jira there is no live transition lookup: the state name resolves to a
//! UUID directly from the cached catalogue, and the same `issueUpdate` mutation
//! the port's `update` uses carries it, with `stateId` as the input field.

use serde_json::json;
use serde_json::Value;

use crate::client::LinearClient;
use crate::surface::interpret;
use crate::surface::SurfaceError;

const ISSUE_UPDATE: &str = "mutation($id: String!, $input: IssueUpdateInput!) {
    issueUpdate(id: $id, input: $input) {
      success issue { id identifier state { name } }
    }
  }";

impl LinearClient {
    /// Resolves a state name to its UUID through the catalogue, refusing a name
    /// that matches no state or more than one.
    ///
    /// # Errors
    ///
    /// [`SurfaceError::UnknownState`] or [`SurfaceError::AmbiguousState`].
    pub fn resolve_state(&self, name: &str) -> Result<String, SurfaceError> {
        let matches = self.states().resolve_all(name);
        match matches.as_slice() {
            [id] => Ok(id.clone()),
            [] => Err(SurfaceError::UnknownState {
                name: name.to_owned(),
            }),
            _ => Err(SurfaceError::AmbiguousState {
                name: name.to_owned(),
                count: matches.len(),
            }),
        }
    }

    /// Transitions an issue to the named state.
    ///
    /// # Errors
    ///
    /// The resolution errors above, plus a transport failure or a response
    /// carrying `errors[]`.
    pub fn transition(
        &self,
        identifier: &str,
        state_name: &str,
    ) -> Result<Value, SurfaceError> {
        let state_id = self.resolve_state(state_name)?;
        let variables = json!({
            "id": identifier,
            "input": { "stateId": state_id },
        });
        let received = self.transport().send(ISSUE_UPDATE, &variables)?;
        interpret(&received, "transition")
    }
}

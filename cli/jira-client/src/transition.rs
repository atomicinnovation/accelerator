//! Workflow transitions: a GET to list the available transitions, a
//! case-insensitive name match, then a POST to apply the resolved transition.

use reqwest::Method;
use serde_json::json;
use serde_json::Value;

use crate::adf::markdown_to_document;
use crate::client::JiraClient;
use crate::surface::SurfaceError;

/// One available transition: its numeric id and the state it leads to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub id: String,
    pub name: String,
}

/// What to transition to: a state name (resolved via a GET) or a known
/// numeric transition id (which skips the lookup).
pub enum Target {
    State(String),
    Id(String),
}

impl JiraClient {
    /// Lists the transitions available from the issue's current state.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a refused key, a transport failure or a non-2xx
    /// status.
    pub fn list_transitions(
        &self,
        key: &str,
    ) -> Result<Vec<Transition>, SurfaceError> {
        let path = Self::issue_path(key, "/transitions")?;
        let received = self.transport().send(&Method::GET, &path, &[], None)?;
        if received.status < 200 || received.status >= 300 {
            return Err(SurfaceError::status(
                "transition lookup",
                received.status,
                received.body,
            ));
        }
        let parsed: Value =
            serde_json::from_str(&received.body).map_err(|error| {
                SurfaceError::BadResponse {
                    operation: "transition lookup",
                    reason: format!("the response was not JSON: {error}"),
                }
            })?;
        Ok(parse_transitions(&parsed))
    }

    /// Resolves `target` to a transition id, refusing a name that matches no
    /// transition or more than one.
    ///
    /// # Errors
    ///
    /// [`SurfaceError::TransitionNotFound`] or
    /// [`SurfaceError::TransitionAmbiguous`], plus the lookup errors above.
    pub fn resolve_transition(
        &self,
        key: &str,
        target: &Target,
    ) -> Result<String, SurfaceError> {
        match target {
            Target::Id(id) => Ok(id.clone()),
            Target::State(state) => {
                let matches: Vec<Transition> = self
                    .list_transitions(key)?
                    .into_iter()
                    .filter(|transition| {
                        transition.name.eq_ignore_ascii_case(state)
                    })
                    .collect();
                match matches.len() {
                    0 => Err(SurfaceError::TransitionNotFound {
                        state: state.clone(),
                    }),
                    1 => Ok(matches[0].id.clone()),
                    _ => Err(SurfaceError::TransitionAmbiguous {
                        state: state.clone(),
                        matches,
                    }),
                }
            }
        }
    }

    /// Applies a transition, optionally setting a resolution and folding a
    /// comment into `update.comment[].add`.
    ///
    /// # Errors
    ///
    /// The resolution errors above, a body that will not convert, a transport
    /// failure, or a non-2xx status.
    pub fn transition(
        &self,
        key: &str,
        target: &Target,
        resolution: Option<&str>,
        comment: Option<&str>,
        notify: bool,
    ) -> Result<(), SurfaceError> {
        let transition_id = self.resolve_transition(key, target)?;
        let path = Self::issue_path(key, "/transitions")?;
        let payload = transition_payload(&transition_id, resolution, comment)?;
        let body =
            serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_owned());
        let query: Vec<(&str, &str)> = if notify {
            Vec::new()
        } else {
            vec![("notifyUsers", "false")]
        };
        let received =
            self.transport()
                .send(&Method::POST, &path, &query, Some(&body))?;
        if received.status >= 200 && received.status < 300 {
            return Ok(());
        }
        Err(SurfaceError::status(
            "transition",
            received.status,
            received.body,
        ))
    }
}

fn parse_transitions(response: &Value) -> Vec<Transition> {
    response
        .get("transitions")
        .and_then(Value::as_array)
        .map(|transitions| {
            transitions
                .iter()
                .filter_map(|transition| {
                    let id = transition.get("id").and_then(Value::as_str)?;
                    let name = transition
                        .pointer("/to/name")
                        .and_then(Value::as_str)?;
                    Some(Transition {
                        id: id.to_owned(),
                        name: name.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn transition_payload(
    transition_id: &str,
    resolution: Option<&str>,
    comment: Option<&str>,
) -> Result<Value, SurfaceError> {
    let mut payload = json!({ "transition": { "id": transition_id } });
    if let Some(resolution) = resolution {
        payload["fields"] = json!({ "resolution": { "name": resolution } });
    }
    if let Some(comment) = comment {
        let document = markdown_to_document(comment, None)?;
        payload["update"] =
            json!({ "comment": [{ "add": { "body": document } }] });
    }
    Ok(payload)
}

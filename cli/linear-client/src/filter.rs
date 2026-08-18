//! `IssueFilter` composition, transcribed from `linear-search-flow.sh:128-155`.
//!
//! Linear's complexity is scored at 0.1 per property plus 1 per object,
//! multiplied by a connection's page size, with a hard rejection above 10,000
//! points — so an explicit `first:` is **always** passed and never left to the
//! API's default of 50.

use serde_json::json;
use serde_json::Map;
use serde_json::Value;

use crate::error::ClientError;

/// Resolves a workflow-state name to its UUID.
///
/// A port from the outset so the cache-backed implementation can land later
/// without a constructor change.
pub trait StateResolver {
    fn resolve(&self, name: &str) -> Option<String>;
}

/// A fixed map, used by the search suites and by any caller with no catalogue.
#[derive(Debug, Clone, Default)]
pub struct FixedStates(pub std::collections::BTreeMap<String, String>);

impl StateResolver for FixedStates {
    fn resolve(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

/// Everything the search surface accepts, in the bash's own shape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    pub team_id: Option<String>,
    pub state: Option<String>,
    pub assignee: Option<String>,
    pub label: Option<String>,
    pub text: Option<String>,
}

/// The page size a fetch requests. 250 is Linear's bulk ceiling and the value
/// `work-item-fetch-remote.sh:61` uses.
pub const FETCH_PAGE_SIZE: u32 = 250;

/// Composes the filter.
///
/// Merge order is the oracle's: state, assignee, label, then title.
///
/// # Errors
///
/// [`ClientError::UnknownState`] when a state name is not in the catalogue —
/// a refusal rather than a filter over the literal name, which would silently
/// return the wrong issue set.
pub fn compose(
    search: &Search,
    states: &dyn StateResolver,
) -> Result<Value, ClientError> {
    let mut filter = Map::new();
    if let Some(team) = &search.team_id {
        filter.insert("team".to_owned(), json!({"id": {"eq": team}}));
    }
    if let Some(state) = &search.state {
        let id =
            states
                .resolve(state)
                .ok_or_else(|| ClientError::UnknownState {
                    name: state.clone(),
                })?;
        filter.insert("state".to_owned(), json!({"id": {"eq": id}}));
    }
    if let Some(assignee) = &search.assignee {
        filter.insert(
            "assignee".to_owned(),
            json!({"name": {"eqIgnoreCase": assignee}}),
        );
    }
    if let Some(label) = &search.label {
        filter.insert("labels".to_owned(), json!({"name": {"eq": label}}));
    }
    if let Some(text) = &search.text {
        filter.insert("title".to_owned(), json!({"containsIgnoreCase": text}));
    }
    Ok(Value::Object(filter))
}

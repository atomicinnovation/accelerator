//! `IssueFilter` composition.
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
pub trait StateResolver {
    /// The single UUID a name resolves to, or `None` when it names no state or
    /// more than one.
    fn resolve(&self, name: &str) -> Option<String>;

    /// Every UUID whose display name matches, so a caller can tell an unknown
    /// name from an ambiguous one — the distinction the transition flow draws
    /// between `E_TRANSITION_STATE_NOT_IN_CATALOGUE` and
    /// `E_TRANSITION_STATE_AMBIGUOUS`.
    fn resolve_all(&self, name: &str) -> Vec<String> {
        self.resolve(name).into_iter().collect()
    }
}

/// A fixed map, used by the search suites and by any caller with no catalogue.
#[derive(Debug, Clone, Default)]
pub struct FixedStates(pub std::collections::BTreeMap<String, String>);

impl StateResolver for FixedStates {
    fn resolve(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

/// Resolves a team **key** — the `ENG` in `ENG-42` — to the team UUID a search
/// filter needs.
///
/// A separate job from [`StateResolver`]: a config names the team by its key,
/// but the `{team:{id:{eq:…}}}` filter is evaluated against the UUID, so an
/// unresolved key silently matches no team.
pub trait TeamResolver {
    /// The team UUID `key` resolves to, or `None` when it matches no team.
    fn resolve(&self, key: &str) -> Option<String>;
}

/// A fixed map, used by the search suites and by any caller with no catalogue.
#[derive(Debug, Clone, Default)]
pub struct FixedTeam(pub std::collections::BTreeMap<String, String>);

impl TeamResolver for FixedTeam {
    fn resolve(&self, key: &str) -> Option<String> {
        self.0.get(key.trim()).cloned()
    }
}

/// Everything the search surface accepts, in its own shape.
///
/// `state`, `assignee` and `label` carry a value list so a filter key configured
/// with several values lowers to the `in` operator (values OR'd); a single value
/// keeps its `eq` form. An empty list is an unset filter. `team_id` and `text`
/// stay single: the team is the base scope, and text is a contains-match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    pub team_id: Option<String>,
    pub state: Vec<String>,
    pub assignee: Vec<String>,
    pub label: Vec<String>,
    pub text: Option<String>,
}

/// The page size a fetch requests. 250 is Linear's bulk ceiling.
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
    if !search.state.is_empty() {
        let ids =
            search
                .state
                .iter()
                .map(|name| {
                    states.resolve(name).ok_or_else(|| {
                        ClientError::UnknownState { name: name.clone() }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
        filter.insert("state".to_owned(), json!({"id": comparator(&ids)}));
    }
    if !search.assignee.is_empty() {
        filter.insert(
            "assignee".to_owned(),
            json!({"name": ignore_case_comparator(&search.assignee)}),
        );
    }
    if !search.label.is_empty() {
        filter.insert(
            "labels".to_owned(),
            json!({"name": comparator(&search.label)}),
        );
    }
    if let Some(text) = &search.text {
        filter.insert("title".to_owned(), json!({"containsIgnoreCase": text}));
    }
    Ok(Value::Object(filter))
}

/// One value lowers to `eq`; several lower to `in` (OR'd). Empty is never passed
/// here — the caller skips an unset filter.
fn comparator(values: &[String]) -> Value {
    match values {
        [single] => json!({"eq": single}),
        many => json!({"in": many}),
    }
}

/// The case-insensitive counterpart for `assignee`: a single value keeps its
/// `eqIgnoreCase` form; several fall back to a case-sensitive `in`, Linear's only
/// multi-value operator on a name.
fn ignore_case_comparator(values: &[String]) -> Value {
    match values {
        [single] => json!({"eqIgnoreCase": single}),
        many => json!({"in": many}),
    }
}

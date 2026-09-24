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
use crate::resolution::ResolverSet;
use crate::resolution::SingleResolution;

/// Everything the search surface accepts, in its own shape.
///
/// `state`, `assignee` and `label` carry a value list so a filter key
/// configured with several values lowers to the `in` operator (values OR'd); a
/// single value keeps its `eq` form. An empty list is an unset filter.
/// `team_id` and `text` stay single: the team is the base scope, and text is a
/// contains-match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    pub team_id: Option<String>,
    /// Further team UUIDs to broaden discovery onto. With `team_id` set and
    /// this non-empty, the base and each additional team lower to one
    /// `team: { id: { in: [...] } }` clause; empty, `team_id` alone lowers to
    /// `team: { id: { eq } }`.
    pub team_ids: Vec<String>,
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
/// [`ClientError::UnknownState`] when a state name does not resolve to exactly
/// one base-team state — a refusal rather than a filter over the literal name,
/// which would silently return the wrong issue set.
pub fn compose(
    search: &Search,
    resolvers: &ResolverSet,
) -> Result<Value, ClientError> {
    let mut filter = Map::new();
    let teams: Vec<&String> = search
        .team_id
        .iter()
        .chain(search.team_ids.iter())
        .collect();
    match teams.as_slice() {
        [] => {}
        [single] => {
            filter.insert("team".to_owned(), json!({"id": {"eq": single}}));
        }
        many => {
            filter.insert("team".to_owned(), json!({"id": {"in": many}}));
        }
    }
    if !search.state.is_empty() {
        let ids = search
            .state
            .iter()
            .map(|name| match resolvers.team_states().resolve(name) {
                SingleResolution::Resolved(id) => Ok(id),
                SingleResolution::Unresolved(_) => {
                    Err(ClientError::UnknownState { name: name.clone() })
                }
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

/// One value lowers to `eq`; several lower to `in` (OR'd). Empty is never
/// passed here — the caller skips an unset filter.
fn comparator(values: &[String]) -> Value {
    match values {
        [single] => json!({"eq": single}),
        many => json!({"in": many}),
    }
}

/// The case-insensitive counterpart for `assignee`: a single value keeps its
/// `eqIgnoreCase` form; several fall back to a case-sensitive `in`, Linear's
/// only multi-value operator on a name.
fn ignore_case_comparator(values: &[String]) -> Value {
    match values {
        [single] => json!({"eqIgnoreCase": single}),
        many => json!({"in": many}),
    }
}

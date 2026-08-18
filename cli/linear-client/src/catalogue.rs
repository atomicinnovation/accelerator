//! The cache-backed [`StateResolver`], reading workflow states out of the
//! `catalogue.json` that discovery writes.
//!
//! Name matching is case-insensitive and trimmed, transcribed from
//! `linear-transition-flow.sh:116-134`: every state whose display name matches
//! is collected, so a name two states share resolves ambiguously rather than
//! silently picking one.

use std::path::Path;

use serde_json::Value;

use crate::filter::StateResolver;

/// Workflow states loaded from a catalogue, each a display name paired with its
/// UUID.
#[derive(Debug, Clone, Default)]
pub struct CatalogueStates {
    states: Vec<(String, String)>,
}

impl CatalogueStates {
    /// Loads the states from `<integrations_root>/linear/catalogue.json`,
    /// yielding an empty resolver when the catalogue is absent or unreadable —
    /// the same "no catalogue, nothing resolves" outcome the bash reaches when
    /// `catalogue.json` is missing.
    #[must_use]
    pub fn load(integrations_root: &Path) -> Self {
        let path = integrations_root.join("linear/catalogue.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .map(|catalogue| Self::from_catalogue(&catalogue))
            .unwrap_or_default()
    }

    /// Builds a resolver directly from a parsed catalogue.
    #[must_use]
    pub fn from_catalogue(catalogue: &Value) -> Self {
        let states = catalogue
            .get("workflowStates")
            .and_then(Value::as_array)
            .map(|states| {
                states
                    .iter()
                    .filter_map(|state| {
                        let name = state.get("name").and_then(Value::as_str)?;
                        let id = state.get("id").and_then(Value::as_str)?;
                        Some((name.to_owned(), id.to_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { states }
    }
}

fn normalise(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

impl StateResolver for CatalogueStates {
    fn resolve(&self, name: &str) -> Option<String> {
        let mut matches = self.resolve_all(name);
        match matches.len() {
            1 => matches.pop(),
            _ => None,
        }
    }

    fn resolve_all(&self, name: &str) -> Vec<String> {
        let wanted = normalise(name);
        self.states
            .iter()
            .filter(|(state_name, _)| normalise(state_name) == wanted)
            .map(|(_, id)| id.clone())
            .collect()
    }
}

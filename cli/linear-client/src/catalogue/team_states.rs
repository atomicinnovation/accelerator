//! The base team's workflow states, the only states a transition may target.

use super::document::CatalogueDocument;
use super::entries::normalise;
use super::section::CatalogueSection;
use crate::resolution::CatalogueGap;
use crate::resolution::NameResolver;
use crate::resolution::NameUnresolved;
use crate::resolution::SingleResolution;

/// Each non-archived base-team state as its normalised name and id.
pub struct TeamStates {
    states: Result<Vec<(String, String)>, CatalogueGap>,
}

impl TeamStates {
    #[must_use]
    pub fn of(document: &CatalogueDocument) -> Self {
        Self {
            states: base_team_states(document),
        }
    }
}

fn base_team_states(
    document: &CatalogueDocument,
) -> Result<Vec<(String, String)>, CatalogueGap> {
    let damage = document.damage();
    if damage.teams_unreadable() {
        return Err(CatalogueGap::Damaged);
    }
    let base = document.base_entry().ok_or(CatalogueGap::Absent)?;
    if damage.section_damaged(&base.id, CatalogueSection::States) {
        return Err(CatalogueGap::Damaged);
    }
    let states = base.states.as_ref().ok_or(CatalogueGap::Absent)?;
    Ok(states
        .iter()
        .filter(|state| state.archived_at.is_none())
        .map(|state| (normalise(&state.name), state.id.clone()))
        .collect())
}

impl NameResolver for TeamStates {
    fn resolve(&self, value: &str) -> SingleResolution {
        let states = match &self.states {
            Ok(states) => states,
            Err(gap) => {
                return SingleResolution::Unresolved(
                    NameUnresolved::NotCatalogued(*gap),
                );
            }
        };
        let wanted = normalise(value);
        let matched: Vec<&str> = states
            .iter()
            .filter(|(name, _)| !wanted.is_empty() && *name == wanted)
            .map(|(_, id)| id.as_str())
            .collect();
        match matched.as_slice() {
            [] => SingleResolution::Unresolved(NameUnresolved::NotFound),
            [id] => SingleResolution::Resolved((*id).to_owned()),
            many => SingleResolution::Unresolved(NameUnresolved::Ambiguous {
                count: many.len(),
            }),
        }
    }
}

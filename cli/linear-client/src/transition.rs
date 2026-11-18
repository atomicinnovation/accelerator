//! The workflow state a transition targets.
//!
//! Unlike Jira there is no live transition lookup: the state name resolves to a
//! UUID directly from the cached catalogue, and an update carries it as its
//! `stateId`.

use crate::client::LinearClient;
use crate::resolution::NameUnresolved;
use crate::resolution::SingleResolution;
use crate::surface::SurfaceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateId(String);

impl StateId {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl LinearClient {
    /// Resolves a state name to its UUID among the base team's states,
    /// refusing a name that matches no state or more than one.
    ///
    /// # Errors
    ///
    /// [`SurfaceError::UnknownState`] or [`SurfaceError::AmbiguousState`].
    pub fn resolve_state(&self, name: &str) -> Result<StateId, SurfaceError> {
        match self.resolvers().team_states().resolve(name) {
            SingleResolution::Resolved(id) => Ok(StateId(id)),
            SingleResolution::Unresolved(
                NameUnresolved::NotFound | NameUnresolved::NotCatalogued(_),
            ) => Err(SurfaceError::UnknownState {
                name: name.to_owned(),
            }),
            SingleResolution::Unresolved(NameUnresolved::Ambiguous {
                count,
            }) => Err(SurfaceError::AmbiguousState {
                name: name.to_owned(),
                count,
            }),
        }
    }
}

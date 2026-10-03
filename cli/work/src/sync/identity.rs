//! The actions a sync run's identity pass reports, beside the engine's own.

/// What the identity pass did to one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityAction {
    /// The item's remote issue now answers to another key.
    KeyChanged,
    /// The tracker answered that no issue has the item's stored key.
    NotFound,
    /// A retirement an earlier run left unfinished was completed.
    Resumed,
}

impl IdentityAction {
    #[must_use]
    pub fn from_keyword(raw: &str) -> Option<Self> {
        Some(match raw {
            "key-changed" => Self::KeyChanged,
            "not-found" => Self::NotFound,
            "resumed" => Self::Resumed,
            _ => return None,
        })
    }
}

impl std::fmt::Display for IdentityAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::KeyChanged => "key-changed",
            Self::NotFound => "not-found",
            Self::Resumed => "resumed",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::IdentityAction;

    #[test]
    fn every_action_round_trips_through_its_keyword() {
        for action in [
            IdentityAction::KeyChanged,
            IdentityAction::NotFound,
            IdentityAction::Resumed,
        ] {
            assert_eq!(
                IdentityAction::from_keyword(&action.to_string()),
                Some(action)
            );
        }
    }
}

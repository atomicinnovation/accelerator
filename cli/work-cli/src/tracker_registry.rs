//! Resolves a `RemoteTracker` from the `work.integration` config key —
//! the composition root selecting `Box<dyn RemoteTracker>`, above the port.

use tracker::RemoteTracker;

pub enum SelectionError {
    Unset,
    Unrecognised { name: String },
    NotAvailable { name: String },
}

impl SelectionError {
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Unset => "work.integration is not set. Recognised \
                trackers: linear, jira, trello, github-issues. Configure \
                one with /accelerator:configure."
                .to_owned(),
            Self::Unrecognised { name } => format!(
                "work.integration names an unrecognised tracker: '{name}'. \
                 Recognised trackers: linear, jira, trello, github-issues."
            ),
            Self::NotAvailable { name } => format!(
                "work.integration names '{name}', which is recognised but \
                 has no client wired yet."
            ),
        }
    }
}

pub trait TrackerRegistry {
    /// # Errors
    ///
    /// [`SelectionError`] when `name` is empty, unrecognised, or names a
    /// tracker with no client built.
    fn resolve(
        &self,
        name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError>;
}

/// The production registry. Every provider is unwired until 0171: the four
/// trackers the create/update bridges' own dispatch taxonomy recognises
/// (`linear`, `jira`, `trello`, `github-issues`) are all not-yet-available;
/// everything else falls to `Unrecognised`.
pub struct ConfiguredTrackers;

impl TrackerRegistry for ConfiguredTrackers {
    fn resolve(
        &self,
        name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
        match name {
            "" => Err(SelectionError::Unset),
            "linear" | "jira" | "trello" | "github-issues" => {
                Err(SelectionError::NotAvailable {
                    name: name.to_owned(),
                })
            }
            other => Err(SelectionError::Unrecognised {
                name: other.to_owned(),
            }),
        }
    }
}

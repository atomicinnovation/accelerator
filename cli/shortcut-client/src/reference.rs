//! What a user hands `show`: a key, or the URL Shortcut's web app gives a
//! story or an epic.

use std::fmt;

use reqwest::Url;

use crate::paths::WorkspaceSlug;
use crate::shortcut_key::canonical_number;
use crate::shortcut_key::EpicId;
use crate::shortcut_key::ShortcutKey;
use crate::shortcut_key::StoryId;

const APP_HOST: &str = "app.shortcut.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutReference {
    Key(ShortcutKey),
    Url { workspace: String, key: ShortcutKey },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceRefusal {
    Unrecognised(String),
    ForeignWorkspace { found: String, configured: String },
}

impl fmt::Display for ReferenceRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unrecognised(text) => write!(
                formatter,
                "{text:?} is not a Shortcut key or entity URL — expected \
                 sc-<id>, epic-<id>, or an https://{APP_HOST} story or epic URL"
            ),
            Self::ForeignWorkspace { found, configured } => write!(
                formatter,
                "the URL names the Shortcut workspace {found:?}, but this \
                 repository is configured for {configured:?}"
            ),
        }
    }
}

impl std::error::Error for ReferenceRefusal {}

impl ShortcutReference {
    /// # Errors
    ///
    /// [`ReferenceRefusal::Unrecognised`] for text that is neither a key nor
    /// a story or epic URL on Shortcut's web app.
    pub fn parse(text: &str) -> Result<Self, ReferenceRefusal> {
        ShortcutKey::parse(text).map(Self::Key).or_else(|_| {
            entity_url(text)
                .map(|(workspace, key)| Self::Url { workspace, key })
                .ok_or_else(|| ReferenceRefusal::Unrecognised(text.to_owned()))
        })
    }

    /// # Errors
    ///
    /// [`ReferenceRefusal::ForeignWorkspace`] for a URL into a workspace
    /// other than `configured`.
    pub fn key_in(
        self,
        configured: &WorkspaceSlug,
    ) -> Result<ShortcutKey, ReferenceRefusal> {
        match self {
            Self::Key(key) => Ok(key),
            Self::Url { workspace, key }
                if workspace == configured.as_str() =>
            {
                Ok(key)
            }
            Self::Url { workspace, .. } => {
                Err(ReferenceRefusal::ForeignWorkspace {
                    found: workspace,
                    configured: configured.as_str().to_owned(),
                })
            }
        }
    }
}

fn entity_url(text: &str) -> Option<(String, ShortcutKey)> {
    let url = Url::parse(text).ok()?;
    if url.scheme() != "https" || url.host_str() != Some(APP_HOST) {
        return None;
    }
    let mut segments = url.path_segments()?;
    let workspace = segments.next().filter(|slug| !slug.is_empty())?;
    let collection = segments.next()?;
    let number = canonical_number(segments.next()?)?;
    let key = match collection {
        "story" => ShortcutKey::Story(StoryId::new(number)),
        "epic" => ShortcutKey::Epic(EpicId::new(number)),
        _ => return None,
    };
    Some((workspace.to_owned(), key))
}

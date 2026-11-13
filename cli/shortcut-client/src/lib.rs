//! The Shortcut provider client, over Shortcut's REST API v4.
//!
//! No ADF layer: Shortcut stores Markdown, so a body passes through verbatim
//! in both directions.

pub mod shortcut_key;

pub use crate::shortcut_key::Collection;
pub use crate::shortcut_key::EpicId;
pub use crate::shortcut_key::NotAStory;
pub use crate::shortcut_key::ShortcutKey;
pub use crate::shortcut_key::StoryId;
pub use crate::shortcut_key::UnparseableKey;

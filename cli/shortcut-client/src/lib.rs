//! The Shortcut provider client, over Shortcut's REST API v4.
//!
//! No ADF layer: Shortcut stores Markdown, so a body passes through verbatim
//! in both directions.

pub mod attach;
pub mod auth;
pub mod cache;
pub mod catalogue;
pub mod classify;
pub mod client;
pub mod comment;
pub mod discovery;
pub mod error;
pub mod filter;
pub mod new_entity;
pub mod paging;
pub mod paths;
pub mod reference;
pub mod shortcut_key;
pub mod story_state;
pub mod team_key;
pub mod transition;
pub mod transport;
pub mod update;
pub mod workspace;
pub mod writes;

pub use crate::auth::Credentials;
pub use crate::client::ShortcutClient;
pub use crate::error::ClientError;
pub use crate::new_entity::story_type_for_kind;
pub use crate::new_entity::NewEntity;
pub use crate::new_entity::StoryType;
pub use crate::new_entity::UnknownKind;
pub use crate::shortcut_key::Collection;
pub use crate::shortcut_key::EpicId;
pub use crate::shortcut_key::NotAStory;
pub use crate::shortcut_key::ShortcutKey;
pub use crate::shortcut_key::StoryId;
pub use crate::shortcut_key::UnparseableKey;
pub use reqwest::Method;
pub use reqwest::Url;

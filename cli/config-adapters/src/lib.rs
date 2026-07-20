//! Outbound adapters for the configuration hexagon.
//!
//! Owns every serde, YAML, and filesystem concern the serde-free `config` core
//! is kept clear of: frontmatter splitting, the typed YAML (de)serialization,
//! project-root discovery, and the atomic write. The `config` core sees only
//! the `Node` tree these adapters hand it.

mod compose;
mod document;
mod render;
mod store;

pub mod legacy;

pub use compose::compose;
pub use render::{render_resolved, render_value, ABSENT_SENTINEL};
pub use store::{FileConfigStore, LegacyPolicy};

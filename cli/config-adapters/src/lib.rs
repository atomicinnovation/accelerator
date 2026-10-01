//! Outbound adapters for the configuration hexagon.
//!
//! Owns every serde, YAML, and filesystem concern the serde-free `config` core
//! is kept clear of: frontmatter splitting, the typed YAML (de)serialization,
//! project-root discovery, the atomic write, and the credential ladder's
//! environment, filesystem, and helper-process ports. The `config` core sees
//! only the `Node` tree and the port traits these adapters hand it.

mod compose;
mod document;
mod render;
mod store;

pub mod credentials;
pub mod legacy;

pub use compose::{compose, Composed};
pub use render::{render_resolved, render_value, ABSENT_SENTINEL};
pub use store::{plugin_root_from_env, FileConfigStore, LegacyPolicy};

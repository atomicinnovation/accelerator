//! The configuration domain core: value objects, ports, the precedence
//! resolution and nested-path walk, the recognised-key catalogue, and the pure
//! legacy-layout predicate.
//!
//! Depends on no infrastructure — no serde, YAML, or filesystem crate enters
//! its closure. Those concerns live in the `config-adapters` crate.

pub mod catalogue;
pub mod error;
pub mod key;
pub mod legacy;
pub mod level;
pub mod node;
pub mod service;

pub use crate::error::ConfigError;
pub use crate::error::Existing;
pub use crate::key::Key;
pub use crate::level::Level;
pub use crate::node::Mapping;
pub use crate::node::Node;
pub use crate::node::Scalar;
pub use crate::service::ConfigAccess;
pub use crate::service::ConfigService;
pub use crate::service::ReadConfigLevel;
pub use crate::service::Resolved;
pub use crate::service::Value;
pub use crate::service::WriteConfigLevel;

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

pub use error::{ConfigError, Existing};
pub use key::Key;
pub use level::Level;
pub use node::{Mapping, Node, Scalar};
pub use service::{
    ConfigAccess, ConfigService, ReadConfigLevel, Resolved, Value,
    WriteConfigLevel,
};

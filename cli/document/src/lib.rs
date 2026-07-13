//! The markdown-with-YAML-frontmatter document protocol.
//!
//! One implementation of fence splitting, frontmatter parsing, and round-trip
//! rendering, shared by every adapter that reads or writes a meta document.
//! serde-saphyr is confined to this crate.

pub mod error;
pub mod fence;
pub mod parse;
pub mod render;
mod tags;
pub mod value;

pub use crate::error::DocumentError;
pub use crate::fence::{fence_offsets, split, Split};
pub use crate::parse::parse;
pub use crate::render::render;
pub use crate::value::{Mapping, Scalar, Yaml};

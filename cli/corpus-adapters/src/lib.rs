//! Outbound adapters and the imperative shell for the corpus hexagon.
//!
//! Translates the `document` format layer into the `corpus` domain, supplies
//! the regex-backed scanner and the config-sourced doc-type table, and invokes
//! the pure `corpus` conventions. The domain algorithms live in `corpus`; this
//! crate owns only the infra boundary and the orchestration.

pub mod doc_type;
pub mod document;
pub mod patcher;
pub mod scanner;

pub use crate::document::{parse, FrontmatterState, ParsedDocument};
pub use crate::patcher::{patch_status, PatchError};
pub use crate::scanner::RegexScanner;

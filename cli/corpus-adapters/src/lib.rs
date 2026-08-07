//! Outbound adapters and the imperative shell for the corpus hexagon.
//!
//! Translates the `document` format layer into the `corpus` domain, supplies
//! the regex-backed scanner and the config-sourced doc-type table, and invokes
//! the pure `corpus` conventions. The domain algorithms live in `corpus`; this
//! crate owns only the infra boundary and the orchestration.

pub mod assemble;
pub mod doc_type;
pub mod document;
mod jsonl;
pub mod lock;
pub mod metadata;
pub mod patcher;
pub mod scanner;
pub mod store;
pub mod work_item_pattern;

pub use crate::assemble::{assemble, AssembledDocument};
pub use crate::document::{parse, FrontmatterState, ParsedDocument};
pub use crate::lock::{acquire, LockGuard, LockOptions};
pub use crate::metadata::{derive, derive_at, ClockError, SystemClock};
pub use crate::patcher::{patch_status, PatchError};
pub use crate::scanner::RegexScanner;
pub use crate::store::FileCorpusStore;
pub use crate::work_item_pattern::{
    canonicalise_id, compile_format_string, compile_scan_regex, parse_full_id,
    pattern_max_number, ParsedId, PatternError,
};

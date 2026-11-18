//! Outbound adapters and the imperative shell for the corpus hexagon.
//!
//! Translates the `document` format layer into the `corpus` domain, supplies
//! the config-sourced doc-type table, and invokes the pure `corpus`
//! conventions. The domain algorithms live in `corpus`; this
//! crate owns only the infra boundary and the orchestration.

pub mod body;
pub mod doc_type;
pub mod document;
pub mod fs;
pub mod jsonl;
pub mod lock;
pub mod metadata;
pub mod patcher;
pub mod recovery;
pub mod resolve;
pub mod store;

pub use crate::body::CommonMarkBody;
pub use crate::document::{
    parse, FrontmatterState, ParsedDocument, YamlFrontmatter,
};
pub use crate::fs::{RealFs, TypeDirectoryLister};
pub use crate::lock::LockdirLock;
pub use crate::metadata::{
    derive, derive_at, ClockError, SystemClock, VcsBackedRepoFactsProbe,
};
pub use crate::patcher::{patch_status, PatchError};
pub use crate::recovery::FileRecoveryCopies;
pub use crate::store::FileCorpusStore;

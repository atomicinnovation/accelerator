//! How a set names its documents, findings, level notes and reports on disk.

pub mod entry;
pub mod finding_path;
pub mod lineage;
pub mod note_ref;
pub mod slug;
pub mod stem;

pub const BRIEF: &str = "brief.md";
pub const OUTLINE: &str = "outline.md";
pub const MANIFEST: &str = "manifest.md";
pub const SYNTHESIS: &str = "synthesis.md";
pub const FINDINGS: &str = "findings";
pub const REPORTS: &str = "reports";

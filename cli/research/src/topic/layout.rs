//! How a set names its documents, findings, level notes and reports on disk.

pub mod entry;
pub mod finding_path;
pub mod lineage;
pub mod note_ref;
pub mod slug;
pub mod stem;

pub use research_declarations::topic::BRIEF;
pub use research_declarations::topic::FINDINGS;
pub use research_declarations::topic::LEVELS_SUFFIX;
pub use research_declarations::topic::MANIFEST;
pub use research_declarations::topic::OUTLINE;
pub use research_declarations::topic::REPORTS;
pub use research_declarations::topic::SYNTHESIS;

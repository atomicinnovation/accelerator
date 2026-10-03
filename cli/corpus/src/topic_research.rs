//! Conventions of the `topic-research` document type's set layout.

mod finding_path;
mod lineage;
mod stem;

pub use crate::topic_research::finding_path::is_finding_path;
pub use crate::topic_research::finding_path::is_level_note_path;
pub use crate::topic_research::lineage::Lineage;
pub use crate::topic_research::stem::Stem;

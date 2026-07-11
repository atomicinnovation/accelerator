//! The meta-corpus domain: the serde-free value model, the doc-type fact and
//! its inference matcher, typed-linkage parsing and resolution, the slug and
//! work-item-ID conventions, and the artifact-metadata contract.
//!
//! Kernel-only: no serde, YAML, regex, or filesystem crate enters its closure.
//! The convention algorithms take infra-sourced data (a compiled scanner, a
//! doc-type table, a parsed frontmatter value) by injection.

pub mod doc_type;
pub mod metadata;
pub mod slug;
pub mod typed_ref;
pub mod value;
pub mod work_item_id;

pub use crate::doc_type::DocTypeKey;
pub use crate::metadata::ArtifactMetadata;
pub use crate::metadata::Clock;
pub use crate::metadata::FilenameTimestampFormat;
pub use crate::typed_ref::parse_typed_ref;
pub use crate::typed_ref::TypedRef;
pub use crate::value::FrontmatterValue;
pub use crate::value::Mapping;
pub use crate::value::Scalar;
pub use crate::work_item_id::IdScan;
pub use crate::work_item_id::IdScanner;
pub use crate::work_item_id::WorkItemIdScheme;

//! The meta-corpus domain: the serde-free value model, the doc-type fact and
//! its inference matcher, typed-linkage parsing and resolution, the slug
//! convention, and the artifact-metadata contract.
//!
//! Kernel-only: no serde, YAML, regex, or filesystem crate enters its closure.
//! Work-item IDs arrive through the `IdConvention` and `LinkageIds` ports, and
//! the convention algorithms take the other infra-sourced data (a doc-type
//! table, a parsed frontmatter value) by injection.

pub mod adr;
pub mod body;
pub mod cluster;
pub mod doc_type;
pub mod frontmatter;
pub mod frontmatter_validation;
pub mod id_convention;
pub mod linkage;
pub mod linkage_ids;
pub mod lock;
pub mod metadata;
pub mod record;
pub mod references;
pub mod relative_links;
pub mod resolve;
pub mod scan;
pub mod slug;
pub mod store;
pub mod typed_ref;
pub mod value;

pub use crate::doc_type::DocTypeKey;
pub use crate::id_convention::IdConvention;
pub use crate::linkage::Band;
pub use crate::linkage::LinkageRecord;
pub use crate::linkage_ids::LinkageIds;
pub use crate::metadata::ArtifactMetadata;
pub use crate::metadata::Clock;
pub use crate::metadata::FilenameTimestampFormat;
pub use crate::metadata::RepoFactsProbe;
pub use crate::metadata::RepositoryFacts;
pub use crate::record::Outcome;
pub use crate::record::Record;
pub use crate::store::AtomicWrite;
pub use crate::store::FileRemove;
pub use crate::store::RecordStore;
pub use crate::store::StoreError;
pub use crate::typed_ref::parse_typed_ref;
pub use crate::typed_ref::TypedRef;
pub use crate::value::FrontmatterValue;
pub use crate::value::Mapping;
pub use crate::value::Scalar;

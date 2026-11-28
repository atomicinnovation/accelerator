use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::ptr;

use crate::cluster::ClusterKeyRule;
use crate::cluster::CompletenessMark;
use crate::doc_type::Facet;
use crate::doc_type::LibraryPlacement;
use crate::doc_type::Listing;
use crate::linkage::LinkageIdRule;
use crate::slug::SlugRule;

/// A meta document type, declared as data: its identity on each surface and
/// the facts and rules every consumer reads from it.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct DocType {
    pub wire_token: &'static str,
    pub config_path_key: Option<&'static str>,
    /// The typed-linkage vocabulary name, as used in `<type>:<id>`
    /// references. `None` for a virtual type.
    pub linkage_type_name: Option<&'static str>,
    pub label: &'static str,
    pub in_lifecycle: bool,
    pub carries_target_frontmatter: bool,
    pub participates_in_lifecycle: bool,
    pub in_kanban: bool,
    pub is_virtual: bool,
    pub slug_rule: SlugRule,
    pub cluster_key_rule: ClusterKeyRule,
    /// Where the type's documents sort among a lifecycle cluster's members;
    /// `None` sorts after every ranked type.
    pub lifecycle_rank: Option<u8>,
    pub completeness_mark: Option<CompletenessMark>,
    pub linkage_id_rule: LinkageIdRule,
    /// Whether the type's documents give the cluster they join its slug and
    /// key, and lend their slug to the documents that share it.
    pub cluster_anchor: bool,
    pub library_placement: Option<LibraryPlacement>,
    pub facets: &'static [Facet],
    pub listing: Listing,
}

/// A handle to a declared [`DocType`], identified by its wire token.
#[derive(Clone, Copy)]
pub struct DocTypeKey(&'static DocType);

impl DocTypeKey {
    #[must_use]
    pub const fn declared(doc_type: &'static DocType) -> Self {
        Self(doc_type)
    }

    #[must_use]
    pub const fn wire_token(self) -> &'static str {
        self.0.wire_token
    }

    #[must_use]
    pub const fn config_path_key(self) -> Option<&'static str> {
        self.0.config_path_key
    }

    #[must_use]
    pub const fn linkage_type_name(self) -> Option<&'static str> {
        self.0.linkage_type_name
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        self.0.label
    }

    #[must_use]
    pub const fn in_lifecycle(self) -> bool {
        self.0.in_lifecycle
    }

    #[must_use]
    pub const fn carries_target_frontmatter(self) -> bool {
        self.0.carries_target_frontmatter
    }

    #[must_use]
    pub const fn participates_in_lifecycle(self) -> bool {
        self.0.participates_in_lifecycle
    }

    #[must_use]
    pub const fn in_kanban(self) -> bool {
        self.0.in_kanban
    }

    #[must_use]
    pub const fn is_virtual(self) -> bool {
        self.0.is_virtual
    }

    #[must_use]
    pub const fn slug_rule(self) -> SlugRule {
        self.0.slug_rule
    }

    #[must_use]
    pub const fn cluster_key_rule(self) -> ClusterKeyRule {
        self.0.cluster_key_rule
    }

    #[must_use]
    pub const fn lifecycle_rank(self) -> Option<u8> {
        self.0.lifecycle_rank
    }

    #[must_use]
    pub const fn completeness_mark(self) -> Option<CompletenessMark> {
        self.0.completeness_mark
    }

    #[must_use]
    pub const fn linkage_id_rule(self) -> LinkageIdRule {
        self.0.linkage_id_rule
    }

    #[must_use]
    pub const fn cluster_anchor(self) -> bool {
        self.0.cluster_anchor
    }

    #[must_use]
    pub const fn library_placement(self) -> Option<LibraryPlacement> {
        self.0.library_placement
    }

    #[must_use]
    pub const fn facets(self) -> &'static [Facet] {
        self.0.facets
    }

    #[must_use]
    pub const fn listing(self) -> Listing {
        self.0.listing
    }

    #[must_use]
    pub const fn is_set(self) -> bool {
        matches!(self.0.listing, Listing::Set)
    }
}

impl PartialEq for DocTypeKey {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.0, other.0) || self.wire_token() == other.wire_token()
    }
}

impl Eq for DocTypeKey {}

impl Hash for DocTypeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.wire_token().hash(state);
    }
}

impl PartialOrd for DocTypeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DocTypeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wire_token().cmp(other.wire_token())
    }
}

impl fmt::Debug for DocTypeKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_token())
    }
}

#[cfg(feature = "test-support")]
impl DocType {
    pub const FLAT_DEFAULTS: Self = Self {
        wire_token: "",
        config_path_key: None,
        linkage_type_name: None,
        label: "",
        in_lifecycle: false,
        carries_target_frontmatter: false,
        participates_in_lifecycle: false,
        in_kanban: false,
        is_virtual: false,
        slug_rule: SlugRule::VerbatimStem,
        cluster_key_rule: ClusterKeyRule::Unclustered,
        lifecycle_rank: None,
        completeness_mark: None,
        linkage_id_rule: LinkageIdRule::Stem,
        cluster_anchor: false,
        library_placement: None,
        facets: &[],
        listing: Listing::Flat {
            drafts_subdirectory: None,
            filename_filter: crate::doc_type::FilenameFilter::Markdown,
        },
    };
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{DocType, DocTypeKey};

    static ALPHA: DocType = DocType {
        wire_token: "alpha",
        label: "Alpha",
        ..DocType::FLAT_DEFAULTS
    };

    static ALPHA_AGAIN: DocType = DocType {
        wire_token: "alpha",
        label: "Alpha again",
        ..DocType::FLAT_DEFAULTS
    };

    static BETA: DocType = DocType {
        wire_token: "beta",
        label: "Beta",
        ..DocType::FLAT_DEFAULTS
    };

    #[test]
    fn keys_to_declarations_with_one_wire_token_are_equal() {
        assert_eq!(
            DocTypeKey::declared(&ALPHA),
            DocTypeKey::declared(&ALPHA_AGAIN)
        );
        assert_ne!(DocTypeKey::declared(&ALPHA), DocTypeKey::declared(&BETA));
    }

    #[test]
    fn keys_hash_and_order_by_wire_token() {
        let keys: HashSet<DocTypeKey> = [
            DocTypeKey::declared(&ALPHA),
            DocTypeKey::declared(&ALPHA_AGAIN),
            DocTypeKey::declared(&BETA),
        ]
        .into_iter()
        .collect();
        assert_eq!(keys.len(), 2);
        assert!(DocTypeKey::declared(&ALPHA) < DocTypeKey::declared(&BETA));
    }

    #[test]
    fn a_key_reads_its_declared_facts() {
        let beta = DocTypeKey::declared(&BETA);
        assert_eq!(beta.wire_token(), "beta");
        assert_eq!(beta.label(), "Beta");
        assert_eq!(format!("{beta:?}"), "beta");
    }
}

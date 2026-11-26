//! The document-type fact: declared doc types with their rules, the
//! catalogue that holds them, and the pure path-inference matcher.

mod built_in;
mod catalogue;
mod declaration;
mod infer;
mod library;
mod listing;

pub use crate::doc_type::catalogue::CatalogueError;
pub use crate::doc_type::catalogue::DocTypeCatalogue;
pub use crate::doc_type::catalogue::Identity;
pub use crate::doc_type::declaration::DocType;
pub use crate::doc_type::declaration::DocTypeKey;
pub use crate::doc_type::infer::infer;
pub use crate::doc_type::library::Facet;
pub use crate::doc_type::library::LibraryPhase;
pub use crate::doc_type::library::LibraryPlacement;
pub use crate::doc_type::listing::FilenameFilter;
pub use crate::doc_type::listing::Listing;

#[cfg(test)]
pub(crate) use crate::doc_type::declaration::UNREMARKABLE;

#[cfg(test)]
mod tests {
    use super::{DocTypeCatalogue, DocTypeKey};

    #[test]
    fn topic_research_mirrors_the_design_inventories_predicate_profile() {
        // Both are nested-manifest Discover types; the only intended divergence
        // is nested_root_filename. Pin the full boolean profile so the
        // shared shape is asserted rather than inferred.
        let topic = DocTypeKey::TOPIC_RESEARCH;
        let inventory = DocTypeKey::DESIGN_INVENTORIES;
        assert_eq!(topic.in_lifecycle(), inventory.in_lifecycle());
        assert_eq!(
            topic.carries_target_frontmatter(),
            inventory.carries_target_frontmatter()
        );
        assert_eq!(
            topic.participates_in_lifecycle(),
            inventory.participates_in_lifecycle()
        );
        assert_eq!(topic.in_kanban(), inventory.in_kanban());
        assert_eq!(topic.is_virtual(), inventory.is_virtual());

        assert_eq!(topic.nested_root_filename(), Some("manifest.md"));
    }

    #[test]
    fn wire_token_round_trips_for_every_built_in_type() {
        let catalogue = DocTypeCatalogue::built_in();
        for kind in catalogue.doc_types() {
            assert_eq!(
                catalogue.from_wire_token(kind.wire_token()),
                Some(*kind)
            );
        }
        assert_eq!(catalogue.from_wire_token("bogus"), None);
    }

    #[test]
    fn templates_is_virtual_and_out_of_lifecycle() {
        assert!(DocTypeKey::TEMPLATES.is_virtual());
        assert_eq!(DocTypeKey::TEMPLATES.config_path_key(), None);
        assert!(!DocTypeKey::TEMPLATES.in_lifecycle());
    }

    #[test]
    fn pr_descriptions_carries_three_genuinely_different_names() {
        // The one type whose config key, wire form, and linkage type name all
        // differ. Collapsing any two into one "name" would silently break the
        // other surface, so each is pinned.
        let prs = DocTypeKey::PR_DESCRIPTIONS;
        assert_eq!(prs.config_path_key(), Some("prs"));
        assert_eq!(prs.wire_token(), "pr-descriptions");
        assert_eq!(prs.linkage_type_name(), Some("pr-description"));
    }

    #[test]
    fn root_cause_analyses_is_an_out_of_lifecycle_peer() {
        let rca = DocTypeKey::ROOT_CAUSE_ANALYSES;
        assert_eq!(rca.config_path_key(), Some("research_issues"));
        assert!(!rca.in_lifecycle());
        assert!(!rca.participates_in_lifecycle());
        assert!(!rca.is_virtual());
    }

    #[test]
    fn work_items_is_the_only_kanban_type() {
        for kind in DocTypeCatalogue::built_in().doc_types() {
            assert_eq!(kind.in_kanban(), *kind == DocTypeKey::WORK_ITEMS);
        }
    }

    #[test]
    fn carries_target_frontmatter_covers_review_and_validation_types() {
        let review_and_validation_types = [
            DocTypeKey::PLAN_REVIEWS,
            DocTypeKey::WORK_ITEM_REVIEWS,
            DocTypeKey::PR_REVIEWS,
            DocTypeKey::VALIDATIONS,
        ];
        for kind in DocTypeCatalogue::built_in().doc_types() {
            assert_eq!(
                kind.carries_target_frontmatter(),
                review_and_validation_types.contains(kind)
            );
        }
    }
}

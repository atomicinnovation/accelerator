//! Every context's declarations, assembled into the registrations the
//! platform crates need.

use std::sync::Arc;

use corpus::doc_type::built_in;
use corpus::doc_type::CatalogueError;
use corpus::doc_type::DeclaredSet;
use corpus::DocTypeCatalogue;
use corpus::DocTypeKey;

/// Each set type the corpus declares, paired with the set declaration its
/// owner makes.
#[must_use]
pub fn set_declarations() -> &'static [DeclaredSet] {
    static SET_DECLARATIONS: [DeclaredSet; 2] = [
        (
            DocTypeKey::TOPIC_RESEARCH,
            &research_declarations::topic::SET,
        ),
        (
            DocTypeKey::DESIGN_INVENTORIES,
            &design_declarations::inventory::SET,
        ),
    ];
    &SET_DECLARATIONS
}

/// The complete doc type catalogue every composition root serves.
///
/// # Errors
///
/// A [`CatalogueError`] when the declarations do not form a catalogue.
pub fn catalogue() -> Result<Arc<DocTypeCatalogue>, CatalogueError> {
    DocTypeCatalogue::new(built_in::doc_types(), set_declarations())
        .map(Arc::new)
}

#[cfg(test)]
mod tests {
    use corpus::doc_type::built_in;
    use corpus::set::Presentation;
    use corpus::DocTypeCatalogue;

    use crate::catalogue;

    fn set_of(
        catalogue: &DocTypeCatalogue,
        wire_token: &str,
    ) -> Option<(&'static str, Presentation)> {
        let matcher =
            catalogue.set_matcher(catalogue.from_wire_token(wire_token)?)?;
        Some((matcher.root(), matcher.presentation()))
    }

    #[test]
    fn the_catalogue_holds_the_built_in_doc_types_in_order() {
        assert_eq!(
            catalogue().map(|catalogue| catalogue.doc_types().to_vec()),
            Ok(built_in::doc_types().to_vec())
        );
    }

    #[test]
    fn the_catalogue_pairs_each_set_type_with_its_owners_set_declaration() {
        let catalogue = catalogue();
        assert_eq!(
            catalogue
                .as_deref()
                .ok()
                .and_then(|catalogue| set_of(catalogue, "topic-research")),
            Some(("manifest.md", Presentation::SetPage))
        );
        assert_eq!(
            catalogue
                .as_deref()
                .ok()
                .and_then(|catalogue| set_of(catalogue, "design-inventories")),
            Some(("inventory.md", Presentation::EntryPoint))
        );
    }
}

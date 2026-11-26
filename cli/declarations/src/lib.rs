//! Every context's declarations, assembled into the registrations the
//! platform crates need.

use std::sync::Arc;

use corpus::doc_type::CatalogueError;
use corpus::DocTypeCatalogue;

/// The complete doc type catalogue every composition root serves.
///
/// # Errors
///
/// A [`CatalogueError`] when two declarations claim the same identity.
pub fn catalogue() -> Result<Arc<DocTypeCatalogue>, CatalogueError> {
    let built_in = DocTypeCatalogue::built_in();
    DocTypeCatalogue::new(built_in.doc_types().to_vec()).map(Arc::new)
}

#[cfg(test)]
mod tests {
    use corpus::DocTypeCatalogue;

    use crate::catalogue;

    #[test]
    fn the_catalogue_holds_the_built_in_doc_types_in_order() {
        assert_eq!(
            catalogue().map(|catalogue| catalogue.doc_types().to_vec()),
            Ok(DocTypeCatalogue::built_in().doc_types().to_vec())
        );
    }
}

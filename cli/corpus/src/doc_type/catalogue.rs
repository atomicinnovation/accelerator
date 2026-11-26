use std::collections::HashSet;
use std::fmt;

use crate::doc_type::built_in::IN_CATALOGUE_ORDER;
use crate::doc_type::DocTypeKey;

/// The ordered doc types a composition knows, looked up by each of their
/// identities.
#[derive(Debug, Clone)]
pub struct DocTypeCatalogue {
    doc_types: Vec<DocTypeKey>,
}

/// Which identity two declarations in one catalogue share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identity {
    WireToken,
    LinkageTypeName,
    ConfigPathKey,
}

/// Two declarations in one catalogue claim the same identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueError {
    pub identity: Identity,
    pub value: &'static str,
}

impl fmt::Display for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WireToken => "wire token",
            Self::LinkageTypeName => "linkage type name",
            Self::ConfigPathKey => "config path key",
        })
    }
}

impl fmt::Display for CatalogueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "two doc types are declared with the {} {:?}",
            self.identity, self.value
        )
    }
}

impl std::error::Error for CatalogueError {}

impl DocTypeCatalogue {
    /// # Errors
    ///
    /// Returns a [`CatalogueError`] naming the clash when two declarations
    /// share a wire token, a linkage type name or a config path key.
    pub fn new(doc_types: Vec<DocTypeKey>) -> Result<Self, CatalogueError> {
        refuse_shared(&doc_types, Identity::WireToken, |kind| {
            Some(kind.wire_token())
        })?;
        refuse_shared(
            &doc_types,
            Identity::LinkageTypeName,
            DocTypeKey::linkage_type_name,
        )?;
        refuse_shared(
            &doc_types,
            Identity::ConfigPathKey,
            DocTypeKey::config_path_key,
        )?;
        Ok(Self { doc_types })
    }

    #[must_use]
    pub fn built_in() -> Self {
        Self {
            doc_types: IN_CATALOGUE_ORDER.to_vec(),
        }
    }

    #[must_use]
    pub fn doc_types(&self) -> &[DocTypeKey] {
        &self.doc_types
    }

    #[must_use]
    pub fn from_wire_token(&self, token: &str) -> Option<DocTypeKey> {
        self.find(|kind| kind.wire_token() == token)
    }

    #[must_use]
    pub fn from_linkage_type_name(&self, name: &str) -> Option<DocTypeKey> {
        self.find(|kind| kind.linkage_type_name() == Some(name))
    }

    #[must_use]
    pub fn from_config_path_key(&self, key: &str) -> Option<DocTypeKey> {
        self.find(|kind| kind.config_path_key() == Some(key))
    }

    fn find(&self, matches: impl Fn(DocTypeKey) -> bool) -> Option<DocTypeKey> {
        self.doc_types.iter().copied().find(|kind| matches(*kind))
    }
}

fn refuse_shared(
    doc_types: &[DocTypeKey],
    identity: Identity,
    identity_of: impl Fn(DocTypeKey) -> Option<&'static str>,
) -> Result<(), CatalogueError> {
    let mut seen = HashSet::new();
    for value in doc_types.iter().filter_map(|kind| identity_of(*kind)) {
        if !seen.insert(value) {
            return Err(CatalogueError { identity, value });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CatalogueError, DocTypeCatalogue, Identity};
    use crate::doc_type::{DocType, DocTypeKey};

    const UNREMARKABLE: DocType = DocType {
        wire_token: "",
        config_path_key: None,
        linkage_type_name: None,
        label: "",
        in_lifecycle: false,
        carries_target_frontmatter: false,
        participates_in_lifecycle: false,
        in_kanban: false,
        is_virtual: false,
        nested_root_filename: None,
    };

    static GADGETS: DocType = DocType {
        wire_token: "gadgets",
        config_path_key: Some("gadgets"),
        linkage_type_name: Some("gadget"),
        ..UNREMARKABLE
    };

    static GADGETS_AGAIN: DocType = DocType {
        wire_token: "gadgets",
        ..UNREMARKABLE
    };

    static WIDGETS_LINKED_AS_GADGET: DocType = DocType {
        wire_token: "widgets",
        linkage_type_name: Some("gadget"),
        ..UNREMARKABLE
    };

    static WIDGETS_CONFIGURED_AS_GADGETS: DocType = DocType {
        wire_token: "widgets",
        config_path_key: Some("gadgets"),
        ..UNREMARKABLE
    };

    static UNLINKED_WIDGETS: DocType = DocType {
        wire_token: "widgets",
        ..UNREMARKABLE
    };

    fn catalogue_of(
        declarations: &[&'static DocType],
    ) -> Result<DocTypeCatalogue, CatalogueError> {
        DocTypeCatalogue::new(
            declarations
                .iter()
                .map(|declaration| DocTypeKey::declared(declaration))
                .collect(),
        )
    }

    #[test]
    fn refuses_two_declarations_sharing_a_wire_token() {
        assert_eq!(
            catalogue_of(&[&GADGETS, &GADGETS_AGAIN]).err(),
            Some(CatalogueError {
                identity: Identity::WireToken,
                value: "gadgets",
            })
        );
    }

    #[test]
    fn refuses_two_declarations_sharing_a_linkage_type_name() {
        assert_eq!(
            catalogue_of(&[&GADGETS, &WIDGETS_LINKED_AS_GADGET]).err(),
            Some(CatalogueError {
                identity: Identity::LinkageTypeName,
                value: "gadget",
            })
        );
    }

    #[test]
    fn refuses_two_declarations_sharing_a_config_path_key() {
        assert_eq!(
            catalogue_of(&[&GADGETS, &WIDGETS_CONFIGURED_AS_GADGETS]).err(),
            Some(CatalogueError {
                identity: Identity::ConfigPathKey,
                value: "gadgets",
            })
        );
    }

    #[test]
    fn declarations_without_a_linkage_name_or_path_key_do_not_clash(
    ) -> Result<(), CatalogueError> {
        let catalogue = catalogue_of(&[&UNLINKED_WIDGETS, &GADGETS_AGAIN])?;
        assert_eq!(catalogue.doc_types().len(), 2);
        Ok(())
    }

    #[test]
    fn looks_a_declaration_up_by_each_identity() -> Result<(), CatalogueError> {
        let catalogue = catalogue_of(&[&GADGETS, &UNLINKED_WIDGETS])?;
        let gadgets = DocTypeKey::declared(&GADGETS);
        assert_eq!(catalogue.from_wire_token("gadgets"), Some(gadgets));
        assert_eq!(catalogue.from_linkage_type_name("gadget"), Some(gadgets));
        assert_eq!(catalogue.from_config_path_key("gadgets"), Some(gadgets));
        assert_eq!(catalogue.from_wire_token("bogus"), None);
        Ok(())
    }

    #[test]
    fn the_built_in_declarations_form_a_valid_catalogue(
    ) -> Result<(), CatalogueError> {
        let built_in = DocTypeCatalogue::built_in();
        let validated = DocTypeCatalogue::new(built_in.doc_types().to_vec())?;
        assert_eq!(validated.doc_types(), built_in.doc_types());
        assert_eq!(validated.doc_types().len(), 15);
        Ok(())
    }
}

use std::collections::HashSet;
use std::fmt;

use crate::doc_type::DocTypeKey;
use crate::set::SetDeclaration;
use crate::set::SetMatcher;
use crate::set::SlotPatternError;

/// A set type paired with the set declaration its owner makes for it.
pub type DeclaredSet = (DocTypeKey, &'static SetDeclaration);

/// The ordered doc types a composition knows, looked up by each of their
/// identities, with the set declaration of each set type.
#[derive(Debug, Clone, Default)]
pub struct DocTypeCatalogue {
    doc_types: Vec<DocTypeKey>,
    set_matchers: Vec<(DocTypeKey, SetMatcher)>,
}

/// Which identity two declarations in one catalogue share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identity {
    WireToken,
    LinkageTypeName,
    ConfigPathKey,
}

/// Why a set of declarations does not form a catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogueError {
    SharedIdentity {
        identity: Identity,
        value: &'static str,
    },
    MissingSetDeclaration {
        kind: DocTypeKey,
    },
    NotASet {
        kind: DocTypeKey,
    },
    SetDeclaredTwice {
        kind: DocTypeKey,
    },
    UnparseableSlot {
        kind: DocTypeKey,
        pattern: &'static str,
        cause: SlotPatternError,
    },
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
        match self {
            Self::SharedIdentity { identity, value } => write!(
                formatter,
                "two doc types are declared with the {identity} {value:?}"
            ),
            Self::MissingSetDeclaration { kind } => write!(
                formatter,
                "the set type {kind:?} has no set declaration"
            ),
            Self::NotASet { kind } => write!(
                formatter,
                "a set declaration is given for {kind:?}, which the \
                 catalogue does not declare as a set type"
            ),
            Self::SetDeclaredTwice { kind } => write!(
                formatter,
                "the set type {kind:?} is given two set declarations"
            ),
            Self::UnparseableSlot {
                kind,
                pattern,
                cause,
            } => write!(
                formatter,
                "the set type {kind:?} declares the slot {pattern:?}, \
                 which does not parse: {cause}"
            ),
        }
    }
}

impl std::error::Error for CatalogueError {}

impl DocTypeCatalogue {
    /// # Errors
    ///
    /// A [`CatalogueError`] when two declarations share a wire token, a
    /// linkage type name or a config path key, when a set type has no set
    /// declaration or two, when a set declaration is given for a type that
    /// is not a set type in the catalogue, or when a slot does not parse.
    pub fn new(
        doc_types: &[DocTypeKey],
        set_declarations: &[DeclaredSet],
    ) -> Result<Self, CatalogueError> {
        refuse_shared(doc_types, Identity::WireToken, |kind| {
            Some(kind.wire_token())
        })?;
        refuse_shared(
            doc_types,
            Identity::LinkageTypeName,
            DocTypeKey::linkage_type_name,
        )?;
        refuse_shared(
            doc_types,
            Identity::ConfigPathKey,
            DocTypeKey::config_path_key,
        )?;
        Ok(Self {
            doc_types: doc_types.to_vec(),
            set_matchers: paired_sets(doc_types, set_declarations)?,
        })
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

    /// The matcher for a set type's paired set declaration; `None` for a
    /// type that is not a set.
    #[must_use]
    pub fn set_matcher(&self, kind: DocTypeKey) -> Option<&SetMatcher> {
        self.set_matchers
            .iter()
            .find(|(set_type, _)| *set_type == kind)
            .map(|(_, matcher)| matcher)
    }

    fn find(&self, matches: impl Fn(DocTypeKey) -> bool) -> Option<DocTypeKey> {
        self.doc_types.iter().copied().find(|kind| matches(*kind))
    }
}

fn paired_sets(
    doc_types: &[DocTypeKey],
    set_declarations: &[DeclaredSet],
) -> Result<Vec<(DocTypeKey, SetMatcher)>, CatalogueError> {
    let mut paired: Vec<(DocTypeKey, SetMatcher)> = Vec::new();
    for &(kind, declaration) in set_declarations {
        if !(kind.is_set() && doc_types.contains(&kind)) {
            return Err(CatalogueError::NotASet { kind });
        }
        if paired.iter().any(|(set_type, _)| *set_type == kind) {
            return Err(CatalogueError::SetDeclaredTwice { kind });
        }
        let matcher =
            SetMatcher::compiled(declaration).map_err(|(pattern, cause)| {
                CatalogueError::UnparseableSlot {
                    kind,
                    pattern,
                    cause,
                }
            })?;
        paired.push((kind, matcher));
    }
    if let Some(&kind) = doc_types.iter().find(|kind| {
        kind.is_set() && !paired.iter().any(|(set_type, _)| set_type == *kind)
    }) {
        return Err(CatalogueError::MissingSetDeclaration { kind });
    }
    Ok(paired)
}

fn refuse_shared(
    doc_types: &[DocTypeKey],
    identity: Identity,
    identity_of: impl Fn(DocTypeKey) -> Option<&'static str>,
) -> Result<(), CatalogueError> {
    let mut seen = HashSet::new();
    for value in doc_types.iter().filter_map(|kind| identity_of(*kind)) {
        if !seen.insert(value) {
            return Err(CatalogueError::SharedIdentity { identity, value });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CatalogueError, DeclaredSet, DocTypeCatalogue, Identity};
    use crate::doc_type::{DocType, DocTypeKey, Listing};
    use crate::set::{
        MemberKind, MemberSlot, Presentation, SetDeclaration, SetMatcher,
        SetShape, SlotPatternError,
    };

    static GADGETS: DocType = DocType {
        wire_token: "gadgets",
        config_path_key: Some("gadgets"),
        linkage_type_name: Some("gadget"),
        ..DocType::FLAT_DEFAULTS
    };

    static GADGETS_AGAIN: DocType = DocType {
        wire_token: "gadgets",
        ..DocType::FLAT_DEFAULTS
    };

    static WIDGETS_LINKED_AS_GADGET: DocType = DocType {
        wire_token: "widgets",
        linkage_type_name: Some("gadget"),
        ..DocType::FLAT_DEFAULTS
    };

    static WIDGETS_CONFIGURED_AS_GADGETS: DocType = DocType {
        wire_token: "widgets",
        config_path_key: Some("gadgets"),
        ..DocType::FLAT_DEFAULTS
    };

    static UNLINKED_WIDGETS: DocType = DocType {
        wire_token: "widgets",
        ..DocType::FLAT_DEFAULTS
    };

    static ALBUMS: DocType = DocType {
        wire_token: "albums",
        listing: Listing::Set,
        ..DocType::FLAT_DEFAULTS
    };

    static DOSSIERS: DocType = DocType {
        wire_token: "dossiers",
        listing: Listing::Set,
        ..DocType::FLAT_DEFAULTS
    };

    static ALBUM: SetDeclaration = SetDeclaration {
        shape: SetShape {
            root: "album.md",
            member_slots: &[MemberSlot {
                pattern: "photos/*.jpg",
                kind: MemberKind::Asset,
            }],
        },
        presentation: Presentation::EntryPoint,
    };

    static DOSSIER: SetDeclaration = SetDeclaration {
        shape: SetShape {
            root: "dossier.md",
            member_slots: &[MemberSlot {
                pattern: "papers/*.md",
                kind: MemberKind::Document,
            }],
        },
        presentation: Presentation::SetPage,
    };

    static SPRAWLING_DOSSIER: SetDeclaration = SetDeclaration {
        shape: SetShape {
            root: "dossier.md",
            member_slots: &[MemberSlot {
                pattern: "papers/**/*.md",
                kind: MemberKind::Document,
            }],
        },
        presentation: Presentation::SetPage,
    };

    const fn key(declaration: &'static DocType) -> DocTypeKey {
        DocTypeKey::declared(declaration)
    }

    fn catalogue_of(
        declarations: &[&'static DocType],
    ) -> Result<DocTypeCatalogue, CatalogueError> {
        let doc_types: Vec<DocTypeKey> = declarations
            .iter()
            .map(|declaration| key(declaration))
            .collect();
        DocTypeCatalogue::new(&doc_types, &[])
    }

    fn sets_catalogue(
        set_declarations: &[DeclaredSet],
    ) -> Result<DocTypeCatalogue, CatalogueError> {
        DocTypeCatalogue::new(
            &[key(&GADGETS), key(&ALBUMS), key(&DOSSIERS)],
            set_declarations,
        )
    }

    #[test]
    fn the_default_catalogue_declares_no_doc_type_and_no_set() {
        let catalogue = DocTypeCatalogue::default();
        assert!(catalogue.doc_types().is_empty());
        assert!(catalogue.set_matcher(key(&ALBUMS)).is_none());
    }

    #[test]
    fn refuses_two_declarations_sharing_a_wire_token() {
        assert_eq!(
            catalogue_of(&[&GADGETS, &GADGETS_AGAIN]).err(),
            Some(CatalogueError::SharedIdentity {
                identity: Identity::WireToken,
                value: "gadgets",
            })
        );
    }

    #[test]
    fn refuses_two_declarations_sharing_a_linkage_type_name() {
        assert_eq!(
            catalogue_of(&[&GADGETS, &WIDGETS_LINKED_AS_GADGET]).err(),
            Some(CatalogueError::SharedIdentity {
                identity: Identity::LinkageTypeName,
                value: "gadget",
            })
        );
    }

    #[test]
    fn refuses_two_declarations_sharing_a_config_path_key() {
        assert_eq!(
            catalogue_of(&[&GADGETS, &WIDGETS_CONFIGURED_AS_GADGETS]).err(),
            Some(CatalogueError::SharedIdentity {
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
        let gadgets = key(&GADGETS);
        assert_eq!(catalogue.from_wire_token("gadgets"), Some(gadgets));
        assert_eq!(catalogue.from_linkage_type_name("gadget"), Some(gadgets));
        assert_eq!(catalogue.from_config_path_key("gadgets"), Some(gadgets));
        assert_eq!(catalogue.from_wire_token("bogus"), None);
        Ok(())
    }

    #[test]
    fn each_set_type_answers_with_its_paired_set_declaration(
    ) -> Result<(), CatalogueError> {
        let catalogue = sets_catalogue(&[
            (key(&ALBUMS), &ALBUM),
            (key(&DOSSIERS), &DOSSIER),
        ])?;
        let albums = catalogue.set_matcher(key(&ALBUMS));
        assert_eq!(albums.map(SetMatcher::root), Some("album.md"));
        assert_eq!(
            albums.map(SetMatcher::presentation),
            Some(Presentation::EntryPoint)
        );
        let dossiers = catalogue.set_matcher(key(&DOSSIERS));
        assert_eq!(dossiers.map(|set| set.shape().root), Some("dossier.md"));
        assert_eq!(
            dossiers.map(SetMatcher::presentation),
            Some(Presentation::SetPage)
        );
        assert!(catalogue.set_matcher(key(&GADGETS)).is_none());
        Ok(())
    }

    #[test]
    fn refuses_a_set_type_with_no_set_declaration() {
        assert_eq!(
            sets_catalogue(&[(key(&ALBUMS), &ALBUM)]).err(),
            Some(CatalogueError::MissingSetDeclaration {
                kind: key(&DOSSIERS)
            })
        );
    }

    #[test]
    fn refuses_a_set_declaration_for_a_type_that_is_not_a_set() {
        assert_eq!(
            sets_catalogue(&[
                (key(&ALBUMS), &ALBUM),
                (key(&DOSSIERS), &DOSSIER),
                (key(&GADGETS), &DOSSIER),
            ])
            .err(),
            Some(CatalogueError::NotASet {
                kind: key(&GADGETS)
            })
        );
    }

    #[test]
    fn refuses_a_set_declaration_for_a_type_outside_the_catalogue() {
        assert_eq!(
            DocTypeCatalogue::new(&[key(&GADGETS)], &[(key(&ALBUMS), &ALBUM)])
                .err(),
            Some(CatalogueError::NotASet { kind: key(&ALBUMS) })
        );
    }

    #[test]
    fn refuses_a_set_type_given_two_set_declarations() {
        assert_eq!(
            sets_catalogue(&[
                (key(&ALBUMS), &ALBUM),
                (key(&DOSSIERS), &DOSSIER),
                (key(&ALBUMS), &DOSSIER),
            ])
            .err(),
            Some(CatalogueError::SetDeclaredTwice { kind: key(&ALBUMS) })
        );
    }

    #[test]
    fn refuses_a_set_declaration_whose_slot_does_not_parse() {
        let refusal = sets_catalogue(&[
            (key(&ALBUMS), &ALBUM),
            (key(&DOSSIERS), &SPRAWLING_DOSSIER),
        ])
        .err();
        assert_eq!(
            refusal,
            Some(CatalogueError::UnparseableSlot {
                kind: key(&DOSSIERS),
                pattern: "papers/**/*.md",
                cause: SlotPatternError::RecursiveWildcard,
            })
        );
        assert_eq!(
            refusal.map(|refusal| refusal.to_string()),
            Some(
                "the set type dossiers declares the slot \"papers/**/*.md\", \
                 which does not parse: a slot's wildcard spans one segment"
                    .to_owned()
            )
        );
    }
}

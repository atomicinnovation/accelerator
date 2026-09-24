//! `catalogue.json`: its document shape, and the resolvers read out of it.
//!
//! A reader loads the file once and forgives what it cannot parse: an absent
//! file resolves nothing, and a damaged part answers as damaged while the rest
//! keeps resolving. Names match trimmed and Unicode case-insensitively.

mod document;
mod entries;
mod section;
mod team_states;

use std::path::Path;

pub use self::document::{
    CatalogueDocument, CatalogueParseError, CatalogueUpdate, CataloguedLabel,
    CataloguedMember, CataloguedProject, CataloguedState, Strictness,
    TeamEntry,
};
pub use self::entries::{FamilyResolver, TeamEntries};
pub use self::section::{CatalogueSection, SectionSet};
pub use self::team_states::TeamStates;

use crate::resolution::ResolverSet;

/// The catalogue under `<integrations_root>/linear/`, read forgivingly: a
/// damaged entry is skipped, and an absent or unparseable file is `None`.
#[must_use]
pub fn read_catalogue(integrations_root: &Path) -> Option<CatalogueDocument> {
    let text =
        std::fs::read_to_string(catalogue_path(integrations_root)).ok()?;
    CatalogueDocument::parse(&text, Strictness::Forgiving).ok()
}

fn catalogue_path(integrations_root: &Path) -> std::path::PathBuf {
    integrations_root.join("linear/catalogue.json")
}

/// Team data a run fetched live rather than read from the catalogue.
#[derive(Debug, Clone, Default)]
pub struct LiveCatalogueData {
    pub entries: Vec<TeamEntry>,
    pub workspace_labels: Option<Vec<CataloguedLabel>>,
}

/// A catalogue read once, for resolving names against.
#[derive(Debug, Clone)]
pub struct Catalogue {
    document: CatalogueDocument,
}

impl Catalogue {
    #[must_use]
    pub fn load(integrations_root: &Path) -> Self {
        match std::fs::read_to_string(catalogue_path(integrations_root)) {
            Ok(text) => Self::from_text(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Self::from_document(CatalogueDocument::default())
            }
            Err(_) => Self::from_document(CatalogueDocument::unreadable()),
        }
    }

    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self::from_document(
            CatalogueDocument::parse(text, Strictness::Forgiving)
                .unwrap_or_else(|_| CatalogueDocument::unreadable()),
        )
    }

    #[must_use]
    pub const fn from_document(document: CatalogueDocument) -> Self {
        Self { document }
    }

    #[must_use]
    pub fn resolver_set(&self) -> ResolverSet {
        ResolverSet::new(
            Box::new(TeamStates::of(&self.document)),
            TeamEntries::from_document(self.document.clone()),
        )
    }

    /// Every catalogued team as `(key, id)`, in id order.
    #[must_use]
    pub fn catalogued_teams(&self) -> Vec<(String, String)> {
        entries::catalogued_teams(&self.document)
    }

    #[must_use]
    pub fn base_team_id(&self) -> Option<String> {
        self.document.base_entry().map(|base| base.id.clone())
    }
}

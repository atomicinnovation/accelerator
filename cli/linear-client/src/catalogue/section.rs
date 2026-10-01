//! The parts of a catalogue a fetch can fill: four per-team sections, plus
//! the workspace labels no team owns.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CatalogueSection {
    States,
    Labels,
    Members,
    Projects,
    WorkspaceLabels,
}

impl CatalogueSection {
    pub const ALL: [Self; 5] = [
        Self::States,
        Self::Labels,
        Self::Members,
        Self::Projects,
        Self::WorkspaceLabels,
    ];

    pub const PER_TEAM: [Self; 4] =
        [Self::States, Self::Labels, Self::Members, Self::Projects];

    const fn belongs_to_a_team(self) -> bool {
        !matches!(self, Self::WorkspaceLabels)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SectionSet(BTreeSet<CatalogueSection>);

impl SectionSet {
    #[must_use]
    pub fn of(sections: &[CatalogueSection]) -> Self {
        Self(sections.iter().copied().collect())
    }

    #[must_use]
    pub fn all() -> Self {
        Self::of(&CatalogueSection::ALL)
    }

    #[must_use]
    pub fn contains(&self, section: CatalogueSection) -> bool {
        self.0.contains(&section)
    }

    pub fn team_sections(&self) -> impl Iterator<Item = CatalogueSection> + '_ {
        self.0
            .iter()
            .copied()
            .filter(|section| section.belongs_to_a_team())
    }
}

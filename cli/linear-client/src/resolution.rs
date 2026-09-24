//! Resolving the names a pull filter or a transition is configured with to
//! the ids Linear filters on.
//!
//! Every filter family resolves over a given set of teams, so a value means
//! the same thing whichever teams a scope spans. Transition keeps its own,
//! narrower resolver over the base team's states, which can yield one id at
//! most.

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::catalogue::CatalogueSection;
use crate::catalogue::FamilyResolver;
use crate::catalogue::LiveCatalogueData;
use crate::catalogue::SectionSet;
use crate::catalogue::TeamEntries;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FilterFamily {
    State,
    Project,
    Label,
    Assignee,
}

impl FilterFamily {
    pub const ALL: [Self; 4] =
        [Self::State, Self::Project, Self::Label, Self::Assignee];

    /// The catalogue sections a value of this family resolves against.
    #[must_use]
    pub fn sections(self) -> SectionSet {
        match self {
            Self::State => SectionSet::of(&[CatalogueSection::States]),
            Self::Project => SectionSet::of(&[CatalogueSection::Projects]),
            Self::Label => SectionSet::of(&[
                CatalogueSection::Labels,
                CatalogueSection::WorkspaceLabels,
            ]),
            Self::Assignee => SectionSet::of(&[CatalogueSection::Members]),
        }
    }

    pub(crate) const fn team_section(self) -> CatalogueSection {
        match self {
            Self::State => CatalogueSection::States,
            Self::Project => CatalogueSection::Projects,
            Self::Label => CatalogueSection::Labels,
            Self::Assignee => CatalogueSection::Members,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmpty<T> {
    pub const fn new(first: T, rest: Vec<T>) -> Self {
        Self { first, rest }
    }

    #[must_use]
    pub fn from_vec(items: Vec<T>) -> Option<Self> {
        let mut items = items.into_iter();
        let first = items.next()?;
        Some(Self::new(first, items.collect()))
    }

    pub const fn first(&self) -> &T {
        &self.first
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.first).chain(self.rest.iter())
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        1 + self.rest.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
}

/// The ids one filter value resolved to, none of them blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIds(NonEmpty<String>);

impl ResolvedIds {
    #[must_use]
    pub fn new(ids: NonEmpty<String>) -> Option<Self> {
        let none_blank = ids.iter().all(|id| !id.trim().is_empty());
        none_blank.then_some(Self(ids))
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}

/// A team named in a refusal: its id always, and its key when one is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamRef {
    pub id: String,
    pub key: Option<String>,
}

/// Why the catalogue cannot answer for a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogueGap {
    /// No catalogue, or no base team in it.
    Absent,
    /// A part of the catalogue that could not be parsed.
    Damaged,
}

/// Which member field a value matched. Earlier tiers win outright, so a
/// value that is one user's email is never weighed against another's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchTier {
    Email,
    FullName,
    DisplayName,
}

impl MatchTier {
    pub const ALL: [Self; 3] = [Self::Email, Self::FullName, Self::DisplayName];
}

/// One of the records an ambiguous value matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    NotFound,
    NotCatalogued {
        section: CatalogueSection,
        cause: CatalogueGap,
    },
    /// A given team whose entry lacks a section the family needs, so absence
    /// from it proves nothing.
    TeamUnfetched {
        team: TeamRef,
    },
    AmbiguousMember {
        tier: MatchTier,
        candidates: Vec<Candidate>,
    },
    /// `team` is `None` for a group no one team owns: the workspace labels,
    /// or projects pooled across the given teams.
    AmbiguousRecord {
        team: Option<TeamRef>,
        candidates: Vec<Candidate>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Resolved(ResolvedIds),
    Unresolved(Unresolved),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SingleResolution {
    Resolved(String),
    Unresolved(NameUnresolved),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameUnresolved {
    NotFound,
    NotCatalogued(CatalogueGap),
    Ambiguous { count: usize },
}

/// Resolves a name within one fixed set of records, to one id at most.
pub trait NameResolver {
    fn resolve(&self, value: &str) -> SingleResolution;
}

/// Resolves a filter value over the records of the given teams.
pub trait TeamScopedResolver {
    fn resolve(&self, value: &str, teams: &[TeamRef]) -> Resolution;
}

/// Exact names mapped to ids, for callers and suites with no catalogue.
#[derive(Debug, Clone, Default)]
pub struct FixedNames(pub BTreeMap<String, String>);

impl NameResolver for FixedNames {
    fn resolve(&self, value: &str) -> SingleResolution {
        self.0.get(value).map_or(
            SingleResolution::Unresolved(NameUnresolved::NotFound),
            |id| SingleResolution::Resolved(id.clone()),
        )
    }
}

/// Everything a client resolves names through: the base team's states for
/// transition, and the team entries every filter family and every question
/// of team identity is answered from.
pub struct ResolverSet {
    team_states: Rc<dyn NameResolver>,
    entries: TeamEntries,
}

impl ResolverSet {
    #[must_use]
    pub fn new(
        team_states: Box<dyn NameResolver>,
        entries: TeamEntries,
    ) -> Self {
        Self {
            team_states: Rc::from(team_states),
            entries,
        }
    }

    #[must_use]
    pub fn team_states(&self) -> &dyn NameResolver {
        self.team_states.as_ref()
    }

    #[must_use]
    pub const fn entries(&self) -> &TeamEntries {
        &self.entries
    }

    #[must_use]
    pub const fn for_family(&self, family: FilterFamily) -> FamilyResolver<'_> {
        self.entries.for_family(family)
    }

    #[must_use]
    pub fn team_by_key(&self, key: &str) -> Option<TeamRef> {
        self.entries.team_by_key(key)
    }

    /// The team `team_id` names, with its catalogued key when it has one.
    #[must_use]
    pub fn team_by_id(&self, team_id: &str) -> TeamRef {
        self.entries.team_by_id(team_id)
    }

    #[must_use]
    pub fn base_team(&self) -> Option<TeamRef> {
        self.entries.base_team()
    }

    /// Whether the catalogue holds the workspace labels.
    ///
    /// # Errors
    ///
    /// [`CatalogueGap::Damaged`] when they could not be parsed.
    pub fn has_workspace_labels(&self) -> Result<bool, CatalogueGap> {
        self.entries.has_workspace_labels()
    }

    /// Every catalogued team as `(key, id)`, in id order.
    #[must_use]
    pub fn catalogued_teams(&self) -> Vec<(String, String)> {
        self.entries.catalogued_teams()
    }

    /// # Errors
    ///
    /// [`CatalogueGap::Damaged`] when the teams, or one of the requested
    /// sections of this team, could not be parsed.
    pub fn covers(
        &self,
        team_id: &str,
        sections: &SectionSet,
    ) -> Result<bool, CatalogueGap> {
        self.entries.covers(team_id, sections)
    }

    /// # Errors
    ///
    /// As [`Self::covers`].
    pub fn is_complete(&self, team_id: &str) -> Result<bool, CatalogueGap> {
        self.entries.is_complete(team_id)
    }

    /// This set with the data a run fetched folded in: each fetched section
    /// replaces the stored one, so every team resolves through one path.
    #[must_use]
    pub fn with_fetched(&self, fetched: &LiveCatalogueData) -> Self {
        Self {
            team_states: Rc::clone(&self.team_states),
            entries: self.entries.with_fetched(fetched),
        }
    }
}

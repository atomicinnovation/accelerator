//! The catalogue's team entries as the one owner of team identity, section
//! coverage and per-family resolution, so no two answers can disagree.

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::OnceLock;

use super::document::CatalogueDocument;
use super::document::CatalogueUpdate;
use super::document::TeamEntry;
use super::section::CatalogueSection;
use super::section::SectionSet;
use super::LiveCatalogueData;
use crate::resolution::Candidate;
use crate::resolution::CatalogueGap;
use crate::resolution::FilterFamily;
use crate::resolution::MatchTier;
use crate::resolution::NonEmpty;
use crate::resolution::Resolution;
use crate::resolution::ResolvedIds;
use crate::resolution::TeamRef;
use crate::resolution::TeamScopedResolver;
use crate::resolution::Unresolved;

pub struct TeamEntries {
    document: CatalogueDocument,
    fetched: BTreeSet<(String, CatalogueSection)>,
    indices: Indices,
}

#[derive(Default)]
struct Indices {
    states: OnceLock<RecordIndex>,
    labels: OnceLock<RecordIndex>,
    projects: OnceLock<RecordIndex>,
    members: OnceLock<[RecordIndex; 3]>,
}

type RecordIndex = HashMap<String, Vec<IndexedRecord>>;

/// A record under its matchable name. `team` is `None` for a workspace
/// label, which no team owns.
#[derive(Debug, Clone)]
struct IndexedRecord {
    team: Option<String>,
    id: String,
    name: String,
    active: bool,
    fetched: bool,
}

impl TeamEntries {
    #[must_use]
    pub fn from_document(document: CatalogueDocument) -> Self {
        Self {
            document,
            fetched: BTreeSet::new(),
            indices: Indices::default(),
        }
    }

    /// Entries known only by `(key, id)`, none of their sections fetched.
    #[must_use]
    pub fn keyed(teams: &[(&str, &str)]) -> Self {
        let mut document = CatalogueDocument::default();
        document.record(CatalogueUpdate {
            entries: teams
                .iter()
                .map(|(key, id)| TeamEntry::identified(id, key, key))
                .collect(),
            ..CatalogueUpdate::default()
        });
        Self::from_document(document)
    }

    #[must_use]
    pub fn team_by_key(&self, key: &str) -> Option<TeamRef> {
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        self.document
            .teams()
            .iter()
            .find(|entry| entry.key.trim() == key)
            .map(team_ref)
    }

    #[must_use]
    pub fn team_by_id(&self, team_id: &str) -> TeamRef {
        self.entry(team_id).map_or_else(
            || TeamRef {
                id: team_id.to_owned(),
                key: None,
            },
            team_ref,
        )
    }

    #[must_use]
    pub fn base_team(&self) -> Option<TeamRef> {
        self.document.base_entry().map(team_ref)
    }

    /// # Errors
    ///
    /// [`CatalogueGap::Damaged`] when the workspace labels could not be
    /// parsed.
    pub fn has_workspace_labels(&self) -> Result<bool, CatalogueGap> {
        if self.document.damage().workspace_labels_damaged() {
            return Err(CatalogueGap::Damaged);
        }
        Ok(self.document.workspace_labels().is_some())
    }

    #[must_use]
    pub fn catalogued_teams(&self) -> Vec<(String, String)> {
        catalogued_teams(&self.document)
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
        let damage = self.document.damage();
        if damage.teams_unreadable()
            || sections
                .team_sections()
                .any(|section| damage.section_damaged(team_id, section))
            || (sections.contains(CatalogueSection::WorkspaceLabels)
                && damage.workspace_labels_damaged())
        {
            return Err(CatalogueGap::Damaged);
        }
        let Some(entry) = self.entry(team_id) else {
            return Ok(false);
        };
        let workspace_labels_covered = !sections
            .contains(CatalogueSection::WorkspaceLabels)
            || self.document.workspace_labels().is_some();
        Ok(workspace_labels_covered
            && sections
                .team_sections()
                .all(|section| carries(entry, section)))
    }

    /// # Errors
    ///
    /// As [`Self::covers`].
    pub fn is_complete(&self, team_id: &str) -> Result<bool, CatalogueGap> {
        self.covers(team_id, &SectionSet::of(&CatalogueSection::PER_TEAM))
    }

    #[must_use]
    pub const fn for_family(&self, family: FilterFamily) -> FamilyResolver<'_> {
        FamilyResolver {
            entries: self,
            family,
        }
    }

    #[must_use]
    pub fn with_fetched(&self, fetched: &LiveCatalogueData) -> Self {
        let mut document = self.document.clone();
        let mut provenance = self.fetched.clone();
        for entry in &fetched.entries {
            for section in CatalogueSection::PER_TEAM {
                if carries(entry, section) {
                    document.damage_mut().repair(&entry.id, section);
                    provenance.insert((entry.id.clone(), section));
                }
            }
        }
        if fetched.workspace_labels.is_some() {
            document.damage_mut().repair_workspace_labels();
        }
        document.record(CatalogueUpdate {
            base_team: None,
            entries: fetched.entries.clone(),
            workspace_labels: fetched.workspace_labels.clone(),
        });
        Self {
            document,
            fetched: provenance,
            indices: Indices::default(),
        }
    }

    fn entry(&self, team_id: &str) -> Option<&TeamEntry> {
        self.document
            .teams()
            .iter()
            .find(|entry| entry.id == team_id)
    }

    fn resolve(
        &self,
        family: FilterFamily,
        value: &str,
        teams: &[TeamRef],
    ) -> Resolution {
        let wanted = normalise(value);
        if wanted.is_empty() {
            return Resolution::Unresolved(Unresolved::NotFound);
        }
        let ids = self.scoped(family, teams).and_then(|scoped| match family {
            FilterFamily::State => {
                per_team(self.states(), &wanted, &scoped, false)
            }
            FilterFamily::Label => {
                per_team(self.labels(), &wanted, &scoped, true)
            }
            FilterFamily::Project => pooled(self.projects(), &wanted, &scoped),
            FilterFamily::Assignee => by_tier(self.members(), &wanted, &scoped),
        });
        match ids.map(|ids| NonEmpty::from_vec(ids).and_then(ResolvedIds::new))
        {
            Ok(Some(ids)) => Resolution::Resolved(ids),
            Ok(None) => Resolution::Unresolved(Unresolved::NotFound),
            Err(unresolved) => Resolution::Unresolved(unresolved),
        }
    }

    /// The given teams, each named by its catalogued key, once every one of
    /// them is known to carry what `family` needs.
    fn scoped(
        &self,
        family: FilterFamily,
        teams: &[TeamRef],
    ) -> Result<Vec<TeamRef>, Unresolved> {
        let gap = |section, cause| Unresolved::NotCatalogued { section, cause };
        let damage = self.document.damage();
        if damage.teams_unreadable() {
            return Err(gap(family.team_section(), CatalogueGap::Damaged));
        }
        if teams.is_empty() {
            return Err(gap(family.team_section(), CatalogueGap::Absent));
        }
        let sections = family.sections();
        if sections.contains(CatalogueSection::WorkspaceLabels) {
            if damage.workspace_labels_damaged() {
                return Err(gap(
                    CatalogueSection::WorkspaceLabels,
                    CatalogueGap::Damaged,
                ));
            }
            if self.document.workspace_labels().is_none() {
                return Err(gap(
                    CatalogueSection::WorkspaceLabels,
                    CatalogueGap::Absent,
                ));
            }
        }
        teams
            .iter()
            .map(|team| {
                if let Some(section) = sections
                    .team_sections()
                    .find(|section| damage.section_damaged(&team.id, *section))
                {
                    return Err(gap(section, CatalogueGap::Damaged));
                }
                match self.entry(&team.id) {
                    Some(entry)
                        if sections
                            .team_sections()
                            .all(|section| carries(entry, section)) =>
                    {
                        Ok(team_ref(entry))
                    }
                    Some(entry) => Err(Unresolved::TeamUnfetched {
                        team: team_ref(entry),
                    }),
                    None => {
                        Err(Unresolved::TeamUnfetched { team: team.clone() })
                    }
                }
            })
            .collect()
    }

    fn was_fetched(&self, team_id: &str, section: CatalogueSection) -> bool {
        self.fetched.contains(&(team_id.to_owned(), section))
    }

    fn states(&self) -> &RecordIndex {
        self.indices.states.get_or_init(|| {
            let mut index = RecordIndex::new();
            for entry in self.document.teams() {
                let fetched =
                    self.was_fetched(&entry.id, CatalogueSection::States);
                for state in entry.states.iter().flatten() {
                    insert(
                        &mut index,
                        &state.name,
                        IndexedRecord {
                            team: Some(entry.id.clone()),
                            id: state.id.clone(),
                            name: state.name.clone(),
                            active: state.archived_at.is_none(),
                            fetched,
                        },
                    );
                }
            }
            index
        })
    }

    fn labels(&self) -> &RecordIndex {
        self.indices.labels.get_or_init(|| {
            let mut index = RecordIndex::new();
            for entry in self.document.teams() {
                let fetched =
                    self.was_fetched(&entry.id, CatalogueSection::Labels);
                for label in entry.labels.iter().flatten() {
                    insert(
                        &mut index,
                        &label.name,
                        IndexedRecord {
                            team: Some(entry.id.clone()),
                            id: label.id.clone(),
                            name: label.name.clone(),
                            active: label.archived_at.is_none(),
                            fetched,
                        },
                    );
                }
            }
            for label in self.document.workspace_labels().into_iter().flatten()
            {
                insert(
                    &mut index,
                    &label.name,
                    IndexedRecord {
                        team: None,
                        id: label.id.clone(),
                        name: label.name.clone(),
                        active: label.archived_at.is_none(),
                        fetched: false,
                    },
                );
            }
            index
        })
    }

    fn projects(&self) -> &RecordIndex {
        self.indices.projects.get_or_init(|| {
            let mut index = RecordIndex::new();
            for entry in self.document.teams() {
                let fetched =
                    self.was_fetched(&entry.id, CatalogueSection::Projects);
                for project in entry.projects.iter().flatten() {
                    insert(
                        &mut index,
                        &project.name,
                        IndexedRecord {
                            team: Some(entry.id.clone()),
                            id: project.id.clone(),
                            name: project.name.clone(),
                            active: project.archived_at.is_none(),
                            fetched,
                        },
                    );
                }
            }
            index
        })
    }

    /// One index per [`MatchTier`], in tier order.
    fn members(&self) -> &[RecordIndex; 3] {
        self.indices.members.get_or_init(|| {
            let mut tiers: [RecordIndex; 3] = Default::default();
            for entry in self.document.teams() {
                let fetched =
                    self.was_fetched(&entry.id, CatalogueSection::Members);
                for member in entry.members.iter().flatten() {
                    let record = IndexedRecord {
                        team: Some(entry.id.clone()),
                        id: member.id.clone(),
                        name: describe_member(&member.name, &member.email),
                        active: member.active,
                        fetched,
                    };
                    let [email, full_name, display_name] = &mut tiers;
                    insert(email, &member.email, record.clone());
                    insert(full_name, &member.name, record.clone());
                    insert(display_name, &member.display_name, record);
                }
            }
            tiers
        })
    }
}

/// A [`TeamEntries`] seen through one filter family.
pub struct FamilyResolver<'a> {
    entries: &'a TeamEntries,
    family: FilterFamily,
}

impl TeamScopedResolver for FamilyResolver<'_> {
    fn resolve(&self, value: &str, teams: &[TeamRef]) -> Resolution {
        self.entries.resolve(self.family, value, teams)
    }
}

pub(super) fn catalogued_teams(
    document: &CatalogueDocument,
) -> Vec<(String, String)> {
    document
        .teams()
        .iter()
        .filter(|entry| !entry.key.trim().is_empty())
        .map(|entry| (entry.key.clone(), entry.id.clone()))
        .collect()
}

fn team_ref(entry: &TeamEntry) -> TeamRef {
    TeamRef {
        id: entry.id.clone(),
        key: Some(entry.key.clone()).filter(|key| !key.trim().is_empty()),
    }
}

const fn carries(entry: &TeamEntry, section: CatalogueSection) -> bool {
    match section {
        CatalogueSection::States => entry.states.is_some(),
        CatalogueSection::Labels => entry.labels.is_some(),
        CatalogueSection::Members => entry.members.is_some(),
        CatalogueSection::Projects => entry.projects.is_some(),
        CatalogueSection::WorkspaceLabels => false,
    }
}

pub(super) fn normalise(value: &str) -> String {
    value.trim().to_lowercase()
}

fn insert(index: &mut RecordIndex, key: &str, record: IndexedRecord) {
    let key = normalise(key);
    if key.is_empty() || record.id.trim().is_empty() {
        return;
    }
    index.entry(key).or_default().push(record);
}

fn describe_member(name: &str, email: &str) -> String {
    if email.trim().is_empty() {
        name.to_owned()
    } else {
        format!("{name} <{email}>")
    }
}

fn matches<'a>(index: &'a RecordIndex, wanted: &str) -> &'a [IndexedRecord] {
    index.get(wanted).map_or(&[], Vec::as_slice)
}

/// Each given team's active-wins match, then the workspace labels' when
/// `with_workspace` is set, collected across the groups.
fn per_team(
    index: &RecordIndex,
    wanted: &str,
    scoped: &[TeamRef],
    with_workspace: bool,
) -> Result<Vec<String>, Unresolved> {
    let found = matches(index, wanted);
    let mut ids = Vec::new();
    for team in scoped {
        let group: Vec<&IndexedRecord> = found
            .iter()
            .filter(|record| record.team.as_deref() == Some(team.id.as_str()))
            .collect();
        let winner = active_wins(&group).map_err(|candidates| {
            Unresolved::AmbiguousRecord {
                team: Some(team.clone()),
                candidates,
            }
        })?;
        ids.extend(winner);
    }
    if with_workspace {
        let group: Vec<&IndexedRecord> = found
            .iter()
            .filter(|record| record.team.is_none())
            .collect();
        let winner = active_wins(&group).map_err(|candidates| {
            Unresolved::AmbiguousRecord {
                team: None,
                candidates,
            }
        })?;
        ids.extend(winner);
    }
    let mut seen = BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    Ok(ids)
}

/// The given teams' records pooled into one group, deduplicated by id.
fn pooled(
    index: &RecordIndex,
    wanted: &str,
    scoped: &[TeamRef],
) -> Result<Vec<String>, Unresolved> {
    let group = scoped_records(matches(index, wanted), scoped);
    active_wins(&group)
        .map(|winner| winner.into_iter().collect())
        .map_err(|candidates| Unresolved::AmbiguousRecord {
            team: None,
            candidates,
        })
}

/// The first tier with any match decides, through active-wins.
fn by_tier(
    tiers: &[RecordIndex; 3],
    wanted: &str,
    scoped: &[TeamRef],
) -> Result<Vec<String>, Unresolved> {
    for (tier, index) in MatchTier::ALL.into_iter().zip(tiers) {
        let group = scoped_records(matches(index, wanted), scoped);
        if group.is_empty() {
            continue;
        }
        return active_wins(&group)
            .map(|winner| winner.into_iter().collect())
            .map_err(|candidates| Unresolved::AmbiguousMember {
                tier,
                candidates,
            });
    }
    Ok(Vec::new())
}

/// The records of the given teams, one per id. A record the run fetched
/// replaces a stored one, since it is the fresher account of that record.
fn scoped_records<'a>(
    found: &'a [IndexedRecord],
    scoped: &[TeamRef],
) -> Vec<&'a IndexedRecord> {
    let mut group: Vec<&IndexedRecord> = Vec::new();
    for record in found.iter().filter(|record| {
        scoped
            .iter()
            .any(|team| record.team.as_deref() == Some(team.id.as_str()))
    }) {
        match group.iter_mut().find(|kept| kept.id == record.id) {
            Some(kept) if record.fetched && !kept.fetched => *kept = record,
            Some(_) => {}
            None => group.push(record),
        }
    }
    group
}

/// One active match wins over any number of inactive ones; with none
/// active, exactly one inactive match resolves. Anything else is ambiguous.
fn active_wins(
    group: &[&IndexedRecord],
) -> Result<Option<String>, Vec<Candidate>> {
    let active: Vec<&IndexedRecord> = group
        .iter()
        .copied()
        .filter(|record| record.active)
        .collect();
    let pool = if active.is_empty() { group } else { &active };
    match pool {
        [] => Ok(None),
        [winner] => Ok(Some(winner.id.clone())),
        many => Err(many
            .iter()
            .map(|record| Candidate {
                id: record.id.clone(),
                name: record.name.clone(),
            })
            .collect()),
    }
}

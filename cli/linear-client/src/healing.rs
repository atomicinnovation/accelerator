//! Keeping `catalogue.json` complete for every synced team.
//!
//! A search that fetches team sections live hands them to a
//! [`CatalogueBackfill`], so an apply-mode run can record them for the teams
//! it syncs instead of fetching them again on every later pull. A synced team
//! is the base team, or a team that owns a tracked work item; no other team
//! is ever written, since the committed file is repo-wide.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;

use tracker::ExternalId;

use crate::cache::CacheError;
use crate::cache::LinearCache;
use crate::catalogue::CatalogueDocument;
use crate::catalogue::CatalogueSection;
use crate::catalogue::CatalogueUpdate;
use crate::catalogue::CataloguedLabel;
use crate::catalogue::LiveCatalogueData;
use crate::catalogue::SectionSet;
use crate::catalogue::TeamEntry;
use crate::discovery::TeamEntryFetch;

/// How many of a prefix's identifiers one heal tries before leaving the
/// prefix for the next.
const LOOKUPS_PER_PREFIX: usize = 3;

/// Receives the team data a search fetched live.
pub trait CatalogueBackfill {
    fn hold(&self, live: LiveCatalogueData);
}

/// Discards what it is handed, for a client whose fetches are never recorded.
pub struct NoBackfill;

impl CatalogueBackfill for NoBackfill {
    fn hold(&self, _live: LiveCatalogueData) {}
}

/// The candidate teams a corpus's Linear identifiers name, each prefix with
/// its identifiers in number order.
///
/// A prefix is only a candidate: an identifier's prefix is its team's key at
/// the time it was assigned, and a foreign tracker's id can look the same, so
/// healing confirms each one against Linear before it writes anything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncedTeams {
    prefixes: BTreeMap<String, Vec<String>>,
}

impl SyncedTeams {
    #[must_use]
    pub fn derive<'a>(ids: impl IntoIterator<Item = &'a ExternalId>) -> Self {
        let mut prefixes: BTreeMap<String, Vec<(u64, String)>> =
            BTreeMap::new();
        for id in ids {
            if let Some((prefix, number)) = team_prefix(id.as_str()) {
                prefixes
                    .entry(prefix.to_owned())
                    .or_default()
                    .push((number, id.as_str().to_owned()));
            }
        }
        Self {
            prefixes: prefixes
                .into_iter()
                .map(|(prefix, mut numbered)| {
                    numbered.sort();
                    numbered.dedup();
                    (prefix, numbered.into_iter().map(|(_, id)| id).collect())
                })
                .collect(),
        }
    }

    pub fn prefixes(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.prefixes
            .iter()
            .map(|(prefix, ids)| (prefix.as_str(), ids.as_slice()))
    }
}

fn team_prefix(identifier: &str) -> Option<(&str, u64)> {
    let (prefix, number) = identifier.split_once('-')?;
    let valid_prefix =
        !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_alphanumeric());
    let digits =
        !number.is_empty() && number.chars().all(|c| c.is_ascii_digit());
    if !(valid_prefix && digits) {
        return None;
    }
    number.parse().ok().map(|number| (prefix, number))
}

/// Why no team-section fetch could be built for a heal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchUnavailable {
    pub reason: String,
}

/// What one heal did, each team named by key and each prefix with the
/// identifiers it was tried with.
#[derive(Debug, Default)]
pub struct HealOutcome {
    /// Teams whose entries this heal recorded.
    pub recorded: Vec<String>,
    /// Teams Linear returned no data for.
    pub unreturned: Vec<String>,
    /// Prefixes none of whose identifiers Linear found.
    pub unconfirmed: Vec<(String, Vec<String>)>,
    /// Prefixes and teams a failed lookup or fetch left unhealed.
    pub fetch_failed: Vec<String>,
    pub fetch_failure: Option<String>,
    pub fetch_unavailable: Option<String>,
    pub write_failure: Option<CacheError>,
}

/// The run's buffer of live-fetched team data, and the policy that records it
/// for synced teams.
#[derive(Default)]
pub struct CatalogueHealing {
    held: Arc<HeldSections>,
}

#[derive(Default)]
struct HeldSections(Mutex<Vec<LiveCatalogueData>>);

impl CatalogueBackfill for HeldSections {
    fn hold(&self, live: LiveCatalogueData) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(live);
    }
}

impl HeldSections {
    /// Everything held, merged as the catalogue would merge it.
    fn take(&self) -> CatalogueDocument {
        let held = std::mem::take(
            &mut *self.0.lock().unwrap_or_else(PoisonError::into_inner),
        );
        let mut merged = CatalogueDocument::default();
        for live in held {
            merged.record(CatalogueUpdate {
                base_team: None,
                entries: live.entries,
                workspace_labels: live.workspace_labels,
            });
        }
        merged
    }
}

type FetchFactory<'a> =
    dyn Fn() -> Result<Box<dyn TeamEntryFetch>, FetchUnavailable> + 'a;

impl CatalogueHealing {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The buffer a client holds its live fetches in.
    #[must_use]
    pub fn backfill(&self) -> Arc<dyn CatalogueBackfill> {
        Arc::clone(&self.held) as Arc<dyn CatalogueBackfill>
    }

    /// Completes the catalogue entry of every synced team that needs it: the
    /// base team, each confirmed team of `synced` the catalogue does not
    /// name, and each catalogued entry missing a section. Held sections are
    /// reused and only the rest fetched; `fetch` is built only when a lookup
    /// or a fetch is needed. An entry is recorded only once complete, and
    /// nothing is written when nothing changes.
    pub fn heal(
        &self,
        synced: &SyncedTeams,
        fetch: &FetchFactory<'_>,
        cache: &LinearCache<'_>,
    ) -> HealOutcome {
        let mut outcome = HealOutcome::default();
        let stored = match cache.load_for_update() {
            Ok(stored) => stored,
            Err(error) => {
                outcome.write_failure = Some(error);
                return outcome;
            }
        };
        let mut heal = Heal {
            stored: &stored,
            held: self.held.take(),
            fetch,
            built: None,
            outcome,
        };
        heal.run(synced, cache);
        heal.outcome
    }
}

/// One heal in progress.
struct Heal<'a> {
    stored: &'a CatalogueDocument,
    held: CatalogueDocument,
    fetch: &'a FetchFactory<'a>,
    built: Option<Result<Box<dyn TeamEntryFetch>, FetchUnavailable>>,
    outcome: HealOutcome,
}

impl Heal<'_> {
    fn run(&mut self, synced: &SyncedTeams, cache: &LinearCache<'_>) {
        let unnamed: Vec<(&str, &[String])> = synced
            .prefixes()
            .filter(|(prefix, _)| {
                !self.stored.teams().iter().any(|team| team.key == *prefix)
            })
            .collect();
        let incomplete: Vec<TeamEntry> = self
            .stored
            .teams()
            .iter()
            .filter(|team| missing_sections(team).next().is_some())
            .map(identity)
            .collect();
        let lacks_workspace_labels = self.stored.workspace_labels().is_none();
        if unnamed.is_empty()
            && incomplete.is_empty()
            && !lacks_workspace_labels
        {
            return;
        }

        let mut to_record: BTreeMap<String, TeamEntry> = incomplete
            .into_iter()
            .map(|team| (team.id.clone(), team))
            .collect();
        for (prefix, identifiers) in unnamed {
            self.confirm(prefix, identifiers, &mut to_record);
        }

        let fetched = self.fetch_missing(&to_record, lacks_workspace_labels);
        self.record(to_record, &fetched, cache);
    }

    fn fetcher(&mut self) -> Result<&dyn TeamEntryFetch, FetchUnavailable> {
        let factory = self.fetch;
        let built = self.built.get_or_insert_with(factory);
        match &*built {
            Ok(fetch) => Ok(&**fetch),
            Err(unavailable) => Err(unavailable.clone()),
        }
    }

    /// Looks the prefix's identifiers up until one belongs to a team keyed by
    /// the prefix, adding every team found to `to_record` unless its entry is
    /// already complete.
    fn confirm(
        &mut self,
        prefix: &str,
        identifiers: &[String],
        to_record: &mut BTreeMap<String, TeamEntry>,
    ) {
        let mut tried = Vec::new();
        let mut found_any = false;
        for identifier in identifiers.iter().take(LOOKUPS_PER_PREFIX) {
            tried.push(identifier.clone());
            let lookup = match self.fetcher() {
                Ok(fetch) => fetch.team_of_identifier(identifier),
                Err(unavailable) => {
                    self.outcome.fetch_unavailable = Some(unavailable.reason);
                    self.outcome.fetch_failed.push(prefix.to_owned());
                    return;
                }
            };
            match lookup {
                Ok(Some(team)) => {
                    found_any = true;
                    let keyed_by_prefix = team.key == prefix;
                    if !self.is_complete(&team.id) {
                        to_record.entry(team.id.clone()).or_insert(team);
                    }
                    if keyed_by_prefix {
                        return;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    self.outcome.fetch_failure = Some(error.to_string());
                    self.outcome.fetch_failed.push(prefix.to_owned());
                    return;
                }
            }
        }
        if !found_any {
            self.outcome.unconfirmed.push((prefix.to_owned(), tried));
        }
    }

    fn is_complete(&self, team_id: &str) -> bool {
        self.stored
            .teams()
            .iter()
            .find(|team| team.id == team_id)
            .is_some_and(|team| missing_sections(team).next().is_none())
    }

    /// The stored and held sections of `team`, merged, with the freshest
    /// identity winning.
    fn known(&self, team: &TeamEntry) -> TeamEntry {
        let mut known = CatalogueDocument::default();
        let stored = self.stored.teams().iter().find(|t| t.id == team.id);
        let held = self.held.teams().iter().find(|t| t.id == team.id);
        known.record(CatalogueUpdate {
            base_team: None,
            entries: [stored.cloned(), Some(identity(team)), held.cloned()]
                .into_iter()
                .flatten()
                .collect(),
            workspace_labels: None,
        });
        known
            .teams()
            .first()
            .cloned()
            .unwrap_or_else(|| identity(team))
    }

    /// Fetches, in one call, every section no stored or held record covers
    /// for the teams to record, plus the workspace labels when neither the
    /// catalogue nor the buffer has them.
    fn fetch_missing(
        &mut self,
        to_record: &BTreeMap<String, TeamEntry>,
        lacks_workspace_labels: bool,
    ) -> Fetched {
        let mut team_ids = Vec::new();
        let mut sections = BTreeSet::new();
        for team in to_record.values() {
            let missing: Vec<CatalogueSection> =
                missing_sections(&self.known(team)).collect();
            if !missing.is_empty() {
                team_ids.push(team.id.clone());
                sections.extend(missing);
            }
        }
        let fetch_labels =
            lacks_workspace_labels && self.held.workspace_labels().is_none();
        if fetch_labels {
            sections.insert(CatalogueSection::WorkspaceLabels);
        }
        if sections.is_empty() {
            return Fetched::default();
        }
        let names: Vec<String> =
            team_ids.iter().map(|id| key_of(to_record, id)).collect();
        let sections: Vec<CatalogueSection> = sections.into_iter().collect();
        let result = match self.fetcher() {
            Ok(fetch) => {
                fetch.fetch_team_entries(&team_ids, &SectionSet::of(&sections))
            }
            Err(unavailable) => {
                self.outcome.fetch_unavailable = Some(unavailable.reason);
                self.outcome.fetch_failed.extend(names);
                return Fetched::default();
            }
        };
        match result {
            Ok(fetched) => {
                self.outcome.unreturned.extend(
                    fetched.unreturned.iter().map(|id| key_of(to_record, id)),
                );
                Fetched {
                    entries: fetched.entries,
                    workspace_labels: fetched.workspace_labels,
                    unreturned: fetched.unreturned,
                }
            }
            Err(error) => {
                self.outcome.fetch_failure = Some(error.to_string());
                self.outcome.fetch_failed.extend(names);
                Fetched::default()
            }
        }
    }

    fn record(
        &mut self,
        to_record: BTreeMap<String, TeamEntry>,
        fetched: &Fetched,
        cache: &LinearCache<'_>,
    ) {
        let mut entries = Vec::new();
        for (id, team) in to_record {
            if fetched.unreturned.contains(&id) {
                continue;
            }
            let fetched_entry = fetched.entries.iter().find(|e| e.id == id);
            let mut combined = CatalogueDocument::default();
            combined.record(CatalogueUpdate {
                base_team: None,
                entries: [Some(self.known(&team)), fetched_entry.cloned()]
                    .into_iter()
                    .flatten()
                    .collect(),
                workspace_labels: None,
            });
            if let Some(complete) = combined
                .teams()
                .first()
                .filter(|entry| missing_sections(entry).next().is_none())
            {
                entries.push(complete.clone());
            }
        }
        let workspace_labels: Option<Vec<CataloguedLabel>> =
            self.stored
                .workspace_labels()
                .is_none()
                .then(|| {
                    fetched.workspace_labels.clone().or_else(|| {
                        self.held.workspace_labels().map(<[_]>::to_vec)
                    })
                })
                .flatten();
        if entries.is_empty() && workspace_labels.is_none() {
            return;
        }
        let mut recorded: Vec<String> =
            entries.iter().map(|entry| entry.key.clone()).collect();
        recorded.sort();
        match cache.record_team_entries(&CatalogueUpdate {
            base_team: None,
            entries,
            workspace_labels,
        }) {
            Ok(_) => self.outcome.recorded = recorded,
            Err(error) => self.outcome.write_failure = Some(error),
        }
    }
}

#[derive(Default)]
struct Fetched {
    entries: Vec<TeamEntry>,
    workspace_labels: Option<Vec<CataloguedLabel>>,
    unreturned: Vec<String>,
}

fn identity(team: &TeamEntry) -> TeamEntry {
    TeamEntry::identified(&team.id, &team.key, &team.name)
}

fn key_of(teams: &BTreeMap<String, TeamEntry>, id: &str) -> String {
    teams
        .get(id)
        .map(|team| team.key.clone())
        .filter(|key| !key.is_empty())
        .unwrap_or_else(|| id.to_owned())
}

fn missing_sections(
    team: &TeamEntry,
) -> impl Iterator<Item = CatalogueSection> + '_ {
    CatalogueSection::PER_TEAM.into_iter().filter(
        move |section| match section {
            CatalogueSection::States => team.states.is_none(),
            CatalogueSection::Labels => team.labels.is_none(),
            CatalogueSection::Members => team.members.is_none(),
            CatalogueSection::Projects => team.projects.is_none(),
            CatalogueSection::WorkspaceLabels => false,
        },
    )
}

//! The pull-filter vocabulary and `IssueFilter` composition.
//!
//! A search moves through three stages, each named for what it guarantees:
//! a [`ConfiguredSearch`] holds the names config gave, a [`ValidatedSearch`]
//! the names pre-flight accepted, and a [`LowerableSearch`] the ids every name
//! resolved to over the scoped teams. Only the last can be composed, so no
//! name ever reaches Linear as a filter value.
//!
//! Linear's complexity is scored at 0.1 per property plus 1 per object,
//! multiplied by a connection's page size, with a hard rejection above 10,000
//! points — so an explicit `first:` is **always** passed and never left to the
//! API's default of 50.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::fmt::Formatter;

use serde_json::json;
use serde_json::Map;
use serde_json::Value;

use crate::catalogue::CatalogueSection;
use crate::catalogue::SectionSet;
use crate::resolution::Candidate;
use crate::resolution::CatalogueGap;
use crate::resolution::FilterFamily;
use crate::resolution::MatchTier;
use crate::resolution::NonEmpty;
use crate::resolution::Resolution;
use crate::resolution::ResolvedIds;
use crate::resolution::ResolverSet;
use crate::resolution::TeamRef;
use crate::resolution::TeamScopedResolver as _;
use crate::resolution::Unresolved;

/// The page size a fetch requests. 250 is Linear's bulk ceiling.
pub const FETCH_PAGE_SIZE: u32 = 250;

const VALIDATED_PREFIX: &str = "validated:";
const TEXT: &str = "text";
const MOST_CANDIDATES_LISTED: usize = 5;

/// Every field string a scope filter can carry. Nothing else spells them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKey {
    /// A name as config gives it.
    Named(FilterFamily),
    /// A name pre-flight accepted, carried to `search` through the port's
    /// string pairs.
    ValidatedName(FilterFamily),
    Text,
}

impl FilterKey {
    #[must_use]
    pub fn parse(field: &str) -> Option<Self> {
        if field == TEXT {
            return Some(Self::Text);
        }
        field.strip_prefix(VALIDATED_PREFIX).map_or_else(
            || family_named(field).map(Self::Named),
            |family| family_named(family).map(Self::ValidatedName),
        )
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => TEXT,
            Self::Named(family) => match family {
                FilterFamily::State => "state",
                FilterFamily::Project => "project",
                FilterFamily::Label => "label",
                FilterFamily::Assignee => "assignee",
            },
            Self::ValidatedName(family) => match family {
                FilterFamily::State => "validated:state",
                FilterFamily::Project => "validated:project",
                FilterFamily::Label => "validated:label",
                FilterFamily::Assignee => "validated:assignee",
            },
        }
    }
}

fn family_named(field: &str) -> Option<FilterFamily> {
    FilterFamily::ALL
        .into_iter()
        .find(|family| FilterKey::Named(*family).as_str() == field)
}

/// The teams a search covers and its title text: everything about a search
/// that needs no resolving.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SearchScope {
    team_ids: Vec<String>,
    text: Option<String>,
}

/// The filter values config gave, by family, not yet checked against any
/// catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredSearch {
    scope: SearchScope,
    values: BTreeMap<FilterFamily, NonEmpty<String>>,
}

/// Filter names pre-flight accepted, still names because the teams they must
/// resolve over may not be known until enumeration.
///
/// The port carries filters only as `(String, String)` pairs, so a validated
/// name travels from `resolve_scope` to `search` under its
/// [`FilterKey::ValidatedName`] spelling, which no configured key can take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSearch {
    scope: SearchScope,
    names: BTreeMap<FilterFamily, NonEmpty<String>>,
}

/// Every filter as the ids it resolved to over the scoped teams: the only
/// stage [`compose`] accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerableSearch {
    scope: SearchScope,
    ids: BTreeMap<FilterFamily, ResolvedIds>,
}

impl ConfiguredSearch {
    /// # Errors
    ///
    /// [`UnresolvedFilters`] naming every blank value and unrecognised key.
    pub fn from_pairs(
        team_ids: Vec<String>,
        pairs: &[(String, String)],
    ) -> Result<Self, UnresolvedFilters> {
        let (scope, values) = read_pairs(team_ids, pairs, |key| match key {
            FilterKey::Named(family) => Some(family),
            FilterKey::ValidatedName(_) | FilterKey::Text => None,
        })?;
        Ok(Self { scope, values })
    }

    /// The same values with the scope narrowed to `team_ids`.
    #[must_use]
    pub fn for_teams(mut self, team_ids: Vec<String>) -> Self {
        self.scope.team_ids = team_ids;
        self
    }

    #[must_use]
    pub fn team_ids(&self) -> &[String] {
        &self.scope.team_ids
    }

    #[must_use]
    pub fn has_filters(&self) -> bool {
        !self.values.is_empty()
    }
}

impl ValidatedSearch {
    /// Reads back what [`Self::to_pairs`] wrote.
    ///
    /// # Errors
    ///
    /// [`UnresolvedFilters`] for a named key that has not passed pre-flight,
    /// a blank value, or an unrecognised key.
    pub fn from_pairs(
        team_ids: Vec<String>,
        pairs: &[(String, String)],
    ) -> Result<Self, UnresolvedFilters> {
        let (scope, names) = read_pairs(team_ids, pairs, |key| match key {
            FilterKey::ValidatedName(family) => Some(family),
            FilterKey::Named(_) | FilterKey::Text => None,
        })?;
        Ok(Self { scope, names })
    }

    #[must_use]
    pub fn to_pairs(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = self
            .names
            .iter()
            .flat_map(|(family, names)| {
                names.iter().map(|name| {
                    (
                        FilterKey::ValidatedName(*family).as_str().to_owned(),
                        name.clone(),
                    )
                })
            })
            .collect();
        if let Some(text) = &self.scope.text {
            pairs.push((TEXT.to_owned(), text.clone()));
        }
        pairs
    }

    #[must_use]
    pub fn team_ids(&self) -> &[String] {
        &self.scope.team_ids
    }

    #[must_use]
    pub fn has_filters(&self) -> bool {
        !self.names.is_empty()
    }

    /// The sections every configured family resolves against.
    #[must_use]
    pub fn needed_sections(&self) -> SectionSet {
        needed_sections(self.names.keys().copied())
    }
}

impl LowerableSearch {
    /// A search over `team_ids` with no filters, as the reconcile read pages
    /// each team.
    #[must_use]
    pub const fn for_teams(team_ids: Vec<String>) -> Self {
        Self {
            scope: SearchScope {
                team_ids,
                text: None,
            },
            ids: BTreeMap::new(),
        }
    }
}

fn needed_sections(families: impl Iterator<Item = FilterFamily>) -> SectionSet {
    let sections: Vec<CatalogueSection> = families
        .flat_map(|family| {
            CatalogueSection::ALL
                .into_iter()
                .filter(move |section| family.sections().contains(*section))
        })
        .collect();
    SectionSet::of(&sections)
}

type FamilyValues = BTreeMap<FilterFamily, NonEmpty<String>>;

fn read_pairs(
    team_ids: Vec<String>,
    pairs: &[(String, String)],
    family_of: impl Fn(FilterKey) -> Option<FilterFamily>,
) -> Result<(SearchScope, FamilyValues), UnresolvedFilters> {
    let mut problems = Vec::new();
    let mut text = None;
    let mut values: BTreeMap<FilterFamily, Vec<String>> = BTreeMap::new();
    for (field, value) in pairs {
        if value.trim().is_empty() {
            problems.push(UnresolvedFilter::Blank { key: field.clone() });
            continue;
        }
        match FilterKey::parse(field) {
            Some(FilterKey::Text) => text = Some(value.clone()),
            Some(key) => match family_of(key) {
                Some(family) => {
                    values.entry(family).or_default().push(value.clone());
                }
                None => problems
                    .push(UnresolvedFilter::Unvalidated { key: field.clone() }),
            },
            None => {
                problems
                    .push(UnresolvedFilter::UnknownKey { key: field.clone() });
            }
        }
    }
    if let Some(refusal) = UnresolvedFilters::of(problems, None) {
        return Err(refusal);
    }
    let values = values
        .into_iter()
        .filter_map(|(family, values)| {
            NonEmpty::from_vec(values).map(|values| (family, values))
        })
        .collect();
    Ok((SearchScope { team_ids, text }, values))
}

/// Validates a configured search before any request.
///
/// `base_only_team` is the team a base-only scope's key resolved to. When its
/// entry covers every configured family, each value is resolved over it now,
/// so a bad value refuses before anything is sent; otherwise every value is
/// carried forward for completion.
///
/// # Errors
///
/// [`UnresolvedFilters`] naming every value that resolved to nothing.
pub fn preflight(
    configured: ConfiguredSearch,
    resolvers: &ResolverSet,
    base_only_team: Option<&TeamRef>,
) -> Result<ValidatedSearch, UnresolvedFilters> {
    let validated = ValidatedSearch {
        scope: configured.scope,
        names: configured.values,
    };
    let Some(team) = base_only_team else {
        return Ok(validated);
    };
    if !validated.has_filters()
        || resolvers.covers(&team.id, &validated.needed_sections()) != Ok(true)
    {
        return Ok(validated);
    }
    resolve_names(&validated, std::slice::from_ref(team), resolvers)?;
    Ok(validated)
}

/// Resolves every validated value over `scoped_teams`.
///
/// # Errors
///
/// [`UnresolvedFilters`] naming every value that resolved to nothing in any
/// scoped team — even one whose family's other values resolved — and every
/// value a scoped team could not answer for.
pub fn complete_for_teams(
    validated: ValidatedSearch,
    scoped_teams: &[TeamRef],
    resolvers: &ResolverSet,
) -> Result<LowerableSearch, UnresolvedFilters> {
    let ids = resolve_names(&validated, scoped_teams, resolvers)?;
    Ok(LowerableSearch {
        scope: validated.scope,
        ids,
    })
}

fn resolve_names(
    validated: &ValidatedSearch,
    scoped_teams: &[TeamRef],
    resolvers: &ResolverSet,
) -> Result<BTreeMap<FilterFamily, ResolvedIds>, UnresolvedFilters> {
    let mut problems = Vec::new();
    let mut resolved = BTreeMap::new();
    for (family, names) in &validated.names {
        let mut ids = BTreeSet::new();
        for name in names.iter() {
            match resolvers.for_family(*family).resolve(name, scoped_teams) {
                Resolution::Resolved(found) => {
                    ids.extend(found.iter().map(str::to_owned));
                }
                Resolution::Unresolved(reason) => {
                    problems.push(UnresolvedFilter::Value {
                        family: *family,
                        value: name.clone(),
                        reason,
                    });
                }
            }
        }
        if let Some(ids) = NonEmpty::from_vec(ids.into_iter().collect())
            .and_then(ResolvedIds::new)
        {
            resolved.insert(*family, ids);
        }
    }
    let base_team = resolvers.base_team().map(|team| team.id);
    UnresolvedFilters::of(problems, base_team).map_or(Ok(resolved), Err)
}

/// Lowers a resolved search to Linear's `IssueFilter`.
#[must_use]
pub fn compose(search: &LowerableSearch) -> Value {
    let mut filter = Map::new();
    match search.scope.team_ids.as_slice() {
        [] => {}
        [single] => {
            filter.insert("team".to_owned(), json!({"id": {"eq": single}}));
        }
        many => {
            filter.insert("team".to_owned(), json!({"id": {"in": many}}));
        }
    }
    for (family, ids) in &search.ids {
        filter.insert(
            lowered_field(*family).to_owned(),
            json!({"id": comparator(ids)}),
        );
    }
    if let Some(text) = &search.scope.text {
        filter.insert("title".to_owned(), json!({"containsIgnoreCase": text}));
    }
    Value::Object(filter)
}

const fn lowered_field(family: FilterFamily) -> &'static str {
    match family {
        FilterFamily::State => "state",
        FilterFamily::Project => "project",
        FilterFamily::Label => "labels",
        FilterFamily::Assignee => "assignee",
    }
}

/// One id lowers to `eq`; several lower to `in` (OR'd).
fn comparator(ids: &ResolvedIds) -> Value {
    let ids: Vec<&str> = ids.iter().collect();
    match ids.as_slice() {
        [single] => json!({"eq": single}),
        many => json!({"in": many}),
    }
}

/// One filter pair that could not become part of a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedFilter {
    Value {
        family: FilterFamily,
        value: String,
        reason: Unresolved,
    },
    Blank {
        key: String,
    },
    UnknownKey {
        key: String,
    },
    /// A configured name offered where only a validated one may appear.
    Unvalidated {
        key: String,
    },
}

impl UnresolvedFilter {
    /// Whether the refusal is the catalogue's to fix rather than the value's.
    #[must_use]
    pub const fn is_catalogue_gap(&self) -> bool {
        matches!(
            self,
            Self::Value {
                reason: Unresolved::NotCatalogued { .. }
                    | Unresolved::TeamUnfetched { .. },
                ..
            }
        )
    }

    #[must_use]
    pub const fn family(&self) -> Option<FilterFamily> {
        match self {
            Self::Value { family, .. } => Some(*family),
            Self::Blank { .. }
            | Self::UnknownKey { .. }
            | Self::Unvalidated { .. } => None,
        }
    }
}

/// Every filter pair a search refused, rendered under one header with a line
/// and a remedy for each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedFilters(Box<Refusal>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Refusal {
    entries: NonEmpty<UnresolvedFilter>,
    base_team: Option<String>,
}

impl UnresolvedFilters {
    /// `None` when there is nothing to refuse. `base_team` is the catalogue's
    /// base team id, which the refresh remedy names.
    #[must_use]
    pub fn of(
        entries: Vec<UnresolvedFilter>,
        base_team: Option<String>,
    ) -> Option<Self> {
        NonEmpty::from_vec(entries)
            .map(|entries| Self(Box::new(Refusal { entries, base_team })))
    }

    pub fn iter(&self) -> impl Iterator<Item = &UnresolvedFilter> {
        self.0.entries.iter()
    }
}

impl Display for UnresolvedFilters {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "pull filters could not be resolved:")?;
        for entry in self.iter() {
            write!(formatter, "\n  ")?;
            write_entry(formatter, entry, self.0.base_team.as_deref())?;
        }
        Ok(())
    }
}

fn write_entry(
    formatter: &mut Formatter<'_>,
    entry: &UnresolvedFilter,
    base_team: Option<&str>,
) -> std::fmt::Result {
    match entry {
        UnresolvedFilter::Blank { key } => write!(
            formatter,
            "E_SEARCH_BLANK_FILTER: {key} has a blank value; remove it or \
             name a value"
        ),
        UnresolvedFilter::UnknownKey { key } => write!(
            formatter,
            "E_SEARCH_UNKNOWN_FILTER: {key:?} is not a Linear filter; use \
             one of state, label, assignee, project"
        ),
        UnresolvedFilter::Unvalidated { key } => write!(
            formatter,
            "E_SEARCH_UNRESOLVED_SCOPE: {key:?} reached search without \
             pre-flight; call resolve_scope first"
        ),
        UnresolvedFilter::Value {
            family,
            value,
            reason,
        } => write_value(formatter, *family, value, reason, base_team),
    }
}

fn write_value(
    formatter: &mut Formatter<'_>,
    family: FilterFamily,
    value: &str,
    reason: &Unresolved,
    base_team: Option<&str>,
) -> std::fmt::Result {
    let noun = family_noun(family);
    match reason {
        Unresolved::NotFound => {
            write!(
                formatter,
                "E_SEARCH_UNKNOWN_{}: no team in scope carries {noun} \
                 {value:?}; check the value. If it was added or renamed in \
                 Linear, pull the latest committed catalogue.json or refresh \
                 with `accelerator linear init discover --team-id {}` and \
                 commit",
                code_family(family),
                base_team.unwrap_or("<uuid>")
            )?;
            if family == FilterFamily::Assignee {
                write!(
                    formatter,
                    ". The value must match a member of a team in scope"
                )?;
            }
            Ok(())
        }
        Unresolved::NotCatalogued {
            cause: CatalogueGap::Absent,
            ..
        } => write!(
            formatter,
            "E_SEARCH_NO_TEAM: {noun} {value:?} cannot be resolved because \
             there is no catalogue base team; run /accelerator:init-linear"
        ),
        Unresolved::NotCatalogued {
            section,
            cause: CatalogueGap::Damaged,
        } => write!(
            formatter,
            "E_SEARCH_CATALOGUE_DAMAGED: {noun} {value:?} cannot be resolved \
             because {} in catalogue.json cannot be read; restore the last \
             good version from version control, or resolve the merge conflict",
            section_name(*section)
        ),
        Unresolved::TeamUnfetched { team } => write!(
            formatter,
            "E_SEARCH_TEAM_UNFETCHED: {noun} {value:?} cannot be resolved \
             because team {} was in scope but Linear returned no data for it; \
             check the credential's access",
            describe_team(team)
        ),
        Unresolved::AmbiguousMember { tier, candidates } => write!(
            formatter,
            "E_SEARCH_AMBIGUOUS_ASSIGNEE: assignee {value:?} matches the \
             {} of {} members in scope; use the user's email",
            tier_name(*tier),
            candidates.len()
        ),
        Unresolved::AmbiguousRecord { team, candidates } => match team {
            Some(team) => write!(
                formatter,
                "E_SEARCH_AMBIGUOUS_{}: {noun} {value:?} matches {} active \
                 {noun}s in team {}; rename or archive one in Linear",
                code_family(family),
                candidates.len(),
                describe_team(team)
            ),
            None => write!(
                formatter,
                "E_SEARCH_AMBIGUOUS_{}: {noun} {value:?} matches {} {noun}s \
                 ({}); rename one in Linear",
                code_family(family),
                candidates.len(),
                listed(candidates)
            ),
        },
    }
}

const fn family_noun(family: FilterFamily) -> &'static str {
    match family {
        FilterFamily::State => "state",
        FilterFamily::Project => "project",
        FilterFamily::Label => "label",
        FilterFamily::Assignee => "assignee",
    }
}

const fn code_family(family: FilterFamily) -> &'static str {
    match family {
        FilterFamily::State => "STATE",
        FilterFamily::Project => "PROJECT",
        FilterFamily::Label => "LABEL",
        FilterFamily::Assignee => "ASSIGNEE",
    }
}

const fn section_name(section: CatalogueSection) -> &'static str {
    match section {
        CatalogueSection::States => "a team's states",
        CatalogueSection::Labels => "a team's labels",
        CatalogueSection::Members => "a team's members",
        CatalogueSection::Projects => "a team's projects",
        CatalogueSection::WorkspaceLabels => "the workspace labels",
    }
}

const fn tier_name(tier: MatchTier) -> &'static str {
    match tier {
        MatchTier::Email => "email",
        MatchTier::FullName => "full name",
        MatchTier::DisplayName => "display name",
    }
}

fn describe_team(team: &TeamRef) -> String {
    team.key.clone().unwrap_or_else(|| team.id.clone())
}

fn listed(candidates: &[Candidate]) -> String {
    let mut shown: Vec<String> = candidates
        .iter()
        .take(MOST_CANDIDATES_LISTED)
        .map(|candidate| format!("{} {}", candidate.name, candidate.id))
        .collect();
    if candidates.len() > MOST_CANDIDATES_LISTED {
        shown.push(format!(
            "and {} more",
            candidates.len() - MOST_CANDIDATES_LISTED
        ));
    }
    shown.join(", ")
}

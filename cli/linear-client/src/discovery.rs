//! Init discovery.
//!
//! The queries that populate the on-disk caches, returned as the cache shapes
//! rather than written. Writing, locking and scaffold upkeep are composition
//! concerns and live in [`crate::cache`].
//!
//! Team sections are fetched for many teams at once: one paginated pass per
//! section, each node attributed to its teams. The query shapes and page
//! sizes are the ones measured against a live tenant to stay within Linear's
//! per-query complexity limit.
//!
//! The interactive team selection is not ported — a client must not prompt —
//! so [`LinearClient::list_teams`] exposes the choices for the skill to
//! render.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde_json::json;
use serde_json::Value;
use tracker::Ceiling;

use crate::catalogue::CatalogueSection;
use crate::catalogue::CataloguedLabel;
use crate::catalogue::SectionSet;
use crate::catalogue::TeamEntry;
use crate::client::LinearClient;
use crate::surface::interpret;
use crate::surface::SurfaceError;
use crate::transport::Deadline;

const VIEWER: &str = "query { viewer { id name } }";
const TEAMS: &str = "query($cursor: String) {
    teams(first: 250, after: $cursor) {
      nodes { id name key }
      pageInfo { hasNextPage endCursor }
    }
  }";
const TEAM_IDENTITIES: &str = "query TeamIdentities($ids: [ID!], $after: String) {
    teams(first: 250, after: $after, includeArchived: true, filter: { id: { in: $ids } }) {
      nodes { id key name }
      pageInfo { hasNextPage endCursor }
    }
  }";
const TEAM_STATES: &str = "query TeamStates($ids: [ID!], $after: String) {
    workflowStates(first: 250, after: $after, includeArchived: true, filter: { team: { id: { in: $ids } } }) {
      nodes { id name type position archivedAt team { id } }
      pageInfo { hasNextPage endCursor }
    }
  }";
const TEAM_LABELS: &str = "query TeamLabels($ids: [ID!], $after: String) {
    issueLabels(first: 250, after: $after, includeArchived: true, filter: { team: { id: { in: $ids } } }) {
      nodes { id name archivedAt team { id } }
      pageInfo { hasNextPage endCursor }
    }
  }";
const WORKSPACE_LABELS: &str = "query WorkspaceLabels($after: String) {
    issueLabels(first: 250, after: $after, includeArchived: true, filter: { team: { null: true } }) {
      nodes { id name archivedAt }
      pageInfo { hasNextPage endCursor }
    }
  }";
const TEAM_MEMBERS: &str = "query TeamMembers($after: String) {
    users(first: 100, after: $after, includeDisabled: true) {
      nodes {
        id name displayName email active
        teams(first: 10) { nodes { id } pageInfo { hasNextPage } }
      }
      pageInfo { hasNextPage endCursor }
    }
  }";
const TEAM_PROJECTS: &str = "query TeamProjects($ids: [ID!], $after: String) {
    projects(first: 100, after: $after, includeArchived: true, filter: { accessibleTeams: { some: { id: { in: $ids } } } }) {
      nodes {
        id name archivedAt
        teams(first: 10) { nodes { id } pageInfo { hasNextPage } }
      }
      pageInfo { hasNextPage endCursor }
    }
  }";

const TEAM_OF_IDENTIFIER: &str = "query TeamOfIdentifier($id: String!) {
    issue(id: $id) { team { id key name } }
  }";

const OPERATION: &str = "discover team entries";
const IDENTIFIER_LOOKUP: &str = "look up an issue's team";
const NOT_FOUND: &str = "Entity not found";

/// The pages a section filtered by team is budgeted at 200 teams.
const TEAM_SCALED_PAGE_BUDGET: usize = 20;
/// The pages a section not filtered by team is budgeted per 1,000 records.
const RECORD_SCALED_PAGE_BUDGET: usize = 10;
const CEILING_HEADROOM: usize = 3;

/// One Relay connection a fetch pages through, and what going past its
/// ceiling means for the operator.
pub(crate) struct PagedConnection {
    pub name: &'static str,
    pub ceiling: Ceiling,
    pub remedy: &'static str,
}

const NARROW_THE_SCOPE: &str = "narrow the pull to fewer teams";
const RECORD_BUDGET_OUTGROWN: &str =
    "the workspace has outgrown the catalogue's record budget";

impl PagedConnection {
    const fn team_scaled(name: &'static str) -> Self {
        Self {
            name,
            ceiling: Ceiling::Bounded(
                CEILING_HEADROOM * TEAM_SCALED_PAGE_BUDGET,
            ),
            remedy: NARROW_THE_SCOPE,
        }
    }

    const fn record_scaled(name: &'static str) -> Self {
        Self {
            name,
            ceiling: Ceiling::Bounded(
                CEILING_HEADROOM * RECORD_SCALED_PAGE_BUDGET,
            ),
            remedy: RECORD_BUDGET_OUTGROWN,
        }
    }

    const fn unbounded(name: &'static str) -> Self {
        Self {
            name,
            ceiling: Ceiling::Unlimited,
            remedy: "",
        }
    }
}

/// How a section's nodes name the teams they belong to.
#[derive(Clone, Copy)]
enum Attribution {
    OwningTeam,
    NestedTeams { connection: &'static str },
    NoTeam,
}

struct SectionQuery {
    document: &'static str,
    connection: PagedConnection,
    attribution: Attribution,
}

impl SectionQuery {
    const fn for_section(section: CatalogueSection) -> Self {
        match section {
            CatalogueSection::States => Self {
                document: TEAM_STATES,
                connection: PagedConnection::team_scaled("workflowStates"),
                attribution: Attribution::OwningTeam,
            },
            CatalogueSection::Labels => Self {
                document: TEAM_LABELS,
                connection: PagedConnection::team_scaled("issueLabels"),
                attribution: Attribution::OwningTeam,
            },
            CatalogueSection::WorkspaceLabels => Self {
                document: WORKSPACE_LABELS,
                connection: PagedConnection::team_scaled("issueLabels"),
                attribution: Attribution::NoTeam,
            },
            CatalogueSection::Members => Self {
                document: TEAM_MEMBERS,
                connection: PagedConnection::record_scaled("users"),
                attribution: Attribution::NestedTeams {
                    connection: "users.teams",
                },
            },
            CatalogueSection::Projects => Self {
                document: TEAM_PROJECTS,
                connection: PagedConnection::record_scaled("projects"),
                attribution: Attribution::NestedTeams {
                    connection: "projects.teams",
                },
            },
        }
    }
}

/// What one fetch of team sections found: an entry per requested team the
/// workspace returned, the workspace labels when asked for, and the requested
/// ids it did not return.
#[derive(Debug, Clone, Default)]
pub struct SectionFetch {
    pub entries: Vec<TeamEntry>,
    pub workspace_labels: Option<Vec<CataloguedLabel>>,
    pub unreturned: Vec<String>,
}

/// The single fetch of team sections that init, completion and healing
/// share.
pub trait TeamEntryFetch {
    /// Fetches `sections` for each of `team_ids`, every section of every team
    /// or nothing.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for any failed page, a connection past its ceiling,
    /// an expired deadline, or a returned team with no workflow states.
    fn fetch_team_entries(
        &self,
        team_ids: &[String],
        sections: &SectionSet,
    ) -> Result<SectionFetch, SurfaceError>;

    /// The team the issue `identifier` belongs to now, which may carry a key
    /// other than the identifier's prefix once the team is renamed or the
    /// issue moved. Only its identity is filled.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for any failure other than Linear finding no such
    /// issue, which is `Ok(None)`.
    fn team_of_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<TeamEntry>, SurfaceError>;
}

impl TeamEntryFetch for LinearClient {
    fn fetch_team_entries(
        &self,
        team_ids: &[String],
        sections: &SectionSet,
    ) -> Result<SectionFetch, SurfaceError> {
        let deadline = self.transport().deadline();
        let mut requested: Vec<String> = team_ids.to_vec();
        requested.sort();
        requested.dedup();

        let mut entries: BTreeMap<String, TeamEntry> = BTreeMap::new();
        if !requested.is_empty() {
            for node in self.page_connection(
                TEAM_IDENTITIES,
                &PagedConnection::team_scaled("teams"),
                Some(&requested),
                &deadline,
            )? {
                let entry: TeamEntry = record_from(node)?;
                if requested.binary_search(&entry.id).is_ok() {
                    entries.insert(entry.id.clone(), entry);
                }
            }
        }
        let unreturned = requested
            .iter()
            .filter(|id| !entries.contains_key(*id))
            .cloned()
            .collect();
        let present: Vec<String> = entries.keys().cloned().collect();

        if !present.is_empty() {
            for section in sections.team_sections() {
                self.fill_section(section, &present, &mut entries, &deadline)?;
            }
        }
        if sections.contains(CatalogueSection::States) {
            refuse_teams_without_states(entries.values())?;
        }

        let workspace_labels = if sections
            .contains(CatalogueSection::WorkspaceLabels)
        {
            let query =
                SectionQuery::for_section(CatalogueSection::WorkspaceLabels);
            let nodes = self.page_connection(
                query.document,
                &query.connection,
                None,
                &deadline,
            )?;
            Some(
                nodes
                    .into_iter()
                    .map(record_from)
                    .collect::<Result<_, _>>()?,
            )
        } else {
            None
        };

        Ok(SectionFetch {
            entries: entries.into_values().collect(),
            workspace_labels,
            unreturned,
        })
    }

    fn team_of_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<TeamEntry>, SurfaceError> {
        let received = self
            .transport()
            .send(TEAM_OF_IDENTIFIER, &json!({ "id": identifier }))?;
        if reports_no_such_issue(&received) {
            return Ok(None);
        }
        let body = interpret(&received, IDENTIFIER_LOOKUP)?;
        match body.pointer("/data/issue/team") {
            None | Some(Value::Null) => Ok(None),
            Some(team) => {
                let field = |name: &str| {
                    team.get(name).and_then(Value::as_str).unwrap_or_default()
                };
                if field("id").is_empty() {
                    return Err(SurfaceError::BadResponse {
                        operation: IDENTIFIER_LOOKUP,
                        reason: "the issue's team carried no id".to_owned(),
                    });
                }
                Ok(Some(TeamEntry::identified(
                    field("id"),
                    field("key"),
                    field("name"),
                )))
            }
        }
    }
}

/// Linear answers an unknown identifier with an `Entity not found` error and
/// no data, which says the issue does not exist rather than that the lookup
/// failed.
fn reports_no_such_issue(received: &crate::transport::Received) -> bool {
    let Some(body) = received.json() else {
        return false;
    };
    let not_found =
        body.get("errors")
            .and_then(Value::as_array)
            .is_some_and(|errors| {
                errors.iter().any(|error| {
                    error
                        .get("message")
                        .and_then(Value::as_str)
                        .is_some_and(|message| message.starts_with(NOT_FOUND))
                })
            });
    not_found && body.get("data").is_none_or(Value::is_null)
}

impl LinearClient {
    /// Verifies credentials against `viewer` and returns the `viewer.json`
    /// shape `{id, name}`.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure, an `errors[]` response, or a
    /// response carrying no viewer id.
    pub fn discover_viewer(&self) -> Result<Value, SurfaceError> {
        let received = self.transport().send(VIEWER, &json!({}))?;
        let body = interpret(&received, "discover viewer")?;
        let id = string_at(&body, "/data/viewer/id").ok_or_else(|| {
            SurfaceError::BadResponse {
                operation: "discover viewer",
                reason: "the response carried no viewer id".to_owned(),
            }
        })?;
        let name = string_at(&body, "/data/viewer/name").unwrap_or_default();
        Ok(json!({ "id": id, "name": name }))
    }

    /// Lists the teams the token can see, as the array the skill renders for
    /// selection. Paginated to exhaustion so a workspace exceeding one page is
    /// fully enumerated.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure or an `errors[]` response.
    pub fn list_teams(&self) -> Result<Value, SurfaceError> {
        Ok(Value::Array(self.paginate_teams()?))
    }

    /// Every visible team node, following the Relay cursor to exhaustion.
    ///
    /// Fails loud on any page failure rather than returning a subset: a
    /// truncated enumeration that dropped a visible team would falsely abort a
    /// valid `additional_teams` or under-scope `all_teams`.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure or an `errors[]` response on
    /// any page.
    pub(crate) fn paginate_teams(&self) -> Result<Vec<Value>, SurfaceError> {
        paginate(
            &PagedConnection::unbounded("teams"),
            &Deadline::never(),
            |cursor| {
                let received = self
                    .transport()
                    .send(TEAMS, &json!({ "cursor": cursor }))?;
                let body = interpret(&received, "list teams")?;
                Ok(body.pointer("/data/teams").cloned().unwrap_or(Value::Null))
            },
        )
    }

    fn page_connection(
        &self,
        document: &'static str,
        connection: &PagedConnection,
        team_ids: Option<&[String]>,
        deadline: &Deadline,
    ) -> Result<Vec<Value>, SurfaceError> {
        paginate(connection, deadline, |cursor| {
            let variables = team_ids.map_or_else(
                || json!({ "after": cursor }),
                |ids| json!({ "ids": ids, "after": cursor }),
            );
            let received = self.transport().send(document, &variables)?;
            let body = interpret(&received, OPERATION)?;
            Ok(body
                .pointer(&format!("/data/{}", connection.name))
                .cloned()
                .unwrap_or(Value::Null))
        })
    }

    fn fill_section(
        &self,
        section: CatalogueSection,
        team_ids: &[String],
        entries: &mut BTreeMap<String, TeamEntry>,
        deadline: &Deadline,
    ) -> Result<(), SurfaceError> {
        let query = SectionQuery::for_section(section);
        for entry in entries.values_mut() {
            if let Some(records) = section_of(entry, section) {
                records.start_empty();
            }
        }
        let nodes = self.page_connection(
            query.document,
            &query.connection,
            Some(team_ids),
            deadline,
        )?;
        for mut node in nodes {
            let owners = attributed_teams(&mut node, query.attribution)?;
            for owner in owners {
                if let Some(records) = entries
                    .get_mut(&owner)
                    .and_then(|entry| section_of(entry, section))
                {
                    records.push(node.clone())?;
                }
            }
        }
        Ok(())
    }
}

/// One team section, seen as a list of nodes to parse into its records.
enum SectionRecords<'a> {
    States(&'a mut Option<Vec<crate::catalogue::CataloguedState>>),
    Labels(&'a mut Option<Vec<CataloguedLabel>>),
    Members(&'a mut Option<Vec<crate::catalogue::CataloguedMember>>),
    Projects(&'a mut Option<Vec<crate::catalogue::CataloguedProject>>),
}

const fn section_of(
    entry: &mut TeamEntry,
    section: CatalogueSection,
) -> Option<SectionRecords<'_>> {
    match section {
        CatalogueSection::States => {
            Some(SectionRecords::States(&mut entry.states))
        }
        CatalogueSection::Labels => {
            Some(SectionRecords::Labels(&mut entry.labels))
        }
        CatalogueSection::Members => {
            Some(SectionRecords::Members(&mut entry.members))
        }
        CatalogueSection::Projects => {
            Some(SectionRecords::Projects(&mut entry.projects))
        }
        CatalogueSection::WorkspaceLabels => None,
    }
}

impl SectionRecords<'_> {
    fn start_empty(self) {
        match self {
            Self::States(records) => *records = Some(Vec::new()),
            Self::Labels(records) => *records = Some(Vec::new()),
            Self::Members(records) => *records = Some(Vec::new()),
            Self::Projects(records) => *records = Some(Vec::new()),
        }
    }

    fn push(self, node: Value) -> Result<(), SurfaceError> {
        match self {
            Self::States(records) => push_record(records, node),
            Self::Labels(records) => push_record(records, node),
            Self::Members(records) => push_record(records, node),
            Self::Projects(records) => push_record(records, node),
        }
    }
}

fn push_record<R: DeserializeOwned>(
    records: &mut Option<Vec<R>>,
    node: Value,
) -> Result<(), SurfaceError> {
    records
        .get_or_insert_with(Vec::new)
        .push(record_from(node)?);
    Ok(())
}

fn record_from<R: DeserializeOwned>(node: Value) -> Result<R, SurfaceError> {
    serde_json::from_value(node).map_err(|error| SurfaceError::BadResponse {
        operation: OPERATION,
        reason: format!("a node did not parse: {error}"),
    })
}

/// The ids of the teams `node` belongs to, with the attribution field removed
/// so it never lands in the record's extra fields.
fn attributed_teams(
    node: &mut Value,
    attribution: Attribution,
) -> Result<Vec<String>, SurfaceError> {
    let Some(object) = node.as_object_mut() else {
        return Ok(Vec::new());
    };
    match attribution {
        Attribution::NoTeam => Ok(Vec::new()),
        Attribution::OwningTeam => Ok(object
            .remove("team")
            .and_then(|team| {
                team.get("id").and_then(Value::as_str).map(str::to_owned)
            })
            .into_iter()
            .collect()),
        Attribution::NestedTeams { connection } => {
            let teams = object.remove("teams").unwrap_or(Value::Null);
            if teams.pointer("/pageInfo/hasNextPage")
                == Some(&Value::Bool(true))
            {
                return Err(SurfaceError::CatalogueTruncated {
                    connection,
                    pages: 1,
                    remedy: "a record belongs to more teams than one page of \
                             its team connection holds",
                });
            }
            Ok(teams
                .get("nodes")
                .and_then(Value::as_array)
                .map(|nodes| {
                    nodes
                        .iter()
                        .filter_map(|team| {
                            team.get("id").and_then(Value::as_str)
                        })
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default())
        }
    }
}

fn refuse_teams_without_states<'a>(
    entries: impl Iterator<Item = &'a TeamEntry>,
) -> Result<(), SurfaceError> {
    let stateless: Vec<&str> = entries
        .filter(|entry| entry.states.as_ref().is_none_or(Vec::is_empty))
        .map(|entry| entry.id.as_str())
        .collect();
    if stateless.is_empty() {
        return Ok(());
    }
    Err(SurfaceError::BadResponse {
        operation: OPERATION,
        reason: format!(
            "team(s) {} have no workflow states",
            stateless.join(", ")
        ),
    })
}

/// Every node of a Relay `connection`, following its cursor page by page.
/// `fetch_page` receives the cursor to resume from and returns the
/// connection object (`nodes` plus `pageInfo`).
///
/// # Errors
///
/// [`SurfaceError::CatalogueTruncated`] when another page would pass the
/// connection's ceiling, [`SurfaceError::DeadlineExpired`] when `deadline`
/// has passed before a page is requested, or the error `fetch_page` returns.
pub(crate) fn paginate(
    connection: &PagedConnection,
    deadline: &Deadline,
    mut fetch_page: impl FnMut(Option<&str>) -> Result<Value, SurfaceError>,
) -> Result<Vec<Value>, SurfaceError> {
    let mut nodes = Vec::new();
    let mut cursor: Option<String> = None;
    let mut pages = 0;
    loop {
        if connection.ceiling.exceeds(pages + 1) {
            return Err(SurfaceError::CatalogueTruncated {
                connection: connection.name,
                pages,
                remedy: connection.remedy,
            });
        }
        if deadline.expired() {
            return Err(SurfaceError::DeadlineExpired {
                operation: connection.name,
            });
        }
        let page = fetch_page(cursor.as_deref())?;
        pages += 1;
        if let Some(page_nodes) = page.get("nodes").and_then(Value::as_array) {
            nodes.extend(page_nodes.iter().cloned());
        }
        let page_info = page.get("pageInfo");
        let has_next = page_info
            .and_then(|info| info.get("hasNextPage"))
            .and_then(Value::as_bool)
            == Some(true);
        cursor = page_info
            .and_then(|info| info.get("endCursor"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        if !has_next || cursor.is_none() {
            return Ok(nodes);
        }
    }
}

fn string_at(body: &Value, pointer: &str) -> Option<String> {
    body.pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;

    use serde_json::json;
    use serde_json::Value;
    use tracker::Ceiling;

    use super::paginate;
    use super::PagedConnection;
    use super::SectionQuery;
    use crate::catalogue::CatalogueSection;
    use crate::surface::SurfaceError;
    use crate::transport::Deadline;

    const fn bounded(name: &'static str, pages: usize) -> PagedConnection {
        PagedConnection {
            name,
            ceiling: Ceiling::Bounded(pages),
            remedy: "narrow it",
        }
    }

    #[test]
    fn each_sections_ceiling_is_three_times_its_page_budget() {
        let ceilings: Vec<(CatalogueSection, Ceiling)> = CatalogueSection::ALL
            .iter()
            .map(|section| {
                (
                    *section,
                    SectionQuery::for_section(*section).connection.ceiling,
                )
            })
            .collect();

        assert_eq!(
            ceilings,
            vec![
                (CatalogueSection::States, Ceiling::Bounded(60)),
                (CatalogueSection::Labels, Ceiling::Bounded(60)),
                (CatalogueSection::Members, Ceiling::Bounded(30)),
                (CatalogueSection::Projects, Ceiling::Bounded(30)),
                (CatalogueSection::WorkspaceLabels, Ceiling::Bounded(60)),
            ]
        );
    }

    fn page(index: usize, has_next: bool) -> Value {
        json!({
            "nodes": [{ "id": format!("node-{index}") }],
            "pageInfo": {
                "hasNextPage": has_next,
                "endCursor": format!("cursor-{index}"),
            },
        })
    }

    #[test]
    fn pagination_exceeding_its_ceiling_fails_loud() {
        let requested = Cell::new(0);

        let error = paginate(
            &bounded("issueLabels", 2),
            &Deadline::starting_now(Duration::from_secs(60)),
            |_cursor| {
                requested.set(requested.get() + 1);
                Ok(page(requested.get(), true))
            },
        )
        .expect_err("a third page is past the ceiling");

        assert!(
            matches!(
                error,
                SurfaceError::CatalogueTruncated {
                    connection: "issueLabels",
                    ..
                }
            ),
            "{error}"
        );
        assert_eq!(requested.get(), 2, "the third page is never requested");
    }

    #[test]
    fn pagination_past_its_deadline_is_retryable() {
        let error = paginate(
            &bounded("issueLabels", 2),
            &Deadline::starting_now(Duration::ZERO),
            |_cursor| Ok(page(1, true)),
        )
        .expect_err("an expired deadline stops the fetch");

        assert!(
            matches!(error, SurfaceError::DeadlineExpired { .. }),
            "{error}"
        );
    }

    #[test]
    fn pagination_threads_each_end_cursor_into_the_next_request() {
        let cursors = std::cell::RefCell::new(Vec::new());

        let nodes = paginate(
            &PagedConnection::unbounded("teams"),
            &Deadline::starting_now(Duration::from_secs(60)),
            |cursor| {
                cursors.borrow_mut().push(cursor.map(str::to_owned));
                let index = cursors.borrow().len();
                Ok(page(index, index < 3))
            },
        )
        .expect("three pages");

        assert_eq!(nodes.len(), 3);
        assert_eq!(
            cursors.into_inner(),
            vec![
                None,
                Some("cursor-1".to_owned()),
                Some("cursor-2".to_owned())
            ]
        );
    }
}

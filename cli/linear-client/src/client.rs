//! `impl RemoteTracker for LinearClient`.
//!
//! No conversion layer in either direction: Linear is Markdown-native, so a
//! body passes through verbatim.

use std::collections::BTreeSet;
use std::path::Path;

use remote_projection::Integration;
use remote_projection::Op;
use serde_json::json;
use serde_json::Value;
use tracker::CreatePreview;
use tracker::Discovery;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::FieldResolution;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker::ValidationOutcome;
use tracker_support::port_body;
use tracker_support::ClockJitter;
use tracker_support::CredentialContext;
use tracker_support::SystemSleeper;
use tracker_support::TransportConfig;

use crate::auth::check_identifier;
use crate::auth::resolve_credentials;
use crate::auth::Credentials;
use crate::catalogue::CatalogueStates;
use crate::classify::carries_errors;
use crate::classify::classify;
use crate::classify::classify_errors;
use crate::classify::Operation;
use crate::classify::Outcome;
use crate::error::ClientError;
use crate::failure::LinearFailure;
use crate::filter::compose;
use crate::filter::Search;
use crate::filter::StateResolver;
use crate::filter::FETCH_PAGE_SIZE;
use crate::surface::interpret as interpret_surface;
use crate::surface::SurfaceError;
use crate::transport::Deadline;
use crate::transport::Received;
use crate::transport::Transport;
use crate::upload::UploadTransport;

const SHOW: &str = "query($id: String!) {
    issue(id: $id) {
      id identifier title updatedAt
      description
    }
  }";

/// The read-side show projection the `show` subcommand renders. The port `show`
/// projects to a stamp and a Markdown body; this keeps the state, assignee and
/// comments the bash `show` table shows, over a query distinct from the port's.
const SHOW_DETAILED: &str = "query($id: String!) {
    issue(id: $id) {
      id identifier title updatedAt
      state { name }
      assignee { name }
      description
      comments { nodes { body } }
    }
  }";

const CREATE: &str = "mutation($input: IssueCreateInput!) {
    issueCreate(input: $input) { success issue { id identifier } }
  }";

const UPDATE: &str = "mutation($id: String!, $input: IssueUpdateInput!) {
    issueUpdate(id: $id, input: $input) { success issue { id identifier } }
  }";

const SEARCH: &str =
    "query($cursor: String, $filter: IssueFilter, $first: Int) {
    issues(first: $first, after: $cursor, filter: $filter) {
      nodes { id identifier title updatedAt }
      pageInfo { hasNextPage endCursor }
    }
  }";

/// The read-side search projection the `search` subcommand renders. Distinct
/// from `SEARCH` — which the sync engine's bulk read (`fetch_all`/`fetch_page`)
/// spends its complexity budget on — so widening the projection never changes
/// the port read's request shape. It selects the state and assignee names the
/// bash search table shows and keeps the title, which the stamps-only port
/// `search` discards.
const SEARCH_PROJECTION: &str =
    "query($cursor: String, $filter: IssueFilter, $first: Int) {
    issues(first: $first, after: $cursor, filter: $filter) {
      nodes {
        id identifier title updatedAt
        state { name }
        assignee { name }
      }
      pageInfo { hasNextPage endCursor }
    }
  }";

/// One page of the bulk read: the issues it accounted for, and the cursor to
/// the next page when there is one.
type Page = (Vec<(String, RemoteTimestamp)>, Option<String>);

/// The accumulated result of a detailed search.
///
/// The raw projection nodes in arrival order, and whether the retrieval was cut
/// short (a cap-hit or deadline, mirroring the bash `.data.issues.truncated`
/// flag).
#[derive(Debug)]
pub struct DetailedPage {
    pub nodes: Vec<Value>,
    pub truncated: bool,
}

pub struct LinearClient {
    transport: Transport,
    upload: UploadTransport,
    team_key: Option<String>,
    states: Box<dyn StateResolver>,
}

impl LinearClient {
    #[must_use]
    pub fn new(
        transport: Transport,
        upload: UploadTransport,
        team_key: Option<String>,
        states: Box<dyn StateResolver>,
    ) -> Self {
        Self {
            transport,
            upload,
            team_key,
            states,
        }
    }

    /// Builds a client from configuration, resolving every value eagerly into
    /// owned data — the registry's returned trait object carries no lifetime.
    ///
    /// # Errors
    ///
    /// [`ClientError`] naming the configuration value that is missing or
    /// refused.
    pub fn from_config(
        context: &CredentialContext<'_>,
        integrations_root: &Path,
    ) -> Result<Self, ClientError> {
        let credentials = resolve_credentials(context, integrations_root)?;
        let team_key = crate::auth::catalogue_team_key(integrations_root);
        let transport = Transport::to_linear(
            credentials,
            TransportConfig::default(),
            Box::new(SystemSleeper),
            Box::new(ClockJitter),
        )?;
        let upload = UploadTransport::production()?;
        let states = CatalogueStates::load(integrations_root);
        Ok(Self::new(transport, upload, team_key, Box::new(states)))
    }

    #[must_use]
    pub const fn transport(&self) -> &Transport {
        &self.transport
    }

    #[must_use]
    pub(crate) const fn upload_transport(&self) -> &UploadTransport {
        &self.upload
    }

    #[must_use]
    pub fn states(&self) -> &dyn StateResolver {
        self.states.as_ref()
    }

    const fn credentials(&self) -> &Credentials {
        self.transport.credentials()
    }

    /// Whether an identifier belongs to the team this client is scoped to.
    ///
    /// A Linear identifier is `<TEAM_KEY>-<number>`, so the prefix answers it —
    /// but only when the team key is known. Without it, nothing can be proved
    /// about scope, and the port's rule then applies: report every unseen id as
    /// indeterminate rather than inferring absence.
    fn in_scope(&self, id: &ExternalId) -> Option<bool> {
        let key = self.team_key.as_ref()?;
        Some(
            id.as_str()
                .split_once('-')
                .is_some_and(|(prefix, _)| prefix == key.as_str()),
        )
    }

    /// A GraphQL call whose failure carries the structured discriminant. The
    /// binary maps its [`Outcome`] straight to an exit code; the port derives a
    /// `TrackerError` from the same value.
    fn call_op(
        &self,
        document: &str,
        variables: &Value,
        operation: Operation,
        detail: &str,
    ) -> Result<Value, LinearFailure> {
        let received =
            self.transport.send(document, variables).map_err(|error| {
                LinearFailure::wire(
                    Outcome::Transport,
                    operation,
                    error.to_string(),
                )
            })?;
        Self::interpret_outcome(&received).map_err(|outcome| {
            LinearFailure::wire(outcome, operation, detail.to_owned())
        })
    }

    /// Classifies a response body the way the bash classifies it — a 200
    /// carrying `errors[]` is a failure, and a 400's body decides between auth,
    /// complexity, rate limiting and a bad request — into the wire [`Outcome`]
    /// the exit code is read from.
    fn interpret_outcome(received: &Received) -> Result<Value, Outcome> {
        let Some(body) = received.json() else {
            return Err(if (200..300).contains(&received.status) {
                Outcome::NonJsonBody
            } else {
                Outcome::Unexpected
            });
        };
        match received.status {
            200..300 if carries_errors(&body) => {
                Err(Outcome::SuccessWithErrors(classify_errors(&body)))
            }
            200..300 => Ok(body),
            401 => Err(Outcome::Unauthorised),
            400 => Err(Outcome::BadRequest(classify_errors(&body))),
            500..600 => Err(Outcome::ServerError),
            _ => Err(Outcome::Unexpected),
        }
    }

    fn interpret(
        received: &Received,
        operation: Operation,
        detail: &str,
    ) -> Result<Value, TrackerError> {
        Self::interpret_outcome(received)
            .map_err(|outcome| classify(outcome, operation, detail))
    }

    /// One page of a search, following the Relay cursor.
    fn fetch_page(
        &self,
        search: &Search,
        cursor: Option<&str>,
        deadline: &Deadline,
    ) -> Result<Page, String> {
        if deadline.expired() {
            return Err("the operation deadline expired".to_owned());
        }
        let filter = compose(search, self.states.as_ref())
            .map_err(|error| error.to_string())?;
        let variables = json!({
            "filter": filter,
            "first": FETCH_PAGE_SIZE,
            "cursor": cursor,
        });
        let received = self
            .transport
            .send(SEARCH, &variables)
            .map_err(|error| error.to_string())?;
        let body = Self::interpret(&received, Operation::Read, "fetch_all")
            .map_err(|error| error.to_string())?;

        let issues = body.pointer("/data/issues");
        let mut found = Vec::new();
        if let Some(nodes) = issues
            .and_then(|issues| issues.get("nodes"))
            .and_then(Value::as_array)
        {
            for node in nodes {
                if let Some(identifier) =
                    node.get("identifier").and_then(Value::as_str)
                {
                    found.push((
                        identifier.to_owned(),
                        stamp(node.get("updatedAt")),
                    ));
                }
            }
        }
        let page = issues.and_then(|issues| issues.get("pageInfo"));
        let cursor = page
            .filter(|page| {
                page.get("hasNextPage").and_then(Value::as_bool) == Some(true)
            })
            .and_then(|page| page.get("endCursor"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        Ok((found, cursor))
    }

    /// Pages a search to exhaustion, returning the accumulated index and, when
    /// the retrieval was cut short, the reason. A cap-hit, a deadline or a
    /// failed page all leave `Some(reason)`; a clean finish leaves `None`.
    fn page_all(
        &self,
        search: &Search,
    ) -> (Vec<(String, RemoteTimestamp)>, Option<String>) {
        let deadline = self.transport.deadline();
        let cap = self.transport.config().max_pages;
        let mut index: Vec<(String, RemoteTimestamp)> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut truncated = None;

        for page in 1..=cap {
            match self.fetch_page(search, cursor.as_deref(), &deadline) {
                Ok((mut found, next)) => {
                    index.append(&mut found);
                    cursor = next;
                    if cursor.is_none() {
                        break;
                    }
                    if page == cap {
                        truncated =
                            Some(format!("the {cap}-page cap was reached"));
                    }
                }
                Err(reason) => {
                    truncated = Some(reason);
                    break;
                }
            }
        }
        if let Some(reason) = &truncated {
            tracing::warn!(
                reason = %reason,
                "linear: the retrieval was incomplete"
            );
        }
        (index, truncated)
    }

    /// Pages a search over the richer [`SEARCH_PROJECTION`] to exhaustion,
    /// returning the raw nodes the `search` subcommand renders. Unlike the port
    /// `search`, a wire failure is an error rather than a degraded page — the
    /// bash search flow propagates the transport code — while a cap-hit or
    /// expired deadline is a successful, truncated result.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a rejected filter (an unknown state), a transport
    /// failure, or a response carrying `errors[]`.
    pub fn search_detailed(
        &self,
        search: &Search,
    ) -> Result<DetailedPage, SurfaceError> {
        let filter = compose(search, self.states.as_ref())?;
        let cap = self.transport.config().max_pages;
        let deadline = self.transport.deadline();
        let mut nodes = Vec::new();
        let mut cursor: Option<String> = None;
        let mut truncated = false;

        for page in 1..=cap {
            if deadline.expired() {
                truncated = true;
                break;
            }
            let variables = json!({
                "filter": filter,
                "first": FETCH_PAGE_SIZE,
                "cursor": cursor,
            });
            let received =
                self.transport.send(SEARCH_PROJECTION, &variables)?;
            let body = interpret_surface(&received, "search")?;

            let issues = body.pointer("/data/issues");
            if let Some(page_nodes) = issues
                .and_then(|issues| issues.get("nodes"))
                .and_then(Value::as_array)
            {
                nodes.extend(page_nodes.iter().cloned());
            }
            let page_info = issues.and_then(|issues| issues.get("pageInfo"));
            let has_next = page_info
                .and_then(|info| info.get("hasNextPage"))
                .and_then(Value::as_bool)
                == Some(true);
            cursor = page_info
                .and_then(|info| info.get("endCursor"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            if !has_next || cursor.is_none() {
                break;
            }
            if page == cap {
                truncated = true;
            }
        }
        Ok(DetailedPage { nodes, truncated })
    }

    /// Fetches one issue's full detail for the `show` subcommand, returning the
    /// raw GraphQL body. A `comments` cap keeps only the last N comment nodes,
    /// reproducing the bash flow's client-side slice.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a refused identifier, a transport failure, or a
    /// response carrying `errors[]`.
    pub fn show_detailed(
        &self,
        id: &ExternalId,
        comments: Option<usize>,
    ) -> Result<Value, SurfaceError> {
        check_identifier(id.as_str())?;
        let received = self
            .transport
            .send(SHOW_DETAILED, &json!({"id": id.as_str()}))?;
        let mut body = interpret_surface(&received, "show")?;
        if let Some(limit) = comments {
            slice_comments(&mut body, limit);
        }
        Ok(body)
    }

    /// `create`, surfacing the structured discriminant. The port `create`
    /// derives its `TrackerError` from this.
    ///
    /// # Errors
    ///
    /// [`LinearFailure`] carrying either the wire outcome or the post-create
    /// unwritable-identifier case.
    pub fn create_op(
        &self,
        title: &str,
        body: &str,
        _kind: &str,
    ) -> Result<ExternalId, LinearFailure> {
        // Linear has no per-issue type: `kind` has no destination.
        let variables = json!({"input": {
            "teamId": self.credentials().team_id,
            "title": title,
            "description": body,
        }});
        let response =
            self.call_op(CREATE, &variables, Operation::Create, title)?;
        let identifier = response
            .pointer("/data/issueCreate/issue/identifier")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                LinearFailure::wire(
                    Outcome::NonJsonBody,
                    Operation::Create,
                    "the create response carried no identifier".to_owned(),
                )
            })?;
        // The issue exists remotely, so an unusable identifier is Terminal.
        check_identifier(identifier).map_err(|error| {
            LinearFailure::UnwritableIdentifier {
                identifier: identifier.to_owned(),
                reason: error.to_string(),
            }
        })?;
        Ok(ExternalId::new(identifier.to_owned()))
    }

    /// `update`, surfacing the structured discriminant.
    ///
    /// # Errors
    ///
    /// [`LinearFailure`] for a refused identifier or a wire failure.
    pub fn update_op(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), LinearFailure> {
        refuse_identifier_op(id, Operation::Update)?;
        let variables = json!({
            "id": id.as_str(),
            "input": {"title": title, "description": body},
        });
        self.call_op(UPDATE, &variables, Operation::Update, id.as_str())?;
        Ok(())
    }

    /// `show`, surfacing the structured discriminant.
    ///
    /// # Errors
    ///
    /// [`LinearFailure`] for a refused identifier, a wire failure, or a
    /// response that cannot be projected.
    pub fn show_op(
        &self,
        id: &ExternalId,
    ) -> Result<RemoteIssue, LinearFailure> {
        refuse_identifier_op(id, Operation::Read)?;
        let received = self
            .transport
            .send(SHOW, &json!({"id": id.as_str()}))
            .map_err(|error| {
            LinearFailure::wire(
                Outcome::Transport,
                Operation::Read,
                error.to_string(),
            )
        })?;
        let body = Self::interpret_outcome(&received).map_err(|outcome| {
            LinearFailure::wire(
                outcome,
                Operation::Read,
                id.as_str().to_owned(),
            )
        })?;
        let projected = remote_projection::project_raw(
            Integration::Linear,
            Op::Body,
            &received.body,
        )
        .map_err(|error| {
            LinearFailure::wire(
                Outcome::NonJsonBody,
                Operation::Read,
                error.to_string(),
            )
        })?;
        Ok(RemoteIssue {
            updated: stamp(body.pointer("/data/issue/updatedAt")),
            body: port_body(&projected),
        })
    }
}

/// A populated stamp is held verbatim. A blank, absent or `null` one is
/// `NotReported`, never `Reported("")`.
fn stamp(value: Option<&Value>) -> RemoteTimestamp {
    value
        .and_then(Value::as_str)
        .filter(|stamp| !stamp.is_empty())
        .map_or(RemoteTimestamp::NotReported, |stamp| {
            RemoteTimestamp::Reported(stamp.to_owned())
        })
}

/// Keeps only the last `limit` comment nodes, reproducing the bash `--comments`
/// client-side slice.
fn slice_comments(body: &mut Value, limit: usize) {
    if let Some(nodes) = body
        .pointer_mut("/data/issue/comments/nodes")
        .and_then(Value::as_array_mut)
    {
        if nodes.len() > limit {
            let start = nodes.len() - limit;
            nodes.drain(0..start);
        }
    }
}

fn refuse_identifier_op(
    id: &ExternalId,
    operation: Operation,
) -> Result<(), LinearFailure> {
    check_identifier(id.as_str()).map_err(|error| {
        LinearFailure::wire(Outcome::Transport, operation, error.to_string())
    })
}

fn refuse_identifier(
    id: &ExternalId,
    operation: Operation,
) -> Result<(), TrackerError> {
    refuse_identifier_op(id, operation).map_err(TrackerError::from)
}

impl RemoteTracker for LinearClient {
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        self.create_op(title, body, kind)
            .map_err(TrackerError::from)
    }

    fn update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), TrackerError> {
        self.update_op(id, title, body).map_err(TrackerError::from)
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        self.show_op(id).map_err(TrackerError::from)
    }

    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError> {
        let mut outcome = FetchOutcome {
            found: Vec::new(),
            absent: Vec::new(),
            indeterminate: Vec::new(),
        };
        let mut seen = BTreeSet::new();
        let requested: Vec<&ExternalId> = ids
            .iter()
            .filter(|id| seen.insert(id.as_str().to_owned()))
            .collect();
        if requested.is_empty() {
            return Ok(outcome);
        }
        for id in &requested {
            refuse_identifier(id, Operation::Read).map_err(|error| {
                TrackerError::Retryable {
                    detail: error.to_string(),
                }
            })?;
        }

        let team_search = Search {
            team_id: Some(self.credentials().team_id.clone()),
            ..Search::default()
        };
        let (index, truncated) = self.page_all(&team_search);

        for id in requested {
            let stamp = index
                .iter()
                .find(|(identifier, _)| identifier == id.as_str())
                .map(|(_, stamp)| stamp.clone());
            if let Some(stamp) = stamp {
                outcome.found.push((id.clone(), stamp));
                continue;
            }
            // An id the retrieval never had scope to see, or a retrieval that
            // was cut short, proves nothing about absence. Reporting either as
            // absent is what makes a sync unlink a live issue.
            if truncated.is_some() || self.in_scope(id) != Some(true) {
                outcome.indeterminate.push(id.clone());
            } else {
                outcome.absent.push(id.clone());
            }
        }
        Ok(outcome)
    }

    fn search(&self, scope: &SearchScope) -> Result<Discovery, TrackerError> {
        let mut search = Search {
            team_id: scope
                .project
                .clone()
                .or_else(|| Some(self.credentials().team_id.clone())),
            ..Search::default()
        };
        for (field, value) in &scope.filters {
            match field.as_str() {
                "state" => search.state = Some(value.clone()),
                "assignee" => search.assignee = Some(value.clone()),
                "label" => search.label = Some(value.clone()),
                "text" => search.text = Some(value.clone()),
                _ => {}
            }
        }
        let (index, truncated) = self.page_all(&search);
        Ok(Discovery {
            found: index
                .into_iter()
                .map(|(id, stamp)| (ExternalId::new(id), stamp))
                .collect(),
            complete: truncated.is_none(),
        })
    }

    fn preview_create(
        &self,
        _kind: &str,
    ) -> Result<CreatePreview, TrackerError> {
        // Single-team, catalogue-fixed: Linear has no project key to resolve
        // and no per-issue type.
        Ok(CreatePreview {
            project: FieldResolution::Unset,
            issue_type: FieldResolution::Unset,
        })
    }

    fn validate_update(
        &self,
        _id: &ExternalId,
        title: &str,
        _body: &str,
    ) -> ValidationOutcome {
        // Local composition check: Linear's GraphQL has no non-mutating
        // update-validation endpoint, so a remote pre-flight is unavailable.
        if title.trim().is_empty() {
            ValidationOutcome::Rejected {
                reasons: vec![
                    "title is required but the payload leaves it empty"
                        .to_owned(),
                ],
            }
        } else {
            ValidationOutcome::Valid
        }
    }
}

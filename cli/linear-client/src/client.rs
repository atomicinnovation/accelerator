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
use crate::filter::compose;
use crate::filter::Search;
use crate::filter::StateResolver;
use crate::filter::FETCH_PAGE_SIZE;
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

/// One page of the bulk read: the issues it accounted for, and the cursor to
/// the next page when there is one.
type Page = (Vec<(String, RemoteTimestamp)>, Option<String>);

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

    /// A GraphQL call, with the body classified the way the bash classifies it:
    /// a 200 carrying `errors[]` is a failure, and a 400's body decides between
    /// auth, complexity, rate limiting and a bad request.
    fn call(
        &self,
        document: &str,
        variables: &Value,
        operation: Operation,
        detail: &str,
    ) -> Result<Value, TrackerError> {
        let received =
            self.transport.send(document, variables).map_err(|error| {
                classify(Outcome::Transport, operation, &error.to_string())
            })?;
        Self::interpret(&received, operation, detail)
    }

    fn interpret(
        received: &Received,
        operation: Operation,
        detail: &str,
    ) -> Result<Value, TrackerError> {
        let outcome = |outcome| classify(outcome, operation, detail);
        let Some(body) = received.json() else {
            return Err(outcome(if (200..300).contains(&received.status) {
                Outcome::NonJsonBody
            } else {
                Outcome::Unexpected
            }));
        };
        match received.status {
            200..300 if carries_errors(&body) => {
                Err(outcome(Outcome::SuccessWithErrors(classify_errors(&body))))
            }
            200..300 => Ok(body),
            401 => Err(outcome(Outcome::Unauthorised)),
            400 => Err(outcome(Outcome::BadRequest(classify_errors(&body)))),
            500..600 => Err(outcome(Outcome::ServerError)),
            _ => Err(outcome(Outcome::Unexpected)),
        }
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

fn refuse_identifier(
    id: &ExternalId,
    operation: Operation,
) -> Result<(), TrackerError> {
    check_identifier(id.as_str()).map_err(|error| {
        classify(Outcome::Transport, operation, &error.to_string())
    })
}

impl RemoteTracker for LinearClient {
    fn create(
        &self,
        title: &str,
        body: &str,
        _kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        // Linear has no per-issue type: `kind` has no destination.
        let variables = json!({"input": {
            "teamId": self.credentials().team_id,
            "title": title,
            "description": body,
        }});
        let response =
            self.call(CREATE, &variables, Operation::Create, title)?;
        let identifier = response
            .pointer("/data/issueCreate/issue/identifier")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                classify(
                    Outcome::NonJsonBody,
                    Operation::Create,
                    "the create response carried no identifier",
                )
            })?;
        // The issue exists remotely, so an unusable identifier is Terminal.
        check_identifier(identifier).map_err(|error| {
            TrackerError::Terminal {
                detail: format!(
                    "linear create: the issue was created as \
                     {identifier:?}, which cannot be written back — {error}"
                ),
            }
        })?;
        Ok(ExternalId::new(identifier.to_owned()))
    }

    fn update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), TrackerError> {
        refuse_identifier(id, Operation::Update)?;
        let variables = json!({
            "id": id.as_str(),
            "input": {"title": title, "description": body},
        });
        self.call(UPDATE, &variables, Operation::Update, id.as_str())?;
        Ok(())
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        refuse_identifier(id, Operation::Read)?;
        let received = self
            .transport
            .send(SHOW, &json!({"id": id.as_str()}))
            .map_err(|error| {
            classify(Outcome::Transport, Operation::Read, &error.to_string())
        })?;
        let body = Self::interpret(&received, Operation::Read, id.as_str())?;
        let projected = remote_projection::project_raw(
            Integration::Linear,
            Op::Body,
            &received.body,
        )
        .map_err(|error| {
            classify(Outcome::NonJsonBody, Operation::Read, &error.to_string())
        })?;
        Ok(RemoteIssue {
            updated: stamp(body.pointer("/data/issue/updatedAt")),
            body: port_body(&projected),
        })
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

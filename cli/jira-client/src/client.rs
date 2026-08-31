//! `impl RemoteTracker for JiraClient`: the four port operations over the
//! bounded transport.

use std::collections::BTreeSet;

use remote_projection::Integration;
use remote_projection::Op;
use reqwest::Method;
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
use tracker::ScopeError;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker::ValidationOutcome;
use tracker_support::port_body;
use tracker_support::ClockJitter;
use tracker_support::CredentialContext;
use tracker_support::SystemSleeper;
use tracker_support::TransportConfig;

use crate::adf::markdown_to_document;
use crate::auth::resolve_credentials;
use crate::classify::Operation;
use crate::classify::Outcome;
use crate::error::ClientError;
use crate::failure::JiraFailure;
use crate::jql::compose;
use crate::jql::key_clause;
use crate::jql::AccountResolver;
use crate::jql::Family;
use crate::jql::FieldResolver;
use crate::jql::FixedResolver;
use crate::jql::Search;
use crate::path;
use crate::transport::Deadline;
use crate::transport::Received;
use crate::transport::Transport;

/// Jira's own bulk-read bounds.
const CHUNK: usize = 50;
const PAGE_SIZE: u64 = 100;

const SEARCH_PATH: &str = "/rest/api/3/search/jql";
const ISSUE_PATH: &str = "/rest/api/3/issue";

/// One discovery page: the issues it accounted for, and the cursor to the next
/// page when there is one.
type DiscoveryPage = (Vec<(ExternalId, RemoteTimestamp)>, Option<String>);

pub struct JiraClient {
    transport: Transport,
    project: String,
    accounts: Box<dyn AccountResolver>,
    fields: Box<dyn FieldResolver>,
}

impl JiraClient {
    #[must_use]
    pub fn new(
        transport: Transport,
        project: String,
        accounts: Box<dyn AccountResolver>,
        fields: Box<dyn FieldResolver>,
    ) -> Self {
        Self {
            transport,
            project,
            accounts,
            fields,
        }
    }

    /// Builds a client from configuration.
    ///
    /// Every value is resolved eagerly into owned data: the registry returns a
    /// `Box<dyn RemoteTracker>` with no lifetime, so a client retaining a
    /// borrow of the registry's `&dyn ConfigAccess` could not be boxed into it.
    ///
    /// # Errors
    ///
    /// [`ClientError`] naming the configuration value that is missing or
    /// refused.
    pub fn from_config(
        context: &CredentialContext<'_>,
    ) -> Result<Self, ClientError> {
        let credentials = resolve_credentials(context)?;
        let project = crate::auth::project_code(context.config)?;
        let transport = Transport::new(
            credentials,
            TransportConfig::default(),
            Box::new(SystemSleeper),
            Box::new(ClockJitter),
        )?;
        Ok(Self::new(
            transport,
            project,
            Box::new(FixedResolver::new()),
            Box::new(FixedResolver::new()),
        ))
    }

    #[must_use]
    pub const fn transport(&self) -> &Transport {
        &self.transport
    }

    #[must_use]
    pub fn accounts(&self) -> &dyn AccountResolver {
        self.accounts.as_ref()
    }

    #[must_use]
    pub fn fields(&self) -> &dyn FieldResolver {
        self.fields.as_ref()
    }

    #[must_use]
    pub fn project(&self) -> &str {
        &self.project
    }

    /// The composed, validated path for one issue.
    ///
    /// Every interpolated segment is percent-encoded and every id is checked
    /// for frontmatter safety first: ids read from the local corpus are as
    /// untrusted as ids from a response, having been written by a previous
    /// sync, by hand, or by a tracker that may since have been compromised.
    /// `suffix` is a fixed literal (`/comment`, `/transitions`), never
    /// caller-supplied.
    pub(crate) fn issue_path(
        id: &str,
        suffix: &str,
    ) -> Result<String, ClientError> {
        Self::assert_identifier(id)?;
        let path = format!("{ISSUE_PATH}/{}{suffix}", path::encode_segment(id));
        path::validate_composed(&path, &[id])?;
        Ok(path)
    }

    /// The composed, validated path for a comment on an issue. Both the issue
    /// key and the comment id are untrusted, so both are checked and encoded.
    pub(crate) fn issue_comment_path(
        key: &str,
        comment_id: &str,
    ) -> Result<String, ClientError> {
        Self::assert_identifier(key)?;
        Self::assert_identifier(comment_id)?;
        let path = format!(
            "{ISSUE_PATH}/{}/comment/{}",
            path::encode_segment(key),
            path::encode_segment(comment_id),
        );
        path::validate_composed(&path, &[key, comment_id])?;
        Ok(path)
    }

    fn assert_identifier(id: &str) -> Result<(), ClientError> {
        tracker_support::identifier_is_safe(id).map_err(|refusal| {
            ClientError::BadIdentifier {
                identifier: id.to_owned(),
                reason: refusal.to_string(),
            }
        })
    }

    /// Reads one issue, returning the response **text** as well as its parsed
    /// form: the projection re-serialises the description, and only the raw
    /// bytes carry a numeric literal faithfully.
    fn read(&self, id: &ExternalId) -> Result<(String, Value), JiraFailure> {
        let path = Self::issue_path(id.as_str(), "")
            .map_err(|error| read_failure(&error.to_string()))?;
        let received = self
            .transport
            .send(
                &Method::GET,
                &path,
                &[("fields", "updated,summary,description")],
                None,
            )
            .map_err(|error| read_failure(&error.to_string()))?;
        let parsed = json_body_op(&received, Operation::Read, id.as_str())?;
        Ok((received.body, parsed))
    }

    /// One 50-key chunk, following the `nextPageToken` cursor.
    ///
    /// The page cap is **per chunk**: a global cap would mark whole chunks
    /// indeterminate for a large corpus. A cap-hit, a deadline expiry and a
    /// failure all resolve the same way — the chunk's keys become
    /// indeterminate, never absent.
    fn fetch_chunk(
        &self,
        chunk: &[&ExternalId],
        deadline: &Deadline,
    ) -> Result<Vec<(String, RemoteTimestamp)>, String> {
        let keys: Vec<String> =
            chunk.iter().map(|id| id.as_str().to_owned()).collect();
        let clause = key_clause(&keys).map_err(|error| error.to_string())?;
        let mut found = Vec::new();
        let mut cursor: Option<String> = None;
        let cap = self.transport.config().max_pages;

        for page in 1..=cap {
            if deadline.expired() {
                return Err("the operation deadline expired".to_owned());
            }
            let mut body = json!({
                "jql": clause,
                "fields": ["updated"],
                "fieldsByKeys": false,
                "maxResults": PAGE_SIZE
            });
            if let Some(token) = &cursor {
                body["nextPageToken"] = json!(token);
            }
            let payload = serde_json::to_string(&body)
                .map_err(|error| error.to_string())?;
            let received = self
                .transport
                .send(&Method::POST, SEARCH_PATH, &[], Some(&payload))
                .map_err(|error| error.to_string())?;
            if !(200..300).contains(&received.status) {
                return Err(format!("status {}", received.status));
            }
            let page_body: Value = serde_json::from_str(&received.body)
                .map_err(|_| "a non-JSON search response".to_owned())?;
            if let Some(issues) =
                page_body.get("issues").and_then(Value::as_array)
            {
                for issue in issues {
                    if let Some(key) = issue.get("key").and_then(Value::as_str)
                    {
                        found.push((key.to_owned(), timestamp(issue)));
                    }
                }
            }
            cursor = page_body
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if cursor.is_none() {
                return Ok(found);
            }
            if page == cap {
                return Err(format!("the {cap}-page cap was reached"));
            }
        }
        Ok(found)
    }

    /// Pages an unkeyed discovery, following the `nextPageToken` cursor.
    ///
    /// A read: every mid-flight failure — a cap-hit, a deadline, a non-2xx, a
    /// non-JSON body, a transport failure — degrades to `complete == false` over
    /// what was seen so far, never an error. Only an uncomposable scope
    /// (no project and not `all_projects`, or an unquotable filter) is a
    /// pre-flight `Err`.
    fn discover(&self, scope: &SearchScope) -> Result<Discovery, TrackerError> {
        let search = Search {
            project: scope.project.clone(),
            all_projects: scope.all_projects,
            families: scope
                .filters
                .iter()
                .map(|(field, value)| Family {
                    field: field.clone(),
                    values: vec![value.clone()],
                })
                .collect(),
            ..Search::default()
        };
        let clause =
            compose(&search, self.accounts.as_ref(), self.fields.as_ref())
                .map_err(|error| {
                    TrackerError::from(JiraFailure::ComposeRejected {
                        detail: error.to_string(),
                    })
                })?;

        let mut found = Vec::new();
        let mut cursor: Option<String> = None;
        let cap = self.transport.config().max_pages;
        let deadline = self.transport.deadline();
        for page in 1..=cap {
            if deadline.expired() {
                return Ok(Discovery {
                    found,
                    complete: false,
                });
            }
            let Some((mut issues, next)) =
                self.discover_page(&clause, cursor.as_deref())
            else {
                return Ok(Discovery {
                    found,
                    complete: false,
                });
            };
            found.append(&mut issues);
            cursor = next;
            if cursor.is_none() {
                return Ok(Discovery {
                    found,
                    complete: true,
                });
            }
            if page == cap {
                return Ok(Discovery {
                    found,
                    complete: false,
                });
            }
        }
        Ok(Discovery {
            found,
            complete: true,
        })
    }

    /// One discovery page: the issues it saw and the next cursor, or `None` on
    /// any failure — which the caller reads as an incomplete retrieval.
    fn discover_page(
        &self,
        clause: &str,
        cursor: Option<&str>,
    ) -> Option<DiscoveryPage> {
        let mut body = json!({
            "jql": clause,
            "fields": ["updated"],
            "fieldsByKeys": false,
            "maxResults": PAGE_SIZE
        });
        if let Some(token) = cursor {
            body["nextPageToken"] = json!(token);
        }
        let payload = serde_json::to_string(&body).ok()?;
        let received = self
            .transport
            .send(&Method::POST, SEARCH_PATH, &[], Some(&payload))
            .ok()?;
        if !(200..300).contains(&received.status) {
            return None;
        }
        let page_body: Value = serde_json::from_str(&received.body).ok()?;
        let mut issues = Vec::new();
        if let Some(array) = page_body.get("issues").and_then(Value::as_array) {
            for issue in array {
                if let Some(key) = issue.get("key").and_then(Value::as_str) {
                    issues.push((
                        ExternalId::new(key.to_owned()),
                        timestamp(issue),
                    ));
                }
            }
        }
        let next = page_body
            .get("nextPageToken")
            .and_then(Value::as_str)
            .map(str::to_owned);
        Some((issues, next))
    }

    /// Resolves the configured project key against the live catalogue: absent
    /// configuration is `Unset`, a key `discover_projects` confirms is
    /// `Resolved`, and a configured key the tracker does not hold is
    /// `Unresolvable` — the state the create preview exists to flag.
    fn resolve_project(&self) -> Result<FieldResolution, TrackerError> {
        if self.project.is_empty() {
            return Ok(FieldResolution::Unset);
        }
        let discovered = self.discover_projects().map_err(|error| {
            TrackerError::from(JiraFailure::ReadFailure {
                detail: error.to_string(),
            })
        })?;
        let exists = discovered
            .get("projects")
            .and_then(Value::as_array)
            .is_some_and(|projects| {
                projects.iter().any(|project| {
                    project.get("key").and_then(Value::as_str)
                        == Some(self.project.as_str())
                })
            });
        if exists {
            Ok(FieldResolution::Resolved(self.project.clone()))
        } else {
            Ok(FieldResolution::Unresolvable(self.project.clone()))
        }
    }

    /// `show`, surfacing the structured discriminant. The port `show` derives
    /// its `TrackerError` from this.
    ///
    /// # Errors
    ///
    /// [`JiraFailure`] for a transport failure, a non-2xx status, a non-JSON
    /// body, or a response that cannot be projected.
    pub fn show_op(&self, id: &ExternalId) -> Result<RemoteIssue, JiraFailure> {
        let (raw, payload) = self.read(id)?;
        let projected =
            remote_projection::project_raw(Integration::Jira, Op::Body, &raw)
                .map_err(|error| read_failure(&error.to_string()))?;
        Ok(RemoteIssue {
            updated: timestamp(&payload),
            // The projection deliberately emits no trailing newline; the port
            // requires exactly one.
            body: port_body(&projected),
        })
    }
}

/// A read never produces `Terminal`: every read failure carries the `Read`
/// operation, whose `Transport` outcome the port derives as `Retryable`.
fn read_failure(detail: &str) -> JiraFailure {
    JiraFailure::wire(Outcome::Transport, Operation::Read, detail.to_owned())
}

/// A 2xx JSON body, surfacing the structured discriminant so a port op reads
/// the granular code before the collapse to `TrackerError`.
pub(crate) fn json_body_op(
    received: &Received,
    operation: Operation,
    detail: &str,
) -> Result<Value, JiraFailure> {
    if !(200..300).contains(&received.status) {
        return Err(JiraFailure::wire(
            Outcome::Status(received.status),
            operation,
            detail.to_owned(),
        ));
    }
    serde_json::from_str(&received.body).map_err(|_| {
        JiraFailure::wire(Outcome::NonJsonBody, operation, detail.to_owned())
    })
}

/// A populated stamp is held verbatim, including Jira's colon-less `+0000`
/// offset. A blank, absent or `null` one is `NotReported`, never
/// `Reported("")`.
fn timestamp(payload: &Value) -> RemoteTimestamp {
    payload
        .pointer("/fields/updated")
        .and_then(Value::as_str)
        .filter(|stamp| !stamp.is_empty())
        .map_or(RemoteTimestamp::NotReported, |stamp| {
            RemoteTimestamp::Reported(stamp.to_owned())
        })
}

/// The `{fields: {summary, description}}` an update sends, shared by `update`
/// and `validate_update` so the previewed payload is the one that would be
/// pushed.
fn update_fields(
    title: &str,
    body: &str,
) -> Result<Value, crate::adf::AdfError> {
    let description = markdown_to_document(body, None)?;
    Ok(json!({"fields": {"summary": title, "description": description}}))
}

impl RemoteTracker for JiraClient {
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        let issue_type = if kind.is_empty() {
            crate::mutation::IssueType::Default
        } else {
            crate::mutation::IssueType::Name(kind)
        };
        let custom = serde_json::Map::new();
        let fields = crate::mutation::CreateFields {
            summary: title,
            body,
            issue_type,
            project: None,
            assignee: None,
            reporter: None,
            priority: None,
            labels: &[],
            components: &[],
            parent: None,
            custom: &custom,
        };
        self.create_op(&fields).map_err(TrackerError::from)
    }

    fn update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), TrackerError> {
        let custom = serde_json::Map::new();
        let edit = crate::mutation::UpdateFields {
            summary: Some(title),
            body: Some(body),
            priority: None,
            assignee: None,
            reporter: None,
            parent: None,
            labels: None,
            add_labels: &[],
            remove_labels: &[],
            components: None,
            add_components: &[],
            remove_components: &[],
            custom: &custom,
            no_notify: false,
        };
        self.update_op(id, &edit).map_err(TrackerError::from)
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
        // The request is a set: duplicates are ignored.
        let mut seen = BTreeSet::new();
        let requested: Vec<&ExternalId> = ids
            .iter()
            .filter(|id| seen.insert(id.as_str().to_owned()))
            .collect();
        // An empty request makes no remote call: composing `key in ()` would
        // produce malformed JQL and fail the whole sync.
        if requested.is_empty() {
            return Ok(outcome);
        }
        for id in &requested {
            tracker_support::identifier_is_safe(id.as_str()).map_err(
                |refusal| {
                    TrackerError::from(JiraFailure::UnsafeQueryId {
                        identifier: id.as_str().to_owned(),
                        reason: refusal.to_string(),
                    })
                },
            )?;
        }

        let deadline = self.transport.deadline();
        for chunk in requested.chunks(CHUNK) {
            match self.fetch_chunk(chunk, &deadline) {
                Ok(found) => {
                    for id in chunk {
                        let stamp = found
                            .iter()
                            .find(|(key, _)| key == id.as_str())
                            .map(|(_, stamp)| stamp.clone());
                        match stamp {
                            Some(stamp) => {
                                outcome.found.push(((*id).clone(), stamp));
                            }
                            None => outcome.absent.push((*id).clone()),
                        }
                    }
                }
                Err(reason) => {
                    tracing::warn!(
                        reason = %reason,
                        keys = chunk.len(),
                        "jira fetch_all: chunk unaccounted for"
                    );
                    outcome
                        .indeterminate
                        .extend(chunk.iter().map(|id| (*id).clone()));
                }
            }
        }
        Ok(outcome)
    }

    fn resolve_scope(
        &self,
        scope: &SearchScope,
    ) -> Result<SearchScope, ScopeError> {
        if scope.project.is_none() && !scope.all_projects {
            return Err(ScopeError {
                detail: "E_JQL_NO_PROJECT: specify a project or all_projects, \
                         or run --push-only to push without discovery"
                    .to_owned(),
            });
        }
        Ok(scope.clone())
    }

    fn search(&self, scope: &SearchScope) -> Result<Discovery, TrackerError> {
        self.discover(scope)
    }

    fn preview_create(
        &self,
        kind: &str,
    ) -> Result<CreatePreview, TrackerError> {
        let project = self.resolve_project()?;
        let issue_type = if kind.is_empty() {
            FieldResolution::Unset
        } else {
            FieldResolution::Resolved(kind.to_owned())
        };
        Ok(CreatePreview {
            project,
            issue_type,
        })
    }

    fn validate_update(
        &self,
        _id: &ExternalId,
        title: &str,
        body: &str,
    ) -> ValidationOutcome {
        let mut reasons = Vec::new();
        if title.trim().is_empty() {
            reasons.push(
                "summary is required but the payload leaves it empty"
                    .to_owned(),
            );
        }
        if let Err(error) = update_fields(title, body) {
            reasons.push(format!("description could not be composed: {error}"));
        }
        if reasons.is_empty() {
            ValidationOutcome::Valid
        } else {
            ValidationOutcome::Rejected { reasons }
        }
    }
}

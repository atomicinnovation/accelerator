//! `impl RemoteTracker for JiraClient`: the four port operations over the
//! bounded transport.

use std::collections::BTreeSet;

use remote_projection::Integration;
use remote_projection::Op;
use reqwest::Method;
use serde_json::json;
use serde_json::Value;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use tracker_support::port_body;
use tracker_support::ClockJitter;
use tracker_support::CredentialContext;
use tracker_support::SystemSleeper;
use tracker_support::TransportConfig;

use crate::adf::markdown_to_document;
use crate::auth::resolve_credentials;
use crate::classify::classify;
use crate::classify::Operation;
use crate::classify::Outcome;
use crate::error::ClientError;
use crate::jql::key_clause;
use crate::jql::AccountResolver;
use crate::jql::FieldResolver;
use crate::jql::FixedResolver;
use crate::path;
use crate::transport::Deadline;
use crate::transport::Received;
use crate::transport::Transport;

/// Jira's own bulk-read bounds.
const CHUNK: usize = 50;
const PAGE_SIZE: u64 = 100;

/// The issue type a `create` uses when the caller supplies no `kind`. The port
/// says the empty string means "the tracker's configured default"; Jira has no
/// per-project default the API exposes, and every tenant ships `Task`.
const DEFAULT_ISSUE_TYPE: &str = "Task";

const SEARCH_PATH: &str = "/rest/api/3/search/jql";
const ISSUE_PATH: &str = "/rest/api/3/issue";

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
    fn read(&self, id: &ExternalId) -> Result<(String, Value), TrackerError> {
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
        let parsed = json_body(&received, Operation::Read, id.as_str())?;
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
}

/// A read never produces `Terminal`, so every read failure routes through the
/// one classifier arm that cannot.
fn read_failure(detail: &str) -> TrackerError {
    classify(Outcome::Transport, Operation::Read, detail)
}

fn json_body(
    received: &Received,
    operation: Operation,
    detail: &str,
) -> Result<Value, TrackerError> {
    if !(200..300).contains(&received.status) {
        return Err(classify(
            Outcome::Status(received.status),
            operation,
            detail,
        ));
    }
    serde_json::from_str(&received.body)
        .map_err(|_| classify(Outcome::NonJsonBody, operation, detail))
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

fn body_conversion_failure(
    error: &crate::adf::AdfError,
    operation: Operation,
) -> TrackerError {
    classify(Outcome::Transport, operation, &error.to_string())
}

impl RemoteTracker for JiraClient {
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        let description =
            markdown_to_document(body, None).map_err(|error| {
                body_conversion_failure(&error, Operation::Create)
            })?;
        let issue_type = if kind.is_empty() {
            DEFAULT_ISSUE_TYPE
        } else {
            kind
        };
        let payload = json!({
            "fields": {
                "project": {"key": self.project},
                "summary": title,
                "issuetype": {"name": issue_type},
                "description": description
            }
        });
        let payload =
            serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_owned());
        let received = self
            .transport
            .send(&Method::POST, ISSUE_PATH, &[], Some(&payload))
            .map_err(|error| {
                classify(
                    Outcome::Transport,
                    Operation::Create,
                    &error.to_string(),
                )
            })?;
        let created = json_body(&received, Operation::Create, title)?;
        let key =
            created.get("key").and_then(Value::as_str).ok_or_else(|| {
                classify(
                    Outcome::NonJsonBody,
                    Operation::Create,
                    "the create response carried no key",
                )
            })?;
        // The issue exists remotely, so an unusable identifier is Terminal: a
        // repeat would duplicate it, and the caller must be told rather than
        // handed a value that would corrupt the work item.
        tracker_support::identifier_is_safe(key).map_err(|refusal| {
            TrackerError::Terminal {
                detail: format!(
                    "jira create: the issue was created as {key:?}, which \
                     cannot be written back — {refusal}"
                ),
            }
        })?;
        Ok(ExternalId::new(key.to_owned()))
    }

    fn update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), TrackerError> {
        let description =
            markdown_to_document(body, None).map_err(|error| {
                body_conversion_failure(&error, Operation::Update)
            })?;
        let path = Self::issue_path(id.as_str(), "").map_err(|error| {
            classify(Outcome::Transport, Operation::Update, &error.to_string())
        })?;
        let payload = json!({
            "fields": {"summary": title, "description": description}
        });
        let payload =
            serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_owned());
        let received = self
            .transport
            .send(&Method::PUT, &path, &[], Some(&payload))
            .map_err(|error| {
                classify(
                    Outcome::Transport,
                    Operation::Update,
                    &error.to_string(),
                )
            })?;
        if (200..300).contains(&received.status) {
            return Ok(());
        }
        Err(classify(
            Outcome::Status(received.status),
            Operation::Update,
            id.as_str(),
        ))
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
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
                |refusal| TrackerError::Retryable {
                    detail: format!(
                        "jira fetch_all: {:?} cannot be embedded in a query \
                         — {refusal}",
                        id.as_str()
                    ),
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
}

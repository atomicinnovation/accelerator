//! The rich `create` and `update` payload assembly.
//!
//! Every principal is already resolved to an accountId and every `--custom`
//! entry to `{field_id: value}` before it reaches here — the client assembles
//! the REST payload but performs no cache lookups, so the wire-failure funnel
//! ([`JiraFailure`]) stays the sole error surface and the resolution errors
//! (bad field, missing site cache, bad principal) are the binary's to map.

use reqwest::Method;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use tracker::ExternalId;

use crate::classify::Operation;
use crate::classify::Outcome;
use crate::client::JiraClient;
use crate::failure::JiraFailure;

/// The issue type a create sets: by name, by numeric id (which wins over a
/// name), or the tracker's default when the caller names neither.
pub enum IssueType<'a> {
    Name(&'a str),
    Id(&'a str),
    Default,
}

/// The resolved fields a create sends. Principals are accountIds and custom
/// fields are `{field_id: value}` already; `project` `None` uses the client's
/// configured project.
pub struct CreateFields<'a> {
    pub summary: &'a str,
    pub body: &'a str,
    pub issue_type: IssueType<'a>,
    pub project: Option<&'a str>,
    pub assignee: Option<&'a str>,
    pub reporter: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub labels: &'a [String],
    pub components: &'a [String],
    pub parent: Option<&'a str>,
    pub custom: &'a Map<String, Value>,
}

/// A field that can be set to a value or explicitly cleared (`assignee`
/// unassigns, `parent` detaches).
pub enum FieldEdit<'a> {
    Set(&'a str),
    Clear,
}

/// The resolved edits an update sends.
///
/// A `None` field is left unchanged; the `labels`/`components` replace-all and
/// the `add_*`/`remove_*` incremental channels are mutually exclusive per field
/// (the binary enforces that).
pub struct UpdateFields<'a> {
    pub summary: Option<&'a str>,
    pub body: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub assignee: Option<FieldEdit<'a>>,
    pub reporter: Option<&'a str>,
    pub parent: Option<FieldEdit<'a>>,
    pub labels: Option<&'a [String]>,
    pub add_labels: &'a [String],
    pub remove_labels: &'a [String],
    pub components: Option<&'a [String]>,
    pub add_components: &'a [String],
    pub remove_components: &'a [String],
    pub custom: &'a Map<String, Value>,
    pub no_notify: bool,
}

const ISSUE_PATH: &str = "/rest/api/3/issue";

impl JiraClient {
    /// `create`, assembling the full field set, surfacing the structured
    /// discriminant. The port `create` delegates here with a minimal field set.
    ///
    /// # Errors
    ///
    /// [`JiraFailure`] carrying the wire outcome or the post-create
    /// unwritable-identifier case.
    pub fn create_op(
        &self,
        fields: &CreateFields<'_>,
    ) -> Result<ExternalId, JiraFailure> {
        let description = crate::adf::markdown_to_document(fields.body, None)
            .map_err(|error| {
            JiraFailure::wire(
                Outcome::Transport,
                Operation::Create,
                error.to_string(),
            )
        })?;
        let mut payload = Map::new();
        payload.insert(
            "project".to_owned(),
            json!({
                "key": fields.project.unwrap_or_else(|| self.project())
            }),
        );
        payload.insert("summary".to_owned(), json!(fields.summary));
        payload.insert(
            "issuetype".to_owned(),
            issue_type_json(&fields.issue_type),
        );
        payload.insert("description".to_owned(), description);
        if let Some(assignee) = fields.assignee {
            payload.insert(
                "assignee".to_owned(),
                json!({ "accountId": assignee }),
            );
        }
        if let Some(reporter) = fields.reporter {
            payload.insert(
                "reporter".to_owned(),
                json!({ "accountId": reporter }),
            );
        }
        if let Some(priority) = fields.priority {
            payload.insert("priority".to_owned(), json!({ "name": priority }));
        }
        if !fields.labels.is_empty() {
            payload.insert("labels".to_owned(), json!(fields.labels));
        }
        if !fields.components.is_empty() {
            payload.insert(
                "components".to_owned(),
                components_json(fields.components),
            );
        }
        if let Some(parent) = fields.parent {
            payload.insert("parent".to_owned(), json!({ "key": parent }));
        }
        for (id, value) in fields.custom {
            payload.insert(id.clone(), value.clone());
        }
        let body = json!({ "fields": Value::Object(payload) });
        let received = self
            .transport()
            .send(&Method::POST, ISSUE_PATH, &[], Some(&serialise(&body)))
            .map_err(|error| {
                JiraFailure::wire(
                    Outcome::Transport,
                    Operation::Create,
                    error.to_string(),
                )
            })?;
        let created = crate::client::json_body_op(
            &received,
            Operation::Create,
            fields.summary,
        )?;
        let key =
            created.get("key").and_then(Value::as_str).ok_or_else(|| {
                JiraFailure::wire(
                    Outcome::NonJsonBody,
                    Operation::Create,
                    "the create response carried no key".to_owned(),
                )
            })?;
        // The issue exists remotely, so an unusable identifier is Terminal: a
        // repeat would duplicate it, and the caller must be told rather than
        // handed a value that would corrupt the work item.
        tracker_support::identifier_is_safe(key).map_err(|refusal| {
            JiraFailure::UnwritableIdentifier {
                identifier: key.to_owned(),
                reason: refusal.to_string(),
            }
        })?;
        Ok(ExternalId::new(key.to_owned()))
    }

    /// `update`, assembling the `fields` (set) and `update` (incremental)
    /// channels, surfacing the structured discriminant. The port `update`
    /// delegates here with a summary-and-body edit.
    ///
    /// # Errors
    ///
    /// [`JiraFailure`] for a body that will not convert, a refused identifier
    /// or a wire failure.
    pub fn update_op(
        &self,
        id: &ExternalId,
        edit: &UpdateFields<'_>,
    ) -> Result<(), JiraFailure> {
        let body = update_payload(edit).map_err(|error| {
            JiraFailure::wire(
                Outcome::Transport,
                Operation::Update,
                error.to_string(),
            )
        })?;
        let path = Self::issue_path(id.as_str(), "").map_err(|error| {
            JiraFailure::wire(
                Outcome::Transport,
                Operation::Update,
                error.to_string(),
            )
        })?;
        let query: Vec<(&str, &str)> = if edit.no_notify {
            vec![("notifyUsers", "false")]
        } else {
            Vec::new()
        };
        let received = self
            .transport()
            .send(&Method::PUT, &path, &query, Some(&serialise(&body)))
            .map_err(|error| {
                JiraFailure::wire(
                    Outcome::Transport,
                    Operation::Update,
                    error.to_string(),
                )
            })?;
        if (200..300).contains(&received.status) {
            return Ok(());
        }
        Err(JiraFailure::wire(
            Outcome::Status(received.status),
            Operation::Update,
            id.as_str().to_owned(),
        ))
    }
}

fn update_payload(
    edit: &UpdateFields<'_>,
) -> Result<Value, crate::adf::AdfError> {
    let mut fields = Map::new();
    if let Some(summary) = edit.summary {
        fields.insert("summary".to_owned(), json!(summary));
    }
    if let Some(body) = edit.body {
        let description = crate::adf::markdown_to_document(body, None)?;
        fields.insert("description".to_owned(), description);
    }
    if let Some(priority) = edit.priority {
        fields.insert("priority".to_owned(), json!({ "name": priority }));
    }
    if let Some(assignee) = &edit.assignee {
        fields.insert("assignee".to_owned(), principal_edit(assignee));
    }
    if let Some(reporter) = edit.reporter {
        fields.insert("reporter".to_owned(), json!({ "accountId": reporter }));
    }
    if let Some(parent) = &edit.parent {
        fields.insert("parent".to_owned(), parent_edit(parent));
    }
    if let Some(labels) = edit.labels {
        fields.insert("labels".to_owned(), json!(labels));
    }
    if let Some(components) = edit.components {
        fields.insert("components".to_owned(), components_json(components));
    }
    for (id, value) in edit.custom {
        fields.insert(id.clone(), value.clone());
    }

    let mut update = Map::new();
    let label_ops =
        add_remove_ops(edit.add_labels, edit.remove_labels, |value| {
            json!(value)
        });
    if !label_ops.is_empty() {
        update.insert("labels".to_owned(), Value::Array(label_ops));
    }
    let component_ops = add_remove_ops(
        edit.add_components,
        edit.remove_components,
        |value| json!({ "name": value }),
    );
    if !component_ops.is_empty() {
        update.insert("components".to_owned(), Value::Array(component_ops));
    }

    let mut payload = Map::new();
    if !fields.is_empty() {
        payload.insert("fields".to_owned(), Value::Object(fields));
    }
    if !update.is_empty() {
        payload.insert("update".to_owned(), Value::Object(update));
    }
    Ok(Value::Object(payload))
}

fn issue_type_json(issue_type: &IssueType<'_>) -> Value {
    match issue_type {
        IssueType::Name(name) => json!({ "name": name }),
        IssueType::Id(id) => json!({ "id": id }),
        IssueType::Default => json!({ "name": "Task" }),
    }
}

fn components_json(components: &[String]) -> Value {
    Value::Array(
        components
            .iter()
            .map(|name| json!({ "name": name }))
            .collect(),
    )
}

fn principal_edit(edit: &FieldEdit<'_>) -> Value {
    match edit {
        FieldEdit::Set(account_id) => json!({ "accountId": account_id }),
        FieldEdit::Clear => json!({ "accountId": Value::Null }),
    }
}

fn parent_edit(edit: &FieldEdit<'_>) -> Value {
    match edit {
        FieldEdit::Set(key) => json!({ "key": key }),
        FieldEdit::Clear => Value::Null,
    }
}

fn add_remove_ops(
    add: &[String],
    remove: &[String],
    shape: impl Fn(&str) -> Value,
) -> Vec<Value> {
    let mut ops = Vec::with_capacity(add.len() + remove.len());
    ops.extend(add.iter().map(|value| json!({ "add": shape(value) })));
    ops.extend(remove.iter().map(|value| json!({ "remove": shape(value) })));
    ops
}

fn serialise(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned())
}

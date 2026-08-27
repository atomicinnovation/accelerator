//! ADF-field rendering for `show` and `search`.
//!
//! Jira returns rich fields (`description`, `environment`, comment bodies) as
//! ADF documents. When rendering is on — the default for `show`, opt-in for
//! `search` — each is replaced in place by its Markdown, reproducing the
//! retiring `jira-render-adf-fields.sh`. A field that is not an ADF document is
//! left untouched, so a partial or already-rendered response passes through.

use jira_client::document_to_markdown;
use serde_json::Value;

/// The rich text fields rendered in place on an issue's `fields` object.
const RICH_FIELDS: &[&str] = &["description", "environment"];

/// Renders every ADF field on one issue's `fields` object to Markdown.
pub fn render_issue(issue: &mut Value) {
    for field in RICH_FIELDS {
        if let Some(value) = issue.pointer_mut(&format!("/fields/{field}")) {
            render_in_place(value);
        }
    }
    if let Some(comments) = issue
        .pointer_mut("/fields/comment/comments")
        .and_then(Value::as_array_mut)
    {
        for comment in comments {
            if let Some(body) = comment.get_mut("body") {
                render_in_place(body);
            }
        }
    }
}

/// Renders every issue in a search envelope's `issues` array.
pub fn render_search(envelope: &mut Value) {
    if let Some(issues) = envelope
        .pointer_mut("/issues")
        .and_then(Value::as_array_mut)
    {
        for issue in issues {
            render_issue(issue);
        }
    }
}

fn render_in_place(value: &mut Value) {
    if is_adf_document(value) {
        if let Ok(markdown) = document_to_markdown(value) {
            *value = Value::String(markdown);
        }
    }
}

fn is_adf_document(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some("doc")
}

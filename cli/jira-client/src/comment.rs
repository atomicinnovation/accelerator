//! Comment operations, transcribed from `jira-comment-flow.sh`: `add` (POST),
//! `list` (GET, offset pagination), `edit` (PUT) and `delete` (DELETE).
//!
//! `$EDITOR` and stdin body resolution stay out of the crate — ADR-0045 forbids
//! an interactive surface in a client — so every operation takes an already
//! resolved Markdown body, which the assembler converts to ADF.

use reqwest::Method;
use serde_json::json;
use serde_json::Value;

use crate::adf::markdown_to_document;
use crate::client::JiraClient;
use crate::surface::SurfaceError;
use crate::transport::Received;

/// Comments per page when the caller names no size, and the inclusive bounds
/// the endpoint accepts (`jira-comment-flow.sh:59,255`).
pub const DEFAULT_PAGE_SIZE: u32 = 50;
const MIN_PAGE_SIZE: u32 = 1;
const MAX_PAGE_SIZE: u32 = 100;

/// The page cap, transcribed from `jira-comment-flow.sh:266`.
const MAX_PAGES: usize = 20;

/// A visibility restriction on a comment (`jira-comment-flow.sh:166-168`).
pub enum Visibility {
    Role(String),
    Group(String),
}

impl Visibility {
    fn to_json(&self) -> Value {
        let (kind, value) = match self {
            Self::Role(value) => ("role", value),
            Self::Group(value) => ("group", value),
        };
        json!({ "type": kind, "value": value })
    }
}

/// The accumulated result of a paginated `list`.
#[derive(Debug)]
pub struct CommentPage {
    pub total: u64,
    pub truncated: bool,
    pub comments: Vec<Value>,
}

impl JiraClient {
    /// Adds a comment. `notify` false suppresses watcher notifications, as
    /// `--no-notify` does.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a refused key, a body that will not convert, a
    /// transport failure or a non-2xx status.
    pub fn add_comment(
        &self,
        key: &str,
        body: &str,
        visibility: Option<&Visibility>,
        notify: bool,
    ) -> Result<Value, SurfaceError> {
        let path = Self::issue_path(key, "/comment")?;
        let payload = comment_payload(body, visibility)?;
        let received = self.send_json(
            &Method::POST,
            &path,
            &notify_query(notify),
            &payload,
        )?;
        parse_ok(&received, "comment add")
    }

    /// Edits a comment in place.
    ///
    /// # Errors
    ///
    /// As [`JiraClient::add_comment`], plus a refused comment id.
    pub fn edit_comment(
        &self,
        key: &str,
        comment_id: &str,
        body: &str,
        visibility: Option<&Visibility>,
        notify: bool,
    ) -> Result<Value, SurfaceError> {
        let path = Self::issue_comment_path(key, comment_id)?;
        let payload = comment_payload(body, visibility)?;
        let received = self.send_json(
            &Method::PUT,
            &path,
            &notify_query(notify),
            &payload,
        )?;
        parse_ok(&received, "comment edit")
    }

    /// Deletes a comment.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a refused id, a transport failure or a non-2xx
    /// status.
    pub fn delete_comment(
        &self,
        key: &str,
        comment_id: &str,
        notify: bool,
    ) -> Result<(), SurfaceError> {
        let path = Self::issue_comment_path(key, comment_id)?;
        let received = self.transport().send(
            &Method::DELETE,
            &path,
            &notify_query(notify),
            None,
        )?;
        if is_success(received.status) {
            return Ok(());
        }
        Err(SurfaceError::status(
            "comment delete",
            received.status,
            received.body,
        ))
    }

    /// Lists comments, following `startAt`/`maxResults` offset pagination up to
    /// the 20-page cap. A cap-hit sets `truncated`.
    ///
    /// # Errors
    ///
    /// [`SurfaceError::BadPageSize`] for a size outside `[1, 100]`, and the
    /// request errors above.
    pub fn list_comments(
        &self,
        key: &str,
        page_size: u32,
        first_page_only: bool,
    ) -> Result<CommentPage, SurfaceError> {
        if !(MIN_PAGE_SIZE..=MAX_PAGE_SIZE).contains(&page_size) {
            return Err(SurfaceError::BadPageSize { got: page_size });
        }
        let path = Self::issue_path(key, "/comment")?;

        let mut start_at: u64 = 0;
        let mut comments: Vec<Value> = Vec::new();
        let mut total: u64 = 0;
        let mut page_count = 0;
        let mut truncated = false;

        loop {
            if page_count >= MAX_PAGES {
                truncated = true;
                break;
            }
            let start_text = start_at.to_string();
            let max_text = page_size.to_string();
            let received = self.transport().send(
                &Method::GET,
                &path,
                &[
                    ("startAt", start_text.as_str()),
                    ("maxResults", max_text.as_str()),
                ],
                None,
            )?;
            if !is_success(received.status) {
                return Err(SurfaceError::status(
                    "comment list",
                    received.status,
                    received.body,
                ));
            }
            let page: Value = parse_ok(&received, "comment list")?;
            let page_total = integer_field(&page, "total")?;
            let page_comments = page
                .get("comments")
                .and_then(Value::as_array)
                .cloned()
                .ok_or_else(|| SurfaceError::BadResponse {
                    operation: "comment list",
                    reason: "no comments array in the page".to_owned(),
                })?;
            let returned = page_comments.len() as u64;

            total = page_total;
            comments.extend(page_comments);
            page_count += 1;

            if first_page_only || returned == 0 {
                break;
            }
            start_at += returned;
            if start_at >= page_total {
                break;
            }
        }

        Ok(CommentPage {
            total,
            truncated,
            comments,
        })
    }

    fn send_json(
        &self,
        method: &Method,
        path: &str,
        query: &[(&str, &str)],
        payload: &Value,
    ) -> Result<Received, SurfaceError> {
        let body =
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_owned());
        Ok(self.transport().send(method, path, query, Some(&body))?)
    }
}

fn comment_payload(
    body: &str,
    visibility: Option<&Visibility>,
) -> Result<Value, SurfaceError> {
    // The bash defaults an empty body to `{}` rather than converting.
    let document = if body.is_empty() {
        json!({})
    } else {
        markdown_to_document(body, None)?
    };
    let mut payload = json!({ "body": document });
    if let Some(visibility) = visibility {
        payload["visibility"] = visibility.to_json();
    }
    Ok(payload)
}

fn notify_query(notify: bool) -> Vec<(&'static str, &'static str)> {
    if notify {
        Vec::new()
    } else {
        vec![("notifyUsers", "false")]
    }
}

const fn is_success(status: u16) -> bool {
    status >= 200 && status < 300
}

fn parse_ok(
    received: &Received,
    operation: &'static str,
) -> Result<Value, SurfaceError> {
    if !is_success(received.status) {
        return Err(SurfaceError::status(
            operation,
            received.status,
            received.body.clone(),
        ));
    }
    if received.body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&received.body).map_err(|error| {
        SurfaceError::BadResponse {
            operation,
            reason: format!("the response was not JSON: {error}"),
        }
    })
}

fn integer_field(page: &Value, name: &str) -> Result<u64, SurfaceError> {
    page.get(name).and_then(Value::as_u64).ok_or_else(|| {
        SurfaceError::BadResponse {
            operation: "comment list",
            reason: format!(".{name} is not an integer"),
        }
    })
}

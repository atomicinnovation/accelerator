//! The read-side projections the `search` and `show` subcommands render.
//!
//! The port `search`/`show` reshape a response into the sync contract — stamps
//! and a projected body. These keep Jira's own wire envelope shape: `search`
//! merges its pages into `{issues}` and marks the envelope `truncated` with a
//! resume `nextPageToken` when the walk stops short; `show` returns the raw
//! issue the binary renders ADF fields over. The composed JQL is exposed
//! separately so the binary can print the audit line before the request.

use reqwest::Method;
use serde_json::json;
use serde_json::Value;
use tracker::Completeness;

use crate::client::JiraClient;
use crate::error::ClientError;
use crate::jql::compose;
use crate::jql::Search;
use crate::surface::SurfaceError;

const SEARCH_PATH: &str = "/rest/api/3/search/jql";

/// A search's merged pages and whether the walk saw everything within the cap.
///
/// The `envelope` is `{issues: [...]}`, plus `truncated: true` and the resume
/// `nextPageToken` when `completeness` is not [`Completeness::Complete`] — so a
/// consumer can detect the truncation and, on a cap-hit, resume with
/// `--page-token`.
#[derive(Debug)]
pub struct SearchPage {
    pub envelope: Value,
    pub completeness: Completeness,
}

impl JiraClient {
    /// The JQL the search surface composes, for the `INFO: composed JQL` audit
    /// line the repointed body prints before the request.
    ///
    /// # Errors
    ///
    /// [`ClientError::BadJql`] when neither a project nor `all_projects` is
    /// given, or when a value cannot be quoted.
    pub fn compose_search_jql(
        &self,
        search: &Search,
    ) -> Result<String, ClientError> {
        compose(search, self.accounts(), self.fields())
    }

    /// Pages a search internally up to the discovery `max_pages` cap, merging
    /// every page's issues into one envelope.
    ///
    /// `page_token` resumes a prior walk from its cursor; `max_results` is the
    /// per-page size. A clean finish is [`Completeness::Complete`]; reaching
    /// the cap is [`Completeness::CapHit`] and an expired deadline
    /// [`Completeness::Transient`] — each carrying the resume cursor in the
    /// envelope. A wire failure, non-2xx status or non-JSON body is an error
    /// the search flow propagates, not a degraded page.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for an uncomposable scope, a transport failure, a
    /// non-2xx status, or a non-JSON body.
    pub fn search_detailed(
        &self,
        search: &Search,
        fields: &[String],
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<SearchPage, SurfaceError> {
        let jql = compose(search, self.accounts(), self.fields())?;
        let cap = self.transport().config().discovery_max_pages;
        let deadline = self.transport().deadline();
        let mut issues: Vec<Value> = Vec::new();
        let mut cursor: Option<String> = page_token.map(str::to_owned);
        let mut completeness = Completeness::Complete;

        let mut page = 0usize;
        loop {
            page += 1;
            if deadline.expired() {
                completeness = Completeness::Transient;
                break;
            }
            let mut body = json!({
                "jql": jql,
                "fields": fields,
                "fieldsByKeys": false,
                "maxResults": max_results,
            });
            if let Some(token) = &cursor {
                body["nextPageToken"] = json!(token);
            }
            let payload = serde_json::to_string(&body)
                .unwrap_or_else(|_| "{}".to_owned());
            let received = self.transport().send(
                &Method::POST,
                SEARCH_PATH,
                &[],
                Some(&payload),
            )?;
            let page_value = parse_ok(received, "search")?;
            if let Some(array) =
                page_value.get("issues").and_then(Value::as_array)
            {
                issues.extend(array.iter().cloned());
            }
            cursor = page_value
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if cursor.is_none() {
                break;
            }
            if cap.reached(page) {
                completeness = Completeness::CapHit;
                break;
            }
        }

        let mut envelope = json!({ "issues": issues });
        if !completeness.is_complete() {
            envelope["truncated"] = json!(true);
            if let Some(token) = cursor {
                envelope["nextPageToken"] = json!(token);
            }
        }
        Ok(SearchPage {
            envelope,
            completeness,
        })
    }

    /// Fetches one issue's full detail for `show`, returning Jira's raw issue
    /// JSON. The binary renders the ADF fields and slices comments over it — the
    /// projection the port `show` applies would lose the description ADF the
    /// `--render-adf` flag exists to render.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a refused identifier, a transport failure, a non-2xx
    /// status, or a non-JSON body.
    pub fn show_detailed(
        &self,
        key: &str,
        fields: &str,
        expand: &str,
    ) -> Result<Value, SurfaceError> {
        let path = Self::issue_path(key, "")?;
        let received = self.transport().send(
            &Method::GET,
            &path,
            &[("fields", fields), ("expand", expand)],
            None,
        )?;
        parse_ok(received, "show")
    }
}

/// A 2xx JSON body, or the surface error the non-2xx / non-JSON case maps to.
fn parse_ok(
    received: crate::transport::Received,
    operation: &'static str,
) -> Result<Value, SurfaceError> {
    if received.status < 200 || received.status >= 300 {
        return Err(SurfaceError::status(
            operation,
            received.status,
            received.body,
        ));
    }
    serde_json::from_str(&received.body).map_err(|error| {
        SurfaceError::BadResponse {
            operation,
            reason: format!("the response was not JSON: {error}"),
        }
    })
}

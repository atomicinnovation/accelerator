//! The read-side projections the `search` and `show` subcommands render.
//!
//! The port `search`/`show` reshape a response into the sync contract — stamps
//! and a projected body. These keep Jira's own wire envelope the retiring bash
//! flows emitted verbatim: `search` echoes `{issues, nextPageToken}`, `show`
//! returns the raw issue the binary renders ADF fields over. The composed JQL
//! is exposed separately so the binary can print the audit line before the
//! request, exactly as the bash flow did.

use reqwest::Method;
use serde_json::json;
use serde_json::Value;

use crate::client::JiraClient;
use crate::error::ClientError;
use crate::jql::compose;
use crate::jql::Search;
use crate::surface::SurfaceError;

const SEARCH_PATH: &str = "/rest/api/3/search/jql";

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

    /// Runs one search page and returns Jira's verbatim response envelope
    /// (`{issues, nextPageToken}`), the shape the bash search flow emitted.
    ///
    /// A single page: `page_token` follows the cursor the caller carries, so
    /// pagination stays the operator's, not the client's.
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
    ) -> Result<Value, SurfaceError> {
        let jql = compose(search, self.accounts(), self.fields())?;
        let mut body = json!({
            "jql": jql,
            "fields": fields,
            "fieldsByKeys": false,
            "maxResults": max_results,
        });
        if let Some(token) = page_token {
            body["nextPageToken"] = json!(token);
        }
        let payload =
            serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_owned());
        let received = self.transport().send(
            &Method::POST,
            SEARCH_PATH,
            &[],
            Some(&payload),
        )?;
        parse_ok(received, "search")
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

//! Loads a retired bash mock's scenario JSON and installs it on a
//! [`MockServer`].
//!
//! A scenario is `{expectations: [{method, path, consume?, response: {status,
//! headers, body}, expect_body_contains?, capture_body?}]}` — a near one-to-one
//! map onto `http-test-support`'s routes. Several expectations sharing a
//! `(method, path)` key (the single-endpoint GraphQL case, marked `consume`)
//! become a `Route::Sequence`; a lone expectation becomes a `Route::Headers`.
//!
//! The schema is a single unified superset across providers — the jira-only
//! `capture_url`/header fields and the linear-only `consume` flag are all
//! optional — so one loader serves both binaries rather than a dialect branch.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

use http_test_support::{MockServer, RequestKey, Route};
use serde::Deserialize;

/// A mock response.
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: String,
}

/// One expectation: a request to match and the response to return.
#[derive(Debug, Clone, Deserialize)]
pub struct Expectation {
    pub method: String,
    pub path: String,
    /// Part of an ordered sequence for its `(method, path)` key — the
    /// single-endpoint case where two POSTs cannot be told apart by key.
    #[serde(default)]
    pub consume: bool,
    pub response: Response,
    /// A substring a test asserts the request body carries.
    #[serde(default)]
    pub expect_body_contains: Option<String>,
    #[serde(default)]
    pub capture_body: bool,
    /// A substring a test asserts the request URL (query) carries — jira only.
    #[serde(default)]
    pub capture_url: Option<String>,
}

/// A parsed scenario file.
#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    pub expectations: Vec<Expectation>,
}

impl Scenario {
    /// Parses a scenario from JSON text.
    ///
    /// # Errors
    ///
    /// [`serde_json::Error`] if the text is not a valid scenario.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// Loads a scenario from a file.
    ///
    /// # Errors
    ///
    /// An [`std::io::Error`] rendered into a `String` if the file is
    /// unreadable, or the parse error if it is not a valid scenario.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Self::from_json(&text).map_err(|error| {
            format!("{}: not a valid scenario: {error}", path.display())
        })
    }

    /// Registers every expectation on `server`, grouping a `(method, path)`
    /// key's expectations into a `Route::Sequence` in declaration order.
    pub fn install(&self, server: &MockServer) {
        let mut order: Vec<RequestKey> = Vec::new();
        let mut grouped: HashMap<RequestKey, Vec<Route>> = HashMap::new();
        for expectation in &self.expectations {
            let key = RequestKey::new(&expectation.method, &expectation.path);
            let route = route_for(&expectation.response);
            if !grouped.contains_key(&key) {
                order.push(key.clone());
            }
            grouped.entry(key).or_default().push(route);
        }
        for key in order {
            let mut routes = grouped.remove(&key).unwrap_or_default();
            let route = if routes.len() == 1 {
                routes.pop().unwrap_or(Route::Status(500))
            } else {
                Route::Sequence(routes)
            };
            server.route(key, route);
        }
    }

    /// The body-substring assertions, as `(RequestKey, substring)`.
    #[must_use]
    pub fn body_expectations(&self) -> Vec<(RequestKey, String)> {
        self.expectations
            .iter()
            .filter_map(|expectation| {
                let key =
                    RequestKey::new(&expectation.method, &expectation.path);
                expectation
                    .expect_body_contains
                    .clone()
                    .map(|substring| (key, substring))
            })
            .collect()
    }
}

fn route_for(response: &Response) -> Route {
    let headers: Vec<(String, String)> = response
        .headers
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    Route::Headers {
        status: response.status,
        headers,
        body: response.body.clone(),
    }
}

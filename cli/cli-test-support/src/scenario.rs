//! Loads a retired mock's scenario JSON and installs it on a
//! [`MockServer`].
//!
//! A scenario is `{expectations?: [{method, path, consume?, response: {status,
//! headers, body}, expect_body_contains?, capture_body?}], graphql_queries?:
//! [{operation, response, expect_body_contains?}]}` — a near one-to-one map
//! onto `http-test-support`'s routes and `graphql-test-support`'s answers.
//! Several HTTP calls sharing a `(method, path)` key (the single-endpoint
//! GraphQL case, marked `consume`), or several queries sharing an operation,
//! become a `Route::Sequence`; a lone one becomes a `Route::Headers`.
//!
//! The schema is a single unified superset across providers — the jira-only
//! `capture_url`/header fields and the linear-only `consume` flag are all
//! optional — so one loader serves both binaries rather than a dialect branch.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;

use graphql_test_support::{MockGraphQLServer, ENDPOINT};
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

/// One HTTP call: a request to match by method and path, and the response to
/// return.
#[derive(Debug, Clone, Deserialize)]
pub struct HttpCallExpectation {
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

impl HttpCallExpectation {
    fn key(&self) -> RequestKey {
        RequestKey::new(&self.method, &self.path)
    }
}

/// One GraphQL query: the operation to answer, and the response to return.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphQLQueryExpectation {
    pub operation: String,
    pub response: Response,
    /// A substring a test asserts the request body carries.
    #[serde(default)]
    pub expect_body_contains: Option<String>,
}

/// A parsed scenario file.
#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    #[serde(default, rename = "expectations")]
    pub http_calls: Vec<HttpCallExpectation>,
    #[serde(default)]
    pub graphql_queries: Vec<GraphQLQueryExpectation>,
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

    /// Registers every HTTP call on `server`, grouping a `(method, path)`
    /// key's calls into a `Route::Sequence` in declaration order.
    ///
    /// # Panics
    ///
    /// If the scenario scripts GraphQL queries, which only a
    /// [`MockGraphQLServer`] can answer.
    pub fn install(&self, server: &MockServer) {
        assert!(
            self.graphql_queries.is_empty(),
            "a scenario scripting GraphQL queries installs on a \
             MockGraphQLServer"
        );
        self.install_http_calls(server);
    }

    /// Answers every GraphQL query on `server`, grouping an operation's
    /// queries into a `Route::Sequence` in declaration order, and registers
    /// every HTTP call on the HTTP mock beneath it.
    ///
    /// # Panics
    ///
    /// If an HTTP call targets the GraphQL endpoint, which it would displace.
    pub fn install_graphql(&self, server: &MockGraphQLServer) {
        let endpoint = RequestKey::post(ENDPOINT);
        assert!(
            self.http_calls.iter().all(|call| call.key() != endpoint),
            "an HTTP call to POST {ENDPOINT} would displace the GraphQL \
             endpoint; script it as a GraphQL query"
        );
        for (operation, route) in sequenced(
            self.graphql_queries
                .iter()
                .map(|query| (query.operation.clone(), &query.response)),
        ) {
            server.answer(&operation, route);
        }
        self.install_http_calls(server.http());
    }

    fn install_http_calls(&self, server: &MockServer) {
        for (key, route) in sequenced(
            self.http_calls
                .iter()
                .map(|call| (call.key(), &call.response)),
        ) {
            server.route(key, route);
        }
    }

    /// The HTTP calls' body-substring assertions, as `(RequestKey,
    /// substring)`.
    #[must_use]
    pub fn body_expectations(&self) -> Vec<(RequestKey, String)> {
        self.http_calls
            .iter()
            .filter_map(|call| {
                call.expect_body_contains
                    .clone()
                    .map(|substring| (call.key(), substring))
            })
            .collect()
    }

    /// The GraphQL queries' body-substring assertions, as `(operation,
    /// substring)`.
    #[must_use]
    pub fn query_body_expectations(&self) -> Vec<(String, String)> {
        self.graphql_queries
            .iter()
            .filter_map(|query| {
                query
                    .expect_body_contains
                    .clone()
                    .map(|substring| (query.operation.clone(), substring))
            })
            .collect()
    }
}

/// Each key's responses as one route, in first-declaration order: a lone
/// response as itself, several as a `Route::Sequence`.
fn sequenced<'a, K: Clone + Eq + Hash>(
    responses: impl Iterator<Item = (K, &'a Response)>,
) -> Vec<(K, Route)> {
    let mut order: Vec<K> = Vec::new();
    let mut grouped: HashMap<K, Vec<Route>> = HashMap::new();
    for (key, response) in responses {
        if !grouped.contains_key(&key) {
            order.push(key.clone());
        }
        grouped.entry(key).or_default().push(route_for(response));
    }
    order
        .into_iter()
        .map(|key| {
            let mut routes = grouped.remove(&key).unwrap_or_default();
            let route = if routes.len() == 1 {
                routes.pop().unwrap_or(Route::Status(500))
            } else {
                Route::Sequence(routes)
            };
            (key, route)
        })
        .collect()
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

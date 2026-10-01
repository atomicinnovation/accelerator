//! A mock GraphQL endpoint for the workspace's client tests: answers,
//! hit counts and request bodies per operation, mounted on an
//! `http-test-support` [`MockHTTPServer`].
//!
//! Every operation shares one path, so a `(method, path)` route cannot tell a
//! client's queries apart; this server reads the operation each request body
//! names and answers it independently, so a multi-query test scripts each
//! query on its own terms whatever order the client sends them in.
//!
//! The server panics on a setup mistake — a poisoned lock — and, when dropped,
//! on any operation it was asked for but given no answer, so a client that
//! sends an unscripted query fails the test rather than hiding behind a
//! status the client may tolerate.
#![allow(clippy::expect_used, clippy::missing_panics_doc)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use http_test_support::{MockHTTPServer, RequestKey, Route, UNMATCHED_STATUS};

/// The path every operation is posted to.
pub const ENDPOINT: &str = "/graphql";

#[derive(Default)]
struct Operations {
    answers: Mutex<HashMap<String, Route>>,
    bodies: Mutex<HashMap<String, Vec<Vec<u8>>>>,
    unanswered: Mutex<Vec<String>>,
}

impl Operations {
    fn respond(&self, body: &[u8]) -> Route {
        let Some(operation) = operation_name(body) else {
            return Route::Status(UNMATCHED_STATUS);
        };
        let earlier_requests = self.record(&operation, body);
        let answer = self
            .answers
            .lock()
            .expect("answers")
            .get(&operation)
            .cloned();
        answer.map_or_else(
            || {
                self.unanswered.lock().expect("unanswered").push(operation);
                Route::Status(UNMATCHED_STATUS)
            },
            |answer| answer.for_hit(earlier_requests),
        )
    }

    fn record(&self, operation: &str, body: &[u8]) -> usize {
        let mut bodies = self.bodies.lock().expect("bodies");
        let received = bodies.entry(operation.to_owned()).or_default();
        received.push(body.to_vec());
        let preceding = received.len() - 1;
        drop(bodies);
        preceding
    }
}

pub struct MockGraphQLServer {
    http: MockHTTPServer,
    operations: Arc<Operations>,
}

impl MockGraphQLServer {
    /// Starts an HTTP mock with the GraphQL endpoint mounted on it.
    #[must_use]
    pub fn start() -> Self {
        let http = MockHTTPServer::start();
        let operations = Arc::new(Operations::default());
        let responder = Arc::clone(&operations);
        http.route(
            RequestKey::post(ENDPOINT),
            Route::ByBody(Arc::new(move |body: &[u8]| responder.respond(body))),
        );
        Self { http, operations }
    }

    /// The base URL for the server (no trailing slash); the endpoint is at
    /// [`ENDPOINT`] beneath it.
    #[must_use]
    pub fn base_url(&self) -> String {
        self.http.base_url()
    }

    /// Answers `operation` with `route`, resolved against how many requests
    /// for that operation came before — so a sequence advances only on the
    /// operation it answers.
    pub fn answer(&self, operation: &str, route: Route) {
        self.operations
            .answers
            .lock()
            .expect("answers")
            .insert(operation.to_owned(), route);
    }

    /// How many requests named `operation`.
    #[must_use]
    pub fn hits(&self, operation: &str) -> usize {
        self.bodies(operation).len()
    }

    /// Every request body naming `operation`, in the order they arrived.
    #[must_use]
    pub fn bodies(&self, operation: &str) -> Vec<Vec<u8>> {
        self.operations
            .bodies
            .lock()
            .expect("bodies")
            .get(operation)
            .cloned()
            .unwrap_or_default()
    }

    /// The body of the most recent request naming `operation`.
    #[must_use]
    pub fn last_body(&self, operation: &str) -> Option<Vec<u8>> {
        self.bodies(operation).pop()
    }

    /// Every operation requested without an answer, in arrival order.
    #[must_use]
    pub fn unanswered(&self) -> Vec<String> {
        self.operations
            .unanswered
            .lock()
            .expect("unanswered")
            .clone()
    }

    /// The HTTP mock the endpoint is mounted on, for what a request carries
    /// whatever its operation — its headers, or the endpoint's total hits.
    #[must_use]
    pub const fn http(&self) -> &MockHTTPServer {
        &self.http
    }
}

impl Drop for MockGraphQLServer {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }
        let unanswered = self.unanswered();
        assert!(
            unanswered.is_empty(),
            "the mock GraphQL server had no answer for these operations: {}",
            unanswered.join(", ")
        );
    }
}

/// The operation a GraphQL request body names: the declared name of a named
/// operation, else the first root field of an anonymous one.
fn operation_name(body: &[u8]) -> Option<String> {
    let request: serde_json::Value = serde_json::from_slice(body).ok()?;
    let document = request.get("query")?.as_str()?.trim_start();
    let declared = ["query", "mutation", "subscription"]
        .iter()
        .find_map(|keyword| document.strip_prefix(keyword))
        .and_then(|rest| leading_name(rest.trim_start()));
    declared.or_else(|| {
        let (_, selection) = document.split_once('{')?;
        leading_name(selection.trim_start())
    })
}

fn leading_name(text: &str) -> Option<String> {
    let name: String = text
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || *character == '_'
        })
        .collect();
    let starts_a_name = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_');
    starts_a_name.then_some(name)
}

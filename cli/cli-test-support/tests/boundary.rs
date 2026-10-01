//! The shared machinery's own boundary tests: a parser or loader bug would
//! otherwise silently weaken every downstream parity, keyword and request
//! assertion in both provider binaries at once.

#![allow(clippy::expect_used)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;

use cli_test_support::{parse_u8_consts, Scenario};
use graphql_test_support::{MockGraphQLServer, ENDPOINT};
use http_test_support::{MockHTTPServer, RequestKey};

/// A raw POST over TCP, returning `(status, body)` — no HTTP client, so the
/// test needs no TLS crypto provider for a plain-http mock.
fn post(server: &MockHTTPServer, path: &str, body: &[u8]) -> (u16, Vec<u8>) {
    let address = server.base_url().replace("http://", "");
    let mut stream = TcpStream::connect(address).expect("connect");
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: mock\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).expect("head");
    stream.write_all(body).expect("body");
    stream.flush().expect("flush");

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).expect("status");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .expect("status code")
        .parse()
        .expect("numeric status");
    let mut length = 0;
    let mut header = String::new();
    loop {
        header.clear();
        if reader.read_line(&mut header).expect("header") == 0
            || header == "\r\n"
            || header == "\n"
        {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut response = vec![0_u8; length];
    if length > 0 {
        reader.read_exact(&mut response).expect("response body");
    }
    (status, response)
}

#[test]
fn the_parser_reads_u8_consts_and_skips_everything_else() {
    let source = "\
//! a doc line, not a const
pub const CLEAN: u8 = 0;
    pub const SEARCH_BAD_FLAG: u8 = 75;
pub const NOT_A_U8: u16 = 9;
pub const MISSING_VALUE: u8 =;
const PRIVATE: u8 = 3;
";
    let parsed = parse_u8_consts(source);

    assert_eq!(
        parsed,
        vec![("CLEAN".to_owned(), 0), ("SEARCH_BAD_FLAG".to_owned(), 75)],
        "only well-formed public u8 consts are read, in file order"
    );
}

#[test]
fn the_loader_maps_a_lone_expectation_to_a_single_route() {
    let scenario = Scenario::from_json(
        r#"{"expectations":[{"method":"post","path":"/graphql",
            "response":{"status":200,"headers":{"Content-Type":"application/json"},
            "body":"{\"ok\":true}"},"expect_body_contains":"query"}]}"#,
    )
    .expect("valid scenario");

    let server = MockHTTPServer::start();
    scenario.install(&server);

    let (status, body) = post(&server, "/graphql", b"{\"query\":\"x\"}");
    assert_eq!(status, 200);
    assert_eq!(body, b"{\"ok\":true}");

    assert_eq!(
        scenario.body_expectations(),
        vec![(RequestKey::post("/graphql"), "query".to_owned())]
    );
}

#[test]
fn the_loader_groups_a_consume_sequence_in_declaration_order() {
    let scenario = Scenario::from_json(
        r#"{"expectations":[
            {"method":"POST","path":"/graphql","consume":true,
             "response":{"status":200,"headers":{},"body":"first"}},
            {"method":"POST","path":"/graphql","consume":true,
             "response":{"status":200,"headers":{},"body":"second"}}
        ]}"#,
    )
    .expect("valid scenario");

    let server = MockHTTPServer::start();
    scenario.install(&server);

    assert_eq!(post(&server, "/graphql", b"a").1, b"first");
    assert_eq!(
        post(&server, "/graphql", b"b").1,
        b"second",
        "the second hit gets the second response, not a repeat of the first"
    );
}

fn query(server: &MockGraphQLServer, operation: &str) -> (u16, Vec<u8>) {
    post(
        server.http(),
        ENDPOINT,
        format!("{{\"query\":\"query {operation} {{ id }}\"}}").as_bytes(),
    )
}

#[test]
fn a_scenario_answers_each_graphql_query_by_its_operation() {
    let scenario = Scenario::from_json(
        r#"{"graphql_queries": [
            {"operation": "TeamStates",
             "response": {"status": 200, "body": "states"},
             "expect_body_contains": "workflowStates"},
            {"operation": "TeamLabels",
             "response": {"status": 200, "body": "labels-1"}},
            {"operation": "TeamLabels",
             "response": {"status": 200, "body": "labels-2"}}
        ]}"#,
    )
    .expect("scenario");
    let server = MockGraphQLServer::start();
    scenario.install_graphql(&server);

    assert_eq!(query(&server, "TeamLabels"), (200, b"labels-1".to_vec()));
    assert_eq!(query(&server, "TeamStates"), (200, b"states".to_vec()));
    assert_eq!(query(&server, "TeamLabels"), (200, b"labels-2".to_vec()));
    assert_eq!(
        scenario.query_body_expectations(),
        vec![("TeamStates".to_owned(), "workflowStates".to_owned())]
    );
    assert!(scenario.body_expectations().is_empty());
}

#[test]
fn a_graphql_scenario_installs_its_http_calls_beside_the_endpoint() {
    let scenario = Scenario::from_json(
        r#"{"expectations": [
                {"method": "PUT", "path": "/upload",
                 "response": {"status": 200, "body": "stored"}}],
            "graphql_queries": [
                {"operation": "Viewer",
                 "response": {"status": 200, "body": "viewer"}}]}"#,
    )
    .expect("scenario");
    let server = MockGraphQLServer::start();
    scenario.install_graphql(&server);

    assert_eq!(query(&server, "Viewer"), (200, b"viewer".to_vec()));
    assert_eq!(server.http().hits(&RequestKey::put("/upload")), 0);
}

#[test]
fn a_graphql_query_expectation_refuses_http_call_fields() {
    let parsed = Scenario::from_json(
        r#"{"graphql_queries": [
            {"operation": "Viewer", "method": "POST", "path": "/graphql",
             "response": {"status": 200, "body": "viewer"}}]}"#,
    );

    assert!(parsed.is_err());
}

#[test]
#[should_panic(expected = "MockGraphQLServer")]
fn a_scenario_with_graphql_queries_refuses_a_plain_http_server() {
    let scenario = Scenario::from_json(
        r#"{"graphql_queries": [{"operation": "Viewer",
            "response": {"status": 200, "body": "viewer"}}]}"#,
    )
    .expect("scenario");

    scenario.install(&MockHTTPServer::start());
}

#[test]
#[should_panic(expected = "/graphql")]
fn a_graphql_scenario_refuses_an_http_call_to_the_endpoint() {
    let scenario = Scenario::from_json(
        r#"{"expectations": [{"method": "POST", "path": "/graphql",
            "response": {"status": 200, "body": "plain"}}]}"#,
    )
    .expect("scenario");

    scenario.install_graphql(&MockGraphQLServer::start());
}

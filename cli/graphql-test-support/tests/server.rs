//! The behaviours these tests pin down: per-operation answers and sequences,
//! per-operation request recording, and the loud failure of an operation no
//! answer was registered for.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;

use graphql_test_support::{MockGraphQLServer, ENDPOINT};
use http_test_support::{RequestKey, Route, UNMATCHED_STATUS};

struct Response {
    status: u16,
    body: Vec<u8>,
}

fn post(server: &MockGraphQLServer, body: &[u8]) -> Response {
    let address = server.base_url().replace("http://", "");
    let mut stream = TcpStream::connect(address).expect("connect");
    let mut request = format!(
        "POST {ENDPOINT} HTTP/1.1\r\nHost: mock\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    stream.write_all(&request).expect("write request");

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).expect("status line");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("a numeric status");
    let mut content_length = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).expect("header");
        if header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body).expect("body");
    Response { status, body }
}

fn query(server: &MockGraphQLServer, document: &str) -> Response {
    let escaped = document.replace('\\', "\\\\").replace('"', "\\\"");
    post(
        server,
        format!("{{\"query\":\"{escaped}\",\"variables\":{{}}}}").as_bytes(),
    )
}

fn json(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

#[test]
fn operations_are_served_their_own_sequences_whatever_the_order() {
    let server = MockGraphQLServer::start();
    server.answer(
        "TeamStates",
        Route::Sequence(vec![json("\"states-1\""), json("\"states-2\"")]),
    );
    server.answer(
        "TeamLabels",
        Route::Sequence(vec![json("\"labels-1\""), json("\"labels-2\"")]),
    );

    let labels_1 =
        query(&server, "query TeamLabels { issueLabels { nodes { id } } }");
    let states_1 = query(
        &server,
        "query TeamStates { workflowStates { nodes { id } } }",
    );
    let labels_2 =
        query(&server, "query TeamLabels { issueLabels { nodes { id } } }");
    let states_2 = query(
        &server,
        "query TeamStates($ids: [ID!]) { workflowStates { nodes { id } } }",
    );

    assert_eq!(labels_1.body, b"\"labels-1\"");
    assert_eq!(labels_2.body, b"\"labels-2\"");
    assert_eq!(states_1.body, b"\"states-1\"");
    assert_eq!(states_2.body, b"\"states-2\"");
}

#[test]
fn two_operations_sharing_a_root_field_get_separate_answers() {
    let server = MockGraphQLServer::start();
    server.answer("TeamLabels", json("\"team\""));
    server.answer("WorkspaceLabels", json("\"workspace\""));

    let workspace = query(
        &server,
        "query WorkspaceLabels { issueLabels(filter: { team: { null: true } }) \
         { nodes { id } } }",
    );
    let team =
        query(&server, "query TeamLabels { issueLabels { nodes { id } } }");

    assert_eq!(workspace.body, b"\"workspace\"");
    assert_eq!(team.body, b"\"team\"");
}

#[test]
fn an_anonymous_query_is_named_for_its_first_root_field() {
    let server = MockGraphQLServer::start();
    server.answer("viewer", json("\"viewer\""));
    server.answer("teams", json("\"teams\""));

    let bare = query(&server, "{ viewer { id } }");
    let keyword = query(
        &server,
        "query($first: Int) { teams(first: $first) { nodes { id } } }",
    );

    assert_eq!(bare.body, b"\"viewer\"");
    assert_eq!(keyword.body, b"\"teams\"");
}

#[test]
fn a_mutation_is_named_like_a_query() {
    let server = MockGraphQLServer::start();
    server.answer("CreateIssue", json("\"created\""));

    let created = query(
        &server,
        "mutation CreateIssue { issueCreate(input: {}) { success } }",
    );

    assert_eq!(created.body, b"\"created\"");
}

#[test]
fn requests_are_recorded_per_operation_and_at_the_endpoint() {
    let server = MockGraphQLServer::start();
    server.answer("TeamStates", json("{}"));
    server.answer("TeamLabels", json("{}"));

    query(
        &server,
        "query TeamStates { workflowStates { nodes { id } } }",
    );
    query(&server, "query TeamLabels { issueLabels { nodes { id } } }");
    query(
        &server,
        "query TeamStates { workflowStates { nodes { id } } }",
    );

    assert_eq!(server.hits("TeamStates"), 2);
    assert_eq!(server.hits("TeamLabels"), 1);
    assert_eq!(server.hits("TeamMembers"), 0);
    assert_eq!(server.bodies("TeamStates").len(), 2);
    assert!(server
        .last_body("TeamLabels")
        .is_some_and(|body| body.windows(11).any(|w| w == b"issueLabels")));
    assert_eq!(server.http().hits(&RequestKey::post(ENDPOINT)), 3);
}

#[test]
fn a_flaky_answer_fails_per_operation_before_succeeding() {
    let server = MockGraphQLServer::start();
    server.answer(
        "TeamStates",
        Route::FlakyThenOk {
            fail_times: 1,
            body: b"ok".to_vec(),
        },
    );
    server.answer("TeamLabels", json("\"labels\""));

    query(&server, "query TeamLabels { issueLabels { nodes { id } } }");
    let failed = query(
        &server,
        "query TeamStates { workflowStates { nodes { id } } }",
    );
    let succeeded = query(
        &server,
        "query TeamStates { workflowStates { nodes { id } } }",
    );

    assert_eq!(failed.status, 500);
    assert_eq!(succeeded.body, b"ok");
}

#[test]
fn an_operation_with_no_answer_is_unmatched_and_listed() {
    let server = MockGraphQLServer::start();
    server.answer("TeamStates", json("{}"));

    let response = query(
        &server,
        "query TeamMembers { teamMemberships { nodes { id } } }",
    );

    assert_eq!(response.status, UNMATCHED_STATUS);
    assert_eq!(server.unanswered(), vec!["TeamMembers".to_owned()]);
    assert_eq!(server.hits("TeamMembers"), 1);
    std::mem::forget(server);
}

#[test]
fn a_body_naming_no_operation_is_unmatched() {
    let server = MockGraphQLServer::start();
    server.answer("TeamStates", json("{}"));

    let response = post(&server, b"not json");

    assert_eq!(response.status, UNMATCHED_STATUS);
    assert!(server.unanswered().is_empty());
}

#[test]
fn dropping_a_server_with_unanswered_operations_panics_naming_them() {
    let outcome = std::panic::catch_unwind(|| {
        let server = MockGraphQLServer::start();
        query(
            &server,
            "query TeamMembers { teamMemberships { nodes { id } } }",
        );
        drop(server);
    });

    let payload = outcome.expect_err("dropping should panic");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_default();
    assert!(message.contains("TeamMembers"), "{message}");
}

#[test]
fn dropping_a_server_whose_operations_were_all_answered_never_panics() {
    let server = MockGraphQLServer::start();
    server.answer("TeamStates", json("{}"));

    query(
        &server,
        "query TeamStates { workflowStates { nodes { id } } }",
    );

    drop(server);
}

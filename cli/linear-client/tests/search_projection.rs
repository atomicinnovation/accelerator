//! The read-side search projection (Decision 20): the widened field selection
//! the `search` subcommand renders, and the cursor round-trip across pages.
//!
//! The port `search` returns stamps only; this op keeps the title and adds the
//! state and assignee names the bash search table shows, over a query distinct
//! from the one the sync bulk read spends its complexity budget on.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::Search;
use serde_json::Value;
use support::client::{brief, client_for, TEAM_ID};

const GRAPHQL: &str = "/graphql";

fn node(identifier: &str, state: &str, assignee: &str) -> String {
    format!(
        "{{\"id\":\"u\",\"identifier\":\"{identifier}\",\"title\":\"t\",\
         \"updatedAt\":\"2026-01-01T00:00:00.000Z\",\
         \"state\":{{\"name\":\"{state}\"}},\
         \"assignee\":{{\"name\":\"{assignee}\"}}}}"
    )
}

fn page(nodes: &str, has_next: bool, cursor: &str) -> String {
    let cursor = if has_next {
        format!("\"{cursor}\"")
    } else {
        "null".to_owned()
    };
    format!(
        "{{\"data\":{{\"issues\":{{\"nodes\":[{nodes}],\
         \"pageInfo\":{{\"hasNextPage\":{has_next},\"endCursor\":{cursor}}}}}}}}}"
    )
}

#[test]
fn the_projection_selects_state_and_assignee_and_follows_the_cursor() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Sequence(vec![
            Route::Json {
                status: 200,
                body: page(&node("ENG-1", "In Progress", "Ada"), true, "c1"),
            },
            Route::Json {
                status: 200,
                body: page(&node("ENG-2", "Done", "Grace"), false, ""),
            },
        ]),
    );

    let client = client_for(&server, brief());
    let search = Search {
        team_id: Some(TEAM_ID.to_owned()),
        text: Some("bug".to_owned()),
        ..Search::default()
    };

    let result = client
        .search_detailed(&search)
        .expect("the projection runs");

    assert_eq!(result.nodes.len(), 2, "both pages accumulate");
    assert!(!result.truncated, "a clean finish is not truncated");
    let name = |node: &Value, pointer: &str| {
        node.pointer(pointer)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    assert_eq!(
        name(&result.nodes[0], "/state/name").as_deref(),
        Some("In Progress")
    );
    assert_eq!(
        name(&result.nodes[0], "/assignee/name").as_deref(),
        Some("Ada")
    );
    assert_eq!(
        name(&result.nodes[1], "/identifier").as_deref(),
        Some("ENG-2")
    );

    let bodies = server.bodies(&RequestKey::post(GRAPHQL));
    assert_eq!(bodies.len(), 2, "two pages were fetched");
    let first = String::from_utf8(bodies[0].clone()).expect("utf8");
    assert!(
        first.contains("state { name }") && first.contains("assignee { name }"),
        "the projection query selects state and assignee: {first}"
    );
    let second: Value = serde_json::from_slice(&bodies[1]).expect("json body");
    assert_eq!(
        second.pointer("/variables/cursor").and_then(Value::as_str),
        Some("c1"),
        "the second page carries the first page's endCursor"
    );
}

#[test]
fn an_unknown_state_filter_is_refused_rather_than_queried() {
    let server = MockServer::start();
    // No route registered: a wire call would 599, so a refusal must precede it.
    let client = client_for(&server, brief());
    let search = Search {
        team_id: Some(TEAM_ID.to_owned()),
        state: Some("Nonexistent".to_owned()),
        ..Search::default()
    };

    let error = client.search_detailed(&search).expect_err("unknown state");

    assert!(
        error.to_string().contains("E_SEARCH_UNKNOWN_STATE"),
        "{error}"
    );
    assert_eq!(
        server.hits(&RequestKey::post(GRAPHQL)),
        0,
        "nothing was sent"
    );
}

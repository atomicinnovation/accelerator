//! `RemoteIssue.body` against the committed corpus, through the port.
//!
//! The recipe itself is pinned in `remote-projection`; what this adds is the
//! port contract on top of it — the trailing newline the projection does not
//! emit, and key-order invariance surviving the client rather than only the
//! recipe.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::path::Path;
use std::path::PathBuf;

use http_test_support::{MockServer, RequestKey, Route};
use support::client::{brief, client_with};
use support::RecordingSleeper;
use tracker::{ExternalId, RemoteTracker as _};

type TestError = Box<dyn std::error::Error>;

fn case_dir(name: &str) -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(
            "../../skills/work/scripts/test-fixtures/work-item-project-remote",
        )
        .join(name)
        .canonicalize()?)
}

/// `expected.txt` is a keyed metadata file, not a raw body: `integration=`,
/// `updated=`, `body=<first line>`, then the canonicalised description. The
/// expected body is reconstructed line-wise and carries **no** trailing
/// newline — comparing a projection plus a newline against it fails.
fn expected_body(name: &str) -> Result<String, TestError> {
    let raw = std::fs::read_to_string(case_dir(name)?.join("expected.txt"))?;
    let lines: Vec<&str> = raw.lines().collect();
    let first = lines[2]
        .strip_prefix("body=")
        .expect("line 3 is body=<first line>");
    Ok(format!("{first}\n{}", lines[3]))
}

fn show_body(name: &str, key: &str) -> Result<String, TestError> {
    let payload = std::fs::read_to_string(case_dir(name)?.join("remote.json"))?;
    let server = MockServer::start();
    server.route(
        RequestKey::get(&format!("/rest/api/3/issue/{key}")),
        Route::Json {
            status: 200,
            body: payload,
        },
    );
    let client =
        client_with(&server.base_url(), brief(), &RecordingSleeper::new());

    Ok(client
        .show(&ExternalId::new(key.to_owned()))
        .expect("show succeeds")
        .body)
}

#[test]
fn the_port_body_is_the_projection_plus_exactly_one_newline(
) -> Result<(), TestError> {
    for (case, key) in
        [("case-jira", "ENG-1"), ("case-jira-reordered", "ENG-2")]
    {
        let body = show_body(case, key)?;
        assert_eq!(body, format!("{}\n", expected_body(case)?), "case {case}");
        assert!(body.ends_with('\n'));
        assert!(!body.ends_with("\n\n"), "exactly one, not two");
    }
    Ok(())
}

#[test]
fn key_order_invariance_survives_the_client() -> Result<(), TestError> {
    // Compared to each other, not to a golden: a client enabling
    // preserve_order would fail this rather than silently rehashing the
    // corpus.
    assert_eq!(
        show_body("case-jira", "ENG-1")?,
        show_body("case-jira-reordered", "ENG-2")?
    );
    Ok(())
}

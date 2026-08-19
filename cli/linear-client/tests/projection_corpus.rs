//! `RemoteIssue.body` against the committed corpus, through the port.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::path::Path;
use std::path::PathBuf;

use http_test_support::{MockServer, RequestKey, Route};
use support::client::{brief, client_for};
use tracker::{ExternalId, RemoteTracker as _};

type TestError = Box<dyn std::error::Error>;

fn corpus(directory: &str) -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../skills/work/scripts/test-fixtures")
        .join(directory)
        .canonicalize()?)
}

/// `expected.txt` is keyed metadata, not a raw body: the expected body is
/// reconstructed line-wise and carries no trailing newline.
fn expected_body(case: &str) -> Result<String, TestError> {
    let raw = std::fs::read_to_string(
        corpus("work-item-project-remote")?
            .join(case)
            .join("expected.txt"),
    )?;
    let lines: Vec<&str> = raw.lines().collect();
    let first = lines[2].strip_prefix("body=").expect("line 3 is body=");
    Ok(format!("{first}\n{}", lines[3]))
}

fn show_body(payload: &str) -> String {
    let server = MockServer::start();
    server.route(
        RequestKey::post("/graphql"),
        Route::Json {
            status: 200,
            body: payload.to_owned(),
        },
    );
    let client = client_for(&server, brief());

    client
        .show(&ExternalId::new("ENG-1".to_owned()))
        .expect("show succeeds")
        .body
}

#[test]
fn the_port_body_is_the_projection_plus_exactly_one_newline(
) -> Result<(), TestError> {
    let payload = std::fs::read_to_string(
        corpus("work-item-project-remote")?
            .join("case-linear")
            .join("remote.json"),
    )?;

    let body = show_body(&payload);

    assert_eq!(body, format!("{}\n", expected_body("case-linear")?));
    assert!(!body.ends_with("\n\n"), "exactly one, not two");
    Ok(())
}

#[test]
fn the_sync_baseline_records_project_through_the_port() -> Result<(), TestError>
{
    // The expected bytes are stated per case rather than derived from the
    // projection, so the assertion does not restate the implementation. The
    // recipe's own hash parity against the committed remote_hash is pinned in
    // remote-projection's corpus test.
    for (case, expected) in [
        // An empty description is an empty line, and the projection therefore
        // already ends in exactly one newline: the port body is unchanged.
        // Emitting a second newline here would change remote_hash and
        // reclassify every such item.
        ("case-linear-empty-description", "Empty description title\n"),
        (
            "case-linear-markdown",
            "Linear title\nBody **markdown** text.\n",
        ),
    ] {
        let directory = corpus("work-item-sync-baseline")?.join(case);
        let payload = std::fs::read_to_string(directory.join("remote.json"))?;

        let body = show_body(&payload);

        assert_eq!(body, expected, "case {case}");
        assert!(body.ends_with('\n'));
        assert!(!body.ends_with("\n\n"), "case {case}: exactly one newline");
    }
    Ok(())
}

#[test]
fn the_port_body_is_what_the_projection_hashes_to() -> Result<(), TestError> {
    // The sync engine digests the body it is handed, so the port body must be
    // byte-identical to the projection the committed remote_hash was taken
    // over — for the empty-description case that means NOT appending a second
    // newline.
    let directory = corpus("work-item-sync-baseline")?
        .join("case-linear-empty-description");
    let payload = std::fs::read_to_string(directory.join("remote.json"))?;
    let projected = remote_projection::project_raw(
        remote_projection::Integration::Linear,
        remote_projection::Op::Body,
        &payload,
    )?;

    assert_eq!(show_body(&payload), projected);
    Ok(())
}

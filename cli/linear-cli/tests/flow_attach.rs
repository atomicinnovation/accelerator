//! The `attach` flow: a link is one `attachmentCreate` mutation; a binary file
//! is the three-step upload (`fileUpload` POST, a raw PUT of the bytes, then
//! `attachmentCreate`), the genuine multi-POST case, asserted per hit.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::{MockServer, RequestKey};
use serde_json::Value;

fn scenario_text(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios")
        .join(format!("{name}.json"));
    std::fs::read_to_string(path).expect("scenario text")
}

#[test]
fn attach_link_posts_one_attachment_create() {
    let server = MockServer::start();
    Scenario::from_json(&scenario_text("attach-link-200"))
        .expect("scenario")
        .install(&server);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["attach", "BLA-1", "--url", "https://example.com/design-doc"],
    );

    assert!(
        output.status.success(),
        "attach exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        stdout.get("outcome").and_then(Value::as_str),
        Some("attached")
    );
    let key = RequestKey::post("/graphql");
    assert_eq!(server.hits(&key), 1);
    let sent = String::from_utf8_lossy(&server.last_body(&key).expect("body"))
        .into_owned();
    assert!(sent.contains("attachmentCreate"), "sent: {sent}");
}

#[test]
fn attach_file_uploads_then_registers_across_two_posts() {
    let server = MockServer::start();
    // The upload URL the mock nominates is the mock itself, so the raw PUT
    // stays on loopback (admitted only under `test-loopback`).
    let text = scenario_text("attach-binary-success")
        .replace("__MOCK_URL__", &server.base_url());
    Scenario::from_json(&text)
        .expect("scenario")
        .install(&server);
    let dir = support::scratch(support::CONFIG);
    let blob = dir.path().join("payload.bin");
    std::fs::write(&blob, b"the attachment bytes").expect("write blob");

    let output = support::run(
        dir.path(),
        &server,
        &[
            "attach",
            "BLA-1",
            "--file",
            blob.to_str().expect("utf-8 path"),
        ],
    );

    assert!(
        output.status.success(),
        "attach --file exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        stdout.get("outcome").and_then(Value::as_str),
        Some("attached")
    );

    // Two POSTs to the single GraphQL endpoint, told apart by the per-hit body
    // log; the bytes went out as one PUT to the nominated upload URL.
    let graphql = RequestKey::post("/graphql");
    assert_eq!(server.hits(&graphql), 2);
    assert_eq!(server.hits(&RequestKey::put("/upload")), 1);
    let bodies = server.bodies(&graphql);
    assert_eq!(bodies.len(), 2);
    assert!(
        String::from_utf8_lossy(&bodies[0]).contains("fileUpload"),
        "first POST registers the upload"
    );
    assert!(
        String::from_utf8_lossy(&bodies[1]).contains("attachmentCreate"),
        "second POST registers the asset"
    );
    let put = server.bodies(&RequestKey::put("/upload"));
    assert_eq!(put.len(), 1);
    assert_eq!(put[0], b"the attachment bytes", "the raw bytes were PUT");
}

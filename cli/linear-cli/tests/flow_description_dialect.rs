//! `create` and `update` send a description escaped for Linear's parser, and
//! `--verbatim` sends it as given.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;
use std::process::Output;
use std::sync::LazyLock;

use cli_test_support::Scenario;
use http_test_support::{MockHTTPServer, RequestKey};
use serde_json::Value;

const NEEDS_BOTH_ESCAPES: &str = "- owned by\n  0160.\n\nSee crates.io.\n";
const ESCAPED: &str = "- owned by\n  0160\\.\n\nSee crates\\.io.\n";
static DESCRIPTION: LazyLock<String> =
    LazyLock::new(|| format!("--description={NEEDS_BOTH_ESCAPES}"));

fn graphql() -> RequestKey {
    RequestKey::post("/graphql")
}

struct Run {
    server: MockHTTPServer,
    output: Output,
}

impl Run {
    fn sent_description(&self) -> String {
        assert!(
            self.output.status.success(),
            "exited {:?}: {}",
            self.output.status.code(),
            String::from_utf8_lossy(&self.output.stderr)
        );
        let key = graphql();
        assert_eq!(self.server.hits(&key), 1, "exactly one mutation is sent");
        let sent: Value = serde_json::from_slice(
            &self.server.last_body(&key).expect("a body"),
        )
        .expect("a JSON request");
        sent["variables"]["input"]["description"]
            .as_str()
            .expect("a description")
            .to_owned()
    }

    fn assert_usage_error_sending_nothing(&self) {
        assert_eq!(
            self.output.status.code(),
            Some(2),
            "stderr: {}",
            String::from_utf8_lossy(&self.output.stderr)
        );
        assert_eq!(self.server.hits(&graphql()), 0);
    }
}

fn run(scenario: &str, args: &[&str]) -> Run {
    let server = MockHTTPServer::start();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/scenarios/{scenario}.json"));
    Scenario::load(&path).expect("scenario").install(&server);
    let dir = support::scratch(support::CONFIG);
    std::fs::write(dir.path().join("body.md"), NEEDS_BOTH_ESCAPES)
        .expect("write body file");
    std::fs::write(
        dir.path().join("work-item.md"),
        format!("---\ntitle: \"A work item\"\n---\n{NEEDS_BOTH_ESCAPES}"),
    )
    .expect("write work-item file");
    let output = support::run(dir.path(), &server, args);
    Run { server, output }
}

fn create(args: &[&str]) -> Run {
    run("create-201", &[&["create"], args].concat())
}

fn update(args: &[&str]) -> Run {
    run("issue-update-200", &[&["update", "BLA-1"], args].concat())
}

fn every_description_source(verbatim: &[&str]) -> Vec<Run> {
    vec![
        create(&[&["work-item.md"], verbatim].concat()),
        create(
            &[&["--title", "t", "--body-file", "body.md"], verbatim].concat(),
        ),
        update(&[&["--body-file", "body.md"], verbatim].concat()),
        update(&[&[DESCRIPTION.as_str()], verbatim].concat()),
    ]
}

#[test]
fn a_work_item_file_is_sent_escaped() {
    assert_eq!(create(&["work-item.md"]).sent_description(), ESCAPED);
}

#[test]
fn a_body_file_is_sent_escaped() {
    assert_eq!(
        create(&["--title", "t", "--body-file", "body.md"]).sent_description(),
        ESCAPED
    );
    assert_eq!(
        update(&["--body-file", "body.md"]).sent_description(),
        ESCAPED
    );
}

#[test]
fn a_description_is_sent_escaped() {
    assert_eq!(update(&[DESCRIPTION.as_str()]).sent_description(), ESCAPED);
}

#[test]
fn verbatim_sends_the_description_as_given() {
    for sent in every_description_source(&["--verbatim"]) {
        assert_eq!(sent.sent_description(), NEEDS_BOTH_ESCAPES);
    }
}

#[test]
fn verbatim_without_a_description_is_refused() {
    update(&["--title", "t", "--verbatim"])
        .assert_usage_error_sending_nothing();
    create(&["--title", "t", "--verbatim"])
        .assert_usage_error_sending_nothing();
}

#[test]
fn a_file_and_a_body_file_behave_as_before() {
    assert_eq!(
        create(&["work-item.md", "--body-file", "missing.md"])
            .sent_description(),
        ESCAPED,
        "without --title, the work-item file's body is sent"
    );
}

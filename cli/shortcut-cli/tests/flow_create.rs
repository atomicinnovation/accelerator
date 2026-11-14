//! `create` end to end: the entity follows the work item's kind, and the
//! `external_id` is written back only after the create lands.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use http_test_support::RequestKey;

const STORIES: &str = "/api/v4/scratch-workspace/stories";
const EPICS: &str = "/api/v4/scratch-workspace/epics";

fn work_item(dir: &Path, kind: &str) -> std::path::PathBuf {
    let path = dir.join("0042-a-thing.md");
    std::fs::write(
        &path,
        format!(
            "---\nid: \"0042\"\ntitle: \"A thing\"\nkind: \"{kind}\"\n\
             external_id: \"\"\n---\n\n# A thing\n\nThe body.\n"
        ),
    )
    .expect("write the work item");
    path
}

fn create(
    scenarios: &[&str],
    kind: &str,
) -> (
    std::process::Output,
    String,
    std::path::PathBuf,
    http_test_support::MockHTTPServer,
    tempfile::TempDir,
) {
    let server = support::serving(scenarios);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let item = work_item(dir.path(), kind);
    let before = std::fs::read_to_string(&item).expect("read");
    let output = support::run(
        dir.path(),
        &server,
        &["create", item.to_str().expect("utf-8")],
    );
    (output, before, item, server, dir)
}

#[test]
fn a_story_is_created_and_linked() {
    let (output, _, item, server, _dir) =
        create(&["story-create-201"], "story");

    assert!(output.status.success(), "{}", support::stderr(&output));
    let stdout = support::stdout_json(&output);
    assert_eq!(stdout["outcome"], "created");
    assert_eq!(stdout["key"], "sc-14");
    let linked = std::fs::read_to_string(item).expect("read");
    assert!(linked.contains("external_id: \"sc-14\""), "{linked}");
    let sent = support::sent_json(&server, &RequestKey::post(STORIES));
    assert_eq!(sent["name"], "A thing");
    assert_eq!(sent["story_type"], "feature");
}

#[test]
fn an_epic_kind_creates_an_epic() {
    let (output, _, item, server, _dir) = create(&["epic-create-201"], "epic");

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(server.hits(&RequestKey::post(EPICS)), 1);
    let linked = std::fs::read_to_string(item).expect("read");
    assert!(linked.contains("external_id: \"epic-"), "{linked}");
}

#[test]
fn a_failed_create_leaves_the_work_item_unchanged() {
    let (output, before, item, server, _dir) =
        create(&["story-create-500"], "story");

    assert!(!output.status.success());
    assert_eq!(std::fs::read_to_string(item).expect("read"), before);
    assert_eq!(server.hits(&RequestKey::post(STORIES)), 1);
}

#[test]
fn an_already_linked_work_item_is_refused_unsent() {
    let server = support::serving(&["story-create-201"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let item = dir.path().join("0043.md");
    std::fs::write(
        &item,
        "---\ntitle: \"Linked\"\nexternal_id: \"sc-9\"\n---\n\nBody\n",
    )
    .expect("write");

    let output = support::run(
        dir.path(),
        &server,
        &["create", item.to_str().expect("utf-8")],
    );

    assert_eq!(output.status.code(), Some(102));
    assert_eq!(server.hits(&RequestKey::post(STORIES)), 0);
}

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

    assert_eq!(output.status.code(), Some(141));
    let stderr = support::stderr(&output);
    assert!(stderr.starts_with("E_WRITE_MAY_HAVE_APPLIED:"), "{stderr}");
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

#[test]
fn a_work_item_file_with_a_title_flag_is_refused_unsent() {
    let server = support::serving(&["story-create-201"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let item = dir.path().join("0043.md");
    let linked = "---\ntitle: \"Linked\"\nexternal_id: \"sc-9\"\n---\n\nBody\n";
    std::fs::write(&item, linked).expect("write");

    let output = support::run(
        dir.path(),
        &server,
        &[
            "create",
            item.to_str().expect("utf-8"),
            "--title",
            "Another",
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        support::stderr(&output)
    );
    assert_eq!(server.hits(&RequestKey::post(STORIES)), 0);
    assert_eq!(std::fs::read_to_string(&item).expect("read"), linked);
}

#[test]
fn a_work_item_with_template_comments_is_created_and_linked() {
    let server = support::serving(&["epic-create-201"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let item = dir.path().join("0044.md");
    std::fs::write(
        &item,
        "---\ntitle: \"Commented\"\nkind: \"epic\"   # story | epic | bug\n\
         external_id: \"\"   # remote tracker identifier\n---\n\nThe body.\n",
    )
    .expect("write");

    let output = support::run(
        dir.path(),
        &server,
        &["create", item.to_str().expect("utf-8")],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(server.hits(&RequestKey::post(EPICS)), 1);
    let linked = std::fs::read_to_string(item).expect("read");
    assert!(linked.contains("external_id: \"epic-"), "{linked}");
}

#[test]
fn linking_adds_a_missing_external_id_and_keeps_the_body() {
    let server = support::serving(&["story-create-201"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let item = dir.path().join("0045.md");
    let body = "\n# Unlinked\n\nFirst paragraph.\n\n---\n\nAfter a rule.\n";
    std::fs::write(&item, format!("---\ntitle: \"Unlinked\"\n---\n{body}"))
        .expect("write");

    let output = support::run(
        dir.path(),
        &server,
        &["create", item.to_str().expect("utf-8")],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    let linked = std::fs::read_to_string(item).expect("read");
    assert!(linked.contains("external_id: \"sc-14\""), "{linked}");
    assert!(linked.ends_with(body), "{linked}");
}

#[cfg(unix)]
#[test]
fn a_create_whose_link_cannot_be_written_reports_the_key() {
    use std::os::unix::fs::PermissionsExt;

    let server = support::serving(&["story-create-201"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let work_dir = dir.path().join("meta/work");
    std::fs::create_dir_all(&work_dir).expect("create the work dir");
    let item = work_item(&work_dir, "story");
    let before = std::fs::read_to_string(&item).expect("read");
    let read_only = std::fs::Permissions::from_mode(0o555);
    std::fs::set_permissions(&work_dir, read_only).expect("chmod");

    let output = support::run(
        dir.path(),
        &server,
        &["create", item.to_str().expect("utf-8")],
    );
    std::fs::set_permissions(&work_dir, std::fs::Permissions::from_mode(0o755))
        .expect("chmod back");

    assert_eq!(
        output.status.code(),
        Some(107),
        "{}",
        support::stderr(&output)
    );
    let stdout = support::stdout_json(&output);
    assert_eq!(stdout["outcome"], "writeback-failed");
    assert_eq!(stdout["key"], "sc-14");
    let stderr = support::stderr(&output);
    assert!(stderr.starts_with("E_CREATE_WRITEBACK_FAILED:"), "{stderr}");
    assert!(stderr.contains("sc-14"), "{stderr}");
    assert_eq!(server.hits(&RequestKey::post(STORIES)), 1);
    assert_eq!(std::fs::read_to_string(item).expect("read"), before);
}

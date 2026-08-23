//! Pins the `E_*` diagnostic names the binary writes to stderr for its
//! argument-validation and seam refusals. Under Decision 11 the repointed
//! bodies branch on the stdout keyword, but the `E_*` names remain the stderr
//! diagnostic a human (and a legacy grep) reads, so each is asserted verbatim.
//! Every case refuses before any request, so no mock is needed.
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use support::Token;

fn stderr_of(dir: &Path, args: &[&str], api_url: Option<&str>) -> String {
    let output = support::run_with(dir, args, api_url, &Token::Present);
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn write(dir: &Path, name: &str, contents: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, contents).expect("write work item");
    path.to_str().expect("utf-8 path").to_owned()
}

#[test]
fn argument_validation_diagnostics_name_their_error() {
    let dir = support::scratch(support::CONFIG);
    let dir = dir.path();

    let cases: Vec<(&str, Vec<String>)> = vec![
        ("E_COMMENT_NO_BODY", vec_of(&["comment", "add", "BLA-1"])),
        ("E_TRANSITION_NO_STATE", vec_of(&["transition", "BLA-1"])),
        (
            "E_ATTACH_BOTH_TARGETS",
            vec_of(&["attach", "BLA-1", "--url", "u", "--file", "f"]),
        ),
        ("E_ATTACH_NO_TARGET", vec_of(&["attach", "BLA-1"])),
        ("E_UPDATE_NO_OPS", vec_of(&["update", "BLA-1"])),
        ("E_CREATE_NO_TITLE", vec_of(&["create"])),
        (
            "E_CREATE_NO_FILE",
            vec_of(&["create", "--title", "T", "--body-file", "/no/such"]),
        ),
    ];

    for (token, args) in &cases {
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let stderr = stderr_of(dir, &borrowed, None);
        assert!(
            stderr.contains(token),
            "expected {token} in stderr for {borrowed:?}, got: {stderr}"
        );
    }
}

#[test]
fn create_from_file_diagnostics_name_their_error() {
    let dir = support::scratch(support::CONFIG);
    let dir = dir.path();

    let no_frontmatter = write(dir, "raw.md", "just a body, no frontmatter\n");
    assert!(stderr_of(dir, &["create", &no_frontmatter], None)
        .contains("E_CREATE_BAD_FRONTMATTER"));

    let synced = write(
        dir,
        "synced.md",
        "---\ntitle: \"Already there\"\nexternal_id: \"BLA-7\"\n---\nbody\n",
    );
    assert!(stderr_of(dir, &["create", &synced], None)
        .contains("E_CREATE_ALREADY_SYNCED"));
}

#[test]
fn a_bad_api_url_names_its_error() {
    let dir = support::scratch(support::CONFIG);
    let stderr =
        stderr_of(dir.path(), &["init", "verify"], Some("::: not a url"));
    assert!(stderr.contains("E_BAD_API_URL"), "got: {stderr}");
}

fn vec_of(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_owned()).collect()
}

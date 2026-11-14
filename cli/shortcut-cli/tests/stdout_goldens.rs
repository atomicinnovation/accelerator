//! Byte-exact stdout for the JSON subcommands. Run with `UPDATE_GOLDEN=1` to
//! rewrite a golden after a deliberate change.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

fn assert_golden(name: &str, actual: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/goldens")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().expect("golden parent"))
            .expect("mkdir goldens");
        std::fs::write(&path, actual).expect("write golden");
        return;
    }
    let expected = std::fs::read(&path)
        .expect("missing golden — run with UPDATE_GOLDEN=1 to create it");
    assert_eq!(
        actual,
        expected.as_slice(),
        "stdout diverged from {}; re-run with UPDATE_GOLDEN=1 to accept",
        path.display()
    );
}

fn drive(scenarios: &[&str], args: &[&str]) -> Vec<u8> {
    let server = support::serving(scenarios);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let output = support::run(dir.path(), &server, args);
    assert!(output.status.success(), "{}", support::stderr(&output));
    output.stdout
}

#[test]
fn show_stdout_matches_the_golden() {
    assert_golden(
        "show.golden",
        &drive(&["story-show-200"], &["show", "sc-14"]),
    );
}

#[test]
fn search_stdout_matches_the_golden() {
    assert_golden("search.golden", &drive(&["search-team-200"], &["search"]));
}

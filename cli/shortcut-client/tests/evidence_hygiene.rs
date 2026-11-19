//! What a credentialed run may leave committed.
//!
//! The live test writes raw responses to the gitignored `tests/captures/`. A
//! person turns those into sanitised fixtures under `tests/fixtures/v4/`, and
//! the run's reduced records go to `tests/evidence/`. This guard runs in the
//! default profile, so `mise run` refuses a fixture or a record that carries a
//! token, an email address or a real workspace slug.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

const PLACEHOLDER_SLUG: &str = "scratch-workspace";
const FORBIDDEN: [&str; 3] = ["sct_", "Bearer ", "@"];

fn crate_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

fn foreign_slugs(contents: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (index, _) in contents.match_indices("/api/v4/") {
        let rest = &contents[index + "/api/v4/".len()..];
        let segment: String = rest
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || *character == '-'
            })
            .collect();
        if !segment.is_empty()
            && segment != PLACEHOLDER_SLUG
            && segment != "whoami"
        {
            found.push(segment);
        }
    }
    for (index, _) in contents.match_indices("\"slug\"") {
        let rest = contents[index + "\"slug\"".len()..].trim_start();
        let value = rest
            .strip_prefix(':')
            .map(str::trim_start)
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.split('"').next());
        if let Some(value) = value.filter(|value| *value != PLACEHOLDER_SLUG) {
            found.push(value.to_owned());
        }
    }
    found
}

#[test]
fn committed_evidence_is_reduced() -> Result<(), TestError> {
    let path = crate_dir().join("tests/evidence/contract-run.txt");
    if !path.exists() {
        return Ok(());
    }
    let contents = std::fs::read_to_string(&path)?;
    tracker_test_support::evidence::is_reduced(&contents)
        .map_err(|violations| format!("{}: {violations:?}", path.display()))?;
    Ok(())
}

#[test]
fn committed_fixtures_carry_no_token_address_or_real_slug(
) -> Result<(), TestError> {
    let mut violations = Vec::new();
    for file in files_under(&crate_dir().join("tests/fixtures/v4")) {
        let contents = std::fs::read_to_string(&file)?;
        for shape in FORBIDDEN {
            if contents.contains(shape) {
                violations.push(format!("{}: holds {shape:?}", file.display()));
            }
        }
        for slug in foreign_slugs(&contents) {
            violations.push(format!(
                "{}: names the workspace {slug:?}, not {PLACEHOLDER_SLUG:?}",
                file.display()
            ));
        }
    }
    assert!(violations.is_empty(), "{violations:#?}");
    Ok(())
}

#[test]
fn the_slug_scan_finds_a_real_workspace() {
    let found = foreign_slugs(
        r#"{"next_page_url": "https://x/api/v4/acme/stories", "slug": "acme"}"#,
    );

    assert_eq!(found, vec!["acme".to_owned(), "acme".to_owned()]);
    assert!(foreign_slugs(
        r#"{"slug": "scratch-workspace", "u": "/api/v4/scratch-workspace/teams"}"#
    )
    .is_empty());
}

#[test]
fn raw_captures_are_ignored_by_version_control() -> Result<(), TestError> {
    let captures = crate_dir().join("tests/captures/probe.json");
    let status = Command::new("git")
        .arg("check-ignore")
        .arg("--quiet")
        .arg("--no-index")
        .arg(&captures)
        .current_dir(crate_dir())
        .status()?;
    assert!(
        status.success(),
        "{} must be gitignored, so raw captures cannot be committed",
        captures.display()
    );
    Ok(())
}

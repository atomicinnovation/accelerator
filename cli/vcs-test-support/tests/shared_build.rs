use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use serde_json::Value;
use vcs_test_support::family::Family;
use vcs_test_support::fixture_log::FixtureLog;
use vcs_test_support::shared_build::SharedBuildRequest;
use vcs_test_support::shared_build::ToolVersions;

type TestError = Box<dyn std::error::Error>;

fn arguments(words: &[&str]) -> Vec<String> {
    words.iter().map(|&word| word.to_owned()).collect()
}

fn matrix_at(tree: &Path) -> SharedBuildRequest {
    SharedBuildRequest {
        trees: BTreeMap::from([(Family::Matrix, tree.to_path_buf())]),
    }
}

fn events(log: &Path) -> Result<Vec<Value>, TestError> {
    fs::read_to_string(log)?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

fn first_line_of_version(binary: &str) -> Result<String, TestError> {
    let output = Command::new(binary).arg("--version").output()?;
    let text = String::from_utf8(output.stdout)?;
    Ok(text.lines().next().ok_or("no version line")?.to_owned())
}

#[test]
fn a_family_flag_requests_that_family_at_its_tree() -> Result<(), TestError> {
    let request = SharedBuildRequest::parse(arguments(&["--matrix", "/a"]))?;

    assert_eq!(
        request.trees,
        BTreeMap::from([(Family::Matrix, PathBuf::from("/a"))])
    );
    Ok(())
}

#[test]
fn both_family_flags_request_both_families() -> Result<(), TestError> {
    let request = SharedBuildRequest::parse(arguments(&[
        "--matrix",
        "/a",
        "--status-log",
        "/b",
    ]))?;

    assert_eq!(
        request.trees,
        BTreeMap::from([
            (Family::Matrix, PathBuf::from("/a")),
            (Family::StatusLog, PathBuf::from("/b")),
        ])
    );
    Ok(())
}

#[test]
fn an_unknown_argument_is_refused_by_name() -> Result<(), TestError> {
    let error = SharedBuildRequest::parse(arguments(&["--matrx", "/a"]))
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("--matrx"), "{error}");
    Ok(())
}

#[test]
fn a_family_flag_without_a_tree_is_refused() -> Result<(), TestError> {
    let error = SharedBuildRequest::parse(arguments(&["--matrix"]))
        .err()
        .ok_or("it should fail")?;

    assert!(
        error.to_string().contains("--matrix needs a tree"),
        "{error}"
    );
    Ok(())
}

#[test]
fn a_request_naming_no_family_is_refused() -> Result<(), TestError> {
    let error = SharedBuildRequest::parse(Vec::new())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("no family"), "{error}");
    Ok(())
}

#[test]
fn a_tree_inside_a_checkout_is_refused_before_anything_is_written(
) -> Result<(), TestError> {
    let checkout = tempfile::tempdir()?;
    fs::create_dir(checkout.path().join(".git"))?;
    let run = checkout.path().join("run");
    let log = checkout.path().join("run.jsonl");

    let failures = matrix_at(&run.join("matrix")).build(&FixtureLog::at(&log));

    assert!(!run.exists(), "{} was created", run.display());
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].family, Family::Matrix);
    assert!(
        failures[0].cause.contains("lies inside the repository"),
        "{}",
        failures[0].cause
    );
    assert!(
        failures[0]
            .cause
            .contains("Point TMPDIR at a directory under /tmp"),
        "{}",
        failures[0].cause
    );
    assert!(!log.exists());
    Ok(())
}

#[test]
fn a_successful_build_records_one_shared_build() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let log = directory.path().join("run.jsonl");
    let tree = directory.path().join("matrix");
    let versions = ToolVersions::detect()?;

    let failures =
        matrix_at(&tree).build_with(&FixtureLog::at(&log), |_, _| Ok(()));

    assert!(failures.is_empty());
    let recorded = events(&log)?;
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0]["event"], "shared-build");
    assert_eq!(recorded[0]["family"], "matrix");
    assert_eq!(recorded[0]["tree"], tree.display().to_string());
    assert_eq!(recorded[0]["git"], versions.git);
    assert_eq!(recorded[0]["jj"], versions.jj);
    Ok(())
}

#[test]
fn a_build_that_cannot_be_logged_is_a_failure_of_its_family(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let unwritable_log = directory.path().to_path_buf();

    let failures = matrix_at(&directory.path().join("matrix"))
        .build_with(&FixtureLog::at(&unwritable_log), |_, _| Ok(()));

    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].family, Family::Matrix);
    assert!(!failures[0].cause.is_empty());
    Ok(())
}

#[test]
fn a_panicking_build_becomes_a_failure_of_its_family() -> Result<(), TestError>
{
    let directory = tempfile::tempdir()?;
    let request = matrix_at(&directory.path().join("matrix"));
    let panicked = AtomicBool::new(false);
    let builder = |_: Family, _: &Path| {
        assert!(panicked.swap(true, Ordering::SeqCst), "the builder broke");
        Ok(())
    };

    let first = request.build_with(&FixtureLog::disabled(), builder);
    let second = request.build_with(&FixtureLog::disabled(), builder);

    assert_eq!(first.len(), 1);
    assert_eq!(first[0].family, Family::Matrix);
    assert!(
        first[0].cause.contains("the builder broke"),
        "{}",
        first[0].cause
    );
    assert!(second.is_empty());
    Ok(())
}

#[test]
fn tool_versions_are_the_first_line_of_each_tool_s_version(
) -> Result<(), TestError> {
    let versions = ToolVersions::detect()?;

    assert_eq!(versions.git, first_line_of_version("git")?);
    assert_eq!(versions.jj, first_line_of_version("jj")?);
    Ok(())
}

#[test]
#[allow(clippy::panic)]
fn a_build_panicking_with_a_formatted_message_names_it() -> Result<(), TestError>
{
    let directory = tempfile::tempdir()?;
    let request = matrix_at(&directory.path().join("matrix"));

    let failures = request.build_with(&FixtureLog::disabled(), |_, tree| {
        std::panic::panic_any(format!("broke at {}", tree.display()))
    });

    assert_eq!(failures.len(), 1);
    assert!(
        failures[0].cause.contains("broke at "),
        "{}",
        failures[0].cause
    );
    Ok(())
}

#[test]
#[allow(clippy::panic)]
fn a_build_panicking_without_a_message_still_fails_its_family(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let request = matrix_at(&directory.path().join("matrix"));

    let failures = request
        .build_with(&FixtureLog::disabled(), |_, _| std::panic::panic_any(42));

    assert_eq!(failures.len(), 1);
    assert!(
        failures[0].cause.contains("an unprintable panic"),
        "{}",
        failures[0].cause
    );
    Ok(())
}

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use vcs_test_support::status_log_manifest::StatusLogManifest;

type TestError = Box<dyn std::error::Error>;

#[test]
fn a_written_manifest_reads_back_unchanged() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let written = StatusLogManifest {
        states: BTreeMap::from([
            ("clean-git".to_owned(), directory.path().join("clean-git")),
            ("dirty-jj".to_owned(), directory.path().join("dirty-jj")),
        ]),
    };
    written.write(directory.path())?;

    let read = StatusLogManifest::read(directory.path())?;

    assert_eq!(read.states, written.states);
    Ok(())
}

#[test]
fn reading_a_directory_without_a_manifest_fails_naming_it(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;

    let error = StatusLogManifest::read(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(
        error
            .to_string()
            .contains(&directory.path().display().to_string()),
        "{error}"
    );
    Ok(())
}

#[test]
fn reading_a_malformed_manifest_fails() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("status-log-states.tsv"),
        "clean-git\n",
    )?;

    let error = StatusLogManifest::read(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("malformed"), "{error}");
    Ok(())
}

#[test]
fn reading_an_empty_manifest_fails() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("status-log-states.tsv"), "")?;

    let error = StatusLogManifest::read(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("empty"), "{error}");
    Ok(())
}

#[test]
fn a_manifest_names_each_state_on_its_own_line() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    StatusLogManifest {
        states: BTreeMap::from([(
            "clean-git".to_owned(),
            PathBuf::from("/fixtures/clean-git"),
        )]),
    }
    .write(directory.path())?;

    assert_eq!(
        fs::read_to_string(directory.path().join("status-log-states.tsv"))?,
        "clean-git\t/fixtures/clean-git\n"
    );
    Ok(())
}

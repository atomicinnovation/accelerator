use std::fs;
use std::path::Path;
use std::path::PathBuf;

use vcs_test_support::family::SharedFamily;
use vcs_test_support::fixtures::Fixture;
use vcs_test_support::fixtures::Matrix;
use vcs_test_support::manifest::Manifest;

type TestError = Box<dyn std::error::Error>;

fn fixture(key: &str, shape: &str, start: PathBuf) -> Fixture {
    Fixture {
        key: key.to_owned(),
        shape: shape.to_owned(),
        start,
    }
}

fn written_manifest(base: &Path) -> Result<Manifest, TestError> {
    let manifest = Manifest {
        fixtures: vec![
            fixture("PG-r", "plain git", base.join("PG")),
            fixture("PG-s", "plain git, subdirectory", base.join("PG/deep/er")),
        ],
    };
    manifest.write(base)?;
    Ok(manifest)
}

fn keys_and_starts(fixtures: &[Fixture]) -> Vec<(String, PathBuf)> {
    fixtures
        .iter()
        .map(|fixture| (fixture.key.clone(), fixture.start.clone()))
        .collect()
}

#[test]
fn a_written_manifest_reads_back_unchanged() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let written = written_manifest(directory.path())?;

    let read = Manifest::read(directory.path())?;

    assert_eq!(
        keys_and_starts(&read.fixtures),
        keys_and_starts(&written.fixtures)
    );
    Ok(())
}

#[test]
fn adopting_a_tree_reads_its_manifest() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let base = directory.path().canonicalize()?;
    let written = written_manifest(&base)?;

    let adopted = Matrix::adopt(directory.path())?;

    assert_eq!(adopted.base, base);
    assert_eq!(
        keys_and_starts(&adopted.fixtures),
        keys_and_starts(&written.fixtures)
    );
    Ok(())
}

#[test]
fn adopting_a_tree_without_a_manifest_fails_naming_it() -> Result<(), TestError>
{
    let directory = tempfile::tempdir()?;
    let base = directory.path().canonicalize()?;

    let error = Matrix::adopt(&base).err().ok_or("it should fail")?;

    assert!(
        error.to_string().contains(&base.display().to_string()),
        "{error}"
    );
    Ok(())
}

#[test]
fn adopting_a_malformed_manifest_fails() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("fixtures.tsv"), "PG-r\tplain git\n")?;

    let error = Matrix::adopt(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("malformed"), "{error}");
    Ok(())
}

#[test]
fn adopting_an_empty_manifest_fails() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("fixtures.tsv"), "")?;

    let error = Matrix::adopt(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("empty"), "{error}");
    Ok(())
}

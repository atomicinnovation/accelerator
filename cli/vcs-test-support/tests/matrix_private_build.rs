use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use tempfile::TempDir;
use vcs_test_support::family::Family;
use vcs_test_support::family::SharedFamily;
use vcs_test_support::fixture_log::FixtureLog;
use vcs_test_support::fixtures::Matrix;
use vcs_test_support::handover::Handover;
use vcs_test_support::shared_build::SharedBuildRequest;
use vcs_test_support::workbench::Workbench;

type TestError = Box<dyn std::error::Error>;

fn shapes(
    matrix: &Matrix,
) -> Result<Vec<(String, String, PathBuf)>, TestError> {
    matrix
        .fixtures
        .iter()
        .map(|fixture| {
            let start = fixture.start.strip_prefix(&matrix.base)?;
            Ok((fixture.key.clone(), fixture.shape.clone(), start.to_owned()))
        })
        .collect()
}

fn shared_build(directory: &Path) -> Result<Matrix, TestError> {
    let tree = directory.join("matrix");
    let request = SharedBuildRequest {
        trees: BTreeMap::from([(Family::Matrix, tree.clone())]),
    };
    let failures =
        request.build(&FixtureLog::at(directory.join("shared.jsonl")));
    assert!(failures.is_empty(), "{failures:?}");
    Ok(Matrix::adopt(&tree)?)
}

enum SharedMatrix {
    Adopted(Box<Workbench>),
    Built(TempDir),
}

impl SharedMatrix {
    fn for_this_run() -> Result<Self, TestError> {
        let handover = Handover::from_environment();
        if handover.shared_tree(Family::Matrix).is_some() {
            Ok(Self::Adopted(Box::new(Workbench::with(handover)?)))
        } else {
            Ok(Self::Built(tempfile::tempdir()?))
        }
    }

    fn shapes(&self) -> Result<Vec<(String, String, PathBuf)>, TestError> {
        match self {
            Self::Adopted(run) => shapes(&*run.matrix()?),
            Self::Built(directory) => shapes(&shared_build(directory.path())?),
        }
    }
}

fn events(log: &Path) -> Result<Vec<Value>, TestError> {
    fs::read_to_string(log)?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

#[test]
fn a_workbench_handed_no_matrix_builds_the_one_a_run_would_share(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let log = directory.path().join("private.jsonl");
    let workbench = Workbench::with(Handover {
        log: FixtureLog::at(&log),
        ..Handover::nothing()
    })?;
    let shared = SharedMatrix::for_this_run()?;

    let private = workbench.matrix()?;

    assert_eq!(shapes(&private)?, shared.shapes()?);
    let recorded = events(&log)?;
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0]["event"], "private-build");
    assert_eq!(recorded[0]["fixtures"], "matrix");
    Ok(())
}

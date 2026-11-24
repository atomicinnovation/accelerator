//! Each stall of the stalled-migration set, built privately against stub
//! migrate executables, so these tests need no migrate binary.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use std::path::PathBuf;

use tempfile::TempDir;
use vcs_test_support::colocation::JjColocation;
use vcs_test_support::fixture_log::FixtureLog;
use vcs_test_support::handover::Handover;
use vcs_test_support::path_scan::files_naming;
use vcs_test_support::repository_state::RepositoryKind;
use vcs_test_support::repository_state::RepositoryState;
use vcs_test_support::stalled_migration::MigrateBinary;
use vcs_test_support::stalled_migration::ParentShape;
use vcs_test_support::stalled_migration::StallKey;
use vcs_test_support::workbench::Workbench;

type TestError = Box<dyn std::error::Error>;

const STALLING: &str = "echo 'MIGRATION STALLED' >&2\nexit 1\n";

struct StubMigrate {
    directory: TempDir,
}

impl StubMigrate {
    fn running(script: &str) -> Result<Self, TestError> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("migrate");
        fs::write(&path, format!("#!/bin/sh\n{script}"))?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        Ok(Self { directory })
    }

    fn binary(&self) -> MigrateBinary {
        MigrateBinary(self.directory.path().join("migrate"))
    }
}

struct LoggedWorkbench {
    _logs: TempDir,
    log: PathBuf,
    workbench: Workbench,
}

impl LoggedWorkbench {
    fn open() -> Result<Self, TestError> {
        let logs = tempfile::tempdir()?;
        let log = logs.path().join("private.jsonl");
        let workbench = Workbench::with(Handover {
            log: FixtureLog::at(&log),
            ..Handover::nothing()
        })?;
        Ok(Self {
            _logs: logs,
            log,
            workbench,
        })
    }

    fn private_builds(&self) -> Result<usize, TestError> {
        Ok(fs::read_to_string(&self.log)?
            .lines()
            .filter(|line| line.contains("\"event\":\"private-build\""))
            .count())
    }
}

const fn parents_of(shape: ParentShape) -> usize {
    match shape {
        ParentShape::SingleParent => 1,
        ParentShape::MergeWithSibling => 2,
    }
}

#[test]
fn every_stall_is_a_jj_repository_on_its_shapes_parents(
) -> Result<(), TestError> {
    let migrate = StubMigrate::running(STALLING)?;

    vcs_test_support::counted::each_of(4, StallKey::ALL, |key| {
        let bench = LoggedWorkbench::open()?;
        let stalls = bench
            .workbench
            .writable_stalled_migrations(key, &migrate.binary())?;
        let stall = stalls.stall(key)?;

        let state =
            RepositoryState::of(&stall.root, bench.workbench.hermetic())?;

        assert_eq!(state.kind, RepositoryKind::Jj(key.colocation), "{key}");
        assert_eq!(
            state.working_copy_parents.len(),
            parents_of(key.shape),
            "{key}"
        );
        assert_ne!(stall.base, stall.sibling, "{key}");
        assert!(files_naming(&stall.root, bench.workbench.path())?.is_empty());
        assert_eq!(bench.private_builds()?, 1, "{key}");
        Ok::<(), TestError>(())
    })
}

fn assert_refused_as_not_stalled(script: &str) -> Result<String, TestError> {
    let migrate = StubMigrate::running(script)?;
    let key = StallKey {
        colocation: JjColocation::Colocated,
        shape: ParentShape::MergeWithSibling,
    };
    let bench = LoggedWorkbench::open()?;

    let error = bench
        .workbench
        .writable_stalled_migrations(key, &migrate.binary())
        .err()
        .ok_or("it should fail")?;

    let message = error.to_string();
    for expected in [
        "the stalled-migration set",
        &key.to_string(),
        "stub says why",
        "cargo nextest run -p accelerator-migrate --test guarded_resume",
    ] {
        assert!(message.contains(expected), "{expected}: {message}");
    }
    Ok(message)
}

#[test]
fn a_migrate_that_succeeds_is_refused_as_not_stalled() -> Result<(), TestError>
{
    let message =
        assert_refused_as_not_stalled("echo 'stub says why' >&2\nexit 0\n")?;

    assert!(message.contains("exited 0"), "{message}");
    Ok(())
}

#[test]
fn a_migrate_that_fails_without_stalling_is_refused() -> Result<(), TestError> {
    let message =
        assert_refused_as_not_stalled("echo 'stub says why' >&2\nexit 1\n")?;

    assert!(message.contains("exited 1"), "{message}");
    Ok(())
}

#[test]
fn a_stall_key_names_its_colocation_and_shape() {
    let names: Vec<String> =
        StallKey::ALL.iter().map(ToString::to_string).collect();

    assert_eq!(
        names,
        [
            "pure-single-parent",
            "pure-merge-with-sibling",
            "colocated-single-parent",
            "colocated-merge-with-sibling",
        ]
    );
}

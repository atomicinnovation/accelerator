//! The zero-spawn property, proven across a crate boundary.
//!
//! Two levels, each through its own black-box binary: the individual taxonomy
//! queries via `vcs-adapters`' reference artefact, and the metadata-read path
//! this crate composes out of them — `VcsBackedRepoFactsProbe` into
//! `vcs_adapters::facts` — via this crate's own. Both idioms for the latter,
//! since `facts` dispatches per `VcsKind`.
//!
//! Scoped to `git`/`jj` specifically rather than "no subprocess at all", because
//! `SystemClock::try_new` spawns `date` unconditionally.
//!
//! Two things are asserted together, since either alone is satisfiable by a
//! broken adapter: no stub marker was written, and every value matches an
//! unrestricted run. An adapter degrading to `None` also writes no marker.
#![cfg(feature = "bash-parity")]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use vcs_test_support::fixtures;
use vcs_test_support::fixtures::Matrix;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::stubs::assert_shadowing_holds;
use vcs_test_support::stubs::reference_artefact;
use vcs_test_support::stubs::Mode;
use vcs_test_support::stubs::Stubs;

type TestError = Box<dyn std::error::Error>;

/// Runs every query against `start` through the reference artefact, optionally
/// with the real binaries stubbed out.
fn query(
    binary: &Path,
    start: &Path,
    stubs: Option<&Stubs>,
) -> Result<String, TestError> {
    let mut command = Command::new(binary);
    command.arg("all").arg(start);
    if let Some(stubs) = stubs {
        stubs.apply(&mut command);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "the reference artefact failed for {}: {}",
            start.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn the_queries_read_git_and_jj_without_spawning_them() -> Result<(), TestError>
{
    // Fail closed on a malformed contract too, or a dropped export silently
    // downgrades the run.
    let mode = Mode::from_environment()?;
    assert_shadowing_holds(mode)?;

    // Adopted rather than built when a root is handed over: building needs the
    // real binaries, which the strong form has already moved out of reach.
    let (_matrix_guard, root) = fixtures::matrix_root()?;
    let matrix = Matrix::build_or_adopt(&root)?;
    let artefact = reference_artefact()?;

    let stub_base = tempfile::Builder::new()
        .prefix("vcs-zero-spawn-")
        .tempdir()?;
    let stubs = Stubs::rooted_at(stub_base.path())?;
    assert!(
        !matrix.fixtures.is_empty(),
        "an empty matrix would pass every assertion below while proving nothing"
    );

    let mut mismatches = String::new();
    for fixture in &matrix.fixtures {
        let unrestricted = query(&artefact, &fixture.start, None)?;
        let stubbed = query(&artefact, &fixture.start, Some(&stubs))?;
        if unrestricted != stubbed {
            writeln!(mismatches, "  {} ({})", fixture.key, fixture.shape)?;
        }
    }

    assert_eq!(
        stubs.spawns()?,
        None,
        "a git or jj subprocess was spawned; the stub recorded it"
    );
    assert!(
        mismatches.is_empty(),
        "values changed when the real binaries were removed, so the adapter \
         degraded rather than reading in-process:\n{mismatches}"
    );

    // Record what stayed reachable, since the strong form is only proven where
    // the mode says so.
    if mode == Mode::PathOnly {
        let reachable = vcs_test_support::stubs::unshadowed_paths()?;
        println!(
            "zero-spawn ran in path-only mode; still reachable by absolute \
             path: {reachable:?}"
        );
    }
    Ok(())
}

/// The metadata-read path's own entry point: `VcsBackedRepoFactsProbe` into
/// `vcs_adapters::facts`, the composition the tests below prove rather than the
/// individual queries the reference artefact above covers.
fn derived_facts(
    start: &Path,
    stubs: Option<&Stubs>,
) -> Result<String, TestError> {
    let mut command =
        Command::new(env!("CARGO_BIN_EXE_corpus-adapters-fixture"));
    command.arg(start);
    if let Some(stubs) = stubs {
        stubs.apply(&mut command);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "the metadata fixture failed for {}: {}",
            start.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn assert_reads_without_spawning(root: &Path) -> Result<(), TestError> {
    let stub_base = tempfile::Builder::new()
        .prefix("corpus-zero-spawn-stubs-")
        .tempdir()?;
    let stubs = Stubs::rooted_at(stub_base.path())?;

    let unrestricted = derived_facts(root, None)?;
    let stubbed = derived_facts(root, Some(&stubs))?;

    assert_eq!(
        stubs.spawns()?,
        None,
        "a git or jj subprocess was spawned; the stub recorded it"
    );
    assert_eq!(
        unrestricted, stubbed,
        "the facts changed when the real binaries were removed, so the \
         metadata-read path degraded rather than reading in-process"
    );
    Ok(())
}

#[test]
fn a_git_metadata_read_spawns_no_subprocess() -> Result<(), TestError> {
    let base = tempfile::Builder::new()
        .prefix("corpus-zero-spawn-git-")
        .tempdir()?;
    let environment = Hermetic::rooted_at(base.path())?;

    let root = base.path().join("repository");
    fs::create_dir_all(&root)?;
    environment.git(&["init", "--quiet"], &root)?;
    environment
        .git(&["commit", "--allow-empty", "--quiet", "-m", "root"], &root)?;

    assert_reads_without_spawning(&root.canonicalize()?)
}

#[test]
fn a_jj_metadata_read_spawns_no_subprocess() -> Result<(), TestError> {
    let base = tempfile::Builder::new()
        .prefix("corpus-zero-spawn-jj-")
        .tempdir()?;
    let environment = Hermetic::rooted_at(base.path())?;
    let root: PathBuf =
        fixtures::pure_jj(&base.path().join("repository"), &environment)?;

    assert_reads_without_spawning(&root)
}

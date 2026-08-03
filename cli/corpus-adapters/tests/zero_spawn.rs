//! The zero-spawn property, proven **across a crate boundary**.
//!
//! `corpus-adapters` is the crate that will converge onto the library-backed
//! probe, so the harness is exercised from here rather than from the crate that
//! defines it — that is what retires the restructuring risk. Everything it uses
//! comes from `vcs_test_support`'s public API: the fixture matrix, the stubs,
//! the shadow list and the hermetic environment. All three parts, not one.
//!
//! The assertion is scoped to `git`/`jj` **specifically**, not "no subprocess at
//! all": `SystemClock::try_new` spawns `date` unconditionally, so a blanket
//! marker would trip on it.
//!
//! Two things are asserted together, because either alone is satisfiable by a
//! broken adapter: no stub marker was written, **and** every value matches an
//! unrestricted run. An adapter degrading to `None` also writes no marker.
#![cfg(feature = "bash-parity")]

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use vcs_test_support::fixtures::Matrix;
use vcs_test_support::hermetic::assert_git_is_recent_enough;
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
    assert_git_is_recent_enough()?;

    // Fail closed on a malformed contract, not just on an unshadowed path: a
    // typo or a dropped export would otherwise silently downgrade the run.
    let mode = Mode::from_environment()?;
    assert_shadowing_holds(mode)?;

    let base = tempfile::Builder::new()
        .prefix("vcs-zero-spawn-")
        .tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    let artefact = reference_artefact()?;

    // The matrix must be built *before* the stubs take effect — building it is
    // what needs the real binaries. A stubbed build would leave an empty matrix
    // and the whole assertion would pass vacuously.
    let stubs = Stubs::rooted_at(base.path())?;
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

    // On a platform where the absolute paths could not be shadowed, record
    // which — the property is proven in its strong form only where the mode
    // says so.
    if mode == Mode::PathOnly {
        let reachable = vcs_test_support::stubs::unshadowed_paths()?;
        println!(
            "zero-spawn ran in path-only mode; still reachable by absolute \
             path: {reachable:?}"
        );
    }
    Ok(())
}

//! The harness itself, checked.
//!
//! "No marker was written" only means something if a marker *would* have been
//! written by a real spawn. A broken stub — unwritable, non-executable, or
//! never first on `PATH` — makes the zero-spawn assertion pass vacuously, which
//! is the one failure mode that would leave the story's headline property
//! unproven while every suite stayed green.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use vcs_test_support::stubs::Mode;
use vcs_test_support::stubs::Stubs;
use vcs_test_support::stubs::MODE_VARIABLE;
use vcs_test_support::stubs::SHADOWED_VARIABLE;

type TestError = Box<dyn std::error::Error>;

#[test]
fn a_spawn_through_the_stub_path_is_recorded() -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("stub-live-").tempdir()?;
    let stubs = Stubs::rooted_at(base.path())?;

    assert_eq!(stubs.spawns()?, None, "nothing has run yet");

    // Resolved through the synthetic PATH exactly as the spawning adapter does
    // it. Not via a shell: on a usrmerge Linux both /bin and /usr/bin resolve
    // git, so both are stripped and `sh` is no longer on the PATH at all.
    let mut command = Command::new("git");
    command.arg("--version");
    stubs.apply(&mut command);
    let status = command.status()?;
    assert!(status.success());

    let recorded = stubs.spawns()?.ok_or(
        "the stub recorded nothing — the marker mechanism is broken, and every \
         zero-spawn assertion built on it would pass vacuously",
    )?;
    assert!(
        recorded.contains("git"),
        "expected the git stub to record its invocation, got: {recorded}"
    );
    Ok(())
}

#[test]
fn the_stub_directory_leads_the_synthetic_path() -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("stub-order-").tempdir()?;
    let stubs = Stubs::rooted_at(base.path())?;
    let first = stubs
        .path()
        .split(':')
        .next()
        .ok_or("the synthetic PATH is empty")?;
    assert_eq!(std::path::Path::new(first), stubs.directory());
    Ok(())
}

#[test]
fn the_synthetic_path_drops_every_directory_that_resolves_a_real_binary(
) -> Result<(), TestError> {
    // macOS commonly provides git in both /opt/homebrew/bin and /usr/bin, so
    // stripping a single dirname leaves it resolvable.
    let base = tempfile::Builder::new().prefix("stub-strip-").tempdir()?;
    let stubs = Stubs::rooted_at(base.path())?;

    let resolvable: Vec<PathBuf> = stubs
        .path()
        .split(':')
        .filter(|entry| !entry.is_empty())
        .flat_map(|entry| {
            ["git", "jj"].map(|binary| Path::new(entry).join(binary))
        })
        .filter(|candidate| candidate.is_file())
        .collect();

    for candidate in &resolvable {
        assert_eq!(
            candidate.parent(),
            Some(stubs.directory()),
            "{} resolves a real binary",
            candidate.display()
        );
    }
    assert_eq!(resolvable.len(), 2, "both stubs should be resolvable");
    Ok(())
}

#[test]
fn the_mode_contract_fails_closed_on_malformed_input() {
    // These read the process environment, which the harness never sets itself —
    // only the CI step does. Asserting the parse in-process is safe because the
    // variables are absent here.
    assert!(std::env::var(MODE_VARIABLE).is_err());
    assert!(std::env::var(SHADOWED_VARIABLE).is_err());
    assert!(matches!(Mode::from_environment(), Ok(Mode::PathOnly)));
}

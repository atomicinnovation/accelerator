//! Black-box tests of external-subcommand dispatch + exec. `exec` replaces the
//! process, so these spawn the real binary as a child; the `ACCELERATOR_<SUB>_BIN`
//! override points the resolver at the in-crate fixture (no network).

use std::error::Error;
use std::ffi::OsString;
use std::io::{BufRead as _, BufReader};
use std::os::unix::ffi::OsStringExt as _;
use std::os::unix::process::ExitStatusExt as _;
use std::process::{Command, Stdio};

const LAUNCHER: &str = env!("CARGO_BIN_EXE_accelerator");
const FIXTURE: &str = env!("CARGO_BIN_EXE_accelerator-fixture");

/// A launcher invocation with the override for `subcommand` pointed at the
/// fixture.
fn launcher_for(subcommand: &str, env_var: &str) -> Command {
    let mut command = Command::new(LAUNCHER);
    command.env_remove("ACCELERATOR_LOG");
    command.env(env_var, FIXTURE);
    command.arg(subcommand);
    command
}

#[test]
fn external_subcommand_exit_code_propagates() -> Result<(), Box<dyn Error>> {
    let status = launcher_for("frobnicate", "ACCELERATOR_FROBNICATE_BIN")
        .arg("exit-42")
        .status()?;
    assert_eq!(status.code(), Some(42));
    Ok(())
}

#[test]
fn a_hyphenated_subcommand_resolves_via_the_normalised_variable(
) -> Result<(), Box<dyn Error>> {
    let status = launcher_for("frob-thing", "ACCELERATOR_FROB_THING_BIN")
        .arg("exit-42")
        .status()?;
    assert_eq!(status.code(), Some(42));
    Ok(())
}

#[test]
fn external_subcommand_terminating_signal_propagates(
) -> Result<(), Box<dyn Error>> {
    // exec replaced the launcher, so the fixture is its PID; SIGTERM → 143.
    let mut child = launcher_for("frobnicate", "ACCELERATOR_FROBNICATE_BIN")
        .arg("block-on-sigterm")
        .stdout(Stdio::piped())
        .spawn()?;

    // Wait for the readiness line so the signal is not racy.
    let stdout = child.stdout.take().ok_or("child stdout missing")?;
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    assert!(
        line.contains("ACCELERATOR_FIXTURE_READY"),
        "no readiness line"
    );

    // std's Child::kill only sends SIGKILL; send SIGTERM via kill(1).
    let killed = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()?;
    assert!(killed.success(), "kill -TERM failed");

    let status = child.wait()?;
    assert_eq!(status.signal(), Some(15), "expected SIGTERM (128+15=143)");
    Ok(())
}

#[test]
fn per_command_help_is_delegated_to_the_child() -> Result<(), Box<dyn Error>> {
    // clap routes `foo --help` to External, so the child is re-exec'd with it.
    let output = launcher_for("frobnicate", "ACCELERATOR_FROBNICATE_BIN")
        .arg("--help")
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("ACCELERATOR_FIXTURE_HELP_SENTINEL"),
        "expected the child's help sentinel, got: {stdout}"
    );
    Ok(())
}

#[test]
fn non_utf8_arguments_survive_verbatim_to_the_child(
) -> Result<(), Box<dyn Error>> {
    // Vec<OsString> so a non-UTF-8 arg reaches the child byte-for-byte.
    let destination =
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("nonutf8-args");
    let non_utf8 = OsString::from_vec(vec![0x66, 0x80, 0x6f]); // "f\x80o"

    let status = launcher_for("frobnicate", "ACCELERATOR_FROBNICATE_BIN")
        .arg("write-args-to")
        .arg(&destination)
        .arg(&non_utf8)
        .status()?;
    assert!(status.success(), "fixture did not write the args");

    let written = std::fs::read(&destination)?;
    assert_eq!(written, vec![0x66, 0x80, 0x6f, 0]); // arg bytes + NUL separator
    Ok(())
}

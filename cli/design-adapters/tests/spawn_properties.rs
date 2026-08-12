//! The spawn's adapter-level properties, against a stub child rather than the
//! real Playwright runner.
//!
//! Every property here — session leadership, stdio redirection, bootstrap-log
//! truncation and mode, the identity handoff and its deterministic EOF — is a
//! property of *spawning a child process*, not of the browser automation the
//! child happens to run. So the child is a stub, and this suite needs no
//! Playwright, no browser and no network. It runs in the default lane.
//!
//! The stub is a `/bin/sh` script written per test. A shell script is the
//! smallest executable that can report its own process group, echo an
//! inherited descriptor and choose its own exit status, and using one here
//! costs nothing: it is a test fixture, not a runtime dependency of the
//! launcher.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use design::executor::daemon_identity::RecordedStartTime;
use design::executor::handoff::Identity;
use design::executor::ports::Spawner as _;
use design_adapters::process::DaemonSpawner;
use design_adapters::process::IDENTITY_FD_VAR;

type TestError = Box<dyn std::error::Error>;

fn write_stub(directory: &Path, body: &str) -> Result<PathBuf, TestError> {
    let path = directory.join("stub-child.sh");
    fs::write(&path, format!("#!/bin/sh\n{body}\n"))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    Ok(path)
}

/// Reads the inherited descriptor into `destination`.
///
/// `<&3` rather than an indirect `<&"$VAR"`, which POSIX `sh` does not
/// portably support — so the stub asserts the variable names the descriptor it
/// then reads, which keeps the contract under test rather than hardcoded past
/// it.
fn read_identity_into(destination: &Path) -> String {
    format!(
        "test \"${IDENTITY_FD_VAR}\" = \"3\" || exit 9\ncat <&3 > {}",
        destination.display()
    )
}

fn spawner(stub: &Path, log: &Path) -> DaemonSpawner {
    DaemonSpawner {
        program: stub.to_path_buf(),
        arguments: Vec::new(),
        bootstrap_log: log.to_path_buf(),
        environment: Vec::new(),
    }
}

/// Waits for a file to hold content, so a test never races the child's write
/// without also hanging if the child never writes.
fn await_content(path: &Path) -> Result<String, TestError> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(body) = fs::read_to_string(path) {
            if !body.trim().is_empty() {
                return Ok(body);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("{} never received content", path.display()).into())
}

/// `nohup … & disown` made the daemon SIGHUP-immune and reparented. `setsid`
/// is what reproduces that, and a differing process group is how it is
/// observed — a Ctrl-C in the caller's session would otherwise reach the
/// daemon mid-crawl.
#[test]
fn the_child_leads_its_own_session_rather_than_the_launcher_s(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let group = work.path().join("group.txt");
    let stub = write_stub(
        work.path(),
        &format!("ps -o pgid= -p $$ > {}", group.display()),
    )?;
    let log = work.path().join("server.bootstrap.log");

    spawner(&stub, &log).spawn()?;

    let child_group: i32 = await_content(&group)?.trim().parse()?;
    let own_group = unsafe { libc::getpgid(0) };
    assert_ne!(
        child_group, own_group,
        "setsid must put the daemon in its own process group"
    );
    Ok(())
}

/// The daemon's chatter must reach the bootstrap log, not the caller's
/// streams — the launcher's own stdout and stderr belong to the client's
/// output.
#[test]
fn the_child_s_streams_are_redirected_to_the_bootstrap_log(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let stub = write_stub(work.path(), "echo out-marker; echo err-marker >&2")?;
    let log = work.path().join("server.bootstrap.log");

    spawner(&stub, &log).spawn()?;

    let captured = await_content(&log)?;
    assert!(captured.contains("out-marker"), "{captured}");
    assert!(captured.contains("err-marker"), "{captured}");
    Ok(())
}

/// Otherwise the timeout envelope's "check this log" points at a previous
/// attempt's output.
#[test]
fn the_bootstrap_log_is_truncated_and_owner_only_before_the_child_writes(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let log = work.path().join("server.bootstrap.log");
    fs::write(&log, "stale output from a previous attempt\n")?;
    fs::set_permissions(&log, fs::Permissions::from_mode(0o644))?;

    let stub = write_stub(work.path(), "echo fresh-marker")?;
    spawner(&stub, &log).spawn()?;

    let captured = await_content(&log)?;
    assert!(
        !captured.contains("stale output"),
        "the log must be truncated: {captured}"
    );
    assert!(captured.contains("fresh-marker"), "{captured}");
    assert_eq!(
        fs::metadata(&log)?.permissions().mode() & 0o777,
        0o600,
        "the log must be owner-only"
    );
    Ok(())
}

/// ADR-0058 names this contract as the port's principal silent-regression
/// risk, so the stub proves it rather than leaving it implicit: the child reads
/// the descriptor it was told about and echoes exactly what the launcher wrote.
#[test]
fn the_child_reads_back_the_identity_the_launcher_wrote(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let echoed = work.path().join("identity.txt");
    let stub = write_stub(work.path(), &read_identity_into(&echoed))?;
    let log = work.path().join("server.bootstrap.log");

    let identity = spawner(&stub, &log).spawn()?;

    let received = await_content(&echoed)?;
    assert_eq!(
        received,
        identity.render(),
        "the child must receive exactly the record the launcher wrote"
    );
    assert_eq!(Identity::parse(&received)?, identity);
    Ok(())
}

/// With the child's inherited write-end copy closed at its own `exec` and the
/// launcher's own copy closed after the write, no writable copy remains — so
/// the child's read terminates instead of blocking. `cat` returning at all is
/// the assertion.
#[test]
fn the_child_s_read_reaches_end_of_input_rather_than_blocking(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let done = work.path().join("done.txt");
    let stub = write_stub(
        work.path(),
        &format!(
            "{}\necho terminated > {}",
            read_identity_into(Path::new("/dev/null")),
            done.display()
        ),
    )?;
    let log = work.path().join("server.bootstrap.log");

    spawner(&stub, &log).spawn()?;

    assert_eq!(await_content(&done)?.trim(), "terminated");
    Ok(())
}

/// The no-partial-record property, proven without a real daemon: a child that
/// never publishes still received all four values first.
#[test]
fn a_child_that_never_publishes_still_received_its_identity(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let echoed = work.path().join("identity.txt");
    let stub = write_stub(
        work.path(),
        &format!("{}\nexit 1", read_identity_into(&echoed)),
    )?;
    let log = work.path().join("server.bootstrap.log");

    let identity = spawner(&stub, &log).spawn()?;

    let received = Identity::parse(&await_content(&echoed)?)?;
    assert_eq!(received, identity);
    assert!(
        !work.path().join("server-info.json").exists(),
        "the stub publishes nothing, which is the point"
    );
    Ok(())
}

/// The launcher observes the child's start time with the same probe the reuse
/// check will later use, so the record is a probe value on any host that can
/// supply one.
#[test]
fn the_recorded_identity_names_the_spawned_child_and_its_start_time(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let stub = write_stub(work.path(), "sleep 2")?;
    let log = work.path().join("server.bootstrap.log");

    let identity = spawner(&stub, &log).spawn()?;

    assert!(identity.pid > 0);
    assert_eq!(identity.token.len(), 32);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    assert!(
        matches!(identity.start_time, RecordedStartTime::Probe(_)),
        "got {:?}",
        identity.start_time
    );
    Ok(())
}

/// A umask, unlike an explicit mode, applies to everything the child creates
/// afterwards — which is how screenshot output lands owner-only today.
#[test]
fn the_child_inherits_an_owner_only_umask() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let created = work.path().join("child-created.txt");
    let stub =
        write_stub(work.path(), &format!("echo x > {}", created.display()))?;
    let log = work.path().join("server.bootstrap.log");

    spawner(&stub, &log).spawn()?;
    await_content(&created)?;

    assert_eq!(
        fs::metadata(&created)?.permissions().mode() & 0o777,
        0o600,
        "a file the child creates with no explicit mode must be owner-only"
    );
    Ok(())
}

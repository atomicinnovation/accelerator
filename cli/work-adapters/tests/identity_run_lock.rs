//! The identity-run lock as a lockdir under the state directory, naming the
//! command that holds it.

use std::fs;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use corpus_adapters::FileCorpusStore;
use work::rekey::outcome::Holder;
use work_adapters::identity_run::HeldIdentityRun;
use work_adapters::identity_run::IdentityRun as _;
use work_adapters::identity_run::IdentityRunAttempt;
use work_adapters::identity_run::IdentityRunLockError;
use work_adapters::identity_run_lock::IdentityRunLock;

type TestError = Box<dyn std::error::Error>;

const DEAD_PID: u32 = 0x7fff_fff0;

fn lockdir(state: &Path) -> std::path::PathBuf {
    state.join("locks/identity-run.lockdir")
}

fn acquired(attempt: IdentityRunAttempt) -> Result<HeldIdentityRun, TestError> {
    match attempt {
        IdentityRunAttempt::Acquired(held) => Ok(held),
        IdentityRunAttempt::HeldBy(_) => Err("the lock was held".into()),
    }
}

fn this_process(command: &str) -> Holder {
    Holder {
        pid: std::process::id(),
        command: Some(command.to_owned()),
    }
}

#[test]
fn the_lockdir_lives_under_the_state_directory() -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let state = root.path().join(".accelerator/state");
    let writer = FileCorpusStore::new(root.path());
    let lock = IdentityRunLock::new(&state, &writer);

    let held = acquired(lock.try_acquire("sync")?)?;

    assert!(lockdir(&state).is_dir());
    drop(held);
    assert!(!lockdir(&state).exists());
    Ok(())
}

#[test]
fn the_lock_creates_its_parent_and_ignore_rule() -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let state = root.path().join(".accelerator/state");
    let writer = FileCorpusStore::new(root.path());
    let lock = IdentityRunLock::new(&state, &writer);

    let _held = acquired(lock.try_acquire("sync")?)?;

    assert_eq!(fs::read_to_string(state.join("locks/.gitignore"))?, "*\n");
    Ok(())
}

#[test]
fn the_command_file_lives_inside_the_lockdir_and_goes_with_it(
) -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let state = root.path().join(".accelerator/state");
    let writer = FileCorpusStore::new(root.path());
    let lock = IdentityRunLock::new(&state, &writer);

    let held = acquired(lock.try_acquire("create")?)?;
    assert_eq!(
        fs::read_to_string(lockdir(&state).join("command"))?,
        "create"
    );
    drop(held);
    assert!(!lockdir(&state).join("command").exists());

    fs::create_dir_all(lockdir(&state))?;
    fs::write(
        lockdir(&state).join("owner.0123456789abcdef"),
        DEAD_PID.to_string(),
    )?;
    fs::write(lockdir(&state).join("command"), "sync")?;
    let reclaimed = acquired(lock.try_acquire("rekey")?)?;
    assert_eq!(
        fs::read_to_string(lockdir(&state).join("command"))?,
        "rekey"
    );
    drop(reclaimed);
    Ok(())
}

#[test]
fn try_acquire_reports_the_holder_without_blocking() -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let state = root.path().join(".accelerator/state");
    let writer = FileCorpusStore::new(root.path());
    let lock = IdentityRunLock::new(&state, &writer);
    let _held = acquired(lock.try_acquire("sync")?)?;

    let started = Instant::now();
    let attempt = lock.try_acquire("create")?;

    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(matches!(
        attempt,
        IdentityRunAttempt::HeldBy(Some(holder)) if holder == this_process("sync")
    ));
    Ok(())
}

#[test]
fn a_holder_without_a_command_file_is_named_by_its_pid_alone(
) -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let state = root.path().join(".accelerator/state");
    let writer = FileCorpusStore::new(root.path());
    let lock = IdentityRunLock::new(&state, &writer);
    let _held = acquired(lock.try_acquire("sync")?)?;
    fs::remove_file(lockdir(&state).join("command"))?;

    let attempt = lock.try_acquire("create")?;

    assert!(matches!(
        attempt,
        IdentityRunAttempt::HeldBy(Some(Holder { pid, command: None }))
            if pid == std::process::id()
    ));
    Ok(())
}

#[test]
fn acquire_within_times_out_at_its_ceiling() -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let state = root.path().join(".accelerator/state");
    let writer = FileCorpusStore::new(root.path());
    let lock = IdentityRunLock::new(&state, &writer);
    let _held = acquired(lock.try_acquire("sync")?)?;

    let ceiling = Duration::from_millis(100);
    let started = Instant::now();
    let outcome = lock.acquire_within("create", ceiling);

    assert!(started.elapsed() >= ceiling);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(matches!(
        outcome,
        Err(IdentityRunLockError::Timeout(Some(holder)))
            if holder == this_process("sync")
    ));
    Ok(())
}

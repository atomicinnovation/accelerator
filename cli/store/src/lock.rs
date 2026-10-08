//! The mkdir-based advisory lock.
//!
//! `mkdir` is the exclusive-acquisition mutex (POSIX, unlike `flock`), the
//! lockdir carries an `owner.<nonce>` PID sentinel, a dead holder is
//! reclaimed single-winner, and contention backs off with jitter up to an
//! injectable ceiling. POSIX-only, matching the darwin + musl target set.
//!
//! The lockdir format also recognises a legacy nonce-less `owner` sentinel:
//! nothing writes it any more, but a holder that died before the nonce upgrade
//! may have left one, so reclaim still reads it.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use rand::Rng as _;
use rustix::io::Errno;
use rustix::process::{test_kill_process, Pid};

/// Sentinel filename prefixes. Each is followed by `.<nonce>` — the nonce
/// is what binds a reclaim to the exact holder it read; see `reclaim_if_stale`.
const OWNER: &str = "owner";
const RECLAIMING: &str = "reclaiming";

/// A lock-acquisition failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    Timeout { path: String },
    NotWritable { path: String },
    Io { path: String, detail: String },
}

impl std::fmt::Display for LockError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { path } => {
                write!(formatter, "lock acquisition timed out on '{path}'")
            }
            Self::NotWritable { path } => {
                write!(formatter, "cannot write under '{path}': not writable")
            }
            Self::Io { path, detail } => {
                write!(formatter, "I/O error on '{path}': {detail}")
            }
        }
    }
}

impl std::error::Error for LockError {}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy)]
pub struct LockOptions {
    pub ceiling_ms: u64,
    pub base_ms: u64,
    pub cap_ms: u64,
}

impl Default for LockOptions {
    fn default() -> Self {
        Self {
            ceiling_ms: 300_000,
            base_ms: 4,
            cap_ms: 256,
        }
    }
}

pub struct LockGuard {
    lockdir: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.lockdir);
    }
}

/// Acquires the mkdir-lock at `lockdir`, blocking (with jittered backoff)
/// until it is free or `opts.ceiling_ms` is exceeded.
///
/// # Errors
/// [`LockError::NotWritable`] or [`LockError::Io`] when `mkdir` itself
/// fails for a reason other than the lockdir already existing;
/// [`LockError::Timeout`] when contention outlives `opts.ceiling_ms`.
pub fn acquire(
    lockdir: &Path,
    opts: LockOptions,
) -> Result<LockGuard, LockError> {
    acquire_with(lockdir, opts, process_is_alive)
}

fn acquire_with(
    lockdir: &Path,
    opts: LockOptions,
    is_alive: impl Fn(i32) -> bool,
) -> Result<LockGuard, LockError> {
    let mut waited_ms = 0u64;
    let mut base_ms = opts.base_ms;
    loop {
        match fs::create_dir(lockdir) {
            Ok(()) => return claim(lockdir),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                reclaim_if_stale(lockdir, &is_alive);
                if waited_ms > opts.ceiling_ms {
                    return Err(LockError::Timeout {
                        path: lockdir.display().to_string(),
                    });
                }
                let jitter = jitter_ms(base_ms);
                std::thread::sleep(Duration::from_millis(jitter));
                waited_ms += jitter;
                if base_ms < opts.cap_ms {
                    base_ms = (base_ms * 2).min(opts.cap_ms);
                }
            }
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                return Err(LockError::NotWritable {
                    path: lockdir.display().to_string(),
                });
            }
            Err(error) => {
                return Err(LockError::Io {
                    path: lockdir.display().to_string(),
                    detail: error.to_string(),
                });
            }
        }
    }
}

/// The PID recorded by the lockdir's current holder.
///
/// `None` when that is ambiguous: no lockdir, no `owner.<nonce>` sentinel or
/// more than one, an unparseable PID, or only a reclaim-in-flight or legacy
/// nonce-less sentinel.
#[must_use]
pub fn holder_pid(lockdir: &Path) -> Option<u32> {
    let name = sole_sentinel(lockdir, OWNER)?;
    u32::try_from(sentinel_pid(lockdir, &name)?).ok()
}

/// Take ownership of a freshly created lockdir by dropping in its sentinel.
///
/// The sentinel's *name* carries a fresh nonce, not just its contents. That is
/// what makes reclaim safe: it gives every holder a filename no other holder
/// will ever present, so a reclaimer acting on a stale read can only ever
/// address the exact holder it read (see `reclaim_if_stale`).
fn claim(lockdir: &Path) -> Result<LockGuard, LockError> {
    let sentinel = format!("{OWNER}.{:016x}", rand::random::<u64>());
    let owner = std::process::id().to_string();
    if let Err(error) = fs::write(lockdir.join(sentinel), owner) {
        let _ = fs::remove_dir_all(lockdir);
        return Err(LockError::Io {
            path: lockdir.display().to_string(),
            detail: error.to_string(),
        });
    }
    Ok(LockGuard {
        lockdir: lockdir.to_path_buf(),
    })
}

/// Reclaim the lockdir when — and only when — its owner is confirmed dead.
///
/// Reclaim is inherently a read-then-act, and the read can go stale: between
/// observing a dead owner and acting on it, another contender can reclaim,
/// `mkdir`, and claim the lock for itself. The act must therefore be *bound to
/// the holder that was read*, or it lands on the new, live one.
///
/// Two earlier shapes both got this wrong. Renaming the whole lockdir aside
/// moved a live holder's directory and freed its path, letting a second
/// acquirer take the same lock. Renaming a fixed-name `owner` sentinel aside
/// failed the same way for the same reason — the new holder's sentinel has the
/// same name, so the stale rename still matched it. Since the guarded
/// operation is a read-modify-write of an append-only store, either is a lost
/// write, not merely a lock anomaly.
///
/// So the sentinel is named `owner.<nonce>` per holder, and the reclaim step
/// renames *that exact name* aside. `rename` is atomic, which makes the step
/// single-winner among contenders that read the same holder, and the nonce
/// makes it a no-op (`ENOENT`) against any *other* holder. The directory is
/// only removed by the contender that won the rename, and only while it is
/// still the directory that was read: winning proves `owner.<nonce>` was
/// present, and a new holder can only exist by way of a `mkdir` that the
/// surviving directory forbids.
fn reclaim_if_stale(lockdir: &Path, is_alive: &impl Fn(i32) -> bool) {
    let Some(sentinel) = reclaimable(lockdir, is_alive) else {
        return;
    };
    // The winner's own PID goes in the NAME, put there atomically by the
    // rename itself — see `reclaimable` for what reads it back.
    let claimed = format!(
        "{RECLAIMING}.{}.{:016x}",
        std::process::id(),
        rand::random::<u64>()
    );
    if fs::rename(lockdir.join(&sentinel), lockdir.join(claimed)).is_ok() {
        let _ = fs::remove_dir_all(lockdir);
    }
}

/// The sentinel a reclaim may take, if any.
fn reclaimable(
    lockdir: &Path,
    is_alive: &impl Fn(i32) -> bool,
) -> Option<String> {
    // The ordinary case: a holder whose PID is dead. `legacy_owner` is the
    // same case in the nonce-less format nothing writes any more — it can only
    // be an orphan left by a holder that died before the upgrade.
    if let Some((name, _)) = dead_sentinel(lockdir, OWNER, is_alive) {
        return Some(name);
    }
    if let Some((name, _)) = legacy_owner(lockdir, is_alive) {
        return Some(name);
    }
    // A `reclaiming.<pid>.<nonce>` is a reclaim between its rename and its
    // removal. Taking it over is only safe once the RECLAIMER is dead, and
    // that PID is read from the name rather than the file: the rename that
    // created the name published it atomically, whereas a write afterwards
    // would leave a window in which the name says nothing.
    //
    // Gating on the reclaimer's liveness — not the original holder's — is what
    // keeps removal single-owner. Re-nonce alone does not: two contenders that
    // each win a rename of a *different* name (one of `owner.<n>`, one of the
    // `reclaiming.<n>` it became) would both believe they may remove, and the
    // slower one's removal would land on whatever fresh, live lockdir the
    // faster one's had already been replaced by. A live reclaimer is left
    // alone; a dead one cannot still be about to remove anything.
    let name = sole_sentinel(lockdir, RECLAIMING)?;
    let pid = name.split('.').nth(1)?.parse::<i32>().ok()?;
    (!is_alive(pid)).then_some(name)
}

/// The single `<prefix>.…` entry in `lockdir`, or `None` when there is no such
/// entry or more than one — which no correct writer produces, so it is read as
/// "do not touch" rather than guessed at.
fn sole_sentinel(lockdir: &Path, prefix: &str) -> Option<String> {
    let mut found = None;
    for entry in fs::read_dir(lockdir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{prefix}.")) {
            if found.is_some() {
                return None;
            }
            found = Some(name);
        }
    }
    found
}

/// The `<prefix>.<nonce>` sentinel and its PID, only when that PID is present,
/// parseable, and confirmed dead. A missing sentinel (holder mid-acquisition),
/// or an empty or unparseable one, yields `None` — treated as a *live* holder,
/// so PID-reuse or the acquisition window can never break a genuinely held
/// lock. Ditto a directory carrying more than one matching sentinel, which no
/// correct writer produces.
fn dead_sentinel(
    lockdir: &Path,
    prefix: &str,
    is_alive: &impl Fn(i32) -> bool,
) -> Option<(String, i32)> {
    let name = sole_sentinel(lockdir, prefix)?;
    let pid = sentinel_pid(lockdir, &name)?;
    (!is_alive(pid)).then_some((name, pid))
}

/// The nonce-less `owner` sentinel, only when its PID is confirmed dead. See
/// the call site for why this format is still read but never written.
fn legacy_owner(
    lockdir: &Path,
    is_alive: &impl Fn(i32) -> bool,
) -> Option<(String, i32)> {
    let pid = sentinel_pid(lockdir, OWNER)?;
    (!is_alive(pid)).then_some((OWNER.to_owned(), pid))
}

fn sentinel_pid(lockdir: &Path, name: &str) -> Option<i32> {
    fs::read_to_string(lockdir.join(name))
        .ok()?
        .trim()
        .parse::<i32>()
        .ok()
}

/// `kill(pid, 0)` via rustix: signalable → alive; `EPERM` → alive (exists but
/// not signalable by this process); only a confirmed `ESRCH` (or any other
/// error) is treated as dead. A non-positive PID is never a real holder, so it
/// is treated as live rather than reclaimed.
fn process_is_alive(pid: i32) -> bool {
    if pid <= 0 {
        return true;
    }
    let Some(pid) = Pid::from_raw(pid) else {
        return true;
    };
    matches!(test_kill_process(pid), Ok(()) | Err(Errno::PERM))
}

fn jitter_ms(base_ms: u64) -> u64 {
    rand::rng().random_range(1..=base_ms.max(1))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use tempfile::TempDir;

    use super::{
        acquire, acquire_with, claim, holder_pid, jitter_ms, process_is_alive,
        reclaim_if_stale, LockError, LockGuard, LockOptions, OWNER, RECLAIMING,
    };

    type TestError = Box<dyn std::error::Error>;

    const STALE_PID: i32 = 0x7fff_fff0;

    fn fast_opts() -> LockOptions {
        LockOptions {
            ceiling_ms: 10,
            base_ms: 1,
            cap_ms: 2,
        }
    }

    /// Seed a held lockdir with a nonce-named owner sentinel, exactly as
    /// `claim` writes one.
    fn seed_held(lockdir: &Path, owner: &str) -> Result<(), TestError> {
        seed_sentinel(lockdir, OWNER, owner)
    }

    fn seed_sentinel(
        lockdir: &Path,
        prefix: &str,
        owner: &str,
    ) -> Result<(), TestError> {
        fs::create_dir_all(lockdir)?;
        fs::write(lockdir.join(format!("{prefix}.0123456789abcdef")), owner)?;
        Ok(())
    }

    /// A reclaim abandoned by the reclaimer at `pid`, in the on-disk shape the
    /// rename leaves behind: the reclaimer's PID lives in the NAME.
    fn seed_abandoned_reclaim(
        lockdir: &Path,
        pid: i32,
    ) -> Result<(), TestError> {
        fs::create_dir_all(lockdir)?;
        fs::write(
            lockdir.join(format!("{RECLAIMING}.{pid}.0123456789abcdef")),
            STALE_PID.to_string(),
        )?;
        Ok(())
    }

    /// The single `<prefix>.<nonce>` entry in a lockdir, if there is one.
    fn sentinel_named(lockdir: &Path, prefix: &str) -> Option<String> {
        fs::read_dir(lockdir)
            .ok()?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.starts_with(&format!("{prefix}.")))
    }

    #[test]
    fn a_dead_owner_is_reclaimed_and_the_lock_acquired() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &STALE_PID.to_string())?;

        let guard = acquire_with(&lockdir, fast_opts(), |_| false)?;
        assert!(sentinel_named(&lockdir, OWNER).is_some());
        drop(guard);
        Ok(())
    }

    fn an_exited_process_pid() -> Result<i32, TestError> {
        let mut child = std::process::Command::new("true").spawn()?;
        child.wait()?;
        Ok(i32::try_from(child.id())?)
    }

    #[test]
    fn an_owner_whose_process_has_exited_is_reclaimed_and_the_lock_acquired(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &an_exited_process_pid()?.to_string())?;

        let guard = acquire(&lockdir, fast_opts())?;
        assert_eq!(holder_pid(&lockdir), Some(std::process::id()));
        drop(guard);
        Ok(())
    }

    #[test]
    fn a_live_owner_is_never_reclaimed() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &STALE_PID.to_string())?;

        let outcome = acquire_with(&lockdir, fast_opts(), |_| true);
        assert!(matches!(outcome, Err(LockError::Timeout { .. })));
        Ok(())
    }

    #[test]
    fn a_missing_owner_is_treated_as_live() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        fs::create_dir_all(&lockdir)?;

        let consulted = AtomicUsize::new(0);
        let outcome = acquire_with(&lockdir, fast_opts(), |_| {
            consulted.fetch_add(1, Ordering::Relaxed);
            false
        });
        assert!(matches!(outcome, Err(LockError::Timeout { .. })));
        assert_eq!(consulted.load(Ordering::Relaxed), 0);
        Ok(())
    }

    #[test]
    fn an_empty_owner_is_treated_as_live() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, "")?;

        let consulted = AtomicUsize::new(0);
        let outcome = acquire_with(&lockdir, fast_opts(), |_| {
            consulted.fetch_add(1, Ordering::Relaxed);
            false
        });
        assert!(matches!(outcome, Err(LockError::Timeout { .. })));
        assert_eq!(consulted.load(Ordering::Relaxed), 0);
        Ok(())
    }

    #[test]
    fn an_unparseable_owner_is_treated_as_live() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, "not-a-pid")?;

        let consulted = AtomicUsize::new(0);
        let outcome = acquire_with(&lockdir, fast_opts(), |_| {
            consulted.fetch_add(1, Ordering::Relaxed);
            false
        });
        assert!(matches!(outcome, Err(LockError::Timeout { .. })));
        assert_eq!(consulted.load(Ordering::Relaxed), 0);
        Ok(())
    }

    #[test]
    fn a_permanently_held_lock_times_out_under_the_ceiling(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &STALE_PID.to_string())?;

        let outcome = acquire_with(&lockdir, fast_opts(), |_| true);
        assert!(matches!(outcome, Err(LockError::Timeout { .. })));
        Ok(())
    }

    #[test]
    fn a_non_already_exists_error_fails_fast() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let file = dir.path().join("regular");
        fs::write(&file, b"i am a file")?;
        let lockdir = file.join("child.lockdir");

        let outcome = acquire_with(&lockdir, LockOptions::default(), |_| true);
        assert!(matches!(outcome, Err(LockError::Io { .. })));
        Ok(())
    }

    #[test]
    fn contended_reclaim_has_a_single_winner() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &STALE_PID.to_string())?;

        let is_alive = |pid: i32| pid != STALE_PID;
        let outcomes = std::thread::scope(
            |scope| -> Result<Vec<Result<LockGuard, LockError>>, TestError> {
                let handles: Vec<_> = (0..2)
                    .map(|_| {
                        scope.spawn(|| {
                            acquire_with(&lockdir, fast_opts(), is_alive)
                        })
                    })
                    .collect();
                let mut results = Vec::new();
                for handle in handles {
                    results.push(handle.join().map_err(|_| "thread panicked")?);
                }
                Ok(results)
            },
        )?;

        let winners = outcomes.iter().filter(|r| r.is_ok()).count();
        let timeouts = outcomes
            .iter()
            .filter(|r| matches!(r, Err(LockError::Timeout { .. })))
            .count();
        assert_eq!(winners, 1, "exactly one acquirer");
        assert_eq!(timeouts, 1, "exactly one timeout");
        Ok(())
    }

    /// The reclaim race, forced rather than sampled.
    ///
    /// Two acquirers can both read the same dead `owner` before either acts.
    /// The `is_alive` hook is the injection point: it fires *between* the
    /// liveness check and the rename, which is exactly the window a second
    /// acquirer needs to reclaim, `mkdir`, and claim the lock for itself. The
    /// first acquirer's rename then lands on a *live* lockdir. Sampling this
    /// with real threads takes thousands of runs (CI has hit it roughly once
    /// in ten full runs); driving the hook pins it deterministically.
    #[test]
    fn a_stale_reclaim_does_not_displace_a_live_holder() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &STALE_PID.to_string())?;

        let winner = std::cell::RefCell::new(None);
        let raced = std::cell::Cell::new(false);
        let is_alive = |pid: i32| {
            // Run the whole of the *other* acquirer the first time we are
            // asked, so the rename below is issued against its live lockdir.
            if pid == STALE_PID && !raced.replace(true) {
                *winner.borrow_mut() =
                    Some(acquire_with(&lockdir, fast_opts(), |p| {
                        p != STALE_PID
                    }));
            }
            pid != STALE_PID
        };

        reclaim_if_stale(&lockdir, &is_alive);

        let held = winner.into_inner();
        assert!(
            matches!(held, Some(Ok(_))),
            "the racing acquirer ran and took the lock"
        );
        // The live holder must still own the path: if the stale reclaim moved
        // it aside, `create_dir` succeeds here and two acquirers hold at once.
        assert!(
            fs::create_dir(&lockdir).is_err(),
            "a live holder's lockdir was displaced by a stale reclaim, so a \
             second acquirer can take the same lock"
        );
        Ok(())
    }

    #[test]
    fn a_nonce_less_owner_left_by_an_older_holder_is_still_reclaimed(
    ) -> Result<(), TestError> {
        // The pre-nonce on-disk format. An orphan in this shape can outlive an
        // upgrade, and if it were unreadable it would wedge the lock for the
        // full ceiling on every acquisition.
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        fs::create_dir_all(&lockdir)?;
        fs::write(lockdir.join(OWNER), STALE_PID.to_string())?;

        let guard =
            acquire_with(&lockdir, fast_opts(), |pid| pid != STALE_PID)?;

        assert!(sentinel_named(&lockdir, OWNER).is_some());
        drop(guard);
        Ok(())
    }

    #[test]
    fn a_nonce_less_owner_that_is_alive_is_never_reclaimed(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        fs::create_dir_all(&lockdir)?;
        fs::write(lockdir.join(OWNER), STALE_PID.to_string())?;

        let outcome = acquire_with(&lockdir, fast_opts(), |_| true);

        assert!(matches!(outcome, Err(LockError::Timeout { .. })));
        Ok(())
    }

    #[test]
    fn a_reclaim_that_died_mid_flight_is_finished_by_the_next_acquirer(
    ) -> Result<(), TestError> {
        // The crash window: the sentinel was renamed aside but the directory
        // was never removed. Left alone this reads as "held by nobody" and
        // would wedge the lock permanently.
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_abandoned_reclaim(&lockdir, STALE_PID)?;

        let guard =
            acquire_with(&lockdir, fast_opts(), |pid| pid != STALE_PID)?;

        assert!(sentinel_named(&lockdir, OWNER).is_some());
        assert!(sentinel_named(&lockdir, RECLAIMING).is_none());
        drop(guard);
        Ok(())
    }

    #[test]
    fn a_reclaim_whose_reclaimer_is_still_alive_is_left_alone(
    ) -> Result<(), TestError> {
        // Same shape, but the reclaimer named in the sentinel is alive, so the
        // reclaim is in flight rather than abandoned. Taking it over would give
        // two contenders the right to remove the same directory — the recovery
        // path must not become a second way to break a held lock.
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_abandoned_reclaim(&lockdir, STALE_PID)?;

        let outcome = acquire_with(&lockdir, fast_opts(), |_| true);

        assert!(matches!(outcome, Err(LockError::Timeout { .. })));
        assert!(sentinel_named(&lockdir, RECLAIMING).is_some());
        Ok(())
    }

    #[test]
    fn claim_releases_the_lockdir_when_the_owner_write_fails(
    ) -> Result<(), TestError> {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        fs::create_dir_all(&lockdir)?;
        // Read-only, so dropping the sentinel in fails. (The sentinel name
        // carries a nonce, so it cannot be blocked by pre-creating a colliding
        // entry — the permission bits are what make the write fail.)
        fs::set_permissions(&lockdir, fs::Permissions::from_mode(0o555))?;

        let outcome = claim(&lockdir);

        assert!(matches!(outcome, Err(LockError::Io { .. })));
        assert!(!lockdir.exists());
        Ok(())
    }

    #[test]
    fn the_current_process_reads_as_alive() {
        let pid = i32::try_from(std::process::id()).unwrap_or(1);
        assert!(process_is_alive(pid));
    }

    #[test]
    fn a_non_positive_pid_is_treated_as_live() {
        assert!(process_is_alive(0));
        assert!(process_is_alive(-1));
    }

    #[test]
    fn jitter_stays_within_one_and_base() {
        for base in [1u64, 4, 16, 256] {
            for _ in 0..1000 {
                let value = jitter_ms(base);
                assert!(
                    (1..=base).contains(&value),
                    "{value} out of 1..={base}"
                );
            }
        }
    }

    #[test]
    fn the_guard_drop_removes_the_lockdir() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");

        let guard = acquire_with(&lockdir, fast_opts(), |_| true)?;
        assert!(lockdir.exists());
        drop(guard);
        assert!(!lockdir.exists());

        let again = acquire_with(&lockdir, fast_opts(), |_| true)?;
        drop(again);
        Ok(())
    }

    #[test]
    fn a_timeout_names_the_contended_lockdir() {
        let error = LockError::Timeout {
            path: "/w/.lockdir".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "lock acquisition timed out on '/w/.lockdir'"
        );
    }

    #[test]
    fn an_unwritable_parent_names_the_lockdir() {
        let error = LockError::NotWritable {
            path: "/w/.lockdir".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "cannot write under '/w/.lockdir': not writable"
        );
    }

    #[test]
    fn an_io_failure_names_the_lockdir_and_its_cause() {
        let error = LockError::Io {
            path: "/w/.lockdir".to_owned(),
            detail: "Not a directory".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "I/O error on '/w/.lockdir': Not a directory"
        );
    }

    #[test]
    fn a_read_only_parent_is_not_writable() -> Result<(), TestError> {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = TempDir::new()?;
        let parent = dir.path().join("sealed");
        fs::create_dir(&parent)?;
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o555))?;
        let lockdir = parent.join("log.lockdir");

        let outcome = acquire_with(&lockdir, fast_opts(), |_| true);

        fs::set_permissions(&parent, fs::Permissions::from_mode(0o755))?;
        assert!(matches!(
            outcome,
            Err(LockError::NotWritable { path }) if path == lockdir.display().to_string()
        ));
        Ok(())
    }

    #[test]
    fn the_holder_is_the_pid_in_the_owner_sentinel() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, "4242\n")?;

        assert_eq!(holder_pid(&lockdir), Some(4242));
        Ok(())
    }

    #[test]
    fn an_absent_lockdir_has_no_holder() -> Result<(), TestError> {
        let dir = TempDir::new()?;

        assert_eq!(holder_pid(&dir.path().join("log.lockdir")), None);
        Ok(())
    }

    #[test]
    fn duplicate_owner_sentinels_have_no_holder() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, "4242")?;
        fs::write(lockdir.join(format!("{OWNER}.fedcba9876543210")), "4343")?;

        assert_eq!(holder_pid(&lockdir), None);
        Ok(())
    }

    #[test]
    fn an_unparseable_owner_has_no_holder() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, "not-a-pid")?;

        assert_eq!(holder_pid(&lockdir), None);
        Ok(())
    }

    #[test]
    fn a_reclaim_in_flight_has_no_holder() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_abandoned_reclaim(&lockdir, STALE_PID)?;

        assert_eq!(holder_pid(&lockdir), None);
        Ok(())
    }

    #[test]
    fn a_nonce_less_owner_has_no_holder() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        fs::create_dir_all(&lockdir)?;
        fs::write(lockdir.join(OWNER), "4242")?;

        assert_eq!(holder_pid(&lockdir), None);
        Ok(())
    }
}

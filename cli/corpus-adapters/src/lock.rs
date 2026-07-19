//! The mkdir-based advisory lock: `mkdir` is the exclusive-acquisition mutex
//! (POSIX, unlike `flock`), the lockdir carries an `owner` PID sentinel, a dead
//! holder is reclaimed single-winner, and contention backs off with jitter up
//! to an injectable ceiling. POSIX-only, matching the bash source and the
//! darwin + musl target set.

use std::fs;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use corpus::StoreError;
use rand::Rng as _;

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

pub fn acquire(
    lockdir: &Path,
    opts: LockOptions,
) -> Result<LockGuard, StoreError> {
    acquire_with(lockdir, opts, process_is_alive)
}

fn acquire_with(
    lockdir: &Path,
    opts: LockOptions,
    is_alive: impl Fn(i32) -> bool,
) -> Result<LockGuard, StoreError> {
    let mut waited_ms = 0u64;
    let mut base_ms = opts.base_ms;
    loop {
        match fs::create_dir(lockdir) {
            Ok(()) => return claim(lockdir),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                reclaim_if_stale(lockdir, &is_alive);
                if waited_ms > opts.ceiling_ms {
                    return Err(StoreError::LockTimeout {
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
                return Err(StoreError::NotWritable {
                    path: lockdir.display().to_string(),
                });
            }
            Err(error) => {
                return Err(StoreError::Io {
                    path: lockdir.display().to_string(),
                    detail: error.to_string(),
                });
            }
        }
    }
}

fn claim(lockdir: &Path) -> Result<LockGuard, StoreError> {
    let owner = std::process::id().to_string();
    if let Err(error) = fs::write(lockdir.join("owner"), owner) {
        let _ = fs::remove_dir_all(lockdir);
        return Err(StoreError::Io {
            path: lockdir.display().to_string(),
            detail: error.to_string(),
        });
    }
    Ok(LockGuard {
        lockdir: lockdir.to_path_buf(),
    })
}

fn reclaim_if_stale(lockdir: &Path, is_alive: &impl Fn(i32) -> bool) {
    let Some(pid) = dead_owner(lockdir, is_alive) else {
        return;
    };
    let discard = discard_path(lockdir);
    if fs::rename(lockdir, &discard).is_err() {
        return;
    }
    if dead_owner(&discard, is_alive) == Some(pid) {
        let _ = fs::remove_dir_all(&discard);
    }
}

/// The owner PID only when it is present, parseable, and confirmed dead. A
/// missing, empty (holder mid-acquisition), or unparseable `owner` file yields
/// `None` — treated as a *live* holder, so PID-reuse or the acquisition window
/// can never break a genuinely held lock.
fn dead_owner(lockdir: &Path, is_alive: &impl Fn(i32) -> bool) -> Option<i32> {
    let owner = fs::read_to_string(lockdir.join("owner")).ok()?;
    let pid = owner.trim().parse::<i32>().ok()?;
    (!is_alive(pid)).then_some(pid)
}

fn discard_path(lockdir: &Path) -> PathBuf {
    let nonce = rand::random::<u64>();
    let mut name = lockdir.as_os_str().to_owned();
    name.push(format!(".{}.{nonce:x}.reclaim", std::process::id()));
    PathBuf::from(name)
}

/// `kill(pid, 0)`: `0` → alive; `EPERM` → alive (exists but not signalable by
/// this process); `ESRCH` → gone. Only a confirmed `ESRCH` is treated as dead.
fn process_is_alive(pid: i32) -> bool {
    // SAFETY: `kill` takes two scalar arguments and returns a scalar; it
    // dereferences no pointers and cannot violate memory safety.
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    IoError::last_os_error().raw_os_error() == Some(libc::EPERM)
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

    use corpus::StoreError;
    use tempfile::TempDir;

    use super::{acquire_with, claim, jitter_ms, LockGuard, LockOptions};

    type TestError = Box<dyn std::error::Error>;

    const STALE_PID: i32 = 0x7fff_fff0;

    fn fast_opts() -> LockOptions {
        LockOptions {
            ceiling_ms: 10,
            base_ms: 1,
            cap_ms: 2,
        }
    }

    fn seed_held(lockdir: &Path, owner: &str) -> Result<(), TestError> {
        fs::create_dir_all(lockdir)?;
        fs::write(lockdir.join("owner"), owner)?;
        Ok(())
    }

    #[test]
    fn a_dead_owner_is_reclaimed_and_the_lock_acquired() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &STALE_PID.to_string())?;

        let guard = acquire_with(&lockdir, fast_opts(), |_| false)?;
        assert!(lockdir.join("owner").exists());
        drop(guard);
        Ok(())
    }

    #[test]
    fn a_live_owner_is_never_reclaimed() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &STALE_PID.to_string())?;

        let outcome = acquire_with(&lockdir, fast_opts(), |_| true);
        assert!(matches!(outcome, Err(StoreError::LockTimeout { .. })));
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
        assert!(matches!(outcome, Err(StoreError::LockTimeout { .. })));
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
        assert!(matches!(outcome, Err(StoreError::LockTimeout { .. })));
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
        assert!(matches!(outcome, Err(StoreError::LockTimeout { .. })));
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
        assert!(matches!(outcome, Err(StoreError::LockTimeout { .. })));
        Ok(())
    }

    #[test]
    fn a_non_already_exists_error_fails_fast() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let file = dir.path().join("regular");
        fs::write(&file, b"i am a file")?;
        let lockdir = file.join("child.lockdir");

        let outcome = acquire_with(&lockdir, LockOptions::default(), |_| true);
        assert!(!matches!(outcome, Err(StoreError::LockTimeout { .. })));
        assert!(outcome.is_err());
        Ok(())
    }

    #[test]
    fn contended_reclaim_has_a_single_winner() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        seed_held(&lockdir, &STALE_PID.to_string())?;

        let is_alive = |pid: i32| pid != STALE_PID;
        let outcomes = std::thread::scope(
            |scope| -> Result<Vec<Result<LockGuard, StoreError>>, TestError> {
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
            .filter(|r| matches!(r, Err(StoreError::LockTimeout { .. })))
            .count();
        assert_eq!(winners, 1, "exactly one acquirer");
        assert_eq!(timeouts, 1, "exactly one timeout");
        Ok(())
    }

    #[test]
    fn claim_releases_the_lockdir_when_the_owner_write_fails(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let lockdir = dir.path().join("log.lockdir");
        fs::create_dir_all(lockdir.join("owner"))?;

        let outcome = claim(&lockdir);
        assert!(matches!(outcome, Err(StoreError::Io { .. })));
        assert!(!lockdir.exists());
        Ok(())
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
}

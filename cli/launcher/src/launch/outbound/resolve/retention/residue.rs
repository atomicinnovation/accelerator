//! Crash residue the bootstrap leaves at the top of a cache root: probe files
//! and the lock directories a killed bootstrap never released.

use std::io;
use std::path::Path;

use super::super::filesystem::FileKind;
use super::super::ownership::is_trusted_directory_entry;
use super::super::prune_report::{Count, Removals};
use super::super::retention_windows::is_aged;
use super::ports::CachePorts;
use super::root_entries::{RootEntry, RootListing};

const PID_FILE: &str = "pid";
const PID_LIMIT: u64 = 64;

/// Whether the bootstrap that wrote a pid is still running.
pub trait BootstrapHolderLiveness {
    fn is_alive(&self, pid: i32) -> bool;
}

/// `kill(pid, 0)`: only `ESRCH` means dead. A pid reused by another process
/// reads as alive, which only keeps residue longer.
pub struct KillZeroProbe;

impl BootstrapHolderLiveness for KillZeroProbe {
    #[cfg(unix)]
    fn is_alive(&self, pid: i32) -> bool {
        use rustix::process::{test_kill_process, Pid};
        Pid::from_raw(pid).is_none_or(|pid| {
            test_kill_process(pid) != Err(rustix::io::Errno::SRCH)
        })
    }

    #[cfg(not(unix))]
    fn is_alive(&self, _pid: i32) -> bool {
        true
    }
}

/// Removes aged probe files and abandoned bootstrap lock directories, under
/// the root lock and over the same listing single-file prune read.
pub struct ResidueSweep<'a> {
    root: &'a Path,
    ports: &'a CachePorts,
}

/// What a lock directory looked like when judged abandoned, so a bootstrap
/// that reclaims it before the unlink is noticed.
#[derive(PartialEq, Eq)]
struct AbandonedLock {
    modified_seconds: Option<u64>,
    pid_contents: Option<Vec<u8>>,
}

impl<'a> ResidueSweep<'a> {
    pub const fn new(root: &'a Path, ports: &'a CachePorts) -> Self {
        Self { root, ports }
    }

    pub fn run(&self, listing: &RootListing) -> Removals {
        let mut removals = Removals::default();
        for (name, entry) in listing.classified() {
            match entry {
                RootEntry::Probe => self.sweep_probe(name, &mut removals),
                RootEntry::BootstrapLock => {
                    self.sweep_lock(name, &mut removals);
                }
                _ => {}
            }
        }
        removals
    }

    fn sweep_probe(&self, name: &str, removals: &mut Removals) {
        let path = self.root.join(name);
        let filesystem = self.ports.filesystem.as_ref();
        if filesystem.entry(&path).is_ok_and(|entry| {
            entry.kind == FileKind::File
                && is_aged(entry.modified_seconds, self.now())
        }) {
            removals.remove_file(filesystem, &path, "aged probe", Count::Once);
        }
    }

    /// Judged twice, the second time immediately before the unlink, so a
    /// bootstrap reclaiming the directory in between keeps it.
    fn sweep_lock(&self, name: &str, removals: &mut Removals) {
        let lock = self.root.join(name);
        let pid = lock.join(PID_FILE);
        let Some(judged) = self.abandoned(&lock, &pid) else {
            return;
        };
        if self.abandoned(&lock, &pid).as_ref() != Some(&judged) {
            return;
        }
        let filesystem = self.ports.filesystem.as_ref();
        if removals
            .remove_file(
                filesystem,
                &pid,
                "abandoned bootstrap lock",
                Count::WithItsEntry,
            )
            .left_nothing()
        {
            removals.remove_dir(
                filesystem,
                &lock,
                "abandoned bootstrap lock",
                Count::Once,
            );
        }
    }

    fn abandoned(&self, lock: &Path, pid: &Path) -> Option<AbandonedLock> {
        let filesystem = self.ports.filesystem.as_ref();
        let entry = filesystem.entry(lock).ok()?;
        if !is_trusted_directory_entry(
            lock,
            &entry,
            filesystem,
            self.ports.ownership.as_ref(),
        ) || !is_aged(entry.modified_seconds, self.now())
        {
            return None;
        }
        let pid_contents = match filesystem.read_bounded(pid, PID_LIMIT) {
            Ok(contents) => Some(contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(_) => return None,
        };
        self.holder_is_gone(pid_contents.as_deref())
            .then_some(AbandonedLock {
                modified_seconds: entry.modified_seconds,
                pid_contents,
            })
    }

    /// A missing or empty pid file, or one naming a dead process, has no
    /// holder; one that does not parse is kept.
    fn holder_is_gone(&self, pid_contents: Option<&[u8]>) -> bool {
        let Some(contents) = pid_contents else {
            return true;
        };
        let Ok(contents) = std::str::from_utf8(contents) else {
            return false;
        };
        if contents.trim().is_empty() {
            return true;
        }
        parse_pid(contents)
            .is_some_and(|pid| !self.ports.bootstrap_holders.is_alive(pid))
    }

    fn now(&self) -> u64 {
        self.ports.clock.now_seconds()
    }
}

/// The pid a bootstrap wrote, as `printf '%s\n' "$$"` writes it.
///
/// Only an integer of 1 or more parses: `kill` reads 0 and negative values as
/// process groups.
#[must_use]
pub fn parse_pid(contents: &str) -> Option<i32> {
    let digits = contents.trim_end_matches(|c: char| c.is_ascii_whitespace());
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().filter(|pid| *pid >= 1)
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::sync::Arc;

    use super::super::super::filesystem::testing::ReportingFilesystem;
    use super::super::super::filesystem::{
        CacheFilesystem as _, EntryMetadata, SystemFilesystem,
    };
    use super::super::super::ownership::{Ownership, PrimaryGroup};
    use super::super::root_entries::RootListing;
    use super::super::testing::{
        aged, now, set_mtime, young, LiveHolders, Ports, Root,
    };
    use super::{
        parse_pid, BootstrapHolderLiveness as _, KillZeroProbe, ResidueSweep,
    };

    const LIVE: i32 = 4242;
    const DEAD: i32 = 4343;

    fn sweep(root: &Root, ports: Ports) -> u64 {
        let ports = ports.build();
        let listing = RootListing::read(
            root.path(),
            ports.filesystem.as_ref(),
            ports.steps.as_ref(),
        )
        .expect("listed");
        let removals = ResidueSweep::new(root.path(), &ports).run(&listing);
        assert!(removals.failures.is_empty(), "{:?}", removals.failures);
        removals.reclaimed
    }

    fn holding(now: u64) -> Ports {
        Ports {
            bootstrap_holders: Arc::new(LiveHolders([LIVE].into())),
            ..Ports::at(now)
        }
    }

    fn lock(root: &Root, platform: &str, pid: Option<&str>, modified: u64) {
        let name = format!(".accelerator-lock-{platform}");
        let lock = root.dir(&name, 0o755);
        if let Some(pid) = pid {
            std::fs::write(lock.join("pid"), pid).expect("pid");
        }
        set_mtime(&lock, modified);
    }

    fn lock_exists(root: &Root, platform: &str) -> bool {
        root.exists(&format!(".accelerator-lock-{platform}"))
    }

    #[test]
    fn old_probe_files_are_swept_and_young_ones_kept() {
        let root = Root::new();
        let now = now();
        root.file(".accelerator-probe-11", b"", aged(now));
        root.file(".accelerator-probe-12-3", b"", aged(now));
        root.file(".accelerator-probe-13", b"", young(now));

        assert_eq!(sweep(&root, Ports::at(now)), 2);
        assert!(!root.exists(".accelerator-probe-11"));
        assert!(!root.exists(".accelerator-probe-12-3"));
        assert!(root.exists(".accelerator-probe-13"));
    }

    #[test]
    fn stale_bootstrap_locks_with_empty_or_dead_pids_are_swept() {
        let root = Root::new();
        let now = now();
        lock(&root, "darwin-arm64", None, aged(now));
        lock(&root, "darwin-x64", Some(""), aged(now));
        lock(&root, "linux-arm64", Some(&format!("{DEAD}\n")), aged(now));
        lock(&root, "linux-x64", Some(&format!("{LIVE}\n")), aged(now));

        assert_eq!(sweep(&root, holding(now)), 3);
        assert!(!lock_exists(&root, "darwin-arm64"));
        assert!(!lock_exists(&root, "darwin-x64"));
        assert!(!lock_exists(&root, "linux-arm64"));
        assert!(lock_exists(&root, "linux-x64"));
    }

    #[test]
    fn a_young_lock_with_a_dead_pid_is_kept() {
        let root = Root::new();
        let now = now();
        lock(&root, "linux-x64", Some(&format!("{DEAD}\n")), young(now));

        assert_eq!(sweep(&root, holding(now)), 0);
        assert!(lock_exists(&root, "linux-x64"));
    }

    #[test]
    fn an_unparseable_or_non_positive_pid_keeps_the_lock_directory() {
        for pid in ["abc\n", "0\n", "-1\n"] {
            let root = Root::new();
            let now = now();
            lock(&root, "linux-x64", Some(pid), aged(now));

            assert_eq!(sweep(&root, holding(now)), 0, "{pid:?}");
            assert!(lock_exists(&root, "linux-x64"), "{pid:?}");
        }
    }

    #[test]
    fn a_symlinked_bootstrap_lock_leaves_its_target_untouched() {
        let root = Root::new();
        let now = now();
        let target = root.dir("elsewhere", 0o700);
        std::fs::write(target.join("pid"), format!("{DEAD}\n")).expect("pid");
        set_mtime(&target, aged(now));
        std::os::unix::fs::symlink(
            &target,
            root.join(".accelerator-lock-linux-x64"),
        )
        .expect("symlink");

        assert_eq!(sweep(&root, holding(now)), 0);
        assert!(target.join("pid").exists());
        assert!(lock_exists(&root, "linux-x64"));
    }

    struct Primary(PrimaryGroup);

    impl Ownership for Primary {
        fn effective_uid(&self) -> u32 {
            rustix::process::geteuid().as_raw()
        }

        fn effective_gid(&self) -> u32 {
            rustix::process::getegid().as_raw()
        }

        fn primary_group(&self) -> PrimaryGroup {
            self.0.clone()
        }
    }

    fn group_writable_lock(root: &Root, now: u64) -> ReportingFilesystem {
        lock(root, "linux-x64", Some(&format!("{DEAD}\n")), aged(now));
        let path = root.join(".accelerator-lock-linux-x64");
        let real = SystemFilesystem.entry(&path).expect("lock metadata");
        ReportingFilesystem::over_system().reporting(
            &path,
            EntryMetadata {
                group: rustix::process::getegid().as_raw(),
                mode: 0o040_775,
                ..real
            },
        )
    }

    #[test]
    fn a_private_group_writable_bootstrap_lock_is_swept() {
        let root = Root::new();
        let now = now();
        let filesystem = group_writable_lock(&root, now);

        let reclaimed = sweep(
            &root,
            Ports {
                filesystem: Arc::new(filesystem),
                ownership: Arc::new(Primary(PrimaryGroup::Private)),
                ..holding(now)
            },
        );

        assert_eq!(reclaimed, 1);
        assert!(!lock_exists(&root, "linux-x64"));
    }

    #[test]
    fn a_shared_group_writable_bootstrap_lock_is_kept() {
        let root = Root::new();
        let now = now();
        let filesystem = group_writable_lock(&root, now);

        let reclaimed = sweep(
            &root,
            Ports {
                filesystem: Arc::new(filesystem),
                ownership: Arc::new(Primary(PrimaryGroup::Shared {
                    name: "staff".to_owned(),
                })),
                ..holding(now)
            },
        );

        assert_eq!(reclaimed, 0);
        assert!(lock_exists(&root, "linux-x64"));
    }

    #[test]
    fn kill_zero_probe_maps_errno_to_liveness() {
        let me = i32::try_from(std::process::id()).expect("pid");
        assert!(KillZeroProbe.is_alive(me));

        let mut child =
            std::process::Command::new("true").spawn().expect("spawned");
        let reaped = i32::try_from(child.id()).expect("pid");
        child.wait().expect("reaped");
        assert!(!KillZeroProbe.is_alive(reaped));

        if rustix::process::geteuid().as_raw() != 0 {
            assert!(KillZeroProbe.is_alive(1), "EPERM is alive");
        }
    }

    #[test]
    fn a_pid_written_by_the_bootstrap_parses() {
        assert_eq!(parse_pid("4242\n"), Some(4242));
        assert_eq!(parse_pid("4242"), Some(4242));
        assert_eq!(parse_pid("4242 \r\n"), Some(4242));
    }

    #[test]
    fn only_a_positive_decimal_integer_is_a_pid() {
        for contents in [
            "",
            "\n",
            "0",
            "-1",
            "+5",
            "abc",
            " 42",
            "4 2",
            "0x10",
            "99999999999",
        ] {
            assert_eq!(parse_pid(contents), None, "{contents:?}");
        }
    }
}

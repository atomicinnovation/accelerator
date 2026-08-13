//! When a process started, as seconds since the epoch.
//!
//! Extracted from the visualiser server so the Playwright launcher can share
//! the primitive without inheriting an application crate's dependency graph.
//! `libc` is the only dependency, because this is two platform reads and some
//! arithmetic.
//!
//! The identity *semantics* built on top of this — what a mismatch means, what
//! an unobtainable value means, whether either warrants signalling anything —
//! deliberately do not live here. Two callers make different choices, and the
//! shared part is only the epoch read.

/// Seconds since the epoch at which `pid` started, or `None` when the host
/// cannot say.
///
/// `None` is a real answer, not a failure: a container with an unreadable
/// `/proc`, or a platform that is neither Linux nor macOS, cannot supply the
/// value, and a caller must decide what to do about that rather than being
/// handed a fabricated number.
#[must_use]
pub fn start_time(pid: i32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let btime = read_proc_btime()?;
        #[expect(
            clippy::cast_sign_loss,
            reason = "_SC_CLK_TCK is positive on every supported target; a \
                      non-positive value falls out as an unobtainable start \
                      time via the zero check in the parse"
        )]
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
        start_time_from_proc_stat(&stat, btime, hz)
    }
    #[cfg(target_os = "macos")]
    {
        start_time_via_sysctl(pid)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

/// The epoch start time a `/proc/<pid>/stat` line encodes, given the boot time
/// and clock-tick rate to resolve it against.
///
/// Pure, so the arithmetic is testable on any host rather than only where
/// `/proc` exists.
///
/// Two details are load-bearing. The `comm` field is parenthesised and may
/// itself contain parentheses and spaces, so the scan takes everything after
/// the **last** `)` rather than the first. And the division truncates: a
/// floating-point or `(btime * hz + ticks) / hz` form differs by up to a
/// second, which is the whole tolerance budget the identity check spends on
/// boundary drift.
#[must_use]
pub fn start_time_from_proc_stat(
    stat: &str,
    btime: u64,
    hz: u64,
) -> Option<u64> {
    if hz == 0 {
        return None;
    }
    let (_, tail) = stat.rsplit_once(')')?;
    let ticks: u64 = tail.split_whitespace().nth(19)?.parse().ok()?;
    Some(btime + ticks / hz)
}

#[cfg(target_os = "linux")]
fn read_proc_btime() -> Option<u64> {
    std::fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// Reads `p_starttime` out of `kinfo_proc` via `sysctl(KERN_PROC_PID)`.
///
/// Not `ps -p <pid> -o lstart=`: that prints a wall-clock string with no
/// offset, so converting it needs the UTC offset *at that instant*, and during
/// the repeated hour of a DST fall-back the string names two distinct instants.
/// `p_starttime` is already epoch-based, which sidesteps both, and the
/// subprocess-free read does not race a parallel test harness.
///
/// Nor `proc_pidinfo(PROC_PIDTBSDINFO)`, whose `proc_bsdinfo` `libc` does bind
/// by name: it is denied for processes the caller does not own, and a probe
/// that returns nothing for another user's process lets the launcher adopt a
/// recycled pid instead of rejecting it. `sysctl` answers for any process.
///
/// `libc` does not export `kinfo_proc`, so the read lands in a raw byte buffer.
/// `p_starttime` is a `timeval` at the head of `extern_proc`'s `p_un` union,
/// itself the first field of `kinfo_proc`, so `tv_sec` sits at byte 0.
#[cfg(target_os = "macos")]
fn start_time_via_sysctl(pid: i32) -> Option<u64> {
    // Stable macOS ABI: CTL_KERN=1, KERN_PROC=14, KERN_PROC_PID=1, and
    // sizeof(kinfo_proc) = 648 on every 64-bit target.
    const CTL_KERN: libc::c_int = 1;
    const KERN_PROC: libc::c_int = 14;
    const KERN_PROC_PID: libc::c_int = 1;
    const KINFO_PROC_SIZE: usize = 648;

    let mut buffer = [0u8; KINFO_PROC_SIZE];
    let mut size: usize = KINFO_PROC_SIZE;
    let mib = [CTL_KERN, KERN_PROC, KERN_PROC_PID, pid];
    let returned = unsafe {
        libc::sysctl(
            mib.as_ptr().cast_mut(),
            u32::try_from(mib.len()).ok()?,
            buffer.as_mut_ptr().cast::<libc::c_void>(),
            &raw mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if returned != 0 || size == 0 {
        return None;
    }
    let seconds = i64::from_ne_bytes(buffer.get(..8)?.try_into().ok()?);
    u64::try_from(seconds).ok().filter(|seconds| *seconds > 0)
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    use super::start_time;

    /// The property both callers depend on: a live process's start time does
    /// not move between reads, so a mismatch means a different process.
    #[test]
    fn a_live_process_reports_a_stable_start_time() {
        let me = i32::try_from(std::process::id()).unwrap_or(-1);
        assert_eq!(start_time(me), start_time(me));
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        assert!(
            start_time(me).is_some(),
            "Linux and macOS can both answer for a live process"
        );
    }

    #[test]
    fn an_implausible_pid_is_unobtainable_rather_than_fabricated() {
        assert_eq!(start_time(-1), None);
    }

    /// Ownership must not narrow the read. The launcher reuses a pid whose
    /// start time it cannot observe, so a probe that went blind on another
    /// user's process would let a recycled pid be adopted rather than
    /// rejected — a permission-denied read is indistinguishable from a host
    /// that cannot supply the value at all.
    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn another_user_s_process_still_reports_a_start_time() {
        assert!(
            start_time(1).is_some(),
            "pid 1 is root-owned and always live"
        );
    }

    /// Bounds the value rather than only its stability: a process cannot have
    /// started before the parent that spawned it, nor in the future. Comparing
    /// two distinct pids against the wall clock is what catches a misread
    /// field, where reading the same pid twice would agree on garbage.
    #[test]
    fn a_process_starts_after_its_parent_and_before_now() {
        let me = i32::try_from(std::process::id()).unwrap_or(-1);
        let parent =
            i32::try_from(std::os::unix::process::parent_id()).unwrap_or(-1);
        let (Some(mine), Some(parents)) = (start_time(me), start_time(parent))
        else {
            return;
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());

        assert!(mine >= parents, "started {mine}, parent {parents}");
        assert!(mine <= now, "started {mine}, now {now}");
    }
}

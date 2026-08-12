//! The start-time probe against the locale and timezone hazards that made the
//! shell version fragile.
//!
//! `test-run.sh` sourced `run.sh` and asserted `start_time_of` agreed under
//! `LANG=C` and `de_DE.UTF-8`, because the shell parsed `ps -p <pid> -o lstart=`
//! with a fixed `%a %b %d %H:%M:%S %Y` pattern that localises day and month
//! names — and on `de_DE` even the field order. When it disagreed, every reuse
//! check failed and the launcher respawned the daemon between commands, losing
//! the crawl's page state.
//!
//! That guard covers the exact bug ADR-0058 names, so it survives here. It does
//! **not** additionally assert agreement with `lib/state.js`: the JavaScript
//! probe is gone, so there is one implementation and nothing to agree with. The
//! equality the shell guard needed is replaced by single ownership, which is
//! the stronger property.
//!
//! It also covers an axis the shell version never had. Reading a kernel epoch
//! value is what removes the locale hazard, and `TZ`-independence should be
//! proven rather than assumed.

use design::executor::daemon_identity::ObservedDaemon;
use design::executor::daemon_identity::ObservedStartTime;
use design::executor::ports::ProcessProbe as _;
use design_adapters::HostProbe;

type TestError = Box<dyn std::error::Error>;

fn own_pid() -> i32 {
    i32::try_from(std::process::id()).unwrap_or(-1)
}

/// The probe for this process, or `None` when the host cannot say.
fn observe() -> Option<u64> {
    match HostProbe.observe(own_pid()) {
        ObservedDaemon::Live(ObservedStartTime::Known(seconds)) => {
            Some(seconds)
        }
        _ => None,
    }
}

/// Sets an environment variable for the duration of `body`.
///
/// Safe here because the test runner is nextest, which runs every test in its
/// own process — nothing else is reading the environment concurrently.
fn with_env<T>(name: &str, value: Option<&str>, body: impl FnOnce() -> T) -> T {
    let previous = std::env::var_os(name);
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
    let outcome = body();
    match previous {
        Some(previous) => std::env::set_var(name, previous),
        None => std::env::remove_var(name),
    }
    outcome
}

/// The guard `test-run.sh:44-63` carried, in the language that replaced it.
#[test]
fn the_probe_agrees_across_locales() -> Result<(), TestError> {
    let baseline =
        observe().ok_or("this host cannot probe its own start time")?;

    for locale in [Some("C"), Some("de_DE.UTF-8"), None] {
        let observed =
            with_env("LC_ALL", locale, || with_env("LANG", locale, observe));
        assert_eq!(
            observed,
            Some(baseline),
            "the start time moved under LANG/LC_ALL={locale:?}"
        );
    }
    Ok(())
}

/// An axis the shell guard never had.
///
/// A `ps`-parsing implementation reads a wall-clock string with no offset, so
/// its result depends on the zone it is interpreted in. A `p_starttime` or
/// `/proc` read is already epoch-based and cannot.
///
/// This proves TZ-independence only, not immunity from a DST fall-back: a live
/// process's start time is one fixed instant, so varying `TZ` around a test run
/// cannot exercise the ambiguous repeated hour. That ambiguity is the reason
/// `ps -p <pid> -o lstart=` was rejected in the first place, not something this
/// guard additionally proves.
#[test]
fn the_probe_agrees_across_timezones() -> Result<(), TestError> {
    let baseline =
        observe().ok_or("this host cannot probe its own start time")?;

    // A half-hour offset, because a whole-hour one would still agree under an
    // implementation that merely truncated to the hour.
    for zone in [None, Some("UTC"), Some("Asia/Kolkata")] {
        let observed = with_env("TZ", zone, observe);
        assert_eq!(
            observed,
            Some(baseline),
            "the start time moved under TZ={zone:?}"
        );
    }
    Ok(())
}

/// The tick rate comes from `sysconf`, compiled into the binary, not from the
/// `getconf` program the retired JavaScript shelled out to.
///
/// That program may simply be absent in a distroless or static-musl container,
/// where the previous implementation would fall through to its weakest path.
/// Asserted over the source, because the property is the *absence* of a
/// subprocess, which a passing call cannot demonstrate.
#[test]
fn the_probe_shells_out_to_nothing() {
    const SOURCE: &str = include_str!("../../process-probe/src/lib.rs");

    // The production half only: the tests legitimately read `process::id()`,
    // which spawns nothing.
    let production = SOURCE.split("#[cfg(test)]").next().unwrap_or(SOURCE);

    for forbidden in ["getconf", "Command::new", "process::Command"] {
        assert!(
            !production.contains(forbidden),
            "the start-time probe must not reach for {forbidden}: an absent \
             program would degrade it to a weaker value in exactly the \
             containers that need it"
        );
    }
    assert!(
        production.contains("_SC_CLK_TCK"),
        "the tick rate must come from sysconf, compiled in"
    );
}

/// Two reads of a live process must agree, which is the property the whole
/// PID-recycle guard rests on.
#[test]
fn the_probe_is_stable_for_a_live_process() -> Result<(), TestError> {
    let first = observe().ok_or("this host cannot probe its own start time")?;
    let second = observe().ok_or("the second read found nothing")?;
    assert_eq!(first, second);
    Ok(())
}

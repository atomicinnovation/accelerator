//! What the state directory recorded about a daemon, and what the host
//! observes about it now.
//!
//! Both sides model "cannot say" as a value rather than an absence, because
//! the reuse verdict turns on *why* a start time is missing: a writer that
//! could not probe one, a reader that cannot interpret what was written, and a
//! host whose `/proc` is unreadable are three different situations with three
//! different safe answers.

/// The provenance of the start time a daemon recorded for itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedStartTime {
    /// A kernel probe value, comparable against a fresh probe within the
    /// tolerance.
    Probe(u64),
    /// A wall-clock value. Not comparable against a kernel probe — the two
    /// measure different things — so it carries no PID-recycle guard.
    ///
    /// A record with no source key at all reads as this rather than as
    /// `Probe`: the retired writer fell back to wall-clock on *any* failure,
    /// so a pre-upgrade record's provenance is genuinely unknown, and holding
    /// it to the tolerance on the strength of a guess would respawn the daemon
    /// on every invocation.
    Wallclock(u64),
    /// The launcher's own probe could not read a start time to hand over.
    WriterUnavailable,
    /// No start time was recorded, it did not parse, or its source was a value
    /// this reader does not recognise. The reader cannot validate what it
    /// cannot interpret.
    AbsentOrUnparseable,
}

/// A daemon the state directory claims exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedDaemon {
    pub pid: i32,
    pub start_time: RecordedStartTime,
}

/// What the state directory says.
///
/// `None` and `PidUnparseable` are distinct because they are distinct
/// situations — nothing was ever written, versus something was written and is
/// unusable — and a single label would conflate a cold start with a corrupted
/// record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedState {
    None,
    PidUnparseable,
    Daemon(RecordedDaemon),
}

/// A start time the host can or cannot supply for a live process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedStartTime {
    Known(u64),
    /// `/proc` unreadable, a hardened container, or an unsupported platform.
    /// A value the domain matches on, not an adapter-side failure.
    Unavailable,
}

/// What the host says about the recorded pid right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedDaemon {
    Live(ObservedStartTime),
    Absent,
}

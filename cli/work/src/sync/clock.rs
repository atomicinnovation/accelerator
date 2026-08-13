//! The run-start epoch, as a sync-owned port rather than a widened
//! `corpus::Clock`.
//!
//! `corpus::Clock` has no epoch operation, and adding one would touch a port
//! `corpus-adapters`, `corpus-cli` and `migrate-adapters` all implement,
//! churn `corpus`'s pinned public-api snapshot, and break every existing
//! fake `Clock`. This is declared beside the code that needs it instead.

/// The wall-clock second a sync run started, for the baseline's global
/// timestamp.
///
/// A failure here must leave the baseline timestamp untouched rather than
/// substituting a fallback: the persisted timestamp is the sole gate on the
/// hash-free local short-circuit, and a value derived too large would mark
/// every local file unchanged and turn every remote-side change into an
/// unconditional pull across the whole corpus. No advance means a full
/// re-hash on the next run, which is slow and correct.
pub trait RunClock {
    /// # Errors
    ///
    /// When the current time cannot be derived.
    fn run_start_epoch(&self) -> Result<u64, kernel::Error>;
}

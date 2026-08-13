//! The run-start epoch, as a sync-owned port rather than a widened
//! `corpus::Clock`, which has no epoch operation and is implemented by
//! several crates that do not need one.

/// The wall-clock second a sync run started, for the baseline's global
/// timestamp.
///
/// A failure must leave the baseline timestamp untouched rather than
/// substituting a fallback: the persisted timestamp is the sole gate on the
/// hash-free local short-circuit, so a value derived too large would mark
/// every local file unchanged and turn every remote-side change into an
/// unconditional pull across the whole corpus. No advance costs a full
/// re-hash on the next run and stays correct.
pub trait RunClock {
    /// # Errors
    ///
    /// When the current time cannot be derived.
    fn run_start_epoch(&self) -> Result<u64, kernel::Error>;
}

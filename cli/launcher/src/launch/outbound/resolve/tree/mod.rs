//! Directory-tree artifact resolution: the adapter behind the tree ports.
//!
//! The orchestration over the leaf modules — `acquire`/`query` for the hit path,
//! `materialise` for the cold path, `verify` for the diagnostic walk. Trees are
//! deliberately not routed through `ResolveBinary::resolve`, whose per-exec
//! re-verify is precisely what they are exempt from.

pub mod attestation;
pub mod claims;
pub mod download;
pub mod extract;
pub mod layout;
pub mod lease;
pub mod pins;
pub mod reap;
pub mod seal;
pub mod table;

#[cfg(unix)]
mod resolver;
#[cfg(unix)]
pub use resolver::{
    ExpectedDigests, MaterialiseStep, NoSteps, StepObserver, TreeResolver,
};

use std::time::Duration;

use crate::launch::core::tree::Clock;

/// The production clock: wall time for the reaper's age comparison, a real
/// sleep for the single-flight waiter's poll.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_seconds(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or(0)
    }

    fn sleep_poll_interval(&self) {
        std::thread::sleep(Duration::from_millis(200));
    }
}

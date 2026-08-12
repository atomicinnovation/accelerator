//! The Playwright launcher's decision logic.
//!
//! The chain is CLI → Node with no shell in between, and these 200-odd lines
//! of shell encoded hard-won fixes that regress silently if a port misses
//! them. So the reuse verdict, the identity contract, the forwarding
//! allowlist and the envelope/exit-code taxonomy live here as pure functions
//! over injected ports, where each is exercised directly rather than through a
//! real process and real elapsed time.

pub mod daemon_identity;
pub mod envelope;
pub mod forwardable;
pub mod handoff;
pub mod launch;
pub mod ports;
pub mod reuse;

pub use crate::executor::daemon_identity::ObservedDaemon;
pub use crate::executor::daemon_identity::ObservedStartTime;
pub use crate::executor::daemon_identity::RecordedDaemon;
pub use crate::executor::daemon_identity::RecordedStartTime;
pub use crate::executor::daemon_identity::RecordedState;
pub use crate::executor::envelope::LauncherError;
pub use crate::executor::handoff::Identity;
pub use crate::executor::launch::Launcher;
pub use crate::executor::reuse::Reuse;

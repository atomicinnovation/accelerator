//! The consent policy's adapters that need VCS.
//!
//! An adapter belongs here only when it needs VCS. VCS-free adapters stay in
//! `config-adapters`, which keeps `gix` and `jj-lib` out of its dependents,
//! the launcher and the visualiser server among them.

mod credentials;
mod roots;
mod runner;
mod tracking;

pub use crate::credentials::credential_ports;
pub use crate::roots::repository_roots;
pub use crate::runner::command_runner;
pub use crate::tracking::VcsConfigFileTracking;

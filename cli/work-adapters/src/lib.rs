//! Outbound adapters for the `work` domain crate: filesystem reads,
//! subprocess-shelling modules (`diff`, VCS identity), and the per-tracker
//! remote-payload projection seam.

pub mod diff_shellout;
pub mod filesystem;
pub mod project_remote;

//! Outbound adapters for the `work` domain crate: filesystem reads and the
//! subprocess-shelling modules (`diff`, VCS identity).

pub mod author;
pub mod diff_shellout;
pub mod filesystem;
pub mod sync;

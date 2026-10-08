//! The consent policy's adapters over the VCS tracking port.
//!
//! The port is injected, so this crate depends on the `vcs` domain alone and
//! links neither `gix` nor `jj-lib`; each composition root chooses whether to.

mod credentials;
mod roots;
mod tracking;

pub use self::credentials::credential_ports;
pub use self::roots::repository_roots;
pub use self::tracking::TrackedConfigFile;

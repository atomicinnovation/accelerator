//! The consent policy's adapters over the VCS tracking port.
//!
//! The port is injected, so this crate depends on the VCS domain alone and
//! keeps `gix` and `jj-lib` out of its dependents, the launcher and the
//! visualiser server among them.

mod credentials;
mod roots;
mod tracking;

pub use self::credentials::credential_ports;
pub use self::roots::repository_roots;
pub use self::tracking::TrackedConfigFile;

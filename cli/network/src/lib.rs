//! The network vocabulary several contexts share: the canonical host a URL
//! names, and how reachable the address it names is.
//!
//! Pure logic only. Nothing here resolves a name or opens a connection.

pub mod authority;
pub mod host;
pub mod host_reach;

pub use crate::authority::Authority;
pub use crate::authority::AuthorityError;
pub use crate::host::Host;
pub use crate::host::HostError;
pub use crate::host_reach::HostReach;

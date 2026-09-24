//! The policy every provider client shares: the bounded-retry schedule,
//! identifier safety, the transport bounds, the ceiling-string conversion, and
//! the `<tracker>.pull` discovery-scope and `<tracker>.push` write-bound config
//! blocks.
//!
//! Admission is deliberately narrow — policy shared by two or more provider
//! clients, with no transport and no provider specifics — so this does not
//! become the place a utility goes when it fits nowhere else. The clients may
//! not import each other; a common downward dependency is how they share a
//! rule without doing so.

pub mod block;
pub mod ceiling;
pub mod identifier;
pub mod mime;
pub mod pull;
pub mod push;
pub mod retry;
pub mod transport;

pub use crate::identifier::identifier_is_safe;
pub use crate::identifier::IdentifierRefusal;
pub use crate::mime::sniff;
pub use crate::retry::ClockJitter;
pub use crate::retry::Jitter;
pub use crate::retry::RetryPolicy;
pub use crate::retry::Sleeper;
pub use crate::retry::SystemSleeper;
pub use crate::transport::port_body;
pub use crate::transport::TransportConfig;

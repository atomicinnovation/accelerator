//! The policy every provider client shares: credential resolution, the
//! bounded-retry schedule, identifier safety, and the transport bounds.
//!
//! Admission is deliberately narrow — policy shared by two or more provider
//! clients, with no transport and no provider specifics — so this does not
//! become the place a utility goes when it fits nowhere else. The clients may
//! not import each other; a common downward dependency is how they share a
//! rule without doing so.

pub mod credentials;
pub mod identifier;
pub mod retry;
pub mod transport;

pub use crate::credentials::refuse_tracked_source;
pub use crate::credentials::resolve_token;
pub use crate::credentials::CommandPolicy;
pub use crate::credentials::CredentialContext;
pub use crate::credentials::CredentialError;
pub use crate::credentials::Environment;
pub use crate::credentials::Provenance;
pub use crate::credentials::ResolvedToken;
pub use crate::credentials::Secret;
pub use crate::credentials::SystemEnvironment;
pub use crate::credentials::TokenKeys;
pub use crate::credentials::TokenSource;
pub use crate::identifier::identifier_is_safe;
pub use crate::identifier::IdentifierRefusal;
pub use crate::retry::ClockJitter;
pub use crate::retry::Jitter;
pub use crate::retry::RetryPolicy;
pub use crate::retry::Sleeper;
pub use crate::retry::SystemSleeper;
pub use crate::transport::port_body;
pub use crate::transport::TransportConfig;

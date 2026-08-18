//! The Linear provider client: credential resolution, a bounded transport over
//! the single GraphQL endpoint, and the body-parsing classification that turns
//! a wire outcome into the port's `TrackerError`.
//!
//! No ADF layer: Linear is Markdown-native, so a body passes through verbatim
//! in both directions (`linear-comment-flow.sh:14`).

pub mod auth;
pub mod classify;
pub mod client;
pub mod error;
pub mod filter;
pub mod transport;

pub use crate::auth::resolve_credentials;
pub use crate::auth::Credentials;
pub use crate::classify::classify;
pub use crate::classify::GraphQlError;
pub use crate::classify::Operation;
pub use crate::classify::Outcome;
pub use crate::client::LinearClient;
pub use crate::error::ClientError;
pub use crate::transport::Received;
pub use crate::transport::Transport;

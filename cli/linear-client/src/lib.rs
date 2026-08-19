//! The Linear provider client: credential resolution, a bounded transport over
//! the single GraphQL endpoint, and the body-parsing classification that turns
//! a wire outcome into the port's `TrackerError`.
//!
//! No ADF layer: Linear is Markdown-native, so a body passes through verbatim
//! in both directions.

pub mod attach;
pub mod auth;
pub mod cache;
pub mod catalogue;
pub mod classify;
pub mod client;
pub mod comment;
pub mod discovery;
pub mod error;
pub mod filter;
pub mod surface;
pub mod transition;
pub mod transport;
pub mod upload;

pub use crate::auth::resolve_credentials;
pub use crate::auth::Credentials;
pub use crate::catalogue::CatalogueStates;
pub use crate::classify::classify;
pub use crate::classify::GraphQlError;
pub use crate::classify::Operation;
pub use crate::classify::Outcome;
pub use crate::client::LinearClient;
pub use crate::error::ClientError;
pub use crate::surface::SurfaceError;
pub use crate::transport::Received;
pub use crate::transport::Transport;
pub use crate::upload::EchoedHeader;
pub use crate::upload::UploadTransport;

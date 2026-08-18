//! The Jira provider client: credential resolution, a bounded transport, and
//! the classification tables that turn a wire outcome into the port's
//! `TrackerError`.
//!
//! Nothing resolves this crate yet — the composition root wires it in a later
//! phase — and no provider request construction for Jira belongs anywhere
//! else.

pub mod adf;
pub mod auth;
pub mod classify;
pub mod client;
pub mod error;
pub mod jql;
pub mod path;
pub mod transport;

pub use crate::adf::document_to_markdown;
pub use crate::adf::markdown_to_document;
pub use crate::adf::AdfError;
pub use crate::auth::resolve_credentials;
pub use crate::auth::Credentials;
pub use crate::classify::classify;
pub use crate::classify::Operation;
pub use crate::classify::Outcome;
pub use crate::client::JiraClient;
pub use crate::error::ClientError;
pub use crate::transport::Received;
pub use crate::transport::Transport;

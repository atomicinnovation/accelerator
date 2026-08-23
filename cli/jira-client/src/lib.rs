//! The Jira provider client: credential resolution, a bounded transport, and
//! the classification tables that turn a wire outcome into the port's
//! `TrackerError`.
//!
//! Nothing resolves this crate yet — the composition root wires it in a later
//! phase — and no provider request construction for Jira belongs anywhere
//! else.

pub mod adf;
pub mod attach;
pub mod auth;
pub mod cache;
pub mod classify;
pub mod client;
pub mod comment;
pub mod custom_fields;
pub mod discovery;
pub mod error;
pub mod failure;
pub mod jql;
pub mod multipart;
pub mod mutation;
pub mod path;
pub mod principal;
pub mod read;
pub mod surface;
pub mod transition;
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
pub use crate::comment::CommentPage;
pub use crate::comment::Visibility;
pub use crate::custom_fields::CustomFieldError;
pub use crate::error::ClientError;
pub use crate::failure::JiraFailure;
pub use crate::mutation::CreateFields;
pub use crate::mutation::FieldEdit;
pub use crate::mutation::IssueType;
pub use crate::mutation::UpdateFields;
pub use crate::principal::PrincipalError;
pub use crate::surface::SurfaceError;
pub use crate::transition::Target;
pub use crate::transition::Transition;
pub use crate::transport::Received;
pub use crate::transport::Transport;

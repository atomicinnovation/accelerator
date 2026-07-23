//! Cross-cutting contracts shared across accelerator subdomains.

pub mod logging;

/// The error taxonomy accelerator subcommands report through.
///
/// A subdomain maps its own richer error enum into `Failed` at the dispatch
/// boundary; `kernel` is the lowest crate and cannot name a subdomain's types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid log filter: {0}")]
    LogFilter(#[from] tracing_subscriber::filter::ParseError),
    #[error("{0}")]
    Failed(String),
    /// A subcommand-scoped, caller-actionable refusal; its meaning is defined
    /// per subcommand, not globally.
    #[error("{0}")]
    Refusal(String),
}

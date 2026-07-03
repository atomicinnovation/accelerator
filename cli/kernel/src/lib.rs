//! Cross-cutting contracts shared across accelerator subdomains.

pub mod logging;

/// The error taxonomy accelerator subcommands report through.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid log filter: {0}")]
    LogFilter(#[from] tracing_subscriber::filter::ParseError),
}

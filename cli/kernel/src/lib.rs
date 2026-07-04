//! Cross-cutting contracts shared across accelerator subdomains.

pub mod logging;

/// The error taxonomy accelerator subcommands report through.
///
/// Deliberately small and genuinely shared: it is the boundary type, not a
/// dumping ground for every subdomain's private failure modes. A subdomain keeps
/// its own rich error enum (e.g. the launcher's [`Failed`]-mapped resolution
/// taxonomy) and maps it into this at the dispatch boundary, so a subdomain
/// never compiles against variants it cannot produce. `kernel` is the lowest
/// crate, so it cannot name a launcher type directly — the launcher owns the
/// `From<ResolutionError>` conversion into [`Error::Failed`].
///
/// [`Failed`]: Error::Failed
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid log filter: {0}")]
    LogFilter(#[from] tracing_subscriber::filter::ParseError),
    /// A subcommand could not complete; the string is a ready-to-print,
    /// user-facing diagnostic already assembled by the failing subdomain.
    #[error("{0}")]
    Failed(String),
}

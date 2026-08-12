//! Cross-cutting contracts shared across accelerator subdomains.

pub mod hooks;
pub mod logging;

/// The error taxonomy accelerator subcommands report through.
///
/// A subdomain maps its own richer error enum into `Failed` at the dispatch
/// boundary; `kernel` is the lowest crate and cannot name a subdomain's types.
///
/// Every variant carries a `String`, and none names a type from outside this
/// crate. `kernel` is the crate every other one depends on, so a foreign type
/// in this enum is a foreign type in the whole workspace's error surface —
/// reachable by anyone matching a variant, and something a consumer must depend
/// on to construct one.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `ACCELERATOR_LOG` held a filter the subscriber could not parse. Carries
    /// what the parser said, not the parser's own error type.
    #[error("invalid log filter: {0}")]
    LogFilter(String),
    #[error("{0}")]
    Failed(String),
    /// A subcommand-scoped, caller-actionable refusal; its meaning is defined
    /// per subcommand, not globally.
    #[error("{0}")]
    Refusal(String),
}

//! Cross-cutting contracts shared across accelerator subdomains.

use std::fmt::Display;
use std::fmt::Formatter;

pub mod hooks;
pub mod logging;

/// The error taxonomy accelerator subcommands report through.
///
/// A subdomain maps its own richer error enum into `Failed` at the dispatch
/// boundary; `kernel` is the lowest crate and cannot name a subdomain's types.
///
/// Every variant carries a `String`, and none names a type from outside this
/// crate — not in a field, and not in a derive. `kernel` is the crate every
/// other one depends on, so a foreign type here lands in the whole workspace's
/// error surface, and a consumer would have to depend on it to construct a
/// variant.
#[derive(Debug)]
pub enum Error {
    /// `ACCELERATOR_LOG` held a filter the subscriber could not parse. Carries
    /// what the parser said, not the parser's own error type.
    LogFilter(String),
    /// A failure a subdomain has already rendered into a message.
    Failed(String),
    /// A subcommand-scoped, caller-actionable refusal; its meaning is defined
    /// per subcommand, not globally.
    Refusal(String),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LogFilter(detail) => {
                write!(formatter, "invalid log filter: {detail}")
            }
            Self::Failed(detail) | Self::Refusal(detail) => {
                formatter.write_str(detail)
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use crate::Error;

    #[test]
    fn a_log_filter_error_names_the_filter_as_the_problem() {
        assert_eq!(
            Error::LogFilter("bad=notalevel".to_owned()).to_string(),
            "invalid log filter: bad=notalevel"
        );
    }

    #[test]
    fn a_failure_renders_its_detail_alone() {
        // Deliberately bare: a subdomain maps its own error's message in here,
        // so a prefix added at this boundary would double up on every one.
        assert_eq!(
            Error::Failed("could not read the config".to_owned()).to_string(),
            "could not read the config"
        );
    }

    #[test]
    fn a_refusal_renders_its_detail_alone() {
        assert_eq!(
            Error::Refusal("tampered binary".to_owned()).to_string(),
            "tampered binary"
        );
    }

    #[test]
    fn an_error_carries_no_cause_to_chain() {
        use std::error::Error as _;

        // No variant holds another error, so the chain ends here. A
        // reintroduced wrapped cause would put that cause's type back into the
        // public surface of the crate everything depends on.
        assert!(Error::Failed(String::new()).source().is_none());
    }
}

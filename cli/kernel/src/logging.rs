//! The stderr `tracing` logging facility every accelerator binary initialises.
//!
//! The reachable, testable behaviour is the env-filter build; the global
//! subscriber install is a thin wrapper. Reads `ACCELERATOR_LOG` (namespaced, not
//! `RUST_LOG`) so it never clobbers unrelated Rust tooling in the same shell.

use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::Error;

fn filter_from_env(raw: Option<&str>) -> Result<EnvFilter, Error> {
    let Some(directives) = raw else {
        return Ok(EnvFilter::default().add_directive(LevelFilter::INFO.into()));
    };
    EnvFilter::builder()
        .parse(directives)
        .map_err(|error| Error::LogFilter(error.to_string()))
}

/// Installs the process-global stderr subscriber from `ACCELERATOR_LOG`.
///
/// # Errors
///
/// Returns [`Error::LogFilter`] when `ACCELERATOR_LOG` holds a malformed filter.
pub fn init() -> Result<(), Error> {
    let raw = std::env::var("ACCELERATOR_LOG").ok();
    let filter = filter_from_env(raw.as_deref())?;
    // Called once from main; the already-initialised case is benign, so the
    // install outcome is discarded to keep init idempotent under test.
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .try_init();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::filter_from_env;
    use crate::Error;

    #[test]
    fn unset_env_builds_a_default_filter() {
        assert!(filter_from_env(None).is_ok());
    }

    #[test]
    fn a_valid_directive_builds_a_filter() {
        assert!(filter_from_env(Some("debug")).is_ok());
    }

    #[test]
    fn a_malformed_directive_is_a_log_filter_error() {
        let Err(Error::LogFilter(detail)) =
            filter_from_env(Some("bad=notalevel"))
        else {
            unreachable!("a malformed directive must be a LogFilter error");
        };
        // The variant holds the detail rather than the parser's own error type,
        // so what the parser said has to be carried across deliberately.
        assert!(
            !detail.is_empty(),
            "the parse failure's own text was dropped"
        );
        assert_eq!(
            Error::LogFilter(detail.clone()).to_string(),
            format!("invalid log filter: {detail}")
        );
    }
}

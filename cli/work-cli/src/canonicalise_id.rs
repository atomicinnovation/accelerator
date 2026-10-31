//! Adapter/binary wiring for `work canonicalise-id`.

use ::config::ConfigAccess;
use work::id_pattern::PatternError;
use work::id_scheme::CanonicaliseError;
use work_adapters::config::SchemeError;

use crate::config::announced_scheme;

pub enum RunOutcome {
    Canonicalised(String),
    Failed(String),
}

fn scheme_failure(error: SchemeError) -> String {
    match error {
        SchemeError::Pattern {
            error: error @ PatternError::BadFormatSpec(_),
            ..
        } => format!("E_PATTERN_BAD_FORMAT_SPEC: {error}"),
        SchemeError::Pattern { error, .. } => format!("E_PATTERN: {error}"),
        SchemeError::Key { error, .. } => format!("E_PATTERN: {error}"),
        other => other.to_string(),
    }
}

/// # Errors
///
/// Never returns `Err`; every failure is reported through [`RunOutcome`].
#[must_use]
pub fn run(config: &dyn ConfigAccess, input: &str) -> RunOutcome {
    let scheme = match announced_scheme(config) {
        Ok(scheme) => scheme,
        Err(error) => return RunOutcome::Failed(scheme_failure(error)),
    };
    match scheme.canonicalise(input) {
        Ok(id) => RunOutcome::Canonicalised(id),
        Err(CanonicaliseError::MissingKey) => RunOutcome::Failed(format!(
            "E_PATTERN_MISSING_KEY: bare number '{input}' under \
             pattern '{}' requires a key value",
            scheme.pattern().as_str()
        )),
        Err(
            error @ (CanonicaliseError::EmptyInput
            | CanonicaliseError::UnrecognisedIdShape(_)),
        ) => RunOutcome::Failed(format!("E_PATTERN_BAD_FORMAT_SPEC: {error}")),
        Err(error @ CanonicaliseError::NoMatch) => {
            RunOutcome::Failed(format!("E_PATTERN: {error}"))
        }
    }
}

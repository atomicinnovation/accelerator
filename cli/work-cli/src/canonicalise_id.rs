//! Adapter/binary wiring for `work canonicalise-id`.

use ::config::resolve_with_deprecated_fallback;
use ::config::ConfigAccess;
use corpus::references_key;
use corpus_adapters::canonicalise_id;
use corpus_adapters::PatternError;

use crate::config::effective_nonempty;
use crate::config::LEGACY_PREFIX_KEY;

pub enum RunOutcome {
    Canonicalised(String),
    Failed(String),
}

const fn error_code(error: &PatternError) -> &'static str {
    match error {
        PatternError::EmptyInput
        | PatternError::UnrecognisedIdShape(_)
        | PatternError::BadFormatSpec(_) => "E_PATTERN_BAD_FORMAT_SPEC",
        _ => "E_PATTERN",
    }
}

/// # Errors
///
/// Never returns `Err`; every failure is reported through [`RunOutcome`].
#[must_use]
pub fn run(config: &dyn ConfigAccess, input: &str) -> RunOutcome {
    let pattern = match effective_nonempty(config, "work.id_pattern") {
        Ok(pattern) => pattern,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let prefix = match resolve_with_deprecated_fallback(
        config,
        "work.key",
        LEGACY_PREFIX_KEY,
        None,
    ) {
        Ok(prefix) => prefix,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    if references_key(&pattern) {
        ::config::emit_deprecation_once(
            LEGACY_PREFIX_KEY,
            prefix.deprecation.as_deref(),
        );
    }
    let project = prefix.value.unwrap_or_default();
    match canonicalise_id(input, &pattern, &project) {
        Ok(id) => RunOutcome::Canonicalised(id),
        Err(PatternError::MissingKey) => RunOutcome::Failed(format!(
            "E_PATTERN_MISSING_PROJECT: bare number '{input}' under \
             pattern '{pattern}' requires a project value"
        )),
        Err(error) => {
            RunOutcome::Failed(format!("{}: {error}", error_code(&error)))
        }
    }
}

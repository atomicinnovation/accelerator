//! Adapter/binary wiring for `work canonicalise-id`. Exact behavioural
//! match for `wip_canonicalise_id`.

use ::config::ConfigAccess;
use corpus_adapters::canonicalise_id;
use corpus_adapters::PatternError;

use crate::config::effective_nonempty;

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
    let project = match effective_nonempty(config, "work.default_project_code")
    {
        Ok(project) => project,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    match canonicalise_id(input, &pattern, &project) {
        Ok(id) => RunOutcome::Canonicalised(id),
        Err(PatternError::MissingProject) => RunOutcome::Failed(format!(
            "E_PATTERN_MISSING_PROJECT: bare number '{input}' under \
             pattern '{pattern}' requires a project value"
        )),
        Err(error) => {
            RunOutcome::Failed(format!("{}: {error}", error_code(&error)))
        }
    }
}

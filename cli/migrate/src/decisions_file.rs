//! Decisions-file parsing and the dry-apply validation pass.
//!
//! Blank lines and CRLF line endings are tolerated; `#`-prefixed lines are
//! NOT comments — bash's own `read_decision` treats a `#`-prefixed line as
//! an unrecognised verb (confirmed against a captured bash golden,
//! `cli/migrate-cli/tests/fixtures/decisions-file/blank-crlf-comments/`).
//! This parser matches that behaviour exactly, not the aspirational
//! "comments ignored" description an earlier planning pass assumed.

use crate::interactive::Decision;
use crate::interactive::InteractiveMigration;
use crate::interactive::Transformation;
use crate::ports::MigrationError;

/// One non-blank, CRLF-normalised line from a decisions file, in file order.
pub struct DecisionLines(Vec<String>);

impl DecisionLines {
    #[must_use]
    pub fn parse(content: &str) -> Self {
        Self(
            content
                .split('\n')
                .map(|line| line.strip_suffix('\r').unwrap_or(line))
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect(),
        )
    }

    /// The parsed decision at `index` (0-based): `None` past the end of the
    /// file, `Some(None)` for an unparseable (unknown-verb) line. A caller
    /// that only ever reads a decisions file already checked by
    /// [`validate`] never observes the `Some(None)` arm.
    #[must_use]
    pub fn decision_at(&self, index: usize) -> Option<Option<Decision>> {
        self.0.get(index).map(|line| match parse_line(line) {
            ParsedLine::Decision(decision) => Some(decision),
            ParsedLine::UnknownVerb(_) => None,
        })
    }
}

enum ParsedLine {
    Decision(Decision),
    UnknownVerb(String),
}

fn parse_line(line: &str) -> ParsedLine {
    if line == "accept" || line.starts_with("accept ") {
        ParsedLine::Decision(Decision::Accept)
    } else if line == "skip" || line.starts_with("skip ") {
        ParsedLine::Decision(Decision::Skip)
    } else if let Some(value) = line.strip_prefix("edit ") {
        ParsedLine::Decision(Decision::Edit(value.to_owned()))
    } else if line == "edit" {
        ParsedLine::Decision(Decision::Edit(String::new()))
    } else {
        ParsedLine::UnknownVerb(line.to_owned())
    }
}

fn unknown_verb_error(position: usize, verb: &str) -> MigrationError {
    MigrationError::new(format!(
        "Error: decisions file position {position}: unknown verb '{verb}' \
         (expected accept | skip | edit <value>)."
    ))
}

/// Validates `content` against `prompts`.
///
/// `prompts` is the exact set `--list`/a live run would prompt for, in order
/// (see [`crate::engine::pending_transformations`]) — this replays every
/// position against `validate_edit`, with no `apply_decision` call and no
/// session-log write. Fails closed, naming the offending position, exactly
/// as bash's `dry_apply_interactive_migration`.
///
/// # Errors
/// The first position-naming validation failure: a missing decision, an
/// unknown verb, a rejected edit with no further line to recover with, or a
/// surplus decision beyond what `prompts` requires.
pub fn validate(
    migration: &dyn InteractiveMigration,
    prompts: &[Transformation],
    content: &str,
) -> Result<(), MigrationError> {
    let lines = DecisionLines::parse(content);
    let mut cursor = 0usize;

    for (index, transformation) in prompts.iter().enumerate() {
        let position = index + 1;
        let mut rejection: Option<String> = None;
        loop {
            let Some(line) = lines.0.get(cursor) else {
                return Err(rejection.map_or_else(
                    || {
                        MigrationError::new(format!(
                            "Error: decisions file is missing a decision \
                             for position {position} ({}).",
                            transformation.short_key()
                        ))
                    },
                    |reason| {
                        MigrationError::new(format!(
                            "Error: decisions file position {position}: \
                             edit rejected ({reason}) and no further \
                             decision to recover."
                        ))
                    },
                ));
            };
            cursor += 1;
            match parse_line(line) {
                ParsedLine::UnknownVerb(verb) => {
                    return Err(unknown_verb_error(position, &verb));
                }
                ParsedLine::Decision(Decision::Edit(value)) => {
                    match migration.validate_edit(transformation, &value) {
                        Ok(()) => break,
                        Err(message) => {
                            rejection = Some(message);
                        }
                    }
                }
                ParsedLine::Decision(Decision::Accept | Decision::Skip) => {
                    break;
                }
            }
        }
    }

    if cursor < lines.0.len() {
        return Err(MigrationError::new(format!(
            "Error: decisions file has a surplus decision at position {} \
             (only {} transformation(s) require one).",
            cursor + 1,
            prompts.len()
        )));
    }
    Ok(())
}

//! The interactive engine loop: emits every transformation once, replays
//! resumed decisions (source drift and `verify_applied` checked first, ahead
//! of anything outcome-specific), and prompts for the rest.

use std::time::Duration;

use corpus::Outcome;

use crate::interactive::Decision;
use crate::interactive::InteractiveMigration;
use crate::interactive::PredicateOutcome;
use crate::interactive::Transformation;
use crate::ports::DecisionError;
use crate::ports::DecisionSource;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::ports::Reporter;
use crate::ports::SessionLog;
use crate::registry::ApplyOutcome;

/// # Errors
/// The predicate's `Fail` message, a validation-exhausted stall, or a
/// [`DecisionSource`] timeout/EOF — each already reported via `reporter`
/// before this returns. Never produces [`ApplyOutcome::NoOpPending`]: an
/// interactive migration either completes every transformation or fails.
pub fn run_interactive(
    migration: &dyn InteractiveMigration,
    ctx: &dyn MigrationContext,
    decisions: &dyn DecisionSource,
    session_log: &dyn SessionLog,
    timeout: Duration,
    reporter: &dyn Reporter,
) -> Result<ApplyOutcome, MigrationError> {
    let id = migration.id();
    let transformations = migration.emit_transformations(ctx);
    let mut recorded = session_log.records()?;

    for (index, transformation) in transformations.iter().enumerate() {
        let existing = recorded
            .iter()
            .position(|record| record.transformation_key == transformation.key)
            .map(|position| recorded.remove(position));

        let already_decided = existing.as_ref().is_some_and(|record| {
            record.proposed_value == transformation.proposed
                && match record.outcome {
                    Outcome::Skipped => true,
                    Outcome::Accepted | Outcome::Edited => {
                        migration.verify_applied(transformation, record)
                    }
                }
        });

        if already_decided {
            continue;
        }
        if existing.is_some() {
            session_log.remove_by_key(&transformation.key)?;
        }

        match migration.evaluate_predicate(transformation) {
            PredicateOutcome::Mechanical => {
                apply(
                    id,
                    transformation,
                    &Decision::Accept,
                    migration,
                    ctx,
                    reporter,
                )?;
            }
            PredicateOutcome::Fail(message) => {
                reporter.interactive_fail(id, &message);
                return Err(MigrationError::new(format!("[{id}] {message}")));
            }
            PredicateOutcome::Prompt => {
                prompt_and_decide(
                    id,
                    transformation,
                    &transformations[index..],
                    migration,
                    ctx,
                    decisions,
                    session_log,
                    timeout,
                    reporter,
                )?;
            }
        }
    }

    Ok(ApplyOutcome::Applied)
}

fn apply(
    id: &str,
    transformation: &Transformation,
    decision: &Decision,
    migration: &dyn InteractiveMigration,
    ctx: &dyn MigrationContext,
    reporter: &dyn Reporter,
) -> Result<(), MigrationError> {
    migration
        .apply_decision(transformation, decision, ctx)
        .map_err(|message| {
            reporter.interactive_fail(id, &message);
            MigrationError::new(format!("[{id}] {message}"))
        })
}

/// The write-ahead-log invariant: `session_log`'s record write must return
/// `Ok` before `apply_decision` is invoked, for every decided outcome —
/// accept, edit, and skip alike.
fn record_then_apply(
    id: &str,
    transformation: &Transformation,
    decision: &Decision,
    migration: &dyn InteractiveMigration,
    ctx: &dyn MigrationContext,
    session_log: &dyn SessionLog,
    reporter: &dyn Reporter,
) -> Result<(), MigrationError> {
    let (outcome, user_value) = match decision {
        Decision::Accept => (Outcome::Accepted, None),
        Decision::Edit(value) => (Outcome::Edited, Some(value.as_str())),
        Decision::Skip => (Outcome::Skipped, None),
    };
    session_log.append(
        &transformation.key,
        outcome,
        &transformation.proposed,
        user_value,
    )?;
    apply(id, transformation, decision, migration, ctx, reporter)
}

#[allow(clippy::too_many_arguments)]
fn prompt_and_decide(
    id: &str,
    transformation: &Transformation,
    remaining: &[Transformation],
    migration: &dyn InteractiveMigration,
    ctx: &dyn MigrationContext,
    decisions: &dyn DecisionSource,
    session_log: &dyn SessionLog,
    timeout: Duration,
    reporter: &dyn Reporter,
) -> Result<(), MigrationError> {
    loop {
        match decisions.next_decision(transformation, timeout) {
            Ok(Decision::Edit(value)) => {
                if let Err(message) =
                    migration.validate_edit(transformation, &value)
                {
                    reporter.interactive_validation_rejected(&message);
                    continue;
                }
                return record_then_apply(
                    id,
                    transformation,
                    &Decision::Edit(value),
                    migration,
                    ctx,
                    session_log,
                    reporter,
                );
            }
            Ok(decision @ (Decision::Accept | Decision::Skip)) => {
                return record_then_apply(
                    id,
                    transformation,
                    &decision,
                    migration,
                    ctx,
                    session_log,
                    reporter,
                );
            }
            Err(DecisionError::NoInputAvailable) => {
                let pending_keys: Vec<String> = remaining
                    .iter()
                    .map(|pending| pending.key.clone())
                    .collect();
                reporter.interactive_stalled(id, &pending_keys);
                return Err(MigrationError::new(format!(
                    "[{id}] MIGRATION STALLED: no decision input available"
                )));
            }
            Err(DecisionError::Timeout | DecisionError::Eof) => {
                reporter.interactive_timeout(id);
                return Err(MigrationError::new(format!(
                    "[{id}] timed out waiting for a decision"
                )));
            }
        }
    }
}

//! The interactive migration contract: ADR-0037/0038 as direct Rust trait
//! calls, not a wire protocol.

use crate::ports::MigrationContext;
use crate::registry::MigrationMeta;

pub struct Transformation {
    pub key: String,
    pub path: String,
    pub anchor: String,
    pub proposed: String,
    pub predicate_value: String,
    pub display: String,
    pub extras: Vec<(String, String)>,
}

pub enum PredicateOutcome {
    Prompt,
    Mechanical,
    Fail(String),
}

#[derive(Debug, Clone)]
pub enum Decision {
    Accept,
    Edit(String),
    Skip,
}

pub trait InteractiveMigration: MigrationMeta {
    fn emit_transformations(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Vec<Transformation>;

    fn evaluate_predicate(
        &self,
        transformation: &Transformation,
    ) -> PredicateOutcome;

    /// # Errors
    /// The rejection message, printed as `"[interactive] {message}"`.
    fn validate_edit(
        &self,
        transformation: &Transformation,
        value: &str,
    ) -> Result<(), String>;

    /// # Errors
    /// The failure message, relayed verbatim as `"[{id}] {message}"`.
    fn apply_decision(
        &self,
        transformation: &Transformation,
        decision: &Decision,
        ctx: &dyn MigrationContext,
    ) -> Result<(), String>;

    /// Consulted on resume, before replaying an accepted/edited record —
    /// never for a skipped one. `true` (the default) replays silently;
    /// `false` is handled identically to source drift.
    fn verify_applied(
        &self,
        _transformation: &Transformation,
        _recorded: &corpus::Record,
    ) -> bool {
        true
    }
}

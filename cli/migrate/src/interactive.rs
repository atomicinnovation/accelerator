//! The interactive migration contract: direct Rust trait calls, not a wire
//! protocol.

use crate::ports::MigrationContext;
use crate::registry::MigrationMeta;

#[derive(Clone)]
pub struct Transformation {
    pub key: String,
    pub path: String,
    pub anchor: String,
    pub proposed: String,
    pub predicate_value: String,
    pub display: String,
    pub extras: Vec<(String, String)>,
}

impl Transformation {
    /// The short, human-facing identifier `--list` and the decisions-file
    /// dry-apply validation pass name a position by, distinct from `key`
    /// above (which is this port's session-log identity,
    /// `{path}#{anchor}`). Migrations that supply one via an `extras` entry
    /// named `linkage_key` (m0007's convention) get it named precisely;
    /// anything else falls back to the full session-log key.
    #[must_use]
    pub fn short_key(&self) -> &str {
        self.extras
            .iter()
            .find(|(name, _)| name == "linkage_key")
            .map_or(self.key.as_str(), |(_, value)| value.as_str())
    }
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

    /// Never called for `Decision::Skip` — the engine records a skip and
    /// stops there. `decision` is always `Accept` or `Edit` here.
    ///
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

    /// Called once, unconditionally, after every transformation this run
    /// decided has been applied (or immediately if there were none to
    /// begin with) — never per-transformation, and never skipped just
    /// because the last decision happened to be a skip. Runs once after the
    /// whole interactive apply loop completes, not threaded through any one
    /// transformation's own callback. The default is a no-op; only a
    /// migration with whole-corpus post-apply validation (m0007) needs to
    /// override it.
    ///
    /// # Errors
    /// The failure message, relayed verbatim as `"[{id}] {message}"`.
    fn finalise(&self, _ctx: &dyn MigrationContext) -> Result<(), String> {
        Ok(())
    }
}

//! The whole binary's exit-code taxonomy, in one place — the single
//! authoritative source for what each code means. Each subcommand's `--help`
//! cross-references this table rather than restating it, so the two
//! runtime-discoverable surfaces cannot drift.
//!
//! The codes fall into three bands emitted from three distinct sources:
//!
//! Process and selection codes (`0`–`6`), emitted directly by the binary:
//!
//! - `0` `CLEAN` — success.
//! - `1` `ERROR` — an internal error.
//! - `2` `USAGE` — a usage error (bad arguments, mutually-exclusive flags).
//! - `3` `RESOLVE_NOT_FOUND` — an identifier resolved to no work item.
//! - `4` `UNRESOLVED` — the run completed but items await a human (unresolved
//!   conflicts, skipped-dirty pulls, remote-absent or indeterminate items).
//! - `5` `REFUSED_BULK_OVERWRITE` — a run refused because it would exceed
//!   `--max-pulls`/`--max-pushes`; zero writes occurred.
//! - `6` `RESOLVE_OUTSIDE_WORKDIR` — a path target names a file that exists but
//!   lies outside the managed work directory.
//! - `7` `KEYED_READ_CAPPED` — the bulk keyed reconcile read hit its
//!   `<tracker>.pull.max_pages` cap, so the un-read items' remote state is
//!   unknown. The run aborted before any write, all-or-nothing across both
//!   directions (the read feeds pull *and* push planning, so `--push-only` does
//!   not bypass it). Distinct from `4` `UNRESOLVED`: nothing was reconciled and
//!   nothing awaits a human — raise the cap (or its `keyed_read` override, or
//!   `unlimited`) and re-run.
//!
//! Tracker-error codes (`70`/`71`), the two-class [`TrackerError`] split that
//! [`for_tracker_error`] maps. This distinction is safety-critical — the work
//! skills branch on it:
//!
//! - `70` `RETRYABLE` — the failure is provably *before* any remote mutation
//!   (argument/validation/auth/connect, a read that failed, or a discovery
//!   search that failed transiently). **Safe to retry.** A read that fails
//!   degrades to presence-only and reports `70`; a read never emits `71`.
//! - `71` `TERMINAL` — the failure is *at or after* a mutation, so its outcome
//!   is unknown. **Never auto-retried.** The hazard differs by path: a remote
//!   `create` is non-idempotent, so a repeat would *double-apply* (a second
//!   issue) — a remote issue may already exist; a whole-item `update` is
//!   idempotent, so there the hazard is response *uncertainty*, not
//!   double-apply. Either way the operator reconciles by hand.
//!
//! Tracker selection/configuration codes (`72`–`74`), a failure to *select or
//! configure* a tracker rather than a tracker-error class. `72`/`73` and the
//! credential branch of `74` come from `SelectionError`; `74` is also emitted
//! pre-flight when a run's discovery scope names no valid target
//! (`RunError::DiscoveryUnconfigured`):
//!
//! - `72` `NOT_AVAILABLE` — the configured tracker is recognised but has no
//!   client wired yet (`trello`/`github-issues`).
//! - `73` `UNRECOGNISED` — `work.integration` is unset or names a tracker
//!   outside `{linear, jira, trello, github-issues}`. Fail closed.
//! - `74` `UNCONFIGURED` — the tracker is wired but a run cannot proceed on its
//!   configuration: its credentials are missing or refused, or a non-push-only
//!   run's discovery scope names no valid target (an unset or unresolvable
//!   key, or a configured `additional_*`/`all_*` entity the credential cannot
//!   see). **No write was made** — the refusal is pre-flight, before the
//!   apply/push phase. A broadened scope is confirmed against a live
//!   entity-enumeration read, so a read may have gone out, but nothing was
//!   mutated — so save locally and fix the config; never reconcile against a
//!   create that never happened.

use tracker::TrackerError;

pub const CLEAN: u8 = 0;
pub const ERROR: u8 = 1;
pub const USAGE: u8 = 2;
pub const RESOLVE_NOT_FOUND: u8 = 3;
pub const UNRESOLVED: u8 = 4;
pub const REFUSED_BULK_OVERWRITE: u8 = 5;
pub const RESOLVE_OUTSIDE_WORKDIR: u8 = 6;
pub const KEYED_READ_CAPPED: u8 = 7;

pub const RETRYABLE: u8 = 70;
pub const TERMINAL: u8 = 71;
pub const NOT_AVAILABLE: u8 = 72;
pub const UNRECOGNISED: u8 = 73;
pub const UNCONFIGURED: u8 = 74;

#[must_use]
pub const fn for_tracker_error(error: &TrackerError) -> u8 {
    match error {
        TrackerError::Retryable { .. } => RETRYABLE,
        TrackerError::Terminal { .. } => TERMINAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keyed_read_abort_has_its_own_code_distinct_from_the_rest() {
        let others = [
            CLEAN,
            ERROR,
            USAGE,
            RESOLVE_NOT_FOUND,
            UNRESOLVED,
            REFUSED_BULK_OVERWRITE,
            RESOLVE_OUTSIDE_WORKDIR,
            RETRYABLE,
            TERMINAL,
            NOT_AVAILABLE,
            UNRECOGNISED,
            UNCONFIGURED,
        ];
        assert!(
            !others.contains(&KEYED_READ_CAPPED),
            "the keyed-read abort must not collide with another code, \
             especially the exit-4 indeterminate code"
        );
        assert_ne!(KEYED_READ_CAPPED, UNRESOLVED);
    }
}

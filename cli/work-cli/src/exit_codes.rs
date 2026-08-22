//! The whole binary's exit-code taxonomy, in one place — the single
//! authoritative source for what each code means. Each subcommand's `--help`
//! cross-references this table rather than restating it, so the two
//! runtime-discoverable surfaces cannot drift.
//!
//! The codes fall into three bands emitted from three distinct sources:
//!
//! Process and selection codes (`0`–`5`), emitted directly by the binary:
//!
//! - `0` `CLEAN` — success.
//! - `1` `ERROR` — an internal error.
//! - `2` `USAGE` — a usage error (bad arguments, mutually-exclusive flags).
//! - `3` `RESOLVE_NOT_FOUND` — an identifier resolved to no work item.
//! - `4` `UNRESOLVED` — the run completed but items await a human (unresolved
//!   conflicts, skipped-dirty pulls, remote-absent or indeterminate items).
//! - `5` `REFUSED_BULK_OVERWRITE` — a run refused because it would exceed
//!   `--max-pulls`/`--max-pushes`; zero writes occurred.
//!
//! Tracker-error codes (`70`/`71`), the two-class [`TrackerError`] split that
//! [`for_tracker_error`] maps. This distinction is safety-critical — the work
//! skills branch on it:
//!
//! - `70` `RETRYABLE` — the failure is provably *before* any remote mutation
//!   (argument/validation/auth/connect, or a read that failed). **Safe to
//!   retry.** A read that fails degrades to presence-only and reports `70`; a
//!   read never emits `71`.
//! - `71` `TERMINAL` — the failure is *at or after* a mutation, so its outcome
//!   is unknown. **Never auto-retried.** The hazard differs by path: a remote
//!   `create` is non-idempotent, so a repeat would *double-apply* (a second
//!   issue) — a remote issue may already exist; a whole-item `update` is
//!   idempotent, so there the hazard is response *uncertainty*, not
//!   double-apply. Either way the operator reconciles by hand.
//!
//! Tracker selection/configuration codes (`72`–`74`), emitted from
//! `SelectionError` — a failure to *select or configure* a tracker, not a
//! tracker-error class:
//!
//! - `72` `NOT_AVAILABLE` — the configured tracker is recognised but has no
//!   client wired yet (`trello`/`github-issues`).
//! - `73` `UNRECOGNISED` — `work.integration` is unset or names a tracker
//!   outside `{linear, jira, trello, github-issues}`. Fail closed.
//! - `74` `UNCONFIGURED` — the tracker is wired but its configuration or
//!   credentials are missing or refused. **Nothing was sent** — save locally
//!   and fix the config; never reconcile against a create that never happened.

use tracker::TrackerError;

pub const CLEAN: u8 = 0;
pub const ERROR: u8 = 1;
pub const USAGE: u8 = 2;
pub const RESOLVE_NOT_FOUND: u8 = 3;
pub const UNRESOLVED: u8 = 4;
pub const REFUSED_BULK_OVERWRITE: u8 = 5;

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

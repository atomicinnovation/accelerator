//! The whole binary's exit-code taxonomy, in one place — the single
//! authoritative source for what each code means, mirroring `work-cli`'s own
//! `exit_codes` module. Both the shared `Outcome`→`report` path and the
//! `resolve` subcommand route through these constants rather than scattering
//! bare `ExitCode::from(n)` literals.
//!
//! Exit `2` carries two per-subcommand meanings that only this table
//! reconciles: [`REFUSAL`] for the four `Outcome`-based subcommands (a
//! `kernel::Error::Refusal`), and [`AMBIGUOUS`] for `resolve` (a slug matched
//! more than one document). `resolve` therefore gives an unknown `--type` its
//! own [`UNKNOWN_TYPE`] rather than reusing the refusal path, so the skill
//! preamble can tell an unregistered type apart from an ambiguous match.
//!
//! - `0` [`RESOLVED`] — success (a resolved slug, or any clean subcommand).
//! - `1` [`INVALID`] — an internal error, or a `resolve` input that is neither
//!   a slug nor a path.
//! - `2` [`AMBIGUOUS`] / [`REFUSAL`] — a `resolve` slug matched multiple
//!   documents; or an `Outcome`-based subcommand refused.
//! - `3` [`NOT_FOUND`] — a `resolve` slug or path matched no document.
//! - `4` [`UNKNOWN_TYPE`] — `resolve --type` named a type not registered as a
//!   `DocTypeKey` (the `topic-research`-until-0278 case).
//! - `6` [`OUTSIDE_ROOT`] — a `resolve` path target exists but lies outside the
//!   configured type directory.

pub const RESOLVED: u8 = 0;
pub const INVALID: u8 = 1;
pub const AMBIGUOUS: u8 = 2;
pub const REFUSAL: u8 = 2;
pub const NOT_FOUND: u8 = 3;
pub const UNKNOWN_TYPE: u8 = 4;
pub const OUTSIDE_ROOT: u8 = 6;

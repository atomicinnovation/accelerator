//! `accelerator-linear`'s exit-code taxonomy — the document of record.
//!
//! Every code equals the value the retiring `linear-*-flow.sh` returned for the
//! same condition, captured pre-deletion into
//! `tests/fixtures/bash-exit-codes.txt`, which is the authoritative name→integer
//! contract. `exit_codes_parity.rs` pins these constants against it; the
//! self-descriptive names plus that fixture carry the mechanical mapping, so
//! this doc names only the bands, the safety-critical classes and the one
//! deliberate divergence rather than re-enumerating every code.
//!
//! Bands:
//!
//! - `0`–`2` — process and usage, emitted by the binary itself.
//! - `11`–`53` — the shared transport/auth codes `linear-graphql.sh` and
//!   `linear-auth.sh` returned; read structurally from the client's
//!   `LinearFailure` (`bash_code(outcome)`) or a `SurfaceError`, never parsed
//!   from a string.
//! - `60`–`138` — the per-flow argument and outcome codes, one block per flow.
//!
//! Safety-critical: `POST_SEND` (`109`) and `WRITEBACK_FAILED` (`107`) mark the
//! "created remotely" states a re-run must not blindly repeat — the create flow
//! surfaces the key and steers the operator to reconcile rather than re-create.
//!
//! Deliberate divergence: the search flow's `SEARCH_*` codes are remapped from
//! bash `70`–`73` to `75`–`78`, off the `70`–`74` band the dispatch layer
//! reserves — a code in that band reaching `accelerator-work` would read as a
//! dispatch verdict.

// Every code is a declared contract the parity test reads textually; a code no
// handler yet references is still part of the surface, not dead.
#![allow(dead_code)]

pub const CLEAN: u8 = 0;
pub const ERROR: u8 = 1;
pub const USAGE: u8 = 2;

pub const UNAUTHORIZED: u8 = 11;
pub const BAD_RESPONSE: u8 = 16;
pub const SERVER_ERROR: u8 = 20;
pub const CONNECT: u8 = 21;
pub const NO_CREDS: u8 = 22;
pub const NO_TOKEN: u8 = 24;
pub const TOKEN_CMD_FAILED: u8 = 25;
pub const TOKEN_MALFORMED: u8 = 27;
pub const LOCAL_PERMS_INSECURE: u8 = 29;
pub const BAD_REQUEST: u8 = 34;
pub const RATELIMITED: u8 = 35;
pub const COMPLEXITY: u8 = 36;
pub const REFRESH_LOCKED: u8 = 53;

pub const INIT_NEEDS_CONFIG: u8 = 60;
pub const INIT_VERIFY_FAILED: u8 = 61;
pub const INIT_NO_TEAM: u8 = 62;

// Remapped off the reserved 70-74 dispatch band; the fixture keeps 70-73.
pub const SEARCH_BAD_FLAG: u8 = 75;
pub const SEARCH_BAD_LIMIT: u8 = 76;
pub const SEARCH_NO_CATALOGUE: u8 = 77;
pub const SEARCH_BAD_STATE: u8 = 78;

pub const SHOW_NO_KEY: u8 = 80;
pub const SHOW_BAD_FLAG: u8 = 81;
pub const SHOW_NOT_FOUND: u8 = 82;

pub const COMMENT_NO_KEY: u8 = 90;
pub const COMMENT_NO_BODY: u8 = 91;
pub const COMMENT_BAD_FLAG: u8 = 92;

pub const CREATE_NO_FILE: u8 = 100;
pub const CREATE_BAD_FRONTMATTER: u8 = 101;
pub const CREATE_ALREADY_SYNCED: u8 = 102;
pub const CREATE_NO_TITLE: u8 = 103;
pub const CREATE_BAD_FLAG: u8 = 104;
pub const CREATE_NO_CATALOGUE: u8 = 105;
pub const CREATE_BAD_IDENTIFIER: u8 = 106;
pub const CREATE_WRITEBACK_FAILED: u8 = 107;
pub const CREATE_PRE_SEND: u8 = 108;
pub const CREATE_POST_SEND: u8 = 109;

pub const UPDATE_NO_KEY: u8 = 110;
pub const UPDATE_NO_OPS: u8 = 111;
pub const UPDATE_BAD_FLAG: u8 = 112;
pub const UPDATE_NO_CATALOGUE: u8 = 113;
pub const UPDATE_BAD_STATE: u8 = 114;

pub const TRANSITION_NO_KEY: u8 = 120;
pub const TRANSITION_NO_STATE: u8 = 121;
pub const TRANSITION_STATE_NOT_IN_CATALOGUE: u8 = 122;
pub const TRANSITION_STATE_AMBIGUOUS: u8 = 123;
pub const TRANSITION_NO_CATALOGUE: u8 = 124;
pub const TRANSITION_BAD_FLAG: u8 = 125;

pub const ATTACH_NO_KEY: u8 = 130;
pub const ATTACH_NO_TARGET: u8 = 131;
pub const ATTACH_BOTH_TARGETS: u8 = 132;
pub const ATTACH_FILE_MISSING: u8 = 133;
pub const ATTACH_BAD_URL: u8 = 134;
pub const ATTACH_BAD_UPLOAD_URL: u8 = 135;
pub const ATTACH_UPLOAD_FAILED: u8 = 136;
pub const ATTACH_REGISTER_FAILED: u8 = 137;
pub const ATTACH_BAD_FLAG: u8 = 138;

use linear_client::cache::CacheError;
use linear_client::classify::{bash_code, Outcome};
use linear_client::{ClientError, LinearFailure, SurfaceError};

/// The exit code for a discovery-cache write failure. A held lock maps to the
/// shared refresh-lock code; an IO or serialisation failure is a generic error.
#[must_use]
pub const fn for_cache(error: &CacheError) -> u8 {
    match error {
        CacheError::LockContended { .. } => REFRESH_LOCKED,
        CacheError::Io { .. } | CacheError::Serialise { .. } => ERROR,
    }
}

/// The exit code for a structured port-op failure (`create`/`update`/`show`).
#[must_use]
pub fn for_failure(failure: &LinearFailure) -> u8 {
    match failure {
        LinearFailure::Wire { outcome, .. } => code_for_outcome(*outcome),
        // Created remotely but unwritable — the non-retryable orphan case.
        LinearFailure::UnwritableIdentifier { .. } => CREATE_WRITEBACK_FAILED,
    }
}

/// The exit code for a surface-flow failure (`search`/`comment`/`transition`/
/// `attach`/`init`). Transport failures carry the same shared codes as the
/// port ops; the flow-specific variants map to their own block.
#[must_use]
pub fn for_surface(error: &SurfaceError) -> u8 {
    match error {
        SurfaceError::Client(client) => for_client(client),
        SurfaceError::GraphQlErrors { body, .. } => {
            code_for_outcome(Outcome::SuccessWithErrors(
                linear_client::classify::classify_errors(
                    &serde_json::from_str(body).unwrap_or_default(),
                ),
            ))
        }
        SurfaceError::Status { status, body, .. } => {
            code_for_status(*status, body)
        }
        SurfaceError::BadResponse { .. } => BAD_RESPONSE,
        SurfaceError::UnknownState { .. } => TRANSITION_STATE_NOT_IN_CATALOGUE,
        SurfaceError::AmbiguousState { .. } => TRANSITION_STATE_AMBIGUOUS,
        SurfaceError::BadLinkUrl { .. } => ATTACH_BAD_URL,
        SurfaceError::FileRefused { .. } => ATTACH_FILE_MISSING,
        SurfaceError::BadUploadUrl { .. } => ATTACH_BAD_UPLOAD_URL,
        SurfaceError::BadEchoedHeader { .. }
        | SurfaceError::UploadFailed { .. } => ATTACH_UPLOAD_FAILED,
        SurfaceError::RegisterFailed { .. } => ATTACH_REGISTER_FAILED,
    }
}

/// The exit code for a client-construction or credential failure.
#[must_use]
pub const fn for_client(error: &ClientError) -> u8 {
    match error {
        ClientError::Credential(_) => NO_TOKEN,
        ClientError::MalformedToken { .. } => TOKEN_MALFORMED,
        ClientError::NoTeam => CREATE_NO_CATALOGUE,
        ClientError::UnknownState { .. } => SEARCH_BAD_STATE,
        ClientError::BadIdentifier { .. } => BAD_REQUEST,
        ClientError::Transport { .. } => CONNECT,
        ClientError::OversizedResponse { .. } => BAD_RESPONSE,
        ClientError::ConfigUnreadable { .. }
        | ClientError::TlsUnavailable { .. } => ERROR,
    }
}

fn code_for_outcome(outcome: Outcome) -> u8 {
    // The shared 11-36 codes fit u8; bash_code never exceeds that band.
    u8::try_from(bash_code(outcome)).unwrap_or(ERROR)
}

fn code_for_status(status: u16, body: &str) -> u8 {
    let parsed = serde_json::from_str(body).unwrap_or_default();
    let outcome = match status {
        401 => Outcome::Unauthorised,
        400 => Outcome::BadRequest(linear_client::classify::classify_errors(
            &parsed,
        )),
        500..=599 => Outcome::ServerError,
        _ => Outcome::Unexpected,
    };
    code_for_outcome(outcome)
}

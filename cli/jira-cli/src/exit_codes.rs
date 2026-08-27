//! `accelerator-jira`'s exit-code taxonomy — the document of record.
//!
//! Every code equals the value the retiring `jira-*` bash cluster returned for
//! the same condition, captured pre-deletion into
//! `tests/fixtures/bash-exit-codes.txt` (from the cluster's `EXIT_CODES.md`
//! namespace and its behavioural reconciliations). `exit_codes_parity.rs` pins
//! these constants against it; the self-descriptive names plus that fixture
//! carry the mechanical mapping, so this doc names only the bands, the
//! safety-critical classes and the deliberate divergences.
//!
//! Bands:
//!
//! - `0`–`2` — process and usage, emitted by the binary itself.
//! - `11`–`53` — the shared transport/auth/jql/adf/fields codes `jira-request`,
//!   `jira-auth`, `jira-jql` and `jira-fields` returned; read structurally from
//!   the client's `JiraFailure` (`bash_code(outcome)`), a `SurfaceError` or a
//!   `ClientError`, never parsed from a string.
//! - `60`–`133` — the per-flow argument and outcome codes, one block per flow.
//!
//! Safety-critical: the post-create "created remotely but unwritable" case maps
//! to `REQ_BAD_RESPONSE` (`16`) — Jira's `jira-emit-key.sh` returned it when a
//! create succeeded but yielded no usable key, warning the operator not to
//! blindly retry. The binary surfaces the key and steers to reconcile.
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
pub const FORBIDDEN: u8 = 12;
pub const NOT_FOUND: u8 = 13;
pub const GONE: u8 = 14;
pub const BAD_SITE: u8 = 15;
pub const REQ_BAD_RESPONSE: u8 = 16;
pub const REQ_BAD_PATH: u8 = 17;
pub const TEST_OVERRIDE_REJECTED: u8 = 18;
pub const RATELIMITED: u8 = 19;
pub const SERVER_ERROR: u8 = 20;
pub const REQ_CONNECT: u8 = 21;
pub const REQ_NO_CREDS: u8 = 22;
pub const TEST_HOOK_REJECTED: u8 = 23;

pub const NO_TOKEN: u8 = 24;
pub const TOKEN_CMD_FAILED: u8 = 25;
pub const AUTH_NO_SITE: u8 = 27;
pub const AUTH_NO_EMAIL: u8 = 28;
pub const LOCAL_PERMS_INSECURE: u8 = 29;

pub const JQL_NO_PROJECT: u8 = 30;
pub const JQL_UNSAFE_VALUE: u8 = 31;
pub const JQL_BAD_FLAG: u8 = 32;
pub const JQL_EMPTY_VALUE: u8 = 33;
pub const REQ_BAD_REQUEST: u8 = 34;

pub const BAD_JSON: u8 = 40;
pub const ADF_UNSUPPORTED: u8 = 41;
pub const ADF_BAD_INPUT: u8 = 42;

pub const FIELD_NOT_FOUND: u8 = 50;
pub const FIELD_CACHE_MISSING: u8 = 51;
pub const FIELD_CACHE_CORRUPT: u8 = 52;
pub const REFRESH_LOCKED: u8 = 53;

pub const INIT_NEEDS_CONFIG: u8 = 60;
pub const INIT_VERIFY_FAILED: u8 = 61;

// Remapped off the reserved 70-74 dispatch band; the fixture keeps 70-73.
pub const SEARCH_BAD_PAGE_TOKEN: u8 = 75;
pub const SEARCH_BAD_LIMIT: u8 = 76;
pub const SEARCH_NO_SITE_CACHE: u8 = 77;
pub const SEARCH_BAD_FLAG: u8 = 78;

pub const SHOW_NO_KEY: u8 = 80;
pub const SHOW_BAD_COMMENTS_LIMIT: u8 = 81;
pub const SHOW_BAD_FLAG: u8 = 82;

pub const RENDER_BAD_INPUT: u8 = 90;

pub const COMMENT_NO_SUBCOMMAND: u8 = 91;
pub const COMMENT_BAD_SUBCOMMAND: u8 = 92;
pub const COMMENT_NO_KEY: u8 = 93;
pub const COMMENT_NO_BODY: u8 = 94;
pub const COMMENT_NO_ID: u8 = 95;
pub const COMMENT_BAD_FLAG: u8 = 96;
pub const COMMENT_BAD_PAGE_SIZE: u8 = 97;
pub const COMMENT_BAD_VISIBILITY: u8 = 98;
pub const COMMENT_BAD_RESPONSE: u8 = 99;

pub const CREATE_NO_PROJECT: u8 = 100;
pub const CREATE_NO_TYPE: u8 = 101;
pub const CREATE_NO_SUMMARY: u8 = 102;
pub const CREATE_BAD_FIELD: u8 = 103;
pub const CREATE_BAD_FLAG: u8 = 104;
pub const CREATE_NO_BODY: u8 = 105;
pub const CREATE_NO_SITE_CACHE: u8 = 106;
pub const CREATE_BAD_ASSIGNEE: u8 = 107;

pub const RESOLVE_NO_PROJECT: u8 = 108;
pub const RESOLVE_ALREADY_SYNCED: u8 = 109;

pub const UPDATE_NO_KEY: u8 = 110;
pub const UPDATE_LABEL_MODE_CONFLICT: u8 = 111;
pub const UPDATE_NO_OPS: u8 = 112;
pub const UPDATE_BAD_FLAG: u8 = 113;
pub const UPDATE_BAD_FIELD: u8 = 114;
pub const UPDATE_NO_SITE_CACHE: u8 = 115;
pub const UPDATE_NO_BODY: u8 = 116;
pub const UPDATE_BAD_ASSIGNEE: u8 = 117;

pub const TRANSITION_NO_KEY: u8 = 120;
pub const TRANSITION_NO_STATE: u8 = 121;
pub const TRANSITION_NOT_FOUND: u8 = 122;
pub const TRANSITION_AMBIGUOUS: u8 = 123;
pub const TRANSITION_BAD_FLAG: u8 = 124;
pub const TRANSITION_NO_BODY: u8 = 125;
pub const TRANSITION_BAD_RESOLUTION: u8 = 126;

pub const ATTACH_NO_KEY: u8 = 130;
pub const ATTACH_NO_FILES: u8 = 131;
pub const ATTACH_FILE_MISSING: u8 = 132;
pub const ATTACH_BAD_FLAG: u8 = 133;

use jira_client::adf::AdfError;
use jira_client::cache::CacheError;
use jira_client::classify::{bash_code, Outcome};
use jira_client::{ClientError, JiraFailure, SurfaceError};
use tracker_support::CredentialError;

/// The exit code for a structured port-op failure (`create`/`update`). A wire
/// outcome carries the granular bash code; the post-create unwritable case is
/// Jira's `REQ_BAD_RESPONSE` — created remotely, do not blindly retry.
#[must_use]
pub fn for_failure(failure: &JiraFailure) -> u8 {
    match failure {
        JiraFailure::Wire { outcome, .. } => code_for_outcome(*outcome),
        JiraFailure::UnwritableIdentifier { .. } => REQ_BAD_RESPONSE,
        JiraFailure::UnsafeQueryId { .. } => JQL_UNSAFE_VALUE,
        JiraFailure::ComposeRejected { .. } => JQL_NO_PROJECT,
        JiraFailure::ReadFailure { .. } => SERVER_ERROR,
    }
}

/// The exit code for a surface-flow failure (`search`/`show`/`comment`/
/// `transition`/`attach`/`init`/`fields`).
#[must_use]
pub fn for_surface(error: &SurfaceError) -> u8 {
    match error {
        SurfaceError::Client(client) => for_client(client),
        SurfaceError::Adf(adf) => for_adf(adf),
        SurfaceError::Status { status, .. } => code_for_status(*status),
        SurfaceError::BadResponse { .. } => REQ_BAD_RESPONSE,
        SurfaceError::BadPageSize { .. } => COMMENT_BAD_PAGE_SIZE,
        SurfaceError::TransitionNotFound { .. } => TRANSITION_NOT_FOUND,
        SurfaceError::TransitionAmbiguous { .. } => TRANSITION_AMBIGUOUS,
        SurfaceError::BadFilename { .. }
        | SurfaceError::FileRefused { .. }
        | SurfaceError::PathEscapesRoot { .. }
        | SurfaceError::BoundaryExhausted => ATTACH_FILE_MISSING,
    }
}

/// The exit code for a client-construction or credential failure.
#[must_use]
pub const fn for_client(error: &ClientError) -> u8 {
    match error {
        ClientError::NoSite => AUTH_NO_SITE,
        ClientError::BadSite { .. } => BAD_SITE,
        ClientError::NoProject => CREATE_NO_PROJECT,
        ClientError::NoEmail => AUTH_NO_EMAIL,
        ClientError::Credential(credential) => for_credential(credential),
        ClientError::BadJql { .. } => JQL_NO_PROJECT,
        ClientError::BadIdentifier { .. } | ClientError::BadPath { .. } => {
            REQ_BAD_PATH
        }
        ClientError::Transport { .. } => REQ_CONNECT,
        ClientError::OversizedResponse { .. } => REQ_BAD_RESPONSE,
        ClientError::AllowlistFromSharedConfig
        | ClientError::ConfigUnreadable { .. }
        | ClientError::TlsUnavailable { .. } => ERROR,
    }
}

/// The exit code for a credential-resolution failure. Jira surfaces these
/// directly through `init verify`; a request-path failure flattens them to
/// `REQ_NO_CREDS` (`22`) at the call site, mirroring the bash `jira-request.sh`.
#[must_use]
pub const fn for_credential(error: &CredentialError) -> u8 {
    match error {
        CredentialError::NoToken { .. }
        | CredentialError::TokenCmdFromSharedConfig { .. }
        | CredentialError::TokenCmdFromTrackedFile { .. }
        | CredentialError::MalformedToken { .. } => NO_TOKEN,
        CredentialError::TokenCmdFailed { .. }
        | CredentialError::TokenCmdTimedOut { .. } => TOKEN_CMD_FAILED,
        CredentialError::LocalPermsInsecure { .. } => LOCAL_PERMS_INSECURE,
        CredentialError::ConfigUnreadable { .. } => ERROR,
    }
}

/// The exit code for a discovery-cache write failure.
#[must_use]
pub const fn for_cache(error: &CacheError) -> u8 {
    match error {
        CacheError::LockContended { .. } => REFRESH_LOCKED,
        CacheError::Incompatible { .. } => FIELD_CACHE_CORRUPT,
        CacheError::Io { .. } | CacheError::Serialise { .. } => ERROR,
    }
}

/// The exit code for an ADF conversion failure, read from the crate's own
/// `code()` band (`40`–`42`).
fn for_adf(error: &AdfError) -> u8 {
    u8::try_from(error.code()).unwrap_or(ADF_BAD_INPUT)
}

fn code_for_outcome(outcome: Outcome) -> u8 {
    u8::try_from(bash_code(outcome)).unwrap_or(ERROR)
}

fn code_for_status(status: u16) -> u8 {
    u8::try_from(bash_code(Outcome::Status(status))).unwrap_or(SERVER_ERROR)
}

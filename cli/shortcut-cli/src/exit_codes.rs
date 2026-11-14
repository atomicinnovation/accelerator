//! `accelerator-shortcut`'s exit codes: the document of record.
//!
//! Shortcut has no shell predecessor, so `tests/fixtures/captured-exit-codes.txt`
//! was written by hand from this file, and `exit_codes_parity.rs` holds the
//! two together. The bands follow Linear's:
//!
//! - `0`–`2`: process and usage.
//! - `11`–`53`: transport, auth and catalogue.
//! - `60`–`69`: init.
//! - `75` upward: one block per flow.
//!
//! `70`–`74` stay free: the dispatch layer reserves them, and a code there
//! reaching `accelerator-work` would read as a dispatch verdict.

// Every code is a declared contract the parity test reads as text; a code no
// handler yet returns is still part of the surface.
#![allow(dead_code)]

use config::consent::Refusal;
use config::consent::RefusalReason;
use config::credentials::CredentialError;
use shortcut_client::cache::CacheError;
use shortcut_client::catalogue::CatalogueError;
use shortcut_client::client::ReadFailure;
use shortcut_client::discovery::DiscoveryError;
use shortcut_client::story_state::StateRefusal;
use shortcut_client::writes::OwnerRefusal;
use shortcut_client::writes::WriteError;
use shortcut_client::ClientError;
use tracker::TrackerError;

pub const CLEAN: u8 = 0;
pub const ERROR: u8 = 1;
pub const USAGE: u8 = 2;

pub const UNAUTHORIZED: u8 = 11;
pub const FORBIDDEN: u8 = 13;
pub const BAD_RESPONSE: u8 = 16;
pub const SERVER_ERROR: u8 = 20;
pub const CONNECT: u8 = 21;
pub const NO_TOKEN: u8 = 24;
pub const TOKEN_CMD_FAILED: u8 = 25;
pub const TOKEN_MALFORMED: u8 = 27;
pub const LOCAL_PERMS_INSECURE: u8 = 29;
pub const NO_CATALOGUE: u8 = 30;
pub const NO_TEAM_KEY: u8 = 31;
pub const BAD_REQUEST: u8 = 34;
pub const RATELIMITED: u8 = 35;
pub const REFRESH_LOCKED: u8 = 53;

pub const INIT_NO_TEAM: u8 = 62;
pub const INIT_LISTING_TRUNCATED: u8 = 63;
pub const INIT_NEWER_CATALOGUE: u8 = 64;

pub const SEARCH_BAD_FILTER: u8 = 75;
pub const SEARCH_FAILED: u8 = 76;
pub const SEARCH_CAP_HIT: u8 = 79;

pub const SHOW_BAD_REFERENCE: u8 = 80;
pub const SHOW_FOREIGN_WORKSPACE: u8 = 81;
pub const SHOW_NOT_FOUND: u8 = 82;

pub const COMMENT_NO_BODY: u8 = 91;

pub const CREATE_NO_TITLE: u8 = 100;
pub const CREATE_BAD_FRONTMATTER: u8 = 101;
pub const CREATE_ALREADY_SYNCED: u8 = 102;
pub const CREATE_NO_BODY_FILE: u8 = 103;
pub const CREATE_WRITEBACK_FAILED: u8 = 107;

pub const UPDATE_NO_FIELDS: u8 = 111;
pub const OWNER_UNKNOWN: u8 = 115;
pub const OWNER_AMBIGUOUS: u8 = 116;

pub const TRANSITION_NO_STATE: u8 = 121;
pub const STATE_UNKNOWN: u8 = 122;
pub const STATE_AMBIGUOUS: u8 = 123;
pub const STATE_NO_WORKFLOW: u8 = 124;

pub const ATTACH_NO_URL: u8 = 131;
pub const ATTACH_BAD_URL: u8 = 134;

pub const WRITE_NOT_APPLIED: u8 = 140;
pub const WRITE_MAY_HAVE_APPLIED: u8 = 141;
pub const WRITE_REFUSED: u8 = 142;

#[must_use]
pub const fn for_client(error: &ClientError) -> u8 {
    match error {
        ClientError::Credential(CredentialError::Consent(rejection)) => {
            for_refusal(&rejection.fatal)
        }
        ClientError::Credential(_) | ClientError::NoToken => NO_TOKEN,
        ClientError::MalformedToken { .. } => TOKEN_MALFORMED,
        ClientError::NotSent { .. } | ClientError::Transport { .. } => CONNECT,
        ClientError::RequestInvalid { .. } => BAD_REQUEST,
        ClientError::OversizedResponse { .. }
        | ClientError::ForeignPage { .. } => BAD_RESPONSE,
        ClientError::NoCatalogue => NO_CATALOGUE,
        ClientError::Catalogue(cache) => for_cache(cache),
        ClientError::TlsUnavailable { .. }
        | ClientError::ConfigUnreadable { .. } => ERROR,
    }
}

#[must_use]
pub const fn for_cache(error: &CacheError) -> u8 {
    match error {
        CacheError::LockContended { .. } => REFRESH_LOCKED,
        CacheError::Incomplete { .. } => INIT_LISTING_TRUNCATED,
        CacheError::Catalogue(CatalogueError::NewerSchema { .. }) => {
            INIT_NEWER_CATALOGUE
        }
        CacheError::Io { .. } | CacheError::Catalogue(_) => ERROR,
    }
}

#[must_use]
pub const fn for_refusal(refusal: &Refusal) -> u8 {
    match refusal.reason() {
        RefusalReason::Provenance | RefusalReason::Value => NO_TOKEN,
        RefusalReason::Command => TOKEN_CMD_FAILED,
        RefusalReason::Malformed => TOKEN_MALFORMED,
        RefusalReason::PersonalFile => LOCAL_PERMS_INSECURE,
    }
}

#[must_use]
pub const fn for_status(status: u16) -> u8 {
    match status {
        401 => UNAUTHORIZED,
        403 => FORBIDDEN,
        400 | 422 => BAD_REQUEST,
        429 => RATELIMITED,
        500..=599 => SERVER_ERROR,
        _ => BAD_RESPONSE,
    }
}

#[must_use]
pub const fn for_discovery(error: &DiscoveryError) -> u8 {
    match error {
        DiscoveryError::Refused { status, .. } => for_status(*status),
        DiscoveryError::Client(client) => for_client(client),
        DiscoveryError::Unusable { .. }
        | DiscoveryError::RepeatedPage { .. } => BAD_RESPONSE,
    }
}

#[must_use]
pub const fn for_read(failure: &ReadFailure) -> u8 {
    match failure {
        ReadFailure::Status { status, .. } => for_status(*status),
        ReadFailure::Unsent(_) => CONNECT,
    }
}

#[must_use]
pub const fn for_search(error: &TrackerError) -> u8 {
    match error {
        TrackerError::Unconfigured { .. } => SEARCH_BAD_FILTER,
        TrackerError::Rejected { .. } => BAD_REQUEST,
        TrackerError::Retryable { .. } | TrackerError::Terminal { .. } => {
            SEARCH_FAILED
        }
    }
}

/// A refused write is a usage error when the caller named the wrong kind of
/// entity, and its own code otherwise.
#[must_use]
pub const fn for_write(error: &WriteError) -> u8 {
    match error {
        WriteError::NotAStory(_) => USAGE,
        WriteError::NotHttps(_) => ATTACH_BAD_URL,
        WriteError::State(StateRefusal::Unknown { .. }) => STATE_UNKNOWN,
        WriteError::State(StateRefusal::Ambiguous { .. }) => STATE_AMBIGUOUS,
        WriteError::State(StateRefusal::WorkflowMissing { .. }) => {
            STATE_NO_WORKFLOW
        }
        WriteError::Owner(OwnerRefusal::Unknown { .. }) => OWNER_UNKNOWN,
        WriteError::Owner(OwnerRefusal::Ambiguous { .. }) => OWNER_AMBIGUOUS,
        WriteError::Read(failure) => for_read(failure),
        WriteError::Tracker(error) => for_tracker_write(error),
    }
}

/// What a retry of a failed write would repeat: nothing landed, it may have
/// landed, or Shortcut refused it outright.
#[must_use]
pub const fn for_tracker_write(error: &TrackerError) -> u8 {
    match error {
        TrackerError::Retryable { .. } => WRITE_NOT_APPLIED,
        TrackerError::Terminal { .. } => WRITE_MAY_HAVE_APPLIED,
        TrackerError::Rejected { .. } => WRITE_REFUSED,
        TrackerError::Unconfigured { .. } => NO_TEAM_KEY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_status_class_has_its_own_code() {
        let rows = [
            (401, UNAUTHORIZED),
            (403, FORBIDDEN),
            (400, BAD_REQUEST),
            (422, BAD_REQUEST),
            (429, RATELIMITED),
            (503, SERVER_ERROR),
            (302, BAD_RESPONSE),
        ];
        for (status, code) in rows {
            assert_eq!(for_status(status), code, "{status}");
        }
    }

    #[test]
    fn a_missing_or_malformed_token_keeps_its_own_code() {
        assert_eq!(for_client(&ClientError::NoToken), NO_TOKEN);
        assert_eq!(
            for_client(&ClientError::MalformedToken {
                found: "a space".to_owned()
            }),
            TOKEN_MALFORMED
        );
        assert_eq!(for_client(&ClientError::NoCatalogue), NO_CATALOGUE);
    }

    #[test]
    fn an_incomplete_listing_is_an_init_code() {
        let error = CacheError::Incomplete {
            listing: "teams".to_owned(),
            cause: "the deadline passed".to_owned(),
        };
        assert_eq!(for_cache(&error), INIT_LISTING_TRUNCATED);
    }

    #[test]
    fn a_search_refused_on_configuration_is_a_filter_code() {
        let error = TrackerError::Unconfigured {
            detail: String::new(),
        };
        assert_eq!(for_search(&error), SEARCH_BAD_FILTER);
    }
}

//! Path validation: the structural rules on the encoded path, and the
//! decode-and-recheck pass that catches an encoded traversal.

#![allow(clippy::expect_used, clippy::panic)]

use jira_client::path::validate;
use jira_client::ClientError;

fn reason(path: &str) -> String {
    match validate(path).expect_err("the path must be refused") {
        ClientError::BadPath { reason, .. } => reason,
        other => panic!("{other}"),
    }
}

#[test]
fn a_well_formed_api_path_is_accepted() {
    for path in [
        "/rest/api/3/myself",
        "/rest/api/3/issue/ABC-1",
        "/rest/api/3/issue/ABC-1/comment/10001",
        "/rest/api/3/search/jql?maxResults=100",
        "/rest/api/3/issue/OWNER%2FREPO-1",
        "/rest/api/3/user?accountId=5f:a1",
    ] {
        assert!(validate(path).is_ok(), "{path} must be accepted");
    }
}

#[test]
fn a_path_outside_the_api_prefix_is_refused() {
    assert_eq!(reason("/rest/api/2/myself"), "not under /rest/api/3/");
    assert_eq!(reason("/rest/agile/1.0/board"), "not under /rest/api/3/");
    assert_eq!(reason("myself"), "not under /rest/api/3/");
}

#[test]
fn a_disallowed_character_is_refused() {
    assert_eq!(
        reason("/rest/api/3/issue/ABC 1"),
        "contains disallowed characters"
    );
    assert_eq!(
        reason("/rest/api/3/issue/<script>"),
        "contains disallowed characters"
    );
}

#[test]
fn a_literal_traversal_is_refused() {
    assert_eq!(
        reason("/rest/api/3/issue/../../mypermissions"),
        "path traversal sequence"
    );
}

#[test]
fn consecutive_slashes_are_refused() {
    assert_eq!(reason("/rest/api/3//issue"), "consecutive slashes");
}

#[test]
fn an_encoded_traversal_is_refused_after_decoding() {
    assert_eq!(
        reason("/rest/api/3/issue/%2e%2e/%2e%2e/mypermissions"),
        "path traversal sequence"
    );
}

#[test]
fn a_double_encoded_traversal_is_refused() {
    assert_eq!(
        reason("/rest/api/3/issue/%252e%252e%252fmypermissions"),
        "path traversal sequence"
    );
}

#[test]
fn an_encoded_control_character_is_refused() {
    assert_eq!(reason("/rest/api/3/issue/%00"), "control character");
    assert_eq!(
        reason("/rest/api/3/issue/%0d%0aHost:x"),
        "control character"
    );
}

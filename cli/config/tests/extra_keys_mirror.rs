//! Pins provider-client keys against the Rust catalogue.

#![allow(clippy::expect_used)]

#[test]
fn the_provider_client_keys_are_registered() {
    for key in ["jira.allowed_sites", "jira.site", "linear.team_id"] {
        assert!(
            config::catalogue::declared(key).is_some(),
            "{key} must be dumpable"
        );
    }
}

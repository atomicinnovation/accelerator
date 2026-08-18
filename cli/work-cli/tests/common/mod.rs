//! Shared helpers for the `accelerator-work` subprocess suites.
//!
//! `accelerator-work` is bin-only, so these tests drive the compiled binary
//! and cannot inject a fake registry. The one thing they must all get right is
//! not letting an inherited provider credential resolve a real client and
//! reach the network — so the credential-environment scrub lives here, once,
//! derived from the clients' own `TokenKeys` so it cannot drift from the ladder
//! it guards.

#![allow(dead_code, clippy::expect_used)]

use std::process::Command;

/// The environment variables the credential ladders read, so a scrubbed child
/// cannot resolve a real client from an inherited token. The four token names
/// come from each client's `token_keys`; the insecure-local override is the
/// fifth rung's gate and has no `TokenKeys` field of its own.
#[must_use]
pub fn provider_env_vars() -> Vec<String> {
    let mut vars = Vec::new();
    for keys in [
        jira_client::auth::token_keys().expect("jira token keys parse"),
        linear_client::auth::token_keys().expect("linear token keys parse"),
    ] {
        vars.push(keys.env.to_owned());
        vars.push(keys.env_command.to_owned());
    }
    vars.push("ACCELERATOR_ALLOW_INSECURE_LOCAL".to_owned());
    vars
}

/// Removes every provider credential variable from `command`'s environment.
pub fn scrub_provider_env(command: &mut Command) {
    for name in provider_env_vars() {
        command.env_remove(name);
    }
}

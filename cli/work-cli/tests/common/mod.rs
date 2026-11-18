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

use corpus::frontmatter_validation::pipeline::validate_text;
use corpus::frontmatter_validation::Violation;
use corpus_adapters::YamlFrontmatter;

/// The environment variables the credential ladders read, so a scrubbed child
/// cannot resolve a real client from an inherited token.
#[must_use]
pub fn provider_env_vars() -> Vec<String> {
    let mut vars = Vec::new();
    for keys in [
        jira_client::auth::token_keys().expect("jira token keys parse"),
        linear_client::auth::token_keys().expect("linear token keys parse"),
        shortcut_client::auth::token_keys().expect("shortcut token keys parse"),
    ] {
        for key in [keys.plaintext, keys.command.descriptor()] {
            vars.extend(key.overrides.iter().map(|name| (*name).to_owned()));
        }
    }
    vars
}

/// Removes every provider credential variable from `command`'s environment.
pub fn scrub_provider_env(command: &mut Command) {
    for name in provider_env_vars() {
        command.env_remove(name);
    }
}

pub const OMIT_WHEN_EMPTY: [&str; 7] = [
    "parent:",
    "blocks:",
    "blocked_by:",
    "derived_from:",
    "relates_to:",
    "source:",
    "external_id:",
];

pub fn frontmatter_of(content: &str) -> String {
    let (frontmatter, _) =
        work_adapters::sync::digest::split_frontmatter_and_body(content)
            .expect("a fenced frontmatter");
    frontmatter
}

pub fn omit_when_empty_lines(frontmatter: &str) -> Vec<&str> {
    frontmatter
        .lines()
        .filter(|line| OMIT_WHEN_EMPTY.iter().any(|key| line.starts_with(key)))
        .collect()
}

pub fn violations(content: &str) -> Vec<Violation> {
    validate_text(content, &YamlFrontmatter)
}

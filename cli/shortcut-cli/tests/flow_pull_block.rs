//! A malformed `shortcut.pull` block is refused before any request, naming
//! the file to fix.
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use support::Token;

#[test]
fn a_pull_value_that_is_not_a_block_of_settings_is_refused() {
    let dir = support::scratch(
        "---\nwork:\n  integration: shortcut\nshortcut:\n  pull: [a, b]\n---\n",
    );

    let output =
        support::run_with(dir.path(), &["show", "sc-1"], None, &Token::Present);

    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        support::stderr(&output)
    );
    assert!(
        support::stderr(&output).contains("must be a block of settings"),
        "{}",
        support::stderr(&output)
    );
}

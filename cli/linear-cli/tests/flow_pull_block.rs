#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use support::Token;

fn with_pull(block: &str) -> String {
    format!("---\nwork:\n  integration: linear\nlinear:\n  pull:{block}---\n")
}

fn refusal(block: &str) -> (Option<i32>, String) {
    let dir = support::scratch(&with_pull(block));
    let output =
        support::run_with(dir.path(), &["show", "X"], None, &Token::Present);
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_pull_value_that_is_not_a_block_of_settings_is_refused() {
    let (code, stderr) = refusal(" [a, b]\n");

    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        stderr,
        "the `pull` value must be a block of settings, not a scalar or list. \
         Fix the `pull` block in .accelerator/config.md.\n"
    );
}

#[test]
fn a_negative_pull_max_items_is_refused() {
    let (code, stderr) = refusal("\n    max_items: -1\n");

    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        stderr,
        "pull `max_items` must be a non-negative integer (0 refuses all) or \
         `unlimited` (got `-1`). Fix the `pull` block in \
         .accelerator/config.md.\n"
    );
}

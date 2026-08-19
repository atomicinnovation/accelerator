//! Proves the differential can fail.
//!
//! A comparison nobody has ever seen reject anything is indistinguishable
//! from one that returns `None` unconditionally, so the wrong classification
//! is planted here in a committed test rather than by hand, once, in a
//! session nobody can re-run.

#![allow(clippy::expect_used)]

mod support;

use support::{
    classify, disagreement, table, Class, RETRYABLE_STATUS, TERMINAL_STATUS,
};

#[test]
fn a_wrong_classification_is_reported_and_names_the_code() {
    let message =
        disagreement("jira", "create", 34, Class::Terminal, RETRYABLE_STATUS)
            .expect(
                "a transcription that disagrees with the bash must be reported",
            );

    assert!(message.contains("code 34"), "{message}");
    assert!(message.contains("transcribed terminal"), "{message}");
    assert!(message.contains("bash says retryable"), "{message}");
}

#[test]
fn a_status_that_is_neither_class_is_reported_as_such() {
    let message = disagreement("linear", "update", 21, Class::Terminal, 0)
        .expect(
            "a mapper returning something outside the taxonomy is a \
             disagreement",
        );

    assert!(message.contains("neither class (exit 0)"), "{message}");
}

#[test]
fn an_agreeing_classification_is_not_reported() {
    assert!(disagreement(
        "jira",
        "create",
        109,
        Class::Terminal,
        TERMINAL_STATUS
    )
    .is_none());
}

#[test]
fn the_default_arm_is_terminal_for_a_code_the_fixture_does_not_list() {
    let rows = table();
    assert_eq!(classify(&rows, "jira", "create", 200), Class::Terminal);
    assert_eq!(classify(&rows, "linear", "create", 200), Class::Terminal);
}

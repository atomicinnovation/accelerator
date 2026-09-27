//! Pins every push outcome's keyword and exit code against a frozen
//! expectation.
//!
//! The work skills branch on both the keyword `work create --push` prints and
//! the code it exits with, so the pairs are literals committed here rather
//! than derived from the code they guard.
#![allow(clippy::expect_used, clippy::panic)]

use work::sync::PushOutcome;

const FROZEN_KEYWORD_EXIT_CODES: &[(&str, u8)] = &[
    ("write-once", 0),
    ("retry", 70),
    ("local-save", 0),
    ("loud-terminal", 71),
    ("rejected", 75),
];

const EVERY_OUTCOME: &[PushOutcome] = &[
    PushOutcome::WriteOnce,
    PushOutcome::Retry,
    PushOutcome::LocalSave,
    PushOutcome::LoudTerminal,
    PushOutcome::Rejected,
];

#[test]
fn every_keyword_maps_to_its_frozen_exit_code() {
    for outcome in EVERY_OUTCOME {
        let (_, frozen) = FROZEN_KEYWORD_EXIT_CODES
            .iter()
            .find(|(keyword, _)| *keyword == outcome.keyword())
            .unwrap_or_else(|| {
                panic!("no frozen code for keyword {}", outcome.keyword())
            });
        assert_eq!(
            outcome.exit_code(),
            *frozen,
            "{} has drifted from its contract",
            outcome.keyword()
        );
    }
    assert_eq!(EVERY_OUTCOME.len(), FROZEN_KEYWORD_EXIT_CODES.len());
}

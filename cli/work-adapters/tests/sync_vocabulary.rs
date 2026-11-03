//! Pins every keyword `work sync` reports to a committed golden, which the
//! `sync-work-items` skill's structure test reads as its vocabulary.
#![allow(clippy::expect_used)]

use work::inventory::MalformedCause;
use work::inventory::UnidentifiedReason;
use work::sync::RefusalReason;
use work_adapters::sync::apply::FailureClass;
use work_adapters::sync::operation::SyncOperation;

const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/sync-vocabulary.golden"
);

const fn unidentified_reasons() -> [UnidentifiedReason; 5] {
    [
        UnidentifiedReason::Unreadable(String::new()),
        UnidentifiedReason::Undecodable,
        UnidentifiedReason::Malformed(MalformedCause::UnclosedFence),
        UnidentifiedReason::NoFrontmatter,
        UnidentifiedReason::NoId,
    ]
}

fn vocabulary() -> String {
    let operations = SyncOperation::ALL
        .iter()
        .map(|operation| ("operation", operation.keyword()));
    let classes = FailureClass::ALL
        .iter()
        .map(|class| ("class", class.keyword()));
    let refusals = RefusalReason::ALL
        .iter()
        .map(|reason| ("refusal", reason.token()));
    let unidentified = unidentified_reasons()
        .iter()
        .map(|reason| ("unidentified", reason.keyword()))
        .collect::<Vec<_>>();
    operations
        .chain(classes)
        .chain(refusals)
        .chain(unidentified)
        .map(|(family, keyword)| [family, "\t", keyword, "\n"].concat())
        .collect()
}

#[test]
fn every_reported_keyword_is_in_the_vocabulary_golden() {
    let golden = std::fs::read_to_string(GOLDEN).expect("golden readable");

    assert_eq!(vocabulary(), golden);
}

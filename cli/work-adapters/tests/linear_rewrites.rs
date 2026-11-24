//! Each cosmetic rewrite `RecordingTracker::rewriting` can apply reproduces
//! what Linear did to the captured fragments in
//! `tests/fixtures/linear-rewrites`.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod captures;

use captures::{fixture, fragments, read_back, TestError};
use tracker_test_support::Rewrite;

/// The rewrites the capture's README records for each fragment. `X09` and
/// `X20` turn a number into an ordered-list marker, which no rewrite here
/// reproduces. `X11`, `X14` and `X16`-`X25` are absent because the
/// simulator splits on whitespace and takes one dot-free name before the
/// last label, so it cannot model ports, multi-label names or boundaries.
fn recorded_rewrites() -> Vec<(&'static str, Vec<Rewrite>)> {
    vec![
        ("R01", vec![Rewrite::EscapeTildes]),
        ("R02", vec![]),
        ("R03", vec![Rewrite::BoldPerLine]),
        ("R04", vec![Rewrite::BoldOffCode]),
        ("R05", vec![Rewrite::AngleBracketDestinations]),
        ("R06", vec![Rewrite::AutolinkUrls]),
        ("R07", vec![Rewrite::StarBullets]),
        ("R08", vec![Rewrite::UppercaseTaskMarkers]),
        ("R09", vec![Rewrite::SpacedTableDelimiters]),
        ("R10", vec![Rewrite::StarBullets]),
        ("R11", vec![Rewrite::EscapeWrappedNumbers]),
        ("R12", vec![Rewrite::AutolinkDottedTokens]),
        (
            "R13",
            vec![Rewrite::JoinCodeSpanLines, Rewrite::StarBullets],
        ),
        ("X01", vec![Rewrite::JoinCodeSpanLines]),
        ("X02", vec![Rewrite::EscapeTildes, Rewrite::BoldPerLine]),
        (
            "X03",
            vec![Rewrite::BlankLineBeforeList, Rewrite::StarBullets],
        ),
        ("X04", vec![Rewrite::EscapeWrappedNumbers]),
        (
            "X05",
            vec![Rewrite::EscapeWrappedNumbers, Rewrite::StarBullets],
        ),
        (
            "X06",
            vec![Rewrite::EscapeWrappedNumbers, Rewrite::StarBullets],
        ),
        ("X07", vec![Rewrite::JoinCodeSpanLines]),
        ("X08", vec![Rewrite::JoinCodeSpanLines]),
        ("X10", vec![Rewrite::AutolinkDottedTokens]),
        ("X12", vec![Rewrite::AutolinkDottedTokens]),
        ("X13", vec![Rewrite::AutolinkDottedTokens]),
        ("X15", vec![Rewrite::AutolinkDottedTokens]),
    ]
}

#[test]
fn each_rewrite_reproduces_the_captured_linear_read_back(
) -> Result<(), TestError> {
    let pushed = fragments(&fixture("linear-rewrites/pushed.md")?, "## ");
    let read_back =
        fragments(&read_back("linear-rewrites/read-back.json")?, "## ");

    for (code, rewrites) in recorded_rewrites() {
        let rewritten = rewrites
            .iter()
            .fold(pushed[code].clone(), |body, rewrite| rewrite.apply(&body));

        assert_eq!(rewritten, read_back[code], "fragment {code}");
    }
    Ok(())
}

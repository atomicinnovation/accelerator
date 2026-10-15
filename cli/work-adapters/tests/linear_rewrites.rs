//! Each cosmetic rewrite `RecordingTracker::rewriting` can apply reproduces
//! what Linear did to the captured fragments in
//! `tests/fixtures/linear-rewrites`.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;

use tracker_test_support::Rewrite;

type TestError = Box<dyn std::error::Error>;

fn capture(name: &str) -> Result<String, TestError> {
    Ok(std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/linear-rewrites")
            .join(name),
    )?)
}

/// Each `## ` section's body, keyed by its heading's fragment code.
fn fragments(markdown: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut current: Option<(String, String)> = None;
    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            sections.extend(current.take());
            let code = heading.split_whitespace().next().unwrap_or_default();
            current = Some((code.to_owned(), String::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    sections.extend(current);
    sections
        .into_iter()
        .map(|(code, body)| (code, body.trim().to_owned()))
        .collect()
}

/// The rewrites the capture's README records for each fragment. `X09`
/// turns a number into an ordered-list marker, which changes meaning, so no
/// cosmetic rewrite reproduces it.
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
    ]
}

#[test]
fn each_rewrite_reproduces_the_captured_linear_read_back(
) -> Result<(), TestError> {
    let pushed = fragments(&capture("pushed.md")?);
    let read_back: serde_json::Value =
        serde_json::from_str(&capture("read-back.json")?)?;
    let read_back = fragments(
        read_back["data"]["issue"]["description"]
            .as_str()
            .ok_or("a captured description")?,
    );

    for (code, rewrites) in recorded_rewrites() {
        let rewritten = rewrites
            .iter()
            .fold(pushed[code].clone(), |body, rewrite| rewrite.apply(&body));

        assert_eq!(rewritten, read_back[code], "fragment {code}");
    }
    Ok(())
}

//! Each cosmetic rewrite `RecordingTracker::rewriting` can apply reproduces
//! what Linear did to the captured fragments in
//! `tests/fixtures/linear-rewrites`.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod captures;

use captures::{
    fixture, fragments, fragments_in_groups, lines, read_back, Fragments,
    TestError,
};
use document::markdown::{events, MarkdownEvent, MarkdownTag};
use tracker_test_support::Rewrite;

/// The rewrites the capture's README records for each fragment. `X09` and
/// `X20` turn a number into an ordered-list marker, which no rewrite here
/// reproduces. `X19`-`X25` are absent because the simulator works line by
/// line and models neither `mailto` links nor emphasis, headings, tables or
/// HTML.
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
        ("X11", vec![Rewrite::AutolinkDottedTokens]),
        ("X12", vec![Rewrite::AutolinkDottedTokens]),
        ("X13", vec![Rewrite::AutolinkDottedTokens]),
        ("X14", vec![Rewrite::AutolinkDottedTokens]),
        ("X15", vec![Rewrite::AutolinkDottedTokens]),
        ("X16", vec![Rewrite::AutolinkDottedTokens]),
        ("X17", vec![Rewrite::AutolinkDottedTokens]),
        ("X18", vec![Rewrite::AutolinkDottedTokens]),
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

const DOTTED_TOKEN_GROUPS: [&str; 9] = [
    "ext",
    "md-name",
    "rs-name",
    "before",
    "after",
    "case",
    "labels",
    "numeric",
    "ports-paths",
];

struct Probes {
    pushed: Fragments,
    read_back: Fragments,
}

impl Probes {
    fn dotted_token_probes(
        pushed: &str,
        read_back_path: &str,
    ) -> Result<Self, TestError> {
        Ok(Self {
            pushed: fragments_in_groups(
                &fixture(pushed)?,
                "#### ",
                &DOTTED_TOKEN_GROUPS,
            ),
            read_back: fragments(&read_back(read_back_path)?, "#### "),
        })
    }

    fn tld_sweep() -> Result<Self, TestError> {
        Ok(Self {
            pushed: lines(&fixture("linear-autolink-probes/tld-sweep.md")?),
            read_back: lines(&read_back(
                "linear-autolink-probes/tld-sweep.read-back.json",
            )?),
        })
    }
}

/// The text of each link to a bare host name, leaving out scheme URLs and
/// `mailto` links, which other rewrites make.
fn dotted_token_link_texts(markdown: &str) -> Vec<String> {
    let mut texts = Vec::new();
    let mut open: Option<(String, String)> = None;
    for (event, _) in events(markdown) {
        match (event, open.as_mut()) {
            (
                MarkdownEvent::Start(MarkdownTag::Link { destination, .. }),
                _,
            ) => {
                open = Some((destination, String::new()));
            }
            (
                MarkdownEvent::Text(text) | MarkdownEvent::Code(text),
                Some((_, link_text)),
            ) => link_text.push_str(&text),
            (MarkdownEvent::End(MarkdownTag::Link { .. }), _) => {
                texts.extend(open.take().and_then(|(destination, text)| {
                    (destination.starts_with("http://") && destination != text)
                        .then_some(text)
                }));
            }
            _ => {}
        }
    }
    texts
}

#[test]
fn the_simulator_links_what_linear_linked_in_every_dotted_token_probe(
) -> Result<(), TestError> {
    let corpora = [
        Probes::dotted_token_probes(
            "linear-autolink-probes/probes.md",
            "linear-autolink-probes/probes.created.json",
        )?,
        Probes::dotted_token_probes(
            "linear-autolink-probes/follow-ups.md",
            "linear-autolink-probes/follow-ups.read-back.json",
        )?,
        Probes::tld_sweep()?,
    ];
    let mut probed = 0;
    let mut failures = Vec::new();
    for corpus in &corpora {
        for (code, pushed) in &corpus.pushed {
            probed += 1;
            let simulated = dotted_token_link_texts(
                &Rewrite::AutolinkDottedTokens.apply(pushed),
            );
            let linked = dotted_token_link_texts(&corpus.read_back[code]);
            if simulated != linked {
                failures.push(format!(
                    "{code} {pushed:?}: simulated {simulated:?}, Linear \
                     linked {linked:?}"
                ));
            }
        }
    }
    assert!(probed > 1_000, "{probed} probes");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

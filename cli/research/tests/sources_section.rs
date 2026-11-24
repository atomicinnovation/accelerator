#![allow(clippy::expect_used)]

use research::topic::sources_section::tier_text::TierText;
use research::topic::sources_section::CitedSource;
use research::topic::sources_section::LineSpan;
use research::topic::sources_section::MetadataPair;
use research::topic::sources_section::SourcesSection;
use research::topic::sources_section::TierReading;
use research::topic::sources_section::UnreadLine;

fn reading(tier: &str) -> TierReading {
    match TierText::parse(tier) {
        Ok(tier_text) => TierReading::Parsed(tier_text),
        Err(rejection) => TierReading::Rejected(rejection.into_text()),
    }
}

fn entry(
    title: &str,
    url: &str,
    tier: &str,
    venue: Option<&str>,
) -> CitedSource {
    CitedSource {
        title: title.to_owned(),
        url: url.to_owned(),
        tier: reading(tier),
        venue: venue.map(str::to_owned),
        metadata: vec![],
    }
}

fn unread(line: usize, text: &str) -> UnreadLine {
    UnreadLine {
        line,
        text: text.to_owned(),
    }
}

fn pair(key: &str, value: &str) -> MetadataPair {
    MetadataPair {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

fn section(lines: &[&str]) -> String {
    [&["## Sources"], lines].concat().join("\n")
}

fn only_entry(body: &str) -> CitedSource {
    let section = SourcesSection::parse(body);
    assert!(section.unread().is_empty(), "{body}");
    assert_eq!(section.entries().len(), 1, "{body}");
    section.entries().first().cloned().expect(body)
}

const TEN_LINES: [&str; 10] = [
    "# Finding",
    "",
    "Some findings.",
    "## Sources",
    "- [A](https://a.example/) — tier-1 — a.example",
    "- [B](https://b.example/) — tier-2",
    "",
    "## Next",
    "",
    "More text.",
];

fn span_read_from(body: &str) -> LineSpan {
    SourcesSection::parse(body).span().expect(body)
}

fn running_to_the_end() -> [String; 2] {
    let without_final_newline = TEN_LINES[..7]
        .iter()
        .copied()
        .chain(["Some prose.", "", "- [C](https://c.example/) — tier-3"])
        .collect::<Vec<&str>>()
        .join("\n");
    let with_final_newline = format!("{without_final_newline}\n");
    [without_final_newline, with_final_newline]
}

#[test]
fn an_entry_carries_its_title_url_tier_and_venue() {
    let body = section(&[
        "- [The Notifier Pattern](https://blog.example/notifier/) — tier-2 — \
         blog.example",
    ]);
    assert_eq!(
        only_entry(&body),
        entry(
            "The Notifier Pattern",
            "https://blog.example/notifier/",
            "tier-2",
            Some("blog.example"),
        )
    );
}

#[test]
fn an_entry_with_nothing_after_its_tier_has_no_venue() {
    for line in [
        "- [T](https://a.example/) — tier-1",
        "- [T](https://a.example/) — tier-1 — ",
    ] {
        assert_eq!(
            only_entry(&section(&[line])),
            entry("T", "https://a.example/", "tier-1", None),
            "{line}"
        );
    }
}

#[test]
fn a_venue_may_itself_hold_the_separator() {
    let body =
        section(&["- [T](https://a.example/) — tier-2 — Journal — Series B"]);
    assert_eq!(
        only_entry(&body).venue.as_deref(),
        Some("Journal — Series B")
    );
}

#[test]
fn a_venue_may_hold_a_markdown_link() {
    let body = section(&[
        "- [T](https://a.example/) — tier-2 — see [x](https://b.example/)",
    ]);
    assert_eq!(
        only_entry(&body),
        entry(
            "T",
            "https://a.example/",
            "tier-2",
            Some("see [x](https://b.example/)"),
        )
    );
}

#[test]
fn a_venue_may_hold_a_markdown_link_and_the_separator() {
    let body = section(&[
        "- [T](https://a.example/) — tier-2 — see [x](https://b.example/) — \
         p. 4",
    ]);
    assert_eq!(
        only_entry(&body),
        entry(
            "T",
            "https://a.example/",
            "tier-2",
            Some("see [x](https://b.example/) — p. 4"),
        )
    );
}

#[test]
fn a_title_may_hold_brackets() {
    let body = section(&["- [A [draft]](https://a.example/) — tier-1"]);
    assert_eq!(only_entry(&body).title, "A [draft]");
}

#[test]
fn the_title_ends_at_the_last_link_close_that_still_reads_as_an_entry() {
    let body = section(&[
        "- [A](https://x.example/) — tier-1 — v](https://a.example/) — tier-2",
    ]);
    assert_eq!(
        only_entry(&body),
        entry(
            "A](https://x.example/) — tier-1 — v",
            "https://a.example/",
            "tier-2",
            None,
        )
    );
}

#[test]
fn a_link_line_that_breaks_the_token_rules_is_unread() {
    for line in [
        "- [T](https://a.example/)",
        "- [T](https://a.example/) — ",
        "- [](https://a.example/) — tier-1",
        "- [T]() — tier-1",
        "- [T](https://a.example/x_(y)) — tier-1",
        "- [T](https://a.example/a b) — tier-1",
    ] {
        let parsed = SourcesSection::parse(&section(&[line]));
        assert!(parsed.entries().is_empty(), "{line}");
        assert_eq!(parsed.unread(), [unread(2, line)], "{line}");
    }
}

#[test]
fn a_rejected_tier_keeps_its_entry_as_written() {
    let body = section(&[
        "- [A](https://a.example/) — tier-1 — a.example",
        "- [B](https://b.example/) — tier-4 — b.example",
        "- [C](https://c.example/) — tier-3 — c.example",
    ]);
    let parsed = SourcesSection::parse(&body);
    assert_eq!(
        parsed.entries(),
        [
            entry("A", "https://a.example/", "tier-1", Some("a.example")),
            CitedSource {
                tier: TierReading::Rejected("tier-4".to_owned()),
                ..entry("B", "https://b.example/", "tier-1", Some("b.example"))
            },
            entry("C", "https://c.example/", "tier-3", Some("c.example")),
        ]
    );
    for (line, rejected) in [
        ("- [T](https://a.example/) — blog.example", "blog.example"),
        ("- [T](https://a.example/) —  tier-1", " tier-1"),
    ] {
        assert_eq!(
            only_entry(&section(&[line])),
            CitedSource {
                tier: TierReading::Rejected(rejected.to_owned()),
                ..entry("T", "https://a.example/", "tier-1", None)
            },
            "{line}"
        );
    }
}

#[test]
fn a_source_listed_twice_is_two_entries() {
    let line = "- [T](https://a.example/) — tier-1";
    let parsed = SourcesSection::parse(&section(&[line, line]));
    let expected = entry("T", "https://a.example/", "tier-1", None);
    assert_eq!(parsed.entries(), [expected.clone(), expected]);
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Absent,
    DeclaredEmpty,
    Present,
}

const fn state_of(section: &SourcesSection) -> State {
    match section {
        SourcesSection::Absent => State::Absent,
        SourcesSection::DeclaredEmpty { .. } => State::DeclaredEmpty,
        SourcesSection::Present { .. } => State::Present,
    }
}

#[test]
fn the_section_state_is_absent_declared_empty_or_present() {
    let one_entry = "- [T](https://a.example/) — tier-1";
    for (body, state, entry_count, unread_lines) in [
        (
            "# Finding\n\nNo sources here.\n".to_owned(),
            State::Absent,
            0,
            vec![],
        ),
        (section(&["None found."]), State::DeclaredEmpty, 0, vec![]),
        (section(&[one_entry]), State::Present, 1, vec![]),
        (
            section(&["None found.", one_entry]),
            State::Present,
            1,
            vec![],
        ),
        (
            section(&["None found.", "Some prose."]),
            State::DeclaredEmpty,
            0,
            vec![unread(3, "Some prose.")],
        ),
        (
            section(&[" None found."]),
            State::Present,
            0,
            vec![unread(2, " None found.")],
        ),
        (
            "# Finding\n## Sources\n## Next\n".to_owned(),
            State::Present,
            0,
            vec![],
        ),
        (
            "# Finding\n## Sources\n".to_owned(),
            State::Present,
            0,
            vec![],
        ),
    ] {
        let parsed = SourcesSection::parse(&body);
        assert_eq!(state_of(&parsed), state, "{body}");
        assert_eq!(parsed.entries().len(), entry_count, "{body}");
        assert_eq!(parsed.unread(), unread_lines, "{body}");
    }
}

#[test]
fn a_heading_with_nothing_under_it_spans_only_its_own_line() {
    for body in [
        "# Finding\n## Sources\n## Next\n",
        "# Finding\n## Sources\n",
    ] {
        let span = SourcesSection::parse(body).span().expect(body);
        assert_eq!((span.start(), span.end()), (2, 3), "{body}");
    }
}

#[test]
fn blank_and_whitespace_only_lines_are_ignored() {
    let body = [
        "## Sources",
        "",
        "- [A](https://a.example/) — tier-1",
        "   ",
        "- [B](https://b.example/) — tier-2",
        "\t",
        "## Next",
    ]
    .join("\n");
    let parsed = SourcesSection::parse(&body);
    assert_eq!(
        parsed.entries(),
        [
            entry("A", "https://a.example/", "tier-1", None),
            entry("B", "https://b.example/", "tier-2", None),
        ]
    );
    assert!(parsed.unread().is_empty());
}

#[test]
fn only_the_first_exactly_named_section_is_read() {
    let body = [
        "## Sources",
        "- [A](https://a.example/) — tier-1",
        "## Sources",
        "- [B](https://b.example/) — tier-2",
    ]
    .join("\n");
    assert_eq!(
        SourcesSection::parse(&body).entries(),
        [entry("A", "https://a.example/", "tier-1", None)]
    );

    for heading in ["## sources", "## Sources and notes", "## Sources "] {
        let body = [heading, "- [A](https://a.example/) — tier-1"].join("\n");
        assert_eq!(
            SourcesSection::parse(&body),
            SourcesSection::Absent,
            "{heading}"
        );
    }
}

#[test]
fn the_section_closes_at_the_next_level_two_heading() {
    let body = [
        "## Sources",
        "- [A](https://a.example/) — tier-1",
        "### Notes",
        "## Appendix",
        "- [B](https://b.example/) — tier-2",
    ]
    .join("\n");
    let parsed = SourcesSection::parse(&body);
    assert_eq!(
        parsed.entries(),
        [entry("A", "https://a.example/", "tier-1", None)]
    );
    assert_eq!(parsed.unread(), [unread(3, "### Notes")]);
}

#[test]
fn unread_lines_are_reported_as_written_in_order() {
    let body = section(&[
        "  - doi: 10.1/x",
        "- [A](https://a.example/) — tier-1",
        "Some prose.",
        "- [B](https://b.example/) — tier-2",
    ]);
    let parsed = SourcesSection::parse(&body);
    assert_eq!(
        parsed.entries(),
        [
            entry("A", "https://a.example/", "tier-1", None),
            entry("B", "https://b.example/", "tier-2", None),
        ]
    );
    assert_eq!(
        parsed.unread(),
        [unread(2, "  - doi: 10.1/x"), unread(4, "Some prose.")]
    );
}

#[test]
fn the_span_runs_from_the_heading_to_the_line_after_the_section() {
    let span_of = |body: &str| {
        SourcesSection::parse(body)
            .span()
            .map(|span| (span.start(), span.end()))
    };

    assert_eq!(span_of(&TEN_LINES.join("\n")), Some((4, 8)));

    for body in running_to_the_end() {
        assert_eq!(span_of(&body), Some((4, 11)), "{body:?}");
    }

    assert_eq!(span_of(&section(&["None found."])), Some((1, 3)));
    assert_eq!(span_of("# Finding\n"), None);
}

#[test]
fn a_crlf_document_reads_as_its_lf_twin() {
    let lines = [
        "# Finding",
        "## Sources",
        "- [A](https://a.example/) — tier-1",
        "  - doi: 10.1/x",
        "Some prose.",
        "- [B](https://b.example/) — tier-2 — b.example",
        "## Next",
    ];
    let lf = SourcesSection::parse(&lines.join("\n"));
    let crlf = SourcesSection::parse(&lines.join("\r\n"));
    assert_eq!(crlf, lf);
    assert_eq!(crlf.entries().len(), 2);
    assert!(crlf.unread().iter().all(|line| !line.text.contains('\r')));
}

#[test]
fn only_one_trailing_carriage_return_is_dropped() {
    let body = "## Sources\nSome prose.\r\r\n";
    assert_eq!(
        SourcesSection::parse(body).unread(),
        [unread(2, "Some prose.\r")]
    );
}

#[test]
fn a_lone_carriage_return_does_not_break_a_line() {
    let body = section(&["a\rb", "Later prose."]);
    assert_eq!(
        SourcesSection::parse(&body).unread(),
        [unread(2, "a\rb"), unread(3, "Later prose.")]
    );
}

#[test]
fn a_metadata_block_belongs_to_the_entry_above_in_written_order() {
    let body = section(&[
        "- [A](https://a.example/) — tier-1",
        "  - doi: 10.1/a",
        "  - accessed: 2026-11-24",
        "- [B](https://b.example/) — tier-2",
    ]);
    let parsed = SourcesSection::parse(&body);
    assert_eq!(
        parsed.entries(),
        [
            CitedSource {
                metadata: vec![
                    pair("doi", "10.1/a"),
                    pair("accessed", "2026-11-24"),
                ],
                ..entry("A", "https://a.example/", "tier-1", None)
            },
            entry("B", "https://b.example/", "tier-2", None),
        ]
    );
    assert!(parsed.unread().is_empty());
}

#[test]
fn a_block_value_is_kept_as_written() {
    for (block_line, expected) in [
        (
            "  - pdf_url: https://a.example/x.pdf",
            pair("pdf_url", "https://a.example/x.pdf"),
        ),
        (
            "  - journal_ref: Phys. Rev.: A 12",
            pair("journal_ref", "Phys. Rev.: A 12"),
        ),
    ] {
        let body = section(&["- [T](https://a.example/) — tier-1", block_line]);
        assert_eq!(only_entry(&body).metadata, [expected], "{block_line}");
    }
}

#[test]
fn a_key_given_twice_yields_two_pairs() {
    let body = section(&[
        "- [T](https://a.example/) — tier-1",
        "  - doi: 10.1/a",
        "  - doi: 10.1/b",
    ]);
    assert_eq!(
        only_entry(&body).metadata,
        [pair("doi", "10.1/a"), pair("doi", "10.1/b")]
    );
}

#[test]
fn a_key_may_hold_digits_and_underscores_after_its_first_letter() {
    let body = section(&[
        "- [T](https://a.example/) — tier-1",
        "  - doi2: 10.1/a",
        "  - openalex_id: W1",
    ]);
    assert_eq!(
        only_entry(&body).metadata,
        [pair("doi2", "10.1/a"), pair("openalex_id", "W1")]
    );
}

#[test]
fn a_block_line_that_breaks_the_token_rules_is_unread() {
    for block_line in [
        "  - DOI: 10.1/x",
        "  - Published Date: 2019",
        "  - doi: ",
        "  - _doi: 10.1/x",
        "  - 2doi: 10.1/x",
        "  - doi:   ",
        "  - not a pair",
    ] {
        let parsed = SourcesSection::parse(&section(&[
            "- [T](https://a.example/) — tier-1",
            block_line,
        ]));
        assert_eq!(
            parsed.entries(),
            [entry("T", "https://a.example/", "tier-1", None)],
            "{block_line}"
        );
        assert_eq!(parsed.unread(), [unread(3, block_line)], "{block_line}");
    }
}

#[test]
fn blank_lines_do_not_separate_a_block_from_its_entry() {
    let body = section(&[
        "- [T](https://a.example/) — tier-1",
        "",
        "  ",
        "  - doi: 10.1/x",
    ]);
    assert_eq!(only_entry(&body).metadata, [pair("doi", "10.1/x")]);
}

#[test]
fn prose_separates_a_block_from_its_entry() {
    let parsed = SourcesSection::parse(&section(&[
        "- [T](https://a.example/) — tier-1",
        "Some prose.",
        "  - doi: 10.1/x",
    ]));
    assert_eq!(
        parsed.entries(),
        [entry("T", "https://a.example/", "tier-1", None)]
    );
    assert_eq!(
        parsed.unread(),
        [unread(3, "Some prose."), unread(4, "  - doi: 10.1/x")]
    );
}

#[test]
fn an_unread_link_separates_a_block_from_the_entry_before_it() {
    let parsed = SourcesSection::parse(&section(&[
        "- [A](https://a.example/) — tier-1",
        "- [B](https://b.example/)",
        "  - doi: 10.1/b",
    ]));
    assert_eq!(
        parsed.entries(),
        [entry("A", "https://a.example/", "tier-1", None)]
    );
    assert_eq!(
        parsed.unread(),
        [
            unread(3, "- [B](https://b.example/)"),
            unread(4, "  - doi: 10.1/b"),
        ]
    );
}

#[test]
fn none_found_separates_a_block_from_its_entry() {
    let parsed = SourcesSection::parse(&section(&[
        "- [T](https://a.example/) — tier-1",
        "None found.",
        "  - doi: 10.1/x",
    ]));
    assert_eq!(state_of(&parsed), State::Present);
    assert_eq!(
        parsed.entries(),
        [entry("T", "https://a.example/", "tier-1", None)]
    );
    assert_eq!(parsed.unread(), [unread(4, "  - doi: 10.1/x")]);
}

#[test]
fn splitting_at_the_section_gives_the_text_before_and_after_it() {
    for separator in ["\n", "\r\n"] {
        let body = TEN_LINES.join(separator);
        assert_eq!(
            span_read_from(&body).split(&body),
            (
                [&TEN_LINES[..3], &[""]].concat().join(separator).as_str(),
                TEN_LINES[7..].join(separator).as_str(),
            ),
            "{body:?}"
        );
    }

    for body in running_to_the_end() {
        assert_eq!(
            span_read_from(&body).split(&body),
            ("# Finding\n\nSome findings.\n", ""),
            "{body:?}"
        );
    }
}

#[test]
fn removing_the_section_keeps_every_other_line_as_written() {
    for separator in ["\n", "\r\n"] {
        let body = TEN_LINES.join(separator);
        let kept = [&TEN_LINES[..3], &TEN_LINES[7..]].concat();
        assert_eq!(
            span_read_from(&body).remove_from(&body),
            kept.join(separator),
            "{body:?}"
        );
    }

    for body in running_to_the_end() {
        assert_eq!(
            span_read_from(&body).remove_from(&body),
            "# Finding\n\nSome findings.\n",
            "{body:?}"
        );
    }
}

#[test]
fn the_citing_once_worked_example_reads_as_three_cited_sources() {
    let before = "# Finding\n\nThree citations of one source.\n\n";
    let sources = concat!(
        "## Sources\n",
        "- [The Notifier Pattern](https://blog.example/notifier/) — tier-2 — ",
        "blog.example\n",
        "  - accessed: 2026-10-02\n",
        "- [Notifier](http://blog.example/notifier) — tier-3 (retracted) — ",
        "Example Blog\n",
        "  - published: 2019\n",
        "- [Notifier](https://www.blog.example/notifier) — tier-1 — ",
        "blog.example\n",
        "  - openalex_id: W1\n",
        "  - doi: 10.example/notifier\n",
        "\n",
    );
    let after = "## Next\n\nWhat follows.\n";
    let body = [before, sources, after].concat();

    let parsed = SourcesSection::parse(&body);

    assert_eq!(
        parsed.entries(),
        [
            CitedSource {
                metadata: vec![pair("accessed", "2026-10-02")],
                ..entry(
                    "The Notifier Pattern",
                    "https://blog.example/notifier/",
                    "tier-2",
                    Some("blog.example"),
                )
            },
            CitedSource {
                metadata: vec![pair("published", "2019")],
                ..entry(
                    "Notifier",
                    "http://blog.example/notifier",
                    "tier-3 (retracted)",
                    Some("Example Blog"),
                )
            },
            CitedSource {
                metadata: vec![
                    pair("openalex_id", "W1"),
                    pair("doi", "10.example/notifier"),
                ],
                ..entry(
                    "Notifier",
                    "https://www.blog.example/notifier",
                    "tier-1",
                    Some("blog.example"),
                )
            },
        ]
    );
    let retracted = parsed.entries().iter().map(|cited| {
        matches!(
            &cited.tier,
            TierReading::Parsed(tier_text) if tier_text.retracted()
        )
    });
    assert_eq!(retracted.collect::<Vec<_>>(), [false, true, false]);
    assert_eq!(state_of(&parsed), State::Present);
    assert!(parsed.unread().is_empty());

    let span = parsed.span().expect(&body);
    assert_eq!((span.start(), span.end()), (5, 14));
    assert_eq!(span.split(&body), (before, after));
    assert_eq!(span.remove_from(&body), [before, after].concat());
}

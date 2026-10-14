use std::ops::Range;

use pulldown_cmark::{BlockQuoteKind, CowStr, Event, Parser, Tag, TagEnd};

use super::{
    github_extensions, renders_as_soft_on_github, unwrap_soft_line_breaks,
};

fn events(markdown: &str) -> Vec<(Event<'_>, Range<usize>)> {
    Parser::new_ext(markdown, github_extensions())
        .into_offset_iter()
        .collect()
}

fn offsets_of(
    markdown: &str,
    is_wanted: fn(&Event<'_>) -> bool,
) -> Vec<(usize, usize)> {
    events(markdown)
        .into_iter()
        .filter(|(event, _)| is_wanted(event))
        .map(|(_, range)| (range.start, range.end))
        .collect()
}

fn soft_break_offsets(markdown: &str) -> Vec<(usize, usize)> {
    offsets_of(markdown, |event| matches!(event, Event::SoftBreak))
}

fn hard_break_offsets(markdown: &str) -> Vec<(usize, usize)> {
    offsets_of(markdown, |event| matches!(event, Event::HardBreak))
}

fn normalised_events(markdown: &str) -> Vec<Event<'static>> {
    let mut normalised: Vec<Event<'static>> = Vec::new();
    for (event, range) in events(markdown) {
        let event = if renders_as_soft_on_github(markdown, &event, &range) {
            Event::Text(CowStr::from(" "))
        } else {
            event.into_static()
        };
        match (normalised.last_mut(), event) {
            (Some(Event::Text(preceding)), Event::Text(text)) => {
                *preceding = CowStr::from(format!("{preceding}{text}"));
            }
            (_, event) => normalised.push(event),
        }
    }
    normalised
}

#[track_caller]
fn assert_unwraps_with_line_endings(
    line_endings: &str,
    input: &str,
    expected: &str,
) {
    let unwrapped = unwrap_soft_line_breaks(input);
    assert_eq!(unwrapped, expected, "unwrap ({line_endings}) of {input:?}");
    assert_eq!(
        unwrap_soft_line_breaks(expected),
        expected,
        "idempotence ({line_endings}) of {input:?}"
    );
    assert_eq!(
        normalised_events(input),
        normalised_events(expected),
        "event stream ({line_endings}) of {input:?}"
    );
}

#[track_caller]
fn assert_unwraps(input: &str, expected: &str) {
    assert_unwraps_with_line_endings("LF", input, expected);
    assert_unwraps_with_line_endings(
        "CRLF",
        &input.replace('\n', "\r\n"),
        &expected.replace('\n', "\r\n"),
    );
}

#[track_caller]
fn assert_unchanged(input: &str) {
    assert_unwraps(input, input);
}

#[test]
fn a_soft_break_covers_only_its_line_ending() {
    assert_eq!(soft_break_offsets("foo \nbar"), [(4, 5)]);
    assert_eq!(
        soft_break_offsets("> first line\n> second line"),
        [(12, 13)]
    );
    assert_eq!(soft_break_offsets("> a  \n>   b\r\n>\t c"), [(11, 13)]);

    let link = events("[a\n](u)");
    let after_soft_break = link.windows(2).find_map(|pair| match pair {
        [(Event::SoftBreak, _), next] => Some(next.clone()),
        _ => None,
    });
    assert!(
        matches!(
            after_soft_break,
            Some((Event::End(TagEnd::Link), Range { start: 0, .. }))
        ),
        "{after_soft_break:?}"
    );
}

#[test]
fn a_hard_break_covers_its_marker_and_line_ending() {
    assert_eq!(hard_break_offsets("foo\t\t\nbar"), [(3, 6)]);
    assert_eq!(hard_break_offsets("foo\t  \nbar"), [(3, 7)]);
    assert_eq!(hard_break_offsets("> foo\t\t\n> bar"), [(5, 8)]);
    assert_eq!(hard_break_offsets("foo\\\nbar"), [(3, 5)]);
    assert_eq!(soft_break_offsets("foo\n \x0B\nbar"), [(3, 4)]);
    assert_eq!(hard_break_offsets("foo\n \x0B\nbar"), [(4, 7)]);
}

#[test]
fn a_wrapped_paragraph_joins_but_a_hard_break_and_code_survive() {
    assert_unwraps(
        "Wrapped\nparagraph  \nhard\nbreak\n\n```\ncode\nline\n```\n",
        "Wrapped paragraph  \nhard break\n\n```\ncode\nline\n```\n",
    );
}

#[test]
fn a_blockquote_continuation_drops_its_marker() {
    assert_unwraps("> first line\n> second line", "> first line second line");
}

#[test]
fn list_item_continuations_join() {
    assert_unwraps("- item\n  continued", "- item continued");
    assert_unwraps("- item\n\tcontinued", "- item continued");
    assert_unwraps("- item\nlazy", "- item lazy");
    assert_unwraps("1. item\n   continued", "1. item continued");
}

#[test]
fn nested_containers_join_inside_their_own_item() {
    assert_unwraps("> - item\n>   continued", "> - item continued");
    assert_unwraps("- a\n  - nested\n    more", "- a\n  - nested more");
    assert_unwraps("> first\nsecond", "> first second");
    assert_unwraps("- [ ] task\n  wrapped", "- [ ] task wrapped");
}

#[test]
fn a_line_that_cannot_interrupt_a_paragraph_joins_as_text() {
    assert_unwraps("foo\n2. bar", "foo 2. bar");
    assert_unwraps("foo\n-bar", "foo -bar");
    assert_unwraps("foo\n#5 bar", "foo #5 bar");
}

#[test]
fn a_marker_after_deep_indentation_stays_literal_text() {
    assert_unwraps("foo\n    > bar", "foo > bar");
}

#[test]
fn a_line_that_interrupts_a_paragraph_starts_its_own_block() {
    assert_unchanged("foo\n- bar");
    assert_unchanged("foo\n1. bar");
    assert_unchanged("foo\n# bar");
    assert_unchanged("foo\n> bar");
}

const FENCED_CODE: &str = "```rust\nlet a = 1;\nlet b = 2;\n```\n";
const INDENTED_CODE: &str = "    let a = 1;\n    let b = 2;\n";
const TABLE: &str = "| a | b |\n| - | - |\n| 1 | 2 |\n";
const HTML_BLOCK: &str = "<div>\nfirst\nsecond\n</div>\n";
const MULTI_LINE_CODE_SPAN: &str = "`code\nspan`\n";
const LINK_REFERENCE_DEFINITION: &str = "[r]:\n  https://example.com\n";
const TWO_TRAILING_SPACES: &str = "first  \nsecond\n";
const TRAILING_BACKSLASH: &str = "first\\\nsecond\n";

const PRESERVED_CONSTRUCTS: [&str; 8] = [
    FENCED_CODE,
    INDENTED_CODE,
    TABLE,
    HTML_BLOCK,
    MULTI_LINE_CODE_SPAN,
    LINK_REFERENCE_DEFINITION,
    TWO_TRAILING_SPACES,
    TRAILING_BACKSLASH,
];

#[test]
fn constructs_without_soft_breaks_are_byte_for_byte_unchanged() {
    for construct in PRESERVED_CONSTRUCTS {
        assert_unchanged(construct);
    }
}

#[test]
fn only_paragraph_soft_breaks_change_among_preserved_constructs() {
    let wrapped_paragraph = "A wrapped\nparagraph.\n";
    let joined_paragraph = "A wrapped paragraph.\n";
    let interleave = |paragraph: &str| {
        PRESERVED_CONSTRUCTS
            .iter()
            .flat_map(|construct| [paragraph, "\n", construct, "\n"])
            .chain([paragraph])
            .collect::<String>()
    };
    assert_unwraps(
        &interleave(wrapped_paragraph),
        &interleave(joined_paragraph),
    );
}

#[test]
fn a_table_interrupting_a_paragraph_stays_intact() {
    assert_unwraps(
        "Some\nwrapped text\n| a | b |\n| - | - |",
        "Some wrapped text\n| a | b |\n| - | - |",
    );
}

#[test]
fn a_setext_heading_joins_above_its_underline() {
    assert_unwraps("Release\nnotes\n=====", "Release notes\n=====");
}

#[test]
fn an_alert_keeps_its_marker_line() {
    let unwrapped = "> [!NOTE]\n> wrapped body";
    assert_unwraps("> [!NOTE]\n> wrapped\n> body", unwrapped);
    assert!(events(unwrapped).iter().any(|(event, _)| matches!(
        event,
        Event::Start(Tag::BlockQuote(Some(BlockQuoteKind::Note)))
    )));
}

#[test]
fn a_lowercase_alert_keeps_its_marker_line() {
    assert_unwraps("> [!note]\n> wrapped\n> body", "> [!note]\n> wrapped body");
}

#[test]
fn an_unknown_alert_kind_joins_as_quoted_text() {
    assert_unwraps("> [!UNKNOWN]\n> a\n> b", "> [!UNKNOWN] a b");
}

#[test]
fn lines_after_a_table_stay_its_rows() {
    assert_unchanged("| a |\n| - |\nrow\nmore");
}

#[test]
fn a_footnote_definition_joins_and_stays_a_definition() {
    let unwrapped = "Text[^1].\n\n[^1]: a wrapped definition";
    assert_unwraps("Text[^1].\n\n[^1]: a wrapped\n    definition", unwrapped);
    assert!(events(unwrapped).iter().any(|(event, _)| matches!(
        event,
        Event::Start(Tag::FootnoteDefinition(label)) if label.as_ref() == "1"
    )));
}

#[test]
fn a_footnote_definitions_lazy_continuation_joins() {
    assert_unwraps("x[^1]\n\n[^1]: a\nb", "x[^1]\n\n[^1]: a b");
}

#[test]
fn dollar_delimited_math_joins_as_text() {
    assert_unwraps("$a\nb$", "$a b$");
}

#[test]
fn trailing_whitespace_before_a_soft_break_is_absorbed() {
    assert_unwraps("foo \nbar", "foo bar");
    assert_unwraps("foo\t\nbar", "foo bar");
    assert_unwraps("foo\x0B\nbar", "foo bar");
    assert_unwraps("foo\x0C\nbar", "foo bar");
}

#[test]
fn whitespace_github_renders_as_soft_joins() {
    assert_unwraps("foo\t\t\nbar", "foo bar");
    assert_unwraps("foo \t\nbar", "foo bar");
    assert_unwraps("foo\t \nbar", "foo bar");
    assert_unwraps("foo  \t\nbar", "foo bar");
    assert_unwraps("foo \x0B\nbar", "foo bar");
    assert_unwraps("> foo\t\t\n> bar", "> foo bar");
    assert_unwraps("- foo\t\t\n  bar", "- foo bar");
}

#[test]
fn two_spaces_after_a_tab_stay_a_hard_break() {
    assert_unchanged("foo\t  \nbar");
}

#[test]
fn a_whitespace_only_continuation_line_joins_into_both_neighbours() {
    assert_unwraps("foo\n \x0B\nbar", "foo  bar");
}

#[test]
fn every_line_of_a_long_paragraph_joins() {
    assert_unwraps("one\ntwo\nthree\nfour", "one two three four");
    assert_unwraps("> one\n> two\n> three", "> one two three");
}

#[test]
fn a_soft_break_beside_multibyte_text_joins() {
    assert_unwraps("café —\nnaïve", "café — naïve");
    assert_unwraps("> café —\n> naïve", "> café — naïve");
}

#[test]
fn inline_html_keeps_its_newline() {
    assert_unchanged("foo <span\nclass=\"x\">bar</span>");
}

#[test]
fn an_escape_at_line_start_survives() {
    assert_unwraps("foo\n\\*bar", "foo \\*bar");
}

#[test]
fn a_soft_break_closing_a_link_joins_inside_its_text() {
    assert_unwraps("[a\n](u)", "[a ](u)");
    assert_unwraps("![a\n](u)", "![a ](u)");
    assert_unwraps("- [a\n  ](u)", "- [a ](u)");
    assert_unwraps("[a\n][r]\n\n[r]: u", "[a ][r]\n\n[r]: u");
}

#[test]
fn indented_lazy_and_inline_continuations_join() {
    assert_unwraps("foo\n   bar", "foo bar");
    assert_unwraps("[a\nb](u)", "[a b](u)");
    assert_unwraps("~~a\nb~~", "~~a b~~");
    assert_unwraps("> a\n  b", "> a b");
}

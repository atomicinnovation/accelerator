use std::ops::Range;

use document::markdown::{events, MarkdownEvent, MarkdownTag};

use super::{
    escape_for_push, escaped_or_original, is_top_level_domain,
    proposed_escapes, reads_alike, with_escapes, Escape,
};

const IANA_ROOT_ZONE: &str = include_str!("tlds-alpha-by-domain.txt");

#[test]
fn the_snapshot_is_iana_shaped() {
    let mut lines = IANA_ROOT_ZONE.lines();
    let version = lines.next().unwrap_or_default();
    assert!(version.starts_with("# Version"), "{version}");
    let labels: Vec<&str> = lines.collect();
    for label in &labels {
        assert!(
            !label.is_empty()
                && label.bytes().all(|byte| byte.is_ascii_uppercase()
                    || byte.is_ascii_digit()
                    || byte == b'-'),
            "{label:?}"
        );
    }
    assert!(labels.len() >= 1_400, "{} labels", labels.len());
}

#[test]
fn top_level_domains_ignore_case() {
    for label in ["io", "IO", "rs", "md", "pid"] {
        assert!(is_top_level_domain(label), "{label}");
    }
    for label in ["toml", "yaml", "g", "3"] {
        assert!(!is_top_level_domain(label), "{label}");
    }
}

fn texts(markdown: &str) -> Vec<(String, Range<usize>)> {
    events(markdown)
        .into_iter()
        .filter_map(|(event, range)| match event {
            MarkdownEvent::Text(text) => Some((text, range)),
            _ => None,
        })
        .collect()
}

#[test]
fn an_escaped_dot_splits_the_text_around_its_backslash() {
    assert_eq!(
        texts("crates\\.io"),
        [("crates".to_owned(), 0..6), (".io".to_owned(), 7..10)]
    );
}

#[test]
fn an_escaped_backslash_before_a_dot_leaves_the_dot_bare() {
    let markdown = "crates\\\\.io";
    let texts = texts(markdown);
    assert_eq!(
        texts,
        [("crates".to_owned(), 0..6), ("\\.io".to_owned(), 7..11)]
    );
    assert_eq!(markdown[texts[1].1.clone()], texts[1].0);
}

#[test]
fn a_multi_line_inline_tag_arrives_as_written() {
    let markdown = "a <span title=\"owned by\n0160. x\">t</span>\n";
    assert_eq!(
        events(markdown),
        [
            (MarkdownEvent::Start(MarkdownTag::Paragraph), 0..42),
            (MarkdownEvent::Text("a ".to_owned()), 0..2),
            (
                MarkdownEvent::InlineHtml(
                    "<span title=\"owned by\n0160. x\">".to_owned()
                ),
                2..33
            ),
            (MarkdownEvent::Text("t".to_owned()), 33..34),
            (MarkdownEvent::InlineHtml("</span>".to_owned()), 34..41),
            (MarkdownEvent::End(MarkdownTag::Paragraph), 0..42),
        ]
    );
}

#[test]
fn an_entity_yields_text_unlike_its_source() {
    let markdown = "a&amp;b";
    let texts = texts(markdown);
    assert_eq!(texts[1], ("&".to_owned(), 1..6));
    assert_ne!(markdown[texts[1].1.clone()], texts[1].0);
}

fn reading(markdown: &str) -> Vec<MarkdownEvent> {
    let mut reading: Vec<MarkdownEvent> = Vec::new();
    for (event, _) in events(markdown) {
        match (reading.last_mut(), event) {
            (
                Some(MarkdownEvent::Text(preceding)),
                MarkdownEvent::Text(text),
            ) => {
                preceding.push_str(&text);
            }
            (_, event) => reading.push(event),
        }
    }
    reading
}

fn is_insertion_of_backslashes(input: &str, output: &str) -> bool {
    let mut unmatched = input.bytes().peekable();
    for byte in output.bytes() {
        if unmatched.peek() == Some(&byte) {
            unmatched.next();
        } else if byte != b'\\' {
            return false;
        }
    }
    unmatched.next().is_none()
}

#[track_caller]
fn assert_escapes_with_line_endings(
    line_endings: &str,
    input: &str,
    expected: &str,
) {
    assert_eq!(
        with_escapes(input, proposed_escapes(input)),
        expected,
        "proposed escapes ({line_endings}) of {input:?}"
    );
    assert_eq!(
        escape_for_push(input),
        expected,
        "escape ({line_endings}) of {input:?}"
    );
    assert_eq!(
        escape_for_push(expected),
        expected,
        "idempotence ({line_endings}) of {input:?}"
    );
    assert!(
        is_insertion_of_backslashes(input, expected),
        "insertions only ({line_endings}) of {input:?}"
    );
    assert_eq!(
        reading(input),
        reading(expected),
        "reading ({line_endings}) of {input:?}"
    );
}

#[track_caller]
fn assert_escapes(input: &str, expected: &str) {
    assert_escapes_with_line_endings("LF", input, expected);
    assert_escapes_with_line_endings(
        "CRLF",
        &input.replace('\n', "\r\n"),
        &expected.replace('\n', "\r\n"),
    );
}

#[track_caller]
fn assert_unescaped(input: &str) {
    assert_escapes(input, input);
}

#[test]
fn a_lone_number_continuing_an_item_is_escaped() {
    assert_escapes("- owned by\n  0160.\n", "- owned by\n  0160\\.\n");
}

#[test]
fn a_lone_parenthesised_number_continuing_an_item_is_escaped() {
    assert_escapes("- owned by\n  0160)\n", "- owned by\n  0160\\)\n");
}

#[test]
fn a_lone_number_continuing_a_top_level_paragraph_is_escaped() {
    assert_escapes("owned by\n0160.\n", "owned by\n0160\\.\n");
}

#[test]
fn a_number_before_text_continuing_a_paragraph_is_escaped() {
    assert_escapes(
        "owned by\n0160. more text\n",
        "owned by\n0160\\. more text\n",
    );
}

#[test]
fn a_number_before_text_continuing_an_item_is_escaped() {
    assert_escapes(
        "- owned by\n  0160. more text\n",
        "- owned by\n  0160\\. more text\n",
    );
}

#[test]
fn a_parenthesised_number_before_text_continuing_an_item_is_escaped() {
    assert_escapes(
        "- owned by\n  0160) more text\n",
        "- owned by\n  0160\\) more text\n",
    );
}

#[test]
fn a_lone_number_continuing_a_blockquote_is_escaped() {
    assert_escapes("> owned by\n> 0160.\n", "> owned by\n> 0160\\.\n");
}

#[test]
fn a_lone_number_continuing_a_nested_item_is_escaped() {
    assert_escapes(
        "- a\n  - owned by\n    0160)\n",
        "- a\n  - owned by\n    0160\\)\n",
    );
}

#[test]
fn a_lone_number_continuing_an_ordered_item_is_escaped() {
    assert_escapes("1. owned by\n   0160.\n", "1. owned by\n   0160\\.\n");
}

#[test]
fn a_ten_digit_number_continuing_an_item_is_escaped() {
    assert_escapes(
        "- owned by\n  1234567890.\n",
        "- owned by\n  1234567890\\.\n",
    );
}

#[test]
fn a_lone_number_after_a_backslash_hard_break_is_escaped() {
    assert_escapes("hard\\\n0160.\n", "hard\\\n0160\\.\n");
}

#[test]
fn a_lone_number_after_a_whitespace_hard_break_is_escaped() {
    assert_escapes("hard  \n0160.\n", "hard  \n0160\\.\n");
}

#[test]
fn a_number_before_a_tab_is_escaped() {
    assert_escapes("foo\n0160.\tbar\n", "foo\n0160\\.\tbar\n");
}

#[test]
fn a_lone_number_after_a_blockquote_starts_a_list_in_commonmark_too() {
    assert_unescaped("> owned by\n0160.\n");
}

#[test]
fn a_lone_number_after_an_item_starts_a_list_in_commonmark_too() {
    assert_unescaped("- owned by\n0160.\n");
}

#[test]
fn a_lone_number_after_a_tab_indent_is_escaped() {
    assert_escapes("- owned by\n\t0160.\n", "- owned by\n\t0160\\.\n");
}

#[test]
fn a_lone_number_continuing_a_task_is_escaped() {
    assert_escapes("- [ ] owned by\n  0160.\n", "- [ ] owned by\n  0160\\.\n");
}

#[test]
fn a_lone_one_is_escaped() {
    assert_escapes("- owned by\n  1.\n", "- owned by\n  1\\.\n");
}

#[test]
fn a_lone_zero_is_escaped() {
    assert_escapes("- owned by\n  0.\n", "- owned by\n  0\\.\n");
}

#[test]
fn consecutive_lone_numbers_are_each_escaped() {
    assert_escapes(
        "- owned by\n  0160.\n  0161.\n",
        "- owned by\n  0160\\.\n  0161\\.\n",
    );
}

#[test]
fn a_list_interrupting_a_paragraph_is_left_alone() {
    assert_unescaped("foo\n1. text\n");
}

#[test]
fn a_list_is_left_alone() {
    assert_unescaped("1. a\n2. b\n");
}

#[test]
fn a_number_without_a_space_after_its_dot_is_left_alone() {
    assert_unescaped("foo\n2026.11 bar\n");
}

#[test]
fn a_number_on_the_first_line_is_left_alone() {
    assert_unescaped("0160.\n");
}

#[test]
fn an_escaped_number_is_left_alone() {
    assert_unescaped("foo\n0160\\.\n");
}

#[test]
fn a_number_in_link_text_is_left_alone() {
    assert_unescaped("[link\n0160. text](u)\n");
}

#[test]
fn a_number_in_image_text_is_left_alone() {
    assert_unescaped("![a\n0160. x](u)\n");
}

#[test]
fn a_number_in_a_code_span_is_left_alone() {
    assert_unescaped("`a\n0160.`");
}

#[test]
fn a_number_in_an_indented_code_block_is_left_alone() {
    assert_unescaped("    a\n    0160.\n");
}

#[test]
fn a_number_in_a_fenced_code_block_is_left_alone() {
    assert_unescaped("```\na\n0160.\n```");
}

#[test]
fn a_number_in_an_html_block_is_left_alone() {
    assert_unescaped("<div>\na\n0160.\n</div>\n");
}

#[test]
fn a_number_in_a_tag_attribute_is_left_alone() {
    assert_unescaped("a <span title=\"owned by\n0160. x\">t</span>\n");
}

#[test]
fn an_escape_in_text_reads_alike() {
    assert!(reads_alike("crates.io", "crates\\.io"));
}

#[test]
fn an_escape_that_ends_a_list_reads_differently() {
    assert!(!reads_alike("1. a\n", "1\\. a\n"));
}

#[test]
fn an_escape_in_a_code_span_reads_differently() {
    assert!(!reads_alike("`crates.io`", "`crates\\.io`"));
}

#[test]
fn an_escape_in_an_inline_tag_reads_differently() {
    assert!(!reads_alike("<a title=\"1.\">", "<a title=\"1\\.\">"));
}

#[test]
fn an_escape_in_an_html_block_reads_differently() {
    assert!(!reads_alike("<div>\n1.\n</div>\n", "<div>\n1\\.\n</div>\n"));
}

#[test]
fn an_escape_in_a_code_block_reads_differently() {
    assert!(!reads_alike("```\n1.\n```\n", "```\n1\\.\n```\n"));
}

#[test]
fn an_escape_in_an_autolink_destination_reads_differently() {
    assert!(!reads_alike("<https://crates.io>", "<https://crates\\.io>"));
}

#[test]
fn an_escape_that_changes_the_reading_is_abandoned() {
    let markdown = "`crates.io`";
    assert_eq!(
        escaped_or_original(markdown, vec![Escape { before: 7 }]),
        markdown
    );
}

use std::ops::Range;

use document::markdown::{events, MarkdownEvent, MarkdownTag};

use super::dotted_tokens::DottedToken;
use super::link_nesting::LinkNesting;
use super::scheme_urls::SchemeUrl;
use super::text_runs::TextRun;
use super::{
    dotted_tokens, escape_for_push, escaped_or_original, is_top_level_domain,
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

#[test]
fn file_names_and_hosts_ending_in_a_top_level_domain_are_escaped() {
    assert_escapes(
        "lib.rs README.md a.b.io:12",
        "lib\\.rs README\\.md a\\.b\\.io:12",
    );
}

#[test]
fn tokens_ending_in_other_labels_are_left_alone() {
    assert_unescaped("config.yaml v1.2.3 e.g. a.b.c");
}

#[test]
fn a_top_level_domain_in_capitals_is_escaped() {
    assert_escapes("Crates.IO", "Crates\\.IO");
}

#[test]
fn a_partly_escaped_token_has_its_other_dots_escaped() {
    assert_escapes("a.b\\.io", "a\\.b\\.io");
}

#[test]
fn a_sentence_final_dot_is_left_alone() {
    assert_escapes("crates.io.", "crates\\.io.");
}

#[test]
fn a_bracketed_token_is_escaped() {
    assert_escapes("(crates.io)", "(crates\\.io)");
}

#[test]
fn a_token_ending_a_path_is_escaped() {
    assert_escapes("src/client.rs", "src/client\\.rs");
}

#[test]
fn an_email_host_is_escaped() {
    assert_escapes("a@b.io", "a@b\\.io");
}

#[test]
fn every_dot_of_a_multi_label_host_is_escaped() {
    assert_escapes("www.crates.io", "www\\.crates\\.io");
}

#[test]
fn a_token_keeps_its_port_but_not_a_label_after_a_colon() {
    assert_escapes("jql.rs:167 and jql.rs:x", "jql\\.rs:167 and jql\\.rs:x");
}

#[test]
fn a_token_in_bold_is_escaped() {
    assert_escapes("**crates.io**", "**crates\\.io**");
}

#[test]
fn a_token_in_a_heading_is_escaped() {
    assert_escapes("# crates.io", "# crates\\.io");
}

#[test]
fn a_token_in_a_table_cell_is_escaped() {
    assert_escapes("| crates.io |\n| - |\n", "| crates\\.io |\n| - |\n");
}

#[test]
fn a_scheme_url_is_left_alone() {
    assert_unescaped("https://crates.io/x.");
}

#[test]
fn a_bracketed_scheme_url_is_left_alone() {
    assert_unescaped("(see https://crates.io)");
}

#[test]
fn a_mailto_url_is_left_alone() {
    assert_unescaped("mailto:a@b.io");
}

#[test]
fn a_token_after_a_sentence_final_scheme_url_is_escaped() {
    assert_escapes(
        "see https://crates.io/x. then crates.io",
        "see https://crates.io/x. then crates\\.io",
    );
}

#[test]
fn a_token_after_a_bracketed_scheme_url_is_escaped() {
    assert_escapes(
        "(https://crates.io). then crates.io",
        "(https://crates.io). then crates\\.io",
    );
}

#[test]
fn an_emphasised_scheme_url_is_left_alone() {
    assert_unescaped("*https://crates.io*");
}

#[test]
fn an_underscore_emphasised_scheme_url_is_left_alone() {
    assert_unescaped("_https://crates.io_");
}

#[test]
fn a_struck_scheme_url_is_left_alone() {
    assert_unescaped("~https://crates.io~");
}

#[test]
fn a_quoted_scheme_url_is_left_alone() {
    assert_unescaped("\"https://crates.io/x\"");
}

#[test]
fn a_scheme_url_after_a_colon_is_left_alone() {
    assert_unescaped("URL:https://crates.io");
}

#[test]
fn schemes_in_any_case_are_left_alone() {
    assert_unescaped("HTTPS://crates.io ftp://crates.io");
}

#[test]
fn a_scheme_url_ends_at_an_angle_bracket() {
    assert_unescaped("https://crates.io>x");
}

#[test]
fn a_host_after_an_unlinked_scheme_is_escaped() {
    assert_escapes(
        "ssh://github.com xhttps://crates.io",
        "ssh://github\\.com xhttps://crates\\.io",
    );
}

#[test]
fn a_token_after_a_scheme_url_closing_a_bracket_is_escaped() {
    assert_escapes(
        "https://crates.io). then crates.io",
        "https://crates.io). then crates\\.io",
    );
}

#[test]
fn a_token_after_non_ascii_text_is_escaped() {
    assert_escapes("naïve — crates.io", "naïve — crates\\.io");
}

#[test]
fn labels_hold_unicode_letters() {
    assert_escapes("café.io 日本.jp", "café\\.io 日本\\.jp");
}

#[test]
fn a_non_ascii_last_label_is_escaped() {
    assert_escapes("foo.рф", "foo\\.рф");
}

#[test]
fn a_token_after_a_tilde_is_escaped() {
    assert_escapes("~crates.io", "~crates\\.io");
}

#[test]
fn a_www_host_is_escaped() {
    assert_escapes("www.foo", "www\\.foo");
}

#[test]
fn a_markdown_file_name_is_escaped_wherever_it_sits() {
    assert_escapes(
        "foo.md/x foo.md:1 a@b.md",
        "foo\\.md/x foo\\.md:1 a@b\\.md",
    );
}

#[test]
fn a_token_after_a_colon_is_escaped() {
    assert_escapes("Note:crates.io", "Note:crates\\.io");
}

#[test]
fn a_token_after_a_doubled_dot_is_escaped() {
    assert_escapes("foo..bar.io", "foo..bar\\.io");
}

#[test]
fn a_token_after_an_identifier_and_a_colon_is_escaped() {
    assert_escapes("PP-906:server.pid", "PP-906:server\\.pid");
}

#[test]
fn a_dot_after_an_escaped_backslash_is_left_alone() {
    assert_unescaped("crates\\\\.io");
}

#[test]
fn a_dot_written_as_an_entity_is_left_alone() {
    assert_unescaped("crates&#46;io");
}

#[test]
fn a_token_in_a_code_span_is_left_alone() {
    assert_unescaped("`crates.io`");
}

#[test]
fn a_token_in_a_fenced_code_block_is_left_alone() {
    assert_unescaped("```\ncrates.io\n```\n");
}

#[test]
fn a_token_in_an_indented_code_block_is_left_alone() {
    assert_unescaped("    crates.io\n");
}

#[test]
fn a_token_in_link_text_is_left_alone() {
    assert_unescaped("[crates.io](u)");
}

#[test]
fn a_token_in_a_link_destination_is_left_alone() {
    assert_unescaped("[t](https://crates.io)");
}

#[test]
fn a_token_in_an_autolink_is_left_alone() {
    assert_unescaped("<https://crates.io>");
}

#[test]
fn a_token_in_an_html_block_is_left_alone() {
    assert_unescaped("<div>\ncrates.io\n</div>\n");
}

#[test]
fn a_token_in_a_tag_attribute_is_left_alone() {
    assert_unescaped("<a href=\"https://crates.io\">t</a>");
}

#[test]
fn an_escaped_token_is_left_alone() {
    assert_unescaped("crates\\.io");
}

#[test]
fn a_token_between_inline_tags_is_escaped() {
    assert_escapes("<span>crates.io</span>", "<span>crates\\.io</span>");
}

#[test]
fn an_entity_bounds_a_token() {
    assert_escapes("a&amp;crates.io", "a&amp;crates\\.io");
}

#[test]
fn dotted_tokens_in_plain_text_include_their_port() {
    let text = "see jql.rs:167, a.b.c and https://crates.io then www.foo.";
    let tokens: Vec<&str> =
        dotted_tokens(text).map(|token| &text[token]).collect();
    assert_eq!(tokens, ["jql.rs:167", "www.foo"]);
}

fn run_texts(markdown: &str) -> Vec<String> {
    TextRun::from_events(markdown, &events(markdown))
        .iter()
        .map(|run| run.characters.iter().map(|run| run.character).collect())
        .collect()
}

#[test]
fn a_text_run_reads_through_an_escape() {
    assert_eq!(run_texts("see crates\\.io"), ["see crates.io"]);
}

#[test]
fn a_text_run_marks_only_the_escaped_character() {
    let markdown = "a.b\\.io";
    let escaped: Vec<(char, usize)> =
        TextRun::from_events(markdown, &events(markdown))[0]
            .characters
            .iter()
            .filter(|run| run.escaped)
            .map(|run| (run.character, run.source))
            .collect();
    assert_eq!(escaped, [('.', 4)]);
}

#[test]
fn text_runs_end_at_breaks_tags_code_and_entities() {
    assert_eq!(
        run_texts("a\nb <i>c</i> `d` e&amp;f **g**"),
        ["a", "b ", "c", " ", " e", "f ", "g"]
    );
}

#[test]
fn text_runs_skip_links_and_images() {
    assert_eq!(run_texts("a [b](u) ![c](v) d"), ["a ", " ", " d"]);
}

fn url_texts(text: &str) -> Vec<String> {
    let run = TextRun::plain(text);
    SchemeUrl::spans(&run.characters)
        .into_iter()
        .map(|span| {
            run.characters[span]
                .iter()
                .map(|run| run.character)
                .collect()
        })
        .collect()
}

#[test]
fn a_scheme_url_sheds_trailing_punctuation_and_unbalanced_brackets() {
    assert_eq!(
        url_texts("(see https://a.io/x_(y)).\" and \"ftp://b.io\"!"),
        ["https://a.io/x_(y)", "ftp://b.io"]
    );
}

#[test]
fn a_scheme_url_needs_a_boundary_before_it() {
    assert_eq!(
        url_texts("xhttps://a.io :mailto:b@c.io a//d.io //e.io"),
        ["mailto:b@c.io", "//e.io"]
    );
}

fn token_texts(word: &str) -> Vec<String> {
    let run = TextRun::plain(word);
    DottedToken::in_word(&run.characters)
        .iter()
        .map(|token| word[token.source()].to_owned())
        .collect()
}

#[test]
fn a_word_yields_each_dotted_token_it_holds() {
    assert_eq!(
        token_texts("Note:a.b.io:12:c.d..e.f.:g"),
        ["a.b.io:12", "c.d", "e.f"]
    );
}

#[test]
fn a_single_label_is_not_a_dotted_token() {
    assert_eq!(token_texts("io:12"), Vec::<String>::new());
}

fn work_item_bodies() -> std::io::Result<Vec<(std::path::PathBuf, String)>> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../meta/work");
    let mut bodies = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "md") {
            let content = std::fs::read_to_string(&path)?;
            bodies.push((path, without_frontmatter(&content).to_owned()));
        }
    }
    Ok(bodies)
}

fn without_frontmatter(content: &str) -> &str {
    content
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(content, |(_, body)| body)
}

#[test]
fn every_work_item_body_escapes_safely() -> std::io::Result<()> {
    let bodies = work_item_bodies()?;
    assert!(
        bodies.len() >= 300,
        "read {} work-item bodies",
        bodies.len()
    );
    for (path, body) in bodies {
        let path = path.display();
        let escaped = escape_for_push(&body);
        assert_eq!(
            with_escapes(&body, proposed_escapes(&body)),
            escaped,
            "{path}: the escapes were abandoned"
        );
        assert_eq!(escape_for_push(&escaped), escaped, "{path}: idempotence");
        assert!(
            is_insertion_of_backslashes(&body, &escaped),
            "{path}: insertions only"
        );
        assert!(reads_alike(&body, &escaped), "{path}: reads alike");
    }
    Ok(())
}

#[test]
fn a_numeric_email_host_is_escaped() {
    assert_escapes("a@1.2.3.4", "a@1\\.2\\.3\\.4");
}

#[test]
fn a_bare_ip_address_is_left_alone() {
    assert_unescaped("1.2.3.4 127.0.0.1:8080");
}

fn link() -> MarkdownTag {
    MarkdownTag::Link {
        destination: "https://x.io".to_owned(),
        title: String::new(),
    }
}

#[test]
fn text_is_inside_a_link_until_its_end() {
    let mut nesting = LinkNesting::default();
    nesting.observe(&MarkdownEvent::Start(link()));
    nesting.observe(&MarkdownEvent::Start(MarkdownTag::Image {
        destination: "a.png".to_owned(),
        title: String::new(),
    }));
    nesting.observe(&MarkdownEvent::End(link()));
    assert!(!nesting.is_outside());

    nesting.observe(&MarkdownEvent::End(link()));
    assert!(nesting.is_outside());
}

#[test]
fn an_unmatched_link_end_leaves_text_outside_every_link() {
    let mut nesting = LinkNesting::default();
    nesting.observe(&MarkdownEvent::End(link()));
    nesting.observe(&MarkdownEvent::Text("a".to_owned()));
    assert!(nesting.is_outside());
}

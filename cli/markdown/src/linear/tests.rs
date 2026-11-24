use std::ops::Range;

use document::markdown::{events, MarkdownEvent, MarkdownTag};

use super::is_top_level_domain;

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

use document::markdown::{events, Alignment, MarkdownEvent, MarkdownTag};

fn without_offsets(source: &str) -> Vec<MarkdownEvent> {
    events(source).into_iter().map(|(event, _)| event).collect()
}

#[test]
fn a_paragraph_yields_text_and_soft_break_events_with_offsets() {
    assert_eq!(
        events("one\ntwo\n"),
        vec![
            (MarkdownEvent::Start(MarkdownTag::Paragraph), 0..8),
            (MarkdownEvent::Text("one".to_owned()), 0..3),
            (MarkdownEvent::SoftBreak, 3..4),
            (MarkdownEvent::Text("two".to_owned()), 4..7),
            (MarkdownEvent::End(MarkdownTag::Paragraph), 0..8),
        ]
    );
}

#[test]
fn angle_bracketed_destinations_are_unwrapped() {
    let link = MarkdownTag::Link {
        destination: "a b".to_owned(),
        title: String::new(),
    };
    assert_eq!(
        without_offsets("[text](<a b>)\n"),
        vec![
            MarkdownEvent::Start(MarkdownTag::Paragraph),
            MarkdownEvent::Start(link.clone()),
            MarkdownEvent::Text("text".to_owned()),
            MarkdownEvent::End(link),
            MarkdownEvent::End(MarkdownTag::Paragraph),
        ]
    );
}

#[test]
fn tables_tasks_and_strikethrough_are_recognised() {
    let recognised = without_offsets("| a |\n|:--|\n| b |\n\n- [x] ~~c~~\n");
    for expected in [
        MarkdownEvent::Start(MarkdownTag::Table(vec![Alignment::Left])),
        MarkdownEvent::Start(MarkdownTag::TableHead),
        MarkdownEvent::Start(MarkdownTag::TableRow),
        MarkdownEvent::Start(MarkdownTag::TableCell),
        MarkdownEvent::TaskListMarker(true),
        MarkdownEvent::Start(MarkdownTag::Strikethrough),
    ] {
        assert!(recognised.contains(&expected), "{expected:?} missing");
    }
}

#[test]
fn fenced_code_is_one_literal_event() {
    let source = "```rust\nlet x;\n## not a heading\n```\n";
    assert_eq!(
        events(source),
        vec![(
            MarkdownEvent::CodeBlock {
                info: "rust".to_owned(),
                literal: "let x;\n## not a heading\n".to_owned(),
            },
            0..source.trim_end().len(),
        )]
    );
}

#[test]
fn an_ordered_list_reports_its_start_number() {
    assert_eq!(
        without_offsets("3. a\n").first(),
        Some(&MarkdownEvent::Start(MarkdownTag::List(Some(3))))
    );
}

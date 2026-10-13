use corpus::body::{
    Alignment, Block, BodyParser, Inline, ListItem, TableCell, TableRow,
    TaskMarker,
};
use corpus_adapters::CommonMarkBody;

fn blocks(markdown: &str) -> Vec<Block> {
    CommonMarkBody
        .parse(markdown)
        .blocks()
        .iter()
        .map(|sourced| sourced.block.clone())
        .collect()
}

fn text(text: &str) -> Inline {
    Inline::Text(text.to_owned())
}

fn item(text_content: &str) -> ListItem {
    ListItem {
        task: None,
        blocks: vec![Block::Paragraph(vec![text(text_content)])],
    }
}

fn list_items(markdown: &str) -> Vec<ListItem> {
    match blocks(markdown).into_iter().next() {
        Some(Block::List { items, .. }) => items,
        _ => Vec::new(),
    }
}

#[test]
fn a_heading_paragraph_and_list_parse_into_the_model() {
    assert_eq!(
        blocks("# Title\n\nSome *emphasised* text\n\n- one\n- two\n"),
        vec![
            Block::Heading {
                level: 1,
                inlines: vec![text("Title")],
            },
            Block::Paragraph(vec![
                text("Some "),
                Inline::Emphasis(vec![text("emphasised")]),
                text(" text"),
            ]),
            Block::List {
                start: None,
                loose: false,
                items: vec![item("one"), item("two")],
            },
        ]
    );
}

#[test]
fn a_loose_list_is_marked_loose() {
    assert_eq!(
        blocks("1. one\n\n2. two\n"),
        vec![Block::List {
            start: Some(1),
            loose: true,
            items: vec![item("one"), item("two")],
        }]
    );
}

#[test]
fn a_task_marker_parses_as_checked_regardless_of_case() {
    let tasks: Vec<_> = list_items("- [x] lower\n- [X] upper\n- [ ] open\n")
        .into_iter()
        .map(|item| item.task)
        .collect();
    assert_eq!(
        tasks,
        vec![
            Some(TaskMarker::Done),
            Some(TaskMarker::Done),
            Some(TaskMarker::Open),
        ]
    );
}

#[test]
fn a_tight_item_holding_a_nested_list_keeps_its_text_as_a_paragraph() {
    let items = list_items("- outer\n  - inner\n");
    assert_eq!(
        items.first().map(|item| item.blocks.clone()),
        Some(vec![
            Block::Paragraph(vec![text("outer")]),
            Block::List {
                start: None,
                loose: false,
                items: vec![item("inner")],
            },
        ])
    );
}

#[test]
fn a_table_and_a_blockquote_parse_into_the_model() {
    let cell = |content: &str| TableCell(vec![text(content)]);
    assert_eq!(
        blocks("| a | b |\n|:-:|--:|\n| c | d |\n\n> quoted\n"),
        vec![
            Block::Table {
                alignments: vec![Alignment::Center, Alignment::Right],
                rows: vec![
                    TableRow(vec![cell("a"), cell("b")]),
                    TableRow(vec![cell("c"), cell("d")]),
                ],
            },
            Block::BlockQuote(vec![Block::Paragraph(vec![text("quoted")])]),
        ]
    );
}

#[test]
fn section_source_text_matches_the_input_slice() {
    let markdown = "# Title\n\nintro\n\n## One\n\nfirst\n\n## Two\n\nsecond\n";
    let body = CommonMarkBody.parse(markdown);
    let sources: Vec<_> = body
        .sections()
        .iter()
        .map(|section| section.source)
        .collect();
    assert_eq!(
        sources,
        vec![
            "# Title\n\nintro\n\n",
            "## One\n\nfirst\n\n",
            "## Two\n\nsecond\n",
        ]
    );
    assert_eq!(sources.concat(), markdown);
}

#[test]
fn a_level_two_heading_inside_fenced_code_does_not_split() {
    let markdown = "## One\n\n```md\n## not a section\n```\n";
    let body = CommonMarkBody.parse(markdown);
    let headings: Vec<_> = body
        .sections()
        .iter()
        .map(|section| section.heading)
        .collect();
    assert_eq!(headings, vec![None, Some("## One\n")]);
}

#[test]
fn a_setext_level_two_heading_starts_a_section() {
    let markdown = "intro\n\nOne\n---\n\nfirst\n";
    let body = CommonMarkBody.parse(markdown);
    let sections = body.sections();
    assert_eq!(
        sections
            .iter()
            .map(|section| section.heading)
            .collect::<Vec<_>>(),
        vec![None, Some("One\n---\n")]
    );
    assert_eq!(
        sections.last().map(|section| section.source),
        Some("One\n---\n\nfirst\n")
    );
}

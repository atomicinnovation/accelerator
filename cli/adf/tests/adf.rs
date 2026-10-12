//! The ADF conversion's stated behaviours, including every deliberate lossy
//! one. A test pinning a quirk names `tests/fixtures/adf-fidelity-quirks.txt`,
//! which carries the reason it is preserved.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;

use adf::{
    document_to_markdown, document_to_markdown_strict, markdown_to_document,
    AdfError,
};
use serde_json::json;
use serde_json::Value;

const QUIRKS: &str = "tests/fixtures/adf-fidelity-quirks.txt";
const SEED: Option<&str> = Some("1");

fn fixture(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .expect("the committed fixture is readable")
}

fn quirk(name: &str) -> String {
    let quirks = fixture(QUIRKS);
    assert!(
        quirks.contains(&format!("quirk {name}")),
        "{name} must be recorded in {QUIRKS} with its rationale"
    );
    name.to_owned()
}

fn render(document: &Value) -> String {
    document_to_markdown(document).expect("the document renders")
}

fn assemble(markdown: &str) -> Value {
    markdown_to_document(markdown, SEED).expect("the markdown assembles")
}

fn refusal(markdown: &str) -> AdfError {
    markdown_to_document(markdown, SEED).expect_err("the markdown is refused")
}

fn strict(document: &Value) -> Result<String, AdfError> {
    document_to_markdown_strict(document)
}

fn round_trip(markdown: &str) -> String {
    strict(&assemble(markdown)).expect("the assembled document renders")
}

fn paragraph(inlines: &Value) -> Value {
    json!({"type": "doc", "content": [{"type": "paragraph", "content": inlines}]})
}

#[test]
fn both_placeholder_strings_are_verbatim_and_position_dependent() {
    let block = render(&json!({
        "type": "doc",
        "content": [{"type": "panel"}]
    }));
    let inline = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [{"type": "emoji"}]}]
    }));

    assert_eq!(block, "[unsupported ADF node: panel]");
    assert_eq!(inline, "[unsupported ADF inline: emoji]");
}

#[test]
fn the_one_remaining_refusal_carries_its_message() {
    let error = refusal("bad\u{1f}byte\n");

    assert_eq!(
        error.to_string(),
        "E_ADF_BAD_INPUT: input contains control byte \\x1e or \\x1f"
    );
    for markdown in ["> quoted\n", "| a | b |\n", "  - nested\n"] {
        assert!(
            markdown_to_document(markdown, SEED).is_ok(),
            "{markdown:?} assembles"
        );
    }
}

#[test]
fn the_round_trip_is_asymmetric_only_for_underline_and_unknown_nodes() {
    let inventory = fixture("tests/fixtures/adf-node-types.txt");
    let rejected: Vec<&str> = inventory
        .lines()
        .filter(|line| line.starts_with("node "))
        .filter(|line| line.contains("hard-reject"))
        .collect();
    assert!(
        rejected.is_empty(),
        "assemble hard-rejects no construct: {rejected:?}"
    );

    let underlined = paragraph(&json!([{
        "type": "text", "text": "under", "marks": [{"type": "underline"}]
    }]));
    assert_eq!(
        assemble(&render(&underlined))["content"][0]["content"],
        json!([{"type": "text", "text": "under"}]),
        "underline does not survive the round trip"
    );

    let panel = json!({"type": "doc", "content": [{"type": "panel"}]});
    assert_eq!(
        assemble(&render(&panel))["content"][0]["type"],
        "paragraph",
        "an unknown node comes back as its placeholder text"
    );
}

#[test]
fn an_ordered_list_assembles_with_the_start_its_markdown_used() {
    let document = assemble("3. three\n");
    let list = &document["content"][0];

    assert_eq!(list["type"], "orderedList");
    assert_eq!(list["attrs"]["order"], 3);
    assert_eq!(render(&document), "3. three");
}

#[test]
fn an_empty_document_renders_to_zero_bytes() {
    assert_eq!(render(&json!({"type": "doc", "content": []})), "");
    assert_eq!(render(&json!({"type": "doc"})), "");
}

#[test]
fn an_unbalanced_emphasis_yields_three_unmerged_nodes() {
    let _ = quirk("unbalanced-emphasis-becomes-an-empty-marked-node");
    let document = assemble("a*b\n");
    let inlines = document["content"][0]["content"]
        .as_array()
        .expect("the paragraph has inlines");

    assert_eq!(inlines.len(), 3, "adjacent text nodes are never merged");
    assert_eq!(inlines[0]["text"], "a");
    assert_eq!(
        inlines[1],
        json!({"type": "text", "text": "", "marks": [{"type": "em"}]}),
        "the middle node is an em-marked empty string, not a literal asterisk"
    );
    assert_eq!(inlines[2]["text"], "b");
}

#[test]
fn a_rejected_href_drops_the_mark_and_keeps_the_text_bare() {
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [{
            "type": "text",
            "text": "click",
            "marks": [{"type": "link", "attrs": {"href": "javascript:alert(1)"}}]
        }]}]
    }));

    assert_eq!(rendered, "click", "no placeholder, no link, just the text");
}

#[test]
fn an_accepted_href_is_emitted_untrimmed() {
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [{
            "type": "text",
            "text": "padded",
            "marks": [{"type": "link", "attrs": {"href": "  https://x"}}]
        }]}]
    }));

    assert_eq!(
        rendered, "[padded](  https://x)",
        "only the scheme check trims; the emitted href is the original"
    );
}

#[test]
fn each_render_abort_condition_produces_its_typed_error() {
    let heading = document_to_markdown(&json!({
        "type": "doc",
        "content": [{"type": "heading", "content": []}]
    }))
    .expect_err("a heading with no level aborts");
    assert_eq!(heading, AdfError::HeadingWithoutLevel);

    let root = document_to_markdown(&json!({"type": "paragraph"}))
        .expect_err("a non-doc root aborts");
    assert!(matches!(root, AdfError::RootNotDoc { .. }));

    for node in ["bulletList", "orderedList", "taskList"] {
        let error = document_to_markdown(&json!({
            "type": "doc",
            "content": [{"type": node}]
        }))
        .expect_err("a list with no content aborts");
        assert!(
            matches!(error, AdfError::ListWithoutContent { .. }),
            "{node}: {error}"
        );
    }
}

#[test]
fn the_marks_pipeline_nests_in_one_fixed_order_whatever_the_array_order() {
    let forward = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [{
            "type": "text", "text": "x",
            "marks": [{"type": "code"}, {"type": "em"}, {"type": "strong"}]
        }]}]
    }));
    let reversed = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [{
            "type": "text", "text": "x",
            "marks": [{"type": "strong"}, {"type": "em"}, {"type": "code"}]
        }]}]
    }));

    assert_eq!(forward, "***`x`***");
    assert_eq!(reversed, forward, "membership decides, not array order");
}

#[test]
fn strike_renders_and_other_marks_outside_the_pipeline_are_dropped() {
    let _ = quirk("marks-ignored-without-a-placeholder");
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [
            {"type": "text", "text": "struck", "marks": [{"type": "strike"}]},
            {"type": "text", "text": "under", "marks": [{"type": "underline"}]}
        ]}]
    }));

    assert_eq!(rendered, "~~struck~~under");
}

#[test]
fn a_list_item_renders_every_child() {
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "bulletList", "content": [{
            "type": "listItem",
            "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": "first"}]},
                {"type": "paragraph", "content": [{"type": "text", "text": "second"}]},
                {"type": "bulletList", "content": [{"type": "listItem", "content": [
                    {"type": "paragraph", "content": [{"type": "text", "text": "nested"}]}
                ]}]}
            ]
        }]}]
    }));

    assert_eq!(rendered, "- first\n\n  second\n  - nested");
}

#[test]
fn a_nested_list_as_a_first_child_renders_as_a_nested_list() {
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "bulletList", "content": [{
            "type": "listItem",
            "content": [{"type": "bulletList", "content": [{"type": "listItem", "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": "deep"}]}
            ]}]}]
        }]}]
    }));

    assert_eq!(rendered, "- \n  - deep");
    assert_eq!(round_trip(&rendered), rendered);
}

#[test]
fn a_code_block_renders_its_first_text_child_only() {
    let _ = quirk("codeblock-first-child-only");
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "codeBlock", "attrs": {"language": "sh"}, "content": [
            {"type": "text", "text": "kept"},
            {"type": "text", "text": "dropped"}
        ]}]
    }));

    assert_eq!(rendered, "```sh\nkept\n```");
}

#[test]
fn the_table_guard_is_narrow_by_design() {
    let _ = quirk("narrow-table-guard");
    let document = assemble("| a | b\n");

    assert_eq!(document["content"][0]["type"], "paragraph");
    assert_eq!(document["content"][0]["content"][0]["text"], "| a | b");
}

#[test]
fn an_indented_continuation_line_stays_a_paragraph_continuation() {
    let document = assemble("paragraph\n  -word continuation\n");

    assert_eq!(
        document["content"],
        json!([{"type": "paragraph", "content": [
            {"type": "text", "text": "paragraph   -word continuation"}
        ]}])
    );
}

#[test]
fn a_hard_break_is_two_spaces_then_a_newline_in_a_paragraph_only() {
    let _ = quirk("hard-break-only-in-a-paragraph");
    let paragraph = assemble("before  \nafter\n");
    assert_eq!(paragraph["content"][0]["content"][1]["type"], "hardBreak");
    assert_eq!(
        render(&paragraph),
        "before  \nafter",
        "the hard break renders as exactly two spaces and a newline"
    );

    let heading = assemble("## trailing  \n");
    assert_eq!(
        heading["content"][0]["content"][0]["text"], "trailing  ",
        "a heading keeps the spaces inside its text"
    );
}

#[test]
fn an_empty_link_text_contributes_nothing() {
    let _ = quirk("empty-link-text-is-dropped");
    let document = assemble("a [](https://x) b\n");
    let inlines = document["content"][0]["content"]
        .as_array()
        .expect("inlines");
    let text: String = inlines
        .iter()
        .map(|node| node["text"].as_str().unwrap_or_default())
        .collect();

    assert_eq!(text, "a  b", "neither a link nor its brackets survive");
}

#[test]
fn only_text_that_would_read_as_syntax_is_escaped() {
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [
            {"type": "text", "text": "*not em* `not code` [not a link] # | _x_"}
        ]}]
    }));

    assert_eq!(
        rendered,
        "\\*not em\\* \\`not code\\` \\[not a link] # | \\_x\\_"
    );
}

#[test]
fn a_seeded_local_id_is_deterministic_and_an_unseeded_one_is_a_counter() {
    let seeded = markdown_to_document("- [ ] task\n", Some("1"))
        .expect("the markdown assembles");
    let counted = markdown_to_document("- [ ] task\n", None)
        .expect("the markdown assembles");

    assert_eq!(
        seeded["content"][0]["content"][0]["attrs"]["localId"],
        "00000000-0000-4000-8000-000000000001"
    );
    assert_eq!(
        counted["content"][0]["content"][0]["attrs"]["localId"], "1",
        "without a seed the bare counter is used, as the oracle does"
    );
}

#[test]
fn a_blockquote_round_trips() {
    let markdown = "> quoted\n>\n> second";
    let document = assemble(markdown);

    assert_eq!(
        document["content"],
        json!([{"type": "blockquote", "content": [
            {"type": "paragraph", "content": [{"type": "text", "text": "quoted"}]},
            {"type": "paragraph", "content": [{"type": "text", "text": "second"}]}
        ]}])
    );
    assert_eq!(render(&document), markdown);
}

#[test]
fn a_pipe_table_round_trips_with_a_header_row() {
    let markdown = "| h1 | h2 |\n| --- | --- |\n| a | b |";
    let document = assemble(markdown);
    let rows = &document["content"][0]["content"];

    assert_eq!(document["content"][0]["type"], "table");
    assert_eq!(rows[0]["type"], "tableRow");
    assert_eq!(rows[0]["content"][0]["type"], "tableHeader");
    assert_eq!(rows[1]["content"][1]["type"], "tableCell");
    assert_eq!(
        rows[1]["content"][1]["content"],
        json!([{"type": "paragraph", "content": [{"type": "text", "text": "b"}]}])
    );
    assert_eq!(render(&document), markdown);
}

#[test]
fn table_alignment_is_dropped_on_assembly() {
    let document = assemble("| a | b |\n| :-: | --: |\n| c | d |\n");

    assert!(
        !adf_text(&document).contains("align"),
        "no alignment attribute: {document}"
    );
    assert_eq!(render(&document), "| a | b |\n| --- | --- |\n| c | d |");
}

#[test]
fn a_thematic_break_assembles_to_a_rule() {
    assert_eq!(assemble("---\n")["content"], json!([{"type": "rule"}]));
}

#[test]
fn a_nested_bullet_list_round_trips() {
    let markdown = "- one\n  - two\n    - three\n- four";

    assert_eq!(round_trip(markdown), markdown);
    assert_eq!(
        assemble(markdown)["content"][0]["content"][0]["content"][1]["type"],
        "bulletList"
    );
}

#[test]
fn a_list_item_with_two_paragraphs_round_trips() {
    let markdown = "1. first\n\n   second\n2. next";
    let document = assemble(markdown);

    assert_eq!(
        document["content"][0]["content"][0]["content"],
        json!([
            {"type": "paragraph", "content": [{"type": "text", "text": "first"}]},
            {"type": "paragraph", "content": [{"type": "text", "text": "second"}]}
        ])
    );
    assert_eq!(render(&document), markdown);
}

#[test]
fn strike_round_trips() {
    let document = assemble("~~gone~~\n");

    assert_eq!(
        document["content"][0]["content"],
        json!([{"type": "text", "text": "gone", "marks": [{"type": "strike"}]}])
    );
    assert_eq!(render(&document), "~~gone~~");
}

#[test]
fn underline_is_still_dropped() {
    let rendered = render(&paragraph(&json!([{
        "type": "text", "text": "under", "marks": [{"type": "underline"}]
    }])));

    assert_eq!(rendered, "under");
}

#[test]
fn an_unknown_node_still_renders_a_placeholder() {
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "panel", "content": []}]
    }));

    assert_eq!(rendered, "[unsupported ADF node: panel]");
}

#[test]
fn a_strict_render_refuses_a_panel() {
    let error = strict(&json!({
        "type": "doc",
        "content": [{"type": "panel", "content": []}]
    }))
    .expect_err("a panel has no Markdown form");

    assert_eq!(
        error,
        AdfError::Unrepresentable {
            node: "panel".to_owned()
        }
    );
}

#[test]
fn a_strict_render_refuses_a_mention() {
    let error = strict(&paragraph(&json!([
        {"type": "text", "text": "hi "},
        {"type": "mention", "attrs": {"id": "1"}}
    ])))
    .expect_err("a mention has no Markdown form");

    assert_eq!(
        error,
        AdfError::Unrepresentable {
            node: "mention".to_owned()
        }
    );
}

#[test]
fn a_strict_render_drops_underline() {
    let rendered = strict(&paragraph(&json!([{
        "type": "text", "text": "under", "marks": [{"type": "underline"}]
    }])))
    .expect("underline is dropped, not refused");

    assert_eq!(rendered, "under");
}

fn adf_text(document: &Value) -> String {
    serde_json::to_string(document).expect("a Value serialises")
}

//! The ADF conversion's stated behaviours, including every deliberate lossy
//! one. A test pinning a quirk names `tests/fixtures/adf-fidelity-quirks.txt`,
//! which carries the reason it is preserved.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;

use jira_client::adf::tokenise::{tokenise, Token};
use jira_client::adf::AdfError;
use jira_client::{document_to_markdown, markdown_to_document};
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

#[test]
fn both_placeholder_strings_are_verbatim_and_position_dependent() {
    let block = render(&json!({
        "type": "doc",
        "content": [{"type": "blockquote"}]
    }));
    let inline = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [{"type": "emoji"}]}]
    }));

    assert_eq!(block, "[unsupported ADF node: blockquote]");
    assert_eq!(inline, "[unsupported ADF inline: emoji]");
}

#[test]
fn each_of_the_four_refusals_carries_its_code_and_message() {
    for (markdown, code, message) in [
        (
            "> quoted\n",
            41,
            "E_ADF_UNSUPPORTED_BLOCKQUOTE: blockquote is not supported",
        ),
        (
            "| a | b |\n",
            41,
            "E_ADF_UNSUPPORTED_TABLE: pipe tables are not supported",
        ),
        (
            "  - nested\n",
            41,
            "E_ADF_UNSUPPORTED_NESTED_LIST: nested lists are not supported",
        ),
        (
            "bad\u{1f}byte\n",
            42,
            "E_ADF_BAD_INPUT: input contains control byte \\x1e or \\x1f",
        ),
    ] {
        let error = refusal(markdown);
        assert_eq!(error.code(), code, "{markdown:?}");
        assert_eq!(error.to_string(), message, "{markdown:?}");
    }
}

#[test]
fn the_round_trip_is_asymmetric_rather_than_lossless() {
    let inventory = fixture("tests/fixtures/adf-node-types.txt");
    let rejected: Vec<&str> = inventory
        .lines()
        .filter(|line| line.starts_with("node "))
        .filter(|line| line.contains("hard-reject"))
        .collect();

    assert_eq!(
        rejected.len(),
        3,
        "assemble hard-rejects exactly three constructs: {rejected:?}"
    );
    for node in ["blockquote", "table"] {
        let rendered = render(&json!({
            "type": "doc",
            "content": [{"type": node}]
        }));
        assert!(
            rendered.starts_with("[unsupported ADF node:"),
            "render accepts {node} and degrades: {rendered}"
        );
        assert!(
            markdown_to_document(
                if node == "table" { "| a |\n" } else { "> q\n" },
                SEED
            )
            .is_err(),
            "assemble refuses {node}"
        );
    }
}

#[test]
fn an_ordered_list_always_assembles_with_order_one() {
    let _ = quirk("ordered-list-order-always-one");
    let document = assemble("3. three\n");
    let list = &document["content"][0];

    assert_eq!(list["type"], "orderedList");
    assert_eq!(list["attrs"]["order"], 1);
    assert_eq!(
        render(&document),
        "1. three",
        "and it renders back with the number it did not keep"
    );
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
fn marks_outside_the_pipeline_are_dropped_without_a_placeholder() {
    let _ = quirk("marks-ignored-without-a-placeholder");
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [{
            "type": "text", "text": "struck",
            "marks": [{"type": "strike"}, {"type": "underline"}]
        }]}]
    }));

    assert_eq!(rendered, "struck");
}

#[test]
fn a_list_item_renders_its_first_child_only() {
    let _ = quirk("listitem-second-child-dropped");
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "bulletList", "content": [{
            "type": "listItem",
            "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": "kept"}]},
                {"type": "paragraph", "content": [{"type": "text", "text": "dropped"}]}
            ]
        }]}]
    }));

    assert_eq!(rendered, "- kept");
}

#[test]
fn a_nested_list_as_a_first_child_renders_inline_placeholders() {
    let _ = quirk("nested-list-as-first-child-renders-inline-placeholders");
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "bulletList", "content": [{
            "type": "listItem",
            "content": [{"type": "bulletList", "content": [{"type": "listItem"}]}]
        }]}]
    }));

    assert_eq!(rendered, "- [unsupported ADF inline: listItem]");
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
fn the_nested_list_guard_rejects_an_indented_continuation_line() {
    let _ = quirk("nested-list-guard-has-no-space-requirement");
    let error = refusal("paragraph\n  -word continuation\n");

    assert_eq!(error, AdfError::UnsupportedNestedList);
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
fn text_is_never_escaped_on_the_way_out() {
    let _ = quirk("no-text-escaping");
    let rendered = render(&json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [
            {"type": "text", "text": "*not em* `not code` [not a link] # | _x_"}
        ]}]
    }));

    assert_eq!(rendered, "*not em* `not code` [not a link] # | _x_");
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
fn the_tokeniser_emits_the_notice_without_failing() {
    let tokenised = tokenise("__not bold__\n").expect("a notice is not fatal");

    assert_eq!(
        tokenised.notices,
        vec![jira_client::adf::tokenise::UNDERSCORE_NOTICE.to_owned()]
    );
    assert_eq!(
        tokenised.tokens,
        vec![Token::Paragraph("__not bold__".to_owned())]
    );
}

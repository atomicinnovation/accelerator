//! The converter's extension beyond the frozen oracle: blockquotes, pipe
//! tables, rules, nested lists, multi-paragraph list items, strike, escaping
//! and ordered-list starts.
//!
//! Sync compares a pulled Jira body with its own source through the round
//! trip, so every body the strict renderer produces must be a fixed point of
//! it.

#![allow(clippy::expect_used, clippy::panic)]

#[path = "support/adf_oracle.rs"]
mod adf_oracle;

use std::path::Path;
use std::path::PathBuf;

use adf::{
    document_to_markdown, document_to_markdown_strict, markdown_to_document,
    AdfError, Unrepresentable,
};
use serde_json::json;
use serde_json::Value;

const EXPECTED_CASE_COUNT: usize = 35;
const SEED: Option<&str> = Some("1");

fn extension_corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adf-extension")
}

fn extension_cases(prefix: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(extension_corpus())
        .expect("the extension corpus is readable")
        .filter_map(|entry| {
            let path = entry.expect("an extension entry").path();
            let name = path.file_name()?.to_string_lossy().into_owned();
            (path.is_dir() && name.starts_with(prefix)).then_some(path)
        })
        .collect();
    found.sort();
    found
}

fn read(case: &Path, name: &str) -> String {
    std::fs::read_to_string(case.join(name)).unwrap_or_else(|_| {
        panic!("{}: {name} is committed", adf_oracle::case_name(case))
    })
}

fn document_in(case: &Path, name: &str) -> Value {
    serde_json::from_str(&read(case, name)).expect("the fixture is JSON")
}

fn strict(document: &Value) -> String {
    document_to_markdown_strict(document).expect("the document renders")
}

fn assemble(markdown: &str) -> Value {
    markdown_to_document(markdown, SEED).expect("the markdown assembles")
}

fn round_trip(markdown: &str) -> String {
    strict(&assemble(markdown))
}

fn paragraph(inlines: &Value) -> Value {
    json!({"type": "doc", "content": [{"type": "paragraph", "content": inlines}]})
}

#[test]
fn the_extension_case_count_is_pinned() {
    let cases =
        extension_cases("render-").len() + extension_cases("assemble-").len();

    assert_eq!(
        cases, EXPECTED_CASE_COUNT,
        "a case was added or removed without updating this pin"
    );
}

#[test]
fn every_extension_case_renders_and_assembles_to_its_expected_output() {
    for case in extension_cases("render-") {
        let name = adf_oracle::case_name(&case);
        let document = document_in(&case, "adf.json");
        let expected = read(&case, "expected.md");

        assert_eq!(
            format!("{}\n", strict(&document)),
            expected,
            "{name}: strict"
        );
        assert_eq!(
            format!(
                "{}\n",
                document_to_markdown(&document).expect("the document renders")
            ),
            expected,
            "{name}: lenient"
        );
    }
    for case in extension_cases("assemble-") {
        let name = adf_oracle::case_name(&case);
        let assembled = assemble(&read(&case, "input.md"));

        assert_eq!(
            adf_oracle::canonical(&assembled),
            adf_oracle::canonical(&document_in(&case, "expected.adf.json")),
            "{name}"
        );
    }
}

#[test]
fn a_rendered_body_is_a_fixed_point_of_the_round_trip() {
    let mut refused = Vec::new();
    let mut compared = 0;

    for case in adf_oracle::cases("render-")
        .into_iter()
        .chain(extension_cases("render-"))
    {
        let name = adf_oracle::case_name(&case);
        let document = document_in(&case, "adf.json");
        match document_to_markdown_strict(&document) {
            Ok(rendered) => {
                assert_eq!(round_trip(&rendered), rendered, "{name}");
                compared += 1;
            }
            Err(error) => refused.push((name, error)),
        }
    }

    let unrepresentable = |node: &str| {
        AdfError::Unrepresentable(Unrepresentable::Node {
            node: node.to_owned(),
        })
    };
    let list = |node: &str| AdfError::ListWithoutContent {
        node: node.to_owned(),
    };
    assert_eq!(
        refused,
        vec![
            (
                "render-abort-bullet-list-without-content".to_owned(),
                list("bulletList")
            ),
            (
                "render-abort-heading-without-level".to_owned(),
                AdfError::HeadingWithoutLevel
            ),
            (
                "render-abort-ordered-list-without-content".to_owned(),
                list("orderedList")
            ),
            (
                "render-abort-root-not-doc".to_owned(),
                AdfError::RootNotDoc {
                    found: "paragraph".to_owned()
                }
            ),
            (
                "render-abort-task-list-without-content".to_owned(),
                list("taskList")
            ),
            (
                "render-block-placeholders".to_owned(),
                unrepresentable("panel")
            ),
            (
                "render-headings".to_owned(),
                AdfError::Unrepresentable(
                    Unrepresentable::HeadingBeyondLevelSix
                )
            ),
            (
                "render-inline-placeholders".to_owned(),
                unrepresentable("emoji")
            ),
            (
                "render-link-scheme-rejected".to_owned(),
                AdfError::Unrepresentable(
                    Unrepresentable::LinkToCodeRunningAddress
                )
            ),
        ]
    );
    assert!(compared > 0, "no rendered body was round-tripped");
}

#[test]
fn adjacent_marked_runs_render_unambiguously() {
    let case = extension_corpus().join("render-adjacent-marks");
    let document = document_in(&case, "adf.json");
    let rendered = strict(&document);

    let runs = |document: &Value| -> Vec<(String, Vec<String>)> {
        document["content"][0]["content"]
            .as_array()
            .expect("the paragraph has inlines")
            .iter()
            .map(|node| {
                let mut marks: Vec<String> = node["marks"]
                    .as_array()
                    .map(|marks| {
                        marks
                            .iter()
                            .map(|mark| mark["type"].to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                marks.sort();
                (node["text"].as_str().unwrap_or("").to_owned(), marks)
            })
            .collect()
    };
    let reassembled = runs(&assemble(&rendered));

    assert_eq!(
        reassembled
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>(),
        "abcdefg"
    );
    assert_eq!(
        reassembled,
        vec![
            ("ab".to_owned(), vec!["\"strong\"".to_owned()]),
            (
                "c".to_owned(),
                vec!["\"em\"".to_owned(), "\"strong\"".to_owned()]
            ),
            ("d".to_owned(), vec!["\"em\"".to_owned()]),
            ("e".to_owned(), vec![]),
            ("fg".to_owned(), vec!["\"code\"".to_owned()]),
        ],
        "each run keeps exactly its own marks"
    );
}

#[test]
fn a_text_node_without_text_renders_nothing() {
    let rendered = strict(&paragraph(&json!([
        {"type": "text", "text": "a"},
        {"type": "text", "marks": [{"type": "strong"}]},
        {"type": "text", "text": "", "marks": [{"type": "code"}]},
        {"type": "text", "text": "b"}
    ])));

    assert_eq!(rendered, "ab");
}

#[test]
fn code_text_is_never_escaped() {
    let case = extension_corpus().join("render-text-like-code-span");
    let rendered = strict(&document_in(&case, "adf.json"));

    assert!(rendered.contains("`a*b_c[d`"), "{rendered}");
    assert!(rendered.contains("\n*x* _y_ [z]\n"), "{rendered}");
    assert!(!rendered.contains('\\'), "{rendered}");
}

#[test]
fn strong_around_a_code_span_assembles_both_marks() {
    let mark_types = |node: &Value| -> Vec<String> {
        node["marks"]
            .as_array()
            .expect("the node is marked")
            .iter()
            .map(|mark| mark["type"].as_str().unwrap_or("").to_owned())
            .collect()
    };

    let strong = assemble("**`all`**\n");
    let linked = assemble("[**`all`**](https://x)\n");

    let node = &strong["content"][0]["content"][0];
    assert_eq!(node["text"], "all");
    assert_eq!(mark_types(node), ["code", "strong"]);

    let node = &linked["content"][0]["content"][0];
    assert_eq!(node["text"], "all");
    assert_eq!(mark_types(node), ["code", "strong", "link"]);
    assert_eq!(
        linked["content"][0]["content"]
            .as_array()
            .expect("inlines")
            .len(),
        1,
        "one text node carries every mark"
    );
}

#[test]
fn text_that_resembles_markdown_syntax_is_escaped_and_survives_the_round_trip()
{
    for text in [
        "> quote",
        "| a | b |",
        "---",
        "- bullet",
        "* bullet",
        "+ bullet",
        "12. item",
        "## heading",
        "```sh",
        "*em* **strong** _under_ `code` [a](b) ~~struck~~",
        "back\\slash \\* end\\",
    ] {
        let document = paragraph(&json!([{"type": "text", "text": text}]));
        let rendered = strict(&document);

        assert_ne!(rendered, text, "{text:?} is escaped");
        assert_eq!(
            assemble(&rendered)["content"][0]["content"],
            json!([{"type": "text", "text": text}]),
            "{text:?} survives as one literal text node via {rendered:?}"
        );
    }
}

#[test]
fn text_that_resembles_no_syntax_is_left_alone() {
    for text in [
        "plain words",
        "a > b | c # d",
        "-word",
        "--- not a rule",
        "#hashtag",
        "1.5 metres",
        "C:\\dir",
        "",
    ] {
        let document = paragraph(&json!([{"type": "text", "text": text}]));

        assert_eq!(strict(&document), text);
    }
}

#[test]
fn ordered_list_start_survives_the_round_trip() {
    let document = assemble("7. seven\n8. eight\n");

    assert_eq!(document["content"][0]["attrs"]["order"], 7);
    assert_eq!(strict(&document), "7. seven\n8. eight");
}

fn doc(blocks: &Value) -> Value {
    json!({"type": "doc", "content": blocks})
}

#[test]
fn a_hard_break_in_a_table_cell_has_no_markdown_form() {
    let document = doc(&json!([{"type": "table", "content": [
        {"type": "tableRow", "content": [
            {"type": "tableHeader", "content": [{"type": "paragraph",
                "content": [
                    {"type": "text", "text": "a"},
                    {"type": "hardBreak"},
                    {"type": "text", "text": "b"}
                ]}]}
        ]}
    ]}]));

    assert_eq!(
        document_to_markdown_strict(&document),
        Err(AdfError::Unrepresentable(
            Unrepresentable::HardBreakInTableCell
        ))
    );
    let lenient =
        document_to_markdown(&document).expect("the document renders");
    assert_eq!(lenient.lines().count(), 2, "{lenient}");
}

fn broken_line() -> Value {
    json!([
        {"type": "text", "text": "a"},
        {"type": "hardBreak"},
        {"type": "text", "text": "b"}
    ])
}

fn heading_with_a_broken_line() -> Value {
    doc(&json!([{"type": "heading", "attrs": {"level": 2},
        "content": broken_line()}]))
}

fn task_item_with_a_broken_line() -> Value {
    doc(
        &json!([{"type": "taskList", "attrs": {"localId": "1"}, "content": [
            {"type": "taskItem", "attrs": {"localId": "2", "state": "TODO"},
                "content": broken_line()}
        ]}]),
    )
}

#[test]
fn a_hard_break_in_a_heading_or_task_item_has_no_markdown_form() {
    assert_eq!(
        document_to_markdown_strict(&heading_with_a_broken_line()),
        Err(AdfError::Unrepresentable(
            Unrepresentable::HardBreakInHeading
        ))
    );
    assert_eq!(
        document_to_markdown_strict(&task_item_with_a_broken_line()),
        Err(AdfError::Unrepresentable(
            Unrepresentable::HardBreakInTaskItem
        ))
    );
}

#[test]
fn a_hard_break_in_a_heading_or_task_item_still_renders_leniently() {
    let lenient = |document: &Value| {
        document_to_markdown(document).expect("the document renders")
    };

    assert_eq!(lenient(&heading_with_a_broken_line()), "## a  \nb");
    assert_eq!(lenient(&task_item_with_a_broken_line()), "- [ ] a  \nb");
}

#[test]
fn code_text_holding_a_backtick_has_no_markdown_form() {
    let document = paragraph(&json!([
        {"type": "text", "text": "a`b", "marks": [{"type": "code"}]}
    ]));

    assert_eq!(
        document_to_markdown_strict(&document),
        Err(AdfError::Unrepresentable(
            Unrepresentable::BacktickInInlineCode
        ))
    );
}

fn table_cell_holding(inline: &Value) -> Value {
    doc(&json!([{"type": "table", "content": [
        {"type": "tableRow", "content": [
            {"type": "tableCell", "content": [{"type": "paragraph",
                "content": [inline]}]}
        ]}
    ]}]))
}

#[test]
fn code_text_holding_a_pipe_in_a_table_cell_has_no_markdown_form() {
    let document = table_cell_holding(
        &json!({"type": "text", "text": "a|b", "marks": [{"type": "code"}]}),
    );

    assert_eq!(
        document_to_markdown_strict(&document),
        Err(AdfError::Unrepresentable(
            Unrepresentable::PipeInTableCellCode
        ))
    );
}

#[test]
fn a_link_whose_href_holds_a_pipe_in_a_table_cell_has_no_markdown_form() {
    let document = table_cell_holding(&json!({"type": "text", "text": "x",
        "marks": [{"type": "link", "attrs": {"href": "https://e.com/a|b"}}]}));

    assert_eq!(
        document_to_markdown_strict(&document),
        Err(AdfError::Unrepresentable(
            Unrepresentable::PipeInTableCellLink
        ))
    );
}

#[test]
fn code_text_holding_a_pipe_outside_a_table_is_a_fixed_point() {
    let rendered = strict(&paragraph(&json!([
        {"type": "text", "text": "a|b", "marks": [{"type": "code"}]}
    ])));

    assert_eq!(rendered, "`a|b`");
    assert_eq!(round_trip(&rendered), rendered);
}

#[test]
fn a_table_cell_of_several_blocks_has_no_markdown_form() {
    let document = doc(&json!([{"type": "table", "content": [
        {"type": "tableRow", "content": [
            {"type": "tableCell", "content": [
                {"type": "paragraph",
                    "content": [{"type": "text", "text": "a"}]},
                {"type": "paragraph",
                    "content": [{"type": "text", "text": "b"}]}
            ]}
        ]}
    ]}]));

    assert_eq!(
        document_to_markdown_strict(&document),
        Err(AdfError::Unrepresentable(
            Unrepresentable::TableCellOfSeveralBlocks
        ))
    );
}

#[test]
fn a_link_to_a_foreign_scheme_is_a_fixed_point() {
    let rendered = strict(&paragraph(&json!([{"type": "text", "text": "chat",
        "marks": [{"type": "link",
            "attrs": {"href": "slack://channel?team=T1&id=C1"}}]}])));

    assert_eq!(rendered, "[chat](slack://channel?team=T1&id=C1)");
    assert_eq!(round_trip(&rendered), rendered);
}

#[test]
fn a_link_to_an_address_that_runs_code_has_no_markdown_form() {
    for href in ["javascript:alert(1)", "data:text/html,x", "vbscript:x"] {
        let document = paragraph(&json!([{"type": "text", "text": "x",
            "marks": [{"type": "link", "attrs": {"href": href}}]}]));

        assert_eq!(
            document_to_markdown_strict(&document),
            Err(AdfError::Unrepresentable(
                Unrepresentable::LinkToCodeRunningAddress
            )),
            "{href}"
        );
    }
}

#[test]
fn a_block_jira_forbids_inside_its_container_is_refused_at_assembly() {
    let nested = |child: &str, container: &str| AdfError::DisallowedNesting {
        child: child.to_owned(),
        container: container.to_owned(),
    };
    for (markdown, refusal) in [
        ("> ## Note\n> body", nested("heading", "blockquote")),
        (
            "> | a | b |\n> | --- | --- |\n> | 1 | 2 |",
            nested("table", "blockquote"),
        ),
        ("> > nested", nested("blockquote", "blockquote")),
        ("> ---", nested("rule", "blockquote")),
        ("> - [ ] task", nested("taskList", "blockquote")),
        ("- item\n\n  ## Heading", nested("heading", "listItem")),
        ("- item\n\n  > quote", nested("blockquote", "listItem")),
        (
            "- item\n\n  | a |\n  | --- |\n  | 1 |",
            nested("table", "listItem"),
        ),
    ] {
        assert_eq!(
            markdown_to_document(markdown, SEED),
            Err(refusal),
            "{markdown:?}"
        );
    }
}

#[test]
fn blocks_jira_allows_inside_a_quote_or_a_list_item_still_assemble() {
    for markdown in [
        "> a quote\n>\n> - with a list\n>\n> ```\n> code\n> ```",
        "- item\n\n  more\n\n  - nested\n\n  ```\n  code\n  ```",
    ] {
        assert!(markdown_to_document(markdown, SEED).is_ok(), "{markdown:?}");
    }
}

#[test]
fn a_disallowed_nesting_names_the_block_and_its_container() {
    assert_eq!(
        AdfError::DisallowedNesting {
            child: "heading".to_owned(),
            container: "blockquote".to_owned(),
        }
        .to_string(),
        "E_ADF_UNSUPPORTED: Jira does not allow a heading inside a blockquote"
    );
}

#[test]
fn an_unrepresentable_refusal_names_the_condition_to_change() {
    let message =
        |reason: Unrepresentable| AdfError::Unrepresentable(reason).to_string();

    assert_eq!(
        [
            message(Unrepresentable::Node {
                node: "panel".to_owned()
            }),
            message(Unrepresentable::HeadingBeyondLevelSix),
            message(Unrepresentable::FenceLineInCodeBlock),
            message(Unrepresentable::BacktickInInlineCode),
            message(Unrepresentable::PipeInTableCellCode),
            message(Unrepresentable::PipeInTableCellLink),
            message(Unrepresentable::HardBreakInTableCell),
            message(Unrepresentable::TableCellOfSeveralBlocks),
            message(Unrepresentable::LinkToCodeRunningAddress),
            message(Unrepresentable::HardBreakInHeading),
            message(Unrepresentable::HardBreakInTaskItem),
        ],
        [
            "E_ADF_UNREPRESENTABLE: a panel has no Markdown form",
            "E_ADF_UNREPRESENTABLE: a heading beyond level six has no \
             Markdown form",
            "E_ADF_UNREPRESENTABLE: a code block holding a line that starts \
             with three backticks has no Markdown form",
            "E_ADF_UNREPRESENTABLE: inline code holding a backtick has no \
             Markdown form",
            "E_ADF_UNREPRESENTABLE: inline code holding a pipe inside a table \
             cell has no Markdown form",
            "E_ADF_UNREPRESENTABLE: a link whose address holds a pipe inside \
             a table cell has no Markdown form",
            "E_ADF_UNREPRESENTABLE: a line break inside a table cell has no \
             Markdown form",
            "E_ADF_UNREPRESENTABLE: a table cell holding more than one \
             paragraph has no Markdown form",
            "E_ADF_UNREPRESENTABLE: a link to a javascript:, vbscript: or \
             data: address has no Markdown form",
            "E_ADF_UNREPRESENTABLE: a line break inside a heading has no \
             Markdown form",
            "E_ADF_UNREPRESENTABLE: a line break inside a task item has no \
             Markdown form",
        ]
    );
}

#[test]
fn a_code_block_holding_a_fence_line_has_no_markdown_form() {
    let document = doc(&json!([{"type": "codeBlock", "content": [
        {"type": "text", "text": "before\n```\nafter"}
    ]}]));

    assert_eq!(
        document_to_markdown_strict(&document),
        Err(AdfError::Unrepresentable(
            Unrepresentable::FenceLineInCodeBlock
        ))
    );
}

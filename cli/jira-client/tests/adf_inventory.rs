//! Cross-checks the committed node/mark inventory against what the conversion
//! code actually handles, in both directions: a type the renderer handles but
//! the fixture omits fails, and so does a fixture row claiming a type is
//! handled when no arm exists.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::Path;

use jira_client::adf::AdfError;

fn source(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .expect("the source is readable")
}

fn inventory() -> Vec<Vec<String>> {
    source("tests/fixtures/adf-node-types.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            line.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn rows(kind: &str) -> Vec<Vec<String>> {
    inventory()
        .into_iter()
        .filter(|row| row[0] == kind)
        .collect()
}

/// The `"type" =>` match arms in the renderer, which is what "handled" means
/// for a node the renderer dispatches on directly.
fn render_arms() -> BTreeSet<String> {
    let render = source("src/adf/render.rs");
    render
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix('"')?;
            let (name, tail) = rest.split_once('"')?;
            tail.trim_start().starts_with("=>").then(|| name.to_owned())
        })
        .collect()
}

fn render_marks() -> BTreeSet<String> {
    let render = source("src/adf/render.rs");
    render
        .split("has(\"")
        .skip(1)
        .filter_map(|tail| tail.split('"').next().map(str::to_owned))
        .collect()
}

/// Nodes the renderer handles structurally rather than through an arm: `doc`
/// is the root check, and a `listItem` / `taskItem` is consumed by its list's
/// own arm.
const HANDLED_WITHOUT_AN_ARM: &[&str] = &["doc", "listItem", "taskItem"];

#[test]
fn every_node_the_renderer_dispatches_on_has_an_inventory_row() {
    let listed: BTreeSet<String> = rows("node")
        .into_iter()
        .filter(|row| row[2] == "handled")
        .map(|row| row[1].clone())
        .collect();

    for arm in render_arms() {
        assert!(
            listed.contains(&arm),
            "the renderer handles {arm} but adf-node-types.txt does not list \
             it as handled"
        );
    }
}

#[test]
fn every_node_the_inventory_calls_handled_is_handled() {
    let arms = render_arms();
    for row in rows("node").into_iter().filter(|row| row[2] == "handled") {
        let node = &row[1];
        assert!(
            arms.contains(node)
                || HANDLED_WITHOUT_AN_ARM.contains(&node.as_str()),
            "adf-node-types.txt claims {node} is handled, but the renderer has \
             no arm for it and it is not handled structurally"
        );
    }
}

#[test]
fn the_mark_inventory_matches_the_pipeline() {
    let listed: BTreeSet<String> = rows("mark")
        .into_iter()
        .filter(|row| row[2] == "handled")
        .map(|row| row[1].clone())
        .collect();

    assert_eq!(
        listed,
        render_marks(),
        "the handled marks and the renderer's pipeline must agree exactly"
    );
}

#[test]
fn every_ignored_mark_really_renders_bare() {
    let ignored: Vec<String> = rows("mark")
        .into_iter()
        .filter(|row| row[2] == "ignored")
        .map(|row| row[1].clone())
        .collect();
    assert!(!ignored.is_empty(), "the inventory lists ignored marks");

    for mark in ignored {
        let document = serde_json::json!({
            "type": "doc",
            "content": [{"type": "paragraph", "content": [{
                "type": "text", "text": "x", "marks": [{"type": mark}]
            }]}]
        });
        assert_eq!(
            jira_client::document_to_markdown(&document)
                .expect("the document renders"),
            "x",
            "{mark} must render bare, with no placeholder"
        );
    }
}

#[test]
fn both_placeholder_strings_appear_verbatim_in_the_renderer() {
    let render = source("src/adf/render.rs");
    let placeholders = rows("placeholder");
    assert_eq!(placeholders.len(), 2, "block and inline");

    for row in placeholders {
        let template = row[2..row.len() - 1].join(" ");
        let prefix = template
            .split('<')
            .next()
            .expect("the template has a prefix")
            .to_owned();
        assert!(
            render.contains(prefix.trim_end()),
            "the renderer must emit {template:?} verbatim"
        );
    }
}

#[test]
fn every_committed_refusal_matches_the_typed_error() {
    let refusals = rows("reject");
    assert_eq!(refusals.len(), 4, "three exit-41 refusals and one exit-42");

    for row in refusals {
        let code: u16 = row[1].parse().expect("the exit code is numeric");
        let name = &row[2];
        let error = match name.as_str() {
            "E_ADF_UNSUPPORTED_BLOCKQUOTE" => AdfError::UnsupportedBlockquote,
            "E_ADF_UNSUPPORTED_TABLE" => AdfError::UnsupportedTable,
            "E_ADF_UNSUPPORTED_NESTED_LIST" => AdfError::UnsupportedNestedList,
            "E_ADF_BAD_INPUT" => AdfError::BadInput,
            other => panic!("unrecognised refusal {other}"),
        };
        assert_eq!(error.code(), code, "{name}");
        assert!(error.to_string().starts_with(name), "{error}");
    }
}

#[test]
fn the_inventory_records_three_abort_conditions() {
    let aborts = rows("abort");
    let names: BTreeSet<String> =
        aborts.iter().map(|row| row[1].clone()).collect();

    assert_eq!(
        names,
        BTreeSet::from([
            "heading-without-level".to_owned(),
            "root-not-doc".to_owned(),
            "list-without-content".to_owned(),
        ]),
        "0210's plan named two; the third was found by running the oracle"
    );
}

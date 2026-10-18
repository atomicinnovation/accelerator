//! The Jira body rules: an ADF description renders to local Markdown, and a
//! pushed body reads back as its ADF round trip.

#![allow(clippy::expect_used)]

mod support;

use http_test_support::MockHTTPServer;
use jira_client::JiraBodyDialect;
use support::client::{brief, client_with};
use support::RecordingSleeper;
use tracker::BodyDialect;

fn doc(content: &str) -> String {
    format!("{{\"content\":[{content}],\"type\":\"doc\",\"version\":1}}")
}

fn paragraph(text: &str) -> String {
    format!(
        "{{\"content\":[{{\"text\":\"{text}\",\"type\":\"text\"}}],\
         \"type\":\"paragraph\"}}"
    )
}

#[test]
fn a_jira_description_renders_to_local_markdown() {
    let description = doc(&format!(
        "{{\"attrs\":{{\"level\":1}},\"content\":[{{\"text\":\"Heading\",\
         \"type\":\"text\"}}],\"type\":\"heading\"}},{}",
        paragraph("Body text")
    ));

    assert_eq!(
        JiraBodyDialect.local_body(&format!("{description}\n")),
        Ok("# Heading\n\nBody text\n".to_owned())
    );
}

#[test]
fn a_null_jira_description_renders_to_an_empty_body() {
    assert_eq!(JiraBodyDialect.local_body("null\n"), Ok(String::new()));
}

#[test]
fn malformed_jira_adf_is_unrenderable() {
    assert!(JiraBodyDialect.local_body("{\"type\":\"doc\"").is_err());
}

#[test]
fn a_jira_description_with_an_unsupported_node_is_unrenderable() {
    let description = doc(&format!(
        "{{\"attrs\":{{\"panelType\":\"info\"}},\"content\":[{}],\
         \"type\":\"panel\"}}",
        paragraph("Inside a panel")
    ));

    let refusal = JiraBodyDialect
        .local_body(&description)
        .expect_err("a panel has no Markdown form");

    assert!(refusal.detail.contains("panel"), "{}", refusal.detail);
}

#[test]
fn a_jira_description_quoting_the_placeholder_text_renders() {
    let description = doc(&paragraph("[unsupported ADF node: panel]"));

    let body = JiraBodyDialect
        .local_body(&description)
        .expect("text that quotes a placeholder is ordinary text");

    assert!(body.contains("unsupported ADF node: panel"), "{body}");
}

#[test]
fn a_jira_body_as_pushed_is_its_round_trip() {
    assert_eq!(
        JiraBodyDialect.as_pushed("* first\n* second\n\n\n"),
        "- first\n- second\n"
    );
}

#[test]
fn a_jira_body_the_converter_rejects_is_as_pushed_verbatim() {
    let body = "a control byte \u{1f} the converter refuses\n";

    assert_eq!(JiraBodyDialect.as_pushed(body), body);
}

#[test]
fn a_jira_body_the_converter_rejects_is_rejected() {
    assert!(JiraBodyDialect
        .rejects("a control byte \u{1f} the converter refuses\n"));
    assert!(!JiraBodyDialect.rejects("# Heading\n\n| a |\n|---|\n| 1 |\n"));
}

#[test]
fn the_client_answers_rejection_through_its_dialect() {
    let server = MockHTTPServer::start();
    let client =
        client_with(&server.base_url(), brief(), &RecordingSleeper::new());

    assert!(client.rejects("a control byte \u{1e}\n"));
    assert!(!client.rejects("text\n"));
}

#[test]
fn the_client_answers_wire_documents_through_its_dialect() {
    let server = MockHTTPServer::start();
    let client =
        client_with(&server.base_url(), brief(), &RecordingSleeper::new());

    assert!(client.is_wire_document(&doc(&paragraph("text"))));
    assert!(!client.is_wire_document("# Heading\n"));
    assert_eq!(
        client.local_body(&doc(&paragraph("text"))),
        Ok("text\n".to_owned())
    );
}

#[test]
fn a_heading_inside_a_quote_is_a_body_jira_rejects() {
    assert!(JiraBodyDialect.rejects("> ## Note\n> body\n"));
}

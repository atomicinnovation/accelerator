//! Every body rule sync applies goes through one comparison over the
//! tracker's dialect.

use corpus_adapters::CommonMarkBody;
use jira_client::JiraBodyDialect;
use sha2::Digest as _;
use tracker::PlainDialect;
use work_adapters::sync::body_comparison::BodyComparison;

const JIRA_PANEL_DESCRIPTION: &str = "{\"content\":[{\"attrs\":{\"panelType\":\
    \"info\"},\"content\":[],\"type\":\"panel\"}],\"type\":\"doc\",\
    \"version\":1}\n";

const JIRA_PARAGRAPH: &str = "Title\n{\"content\":[{\"content\":[{\"text\":\
    \"Remote body\",\"type\":\"text\"}],\"type\":\"paragraph\"}],\"type\":\
    \"doc\"}\n";

fn linear() -> BodyComparison<'static> {
    BodyComparison::new(&PlainDialect, &CommonMarkBody)
}

fn jira() -> BodyComparison<'static> {
    BodyComparison::new(&JiraBodyDialect, &CommonMarkBody)
}

fn sha256_hex(text: &str) -> String {
    hex::encode(sha2::Sha256::digest(text.as_bytes()))
}

#[test]
fn a_linear_description_is_the_local_body_verbatim() {
    assert_eq!(
        linear().local_body("Title\n\n# Heading\n* item\n"),
        Ok("\n# Heading\n* item\n".to_owned())
    );
}

#[test]
fn a_jira_description_is_rendered_through_the_tracker() {
    assert_eq!(
        jira().local_body(JIRA_PARAGRAPH),
        Ok("Remote body\n".to_owned())
    );
}

#[test]
fn an_unrenderable_description_has_no_local_body() {
    assert!(jira()
        .local_body(&format!("Title\n{JIRA_PANEL_DESCRIPTION}"))
        .is_err());
}

#[test]
fn the_body_hash_ignores_cosmetic_rewrites() {
    let comparison = linear();

    assert_eq!(
        comparison.local_body_hash("# Heading\n\n* one\n* two\n"),
        comparison.remote_body_hash("Title\n# Heading\n\n- one\n- two\n")
    );
    assert_ne!(
        comparison.local_body_hash("# Heading\n\n* one\n"),
        comparison.remote_body_hash("Title\n# Heading\n\n1. one\n")
    );
}

#[test]
fn the_remote_hash_strips_the_projected_title_line() {
    let comparison = linear();

    assert_eq!(
        comparison.remote_body_hash("First title\nBody\n"),
        comparison.remote_body_hash("Second title\nBody\n")
    );
    assert_eq!(
        comparison.remote_body_hash("Title\nBody\n"),
        comparison.local_body_hash("Body\n")
    );
}

#[test]
fn a_jira_body_hash_digests_the_round_tripped_body() {
    let comparison = jira();

    assert_eq!(
        comparison.local_body_hash("Remote body\n"),
        comparison.remote_body_hash(JIRA_PARAGRAPH)
    );
    assert_eq!(
        comparison.local_body_hash("Remote body\n"),
        linear().local_body_hash("Remote body\n")
    );
}

#[test]
fn a_jira_body_the_converter_rejects_is_digested_without_the_round_trip() {
    let body = "a control byte \u{1f} the converter refuses\n";

    assert_eq!(jira().local_body_hash(body), linear().local_body_hash(body));
}

#[test]
fn an_unrenderable_jira_remote_hashes_its_canonical_json() {
    assert_eq!(
        jira().remote_body_hash(&format!("Title\n{JIRA_PANEL_DESCRIPTION}")),
        sha256_hex(&format!("unrenderable:{JIRA_PANEL_DESCRIPTION}"))
    );
}

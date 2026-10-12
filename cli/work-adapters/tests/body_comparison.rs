//! Every body rule sync applies goes through one comparison over the
//! tracker's dialect.

use jira_client::JiraBodyDialect;
use tracker::PlainDialect;
use work_adapters::sync::body_comparison::BodyComparison;

const JIRA_PANEL: &str = "Title\n{\"content\":[{\"attrs\":{\"panelType\":\
    \"info\"},\"content\":[],\"type\":\"panel\"}],\"type\":\"doc\",\
    \"version\":1}\n";

#[test]
fn a_linear_description_is_the_local_body_verbatim() {
    let comparison = BodyComparison::new(&PlainDialect);

    assert_eq!(
        comparison.local_body("Title\n\n# Heading\n* item\n"),
        Ok("\n# Heading\n* item\n".to_owned())
    );
}

#[test]
fn a_jira_description_is_rendered_through_the_tracker() {
    let comparison = BodyComparison::new(&JiraBodyDialect);

    assert_eq!(
        comparison.local_body(
            "Title\n{\"content\":[{\"content\":[{\"text\":\"Remote body\",\
             \"type\":\"text\"}],\"type\":\"paragraph\"}],\"type\":\"doc\"}\n"
        ),
        Ok("Remote body\n".to_owned())
    );
}

#[test]
fn an_unrenderable_description_has_no_local_body() {
    let comparison = BodyComparison::new(&JiraBodyDialect);

    assert!(comparison.local_body(JIRA_PANEL).is_err());
}

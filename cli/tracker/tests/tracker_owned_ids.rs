//! Which integrations' issue keys a work item may take as its `id`.

use tracker::supports_tracker_owned_ids;

#[test]
fn only_jira_and_linear_support_tracker_owned_ids() {
    assert!(supports_tracker_owned_ids("jira"));
    assert!(supports_tracker_owned_ids("linear"));
    assert!(!supports_tracker_owned_ids(""));
    assert!(!supports_tracker_owned_ids("trello"));
    assert!(!supports_tracker_owned_ids("github-issues"));
}

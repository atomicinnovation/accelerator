//! Trackers that answer body rules as a shipped client does.

use jira_client::JiraBodyDialect;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker_test_support::RecordingTracker;

/// A recording tracker holding `issues` under Jira's body rules.
pub fn jira_recording_tracker(
    issues: Vec<(ExternalId, RemoteIssue)>,
) -> RecordingTracker {
    RecordingTracker::holding(issues).with_dialect(Box::new(JiraBodyDialect))
}

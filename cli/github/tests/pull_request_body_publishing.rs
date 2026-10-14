use collaboration::PullRequestBodyPublishing as _;
use github::GitHubPullRequestBodyPublishing;

#[test]
fn preparing_for_publishing_unwraps_soft_line_breaks() {
    assert_eq!(
        GitHubPullRequestBodyPublishing
            .prepare_for_publishing("> first line\n> second line"),
        "> first line second line"
    );
}

//! The GitHub implementation of `collaboration`'s `PullRequestBodyPublishing`
//! port.

use collaboration::PullRequestBodyPublishing;

/// GitHub renders a single newline in a pull request body as a line break,
/// so a body is published with its soft line breaks unwrapped.
pub struct GitHubPullRequestBodyPublishing;

impl PullRequestBodyPublishing for GitHubPullRequestBodyPublishing {
    fn prepare_for_publishing(&self, body: &str) -> String {
        markdown::unwrap_soft_line_breaks(body)
    }
}

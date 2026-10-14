//! The GitHub forge implementation of `collaboration`'s ports: an
//! `octocrab`-backed REST client, a `github.com` remote-URL recognizer, and
//! pull request body publishing that unwraps soft line breaks.

mod octocrab_client;
mod pull_request_body_publishing;
mod remote_url_recognizer;

pub use octocrab_client::OctocrabClient;
pub use pull_request_body_publishing::GitHubPullRequestBodyPublishing;
pub use remote_url_recognizer::GitHubRemoteUrlRecognizer;

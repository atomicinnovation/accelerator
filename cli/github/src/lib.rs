//! The GitHub forge implementation of `collaboration`'s ports: an
//! `octocrab`-backed REST client and a `github.com` remote-URL recognizer.

mod octocrab_client;
mod remote_url_recognizer;

pub use octocrab_client::OctocrabClient;
pub use remote_url_recognizer::GitHubRemoteUrlRecognizer;

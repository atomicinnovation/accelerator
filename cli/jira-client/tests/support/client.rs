//! A `JiraClient` pointed at a mock server, with the injected clock the retry
//! suites need.

#![allow(dead_code, clippy::expect_used)]

use std::time::Duration;

use http_test_support::MockServer;
use jira_client::jql::FixedResolver;
use jira_client::transport::Transport;
use jira_client::{Credentials, JiraClient};
use reqwest::Url;
use tracker_support::{Secret, TokenSource, TransportConfig};

use super::{NoJitter, RecordingSleeper};

pub const PROJECT: &str = "ENG";

#[must_use]
pub fn credentials(base: &str) -> Credentials {
    Credentials {
        base: Url::parse(base).expect("a base URL"),
        email: "toby@example.com".to_owned(),
        token: Secret::new("secret-token".to_owned()),
        source: TokenSource::Env,
    }
}

#[must_use]
pub fn brief() -> TransportConfig {
    TransportConfig {
        timeout: Duration::from_millis(400),
        ..TransportConfig::default()
    }
}

/// A client against `server`, with the loopback base the constructor admits
/// and configuration refuses.
#[must_use]
pub fn client_for(server: &MockServer, config: TransportConfig) -> JiraClient {
    client_with(&server.base_url(), config, &RecordingSleeper::new())
}

#[must_use]
pub fn client_with(
    base: &str,
    config: TransportConfig,
    sleeper: &RecordingSleeper,
) -> JiraClient {
    let transport = Transport::new(
        credentials(base),
        config,
        Box::new(sleeper.clone()),
        Box::new(NoJitter),
    )
    .expect("the transport builds");
    JiraClient::new(
        transport,
        PROJECT.to_owned(),
        Box::new(FixedResolver::new()),
        Box::new(FixedResolver::new()),
    )
}

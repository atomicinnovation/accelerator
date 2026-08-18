//! A `LinearClient` pointed at a mock server.

#![allow(dead_code, clippy::expect_used)]

use std::time::Duration;

use http_test_support::MockServer;
use linear_client::filter::FixedStates;
use linear_client::transport::Transport;
use linear_client::{Credentials, LinearClient};
use reqwest::Url;
use tracker_support::{Secret, TokenSource, TransportConfig};

use super::{NoJitter, RecordingSleeper};

pub const TEAM_ID: &str = "5c9f2a1b-0000-4000-8000-000000000001";
pub const TEAM_KEY: &str = "ENG";

#[must_use]
pub fn credentials() -> Credentials {
    Credentials {
        token: Secret::new("lin_api_secret".to_owned()),
        team_id: TEAM_ID.to_owned(),
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

#[must_use]
pub fn client_for(
    server: &MockServer,
    config: TransportConfig,
) -> LinearClient {
    client_with(&server.base_url(), config, Some(TEAM_KEY.to_owned()))
}

/// `team_key` is `None` for the case where only `linear.team_id` is
/// configured, so nothing can be proved about an identifier's scope.
#[must_use]
pub fn client_with(
    base: &str,
    config: TransportConfig,
    team_key: Option<String>,
) -> LinearClient {
    let transport = Transport::new(
        Url::parse(&format!("{base}/graphql")).expect("an endpoint"),
        credentials(),
        config,
        Box::new(RecordingSleeper::new()),
        Box::new(NoJitter),
    )
    .expect("the transport builds");
    LinearClient::new(transport, team_key, Box::new(FixedStates::default()))
}

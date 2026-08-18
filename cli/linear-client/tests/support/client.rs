//! A `LinearClient` pointed at a mock server.

#![allow(dead_code, clippy::expect_used)]

use std::time::Duration;

use http_test_support::MockServer;
use linear_client::filter::{FixedStates, StateResolver};
use linear_client::transport::Transport;
use linear_client::{Credentials, LinearClient, UploadTransport};
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

/// The upload transport tests point at the mock: loopback admitted, and no wait
/// between the bounded PUT attempts so a failure path runs in milliseconds.
#[must_use]
pub fn loopback_upload() -> UploadTransport {
    UploadTransport::new(true, std::time::Duration::ZERO)
        .expect("the upload transport builds")
}

/// A client whose catalogue-backed states are replaced by a fixed resolver, for
/// the transition suites that assert name resolution.
#[must_use]
pub fn client_with_states(
    server: &MockServer,
    states: Box<dyn StateResolver>,
) -> LinearClient {
    let transport = Transport::new(
        Url::parse(&format!("{}/graphql", server.base_url()))
            .expect("an endpoint"),
        credentials(),
        TransportConfig::default(),
        Box::new(RecordingSleeper::new()),
        Box::new(NoJitter),
    )
    .expect("the transport builds");
    LinearClient::new(
        transport,
        loopback_upload(),
        Some(TEAM_KEY.to_owned()),
        states,
    )
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
    LinearClient::new(
        transport,
        loopback_upload(),
        team_key,
        Box::new(FixedStates::default()),
    )
}

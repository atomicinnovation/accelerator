//! A `LinearClient` pointed at a mock server.

#![allow(dead_code, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use http_test_support::MockServer;
use linear_client::catalogue::TeamEntries;
use linear_client::healing::{CatalogueBackfill, NoBackfill};
use linear_client::resolution::{FixedNames, ResolverSet};
use linear_client::transport::Transport;
use linear_client::{Credentials, LinearClient, UploadTransport};
use reqwest::Url;
use tracker_support::{Secret, TokenSource, TransportConfig};

use super::{NoJitter, RecordingSleeper};

pub const TEAM_ID: &str = "5c9f2a1b-0000-4000-8000-000000000001";
pub const TEAM_KEY: &str = "ENG";

/// Resolvers whose catalogue names only the base team, so a scope keyed by
/// `TEAM_KEY` resolves to `TEAM_ID` as the catalogue would in production.
#[must_use]
pub fn base_team_resolvers() -> ResolverSet {
    keyed_resolvers(&[(TEAM_KEY, TEAM_ID)])
}

#[must_use]
pub fn keyed_resolvers(teams: &[(&str, &str)]) -> ResolverSet {
    ResolverSet::new(Box::new(FixedNames::default()), TeamEntries::keyed(teams))
}

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

/// A client whose catalogue names several teams (`key` → UUID), for the
/// multi-team keyed reconcile read: the corpus spans additional teams, so
/// `fetch_all` pages each and `in_scope` accepts each team's prefix.
#[must_use]
pub fn client_with_teams(
    server: &MockServer,
    config: TransportConfig,
    teams: &[(&str, &str)],
) -> LinearClient {
    let transport = Transport::new(
        Url::parse(&format!("{}/graphql", server.base_url()))
            .expect("an endpoint"),
        credentials(),
        config,
        Box::new(RecordingSleeper::new()),
        Box::new(NoJitter),
    )
    .expect("the transport builds");
    LinearClient::new(
        transport,
        loopback_upload(),
        Some(TEAM_KEY.to_owned()),
        keyed_resolvers(teams),
        Arc::new(NoBackfill),
    )
}

/// A client over the given resolvers, for the suites that assert name
/// resolution.
#[must_use]
pub fn client_with_resolvers(
    server: &MockServer,
    resolvers: ResolverSet,
) -> LinearClient {
    client_holding_into(server, resolvers, Arc::new(NoBackfill))
}

/// A client over the given resolvers whose live fetches are handed to
/// `backfill`.
#[must_use]
pub fn client_holding_into(
    server: &MockServer,
    resolvers: ResolverSet,
    backfill: Arc<dyn CatalogueBackfill>,
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
        resolvers,
        backfill,
    )
}

/// `team_key` is `None` for the case where only `linear.team_id` is
/// configured — no team key and no catalogued team — so nothing can be proved
/// about an identifier's scope.
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
    // An unset team key models a client with no catalogue scope knowledge, so
    // the team resolver is empty too — otherwise a catalogued team would prove
    // scope the "nothing is provable" case is built to lack.
    let resolvers = if team_key.is_some() {
        base_team_resolvers()
    } else {
        keyed_resolvers(&[])
    };
    LinearClient::new(
        transport,
        loopback_upload(),
        team_key,
        resolvers,
        Arc::new(NoBackfill),
    )
}

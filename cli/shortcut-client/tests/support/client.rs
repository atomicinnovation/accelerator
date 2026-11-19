//! A `ShortcutClient` pointed at a mock v4 API, with a catalogue matching the
//! committed live fixtures.

use std::path::Path;
use std::time::Duration;

use config::credentials::Secret;
use config::credentials::TokenSource;
use http_test_support::MockHTTPServer;
use http_test_support::Route;
use serde_json::json;
use serde_json::Value;
use shortcut_client::catalogue::Catalogue;
use shortcut_client::transport::Transport;
use shortcut_client::Credentials;
use shortcut_client::ShortcutClient;
use shortcut_client::Url;
use tracker_support::TransportConfig;

use super::NoJitter;
use super::RecordingSleeper;

pub const TEAM: &str = "00000000-0000-4000-8000-000000000002";
pub const OTHER_TEAM: &str = "00000000-0000-4000-8000-000000000009";
pub const TEAM_KEY: &str = "team-1";
pub const WORKFLOW: i64 = 500_000_005;
pub const TO_DO: i64 = 500_000_007;
pub const API: &str = "/api/v4/scratch-workspace";

/// A committed live fixture, as the route that serves it.
pub fn fixture(name: &str) -> Route {
    let (status, body) = fixture_parts(name);
    Route::Json {
        status,
        body: body.to_string(),
    }
}

pub fn fixture_parts(name: &str) -> (u16, Value) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v4")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).expect("the fixture exists");
    let fixture: Value =
        serde_json::from_str(&text).expect("the fixture is JSON");
    let status = fixture["status"]
        .as_u64()
        .and_then(|status| u16::try_from(status).ok())
        .expect("a status");
    (status, fixture["body"].clone())
}

pub fn json_route(status: u16, body: &Value) -> Route {
    Route::Json {
        status,
        body: body.to_string(),
    }
}

pub fn catalogue_value() -> Value {
    json!({
        "schema_version": shortcut_client::catalogue::SCHEMA_VERSION,
        "workspace": {"slug": "scratch-workspace", "name": "Scratch Workspace"},
        "teams": [
            {"id": TEAM, "name": "Team 1", "mention_name": TEAM_KEY,
             "default_workflow_id": WORKFLOW, "workflow_ids": [WORKFLOW]},
            {"id": OTHER_TEAM, "name": "Web", "mention_name": "web",
             "default_workflow_id": 600, "workflow_ids": [600]}
        ],
        "workflows": [
            {"id": WORKFLOW, "name": "Standard", "states": [
                {"id": 500_000_006, "name": "Backlog", "type": "backlog", "position": 1},
                {"id": 500_000_004, "name": "Later", "type": "unstarted", "position": 9},
                {"id": TO_DO, "name": "To Do", "type": "unstarted", "position": 2},
                {"id": 500_000_009, "name": "Done", "type": "done", "position": 10}
            ]},
            {"id": 600, "name": "Design", "states": [
                {"id": 601, "name": "Brief", "type": "unstarted", "position": 1}
            ]}
        ]
    })
}

pub fn catalogue_from(value: &Value) -> Catalogue {
    Catalogue::parse(&value.to_string()).expect("a valid catalogue")
}

pub fn transport_with(config: TransportConfig) -> Transport {
    Transport::new(
        &Credentials {
            token: Secret::new("sct_rw_scratch_t".to_owned()),
            source: TokenSource::Env,
            refusals: Vec::new(),
            notice: None,
        },
        config,
        Box::new(RecordingSleeper::new()),
        Box::new(NoJitter),
    )
    .expect("the transport builds")
}

pub fn fast_config() -> TransportConfig {
    TransportConfig {
        timeout: Duration::from_secs(5),
        ..TransportConfig::default()
    }
}

pub fn client_over(
    server: &MockHTTPServer,
    catalogue: Catalogue,
    team_key: Option<&str>,
) -> ShortcutClient {
    ShortcutClient::new(
        transport_with(fast_config()),
        Url::parse(&server.base_url()).expect("parses"),
        catalogue,
        team_key.map(str::to_owned),
    )
}

pub fn client(server: &MockHTTPServer) -> ShortcutClient {
    client_over(server, catalogue_from(&catalogue_value()), Some(TEAM_KEY))
}

pub fn path(rest: &str) -> String {
    format!("{API}{rest}")
}

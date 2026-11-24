//! The real Jira, Linear and Shortcut clients over loopback mocks, so a push
//! test can assert the bytes each tracker is sent.

use std::time::Duration;

use config::credentials::Secret;
use config::credentials::TokenSource;
use graphql_test_support::MockGraphQLServer;
use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use serde_json::json;
use serde_json::Value;
use tracker::RemoteTracker;
use tracker_support::Jitter;
use tracker_support::Sleeper;
use tracker_support::TransportConfig;

pub const NEEDS_BOTH_ESCAPES: &str = "- owned by\n  0160.\n\nSee crates.io.\n";

const STAMP: &str = "2026-06-01T00:00:00.000Z";
const SHORTCUT_API: &str = "/api/v4/acme";
const SHORTCUT_TEAM: &str = "00000000-0000-4000-8000-000000000002";
const LINEAR_TEAM_KEY: &str = "ENG";
const LINEAR_TEAM_ID: &str = "5c9f2a1b-0000-4000-8000-000000000001";

#[derive(Debug, Clone, Copy)]
pub enum Tracker {
    Jira,
    Linear,
    Shortcut,
}

impl Tracker {
    pub const ALL: [Self; 3] = [Self::Jira, Self::Linear, Self::Shortcut];

    pub const fn integration(self) -> &'static str {
        match self {
            Self::Jira => "jira",
            Self::Linear => "linear",
            Self::Shortcut => "shortcut",
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Jira | Self::Linear => "ENG-1",
            Self::Shortcut => "sc-1",
        }
    }

    /// The description this tracker is sent for `body`: escaped for Linear
    /// alone, converted to ADF for Jira, and as given for Shortcut.
    pub fn expected_description(self, body: &str) -> Value {
        match self {
            Self::Jira => adf::markdown_to_document(body, None)
                .expect("the body converts to ADF"),
            Self::Linear => Value::from(
                body.replace("  0160.\n", "  0160\\.\n")
                    .replace("crates.io.", "crates\\.io."),
            ),
            Self::Shortcut => Value::from(body),
        }
    }

    /// A mock holding one issue titled `title` whose description is
    /// `stored`, answering every create, update and read of it.
    pub fn serve(self, title: &str, stored: &str) -> ServedTracker {
        let shown = self.shown(title, stored);
        match self {
            Self::Jira => serve_jira(shown),
            Self::Linear => serve_linear(shown),
            Self::Shortcut => serve_shortcut(shown, title),
        }
    }

    fn shown(self, title: &str, stored: &str) -> Route {
        let body = match self {
            Self::Jira => json!({"key": "ENG-1", "fields": {
                "updated": "2026-06-01T00:00:00.000+0000",
                "summary": title,
                "description": adf::markdown_to_document(stored, None)
                    .expect("the stored body converts to ADF"),
            }}),
            Self::Linear => json!({"data": {"issue": {
                "id": "i", "identifier": "ENG-1", "title": title,
                "updatedAt": STAMP, "description": stored,
            }}}),
            Self::Shortcut => json!({"entity": {
                "id": 1, "name": title, "description": stored,
                "updated_at": STAMP,
            }}),
        };
        reply(200, &body)
    }

    fn description_of(self, request: &Value) -> Value {
        match self {
            Self::Jira => request["fields"]["description"].clone(),
            Self::Linear => {
                request["variables"]["input"]["description"].clone()
            }
            Self::Shortcut => request["description"].clone(),
        }
    }
}

enum Server {
    Http(MockHTTPServer),
    GraphQl(MockGraphQLServer),
}

pub struct ServedTracker {
    tracker: Tracker,
    server: Server,
    client: Option<Box<dyn RemoteTracker>>,
}

impl ServedTracker {
    /// The client, handed out once, as a registry resolves it.
    pub fn client(&mut self) -> Box<dyn RemoteTracker> {
        self.client.take().expect("the client is taken once")
    }

    /// The description of every create, then of every update, in the order
    /// each was sent.
    pub fn sent_descriptions(&self) -> Vec<Value> {
        let (creates, updates) = match &self.server {
            Server::Http(server) => {
                let (create, update) = match self.tracker {
                    Tracker::Jira => (
                        RequestKey::post("/rest/api/3/issue"),
                        RequestKey::new("PUT", "/rest/api/3/issue/ENG-1"),
                    ),
                    Tracker::Linear | Tracker::Shortcut => (
                        RequestKey::post(&format!("{SHORTCUT_API}/stories")),
                        RequestKey::new(
                            "PATCH",
                            &format!("{SHORTCUT_API}/stories/1"),
                        ),
                    ),
                };
                (server.bodies(&create), server.bodies(&update))
            }
            Server::GraphQl(server) => {
                (server.bodies("issueCreate"), server.bodies("issueUpdate"))
            }
        };
        creates
            .iter()
            .chain(&updates)
            .map(|body| {
                let request: Value =
                    serde_json::from_slice(body).expect("a JSON request");
                self.tracker.description_of(&request)
            })
            .collect()
    }
}

fn reply(status: u16, body: &Value) -> Route {
    Route::Json {
        status,
        body: body.to_string(),
    }
}

struct NoSleep;

impl Sleeper for NoSleep {
    fn sleep(&mut self, _duration: Duration) {}
}

struct NoJitter;

impl Jitter for NoJitter {
    fn offset(&mut self, _spread: u64) -> i64 {
        0
    }
}

fn serve_jira(shown: Route) -> ServedTracker {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get("/rest/api/3/issue/ENG-1"), shown);
    server.route(
        RequestKey::post("/rest/api/3/issue"),
        reply(201, &json!({"id": "10000", "key": "ENG-1"})),
    );
    server.route(
        RequestKey::new("PUT", "/rest/api/3/issue/ENG-1"),
        Route::Status(204),
    );
    let transport = jira_client::transport::Transport::new(
        jira_client::Credentials {
            base: jira_client::transport::Url::parse(&server.base_url())
                .expect("a base URL"),
            email: "tester@example.com".to_owned(),
            token: Secret::new("secret".to_owned()),
            source: TokenSource::Env,
            refusals: Vec::new(),
            notices: Vec::new(),
        },
        TransportConfig::default(),
        Box::new(NoSleep),
        Box::new(NoJitter),
    )
    .expect("the jira transport builds");
    let client = jira_client::JiraClient::new(
        transport,
        "ENG".to_owned(),
        Box::new(jira_client::jql::FixedResolver::new()),
        Box::new(jira_client::jql::FixedResolver::new()),
    );
    ServedTracker {
        tracker: Tracker::Jira,
        server: Server::Http(server),
        client: Some(Box::new(client)),
    }
}

fn serve_linear(shown: Route) -> ServedTracker {
    let server = MockGraphQLServer::start();
    server.answer("issue", shown);
    let issue =
        json!({"success": true, "issue": {"id": "i", "identifier": "ENG-1"}});
    server.answer(
        "issueCreate",
        reply(200, &json!({"data": {"issueCreate": issue}})),
    );
    server.answer(
        "issueUpdate",
        reply(200, &json!({"data": {"issueUpdate": issue}})),
    );
    let transport = linear_client::transport::Transport::new(
        linear_client::transport::Url::parse(&format!(
            "{}/graphql",
            server.base_url()
        ))
        .expect("an endpoint"),
        linear_client::Credentials {
            token: Secret::new("lin_api_secret".to_owned()),
            team_id: LINEAR_TEAM_ID.to_owned(),
            source: TokenSource::Env,
            refusals: Vec::new(),
            notice: None,
        },
        TransportConfig::default(),
        Box::new(NoSleep),
        Box::new(NoJitter),
    )
    .expect("the linear transport builds");
    let client = linear_client::LinearClient::new(
        transport,
        linear_client::UploadTransport::production()
            .expect("the upload transport builds"),
        Some(LINEAR_TEAM_KEY.to_owned()),
        linear_client::resolution::ResolverSet::new(
            Box::new(linear_client::resolution::FixedNames::default()),
            linear_client::catalogue::TeamEntries::keyed(&[(
                LINEAR_TEAM_KEY,
                LINEAR_TEAM_ID,
            )]),
        ),
        std::sync::Arc::new(linear_client::healing::NoBackfill),
    );
    ServedTracker {
        tracker: Tracker::Linear,
        server: Server::GraphQl(server),
        client: Some(Box::new(client)),
    }
}

fn serve_shortcut(shown: Route, title: &str) -> ServedTracker {
    let server = MockHTTPServer::start();
    let entity = reply(
        200,
        &json!({"entity": {"id": 1, "name": title, "updated_at": STAMP}}),
    );
    server.route(RequestKey::get(&format!("{SHORTCUT_API}/stories/1")), shown);
    server.route(
        RequestKey::post(&format!("{SHORTCUT_API}/stories")),
        entity.clone(),
    );
    server.route(
        RequestKey::new("PATCH", &format!("{SHORTCUT_API}/stories/1")),
        entity,
    );
    let transport = shortcut_client::transport::Transport::new(
        &shortcut_client::Credentials {
            token: Secret::new("sct_rw_secret".to_owned()),
            source: TokenSource::Env,
            refusals: Vec::new(),
            notice: None,
        },
        TransportConfig::default(),
        Box::new(NoSleep),
        Box::new(NoJitter),
    )
    .expect("the shortcut transport builds");
    let catalogue = shortcut_client::catalogue::Catalogue::parse(
        &json!({
            "schema_version": 1,
            "workspace": {"slug": "acme", "name": "Acme"},
            "teams": [
                {"id": SHORTCUT_TEAM, "name": "Team 1", "mention_name": "team-1",
                 "default_workflow_id": 500, "workflow_ids": [500]}
            ],
            "workflows": [{"id": 500, "name": "Standard", "states": [
                {"id": 501, "name": "To Do", "type": "unstarted", "position": 1}
            ]}]
        })
        .to_string(),
    )
    .expect("a valid catalogue");
    let client = shortcut_client::ShortcutClient::new(
        transport,
        shortcut_client::Url::parse(&server.base_url()).expect("a base URL"),
        catalogue,
        Some("team-1".to_owned()),
    );
    ServedTracker {
        tracker: Tracker::Shortcut,
        server: Server::Http(server),
        client: Some(Box::new(client)),
    }
}

//! whoami and the discovery listings, against a mock v4 API. The response
//! shapes follow the v4 reference until live evidence pins them.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;

use config::credentials::Secret;
use config::credentials::TokenSource;
use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use serde_json::json;
use serde_json::Value;
use shortcut_client::cache::Listing;
use shortcut_client::discovery::discover;
use shortcut_client::discovery::DiscoveryError;
use shortcut_client::transport::Deadline;
use shortcut_client::transport::Transport;
use shortcut_client::workspace::whoami;
use shortcut_client::Credentials;
use shortcut_client::Url;
use support::NoJitter;
use support::RecordingSleeper;
use tracker_support::TransportConfig;

const MOBILE: &str = "8f1c2d3e-0000-4000-8000-000000000001";
const WEB: &str = "8f1c2d3e-0000-4000-8000-000000000002";

fn transport() -> Transport {
    Transport::new(
        &Credentials {
            token: Secret::new("sct_rw_acme_t".to_owned()),
            source: TokenSource::Env,
            refusals: Vec::new(),
            notice: None,
        },
        TransportConfig::default(),
        Box::new(RecordingSleeper::new()),
        Box::new(NoJitter),
    )
    .expect("builds")
}

fn ok(body: &Value) -> Route {
    Route::Json {
        status: 200,
        body: body.to_string(),
    }
}

fn page(entities: &Value, next: Option<String>) -> Route {
    let mut body = json!({"entities": entities});
    if let Some(next) = next {
        body["next_page_url"] = json!(next);
    }
    ok(&body)
}

fn base(server: &MockHTTPServer) -> Url {
    Url::parse(&server.base_url()).expect("parses")
}

fn whoami_body(scopes: &[&str]) -> Value {
    json!({"entity": {
        "member": {"id": "m-1", "mention_name": "ada", "name": "Ada"},
        "workspace": {"id": "w-1", "slug": "acme", "name": "Acme"},
        "authorization": {"scopes": scopes},
        "default_workflow_state": {"id": 502}
    }})
}

fn serve_a_workspace(server: &MockHTTPServer) {
    server.route(
        RequestKey::get("/api/v4/whoami"),
        ok(&whoami_body(&["read", "write"])),
    );
    server.route(
        RequestKey::get("/api/v4/acme/teams"),
        Route::Sequence(vec![
            page(
                &json!([{"id": MOBILE, "name": "Mobile Team", "mention_name": "mobile",
                         "default_workflow": {"id": 500}}]),
                Some(format!("{}/api/v4/acme/teams?cursor=2", server.base_url())),
            ),
            page(
                &json!([{"id": WEB, "name": "Web", "mention_name": null,
                         "default_workflow": null}]),
                None,
            ),
        ]),
    );
    server.route(
        RequestKey::get(
            "/api/v4/acme/teams/8f1c2d3e-0000-4000-8000-000000000001/workflows",
        ),
        page(&json!([{"id": 500}, {"id": 600}]), None),
    );
    server.route(
        RequestKey::get(
            "/api/v4/acme/teams/8f1c2d3e-0000-4000-8000-000000000002/workflows",
        ),
        page(&json!([{"id": 600}]), None),
    );
    server.route(
        RequestKey::get("/api/v4/acme/workflows"),
        page(
            &json!([{"id": 500, "name": "Engineering"}, {"id": 600, "name": "Design"}]),
            None,
        ),
    );
    server.route(
        RequestKey::get("/api/v4/acme/workflows/500/states"),
        page(
            &json!([
                {"id": 501, "name": "Backlog", "type": "backlog", "position": 1},
                {"id": 502, "name": "Ready", "type": "unstarted", "position": 2}
            ]),
            None,
        ),
    );
    server.route(
        RequestKey::get("/api/v4/acme/workflows/600/states"),
        page(&json!([{"id": 601, "name": "Brief", "type": "unstarted", "position": 1}]), None),
    );
}

#[test]
fn whoami_reports_the_slug_member_and_scopes() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get("/api/v4/whoami"),
        ok(&whoami_body(&["read"])),
    );

    let me = whoami(
        &transport(),
        &base(&server),
        &Deadline::starting_now(Duration::MAX),
    )
    .expect("resolves");

    assert_eq!(me.slug.as_str(), "acme");
    assert_eq!(me.workspace_name, "Acme");
    assert_eq!(me.member_mention_name, "ada");
    assert_eq!(me.scopes, vec!["read".to_owned()]);
    assert_eq!(me.default_state_id, Some(502));
}

#[test]
fn a_rejected_token_explains_itself() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get("/api/v4/whoami"),
        Route::Json {
            status: 401,
            body: "{}".to_owned(),
        },
    );

    let error = whoami(
        &transport(),
        &base(&server),
        &Deadline::starting_now(Duration::MAX),
    )
    .expect_err("refused");

    assert!(matches!(error, DiscoveryError::Refused { status: 401, .. }));
    assert!(
        error.to_string().contains("ACCELERATOR_SHORTCUT_TOKEN"),
        "{error}"
    );
}

#[test]
fn an_unsafe_slug_from_whoami_is_refused() {
    let server = MockHTTPServer::start();
    let mut body = whoami_body(&["read"]);
    body["entity"]["workspace"]["slug"] = json!("../evil");
    server.route(RequestKey::get("/api/v4/whoami"), ok(&body));

    let error = whoami(
        &transport(),
        &base(&server),
        &Deadline::starting_now(Duration::MAX),
    )
    .expect_err("refused");

    assert!(
        matches!(error, DiscoveryError::Unusable { .. }),
        "{error:?}"
    );
}

#[test]
fn discovery_pages_every_listing_and_builds_the_catalogue() {
    let server = MockHTTPServer::start();
    serve_a_workspace(&server);

    let (me, discovered) = discover(
        &transport(),
        &base(&server),
        &Deadline::starting_now(Duration::MAX),
    )
    .expect("discovers");

    assert_eq!(me.slug.as_str(), "acme");
    assert_eq!(discovered.listing, Listing::Complete);
    let catalogue = discovered.catalogue;
    assert_eq!(catalogue.teams().len(), 2);
    let mobile = catalogue.team_for_key("mobile").expect("catalogued");
    assert_eq!(mobile.default_workflow_id, Some(500));
    assert_eq!(mobile.workflow_ids, vec![500, 600]);
    assert_eq!(
        catalogue
            .team_for_key("Web")
            .expect("catalogued")
            .workflow_ids,
        vec![600]
    );
    assert_eq!(catalogue.workflow(500).expect("catalogued").states.len(), 2);
    assert_eq!(
        catalogue.workflow(600).expect("catalogued").states[0].name,
        "Brief"
    );
}

#[test]
fn an_expired_deadline_reports_the_listing_it_cut() {
    let server = MockHTTPServer::start();
    serve_a_workspace(&server);

    let (_, discovered) = discover(
        &transport(),
        &base(&server),
        &Deadline::starting_now(Duration::ZERO),
    )
    .expect("discovery reports rather than fails");

    assert!(
        matches!(&discovered.listing, Listing::Incomplete { cause, .. }
            if cause.contains("deadline")),
        "{:?}",
        discovered.listing
    );
}

#[test]
fn a_repeated_next_page_url_is_a_fault() {
    let server = MockHTTPServer::start();
    serve_a_workspace(&server);
    let same = format!("{}/api/v4/acme/workflows?cursor=1", server.base_url());
    server.route(
        RequestKey::get("/api/v4/acme/workflows"),
        page(&json!([{"id": 500, "name": "Engineering"}]), Some(same)),
    );

    let error = discover(
        &transport(),
        &base(&server),
        &Deadline::starting_now(Duration::MAX),
    )
    .expect_err("a loop is a fault");

    assert!(
        matches!(error, DiscoveryError::RepeatedPage { .. }),
        "{error:?}"
    );
}

#[test]
fn a_page_that_is_not_a_listing_is_unusable() {
    let server = MockHTTPServer::start();
    serve_a_workspace(&server);
    server.route(
        RequestKey::get("/api/v4/acme/workflows"),
        ok(&json!({"nope": 1})),
    );

    let error = discover(
        &transport(),
        &base(&server),
        &Deadline::starting_now(Duration::MAX),
    )
    .expect_err("unusable");

    assert!(
        matches!(error, DiscoveryError::Unusable { .. }),
        "{error:?}"
    );
}

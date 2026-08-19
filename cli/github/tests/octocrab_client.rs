//! Mock-server-backed tests for `OctocrabClient`: the HTTP-shaped branches
//! (repository-lookup and PR-existence-check request failure/non-2xx;
//! PATCH failure) and the credential/redirect seams the domain-level
//! `collaboration` unit tests cannot reach.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use collaboration::ForgeApiError;
use github::OctocrabClient;
use http_test_support::{MockServer, RequestKey, Route};

fn base_uri(server: &MockServer) -> http::Uri {
    server.base_url().parse().expect("base uri")
}

fn author_json(login: &str) -> String {
    format!(
        "{{\"login\":\"{login}\",\"id\":1,\"node_id\":\"n\",\
         \"avatar_url\":\"https://example.com/a\",\"gravatar_id\":\"\",\
         \"url\":\"https://api.github.com/users/{login}\",\
         \"html_url\":\"https://github.com/{login}\",\
         \"followers_url\":\"https://api.github.com/users/{login}/followers\",\
         \"following_url\":\"https://api.github.com/users/{login}/following{{/other_user}}\",\
         \"gists_url\":\"https://api.github.com/users/{login}/gists{{/gist_id}}\",\
         \"starred_url\":\"https://api.github.com/users/{login}/starred{{/owner}}{{/repo}}\",\
         \"subscriptions_url\":\"https://api.github.com/users/{login}/subscriptions\",\
         \"organizations_url\":\"https://api.github.com/users/{login}/orgs\",\
         \"repos_url\":\"https://api.github.com/users/{login}/repos\",\
         \"events_url\":\"https://api.github.com/users/{login}/events{{/privacy}}\",\
         \"received_events_url\":\"https://api.github.com/users/{login}/received_events\",\
         \"type\":\"User\",\"site_admin\":false}}"
    )
}

fn repository_json(owner: &str, name: &str, parent: Option<&str>) -> String {
    let parent_field = parent.map_or_else(String::new, |parent_json| {
        format!(",\"parent\":{parent_json}")
    });
    format!(
        "{{\"id\":1,\"name\":\"{name}\",\
         \"url\":\"https://api.github.com/repos/{owner}/{name}\",\
         \"owner\":{owner_json}{parent_field}}}",
        owner_json = author_json(owner)
    )
}

fn parent_repository_json(owner: &str, name: &str) -> String {
    format!(
        "{{\"id\":2,\"name\":\"{name}\",\
         \"url\":\"https://api.github.com/repos/{owner}/{name}\",\
         \"owner\":{owner_json}}}",
        owner_json = author_json(owner)
    )
}

fn pull_request_json(number: u64) -> String {
    format!(
        "{{\"id\":1,\"number\":{number},\
         \"url\":\"https://api.github.com/repos/owner/repo/pulls/{number}\",\
         \"head\":{{\"ref\":\"feature\",\"sha\":\"abc123\"}},\
         \"base\":{{\"ref\":\"main\",\"sha\":\"def456\"}},\
         \"locked\":false}}"
    )
}

#[tokio::test]
async fn repository_reports_no_parent_for_a_non_fork() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/owner/repo"),
        Route::Json {
            status: 200,
            body: repository_json("owner", "repo", None),
        },
    );
    let client =
        OctocrabClient::with_base_uri(base_uri(&server), None).unwrap();

    let details = client.repository("owner", "repo").await.unwrap();
    assert_eq!(details.parent_owner, None);
    assert_eq!(details.parent_repo, None);
}

#[tokio::test]
async fn repository_reports_the_parent_of_a_fork() {
    let server = MockServer::start();
    let parent = parent_repository_json("upstream-owner", "upstream-repo");
    server.route(
        RequestKey::new("GET", "/repos/fork-owner/fork-repo"),
        Route::Json {
            status: 200,
            body: repository_json("fork-owner", "fork-repo", Some(&parent)),
        },
    );
    let client =
        OctocrabClient::with_base_uri(base_uri(&server), None).unwrap();

    let details = client.repository("fork-owner", "fork-repo").await.unwrap();
    assert_eq!(details.parent_owner, Some("upstream-owner".to_owned()));
    assert_eq!(details.parent_repo, Some("upstream-repo".to_owned()));
}

#[tokio::test]
async fn repository_lookup_failure_surfaces_status_and_message() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/owner/repo"),
        Route::Json {
            status: 404,
            body: "{\"message\":\"Not Found\"}".to_owned(),
        },
    );
    let client =
        OctocrabClient::with_base_uri(base_uri(&server), None).unwrap();

    let error = client.repository("owner", "repo").await.unwrap_err();
    assert_eq!(
        error,
        ForgeApiError::Status {
            code: 404,
            message: "Not Found".to_owned()
        }
    );
}

#[tokio::test]
async fn repository_lookup_transport_failure_is_reported() {
    // No server listening at all — the client cannot even connect.
    let client = OctocrabClient::with_base_uri(
        "http://127.0.0.1:1".parse().unwrap(),
        None,
    )
    .unwrap();

    let error = client.repository("owner", "repo").await.unwrap_err();
    assert!(matches!(error, ForgeApiError::Transport(_)));
}

#[tokio::test]
async fn confirm_pull_request_exists_succeeds_for_a_present_pr() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/owner/repo/pulls/42"),
        Route::Json {
            status: 200,
            body: pull_request_json(42),
        },
    );
    let client =
        OctocrabClient::with_base_uri(base_uri(&server), None).unwrap();

    client
        .confirm_pull_request_exists("owner", "repo", 42)
        .await
        .unwrap();
}

#[tokio::test]
async fn confirm_pull_request_exists_reports_a_404_as_failed() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/owner/repo/pulls/42"),
        Route::Json {
            status: 404,
            body: "{\"message\":\"Not Found\"}".to_owned(),
        },
    );
    let client =
        OctocrabClient::with_base_uri(base_uri(&server), None).unwrap();

    let error = client
        .confirm_pull_request_exists("owner", "repo", 42)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        ForgeApiError::Status {
            code: 404,
            message: "Not Found".to_owned()
        }
    );
}

#[tokio::test]
async fn update_body_succeeds() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("PATCH", "/repos/owner/repo/pulls/42"),
        Route::Json {
            status: 200,
            body: pull_request_json(42),
        },
    );
    let client =
        OctocrabClient::with_base_uri(base_uri(&server), None).unwrap();

    client
        .update_body("owner", "repo", 42, "new body")
        .await
        .unwrap();
}

#[tokio::test]
async fn update_body_reports_a_patch_failure() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("PATCH", "/repos/owner/repo/pulls/42"),
        Route::Json {
            status: 422,
            body: "{\"message\":\"Validation Failed\"}".to_owned(),
        },
    );
    let client =
        OctocrabClient::with_base_uri(base_uri(&server), None).unwrap();

    let error = client
        .update_body("owner", "repo", 42, "new body")
        .await
        .unwrap_err();
    assert_eq!(
        error,
        ForgeApiError::Status {
            code: 422,
            message: "Validation Failed".to_owned()
        }
    );
}

#[tokio::test]
async fn a_redirect_response_is_not_followed() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/owner/repo"),
        Route::Redirect {
            status: 301,
            location: "https://attacker.example/repos/owner/repo".to_owned(),
        },
    );
    let client =
        OctocrabClient::with_base_uri(base_uri(&server), None).unwrap();

    let error = client.repository("owner", "repo").await.unwrap_err();
    assert!(
        !matches!(error, ForgeApiError::Status { code: 200, .. }),
        "a redirect must not resolve as a success"
    );
}

#[tokio::test]
async fn the_configured_token_reaches_the_outbound_request() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/owner/repo"),
        Route::Json {
            status: 200,
            body: repository_json("owner", "repo", None),
        },
    );
    let client = OctocrabClient::with_base_uri(
        base_uri(&server),
        Some("secret-token".to_owned()),
    )
    .unwrap();

    client.repository("owner", "repo").await.unwrap();

    assert_eq!(
        server.last_header(
            &RequestKey::get("/repos/owner/repo"),
            "authorization"
        ),
        Some("Bearer secret-token".to_owned())
    );
}

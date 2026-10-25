#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use support::assert_golden;
use support::fetch_from;
use support::web_fixture;
use support::Run;
use support::CALL_BUDGET_MS;

const RESOLVE: &str = "ACCELERATOR_RESEARCH_TEST_RESOLVE";
const FIXTURE_HOST: &str = "fixture.accelerator.test";
const ARTICLE: &str = "/article";

fn article() -> RequestKey {
    RequestKey::get(ARTICLE)
}

fn fixture_url() -> String {
    format!("http://{FIXTURE_HOST}{ARTICLE}")
}

fn serving(route: Route) -> MockHTTPServer {
    let server = MockHTTPServer::start();
    server.route(article(), route);
    server
}

fn html_page() -> Route {
    Route::Headers {
        status: 200,
        headers: vec![(
            "Content-Type".to_owned(),
            "text/html; charset=utf-8".to_owned(),
        )],
        body: web_fixture("highwire.html"),
    }
}

fn stand_in_for(server: &MockHTTPServer) -> String {
    format!("{FIXTURE_HOST}={}", server.address())
}

/// Runs `fetch web metadata <url>` from a directory outside any project.
fn web_metadata(url: &str, extra: &[&str], env: &[(&str, &str)]) -> Run {
    let outside = tempfile::tempdir().expect("tempdir");
    let mut args = vec!["web", "metadata", url];
    args.extend_from_slice(extra);
    fetch_from(outside.path(), &args, env)
}

fn through_stand_in(server: &MockHTTPServer, url: &str) -> Run {
    web_metadata(url, &[], &[(RESOLVE, &stand_in_for(server))])
}

fn assert_failed_with(run: &Run, line: &str) {
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, format!("{line}\n"));
}

#[test]
fn a_page_prints_its_declared_metadata() {
    let server = serving(html_page());

    let run = through_stand_in(&server, &fixture_url());

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stderr, "");
    assert_golden("web-metadata.json", &run.stdout);
}

#[test]
fn a_page_needs_no_project_and_ignores_academic_overrides() {
    let server = serving(html_page());

    let run = web_metadata(
        &fixture_url(),
        &[],
        &[
            (RESOLVE, &stand_in_for(&server)),
            (
                "ACCELERATOR_OPENALEX_API_URL",
                "https://not-loopback.example",
            ),
            ("ACCELERATOR_ARXIV_API_URL", "https://not-loopback.example"),
        ],
    );

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_golden("web-metadata.json", &run.stdout);
}

#[test]
fn a_page_that_is_not_found_fails_without_naming_its_url() {
    let server = serving(Route::Status(404));

    let run = through_stand_in(&server, &fixture_url());

    assert_failed_with(&run, "E_WEB_STATUS: the page answered HTTP 404");
    assert!(!run.stderr.contains(FIXTURE_HOST));
}

#[test]
fn a_url_that_is_not_http_is_a_usage_error() {
    let run = web_metadata("file:///etc/passwd", &[], &[]);

    assert_eq!(run.code, Some(2));
    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "E_WEB_URL_MALFORMED: the URL is not a fetchable http(s) URL: only \
         http and https are fetched\n"
    );
}

#[test]
fn a_loopback_address_is_rejected_unrequested() {
    let server = serving(html_page());
    let url = format!("http://127.0.0.1:{}{ARTICLE}", server.address().port());

    let run = through_stand_in(&server, &url);

    assert_failed_with(
        &run,
        "E_WEB_HOST_REJECTED: the page's host is a loopback address",
    );
    assert_eq!(server.hits(&article()), 0);
}

#[test]
fn localhost_is_rejected_on_the_address_the_real_resolver_answers() {
    let server = serving(html_page());
    let url = format!("http://localhost:{}{ARTICLE}", server.address().port());

    let run = web_metadata(&url, &[], &[]);

    assert_failed_with(
        &run,
        "E_WEB_HOST_REJECTED: the page's host is a loopback address",
    );
    assert_eq!(server.hits(&article()), 0);
}

#[test]
fn a_fullwidth_address_is_never_requested() {
    let server = serving(html_page());
    let url = format!(
        "http://１２７.０.０.１:{}{ARTICLE}",
        server.address().port()
    );

    let run = web_metadata(&url, &[], &[]);

    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert_eq!(server.hits(&article()), 0);
}

#[test]
fn a_redirect_to_a_private_address_is_rejected() {
    let server = serving(Route::Redirect {
        status: 302,
        location: "http://10.0.0.1/".to_owned(),
    });

    let run = through_stand_in(&server, &fixture_url());

    assert_failed_with(
        &run,
        "E_WEB_HOST_REJECTED: the page's host is a private address",
    );
}

#[test]
fn a_page_that_stalls_past_the_call_budget_times_out() {
    let server = serving(Route::Stall(Duration::from_secs(5)));

    let run = web_metadata(
        &fixture_url(),
        &[],
        &[(RESOLVE, &stand_in_for(&server)), (CALL_BUDGET_MS, "500")],
    );

    assert_failed_with(
        &run,
        "E_WEB_TIMED_OUT: the page did not answer within the time allowed",
    );
}

#[test]
fn a_limit_or_ticket_with_web_is_a_usage_error() {
    for (extra, message) in [
        (
            &["--limit", "3"][..],
            "E_RESEARCH_USAGE: --limit applies only to openalex and arxiv \
             fetches",
        ),
        (
            &["--ticket", "42-9f1c2a"][..],
            "E_RESEARCH_USAGE: --ticket applies only to arxiv fetches",
        ),
    ] {
        let run = web_metadata("https://example.org/", extra, &[]);

        assert_eq!(run.code, Some(2), "{extra:?}");
        assert_eq!(run.stderr, format!("{message}\n"), "{extra:?}");
    }
}

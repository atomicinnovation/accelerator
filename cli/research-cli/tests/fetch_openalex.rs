#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::MockServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use support::config_with;
use support::fetch;
use support::openalex_fixture;
use support::page_of;
use support::Project;
use support::SELECT;

const KEY: &str = "openalex-sentinel-key";
const WORKS: &str = "/works";
const PEERJ: &str = "/works/W2741809807";

fn works() -> RequestKey {
    RequestKey::get(WORKS)
}

fn peerj() -> RequestKey {
    RequestKey::get(PEERJ)
}

const fn json(status: u16, body: String) -> Route {
    Route::Json { status, body }
}

fn server_with(key: RequestKey, route: Route) -> MockServer {
    let server = MockServer::start();
    server.route(key, route);
    server
}

fn headers(status: u16, headers: &[(&str, &str)]) -> Route {
    Route::Headers {
        status,
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        body: String::new(),
    }
}

fn search_page() -> Route {
    json(200, page_of(&["work-doi.json"]))
}

fn assert_usage_error(args: &[&str], message: &str) {
    let server = MockServer::start();
    let run = fetch(&Project::new(), &server, args, &[]);
    assert_eq!(run.code, Some(2), "{args:?}: {}", run.stderr);
    assert!(
        run.stderr.contains(message),
        "{args:?}: expected {message:?} in {}",
        run.stderr
    );
    assert_eq!(server.hits(&works()), 0);
}

#[test]
fn a_limit_outside_one_to_twenty_five_is_a_usage_error() {
    assert_usage_error(
        &["openalex", "search", "graphs", "--limit", "0"],
        "E_RESEARCH_USAGE: --limit must be 1–25 (got 0)",
    );
    assert_usage_error(
        &["openalex", "search", "graphs", "--limit", "26"],
        "E_RESEARCH_USAGE: --limit must be 1–25 (got 26)",
    );
}

#[test]
fn an_unknown_family_or_verb_is_a_usage_error() {
    assert_usage_error(
        &["openAlex", "search", "graphs"],
        "E_RESEARCH_USAGE: unknown family 'openAlex' (expected openalex or \
         arxiv)",
    );
    assert_usage_error(
        &["openalex", "find", "graphs"],
        "E_RESEARCH_USAGE: unknown verb 'find' (expected search or lookup)",
    );
}

#[test]
fn a_missing_or_empty_query_is_a_usage_error() {
    assert_usage_error(
        &["openalex", "search"],
        "E_RESEARCH_USAGE: search needs a query",
    );
    assert_usage_error(
        &["openalex", "search", ",:|!"],
        "E_RESEARCH_USAGE: the query is empty once reserved characters are \
         removed",
    );
}

#[test]
fn a_missing_or_malformed_id_is_a_usage_error() {
    assert_usage_error(
        &["openalex", "lookup"],
        "E_RESEARCH_USAGE: lookup needs an ID",
    );
    assert_usage_error(
        &["openalex", "lookup", "X123"],
        "E_OPENALEX_ID_MALFORMED: expected W123, https://openalex.org/W123, \
         or a DOI (got 'X123')",
    );
}

#[test]
fn a_doi_with_a_parent_segment_is_a_usage_error() {
    assert_usage_error(
        &["openalex", "lookup", "10.1234/a/../b"],
        "E_OPENALEX_ID_MALFORMED",
    );
}

#[test]
fn help_documents_the_families_verbs_limit_range_and_deadline() {
    let output =
        std::process::Command::new(env!("CARGO_BIN_EXE_accelerator-research"))
            .args(["fetch", "--help"])
            .output()
            .expect("run --help");
    let help = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(0));
    for expected in ["openalex", "arxiv", "search", "lookup", "1–25", "100 s"]
    {
        assert!(help.contains(expected), "{expected:?} missing from {help}");
    }
}

#[test]
fn a_search_returns_at_most_the_limit_and_sends_the_normalised_filter() {
    let server =
        server_with(works(), json(200, openalex_fixture("search-30.json")));

    let run = fetch(
        &Project::new(),
        &server,
        &["openalex", "search", "graph,", "neural", "networks"],
        &[],
    );

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let records = run.json()["records"].as_array().expect("records").len();
    assert_eq!(records, 10);
    assert_eq!(
        server.last_query(&works()),
        Some(format!(
            "filter=title_and_abstract.search:graph+neural+networks\
             &per_page=10&{SELECT}"
        ))
    );
}

#[test]
fn a_query_carrying_parameter_syntax_is_encoded_into_one_filter_value() {
    let server = server_with(works(), search_page());

    fetch(
        &Project::new(),
        &server,
        &["openalex", "search", "graphs&per_page=200"],
        &[],
    );

    assert_eq!(
        server.last_query(&works()),
        Some(format!(
            "filter=title_and_abstract.search:graphs%26per_page%3D200\
             &per_page=10&{SELECT}"
        ))
    );
}

#[test]
fn recorded_works_render_every_field() {
    let server = server_with(
        works(),
        json(
            200,
            page_of(&[
                "work-doi.json",
                "work-no-doi.json",
                "work-retracted.json",
            ]),
        ),
    );

    let run =
        fetch(&Project::new(), &server, &["openalex", "search", "x"], &[]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    support::assert_golden("openalex-search.json", &run.stdout);
    let records = run.json()["records"].clone();
    assert_eq!(records[0]["url"], "https://doi.org/10.7717/peerj.4375");
    assert_eq!(records[1]["url"], "https://openalex.org/W2144634347");
    assert_eq!(
        records[2]["url"],
        "https://doi.org/10.1016/s0140-6736%2820%2930367-6"
    );
    assert_eq!(records[2]["retracted"], true);
    assert_eq!(records[2]["tier"], "tier-3");
    assert!(records[0]["abstract"].is_string());
    let signals = records[0]["venue_signals"]
        .as_object()
        .expect("venue signals");
    let mut keys = signals.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "is_core",
            "is_retracted",
            "listed_in",
            "source_type",
            "type",
            "version"
        ]
    );
}

#[test]
fn a_work_is_looked_up_by_id_by_openalex_url_and_by_doi() {
    let server = MockServer::start();
    server.route(peerj(), json(200, openalex_fixture("work-doi.json")));
    let doi_path = "/works/doi:10.1016/s0140-6736%2820%2930367-6";
    server.route(
        RequestKey::get(doi_path),
        json(200, openalex_fixture("work-retracted.json")),
    );
    let project = Project::new();

    for id in [
        "W2741809807",
        "https://openalex.org/W2741809807",
        "10.1016/s0140-6736(20)30367-6",
    ] {
        let run = fetch(&project, &server, &["openalex", "lookup", id], &[]);
        assert_eq!(run.code, Some(0), "{id}: {}", run.stderr);
        assert_eq!(
            run.json()["records"].as_array().map(Vec::len),
            Some(1),
            "{id}"
        );
    }
    assert_eq!(server.hits(&peerj()), 2);
    assert_eq!(server.hits(&RequestKey::get(doi_path)), 1);
}

#[test]
fn a_lookup_miss_is_ok_with_no_records() {
    let server = server_with(peerj(), Route::Status(404));

    let run = fetch(
        &Project::new(),
        &server,
        &["openalex", "lookup", "W2741809807"],
        &[],
    );

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout.trim(), r#"{"status":"ok","records":[]}"#);
}

#[test]
fn a_rejected_key_fails_naming_where_it_came_from() {
    for status in [401, 403] {
        let server = server_with(peerj(), Route::Status(status));

        let run = fetch(
            &Project::new(),
            &server,
            &["openalex", "lookup", "W2741809807"],
            &[("ACCELERATOR_OPENALEX_API_KEY", KEY)],
        );

        assert_eq!(run.code, Some(1), "{status}");
        assert!(
            run.stderr.contains(
                "E_OPENALEX_KEY_REJECTED: the OpenAlex API key from \
                 ACCELERATOR_OPENALEX_API_KEY was rejected"
            ),
            "{status}: {}",
            run.stderr
        );
        assert_eq!(server.hits(&peerj()), 1);
    }
}

#[test]
fn a_refused_keyless_request_asks_for_a_key() {
    let server = server_with(peerj(), Route::Status(403));

    let run = fetch(
        &Project::new(),
        &server,
        &["openalex", "lookup", "W2741809807"],
        &[],
    );

    assert_eq!(run.code, Some(1));
    assert!(
        run.stderr.contains("E_OPENALEX_UNAUTHENTICATED"),
        "{}",
        run.stderr
    );
}

#[test]
fn another_client_error_fails() {
    let server = server_with(works(), Route::Status(400));

    let run =
        fetch(&Project::new(), &server, &["openalex", "search", "x"], &[]);

    assert_eq!(run.code, Some(1));
    assert!(run.stderr.contains("HTTP 400"), "{}", run.stderr);
}

#[test]
fn an_exhausted_budget_is_unavailable_after_one_attempt() {
    for route in [
        Route::Status(409),
        headers(
            429,
            &[
                ("X-RateLimit-Remaining", "5"),
                ("X-RateLimit-Credits-Required", "10"),
            ],
        ),
        headers(429, &[("X-RateLimit-Remaining", "0")]),
    ] {
        let server = server_with(works(), route);

        let run =
            fetch(&Project::new(), &server, &["openalex", "search", "x"], &[]);

        assert_eq!(run.code, Some(0), "{}", run.stderr);
        assert_eq!(
            run.stdout.trim(),
            r#"{"status":"unavailable","source":"openalex","reason":"budget_exhausted","authenticated":false}"#
        );
        assert_eq!(server.hits(&works()), 1);
    }
}

#[test]
fn a_keyed_exhausted_budget_reports_it_was_authenticated() {
    let server = server_with(works(), Route::Status(409));

    let run = fetch(
        &Project::new(),
        &server,
        &["openalex", "search", "x"],
        &[("ACCELERATOR_OPENALEX_API_KEY", KEY)],
    );

    assert_eq!(run.json()["authenticated"], true);
}

#[test]
fn persistent_upstream_errors_retry_on_the_schedule_then_go_unavailable() {
    let server = server_with(works(), Route::Status(503));

    let run =
        fetch(&Project::new(), &server, &["openalex", "search", "x"], &[]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(server.hits(&works()), 4);
    assert_eq!(run.waits, [3000, 6000, 12000]);
    assert_eq!(run.json()["reason"], "upstream_error");
    assert!(
        run.stderr.contains(
            "openalex search unavailable (upstream_error) after 4 attempts"
        ),
        "{}",
        run.stderr
    );
}

#[test]
fn a_retry_after_hint_replaces_the_backoff() {
    let server = server_with(
        works(),
        Route::Sequence(vec![
            headers(429, &[("Retry-After", "5")]),
            search_page(),
        ]),
    );

    let run =
        fetch(&Project::new(), &server, &["openalex", "search", "x"], &[]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.waits, [5000]);
    assert_eq!(run.json()["records"].as_array().map(Vec::len), Some(1));
}

#[test]
fn a_same_origin_redirect_reaches_the_surviving_work() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/works/W1"),
        Route::Redirect {
            status: 301,
            location: format!("{}{PEERJ}?{SELECT}", server.base_url()),
        },
    );
    server.route(peerj(), json(200, openalex_fixture("work-doi.json")));

    let run =
        fetch(&Project::new(), &server, &["openalex", "lookup", "W1"], &[]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(
        run.json()["records"][0]["url"],
        "https://doi.org/10.7717/peerj.4375"
    );
}

#[test]
fn a_redirect_off_the_origin_fails_without_offering_the_key() {
    let server = MockServer::start();
    let elsewhere = MockServer::start();
    elsewhere.route(peerj(), json(200, openalex_fixture("work-doi.json")));
    let https = server.base_url().replacen("http://", "https://", 1);
    for (id, location) in [
        ("W1", format!("{}{PEERJ}", elsewhere.base_url())),
        ("W2", format!("{https}{PEERJ}")),
    ] {
        server.route(
            RequestKey::get(&format!("/works/{id}")),
            Route::Redirect {
                status: 301,
                location,
            },
        );

        let run = fetch(
            &Project::new(),
            &server,
            &["openalex", "lookup", id],
            &[("ACCELERATOR_OPENALEX_API_KEY", KEY)],
        );

        assert_eq!(run.code, Some(1), "{id}: {}", run.stderr);
    }
    assert_eq!(elsewhere.hits(&peerj()), 0);
    assert_eq!(elsewhere.last_header(&peerj(), "Authorization"), None);
}

#[test]
fn a_fourth_redirect_fails() {
    let server = MockServer::start();
    for hop in 1..=4 {
        server.route(
            RequestKey::get(&format!("/works/W{hop}")),
            Route::Redirect {
                status: 301,
                location: format!("{}/works/W{}", server.base_url(), hop + 1),
            },
        );
    }
    server.route(
        RequestKey::get("/works/W5"),
        json(200, openalex_fixture("work-doi.json")),
    );

    let run =
        fetch(&Project::new(), &server, &["openalex", "lookup", "W1"], &[]);

    assert_eq!(run.code, Some(1), "{}", run.stderr);
}

#[test]
fn an_oversized_body_fails() {
    let server = server_with(
        works(),
        Route::Bytes {
            status: 200,
            body: vec![b' '; 8 * 1024 * 1024 + 1],
        },
    );

    let run =
        fetch(&Project::new(), &server, &["openalex", "search", "x"], &[]);

    assert_eq!(run.code, Some(1));
}

#[test]
fn a_malformed_body_fails_as_undecodable() {
    let server = server_with(works(), json(200, "{\"results\":".to_owned()));

    let run =
        fetch(&Project::new(), &server, &["openalex", "search", "x"], &[]);

    assert_eq!(run.code, Some(1));
    assert!(
        run.stderr.contains(
            "E_RESEARCH_UNDECODABLE: OpenAlex returned a response that could \
             not be read"
        ),
        "{}",
        run.stderr
    );
}

#[test]
fn every_request_names_its_client() {
    let server = server_with(works(), search_page());

    fetch(&Project::new(), &server, &["openalex", "search", "x"], &[]);

    assert_eq!(
        server.last_header(&works(), "User-Agent"),
        Some(format!(
            "accelerator-research/{}",
            env!("CARGO_PKG_VERSION")
        ))
    );
}

fn authorization(project: &Project, env: &[(&str, &str)]) -> Option<String> {
    let server = server_with(works(), search_page());
    let run = fetch(project, &server, &["openalex", "search", "x"], env);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    server.last_header(&works(), "Authorization")
}

fn bearer(key: &str) -> String {
    format!("Bearer {key}")
}

#[test]
fn each_rung_of_the_ladder_supplies_the_key() {
    assert_eq!(
        authorization(
            &Project::new(),
            &[("ACCELERATOR_OPENALEX_API_KEY", "k1")]
        ),
        Some(bearer("k1"))
    );
    assert_eq!(
        authorization(
            &Project::new(),
            &[("ACCELERATOR_OPENALEX_API_KEY_CMD", "printf k2")]
        ),
        Some(bearer("k2"))
    );
    assert_eq!(
        authorization(
            &Project::new()
                .personal_config(&config_with("  api_key: k3\n"), 0o600),
            &[]
        ),
        Some(bearer("k3"))
    );
    assert_eq!(
        authorization(
            &Project::new().personal_config(
                &config_with("  api_key_cmd: printf k4\n"),
                0o600
            ),
            &[]
        ),
        Some(bearer("k4"))
    );
    assert_eq!(
        authorization(
            &Project::new().shared_config(&config_with("  api_key: k5\n")),
            &[]
        ),
        Some(bearer("k5"))
    );
}

#[test]
fn a_personal_config_without_a_key_leaves_the_call_keyless() {
    let project = Project::new()
        .personal_config("---\n---\n", 0o600)
        .shared_config(&config_with("  api_key: shared\n"));

    assert_eq!(authorization(&project, &[]), None);
}

#[test]
fn no_key_anywhere_leaves_the_call_keyless() {
    assert_eq!(authorization(&Project::new(), &[]), None);
}

#[test]
fn a_shared_key_command_is_refused_before_any_request() {
    let server = server_with(works(), search_page());
    let project =
        Project::new().shared_config(&config_with("  api_key_cmd: printf k\n"));

    let run = fetch(&project, &server, &["openalex", "search", "x"], &[]);

    assert_eq!(run.code, Some(1));
    assert!(
        run.stderr
            .contains("E_TOKEN_CMD_FROM_SHARED_CONFIG: openalex.api_key_cmd"),
        "{}",
        run.stderr
    );
    assert_eq!(server.hits(&works()), 0);
}

#[test]
fn an_environment_key_outranks_a_shared_key_command() {
    let project =
        Project::new().shared_config(&config_with("  api_key_cmd: printf k\n"));

    assert_eq!(
        authorization(&project, &[("ACCELERATOR_OPENALEX_API_KEY", "env")]),
        Some(bearer("env"))
    );
}

fn assert_refused(project: &Project, message: &str) {
    let server = server_with(works(), search_page());
    let run = fetch(project, &server, &["openalex", "search", "x"], &[]);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(
        run.stderr.contains(message),
        "{message:?} in {}",
        run.stderr
    );
    assert_eq!(server.hits(&works()), 0);
}

#[test]
fn a_tracked_personal_config_supplying_a_key_is_refused() {
    assert_refused(
        &Project::under_git()
            .personal_config(&config_with("  api_key: leaked\n"), 0o600)
            .track_personal_config(),
        "E_TOKEN_FROM_TRACKED_FILE: openalex.api_key",
    );
    assert_refused(
        &Project::under_git()
            .personal_config(&config_with("  api_key_cmd: printf k\n"), 0o600)
            .track_personal_config(),
        "E_TOKEN_CMD_FROM_TRACKED_FILE: openalex.api_key_cmd",
    );
}

#[test]
fn a_group_readable_personal_config_supplying_a_key_is_refused() {
    assert_refused(
        &Project::under_git()
            .personal_config(&config_with("  api_key: leaked\n"), 0o640)
            .track_personal_config(),
        "E_LOCAL_PERMS_INSECURE",
    );
}

#[test]
fn a_tracked_personal_config_without_a_key_leaves_the_call_keyless() {
    let project = Project::under_git()
        .personal_config("---\n---\n", 0o600)
        .track_personal_config();

    assert_eq!(authorization(&project, &[]), None);
}

#[test]
fn no_key_leaks_into_the_url_or_any_output() {
    let server = server_with(works(), Route::Status(503));
    let keyed = fetch(
        &Project::new(),
        &server,
        &["openalex", "search", "x"],
        &[("ACCELERATOR_OPENALEX_API_KEY", KEY)],
    );
    let failing_command = fetch(
        &Project::new(),
        &server,
        &["openalex", "search", "x"],
        &[(
            "ACCELERATOR_OPENALEX_API_KEY_CMD",
            &format!("printf {KEY}; exit 1"),
        )],
    );

    assert!(server
        .last_query(&works())
        .is_some_and(|query| !query.contains(KEY)));
    for run in [&keyed, &failing_command] {
        assert!(!run.stdout.contains(KEY), "{}", run.stdout);
        assert!(!run.stderr.contains(KEY), "{}", run.stderr);
    }
    assert_eq!(failing_command.code, Some(1));
}

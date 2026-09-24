#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::MockServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use support::arxiv_fixture;
use support::assert_golden;
use support::fetch;
use support::Project;
use support::Run;

const WITHDRAWN: &str = "2608.21129";

fn query() -> RequestKey {
    RequestKey::get("/api/query")
}

fn oai() -> RequestKey {
    RequestKey::get("/oai")
}

fn feed(name: &str) -> Route {
    xml(200, arxiv_fixture(name))
}

const fn xml(status: u16, body: String) -> Route {
    Route::Bytes {
        status,
        body: body.into_bytes(),
    }
}

fn server_with(routes: Vec<(RequestKey, Route)>) -> MockServer {
    let server = MockServer::start();
    for (key, route) in routes {
        server.route(key, route);
    }
    server
}

fn records(run: &Run) -> Vec<serde_json::Value> {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let document = run.json();
    assert_eq!(document["status"], "ok", "{document}");
    document["records"].as_array().expect("records").clone()
}

fn search_with_first_comment(comment: &str) -> Route {
    let feed = arxiv_fixture("search-3.xml").replacen(
        "12 pages, 7 figures, 7 tables; published in Neural Networks; code \
         available at https://github.com/cynricfu/MECCH",
        comment,
        1,
    );
    xml(200, feed)
}

#[test]
fn a_search_returns_tier_two_records_and_sends_the_translated_query() {
    let server = server_with(vec![(query(), feed("search-3.xml"))]);

    let run = fetch(
        &Project::new(),
        &server,
        &[
            "arxiv", "search", "graph", "neural", "networks", "--limit", "3",
        ],
        &[],
    );

    let records = records(&run);
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|record| record["tier"] == "tier-2"));
    assert_eq!(
        server.last_query(&query()),
        Some(
            "search_query=all:graph+AND+all:neural+AND+all:networks\
             &max_results=3&sortBy=relevance"
                .to_owned()
        )
    );
    assert_eq!(server.hits(&oai()), 0);
}

#[test]
fn recorded_entries_render_every_field() {
    let server = server_with(vec![(query(), feed("search-3.xml"))]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "search", "graphs"],
        &[],
    );

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_golden("arxiv-search.json", &run.stdout);
}

#[test]
fn a_versioned_lookup_queries_the_stripped_id_and_returns_its_entry() {
    let server = server_with(vec![(query(), feed("search-3.xml"))]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "lookup", "2211.12792v2"],
        &[],
    );

    let records = records(&run);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["url"], "https://arxiv.org/abs/2211.12792");
    assert_eq!(
        server.last_query(&query()),
        Some("id_list=2211.12792".to_owned())
    );
}

#[test]
fn a_confirmed_withdrawal_is_tier_three_and_withdrawn() {
    let server = server_with(vec![
        (query(), feed("lookup-2608.21129.xml")),
        (oai(), feed("oai-2608.21129.xml")),
    ]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "lookup", WITHDRAWN],
        &[],
    );

    let records = records(&run);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["tier"], "tier-3");
    assert_eq!(records[0]["withdrawn"], true);
    assert_eq!(records[0]["retracted"], false);
    assert_eq!(
        server.last_query(&oai()),
        Some(format!(
            "verb=GetRecord&identifier=oai:arXiv.org:{WITHDRAWN}\
             &metadataPrefix=arXivRaw"
        ))
    );
}

#[test]
fn a_withdrawal_comment_on_a_live_latest_version_stays_tier_two() {
    let server = server_with(vec![
        (
            query(),
            search_with_first_comment(
                "This paper has been withdrawn from the workshop and \
                 extended",
            ),
        ),
        (oai(), feed("oai-2211.12792.xml")),
    ]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "search", "graphs"],
        &[],
    );

    let records = records(&run);
    assert_eq!(server.hits(&oai()), 1);
    assert_eq!(records[0]["tier"], "tier-2");
    assert_eq!(records[0]["withdrawn"], false);
}

#[test]
fn a_confirmation_that_never_succeeds_makes_the_call_unavailable() {
    let server = server_with(vec![
        (query(), feed("lookup-2608.21129.xml")),
        (oai(), Route::Status(503)),
    ]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "lookup", WITHDRAWN],
        &[],
    );

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(
        run.json(),
        serde_json::json!({
            "status": "unavailable",
            "source": "arxiv",
            "reason": "upstream_error",
        })
    );
    assert_eq!(server.hits(&oai()), 4);
}

#[test]
fn a_confirmed_withdrawal_is_not_confirmed_again_by_a_later_call() {
    let project = Project::new();
    let server = server_with(vec![
        (query(), feed("lookup-2608.21129.xml")),
        (oai(), feed("oai-2608.21129.xml")),
    ]);

    fetch(&project, &server, &["arxiv", "lookup", WITHDRAWN], &[]);
    let again = fetch(&project, &server, &["arxiv", "lookup", WITHDRAWN], &[]);

    assert_eq!(records(&again)[0]["withdrawn"], true);
    assert_eq!(server.hits(&query()), 2);
    assert_eq!(server.hits(&oai()), 1);
}

#[test]
fn an_empty_feed_is_ok_with_no_records() {
    let server = server_with(vec![(query(), feed("empty.xml"))]);

    let run =
        fetch(&Project::new(), &server, &["arxiv", "search", "zzqx"], &[]);

    assert!(records(&run).is_empty());
}

#[test]
fn a_lookup_answered_with_some_other_entry_is_ok_with_no_records() {
    let server = server_with(vec![(query(), feed("search-3.xml"))]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "lookup", "2601.00001"],
        &[],
    );

    assert!(records(&run).is_empty());
}

#[test]
fn an_error_feed_fails_the_call_with_arxivs_reason() {
    let server = server_with(vec![(query(), feed("error.xml"))]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "lookup", "2608.21129"],
        &[],
    );

    assert_eq!(run.code, Some(1));
    assert!(
        run.stderr.contains(
            "E_RESEARCH_CLIENT_ERROR: arXiv rejected the request: incorrect \
             id format for 2608.2112x"
        ),
        "{}",
        run.stderr
    );
    assert_eq!(run.stdout, "");
}

#[test]
fn a_malformed_feed_is_undecodable() {
    let server = server_with(vec![(query(), xml(200, "<feed".to_owned()))]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "search", "graphs"],
        &[],
    );

    assert_eq!(run.code, Some(1));
    assert!(
        run.stderr
            .contains("E_RESEARCH_UNDECODABLE: arXiv returned"),
        "{}",
        run.stderr
    );
}

#[test]
fn a_malformed_id_is_a_usage_error_before_any_request() {
    let server = MockServer::start();

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "lookup", "2608.2112x"],
        &[],
    );

    assert_eq!(run.code, Some(2));
    assert!(
        run.stderr.contains(
            "E_ARXIV_ID_MALFORMED: expected 2608.21129[vN] or \
             archive/1234567 (got '2608.2112x')"
        ),
        "{}",
        run.stderr
    );
    assert_eq!(server.hits(&query()), 0);
}

#[test]
fn a_forbidden_or_not_acceptable_answer_is_retried_as_throttling() {
    for status in [403, 406] {
        let server = server_with(vec![(query(), Route::Status(status))]);

        let run = fetch(
            &Project::new(),
            &server,
            &["arxiv", "search", "graphs"],
            &[],
        );

        assert_eq!(run.code, Some(0), "{status}: {}", run.stderr);
        assert_eq!(run.json()["reason"], "rate_limited", "{status}");
        assert_eq!(server.hits(&query()), 4, "{status}");
        assert_eq!(run.waits, [3000, 6000, 12000], "{status}");
    }
}

#[test]
fn both_the_query_and_the_confirmation_ask_for_atom_as_accelerator() {
    let server = server_with(vec![
        (query(), feed("lookup-2608.21129.xml")),
        (oai(), feed("oai-2608.21129.xml")),
    ]);

    fetch(
        &Project::new(),
        &server,
        &["arxiv", "lookup", WITHDRAWN],
        &[],
    );

    for key in [query(), oai()] {
        assert_eq!(
            server.last_header(&key, "accept").as_deref(),
            Some("application/atom+xml"),
            "{key:?}"
        );
        assert!(
            server
                .last_header(&key, "user-agent")
                .is_some_and(|agent| agent.starts_with("accelerator-research/")),
            "{key:?}"
        );
    }
}

#[test]
fn a_confirmation_is_paced_three_seconds_after_its_query() {
    let server = server_with(vec![
        (query(), feed("lookup-2608.21129.xml")),
        (oai(), feed("oai-2608.21129.xml")),
    ]);

    let run = fetch(
        &Project::new(),
        &server,
        &["arxiv", "lookup", WITHDRAWN],
        &[],
    );

    assert_eq!(run.waits, [3000], "{}", run.stderr);
}

#[test]
fn a_throttled_call_defers_the_next_calls_first_request() {
    let project = Project::new();
    let server = server_with(vec![(
        query(),
        Route::Sequence(vec![
            Route::Headers {
                status: 429,
                headers: vec![("Retry-After".to_owned(), "20".to_owned())],
                body: String::new(),
            },
            feed("search-3.xml"),
        ]),
    )]);

    let throttled =
        fetch(&project, &server, &["arxiv", "search", "graphs"], &[]);
    project.clear_clock_log();
    let deferred =
        fetch(&project, &server, &["arxiv", "search", "graphs"], &[]);

    let deferral: u64 = throttled.waits.iter().sum();
    assert_eq!(throttled.waits, [20_000], "{}", throttled.stderr);
    assert_eq!(deferred.waits, [deferral], "{}", deferred.stderr);
}

#[test]
fn a_lock_held_past_the_deadline_reports_lock_contention() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    std::fs::create_dir_all(project.research_scratch()).expect("mkdir");
    let lock =
        std::fs::File::create(project.research_scratch().join("arxiv.lock"))
            .expect("create lock");
    flock(&lock, FlockOperation::NonBlockingLockExclusive).expect("lock");

    let run = fetch(&project, &server, &["arxiv", "search", "graphs"], &[]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(
        run.json(),
        serde_json::json!({
            "status": "unavailable",
            "source": "arxiv",
            "reason": "rate_limited",
            "cause": "lock_contention",
        })
    );
    assert!(run.stderr.contains("lock contention"), "{}", run.stderr);
    assert_eq!(server.hits(&query()), 0);
}

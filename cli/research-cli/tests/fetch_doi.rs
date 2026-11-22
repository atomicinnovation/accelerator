#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::process::Child;
use std::time::Duration;
use std::time::Instant;

use http_test_support::refused_base_url;
use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use research::sources::metadata::RECOMMENDED_KEYS;
use research::sources::service::CrossrefPool;
use research::sources::service::Service;
use support::arxiv_fixture;
use support::crossref_fixture;
use support::doi_binding;
use support::fetch;
use support::fetch_from;
use support::search_binding;
use support::spawn_in_real_time;
use support::web_fixture;
use support::LegacyFile;
use support::Project;
use support::Run;
use support::SeededTicket;
use support::CALL_BUDGET_MS;
use support::CLOCK_EPOCH_MS;

const PUBLIC: Service = Service::Crossref(CrossrefPool::Public);
const PROCEEDINGS: &str = "10.1145/371920.372066";
const PROCEEDINGS_OK: &str = "{\"status\":\"ok\",\"metadata\":{\"publisher\":\
                              \"ACM\"},\"venue\":\"Proceedings of the 10th \
                              international conference on World Wide Web\"}\n";
const EMPTY_OK: &str = "{\"status\":\"ok\",\"metadata\":{}}\n";
const CALL_BUDGET: u128 = 100_000;

fn works(doi: &str) -> RequestKey {
    RequestKey::get(&format!("/works/{doi}"))
}

fn record(name: &str) -> Route {
    Route::Json {
        status: 200,
        body: crossref_fixture(name),
    }
}

fn throttled(seconds: u64) -> Route {
    Route::Headers {
        status: 429,
        headers: vec![("Retry-After".to_owned(), seconds.to_string())],
        body: String::new(),
    }
}

fn serving(doi: &str, route: Route) -> MockHTTPServer {
    let server = MockHTTPServer::start();
    server.route(works(doi), route);
    server
}

fn proceedings_server() -> MockHTTPServer {
    serving(PROCEEDINGS, record("proceedings-371920-372066.json"))
}

fn doi_metadata(project: &Project, server: &MockHTTPServer, doi: &str) -> Run {
    fetch(project, server, &["doi", "metadata", doi], &[])
}

const fn presenting<'a>(doi: &'a str, ticket: &'a str) -> [&'a str; 5] {
    ["doi", "metadata", doi, "--ticket", ticket]
}

fn assert_printed(run: &Run, stdout: &str) {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, stdout);
}

fn assert_unavailable(run: &Run, reason: &str) {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(
        run.json(),
        serde_json::json!({
            "status": "unavailable",
            "source": "crossref",
            "reason": reason,
        })
    );
}

fn assert_within_call_budget(run: &Run, started: Instant) {
    let elapsed = started.elapsed().as_millis();
    let waited: u64 = run.waits.iter().sum();
    assert!(
        elapsed + u128::from(waited) <= CALL_BUDGET,
        "{elapsed} ms elapsed and {waited} ms waited"
    );
}

fn assert_usage_error(run: &Run, code: &str) {
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(run.stderr.starts_with(code), "{}", run.stderr);
}

/// The ticket of a call that printed `waiting`.
fn waiting(run: &Run) -> (String, u64) {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let document = run.json();
    assert_eq!(document["status"], "waiting", "{document}");
    assert_eq!(document["source"], "crossref", "{document}");
    (
        document["ticket"].as_str().expect("a ticket").to_owned(),
        document["position"].as_u64().expect("a position"),
    )
}

#[test]
fn a_proceedings_article_prints_its_venue_and_publisher_and_no_tier() {
    let project = Project::new();
    let server = proceedings_server();

    let run = doi_metadata(&project, &server, PROCEEDINGS);

    assert_printed(&run, PROCEEDINGS_OK);
    assert!(run.json().get("tier").is_none());
}

#[test]
fn a_doi_is_asked_of_crossref_once_with_no_query_and_no_credentials() {
    let project = Project::new();
    let server = proceedings_server();

    doi_metadata(&project, &server, PROCEEDINGS);

    let key = works(PROCEEDINGS);
    assert_eq!(server.queries(&key), [None]);
    for header in ["Authorization", "Crossref-Plus-API-Token"] {
        assert_eq!(server.last_header(&key, header), None, "{header}");
    }
    assert_eq!(
        server.last_header(&key, "User-Agent"),
        Some(format!(
            "accelerator-research/{}",
            env!("CARGO_PKG_VERSION")
        ))
    );
}

#[test]
fn every_form_of_a_doi_prints_the_same_output() {
    let project = Project::new();
    let server = proceedings_server();

    for form in [
        PROCEEDINGS,
        "https://doi.org/10.1145/371920.372066",
        "https://dx.doi.org/10.1145/371920.372066",
    ] {
        assert_printed(&doi_metadata(&project, &server, form), PROCEEDINGS_OK);
    }
}

#[test]
fn of_two_container_titles_the_first_is_the_venue() {
    let project = Project::new();
    let doi = "10.4213/tmf1151";
    let server = serving(doi, record("journal-two-container-titles.json"));

    let run = doi_metadata(&project, &server, doi);

    assert_printed(
        &run,
        "{\"status\":\"ok\",\"metadata\":{\"publisher\":\"Steklov \
         Mathematical Institute\"},\"venue\":\"Теоретическая и \
         математическая физика\"}\n",
    );
}

#[test]
fn a_book_chapter_prints_its_publisher_and_no_venue() {
    let project = Project::new();
    let doi = "10.1007/978-1-349-06671-1_23";
    let server = serving(doi, record("book-chapter.json"));

    let run = doi_metadata(&project, &server, doi);

    assert_printed(
        &run,
        "{\"status\":\"ok\",\"metadata\":{\"publisher\":\"Palgrave \
         Macmillan UK\"}}\n",
    );
}

#[test]
fn an_article_without_container_titles_prints_its_publisher_alone() {
    for fixture in [
        "publisher-no-container-title.json",
        "publisher-empty-container-title.json",
    ] {
        let project = Project::new();
        let server = serving(PROCEEDINGS, record(fixture));

        let run = doi_metadata(&project, &server, PROCEEDINGS);

        assert_printed(
            &run,
            "{\"status\":\"ok\",\"metadata\":{\"publisher\":\"ACM\"}}\n",
        );
    }
}

#[test]
fn a_record_with_neither_title_nor_publisher_prints_empty_metadata() {
    let project = Project::new();
    let server = serving(PROCEEDINGS, record("bare.json"));

    assert_printed(&doi_metadata(&project, &server, PROCEEDINGS), EMPTY_OK);
}

#[test]
fn a_doi_crossref_holds_no_record_for_prints_empty_metadata() {
    let project = Project::new();
    let doi = "10.48550/arXiv.1706.03762";
    let server = serving(doi, Route::Status(404));

    assert_printed(&doi_metadata(&project, &server, doi), EMPTY_OK);
}

#[test]
fn an_unreachable_crossref_is_unavailable_within_the_call_budget() {
    let project = Project::new();
    let server = MockHTTPServer::start();
    let refused = refused_base_url();
    let started = Instant::now();

    let run = fetch(
        &project,
        &server,
        &["doi", "metadata", PROCEEDINGS],
        &[("ACCELERATOR_CROSSREF_API_URL", &refused)],
    );

    assert_within_call_budget(&run, started);
    assert_unavailable(&run, "upstream_error");
}

#[test]
fn a_bad_request_is_unavailable_after_one_request() {
    let project = Project::new();
    let server = serving(PROCEEDINGS, Route::Status(400));

    let run = doi_metadata(&project, &server, PROCEEDINGS);

    assert_unavailable(&run, "upstream_error");
    assert_eq!(server.hits(&works(PROCEEDINGS)), 1);
    assert_eq!(
        run.stderr,
        "research fetch: doi metadata unavailable (upstream_error) after 1 \
         attempt\n"
    );
}

#[test]
fn retries_that_run_out_report_the_last_attempts_reason_within_the_budget() {
    for (attempts, reason) in [
        (vec![Route::Status(503)], "upstream_error"),
        (vec![Route::Status(429)], "rate_limited"),
        (
            vec![
                Route::Status(503),
                Route::Status(503),
                Route::Status(503),
                Route::Status(429),
            ],
            "rate_limited",
        ),
        (
            vec![
                Route::Status(429),
                Route::Status(429),
                Route::Status(429),
                Route::Status(503),
            ],
            "upstream_error",
        ),
    ] {
        let project = Project::new();
        let server = serving(PROCEEDINGS, Route::Sequence(attempts));
        let started = Instant::now();

        let run = doi_metadata(&project, &server, PROCEEDINGS);

        assert_within_call_budget(&run, started);
        assert_unavailable(&run, reason);
        assert_eq!(server.hits(&works(PROCEEDINGS)), 4, "{reason}");
    }
}

fn eventually(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(started.elapsed() < Duration::from_secs(30), "never {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn finish(child: Child) -> serde_json::Value {
    let output = child.wait_with_output().expect("wait");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("a JSON document")
}

#[test]
fn a_retry_after_holds_back_every_caller_in_the_project() {
    const ANSWERED_AFTER: Duration = Duration::from_secs(1);
    const RETRY_AFTER: Duration = Duration::from_secs(5);
    let project = Project::new();
    let server = serving(
        PROCEEDINGS,
        Route::Sequence(vec![
            Route::Delayed {
                delay: ANSWERED_AFTER,
                route: Box::new(throttled(RETRY_AFTER.as_secs())),
            },
            record("proceedings-371920-372066.json"),
        ]),
    );
    let key = works(PROCEEDINGS);

    let a = spawn_in_real_time(
        &project,
        &server,
        &["doi", "metadata", PROCEEDINGS],
    );
    eventually("sent A's request", || server.hits(&key) == 1);
    let b = spawn_in_real_time(
        &project,
        &server,
        &["doi", "metadata", PROCEEDINGS],
    );
    eventually("queued B", || project.service_tickets(PUBLIC).len() == 2);
    assert_eq!(finish(a)["status"], "ok");
    assert_eq!(finish(b)["status"], "ok");

    let hits = server.hit_instants(&key);
    assert_eq!(hits.len(), 3);
    let throttled_at = hits[0] + ANSWERED_AFTER;
    for (caller, at) in [("A's retry", hits[1]), ("B's first", hits[2])] {
        let after = at.saturating_duration_since(throttled_at);
        assert!(
            at >= throttled_at + RETRY_AFTER,
            "{caller} arrived {after:?} after the 429"
        );
        assert!(
            at < throttled_at + RETRY_AFTER + Duration::from_secs(1),
            "{caller} arrived {after:?} after the 429"
        );
    }
}

#[test]
fn an_arxiv_deferral_never_holds_back_a_crossref_call() {
    let project = Project::new();
    let server = proceedings_server();
    project.defer_service(Service::Arxiv, 60);

    let run = doi_metadata(&project, &server, PROCEEDINGS);

    assert_printed(&run, PROCEEDINGS_OK);
    assert_eq!(run.waits, Vec::<u64>::new());
}

#[test]
fn a_crossref_deferral_never_holds_back_an_arxiv_call() {
    let project = Project::new();
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get("/api/query"),
        Route::Bytes {
            status: 200,
            body: arxiv_fixture("empty.xml").into_bytes(),
        },
    );
    project.defer_service(PUBLIC, 60);

    let run = fetch(&project, &server, &["arxiv", "lookup", "2608.21129"], &[]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.json()["status"], "ok");
    assert_eq!(run.waits, Vec::<u64>::new());
}

#[test]
fn a_call_behind_a_live_ticket_waits_within_the_budget_then_resumes() {
    let project = Project::new();
    let server = proceedings_server();
    project.seed_service_ticket(
        PUBLIC,
        "1-aaaaaa",
        &SeededTicket::live(&doi_binding("10.1145/3448016.3457556"), 10),
    );
    let ahead = project.hold_service_ticket(PUBLIC, "1-aaaaaa");
    let started = Instant::now();

    let run = doi_metadata(&project, &server, PROCEEDINGS);

    assert_within_call_budget(&run, started);
    let (ticket, position) = waiting(&run);
    assert!(position >= 1, "{position}");
    assert_eq!(server.hits(&works(PROCEEDINGS)), 0);
    assert_eq!(
        run.stderr,
        format!(
            "research fetch: doi metadata waiting at position {position}; \
             re-present with --ticket {ticket}\n"
        )
    );

    drop(ahead);
    project.clear_clock_log();
    let resumed =
        fetch(&project, &server, &presenting(PROCEEDINGS, &ticket), &[]);

    assert_printed(&resumed, PROCEEDINGS_OK);
}

#[test]
fn a_held_arxiv_lock_from_an_older_release_never_holds_back_a_doi_call() {
    let project = Project::new();
    let server = proceedings_server();
    let _older = project.hold_legacy(LegacyFile::Lock);

    assert_printed(
        &doi_metadata(&project, &server, PROCEEDINGS),
        PROCEEDINGS_OK,
    );
}

#[test]
fn a_crossref_ticket_presented_to_arxiv_is_a_mismatch() {
    let project = Project::new();
    let server = MockHTTPServer::start();
    project.seed_service_ticket(
        PUBLIC,
        "1-aaaaaa",
        &SeededTicket::absent(&doi_binding(PROCEEDINGS), 20, 10),
    );

    let run = fetch(
        &project,
        &server,
        &["arxiv", "search", "graphs", "--ticket", "1-aaaaaa"],
        &[],
    );

    assert_usage_error(&run, "E_ARXIV_TICKET_MISMATCH");
    assert_eq!(server.hits(&RequestKey::get("/api/query")), 0);
}

#[test]
fn an_arxiv_ticket_presented_to_doi_is_a_mismatch() {
    let project = Project::new();
    let server = proceedings_server();
    project.seed_ticket(
        "1-aaaaaa",
        &SeededTicket::absent(&search_binding("graphs", 10), 20, 10),
    );

    let run =
        fetch(&project, &server, &presenting(PROCEEDINGS, "1-aaaaaa"), &[]);

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "E_DOI_TICKET_MISMATCH: ticket 1-aaaaaa was issued by fetch arxiv; \
         drop --ticket and re-run the call\n"
    );
    assert_eq!(server.hits(&works(PROCEEDINGS)), 0);
}

#[test]
fn a_doi_ticket_presented_with_another_doi_is_a_mismatch() {
    let project = Project::new();
    let server = proceedings_server();
    project.seed_service_ticket(
        PUBLIC,
        "1-aaaaaa",
        &SeededTicket::absent(&doi_binding("10.1145/3448016.3457556"), 20, 10),
    );

    let run =
        fetch(&project, &server, &presenting(PROCEEDINGS, "1-aaaaaa"), &[]);

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "E_DOI_TICKET_MISMATCH: ticket 1-aaaaaa was issued for metadata \
         10.1145/3448016.3457556 (differing: doi); re-present it with that \
         call, or drop --ticket\n"
    );
    assert_eq!(server.hits(&works(PROCEEDINGS)), 0);
}

#[test]
fn a_malformed_ticket_is_a_usage_error_before_any_request() {
    let project = Project::new();
    let server = proceedings_server();

    let run = fetch(&project, &server, &presenting(PROCEEDINGS, "42"), &[]);

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "E_DOI_TICKET_MALFORMED: expected a ticket such as 42-9f1c2a (got \
         '42')\n"
    );
    assert_eq!(server.hits(&works(PROCEEDINGS)), 0);
}

#[test]
fn a_ticket_another_call_is_presenting_is_a_usage_error() {
    let project = Project::new();
    let server = proceedings_server();
    project.seed_service_ticket(
        PUBLIC,
        "4-dddddd",
        &SeededTicket::live(&doi_binding(PROCEEDINGS), 20),
    );
    let _elsewhere = project.hold_service_ticket(PUBLIC, "4-dddddd");

    let run =
        fetch(&project, &server, &presenting(PROCEEDINGS, "4-dddddd"), &[]);

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "E_DOI_TICKET_LIVE: ticket 4-dddddd is already being presented by \
         another call; use that call's output, or re-present it once that \
         call has returned\n"
    );
    assert_eq!(server.hits(&works(PROCEEDINGS)), 0);
}

fn past_its_cap(last_retryable: Option<&str>) -> Run {
    let project = Project::new();
    let server = proceedings_server();
    project.seed_service_ticket(
        PUBLIC,
        "1-aaaaaa",
        &SeededTicket {
            last_retryable,
            ..SeededTicket::absent(&doi_binding(PROCEEDINGS), 901, 10)
        },
    );

    let run =
        fetch(&project, &server, &presenting(PROCEEDINGS, "1-aaaaaa"), &[]);

    assert_eq!(server.hits(&works(PROCEEDINGS)), 0);
    run
}

#[test]
fn a_ticket_past_its_cap_with_no_upstream_failure_is_lock_contention() {
    let run = past_its_cap(None);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(
        run.json(),
        serde_json::json!({
            "status": "unavailable",
            "source": "crossref",
            "reason": "rate_limited",
            "cause": "lock_contention",
        })
    );
}

#[test]
fn a_ticket_past_its_cap_reports_its_last_upstream_failure() {
    for reason in ["upstream_error", "rate_limited"] {
        assert_unavailable(&past_its_cap(Some(reason)), reason);
    }
}

#[test]
fn a_call_still_in_backoff_waits_and_its_expired_ticket_reports_the_backoff() {
    let project = Project::new();
    let server = serving(
        PROCEEDINGS,
        Route::Headers {
            status: 503,
            headers: vec![("Retry-After".to_owned(), "30".to_owned())],
            body: String::new(),
        },
    );

    let first = doi_metadata(&project, &server, PROCEEDINGS);
    let (ticket, _) = waiting(&first);
    let re_present_after = |seconds: u64| {
        let epoch =
            CLOCK_EPOCH_MS.parse::<u64>().expect("an epoch") + seconds * 1000;
        fetch(
            &project,
            &server,
            &presenting(PROCEEDINGS, &ticket),
            &[("ACCELERATOR_RESEARCH_TEST_CLOCK_EPOCH", &epoch.to_string())],
        )
    };
    for seconds in [250, 500, 750] {
        assert_eq!(waiting(&re_present_after(seconds)).0, ticket);
    }

    assert_unavailable(&re_present_after(901), "upstream_error");
}

#[test]
fn a_malformed_doi_is_a_usage_error() {
    let project = Project::new();
    let server = MockHTTPServer::start();

    let run = doi_metadata(&project, &server, "not-a-doi");

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "E_DOI_MALFORMED: expected a DOI such as 10.1145/371920.372066 or \
         https://doi.org/10.1145/371920.372066 (got 'not-a-doi')\n"
    );
}

#[test]
fn an_unreadable_record_is_unavailable_and_reported_once() {
    let project = Project::new();
    let server = serving(
        PROCEEDINGS,
        Route::Bytes {
            status: 200,
            body: b"<html>not JSON</html>".to_vec(),
        },
    );

    let run = doi_metadata(&project, &server, PROCEEDINGS);

    assert_unavailable(&run, "upstream_error");
    let reported = run
        .stderr
        .lines()
        .filter(|line| line.contains(PROCEEDINGS))
        .count();
    assert_eq!(reported, 1, "{}", run.stderr);
}

#[test]
fn a_ticket_re_presented_in_another_letter_case_resumes() {
    let project = Project::new();
    let server = serving("10.48550/ARXIV.1706.03762", Route::Status(404));

    let first = fetch(
        &project,
        &server,
        &["doi", "metadata", "10.48550/arXiv.1706.03762"],
        &[(CALL_BUDGET_MS, "30100")],
    );
    let (ticket, _) = waiting(&first);
    let resumed = fetch(
        &project,
        &server,
        &presenting("10.48550/ARXIV.1706.03762", &ticket),
        &[],
    );

    assert_printed(&resumed, EMPTY_OK);
    assert_eq!(project.service_tickets(PUBLIC), Vec::<String>::new());
}

#[test]
fn every_key_doi_metadata_prints_follows_every_key_web_metadata_prints() {
    let page = MockHTTPServer::start();
    page.route(
        RequestKey::get("/article"),
        Route::Headers {
            status: 200,
            headers: vec![(
                "Content-Type".to_owned(),
                "text/html; charset=utf-8".to_owned(),
            )],
            body: web_fixture("every-key.html"),
        },
    );
    let outside = tempfile::tempdir().expect("tempdir");
    let web = fetch_from(
        outside.path(),
        &["web", "metadata", "http://fixture.accelerator.test/article"],
        &[(
            "ACCELERATOR_RESEARCH_TEST_RESOLVE",
            &format!("fixture.accelerator.test={}", page.address()),
        )],
    );
    let doi = doi_metadata(&Project::new(), &proceedings_server(), PROCEEDINGS);

    let ranks = |run: &Run| -> Vec<usize> {
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        run.json()["metadata"]
            .as_object()
            .expect("a metadata map")
            .keys()
            .map(|key| {
                RECOMMENDED_KEYS
                    .iter()
                    .position(|recommended| recommended == key)
                    .unwrap_or_else(|| panic!("{key} is not recommended"))
            })
            .collect()
    };
    let web_ranks = ranks(&web);
    let doi_ranks = ranks(&doi);
    assert_eq!(web_ranks.len(), 4, "{}", web.stdout);
    assert!(!doi_ranks.is_empty(), "{}", doi.stdout);
    assert!(
        web_ranks.iter().max() < doi_ranks.iter().min(),
        "web printed {} and doi printed {}",
        web.stdout,
        doi.stdout
    );
}

/// Starts `calls` DOI calls behind a barrier holding every serving slot,
/// releases it once all have joined, and returns the instants their
/// requests arrived.
fn hits_released_together(
    project: &Project,
    server: &MockHTTPServer,
    calls: usize,
) -> Vec<Instant> {
    let barrier = project.hold_barrier(PUBLIC);
    let barrier_tickets = project.service_tickets(PUBLIC).len();
    let children = (0..calls)
        .map(|_| {
            spawn_in_real_time(
                project,
                server,
                &["doi", "metadata", PROCEEDINGS],
            )
        })
        .collect::<Vec<_>>();
    eventually("queued every call", || {
        project.service_tickets(PUBLIC).len() == barrier_tickets + calls
    });
    drop(barrier);
    for child in children {
        assert_eq!(finish(child)["status"], "ok");
    }
    server.hit_instants(&works(PROCEEDINGS))
}

fn answered_after(delay: Duration) -> MockHTTPServer {
    serving(
        PROCEEDINGS,
        Route::Delayed {
            delay,
            route: Box::new(record("proceedings-371920-372066.json")),
        },
    )
}

#[test]
fn public_pool_calls_start_no_more_than_five_a_second_and_all_within_two() {
    const INTERVAL: Duration = Duration::from_millis(210);
    let project = Project::new();
    let server = answered_after(Duration::from_millis(50));

    let hits = hits_released_together(&project, &server, 10);

    assert_eq!(hits.len(), 10);
    for (index, start) in hits.iter().enumerate() {
        let in_window = hits[index..]
            .iter()
            .take_while(|later| **later - *start < Duration::from_secs(1))
            .count();
        assert!(
            in_window <= 5,
            "{in_window} starts in a second from {index}"
        );
    }
    let spread = hits[9] - hits[0];
    let largest_gap = hits
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .max()
        .expect("gaps");
    assert!(
        spread < Duration::from_secs(2),
        "the 10th started {spread:?} after the first: slack {:?}, largest \
         hand-off overhead {:?}",
        Duration::from_secs(2).saturating_sub(INTERVAL * 9),
        largest_gap.saturating_sub(INTERVAL)
    );
    assert_eq!(project.service_requests(PUBLIC).len(), 10);
}

#[test]
fn public_pool_calls_never_overlap_in_flight() {
    const ANSWERED_AFTER: Duration = Duration::from_secs(2);
    let project = Project::new();
    let server = answered_after(ANSWERED_AFTER);

    let hits = hits_released_together(&project, &server, 10);

    assert_eq!(hits.len(), 10);
    for pair in hits.windows(2) {
        assert!(
            pair[1] - pair[0] >= ANSWERED_AFTER,
            "a request started {:?} after the one before it",
            pair[1] - pair[0]
        );
    }
}

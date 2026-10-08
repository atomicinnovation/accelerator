#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use support::arxiv_fixture;
use support::assert_golden;
use support::fetch;
use support::search_binding;
use support::Project;
use support::Run;
use support::SeededTicket;
use support::CALL_BUDGET_MS;

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

fn server_with(routes: Vec<(RequestKey, Route)>) -> MockHTTPServer {
    let server = MockHTTPServer::start();
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
    let server = MockHTTPServer::start();

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
fn a_lock_held_past_the_window_waits_with_a_ticket() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    let _arxiv = project.hold("arxiv.lock");

    let run = fetch(&project, &server, &["arxiv", "search", "graphs"], &[]);

    let ticket = waiting(&run, 1);
    assert!(project.ticket_exists(&ticket));
    assert!(project.contention_lines().is_empty());
    assert_eq!(server.hits(&query()), 0);
}
const GRAPHS_QUERY: [&str; 3] = ["arxiv", "search", "graphs"];

fn graphs() -> String {
    search_binding("graphs", 10)
}

const fn budget(millis: &str) -> [(&str, &str); 1] {
    [(CALL_BUDGET_MS, millis)]
}

/// The ticket of a call that printed `waiting` at `position`.
fn waiting(run: &Run, position: u64) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let document = run.json();
    assert_eq!(document["status"], "waiting", "{document}");
    assert_eq!(document["source"], "arxiv", "{document}");
    assert_eq!(document["position"], position, "{document}");
    document["ticket"].as_str().expect("a ticket").to_owned()
}

fn presenting<'a>(args: &[&'a str], ticket: &'a str) -> Vec<&'a str> {
    let mut presented = args.to_vec();
    presented.extend(["--ticket", ticket]);
    presented
}

fn number_of(ticket: &str) -> u64 {
    ticket
        .split_once('-')
        .and_then(|(number, _)| number.parse().ok())
        .expect("a ticket number")
}

#[test]
fn a_waiting_document_carries_ticket_and_position() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);

    let run = fetch(&project, &server, &GRAPHS_QUERY, &budget("32000"));

    assert_eq!(run.stdout.trim().lines().count(), 1, "{}", run.stdout);
    let ticket = waiting(&run, 1);
    assert_eq!(number_of(&ticket), 1);
    assert_eq!(ticket.len(), "1-".len() + 6, "{ticket}");
    assert!(
        run.stderr.contains(&format!(
            "research fetch: arxiv search waiting at position 1; \
             re-present with --ticket {ticket}"
        )),
        "{}",
        run.stderr
    );
    assert_eq!(server.hits(&query()), 0);
}

#[test]
fn a_40_s_budget_behind_a_live_ticket_waits_at_position_2() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    let _arxiv = project.hold("arxiv.lock");
    project.seed_ticket("1-aaaaaa", &SeededTicket::live(&graphs(), 10));
    let _ahead = project.hold_ticket("1-aaaaaa");

    let run = fetch(&project, &server, &GRAPHS_QUERY, &budget("40000"));

    waiting(&run, 2);
}

#[test]
fn a_re_presented_ticket_is_served_and_leaves_the_queue() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    let first = fetch(&project, &server, &GRAPHS_QUERY, &budget("32000"));
    let ticket = waiting(&first, 1);

    let again =
        fetch(&project, &server, &presenting(&GRAPHS_QUERY, &ticket), &[]);

    assert_eq!(records(&again).len(), 3);
    let names: Vec<_> = project
        .queue_files()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["queue.lock"]);
}

#[test]
fn a_re_presentation_after_mid_call_exhaustion_repeats_the_search_and_reuses_cached_confirmations(
) {
    let project = Project::new();
    let server = server_with(vec![
        (query(), feed("search-2-withdrawal-candidates.xml")),
        (
            oai(),
            Route::Sequence(vec![
                feed("oai-2608.21129.xml"),
                feed("oai-2211.12792.xml"),
            ]),
        ),
    ]);
    let first = fetch(&project, &server, &GRAPHS_QUERY, &budget("35000"));
    let ticket = waiting(&first, 1);
    assert_eq!(server.hits(&oai()), 1);
    project.seed_ticket("9-bbbbbb", &SeededTicket::live(&graphs(), 0));
    let _later = project.hold_ticket("9-bbbbbb");

    let again =
        fetch(&project, &server, &presenting(&GRAPHS_QUERY, &ticket), &[]);

    let records = records(&again);
    assert_eq!(records[0]["withdrawn"], true);
    assert_eq!(records[1]["withdrawn"], false);
    assert_eq!(server.hits(&query()), 2);
    let confirmed: Vec<_> = server
        .queries(&oai())
        .into_iter()
        .map(|query| query.expect("a query"))
        .collect();
    assert_eq!(confirmed.len(), 2, "{confirmed:?}");
    assert_eq!(
        confirmed
            .iter()
            .filter(|query| query.contains("2608.21129"))
            .count(),
        1,
        "{confirmed:?}"
    );
}

#[test]
fn a_ticket_with_another_query_exits_2_naming_it_and_changes_nothing() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    let attention = search_binding("attention heads", 10);
    project.seed_ticket("5-eeeeee", &SeededTicket::absent(&attention, 20, 10));
    project.seed_ticket(
        "3-cccccc",
        &SeededTicket {
            ended_ago: None,
            ..SeededTicket::absent(&graphs(), 600, 499)
        },
    );
    project.seed_ticket("2-bbbbbb", &SeededTicket::absent(&graphs(), 600, 400));
    drop(project.hold("arxiv-queue/queue.lock"));
    let before = project.queue_files();

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "5-eeeeee"),
        &[],
    );

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(
        run.stderr.contains("E_ARXIV_TICKET_MISMATCH"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains(
            "issued for search 'attention heads' --limit 10 (differing: query)"
        ),
        "{}",
        run.stderr
    );
    assert_eq!(run.stdout, "");
    assert_eq!(server.hits(&query()), 0);
    assert_eq!(project.queue_files(), before);
}

fn mismatch_line(issued: &str, presented: &[&str]) -> String {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket("5-eeeeee", &SeededTicket::absent(issued, 20, 10));

    let run = fetch(&project, &server, &presenting(presented, "5-eeeeee"), &[]);

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    run.stderr
        .lines()
        .find(|line| line.starts_with("E_ARXIV_TICKET_MISMATCH"))
        .unwrap_or_else(|| panic!("no mismatch line in {}", run.stderr))
        .to_owned()
}

#[test]
fn a_ticket_with_another_limit_names_the_issued_call_and_its_limit() {
    let line = mismatch_line(
        &graphs(),
        &["arxiv", "search", "graphs", "--limit", "5"],
    );

    assert!(
        line.contains(
            "issued for search 'graphs' --limit 10 (differing: --limit)"
        ),
        "{line}"
    );
}

#[test]
fn a_search_ticket_presented_for_a_lookup_names_the_whole_issued_search() {
    let line = mismatch_line(&graphs(), &["arxiv", "lookup", "2608.21129"]);

    assert!(
        line.contains(
            "issued for search 'graphs' --limit 10 (differing: verb)"
        ),
        "{line}"
    );
}

#[test]
fn a_ticket_with_another_query_and_limit_names_both() {
    let line = mismatch_line(
        &search_binding("attention heads", 10),
        &["arxiv", "search", "graphs", "--limit", "5"],
    );

    assert!(
        line.contains(
            "issued for search 'attention heads' --limit 10 \
             (differing: query, --limit)"
        ),
        "{line}"
    );
}

#[test]
fn an_expired_ticket_with_another_query_rejoins_with_a_fresh_ticket() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket(
        "3-cccccc",
        &SeededTicket::absent(&search_binding("attention heads", 10), 400, 301),
    );

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "3-cccccc"),
        &budget("32000"),
    );

    let ticket = waiting(&run, 1);
    assert_eq!(number_of(&ticket), 4);
    assert!(!project.ticket_exists("3-cccccc"));
}

#[test]
fn a_ticket_re_presented_300_s_after_its_end_keeps_its_number() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket("1-aaaaaa", &SeededTicket::absent(&graphs(), 400, 300));
    project.seed_ticket("3-cccccc", &SeededTicket::live(&graphs(), 5));
    let _later = project.hold_ticket("3-cccccc");

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "1-aaaaaa"),
        &budget("32000"),
    );

    assert_eq!(waiting(&run, 1), "1-aaaaaa");
}

#[test]
fn at_301_s_it_rejoins_with_a_fresh_ticket() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket("1-aaaaaa", &SeededTicket::absent(&graphs(), 400, 301));
    project.seed_ticket("3-cccccc", &SeededTicket::live(&graphs(), 5));
    let _later = project.hold_ticket("3-cccccc");

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "1-aaaaaa"),
        &budget("32000"),
    );

    let ticket = waiting(&run, 2);
    assert_ne!(ticket, "1-aaaaaa");
    assert!(number_of(&ticket) > 3, "{ticket}");
}

#[test]
fn a_ticket_re_presented_900_s_after_issue_is_queued() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket("1-aaaaaa", &SeededTicket::absent(&graphs(), 900, 10));

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "1-aaaaaa"),
        &budget("32000"),
    );

    assert_eq!(waiting(&run, 1), "1-aaaaaa");
}

#[test]
fn at_901_s_it_is_lock_contention_with_one_log_entry() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket("1-aaaaaa", &SeededTicket::absent(&graphs(), 901, 10));

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "1-aaaaaa"),
        &[],
    );

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
    assert_eq!(server.hits(&query()), 0);
    let lines = project.contention_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].ends_with(" ticket_past_cap 1-aaaaaa"), "{lines:?}");
}

#[test]
fn a_ticket_issued_950_s_ago_and_ended_400_s_ago_rejoins_afresh() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket("1-aaaaaa", &SeededTicket::absent(&graphs(), 950, 400));

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "1-aaaaaa"),
        &budget("32000"),
    );

    assert_ne!(waiting(&run, 1), "1-aaaaaa");
    assert!(project.contention_lines().is_empty());
}

#[test]
fn a_position_counts_expired_absent_and_live_tickets_correctly() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket("1-aaaaaa", &SeededTicket::absent(&graphs(), 600, 400));
    project.seed_ticket("2-bbbbbb", &SeededTicket::absent(&graphs(), 200, 100));
    project.seed_ticket("3-cccccc", &SeededTicket::live(&graphs(), 50));
    let _live = project.hold_ticket("3-cccccc");
    let _arxiv = project.hold("arxiv.lock");

    let run = fetch(&project, &server, &GRAPHS_QUERY, &budget("33100"));

    let ticket = waiting(&run, 3);
    assert_eq!(number_of(&ticket), 4);
}

#[test]
fn a_malformed_ticket_is_a_usage_error() {
    let server = MockHTTPServer::start();

    let run = fetch(
        &Project::new(),
        &server,
        &presenting(&GRAPHS_QUERY, "42"),
        &[],
    );

    assert_eq!(run.code, Some(2));
    assert!(
        run.stderr.starts_with("E_ARXIV_TICKET_MALFORMED"),
        "{}",
        run.stderr
    );
    assert_eq!(server.hits(&query()), 0);
}

#[test]
fn a_repeated_ticket_flag_is_a_usage_error() {
    let server = MockHTTPServer::start();
    let mut args = presenting(&GRAPHS_QUERY, "1-aaaaaa");
    args.extend(["--ticket", "2-bbbbbb"]);

    let run = fetch(&Project::new(), &server, &args, &[]);

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(server.hits(&query()), 0);
}

#[test]
fn a_ticket_presented_while_live_elsewhere_exits_2() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket("4-dddddd", &SeededTicket::live(&graphs(), 20));
    let _elsewhere = project.hold_ticket("4-dddddd");
    let before = project.queue_files();

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "4-dddddd"),
        &[],
    );

    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(
        run.stderr.contains(
            "E_ARXIV_TICKET_LIVE: ticket 4-dddddd is already being presented \
             by another call; use that call's output, or re-present it once \
             that call has returned"
        ),
        "{}",
        run.stderr
    );
    assert_eq!(server.hits(&query()), 0);
    let after: Vec<_> = project
        .queue_files()
        .into_iter()
        .filter(|(name, _)| name != "queue.lock")
        .collect();
    let before: Vec<_> = before
        .into_iter()
        .filter(|(name, _)| name != "queue.lock")
        .collect();
    assert_eq!(after, before);
}

#[test]
fn an_unusable_queue_ends_a_contended_call_in_lock_contention() {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    std::fs::create_dir_all(project.queue_dir().join("queue.lock"))
        .expect("an unusable queue lock");
    let _arxiv = project.hold("arxiv.lock");

    let run = fetch(&project, &server, &GRAPHS_QUERY, &budget("33100"));

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.json()["cause"], "lock_contention");
    assert_eq!(run.json()["reason"], "rate_limited");
    assert_eq!(server.hits(&query()), 0);
    let lines = project.contention_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].ends_with(" queue_unusable"), "{lines:?}");
}

#[test]
fn a_ticket_over_the_cap_after_an_upstream_failure_is_unavailable_with_that_reason(
) {
    let project = Project::new();
    let server = server_with(vec![(query(), feed("search-3.xml"))]);
    project.seed_ticket(
        "1-aaaaaa",
        &SeededTicket {
            last_retryable: Some("upstream_error"),
            ..SeededTicket::absent(&graphs(), 901, 10)
        },
    );

    let run = fetch(
        &project,
        &server,
        &presenting(&GRAPHS_QUERY, "1-aaaaaa"),
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
    assert_eq!(server.hits(&query()), 0);
    assert!(project.contention_lines().is_empty());
}

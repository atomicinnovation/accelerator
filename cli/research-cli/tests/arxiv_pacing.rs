//! arXiv pacing across real processes on the real clock: every wait here
//! really passes.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::process::Child;
use std::time::Duration;
use std::time::Instant;

use http_test_support::MockServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use support::arxiv_fixture;
use support::spawn_in_real_time;
use support::Project;

const SPACING: Duration = Duration::from_secs(3);
const STALL: Duration = Duration::from_secs(5);
const SEARCH: [&str; 3] = ["arxiv", "search", "graphs"];

fn query() -> RequestKey {
    RequestKey::get("/api/query")
}

fn feed(name: &str) -> Route {
    Route::Bytes {
        status: 200,
        body: arxiv_fixture(name).into_bytes(),
    }
}

fn finish(child: Child) {
    let output = child.wait_with_output().expect("wait");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn wait_for_hits(server: &MockServer, key: &RequestKey, hits: usize) {
    let started = Instant::now();
    while server.hits(key) < hits {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "no hit {hits} on {key:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_second_process_waits_for_the_first_processs_response_to_finish() {
    let project = Project::new();
    let server = MockServer::start();
    server.route(
        query(),
        Route::Sequence(vec![Route::Stall(STALL), feed("search-3.xml")]),
    );

    let first = spawn_in_real_time(&project, &server, &SEARCH);
    wait_for_hits(&server, &query(), 1);
    let second = spawn_in_real_time(&project, &server, &SEARCH);
    wait_for_hits(&server, &query(), 2);
    finish(second);
    finish(first);

    let hits = server.hit_instants(&query());
    assert!(
        hits[1] - hits[0] >= STALL,
        "the second request left {:?} after the first",
        hits[1] - hits[0]
    );
}

#[test]
fn sequential_calls_are_spaced_three_seconds_apart() {
    let project = Project::new();
    let server = MockServer::start();
    server.route(query(), feed("search-3.xml"));

    finish(spawn_in_real_time(&project, &server, &SEARCH));
    finish(spawn_in_real_time(&project, &server, &SEARCH));

    let hits = server.hit_instants(&query());
    assert!(hits[1] - hits[0] >= SPACING, "{:?}", hits[1] - hits[0]);
}

#[test]
fn a_withdrawal_confirmation_is_spaced_three_seconds_after_its_search() {
    let project = Project::new();
    let server = MockServer::start();
    server.route(query(), feed("lookup-2608.21129.xml"));
    server.route(RequestKey::get("/oai"), feed("oai-2608.21129.xml"));

    finish(spawn_in_real_time(&project, &server, &SEARCH));

    let search = server.hit_instants(&query());
    let confirmation = server.hit_instants(&RequestKey::get("/oai"));
    assert_eq!(confirmation.len(), 1);
    assert!(
        confirmation[0] - search[0] >= SPACING,
        "{:?}",
        confirmation[0] - search[0]
    );
}

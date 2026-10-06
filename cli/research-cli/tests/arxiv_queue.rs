//! The arXiv queue across real processes on the real clock: who is served
//! first once the lock frees, and what a killed waiter leaves behind.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::fs::File;
use std::process::Child;
use std::time::Duration;
use std::time::Instant;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use support::arxiv_fixture;
use support::search_binding;
use support::spawn_in_real_time;
use support::Project;
use support::SeededTicket;

const SPACING: Duration = Duration::from_secs(3);

fn query() -> RequestKey {
    RequestKey::get("/api/query")
}

fn server() -> MockHTTPServer {
    let server = MockHTTPServer::start();
    server.route(
        query(),
        Route::Bytes {
            status: 200,
            body: arxiv_fixture("search-3.xml").into_bytes(),
        },
    );
    server
}

fn search(project: &Project, server: &MockHTTPServer, word: &str) -> Child {
    spawn_in_real_time(project, server, &["arxiv", "search", word])
}

fn re_present(
    project: &Project,
    server: &MockHTTPServer,
    word: &str,
    ticket: &str,
) -> Child {
    spawn_in_real_time(
        project,
        server,
        &["arxiv", "search", word, "--ticket", ticket],
    )
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

fn eventually(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(started.elapsed() < Duration::from_secs(30), "never {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Waits for one more ticket than `known` to appear, and returns it.
fn next_ticket(project: &Project, known: &[String]) -> String {
    eventually("issued a ticket", || {
        project
            .tickets()
            .iter()
            .any(|ticket| !known.contains(ticket))
    });
    project
        .tickets()
        .into_iter()
        .find(|ticket| !known.contains(ticket))
        .expect("a new ticket")
}

/// The search words in the order their queries reached the server.
fn served_order(server: &MockHTTPServer) -> Vec<String> {
    server
        .queries(&query())
        .into_iter()
        .map(|query| {
            let query = query.expect("a query");
            ["alpha", "beta", "gamma"]
                .into_iter()
                .find(|word| query.contains(&format!("all:{word}")))
                .unwrap_or_else(|| panic!("an unexpected query {query}"))
                .to_owned()
        })
        .collect()
}

fn hold_arxiv(project: &Project) -> File {
    project.hold("arxiv.lock")
}

#[test]
fn live_tickets_are_served_in_issue_order() {
    let project = Project::new();
    let server = server();
    let arxiv = hold_arxiv(&project);
    let mut known = Vec::new();
    let mut waiters = Vec::new();
    for word in ["alpha", "beta", "gamma"] {
        waiters.push(search(&project, &server, word));
        known.push(next_ticket(&project, &known));
    }

    drop(arxiv);
    for waiter in waiters {
        assert_eq!(finish(waiter)["status"], "ok");
    }

    assert_eq!(served_order(&server), ["alpha", "beta", "gamma"]);
}

#[test]
fn an_absent_ticket_is_skipped_then_served_before_later_tickets() {
    let project = Project::new();
    let server = server();
    project.seed_ticket_now(
        "1-aaaaaa",
        &SeededTicket::absent(&search_binding("alpha", 10), 20, 10),
    );

    assert_eq!(finish(search(&project, &server, "beta"))["status"], "ok");
    let arxiv = hold_arxiv(&project);
    let later = search(&project, &server, "gamma");
    next_ticket(&project, &["1-aaaaaa".to_owned()]);
    let resumed = re_present(&project, &server, "alpha", "1-aaaaaa");
    eventually("resumed the absent ticket", || {
        project.ticket_is_held("1-aaaaaa")
    });
    drop(arxiv);

    assert_eq!(finish(resumed)["status"], "ok");
    assert_eq!(finish(later)["status"], "ok");
    assert_eq!(served_order(&server), ["beta", "alpha", "gamma"]);
}

#[test]
fn a_killed_waiter_frees_its_place_at_once() {
    let project = Project::new();
    let server = server();
    let arxiv = hold_arxiv(&project);
    let mut killed = search(&project, &server, "alpha");
    let killed_ticket = next_ticket(&project, &[]);
    let next = search(&project, &server, "beta");
    next_ticket(&project, std::slice::from_ref(&killed_ticket));
    killed.kill().expect("kill");
    killed.wait().expect("reap");

    let released = Instant::now();
    drop(arxiv);
    assert_eq!(finish(next)["status"], "ok");

    let hits = server.hit_instants(&query());
    assert!(
        hits[0] - released < SPACING,
        "the next waiter was served {:?} after the release",
        hits[0] - released
    );

    let arxiv = hold_arxiv(&project);
    let later = search(&project, &server, "gamma");
    next_ticket(&project, std::slice::from_ref(&killed_ticket));
    let resumed = re_present(&project, &server, "alpha", &killed_ticket);
    eventually("resumed the killed ticket", || {
        project.ticket_is_held(&killed_ticket)
    });
    drop(arxiv);

    assert_eq!(finish(resumed)["status"], "ok");
    assert_eq!(finish(later)["status"], "ok");
    assert_eq!(served_order(&server), ["beta", "alpha", "gamma"]);
}

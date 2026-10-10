//! Thirty arXiv fetches through the queue at once, on the real clock, each
//! re-presented until it settles, as researchers do.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;
use std::time::Instant;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use support::arxiv_fixture;
use support::spawn_in_real_time_with;
use support::Project;
use support::CALL_BUDGET_MS;

const CALLS: u32 = 30;
const RESPONSE: Duration = Duration::from_secs(1);
const SPACING: Duration = Duration::from_secs(3);

fn query() -> RequestKey {
    RequestKey::get("/api/query")
}

/// The final document of one fetch request, and how many times it waited.
fn settled(
    project: &Project,
    server: &MockHTTPServer,
) -> (serde_json::Value, u32) {
    let mut ticket: Option<String> = None;
    let mut waited = 0;
    loop {
        let mut args = vec!["arxiv", "search", "graphs"];
        if let Some(ticket) = &ticket {
            args.extend(["--ticket", ticket]);
        }
        let output = spawn_in_real_time_with(
            project,
            server,
            &args,
            &[(CALL_BUDGET_MS, "40000")],
        )
        .wait_with_output()
        .expect("wait");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let document: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("a JSON document");
        if document["status"] != "waiting" {
            return (document, waited);
        }
        waited += 1;
        ticket =
            Some(document["ticket"].as_str().expect("a ticket").to_owned());
    }
}

#[test]
#[ignore = "real time, minutes: run with mise run test:integration:arxiv-queue-stress"]
fn thirty_requests_re_presenting_until_settled_all_succeed() {
    let project = Project::new();
    let server = MockHTTPServer::start();
    server.route(
        query(),
        Route::Delayed {
            delay: RESPONSE,
            route: Box::new(Route::Bytes {
                status: 200,
                body: arxiv_fixture("search-3.xml").into_bytes(),
            }),
        },
    );
    let started = Instant::now();

    let (finished, results) = std::sync::mpsc::channel();
    std::thread::scope(|drivers| {
        for _ in 0..CALLS {
            let finished = finished.clone();
            let (project, server) = (&project, &server);
            drivers.spawn(move || {
                finished.send(settled(project, server)).expect("send");
            });
        }
    });
    drop(finished);
    let results: Vec<(serde_json::Value, u32)> = results.into_iter().collect();

    let elapsed = started.elapsed();
    for (document, _) in &results {
        assert_eq!(document["status"], "ok", "{document}");
    }
    assert!(
        results.iter().any(|(_, waited)| *waited > 0),
        "no call ever waited, so the queue was never exercised"
    );
    let hits = server.hit_instants(&query());
    assert_eq!(hits.len(), 30);
    for pair in hits.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(gap >= RESPONSE + SPACING, "requests {gap:?} apart");
        assert!(gap <= Duration::from_secs(8), "arXiv idle for {gap:?}");
    }
    assert!(
        elapsed <= (RESPONSE + SPACING) * CALLS + Duration::from_secs(60),
        "took {elapsed:?}"
    );
}

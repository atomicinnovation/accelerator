#![allow(clippy::expect_used)]

use std::time::Duration;
use std::time::Instant;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use research_adapters::bounded::current_thread_runtime;
use research_adapters::bounded::read_capped;
use research_adapters::bounded::within;
use research_adapters::bounded::CappedBody;

#[test]
fn work_that_finishes_within_its_budget_answers_its_own_value() {
    let runtime = current_thread_runtime().expect("a runtime");

    let answer =
        within(&runtime, Duration::from_secs(5), async { "done" }, "late");

    assert_eq!(answer, "done");
}

#[test]
fn work_that_outlives_its_budget_answers_timed_out_once_it_passes() {
    let runtime = current_thread_runtime().expect("a runtime");
    let started = Instant::now();

    let answer = within(
        &runtime,
        Duration::from_millis(50),
        std::future::pending::<&str>(),
        "late",
    );

    assert_eq!(answer, "late");
    assert!(started.elapsed() < Duration::from_millis(500));
}

fn capped(length: usize, cap: usize) -> CappedBody {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get("/body"),
        Route::Bytes {
            status: 200,
            body: vec![b'x'; length],
        },
    );
    let runtime = current_thread_runtime().expect("a runtime");
    let url = format!("{}/body", server.base_url());
    runtime.block_on(async {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("a client");
        let response = client.get(url).send().await.expect("a response");
        read_capped(response, cap).await.expect("a body")
    })
}

#[test]
fn a_body_within_its_cap_is_read_whole() {
    for length in [0, 99, 100] {
        let body = capped(length, 100);

        assert_eq!(body.head.len(), length, "{length}");
        assert!(!body.overflowed, "{length}");
    }
}

#[test]
fn a_body_past_its_cap_keeps_its_head_and_reports_the_overflow() {
    let body = capped(101, 100);

    assert_eq!(body.head.len(), 100);
    assert!(body.overflowed);
}

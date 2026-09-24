#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;

use research::classify::ClientRejection;
use research::classify::FetchError;
use research::classify::Reason;
use research::classify::Received;
use research::classify::Response;
use research::fetch::fetch_openalex;
use research::fetch::Cause;
use research::fetch::FetchOutcome;
use research::fetch::OpenAlexFetch;
use research::fetch::OpenAlexPorts;
use research::fetch::Unavailable;
use research::openalex::Work;
use research::record::Record;
use research::request::ApiKey;
use research::request::Endpoint;
use research::request::Family;
use research::request::KeySource;
use research::request::Limit;
use research::request::OpenAlexId;
use research::request::OpenAlexQuery;
use research::request::OpenAlexRequest;

use support::body;
use support::retry_after;
use support::secs;
use support::status;
use support::RecordingClock;
use support::RecordingGate;
use support::ScriptedTransport;
use support::StubOpenAlexDecoder;

fn works(count: usize) -> Vec<Work> {
    (1..=count)
        .map(|n| Work {
            id: format!("https://openalex.org/W{n}"),
            work_type: "article".to_owned(),
            ..Work::default()
        })
        .collect()
}

fn search(limit: &str) -> OpenAlexRequest {
    OpenAlexRequest::Search {
        query: OpenAlexQuery::parse("graph neural networks").expect("valid"),
        limit: Limit::parse(limit).expect("valid"),
    }
}

fn lookup() -> OpenAlexRequest {
    OpenAlexRequest::Lookup(OpenAlexId::parse("W7").expect("valid"))
}

fn key() -> ApiKey {
    ApiKey::new(
        "secret-value".to_owned(),
        KeySource::new("ACCELERATOR_OPENALEX_API_KEY"),
    )
}

struct Harness {
    clock: RecordingClock,
    gate: RecordingGate,
    decoder: StubOpenAlexDecoder,
    api: Endpoint,
}

impl Harness {
    fn new() -> Self {
        Self::with_gate(RecordingGate::default())
    }

    fn with_gate(gate: RecordingGate) -> Self {
        Self {
            clock: RecordingClock::new(),
            gate,
            decoder: StubOpenAlexDecoder { works: works(3) },
            api: Endpoint::new("https://api.openalex.org"),
        }
    }

    fn fetch(
        &self,
        transport: &ScriptedTransport<'_>,
        request: &OpenAlexRequest,
        key: Option<&ApiKey>,
    ) -> FetchOutcome {
        fetch_openalex(
            &OpenAlexFetch {
                request,
                api: &self.api,
                key,
            },
            &OpenAlexPorts {
                transport,
                decoder: &self.decoder,
                clock: &self.clock,
                gate: &self.gate,
            },
            &self.clock.deadline(),
        )
    }
}

const fn unavailable(reason: Reason) -> FetchOutcome {
    FetchOutcome::Unavailable(Unavailable::new(reason))
}

fn records(outcome: FetchOutcome) -> Vec<Record> {
    match outcome {
        FetchOutcome::Records(records) => records,
        other => panic!("expected records, got {other:?}"),
    }
}

#[test]
fn a_server_error_on_every_attempt_backs_off_three_six_twelve() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(500)]);

    let outcome = harness.fetch(&transport, &search("10"), None);

    assert_eq!(outcome, unavailable(Reason::UpstreamError));
    assert_eq!(transport.hits(), 4);
    assert_eq!(harness.clock.slept(), [secs(3), secs(6), secs(12)]);
}

#[test]
fn a_long_retry_after_is_clamped_and_the_deadline_ends_the_call() {
    let harness = Harness::new();
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![retry_after(429, 120)],
    );

    let outcome = harness.fetch(&transport, &search("10"), None);

    assert_eq!(outcome, unavailable(Reason::RateLimited));
    assert_eq!(transport.hits(), 3);
    assert_eq!(harness.clock.slept(), [secs(30), secs(30)]);
}

#[test]
fn timeouts_on_every_attempt_end_as_upstream_errors_within_the_deadline() {
    let harness = Harness::new();
    let transport = ScriptedTransport::new(
        &harness.clock,
        vec![(Response::TimedOut, secs(30))],
    );

    let outcome = harness.fetch(&transport, &search("10"), None);

    assert_eq!(outcome, unavailable(Reason::UpstreamError));
    assert_eq!(transport.hits(), 3);
    assert_eq!(harness.clock.slept(), [secs(3), secs(6)]);
}

#[test]
fn a_retry_that_succeeds_returns_its_records() {
    let harness = Harness::new();
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![retry_after(429, 5), body(b"page")],
    );

    let found = records(harness.fetch(&transport, &search("10"), None));

    assert_eq!(found.len(), 3);
    assert_eq!(harness.clock.slept(), [secs(5)]);
}

#[test]
fn budget_exhaustion_makes_exactly_one_attempt() {
    for exhausted in [
        status(409),
        Response::Received(Received {
            rate_limit_remaining: Some(5),
            rate_limit_credits_required: Some(10),
            ..Received::status(429)
        }),
        Response::Received(Received {
            rate_limit_remaining: Some(0),
            ..Received::status(429)
        }),
    ] {
        let harness = Harness::new();
        let transport =
            ScriptedTransport::immediate(&harness.clock, vec![exhausted]);

        let outcome = harness.fetch(&transport, &search("10"), None);

        assert_eq!(outcome, unavailable(Reason::BudgetExhausted));
        assert_eq!(transport.hits(), 1);
        assert_eq!(harness.gate.deferrals(), [None]);
    }
}

#[test]
fn the_final_attempt_decides_the_reason() {
    let harness = Harness::new();
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![status(429), status(500)],
    );

    let outcome = harness.fetch(&transport, &search("10"), None);

    assert_eq!(outcome, unavailable(Reason::UpstreamError));
    assert_eq!(transport.hits(), 4);
}

#[test]
fn a_throttled_attempt_defers_other_callers_by_its_scheduled_wait() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(429)]);

    harness.fetch(&transport, &search("10"), None);

    let at = |seconds| Some(harness.clock.wall_at(secs(seconds)));
    assert_eq!(harness.gate.deferrals(), [at(3), at(9), at(21), at(33)]);
}

#[test]
fn a_throttled_attempt_defers_by_its_clamped_retry_after() {
    let harness = Harness::new();
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![retry_after(429, 120)],
    );

    harness.fetch(&transport, &search("10"), None);

    let at = |seconds| Some(harness.clock.wall_at(secs(seconds)));
    assert_eq!(harness.gate.deferrals(), [at(30), at(60), at(90)]);
}

#[test]
fn only_throttling_defers_other_callers() {
    for response in
        [status(503), Response::TimedOut, Response::ConnectionFailed]
    {
        let harness = Harness::new();
        let transport =
            ScriptedTransport::immediate(&harness.clock, vec![response]);

        harness.fetch(&transport, &search("10"), None);

        assert!(harness.gate.deferrals().iter().all(Option::is_none));
    }
}

#[test]
fn a_search_returns_no_more_records_than_its_limit() {
    let harness = Harness {
        decoder: StubOpenAlexDecoder { works: works(30) },
        ..Harness::new()
    };
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(b"page")]);

    let found = records(harness.fetch(&transport, &search("10"), None));

    assert_eq!(found.len(), 10);
    assert_eq!(found[0].url(), "https://openalex.org/W1");
}

#[test]
fn a_work_that_cannot_be_cited_is_dropped() {
    let mut uncitable = works(2);
    uncitable[0].id = "https://evil.example/W1".to_owned();
    let harness = Harness {
        decoder: StubOpenAlexDecoder { works: uncitable },
        ..Harness::new()
    };
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(b"page")]);

    let found = records(harness.fetch(&transport, &search("10"), None));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].url(), "https://openalex.org/W2");
}

#[test]
fn a_lookup_returns_the_one_work() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(b"work")]);

    let found = records(harness.fetch(&transport, &lookup(), None));

    assert_eq!(found.len(), 1);
    assert_eq!(
        transport.urls()[0].split('?').next(),
        Some("https://api.openalex.org/works/W7")
    );
}

#[test]
fn a_lookup_miss_returns_no_records() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(404)]);

    assert_eq!(
        harness.fetch(&transport, &lookup(), None),
        FetchOutcome::Records(Vec::new())
    );
}

#[test]
fn a_rejected_key_fails_without_retrying() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(401)]);

    let outcome = harness.fetch(&transport, &search("10"), Some(&key()));

    assert_eq!(
        outcome,
        FetchOutcome::Failed(FetchError::KeyRejected(KeySource::new(
            "ACCELERATOR_OPENALEX_API_KEY"
        )))
    );
    assert_eq!(transport.hits(), 1);
    assert_eq!(transport.bearers(), [Some("secret-value".to_owned())]);
}

#[test]
fn a_client_error_fails_without_retrying() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(400)]);

    assert_eq!(
        harness.fetch(&transport, &search("10"), None),
        FetchOutcome::Failed(FetchError::ClientError {
            family: Family::OpenAlex,
            rejection: ClientRejection::Status(400),
        })
    );
    assert_eq!(transport.hits(), 1);
}

#[test]
fn an_undecodable_body_fails() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(b"garbage")]);

    assert_eq!(
        harness.fetch(&transport, &search("10"), None),
        FetchOutcome::Failed(FetchError::UndecodableResponse(Family::OpenAlex))
    );
}

#[test]
fn a_gate_refusal_ends_the_call_unavailable() {
    let harness = Harness::with_gate(RecordingGate::refusing_from(
        1,
        Unavailable::lock_contention(),
    ));
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(b"page")]);

    let outcome = harness.fetch(&transport, &search("10"), None);

    assert_eq!(
        outcome,
        FetchOutcome::Unavailable(Unavailable::lock_contention())
    );
    assert_eq!(transport.hits(), 0);
}

#[test]
fn a_gate_refusal_after_a_retryable_attempt_reports_that_attempt() {
    let harness = Harness::with_gate(RecordingGate::refusing_from(
        2,
        Unavailable::lock_contention(),
    ));
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(500)]);

    let FetchOutcome::Unavailable(unavailable) =
        harness.fetch(&transport, &search("10"), None)
    else {
        panic!("expected unavailable");
    };

    assert_eq!(unavailable.reason(), Reason::UpstreamError);
    assert_eq!(unavailable.cause(), Some(Cause::LockContention));
}

#[test]
fn time_already_spent_leaves_room_for_fewer_attempts() {
    let harness = Harness::new();
    harness.clock.advance(secs(65));
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(500)]);

    let outcome = harness.fetch(&transport, &search("10"), None);

    assert_eq!(outcome, unavailable(Reason::UpstreamError));
    assert_eq!(transport.hits(), 2);
    assert_eq!(harness.clock.slept(), [secs(3)]);
}

#[test]
fn an_attempt_ending_exactly_on_the_deadline_is_admitted() {
    let harness = Harness::new();
    harness.clock.advance(secs(70));
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(500)]);

    harness.fetch(&transport, &search("10"), None);

    assert_eq!(transport.hits(), 1);
}

#[test]
fn no_attempt_fits_once_too_little_of_the_deadline_remains() {
    let harness = Harness::new();
    harness.clock.advance(secs(70) + Duration::from_millis(1));
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(500)]);

    let outcome = harness.fetch(&transport, &search("10"), None);

    assert_eq!(outcome, unavailable(Reason::RateLimited));
    assert_eq!(transport.hits(), 0);
}

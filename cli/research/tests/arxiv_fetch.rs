#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;

use research::sources::arxiv::Entry;
use research::sources::classify::ClientRejection;
use research::sources::classify::FetchError;
use research::sources::classify::Reason;
use research::sources::fetch::fetch_arxiv;
use research::sources::fetch::ArxivFetch;
use research::sources::fetch::ArxivPorts;
use research::sources::fetch::Contention;
use research::sources::fetch::FetchOutcome;
use research::sources::fetch::Unavailable;
use research::sources::queue::Argument;
use research::sources::queue::Position;
use research::sources::queue::Ticket;
use research::sources::queue::TicketRejection;
use research::sources::record::Record;
use research::sources::record::Tier;
use research::sources::request::ArxivId;
use research::sources::request::ArxivQuery;
use research::sources::request::ArxivRequest;
use research::sources::request::Endpoint;
use research::sources::request::Family;
use research::sources::request::Limit;

use support::body;
use support::retry_after;
use support::secs;
use support::status;
use support::Event;
use support::MemoryConfirmations;
use support::RecordingClock;
use support::RecordingContention;
use support::RecordingGate;
use support::ScriptedJoin;
use support::ScriptedQueue;
use support::ScriptedTransport;
use support::StubArxivDecoder;
use support::Timeline;
use support::EMPTY_FEED;
use support::ERROR_FEED;
use support::FEED;
use support::LIVE_RAW;
use support::WITHDRAWN_RAW;

const WITHDRAWAL: &str = "This paper has been withdrawn by the authors.";

fn entry(id: &str, comment: Option<&str>) -> Entry {
    Entry {
        id: format!("http://arxiv.org/abs/{id}"),
        title: format!("Entry {id}"),
        comment: comment.map(str::to_owned),
        ..Entry::default()
    }
}

fn search(limit: &str) -> ArxivRequest {
    ArxivRequest::Search {
        query: ArxivQuery::parse("graph neural networks").expect("valid"),
        limit: Limit::parse(limit).expect("valid"),
    }
}

fn lookup(id: &str) -> ArxivRequest {
    ArxivRequest::Lookup(ArxivId::parse(id).expect("valid"))
}

const SPACING: Duration = secs(3);

struct Harness {
    timeline: Timeline,
    clock: RecordingClock,
    gate: RecordingGate,
    queue: ScriptedQueue,
    contention: RecordingContention,
    decoder: StubArxivDecoder,
    api: Endpoint,
    oai: Endpoint,
}

impl Harness {
    fn with(entries: Vec<Entry>) -> Self {
        let timeline = Timeline::default();
        Self {
            clock: RecordingClock::on(timeline.clone()),
            gate: RecordingGate::on(timeline.clone()).spacing(SPACING),
            queue: ScriptedQueue::on(timeline.clone()),
            timeline,
            contention: RecordingContention::default(),
            decoder: StubArxivDecoder { entries },
            api: Endpoint::new("https://export.arxiv.org"),
            oai: Endpoint::new("https://oaipmh.arxiv.org"),
        }
    }

    fn gate(
        mut self,
        scripted: impl FnOnce(RecordingGate) -> RecordingGate,
    ) -> Self {
        self.gate = scripted(self.gate);
        self
    }

    fn queue(
        mut self,
        scripted: impl FnOnce(ScriptedQueue) -> ScriptedQueue,
    ) -> Self {
        self.queue = scripted(self.queue);
        self
    }

    fn waiting_at(&self, position: u32) -> FetchOutcome {
        FetchOutcome::Waiting {
            ticket: self.queue.ticket(),
            position: Position::new(position),
        }
    }

    fn fetch(
        &self,
        transport: &ScriptedTransport<'_>,
        confirmations: &MemoryConfirmations<'_>,
        request: &ArxivRequest,
    ) -> FetchOutcome {
        fetch_arxiv(
            &ArxivFetch {
                request,
                presented: None,
                api: &self.api,
                oai: &self.oai,
            },
            &ArxivPorts {
                transport,
                decoder: &self.decoder,
                clock: &self.clock,
                gate: &self.gate,
                confirmations,
                queue: &self.queue,
                contention: &self.contention,
            },
            &self.clock.deadline(),
        )
    }
}

fn records(outcome: FetchOutcome) -> Vec<Record> {
    match outcome {
        FetchOutcome::Records(records) => records,
        other => panic!("expected records, got {other:?}"),
    }
}

fn is_confirmation(url: &str) -> bool {
    url.starts_with("https://oaipmh.arxiv.org/oai?")
}

const fn unavailable(reason: Reason) -> FetchOutcome {
    FetchOutcome::Unavailable(Unavailable::new(reason))
}

const fn millis(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn a_search_returns_tier_two_records_up_to_its_limit() {
    let harness = Harness::with(
        (1..=5)
            .map(|n| entry(&format!("2608.0000{n}v1"), None))
            .collect(),
    );
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let found =
        records(harness.fetch(&transport, &confirmations, &search("3")));

    assert_eq!(found.len(), 3);
    assert!(found.iter().all(|record| record.tier() == Tier::Two));
    assert_eq!(found[0].url(), "https://arxiv.org/abs/2608.00001");
}

#[test]
fn an_entry_that_cannot_be_cited_is_dropped() {
    let mut foreign = entry("2608.00001v1", None);
    foreign.id = "https://evil.example/abs/2608.00001v1".to_owned();
    let harness = Harness::with(vec![foreign, entry("2608.00002v1", None)]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let found =
        records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].url(), "https://arxiv.org/abs/2608.00002");
}

#[test]
fn a_versioned_lookup_queries_the_unversioned_id() {
    let harness = Harness::with(vec![entry("2608.21129v2", None)]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let found = records(harness.fetch(
        &transport,
        &confirmations,
        &lookup("2608.21129v2"),
    ));

    assert_eq!(found.len(), 1);
    assert_eq!(
        transport.urls(),
        ["https://export.arxiv.org/api/query?id_list=2608.21129"]
    );
}

#[test]
fn a_lookup_answered_with_an_empty_feed_finds_nothing() {
    let harness = Harness::with(Vec::new());
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(EMPTY_FEED)]);

    assert_eq!(
        harness.fetch(&transport, &confirmations, &lookup("2608.21129")),
        FetchOutcome::Records(Vec::new())
    );
}

#[test]
fn a_lookup_answered_with_a_different_entry_finds_nothing() {
    let harness = Harness::with(vec![entry("2608.21130v1", None)]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    assert_eq!(
        harness.fetch(&transport, &confirmations, &lookup("2608.21129")),
        FetchOutcome::Records(Vec::new())
    );
}

#[test]
fn a_confirmed_withdrawal_is_tier_three_and_withdrawn() {
    let harness = Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![body(FEED), body(WITHDRAWN_RAW)],
    );

    let found =
        records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(found[0].tier(), Tier::Three);
    assert!(found[0].withdrawn());
    assert!(!found[0].retracted());
    assert_eq!(
        transport.urls()[1],
        "https://oaipmh.arxiv.org/oai?verb=GetRecord&identifier=\
         oai:arXiv.org:2608.21129&metadataPrefix=arXivRaw"
    );
}

#[test]
fn a_withdrawal_comment_the_raw_record_contradicts_stays_tier_two() {
    let harness = Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![body(FEED), body(LIVE_RAW)],
    );

    let found =
        records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(found[0].tier(), Tier::Two);
    assert!(!found[0].withdrawn());
}

#[test]
fn an_entry_with_no_withdrawal_comment_needs_no_confirmation() {
    let harness = Harness::with(vec![entry("2608.21129v1", Some("12 pages"))]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(transport.hits(), 1);
}

#[test]
fn a_recalled_verdict_needs_no_attempt() {
    let harness = Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
    let confirmations =
        MemoryConfirmations::remembering(&harness.gate, "2608.21129v2", true);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let found =
        records(harness.fetch(&transport, &confirmations, &search("10")));

    assert!(found[0].withdrawn());
    assert_eq!(harness.gate.turns_served(), 1);
    assert_eq!(harness.gate.attempts(), 1);
    assert_eq!(transport.hits(), 1);
}

#[test]
fn a_verdict_recalled_for_an_older_version_does_not_apply() {
    let harness = Harness::with(vec![entry("2608.21129v3", Some(WITHDRAWAL))]);
    let confirmations =
        MemoryConfirmations::remembering(&harness.gate, "2608.21129v2", true);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![body(FEED), body(LIVE_RAW)],
    );

    let found =
        records(harness.fetch(&transport, &confirmations, &search("10")));

    assert!(!found[0].withdrawn());
    assert_eq!(harness.gate.turns_served(), 1);
    assert_eq!(harness.gate.attempts(), 2);
}

#[test]
fn a_confirmed_verdict_is_recorded_while_serving() {
    let harness = Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![body(FEED), body(WITHDRAWN_RAW)],
    );

    harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(
        confirmations.verdicts(),
        [(ArxivId::parse("2608.21129v2").expect("valid"), true)]
    );
    assert_eq!(confirmations.recorded_while_serving(), [true]);
}

#[test]
fn entries_sharing_a_withdrawal_candidate_need_one_confirmation() {
    let harness = Harness::with(vec![
        entry("2608.21129v2", Some(WITHDRAWAL)),
        entry("2608.21129v2", Some(WITHDRAWAL)),
    ]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![body(FEED), body(WITHDRAWN_RAW)],
    );

    let found =
        records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(
        transport
            .urls()
            .iter()
            .filter(|url| is_confirmation(url))
            .count(),
        1
    );
    assert!(found.iter().all(Record::withdrawn));
}

#[test]
fn an_unavailable_confirmation_makes_the_whole_call_unavailable() {
    let harness = Harness::with(vec![
        entry("2608.00001v1", None),
        entry("2608.21129v2", Some(WITHDRAWAL)),
    ]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![body(FEED), status(503)],
    );

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(
        outcome,
        FetchOutcome::Unavailable(Unavailable::new(Reason::UpstreamError))
    );
    assert!(confirmations.verdicts().is_empty());
}

#[test]
fn a_confirmation_that_does_not_settle_is_tried_again_by_a_later_call() {
    for unsettled in [status(503), status(400), body(b"garbage")] {
        let harness =
            Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
        let confirmations = MemoryConfirmations::new(&harness.gate);
        let first = ScriptedTransport::immediate(
            &harness.clock,
            vec![body(FEED), unsettled],
        );
        harness.fetch(&first, &confirmations, &search("10"));
        assert!(confirmations.verdicts().is_empty());

        let second = ScriptedTransport::immediate(
            &harness.clock,
            vec![body(FEED), body(WITHDRAWN_RAW)],
        );
        let found =
            records(harness.fetch(&second, &confirmations, &search("10")));
        assert!(found[0].withdrawn());
    }
}

#[test]
fn a_confirmation_inherits_what_remains_of_the_deadline() {
    let harness = Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::new(
        &harness.clock,
        vec![(body(FEED), secs(71)), (body(WITHDRAWN_RAW), secs(0))],
    );

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(1));
    assert_eq!(harness.queue.stepped_aside(), [None]);
    assert_eq!(transport.hits(), 1);
}

#[test]
fn arxiv_throttling_defers_other_callers() {
    let harness = Harness::with(Vec::new());
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(403)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(
        outcome,
        FetchOutcome::Unavailable(Unavailable::new(Reason::RateLimited))
    );
    assert_eq!(
        harness.gate.deferrals()[0],
        Some(harness.clock.wall_at(secs(3)))
    );
}

#[test]
fn an_undecodable_feed_fails() {
    let harness = Harness::with(Vec::new());
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(b"garbage")]);

    assert_eq!(
        harness.fetch(&transport, &confirmations, &search("10")),
        FetchOutcome::Failed(FetchError::UndecodableResponse(Family::Arxiv))
    );
}

#[test]
fn an_error_feed_is_a_client_error() {
    let harness = Harness::with(Vec::new());
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(ERROR_FEED)]);

    assert_eq!(
        harness.fetch(&transport, &confirmations, &search("10")),
        FetchOutcome::Failed(FetchError::ClientError {
            family: Family::Arxiv,
            rejection: ClientRejection::ErrorFeed(
                "incorrect id format".to_owned()
            ),
        })
    );
}

fn rejected(ticket: &Ticket) -> TicketRejection {
    TicketRejection::Mismatch {
        ticket: ticket.clone(),
        issued: research::sources::queue::Binding::of(&search("5")),
        differing: vec![Argument::Limit],
    }
}

fn over_cap(harness: &Harness, last_retryable: Option<Reason>) -> ScriptedJoin {
    ScriptedJoin::OverCap {
        ticket: harness.queue.ticket(),
        last_retryable,
    }
}

#[test]
fn a_whole_call_is_served_in_one_turn() {
    let harness = Harness::with(vec![
        entry("2608.21129v2", Some(WITHDRAWAL)),
        entry("2608.21130v1", Some(WITHDRAWAL)),
    ]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![body(FEED), status(503), body(WITHDRAWN_RAW), body(LIVE_RAW)],
    );

    records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(harness.gate.turns_served(), 1);
    assert_eq!(
        harness.timeline.events(),
        [
            Event::Served,
            Event::Attempt,
            Event::Attempt,
            Event::Slept(secs(3)),
            Event::Attempt,
            Event::Attempt,
            Event::Released,
            Event::Left,
        ]
    );
}

#[test]
fn a_front_invocation_with_32_s_left_waits_without_a_request() {
    let harness = Harness::with(Vec::new());
    harness.clock.leaving(secs(32));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(1));
    assert_eq!(harness.gate.attempts(), 0);
    assert_eq!(harness.queue.stepped_aside(), [None]);
}

#[test]
fn with_33_s_left_it_is_served() {
    let harness = Harness::with(Vec::new());
    harness.clock.leaving(secs(33));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(harness.gate.attempts(), 1);
    assert_eq!(harness.queue.left(), 1);
}

#[test]
fn a_6_s_backoff_with_38_s_left_releases_and_waits() {
    let harness = Harness::with(Vec::new());
    harness.clock.leaving(secs(38));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![retry_after(503, 6), body(FEED)],
    );

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(1));
    assert_eq!(
        harness.timeline.events(),
        [
            Event::Served,
            Event::Attempt,
            Event::Released,
            Event::SteppedAside,
        ]
    );
}

#[test]
fn a_6_s_backoff_with_39_s_left_retries_under_the_turn() {
    let harness = Harness::with(Vec::new());
    harness.clock.leaving(secs(39));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![retry_after(503, 6), body(FEED)],
    );

    records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(
        harness.timeline.events(),
        [
            Event::Served,
            Event::Attempt,
            Event::Slept(secs(6)),
            Event::Attempt,
            Event::Released,
            Event::Left,
        ]
    );
}

#[test]
fn a_later_request_needs_a_full_serving_window() {
    let harness = Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::new(
        &harness.clock,
        vec![(body(FEED), secs(68)), (body(WITHDRAWN_RAW), secs(0))],
    );

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(1));
    assert_eq!(harness.gate.attempts(), 1);
    assert_eq!(harness.queue.stepped_aside(), [None]);
    assert_eq!(harness.queue.left(), 0);
}

#[test]
fn a_40_s_budget_behind_a_20_s_holder_waits_at_position_2() {
    let harness = Harness::with(Vec::new())
        .queue(|queue| queue.not_front_for(200).at_position(2));
    harness.clock.leaving(secs(40));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(2));
    assert_eq!(transport.hits(), 0);
    assert!(harness.contention.recorded().is_empty());
}

#[test]
fn a_gate_busy_briefly_serves_the_call_once_it_frees() {
    let harness = Harness::with(Vec::new()).gate(|gate| gate.busy_for(2));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(harness.clock.slept(), [millis(100), millis(100)]);
    assert!(harness.contention.recorded().is_empty());
}

#[test]
fn a_free_gate_out_of_budget_records_no_contention() {
    let harness = Harness::with(Vec::new());
    harness.clock.leaving(secs(32));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(1));
    assert!(harness.contention.recorded().is_empty());
}

#[test]
fn a_cached_confirmation_is_recalled_inside_the_turn() {
    let harness = Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
    let confirmations =
        MemoryConfirmations::remembering(&harness.gate, "2608.21129v2", true);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(confirmations.recalled_while_serving(), [true]);
}

#[test]
fn a_second_confirmation_the_budget_cannot_cover_waits_and_keeps_its_place() {
    let harness = Harness::with(vec![
        entry("2608.21129v2", Some(WITHDRAWAL)),
        entry("2608.21130v1", Some(WITHDRAWAL)),
    ]);
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::new(
        &harness.clock,
        vec![(body(FEED), secs(30)), (body(WITHDRAWN_RAW), secs(40))],
    );

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(1));
    assert_eq!(
        confirmations.verdicts(),
        [(ArxivId::parse("2608.21129v2").expect("valid"), true)]
    );
    assert_eq!(harness.queue.stepped_aside(), [None]);
    assert_eq!(harness.queue.left(), 0);
}

#[test]
fn four_429s_are_rate_limited_without_a_cause_and_leave_the_queue() {
    let harness = Harness::with(Vec::new());
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(429)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, unavailable(Reason::RateLimited));
    assert_eq!(transport.hits(), 4);
    assert_eq!(harness.queue.left(), 1);
    assert!(harness.queue.stepped_aside().is_empty());
}

#[test]
fn every_settled_outcome_leaves_exactly_once() {
    for (answer, settled) in [
        (body(FEED), "records"),
        (body(EMPTY_FEED), "none found"),
        (body(b"garbage"), "failed"),
        (status(503), "unavailable"),
    ] {
        let harness = Harness::with(vec![entry("2608.00001v1", None)]);
        let confirmations = MemoryConfirmations::new(&harness.gate);
        let transport =
            ScriptedTransport::immediate(&harness.clock, vec![answer]);

        harness.fetch(&transport, &confirmations, &search("10"));

        let events = harness.timeline.events();
        assert_eq!(harness.queue.left(), 1, "{settled}");
        assert!(harness.queue.stepped_aside().is_empty(), "{settled}");
        assert_eq!(
            events[events.len() - 2..],
            [Event::Released, Event::Left],
            "{settled}"
        );
    }
}

#[test]
fn a_retry_after_past_the_budget_waits() {
    let harness = Harness::with(Vec::new());
    harness.clock.leaving(secs(60));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![retry_after(429, 30)],
    );

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(1));
    assert_eq!(harness.queue.stepped_aside(), [Some(Reason::RateLimited)]);
}

#[test]
fn a_deferral_the_turn_cannot_admit_waits() {
    let harness = Harness::with(Vec::new()).gate(|gate| gate.refusing_from(1));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, harness.waiting_at(1));
    assert_eq!(transport.hits(), 0);
}

#[test]
fn a_503_then_an_exhausted_budget_steps_aside_with_its_reason() {
    let harness = Harness::with(Vec::new());
    harness.clock.leaving(secs(38));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![retry_after(503, 6), body(FEED)],
    );

    harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(harness.queue.stepped_aside(), [Some(Reason::UpstreamError)]);
}

#[test]
fn an_over_cap_join_is_lock_contention_without_a_request() {
    let harness = Harness::with(Vec::new());
    let verdict = over_cap(&harness, None);
    let harness = harness.queue(|queue| queue.joining(verdict));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(
        outcome,
        FetchOutcome::Unavailable(Unavailable::lock_contention())
    );
    assert_eq!(
        harness.contention.recorded(),
        [Contention::TicketPastCap(harness.queue.ticket())]
    );
    assert_eq!(transport.hits(), 0);
}

#[test]
fn an_over_cap_join_after_an_upstream_failure_reports_that_failure() {
    let harness = Harness::with(Vec::new());
    let verdict = over_cap(&harness, Some(Reason::UpstreamError));
    let harness = harness.queue(|queue| queue.joining(verdict));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, unavailable(Reason::UpstreamError));
    assert!(harness.contention.recorded().is_empty());
    assert_eq!(transport.hits(), 0);
}

#[test]
fn a_mismatch_is_rejected_without_a_request() {
    let harness = Harness::with(Vec::new());
    let rejection = rejected(&harness.queue.ticket());
    let harness = harness.queue(|queue| {
        queue.joining(ScriptedJoin::Rejected(rejection.clone()))
    });
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, FetchOutcome::Rejected(rejection));
    assert_eq!(transport.hits(), 0);
}

#[test]
fn a_ticket_crossing_900_s_while_waiting_is_still_served() {
    let harness =
        Harness::with(Vec::new()).queue(|queue| queue.not_front_for(600));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(harness.queue.left(), 1);
}

#[test]
fn an_unqueued_call_out_of_budget_is_lock_contention_not_waiting() {
    for refusing in [false, true] {
        let harness = Harness::with(Vec::new())
            .queue(|queue| queue.joining(ScriptedJoin::Unqueued))
            .gate(|gate| {
                if refusing {
                    gate.refusing_from(1)
                } else {
                    gate
                }
            });
        if !refusing {
            harness.clock.leaving(secs(32));
        }
        let confirmations = MemoryConfirmations::new(&harness.gate);
        let transport =
            ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

        let outcome = harness.fetch(&transport, &confirmations, &search("10"));

        assert_eq!(
            outcome,
            FetchOutcome::Unavailable(Unavailable::lock_contention())
        );
        assert_eq!(harness.contention.recorded(), [Contention::QueueUnusable]);
    }
}

#[test]
fn an_unqueued_call_out_of_budget_after_a_503_reports_that_failure() {
    let harness = Harness::with(Vec::new())
        .queue(|queue| queue.joining(ScriptedJoin::Unqueued));
    harness.clock.leaving(secs(38));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport = ScriptedTransport::immediate(
        &harness.clock,
        vec![retry_after(503, 6), body(FEED)],
    );

    let outcome = harness.fetch(&transport, &confirmations, &search("10"));

    assert_eq!(outcome, unavailable(Reason::UpstreamError));
    assert!(harness.contention.recorded().is_empty());
}

#[test]
fn an_unqueued_call_is_served_without_a_place() {
    let harness = Harness::with(vec![entry("2608.00001v1", None)])
        .queue(|queue| queue.joining(ScriptedJoin::Unqueued));
    let confirmations = MemoryConfirmations::new(&harness.gate);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let found =
        records(harness.fetch(&transport, &confirmations, &search("10")));

    assert_eq!(found.len(), 1);
    assert_eq!(harness.queue.left(), 0);
}

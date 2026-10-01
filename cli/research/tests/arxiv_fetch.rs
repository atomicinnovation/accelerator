#![allow(clippy::expect_used, clippy::panic)]

mod support;

use research::arxiv::Entry;
use research::classify::ClientRejection;
use research::classify::FetchError;
use research::classify::Reason;
use research::fetch::fetch_arxiv;
use research::fetch::ArxivFetch;
use research::fetch::ArxivPorts;
use research::fetch::FetchOutcome;
use research::fetch::Unavailable;
use research::record::Record;
use research::record::Tier;
use research::request::ArxivId;
use research::request::ArxivQuery;
use research::request::ArxivRequest;
use research::request::Endpoint;
use research::request::Family;
use research::request::Limit;

use support::body;
use support::secs;
use support::status;
use support::MemoryConfirmations;
use support::RecordingClock;
use support::RecordingGate;
use support::ScriptedTransport;
use support::StubArxivDecoder;
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

struct Harness {
    clock: RecordingClock,
    gate: RecordingGate,
    decoder: StubArxivDecoder,
    api: Endpoint,
    oai: Endpoint,
}

impl Harness {
    fn with(entries: Vec<Entry>) -> Self {
        Self {
            clock: RecordingClock::new(),
            gate: RecordingGate::default(),
            decoder: StubArxivDecoder { entries },
            api: Endpoint::new("https://export.arxiv.org"),
            oai: Endpoint::new("https://oaipmh.arxiv.org"),
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
                api: &self.api,
                oai: &self.oai,
            },
            &ArxivPorts {
                transport,
                decoder: &self.decoder,
                clock: &self.clock,
                gate: &self.gate,
                confirmations,
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
fn a_recalled_verdict_needs_no_gate_pass() {
    let harness = Harness::with(vec![entry("2608.21129v2", Some(WITHDRAWAL))]);
    let confirmations =
        MemoryConfirmations::remembering(&harness.gate, "2608.21129v2", true);
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(FEED)]);

    let found =
        records(harness.fetch(&transport, &confirmations, &search("10")));

    assert!(found[0].withdrawn());
    assert_eq!(harness.gate.passes(), 1);
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
    assert_eq!(harness.gate.passes(), 2);
}

#[test]
fn a_confirmed_verdict_is_recorded_from_inside_its_gate_pass() {
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
    assert_eq!(confirmations.recorded_inside_pass(), [true]);
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

    assert_eq!(
        outcome,
        FetchOutcome::Unavailable(Unavailable::new(Reason::RateLimited))
    );
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

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::cell::RefCell;

use research::sources::classify::Reason;
use research::sources::crossref::CrossrefWork;
use research::sources::crossref::DoiMetadata;
use research::sources::crossref::Venue;
use research::sources::fetch::fetch_doi;
use research::sources::fetch::CrossrefDecoder;
use research::sources::fetch::DecodeFailure;
use research::sources::fetch::DoiFetch;
use research::sources::fetch::DoiPorts;
use research::sources::fetch::Unavailable;
use research::sources::ports::Diagnostics;
use research::sources::request::Doi;
use research::sources::request::Endpoint;
use research::sources::serving::Queued;

use support::body;
use support::status;
use support::RecordingClock;
use support::RecordingContention;
use support::RecordingGate;
use support::ScriptedQueue;
use support::ScriptedTransport;
use support::Timeline;

const RECORD: &[u8] = b"record";

/// Decodes [`RECORD`] as a proceedings article; anything else is
/// undecodable.
struct StubCrossrefDecoder;

impl CrossrefDecoder for StubCrossrefDecoder {
    fn work(&self, body: &[u8]) -> Result<CrossrefWork, DecodeFailure> {
        if body != RECORD {
            return Err(DecodeFailure::Undecodable);
        }
        Ok(CrossrefWork {
            kind: Some("proceedings-article".to_owned()),
            container_titles: vec!["WWW".to_owned()],
            publisher: Some("ACM".to_owned()),
        })
    }
}

#[derive(Default)]
struct RecordingDiagnostics(RefCell<Vec<String>>);

impl Diagnostics for RecordingDiagnostics {
    fn report(&self, line: &str) {
        self.0.borrow_mut().push(line.to_owned());
    }
}

struct Harness {
    clock: RecordingClock,
    gate: RecordingGate,
    queue: ScriptedQueue,
    contention: RecordingContention,
    diagnostics: RecordingDiagnostics,
}

impl Harness {
    fn new() -> Self {
        let timeline = Timeline::default();
        Self {
            clock: RecordingClock::on(timeline.clone()),
            gate: RecordingGate::on(timeline.clone()),
            queue: ScriptedQueue::on(timeline),
            contention: RecordingContention::default(),
            diagnostics: RecordingDiagnostics::default(),
        }
    }

    fn fetch(&self, transport: &ScriptedTransport<'_>) -> Queued<DoiMetadata> {
        fetch_doi(
            &DoiFetch {
                doi: &Doi::parse("10.1145/371920.372066").expect("a DOI"),
                presented: None,
                api: &Endpoint::new("https://crossref.test"),
                contact: None,
            },
            &DoiPorts {
                transport,
                decoder: &StubCrossrefDecoder,
                clock: &self.clock,
                gate: &self.gate,
                queue: &self.queue,
                contention: &self.contention,
                diagnostics: &self.diagnostics,
            },
            &self.clock.deadline(),
        )
    }
}

#[test]
fn a_delivered_record_is_served_as_its_venue_and_publisher() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(RECORD)]);

    let Queued::Served(metadata) = harness.fetch(&transport) else {
        panic!("expected a served call");
    };

    assert_eq!(metadata.venue.as_ref().map(Venue::as_str), Some("WWW"));
    assert_eq!(
        metadata.metadata.entries().collect::<Vec<_>>(),
        [("publisher", "ACM")]
    );
    assert_eq!(
        transport.urls(),
        ["https://crossref.test/works/10.1145/371920.372066"]
    );
    assert_eq!(harness.queue.left(), 1);
}

#[test]
fn a_miss_is_served_as_empty_metadata() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(404)]);

    assert_eq!(
        harness.fetch(&transport),
        Queued::Served(DoiMetadata::default())
    );
}

#[test]
fn a_rejected_request_is_unavailable_after_one_attempt() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![status(400)]);

    assert_eq!(
        harness.fetch(&transport),
        Queued::Unavailable(Unavailable::new(Reason::UpstreamError))
    );
    assert_eq!(transport.hits(), 1);
}

#[test]
fn an_undecodable_record_is_unavailable_and_reported_once() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(b"<html>")]);

    assert_eq!(
        harness.fetch(&transport),
        Queued::Unavailable(Unavailable::new(Reason::UpstreamError))
    );
    let reported = harness.diagnostics.0.borrow();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(
        reported[0].contains("doi metadata 10.1145/371920.372066"),
        "{reported:?}"
    );
    assert!(reported[0].contains("could not be read"), "{reported:?}");
}

#[test]
fn a_served_record_reports_nothing() {
    let harness = Harness::new();
    let transport =
        ScriptedTransport::immediate(&harness.clock, vec![body(RECORD)]);

    harness.fetch(&transport);

    assert!(harness.diagnostics.0.borrow().is_empty());
}

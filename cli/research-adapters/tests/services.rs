#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::rc::Rc;

use research::sources::fetch::Clock as _;
use research::sources::fetch::PacingGate as _;
use research::sources::queue::Binding;
use research::sources::queue::Joining;
use research::sources::queue::Provenance;
use research::sources::queue::ServiceQueue as _;
use research::sources::queue::Ticket;
use research::sources::queue::TicketRejection;
use research::sources::request::ArxivQuery;
use research::sources::request::ArxivRequest;
use research::sources::request::Family;
use research::sources::request::Limit;
use research::sources::schedule::Deadline;
use research::sources::service::CrossrefPool;
use research::sources::service::Service;
use research_adapters::services::ServiceAdapters;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use support::adapter_context;
use support::previous_release;
use support::secs;
use support::LegacyFile;
use support::RecordedDiagnostics;
use support::RecordingClock;
use support::Scratch;

const PUBLIC: CrossrefPool = CrossrefPool::Public;

fn graphs() -> Binding {
    Binding::of(&ArxivRequest::Search {
        query: ArxivQuery::parse("graphs").expect("a query"),
        limit: Limit::parse("10").expect("a limit"),
    })
}

fn ticket(text: &str) -> Ticket {
    Ticket::parse(text).expect("a ticket")
}

struct Harness {
    scratch: Scratch,
    clock: Rc<RecordingClock>,
    diagnostics: Rc<RecordedDiagnostics>,
}

impl Harness {
    fn new() -> Self {
        Self {
            scratch: Scratch::new(),
            clock: RecordingClock::new(),
            diagnostics: Rc::default(),
        }
    }

    fn arxiv(&self, upgrading: bool) -> ServiceAdapters {
        ServiceAdapters::for_arxiv(
            &adapter_context(
                &self.scratch,
                self.clock.clone(),
                self.diagnostics.clone(),
            ),
            upgrading.then(|| previous_release(&self.scratch)),
        )
    }

    fn crossref(&self, pool: CrossrefPool) -> ServiceAdapters {
        ServiceAdapters::for_crossref(
            pool,
            &adapter_context(
                &self.scratch,
                self.clock.clone(),
                self.diagnostics.clone(),
            ),
        )
    }

    fn deadline(&self) -> Deadline {
        Deadline::starting(self.clock.now(), secs(100), secs(30))
    }

    fn hold_legacy_lock(&self) -> std::fs::File {
        let lock = self
            .scratch
            .open(self.scratch.legacy_arxiv_state(LegacyFile::Lock));
        flock(&lock, FlockOperation::NonBlockingLockExclusive)
            .expect("the test holds arxiv.lock");
        lock
    }

    fn presented(
        &self,
        adapters: &ServiceAdapters,
        text: &str,
    ) -> Option<TicketRejection> {
        let presented = ticket(text);
        match adapters
            .queue
            .join(&graphs(), Some(&presented), &self.deadline())
        {
            Joining::Rejected(rejection) => Some(rejection),
            _ => None,
        }
    }
}

fn foreign(text: &str, provenance: Provenance) -> TicketRejection {
    TicketRejection::Foreign {
        ticket: ticket(text),
        provenance,
    }
}

#[test]
fn arxiv_s_gate_takes_the_previous_release_lock_only_while_upgrading() {
    let harness = Harness::new();
    let _older = harness.hold_legacy_lock();

    assert!(harness.arxiv(true).gate.try_serve().is_none());
    assert!(harness.arxiv(false).gate.try_serve().is_some());
}

#[test]
fn crossref_s_gate_never_takes_the_previous_release_lock() {
    let harness = Harness::new();
    let _older = harness.hold_legacy_lock();

    assert!(harness.crossref(PUBLIC).gate.try_serve().is_some());
}

#[test]
fn arxiv_refuses_a_ticket_issued_by_either_crossref_pool() {
    let harness = Harness::new();
    harness
        .scratch
        .seed_service_ticket(Service::Crossref(PUBLIC), "4-aaaaaa");
    harness.scratch.seed_service_ticket(
        Service::Crossref(CrossrefPool::Polite),
        "5-bbbbbb",
    );
    let arxiv = harness.arxiv(false);

    assert_eq!(
        harness.presented(&arxiv, "4-aaaaaa"),
        Some(foreign(
            "4-aaaaaa",
            Provenance::IssuedElsewhere(Family::Doi)
        ))
    );
    assert_eq!(
        harness.presented(&arxiv, "5-bbbbbb"),
        Some(foreign(
            "5-bbbbbb",
            Provenance::IssuedElsewhere(Family::Doi)
        ))
    );
}

#[test]
fn a_ticket_from_the_other_crossref_pool_joins_afresh_with_one_diagnostic() {
    let harness = Harness::new();
    harness.scratch.seed_service_ticket(
        Service::Crossref(CrossrefPool::Polite),
        "5-bbbbbb",
    );
    let public = harness.crossref(PUBLIC);

    let joined = public.queue.join(
        &graphs(),
        Some(&ticket("5-bbbbbb")),
        &harness.deadline(),
    );

    let Joining::Queued(place) = joined else {
        panic!("the ticket joins the public pool afresh");
    };
    assert_ne!(place.ticket(), &ticket("5-bbbbbb"));
    assert_eq!(
        harness.diagnostics.lines(),
        vec!["research fetch: crossref-public queue ticket 5-bbbbbb was \
             issued in Crossref's polite pool, which crossref.mailto no \
             longer selects; joining afresh"
            .to_owned()]
    );
}

#[test]
fn crossref_refuses_an_arxiv_ticket_but_not_the_other_pool_s() {
    let harness = Harness::new();
    harness
        .scratch
        .seed_service_ticket(Service::Arxiv, "4-aaaaaa");
    harness.scratch.seed_service_ticket(
        Service::Crossref(CrossrefPool::Polite),
        "5-bbbbbb",
    );
    let public = harness.crossref(PUBLIC);

    assert_eq!(
        harness.presented(&public, "4-aaaaaa"),
        Some(foreign(
            "4-aaaaaa",
            Provenance::IssuedElsewhere(Family::Arxiv)
        ))
    );
    assert_eq!(harness.presented(&public, "5-bbbbbb"), None);
}

#[test]
fn only_an_upgrading_arxiv_queue_refuses_a_previous_release_ticket() {
    let harness = Harness::new();
    harness.scratch.seed_legacy_ticket("1-e85851");

    assert_eq!(
        harness.presented(&harness.arxiv(true), "1-e85851"),
        Some(foreign("1-e85851", Provenance::IssuedBeforeUpgrade))
    );
    assert_eq!(harness.presented(&harness.arxiv(false), "1-e85851"), None);
    assert_eq!(
        harness.presented(&harness.crossref(PUBLIC), "1-e85851"),
        None
    );
}

#[test]
fn listing_another_queue_never_changes_it() {
    let harness = Harness::new();
    harness.scratch.seed_legacy_ticket("1-e85851");
    let legacy = harness.scratch.legacy_arxiv_state(LegacyFile::Queue);
    let before = std::fs::read_dir(&legacy).expect("list").count();

    harness.presented(&harness.arxiv(true), "1-e85851");

    assert_eq!(std::fs::read_dir(&legacy).expect("list").count(), before);
}

#[test]
fn a_ticket_the_queue_issued_resumes_without_listing_any_other_queue() {
    let harness = Harness::new();
    let legacy = harness.scratch.legacy_arxiv_state(LegacyFile::Queue);
    harness.scratch.write(&legacy, "not a directory");
    let arxiv = harness.arxiv(true);
    let issued = match arxiv.queue.join(&graphs(), None, &harness.deadline()) {
        Joining::Queued(place) => place.ticket().clone(),
        _ => panic!("a fresh call queues"),
    };

    let resumed =
        arxiv
            .queue
            .join(&graphs(), Some(&issued), &harness.deadline());

    assert!(matches!(resumed, Joining::Queued(_)));
    drop(resumed);
    assert_eq!(harness.diagnostics.lines(), Vec::<String>::new());
}

#[test]
fn an_unlistable_foreign_queue_leaves_the_ticket_to_join_with_one_diagnostic() {
    let harness = Harness::new();
    let legacy = harness.scratch.legacy_arxiv_state(LegacyFile::Queue);
    harness.scratch.write(&legacy, "not a directory");

    let arxiv = harness.arxiv(true);
    let joined = arxiv.queue.join(
        &graphs(),
        Some(&ticket("1-e85851")),
        &harness.deadline(),
    );

    assert!(matches!(joined, Joining::Queued(_)));
    drop(joined);
    let reported = harness.diagnostics.lines();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(
        reported[0].contains(&legacy.display().to_string()),
        "{reported:?}"
    );
}

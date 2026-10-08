#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::cell::Cell;
use std::fs::File;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;

use research::sources::classify::Reason;
use research::sources::fetch::Clock;
use research::sources::queue::ArxivQueue as _;
use research::sources::queue::Binding;
use research::sources::queue::Joining;
use research::sources::queue::Nonce;
use research::sources::queue::Place;
use research::sources::queue::Ticket;
use research::sources::queue::TicketRejection;
use research::sources::request::ArxivId;
use research::sources::request::ArxivQuery;
use research::sources::request::ArxivRequest;
use research::sources::request::Limit;
use research::sources::schedule::Deadline;
use research_adapters::clock::SystemClock;
use research_adapters::queue::random_nonce;
use research_adapters::queue::FileArxivQueue;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use support::secs;
use support::RecordedDiagnostics;
use support::RecordingClock;
use support::Scratch;
use support::TicketRecord;
use support::GRAPHS;

const SPACING: Duration = secs(3);

fn search(query: &str, limit: &str) -> Binding {
    Binding::of(&ArxivRequest::Search {
        query: ArxivQuery::parse(query).expect("a query"),
        limit: Limit::parse(limit).expect("a limit"),
    })
}

fn graphs() -> Binding {
    search("graphs", "10")
}

fn ticket(text: &str) -> Ticket {
    Ticket::parse(text).expect("a ticket")
}

fn nonce_of_nth_ticket(n: u32) -> String {
    format!("{n:06x}")
}

struct Harness {
    scratch: Scratch,
    clock: Rc<RecordingClock>,
    diagnostics: Rc<RecordedDiagnostics>,
    issued: Rc<Cell<u32>>,
}

impl Harness {
    fn new() -> Self {
        Self {
            scratch: Scratch::new(),
            clock: RecordingClock::new(),
            diagnostics: Rc::default(),
            issued: Rc::default(),
        }
    }

    fn queue(&self) -> FileArxivQueue {
        let issued = self.issued.clone();
        FileArxivQueue::new(
            self.scratch.queue_dir(),
            self.clock.clone(),
            self.diagnostics.clone(),
            Box::new(move || {
                issued.set(issued.get() + 1);
                Nonce::from_hex(&nonce_of_nth_ticket(issued.get()))
                    .expect("a nonce")
            }),
        )
    }

    fn deadline(&self) -> Deadline {
        Deadline::starting(self.clock.now(), secs(100), secs(30))
    }

    fn seed(&self, ticket: &str, record: &TicketRecord<'_>) {
        self.scratch.seed_ticket(ticket, record);
    }

    /// A ticket issued `issued` ago whose last call ended `ended` ago.
    fn seed_absent(&self, ticket: &str, issued: u64, ended: u64) {
        self.seed(
            ticket,
            &TicketRecord {
                binding: GRAPHS,
                issued: self.clock.ago(secs(issued)),
                presented: self.clock.ago(secs(issued.min(ended + 1))),
                ended: Some(self.clock.ago(secs(ended))),
                last_retryable: None,
            },
        );
    }

    /// A ticket issued `issued` ago whose call is still running.
    fn seed_live(&self, ticket: &str, issued: u64) -> File {
        self.seed(
            ticket,
            &TicketRecord {
                binding: GRAPHS,
                issued: self.clock.ago(secs(issued)),
                presented: self.clock.ago(secs(issued)),
                ended: None,
                last_retryable: None,
            },
        );
        self.scratch.hold_ticket(ticket)
    }

    fn reported(&self) -> Vec<String> {
        self.diagnostics.lines()
    }
}

fn queued<'q>(joining: Joining<'q>) -> Box<dyn Place + 'q> {
    match joining {
        Joining::Queued(place) => place,
        Joining::Unqueued => panic!("unqueued"),
        Joining::Unheld { ticket, .. } => panic!("{ticket} unheld"),
        Joining::Rejected(rejection) => panic!("rejected: {rejection:?}"),
        Joining::OverCap { ticket, .. } => panic!("{ticket} over the cap"),
    }
}

fn join<'q>(
    harness: &Harness,
    queue: &'q FileArxivQueue,
    presented: Option<&str>,
) -> Joining<'q> {
    join_as(harness, queue, &graphs(), presented)
}

fn join_as<'q>(
    harness: &Harness,
    queue: &'q FileArxivQueue,
    binding: &Binding,
    presented: Option<&str>,
) -> Joining<'q> {
    queue.join(
        binding,
        presented.map(ticket).as_ref(),
        &harness.deadline(),
        SPACING,
    )
}

fn millis_of(record: &serde_json::Value, field: &str) -> Option<u64> {
    record[field].as_u64()
}

fn wall_millis(harness: &Harness) -> u64 {
    u64::try_from(support::millis(harness.clock.wall_now()))
        .expect("milliseconds fit")
}

#[test]
fn joining_an_empty_queue_issues_ticket_one_and_holds_its_lock() {
    let harness = Harness::new();
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, None));

    let issued = format!("1-{}", nonce_of_nth_ticket(1));
    assert_eq!(place.ticket().to_string(), issued);
    assert!(harness.scratch.ticket_is_held(&issued));
    let record = harness.scratch.record(&issued).expect("a record");
    assert_eq!(millis_of(&record, "issued_ms"), Some(wall_millis(&harness)));
    assert_eq!(
        millis_of(&record, "presented_ms"),
        Some(wall_millis(&harness))
    );
    assert!(record["ended_ms"].is_null());
    assert!(record["last_retryable"].is_null());
    assert_eq!(record["schema_version"], 1);
    assert_eq!(record["verb"], "search");
    assert_eq!(record["query"], "graphs");
    assert_eq!(record["limit"], 10);
}

#[test]
fn successive_joins_issue_increasing_numbers() {
    let harness = Harness::new();
    let queue = harness.queue();

    let first = queued(join(&harness, &queue, None));
    let second = queued(join(&harness, &queue, None));

    assert_eq!(first.ticket().number(), 1);
    assert_eq!(second.ticket().number(), 2);
}

#[test]
fn stepping_aside_stamps_the_end_and_releases_the_ticket_lock() {
    let harness = Harness::new();
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    let issued = place.ticket().to_string();
    harness.clock.advance(secs(5));

    let position = place.step_aside(None, &harness.deadline());

    assert_eq!(position.get(), 1);
    assert!(!harness.scratch.ticket_is_held(&issued));
    let record = harness.scratch.record(&issued).expect("a record");
    assert_eq!(millis_of(&record, "ended_ms"), Some(wall_millis(&harness)));
}

#[test]
fn leaving_removes_the_record_and_its_lock() {
    let harness = Harness::new();
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));

    place.leave(&harness.deadline());

    assert_eq!(harness.scratch.queue_names(), ["queue.lock"]);
}

#[test]
fn a_resumed_ticket_keeps_its_number_and_clears_its_end() {
    let harness = Harness::new();
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    let issued = place.ticket().to_string();
    let issued_ms = wall_millis(&harness);
    place.step_aside(None, &harness.deadline());
    harness.clock.advance(secs(10));

    let resumed = queued(join(&harness, &queue, Some(&issued)));

    assert_eq!(resumed.ticket().to_string(), issued);
    let record = harness.scratch.record(&issued).expect("a record");
    assert_eq!(millis_of(&record, "issued_ms"), Some(issued_ms));
    assert_eq!(
        millis_of(&record, "presented_ms"),
        Some(wall_millis(&harness))
    );
    assert!(record["ended_ms"].is_null());
}

#[test]
fn a_resumed_lookup_ticket_keeps_its_number_and_its_versioned_id() {
    let harness = Harness::new();
    let queue = harness.queue();
    let lookup = Binding::of(&ArxivRequest::Lookup(
        ArxivId::parse("2608.21129v2").expect("an arXiv ID"),
    ));
    let place = queued(join_as(&harness, &queue, &lookup, None));
    let issued = place.ticket().to_string();
    let issued_ms = wall_millis(&harness);
    place.step_aside(None, &harness.deadline());
    harness.clock.advance(secs(10));

    let resumed = queued(join_as(&harness, &queue, &lookup, Some(&issued)));

    assert_eq!(resumed.ticket().to_string(), issued);
    let record = harness.scratch.record(&issued).expect("a record");
    assert_eq!(millis_of(&record, "issued_ms"), Some(issued_ms));
    assert_eq!(record["verb"], "lookup");
    assert_eq!(record["id"], "2608.21129v2");
}

fn seed_neighbours(harness: &Harness) {
    harness.seed_absent("2-bbbbbb", 600, 400);
    harness.seed(
        "3-cccccc",
        &TicketRecord {
            binding: GRAPHS,
            issued: harness.clock.ago(secs(600)),
            presented: harness.clock.ago(secs(500)),
            ended: None,
            last_retryable: None,
        },
    );
    harness.scratch.seed_queue_lock();
}

#[test]
fn a_mismatch_leaves_every_queue_file_unchanged() {
    let harness = Harness::new();
    seed_neighbours(&harness);
    harness.seed_absent("5-eeeeee", 20, 10);
    let before = harness.scratch.queue_files();
    let queue = harness.queue();

    let joining = queue.join(
        &search("graphs", "5"),
        Some(&ticket("5-eeeeee")),
        &harness.deadline(),
        SPACING,
    );

    assert!(matches!(
        joining,
        Joining::Rejected(TicketRejection::Mismatch { .. })
    ));
    assert_eq!(harness.scratch.queue_files(), before);
}

#[test]
fn an_already_live_rejection_leaves_every_queue_file_unchanged() {
    let harness = Harness::new();
    seed_neighbours(&harness);
    let _held = harness.seed_live("5-eeeeee", 20);
    let before = harness.scratch.queue_files();
    let queue = harness.queue();

    let joining = join(&harness, &queue, Some("5-eeeeee"));

    assert!(matches!(
        joining,
        Joining::Rejected(TicketRejection::AlreadyLive(_))
    ));
    assert_eq!(harness.scratch.queue_files(), before);
}

#[test]
fn a_ticket_held_elsewhere_is_already_live() {
    let harness = Harness::new();
    let _held = harness.seed_live("4-dddddd", 20);
    let queue = harness.queue();

    let Joining::Rejected(TicketRejection::AlreadyLive(live)) =
        join(&harness, &queue, Some("4-dddddd"))
    else {
        panic!("expected an already-live rejection");
    };

    assert_eq!(live, ticket("4-dddddd"));
}

#[test]
fn a_dropped_place_is_stamped_by_the_next_join() {
    let harness = Harness::new();
    let queue = harness.queue();
    let killed = queued(join(&harness, &queue, None));
    let issued = killed.ticket().to_string();
    drop(killed);
    harness.clock.advance(secs(20));

    let _next = queued(join(&harness, &queue, None));

    let record = harness.scratch.record(&issued).expect("a record");
    assert_eq!(millis_of(&record, "ended_ms"), Some(wall_millis(&harness)));
}

#[test]
fn asking_is_front_writes_nothing_and_takes_no_queue_lock() {
    let harness = Harness::new();
    let queue = harness.queue();
    let _first = queued(join(&harness, &queue, None));
    let second = queued(join(&harness, &queue, None));
    let _queue_lock = harness.scratch.hold_queue_lock();
    let before = harness.scratch.queue_files();

    assert!(!second.is_front());
    assert_eq!(harness.scratch.queue_files(), before);
}

#[test]
fn the_front_skips_a_live_ticket_ahead_once_it_is_abandoned() {
    let harness = Harness::new();
    let _held = harness.seed_live("1-aaaaaa", 1000);
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    assert!(!place.is_front());

    harness.clock.advance(secs(1));

    assert!(place.is_front());
}

#[test]
fn the_front_skips_an_absent_ticket_ahead() {
    let harness = Harness::new();
    harness.seed_absent("1-aaaaaa", 20, 10);
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, None));

    assert_eq!(place.ticket().number(), 2);
    assert!(place.is_front());
}

#[test]
fn the_front_waits_for_a_live_ticket_ahead() {
    let harness = Harness::new();
    let _held = harness.seed_live("1-aaaaaa", 10);
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, None));

    assert!(!place.is_front());
}

#[test]
fn a_ticket_ended_301_s_ago_is_pruned_and_rejoins_at_the_back() {
    let harness = Harness::new();
    harness.seed_absent("1-aaaaaa", 400, 301);
    let _held = harness.seed_live("4-dddddd", 10);
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, Some("1-aaaaaa")));

    assert_eq!(place.ticket().number(), 5);
    assert!(harness.scratch.record("1-aaaaaa").is_none());
    assert!(!harness
        .scratch
        .queue_names()
        .contains(&"1-aaaaaa.lock".to_owned()));
}

#[test]
fn a_ticket_over_the_cap_is_removed_and_reported_over_cap() {
    let harness = Harness::new();
    harness.seed_absent("1-aaaaaa", 901, 10);
    harness.scratch.seed_lock("1-aaaaaa");
    let queue = harness.queue();

    let joining = join(&harness, &queue, Some("1-aaaaaa"));

    let Joining::OverCap {
        ticket: over,
        last_retryable,
    } = joining
    else {
        panic!("expected over the cap");
    };
    assert_eq!(over, ticket("1-aaaaaa"));
    assert_eq!(last_retryable, None);
    assert_eq!(harness.scratch.queue_names(), ["queue.lock"]);
}

#[test]
fn a_reason_recorded_by_step_aside_is_carried_by_a_later_over_cap() {
    let harness = Harness::new();
    let queue = harness.queue();
    let issued = queued(join(&harness, &queue, None)).ticket().to_string();
    for _ in 0..3 {
        harness.clock.advance(secs(299));
        let place = queued(join(&harness, &queue, Some(&issued)));
        place.step_aside(Some(Reason::UpstreamError), &harness.deadline());
    }
    harness.clock.advance(secs(104));

    let joining = join(&harness, &queue, Some(&issued));

    let Joining::OverCap { last_retryable, .. } = joining else {
        panic!("expected over the cap");
    };
    assert_eq!(last_retryable, Some(Reason::UpstreamError));
    assert_eq!(harness.scratch.queue_names(), ["queue.lock"]);
}

#[test]
fn a_resume_clears_the_last_retryable_reason() {
    let harness = Harness::new();
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    let issued = place.ticket().to_string();
    place.step_aside(Some(Reason::UpstreamError), &harness.deadline());
    let stepped = harness.scratch.record(&issued).expect("a record");
    assert_eq!(stepped["last_retryable"], "upstream_error");
    harness.clock.advance(secs(10));

    let _resumed = queued(join(&harness, &queue, Some(&issued)));

    let record = harness.scratch.record(&issued).expect("a record");
    assert!(record["last_retryable"].is_null());
}

#[test]
fn a_corrupt_record_whose_lock_is_free_is_reported_and_removed() {
    let harness = Harness::new();
    harness.scratch.seed_raw_record(
        "3-cccccc.json",
        b"{not json",
        harness.clock.ago(secs(5)),
    );
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, None));

    assert_eq!(place.ticket().number(), 4);
    assert!(harness.scratch.record_bytes("3-cccccc").is_none());
    assert!(harness
        .reported()
        .iter()
        .any(|line| line.contains("3-cccccc.json")));
}

#[test]
fn a_corrupt_record_whose_lock_is_held_is_reported_and_kept() {
    let harness = Harness::new();
    harness.scratch.seed_raw_record(
        "3-cccccc.json",
        b"{not json",
        harness.clock.ago(secs(5)),
    );
    let _held = harness.scratch.hold_ticket("3-cccccc");
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, None));

    assert!(!place.is_front());
    assert!(harness.scratch.record_bytes("3-cccccc").is_some());
    assert!(harness
        .reported()
        .iter()
        .any(|line| line.contains("3-cccccc.json")));
}

fn seed_corrupt_record_behind_an_unprobeable_lock(harness: &Harness) {
    harness.scratch.seed_raw_record(
        "3-cccccc.json",
        b"{not json",
        harness.clock.ago(secs(0)),
    );
    std::fs::create_dir(harness.scratch.queue_path("3-cccccc.lock"))
        .expect("an unprobeable lock");
}

#[test]
fn a_corrupt_record_behind_an_unprobeable_lock_holds_the_front_for_one_invocation_after_its_last_write(
) {
    let harness = Harness::new();
    seed_corrupt_record_behind_an_unprobeable_lock(&harness);
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    harness.clock.advance(secs(100));
    assert!(!place.is_front());

    harness.clock.advance(secs(1));

    assert!(place.is_front());
}

#[test]
fn a_corrupt_record_behind_an_unprobeable_lock_is_removed_once_abandoned_by_its_age(
) {
    let harness = Harness::new();
    seed_corrupt_record_behind_an_unprobeable_lock(&harness);
    let queue = harness.queue();
    let _first = queued(join(&harness, &queue, None));
    harness.clock.advance(secs(1000));
    let _second = queued(join(&harness, &queue, None));
    assert!(harness.scratch.record_bytes("3-cccccc").is_some());

    harness.clock.advance(secs(1));
    let _third = queued(join(&harness, &queue, None));

    assert!(harness.scratch.record_bytes("3-cccccc").is_none());
}

#[test]
fn an_unreadable_record_ahead_still_holds_the_front_until_abandoned() {
    let harness = Harness::new();
    harness.scratch.seed_raw_record(
        "3-cccccc.json",
        b"{not json",
        harness.clock.ago(secs(0)),
    );
    let _held = harness.scratch.hold_ticket("3-cccccc");
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    harness.clock.advance(secs(1000));
    assert!(!place.is_front());

    harness.clock.advance(secs(1));

    assert!(place.is_front());
}

#[test]
fn a_lock_that_vanishes_mid_probe_counts_as_absent() {
    let harness = Harness::new();
    let held = harness.seed_live("1-aaaaaa", 10);
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    assert!(!place.is_front());

    drop(held);
    std::fs::remove_file(harness.scratch.queue_path("1-aaaaaa.lock"))
        .expect("remove the lock");

    assert!(place.is_front());
}

#[test]
fn two_concurrent_probes_both_read_free() {
    let harness = Harness::new();
    harness.seed_absent("1-aaaaaa", 20, 10);
    let probing = harness.scratch.open_queue_file("1-aaaaaa.lock");
    flock(&probing, FlockOperation::NonBlockingLockShared)
        .expect("a probe's shared lock");
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, None));

    assert!(place.is_front());
}

#[test]
fn a_probe_that_has_unlocked_but_kept_its_file_open_does_not_block_own() {
    let harness = Harness::new();
    harness.seed_absent("1-aaaaaa", 20, 10);
    let probed = harness.scratch.open_queue_file("1-aaaaaa.lock");
    flock(&probed, FlockOperation::NonBlockingLockShared).expect("probe");
    flock(&probed, FlockOperation::Unlock).expect("unlock");
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, Some("1-aaaaaa")));

    assert_eq!(place.ticket(), &ticket("1-aaaaaa"));
}

fn real_time_queue(scratch: &Scratch) -> FileArxivQueue {
    FileArxivQueue::new(
        scratch.queue_dir(),
        Rc::new(SystemClock),
        Rc::new(RecordedDiagnostics::default()),
        Box::new(random_nonce),
    )
}

fn real_time_deadline() -> Deadline {
    Deadline::starting(Instant::now(), secs(100), secs(30))
}

fn hold_shared_for(scratch: &Scratch, ticket: &str, held: Duration) {
    let probing = scratch.open_queue_file(&format!("{ticket}.lock"));
    flock(&probing, FlockOperation::NonBlockingLockShared)
        .expect("a probe's shared lock");
    std::thread::spawn(move || {
        std::thread::sleep(held);
        drop(probing);
    });
}

#[test]
fn own_succeeds_once_a_briefly_held_shared_lock_is_released() {
    let harness = Harness::new();
    harness.seed_absent("1-aaaaaa", 20, 10);
    harness.scratch.seed_lock("1-aaaaaa");
    hold_shared_for(&harness.scratch, "1-aaaaaa", Duration::from_millis(5));
    let queue = real_time_queue(&harness.scratch);

    let joining = queue.join(
        &graphs(),
        Some(&ticket("1-aaaaaa")),
        &real_time_deadline(),
        SPACING,
    );

    assert_eq!(queued(joining).ticket(), &ticket("1-aaaaaa"));
}

fn unheld_ticket(joining: &Joining<'_>) -> Option<String> {
    match joining {
        Joining::Unheld { ticket, .. } => Some(ticket.to_string()),
        _ => None,
    }
}

#[test]
fn own_fails_when_a_shared_lock_outlasts_its_patience() {
    let harness = Harness::new();
    harness.seed(
        "1-aaaaaa",
        &TicketRecord {
            binding: GRAPHS,
            issued: std::time::SystemTime::now() - secs(20),
            presented: std::time::SystemTime::now() - secs(11),
            ended: Some(std::time::SystemTime::now() - secs(10)),
            last_retryable: None,
        },
    );
    harness.scratch.seed_lock("1-aaaaaa");
    hold_shared_for(&harness.scratch, "1-aaaaaa", secs(3));
    let queue = real_time_queue(&harness.scratch);
    let started = Instant::now();

    let joining = queue.join(
        &graphs(),
        Some(&ticket("1-aaaaaa")),
        &real_time_deadline(),
        SPACING,
    );

    assert_eq!(unheld_ticket(&joining).as_deref(), Some("1-aaaaaa"));
    assert!(started.elapsed() < Duration::from_millis(1500));
    assert!(harness.scratch.queue_lock_is_free());
}

#[test]
fn a_record_of_another_schema_version_is_not_parsed_as_this_one() {
    let harness = Harness::new();
    let at = support::millis(harness.clock.ago(secs(10)));
    harness.scratch.seed_raw_record(
        "3-cccccc.json",
        format!(
            "{{\"schema_version\":2,\"issued_ms\":{at},\"presented_ms\":{at},\
             \"ended_ms\":{at},\"last_retryable\":null,{GRAPHS}}}"
        )
        .as_bytes(),
        harness.clock.ago(secs(10)),
    );
    let _held = harness.scratch.hold_ticket("3-cccccc");
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, Some("3-cccccc")));

    assert_eq!(place.ticket().number(), 4);
    assert!(harness.scratch.record_bytes("3-cccccc").is_some());
}

#[test]
fn a_file_whose_name_is_not_a_ticket_is_ignored() {
    let harness = Harness::new();
    let at = harness.clock.ago(secs(5000));
    harness.scratch.seed_raw_record("notes.json", b"{}", at);
    harness.scratch.seed_raw_record("99.json", b"{}", at);
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, None));

    assert_eq!(place.ticket().number(), 1);
    let names = harness.scratch.queue_names();
    assert!(names.contains(&"notes.json".to_owned()), "{names:?}");
    assert!(names.contains(&"99.json".to_owned()), "{names:?}");
}

#[test]
fn an_orphan_ticket_lock_that_probes_free_is_removed() {
    let harness = Harness::new();
    harness.scratch.seed_lock("7-aaaaaa");
    let queue = harness.queue();

    let place = queued(join(&harness, &queue, None));

    assert_eq!(place.ticket().number(), 1);
    assert!(!harness
        .scratch
        .queue_names()
        .contains(&"7-aaaaaa.lock".to_owned()));
}

#[test]
fn a_ticket_whose_lock_cannot_be_probed_holds_the_front_for_one_invocation_after_its_presentation(
) {
    let harness = Harness::new();
    harness.seed(
        "1-aaaaaa",
        &TicketRecord {
            binding: GRAPHS,
            issued: harness.clock.ago(secs(0)),
            presented: harness.clock.ago(secs(0)),
            ended: None,
            last_retryable: None,
        },
    );
    std::fs::create_dir(harness.scratch.queue_path("1-aaaaaa.lock"))
        .expect("an unprobeable lock");
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    harness.clock.advance(secs(100));
    assert!(!place.is_front());

    harness.clock.advance(secs(1));

    assert!(place.is_front());
}

#[test]
fn an_unprobeable_lock_is_reported_once_with_its_error() {
    let harness = Harness::new();
    harness.seed_live("1-aaaaaa", 0);
    let lock = harness.scratch.queue_path("1-aaaaaa.lock");
    std::fs::remove_file(&lock).expect("remove the held lock");
    std::fs::create_dir(&lock).expect("an unprobeable lock");
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));

    for _ in 0..3 {
        assert!(!place.is_front());
    }

    let reported: Vec<String> = harness
        .reported()
        .into_iter()
        .filter(|line| line.contains("1-aaaaaa.lock"))
        .collect();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(reported[0].contains("directory"), "{reported:?}");
}

#[test]
fn stepping_aside_or_leaving_without_an_own_record_succeeds() {
    let harness = Harness::new();
    let queue = harness.queue();
    let stepping = queued(join(&harness, &queue, None));
    let leaving = queued(join(&harness, &queue, None));
    for place in [&stepping, &leaving] {
        std::fs::remove_file(
            harness
                .scratch
                .queue_path(&format!("{}.json", place.ticket())),
        )
        .expect("remove the record");
    }

    assert_eq!(stepping.step_aside(None, &harness.deadline()).get(), 1);
    leaving.leave(&harness.deadline());

    assert_eq!(harness.scratch.queue_names(), ["queue.lock"]);
}

#[test]
fn an_unlockable_queue_joins_unqueued_with_a_diagnostic() {
    let harness = Harness::new();
    std::fs::create_dir_all(harness.scratch.queue_path("queue.lock"))
        .expect("an unlockable queue lock");
    let queue = harness.queue();

    let joining = join(&harness, &queue, None);

    assert!(matches!(joining, Joining::Unqueued));
    assert!(harness
        .reported()
        .iter()
        .any(|line| line.contains("queue.lock")));
}

#[cfg(unix)]
#[test]
fn a_failed_record_write_leaves_a_resumed_ticket_unheld_and_its_lock_free() {
    use std::os::unix::fs::PermissionsExt as _;

    let harness = Harness::new();
    harness.seed_absent("1-aaaaaa", 20, 10);
    harness.scratch.seed_lock("1-aaaaaa");
    harness.scratch.seed_queue_lock();
    let record = harness.scratch.record_bytes("1-aaaaaa");
    let directory = harness.scratch.queue_directory();
    let read_only = |mode| {
        std::fs::set_permissions(
            &directory,
            std::fs::Permissions::from_mode(mode),
        )
        .expect("chmod the queue");
    };
    read_only(0o555);
    let probe = directory.join("probe");
    if std::fs::write(&probe, b"x").is_ok() {
        read_only(0o755);
        eprintln!("skipped: a read-only directory accepts writes, as for root");
        return;
    }
    let queue = harness.queue();

    let joining = join(&harness, &queue, Some("1-aaaaaa"));
    let unheld = unheld_ticket(&joining);
    drop(joining);
    read_only(0o755);

    assert_eq!(unheld.as_deref(), Some("1-aaaaaa"));
    assert!(!harness.reported().is_empty());
    assert!(!harness.scratch.ticket_is_held("1-aaaaaa"));
    assert_eq!(harness.scratch.record_bytes("1-aaaaaa"), record);
}

#[test]
fn a_queue_lock_held_past_the_serving_window_joins_unqueued() {
    let harness = Harness::new();
    let _queue_lock = harness.scratch.hold_queue_lock();
    let queue = harness.queue();
    let deadline = harness.deadline();

    let joining = queue.join(&graphs(), None, &deadline, SPACING);

    assert!(matches!(joining, Joining::Unqueued));
    assert!(deadline.admits_attempt_after(harness.clock.now(), SPACING));
}

#[test]
fn a_queue_lock_held_past_the_deadline_leaves_the_record_for_stamping() {
    let harness = Harness::new();
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    let issued = place.ticket().to_string();
    let queue_lock = harness.scratch.hold_queue_lock();

    place.leave(&harness.deadline());

    assert!(!harness.scratch.ticket_is_held(&issued));
    assert!(harness.scratch.record(&issued).is_some());
    drop(queue_lock);
    let _next = queued(join(&harness, &queue, None));
    let record = harness.scratch.record(&issued).expect("a record");
    assert_eq!(millis_of(&record, "ended_ms"), Some(wall_millis(&harness)));
}

#[test]
fn a_queue_lock_held_past_the_deadline_leaves_step_aside_unstamped() {
    let harness = Harness::new();
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    let issued = place.ticket().to_string();
    let queue_lock = harness.scratch.hold_queue_lock();

    let position = place.step_aside(None, &harness.deadline());

    assert_eq!(position.get(), 1);
    assert!(!harness.scratch.ticket_is_held(&issued));
    assert!(
        harness.scratch.record(&issued).expect("a record")["ended_ms"]
            .is_null()
    );
    drop(queue_lock);
    let _next = queued(join(&harness, &queue, None));
    let record = harness.scratch.record(&issued).expect("a record");
    assert_eq!(millis_of(&record, "ended_ms"), Some(wall_millis(&harness)));
}

#[test]
fn an_unlistable_queue_reports_once_and_is_front() {
    let harness = Harness::new();
    let queue = harness.queue();
    let place = queued(join(&harness, &queue, None));
    let directory = harness.scratch.queue_directory();
    let moved = directory.with_file_name("arxiv-queue-moved");
    std::fs::rename(&directory, &moved).expect("move the queue away");
    std::fs::write(&directory, b"not a directory").expect("block the queue");
    let before = harness.reported().len();

    assert!(place.is_front());
    assert!(place.is_front());

    assert_eq!(harness.reported().len(), before + 1);
    std::fs::remove_file(&directory).expect("unblock the queue");
    std::fs::rename(&moved, &directory).expect("restore the queue");
}

#[test]
fn a_failed_prune_is_reported_and_the_pass_continues() {
    let harness = Harness::new();
    let stale = harness.scratch.queue_path(".tmp-stuck");
    std::fs::create_dir_all(stale.join("inside")).expect("an unremovable temp");
    support::set_modified(&stale, harness.clock.ago(secs(101)));
    harness.seed_absent("2-bbbbbb", 600, 400);
    let queue = harness.queue();

    let _place = queued(join(&harness, &queue, None));

    assert!(harness.scratch.record("2-bbbbbb").is_none());
    assert!(harness
        .reported()
        .iter()
        .any(|line| line.contains(".tmp-stuck")));
}

#[test]
fn a_stale_temp_file_is_removed_and_a_fresh_one_kept() {
    let harness = Harness::new();
    harness
        .scratch
        .seed_temp_file(".tmp-stale", harness.clock.ago(secs(101)));
    harness
        .scratch
        .seed_temp_file(".tmp-fresh", harness.clock.ago(secs(99)));
    let queue = harness.queue();

    let _place = queued(join(&harness, &queue, None));

    let names = harness.scratch.queue_names();
    assert!(!names.contains(&".tmp-stale".to_owned()), "{names:?}");
    assert!(names.contains(&".tmp-fresh".to_owned()), "{names:?}");
}

#[test]
fn concurrent_joins_issue_distinct_consecutive_numbers() {
    let scratch = Scratch::new();
    let root = scratch.root_path();
    let queue_directory = scratch.queue_directory();
    let (issued, numbers) = std::sync::mpsc::channel();

    std::thread::scope(|joiners| {
        for _ in 0..8 {
            let issued = issued.clone();
            let (root, queue_directory) = (&root, &queue_directory);
            joiners.spawn(move || {
                let queue = FileArxivQueue::new(
                    research_adapters::scratch::ScratchDir::new(
                        root,
                        queue_directory,
                    ),
                    Rc::new(SystemClock),
                    Rc::new(RecordedDiagnostics::default()),
                    Box::new(random_nonce),
                );
                let place = queued(queue.join(
                    &graphs(),
                    None,
                    &real_time_deadline(),
                    SPACING,
                ));
                issued.send(place.ticket().number()).expect("send");
            });
        }
    });
    drop(issued);

    let mut numbers: Vec<u64> = numbers.into_iter().collect();
    numbers.sort_unstable();
    assert_eq!(numbers, (1..=8).collect::<Vec<_>>());
    let records = scratch
        .queue_names()
        .iter()
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "json")
        })
        .count();
    assert_eq!(records, 8);
}

#[test]
fn a_live_ticket_crossing_900_s_survives_housekeeping() {
    let harness = Harness::new();
    harness.seed_absent("1-aaaaaa", 899, 10);
    harness.scratch.seed_lock("1-aaaaaa");
    let queue = harness.queue();
    let crossing = queued(join(&harness, &queue, Some("1-aaaaaa")));
    harness.clock.advance(secs(51));

    let _later = queued(join(&harness, &queue, None));

    assert!(crossing.is_front());
    assert!(harness.scratch.record("1-aaaaaa").is_some());
    assert!(harness.scratch.ticket_is_held("1-aaaaaa"));
}

#[test]
fn a_dropped_place_is_no_longer_ahead_at_once() {
    let harness = Harness::new();
    let queue = harness.queue();
    let ahead = queued(join(&harness, &queue, None));
    let behind = queued(join(&harness, &queue, None));
    assert!(!behind.is_front());

    drop(ahead);

    assert!(behind.is_front());
}

#[test]
fn three_places_are_front_in_issue_order_whichever_asks_first() {
    let harness = Harness::new();
    let queue = harness.queue();
    let first = queued(join(&harness, &queue, None));
    let second = queued(join(&harness, &queue, None));
    let third = queued(join(&harness, &queue, None));

    assert!(!third.is_front());
    assert!(first.is_front());
    assert!(!second.is_front());
    first.leave(&harness.deadline());

    assert!(second.is_front());
    assert!(!third.is_front());
}

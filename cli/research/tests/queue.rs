#![allow(clippy::expect_used, clippy::panic)]

use std::cell::Cell;
use std::time::Duration;
use std::time::SystemTime;

use research::sources::classify::Reason;
use research::sources::queue::is_abandoned;
use research::sources::queue::is_front_given;
use research::sources::queue::Argument;
use research::sources::queue::Binding;
use research::sources::queue::Contender;
use research::sources::queue::Nonce;
use research::sources::queue::Presence;
use research::sources::queue::Presentation;
use research::sources::queue::Queue;
use research::sources::queue::Standing;
use research::sources::queue::Ticket;
use research::sources::queue::TicketRejection;
use research::sources::request::parse_ticket;
use research::sources::request::ArxivId;
use research::sources::request::ArxivQuery;
use research::sources::request::ArxivRequest;
use research::sources::request::Family;
use research::sources::request::Limit;
use research::sources::request::RequestError;

const fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + secs(1_800_000_000)
}

fn ago(seconds: u64) -> SystemTime {
    now() - secs(seconds)
}

fn ticket(text: &str) -> Ticket {
    Ticket::parse(text).expect("a well-formed ticket")
}

fn search(query: &str, limit: &str) -> Binding {
    Binding::of(&ArxivRequest::Search {
        query: ArxivQuery::parse(query).expect("a query"),
        limit: Limit::parse(limit).expect("a limit"),
    })
}

fn lookup(id: &str) -> Binding {
    Binding::of(&ArxivRequest::Lookup(ArxivId::parse(id).expect("an ID")))
}

fn graphs() -> Binding {
    search("graphs", "10")
}

/// A live ticket issued and last presented `issued` seconds ago, until a
/// builder method says otherwise.
struct Seed {
    ticket: Ticket,
    binding: Binding,
    issued_at: SystemTime,
    presented_at: SystemTime,
    last_retryable: Option<Reason>,
    presence: Presence,
}

impl Seed {
    fn new(text: &str, issued: u64) -> Self {
        Self {
            ticket: ticket(text),
            binding: graphs(),
            issued_at: ago(issued),
            presented_at: ago(issued),
            last_retryable: None,
            presence: Presence::Live,
        }
    }

    fn presented(mut self, seconds_ago: u64) -> Self {
        self.presented_at = ago(seconds_ago);
        self
    }

    fn ended(mut self, seconds_ago: u64) -> Self {
        self.presence = Presence::Absent {
            ended_at: Some(ago(seconds_ago)),
        };
        self
    }

    const fn ended_at(mut self, at: SystemTime) -> Self {
        self.presence = Presence::Absent { ended_at: Some(at) };
        self
    }

    const fn unstamped(mut self) -> Self {
        self.presence = Presence::Absent { ended_at: None };
        self
    }

    const fn unprobeable(mut self) -> Self {
        self.presence = Presence::Unknown;
        self
    }

    fn bound_to(mut self, binding: Binding) -> Self {
        self.binding = binding;
        self
    }

    const fn retried(mut self, reason: Reason) -> Self {
        self.last_retryable = Some(reason);
        self
    }

    fn standing(self) -> Standing {
        Standing {
            ticket: self.ticket,
            binding: self.binding,
            issued_at: self.issued_at,
            presented_at: self.presented_at,
            last_retryable: self.last_retryable,
            presence: self.presence,
        }
    }
}

fn queue(seeds: Vec<Seed>) -> Queue {
    Queue::new(seeds.into_iter().map(Seed::standing).collect(), Vec::new())
}

fn present(queue: &Queue, presented: &str, binding: &Binding) -> Presentation {
    queue.present(Some(&ticket(presented)), binding, now())
}

const fn live(issued_at: SystemTime) -> Contender {
    Contender {
        presence: Presence::Live,
        issued_at,
        presented_at: issued_at,
    }
}

const fn absent(issued_at: SystemTime) -> Contender {
    Contender {
        presence: Presence::Absent { ended_at: None },
        issued_at,
        presented_at: issued_at,
    }
}

const fn unprobeable(
    issued_at: SystemTime,
    presented_at: SystemTime,
) -> Contender {
    Contender {
        presence: Presence::Unknown,
        issued_at,
        presented_at,
    }
}

#[test]
fn tickets_round_trip_through_their_text_form() {
    for text in ["42-9f1c2a", "1-000000", "18446744073709551615-abcdef"] {
        assert_eq!(ticket(text).to_string(), text);
    }
    assert_eq!(
        Ticket::issue(7, Nonce::from_hex("a1b2c3").expect("a nonce"))
            .to_string(),
        "7-a1b2c3"
    );
}

#[test]
fn a_nonce_keeps_the_low_24_bits_it_is_made_from() {
    assert_eq!(
        Nonce::from_bits(0xff12_3456),
        Nonce::from_hex("123456").expect("a nonce")
    );
}

#[test]
fn malformed_tickets_are_refused() {
    for text in [
        "42",
        "42-XYZ",
        "-1-abcdef",
        "",
        "42-abcdef;x",
        "42-ABCDEF",
        "42-abc",
        "42-abcdef0",
        "+42-abcdef",
        "042-abcdef",
        "18446744073709551616-abcdef",
    ] {
        assert_eq!(
            Ticket::parse(text),
            Err(RequestError::TicketMalformed(text.to_owned())),
            "{text:?}"
        );
    }
}

#[test]
fn a_malformed_ticket_names_its_error_code() {
    let refused = parse_ticket(Family::Arxiv, Some("42")).expect_err("refused");

    assert!(refused.to_string().starts_with("E_ARXIV_TICKET_MALFORMED"));
}

#[test]
fn a_ticket_on_an_openalex_call_is_not_queued() {
    let refused =
        parse_ticket(Family::OpenAlex, Some("42-9f1c2a")).expect_err("refused");

    assert_eq!(refused, RequestError::TicketNotQueued(Family::OpenAlex));
    assert_eq!(
        refused.to_string(),
        "E_RESEARCH_USAGE: --ticket applies only to arxiv fetches"
    );
}

#[test]
fn an_absent_ticket_flag_parses_to_no_ticket() {
    assert_eq!(parse_ticket(Family::Arxiv, None), Ok(None));
    assert_eq!(parse_ticket(Family::OpenAlex, None), Ok(None));
    assert_eq!(
        parse_ticket(Family::Arxiv, Some("42-9f1c2a")),
        Ok(Some(ticket("42-9f1c2a")))
    );
}

#[test]
fn tickets_are_ordered_by_number_alone() {
    assert!(ticket("3-ffffff").is_ahead_of(&ticket("4-000000")));
    assert!(!ticket("4-000000").is_ahead_of(&ticket("3-ffffff")));
    assert_ne!(ticket("3-aaaaaa"), ticket("3-bbbbbb"));
}

#[test]
fn differing_names_each_mismatched_argument() {
    assert_eq!(
        search("attention heads", "10").differing(&search("graphs", "5")),
        [Argument::Query, Argument::Limit]
    );
    assert!(graphs().differing(&graphs()).is_empty());
    assert_eq!(
        lookup("2608.21129").differing(&lookup("2608.21130")),
        [Argument::Id]
    );
}

#[test]
fn a_search_and_a_lookup_differ_only_by_verb() {
    assert_eq!(graphs().differing(&lookup("2608.21129")), [Argument::Verb]);
}

#[test]
fn an_argument_is_named_as_the_profile_spells_it() {
    assert_eq!(
        [
            Argument::Verb,
            Argument::Query,
            Argument::Id,
            Argument::Limit
        ]
        .map(Argument::code),
        ["verb", "query", "id", "--limit"]
    );
}

#[test]
fn no_ticket_joins() {
    assert_eq!(
        queue(vec![Seed::new("1-aaaaaa", 10)]).present(None, &graphs(), now()),
        Presentation::Join
    );
}

#[test]
fn an_unknown_ticket_joins() {
    assert_eq!(
        present(
            &queue(vec![Seed::new("1-aaaaaa", 10)]),
            "2-bbbbbb",
            &graphs()
        ),
        Presentation::Join
    );
}

#[test]
fn a_ticket_whose_number_is_reissued_under_another_nonce_joins() {
    let queue = queue(vec![
        Seed::new("3-aaaaaa", 10).bound_to(lookup("2608.21129"))
    ]);

    assert_eq!(present(&queue, "3-bbbbbb", &graphs()), Presentation::Join);
}

#[test]
fn an_expired_ticket_with_other_arguments_joins_afresh() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 400).ended(301)]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &lookup("2608.21129")),
        Presentation::Join
    );
}

#[test]
fn a_live_ticket_presented_again_is_already_live() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 10)]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::Rejected(TicketRejection::AlreadyLive(ticket(
            "3-aaaaaa"
        )))
    );
}

#[test]
fn a_mismatched_live_ticket_is_a_mismatch() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 10)]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &search("graphs", "5")),
        Presentation::Rejected(TicketRejection::Mismatch {
            ticket: ticket("3-aaaaaa"),
            issued: graphs(),
            differing: vec![Argument::Limit],
        })
    );
}

#[test]
fn a_mismatched_ticket_past_the_cap_is_a_mismatch() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 950).ended(10)]);

    assert!(matches!(
        present(&queue, "3-aaaaaa", &search("graphs", "5")),
        Presentation::Rejected(TicketRejection::Mismatch { .. })
    ));
}

#[test]
fn a_live_ticket_past_the_cap_is_already_live() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 950)]);

    assert!(matches!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::Rejected(TicketRejection::AlreadyLive(_))
    ));
}

#[test]
fn a_ticket_ended_exactly_300_s_ago_resumes() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 400).ended(300)]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::Resume(ticket("3-aaaaaa"))
    );
}

#[test]
fn a_ticket_ended_301_s_ago_joins_afresh() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 400).ended(301)]);

    assert_eq!(present(&queue, "3-aaaaaa", &graphs()), Presentation::Join);
}

#[test]
fn a_ticket_issued_exactly_900_s_ago_resumes() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 900).ended(10)]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::Resume(ticket("3-aaaaaa"))
    );
}

#[test]
fn a_ticket_issued_901_s_ago_is_over_the_cap() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 901).ended(10)]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::OverCap {
            ticket: ticket("3-aaaaaa"),
            last_retryable: None,
        }
    );
}

#[test]
fn an_over_cap_ticket_carries_its_last_retryable_reason() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 901)
        .ended(10)
        .retried(Reason::UpstreamError)]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::OverCap {
            ticket: ticket("3-aaaaaa"),
            last_retryable: Some(Reason::UpstreamError),
        }
    );
}

#[test]
fn an_expired_ticket_past_the_cap_joins_rather_than_hitting_the_cap() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 950).ended(400)]);

    assert_eq!(present(&queue, "3-aaaaaa", &graphs()), Presentation::Join);
}

#[test]
fn an_unstamped_ticket_is_judged_by_its_effective_end() {
    let queue =
        queue(vec![Seed::new("3-aaaaaa", 950).presented(500).unstamped()]);

    assert_eq!(present(&queue, "3-aaaaaa", &graphs()), Presentation::Join);
}

#[test]
fn an_unstamped_ticket_is_stamped_no_later_than_one_invocation_after_its_presentation(
) {
    let queue =
        queue(vec![Seed::new("3-aaaaaa", 600).presented(500).unstamped()]);

    assert_eq!(queue.stamps(now()), [(ticket("3-aaaaaa"), ago(400))]);
    assert_eq!(queue.expired(now()), [ticket("3-aaaaaa")]);
}

#[test]
fn an_unstamped_ticket_presented_moments_ago_is_stamped_now() {
    let queue =
        queue(vec![Seed::new("3-aaaaaa", 600).presented(5).unstamped()]);

    assert_eq!(queue.stamps(now()), [(ticket("3-aaaaaa"), now())]);
    assert!(queue.expired(now()).is_empty());
}

#[test]
fn only_absent_unstamped_tickets_are_stamped() {
    let queue = queue(vec![
        Seed::new("1-aaaaaa", 600).presented(5),
        Seed::new("2-bbbbbb", 600).ended(5),
    ]);

    assert!(queue.stamps(now()).is_empty());
}

#[test]
fn position_counts_every_unexpired_ticket_ahead_live_or_absent() {
    let queue = queue(vec![
        Seed::new("1-aaaaaa", 600).ended(400),
        Seed::new("2-bbbbbb", 200).ended(100),
        Seed::new("3-cccccc", 100),
        Seed::new("4-dddddd", 50),
        Seed::new("5-eeeeee", 10),
    ]);

    assert_eq!(queue.position_of(&ticket("4-dddddd"), now()).get(), 3);
}

#[test]
fn an_absent_ticket_ahead_does_not_hold_the_front() {
    assert!(is_front_given([absent(ago(20))], now()));
    assert!(!is_front_given([absent(ago(20)), live(ago(10))], now()));
}

#[test]
fn a_resumed_ticket_outranks_every_later_ticket() {
    let resumed = ticket("2-bbbbbb");

    assert!(resumed.is_ahead_of(&ticket("3-cccccc")));
    assert!(is_front_given([absent(ago(400))], now()));
    assert!(!is_front_given([live(ago(400))], now()));
}

#[test]
fn the_next_number_follows_the_largest_present() {
    assert_eq!(queue(Vec::new()).next_number(), 1);
    assert_eq!(
        queue(vec![Seed::new("4-aaaaaa", 10), Seed::new("2-bbbbbb", 10)])
            .next_number(),
        5
    );
}

#[test]
fn the_next_number_follows_an_absent_highest_ticket() {
    let queue = queue(vec![
        Seed::new("2-aaaaaa", 10),
        Seed::new("7-bbbbbb", 100).ended(50),
    ]);

    assert_eq!(queue.next_number(), 8);
}

#[test]
fn the_next_number_follows_an_unparsed_highest_ticket() {
    let queue = Queue::new(
        vec![Seed::new("2-aaaaaa", 10).standing()],
        vec![ticket("9-cccccc")],
    );

    assert_eq!(queue.next_number(), 10);
}

#[test]
fn the_front_is_decided_by_the_first_live_unabandoned_contender() {
    assert!(is_front_given([], now()));
    assert!(is_front_given([absent(ago(10)), absent(ago(5))], now()));
    assert!(!is_front_given([absent(ago(10)), live(ago(5))], now()));
}

#[test]
fn is_front_given_reads_no_contender_past_the_first_that_blocks() {
    let read = Cell::new(0);
    let contenders = [absent(ago(10)), live(ago(5)), live(ago(3))];

    let front = is_front_given(
        contenders.into_iter().inspect(|_| read.set(read.get() + 1)),
        now(),
    );

    assert!(!front);
    assert_eq!(read.get(), 2);
}

#[test]
fn a_live_ticket_issued_1000_s_ago_still_holds_the_front() {
    assert!(!is_front_given([live(ago(1000))], now()));
    assert!(!is_abandoned(ago(1000), now()));
}

#[test]
fn at_1001_s_it_is_abandoned_and_no_longer_holds_the_front() {
    assert!(is_front_given([live(ago(1001))], now()));
    assert!(is_abandoned(ago(1001), now()));
}

#[test]
fn an_unprobeable_contender_holds_the_front_for_one_invocation_after_presentation(
) {
    assert!(!is_front_given([unprobeable(ago(500), ago(100))], now()));
    assert!(is_front_given([unprobeable(ago(500), ago(101))], now()));
}

#[test]
fn an_unprobeable_contender_presented_moments_ago_yields_once_abandoned() {
    assert!(is_front_given([unprobeable(ago(1001), ago(0))], now()));
}

#[test]
fn an_unprobeable_ticket_presented_again_within_one_invocation_is_already_live()
{
    let queue = queue(vec![Seed::new("3-aaaaaa", 500)
        .presented(100)
        .unprobeable()]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::Rejected(TicketRejection::AlreadyLive(ticket(
            "3-aaaaaa"
        )))
    );
}

#[test]
fn an_unprobeable_ticket_presented_again_after_one_invocation_resumes() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 500)
        .presented(101)
        .unprobeable()]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::Resume(ticket("3-aaaaaa"))
    );
}

#[test]
fn an_unprobeable_ticket_expires_300_s_after_its_invocation_could_end() {
    let at_expiry = queue(vec![Seed::new("3-aaaaaa", 500)
        .presented(400)
        .unprobeable()]);
    let past_expiry = queue(vec![Seed::new("3-aaaaaa", 500)
        .presented(401)
        .unprobeable()]);

    assert!(at_expiry.expired(now()).is_empty());
    assert_eq!(past_expiry.expired(now()), [ticket("3-aaaaaa")]);
}

#[test]
fn an_unprobeable_ticket_is_never_stamped() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 500)
        .presented(200)
        .unprobeable()]);

    assert!(queue.stamps(now()).is_empty());
}

#[test]
fn an_abandoned_ticket_expires_300_s_after_abandonment() {
    let at_expiry = queue(vec![Seed::new("3-aaaaaa", 1300)]);
    let past_expiry = queue(vec![Seed::new("3-aaaaaa", 1301)]);

    assert!(at_expiry.expired(now()).is_empty());
    assert_eq!(past_expiry.expired(now()), [ticket("3-aaaaaa")]);
}

#[test]
fn a_live_abandoned_ticket_presented_again_is_not_already_live() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 1200)]);

    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::OverCap {
            ticket: ticket("3-aaaaaa"),
            last_retryable: None,
        }
    );
}

#[test]
fn a_ticket_issued_1050_s_ago_and_ended_400_s_ago_joins_afresh() {
    let queue = queue(vec![Seed::new("3-aaaaaa", 1050).ended(400)]);

    assert_eq!(present(&queue, "3-aaaaaa", &graphs()), Presentation::Join);
}

#[test]
fn a_stamp_later_than_now_neither_expires_nor_passes_the_cap() {
    let ahead = now() + secs(5000);
    let mut seed = Seed::new("3-aaaaaa", 0).ended_at(ahead);
    seed.issued_at = ahead;
    seed.presented_at = ahead;
    let queue = queue(vec![seed]);

    assert!(queue.expired(now()).is_empty());
    assert_eq!(
        present(&queue, "3-aaaaaa", &graphs()),
        Presentation::Resume(ticket("3-aaaaaa"))
    );
}

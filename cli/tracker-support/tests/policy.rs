//! The shared policy both providers inherit: identifier safety, the retry
//! schedule, and the port's body shape.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::time::Duration;

use tracker_support::identifier::IdentifierRefusal;
use tracker_support::{
    identifier_is_safe, port_body, Jitter, RetryPolicy, TransportConfig,
};

fn unescape(raw: &str) -> String {
    let mut out = String::new();
    let mut characters = raw.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('0') => out.push('\0'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(character);
        }
    }
    out
}

fn identifier_cases() -> Vec<(String, Option<IdentifierRefusal>)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/identifiers.txt");
    let raw =
        std::fs::read_to_string(path).expect("the identifier fixture is read");
    raw.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (verdict, candidate) =
                line.split_once('\t').unwrap_or((line, ""));
            let expected = match verdict {
                "accept" => None,
                "empty" => Some(IdentifierRefusal::Empty),
                "control" => Some(IdentifierRefusal::ControlCharacter),
                "separator" => Some(IdentifierRefusal::DocumentSeparator),
                "comment" => Some(IdentifierRefusal::CommentTrigger),
                other => panic!("unrecognised verdict {other}"),
            };
            (unescape(candidate), expected)
        })
        .collect()
}

#[test]
fn identifier_safety_matches_the_committed_fixture_exactly() {
    let cases = identifier_cases();
    assert!(!cases.is_empty(), "the fixture must drive the assertions");

    for (candidate, expected) in cases {
        assert_eq!(
            identifier_is_safe(&candidate).err(),
            expected,
            "identifier {candidate:?}"
        );
    }
}

#[test]
fn the_fixture_covers_every_refusal_and_the_accepting_case() {
    let verdicts: Vec<_> = identifier_cases()
        .into_iter()
        .map(|(_, expected)| expected)
        .collect();
    for refusal in [
        IdentifierRefusal::Empty,
        IdentifierRefusal::ControlCharacter,
        IdentifierRefusal::DocumentSeparator,
        IdentifierRefusal::CommentTrigger,
    ] {
        assert!(
            verdicts.contains(&Some(refusal)),
            "the fixture must exercise {refusal:?}"
        );
    }
    assert!(verdicts.contains(&None));
}

/// Yields a fixed offset, so a delay sequence is asserted as data.
struct SeededJitter {
    offsets: Vec<i64>,
    next: usize,
}

impl SeededJitter {
    const fn new(offsets: Vec<i64>) -> Self {
        Self { offsets, next: 0 }
    }
}

impl Jitter for SeededJitter {
    fn offset(&mut self, spread: u64) -> i64 {
        let offset = self.offsets.get(self.next).copied().unwrap_or(0);
        self.next += 1;
        assert!(
            offset.unsigned_abs() <= spread,
            "the seeded offset must be inside the ±30% spread"
        );
        offset
    }
}

#[test]
fn the_delay_sequence_is_exponential_with_the_seeded_offset() {
    let policy = RetryPolicy::default();
    let mut jitter = SeededJitter::new(vec![0, 0, 0]);

    let delays: Vec<_> = (1..=policy.max_attempts)
        .map(|attempt| policy.delay_for(attempt, None, &mut jitter))
        .collect();

    assert_eq!(
        delays,
        vec![
            Some(Duration::from_secs(1)),
            Some(Duration::from_secs(2)),
            Some(Duration::from_secs(4)),
            None,
        ],
        "four attempts means three delays, then exhaustion"
    );
}

#[test]
fn the_offset_moves_the_delay_within_thirty_percent() {
    let policy = RetryPolicy::default();
    let mut jitter = SeededJitter::new(vec![-1]);

    assert_eq!(
        policy.delay_for(3, None, &mut jitter),
        Some(Duration::from_secs(3)),
        "4s base with a -1s offset"
    );
}

#[test]
fn retry_after_is_honoured_as_a_duration_rather_than_as_a_trigger() {
    let policy = RetryPolicy::default();
    let mut jitter = SeededJitter::new(vec![0]);

    assert_eq!(
        policy.delay_for(1, Some(Duration::from_secs(7)), &mut jitter),
        Some(Duration::from_secs(7)),
        "the hint wins over the 1s the default backoff would have taken"
    );
}

#[test]
fn a_delay_is_clamped_to_the_bash_floor_and_ceiling() {
    let policy = RetryPolicy::default();
    let mut jitter = SeededJitter::new(vec![0, 0]);

    assert_eq!(
        policy.delay_for(1, Some(Duration::from_millis(10)), &mut jitter),
        Some(Duration::from_secs(1))
    );
    assert_eq!(
        policy.delay_for(1, Some(Duration::from_secs(3600)), &mut jitter),
        Some(Duration::from_secs(60))
    );
}

#[test]
fn no_delay_is_offered_once_the_attempts_are_exhausted() {
    let policy = RetryPolicy::default();
    let mut jitter = SeededJitter::new(vec![0]);

    assert_eq!(
        policy.delay_for(4, Some(Duration::from_secs(7)), &mut jitter),
        None,
        "even a Retry-After does not buy a fifth attempt"
    );
}

#[test]
fn the_transport_bounds_are_the_transcribed_ones() {
    let config = TransportConfig::default();

    assert_eq!(config.timeout, Duration::from_secs(30));
    assert_eq!(config.max_response_bytes, 8 * 1024 * 1024);
    assert_eq!(config.max_pages, 20);
}

#[test]
fn the_port_body_carries_exactly_one_trailing_newline() {
    assert_eq!(port_body("summary\nbody"), "summary\nbody\n");
    assert_eq!(port_body("summary\nbody\n"), "summary\nbody\n");
    assert_eq!(port_body(""), "\n");
}

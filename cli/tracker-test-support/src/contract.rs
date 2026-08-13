//! The `RemoteTracker` contract harness, parameterised over implementations.
//!
//! 0204 handed over a gap that neither the port's own types nor
//! `tracker/tests/port.rs`'s trait-object test can close: an implementor
//! that quietly stops implementing an operation once the trait gains a
//! default-bodied method is invisible to both, because Rust has no
//! compile-time assertion that a trait method is required, and an
//! all-through-a-trait-object test provably does not catch it. The only
//! mitigation available is this harness's per-operation behavioural
//! assertions — an implementor inheriting a no-op or panicking default fails
//! them. Recorded here so the next reader does not re-derive it.
//!
//! [`unaccounted_id_is_indeterminate_not_absent`] and
//! [`a_failing_read_is_retryable`] hold obligations the port documents but
//! cannot enforce, and are conditions an implementation must be *induced*
//! into rather than behaviours it exhibits unprompted. That is why every
//! function here takes a [`ContractSubject`] rather than a bare
//! `&dyn RemoteTracker`: truncation and a lost write are configurations of
//! the fake (`RecordingTracker::truncating`, `::losing`), and a real client
//! supplies the same conditions through `unaccountable_id`/`unreadable_id`
//! rather than being asked to misbehave.
//!
//! Every function here panics on a violated property rather than returning
//! a `Result` — the same shape a `#[test]` function itself uses, since these
//! are assertions a caller invokes from inside its own test.
#![allow(clippy::expect_used, clippy::missing_panics_doc)]

use std::fmt::Display;
use std::fmt::Formatter;

use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::RemoteTracker;
use tracker::TrackerError;

/// One implementation under test, plus the ids it can supply the two
/// induced conditions with.
pub trait ContractSubject {
    fn tracker(&self) -> &dyn RemoteTracker;

    /// An id this implementation will report as `indeterminate` rather than
    /// `absent` — a truncated or otherwise incomplete retrieval.
    fn unaccountable_id(&self) -> ExternalId;

    /// An id whose `show` this implementation will fail.
    fn unreadable_id(&self) -> ExternalId;
}

/// Every distinct id in `requested` appears in exactly one of `outcome`'s
/// three vectors.
///
/// Lifted verbatim from `tracker/tests/port.rs`'s own `partitions_totally` —
/// dependency-free, so it is shared rather than duplicated.
pub fn partitions_totally(outcome: &FetchOutcome, requested: &[ExternalId]) {
    let mut reported: Vec<&ExternalId> = outcome
        .found
        .iter()
        .map(|(id, _)| id)
        .chain(outcome.absent.iter())
        .chain(outcome.indeterminate.iter())
        .collect();
    let reported_count = reported.len();
    reported.sort_by_key(|id| id.as_str().to_owned());
    reported.dedup();
    assert_eq!(
        reported_count,
        reported.len(),
        "an id was reported more than once across the three vectors"
    );

    let mut expected: Vec<&ExternalId> = requested.iter().collect();
    expected.sort_by_key(|id| id.as_str().to_owned());
    expected.dedup();
    assert_eq!(
        reported, expected,
        "the partition does not cover exactly the requested ids"
    );
}

/// A created issue can immediately be read back.
pub fn create_then_show_round_trips(subject: &dyn ContractSubject) {
    let tracker = subject.tracker();
    let created = tracker
        .create("Contract title", "Contract body\n", "story")
        .expect("create must succeed for a conformant implementation");
    let issue = tracker
        .show(&created)
        .expect("show must succeed immediately after create");
    assert!(
        !issue.body.is_empty(),
        "a freshly created issue must project a non-empty body"
    );
}

/// `update` replaces an issue's content, not merges into it.
pub fn update_replaces_whole_content(subject: &dyn ContractSubject) {
    let tracker = subject.tracker();
    let id = tracker
        .create("Original title", "Original body\n", "story")
        .expect("create must succeed for a conformant implementation");
    let before = tracker
        .show(&id)
        .expect("show must succeed after create")
        .body;

    tracker
        .update(&id, "Updated title", "Updated body\n")
        .expect("update must succeed for a conformant implementation");
    let after = tracker
        .show(&id)
        .expect("show must succeed after update")
        .body;

    assert_ne!(before, after, "update must replace the issue's content");
}

/// A bulk fetch partitions every requested id exactly once.
pub fn fetch_all_partitions_totally(
    subject: &dyn ContractSubject,
    ids: &[ExternalId],
) {
    let outcome = subject
        .tracker()
        .fetch_all(ids)
        .expect("fetch_all must succeed for a conformant implementation");
    partitions_totally(&outcome, ids);
}

/// An id a bulk retrieval could not account for is `indeterminate`, never
/// `absent` — inferring absence from an incomplete retrieval is what makes a
/// sync delete an issue that still exists.
pub fn unaccounted_id_is_indeterminate_not_absent(
    subject: &dyn ContractSubject,
) {
    let unseen = subject.unaccountable_id();
    let outcome = subject
        .tracker()
        .fetch_all(std::slice::from_ref(&unseen))
        .expect("fetch_all must succeed for a conformant implementation");
    assert!(
        !outcome.absent.contains(&unseen),
        "an id the retrieval could not account for must not be reported absent"
    );
    assert!(
        outcome.indeterminate.contains(&unseen),
        "an id the retrieval could not account for must be reported indeterminate"
    );
}

/// `show`'s error is never `Terminal` — a read mutates nothing, so the
/// terminal class, which means "a mutation may have applied", cannot arise.
pub fn a_failing_read_is_retryable(subject: &dyn ContractSubject) {
    let id = subject.unreadable_id();
    let error = subject
        .tracker()
        .show(&id)
        .expect_err("the configured id must fail to read");
    assert!(
        !matches!(error, TrackerError::Terminal { .. }),
        "a read must never be classified terminal"
    );
}

/// The reason the harness must fail rather than skip when the opt-in
/// environment variable is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractGateError {
    NotOptedIn,
}

impl Display for ContractGateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotOptedIn => write!(
                formatter,
                "ACCELERATOR_TRACKER_CONTRACT=1 is not set — run this \
                 harness through `mise run test:integration:tracker-contract`"
            ),
        }
    }
}

impl std::error::Error for ContractGateError {}

/// The conformance set: properties every implementation must satisfy.
///
/// Excludes the shape-specific obligations
/// ([`unaccounted_id_is_indeterminate_not_absent`],
/// [`a_failing_read_is_retryable`]), which need a subject deliberately
/// configured into the condition they assert rather than one exhibiting it
/// unprompted.
///
/// Errors — never skips — when `ACCELERATOR_TRACKER_CONTRACT` is not `1`, so
/// a dropped or misspelled env var in the task, or a hand run of the
/// profile, cannot make every contract binary exit 0 having asserted
/// nothing. Returns the count of properties executed, which the caller must
/// assert is non-zero: a count assertion alone cannot distinguish "gate
/// closed" from "ran nothing", because both give zero.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn run_all(
    subject: &dyn ContractSubject,
    ids: &[ExternalId],
) -> Result<usize, ContractGateError> {
    if std::env::var("ACCELERATOR_TRACKER_CONTRACT").as_deref() != Ok("1") {
        return Err(ContractGateError::NotOptedIn);
    }

    create_then_show_round_trips(subject);
    update_replaces_whole_content(subject);
    fetch_all_partitions_totally(subject, ids);

    Ok(3)
}

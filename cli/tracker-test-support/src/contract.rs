//! The `RemoteTracker` contract harness, parameterised over implementations.
//!
//! Behavioural assertions are the only available check that an implementor
//! still implements every operation: Rust has no compile-time assertion that
//! a trait method is required, so an implementor inheriting a default body
//! is invisible to both the port's types and a trait-object test.
//!
//! Every function takes a [`ContractSubject`] rather than a bare
//! `&dyn RemoteTracker` because truncation and a lost write are conditions
//! an implementation must be *induced* into: the fake configures itself,
//! and a real client nominates ids through `unaccountable_id`/
//! `unreadable_id` rather than being asked to misbehave.
#![allow(clippy::expect_used, clippy::missing_panics_doc)]

use std::fmt::Display;
use std::fmt::Formatter;

use tracker::CreatePreview;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker::ValidationOutcome;

/// One implementation under test, plus the ids it supplies the two induced
/// conditions with.
pub trait ContractSubject {
    fn tracker(&self) -> &dyn RemoteTracker;

    /// An id this implementation will report as `indeterminate` rather than
    /// `absent` — a truncated or otherwise incomplete retrieval.
    fn unaccountable_id(&self) -> ExternalId;

    /// An id whose `show` this implementation will fail.
    fn unreadable_id(&self) -> ExternalId;

    /// Whether [`unaccountable_id`] is actually reported `indeterminate` by
    /// this subject.
    ///
    /// An implementation whose only indeterminate path is a failed retrieval —
    /// a transport or server failure a live tenant will not produce for a
    /// benign id, reproducible only by a mock — declares `false`. The
    /// nominate-an-indeterminate-id conformance is then skipped for it, because
    /// no id it can name would satisfy the property; the property stays
    /// enforced against such an implementation offline, where a mock forces the
    /// failure. A subject with a structural indeterminate path (an id outside a
    /// scope it cannot see past) keeps the default.
    ///
    /// [`unaccountable_id`]: ContractSubject::unaccountable_id
    fn can_nominate_indeterminate(&self) -> bool {
        true
    }

    /// A scope whose discovery this implementation reports **incomplete** — a
    /// truncated retrieval, a page cap, or a failed query.
    ///
    /// The default is a bare scope. A subject that can only be truncated through
    /// a specific configuration overrides both this and
    /// [`can_induce_truncation`].
    ///
    /// [`can_induce_truncation`]: ContractSubject::can_induce_truncation
    fn truncating_scope(&self) -> SearchScope {
        SearchScope::default()
    }

    /// Whether [`truncating_scope`] actually yields `complete == false` for this
    /// subject.
    ///
    /// A subject with no inducible truncation path — a live tenant that returns
    /// a clean complete result for any benign scope — declares `false`, and
    /// [`timed_conformance`] skips the truncation property for it, exactly as
    /// [`can_nominate_indeterminate`] gates the indeterminate property. The
    /// property stays enforced offline against a mock that forces the cut-off.
    ///
    /// [`truncating_scope`]: ContractSubject::truncating_scope
    /// [`can_nominate_indeterminate`]: ContractSubject::can_nominate_indeterminate
    fn can_induce_truncation(&self) -> bool {
        true
    }

    /// The `kind` this subject's create preview resolves. The default is the
    /// empty string — the tracker's configured default issue type.
    fn preview_kind(&self) -> String {
        String::new()
    }
}

/// Every distinct id in `requested` appears in exactly one of `outcome`'s
/// three vectors.
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

/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn create_then_show_round_trips(
    subject: &dyn ContractSubject,
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    create_then_show_round_trips_property(subject);
    Ok(())
}

/// The assertions, ungated.
///
/// An offline caller with a mock-backed subject enforces them in the default
/// profile. The gate stays on the wrapper: reaching a property directly must
/// not thereby reach a live provider.
///
/// The empty `kind` is the port's "tracker's configured default": a live tenant
/// has no issue type named after a fixed literal, and a hardcoded one is
/// case-sensitively rejected by real Jira. Empty resolves to each provider's
/// own default (Jira's `Task`; Linear ignores kind).
pub fn create_then_show_round_trips_property(subject: &dyn ContractSubject) {
    let tracker = subject.tracker();
    let created = tracker
        .create("Contract title", "Contract body\n", "")
        .expect("create must succeed for a conformant implementation");
    let issue = tracker
        .show(&created)
        .expect("show must succeed immediately after create");
    assert!(
        !issue.body.is_empty(),
        "a freshly created issue must project a non-empty body"
    );
}

/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn update_replaces_whole_content(
    subject: &dyn ContractSubject,
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    update_replaces_whole_content_property(subject);
    Ok(())
}

/// The assertions, ungated.
///
/// An offline caller with a mock-backed subject enforces them in the default
/// profile. The gate stays on the wrapper: reaching a property directly must
/// not thereby reach a live provider.
pub fn update_replaces_whole_content_property(subject: &dyn ContractSubject) {
    let tracker = subject.tracker();
    let id = tracker
        .create("Original title", "Original body\n", "")
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

/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn fetch_all_partitions_totally(
    subject: &dyn ContractSubject,
    ids: &[ExternalId],
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    fetch_all_partitions_totally_property(subject, ids);
    Ok(())
}

/// The assertions, ungated.
///
/// See [`create_then_show_round_trips_property`].
pub fn fetch_all_partitions_totally_property(
    subject: &dyn ContractSubject,
    ids: &[ExternalId],
) {
    let outcome = subject
        .tracker()
        .fetch_all(ids)
        .expect("fetch_all must succeed for a conformant implementation");
    partitions_totally(&outcome, ids);
}

/// Inferring absence from an incomplete retrieval is what makes a sync
/// delete an issue that still exists.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn unaccounted_id_is_indeterminate_not_absent(
    subject: &dyn ContractSubject,
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    unaccounted_id_is_indeterminate_not_absent_property(subject);
    Ok(())
}

/// The assertions, ungated.
///
/// An offline caller with a mock-backed subject enforces them in the default
/// profile. The gate stays on the wrapper: reaching a property directly must
/// not thereby reach a live provider.
pub fn unaccounted_id_is_indeterminate_not_absent_property(
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

/// A read mutates nothing, so the terminal class — "a mutation may have
/// applied" — cannot arise.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn a_failing_read_is_retryable(
    subject: &dyn ContractSubject,
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    a_failing_read_is_retryable_property(subject);
    Ok(())
}

/// The assertions, ungated.
///
/// An offline caller with a mock-backed subject enforces them in the default
/// profile. The gate stays on the wrapper: reaching a property directly must
/// not thereby reach a live provider.
pub fn a_failing_read_is_retryable_property(subject: &dyn ContractSubject) {
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

/// A truncated discovery must report `complete == false`, so a caller never
/// mistakes a cut-off query for the whole of what the tracker holds.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn search_reports_truncation(
    subject: &dyn ContractSubject,
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    search_reports_truncation_property(subject);
    Ok(())
}

/// The assertions, ungated.
///
/// An offline caller with a mock-backed subject enforces them in the default
/// profile. The gate stays on the wrapper: reaching a property directly must
/// not thereby reach a live provider.
pub fn search_reports_truncation_property(subject: &dyn ContractSubject) {
    let scope = subject.truncating_scope();
    let discovery = subject
        .tracker()
        .search(&scope)
        .expect("search must succeed for a conformant implementation");
    assert!(
        !discovery.complete,
        "a truncated discovery must report complete == false"
    );
}

/// A create preview contacts the tracker but mutates nothing.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn preview_create_makes_no_mutation(
    subject: &dyn ContractSubject,
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    preview_create_makes_no_mutation_property(subject);
    Ok(())
}

/// The assertions, ungated.
///
/// A preview must resolve without error. The no-mutation invariant proper — no
/// `create` observed — is additionally pinned offline, where a mock records
/// whether a create request was sent; the shared property cannot inspect a live
/// tracker's state.
pub fn preview_create_makes_no_mutation_property(
    subject: &dyn ContractSubject,
) {
    subject
        .tracker()
        .preview_create(&subject.preview_kind())
        .expect("preview_create must succeed for a conformant implementation");
}

/// A create preview resolves each field to the expected three-state outcome.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn preview_create_resolves_fields(
    subject: &dyn ContractSubject,
    expected: &CreatePreview,
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    preview_create_resolves_fields_property(subject, expected);
    Ok(())
}

/// The assertions, ungated.
///
/// Enforced offline, where three separately-configured mocks stand in for a
/// resolved, an unset and an unresolvable field.
pub fn preview_create_resolves_fields_property(
    subject: &dyn ContractSubject,
    expected: &CreatePreview,
) {
    let preview = subject
        .tracker()
        .preview_create(&subject.preview_kind())
        .expect("preview_create must succeed for a conformant implementation");
    assert_eq!(
        &preview, expected,
        "the create preview must resolve each field to its expected state"
    );
}

/// A payload missing a locally-required field is `Rejected` naming it; a
/// complete payload is `Valid`. The check is local, so no remote state is
/// asserted — the type makes a mutation unrepresentable.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn validate_update_reports_outcome(
    subject: &dyn ContractSubject,
) -> Result<(), ContractGateError> {
    ensure_opted_in()?;
    validate_update_reports_outcome_property(subject);
    Ok(())
}

/// The assertions, ungated.
///
/// An empty title is the locally-required omission both providers reject: the
/// composed payload's summary/title field would be blank.
pub fn validate_update_reports_outcome_property(subject: &dyn ContractSubject) {
    let id = ExternalId::new("PREVIEW-1".to_owned());
    let valid = subject.tracker().validate_update(
        &id,
        "A present title",
        "A present body\n",
    );
    assert_eq!(
        valid,
        ValidationOutcome::Valid,
        "a complete payload must validate"
    );

    let rejected =
        subject
            .tracker()
            .validate_update(&id, "", "A present body\n");
    assert!(
        matches!(rejected, ValidationOutcome::Rejected { .. }),
        "a payload missing a required field must be rejected, got {rejected:?}"
    );
    if let ValidationOutcome::Rejected { reasons } = rejected {
        assert!(
            !reasons.is_empty(),
            "a rejection must name the missing field"
        );
    }
}

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

fn opted_in(raw: Option<&str>) -> bool {
    raw == Some("1")
}

/// The opt-in gate, checked by every entry point that touches the tracker
/// rather than by [`run_all`] alone: a caller reaching a property function
/// directly must not thereby reach a live provider.
fn ensure_opted_in() -> Result<(), ContractGateError> {
    if opted_in(
        std::env::var("ACCELERATOR_TRACKER_CONTRACT")
            .ok()
            .as_deref(),
    ) {
        Ok(())
    } else {
        Err(ContractGateError::NotOptedIn)
    }
}

/// The conformance set: properties every implementation must satisfy
/// unprompted, excluding the two that need a deliberately configured
/// subject.
///
/// Errors rather than skips when the gate is closed, so a dropped or
/// misspelled env var cannot make every contract binary exit 0 having
/// asserted nothing. The returned count of properties executed must be
/// asserted non-zero by the caller: on its own a count assertion cannot
/// distinguish a closed gate from an empty run, since both give zero.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn run_all(
    subject: &dyn ContractSubject,
    ids: &[ExternalId],
) -> Result<usize, ContractGateError> {
    create_then_show_round_trips(subject)?;
    update_replaces_whole_content(subject)?;
    fetch_all_partitions_totally(subject, ids)?;
    preview_create_makes_no_mutation(subject)?;
    validate_update_reports_outcome(subject)?;

    Ok(5)
}

/// Run the full conformance set against a live subject, timing each property,
/// and return the reduced records a contract run commits as evidence.
///
/// A property that fails panics through this rather than returning `FAIL`, so
/// evidence is only ever produced for a passing run — you do not commit a
/// record of a run that did not conform. The gate is checked once up front.
///
/// # Errors
///
/// [`ContractGateError::NotOptedIn`] when the gate is closed.
pub fn timed_conformance(
    subject: &dyn ContractSubject,
    ids: &[ExternalId],
) -> Result<Vec<crate::evidence::EvidenceRecord>, ContractGateError> {
    ensure_opted_in()?;

    let mut records = Vec::new();
    let run = |name: &str,
               count: usize,
               property: &dyn Fn()|
     -> crate::evidence::EvidenceRecord {
        let started = std::time::Instant::now();
        property();
        crate::evidence::EvidenceRecord {
            name: name.to_owned(),
            passed: true,
            count,
            duration: started.elapsed(),
        }
    };

    records.push(run("create_then_show_round_trips", 1, &|| {
        create_then_show_round_trips_property(subject);
    }));
    records.push(run("update_replaces_whole_content", 1, &|| {
        update_replaces_whole_content_property(subject);
    }));
    records.push(run(
        "fetch_all_partitions_totally",
        ids.len().max(1),
        &|| {
            fetch_all_partitions_totally_property(subject, ids);
        },
    ));
    if subject.can_nominate_indeterminate() {
        records.push(run(
            "unaccounted_id_is_indeterminate_not_absent",
            1,
            &|| {
                unaccounted_id_is_indeterminate_not_absent_property(subject);
            },
        ));
    }
    records.push(run("a_failing_read_is_retryable", 1, &|| {
        a_failing_read_is_retryable_property(subject);
    }));
    records.push(run("preview_create_makes_no_mutation", 1, &|| {
        preview_create_makes_no_mutation_property(subject);
    }));
    records.push(run("validate_update_reports_outcome", 1, &|| {
        validate_update_reports_outcome_property(subject);
    }));
    if subject.can_induce_truncation() {
        records.push(run("search_reports_truncation", 1, &|| {
            search_reports_truncation_property(subject);
        }));
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::opted_in;
    use super::ContractGateError;
    use crate::RecordingTracker;

    type GatedCall = fn(&RecordingTracker) -> Result<(), ContractGateError>;

    /// Every entry point that touches the tracker, so a new one added
    /// without a gate fails this rather than reaching a live provider.
    fn gated_calls() -> Vec<(&'static str, GatedCall)> {
        vec![
            ("create_then_show_round_trips", |subject| {
                super::create_then_show_round_trips(subject)
            }),
            ("update_replaces_whole_content", |subject| {
                super::update_replaces_whole_content(subject)
            }),
            ("fetch_all_partitions_totally", |subject| {
                super::fetch_all_partitions_totally(subject, &[])
            }),
            ("unaccounted_id_is_indeterminate_not_absent", |subject| {
                super::unaccounted_id_is_indeterminate_not_absent(subject)
            }),
            ("a_failing_read_is_retryable", |subject| {
                super::a_failing_read_is_retryable(subject)
            }),
            ("search_reports_truncation", |subject| {
                super::search_reports_truncation(subject)
            }),
            ("preview_create_makes_no_mutation", |subject| {
                super::preview_create_makes_no_mutation(subject)
            }),
            ("preview_create_resolves_fields", |subject| {
                super::preview_create_resolves_fields(
                    subject,
                    &tracker::CreatePreview {
                        project: tracker::FieldResolution::Unset,
                        issue_type: tracker::FieldResolution::Unset,
                    },
                )
            }),
            ("validate_update_reports_outcome", |subject| {
                super::validate_update_reports_outcome(subject)
            }),
            ("timed_conformance", |subject| {
                super::timed_conformance(subject, &[]).map(|_| ())
            }),
        ]
    }

    #[test]
    fn every_tracker_touching_entry_point_refuses_when_the_gate_is_closed() {
        // nextest runs each test in its own process, so removing the
        // variable here cannot race another test.
        std::env::remove_var("ACCELERATOR_TRACKER_CONTRACT");
        let subject = RecordingTracker::holding(Vec::new());

        for (name, call) in gated_calls() {
            assert_eq!(
                call(&subject),
                Err(ContractGateError::NotOptedIn),
                "{name} ran with the gate closed — it would reach a live \
                 provider once a real client implements ContractSubject"
            );
        }
    }

    #[test]
    fn only_the_exact_opt_in_value_opens_the_gate() {
        assert!(opted_in(Some("1")));
        for closed in [None, Some(""), Some("0"), Some("true"), Some(" 1")] {
            assert!(!opted_in(closed), "{closed:?} must not open the gate");
        }
    }
}

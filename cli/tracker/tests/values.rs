//! The value objects the port carries — [`Ceiling`] and [`Completeness`] —
//! exercised as an external consumer sees them. The crate ships no inline test
//! module (see `structure.rs`), so its unit coverage lives here.

use tracker::Ceiling;
use tracker::Completeness;

#[test]
fn merge_is_cap_hit_dominant_then_transient_then_complete() {
    assert_eq!(
        Completeness::CapHit.merge(Completeness::Transient),
        Completeness::CapHit
    );
    assert_eq!(
        Completeness::Transient.merge(Completeness::CapHit),
        Completeness::CapHit
    );
    assert_eq!(
        Completeness::Transient.merge(Completeness::Complete),
        Completeness::Transient
    );
    assert_eq!(
        Completeness::Complete.merge(Completeness::Transient),
        Completeness::Transient
    );
    assert_eq!(
        Completeness::Complete.merge(Completeness::Complete),
        Completeness::Complete
    );
}

#[test]
fn only_complete_reads_as_complete() {
    assert!(Completeness::Complete.is_complete());
    assert!(!Completeness::CapHit.is_complete());
    assert!(!Completeness::Transient.is_complete());
}

#[test]
fn a_bounded_ceiling_is_exceeded_only_beyond_its_cap() {
    assert!(!Ceiling::Bounded(3).exceeds(3));
    assert!(Ceiling::Bounded(3).exceeds(4));
    assert!(Ceiling::Bounded(0).exceeds(1));
    assert!(!Ceiling::Bounded(0).exceeds(0));
}

#[test]
fn an_unlimited_ceiling_is_never_exceeded_or_reached() {
    assert!(!Ceiling::Unlimited.exceeds(usize::MAX));
    assert!(!Ceiling::Unlimited.reached(usize::MAX));
}

#[test]
fn a_bounded_ceiling_is_reached_at_its_cap() {
    assert!(!Ceiling::Bounded(2).reached(1));
    assert!(Ceiling::Bounded(2).reached(2));
    assert!(Ceiling::Bounded(2).reached(3));
}

#[test]
fn a_ceiling_renders_its_bound_or_the_unlimited_sentinel() {
    assert_eq!(Ceiling::Bounded(50).to_string(), "50");
    assert_eq!(Ceiling::Unlimited.to_string(), "unlimited");
}

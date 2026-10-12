#![allow(clippy::expect_used)]

use std::num::NonZeroU32;

use research::conduct::concurrency::ConcurrencyWarning;
use research::conduct::concurrency::EmptyConcurrency;
use research::conduct::concurrency::Provenance;
use research::conduct::concurrency::RequestedConcurrency;
use research::conduct::concurrency::ResolvedConcurrency;
use research::conduct::concurrency::SubagentCap;

const fn cap(slots: u32) -> SubagentCap {
    SubagentCap::new(NonZeroU32::new(slots).expect("a non-zero cap"))
}

fn requested(value: &str, provenance: Provenance) -> RequestedConcurrency {
    RequestedConcurrency::Given {
        value: value.to_owned(),
        provenance,
    }
}

fn resolve(
    value: &str,
    provenance: Provenance,
    cap: SubagentCap,
) -> ResolvedConcurrency {
    requested(value, provenance)
        .resolve(cap)
        .expect("a non-empty request resolves")
}

const fn agents(resolved: &ResolvedConcurrency) -> u32 {
    resolved.concurrency.agents().get()
}

#[test]
fn a_valid_request_at_or_below_the_cap_passes_unwarned() {
    for (value, provenance, setting, expected) in [
        ("20", Provenance::ConfigFile, 20, 20),
        ("24", Provenance::ConfigFile, 30, 24),
        ("1", Provenance::Flag, 20, 1),
        ("020", Provenance::Flag, 20, 20),
    ] {
        let resolved = resolve(value, provenance, cap(setting));
        assert_eq!(
            agents(&resolved),
            expected,
            "{value:?} from {provenance:?} at cap {setting}"
        );
        assert_eq!(
            resolved.warnings,
            [],
            "{value:?} from {provenance:?} at cap {setting}"
        );
    }
}

#[test]
fn a_request_above_the_cap_clamps_with_a_warning_when_set_explicitly() {
    for (value, provenance) in
        [("24", Provenance::ConfigFile), ("50", Provenance::Flag)]
    {
        let resolved = resolve(value, provenance, cap(20));
        assert_eq!(agents(&resolved), 20, "{value:?} from {provenance:?}");
        assert_eq!(
            resolved.warnings,
            [ConcurrencyWarning::AboveCap {
                given: value.to_owned(),
                provenance,
                cap: cap(20),
            }],
            "{value:?} from {provenance:?}"
        );
    }
}

#[test]
fn the_built_in_default_is_the_harness_default_bounded_silently_by_the_cap() {
    for (setting, expected) in [(8, 8), (50, 20)] {
        let resolved = RequestedConcurrency::BuiltInDefault
            .resolve(cap(setting))
            .expect("the built-in default resolves");
        assert_eq!(agents(&resolved), expected, "cap {setting:?}");
        assert_eq!(resolved.warnings, [], "cap {setting:?}");
    }
}

#[test]
fn a_malformed_request_clamps_to_1_before_the_cap_applies() {
    let flag = ["0", "-3", "50.5", "many", "+5", " 5", "٥"]
        .map(|value| (value, Provenance::Flag));
    let config = ["0", "many"].map(|value| (value, Provenance::ConfigFile));
    for (value, provenance) in flag.into_iter().chain(config) {
        let resolved = resolve(value, provenance, cap(20));
        assert_eq!(agents(&resolved), 1, "{value:?} from {provenance:?}");
        assert_eq!(
            resolved.warnings,
            [ConcurrencyWarning::NotAPositiveInteger {
                given: value.to_owned(),
                provenance,
            }],
            "{value:?} from {provenance:?}"
        );
    }
}

#[test]
fn an_empty_request_is_refused() {
    for provenance in [Provenance::Flag, Provenance::ConfigFile] {
        assert_eq!(
            requested("", provenance).resolve(cap(20)),
            Err(EmptyConcurrency { provenance }),
            "empty from {provenance:?}"
        );
    }
}

#[test]
fn a_request_too_large_for_u32_still_clamps_naming_what_was_given() {
    let resolved = resolve("99999999999", Provenance::Flag, cap(20));
    assert_eq!(agents(&resolved), 20);
    assert_eq!(
        resolved.warnings,
        [ConcurrencyWarning::AboveCap {
            given: "99999999999".to_owned(),
            provenance: Provenance::Flag,
            cap: cap(20),
        }]
    );
}

#[test]
fn a_concurrency_admits_at_most_its_agents() {
    let twenty = resolve("20", Provenance::Flag, cap(20)).concurrency;
    assert_eq!(twenty.agents(), NonZeroU32::new(20).expect("non-zero"));
    assert_eq!(twenty.admits(25), 20);
    assert_eq!(twenty.admits(5), 5);

    let widest = resolve(
        "99999999999",
        Provenance::Flag,
        SubagentCap::new(NonZeroU32::MAX),
    )
    .concurrency;
    assert_eq!(widest.admits(25), 25);
}

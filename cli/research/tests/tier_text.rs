#![allow(clippy::expect_used)]

use research::sources::record::Tier;
use research::topic::sources_section::tier_text::TierText;

fn read(text: &str) -> (Tier, bool, bool, Vec<Tier>) {
    let tier_text = TierText::parse(text).expect(text);
    (
        tier_text.base(),
        tier_text.retracted(),
        tier_text.withdrawn(),
        tier_text.also().to_vec(),
    )
}

#[test]
fn a_bare_base_tier_has_no_marks_and_no_also_tiers() {
    for (text, base) in [
        ("tier-1", Tier::One),
        ("tier-2", Tier::Two),
        ("tier-3", Tier::Three),
    ] {
        assert_eq!(read(text), (base, false, false, vec![]), "{text}");
    }
}

#[test]
fn a_single_mark_is_read_onto_its_base() {
    for (text, retracted, withdrawn) in [
        ("tier-3 (retracted)", true, false),
        ("tier-3 (withdrawn)", false, true),
    ] {
        assert_eq!(
            read(text),
            (Tier::Three, retracted, withdrawn, vec![]),
            "{text}"
        );
    }
}

#[test]
fn also_tiers_are_kept_in_written_order() {
    for (text, also) in [
        ("tier-3 (also tier-1)", vec![Tier::One]),
        ("tier-3 (also tier-2, tier-1)", vec![Tier::Two, Tier::One]),
    ] {
        assert_eq!(read(text), (Tier::Three, false, false, also), "{text}");
    }
}

#[test]
fn marks_and_also_tiers_combine_in_canonical_order() {
    for (text, expected) in [
        (
            "tier-3 (retracted; withdrawn; also tier-1)",
            (Tier::Three, true, true, vec![Tier::One]),
        ),
        (
            "tier-3 (retracted; withdrawn)",
            (Tier::Three, true, true, vec![]),
        ),
        (
            "tier-3 (withdrawn; also tier-1)",
            (Tier::Three, false, true, vec![Tier::One]),
        ),
    ] {
        assert_eq!(read(text), expected, "{text}");
    }
}

#[test]
fn every_tier_text_the_citing_once_rule_gives_parses() {
    for (text, expected) in [
        ("tier-1", (Tier::One, false, false, vec![])),
        ("tier-2", (Tier::Two, false, false, vec![])),
        ("tier-3", (Tier::Three, false, false, vec![])),
        ("tier-3 (retracted)", (Tier::Three, true, false, vec![])),
        (
            "tier-3 (also tier-1)",
            (Tier::Three, false, false, vec![Tier::One]),
        ),
        (
            "tier-3 (also tier-2)",
            (Tier::Three, false, false, vec![Tier::Two]),
        ),
        (
            "tier-3 (also tier-2, tier-1)",
            (Tier::Three, false, false, vec![Tier::Two, Tier::One]),
        ),
        (
            "tier-3 (retracted; also tier-1)",
            (Tier::Three, true, false, vec![Tier::One]),
        ),
        (
            "tier-3 (retracted; also tier-2, tier-1)",
            (Tier::Three, true, false, vec![Tier::Two, Tier::One]),
        ),
        (
            "tier-3 (retracted; withdrawn; also tier-1)",
            (Tier::Three, true, true, vec![Tier::One]),
        ),
    ] {
        assert_eq!(read(text), expected, "{text}");
    }
}

#[test]
fn rule_breaking_text_of_the_right_shape_is_accepted_as_written() {
    for (text, expected) in [
        ("tier-1 (retracted)", (Tier::One, true, false, vec![])),
        (
            "tier-3 (also tier-3)",
            (Tier::Three, false, false, vec![Tier::Three]),
        ),
        (
            "tier-3 (also tier-1, tier-2)",
            (Tier::Three, false, false, vec![Tier::One, Tier::Two]),
        ),
    ] {
        assert_eq!(read(text), expected, "{text}");
    }
}

#[test]
fn text_that_is_not_tier_text_is_rejected_with_the_whole_input() {
    for text in [
        "tier-4",
        "tier-3 (unknown)",
        "tier-3 (also tier-9)",
        "",
        "tier-3 ()",
        "tier-3 (withdrawn; retracted)",
        "tier-3 (also tier-1; retracted)",
        "tier-3 (retracted; retracted)",
        "tier-3(retracted)",
        "tier-3 (retracted;withdrawn)",
        "tier-3 (also tier-2,tier-1)",
        "tier-3  (retracted)",
        "tier-3 (also)",
        "tier-3 (retracted) x",
        "Tier-1",
        " tier-1 ",
    ] {
        let rejection = TierText::parse(text).expect_err(text);
        assert_eq!(rejection.text(), text);
        assert_eq!(rejection.to_string(), format!("not tier text: `{text}`"));
    }
}

#[test]
fn every_accepted_tier_text_renders_back_identically() {
    for text in [
        "tier-1",
        "tier-2",
        "tier-3",
        "tier-3 (retracted)",
        "tier-3 (withdrawn)",
        "tier-3 (also tier-1)",
        "tier-3 (also tier-2)",
        "tier-3 (also tier-2, tier-1)",
        "tier-3 (retracted; also tier-1)",
        "tier-3 (retracted; also tier-2, tier-1)",
        "tier-3 (retracted; withdrawn)",
        "tier-3 (withdrawn; also tier-1)",
        "tier-3 (retracted; withdrawn; also tier-1)",
        "tier-1 (retracted)",
        "tier-3 (also tier-3)",
        "tier-3 (also tier-1, tier-2)",
    ] {
        assert_eq!(TierText::parse(text).expect(text).to_string(), text);
    }
}

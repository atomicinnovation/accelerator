#![allow(clippy::expect_used)]

use research::arxiv::is_withdrawal_candidate;
use research::arxiv::latest_version_withdrawn;
use research::arxiv::normalise;
use research::arxiv::Entry;
use research::arxiv::RawVersion;
use research::record::Tier;
use research::record::VenueSignals;

fn entry() -> Entry {
    Entry {
        id: "http://arxiv.org/abs/2608.21129v2".to_owned(),
        title: "Attention heads specialise".to_owned(),
        summary: Some("We study attention.".to_owned()),
        authors: vec!["Ada Lovelace".to_owned()],
        comment: None,
        journal_ref: Some("J. Attn. 1 (2026)".to_owned()),
        doi: Some("10.1234/attn".to_owned()),
        primary_category: Some("cs.LG".to_owned()),
    }
}

fn version(size: &str, source_type: Option<&str>) -> RawVersion {
    RawVersion {
        size: size.to_owned(),
        source_type: source_type.map(str::to_owned),
    }
}

fn excerpt_of(summary: &str) -> String {
    let record = normalise(
        &Entry {
            summary: Some(summary.to_owned()),
            ..entry()
        },
        false,
    )
    .expect("valid entry");
    record.abstract_excerpt().unwrap_or_default().to_owned()
}

#[test]
fn a_withdrawal_comment_is_a_candidate_whatever_its_case() {
    for comment in [
        "This paper has been withdrawn by the authors.",
        "  this ARTICLE is withdrawn",
        "This submission has been withdrawn",
        "This manuscript is withdrawn due to an error",
        "Withdrawn",
        "withdrawn: duplicate",
        "This paper has been\n  withdrawn by the author",
    ] {
        assert!(is_withdrawal_candidate(comment), "{comment}");
    }
}

#[test]
fn a_comment_merely_mentioning_withdrawal_is_not_a_candidate() {
    for comment in [
        "Supersedes arXiv:2510.26642, which has been withdrawn",
        "This thesis has been withdrawn",
        "This paper was withdrawn",
        "12 pages",
        "",
    ] {
        assert!(!is_withdrawal_candidate(comment), "{comment}");
    }
}

#[test]
fn only_an_empty_latest_version_of_source_type_i_is_withdrawn() {
    let withdrawn = [version("412kb", Some("D")), version("0kb", Some("I"))];
    assert!(latest_version_withdrawn(&withdrawn));

    let superseded = [version("0kb", Some("I")), version("415kb", Some("D"))];
    assert!(!latest_version_withdrawn(&superseded));

    assert!(!latest_version_withdrawn(&[version("0kb", Some("D"))]));
    assert!(!latest_version_withdrawn(&[version("12kb", Some("I"))]));
    assert!(!latest_version_withdrawn(&[version("0kb", None)]));
    assert!(!latest_version_withdrawn(&[]));
}

#[test]
fn an_entry_normalises_every_record_field() {
    let record = normalise(&entry(), false).expect("valid entry");
    assert_eq!(record.title(), Some("Attention heads specialise"));
    assert_eq!(record.authors(), ["Ada Lovelace"]);
    assert_eq!(record.url(), "https://arxiv.org/abs/2608.21129");
    assert_eq!(record.venue(), Some("arXiv (cs.LG)"));
    assert_eq!(record.abstract_excerpt(), Some("We study attention."));
    assert_eq!(record.tier(), Tier::Two);
    assert!(!record.retracted());
    assert!(!record.withdrawn());
    assert_eq!(
        record.venue_signals(),
        &VenueSignals::Arxiv {
            journal_ref: Some("J. Attn. 1 (2026)".to_owned()),
            doi: Some("10.1234/attn".to_owned()),
        }
    );
}

#[test]
fn an_entry_without_a_category_is_venued_on_arxiv_alone() {
    let uncategorised = Entry {
        primary_category: None,
        ..entry()
    };
    let record = normalise(&uncategorised, false).expect("valid entry");
    assert_eq!(record.venue(), Some("arXiv"));
}

#[test]
fn a_withdrawn_entry_is_tier_three_and_marked_withdrawn() {
    let record = normalise(&entry(), true).expect("valid entry");
    assert_eq!(record.tier(), Tier::Three);
    assert!(record.withdrawn());
    assert!(!record.retracted());
}

#[test]
fn an_entry_whose_id_is_not_an_arxiv_abstract_url_is_dropped() {
    let foreign = Entry {
        id: "https://evil.example/abs/2608.21129".to_owned(),
        ..entry()
    };
    assert!(normalise(&foreign, false).is_none());
}

#[test]
fn an_abstract_of_exactly_six_hundred_characters_is_kept_whole() {
    let summary = "a".repeat(600);
    assert_eq!(excerpt_of(&summary), summary);
}

#[test]
fn a_longer_abstract_is_cut_at_a_word_boundary_within_six_hundred() {
    let summary = format!("{} {}", "a".repeat(590), "b".repeat(10));
    let excerpt = excerpt_of(&summary);
    assert_eq!(excerpt, format!("{}…", "a".repeat(590)));
    assert!(excerpt.chars().count() <= 600);
}

#[test]
fn a_boundary_falling_exactly_at_the_cut_keeps_the_whole_word() {
    let summary = format!("{} {}", "a".repeat(599), "b".repeat(5));
    assert_eq!(excerpt_of(&summary), format!("{}…", "a".repeat(599)));
}

#[test]
fn an_unbroken_abstract_is_cut_mid_word() {
    let excerpt = excerpt_of(&"a".repeat(601));
    assert_eq!(excerpt, format!("{}…", "a".repeat(599)));
}

#[test]
fn an_abstract_is_measured_in_characters_not_bytes() {
    let summary = "é".repeat(600);
    assert_eq!(excerpt_of(&summary), summary);

    let longer = format!("{} {}", "é".repeat(400), "ü".repeat(300));
    assert_eq!(excerpt_of(&longer), format!("{}…", "é".repeat(400)));
}

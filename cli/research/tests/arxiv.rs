#![allow(clippy::expect_used)]

use research::sources::arxiv::is_withdrawal_candidate;
use research::sources::arxiv::latest_version_withdrawn;
use research::sources::arxiv::normalise;
use research::sources::arxiv::Entry;
use research::sources::arxiv::RawVersion;
use research::sources::metadata::date::PartialDate;
use research::sources::record::Tier;
use research::sources::record::VenueSignals;

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
        published: Some("2026-08-21T14:06:59Z".to_owned()),
        updated: Some("2026-08-24T04:57:13Z".to_owned()),
    }
}

fn accessed() -> PartialDate {
    PartialDate::parse("2027-01-15").expect("valid date")
}

fn metadata_of(entry: &Entry) -> Vec<(String, String)> {
    normalise(entry, false, accessed())
        .expect("valid entry")
        .metadata()
        .entries()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn dated_entry() -> Entry {
    Entry {
        id: "http://arxiv.org/abs/2305.12345v2".to_owned(),
        doi: Some("10.48550/ARXIV.2305.12345".to_owned()),
        published: Some("2023-05-20T17:59:59Z".to_owned()),
        updated: Some("2023-06-01T09:00:00Z".to_owned()),
        ..entry()
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
        accessed(),
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
    let record = normalise(&entry(), false, accessed()).expect("valid entry");
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
    let record =
        normalise(&uncategorised, false, accessed()).expect("valid entry");
    assert_eq!(record.venue(), Some("arXiv"));
}

#[test]
fn a_withdrawn_entry_is_tier_three_and_marked_withdrawn() {
    let record = normalise(&entry(), true, accessed()).expect("valid entry");
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
    assert!(normalise(&foreign, false, accessed()).is_none());
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

#[test]
fn an_entry_records_its_doi_versioned_id_and_dates_in_order() {
    assert_eq!(
        metadata_of(&dated_entry()),
        pairs(&[
            ("doi", "10.48550/arxiv.2305.12345"),
            ("arxiv_id", "2305.12345v2"),
            ("published", "2023-05-20"),
            ("updated", "2023-06-01"),
            ("accessed", "2027-01-15"),
        ])
    );
}

#[test]
fn an_entry_without_a_doi_records_no_doi() {
    let entry = Entry {
        doi: None,
        ..dated_entry()
    };
    assert!(metadata_of(&entry).iter().all(|(key, _)| key != "doi"));
}

#[test]
fn an_unusable_published_date_is_left_out_while_the_rest_remain() {
    let entry = Entry {
        published: Some("20/05/2023".to_owned()),
        ..dated_entry()
    };
    assert_eq!(
        metadata_of(&entry),
        pairs(&[
            ("doi", "10.48550/arxiv.2305.12345"),
            ("arxiv_id", "2305.12345v2"),
            ("updated", "2023-06-01"),
            ("accessed", "2027-01-15"),
        ])
    );
}

#[test]
fn no_record_repeats_its_url_title_or_venue_in_its_metadata() {
    let keys = metadata_of(&dated_entry())
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    for entry_line_field in ["url", "title", "container_title", "journal_title"]
    {
        assert!(!keys.iter().any(|key| key == entry_line_field));
    }
}

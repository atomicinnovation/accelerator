#![allow(clippy::expect_used)]

use research::sources::metadata::date::PartialDate;
use research::sources::openalex::abstract_from_inverted_index;
use research::sources::openalex::normalise;
use research::sources::openalex::Location;
use research::sources::openalex::Source;
use research::sources::openalex::Work;
use research::sources::record::Tier;
use research::sources::record::VenueSignals;

fn index(pairs: &[(&str, &[usize])]) -> Vec<(String, Vec<usize>)> {
    pairs
        .iter()
        .map(|(word, positions)| ((*word).to_owned(), positions.to_vec()))
        .collect()
}

fn accessed() -> PartialDate {
    PartialDate::parse("2027-01-15").expect("valid date")
}

fn metadata_of(work: &Work) -> Vec<(String, String)> {
    normalise(work, accessed())
        .expect("valid work")
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

fn published_article() -> Work {
    Work {
        id: "https://openalex.org/W2741809807".to_owned(),
        doi: Some("https://doi.org/10.7717/peerj.4375".to_owned()),
        publication_date: Some("2018-02-13".to_owned()),
        title: Some("The state of OA".to_owned()),
        work_type: "article".to_owned(),
        authors: vec!["Heather Piwowar".to_owned(), "Jason Priem".to_owned()],
        primary_location: Some(Location {
            version: Some("publishedVersion".to_owned()),
            source: Some(Source {
                display_name: Some("PeerJ".to_owned()),
                source_type: Some("journal".to_owned()),
                is_core: true,
                listed_in: vec!["cwts-core".to_owned(), "medline".to_owned()],
            }),
        }),
        is_retracted: false,
        abstract_inverted_index: Some(index(&[
            ("Despite", &[0]),
            ("growing", &[1]),
            ("interest", &[2]),
        ])),
    }
}

#[test]
fn an_abstract_is_rebuilt_in_position_order() {
    let rebuilt = abstract_from_inverted_index(&index(&[
        ("the", &[0, 3]),
        ("cat", &[1]),
        ("sat", &[2]),
        ("mat", &[4]),
    ]));
    assert_eq!(rebuilt, "the cat sat the mat");
}

#[test]
fn a_work_normalises_every_record_field() {
    let record =
        normalise(&published_article(), accessed()).expect("valid work");
    assert_eq!(record.title(), Some("The state of OA"));
    assert_eq!(record.authors(), ["Heather Piwowar", "Jason Priem"]);
    assert_eq!(record.url(), "https://doi.org/10.7717/peerj.4375");
    assert_eq!(record.venue(), Some("PeerJ"));
    assert_eq!(record.abstract_excerpt(), Some("Despite growing interest"));
    assert_eq!(record.tier(), Tier::One);
    assert!(!record.retracted());
    assert!(!record.withdrawn());
    assert_eq!(
        record.venue_signals(),
        &VenueSignals::OpenAlex {
            source_type: Some("journal".to_owned()),
            version: Some("publishedVersion".to_owned()),
            is_core: Some(true),
            listed_in: vec!["cwts-core".to_owned(), "medline".to_owned()],
            work_type: "article".to_owned(),
            is_retracted: false,
        }
    );
}

#[test]
fn a_work_without_a_doi_is_cited_by_its_openalex_url() {
    let work = Work {
        doi: None,
        ..published_article()
    };
    let record = normalise(&work, accessed()).expect("valid work");
    assert_eq!(record.url(), "https://openalex.org/W2741809807");
}

#[test]
fn a_work_whose_doi_is_invalid_is_cited_by_its_openalex_url() {
    let work = Work {
        doi: Some("https://doi.org/10.1234/../../works".to_owned()),
        ..published_article()
    };
    let record = normalise(&work, accessed()).expect("valid work");
    assert_eq!(record.url(), "https://openalex.org/W2741809807");
}

#[test]
fn a_doi_with_unsafe_characters_is_cited_percent_encoded() {
    let work = Work {
        doi: Some("https://doi.org/10.1002/a)b<c".to_owned()),
        ..published_article()
    };
    let record = normalise(&work, accessed()).expect("valid work");
    assert_eq!(record.url(), "https://doi.org/10.1002/a%29b%3Cc");
}

#[test]
fn a_doi_resolved_through_dx_is_cited_at_doi_org() {
    let work = Work {
        doi: Some("http://dx.doi.org/10.7717/peerj.4375".to_owned()),
        ..published_article()
    };
    let record = normalise(&work, accessed()).expect("valid work");
    assert_eq!(record.url(), "https://doi.org/10.7717/peerj.4375");
}

#[test]
fn a_work_whose_id_is_not_an_openalex_work_is_dropped() {
    let work = Work {
        id: "https://evil.example/W1".to_owned(),
        ..published_article()
    };
    assert!(normalise(&work, accessed()).is_none());
}

#[test]
fn a_retracted_work_is_marked_retracted() {
    let work = Work {
        is_retracted: true,
        ..published_article()
    };
    let record = normalise(&work, accessed()).expect("valid work");
    assert!(record.retracted());
    assert_eq!(record.tier(), Tier::Three);
}

#[test]
fn a_sourceless_work_reports_empty_venue_signals() {
    let work = Work {
        primary_location: None,
        abstract_inverted_index: None,
        ..published_article()
    };
    let record = normalise(&work, accessed()).expect("valid work");
    assert_eq!(record.venue(), None);
    assert_eq!(record.abstract_excerpt(), None);
    assert_eq!(
        record.venue_signals(),
        &VenueSignals::OpenAlex {
            source_type: None,
            version: None,
            is_core: None,
            listed_in: Vec::new(),
            work_type: "article".to_owned(),
            is_retracted: false,
        }
    );
}

#[test]
fn a_work_records_its_doi_id_publication_and_access_dates_in_order() {
    let work = Work {
        doi: Some("https://doi.org/10.7717/PeerJ.4375".to_owned()),
        ..published_article()
    };
    assert_eq!(
        metadata_of(&work),
        pairs(&[
            ("doi", "10.7717/peerj.4375"),
            ("openalex_id", "W2741809807"),
            ("published", "2018-02-13"),
            ("accessed", "2027-01-15"),
        ])
    );
}

#[test]
fn a_work_without_a_doi_records_no_doi() {
    let work = Work {
        doi: None,
        ..published_article()
    };
    assert_eq!(
        metadata_of(&work),
        pairs(&[
            ("openalex_id", "W2741809807"),
            ("published", "2018-02-13"),
            ("accessed", "2027-01-15"),
        ])
    );
}

#[test]
fn an_unusable_publication_date_is_left_out_while_the_rest_remain() {
    let work = Work {
        publication_date: Some("unknown".to_owned()),
        ..published_article()
    };
    assert_eq!(
        metadata_of(&work),
        pairs(&[
            ("doi", "10.7717/peerj.4375"),
            ("openalex_id", "W2741809807"),
            ("accessed", "2027-01-15"),
        ])
    );
}

#[test]
fn no_record_repeats_its_url_title_or_venue_in_its_metadata() {
    let keys = metadata_of(&published_article())
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    for entry_line_field in ["url", "title", "container_title", "journal_title"]
    {
        assert!(!keys.iter().any(|key| key == entry_line_field));
    }
}

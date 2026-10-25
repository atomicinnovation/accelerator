#![allow(clippy::expect_used)]

use research::sources::metadata::date::PartialDate;
use research::sources::metadata::SourceMetadata;
use research::sources::metadata::RECOMMENDED_KEYS;
use research::sources::request::ArxivId;
use research::sources::request::Doi;
use research::sources::request::WorkId;

fn date(raw: &str) -> PartialDate {
    PartialDate::parse(raw).expect("valid date")
}

fn entries(metadata: &SourceMetadata) -> Vec<(&str, &str)> {
    metadata.entries().collect()
}

#[test]
fn recommended_keys_come_out_in_table_order_whatever_the_recording_order() {
    let mut metadata = SourceMetadata::default();
    metadata.record_accessed(date("2027-01-15"));
    metadata.record_updated(date("2023-06-01"));
    metadata.record_published(date("2023-05-20"));
    metadata.record_openalex_id(
        &WorkId::from_openalex("W2741809807").expect("valid"),
    );
    metadata.record_arxiv_id(&ArxivId::parse("2305.12345v2").expect("valid"));
    metadata
        .record_doi(&Doi::parse("10.48550/ARXIV.2305.12345").expect("valid"));

    assert_eq!(
        entries(&metadata),
        [
            ("doi", "10.48550/arxiv.2305.12345"),
            ("arxiv_id", "2305.12345v2"),
            ("openalex_id", "W2741809807"),
            ("published", "2023-05-20"),
            ("updated", "2023-06-01"),
            ("accessed", "2027-01-15"),
        ]
    );
}

#[test]
fn a_key_recorded_twice_keeps_its_first_value() {
    let mut metadata = SourceMetadata::default();
    metadata.record_doi(&Doi::parse("10.1000/FIRST").expect("valid"));
    metadata.record_doi(&Doi::parse("10.1000/SECOND").expect("valid"));
    metadata.record_published(date("2010"));
    metadata.record_published(date("2011-01-01"));

    assert_eq!(
        entries(&metadata),
        [("doi", "10.1000/first"), ("published", "2010")]
    );
}

#[test]
fn nothing_recorded_is_empty() {
    assert_eq!(entries(&SourceMetadata::default()), []);
}

#[test]
fn the_recommended_keys_are_the_vocabulary_table_in_order() {
    assert_eq!(
        RECOMMENDED_KEYS,
        [
            "doi",
            "arxiv_id",
            "openalex_id",
            "pmid",
            "issn",
            "published",
            "updated",
            "accessed",
            "type",
            "authors",
            "publisher",
            "volume",
            "issue",
            "pages",
            "license",
            "journal_ref",
            "primary_category",
            "oa_status",
            "pdf_url",
        ]
    );
}

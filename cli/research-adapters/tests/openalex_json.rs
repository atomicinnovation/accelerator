#![allow(clippy::expect_used, clippy::panic)]

use research::fetch::DecodeFailure;
use research::fetch::OpenAlexDecoder as _;
use research::openalex::Location;
use research::openalex::Source;
use research::openalex::Work;
use research_adapters::openalex_json::JsonOpenAlexDecoder;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/openalex")
        .join(name);
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn work(name: &str) -> Work {
    JsonOpenAlexDecoder
        .work(&fixture(name))
        .unwrap_or_else(|failure| panic!("{name}: {failure:?}"))
}

#[test]
fn a_recorded_page_of_works_decodes_every_result_in_order() {
    let works = JsonOpenAlexDecoder
        .works(&fixture("search-30.json"))
        .expect("a page of works");

    assert_eq!(works.len(), 30);
    assert_eq!(works[0].id, "https://openalex.org/W2116341502");
    assert_eq!(
        works[0].doi.as_deref(),
        Some("https://doi.org/10.1109/tnn.2008.2005605")
    );
    assert_eq!(works[0].work_type, "article");
    assert_eq!(works[0].authors.len(), 5);
}

#[test]
fn a_recorded_work_decodes_its_venue_signals_and_abstract() {
    let work = work("work-doi.json");

    assert_eq!(work.id, "https://openalex.org/W2741809807");
    assert_eq!(
        work.doi.as_deref(),
        Some("https://doi.org/10.7717/peerj.4375")
    );
    assert_eq!(
        work.authors.first().map(String::as_str),
        Some("Heather Piwowar")
    );
    assert!(!work.is_retracted);
    let location = work.primary_location.expect("a primary location");
    assert_eq!(location.version.as_deref(), Some("publishedVersion"));
    let source = location.source.expect("a source");
    assert_eq!(source.display_name.as_deref(), Some("PeerJ"));
    assert_eq!(source.source_type.as_deref(), Some("journal"));
    assert!(source.is_core);
    assert!(source.listed_in.iter().any(|list| list == "medline"));
    assert!(work
        .abstract_inverted_index
        .is_some_and(|index| !index.is_empty()));
}

#[test]
fn a_recorded_work_without_a_doi_decodes_its_repository_source() {
    let work = work("work-no-doi.json");

    assert_eq!(work.doi, None);
    assert_eq!(
        work.primary_location
            .and_then(|location| location.source)
            .and_then(|source| source.source_type)
            .as_deref(),
        Some("repository")
    );
}

#[test]
fn a_recorded_retracted_work_is_retracted() {
    assert!(work("work-retracted.json").is_retracted);
}

#[test]
fn null_locations_sources_and_abstracts_decode_as_absent() {
    let body = br#"{"id":"https://openalex.org/W1","doi":null,
        "display_name":null,"type":"article","authorships":[],
        "primary_location":null,"is_retracted":false,
        "abstract_inverted_index":null}"#;

    assert_eq!(
        JsonOpenAlexDecoder.work(body),
        Ok(Work {
            id: "https://openalex.org/W1".to_owned(),
            work_type: "article".to_owned(),
            ..Work::default()
        })
    );
}

#[test]
fn a_location_without_a_source_keeps_its_version() {
    let body = br#"{"id":"https://openalex.org/W1","type":"article",
        "primary_location":{"version":"submittedVersion","source":null}}"#;

    assert_eq!(
        JsonOpenAlexDecoder
            .work(body)
            .map(|work| work.primary_location),
        Ok(Some(Location {
            version: Some("submittedVersion".to_owned()),
            source: None,
        }))
    );
}

#[test]
fn a_source_with_null_flags_is_neither_core_nor_listed() {
    let body = br#"{"id":"https://openalex.org/W1","type":"article",
        "primary_location":{"source":{"type":"journal","is_core":null,
        "listed_in":null,"display_name":"J"}}}"#;

    assert_eq!(
        JsonOpenAlexDecoder
            .work(body)
            .map(|work| work.primary_location.and_then(|at| at.source)),
        Ok(Some(Source {
            display_name: Some("J".to_owned()),
            source_type: Some("journal".to_owned()),
            is_core: false,
            listed_in: Vec::new(),
        }))
    );
}

#[test]
fn authors_without_a_display_name_are_skipped() {
    let body = br#"{"id":"https://openalex.org/W1","type":"article",
        "authorships":[{"author":{"display_name":"A"}},{"author":{}},
        {"author":{"display_name":null}}]}"#;

    assert_eq!(
        JsonOpenAlexDecoder.work(body).map(|work| work.authors),
        Ok(vec!["A".to_owned()])
    );
}

#[test]
fn an_abstract_index_decodes_every_word_and_position() {
    let body = br#"{"id":"https://openalex.org/W1","type":"article",
        "abstract_inverted_index":{"graphs":[1],"Neural":[0,2]}}"#;
    let mut index = JsonOpenAlexDecoder
        .work(body)
        .expect("a work")
        .abstract_inverted_index
        .expect("an index");
    index.sort();

    assert_eq!(
        index,
        vec![
            ("Neural".to_owned(), vec![0, 2]),
            ("graphs".to_owned(), vec![1])
        ]
    );
}

#[test]
fn a_body_that_is_not_json_is_undecodable() {
    assert_eq!(
        JsonOpenAlexDecoder.works(b"<html>busy</html>"),
        Err(DecodeFailure::Undecodable)
    );
    assert_eq!(
        JsonOpenAlexDecoder.work(b""),
        Err(DecodeFailure::Undecodable)
    );
}

#[test]
fn a_work_without_an_id_is_undecodable() {
    assert_eq!(
        JsonOpenAlexDecoder.work(br#"{"type":"article"}"#),
        Err(DecodeFailure::Undecodable)
    );
    assert_eq!(
        JsonOpenAlexDecoder.works(br#"{"results":[{"type":"article"}]}"#),
        Err(DecodeFailure::Undecodable)
    );
}

#[test]
fn a_single_work_is_not_a_page_and_a_page_is_not_a_work() {
    assert_eq!(
        JsonOpenAlexDecoder.works(&fixture("work-doi.json")),
        Err(DecodeFailure::Undecodable)
    );
    assert_eq!(
        JsonOpenAlexDecoder.work(&fixture("search-30.json")),
        Err(DecodeFailure::Undecodable)
    );
}

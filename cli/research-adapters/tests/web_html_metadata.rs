#![allow(clippy::expect_used, clippy::panic)]

use research::sources::metadata::date::PartialDate;
use research::sources::web::declarations::PageDecoder as _;
use research::sources::web::extract::declared_metadata;
use research_adapters::web_html::WebHtmlDecoder;

fn metadata_of(name: &str) -> Vec<(String, String)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/web")
        .join(name);
    let html = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let accessed = PartialDate::parse("2027-01-15").expect("valid date");
    declared_metadata(&WebHtmlDecoder.declarations(&html), accessed)
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

#[test]
fn a_highwire_page_yields_its_doi_and_publication_date() {
    assert_eq!(
        metadata_of("highwire.html"),
        pairs(&[
            ("doi", "10.7717/peerj.4375"),
            ("published", "2018-02-13"),
            ("accessed", "2027-01-15"),
        ])
    );
}

#[test]
fn a_property_value_identifier_yields_the_doi() {
    assert_eq!(
        metadata_of("linked-data-values.html"),
        pairs(&[("doi", "10.1000/ld1"), ("accessed", "2027-01-15")])
    );
}

#[test]
fn the_article_in_a_graph_yields_its_publication_date() {
    assert_eq!(
        metadata_of("linked-data-graph.html"),
        pairs(&[("published", "2020-01-02"), ("accessed", "2027-01-15")])
    );
}

#[test]
fn a_date_published_beside_markup_in_a_string_is_read() {
    assert_eq!(
        metadata_of("linked-data-markup-in-string.html"),
        pairs(&[("published", "2019-03-04"), ("accessed", "2027-01-15")])
    );
}

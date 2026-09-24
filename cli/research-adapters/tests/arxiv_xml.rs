#![allow(clippy::expect_used, clippy::panic)]

use research::arxiv::Entry;
use research::arxiv::RawVersion;
use research::fetch::ArxivDecoder as _;
use research::fetch::DecodeFailure;
use research_adapters::arxiv_xml::XmlArxivDecoder;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/arxiv")
        .join(name);
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn entries(body: &[u8]) -> Vec<Entry> {
    XmlArxivDecoder
        .entries(body)
        .unwrap_or_else(|failure| panic!("{failure:?}"))
}

fn versions(body: &[u8]) -> Vec<RawVersion> {
    XmlArxivDecoder
        .raw_versions(body)
        .unwrap_or_else(|failure| panic!("{failure:?}"))
}

fn version(size: &str, source_type: Option<&str>) -> RawVersion {
    RawVersion {
        size: size.to_owned(),
        source_type: source_type.map(str::to_owned),
    }
}

#[test]
fn a_recorded_search_feed_decodes_every_entry_in_order() {
    let entries = entries(&fixture("search-3.xml"));

    assert_eq!(entries.len(), 3);
    let first = &entries[0];
    assert_eq!(first.id, "http://arxiv.org/abs/2211.12792v2");
    assert_eq!(
        first.title,
        "MECCH: Metapath Context Convolution-based Heterogeneous Graph \
         Neural Networks"
    );
    assert_eq!(first.authors, ["Xinyu Fu", "Irwin King"]);
    assert_eq!(
        first.journal_ref.as_deref(),
        Some("Neural Networks 170 (2024) 266-275")
    );
    assert_eq!(first.doi.as_deref(), Some("10.1016/j.neunet.2023.11.030"));
    assert_eq!(first.primary_category.as_deref(), Some("cs.LG"));
    assert!(first
        .summary
        .as_deref()
        .is_some_and(|summary| summary.starts_with("Heterogeneous graph")));
    assert!(first
        .comment
        .as_deref()
        .is_some_and(|comment| comment.starts_with("12 pages")));
    assert_eq!(entries[1].doi, None);
    assert_eq!(entries[1].journal_ref, None);
}

#[test]
fn the_recorded_withdrawn_lookup_carries_its_withdrawal_comment() {
    let entries = entries(&fixture("lookup-2608.21129.xml"));

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "http://arxiv.org/abs/2608.21129v2");
    assert_eq!(entries[0].authors, ["Qingtian Liu"]);
    assert!(entries[0].is_withdrawal_candidate());
}

#[test]
fn an_empty_feed_decodes_to_no_entries() {
    assert!(entries(&fixture("empty.xml")).is_empty());
}

#[test]
fn a_feed_whose_single_entry_is_titled_error_is_an_error_feed() {
    assert_eq!(
        XmlArxivDecoder.entries(&fixture("error.xml")),
        Err(DecodeFailure::ErrorFeed(
            "incorrect id format for 2608.2112x".to_owned()
        ))
    );
}

#[test]
fn elements_are_matched_by_namespace_whatever_their_prefix_or_order() {
    let feed = br#"<?xml version="1.0"?>
<a:feed xmlns:a="http://www.w3.org/2005/Atom" xmlns:x="http://arxiv.org/schemas/atom">
  <a:entry>
    <x:doi>10.1000/xyz</x:doi>
    <a:author><a:name>Ada Lovelace</a:name></a:author>
    <a:title>Notes</a:title>
    <x:primary_category term="math.HO"/>
    <a:id>http://arxiv.org/abs/2601.00001v1</a:id>
    <title>An unqualified impostor</title>
  </a:entry>
</a:feed>"#;

    let entries = entries(feed);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "http://arxiv.org/abs/2601.00001v1");
    assert_eq!(entries[0].title, "Notes");
    assert_eq!(entries[0].authors, ["Ada Lovelace"]);
    assert_eq!(entries[0].doi.as_deref(), Some("10.1000/xyz"));
    assert_eq!(entries[0].primary_category.as_deref(), Some("math.HO"));
}

#[test]
fn whitespace_in_titles_and_summaries_collapses_to_single_spaces() {
    let feed = br#"<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>http://arxiv.org/abs/2601.00001v1</id>
    <title>  A title
      wrapped   over lines </title>
    <summary>
  A summary
  wrapped too.
    </summary>
  </entry>
</feed>"#;

    let entries = entries(feed);

    assert_eq!(entries[0].title, "A title wrapped over lines");
    assert_eq!(
        entries[0].summary.as_deref(),
        Some("A summary wrapped too.")
    );
}

#[test]
fn a_body_that_is_not_xml_is_undecodable() {
    assert_eq!(
        XmlArxivDecoder.entries(b"Rate exceeded."),
        Err(DecodeFailure::Undecodable)
    );
}

#[test]
fn a_truncated_feed_is_undecodable() {
    let feed = fixture("search-3.xml");
    let truncated = &feed[..feed.len() / 2];

    assert_eq!(
        XmlArxivDecoder.entries(truncated),
        Err(DecodeFailure::Undecodable)
    );
}

#[test]
fn a_document_that_is_not_an_atom_feed_is_undecodable() {
    assert_eq!(
        XmlArxivDecoder.entries(&fixture("oai-2608.21129.xml")),
        Err(DecodeFailure::Undecodable)
    );
}

#[test]
fn an_entry_missing_its_id_or_title_is_undecodable() {
    let without_id = br#"<feed xmlns="http://www.w3.org/2005/Atom">
  <entry><title>Untitled</title></entry>
</feed>"#;
    let without_title = br#"<feed xmlns="http://www.w3.org/2005/Atom">
  <entry><id>http://arxiv.org/abs/2601.00001v1</id></entry>
</feed>"#;

    assert_eq!(
        XmlArxivDecoder.entries(without_id),
        Err(DecodeFailure::Undecodable)
    );
    assert_eq!(
        XmlArxivDecoder.entries(without_title),
        Err(DecodeFailure::Undecodable)
    );
}

#[test]
fn a_document_type_declaration_is_refused_rather_than_expanded() {
    let feed = br#"<?xml version="1.0"?>
<!DOCTYPE feed [<!ENTITY a "aaaaaaaaaa">]>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry><id>http://arxiv.org/abs/2601.00001v1</id><title>&a;</title></entry>
</feed>"#;

    assert_eq!(
        XmlArxivDecoder.entries(feed),
        Err(DecodeFailure::Undecodable)
    );
}

#[test]
fn the_recorded_withdrawal_record_ends_with_an_empty_type_i_version() {
    assert_eq!(
        versions(&fixture("oai-2608.21129.xml")),
        [version("337kb", None), version("0kb", Some("I"))]
    );
}

#[test]
fn a_recorded_live_record_lists_its_versions_in_document_order() {
    assert_eq!(
        versions(&fixture("oai-2211.12792.xml")),
        [version("4245kb", Some("D")), version("3709kb", Some("D"))]
    );
}

#[test]
fn an_oai_error_is_an_error_feed_naming_its_code() {
    let body = br#"<?xml version="1.0" encoding="UTF-8"?>
<OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/">
  <request verb="GetRecord">http://oaipmh.arxiv.org/oai</request>
  <error code="idDoesNotExist">No matching identifier</error>
</OAI-PMH>"#;

    assert_eq!(
        XmlArxivDecoder.raw_versions(body),
        Err(DecodeFailure::ErrorFeed(
            "idDoesNotExist: No matching identifier".to_owned()
        ))
    );
}

#[test]
fn a_record_that_is_not_arxiv_raw_is_undecodable() {
    assert_eq!(
        XmlArxivDecoder.raw_versions(&fixture("search-3.xml")),
        Err(DecodeFailure::Undecodable)
    );
    assert_eq!(
        XmlArxivDecoder.raw_versions(b"<html>"),
        Err(DecodeFailure::Undecodable)
    );
}

#[test]
fn a_version_without_a_size_is_undecodable() {
    let body = br#"<OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/">
  <GetRecord><record><metadata>
    <arXivRaw xmlns="http://arxiv.org/OAI/arXivRaw/">
      <version version="v1"><date>Mon, 1 Jan 2026</date></version>
    </arXivRaw>
  </metadata></record></GetRecord>
</OAI-PMH>"#;

    assert_eq!(
        XmlArxivDecoder.raw_versions(body),
        Err(DecodeFailure::Undecodable)
    );
}

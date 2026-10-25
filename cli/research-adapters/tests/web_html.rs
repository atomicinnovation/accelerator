#![allow(clippy::panic)]

use research::sources::web::declarations::LinkedDataNode;
use research::sources::web::declarations::LinkedDataValue;
use research::sources::web::declarations::MetaTag;
use research::sources::web::declarations::PageDeclarations;
use research::sources::web::declarations::PageDecoder as _;
use research_adapters::web_html::WebHtmlDecoder;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/web")
        .join(name);
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn declarations(name: &str) -> PageDeclarations {
    WebHtmlDecoder.declarations(&fixture(name))
}

fn meta(name: &str, content: &str) -> MetaTag {
    MetaTag {
        name: name.to_owned(),
        content: content.to_owned(),
    }
}

fn text(value: &str) -> LinkedDataValue {
    LinkedDataValue::Text(value.to_owned())
}

fn node(
    types: &[&str],
    properties: &[(&str, LinkedDataValue)],
) -> LinkedDataNode {
    LinkedDataNode {
        types: types.iter().map(|kind| (*kind).to_owned()).collect(),
        properties: properties
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect(),
    }
}

#[test]
fn highwire_tags_with_content_are_read_in_document_order() {
    assert_eq!(
        declarations("highwire.html").meta_tags,
        [
            meta("citation_title", "A study & its data"),
            meta("citation_doi", "10.7717/peerj.4375"),
            meta("citation_publication_date", "2018/02/13"),
            meta("citation_journal_title", "PeerJ"),
            meta("description", "Named before property"),
        ]
    );
}

#[test]
fn opengraph_tags_are_named_by_their_property() {
    assert_eq!(
        declarations("opengraph.html").meta_tags,
        [
            meta("og:title", "An OpenGraph page"),
            meta("article:published_time", "2021-07-08T12:00:00+00:00"),
            meta("article:modified_time", "2021-07-09T08:30:00+00:00"),
        ]
    );
}

#[test]
fn markup_inside_a_linked_data_string_stays_text() {
    assert_eq!(
        declarations("linked-data-markup-in-string.html").linked_data,
        [node(
            &["ScholarlyArticle"],
            &[
                ("abstract", text("Uses <b>bold</b> and </p> inside")),
                ("datePublished", text("2019-03-04")),
            ],
        )]
    );
}

#[test]
fn a_graph_lists_its_container_then_each_member() {
    assert_eq!(
        declarations("linked-data-graph.html").linked_data,
        [
            node(&[], &[("@context", text("https://schema.org"))]),
            node(&["WebSite"], &[("name", text("A site"))]),
            node(&["Article"], &[("datePublished", text("2020-01-02"))]),
        ]
    );
}

#[test]
fn malformed_markup_still_yields_its_meta_tags() {
    assert_eq!(
        declarations("malformed.html").meta_tags,
        [
            meta("citation_doi", "10.1000/UNQUOTED"),
            meta("citation_date", "2011-06-07"),
            meta("DC.Identifier", "doi:10.1000/DC1"),
        ]
    );
}

#[test]
fn bytes_that_are_not_utf8_do_not_stop_the_read() {
    let html = b"<title>caf\xe9</title>\
        <meta name=\"citation_doi\" content=\"10.1000/AFTER\">";

    assert_eq!(
        WebHtmlDecoder.declarations(html).meta_tags,
        [meta("citation_doi", "10.1000/AFTER")]
    );
}

#[test]
fn an_unparseable_script_is_skipped_and_a_later_one_read() {
    assert_eq!(
        declarations("linked-data-unparseable-then-valid.html").linked_data,
        [node(&["Report"], &[("datePublished", text("2022-02-03"))])]
    );
}

#[test]
fn types_are_read_from_a_string_or_an_array_and_may_be_absent() {
    assert_eq!(
        declarations("linked-data-types.html").linked_data,
        [
            node(&["Article"], &[("name", text("string type"))]),
            node(&["Thing", "BlogPosting"], &[("name", text("array type"))]),
            node(&[], &[("name", text("no type"))]),
        ]
    );
}

#[test]
fn nested_objects_stay_nodes_scalars_become_text_and_nulls_drop() {
    assert_eq!(
        declarations("linked-data-values.html").linked_data,
        [node(
            &["ScholarlyArticle"],
            &[
                (
                    "author",
                    LinkedDataValue::List(vec![LinkedDataValue::Node(node(
                        &["Person"],
                        &[("name", text("A. Author"))],
                    ))]),
                ),
                (
                    "identifier",
                    LinkedDataValue::Node(node(
                        &["PropertyValue"],
                        &[
                            ("propertyID", text("doi")),
                            ("value", text("10.1000/LD1")),
                        ],
                    )),
                ),
                ("isAccessibleForFree", text("true")),
                ("pageStart", text("12")),
            ],
        )]
    );
}

#![allow(clippy::expect_used, clippy::panic)]

use research::sources::crossref::CrossrefWork;
use research::sources::fetch::CrossrefDecoder as _;
use research_adapters::crossref_json::JsonCrossrefDecoder;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/crossref")
        .join(name);
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn work(name: &str) -> CrossrefWork {
    decoded(&fixture(name))
}

fn decoded(body: &[u8]) -> CrossrefWork {
    JsonCrossrefDecoder
        .work(body)
        .unwrap_or_else(|failure| panic!("{failure:?}"))
}

fn titles(titles: &[&str]) -> Vec<String> {
    titles.iter().map(|title| (*title).to_owned()).collect()
}

#[test]
fn a_recorded_proceedings_article_decodes_its_type_title_and_publisher() {
    assert_eq!(
        work("proceedings-371920-372066.json"),
        CrossrefWork {
            kind: Some("proceedings-article".to_owned()),
            container_titles: titles(&[
                "Proceedings of the 10th international conference on World \
                 Wide Web"
            ]),
            publisher: Some("ACM".to_owned()),
        }
    );
}

#[test]
fn a_recorded_journal_article_keeps_both_container_titles_in_order() {
    assert_eq!(
        work("journal-two-container-titles.json").container_titles,
        titles(&[
            "Теоретическая и математическая физика",
            "Teoreticheskaya i Matematicheskaya Fizika",
        ])
    );
}

#[test]
fn a_recorded_book_chapter_decodes_its_book_and_publisher() {
    assert_eq!(
        work("book-chapter.json"),
        CrossrefWork {
            kind: Some("book-chapter".to_owned()),
            container_titles: titles(&["Clinical Pharmacology in Psychiatry"]),
            publisher: Some("Palgrave Macmillan UK".to_owned()),
        }
    );
}

#[test]
fn an_absent_or_empty_container_title_decodes_to_no_titles() {
    for name in [
        "publisher-no-container-title.json",
        "publisher-empty-container-title.json",
    ] {
        let work = work(name);
        assert_eq!(work.container_titles, Vec::<String>::new(), "{name}");
        assert_eq!(work.publisher.as_deref(), Some("ACM"), "{name}");
    }
}

#[test]
fn a_bare_record_decodes_to_its_type_alone() {
    assert_eq!(
        work("bare.json"),
        CrossrefWork {
            kind: Some("journal-article".to_owned()),
            ..CrossrefWork::default()
        }
    );
}

#[test]
fn null_or_unexpected_fields_decode_as_absent() {
    for body in [
        r#"{"message":{"container-title":null,"publisher":null}}"#,
        r#"{"message":{"container-title":"Nature","publisher":7}}"#,
        r#"{"message":{"container-title":["Nature",3],"type":null}}"#,
    ] {
        let work = decoded(body.as_bytes());
        assert_eq!(work.kind, None, "{body}");
        assert_eq!(work.publisher, None, "{body}");
    }
    assert_eq!(
        decoded(br#"{"message":{"container-title":["Nature",3]}}"#)
            .container_titles,
        titles(&["Nature"])
    );
}

#[test]
fn a_body_without_a_message_is_unreadable_and_says_why() {
    for (body, why) in [
        (&br#"{"status":"ok"}"#[..], "missing field `message`"),
        (b"Resource not found.", "expected value"),
        (b"", "EOF while parsing"),
    ] {
        let failure = JsonCrossrefDecoder
            .work(body)
            .expect_err("unreadable")
            .to_string();
        assert!(
            failure.contains(why),
            "{}: {failure}",
            String::from_utf8_lossy(body)
        );
    }
}

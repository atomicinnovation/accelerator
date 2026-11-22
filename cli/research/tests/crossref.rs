use research::sources::crossref::normalise;
use research::sources::crossref::CrossrefWork;
use research::sources::crossref::DoiMetadata;
use research::sources::crossref::Venue;

fn work(kind: &str, titles: &[&str], publisher: Option<&str>) -> CrossrefWork {
    CrossrefWork {
        kind: Some(kind.to_owned()),
        container_titles: titles
            .iter()
            .map(|title| (*title).to_owned())
            .collect(),
        publisher: publisher.map(str::to_owned),
    }
}

fn venue(metadata: &DoiMetadata) -> Option<&str> {
    metadata.venue.as_ref().map(Venue::as_str)
}

fn entries(metadata: &DoiMetadata) -> Vec<(&str, &str)> {
    metadata.metadata.entries().collect()
}

#[test]
fn a_journal_article_takes_its_container_title_as_its_venue() {
    let normalised =
        normalise(&work("journal-article", &["Nature"], Some("Springer")));

    assert_eq!(venue(&normalised), Some("Nature"));
    assert_eq!(entries(&normalised), [("publisher", "Springer")]);
}

#[test]
fn a_proceedings_article_takes_its_container_title_as_its_venue() {
    let normalised = normalise(&work(
        "proceedings-article",
        &["Proceedings of the 10th international conference on World Wide Web"],
        Some("ACM"),
    ));

    assert_eq!(
        venue(&normalised),
        Some("Proceedings of the 10th international conference on World Wide Web")
    );
}

#[test]
fn of_two_container_titles_the_first_is_the_venue() {
    let normalised =
        normalise(&work("journal-article", &["First", "Second"], None));

    assert_eq!(venue(&normalised), Some("First"));
}

#[test]
fn a_venue_has_its_whitespace_collapsed() {
    let normalised =
        normalise(&work("journal-article", &["  Annals of\n  Physics "], None));

    assert_eq!(venue(&normalised), Some("Annals of Physics"));
}

#[test]
fn a_blank_first_container_title_is_no_venue() {
    let normalised =
        normalise(&work("journal-article", &["  ", "Second"], None));

    assert_eq!(venue(&normalised), None);
}

#[test]
fn a_book_chapter_has_no_venue_but_keeps_its_publisher() {
    let normalised = normalise(&work(
        "book-chapter",
        &["Clinical Pharmacology in Psychiatry"],
        Some("Palgrave Macmillan UK"),
    ));

    assert_eq!(venue(&normalised), None);
    assert_eq!(
        entries(&normalised),
        [("publisher", "Palgrave Macmillan UK")]
    );
}

#[test]
fn a_work_of_no_type_has_no_venue() {
    let normalised = normalise(&CrossrefWork {
        kind: None,
        ..work("journal-article", &["Nature"], None)
    });

    assert_eq!(venue(&normalised), None);
}

#[test]
fn an_article_without_container_titles_has_no_venue() {
    let normalised = normalise(&work("journal-article", &[], Some("ACM")));

    assert_eq!(venue(&normalised), None);
    assert_eq!(entries(&normalised), [("publisher", "ACM")]);
}

#[test]
fn a_work_without_a_publisher_has_empty_metadata() {
    let normalised = normalise(&work("journal-article", &["Nature"], None));

    assert_eq!(entries(&normalised), []);
}

#[test]
fn a_blank_publisher_is_skipped() {
    let normalised = normalise(&work("journal-article", &[], Some("  ")));

    assert_eq!(entries(&normalised), []);
}

#[test]
fn a_miss_is_empty_metadata_with_no_venue() {
    let miss = DoiMetadata::default();

    assert_eq!(venue(&miss), None);
    assert_eq!(entries(&miss), []);
}

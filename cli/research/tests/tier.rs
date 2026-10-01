use research::arxiv;
use research::arxiv::Entry;
use research::openalex::Location;
use research::openalex::Source;
use research::openalex::Work;
use research::record::Tier;
use research::tier::arxiv_tier;
use research::tier::openalex_tier;

struct Venue {
    source_type: &'static str,
    is_core: bool,
    medline: bool,
}

const JOURNAL: Venue = Venue {
    source_type: "journal",
    is_core: false,
    medline: false,
};
const CORE_JOURNAL: Venue = Venue {
    source_type: "journal",
    is_core: true,
    medline: false,
};
const CONFERENCE: Venue = Venue {
    source_type: "conference",
    is_core: false,
    medline: false,
};
const REPOSITORY: Venue = Venue {
    source_type: "repository",
    is_core: false,
    medline: false,
};
const CORE_REPOSITORY: Venue = Venue {
    source_type: "repository",
    is_core: true,
    medline: false,
};
const MEDLINE_JOURNAL: Venue = Venue {
    source_type: "journal",
    is_core: false,
    medline: true,
};
const BOOK_SERIES: Venue = Venue {
    source_type: "book series",
    is_core: false,
    medline: false,
};

fn work(work_type: &str, venue: Option<&Venue>, version: Option<&str>) -> Work {
    Work {
        id: "https://openalex.org/W1".to_owned(),
        work_type: work_type.to_owned(),
        primary_location: Some(Location {
            version: version.map(str::to_owned),
            source: venue.map(|venue| Source {
                display_name: Some("Venue".to_owned()),
                source_type: Some(venue.source_type.to_owned()),
                is_core: venue.is_core,
                listed_in: if venue.medline {
                    vec!["doaj".to_owned(), "medline".to_owned()]
                } else {
                    vec!["doaj".to_owned()]
                },
            }),
        }),
        ..Work::default()
    }
}

#[test]
fn a_published_journal_article_is_tier_one() {
    let article = work("article", Some(&JOURNAL), Some("publishedVersion"));
    assert_eq!(openalex_tier(&article), Tier::One);
}

#[test]
fn an_accepted_conference_article_is_tier_one() {
    let article = work("article", Some(&CONFERENCE), Some("acceptedVersion"));
    assert_eq!(openalex_tier(&article), Tier::One);
}

#[test]
fn a_submitted_article_in_a_core_journal_is_tier_one() {
    let article =
        work("article", Some(&CORE_JOURNAL), Some("submittedVersion"));
    assert_eq!(openalex_tier(&article), Tier::One);
}

#[test]
fn a_versionless_article_in_a_medline_source_is_tier_one() {
    let article = work("article", Some(&MEDLINE_JOURNAL), None);
    assert_eq!(openalex_tier(&article), Tier::One);
}

#[test]
fn a_preprint_in_a_journal_is_tier_two_whatever_its_version() {
    let preprint = work("preprint", Some(&JOURNAL), Some("publishedVersion"));
    assert_eq!(openalex_tier(&preprint), Tier::Two);
}

#[test]
fn a_preprint_in_a_core_repository_is_tier_two() {
    let preprint =
        work("preprint", Some(&CORE_REPOSITORY), Some("submittedVersion"));
    assert_eq!(openalex_tier(&preprint), Tier::Two);
}

#[test]
fn an_article_in_an_uncurated_repository_is_tier_two() {
    let article = work("article", Some(&REPOSITORY), Some("publishedVersion"));
    assert_eq!(openalex_tier(&article), Tier::Two);
}

#[test]
fn a_submitted_article_in_an_uncurated_journal_is_tier_two() {
    let article = work("article", Some(&JOURNAL), Some("submittedVersion"));
    assert_eq!(openalex_tier(&article), Tier::Two);
}

#[test]
fn a_submitted_conference_article_is_tier_two() {
    let article = work("article", Some(&CONFERENCE), Some("submittedVersion"));
    assert_eq!(openalex_tier(&article), Tier::Two);
}

#[test]
fn a_retracted_article_is_tier_three_even_in_a_core_journal() {
    let retracted = Work {
        is_retracted: true,
        ..work("article", Some(&CORE_JOURNAL), Some("publishedVersion"))
    };
    assert_eq!(openalex_tier(&retracted), Tier::Three);
}

#[test]
fn a_versionless_article_in_an_uncurated_journal_is_tier_three() {
    let article = work("article", Some(&JOURNAL), None);
    assert_eq!(openalex_tier(&article), Tier::Three);
}

#[test]
fn an_article_with_no_primary_source_is_tier_three() {
    let sourceless = work("article", None, Some("publishedVersion"));
    let locationless = Work {
        primary_location: None,
        ..work("article", None, None)
    };
    assert_eq!(openalex_tier(&sourceless), Tier::Three);
    assert_eq!(openalex_tier(&locationless), Tier::Three);
}

#[test]
fn a_preprint_with_no_primary_source_is_tier_two() {
    let preprint = work("preprint", None, None);
    assert_eq!(openalex_tier(&preprint), Tier::Two);
}

#[test]
fn a_published_chapter_in_an_uncurated_book_series_is_tier_three() {
    let chapter =
        work("book-chapter", Some(&BOOK_SERIES), Some("publishedVersion"));
    assert_eq!(openalex_tier(&chapter), Tier::Three);
}

fn arxiv_entry(journal_ref: Option<&str>, doi: Option<&str>) -> Entry {
    Entry {
        id: "http://arxiv.org/abs/2608.21129v1".to_owned(),
        title: "Entry".to_owned(),
        journal_ref: journal_ref.map(str::to_owned),
        doi: doi.map(str::to_owned),
        ..Entry::default()
    }
}

fn arxiv_record_tier(entry: &Entry, withdrawn: bool) -> Option<Tier> {
    arxiv::normalise(entry, withdrawn).map(|record| record.tier())
}

#[test]
fn an_arxiv_entry_with_a_journal_ref_and_doi_is_tier_two() {
    let entry = arxiv_entry(Some("J. Attn. 1 (2026)"), Some("10.1234/attn"));
    assert_eq!(arxiv_record_tier(&entry, false), Some(Tier::Two));
}

#[test]
fn an_arxiv_entry_with_neither_is_tier_two() {
    let entry = arxiv_entry(None, None);
    assert_eq!(arxiv_record_tier(&entry, false), Some(Tier::Two));
}

#[test]
fn a_withdrawn_arxiv_entry_is_tier_three() {
    let entry = arxiv_entry(Some("J. Attn. 1 (2026)"), Some("10.1234/attn"));
    assert_eq!(arxiv_record_tier(&entry, true), Some(Tier::Three));
    assert_eq!(arxiv_tier(true), Tier::Three);
}

#[test]
fn a_tier_renders_as_its_citation_label() {
    assert_eq!(Tier::One.to_string(), "tier-1");
    assert_eq!(Tier::Two.to_string(), "tier-2");
    assert_eq!(Tier::Three.to_string(), "tier-3");
}

//! Which declarations supply each key, and in what order they are tried:
//! schemes in precedence order, then each scheme's fields in order.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Highwire,
    LinkedData,
    DublinCore,
    OpenGraph,
}

impl Scheme {
    pub const PRECEDENCE: [Self; 4] = [
        Self::Highwire,
        Self::LinkedData,
        Self::DublinCore,
        Self::OpenGraph,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebKey {
    Doi,
    Published,
    Updated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// A meta tag name, matched whatever its ASCII case.
    Meta(&'static str),
    LinkedData(LinkedDataField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkedDataField {
    /// A property whose value is text.
    Text(&'static str),
    /// `identifier`: text, a `PropertyValue` whose `propertyID` is `doi`, or
    /// a list of either.
    Identifier,
    /// `sameAs`: text or a list of text.
    SameAs,
}

/// The fields a scheme supplies a key from, in the order they are tried.
pub type SchemeFields = (Scheme, &'static [Field]);

/// The schemes a key is read from, in precedence order.
pub type KeyPrecedence = (WebKey, &'static [SchemeFields]);

pub const WEB_KEYS: [KeyPrecedence; 3] = [
    (
        WebKey::Doi,
        &[
            (Scheme::Highwire, &[Field::Meta("citation_doi")]),
            (
                Scheme::LinkedData,
                &[
                    Field::LinkedData(LinkedDataField::Identifier),
                    Field::LinkedData(LinkedDataField::SameAs),
                ],
            ),
            (
                Scheme::DublinCore,
                &[
                    Field::Meta("dc.identifier"),
                    Field::Meta("dcterms.identifier"),
                ],
            ),
        ],
    ),
    (
        WebKey::Published,
        &[
            (
                Scheme::Highwire,
                &[
                    Field::Meta("citation_publication_date"),
                    Field::Meta("citation_date"),
                    Field::Meta("citation_online_date"),
                ],
            ),
            (
                Scheme::LinkedData,
                &[Field::LinkedData(LinkedDataField::Text("datePublished"))],
            ),
            (
                Scheme::DublinCore,
                &[Field::Meta("dcterms.issued"), Field::Meta("dc.date")],
            ),
            (Scheme::OpenGraph, &[Field::Meta("article:published_time")]),
        ],
    ),
    (
        WebKey::Updated,
        &[
            (
                Scheme::LinkedData,
                &[Field::LinkedData(LinkedDataField::Text("dateModified"))],
            ),
            (Scheme::DublinCore, &[Field::Meta("dcterms.modified")]),
            (Scheme::OpenGraph, &[Field::Meta("article:modified_time")]),
        ],
    ),
];

/// The JSON-LD types whose first node describes the page's work.
pub const ARTICLE_TYPES: [&str; 11] = [
    "ScholarlyArticle",
    "Article",
    "NewsArticle",
    "BlogPosting",
    "Report",
    "Thesis",
    "Book",
    "Chapter",
    "Dataset",
    "CreativeWork",
    "WebPage",
];

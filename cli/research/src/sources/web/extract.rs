//! The metadata a page's declarations support.
//!
//! For each key, the first declaration in precedence order whose value
//! passes the key's value form wins, except that a date is the most precise
//! usable one within the first scheme declaring any.

use crate::sources::metadata::date::PartialDate;
use crate::sources::metadata::SourceMetadata;
use crate::sources::request::Doi;
use crate::sources::web::declarations::LinkedDataNode;
use crate::sources::web::declarations::LinkedDataValue;
use crate::sources::web::declarations::PageDeclarations;
use crate::sources::web::precedence::Field;
use crate::sources::web::precedence::LinkedDataField;
use crate::sources::web::precedence::WebKey;
use crate::sources::web::precedence::ARTICLE_TYPES;
use crate::sources::web::precedence::WEB_KEYS;

pub fn declared_metadata(
    page: &PageDeclarations,
    accessed: PartialDate,
) -> SourceMetadata {
    let article = page
        .linked_data
        .iter()
        .find(|node| node.has_any_type_of(&ARTICLE_TYPES));
    let mut metadata = SourceMetadata::default();
    for (key, schemes) in WEB_KEYS {
        let mut by_scheme = schemes.iter().map(|(_, fields)| {
            fields
                .iter()
                .flat_map(|field| declarations_of(*field, page, article))
                .map(tidied)
        });
        match key {
            WebKey::Doi => {
                if let Some(doi) =
                    by_scheme.flatten().find_map(|value| Doi::parse(&value))
                {
                    metadata.record_doi(&doi);
                }
            }
            WebKey::Published => {
                if let Some(date) = by_scheme.find_map(most_precise_date) {
                    metadata.record_published(date);
                }
            }
            WebKey::Updated => {
                if let Some(date) = by_scheme.find_map(most_precise_date) {
                    metadata.record_updated(date);
                }
            }
        }
    }
    metadata.record_accessed(accessed);
    metadata
}

/// Of equally precise dates, the first declared.
fn most_precise_date(
    declared: impl Iterator<Item = String>,
) -> Option<PartialDate> {
    declared
        .filter_map(|value| PartialDate::parse(&value))
        .reduce(|kept, next| {
            if next.precision() > kept.precision() {
                next
            } else {
                kept
            }
        })
}

fn declarations_of<'a>(
    field: Field,
    page: &'a PageDeclarations,
    article: Option<&'a LinkedDataNode>,
) -> Vec<&'a str> {
    match field {
        Field::Meta(name) => page
            .meta_tags
            .iter()
            .filter(|tag| tag.name.eq_ignore_ascii_case(name))
            .map(|tag| tag.content.as_str())
            .collect(),
        Field::LinkedData(field) => article
            .map(|article| linked_data_of(field, article))
            .unwrap_or_default(),
    }
}

fn linked_data_of(
    field: LinkedDataField,
    article: &LinkedDataNode,
) -> Vec<&str> {
    match field {
        LinkedDataField::Text(name) => {
            article.text_of(name).into_iter().collect()
        }
        LinkedDataField::Identifier => article
            .property("identifier")
            .map(identifiers)
            .unwrap_or_default(),
        LinkedDataField::SameAs => {
            article.property("sameAs").map(texts).unwrap_or_default()
        }
    }
}

fn identifiers(value: &LinkedDataValue) -> Vec<&str> {
    match value {
        LinkedDataValue::Text(text) => vec![text],
        LinkedDataValue::Node(node) => {
            let names_a_doi = node
                .text_of("propertyID")
                .is_some_and(|id| id.eq_ignore_ascii_case("doi"));
            node.text_of("value")
                .filter(|_| names_a_doi)
                .into_iter()
                .collect()
        }
        LinkedDataValue::List(values) => {
            values.iter().flat_map(identifiers).collect()
        }
    }
}

fn texts(value: &LinkedDataValue) -> Vec<&str> {
    match value {
        LinkedDataValue::Text(text) => vec![text],
        LinkedDataValue::Node(_) => Vec::new(),
        LinkedDataValue::List(values) => values
            .iter()
            .filter_map(|value| match value {
                LinkedDataValue::Text(text) => Some(text.as_str()),
                LinkedDataValue::Node(_) | LinkedDataValue::List(_) => None,
            })
            .collect(),
    }
}

/// Line breaks become single spaces rather than vanishing, so a value
/// broken mid-token stays unusable instead of being silently rejoined.
fn tidied(declared: &str) -> String {
    declared
        .split(['\r', '\n'])
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

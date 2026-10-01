//! The Atom feed the arXiv query API answers with.

use research::arxiv::Entry;
use research::fetch::DecodeFailure;
use roxmltree::Document;
use roxmltree::Node;

use super::child;
use super::children;
use super::collapsed;
use super::is_element;
use super::text_of;

const ATOM: &str = "http://www.w3.org/2005/Atom";
const ARXIV: &str = "http://arxiv.org/schemas/atom";

/// arXiv reports a rejected query as a feed whose one entry is titled
/// `Error`, its summary saying why.
const ERROR_TITLE: &str = "Error";

pub fn entries(document: &Document<'_>) -> Result<Vec<Entry>, DecodeFailure> {
    let feed = document.root_element();
    if !is_element(feed, ATOM, "feed") {
        return Err(DecodeFailure::Undecodable);
    }
    let entries = children(feed, ATOM, "entry")
        .map(entry)
        .collect::<Result<Vec<_>, _>>()?;
    match entries.as_slice() {
        [only] if only.title == ERROR_TITLE => Err(DecodeFailure::ErrorFeed(
            only.summary.clone().unwrap_or_default(),
        )),
        _ => Ok(entries),
    }
}

fn entry(node: Node<'_, '_>) -> Result<Entry, DecodeFailure> {
    let required = |name| {
        child(node, ATOM, name)
            .map(|found| collapsed(&text_of(found)))
            .ok_or(DecodeFailure::Undecodable)
    };
    let optional =
        |namespace, name| child(node, namespace, name).map(text_of_trimmed);
    Ok(Entry {
        id: required("id")?,
        title: required("title")?,
        summary: child(node, ATOM, "summary")
            .map(|summary| collapsed(&text_of(summary))),
        authors: children(node, ATOM, "author")
            .filter_map(|author| child(author, ATOM, "name"))
            .map(text_of_trimmed)
            .collect(),
        comment: optional(ARXIV, "comment"),
        journal_ref: optional(ARXIV, "journal_ref"),
        doi: optional(ARXIV, "doi"),
        primary_category: child(node, ARXIV, "primary_category")
            .and_then(|category| category.attribute("term"))
            .map(str::to_owned),
    })
}

fn text_of_trimmed(node: Node<'_, '_>) -> String {
    text_of(node).trim().to_owned()
}

//! arXiv's XML responses — the Atom query feed and the OAI-PMH `arXivRaw`
//! record — decoded into the domain's typed inputs.
//!
//! Elements are matched by namespace URI, never by prefix or position, so a
//! re-prefixed or reordered response still decodes.

mod arxiv_raw;
mod atom;

use research::arxiv::Entry;
use research::arxiv::RawVersion;
use research::fetch::ArxivDecoder;
use research::fetch::DecodeFailure;
use roxmltree::Document;
use roxmltree::Node;

pub struct XmlArxivDecoder;

impl ArxivDecoder for XmlArxivDecoder {
    fn entries(&self, body: &[u8]) -> Result<Vec<Entry>, DecodeFailure> {
        atom::entries(&parse(body)?)
    }

    fn raw_versions(
        &self,
        body: &[u8],
    ) -> Result<Vec<RawVersion>, DecodeFailure> {
        arxiv_raw::versions(&parse(body)?)
    }
}

fn parse(body: &[u8]) -> Result<Document<'_>, DecodeFailure> {
    let text =
        std::str::from_utf8(body).map_err(|_| DecodeFailure::Undecodable)?;
    Document::parse(text).map_err(|_| DecodeFailure::Undecodable)
}

fn children<'a, 'input: 'a>(
    parent: Node<'a, 'input>,
    namespace: &'static str,
    name: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    parent
        .children()
        .filter(move |child| is_element(*child, namespace, name))
}

fn is_element(node: Node<'_, '_>, namespace: &str, name: &str) -> bool {
    node.tag_name().namespace() == Some(namespace)
        && node.tag_name().name() == name
}

fn child<'a, 'input>(
    parent: Node<'a, 'input>,
    namespace: &'static str,
    name: &'static str,
) -> Option<Node<'a, 'input>> {
    children(parent, namespace, name).next()
}

fn text_of(node: Node<'_, '_>) -> String {
    node.descendants()
        .filter(Node::is_text)
        .filter_map(|text| text.text())
        .collect()
}

/// Feed text is line-wrapped for display, so runs of whitespace compare as
/// one space.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

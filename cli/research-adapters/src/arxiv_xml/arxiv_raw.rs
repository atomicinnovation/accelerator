//! The OAI-PMH `arXivRaw` record whose version history shows a withdrawal.

use research::arxiv::RawVersion;
use research::fetch::DecodeFailure;
use roxmltree::Document;
use roxmltree::Node;

use super::child;
use super::children;
use super::is_element;
use super::text_of;

const OAI: &str = "http://www.openarchives.org/OAI/2.0/";
const ARXIV_RAW: &str = "http://arxiv.org/OAI/arXivRaw/";

pub fn versions(
    document: &Document<'_>,
) -> Result<Vec<RawVersion>, DecodeFailure> {
    let root = document.root_element();
    if root.tag_name().namespace() != Some(OAI) {
        return Err(DecodeFailure::Undecodable);
    }
    if let Some(error) = child(root, OAI, "error") {
        return Err(DecodeFailure::ErrorFeed(format!(
            "{}: {}",
            error.attribute("code").unwrap_or("error"),
            text_of(error).trim()
        )));
    }
    let record = root
        .descendants()
        .find(|node| is_element(*node, ARXIV_RAW, "arXivRaw"))
        .ok_or(DecodeFailure::Undecodable)?;
    children(record, ARXIV_RAW, "version")
        .map(version)
        .collect()
}

fn version(node: Node<'_, '_>) -> Result<RawVersion, DecodeFailure> {
    let field = |name| {
        child(node, ARXIV_RAW, name)
            .map(|found| text_of(found).trim().to_owned())
    };
    Ok(RawVersion {
        size: field("size").ok_or(DecodeFailure::Undecodable)?,
        source_type: field("source_type"),
    })
}

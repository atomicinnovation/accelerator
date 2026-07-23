//! The per-document assembler: reads a document, parses it, and invokes the
//! pure `corpus` conventions. It orchestrates; it holds none of the convention
//! logic itself.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::Path;
use std::path::PathBuf;

use corpus::DocTypeKey;
use corpus::IdScanner;
use corpus::LinkageRecord;
use corpus::WorkItemIdScheme;

use crate::document::FrontmatterState;

/// A document with every derived convention resolved.
#[derive(Debug, Clone)]
pub struct AssembledDocument {
    pub kind: DocTypeKey,
    pub slug: Option<String>,
    pub work_item_id: Option<String>,
    pub state: FrontmatterState,
    pub body: String,
    pub linkage: Vec<LinkageRecord>,
}

/// Assembles `raw` at `path`, or `None` when the path falls outside every
/// configured doc-type directory.
#[must_use]
pub fn assemble<S: BuildHasher>(
    path: &Path,
    raw: &[u8],
    doc_paths: &HashMap<String, PathBuf, S>,
    scheme: &WorkItemIdScheme,
    scanner: &dyn IdScanner,
) -> Option<AssembledDocument> {
    let table = crate::doc_type::table_from_paths(doc_paths);
    let kind = corpus::doc_type::infer(path, &table)?;
    let filename = path.file_name()?.to_str()?;

    let parsed = crate::document::parse(raw);
    let slug = corpus::slug::derive(kind, filename, scheme, scanner);
    let work_item_id = scheme.extract_id(filename, scanner);

    let source_type =
        corpus::linkage::type_from_path(&path.to_string_lossy(), &table)
            .unwrap_or("unknown");
    let linkage =
        corpus::linkage::parse_document(source_type, &parsed.body, &table);

    Some(AssembledDocument {
        kind,
        slug,
        work_item_id,
        state: parsed.state,
        body: parsed.body,
        linkage,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use corpus::{Band, WorkItemIdScheme};

    use super::assemble;
    use crate::scanner::RegexScanner;

    type TestError = Box<dyn std::error::Error>;

    fn doc_paths() -> HashMap<String, PathBuf> {
        let mut paths = HashMap::new();
        paths.insert("work".to_owned(), PathBuf::from("meta/work"));
        paths.insert("plans".to_owned(), PathBuf::from("meta/plans"));
        paths
    }

    #[test]
    fn assembles_a_work_item_end_to_end() -> Result<(), TestError> {
        let scanner = RegexScanner::compile("^([0-9]+)-")?;
        let scheme = WorkItemIdScheme::numeric();
        let raw = b"---\nstatus: ready\n---\n\
                    ## Dependencies\n- Blocks: 0061\n";

        let assembled = assemble(
            Path::new("meta/work/0042-ship-the-thing.md"),
            raw,
            &doc_paths(),
            &scheme,
            &scanner,
        )
        .ok_or("expected the document to assemble")?;

        assert_eq!(assembled.kind, corpus::DocTypeKey::WorkItems);
        assert_eq!(assembled.slug.as_deref(), Some("ship-the-thing"));
        assert_eq!(assembled.work_item_id.as_deref(), Some("0042"));
        assert_eq!(assembled.linkage.len(), 1);
        assert_eq!(assembled.linkage[0].target_ref, "work-item:0061");
        assert_eq!(assembled.linkage[0].band, Band::Resolved);
        Ok(())
    }

    #[test]
    fn a_path_outside_every_configured_directory_does_not_assemble(
    ) -> Result<(), TestError> {
        let scanner = RegexScanner::compile("^([0-9]+)-")?;
        let assembled = assemble(
            Path::new("meta/notes/2026-01-01-x.md"),
            b"---\n---\n",
            &doc_paths(),
            &WorkItemIdScheme::numeric(),
            &scanner,
        );
        assert!(assembled.is_none());
        Ok(())
    }
}

//! The per-entry table describing everything a tree should contain.
//!
//! The table ships **inside** the archive, as its first member, and its digest
//! is carried in the signed attestation. Both halves do different jobs. Inside
//! and first, because a `tar.gz` is a stream: a table sorting last would force
//! either buffering every member's digest or a second inflate. Digested in the
//! attestation, because the archive is discarded once verified — after which
//! the table would otherwise be an ordinary file inside a tree the owning user
//! can rewrite, with no digest recorded anywhere, making every verification
//! check against an oracle any local process could edit to match a substitution.
//!
//! The producer hashes the table file as it places it in the archive and the
//! consumer hashes the same bytes as they are extracted, so the two sides need
//! no agreement about the table's internal shape for that digest to hold — only
//! for reading it back.

use std::collections::BTreeMap;

use crate::launch::core::tree::TreeError;
use crate::launch::core::tree_entry::EntryKind;

/// The table's own name at the tree root.
pub const TABLE_NAME: &str = ".files";

/// The table's shape version, gated in the same "unknown additive fields
/// ignored, higher version refused" discipline the manifest documents.
pub const TABLE_FORMAT_VERSION: u32 = 1;

const FIELD: char = '\t';
const NO_DIGEST: &str = "-";

/// What the table says one entry should be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub kind: EntryKind,
    /// The mode as extracted, before sealing. The sealed mode is a
    /// deterministic function of this, so verification computes what it should
    /// find rather than recording it twice.
    pub mode: u32,
    pub size: u64,
    /// Lowercase hex, and `None` for anything that is not a regular file.
    pub sha256: Option<String>,
    pub link_target: Option<String>,
}

/// Every entry a materialised tree should contain, keyed by path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTable {
    rows: BTreeMap<String, TableRow>,
}

impl FileTable {
    /// Read a table as it was written into the archive.
    ///
    /// # Errors
    ///
    /// [`TreeError::Extraction`] for a malformed table, or
    /// [`TreeError::LayoutUnsupported`] for a version this launcher does not
    /// read.
    pub fn parse(bytes: &[u8]) -> Result<Self, TreeError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| malformed("the table is not UTF-8"))?;
        let mut lines = text.lines();
        let version = lines
            .next()
            .and_then(|line| line.strip_prefix("version "))
            .ok_or_else(|| malformed("the table has no version line"))?
            .trim()
            .parse::<u32>()
            .map_err(|_| malformed("the table's version is not a number"))?;
        if version > TABLE_FORMAT_VERSION {
            return Err(TreeError::LayoutUnsupported {
                found: version,
                supported: TABLE_FORMAT_VERSION,
            });
        }

        let mut rows = BTreeMap::new();
        for line in lines.filter(|line| !line.trim().is_empty()) {
            let (path, row) = parse_row(line)?;
            if rows.insert(path.clone(), row).is_some() {
                return Err(malformed(&format!(
                    "the table describes '{path}' twice"
                )));
            }
        }
        Ok(Self { rows })
    }

    #[must_use]
    pub fn row(&self, path: &str) -> Option<&TableRow> {
        self.rows.get(path)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &TableRow)> {
        self.rows.iter()
    }
}

fn parse_row(line: &str) -> Result<(String, TableRow), TreeError> {
    let mut fields = line.split(FIELD);
    let kind = fields.next().unwrap_or_default();
    let mode = fields.next().unwrap_or_default();
    let size = fields.next().unwrap_or_default();
    let digest = fields.next().unwrap_or_default();
    let path = fields
        .next()
        .filter(|path| !path.is_empty())
        .ok_or_else(|| malformed("a table row has no path"))?;
    let link_target = fields.next();
    if fields.next().is_some() {
        return Err(malformed("a table row has trailing fields"));
    }

    let kind = match kind {
        "f" => EntryKind::File,
        "d" => EntryKind::Directory,
        "l" => EntryKind::Symlink,
        other => {
            return Err(malformed(&format!("unknown entry kind '{other}'")))
        }
    };
    let mode = u32::from_str_radix(mode, 8)
        .map_err(|_| malformed("a table row's mode is not octal"))?;
    let size = size
        .parse::<u64>()
        .map_err(|_| malformed("a table row's size is not a number"))?;
    let sha256 = match (kind, digest) {
        (EntryKind::File, digest)
            if super::layout::is_wellformed_digest(digest) =>
        {
            Some(digest.to_owned())
        }
        (EntryKind::File, _) => {
            return Err(malformed("a file row carries no usable digest"))
        }
        (_, NO_DIGEST) => None,
        _ => return Err(malformed("only a file row may carry a digest")),
    };
    let link_target = match (kind, link_target) {
        (EntryKind::Symlink, Some(target)) if !target.is_empty() => {
            Some(target.to_owned())
        }
        (EntryKind::Symlink, _) => {
            return Err(malformed("a symlink row names no target"))
        }
        (_, None) => None,
        _ => return Err(malformed("only a symlink row may name a target")),
    };

    Ok((
        path.to_owned(),
        TableRow {
            kind,
            mode,
            size,
            sha256,
            link_target,
        },
    ))
}

fn malformed(detail: &str) -> TreeError {
    TreeError::Extraction {
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::launch::core::tree::TreeError;
    use crate::launch::core::tree_entry::EntryKind;

    use super::{FileTable, TABLE_FORMAT_VERSION};

    const DIGEST: &str =
        "abc0000000000000000000000000000000000000000000000000000000000123";

    fn table() -> String {
        format!(
            "version 1\n\
             d\t755\t0\t-\tlib\n\
             f\t644\t120\t{DIGEST}\tlib/icudtl.dat\n\
             f\t755\t9000\t{DIGEST}\tchrome-headless-shell\n\
             l\t777\t0\t-\tlib/current\t../lib\n"
        )
    }

    #[test]
    fn every_entry_shape_round_trips_through_a_row() {
        let parsed = FileTable::parse(table().as_bytes()).expect("parses");
        assert_eq!(parsed.len(), 4);

        let directory = parsed.row("lib").expect("the directory row");
        assert_eq!(directory.kind, EntryKind::Directory);
        assert_eq!(directory.mode, 0o755);
        assert!(directory.sha256.is_none());

        let file = parsed.row("lib/icudtl.dat").expect("the file row");
        assert_eq!(file.kind, EntryKind::File);
        assert_eq!(file.size, 120);
        assert_eq!(file.sha256.as_deref(), Some(DIGEST));

        let link = parsed.row("lib/current").expect("the symlink row");
        assert_eq!(link.kind, EntryKind::Symlink);
        assert_eq!(link.link_target.as_deref(), Some("../lib"));
    }

    #[test]
    fn a_higher_table_version_is_refused_rather_than_read() {
        let ahead = format!("version {}\n", TABLE_FORMAT_VERSION + 1);
        assert!(matches!(
            FileTable::parse(ahead.as_bytes()),
            Err(TreeError::LayoutUnsupported { .. })
        ));
    }

    #[test]
    fn a_malformed_table_is_refused_rather_than_partially_read() {
        let cases = [
            "",
            "no version line\n",
            "version x\n",
            // A file row with no digest is exactly the shape that would make a
            // later content check vacuous.
            "version 1\nf\t644\t1\t-\tlib/thing\n",
            "version 1\nf\t644\t1\tnothex\tlib/thing\n",
            "version 1\nq\t644\t1\t-\tlib/thing\n",
            "version 1\nf\t99\t1\t-\tlib/thing\n",
            "version 1\nf\t644\tbig\t-\tlib/thing\n",
            "version 1\nl\t777\t0\t-\tlib/link\n",
            &format!("version 1\nd\t755\t0\t{DIGEST}\tlib\n"),
            "version 1\nf\t644\t1\t-\t\n",
            &format!("version 1\nf\t644\t1\t{DIGEST}\tlib/a\tspurious\n"),
        ];
        for case in cases {
            assert!(
                FileTable::parse(case.as_bytes()).is_err(),
                "{case:?} was parsed rather than refused"
            );
        }
    }

    #[test]
    fn a_table_describing_one_path_twice_is_refused() {
        let duplicated = format!(
            "version 1\n\
             f\t644\t1\t{DIGEST}\tlib/thing\n\
             f\t644\t2\t{DIGEST}\tlib/thing\n"
        );
        assert!(FileTable::parse(duplicated.as_bytes()).is_err());
    }
}

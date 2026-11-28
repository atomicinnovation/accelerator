use std::path::Path;
use std::path::PathBuf;

use crate::doc_type::DocTypeKey;

/// Infers a document's type from its path, choosing the type whose configured
/// directory is the longest segment-anchored match. An exact-length tie keeps
/// the first entry in `table`.
#[must_use]
pub fn infer(
    path: &Path,
    table: &[(DocTypeKey, PathBuf)],
) -> Option<DocTypeKey> {
    let path = path.to_str()?;
    let mut best: Option<(usize, DocTypeKey)> = None;
    for (kind, dir) in table {
        let Some(dir) = dir.to_str() else {
            continue;
        };
        if dir.is_empty() || !segment_match(path, dir) {
            continue;
        }
        if best.is_none_or(|(best_len, _)| dir.len() > best_len) {
            best = Some((dir.len(), *kind));
        }
    }
    best.map(|(_, kind)| kind)
}

fn segment_match(path: &str, dir: &str) -> bool {
    let mut prefix = String::with_capacity(dir.len() + 1);
    prefix.push_str(dir);
    prefix.push('/');
    if path.starts_with(&prefix) {
        return true;
    }
    let mut embedded = String::with_capacity(dir.len() + 2);
    embedded.push('/');
    embedded.push_str(dir);
    embedded.push('/');
    path.contains(&embedded)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::infer;
    use crate::doc_type::DocTypeKey;

    fn table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![
            (DocTypeKey::PLANS, PathBuf::from("meta/plans")),
            (
                DocTypeKey::PLAN_REVIEWS,
                PathBuf::from("meta/reviews/plans"),
            ),
            (DocTypeKey::WORK_ITEMS, PathBuf::from("meta/work")),
        ]
    }

    #[test]
    fn infer_matches_a_segment_anchored_directory() {
        assert_eq!(
            infer(Path::new("meta/work/0042-foo.md"), &table()),
            Some(DocTypeKey::WORK_ITEMS)
        );
    }

    #[test]
    fn infer_prefers_the_longest_configured_directory() {
        let path = Path::new("meta/reviews/plans/2026-01-01-foo-review-1.md");
        assert_eq!(infer(path, &table()), Some(DocTypeKey::PLAN_REVIEWS));
    }

    #[test]
    fn infer_returns_none_when_no_directory_matches() {
        assert_eq!(infer(Path::new("meta/notes/foo.md"), &table()), None);
    }

    #[test]
    fn infer_requires_a_whole_segment_not_a_prefix() {
        assert_eq!(infer(Path::new("meta/plans-archive/x.md"), &table()), None);
    }

    #[test]
    fn infer_breaks_an_exact_length_tie_by_keeping_the_first_entry() {
        let tied = vec![
            (DocTypeKey::PLAN_REVIEWS, PathBuf::from("meta/reviews")),
            (DocTypeKey::PR_REVIEWS, PathBuf::from("meta/reviews")),
        ];
        assert_eq!(
            infer(Path::new("meta/reviews/2026-01-01-x-review-1.md"), &tied),
            Some(DocTypeKey::PLAN_REVIEWS)
        );
    }

    #[test]
    fn infer_matches_a_directory_as_an_interior_path_segment() {
        let path = Path::new("/tmp/checkout/meta/work/0042-foo.md");
        assert_eq!(infer(path, &table()), Some(DocTypeKey::WORK_ITEMS));
    }
}

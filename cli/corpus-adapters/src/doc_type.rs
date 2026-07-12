//! Sourcing the doc-type `type -> dir` table from configured paths and handing
//! it to the pure `corpus::doc_type::infer` matcher.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::Path;
use std::path::PathBuf;

use corpus::DocTypeKey;

/// Builds the `(DocTypeKey, dir)` table from a config-keyed path map, keeping
/// only the types whose configured directory is present.
#[must_use]
pub fn table_from_paths<S: BuildHasher>(
    doc_paths: &HashMap<String, PathBuf, S>,
) -> Vec<(DocTypeKey, PathBuf)> {
    DocTypeKey::all()
        .into_iter()
        .filter_map(|kind| {
            let key = kind.config_path_key()?;
            let dir = doc_paths.get(key)?.clone();
            Some((kind, dir))
        })
        .collect()
}

/// Infers a document's type from its path against the configured directories.
#[must_use]
pub fn infer<S: BuildHasher>(
    path: &Path,
    doc_paths: &HashMap<String, PathBuf, S>,
) -> Option<DocTypeKey> {
    corpus::doc_type::infer(path, &table_from_paths(doc_paths))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use corpus::DocTypeKey;

    use super::infer;

    fn paths() -> HashMap<String, PathBuf> {
        let mut paths = HashMap::new();
        paths.insert("plans".to_owned(), PathBuf::from("meta/plans"));
        paths.insert(
            "review_plans".to_owned(),
            PathBuf::from("meta/reviews/plans"),
        );
        paths.insert("work".to_owned(), PathBuf::from("meta/work"));
        paths
    }

    #[test]
    fn infers_from_the_configured_directory() {
        assert_eq!(
            infer(Path::new("meta/work/0042-foo.md"), &paths()),
            Some(DocTypeKey::WorkItems)
        );
    }

    #[test]
    fn prefers_the_longest_matching_directory() {
        let path = Path::new("meta/reviews/plans/2026-01-01-x-review-1.md");
        assert_eq!(infer(path, &paths()), Some(DocTypeKey::PlanReviews));
    }

    #[test]
    fn returns_none_for_an_unconfigured_path() {
        assert_eq!(infer(Path::new("meta/notes/x.md"), &paths()), None);
    }
}

//! The doc-type-scanning `CorpusIndex` adapter migration 0007 needs: the
//! set of `(type, id)` pairs a resolved-band body reference is checked
//! against before it applies mechanically.
//!
//! Both sides of the comparison are derived through
//! `corpus::linkage::resolve_path_target` — the same function that
//! produces a reference's own target — rather than
//! `corpus::work_item_id`/`corpus::slug` (display conventions that strip
//! or project-prefix an id differently), so the comparison is correct by
//! construction rather than by two independently-written conventions
//! happening to agree.

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use corpus::doc_type::DocTypeKey;
use corpus::scan::CorpusWalker;
use corpus_adapters::fs::RealFs;
use migrate::ports::CorpusIndex;

pub struct FileCorpusIndex {
    targets: HashSet<(String, String)>,
}

impl FileCorpusIndex {
    #[must_use]
    pub fn build(table: &[(DocTypeKey, PathBuf)]) -> Self {
        let roots: Vec<PathBuf> =
            table.iter().map(|(_, dir)| dir.clone()).collect();
        let files = RealFs.walk_markdown(&roots).unwrap_or_default();
        let mut targets = HashSet::new();
        for file in files {
            if let Some((kind, id)) =
                corpus::linkage::resolve_path_target(&path_str(&file), table)
            {
                targets.insert((kind.to_owned(), id));
            }
        }
        Self { targets }
    }
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl CorpusIndex for FileCorpusIndex {
    fn target_exists(&self, target_type: &str, target_id: &str) -> bool {
        self.targets
            .contains(&(target_type.to_owned(), target_id.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::FileCorpusIndex;
    use corpus::doc_type::DocTypeKey;
    use migrate::ports::CorpusIndex as _;
    use tempfile::TempDir;

    #[test]
    fn a_work_item_resolves_by_its_filename_derived_id(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let work = dir.path().join("meta/work");
        std::fs::create_dir_all(&work)?;
        std::fs::write(work.join("0042-foo.md"), "")?;
        let table = vec![(DocTypeKey::WorkItems, work)];

        let index = FileCorpusIndex::build(&table);
        assert!(index.target_exists("work-item", "0042"));
        assert!(!index.target_exists("work-item", "0099"));
        Ok(())
    }

    #[test]
    fn a_plan_resolves_by_its_full_stem_not_a_stripped_slug(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let plans = dir.path().join("meta/plans");
        std::fs::create_dir_all(&plans)?;
        std::fs::write(
            plans.join("2026-05-13-0055-sidebar-activity-feed.md"),
            "",
        )?;
        let table = vec![(DocTypeKey::Plans, plans)];

        let index = FileCorpusIndex::build(&table);
        assert!(index
            .target_exists("plan", "2026-05-13-0055-sidebar-activity-feed"));
        assert!(!index.target_exists("plan", "sidebar-activity-feed"));
        Ok(())
    }
}

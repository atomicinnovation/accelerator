//! The shipped `templates/` skeletons must carry zero template-shape
//! violations against the embedded schema TSV. A repository-integrity guard on
//! this repo's own artefacts, driven directly through the library rather than a
//! command-line surface: template resolution overlays user overrides, and the
//! plugin's default skeletons only ever exist at the repo root.

mod common;

use common::repo_root;
use common::TestError;

use corpus_adapters::frontmatter_validation::validate_templates;
use corpus_adapters::RealFs;

#[test]
fn the_shipped_templates_tree_is_clean() -> Result<(), TestError> {
    let violations = validate_templates(&repo_root()?, &RealFs)?;
    assert!(
        violations.is_empty(),
        "the shipped templates/ tree must carry zero template-shape \
         violations: {}",
        violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}

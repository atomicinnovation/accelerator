use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use corpus::frontmatter_validation::pipeline::build_index;
use corpus::frontmatter_validation::pipeline::validate_path;
use corpus::frontmatter_validation::pipeline::validate_text;
use corpus::frontmatter_validation::Violation;
use corpus::scan::CorpusWalker;
use corpus::scan::FileReader;
use corpus::DocTypeKey;
use corpus_adapters::YamlFrontmatter;

type TestError = Box<dyn std::error::Error>;

const TAGGED: &str = "---\nkey: !custom value\n---\nbody\n";
const SEQUENCE_ROOT: &str = "---\n- a\n- b\n---\nbody\n";

#[derive(Default)]
struct InMemoryCorpus {
    files: HashMap<PathBuf, String>,
}

impl InMemoryCorpus {
    fn with_file(mut self, path: &str, content: &str) -> Self {
        self.files.insert(PathBuf::from(path), content.to_owned());
        self
    }
}

impl FileReader for InMemoryCorpus {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        Ok(self.files.get(path).cloned())
    }
}

impl CorpusWalker for InMemoryCorpus {
    fn walk_markdown(
        &self,
        roots: &[PathBuf],
    ) -> Result<Vec<PathBuf>, kernel::Error> {
        Ok(self
            .files
            .keys()
            .filter(|path| roots.iter().any(|root| path.starts_with(root)))
            .cloned()
            .collect())
    }
}

fn work_item(id: &str) -> String {
    format!(
        "---\ntype: \"work-item\"\nid: \"{id}\"\ntitle: \"t\"\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: \"a\"\ntags: []\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: \"a\"\n\
         schema_version: 1\nstatus: \"draft\"\nkind: \"task\"\n\
         priority: \"normal\"\n---\nbody\n"
    )
}

#[test]
fn a_tagged_or_non_mapping_root_fails_as_no_fence() {
    for content in [TAGGED, SEQUENCE_ROOT] {
        assert_eq!(
            validate_text(content, &YamlFrontmatter),
            vec![Violation::NoFence],
            "{content:?}"
        );
    }
}

#[test]
fn a_tagged_file_read_from_the_corpus_fails_as_no_fence(
) -> Result<(), TestError> {
    let corpus =
        InMemoryCorpus::default().with_file("meta/work/0001.md", TAGGED);
    assert_eq!(
        validate_path(
            Path::new("meta/work/0001.md"),
            &corpus,
            &YamlFrontmatter
        )?,
        vec![Violation::NoFence]
    );
    Ok(())
}

#[test]
fn a_well_formed_file_passes() {
    assert!(validate_text(&work_item("0001"), &YamlFrontmatter).is_empty());
}

#[test]
fn the_index_excludes_a_tagged_file() -> Result<(), TestError> {
    let corpus = InMemoryCorpus::default()
        .with_file("meta/work/0001.md", TAGGED)
        .with_file("meta/work/0002.md", &work_item("0002"));
    let table = [(DocTypeKey::WorkItems, PathBuf::from("meta/work"))];

    let index = build_index(&table, &corpus, &corpus, &YamlFrontmatter)?;

    assert!(!index.contains("work-item:0001"));
    assert!(index.contains("work-item:0002"));
    Ok(())
}

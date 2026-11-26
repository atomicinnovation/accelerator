//! Production code reads a doc type's declared facts and rules and never
//! names a built-in doc type, outside the declarations themselves and the
//! code that owns one doc type's domain.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

use cli_test_support::source_scan::{
    built_in_doc_type_references, rust_sources_under,
};

const BUILT_IN_DECLARATIONS: &str = "corpus/src/doc_type/built_in.rs";

const OWNERS_OF_ONE_DOC_TYPE: [&str; 2] =
    ["research-cli/src/topic_set.rs", "corpus-cli/src/main.rs"];

const FROZEN_MIGRATIONS: &str = "migrate/src/migrations/";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the cli workspace")
        .to_path_buf()
}

fn relative(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .expect("a source under the workspace")
        .to_string_lossy()
        .replace('\\', "/")
}

fn may_name_built_in_doc_types(relative_path: &str) -> bool {
    relative_path == BUILT_IN_DECLARATIONS
        || OWNERS_OF_ONE_DOC_TYPE.contains(&relative_path)
        || relative_path.starts_with(FROZEN_MIGRATIONS)
}

#[test]
fn no_production_code_names_a_built_in_doc_type() {
    let sources = rust_sources_under(&workspace_root());
    assert!(
        sources.len() >= 100,
        "scanned only {} sources under {}",
        sources.len(),
        workspace_root().display()
    );
    let mut offences = Vec::new();
    for path in &sources {
        let relative_path = relative(path);
        if may_name_built_in_doc_types(&relative_path) {
            continue;
        }
        let source = std::fs::read_to_string(path).expect("a source");
        offences.extend(
            built_in_doc_type_references(&source)
                .into_iter()
                .map(|offence| format!("{relative_path}:{offence}")),
        );
    }
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn each_built_in_doc_type_declares_every_field_itself() {
    let declarations =
        std::fs::read_to_string(workspace_root().join(BUILT_IN_DECLARATIONS))
            .expect("the built-in declarations");
    assert!(!declarations.contains("FLAT_DEFAULTS"));
}

#[test]
fn the_exempt_sources_exist() {
    let root = workspace_root();
    for exempt in [BUILT_IN_DECLARATIONS, FROZEN_MIGRATIONS]
        .into_iter()
        .chain(OWNERS_OF_ONE_DOC_TYPE)
    {
        assert!(root.join(exempt).exists(), "{exempt}");
    }
}

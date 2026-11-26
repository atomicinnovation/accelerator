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

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("a source under the workspace")
        .to_string_lossy()
        .replace('\\', "/")
}

fn may_name_built_in_doc_types(relative_path: &str) -> bool {
    relative_path == BUILT_IN_DECLARATIONS
        || OWNERS_OF_ONE_DOC_TYPE.contains(&relative_path)
        || relative_path.starts_with(FROZEN_MIGRATIONS)
        || is_declared_by_a_declarations_crate(relative_path)
}

fn is_declared_by_a_declarations_crate(relative_path: &str) -> bool {
    relative_path
        .split_once("/src/")
        .is_some_and(|(crate_dir, _)| {
            crate_dir == "declarations" || crate_dir.ends_with("-declarations")
        })
}

fn offences_under(root: &Path, sources: &[PathBuf]) -> Vec<String> {
    let mut offences = Vec::new();
    for path in sources {
        let relative_path = relative(root, path);
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
    offences
}

#[test]
fn no_production_code_names_a_built_in_doc_type() {
    let root = workspace_root();
    let sources = rust_sources_under(&root);
    assert!(
        sources.len() >= 100,
        "scanned only {} sources under {}",
        sources.len(),
        root.display()
    );
    let offences = offences_under(&root, &sources);
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn only_the_declarations_crates_and_their_assembly_may_name_one() {
    let root = tempfile::tempdir().expect("a scratch workspace");
    let naming = "pub const KIND: DocTypeKey = DocTypeKey::PLANS;\n";
    for crate_dir in ["declarations", "research-declarations", "research"] {
        let src = root.path().join(crate_dir).join("src");
        std::fs::create_dir_all(&src).expect("a crate source directory");
        std::fs::write(src.join("lib.rs"), naming).expect("a seeded source");
    }
    let sources = rust_sources_under(root.path());

    assert_eq!(
        offences_under(root.path(), &sources),
        vec![format!("research/src/lib.rs:1: {}", naming.trim())]
    );
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

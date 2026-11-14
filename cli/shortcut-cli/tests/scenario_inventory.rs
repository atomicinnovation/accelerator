//! Every scenario under `tests/fixtures/scenarios/` is named by a test, and
//! the count is pinned, so a fixture cannot be added or dropped unnoticed.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

const EXPECTED_SCENARIO_COUNT: usize = 21;

fn test_sources(dir: &Path, corpus: &mut String) {
    for entry in std::fs::read_dir(dir).expect("read tests dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            test_sources(&path, corpus);
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && path
                .file_name()
                .is_some_and(|name| name != "scenario_inventory.rs")
        {
            corpus.push_str(&std::fs::read_to_string(&path).expect("read rs"));
        }
    }
}

#[test]
fn every_scenario_fixture_is_named_by_a_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = String::new();
    test_sources(&root.join("tests"), &mut sources);

    let mut stems: Vec<String> =
        std::fs::read_dir(root.join("tests/fixtures/scenarios"))
            .expect("read scenarios dir")
            .map(|entry| entry.expect("entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_owned)
            })
            .collect();
    stems.sort();

    assert_eq!(stems.len(), EXPECTED_SCENARIO_COUNT, "{stems:?}");
    for stem in &stems {
        assert!(
            sources.contains(&format!("\"{stem}\"")),
            "scenario {stem:?} is named by no test"
        );
    }
}

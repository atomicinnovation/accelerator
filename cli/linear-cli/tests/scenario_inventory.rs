//! Every scenario fixture under `tests/fixtures/scenarios/` must be referenced
//! by at least one test (Decision 15), so "ported" provably means "consumed":
//! a scenario carried over from the retiring bash cluster but driven by nothing
//! would re-create the dead surface the migration bar exists to prevent.
//!
//! "Referenced" is asserted structurally — the fixture stem appears in a test
//! source that loads and asserts against it. The count is pinned so a new
//! fixture cannot be added without a test, and a deletion cannot pass silently.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

const EXPECTED_SCENARIO_COUNT: usize = 15;

fn read_all_test_sources() -> String {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut corpus = String::new();
    collect(&tests, &mut corpus);
    corpus
}

fn collect(dir: &Path, corpus: &mut String) {
    for entry in std::fs::read_dir(dir).expect("read tests dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect(&path, corpus);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            corpus.push_str(&std::fs::read_to_string(&path).expect("read rs"));
        }
    }
}

#[test]
fn every_scenario_fixture_is_referenced_by_a_test() {
    let scenarios =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scenarios");
    let sources = read_all_test_sources();

    let mut stems: Vec<String> = std::fs::read_dir(&scenarios)
        .expect("read scenarios dir")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .expect("utf-8 stem")
                .to_owned()
        })
        .collect();
    stems.sort();

    assert_eq!(
        stems.len(),
        EXPECTED_SCENARIO_COUNT,
        "scenario count changed; update the pin and confirm each is consumed: \
         {stems:?}"
    );
    for stem in &stems {
        assert!(
            sources.contains(stem.as_str()),
            "scenario {stem:?} is not referenced by any test — port it into a \
             test or remove it (Decision 15)"
        );
    }
}

//! Keeps the committed bash-parity baseline describing the corpus it was
//! taken against, so it cannot drift silently before 0212 reads it.

#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

type TestError = Box<dyn std::error::Error>;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn baseline() -> Result<String, TestError> {
    Ok(std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/bash-parity-baseline.txt"),
    )?)
}

fn rows(raw: &str, kind: &str) -> Vec<Vec<String>> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next()? == kind)
                .then(|| fields.map(str::to_owned).collect::<Vec<_>>())
        })
        .collect()
}

fn recorded_cases(raw: &str) -> Vec<(String, BTreeSet<String>)> {
    let mut directories: Vec<(String, BTreeSet<String>)> = Vec::new();
    for row in rows(raw, "case") {
        let directory = row[0].clone();
        let case = row[1].clone();
        match directories.iter_mut().find(|(name, _)| *name == directory) {
            Some((_, cases)) => {
                cases.insert(case);
            }
            None => {
                directories.push((directory, BTreeSet::from([case])));
            }
        }
    }
    directories
}

fn present_cases(
    root: &Path,
    directory: &str,
) -> Result<BTreeSet<String>, TestError> {
    let path = root
        .join("skills/work/scripts/test-fixtures")
        .join(directory);
    let mut cases = BTreeSet::new();
    for entry in std::fs::read_dir(&path)? {
        cases.insert(entry?.file_name().to_string_lossy().into_owned());
    }
    Ok(cases)
}

#[test]
fn the_baseline_still_describes_the_fixture_corpus() -> Result<(), TestError> {
    let raw = baseline()?;
    let root = repo_root()?;
    let recorded = recorded_cases(&raw);
    assert!(
        !recorded.is_empty(),
        "the baseline records no case at all — it would pass vacuously"
    );

    for (directory, cases) in recorded {
        let present = present_cases(&root, &directory)?;
        let appeared: Vec<_> = present.difference(&cases).collect();
        let vanished: Vec<_> = cases.difference(&present).collect();
        assert!(
            appeared.is_empty() && vanished.is_empty(),
            "{directory} has drifted from the committed baseline — \
             appeared: {appeared:?}, vanished: {vanished:?}. The baseline \
             records {} cases here and the tree carries {}; update \
             cli/work-adapters/tests/fixtures/bash-parity-baseline.txt so \
             0212 attributes its conversion against the real corpus.",
            cases.len(),
            present.len()
        );
    }
    Ok(())
}

#[test]
fn every_recorded_parity_test_still_exists_with_its_recorded_count(
) -> Result<(), TestError> {
    let raw = baseline()?;
    let root = repo_root()?;
    let recorded = rows(&raw, "test");
    assert_eq!(
        recorded.len(),
        11,
        "the baseline names the eleven parity tests 0212 converts"
    );

    for row in recorded {
        let path = root.join(&row[0]);
        let expected: usize = row[1].parse()?;
        let source = std::fs::read_to_string(&path)
            .map_err(|error| format!("{}: {error}", row[0]))?;
        let present = source.matches("#[test]").count();
        assert_eq!(
            present, expected,
            "{} carries {present} tests, baseline records {expected} — \
             update the baseline so 0212's attribution is honest",
            row[0]
        );
    }
    Ok(())
}

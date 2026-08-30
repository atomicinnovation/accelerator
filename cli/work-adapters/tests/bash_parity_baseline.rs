//! Keeps the committed parity baseline describing the relocated corpus it was
//! taken against, so it cannot drift silently. Guards three things: the set of
//! case directories per corpus, the `#[test]` count of each converted parity
//! test, and the byte-identity of every committed golden.

#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use sha2::Digest as _;
use sha2::Sha256;

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

/// Where each corpus was relocated to, repo-relative. The corpus is split by
/// its actual consumer rather than a single shared tree.
fn corpus_home(directory: &str) -> Result<&'static str, TestError> {
    Ok(match directory {
        "work-item-normalise" => "cli/work/tests/fixtures",
        "work-item-project-remote" => "cli/remote-projection/tests/fixtures",
        "work-item-sync-baseline" => "cli/work-adapters/tests/fixtures",
        other => {
            return Err(format!(
                "no relocated home recorded for corpus {other}"
            )
            .into())
        }
    })
}

/// Every corpus directory a home may hold, so a relocated corpus with no
/// recorded row is caught rather than silently unguarded.
const RELOCATED_CORPORA: &[&str] = &[
    "work-item-normalise",
    "work-item-project-remote",
    "work-item-sync-baseline",
];

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
    let path = root.join(corpus_home(directory)?).join(directory);
    let mut cases = BTreeSet::new();
    for entry in std::fs::read_dir(&path)? {
        cases.insert(entry?.file_name().to_string_lossy().into_owned());
    }
    Ok(cases)
}

fn sha256_of(path: &Path) -> Result<String, TestError> {
    let mut hasher = Sha256::new();
    hasher.update(std::fs::read(path)?);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_golden(name: &str) -> bool {
    name == "expected.txt"
        || name == "expected.json"
        || name.ends_with(".golden")
        || name == "work-item-sync-classify.json"
}

fn present_goldens(
    root: &Path,
    home: &str,
) -> Result<BTreeSet<String>, TestError> {
    let mut found = BTreeSet::new();
    let mut stack = vec![root.join(home)];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(is_golden)
            {
                let relative =
                    path.strip_prefix(root)?.to_string_lossy().into_owned();
                found.insert(relative);
            }
        }
    }
    Ok(found)
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
             cli/work-adapters/tests/fixtures/bash-parity-baseline.txt.",
            cases.len(),
            present.len()
        );
    }
    Ok(())
}

#[test]
fn every_relocated_corpus_is_covered_by_a_recorded_row() -> Result<(), TestError>
{
    let raw = baseline()?;
    let root = repo_root()?;
    let recorded: BTreeSet<String> = recorded_cases(&raw)
        .into_iter()
        .map(|(directory, _)| directory)
        .collect();

    for corpus in RELOCATED_CORPORA {
        let home = root.join(corpus_home(corpus)?).join(corpus);
        assert!(
            home.is_dir(),
            "{corpus} is not present at its recorded home {}",
            home.display()
        );
        assert!(
            recorded.contains(*corpus),
            "{corpus} is present on disk but has no case row in the baseline \
             — its destination would drift unguarded"
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
        8,
        "the baseline names the eight pure-Rust parity tests left behind"
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
             update the baseline so its attribution is honest",
            row[0]
        );
    }
    Ok(())
}

#[test]
fn every_committed_golden_matches_its_recorded_hash() -> Result<(), TestError> {
    let raw = baseline()?;
    let root = repo_root()?;
    let recorded = rows(&raw, "hash");
    assert!(
        !recorded.is_empty(),
        "the baseline records no golden hash — it would pass vacuously"
    );

    let mut recorded_paths = BTreeSet::new();
    for row in &recorded {
        let [relative, expected] = &row[..] else {
            return Err(format!("malformed hash row: {row:?}").into());
        };
        recorded_paths.insert(relative.clone());
        let actual = sha256_of(&root.join(relative))?;
        assert_eq!(
            &actual, expected,
            "{relative} has changed — the goldens are the frozen bash oracle \
             and must never be regenerated from Rust output"
        );
    }

    let mut present = BTreeSet::new();
    for home in [
        "cli/work/tests/fixtures",
        "cli/work-adapters/tests/fixtures",
        "cli/remote-projection/tests/fixtures",
    ] {
        present.extend(present_goldens(&root, home)?);
    }
    let unguarded: Vec<_> = present.difference(&recorded_paths).collect();
    assert!(
        unguarded.is_empty(),
        "these committed goldens have no hash row and would be unguarded: \
         {unguarded:?}"
    );

    Ok(())
}

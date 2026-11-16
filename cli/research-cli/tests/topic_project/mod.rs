//! A throwaway project holding a copy of a fixture topic-research set, for
//! running `accelerator-research topic` against.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

pub type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-research");
pub const TOPICS: &str = "meta/research/topics";

pub struct Project {
    _dir: tempfile::TempDir,
    pub root: PathBuf,
}

impl Project {
    pub fn with_fixture_set(fixture: &str) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix("research-topic-")
            .tempdir()?;
        let root = dir.path().canonicalize()?;
        fs::create_dir_all(root.join(".git"))?;
        let project = Self { _dir: dir, root };
        copy_tree(&fixtures().join(fixture), &project.set(fixture))?;
        Ok(project)
    }

    pub fn set(&self, slug: &str) -> PathBuf {
        self.root.join(TOPICS).join(slug)
    }

    pub fn run(&self, verb: &str, args: &[&str]) -> Result<Output, TestError> {
        Ok(Command::new(BIN)
            .current_dir(&self.root)
            .args(["topic", verb])
            .args(args)
            .output()?)
    }
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../corpus-cli/tests/fixtures")
}

pub fn copy_tree(from: &Path, to: &Path) -> Result<(), TestError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

pub fn tree(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, TestError> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                files.insert(path.clone(), Vec::new());
                pending.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path)?);
            }
        }
    }
    Ok(files)
}

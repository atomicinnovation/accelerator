//! The cache write path: atomic writes, lock-before-write for the catalogue,
//! and idempotent scaffold upkeep — driven against a fake filesystem — plus the
//! real lock's shared `.lock` directory and owner sentinel.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use corpus_adapters::LockOptions;
use linear_client::cache::{
    CacheError, Filesystem, LinearCache, SystemFilesystem,
};
use linear_client::catalogue::CatalogueTeam;
use linear_client::filter::TeamResolver;
use serde_json::json;
use tempfile::TempDir;

#[derive(Default)]
struct FakeFs {
    files: RefCell<BTreeMap<PathBuf, String>>,
    fail_writes: Cell<bool>,
    lock_held: Cell<bool>,
    lock_entered: Cell<bool>,
}

impl FakeFs {
    fn get(&self, path: &Path) -> Option<String> {
        self.files.borrow().get(path).cloned()
    }

    fn lines(&self, path: &Path) -> Vec<String> {
        self.get(path)
            .map(|content| content.lines().map(str::to_owned).collect())
            .unwrap_or_default()
    }
}

impl Filesystem for FakeFs {
    fn exists(&self, path: &Path) -> bool {
        self.files.borrow().contains_key(path)
    }

    fn read(&self, path: &Path) -> Option<String> {
        self.get(path)
    }

    fn write_atomic(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), CacheError> {
        if self.fail_writes.get() {
            return Err(CacheError::Io {
                path: path.display().to_string(),
                detail: "injected mid-write failure".to_owned(),
            });
        }
        self.files.borrow_mut().insert(
            path.to_path_buf(),
            String::from_utf8_lossy(bytes).into_owned(),
        );
        Ok(())
    }

    fn ensure_present(&self, path: &Path) -> Result<(), CacheError> {
        self.files
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_default();
        Ok(())
    }

    fn append_line(&self, path: &Path, line: &str) -> Result<(), CacheError> {
        let mut files = self.files.borrow_mut();
        let entry = files.entry(path.to_path_buf()).or_default();
        entry.push_str(line);
        entry.push('\n');
        Ok(())
    }

    fn with_lock(
        &self,
        lockdir: &Path,
        body: &mut dyn FnMut() -> Result<(), CacheError>,
    ) -> Result<(), CacheError> {
        if self.lock_held.get() {
            return Err(CacheError::LockContended {
                path: lockdir.display().to_string(),
            });
        }
        self.lock_entered.set(true);
        body()
    }
}

fn cache_root() -> PathBuf {
    PathBuf::from("/state/linear")
}

#[test]
fn write_viewer_writes_the_shape_and_the_scaffold() {
    let fs = FakeFs::default();
    let cache = LinearCache::new(&fs, cache_root());

    cache
        .write_viewer(&json!({ "id": "u1", "name": "Ada" }))
        .expect("the viewer is written");

    assert!(fs.get(&cache_root().join("viewer.json")).is_some());
    let rules = fs.lines(&cache_root().join(".gitignore"));
    assert_eq!(
        rules,
        vec!["viewer.json", ".refresh-meta.json", ".lock/"],
        "catalogue.json is committed, so it is not gitignored"
    );
    assert!(fs.exists(&cache_root().join(".gitkeep")));
}

#[test]
fn the_scaffold_is_idempotent_across_two_runs() {
    let fs = FakeFs::default();
    let cache = LinearCache::new(&fs, cache_root());
    let shape = json!({ "id": "u1", "name": "Ada" });

    cache.write_viewer(&shape).expect("first run");
    cache.write_viewer(&shape).expect("second run");

    let rules = fs.lines(&cache_root().join(".gitignore"));
    assert_eq!(rules.len(), 3, "no rule is duplicated on the second run");
}

#[test]
fn a_failed_write_surfaces_and_leaves_prior_content() {
    let fs = FakeFs::default();
    fs.files
        .borrow_mut()
        .insert(cache_root().join("viewer.json"), "OLD".to_owned());
    fs.fail_writes.set(true);
    let cache = LinearCache::new(&fs, cache_root());

    let error = cache
        .write_viewer(&json!({ "id": "new" }))
        .expect_err("the write fails");

    assert!(matches!(error, CacheError::Io { .. }));
    assert_eq!(
        fs.get(&cache_root().join("viewer.json")).as_deref(),
        Some("OLD")
    );
}

#[test]
fn the_catalogue_is_written_under_the_lock() {
    let fs = FakeFs::default();
    let cache = LinearCache::new(&fs, cache_root());

    cache
        .write_catalogue(&json!({ "team": {}, "workflowStates": [] }))
        .expect("the catalogue is written");

    assert!(fs.lock_entered.get());
    assert!(fs.exists(&cache_root().join("catalogue.json")));
}

#[test]
fn a_held_lock_is_contention_not_a_clobber() {
    let fs = FakeFs::default();
    fs.lock_held.set(true);
    let cache = LinearCache::new(&fs, cache_root());

    let error = cache
        .write_catalogue(&json!({ "team": {} }))
        .expect_err("contended");

    assert!(matches!(error, CacheError::LockContended { .. }));
    assert!(!fs.exists(&cache_root().join("catalogue.json")));
}

#[test]
fn the_real_lock_creates_the_shared_lock_dir_with_the_owner_sentinel() {
    let dir = TempDir::new().expect("a temp dir");
    let fs = SystemFilesystem::new(dir.path().to_path_buf());
    let lockdir = dir.path().join(".lock");

    let mut checked = false;
    fs.with_lock(&lockdir, &mut || {
        assert!(lockdir.exists(), "the shared .lock dir is created");
        let has_owner = std::fs::read_dir(&lockdir)
            .expect("the lockdir is readable")
            .flatten()
            .any(|entry| {
                entry.file_name().to_string_lossy().starts_with("owner.")
            });
        assert!(has_owner, "the shared owner.<nonce> sentinel is written");
        checked = true;
        Ok(())
    })
    .expect("the lock is acquired");

    assert!(checked);
    assert!(!lockdir.exists(), "the lock is released on completion");
}

#[test]
fn the_real_lock_times_out_rather_than_stealing_a_bash_held_lock() {
    let dir = TempDir::new().expect("a temp dir");
    let lockdir = dir.path().join(".lock");
    std::fs::create_dir(&lockdir).expect("a pre-held lock");
    std::fs::write(lockdir.join("holder.pid"), "999999\n")
        .expect("the bash sentinel is written");

    let fs = SystemFilesystem::new(dir.path().to_path_buf()).with_lock_options(
        LockOptions {
            ceiling_ms: 40,
            base_ms: 4,
            cap_ms: 8,
        },
    );
    let error = fs
        .with_lock(&lockdir, &mut || Ok(()))
        .expect_err("the held lock is not stolen");

    assert!(matches!(error, CacheError::LockContended { .. }));
    assert!(lockdir.exists(), "the pre-held lock is left intact");
}

#[test]
fn grow_catalogue_adds_a_new_team_preserving_the_base_and_states() {
    let fs = FakeFs::default();
    fs.files.borrow_mut().insert(
        cache_root().join("catalogue.json"),
        json!({
            "team": { "key": "ENG", "id": "eng-id", "name": "Engineering" },
            "workflowStates": [ { "id": "s1", "name": "Todo" } ],
        })
        .to_string(),
    );
    let cache = LinearCache::new(&fs, cache_root());

    let added = cache
        .grow_catalogue(&[(
            "OPS".to_owned(),
            "ops-id".to_owned(),
            "Operations".to_owned(),
        )])
        .expect("the catalogue grows");

    assert_eq!(added, vec!["OPS".to_owned()]);
    assert!(fs.lock_entered.get(), "growth runs under the lock");

    let raw = fs
        .get(&cache_root().join("catalogue.json"))
        .expect("the catalogue is written");
    let catalogue: serde_json::Value =
        serde_json::from_str(&raw).expect("the catalogue stays valid JSON");
    assert_eq!(
        catalogue
            .pointer("/team/key")
            .and_then(|value| value.as_str()),
        Some("ENG"),
        "the base team is preserved"
    );
    assert!(
        catalogue.get("workflowStates").is_some(),
        "the workflow states are preserved"
    );

    let resolver = CatalogueTeam::from_catalogue(&catalogue);
    assert_eq!(
        resolver.resolve("OPS").as_deref(),
        Some("ops-id"),
        "the newly-imported team resolves from the grown catalogue"
    );
    assert_eq!(
        resolver.resolve("ENG").as_deref(),
        Some("eng-id"),
        "the base team still resolves"
    );
}

#[test]
fn grow_catalogue_does_not_re_add_an_already_catalogued_team() {
    let fs = FakeFs::default();
    fs.files.borrow_mut().insert(
        cache_root().join("catalogue.json"),
        json!({ "team": { "key": "ENG", "id": "eng-id", "name": "Eng" } })
            .to_string(),
    );
    let cache = LinearCache::new(&fs, cache_root());

    let first = cache
        .grow_catalogue(&[(
            "OPS".to_owned(),
            "ops-id".to_owned(),
            "Ops".to_owned(),
        )])
        .expect("first growth");
    assert_eq!(first, vec!["OPS".to_owned()]);

    let second = cache
        .grow_catalogue(&[
            ("ENG".to_owned(), "eng-id".to_owned(), "Eng".to_owned()),
            ("OPS".to_owned(), "ops-id".to_owned(), "Ops".to_owned()),
        ])
        .expect("second growth");
    assert!(
        second.is_empty(),
        "a team already catalogued — base or grown — is not re-added"
    );
}

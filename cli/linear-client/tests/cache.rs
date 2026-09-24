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
use linear_client::catalogue::{
    CatalogueDocument, CatalogueUpdate, CataloguedLabel, Strictness, TeamEntry,
};
use serde_json::{json, Value};
use tempfile::TempDir;

#[derive(Default)]
struct FakeFs {
    files: RefCell<BTreeMap<PathBuf, String>>,
    unreadable: RefCell<Vec<PathBuf>>,
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

    fn read(&self, path: &Path) -> Result<Option<String>, CacheError> {
        if self.unreadable.borrow().iter().any(|locked| locked == path) {
            return Err(CacheError::Io {
                path: path.display().to_string(),
                detail: "injected read failure".to_owned(),
            });
        }
        Ok(self.get(path))
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

fn catalogue_path() -> PathBuf {
    cache_root().join("catalogue.json")
}

fn seed(fs: &FakeFs, catalogue: &Value) {
    fs.files
        .borrow_mut()
        .insert(catalogue_path(), catalogue.to_string());
}

fn written(fs: &FakeFs) -> Value {
    let raw = fs.get(&catalogue_path()).expect("the catalogue is written");
    serde_json::from_str(&raw).expect("the catalogue stays valid JSON")
}

fn entry(value: Value) -> TeamEntry {
    serde_json::from_value(value).expect("a well-formed team entry")
}

fn labels(value: Value) -> Vec<CataloguedLabel> {
    serde_json::from_value(value).expect("well-formed labels")
}

const fn update(entries: Vec<TeamEntry>) -> CatalogueUpdate {
    CatalogueUpdate {
        base_team: None,
        entries,
        workspace_labels: None,
    }
}

fn record(fs: &FakeFs, update: &CatalogueUpdate) -> Vec<String> {
    LinearCache::new(fs, cache_root())
        .record_team_entries(update)
        .expect("the entries are recorded")
}

fn state(id: &str, name: &str) -> Value {
    json!({
        "id": id, "name": name, "type": "started", "position": 1,
        "archivedAt": null,
    })
}

fn label(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "archivedAt": null })
}

fn member(id: &str, name: &str) -> Value {
    json!({
        "id": id, "name": name, "displayName": name,
        "email": format!("{name}@example.com"), "active": true,
    })
}

fn project(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "archivedAt": null })
}

fn complete_entry(id: &str, key: &str) -> Value {
    json!({
        "id": id, "key": key, "name": format!("{key} team"),
        "states": [state(&format!("{id}-s1"), "Todo")],
        "labels": [label(&format!("{id}-l1"), "Bug")],
        "members": [member(&format!("{id}-m1"), "ada")],
        "projects": [project(&format!("{id}-p1"), "Alpha")],
    })
}

fn team_at<'a>(catalogue: &'a Value, id: &str) -> &'a Value {
    catalogue["teams"]
        .as_array()
        .expect("teams is an array")
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap_or_else(|| panic!("no teams entry {id}"))
}

fn team_ids(catalogue: &Value) -> Vec<String> {
    catalogue["teams"]
        .as_array()
        .expect("teams is an array")
        .iter()
        .map(|entry| entry["id"].as_str().expect("an id").to_owned())
        .collect()
}

#[test]
fn the_catalogue_is_written_under_the_lock() {
    let fs = FakeFs::default();

    record(&fs, &update(vec![entry(complete_entry("t-a", "AAA"))]));

    assert!(fs.lock_entered.get());
    assert!(fs.exists(&catalogue_path()));
}

#[test]
fn a_held_lock_is_contention_not_a_clobber() {
    let fs = FakeFs::default();
    fs.lock_held.set(true);
    let cache = LinearCache::new(&fs, cache_root());

    let error = cache
        .record_team_entries(&update(vec![entry(complete_entry("t-a", "A"))]))
        .expect_err("contended");

    assert!(matches!(error, CacheError::LockContended { .. }));
    assert!(!fs.exists(&catalogue_path()));
}

#[test]
fn recording_replaces_each_carried_section_and_keeps_the_rest() {
    let fs = FakeFs::default();
    seed(&fs, &json!({ "teams": [complete_entry("t-a", "AAA")] }));

    record(
        &fs,
        &update(vec![entry(json!({
            "id": "t-a", "key": "AAA", "name": "AAA team",
            "labels": [label("t-a-l2", "Feature")],
        }))]),
    );

    let catalogue = written(&fs);
    let team = team_at(&catalogue, "t-a");
    assert_eq!(team["labels"], json!([label("t-a-l2", "Feature")]));
    assert_eq!(team["states"], json!([state("t-a-s1", "Todo")]));
    assert_eq!(team["members"], json!([member("t-a-m1", "ada")]));
    assert_eq!(team["projects"], json!([project("t-a-p1", "Alpha")]));
}

#[test]
fn two_entries_with_one_id_in_an_update_merge_section_by_section() {
    let fs = FakeFs::default();

    record(
        &fs,
        &update(vec![
            entry(json!({
                "id": "t-a", "key": "AAA", "name": "A",
                "states": [state("s-early", "Todo")],
                "labels": [label("l-1", "Bug")],
            })),
            entry(json!({
                "id": "t-a", "key": "AAA", "name": "A",
                "states": [state("s-late", "Doing")],
                "members": [member("m-1", "ada")],
            })),
        ]),
    );

    let catalogue = written(&fs);
    assert_eq!(team_ids(&catalogue), vec!["t-a"]);
    let team = team_at(&catalogue, "t-a");
    assert_eq!(team["states"], json!([state("s-late", "Doing")]));
    assert_eq!(team["labels"], json!([label("l-1", "Bug")]));
    assert_eq!(team["members"], json!([member("m-1", "ada")]));
}

#[test]
fn a_renamed_team_key_replaces_the_stored_key() {
    let fs = FakeFs::default();
    seed(&fs, &json!({ "teams": [complete_entry("t-a", "OLD")] }));

    record(
        &fs,
        &update(vec![entry(
            json!({ "id": "t-a", "key": "NEW", "name": "Renamed" }),
        )]),
    );

    let catalogue = written(&fs);
    let team = team_at(&catalogue, "t-a");
    assert_eq!(team["key"], "NEW");
    assert_eq!(team["name"], "Renamed");
    assert_eq!(team["states"], json!([state("t-a-s1", "Todo")]));
}

#[test]
fn a_replaced_section_keeps_each_records_stored_extra() {
    let fs = FakeFs::default();
    let mut stored = complete_entry("t-a", "AAA");
    stored["labels"][0]["colour"] = json!("#f00");
    seed(&fs, &json!({ "teams": [stored] }));

    record(
        &fs,
        &update(vec![entry(json!({
            "id": "t-a", "key": "AAA", "name": "AAA team",
            "labels": [label("t-a-l1", "Defect"), label("t-a-l9", "New")],
        }))]),
    );

    let catalogue = written(&fs);
    let team = team_at(&catalogue, "t-a");
    assert_eq!(
        team["labels"],
        json!([
            { "id": "t-a-l1", "name": "Defect", "archivedAt": null,
              "colour": "#f00" },
            label("t-a-l9", "New"),
        ])
    );
}

#[test]
fn a_new_entry_is_inserted_in_id_order_and_others_are_unchanged() {
    let fs = FakeFs::default();
    seed(
        &fs,
        &json!({
            "teams": [complete_entry("t-a", "AAA"), complete_entry("t-c", "CCC")],
        }),
    );
    let before = written(&fs);

    let added = record(
        &fs,
        &update(vec![entry(
            json!({ "id": "t-b", "key": "BBB", "name": "B" }),
        )]),
    );

    let catalogue = written(&fs);
    assert_eq!(added, vec!["BBB".to_owned()]);
    assert_eq!(team_ids(&catalogue), vec!["t-a", "t-b", "t-c"]);
    assert_eq!(team_at(&catalogue, "t-a"), team_at(&before, "t-a"));
    assert_eq!(team_at(&catalogue, "t-c"), team_at(&before, "t-c"));
}

#[test]
fn recording_writes_the_legacy_projections_from_the_base_entry() {
    let fs = FakeFs::default();
    let mut base = complete_entry("t-a", "AAA");
    base["states"] = json!([
        { "id": "s-1", "name": "Todo", "type": "unstarted", "position": 0,
          "archivedAt": null },
        { "id": "s-2", "name": "Old", "type": "started", "position": 3,
          "archivedAt": "2026-01-01T00:00:00.000Z" },
    ]);

    record(
        &fs,
        &CatalogueUpdate {
            base_team: Some("t-a".to_owned()),
            entries: vec![entry(base), entry(complete_entry("t-b", "BBB"))],
            workspace_labels: None,
        },
    );

    let catalogue = written(&fs);
    assert_eq!(catalogue["baseTeam"], "t-a");
    assert_eq!(
        catalogue["team"],
        json!({ "id": "t-a", "key": "AAA", "name": "AAA team" })
    );
    assert_eq!(
        catalogue["workflowStates"],
        json!([
            { "id": "s-1", "name": "Todo", "type": "unstarted", "position": 0 },
        ])
    );
}

#[test]
fn recording_onto_a_legacy_file_sets_base_team_from_the_legacy_team() {
    let fs = FakeFs::default();
    seed(
        &fs,
        &json!({
            "team": { "id": "t-a", "key": "AAA", "name": "A" },
            "workflowStates": [
                { "id": "s-1", "name": "Todo", "type": "unstarted",
                  "position": 0 },
            ],
        }),
    );

    record(
        &fs,
        &update(vec![entry(
            json!({ "id": "t-b", "key": "BBB", "name": "B" }),
        )]),
    );

    let catalogue = written(&fs);
    assert_eq!(catalogue["baseTeam"], "t-a");
    assert_eq!(team_ids(&catalogue), vec!["t-a", "t-b"]);
    assert_eq!(
        catalogue["team"],
        json!({ "id": "t-a", "key": "AAA", "name": "A" })
    );
}

#[test]
fn recording_never_re_points_an_existing_base_team() {
    let fs = FakeFs::default();
    seed(
        &fs,
        &json!({
            "baseTeam": "t-a",
            "teams": [complete_entry("t-a", "AAA")],
        }),
    );

    record(&fs, &update(vec![entry(complete_entry("t-b", "BBB"))]));
    assert_eq!(written(&fs)["baseTeam"], "t-a");

    record(
        &fs,
        &CatalogueUpdate {
            base_team: Some("t-b".to_owned()),
            entries: Vec::new(),
            workspace_labels: None,
        },
    );
    assert_eq!(written(&fs)["baseTeam"], "t-b");
    assert_eq!(written(&fs)["team"]["key"], "BBB");
}

#[test]
fn recording_onto_an_absent_catalogue_writes_every_key() {
    let fs = FakeFs::default();

    record(
        &fs,
        &CatalogueUpdate {
            base_team: Some("t-a".to_owned()),
            entries: vec![entry(complete_entry("t-a", "AAA"))],
            workspace_labels: Some(labels(json!([label("w-1", "Security")]))),
        },
    );

    let catalogue = written(&fs);
    let keys: Vec<&String> =
        catalogue.as_object().expect("an object").keys().collect();
    assert_eq!(
        keys,
        vec!["baseTeam", "labels", "team", "teams", "workflowStates"]
    );
    assert_eq!(catalogue["labels"], json!([label("w-1", "Security")]));
}

#[test]
fn recording_preserves_unknown_keys_at_every_level() {
    let fs = FakeFs::default();
    let mut stored = complete_entry("t-a", "AAA");
    stored["futureEntryKey"] = json!({ "kept": true });
    stored["states"][0]["futureStateKey"] = json!(7);
    seed(
        &fs,
        &json!({ "futureTopKey": [1, 2], "baseTeam": "t-a", "teams": [stored] }),
    );

    record(&fs, &update(vec![entry(complete_entry("t-a", "AAA"))]));

    let catalogue = written(&fs);
    assert_eq!(catalogue["futureTopKey"], json!([1, 2]));
    let team = team_at(&catalogue, "t-a");
    assert_eq!(team["futureEntryKey"], json!({ "kept": true }));
    assert_eq!(team["states"][0]["futureStateKey"], json!(7));
    let entry_keys: Vec<&String> =
        team.as_object().expect("an object").keys().collect();
    assert_eq!(
        entry_keys,
        vec![
            "futureEntryKey",
            "id",
            "key",
            "labels",
            "members",
            "name",
            "projects",
            "states",
        ],
        "no section key is duplicated into the entry's extra keys"
    );
}

#[test]
fn recording_normalises_null_sections() {
    let fs = FakeFs::default();
    seed(
        &fs,
        &json!({
            "teams": [{ "id": "t-a", "key": "AAA", "name": "A",
                        "labels": null, "states": [state("s-1", "Todo")] }],
        }),
    );

    record(&fs, &update(Vec::new()));

    let team = team_at(&written(&fs), "t-a").clone();
    assert!(team.get("labels").is_none(), "{team}");
    assert_eq!(team["states"], json!([state("s-1", "Todo")]));
}

#[test]
fn the_written_file_matches_the_golden_byte_for_byte() {
    let fs = FakeFs::default();
    let mut base = entry(json!({
        "id": "t-b", "key": "PP", "name": "Product Pod",
        "states": [
            { "id": "s-2", "name": "In Review", "type": "started",
              "position": 1002.5, "archivedAt": null },
            { "id": "s-1", "name": "Todo", "type": "unstarted",
              "position": 1002, "archivedAt": null },
        ],
        "labels": [label("l-2", "Feature"), label("l-1", "Bug")],
        "members": [member("m-2", "grace"), member("m-1", "ada")],
        "projects": [project("p-2", "Beta"), project("p-1", "Alpha")],
    }));
    base.extra.insert("zeta".to_owned(), json!(true));
    record(
        &fs,
        &CatalogueUpdate {
            base_team: Some("t-b".to_owned()),
            entries: vec![
                base,
                entry(json!({ "id": "t-a", "key": "OPS", "name": "Ops" })),
            ],
            workspace_labels: Some(labels(json!([
                label("w-2", "Security"),
                label("w-1", "Compliance"),
            ]))),
        },
    );
    let golden = include_str!("fixtures/catalogue-document.golden.json");

    let first = fs.get(&catalogue_path()).expect("written");
    assert_eq!(first, golden);

    record(&fs, &update(Vec::new()));
    let second = fs.get(&catalogue_path()).expect("re-written");
    assert_eq!(second, golden, "re-writing an unchanged document is stable");
}

#[test]
fn writers_refuse_an_unreadable_or_unparseable_catalogue() {
    let cases: Vec<(&str, Option<&str>, Option<&str>)> = vec![
        (
            "merge conflict markers",
            Some("<<<<<<< ours\n{}\n=======\n{}\n>>>>>>> theirs\n"),
            None,
        ),
        ("a non-object", Some("[1, 2]"), None),
        (
            "a non-array teams",
            Some(r#"{"teams": {"id": "t-a"}}"#),
            None,
        ),
        (
            "an entry failing typed parsing",
            Some(r#"{"teams": [{"id": "t-a", "key": 7, "name": "A"}]}"#),
            Some("t-a"),
        ),
        (
            "an entry with a blank id",
            Some(r#"{"teams": [{"id": " ", "key": "A", "name": "A"}]}"#),
            Some("teams[0]"),
        ),
        ("a read error", None, None),
    ];
    for (case, content, named) in cases {
        let fs = FakeFs::default();
        match content {
            Some(text) => {
                fs.files
                    .borrow_mut()
                    .insert(catalogue_path(), text.to_owned());
            }
            None => fs.unreadable.borrow_mut().push(catalogue_path()),
        }
        let before = fs.get(&catalogue_path());

        let error = LinearCache::new(&fs, cache_root())
            .record_team_entries(&update(vec![entry(complete_entry(
                "t-z", "ZZZ",
            ))]))
            .expect_err(case);

        match content {
            Some(_) => assert!(
                matches!(error, CacheError::Unparseable { .. }),
                "{case}: {error:?}"
            ),
            None => {
                assert!(matches!(error, CacheError::Io { .. }), "{case}");
            }
        }
        if let Some(named) = named {
            assert!(error.to_string().contains(named), "{case}: {error}");
        }
        assert_eq!(fs.get(&catalogue_path()), before, "{case}: untouched");
    }
}

#[test]
fn an_unparseable_catalogue_names_restoring_it_before_deleting_it() {
    let fs = FakeFs::default();
    fs.files
        .borrow_mut()
        .insert(catalogue_path(), "[]".to_owned());

    let message = LinearCache::new(&fs, cache_root())
        .record_team_entries(&update(Vec::new()))
        .expect_err("refused")
        .to_string();

    let restore = message.find("version control").expect("restore named");
    let delete = message.find("delete").expect("deletion named");
    assert!(restore < delete, "{message}");
}

#[test]
fn growing_a_team_changes_only_its_entry_and_the_projections() {
    let fs = FakeFs::default();
    let legacy_states = json!([
        { "id": "s-1", "name": "Todo", "type": "unstarted", "position": 0 },
    ]);
    seed(
        &fs,
        &json!({
            "team": { "id": "t-m", "key": "MMM", "name": "M" },
            "workflowStates": legacy_states,
            "teams": [{ "key": "ZZZ", "id": "t-z", "name": "Z" }],
        }),
    );

    let added = record(
        &fs,
        &update(vec![entry(
            json!({ "id": "t-b", "key": "BBB", "name": "B" }),
        )]),
    );

    let catalogue = written(&fs);
    assert_eq!(added, vec!["BBB".to_owned()]);
    assert_eq!(team_ids(&catalogue), vec!["t-b", "t-m", "t-z"]);
    assert_eq!(
        team_at(&catalogue, "t-z"),
        &json!({ "id": "t-z", "key": "ZZZ", "name": "Z" })
    );
    assert_eq!(
        catalogue["team"],
        json!({ "id": "t-m", "key": "MMM", "name": "M" })
    );
    assert_eq!(catalogue["workflowStates"], legacy_states);
}

fn parse(catalogue: &Value) -> CatalogueDocument {
    CatalogueDocument::parse(&catalogue.to_string(), Strictness::Forgiving)
        .expect("the catalogue parses")
}

#[test]
fn a_legacy_catalogue_reads_as_incomplete_entries() {
    let document = parse(&json!({
        "team": { "id": "t-m", "key": "MMM", "name": "M" },
        "workflowStates": [
            { "id": "s-1", "name": "Todo", "type": "unstarted", "position": 0 },
        ],
        "teams": [{ "key": "ZZZ", "id": "t-z", "name": "Z" }],
    }));

    let base = document.base_entry().expect("a base entry");
    assert_eq!(base.id, "t-m");
    assert_eq!(base.key, "MMM");
    assert_eq!(base.states.as_ref().map(Vec::len), Some(1));
    assert!(base.labels.is_none());
    assert!(base.members.is_none());
    assert!(base.projects.is_none());
    let grown = document
        .teams()
        .iter()
        .find(|team| team.id == "t-z")
        .expect("the 0229 entry");
    assert!(grown.states.is_none());
    assert!(grown.labels.is_none());
    assert!(grown.members.is_none());
    assert!(grown.projects.is_none());
}

#[test]
fn a_teams_entry_matching_the_legacy_team_is_the_base_entry() {
    let legacy = json!([
        { "id": "s-legacy", "name": "Todo", "type": "unstarted",
          "position": 0 },
    ]);
    let without_states = parse(&json!({
        "team": { "id": "t-a", "key": "AAA", "name": "A" },
        "workflowStates": legacy,
        "teams": [{ "id": "t-a", "key": "AAA", "name": "A",
                    "labels": [label("l-1", "Bug")] }],
    }));
    let with_states = parse(&json!({
        "team": { "id": "t-a", "key": "AAA", "name": "A" },
        "workflowStates": legacy,
        "teams": [{ "id": "t-a", "key": "AAA", "name": "A",
                    "states": [state("s-own", "Doing")] }],
    }));

    let filled = without_states.base_entry().expect("a base entry");
    assert_eq!(filled.states.as_ref().expect("states")[0].id, "s-legacy");
    assert_eq!(filled.labels.as_ref().map(Vec::len), Some(1));
    assert_eq!(without_states.teams().len(), 1);
    let kept = with_states.base_entry().expect("a base entry");
    assert_eq!(kept.states.as_ref().expect("states")[0].id, "s-own");
}

#[test]
fn unsorted_teams_written_by_an_older_binary_read_and_re_sort() {
    let fs = FakeFs::default();
    seed(
        &fs,
        &json!({
            "teams": [
                { "key": "CCC", "id": "t-c", "name": "C" },
                { "key": "AAA", "id": "t-a", "name": "A" },
                { "key": "BBB", "id": "t-b", "name": "B" },
            ],
        }),
    );

    record(&fs, &update(Vec::new()));

    assert_eq!(team_ids(&written(&fs)), vec!["t-a", "t-b", "t-c"]);
}

#[test]
fn duplicate_team_ids_merge_section_by_section_on_read() {
    let document = parse(&json!({
        "teams": [
            { "key": "OLD", "id": "t-a", "name": "A",
              "states": [state("s-1", "Todo")] },
            { "key": "AAA", "id": "t-a", "name": "A",
              "labels": [label("l-1", "Bug")] },
        ],
    }));

    assert_eq!(document.teams().len(), 1);
    let team = &document.teams()[0];
    assert_eq!(team.key, "AAA");
    assert_eq!(team.states.as_ref().map(Vec::len), Some(1));
    assert_eq!(team.labels.as_ref().map(Vec::len), Some(1));
}

#[test]
fn scaffold_upkeep_stays_lenient_about_an_unreadable_gitignore() {
    let fs = FakeFs::default();
    fs.unreadable
        .borrow_mut()
        .push(cache_root().join(".gitignore"));

    record(&fs, &update(vec![entry(complete_entry("t-a", "AAA"))]));

    assert_eq!(team_ids(&written(&fs)), vec!["t-a"]);
    assert_eq!(fs.get(&cache_root().join(".gitignore")), None);
}

#[test]
fn appending_to_an_unreadable_gitignore_refuses_rather_than_overwriting() {
    let dir = TempDir::new().expect("a temp dir");
    let gitignore = dir.path().join(".gitignore");
    std::fs::create_dir(&gitignore).expect("a directory where a file goes");
    let fs = SystemFilesystem::new(dir.path().to_path_buf());

    let error = fs
        .append_line(&gitignore, "viewer.json")
        .expect_err("the unreadable file is not overwritten");

    assert!(matches!(error, CacheError::Io { .. }));
    assert!(gitignore.is_dir());
}

#[test]
fn a_forgiving_read_keeps_the_legacy_base_despite_a_damaged_teams_array() {
    let document = parse(&json!({
        "team": { "id": "t-a", "key": "AAA", "name": "A" },
        "teams": { "not": "an array" },
    }));

    assert_eq!(
        document.base_entry().map(|base| base.key.as_str()),
        Some("AAA")
    );
}

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::sync::Arc;
use std::thread;

use serde_json::json;
use serde_json::Value;
use shortcut_client::cache::CacheError;
use shortcut_client::cache::Listing;
use shortcut_client::cache::ShortcutCache;
use shortcut_client::cache::SystemFilesystem;
use shortcut_client::catalogue::Catalogue;
use shortcut_client::catalogue::CatalogueError;
use shortcut_client::catalogue::SCHEMA_VERSION;
use tempfile::TempDir;

const TEAM: &str = "8f1c2d3e-0000-4000-8000-000000000001";

fn catalogue(name: &str) -> Catalogue {
    Catalogue::parse(
        &json!({
            "schema_version": SCHEMA_VERSION,
            "workspace": {"slug": "acme", "name": name},
            "teams": [{"id": TEAM, "name": "Mobile", "mention_name": "mobile",
                       "default_workflow_id": 500, "workflow_ids": [500]}],
            "workflows": [{"id": 500, "name": "Engineering", "states": []}]
        })
        .to_string(),
    )
    .expect("a valid catalogue")
}

fn state_dir(root: &TempDir) -> std::path::PathBuf {
    root.path().join("integrations/shortcut")
}

fn cache_over<'a>(
    fs: &'a SystemFilesystem,
    root: &TempDir,
) -> ShortcutCache<'a> {
    ShortcutCache::new(fs, state_dir(root))
}

fn on_disk(root: &TempDir) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(state_dir(root).join("catalogue.json"))
            .expect("written"),
    )
    .expect("json")
}

#[test]
fn an_absent_catalogue_loads_as_none() {
    let root = TempDir::new().expect("scratch");
    let fs = SystemFilesystem::new(root.path().to_path_buf());

    assert!(cache_over(&fs, &root).load().expect("reads").is_none());
}

#[test]
fn a_complete_catalogue_is_written_and_reloads() {
    let root = TempDir::new().expect("scratch");
    let fs = SystemFilesystem::new(root.path().to_path_buf());
    let cache = cache_over(&fs, &root);

    cache
        .write(catalogue("Acme"), Listing::Complete)
        .expect("writes");

    let loaded = cache.load().expect("reads").expect("present");
    assert_eq!(loaded.workspace().name, "Acme");
}

#[test]
fn an_incomplete_catalogue_is_never_written() {
    let root = TempDir::new().expect("scratch");
    let fs = SystemFilesystem::new(root.path().to_path_buf());
    let cache = cache_over(&fs, &root);

    let error = cache
        .write(
            catalogue("Acme"),
            Listing::Incomplete {
                listing: "teams".to_owned(),
                cause: "the deadline passed".to_owned(),
            },
        )
        .expect_err("refused");

    assert!(matches!(error, CacheError::Incomplete { .. }));
    let message = error.to_string();
    assert!(message.contains("teams"), "{message}");
    assert!(message.contains("the deadline passed"), "{message}");
    assert!(message.contains("retry"), "{message}");
    assert!(!state_dir(&root).join("catalogue.json").exists());
}

#[test]
fn a_newer_catalogue_on_disk_is_never_replaced() {
    let root = TempDir::new().expect("scratch");
    let fs = SystemFilesystem::new(root.path().to_path_buf());
    std::fs::create_dir_all(state_dir(&root)).expect("dir");
    let newer = json!({"schema_version": SCHEMA_VERSION + 1, "anything": true});
    std::fs::write(state_dir(&root).join("catalogue.json"), newer.to_string())
        .expect("seed");

    let error = cache_over(&fs, &root)
        .write(catalogue("Acme"), Listing::Complete)
        .expect_err("refused");

    assert!(
        matches!(
            error,
            CacheError::Catalogue(CatalogueError::NewerSchema { .. })
        ),
        "{error:?}"
    );
    assert_eq!(on_disk(&root), newer);
}

#[test]
fn a_rewrite_keeps_fields_a_newer_teammate_added() {
    let root = TempDir::new().expect("scratch");
    let fs = SystemFilesystem::new(root.path().to_path_buf());
    let cache = cache_over(&fs, &root);
    cache
        .write(catalogue("Acme"), Listing::Complete)
        .expect("first");
    let mut seeded = on_disk(&root);
    seeded["teams"][0]["colour"] = json!("blue");
    std::fs::write(state_dir(&root).join("catalogue.json"), seeded.to_string())
        .expect("seed");

    cache
        .write(catalogue("Acme Inc"), Listing::Complete)
        .expect("second");

    let written = on_disk(&root);
    assert_eq!(written["workspace"]["name"], json!("Acme Inc"));
    assert_eq!(written["teams"][0]["colour"], json!("blue"));
}

#[test]
fn a_write_scaffolds_the_gitignore() {
    let root = TempDir::new().expect("scratch");
    let fs = SystemFilesystem::new(root.path().to_path_buf());

    cache_over(&fs, &root)
        .write(catalogue("Acme"), Listing::Complete)
        .expect("writes");

    let ignore = std::fs::read_to_string(state_dir(&root).join(".gitignore"))
        .expect("scaffolded");
    for rule in ["viewer.json", ".refresh-meta.json", ".lock/"] {
        assert!(ignore.lines().any(|line| line == rule), "{rule}: {ignore}");
    }
    assert!(!ignore.contains("catalogue.json"), "{ignore}");
}

#[test]
fn concurrent_writers_leave_one_valid_catalogue() {
    let root = Arc::new(TempDir::new().expect("scratch"));
    let writers: Vec<_> = ["One", "Two", "Three", "Four"]
        .into_iter()
        .map(|name| {
            let root = Arc::clone(&root);
            thread::spawn(move || {
                let fs = SystemFilesystem::new(root.path().to_path_buf());
                ShortcutCache::new(&fs, state_dir(&root))
                    .write(catalogue(name), Listing::Complete)
            })
        })
        .collect();
    for writer in writers {
        let outcome = writer.join().expect("joins");
        assert!(
            matches!(outcome, Ok(()) | Err(CacheError::LockContended { .. })),
            "{outcome:?}"
        );
    }

    let name = on_disk(&root)["workspace"]["name"].clone();
    assert!(["One", "Two", "Three", "Four"]
        .iter()
        .any(|n| name == json!(n)));
}

#[test]
fn an_invalid_catalogue_on_disk_is_reported_as_a_load_error() {
    let root = TempDir::new().expect("scratch");
    let fs = SystemFilesystem::new(root.path().to_path_buf());
    std::fs::create_dir_all(state_dir(&root)).expect("dir");
    std::fs::write(Path::new(&state_dir(&root)).join("catalogue.json"), "{")
        .expect("seed");

    assert!(matches!(
        cache_over(&fs, &root).load(),
        Err(CacheError::Catalogue(CatalogueError::Unparseable { .. }))
    ));
}

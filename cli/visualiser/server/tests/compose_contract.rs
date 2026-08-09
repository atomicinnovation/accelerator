//! Contract for the Model-1 config composition (`compose::load`): the full
//! resolved set the retired `write-visualiser-config.sh` produced — the 13
//! doc-path keys, the template set with three-tier resolution incl.
//! `config_override_source`, the kanban columns, and the work-item scheme.

use std::path::{Path, PathBuf};

use accelerator_visualiser::compose::{load, Params};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// A throwaway project whose config overrides exercise both levels.
fn seed_project(dir: &Path) {
    use std::os::unix::fs::PermissionsExt as _;

    let acc = dir.join(".accelerator");
    std::fs::create_dir_all(acc.join("tmp")).unwrap();
    std::fs::write(acc.join("tmp/.gitignore"), "").unwrap();
    std::fs::write(
        acc.join("config.md"),
        "---\n\
         paths:\n  work: custom/work\n\
         templates:\n  plan: custom/plan.md\n\
         visualiser:\n  kanban_columns: [ready, in-progress, done]\n  idle_timeout: 30m\n\
         work:\n  id_pattern: \"{project}-{number:04d}\"\n  default_project_code: ENG\n---\n",
    )
    .unwrap();
    let local = acc.join("config.local.md");
    std::fs::write(&local, "---\ntemplates:\n  adr: local/adr.md\n---\n")
        .unwrap();
    std::fs::set_permissions(&local, std::fs::Permissions::from_mode(0o600))
        .unwrap();
}

fn compose(dir: &Path) -> accelerator_visualiser::config::Config {
    load(Params {
        cwd: dir.to_path_buf(),
        plugin_root: repo_root(),
        owner_pid: 0,
        owner_start_time: None,
        host: "127.0.0.1".to_string(),
    })
    .expect("compose the fixture project")
}

#[test]
fn resolves_all_thirteen_doc_paths_with_overrides_and_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path());
    let cfg = compose(tmp.path());

    let mut keys: Vec<&str> =
        cfg.doc_paths.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "decisions",
            "notes",
            "plans",
            "prs",
            "research_codebase",
            "research_design_gaps",
            "research_design_inventories",
            "research_issues",
            "review_plans",
            "review_prs",
            "review_work",
            "validations",
            "work",
        ]
    );
    // Overridden path is honoured; unset paths take the catalogue default.
    assert_eq!(cfg.doc_paths["work"], tmp.path().join("custom/work"));
    assert_eq!(
        cfg.doc_paths["decisions"],
        tmp.path().join("meta/decisions")
    );
    assert_eq!(
        cfg.doc_paths["research_codebase"],
        tmp.path().join("meta/research/codebase")
    );
}

#[test]
fn template_set_matches_plugin_templates_with_tiered_sources() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path());
    let cfg = compose(tmp.path());

    let mut expected: Vec<String> =
        std::fs::read_dir(repo_root().join("templates"))
            .unwrap()
            .filter_map(|e| {
                let p = e.ok()?.path();
                if p.is_file() && p.extension()? == "md" {
                    Some(p.file_stem()?.to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .collect();
    expected.sort();
    let mut actual: Vec<String> = cfg.templates.keys().cloned().collect();
    actual.sort();
    assert_eq!(
        actual, expected,
        "template set must equal plugin templates/*.md"
    );

    // Team-level override (config.md) records its source file.
    let plan = &cfg.templates["plan"];
    assert_eq!(
        plan.config_override.as_deref(),
        Some(Path::new("custom/plan.md"))
    );
    assert_eq!(
        plan.config_override_source.as_deref(),
        Some(".accelerator/config.md")
    );
    // Personal-level override (config.local.md) records its source file.
    let adr = &cfg.templates["adr"];
    assert_eq!(
        adr.config_override.as_deref(),
        Some(Path::new("local/adr.md"))
    );
    assert_eq!(
        adr.config_override_source.as_deref(),
        Some(".accelerator/config.local.md")
    );
    // An un-overridden template has no config override or source, and its
    // plugin default points into the plugin templates dir.
    let note = &cfg.templates["note"];
    assert!(note.config_override.is_none());
    assert!(note.config_override_source.is_none());
    assert!(note.plugin_default.ends_with("note.md"));
    assert!(note.user_override.ends_with("note.md"));
}

#[test]
fn resolves_kanban_idle_and_work_item_scheme() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path());
    let cfg = compose(tmp.path());

    assert_eq!(
        cfg.kanban_columns.as_deref(),
        Some(
            [
                "ready".to_string(),
                "in-progress".to_string(),
                "done".to_string()
            ]
            .as_slice()
        )
    );
    assert_eq!(cfg.resolve_idle_limit_ms().unwrap(), 30 * 60 * 1000);

    let work_item = cfg.work_item.as_ref().expect("work_item resolved");
    assert_eq!(work_item.scan_regex, "^ENG-([0-9]+)-");
    assert_eq!(work_item.default_project_code.as_deref(), Some("ENG"));
}

#[test]
fn unconfigured_project_uses_catalogue_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let acc = tmp.path().join(".accelerator");
    std::fs::create_dir_all(&acc).unwrap();
    std::fs::write(acc.join("config.md"), "---\n---\n").unwrap();
    let cfg = compose(tmp.path());

    assert_eq!(cfg.doc_paths["work"], tmp.path().join("meta/work"));
    // Absent kanban → resolver applies the catalogue's seven defaults.
    assert!(cfg.kanban_columns.is_none());
    assert_eq!(cfg.resolve_kanban_columns().unwrap().len(), 7);
    // Absent idle → catalogue 8h default.
    assert_eq!(cfg.resolve_idle_limit_ms().unwrap(), 8 * 60 * 60 * 1000);
    // Absent work scheme → numeric default.
    let work_item = cfg.work_item.as_ref().unwrap();
    assert_eq!(work_item.scan_regex, "^([0-9]+)-");
    assert!(work_item.default_project_code.is_none());
}

/// The emptiness rule lives in `with_plugin_root`, so the server inherits it:
/// an empty root refuses rather than resolving plugin templates against cwd.
#[test]
fn an_empty_plugin_root_refuses_to_compose() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path());
    let error = load(Params {
        cwd: tmp.path().to_path_buf(),
        plugin_root: PathBuf::new(),
        owner_pid: 0,
        owner_start_time: None,
        host: "127.0.0.1".to_string(),
    })
    .expect_err("an empty plugin root composed a config");
    assert!(
        error.to_string().contains("ACCELERATOR_PLUGIN_ROOT"),
        "the error does not name the variable: {error}"
    );
}

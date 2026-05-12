use std::process::Command;

use accelerator_visualiser::config::Config;

#[test]
fn write_visualiser_config_produces_valid_config_json() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../scripts/write-visualiser-config.sh");
    assert!(
        script.exists(),
        "write-visualiser-config.sh not found at {}",
        script.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("project");
    std::fs::create_dir_all(&project_root).unwrap();

    let output = Command::new("bash")
        .arg(&script)
        .args(["--plugin-version", "0.0.0-contract-test"])
        .args(["--project-root", project_root.to_str().unwrap()])
        .args(["--tmp-dir", tmp.path().join("visualiser").to_str().unwrap()])
        .args([
            "--log-file",
            tmp.path().join("server.log").to_str().unwrap(),
        ])
        .args(["--owner-pid", "0"])
        .output()
        .expect("spawn write-visualiser-config.sh");
    assert!(
        output.status.success(),
        "script failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cfg_path = tmp.path().join("config.json");
    std::fs::write(&cfg_path, &output.stdout).unwrap();
    let cfg = Config::from_path(&cfg_path)
        .expect("config.json produced by script must deserialise as Config");

    assert_eq!(cfg.plugin_version, "0.0.0-contract-test");
    assert_eq!(cfg.host, "127.0.0.1");
    assert_eq!(cfg.owner_pid, 0);
    assert_eq!(
        cfg.doc_paths.len(),
        13,
        "expected 13 doc_paths, got {:?}",
        cfg.doc_paths.keys().collect::<Vec<_>>()
    );
    for key in [
        "decisions",
        "work",
        "review_work",
        "plans",
        "research_codebase",
        "research_issues",
        "review_plans",
        "review_prs",
        "validations",
        "notes",
        "prs",
        "research_design_gaps",
        "research_design_inventories",
    ] {
        assert!(
            cfg.doc_paths.contains_key(key),
            "doc_paths missing key: {key}"
        );
    }
    assert_eq!(
        cfg.templates.len(),
        8,
        "expected 8 templates, got {:?}",
        cfg.templates.keys().collect::<Vec<_>>()
    );
    for name in ["adr", "plan", "codebase-research", "validation", "pr-description", "work-item", "design-gap", "design-inventory"] {
        assert!(
            cfg.templates.contains_key(name),
            "templates missing: {name}"
        );
        let tiers = cfg.templates.get(name).unwrap();
        assert!(
            tiers
                .plugin_default
                .to_string_lossy()
                .ends_with(&format!("{name}.md")),
            "plugin_default for {name} should end with {name}.md, got {}",
            tiers.plugin_default.display()
        );
    }
}

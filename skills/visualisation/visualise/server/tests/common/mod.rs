use std::collections::HashMap;
use std::path::Path;

use accelerator_visualiser::config::{
    Config, RawWorkItemConfig, TemplateTiers,
};

// Shared helper compiled into every integration test binary; only some use
// it, so binaries that don't would otherwise flag it as dead code.
#[allow(dead_code)]
pub fn set_mtime_ms(path: &Path, ms: i64) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::time::{Duration, SystemTime};
    let f = OpenOptions::new().write(true).open(path)?;
    f.set_modified(SystemTime::UNIX_EPOCH + Duration::from_millis(ms as u64))
}

pub fn seeded_cfg(tmp: &Path) -> Config {
    let meta = tmp.join("meta");
    let decisions = meta.join("decisions");
    let plans = meta.join("plans");
    let reviews = meta.join("reviews/plans");
    let issues = meta.join("research/issues");
    let tmp_dir = meta.join("tmp/visualiser");
    for d in [&decisions, &plans, &reviews, &issues, &tmp_dir] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(
        decisions.join("ADR-0001-foo.md"),
        "---\nadr_id: ADR-0001\ntitle: The Foo Decision\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        plans.join("2026-04-18-foo.md"),
        "---\ntitle: The Foo Plan\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        reviews.join("2026-04-18-foo-review-1.md"),
        "---\ntarget: \"meta/plans/2026-04-18-foo.md\"\n---\n",
    )
    .unwrap();
    // One isolated RCA so the Operate phase / `root-cause-analyses` count is a
    // deterministic 1 with an unambiguous `latest`. The title/slug avoid every
    // search-suite query token so adding it cannot perturb api_search totals.
    std::fs::write(
        issues.join("2026-06-10-example-rca.md"),
        "---\ntitle: \"Example RCA\"\ntype: issue-research\nstatus: resolved\n---\n# body\n",
    )
    .unwrap();

    let tpl_dir = tmp.join("plugin-templates");
    std::fs::create_dir_all(&tpl_dir).unwrap();
    let mut templates = HashMap::new();
    for name in [
        "adr",
        "plan",
        "research",
        "validation",
        "pr-description",
        "work-item",
        "design-gap",
        "design-inventory",
    ] {
        let pd = tpl_dir.join(format!("{name}.md"));
        std::fs::write(&pd, format!("# {name} plugin default\n")).unwrap();
        templates.insert(
            name.to_string(),
            TemplateTiers {
                config_override: None,
                user_override: meta.join(format!("templates/{name}.md")),
                plugin_default: pd,
                config_override_source: None,
            },
        );
    }

    let mut doc_paths = HashMap::new();
    doc_paths.insert("decisions".into(), decisions);
    doc_paths.insert("work".into(), meta.join("work"));
    doc_paths.insert("plans".into(), plans);
    doc_paths
        .insert("research_codebase".into(), meta.join("research/codebase"));
    doc_paths.insert("research_issues".into(), meta.join("research/issues"));
    doc_paths.insert("review_plans".into(), reviews);
    doc_paths.insert("review_prs".into(), meta.join("reviews/prs"));
    doc_paths.insert("review_work".into(), meta.join("reviews/work"));
    doc_paths.insert("validations".into(), meta.join("validations"));
    doc_paths.insert("notes".into(), meta.join("notes"));
    doc_paths.insert("prs".into(), meta.join("prs"));
    doc_paths.insert(
        "research_design_gaps".into(),
        meta.join("research/design-gaps"),
    );
    doc_paths.insert(
        "research_design_inventories".into(),
        meta.join("research/design-inventories"),
    );

    Config {
        plugin_root: tmp.to_path_buf(),
        plugin_version: "test".into(),
        project_root: tmp.to_path_buf(),
        tmp_path: tmp_dir,
        host: "127.0.0.1".into(),
        owner_pid: 0,
        owner_start_time: None,
        log_path: tmp.join("server.log"),
        doc_paths,
        templates,
        work_item: None,
        kanban_columns: None,
        idle_timeout: None,
        editor: None,
        editor_project: None,
    }
}

// Shared helper compiled into every integration test binary; only the
// work-item suites call it, so the others would flag it as dead code.
#[allow(dead_code)]
pub fn seeded_cfg_with_work_items(tmp: &Path) -> Config {
    let mut cfg = seeded_cfg(tmp);
    let work = tmp.join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(
        work.join("0001-todo-fixture.md"),
        "---\ntitle: \"Todo fixture\"\ntype: adr-creation-task\nstatus: todo\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        work.join("0002-done-fixture.md"),
        "---\ntitle: \"Done fixture\"\ntype: adr-creation-task\nstatus: done\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        work.join("0003-other-fixture.md"),
        "---\ntitle: \"Blocked fixture\"\ntype: adr-creation-task\nstatus: blocked\n---\n# body\n",
    )
    .unwrap();
    cfg.doc_paths.insert("work".into(), work);
    cfg
}

// Seeds the standard corpus plus a project-code work-item config (e.g.
// `ENG`), so frontmatter `id: "0042"` normalises to `ENG-0042`. Used by the
// search id-matching suite to exercise the non-numeric / project-code paths.
#[allow(dead_code)]
pub fn seeded_cfg_project_code(tmp: &Path, code: &str) -> Config {
    let mut cfg = seeded_cfg(tmp);
    cfg.work_item = Some(RawWorkItemConfig {
        scan_regex: format!("^({code}-[0-9]{{4}})-"),
        id_pattern: "{project}-{number:04d}".to_string(),
        default_project_code: Some(code.to_string()),
    });
    cfg
}

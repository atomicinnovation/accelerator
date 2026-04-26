use std::collections::HashMap;
use std::path::Path;

use accelerator_visualiser::config::{Config, TemplateTiers};

pub fn seeded_cfg(tmp: &Path) -> Config {
    let meta = tmp.join("meta");
    let decisions = meta.join("decisions");
    let plans = meta.join("plans");
    let reviews = meta.join("reviews/plans");
    let tmp_dir = meta.join("tmp/visualiser");
    for d in [&decisions, &plans, &reviews, &tmp_dir] {
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

    let tpl_dir = tmp.join("plugin-templates");
    std::fs::create_dir_all(&tpl_dir).unwrap();
    let mut templates = HashMap::new();
    for name in ["adr", "plan", "research", "validation", "pr-description"] {
        let pd = tpl_dir.join(format!("{name}.md"));
        std::fs::write(&pd, format!("# {name} plugin default\n")).unwrap();
        templates.insert(
            name.to_string(),
            TemplateTiers {
                config_override: None,
                user_override: meta.join(format!("templates/{name}.md")),
                plugin_default: pd,
            },
        );
    }

    let mut doc_paths = HashMap::new();
    doc_paths.insert("decisions".into(), decisions);
    doc_paths.insert("tickets".into(), meta.join("tickets"));
    doc_paths.insert("plans".into(), plans);
    doc_paths.insert("research".into(), meta.join("research"));
    doc_paths.insert("review_plans".into(), reviews);
    doc_paths.insert("review_prs".into(), meta.join("reviews/prs"));
    doc_paths.insert("validations".into(), meta.join("validations"));
    doc_paths.insert("notes".into(), meta.join("notes"));
    doc_paths.insert("prs".into(), meta.join("prs"));

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
    }
}

#[allow(dead_code)]
pub fn seeded_cfg_with_tickets(tmp: &Path) -> Config {
    let mut cfg = seeded_cfg(tmp);
    let tickets = tmp.join("meta/tickets");
    std::fs::create_dir_all(&tickets).unwrap();
    std::fs::write(
        tickets.join("0001-todo-fixture.md"),
        "---\ntitle: \"Todo fixture\"\ntype: adr-creation-task\nstatus: todo\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        tickets.join("0002-done-fixture.md"),
        "---\ntitle: \"Done fixture\"\ntype: adr-creation-task\nstatus: done\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        tickets.join("0003-other-fixture.md"),
        "---\ntitle: \"Blocked fixture\"\ntype: adr-creation-task\nstatus: blocked\n---\n# body\n",
    )
    .unwrap();
    cfg.doc_paths
        .insert("tickets".into(), tickets);
    cfg
}

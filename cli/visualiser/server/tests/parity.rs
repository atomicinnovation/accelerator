//! Parity fixtures for the domain surfaces retired onto the shared crates.
//!
//! Each case pins the server-visible behaviour of the shared parser/conventions
//! so the retirement stays reproducible after the old private modules are gone.
//! Where the shared parser (serde-saphyr, YAML 1.2 with 1.1-style booleans)
//! differs from the retired `serde_yml` engine, the pinned value is the shared
//! parser's — a deliberate, documented dialect adoption, not a regression.

use std::collections::HashMap;
use std::path::PathBuf;

use accelerator_visualiser::clusters::{
    compute_clusters_with_backfill, ClusterContext,
};
use accelerator_visualiser::config::WorkItemConfig;
use accelerator_visualiser::frontmatter::{self, FrontmatterState};
use accelerator_visualiser::indexer::IndexEntry;
use corpus::DocTypeKey;

/// Slug derivation and work-item id admission now delegate to corpus; pin
/// representative cases so the delegation stays behaviour-stable.
#[test]
fn slug_and_id_conventions_are_pinned() {
    let cfg = WorkItemConfig::default_numeric();
    let derive = |kind, f: &str| {
        corpus::slug::derive(kind, f, cfg.scheme(), cfg.scanner())
    };
    assert_eq!(
        derive(DocTypeKey::WorkItems, "0042-ship-it.md").as_deref(),
        Some("ship-it")
    );
    assert_eq!(
        derive(DocTypeKey::Decisions, "ADR-0012-some-choice.md").as_deref(),
        Some("some-choice")
    );
    assert_eq!(
        corpus::slug::derive_work_item("0007-thing.md", cfg.scanner())
            .as_deref(),
        Some("thing")
    );
    assert_eq!(corpus::slug::humanise_slug("2026-07-23-my-note"), "My Note");

    assert_eq!(cfg.extract_id("0042-x.md").as_deref(), Some("0042"));
    assert_eq!(cfg.extract_id("no-id.md"), None);
    assert_eq!(cfg.normalise_id("  42  ").as_deref(), Some("42"));
    assert_eq!(cfg.normalise_id("ENG-7").as_deref(), Some("ENG-7"));
    assert!(cfg.is_canonical_id_token("0042"));
    assert!(!cfg.is_canonical_id_token("42"));
}

/// The doc-type wire token and config-path key are the load-bearing SPA/config
/// contract. Pin all 14 variants to the exact tokens the retired serde-derived
/// enum produced, so the swap onto `corpus::DocTypeKey` is provably
/// wire-stable.
#[test]
fn doc_type_wire_and_config_keys_are_pinned() {
    let rows: &[(DocTypeKey, &str, Option<&str>)] = &[
        (DocTypeKey::Decisions, "decisions", Some("decisions")),
        (DocTypeKey::WorkItems, "work-items", Some("work")),
        (DocTypeKey::Plans, "plans", Some("plans")),
        (DocTypeKey::Research, "research", Some("research_codebase")),
        (
            DocTypeKey::PlanReviews,
            "plan-reviews",
            Some("review_plans"),
        ),
        (DocTypeKey::PrReviews, "pr-reviews", Some("review_prs")),
        (
            DocTypeKey::WorkItemReviews,
            "work-item-reviews",
            Some("review_work"),
        ),
        (DocTypeKey::Validations, "validations", Some("validations")),
        (DocTypeKey::Notes, "notes", Some("notes")),
        (DocTypeKey::PrDescriptions, "pr-descriptions", Some("prs")),
        (
            DocTypeKey::DesignGaps,
            "design-gaps",
            Some("research_design_gaps"),
        ),
        (
            DocTypeKey::DesignInventories,
            "design-inventories",
            Some("research_design_inventories"),
        ),
        (
            DocTypeKey::RootCauseAnalyses,
            "root-cause-analyses",
            Some("research_issues"),
        ),
        (DocTypeKey::Templates, "templates", None),
    ];
    assert_eq!(rows.len(), DocTypeKey::all().len());
    for (kind, wire, config_key) in rows {
        assert_eq!(kind.wire_str(), *wire, "wire for {kind:?}");
        assert_eq!(DocTypeKey::from_wire_str(wire), Some(*kind));
        assert_eq!(kind.config_path_key(), *config_key, "config for {kind:?}");
        if let Some(key) = kind.config_path_key() {
            assert!(
                config::catalogue::PATH_KEYS
                    .iter()
                    .any(|(path_key, _)| path_key.strip_prefix("paths.")
                        == Some(key)),
                "{kind:?} claims config path key {key:?}, which \
                 config::catalogue::PATH_KEYS does not declare"
            );
        }
    }
}

fn parsed_json(input: &str) -> serde_json::Value {
    match frontmatter::parse(input.as_bytes()).state {
        FrontmatterState::Parsed(m) => {
            serde_json::Value::Object(m.into_iter().collect())
        }
        FrontmatterState::Absent => serde_json::json!("absent"),
        FrontmatterState::Malformed => serde_json::json!("malformed"),
    }
}

/// The frontmatter map is serialised to JSON for the SPA, so a value-type flip
/// is a wire change. Each row pins the exact JSON the shared parser produces.
#[test]
fn frontmatter_scalar_dialect_is_pinned() {
    let cases: &[(&str, serde_json::Value)] = &[
        // Ordinary strings/words are unchanged.
        (
            "---\ntitle: Hello World\nstatus: ready\n---\n",
            serde_json::json!({"title": "Hello World", "status": "ready"}),
        ),
        (
            "---\na: true\nb: false\n---\n",
            serde_json::json!({"a": true, "b": false}),
        ),
        // FLIP: YAML 1.1-style booleans parse as bool (were strings).
        ("---\nflag: yes\n---\n", serde_json::json!({"flag": true})),
        ("---\nflag: no\n---\n", serde_json::json!({"flag": false})),
        ("---\nflag: on\n---\n", serde_json::json!({"flag": true})),
        ("---\nflag: off\n---\n", serde_json::json!({"flag": false})),
        ("---\nflag: y\n---\n", serde_json::json!({"flag": true})),
        ("---\nflag: n\n---\n", serde_json::json!({"flag": false})),
        // Plain integers and quoted numerics are unchanged.
        ("---\ncount: 42\n---\n", serde_json::json!({"count": 42})),
        ("---\nn: -5\n---\n", serde_json::json!({"n": -5})),
        (
            "---\nversion: \"1.20\"\n---\n",
            serde_json::json!({"version": "1.20"}),
        ),
        // null spellings.
        (
            "---\na: ~\nb: null\nc:\n---\n",
            serde_json::json!({"a": null, "b": null, "c": null}),
        ),
        // Sequences and nested maps round-trip.
        (
            "---\ntags: [a, b]\n---\n",
            serde_json::json!({"tags": ["a", "b"]}),
        ),
        (
            "---\nmeta:\n  k: v\n  n: 1\n---\n",
            serde_json::json!({"meta": {"k": "v", "n": 1}}),
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(&parsed_json(input), expected, "input: {input:?}");
    }
}

fn entry(
    kind: DocTypeKey,
    path: &str,
    slug: &str,
    frontmatter: serde_json::Value,
) -> IndexEntry {
    IndexEntry {
        r#type: kind,
        path: PathBuf::from(path),
        rel_path: PathBuf::from(path),
        slug: Some(slug.to_string()),
        work_item_id: None,
        title: slug.to_string(),
        frontmatter,
        frontmatter_state: "parsed".to_string(),
        work_item_refs: Vec::new(),
        mtime_ms: 1,
        size: 0,
        etag: "sha256-x".to_string(),
        body_preview: String::new(),
        completeness: None,
        linked_count: 0,
        cluster_key: None,
    }
}

/// Lifecycle clustering (the typed-linkage walk retired onto `corpus::cluster`)
/// stays behaviour-stable: a work item, its `parent:`-linked plan, and a
/// same-slug note cluster as before — the plan adopts the work-item id, and the
/// orphan-by-design note stays in its own bucket.
#[test]
fn clustering_typed_linkage_is_pinned() {
    let cfg = WorkItemConfig::default_numeric();
    let mut wi = entry(
        DocTypeKey::WorkItems,
        "/repo/meta/work/0040-pipeline.md",
        "pipeline",
        serde_json::Value::Null,
    );
    wi.work_item_id = Some("0040".to_string());
    let plan = entry(
        DocTypeKey::Plans,
        "/repo/meta/plans/2026-05-31-0040-pipeline.md",
        "pipeline",
        serde_json::json!({ "parent": "work-item:0040" }),
    );
    let note = entry(
        DocTypeKey::Notes,
        "/repo/meta/notes/loose-thought.md",
        "loose-thought",
        serde_json::Value::Null,
    );
    let entries = vec![wi, plan, note];

    let work_item_by_id: HashMap<String, PathBuf> = [(
        "0040".to_string(),
        PathBuf::from("/repo/meta/work/0040-pipeline.md"),
    )]
    .into_iter()
    .collect();
    let plans_by_id = HashMap::new();
    let root = PathBuf::from("/repo");
    let ctx = ClusterContext::from_entries(
        &entries,
        &work_item_by_id,
        &plans_by_id,
        &root,
        &cfg,
    );
    let (clusters, _, cluster_key_by_path) =
        compute_clusters_with_backfill(&entries, &ctx);

    assert_eq!(clusters.len(), 2);
    let pipeline = clusters
        .iter()
        .find(|c| c.slug == "pipeline")
        .expect("pipeline cluster");
    assert_eq!(pipeline.cluster_key.as_deref(), Some("0040"));
    assert_eq!(pipeline.completeness.present, vec!["work-items", "plans"]);
    assert_eq!(
        cluster_key_by_path
            [&PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md")]
            .as_deref(),
        Some("0040")
    );
    let note = clusters
        .iter()
        .find(|c| c.slug == "loose-thought")
        .expect("note cluster");
    assert_eq!(note.cluster_key, None);
}

/// Structure boundaries: state classification across the fence edge cases.
#[test]
fn frontmatter_structure_states_are_pinned() {
    assert_eq!(
        parsed_json("# Heading\nbody\n"),
        serde_json::json!("absent")
    );
    assert_eq!(
        parsed_json("---\ntitle: Hi\nno closing fence"),
        serde_json::json!("malformed")
    );
    // A non-mapping root is malformed.
    assert_eq!(
        parsed_json("---\n- a\n- b\n---\nbody\n"),
        serde_json::json!("malformed")
    );
    // Empty frontmatter is an empty parsed map.
    assert_eq!(parsed_json("---\n---\nbody\n"), serde_json::json!({}));
    // CRLF fences parse.
    assert_eq!(
        parsed_json("---\r\ntitle: Hi\r\n---\r\nbody\r\n"),
        serde_json::json!({"title": "Hi"})
    );
    // No trailing newline after the closing fence still parses.
    assert_eq!(
        parsed_json("---\ntitle: Hi\n---\nbody"),
        serde_json::json!({"title": "Hi"})
    );
}

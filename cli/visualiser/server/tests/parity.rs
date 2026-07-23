//! Parity fixtures for the domain surfaces retired onto the shared crates.
//!
//! Each case pins the server-visible behaviour of the shared parser/conventions
//! so the retirement stays reproducible after the old private modules are gone.
//! Where the shared parser (serde-saphyr, YAML 1.2 with 1.1-style booleans)
//! differs from the retired `serde_yml` engine, the pinned value is the shared
//! parser's — a deliberate, documented dialect adoption, not a regression.

use accelerator_visualiser::frontmatter::{self, FrontmatterState};
use corpus::DocTypeKey;

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

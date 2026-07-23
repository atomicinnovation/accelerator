use crate::slug::humanise_slug;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterState {
    Parsed(BTreeMap<String, serde_json::Value>),
    Absent,
    Malformed,
}

impl FrontmatterState {
    pub fn as_str(&self) -> &'static str {
        match self {
            FrontmatterState::Parsed(_) => "parsed",
            FrontmatterState::Absent => "absent",
            FrontmatterState::Malformed => "malformed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parsed {
    pub state: FrontmatterState,
    pub body: String,
}

pub fn parse(raw: &[u8]) -> Parsed {
    let parsed = corpus_adapters::parse(raw);
    let state = match parsed.state {
        corpus_adapters::FrontmatterState::Parsed(m) => {
            FrontmatterState::Parsed(mapping_to_json(&m))
        }
        corpus_adapters::FrontmatterState::Absent => FrontmatterState::Absent,
        corpus_adapters::FrontmatterState::Malformed => {
            FrontmatterState::Malformed
        }
    };
    Parsed {
        state,
        body: parsed.body,
    }
}

fn mapping_to_json(
    mapping: &corpus::value::Mapping,
) -> BTreeMap<String, serde_json::Value> {
    mapping
        .entries()
        .iter()
        .map(|(k, v)| (k.clone(), fv_to_json(v)))
        .collect()
}

fn fv_to_json(value: &corpus::value::FrontmatterValue) -> serde_json::Value {
    use corpus::value::{FrontmatterValue as F, Scalar};
    use serde_json::Value as J;
    match value {
        F::Scalar(Scalar::Null) => J::Null,
        F::Scalar(Scalar::Bool(b)) => J::Bool(*b),
        F::Scalar(Scalar::Int(i)) => J::Number((*i).into()),
        F::Scalar(Scalar::Float(f)) => {
            serde_json::Number::from_f64(*f).map_or(J::Null, J::Number)
        }
        F::Scalar(Scalar::String(s)) => J::String(s.clone()),
        F::Sequence(items) => J::Array(items.iter().map(fv_to_json).collect()),
        F::Mapping(m) => J::Object(
            m.entries()
                .iter()
                .map(|(k, v)| (k.clone(), fv_to_json(v)))
                .collect(),
        ),
    }
}

const PREVIEW_MAX_CHARS: usize = 200;

pub fn body_preview_from(body: &str) -> String {
    let mut buf = String::new();
    let mut char_count: usize = 0;
    'outer: for line in body.lines() {
        let trimmed = line.trim();
        let is_break = trimmed.is_empty() || trimmed.starts_with('#');
        if is_break {
            if !buf.is_empty() {
                break;
            }
            continue;
        }
        if !buf.is_empty() {
            buf.push(' ');
            char_count += 1;
        }
        let mut last_was_space = false;
        for ch in trimmed.chars() {
            if ch.is_whitespace() {
                if !last_was_space {
                    buf.push(' ');
                    char_count += 1;
                    last_was_space = true;
                }
            } else {
                buf.push(ch);
                char_count += 1;
                last_was_space = false;
            }
            // Collect one extra char past the limit so the truncation
            // branch below can distinguish "exactly 200" from "> 200".
            if char_count > PREVIEW_MAX_CHARS {
                break 'outer;
            }
        }
    }

    if char_count > PREVIEW_MAX_CHARS {
        let truncated: String = buf.chars().take(PREVIEW_MAX_CHARS).collect();
        format!("{truncated}…")
    } else {
        buf
    }
}

pub fn title_from(
    parsed: &FrontmatterState,
    body: &str,
    filename_stem: &str,
) -> String {
    // Cascade layer 1/3 — frontmatter.title (verbatim when non-blank;
    // whitespace-only is treated as absent so it can't render a blank <h1>).
    if let FrontmatterState::Parsed(m) = parsed {
        if let Some(v) = m.get("title") {
            if let Some(s) = v.as_str() {
                if !s.trim().is_empty() {
                    return s.trim().to_string(); // trim to match layer 2's H1
                }
            }
        }
    }
    // Cascade layer 2/3 — first H1 in the body.
    for line in body.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    // Cascade layer 3/3 — humanise_slug(stem): humanised fallback so no detail
    // page renders an unhumanised filename stem as its <h1>.
    humanise_slug(filename_stem)
}

/// Stringifies a frontmatter number as a cross-reference token. An
/// integer-valued number renders without a fractional part so a leading-zero id
/// that parsed as a float (e.g. `0007` -> `7.0`) still canonicalises as a bare
/// numeric id downstream rather than being dropped.
fn number_ref_string(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        i.to_string()
    } else if let Some(u) = n.as_u64() {
        u.to_string()
    } else if let Some(f) = n.as_f64() {
        if f.fract() == 0.0 && f.abs() < 1e15 {
            (f as i64).to_string()
        } else {
            n.to_string()
        }
    } else {
        n.to_string()
    }
}

/// Reads cross-reference keys from frontmatter for work-item aggregation.
///
/// Reads `work_item_id:` (preferred) or, failing that, the typed `target:`
/// (ADR-0034 `doc-type:id` form) as the cross-reference scalar, then
/// unconditionally aggregates `parent:` and `related:` (each a scalar or an
/// array). All non-empty string/numeric values are aggregated into a single
/// `Vec<String>`. Returns an empty Vec when no recognised key is present or
/// all values are empty.
pub fn read_ref_keys(parsed: &FrontmatterState) -> Vec<String> {
    let FrontmatterState::Parsed(m) = parsed else {
        return Vec::new();
    };
    let extract_scalar = |v: &serde_json::Value| -> Option<String> {
        match v {
            serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
            serde_json::Value::Number(n) => Some(number_ref_string(n)),
            _ => None,
        }
    };
    let extract_values = |v: &serde_json::Value| -> Vec<String> {
        match v {
            serde_json::Value::Array(arr) => {
                arr.iter().filter_map(&extract_scalar).collect()
            }
            other => extract_scalar(other).into_iter().collect(),
        }
    };

    let mut refs: Vec<String> = Vec::new();

    // `work_item_id:` is preferred; the typed `target:` is the fallback
    // (scalar only).
    if let Some(v) = m.get("work_item_id") {
        if let Some(s) = extract_scalar(v) {
            refs.push(s);
        }
    } else if let Some(v) = m.get("target") {
        // Final fallback per ADR-0034 §"Forms": typed-linkage `doc-type:id`
        // form. Currently only `work-item:` is extracted here; other
        // prefixes are not aggregated into work-item refs.
        if let Some(s) = extract_scalar(v) {
            if let Some(crate::typed_ref::TypedRef::WorkItem(id)) =
                crate::typed_ref::parse_typed_ref(&s)
            {
                refs.push(id);
            }
        }
    }

    // `parent:` and `related:` always aggregate (scalar or array).
    if let Some(v) = m.get("parent") {
        refs.extend(extract_values(v));
    }
    if let Some(v) = m.get("related") {
        refs.extend(extract_values(v));
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    #[test]
    fn parsed_extracts_mapping_and_body() {
        let raw = b("---\ntitle: Foo\nstatus: done\n---\n# Body\n\ntext\n");
        let p = parse(&raw);
        match p.state {
            FrontmatterState::Parsed(m) => {
                assert_eq!(
                    m.get("title").and_then(|v| v.as_str()),
                    Some("Foo")
                );
                assert_eq!(
                    m.get("status").and_then(|v| v.as_str()),
                    Some("done")
                );
            }
            other => panic!("expected Parsed, got {other:?}"),
        }
        assert!(p.body.starts_with("# Body"));
    }

    #[test]
    fn absent_when_no_leading_fence() {
        let raw = b("# Notes about subagents\n\nSome content.\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Absent));
        assert!(p.body.starts_with("# Notes"));
    }

    #[test]
    fn malformed_when_yaml_fails() {
        let raw = b("---\ntitle: \"unclosed\nstatus: done\n---\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Malformed));
    }

    #[test]
    fn quoted_scalar_with_trailing_whitespace_parses() {
        // The old libyml-backed engine panicked on a quoted scalar ending in a
        // run of trailing whitespace ("String join would overflow memory
        // bounds"), so such a note surfaced as Malformed. The shared parser has
        // no such bug: the note parses and renders (regression for a migrated
        // note whose H1-derived title carried ~34 trailing spaces).
        let title = format!("Security Lens{}", " ".repeat(34));
        let raw = b(&format!(
            "---\ntitle: \"{title}\"\nstatus: done\n---\nbody\n"
        ));
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Parsed(_)));
        assert_eq!(p.body, "body\n");
    }

    #[test]
    fn malformed_when_no_closing_fence() {
        let raw = b("---\ntitle: foo\nstatus: done\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Malformed));
    }

    #[test]
    fn malformed_when_yaml_root_is_not_mapping() {
        let raw = b("---\n- a\n- b\n---\nbody\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Malformed));
    }

    #[test]
    fn empty_frontmatter_parses_as_empty_mapping() {
        let raw = b("---\n---\n# Heading\n");
        let p = parse(&raw);
        match p.state {
            FrontmatterState::Parsed(m) => assert!(m.is_empty()),
            other => panic!("expected empty Parsed, got {other:?}"),
        }
    }

    #[test]
    fn title_cascade_prefers_frontmatter() {
        let raw = b("---\ntitle: From FM\n---\n# H1 Body\n");
        let p = parse(&raw);
        let t = title_from(&p.state, &p.body, "fallback");
        assert_eq!(t, "From FM");
    }

    #[test]
    fn title_cascade_falls_back_to_first_h1() {
        let raw = b("---\nstatus: done\n---\n# From H1\n# Second\n");
        let p = parse(&raw);
        let t = title_from(&p.state, &p.body, "fallback");
        assert_eq!(t, "From H1");
    }

    #[test]
    fn title_cascade_falls_back_to_filename_stem() {
        let raw = b("body without h1\n");
        let p = parse(&raw);
        let t = title_from(&p.state, &p.body, "2026-04-18-my-doc");
        assert_eq!(t, "My Doc"); // humanised (date prefix stripped), not the raw stem
    }

    #[test]
    fn title_cascade_humanises_stem_for_every_doc_kind() {
        use crate::docs::DocTypeKey;

        // No frontmatter.title and no first H1 → layer 3 for every kind.
        let raw = b("body without h1\n");
        let p = parse(&raw);
        let stem = "0042-test-fixture";

        // Forward-looking invariant: title_from is kind-agnostic today, so this
        // loop runs the same path 13 times. We assert only that no kind renders
        // the raw stem — comparing against humanise_slug(stem) would be a
        // tautology (production calls the same helper). It guards against a
        // future per-kind refactor regressing any kind back to an unhumanised
        // stem.
        for kind in DocTypeKey::all() {
            let t = title_from(&p.state, &p.body, stem);
            assert_ne!(t, stem, "kind={kind:?} must not render the raw stem");
        }

        // Concrete-literal oracle: gives the test real value-level bite,
        // guarding against a shared bug in the test oracle and the production
        // path both computing the same wrong value.
        assert_eq!(title_from(&p.state, &p.body, stem), "Test Fixture");
    }

    #[test]
    fn title_cascade_blank_frontmatter_title_falls_through_to_humanised_stem() {
        // Drive the layer-1 guard directly at the title_from boundary by
        // building FrontmatterState::Parsed ourselves. We do NOT round-trip
        // through parse(): a quoted whitespace-only YAML scalar can trip
        // libyml's trailing-whitespace panic and surface as Malformed (see
        // `malformed_when_quoted_scalar_has_trailing_whitespace`), which would
        // skip the Parsed arm entirely and make this test pass for the wrong
        // reason — green even if the `!s.trim().is_empty()` guard were reverted.
        for blank in ["", "   "] {
            let mut m = std::collections::BTreeMap::new();
            m.insert(
                "title".to_string(),
                serde_json::Value::String(blank.to_string()),
            );
            let state = FrontmatterState::Parsed(m);
            let t =
                title_from(&state, "body without h1\n", "0042-test-fixture");
            assert_eq!(t, "Test Fixture", "blank title {blank:?}");
        }
    }

    #[test]
    fn read_ref_keys_returns_empty_when_neither_key_present() {
        let raw = b("---\ntitle: foo\n---\nbody\n");
        let p = parse(&raw);
        assert!(read_ref_keys(&p.state).is_empty());
    }

    #[test]
    fn read_ref_keys_reads_work_item_id_key() {
        let raw = b("---\nwork_item_id: \"0042\"\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["0042".to_string()]);
    }

    #[test]
    fn read_ref_keys_extracts_work_item_id_from_target_when_no_alias() {
        let raw = b("---\ntype: work-item-review\ntarget: \"work-item:0042\"\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["0042".to_string()]);
    }

    #[test]
    fn read_ref_keys_prefers_work_item_id_alias_over_target() {
        let raw = b("---\ntype: work-item-review\nwork_item_id: \"0007\"\ntarget: \"work-item:0042\"\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["0007".to_string()]);
    }

    #[test]
    fn read_ref_keys_target_and_alias_same_value_no_double_counting() {
        let raw = b("---\nwork_item_id: \"0042\"\ntarget: \"work-item:0042\"\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["0042".to_string()]);
    }

    #[test]
    fn read_ref_keys_ignores_non_work_item_target_prefix() {
        let raw = b("---\ntype: plan-review\ntarget: \"plan:2026-01-01-foo\"\n---\nbody\n");
        let p = parse(&raw);
        assert!(read_ref_keys(&p.state).is_empty());
    }

    #[test]
    fn read_ref_keys_ignores_pr_target_prefix() {
        let raw = b("---\ntype: pr-review\ntarget: \"pr:123\"\n---\nbody\n");
        let p = parse(&raw);
        assert!(read_ref_keys(&p.state).is_empty());
    }

    #[test]
    fn read_ref_keys_empty_work_item_prefix_id_returns_empty() {
        let raw = b("---\ntarget: \"work-item:\"\n---\nbody\n");
        let p = parse(&raw);
        assert!(read_ref_keys(&p.state).is_empty());
    }

    #[test]
    fn read_ref_keys_absent_frontmatter_returns_empty() {
        let raw = b("no frontmatter here\n");
        let p = parse(&raw);
        assert!(read_ref_keys(&p.state).is_empty());
    }

    #[test]
    fn read_ref_keys_reads_parent_and_related() {
        // Unquoted leading-zero ids parse as numbers, so read_ref_keys yields
        // the numeric string (`0007` -> `7`); canonicalise_refs zero-pads them
        // back to the canonical width downstream, so resolution is unchanged.
        let raw = b("---\nparent: 0007\nrelated: [0001, 0002]\n---\nbody\n");
        let p = parse(&raw);
        let refs = read_ref_keys(&p.state);
        assert_eq!(refs.len(), 3);
        assert!(refs.contains(&"7".to_string()));
        assert!(refs.contains(&"1".to_string()));
        assert!(refs.contains(&"2".to_string()));
    }

    #[test]
    fn read_ref_keys_handles_scalar_related_as_single_element() {
        let raw = b("---\nrelated: \"0007\"\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["0007".to_string()]);
    }

    #[test]
    fn read_ref_keys_handles_array_parent_as_multi_element() {
        let raw = b("---\nparent: [0001, 0002]\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state).len(), 2);
    }

    #[test]
    fn read_ref_keys_handles_null_and_empty_string_as_empty() {
        for raw in [
            b("---\nparent: null\n---\n"),
            b("---\nrelated: null\n---\n"),
            b("---\nrelated: \"\"\n---\n"),
            b("---\n---\n"),
        ] {
            let p = parse(&raw);
            assert!(read_ref_keys(&p.state).is_empty(), "body={raw:?}");
        }
    }

    #[test]
    fn read_ref_keys_handles_int_and_string_as_equivalent_raw() {
        let int_raw = b("---\nparent: 42\n---\nbody\n");
        let str_raw = b("---\nparent: \"42\"\n---\nbody\n");
        let int_p = parse(&int_raw);
        let str_p = parse(&str_raw);
        assert_eq!(read_ref_keys(&int_p.state), read_ref_keys(&str_p.state));
    }

    #[test]
    fn read_ref_keys_aggregates_work_item_id_and_parent_and_related() {
        // Quoted ids keep their leading zeros; unquoted leading-zero ids parse
        // as numbers (`0007` -> `7`, `0011` -> `11`) and are re-padded by
        // canonicalise_refs downstream.
        let raw = b("---\nwork_item_id: \"0042\"\nparent: 0007\nrelated: [0011]\n---\nbody\n");
        let p = parse(&raw);
        let refs = read_ref_keys(&p.state);
        assert_eq!(refs.len(), 3);
        assert!(refs.contains(&"0042".to_string()));
        assert!(refs.contains(&"7".to_string()));
        assert!(refs.contains(&"11".to_string()));
    }

    #[test]
    fn block_sequences_survive_round_trip_to_json() {
        let raw = b("---\ntags:\n  - foo\n  - bar\n---\n");
        let p = parse(&raw);
        match p.state {
            FrontmatterState::Parsed(m) => {
                let tags = m.get("tags").unwrap();
                let arr = tags.as_array().unwrap();
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0].as_str(), Some("foo"));
                assert_eq!(arr[1].as_str(), Some("bar"));
            }
            other => panic!("expected Parsed, got {other:?}"),
        }
    }

    #[test]
    fn windows_crlf_line_endings_are_tolerated() {
        let raw = b("---\r\ntitle: Foo\r\n---\r\nbody\r\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Parsed(_)));
    }

    #[test]
    fn invalid_utf8_falls_back_to_lossy_decode() {
        let mut raw = b"---\ntitle: Foo\n---\nbody\n".to_vec();
        raw.push(0xff);
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Parsed(_)));
    }
}

#[cfg(test)]
mod body_preview_tests {
    use super::body_preview_from;

    #[test]
    fn empty_body_returns_empty_string() {
        assert_eq!(body_preview_from(""), "");
        assert_eq!(body_preview_from("   \n\n   "), "");
    }

    #[test]
    fn skips_leading_h1_to_avoid_duplicating_title() {
        let body = "# The Foo Plan\n\nThis is the body of the plan.\n";
        assert_eq!(body_preview_from(body), "This is the body of the plan.");
    }

    #[test]
    fn takes_first_non_heading_paragraph() {
        let body = "## Section\n\nFirst sentence here.\n\n## Next\n\nSecond.\n";
        assert_eq!(body_preview_from(body), "First sentence here.");
    }

    #[test]
    fn collapses_internal_whitespace() {
        let body = "Line one.\nLine two.\n\tLine three.\n";
        assert_eq!(body_preview_from(body), "Line one. Line two. Line three.");
    }

    #[test]
    fn truncates_with_ellipsis_at_200_chars() {
        let long = "abcdefghij".repeat(30); // 300 chars
        let preview = body_preview_from(&long);
        assert_eq!(preview.chars().count(), 201);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn truncation_respects_utf8_boundaries() {
        let body = "é".repeat(300);
        let preview = body_preview_from(&body);
        assert!(std::str::from_utf8(preview.as_bytes()).is_ok());
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn body_with_only_headings_returns_empty() {
        let body = "# H1\n## H2\n### H3\n";
        assert_eq!(body_preview_from(body), "");
    }

    #[test]
    fn heading_after_content_terminates_the_preview() {
        let body = "First para.\n## Heading\nMore text.\n";
        assert_eq!(body_preview_from(body), "First para.");
    }

    #[test]
    fn body_of_exactly_200_chars_is_not_truncated() {
        let exact = "a".repeat(200);
        let preview = body_preview_from(&exact);
        assert_eq!(preview.chars().count(), 200);
        assert!(!preview.ends_with('…'));
    }

    #[test]
    fn body_of_201_chars_truncates_with_ellipsis() {
        let just_over = "a".repeat(201);
        let preview = body_preview_from(&just_over);
        assert_eq!(preview.chars().count(), 201); // 200 + '…'
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn joins_multi_line_first_paragraph_with_single_spaces() {
        let body = "Line one.\nLine two.\nLine three.\n\nNext paragraph.\n";
        assert_eq!(body_preview_from(body), "Line one. Line two. Line three.");
    }
}

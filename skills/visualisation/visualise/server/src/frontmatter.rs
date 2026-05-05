use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
pub enum FenceError {
    Malformed,
}

/// Returns the byte range `(yaml_start, body_start)` of the YAML content
/// region inside `---` fences, or `None` when no frontmatter is present
/// (file doesn't start with `---\n` or `---\r\n`).
///
/// Returns `Err(Malformed)` when the opening fence is found but no closing
/// fence exists within the 1 MiB scan window.
///
/// Works on raw bytes — invalid UTF-8 in the body does not affect fence
/// detection because `---` is pure ASCII.
///
/// `yaml_start`: byte offset of the first character after the opening fence line.
/// `body_start`: byte offset of the first character after the closing fence line.
pub fn fence_offsets(raw: &[u8]) -> Result<Option<(usize, usize)>, FenceError> {
    // Find the end of the first line.
    let first_lf = match raw.iter().position(|&b| b == b'\n') {
        Some(p) => p,
        None => {
            // Single line with no newline: can't be valid frontmatter.
            return Ok(None);
        }
    };
    // Strip optional CRLF.
    let first_line_end = if first_lf > 0 && raw[first_lf - 1] == b'\r' {
        first_lf - 1
    } else {
        first_lf
    };
    if &raw[..first_line_end] != b"---" {
        return Ok(None);
    }

    const MAX_SCAN: usize = 1 << 20;
    let scan_end = raw.len().min(MAX_SCAN);
    let yaml_start = first_lf + 1;
    if yaml_start >= raw.len() {
        return Err(FenceError::Malformed);
    }

    let mut pos = yaml_start;
    while pos < scan_end {
        let line_lf = raw[pos..scan_end]
            .iter()
            .position(|&b| b == b'\n')
            .map(|n| pos + n);
        let line_lf = match line_lf {
            Some(p) => p,
            None => break, // no newline found before scan_end
        };
        let line_end = if line_lf > pos && raw[line_lf - 1] == b'\r' {
            line_lf - 1
        } else {
            line_lf
        };
        if &raw[pos..line_end] == b"---" {
            let body_start = if line_lf < raw.len() {
                line_lf + 1
            } else {
                raw.len()
            };
            return Ok(Some((yaml_start, body_start)));
        }
        pos = line_lf + 1;
    }

    Err(FenceError::Malformed)
}

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
    let s = match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(raw).into_owned(),
    };

    let (yaml_start, body_start) = match fence_offsets(raw) {
        Ok(None) => {
            return Parsed {
                state: FrontmatterState::Absent,
                body: s,
            }
        }
        Err(FenceError::Malformed) => {
            return Parsed {
                state: FrontmatterState::Malformed,
                body: s,
            }
        }
        Ok(Some(offsets)) => offsets,
    };

    // The YAML content lives between yaml_start and the closing fence line.
    // body_start points to the first byte after the closing `---\n`.
    // We need the YAML string: everything from yaml_start up to (but not
    // including) the closing `---` line. Since body_start is right after
    // the closing fence newline, we find the closing fence by searching
    // backwards from body_start in the raw bytes.
    let yaml_region = &s[yaml_start..body_start];
    // Strip the closing `---` line (and its preceding newline) from the end.
    let yaml_src = if let Some(close_pos) = yaml_region.rfind("---") {
        yaml_region[..close_pos]
            .trim_end_matches('\r')
            .trim_end_matches('\n')
    } else {
        yaml_region.trim_end_matches('\r').trim_end_matches('\n')
    };
    let body = s[body_start..].trim_start_matches('\n').to_string();

    let value: serde_yml::Value = match serde_yml::from_str(yaml_src) {
        Ok(v) => v,
        Err(_) => {
            return Parsed {
                state: FrontmatterState::Malformed,
                body,
            }
        }
    };

    let mapping = match value {
        serde_yml::Value::Mapping(m) => m,
        serde_yml::Value::Null => serde_yml::Mapping::new(),
        _ => {
            return Parsed {
                state: FrontmatterState::Malformed,
                body,
            }
        }
    };

    let mut out: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for (k, v) in mapping {
        let key = match k {
            serde_yml::Value::String(s) => s,
            other => match serde_yml::to_string(&other) {
                Ok(s) => s.trim().to_string(),
                Err(_) => {
                    return Parsed {
                        state: FrontmatterState::Malformed,
                        body,
                    }
                }
            },
        };
        let json_val = match yml_to_json(&v) {
            Some(v) => v,
            None => {
                return Parsed {
                    state: FrontmatterState::Malformed,
                    body,
                }
            }
        };
        out.insert(key, json_val);
    }

    Parsed {
        state: FrontmatterState::Parsed(out),
        body,
    }
}

fn yml_to_json(v: &serde_yml::Value) -> Option<serde_json::Value> {
    use serde_json::Value as J;
    Some(match v {
        serde_yml::Value::Null => J::Null,
        serde_yml::Value::Bool(b) => J::Bool(*b),
        serde_yml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                J::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                J::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(J::Number)
                    .unwrap_or(J::Null)
            } else {
                J::Null
            }
        }
        serde_yml::Value::String(s) => J::String(s.clone()),
        serde_yml::Value::Sequence(items) => {
            let mut arr = Vec::with_capacity(items.len());
            for item in items {
                arr.push(yml_to_json(item)?);
            }
            J::Array(arr)
        }
        serde_yml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yml::Value::String(s) => s.clone(),
                    other => serde_yml::to_string(other).ok()?.trim().to_string(),
                };
                obj.insert(key, yml_to_json(v)?);
            }
            J::Object(obj)
        }
        serde_yml::Value::Tagged(_) => return None,
    })
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

pub fn title_from(parsed: &FrontmatterState, body: &str, filename_stem: &str) -> String {
    if let FrontmatterState::Parsed(m) = parsed {
        if let Some(v) = m.get("title") {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
    }
    for line in body.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    filename_stem.to_string()
}

/// Reads cross-reference keys from frontmatter for work-item aggregation.
///
/// Reads `work-item:` (preferred) or `ticket:` (legacy fallback) as a scalar,
/// plus `parent:` and `related:` which may each be a scalar or an array.
/// All non-empty string/numeric values are aggregated into a single `Vec<String>`.
/// Returns an empty Vec when no recognised key is present or all values are empty.
pub fn read_ref_keys(parsed: &FrontmatterState) -> Vec<String> {
    let FrontmatterState::Parsed(m) = parsed else {
        return Vec::new();
    };
    let extract_scalar = |v: &serde_json::Value| -> Option<String> {
        match v {
            serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    };
    let extract_values = |v: &serde_json::Value| -> Vec<String> {
        match v {
            serde_json::Value::Array(arr) => {
                arr.iter().filter_map(|item| extract_scalar(item)).collect()
            }
            other => extract_scalar(other).into_iter().collect(),
        }
    };

    let mut refs: Vec<String> = Vec::new();

    // `work-item:` wins over legacy `ticket:` when both are present (scalar only).
    if let Some(v) = m.get("work-item") {
        if let Some(s) = extract_scalar(v) {
            refs.push(s);
        }
    } else if let Some(v) = m.get("ticket") {
        if let Some(s) = extract_scalar(v) {
            refs.push(s);
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
                assert_eq!(m.get("title").and_then(|v| v.as_str()), Some("Foo"));
                assert_eq!(m.get("status").and_then(|v| v.as_str()), Some("done"));
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
        assert_eq!(t, "2026-04-18-my-doc");
    }

    #[test]
    fn read_ref_keys_returns_empty_when_neither_key_present() {
        let raw = b("---\ntitle: foo\n---\nbody\n");
        let p = parse(&raw);
        assert!(read_ref_keys(&p.state).is_empty());
    }

    #[test]
    fn read_ref_keys_reads_work_item_key() {
        let raw = b("---\nwork-item: \"0042\"\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["0042".to_string()]);
    }

    #[test]
    fn read_ref_keys_reads_legacy_ticket_key() {
        let raw = b("---\nticket: 1478\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["1478".to_string()]);
    }

    #[test]
    fn read_ref_keys_with_both_legacy_and_current_keys_prefers_current() {
        let raw = b("---\nwork-item: \"0007\"\nticket: 0042\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["0007".to_string()]);
    }

    #[test]
    fn read_ref_keys_absent_frontmatter_returns_empty() {
        let raw = b("no frontmatter here\n");
        let p = parse(&raw);
        assert!(read_ref_keys(&p.state).is_empty());
    }

    #[test]
    fn read_ref_keys_numeric_ticket_value_is_stringified() {
        let raw = b("---\nticket: 1478\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(read_ref_keys(&p.state), vec!["1478".to_string()]);
    }

    #[test]
    fn read_ref_keys_reads_parent_and_related() {
        let raw = b("---\nparent: 0007\nrelated: [0001, 0002]\n---\nbody\n");
        let p = parse(&raw);
        let refs = read_ref_keys(&p.state);
        assert_eq!(refs.len(), 3);
        assert!(refs.contains(&"0007".to_string()));
        assert!(refs.contains(&"0001".to_string()));
        assert!(refs.contains(&"0002".to_string()));
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
    fn read_ref_keys_aggregates_work_item_and_parent_and_related() {
        let raw = b("---\nwork-item: \"0042\"\nparent: 0007\nrelated: [0011]\n---\nbody\n");
        let p = parse(&raw);
        let refs = read_ref_keys(&p.state);
        assert_eq!(refs.len(), 3);
        assert!(refs.contains(&"0042".to_string()));
        assert!(refs.contains(&"0007".to_string()));
        assert!(refs.contains(&"0011".to_string()));
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

#[cfg(test)]
mod fence_offsets_tests {
    use super::{fence_offsets, FenceError};

    #[test]
    fn returns_none_when_no_leading_fence() {
        assert_eq!(fence_offsets(b"# Heading\n"), Ok(None));
    }

    #[test]
    fn returns_none_for_empty_input() {
        assert_eq!(fence_offsets(b""), Ok(None));
    }

    #[test]
    fn returns_offsets_for_simple_frontmatter() {
        // "---\nstatus: todo\n---\nbody\n"
        let raw = b"---\nstatus: todo\n---\nbody\n";
        let r = fence_offsets(raw).unwrap().unwrap();
        assert_eq!(r.0, 4); // yaml_start: after "---\n"
        assert_eq!(r.1, 21); // body_start: after closing "---\n"
        assert_eq!(&raw[r.0..r.1], b"status: todo\n---\n");
    }

    #[test]
    fn returns_malformed_when_no_closing_fence() {
        assert_eq!(
            fence_offsets(b"---\ntitle: foo\n"),
            Err(FenceError::Malformed)
        );
    }

    #[test]
    fn returns_malformed_when_only_opening_fence_and_no_content() {
        assert_eq!(fence_offsets(b"---\n"), Err(FenceError::Malformed));
    }

    #[test]
    fn handles_crlf_fences() {
        let raw = b"---\r\ntitle: Foo\r\n---\r\nbody\r\n";
        let r = fence_offsets(raw).unwrap().unwrap();
        assert_eq!(r.0, 5); // yaml_start: after "---\r\n"
        assert_eq!(&raw[r.0..r.0 + 10], b"title: Foo");
    }

    #[test]
    fn body_start_is_after_closing_fence_newline() {
        let raw = b"---\ntitle: foo\n---\nbody starts here\n";
        let (_, body_start) = fence_offsets(raw).unwrap().unwrap();
        assert_eq!(&raw[body_start..], b"body starts here\n");
    }

    #[test]
    fn empty_frontmatter_returns_offsets() {
        // "---\n---\n"
        let raw = b"---\n---\nbody\n";
        let r = fence_offsets(raw).unwrap().unwrap();
        assert_eq!(r.0, 4); // yaml_start
        assert_eq!(r.1, 8); // body_start (after second ---\n)
    }
}

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
    let s = match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(raw).into_owned(),
    };

    let first_line_end = s.find('\n').unwrap_or(s.len());
    let first_line = s[..first_line_end].trim_end_matches('\r');
    if first_line != "---" {
        return Parsed { state: FrontmatterState::Absent, body: s };
    }

    const MAX_SCAN: usize = 1 << 20;
    let scan_end = s.len().min(MAX_SCAN);
    let after_first = first_line_end + 1;
    if after_first >= s.len() {
        return Parsed { state: FrontmatterState::Malformed, body: s };
    }

    let mut close_at: Option<usize> = None;
    let mut pos = after_first;
    while pos < scan_end {
        let line_end = s[pos..].find('\n').map(|n| pos + n).unwrap_or(s.len());
        let line = s[pos..line_end].trim_end_matches('\r');
        if line == "---" {
            close_at = Some(line_end);
            break;
        }
        pos = line_end + 1;
    }

    let close = match close_at {
        Some(c) => c,
        None => return Parsed { state: FrontmatterState::Malformed, body: s },
    };

    let yaml_start = first_line_end + 1;
    let yaml_end = s[..close]
        .rfind('\n')
        .map(|n| n + 1)
        .unwrap_or(yaml_start);
    let yaml_src = &s[yaml_start..yaml_end.saturating_sub(1).max(yaml_start)];
    let body_start = (close + 1).min(s.len());
    let body = s[body_start..].trim_start_matches('\n').to_string();

    let value: serde_yml::Value = match serde_yml::from_str(yaml_src) {
        Ok(v) => v,
        Err(_) => return Parsed { state: FrontmatterState::Malformed, body },
    };

    let mapping = match value {
        serde_yml::Value::Mapping(m) => m,
        serde_yml::Value::Null => serde_yml::Mapping::new(),
        _ => return Parsed { state: FrontmatterState::Malformed, body },
    };

    let mut out: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for (k, v) in mapping {
        let key = match k {
            serde_yml::Value::String(s) => s,
            other => match serde_yml::to_string(&other) {
                Ok(s) => s.trim().to_string(),
                Err(_) => return Parsed { state: FrontmatterState::Malformed, body },
            },
        };
        let json_val = match yml_to_json(&v) {
            Some(v) => v,
            None => return Parsed { state: FrontmatterState::Malformed, body },
        };
        out.insert(key, json_val);
    }

    Parsed { state: FrontmatterState::Parsed(out), body }
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
                serde_json::Number::from_f64(f).map(J::Number).unwrap_or(J::Null)
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

pub fn ticket_of(parsed: &FrontmatterState) -> Option<String> {
    match parsed {
        FrontmatterState::Parsed(m) => match m.get("ticket") {
            Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
            Some(serde_json::Value::Number(n)) => Some(n.to_string()),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(s: &str) -> Vec<u8> { s.as_bytes().to_vec() }

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
    fn ticket_of_absent_value_returns_none() {
        let raw = b("---\nticket:\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(ticket_of(&p.state), None);
    }

    #[test]
    fn ticket_of_null_returns_none() {
        let raw = b("---\nticket: null\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(ticket_of(&p.state), None);
    }

    #[test]
    fn ticket_of_empty_string_returns_none() {
        let raw = b("---\nticket: \"\"\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(ticket_of(&p.state), None);
    }

    #[test]
    fn ticket_of_numeric_value_is_stringified() {
        let raw = b("---\nticket: 1478\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(ticket_of(&p.state).as_deref(), Some("1478"));
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

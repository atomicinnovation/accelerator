//! Inject-or-replace-or-append-if-absent linkage merge. A single key is
//! always replaced outright; a list key gains `value` only if not already
//! a member, preserving existing member order.

use crate::migrations::m0007::quote::is_strict_fence;
use crate::migrations::m0007::schema::Cardinality;

/// Merges `value` into `key`, injecting it just before the closing fence
/// when the key is absent. Idempotent: re-merging an already-present value
/// reproduces identical bytes.
#[must_use]
pub fn merge(
    content: &str,
    key: &str,
    value: &str,
    cardinality: Cardinality,
) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_fm = false;
    let mut seen_open = false;
    let mut done = false;

    for line in content.lines() {
        if !seen_open && is_strict_fence(line) {
            seen_open = true;
            in_fm = true;
            out.push(line.to_owned());
            continue;
        }
        if in_fm && is_strict_fence(line) {
            if !done {
                out.push(rendered_line(key, value, cardinality, &[]));
            }
            in_fm = false;
            out.push(line.to_owned());
            continue;
        }
        if in_fm && line.starts_with(&format!("{key}:")) {
            done = true;
            match cardinality {
                Cardinality::Single => {
                    out.push(rendered_line(key, value, cardinality, &[]));
                }
                Cardinality::List => {
                    let members = parse_members(&line[key.len() + 1..]);
                    out.push(rendered_line(key, value, cardinality, &members));
                }
            }
            continue;
        }
        out.push(line.to_owned());
    }

    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn rendered_line(
    key: &str,
    value: &str,
    cardinality: Cardinality,
    existing_members: &[String],
) -> String {
    match cardinality {
        Cardinality::Single => format!("{key}: \"{value}\""),
        Cardinality::List => {
            let mut members = existing_members.to_vec();
            if !members.iter().any(|member| member == value) {
                members.push(value.to_owned());
            }
            let rendered = members
                .iter()
                .map(|member| format!("\"{member}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{key}: [{rendered}]")
        }
    }
}

fn parse_members(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    let raw = raw.strip_prefix('[').unwrap_or(raw);
    let raw = raw.strip_suffix(']').unwrap_or(raw);
    raw.split(',')
        .map(|element| {
            element
                .trim_matches(|c: char| c == ' ' || c == '\t' || c == '"')
                .to_owned()
        })
        .filter(|element| !element.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::merge;
    use crate::migrations::m0007::schema::Cardinality;

    #[test]
    fn injects_a_single_key_absent_from_the_frontmatter() {
        let content = "---\ntitle: t\n---\nbody\n";
        let result = merge(content, "target", "plan:0042", Cardinality::Single);
        assert_eq!(result, "---\ntitle: t\ntarget: \"plan:0042\"\n---\nbody\n");
    }

    #[test]
    fn replaces_an_existing_single_key() {
        let content = "---\ntarget: \"plan:0001\"\n---\nbody\n";
        let result = merge(content, "target", "plan:0042", Cardinality::Single);
        assert_eq!(result, "---\ntarget: \"plan:0042\"\n---\nbody\n");
    }

    #[test]
    fn injects_a_list_key_absent_from_the_frontmatter() {
        let content = "---\ntitle: t\n---\nbody\n";
        let result =
            merge(content, "blocks", "work-item:0042", Cardinality::List);
        assert_eq!(
            result,
            "---\ntitle: t\nblocks: [\"work-item:0042\"]\n---\nbody\n"
        );
    }

    #[test]
    fn appends_to_an_existing_list_preserving_order() {
        let content = "---\nblocks: [\"work-item:0001\"]\n---\nbody\n";
        let result =
            merge(content, "blocks", "work-item:0042", Cardinality::List);
        assert_eq!(
            result,
            "---\nblocks: [\"work-item:0001\", \"work-item:0042\"]\n---\nbody\n"
        );
    }

    #[test]
    fn a_member_already_present_is_not_duplicated() {
        let content = "---\nblocks: [\"work-item:0042\"]\n---\nbody\n";
        let result =
            merge(content, "blocks", "work-item:0042", Cardinality::List);
        assert_eq!(result, content);
    }
}

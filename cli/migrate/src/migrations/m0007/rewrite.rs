//! The single-pass frontmatter rewrite: a line-oriented rewrite, not a
//! structural YAML rewrite, so a migration's output stays byte-for-byte
//! equal to the captured golden fixtures wherever nothing changed. Only
//! the frontmatter fence region is rewritten; the body passes through
//! verbatim. Presumes `content` already carries a strict leading fence —
//! the caller routes fence-less files to the backfill instead.
//!
//! Every `has_*` presence flag (`has_title`, `has_date`, …) is a snapshot
//! of the file's *original* state, taken once before the rewrite begins,
//! never scan-local state. Only what *this pass itself decides to emit*
//! (`emitted_id`, `emitted_revision`, `emitted_title`, the seeded
//! `date`/`author` values) is tracked while scanning.

use std::path::Path;
use std::path::PathBuf;

use corpus::doc_type::DocTypeKey;
use corpus::frontmatter_validation::parse_entries;
use corpus::frontmatter_validation::raw_value;
use corpus::frontmatter_validation::schema as fm_schema;

use crate::migrations::m0007::fence::body_text;
use crate::migrations::m0007::fence::frontmatter_text;
use crate::migrations::m0007::linkage_value;
use crate::migrations::m0007::quote::is_empty_value;
use crate::migrations::m0007::quote::is_strict_fence;
use crate::migrations::m0007::quote::normalise_value;
use crate::migrations::m0007::quote::refuses;
use crate::migrations::m0007::quote::semantic_inner;
use crate::migrations::m0007::schema::canonical_type;
use crate::migrations::m0007::schema::is_linkage_key;
use crate::migrations::m0007::schema::own_id_key_for_type;
use crate::migrations::m0007::status_map::map_status;

/// The identity stem for a file, type-aware: a design-inventory's stem is
/// its parent directory (the manifest itself is always `inventory.md`);
/// every other type is its own filename stem.
#[must_use]
pub fn derive_stem(relpath: &str, linkage_type: &str) -> String {
    if linkage_type == "design-inventory" {
        Path::new(relpath)
            .parent()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        Path::new(relpath)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

/// The digits of a genuine `pr-`/`PR-` segment (start-of-stem or
/// hyphen-preceded), else a leading number on a stem that is not
/// date-prefixed. `None` when neither applies.
#[must_use]
pub fn pr_number_of(stem: &str) -> Option<String> {
    let bytes = stem.as_bytes();
    for index in 0..bytes.len() {
        let at_segment_start = index == 0 || bytes[index - 1] == b'-';
        if !at_segment_start {
            continue;
        }
        let rest = &stem[index..];
        let Some(after_pr) = rest
            .strip_prefix("PR")
            .or_else(|| rest.strip_prefix("Pr"))
            .or_else(|| rest.strip_prefix("pR"))
            .or_else(|| rest.strip_prefix("pr"))
        else {
            continue;
        };
        let after_dash = after_pr.strip_prefix('-').unwrap_or(after_pr);
        let digits: String = after_dash
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if !digits.is_empty() {
            return Some(digits);
        }
    }
    if is_date_prefixed(stem) {
        return None;
    }
    let leading: String =
        stem.chars().take_while(char::is_ascii_digit).collect();
    (!leading.is_empty()).then_some(leading)
}

fn is_date_prefixed(stem: &str) -> bool {
    let bytes = stem.as_bytes();
    bytes.len() >= 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

/// The document's identity, type-aware: a pr-description is identified by
/// its PR number (falling back to its stem when none is derivable); every
/// other type is identified by its stem.
#[must_use]
pub fn derive_id(linkage_type: &str, stem: &str) -> String {
    if linkage_type == "pr-description" {
        pr_number_of(stem).unwrap_or_else(|| stem.to_owned())
    } else {
        stem.to_owned()
    }
}

/// The resolved type for a file: its own non-empty `type:` value, else path
/// inference — both passed through the legacy-alias fold.
#[must_use]
pub fn resolve_type(
    content: &str,
    relpath: &str,
    table: &[(DocTypeKey, PathBuf)],
) -> Option<&'static str> {
    let own = get(content, "type")
        .as_deref()
        .map(semantic_inner)
        .map(str::to_owned)
        .filter(|value| !value.is_empty());
    let resolved = own
        .as_deref()
        .or_else(|| corpus::linkage::type_from_path(relpath, table))?;
    Some(fm_schema::row_for(canonical_type(resolved))?.linkage_type)
}

fn get(content: &str, key: &str) -> Option<String> {
    frontmatter_text(content)
        .map(parse_entries)
        .and_then(|entries| raw_value(&entries, key).map(ToOwned::to_owned))
}

fn first_body_h1(content: &str) -> Option<String> {
    let body = body_text(content)?;
    let line = body.lines().find(|line| line.starts_with("# "))?;
    Some(line[2..].trim_end().replace('"', ""))
}

fn presence(content: &str, key: &str) -> bool {
    get(content, key).is_some_and(|value| !value.is_empty())
}

fn filename_date_prefix(stem: &str) -> Option<String> {
    is_date_prefixed(stem).then(|| format!("{}T00:00:00+00:00", &stem[..10]))
}

/// The inputs a caller (the migration's mechanical apply step) must
/// compute per file before calling [`rewrite`].
pub struct RewriteInput<'a> {
    pub file_display: &'a str,
    pub relpath: &'a str,
    pub repo_name: &'a str,
    pub table: &'a [(DocTypeKey, PathBuf)],
}

/// Applies the single-pass rewrite, returning the rewritten content and
/// every diagnostic emitted (REFUSE/DIVERGE/MALFORMED) without any
/// `0007: ` relay-wrap — nothing downstream parses these strings, so only
/// the tagged text itself is reproduced.
#[must_use]
pub fn rewrite(content: &str, input: &RewriteInput) -> (String, Vec<String>) {
    let Some(linkage_type) = resolve_type(content, input.relpath, input.table)
    else {
        return (content.to_owned(), Vec::new());
    };
    let Some(row) = fm_schema::row_for(linkage_type) else {
        return (content.to_owned(), Vec::new());
    };

    let stem = derive_stem(input.relpath, linkage_type);
    let id_from_stem = if linkage_type == "work-item" {
        let leading: String =
            stem.chars().take_while(char::is_ascii_digit).collect();
        if leading.is_empty() {
            stem.clone()
        } else {
            leading
        }
    } else {
        derive_id(linkage_type, &stem)
    };
    let canonical_id =
        (linkage_type == "pr-description").then(|| id_from_stem.clone());

    let has_title = presence(content, "title");
    let has_date = presence(content, "date");
    let has_author = presence(content, "author");

    let title_default = if has_title {
        None
    } else {
        Some(first_body_h1(content).unwrap_or_else(|| stem.clone()))
    };
    let author_default = (!has_author).then(|| "Unknown".to_owned());
    let date_default =
        (!has_date).then(|| filename_date_prefix(&stem)).flatten();
    let revision_default: Option<String> = None;

    let cur_title = get(content, "title")
        .as_deref()
        .map(semantic_inner)
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
        .or_else(|| title_default.clone())
        .unwrap_or_default();

    let mut diagnostics = Vec::new();
    let backfill_extras = required_extras_backfill(
        content,
        input.file_display,
        linkage_type,
        &stem,
        &cur_title,
        &mut diagnostics,
    );

    let state = State {
        file_display: input.file_display,
        linkage_type,
        anchored: row.code_state_anchored,
        own_id_key: own_id_key_for_type(linkage_type),
        forbidden: row.forbidden_own_id_keys,
        status_vocab: row.status_vocab,
        canonical_id: canonical_id.as_deref(),
        id_from_stem: &id_from_stem,
        title_default: title_default.as_deref(),
        author_default: author_default.as_deref(),
        date_default: date_default.as_deref(),
        revision_default: revision_default.as_deref(),
        repo_name: input.repo_name,
        backfill_extras: &backfill_extras,
        table: input.table,
        has_type: presence(content, "type"),
        has_id: presence(content, "id"),
        has_title,
        has_tags: presence(content, "tags"),
        has_schema: presence(content, "schema_version"),
        has_lu: presence(content, "last_updated"),
        has_lub: presence(content, "last_updated_by"),
        has_date,
        has_author,
        has_revision: presence(content, "revision"),
        has_repository: presence(content, "repository"),
        has_priority: presence(content, "priority"),
    };
    let (body, mut rule_diagnostics) = apply_rules(content, &state);
    diagnostics.append(&mut rule_diagnostics);
    (body, diagnostics)
}

#[allow(clippy::struct_excessive_bools)]
struct State<'a> {
    file_display: &'a str,
    linkage_type: &'a str,
    anchored: bool,
    own_id_key: Option<&'static str>,
    forbidden: &'static [&'static str],
    status_vocab: &'static [&'static str],
    canonical_id: Option<&'a str>,
    id_from_stem: &'a str,
    title_default: Option<&'a str>,
    author_default: Option<&'a str>,
    date_default: Option<&'a str>,
    revision_default: Option<&'a str>,
    repo_name: &'a str,
    backfill_extras: &'a [(&'static str, String)],
    table: &'a [(DocTypeKey, PathBuf)],
    has_type: bool,
    has_id: bool,
    has_title: bool,
    has_tags: bool,
    has_schema: bool,
    has_lu: bool,
    has_lub: bool,
    has_date: bool,
    has_author: bool,
    has_revision: bool,
    has_repository: bool,
    has_priority: bool,
}

/// What this rewrite pass itself decides to emit while scanning — distinct
/// from `State`'s `has_*` flags, which describe the file *before* the scan
/// started.
#[derive(Default)]
struct Emitted {
    id: bool,
    revision: bool,
    title: bool,
    date_value: Option<String>,
    author_value: Option<String>,
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::cognitive_complexity)]
fn apply_rules(content: &str, state: &State) -> (String, Vec<String>) {
    let mut out: Vec<String> = Vec::new();
    let mut diagnostics = Vec::new();
    let mut emitted = Emitted::default();

    let mut in_fm = false;
    let mut seen_open = false;

    for line in content.lines() {
        if !seen_open && is_strict_fence(line) {
            seen_open = true;
            in_fm = true;
            out.push(line.to_owned());
            continue;
        }
        if in_fm && is_strict_fence(line) {
            close_fence(state, &mut emitted, &mut out, &mut diagnostics);
            in_fm = false;
            out.push(line.to_owned());
            continue;
        }
        if !in_fm {
            out.push(line.to_owned());
            continue;
        }
        let Some((key, value)) = split_key_value(line) else {
            out.push(line.to_owned());
            continue;
        };

        match key.as_str() {
            "type" => {
                if !is_empty_value(&value) {
                    out.push(format!("type: {}", canonical_type(&value)));
                }
                continue;
            }
            "git_commit" => {
                if !state.has_revision && !value.is_empty() {
                    out.push(format!("revision: {}", normalise_value(&value)));
                    emitted.revision = true;
                }
                continue;
            }
            "branch" => continue,
            "ticket" | "ticket_id" => {
                if !is_empty_value(&value) {
                    diagnostics.push(format!(
                        "0007-DIVERGE[dropped-legacy-key]: {} — dropped {key}: {value}",
                        state.file_display
                    ));
                }
                continue;
            }
            "skill" => {
                out.push(format!("producer: {value}"));
                continue;
            }
            _ => {}
        }

        if state.own_id_key == Some(key.as_str()) {
            if refuses(&value) {
                out.push(line.to_owned());
                diagnostics.push(format!(
                    "0007-REFUSE: {} — refused {key} (unsafe value shape)",
                    state.file_display
                ));
            } else {
                out.push(format!("id: {}", normalise_value(&value)));
                emitted.id = true;
            }
            continue;
        }
        if key == "id" {
            if refuses(&value) {
                out.push(line.to_owned());
                diagnostics.push(format!(
                    "0007-REFUSE: {} — refused id (unsafe value shape)",
                    state.file_display
                ));
                continue;
            }
            if let Some(canonical_id) = state.canonical_id {
                if normalise_value(&value) != format!("\"{canonical_id}\"") {
                    diagnostics.push(format!(
                        "0007-DIVERGE[id-canonicalised]: {} — id {value} → \"{canonical_id}\"",
                        state.file_display
                    ));
                }
                out.push(format!("id: \"{canonical_id}\""));
            } else {
                out.push(format!("id: {}", normalise_value(&value)));
            }
            emitted.id = true;
            continue;
        }

        if state.forbidden.contains(&key.as_str()) {
            if key == "pr_title"
                && !state.has_title
                && !emitted.title
                && !is_empty_value(&value)
            {
                out.push(format!("title: {}", normalise_value(&value)));
                emitted.title = true;
            } else if key == "pr_title" && !is_empty_value(&value) {
                diagnostics.push(format!(
                    "0007-DIVERGE[discarded-key]: {} — pr_title discarded (title present): {value}",
                    state.file_display
                ));
            }
            continue;
        }

        if key == "date" || key == "last_updated" {
            if let Some(normalised) = normalise_date(&value) {
                out.push(format!("{key}: {normalised}"));
                if key == "date" {
                    emitted.date_value = Some(normalised);
                }
            } else {
                out.push(line.to_owned());
                diagnostics.push(format!(
                    "0007-DIVERGE[nonconforming-base-field]: {} — {key} not a normalisable ISO date: {value}",
                    state.file_display
                ));
            }
            continue;
        }

        if key == "author" {
            emitted.author_value = Some(value.clone());
            out.push(line.to_owned());
            continue;
        }

        if key == "status" {
            let inner = semantic_inner(&value);
            if inner.is_empty() {
                continue;
            }
            if state.status_vocab.contains(&inner) {
                out.push(line.to_owned());
                continue;
            }
            if let Some(mapped) = map_status(state.linkage_type, inner) {
                out.push(format!("status: {mapped}"));
            } else {
                out.push(line.to_owned());
                diagnostics.push(format!(
                    "0007-DIVERGE[unmapped-status]: {} — status '{inner}' not in vocab and unmapped",
                    state.file_display
                ));
            }
            continue;
        }

        if is_linkage_key(&key) {
            if is_empty_value(&value) {
                continue;
            }
            let outcome = linkage_value::normalise(
                &value,
                state.linkage_type,
                &key,
                state.table,
            );
            out.push(format!("{key}: {}", outcome.value));
            if outcome.unmapped_path {
                diagnostics.push(format!(
                    "0007-DIVERGE[unmapped-dir]: {} — {key} has a path under an unmapped directory: {value}",
                    state.file_display
                ));
            }
            if outcome.bare_ambiguous {
                diagnostics.push(format!(
                    "0007-DIVERGE[parent-ambiguous]: {} — {key} has a multi-candidate bare-number target (route to hook): {value}",
                    state.file_display
                ));
            }
            continue;
        }

        // Omit-when-empty: every remaining key except `tags`, which is
        // always kept even when empty (`omit_when_empty_key` returns false
        // only for `tags`).
        if key != "tags" && is_empty_value(&value) {
            continue;
        }
        out.push(line.to_owned());
    }

    if !seen_open {
        diagnostics.push(format!(
            "0007-MALFORMED: {} — no frontmatter fence detected",
            state.file_display
        ));
    }

    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    (result, diagnostics)
}

fn split_key_value(line: &str) -> Option<(String, String)> {
    let mut chars = line.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    let colon = line.find(':')?;
    let key = &line[..colon];
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let value = line[colon + 1..].trim();
    Some((key.to_owned(), value.to_owned()))
}

fn normalise_date(value: &str) -> Option<String> {
    let inner = semantic_inner(value);
    let bytes = inner.as_bytes();
    let date_shaped = bytes.len() >= 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit);
    if !date_shaped {
        return None;
    }
    if inner.len() == 10 {
        return Some(format!("\"{inner}T00:00:00+00:00\""));
    }
    if is_full_iso(inner) {
        return Some(format!("\"{inner}\""));
    }
    if let Some(colon_inserted) = colonless_offset(inner) {
        return Some(format!("\"{colon_inserted}\""));
    }
    // Last resort: a value that merely starts with a date but is otherwise
    // non-ISO — keep the date, drop the unrepresentable time.
    Some(format!("\"{}T00:00:00+00:00\"", &inner[..10]))
}

fn is_full_iso(inner: &str) -> bool {
    let bytes = inner.as_bytes();
    if bytes.len() < 20
        || bytes[10] != b'T'
        || !bytes[11..13].iter().all(u8::is_ascii_digit)
        || bytes[13] != b':'
        || !bytes[14..16].iter().all(u8::is_ascii_digit)
        || bytes[16] != b':'
        || !bytes[17..19].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    match &inner[19..] {
        "Z" => true,
        rest if rest.len() == 6 => {
            let rest = rest.as_bytes();
            matches!(rest[0], b'+' | b'-')
                && rest[1].is_ascii_digit()
                && rest[2].is_ascii_digit()
                && rest[3] == b':'
                && rest[4].is_ascii_digit()
                && rest[5].is_ascii_digit()
        }
        _ => false,
    }
}

fn colonless_offset(inner: &str) -> Option<String> {
    let bytes = inner.as_bytes();
    if bytes.len() != 24
        || bytes[10] != b'T'
        || !bytes[11..13].iter().all(u8::is_ascii_digit)
        || bytes[13] != b':'
        || !bytes[14..16].iter().all(u8::is_ascii_digit)
        || bytes[16] != b':'
        || !bytes[17..19].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let sign = bytes[19];
    if !matches!(sign, b'+' | b'-')
        || !bytes[20..24].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let base = &inner[..19];
    let offset = &inner[19..23];
    Some(format!("{base}{}:{}", &offset[..3], &offset[3..]))
}

fn close_fence(
    state: &State,
    emitted: &mut Emitted,
    out: &mut Vec<String>,
    diagnostics: &mut Vec<String>,
) {
    if !state.has_type {
        out.push(format!("type: {}", state.linkage_type));
    }
    if !emitted.id && !state.has_id {
        if state.id_from_stem.is_empty() {
            diagnostics.push(format!(
                "0007-REFUSE: {} — no id and no derivable filename stem",
                state.file_display
            ));
        } else {
            out.push(format!("id: \"{}\"", state.id_from_stem));
        }
    }
    if !state.has_title && !emitted.title {
        if let Some(title_default) = state.title_default {
            if !title_default.is_empty() {
                out.push(format!("title: \"{title_default}\""));
            }
        }
    }
    if !state.has_date {
        if let Some(date_default) = state.date_default {
            out.push(format!("date: \"{date_default}\""));
            if emitted.date_value.is_none() {
                emitted.date_value = Some(format!("\"{date_default}\""));
            }
        }
    }
    if !state.has_author {
        if let Some(author_default) = state.author_default {
            out.push(format!("author: {author_default}"));
        }
    }
    if !state.has_tags {
        out.push("tags: []".to_owned());
    }
    if !state.has_schema {
        out.push("schema_version: 1".to_owned());
    }
    if !state.has_lu {
        let seed = emitted.date_value.clone().or_else(|| {
            state.date_default.map(|default| format!("\"{default}\""))
        });
        if let Some(seed) = seed {
            out.push(format!("last_updated: {seed}"));
        }
    }
    if !state.has_lub {
        let seed_by = emitted
            .author_value
            .clone()
            .or_else(|| state.author_default.map(ToOwned::to_owned));
        if let Some(seed_by) = seed_by {
            out.push(format!("last_updated_by: {seed_by}"));
        }
    }
    if state.linkage_type == "work-item" && !state.has_priority {
        out.push("priority: medium".to_owned());
    }
    if state.anchored {
        if !emitted.revision && !state.has_revision {
            if let Some(revision_default) = state.revision_default {
                out.push(format!("revision: \"{revision_default}\""));
            } else {
                diagnostics.push(format!(
                    "0007-DIVERGE[author-lookup-failed]: {} — anchored type missing revision (no git_commit, no VCS revision)",
                    state.file_display
                ));
            }
        }
        if !state.has_repository {
            if state.repo_name.is_empty() {
                diagnostics.push(format!(
                    "0007-DIVERGE[nonconforming-base-field]: {} — anchored type missing repository",
                    state.file_display
                ));
            } else {
                out.push(format!("repository: \"{}\"", state.repo_name));
            }
        }
    }
    for (name, value) in state.backfill_extras {
        emit_backfill_extra(name, value, out, diagnostics, state.file_display);
    }
}

fn emit_backfill_extra(
    name: &str,
    value: &str,
    out: &mut Vec<String>,
    diagnostics: &mut Vec<String>,
    file_display: &str,
) {
    match name {
        "lenses" => {
            out.push(format!("lenses: [\"{value}\"]"));
            diagnostics.push(format!(
                "0007-DIVERGE[backfilled-extra]: {file_display} — {name} backfilled with sentinel; review manually"
            ));
        }
        "verdict" => {
            out.push(format!("verdict: {}", normalise_value(value)));
            diagnostics.push(format!(
                "0007-DIVERGE[backfilled-extra]: {file_display} — {name} backfilled with sentinel; review manually"
            ));
        }
        "pr_number"
        | "review_number"
        | "sequence"
        | "review_pass"
        | "screenshots_incomplete" => {
            out.push(format!("{name}: {value}"));
        }
        _ => out.push(format!("{name}: {}", normalise_value(value))),
    }
}

/// Every default value for a type's required-and-currently-absent extras,
/// as `(name, value)` pairs — the packed backfill channel, unpacked.
/// Underivable string/enum extras are forced to the `unknown` sentinel with
/// a `backfill-sentinel` breadcrumb (distinct from `lenses`/`verdict`'s own
/// `backfilled-extra` tag, emitted at emission time instead).
fn required_extras_backfill(
    content: &str,
    file_display: &str,
    linkage_type: &str,
    stem: &str,
    cur_title: &str,
    diagnostics: &mut Vec<String>,
) -> Vec<(&'static str, String)> {
    let Some(row) = fm_schema::row_for(linkage_type) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for &extra in row.extras {
        if fm_schema::OPTIONAL_EXTRAS.contains(&extra) {
            continue;
        }
        let current = get(content, extra);
        if current.is_some_and(|value| !is_empty_value(&value)) {
            continue;
        }
        let mut value = extra_default(extra, stem, cur_title);
        if value.is_empty() {
            "unknown".clone_into(&mut value);
            diagnostics.push(format!(
                "0007-DIVERGE[backfill-sentinel]: {file_display} — required extra '{extra}' has no derivable default; stamped 'unknown'"
            ));
        }
        result.push((extra, value));
    }
    result
}

fn extra_default(name: &str, stem: &str, cur_title: &str) -> String {
    match name {
        "topic" => cur_title.replace('"', ""),
        "pr_number" => pr_number_of(stem).unwrap_or_default(),
        "review_number" | "review_pass" | "sequence" => "1".to_owned(),
        "screenshots_incomplete" => "true".to_owned(),
        "verdict" | "lenses" => "unknown".to_owned(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use corpus::doc_type::DocTypeKey;

    use super::rewrite;
    use super::RewriteInput;

    fn table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![(DocTypeKey::WorkItems, PathBuf::from("meta/work"))]
    }

    fn input<'a>(
        relpath: &'a str,
        table: &'a [(DocTypeKey, PathBuf)],
    ) -> RewriteInput<'a> {
        RewriteInput {
            file_display: relpath,
            relpath,
            repo_name: "repo",
            table,
        }
    }

    #[test]
    fn the_mechanical_rewrite_happy_path_matches_every_bash_rule_at_once() {
        let content = "---\nwork_item_id: 0042\nkind: task\npriority: medium\n\
             skill: create-work-item\ngit_commit: deadbeef123\ndate: \
             2026-01-01\nticket: LEGACY-1\n---\n\n# 0042: A Work Item\n";
        let table = table();

        let (rewritten, diagnostics) =
            rewrite(content, &input("meta/work/0042-a-work-item.md", &table));

        assert!(
            rewritten.contains("id: \"0042\"\n"),
            "own id key not canonicalised: {rewritten}"
        );
        assert!(
            !rewritten.contains("work_item_id:"),
            "the old own-id key must not survive: {rewritten}"
        );
        assert!(
            rewritten.contains("producer: create-work-item\n"),
            "skill: must fold to producer:: {rewritten}"
        );
        assert!(
            rewritten.contains("revision: \"deadbeef123\"\n"),
            "git_commit: must fold to revision:: {rewritten}"
        );
        assert!(
            !rewritten.contains("git_commit:"),
            "the old git_commit: key must not survive: {rewritten}"
        );
        assert!(
            !rewritten.contains("ticket:"),
            "ticket: must be dropped: {rewritten}"
        );
        assert!(
            diagnostics.iter().any(|line| line
                .starts_with("0007-DIVERGE[dropped-legacy-key]:")
                && line.contains("ticket: LEGACY-1")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn rewriting_an_already_rewritten_document_is_a_byte_for_byte_no_op() {
        let content = "---\nwork_item_id: 0042\nkind: task\npriority: medium\n\
             skill: create-work-item\ngit_commit: deadbeef123\ndate: \
             2026-01-01\nticket: LEGACY-1\n---\n\n# 0042: A Work Item\n";
        let table = table();

        let (once, _) =
            rewrite(content, &input("meta/work/0042-a-work-item.md", &table));
        let (twice, diagnostics_twice) =
            rewrite(&once, &input("meta/work/0042-a-work-item.md", &table));

        assert_eq!(once, twice, "a second pass must be a pure no-op");
        assert!(
            diagnostics_twice.is_empty(),
            "a second pass must raise no fresh diagnostics: {diagnostics_twice:?}"
        );
    }

    #[test]
    fn an_empty_pr_title_with_no_existing_title_falls_back_to_the_default() {
        let content = "---\nwork_item_id: 0042\nkind: task\n\
             priority: medium\npr_title:\n---\n\n# 0042: Fallback Title\n";
        let table = table();

        let (rewritten, _) = rewrite(
            content,
            &input("meta/work/0042-fallback-title.md", &table),
        );

        assert!(
            rewritten.contains("title: \"0042: Fallback Title\"\n"),
            "expected the H1-derived default title (verbatim, ID prefix \
             included): {rewritten}"
        );
        assert!(
            !rewritten.contains("title: \"\"\n"),
            "an empty pr_title must never leave an empty placeholder: \
             {rewritten}"
        );
    }

    #[test]
    fn a_nonempty_pr_title_folds_to_title_when_no_title_exists() {
        let content = "---\nreview_number: 1\nverdict: approved\n\
             lenses: [\"correctness\"]\npr_number: 42\n\
             pr_title: The Real Title\n---\n\n# Stem\n";
        let table = pr_review_table();

        let (rewritten, diagnostics) =
            rewrite(content, &input("meta/reviews/prs/pr-42-stem.md", &table));

        assert!(
            rewritten.contains("title: \"The Real Title\"\n"),
            "{rewritten}"
        );
        assert!(!rewritten.contains("pr_title:"), "{rewritten}");
        assert!(
            !diagnostics.iter().any(|line| line.contains("pr_title")),
            "a clean fold must not raise a diagnostic: {diagnostics:?}"
        );
    }

    #[test]
    fn a_differing_pr_title_is_dropped_with_a_diverge_diagnostic() {
        let content = "---\nreview_number: 1\nverdict: approved\n\
             lenses: [\"correctness\"]\npr_number: 42\n\
             title: Kept Title\npr_title: Discarded Title\n---\n\n# Stem\n";
        let table = pr_review_table();

        let (rewritten, diagnostics) =
            rewrite(content, &input("meta/reviews/prs/pr-42-stem.md", &table));

        assert!(rewritten.contains("title: Kept Title\n"), "{rewritten}");
        assert!(!rewritten.contains("pr_title:"), "{rewritten}");
        assert!(
            diagnostics
                .iter()
                .any(|line| line.starts_with("0007-DIVERGE[discarded-key]:")
                    && line.contains("pr_title")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn an_equal_pr_title_is_still_dropped_with_a_diverge_diagnostic() {
        let content = "---\nreview_number: 1\nverdict: approved\n\
             lenses: [\"correctness\"]\npr_number: 42\n\
             title: \"Same Title\"\npr_title: \"Same Title\"\n---\n\n\
             # Stem\n";
        let table = pr_review_table();

        let (rewritten, diagnostics) =
            rewrite(content, &input("meta/reviews/prs/pr-42-stem.md", &table));

        assert!(rewritten.contains("title: \"Same Title\"\n"), "{rewritten}");
        assert!(!rewritten.contains("pr_title:"), "{rewritten}");
        assert!(
            diagnostics
                .iter()
                .any(|line| line.starts_with("0007-DIVERGE[discarded-key]:")
                    && line.contains("pr_title")),
            "the bash original discards on any pre-existing title:, with no \
             equal-value special case: {diagnostics:?}"
        );
    }

    #[test]
    fn a_ticket_id_key_is_dropped_with_its_own_diverge_diagnostic() {
        let content = "---\nwork_item_id: 0042\nkind: task\n\
             priority: medium\nticket_id: LEGACY-2\n---\n\n# Stem\n";
        let table = table();

        let (rewritten, diagnostics) =
            rewrite(content, &input("meta/work/0042-stem.md", &table));

        assert!(!rewritten.contains("ticket_id:"), "{rewritten}");
        assert!(
            diagnostics.iter().any(|line| line
                .starts_with("0007-DIVERGE[dropped-legacy-key]:")
                && line.contains("ticket_id: LEGACY-2")),
            "{diagnostics:?}"
        );
    }

    fn note_table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![(DocTypeKey::Notes, PathBuf::from("meta/notes"))]
    }

    fn pr_review_table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![(DocTypeKey::PrReviews, PathBuf::from("meta/reviews/prs"))]
    }

    fn design_inventory_table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![(
            DocTypeKey::DesignInventories,
            PathBuf::from("meta/research/design-inventories"),
        )]
    }

    #[test]
    fn topic_backfills_silently_from_the_title_when_absent() {
        let content = "---\ntitle: My Note\n---\n\n# My Note\n\nSome body.\n";
        let table = note_table();

        let (rewritten, diagnostics) = rewrite(
            content,
            &input("meta/notes/2026-01-01-my-note.md", &table),
        );

        assert!(rewritten.contains("topic: \"My Note\"\n"), "{rewritten}");
        assert!(
            !diagnostics.iter().any(|line| line.contains("topic")),
            "a derivable topic must not raise its own diagnostic: \
             {diagnostics:?}"
        );
    }

    #[test]
    fn a_semicolon_bearing_title_survives_into_the_backfilled_topic() {
        let content = "---\ntitle: \"Fix; then ship\"\n---\n\n\
             # Fix; then ship\n\nBody.\n";
        let table = note_table();

        let (rewritten, _) =
            rewrite(content, &input("meta/notes/2026-01-01-fix.md", &table));

        assert!(
            rewritten.contains("topic: \"Fix; then ship\"\n"),
            "{rewritten}"
        );
    }

    #[test]
    fn verdict_and_lenses_get_the_quoted_unknown_sentinel_with_a_diagnostic() {
        let content = "---\nreview_number: 1\npr_number: 42\n---\n\n\
             # Review\n";
        let table = pr_review_table();

        let (rewritten, diagnostics) = rewrite(
            content,
            &input("meta/reviews/prs/pr-42-review.md", &table),
        );

        assert!(rewritten.contains("verdict: \"unknown\"\n"), "{rewritten}");
        assert!(rewritten.contains("lenses: [\"unknown\"]\n"), "{rewritten}");
        for extra in ["verdict", "lenses"] {
            assert!(
                diagnostics.iter().any(|line| line
                    .starts_with("0007-DIVERGE[backfilled-extra]:")
                    && line.contains(extra)),
                "missing backfilled-extra diagnostic for {extra}: \
                 {diagnostics:?}"
            );
        }
    }

    #[test]
    fn pr_number_derives_from_a_pr_token_not_the_date_prefix_year() {
        let content = "---\nreview_number: 1\nverdict: approved\n\
             lenses: [\"correctness\"]\n---\n\n# Review\n";
        let table = pr_review_table();

        let (rewritten, diagnostics) = rewrite(
            content,
            &input("meta/reviews/prs/2026-06-17-pr-430-review.md", &table),
        );

        assert!(rewritten.contains("pr_number: 430\n"), "{rewritten}");
        assert!(
            !diagnostics.iter().any(|line| line.contains("pr_number")),
            "a derivable pr_number must not raise a diagnostic: \
             {diagnostics:?}"
        );
    }

    #[test]
    fn pr_number_falls_back_to_the_bare_unquoted_unknown_sentinel() {
        let content = "---\nreview_number: 1\nverdict: approved\n\
             lenses: [\"correctness\"]\n---\n\n# Review\n";
        let table = pr_review_table();

        let (rewritten, diagnostics) = rewrite(
            content,
            &input("meta/reviews/prs/2026-06-17-no-number-here.md", &table),
        );

        assert!(
            rewritten.contains("pr_number: unknown\n"),
            "expected a bare, unquoted sentinel: {rewritten}"
        );
        assert!(
            !rewritten.contains("pr_number: \"unknown\"\n"),
            "pr_number must never be quoted: {rewritten}"
        );
        assert!(
            diagnostics.iter().any(|line| line
                .starts_with("0007-DIVERGE[backfill-sentinel]:")
                && line.contains("pr_number")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn numeric_and_boolean_extras_are_emitted_bare_not_quoted() {
        let content = "---\nsource: known\nsource_kind: repo\n\
             source_location: \"https://example.com\"\ncrawler: manual\n\
             ---\n\n# Inventory\n";
        let table = design_inventory_table();

        let (rewritten, _) = rewrite(
            content,
            &input(
                "meta/research/design-inventories/auth/inventory.md",
                &table,
            ),
        );

        assert!(rewritten.contains("sequence: 1\n"), "{rewritten}");
        assert!(
            rewritten.contains("screenshots_incomplete: true\n"),
            "{rewritten}"
        );
        assert!(
            !rewritten.contains("sequence: \"1\"\n"),
            "sequence must never be quoted: {rewritten}"
        );
    }

    #[test]
    fn the_string_enum_sentinel_bundle_routes_to_quoted_unknown() {
        let content = "---\nsequence: 1\nscreenshots_incomplete: true\n\
             ---\n\n# Inventory\n";
        let table = design_inventory_table();

        let (rewritten, diagnostics) = rewrite(
            content,
            &input(
                "meta/research/design-inventories/auth/inventory.md",
                &table,
            ),
        );

        for extra in ["source", "source_kind", "source_location", "crawler"] {
            assert!(
                rewritten.contains(&format!("{extra}: \"unknown\"\n")),
                "expected {extra} to get the quoted sentinel: {rewritten}"
            );
            assert!(
                diagnostics.iter().any(|line| line
                    .starts_with("0007-DIVERGE[backfill-sentinel]:")
                    && line.contains(extra)),
                "missing backfill-sentinel diagnostic for {extra}: \
                 {diagnostics:?}"
            );
        }
    }

    #[test]
    fn populated_extras_are_not_clobbered_and_raise_no_diagnostics() {
        let content = "---\nverdict: approved\nlenses: [\"correctness\"]\n\
             review_number: 2\npr_number: 42\n---\n\n# Review\n";
        let table = pr_review_table();

        let (rewritten, diagnostics) = rewrite(
            content,
            &input("meta/reviews/prs/pr-42-review.md", &table),
        );

        assert!(rewritten.contains("verdict: approved\n"), "{rewritten}");
        assert!(
            rewritten.contains("lenses: [\"correctness\"]\n"),
            "{rewritten}"
        );
        assert!(rewritten.contains("review_number: 2\n"), "{rewritten}");
        assert!(rewritten.contains("pr_number: 42\n"), "{rewritten}");
        assert!(
            diagnostics.is_empty(),
            "a fully-populated document must raise no backfill \
             diagnostics: {diagnostics:?}"
        );
    }

    #[test]
    fn an_absent_type_key_infers_from_the_configured_path() {
        let content = "---\nreview_number: 1\nverdict: approved\n\
             lenses: [\"correctness\"]\npr_number: 42\n---\n\n# Stem\n";
        let table = pr_review_table();

        let resolved =
            super::resolve_type(content, "meta/reviews/prs/pr-42.md", &table);

        assert_eq!(resolved, Some("pr-review"));
    }

    #[test]
    fn a_path_outside_every_configured_directory_has_no_resolved_type() {
        let content = "---\ntitle: A doc\n---\n\n# A doc\n";
        let table = table();

        let resolved =
            super::resolve_type(content, "meta/docs/logging-guide.md", &table);

        assert_eq!(resolved, None);
    }

    #[test]
    fn pr_number_of_derives_from_a_pr_token_or_a_bare_leading_number() {
        for (stem, expected) in [
            ("2026-06-17-pr-416-review", Some("416")),
            ("2026-06-17-summary", None),
            ("2026-06-17-0114-foo", None),
            ("240-description", Some("240")),
            ("expr-3-foo", None),
        ] {
            assert_eq!(
                super::pr_number_of(stem),
                expected.map(str::to_owned),
                "stem {stem}"
            );
        }
    }

    #[test]
    fn the_required_extras_contract_has_not_silently_drifted() {
        use corpus::frontmatter_validation::schema as fm_schema;

        let required_for = |linkage_type: &str| -> Vec<&'static str> {
            fm_schema::row_for(linkage_type)
                .map(|row| {
                    row.extras
                        .iter()
                        .copied()
                        .filter(|extra| {
                            !fm_schema::OPTIONAL_EXTRAS.contains(extra)
                        })
                        .collect()
                })
                .unwrap_or_default()
        };

        let pr_review_required = required_for("pr-review");
        for extra in ["verdict", "lenses", "review_number", "pr_number"] {
            assert!(
                pr_review_required.contains(&extra),
                "expected {extra} to remain a required pr-review extra: \
                 {pr_review_required:?}"
            );
        }

        for linkage_type in ["note", "codebase-research", "issue-research"] {
            let required = required_for(linkage_type);
            assert!(
                required.contains(&"topic"),
                "expected topic to remain required for {linkage_type}: \
                 {required:?}"
            );
        }
    }

    #[test]
    fn backfill_completes_rather_than_aborting_when_underivable() {
        // The sentinel path, not a hard failure — distinct from a prepass
        // REFUSE, which does abort the whole migration.
        let content = "---\nreview_number: 1\nverdict: approved\n\
             lenses: [\"correctness\"]\n---\n\n# Review\n";
        let table = pr_review_table();

        let (_, diagnostics) = rewrite(
            content,
            &input("meta/reviews/prs/no-number-anywhere.md", &table),
        );

        // rewrite() itself never fails — it always returns; the migration
        // only aborts on REFUSE/MALFORMED, which this scenario never raises.
        assert!(
            diagnostics
                .iter()
                .any(|line| line.contains("backfill-sentinel")),
            "{diagnostics:?}"
        );
    }
}

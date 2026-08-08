//! The read-only precondition prepass: every refusal aborts the whole
//! migration before any mutation. A line-oriented port of
//! `precondition_prepass`.

use std::path::PathBuf;

use corpus::doc_type::DocTypeKey;

use crate::migrations::m0007::fence;
use crate::migrations::m0007::quote::is_quoted;
use crate::migrations::m0007::quote::semantic_inner;
use crate::migrations::m0007::rewrite::derive_id;
use crate::migrations::m0007::rewrite::derive_stem;
use crate::migrations::m0007::rewrite::resolve_type;
use crate::migrations::m0007::schema::own_id_key_for_type;

fn non_empty_semantic(raw: Option<&str>) -> Option<String> {
    raw.map(semantic_inner)
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
}

/// Every `0007-REFUSE` diagnostic the prepass finds, in scan order. An empty
/// result means the migration may proceed.
#[must_use]
pub fn precondition_prepass(
    files: &[(String, String)],
    table: &[(DocTypeKey, PathBuf)],
) -> Vec<String> {
    let mut refusals = Vec::new();
    let mut seen_ids: Vec<String> = Vec::new();

    for (relpath, content) in files {
        if !fence::has_strict_fence(content) {
            continue;
        }
        let Some(linkage_type) = resolve_type(content, relpath, table) else {
            continue;
        };

        if linkage_type == "work-item" {
            let kind = fence::get(content, "kind");
            if kind.as_deref().unwrap_or_default().is_empty() {
                refusals.push(format!(
                    "0007-REFUSE: {relpath} — work-item missing kind: \
                     (run migration 0005 first)"
                ));
            }
            let own = non_empty_semantic(
                fence::get(content, "work_item_id").as_deref(),
            );
            let stem = derive_stem(relpath, linkage_type);
            let filename_id: String =
                stem.chars().take_while(char::is_ascii_digit).collect();
            if let Some(own) = &own {
                if !filename_id.is_empty() && own != &filename_id {
                    refusals.push(format!(
                        "0007-REFUSE: {relpath} — own work_item_id '{own}' \
                         != filename id '{filename_id}'"
                    ));
                }
            }
        } else if let Some(raw) = fence::get(content, "work_item_id") {
            if !raw.is_empty() && !is_quoted(&raw) {
                refusals.push(format!(
                    "0007-REFUSE: {relpath} — foreign work_item_id \
                     unquoted: {raw} (run migration 0006 first)"
                ));
            }
        }

        let own_id = own_id_key_for_type(linkage_type).and_then(|key| {
            non_empty_semantic(fence::get(content, key).as_deref())
        });
        let id = own_id
            .or_else(|| {
                non_empty_semantic(fence::get(content, "id").as_deref())
            })
            .unwrap_or_else(|| {
                let stem = derive_stem(relpath, linkage_type);
                derive_id(linkage_type, &stem)
            });
        let typed_id = format!("{linkage_type}:{id}");
        if seen_ids.contains(&typed_id) {
            refusals.push(format!(
                "0007-REFUSE: {relpath} — duplicate post-rewrite id \
                 '{typed_id}'"
            ));
        }
        seen_ids.push(typed_id);
    }

    refusals
}

#[cfg(test)]
mod tests {
    use super::precondition_prepass;
    use corpus::doc_type::DocTypeKey;
    use std::path::PathBuf;

    fn table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![(DocTypeKey::WorkItems, PathBuf::from("meta/work"))]
    }

    fn work_item(body: &str) -> String {
        format!(
            "---\ntype: work-item\nid: \"0042\"\ntitle: t\n\
             date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
             last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
             schema_version: 1\n{body}---\nbody\n"
        )
    }

    #[test]
    fn a_work_item_missing_kind_is_refused() {
        let files = vec![(
            "meta/work/0042-foo.md".to_owned(),
            work_item("status: draft\npriority: medium\n"),
        )];
        let refusals = precondition_prepass(&files, &table());
        assert!(refusals
            .iter()
            .any(|r| r.contains("missing kind") && r.contains("0042-foo.md")));
    }

    #[test]
    fn a_matching_kind_and_own_id_passes() {
        let files = vec![(
            "meta/work/0042-foo.md".to_owned(),
            work_item("kind: task\nstatus: draft\npriority: medium\n"),
        )];
        assert!(precondition_prepass(&files, &table()).is_empty());
    }

    #[test]
    fn a_mismatched_own_work_item_id_is_refused() {
        let content = "---\ntype: work-item\nwork_item_id: \"0099\"\n\
             id: \"0042\"\ntitle: t\ndate: \"2026-01-01T00:00:00Z\"\n\
             author: a\ntags: []\nkind: task\nstatus: draft\n\
             priority: medium\nlast_updated: \"2026-01-01T00:00:00Z\"\n\
             last_updated_by: a\nschema_version: 1\n---\nbody\n";
        let files =
            vec![("meta/work/0042-foo.md".to_owned(), content.to_owned())];
        let refusals = precondition_prepass(&files, &table());
        assert!(refusals.iter().any(|r| r.contains("!= filename id")));
    }

    #[test]
    fn an_unquoted_foreign_work_item_id_is_refused() {
        let content = "---\ntype: plan\nwork_item_id: 0042\nid: \"x\"\n\
             title: t\ndate: \"2026-01-01T00:00:00Z\"\nauthor: a\n\
             tags: []\nlast_updated: \"2026-01-01T00:00:00Z\"\n\
             last_updated_by: a\nschema_version: 1\n---\nbody\n";
        let files =
            vec![("meta/decisions/x.md".to_owned(), content.to_owned())];
        let table = vec![(DocTypeKey::Plans, PathBuf::from("meta/decisions"))];
        let refusals = precondition_prepass(&files, &table);
        assert!(refusals.iter().any(|r| r.contains("unquoted")));
    }

    #[test]
    fn a_quoted_foreign_work_item_id_passes() {
        let content = "---\ntype: plan\nwork_item_id: \"0042\"\nid: \"x\"\n\
             title: t\ndate: \"2026-01-01T00:00:00Z\"\nauthor: a\n\
             tags: []\nlast_updated: \"2026-01-01T00:00:00Z\"\n\
             last_updated_by: a\nschema_version: 1\n---\nbody\n";
        let files =
            vec![("meta/decisions/x.md".to_owned(), content.to_owned())];
        let table = vec![(DocTypeKey::Plans, PathBuf::from("meta/decisions"))];
        assert!(precondition_prepass(&files, &table).is_empty());
    }

    #[test]
    fn two_work_items_sharing_a_post_rewrite_id_are_refused() {
        let a = work_item("kind: task\nstatus: draft\npriority: medium\n")
            .replace("id: \"0042\"", "id: \"0099\"");
        let b = work_item("kind: task\nstatus: draft\npriority: medium\n")
            .replace("id: \"0042\"", "id: \"0099\"");
        let files = vec![
            ("meta/work/a.md".to_owned(), a),
            ("meta/work/b.md".to_owned(), b),
        ];
        let refusals = precondition_prepass(&files, &table());
        assert!(refusals
            .iter()
            .any(|r| r.contains("duplicate post-rewrite id")));
    }
}

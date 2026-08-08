//! Port of `0005-rename-work-item-type-to-kind.sh`.
//!
//! Renames the `type:` frontmatter key and the `**Type**:` body label to
//! `kind:`/`**Kind**:` across every `.md` file under the configured work
//! directory, dropping the old form on divergence rather than aborting.

use crate::migrations::text::filter_lines;
use crate::migrations::text::map_lines;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0005;

impl MigrationMeta for Migration0005 {
    fn id(&self) -> &'static str {
        "0005-rename-work-item-type-to-kind"
    }

    fn description(&self) -> &'static str {
        "Rename work-item type field to kind in frontmatter and body \
         labels."
    }
}

impl Migration for Migration0005 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let work_dir_rel = ctx.config_value("paths.work").unwrap_or_default();
        if work_dir_rel.is_empty() {
            return Err(MigrationError::new(
                "0005: config path returned empty for 'work'",
            ));
        }
        if is_dangerous_path(&work_dir_rel) {
            return Err(MigrationError::new(format!(
                "0005: refusing dangerous paths.work value: {work_dir_rel}"
            )));
        }

        let root = ctx.root().to_path_buf();
        let work_dir = root.join(&work_dir_rel);
        if !ctx.dir_exists(&work_dir) {
            eprintln!(
                "Warning: 0005: work directory does not exist: \
                 {work_dir_rel}"
            );
            println!("0005: rewrote 0 file(s) under {work_dir_rel}");
            return Ok(ApplyOutcome::Applied);
        }

        let mut rewrote = 0usize;
        for file in ctx.list_md_files(&work_dir)? {
            let Some(content) = ctx.read(&file)? else {
                continue;
            };
            let mut touched = false;
            let mut rewritten = content;

            if let Some(next) = rewrite_pair(
                &rewritten,
                &file,
                "type:",
                "kind:",
                "0005: divergent type/kind",
                "type",
                "kind",
            ) {
                rewritten = next;
                touched = true;
            }
            if let Some(next) = rewrite_pair(
                &rewritten,
                &file,
                "**Type**:",
                "**Kind**:",
                "0005: divergent **Type**/**Kind** body label",
                "Type",
                "Kind",
            ) {
                rewritten = next;
                touched = true;
            }

            if touched {
                ctx.write(&file, &rewritten)?;
                rewrote += 1;
            }
        }

        println!("0005: rewrote {rewrote} file(s) under {work_dir_rel}");
        Ok(ApplyOutcome::Applied)
    }
}

fn is_dangerous_path(path: &str) -> bool {
    path == "."
        || path == ".."
        || path == "/"
        || path.starts_with('/')
        || path.ends_with("/..")
        || path.starts_with("../")
        || path.contains("/../")
}

fn value_after(content: &str, prefix: &str) -> Option<String> {
    content.lines().find_map(|line| {
        line.strip_prefix(prefix)
            .map(|rest| rest.trim_start().to_owned())
    })
}

/// One old/new pair's rewrite: drops the old line (warning on divergence)
/// when the new form is already present, else renames the prefix. `None`
/// when the old form is absent (nothing to do).
#[allow(clippy::too_many_arguments)]
fn rewrite_pair(
    content: &str,
    file: &std::path::Path,
    old_prefix: &str,
    new_prefix: &str,
    divergence_header: &str,
    old_label: &str,
    new_label: &str,
) -> Option<String> {
    if !content.lines().any(|line| line.starts_with(old_prefix)) {
        return None;
    }
    if content.lines().any(|line| line.starts_with(new_prefix)) {
        let old_value = value_after(content, old_prefix).unwrap_or_default();
        let new_value = value_after(content, new_prefix).unwrap_or_default();
        if old_value != new_value {
            eprintln!(
                "Warning: {divergence_header} in {} — kept \
                 {new_label}={new_value}, dropped {old_label}={old_value}",
                file.display()
            );
        }
        Some(filter_lines(content, |line| !line.starts_with(old_prefix)))
    } else {
        Some(map_lines(content, |line| {
            line.strip_prefix(old_prefix).map_or_else(
                || line.to_owned(),
                |rest| format!("{new_prefix}{rest}"),
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{is_dangerous_path, rewrite_pair};

    #[test]
    fn dangerous_paths_are_refused() {
        for path in [".", "..", "/", "/abs", "a/..", "../b", "a/../b"] {
            assert!(is_dangerous_path(path), "{path}");
        }
        for path in ["meta/work", "custom/work"] {
            assert!(!is_dangerous_path(path), "{path}");
        }
    }

    #[test]
    fn renames_the_frontmatter_key_when_kind_absent() {
        let content = "---\ntype: work-item\ntitle: x\n---\n";
        assert_eq!(
            rewrite_pair(
                content,
                Path::new("f.md"),
                "type:",
                "kind:",
                "divergent type/kind",
                "type",
                "kind",
            ),
            Some("---\nkind: work-item\ntitle: x\n---\n".to_owned())
        );
    }

    #[test]
    fn drops_type_when_kind_already_present() {
        let content = "---\nkind: work-item\ntype: work-item\n---\n";
        assert_eq!(
            rewrite_pair(
                content,
                Path::new("f.md"),
                "type:",
                "kind:",
                "divergent type/kind",
                "type",
                "kind",
            ),
            Some("---\nkind: work-item\n---\n".to_owned())
        );
    }

    #[test]
    fn leaves_a_file_without_the_old_key_untouched() {
        let content = "---\nkind: work-item\n---\n";
        assert!(rewrite_pair(
            content,
            Path::new("f.md"),
            "type:",
            "kind:",
            "divergent type/kind",
            "type",
            "kind"
        )
        .is_none());
    }

    #[test]
    fn renames_the_body_label() {
        let content = "# Title\n\n**Type**: work-item\n";
        assert_eq!(
            rewrite_pair(
                content,
                Path::new("f.md"),
                "**Type**:",
                "**Kind**:",
                "divergent **Type**/**Kind** body label",
                "Type",
                "Kind",
            ),
            Some("# Title\n\n**Kind**: work-item\n".to_owned())
        );
    }
}

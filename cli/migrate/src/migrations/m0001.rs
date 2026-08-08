//! Port of `0001-rename-tickets-to-work.sh`.
//!
//! Renames `ticket_id:`/`tickets`/`ticket_revise_*` terminology to
//! `work_item_id:`/`work`/`work_item_revise_*` across frontmatter, the
//! `meta/tickets` → `meta/work` directory pair, and the team/local config
//! files.

use crate::migrations::text::filter_lines;
use crate::migrations::text::flat_rename;
use crate::migrations::text::flat_rename_default;
use crate::migrations::text::indented_rename;
use crate::migrations::text::indented_rename_default;
use crate::migrations::text::map_lines;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0001;

impl MigrationMeta for Migration0001 {
    fn id(&self) -> &'static str {
        "0001-rename-tickets-to-work"
    }

    fn description(&self) -> &'static str {
        "Rename tickets/work-item terminology in meta/ and config files"
    }
}

impl Migration for Migration0001 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let root = ctx.root().to_path_buf();
        let config_team = root.join(".claude/accelerator.md");
        let config_local = root.join(".claude/accelerator.local.md");

        for cfg in [&config_team, &config_local] {
            if let Some(content) = ctx.read(cfg)? {
                if document::parse(&content).is_err() {
                    return Err(MigrationError::new(format!(
                        "Error: malformed frontmatter in {} — cannot proceed.\n\
                         Fix the config file and re-run /accelerator:migrate.",
                        cfg.display()
                    )));
                }
            }
        }

        let pinned_tickets =
            ctx.config_value("paths.tickets").unwrap_or_default();
        let tickets_is_default =
            pinned_tickets.is_empty() || pinned_tickets == "meta/tickets";
        let tickets_dir = if tickets_is_default {
            root.join("meta/tickets")
        } else {
            root.join(&pinned_tickets)
        };

        let pinned_review_tickets =
            ctx.config_value("paths.review_tickets").unwrap_or_default();
        let review_tickets_is_default = pinned_review_tickets.is_empty()
            || pinned_review_tickets == "meta/reviews/tickets";
        let review_tickets_dir = if review_tickets_is_default {
            root.join("meta/reviews/tickets")
        } else {
            root.join(&pinned_review_tickets)
        };

        let work_dir = root.join("meta/work");
        let review_work_dir = root.join("meta/reviews/work");

        for file in ctx.list_md_files(&tickets_dir)? {
            if let Some(content) = ctx.read(&file)? {
                if let Some(rewritten) = rewrite_ticket_frontmatter(&content) {
                    ctx.write(&file, &rewritten)?;
                }
            }
        }

        if tickets_is_default {
            ctx.merge_move(&tickets_dir, &work_dir)?;
        }
        if review_tickets_is_default {
            ctx.merge_move(&review_tickets_dir, &review_work_dir)?;
        }

        for cfg in [&config_team, &config_local] {
            if let Some(content) = ctx.read(cfg)? {
                let rewritten = rewrite_config(&content);
                if rewritten != content {
                    ctx.write(cfg, &rewritten)?;
                }
            }
        }

        Ok(ApplyOutcome::Applied)
    }
}

fn rewrite_ticket_frontmatter(content: &str) -> Option<String> {
    let has_ticket_id =
        content.lines().any(|line| line.starts_with("ticket_id:"));
    if !has_ticket_id {
        return None;
    }
    let has_work_item_id = content
        .lines()
        .any(|line| line.starts_with("work_item_id:"));
    if has_work_item_id {
        Some(filter_lines(content, |line| {
            !line.starts_with("ticket_id:")
        }))
    } else {
        Some(map_lines(content, |line| {
            line.strip_prefix("ticket_id:").map_or_else(
                || line.to_owned(),
                |rest| format!("work_item_id:{rest}"),
            )
        }))
    }
}

fn rewrite_config(content: &str) -> String {
    map_lines(content, rewrite_config_line)
}

fn rewrite_config_line(line: &str) -> String {
    if let Some(rewritten) = indented_rename_default(
        line,
        "tickets",
        "meta/tickets",
        "work: meta/work",
    ) {
        return rewritten;
    }
    if let Some(rewritten) = indented_rename(line, "tickets", "work") {
        return rewritten;
    }
    if let Some(rewritten) = indented_rename_default(
        line,
        "review_tickets",
        "meta/reviews/tickets",
        "review_work: meta/reviews/work",
    ) {
        return rewritten;
    }
    if let Some(rewritten) =
        indented_rename(line, "review_tickets", "review_work")
    {
        return rewritten;
    }
    if let Some(rewritten) = flat_rename_default(
        line,
        "paths.tickets",
        "meta/tickets",
        "paths.work: meta/work",
    ) {
        return rewritten;
    }
    if let Some(rewritten) = flat_rename(line, "paths.tickets", "paths.work") {
        return rewritten;
    }
    if let Some(rewritten) = flat_rename_default(
        line,
        "paths.review_tickets",
        "meta/reviews/tickets",
        "paths.review_work: meta/reviews/work",
    ) {
        return rewritten;
    }
    if let Some(rewritten) =
        flat_rename(line, "paths.review_tickets", "paths.review_work")
    {
        return rewritten;
    }
    if let Some(rewritten) = indented_rename(
        line,
        "ticket_revise_severity",
        "work_item_revise_severity",
    ) {
        return rewritten;
    }
    if let Some(rewritten) = indented_rename(
        line,
        "ticket_revise_major_count",
        "work_item_revise_major_count",
    ) {
        return rewritten;
    }
    if let Some(rewritten) =
        indented_rename(line, "min_lenses_ticket", "min_lenses_work_item")
    {
        return rewritten;
    }
    if let Some(rewritten) = flat_rename(
        line,
        "review.ticket_revise_severity",
        "review.work_item_revise_severity",
    ) {
        return rewritten;
    }
    if let Some(rewritten) = flat_rename(
        line,
        "review.ticket_revise_major_count",
        "review.work_item_revise_major_count",
    ) {
        return rewritten;
    }
    if let Some(rewritten) = flat_rename(
        line,
        "review.min_lenses_ticket",
        "review.min_lenses_work_item",
    ) {
        return rewritten;
    }
    line.to_owned()
}

#[cfg(test)]
mod tests {
    use super::{rewrite_config, rewrite_ticket_frontmatter};

    #[test]
    fn renames_ticket_id_when_work_item_id_absent() {
        let content = "---\nticket_id: 0001\n---\n\n# Foo\n";
        assert_eq!(
            rewrite_ticket_frontmatter(content),
            Some("---\nwork_item_id: 0001\n---\n\n# Foo\n".to_owned())
        );
    }

    #[test]
    fn drops_ticket_id_when_work_item_id_already_present() {
        let content =
            "---\nwork_item_id: 0001\nticket_id: 0001\n---\n\n# Foo\n";
        assert_eq!(
            rewrite_ticket_frontmatter(content),
            Some("---\nwork_item_id: 0001\n---\n\n# Foo\n".to_owned())
        );
    }

    #[test]
    fn leaves_files_without_ticket_id_untouched() {
        assert_eq!(
            rewrite_ticket_frontmatter("---\nwork_item_id: 0001\n---\n"),
            None
        );
    }

    #[test]
    fn rewrites_default_and_custom_config_values() {
        let content = "---\npaths:\n  tickets: meta/tickets\n  review_tickets: meta/custom\nreview:\n  ticket_revise_severity: major\n---\n";
        let rewritten = rewrite_config(content);
        assert_eq!(
            rewritten,
            "---\npaths:\n  work: meta/work\n  review_work: meta/custom\nreview:\n  work_item_revise_severity: major\n---\n"
        );
    }

    #[test]
    fn does_not_match_a_comment_mentioning_tickets() {
        let content = "# tickets: meta/tickets\n";
        assert_eq!(rewrite_config(content), content);
    }
}

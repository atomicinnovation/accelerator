//! Renames tickets terminology to work-item terminology.
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
                if ctx.parse_frontmatter(&content).is_err() {
                    return Err(MigrationError::new(format!(
                        "Error: malformed frontmatter in {} — cannot proceed.\n\
                         Fix the config file and re-run /accelerator:migrate.",
                        cfg.display()
                    )));
                }
            }
        }

        let pinned_tickets =
            ctx.config_value("paths.tickets")?.unwrap_or_default();
        let tickets_is_default =
            pinned_tickets.is_empty() || pinned_tickets == "meta/tickets";
        let tickets_dir = if tickets_is_default {
            root.join("meta/tickets")
        } else {
            root.join(&pinned_tickets)
        };

        let pinned_review_tickets = ctx
            .config_value("paths.review_tickets")?
            .unwrap_or_default();
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
#[allow(clippy::panic)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;

    use super::{rewrite_config, rewrite_ticket_frontmatter, Migration0001};
    use crate::ports::{
        CorpusIndex, DocTypeDir, MigrationContext, MigrationError,
    };
    use crate::registry::Migration;

    struct NoIndex;
    impl CorpusIndex for NoIndex {
        fn target_exists(&self, _target_type: &str, _target_id: &str) -> bool {
            false
        }
    }

    const UNPARSEABLE: &str = "---\nunparseable\n---\n";

    struct ConfigCtx {
        root: PathBuf,
        files: RefCell<HashMap<PathBuf, String>>,
        index: NoIndex,
    }

    impl ConfigCtx {
        fn with_team_config(content: &str) -> Self {
            let root = PathBuf::from("/repo");
            let files = HashMap::from([(
                root.join(".claude/accelerator.md"),
                content.to_owned(),
            )]);
            Self {
                root,
                files: RefCell::new(files),
                index: NoIndex,
            }
        }

        fn team_config(&self) -> Option<String> {
            self.files
                .borrow()
                .get(&self.root.join(".claude/accelerator.md"))
                .cloned()
        }
    }

    impl MigrationContext for ConfigCtx {
        fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
            Vec::new()
        }
        fn corpus_index(&self) -> &dyn CorpusIndex {
            &self.index
        }
        fn root(&self) -> &Path {
            &self.root
        }
        fn write(
            &self,
            path: &Path,
            content: &str,
        ) -> Result<(), MigrationError> {
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), content.to_owned());
            Ok(())
        }
        fn read(&self, path: &Path) -> Result<Option<String>, MigrationError> {
            Ok(self.files.borrow().get(path).cloned())
        }
        fn parse_frontmatter(
            &self,
            content: &str,
        ) -> Result<corpus::FrontmatterValue, MigrationError> {
            if content == UNPARSEABLE {
                return Err(MigrationError::new("invalid frontmatter YAML"));
            }
            Ok(corpus::FrontmatterValue::Mapping(corpus::Mapping::new()))
        }
    }

    #[test]
    fn a_config_the_context_cannot_parse_stops_the_migration() {
        let ctx = ConfigCtx::with_team_config(UNPARSEABLE);

        let Err(error) = Migration0001.apply(&ctx) else {
            panic!("an unparseable config must stop the migration");
        };

        assert_eq!(
            error.to_string(),
            "Error: malformed frontmatter in /repo/.claude/accelerator.md — \
             cannot proceed.\n\
             Fix the config file and re-run /accelerator:migrate."
        );
        assert_eq!(ctx.team_config().as_deref(), Some(UNPARSEABLE));
    }

    #[test]
    fn a_config_the_context_parses_is_rewritten() -> Result<(), MigrationError>
    {
        let ctx = ConfigCtx::with_team_config(
            "---\npaths:\n  tickets: meta/tickets\n---\n",
        );

        Migration0001.apply(&ctx)?;

        assert_eq!(
            ctx.team_config().as_deref(),
            Some("---\npaths:\n  work: meta/work\n---\n")
        );
        Ok(())
    }

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

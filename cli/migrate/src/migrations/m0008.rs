//! Re-renders every corpus document and `.accelerator/config.md` through the
//! canonical frontmatter emitter.
//!
//! The transformation is "re-render": the context's canonical emitter is the
//! single definition of canonical form, so this migration never re-encodes the
//! quoting predicate. Every rewrite is guarded against value change — a
//! re-parsed value tree that differs from the original, or a re-rendered
//! `meta/` file that still fails structural validation, aborts the run and
//! leaves the ledger unmarked, so VCS revert (and a re-run after the fix) is
//! the recovery path. Dropped inline comments and CRLF endings — which this
//! repository's corpus does not carry, but a downstream one might — are
//! surfaced per file as `0008-LOSSY` diagnostics and proceed, since blocking
//! adoption over a comment would be worse than the visible loss.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0008;

impl MigrationMeta for Migration0008 {
    fn id(&self) -> &'static str {
        "0008-canonical-frontmatter-quoting"
    }

    fn description(&self) -> &'static str {
        "Re-render every meta/ document and .accelerator/config.md through \
         the canonical frontmatter emitter — every string double-quoted, \
         integers/booleans/null bare."
    }
}

impl Migration for Migration0008 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let mut rewritten = 0usize;
        let mut lossy = 0usize;
        let mut pre_migration = Vec::new();
        for (path, kind) in enumerate(ctx)? {
            let Some(original) = ctx.read(&path)? else {
                continue;
            };
            if kind == FileKind::Meta {
                pre_migration.push((path.clone(), original.clone()));
            }
            if let Rewrite::Written { loss } =
                canonicalise(ctx, &path, kind, &original)?
            {
                rewritten += 1;
                if let Some(reason) = loss {
                    lossy += 1;
                    eprintln!("0008-LOSSY {}: {reason}", path.display());
                }
            }
        }
        let realigned = ctx.realign_sync_baseline(&pre_migration)?;
        eprintln!(
            "0008: {rewritten} file(s) re-rendered, {lossy} with dropped \
             comments/CRLF, {realigned} sync baseline(s) realigned — revert \
             this migration commit to recover"
        );
        Ok(ApplyOutcome::Applied)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Meta,
    Config,
}

enum Rewrite {
    Unchanged,
    Written { loss: Option<String> },
}

/// Every `meta/` doc-type file plus `.accelerator/config.md`.
///
/// Walks the full `doc_type_dirs()` set directly rather than the
/// linkage-filtered corpus table, so a local-only doc type with no linkage
/// type name is still reached.
fn enumerate(
    ctx: &dyn MigrationContext,
) -> Result<Vec<(PathBuf, FileKind)>, MigrationError> {
    let mut meta = BTreeSet::new();
    for dir in ctx.doc_type_dirs() {
        for file in ctx.list_md_files(&dir.dir)? {
            meta.insert(file);
        }
    }
    let mut files: Vec<(PathBuf, FileKind)> = meta
        .into_iter()
        .map(|path| (path, FileKind::Meta))
        .collect();
    let config = ctx.root().join(".accelerator/config.md");
    if ctx.read(&config)?.is_some() {
        files.push((config, FileKind::Config));
    }
    Ok(files)
}

fn canonicalise(
    ctx: &dyn MigrationContext,
    path: &Path,
    kind: FileKind,
    original: &str,
) -> Result<Rewrite, MigrationError> {
    let rendered = ctx
        .render_canonical(original)
        .map_err(|error| at(path, &error.to_string()))?;

    let before = ctx
        .parse_frontmatter(original)
        .map_err(|error| at(path, &error.to_string()))?;
    let after = ctx
        .parse_frontmatter(&rendered)
        .map_err(|error| at(path, &error.to_string()))?;
    if before != after {
        return Err(at(path, "re-rendering changed a frontmatter value"));
    }

    if kind == FileKind::Meta {
        let frontmatter = ctx
            .frontmatter_text(&rendered)
            .map_err(|error| at(path, &error.to_string()))?;
        if let Some(violation) =
            corpus::frontmatter_validation::validate_file(&frontmatter).first()
        {
            return Err(at(
                path,
                &format!(
                    "re-rendered frontmatter still violates the standard: \
                     {violation}"
                ),
            ));
        }
    }

    if rendered == original {
        return Ok(Rewrite::Unchanged);
    }
    let loss = detect_loss(ctx, original);
    ctx.write(path, &rendered)?;
    Ok(Rewrite::Written { loss })
}

fn at(path: &Path, message: &str) -> MigrationError {
    MigrationError::new(format!(
        "0008: {}: {message} — revert this migration commit to recover",
        path.display()
    ))
}

/// A tractable, testable predicate on the original bytes: an inline `#`
/// comment, a CRLF ending in the frontmatter, or content that did not
/// round-trip through UTF-8 (surfaced as the replacement character on read).
fn detect_loss(ctx: &dyn MigrationContext, original: &str) -> Option<String> {
    if original.contains('\u{FFFD}') {
        return Some("non-UTF-8 bytes replaced on read".to_owned());
    }
    let frontmatter = ctx.frontmatter_text(original).ok()?;
    if frontmatter.contains('\r') {
        return Some("CRLF line ending in frontmatter".to_owned());
    }
    if has_frontmatter_comment(&frontmatter) {
        return Some("inline frontmatter comment".to_owned());
    }
    None
}

fn has_frontmatter_comment(frontmatter: &str) -> bool {
    frontmatter.lines().any(|line| {
        let bytes = line.as_bytes();
        let mut double = false;
        let mut single = false;
        for (index, &byte) in bytes.iter().enumerate() {
            match byte {
                b'"' if !single => double = !double,
                b'\'' if !double => single = !single,
                b'#' if !double
                    && !single
                    && (index == 0
                        || bytes[index - 1] == b' '
                        || bytes[index - 1] == b'\t') =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;

    use super::{detect_loss, Migration, Migration0008};
    use crate::ports::{
        CorpusIndex, DocTypeDir, MigrationContext, MigrationError,
    };
    use crate::registry::ApplyOutcome;

    struct NoIndex;
    impl CorpusIndex for NoIndex {
        fn target_exists(&self, _target_type: &str, _target_id: &str) -> bool {
            false
        }
    }

    /// Renders through an explicit table, and reads a frontmatter value as
    /// its fenced text without double quotes or comment lines, so quoting a
    /// string or dropping a comment keeps the value and retyping one does
    /// not.
    struct TestCtx {
        root: PathBuf,
        dirs: Vec<DocTypeDir>,
        files: RefCell<HashMap<PathBuf, String>>,
        renders: HashMap<String, Result<String, String>>,
        writes: RefCell<Vec<PathBuf>>,
        realigned_from: RefCell<Vec<(PathBuf, String)>>,
        index: NoIndex,
    }

    impl TestCtx {
        fn new() -> Self {
            Self {
                root: PathBuf::from("/repo"),
                dirs: vec![DocTypeDir {
                    doc_type: "work-item".to_owned(),
                    dir: PathBuf::from("/repo/meta/work"),
                }],
                files: RefCell::new(HashMap::new()),
                renders: HashMap::new(),
                writes: RefCell::new(Vec::new()),
                realigned_from: RefCell::new(Vec::new()),
                index: NoIndex,
            }
        }

        fn with_file(self, path: &str, content: &str) -> Self {
            self.files
                .borrow_mut()
                .insert(PathBuf::from(path), content.to_owned());
            self
        }

        fn rendering(mut self, original: &str, rendered: &str) -> Self {
            self.renders
                .insert(original.to_owned(), Ok(rendered.to_owned()));
            self
        }

        fn failing_to_render(mut self, original: &str, error: &str) -> Self {
            self.renders
                .insert(original.to_owned(), Err(error.to_owned()));
            self
        }

        fn content(&self, path: &str) -> Option<String> {
            self.files.borrow().get(Path::new(path)).cloned()
        }
    }

    fn fenced_text(content: &str) -> Result<String, MigrationError> {
        let Some(rest) = content.strip_prefix("---\n") else {
            return Ok(String::new());
        };
        rest.find("---\n")
            .map(|end| rest[..end].to_owned())
            .ok_or_else(|| {
                MigrationError::new("unterminated frontmatter block")
            })
    }

    impl MigrationContext for TestCtx {
        fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
            self.dirs.clone()
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
            self.writes.borrow_mut().push(path.to_path_buf());
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), content.to_owned());
            Ok(())
        }
        fn read(&self, path: &Path) -> Result<Option<String>, MigrationError> {
            Ok(self.files.borrow().get(path).cloned())
        }
        fn list_md_files(
            &self,
            dir: &Path,
        ) -> Result<Vec<PathBuf>, MigrationError> {
            let mut files: Vec<PathBuf> = self
                .files
                .borrow()
                .keys()
                .filter(|path| path.starts_with(dir))
                .cloned()
                .collect();
            files.sort();
            Ok(files)
        }
        fn realign_sync_baseline(
            &self,
            pre_migration: &[(PathBuf, String)],
        ) -> Result<usize, MigrationError> {
            self.realigned_from
                .borrow_mut()
                .extend(pre_migration.iter().cloned());
            Ok(0)
        }
        fn parse_frontmatter(
            &self,
            content: &str,
        ) -> Result<corpus::FrontmatterValue, MigrationError> {
            let value: String = fenced_text(content)?
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .map(|line| line.replace('"', "") + "\n")
                .collect();
            Ok(corpus::FrontmatterValue::Scalar(corpus::Scalar::String(
                value,
            )))
        }
        fn frontmatter_text(
            &self,
            content: &str,
        ) -> Result<String, MigrationError> {
            fenced_text(content)
        }
        fn render_canonical(
            &self,
            content: &str,
        ) -> Result<String, MigrationError> {
            self.renders
                .get(content)
                .cloned()
                .unwrap_or_else(|| Ok(content.to_owned()))
                .map_err(MigrationError::new)
        }
    }

    fn apply(ctx: &TestCtx) -> Result<ApplyOutcome, MigrationError> {
        Migration0008.apply(ctx)
    }

    /// A canonical, structurally-complete work item, so the re-render's own
    /// `validate_file` gate has no unrelated violation to trip on.
    fn canonical_work_item(extra_lines: &str) -> String {
        format!(
            "---\ntype: \"work-item\"\nid: \"0001\"\ntitle: \"Bare\"\n\
             date: \"2026-01-01T00:00:00+00:00\"\nauthor: \"Toby\"\ntags: []\n\
             last_updated: \"2026-01-01T00:00:00+00:00\"\n\
             last_updated_by: \"Toby\"\nschema_version: 1\nstatus: \"draft\"\n\
             kind: \"feature\"\npriority: \"normal\"\n{extra_lines}---\nbody\n"
        )
    }

    fn bare_title(content: &str) -> String {
        content.replace("title: \"Bare\"", "title: Bare")
    }

    #[test]
    fn a_re_render_is_written_in_place() {
        let rendered = canonical_work_item("");
        let original = bare_title(&rendered);
        let ctx = TestCtx::new()
            .with_file("/repo/meta/work/0001-x.md", &original)
            .rendering(&original, &rendered);

        apply(&ctx).expect("apply");

        assert_eq!(ctx.content("/repo/meta/work/0001-x.md"), Some(rendered));
    }

    #[test]
    fn an_already_canonical_file_is_not_written() {
        let ctx = TestCtx::new()
            .with_file("/repo/meta/work/0001-x.md", &canonical_work_item(""));

        apply(&ctx).expect("apply");

        assert!(ctx.writes.borrow().is_empty());
    }

    #[test]
    fn a_value_changing_re_render_aborts_and_writes_nothing() {
        let original = canonical_work_item("ratio: 1.0\n");
        let ctx = TestCtx::new()
            .with_file("/repo/meta/work/0004-f.md", &original)
            .rendering(&original, &canonical_work_item("ratio: 1\n"));

        let Err(error) = apply(&ctx) else {
            panic!("a value change must abort");
        };

        assert_eq!(
            error.to_string(),
            "0008: /repo/meta/work/0004-f.md: re-rendering changed a \
             frontmatter value — revert this migration commit to recover"
        );
        assert!(ctx.writes.borrow().is_empty());
    }

    #[test]
    fn a_render_failure_aborts_naming_the_file() {
        let original = canonical_work_item("");
        let ctx = TestCtx::new()
            .with_file("/repo/meta/work/0001-x.md", &original)
            .failing_to_render(&original, "invalid frontmatter YAML: bad");

        let Err(error) = apply(&ctx) else {
            panic!("a render failure must abort");
        };

        assert_eq!(
            error.to_string(),
            "0008: /repo/meta/work/0001-x.md: invalid frontmatter YAML: bad \
             — revert this migration commit to recover"
        );
    }

    #[test]
    fn a_re_rendered_meta_file_that_still_violates_the_standard_aborts() {
        let rendered = "---\ntype: \"work-item\"\ntitle: \"Bare\"\n---\nbody\n";
        let original = bare_title(rendered);
        let ctx = TestCtx::new()
            .with_file("/repo/meta/work/0001-x.md", &original)
            .rendering(&original, rendered);

        let Err(error) = apply(&ctx) else {
            panic!("an invalid re-render must abort");
        };

        assert!(
            error.to_string().contains(
                "re-rendered frontmatter still violates the standard"
            ),
            "{error}"
        );
        assert!(ctx.writes.borrow().is_empty());
    }

    #[test]
    fn config_frontmatter_is_re_rendered_without_the_corpus_standard() {
        let original = "---\nvisualiser:\n  theme: dark\n---\nbody\n";
        let rendered = "---\nvisualiser:\n  theme: \"dark\"\n---\nbody\n";
        let ctx = TestCtx::new()
            .with_file("/repo/.accelerator/config.md", original)
            .rendering(original, rendered);

        apply(&ctx).expect("apply");

        assert_eq!(
            ctx.content("/repo/.accelerator/config.md").as_deref(),
            Some(rendered)
        );
    }

    #[test]
    fn only_meta_files_reach_baseline_realignment_with_their_original_bytes() {
        let rendered = canonical_work_item("");
        let original = bare_title(&rendered);
        let ctx = TestCtx::new()
            .with_file("/repo/meta/work/0001-x.md", &original)
            .with_file("/repo/.accelerator/config.md", "---\nk: v\n---\n")
            .rendering(&original, &rendered);

        apply(&ctx).expect("apply");

        assert_eq!(
            *ctx.realigned_from.borrow(),
            vec![(PathBuf::from("/repo/meta/work/0001-x.md"), original)]
        );
    }

    #[test]
    fn a_lossy_file_is_still_written_and_the_run_succeeds() {
        let rendered = canonical_work_item("");
        let original =
            canonical_work_item("# a standalone frontmatter comment\n");
        let ctx = TestCtx::new()
            .with_file("/repo/meta/work/0006-c.md", &original)
            .rendering(&original, &rendered);

        let outcome = apply(&ctx).expect("apply");

        assert!(matches!(outcome, ApplyOutcome::Applied));
        assert_eq!(ctx.content("/repo/meta/work/0006-c.md"), Some(rendered));
    }

    #[test]
    fn enumeration_reaches_a_doc_type_with_no_linkage_type_name() {
        let mut ctx = TestCtx::new();
        ctx.dirs.push(DocTypeDir {
            doc_type: "local-only-not-a-linkage-type".to_owned(),
            dir: PathBuf::from("/repo/meta/local"),
        });
        let rendered = canonical_work_item("");
        let original = bare_title(&rendered);
        let ctx = ctx
            .with_file("/repo/meta/local/x.md", &original)
            .rendering(&original, &rendered);

        apply(&ctx).expect("apply");

        assert_eq!(ctx.content("/repo/meta/local/x.md"), Some(rendered));
    }

    #[test]
    fn a_clean_lf_document_emits_no_loss_diagnostic() {
        let clean = "---\ntype: work-item\nid: \"0005\"\ntitle: Bare\n\
             status: draft\nschema_version: 1\n---\nbody\n";
        assert_eq!(detect_loss(&TestCtx::new(), clean), None);
    }

    #[test]
    fn a_comment_a_crlf_and_a_replacement_char_each_report_loss() {
        let ctx = TestCtx::new();
        let comment = "---\ntype: work-item\ntitle: T # inline\n---\nbody\n";
        let crlf = "---\ntype: work-item\r\ntitle: T\r\n---\nbody\r\n";
        let non_utf8 = "---\ntype: work-item\ntitle: \u{FFFD}\n---\nbody\n";
        assert_eq!(
            detect_loss(&ctx, comment).as_deref(),
            Some("inline frontmatter comment")
        );
        assert_eq!(
            detect_loss(&ctx, crlf).as_deref(),
            Some("CRLF line ending in frontmatter")
        );
        assert_eq!(
            detect_loss(&ctx, non_utf8).as_deref(),
            Some("non-UTF-8 bytes replaced on read")
        );
    }

    #[test]
    fn a_hash_inside_quotes_is_not_a_comment() {
        let quoted = "---\ntitle: \"Issue #12\"\nalt: 'see #3'\n---\nbody\n";
        assert_eq!(detect_loss(&TestCtx::new(), quoted), None);
    }

    #[test]
    fn content_whose_frontmatter_cannot_be_split_reports_no_loss() {
        let unterminated = "---\ntitle: T # inline\n";
        assert_eq!(detect_loss(&TestCtx::new(), unterminated), None);
    }
}

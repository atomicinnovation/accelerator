//! Re-renders every corpus document and `.accelerator/config.md` through the
//! canonical frontmatter emitter.
//!
//! The transformation is "re-render": the emitter (`document::render`) is the
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
    let rendered =
        render_canonical(original).map_err(|error| at(path, &error))?;

    let before = document::parse(original)
        .map_err(|error| at(path, &error.to_string()))?;
    let after = document::parse(&rendered)
        .map_err(|error| at(path, &error.to_string()))?;
    if before != after {
        return Err(at(path, "re-rendering changed a frontmatter value"));
    }

    if kind == FileKind::Meta {
        let frontmatter = document::split(&rendered)
            .map_err(|error| at(path, &error.to_string()))?
            .frontmatter;
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
    let loss = detect_loss(original);
    ctx.write(path, &rendered)?;
    Ok(Rewrite::Written { loss })
}

fn render_canonical(content: &str) -> Result<String, String> {
    let frontmatter = document::parse(content).map_err(|e| e.to_string())?;
    document::render(Some(content), &frontmatter).map_err(|e| e.to_string())
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
fn detect_loss(original: &str) -> Option<String> {
    if original.contains('\u{FFFD}') {
        return Some("non-UTF-8 bytes replaced on read".to_owned());
    }
    let frontmatter = document::split(original).ok()?.frontmatter;
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

    use super::{detect_loss, render_canonical, Migration, Migration0008};
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

    struct TestCtx {
        root: PathBuf,
        dirs: Vec<DocTypeDir>,
        files: RefCell<HashMap<PathBuf, String>>,
        index: NoIndex,
        realigned: usize,
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
                index: NoIndex,
                realigned: 0,
            }
        }

        fn with_file(self, path: &str, content: &str) -> Self {
            self.files
                .borrow_mut()
                .insert(PathBuf::from(path), content.to_owned());
            self
        }

        fn content(&self, path: &str) -> Option<String> {
            self.files.borrow().get(Path::new(path)).cloned()
        }
    }

    impl MigrationContext for TestCtx {
        fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
            self.dirs.clone()
        }
        fn revision(&self) -> Option<String> {
            None
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
            _pre_migration: &[(PathBuf, String)],
        ) -> Result<usize, MigrationError> {
            Ok(self.realigned)
        }
    }

    fn apply(ctx: &TestCtx) -> Result<ApplyOutcome, MigrationError> {
        Migration0008.apply(ctx)
    }

    /// A structurally-complete work item, so the re-render's own
    /// `validate_file` gate has no unrelated base-field violation to trip on.
    fn valid_work_item(extra_lines: &str) -> String {
        format!(
            "---\ntype: work-item\nid: \"0001\"\ntitle: Bare\n\
             date: \"2026-01-01T00:00:00+00:00\"\nauthor: Toby\ntags: []\n\
             last_updated: \"2026-01-01T00:00:00+00:00\"\n\
             last_updated_by: Toby\nschema_version: 1\nstatus: draft\n\
             kind: feature\npriority: normal\n{extra_lines}---\nbody\n"
        )
    }

    #[test]
    fn a_bare_document_is_re_rendered_with_every_string_quoted() {
        let ctx = TestCtx::new()
            .with_file("/repo/meta/work/0001-x.md", &valid_work_item(""));
        apply(&ctx).expect("apply");
        let content = ctx.content("/repo/meta/work/0001-x.md").expect("file");
        assert!(content.contains("title: \"Bare\""), "{content}");
        assert!(content.contains("status: \"draft\""), "{content}");
        assert!(content.contains("schema_version: 1"), "{content}");
        assert!(content.contains("body\n"), "{content}");
    }

    #[test]
    fn a_block_linkage_sequence_with_colons_reflows_to_quoted_flow() {
        let fixture = valid_work_item(
            "relates_to:\n  - work-item:0194\n  - adr:ADR-0034\n",
        );
        let ctx =
            TestCtx::new().with_file("/repo/meta/work/0002-y.md", &fixture);
        apply(&ctx).expect("apply");
        let content = ctx.content("/repo/meta/work/0002-y.md").expect("file");
        assert!(
            content
                .contains("relates_to: [\"work-item:0194\", \"adr:ADR-0034\"]"),
            "{content}"
        );
    }

    #[test]
    fn re_rendering_is_a_byte_level_fixed_point() {
        let original = "---\ntype: work-item\nid: \"0003\"\n\
             title: A long title that runs well past eighty columns to prove \
             no block scalar refold happens here at all\n\
             tags: [alpha, beta]\nschema_version: 1\n---\nbody\n";
        let once = render_canonical(original).expect("first");
        let twice = render_canonical(&once).expect("second");
        assert_eq!(once, twice, "second pass must be byte-identical");
    }

    #[test]
    fn config_untyped_frontmatter_quotes_strings_and_leaves_integers_bare() {
        let ctx = TestCtx::new().with_file(
            "/repo/.accelerator/config.md",
            "---\nvisualiser:\n  port: 8080\n  theme: dark\n\
             tags:\n  - one\n  - two\n---\nbody\n",
        );
        apply(&ctx).expect("apply");
        let content =
            ctx.content("/repo/.accelerator/config.md").expect("file");
        assert!(content.contains("port: 8080"), "{content}");
        assert!(content.contains("theme: \"dark\""), "{content}");
        assert!(content.contains("tags: [\"one\", \"two\"]"), "{content}");
    }

    #[test]
    fn a_value_retyping_re_render_aborts_and_writes_nothing() {
        let fixture = valid_work_item("ratio: 1.0\n");
        let ctx =
            TestCtx::new().with_file("/repo/meta/work/0004-f.md", &fixture);
        let Err(error) = apply(&ctx) else {
            panic!("float coercion must abort");
        };
        assert!(error.to_string().contains("0004-f.md"), "{error}");
        let content = ctx.content("/repo/meta/work/0004-f.md").expect("file");
        assert!(content.contains("ratio: 1.0"), "the file must be untouched");
    }

    #[test]
    fn a_clean_lf_document_emits_no_loss_diagnostic() {
        let clean = "---\ntype: work-item\nid: \"0005\"\ntitle: Bare\n\
             status: draft\nschema_version: 1\n---\nbody\n";
        assert_eq!(detect_loss(clean), None);
    }

    #[test]
    fn a_comment_a_crlf_and_a_replacement_char_each_report_loss() {
        let comment = "---\ntype: work-item\ntitle: T # inline\n---\nbody\n";
        let crlf = "---\r\ntype: work-item\r\ntitle: T\r\n---\r\nbody\r\n";
        let non_utf8 = "---\ntype: work-item\ntitle: \u{FFFD}\n---\nbody\n";
        assert!(detect_loss(comment).is_some());
        assert!(detect_loss(crlf).is_some());
        assert!(detect_loss(non_utf8).is_some());
    }

    #[test]
    fn a_lossy_file_is_still_written_and_the_run_succeeds() {
        let fixture = valid_work_item("# a standalone frontmatter comment\n");
        let ctx =
            TestCtx::new().with_file("/repo/meta/work/0006-c.md", &fixture);
        let outcome = apply(&ctx).expect("apply");
        assert!(matches!(outcome, ApplyOutcome::Applied));
        let content = ctx.content("/repo/meta/work/0006-c.md").expect("file");
        assert!(!content.contains("standalone"), "comment must be dropped");
        assert!(content.contains("title: \"Bare\""), "{content}");
    }

    #[test]
    fn enumeration_reaches_a_doc_type_with_no_linkage_type_name() {
        let mut ctx = TestCtx::new();
        ctx.dirs.push(DocTypeDir {
            doc_type: "local-only-not-a-linkage-type".to_owned(),
            dir: PathBuf::from("/repo/meta/local"),
        });
        let ctx = ctx.with_file("/repo/meta/local/x.md", &valid_work_item(""));
        apply(&ctx).expect("apply");
        let content = ctx.content("/repo/meta/local/x.md").expect("file");
        assert!(content.contains("title: \"Bare\""), "{content}");
    }
}

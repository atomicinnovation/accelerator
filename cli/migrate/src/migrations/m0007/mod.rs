//! The mechanical base-field/identity/provenance/status rewrite plus the
//! interactive body-section typed-linkage contract.
//!
//! The mechanical work (precondition prepass, fence-less backfill, the
//! single-pass rewrite, structural self-validation) all happens inside
//! [`InteractiveMigration::emit_transformations`] — the only callback the
//! engine calls unconditionally, before any prompting. There is no
//! `Result`-returning hook to signal a mechanical failure from there, so a
//! failure is carried as a single sentinel `Transformation` whose
//! `evaluate_predicate` returns `Fail` — the same "abort the whole
//! migration with `[id] message`" path a real predicate failure takes.
//!
//! The whole-corpus, post-interactive-apply referential-integrity check
//! runs via [`InteractiveMigration::finalise`] — the engine calls it once,
//! unconditionally, after every transformation this run decided has been
//! applied (or immediately if there were none to begin with), not threaded
//! through any one transformation's own callback.

mod backfill;
mod fence;
mod linkage_value;
mod merge;
mod prepass;
mod quote;
mod rewrite;
mod schema;
mod status_map;

use std::path::PathBuf;

use corpus::doc_type::DocTypeKey;
use corpus::linkage::Band;

use crate::interactive::Decision;
use crate::interactive::InteractiveMigration;
use crate::interactive::PredicateOutcome;
use crate::interactive::Transformation;
use crate::ports::DocTypeDir;
use crate::ports::MigrationContext;
use crate::registry::MigrationMeta;

const MECHANICAL_FAIL: &str = "__mechanical_fail__";

pub struct Migration0007;

impl MigrationMeta for Migration0007 {
    fn id(&self) -> &'static str {
        "0007-unify-meta-corpus-frontmatter"
    }

    fn description(&self) -> &'static str {
        "Unify the meta/ corpus to the ADR-0033/0034 schema — base fields, \
         identity, provenance, status reconciliation, fence-less backfill, \
         and (interactive) body-section typed linkage."
    }
}

/// The corpus-relative table `corpus::linkage`'s text-matching functions
/// need (`path_roots`/`extract_doc_paths` scan prose for repo-relative
/// tokens like `meta/work/0042.md`, never an absolute filesystem path) —
/// `ctx.doc_type_dirs()` returns directories already joined against the
/// project root (the `MigrationContext` contract, matching what
/// `ctx.list_md_files` needs for real filesystem walking), so this strips
/// that root back off before the dirs reach any text-matching call.
fn linkage_table(
    dirs: &[DocTypeDir],
    root: &std::path::Path,
) -> Vec<(DocTypeKey, PathBuf)> {
    dirs.iter()
        .filter_map(|dir| {
            DocTypeKey::from_linkage_type_name(&dir.doc_type).map(|key| {
                let relative = dir.dir.strip_prefix(root).unwrap_or(&dir.dir);
                (key, relative.to_path_buf())
            })
        })
        .collect()
}

fn corpus_files(
    ctx: &dyn MigrationContext,
    table: &[(DocTypeKey, PathBuf)],
) -> Result<Vec<(String, String)>, String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for (_, dir) in table {
        paths.extend(
            ctx.list_md_files(&ctx.root().join(dir))
                .map_err(|error| error.to_string())?,
        );
    }
    paths.sort();
    paths.dedup();

    let mut files = Vec::new();
    for path in paths {
        let Some(content) =
            ctx.read(&path).map_err(|error| error.to_string())?
        else {
            continue;
        };
        let relpath = path
            .strip_prefix(ctx.root())
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        files.push((relpath, content));
    }
    Ok(files)
}

fn mechanical_fail(message: impl Into<String>) -> Vec<Transformation> {
    vec![Transformation {
        key: "__mechanical__".to_owned(),
        path: String::new(),
        anchor: String::new(),
        proposed: message.into(),
        predicate_value: MECHANICAL_FAIL.to_owned(),
        display: String::new(),
        extras: Vec::new(),
    }]
}

fn run_mechanical(
    ctx: &dyn MigrationContext,
) -> Result<Vec<Transformation>, String> {
    let table = linkage_table(&ctx.doc_type_dirs(), ctx.root());
    let files = corpus_files(ctx, &table)?;

    let refusals = prepass::precondition_prepass(&files, &table);
    if !refusals.is_empty() {
        for refusal in &refusals {
            eprintln!("{refusal}");
        }
        return Err(
            "precondition pre-pass refused — zero files mutated — resolve \
             the refusals above (or revert meta/ via your VCS), then re-run"
                .to_owned(),
        );
    }

    let repo_name = ctx
        .root()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    for (relpath, content) in &files {
        if fence::has_strict_fence(content) {
            continue;
        }
        let (backfilled, diagnostics) =
            backfill::backfill(content, relpath, &table, &repo_name);
        for diagnostic in &diagnostics {
            eprintln!("{diagnostic}");
        }
        ctx.write(&ctx.root().join(relpath), &backfilled)
            .map_err(|error| error.to_string())?;
    }

    let files = corpus_files(ctx, &table)?;
    let mut refuse_or_malformed = 0usize;
    for (relpath, content) in &files {
        let input = rewrite::RewriteInput {
            file_display: relpath,
            relpath,
            repo_name: &repo_name,
            table: &table,
        };
        let (rewritten, diagnostics) = rewrite::rewrite(content, &input);
        for diagnostic in &diagnostics {
            eprintln!("{diagnostic}");
            if diagnostic.starts_with("0007-REFUSE")
                || diagnostic.starts_with("0007-MALFORMED")
            {
                refuse_or_malformed += 1;
            }
        }
        if &rewritten != content {
            ctx.write(&ctx.root().join(relpath), &rewritten)
                .map_err(|error| error.to_string())?;
        }
    }
    if refuse_or_malformed > 0 {
        return Err(format!(
            "{refuse_or_malformed} REFUSE/MALFORMED — failing — revert \
             meta/ via your VCS to recover, then re-run"
        ));
    }

    let files = corpus_files(ctx, &table)?;
    let abs: Vec<PathBuf> =
        files.iter().map(|(rel, _)| ctx.root().join(rel)).collect();
    ctx.validate_frontmatter(&abs).map_err(|_| {
        "structural frontmatter validation failed after rewrite — revert \
         meta/ via your VCS to recover, then re-run"
            .to_owned()
    })?;

    let transformations = emit_linkage_transformations(ctx, &files, &table);
    Ok(transformations)
}

fn emit_linkage_transformations(
    ctx: &dyn MigrationContext,
    files: &[(String, String)],
    table: &[(DocTypeKey, PathBuf)],
) -> Vec<Transformation> {
    let index = ctx.corpus_index();
    let mut transformations = Vec::new();

    for (relpath, content) in files {
        let Some(linkage_type) = rewrite::resolve_type(content, relpath, table)
        else {
            continue;
        };
        for record in
            corpus::linkage::parse_document(linkage_type, content, table)
        {
            if record.key.is_empty() || record.target_ref.is_empty() {
                continue;
            }
            if record.band == Band::Resolved
                && !record.target_ref.starts_with("pr:")
            {
                if let Some((target_type, target_id)) =
                    record.target_ref.split_once(':')
                {
                    if !index.target_exists(target_type, target_id) {
                        eprintln!(
                            "0007-DIVERGE[reverse-orphan]: {relpath} — \
                             resolved {} target '{}' resolves to no \
                             artifact; skipped",
                            record.key, record.target_ref
                        );
                        continue;
                    }
                }
            }
            let band = record.band.as_str();
            transformations.push(Transformation {
                key: format!("{relpath}#{}", record.anchor),
                path: relpath.clone(),
                anchor: record.anchor.clone(),
                proposed: format!("{}={}", record.key, record.target_ref),
                predicate_value: band.to_owned(),
                display: format!(
                    "Proposed linkage: {}: \"{}\"\nSection anchor: {}\nBand: {band}",
                    record.key, record.target_ref, record.anchor
                ),
                extras: vec![
                    ("band".to_owned(), band.to_owned()),
                    ("linkage_key".to_owned(), record.key.clone()),
                ],
            });
        }
    }
    transformations
}

fn referential_error() -> String {
    "referential-integrity frontmatter validation failed after the \
     interactive apply — revert meta/ via your VCS to recover, then re-run"
        .to_owned()
}

impl InteractiveMigration for Migration0007 {
    fn emit_transformations(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Vec<Transformation> {
        match run_mechanical(ctx) {
            Ok(transformations) => transformations,
            Err(message) => mechanical_fail(message),
        }
    }

    fn evaluate_predicate(
        &self,
        transformation: &Transformation,
    ) -> PredicateOutcome {
        if transformation.predicate_value == MECHANICAL_FAIL {
            return PredicateOutcome::Fail(transformation.proposed.clone());
        }
        if transformation.predicate_value == "ambiguous" {
            PredicateOutcome::Prompt
        } else {
            PredicateOutcome::Mechanical
        }
    }

    fn validate_edit(
        &self,
        _transformation: &Transformation,
        value: &str,
    ) -> Result<(), String> {
        let Some((key, target)) = value.split_once('=') else {
            return Err("edit must be <linkage-key>=<doc-type:id>".to_owned());
        };
        if schema::linkage_cardinality(key).is_none() {
            return Err(format!("unknown linkage key '{key}'"));
        }
        if !corpus::frontmatter_validation::shape::is_well_formed(target) {
            return Err(format!(
                "target '{target}' is not a typed doc-type:id reference"
            ));
        }
        Ok(())
    }

    fn apply_decision(
        &self,
        transformation: &Transformation,
        decision: &Decision,
        ctx: &dyn MigrationContext,
    ) -> Result<(), String> {
        let value = match decision {
            Decision::Accept => Some(transformation.proposed.as_str()),
            Decision::Edit(value) => Some(value.as_str()),
            Decision::Skip => {
                unreachable!("the engine never calls apply_decision for a skip")
            }
        };
        if let Some(value) = value {
            if let Some((key, target)) = value.split_once('=') {
                if !key.is_empty() && !target.is_empty() {
                    apply_linkage(transformation, key, target, ctx)?;
                }
            }
        }
        Ok(())
    }

    fn finalise(&self, ctx: &dyn MigrationContext) -> Result<(), String> {
        ctx.validate_frontmatter(&[])
            .map_err(|_| referential_error())
    }
}

fn apply_linkage(
    transformation: &Transformation,
    key: &str,
    target: &str,
    ctx: &dyn MigrationContext,
) -> Result<(), String> {
    let abs = ctx.root().join(&transformation.path);
    let Some(content) = ctx.read(&abs).map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let cardinality =
        schema::linkage_cardinality(key).unwrap_or(schema::Cardinality::List);
    if cardinality == schema::Cardinality::Single {
        if let Some(existing) = fence::get(&content, key) {
            let existing = quote::semantic_inner(&existing);
            if !existing.is_empty() && existing != target {
                eprintln!(
                    "0007-DIVERGE[parent-conflict]: {} — {key} already \
                     '{existing}'; not overwriting with '{target}'",
                    transformation.path
                );
                return Ok(());
            }
        }
    }
    let merged = merge::merge(&content, key, target, cardinality);
    if merged != content {
        ctx.write(&abs, &merged)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

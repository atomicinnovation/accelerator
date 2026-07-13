//! The dir→type fact is declared once, in `DocTypeKey`. Two other surfaces
//! consume it — the bash doc-type registry and the 0007 rewrite awk — and both
//! are *matchers* resolved at runtime, not static tables to scrape. So this
//! suite executes them and asserts the mapping they produce equals the crate's.
//!
//! bash and awk are asserted present and hard-fail with a naming diagnostic;
//! Rust's harness has no skip primitive, so a silent early return would register
//! as a green PASS.

mod common;

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use common::{doc_type_table, require_script, TestError};
use corpus::DocTypeKey;

const RECORD_SEPARATOR: char = '\u{1e}';

static PROBES_WRITTEN: AtomicU64 = AtomicU64::new(0);

/// A representative filename per type, exercising each id-derivation arm: the
/// numeric-prefix arm, the ADR arm, the nested-manifest arm, and the whole-stem
/// default.
const fn probe_filename(kind: DocTypeKey) -> &'static str {
    match kind {
        DocTypeKey::WorkItems => "0030-target.md",
        DocTypeKey::Decisions => "ADR-0050-some-decision.md",
        DocTypeKey::DesignInventories => "2026-01-01-buttons/inventory.md",
        _ => "2026-05-13-0055-feature.md",
    }
}

/// One probe path per configured directory, so a re-pathed corpus still probes
/// the directories the config actually resolves.
fn probes(table: &[(DocTypeKey, PathBuf)]) -> Vec<(DocTypeKey, String)> {
    table
        .iter()
        .map(|(kind, dir)| {
            (
                *kind,
                format!("{}/{}", dir.display(), probe_filename(*kind)),
            )
        })
        .collect()
}

/// Runs the 0007 rewrite awk's `path_to_typed` over `paths`, feeding it a
/// doc-type table whose type names come from `DocTypeKey`.
fn awk_typed_refs(
    table: &[(DocTypeKey, PathBuf)],
    paths: &[String],
) -> Result<Vec<String>, TestError> {
    let frag =
        require_script("skills/config/migrate/scripts/frontmatter-frag.awk")?;
    let body = require_script(
        "skills/config/migrate/scripts/0007-frontmatter-rewrite.awk",
    )?;

    let mut rows = Vec::new();
    for (kind, dir) in table {
        let name = kind
            .linkage_type_name()
            .ok_or("a virtual type reached the awk table")?;
        rows.push(format!("{name}\t{}", dir.display()));
    }
    let channel = rows.join(&RECORD_SEPARATOR.to_string());

    let mut program = String::from("BEGIN {\n");
    for path in paths {
        writeln!(program, "  print path_to_typed(\"{path}\")")?;
    }
    program.push_str("}\n");

    let probe = std::env::temp_dir().join(format!(
        "corpus-doc-type-probe-{}-{}.awk",
        std::process::id(),
        PROBES_WRITTEN.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&probe, program)?;

    let output = Command::new("awk")
        .arg("-v")
        .arg(format!("doc_type_table={channel}"))
        .arg("-f")
        .arg(&frag)
        .arg("-f")
        .arg(&body)
        .arg("-f")
        .arg(&probe)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            format!("could not run awk (is awk present?): {error}")
        })?;
    fs::remove_file(&probe)?;

    if !output.status.success() {
        return Err(format!(
            "the 0007 rewrite awk failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::to_owned)
        .collect())
}

#[test]
fn every_non_virtual_type_is_registered_exactly_once() -> Result<(), TestError>
{
    let table = doc_type_table()?;
    let declared: Vec<DocTypeKey> = DocTypeKey::all()
        .into_iter()
        .filter(|kind| kind.linkage_type_name().is_some())
        .collect();

    for kind in &declared {
        let resolved = table.iter().filter(|(key, _)| key == kind).count();
        assert_eq!(
            resolved, 1,
            "{kind:?} resolves to {resolved} directories in the doc-type \
             registry; expected exactly one"
        );
    }

    assert_eq!(
        table.len(),
        declared.len(),
        "the registry resolves {} directories but the crate declares {} \
         non-virtual types",
        table.len(),
        declared.len()
    );
    Ok(())
}

#[test]
fn the_rewrite_awk_agrees_on_the_directory_to_type_mapping(
) -> Result<(), TestError> {
    let table = doc_type_table()?;
    let probes = probes(&table);
    let paths: Vec<String> =
        probes.iter().map(|(_, path)| path.clone()).collect();

    let expected: Vec<&str> = probes
        .iter()
        .map(|(kind, path)| {
            corpus::doc_type::infer(Path::new(path), &table)
                .and_then(DocTypeKey::linkage_type_name)
                .ok_or_else(|| {
                    format!("{kind:?} probe {path} resolved to no type")
                })
        })
        .collect::<Result<_, String>>()?;

    let actual: Vec<String> = awk_typed_refs(&table, &paths)?
        .iter()
        .map(|typed| {
            typed
                .split_once(':')
                .map_or_else(|| typed.clone(), |(name, _)| name.to_owned())
        })
        .collect();

    assert_eq!(
        actual, expected,
        "the 0007 rewrite awk and the crate disagree on dir→type\n  \
         probes: {paths:#?}"
    );
    assert_eq!(
        expected.len(),
        table.len(),
        "every configured directory must be probed"
    );
    Ok(())
}

/// Every id-derivation arm, across every configured directory: the work-item
/// numeric prefix, the ADR prefix, the design-inventory parent directory (a
/// nested manifest, whose basename is always `inventory`), and the whole-stem
/// default.
#[test]
fn the_rewrite_awk_agrees_on_every_id_arm() -> Result<(), TestError> {
    let table = doc_type_table()?;
    let probes = probes(&table);
    let paths: Vec<String> =
        probes.iter().map(|(_, path)| path.clone()).collect();

    let expected: Vec<String> = probes
        .iter()
        .map(|(kind, path)| {
            let (name, id) = corpus::linkage::resolve_path_target(path, &table)
                .ok_or_else(|| {
                    format!("{kind:?} probe {path} resolved to no target")
                })?;
            Ok(format!("{name}:{id}"))
        })
        .collect::<Result<_, String>>()?;

    let actual = awk_typed_refs(&table, &paths)?;

    assert_eq!(
        actual, expected,
        "the 0007 rewrite awk and the crate disagree on type:id\n  \
         probes: {paths:#?}"
    );
    Ok(())
}

//! The dir→type fact is declared once, in `DocTypeKey`. The bash
//! `config-defaults.sh` `PATH_KEYS` registry and `scripts/linkage-type-pairs.tsv`
//! each declare a related fact independently, so this suite cross-checks the
//! crate against them.
//!
//! The 0007 migration's own awk rewrite used to be cross-checked here too,
//! as a second independent dir→type matcher — removed once that awk was
//! retired in favour of the native `accelerator-migrate` port, which calls
//! `corpus::doc_type::infer`/`corpus::linkage::resolve_path_target` directly
//! rather than re-deriving the mapping; there is no longer a second surface
//! to disagree with.
//!
//! bash is asserted present and hard-fails with a naming diagnostic; Rust's
//! harness has no skip primitive, so a silent early return would register as
//! a green PASS.
#![cfg(feature = "bash-parity")]

mod common;

use std::fs;
use std::process::Command;

use common::{doc_type_table, require_file, TestError};
use corpus::DocTypeKey;

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

/// Every doc type's config key must exist in the bash config schema.
///
/// `config_path_key` is how `corpus-adapters` keys the resolved doc-paths map, so
/// a key renamed in `config-defaults.sh` without updating `DocTypeKey` would
/// silently drop that type from the table — the document would simply stop being
/// classified, with nothing to report it.
#[test]
fn every_config_path_key_exists_in_the_config_schema() -> Result<(), TestError>
{
    let defaults = require_file("scripts/config-defaults.sh")?;
    let output = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source {}; printf '%s\\n' \"${{PATH_KEYS[@]}}\"",
            defaults.display()
        ))
        .output()
        .map_err(|error| format!("could not read PATH_KEYS: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "config-defaults.sh failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let declared: Vec<String> = String::from_utf8(output.stdout)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    assert!(
        !declared.is_empty(),
        "config-defaults.sh declared no PATH_KEYS — the probe has broken"
    );

    for kind in DocTypeKey::all() {
        let Some(key) = kind.config_path_key() else {
            continue;
        };
        let qualified = format!("paths.{key}");
        assert!(
            declared.contains(&qualified),
            "{kind:?} claims the config key {qualified:?}, which the config \
             schema does not declare — the crate and the config have drifted"
        );
    }
    Ok(())
}

/// `corpus::linkage::TYPE_PAIRS` and `scripts/linkage-type-pairs.tsv` are the
/// same table written twice — bash reads the file at runtime, the crate compiles
/// it in. Nothing but this test stops them drifting apart.
#[test]
fn the_type_pair_table_matches_the_tsv() -> Result<(), TestError> {
    let tsv = require_file("scripts/linkage-type-pairs.tsv")?;
    let raw = fs::read_to_string(&tsv)?;

    let mut rows: Vec<(String, String, String)> = Vec::new();
    for line in raw.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [source, key, target] = fields.as_slice() else {
            return Err(format!("malformed pair row: {line:?}").into());
        };
        rows.push((
            (*source).to_owned(),
            (*key).to_owned(),
            (*target).to_owned(),
        ));
    }

    let compiled: Vec<(String, String, String)> = corpus::linkage::TYPE_PAIRS
        .iter()
        .map(|(source, key, target)| {
            (
                (*source).to_owned(),
                (*key).to_owned(),
                (*target).to_owned(),
            )
        })
        .collect();

    assert_eq!(
        compiled, rows,
        "the crate's TYPE_PAIRS and linkage-type-pairs.tsv have drifted apart"
    );
    Ok(())
}

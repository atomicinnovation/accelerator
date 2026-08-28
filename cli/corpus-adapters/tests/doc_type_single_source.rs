//! The dir→type fact is declared once, in `DocTypeKey`.
//! `scripts/linkage-type-pairs.tsv` declares a related fact independently, so
//! this suite cross-checks the crate against it.
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

//! The dir→type registry is declared once, in `DocTypeKey`. This suite
//! cross-checks it against the compiled `config paths --doc-types` resolver:
//! every non-virtual type must resolve to exactly one directory, and the
//! resolved table and the declared types must be the same size.

mod common;

use common::{doc_type_table, TestError};
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

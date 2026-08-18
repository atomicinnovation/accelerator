//! The committed contract evidence, if present, is the reduced form.
//!
//! A live run writes `tests/evidence/contract-run.txt` through the harness;
//! this guard refuses one that carries payloads or secrets. It runs in the
//! default profile — it is not the `contract` binary — so `mise run` enforces
//! it. When the file is absent, no credentialed run has produced it yet and
//! there is nothing to guard; the reduction and the guard itself are proven by
//! `tracker-test-support`'s own unit tests regardless.

use std::path::Path;

type TestError = Box<dyn std::error::Error>;

#[test]
fn committed_evidence_is_reduced() -> Result<(), TestError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/evidence/contract-run.txt");
    if !path.exists() {
        return Ok(());
    }
    let contents = std::fs::read_to_string(&path)?;
    tracker_test_support::evidence::is_reduced(&contents)
        .map_err(|violations| format!("{}: {violations:?}", path.display()))?;
    Ok(())
}

//! Retirement records kept as JSON files in an ignored state directory.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use corpus::store::AtomicWrite;
use corpus::StoreError;
use work::retirement::Retirement;
use work::retirement::RetirementRecord;
use work_adapters::retirement_records::FileRetirementRecords;
use work_adapters::retirement_records::RetirementRecords as _;

struct PlainWrite;

impl AtomicWrite for PlainWrite {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        std::fs::write(path, bytes).map_err(|error| StoreError::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        })
    }
}

fn record() -> RetirementRecord {
    RetirementRecord::of(&Retirement {
        old_id: "PP-760",
        new_id: "ENG-42",
        new_external_id: Some("ENG-42"),
    })
}

#[test]
fn a_saved_record_is_outstanding_until_removed() {
    let state = tempfile::tempdir().unwrap();
    let records = FileRetirementRecords::new(state.path(), &PlainWrite);

    records.save(&record()).unwrap();
    assert_eq!(records.outstanding().unwrap(), vec![record()]);

    records.remove(&record()).unwrap();
    assert!(records.outstanding().unwrap().is_empty());
}

#[test]
fn a_record_whose_ids_would_name_a_path_is_refused() {
    let state = tempfile::tempdir().unwrap();
    let records = FileRetirementRecords::new(state.path(), &PlainWrite);
    let escaping = RetirementRecord::of(&Retirement {
        old_id: "PP-760",
        new_id: "../../escaped",
        new_external_id: Some("../../escaped"),
    });

    assert!(matches!(
        records.save(&escaping),
        Err(StoreError::Validation { .. })
    ));
    assert!(matches!(
        records.remove(&escaping),
        Err(StoreError::Validation { .. })
    ));
    assert!(!state.path().join("escaped.json").exists());
}

#[test]
fn the_record_file_carries_its_schema_and_is_named_by_both_ids() {
    let state = tempfile::tempdir().unwrap();
    let records = FileRetirementRecords::new(state.path(), &PlainWrite);

    records.save(&record()).unwrap();

    let raw = std::fs::read_to_string(
        state.path().join("retirement-records/PP-760--ENG-42.json"),
    )
    .unwrap();
    assert!(raw.contains("\"schema\":1"), "{raw}");
}

#[test]
fn the_records_directory_is_ignored_before_the_first_record() {
    let state = tempfile::tempdir().unwrap();
    let records = FileRetirementRecords::new(state.path(), &PlainWrite);

    records.save(&record()).unwrap();

    assert_eq!(
        std::fs::read_to_string(
            state.path().join("retirement-records/.gitignore")
        )
        .unwrap(),
        "*\n"
    );
}

#[test]
fn no_records_directory_means_nothing_outstanding() {
    let state = tempfile::tempdir().unwrap();
    let records = FileRetirementRecords::new(state.path(), &PlainWrite);

    assert!(records.outstanding().unwrap().is_empty());
    records.remove(&record()).unwrap();
}

#[test]
fn a_malformed_record_is_reported_rather_than_skipped() {
    let state = tempfile::tempdir().unwrap();
    let dir = state.path().join("retirement-records");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("PP-1--ENG-1.json"), "{").unwrap();
    let records = FileRetirementRecords::new(state.path(), &PlainWrite);

    assert!(matches!(
        records.outstanding(),
        Err(StoreError::Validation { .. })
    ));
}

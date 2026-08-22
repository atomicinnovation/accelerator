//! The downgrade vocabulary's goldens, and the drift checks that keep the
//! compiled-in tables agreeing with the files they were lifted from.
//!
//! Exhaustive by construction: the golden test iterates the reason enum, so a
//! variant added without a golden fails rather than going unchecked, and a
//! golden left behind by a removed variant fails too.
//!
//! To regenerate a golden after a deliberate message change, run this suite
//! with `UPDATE_DOWNGRADE_GOLDENS=1`.

use std::fs;
use std::path::PathBuf;

use design::DowngradeReason;

type TestError = Box<dyn std::error::Error>;

fn golden_path(reason: DowngradeReason) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/notify-downgrade")
        .join(format!("{}.expected.txt", reason.key()))
}

#[test]
fn every_reason_reproduces_its_golden_byte_for_byte() -> Result<(), TestError> {
    let updating = std::env::var_os("UPDATE_DOWNGRADE_GOLDENS").is_some();
    for reason in DowngradeReason::ALL {
        let path = golden_path(reason);
        let rendered = format!("{}\n", reason.message());
        if updating {
            fs::write(&path, &rendered)?;
            continue;
        }
        let golden = fs::read_to_string(&path).map_err(|error| {
            format!("{} has no golden: {error}", reason.key())
        })?;
        assert_eq!(
            rendered,
            golden,
            "{} drifted from {}",
            reason.key(),
            path.display()
        );
    }
    Ok(())
}

#[test]
fn no_golden_survives_without_a_reason_to_produce_it() -> Result<(), TestError>
{
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/notify-downgrade");
    let keys: Vec<String> = DowngradeReason::ALL
        .iter()
        .map(|reason| format!("{}.expected.txt", reason.key()))
        .collect();
    for entry in fs::read_dir(&directory)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        assert!(
            keys.contains(&name),
            "{name} is a golden for no reason in the vocabulary"
        );
    }
    Ok(())
}

//! The downgrade vocabulary's goldens, and the drift checks that keep the
//! compiled-in tables agreeing with the files they were lifted from.
//!
//! Exhaustive by construction: the golden test iterates the reason enum, so a
//! variant added without a golden fails rather than going unchecked. That is
//! what replaces `test-notify-downgrade.sh`'s message-key/fixture
//! set-equality check.
//!
//! To regenerate a golden after a deliberate message change, run this suite
//! with `UPDATE_DOWNGRADE_GOLDENS=1`. That is the replacement for
//! `regenerate-notify-downgrade-fixtures.sh`.

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

/// The shell read its messages from this file at runtime. The table is now
/// compiled in, so exhaustiveness is a compile error — but the file is still
/// the record of what the shell emitted, and the two must not diverge
/// silently while `ensure-playwright.sh` is still passing these keys through.
#[test]
fn the_compiled_table_still_agrees_with_the_shell_s_message_file(
) -> Result<(), TestError> {
    const MESSAGES: &str =
        include_str!("fixtures/notify-downgrade-messages.json");

    for reason in DowngradeReason::ALL {
        let recorded =
            messages_entry(MESSAGES, reason.key()).ok_or_else(|| {
                format!("{} is absent from the file", reason.key())
            })?;
        assert_eq!(
            reason.message(),
            recorded,
            "{} drifted from the shell's message file",
            reason.key()
        );
    }
    assert_eq!(
        MESSAGES.matches("\": \"").count(),
        DowngradeReason::ALL.len(),
        "the file carries a key the vocabulary does not"
    );
    Ok(())
}

/// A whole JSON parser is not warranted for a flat string-to-string table this
/// test owns both sides of.
fn messages_entry<'a>(messages: &'a str, key: &str) -> Option<&'a str> {
    messages
        .lines()
        .find_map(|line| line.trim().strip_prefix(&format!("\"{key}\": \"")))
        .map(|rest| rest.trim_end_matches(',').trim_end_matches('"'))
}

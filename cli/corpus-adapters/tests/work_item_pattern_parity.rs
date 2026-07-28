//! Differential parity: the Rust `compile_scan_regex` must agree, output for
//! output, with the live bash `work-item-pattern.sh --compile-scan` it ports,
//! so the two implementations cannot drift.
//!
//! The bash script is the oracle. An absent script or bash hard-fails rather
//! than skipping: Rust's harness has no skip primitive, so a silent early
//! return would register as a green PASS.
#![cfg(feature = "bash-parity")]
// Test inputs are DSL pattern strings whose `{…}` tokens are not format args.
#![allow(clippy::literal_string_with_formatting_args)]

mod common;

use std::path::Path;
use std::process::Command;

use common::{require_script, TestError};
use corpus_adapters::compile_scan_regex;

const SCRIPT: &str = "skills/work/scripts/work-item-pattern.sh";

/// The scan regex bash emits, or `None` when bash rejects the pattern.
fn shell_compile_scan(
    script: &Path,
    pattern: &str,
    project: &str,
) -> Result<Option<String>, TestError> {
    let out = Command::new("bash")
        .arg(script)
        .args(["--compile-scan", pattern, project])
        .output()?;
    if out.status.success() {
        Ok(Some(String::from_utf8(out.stdout)?.trim_end().to_owned()))
    } else {
        Ok(None)
    }
}

#[test]
fn valid_patterns_match_the_shell_compiler() -> Result<(), TestError> {
    let script = require_script(SCRIPT)?;
    let cases = [
        ("{number:04d}", ""),
        ("{number}", ""),
        ("{number:03d}", ""),
        ("{number:010d}", ""),
        ("{project}-{number:04d}", "PROJ"),
        ("{project}-{number}", "Eng"),
        ("TASK-{number:03d}", ""),
        ("v{number:02d}.{number:03d}", ""),
        ("a.{number}", ""),
        ("{{lit}}-{number}", ""),
        ("{project}_{number:05d}", "Team2"),
    ];
    for (pattern, project) in cases {
        let rust =
            compile_scan_regex(pattern, project).map_err(|e| -> TestError {
                format!("rust rejected {pattern:?}: {e}").into()
            })?;
        let shell = shell_compile_scan(&script, pattern, project)?.ok_or_else(
            || -> TestError { format!("shell rejected {pattern:?}").into() },
        )?;
        assert_eq!(
            rust, shell,
            "scan regex mismatch for pattern {pattern:?} project {project:?}"
        );
    }
    Ok(())
}

#[test]
fn invalid_patterns_are_rejected_by_both() -> Result<(), TestError> {
    let script = require_script(SCRIPT)?;
    let cases = [
        ("", ""),
        ("{project}-", "PROJ"),
        ("{project}{number}", "PROJ"),
        ("a/b{number}", ""),
        ("{number:9x}", ""),
        ("{bogus}{number}", ""),
        ("{number:}", ""),
        ("{project}-{number}", ""),
        ("{project}-{number}", "1PROJ"),
        ("{number", ""),
        ("}{number}", ""),
    ];
    for (pattern, project) in cases {
        assert!(
            compile_scan_regex(pattern, project).is_err(),
            "rust must reject {pattern:?}"
        );
        assert!(
            shell_compile_scan(&script, pattern, project)?.is_none(),
            "shell must reject {pattern:?}"
        );
    }
    Ok(())
}

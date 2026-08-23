//! The committed exit-code transcription and the classification rule it
//! implies, with the comparison the self-test runs against it.
//!
//! Consumed by `mapper_differential_self_test.rs`, which proves the comparison
//! can reject a disagreement.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

pub const RETRYABLE_STATUS: i32 = 70;
pub const TERMINAL_STATUS: i32 = 71;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Retryable,
    Terminal,
}

impl Class {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Retryable => "retryable",
            Self::Terminal => "terminal",
        }
    }

    pub const fn status(self) -> i32 {
        match self {
            Self::Retryable => RETRYABLE_STATUS,
            Self::Terminal => TERMINAL_STATUS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    pub code: u8,
    pub provider: String,
    pub operation: String,
    pub class: Class,
    pub source: String,
}

pub fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bridge-exit-code-tables.txt")
}

pub fn table() -> Vec<Row> {
    let raw = std::fs::read_to_string(fixture_path())
        .expect("the committed exit-code table is readable");
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_row)
        .collect()
}

fn parse_row(line: &str) -> Row {
    let mut fields = line.split_whitespace();
    let code = fields
        .next()
        .expect("column 1 is the code")
        .parse()
        .expect("the code is numeric");
    let provider = fields.next().expect("column 2 is the provider").to_owned();
    let operation =
        fields.next().expect("column 3 is the operation").to_owned();
    let class = match fields.next().expect("column 4 is the class") {
        "retryable" => Class::Retryable,
        "terminal" => Class::Terminal,
        other => panic!("unrecognised class {other}"),
    };
    let source = fields.collect::<Vec<_>>().join(" ");
    Row {
        code,
        provider,
        operation,
        class,
        source,
    }
}

/// The transcription's classification rule: a listed code takes its listed
/// class, and every unlisted code falls to `terminal` — the default arm every
/// one of the five mappers carries.
pub fn classify(
    rows: &[Row],
    provider: &str,
    operation: &str,
    code: u8,
) -> Class {
    rows.iter()
        .find(|row| {
            row.provider == provider
                && row.operation == operation
                && row.code == code
        })
        .map_or(Class::Terminal, |row| row.class)
}

/// Compares one transcribed classification against the status the bash mapper
/// actually returned, naming the code and both classifications on a
/// disagreement.
pub fn disagreement(
    provider: &str,
    operation: &str,
    code: u8,
    transcribed: Class,
    bash_status: i32,
) -> Option<String> {
    if bash_status == transcribed.status() {
        return None;
    }
    let bash = match bash_status {
        RETRYABLE_STATUS => "retryable".to_owned(),
        TERMINAL_STATUS => "terminal".to_owned(),
        other => format!("neither class (exit {other})"),
    };
    Some(format!(
        "{provider} {operation} code {code}: transcribed {}, bash says {bash}",
        transcribed.name()
    ))
}

//! Adapter/binary wiring for `work show`: without `--field`, prints the
//! file verbatim; with `--field`, distinguishes missing-file /
//! no-frontmatter / unclosed-frontmatter before calling
//! `work::show::read_field_raw` (with the own-identity alias fallback).

use std::path::Path;

use work::own_identity::own_identity_alias;
use work::show::read_field_raw;

pub enum RunOutcome {
    FullFile(String),
    FieldValue(String),
    MissingFile,
    NoFrontmatter,
    UnclosedFrontmatter,
    FieldMissing,
}

enum FrontmatterState {
    Found(String),
    NoFrontmatter,
    Unclosed,
}

fn frontmatter_state(content: &str) -> FrontmatterState {
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return FrontmatterState::NoFrontmatter;
    };
    if first.trim_end() != "---" {
        return FrontmatterState::NoFrontmatter;
    }
    let mut block = String::new();
    for line in lines {
        if line.trim_end() == "---" {
            return FrontmatterState::Found(block);
        }
        block.push_str(line);
        block.push('\n');
    }
    FrontmatterState::Unclosed
}

fn run_field(content: &str, field: &str) -> RunOutcome {
    let frontmatter = match frontmatter_state(content) {
        FrontmatterState::NoFrontmatter => return RunOutcome::NoFrontmatter,
        FrontmatterState::Unclosed => return RunOutcome::UnclosedFrontmatter,
        FrontmatterState::Found(block) => block,
    };
    if let Some(value) = read_field_raw(&frontmatter, field) {
        return RunOutcome::FieldValue(value);
    }
    if let Some(alias) = own_identity_alias(field) {
        if let Some(value) = read_field_raw(&frontmatter, alias) {
            return RunOutcome::FieldValue(value);
        }
    }
    RunOutcome::FieldMissing
}

#[must_use]
pub fn run(path: &Path, field: Option<&str>) -> RunOutcome {
    let Ok(content) = std::fs::read_to_string(path) else {
        return RunOutcome::MissingFile;
    };
    match field {
        Some(field) => run_field(&content, field),
        None => RunOutcome::FullFile(content),
    }
}

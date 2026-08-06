//! Loads `hooks/test-fixtures/masks.toml`, the volatile-field mask table
//! shared with the Python golden-generation script.

use std::path::Path;

use crate::Error;

/// One named volatile-field pattern, with the positive/negative sample
/// pair it is pinned against.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Mask {
    pub name: String,
    pub regex: String,
    pub description: String,
    pub sample_match: String,
    pub sample_no_match: String,
}

#[derive(Debug, serde::Deserialize)]
struct MasksFile {
    pattern: Vec<Mask>,
}

/// Reads and parses the mask table at `path`.
///
/// # Errors
///
/// When the file cannot be read or does not parse as the expected TOML
/// shape.
pub fn load(path: &Path) -> Result<Vec<Mask>, Error> {
    let text = std::fs::read_to_string(path)?;
    let file: MasksFile = toml::from_str(&text).map_err(|error| {
        Error::message(format!("parsing {}: {error}", path.display()))
    })?;
    Ok(file.pattern)
}

/// Replaces every match of every mask pattern in `text` with
/// `<PATTERN_NAME>`, applying the masks in the order they appear in the
/// loaded table.
///
/// # Errors
///
/// When a mask's `regex` field fails to compile.
pub fn apply(masks: &[Mask], text: &str) -> Result<String, Error> {
    let mut result = text.to_string();
    for mask in masks {
        let compiled = regex::Regex::new(&mask.regex).map_err(|error| {
            Error::message(format!("compiling mask {}: {error}", mask.name))
        })?;
        let placeholder = format!("<{}>", mask.name.to_uppercase());
        result = compiled
            .replace_all(&result, placeholder.as_str())
            .into_owned();
    }
    Ok(result)
}

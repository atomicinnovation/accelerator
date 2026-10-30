//! The tab-separated records a command prints to stdout, each field made
//! safe so no value can split or forge a record.

use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Default)]
pub struct TsvRecord {
    fields: Vec<String>,
}

impl TsvRecord {
    #[must_use]
    pub fn of<I, F>(fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Display,
    {
        Self {
            fields: fields
                .into_iter()
                .map(|field| printable(&single_line(&field.to_string())))
                .collect(),
        }
    }
}

impl Display for TsvRecord {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.fields.join("\t"))
    }
}

/// Collapses tab, newline and carriage return to a space, so a value cannot
/// break the single-record TSV format.
pub fn single_line(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}

/// Escapes every control character but the newline, so text from paths,
/// filenames or a tracker cannot move the cursor, clear the screen or
/// overwrite a line, while a deliberately multi-line warning keeps its
/// layout.
pub fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && character != '\n' {
                character.escape_default().collect()
            } else {
                character.to_string()
            }
        })
        .collect()
}

/// A failure's stderr line, built from its parts so no cause can forge a
/// second line; `failed` is `failed`, or `would fail` on a preview.
#[must_use]
pub fn failure_line(
    failure: &work_adapters::sync::report::RunFailure,
    failed: &str,
) -> String {
    let label = match failure.severity {
        work_adapters::sync::report::FailureSeverity::Error => "error",
        work_adapters::sync::report::FailureSeverity::Warning => "warning",
    };
    format!(
        "{label}: {}: {} {failed}: {}",
        single_line(&failure.subject),
        failure.operation,
        single_line(&failure.cause)
    )
}

#[cfg(test)]
mod tests {
    use super::TsvRecord;

    #[test]
    fn a_filename_with_a_newline_and_tab_yields_one_record() {
        let record =
            TsvRecord::of(["#", "kept-edits", "0001", "a\nb\tc\x1bd.md"]);

        assert_eq!(record.to_string(), "#\tkept-edits\t0001\ta b c\\u{1b}d.md");
    }
}

//! The read-only `counts` command: the counts a set's files give, beside
//! those its documents record.

use std::path::Path;

use corpus::scan::DirReader;
use corpus::scan::DirectoryProbe;
use corpus::scan::FileReader;
use corpus_adapters::YamlFrontmatter;
use research::topic::counts::StoredCount;
use research_adapters::topic::read_counts;
use serde_json::json;
use serde_json::Value;

use crate::counts_wording;
use crate::topic_command::Printed;

/// Prints the set's derived and stored counts and whether they agree.
///
/// # Errors
///
/// A message when the set's directories or files cannot be read; a stored
/// count that is missing or unreadable is a mismatch, never an error.
pub fn run_counts<F: DirReader + FileReader + DirectoryProbe>(
    set_root: &Path,
    fs: &F,
) -> Result<Printed, String> {
    let reading = read_counts(set_root, fs, &YamlFrontmatter)
        .map_err(|error| error.to_string())?;
    let freshness = reading.freshness();
    let stored = &reading.stored;
    let rendered = json!({
        "derived": {
            "round_count": reading.findings.round_count(),
            "finding_count": reading.findings.finding_count(),
            "report_count": reading.reports.report_count(),
        },
        "stored": {
            "round_count": stored_value(&stored.round_count),
            "finding_count": stored_value(&stored.finding_count),
            "report_count": stored_value(&stored.report_count),
            "rounds_covered": stored_value(&stored.rounds_covered),
        },
        "fresh": freshness.is_fresh(),
        "mismatches": freshness
            .mismatches
            .iter()
            .map(counts_wording::mismatch)
            .collect::<Vec<_>>(),
    });
    Ok(Printed {
        stdout: format!("{rendered}\n"),
        stderr: String::new(),
    })
}

fn stored_value(count: &StoredCount) -> Value {
    match count {
        StoredCount::Count(value) => json!(value),
        StoredCount::Missing | StoredCount::Unreadable(_) => Value::Null,
    }
}

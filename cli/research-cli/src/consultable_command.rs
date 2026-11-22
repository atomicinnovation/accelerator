//! The read-only `consultable` command: whether `ask` and `report` may
//! consult a set.

use std::path::Path;

use corpus::scan::DirReader;
use corpus::scan::DirectoryProbe;
use corpus::scan::FileReader;
use corpus_adapters::YamlFrontmatter;
use research::topic::consultation::Consultation;
use research::topic::consultation::Refusal;
use research_adapters::topic::level_notes_of;
use research_adapters::topic::read_counts;
use research_adapters::topic::read_plan;
use research_adapters::topic::read_set_status;
use research_adapters::unicode_text::UnicodeTables;
use serde_json::json;
use serde_json::Value;

use crate::counts_wording;
use crate::plan_wording;
use crate::topic_command::Printed;

/// Prints the set's consultation verdict: open with the documents to read,
/// or the first gate it fails.
///
/// # Errors
///
/// A message when the profiles directory is missing, before any gate, or
/// when the set cannot be read as far as its verdict needs, so a set is
/// never opened on a read that failed.
pub fn run_consultable<F: DirReader + FileReader + DirectoryProbe>(
    set_root: &Path,
    profiles_dir: &Path,
    fs: &F,
) -> Result<Printed, String> {
    if !fs.is_dir(profiles_dir) {
        return Err(format!(
            "no profiles directory at {}",
            profiles_dir.display()
        ));
    }
    let status = read_set_status(set_root, fs, &YamlFrontmatter)
        .map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();
    let verdict = Consultation::judge(
        status,
        || read_counts(set_root, fs, &YamlFrontmatter),
        || {
            let reading = read_plan(
                set_root,
                profiles_dir,
                fs,
                &YamlFrontmatter,
                &UnicodeTables,
            )?;
            warnings = reading
                .plan
                .warnings
                .iter()
                .map(plan_wording::warning)
                .chain(reading.warnings.iter().map(ToString::to_string))
                .collect();
            Ok(reading.plan)
        },
    )
    .map_err(|error| error.to_string())?;
    let level_notes = match &verdict {
        Consultation::Open(documents) => {
            level_notes(set_root, documents.findings(), fs)?
        }
        Consultation::Refused(_) => Vec::new(),
    };
    Ok(Printed {
        stdout: format!(
            "{}\n",
            rendered(&verdict, &warnings, &level_notes, set_root)
        ),
        stderr: String::new(),
    })
}

type LevelNotes = Vec<(String, Vec<String>)>;

fn level_notes<F: DirReader + DirectoryProbe>(
    set_root: &Path,
    findings: &[String],
    fs: &F,
) -> Result<LevelNotes, String> {
    let mut composed = Vec::new();
    for finding in findings {
        let name = finding.rsplit('/').next().unwrap_or(finding);
        let notes = level_notes_of(set_root, name, fs)
            .map_err(|error| error.to_string())?;
        if !notes.is_empty() {
            composed.push((finding.clone(), notes));
        }
    }
    Ok(composed)
}

fn rendered(
    verdict: &Consultation,
    warnings: &[String],
    level_notes: &LevelNotes,
    set_root: &Path,
) -> Value {
    let absolute = |relative: &String| {
        set_root.join(relative).to_string_lossy().into_owned()
    };
    match verdict {
        Consultation::Open(documents) => {
            let findings: Vec<_> =
                documents.findings().iter().map(absolute).collect();
            let reports: Vec<_> =
                documents.reports().iter().map(absolute).collect();
            let level_notes: serde_json::Map<String, Value> = level_notes
                .iter()
                .map(|(finding, notes)| {
                    (
                        absolute(finding),
                        notes.iter().map(absolute).collect::<Value>(),
                    )
                })
                .collect();
            json!({
                "consultable": true,
                "findings": findings,
                "reports": reports,
                "level_notes": level_notes,
                "highest_round": documents.highest_round(),
            })
        }
        Consultation::Refused(Refusal::Status { status }) => json!({
            "consultable": false,
            "gate": "status",
            "status": status.name(),
        }),
        Consultation::Refused(Refusal::Freshness { mismatches }) => json!({
            "consultable": false,
            "gate": "freshness",
            "mismatches": mismatches
                .iter()
                .map(counts_wording::mismatch)
                .collect::<Vec<_>>(),
        }),
        Consultation::Refused(Refusal::Conducted {
            unanswered,
            held_back,
        }) => json!({
            "consultable": false,
            "gate": "conducted",
            "unanswered": unanswered
                .iter()
                .map(|pair| json!({
                    "question": pair.question,
                    "profile": pair.profile,
                }))
                .collect::<Vec<_>>(),
            "held_back": held_back
                .iter()
                .map(|item| json!({
                    "question": item.question,
                    "reasons": plan_wording::hold_back_reasons(&item.hold_back),
                }))
                .collect::<Vec<_>>(),
            "warnings": warnings,
        }),
    }
}

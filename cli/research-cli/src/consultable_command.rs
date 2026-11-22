//! The read-only `consultable` command: whether `ask` and `report` may
//! consult a set.

use std::path::Path;

use corpus::scan::DirReader;
use corpus::scan::DirectoryProbe;
use corpus::scan::FileReader;
use corpus_adapters::YamlFrontmatter;
use research::topic::consultation::Consultation;
use research::topic::consultation::Refusal;
use research::topic::plan::HoldBack;
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
    Ok(Printed {
        stdout: format!("{}\n", rendered(&verdict, &warnings, set_root)),
        stderr: String::new(),
    })
}

fn rendered(
    verdict: &Consultation,
    warnings: &[String],
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
            json!({
                "consultable": true,
                "findings": findings,
                "reports": reports,
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
                    "reasons": hold_back_reasons(&item.hold_back),
                }))
                .collect::<Vec<_>>(),
            "warnings": warnings,
        }),
    }
}

fn hold_back_reasons(hold_back: &HoldBack) -> Vec<String> {
    match hold_back {
        HoldBack::Skipped(skips) => {
            skips.iter().map(plan_wording::skip_reason).collect()
        }
        HoldBack::IndexesExhausted => {
            vec!["no finding index is left to allocate to it".to_owned()]
        }
    }
}

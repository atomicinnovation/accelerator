//! How a freshness mismatch reads to whoever runs `finalise`, `ask` or
//! `report`.

use research::topic::counts::CountField;
use research::topic::counts::FreshnessCount;
use research::topic::counts::Mismatch;
use research::topic::counts::ReportCountDrift;

pub fn mismatch(mismatch: &Mismatch) -> String {
    match mismatch {
        Mismatch::Missing { count } => {
            let field = CountField::Manifest(*count);
            format!("{} records no {}", field.document(), field.key())
        }
        Mismatch::Unreadable { field, raw } => format!(
            "{} records {} '{}', which is not a whole number",
            field.document(),
            field.key(),
            raw.escape_debug()
        ),
        Mismatch::Disagrees {
            count,
            stored,
            derived,
        } => {
            let field = CountField::Manifest(*count);
            let on_disk = match count {
                FreshnessCount::RoundCount => {
                    format!("the findings reach round {derived}")
                }
                FreshnessCount::FindingCount => {
                    format!("{derived} findings are on disk")
                }
            };
            format!(
                "{} records {} {stored} but {on_disk}",
                field.document(),
                field.key()
            )
        }
        Mismatch::UncoveredRound { derived, covered } => format!(
            "the findings reach round {derived} but {} covers up to round \
             {covered}",
            CountField::RoundsCovered.document()
        ),
    }
}

pub fn report_count_drift(drift: &ReportCountDrift) -> String {
    match drift {
        ReportCountDrift::Disagrees { stored, derived } => format!(
            "manifest.md records report_count {stored} but {derived} reports \
             are on disk"
        ),
        ReportCountDrift::Unreadable(raw) => format!(
            "manifest.md records report_count '{}', which is not a whole \
             number",
            raw.escape_debug()
        ),
    }
}

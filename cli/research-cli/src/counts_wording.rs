//! How a freshness mismatch reads to whoever runs `finalise`, `ask` or
//! `report`.

use research::topic::counts::CountField;
use research::topic::counts::ManifestCount;
use research::topic::counts::Mismatch;

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
            count: ManifestCount::RoundCount,
            stored,
            derived,
        } => format!(
            "manifest.md records round_count {stored} but the findings reach \
             round {derived}"
        ),
        Mismatch::Disagrees {
            count: ManifestCount::FindingCount,
            stored,
            derived,
        } => format!(
            "manifest.md records finding_count {stored} but {derived} \
             findings are on disk"
        ),
        Mismatch::UncoveredRound { derived, covered } => format!(
            "the findings reach round {derived} but synthesis.md covers up to \
             round {covered}"
        ),
    }
}

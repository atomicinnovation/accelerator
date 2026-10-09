//! How a draft's promotion is reported by `work sync` and `work promote`:
//! one four-column row, the trailing detail lines a person acts on, and the
//! code it exits with.

use work::promotion::Promotion;
use work_adapters::promotion::Detail;
use work_adapters::promotion::DetailSource;
use work_adapters::promotion::PromotionOutcome;
use work_adapters::promotion::PromotionRow;

use crate::exit_codes;

#[must_use]
pub fn promotion_line(row: &PromotionRow) -> String {
    let (action, state, detail) = match &row.outcome {
        PromotionOutcome::Previewed => {
            ("promote", "unsynced".to_owned(), "-".to_owned())
        }
        PromotionOutcome::Promoted(Promotion::Completed(key, state)) => {
            ("promoted", state.to_string(), key.to_string())
        }
        PromotionOutcome::Promoted(Promotion::AlreadyDone(key)) => {
            ("already-promoted", "synced".to_owned(), key.to_string())
        }
        PromotionOutcome::NotPromoted { reason, .. } => (
            "not-promoted",
            "unsynced".to_owned(),
            reason.keyword().to_owned(),
        ),
    };
    format!("{}\t{action}\t{state}\t{detail}", row.draft)
}

pub fn detail_lines(row: &PromotionRow) -> impl Iterator<Item = String> + '_ {
    row.details
        .iter()
        .map(|detail| detail_line(&row.draft, detail))
}

#[must_use]
pub fn detail_line(id: &str, detail: &Detail) -> String {
    let path = detail.path.display();
    match &detail.source {
        DetailSource::Holder => format!("#\tdetail\t{id}\tholder\t{path}"),
        DetailSource::Vcs => format!("#\tdetail\t{id}\tvcs\t{path}"),
        DetailSource::Recovery { location } => {
            format!("#\tdetail\t{id}\trecovery\t{path}\t{}", location.display())
        }
        DetailSource::Item => format!("#\tdetail\t{id}\titem\t{path}"),
    }
}

#[must_use]
pub const fn promotion_exit(row: &PromotionRow) -> u8 {
    match &row.outcome {
        PromotionOutcome::NotPromoted { reason, held_key } => {
            reason.exit_code(held_key.is_some())
        }
        PromotionOutcome::Previewed | PromotionOutcome::Promoted(_) => {
            exit_codes::CLEAN
        }
    }
}

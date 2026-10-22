//! How a draft's promotion is reported by `work sync` and `work promote`:
//! one four-column row, the trailing detail lines a person acts on, and the
//! code it exits with.

use work::promotion::Promotion;
use work_adapters::promotion::Detail;
use work_adapters::promotion::DetailSource;
use work_adapters::promotion::PromotionOutcome;
use work_adapters::promotion::PromotionRow;

use crate::exit_codes;
use crate::report_records::TsvRecord;

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
    let path = detail.path.display().to_string();
    let source = |kind: &str| vec![kind.to_owned(), path.clone()];
    let fields = match &detail.source {
        DetailSource::Holder => source("holder"),
        DetailSource::Vcs => source("vcs"),
        DetailSource::Recovery { location } => {
            [source("recovery"), vec![location.display().to_string()]].concat()
        }
        DetailSource::Item => source("item"),
    };
    TsvRecord::of(
        ["#", "detail", id]
            .into_iter()
            .map(str::to_owned)
            .chain(fields),
    )
    .to_string()
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use work_adapters::promotion::Detail;
    use work_adapters::promotion::DetailSource;

    use super::detail_line;

    #[test]
    fn an_ordinary_promotion_detail_line_is_byte_identical() {
        let detail = |source| Detail {
            source,
            path: PathBuf::from("meta/work/0001-child.md"),
        };

        assert_eq!(
            detail_line("draft-k7mq3x", &detail(DetailSource::Vcs)),
            "#\tdetail\tdraft-k7mq3x\tvcs\tmeta/work/0001-child.md"
        );
        assert_eq!(
            detail_line(
                "draft-k7mq3x",
                &detail(DetailSource::Recovery {
                    location: PathBuf::from(".accelerator/state/r"),
                })
            ),
            "#\tdetail\tdraft-k7mq3x\trecovery\tmeta/work/0001-child.md\t\
             .accelerator/state/r"
        );
    }
}

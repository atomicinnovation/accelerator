//! How a run ledger that `--start` replaced reads to whoever runs
//! `conduct`.

use research::conduct::replaced_run::ReplacedRun;

use crate::topic_command::ReplacedLedger;

pub fn corrupt_notice(replaced: &ReplacedLedger) -> Option<String> {
    match replaced {
        ReplacedLedger::Corrupt => Some(
            "replaced a corrupt run ledger; if its run is still going it \
             stops at its next batch, and notes its last batch writes may \
             show as unexpected"
                .to_owned(),
        ),
        ReplacedLedger::Absent | ReplacedLedger::Readable(_) => None,
    }
}

pub fn replaced_notice(replaced: &ReplacedRun) -> String {
    let notice = format!(
        "replaced the ledger of run {}; if it is still going it stops at \
         its next batch",
        replaced.run()
    );
    if replaced.quarantine().is_empty() {
        notice
    } else {
        format!(
            "{notice}; rename each replaced_run quarantine path to \
             .<name>.invalid beside it, never deleting it, before acting on \
             this plan"
        )
    }
}

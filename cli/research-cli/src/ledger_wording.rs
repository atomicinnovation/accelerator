//! How a `conduct` run's ledger reads to whoever runs `conduct`: one that
//! `--start` replaced, one another run owns, or one that cannot be used.

use std::fmt::Display;
use std::path::Path;

use research::conduct::ledger::RunId;
use research::conduct::replaced_run::ReplacedRun;

pub fn corrupt_notice() -> String {
    "replaced a corrupt run ledger; if its run is still going it stops at \
     its next batch, and notes its last batch writes may show as unexpected"
        .to_owned()
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
            "{notice}; before acting on this plan, rename each \
             replaced_run quarantine path to .<name>.invalid beside it, \
             never deleting it"
        )
    }
}

pub fn superseded(run: &RunId, owner: &RunId) -> String {
    format!(
        "E_TOPIC_RESEARCH_RUN_SUPERSEDED: run {run} was superseded by run \
         {owner}; another conduct run now owns this set, so let it finish"
    )
}

pub fn no_usable_ledger(run: &RunId) -> String {
    format!(
        "E_TOPIC_RESEARCH_RUN_LEDGER: no usable ledger for run {run}; re-run \
         conduct to start a fresh run"
    )
}

pub fn unreadable(ledger: &Path, error: impl Display) -> String {
    format!(
        "E_TOPIC_RESEARCH_RUN_LEDGER: could not read {}: {error}",
        ledger.display()
    )
}

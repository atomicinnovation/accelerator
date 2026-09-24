//! Withdrawal verdicts already confirmed against `arXivRaw`, kept across
//! calls so an entry is confirmed once per version rather than once per call.

use std::collections::BTreeMap;
use std::rc::Rc;

use research::fetch::ConfirmationCache;
use research::request::ArxivId;
use serde::Deserialize;
use serde::Serialize;

use crate::diagnostics::Diagnostics;
use crate::scratch::ScratchDir;

const CACHE: &str = "arxiv-withdrawals.json";

/// Verdicts shared by every call in the project.
///
/// Every write re-reads and merges before replacing the file, so callers
/// sharing the directory keep each other's verdicts. The fetch writes only
/// while holding the arXiv pacing lock, which serialises those writers.
pub struct FileConfirmationCache {
    scratch: ScratchDir,
    diagnostics: Rc<dyn Diagnostics>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Verdict {
    version: Option<u32>,
    withdrawn: bool,
}

type Verdicts = BTreeMap<String, Verdict>;

impl FileConfirmationCache {
    pub fn new(scratch: ScratchDir, diagnostics: Rc<dyn Diagnostics>) -> Self {
        Self {
            scratch,
            diagnostics,
        }
    }

    fn verdicts(&self) -> Verdicts {
        self.scratch
            .read(CACHE)
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
}

impl ConfirmationCache for FileConfirmationCache {
    fn recall(&self, entry: &ArxivId) -> Option<bool> {
        self.verdicts()
            .get(entry.unversioned())
            .filter(|verdict| verdict.version == entry.version())
            .map(|verdict| verdict.withdrawn)
    }

    fn record(&self, entry: &ArxivId, withdrawn: bool) {
        let mut verdicts = self.verdicts();
        verdicts.insert(
            entry.unversioned().to_owned(),
            Verdict {
                version: entry.version(),
                withdrawn,
            },
        );
        let written = serde_json::to_vec(&verdicts)
            .map_err(|error| error.to_string())
            .and_then(|bytes| self.scratch.replace(CACHE, &bytes));
        if let Err(error) = written {
            self.diagnostics
                .report(&format!("research fetch: arxiv {error}"));
        }
    }
}

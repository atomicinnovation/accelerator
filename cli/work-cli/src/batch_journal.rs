//! The batch journal: what each entry of a pushed `work create-batch`
//! became, so rerunning a batch creates only the entries it never reached.
//!
//! An entry is matched by its content digest, or by its title when a
//! regenerated manifest of the same batch reworded the body, never by its
//! position, so a reordered or regenerated manifest resumes identically.

use std::path::Path;
use std::path::PathBuf;

use corpus::AtomicWrite;
use corpus_adapters::FileCorpusStore;
use serde_json::json;
use serde_json::Value;

const SCHEMA: u64 = 1;
const DIRECTORY: &str = "batch-journal";
const FILE: &str = "journal.json";
const RETENTION_SECONDS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub content_digest: String,
    /// The fingerprint of the manifest that recorded the entry; empty for an
    /// entry an older binary recorded, which only its digest then matches.
    pub batch: String,
    pub title: String,
    /// The ID the entry's item took: a draft's, a local number, or a key.
    pub id: Option<String>,
    pub key: Option<String>,
    /// `None` while the entry's draft is written but its create unfinished.
    pub outcome: Option<String>,
    pub recorded_at: u64,
}

pub struct BatchJournal {
    dir: PathBuf,
    entries: Vec<JournalEntry>,
    claimed: Vec<bool>,
}

impl BatchJournal {
    /// Opens the journal under `state_dir`, dropping entries recorded more
    /// than 30 days before `now`. Fail-closed: the directory's `*`
    /// `.gitignore` is written and verified before anything is read.
    ///
    /// # Errors
    ///
    /// A message naming the journal when its directory cannot be ignored
    /// or its file cannot be parsed.
    pub fn open(state_dir: &Path, now: u64) -> Result<Self, String> {
        let dir = state_dir.join(DIRECTORY);
        ignore_everything_in(&dir).map_err(|error| {
            format!(
                "could not prepare the batch journal at {}: {error}",
                dir.display()
            )
        })?;
        let path = dir.join(FILE);
        let entries = match std::fs::read_to_string(&path) {
            Ok(text) => parse(&text).ok_or_else(|| {
                format!(
                    "the batch journal at {} could not be parsed; inspect or \
                     remove it",
                    path.display()
                )
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Vec::new()
            }
            Err(error) => {
                return Err(format!(
                    "could not read the batch journal at {}: {error}",
                    path.display()
                ));
            }
        };
        let entries: Vec<JournalEntry> = entries
            .into_iter()
            .filter(|entry| {
                now.saturating_sub(entry.recorded_at) <= RETENTION_SECONDS
            })
            .collect();
        Ok(Self {
            claimed: vec![false; entries.len()],
            dir,
            entries,
        })
    }

    /// Claims the unclaimed entry recorded for this content, preferring
    /// one whose digest matches over one `batch` recorded under this title.
    /// An entry claimed by title is recorded under `content_digest` from
    /// then on, so its superseded digest no longer lingers.
    ///
    /// # Errors
    ///
    /// As [`BatchJournal::record`], when a title claim cannot be saved.
    pub fn claim(
        &mut self,
        content_digest: &str,
        batch: &str,
        title: &str,
    ) -> Result<Option<JournalEntry>, String> {
        let unclaimed = |matches: &dyn Fn(&JournalEntry) -> bool| {
            self.entries
                .iter()
                .zip(&self.claimed)
                .position(|(entry, claimed)| !claimed && matches(entry))
        };
        if let Some(position) =
            unclaimed(&|entry| entry.content_digest == content_digest)
        {
            self.claimed[position] = true;
            return Ok(Some(self.entries[position].clone()));
        }
        let Some(position) = unclaimed(&|entry| {
            !entry.batch.is_empty()
                && entry.batch == batch
                && entry.title == title
        }) else {
            return Ok(None);
        };
        self.claimed[position] = true;
        content_digest.clone_into(&mut self.entries[position].content_digest);
        self.save()?;
        Ok(Some(self.entries[position].clone()))
    }

    /// Records `entry`, replacing whatever was recorded for its digest.
    ///
    /// # Errors
    ///
    /// A message naming the journal when it cannot be written.
    pub fn record(&mut self, entry: JournalEntry) -> Result<(), String> {
        if let Some(position) = self.entries.iter().position(|recorded| {
            recorded.content_digest == entry.content_digest
        }) {
            self.entries[position] = entry;
            self.claimed[position] = true;
        } else {
            self.entries.push(entry);
            self.claimed.push(true);
        }
        self.save()
    }

    /// Drops what was recorded for `content_digest`, once it no longer
    /// names anything a rerun could report.
    ///
    /// # Errors
    ///
    /// As [`BatchJournal::record`].
    pub fn forget(&mut self, content_digest: &str) -> Result<(), String> {
        let entries = std::mem::take(&mut self.entries);
        let claimed = std::mem::take(&mut self.claimed);
        (self.entries, self.claimed) = entries
            .into_iter()
            .zip(claimed)
            .filter(|(entry, _)| entry.content_digest != content_digest)
            .unzip();
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        let rendered = json!({
            "schema": SCHEMA,
            "entries": self.entries.iter().map(render).collect::<Vec<_>>(),
        });
        let path = self.dir.join(FILE);
        AtomicWrite::write(
            &FileCorpusStore::new(&self.dir),
            &path,
            format!("{rendered}\n").as_bytes(),
        )
        .map_err(|error| {
            format!(
                "could not write the batch journal at {}: {error}",
                path.display()
            )
        })
    }
}

fn ignore_everything_in(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let gitignore = dir.join(".gitignore");
    std::fs::write(&gitignore, "*\n")?;
    if !std::fs::read_to_string(&gitignore)?.contains('*') {
        return Err(std::io::Error::other(
            "the batch journal's .gitignore could not be verified",
        ));
    }
    Ok(())
}

fn render(entry: &JournalEntry) -> Value {
    json!({
        "content_digest": entry.content_digest,
        "batch": entry.batch,
        "title": entry.title,
        "id": entry.id,
        "key": entry.key,
        "outcome": entry.outcome,
        "recorded_at": entry.recorded_at,
    })
}

fn parse(text: &str) -> Option<Vec<JournalEntry>> {
    let value: Value = serde_json::from_str(text).ok()?;
    if value.get("schema").and_then(Value::as_u64) != Some(SCHEMA) {
        return None;
    }
    value
        .get("entries")?
        .as_array()?
        .iter()
        .map(|entry| {
            let text = |field: &str| {
                entry.get(field).and_then(Value::as_str).map(str::to_owned)
            };
            Some(JournalEntry {
                content_digest: text("content_digest")?,
                batch: text("batch").unwrap_or_default(),
                title: text("title")?,
                id: text("id"),
                key: text("key"),
                outcome: text("outcome"),
                recorded_at: entry.get("recorded_at")?.as_u64()?,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    const DAY: u64 = 24 * 60 * 60;
    const BATCH: &str = "batch-one";

    fn entry(digest: &str, title: &str, recorded_at: u64) -> JournalEntry {
        JournalEntry {
            content_digest: digest.to_owned(),
            batch: BATCH.to_owned(),
            title: title.to_owned(),
            id: Some("draft-k7mq3x".to_owned()),
            key: Some("PP-901".to_owned()),
            outcome: Some("write-once".to_owned()),
            recorded_at,
        }
    }

    #[test]
    fn a_recorded_entry_is_claimed_by_digest_on_reopening() {
        let state = tempfile::tempdir().expect("tempdir");
        let mut journal = BatchJournal::open(state.path(), 100).expect("open");
        journal.record(entry("d1", "Epic", 100)).expect("record");

        let mut reopened = BatchJournal::open(state.path(), 100).expect("open");

        assert_eq!(
            reopened.claim("d1", BATCH, "Renamed").expect("saved"),
            Some(entry("d1", "Epic", 100))
        );
        assert_eq!(
            reopened.claim("d1", BATCH, "Renamed").expect("saved"),
            None,
            "claimed once"
        );
    }

    #[test]
    fn an_entry_whose_body_was_reworded_is_claimed_by_title() {
        let state = tempfile::tempdir().expect("tempdir");
        let mut journal = BatchJournal::open(state.path(), 100).expect("open");
        journal.record(entry("d1", "Epic", 100)).expect("record");
        journal.record(entry("d2", "Story", 100)).expect("record");

        let mut reopened = BatchJournal::open(state.path(), 100).expect("open");

        assert_eq!(
            reopened
                .claim("reworded", BATCH, "Story")
                .expect("saved")
                .map(|found| found.title),
            Some("Story".to_owned())
        );
    }

    #[test]
    fn a_title_claimed_entry_can_be_forgotten() {
        let state = tempfile::tempdir().expect("tempdir");
        let mut journal = BatchJournal::open(state.path(), 100).expect("open");
        journal.record(entry("d1", "Story", 100)).expect("record");
        let mut reworded = BatchJournal::open(state.path(), 100).expect("open");
        let claimed = reworded
            .claim("reworded", BATCH, "Story")
            .expect("saved")
            .expect("claimed");

        reworded.forget(&claimed.content_digest).expect("forget");

        let mut reopened = BatchJournal::open(state.path(), 100).expect("open");
        assert_eq!(reopened.claim("d1", BATCH, "Story").expect("saved"), None);
        assert_eq!(
            reopened.claim("reworded", BATCH, "Story").expect("saved"),
            None
        );
    }

    #[test]
    fn entries_older_than_thirty_days_are_pruned() {
        let state = tempfile::tempdir().expect("tempdir");
        let mut journal = BatchJournal::open(state.path(), 0).expect("open");
        journal.record(entry("old", "Old", 0)).expect("record");
        journal
            .record(entry("recent", "Recent", 2 * DAY))
            .expect("record");

        let mut reopened =
            BatchJournal::open(state.path(), 31 * DAY).expect("open");

        assert_eq!(reopened.claim("old", BATCH, "Old").expect("saved"), None);
        assert!(reopened
            .claim("recent", BATCH, "Recent")
            .expect("saved")
            .is_some());
    }

    #[test]
    fn a_title_claim_moves_the_entry_to_the_reworded_digest() {
        let state = tempfile::tempdir().expect("tempdir");
        let mut journal = BatchJournal::open(state.path(), 100).expect("open");
        journal.record(entry("d1", "Story", 100)).expect("record");
        let mut reworded = BatchJournal::open(state.path(), 100).expect("open");
        reworded
            .claim("reworded", BATCH, "Story")
            .expect("saved")
            .expect("claimed");

        let mut reopened = BatchJournal::open(state.path(), 100).expect("open");

        assert_eq!(
            reopened.claim("d1", "other", "Other").expect("saved"),
            None
        );
        assert!(reopened
            .claim("reworded", "other", "Other")
            .expect("saved")
            .is_some());
    }

    #[test]
    fn a_title_another_batch_recorded_is_not_claimed() {
        let state = tempfile::tempdir().expect("tempdir");
        let mut journal = BatchJournal::open(state.path(), 100).expect("open");
        journal
            .record(entry("d1", "Add tests", 100))
            .expect("record");

        let mut reopened = BatchJournal::open(state.path(), 100).expect("open");

        assert_eq!(
            reopened
                .claim("d2", "batch-two", "Add tests")
                .expect("saved"),
            None
        );
        assert!(reopened
            .claim("d1", "batch-two", "Add tests")
            .expect("saved")
            .is_some());
    }

    #[test]
    fn the_journal_directory_ignores_everything_in_it() {
        let state = tempfile::tempdir().expect("tempdir");

        BatchJournal::open(state.path(), 0).expect("open");

        assert_eq!(
            std::fs::read_to_string(
                state.path().join("batch-journal/.gitignore")
            )
            .expect("ignored"),
            "*\n"
        );
    }

    #[test]
    fn a_forgotten_entry_is_no_longer_claimed() {
        let state = tempfile::tempdir().expect("tempdir");
        let mut journal = BatchJournal::open(state.path(), 0).expect("open");
        journal.record(entry("d1", "Epic", 0)).expect("record");
        journal.forget("d1").expect("forget");

        let mut reopened = BatchJournal::open(state.path(), 0).expect("open");

        assert_eq!(reopened.claim("d1", BATCH, "Epic").expect("saved"), None);
    }

    #[test]
    fn an_unparseable_journal_refuses_to_open() {
        let state = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(state.path().join("batch-journal"))
            .expect("dir");
        std::fs::write(state.path().join("batch-journal/journal.json"), "{")
            .expect("write");

        assert!(BatchJournal::open(state.path(), 0).is_err());
    }
}

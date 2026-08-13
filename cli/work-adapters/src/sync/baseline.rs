//! The sync baseline document: `<integrations>/<system>/last-sync.json`.
//! Port of `work-item-sync-baseline.sh`'s read/render contract.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use tracker::RemoteTimestamp;

/// One item's baseline record, exactly as persisted: all three fields are
/// raw strings, with an empty string meaning absent — the same convention
/// bash's `// empty` plus `[ -n ]` uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub remote_updated_at: RemoteTimestamp,
    pub remote_hash: String,
    pub local_hash: String,
}

/// Whether reading the document had to degrade, and how.
///
/// Reported rather than swallowed, even though behaviour is unaffected: a
/// conflict-markered `last-sync.json` turns one silent parse failure into a
/// corpus-wide re-sync with nothing pointing at the cause unless this is
/// surfaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Degradation {
    None,
    /// The whole document was not valid JSON (or not a JSON object) —
    /// degrades to an empty baseline, forcing a full re-hash.
    Unparseable {
        detail: String,
    },
    /// The document parsed, but one or more entries were not JSON objects
    /// and could not be read at all. Every other entry is preserved.
    EntriesDiscarded {
        ids: Vec<String>,
    },
}

pub struct Baseline {
    timestamp: u64,
    items: BTreeMap<String, Entry>,
}

fn stamp_to_json(stamp: &RemoteTimestamp) -> &str {
    stamp.reported().unwrap_or("")
}

fn stamp_from_json(raw: &str) -> RemoteTimestamp {
    if raw.is_empty() {
        RemoteTimestamp::NotRead
    } else {
        RemoteTimestamp::Reported(raw.to_owned())
    }
}

/// Reads one string field, tolerating a missing or wrongly-typed value as
/// empty — mirroring bash's `jq -r '.field // empty'`. Never discards the
/// entry over one bad field: only a whole-entry problem (not a JSON object
/// at all) does that.
fn string_field(object: &serde_json::Map<String, Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn parse_entry(value: &Value) -> Option<Entry> {
    let object = value.as_object()?;
    Some(Entry {
        remote_updated_at: stamp_from_json(&string_field(
            object,
            "remote_updated_at",
        )),
        remote_hash: string_field(object, "remote_hash"),
        local_hash: string_field(object, "local_hash"),
    })
}

fn render_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn render_entry(entry: &Entry) -> String {
    format!(
        "{{\"remote_updated_at\":{},\"remote_hash\":{},\"local_hash\":{}}}",
        render_string(stamp_to_json(&entry.remote_updated_at)),
        render_string(&entry.remote_hash),
        render_string(&entry.local_hash),
    )
}

impl Baseline {
    const fn empty() -> Self {
        Self {
            timestamp: 0,
            items: BTreeMap::new(),
        }
    }

    /// Reads the document. `None` (no file yet) and unparseable content
    /// both degrade to an empty baseline — every baseline hash absent, so
    /// every item classifies fully re-hashed rather than crashing — but
    /// only unparseable content is reported as a [`Degradation`]: a missing
    /// file is the routine first-sync state, not a problem.
    #[must_use]
    pub fn read(content: Option<&str>) -> (Self, Degradation) {
        let Some(raw) = content else {
            return (Self::empty(), Degradation::None);
        };

        let parsed = serde_json::from_str::<Value>(raw)
            .ok()
            .and_then(|value| value.as_object().cloned());
        let Some(object) = parsed else {
            return (
                Self::empty(),
                Degradation::Unparseable {
                    detail: "content is not a JSON object".to_owned(),
                },
            );
        };

        // A missing or non-integer timestamp reads as 0 — bash is tolerant
        // twice over (`jq -r '.timestamp // 0'` plus the classify script's
        // own non-numeric coercion) — with every entry still preserved.
        let timestamp =
            object.get("timestamp").and_then(Value::as_u64).unwrap_or(0);

        let mut items = BTreeMap::new();
        let mut discarded = Vec::new();
        if let Some(raw_items) = object.get("items").and_then(Value::as_object)
        {
            for (id, raw_entry) in raw_items {
                match parse_entry(raw_entry) {
                    Some(entry) => {
                        items.insert(id.clone(), entry);
                    }
                    None => discarded.push(id.clone()),
                }
            }
        }

        let degradation = if discarded.is_empty() {
            Degradation::None
        } else {
            Degradation::EntriesDiscarded { ids: discarded }
        };

        (Self { timestamp, items }, degradation)
    }

    /// Compact single-line JSON, `timestamp` before `items`, with a
    /// trailing newline — matching `jq -c`'s own output and the field
    /// order `work-item-sync-baseline.sh`'s own `jq` calls construct.
    ///
    /// Built by hand rather than through `serde_json::Value`: this crate's
    /// `serde_json` runs without `preserve_order`, so a `Map`/`Value`
    /// serialises its keys alphabetically (`items` before `timestamp`),
    /// which `project_remote`'s canonicalisation depends on elsewhere in
    /// this same crate — turning that feature on to fix ordering here would
    /// break it there.
    #[must_use]
    pub fn render(&self) -> String {
        let mut items = String::from("{");
        for (index, (id, entry)) in self.items.iter().enumerate() {
            if index > 0 {
                items.push(',');
            }
            items.push_str(&render_string(id));
            items.push(':');
            items.push_str(&render_entry(entry));
        }
        items.push('}');
        format!("{{\"timestamp\":{},\"items\":{items}}}\n", self.timestamp)
    }

    pub fn set(&mut self, id: &str, entry: Entry) {
        self.items.insert(id.to_owned(), entry);
    }

    pub fn remove(&mut self, id: &str) {
        self.items.remove(id);
    }

    pub const fn set_timestamp(&mut self, epoch: u64) {
        self.timestamp = epoch;
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.items.get(id)
    }

    #[must_use]
    pub const fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// `<integrations>/<integration>/last-sync.json`, matching
/// `_wisb_path` (`work-item-sync-baseline.sh:54-75`).
#[must_use]
pub fn path(integrations_dir: &Path, integration: &str) -> PathBuf {
    integrations_dir.join(integration).join("last-sync.json")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::path;
    use super::Baseline;
    use super::Degradation;
    use super::Entry;
    use std::path::Path;
    use tracker::RemoteTimestamp;

    #[test]
    fn a_missing_baseline_reads_as_empty_with_no_degradation() {
        let (baseline, degradation) = Baseline::read(None);
        assert_eq!(baseline.timestamp(), 0);
        assert_eq!(baseline.get("0001"), None);
        assert_eq!(degradation, Degradation::None);
    }

    #[test]
    fn unparseable_content_reads_as_empty_and_reports_degradation() {
        let (baseline, degradation) =
            Baseline::read(Some("<<<<<<< HEAD\nconflict\n"));
        assert_eq!(baseline.timestamp(), 0);
        assert!(matches!(degradation, Degradation::Unparseable { .. }));
    }

    #[test]
    fn a_non_integer_timestamp_reads_as_zero_with_entries_preserved() {
        let (baseline, degradation) = Baseline::read(Some(
            r#"{"timestamp":"not-a-number","items":{"0001":{"remote_updated_at":"x","remote_hash":"h","local_hash":"h"}}}"#,
        ));
        assert_eq!(baseline.timestamp(), 0);
        assert!(baseline.get("0001").is_some());
        assert_eq!(degradation, Degradation::None);
    }

    #[test]
    fn one_malformed_entry_is_discarded_and_others_survive() {
        let (baseline, degradation) = Baseline::read(Some(
            r#"{"timestamp":1,"items":{"0001":"not-an-object","0002":{"remote_updated_at":"x","remote_hash":"h","local_hash":"h"}}}"#,
        ));
        assert_eq!(baseline.get("0001"), None);
        assert!(baseline.get("0002").is_some());
        assert_eq!(
            degradation,
            Degradation::EntriesDiscarded {
                ids: vec!["0001".to_owned()]
            }
        );
    }

    #[test]
    fn a_missing_field_within_an_entry_reads_as_absent_not_discarded() {
        let (baseline, degradation) = Baseline::read(Some(
            r#"{"timestamp":1,"items":{"0001":{"remote_hash":"h"}}}"#,
        ));
        let entry = baseline.get("0001").expect("entry survives");
        assert_eq!(entry.local_hash, "");
        assert_eq!(entry.remote_updated_at, RemoteTimestamp::NotRead);
        assert_eq!(degradation, Degradation::None);
    }

    #[test]
    fn render_round_trips_through_read() {
        let mut baseline = Baseline::empty();
        baseline.set_timestamp(1_700_000_000);
        baseline.set(
            "0001",
            Entry {
                remote_updated_at: RemoteTimestamp::Reported(
                    "2026-06-01T00:00:00Z".to_owned(),
                ),
                remote_hash: "rh".to_owned(),
                local_hash: "lh".to_owned(),
            },
        );

        let rendered = baseline.render();
        let (read_back, degradation) = Baseline::read(Some(&rendered));

        assert_eq!(degradation, Degradation::None);
        assert_eq!(read_back.timestamp(), 1_700_000_000);
        assert_eq!(read_back.get("0001"), baseline.get("0001"));
    }

    #[test]
    fn render_emits_timestamp_before_items_with_a_trailing_newline() {
        let baseline = Baseline::empty();
        let rendered = baseline.render();
        assert!(rendered.starts_with("{\"timestamp\":0,\"items\":"));
        assert!(rendered.ends_with('\n'));
        assert_eq!(rendered.matches('\n').count(), 1);
    }

    #[test]
    fn a_reported_stamp_round_trips_but_not_reported_collapses_to_not_read() {
        let mut baseline = Baseline::empty();
        baseline.set(
            "0001",
            Entry {
                remote_updated_at: RemoteTimestamp::NotReported,
                remote_hash: String::new(),
                local_hash: String::new(),
            },
        );
        let (read_back, _) = Baseline::read(Some(&baseline.render()));
        assert_eq!(
            read_back
                .get("0001")
                .expect("entry present")
                .remote_updated_at,
            RemoteTimestamp::NotRead
        );
    }

    #[test]
    fn an_empty_string_hash_reads_back_as_an_empty_string() {
        let mut baseline = Baseline::empty();
        baseline.set(
            "0001",
            Entry {
                remote_updated_at: RemoteTimestamp::NotRead,
                remote_hash: String::new(),
                local_hash: String::new(),
            },
        );
        let (read_back, _) = Baseline::read(Some(&baseline.render()));
        let entry = read_back.get("0001").expect("entry present");
        assert_eq!(entry.remote_hash, "");
        assert_eq!(entry.local_hash, "");
    }

    #[test]
    fn path_inserts_the_integration_segment() {
        let resolved =
            path(Path::new("/repo/.accelerator/state/integrations"), "jira");
        assert_eq!(
            resolved,
            Path::new(
                "/repo/.accelerator/state/integrations/jira/last-sync.json"
            )
        );
    }
}

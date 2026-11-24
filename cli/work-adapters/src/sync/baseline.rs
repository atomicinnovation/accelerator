//! The sync baseline document: `<integrations>/<system>/last-sync.json`.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Map;
use serde_json::Value;
use tracker::ExternalId;
use tracker::RemoteTimestamp;
use work::sync::BaselineFingerprints;
use work::sync::DualRecipeFingerprints;
use work::sync::LegacyHashes;
use work::sync::LocalFingerprints;
use work::sync::RemoteFingerprints;
use work::sync::SplitFingerprints;

use crate::sync::push_fidelity::RewriteNotice;
use crate::sync::rewrite_mark::render_object;
use crate::sync::rewrite_mark::render_string;
use crate::sync::rewrite_mark::MarkChange;
use crate::sync::rewrite_mark::MarkState;
use crate::sync::rewrite_mark::RewriteMark;

/// The digest recipe a baseline entry was recorded under. An entry with no
/// `digest_recipe` key is the legacy recipe.
pub struct DigestRecipe;

impl DigestRecipe {
    /// Bumped by any change that moves a recorded digest: the canonical
    /// body form, the tracker's pushed-body round trip, the synced field
    /// set, or the Markdown parser's output. The golden digests only sample
    /// the parser's output, so a parser release joins the current recipe
    /// only when its upstream diff touches nothing but disabled extensions
    /// or non-parse code and every golden digest still holds.
    pub const CURRENT: u64 = 1;
    const KEY: &'static str = "digest_recipe";
}

/// What every recipe records beside its digests.
///
/// `local_synced_at` is the item's own change-detection watermark — the epoch
/// the mtime pre-filter gates against. An entry read from a document written
/// before this field existed backfills it from the document-level `timestamp`,
/// so gating is unchanged for an old baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStamps {
    pub remote_updated_at: RemoteTimestamp,
    pub local_synced_at: u64,
}

/// One item's baseline record.
///
/// A split entry keeps the legacy hashes of the bodies it was recorded from,
/// so a binary that knows only the legacy recipe still reads it as a valid
/// entry. An entry under a recipe this binary does not know is carried
/// unchanged, so a newer binary's baseline survives this one's writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Legacy {
        stamps: SyncStamps,
        legacy: LegacyHashes,
    },
    Split(SplitEntry),
    Unsupported {
        raw: Value,
    },
}

/// A recipe-1 entry, with the rewrite mark it carries and every key a later
/// build wrote that this one does not know, so neither is lost on a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitEntry {
    stamps: SyncStamps,
    legacy: LegacyHashes,
    split: SplitFingerprints,
    mark: MarkState,
    unknown: BTreeMap<String, Value>,
}

impl SplitEntry {
    const KNOWN_KEYS: [&'static str; 11] = [
        DigestRecipe::KEY,
        "local_body_hash",
        "local_fields_hash",
        "local_frontmatter_hash",
        "local_hash",
        "local_synced_at",
        "remote_body_hash",
        "remote_fields_hash",
        "remote_hash",
        "remote_updated_at",
        RewriteMark::KEY,
    ];

    const fn unmarked(
        stamps: SyncStamps,
        legacy: LegacyHashes,
        split: SplitFingerprints,
    ) -> Self {
        Self {
            stamps,
            legacy,
            split,
            mark: MarkState::Absent,
            unknown: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn stamps(&self) -> &SyncStamps {
        &self.stamps
    }

    #[must_use]
    pub const fn legacy(&self) -> &LegacyHashes {
        &self.legacy
    }

    #[must_use]
    pub const fn split(&self) -> &SplitFingerprints {
        &self.split
    }

    #[must_use]
    pub const fn mark(&self) -> &MarkState {
        &self.mark
    }

    /// The same entry with its mark as `change` leaves it.
    #[must_use]
    pub fn marked(self, change: MarkChange<'_>) -> Self {
        Self {
            mark: self.mark.after(change),
            ..self
        }
    }

    /// This entry in place of `previous`, with the mark `change` leaves
    /// over `previous`'s and the keys `previous` held that this build does
    /// not know.
    #[must_use]
    pub fn superseding(self, previous: &Self, change: MarkChange<'_>) -> Self {
        Self {
            mark: previous.mark.after(change),
            unknown: previous.unknown.clone(),
            ..self
        }
    }

    /// The same entry with its local digests moved to a new local file's.
    #[must_use]
    pub fn with_local_fingerprints(
        self,
        legacy_local: String,
        local: LocalFingerprints,
    ) -> Self {
        Self {
            legacy: LegacyHashes {
                local: Some(legacy_local),
                ..self.legacy
            },
            split: SplitFingerprints {
                local: Some(local),
                ..self.split
            },
            ..self
        }
    }

    #[must_use]
    pub fn without_local_digests(self) -> Self {
        Self {
            legacy: LegacyHashes {
                local: None,
                ..self.legacy
            },
            split: SplitFingerprints {
                local: None,
                ..self.split
            },
            ..self
        }
    }

    #[cfg(test)]
    pub(crate) fn with_mark(self, mark: MarkState) -> Self {
        Self { mark, ..self }
    }

    fn parse(
        object: &Map<String, Value>,
        stamps: SyncStamps,
        legacy: LegacyHashes,
    ) -> Self {
        Self {
            stamps,
            legacy,
            split: split_from(object),
            mark: MarkState::parse(object),
            unknown: object
                .iter()
                .filter(|(key, _)| !Self::KNOWN_KEYS.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        }
    }

    fn render(&self) -> String {
        let local = self.split.local.as_ref();
        let remote = self.split.remote.as_ref();
        let mut fields: BTreeMap<String, String> = self
            .unknown
            .iter()
            .map(|(key, value)| (key.clone(), value.to_string()))
            .collect();
        for (key, value) in [
            (DigestRecipe::KEY, DigestRecipe::CURRENT.to_string()),
            (
                "local_body_hash",
                render_digest(local.map(|local| &local.body)),
            ),
            (
                "local_fields_hash",
                render_digest(local.map(|local| &local.fields)),
            ),
            (
                "local_frontmatter_hash",
                render_digest(local.map(|local| &local.frontmatter)),
            ),
            ("local_hash", render_digest(self.legacy.local.as_ref())),
            ("local_synced_at", self.stamps.local_synced_at.to_string()),
            (
                "remote_body_hash",
                render_digest(remote.map(|remote| &remote.body)),
            ),
            (
                "remote_fields_hash",
                render_digest(remote.map(|remote| &remote.fields)),
            ),
            ("remote_hash", render_digest(self.legacy.remote.as_ref())),
            (
                "remote_updated_at",
                render_string(stamp_to_json(&self.stamps.remote_updated_at)),
            ),
        ] {
            fields.insert(key.to_owned(), value);
        }
        if let Some(mark) = self.mark.rendered() {
            fields.insert(RewriteMark::KEY.to_owned(), mark);
        }
        render_object(&fields)
    }
}

impl Entry {
    /// The rewrite an earlier push left standing on the item at `path`
    /// known remotely as `external_id`, if this entry still marks one.
    #[must_use]
    pub fn standing_rewrite(
        &self,
        path: &Path,
        external_id: &ExternalId,
    ) -> Option<RewriteNotice> {
        let Self::Split(entry) = self else {
            return None;
        };
        entry
            .mark()
            .mark()
            .filter(|mark| !mark.is_accepted())
            .map(|mark| mark.notice(path, external_id))
    }

    /// The notice a push leaves on the item at `path` known remotely as
    /// `external_id`, once this entry records it: none while the entry
    /// holds an accepted rewrite, else the push's own notice, else the
    /// rewrite the entry still stands on.
    #[must_use]
    pub fn rewrite_after_push(
        &self,
        on_push: Option<RewriteNotice>,
        path: &Path,
        external_id: &ExternalId,
    ) -> Option<RewriteNotice> {
        if self.holds_an_accepted_rewrite() {
            return None;
        }
        on_push.or_else(|| self.standing_rewrite(path, external_id))
    }

    fn holds_an_accepted_rewrite(&self) -> bool {
        matches!(self, Self::Split(entry) if entry.mark().mark().is_some_and(RewriteMark::is_accepted))
    }

    /// A legacy entry from the strings an earlier release persisted, an
    /// empty hash meaning absent.
    #[must_use]
    pub fn persisted_legacy(
        remote_updated_at: RemoteTimestamp,
        remote_hash: String,
        local_hash: String,
        local_synced_at: u64,
    ) -> Self {
        let recorded =
            |hash: String| Some(hash).filter(|hash| !hash.is_empty());
        Self::Legacy {
            stamps: SyncStamps {
                remote_updated_at,
                local_synced_at,
            },
            legacy: LegacyHashes {
                local: recorded(local_hash),
                remote: recorded(remote_hash),
            },
        }
    }

    /// A recipe-1 entry recording both recipes' fingerprints.
    #[must_use]
    pub fn recorded(
        stamps: SyncStamps,
        fingerprints: DualRecipeFingerprints,
    ) -> Self {
        Self::Split(SplitEntry::unmarked(
            stamps,
            fingerprints.legacy,
            fingerprints.split,
        ))
    }

    #[must_use]
    pub const fn stamps(&self) -> Option<&SyncStamps> {
        match self {
            Self::Legacy { stamps, .. }
            | Self::Split(SplitEntry { stamps, .. }) => Some(stamps),
            Self::Unsupported { .. } => None,
        }
    }

    #[must_use]
    pub const fn legacy(&self) -> Option<&LegacyHashes> {
        match self {
            Self::Legacy { legacy, .. }
            | Self::Split(SplitEntry { legacy, .. }) => Some(legacy),
            Self::Unsupported { .. } => None,
        }
    }

    #[must_use]
    pub const fn split(&self) -> Option<&SplitFingerprints> {
        match self {
            Self::Split(SplitEntry { split, .. }) => Some(split),
            Self::Legacy { .. } | Self::Unsupported { .. } => None,
        }
    }

    /// The fingerprints classification compares under this entry's recipe;
    /// `None` for an entry under a recipe this binary does not know.
    #[must_use]
    pub fn fingerprints(&self) -> Option<BaselineFingerprints> {
        match self {
            Self::Legacy { legacy, .. } => {
                Some(BaselineFingerprints::Legacy(legacy.clone()))
            }
            Self::Split(SplitEntry { split, .. }) => {
                Some(BaselineFingerprints::Split(split.clone()))
            }
            Self::Unsupported { .. } => None,
        }
    }

    /// The same entry with every local digest forgotten, so the next run
    /// reads the local side as changed.
    #[must_use]
    pub fn without_local_digests(self) -> Self {
        match self {
            Self::Legacy { stamps, legacy } => Self::Legacy {
                stamps,
                legacy: LegacyHashes {
                    local: None,
                    ..legacy
                },
            },
            Self::Split(split) => Self::Split(split.without_local_digests()),
            unsupported @ Self::Unsupported { .. } => unsupported,
        }
    }

    const fn stamps_mut(&mut self) -> Option<&mut SyncStamps> {
        match self {
            Self::Legacy { stamps, .. }
            | Self::Split(SplitEntry { stamps, .. }) => Some(stamps),
            Self::Unsupported { .. } => None,
        }
    }
}

/// Whether reading the document had to degrade, and how.
///
/// Reported rather than swallowed even though behaviour is unaffected: a
/// conflict-markered `last-sync.json` turns one silent parse failure into a
/// corpus-wide re-sync with nothing pointing at the cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Degradation {
    None,
    /// Degrades to an empty baseline, forcing a full re-hash.
    Unparseable {
        detail: String,
    },
    /// One or more entries were not JSON objects. Every other entry is
    /// preserved.
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

/// Tolerates a missing or wrongly-typed value as empty. One bad field never
/// discards the entry; only a whole-entry problem does.
fn string_field(object: &Map<String, Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

/// The persisted empty string is an absent digest; it never leaves the
/// codec.
fn digest_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    Some(string_field(object, key)).filter(|digest| !digest.is_empty())
}

/// A missing or non-integer watermark reads as the document `timestamp`, never
/// a raw untrusted value, so an old baseline gates exactly as before and a
/// corrupt or future-written value cannot bury an edit.
fn synced_at_field(
    object: &Map<String, Value>,
    document_timestamp: u64,
) -> u64 {
    object
        .get("local_synced_at")
        .and_then(Value::as_u64)
        .unwrap_or(document_timestamp)
}

fn stamps_from(
    object: &Map<String, Value>,
    document_timestamp: u64,
) -> SyncStamps {
    SyncStamps {
        remote_updated_at: stamp_from_json(&string_field(
            object,
            "remote_updated_at",
        )),
        local_synced_at: synced_at_field(object, document_timestamp),
    }
}

fn legacy_from(object: &Map<String, Value>) -> LegacyHashes {
    LegacyHashes {
        local: digest_field(object, "local_hash"),
        remote: digest_field(object, "remote_hash"),
    }
}

/// A side whose digests are not all recorded reads as absent, so it can
/// only ever classify as changed.
fn split_from(object: &Map<String, Value>) -> SplitFingerprints {
    let local = match (
        digest_field(object, "local_frontmatter_hash"),
        digest_field(object, "local_fields_hash"),
        digest_field(object, "local_body_hash"),
    ) {
        (Some(frontmatter), Some(fields), Some(body)) => {
            Some(LocalFingerprints {
                frontmatter,
                fields,
                body,
            })
        }
        _ => None,
    };
    let remote = match (
        digest_field(object, "remote_body_hash"),
        digest_field(object, "remote_fields_hash"),
    ) {
        (Some(body), Some(fields)) => Some(RemoteFingerprints { body, fields }),
        _ => None,
    };
    SplitFingerprints { local, remote }
}

fn is_supported_recipe(marker: &Value) -> bool {
    marker
        .as_u64()
        .is_some_and(|recipe| (1..=DigestRecipe::CURRENT).contains(&recipe))
}

fn parse_entry(value: &Value, document_timestamp: u64) -> Option<Entry> {
    let object = value.as_object()?;
    let stamps = stamps_from(object, document_timestamp);
    let legacy = legacy_from(object);
    Some(match object.get(DigestRecipe::KEY) {
        None => Entry::Legacy { stamps, legacy },
        Some(marker) if is_supported_recipe(marker) => {
            Entry::Split(SplitEntry::parse(object, stamps, legacy))
        }
        Some(_) => Entry::Unsupported { raw: value.clone() },
    })
}

fn render_digest(digest: Option<&String>) -> String {
    render_string(digest.map_or("", String::as_str))
}

fn render_legacy(stamps: &SyncStamps, legacy: &LegacyHashes) -> String {
    format!(
        "{{\"remote_updated_at\":{},\"remote_hash\":{},\"local_hash\":{},\
         \"local_synced_at\":{}}}",
        render_string(stamp_to_json(&stamps.remote_updated_at)),
        render_digest(legacy.remote.as_ref()),
        render_digest(legacy.local.as_ref()),
        stamps.local_synced_at,
    )
}

fn render_entry(entry: &Entry) -> String {
    match entry {
        Entry::Legacy { stamps, legacy } => render_legacy(stamps, legacy),
        Entry::Split(split) => split.render(),
        Entry::Unsupported { raw } => raw.to_string(),
    }
}

impl Baseline {
    const fn empty() -> Self {
        Self {
            timestamp: 0,
            items: BTreeMap::new(),
        }
    }

    /// A missing file and unparseable content both degrade to an empty
    /// baseline, forcing a full re-hash rather than crashing. Only the
    /// latter is a [`Degradation`]: a missing file is the routine
    /// first-sync state.
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

        // A missing or non-integer timestamp reads as 0, preserving every
        // entry rather than discarding the document.
        let timestamp =
            object.get("timestamp").and_then(Value::as_u64).unwrap_or(0);

        let mut items = BTreeMap::new();
        let mut discarded = Vec::new();
        if let Some(raw_items) = object.get("items").and_then(Value::as_object)
        {
            for (id, raw_entry) in raw_items {
                match parse_entry(raw_entry, timestamp) {
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
    /// trailing newline.
    ///
    /// Built by hand rather than through `serde_json::Value`: this crate's
    /// `serde_json` runs without `preserve_order`, so a `Map` would
    /// serialise alphabetically, and enabling that feature to fix the order
    /// here would break the canonicalisation `project_remote` depends on.
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

    pub(crate) fn set(&mut self, id: &str, entry: Entry) {
        self.items.insert(id.to_owned(), entry);
    }

    /// Sets `id`'s entry exactly as given, for a test to start from.
    #[cfg(feature = "test-support")]
    pub fn seed(&mut self, id: &str, entry: Entry) {
        self.set(id, entry);
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
    pub fn ids(&self) -> Vec<String> {
        self.items.keys().cloned().collect()
    }

    /// Advances one entry's change-detection watermark. A no-op for an id with
    /// no entry, or one under an unknown recipe: an item is watermarked only
    /// once it has a baseline record this binary can write.
    pub fn advance_watermark(&mut self, id: &str, epoch: u64) {
        if let Some(stamps) = self.items.get_mut(id).and_then(Entry::stamps_mut)
        {
            stamps.local_synced_at = epoch;
        }
    }

    #[must_use]
    pub const fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// `<integrations>/<integration>/last-sync.json`.
#[must_use]
pub fn path(integrations_dir: &Path, integration: &str) -> PathBuf {
    integrations_dir.join(integration).join("last-sync.json")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::Path;

    use serde_json::Value;
    use tracker::RemoteTimestamp;
    use work::sync::DualRecipeFingerprints;
    use work::sync::LegacyHashes;
    use work::sync::LocalFingerprints;
    use work::sync::RemoteFingerprints;
    use work::sync::SplitFingerprints;

    use super::path;
    use super::Baseline;
    use super::Degradation;
    use super::Entry;
    use super::SyncStamps;
    use crate::sync::push_fidelity::ChangedLine;
    use crate::sync::push_fidelity::ChangedLines;
    use crate::sync::push_fidelity::PushFidelity;
    use crate::sync::push_fidelity::RewriteKind;
    use crate::sync::push_fidelity::RewrittenLines;
    use crate::sync::rewrite_mark::MarkChange;
    use crate::sync::rewrite_mark::MarkState;
    use crate::sync::rewrite_mark::RewriteMark;

    fn stamps(
        remote_updated_at: RemoteTimestamp,
        synced_at: u64,
    ) -> SyncStamps {
        SyncStamps {
            remote_updated_at,
            local_synced_at: synced_at,
        }
    }

    fn legacy_entry(remote: &str, local: &str, synced_at: u64) -> Entry {
        let digest = |raw: &str| Some(raw.to_owned()).filter(|d| !d.is_empty());
        Entry::Legacy {
            stamps: stamps(RemoteTimestamp::NotRead, synced_at),
            legacy: LegacyHashes {
                local: digest(local),
                remote: digest(remote),
            },
        }
    }

    fn split_entry() -> Entry {
        Entry::recorded(
            stamps(
                RemoteTimestamp::Reported("2026-06-01T00:00:00Z".to_owned()),
                1_699_000_000,
            ),
            DualRecipeFingerprints {
                legacy: LegacyHashes {
                    local: Some("legacy-local".to_owned()),
                    remote: Some("legacy-remote".to_owned()),
                },
                split: SplitFingerprints {
                    local: Some(LocalFingerprints {
                        frontmatter: "frontmatter".to_owned(),
                        fields: "local-fields".to_owned(),
                        body: "local-body".to_owned(),
                    }),
                    remote: Some(RemoteFingerprints {
                        body: "remote-body".to_owned(),
                        fields: "remote-fields".to_owned(),
                    }),
                },
            },
        )
    }

    fn document_with(entry: &str) -> String {
        format!("{{\"timestamp\":1700000000,\"items\":{{\"0001\":{entry}}}}}\n")
    }

    fn read_one(entry: &str) -> (Entry, Degradation) {
        let (baseline, degradation) =
            Baseline::read(Some(&document_with(entry)));
        (
            baseline.get("0001").cloned().expect("entry present"),
            degradation,
        )
    }

    fn rendered_entry(entry: Entry) -> String {
        let mut baseline = Baseline::empty();
        baseline.set_timestamp(1_700_000_000);
        baseline.set("0001", entry);
        let rendered = baseline.render();
        rendered
            .strip_prefix("{\"timestamp\":1700000000,\"items\":{\"0001\":")
            .and_then(|rest| rest.strip_suffix("}}\n"))
            .expect("one entry")
            .to_owned()
    }

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
    fn an_entry_watermark_round_trips_through_render_and_read() {
        let mut baseline = Baseline::empty();
        baseline.set_timestamp(1_700_000_000);
        baseline.set("0001", legacy_entry("", "", 1_699_500_000));
        let (read_back, _) = Baseline::read(Some(&baseline.render()));
        assert_eq!(
            read_back
                .get("0001")
                .and_then(Entry::stamps)
                .expect("entry present")
                .local_synced_at,
            1_699_500_000
        );
    }

    #[test]
    fn an_entry_without_a_watermark_backfills_from_the_document_timestamp() {
        let (entry, degradation) = read_one(
            r#"{"remote_updated_at":"x","remote_hash":"h","local_hash":"h"}"#,
        );
        assert_eq!(
            entry.stamps().expect("stamps").local_synced_at,
            1_700_000_000
        );
        assert_eq!(degradation, Degradation::None);
    }

    #[test]
    fn a_non_integer_watermark_backfills_from_the_document_timestamp() {
        let (entry, _) = read_one(
            r#"{"remote_updated_at":"x","remote_hash":"h","local_hash":"h","local_synced_at":"corrupt"}"#,
        );
        assert_eq!(
            entry.stamps().expect("stamps").local_synced_at,
            1_700_000_000
        );
    }

    #[test]
    fn a_missing_field_within_an_entry_reads_as_absent_not_discarded() {
        let (entry, degradation) = read_one(r#"{"remote_hash":"h"}"#);
        let Entry::Legacy { stamps, legacy } = entry else {
            panic!("an unmarked entry is legacy");
        };
        assert_eq!(legacy.local, None);
        assert_eq!(stamps.remote_updated_at, RemoteTimestamp::NotRead);
        assert_eq!(degradation, Degradation::None);
    }

    #[test]
    fn render_round_trips_through_read() {
        let mut baseline = Baseline::empty();
        baseline.set_timestamp(1_700_000_000);
        baseline.set(
            "0001",
            Entry::Legacy {
                stamps: stamps(
                    RemoteTimestamp::Reported(
                        "2026-06-01T00:00:00Z".to_owned(),
                    ),
                    1_699_000_000,
                ),
                legacy: LegacyHashes {
                    local: Some("lh".to_owned()),
                    remote: Some("rh".to_owned()),
                },
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
            Entry::Legacy {
                stamps: stamps(RemoteTimestamp::NotReported, 0),
                legacy: LegacyHashes::default(),
            },
        );
        let (read_back, _) = Baseline::read(Some(&baseline.render()));
        assert_eq!(
            read_back
                .get("0001")
                .and_then(Entry::stamps)
                .expect("entry present")
                .remote_updated_at,
            RemoteTimestamp::NotRead
        );
    }

    #[test]
    fn an_absent_legacy_hash_renders_as_empty_and_reads_back_as_absent() {
        let rendered = rendered_entry(legacy_entry("", "", 0));
        assert!(rendered.contains("\"remote_hash\":\"\""), "{rendered}");
        assert!(rendered.contains("\"local_hash\":\"\""), "{rendered}");

        let (entry, _) = read_one(&rendered);
        assert_eq!(entry.legacy(), Some(&LegacyHashes::default()));
    }

    #[test]
    fn an_unmarked_entry_reads_as_legacy() {
        let (entry, _) = read_one(
            r#"{"remote_updated_at":"t","remote_hash":"r","local_hash":"l","local_synced_at":5}"#,
        );
        assert_eq!(
            entry,
            Entry::Legacy {
                stamps: stamps(RemoteTimestamp::Reported("t".to_owned()), 5),
                legacy: LegacyHashes {
                    local: Some("l".to_owned()),
                    remote: Some("r".to_owned()),
                },
            }
        );
    }

    #[test]
    fn a_legacy_entry_renders_byte_identically() {
        let raw = r#"{"remote_updated_at":"t","remote_hash":"r","local_hash":"l","local_synced_at":5}"#;
        let (entry, _) = read_one(raw);
        assert_eq!(rendered_entry(entry), raw);
    }

    #[test]
    fn a_recipe_one_entry_round_trips_through_render_and_read() {
        let (entry, degradation) = read_one(&rendered_entry(split_entry()));
        assert_eq!(entry, split_entry());
        assert_eq!(degradation, Degradation::None);
    }

    #[test]
    fn a_recipe_one_entry_renders_its_keys_in_fixed_order() {
        assert_eq!(
            rendered_entry(split_entry()),
            "{\"digest_recipe\":1,\"local_body_hash\":\"local-body\",\
             \"local_fields_hash\":\"local-fields\",\
             \"local_frontmatter_hash\":\"frontmatter\",\
             \"local_hash\":\"legacy-local\",\"local_synced_at\":1699000000,\
             \"remote_body_hash\":\"remote-body\",\
             \"remote_fields_hash\":\"remote-fields\",\
             \"remote_hash\":\"legacy-remote\",\
             \"remote_updated_at\":\"2026-06-01T00:00:00Z\"}"
        );
    }

    #[test]
    fn a_blanked_recipe_one_hash_renders_as_empty_and_reads_back_as_absent() {
        let blanked = split_entry().without_local_digests();
        let rendered = rendered_entry(blanked.clone());
        for key in [
            "local_body_hash",
            "local_fields_hash",
            "local_frontmatter_hash",
            "local_hash",
        ] {
            assert!(
                rendered.contains(&format!("\"{key}\":\"\"")),
                "{rendered}"
            );
        }

        let (entry, _) = read_one(&rendered);
        assert_eq!(entry, blanked);
        assert_eq!(entry.split().and_then(|split| split.local.as_ref()), None);
        assert_eq!(
            entry.legacy().and_then(|legacy| legacy.local.as_ref()),
            None
        );
    }

    #[test]
    fn an_unknown_recipe_entry_survives_a_write_byte_identically() {
        let raw = r#"{"digest_recipe":2,"local_body_hash":"b","novel_key":[1,2],"remote_updated_at":"t"}"#;
        let (entry, _) = read_one(raw);
        assert!(matches!(entry, Entry::Unsupported { .. }), "{entry:?}");
        assert_eq!(rendered_entry(entry), raw);
    }

    #[test]
    fn an_unknown_recipe_entry_reports_no_degradation() {
        let (_, degradation) = read_one(r#"{"digest_recipe":2}"#);
        assert_eq!(degradation, Degradation::None);
    }

    #[test]
    fn a_non_integer_recipe_marker_is_preserved_as_unsupported() {
        for marker in [r#""1""#, "1.5", "null", "-1"] {
            let raw = format!("{{\"digest_recipe\":{marker}}}");
            let (entry, _) = read_one(&raw);
            assert!(matches!(entry, Entry::Unsupported { .. }), "{raw}");
            assert_eq!(rendered_entry(entry), raw);
        }
    }

    #[test]
    fn a_zero_recipe_marker_is_preserved_as_unsupported() {
        let raw = r#"{"digest_recipe":0}"#;
        let (entry, _) = read_one(raw);
        assert!(matches!(entry, Entry::Unsupported { .. }), "{entry:?}");
        assert_eq!(rendered_entry(entry), raw);
    }

    #[test]
    fn an_unknown_recipe_entry_keeps_its_watermark() {
        let raw = r#"{"digest_recipe":2,"local_synced_at":5}"#;
        let (baseline, _) = Baseline::read(Some(&document_with(raw)));
        let mut baseline = baseline;
        baseline.advance_watermark("0001", 9);
        assert_eq!(
            baseline.render(),
            document_with(raw),
            "an entry this binary cannot write is never rewritten"
        );
    }

    #[test]
    fn removing_an_unsupported_entry_drops_it() {
        let (mut baseline, _) =
            Baseline::read(Some(&document_with(r#"{"digest_recipe":2}"#)));
        baseline.remove("0001");
        assert_eq!(baseline.get("0001"), None);
    }

    #[test]
    fn a_recipe_one_entry_rewritten_by_the_legacy_writer_reads_back_as_a_valid_legacy_entry(
    ) {
        let recipe_one: Value =
            serde_json::from_str(&rendered_entry(split_entry()))
                .expect("rendered entry is JSON");
        let legacy_writer_output = format!(
            "{{\"remote_updated_at\":{},\"remote_hash\":{},\"local_hash\":{},\
             \"local_synced_at\":{}}}",
            recipe_one["remote_updated_at"],
            recipe_one["remote_hash"],
            recipe_one["local_hash"],
            recipe_one["local_synced_at"],
        );

        let (entry, _) = read_one(&legacy_writer_output);

        let Entry::Split(split) = split_entry() else {
            unreachable!("the fixture entry is split");
        };
        assert_eq!(
            entry,
            Entry::Legacy {
                stamps: split.stamps().clone(),
                legacy: split.legacy().clone(),
            }
        );
    }

    const MARK: &str = "\"rewritten_on_push\":{\"lines\":[{\"section\":\
        \"Notes\",\"sent\":\"See crates.io.\",\"stored\":\
        \"See [crates.io](<http://crates.io>).\"}],\"more\":7,\
        \"sections\":[\"Notes\"]}";

    const UNMARKED: &str = "{\"digest_recipe\":1,\
        \"local_body_hash\":\"local-body\",\
        \"local_fields_hash\":\"local-fields\",\
        \"local_frontmatter_hash\":\"frontmatter\",\
        \"local_hash\":\"legacy-local\",\"local_synced_at\":1699000000,\
        \"remote_body_hash\":\"remote-body\",\
        \"remote_fields_hash\":\"remote-fields\",\
        \"remote_hash\":\"legacy-remote\",\
        \"remote_updated_at\":\"2026-06-01T00:00:00Z\"}";

    fn with_key(unmarked: &str, member: &str) -> String {
        let inner = unmarked
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
            .expect("an object");
        let mut members: Vec<&str> = inner.split(',').collect();
        members.push(member);
        members.sort_by_key(|member| member.split(':').next());
        format!("{{{}}}", members.join(","))
    }

    fn changed(section: &str, sent: &str, stored: &str) -> ChangedLine {
        ChangedLine {
            section: section.to_owned(),
            sent: sent.to_owned(),
            stored: stored.to_owned(),
        }
    }

    fn crates_io_mark(more: usize) -> RewriteMark {
        RewriteMark::from(RewrittenLines {
            sections: vec!["Notes".to_owned()],
            lines: vec![changed(
                "Notes",
                "See crates.io.",
                "See [crates.io](<http://crates.io>).",
            )],
            more,
        })
    }

    fn marked(state: MarkState) -> Entry {
        let Entry::Split(split) = split_entry() else {
            unreachable!("the fixture entry is split");
        };
        Entry::Split(split.with_mark(state))
    }

    fn mark_of(entry: &Entry) -> &MarkState {
        let Entry::Split(split) = entry else {
            panic!("a split entry: {entry:?}");
        };
        split.mark()
    }

    #[test]
    fn only_a_marked_entry_holds_a_standing_rewrite() {
        let path = Path::new("meta/work/0001.md");
        let key = tracker::ExternalId::new("PP-1".to_owned());
        let Entry::Split(split) = split_entry() else {
            unreachable!("the fixture entry is split");
        };
        let legacy = Entry::Legacy {
            stamps: split.stamps().clone(),
            legacy: split.legacy().clone(),
        };

        let standing = marked(MarkState::Marked(Box::new(crates_io_mark(2))))
            .standing_rewrite(path, &key);

        assert_eq!(
            standing.map(|notice| (notice.kind, notice.lines)),
            Some((
                RewriteKind::StillOnPush,
                crates_io_mark(2).rewritten_lines()
            ))
        );
        assert_eq!(split_entry().standing_rewrite(path, &key), None);
        assert_eq!(legacy.standing_rewrite(path, &key), None);
    }

    #[test]
    fn an_accepted_rewrite_silences_its_push_and_a_marked_one_stands() {
        let path = Path::new("meta/work/0001.md");
        let key = tracker::ExternalId::new("PP-1".to_owned());
        let marked_entry =
            marked(MarkState::Marked(Box::new(crates_io_mark(0))));
        let accepted_entry = marked(
            MarkState::Marked(Box::new(crates_io_mark(0)))
                .after(MarkChange::Accept),
        );
        let pushed = marked_entry.standing_rewrite(path, &key);

        assert_eq!(
            accepted_entry.rewrite_after_push(pushed.clone(), path, &key),
            None
        );
        assert_eq!(
            marked_entry.rewrite_after_push(None, path, &key),
            marked_entry.standing_rewrite(path, &key)
        );
        assert_eq!(
            split_entry().rewrite_after_push(pushed.clone(), path, &key),
            pushed
        );
    }

    #[test]
    fn an_accepted_rewrite_no_longer_stands() {
        let path = Path::new("meta/work/0001.md");
        let key = tracker::ExternalId::new("PP-1".to_owned());
        let accepted = MarkState::Marked(Box::new(crates_io_mark(0)))
            .after(MarkChange::Accept);

        assert_eq!(marked(accepted).standing_rewrite(path, &key), None);
    }

    #[test]
    fn a_rewrite_mark_round_trips() {
        let entry = marked(MarkState::Marked(Box::new(crates_io_mark(7))));
        let rendered = rendered_entry(entry.clone());

        assert_eq!(rendered, with_key(UNMARKED, MARK));
        assert_eq!(read_one(&rendered).0, entry);
    }

    #[test]
    fn an_entry_without_the_mark_renders_as_before() {
        assert_eq!(rendered_entry(split_entry()), UNMARKED);
    }

    #[test]
    fn an_older_entry_reads_as_unmarked() {
        let (entry, _) = read_one(UNMARKED);

        assert_eq!(mark_of(&entry), &MarkState::Absent);
    }

    #[test]
    fn mark_text_is_rendered_as_json_strings() {
        let awkward = "quote \" slash \\ tab \t newline \n café —";
        let mark = RewriteMark::from(RewrittenLines::bounded(vec![changed(
            awkward, awkward, awkward,
        )]));
        let entry = marked(MarkState::Marked(Box::new(mark)));
        let rendered = rendered_entry(entry.clone());

        assert!(
            serde_json::from_str::<Value>(&document_with(&rendered)).is_ok(),
            "{rendered}"
        );
        assert_eq!(read_one(&rendered).0, entry);
    }

    #[test]
    fn a_malformed_mark_is_kept_raw() {
        for raw in [
            "\"rewritten_on_push\":\"garbled\"",
            "\"rewritten_on_push\":{\"more\":1}",
            "\"rewritten_on_push\":{\"lines\":[{\"section\":\"Notes\",\
             \"sent\":\"a\"}]}",
            "\"rewritten_on_push\":{\"lines\":[{\"section\":\"Notes\",\
             \"sent\":\"a\",\"stored\":3}]}",
        ] {
            let stored = with_key(UNMARKED, raw);
            let (entry, _) = read_one(&stored);

            assert!(
                matches!(mark_of(&entry), MarkState::Unreadable(_)),
                "{raw}"
            );
            assert_eq!(entry.split(), split_entry().split(), "{raw}");
            let Entry::Split(split) = entry else {
                unreachable!("a split entry");
            };
            let kept = split.marked(MarkChange::Keep);
            assert_eq!(rendered_entry(Entry::Split(kept)), stored, "{raw}");
        }
    }

    #[test]
    fn a_mark_with_a_missing_or_wrong_more_reads_as_zero() {
        for more in ["", ",\"more\":\"seven\"", ",\"more\":-1"] {
            let raw = format!(
                "\"rewritten_on_push\":{{\"lines\":[{{\"section\":\
                 \"Notes\",\"sent\":\"See crates.io.\",\"stored\":\
                 \"See [crates.io](<http://crates.io>).\"}}]{more}}}"
            );
            let (entry, _) = read_one(&with_key(UNMARKED, &raw));

            assert_eq!(
                mark_of(&entry).mark().map(RewriteMark::rewritten_lines),
                Some(crates_io_mark(0).rewritten_lines()),
                "{raw}"
            );
        }
    }

    #[test]
    fn known_keys_are_not_unknown() {
        let stored = with_key(UNMARKED, "\"novel\":[1]");
        let rendered = rendered_entry(read_one(&stored).0);

        assert_eq!(rendered, stored);
        for key in [
            "digest_recipe",
            "local_body_hash",
            "local_fields_hash",
            "local_frontmatter_hash",
            "\"local_hash",
            "local_synced_at",
            "remote_body_hash",
            "remote_fields_hash",
            "\"remote_hash",
            "remote_updated_at",
            "novel",
        ] {
            assert_eq!(rendered.matches(key).count(), 1, "{key}: {rendered}");
        }
    }

    #[test]
    fn a_new_mark_replaces_a_malformed_one() {
        let (entry, _) =
            read_one(&with_key(UNMARKED, "\"rewritten_on_push\":\"garbled\""));
        let Entry::Split(split) = entry else {
            unreachable!("a split entry");
        };
        let rewrite = PushFidelity::Rewritten {
            changes: changed(
                "Notes",
                "See crates.io.",
                "See [crates.io](<http://crates.io>).",
            )
            .into(),
        };

        let replaced = rendered_entry(Entry::Split(
            split.clone().marked(MarkChange::Judged(&rewrite)),
        ));
        assert_eq!(replaced.matches("rewritten_on_push").count(), 1);
        assert_eq!(
            replaced,
            with_key(UNMARKED, &MARK.replace("\"more\":7", "\"more\":0"))
        );

        let cleared =
            rendered_entry(Entry::Split(split.marked(MarkChange::Clear)));
        assert_eq!(cleared, UNMARKED);
    }

    #[test]
    fn extra_mark_fields_are_kept() {
        let raw = "\"rewritten_on_push\":{\"added\":true,\"lines\":[{\
                   \"later\":1,\"section\":\"Notes\",\"sent\":\"a\",\
                   \"stored\":\"b\"}],\"more\":0,\"sections\":[\"Notes\"]}";
        let stored = with_key(UNMARKED, raw);

        assert_eq!(rendered_entry(read_one(&stored).0), stored);
    }

    #[test]
    fn a_mark_is_bounded() {
        let long = format!("{}—tail", "x".repeat(199));
        let mut changes = vec![changed("Notes", &long, "stored")];
        changes.extend((1..12).map(|n| {
            changed("Notes", &format!("sent {n}"), &format!("stored {n}"))
        }));
        let entry = marked(MarkState::Marked(Box::new(crates_io_mark(0))));
        let Entry::Split(split) = entry else {
            unreachable!("a split entry");
        };

        let after =
            split.marked(MarkChange::Judged(&PushFidelity::Rewritten {
                changes: ChangedLines::of(changes.clone())
                    .unwrap_or_else(|| unreachable!("twelve changed lines")),
            }));

        let bounded = RewrittenLines::bounded(changes);
        assert_eq!(bounded.lines.len(), 5);
        assert_eq!(bounded.more, 7);
        assert_eq!(bounded.lines[0].sent, format!("{}…", "x".repeat(199)));
        assert_eq!(
            after.mark().mark().map(RewriteMark::rewritten_lines),
            Some(bounded)
        );
    }

    #[test]
    fn unknown_split_keys_survive_a_rewrite() {
        let stored = with_key(UNMARKED, "\"aardvark\":\"first\"");
        assert!(stored.starts_with("{\"aardvark\""), "{stored}");
        let (entry, _) = read_one(&stored);
        let Entry::Split(split) = entry else {
            unreachable!("a split entry");
        };

        let rendered = rendered_entry(Entry::Split(
            split.marked(MarkChange::Judged(&PushFidelity::Preserved)),
        ));

        assert_eq!(rendered, stored);
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

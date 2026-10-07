//! A `conduct` run's ledger, stored as JSON beside its set.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::AtomicWrite;
use corpus::FileRemove;
use corpus::StoreError;
use research::conduct::ledger::RunId;
use research::conduct::ledger::RunLedger;
use research::conduct::memory::PendingBatch;
use research::conduct::memory::RunMemory;
use research::conduct::observed::Observed;
use research::conduct::spawn::SpawnRef;
use research::topic::claims::ClaimedIndexes;
use research::topic::evidence::Digest;
use research::topic::layout::note_ref::NoteRef;
use research::topic::layout::stem::Stem;
use research::topic::question::NormalisedQuestion;
use research::topic::question::UnicodeText;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;

const LEDGER_NAME: &str = ".conduct-run.json";

#[must_use]
pub fn run_ledger_path(set_dir: &Path) -> PathBuf {
    set_dir.join(LEDGER_NAME)
}

#[derive(Debug)]
pub enum LedgerError {
    /// The file is not a ledger: a field is missing or outside its type.
    Corrupt,
    Unreadable(kernel::Error),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt => f.write_str("the run ledger is corrupt"),
            Self::Unreadable(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for LedgerError {}

/// The ledger stored for the set at `set_dir`, if any.
///
/// # Errors
///
/// [`LedgerError::Corrupt`] when the file holds anything but a well-formed
/// ledger, and [`LedgerError::Unreadable`] when it cannot be read.
pub fn read_run_ledger<F: FileReader>(
    set_dir: &Path,
    fs: &F,
    unicode: &dyn UnicodeText,
) -> Result<Option<RunLedger>, LedgerError> {
    let Some(text) = fs
        .read(&run_ledger_path(set_dir))
        .map_err(LedgerError::Unreadable)?
    else {
        return Ok(None);
    };
    let value: Value =
        serde_json::from_str(&text).map_err(|_| LedgerError::Corrupt)?;
    ledger_from(&value, unicode)
        .map(Some)
        .ok_or(LedgerError::Corrupt)
}

/// # Errors
///
/// [`StoreError`] when the store refuses or fails the write.
pub fn write_run_ledger<S: AtomicWrite>(
    set_dir: &Path,
    ledger: &RunLedger,
    store: &S,
) -> Result<(), StoreError> {
    let text = format!("{:#}\n", ledger_to(ledger));
    store.write(&run_ledger_path(set_dir), text.as_bytes())
}

/// Succeeds when the set has no ledger.
///
/// # Errors
///
/// [`StoreError`] when the store refuses or fails the removal.
pub fn delete_run_ledger<R: FileRemove>(
    set_dir: &Path,
    remover: &R,
) -> Result<(), StoreError> {
    remover.remove(&run_ledger_path(set_dir))
}

fn ledger_to(ledger: &RunLedger) -> Value {
    let memory = ledger.memory();
    let spawns = |spawns: &mut dyn Iterator<Item = &SpawnRef>| -> Value {
        spawns.map(ToString::to_string).collect()
    };
    let claims: Map<String, Value> = ledger
        .claims()
        .iter()
        .map(|(question, index)| (question.as_str().to_owned(), json!(index)))
        .collect();
    let seen_notes: Map<String, Value> = memory
        .seen
        .notes
        .iter()
        .map(|(at, digest)| (at.to_string(), json!(hex(digest))))
        .collect();
    json!({
        "run": ledger.run().to_string(),
        "pending": {
            "number": memory.pending.number(),
            "spawns": spawns(&mut memory.pending.spawns().iter()),
        },
        "claims": claims,
        "attempted": spawns(&mut memory.attempted.iter()),
        "just_acknowledged": spawns(&mut memory.just_acknowledged.iter()),
        "seen": {
            "notes": seen_notes,
            "answered": memory
                .seen
                .answered
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        },
    })
}

fn ledger_from(value: &Value, unicode: &dyn UnicodeText) -> Option<RunLedger> {
    let pending = value.get("pending")?;
    let seen = value.get("seen")?;
    Some(RunLedger::from_parts(
        RunId::parse(value.get("run")?.as_str()?)?,
        claims_from(value.get("claims")?, unicode)?,
        RunMemory {
            pending: PendingBatch::new(
                number(pending.get("number")?)?,
                spawn_list(pending.get("spawns")?)?,
            ),
            attempted: spawn_list(value.get("attempted")?)?
                .into_iter()
                .collect(),
            just_acknowledged: spawn_list(value.get("just_acknowledged")?)?
                .into_iter()
                .collect(),
            seen: Observed {
                notes: seen_notes_from(seen.get("notes")?)?,
                answered: seen
                    .get("answered")?
                    .as_array()?
                    .iter()
                    .map(|stem| Stem::parse(stem.as_str()?))
                    .collect::<Option<BTreeSet<_>>>()?,
            },
        },
    ))
}

fn number(value: &Value) -> Option<u32> {
    u32::try_from(value.as_u64()?).ok()
}

fn spawn_list(value: &Value) -> Option<Vec<SpawnRef>> {
    value
        .as_array()?
        .iter()
        .map(|spawn| SpawnRef::parse(spawn.as_str()?))
        .collect()
}

/// Folding is idempotent, so a key that folds to other text was never
/// written by this store.
fn claims_from(
    value: &Value,
    unicode: &dyn UnicodeText,
) -> Option<ClaimedIndexes> {
    let mut claims = ClaimedIndexes::default();
    for (key, index) in value.as_object()? {
        let question = NormalisedQuestion::of(key, unicode);
        if question.as_str() != key {
            return None;
        }
        claims.claim(question, number(index)?);
    }
    Some(claims)
}

fn seen_notes_from(value: &Value) -> Option<BTreeMap<NoteRef, Digest>> {
    value
        .as_object()?
        .iter()
        .map(|(key, digest)| {
            let SpawnRef::Node(at) = SpawnRef::parse(key)? else {
                return None;
            };
            Some((at, digest_from(digest.as_str()?)?))
        })
        .collect()
}

fn hex(digest: &Digest) -> String {
    digest.bytes().iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn digest_from(text: &str) -> Option<Digest> {
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0; 32];
    for (byte, pair) in bytes.iter_mut().zip(text.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(Digest::new(bytes))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::path::PathBuf;

    use corpus::scan::FileReader;
    use corpus::AtomicWrite;
    use corpus::FileRemove;
    use corpus::StoreError;
    use research::conduct::ledger::RunId;
    use research::conduct::ledger::RunLedger;
    use research::conduct::memory::PendingBatch;
    use research::conduct::memory::RunMemory;
    use research::conduct::observed::Observed;
    use research::conduct::spawn::SpawnRef;
    use research::topic::claims::ClaimedIndexes;
    use research::topic::evidence::Digest;
    use research::topic::layout::lineage::Lineage;
    use research::topic::layout::note_ref::NoteRef;
    use research::topic::layout::stem::Stem;
    use research::topic::question::NormalisedQuestion;
    use serde_json::json;

    use super::delete_run_ledger;
    use super::read_run_ledger;
    use super::write_run_ledger;
    use super::LedgerError;
    use crate::unicode_text::UnicodeTables;

    type TestError = Box<dyn std::error::Error>;

    const SET: &str = "/set";
    const LEDGER: &str = "/set/.conduct-run.json";

    #[derive(Default)]
    struct StubStore {
        files: RefCell<BTreeMap<PathBuf, String>>,
    }

    impl StubStore {
        fn holding(text: &str) -> Self {
            let store = Self::default();
            store
                .files
                .borrow_mut()
                .insert(PathBuf::from(LEDGER), text.to_owned());
            store
        }
    }

    impl FileReader for StubStore {
        fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
            Ok(self.files.borrow().get(path).cloned())
        }
    }

    impl AtomicWrite for StubStore {
        fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
            self.files.borrow_mut().insert(
                path.to_path_buf(),
                String::from_utf8_lossy(bytes).into_owned(),
            );
            Ok(())
        }
    }

    impl FileRemove for StubStore {
        fn remove(&self, path: &Path) -> Result<(), StoreError> {
            self.files.borrow_mut().remove(path);
            Ok(())
        }
    }

    fn read(store: &StubStore) -> Result<Option<RunLedger>, LedgerError> {
        read_run_ledger(Path::new(SET), store, &UnicodeTables)
    }

    fn spawn(text: &str) -> SpawnRef {
        SpawnRef::parse(text).expect("spawn ref")
    }

    fn note_ref(stem: &str, lineage: &str) -> NoteRef {
        NoteRef {
            stem: Stem::parse(stem).expect("stem"),
            lineage: Lineage::parse(lineage).expect("lineage"),
        }
    }

    fn full_ledger() -> RunLedger {
        let mut claims = ClaimedIndexes::default();
        claims.claim(NormalisedQuestion::of("A  ?", &UnicodeTables), 3);
        RunLedger::from_parts(
            RunId::parse("2026-09-27_10-00-00-7").expect("run id"),
            claims,
            RunMemory {
                pending: PendingBatch::new(
                    2,
                    vec![spawn("03-a-web:2-1"), spawn("04-b-web")],
                ),
                attempted: BTreeSet::from([spawn("03-a-web:1")]),
                just_acknowledged: BTreeSet::from([spawn("03-a-web:1")]),
                seen: Observed {
                    notes: BTreeMap::from([(
                        note_ref("03-a-web", "1"),
                        Digest::new([0xab; 32]),
                    )]),
                    answered: BTreeSet::from([
                        Stem::parse("01-c-web").expect("stem")
                    ]),
                },
            },
        )
    }

    fn stored(value: &serde_json::Value) -> StubStore {
        StubStore::holding(&value.to_string())
    }

    fn well_formed() -> serde_json::Value {
        json!({
            "run": "r1",
            "pending": {"number": 0, "spawns": []},
            "claims": {"A?": 1},
            "attempted": [],
            "just_acknowledged": [],
            "seen": {"notes": {}, "answered": []},
        })
    }

    #[test]
    fn a_missing_run_ledger_reads_as_none() -> Result<(), TestError> {
        assert_eq!(read(&StubStore::default())?, None);
        Ok(())
    }

    #[test]
    fn an_unparseable_run_ledger_is_corrupt() {
        for text in ["", "{", "[]", "{\"run\": \"r1\"}"] {
            assert!(
                matches!(
                    read(&StubStore::holding(text)),
                    Err(LedgerError::Corrupt)
                ),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_well_formed_run_ledger_reads() -> Result<(), TestError> {
        let ledger = read(&stored(&well_formed()))?.ok_or("no ledger")?;
        assert_eq!(ledger.run(), &RunId::parse("r1").ok_or("run id")?);
        Ok(())
    }

    #[test]
    fn a_run_ledger_round_trips_through_its_file() -> Result<(), TestError> {
        let store = StubStore::default();
        let ledger = full_ledger();

        write_run_ledger(Path::new(SET), &ledger, &store)?;

        assert_eq!(read(&store)?, Some(ledger));
        Ok(())
    }

    #[test]
    fn a_written_ledger_names_its_fields_after_the_memory_it_keeps(
    ) -> Result<(), TestError> {
        let store = StubStore::default();
        write_run_ledger(Path::new(SET), &full_ledger(), &store)?;
        let written: serde_json::Value = serde_json::from_str(
            &store.read(Path::new(LEDGER))?.ok_or("no ledger")?,
        )?;
        let keys = |value: &serde_json::Value| -> Vec<String> {
            value
                .as_object()
                .map(|fields| fields.keys().cloned().collect())
                .unwrap_or_default()
        };
        assert_eq!(
            keys(&written),
            [
                "attempted",
                "claims",
                "just_acknowledged",
                "pending",
                "run",
                "seen"
            ]
        );
        assert_eq!(keys(&written["seen"]), ["answered", "notes"]);
        assert_eq!(written["claims"], json!({"A ?": 3}));
        Ok(())
    }

    #[test]
    fn a_ledger_field_outside_its_value_type_is_corrupt() {
        let breaches = [
            ("/run", json!("a b")),
            ("/pending", json!({"number": -1, "spawns": []})),
            ("/pending", json!({"number": 0, "spawns": ["03-a:2-0"]})),
            ("/claims", json!({"A?": "one"})),
            ("/attempted", json!(["03-A"])),
            ("/just_acknowledged", json!([":1"])),
            ("/seen", json!([])),
            ("/seen/notes", json!({"03-a-web": "00"})),
            ("/seen/notes", json!({"03-a-web:1": "zz"})),
            ("/seen/notes", json!({"03-a-web:1": "ab".repeat(31)})),
            ("/seen/answered", json!(["03-a-web:1"])),
        ];
        for (field, value) in breaches {
            let mut ledger = well_formed();
            *ledger.pointer_mut(field).expect("a ledger field") = value.clone();
            assert!(
                matches!(read(&stored(&ledger)), Err(LedgerError::Corrupt)),
                "{field}: {value}"
            );
        }
    }

    #[test]
    fn a_claims_key_that_is_not_already_normalised_is_corrupt() {
        let mut ledger = well_formed();
        ledger["claims"] = json!({"A  ?": 1});
        assert!(matches!(read(&stored(&ledger)), Err(LedgerError::Corrupt)));
    }

    #[test]
    fn deleting_an_absent_run_ledger_succeeds() -> Result<(), TestError> {
        delete_run_ledger(Path::new(SET), &StubStore::default())?;
        Ok(())
    }

    #[test]
    fn deleting_a_run_ledger_removes_its_file() -> Result<(), TestError> {
        let store = StubStore::default();
        write_run_ledger(Path::new(SET), &full_ledger(), &store)?;

        delete_run_ledger(Path::new(SET), &store)?;

        assert_eq!(store.read(Path::new(LEDGER))?, None);
        Ok(())
    }
}

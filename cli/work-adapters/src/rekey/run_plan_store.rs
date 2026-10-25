//! The re-key run plan as one JSON file under
//! `.accelerator/state/rekey/`, ignored by version control.

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use corpus::store::FileRemove;
use corpus::WorkItemIdScheme;
use serde_json::json;
use serde_json::Value;
use time::OffsetDateTime;
use work::draft_id::DraftId;
use work::identity::ItemIdentity;
use work::rekey::entry::RekeyEntry;
use work::rekey::entry::RekeyTarget;
use work::rekey::fingerprint::IdentityFingerprint;
use work::rekey::outcome::NotRekeyed;
use work::rekey::run_plan::readable_schema;
use work::rekey::run_plan::InProgressPlan;
use work::rekey::run_plan::ProposedPlan;
use work::rekey::run_plan::RunPlan;
use work::rekey::run_plan::Unreadability;
use work::rekey::run_plan::UnreadablePlan;
use work::rekey::run_plan::RUN_PLAN_SCHEMA;
use work::rekey::write_set::Base;
use work::rekey::write_set::Digest;
use work::rekey::write_set::Recorded;
use work::rekey::write_set::StartState;
use work::rekey::write_set::WriteSet;
use work::tracker_key::TrackerKey;

use crate::ignored_dir::prepare_ignored_dir;
use work::identity_run::RunPlanGate;
use work::identity_run::RunPlanStatus;

const PLAN_FILE: &str = "run-plan.json";

pub trait RunPlanStore {
    /// # Errors
    ///
    /// [`UnreadablePlan`] when the plan cannot be read, does not parse, or
    /// has a schema newer than this reader's.
    fn load(&self) -> Result<Option<RunPlan>, UnreadablePlan>;

    /// # Errors
    ///
    /// [`NotRekeyed::StateUnwritable`] when the plan cannot be written.
    fn save(&self, plan: &RunPlan) -> Result<(), NotRekeyed>;

    /// Removing an absent plan succeeds.
    ///
    /// # Errors
    ///
    /// [`NotRekeyed::StateUnwritable`] when the plan cannot be removed.
    fn remove(&self) -> Result<(), NotRekeyed>;

    /// Moves the plan aside, under a timestamped name beside it, and
    /// returns where it went. A run stopped part way leaves both copies.
    ///
    /// # Errors
    ///
    /// [`NotRekeyed::StateUnwritable`] when the plan cannot be moved.
    fn set_aside(&self) -> Result<PathBuf, NotRekeyed>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RekeyStatus {
    Absent,
    Proposed,
    InProgress,
    Unreadable(UnreadablePlan),
}

/// Paths persist repo-relative with `/` separators, so a plan survives the
/// repository moving; they are re-rooted when loaded.
pub struct FileRunPlanStore<'a> {
    dir: PathBuf,
    repo_root: PathBuf,
    writer: &'a dyn AtomicWrite,
    remover: &'a dyn FileRemove,
}

impl<'a> FileRunPlanStore<'a> {
    #[must_use]
    pub fn new(
        state_dir: &Path,
        repo_root: &Path,
        writer: &'a dyn AtomicWrite,
        remover: &'a dyn FileRemove,
    ) -> Self {
        Self {
            dir: state_dir.join("rekey"),
            repo_root: repo_root.to_path_buf(),
            writer,
            remover,
        }
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.dir.join(PLAN_FILE)
    }

    #[must_use]
    pub fn status(&self) -> RekeyStatus {
        match self.load() {
            Ok(None) => RekeyStatus::Absent,
            Ok(Some(RunPlan::Proposed(_))) => RekeyStatus::Proposed,
            Ok(Some(RunPlan::InProgress(_))) => RekeyStatus::InProgress,
            Err(unreadable) => RekeyStatus::Unreadable(unreadable),
        }
    }

    fn relative(&self, path: &Path) -> String {
        let relative = path.strip_prefix(&self.repo_root).unwrap_or(path);
        relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => Some(part.to_string_lossy()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn rooted(&self, relative: &str) -> PathBuf {
        self.repo_root.join(relative)
    }

    fn encode(&self, plan: &RunPlan) -> String {
        let (state, fingerprint, entries, write_set) = match plan {
            RunPlan::Proposed(plan) => {
                ("proposed", plan.fingerprint(), plan.entries(), None)
            }
            RunPlan::InProgress(plan) => (
                "in_progress",
                plan.fingerprint(),
                plan.entries(),
                Some(plan.write_set()),
            ),
        };
        let mut encoded = json!({
            "schema": RUN_PLAN_SCHEMA,
            "state": state,
            "fingerprint": self.encode_fingerprint(fingerprint),
            "entries": entries.iter().map(encode_entry).collect::<Vec<_>>(),
        });
        if let Some(write_set) = write_set {
            encoded["write_set"] = encode_write_set(write_set);
        }
        format!("{encoded:#}\n")
    }

    fn encode_fingerprint(&self, fingerprint: &IdentityFingerprint) -> Value {
        let scheme = fingerprint.scheme();
        json!({
            "id_pattern": scheme.id_pattern,
            "key": scheme.key,
            "items": fingerprint
                .items()
                .map(|item| json!({
                    "path": self.relative(&item.path),
                    "id": item.id,
                    "aliases": item.aliases,
                    "external_id": item.external_id,
                }))
                .collect::<Vec<_>>(),
        })
    }

    fn decode(&self, raw: &str) -> Result<RunPlan, Unreadability> {
        let value: Value = serde_json::from_str(raw)
            .map_err(|error| Unreadability::Malformed(error.to_string()))?;
        let schema = value
            .get("schema")
            .and_then(Value::as_u64)
            .and_then(|schema| u32::try_from(schema).ok())
            .ok_or_else(|| malformed("no readable 'schema'"))?;
        readable_schema(schema)?;
        let fingerprint =
            self.decode_fingerprint(field(&value, "fingerprint")?)?;
        let entries = array(&value, "entries")?
            .iter()
            .map(decode_entry)
            .collect::<Result<Vec<_>, _>>()?;
        match text(&value, "state")? {
            "proposed" => {
                Ok(RunPlan::Proposed(ProposedPlan::new(fingerprint, entries)))
            }
            "in_progress" => Ok(RunPlan::InProgress(InProgressPlan::new(
                fingerprint,
                entries,
                decode_write_set(&value)?,
            ))),
            other => Err(malformed(&format!("unknown state '{other}'"))),
        }
    }

    fn decode_fingerprint(
        &self,
        value: &Value,
    ) -> Result<IdentityFingerprint, Unreadability> {
        let scheme = WorkItemIdScheme {
            id_pattern: text(value, "id_pattern")?.to_owned(),
            key: optional_text(value, "key")?,
        };
        let items = array(value, "items")?
            .iter()
            .map(|item| {
                Ok(ItemIdentity {
                    path: self.rooted(text(item, "path")?),
                    id: text(item, "id")?.to_owned(),
                    aliases: array(item, "aliases")?
                        .iter()
                        .map(|alias| {
                            alias
                                .as_str()
                                .map(str::to_owned)
                                .ok_or_else(|| malformed("a non-text alias"))
                        })
                        .collect::<Result<_, _>>()?,
                    external_id: optional_text(item, "external_id")?,
                })
            })
            .collect::<Result<Vec<_>, Unreadability>>()?;
        Ok(IdentityFingerprint::of(&scheme, &items))
    }

    fn unreadable(&self, cause: Unreadability) -> UnreadablePlan {
        UnreadablePlan {
            path: self.path(),
            cause,
        }
    }

    fn unwritable(&self, error: &impl std::fmt::Display) -> NotRekeyed {
        NotRekeyed::StateUnwritable(format!(
            "{}: {error}",
            self.path().display()
        ))
    }
}

fn malformed(detail: &str) -> Unreadability {
    Unreadability::Malformed(detail.to_owned())
}

fn field<'v>(value: &'v Value, name: &str) -> Result<&'v Value, Unreadability> {
    value
        .get(name)
        .ok_or_else(|| malformed(&format!("missing '{name}'")))
}

fn text<'v>(value: &'v Value, name: &str) -> Result<&'v str, Unreadability> {
    field(value, name)?
        .as_str()
        .ok_or_else(|| malformed(&format!("'{name}' is not text")))
}

fn optional_text(
    value: &Value,
    name: &str,
) -> Result<Option<String>, Unreadability> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(malformed(&format!("'{name}' is not text"))),
    }
}

fn array<'v>(
    value: &'v Value,
    name: &str,
) -> Result<&'v Vec<Value>, Unreadability> {
    field(value, name)?
        .as_array()
        .ok_or_else(|| malformed(&format!("'{name}' is not a list")))
}

fn encode_entry(entry: &RekeyEntry) -> Value {
    let kind = match entry.target {
        RekeyTarget::Renumbered(_) => "renumbered",
        RekeyTarget::TrackerKey(_) => "tracker_key",
        RekeyTarget::Draft(_) => "draft",
    };
    json!({
        "old": entry.old,
        "target": { "kind": kind, "id": entry.target.id() },
    })
}

fn decode_entry(value: &Value) -> Result<RekeyEntry, Unreadability> {
    let target = field(value, "target")?;
    let id = text(target, "id")?;
    let unparseable =
        |kind: &str| malformed(&format!("'{id}' is not a {kind}"));
    let target = match text(target, "kind")? {
        "renumbered" => RekeyTarget::Renumbered(id.to_owned()),
        "tracker_key" => RekeyTarget::TrackerKey(
            TrackerKey::parse(id).ok_or_else(|| unparseable("tracker key"))?,
        ),
        "draft" => RekeyTarget::Draft(
            DraftId::parse(id).ok_or_else(|| unparseable("draft ID"))?,
        ),
        other => return Err(malformed(&format!("unknown target '{other}'"))),
    };
    Ok(RekeyEntry {
        old: text(value, "old")?.to_owned(),
        target,
    })
}

fn encode_write_set(write_set: &WriteSet) -> Value {
    Value::Array(
        write_set
            .iter()
            .map(|(path, recorded)| {
                json!({
                    "path": path,
                    "digest": digest_of(&recorded.state),
                    "base": recorded.base.as_ref().map(Base::commits),
                    "intended": write_set
                        .intended(path)
                        .iter()
                        .map(digest_of)
                        .collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

fn digest_of(state: &StartState) -> Option<&str> {
    match state {
        StartState::Absent => None,
        StartState::Content(digest) => Some(digest.as_str()),
    }
}

fn state_of(digest: Option<&str>) -> StartState {
    digest.map_or(StartState::Absent, |digest| {
        StartState::Content(Digest::new(digest))
    })
}

fn decode_intended(
    path: &str,
    entry: &Value,
) -> Result<Vec<(String, StartState)>, Unreadability> {
    match entry.get("intended") {
        None => Ok(Vec::new()),
        Some(Value::Array(digests)) => digests
            .iter()
            .map(|digest| match digest {
                Value::Null => Ok((path.to_owned(), StartState::Absent)),
                Value::String(digest) => {
                    Ok((path.to_owned(), state_of(Some(digest))))
                }
                _ => Err(malformed("a non-text intended digest")),
            })
            .collect(),
        Some(_) => Err(malformed("'intended' is not a list")),
    }
}

fn decode_write_set(value: &Value) -> Result<WriteSet, Unreadability> {
    let mut intended = Vec::new();
    let recorded = array(value, "write_set")?
        .iter()
        .map(|entry| {
            let state = state_of(optional_text(entry, "digest")?.as_deref());
            let base = match entry.get("base") {
                None | Some(Value::Null) => None,
                Some(Value::Array(commits)) => Some(Base::new(
                    commits
                        .iter()
                        .map(|commit| {
                            commit.as_str().map(str::to_owned).ok_or_else(
                                || malformed("a non-text base commit"),
                            )
                        })
                        .collect::<Result<_, _>>()?,
                )),
                Some(_) => return Err(malformed("'base' is not a list")),
            };
            let path = text(entry, "path")?;
            intended.extend(decode_intended(path, entry)?);
            Ok((path.to_owned(), Recorded { state, base }))
        })
        .collect::<Result<Vec<_>, Unreadability>>()?;
    Ok(WriteSet::default()
        .extended_with(recorded)
        .intending(intended))
}

fn discarded_name() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "run-plan.discarded-{:04}{:02}{:02}T{:02}{:02}{:02}.{:09}Z.json",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.nanosecond()
    )
}

impl RunPlanStore for FileRunPlanStore<'_> {
    fn load(&self) -> Result<Option<RunPlan>, UnreadablePlan> {
        let raw = match std::fs::read_to_string(self.path()) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => {
                return Err(self.unreadable(malformed(&error.to_string())));
            }
        };
        self.decode(&raw)
            .map(Some)
            .map_err(|cause| self.unreadable(cause))
    }

    fn save(&self, plan: &RunPlan) -> Result<(), NotRekeyed> {
        prepare_ignored_dir(&self.dir, self.writer)
            .map_err(|error| self.unwritable(&error))?;
        self.writer
            .write(&self.path(), self.encode(plan).as_bytes())
            .map_err(|error| self.unwritable(&error))
    }

    fn remove(&self) -> Result<(), NotRekeyed> {
        self.remover
            .remove(&self.path())
            .map_err(|error| self.unwritable(&error))
    }

    fn set_aside(&self) -> Result<PathBuf, NotRekeyed> {
        let aside = self.dir.join(discarded_name());
        let bytes = std::fs::read(self.path())
            .map_err(|error| self.unwritable(&error))?;
        self.writer
            .write(&aside, &bytes)
            .map_err(|error| self.unwritable(&error))?;
        self.remove()?;
        Ok(aside)
    }
}

impl RunPlanStatus for FileRunPlanStore<'_> {
    fn gate(&self) -> RunPlanGate {
        match self.status() {
            RekeyStatus::Absent | RekeyStatus::Proposed => RunPlanGate::Clear,
            RekeyStatus::InProgress => RunPlanGate::InProgress,
            RekeyStatus::Unreadable(plan) => RunPlanGate::Unreadable(plan),
        }
    }
}

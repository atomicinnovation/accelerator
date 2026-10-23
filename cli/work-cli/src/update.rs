//! Adapter/binary wiring for `work update`: validates every `--set`/
//! `--append`/`--remove` key before touching anything, then applies the
//! validated mutations as one `document::Yaml` tree edit and performs one
//! atomic write, under a per-file mkdir-lock.

use std::path::Path;

use ::config::ConfigAccess;
use corpus::lock::ExclusiveLock as _;
use corpus::lock::LockName;
use corpus::AtomicWrite;
use corpus_adapters::CommonMarkBody;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::LockdirLock;
use corpus_adapters::RealFs;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use tracker::TrackerError;
use work::tags::mutate_tags;
use work::tags::parse_current_tags;
use work::tags::TagAction;
use work::tags::TagError;
use work::tags::TagMutation;
use work::update::mutate_list;
use work::update::validate_set_key;
use work::update::ListAction;
use work::update::ListMutation;
use work::update::UpdateError;
use work::update::LIST_FIELDS;
use work_adapters::sync::baseline;
use work_adapters::sync::baseline_digester::BaselineDigester;
use work_adapters::sync::baseline_store::BaselineStore;

use crate::cli::UpdateArgs;
use crate::config::effective_nonempty;
use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;

pub enum RunOutcome {
    Updated,
    Failed(String),
    PushNotAvailable(String),
    PushUnrecognised(String),
    PushUnconfigured(String),
    PushRetryable(String),
    PushTerminal(String),
    PushRejected(String),
    /// The identity run an `update --push` needs could not begin; nothing
    /// was read or written.
    Refused {
        message: String,
        code: u8,
    },
}

fn set_key_error_message(error: &UpdateError) -> String {
    match error {
        UpdateError::IdImmutable => {
            "Error: own-identity (id) cannot be changed — the filename \
             prefix is the authoritative work item ID. To renumber a work \
             item, rename the file (e.g. jj mv) and update the id field \
             to match. The id field is always a quoted string."
                .to_owned()
        }
        UpdateError::ScalarSetOnListField { key } => {
            format!(
                "'{key}' is not a valid --set target — it is a list field; \
                 use --append/--remove (or --add-tag/--remove-tag for \
                 tags) instead"
            )
        }
    }
}

fn tag_error_message(error: TagError) -> String {
    match error {
        TagError::BlockStyleTags => {
            "Error: tags field is in block format — convert to tags: \
             [...] first. Example: tags: [api, search]"
                .to_owned()
        }
    }
}

fn validate_list_key(key: &str) -> Result<(), String> {
    if key == "tags" {
        return Err("'tags' is not a valid --append/--remove key; use \
             --add-tag/--remove-tag instead"
            .to_owned());
    }
    if !LIST_FIELDS.contains(&key) {
        return Err(format!(
            "'{key}' is not a valid --append/--remove key; supported: \
             blocks, blocked_by, derived_from, relates_to"
        ));
    }
    Ok(())
}

fn string_items(mapping: &Mapping, key: &str) -> Vec<String> {
    match mapping.get(key) {
        Some(Yaml::Sequence(items)) => items
            .iter()
            .filter_map(|item| match item {
                Yaml::Scalar(Scalar::String(text)) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn sequence_of(items: Vec<String>) -> Yaml {
    Yaml::Sequence(
        items
            .into_iter()
            .map(|item| Yaml::Scalar(Scalar::String(item)))
            .collect(),
    )
}

fn apply_tags(
    mapping: &mut Mapping,
    action: TagAction,
    tag: &str,
    current_raw: &mut String,
) -> Result<(), String> {
    let mutation =
        mutate_tags(current_raw, action, tag).map_err(tag_error_message)?;
    if let TagMutation::Changed(canonical) = mutation {
        mapping.set(
            "tags".to_owned(),
            sequence_of(parse_current_tags(&canonical)),
        );
        *current_raw = format!("tags: {canonical}\n");
    }
    Ok(())
}

enum TryRunError {
    Generic(String),
    NotAvailable(String),
    Unrecognised(String),
    Unconfigured(String),
    Retryable(String),
    Terminal(String),
    Rejected(String),
    Refused { message: String, code: u8 },
}

impl From<String> for TryRunError {
    fn from(message: String) -> Self {
        Self::Generic(message)
    }
}

fn scalar_string(mapping: &Mapping, key: &str) -> Option<String> {
    match mapping.get(key) {
        Some(Yaml::Scalar(Scalar::String(text))) => Some(text.clone()),
        _ => None,
    }
}

struct PushedFields {
    external_id: Option<String>,
    id: Option<String>,
    title: Option<String>,
}

fn push_update(
    start: &Path,
    config: &dyn ConfigAccess,
    registry: &dyn TrackerRegistry,
    fields: PushedFields,
    rendered: &str,
    path: &Path,
) -> Result<(), TryRunError> {
    let PushedFields {
        external_id,
        id,
        title,
    } = fields;
    let external_id = external_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            TryRunError::Generic(format!(
                "E_PUSH_UNSYNCED: {} has no external_id — push is only \
                 valid for an item already synced to a remote tracker; use \
                 `create --push` for an unsynced one.",
                path.display()
            ))
        })?;
    let id = id.ok_or_else(|| {
        TryRunError::Generic(format!(
            "{} has no id field in its frontmatter",
            path.display()
        ))
    })?;
    let title = title.ok_or_else(|| {
        TryRunError::Generic(format!(
            "{} has no title field in its frontmatter",
            path.display()
        ))
    })?;
    let body = document::split(rendered)
        .map_err(|error| TryRunError::Generic(error.to_string()))?
        .body;

    let integration = effective_nonempty(config, "work.integration")
        .map_err(|error| TryRunError::Generic(error.to_string()))?;
    let tracker =
        registry
            .resolve(&integration)
            .map_err(|error| match error {
                SelectionError::NotAvailable { .. } => {
                    TryRunError::NotAvailable(error.message())
                }
                SelectionError::Unconfigured { .. } => {
                    TryRunError::Unconfigured(error.message())
                }
                SelectionError::Unset | SelectionError::Unrecognised { .. } => {
                    TryRunError::Unrecognised(error.message())
                }
            })?;

    let root = config_adapters::FileConfigStore::discover_root(start);
    let integrations_root = crate::sync::integrations_dir(config, &root)
        .map_err(|error| TryRunError::Generic(error.to_string()))?;
    let baseline_path = baseline::path(&integrations_root, &integration);
    let file_reader = RealFs;
    let baseline_writer = FileCorpusStore::new(
        baseline_path.parent().unwrap_or(&integrations_root),
    );
    let baseline_store =
        BaselineStore::new(baseline_path, &file_reader, &baseline_writer);
    let external_id = tracker::ExternalId::new(external_id);

    tracker
        .update(&external_id, Some(&title), &body)
        .map_err(|error| push_failure(error, path, &baseline_store, &id))?;

    let store =
        FileCorpusStore::new(path.parent().unwrap_or_else(|| Path::new(".")));
    AtomicWrite::write(&store, path, rendered.as_bytes())
        .map_err(|error| TryRunError::Generic(error.to_string()))?;

    let remote = tracker.show(&external_id).ok();
    let fingerprints = BaselineDigester::new(tracker.as_ref(), &CommonMarkBody)
        .digests(rendered, remote.as_ref().map(|issue| issue.body.as_str()))
        .map_err(|error| TryRunError::Generic(error.to_string()))?;
    baseline_store
        .set(
            &id,
            baseline::Entry::recorded(
                baseline::SyncStamps {
                    remote_updated_at: remote
                        .map_or(tracker::RemoteTimestamp::NotRead, |issue| {
                            issue.updated
                        }),
                    local_synced_at: 0,
                },
                fingerprints,
            ),
        )
        .map_err(|error| TryRunError::Generic(error.to_string()))?;

    Ok(())
}

/// A terminal failure may have applied remotely, so it clears the baseline
/// entry and the next sync reconciles the item as a conflict.
fn push_failure(
    error: TrackerError,
    path: &Path,
    baseline_store: &BaselineStore<'_>,
    id: &str,
) -> TryRunError {
    match error {
        TrackerError::Retryable { detail } => TryRunError::Retryable(format!(
            "E_PUSH_RETRYABLE: pushing {} failed and can be retried: {detail}",
            path.display()
        )),
        TrackerError::Unconfigured { detail } => {
            TryRunError::Unconfigured(detail)
        }
        TrackerError::Rejected { detail } => TryRunError::Rejected(format!(
            "E_PUSH_REJECTED: pushing {} was refused before anything was \
             sent, and the request must change: {detail}",
            path.display()
        )),
        TrackerError::Terminal { detail } => {
            baseline_store.remove(id).ok();
            TryRunError::Terminal(format!(
                "E_PUSH_TERMINAL: pushing {} failed and may have partially \
                 applied: {detail}. The baseline entry has been cleared; the \
                 next sync will classify this item as a conflict.",
                path.display()
            ))
        }
    }
}

fn try_run(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &UpdateArgs,
    registry: &dyn TrackerRegistry,
) -> Result<(), TryRunError> {
    for (key, _) in &args.sets {
        validate_set_key(key).map_err(|error| set_key_error_message(&error))?;
    }
    for (key, _) in args.appends.iter().chain(&args.removes) {
        validate_list_key(key)?;
    }

    let _identity_run = if args.push {
        let root = config_adapters::FileConfigStore::discover_root(start);
        let guard =
            crate::identity_run::begin(&root, "update").map_err(|refusal| {
                let (message, code) =
                    crate::identity_run::refused(refusal, &root);
                TryRunError::Refused { message, code }
            })?;
        Some(guard)
    } else {
        None
    };
    let work_dir = args.path.parent().unwrap_or_else(|| Path::new("."));
    let _guard = LockdirLock::new(work_dir)
        .acquire(&LockName::ForFile(args.path.clone()))
        .map_err(|error| {
            format!("could not acquire the update lock: {error}")
        })?;

    let content = std::fs::read_to_string(&args.path).map_err(|error| {
        format!("could not read {}: {error}", args.path.display())
    })?;
    let split = document::split(&content).map_err(|error| error.to_string())?;
    let mut yaml =
        document::parse(&content).map_err(|error| error.to_string())?;
    let Yaml::Mapping(mapping) = &mut yaml else {
        return Err(TryRunError::Generic(
            "frontmatter is not a mapping".to_owned(),
        ));
    };

    for (key, value) in &args.sets {
        mapping.set(key.clone(), Yaml::Scalar(Scalar::String(value.clone())));
    }

    let mut current_raw_tags = split.frontmatter;
    for tag in &args.add_tags {
        apply_tags(mapping, TagAction::Add, tag, &mut current_raw_tags)?;
    }
    for tag in &args.remove_tags {
        apply_tags(mapping, TagAction::Remove, tag, &mut current_raw_tags)?;
    }

    for (key, value) in &args.appends {
        let current = string_items(mapping, key);
        if let ListMutation::Changed(next) =
            mutate_list(&current, ListAction::Append, value)
        {
            mapping.set(key.clone(), sequence_of(next));
        }
    }
    for (key, value) in &args.removes {
        let current = string_items(mapping, key);
        if let ListMutation::Changed(next) =
            mutate_list(&current, ListAction::Remove, value)
        {
            mapping.set(key.clone(), sequence_of(next));
        }
    }

    let pushed_fields = args.push.then(|| PushedFields {
        external_id: scalar_string(mapping, "external_id"),
        id: scalar_string(mapping, "id"),
        title: scalar_string(mapping, "title"),
    });

    let rendered = document::render(Some(&content), &yaml)
        .map_err(|error| error.to_string())?;

    if let Some(fields) = pushed_fields {
        return push_update(
            start, config, registry, fields, &rendered, &args.path,
        );
    }

    let store = FileCorpusStore::new(
        args.path.parent().unwrap_or_else(|| Path::new(".")),
    );
    AtomicWrite::write(&store, &args.path, rendered.as_bytes())
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &UpdateArgs,
    registry: &dyn TrackerRegistry,
) -> RunOutcome {
    match try_run(start, config, args, registry) {
        Ok(()) => RunOutcome::Updated,
        Err(TryRunError::Generic(message)) => RunOutcome::Failed(message),
        Err(TryRunError::NotAvailable(message)) => {
            RunOutcome::PushNotAvailable(message)
        }
        Err(TryRunError::Unrecognised(message)) => {
            RunOutcome::PushUnrecognised(message)
        }
        Err(TryRunError::Unconfigured(message)) => {
            RunOutcome::PushUnconfigured(message)
        }
        Err(TryRunError::Retryable(message)) => {
            RunOutcome::PushRetryable(message)
        }
        Err(TryRunError::Terminal(message)) => {
            RunOutcome::PushTerminal(message)
        }
        Err(TryRunError::Rejected(message)) => {
            RunOutcome::PushRejected(message)
        }
        Err(TryRunError::Refused { message, code }) => {
            RunOutcome::Refused { message, code }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use ::config::ConfigError;
    use ::config::Key;
    use ::config::Level;
    use ::config::Resolved;
    use ::config::Scalar as ConfigScalar;
    use ::config::Value as ConfigValue;
    use tracker::PlainDialect;
    use tracker::RemoteTracker;
    use tracker_test_support::RecordingTracker;
    use work_adapters::sync::digest;

    use super::*;

    struct FakeConfig(HashMap<String, String>);

    impl ConfigAccess for FakeConfig {
        fn get(
            &self,
            key: &Key,
            _level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            Ok(self
                .0
                .get(&key.to_string())
                .map_or(Resolved::Absent, |value| {
                    Resolved::Found(ConfigValue::Scalar(ConfigScalar::String(
                        value.clone(),
                    )))
                }))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            unreachable!("update never writes config")
        }
    }

    struct FixedRegistry(RefCell<Option<Box<dyn RemoteTracker>>>);

    impl TrackerRegistry for FixedRegistry {
        fn resolve(
            &self,
            _name: &str,
        ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
            Ok(self.0.borrow_mut().take().expect("resolved more than once"))
        }
    }

    fn guarded_repo() -> (tempfile::TempDir, std::path::PathBuf, &'static str) {
        const ITEM: &str = "---\nid: \"0001\"\ntitle: \"Title\"\n\
                            external_id: \"ENG-1\"\n---\n\nBody\n";
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
        let item = root.path().join("0001.md");
        std::fs::write(&item, ITEM).expect("seed the item");
        crate::test_support::rekey_in_progress(root.path());
        (root, item, ITEM)
    }

    fn update_args(path: std::path::PathBuf, push: bool) -> UpdateArgs {
        UpdateArgs {
            path,
            sets: vec![("status".to_owned(), "ready".to_owned())],
            add_tags: Vec::new(),
            remove_tags: Vec::new(),
            appends: Vec::new(),
            removes: Vec::new(),
            push,
        }
    }

    fn linear() -> FakeConfig {
        FakeConfig(HashMap::from([(
            "work.integration".to_owned(),
            "linear".to_owned(),
        )]))
    }

    fn untouched_registry() -> FixedRegistry {
        FixedRegistry(RefCell::new(None))
    }

    #[test]
    fn update_push_refuses_while_a_rekey_is_in_progress() {
        let (root, item, original) = guarded_repo();

        let outcome = run(
            root.path(),
            &linear(),
            &update_args(item.clone(), true),
            &untouched_registry(),
        );

        let RunOutcome::Refused { message, code } = outcome else {
            panic!("an update --push ran during a re-key");
        };
        assert_eq!(code, crate::exit_codes::UNRESOLVED);
        assert!(
            message.starts_with("Error: E_REKEY_IN_PROGRESS"),
            "{message}"
        );
        assert_eq!(std::fs::read_to_string(&item).expect("item"), original);
        assert!(!root.path().join("0001.md.lockdir").exists());
    }

    #[test]
    fn update_push_takes_the_identity_run_lock_before_the_file_lock() {
        use corpus::lock::ExclusiveLock as _;

        let (root, item, _) = guarded_repo();
        let _file_lock = LockdirLock::new(root.path())
            .acquire(&LockName::ForFile(item.clone()))
            .expect("the file lock is free");
        let (done, finished) = std::sync::mpsc::channel();
        let start = root.path().to_path_buf();

        std::thread::spawn(move || {
            let outcome = run(
                &start,
                &linear(),
                &update_args(item, true),
                &untouched_registry(),
            );
            let _ = done.send(matches!(outcome, RunOutcome::Refused { .. }));
        });

        assert_eq!(
            finished.recv_timeout(std::time::Duration::from_secs(10)),
            Ok(true),
            "the update waited on the file lock before the identity run"
        );
    }

    #[test]
    fn update_without_push_is_not_guarded() {
        let (root, item, _) = guarded_repo();

        let outcome = run(
            root.path(),
            &linear(),
            &update_args(item.clone(), false),
            &untouched_registry(),
        );

        assert!(matches!(outcome, RunOutcome::Updated));
        assert!(std::fs::read_to_string(&item)
            .expect("item")
            .contains("status: \"ready\""));
    }

    #[test]
    fn an_unconfigured_push_keeps_the_baseline_and_exits_74() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
        let item = root.path().join("0001.md");
        std::fs::write(
            &item,
            "---\nid: \"0001\"\ntitle: \"Title\"\nexternal_id: \"ENG-1\"\n\
             ---\n\nBody\n",
        )
        .expect("seed the item");
        let baseline = root.path().join("integrations/linear/last-sync.json");
        std::fs::create_dir_all(baseline.parent().expect("parent"))
            .expect("baseline dir");
        let seeded = "{\"timestamp\":0,\"items\":{\"0001\":{\
            \"remote_updated_at\":\"2026-06-01T00:00:00Z\",\
            \"remote_hash\":\"r\",\"local_hash\":\"l\"}}}\n";
        std::fs::write(&baseline, seeded).expect("seed the baseline");
        let config = FakeConfig(
            [
                ("work.integration", "linear"),
                ("paths.integrations", "integrations"),
            ]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect(),
        );
        let registry = FixedRegistry(RefCell::new(Some(Box::new(
            RecordingTracker::holding(Vec::new()).failing_update(
                tracker::ExternalId::new("ENG-1".to_owned()),
                TrackerError::Unconfigured {
                    detail: "no team in scope carries label \"typo\""
                        .to_owned(),
                },
            ),
        ))));
        let args = UpdateArgs {
            path: item,
            sets: Vec::new(),
            add_tags: Vec::new(),
            remove_tags: Vec::new(),
            appends: Vec::new(),
            removes: Vec::new(),
            push: true,
        };

        let outcome = run(root.path(), &config, &args, &registry);

        let RunOutcome::PushUnconfigured(message) = outcome else {
            panic!("an unconfigured push exits 74");
        };
        assert_eq!(message, "no team in scope carries label \"typo\"");
        assert_eq!(
            std::fs::read_to_string(&baseline).expect("baseline"),
            seeded,
            "an unconfigured push sent nothing, so the baseline stands"
        );
    }

    #[test]
    fn a_pushed_update_records_a_recipe_one_entry() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
        let item = root.path().join("0001.md");
        std::fs::write(
            &item,
            "---\nid: \"0001\"\ntitle: \"Title\"\nexternal_id: \"ENG-1\"\n\
             ---\n\nBody\n",
        )
        .expect("seed the item");
        std::fs::create_dir_all(root.path().join("integrations/linear"))
            .expect("baseline dir");
        let config = FakeConfig(
            [
                ("work.integration", "linear"),
                ("paths.integrations", "integrations"),
            ]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect(),
        );
        let external_id = tracker::ExternalId::new("ENG-1".to_owned());
        let registry = FixedRegistry(RefCell::new(Some(Box::new(
            RecordingTracker::holding(vec![(
                external_id.clone(),
                tracker::RemoteIssue {
                    key: external_id,
                    updated: tracker::RemoteTimestamp::NotReported,
                    body: "Title\nOld body\n".to_owned(),
                },
            )]),
        ))));
        let args = UpdateArgs {
            path: item.clone(),
            sets: Vec::new(),
            add_tags: Vec::new(),
            remove_tags: Vec::new(),
            appends: Vec::new(),
            removes: Vec::new(),
            push: true,
        };

        let outcome = run(root.path(), &config, &args, &registry);

        assert!(matches!(outcome, RunOutcome::Updated), "the push succeeds");
        let written = std::fs::read_to_string(&item).expect("item");
        let (_, pushed_body) =
            digest::split_frontmatter_and_body(&written).expect("body");
        let expected = BaselineDigester::new(&PlainDialect, &CommonMarkBody)
            .digests(&written, Some(&format!("Title\n{pushed_body}")))
            .expect("digests");
        let (baseline, _) = baseline::Baseline::read(
            std::fs::read_to_string(
                root.path().join("integrations/linear/last-sync.json"),
            )
            .ok()
            .as_deref(),
        );
        let Some(baseline::Entry::Split { legacy, split, .. }) =
            baseline.get("0001")
        else {
            panic!("a recipe-1 entry is recorded");
        };
        assert_eq!(split, &expected.split);
        assert_eq!(legacy, &expected.legacy);
    }

    #[test]
    fn an_update_whose_request_is_invalid_exits_75_and_keeps_the_baseline() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
        let item = root.path().join("0001.md");
        std::fs::write(
            &item,
            "---\nid: \"0001\"\ntitle: \"Title\"\nexternal_id: \"ENG-1\"\n\
             ---\n\nBody\n",
        )
        .expect("seed the item");
        let baseline = root.path().join("integrations/jira/last-sync.json");
        std::fs::create_dir_all(baseline.parent().expect("parent"))
            .expect("baseline dir");
        let seeded = "{\"timestamp\":0,\"items\":{\"0001\":{\
            \"remote_updated_at\":\"2026-06-01T00:00:00Z\",\
            \"remote_hash\":\"r\",\"local_hash\":\"l\"}}}\n";
        std::fs::write(&baseline, seeded).expect("seed the baseline");
        let config = FakeConfig(
            [
                ("work.integration", "jira"),
                ("paths.integrations", "integrations"),
            ]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect(),
        );
        let registry = FixedRegistry(RefCell::new(Some(Box::new(
            RecordingTracker::holding(Vec::new()).failing_update(
                tracker::ExternalId::new("ENG-1".to_owned()),
                TrackerError::Rejected {
                    detail: "jira update: the body has a table".to_owned(),
                },
            ),
        ))));
        let args = UpdateArgs {
            path: item,
            sets: Vec::new(),
            add_tags: Vec::new(),
            remove_tags: Vec::new(),
            appends: Vec::new(),
            removes: Vec::new(),
            push: true,
        };

        let outcome = run(root.path(), &config, &args, &registry);

        let RunOutcome::PushRejected(message) = outcome else {
            panic!("an invalid request exits 75");
        };
        assert!(message.starts_with("E_PUSH_REJECTED: "), "{message}");
        assert!(message.contains("the body has a table"), "{message}");
        assert_eq!(
            std::fs::read_to_string(&baseline).expect("baseline"),
            seeded,
            "nothing was applied, so the baseline stands"
        );
    }
}

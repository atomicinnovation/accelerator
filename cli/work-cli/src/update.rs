//! Adapter/binary wiring for `work update`: validates every `--set`/
//! `--append`/`--remove` key before touching anything, then applies the
//! validated mutations as one `document::Yaml` tree edit and performs one
//! atomic write, under a per-file mkdir-lock.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

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
use work_adapters::sync::push_verdict::PushOrigin;
use work_adapters::sync::push_verdict::PushVerdict;

use crate::cli::UpdateArgs;
use crate::config::effective_nonempty;
use crate::exit_codes;
use crate::refusal::Refusal;
use crate::report_records::rewrite_lines;
use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;

pub enum RunOutcome {
    /// `warnings` are the lines stderr carries about a push that applied.
    Updated {
        warnings: Vec<String>,
    },
    Failed(String),
    PushNotAvailable(String),
    PushUnrecognised(String),
    PushUnconfigured(String),
    PushRetryable(String),
    PushTerminal(String),
    PushRejected(String),
    /// The identity run an `update --push` needs could not begin; nothing
    /// was read or written.
    Refused(Refusal),
}

impl RunOutcome {
    /// Prints this outcome as `accelerator work update` does, returning the
    /// code it exits with.
    pub fn report(self, _out: &mut dyn Write, err: &mut dyn Write) -> ExitCode {
        let failed = |err: &mut dyn Write, message: &str, code: u8| {
            let _ = writeln!(err, "{message}");
            ExitCode::from(code)
        };
        match self {
            Self::Updated { warnings } => {
                for warning in warnings {
                    let _ = writeln!(err, "{warning}");
                }
                ExitCode::SUCCESS
            }
            Self::Failed(message) => {
                let _ = writeln!(err, "{message}");
                ExitCode::FAILURE
            }
            Self::PushNotAvailable(message) => {
                failed(err, &message, exit_codes::NOT_AVAILABLE)
            }
            Self::PushUnrecognised(message) => {
                failed(err, &message, exit_codes::UNRECOGNISED)
            }
            Self::PushUnconfigured(message) => {
                failed(err, &message, exit_codes::UNCONFIGURED)
            }
            Self::PushRetryable(message) => {
                failed(err, &message, exit_codes::RETRYABLE)
            }
            Self::PushTerminal(message) => {
                failed(err, &message, exit_codes::TERMINAL)
            }
            Self::PushRejected(message) => {
                failed(err, &message, exit_codes::REJECTED)
            }
            Self::Refused(refusal) => refusal.report_to(err),
        }
    }
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
    Refused(Refusal),
}

impl From<Refusal> for TryRunError {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(refusal)
    }
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
) -> Result<Vec<String>, TryRunError> {
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
    let digester = BaselineDigester::new(tracker.as_ref(), &CommonMarkBody);
    let read_back = remote.as_ref().map(|issue| issue.body.as_str());
    let fingerprints = digester
        .digests(rendered, read_back)
        .map_err(|error| TryRunError::Generic(error.to_string()))?;
    let verdict = PushVerdict::judged(
        PushOrigin::Pushed,
        digester.comparison(),
        rendered,
        read_back,
    );
    let recorded = baseline_store
        .record(
            &id,
            pushed_entry(remote, fingerprints),
            verdict.mark_change(),
        )
        .map_err(|error| TryRunError::Generic(error.to_string()))?;
    work_adapters::sync::adoption_records::retire_confirmed(
        &integrations_root,
        &integration,
        &baseline_writer,
        &recorded,
    );

    Ok(rewrite_warnings(
        verdict,
        recorded.get(&id),
        (path, &external_id),
        &integration,
    ))
}

/// The baseline entry a push records from its read-back; an unread remote
/// leaves its stamp unread.
fn pushed_entry(
    remote: Option<tracker::RemoteIssue>,
    fingerprints: work::sync::DualRecipeFingerprints,
) -> baseline::Entry {
    baseline::Entry::recorded(
        baseline::SyncStamps {
            remote_updated_at: remote
                .map_or(tracker::RemoteTimestamp::NotRead, |issue| {
                    issue.updated
                }),
            local_synced_at: 0,
        },
        fingerprints,
    )
}

/// The block stderr carries for the item at `path` known remotely as
/// `external_id`: of this push when its read-back was rewritten, else of
/// the rewrite its recorded entry still marks.
fn rewrite_warnings(
    verdict: PushVerdict,
    recorded: Option<&baseline::Entry>,
    (path, external_id): (&Path, &tracker::ExternalId),
    integration: &str,
) -> Vec<String> {
    verdict
        .into_notice(path, external_id)
        .or_else(|| {
            recorded.and_then(|entry| entry.standing_rewrite(path, external_id))
        })
        .map(|notice| {
            rewrite_lines(&notice, &path.display().to_string(), integration)
        })
        .unwrap_or_default()
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
) -> Result<Vec<String>, TryRunError> {
    for (key, _) in &args.sets {
        validate_set_key(key).map_err(|error| set_key_error_message(&error))?;
    }
    for (key, _) in args.appends.iter().chain(&args.removes) {
        validate_list_key(key)?;
    }

    let _identity_run = if args.push {
        let root = config_adapters::FileConfigStore::discover_root(start);
        let guard = crate::identity_run::begin(&root, "update")
            .map_err(|refusal| crate::identity_run::refused(refusal, &root))?;
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
    Ok(Vec::new())
}

#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &UpdateArgs,
    registry: &dyn TrackerRegistry,
) -> RunOutcome {
    match try_run(start, config, args, registry) {
        Ok(warnings) => RunOutcome::Updated { warnings },
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
        Err(TryRunError::Refused(refusal)) => RunOutcome::Refused(refusal),
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
    use work_adapters::sync::rewrite_mark::MarkState;

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

        let RunOutcome::Refused(Refusal { message, code }) = outcome else {
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
            let _ = done.send(matches!(outcome, RunOutcome::Refused(_)));
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

        assert!(matches!(outcome, RunOutcome::Updated { .. }));
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

    const MARKED_BASELINE: &str = "{\"timestamp\":0,\"items\":{\"0001\":{\
        \"digest_recipe\":1,\"local_body_hash\":\"b\",\
        \"local_fields_hash\":\"f\",\"local_frontmatter_hash\":\"m\",\
        \"local_hash\":\"l\",\"local_synced_at\":0,\
        \"remote_body_hash\":\"rb\",\"remote_fields_hash\":\"rf\",\
        \"remote_hash\":\"r\",\"remote_updated_at\":\"\",\
        \"rewritten_on_push\":{\"lines\":[{\"section\":\"Earlier\",\
        \"sent\":\"before\",\"stored\":\"after\"}],\"more\":0}}}}\n";

    struct PushedRepo {
        root: tempfile::TempDir,
        item: std::path::PathBuf,
        content: String,
        config: FakeConfig,
    }

    impl PushedRepo {
        fn on(tracker: crate::test_support::real_trackers::Tracker) -> Self {
            let root = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
            std::fs::create_dir_all(
                root.path().join("integrations").join(tracker.integration()),
            )
            .expect("baseline dir");
            let content = format!(
                "---\nid: \"0001\"\ntitle: \"Title\"\nexternal_id: \"{}\"\n\
                 ---\n{}",
                tracker.key(),
                crate::test_support::real_trackers::NEEDS_BOTH_ESCAPES
            );
            let item = root.path().join("0001.md");
            std::fs::write(&item, &content).expect("seed the item");
            let config = FakeConfig(
                [
                    ("work.integration", tracker.integration()),
                    ("paths.integrations", "integrations"),
                ]
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
            );
            Self {
                root,
                item,
                content,
                config,
            }
        }

        fn body(&self) -> String {
            document::split(&self.content).expect("a body").body
        }

        fn baseline_path(
            &self,
            tracker: crate::test_support::real_trackers::Tracker,
        ) -> std::path::PathBuf {
            self.root.path().join(format!(
                "integrations/{}/last-sync.json",
                tracker.integration()
            ))
        }

        fn marked(
            self,
            tracker: crate::test_support::real_trackers::Tracker,
        ) -> Self {
            std::fs::write(self.baseline_path(tracker), MARKED_BASELINE)
                .expect("seed the marked baseline");
            self
        }

        fn mark(
            &self,
            tracker: crate::test_support::real_trackers::Tracker,
        ) -> MarkState {
            let text = std::fs::read_to_string(self.baseline_path(tracker))
                .expect("the baseline is recorded");
            match baseline::Baseline::read(Some(&text)).0.get("0001") {
                Some(baseline::Entry::Split(entry)) => entry.mark().clone(),
                other => panic!("a recipe-1 entry is recorded: {other:?}"),
            }
        }

        fn push(
            &self,
            served: &mut crate::test_support::real_trackers::ServedTracker,
        ) -> RunOutcome {
            run(
                self.root.path(),
                &self.config,
                &pushing_nothing_else(self.item.clone()),
                &crate::test_support::FixedRegistry::holding_boxed(
                    served.client(),
                ),
            )
        }

        fn assert_unchanged(&self) {
            assert_eq!(
                std::fs::read_to_string(&self.item).expect("item"),
                self.content
            );
        }
    }

    fn pushing_nothing_else(path: std::path::PathBuf) -> UpdateArgs {
        UpdateArgs {
            sets: Vec::new(),
            ..update_args(path, true)
        }
    }

    #[test]
    fn update_push_escapes_for_linear_alone() {
        use crate::test_support::real_trackers::Tracker;

        for tracker in Tracker::ALL {
            let repo = PushedRepo::on(tracker);
            let mut served = tracker.serve("Title", &repo.body());

            let outcome = repo.push(&mut served);

            assert!(
                matches!(outcome, RunOutcome::Updated { .. }),
                "{tracker:?}"
            );
            assert_eq!(
                served.sent_descriptions(),
                [tracker.expected_description(&repo.body())],
                "{tracker:?}"
            );
            repo.assert_unchanged();
        }
    }

    fn warnings_of(outcome: RunOutcome) -> Vec<String> {
        match outcome {
            RunOutcome::Updated { warnings } => warnings,
            _ => panic!("the push did not succeed"),
        }
    }

    #[test]
    fn a_rewritten_update_push_warns_on_stderr_and_exits_zero() {
        use crate::test_support::real_trackers::Tracker;

        let repo = PushedRepo::on(Tracker::Linear);
        let linked = repo
            .body()
            .replace("crates.io.", "[crates.io](<http://crates.io>).");
        let mut served = Tracker::Linear.serve("Title", &linked);

        let printed = crate::test_support::printed(|out, err| {
            repo.push(&mut served).report(out, err)
        });

        let label = repo.item.display().to_string();
        assert_eq!(printed.code, std::process::ExitCode::SUCCESS);
        assert_eq!(printed.out, "");
        assert!(
            printed.err.starts_with(&format!(
                "warning: {label}: rewritten on push: (preamble)\n"
            )),
            "{}",
            printed.err
        );
        assert!(
            printed.err.contains("sent `See crates.io.`"),
            "{}",
            printed.err
        );
    }

    #[test]
    fn update_push_warns_when_the_read_back_is_rewritten() {
        use crate::test_support::real_trackers::Tracker;

        for tracker in Tracker::ALL {
            let repo = PushedRepo::on(tracker);
            let linked = repo
                .body()
                .replace("crates.io.", "[crates.io](<http://crates.io>).");
            let mut served = tracker.serve("Title", &linked);

            let warnings = warnings_of(repo.push(&mut served));

            let label = repo.item.display().to_string();
            assert_eq!(
                warnings.first(),
                Some(&format!(
                    "warning: {label}: rewritten on push: (preamble)"
                )),
                "{tracker:?}"
            );
            assert!(
                warnings[1..]
                    .iter()
                    .all(|line| line.starts_with(&format!("note: {label}: "))),
                "{tracker:?}: {warnings:?}"
            );
            assert!(
                warnings
                    .iter()
                    .any(|line| line.contains("sent `See crates.io.`")),
                "{tracker:?}: {warnings:?}"
            );
            let baseline =
                std::fs::read_to_string(repo.root.path().join(format!(
                    "integrations/{}/last-sync.json",
                    tracker.integration()
                )))
                .expect("the baseline is recorded");
            assert!(baseline.contains("\"0001\""), "{tracker:?}");
            assert!(
                matches!(repo.mark(tracker), MarkState::Marked(_)),
                "{tracker:?}"
            );
            repo.assert_unchanged();
        }
    }

    #[test]
    fn a_faithful_update_push_clears_the_mark() {
        use crate::test_support::real_trackers::Tracker;

        for tracker in Tracker::ALL {
            let repo = PushedRepo::on(tracker).marked(tracker);
            let mut served = tracker.serve("Title", &repo.body());

            repo.push(&mut served);

            assert_eq!(repo.mark(tracker), MarkState::Absent, "{tracker:?}");
        }
    }

    #[test]
    fn an_unjudged_update_push_keeps_the_mark() {
        use crate::test_support::real_trackers::Tracker;

        let repo = PushedRepo::on(Tracker::Linear).marked(Tracker::Linear);
        let before = repo.mark(Tracker::Linear);
        let external_id = tracker::ExternalId::new("ENG-1".to_owned());
        let unreadable = RecordingTracker::holding(vec![(
            external_id.clone(),
            tracker::RemoteIssue {
                key: external_id.clone(),
                updated: tracker::RemoteTimestamp::NotReported,
                body: "Title\nOld body\n".to_owned(),
            },
        )])
        .failing_show(
            external_id,
            TrackerError::Retryable {
                detail: "the read-back timed out".to_owned(),
            },
        );

        let outcome = run(
            repo.root.path(),
            &repo.config,
            &pushing_nothing_else(repo.item.clone()),
            &crate::test_support::FixedRegistry::holding(unreadable),
        );

        let label = repo.item.display().to_string();
        assert_eq!(
            warnings_of(outcome),
            [
                format!("warning: {label}: still rewritten on push: Earlier"),
                format!(
                    "note: {label}: Earlier: sent `before`, stored `after`"
                ),
                format!(
                    "note: {label}: to keep the meaning, put the sent text in \
                     backticks or reword it, then run `accelerator work \
                     update {label} --push`"
                ),
            ]
        );
        assert_eq!(repo.mark(Tracker::Linear), before);
    }

    #[test]
    fn a_faithful_update_push_warns_of_nothing() {
        use crate::test_support::real_trackers::Tracker;

        for tracker in Tracker::ALL {
            let repo = PushedRepo::on(tracker);
            let mut served = tracker.serve("Title", &repo.body());

            assert_eq!(
                warnings_of(repo.push(&mut served)),
                Vec::<String>::new(),
                "{tracker:?}"
            );
        }
    }

    #[test]
    fn update_push_repairs_a_corrupted_remote() {
        use crate::test_support::real_trackers::Tracker;

        let repo = PushedRepo::on(Tracker::Linear);
        let corrupted = repo
            .body()
            .replace("crates.io.", "[crates.io](<http://crates.io>).");
        let baseline_path =
            repo.root.path().join("integrations/linear/last-sync.json");
        let store = FileCorpusStore::new(
            baseline_path.parent().expect("a baseline directory"),
        );
        BaselineStore::new(baseline_path, &RealFs, &store)
            .seed(
                "0001",
                baseline::Entry::recorded(
                    baseline::SyncStamps {
                        remote_updated_at: tracker::RemoteTimestamp::NotRead,
                        local_synced_at: 0,
                    },
                    BaselineDigester::new(&PlainDialect, &CommonMarkBody)
                        .digests(
                            &repo.content,
                            Some(&format!("Title\n{corrupted}")),
                        )
                        .expect("digests"),
                ),
            )
            .expect("the corrupted baseline is seeded");
        let mut served = Tracker::Linear.serve("Title", &corrupted);

        let outcome = repo.push(&mut served);

        assert!(matches!(outcome, RunOutcome::Updated { .. }));
        assert_eq!(
            served.sent_descriptions(),
            [Tracker::Linear.expected_description(&repo.body())]
        );
        repo.assert_unchanged();
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

        assert!(
            matches!(outcome, RunOutcome::Updated { .. }),
            "the push succeeds"
        );
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
        let Some((legacy, split)) = baseline
            .get("0001")
            .and_then(|entry| entry.legacy().zip(entry.split()))
        else {
            panic!("a recipe-1 entry is recorded");
        };
        assert_eq!(split, &expected.split);
        assert_eq!(legacy, &expected.legacy);
    }

    #[test]
    fn a_pushed_update_retires_the_adoption_record_it_confirms() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
        let item = root.path().join("0001.md");
        std::fs::write(
            &item,
            "---\nid: \"0001\"\ntitle: \"Title\"\nexternal_id: \"ENG-1\"\n\
             ---\n\nBody\n",
        )
        .expect("seed the item");
        let record = root.path().join("integrations/linear/adopted/0001.json");
        std::fs::create_dir_all(record.parent().expect("parent"))
            .expect("record dir");
        std::fs::write(
            &record,
            "{\"previous_key\":null,\"key\":\"ENG-1\",\"adopted_at\":1}\n",
        )
        .expect("seed the record");
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
            path: item,
            sets: Vec::new(),
            add_tags: Vec::new(),
            remove_tags: Vec::new(),
            appends: Vec::new(),
            removes: Vec::new(),
            push: true,
        };

        let outcome = run(root.path(), &config, &args, &registry);

        assert!(
            matches!(outcome, RunOutcome::Updated { .. }),
            "the push succeeds"
        );
        assert!(!record.exists(), "the confirmed link's record is retired");
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

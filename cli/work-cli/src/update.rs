//! Adapter/binary wiring for `work update`: validates every `--set`/
//! `--append`/`--remove` key before touching anything, then applies the
//! validated mutations as one `document::Yaml` tree edit and performs one
//! atomic write, under a per-file mkdir-lock.

use std::path::Path;

use ::config::ConfigAccess;
use corpus::AtomicWrite;
use corpus_adapters::lock::acquire;
use corpus_adapters::lock::LockOptions;
use corpus_adapters::FileCorpusStore;
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
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::digest;

use crate::cli::UpdateArgs;
use crate::config::effective_nonempty;
use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;

pub enum RunOutcome {
    Updated,
    Failed(String),
    PushNotAvailable(String),
    PushUnrecognised(String),
    PushRetryable(String),
    PushTerminal(String),
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
    Retryable(String),
    Terminal(String),
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
    let mut baseline_store =
        BaselineStore::new(baseline_path, &file_reader, &baseline_writer);
    let external_id = tracker::ExternalId::new(external_id);

    match tracker.update(&external_id, &title, &body) {
        Ok(()) => {}
        Err(TrackerError::Retryable { detail }) => {
            return Err(TryRunError::Retryable(format!(
                "E_PUSH_RETRYABLE: pushing {} failed and can be retried: \
                 {detail}",
                path.display()
            )));
        }
        Err(TrackerError::Terminal { detail }) => {
            baseline_store.remove(&id).ok();
            return Err(TryRunError::Terminal(format!(
                "E_PUSH_TERMINAL: pushing {} failed and may have partially \
                 applied: {detail}. The baseline entry has been cleared; \
                 the next sync will classify this item as a conflict.",
                path.display()
            )));
        }
    }

    let store =
        FileCorpusStore::new(path.parent().unwrap_or_else(|| Path::new(".")));
    AtomicWrite::write(&store, path, rendered.as_bytes())
        .map_err(|error| TryRunError::Generic(error.to_string()))?;

    let (remote_updated, remote_hash) = match tracker.show(&external_id) {
        Ok(issue) => (issue.updated, digest::remote_body(&issue.body)),
        Err(_) => (tracker::RemoteTimestamp::NotRead, String::new()),
    };
    let local_hash = digest::local(rendered)
        .map_err(|error| TryRunError::Generic(error.to_string()))?;
    baseline_store
        .set(
            &id,
            work_adapters::sync::baseline::Entry {
                remote_updated_at: remote_updated,
                remote_hash,
                local_hash,
            },
        )
        .map_err(|error| TryRunError::Generic(error.to_string()))?;

    Ok(())
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

    let lockdir = {
        let mut name = args.path.as_os_str().to_owned();
        name.push(".lockdir");
        std::path::PathBuf::from(name)
    };
    let _guard =
        acquire(&lockdir, LockOptions::default()).map_err(|error| {
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
        Err(TryRunError::Retryable(message)) => {
            RunOutcome::PushRetryable(message)
        }
        Err(TryRunError::Terminal(message)) => {
            RunOutcome::PushTerminal(message)
        }
    }
}

//! Adapter/binary wiring for `work update`: validates every `--set`/
//! `--append`/`--remove` key before touching anything, then applies the
//! validated mutations as one `document::Yaml` tree edit and performs one
//! atomic write, under a per-file mkdir-lock.

use std::path::Path;

use corpus::AtomicWrite;
use corpus_adapters::lock::acquire;
use corpus_adapters::lock::LockOptions;
use corpus_adapters::FileCorpusStore;
use document::Mapping;
use document::Scalar;
use document::Yaml;
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

use crate::cli::UpdateArgs;

pub enum RunOutcome {
    Updated,
    Failed(String),
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

fn try_run(args: &UpdateArgs) -> Result<(), String> {
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
        return Err("frontmatter is not a mapping".to_owned());
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

    let rendered = document::render(Some(&content), &yaml)
        .map_err(|error| error.to_string())?;
    let store = FileCorpusStore::new(
        args.path.parent().unwrap_or_else(|| Path::new(".")),
    );
    AtomicWrite::write(&store, &args.path, rendered.as_bytes())
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[must_use]
pub fn run(args: &UpdateArgs) -> RunOutcome {
    match try_run(args) {
        Ok(()) => RunOutcome::Updated,
        Err(message) => RunOutcome::Failed(message),
    }
}

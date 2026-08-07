//! Adapter/binary wiring for `work create`: allocates the next ID under a
//! per-directory lock, derives metadata, composes the frontmatter via
//! `work::create::compose_frontmatter`, and performs one atomic write.
//! `work create` has no bash original to inherit exit codes from — see the
//! plan's own exit-code contract.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use ::config::ReadTemplate;
use corpus::AtomicWrite;
use corpus::FilenameTimestampFormat;
use corpus_adapters::compile_scan_regex;
use corpus_adapters::lock::acquire;
use corpus_adapters::lock::LockOptions;
use corpus_adapters::metadata::derive_at;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::RegexScanner;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use work::create::assert_matches_template_schema;
use work::create::compose_frontmatter;
use work::create::resolve_author;
use work::create::CreateInputs;
use work::create::FieldValue;
use work::create::TypedLinkage;
use work::next_number::allocate;
use work::next_number::AllocationError;
use work::resolve::DirectoryLister;
use work_adapters::author::VcsBackedIdentityProbe;
use work_adapters::filesystem::FilesystemLister;

use crate::config::configured_override;
use crate::config::resolve_scheme;
use crate::config::resolve_work_dir;
use crate::config::templates_dir;

const ID_PLACEHOLDER: &str = "NNNN";
const TITLE_PLACEHOLDER: &str = "Title as Short Noun Phrase";
const LOCK_FILE_NAME: &str = ".accelerator-work-create.lockdir";

pub struct CreateArgs {
    pub title: String,
    pub kind: String,
    pub priority: String,
    pub status: String,
    pub parent: Option<String>,
    pub tags: Vec<String>,
    pub blocks: Vec<String>,
    pub blocked_by: Vec<String>,
    pub derived_from: Vec<String>,
    pub relates_to: Vec<String>,
    pub source: Option<String>,
    pub project: Option<String>,
    pub author: Option<String>,
    pub producer: String,
    pub body_file: Option<PathBuf>,
}

pub enum RunOutcome {
    Created(PathBuf),
    Failed(String),
}

const SLUG_MAX_LEN: usize = 60;

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut last_was_hyphen = true;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            slug.push('-');
            last_was_hyphen = true;
        }
    }
    let trimmed = slug.trim_end_matches('-');
    if trimmed.len() <= SLUG_MAX_LEN {
        return trimmed.to_owned();
    }
    let cut = trimmed[..SLUG_MAX_LEN].rfind('-').unwrap_or(SLUG_MAX_LEN);
    trimmed[..cut].trim_end_matches('-').to_owned()
}

fn allocation_message(error: &AllocationError, pattern: &str) -> String {
    match error {
        AllocationError::MissingProject => {
            format!(
                "E_PATTERN_MISSING_PROJECT: pattern '{pattern}' contains \
                 {{project}} but no value supplied — pass --project or set \
                 work.default_project_code"
            )
        }
        AllocationError::ProjectUnused => {
            format!(
                "E_PATTERN_PROJECT_UNUSED: --project is meaningless for \
                 pattern '{pattern}' (no {{project}} token)"
            )
        }
        AllocationError::Overflow {
            highest,
            highest_file,
            cap,
            ..
        } => {
            if highest > cap {
                format!(
                    "E_PATTERN_OVERFLOW: out-of-width file '{}' has number \
                     {highest} exceeding the pattern '{pattern}' cap of \
                     {cap}. Rename the stray file or widen the pattern.",
                    highest_file.as_deref().unwrap_or("<unknown>")
                )
            } else {
                format!(
                    "E_PATTERN_OVERFLOW: pattern '{pattern}' number space \
                     exhausted (highest={highest}, cap={cap}). Archive \
                     completed work items or widen the pattern."
                )
            }
        }
    }
}

fn field_to_yaml(value: FieldValue) -> Yaml {
    match value {
        FieldValue::Scalar(text) => Yaml::Scalar(Scalar::String(text)),
        FieldValue::Sequence(items) => Yaml::Sequence(
            items
                .into_iter()
                .map(|item| Yaml::Scalar(Scalar::String(item)))
                .collect(),
        ),
        FieldValue::Int(number) => Yaml::Scalar(Scalar::Int(number)),
    }
}

fn template_frontmatter_keys(content: &str) -> Result<Vec<String>, String> {
    let parsed = document::parse(content).map_err(|error| error.to_string())?;
    match parsed {
        Yaml::Mapping(mapping) => Ok(mapping
            .entries()
            .iter()
            .map(|(key, _)| key.clone())
            .collect()),
        Yaml::Scalar(_) | Yaml::Sequence(_) => {
            Err("template frontmatter is not a mapping".to_owned())
        }
    }
}

fn allocate_id(
    scheme: &corpus::WorkItemIdScheme,
    work_dir: &Path,
    project: Option<&str>,
) -> Result<String, String> {
    let filenames = FilesystemLister::new(work_dir).filenames();
    let scan_regex =
        compile_scan_regex(&scheme.id_pattern, project.unwrap_or(""))
            .map_err(|error| error.to_string())?;
    let scanner = RegexScanner::compile(&scan_regex)
        .map_err(|error| error.to_string())?;
    let allocated = allocate(scheme, project, 1, &filenames, &scanner)
        .map_err(|error| allocation_message(&error, &scheme.id_pattern))?;
    allocated
        .into_iter()
        .next()
        .ok_or_else(|| "allocation produced no ID".to_owned())
}

fn resolve_and_check_template(
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
) -> Result<::config::ResolvedTemplate, String> {
    let templates_dir_value =
        templates_dir(config).map_err(|error| error.to_string())?;
    let template_override = configured_override(config, "templates.work-item")
        .map_err(|error| error.to_string())?;
    let resolved_template = templates
        .resolve_template(
            "work-item",
            template_override.as_deref(),
            &templates_dir_value,
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "work-item template not found".to_owned())?;

    let template_keys = template_frontmatter_keys(&resolved_template.content)?;
    assert_matches_template_schema(&template_keys).map_err(|drift| {
        format!(
            "work-item template schema has drifted from the known fields \
             — missing from template: {:?}, unknown to this work item: \
             {:?}",
            drift.missing_from_template, drift.unknown_to_this_work_item
        )
    })?;
    Ok(resolved_template)
}

fn render_frontmatter(inputs: &CreateInputs<'_>) -> Result<String, String> {
    let fields = compose_frontmatter(inputs);
    let mut mapping = Mapping::new();
    for (key, value) in fields {
        mapping.push(key, field_to_yaml(value));
    }
    document::render(None, &Yaml::Mapping(mapping))
        .map_err(|error| error.to_string())
}

fn resolve_body(
    args: &CreateArgs,
    resolved_template: &::config::ResolvedTemplate,
    id: &str,
) -> Result<String, String> {
    let raw_body = match &args.body_file {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|error| format!("could not read --body-file: {error}"))?,
        None => {
            document::split(&resolved_template.content)
                .map_err(|error| error.to_string())?
                .body
        }
    };
    Ok(raw_body
        .replace(ID_PLACEHOLDER, id)
        .replace(TITLE_PLACEHOLDER, &args.title))
}

fn try_run(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateArgs,
) -> Result<PathBuf, String> {
    let scheme = resolve_scheme(config).map_err(|error| error.to_string())?;
    let root = config_adapters::FileConfigStore::discover_root(start);
    let work_dir =
        resolve_work_dir(config, &root).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&work_dir).map_err(|error| {
        format!("could not create the work-item directory: {error}")
    })?;

    let lockdir = work_dir.join(LOCK_FILE_NAME);
    let _guard =
        acquire(&lockdir, LockOptions::default()).map_err(|error| {
            format!("could not acquire the work-item creation lock: {error}")
        })?;

    let project = args
        .project
        .clone()
        .or_else(|| scheme.default_project_code.clone());
    let id = allocate_id(&scheme, &work_dir, project.as_deref())?;

    let metadata =
        derive_at(&root, FilenameTimestampFormat::DateTimeUnderscored)
            .map_err(|error| error.to_string())?;
    let author =
        resolve_author(args.author.as_deref(), &VcsBackedIdentityProbe)?;
    let resolved_template = resolve_and_check_template(config, templates)?;

    let inputs = CreateInputs {
        id: &id,
        title: &args.title,
        kind: &args.kind,
        priority: &args.priority,
        status: &args.status,
        linkage: TypedLinkage {
            parent: args.parent.as_deref(),
            blocks: &args.blocks,
            blocked_by: &args.blocked_by,
            derived_from: &args.derived_from,
            relates_to: &args.relates_to,
            source: args.source.as_deref(),
        },
        tags: &args.tags,
        author: &author,
        producer: &args.producer,
        date: &metadata.datetime_utc,
    };
    let frontmatter_block = render_frontmatter(&inputs)?;
    let body = resolve_body(args, &resolved_template, &id)?;

    let slug = slugify(&args.title);
    let target = work_dir.join(format!("{id}-{slug}.md"));
    if target.exists() {
        return Err(format!(
            "refusing to overwrite an existing file: {}",
            target.display()
        ));
    }

    let content = format!("{frontmatter_block}{body}");
    let store = FileCorpusStore::new(&work_dir);
    AtomicWrite::write(&store, &target, content.as_bytes())
        .map_err(|error| error.to_string())?;

    Ok(target)
}

/// # Errors
///
/// Never returns `Err`; every failure is reported through [`RunOutcome`].
#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateArgs,
) -> RunOutcome {
    match try_run(start, config, templates, args) {
        Ok(path) => RunOutcome::Created(path),
        Err(message) => RunOutcome::Failed(message),
    }
}

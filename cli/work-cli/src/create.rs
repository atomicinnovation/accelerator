//! Adapter/binary wiring for `work create`: allocates the next ID under a
//! per-directory lock, derives metadata, composes the frontmatter via
//! `work::create::compose_frontmatter`, and performs one atomic write.
//! `work create` has no bash original to inherit exit codes from.

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
use corpus_adapters::metadata::VcsBackedRepoFactsProbe;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::RegexScanner;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use tracker::ExternalId;
use tracker::TrackerError;
use work::create::assert_matches_template_schema;
use work::create::compose_frontmatter;
use work::create::resolve_author;
use work::create::CreateInputs;
use work::create::FieldValue;
use work::create::TypedLinkage;
use work::next_number::allocate;
use work::next_number::AllocationError;
use work::resolve::DirectoryLister;
use work::sync::MarkerState;
use work::sync::PendingPush;
use work::sync::PushOutcome;
use work::sync::PushPrecondition;
use work::sync::RefusalReason;
use work::sync::RequestFingerprint;
use work_adapters::author::VcsBackedIdentityProbe;
use work_adapters::filesystem::FilesystemLister;

use crate::config::configured_override;
use crate::config::effective_nonempty;
use crate::config::resolve_scheme;
use crate::config::resolve_work_dir;
use crate::config::templates_dir;
use crate::exit_codes;
use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;

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
    pub push: bool,
}

pub struct PushReport {
    pub outcome: PushOutcome,
    pub external_id: Option<String>,
}

pub enum RunOutcome {
    Created {
        path: PathBuf,
        push: Option<PushReport>,
    },
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

fn corpus_carries_external_id(
    work_dir: &Path,
    external_id: &ExternalId,
) -> bool {
    let Ok(entries) = std::fs::read_dir(work_dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("md") {
            return false;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            return false;
        };
        let Ok((frontmatter, _)) =
            work_adapters::sync::digest::split_frontmatter_and_body(&content)
        else {
            return false;
        };
        work::show::read_field_raw(&frontmatter, "external_id").is_some_and(
            |raw| {
                raw.trim_matches(|c: char| {
                    c.is_ascii_whitespace() || c == '"' || c == '\''
                }) == external_id.as_str()
            },
        )
    })
}

fn refusal_message(
    reason: RefusalReason,
    marker_path: &Path,
    marker: Option<&PendingPush>,
) -> String {
    match reason {
        RefusalReason::MarkerUnreadable => format!(
            "E_PUSH_MARKER_UNREADABLE: the pending-push marker at {} could \
             not be parsed; a previous create may have partially applied. \
             Inspect or remove the marker before retrying.",
            marker_path.display()
        ),
        RefusalReason::PriorAttemptUnknownOutcome => {
            let Some(PendingPush::Attempted { request }) = marker else {
                unreachable!("PriorAttemptUnknownOutcome always carries an Attempted marker")
            };
            format!(
                "E_PUSH_PENDING: a previous create attempt for '{}' at {} \
                 (recorded at {}{}) has an unknown outcome — a remote \
                 issue may already exist. Inspect it, then remove the \
                 marker to retry.",
                request.title,
                marker_path.display(),
                request.attempted_at,
                request
                    .failure
                    .as_deref()
                    .map(|detail| format!(", failure: {detail}"))
                    .unwrap_or_default()
            )
        }
        RefusalReason::FingerprintMismatch => format!(
            "E_PUSH_FINGERPRINT_MISMATCH: the pending-push marker at {} was \
             recorded for a different request with the same title but a \
             different body or kind. Remove the marker to force a new \
             create.",
            marker_path.display()
        ),
        RefusalReason::AlreadyWritten => {
            let Some(PendingPush::Created { external_id, .. }) = marker else {
                unreachable!("AlreadyWritten always carries a Created marker")
            };
            format!(
                "E_PUSH_ALREADY_WRITTEN: the pending-push marker at {} \
                 claims external_id '{}', already carried by a work item \
                 on disk. Remove the marker if this is a genuine duplicate \
                 create.",
                marker_path.display(),
                external_id.as_str()
            )
        }
    }
}

const fn dispatch_code_for_selection_error(error: &SelectionError) -> u8 {
    match error {
        SelectionError::NotAvailable { .. } => exit_codes::NOT_AVAILABLE,
        SelectionError::Unconfigured { .. } => exit_codes::UNCONFIGURED,
        SelectionError::Unset | SelectionError::Unrecognised { .. } => {
            exit_codes::UNRECOGNISED
        }
    }
}

fn attempted_at_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

struct PushExecution {
    outcome: PushOutcome,
    external_id: Option<String>,
    marker_to_delete_after_write: Option<PathBuf>,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn execute_push(
    integrations_root: &Path,
    integration: &str,
    slug: &str,
    args: &CreateArgs,
    body: &str,
    work_dir: &Path,
    registry: &dyn TrackerRegistry,
) -> Result<PushExecution, String> {
    let marker_path = work_adapters::sync::pending_push::path(
        integrations_root,
        integration,
        slug,
    );
    let marker_store =
        FileCorpusStore::new(marker_path.parent().unwrap_or(integrations_root));
    let marker_content = std::fs::read_to_string(&marker_path).ok();
    let parsed =
        work_adapters::sync::pending_push::read(marker_content.as_deref());
    let digest = work_adapters::sync::pending_push::request_digest(
        &args.title,
        body,
        &args.kind,
    );

    let marker_state = match &parsed {
        Err(_) => MarkerState::Unreadable,
        Ok(None) => MarkerState::Absent,
        Ok(Some(marker)) => MarkerState::Present(marker),
    };
    let corpus_carries =
        |id: &ExternalId| corpus_carries_external_id(work_dir, id);
    let precondition =
        work::sync::push_precondition(&marker_state, &digest, &corpus_carries);

    match precondition {
        PushPrecondition::Refuse(reason) => Err(refusal_message(
            reason,
            &marker_path,
            parsed.ok().flatten().as_ref(),
        )),
        PushPrecondition::ReuseId(external_id) => Ok(PushExecution {
            outcome: PushOutcome::WriteOnce,
            external_id: Some(external_id.as_str().to_owned()),
            marker_to_delete_after_write: Some(marker_path),
        }),
        PushPrecondition::Proceed => {
            let tracker = match registry.resolve(integration) {
                Ok(tracker) => tracker,
                Err(error) => {
                    let outcome = work::sync::push_decide(
                        dispatch_code_for_selection_error(&error),
                        1,
                        false,
                    );
                    return Ok(PushExecution {
                        outcome,
                        external_id: None,
                        marker_to_delete_after_write: None,
                    });
                }
            };

            let fingerprint = RequestFingerprint {
                title: args.title.clone(),
                digest,
                attempted_at: attempted_at_epoch(),
                failure: None,
            };
            let attempted = PendingPush::Attempted {
                request: fingerprint.clone(),
            };
            AtomicWrite::write(
                &marker_store,
                &marker_path,
                work_adapters::sync::pending_push::render(&attempted)
                    .as_bytes(),
            )
            .map_err(|error| error.to_string())?;

            match tracker.create(&args.title, body, &args.kind) {
                Ok(external_id) => {
                    let created = PendingPush::Created {
                        request: fingerprint,
                        external_id: external_id.clone(),
                    };
                    AtomicWrite::write(
                        &marker_store,
                        &marker_path,
                        work_adapters::sync::pending_push::render(&created)
                            .as_bytes(),
                    )
                    .map_err(|error| error.to_string())?;
                    Ok(PushExecution {
                        outcome: PushOutcome::WriteOnce,
                        external_id: Some(external_id.as_str().to_owned()),
                        marker_to_delete_after_write: Some(marker_path),
                    })
                }
                Err(error @ TrackerError::Retryable { .. }) => {
                    std::fs::remove_file(&marker_path).ok();
                    let outcome = work::sync::push_decide(
                        exit_codes::for_tracker_error(&error),
                        1,
                        false,
                    );
                    Ok(PushExecution {
                        outcome,
                        external_id: None,
                        marker_to_delete_after_write: None,
                    })
                }
                Err(TrackerError::Terminal { detail }) => {
                    let failed = PendingPush::Attempted {
                        request: RequestFingerprint {
                            failure: Some(detail),
                            ..fingerprint
                        },
                    };
                    AtomicWrite::write(
                        &marker_store,
                        &marker_path,
                        work_adapters::sync::pending_push::render(&failed)
                            .as_bytes(),
                    )
                    .map_err(|error| error.to_string())?;
                    let outcome =
                        work::sync::push_decide(exit_codes::TERMINAL, 1, false);
                    Ok(PushExecution {
                        outcome,
                        external_id: None,
                        marker_to_delete_after_write: None,
                    })
                }
            }
        }
    }
}

fn try_run(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateArgs,
    registry: &dyn TrackerRegistry,
) -> Result<(PathBuf, Option<PushReport>), String> {
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

    let metadata = derive_at(
        &root,
        FilenameTimestampFormat::DateTimeUnderscored,
        &VcsBackedRepoFactsProbe,
    )
    .map_err(|error| error.to_string())?;
    let author =
        resolve_author(args.author.as_deref(), &VcsBackedIdentityProbe)?;
    let resolved_template = resolve_and_check_template(config, templates)?;
    let body = resolve_body(args, &resolved_template, &id)?;

    let slug = slugify(&args.title);
    let target = work_dir.join(format!("{id}-{slug}.md"));
    if target.exists() {
        return Err(format!(
            "refusing to overwrite an existing file: {}",
            target.display()
        ));
    }

    let (external_id, push_report, marker_to_delete_after_write) = if args.push
    {
        let integration = effective_nonempty(config, "work.integration")
            .map_err(|error| error.to_string())?;
        let integrations_root = crate::sync::integrations_dir(config, &root)
            .map_err(|error| error.to_string())?;
        let execution = execute_push(
            &integrations_root,
            &integration,
            &slug,
            args,
            &body,
            &work_dir,
            registry,
        )?;
        (
            execution.external_id.clone(),
            Some(PushReport {
                outcome: execution.outcome,
                external_id: execution.external_id,
            }),
            execution.marker_to_delete_after_write,
        )
    } else {
        (None, None, None)
    };

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
        external_id: external_id.as_deref(),
    };
    let frontmatter_block = render_frontmatter(&inputs)?;

    let content = format!("{frontmatter_block}{body}");
    let store = FileCorpusStore::new(&work_dir);
    AtomicWrite::write(&store, &target, content.as_bytes())
        .map_err(|error| error.to_string())?;

    if let Some(marker_path) = marker_to_delete_after_write {
        std::fs::remove_file(&marker_path).ok();
    }

    Ok((target, push_report))
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
    registry: &dyn TrackerRegistry,
) -> RunOutcome {
    match try_run(start, config, templates, args, registry) {
        Ok((path, push)) => RunOutcome::Created { path, push },
        Err(message) => RunOutcome::Failed(message),
    }
}

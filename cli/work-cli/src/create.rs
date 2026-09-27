//! Adapter/binary wiring for `work create`: allocates the next ID under a
//! per-directory lock, derives metadata, composes the frontmatter via
//! `work::create::compose_frontmatter`, and performs one atomic write.

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
use tracker::CreatePreview;
use tracker::ExternalId;
use tracker::FieldResolution;
use tracker::TrackerError;
use work::create::assert_matches_template_schema;
use work::create::compose_frontmatter;
use work::create::resolve_author;
use work::create::CreateInputs;
use work::create::FieldValue;
use work::create::TypedLinkage;
use work::identity::linker_of;
use work::next_number::allocate;
use work::next_number::AllocationError;
use work::resolve::DirectoryLister;
use work::sync::MarkerState;
use work::sync::PendingPush;
use work::sync::PushOutcome;
use work::sync::PushPrecondition;
use work::sync::RefusalReason;
use work::sync::RequestFingerprint;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles;
use work_adapters::author::VcsBackedIdentityProbe;
use work_adapters::filesystem::FilesystemLister;
use work_adapters::filesystem::FilesystemWorkItemFiles;

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
pub const LOCK_FILE_NAME: &str = ".accelerator-work-create.lockdir";

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
    pub dry_run: bool,
}

pub struct PushReport {
    pub outcome: PushOutcome,
    pub external_id: Option<String>,
    pub cause: Option<String>,
}

pub enum RunOutcome {
    Created {
        path: PathBuf,
        push: Option<PushReport>,
    },
    Previewed(String),
    PreviewFailed {
        message: String,
        code: u8,
    },
    Failed(String),
}

const SLUG_MAX_LEN: usize = 60;

pub fn slugify(title: &str) -> String {
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
        AllocationError::MissingKey => {
            format!(
                "E_PATTERN_MISSING_KEY: pattern '{pattern}' references the \
                 {{key}} prefix but no value supplied — pass --project or \
                 set work.key"
            )
        }
        AllocationError::ProjectUnused => {
            format!(
                "E_PATTERN_KEY_UNUSED: --project is meaningless for pattern \
                 '{pattern}' (no {{key}} token)"
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

pub fn allocate_id(
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

pub fn render_frontmatter(inputs: &CreateInputs<'_>) -> Result<String, String> {
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
    let files = FilesystemWorkItemFiles::new(work_dir)
        .files()
        .unwrap_or_default();
    linker_of(external_id.as_str(), &identities(&files)).is_some()
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

const LINEAR_INTEGRATION: &str = "linear";

fn field_value_and_source(
    resolution: &FieldResolution,
) -> (&str, &'static str) {
    match resolution {
        FieldResolution::Resolved(value) => (value, "configured"),
        FieldResolution::Unset => ("", "default"),
        FieldResolution::Unresolvable(value) => (value, "unresolvable"),
    }
}

/// Renders a create preview as the single tab-separated line the create skill
/// parses. Linear has no user-resolvable type or project fields (its team and
/// issue-type catalogue are fixed), so its line is a fixed sentinel the skill
/// reads as "nothing to resolve"; every other provider renders its issue-type
/// and project as `<value>\t<source>` pairs, where an `unresolvable` project
/// source is the state the create-preview exists to surface.
fn render_create_preview(integration: &str, preview: &CreatePreview) -> String {
    if integration == LINEAR_INTEGRATION {
        return format!(
            "{integration}\t(no user-resolvable type/project fields)"
        );
    }
    let (type_value, type_source) = field_value_and_source(&preview.issue_type);
    let (project_value, project_source) =
        field_value_and_source(&preview.project);
    format!(
        "{integration}\t{type_value}\t{type_source}\t{project_value}\t\
         {project_source}"
    )
}

fn preview_push(
    integration: &str,
    kind: &str,
    registry: &dyn TrackerRegistry,
) -> RunOutcome {
    let tracker = match registry.resolve(integration) {
        Ok(tracker) => tracker,
        Err(error) => {
            return RunOutcome::PreviewFailed {
                message: error.message(),
                code: dispatch_code_for_selection_error(&error),
            };
        }
    };
    match tracker.preview_create(kind) {
        Ok(preview) => {
            RunOutcome::Previewed(render_create_preview(integration, &preview))
        }
        Err(error) => RunOutcome::PreviewFailed {
            code: exit_codes::for_tracker_error(&error),
            message: format!(
                "could not resolve the create preview against '{integration}': \
                 {}",
                error.into_detail()
            ),
        },
    }
}

enum CreateRetryOutcome {
    Created(ExternalId),
    Exhausted,
    Terminal(String),
    Rejected(String),
}

/// Drives the create call with the retryable/terminal policy `push_decide`
/// owns: a `70` (retryable) failure is retried once, then the item is saved
/// unsynced; a `71` (terminal) failure is never retried — a remote issue may
/// already exist; a `75` (rejected) failure is never retried either, because
/// the same request would be refused again. The retry count is bounded by
/// `push_decide` returning `Retry` only on the first attempt.
fn drive_create_retry<F>(mut attempt_create: F) -> CreateRetryOutcome
where
    F: FnMut() -> Result<ExternalId, TrackerError>,
{
    let mut attempt: u8 = 1;
    loop {
        match attempt_create() {
            Ok(external_id) => return CreateRetryOutcome::Created(external_id),
            Err(error) => {
                let code = exit_codes::for_tracker_error(&error);
                match work::sync::push_decide(code, attempt, false) {
                    PushOutcome::Retry => attempt += 1,
                    PushOutcome::LocalSave => {
                        return CreateRetryOutcome::Exhausted
                    }
                    PushOutcome::LoudTerminal => {
                        return CreateRetryOutcome::Terminal(
                            error.into_detail(),
                        )
                    }
                    PushOutcome::Rejected => {
                        return CreateRetryOutcome::Rejected(
                            error.into_detail(),
                        )
                    }
                    PushOutcome::WriteOnce => {
                        unreachable!("a non-zero code never yields WriteOnce")
                    }
                }
            }
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
    cause: Option<String>,
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
            cause: None,
            marker_to_delete_after_write: Some(marker_path),
        }),
        PushPrecondition::Proceed => {
            work_adapters::sync::pending_push::prepare_dir(
                integrations_root,
                integration,
            )
            .map_err(|error| error.to_string())?;
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
                        cause: None,
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

            match drive_create_retry(|| {
                tracker.create(&args.title, body, &args.kind)
            }) {
                CreateRetryOutcome::Created(external_id) => {
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
                        cause: None,
                        marker_to_delete_after_write: Some(marker_path),
                    })
                }
                CreateRetryOutcome::Exhausted => {
                    std::fs::remove_file(&marker_path).ok();
                    Ok(PushExecution {
                        outcome: PushOutcome::LocalSave,
                        external_id: None,
                        cause: None,
                        marker_to_delete_after_write: None,
                    })
                }
                CreateRetryOutcome::Rejected(cause) => {
                    std::fs::remove_file(&marker_path).ok();
                    Ok(PushExecution {
                        outcome: PushOutcome::Rejected,
                        external_id: None,
                        cause: Some(cause),
                        marker_to_delete_after_write: None,
                    })
                }
                CreateRetryOutcome::Terminal(detail) => {
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
                    Ok(PushExecution {
                        outcome: PushOutcome::LoudTerminal,
                        external_id: None,
                        cause: None,
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

    let project = args.project.clone().or_else(|| scheme.key.clone());
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
                cause: execution.cause,
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
    if args.dry_run {
        let integration = match effective_nonempty(config, "work.integration") {
            Ok(value) => value,
            Err(error) => return RunOutcome::Failed(error.to_string()),
        };
        return preview_push(&integration, &args.kind, registry);
    }
    match try_run(start, config, templates, args, registry) {
        Ok((path, push)) => RunOutcome::Created { path, push },
        Err(message) => RunOutcome::Failed(message),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::cell::Cell;
    use std::cell::RefCell;

    use tracker::CreatePreview;
    use tracker::FieldResolution;
    use tracker::RemoteTracker;
    use tracker_test_support::RecordingTracker;

    use super::*;

    fn retryable() -> TrackerError {
        TrackerError::Retryable {
            detail: "connection refused".to_owned(),
        }
    }

    fn terminal() -> TrackerError {
        TrackerError::Terminal {
            detail: "response lost after send".to_owned(),
        }
    }

    fn rejected() -> TrackerError {
        TrackerError::Rejected {
            detail: "jira create: the body has a table".to_owned(),
        }
    }

    #[test]
    fn the_push_duplicate_check_sees_external_ids_in_drafts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let drafts = dir.path().join("drafts");
        std::fs::create_dir_all(&drafts).expect("drafts dir");
        std::fs::write(
            drafts.join("draft-k7mq3x-title.md"),
            "---\nid: \"draft-k7mq3x\"\nexternal_id: \"ENG-42\"\n---\n",
        )
        .expect("write draft");

        let carries = |key: &str| {
            super::corpus_carries_external_id(
                dir.path(),
                &ExternalId::new(key.to_owned()),
            )
        };

        assert!(carries("ENG-42"));
        assert!(carries("eng-42"));
        assert!(!carries("ENG-43"));
    }

    #[test]
    fn a_retryable_create_is_retried_once_then_succeeds() {
        let calls = Cell::new(0u8);
        let outcome = drive_create_retry(|| {
            let seen = calls.get();
            calls.set(seen + 1);
            if seen == 0 {
                Err(retryable())
            } else {
                Ok(ExternalId::new("ENG-1".to_owned()))
            }
        });
        assert_eq!(calls.get(), 2);
        assert!(matches!(
            outcome,
            CreateRetryOutcome::Created(id) if id.as_str() == "ENG-1"
        ));
    }

    #[test]
    fn two_retryable_creates_exhaust_and_save_locally() {
        let calls = Cell::new(0u8);
        let outcome = drive_create_retry(|| {
            calls.set(calls.get() + 1);
            Err::<ExternalId, _>(retryable())
        });
        assert_eq!(calls.get(), 2, "the retry is bounded to one re-attempt");
        assert!(matches!(outcome, CreateRetryOutcome::Exhausted));
    }

    #[test]
    fn a_terminal_create_is_never_retried() {
        let calls = Cell::new(0u8);
        let outcome = drive_create_retry(|| {
            calls.set(calls.get() + 1);
            Err::<ExternalId, _>(terminal())
        });
        assert_eq!(calls.get(), 1, "a terminal failure is never retried");
        assert!(matches!(outcome, CreateRetryOutcome::Terminal(_)));
    }

    #[test]
    fn a_rejected_create_is_never_retried() {
        let calls = Cell::new(0u8);
        let outcome = drive_create_retry(|| {
            calls.set(calls.get() + 1);
            Err::<ExternalId, _>(rejected())
        });
        assert_eq!(calls.get(), 1, "a retry would be rejected identically");
        assert!(matches!(
            outcome,
            CreateRetryOutcome::Rejected(cause)
                if cause == "jira create: the body has a table"
        ));
    }

    #[test]
    fn a_first_attempt_success_is_not_retried() {
        let calls = Cell::new(0u8);
        let outcome = drive_create_retry(|| {
            calls.set(calls.get() + 1);
            Ok(ExternalId::new("ENG-2".to_owned()))
        });
        assert_eq!(calls.get(), 1);
        assert!(matches!(outcome, CreateRetryOutcome::Created(_)));
    }

    #[test]
    fn jira_preview_renders_five_tab_fields_with_their_sources() {
        let preview = CreatePreview {
            project: FieldResolution::Resolved("PROJ".to_owned()),
            issue_type: FieldResolution::Resolved("bug".to_owned()),
        };
        assert_eq!(
            render_create_preview("jira", &preview),
            "jira\tbug\tconfigured\tPROJ\tconfigured"
        );
    }

    #[test]
    fn jira_preview_marks_an_unresolvable_project() {
        let preview = CreatePreview {
            project: FieldResolution::Unresolvable("GONE".to_owned()),
            issue_type: FieldResolution::Unset,
        };
        assert_eq!(
            render_create_preview("jira", &preview),
            "jira\t\tdefault\tGONE\tunresolvable"
        );
    }

    #[test]
    fn linear_preview_has_no_user_resolvable_fields() {
        let preview = CreatePreview {
            project: FieldResolution::Unset,
            issue_type: FieldResolution::Unset,
        };
        assert_eq!(
            render_create_preview("linear", &preview),
            "linear\t(no user-resolvable type/project fields)"
        );
    }

    struct FixedRegistry(RefCell<Option<Box<dyn RemoteTracker>>>);

    impl FixedRegistry {
        fn holding(tracker: impl RemoteTracker + 'static) -> Self {
            Self(RefCell::new(Some(Box::new(tracker))))
        }
    }

    impl TrackerRegistry for FixedRegistry {
        fn resolve(
            &self,
            _name: &str,
        ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
            Ok(self.0.borrow_mut().take().expect("resolved more than once"))
        }
    }

    #[test]
    fn a_transport_failure_previews_as_retryable_not_a_block() {
        let registry = FixedRegistry::holding(
            RecordingTracker::holding(Vec::new()).failing_preview(retryable()),
        );
        match preview_push("jira", "bug", &registry) {
            RunOutcome::PreviewFailed { code, .. } => {
                assert_eq!(code, exit_codes::RETRYABLE);
            }
            _ => {
                panic!("a transport failure must preview as PreviewFailed(70)")
            }
        }
    }

    #[test]
    fn an_unresolvable_project_previews_at_exit_zero_with_the_marker() {
        let registry = FixedRegistry::holding(
            RecordingTracker::holding(Vec::new()).previewing(CreatePreview {
                project: FieldResolution::Unresolvable("GONE".to_owned()),
                issue_type: FieldResolution::Resolved("bug".to_owned()),
            }),
        );
        match preview_push("jira", "bug", &registry) {
            RunOutcome::Previewed(line) => {
                assert!(line.ends_with("GONE\tunresolvable"), "{line}");
            }
            _ => panic!("an unresolvable project must still preview at exit 0"),
        }
    }

    struct FakeConfig(std::collections::HashMap<String, String>);

    impl ConfigAccess for FakeConfig {
        fn get(
            &self,
            key: &::config::Key,
            _level: Option<::config::Level>,
        ) -> Result<::config::Resolved, ::config::ConfigError> {
            Ok(self.0.get(&key.to_string()).map_or(
                ::config::Resolved::Absent,
                |value| {
                    ::config::Resolved::Found(::config::Value::Scalar(
                        ::config::Scalar::String(value.clone()),
                    ))
                },
            ))
        }

        fn set(
            &self,
            _key: &::config::Key,
            _value: &str,
            _level: ::config::Level,
        ) -> Result<(), ::config::ConfigError> {
            unreachable!("create never writes config")
        }
    }

    struct PluginWorkItemTemplate;

    impl ReadTemplate for PluginWorkItemTemplate {
        fn resolve_template(
            &self,
            _name: &str,
            _config_path: Option<&str>,
            _templates_dir: &str,
        ) -> Result<Option<::config::ResolvedTemplate>, ::config::ConfigError>
        {
            self.plugin_default("work-item")
        }

        fn template_names(&self) -> Result<Vec<String>, ::config::ConfigError> {
            Ok(vec!["work-item".to_owned()])
        }

        fn plugin_default(
            &self,
            _name: &str,
        ) -> Result<Option<::config::ResolvedTemplate>, ::config::ConfigError>
        {
            Ok(Some(::config::ResolvedTemplate {
                source: ::config::TemplateSource::PluginDefault,
                abs_path: "templates/work-item.md".to_owned(),
                display_path: "templates/work-item.md".to_owned(),
                content: include_str!("../../../templates/work-item.md")
                    .to_owned(),
                warning: None,
            }))
        }
    }

    struct PushingRepo {
        root: tempfile::TempDir,
        config: FakeConfig,
    }

    impl PushingRepo {
        fn new() -> Self {
            let root = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
            let config = FakeConfig(
                [
                    ("work.integration", "jira"),
                    ("paths.integrations", "integrations"),
                ]
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
            );
            Self { root, config }
        }

        fn create(&self, tracker: RecordingTracker) -> RunOutcome {
            let args = CreateArgs {
                title: "A tabled idea".to_owned(),
                kind: "task".to_owned(),
                priority: "low".to_owned(),
                status: "draft".to_owned(),
                parent: None,
                tags: Vec::new(),
                blocks: Vec::new(),
                blocked_by: Vec::new(),
                derived_from: Vec::new(),
                relates_to: Vec::new(),
                source: None,
                project: None,
                author: Some("A Tester".to_owned()),
                producer: "create-work-item".to_owned(),
                body_file: None,
                push: true,
                dry_run: false,
            };
            run(
                self.root.path(),
                &self.config,
                &PluginWorkItemTemplate,
                &args,
                &FixedRegistry::holding(tracker),
            )
        }

        fn marker(&self) -> PathBuf {
            work_adapters::sync::pending_push::path(
                &self.root.path().join("integrations"),
                "jira",
                "a-tabled-idea",
            )
        }
    }

    #[test]
    fn a_rejected_push_is_rejected_exits_75_and_leaves_no_marker() {
        let repo = PushingRepo::new();

        let outcome = repo.create(
            RecordingTracker::holding(Vec::new()).failing_create(rejected()),
        );

        let RunOutcome::Created {
            push: Some(report), ..
        } = outcome
        else {
            panic!("a rejected push still creates the local item");
        };
        assert_eq!(report.outcome, PushOutcome::Rejected);
        assert_eq!(report.outcome.exit_code(), exit_codes::REJECTED);
        assert_eq!(
            report.cause.as_deref(),
            Some("jira create: the body has a table")
        );
        assert!(!repo.marker().exists(), "no issue can exist");
    }

    #[test]
    fn a_rejected_legacy_create_still_writes_the_unsynced_file_and_prints_its_path(
    ) {
        let repo = PushingRepo::new();

        let outcome = repo.create(
            RecordingTracker::holding(Vec::new()).failing_create(rejected()),
        );

        let RunOutcome::Created { path, .. } = outcome else {
            panic!("a rejected push still creates the local item");
        };
        let written = std::fs::read_to_string(&path).expect("the file exists");
        assert!(
            path.ends_with("meta/work/0001-a-tabled-idea.md"),
            "{}",
            path.display()
        );
        assert!(!written.contains("external_id"), "{written}");
    }
}

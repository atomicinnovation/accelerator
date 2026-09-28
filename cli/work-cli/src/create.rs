//! Adapter/binary wiring for `work create`: places the new item, composes
//! its frontmatter via `work::create::compose_frontmatter`, and dispatches to
//! one of three creation strategies: a locally numbered item (optionally
//! pushed), a draft, or under `{tracker}` a draft promoted onto the issue the
//! tracker creates for it.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use ::config::ReadTemplate;
use corpus::store::ExclusiveCreate;
use corpus::store::RemoveFile;
use corpus::AtomicWrite;
use corpus::FilenameTimestampFormat;
use corpus::IdOwnership;
use corpus_adapters::compile_scan_regex;
use corpus_adapters::metadata::derive_at;
use corpus_adapters::metadata::VcsBackedRepoFactsProbe;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::RealFs;
use corpus_adapters::RegexScanner;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use store::lock::acquire;
use store::lock::LockGuard;
use store::lock::LockOptions;
use tracker::CreatePreview;
use tracker::ExternalId;
use tracker::FieldResolution;
use tracker::RemoteTracker;
use vcs_adapters::library::InProcessProbe;
use work::create::assert_matches_template_schema;
use work::create::compose_frontmatter;
use work::create::resolve_author;
use work::create::CreateInputs;
use work::create::FieldValue;
use work::create::TypedLinkage;
use work::draft_id::mint_draft_id;
use work::draft_id::DraftId;
use work::draft_id::SuffixDraws;
use work::identity::linker_of;
use work::next_number::allocate;
use work::next_number::AllocationError;
use work::promotion::IntendedBaseline;
use work::promotion::NotPromoted;
use work::promotion::Promotion;
use work::promotion::PromotionMode;
use work::promotion::PromotionStage;
use work::promotion::ReadBack;
use work::promotion::RemoteHash;
use work::promotion::ID_PLACEHOLDER as PROMOTION_PLACEHOLDER;
use work::resolve::DirectoryLister;
use work::retirement::RetirementFailure;
use work::retirement::RetirementRefusal;
use work::sync::MarkerState;
use work::sync::PendingPush;
use work::sync::PushOutcome;
use work::sync::PushPrecondition;
use work::sync::RefusalReason;
use work::sync::RequestFingerprint;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles;
use work_adapters::author::VcsBackedIdentityProbe;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::filesystem::drafts_dir;
use work_adapters::filesystem::FilesystemLister;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::promotion::held_key;
use work_adapters::promotion::promote;
use work_adapters::promotion::PromotionPorts;
use work_adapters::promotion_records::FilePromotionRecords;
use work_adapters::promotion_records::PromotionRecords;
use work_adapters::remote_create::send_create;
use work_adapters::remote_create::CreateRequest;
use work_adapters::remote_create::RemoteCreate;
use work_adapters::sync::baseline;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::created_baseline::record_created_baseline;
use work_adapters::sync::digest;
use work_adapters::sync::pending_push;
use work_adapters::sync::pending_push::Marker;

use crate::config::configured_override;
use crate::config::effective_nonempty;
use crate::config::resolve_scheme;
use crate::config::resolve_work_dir;
use crate::config::templates_dir;
use crate::exit_codes;
use crate::identity_workspace::IdentityWorkspace;
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
    /// What stderr tells the user beside the keyword: a rejected request's
    /// cause, a blocking item and its remedy, or the paths to restore.
    pub cause: Option<String>,
}

pub enum RunOutcome {
    /// `path` is a file that exists, or `None` when an issue was created but
    /// no local file carries it.
    Created {
        path: Option<PathBuf>,
        push: Option<PushReport>,
    },
    Previewed(String),
    PreviewFailed {
        message: String,
        code: u8,
    },
    /// A create of the same content is already pending; nothing was sent.
    Pending(String),
    Failed(String),
}

/// The file store every creation strategy writes items through.
pub trait CreationStore: AtomicWrite + ExclusiveCreate + RemoveFile {}

impl<T: AtomicWrite + ExclusiveCreate + RemoveFile> CreationStore for T {}

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

pub const fn dispatch_code_for_selection_error(error: &SelectionError) -> u8 {
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

fn attempted_at_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

/// Everything a creation strategy needs that does not depend on the ID the
/// new item takes.
struct CreationContext<'a> {
    args: &'a CreateArgs,
    config: &'a dyn ConfigAccess,
    scheme: corpus::WorkItemIdScheme,
    root: PathBuf,
    work_dir: PathBuf,
    template: ::config::ResolvedTemplate,
    author: String,
    date: String,
}

impl CreationContext<'_> {
    fn body(&self, id: &str) -> Result<String, String> {
        resolve_body(self.args, &self.template, id)
    }

    fn content(
        &self,
        id: &str,
        external_id: Option<&str>,
    ) -> Result<String, String> {
        let args = self.args;
        let inputs = CreateInputs {
            id,
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
            author: &self.author,
            producer: &args.producer,
            date: &self.date,
            external_id,
        };
        Ok(format!(
            "{}{}",
            render_frontmatter(&inputs)?,
            self.body(id)?
        ))
    }

    fn slug(&self) -> String {
        slugify(&self.args.title)
    }

    fn lock(&self) -> Result<LockGuard, String> {
        acquire(&self.work_dir.join(LOCK_FILE_NAME), LockOptions::default())
            .map_err(|error| {
                format!(
                    "could not acquire the work-item creation lock: {error}"
                )
            })
    }

    fn pending_push_location(&self) -> Result<(PathBuf, String), String> {
        let integration = effective_nonempty(self.config, "work.integration")
            .map_err(|error| error.to_string())?;
        let integrations_root =
            crate::sync::integrations_dir(self.config, &self.root)
                .map_err(|error| error.to_string())?;
        Ok((integrations_root, integration))
    }
}

/// What a creation strategy left on disk, and what became of its push.
struct CreationOutcome {
    path: Option<PathBuf>,
    push: Option<PushReport>,
}

impl CreationOutcome {
    const fn unpushed(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            push: None,
        }
    }
}

fn pushed(
    path: Option<PathBuf>,
    outcome: PushOutcome,
    key: Option<&ExternalId>,
    cause: Option<String>,
) -> CreationOutcome {
    CreationOutcome {
        path,
        push: Some(PushReport {
            outcome,
            external_id: key.map(|key| key.as_str().to_owned()),
            cause,
        }),
    }
}

fn refuse_to_overwrite(target: &Path) -> Result<(), String> {
    if target.exists() {
        return Err(format!(
            "refusing to overwrite an existing file: {}",
            target.display()
        ));
    }
    Ok(())
}

fn write_new(
    store: &dyn CreationStore,
    target: &Path,
    content: &str,
) -> Result<(), String> {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).map_err(|error| {
        format!("could not create {}: {error}", dir.display())
    })?;
    refuse_to_overwrite(target)?;
    AtomicWrite::write(store, target, content.as_bytes())
        .map_err(|error| error.to_string())
}

/// Writes the draft under the create lock, minting an ID no item holds.
fn write_draft(
    context: &CreationContext<'_>,
    store: &dyn CreationStore,
    draws: &mut dyn SuffixDraws,
) -> Result<(DraftId, PathBuf), String> {
    let _guard = context.lock()?;
    let files = FilesystemWorkItemFiles::new(&context.work_dir)
        .files()
        .map_err(|error| error.to_string())?;
    let draft = mint_draft_id(draws, &identities(&files))
        .map_err(|error| error.to_string())?;
    let target = drafts_dir(&context.work_dir).join(format!(
        "{}-{}.md",
        draft.as_str(),
        context.slug()
    ));
    write_new(store, &target, &context.content(draft.as_str(), None)?)?;
    Ok((draft, target))
}

fn save_draft(
    context: &CreationContext<'_>,
    store: &dyn CreationStore,
    draws: &mut dyn SuffixDraws,
) -> Result<CreationOutcome, String> {
    let (_, path) = write_draft(context, store, draws)?;
    Ok(CreationOutcome::unpushed(path))
}

/// A legacy push's result: the key it obtained, if any, the tracker it
/// reached, and the marker to spend once the item carries the key.
struct LegacyPush {
    outcome: PushOutcome,
    key: Option<ExternalId>,
    cause: Option<String>,
    tracker: Option<Box<dyn RemoteTracker>>,
    marker: Option<PathBuf>,
}

impl LegacyPush {
    const fn without_issue(
        outcome: PushOutcome,
        cause: Option<String>,
    ) -> Self {
        Self {
            outcome,
            key: None,
            cause,
            tracker: None,
            marker: None,
        }
    }
}

fn write_marker(
    store: &FileCorpusStore,
    path: &Path,
    marker: &PendingPush,
) -> Result<(), String> {
    AtomicWrite::write(store, path, pending_push::render(marker).as_bytes())
        .map_err(|error| error.to_string())
}

/// Pushes a locally numbered item through its slug-named `pending_push`
/// marker, whose recovery rules older binaries share.
fn push_legacy_item(
    context: &CreationContext<'_>,
    body: &str,
    registry: &dyn TrackerRegistry,
) -> Result<LegacyPush, String> {
    let args = context.args;
    let (integrations_root, integration) = context.pending_push_location()?;
    let marker_path =
        pending_push::path(&integrations_root, &integration, &context.slug());
    let marker_store = FileCorpusStore::new(
        marker_path.parent().unwrap_or(&integrations_root),
    );
    let marker_content = std::fs::read_to_string(&marker_path).ok();
    let parsed = pending_push::read(marker_content.as_deref());
    let digest = pending_push::request_digest(&args.title, body, &args.kind);
    let marker_state = match &parsed {
        Err(_) => MarkerState::Unreadable,
        Ok(None) => MarkerState::Absent,
        Ok(Some(marker)) => MarkerState::Present(marker),
    };
    let corpus_carries =
        |id: &ExternalId| corpus_carries_external_id(&context.work_dir, id);
    let precondition =
        work::sync::push_precondition(&marker_state, &digest, &corpus_carries);

    let tracker_for_read_back = || registry.resolve(&integration).ok();
    match precondition {
        PushPrecondition::Refuse(reason) => Err(refusal_message(
            reason,
            &marker_path,
            parsed.ok().flatten().as_ref(),
        )),
        PushPrecondition::ReuseId(key) => Ok(LegacyPush {
            outcome: PushOutcome::WriteOnce,
            key: Some(key),
            cause: None,
            tracker: tracker_for_read_back(),
            marker: Some(marker_path),
        }),
        PushPrecondition::Proceed => {
            pending_push::prepare_dir(&integrations_root, &integration)
                .map_err(|error| error.to_string())?;
            let tracker = match registry.resolve(&integration) {
                Ok(tracker) => tracker,
                Err(error) => {
                    let outcome = work::sync::push_decide(
                        dispatch_code_for_selection_error(&error),
                        1,
                        false,
                    );
                    return Ok(LegacyPush::without_issue(outcome, None));
                }
            };
            let fingerprint = RequestFingerprint {
                title: args.title.clone(),
                digest,
                attempted_at: attempted_at_epoch(),
                failure: None,
            };
            write_marker(
                &marker_store,
                &marker_path,
                &PendingPush::Attempted {
                    request: fingerprint.clone(),
                },
            )?;
            send_legacy_create(
                &CreateRequest {
                    title: &args.title,
                    body,
                    kind: &args.kind,
                },
                tracker,
                &marker_store,
                marker_path,
                fingerprint,
            )
        }
    }
}

/// Sends the create and records its outcome in the marker: the key once
/// created, the failure when the outcome is unknown, and nothing once no
/// issue can exist.
fn send_legacy_create(
    request: &CreateRequest<'_>,
    tracker: Box<dyn RemoteTracker>,
    marker_store: &FileCorpusStore,
    marker_path: PathBuf,
    fingerprint: RequestFingerprint,
) -> Result<LegacyPush, String> {
    match send_create(request, tracker.as_ref()) {
        RemoteCreate::Created(key) => {
            write_marker(
                marker_store,
                &marker_path,
                &PendingPush::Created {
                    request: fingerprint,
                    external_id: key.clone(),
                },
            )?;
            Ok(LegacyPush {
                outcome: PushOutcome::WriteOnce,
                key: Some(key),
                cause: None,
                tracker: Some(tracker),
                marker: Some(marker_path),
            })
        }
        RemoteCreate::TrackerUnreachable => {
            std::fs::remove_file(&marker_path).ok();
            Ok(LegacyPush::without_issue(PushOutcome::LocalSave, None))
        }
        RemoteCreate::Rejected { detail } => {
            std::fs::remove_file(&marker_path).ok();
            Ok(LegacyPush::without_issue(
                PushOutcome::Rejected,
                Some(detail),
            ))
        }
        RemoteCreate::OutcomeUnknown { detail } => {
            write_marker(
                marker_store,
                &marker_path,
                &PendingPush::Attempted {
                    request: RequestFingerprint {
                        failure: Some(detail),
                        ..fingerprint
                    },
                },
            )?;
            Ok(LegacyPush::without_issue(PushOutcome::LoudTerminal, None))
        }
    }
}

fn read_back_of(tracker: &dyn RemoteTracker, key: &ExternalId) -> RemoteHash {
    tracker.show(key).map_or(RemoteHash::Unknown, |issue| {
        RemoteHash::Known(ReadBack {
            hash: digest::remote_body(&issue.body),
            updated: issue.updated,
        })
    })
}

fn baseline_path(context: &CreationContext<'_>) -> Result<PathBuf, String> {
    let (integrations_root, integration) = context.pending_push_location()?;
    Ok(baseline::path(&integrations_root, &integration))
}

/// Records what the item and its new issue look like now, so the next sync
/// classifies the pair as synced.
fn record_baseline_after_create(
    context: &CreationContext<'_>,
    item_id: &str,
    written: &str,
    remote: RemoteHash,
) -> Result<(), String> {
    let path = baseline_path(context)?;
    let dir = path.parent().unwrap_or(&context.root).to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let writer = FileCorpusStore::new(&dir);
    let store = BaselineStore::new(path, &RealFs, &writer);
    record_created_baseline(
        item_id,
        &IntendedBaseline {
            remote_hash: remote,
            local_hash: digest::local(written)
                .map_err(|error| error.to_string())?,
        },
        &store,
        attempted_at_epoch(),
    )
    .map_err(|error| error.to_string())
}

/// Writes a locally numbered item, pushing it first when asked. The write
/// is retried once, because once an issue exists a failed write leaves it
/// carried by no item.
fn create_local_item(
    context: &CreationContext<'_>,
    store: &dyn CreationStore,
    registry: &dyn TrackerRegistry,
) -> Result<CreationOutcome, String> {
    let _guard = context.lock()?;
    let project = context
        .args
        .project
        .clone()
        .or_else(|| context.scheme.key.clone());
    let id =
        allocate_id(&context.scheme, &context.work_dir, project.as_deref())?;
    let target = context.work_dir.join(format!("{id}-{}.md", context.slug()));
    refuse_to_overwrite(&target)?;

    if !context.args.push {
        write_new(store, &target, &context.content(&id, None)?)?;
        return Ok(CreationOutcome::unpushed(target));
    }
    let push = push_legacy_item(context, &context.body(&id)?, registry)?;
    let item =
        context.content(&id, push.key.as_ref().map(ExternalId::as_str))?;
    let written = write_new(store, &target, &item)
        .or_else(|_| write_new(store, &target, &item));
    let Some(key) = push.key else {
        written?;
        return Ok(pushed(Some(target), push.outcome, None, push.cause));
    };
    if written.is_err() {
        return Ok(pushed(
            None,
            PushOutcome::CreatedUnwritten,
            Some(&key),
            None,
        ));
    }
    let remote = push
        .tracker
        .as_deref()
        .map_or(RemoteHash::Unknown, |tracker| read_back_of(tracker, &key));
    record_baseline_after_create(context, &id, &item, remote)?;
    if let Some(marker) = push.marker {
        std::fs::remove_file(marker).ok();
    }
    Ok(pushed(
        Some(target),
        PushOutcome::WriteOnce,
        Some(&key),
        None,
    ))
}

/// How an earlier create of the same content is still pending.
enum PendingCreate {
    Record {
        draft: DraftId,
        stage: PromotionStage,
    },
    UnreadableRecord(PathBuf),
    LegacyMarker(PathBuf),
    Draft(PathBuf),
}

impl PendingCreate {
    fn message(&self, title: &str) -> String {
        match self {
            Self::Record {
                draft,
                stage: PromotionStage::Attempted,
            } => format!(
                "E_PUSH_PENDING: {draft} records a create of this content whose \
                 outcome is unknown; if the tracker has its issue run `work \
                 promote {draft} --adopt <KEY>`, otherwise `work promote \
                 {draft} --create`",
                draft = draft.as_str()
            ),
            Self::Record { draft, .. } => format!(
                "E_PUSH_PENDING: {draft} is already being promoted onto its \
                 issue; run `work promote {draft}` or `work sync` to finish it",
                draft = draft.as_str()
            ),
            Self::UnreadableRecord(path) => format!(
                "E_PUSH_PENDING: {} could not be read and may record a create \
                 of this content; inspect it before creating again",
                path.display()
            ),
            Self::LegacyMarker(path) => format!(
                "E_PUSH_PENDING: {} records an earlier create titled '{title}'; \
                 check the tracker for an issue titled '{title}' first, then \
                 inspect or remove the named marker",
                path.display()
            ),
            Self::Draft(path) => format!(
                "E_DRAFT_EXISTS: {} already holds this content; run `work \
                 promote <draft>` or `work sync`",
                path.display()
            ),
        }
    }
}

/// The draft's own content digest: the same content digests alike
/// whatever draft ID each copy was given.
fn draft_content_digest(content: &str, id: &str) -> Option<String> {
    let (frontmatter, body) =
        digest::split_frontmatter_and_body(content).ok()?;
    let field = |key| work::show::read_field_raw(&frontmatter, key);
    Some(pending_push::content_digest(
        &field("title")?,
        &body,
        &field("kind").unwrap_or_default(),
        id,
    ))
}

/// Finds an earlier create of the same content still pending, so a rerun
/// after a tracker error, a kill or an unwritten create never sends a
/// second create.
fn pending_create(
    context: &CreationContext<'_>,
    records: &dyn PromotionRecords,
    integrations_root: &Path,
    integration: &str,
) -> Result<Option<PendingCreate>, String> {
    let (_, placeholder_body) = digest::split_frontmatter_and_body(
        &context.content(PROMOTION_PLACEHOLDER, None)?,
    )
    .map_err(|error| error.to_string())?;
    let wanted = pending_push::content_digest(
        &context.args.title,
        &placeholder_body,
        &context.args.kind,
        PROMOTION_PLACEHOLDER,
    );
    let files = FilesystemWorkItemFiles::new(&context.work_dir)
        .files()
        .map_err(|error| error.to_string())?;
    let drafts: Vec<(DraftId, &Path, &str)> = files
        .iter()
        .filter_map(|file| {
            let identity = work::work_item_files::identity_of(file)?;
            let draft = DraftId::parse(&identity.id)?;
            Some((draft, file.path.as_path(), file.content.as_str()))
        })
        .collect();
    let draft_exists =
        |wanted: &DraftId| drafts.iter().any(|(draft, _, _)| draft == wanted);

    for entry in records.outstanding().map_err(|error| error.to_string())? {
        match entry {
            Ok(record) if record.content_digest == wanted => {
                return Ok(Some(PendingCreate::Record {
                    draft: record.draft_id,
                    stage: record.stage,
                }));
            }
            Ok(_) => {}
            Err(unreadable) => {
                let names_live_draft = unreadable
                    .path
                    .file_stem()
                    .and_then(std::ffi::OsStr::to_str)
                    .and_then(DraftId::parse)
                    .is_some_and(|draft| draft_exists(&draft));
                if names_live_draft {
                    return Ok(Some(PendingCreate::UnreadableRecord(
                        unreadable.path,
                    )));
                }
            }
        }
    }
    let markers = pending_push::outstanding(integrations_root, integration)
        .map_err(|error| error.to_string())?;
    for (path, marker) in markers.into_iter().flatten() {
        if let Marker::Legacy(
            PendingPush::Attempted { request }
            | PendingPush::Created { request, .. },
        ) = marker
        {
            if request.title == context.args.title {
                return Ok(Some(PendingCreate::LegacyMarker(path)));
            }
        }
    }
    Ok(drafts
        .iter()
        .find(|(draft, _, content)| {
            draft_content_digest(content, draft.as_str()).as_deref()
                == Some(wanted.as_str())
        })
        .map(|(_, path, _)| PendingCreate::Draft(path.to_path_buf())))
}

pub fn blocked_remedy(
    refusal: &RetirementRefusal,
    key: &ExternalId,
    draft: &DraftId,
) -> String {
    let draft = draft.as_str();
    match refusal {
        RetirementRefusal::IdTaken { holder, .. }
        | RetirementRefusal::KeyLinked { holder } => format!(
            "{holder} already carries {key}; if it is the same issue, delete \
             the draft or merge it into {holder}, otherwise fix {holder}, \
             then `work promote {draft}`",
            holder = holder.display()
        ),
        RetirementRefusal::TargetExists(path) => format!(
            "move {} aside, then `work promote {draft}`",
            path.display()
        ),
        RetirementRefusal::ItemNotFound(_) => {
            format!("the draft {draft} is gone; the issue {key} is unlinked")
        }
    }
}

fn existing(paths: &[&Path]) -> Option<PathBuf> {
    paths
        .iter()
        .find(|path| path.exists())
        .map(|path| path.to_path_buf())
}

/// A retirement that could not restore every path: the path line names
/// whichever of the draft and the target exists, and stderr the paths to
/// restore.
fn incomplete_outcome(
    failure: &RetirementFailure,
    context: &CreationContext<'_>,
    draft: &DraftId,
    draft_path: &Path,
    key: Option<&ExternalId>,
) -> CreationOutcome {
    let target = key.map(|key| {
        context
            .work_dir
            .join(format!("{}-{}.md", key.as_str(), context.slug()))
    });
    let mut candidates = vec![draft_path];
    if let Some(target) = &target {
        candidates.push(target);
    }
    let cause = key.map(|key| {
        let retirement = work::retirement::Retirement {
            old_id: draft.as_str(),
            new_id: key.as_str(),
            new_external_id: Some(key.as_str()),
        };
        failure.message(
            &retirement,
            &context
                .root
                .join(crate::sync::STATE_DIR)
                .join(retirement.recovery_dir()),
        )
    });
    pushed(
        existing(&candidates),
        PushOutcome::RetirementIncomplete,
        key,
        cause,
    )
}

/// The create outcome a draft left unpromoted reports.
pub const fn create_outcome_of(reason: &NotPromoted) -> PushOutcome {
    match reason {
        NotPromoted::TrackerUnreachable
        | NotPromoted::RecordUnwritable { key: None, .. } => {
            PushOutcome::LocalSave
        }
        NotPromoted::CreateOutcomeUnknown
        | NotPromoted::EarlierAttemptUnconfirmed => PushOutcome::LoudTerminal,
        NotPromoted::RequestRejected { .. } => PushOutcome::Rejected,
        NotPromoted::Refused(RetirementRefusal::ItemNotFound(_))
        | NotPromoted::ReadBackFailed(_)
        | NotPromoted::RetirementFailed(RetirementFailure::RolledBack {
            ..
        })
        | NotPromoted::RecordUnwritable { key: Some(_), .. } => {
            PushOutcome::CreatedUnwritten
        }
        NotPromoted::Refused(_)
        | NotPromoted::AdoptedIssueMissing(_)
        | NotPromoted::AdoptConflictsWithRecordedKey { .. } => {
            PushOutcome::CreatedBlocked
        }
        NotPromoted::RetirementFailed(
            RetirementFailure::RestoreIncomplete { .. },
        ) => PushOutcome::RetirementIncomplete,
    }
}

/// Maps a promotion's result onto the create outcome the skills read.
fn promotion_outcome(
    result: Result<Promotion, NotPromoted>,
    context: &CreationContext<'_>,
    draft: &DraftId,
    draft_path: PathBuf,
    records: &dyn PromotionRecords,
) -> CreationOutcome {
    let reason = match result {
        Ok(Promotion::Completed(key, _) | Promotion::AlreadyDone(key)) => {
            let path = FilesystemWorkItemFiles::new(&context.work_dir)
                .files()
                .ok()
                .and_then(|files| {
                    identities(&files)
                        .into_iter()
                        .find(|item| item.id.eq_ignore_ascii_case(key.as_str()))
                })
                .map_or_else(
                    || {
                        context.work_dir.join(format!(
                            "{}-{}.md",
                            key.as_str(),
                            context.slug()
                        ))
                    },
                    |item| item.path,
                );
            return pushed(
                Some(path),
                PushOutcome::WriteOnce,
                Some(&key),
                None,
            );
        }
        Err(reason) => reason,
    };
    let key = held_key(&reason, draft, records);
    let outcome = create_outcome_of(&reason);
    let shown_key = match outcome {
        PushOutcome::CreatedUnwritten | PushOutcome::CreatedBlocked => {
            key.as_ref()
        }
        _ => None,
    };
    match reason {
        NotPromoted::RetirementFailed(
            failure @ RetirementFailure::RestoreIncomplete { .. },
        ) => incomplete_outcome(
            &failure,
            context,
            draft,
            &draft_path,
            key.as_ref(),
        ),
        NotPromoted::Refused(RetirementRefusal::ItemNotFound(_)) => {
            pushed(None, outcome, shown_key, None)
        }
        NotPromoted::Refused(refusal) => {
            let remedy =
                key.as_ref().map(|key| blocked_remedy(&refusal, key, draft));
            pushed(Some(draft_path), outcome, shown_key, remedy)
        }
        NotPromoted::RequestRejected { detail }
        | NotPromoted::RecordUnwritable { detail, .. } => {
            pushed(Some(draft_path), outcome, shown_key, Some(detail))
        }
        _ => pushed(Some(draft_path), outcome, shown_key, None),
    }
}

/// Under `{tracker}` with `--push`: writes a draft, then promotes it, so an
/// interrupted create always leaves a draft its record belongs to.
fn create_tracker_keyed_item(
    context: &CreationContext<'_>,
    store: &dyn CreationStore,
    draws: &mut dyn SuffixDraws,
    registry: &dyn TrackerRegistry,
) -> Result<CreationOutcome, CreateFailure> {
    let (integrations_root, integration) = context.pending_push_location()?;
    std::fs::create_dir_all(&integrations_root)
        .map_err(|error| error.to_string())?;
    let records_store = FileCorpusStore::new(&integrations_root);
    let records = FilePromotionRecords::new(
        &integrations_root,
        &integration,
        &records_store,
    );
    if let Some(pending) =
        pending_create(context, &records, &integrations_root, &integration)?
    {
        return Err(CreateFailure::Pending(
            pending.message(&context.args.title),
        ));
    }
    let (draft, draft_path) = write_draft(context, store, draws)?;
    let Ok(tracker) = registry.resolve(&integration) else {
        return Ok(pushed(
            Some(draft_path),
            PushOutcome::LocalSave,
            None,
            None,
        ));
    };

    let workspace = IdentityWorkspace::open(
        context.config,
        &context.root,
        &context.work_dir,
        &integrations_root,
        &integration,
    )?;
    let baseline = workspace.baseline(store);
    let retirement = workspace.retirement_ports(store, &baseline);
    let result = promote(
        &draft,
        &PromotionMode::Standard,
        &PromotionPorts {
            tracker: tracker.as_ref(),
            retirement: &retirement,
            records: &records,
        },
    );
    Ok(promotion_outcome(
        result, context, &draft, draft_path, &records,
    ))
}

/// Why a create wrote nothing: an earlier create of the same content is
/// still pending, or something failed.
enum CreateFailure {
    Pending(String),
    Failed(String),
}

impl From<String> for CreateFailure {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

fn creation_context<'a>(
    start: &Path,
    config: &'a dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &'a CreateArgs,
) -> Result<CreationContext<'a>, String> {
    let scheme = resolve_scheme(config).map_err(|error| error.to_string())?;
    let root = config_adapters::FileConfigStore::discover_root(start);
    let work_dir =
        resolve_work_dir(config, &root).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&work_dir).map_err(|error| {
        format!("could not create the work-item directory: {error}")
    })?;
    if scheme.ownership() == IdOwnership::Tracker && args.project.is_some() {
        return Err(allocation_message(
            &AllocationError::ProjectUnused,
            &scheme.id_pattern,
        ));
    }
    let metadata = derive_at(
        &root,
        FilenameTimestampFormat::DateTimeUnderscored,
        &VcsBackedRepoFactsProbe::new(&InProcessProbe),
    )
    .map_err(|error| error.to_string())?;
    let author = resolve_author(
        args.author.as_deref(),
        &VcsBackedIdentityProbe::new(&InProcessProbe),
    )?;
    let template = resolve_and_check_template(config, templates)?;
    Ok(CreationContext {
        args,
        config,
        scheme,
        root,
        work_dir,
        template,
        author,
        date: metadata.datetime_utc,
    })
}

fn try_run(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateArgs,
    seams: &mut Seams<'_>,
) -> Result<CreationOutcome, CreateFailure> {
    let context = creation_context(start, config, templates, args)?;
    let store = (seams.store_at)(&context.root);
    match (context.scheme.ownership(), args.push) {
        (IdOwnership::Local, _) => {
            Ok(create_local_item(&context, store.as_ref(), seams.registry)?)
        }
        (IdOwnership::Tracker, false) => {
            Ok(save_draft(&context, store.as_ref(), seams.draws)?)
        }
        (IdOwnership::Tracker, true) => create_tracker_keyed_item(
            &context,
            store.as_ref(),
            seams.draws,
            seams.registry,
        ),
    }
}

/// What `run` takes from its caller beyond the request: the tracker, the
/// store items are written through, and the source of draft suffixes.
pub struct Seams<'a> {
    pub registry: &'a dyn TrackerRegistry,
    pub store_at: &'a dyn Fn(&Path) -> Box<dyn CreationStore>,
    pub draws: &'a mut dyn SuffixDraws,
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
    let store_at = |root: &Path| -> Box<dyn CreationStore> {
        Box::new(FileCorpusStore::new(root))
    };
    run_with(
        start,
        config,
        templates,
        args,
        &mut Seams {
            registry,
            store_at: &store_at,
            draws: &mut RandomSuffixDraws,
        },
    )
}

#[must_use]
pub fn run_with(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateArgs,
    seams: &mut Seams<'_>,
) -> RunOutcome {
    if args.dry_run {
        let integration = match effective_nonempty(config, "work.integration") {
            Ok(value) => value,
            Err(error) => return RunOutcome::Failed(error.to_string()),
        };
        return preview_push(&integration, &args.kind, seams.registry);
    }
    match try_run(start, config, templates, args, seams) {
        Ok(CreationOutcome { path, push }) => {
            RunOutcome::Created { path, push }
        }
        Err(CreateFailure::Pending(message)) => RunOutcome::Pending(message),
        Err(CreateFailure::Failed(message)) => RunOutcome::Failed(message),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::cell::RefCell;

    use tracker::CreatePreview;
    use tracker::FieldResolution;
    use tracker::RemoteTracker;
    use tracker::TrackerError;
    use tracker_test_support::RecordingTracker;

    use super::*;
    use std::rc::Rc;

    use crate::test_support::StubRegistry;

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
            Self::configured(&[
                ("work.integration", "jira"),
                ("paths.integrations", "integrations"),
            ])
        }

        fn tracker_owned() -> Self {
            Self::configured(&[
                ("work.id_pattern", "{tracker}"),
                ("work.integration", "linear"),
                ("paths.integrations", "integrations"),
            ])
        }

        fn configured(pairs: &[(&str, &str)]) -> Self {
            let root = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
            let config = FakeConfig(
                pairs
                    .iter()
                    .map(|(key, value)| {
                        ((*key).to_owned(), (*value).to_owned())
                    })
                    .collect(),
            );
            Self { root, config }
        }

        fn create(&self, tracker: RecordingTracker) -> RunOutcome {
            self.create_pushing(true, tracker)
        }

        fn create_pushing(
            &self,
            push: bool,
            tracker: RecordingTracker,
        ) -> RunOutcome {
            self.create_with(&Self::args(push), tracker)
        }

        fn args(push: bool) -> CreateArgs {
            CreateArgs {
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
                push,
                dry_run: false,
            }
        }

        fn create_with(
            &self,
            args: &CreateArgs,
            tracker: RecordingTracker,
        ) -> RunOutcome {
            run(
                self.root.path(),
                &self.config,
                &PluginWorkItemTemplate,
                args,
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

        let RunOutcome::Created {
            path: Some(path), ..
        } = outcome
        else {
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

    #[test]
    fn create_without_push_under_tracker_writes_a_draft() {
        let repo = PushingRepo::tracker_owned();

        let outcome =
            repo.create_pushing(false, RecordingTracker::holding(Vec::new()));

        let RunOutcome::Created {
            path: Some(path),
            push: None,
        } = outcome
        else {
            panic!("an offline create under {{tracker}} writes a draft");
        };
        let written = std::fs::read_to_string(&path).expect("the draft exists");
        let identity = work::work_item_files::identity_of(
            &work::work_item_files::WorkItemFile {
                path: path.clone(),
                content: written.clone(),
            },
        )
        .expect("the draft has an id");
        let id = identity.id;
        assert!(
            work::draft_id::DraftId::parse(&id)
                .is_some_and(|draft| draft.as_str() == id),
            "{id}"
        );
        assert_eq!(
            path,
            repo.root
                .path()
                .join("meta/work/drafts")
                .join(format!("{id}-a-tabled-idea.md"))
        );
        assert!(
            written.contains(&format!("\n# {id}: A tabled idea\n")),
            "{written}"
        );
        assert_eq!(identity.external_id, None);
        assert!(!repo.marker().exists(), "no push was attempted");
    }

    #[test]
    fn create_with_project_under_tracker_is_e_pattern_key_unused() {
        let repo = PushingRepo::tracker_owned();
        let args = CreateArgs {
            project: Some("ENG".to_owned()),
            ..PushingRepo::args(false)
        };

        let outcome =
            repo.create_with(&args, RecordingTracker::holding(Vec::new()));

        let RunOutcome::Failed(message) = outcome else {
            panic!("--project names no token under {{tracker}}");
        };
        assert!(message.starts_with("E_PATTERN_KEY_UNUSED: "), "{message}");
    }

    struct Faults {
        inner: FileCorpusStore,
        applies: fn(&Path) -> bool,
        left: std::cell::Cell<usize>,
    }

    impl Faults {
        fn check(&self, path: &Path) -> Result<(), corpus::StoreError> {
            if (self.applies)(path) && self.left.get() > 0 {
                self.left.set(self.left.get() - 1);
                return Err(corpus::StoreError::Io {
                    path: path.display().to_string(),
                    detail: "injected".to_owned(),
                });
            }
            Ok(())
        }
    }

    impl AtomicWrite for Faults {
        fn write(
            &self,
            path: &Path,
            bytes: &[u8],
        ) -> Result<(), corpus::StoreError> {
            self.check(path)?;
            self.inner.write(path, bytes)
        }
    }

    impl ExclusiveCreate for Faults {
        fn create_new(
            &self,
            path: &Path,
            bytes: &[u8],
        ) -> Result<(), corpus::StoreError> {
            self.check(path)?;
            self.inner.create_new(path, bytes)
        }
    }

    impl RemoveFile for Faults {
        fn remove(&self, path: &Path) -> Result<(), corpus::StoreError> {
            self.check(path)?;
            self.inner.remove(path)
        }
    }

    fn in_the_work_directory_itself(path: &Path) -> bool {
        path.parent().is_some_and(|dir| dir.ends_with("meta/work"))
            && path.extension().is_some_and(|extension| extension == "md")
    }

    const KEY: &str = "REC-1";

    /// Which store operations fail, and how many times.
    type Failing = Option<(fn(&Path) -> bool, usize)>;

    impl PushingRepo {
        fn tracker_owned_on(integration: &str) -> Self {
            Self::configured(&[
                ("work.id_pattern", "{tracker}"),
                ("work.integration", integration),
                ("paths.integrations", "integrations"),
            ])
        }

        fn push_through(
            &self,
            tracker: &Rc<RecordingTracker>,
            failing: Failing,
        ) -> RunOutcome {
            self.run_args(&Self::args(true), tracker, failing)
        }

        fn run_args(
            &self,
            args: &CreateArgs,
            tracker: &Rc<RecordingTracker>,
            failing: Failing,
        ) -> RunOutcome {
            let (applies, left) = failing.unwrap_or((|_| false, 0));
            let store_at = move |root: &Path| -> Box<dyn CreationStore> {
                Box::new(Faults {
                    inner: FileCorpusStore::new(root),
                    applies,
                    left: std::cell::Cell::new(left),
                })
            };
            run_with(
                self.root.path(),
                &self.config,
                &PluginWorkItemTemplate,
                args,
                &mut Seams {
                    registry: &StubRegistry(Rc::clone(tracker)),
                    store_at: &store_at,
                    draws: &mut RandomSuffixDraws,
                },
            )
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.path().join(relative)
        }

        fn drafts(&self) -> Vec<PathBuf> {
            std::fs::read_dir(self.path("meta/work/drafts"))
                .map(|entries| {
                    entries.flatten().map(|entry| entry.path()).collect()
                })
                .unwrap_or_default()
        }

        fn records_dir(&self) -> PathBuf {
            self.path("integrations/linear/pending-push")
        }

        fn records(&self) -> Vec<PathBuf> {
            std::fs::read_dir(self.records_dir())
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension().is_some_and(|ext| ext == "json")
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        fn baseline(&self, integration: &str) -> String {
            std::fs::read_to_string(
                self.path(&format!(
                    "integrations/{integration}/last-sync.json"
                )),
            )
            .unwrap_or_default()
        }
    }

    fn created(outcome: RunOutcome) -> (Option<PathBuf>, PushReport) {
        match outcome {
            RunOutcome::Created {
                path,
                push: Some(report),
            } => (path, report),
            RunOutcome::Failed(message) | RunOutcome::Pending(message) => {
                panic!("the create failed: {message}")
            }
            _ => panic!("the create pushed nothing"),
        }
    }

    fn pending(outcome: RunOutcome) -> String {
        match outcome {
            RunOutcome::Pending(message) => message,
            RunOutcome::Failed(message) => panic!("failed: {message}"),
            _ => panic!("the rerun was not refused as pending"),
        }
    }

    fn creates(tracker: &RecordingTracker) -> usize {
        tracker
            .calls()
            .iter()
            .filter(|call| {
                matches!(call, tracker_test_support::Call::Create { .. })
            })
            .count()
    }

    #[test]
    fn a_reachable_tracker_create_writes_an_item_keyed_by_the_tracker() {
        for integration in ["linear", "jira"] {
            let repo = PushingRepo::tracker_owned_on(integration);
            let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

            let (path, report) = created(repo.push_through(&tracker, None));

            let path = path.expect("the item exists");
            assert_eq!(report.outcome, PushOutcome::WriteOnce);
            assert_eq!(report.external_id.as_deref(), Some(KEY));
            assert_eq!(path, repo.path("meta/work/REC-1-a-tabled-idea.md"));
            let written = std::fs::read_to_string(&path).expect("read");
            let identity = work::work_item_files::identity_of(
                &work::work_item_files::WorkItemFile {
                    path: path.clone(),
                    content: written.clone(),
                },
            )
            .expect("identity");
            assert_eq!(identity.id, KEY, "{integration}");
            assert_eq!(identity.external_id.as_deref(), Some(KEY));
            assert!(
                written.contains("\n# REC-1: A tabled idea\n"),
                "{written}"
            );
            assert!(repo.drafts().is_empty(), "the draft is gone");
            assert!(repo.baseline(integration).contains(KEY));
        }
    }

    #[test]
    fn the_draft_exists_when_the_create_is_sent() {
        let repo = PushingRepo::tracker_owned();
        let mut recording = RecordingTracker::holding(Vec::new());
        let park = recording.parking_create();
        let tracker = Rc::new(recording);
        let drafts_dir = repo.path("meta/work/drafts");
        let watcher = std::thread::spawn(move || {
            park.wait_parked();
            let seen = std::fs::read_dir(&drafts_dir)
                .map(|entries| entries.flatten().count())
                .unwrap_or_default();
            park.release();
            seen
        });

        created(repo.push_through(&tracker, None));

        assert_eq!(watcher.join().expect("joined"), 1);
    }

    #[test]
    fn the_remote_description_is_updated_to_carry_the_tracker_key_h1() {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        created(repo.push_through(&tracker, None));

        let remote = tracker
            .show(&ExternalId::new(KEY.to_owned()))
            .expect("held");
        assert!(
            remote.body.contains("# REC-1: A tabled idea"),
            "{}",
            remote.body
        );
    }

    #[test]
    fn a_local_save_leaves_the_draft_and_no_marker() {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(
            RecordingTracker::holding(Vec::new()).failing_create(retryable()),
        );

        let (path, report) = created(repo.push_through(&tracker, None));

        assert_eq!(report.outcome, PushOutcome::LocalSave);
        assert_eq!(path, repo.drafts().first().cloned());
        assert!(repo.records().is_empty());
    }

    #[test]
    fn a_tracker_error_leaves_the_draft_and_an_attempted_marker_named_after_it()
    {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(
            RecordingTracker::holding(Vec::new()).failing_create(terminal()),
        );

        let (path, report) = created(repo.push_through(&tracker, None));

        assert_eq!(report.outcome, PushOutcome::LoudTerminal);
        let draft = path.expect("the draft");
        let draft_id = draft
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .map(|name| name[..12].to_owned())
            .expect("a draft name");
        assert_eq!(
            repo.records(),
            vec![repo.records_dir().join(format!("{draft_id}.json"))],
            "the_marker_is_named_after_the_draft_id_not_the_slug"
        );
    }

    #[test]
    fn a_failed_retirement_is_retried_once_and_succeeds_as_write_once() {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let (path, report) =
            created(repo.push_through(
                &tracker,
                Some((in_the_work_directory_itself, 1)),
            ));

        assert_eq!(report.outcome, PushOutcome::WriteOnce);
        assert_eq!(path, Some(repo.path("meta/work/REC-1-a-tabled-idea.md")));
    }

    #[test]
    fn two_failed_retirements_leave_the_draft_and_a_promotion_record_holding_the_key(
    ) {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let (path, report) =
            created(repo.push_through(
                &tracker,
                Some((in_the_work_directory_itself, 2)),
            ));

        assert_eq!(report.outcome, PushOutcome::CreatedUnwritten);
        assert_eq!(report.outcome.exit_code(), exit_codes::TERMINAL);
        assert_eq!(report.external_id.as_deref(), Some(KEY));
        assert_eq!(path, repo.drafts().first().cloned());
        let records = repo.records();
        assert_eq!(records.len(), 1);
        let record = std::fs::read_to_string(&records[0]).expect("record");
        assert!(record.contains("\"external_id\":\"REC-1\""), "{record}");
    }

    #[test]
    fn a_legacy_pattern_create_whose_local_write_fails_twice_is_created_unwritten(
    ) {
        let repo = PushingRepo::new();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let (path, report) =
            created(repo.push_through(
                &tracker,
                Some((in_the_work_directory_itself, 2)),
            ));

        assert_eq!(
            path, None,
            "a_legacy_created_unwritten_prints_an_empty_line_one"
        );
        assert_eq!(report.outcome, PushOutcome::CreatedUnwritten);
        assert_eq!(report.external_id.as_deref(), Some(KEY));
        assert!(repo.marker().exists(), "the created marker holds the key");
    }

    #[test]
    fn a_write_once_create_records_a_baseline_from_a_read_back() {
        let repo = PushingRepo::new();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let (path, report) = created(repo.push_through(&tracker, None));

        assert_eq!(report.outcome, PushOutcome::WriteOnce);
        let written =
            std::fs::read_to_string(path.expect("written")).expect("read");
        let baseline = repo.baseline("jira");
        assert!(baseline.contains("\"0001\""), "{baseline}");
        assert!(
            baseline.contains(&digest::local(&written).expect("digest")),
            "{baseline}"
        );
        assert!(!repo.marker().exists());
    }

    #[test]
    fn a_legacy_pattern_create_still_names_its_marker_by_slug() {
        let repo = PushingRepo::new();
        let tracker = Rc::new(
            RecordingTracker::holding(Vec::new()).failing_create(terminal()),
        );

        let (_, report) = created(repo.push_through(&tracker, None));

        assert_eq!(report.outcome, PushOutcome::LoudTerminal);
        assert!(repo.marker().exists());
    }

    #[test]
    fn rerunning_a_create_sends_no_second_create_at_any_record_stage() {
        let repo = PushingRepo::tracker_owned();
        let crashed = Rc::new(
            RecordingTracker::holding(Vec::new())
                .creating_then_failing(terminal()),
        );
        created(repo.push_through(&crashed, None));
        let rerun = Rc::new(RecordingTracker::holding(Vec::new()));

        let message = pending(repo.push_through(&rerun, None));

        assert!(message.starts_with("E_PUSH_PENDING: draft-"), "{message}");
        assert!(message.contains("--adopt <KEY>"), "{message}");
        assert_eq!(creates(&rerun), 0);
        assert_eq!(
            repo.drafts().len(),
            1,
            "rerunning_a_create_after_a_crash_leaves_the_first_draft_visible"
        );

        let retitled = Rc::new(
            RecordingTracker::holding(Vec::new())
                .failing_show(ExternalId::new(KEY.to_owned()), retryable()),
        );
        let other = PushingRepo::tracker_owned();
        let (_, report) = created(other.push_through(&retitled, None));
        assert_eq!(report.outcome, PushOutcome::CreatedUnwritten);
        let again = pending(other.push_through(&retitled, None));
        assert!(again.contains("work promote"), "{again}");
        assert_eq!(creates(&retitled), 1);
    }

    #[test]
    fn a_rerun_matching_an_existing_draft_is_e_draft_exists() {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        repo.run_args(&PushingRepo::args(false), &tracker, None);

        let message = pending(repo.push_through(&tracker, None));

        assert!(message.starts_with("E_DRAFT_EXISTS: "), "{message}");
        assert_eq!(creates(&tracker), 0);
    }

    #[test]
    fn an_attempted_record_for_a_different_request_does_not_block_a_tracker_create(
    ) {
        let repo = PushingRepo::tracker_owned();
        let crashed = Rc::new(
            RecordingTracker::holding(Vec::new()).failing_create(terminal()),
        );
        created(repo.push_through(&crashed, None));
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let other = CreateArgs {
            title: "Something else".to_owned(),
            ..PushingRepo::args(true)
        };

        let (_, report) = created(repo.run_args(&other, &tracker, None));

        assert_eq!(report.outcome, PushOutcome::WriteOnce);
    }

    #[test]
    fn a_legacy_marker_blocks_a_same_titled_tracker_create() {
        let repo = PushingRepo::tracker_owned();
        let marker = pending_push::path(
            &repo.path("integrations"),
            "linear",
            "a-tabled-idea",
        );
        std::fs::create_dir_all(marker.parent().expect("dir")).expect("dir");
        std::fs::write(
            &marker,
            pending_push::render(&PendingPush::Attempted {
                request: RequestFingerprint {
                    title: "A tabled idea".to_owned(),
                    digest: "old".to_owned(),
                    attempted_at: 1,
                    failure: None,
                },
            }),
        )
        .expect("marker");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let message = pending(repo.push_through(&tracker, None));

        assert!(message.contains("check the tracker"), "{message}");
        assert_eq!(creates(&tracker), 0);
    }

    #[test]
    fn a_collision_after_a_create_is_created_blocked_naming_the_holder() {
        let repo = PushingRepo::tracker_owned();
        std::fs::create_dir_all(repo.path("meta/work")).expect("work dir");
        std::fs::write(
            repo.path("meta/work/0002-holder.md"),
            "---\nid: \"0002\"\naliases: [\"REC-1\"]\n---\n\n# 0002: Holder\n",
        )
        .expect("holder");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let (path, report) = created(repo.push_through(&tracker, None));

        assert_eq!(report.outcome, PushOutcome::CreatedBlocked);
        assert_eq!(report.outcome.exit_code(), exit_codes::UNRESOLVED);
        assert_eq!(report.external_id.as_deref(), Some(KEY));
        assert_eq!(path, repo.drafts().first().cloned());
        let cause = report.cause.expect("a remedy");
        assert!(cause.contains("0002-holder.md"), "{cause}");
        assert!(cause.contains("work promote draft-"), "{cause}");
    }

    #[test]
    fn a_tracker_create_leaves_legacy_items_byte_identical() {
        let repo = PushingRepo::tracker_owned();
        std::fs::create_dir_all(repo.path("meta/work")).expect("work dir");
        let legacy = [
            ("meta/work/0042-a.md", "---\nid: \"0042\"\n---\n\n# 0042: A\n"),
            (
                "meta/work/ACC-0042-b.md",
                "---\nid: \"ACC-0042\"\n---\n\n# ACC-0042: B\n",
            ),
            (
                "meta/work/0230-c.md",
                "---\nid: \"0230\"\nexternal_id: \"PP-760\"\n---\n\n# 0230: C\n",
            ),
        ];
        for (path, content) in legacy {
            std::fs::write(repo.path(path), content).expect("legacy");
        }
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        created(repo.push_through(&tracker, None));

        for (path, content) in legacy {
            assert_eq!(
                std::fs::read_to_string(repo.path(path)).expect("read"),
                content
            );
        }
    }
}

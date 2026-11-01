//! Adapter/binary wiring for `work create`: places the new item, composes
//! its frontmatter via `work::create::compose_frontmatter`, and dispatches to
//! one of three creation strategies: a locally numbered item (optionally
//! pushed), a draft, or under `{tracker}` a draft promoted onto the issue the
//! tracker creates for it.

use corpus_adapters::CommonMarkBody;
use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use ::config::ReadTemplate;
use corpus::FilenameTimestampFormat;
use corpus_adapters::metadata::derive_at;
use corpus_adapters::metadata::VcsBackedRepoFactsProbe;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::LockdirLock;
use corpus_adapters::RealFs;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use tracker::CreatePreview;
use tracker::FieldResolution;
use tracker::RemoteTracker;
use vcs_adapters::library::InProcessProbe;
use work::create::assert_matches_template_schema;
use work::create::compose_frontmatter;
use work::create::resolve_author;
use work::create::CreateInputs;
use work::create::FieldValue;
use work::create::TypedLinkage;
use work::draft_id::DraftId;
use work::draft_id::SuffixDraws;
use work::id_scheme::IdOwnership;
use work::id_scheme::WorkItemIdScheme;
use work::identity_run::IdentityRunGuard;
use work::next_number::allocate;
use work::next_number::allocation_key;
use work::next_number::AllocationError;
use work::promotion::PromotionMode;
use work::requested_id::RequestedId;
use work::resolve::DirectoryLister;
use work_adapters::author::VcsBackedIdentityProbe;
pub use work_adapters::creation::blocked_remedy;
use work_adapters::creation::create_local_item;
use work_adapters::creation::create_tracker_keyed_item;
use work_adapters::creation::refuse_unfinished_push;
use work_adapters::creation::save_draft;
pub use work_adapters::creation::CreateFailure;
use work_adapters::creation::CreateLockGuard;
use work_adapters::creation::Creation;
pub use work_adapters::creation::CreationOutcome;
pub use work_adapters::creation::CreationStore;
use work_adapters::creation::DraftPromotionOutcome;
pub use work_adapters::creation::Drafted;
use work_adapters::creation::IntegrationState;
use work_adapters::creation::LegacyPushing;
use work_adapters::creation::LocalId;
use work_adapters::creation::NewItem;
pub use work_adapters::creation::PushReport;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::filesystem::FilesystemLister;
use work_adapters::promotion::promote;
use work_adapters::promotion::PromotionPorts;
use work_adapters::promotion::PromotionRow;

use crate::bounded_roots::configure_warning;
use crate::config::configured_override;
use crate::config::effective_nonempty;
use crate::config::locate_work_items;
use crate::config::templates_dir;
use crate::config::WorkItemLocation;
use crate::exit_codes;
use crate::identity_run::IdentityRunStart;
use crate::identity_workspace::IdentityWorkspace;
use crate::identity_workspace::OpenFailure;
use crate::requested_id::admit_requested_id;
use crate::requested_id::refuse_if_requested_id_taken;
use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;
use work_adapters::promotion_records::PromotionRecords;

const ID_PLACEHOLDER: &str = "NNNN";
const TITLE_PLACEHOLDER: &str = "Title as Short Noun Phrase";

#[derive(Clone)]
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
    pub id: Option<String>,
    pub push: bool,
    pub dry_run: bool,
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
    /// The identity run could not begin; nothing was read or written.
    Refused {
        message: String,
        code: u8,
    },
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

/// The next ID under `scheme`, rendered under `project` when given and
/// `scheme`'s own key otherwise.
pub fn allocate_id(
    scheme: &WorkItemIdScheme,
    root: &Path,
    work_dir: &Path,
    project: Option<&str>,
    _create: &CreateLockGuard,
) -> Result<String, String> {
    let pattern = scheme.pattern();
    let refused = |error: AllocationError| {
        error.message(pattern, work_dir.strip_prefix(root).unwrap_or(work_dir))
    };
    let key =
        allocation_key(pattern, project, scheme.key()).map_err(refused)?;
    let filenames = FilesystemLister::new(work_dir).filenames();
    let allocated =
        allocate(scheme, key.as_ref(), 1, &filenames).map_err(refused)?;
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
    location: WorkItemLocation,
    template: ::config::ResolvedTemplate,
    author: String,
    date: String,
}

impl NewItem for CreationContext<'_> {
    fn title(&self) -> &str {
        &self.args.title
    }

    fn kind(&self) -> &str {
        &self.args.kind
    }

    fn slug(&self) -> String {
        slugify(&self.args.title)
    }

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
}

fn integration_state(
    config: &dyn ConfigAccess,
    repo_root: &Path,
) -> Result<IntegrationState, String> {
    let name = effective_nonempty(config, "work.integration")
        .map_err(|error| error.to_string())?;
    let root = crate::sync::integrations_dir(config, repo_root)
        .map_err(|error| error.to_string())?;
    Ok(IntegrationState { root, name })
}

impl CreationContext<'_> {
    fn integration_state(&self) -> Result<IntegrationState, String> {
        integration_state(self.config, &self.location.root)
    }

    fn allocate_local_id(
        &self,
        create: &CreateLockGuard,
    ) -> Result<String, String> {
        allocate_id(
            &self.location.scheme,
            &self.location.root,
            &self.location.work_dir,
            self.args.project.as_deref(),
            create,
        )
    }
}

/// The context, beside the ID `args` requests once admitted. Nothing is
/// created before the request is admitted.
fn creation_context<'a>(
    start: &Path,
    config: &'a dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &'a CreateArgs,
) -> Result<(CreationContext<'a>, Option<RequestedId>), String> {
    let location = locate_work_items(start, config)?;
    let requested = admit_requested_id(args, &location.scheme)?;
    std::fs::create_dir_all(&location.work_dir).map_err(|error| {
        format!("could not create the work-item directory: {error}")
    })?;
    let pattern = location.scheme.pattern();
    if pattern.is_tracker_owned() {
        allocation_key(pattern, args.project.as_deref(), None)
            .map_err(|error| error.message(pattern, &location.work_dir))?;
    }
    let metadata = derive_at(
        &location.root,
        FilenameTimestampFormat::DateTimeUnderscored,
        &VcsBackedRepoFactsProbe::new(&InProcessProbe),
    )
    .map_err(|error| error.to_string())?;
    let author = resolve_author(
        args.author.as_deref(),
        &VcsBackedIdentityProbe::new(&InProcessProbe),
    )?;
    let template = resolve_and_check_template(config, templates)?;
    let context = CreationContext {
        args,
        config,
        location,
        template,
        author,
        date: metadata.datetime_utc,
    };
    Ok((context, requested))
}

/// Creates one item through the strategy its ID ownership and `--push`
/// select, calling `drafted` as soon as a draft is on disk.
///
/// # Errors
///
/// [`CreateFailure::Pending`] when an earlier create of the same content
/// is still pending, otherwise [`CreateFailure::Failed`].
pub fn create_item(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateArgs,
    seams: &mut Seams<'_>,
    drafted: &mut Drafted<'_>,
    identity_run: &IdentityRunGuard,
) -> Result<CreationOutcome, CreateFailure> {
    let (context, requested) =
        creation_context(start, config, templates, args)?;
    let location = &context.location;
    let state_dir = location.root.join(crate::sync::STATE_DIR);
    let lock = LockdirLock::new(&location.work_dir);
    let creation = Creation {
        item: &context,
        parser: &CommonMarkBody,
        repo_root: &location.root,
        work_dir: &location.work_dir,
        state_dir: &state_dir,
        store_at: seams.store_at,
        reader: &RealFs,
        lock: &lock,
        now: attempted_at_epoch(),
    };
    let registry = seams.registry;
    let allocate = |create: &CreateLockGuard| context.allocate_local_id(create);
    let local_id = requested
        .as_ref()
        .map_or(LocalId::Allocate(&allocate), LocalId::Requested);
    match (location.scheme.ownership(), args.push) {
        (IdOwnership::Local, false) => {
            Ok(create_local_item(&creation, &local_id, None, identity_run)?)
        }
        (IdOwnership::Local, true) => {
            let state = context.integration_state()?;
            let tracker = || {
                registry.resolve(&state.name).map_err(|error| {
                    work::sync::push_decide(
                        dispatch_code_for_selection_error(&error),
                        1,
                        false,
                    )
                })
            };
            Ok(create_local_item(
                &creation,
                &local_id,
                Some(&LegacyPushing {
                    state: &state,
                    tracker: &tracker,
                }),
                identity_run,
            )?)
        }
        (IdOwnership::Tracker, false) => {
            Ok(save_draft(&creation, seams.draws, drafted, identity_run)?)
        }
        (IdOwnership::Tracker, true) => {
            let state = context.integration_state()?;
            let records_store = (seams.store_at)(&state.root);
            let records = state.promotion_records(records_store.as_ref());
            let store_at = seams.store_at;
            let mut promotion = |draft: &DraftId, draft_path: PathBuf| {
                let Ok(tracker) = registry.resolve(&state.name) else {
                    return Ok(DraftPromotionOutcome::NoTracker);
                };
                let workspace = match IdentityWorkspace::for_identity_change(
                    config,
                    &location.root,
                    &location.work_dir,
                    &state.root,
                    &state.name,
                ) {
                    Ok((workspace, _)) => workspace,
                    Err(OpenFailure::Refused(refusal)) => {
                        return Ok(DraftPromotionOutcome::Refused {
                            warning: configure_warning(&refusal),
                        })
                    }
                    Err(OpenFailure::Failed(message)) => return Err(message),
                };
                let store = store_at(&location.root);
                Ok(attempt_promotion(
                    draft,
                    draft_path,
                    &workspace,
                    store.as_ref(),
                    tracker.as_ref(),
                    &records,
                    identity_run,
                ))
            };
            create_tracker_keyed_item(
                &creation,
                &state,
                &records,
                seams.draws,
                drafted,
                &mut promotion,
                identity_run,
            )
        }
    }
}

/// Promotes the draft `create --push` just saved, through `workspace`.
fn attempt_promotion(
    draft: &DraftId,
    draft_path: PathBuf,
    workspace: &IdentityWorkspace,
    store: &dyn CreationStore,
    tracker: &dyn RemoteTracker,
    records: &dyn PromotionRecords,
    identity_run: &IdentityRunGuard,
) -> DraftPromotionOutcome {
    let baseline = workspace.baseline(store);
    let retirement = workspace.retirement_ports(store, &baseline);
    let ports = PromotionPorts {
        tracker,
        parser: &CommonMarkBody,
        retirement: &retirement,
        records,
    };
    let result = promote(draft, &PromotionMode::Standard, &ports, identity_run);
    DraftPromotionOutcome::Attempted(PromotionRow::of(
        draft,
        draft_path,
        result,
        &ports,
        workspace.state_dir(),
    ))
}

/// What `run` takes from its caller beyond the request: the tracker, the
/// store items are written through, and the source of draft suffixes.
pub struct Seams<'a> {
    pub registry: &'a dyn TrackerRegistry,
    pub store_at: &'a dyn Fn(&Path) -> Box<dyn CreationStore>,
    pub draws: &'a mut dyn SuffixDraws,
    pub identity_run: &'a IdentityRunStart<'a>,
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
            identity_run: &crate::identity_run::begin,
        },
    )
}

/// The real create's `--id` checks, in its order, without its locks: a
/// pass reserves nothing. An integration that cannot be read fails where
/// the real create's does, before the taken check only under `--push`.
fn refuse_requested_id_before_preview(
    location: &WorkItemLocation,
    args: &CreateArgs,
    integration: Result<&IntegrationState, &String>,
) -> Result<(), String> {
    let Some(requested) = admit_requested_id(args, &location.scheme)? else {
        return Ok(());
    };
    if args.push {
        let state = integration.map_err(String::clone)?;
        refuse_unfinished_push(state, &slugify(&args.title), &RealFs)?;
    }
    refuse_if_requested_id_taken(&requested, location)
}

fn preview_requested_id(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &CreateArgs,
    registry: &dyn TrackerRegistry,
) -> RunOutcome {
    let location = match locate_work_items(start, config) {
        Ok(location) => location,
        Err(message) => return RunOutcome::Failed(message),
    };
    let integration = integration_state(config, &location.root);
    if let Err(refusal) = refuse_requested_id_before_preview(
        &location,
        args,
        integration.as_ref(),
    ) {
        return RunOutcome::Failed(refusal);
    }
    match integration {
        Ok(state) => preview_push(&state.name, &args.kind, registry),
        Err(message) => RunOutcome::Failed(message),
    }
}

#[must_use]
pub fn run_with(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateArgs,
    seams: &mut Seams<'_>,
) -> RunOutcome {
    if args.dry_run && args.id.is_some() {
        return preview_requested_id(start, config, args, seams.registry);
    }
    if args.dry_run {
        let integration = match effective_nonempty(config, "work.integration") {
            Ok(value) => value,
            Err(error) => return RunOutcome::Failed(error.to_string()),
        };
        return preview_push(&integration, &args.kind, seams.registry);
    }
    let root = config_adapters::FileConfigStore::discover_root(start);
    let identity_run = match (seams.identity_run)(&root, "create") {
        Ok(guard) => guard,
        Err(refusal) => {
            let (message, code) = crate::identity_run::refused(refusal, &root);
            return RunOutcome::Refused { message, code };
        }
    };
    match create_item(
        start,
        config,
        templates,
        args,
        seams,
        &mut |_| Ok(()),
        &identity_run,
    ) {
        Ok(CreationOutcome { path, push }) => {
            RunOutcome::Created { path, push }
        }
        Err(CreateFailure::Pending { message, .. }) => {
            RunOutcome::Pending(message)
        }
        Err(CreateFailure::Failed(message)) => RunOutcome::Failed(message),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::literal_string_with_formatting_args
)]
mod tests {
    use tracker::CreatePreview;
    use tracker::ExternalId;
    use tracker::FieldResolution;
    use tracker::RemoteTracker;
    use tracker::TrackerError;
    use tracker_test_support::RecordingTracker;

    use super::*;
    use std::rc::Rc;
    use work::sync::PendingPush;
    use work::sync::PushOutcome;
    use work::sync::RequestFingerprint;
    use work_adapters::sync::pending_push;

    use crate::test_support::FakeConfig;
    use crate::test_support::FixedRegistry;
    use crate::test_support::PushingRepo;

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

    #[test]
    fn a_create_push_with_a_symlinked_retirement_recovery_saves_locally_before_any_tracker_call(
    ) {
        let repo = PushingRepo::tracker_owned();
        let elsewhere = tempfile::tempdir().expect("tempdir");
        let state = repo.root.path().join(".accelerator/state");
        std::fs::create_dir_all(&state).expect("mkdir");
        std::os::unix::fs::symlink(
            elsewhere.path(),
            state.join("retirement-recovery"),
        )
        .expect("symlink");
        let tracker = RecordingTracker::holding(Vec::new());

        let outcome = repo.create(tracker);

        let RunOutcome::Created {
            path: Some(draft),
            push: Some(report),
        } = outcome
        else {
            panic!("a refused root still saves the draft");
        };
        assert!(draft.exists(), "the draft is saved");
        assert_eq!(report.outcome, PushOutcome::LocalSave);
        assert_eq!(report.outcome.exit_code(), exit_codes::CLEAN);
        let warning = report.warning.expect("the refusal is named");
        assert!(
            warning.starts_with(
                "warning: -: configure failed: .accelerator/state/\
                 retirement-recovery is a symlink to "
            ),
            "{warning}"
        );
        assert!(std::fs::read_dir(elsewhere.path())
            .expect("readable")
            .next()
            .is_none());
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

    fn refusal_of(outcome: RunOutcome) -> (String, u8) {
        match outcome {
            RunOutcome::Refused { message, code } => (message, code),
            _ => panic!("the create was not refused"),
        }
    }

    #[test]
    fn create_refuses_while_a_rekey_is_in_progress_and_writes_nothing() {
        for push in [false, true] {
            let repo = PushingRepo::new();
            crate::test_support::rekey_in_progress(repo.root.path());
            let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

            let (message, code) = refusal_of(repo.run_args(
                &PushingRepo::args(push),
                &tracker,
                None,
            ));

            assert_eq!(code, exit_codes::UNRESOLVED, "push: {push}");
            assert!(
                message.starts_with("Error: E_REKEY_IN_PROGRESS"),
                "{message}"
            );
            assert!(message.contains("accelerator work rekey"), "{message}");
            assert!(tracker.calls().is_empty(), "push: {push}");
            assert!(
                crate::test_support::meta_tree(repo.root.path()).is_empty(),
                "push: {push}"
            );
        }
    }

    #[test]
    fn create_dry_run_is_not_guarded() {
        let repo = PushingRepo::new();
        crate::test_support::rekey_in_progress(repo.root.path());
        let args = CreateArgs {
            dry_run: true,
            ..PushingRepo::args(true)
        };

        let outcome =
            repo.create_with(&args, RecordingTracker::holding(Vec::new()));

        assert!(matches!(
            outcome,
            RunOutcome::Previewed(_) | RunOutcome::PreviewFailed { .. }
        ));
    }

    #[test]
    fn create_in_a_repository_without_a_work_directory_still_succeeds() {
        let repo = PushingRepo::new();
        assert!(!repo.path("meta/work").exists());

        let outcome =
            repo.create_pushing(false, RecordingTracker::holding(Vec::new()));

        assert!(matches!(outcome, RunOutcome::Created { path: Some(_), .. }));
    }

    fn in_the_work_directory_itself(path: &Path) -> bool {
        path.parent().is_some_and(|dir| dir.ends_with("meta/work"))
            && path.extension().is_some_and(|extension| extension == "md")
    }

    const KEY: &str = "REC-1";

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
        assert!(
            report
                .cause
                .as_deref()
                .is_some_and(|c| c.contains("injected")),
            "{:?}",
            report.cause
        );
        assert!(repo.marker().exists(), "the created marker holds the key");
    }

    #[test]
    fn a_created_marker_over_an_unlistable_corpus_fails_rather_than_reusing_its_key(
    ) {
        use std::os::unix::fs::PermissionsExt as _;

        let repo = PushingRepo::new();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        created(
            repo.push_through(
                &tracker,
                Some((in_the_work_directory_itself, 2)),
            ),
        );
        std::fs::write(
            repo.path("meta/work/linked.md"),
            format!("---\nid: \"linked\"\nexternal_id: \"{KEY}\"\n---\n"),
        )
        .expect("linked item");
        let drafts = repo.path("meta/work/drafts");
        std::fs::create_dir_all(&drafts).expect("drafts");
        std::fs::set_permissions(
            &drafts,
            std::fs::Permissions::from_mode(0o000),
        )
        .expect("chmod");

        let outcome = repo.push_through(&tracker, None);

        std::fs::set_permissions(
            &drafts,
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("chmod back");
        let RunOutcome::Failed(message) = outcome else {
            panic!("the push was not refused");
        };
        assert!(message.contains("drafts"), "{message}");
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
        let (baseline, _) =
            work_adapters::sync::baseline::Baseline::read(Some(&baseline));
        let remote = tracker
            .show(&tracker::ExternalId::new(
                report.external_id.expect("a key"),
            ))
            .expect("the created issue");
        let expected =
            work_adapters::sync::baseline_digester::BaselineDigester::new(
                tracker.as_ref(),
                &CommonMarkBody,
            )
            .digests(&written, Some(&remote.body))
            .expect("digests");
        let Some(work_adapters::sync::baseline::Entry::Split {
            legacy,
            split,
            ..
        }) = baseline.get("0001")
        else {
            panic!("a recipe-1 entry is recorded");
        };
        assert_eq!(split, &expected.split);
        assert_eq!(legacy, &expected.legacy);
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
    fn a_draft_written_while_a_create_waits_for_the_lock_is_e_draft_exists() {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        repo.run_args(&PushingRepo::args(false), &tracker, None);
        let concurrent_draft = repo.drafts().remove(0);
        let content =
            std::fs::read_to_string(&concurrent_draft).expect("draft");
        std::fs::remove_file(&concurrent_draft).expect("remove");
        let lock_path =
            repo.path("meta/work").join(LockdirLock::CREATE_LOCKDIR);
        let (locked, lock_held) = std::sync::mpsc::channel();
        let concurrent = std::thread::spawn(move || {
            let _guard = store::lock::acquire(
                &lock_path,
                store::lock::LockOptions::default(),
            )
            .expect("lock");
            locked.send(()).expect("signal");
            std::thread::sleep(std::time::Duration::from_millis(300));
            std::fs::write(&concurrent_draft, content).expect("write");
        });
        lock_held.recv().expect("locked");

        let message = pending(repo.push_through(&tracker, None));

        concurrent.join().expect("joined");
        assert!(message.starts_with("E_DRAFT_EXISTS: "), "{message}");
        assert_eq!(creates(&tracker), 0);
    }

    #[test]
    fn a_rerun_whose_title_the_frontmatter_escapes_is_e_draft_exists() {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let quoted = |push| CreateArgs {
            title: r#"Say "hi" to C:\temp"#.to_owned(),
            ..PushingRepo::args(push)
        };
        repo.run_args(&quoted(false), &tracker, None);

        let message = pending(repo.run_args(&quoted(true), &tracker, None));

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

    fn written_at(outcome: RunOutcome) -> PathBuf {
        match outcome {
            RunOutcome::Created {
                path: Some(path), ..
            } => path,
            RunOutcome::Failed(message) | RunOutcome::Pending(message) => {
                panic!("the create failed: {message}")
            }
            _ => panic!("the create wrote no file"),
        }
    }

    fn failure(outcome: RunOutcome) -> String {
        match outcome {
            RunOutcome::Failed(message) => message,
            RunOutcome::Created { path, .. } => {
                panic!("the create was not refused: wrote {path:?}")
            }
            _ => panic!("the create did not fail"),
        }
    }

    fn assert_named(repo: &PushingRepo, path: &Path, id: &str) {
        assert_eq!(
            path,
            repo.path(&format!("meta/work/{id}-a-tabled-idea.md"))
        );
        let written = std::fs::read_to_string(path).expect("written");
        assert!(written.contains(&format!("\nid: \"{id}\"\n")), "{written}");
        assert!(
            written.contains(&format!("\n# {id}: A tabled idea\n")),
            "{written}"
        );
        assert!(!written.contains("NNNN"), "{written}");
    }

    fn create_local(repo: &PushingRepo, args: &CreateArgs) -> RunOutcome {
        repo.run_args(
            args,
            &Rc::new(RecordingTracker::holding(Vec::new())),
            None,
        )
    }

    #[test]
    fn an_untaken_id_names_the_written_item() {
        let repo = PushingRepo::new();

        let path = written_at(create_local(
            &repo,
            &PushingRepo::args_with_id("0400", false),
        ));

        assert_named(&repo, &path, "0400");
    }

    #[test]
    fn an_untaken_id_under_a_key_pattern_is_written_in_the_case_given() {
        for id in ["XY-0400", "xy-0400", "Xy-0400", "X-0400", "X1-0400"] {
            let repo = PushingRepo::keyed();

            let path = written_at(create_local(
                &repo,
                &PushingRepo::args_with_id(id, false),
            ));

            assert_named(&repo, &path, id);
        }
    }

    #[test]
    fn the_lowest_and_highest_numbers_are_accepted() {
        for id in ["0001", "9999"] {
            let repo = PushingRepo::new();

            let path = written_at(create_local(
                &repo,
                &PushingRepo::args_with_id(id, false),
            ));

            assert_named(&repo, &path, id);
        }
    }

    #[test]
    fn a_different_key_is_not_taken() {
        let repo = PushingRepo::keyed();
        repo.holding("meta/work/PP-0400-a.md", "");

        let path = written_at(create_local(
            &repo,
            &PushingRepo::args_with_id("XY-0400", false),
        ));

        assert_named(&repo, &path, "XY-0400");
    }

    type RepoFor = fn() -> PushingRepo;

    fn assert_refused_untouched(
        repo: &PushingRepo,
        args: &CreateArgs,
        code: &str,
    ) -> String {
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let before = crate::test_support::meta_tree(repo.root.path());

        let message = failure(repo.run_args(args, &tracker, None));

        assert!(message.starts_with(&format!("{code}: ")), "{message}");
        assert_eq!(crate::test_support::meta_tree(repo.root.path()), before);
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
        message
    }

    #[test]
    fn each_admission_refusal_writes_nothing_and_sends_nothing() {
        let cases: [(RepoFor, &str, Option<&str>, &str); 5] = [
            (
                PushingRepo::keyed,
                "XY-0400",
                Some("XY"),
                "E_ID_WITH_PROJECT",
            ),
            (
                PushingRepo::tracker_owned,
                "0400",
                None,
                "E_ID_TRACKER_OWNED",
            ),
            (
                PushingRepo::new,
                "draft-k7mq3x",
                None,
                "E_ID_DRAFT_RESERVED",
            ),
            (PushingRepo::new, "400", None, "E_ID_PATTERN_MISMATCH"),
            (PushingRepo::new, "../0400", None, "E_ID_PATTERN_MISMATCH"),
        ];
        for (repo, id, project, code) in cases {
            for push in [false, true] {
                let repo = repo();
                let args = CreateArgs {
                    project: project.map(str::to_owned),
                    ..PushingRepo::args_with_id(id, push)
                };

                assert_refused_untouched(&repo, &args, code);

                assert!(!repo.path("meta/work").exists(), "{id} push: {push}");
            }
        }
    }

    #[test]
    fn a_config_error_precedes_every_id_refusal() {
        let repo = PushingRepo::configured(&[
            ("work.id_pattern", "{key}-{number:04d}"),
            ("work.integration", "jira"),
        ]);
        let args = CreateArgs {
            project: Some("XY".to_owned()),
            ..PushingRepo::args_with_id("XY-0400", false)
        };

        assert_refused_untouched(&repo, &args, "E_WORK_KEY_REQUIRED");
    }

    #[test]
    fn a_malformed_pattern_reports_its_own_error_before_any_id_refusal() {
        for pattern in ["{number:4d}", "x/{number:04d}"] {
            let repo = PushingRepo::configured(&[("work.id_pattern", pattern)]);
            let with_id = CreateArgs {
                project: Some("XY".to_owned()),
                ..PushingRepo::args_with_id("0400", false)
            };

            let refused = assert_refused_untouched(
                &repo,
                &with_id,
                "E_WORK_ID_PATTERN_INVALID",
            );

            assert_eq!(
                refused,
                failure(create_local(&repo, &PushingRepo::args(false)))
            );
        }
    }

    #[test]
    fn an_id_with_an_invalid_project_is_refused_as_with_project() {
        let repos: [RepoFor; 2] =
            [PushingRepo::keyed, PushingRepo::tracker_owned];
        for repo in repos {
            let args = CreateArgs {
                project: Some("1X".to_owned()),
                ..PushingRepo::args_with_id("0400", false)
            };

            assert_refused_untouched(&repo(), &args, "E_ID_WITH_PROJECT");
        }
    }

    #[test]
    fn an_invalid_project_without_an_id_keeps_todays_order() {
        let args = CreateArgs {
            project: Some("1X".to_owned()),
            ..PushingRepo::args(false)
        };

        assert_refused_untouched(
            &PushingRepo::new(),
            &args,
            "E_PATTERN_KEY_UNUSED",
        );
        let keyed = assert_refused_untouched(
            &PushingRepo::keyed(),
            &args,
            "E_WORK_KEY_INVALID",
        );
        assert!(keyed.starts_with("E_WORK_KEY_INVALID: --project '1X':"));
    }

    #[test]
    fn a_taken_id_is_refused_naming_its_holder() {
        let cases: [(RepoFor, &str, &str, &str, &str); 4] = [
            (
                PushingRepo::new,
                "meta/work/0400-other.md",
                "",
                "0400",
                "its filename prefix '0400'",
            ),
            (
                PushingRepo::keyed,
                "meta/work/xy-0400-other.md",
                "",
                "XY-0400",
                "its filename prefix 'xy-0400'",
            ),
            (
                PushingRepo::new,
                "meta/work/drafts/draft-k7mq3x-x.md",
                "---\nid: \"0400\"\n---\n",
                "0400",
                "its id '0400'",
            ),
            (
                PushingRepo::new,
                "meta/work/0007-holder.md",
                "---\nid: \"0007\"\nexternal_id: \"0400\"\n---\n",
                "0400",
                "its external_id '0400'",
            ),
        ];
        for (repo, holder, content, id, holding) in cases {
            for push in [false, true] {
                let repo = repo();
                repo.holding(holder, content);

                let message = assert_refused_untouched(
                    &repo,
                    &PushingRepo::args_with_id(id, push),
                    "E_ID_TAKEN",
                );

                assert!(
                    message.contains(&format!("held by {holder} as {holding}")),
                    "{message}"
                );
            }
        }
    }

    #[test]
    fn an_id_over_an_unlistable_corpus_fails_rather_than_passing_as_untaken() {
        use std::os::unix::fs::PermissionsExt as _;

        let repo = PushingRepo::new();
        let drafts = repo.path("meta/work/drafts");
        std::fs::create_dir_all(&drafts).expect("drafts");
        std::fs::set_permissions(
            &drafts,
            std::fs::Permissions::from_mode(0o000),
        )
        .expect("chmod");

        let outcome =
            create_local(&repo, &PushingRepo::args_with_id("0400", false));

        std::fs::set_permissions(
            &drafts,
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("chmod back");
        let message = failure(outcome);
        assert!(message.contains("drafts"), "{message}");
        assert!(!repo.path("meta/work/0400-a-tabled-idea.md").exists());
    }

    #[test]
    fn a_holder_whose_content_cannot_be_read_still_takes_its_id() {
        let repo = PushingRepo::new();
        repo.holding("meta/work/0400-other.md", [0xff, 0xfe, 0xfd]);

        let message = failure(create_local(
            &repo,
            &PushingRepo::args_with_id("0400", false),
        ));

        assert!(message.starts_with("E_ID_TAKEN: "), "{message}");
        assert!(message.contains("meta/work/0400-other.md"), "{message}");
    }

    #[test]
    fn an_id_taken_while_the_create_waits_for_the_lock_is_refused() {
        let repo = PushingRepo::new();
        std::fs::create_dir_all(repo.path("meta/work")).expect("work dir");
        let lock_path =
            repo.path("meta/work").join(LockdirLock::CREATE_LOCKDIR);
        let concurrent_item = repo.path("meta/work/0400-other-title.md");
        let (locked, lock_held) = std::sync::mpsc::channel();
        let concurrent = std::thread::spawn(move || {
            let _guard = store::lock::acquire(
                &lock_path,
                store::lock::LockOptions::default(),
            )
            .expect("lock");
            locked.send(()).expect("signal");
            std::thread::sleep(std::time::Duration::from_millis(300));
            std::fs::write(&concurrent_item, "").expect("write");
        });
        lock_held.recv().expect("locked");

        let message = failure(create_local(
            &repo,
            &PushingRepo::args_with_id("0400", false),
        ));

        concurrent.join().expect("joined");
        assert!(message.starts_with("E_ID_TAKEN: "), "{message}");
        assert!(message.contains("0400-other-title.md"), "{message}");
    }

    #[test]
    fn a_rekey_in_progress_refuses_before_any_id_check() {
        let repo = PushingRepo::new();
        crate::test_support::rekey_in_progress(repo.root.path());

        let (message, _) = refusal_of(create_local(
            &repo,
            &PushingRepo::args_with_id("400", false),
        ));

        assert!(
            message.starts_with("Error: E_REKEY_IN_PROGRESS"),
            "{message}"
        );
    }

    fn numbered(repo: &PushingRepo, ids: impl IntoIterator<Item = String>) {
        for id in ids {
            repo.holding(&format!("meta/work/{id}-item.md"), "");
        }
    }

    #[test]
    fn allocation_after_an_id_continues_above_it() {
        let repo = PushingRepo::new();
        numbered(&repo, (1..=10).map(|number| format!("{number:04}")));
        written_at(create_local(
            &repo,
            &PushingRepo::args_with_id("0400", false),
        ));

        let path = written_at(create_local(&repo, &PushingRepo::args(false)));

        assert_eq!(path, repo.path("meta/work/0401-a-tabled-idea.md"));
    }

    #[test]
    fn each_key_keeps_its_own_sequence_after_an_id() {
        let repo = PushingRepo::keyed();
        numbered(&repo, (1..=10).map(|number| format!("PP-{number:04}")));
        let next =
            || written_at(create_local(&repo, &PushingRepo::args(false)));

        written_at(create_local(
            &repo,
            &PushingRepo::args_with_id("XY-0400", false),
        ));
        assert_eq!(next(), repo.path("meta/work/PP-0011-a-tabled-idea.md"));

        let under_xy = CreateArgs {
            project: Some("XY".to_owned()),
            ..PushingRepo::args(false)
        };
        assert_eq!(
            written_at(create_local(&repo, &under_xy)),
            repo.path("meta/work/XY-0401-a-tabled-idea.md")
        );

        written_at(create_local(
            &repo,
            &PushingRepo::args_with_id("pp-0500", false),
        ));
        assert_eq!(next(), repo.path("meta/work/PP-0501-a-tabled-idea.md"));
    }

    #[test]
    fn an_id_at_the_cap_exhausts_automatic_allocation() {
        let repo = PushingRepo::new();
        written_at(create_local(
            &repo,
            &PushingRepo::args_with_id("9999", false),
        ));
        let before = crate::test_support::meta_tree(repo.root.path());

        let message = failure(create_local(&repo, &PushingRepo::args(false)));

        assert!(message.starts_with("E_PATTERN_OVERFLOW: "), "{message}");
        assert!(
            message.contains("meta/work/9999-a-tabled-idea.md"),
            "{message}"
        );
        assert!(message.contains("a free lower ID"), "{message}");
        assert_eq!(crate::test_support::meta_tree(repo.root.path()), before);
    }

    fn written(path: &Path) -> String {
        std::fs::read_to_string(path).expect("written")
    }

    #[test]
    fn an_id_pushed_to_an_accepting_tracker_is_written_with_its_external_id() {
        let repo = PushingRepo::new();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let (path, report) = created(repo.run_args(
            &PushingRepo::args_with_id("0400", true),
            &tracker,
            None,
        ));

        let path = path.expect("written");
        assert_eq!(report.outcome, PushOutcome::WriteOnce);
        assert_eq!(creates(&tracker), 1);
        assert_named(&repo, &path, "0400");
        assert!(written(&path).contains(&format!("external_id: \"{KEY}\"")));
    }

    fn pushed_with_and_without_an_id(
        failing: fn() -> TrackerError,
    ) -> (PushReport, PushReport, PathBuf) {
        let push = |args: &CreateArgs| {
            let repo = PushingRepo::new();
            let tracker = Rc::new(
                RecordingTracker::holding(Vec::new()).failing_create(failing()),
            );
            let (path, report) = created(repo.run_args(args, &tracker, None));
            (report, path, repo)
        };
        let (without, _, _) = push(&PushingRepo::args(true));
        let (with, path, repo) = push(&PushingRepo::args_with_id("0400", true));
        let path = path.expect("written");
        assert_named(&repo, &path, "0400");
        assert!(!written(&path).contains("external_id"));
        (without, with, path)
    }

    #[test]
    fn a_rejected_push_under_an_id_matches_the_same_push_without_one() {
        let (without, with, _) = pushed_with_and_without_an_id(rejected);

        assert_eq!(with.outcome, without.outcome);
        assert_eq!(with.outcome.exit_code(), without.outcome.exit_code());
    }

    #[test]
    fn an_unreachable_tracker_under_an_id_saves_it_unsynced_as_without_one() {
        let (without, with, _) = pushed_with_and_without_an_id(retryable);

        assert_eq!(without.outcome, PushOutcome::LocalSave);
        assert_eq!(with.outcome, PushOutcome::LocalSave);
    }

    fn after_an_unknown_push_outcome_at_0400() -> PushingRepo {
        let repo = PushingRepo::new();
        let crashed = Rc::new(
            RecordingTracker::holding(Vec::new()).failing_create(terminal()),
        );
        let (_, report) = created(repo.run_args(
            &PushingRepo::args_with_id("0400", true),
            &crashed,
            None,
        ));
        assert_eq!(report.outcome, PushOutcome::LoudTerminal);
        repo
    }

    #[test]
    fn a_rerun_after_an_unknown_push_outcome_reports_push_pending() {
        let repo = after_an_unknown_push_outcome_at_0400();
        let rerun = Rc::new(RecordingTracker::holding(Vec::new()));

        let message = failure(repo.run_args(
            &PushingRepo::args_with_id("0400", true),
            &rerun,
            None,
        ));

        assert!(message.starts_with("E_PUSH_PENDING: "), "{message}");
        assert!(message.contains("remove the marker"), "{message}");
        assert!(rerun.calls().is_empty(), "{:?}", rerun.calls());
    }

    #[test]
    fn a_rerun_after_a_created_unwritten_push_reuses_its_key() {
        let repo = PushingRepo::new();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let args = PushingRepo::args_with_id("0400", true);
        let (_, report) = created(repo.run_args(
            &args,
            &tracker,
            Some((in_the_work_directory_itself, 2)),
        ));
        assert_eq!(report.outcome, PushOutcome::CreatedUnwritten);

        let (path, report) = created(repo.run_args(&args, &tracker, None));

        let path = path.expect("written");
        assert_eq!(report.outcome, PushOutcome::WriteOnce);
        assert_named(&repo, &path, "0400");
        assert!(written(&path).contains(&format!("external_id: \"{KEY}\"")));
        assert_eq!(creates(&tracker), 1);
    }

    #[test]
    fn a_different_item_at_a_pending_id_is_still_taken() {
        let repo = after_an_unknown_push_outcome_at_0400();
        let other = CreateArgs {
            title: "Something else".to_owned(),
            ..PushingRepo::args_with_id("0400", true)
        };

        let message = assert_refused_untouched(&repo, &other, "E_ID_TAKEN");

        assert!(message.contains("0400-a-tabled-idea.md"), "{message}");
    }

    #[test]
    fn a_local_create_with_an_id_reads_no_push_marker() {
        let repo = PushingRepo::new();
        let marker = repo.marker();
        std::fs::create_dir_all(marker.parent().expect("dir")).expect("dir");
        std::fs::write(&marker, "{ not a marker").expect("marker");

        let path = written_at(create_local(
            &repo,
            &PushingRepo::args_with_id("0400", false),
        ));

        assert_named(&repo, &path, "0400");
    }

    fn dry_run(args: &CreateArgs) -> CreateArgs {
        CreateArgs {
            dry_run: true,
            ..args.clone()
        }
    }

    fn dry_run_outcome(outcome: RunOutcome) -> String {
        match outcome {
            RunOutcome::Previewed(line) => format!("previewed: {line}"),
            RunOutcome::PreviewFailed { message, code } => {
                format!("preview failed ({code}): {message}")
            }
            RunOutcome::Failed(message) => format!("failed: {message}"),
            _ => panic!("the dry-run wrote or pushed"),
        }
    }

    fn dry_run_through(
        repo: &PushingRepo,
        args: &CreateArgs,
        tracker: &Rc<RecordingTracker>,
    ) -> String {
        dry_run_outcome(repo.run_args(&dry_run(args), tracker, None))
    }

    fn untouched_tracker() -> Rc<RecordingTracker> {
        Rc::new(RecordingTracker::holding(Vec::new()))
    }

    fn assert_previewed(outcome: &str) {
        assert!(outcome.starts_with("previewed: "), "{outcome}");
    }

    fn assert_failed_with(outcome: &str, code: &str) {
        assert!(
            outcome.starts_with(&format!("failed: {code}: ")),
            "{outcome}"
        );
    }

    #[test]
    fn a_dry_run_refuses_an_id_as_the_real_create_does() {
        let holding_0400: RepoFor = || {
            let repo = PushingRepo::new();
            repo.holding("meta/work/0400-other.md", "");
            repo
        };
        let cases: [(RepoFor, &str, Option<&str>, &str); 5] = [
            (
                PushingRepo::keyed,
                "XY-0400",
                Some("XY"),
                "E_ID_WITH_PROJECT",
            ),
            (
                PushingRepo::tracker_owned,
                "0400",
                None,
                "E_ID_TRACKER_OWNED",
            ),
            (
                PushingRepo::new,
                "draft-k7mq3x",
                None,
                "E_ID_DRAFT_RESERVED",
            ),
            (PushingRepo::new, "400", None, "E_ID_PATTERN_MISMATCH"),
            (holding_0400, "0400", None, "E_ID_TAKEN"),
        ];
        for (repo, id, project, code) in cases {
            for push in [false, true] {
                let repo = repo();
                let args = CreateArgs {
                    project: project.map(str::to_owned),
                    ..PushingRepo::args_with_id(id, push)
                };
                let tracker = untouched_tracker();
                let before = crate::test_support::meta_tree(repo.root.path());

                let outcome = dry_run_through(&repo, &args, &tracker);

                assert_failed_with(&outcome, code);
                assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
                assert_eq!(
                    crate::test_support::meta_tree(repo.root.path()),
                    before,
                    "{id} push: {push}"
                );
                assert_eq!(
                    outcome,
                    format!("failed: {}", failure(create_local(&repo, &args)))
                );
            }
        }
    }

    #[test]
    fn a_dry_run_with_an_untaken_id_previews_as_without_one() {
        let repos: [(RepoFor, &str); 2] =
            [(PushingRepo::new, "0400"), (PushingRepo::keyed, "PP-0400")];
        let trackers: [fn() -> RecordingTracker; 2] = [
            || RecordingTracker::holding(Vec::new()),
            || {
                RecordingTracker::holding(Vec::new())
                    .failing_preview(retryable())
            },
        ];
        for (repo, id) in repos {
            for tracker in trackers {
                for push in [false, true] {
                    let repo = repo();
                    let through = |args: &CreateArgs| {
                        dry_run_through(&repo, args, &Rc::new(tracker()))
                    };

                    let with = through(&PushingRepo::args_with_id(id, push));

                    assert_eq!(with, through(&PushingRepo::args(push)));
                    assert!(!with.starts_with("failed: "), "{with}");
                    assert!(!repo.path("meta/work").exists(), "{id}");
                }
            }
        }
    }

    #[test]
    fn a_dry_run_reports_the_config_error_the_real_create_does() {
        let repo = PushingRepo::configured(&[
            ("work.id_pattern", "{key}-{number:04d}"),
            ("work.integration", "jira"),
        ]);
        let args = CreateArgs {
            project: Some("XY".to_owned()),
            ..PushingRepo::args_with_id("XY-0400", false)
        };
        let tracker = untouched_tracker();

        let outcome = dry_run_through(&repo, &args, &tracker);

        assert_failed_with(&outcome, "E_WORK_KEY_REQUIRED");
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
    }

    #[test]
    fn a_dry_run_reports_a_malformed_pattern_as_the_real_create_does() {
        let repo = PushingRepo::configured(&[
            ("work.id_pattern", "{number:4d}"),
            ("work.integration", "jira"),
        ]);
        let args = PushingRepo::args_with_id("0400", false);
        let tracker = untouched_tracker();

        let outcome = dry_run_through(&repo, &args, &tracker);

        assert_failed_with(&outcome, "E_WORK_ID_PATTERN_INVALID");
        assert_eq!(
            outcome,
            format!("failed: {}", failure(create_local(&repo, &args)))
        );
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
    }

    #[test]
    fn a_dry_run_over_an_unlistable_corpus_fails_rather_than_previewing() {
        use std::os::unix::fs::PermissionsExt as _;

        let repo = PushingRepo::new();
        let drafts = repo.path("meta/work/drafts");
        std::fs::create_dir_all(&drafts).expect("drafts");
        std::fs::set_permissions(
            &drafts,
            std::fs::Permissions::from_mode(0o000),
        )
        .expect("chmod");
        let tracker = untouched_tracker();

        let outcome = dry_run_through(
            &repo,
            &PushingRepo::args_with_id("0400", false),
            &tracker,
        );

        std::fs::set_permissions(
            &drafts,
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("chmod back");
        assert!(outcome.starts_with("failed: "), "{outcome}");
        assert!(outcome.contains("drafts"), "{outcome}");
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
    }

    #[test]
    fn a_dry_run_checks_the_id_without_waiting_for_the_create_lock() {
        let cases = [
            (Some("meta/work/0400-other.md"), "failed: E_ID_TAKEN: "),
            (None, "previewed: "),
        ];
        for (holder, expected) in cases {
            let repo = std::sync::Arc::new(PushingRepo::new());
            std::fs::create_dir_all(repo.path("meta/work")).expect("work dir");
            if let Some(holder) = holder {
                repo.holding(holder, "");
            }
            let create_lock = store::lock::acquire(
                &repo.path("meta/work").join(LockdirLock::CREATE_LOCKDIR),
                store::lock::LockOptions::default(),
            )
            .expect("the create lock is free");
            let (done, finished) = std::sync::mpsc::channel();
            let checking = std::sync::Arc::clone(&repo);

            let dry_run = std::thread::spawn(move || {
                let outcome = dry_run_through(
                    &checking,
                    &PushingRepo::args_with_id("0400", true),
                    &untouched_tracker(),
                );
                let _ = done.send(outcome);
            });

            let outcome =
                finished.recv_timeout(std::time::Duration::from_secs(10));
            drop(create_lock);
            dry_run.join().expect("joined");
            let outcome = outcome.expect("the dry-run waited on the lock");
            assert!(outcome.starts_with(expected), "{outcome}");
        }
    }

    #[test]
    fn a_dry_run_without_an_id_resolves_no_scheme() {
        let repo = PushingRepo::configured(&[
            ("work.id_pattern", "{key}-{number:04d}"),
            ("work.integration", "jira"),
        ]);

        assert_previewed(&dry_run_through(
            &repo,
            &PushingRepo::args(true),
            &untouched_tracker(),
        ));
    }

    #[test]
    fn a_dry_run_with_an_id_is_still_not_guarded() {
        let repo = PushingRepo::new();
        crate::test_support::rekey_in_progress(repo.root.path());

        assert_previewed(&dry_run_through(
            &repo,
            &PushingRepo::args_with_id("0400", true),
            &untouched_tracker(),
        ));
    }

    #[test]
    fn a_dry_run_after_an_unknown_push_outcome_reports_push_pending() {
        let repo = after_an_unknown_push_outcome_at_0400();
        let tracker = untouched_tracker();

        let outcome = dry_run_through(
            &repo,
            &PushingRepo::args_with_id("0400", true),
            &tracker,
        );

        assert_failed_with(&outcome, "E_PUSH_PENDING");
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
    }

    fn with_a_corrupt_marker(repo: &PushingRepo) {
        let marker = repo.marker();
        std::fs::create_dir_all(marker.parent().expect("dir")).expect("dir");
        std::fs::write(&marker, "{ not a marker").expect("marker");
    }

    #[test]
    fn a_dry_run_over_an_unreadable_push_marker_reports_it() {
        let repo = PushingRepo::new();
        with_a_corrupt_marker(&repo);

        let outcome = dry_run_through(
            &repo,
            &PushingRepo::args_with_id("0400", true),
            &untouched_tracker(),
        );

        assert_failed_with(&outcome, "E_PUSH_MARKER_UNREADABLE");
    }

    #[test]
    fn a_dry_run_after_a_created_unwritten_push_previews() {
        let repo = PushingRepo::new();
        let tracker = untouched_tracker();
        let args = PushingRepo::args_with_id("0400", true);
        let (_, report) = created(repo.run_args(
            &args,
            &tracker,
            Some((in_the_work_directory_itself, 2)),
        ));
        assert_eq!(report.outcome, PushOutcome::CreatedUnwritten);

        assert_previewed(&dry_run_through(&repo, &args, &tracker));

        let (path, report) = created(repo.run_args(&args, &tracker, None));
        assert_eq!(report.outcome, PushOutcome::WriteOnce);
        assert_named(&repo, &path.expect("written"), "0400");
        assert_eq!(creates(&tracker), 1);
    }

    fn without_an_integration() -> PushingRepo {
        PushingRepo::configured(&[("paths.integrations", "integrations")])
    }

    #[test]
    fn a_dry_run_without_an_integration_refuses_the_id_first() {
        for push in [false, true] {
            let outcome = dry_run_through(
                &without_an_integration(),
                &PushingRepo::args_with_id("400", push),
                &untouched_tracker(),
            );

            assert_failed_with(&outcome, "E_ID_PATTERN_MISMATCH");
        }
    }

    struct UnreadableIntegrationsDir<'a>(&'a FakeConfig);

    impl ConfigAccess for UnreadableIntegrationsDir<'_> {
        fn get(
            &self,
            key: &::config::Key,
            level: Option<::config::Level>,
        ) -> Result<::config::Resolved, ::config::ConfigError> {
            if key.to_string() == "paths.integrations" {
                return Err(::config::ConfigError::Invalid {
                    detail: "paths.integrations could not be read".to_owned(),
                });
            }
            self.0.get(key, level)
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

    #[test]
    fn a_pushed_dry_run_reports_an_unreadable_integrations_dir_before_the_taken_check(
    ) {
        let repo = PushingRepo::new();
        repo.holding("meta/work/0400-other.md", "");
        let config = UnreadableIntegrationsDir(&repo.config);
        let under_config = |args: &CreateArgs| {
            repo.run_args_under(&config, args, &untouched_tracker(), None)
        };
        let pushed = PushingRepo::args_with_id("0400", true);

        let outcome = dry_run_outcome(under_config(&dry_run(&pushed)));

        assert!(outcome.contains("could not be read"), "{outcome}");
        assert_eq!(
            outcome,
            format!("failed: {}", failure(under_config(&pushed)))
        );
        assert_failed_with(
            &dry_run_outcome(under_config(&dry_run(
                &PushingRepo::args_with_id("0400", false),
            ))),
            "E_ID_TAKEN",
        );
    }

    #[test]
    fn a_dry_run_without_push_reads_no_push_marker() {
        let repo = PushingRepo::new();
        with_a_corrupt_marker(&repo);

        assert_previewed(&dry_run_through(
            &repo,
            &PushingRepo::args_with_id("0400", false),
            &untouched_tracker(),
        ));
    }
}

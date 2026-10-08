//! Adapter/binary wiring for `work create`: places the new item, composes
//! its frontmatter via `work::create::compose_frontmatter`, and dispatches to
//! one of three creation strategies: a locally numbered item (optionally
//! pushed), a draft, or under `{tracker}` a draft promoted onto the issue the
//! tracker creates for it.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use ::config::ReadTemplate;
use corpus::FilenameTimestampFormat;
use corpus::IdOwnership;
use corpus_adapters::compile_scan_regex;
use corpus_adapters::metadata::derive_at;
use corpus_adapters::metadata::VcsBackedRepoFactsProbe;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::LockdirLock;
use corpus_adapters::RealFs;
use corpus_adapters::RegexScanner;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use tracker::CreatePreview;
use tracker::FieldResolution;
use vcs_adapters::library::InProcessProbe;
use work::create::assert_matches_template_schema;
use work::create::compose_frontmatter;
use work::create::resolve_author;
use work::create::CreateInputs;
use work::create::FieldValue;
use work::create::TypedLinkage;
use work::draft_id::DraftId;
use work::draft_id::SuffixDraws;
use work::next_number::allocate;
use work::next_number::AllocationError;
use work::promotion::PromotionMode;
use work::resolve::DirectoryLister;
use work_adapters::author::VcsBackedIdentityProbe;
pub use work_adapters::creation::blocked_remedy;
use work_adapters::creation::create_local_item;
use work_adapters::creation::create_tracker_keyed_item;
use work_adapters::creation::save_draft;
pub use work_adapters::creation::CreateFailure;
use work_adapters::creation::Creation;
pub use work_adapters::creation::CreationOutcome;
pub use work_adapters::creation::CreationStore;
pub use work_adapters::creation::Drafted;
use work_adapters::creation::IntegrationState;
use work_adapters::creation::LegacyPushing;
use work_adapters::creation::NewItem;
pub use work_adapters::creation::PushReport;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::filesystem::FilesystemLister;
use work_adapters::promotion::promote;
use work_adapters::promotion::PromotionPorts;
use work_adapters::promotion::PromotionRow;

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
pub const LOCK_FILE_NAME: &str = LockdirLock::CREATE_LOCKDIR;

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

impl CreationContext<'_> {
    fn integration_state(&self) -> Result<IntegrationState, String> {
        let name = effective_nonempty(self.config, "work.integration")
            .map_err(|error| error.to_string())?;
        let root = crate::sync::integrations_dir(self.config, &self.root)
            .map_err(|error| error.to_string())?;
        Ok(IntegrationState { root, name })
    }

    fn allocate_local_id(&self) -> Result<String, String> {
        let project = self
            .args
            .project
            .clone()
            .or_else(|| self.scheme.key.clone());
        allocate_id(&self.scheme, &self.work_dir, project.as_deref())
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
) -> Result<CreationOutcome, CreateFailure> {
    let context = creation_context(start, config, templates, args)?;
    let state_dir = context.root.join(crate::sync::STATE_DIR);
    let lock = LockdirLock::new(&context.work_dir);
    let creation = Creation {
        item: &context,
        repo_root: &context.root,
        work_dir: &context.work_dir,
        state_dir: &state_dir,
        store_at: seams.store_at,
        reader: &RealFs,
        lock: &lock,
        now: attempted_at_epoch(),
    };
    let registry = seams.registry;
    match (context.scheme.ownership(), args.push) {
        (IdOwnership::Local, false) => Ok(create_local_item(
            &creation,
            &|| context.allocate_local_id(),
            None,
        )?),
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
                &|| context.allocate_local_id(),
                Some(&LegacyPushing {
                    state: &state,
                    tracker: &tracker,
                }),
            )?)
        }
        (IdOwnership::Tracker, false) => {
            Ok(save_draft(&creation, seams.draws, drafted)?)
        }
        (IdOwnership::Tracker, true) => {
            let state = context.integration_state()?;
            let records_store = (seams.store_at)(&state.root);
            let records = state.promotion_records(records_store.as_ref());
            let store_at = seams.store_at;
            let mut promotion = |draft: &DraftId, draft_path: PathBuf| {
                let Ok(tracker) = registry.resolve(&state.name) else {
                    return Ok(None);
                };
                let workspace = IdentityWorkspace::open(
                    config,
                    &context.root,
                    &context.work_dir,
                    &state.root,
                    &state.name,
                )?;
                let store = store_at(&context.root);
                let baseline = workspace.baseline(store.as_ref());
                let retirement =
                    workspace.retirement_ports(store.as_ref(), &baseline);
                let ports = PromotionPorts {
                    tracker: tracker.as_ref(),
                    retirement: &retirement,
                    records: &records,
                };
                let result = promote(draft, &PromotionMode::Standard, &ports);
                Ok(Some(PromotionRow::of(
                    draft,
                    draft_path,
                    result,
                    &ports,
                    workspace.state_dir(),
                )))
            };
            create_tracker_keyed_item(
                &creation,
                &state,
                &records,
                seams.draws,
                drafted,
                &mut promotion,
            )
        }
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
    match create_item(start, config, templates, args, seams, &mut |_| Ok(())) {
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
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::cell::RefCell;

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
    use work_adapters::sync::digest;
    use work_adapters::sync::pending_push;

    use crate::test_support::FakeConfig;
    use crate::test_support::Faults;
    use crate::test_support::PluginWorkItemTemplate;
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
    fn a_draft_written_while_a_create_waits_for_the_lock_is_e_draft_exists() {
        let repo = PushingRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        repo.run_args(&PushingRepo::args(false), &tracker, None);
        let concurrent_draft = repo.drafts().remove(0);
        let content =
            std::fs::read_to_string(&concurrent_draft).expect("draft");
        std::fs::remove_file(&concurrent_draft).expect("remove");
        let lock_path = repo.path("meta/work").join(LOCK_FILE_NAME);
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
}

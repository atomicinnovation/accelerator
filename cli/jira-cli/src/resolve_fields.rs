//! The `resolve-fields` contract: the single source of truth for the kind →
//! issue-type and the `--project` precedence a create resolves, emitted as one
//! tab-separated line. Read-only and API-free (Decision 4) — it reads
//! `work.default_project_code` through the config crates, never the tracker.
//!
//! Output: `<issue_type>\t<issue_type_source>\t<project>\t<project_source>` with
//! a trailing newline the caller's `read` depends on.

use config::ConfigAccess;
use config::Key;
use config_adapters::compose;
use config_adapters::LegacyPolicy;

use crate::cli::ResolveFieldsArgs;
use crate::frontmatter;

/// A resolved create field set.
pub struct Resolution {
    pub issue_type: String,
    pub issue_type_source: String,
    pub project: String,
    pub project_source: String,
}

impl Resolution {
    /// The tab-separated line the caller reads, trailing newline included.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\n",
            self.issue_type,
            self.issue_type_source,
            self.project,
            self.project_source,
        )
    }
}

/// Why a resolution could not be produced.
pub enum ResolveError {
    /// `--file` already carries a non-empty `external_id`.
    AlreadySynced { file: String, external_id: String },
    /// No project could be resolved from a flag, config or a project-coded id.
    NoProject,
    /// A malformed invocation (both modes, neither mode, unreadable file).
    Usage(String),
}

/// The kind → `(issue_type, source)` mapping. `source` is `mapped` for a known
/// kind, `default` for anything else, which still resolves to `Task`.
#[must_use]
fn issue_type(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "story" => ("Story", "mapped"),
        "bug" => ("Bug", "mapped"),
        "epic" => ("Epic", "mapped"),
        "task" | "spike" => ("Task", "mapped"),
        _ => ("Task", "default"),
    }
}

/// The project code embedded in a project-coded id (`PROJ-42` → `PROJ`), or
/// `None` for a bare-numeric or legacy id.
#[must_use]
fn project_from_id(id: &str) -> Option<&str> {
    let (prefix, number) = id.split_once('-')?;
    let starts_alpha = prefix
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic());
    let prefix_ok =
        starts_alpha && prefix.chars().all(|c| c.is_ascii_alphanumeric());
    let number_ok =
        !number.is_empty() && number.chars().all(|c| c.is_ascii_digit());
    (prefix_ok && number_ok).then_some(prefix)
}

/// The pure resolution over already-parsed inputs, with the configured default
/// project injected — the seam a unit test drives without a config directory.
///
/// # Errors
///
/// [`ResolveError::NoProject`] when no project resolves.
pub fn resolve_from(
    kind: &str,
    id: Option<&str>,
    project_flag: Option<&str>,
    config_project: Option<&str>,
) -> Result<Resolution, ResolveError> {
    let (issue_type, issue_type_source) = issue_type(kind);

    let (project, project_source) = if let Some(flag) = project_flag {
        (flag.to_owned(), "flag")
    } else if let Some(configured) = config_project.filter(|c| !c.is_empty()) {
        (configured.to_owned(), "config")
    } else if let Some(from_id) = id.and_then(project_from_id) {
        (from_id.to_owned(), "id")
    } else {
        return Err(ResolveError::NoProject);
    };

    Ok(Resolution {
        issue_type: issue_type.to_owned(),
        issue_type_source: issue_type_source.to_owned(),
        project,
        project_source: project_source.to_owned(),
    })
}

/// Resolves the create field set from CLI args, reading config and, in file
/// mode, the work-item frontmatter.
///
/// # Errors
///
/// [`ResolveError`] for a malformed invocation, an already-synced file, or an
/// unresolvable project.
pub fn resolve(args: &ResolveFieldsArgs) -> Result<Resolution, ResolveError> {
    let config_project = configured_default_project();

    match (&args.file, &args.kind) {
        (Some(path), _) => {
            let content = std::fs::read_to_string(path).map_err(|error| {
                ResolveError::Usage(format!(
                    "--file {} is not readable: {error}",
                    path.display()
                ))
            })?;
            let block = frontmatter::block(&content).ok_or_else(|| {
                ResolveError::Usage(format!(
                    "{} has no parseable frontmatter",
                    path.display()
                ))
            })?;
            let external_id = frontmatter::field(block, "external_id")
                .map(|value| value.trim_matches(is_quote_or_space).to_owned())
                .filter(|value| !value.is_empty());
            if let Some(external_id) = external_id {
                return Err(ResolveError::AlreadySynced {
                    file: path.display().to_string(),
                    external_id,
                });
            }
            let kind = frontmatter::field(block, "kind").unwrap_or_default();
            let id = frontmatter::field(block, "id");
            resolve_from(
                &kind,
                id.as_deref(),
                args.project.as_deref(),
                config_project.as_deref(),
            )
        }
        (None, Some(kind)) => resolve_from(
            kind,
            args.id.as_deref(),
            args.project.as_deref(),
            config_project.as_deref(),
        ),
        (None, None) => Err(ResolveError::Usage(
            "pass --file <work-item-file> or --kind KIND".to_owned(),
        )),
    }
}

const fn is_quote_or_space(c: char) -> bool {
    c.is_whitespace() || c == '"' || c == '\''
}

/// `work.default_project_code` from the composed config, or `None` when unset or
/// unreadable — an unresolvable project is the resolver's own reportable state.
fn configured_default_project() -> Option<String> {
    let start = std::env::current_dir().ok()?;
    let composed = compose(&start, LegacyPolicy::Reject).ok()?;
    let key = Key::parse("work.default_project_code").ok()?;
    let value = composed
        .service
        .effective_nonempty(&key, None)
        .ok()?
        .rendered();
    (!value.is_empty()).then_some(value)
}

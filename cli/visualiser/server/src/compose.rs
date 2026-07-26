//! Composes the server's runtime [`Config`] directly from `.accelerator/*.md`,
//! the Model-1 replacement for the retired `config.json` writer. Each concern —
//! doc-path resolution, template-tier resolution, work-item-scheme assembly,
//! kanban/idle resolution, and the `ACCELERATOR_VISUALISER_*` env overlay — is a
//! focused function so it can be reasoned about and tested on its own.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use config::{
    ConfigAccess, ConfigError, Key, Level, ReadTemplate, Resolved, Source,
};
use config_adapters::{FileConfigStore, LegacyPolicy};

use corpus_adapters::work_item_pattern;

use crate::config::{Config, RawWorkItemConfig, TemplateTiers};

#[derive(Debug, thiserror::Error)]
pub enum ComposeError {
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("invalid work-item id pattern: {0}")]
    Pattern(#[from] work_item_pattern::PatternError),
}

/// Inputs the runtime supplies that are not config keys: the working directory
/// (from which the project root is discovered), the plugin root, the owner
/// identity (CLI flags), and the resolved bind host.
pub struct Params {
    pub cwd: PathBuf,
    pub plugin_root: PathBuf,
    pub owner_pid: i32,
    pub owner_start_time: Option<u64>,
    pub host: String,
}

/// Build the runtime [`Config`] by discovering the project root from `cwd` and
/// resolving every config-derived field through `ConfigService::effective`.
pub fn load(params: Params) -> Result<Config, ComposeError> {
    let composed = config_adapters::compose(&params.cwd, LegacyPolicy::Reject)?;
    let store = composed
        .store
        .with_plugin_root(Some(params.plugin_root.clone()));
    let service = &composed.service;
    let project_root = FileConfigStore::discover_root(&params.cwd);

    let tmp_rel = effective_path(service, "paths.tmp")?;
    let tmp_path =
        safe_dir(&project_root, &tmp_rel, "paths.tmp")?.join("visualiser");
    let log_path = tmp_path.join("server.log");

    let doc_paths = resolve_doc_paths(service, &project_root)?;
    let templates_rel = effective_path(service, "paths.templates")?;
    let templates = resolve_templates(
        service,
        &store,
        &project_root,
        &templates_rel,
        &params.plugin_root,
    )?;
    let work_item = Some(resolve_work_item(service)?);
    let kanban_columns = resolve_kanban(service)?;
    let idle_timeout = resolve_idle(service)?;
    let editor = resolve_optional(
        service,
        "visualiser.editor",
        "ACCELERATOR_VISUALISER_EDITOR",
    )?;
    let editor_project = resolve_optional(
        service,
        "visualiser.editor_project",
        "ACCELERATOR_VISUALISER_EDITOR_PROJECT",
    )?;

    Ok(Config {
        plugin_root: params.plugin_root,
        plugin_version: crate::VERSION.to_string(),
        project_root,
        tmp_path,
        host: params.host,
        owner_pid: params.owner_pid,
        owner_start_time: params.owner_start_time,
        log_path,
        doc_paths,
        templates,
        work_item,
        kanban_columns,
        idle_timeout,
        editor,
        editor_project,
    })
}

/// Validate a configured relative directory with the shared config safety rules,
/// then normalise it and join it onto `root`.
fn safe_dir(
    root: &Path,
    rel: &str,
    key: &str,
) -> Result<PathBuf, ComposeError> {
    if rel.is_empty()
        || rel.contains(['\t', '\n', '\0'])
        || config::paths::is_unsafe(rel)
    {
        return Err(ComposeError::Config(ConfigError::Invalid {
            detail: format!("{key} resolves to an unsafe path: {rel:?}"),
        }));
    }
    Ok(root.join(config::paths::normalise(rel)))
}

fn effective_path(
    service: &impl ConfigAccess,
    key: &str,
) -> Result<String, ComposeError> {
    Ok(service
        .effective_nonempty(&Key::parse(key)?, None)?
        .rendered())
}

/// Resolve the 13 doc-type directories through the shared config resolver and
/// key them by path-key against the project root.
fn resolve_doc_paths(
    service: &impl ConfigAccess,
    project_root: &Path,
) -> Result<HashMap<String, PathBuf>, ComposeError> {
    let mut map = HashMap::new();
    for resolved in config::paths::doc_type_dirs(service)? {
        map.insert(
            resolved.path_key.to_string(),
            project_root.join(&resolved.dir),
        );
    }
    Ok(map)
}

fn source_to_file(source: Source) -> Option<String> {
    let level = match source {
        Source::Personal => Level::Personal,
        Source::Team => Level::Team,
        Source::Catalogue | Source::Unset => return None,
    };
    Some(level.filename().to_string())
}

fn resolve_templates(
    service: &impl ConfigAccess,
    store: &FileConfigStore,
    project_root: &Path,
    templates_rel: &str,
    plugin_root: &Path,
) -> Result<HashMap<String, TemplateTiers>, ComposeError> {
    let user_root = safe_dir(project_root, templates_rel, "paths.templates")?;
    let plugin_templates = plugin_root.join("templates");
    let mut map = HashMap::new();
    for name in store.template_names() {
        let res = service
            .effective(&Key::parse(&format!("templates.{name}"))?, None)?;
        let configured =
            res.configured_value().filter(|v| !v.trim().is_empty());
        let (config_override, config_override_source) = match configured {
            Some(path) => {
                (Some(PathBuf::from(path)), source_to_file(res.source()))
            }
            None => (None, None),
        };
        map.insert(
            name.clone(),
            TemplateTiers {
                config_override,
                user_override: user_root.join(format!("{name}.md")),
                plugin_default: plugin_templates.join(format!("{name}.md")),
                config_override_source,
            },
        );
    }
    Ok(map)
}

fn resolve_work_item(
    service: &impl ConfigAccess,
) -> Result<RawWorkItemConfig, ComposeError> {
    let id_pattern = service
        .effective_nonempty(&Key::parse("work.id_pattern")?, None)?
        .rendered();
    let code = service
        .effective(&Key::parse("work.default_project_code")?, None)?
        .rendered();
    let default_project_code = (!code.trim().is_empty()).then(|| code.clone());
    let scan_regex = work_item_pattern::compile_scan_regex(
        &id_pattern,
        default_project_code.as_deref().unwrap_or(""),
    )?;
    Ok(RawWorkItemConfig {
        scan_regex,
        id_pattern,
        default_project_code,
    })
}

/// Resolve the kanban columns to an explicit list, `None` when the key is
/// absent (the boot-time resolver then applies the catalogue default). An
/// explicit empty (or null) list stays `Some(vec![])` so the resolver rejects
/// it at boot.
fn resolve_kanban(
    service: &impl ConfigAccess,
) -> Result<Option<Vec<String>>, ComposeError> {
    let key = Key::parse("visualiser.kanban_columns")?;
    Ok(match service.get(&key, None)? {
        Resolved::Found(value) => Some(value.as_string_sequence()),
        Resolved::Absent => None,
    })
}

fn env_nonempty(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|value| !value.is_empty())
}

/// Idle window precedence: env var > `visualiser.idle_timeout` > `None` (the
/// boot resolver applies the catalogue 8h default). A config-present value —
/// including an empty or null spelling — is returned verbatim so the resolver
/// rejects it at boot rather than silently defaulting.
fn resolve_idle(
    service: &impl ConfigAccess,
) -> Result<Option<String>, ComposeError> {
    if let Some(value) = env_nonempty("ACCELERATOR_VISUALISER_IDLE_TIMEOUT") {
        return Ok(Some(value));
    }
    let res =
        service.effective(&Key::parse("visualiser.idle_timeout")?, None)?;
    Ok(res.is_from_config().then(|| res.rendered()))
}

/// Editor / editor-project precedence: env var > config key > `None` (button
/// disabled). Whitespace-only values collapse to `None`; a set-but-empty env
/// var falls through to the config key.
fn resolve_optional(
    service: &impl ConfigAccess,
    key: &str,
    env_var: &str,
) -> Result<Option<String>, ComposeError> {
    if let Some(value) = env_nonempty(env_var) {
        let trimmed = value.trim().to_string();
        return Ok((!trimmed.is_empty()).then_some(trimmed));
    }
    let res = service.effective(&Key::parse(key)?, None)?;
    Ok(res
        .configured_value()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

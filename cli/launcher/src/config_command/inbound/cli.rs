//! Maps a parsed `config` request onto the injected core and presents the
//! result.
//!
//! Each handler resolves a [`Rendered`] or a [`Failure`]. A `Failure::Read` is a
//! read/IO failure the fail-safe boundary may degrade — by suppression for the
//! scalars and `work`, by a `## <Name> Unavailable` notice for the block
//! commands. A `Failure::Refusal` is a validation refusal that stays fail-closed
//! regardless of `--fail-safe`, so a bad `work.integration` enum is never
//! papered over into empty-and-exit-0.

use config::{catalogue, ConfigError, Key, Level, Resolved};

use crate::config_command::core::context::{self as context_core, SkillFile};
use crate::config_command::core::review::{self as review_view, Mode};
use crate::config_command::core::{
    agents as agents_view, dump as dump_view, paths as paths_view,
    summary as summary_view, template as template_view, ConfigStack, OnFailure,
};
use crate::config_command::render::{
    self, agents as agents_render, context as context_render,
    dump as dump_render, instructions as instructions_render,
    paths as paths_render, review as review_render, summary as summary_render,
    template as template_render, Rendered,
};

/// A parsed `config` request, owned by this module so the hexagon never names
/// the launcher's clap tree. The composition boundary maps the clap
/// `ConfigAction` onto this.
pub enum Action {
    Get {
        key: String,
        default: Option<String>,
        level: Option<Level>,
        explain: bool,
        on_failure: OnFailure,
    },
    Path {
        key: String,
        default: Option<String>,
        level: Option<Level>,
        explain: bool,
        on_failure: OnFailure,
    },
    Agent {
        name: String,
        on_failure: OnFailure,
    },
    Agents {
        on_failure: OnFailure,
    },
    Work {
        key: String,
        on_failure: OnFailure,
    },
    Context {
        skill: Option<String>,
        on_failure: OnFailure,
    },
    Instructions {
        skill: String,
        on_failure: OnFailure,
    },
    Paths {
        doc_types: bool,
        all: bool,
        on_failure: OnFailure,
    },
    Dump {
        on_failure: OnFailure,
    },
    Review {
        mode: Mode,
        on_failure: OnFailure,
    },
    Summary {
        hook: bool,
        on_failure: OnFailure,
    },
    Template {
        name: String,
        on_failure: OnFailure,
    },
    TemplatesList {
        on_failure: OnFailure,
    },
    TemplatesShow {
        name: String,
        on_failure: OnFailure,
    },
}

/// Runs a parsed `config` request against the composed stack.
///
/// # Errors
///
/// A [`ConfigError`] when a read fails and the request's `on_failure` is
/// [`OnFailure::Fail`], or when a validation refusal fires regardless of it.
pub fn run(stack: &ConfigStack, action: &Action) -> Result<(), ConfigError> {
    match action {
        Action::Get {
            key,
            default,
            level,
            explain,
            on_failure,
        } => finish(
            resolve_get(stack, key, default.as_deref(), *level, *explain),
            *on_failure,
            Degrade::Suppress,
        ),
        Action::Path {
            key,
            default,
            level,
            explain,
            on_failure,
        } => finish(
            resolve_path(stack, key, default.as_deref(), *level, *explain),
            *on_failure,
            Degrade::Suppress,
        ),
        Action::Agent { name, on_failure } => {
            finish(resolve_agent(stack, name), *on_failure, Degrade::Suppress)
        }
        Action::Agents { on_failure } => finish(
            resolve_agents(stack),
            *on_failure,
            Degrade::Notice(agents_render::render_unavailable),
        ),
        Action::Work { key, on_failure } => {
            finish(resolve_work(stack, key), *on_failure, Degrade::Suppress)
        }
        Action::Context { skill, on_failure } => {
            run_context(stack, skill.as_deref(), *on_failure)
        }
        Action::Instructions { skill, on_failure } => {
            run_instructions(stack, skill, *on_failure)
        }
        Action::Paths {
            doc_types,
            all,
            on_failure,
        } => finish(
            resolve_paths(stack, *doc_types, *all),
            *on_failure,
            Degrade::Notice(paths_render::render_unavailable),
        ),
        Action::Dump { on_failure } => finish(
            resolve_dump(stack),
            *on_failure,
            Degrade::Notice(dump_render::render_unavailable),
        ),
        Action::Review { mode, on_failure } => finish(
            resolve_review(stack, *mode),
            *on_failure,
            Degrade::Notice(review_render::render_unavailable),
        ),
        Action::Summary { hook, on_failure } => finish(
            resolve_summary(stack, *hook),
            *on_failure,
            Degrade::Suppress,
        ),
        Action::Template { name, on_failure } => finish(
            resolve_template(stack, name),
            *on_failure,
            Degrade::Notice(template_render::render_unavailable),
        ),
        Action::TemplatesList { on_failure } => finish(
            resolve_templates_list(stack),
            *on_failure,
            Degrade::Notice(template_render::render_unavailable),
        ),
        Action::TemplatesShow { name, on_failure } => finish(
            resolve_templates_show(stack, name),
            *on_failure,
            Degrade::Notice(template_render::render_unavailable),
        ),
    }
}

fn resolve_template(
    stack: &ConfigStack,
    name: &str,
) -> Result<Rendered, Failure> {
    template_view::resolve(stack.config(), stack.templates(), name)?
        .map_or_else(
            || Err(Failure::Refusal(not_found(stack, name))),
            |resolved| Ok(template_render::fenced(&resolved)),
        )
}

fn resolve_templates_show(
    stack: &ConfigStack,
    name: &str,
) -> Result<Rendered, Failure> {
    template_view::resolve(stack.config(), stack.templates(), name)?
        .map_or_else(
            || Err(Failure::Refusal(not_found(stack, name))),
            |resolved| Ok(template_render::show(&resolved)),
        )
}

fn resolve_templates_list(stack: &ConfigStack) -> Result<Rendered, Failure> {
    let rows = template_view::list(stack.config(), stack.templates())?;
    Ok(template_render::list(&rows))
}

fn not_found(stack: &ConfigStack, name: &str) -> ConfigError {
    ConfigError::Invalid {
        detail: format!(
            "Template '{name}' not found. Available templates: {}",
            template_view::available(stack.templates())
        ),
    }
}

fn resolve_review(
    stack: &ConfigStack,
    mode: Mode,
) -> Result<Rendered, Failure> {
    let view = review_view::assemble(stack.config(), stack.lenses(), mode)?;
    Ok(review_render::render(&view, mode))
}

fn resolve_summary(
    stack: &ConfigStack,
    hook: bool,
) -> Result<Rendered, Failure> {
    let summary = summary_view::assemble(
        stack.config(),
        stack.levels(),
        stack.content(),
        stack.lenses(),
    )?;
    let stdout = match (summary, hook) {
        (None, _) => String::new(),
        (Some(text), false) => format!("{text}\n"),
        (Some(text), true) => {
            format!("{}\n", summary_render::hook_envelope(&text))
        }
    };
    Ok(Rendered::new(stdout))
}

fn resolve_dump(stack: &ConfigStack) -> Result<Rendered, Failure> {
    Ok(
        dump_view::assemble(stack.config(), stack.levels())?.map_or_else(
            || Rendered::new(String::new()),
            |rows| dump_render::render(&rows),
        ),
    )
}

fn resolve_paths(
    stack: &ConfigStack,
    doc_types: bool,
    all: bool,
) -> Result<Rendered, Failure> {
    if doc_types {
        let view = paths_view::doc_types(stack.config())?;
        Ok(paths_render::doc_types(&view))
    } else {
        let paths = paths_view::configured(stack.config(), all)?;
        Ok(paths_render::configured(&paths))
    }
}

/// One member of a context/instructions output: a rendered block, an
/// `Unavailable` notice absorbed under `--fail-safe`, or nothing.
enum Section {
    Block(String),
    Notice(&'static str),
    Empty,
}

/// Resolves one section, degrading a per-source failure to its notice under
/// `--fail-safe` (with the diagnostic on stderr) or propagating it otherwise.
fn section(
    assembled: Result<Option<String>, ConfigError>,
    notice: &'static str,
    on_failure: OnFailure,
) -> Result<Section, ConfigError> {
    match assembled {
        Ok(Some(block)) => Ok(Section::Block(block)),
        Ok(None) => Ok(Section::Empty),
        Err(error) if on_failure == OnFailure::Degrade => {
            eprintln!("{error}");
            Ok(Section::Notice(notice))
        }
        Err(error) => Err(error),
    }
}

/// Emits the surviving sections joined by one blank line, with a single
/// trailing newline; nothing when none survive.
fn emit_sections(sections: &[Section]) {
    let parts: Vec<&str> = sections
        .iter()
        .filter_map(|section| match section {
            Section::Block(block) => Some(block.as_str()),
            Section::Notice(header) => Some(*header),
            Section::Empty => None,
        })
        .collect();
    if !parts.is_empty() {
        println!("{}", parts.join("\n\n"));
    }
}

fn run_context(
    stack: &ConfigStack,
    skill: Option<&str>,
    on_failure: OnFailure,
) -> Result<(), ConfigError> {
    let project = section(
        context_core::project_body(stack.content())
            .map(|body| body.map(|body| context_render::project(&body))),
        context_render::PROJECT_UNAVAILABLE,
        on_failure,
    )?;
    let mut sections = vec![project];
    if let Some(name) = skill {
        let block =
            context_core::skill_body(stack.content(), name, SkillFile::Context)
                .map(|content| {
                    content.map(|body| context_render::skill(name, &body))
                });
        sections.push(section(
            block,
            context_render::SKILL_UNAVAILABLE,
            on_failure,
        )?);
    }
    emit_sections(&sections);
    Ok(())
}

fn run_instructions(
    stack: &ConfigStack,
    skill: &str,
    on_failure: OnFailure,
) -> Result<(), ConfigError> {
    let block = context_core::skill_body(
        stack.content(),
        skill,
        SkillFile::Instructions,
    )
    .map(|content| {
        content.map(|body| instructions_render::render(skill, &body))
    });
    let rendered =
        section(block, instructions_render::UNAVAILABLE, on_failure)?;
    emit_sections(&[rendered]);
    Ok(())
}

/// A handler failure, tagged by whether `--fail-safe` may degrade it.
enum Failure {
    /// A read/IO failure the fail-safe boundary may absorb.
    Read(ConfigError),
    /// A validation refusal that stays fail-closed regardless of `--fail-safe`.
    Refusal(ConfigError),
}

impl From<ConfigError> for Failure {
    fn from(error: ConfigError) -> Self {
        match error {
            ConfigError::Invalid { .. } => Self::Refusal(error),
            _ => Self::Read(error),
        }
    }
}

/// How a block degrades under `--fail-safe`: silently, or with a notice.
#[derive(Clone, Copy)]
enum Degrade {
    Suppress,
    Notice(fn() -> Rendered),
}

fn finish(
    resolved: Result<Rendered, Failure>,
    on_failure: OnFailure,
    degrade: Degrade,
) -> Result<(), ConfigError> {
    match resolved {
        Ok(rendered) => {
            render::emit(&rendered);
            Ok(())
        }
        Err(Failure::Read(error)) if on_failure == OnFailure::Degrade => {
            eprintln!("{error}");
            if let Degrade::Notice(unavailable) = degrade {
                render::emit(&unavailable());
            }
            Ok(())
        }
        Err(Failure::Read(error) | Failure::Refusal(error)) => Err(error),
    }
}

fn resolve_get(
    stack: &ConfigStack,
    raw_key: &str,
    default: Option<&str>,
    level: Option<Level>,
    explain: bool,
) -> Result<Rendered, Failure> {
    let key = Key::parse(raw_key)?;
    let value = match stack.config().get(&key, level)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => default.unwrap_or_default().to_owned(),
    };
    Ok(Rendered {
        stdout: format!("{value}\n"),
        warnings: explain_lines(stack, &key, level, explain)?,
    })
}

fn resolve_path(
    stack: &ConfigStack,
    raw_key: &str,
    default: Option<&str>,
    level: Option<Level>,
    explain: bool,
) -> Result<Rendered, Failure> {
    let full = format!("paths.{raw_key}");
    let key = Key::parse(&full)?;
    let mut warnings = Vec::new();
    let fallback = path_fallback(default, raw_key, &full, &mut warnings);
    let value = match stack.config().get(&key, level)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => fallback,
    };
    warnings.extend(explain_lines(stack, &key, level, explain)?);
    Ok(Rendered {
        stdout: format!("{value}\n"),
        warnings,
    })
}

/// The `--explain` resolution provenance for a scalar read: which level file
/// supplied the value and which files were consulted. Emitted on stderr only.
fn explain_lines(
    stack: &ConfigStack,
    key: &Key,
    level: Option<Level>,
    explain: bool,
) -> Result<Vec<String>, ConfigError> {
    if !explain {
        return Ok(Vec::new());
    }
    let mut lines = Vec::new();
    let levels: &[Level] = match level {
        Some(Level::Team) => &[Level::Team],
        Some(Level::Personal) => &[Level::Personal],
        None => &[Level::Team, Level::Personal],
    };
    let mut winner = "default";
    for probe in levels {
        let present = matches!(
            stack.config().get(key, Some(*probe))?,
            Resolved::Found(_)
        );
        lines.push(format!(
            "{probe} ({}): {}",
            level_file(*probe),
            if present { "set" } else { "not set" }
        ));
        if present {
            winner = match probe {
                Level::Team => "team",
                Level::Personal => "personal",
            };
        }
    }
    lines.push(format!("resolved from: {winner}"));
    Ok(lines)
}

const fn level_file(level: Level) -> &'static str {
    match level {
        Level::Team => ".accelerator/config.md",
        Level::Personal => ".accelerator/config.local.md",
    }
}

/// The value a `path` miss falls back to: an explicit non-empty default wins,
/// else the catalogue default, else empty with a stderr warning naming the key.
fn path_fallback(
    default: Option<&str>,
    raw_key: &str,
    full_key: &str,
    warnings: &mut Vec<String>,
) -> String {
    if let Some(explicit) = default.filter(|value| !value.is_empty()) {
        return explicit.to_owned();
    }
    if let Some(value) = catalogue::default_for(full_key) {
        return config::render_value(&value);
    }
    warnings.push(unknown_path_key_warning(raw_key));
    String::new()
}

fn unknown_path_key_warning(key: &str) -> String {
    match key {
        "design_inventories" | "design_gaps" => format!(
            "accelerator config path: key '{key}' was renamed by migration \
             0004 to 'research_{key}'; run /accelerator:migrate"
        ),
        _ => format!(
            "accelerator config path: unknown key '{key}' — no centralized \
             default"
        ),
    }
}

fn resolve_agent(stack: &ConfigStack, name: &str) -> Result<Rendered, Failure> {
    let key = Key::parse(&format!("agents.{name}"))?;
    let value = match stack.config().get(&key, None)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => format!("{}{name}", catalogue::AGENT_PREFIX),
    };
    Ok(Rendered::new(format!("{value}\n")))
}

fn resolve_agents(stack: &ConfigStack) -> Result<Rendered, Failure> {
    let view = agents_view::assemble(stack.config(), stack.levels())?;
    Ok(agents_render::render(&view))
}

fn resolve_work(stack: &ConfigStack, key: &str) -> Result<Rendered, Failure> {
    let full = format!("work.{key}");
    let parsed = Key::parse(&full)?;
    let mut warnings = Vec::new();
    let fallback = work_fallback(key, &full, &mut warnings);
    let value = match stack.config().get(&parsed, None)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => fallback,
    };
    if key == "integration"
        && !value.is_empty()
        && !catalogue::WORK_INTEGRATION_VALUES.contains(&value.as_str())
    {
        return Err(Failure::Refusal(bad_integration(&value)));
    }
    Ok(Rendered {
        stdout: format!("{value}\n"),
        warnings,
    })
}

/// The default a `work` miss falls back to: the catalogue default, else empty
/// with a stderr warning naming the unrecognised key.
fn work_fallback(
    key: &str,
    full_key: &str,
    warnings: &mut Vec<String>,
) -> String {
    if let Some(value) = catalogue::default_for(full_key) {
        return config::render_value(&value);
    }
    warnings.push(unknown_work_key_warning(key));
    String::new()
}

fn unknown_work_key_warning(key: &str) -> String {
    format!(
        "accelerator config work: unknown key 'work.{key}' — no centralized \
         default"
    )
}

fn bad_integration(value: &str) -> ConfigError {
    let allowed = catalogue::WORK_INTEGRATION_VALUES.join(", ");
    ConfigError::Invalid {
        detail: format!(
            "work.integration must be one of: {allowed} (got '{value}'). \
             Update work.integration in .accelerator/config.md."
        ),
    }
}

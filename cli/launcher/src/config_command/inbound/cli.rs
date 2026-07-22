//! Maps a parsed `config` request onto the injected core and presents the
//! result.
//!
//! Each handler resolves a [`Rendered`] or a [`Failure`]. A `Failure::Read` is a
//! read/IO failure the fail-safe boundary may degrade — by suppression for the
//! scalars and `work`, by a `## <Name> Unavailable` notice for the block
//! commands. A `Failure::Refusal` is a validation refusal that stays fail-closed
//! regardless of `--fail-safe`, so a bad `work.integration` enum is never
//! papered over into empty-and-exit-0.

use config::{ConfigError, EjectOutcome, Key, Level, TemplateSource};

use crate::config_command::core::context::{self as context_core, SkillFile};
use crate::config_command::core::review::{self as review_view, Mode};
use crate::config_command::core::{
    agents as agents_view, dump as dump_view, get as get_view,
    init as init_view, paths as paths_view, summary as summary_view,
    template as template_view, work as work_view, ConfigStack, OnFailure,
    ScalarView,
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
    Set {
        key: String,
        value: String,
        level: Level,
    },
    Init,
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
    TemplatesEject {
        name: Option<String>,
        all: bool,
        force: bool,
        dry_run: bool,
    },
    TemplatesDiff {
        name: String,
    },
    TemplatesReset {
        name: String,
        confirm: bool,
    },
}

/// Runs a parsed `config` request against the composed stack.
///
/// The write subcommands that overload exit code 2 (`templates eject`, `diff`
/// and `reset`) map their refusals to [`kernel::Error::Refusal`]; everything
/// else fails through [`kernel::Error::Failed`].
///
/// # Errors
///
/// A [`kernel::Error`] when a read fails and the request's `on_failure` is
/// [`OnFailure::Fail`], or when a validation or confirmation refusal fires.
pub fn run(stack: &ConfigStack, action: &Action) -> Result<(), kernel::Error> {
    match action {
        Action::TemplatesEject {
            name,
            all,
            force,
            dry_run,
        } => run_eject(stack, name.as_deref(), *all, *force, *dry_run),
        Action::TemplatesDiff { name } => run_diff(stack, name),
        Action::TemplatesReset { name, confirm } => {
            run_reset(stack, name, *confirm)
        }
        _ => run_read(stack, action).map_err(kernel::Error::from),
    }
}

fn run_read(stack: &ConfigStack, action: &Action) -> Result<(), ConfigError> {
    match action {
        Action::Get {
            key,
            default,
            level,
            explain,
            on_failure,
        } => finish_scalar(
            get_view::resolve(
                stack.config(),
                key,
                default.as_deref(),
                *level,
                *explain,
            ),
            *on_failure,
        ),
        Action::Path {
            key,
            default,
            level,
            explain,
            on_failure,
        } => finish_scalar(
            paths_view::resolve(
                stack.config(),
                key,
                default.as_deref(),
                *level,
                *explain,
            ),
            *on_failure,
        ),
        Action::Set { key, value, level } => run_set(stack, key, value, *level),
        Action::Init => init_view::run(stack.config(), stack.scaffold()),
        Action::Agent { name, on_failure } => finish_scalar(
            agents_view::resolve(stack.config(), name),
            *on_failure,
        ),
        Action::Agents { on_failure } => finish(
            resolve_agents(stack),
            *on_failure,
            Degrade::Notice(agents_render::render_unavailable),
        ),
        Action::Work { key, on_failure } => {
            finish_scalar(work_view::resolve(stack.config(), key), *on_failure)
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
        Action::TemplatesEject { .. }
        | Action::TemplatesDiff { .. }
        | Action::TemplatesReset { .. } => unreachable!(),
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
    let (summary, warnings) = summary_view::assemble(
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
    Ok(Rendered { stdout, warnings })
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

/// Renders a scalar view to stdout-plus-warnings and dispatches it through
/// [`finish`] with the scalar suppression policy. A [`ConfigError::Invalid`]
/// stays a fail-closed refusal via [`From<ConfigError>`].
fn finish_scalar(
    view: Result<ScalarView, ConfigError>,
    on_failure: OnFailure,
) -> Result<(), ConfigError> {
    let rendered = view.map_err(Failure::from).map(|view| Rendered {
        stdout: format!("{}\n", view.value),
        warnings: view.warnings,
    });
    finish(rendered, on_failure, Degrade::Suppress)
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

/// Writes a value at a key, silent on success. Fails closed and loud (no
/// `--fail-safe`): a write is never a prompt-splice site.
fn run_set(
    stack: &ConfigStack,
    raw_key: &str,
    value: &str,
    level: Level,
) -> Result<(), ConfigError> {
    let key = Key::parse(raw_key)?;
    stack.config().set(&key, value, level)
}

/// Copies plugin default templates into the user directory. A single name
/// reports its own outcome; `--all` walks every template and aggregates.
fn run_eject(
    stack: &ConfigStack,
    name: Option<&str>,
    all: bool,
    force: bool,
    dry_run: bool,
) -> Result<(), kernel::Error> {
    let dir = template_view::templates_dir(stack.config())?;
    if all {
        return eject_all(stack, &dir, force, dry_run);
    }
    let Some(name) = name else {
        return Err(kernel::Error::Failed(
            "config templates eject requires a template name or --all"
                .to_owned(),
        ));
    };
    template_view::validate(name)?;
    let available = template_view::available_or_none(stack.templates());
    let result = stack.overrides().eject(name, &dir, force, dry_run)?;
    let text = template_render::eject_text(
        result.outcome,
        &result.key,
        &result.display,
        &available,
    );
    match result.outcome {
        EjectOutcome::Ejected
        | EjectOutcome::Overwritten
        | EjectOutcome::WouldEject
        | EjectOutcome::WouldOverwrite => {
            println!("{text}");
            Ok(())
        }
        EjectOutcome::WouldSkip => {
            println!("{text}");
            Err(kernel::Error::Refusal(String::new()))
        }
        EjectOutcome::Exists => Err(kernel::Error::Refusal(text)),
        EjectOutcome::NoDefault => Err(kernel::Error::Failed(text)),
    }
}

/// Ejects every template, printing each outcome as it lands; any error wins the
/// exit code over any already-exists.
fn eject_all(
    stack: &ConfigStack,
    dir: &str,
    force: bool,
    dry_run: bool,
) -> Result<(), kernel::Error> {
    let available = template_view::available_or_none(stack.templates());
    let mut had_error = false;
    let mut had_exists = false;
    for key in stack.templates().template_names() {
        let result = stack.overrides().eject(&key, dir, force, dry_run)?;
        let text = template_render::eject_text(
            result.outcome,
            &result.key,
            &result.display,
            &available,
        );
        match result.outcome {
            EjectOutcome::Exists | EjectOutcome::NoDefault => {
                eprintln!("{text}");
            }
            _ => println!("{text}"),
        }
        match result.outcome {
            EjectOutcome::NoDefault => had_error = true,
            EjectOutcome::Exists | EjectOutcome::WouldSkip => had_exists = true,
            _ => {}
        }
    }
    if had_error {
        Err(kernel::Error::Failed(
            template_render::EJECT_ALL_ERROR.to_owned(),
        ))
    } else if had_exists {
        Err(kernel::Error::Refusal(
            template_render::EJECT_ALL_EXISTS.to_owned(),
        ))
    } else {
        Ok(())
    }
}

/// Shows the unified diff between a customised template and the plugin default.
/// Exits 2 when there is no override to compare.
fn run_diff(stack: &ConfigStack, name: &str) -> Result<(), kernel::Error> {
    template_view::validate(name)?;
    let Some(default) = stack.templates().plugin_default(name)? else {
        return Err(kernel::Error::Failed(unknown_template(stack, name)));
    };
    let user = template_view::resolve(stack.config(), stack.templates(), name)?
        .filter(|resolved| resolved.source != TemplateSource::PluginDefault);
    let Some(user) = user else {
        return Err(kernel::Error::Refusal(format!(
            "No customised template found for '{name}' — using plugin default."
        )));
    };
    print!("{}", template_render::diff_report(&default, &user));
    Ok(())
}

/// Reports (or, with `confirm`, deletes) the override that shadows a plugin
/// default. Exits 2 when there is no override.
fn run_reset(
    stack: &ConfigStack,
    name: &str,
    confirm: bool,
) -> Result<(), kernel::Error> {
    template_view::validate(name)?;
    if stack.templates().plugin_default(name)?.is_none() {
        return Err(kernel::Error::Failed(unknown_template(stack, name)));
    }
    let resolved =
        template_view::resolve(stack.config(), stack.templates(), name)?
            .filter(|resolved| {
                resolved.source != TemplateSource::PluginDefault
            });
    let Some(resolved) = resolved else {
        return Err(kernel::Error::Refusal(format!(
            "No customised template found for '{name}' — already using \
             plugin default."
        )));
    };
    if confirm {
        stack.overrides().delete(&resolved.abs_path)?;
        print!(
            "{}",
            template_render::reset_confirmed(resolved.source, name)
        );
    } else {
        let within = stack.overrides().within_project(&resolved.abs_path);
        print!("{}", template_render::reset_found(&resolved, within, name));
    }
    Ok(())
}

fn unknown_template(stack: &ConfigStack, name: &str) -> String {
    format!(
        "Error: Unknown template '{name}'. Available: {}",
        template_view::available_or_none(stack.templates())
    )
}

fn resolve_agents(stack: &ConfigStack) -> Result<Rendered, Failure> {
    let view = agents_view::assemble(stack.config(), stack.levels())?;
    Ok(agents_render::render(&view))
}

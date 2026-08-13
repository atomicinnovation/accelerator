//! `accelerator-work` — the `work create|show|resolve|diff|update`
//! sub-binary, dispatched by the `accelerator` launcher.

mod canonicalise_id;
mod cli;
mod config;
mod create;
mod diff;
mod exit_codes;
mod next_number;
mod resolve;
mod show;
mod template_hints;
mod update;

use std::path::Path;
use std::process::ExitCode;

use ::config::ConfigAccess;
use clap::Parser as _;
use config_adapters::compose;
use config_adapters::plugin_root_from_env;
use config_adapters::LegacyPolicy;

use crate::cli::Cli;
use crate::cli::Command;
use crate::resolve::RunOutcome;

fn current_dir() -> Result<std::path::PathBuf, kernel::Error> {
    std::env::current_dir().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        ))
    })
}

fn run_resolve(input: &str) -> ExitCode {
    let start = match current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let composed = match compose(&start, LegacyPolicy::Reject) {
        Ok(composed) => composed,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let service: &dyn ConfigAccess = &composed.service;

    match resolve::run(&start, service, input) {
        Ok(RunOutcome::Resolved(path)) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Ok(RunOutcome::Ambiguous(candidates)) => {
            eprintln!(
                "E_RESOLVE_AMBIGUOUS: multiple work items match '{input}':"
            );
            for candidate in candidates {
                if candidate.tag.is_empty() {
                    eprintln!("  {}", candidate.path);
                } else {
                    eprintln!("  {} [{}]", candidate.path, candidate.tag);
                }
            }
            ExitCode::from(exit_codes::USAGE)
        }
        Ok(RunOutcome::NotFound(message)) => {
            eprintln!("E_RESOLVE_NOT_FOUND: {message}");
            ExitCode::from(exit_codes::RESOLVE_NOT_FOUND)
        }
        Ok(RunOutcome::Invalid(message)) => {
            eprintln!("E_RESOLVE_INVALID: {message}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_template_hints(field: &str) -> ExitCode {
    let start = match current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let composed = match compose(&start, LegacyPolicy::Reject) {
        Ok(composed) => composed,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let service: &dyn ConfigAccess = &composed.service;
    let store = composed.store.with_plugin_root(plugin_root_from_env());
    template_hints::run(service, &store, field);
    ExitCode::SUCCESS
}

fn run_show(path: &Path, field: Option<&str>) -> ExitCode {
    use crate::show::RunOutcome as ShowOutcome;
    let basename = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    match show::run(path, field) {
        ShowOutcome::FullFile(content) => {
            print!("{content}");
            ExitCode::SUCCESS
        }
        ShowOutcome::FieldValue(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        ShowOutcome::MissingFile => {
            eprintln!("Error: File not found: {}", path.display());
            ExitCode::FAILURE
        }
        ShowOutcome::NoFrontmatter => {
            eprintln!(
                "Error: No YAML frontmatter in {basename}. Add a '---' \
                 line as the first line of the file."
            );
            ExitCode::FAILURE
        }
        ShowOutcome::UnclosedFrontmatter => {
            eprintln!(
                "Error: YAML frontmatter opened but not closed in \
                 {basename}. Add a '---' line after the last frontmatter \
                 key."
            );
            ExitCode::FAILURE
        }
        ShowOutcome::FieldMissing => {
            let field = field.unwrap_or_default();
            eprintln!(
                "Error: No '{field}' field found in frontmatter of \
                 {basename}."
            );
            ExitCode::FAILURE
        }
    }
}

fn run_diff(local: &Path, remote: &Path) -> ExitCode {
    match diff::run(local, remote) {
        diff::RunOutcome::Rendered(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        diff::RunOutcome::NonFileArgument => {
            eprintln!("work-item-section-diff: both arguments must be files");
            ExitCode::from(2)
        }
        diff::RunOutcome::DiffUnavailable => {
            eprintln!(
                "`diff` is required on PATH for `work diff`; install it or \
                 check PATH"
            );
            ExitCode::FAILURE
        }
    }
}

fn create_args_from_cli(cli_args: cli::CreateArgs) -> create::CreateArgs {
    create::CreateArgs {
        title: cli_args.title,
        kind: cli_args.kind,
        priority: cli_args.priority,
        status: cli_args.status,
        parent: cli_args.parent,
        tags: cli_args.tags,
        blocks: cli_args.blocks,
        blocked_by: cli_args.blocked_by,
        derived_from: cli_args.derived_from,
        relates_to: cli_args.relates_to,
        source: cli_args.source,
        project: cli_args.project,
        author: cli_args.author,
        producer: cli_args.producer,
        body_file: cli_args.body_file,
    }
}

fn run_create(cli_args: cli::CreateArgs) -> ExitCode {
    let start = match current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let composed = match compose(&start, LegacyPolicy::Reject) {
        Ok(composed) => composed,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let service: &dyn ConfigAccess = &composed.service;
    let store = composed.store.with_plugin_root(plugin_root_from_env());
    let args = create_args_from_cli(cli_args);

    match create::run(&start, service, &store, &args) {
        create::RunOutcome::Created(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        create::RunOutcome::Failed(message) => {
            eprintln!("Error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run_update(cli_args: &cli::UpdateArgs) -> ExitCode {
    match update::run(cli_args) {
        update::RunOutcome::Updated => ExitCode::SUCCESS,
        update::RunOutcome::Failed(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run_canonicalise_id(input: &str) -> ExitCode {
    let start = match current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let composed = match compose(&start, LegacyPolicy::Reject) {
        Ok(composed) => composed,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let service: &dyn ConfigAccess = &composed.service;
    match canonicalise_id::run(service, input) {
        canonicalise_id::RunOutcome::Canonicalised(id) => {
            println!("{id}");
            ExitCode::SUCCESS
        }
        canonicalise_id::RunOutcome::Failed(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run_next_number(project: Option<&str>, count: u32) -> ExitCode {
    let start = match current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let composed = match compose(&start, LegacyPolicy::Reject) {
        Ok(composed) => composed,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let service: &dyn ConfigAccess = &composed.service;
    match next_number::run(&start, service, project, count) {
        next_number::RunOutcome::Allocated(ids) => {
            for id in ids {
                println!("{id}");
            }
            ExitCode::SUCCESS
        }
        next_number::RunOutcome::Overflow { partial, message } => {
            for id in partial {
                println!("{id}");
            }
            eprintln!("{message}");
            ExitCode::FAILURE
        }
        next_number::RunOutcome::Failed(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Resolve { input } => run_resolve(&input),
        Command::TemplateHints { field } => run_template_hints(&field),
        Command::Show { path, field } => run_show(&path, field.as_deref()),
        Command::Diff { local, remote } => run_diff(&local, &remote),
        Command::Create(args) => run_create(*args),
        Command::Update(args) => run_update(&args),
        Command::CanonicaliseId { input } => run_canonicalise_id(&input),
        Command::NextNumber { project, count } => {
            run_next_number(project.as_deref(), count)
        }
    }
}

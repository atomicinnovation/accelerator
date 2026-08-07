//! `accelerator-work` — the `work create|show|resolve|diff|update`
//! sub-binary, dispatched by the `accelerator` launcher.

mod cli;
mod diff;
mod resolve;
mod show;
mod template_hints;

use std::path::Path;
use std::process::ExitCode;

use clap::Parser as _;
use config::ConfigAccess;
use config_adapters::compose;
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
            ExitCode::from(2)
        }
        Ok(RunOutcome::NotFound(message)) => {
            eprintln!("E_RESOLVE_NOT_FOUND: {message}");
            ExitCode::from(3)
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
    template_hints::run(service, &composed.store, field);
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Resolve { input } => run_resolve(&input),
        Command::TemplateHints { field } => run_template_hints(&field),
        Command::Show { path, field } => run_show(&path, field.as_deref()),
        Command::Diff { local, remote } => run_diff(&local, &remote),
    }
}

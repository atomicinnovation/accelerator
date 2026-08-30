//! `accelerator-corpus` — the `corpus adr|metadata|linkage|frontmatter`
//! sub-binary, dispatched by the `accelerator` launcher.

mod adr;
mod cli;
mod config;
mod frontmatter;
mod linkage;
mod metadata;
mod outcome;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser as _;
use corpus::FilenameTimestampFormat;
use corpus_adapters::frontmatter_validation::Checks;
use corpus_adapters::RealFs;

use crate::cli::AdrAction;
use crate::cli::CheckKind;
use crate::cli::Cli;
use crate::cli::Command;
use crate::cli::FrontmatterAction;
use crate::cli::LinkageAction;
use crate::cli::MetadataAction;
use crate::outcome::Outcome;

fn current_dir() -> Result<PathBuf, kernel::Error> {
    std::env::current_dir().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        ))
    })
}

fn next_number(count: &str) -> Result<Outcome, kernel::Error> {
    let composed = config::compose(&current_dir()?)?;
    let decisions_dir = config::resolve_decisions_dir(&composed)?;
    adr::run_next_number(count, &decisions_dir, &RealFs)
}

/// Degrades a failure to a silent, exit-0 empty [`Outcome`] under
/// `fail_safe` — printing the diagnostic to stderr first, matching
/// `accelerator config path --fail-safe`'s scalar-suppression policy —
/// rather than propagating it.
fn run_next_number(
    count: &str,
    fail_safe: bool,
) -> Result<Outcome, kernel::Error> {
    match next_number(count) {
        Ok(outcome) => Ok(outcome),
        Err(error) if fail_safe => {
            eprintln!("{error}");
            Ok(Outcome::default())
        }
        Err(error) => Err(error),
    }
}

fn run_adr(action: AdrAction) -> Result<Outcome, kernel::Error> {
    match action {
        AdrAction::NextNumber { count, fail_safe } => {
            run_next_number(&count, fail_safe)
        }
        AdrAction::ReadStatus { file } => {
            adr::run_read_status(file.as_deref(), &RealFs)
        }
    }
}

fn run_metadata(action: &MetadataAction) -> Result<Outcome, kernel::Error> {
    match action {
        MetadataAction::Derive {
            filename_timestamp_format,
        } => {
            let format =
                FilenameTimestampFormat::from(*filename_timestamp_format);
            let derived = corpus_adapters::metadata::derive_at(
                &current_dir()?,
                format,
                &corpus_adapters::metadata::VcsBackedRepoFactsProbe,
            );
            metadata::run_derive(derived, format)
        }
    }
}

fn run_linkage(action: LinkageAction) -> Result<Outcome, kernel::Error> {
    match action {
        LinkageAction::Extract { file, source_type } => {
            let composed = config::compose(&current_dir()?)?;
            let table = config::table_from_config(&composed.service)?;
            linkage::run_extract(&file, source_type.as_deref(), &table, &RealFs)
        }
    }
}

fn resolve_checks(kinds: &[CheckKind]) -> Checks {
    Checks {
        structure: kinds.contains(&CheckKind::Structure),
        references: kinds.contains(&CheckKind::References),
        canonical: kinds.contains(&CheckKind::Structure),
    }
}

fn run_frontmatter(
    action: FrontmatterAction,
) -> Result<Outcome, kernel::Error> {
    match action {
        FrontmatterAction::Validate { dir, file, checks } => {
            let composed = config::compose(&current_dir()?)?;
            let table: Vec<_> = config::table_from_config(&composed.service)?
                .into_iter()
                .map(|(kind, dir)| (kind, composed.project_root.join(dir)))
                .collect();
            frontmatter::run_validate(
                &dir,
                &file,
                resolve_checks(&checks),
                &table,
                &RealFs,
            )
        }
        FrontmatterAction::ValidateTemplates => {
            let composed = config::compose(&current_dir()?)?;
            frontmatter::run_validate_templates(&composed.project_root, &RealFs)
        }
        FrontmatterAction::PrintSchema => Ok(frontmatter::run_print_schema()),
    }
}

fn report(error: &kernel::Error) -> ExitCode {
    let message = error.to_string();
    if !message.is_empty() {
        eprintln!("{message}");
    }
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Adr { action } => run_adr(action),
        Command::Metadata { action } => run_metadata(&action),
        Command::Linkage { action } => run_linkage(action),
        Command::Frontmatter { action } => run_frontmatter(action),
    };
    match result {
        Ok(outcome) => {
            print!("{}", outcome.stdout);
            eprint!("{}", outcome.stderr);
            ExitCode::SUCCESS
        }
        Err(error) => report(&error),
    }
}

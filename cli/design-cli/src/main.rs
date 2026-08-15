//! `accelerator-design` — the `design validate-source|resolve-auth|
//! scrub-secrets|notify-downgrade|audit-cue-phrases` sub-binary, dispatched by
//! the `accelerator` launcher.

mod cli;
mod commands;
mod config;
mod executor;
mod report;

use std::process::ExitCode;

use clap::Parser as _;
use design::Allowances;
use design::DowngradeReason;
use design_adapters::filesystem;
use design_adapters::CompiledCuePhrases;

use crate::cli::Cli;
use crate::cli::Command;
use crate::report::Report;

fn run(command: Command) -> Result<Report, kernel::Error> {
    match command {
        Command::ValidateSource {
            location,
            allow_internal,
            allow_insecure_scheme,
        } => Ok(commands::validate_source(
            &location,
            Allowances {
                internal: allow_internal,
                insecure_scheme: allow_insecure_scheme,
            },
            &filesystem::check_directory,
        )),
        Command::ResolveAuth => {
            commands::resolve_auth(&design_adapters::credentials_from_env())
        }
        Command::ScrubSecrets { file } => commands::scrub_secrets(
            &file,
            &design_adapters::named_secrets_from_env(),
        ),
        Command::NotifyDowngrade { reason, .. } => {
            Ok(commands::notify_downgrade(DowngradeReason::from(reason)))
        }
        Command::AuditCuePhrases { file } => {
            commands::audit_cue_phrases(&file, &CompiledCuePhrases::new()?)
        }
        // Handled before dispatch: it never returns a Report.
        Command::Executor { .. } => unreachable!("dispatched in main"),
    }
}

/// Usage errors reach exit 2 through `Refusal`, matching every other
/// sub-binary; everything else is an internal failure at exit 1.
fn report_error(error: &kernel::Error) -> ExitCode {
    let message = error.to_string();
    if !message.is_empty() {
        eprintln!("error: {message}");
    }
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
}

fn main() -> ExitCode {
    let command = Cli::parse().command;
    // The executor owns its own exit status: on the success path it replaces
    // the process image with the client, so there is no `Report` for it to
    // return through.
    if let Command::Executor { command, arguments } = command {
        return executor::run(&command, &arguments);
    }
    match run(command) {
        Ok(Report::Accepted { stdout, stderr }) => {
            print!("{stdout}");
            eprint!("{stderr}");
            ExitCode::SUCCESS
        }
        Ok(Report::Rejected { stderr }) => {
            eprint!("{stderr}");
            ExitCode::FAILURE
        }
        Err(error) => report_error(&error),
    }
}

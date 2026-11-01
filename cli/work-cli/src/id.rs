//! The `work id` verb group: canonicalising an existing item's ID and
//! previewing the IDs allocation would give next.

mod canonicalise;
mod next;

use std::process::ExitCode;

use ::config::ConfigAccess;

use crate::cli::IdAction;
use crate::composed_here;
use crate::exit_codes;
use crate::Intent;

pub fn run(action: IdAction) -> ExitCode {
    match action {
        IdAction::Canonicalise { input } => run_canonicalise(&input),
        IdAction::Next { project, count } => {
            run_next(project.as_deref(), count)
        }
    }
}

/// Names a retired verb's replacement and runs nothing.
pub fn retired(pointer: &str) -> ExitCode {
    eprintln!("{pointer}");
    ExitCode::from(exit_codes::USAGE)
}

fn run_canonicalise(input: &str) -> ExitCode {
    let (_, composed) = match composed_here(Intent::Read) {
        Ok(composed) => composed,
        Err(code) => return code,
    };
    let service: &dyn ConfigAccess = &composed.service;
    match canonicalise::run(service, input) {
        canonicalise::RunOutcome::Canonicalised(id) => {
            println!("{id}");
            ExitCode::SUCCESS
        }
        canonicalise::RunOutcome::Failed(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run_next(project: Option<&str>, count: u32) -> ExitCode {
    let (start, composed) = match composed_here(Intent::Read) {
        Ok(composed) => composed,
        Err(code) => return code,
    };
    let service: &dyn ConfigAccess = &composed.service;
    match next::run(
        &start,
        service,
        project,
        count,
        &crate::identity_run::begin,
    ) {
        next::RunOutcome::Allocated(ids) => {
            for id in ids {
                println!("{id}");
            }
            ExitCode::SUCCESS
        }
        next::RunOutcome::Overflow { partial, message } => {
            for id in partial {
                println!("{id}");
            }
            eprintln!("{message}");
            ExitCode::FAILURE
        }
        next::RunOutcome::Failed(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
        next::RunOutcome::Refused { message, code } => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

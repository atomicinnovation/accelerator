//! `accelerator-migrate` — applies pending meta-directory schema migrations,
//! dispatched by the `accelerator` launcher.

mod cli;
mod render;

use std::process::ExitCode;

use clap::Parser as _;
use config_adapters::FileConfigStore;
use migrate::ledger;
use migrate_adapters::context::FileMigrationContext;
use migrate_adapters::ledger_store::FileLedgerStore;

use crate::cli::Cli;
use crate::render::StdoutReporter;

fn project_root() -> Result<std::path::PathBuf, kernel::Error> {
    let cwd = std::env::current_dir().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        ))
    })?;
    Ok(FileConfigStore::discover_root(&cwd))
}

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    let root = project_root()?;
    let ledger_store = FileLedgerStore::new(&root);

    if let Some(id) = &cli.skip {
        ledger::skip(&ledger_store, id)?;
        println!("Skipped migration: {id}");
        return Ok(());
    }
    if let Some(id) = &cli.unskip {
        ledger::unskip(&ledger_store, id)?;
        println!("Unskipped migration: {id}");
        return Ok(());
    }
    if let Some(id) = &cli.unapply {
        ledger::unapply(&ledger_store, id)?;
        println!("Unapplied migration: {id}");
        return Ok(());
    }
    if cli.list || cli.decisions_file.is_some() {
        return Err(kernel::Error::Failed("not yet implemented".to_owned()));
    }

    let ctx = FileMigrationContext::new(&root);
    let reporter = StdoutReporter;
    let entries = migrate::registry::registry();
    migrate::lifecycle::run_pending(&entries, &ctx, &ledger_store, &reporter)?;
    Ok(())
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
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error),
    }
}

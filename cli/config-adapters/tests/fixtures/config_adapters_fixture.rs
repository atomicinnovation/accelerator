//! Composition-root demonstration and black-box test entry point. Constructs
//! its own adapters via `compose`, enforces the fail-closed legacy guard, and
//! resolves `paths.work`. Not a shipped artifact.

#![allow(
    clippy::exit,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::restriction
)]

use std::process::ExitCode;

use config::{ConfigAccess, Key, Resolved};

fn main() -> ExitCode {
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let service = match config_adapters::compose(&cwd) {
        Ok(service) => service,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    match service.get(
        &Key::parse("paths.work").expect("constant key parses"),
        None,
    ) {
        Ok(Resolved::Found(value)) => {
            println!("{}", config_adapters::render_value(&value));
            ExitCode::SUCCESS
        }
        Ok(Resolved::Absent) => {
            eprintln!("paths.work not set");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

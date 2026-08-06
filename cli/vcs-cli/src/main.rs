//! `accelerator-vcs` — the `vcs detect|status|log|guard` sub-binary,
//! dispatched by the `accelerator` launcher.

mod cli;
mod detect;
mod guard;
mod log;
mod status;

use std::io::Read as _;
use std::process::ExitCode;

use clap::Parser as _;
use vcs_adapters::library::InProcessProbe;

use crate::cli::Cli;
use crate::cli::Command;

fn current_dir() -> Result<std::path::PathBuf, kernel::Error> {
    std::env::current_dir().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        ))
    })
}

fn run_detect(descriptive: bool, fail_safe: bool) -> Result<(), kernel::Error> {
    let start = current_dir()?;
    let probe = InProcessProbe;
    if let Some(output) = detect::run(&start, &probe, descriptive, fail_safe)? {
        println!("{output}");
    }
    Ok(())
}

fn run_status() -> Result<(), kernel::Error> {
    println!("{}", status::run(&current_dir()?));
    Ok(())
}

fn run_log() -> Result<(), kernel::Error> {
    println!("{}", log::run(&current_dir()?));
    Ok(())
}

/// The tool call's command, read from stdin as `PreToolUse`'s own JSON
/// envelope. `None` for anything that isn't a non-empty `.tool_input.command`
/// string — an unreadable stdin, invalid JSON, or a missing/empty field —
/// mirroring the shell's own silent-allow behaviour on the equivalent `jq`
/// failure.
fn read_command_from_stdin() -> Option<String> {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).ok()?;
    let input: serde_json::Value = serde_json::from_str(&buffer).ok()?;
    let command = input.get("tool_input")?.get("command")?.as_str()?;
    (!command.is_empty()).then(|| command.to_owned())
}

fn run_guard(fail_safe: bool) -> Result<(), kernel::Error> {
    let start = current_dir()?;
    let probe = InProcessProbe;
    let command = read_command_from_stdin();
    if let Some(output) =
        guard::run(&start, &probe, command.as_deref(), fail_safe)?
    {
        println!("{output}");
    }
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
    let result = match cli.command {
        Command::Detect {
            format: _,
            descriptive,
            fail_safe,
        } => run_detect(descriptive, fail_safe),
        Command::Status { fail_safe: _ } => run_status(),
        Command::Log { fail_safe: _ } => run_log(),
        Command::Guard {
            format: _,
            fail_safe,
        } => run_guard(fail_safe),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error),
    }
}

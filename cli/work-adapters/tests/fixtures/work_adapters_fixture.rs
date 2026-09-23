//! Black-box test entry point for `VcsWorkingCopyStatus`, run in a child
//! process so a test can give it a hermetic environment. Not a shipped
//! artifact.
//!
//! Usage: `work-adapters-fixture <start> <path>...`, printing
//! `<path>\t<dirty|clean|unknown>` for each path.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::restriction)]

use std::path::Path;
use std::process::ExitCode;

use work::sync::Dirtiness;
use work_adapters::sync::fetch::WorkingCopyStatus as _;
use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

fn main() -> ExitCode {
    if let Err(error) = kernel::logging::init_if_requested() {
        eprintln!("{error}");
    }
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((start, paths)) = arguments.split_first() else {
        eprintln!("usage: work-adapters-fixture <start> <path>...");
        return ExitCode::from(2);
    };
    let status = VcsWorkingCopyStatus::probed_from(Path::new(start));
    for path in paths {
        let dirtiness = match status.is_dirty(Path::new(path)) {
            Dirtiness::Dirty => "dirty",
            Dirtiness::Clean => "clean",
            Dirtiness::Unknown => "unknown",
        };
        println!("{path}\t{dirtiness}");
    }
    ExitCode::SUCCESS
}

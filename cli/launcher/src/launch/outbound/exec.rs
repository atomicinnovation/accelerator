//! The Unix `exec` outbound adapter — process-replacing dispatch so the child's
//! exit code and terminating signal propagate to the launcher's caller.

use std::ffi::OsString;
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::process::Command;

use crate::launch::core::{ExecBinary, ResolutionError};

/// Replaces the current process image with the resolved binary.
pub struct UnixExec;

impl ExecBinary for UnixExec {
    fn exec(&self, program: &Path, args: &[OsString]) -> ResolutionError {
        // `exec` returns ONLY on failure; on success the process image is
        // replaced and control never reaches the wrap below.
        let source = Command::new(program).args(args).exec();
        ResolutionError::Exec {
            program: program.to_path_buf(),
            source,
        }
    }
}

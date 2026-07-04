//! The Phase 1 resolver: the `ACCELERATOR_<SUB>_BIN` override or nothing.
//!
//! It is the driven `ResolveBinary` adapter dispatch and exec are proven against
//! before the real fetch → verify → cache adapter lands: it honours the offline
//! override escape hatch and otherwise reports the subcommand unresolved (no
//! network). The real resolver applies the *same* override step first.

use std::path::PathBuf;

use crate::launch::core::{ExternalCommand, ResolutionError, ResolveBinary};
use crate::launch::outbound::override_path;

/// Resolves a subcommand to its `ACCELERATOR_<SUB>_BIN` override if set, else
/// reports it unresolved.
pub struct EnvOverrideResolver;

impl ResolveBinary for EnvOverrideResolver {
    fn resolve(
        &self,
        command: &ExternalCommand,
    ) -> Result<PathBuf, ResolutionError> {
        if let Some(path) = override_path(&command.name)? {
            return Ok(path);
        }
        Err(ResolutionError::Unresolved {
            name: command.name.clone(),
        })
    }
}

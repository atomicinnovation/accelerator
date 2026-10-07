//! The interactive session log's location:
//! `.accelerator/state/migrations-<id>-session.jsonl`.

use std::path::Path;
use std::path::PathBuf;

#[must_use]
pub fn session_log_path(root: &Path, migration_id: &str) -> PathBuf {
    root.join(format!(
        ".accelerator/state/migrations-{migration_id}-session.jsonl"
    ))
}

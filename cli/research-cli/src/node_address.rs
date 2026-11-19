//! Where a node of a pair's research tree lives, as both a plan's node entry
//! and the node query print it, so a note written to one agrees with the
//! other.

use std::path::Path;

use research::topic::layout::note_ref::NoteRef;
use research::topic::tree::follow_up_cap;
use serde_json::json;
use serde_json::Value;

/// The node at `at` in the set at `set_root`: its lineage, level, follow-up
/// cap, note id and absolute note path.
pub fn addressed(set_root: &Path, at: &NoteRef) -> Value {
    let level = at.lineage.level();
    json!({
        "lineage": at.lineage.to_string(),
        "level": level,
        "cap": follow_up_cap(level),
        "id": at.id(&set_slug(set_root)),
        "path": set_root.join(at.path()).to_string_lossy(),
    })
}

/// The name a set's note ids are scoped by: its directory's, whatever named
/// the set on the command line.
fn set_slug(set_root: &Path) -> String {
    set_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

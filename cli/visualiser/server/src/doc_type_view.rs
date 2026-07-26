//! The API's doc-type descriptor: server-specific presentation over the shared
//! `corpus::DocTypeKey`, built from the resolved config.

use std::path::PathBuf;

use corpus::DocTypeKey;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocType {
    #[serde(with = "crate::doc_type_serde")]
    pub key: DocTypeKey,
    pub label: String,
    pub dir_path: Option<PathBuf>,
    pub in_lifecycle: bool,
    pub in_kanban: bool,
    pub r#virtual: bool,
    /// Number of indexed entries of this doc type as of the API call.
    ///
    /// On the JSON wire, this field is always populated by the
    /// `api::types::types` handler from the live indexer state. Templates
    /// is excluded from the index and so observes `count = 0` via
    /// `unwrap_or(0)` in the handler.
    ///
    /// In-process, `describe_types` constructs `DocType` values with
    /// `count: 0` as a placeholder — the API handler MUST overwrite this
    /// before serialisation. A non-handler consumer of `describe_types`
    /// (e.g., a future CLI introspector) would observe the placeholder
    /// directly and SHOULD NOT trust this field; consider splitting the
    /// type if a second consumer appears.
    pub count: usize,
}

#[must_use]
pub fn describe_types(cfg: &crate::config::Config) -> Vec<DocType> {
    let mut out = Vec::with_capacity(DocTypeKey::all().len());
    for key in DocTypeKey::all() {
        let dir_path = key
            .config_path_key()
            .and_then(|k| cfg.doc_paths.get(k).cloned());
        out.push(DocType {
            key,
            label: key.label().to_string(),
            dir_path,
            in_lifecycle: key.in_lifecycle(),
            in_kanban: key.in_kanban(),
            r#virtual: key.is_virtual(),
            count: 0,
        });
    }
    out
}

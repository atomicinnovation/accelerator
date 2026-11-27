use std::path::Path;
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use corpus::DocTypeKey;
use serde::Serialize;

use super::ApiError;
use crate::doc_type_view::PresentationView;
use crate::indexer::{AssetRecord, IndexEntry, IndexedSet};
use crate::server::AppState;

#[derive(Serialize)]
pub(crate) struct SetResponse {
    set: SetSummary,
    root: SetDocument,
    members: Vec<SetMember>,
}

#[derive(Serialize)]
struct SetSummary {
    #[serde(rename = "type", with = "crate::doc_type_serde")]
    kind: DocTypeKey,
    id: String,
    directory: String,
    presentation: PresentationView,
}

#[derive(Serialize)]
#[serde(untagged)]
enum SetMember {
    Document(SetDocument),
    Asset(SetAsset),
}

#[derive(Serialize)]
#[serde(tag = "kind", rename = "document", rename_all = "camelCase")]
struct SetDocument {
    set_relative_path: String,
    title: String,
    frontmatter: serde_json::Value,
    frontmatter_state: String,
    mtime_ms: i64,
    etag: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename = "asset", rename_all = "camelCase")]
struct SetAsset {
    set_relative_path: String,
    size: u64,
    mtime_ms: i64,
}

impl SetMember {
    fn set_relative_path(&self) -> &str {
        match self {
            Self::Document(document) => &document.set_relative_path,
            Self::Asset(asset) => &asset.set_relative_path,
        }
    }
}

impl From<IndexEntry> for SetDocument {
    fn from(entry: IndexEntry) -> Self {
        Self {
            set_relative_path: entry
                .set_placement()
                .map(|p| p.set_relative_path.clone())
                .unwrap_or_default(),
            title: entry.title,
            frontmatter: entry.frontmatter,
            frontmatter_state: entry.frontmatter_state,
            mtime_ms: entry.mtime_ms,
            etag: entry.etag,
        }
    }
}

impl From<AssetRecord> for SetMember {
    fn from(asset: AssetRecord) -> Self {
        Self::Asset(SetAsset {
            set_relative_path: asset.placement.set_relative_path,
            size: asset.size,
            mtime_ms: asset.mtime_ms,
        })
    }
}

pub(crate) async fn set_get(
    State(state): State<Arc<AppState>>,
    AxumPath((type_token, set_id)): AxumPath<(String, String)>,
) -> Result<Json<SetResponse>, ApiError> {
    let kind = state
        .doc_types
        .from_wire_token(&type_token)
        .ok_or(ApiError::InvalidDocType(type_token))?;
    let not_found = || ApiError::NotFound(set_id.clone());
    let set = state.doc_types.set_matcher(kind).ok_or_else(not_found)?;
    let IndexedSet {
        root,
        members,
        assets,
    } = state
        .indexer
        .set(kind, &set_id)
        .await
        .ok_or_else(not_found)?;
    let context = root.set_placement().ok_or_else(not_found)?.context.clone();

    let mut members: Vec<SetMember> = members
        .into_iter()
        .map(|document| SetMember::Document(document.into()))
        .chain(assets.into_iter().map(SetMember::from))
        .collect();
    members.sort_by(|a, b| a.set_relative_path().cmp(b.set_relative_path()));

    Ok(Json(SetResponse {
        set: SetSummary {
            kind,
            id: context.set_id.clone(),
            directory: served_directory_of(&root),
            presentation: set.presentation().into(),
        },
        root: root.into(),
        members,
    }))
}

fn served_directory_of(root: &IndexEntry) -> String {
    root.rel_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_string_lossy()
        .replace('\\', "/")
}

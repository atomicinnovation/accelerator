use std::sync::Arc;

use axum::{extract::{Path as AxumPath, State}, Json};
use serde::Serialize;

use crate::server::AppState;
use super::ApiError;

#[derive(Serialize)]
pub(crate) struct LifecycleListResponse {
    clusters: Vec<crate::clusters::LifecycleCluster>,
}

pub(crate) async fn lifecycle_list(
    State(state): State<Arc<AppState>>,
) -> Json<LifecycleListResponse> {
    Json(LifecycleListResponse {
        clusters: state.clusters.read().await.clone(),
    })
}

pub(crate) async fn lifecycle_one(
    State(state): State<Arc<AppState>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<Json<crate::clusters::LifecycleCluster>, ApiError> {
    let all = state.clusters.read().await;
    all.iter()
        .find(|c| c.slug == slug)
        .cloned()
        .map(Json)
        .ok_or(ApiError::NotFound(slug))
}

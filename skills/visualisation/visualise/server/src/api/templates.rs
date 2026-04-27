use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde::Serialize;

use super::ApiError;
use crate::server::AppState;

#[derive(Serialize)]
pub(crate) struct TemplatesListResponse {
    templates: Vec<crate::templates::TemplateSummary>,
}

pub(crate) async fn templates_list(
    State(state): State<Arc<AppState>>,
) -> Json<TemplatesListResponse> {
    Json(TemplatesListResponse {
        templates: state.templates.list(),
    })
}

pub(crate) async fn template_detail(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<crate::templates::TemplateDetail>, ApiError> {
    state
        .templates
        .detail(&name)
        .map(Json)
        .ok_or(ApiError::NotFound(name))
}

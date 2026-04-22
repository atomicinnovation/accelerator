use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::docs::describe_types;
use crate::server::AppState;

#[derive(Serialize)]
pub(crate) struct TypesResponse {
    types: Vec<crate::docs::DocType>,
}

pub(crate) async fn types(
    State(state): State<Arc<AppState>>,
) -> Json<TypesResponse> {
    Json(TypesResponse {
        types: describe_types(&state.cfg),
    })
}

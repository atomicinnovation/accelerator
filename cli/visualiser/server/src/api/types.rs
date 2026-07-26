use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::doc_type_view::describe_types;
use crate::server::AppState;

#[derive(Serialize)]
pub(crate) struct TypesResponse {
    types: Vec<crate::doc_type_view::DocType>,
}

pub(crate) async fn types(
    State(state): State<Arc<AppState>>,
) -> Json<TypesResponse> {
    let mut types = describe_types(&state.cfg);
    let counts = state.indexer.counts_by_type().await;
    for t in &mut types {
        t.count = counts.get(&t.key).copied().unwrap_or(0);
    }
    Json(TypesResponse { types })
}

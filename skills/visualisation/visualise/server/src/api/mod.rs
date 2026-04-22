use axum::Router;
use std::sync::Arc;

use crate::server::AppState;

pub fn mount(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
}

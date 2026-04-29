use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ServerInfoBody {
    pub name: &'static str,
    pub version: &'static str,
}

pub(crate) async fn get_info() -> impl IntoResponse {
    let body = Json(ServerInfoBody {
        name: "accelerator-visualiser",
        version: crate::VERSION,
    });
    ([(axum::http::header::CACHE_CONTROL, "no-cache")], body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get, Router};
    use http_body_util::BodyExt;
    use tower::ServiceExt as _;

    #[tokio::test]
    async fn info_returns_name_and_version_with_no_cache() {
        let app: Router = Router::new().route("/api/info", get(get_info));

        let resp = app
            .oneshot(Request::builder().uri("/api/info").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok()),
            Some("no-cache"),
        );
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["name"], "accelerator-visualiser");
        assert_eq!(v["version"], crate::VERSION);
    }
}

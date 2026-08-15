//! axum server bootstrap. Binds a random port on 127.0.0.1,
//! writes server-info.json + server.pid once the listener is
//! live, and serves a single placeholder route behind a
//! default-deny middleware stack. Signal handling and
//! owner-PID / idle watches land in later phases.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{any, get},
    Router,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::info;

use crate::config::Config;
use crate::shutdown::ShutdownReason;

/// 1 MiB cap on request bodies. The placeholder route never reads a body,
/// but the cap is the default-deny baseline every later phase inherits.
const REQUEST_BODY_LIMIT: usize = 1_048_576;

/// 30s request timeout — long enough for markdown rendering and diff
/// responses in later phases, short enough that a stuck handler can't
/// pin a worker forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct AppState {
    pub cfg: Arc<Config>,
    pub kanban_columns: Arc<Vec<crate::config::KanbanColumn>>,
    /// Resolved idle auto-shutdown window in milliseconds, or
    /// `config::DISABLED_IDLE_LIMIT_MS` when idle shutdown is disabled.
    /// Fed into `lifecycle::Settings` in `run`.
    pub idle_limit_ms: i64,
    pub file_driver: Arc<crate::file_driver::LocalFileDriver>,
    pub indexer: Arc<crate::indexer::Indexer>,
    pub templates: Arc<arc_swap::ArcSwap<crate::templates::TemplateResolver>>,
    pub template_change_handler: Arc<crate::watcher::TemplateChangeHandler>,
    pub clusters: Arc<RwLock<Vec<crate::clusters::LifecycleCluster>>>,
    pub http_activity: Arc<crate::activity::Activity>,
    pub activity_feed: Arc<crate::activity_feed::ActivityRingBuffer>,
    pub sse_hub: Arc<crate::sse_hub::SseHub>,
    pub write_coordinator: Arc<crate::write_coordinator::WriteCoordinator>,
}

impl AppState {
    pub async fn build(
        cfg: Config,
        http_activity: Arc<crate::activity::Activity>,
    ) -> Result<Arc<Self>, AppStateError> {
        let kanban_columns = Arc::new(cfg.resolve_kanban_columns()?);
        let idle_limit_ms = cfg.resolve_idle_limit_ms()?;
        let cfg = Arc::new(cfg);
        let template_roots =
            crate::file_driver::template_extra_roots(&cfg.templates);
        let work_root = cfg
            .doc_paths
            .get("work")
            .cloned()
            .map(|p| vec![p])
            .unwrap_or_default();
        let driver = Arc::new(crate::file_driver::LocalFileDriver::new(
            &cfg.doc_paths,
            template_roots,
            work_root,
        ));
        let work_item_cfg = Arc::new(match cfg.work_item.clone() {
            Some(raw) => crate::config::WorkItemConfig::from_raw(raw)?,
            None => crate::config::WorkItemConfig::default_numeric(),
        });
        let indexer = Arc::new(
            crate::indexer::Indexer::build(
                driver.clone(),
                cfg.project_root.clone(),
                work_item_cfg,
            )
            .await?,
        );
        let templates = Arc::new(arc_swap::ArcSwap::from_pointee(
            crate::templates::TemplateResolver::build(
                &cfg.templates,
                driver.as_ref(),
                &cfg.project_root,
                &cfg.plugin_root,
            )
            .await,
        ));
        let snapshot = indexer.all().await;
        let work_item_by_id = indexer.work_item_by_id_snapshot().await;
        let plans_by_id = indexer.plans_by_id_snapshot().await;
        let cluster_ctx = crate::clusters::ClusterContext::from_entries(
            &snapshot,
            &work_item_by_id,
            &plans_by_id,
            indexer.project_root(),
            indexer.work_item_cfg(),
        );
        let (cluster_seed, completeness_backfill, cluster_key_backfill) =
            crate::clusters::compute_clusters_with_backfill(
                &snapshot,
                &cluster_ctx,
            );
        let linked_counts = crate::related::collect_linked_counts(
            &indexer,
            &cluster_seed,
            &snapshot,
        )
        .await;
        indexer
            .apply_completeness_backfill(completeness_backfill)
            .await;
        indexer
            .apply_cluster_key_backfill(cluster_key_backfill)
            .await;
        indexer.apply_linked_count_backfill(linked_counts).await;
        let clusters = Arc::new(RwLock::new(cluster_seed));
        let sse_hub = Arc::new(crate::sse_hub::SseHub::new(256));
        let activity_feed =
            Arc::new(crate::activity_feed::ActivityRingBuffer::new());
        let write_coordinator =
            Arc::new(crate::write_coordinator::WriteCoordinator::new());
        let tier_index =
            crate::watcher::TierPathIndex::build(&cfg.templates).await;
        let driver_dyn: Arc<dyn crate::file_driver::FileDriver> =
            driver.clone();
        let template_change_handler =
            Arc::new(crate::watcher::TemplateChangeHandler::spawn(
                templates.clone(),
                Arc::new(cfg.templates.clone()),
                driver_dyn,
                tier_index,
                sse_hub.clone(),
                Arc::new(cfg.project_root.clone()),
                Arc::new(cfg.plugin_root.clone()),
            ));
        Ok(Arc::new(Self {
            cfg,
            kanban_columns,
            idle_limit_ms,
            file_driver: driver,
            indexer,
            templates,
            template_change_handler,
            clusters,
            http_activity,
            activity_feed,
            sse_hub,
            write_coordinator,
        }))
    }
}

async fn canonical_or_self(p: PathBuf) -> PathBuf {
    tokio::fs::canonicalize(&p).await.unwrap_or(p)
}

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("indexer build failed: {0}")]
    Indexer(#[from] crate::file_driver::FileDriverError),
    #[error("invalid configuration: {0}")]
    Config(#[from] crate::config::ConfigError),
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub version: String,
    pub pid: i32,
    /// Process start-time stamp, used for PID-identity checks.
    /// Seconds-since-epoch. `None` on platforms where it can't
    /// be obtained — callers fall back to bare PID comparison.
    pub start_time: Option<u64>,
    pub host: String,
    pub port: u16,
    pub url: String,
    pub log_path: PathBuf,
    pub tmp_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("host {0} is not a loopback address")]
    NonLoopbackHost(String),
    #[error("startup failed: {0}")]
    Startup(#[from] AppStateError),
    #[error("failed to bind listener on {addr}: {source}")]
    Bind {
        addr: String,
        source: std::io::Error,
    },
    #[error("failed to write lifecycle file {path}: {source}")]
    LifecycleWrite {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Serve(#[from] std::io::Error),
}

pub fn build_router(state: Arc<AppState>) -> Router {
    build_router_with_spa(state, crate::assets::apply_spa_serving)
}

/// Like `build_router` but points the SPA fallback at a caller-supplied
/// `dist_path`. Only exists under `dev-frontend` — under `embed-dist`
/// the dist is baked into the binary and cannot be swapped at runtime.
/// Callers that need to test the embed-dist handler use `serve_embedded<E>`
/// with a fixture embed type instead.
#[cfg(feature = "dev-frontend")]
pub fn build_router_with_dist(
    state: Arc<AppState>,
    dist_path: std::path::PathBuf,
) -> Router {
    build_router_with_spa(state, move |router| {
        crate::assets::apply_spa_serving_with_dist_path(router, dist_path)
    })
}

fn build_router_with_spa<F: FnOnce(Router) -> Router>(
    state: Arc<AppState>,
    attach_spa: F,
) -> Router {
    let api_router = Router::new()
        .route("/api/healthz", get(healthz))
        .route("/api/info", get(crate::api::info::get_info))
        .merge(crate::api::mount(state.clone()))
        .route("/api/*rest", any(api_not_found))
        .with_state(state.clone());

    attach_spa(api_router)
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(
                    tower_http::trace::DefaultMakeSpan::new()
                        .level(tracing::Level::INFO)
                        .include_headers(false),
                )
                .on_response(
                    tower_http::trace::DefaultOnResponse::new()
                        .level(tracing::Level::INFO)
                        .latency_unit(tower_http::LatencyUnit::Millis),
                ),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.http_activity.clone(),
            crate::activity::middleware,
        ))
        .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
        .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
        // origin_guard (inner) runs after host_header_guard (outer).
        // host_header_guard rejects DNS-rebinding; origin_guard rejects cross-origin
        // state-changing requests as defence-in-depth against CSRF if a future
        // maintainer ever adds a permissive CORS layer.
        .layer(middleware::from_fn(origin_guard))
        .layer(middleware::from_fn(host_header_guard))
        .layer(middleware::from_fn(version_header))
}

async fn version_header(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        "accelerator-visualiser-version",
        axum::http::HeaderValue::from_static(crate::VERSION),
    );
    resp
}

async fn healthz() -> &'static str {
    "ok\n"
}

async fn api_not_found(
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": "not-found",
            "path": uri.path(),
        })),
    )
}

pub async fn run(cfg: Config, info_path: &Path) -> Result<(), ServerError> {
    let host: IpAddr = cfg
        .host
        .parse()
        .map_err(|_| ServerError::NonLoopbackHost(cfg.host.clone()))?;
    if !bind_host_is_allowed(&host, e2e_insecure_allowed()) {
        return Err(ServerError::NonLoopbackHost(cfg.host.clone()));
    }

    let activity = Arc::new(crate::activity::Activity::new());
    let state = AppState::build(cfg, activity.clone()).await?;
    let app = build_router(state.clone());

    let bind_addr = SocketAddr::new(host, 0);
    let listener = TcpListener::bind(bind_addr).await.map_err(|source| {
        ServerError::Bind {
            addr: bind_addr.to_string(),
            source,
        }
    })?;
    let local = listener.local_addr().map_err(|source| ServerError::Bind {
        addr: bind_addr.to_string(),
        source,
    })?;

    let info = ServerInfo {
        version: crate::VERSION.to_string(),
        pid: std::process::id() as i32,
        start_time: process_probe::start_time(std::process::id() as i32),
        host: state.cfg.host.clone(),
        port: local.port(),
        url: format!("http://{}:{}", state.cfg.host, local.port()),
        log_path: state.cfg.log_path.clone(),
        tmp_path: state.cfg.tmp_path.clone(),
    };

    // Register signal handlers before announcing readiness via
    // server-info.json: once the launcher (or a test) observes that file it
    // may immediately send SIGTERM, and a handler registered afterwards would
    // race the default-disposition kill. Any signal that arrives before the
    // serve loop polls `rx` is buffered in the channel and drains a graceful
    // shutdown as soon as serving begins.
    let (tx, mut rx) = mpsc::channel::<ShutdownReason>(4);
    spawn_signal_handlers(&tx)?;

    // Write PID file first (smaller artefact, faster to land) then
    // server-info.json. Both atomic-rename; order matters only to
    // the launcher's poll-for-readiness, which keys on
    // server-info.json.
    let pid_path = info_path.with_file_name("server.pid");
    write_pid_file(&pid_path, info.pid).map_err(|source| {
        ServerError::LifecycleWrite {
            path: pid_path.clone(),
            source,
        }
    })?;
    write_server_info(info_path, &info).map_err(|source| {
        ServerError::LifecycleWrite {
            path: info_path.to_path_buf(),
            source,
        }
    })?;
    info!(url = %info.url, pid = info.pid, start_time = ?info.start_time, "server-started");
    crate::lifecycle::spawn(
        activity.clone(),
        state.cfg.owner_pid,
        state.cfg.owner_start_time,
        crate::lifecycle::Settings {
            tick: crate::lifecycle::Settings::DEFAULT.tick,
            idle_limit_ms: state.idle_limit_ms,
        },
        tx.clone(),
    );

    let mut watch_dirs: Vec<std::path::PathBuf> = Vec::new();
    for p in state.cfg.doc_paths.values() {
        watch_dirs.push(canonical_or_self(p.clone()).await);
    }
    for p in crate::file_driver::template_extra_roots(&state.cfg.templates) {
        watch_dirs.push(canonical_or_self(p).await);
    }
    watch_dirs.sort();
    watch_dirs.dedup();
    let watcher_handle = crate::watcher::spawn(
        watch_dirs,
        state.cfg.project_root.clone(),
        state.indexer.clone(),
        state.clusters.clone(),
        state.sse_hub.clone(),
        state.activity_feed.clone(),
        state.write_coordinator.clone(),
        Some(state.template_change_handler.clone()),
        crate::watcher::Settings::DEFAULT,
    );
    tokio::spawn(async move {
        if let Err(e) = watcher_handle.await {
            tracing::error!(
                error = %e,
                "filesystem watcher task exited unexpectedly; \
                 file-change notifications are disabled until the server restarts",
            );
        }
    });

    let info_path = info_path.to_path_buf();
    let pid_path = info_path.with_file_name("server.pid");
    let stopped_path = info_path.with_file_name("server-stopped.json");
    let shutdown_signal = async move {
        // `rx.recv()` only returns None if every Sender has been
        // dropped before producing a reason — a programming bug,
        // not a real shutdown. Distinguish it via the dedicated
        // `StartupFailure` variant so the audit trail records the
        // anomaly instead of falsely attributing it to SIGTERM.
        let reason = rx.recv().await.unwrap_or(ShutdownReason::StartupFailure);
        info!(?reason, "shutdown requested");
        // Order matters: write server-stopped.json first, then
        // remove server-info.json + server.pid only if the stopped
        // write succeeded. If the stopped write fails (disk-full,
        // read-only FS, EXDEV), leave info.json + server.pid in
        // place — the launcher's stale-PID reuse path treats that
        // as "previous instance left state behind" and recovers
        // cleanly on next launch. The reverse order, or
        // unconditional removal, yields a {no info, no stopped}
        // state that breaks the post-shutdown audit invariant.
        match write_server_stopped(&stopped_path, reason) {
            Ok(()) => {
                let _ = std::fs::remove_file(&info_path);
                let _ = std::fs::remove_file(&pid_path);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "failed to write server-stopped.json; preserving server-info.json and server.pid for next-launch recovery"
                );
            }
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;
    Ok(())
}

fn spawn_signal_handlers(
    tx: &mpsc::Sender<ShutdownReason>,
) -> std::io::Result<()> {
    use tokio::signal::unix::{signal, SignalKind};
    // Create the signal streams synchronously so the OS-level handlers are
    // installed before this returns — only the blocking `recv().await` is
    // deferred to a task. Installing inside the spawned task instead would
    // leave a window where a signal arriving before the task is first
    // scheduled hits the default (terminate) disposition.
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    tokio::spawn({
        let tx = tx.clone();
        async move {
            sigterm.recv().await;
            let _ = tx.send(ShutdownReason::Sigterm).await;
        }
    });
    tokio::spawn({
        let tx = tx.clone();
        async move {
            sigint.recv().await;
            let _ = tx.send(ShutdownReason::Sigint).await;
        }
    });
    Ok(())
}

/// Atomically publishes a visualiser state file (pid / info / stopped sentinel)
/// at owner-only `0600` through the shared store — the file reveals the listener
/// URL and process identity, and lives under the user's project tree where other
/// local accounts may have traversal. Temp-write, fsync, rename, fsync the dir.
fn write_state_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "state file path has no parent",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    store::atomic_write(
        path,
        bytes,
        &store::WriteBounds {
            permitted_root: dir,
            project_root: dir,
        },
        store::NewFileMode::Set(0o600),
    )
    .map_err(|error| std::io::Error::other(error.to_string()))
}

pub(crate) fn write_server_stopped(
    path: &Path,
    reason: ShutdownReason,
) -> std::io::Result<()> {
    let record = serde_json::json!({
        "reason": reason,
        // System-clock read — if this errs (pre-epoch clock) we
        // emit a null timestamp rather than a silent 0 that would
        // read as a legitimate 1970-01-01 exit.
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs()),
    });
    let mut bytes = serde_json::to_vec_pretty(&record)?;
    bytes.push(b'\n');
    write_state_file(path, &bytes)
}

fn write_server_info(path: &Path, info: &ServerInfo) -> std::io::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(info)?;
    bytes.push(b'\n');
    write_state_file(path, &bytes)
}

fn write_pid_file(path: &Path, pid: i32) -> std::io::Result<()> {
    write_state_file(path, format!("{pid}\n").as_bytes())
}

/// Whether the containerised visual-regression flow has opted into a
/// non-loopback bind and a relaxed Host-header guard.
///
/// Defence-in-depth gate: this bypass is compiled in **only** for the
/// `dev-frontend` (test/dev) binary — the release `embed-dist` binary never
/// contains it — and additionally requires the explicit
/// `ACCELERATOR_VISUALISER_E2E_INSECURE` env var at runtime, set only by the
/// Docker visual-regression task. Normal `mise run dev` (also dev-frontend,
/// but without the env var) stays strictly loopback-only.
#[cfg(feature = "dev-frontend")]
fn e2e_insecure_allowed() -> bool {
    std::env::var_os("ACCELERATOR_VISUALISER_E2E_INSECURE").is_some()
}

#[cfg(not(feature = "dev-frontend"))]
fn e2e_insecure_allowed() -> bool {
    false
}

/// The startup bind is allowed iff the host is loopback, or the e2e bypass is
/// active (the container reaches the host over a non-loopback bridge gateway).
fn bind_host_is_allowed(host: &IpAddr, insecure: bool) -> bool {
    insecure || host.is_loopback()
}

/// The Host header is accepted iff it names a loopback origin (the
/// DNS-rebinding defence), or the e2e bypass is active (the container reaches
/// the host via `host.docker.internal`, whose Host header is non-loopback).
fn host_header_is_allowed(host_part: &str, insecure: bool) -> bool {
    insecure
        || host_part == "127.0.0.1"
        || host_part == "localhost"
        || host_part.is_empty()
}

async fn host_header_guard(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Defence-in-depth against DNS-rebinding: only accept the
    // Host header values we actually bind to.
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let (host_part, _) = host.split_once(':').unwrap_or((host, ""));
    if host_header_is_allowed(host_part, e2e_insecure_allowed()) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// True iff `origin` names a genuine loopback HTTP origin: scheme `http` and an
/// authority host of exactly `127.0.0.1` or `localhost` (any port). The origin
/// is parsed as a URI and the authority host compared exactly, so a
/// loopback-lookalike (`http://127.0.0.1.evil.com`) or a userinfo smuggle
/// (`http://127.0.0.1:x@evil.com`) resolves to its true host and is rejected —
/// both defeat a naive `starts_with` prefix match.
fn origin_is_loopback(origin: &str) -> bool {
    let Ok(uri) = origin.parse::<axum::http::Uri>() else {
        return false;
    };
    if uri.scheme_str() != Some("http") {
        return false;
    }
    matches!(uri.host(), Some("127.0.0.1" | "localhost"))
}

async fn origin_guard(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Reject state-changing requests from foreign origins. Browsers always send
    // Origin on cross-origin PATCH/POST/PUT/DELETE; curl and server-to-server
    // callers omit it, so requests with no Origin header are allowed through.
    let is_state_changing = matches!(
        req.method(),
        &axum::http::Method::PATCH
            | &axum::http::Method::POST
            | &axum::http::Method::PUT
            | &axum::http::Method::DELETE
    );
    if is_state_changing {
        if let Some(origin) = req.headers().get("origin") {
            let s = origin.to_str().unwrap_or("");
            if !origin_is_loopback(s) {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn minimal_config(tmp: &Path) -> Config {
        Config {
            plugin_root: tmp.to_path_buf(),
            plugin_version: "test".into(),
            project_root: tmp.to_path_buf(),
            tmp_path: tmp.to_path_buf(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: tmp.join("server.log"),
            doc_paths: HashMap::new(),
            templates: HashMap::new(),
            work_item: None,
            kanban_columns: None,
            idle_timeout: None,
            editor: None,
            editor_project: None,
        }
    }

    #[test]
    fn write_server_info_roundtrips() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let info_path = dir.path().join("server-info.json");
        let info = ServerInfo {
            version: "0.0.0-test".into(),
            pid: 42,
            start_time: Some(1_700_000_000),
            host: "127.0.0.1".into(),
            port: 1234,
            url: "http://127.0.0.1:1234".into(),
            log_path: dir.path().join("server.log"),
            tmp_path: dir.path().to_path_buf(),
        };
        write_server_info(&info_path, &info).unwrap();
        let bytes = std::fs::read(&info_path).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["port"], 1234);
        assert_eq!(v["url"], "http://127.0.0.1:1234");
        assert_eq!(v["pid"], 42);
        assert_eq!(v["start_time"], 1_700_000_000);
        let mode =
            std::fs::metadata(&info_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "server-info.json must be owner-only");
    }

    #[test]
    fn write_pid_file_roundtrips() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("server.pid");
        write_pid_file(&p, 9999).unwrap();
        let content = std::fs::read_to_string(&p).unwrap();
        assert_eq!(content.trim(), "9999");
        assert!(content.ends_with('\n'));
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[tokio::test]
    async fn non_loopback_host_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(dir.path());
        cfg.host = "0.0.0.0".into();
        let err = run(cfg, &dir.path().join("server-info.json"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ServerError::NonLoopbackHost(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn bind_host_allowed_only_for_loopback_unless_insecure() {
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();
        let any: IpAddr = "0.0.0.0".parse().unwrap();
        // Secure (default): only loopback binds.
        assert!(bind_host_is_allowed(&loopback, false));
        assert!(!bind_host_is_allowed(&any, false));
        // Insecure (e2e Docker opt-in): a non-loopback bind is permitted.
        assert!(bind_host_is_allowed(&any, true));
        assert!(bind_host_is_allowed(&loopback, true));
    }

    #[test]
    fn host_header_allowed_only_for_loopback_unless_insecure() {
        // Secure (default): loopback origins and empty only.
        assert!(host_header_is_allowed("127.0.0.1", false));
        assert!(host_header_is_allowed("localhost", false));
        assert!(host_header_is_allowed("", false));
        assert!(!host_header_is_allowed("host.docker.internal", false));
        // Insecure (e2e Docker opt-in): the container's bridge-gateway Host
        // (host.docker.internal) is accepted.
        assert!(host_header_is_allowed("host.docker.internal", true));
    }

    #[test]
    fn origin_guard_accepts_genuine_loopback_origins() {
        assert!(origin_is_loopback("http://127.0.0.1"));
        assert!(origin_is_loopback("http://127.0.0.1:8080"));
        assert!(origin_is_loopback("http://localhost"));
        assert!(origin_is_loopback("http://localhost:3000"));
    }

    #[test]
    fn origin_guard_rejects_lookalikes_userinfo_and_foreign() {
        // Loopback-lookalike host and userinfo-@ smuggle both defeat a naive
        // prefix check but resolve to a foreign authority host here.
        assert!(!origin_is_loopback("http://127.0.0.1.evil.com"));
        assert!(!origin_is_loopback("http://127.0.0.1:x@evil.com"));
        assert!(!origin_is_loopback("http://localhost.evil.com"));
        // Wrong scheme, foreign host, and unparseable origins.
        assert!(!origin_is_loopback("https://127.0.0.1"));
        assert!(!origin_is_loopback("https://evil.example"));
        assert!(!origin_is_loopback("null"));
        assert!(!origin_is_loopback(""));
    }

    #[tokio::test]
    async fn idle_timeout_resolves_into_app_state() {
        // Pins the resolve→store→Settings wiring: a configured idle_timeout
        // must reach AppState.idle_limit_ms (and thence lifecycle::spawn in
        // `run`), not just be parsed in isolation.
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(dir.path());
        cfg.idle_timeout = Some("30m".to_string());
        let activity = Arc::new(crate::activity::Activity::new());
        let state = AppState::build(cfg, activity).await.unwrap();
        assert_eq!(state.idle_limit_ms, 30 * 60 * 1000);
    }

    #[test]
    fn write_server_stopped_produces_parseable_json() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("server-stopped.json");
        write_server_stopped(&p, ShutdownReason::Sigterm).unwrap();
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!(v["reason"], "sigterm");
        assert!(v["timestamp"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn shutdown_preserves_info_when_stopped_write_fails() {
        let dir = tempfile::tempdir().unwrap();
        let info_path = dir.path().join("server-info.json");
        let pid_path = dir.path().join("server.pid");
        let stopped_path = dir.path().join("server-stopped.json");

        // Seed fake lifecycle files as if the server were live.
        std::fs::write(&info_path, r#"{"url":"http://127.0.0.1:1"}"#).unwrap();
        std::fs::write(&pid_path, "9999\n").unwrap();

        // Block the stopped-file write by occupying its path with a
        // non-empty directory that tempfile::persist cannot replace.
        std::fs::create_dir(&stopped_path).unwrap();
        std::fs::write(stopped_path.join("blocker"), "x").unwrap();

        match write_server_stopped(&stopped_path, ShutdownReason::Sigterm) {
            Ok(()) => panic!("expected write_server_stopped to fail"),
            Err(e) => {
                tracing::warn!(error = %e, "expected failure");
            }
        }

        assert!(
            info_path.exists(),
            "server-info.json must be preserved when stopped-write fails"
        );
        assert!(
            pid_path.exists(),
            "server.pid must be preserved when stopped-write fails"
        );
    }

    #[cfg(feature = "dev-frontend")]
    fn seed_stub_dist(tmp: &std::path::Path) {
        std::fs::write(
            tmp.join("index.html"),
            "<!doctype html><html>stub</html>",
        )
        .unwrap();
    }

    #[cfg(feature = "dev-frontend")]
    async fn build_minimal_state(tmp: &std::path::Path) -> Arc<AppState> {
        let cfg = minimal_config(tmp);
        let activity = Arc::new(crate::activity::Activity::new());
        AppState::build(cfg, activity).await.unwrap()
    }

    #[cfg(feature = "dev-frontend")]
    #[tokio::test]
    async fn serves_spa_root_and_writes_info() {
        let dir = tempfile::tempdir().unwrap();
        let info_path = dir.path().join("server-info.json");

        let dist = tempfile::tempdir().unwrap();
        seed_stub_dist(dist.path());

        let state = build_minimal_state(dir.path()).await;
        let app =
            build_router_with_dist(state.clone(), dist.path().to_path_buf());

        let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await
        {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "SKIP: TCP bind not permitted in this environment: {e}"
                );
                return;
            }
            Err(e) => panic!("unexpected bind error: {e}"),
        };
        let port = listener.local_addr().unwrap().port();

        let info = ServerInfo {
            version: crate::VERSION.to_string(),
            pid: std::process::id() as i32,
            start_time: process_probe::start_time(std::process::id() as i32),
            host: "127.0.0.1".into(),
            port,
            url: format!("http://127.0.0.1:{port}"),
            log_path: state.cfg.log_path.clone(),
            tmp_path: state.cfg.tmp_path.clone(),
        };
        write_server_info(&info_path, &info).unwrap();

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        assert!(info_path.exists(), "server-info.json must exist");
        let url = format!("http://127.0.0.1:{port}");
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = resp.text().await.unwrap();
        assert!(
            body.contains("<!doctype html") || body.contains("<!DOCTYPE html"),
            "expected HTML, got: {body:.200}",
        );

        handle.abort();
    }

    #[cfg(feature = "dev-frontend")]
    #[tokio::test]
    async fn spa_fallback_is_covered_by_host_header_guard() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;

        let dir = tempfile::tempdir().unwrap();
        let dist = tempfile::tempdir().unwrap();
        seed_stub_dist(dist.path());
        let state = build_minimal_state(dir.path()).await;
        let app = build_router_with_dist(state, dist.path().to_path_buf());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/library/decisions")
                    .header("host", "evil.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[cfg(feature = "dev-frontend")]
    #[tokio::test]
    async fn spa_fallback_updates_activity() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;

        let dir = tempfile::tempdir().unwrap();
        let dist = tempfile::tempdir().unwrap();
        seed_stub_dist(dist.path());
        let state = build_minimal_state(dir.path()).await;
        let before = state.http_activity.last_millis();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let app =
            build_router_with_dist(state.clone(), dist.path().to_path_buf());

        let _ = app
            .oneshot(
                Request::builder()
                    .uri("/library/decisions")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let after = state.http_activity.last_millis();
        assert!(
            after > before,
            "expected activity to update (before={before}, after={after})"
        );
    }

    #[cfg(feature = "dev-frontend")]
    #[tokio::test]
    async fn unmatched_api_path_returns_json_404_not_spa_html() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;

        let dir = tempfile::tempdir().unwrap();
        let dist = tempfile::tempdir().unwrap();
        seed_stub_dist(dist.path());
        let state = build_minimal_state(dir.path()).await;
        let app = build_router_with_dist(state, dist.path().to_path_buf());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/bogus")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.contains("application/json"),
            "expected JSON 404, got content-type: {ct}",
        );
    }

    #[cfg(feature = "dev-frontend")]
    #[tokio::test]
    async fn spa_asset_is_brotli_encoded_for_br_clients() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;

        let dir = tempfile::tempdir().unwrap();
        let dist = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dist.path().join("assets")).unwrap();
        std::fs::write(
            dist.path().join("assets/app.js"),
            "// ".to_string() + &"x".repeat(4096),
        )
        .unwrap();
        std::fs::write(dist.path().join("index.html"), "<!doctype html>")
            .unwrap();

        let state = build_minimal_state(dir.path()).await;
        let app = build_router_with_dist(state, dist.path().to_path_buf());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .header("host", "127.0.0.1")
                    .header("accept-encoding", "br")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let ce = resp
            .headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(ce, "br", "expected Content-Encoding: br, got: {ce:?}");
    }
}

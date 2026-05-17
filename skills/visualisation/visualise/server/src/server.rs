//! axum server bootstrap. Binds a random port on 127.0.0.1,
//! writes server-info.json + server.pid once the listener is
//! live, and serves a single placeholder route behind a
//! default-deny middleware stack. Signal handling and
//! owner-PID / idle watches land in later phases.

use std::io::Write;
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
    pub file_driver: Arc<crate::file_driver::LocalFileDriver>,
    pub indexer: Arc<crate::indexer::Indexer>,
    pub templates: Arc<crate::templates::TemplateResolver>,
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
        let cfg = Arc::new(cfg);
        let template_roots = crate::file_driver::template_extra_roots(&cfg.templates);
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
            crate::indexer::Indexer::build(driver.clone(), cfg.project_root.clone(), work_item_cfg).await?,
        );
        let templates = Arc::new(
            crate::templates::TemplateResolver::build(&cfg.templates, driver.as_ref()).await,
        );
        let cluster_seed = crate::clusters::compute_clusters(&indexer.all().await);
        let clusters = Arc::new(RwLock::new(cluster_seed));
        let sse_hub = Arc::new(crate::sse_hub::SseHub::new(256));
        let activity_feed = Arc::new(crate::activity_feed::ActivityRingBuffer::new());
        let write_coordinator = Arc::new(crate::write_coordinator::WriteCoordinator::new());
        Ok(Arc::new(Self {
            cfg,
            kanban_columns,
            file_driver: driver,
            indexer,
            templates,
            clusters,
            http_activity,
            activity_feed,
            sse_hub,
            write_coordinator,
        }))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("indexer build failed: {0}")]
    Indexer(#[from] crate::file_driver::FileDriverError),
    #[error("invalid work-item config: {0}")]
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
pub fn build_router_with_dist(state: Arc<AppState>, dist_path: std::path::PathBuf) -> Router {
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

async fn api_not_found(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
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
    if !host.is_loopback() {
        return Err(ServerError::NonLoopbackHost(cfg.host.clone()));
    }

    let activity = Arc::new(crate::activity::Activity::new());
    let state = AppState::build(cfg, activity.clone()).await?;
    let app = build_router(state.clone());

    let bind_addr = SocketAddr::new(host, 0);
    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|source| ServerError::Bind {
            addr: bind_addr.to_string(),
            source,
        })?;
    let local = listener.local_addr().map_err(|source| ServerError::Bind {
        addr: bind_addr.to_string(),
        source,
    })?;

    let info = ServerInfo {
        version: crate::VERSION.to_string(),
        pid: std::process::id() as i32,
        start_time: process_start_time(std::process::id() as i32),
        host: state.cfg.host.clone(),
        port: local.port(),
        url: format!("http://{}:{}", state.cfg.host, local.port()),
        log_path: state.cfg.log_path.clone(),
        tmp_path: state.cfg.tmp_path.clone(),
    };

    // Write PID file first (smaller artefact, faster to land) then
    // server-info.json. Both atomic-rename; order matters only to
    // the launcher's poll-for-readiness, which keys on
    // server-info.json.
    let pid_path = info_path.with_file_name("server.pid");
    write_pid_file(&pid_path, info.pid).map_err(|source| ServerError::LifecycleWrite {
        path: pid_path.clone(),
        source,
    })?;
    write_server_info(info_path, &info).map_err(|source| ServerError::LifecycleWrite {
        path: info_path.to_path_buf(),
        source,
    })?;
    info!(url = %info.url, pid = info.pid, start_time = ?info.start_time, "server-started");

    let (tx, mut rx) = mpsc::channel::<ShutdownReason>(4);
    spawn_signal_handlers(tx.clone());
    crate::lifecycle::spawn(
        activity.clone(),
        state.cfg.owner_pid,
        state.cfg.owner_start_time,
        crate::lifecycle::Settings::DEFAULT,
        tx.clone(),
    );

    let watch_dirs: Vec<std::path::PathBuf> = state.cfg.doc_paths.values().cloned().collect();
    let watcher_handle = crate::watcher::spawn(
        watch_dirs,
        state.cfg.project_root.clone(),
        state.indexer.clone(),
        state.clusters.clone(),
        state.sse_hub.clone(),
        state.activity_feed.clone(),
        state.write_coordinator.clone(),
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

fn spawn_signal_handlers(tx: mpsc::Sender<ShutdownReason>) {
    use tokio::signal::unix::{signal, SignalKind};
    tokio::spawn({
        let tx = tx.clone();
        async move {
            let mut s = signal(SignalKind::terminate()).expect("SIGTERM handler");
            s.recv().await;
            let _ = tx.send(ShutdownReason::Sigterm).await;
        }
    });
    tokio::spawn(async move {
        let mut s = signal(SignalKind::interrupt()).expect("SIGINT handler");
        s.recv().await;
        let _ = tx.send(ShutdownReason::Sigint).await;
    });
}

fn write_server_stopped(path: &Path, reason: ShutdownReason) -> std::io::Result<()> {
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
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "server-stopped.json path has no parent",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    serde_json::to_writer_pretty(&mut tmp, &record)?;
    tmp.as_file_mut().write_all(b"\n")?;
    use std::os::unix::fs::PermissionsExt;
    tmp.as_file()
        .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(path)?;
    Ok(())
}

fn write_server_info(path: &Path, info: &ServerInfo) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "server-info.json path has no parent",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    serde_json::to_writer_pretty(&mut tmp, info)?;
    tmp.as_file_mut().write_all(b"\n")?;
    // Owner-only read/write — the file reveals listener URL and
    // process identity, and lives under the user's project tree
    // where other local accounts may have traversal.
    use std::os::unix::fs::PermissionsExt;
    tmp.as_file()
        .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(path)?;
    Ok(())
}

fn write_pid_file(path: &Path, pid: i32) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "server.pid path has no parent",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    writeln!(tmp.as_file_mut(), "{pid}")?;
    use std::os::unix::fs::PermissionsExt;
    tmp.as_file()
        .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(path)?;
    Ok(())
}

/// Seconds-since-epoch at which `pid` started, if obtainable.
/// macOS: `ps -p <pid> -o lstart=` → `date -j -f` → epoch.
/// Linux: `/proc/<pid>/stat` field 22 (clock ticks since boot) +
///        `/proc/stat` `btime` → absolute epoch.
/// Returns None on any parse or IO failure — the caller falls back
/// to bare PID comparison.
pub(crate) fn process_start_time(pid: i32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // Field 22 (1-indexed) is starttime in clock ticks since boot.
        // The command field (field 2) is wrapped in parens and may
        // contain spaces, so skip past the last ')' before splitting.
        let tail = stat.rsplit_once(')').map(|(_, t)| t)?;
        let starttime_ticks: u64 = tail.split_whitespace().nth(19)?.parse().ok()?;
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
        if hz == 0 {
            return None;
        }
        let btime_line = std::fs::read_to_string("/proc/stat")
            .ok()?
            .lines()
            .find(|l| l.starts_with("btime "))?
            .to_string();
        let btime: u64 = btime_line.split_whitespace().nth(1)?.parse().ok()?;
        Some(btime + starttime_ticks / hz)
    }
    #[cfg(target_os = "macos")]
    {
        // Read p_starttime directly via sysctl(KERN_PROC_PID) rather than
        // shelling out to `ps` + `date`. The subprocess approach races with
        // Tokio's internal infrastructure when tests run in parallel, causing
        // sporadic None returns. tv_sec is a UTC epoch value and matches what
        // `date -j -f "%a %b %d %H:%M:%S %Y" "$(ps -p pid -o lstart=)" +%s`
        // returns in _launcher-helpers.sh: both truncate to the same second.
        //
        // libc does not export kinfo_proc, so we use a raw byte buffer.
        // p_starttime (a timeval) is the first field of extern_proc's p_un
        // union, which is the first field of kinfo_proc — tv_sec is at byte 0.
        //
        // CTL_KERN=1, KERN_PROC=14, KERN_PROC_PID=1 (stable macOS ABI).
        // sizeof(kinfo_proc) = 648 on all 64-bit macOS targets.
        const CTL_KERN: libc::c_int = 1;
        const KERN_PROC: libc::c_int = 14;
        const KERN_PROC_PID: libc::c_int = 1;
        const KINFO_PROC_SIZE: usize = 648;
        let mut buf = [0u8; KINFO_PROC_SIZE];
        let mut size: usize = KINFO_PROC_SIZE;
        let mib = [CTL_KERN, KERN_PROC, KERN_PROC_PID, pid];
        let ret = unsafe {
            libc::sysctl(
                mib.as_ptr() as *mut libc::c_int,
                mib.len() as libc::c_uint,
                buf.as_mut_ptr() as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if ret != 0 || size == 0 {
            return None;
        }
        let tv_sec = i64::from_ne_bytes(buf[..8].try_into().ok()?);
        if tv_sec <= 0 {
            return None;
        }
        Some(tv_sec as u64)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

async fn host_header_guard(req: Request, next: Next) -> Result<Response, StatusCode> {
    // Defence-in-depth against DNS-rebinding: only accept the
    // Host header values we actually bind to.
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let (host_part, _) = host.split_once(':').unwrap_or((host, ""));
    if host_part == "127.0.0.1" || host_part == "localhost" || host_part.is_empty() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

async fn origin_guard(req: Request, next: Next) -> Result<Response, StatusCode> {
    // Reject state-changing requests from foreign origins. Browsers always send
    // Origin on cross-origin PATCH/POST/PUT/DELETE; curl and server-to-server
    // callers omit it, so requests with no Origin header are allowed through.
    // Allowed: http://127.0.0.1[:<port>] and http://localhost[:<port>].
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
            let allowed = s.starts_with("http://127.0.0.1") || s.starts_with("http://localhost");
            if !allowed {
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
        }
    }

    #[test]
    fn write_server_info_roundtrips() {
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
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&info_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "server-info.json must be owner-only");
    }

    #[test]
    fn write_pid_file_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("server.pid");
        write_pid_file(&p, 9999).unwrap();
        let content = std::fs::read_to_string(&p).unwrap();
        assert_eq!(content.trim(), "9999");
        assert!(content.ends_with('\n'));
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn process_start_time_is_stable_for_same_pid() {
        let me = std::process::id() as i32;
        let first = process_start_time(me);
        let second = process_start_time(me);
        assert_eq!(first, second, "start_time must be stable for the same PID");
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        assert!(first.is_some(), "start_time should resolve on Linux/macOS");
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
    fn write_server_stopped_produces_parseable_json() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("server-stopped.json");
        write_server_stopped(&p, ShutdownReason::Sigterm).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
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
        std::fs::write(tmp.join("index.html"), "<!doctype html><html>stub</html>").unwrap();
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
        let app = build_router_with_dist(state.clone(), dist.path().to_path_buf());

        let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("SKIP: TCP bind not permitted in this environment: {e}");
                return;
            }
            Err(e) => panic!("unexpected bind error: {e}"),
        };
        let port = listener.local_addr().unwrap().port();

        let info = ServerInfo {
            version: crate::VERSION.to_string(),
            pid: std::process::id() as i32,
            start_time: process_start_time(std::process::id() as i32),
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
        let app = build_router_with_dist(state.clone(), dist.path().to_path_buf());

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
        std::fs::write(dist.path().join("index.html"), "<!doctype html>").unwrap();

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

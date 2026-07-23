use std::convert::Infallible;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path as AxumPath, State};
use axum::http::{header, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use rust_embed::Embed;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::dashboard::data;
use crate::error::DowError;

#[derive(Embed)]
#[folder = "dashboard-frontend/"]
struct Assets;

#[derive(Clone)]
struct AppState {
    doc_root: PathBuf,
    notify_tx: Arc<broadcast::Sender<()>>,
    connections: Arc<AtomicUsize>,
}

pub async fn start(doc_root: PathBuf, port: u16, no_open: bool) -> Result<i32, DowError> {
    let (notify_tx, _) = broadcast::channel::<()>(16);
    let notify_tx = Arc::new(notify_tx);
    let connections = Arc::new(AtomicUsize::new(0));

    let _watcher = crate::dashboard::watcher::spawn_watcher(&doc_root, notify_tx.clone());

    let state = AppState {
        doc_root,
        notify_tx,
        connections: connections.clone(),
    };

    let app = Router::new()
        .route("/api/data", get(handle_data))
        .route("/api/events", get(handle_sse))
        .route("/api/task/{id}/done", post(handle_task_done))
        .route("/api/task/{id}/reopen", post(handle_task_reopen))
        .route("/api/issue/{id}/close", post(handle_issue_close))
        .route("/api/issue/{id}/reopen", post(handle_issue_reopen))
        .route("/", get(handle_index))
        .route("/assets/{*path}", get(handle_asset))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| DowError::new(format!("Failed to bind port {}: {}", port, e), 1))?;

    eprintln!("[dow dashboard] Listening on http://127.0.0.1:{}", port);

    if !no_open {
        let url = format!("http://127.0.0.1:{}", port);
        tokio::spawn(async move {
            if open::that(&url).is_err() {
                eprintln!("[dow dashboard] Could not open browser. Visit {}", url);
            }
        });
    }

    let connections_clone = connections.clone();
    let shutdown_signal = async move {
        // 等待初始连接（给浏览器 30 秒打开时间）
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let count = connections_clone.load(Ordering::Relaxed);
            if count == 0 {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let count = connections_clone.load(Ordering::Relaxed);
                if count == 0 {
                    eprintln!("[dow dashboard] No connections, shutting down.");
                    return;
                }
            }
        }
    };

    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("\n[dow dashboard] Interrupted, shutting down.");
    };

    let shutdown = async {
        tokio::select! {
            _ = shutdown_signal => {}
            _ = ctrl_c => {}
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| DowError::new(format!("Server error: {}", e), 1))?;

    Ok(0)
}

async fn handle_data(State(state): State<AppState>) -> impl IntoResponse {
    let project_data = data::collect_project_data(&state.doc_root);
    axum::Json(project_data)
}

async fn handle_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    state.connections.fetch_add(1, Ordering::Relaxed);
    let connections = state.connections.clone();
    let doc_root = state.doc_root.clone();

    let rx = state.notify_tx.subscribe();
    let stream = BroadcastStream::new(rx);

    let event_stream = stream.filter_map(move |msg| match msg {
            Ok(_) => {
                let project_data = data::collect_project_data(&doc_root);
                let json = serde_json::to_string(&project_data).unwrap_or_default();
            Some(Ok::<_, Infallible>(
                Event::default().event("update").data(json),
            ))
            }
            Err(_) => None,
    });

    let guard = ConnectionGuard(connections);
    let event_stream = event_stream.map(move |item| {
        let _ = &guard;
        item
    });

    Sse::new(event_stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)))
}

struct ConnectionGuard(Arc<AtomicUsize>);

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

async fn handle_index() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(file) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            file.data.to_vec(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
    }
}

async fn handle_asset(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    match Assets::get(&path) {
        Some(file) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                file.data.to_vec(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

// ─── Action API Types ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ActionResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ActionResponse {
    fn success() -> Self {
        Self { ok: true, error: None }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self { ok: false, error: Some(msg.into()) }
    }
}

// ─── Task Done ───────────────────────────────────────────────────────────────

async fn handle_task_done(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let doc_root = state.doc_root.clone();
    let result = tokio::task::spawn_blocking(move || task_done(&doc_root, &id)).await;
    match result {
        Ok(Ok(())) => (StatusCode::OK, axum::Json(ActionResponse::success())),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, axum::Json(ActionResponse::err(e))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ActionResponse::err(e.to_string()))),
    }
}

// ─── Task Reopen ─────────────────────────────────────────────────────────────

async fn handle_task_reopen(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let doc_root = state.doc_root.clone();
    let result = tokio::task::spawn_blocking(move || task_reopen(&doc_root, &id)).await;
    match result {
        Ok(Ok(())) => (StatusCode::OK, axum::Json(ActionResponse::success())),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, axum::Json(ActionResponse::err(e))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ActionResponse::err(e.to_string()))),
    }
}

// ─── Issue Close ─────────────────────────────────────────────────────────────

async fn handle_issue_close(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let doc_root = state.doc_root.clone();
    let result = tokio::task::spawn_blocking(move || issue_close(&doc_root, &id)).await;
    match result {
        Ok(Ok(())) => (StatusCode::OK, axum::Json(ActionResponse::success())),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, axum::Json(ActionResponse::err(e))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ActionResponse::err(e.to_string()))),
    }
}

// ─── Issue Reopen ────────────────────────────────────────────────────────────

async fn handle_issue_reopen(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let doc_root = state.doc_root.clone();
    let result = tokio::task::spawn_blocking(move || issue_reopen(&doc_root, &id)).await;
    match result {
        Ok(Ok(())) => (StatusCode::OK, axum::Json(ActionResponse::success())),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, axum::Json(ActionResponse::err(e))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ActionResponse::err(e.to_string()))),
    }
}

// ─── File Operations ─────────────────────────────────────────────────────────

fn task_done(doc_root: &Path, id: &str) -> Result<(), String> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return Err("Task directory does not exist".to_string());
    }

    let normalized = crate::core::item_id::normalize_full(id);
    let all_files = crate::core::task_store::iter_task_files(&task_dir);

    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            if content.contains(&format!("- [ ] {}:", normalized)) {
                let new_content = content.replace(
                    &format!("- [ ] {}:", normalized),
                    &format!("- [x] {}:", normalized),
                );

                // If all tasks done, rename to done_
                if !crate::core::task_store::has_undone_items(&new_content) {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let done_filename = format!("done_{}", filename);
                    let done_path = task_dir.join(&done_filename);
                    fs::write(path, &new_content).map_err(|e| e.to_string())?;
                    fs::rename(path, &done_path).map_err(|e| e.to_string())?;
                } else {
                    fs::write(path, &new_content).map_err(|e| e.to_string())?;
                }
                return Ok(());
            }
        }
    }

    Err(format!("Pending task {} not found", id))
}

fn task_reopen(doc_root: &Path, id: &str) -> Result<(), String> {
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return Err("Task directory does not exist".to_string());
    }

    let normalized = crate::core::item_id::normalize_full(id);

    // Search in all task files including done_ prefixed ones
    let entries = fs::read_dir(&task_dir).map_err(|e| e.to_string())?;
    let all_files: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".md") && (name.starts_with("task_") || name.starts_with("done_"))
        })
        .map(|e| e.path())
        .collect();

    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            if content.contains(&format!("- [x] {}:", normalized)) {
                let new_content = content.replace(
                    &format!("- [x] {}:", normalized),
                    &format!("- [ ] {}:", normalized),
                );
                fs::write(path, &new_content).map_err(|e| e.to_string())?;

                // If file has done_ prefix, remove it
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                if filename.starts_with("done_") {
                    let new_name = filename.strip_prefix("done_").unwrap();
                    let new_path = task_dir.join(new_name);
                    fs::rename(path, &new_path).map_err(|e| e.to_string())?;
                }
                return Ok(());
            }
        }
    }

    Err(format!("Completed task {} not found", id))
}

fn issue_close(doc_root: &Path, id: &str) -> Result<(), String> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return Err("Issue directory does not exist".to_string());
    }

    let normalized = crate::core::item_id::normalize_full(id);

    let entries = fs::read_dir(&issue_dir).map_err(|e| e.to_string())?;
    let all_files: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".md") && name.starts_with("issue_")
        })
        .map(|e| e.path())
        .collect();

    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            // Match both ASCII and full-width colons
            let marker_ascii = format!("- [ ] {}: ", normalized);
            let marker_fullwidth = format!("- [ ] {}：", normalized);
            if content.contains(&marker_ascii) || content.contains(&marker_fullwidth) {
                let new_content = content
                    .lines()
                    .map(|line| {
                        if line.starts_with("- [ ]")
                            && (line.contains(&format!("{}：", normalized))
                                || line.contains(&format!("{}: ", normalized)))
                        {
                            line.replacen("- [ ]", "- [x]", 1)
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
                    format!("{}\n", new_content)
                } else {
                    new_content
                };
                fs::write(path, &final_content).map_err(|e| e.to_string())?;

                // If all issues closed, rename to closed_
                let updated = fs::read_to_string(path).unwrap_or_default();
                let total: usize = updated.lines().filter(|l| l.starts_with("- [")).count();
                let done: usize = updated.lines().filter(|l| l.starts_with("- [x]")).count();
                if total > 0 && total == done {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let new_filename = format!("closed_{}", filename);
                    let new_path = issue_dir.join(&new_filename);
                    fs::rename(path, &new_path).map_err(|e| e.to_string())?;
                }
                return Ok(());
            }
        }
    }

    Err(format!("Open issue {} not found", id))
}

fn issue_reopen(doc_root: &Path, id: &str) -> Result<(), String> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return Err("Issue directory does not exist".to_string());
    }

    let normalized = crate::core::item_id::normalize_full(id);

    let entries = fs::read_dir(&issue_dir).map_err(|e| e.to_string())?;
    let all_files: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".md")
                && (name.starts_with("issue_") || name.starts_with("closed_"))
        })
        .map(|e| e.path())
        .collect();

    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            // Match both ASCII and full-width colons
            let marker_ascii = format!("- [x] {}: ", normalized);
            let marker_fullwidth = format!("- [x] {}：", normalized);
            if content.contains(&marker_ascii) || content.contains(&marker_fullwidth) {
                let new_content = content
                    .lines()
                    .map(|line| {
                        if line.starts_with("- [x]")
                            && (line.contains(&format!("{}：", normalized))
                                || line.contains(&format!("{}: ", normalized)))
                        {
                            line.replacen("- [x]", "- [ ]", 1)
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let final_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
                    format!("{}\n", new_content)
                } else {
                    new_content
                };
                fs::write(path, &final_content).map_err(|e| e.to_string())?;

                // Remove closed_ prefix if present
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                if filename.starts_with("closed_") {
                    let new_name = filename.strip_prefix("closed_").unwrap();
                    let new_path = issue_dir.join(new_name);
                    fs::rename(path, &new_path).map_err(|e| e.to_string())?;
                }
                return Ok(());
            }
        }
    }

    Err(format!("Closed issue {} not found", id))
}

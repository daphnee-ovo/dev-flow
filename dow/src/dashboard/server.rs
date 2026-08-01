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
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::dashboard::data;
use crate::error::DowError;

#[derive(Embed)]
#[folder = "dashboard-frontend/"]
struct Assets;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) doc_root: PathBuf,
    pub(crate) notify_tx: Arc<broadcast::Sender<()>>,
    pub(crate) connections: Arc<AtomicUsize>,
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
        .route("/api/task/{id}/update", post(handle_task_update))
        .route("/api/issue/{id}/close", post(handle_issue_close))
        .route("/api/issue/{id}/reopen", post(handle_issue_reopen))
        .route("/api/issue/{id}/update", post(handle_issue_update))
        .nest("/api/v1", crate::dashboard::api_v1::build_v1_router())
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

#[derive(Deserialize)]
struct UpdateBody {
    field: String,
    value: String,
}

#[derive(Deserialize, Default)]
struct IssueCloseBody {
    #[serde(default)]
    fix: Option<String>,
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

// ─── Task Update ─────────────────────────────────────────────────────────────

async fn handle_task_update(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    axum::Json(body): axum::Json<UpdateBody>,
) -> impl IntoResponse {
    let doc_root = state.doc_root.clone();
    let result = tokio::task::spawn_blocking(move || {
        task_update_field(&doc_root, &id, &body.field, &body.value)
    }).await;
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
    body: Option<axum::Json<IssueCloseBody>>,
) -> impl IntoResponse {
    let doc_root = state.doc_root.clone();
    let fix_text = body.and_then(|b| b.0.fix);
    let result = tokio::task::spawn_blocking(move || {
        issue_close(&doc_root, &id, fix_text.as_deref())
    }).await;
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

// ─── Issue Update ────────────────────────────────────────────────────────────

async fn handle_issue_update(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    axum::Json(body): axum::Json<UpdateBody>,
) -> impl IntoResponse {
    let doc_root = state.doc_root.clone();
    let result = tokio::task::spawn_blocking(move || {
        issue_update_field(&doc_root, &id, &body.field, &body.value)
    }).await;
    match result {
        Ok(Ok(())) => (StatusCode::OK, axum::Json(ActionResponse::success())),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, axum::Json(ActionResponse::err(e))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ActionResponse::err(e.to_string()))),
    }
}

// ─── File Operations ─────────────────────────────────────────────────────────

pub(crate) fn task_done(doc_root: &Path, id: &str) -> Result<(), String> {
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

pub(crate) fn task_reopen(doc_root: &Path, id: &str) -> Result<(), String> {
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


pub(crate) fn issue_close(doc_root: &Path, id: &str, fix_text: Option<&str>) -> Result<(), String> {
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
            if !entry_matches_id(&content, &normalized, false) {
                continue;
            }

            // Check if fix field is populated
            let fix_value = get_field_in_entry(&content, &normalized, "fix");
            let fix_empty = fix_value.as_ref().map_or(true, |v| v.trim().is_empty());

            let content_to_close = if fix_empty {
                match fix_text {
                    Some(text) if !text.trim().is_empty() => {
                        // Write fix field first
                        let has_fix = has_field_in_entry(&content, &normalized, "fix");
                        if has_fix {
                            replace_field_in_entry(&content, &normalized, "fix", text)
                        } else {
                            insert_field_in_entry(&content, &normalized, "fix", text)
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Cannot close {}: fix field is empty. Provide a fix description.",
                            id
                        ));
                    }
                }
            } else {
                content.clone()
            };

            // Close: change [ ] to [x]
            let closed = close_issue_in_content(&content_to_close, &normalized);
            fs::write(path, &closed).map_err(|e| e.to_string())?;

            // If all issues closed, rename to closed_
            let final_content = fs::read_to_string(path).unwrap_or_default();
            let total: usize = final_content.lines().filter(|l| l.starts_with("- [")).count();
            let done: usize = final_content.lines().filter(|l| l.starts_with("- [x]")).count();
            if total > 0 && total == done {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                let new_filename = format!("closed_{}", filename);
                let new_path = issue_dir.join(&new_filename);
                fs::rename(path, &new_path).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
    }

    Err(format!("Open issue {} not found", id))
}

pub(crate) fn issue_reopen(doc_root: &Path, id: &str) -> Result<(), String> {
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
            if !entry_matches_id(&content, &normalized, true) {
                continue;
            }

            let new_content = content
                .lines()
                .map(|line| {
                    if line.starts_with("- [x]") && line_contains_id(line, &normalized) {
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

    Err(format!("Closed issue {} not found", id))
}

// ─── Field Update Operations ─────────────────────────────────────────────────

const TASK_ALLOWED_FIELDS: &[(&str, &[&str])] = &[
    ("priority", &["P0", "P1", "P2"]),
    ("type", &["feat", "fix", "refactor", "docs", "perf", "test", "style"]),
    ("complexity", &["S", "M", "L"]),
];

const ISSUE_ALLOWED_FIELDS: &[(&str, &[&str])] = &[
    ("severity", &["P0", "P1", "P2"]),
];

pub(crate) fn task_update_field(doc_root: &Path, id: &str, field: &str, value: &str) -> Result<(), String> {
    // Validate field and value
    let allowed = TASK_ALLOWED_FIELDS
        .iter()
        .find(|(f, _)| *f == field)
        .ok_or_else(|| format!("Field '{}' is not editable", field))?;

    if !allowed.1.contains(&value) {
        return Err(format!(
            "Invalid value '{}' for field '{}'. Allowed: {:?}",
            value, field, allowed.1
        ));
    }

    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return Err("Task directory does not exist".to_string());
    }

    let normalized = crate::core::item_id::normalize_full(id);

    // Search in all task files (including done_)
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
            if !content.contains(&format!("{}:", normalized)) {
                continue;
            }

            let new_content = replace_field_in_entry(&content, &normalized, field, value);
            if new_content == content {
                return Err(format!("Field '{}' not found in task {}", field, id));
            }
            fs::write(path, &new_content).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    Err(format!("Task {} not found", id))
}

pub(crate) fn issue_update_field(doc_root: &Path, id: &str, field: &str, value: &str) -> Result<(), String> {
    // Validate field and value
    let allowed = ISSUE_ALLOWED_FIELDS
        .iter()
        .find(|(f, _)| *f == field)
        .ok_or_else(|| format!("Field '{}' is not editable", field))?;

    if !allowed.1.contains(&value) {
        return Err(format!(
            "Invalid value '{}' for field '{}'. Allowed: {:?}",
            value, field, allowed.1
        ));
    }

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
            if !content.contains(&normalized) {
                continue;
            }

            let new_content = replace_field_in_entry(&content, &normalized, field, value);
            if new_content == content {
                return Err(format!("Field '{}' not found in issue {}", field, id));
            }
            fs::write(path, &new_content).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    Err(format!("Issue {} not found", id))
}

// ─── Entry Helpers ───────────────────────────────────────────────────────────

/// Check if a line contains the given ID (with ASCII or full-width colon separator)
fn line_contains_id(line: &str, id: &str) -> bool {
    line.contains(&format!("{}\u{ff1a}", id)) || line.contains(&format!("{}: ", id))
}

/// Check if an entry for the given ID exists in the content
/// `checked` = true means look for [x], false means look for [ ]
fn entry_matches_id(content: &str, id: &str, checked: bool) -> bool {
    let prefix = if checked { "- [x]" } else { "- [ ]" };
    content.lines().any(|line| line.starts_with(prefix) && line_contains_id(line, id))
}

/// Close an issue entry by changing [ ] to [x], preserving line endings
fn close_issue_in_content(content: &str, id: &str) -> String {
    let new_content = content
        .lines()
        .map(|line| {
            if line.starts_with("- [ ]") && line_contains_id(line, id) {
                line.replacen("- [ ]", "- [x]", 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if content.ends_with('\n') && !new_content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    }
}

/// Check if a field exists in an entry block for the given ID
fn has_field_in_entry(content: &str, id: &str, field: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_entry = false;
    let field_prefix = format!("- {}:", field);

    for line in &lines {
        if (line.starts_with("- [ ]") || line.starts_with("- [x]")) && line_contains_id(line, id) {
            in_entry = true;
            continue;
        }
        if in_entry {
            if line.starts_with("- [ ]") || line.starts_with("- [x]") {
                break;
            }
            if line.trim().starts_with(&field_prefix) {
                return true;
            }
        }
    }
    false
}

/// Get the value of a field in an entry block for the given ID
fn get_field_in_entry(content: &str, id: &str, field: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_entry = false;
    let field_prefix = format!("- {}:", field);
    // Also match full-width colon
    let field_prefix_fw = format!("- {}\u{ff1a}", field);

    for line in &lines {
        if (line.starts_with("- [ ]") || line.starts_with("- [x]")) && line_contains_id(line, id) {
            in_entry = true;
            continue;
        }
        if in_entry {
            if line.starts_with("- [ ]") || line.starts_with("- [x]") {
                break;
            }
            let trimmed = line.trim();
            if trimmed.starts_with(&field_prefix) {
                return Some(trimmed[field_prefix.len()..].trim().to_string());
            }
            if trimmed.starts_with(&field_prefix_fw) {
                return Some(trimmed[field_prefix_fw.len()..].trim().to_string());
            }
        }
    }
    None
}

/// Replace a field value in an entry block for the given ID
pub(crate) fn replace_field_in_entry(content: &str, id: &str, field: &str, new_value: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_entry = false;
    let mut replaced = false;
    let field_prefix = format!("- {}:", field);
    let field_prefix_fw = format!("- {}\u{ff1a}", field);

    for line in &lines {
        if (line.starts_with("- [ ]") || line.starts_with("- [x]")) && line_contains_id(line, id) {
            in_entry = true;
            result.push(line.to_string());
            continue;
        }
        if in_entry && !replaced {
            if line.starts_with("- [ ]") || line.starts_with("- [x]") {
                in_entry = false;
                result.push(line.to_string());
                continue;
            }
            let trimmed = line.trim();
            if trimmed.starts_with(&field_prefix) || trimmed.starts_with(&field_prefix_fw) {
                // Preserve indentation
                let indent = line.len() - line.trim_start().len();
                let spaces: String = " ".repeat(indent);
                result.push(format!("{}- {}: {}", spaces, field, new_value));
                replaced = true;
                continue;
            }
        }
        result.push(line.to_string());
    }

    let new_content = result.join("\n");
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    }
}

/// Insert a new field in an entry block for the given ID (after the last existing field)
pub(crate) fn insert_field_in_entry(content: &str, id: &str, field: &str, value: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_entry = false;
    let mut inserted = false;
    let mut last_field_idx = 0;

    // First pass: find the last field line index in the entry
    for (i, line) in lines.iter().enumerate() {
        if (line.starts_with("- [ ]") || line.starts_with("- [x]")) && line_contains_id(line, id) {
            in_entry = true;
            last_field_idx = i;
            continue;
        }
        if in_entry {
            if line.starts_with("- [ ]") || line.starts_with("- [x]") {
                break;
            }
            if line.trim().starts_with("- ") {
                last_field_idx = i;
            }
        }
    }

    // Second pass: insert after last_field_idx
    in_entry = false;
    for (i, line) in lines.iter().enumerate() {
        result.push(line.to_string());
        if i == last_field_idx && !inserted {
            // Detect indentation from the line
            let indent = line.len() - line.trim_start().len();
            let spaces: String = " ".repeat(indent);
            result.push(format!("{}- {}: {}", spaces, field, value));
            inserted = true;
        }
    }

    let new_content = result.join("\n");
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    }
}

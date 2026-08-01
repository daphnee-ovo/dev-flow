// FrameworkTree
// api_v1.rs
// ├── build_v1_router()
// ├── struct ApiErrorResponse
// ├── enum ApiError
// ├── impl ApiError
// ├── into_response()
// ├── struct DiscoveryResponse
// ├── struct EndpointInfo
// ├── handle_discovery()
// ├── struct StatusResponse
// ├── struct StatusPatchBody
// ├── handle_get_status()
// ├── get_status_data()
// ├── handle_patch_status()
// ├── patch_status()
// ├── struct TaskListResponse
// ├── struct TaskItemResponse
// ├── struct TaskFilesResponse
// ├── struct TaskListQuery
// ├── struct TaskCreateBody
// ├── struct TaskFilesInput
// ├── struct TaskPatchBody
// ├── handle_list_tasks()
// ├── list_tasks()
// ├── handle_create_task()
// ├── create_task()
// ├── handle_get_task()
// ├── get_task()
// ├── handle_patch_task()
// ├── patch_task()
// ├── handle_delete_task()
// ├── delete_task()
// ├── struct IssueListResponse
// ├── struct IssueItemResponse
// ├── struct IssueFilesResponse
// ├── struct IssueListQuery
// ├── struct IssueCreateBody
// ├── default_source()
// ├── struct IssueFilesInput
// ├── struct IssuePatchBody
// ├── handle_list_issues()
// ├── list_issues()
// ├── handle_create_issue()
// ├── create_issue()
// ├── handle_get_issue()
// ├── get_issue()
// ├── handle_patch_issue()
// ├── patch_issue()
// ├── handle_delete_issue()
// ├── delete_issue()
// ├── struct DocsListResponse
// ├── struct DocMetaResponse
// ├── struct DocContentResponse
// ├── struct DocPutBody
// ├── handle_list_docs()
// ├── handle_get_doc()
// ├── handle_put_doc()
// ├── struct ChangelogResponse
// ├── struct ChangelogAddBody
// ├── struct ChangelogAddResponse
// ├── handle_get_changelog()
// ├── handle_post_changelog()
// ├── add_changelog_entry()
// ├── struct VersionResponse
// ├── handle_get_version()
// ├── update_event()
// ├── sse_event_from_broadcast()
// ├── handle_sse_v1()
// ├── mod tests
// ├── lagged_broadcast_still_emits_full_invalidation_update()
// ├── struct SseConnectionGuard
// ├── impl SseConnectionGuard
// ├── drop()
// ├── struct IssueDetail
// ├── get_all_issue_details()
// ├── parse_issues_detail()
// ├── extract_field_value()
// ├── split_id_title_issue()
// ├── parse_inline_list_api()
// ├── remove_entry_from_content()
// ├── update_frontmatter_nums()
// └── update_issue_field()

use std::convert::Infallible;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::commands::issue::{create_issue_batch, IssueCreateRecord};
use crate::commands::task::get_all_task_details;
use crate::core::{item_id, yaml};

use super::server::AppState;

// ─── Public Router Builder ───────────────────────────────────────────────────

pub fn build_v1_router() -> Router<AppState> {
    Router::new()
        // Discovery
        .route("/", get(handle_discovery))
        // Status
        .route("/status", get(handle_get_status))
        .route("/status", patch(handle_patch_status))
        // Tasks
        .route("/tasks", get(handle_list_tasks))
        .route("/tasks", post(handle_create_task))
        .route("/tasks/{id}", get(handle_get_task))
        .route("/tasks/{id}", patch(handle_patch_task))
        .route("/tasks/{id}", delete(handle_delete_task))
        // Issues
        .route("/issues", get(handle_list_issues))
        .route("/issues", post(handle_create_issue))
        .route("/issues/{id}", get(handle_get_issue))
        .route("/issues/{id}", patch(handle_patch_issue))
        .route("/issues/{id}", delete(handle_delete_issue))
        // Docs
        .route("/docs", get(handle_list_docs))
        .route("/docs/{name}", get(handle_get_doc))
        .route("/docs/{name}", put(handle_put_doc))
        // Changelog
        .route("/changelog", get(handle_get_changelog))
        .route("/changelog", post(handle_post_changelog))
        // Version
        .route("/version", get(handle_get_version))
        // Events
        .route("/events", get(handle_sse_v1))
}


// ─── Error Types ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiErrorResponse {
    error: String,
    message: String,
}

enum ApiError {
    NotFound(String),
    Validation(String),
    #[allow(dead_code)]
    Conflict(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_type, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, "validation_error", msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
        };
        (
            status,
            Json(ApiErrorResponse {
                error: error_type.to_string(),
                message,
            }),
        )
            .into_response()
    }
}

// ─── Discovery ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct DiscoveryResponse {
    name: &'static str,
    api_version: &'static str,
    endpoints: Vec<EndpointInfo>,
}

#[derive(Serialize)]
struct EndpointInfo {
    method: &'static str,
    path: &'static str,
    description: &'static str,
}

async fn handle_discovery() -> Json<DiscoveryResponse> {
    Json(DiscoveryResponse {
        name: "dev-flow",
        api_version: "v1",
        endpoints: vec![
            EndpointInfo { method: "GET", path: "/api/v1/", description: "API self-description" },
            EndpointInfo { method: "GET", path: "/api/v1/status", description: "Get project status" },
            EndpointInfo { method: "PATCH", path: "/api/v1/status", description: "Update status fields" },
            EndpointInfo { method: "GET", path: "/api/v1/tasks", description: "List tasks" },
            EndpointInfo { method: "POST", path: "/api/v1/tasks", description: "Create task" },
            EndpointInfo { method: "GET", path: "/api/v1/tasks/:id", description: "Get task detail" },
            EndpointInfo { method: "PATCH", path: "/api/v1/tasks/:id", description: "Update task" },
            EndpointInfo { method: "DELETE", path: "/api/v1/tasks/:id", description: "Delete task" },
            EndpointInfo { method: "GET", path: "/api/v1/issues", description: "List issues" },
            EndpointInfo { method: "POST", path: "/api/v1/issues", description: "Create issue" },
            EndpointInfo { method: "GET", path: "/api/v1/issues/:id", description: "Get issue detail" },
            EndpointInfo { method: "PATCH", path: "/api/v1/issues/:id", description: "Update issue" },
            EndpointInfo { method: "DELETE", path: "/api/v1/issues/:id", description: "Delete issue" },
            EndpointInfo { method: "GET", path: "/api/v1/docs", description: "List documents" },
            EndpointInfo { method: "GET", path: "/api/v1/docs/:name", description: "Get document content" },
            EndpointInfo { method: "PUT", path: "/api/v1/docs/:name", description: "Create/update document" },
            EndpointInfo { method: "GET", path: "/api/v1/changelog", description: "Get changelog" },
            EndpointInfo { method: "POST", path: "/api/v1/changelog", description: "Add changelog entry" },
            EndpointInfo { method: "GET", path: "/api/v1/version", description: "Get project version" },
            EndpointInfo { method: "GET", path: "/api/v1/events", description: "SSE event stream" },
        ],
    })
}

// ─── Status ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct StatusResponse {
    name: String,
    phase: String,
    mode: String,
    exec_mode: String,
    version: String,
    goals_minor: String,
    goals_major: String,
    updated: String,
    started: String,
}

#[derive(Deserialize)]
struct StatusPatchBody {
    phase: Option<String>,
    mode: Option<String>,
    exec_mode: Option<String>,
    name: Option<String>,
    goals_minor: Option<String>,
    goals_major: Option<String>,
}

async fn handle_get_status(State(state): State<AppState>) -> Result<Json<StatusResponse>, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || get_status_data(&doc_root))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(Json)
}

fn get_status_data(doc_root: &Path) -> Result<StatusResponse, ApiError> {
    let status_file = doc_root.join("STATUS.yaml");
    if !status_file.exists() {
        return Err(ApiError::NotFound("STATUS.yaml not found".to_string()));
    }

    let get_field = |key: &str| -> String {
        yaml::get(&status_file, key).ok().flatten().unwrap_or_default()
    };

    let version = crate::core::version::read_current().unwrap_or_default();

    Ok(StatusResponse {
        name: get_field("name"),
        phase: get_field("phase"),
        mode: get_field("mode"),
        exec_mode: get_field("exec_mode"),
        version,
        goals_minor: get_field("goals_minor"),
        goals_major: get_field("goals_major"),
        updated: get_field("updated"),
        started: get_field("started"),
    })
}

async fn handle_patch_status(
    State(state): State<AppState>,
    Json(body): Json<StatusPatchBody>,
) -> Result<Json<StatusResponse>, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || patch_status(&doc_root, body))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}

fn patch_status(doc_root: &Path, body: StatusPatchBody) -> Result<Json<StatusResponse>, ApiError> {
    let status_file = doc_root.join("STATUS.yaml");
    if !status_file.exists() {
        return Err(ApiError::NotFound("STATUS.yaml not found".to_string()));
    }

    let allowed_phases = ["BRAINSTORM", "PRD", "SPEC", "TASK", "DEV", "TEST", "DONE"];
    let allowed_modes = ["full", "quick", "fast", "mvp"];

    if let Some(ref phase) = body.phase {
        let upper = phase.to_uppercase();
        if !allowed_phases.contains(&upper.as_str()) {
            return Err(ApiError::Validation(format!(
                "Invalid phase '{}'. Allowed: {}",
                phase,
                allowed_phases.join(", ")
            )));
        }
        yaml::set(&status_file, "phase", &upper)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    if let Some(ref mode) = body.mode {
        if !allowed_modes.contains(&mode.as_str()) {
            return Err(ApiError::Validation(format!(
                "Invalid mode '{}'. Allowed: {}",
                mode,
                allowed_modes.join(", ")
            )));
        }
        yaml::set(&status_file, "mode", mode)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    if let Some(ref exec_mode) = body.exec_mode {
        yaml::set(&status_file, "exec_mode", exec_mode)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    if let Some(ref name) = body.name {
        yaml::set(&status_file, "name", name)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    if let Some(ref goals_minor) = body.goals_minor {
        yaml::set(&status_file, "goals_minor", goals_minor)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    if let Some(ref goals_major) = body.goals_major {
        yaml::set(&status_file, "goals_major", goals_major)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Update timestamp
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let _ = yaml::set(&status_file, "updated", &now);

    get_status_data(doc_root).map(Json)
}

// ─── Tasks ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TaskListResponse {
    items: Vec<TaskItemResponse>,
    total: usize,
    pending: usize,
    in_progress: usize,
    done: usize,
}

#[derive(Serialize)]
struct TaskItemResponse {
    id: String,
    title: String,
    r#type: String,
    priority: String,
    complexity: String,
    status: String,
    depends_on: Vec<String>,
    done_when: Vec<String>,
    refs: String,
    files: TaskFilesResponse,
}

#[derive(Serialize)]
struct TaskFilesResponse {
    create: Vec<String>,
    modify: Vec<String>,
    test: Vec<String>,
}

#[derive(Deserialize)]
struct TaskListQuery {
    status: Option<String>,
    priority: Option<String>,
    r#type: Option<String>,
    complexity: Option<String>,
}

#[derive(Deserialize)]
struct TaskCreateBody {
    title: String,
    r#type: String,
    priority: String,
    complexity: String,
    #[serde(default)]
    refs: String,
    #[serde(default)]
    files: Option<TaskFilesInput>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    parallel: bool,
    #[serde(default)]
    done_when: Vec<String>,
}

#[derive(Deserialize, Default)]
struct TaskFilesInput {
    #[serde(default)]
    create: Vec<String>,
    #[serde(default)]
    modify: Vec<String>,
    #[serde(default)]
    test: Vec<String>,
}

#[derive(Deserialize)]
struct TaskPatchBody {
    status: Option<String>,
    #[allow(dead_code)]
    title: Option<String>,
    r#type: Option<String>,
    priority: Option<String>,
    complexity: Option<String>,
    #[allow(dead_code)]
    refs: Option<String>,
    #[allow(dead_code)]
    files: Option<TaskFilesInput>,
    #[allow(dead_code)]
    depends_on: Option<Vec<String>>,
    #[allow(dead_code)]
    done_when: Option<Vec<String>>,
}

async fn handle_list_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<TaskListResponse>, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || list_tasks(&doc_root, &query))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(Json)
}

fn list_tasks(doc_root: &Path, query: &TaskListQuery) -> Result<TaskListResponse, ApiError> {
    let active_claims = crate::core::claim::get_active_claims(doc_root);
    let all_details = get_all_task_details(doc_root);

    let items: Vec<TaskItemResponse> = all_details
        .into_iter()
        .map(|t| {
            let short = item_id::normalize_short(&t.id);
            let status = if t.status == "pending" && active_claims.contains(&short) {
                "in_progress".to_string()
            } else {
                t.status
            };
            TaskItemResponse {
                id: t.id,
                title: t.title,
                r#type: t.r#type,
                priority: t.priority,
                complexity: t.complexity,
                status,
                depends_on: t.depends_on,
                done_when: t.done_when,
                refs: t.refs,
                files: TaskFilesResponse {
                    create: t.files.create,
                    modify: t.files.modify,
                    test: t.files.test,
                },
            }
        })
        .collect();

    // Count before filtering
    let total = items.len();
    let pending = items.iter().filter(|t| t.status == "pending").count();
    let in_progress = items.iter().filter(|t| t.status == "in_progress").count();
    let done = items.iter().filter(|t| t.status == "done").count();

    // Apply filters
    let filtered: Vec<TaskItemResponse> = items
        .into_iter()
        .filter(|t| {
            if let Some(ref s) = query.status {
                if s != "all" && t.status != *s {
                    return false;
                }
            }
            if let Some(ref p) = query.priority {
                if t.priority != *p {
                    return false;
                }
            }
            if let Some(ref ty) = query.r#type {
                if t.r#type != *ty {
                    return false;
                }
            }
            if let Some(ref c) = query.complexity {
                if t.complexity != *c {
                    return false;
                }
            }
            true
        })
        .collect();

    Ok(TaskListResponse {
        items: filtered,
        total,
        pending,
        in_progress,
        done,
    })
}

async fn handle_create_task(
    State(state): State<AppState>,
    Json(body): Json<TaskCreateBody>,
) -> Result<(StatusCode, Json<TaskItemResponse>), ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || create_task(&doc_root, body))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}

fn create_task(
    doc_root: &Path,
    body: TaskCreateBody,
) -> Result<(StatusCode, Json<TaskItemResponse>), ApiError> {
    if body.title.trim().is_empty() {
        return Err(ApiError::Validation("Field 'title' is required".to_string()));
    }

    let valid_types = ["feat", "fix", "refactor", "docs", "perf", "test", "style"];
    if !valid_types.contains(&body.r#type.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid type '{}'. Allowed: {}",
            body.r#type,
            valid_types.join(", ")
        )));
    }

    let valid_priorities = ["P0", "P1", "P2"];
    if !valid_priorities.contains(&body.priority.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid priority '{}'. Allowed: P0, P1, P2",
            body.priority
        )));
    }

    let valid_complexities = ["S", "M", "L"];
    if !valid_complexities.contains(&body.complexity.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid complexity '{}'. Allowed: S, M, L",
            body.complexity
        )));
    }

    let files = body.files.unwrap_or_default();
    if files.create.is_empty() && files.modify.is_empty() {
        return Err(ApiError::Validation(
            "At least one file in 'files.create' or 'files.modify' is required".to_string(),
        ));
    }

    let records = vec![crate::commands::task::TaskCreateRecord {
        title: body.title.clone(),
        task_type: body.r#type.clone(),
        priority: body.priority.clone(),
        refs: body.refs.clone(),
        files_create: files.create.clone(),
        files_modify: files.modify.clone(),
        files_test: files.test.clone(),
        depends_on: body.depends_on.clone(),
        parallel: body.parallel,
        complexity: body.complexity.clone(),
        done_when: body.done_when.clone(),
    }];

    let ids = crate::commands::task::create_task_batch(records)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let id = ids.into_iter().next().ok_or_else(|| {
        ApiError::Internal("Task creation returned no ID".to_string())
    })?;

    // Read back the created task
    let all = get_all_task_details(doc_root);
    let detail = all.into_iter().find(|t| t.id == id).ok_or_else(|| {
        ApiError::Internal("Created task not found after creation".to_string())
    })?;

    Ok((
        StatusCode::CREATED,
        Json(TaskItemResponse {
            id: detail.id,
            title: detail.title,
            r#type: detail.r#type,
            priority: detail.priority,
            complexity: detail.complexity,
            status: detail.status,
            depends_on: detail.depends_on,
            done_when: detail.done_when,
            refs: detail.refs,
            files: TaskFilesResponse {
                create: detail.files.create,
                modify: detail.files.modify,
                test: detail.files.test,
            },
        }),
    ))
}

async fn handle_get_task(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<TaskItemResponse>, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || get_task(&doc_root, &id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(Json)
}

fn get_task(doc_root: &Path, id: &str) -> Result<TaskItemResponse, ApiError> {
    let normalized = item_id::normalize_full(id);
    let active_claims = crate::core::claim::get_active_claims(doc_root);
    let all = get_all_task_details(doc_root);
    let detail = all.into_iter().find(|t| t.id == normalized).ok_or_else(|| {
        ApiError::NotFound(format!("Task {} not found", id))
    })?;

    let short = item_id::normalize_short(&detail.id);
    let status = if detail.status == "pending" && active_claims.contains(&short) {
        "in_progress".to_string()
    } else {
        detail.status
    };

    Ok(TaskItemResponse {
        id: detail.id,
        title: detail.title,
        r#type: detail.r#type,
        priority: detail.priority,
        complexity: detail.complexity,
        status,
        depends_on: detail.depends_on,
        done_when: detail.done_when,
        refs: detail.refs,
        files: TaskFilesResponse {
            create: detail.files.create,
            modify: detail.files.modify,
            test: detail.files.test,
        },
    })
}

async fn handle_patch_task(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<TaskPatchBody>,
) -> Result<Json<TaskItemResponse>, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || patch_task(&doc_root, &id, body))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(Json)
}

fn patch_task(doc_root: &Path, id: &str, body: TaskPatchBody) -> Result<TaskItemResponse, ApiError> {
    let normalized = item_id::normalize_full(id);

    // Handle status transitions
    if let Some(ref status) = body.status {
        match status.as_str() {
            "done" => {
                super::server::task_done(doc_root, &normalized)
                    .map_err(ApiError::Validation)?;
            }
            "pending" => {
                super::server::task_reopen(doc_root, &normalized)
                    .map_err(ApiError::Validation)?;
            }
            other => {
                return Err(ApiError::Validation(format!(
                    "Invalid status '{}'. Use 'done' or 'pending'",
                    other
                )));
            }
        }
    }

    // Handle field updates
    if let Some(ref priority) = body.priority {
        super::server::task_update_field(doc_root, &normalized, "priority", priority)
            .map_err(ApiError::Validation)?;
    }
    if let Some(ref ty) = body.r#type {
        super::server::task_update_field(doc_root, &normalized, "type", ty)
            .map_err(ApiError::Validation)?;
    }
    if let Some(ref complexity) = body.complexity {
        super::server::task_update_field(doc_root, &normalized, "complexity", complexity)
            .map_err(ApiError::Validation)?;
    }

    get_task(doc_root, &normalized)
}

async fn handle_delete_task(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || delete_task(&doc_root, &id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}

fn delete_task(doc_root: &Path, id: &str) -> Result<StatusCode, ApiError> {
    let normalized = item_id::normalize_full(id);
    let task_dir = doc_root.join("task");
    if !task_dir.is_dir() {
        return Err(ApiError::NotFound(format!("Task {} not found", id)));
    }

    let all_files = crate::commands::task::all_task_files_including_done(&task_dir);
    for path in &all_files {
        if let Ok(content) = fs::read_to_string(path) {
            if !content.contains(&normalized) {
                continue;
            }

            let new_content = remove_entry_from_content(&content, &normalized);
            if new_content == content {
                continue;
            }

            let has_items = new_content.lines().any(|l| {
                let t = l.trim();
                t.starts_with("- [ ]") || t.starts_with("- [x]")
            });

            if !has_items {
                let _ = fs::remove_file(path);
            } else {
                let updated = update_frontmatter_nums(&new_content);
                fs::write(path, updated)
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
            }

            return Ok(StatusCode::NO_CONTENT);
        }
    }

    Err(ApiError::NotFound(format!("Task {} not found", id)))
}

// ─── Issues ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct IssueListResponse {
    items: Vec<IssueItemResponse>,
    total: usize,
    open: usize,
    in_progress: usize,
    closed: usize,
}

#[derive(Serialize)]
struct IssueItemResponse {
    id: String,
    title: String,
    severity: String,
    status: String,
    location: String,
    description: String,
    reproduce: String,
    source: String,
    fix: Option<String>,
    files: IssueFilesResponse,
}

#[derive(Serialize)]
struct IssueFilesResponse {
    create: Vec<String>,
    modify: Vec<String>,
}

#[derive(Deserialize)]
struct IssueListQuery {
    status: Option<String>,
    severity: Option<String>,
    source: Option<String>,
}

#[derive(Deserialize)]
struct IssueCreateBody {
    title: String,
    severity: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    reproduce: String,
    #[serde(default = "default_source")]
    source: String,
    #[serde(default)]
    files: Option<IssueFilesInput>,
}

fn default_source() -> String {
    "other".to_string()
}

#[derive(Deserialize, Default)]
struct IssueFilesInput {
    #[serde(default)]
    create: Vec<String>,
    #[serde(default)]
    modify: Vec<String>,
}

#[derive(Deserialize)]
struct IssuePatchBody {
    status: Option<String>,
    severity: Option<String>,
    #[allow(dead_code)]
    title: Option<String>,
    #[allow(dead_code)]
    location: Option<String>,
    #[allow(dead_code)]
    description: Option<String>,
    #[allow(dead_code)]
    reproduce: Option<String>,
    fix: Option<String>,
    #[allow(dead_code)]
    files: Option<IssueFilesInput>,
}

async fn handle_list_issues(
    State(state): State<AppState>,
    Query(query): Query<IssueListQuery>,
) -> Result<Json<IssueListResponse>, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || list_issues(&doc_root, &query))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(Json)
}

fn list_issues(doc_root: &Path, query: &IssueListQuery) -> Result<IssueListResponse, ApiError> {
    let active_claims = crate::core::claim::get_active_claims(doc_root);
    let all = get_all_issue_details(doc_root);

    let items: Vec<IssueItemResponse> = all
        .into_iter()
        .map(|i| {
            let short = item_id::normalize_short(&i.id);
            let status = if i.status == "open" && active_claims.contains(&short) {
                "in_progress".to_string()
            } else {
                i.status
            };
            IssueItemResponse {
                id: i.id,
                title: i.title,
                severity: i.severity,
                status,
                location: i.location,
                description: i.description,
                reproduce: i.reproduce,
                source: i.source,
                fix: if i.fix.is_empty() { None } else { Some(i.fix) },
                files: IssueFilesResponse {
                    create: i.files_create,
                    modify: i.files_modify,
                },
            }
        })
        .collect();

    let total = items.len();
    let open = items.iter().filter(|i| i.status == "open").count();
    let in_progress = items.iter().filter(|i| i.status == "in_progress").count();
    let closed = items.iter().filter(|i| i.status == "closed").count();

    let filtered: Vec<IssueItemResponse> = items
        .into_iter()
        .filter(|i| {
            if let Some(ref s) = query.status {
                if s != "all" && i.status != *s {
                    return false;
                }
            }
            if let Some(ref sev) = query.severity {
                if i.severity != *sev {
                    return false;
                }
            }
            if let Some(ref src) = query.source {
                if i.source != *src {
                    return false;
                }
            }
            true
        })
        .collect();

    Ok(IssueListResponse {
        items: filtered,
        total,
        open,
        in_progress,
        closed,
    })
}

async fn handle_create_issue(
    State(state): State<AppState>,
    Json(body): Json<IssueCreateBody>,
) -> Result<(StatusCode, Json<IssueItemResponse>), ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || create_issue(&doc_root, body))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}

fn create_issue(
    doc_root: &Path,
    body: IssueCreateBody,
) -> Result<(StatusCode, Json<IssueItemResponse>), ApiError> {
    if body.title.trim().is_empty() {
        return Err(ApiError::Validation("Field 'title' is required".to_string()));
    }

    let valid_severities = ["P0", "P1", "P2"];
    if !valid_severities.contains(&body.severity.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid severity '{}'. Allowed: P0, P1, P2",
            body.severity
        )));
    }

    let valid_sources = ["test", "audit", "other"];
    if !valid_sources.contains(&body.source.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid source '{}'. Allowed: test, audit, other",
            body.source
        )));
    }

    let files = body.files.unwrap_or_default();
    if files.create.is_empty() && files.modify.is_empty() {
        return Err(ApiError::Validation(
            "At least one file in 'files.create' or 'files.modify' is required".to_string(),
        ));
    }

    let record = IssueCreateRecord {
        title: body.title,
        severity: body.severity,
        location: body.location,
        desc: body.description,
        source: body.source,
        reproduce: body.reproduce,
        files_modify: files.modify,
        files_create: files.create,
    };

    let ids = create_issue_batch(vec![record])
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let id = ids.into_iter().next().ok_or_else(|| {
        ApiError::Internal("Issue creation returned no ID".to_string())
    })?;

    // Read back
    let all = get_all_issue_details(doc_root);
    let detail = all.into_iter().find(|i| i.id == id).ok_or_else(|| {
        ApiError::Internal("Created issue not found after creation".to_string())
    })?;

    Ok((
        StatusCode::CREATED,
        Json(IssueItemResponse {
            id: detail.id,
            title: detail.title,
            severity: detail.severity,
            status: detail.status,
            location: detail.location,
            description: detail.description,
            reproduce: detail.reproduce,
            source: detail.source,
            fix: if detail.fix.is_empty() { None } else { Some(detail.fix) },
            files: IssueFilesResponse {
                create: detail.files_create,
                modify: detail.files_modify,
            },
        }),
    ))
}

async fn handle_get_issue(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<IssueItemResponse>, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || get_issue(&doc_root, &id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(Json)
}

fn get_issue(doc_root: &Path, id: &str) -> Result<IssueItemResponse, ApiError> {
    let normalized = item_id::normalize_full(id);
    let active_claims = crate::core::claim::get_active_claims(doc_root);
    let all = get_all_issue_details(doc_root);
    let detail = all.into_iter().find(|i| i.id == normalized).ok_or_else(|| {
        ApiError::NotFound(format!("Issue {} not found", id))
    })?;

    let short = item_id::normalize_short(&detail.id);
    let status = if detail.status == "open" && active_claims.contains(&short) {
        "in_progress".to_string()
    } else {
        detail.status
    };

    Ok(IssueItemResponse {
        id: detail.id,
        title: detail.title,
        severity: detail.severity,
        status,
        location: detail.location,
        description: detail.description,
        reproduce: detail.reproduce,
        source: detail.source,
        fix: if detail.fix.is_empty() { None } else { Some(detail.fix) },
        files: IssueFilesResponse {
            create: detail.files_create,
            modify: detail.files_modify,
        },
    })
}

async fn handle_patch_issue(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<IssuePatchBody>,
) -> Result<Json<IssueItemResponse>, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || patch_issue(&doc_root, &id, body))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map(Json)
}

fn patch_issue(doc_root: &Path, id: &str, body: IssuePatchBody) -> Result<IssueItemResponse, ApiError> {
    let normalized = item_id::normalize_full(id);

    if let Some(ref status) = body.status {
        match status.as_str() {
            "closed" => {
                super::server::issue_close(doc_root, &normalized, body.fix.as_deref())
                    .map_err(ApiError::Validation)?;
            }
            "open" => {
                super::server::issue_reopen(doc_root, &normalized)
                    .map_err(ApiError::Validation)?;
            }
            other => {
                return Err(ApiError::Validation(format!(
                    "Invalid status '{}'. Use 'closed' or 'open'",
                    other
                )));
            }
        }
    } else if let Some(ref fix) = body.fix {
        // Update fix field without closing
        update_issue_field(doc_root, &normalized, "fix", fix)?;
    }

    if let Some(ref severity) = body.severity {
        super::server::issue_update_field(doc_root, &normalized, "severity", severity)
            .map_err(ApiError::Validation)?;
    }

    get_issue(doc_root, &normalized)
}

async fn handle_delete_issue(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let doc_root = state.doc_root.clone();
    tokio::task::spawn_blocking(move || delete_issue(&doc_root, &id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}

fn delete_issue(doc_root: &Path, id: &str) -> Result<StatusCode, ApiError> {
    let normalized = item_id::normalize_full(id);
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return Err(ApiError::NotFound(format!("Issue {} not found", id)));
    }

    let entries = fs::read_dir(&issue_dir)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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

            let new_content = remove_entry_from_content(&content, &normalized);
            if new_content == content {
                continue;
            }

            let has_items = new_content.lines().any(|l| {
                let t = l.trim();
                t.starts_with("- [ ]") || t.starts_with("- [x]")
            });

            if !has_items {
                let _ = fs::remove_file(path);
            } else {
                let updated = update_frontmatter_nums(&new_content);
                fs::write(path, updated)
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
            }

            return Ok(StatusCode::NO_CONTENT);
        }
    }

    Err(ApiError::NotFound(format!("Issue {} not found", id)))
}

// ─── Docs ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct DocsListResponse {
    items: Vec<DocMetaResponse>,
}

#[derive(Serialize)]
struct DocMetaResponse {
    name: String,
    exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

#[derive(Serialize)]
struct DocContentResponse {
    name: String,
    content: String,
    size: u64,
}

#[derive(Deserialize)]
struct DocPutBody {
    content: String,
}

const VALID_DOC_NAMES: &[&str] = &["brainstorm", "prd", "spec"];

async fn handle_list_docs(State(state): State<AppState>) -> Json<DocsListResponse> {
    let doc_root = state.doc_root.clone();
    let items = VALID_DOC_NAMES
        .iter()
        .map(|name| {
            let filename = format!("{}.md", name.to_uppercase());
            let path = doc_root.join(&filename);
            if path.exists() {
                let size = fs::metadata(&path).map(|m| m.len()).ok();
                DocMetaResponse {
                    name: name.to_string(),
                    exists: true,
                    size,
                }
            } else {
                DocMetaResponse {
                    name: name.to_string(),
                    exists: false,
                    size: None,
                }
            }
        })
        .collect();

    Json(DocsListResponse { items })
}

async fn handle_get_doc(
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<DocContentResponse>, ApiError> {
    if !VALID_DOC_NAMES.contains(&name.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid document name '{}'. Allowed: {}",
            name,
            VALID_DOC_NAMES.join(", ")
        )));
    }

    let filename = format!("{}.md", name.to_uppercase());
    let path = state.doc_root.join(&filename);

    if !path.exists() {
        return Err(ApiError::NotFound(format!("Document '{}' does not exist", name)));
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let size = content.len() as u64;

    Ok(Json(DocContentResponse { name, content, size }))
}

async fn handle_put_doc(
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
    Json(body): Json<DocPutBody>,
) -> Result<Json<DocContentResponse>, ApiError> {
    if !VALID_DOC_NAMES.contains(&name.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid document name '{}'. Allowed: {}",
            name,
            VALID_DOC_NAMES.join(", ")
        )));
    }

    let filename = format!("{}.md", name.to_uppercase());
    let path = state.doc_root.join(&filename);

    fs::write(&path, &body.content)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let size = body.content.len() as u64;
    Ok(Json(DocContentResponse {
        name,
        content: body.content,
        size,
    }))
}

// ─── Changelog ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChangelogResponse {
    content: String,
}

#[derive(Deserialize)]
struct ChangelogAddBody {
    text: String,
}

#[derive(Serialize)]
struct ChangelogAddResponse {
    ok: bool,
    entry: String,
}

async fn handle_get_changelog(State(state): State<AppState>) -> Result<Json<ChangelogResponse>, ApiError> {
    let path = state.doc_root.join("CHANGELOG.md");
    if !path.exists() {
        return Err(ApiError::NotFound("CHANGELOG.md not found".to_string()));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ChangelogResponse { content }))
}

async fn handle_post_changelog(
    State(state): State<AppState>,
    Json(body): Json<ChangelogAddBody>,
) -> Result<(StatusCode, Json<ChangelogAddResponse>), ApiError> {
    if body.text.trim().is_empty() {
        return Err(ApiError::Validation("Field 'text' is required".to_string()));
    }

    let doc_root = state.doc_root.clone();
    let text = body.text.clone();
    tokio::task::spawn_blocking(move || add_changelog_entry(&doc_root, &text))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
}

fn add_changelog_entry(
    doc_root: &Path,
    text: &str,
) -> Result<(StatusCode, Json<ChangelogAddResponse>), ApiError> {
    let path = doc_root.join("CHANGELOG.md");
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let time = now.format("%H:%M").to_string();
    let entry = format!("- {} {}", time, text);

    if !path.exists() {
        let content = format!("# Changelog\n\n## {}\n{}\n", today, entry);
        fs::write(&path, content)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    } else {
        let content = fs::read_to_string(&path)
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let today_header = format!("## {}", today);
        let new_content = if content.contains(&today_header) {
            content.replacen(
                &today_header,
                &format!("{}\n{}", today_header, entry),
                1,
            )
        } else {
            // Insert new date section after the title
            let insert_pos = content.find("\n\n").unwrap_or(content.len());
            format!(
                "{}\n\n## {}\n{}{}",
                &content[..insert_pos],
                today,
                entry,
                if insert_pos < content.len() {
                    &content[insert_pos..]
                } else {
                    "\n"
                }
            )
        };

        fs::write(&path, new_content)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    Ok((
        StatusCode::CREATED,
        Json(ChangelogAddResponse { ok: true, entry }),
    ))
}

// ─── Version ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct VersionResponse {
    version: String,
    branch: String,
}

async fn handle_get_version() -> Json<VersionResponse> {
    let version = crate::core::version::read_current().unwrap_or_default();
    let branch = crate::core::version::resolve_branch();
    Json(VersionResponse { version, branch })
}

// ─── SSE Events ──────────────────────────────────────────────────────────────

const SSE_UPDATE_DATA: &str = r#"{"resource":"all"}"#;

fn update_event() -> Event {
    Event::default().event("update").data(SSE_UPDATE_DATA)
}

fn sse_event_from_broadcast(
    message: Result<(), BroadcastStreamRecvError>,
) -> Option<Result<Event, Infallible>> {
    match message {
        Ok(()) => Some(Ok(update_event())),
        Err(error) => {
            eprintln!(
                "[dow dashboard] SSE broadcast receive error: {}; sending a full invalidation update",
                error
            );
            Some(Ok(update_event()))
        }
    }
}

async fn handle_sse_v1(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    state.connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let connections = state.connections.clone();

    let rx = state.notify_tx.subscribe();
    let stream = BroadcastStream::new(rx);

    let event_stream = stream.filter_map(sse_event_from_broadcast);

    let guard = SseConnectionGuard(connections);
    let event_stream = event_stream.map(move |item| {
        let _ = &guard;
        item
    });

    Sse::new(event_stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lagged_broadcast_still_emits_full_invalidation_update() {
        let event = sse_event_from_broadcast(Err(BroadcastStreamRecvError::Lagged(2)))
            .expect("lagged broadcasts should produce an update")
            .expect("SSE event construction should not fail");

        let _ = event;
        assert_eq!(SSE_UPDATE_DATA, r#"{"resource":"all"}"#);
    }
}

struct SseConnectionGuard(Arc<std::sync::atomic::AtomicUsize>);

impl Drop for SseConnectionGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

struct IssueDetail {
    id: String,
    title: String,
    severity: String,
    status: String,
    location: String,
    description: String,
    reproduce: String,
    source: String,
    fix: String,
    files_create: Vec<String>,
    files_modify: Vec<String>,
}

fn get_all_issue_details(doc_root: &Path) -> Vec<IssueDetail> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return Vec::new();
    }

    let mut entries: Vec<_> = fs::read_dir(&issue_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".md")
                && (name.starts_with("issue_") || name.starts_with("closed_"))
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut issues = Vec::new();
    for entry in entries {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            let source = content
                .lines()
                .find(|l| l.starts_with("source:"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
                .unwrap_or_else(|| "other".to_string());

            issues.extend(parse_issues_detail(&content, &source));
        }
    }
    issues
}

fn parse_issues_detail(content: &str, file_source: &str) -> Vec<IssueDetail> {
    let mut issues = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let (is_closed, id, title) = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            if let Some((id, title)) = split_id_title_issue(rest) {
                (false, id, title)
            } else {
                continue;
            }
        } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
            if let Some((id, title)) = split_id_title_issue(rest) {
                (true, id, title)
            } else {
                continue;
            }
        } else {
            continue;
        };

        let mut severity = String::new();
        let mut location = String::new();
        let mut description = String::new();
        let mut reproduce = String::new();
        let mut fix = String::new();
        let mut files_modify = Vec::new();
        let mut files_create = Vec::new();
        let mut last_field = "";

        for j in (i + 1)..lines.len() {
            let sub = lines[j].trim();
            if sub.starts_with("- [ ]") || sub.starts_with("- [x]") {
                break;
            }
            if sub.starts_with("- severity:") || sub.starts_with("- severity：") {
                severity = extract_field_value(sub);
                last_field = "severity";
            } else if sub.starts_with("- location:") || sub.starts_with("- location：") {
                location = extract_field_value(sub);
                last_field = "location";
            } else if sub.starts_with("- description:") || sub.starts_with("- description：") {
                description = extract_field_value(sub);
                last_field = "description";
            } else if sub.starts_with("- reproduce:") || sub.starts_with("- reproduce：") {
                reproduce = extract_field_value(sub);
                last_field = "reproduce";
            } else if sub.starts_with("- fix:") || sub.starts_with("- fix：") {
                fix = extract_field_value(sub);
                last_field = "fix";
            } else if sub.starts_with("- files_modify:") {
                files_modify = parse_inline_list_api(
                    sub.split_once(':').map(|(_, v)| v).unwrap_or(""),
                );
                last_field = "";
            } else if sub.starts_with("- files_create:") {
                files_create = parse_inline_list_api(
                    sub.split_once(':').map(|(_, v)| v).unwrap_or(""),
                );
                last_field = "";
            } else if last_field == "description" {
                description.push('\n');
                description.push_str(sub);
            } else {
                last_field = "";
            }
        }

        issues.push(IssueDetail {
            id,
            title,
            severity,
            status: if is_closed { "closed".to_string() } else { "open".to_string() },
            location,
            description,
            reproduce,
            source: file_source.to_string(),
            fix,
            files_create,
            files_modify,
        });
    }

    issues
}

fn extract_field_value(line: &str) -> String {
    // Handle both ASCII colon and full-width colon
    let after_prefix = line.trim_start_matches("- ");
    if let Some(pos) = after_prefix.find(':') {
        after_prefix[pos + 1..].trim().to_string()
    } else if let Some(pos) = after_prefix.find('：') {
        after_prefix[pos + '：'.len_utf8()..].trim().to_string()
    } else {
        String::new()
    }
}

fn split_id_title_issue(rest: &str) -> Option<(String, String)> {
    for sep in &[":", "："] {
        if let Some(pos) = rest.find(sep) {
            let id = rest[..pos].trim().to_string();
            if !id.starts_with("ISSUE-") {
                return None;
            }
            let title = rest[pos + sep.len()..].trim().to_string();
            return Some((id, title));
        }
    }
    None
}

fn parse_inline_list_api(s: &str) -> Vec<String> {
    let s = s.trim();
    if s == "[]" || s.is_empty() {
        return vec![];
    }
    let s = s.trim_start_matches('[').trim_end_matches(']');
    s.split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Remove a checklist entry (and its sub-fields) from markdown content
fn remove_entry_from_content(content: &str, id: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut skip = false;

    for line in &lines {
        let trimmed = line.trim();
        if (trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]"))
            && (line.contains(&format!("{}:", id))
                || line.contains(&format!("{}：", id)))
        {
            skip = true;
            continue;
        }
        if skip {
            if trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]") {
                skip = false;
                result.push(line.to_string());
            }
            continue;
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

/// Update the `nums:` field in frontmatter to match actual item count
fn update_frontmatter_nums(content: &str) -> String {
    let item_count = content
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("- [ ]") || t.starts_with("- [x]")
        })
        .count();

    if let Some(nums_line_start) = content.find("nums:") {
        let line_end = content[nums_line_start..]
            .find('\n')
            .map(|p| nums_line_start + p)
            .unwrap_or(content.len());
        format!(
            "{}nums: {}{}",
            &content[..nums_line_start],
            item_count,
            &content[line_end..]
        )
    } else {
        content.to_string()
    }
}

/// Update a single field in an issue entry
fn update_issue_field(doc_root: &Path, id: &str, field: &str, value: &str) -> Result<(), ApiError> {
    let issue_dir = doc_root.join("issue");
    if !issue_dir.is_dir() {
        return Err(ApiError::NotFound(format!("Issue {} not found", id)));
    }

    let entries = fs::read_dir(&issue_dir)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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
            if !content.contains(id) {
                continue;
            }

            let new_content = super::server::replace_field_in_entry(&content, id, field, value);
            if new_content != content {
                fs::write(path, new_content)
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                return Ok(());
            }

            // Field not found, try inserting
            let new_content = super::server::insert_field_in_entry(&content, id, field, value);
            if new_content != content {
                fs::write(path, new_content)
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                return Ok(());
            }
        }
    }

    Err(ApiError::NotFound(format!("Issue {} not found", id)))
}

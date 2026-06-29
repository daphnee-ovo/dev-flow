use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path as AxumPath, State};
use axum::http::{header, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use rust_embed::Embed;
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
        .route("/", get(handle_index))
        .route("/assets/{*path}", get(handle_asset))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        DowError::new(format!("Failed to bind port {}: {}", port, e), 1)
    })?;

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

    let event_stream = stream.filter_map(move |msg| {
        match msg {
            Ok(_) => {
                let project_data = data::collect_project_data(&doc_root);
                let json = serde_json::to_string(&project_data).unwrap_or_default();
                Some(Ok::<_, Infallible>(Event::default().event("update").data(json)))
            }
            Err(_) => None,
        }
    });

    let guard = ConnectionGuard(connections);
    let event_stream = event_stream.map(move |item| {
        let _ = &guard;
        item
    });

    Sse::new(event_stream).keep_alive(
        axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)),
    )
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
        ).into_response(),
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
            ).into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

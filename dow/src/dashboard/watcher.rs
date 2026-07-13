use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tokio::sync::broadcast;

pub fn spawn_watcher(
    doc_root: &Path,
    tx: Arc<broadcast::Sender<()>>,
) -> notify_debouncer_mini::Debouncer<notify::RecommendedWatcher> {
    let doc_root = doc_root.to_path_buf();

    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        move |events: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
        if let Ok(events) = events {
            let has_relevant = events.iter().any(|e| e.kind == DebouncedEventKind::Any);
            if has_relevant {
                let _ = tx.send(());
            }
        }
        },
    )
    .expect("failed to create file watcher");

    debouncer
        .watcher()
        .watch(&doc_root, RecursiveMode::Recursive)
        .expect("failed to watch .dev-doc/");

    debouncer
}

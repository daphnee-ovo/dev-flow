// FrameworkTree
// watcher.rs
// ├── spawn_watcher()
// ├── broadcast_update()
// ├── mod tests
// ├── rewritten_status_event_from_debouncer_triggers_update()
// └── continuous_debounced_events_trigger_an_update()

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEvent};
use tokio::sync::broadcast;

pub fn spawn_watcher(
    doc_root: &Path,
    tx: Arc<broadcast::Sender<()>>,
) -> notify::Result<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>> {
    let doc_root = doc_root.to_path_buf();
    let log_root = doc_root.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        move |result: DebounceEventResult| match result {
            Ok(events) => broadcast_update(&log_root, &events, tx.as_ref()),
            Err(error) => eprintln!(
                "[dow dashboard] File watcher error for {}: {}",
                log_root.display(),
                error
            ),
        },
    )?;

    debouncer
        .watcher()
        .watch(&doc_root, RecursiveMode::Recursive)?;

    Ok(debouncer)
}

fn broadcast_update(
    doc_root: &Path,
    events: &[DebouncedEvent],
    tx: &broadcast::Sender<()>,
) {
    if events.is_empty() {
        return;
    }

    let receiver_count = tx.receiver_count();
    let event_summary = events
        .iter()
        .map(|event| format!("{:?}:{}", event.kind, event.path.display()))
        .collect::<Vec<_>>()
        .join(", ");

    eprintln!(
        "[dow dashboard] Detected file update for {} ({} SSE receiver(s); events: {})",
        doc_root.display(),
        receiver_count,
        event_summary
    );

    if let Err(error) = tx.send(()) {
        eprintln!(
            "[dow dashboard] Failed to broadcast file update for {} ({} SSE receiver(s); events: {}): {}",
            doc_root.display(),
            receiver_count,
            event_summary,
            error
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify_debouncer_mini::DebouncedEventKind;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn rewritten_status_event_from_debouncer_triggers_update() {
        let temp = tempdir().unwrap();
        let status = temp.path().join("STATUS.yaml");
        fs::write(&status, "name: test\n").unwrap();
        fs::write(&status, "name: changed\n").unwrap();

        let (tx, mut rx) = broadcast::channel(16);
        let event = DebouncedEvent::new(status, DebouncedEventKind::AnyContinuous);
        broadcast_update(temp.path(), &[event], &tx);

        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn continuous_debounced_events_trigger_an_update() {
        let (tx, mut rx) = broadcast::channel(16);
        let event = DebouncedEvent::new(
            Path::new("STATUS.yaml").to_path_buf(),
            DebouncedEventKind::AnyContinuous,
        );

        broadcast_update(Path::new(".dev-doc"), &[event], &tx);

        assert!(rx.try_recv().is_ok());
    }
}
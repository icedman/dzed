use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

use syntect::highlighting::Theme;
use text::BufferSnapshot;

use crate::display::wrap_map::{WrapMap, WrapSnapshot};
use crate::highlight::{Highlights, StyleCache};
use crate::treesitter::grammars::Grammar;
use crate::treesitter::{SyntaxTree, TreeSitterParser};

/// A unique task ID used to track task sequence and avoid applying stale/obsolete updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

/// Background tasks that can be performed out-of-band to prevent UI blocking.
pub enum BackgroundTask {
    /// Incremental or full-file syntax highlighting task.
    Highlight {
        owner_id: usize,
        file_path: String,
        snapshot: BufferSnapshot,
        start_row: u32,
        row_count: u32,
        theme: Arc<Theme>,
        task_id: TaskId,
        latest_task_id: Arc<AtomicU64>,
    },
    /// Soft-wrap mapping recalculation task.
    Wrap {
        owner_id: usize,
        file_path: String,
        snapshot: BufferSnapshot,
        wrap_width: Option<u32>,
        task_id: TaskId,
        latest_task_id: Arc<AtomicU64>,
    },
    /// Full Tree-sitter parse of an immutable buffer snapshot.
    Parse {
        owner_id: usize,
        file_path: String,
        snapshot: BufferSnapshot,
        grammar: Grammar,
        task_id: TaskId,
        latest_task_id: Arc<AtomicU64>,
    },
}

/// The output results returned by the background thread worker.
pub enum BackgroundResult {
    /// Syntax highlighting calculations completed successfully.
    HighlightComplete {
        owner_id: usize,
        file_path: String,
        style_cache: HashMap<u32, StyleCache>,
        task_id: TaskId,
    },
    /// Wrapping layout calculations completed successfully.
    WrapComplete {
        owner_id: usize,
        file_path: String,
        wrap_snapshot: WrapSnapshot,
        task_id: TaskId,
    },
    /// Tree-sitter parse completed successfully.
    ParseComplete {
        owner_id: usize,
        file_path: String,
        syntax_tree: SyntaxTree,
        task_id: TaskId,
    },
}

/// A background thread worker that coordinates asynchronous work pipelines.
pub struct BackgroundWorker {
    task_tx: mpsc::Sender<BackgroundTask>,
    result_rx: mpsc::Receiver<BackgroundResult>,
}

impl BackgroundWorker {
    /// Creates and boots up a new background thread worker.
    pub fn new() -> Self {
        let (task_tx, task_rx) = mpsc::channel::<BackgroundTask>();
        let (result_tx, result_rx) = mpsc::channel::<BackgroundResult>();

        // Spawn a dedicated worker thread
        let worker_tx = result_tx.clone();
        thread::spawn(move || {
            while let Ok(task) = task_rx.recv() {
                match task {
                    BackgroundTask::Highlight {
                        owner_id,
                        file_path,
                        snapshot,
                        start_row,
                        row_count,
                        theme,
                        task_id,
                        latest_task_id,
                    } => {
                        // Cooperative cancellation check: abort if a newer edit task was already spawned
                        if latest_task_id.load(Ordering::SeqCst) > task_id.0 {
                            continue;
                        }

                        // Instantiate a separate Highlights worker for processing on this thread
                        let mut hl = Highlights::new(&file_path);

                        // Highlight the requested block of lines synchronously inside this thread
                        hl.highlight_lines(&snapshot, start_row, row_count, &theme);

                        // Extract computed style cache
                        let style_cache = hl.get_style_cache().clone();

                        // Final cancellation check before committing channel payload
                        if latest_task_id.load(Ordering::SeqCst) > task_id.0 {
                            continue;
                        }

                        let _ = worker_tx.send(BackgroundResult::HighlightComplete {
                            owner_id,
                            file_path,
                            style_cache,
                            task_id,
                        });
                    }
                    BackgroundTask::Wrap {
                        owner_id,
                        file_path,
                        snapshot,
                        wrap_width,
                        task_id,
                        latest_task_id,
                    } => {
                        // Cooperative cancellation check
                        if latest_task_id.load(Ordering::SeqCst) > task_id.0 {
                            continue;
                        }

                        // Compute wrap coordinates under a temporary WrapMap
                        let wrap_map = WrapMap::new(snapshot, wrap_width);
                        let wrap_snapshot = wrap_map.snapshot();

                        // Final cancellation check
                        if latest_task_id.load(Ordering::SeqCst) > task_id.0 {
                            continue;
                        }

                        let _ = worker_tx.send(BackgroundResult::WrapComplete {
                            owner_id,
                            file_path,
                            wrap_snapshot,
                            task_id,
                        });
                    }
                    BackgroundTask::Parse {
                        owner_id,
                        file_path,
                        snapshot,
                        grammar,
                        task_id,
                        latest_task_id,
                    } => {
                        if latest_task_id.load(Ordering::SeqCst) > task_id.0 {
                            continue;
                        }

                        let Ok(mut parser) = TreeSitterParser::new(grammar) else {
                            continue;
                        };
                        let Ok(syntax_tree) = parser.parse(&snapshot, None) else {
                            continue;
                        };

                        if latest_task_id.load(Ordering::SeqCst) > task_id.0 {
                            continue;
                        }

                        let _ = worker_tx.send(BackgroundResult::ParseComplete {
                            owner_id,
                            file_path,
                            syntax_tree,
                            task_id,
                        });
                    }
                }
            }
        });

        Self { task_tx, result_rx }
    }

    /// Dispatches a background task.
    pub fn spawn_task(&self, task: BackgroundTask) {
        let _ = self.task_tx.send(task);
    }

    /// Non-blockingly polls for completed background results.
    pub fn try_recv(&self) -> Option<BackgroundResult> {
        self.result_rx.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clock::ReplicaId;
    use std::time::{Duration, Instant};
    use text::{Buffer, BufferId};

    #[test]
    fn parses_buffer_snapshots_on_the_background_worker() {
        let worker = BackgroundWorker::new();
        let buffer = Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), "fn main() {}");
        let latest_task_id = Arc::new(AtomicU64::new(1));

        worker.spawn_task(BackgroundTask::Parse {
            owner_id: 42,
            file_path: "main.rs".into(),
            snapshot: buffer.snapshot().clone(),
            grammar: Grammar::Rust,
            task_id: TaskId(1),
            latest_task_id,
        });

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(BackgroundResult::ParseComplete {
                owner_id,
                file_path,
                syntax_tree,
                task_id,
            }) = worker.try_recv()
            {
                assert_eq!(owner_id, 42);
                assert_eq!(file_path, "main.rs");
                assert_eq!(task_id, TaskId(1));
                assert_eq!(syntax_tree.root_kind(), "source_file");
                break;
            }

            assert!(Instant::now() < deadline, "background parse timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

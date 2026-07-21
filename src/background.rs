use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

use syntect::highlighting::Theme;
use text::BufferSnapshot;

use crate::display::wrap_map::{WrapMap, WrapSnapshot};
use crate::highlight::{Highlights, StateCache, StyleCache};

/// A unique task ID used to track task sequence and avoid applying stale/obsolete updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

/// Background tasks that can be performed out-of-band to prevent UI blocking.
pub enum BackgroundTask {
    /// Incremental or full-file syntax highlighting task.
    Highlight {
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
        file_path: String,
        snapshot: BufferSnapshot,
        wrap_width: Option<u32>,
        task_id: TaskId,
        latest_task_id: Arc<AtomicU64>,
    },
}

/// The output results returned by the background thread worker.
pub enum BackgroundResult {
    /// Syntax highlighting calculations completed successfully.
    HighlightComplete {
        file_path: String,
        style_cache: HashMap<u32, StyleCache>,
        task_id: TaskId,
    },
    /// Wrapping layout calculations completed successfully.
    WrapComplete {
        file_path: String,
        wrap_snapshot: WrapSnapshot,
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
                            file_path,
                            style_cache,
                            task_id,
                        });
                    }
                    BackgroundTask::Wrap {
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
                            file_path,
                            wrap_snapshot,
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

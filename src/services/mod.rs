pub mod background;
pub mod clipboard;
pub mod search;
pub mod treesitter;

pub struct Services {
    pub background_worker: background::BackgroundWorker,
    pub clipboard: std::cell::RefCell<crate::services::clipboard::Clipboard>,
    pub search: search::Search,
}

impl Services {
    pub fn new() -> Self {
        Self {
            background_worker: background::BackgroundWorker::new(),
            clipboard: std::cell::RefCell::new(clipboard::Clipboard::new()),
            search: search::Search::new(),
        }
    }
}

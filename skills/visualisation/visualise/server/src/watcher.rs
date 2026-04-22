use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::clusters::LifecycleCluster;
use crate::indexer::Indexer;
use crate::sse_hub::SseHub;

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub debounce: Duration,
}

impl Settings {
    pub const DEFAULT: Settings = Settings {
        debounce: Duration::from_millis(100),
    };
}

pub fn spawn(
    _dirs: Vec<PathBuf>,
    _project_root: PathBuf,
    _indexer: Arc<Indexer>,
    _clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    _hub: Arc<SseHub>,
    _settings: Settings,
) -> JoinHandle<()> {
    todo!("watcher::spawn will be implemented in Step 3")
}

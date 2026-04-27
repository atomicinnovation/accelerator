use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const DEFAULT_TTL: Duration = Duration::from_secs(5);
const DEFAULT_MAX_ENTRIES: usize = 256;

pub struct WriteCoordinator {
    recent: Mutex<HashMap<PathBuf, Instant>>,
    ttl: Duration,
    max_entries: usize,
    now: Box<dyn Fn() -> Instant + Send + Sync>,
}

impl WriteCoordinator {
    pub fn new() -> Self {
        Self {
            recent: Mutex::new(HashMap::new()),
            ttl: DEFAULT_TTL,
            max_entries: DEFAULT_MAX_ENTRIES,
            now: Box::new(Instant::now),
        }
    }

    pub fn with_clock(ttl: Duration, now: Box<dyn Fn() -> Instant + Send + Sync>) -> Self {
        Self {
            recent: Mutex::new(HashMap::new()),
            ttl,
            max_entries: DEFAULT_MAX_ENTRIES,
            now,
        }
    }

    pub fn mark_self_write(&self, canonical: &Path) {
        let mut map = self.recent.lock().unwrap();
        let now = (self.now)();
        if map.len() >= self.max_entries {
            // Evict the oldest entry (FIFO cap)
            if let Some(oldest) = map.iter().min_by_key(|(_, t)| *t).map(|(k, _)| k.clone()) {
                map.remove(&oldest);
            }
        }
        map.insert(canonical.to_path_buf(), now);
    }

    pub fn should_suppress(&self, canonical: &Path) -> bool {
        let mut map = self.recent.lock().unwrap();
        let now = (self.now)();
        // Lazy TTL pruning
        map.retain(|_, t| now.duration_since(*t) < self.ttl);
        // Non-consuming: suppress every echo inside the TTL window
        map.contains_key(canonical)
    }

    pub fn unmark(&self, canonical: &Path) {
        self.recent.lock().unwrap().remove(canonical);
    }
}

impl Default for WriteCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn spy_clock(shared: Arc<Mutex<Instant>>) -> Box<dyn Fn() -> Instant + Send + Sync> {
        Box::new(move || *shared.lock().unwrap())
    }

    #[test]
    fn mark_then_suppress_returns_true() {
        let path = PathBuf::from("/tmp/foo.md");
        let wc = WriteCoordinator::new();
        wc.mark_self_write(&path);
        assert!(wc.should_suppress(&path));
    }

    #[test]
    fn suppress_without_mark_returns_false() {
        let path = PathBuf::from("/tmp/foo.md");
        let wc = WriteCoordinator::new();
        assert!(!wc.should_suppress(&path));
    }

    #[test]
    fn suppress_after_ttl_returns_false() {
        let t = Arc::new(Mutex::new(Instant::now()));
        let wc = WriteCoordinator::with_clock(Duration::from_secs(1), spy_clock(t.clone()));
        let path = PathBuf::from("/tmp/foo.md");
        wc.mark_self_write(&path);
        *t.lock().unwrap() += Duration::from_secs(2);
        assert!(!wc.should_suppress(&path));
    }

    #[test]
    fn suppress_is_non_consuming_within_ttl() {
        let wc = WriteCoordinator::new();
        let path = PathBuf::from("/tmp/foo.md");
        wc.mark_self_write(&path);
        assert!(wc.should_suppress(&path));
        // Second call in same TTL window still suppresses
        assert!(wc.should_suppress(&path));
    }

    #[test]
    fn unmark_prevents_suppress() {
        let wc = WriteCoordinator::new();
        let path = PathBuf::from("/tmp/foo.md");
        wc.mark_self_write(&path);
        wc.unmark(&path);
        assert!(!wc.should_suppress(&path));
    }

    #[test]
    fn fifo_cap_evicts_oldest() {
        let t = Arc::new(Mutex::new(Instant::now()));
        let mut wc = WriteCoordinator::with_clock(Duration::from_secs(60), spy_clock(t.clone()));
        wc.max_entries = 2;

        let p1 = PathBuf::from("/tmp/a.md");
        let p2 = PathBuf::from("/tmp/b.md");
        let p3 = PathBuf::from("/tmp/c.md");

        wc.mark_self_write(&p1);
        *t.lock().unwrap() += Duration::from_millis(1);
        wc.mark_self_write(&p2);
        *t.lock().unwrap() += Duration::from_millis(1);
        // Inserting p3 should evict p1 (oldest)
        wc.mark_self_write(&p3);

        assert!(!wc.should_suppress(&p1), "p1 should have been evicted");
        assert!(wc.should_suppress(&p2));
        assert!(wc.should_suppress(&p3));
    }
}

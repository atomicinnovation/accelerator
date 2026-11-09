//! A `SingleFlightLocks` decorator over the real locks that records every
//! call and tracks which paths are held until each guard drops, so a test can
//! see a lock call made on a path this process already holds.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use accelerator::launch::core::tree::TreeError;
use accelerator::launch::outbound::resolve::single_flight::{
    LockGuard, LockProbe, SingleFlightLocks,
};
use accelerator::launch::outbound::resolve::tree::locks::FlockLocks;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockCall {
    Take,
    TryTake,
    WaitExisting,
    Probe,
}

#[derive(Default)]
struct State {
    calls: Vec<(LockCall, PathBuf)>,
    held: BTreeSet<PathBuf>,
    on_held: Vec<(LockCall, PathBuf)>,
}

#[derive(Clone, Default)]
pub struct RecordingLocks {
    state: Arc<Mutex<State>>,
}

impl RecordingLocks {
    pub fn calls(&self) -> Vec<(LockCall, PathBuf)> {
        self.state.lock().expect("state").calls.clone()
    }

    pub fn calls_on(&self, path: &Path) -> Vec<LockCall> {
        self.calls()
            .into_iter()
            .filter(|(_, called)| called == path)
            .map(|(call, _)| call)
            .collect()
    }

    pub fn calls_on_held_locks(&self) -> Vec<(LockCall, PathBuf)> {
        self.state.lock().expect("state").on_held.clone()
    }

    fn record(&self, call: LockCall, path: &Path) {
        let mut state = self.state.lock().expect("state");
        if state.held.contains(path) {
            state.on_held.push((call, path.to_path_buf()));
        }
        state.calls.push((call, path.to_path_buf()));
    }

    fn holding(&self, path: &Path, guard: LockGuard) -> LockGuard {
        self.state
            .lock()
            .expect("state")
            .held
            .insert(path.to_path_buf());
        LockGuard::holding(Releasing {
            path: path.to_path_buf(),
            state: Arc::clone(&self.state),
            _guard: guard,
        })
    }
}

/// Forgets its path as held when dropped, just before the guard it wraps
/// releases the lock.
struct Releasing {
    path: PathBuf,
    state: Arc<Mutex<State>>,
    _guard: LockGuard,
}

impl Drop for Releasing {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.held.remove(&self.path);
        }
    }
}

impl SingleFlightLocks for RecordingLocks {
    fn take(&self, path: &Path) -> Result<LockGuard, TreeError> {
        self.record(LockCall::Take, path);
        FlockLocks.take(path).map(|guard| self.holding(path, guard))
    }

    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError> {
        self.record(LockCall::TryTake, path);
        Ok(FlockLocks
            .try_take(path)?
            .map(|guard| self.holding(path, guard)))
    }

    fn wait_existing(
        &self,
        path: &Path,
    ) -> Result<Option<LockGuard>, TreeError> {
        self.record(LockCall::WaitExisting, path);
        Ok(FlockLocks
            .wait_existing(path)?
            .map(|guard| self.holding(path, guard)))
    }

    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError> {
        self.record(LockCall::Probe, path);
        Ok(match FlockLocks.probe(path)? {
            LockProbe::Taken(guard) => {
                LockProbe::Taken(self.holding(path, guard))
            }
            probed => probed,
        })
    }
}

//! Test doubles the client's own seams need: a fixed config and environment,
//! a provenance answer, and the clock and jitter the retry suites assert as
//! data rather than by wall clock.

#![allow(dead_code, clippy::expect_used)]

pub mod client;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use config::{ConfigError, Key, Level, Resolved, Scalar, Value};
use tracker_support::{
    CommandPolicy, CredentialContext, Environment, Jitter, Provenance, Sleeper,
};

pub struct FixedConfig {
    personal: BTreeMap<String, String>,
    team: BTreeMap<String, String>,
}

impl FixedConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            personal: BTreeMap::new(),
            team: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_personal(mut self, key: &str, value: &str) -> Self {
        self.personal.insert(key.to_owned(), value.to_owned());
        self
    }

    #[must_use]
    pub fn with_team(mut self, key: &str, value: &str) -> Self {
        self.team.insert(key.to_owned(), value.to_owned());
        self
    }
}

impl config::ConfigAccess for FixedConfig {
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError> {
        let name = key.to_string();
        let found = match level {
            Some(Level::Personal) => self.personal.get(&name),
            Some(Level::Team) => self.team.get(&name),
            // Full stack: personal over team.
            None => self.personal.get(&name).or_else(|| self.team.get(&name)),
        };
        Ok(found.map_or(Resolved::Absent, |value| {
            Resolved::Found(Value::Scalar(Scalar::String(value.clone())))
        }))
    }

    fn set(
        &self,
        _key: &Key,
        _value: &str,
        _level: Level,
    ) -> Result<(), ConfigError> {
        unreachable!("the client never writes config")
    }
}

pub struct FixedEnvironment(BTreeMap<String, String>);

impl FixedEnvironment {
    #[must_use]
    pub const fn empty() -> Self {
        Self(BTreeMap::new())
    }

    #[must_use]
    pub fn with(mut self, name: &str, value: &str) -> Self {
        self.0.insert(name.to_owned(), value.to_owned());
        self
    }
}

impl Environment for FixedEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

pub struct FixedProvenance {
    tracked: Vec<PathBuf>,
}

impl FixedProvenance {
    #[must_use]
    pub const fn nothing_tracked() -> Self {
        Self {
            tracked: Vec::new(),
        }
    }

    #[must_use]
    pub fn tracking(path: &Path) -> Self {
        Self {
            tracked: vec![path.to_path_buf()],
        }
    }
}

impl Provenance for FixedProvenance {
    fn is_tracked(&self, path: &Path) -> bool {
        self.tracked.iter().any(|tracked| tracked == path)
    }
}

#[must_use]
pub fn context<'a>(
    environment: &'a dyn Environment,
    config: &'a dyn config::ConfigAccess,
    provenance: &'a dyn Provenance,
    root: &Path,
) -> CredentialContext<'a> {
    CredentialContext {
        environment,
        config,
        provenance,
        personal_config: root.join("config.local.md"),
        insecure_marker: root.join("allow-insecure-local"),
        command: CommandPolicy::rooted_at(root.to_path_buf()),
    }
}

/// Records what it was asked to sleep for, and never waits: the retry
/// assertions read the sequence rather than the wall clock.
#[derive(Clone, Default)]
pub struct RecordingSleeper {
    slept: Rc<RefCell<Vec<Duration>>>,
}

impl RecordingSleeper {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn slept(&self) -> Vec<Duration> {
        self.slept.borrow().clone()
    }
}

impl Sleeper for RecordingSleeper {
    fn sleep(&mut self, duration: Duration) {
        self.slept.borrow_mut().push(duration);
    }
}

/// A jitter that always chooses no offset, so an expected backoff is exactly
/// the exponential term.
pub struct NoJitter;

impl Jitter for NoJitter {
    fn offset(&mut self, _spread: u64) -> i64 {
        0
    }
}

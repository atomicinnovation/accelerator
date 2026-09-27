//! Test doubles the client's own seams need: a fixed config and environment,
//! a tracking answer, and the clock and jitter the retry suites assert as
//! data rather than by wall clock.

#![allow(dead_code, clippy::expect_used)]

pub mod client;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use config::consent::{
    CommandExecution, CommandPolicy, ConfigFileTracking, ProvenanceContext,
    RepositoryRoots, Runner, Tracking,
};
use config::credentials::{CredentialContext, Environment};
use config::{ConfigError, Key, Level, PersonalFile, Resolved, Scalar, Value};
use config_adapters::credentials::{
    BashCommandRunner, SystemEnvironment,
};
use tracker_support::{Jitter, Sleeper};

/// A personal value implies a readable personal file; a config with none has
/// no personal file at all.
pub struct FixedConfig {
    personal: BTreeMap<String, String>,
    team: BTreeMap<String, String>,
    personal_file: PersonalFile,
}

impl FixedConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            personal: BTreeMap::new(),
            team: BTreeMap::new(),
            personal_file: PersonalFile::Absent,
        }
    }

    #[must_use]
    pub fn with_personal(mut self, key: &str, value: &str) -> Self {
        self.personal.insert(key.to_owned(), value.to_owned());
        self.personal_file = PersonalFile::Readable;
        self
    }

    #[must_use]
    pub fn with_ignored_personal_file(mut self, path: &Path) -> Self {
        self.personal.clear();
        self.personal_file = PersonalFile::Ignored {
            path: path.to_path_buf(),
            mode: 0o644,
        };
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

    fn personal_file(&self) -> &PersonalFile {
        &self.personal_file
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

/// What the VCS answers about every file: tracked when named, untracked
/// otherwise, or unknown for all.
pub struct FixedTracking {
    tracked: Vec<PathBuf>,
    answer_for_all: Option<Tracking>,
}

impl FixedTracking {
    #[must_use]
    pub const fn nothing_tracked() -> Self {
        Self {
            tracked: Vec::new(),
            answer_for_all: None,
        }
    }

    #[must_use]
    pub fn tracking(path: &Path) -> Self {
        Self {
            tracked: vec![path.to_path_buf()],
            answer_for_all: None,
        }
    }

    #[must_use]
    pub const fn everything_tracked() -> Self {
        Self {
            tracked: Vec::new(),
            answer_for_all: Some(Tracking::Tracked),
        }
    }

    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            tracked: Vec::new(),
            answer_for_all: Some(Tracking::Unknown),
        }
    }
}

impl ConfigFileTracking for FixedTracking {
    fn tracking(&self, path: &Path) -> Tracking {
        self.answer_for_all.unwrap_or_else(|| {
            if self.tracked.iter().any(|tracked| tracked == path) {
                Tracking::Tracked
            } else {
                Tracking::Untracked
            }
        })
    }
}

#[must_use]
pub fn context<'a>(
    environment: &'a dyn Environment,
    config: &'a dyn config::ConfigAccess,
    tracking: &'a FixedTracking,
    root: &Path,
) -> CredentialContext<'a> {
    CredentialContext {
        provenance: ProvenanceContext {
            config,
            tracking,
            environment,
            personal_config: root.join("config.local.md"),
        },
        execution: CommandExecution {
            runner: runner_rooted_at(root),
            timeout: CommandPolicy::DEFAULT_TIMEOUT,
        },
    }
}

/// A real runner judging `root` as the repository. Leaked, because a context
/// borrows its runner for as long as the test holds it.
#[must_use]
pub fn runner_rooted_at(root: &Path) -> &'static Runner {
    Box::leak(Box::new(Runner::new(Box::new(BashCommandRunner::new(
        RepositoryRoots::complete(vec![root.to_path_buf()]),
        Box::new(SystemEnvironment),
        std::env::temp_dir(),
    )))))
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

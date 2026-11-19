//! The per-run record of every fixture a test built, adopted or copied.

use std::env;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::thread;

use serde::Serialize;

use crate::family::Family;
use crate::family::Fixtures;
use crate::Error;

pub const FIXTURE_LOG_VARIABLE: &str = "ACCELERATOR_FIXTURE_LOG";

#[derive(Debug, Serialize)]
#[serde(
    tag = "event",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
pub enum Event<'a> {
    SharedBuild {
        family: Family,
        tree: &'a Path,
        git: &'a str,
        jj: &'a str,
    },
    Adoption {
        family: Family,
        test: &'a TestName,
        hermetic_root: &'a Path,
    },
    PrivateBuild {
        fixtures: Fixtures,
        test: &'a TestName,
        hermetic_root: &'a Path,
    },
}

#[derive(Debug, Clone, Default)]
pub struct FixtureLog(Option<PathBuf>);

impl FixtureLog {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self(Some(path.into()))
    }

    #[must_use]
    pub const fn disabled() -> Self {
        Self(None)
    }

    #[must_use]
    pub fn from_environment() -> Self {
        Self::from_value(env::var(FIXTURE_LOG_VARIABLE).ok())
    }

    #[must_use]
    pub fn from_value(value: Option<String>) -> Self {
        value
            .filter(|path| !path.is_empty())
            .map_or_else(Self::disabled, Self::at)
    }

    #[must_use]
    pub fn location(&self) -> Option<&Path> {
        self.0.as_deref()
    }

    /// Appends `event` as one line.
    ///
    /// Concurrent tests share the log, so the line goes out in a single
    /// `write_all` on an append-mode descriptor, which POSIX keeps whole.
    ///
    /// # Errors
    ///
    /// When the event cannot be serialised or the log cannot be written.
    pub fn record(&self, event: &Event<'_>) -> Result<(), Error> {
        let Some(path) = &self.0 else {
            return Ok(());
        };
        let mut line = serde_json::to_string(event)
            .map_err(|error| Error::message(error.to_string()))?;
        line.push('\n');
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all(line.as_bytes())?;
        Ok(())
    }
}

/// The test a fixture event belongs to, as nextest names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct TestName(String);

impl TestName {
    #[must_use]
    pub fn current() -> Self {
        Self::from_variables(
            env::var("NEXTEST_BINARY_ID").ok().as_deref(),
            env::var("NEXTEST_TEST_NAME").ok().as_deref(),
        )
    }

    #[must_use]
    pub fn from_variables(binary: Option<&str>, test: Option<&str>) -> Self {
        match (binary, test) {
            (Some(binary), Some(test))
                if !binary.is_empty() && !test.is_empty() =>
            {
                Self(format!("{binary} {test}"))
            }
            _ => Self(thread::current().name().unwrap_or("unnamed").to_owned()),
        }
    }
}

impl Default for TestName {
    fn default() -> Self {
        Self::current()
    }
}

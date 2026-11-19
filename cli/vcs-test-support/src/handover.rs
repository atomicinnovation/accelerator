//! What a run's build step hands each test: the shared trees, where to open
//! workbenches, and where to log.

use std::collections::BTreeMap;
use std::env;
use std::path::Path;
use std::path::PathBuf;

use crate::family::Family;
use crate::fixture_log::FixtureLog;
use crate::fixture_log::TestName;
use crate::fixture_log::FIXTURE_LOG_VARIABLE;

pub const SHARED_WORKBENCHES_VARIABLE: &str = "ACCELERATOR_SHARED_WORKBENCHES";

#[derive(Debug, Clone, Default)]
pub struct Handover {
    pub trees: BTreeMap<Family, PathBuf>,
    pub workbenches: Option<PathBuf>,
    pub log: FixtureLog,
    pub test: TestName,
}

impl Handover {
    #[must_use]
    pub fn from_environment() -> Self {
        Self::from_variables(|name| env::var(name).ok())
    }

    /// Reads each variable through `lookup`, treating an empty value as unset.
    ///
    /// A handed path is canonicalised so every fixture event names one
    /// spelling of it; one that does not exist is kept as given, so the test
    /// that reaches for it fails naming what it was handed.
    pub fn from_variables(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let set = |name: &str| lookup(name).filter(|value| !value.is_empty());
        let handed_path = |name: &str| {
            set(name).map(|value| {
                let path = PathBuf::from(value);
                path.canonicalize().unwrap_or(path)
            })
        };
        Self {
            trees: Family::ALL
                .into_iter()
                .filter_map(|family| {
                    handed_path(family.handover_variable())
                        .map(|tree| (family, tree))
                })
                .collect(),
            workbenches: handed_path(SHARED_WORKBENCHES_VARIABLE),
            log: FixtureLog::from_value(set(FIXTURE_LOG_VARIABLE)),
            test: TestName::from_variables(
                set("NEXTEST_BINARY_ID").as_deref(),
                set("NEXTEST_TEST_NAME").as_deref(),
            ),
        }
    }

    #[must_use]
    pub fn shared_tree(&self, family: Family) -> Option<&Path> {
        self.trees.get(&family).map(PathBuf::as_path)
    }
}

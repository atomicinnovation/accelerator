//! The build step's side of sharing: building each requested family once, in
//! the tree the run names for it, before any test runs.

use std::any::Any;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use crate::family::Family;
use crate::fixture_log::Event;
use crate::fixture_log::FixtureLog;
use crate::fixtures::Matrix;
use crate::hermetic::tool_version;
use crate::stalled_migration::MigrateBinary;
use crate::stalled_migration::StalledMigrations;
use crate::status_log::States;
use crate::Error;

/// The families a run asks the build step for, each with the tree to build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedBuildRequest {
    pub trees: BTreeMap<Family, PathBuf>,
}

/// A family whose shared build did not finish, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyFailure {
    pub family: Family,
    pub cause: String,
}

/// The `git` and `jj` a shared build was made with, as each reports itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolVersions {
    pub git: String,
    pub jj: String,
}

impl ToolVersions {
    /// # Errors
    ///
    /// When either tool cannot be run.
    pub fn detect() -> Result<Self, Error> {
        Ok(Self {
            git: first_line(&tool_version("git")?),
            jj: first_line(&tool_version("jj")?),
        })
    }
}

impl SharedBuildRequest {
    /// Reads `--<family> <tree>` pairs.
    ///
    /// # Errors
    ///
    /// When an argument names no family, a family flag has no tree, or no
    /// family is named at all.
    pub fn parse(
        arguments: impl IntoIterator<Item = String>,
    ) -> Result<Self, Error> {
        let mut trees = BTreeMap::new();
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            let family = Family::ALL
                .into_iter()
                .find(|family| family.flag() == argument)
                .ok_or_else(|| {
                    Error::message(format!(
                        "unrecognised argument {argument}; {}",
                        usage()
                    ))
                })?;
            let tree = arguments.next().ok_or_else(|| {
                Error::message(format!("{argument} needs a tree; {}", usage()))
            })?;
            trees.insert(family, PathBuf::from(tree));
        }
        if trees.is_empty() {
            return Err(Error::message(format!(
                "no family requested; {}",
                usage()
            )));
        }
        Ok(Self { trees })
    }

    /// Builds every requested family concurrently, each in its own tree.
    pub fn build(&self, log: &FixtureLog) -> Vec<FamilyFailure> {
        self.build_with(log, build_for_sharing)
    }

    /// Builds every requested family with `builder`, on a thread each, and
    /// records a shared build for each family that finishes.
    pub fn build_with(
        &self,
        log: &FixtureLog,
        builder: impl Fn(Family, &Path) -> Result<(), Error> + Sync,
    ) -> Vec<FamilyFailure> {
        let versions = match ToolVersions::detect() {
            Ok(versions) => versions,
            Err(error) => return self.every_family_failing_with(&error),
        };
        let builder = &builder;
        let outcomes = thread::scope(|scope| {
            let mut running = Vec::new();
            for (&family, tree) in &self.trees {
                running.push((
                    family,
                    tree,
                    scope.spawn(move || {
                        let started = Instant::now();
                        builder(family, tree)
                            .map(|()| started.elapsed().as_secs_f64())
                    }),
                ));
            }
            running
                .into_iter()
                .map(|(family, tree, build)| (family, tree, build.join()))
                .collect::<Vec<_>>()
        });
        outcomes
            .into_iter()
            .filter_map(|(family, tree, outcome)| {
                let cause = match outcome {
                    Ok(Ok(seconds)) => log
                        .record(&Event::SharedBuild {
                            family,
                            tree,
                            git: &versions.git,
                            jj: &versions.jj,
                            seconds,
                        })
                        .err()?
                        .to_string(),
                    Ok(Err(error)) => error.to_string(),
                    Err(panic) => panic_cause(family, panic.as_ref()),
                };
                Some(FamilyFailure { family, cause })
            })
            .collect()
    }

    fn every_family_failing_with(&self, error: &Error) -> Vec<FamilyFailure> {
        self.trees
            .keys()
            .map(|&family| FamilyFailure {
                family,
                cause: error.to_string(),
            })
            .collect()
    }
}

fn build_for_sharing(family: Family, tree: &Path) -> Result<(), Error> {
    match family {
        Family::Matrix => Matrix::build_for_sharing(tree).map(drop),
        Family::StatusLog => States::build_for_sharing(tree).map(drop),
        Family::StalledMigration => {
            let migrate = MigrateBinary::from_environment()?;
            StalledMigrations::build_for_sharing(tree, &migrate).map(drop)
        }
    }
}

fn usage() -> String {
    let flags: Vec<String> = Family::ALL
        .iter()
        .map(|family| format!("{} <tree>", family.flag()))
        .collect();
    format!("usage: shared-fixtures {}", flags.join(" "))
}

fn panic_cause(family: Family, panic: &(dyn Any + Send)) -> String {
    let message = panic
        .downcast_ref::<&str>()
        .map(ToString::to_string)
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "an unprintable panic".to_owned());
    format!("building {family} panicked: {message}")
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().to_owned()
}

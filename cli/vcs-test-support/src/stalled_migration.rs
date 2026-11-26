//! A jj repository on which migrate's run has stalled, in each colocation and
//! parent shape the guarded-resume tests start from.

pub mod manifest;
pub mod stalling_corpus;

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;

use crate::colocation::JjColocation;
use crate::family::CopyableFamily;
use crate::family::Family;
use crate::family::PrivateBuild;
use crate::family::SharedFamily;
use crate::hermetic::assert_no_repository_ancestor;
use crate::hermetic::Hermetic;
use crate::Error;
use manifest::StalledMigrationManifest;

/// Names the migrate executable a shared build of the set runs.
pub const MIGRATE_VARIABLE: &str = "ACCELERATOR_SHARED_FIXTURES_MIGRATE";

const STALL_MARKER: &str = "MIGRATION STALLED";

const REPRODUCTION: &str =
    "cargo nextest run -p accelerator-migrate --test guarded_resume";

/// The migrate executable whose run each stall stops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateBinary(pub PathBuf);

impl MigrateBinary {
    /// The binary the run's build step hands the stalled-migration builder.
    ///
    /// # Errors
    ///
    /// When the build step named no binary.
    pub fn from_environment() -> Result<Self, Error> {
        env::var_os(MIGRATE_VARIABLE)
            .filter(|value| !value.is_empty())
            .map(|value| Self(PathBuf::from(value)))
            .ok_or_else(|| {
                Error::message(format!(
                    "building {} needs {MIGRATE_VARIABLE}",
                    Family::StalledMigration
                ))
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParentShape {
    SingleParent,
    MergeWithSibling,
}

impl ParentShape {
    pub const ALL: [Self; 2] = [Self::SingleParent, Self::MergeWithSibling];

    const fn name(self) -> &'static str {
        match self {
            Self::SingleParent => "single-parent",
            Self::MergeWithSibling => "merge-with-sibling",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StallKey {
    pub colocation: JjColocation,
    pub shape: ParentShape,
}

impl StallKey {
    pub const ALL: [Self; 4] = [
        Self::of(JjColocation::Pure, ParentShape::SingleParent),
        Self::of(JjColocation::Pure, ParentShape::MergeWithSibling),
        Self::of(JjColocation::Colocated, ParentShape::SingleParent),
        Self::of(JjColocation::Colocated, ParentShape::MergeWithSibling),
    ];

    const fn of(colocation: JjColocation, shape: ParentShape) -> Self {
        Self { colocation, shape }
    }
}

impl fmt::Display for StallKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}-{}",
            self.colocation.name(),
            self.shape.name()
        )
    }
}

impl FromStr for StallKey {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self, Error> {
        Self::ALL
            .into_iter()
            .find(|key| key.to_string() == name)
            .ok_or_else(|| Error::message(format!("no stall is keyed {name}")))
    }
}

/// One stalled run: the repository, the change holding the committed corpus,
/// and the sibling change beside it that adds `meta/sibling.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stall {
    pub root: PathBuf,
    pub base: String,
    pub sibling: String,
}

impl Stall {
    fn build(
        key: StallKey,
        directory: &Path,
        hermetic: &Hermetic,
        migrate: &MigrateBinary,
    ) -> Result<Self, Error> {
        let root = directory.join(key.to_string());
        fs::create_dir_all(&root)?;
        hermetic.jj(&["git", "init", key.colocation.flag()], &root)?;
        stalling_corpus::seed_output_then_ambiguous_reference(&root)?;
        hermetic.jj(&["commit", "-m", "corpus"], &root)?;
        let base = change_id(hermetic, &root, "@-")?;
        hermetic.jj(&["describe", "-m", "sibling"], &root)?;
        fs::write(root.join("meta/sibling.md"), "sibling\n")?;
        let sibling = change_id(hermetic, &root, "@")?;
        match key.shape {
            ParentShape::SingleParent => hermetic.jj(&["new", &base], &root)?,
            ParentShape::MergeWithSibling => {
                hermetic.jj(&["new", &base, &sibling], &root)?
            }
        };
        let output =
            stalling_corpus::migrate_command(&migrate.0, &root, hermetic)
                .output()
                .map_err(|error| {
                    Error::message(format!(
                        "building {}: cannot run migrate at {}: {error}",
                        Family::StalledMigration,
                        migrate.0.display()
                    ))
                })?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.code() != Some(1) || !stderr.contains(STALL_MARKER) {
            let exit = output.status.code().map_or_else(
                || "on a signal".to_owned(),
                |code| code.to_string(),
            );
            return Err(Error::message(format!(
                "building {}: migrate did not stall in {key}; it exited \
                 {exit} saying: {}\n`{REPRODUCTION}` reproduces it",
                Family::StalledMigration,
                stderr.trim()
            )));
        }
        Ok(Self {
            root,
            base,
            sibling,
        })
    }
}

fn change_id(
    hermetic: &Hermetic,
    root: &Path,
    revision: &str,
) -> Result<String, Error> {
    hermetic.jj(
        &["log", "--no-graph", "-r", revision, "-T", "change_id"],
        root,
    )
}

/// The set's stalls beneath one base: the run's shared build, or a private
/// build in a workbench.
#[derive(Debug)]
pub struct StalledMigrations {
    pub base: PathBuf,
    pub stalls: BTreeMap<StallKey, Stall>,
}

impl StalledMigrations {
    /// Builds every stall at `base` for every test of a run to copy, writing
    /// the manifest adoption reads.
    ///
    /// # Errors
    ///
    /// When `base` lies inside a repository, any stall fails to build, or the
    /// manifest cannot be written.
    pub(crate) fn build_for_sharing(
        base: &Path,
        migrate: &MigrateBinary,
    ) -> Result<Self, Error> {
        assert_no_repository_ancestor(base)?;
        fs::create_dir_all(base)?;
        let base = base.canonicalize()?;
        let hermetic = Hermetic::rooted_at(&base)?;
        let built = Self::build_every_stall(&base, &hermetic, migrate)?;
        StalledMigrationManifest {
            stalls: built.stalls.clone(),
        }
        .write(&built.base)?;
        Ok(built)
    }

    /// Builds the stalls in parallel: each is its own repository, and a
    /// shared build runs before any test, while the CPUs are free.
    fn build_every_stall(
        base: &Path,
        hermetic: &Hermetic,
        migrate: &MigrateBinary,
    ) -> Result<Self, Error> {
        let base = base.canonicalize()?;
        let base_ref = &base;
        let outcomes = thread::scope(|scope| {
            let mut building = Vec::new();
            for key in StallKey::ALL {
                building.push((
                    key,
                    scope.spawn(move || {
                        Stall::build(key, base_ref, hermetic, migrate)
                    }),
                ));
            }
            building
                .into_iter()
                .map(|(key, stall)| {
                    let stall = stall.join().unwrap_or_else(|_| {
                        Err(Error::message(format!(
                            "building the stall {key} panicked"
                        )))
                    });
                    (key, stall)
                })
                .collect::<Vec<_>>()
        });
        let mut stalls = BTreeMap::new();
        let mut failures = Vec::new();
        for (key, outcome) in outcomes {
            match outcome {
                Ok(stall) => {
                    stalls.insert(key, stall);
                }
                Err(error) => failures.push(error.to_string()),
            }
        }
        if failures.is_empty() {
            Ok(Self { base, stalls })
        } else {
            Err(Error::message(failures.join("\n")))
        }
    }

    /// # Errors
    ///
    /// When the set holds no stall for `key`.
    pub fn stall(&self, key: StallKey) -> Result<&Stall, Error> {
        self.stalls
            .get(&key)
            .ok_or_else(|| Error::message(format!("no stall keyed {key}")))
    }
}

impl SharedFamily for StalledMigrations {
    const FAMILY: Family = Family::StalledMigration;
    type Inputs = MigrateBinary;

    fn adopt(tree: &Path) -> Result<Self, Error> {
        let base = tree.canonicalize().map_err(|error| {
            Error::message(format!("{}: {error}", tree.display()))
        })?;
        let StalledMigrationManifest { stalls } =
            StalledMigrationManifest::read(&base)?;
        Ok(Self { base, stalls })
    }

    fn build_privately(
        directory: &Path,
        hermetic: &Hermetic,
        migrate: &MigrateBinary,
        _token: PrivateBuild,
    ) -> Result<Self, Error> {
        Self::build_every_stall(directory, hermetic, migrate)
    }
}

impl CopyableFamily for StalledMigrations {
    type Part = StallKey;

    fn entries(key: &StallKey, _tree: &Path) -> Result<Vec<String>, Error> {
        Ok(vec![key.to_string()])
    }

    fn build_part_privately(
        key: StallKey,
        directory: &Path,
        hermetic: &Hermetic,
        migrate: &MigrateBinary,
        _token: PrivateBuild,
    ) -> Result<Self, Error> {
        let base = directory.canonicalize()?;
        let stall = Stall::build(key, &base, hermetic, migrate)?;
        Ok(Self {
            base,
            stalls: BTreeMap::from([(key, stall)]),
        })
    }

    fn rebased(self, from: &Path, onto: &Path, entries: &[String]) -> Self {
        let copied = |relative: &Path| {
            entries.iter().any(|entry| relative.starts_with(entry))
        };
        Self {
            base: onto.to_path_buf(),
            stalls: self
                .stalls
                .into_iter()
                .filter_map(|(key, stall)| {
                    let relative = stall.root.strip_prefix(from).ok()?;
                    copied(relative).then(|| {
                        (
                            key,
                            Stall {
                                root: onto.join(relative),
                                ..stall
                            },
                        )
                    })
                })
                .collect(),
        }
    }
}

//! The directories a command writes under unattended, each checked once,
//! before any tracker call, against where it really resolves and who
//! configured it.

use std::fmt;
use std::io::ErrorKind;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use ::config::consent::Tracking;
use ::config::provenance::PathSource;
use work::retirement::RECOVERY_PARENT;
use work_adapters::sync::apply::FailureClass;
use work_adapters::sync::operation::SyncOperation;
use work_adapters::sync::recovery_label::PARENT as SYNC_RECOVERY_PARENT;
use work_adapters::sync::report::FailureSeverity;
use work_adapters::sync::report::RunFailure;

use crate::report_records::failure_line;
use crate::sync::STATE_DIR;

/// The configured directories a sync run writes under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKey {
    Work,
    Integrations,
}

impl RootKey {
    pub const fn config_key(self) -> &'static str {
        match self {
            Self::Work => "paths.work",
            Self::Integrations => "paths.integrations",
        }
    }

    const fn directory(self) -> &'static str {
        match self {
            Self::Work => "work directory",
            Self::Integrations => "integrations directory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredRoot {
    pub key: RootKey,
    pub path: PathBuf,
    pub source: PathSource,
}

/// A root with `.` and `..` resolved, and the directory every write under
/// it must resolve inside: the repository for a root inside it, the root
/// itself for one the user configured outside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedRoot {
    root: PathBuf,
    bound: PathBuf,
}

impl BoundedRoot {
    /// # Errors
    ///
    /// [`RootRefusal`] when a root inside the repository resolves outside
    /// it, or a root outside it was not set in an untracked
    /// `config.local.md`.
    pub fn for_configured(
        configured: &ConfiguredRoot,
        repository: &Path,
    ) -> Result<Self, RootRefusal> {
        let root = lexically_normalised(&configured.path);
        let key = configured.key;
        if root.starts_with(repository) {
            return if resolves_within(&root, repository) {
                Ok(Self {
                    root,
                    bound: repository.to_path_buf(),
                })
            } else {
                Err(RootRefusal::SymlinkOut { key, path: root })
            };
        }
        match configured.source {
            PathSource::Personal(Tracking::Untracked) => Ok(Self {
                bound: root.clone(),
                root,
            }),
            PathSource::Personal(Tracking::Tracked) => {
                Err(RootRefusal::OutsideFromTrackedPersonal { key })
            }
            PathSource::Personal(Tracking::Unknown) => {
                Err(RootRefusal::OutsideOfUnknownTracking { key })
            }
            PathSource::Team | PathSource::Default => {
                Err(RootRefusal::OutsideFromTeam { key })
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn bound(&self) -> &Path {
        &self.bound
    }

    pub fn child(&self, name: &str) -> Self {
        Self {
            root: self.root.join(name),
            bound: self.bound.clone(),
        }
    }
}

/// Why a command refuses to write under a configured root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootRefusal {
    SymlinkOut {
        key: RootKey,
        path: PathBuf,
    },
    OutsideFromTeam {
        key: RootKey,
    },
    OutsideFromTrackedPersonal {
        key: RootKey,
    },
    OutsideOfUnknownTracking {
        key: RootKey,
    },
    StateRootSymlinkOut,
    RecoveryParentSymlinked {
        parent: &'static str,
        target: PathBuf,
    },
    IntegrationStateSymlinked {
        path: PathBuf,
        target: PathBuf,
    },
    Overlapping,
    ContainsState {
        key: RootKey,
    },
}

impl fmt::Display for RootRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SymlinkOut {
                key: key @ RootKey::Work,
                path,
            } => write!(
                formatter,
                "{} {} is a symlink outside the repository; move it inside \
                 the repository",
                key.directory(),
                path.display()
            ),
            Self::SymlinkOut {
                key: key @ RootKey::Integrations,
                path,
            } => write!(
                formatter,
                "{} {} is a symlink outside the repository; set {} to its \
                 absolute target in .accelerator/config.local.md to sync it",
                key.directory(),
                path.display(),
                key.config_key()
            ),
            Self::OutsideFromTeam { key } => write!(
                formatter,
                "{} points outside the repository from the team config; set \
                 it in .accelerator/config.local.md to sync there (other work \
                 commands still use it; only sync writes there unattended)",
                key.config_key()
            ),
            Self::OutsideFromTrackedPersonal { key } => write!(
                formatter,
                "{} points outside the repository from a config.local.md \
                 tracked by version control; untrack it (git rm --cached \
                 .accelerator/config.local.md, or jj file untrack \
                 .accelerator/config.local.md) to sync there",
                key.config_key()
            ),
            Self::OutsideOfUnknownTracking { key } => write!(
                formatter,
                "could not determine whether config.local.md is tracked; {key} \
                 outside the repository is refused until it can be; check \
                 that git or jj status works in this repository, or set {key} \
                 inside the repository",
                key = key.config_key()
            ),
            Self::StateRootSymlinkOut => write!(
                formatter,
                "{STATE_DIR} is a symlink outside the repository; move it \
                 back inside the repository"
            ),
            Self::RecoveryParentSymlinked { parent, target } => write!(
                formatter,
                "{STATE_DIR}/{parent} is a symlink to {}; move its contents \
                 back into {STATE_DIR}/{parent}/, then remove the link",
                target.display()
            ),
            Self::IntegrationStateSymlinked { path, target } => write!(
                formatter,
                "{} is a symlink to {}; move its contents back into {}/, then \
                 remove the link",
                path.display(),
                target.display(),
                path.display()
            ),
            Self::Overlapping => formatter.write_str(
                "paths.work and paths.integrations overlap; configure them as \
                 separate directories",
            ),
            Self::ContainsState { key } => write!(
                formatter,
                "{} contains {STATE_DIR}; configure it as a directory beside \
                 it",
                key.config_key()
            ),
        }
    }
}

/// The stderr line a refused root prints: a run-level failure, so its
/// subject is `-`.
pub fn configure_failure(refusal: &RootRefusal) -> String {
    failure_line(&refused_root(refusal, FailureSeverity::Error), "failed")
}

/// As [`configure_failure`], for a command that still saves what it can
/// and only cannot send it.
pub fn configure_warning(refusal: &RootRefusal) -> String {
    failure_line(&refused_root(refusal, FailureSeverity::Warning), "failed")
}

fn refused_root(
    refusal: &RootRefusal,
    severity: FailureSeverity,
) -> RunFailure {
    RunFailure {
        subject: "-".to_owned(),
        operation: SyncOperation::Configure,
        cause: refusal.to_string(),
        class: FailureClass::Unconfigured,
        severity,
    }
}

/// The state root, whose recovery copies a retirement or promotion keeps,
/// checked before anything is written under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedStateRoots {
    pub recovery_state: BoundedRoot,
}

impl ValidatedStateRoots {
    /// # Errors
    ///
    /// [`RootRefusal`] when the state root resolves outside the repository
    /// or either recovery parent is a symlink.
    pub fn check(repository: &Path) -> Result<Self, RootRefusal> {
        let state = repository.join(STATE_DIR);
        if !resolves_within(&state, repository) {
            return Err(RootRefusal::StateRootSymlinkOut);
        }
        for parent in [SYNC_RECOVERY_PARENT, RECOVERY_PARENT] {
            let path = state.join(parent);
            if std::fs::symlink_metadata(&path)
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                return Err(RootRefusal::RecoveryParentSymlinked {
                    parent,
                    target: std::fs::read_link(&path).unwrap_or(path),
                });
            }
        }
        Ok(Self {
            recovery_state: BoundedRoot {
                root: state,
                bound: repository.to_path_buf(),
            },
        })
    }
}

/// Every root a sync run writes under, checked together so each store
/// keeps refusing the others' files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRoots {
    pub work_items: BoundedRoot,
    pub integrations: BoundedRoot,
    pub recovery_state: BoundedRoot,
}

impl ValidatedRoots {
    /// # Errors
    ///
    /// [`RootRefusal`] when a root is refused alone, or the work and
    /// integrations roots overlap or contain the state root.
    pub fn check(
        work: &ConfiguredRoot,
        integrations: &ConfiguredRoot,
        repository: &Path,
    ) -> Result<Self, RootRefusal> {
        let ValidatedStateRoots { recovery_state } =
            ValidatedStateRoots::check(repository)?;
        let work_items = BoundedRoot::for_configured(work, repository)?;
        let integrations =
            BoundedRoot::for_configured(integrations, repository)?;
        let comparable = Comparable::probed_from(repository);
        let (work_path, integrations_path, state_path) = (
            comparable.form_of(work_items.root()),
            comparable.form_of(integrations.root()),
            comparable.form_of(recovery_state.root()),
        );
        if work_path.starts_with(&integrations_path)
            || integrations_path.starts_with(&work_path)
        {
            return Err(RootRefusal::Overlapping);
        }
        for (key, root) in [
            (RootKey::Work, &work_path),
            (RootKey::Integrations, &integrations_path),
        ] {
            if state_path.starts_with(root) {
                return Err(RootRefusal::ContainsState { key });
            }
        }
        Ok(Self {
            work_items,
            integrations,
            recovery_state,
        })
    }

    /// # Errors
    ///
    /// [`RootRefusal::IntegrationStateSymlinked`] when the integration's
    /// state directory is a symlink, which its own adapters would follow.
    pub fn integration_state(
        &self,
        integration: &str,
    ) -> Result<BoundedRoot, RootRefusal> {
        let state = self.integrations.child(integration);
        if let Ok(target) = std::fs::read_link(state.root()) {
            return Err(RootRefusal::IntegrationStateSymlinked {
                path: state.root().to_path_buf(),
                target,
            });
        }
        Ok(state)
    }
}

/// How two roots are compared for overlap: resolved through symlinks, and
/// case-folded where the filesystem ignores case.
struct Comparable {
    folds_case: bool,
}

impl Comparable {
    fn probed_from(repository: &Path) -> Self {
        Self {
            folds_case: ignores_case(repository),
        }
    }

    fn form_of(&self, path: &Path) -> PathBuf {
        let resolved = resolved_allowing_missing(path)
            .unwrap_or_else(|| path.to_path_buf());
        if self.folds_case {
            PathBuf::from(resolved.to_string_lossy().to_lowercase())
        } else {
            resolved
        }
    }
}

/// Whether the filesystem holding `directory` finds it under its own name
/// with its letters' case swapped.
fn ignores_case(directory: &Path) -> bool {
    let Ok(canonical) = std::fs::canonicalize(directory) else {
        return false;
    };
    let Some(name) = canonical.file_name().and_then(|name| name.to_str())
    else {
        return false;
    };
    let swapped: String = name
        .chars()
        .map(|character| {
            if character.is_lowercase() {
                character.to_ascii_uppercase()
            } else {
                character.to_ascii_lowercase()
            }
        })
        .collect();
    if swapped == name {
        return false;
    }
    same_file(&canonical, &canonical.with_file_name(swapped))
}

#[cfg(unix)]
fn same_file(left: &Path, right: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(left), std::fs::metadata(right)) {
        (Ok(left), Ok(right)) => {
            left.dev() == right.dev() && left.ino() == right.ino()
        }
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_file(_left: &Path, right: &Path) -> bool {
    right.exists()
}

fn resolves_within(path: &Path, repository: &Path) -> bool {
    match (
        resolved_allowing_missing(path),
        std::fs::canonicalize(repository),
    ) {
        (Some(resolved), Ok(repository)) => resolved.starts_with(repository),
        _ => false,
    }
}

/// `path` through every symlink in its longest existing ancestor, with the
/// rest appended as written; `None` when an ancestor cannot be resolved.
fn resolved_allowing_missing(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        match std::fs::canonicalize(ancestor) {
            Ok(canonical) => {
                return path
                    .strip_prefix(ancestor)
                    .ok()
                    .map(|tail| canonical.join(tail));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(_) => return None,
        }
    }
    None
}

fn lexically_normalised(path: &Path) -> PathBuf {
    let mut normalised = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalised.pop();
            }
            other => normalised.push(other),
        }
    }
    normalised
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use ::config::consent::Tracking;
    use ::config::provenance::PathSource;

    use super::BoundedRoot;
    use super::ConfiguredRoot;
    use super::RootKey;
    use super::RootRefusal;
    use super::ValidatedRoots;
    use super::ValidatedStateRoots;

    const UNTRACKED: PathSource = PathSource::Personal(Tracking::Untracked);

    struct Layout {
        repository: tempfile::TempDir,
        outside: tempfile::TempDir,
    }

    impl Layout {
        fn new() -> Self {
            let repository = tempfile::tempdir().expect("tempdir");
            let outside = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir_all(repository.path().join("meta/work"))
                .expect("mkdir");
            Self {
                repository,
                outside,
            }
        }

        fn repository(&self) -> &Path {
            self.repository.path()
        }

        fn bound(
            &self,
            path: PathBuf,
            source: PathSource,
        ) -> Result<BoundedRoot, RootRefusal> {
            BoundedRoot::for_configured(
                &ConfiguredRoot {
                    key: RootKey::Integrations,
                    path,
                    source,
                },
                self.repository(),
            )
        }
    }

    enum Expected {
        BoundedByTheRepository(PathBuf),
        BoundedByItself(PathBuf),
        Refused(RootRefusal),
    }

    type Shape = (&'static str, PathBuf, PathSource, Expected);

    fn shapes_inside(repository: &Path) -> Vec<Shape> {
        let inside = |relative: &str| repository.join(relative);
        vec![
            (
                "an in-repository root",
                inside(".accelerator/state/integrations"),
                PathSource::Default,
                Expected::BoundedByTheRepository(inside(
                    ".accelerator/state/integrations",
                )),
            ),
            (
                "an in-repository root that is a symlink out",
                inside("linked-out"),
                UNTRACKED,
                Expected::Refused(RootRefusal::SymlinkOut {
                    key: RootKey::Integrations,
                    path: inside("linked-out"),
                }),
            ),
            (
                "an absolute root naming a path inside the repository",
                inside("state"),
                PathSource::Team,
                Expected::BoundedByTheRepository(inside("state")),
            ),
            (
                "a dotdot root normalising back inside",
                inside(".accelerator/../.accelerator/state/x"),
                PathSource::Team,
                Expected::BoundedByTheRepository(inside(
                    ".accelerator/state/x",
                )),
            ),
            (
                "an outside team value replaced by an inside personal one",
                inside("personal-integrations"),
                UNTRACKED,
                Expected::BoundedByTheRepository(inside(
                    "personal-integrations",
                )),
            ),
        ]
    }

    fn shapes_outside(repository: &Path, outside: &Path) -> Vec<Shape> {
        let key = RootKey::Integrations;
        let from = |source: PathSource, expected: Expected| -> Shape {
            ("", outside.to_path_buf(), source, expected)
        };
        let named = |name: &'static str, shape: Shape| -> Shape {
            (name, shape.1, shape.2, shape.3)
        };
        vec![
            named(
                "an absolute root outside the repository",
                from(UNTRACKED, Expected::BoundedByItself(outside.into())),
            ),
            (
                "a dotdot root outside the repository",
                repository.join("../shared-integrations"),
                UNTRACKED,
                Expected::BoundedByItself(
                    repository.parent().unwrap().join("shared-integrations"),
                ),
            ),
            named(
                "an outside root from the team config",
                from(
                    PathSource::Team,
                    Expected::Refused(RootRefusal::OutsideFromTeam { key }),
                ),
            ),
            named(
                "an outside default",
                from(
                    PathSource::Default,
                    Expected::Refused(RootRefusal::OutsideFromTeam { key }),
                ),
            ),
            named(
                "an outside root from a tracked personal config",
                from(
                    PathSource::Personal(Tracking::Tracked),
                    Expected::Refused(
                        RootRefusal::OutsideFromTrackedPersonal { key },
                    ),
                ),
            ),
            named(
                "an outside root from a personal config of unknown tracking",
                from(
                    PathSource::Personal(Tracking::Unknown),
                    Expected::Refused(RootRefusal::OutsideOfUnknownTracking {
                        key,
                    }),
                ),
            ),
            named(
                "the same outside value in both files",
                from(UNTRACKED, Expected::BoundedByItself(outside.into())),
            ),
        ]
    }

    #[test]
    fn bounded_root_for_each_configured_shape() {
        let layout = Layout::new();
        let repository = layout.repository().to_path_buf();
        std::os::unix::fs::symlink(
            layout.outside.path(),
            repository.join("linked-out"),
        )
        .expect("symlink");
        let outside = layout.outside.path().join("integrations");
        let shapes = shapes_inside(&repository)
            .into_iter()
            .chain(shapes_outside(&repository, &outside));
        for (shape, path, source, expected) in shapes {
            let bounded = layout.bound(path, source);
            match expected {
                Expected::BoundedByTheRepository(root) => {
                    let bounded = bounded.expect(shape);
                    assert_eq!(bounded.root(), root, "{shape}");
                    assert_eq!(bounded.bound(), repository, "{shape}");
                }
                Expected::BoundedByItself(root) => {
                    let bounded = bounded.expect(shape);
                    assert_eq!(bounded.root(), root, "{shape}");
                    assert_eq!(bounded.bound(), root, "{shape}");
                }
                Expected::Refused(refusal) => {
                    assert_eq!(bounded, Err(refusal), "{shape}");
                }
            }
        }
    }

    fn configured(key: RootKey, path: PathBuf) -> ConfiguredRoot {
        ConfiguredRoot {
            key,
            path,
            source: UNTRACKED,
        }
    }

    #[test]
    fn a_nested_integrations_root_inside_the_work_root_overlaps() {
        let layout = Layout::new();
        let repository = layout.repository();

        let checked = ValidatedRoots::check(
            &configured(RootKey::Work, repository.join("meta/work")),
            &configured(
                RootKey::Integrations,
                repository.join("meta/work/integrations"),
            ),
            repository,
        );

        assert_eq!(checked, Err(RootRefusal::Overlapping));
    }

    #[test]
    fn a_work_root_containing_the_state_root_is_refused() {
        let layout = Layout::new();
        let repository = layout.repository();

        let checked = ValidatedRoots::check(
            &configured(RootKey::Work, repository.join(".accelerator")),
            &configured(RootKey::Integrations, repository.join("integrations")),
            repository,
        );

        assert_eq!(
            checked,
            Err(RootRefusal::ContainsState { key: RootKey::Work })
        );
    }

    #[test]
    fn roots_aliased_through_a_symlink_overlap() {
        let layout = Layout::new();
        let repository = layout.repository();
        std::os::unix::fs::symlink(
            repository.join("meta/work"),
            repository.join("alias"),
        )
        .expect("symlink");

        let checked = ValidatedRoots::check(
            &configured(RootKey::Work, repository.join("meta/work")),
            &configured(RootKey::Integrations, repository.join("alias/x")),
            repository,
        );

        assert_eq!(checked, Err(RootRefusal::Overlapping));
    }

    #[test]
    fn a_symlinked_state_root_is_refused() {
        let layout = Layout::new();
        std::fs::create_dir_all(layout.repository().join(".accelerator"))
            .expect("mkdir");
        std::os::unix::fs::symlink(
            layout.outside.path(),
            layout.repository().join(".accelerator/state"),
        )
        .expect("symlink");

        assert_eq!(
            ValidatedStateRoots::check(layout.repository()),
            Err(RootRefusal::StateRootSymlinkOut)
        );
    }

    #[test]
    fn a_symlinked_recovery_parent_is_refused_naming_its_target() {
        for parent in ["sync-recovery", "retirement-recovery"] {
            let layout = Layout::new();
            let state = layout.repository().join(".accelerator/state");
            std::fs::create_dir_all(&state).expect("mkdir");
            std::os::unix::fs::symlink(
                layout.outside.path(),
                state.join(parent),
            )
            .expect("symlink");

            assert_eq!(
                ValidatedStateRoots::check(layout.repository()),
                Err(RootRefusal::RecoveryParentSymlinked {
                    parent: if parent == "sync-recovery" {
                        "sync-recovery"
                    } else {
                        "retirement-recovery"
                    },
                    target: layout.outside.path().to_path_buf(),
                }),
                "{parent}"
            );
        }
    }

    #[test]
    fn each_refusal_names_what_to_change() {
        let cases = [
            (
                RootRefusal::SymlinkOut {
                    key: RootKey::Integrations,
                    path: PathBuf::from("/r/state"),
                },
                "integrations directory /r/state is a symlink outside the \
                 repository; set paths.integrations to its absolute target \
                 in .accelerator/config.local.md to sync it",
            ),
            (
                RootRefusal::SymlinkOut {
                    key: RootKey::Work,
                    path: PathBuf::from("/r/meta/work"),
                },
                "work directory /r/meta/work is a symlink outside the \
                 repository; move it inside the repository",
            ),
            (
                RootRefusal::OutsideFromTeam {
                    key: RootKey::Integrations,
                },
                "paths.integrations points outside the repository from the \
                 team config; set it in .accelerator/config.local.md to sync \
                 there (other work commands still use it; only sync writes \
                 there unattended)",
            ),
            (
                RootRefusal::StateRootSymlinkOut,
                ".accelerator/state is a symlink outside the repository; \
                 move it back inside the repository",
            ),
            (
                RootRefusal::RecoveryParentSymlinked {
                    parent: "sync-recovery",
                    target: PathBuf::from("/elsewhere"),
                },
                ".accelerator/state/sync-recovery is a symlink to /elsewhere; \
                 move its contents back into .accelerator/state/sync-recovery/, \
                 then remove the link",
            ),
            (
                RootRefusal::Overlapping,
                "paths.work and paths.integrations overlap; configure them \
                 as separate directories",
            ),
            (
                RootRefusal::ContainsState {
                    key: RootKey::Work,
                },
                "paths.work contains .accelerator/state; configure it as a \
                 directory beside it",
            ),
        ];
        for (refusal, text) in cases {
            assert_eq!(refusal.to_string(), text);
        }
    }
}

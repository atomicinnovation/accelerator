//! Ignores the state one machine's syncs leave behind.
//!
//! Adds to `.accelerator/.gitignore` the recovery copies, pending-push
//! markers, conflict dossiers and adoption records no collaborator should
//! receive, and the same rules to the root `.gitignore` for an integrations
//! root moved elsewhere in the repository. An ignore rule untracks nothing,
//! so any such file already tracked is named with the command that untracks
//! it.

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use crate::migrations::text::ensure_lines;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::ports::TrackedState;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0011;

impl MigrationMeta for Migration0011 {
    fn id(&self) -> &'static str {
        "0011-ignore-machine-local-sync-state"
    }

    fn description(&self) -> &'static str {
        "Ignore machine-local sync state: recovery copies, pending-push \
         markers, conflict dossiers and adoption records."
    }
}

const STATE_RULES: [&str; 5] = [
    "state/retirement-recovery/",
    "state/sync-recovery/",
    "state/integrations/*/pending-push/",
    "state/integrations/*/conflicts/",
    "state/integrations/*/adopted/",
];

const RECOVERY_DIRECTORIES: [&str; 2] =
    ["retirement-recovery", "sync-recovery"];

const INTEGRATION_LOCAL_DIRECTORIES: [&str; 3] =
    ["pending-push", "conflicts", "adopted"];

impl Migration for Migration0011 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let root = ctx.root().to_path_buf();
        ensure_lines(ctx, &root.join(".accelerator/.gitignore"), &STATE_RULES)?;
        let moved = moved_integrations_root(ctx, &root)?;
        if let Some(moved) = &moved {
            let rules = root_rules(moved);
            let rules: Vec<&str> = rules.iter().map(String::as_str).collect();
            ensure_lines(ctx, &root.join(".gitignore"), &rules)?;
        }
        let state = root.join(".accelerator/state");
        let tracked = [
            ctx.version_control(&state)
                .map(|found| within(found, &state, is_machine_local_state)),
            moved.as_ref().and_then(|moved| {
                let integrations = root.join(moved);
                ctx.version_control(&integrations).map(|found| {
                    within(found, &integrations, is_integration_local)
                })
            }),
        ];
        if let Some(note) = tracked_note(&root, tracked.into_iter().flatten()) {
            eprintln!("{note}");
        }
        Ok(ApplyOutcome::Applied)
    }
}

/// `paths.integrations` relative to the root, when it is overridden to a
/// place inside the repository that the state rules do not cover.
fn moved_integrations_root(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<Option<PathBuf>, MigrationError> {
    let Some(configured) =
        ctx.configured_path_override("paths.integrations")?
    else {
        return Ok(None);
    };
    let configured = Path::new(&configured);
    let absolute = if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        root.join(configured)
    };
    Ok(absolute
        .strip_prefix(root)
        .ok()
        .filter(|relative| {
            relative
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        })
        .filter(|relative| !relative.starts_with(".accelerator/state"))
        .map(Path::to_path_buf))
}

fn root_rules(moved: &Path) -> Vec<String> {
    INTEGRATION_LOCAL_DIRECTORIES
        .iter()
        .map(|directory| format!("/{}/*/{directory}/", moved.display()))
        .collect()
}

fn within(
    found: TrackedState,
    under: &Path,
    machine_local: fn(&Path) -> bool,
) -> TrackedState {
    TrackedState {
        tracked: found
            .tracked
            .into_iter()
            .filter(|path| path.strip_prefix(under).is_ok_and(machine_local))
            .collect(),
        ..found
    }
}

fn names(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect()
}

fn is_machine_local_state(relative: &Path) -> bool {
    match names(relative).as_slice() {
        [first, ..] if RECOVERY_DIRECTORIES.contains(&first.as_str()) => true,
        [first, rest @ ..] if first == "integrations" => {
            is_integration_local(&rest.iter().collect::<PathBuf>())
        }
        _ => false,
    }
}

fn is_integration_local(relative: &Path) -> bool {
    matches!(
        names(relative).as_slice(),
        [_, directory, _, ..]
            if INTEGRATION_LOCAL_DIRECTORIES.contains(&directory.as_str())
    )
}

/// The note naming every tracked machine-local file with the command that
/// untracks it; `None` when nothing is tracked.
fn tracked_note(
    root: &Path,
    found: impl Iterator<Item = TrackedState>,
) -> Option<String> {
    let listing: String = found
        .flat_map(|state| {
            let kind = state.kind;
            state.tracked.into_iter().map(move |path| {
                let shown =
                    path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                format!(
                    "\n  {}  ({})",
                    shown.display(),
                    kind.untrack_command(&shown)
                )
            })
        })
        .collect();
    (!listing.is_empty()).then(|| {
        format!(
            "note: 0011: version control tracks these machine-local files, \
             and an ignore rule cannot untrack them:{listing}\n\
             Committing an untrack removes the files from collaborators' \
             working copies, so first settle each tracked pending-push \
             marker with `accelerator work sync --resolve <id>=adopt:<KEY>` \
             or `--resolve <id>=create`, and back up each tracked recovery \
             directory."
        )
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::path::PathBuf;

    use super::tracked_note;
    use super::Migration0011;
    use crate::ports::CorpusIndex;
    use crate::ports::DocTypeDir;
    use crate::ports::MigrationContext;
    use crate::ports::MigrationError;
    use crate::ports::TrackedState;
    use crate::ports::VersionControl;
    use crate::registry::Migration as _;

    type TestResult = Result<(), MigrationError>;

    struct Context {
        root: PathBuf,
        files: RefCell<BTreeMap<PathBuf, String>>,
        writes: RefCell<usize>,
        integrations: Option<String>,
        tracked: Option<TrackedState>,
    }

    impl Context {
        fn new() -> Self {
            Self {
                root: PathBuf::from("/repo"),
                files: RefCell::new(BTreeMap::new()),
                writes: RefCell::new(0),
                integrations: None,
                tracked: None,
            }
        }

        fn holding(self, relative: &str, content: &str) -> Self {
            self.files
                .borrow_mut()
                .insert(self.root.join(relative), content.to_owned());
            self
        }

        fn content(&self, relative: &str) -> String {
            self.files
                .borrow()
                .get(&self.root.join(relative))
                .cloned()
                .unwrap_or_default()
        }
    }

    struct NoIndex;

    impl CorpusIndex for NoIndex {
        fn target_exists(&self, _: &str, _: &str) -> bool {
            false
        }
    }

    impl MigrationContext for Context {
        fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
            Vec::new()
        }

        fn corpus_index(&self) -> &dyn CorpusIndex {
            &NoIndex
        }

        fn write(&self, path: &Path, content: &str) -> TestResult {
            *self.writes.borrow_mut() += 1;
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), content.to_owned());
            Ok(())
        }

        fn root(&self) -> &Path {
            &self.root
        }

        fn configured_path_override(
            &self,
            key: &str,
        ) -> Result<Option<String>, MigrationError> {
            Ok((key == "paths.integrations")
                .then(|| self.integrations.clone())
                .flatten())
        }

        fn version_control(&self, under: &Path) -> Option<TrackedState> {
            self.tracked.clone().map(|state| TrackedState {
                tracked: state
                    .tracked
                    .into_iter()
                    .filter(|path| path.starts_with(under))
                    .collect(),
                ..state
            })
        }

        fn read(&self, path: &Path) -> Result<Option<String>, MigrationError> {
            Ok(self.files.borrow().get(path).cloned())
        }
    }

    const RULES: &str = "state/retirement-recovery/\nstate/sync-recovery/\n\
                         state/integrations/*/pending-push/\n\
                         state/integrations/*/conflicts/\n\
                         state/integrations/*/adopted/\n";

    #[test]
    fn a_fresh_repository_ignores_every_machine_local_directory() -> TestResult
    {
        let ctx = Context::new();

        Migration0011.apply(&ctx)?;

        assert_eq!(ctx.content(".accelerator/.gitignore"), RULES);
        assert_eq!(ctx.content(".gitignore"), "");
        Ok(())
    }

    #[test]
    fn a_repository_already_ignoring_them_is_unchanged() -> TestResult {
        let ctx = Context::new().holding(
            ".accelerator/.gitignore",
            &format!("config.local.md\n{RULES}"),
        );

        Migration0011.apply(&ctx)?;

        assert_eq!(*ctx.writes.borrow(), 0);
        Ok(())
    }

    #[test]
    fn a_moved_integrations_root_is_ignored_from_the_root() -> TestResult {
        let ctx = Context {
            integrations: Some("meta/integrations".to_owned()),
            ..Context::new()
        };

        Migration0011.apply(&ctx)?;

        assert_eq!(
            ctx.content(".gitignore"),
            "/meta/integrations/*/pending-push/\n\
             /meta/integrations/*/conflicts/\n\
             /meta/integrations/*/adopted/\n"
        );
        Ok(())
    }

    #[test]
    fn an_integrations_root_outside_the_repository_is_left_alone() -> TestResult
    {
        let ctx = Context {
            integrations: Some("/elsewhere/integrations".to_owned()),
            ..Context::new()
        };

        Migration0011.apply(&ctx)?;

        assert_eq!(ctx.content(".gitignore"), "");
        Ok(())
    }

    #[test]
    fn a_tracked_marker_is_named_with_its_untrack_command() {
        let note = tracked_note(
            Path::new("/repo"),
            std::iter::once(TrackedState {
                kind: VersionControl::Git,
                tracked: vec![PathBuf::from(
                    "/repo/.accelerator/state/integrations/jira/pending-push/\
                     0001.json",
                )],
            }),
        );

        let note = note.unwrap_or_default();
        assert!(
            note.contains(
                "\n  .accelerator/state/integrations/jira/pending-push/\
                 0001.json  (git rm --cached -r \
                 .accelerator/state/integrations/jira/pending-push/0001.json)"
            ),
            "{note}"
        );
        assert!(note.contains("=adopt:<KEY>"), "{note}");
    }

    #[test]
    fn only_machine_local_files_are_named() -> TestResult {
        let ctx = Context {
            tracked: Some(TrackedState {
                kind: VersionControl::Jj,
                tracked: vec![
                    PathBuf::from(
                        "/repo/.accelerator/state/migrations-applied",
                    ),
                    PathBuf::from(
                        "/repo/.accelerator/state/integrations/jira/\
                         last-sync.json",
                    ),
                ],
            }),
            ..Context::new()
        };

        Migration0011.apply(&ctx)?;

        assert_eq!(
            tracked_note(
                &ctx.root,
                ctx.version_control(&ctx.root.join(".accelerator/state"))
                    .map(|found| super::within(
                        found,
                        &ctx.root.join(".accelerator/state"),
                        super::is_machine_local_state
                    ))
                    .into_iter()
            ),
            None
        );
        Ok(())
    }

    #[test]
    fn a_context_without_version_control_prints_no_untrack_note() {
        assert_eq!(tracked_note(Path::new("/repo"), std::iter::empty()), None);
    }
}

//! `vcs detect`: the checkout's VCS mode and any workspace/worktree
//! boundary, reproducing `hooks/vcs-detect.sh`'s prose.

use std::path::Path;

use vcs::classify::CheckoutProbe;
use vcs::classify::Classification;
use vcs::mode::Mode;
use vcs::mode::ModeProbe;

const JJ_COMMAND_REFERENCE: &str = concat!(
    "VCS Command Reference:\n",
    "- Use `jj status` instead of `git status`\n",
    "- Use `jj diff` instead of `git diff`\n",
    "- Use `jj log` instead of `git log`\n",
    "- Use `jj commit -m \"message\"` instead of `git add + git commit` \
     (there is no staging area; all tracked changes are included)\n",
    "- Use `jj squash` instead of `git commit --amend`\n",
    "- Use `jj describe -m \"message\"` to edit a commit message\n",
    "- Use `jj git push` instead of `git push`\n",
    "- Use `jj bookmark list` instead of `git branch`\n",
    "- Use `jj new` to start a new change after the current one\n",
    "- Use `jj bookmark list` or `jj status` instead of `git branch \
     --show-current`\n",
    "\n",
    "Key conceptual differences from git:\n",
    "- No staging area: all tracked changes are automatically part of the \
     working-copy commit\n",
    "- The working copy is always a commit — there is no \"uncommitted\" \
     state\n",
    "- `jj new` creates a new empty change (like finishing current work \
     and starting fresh)\n",
    "- `jj describe` edits any commit's message without `--amend` \
     semantics\n",
    "\n",
    "IMPORTANT: Do NOT use raw git commands for VCS operations. Always \
     use jj.\n",
    "The `gh` CLI for GitHub operations remains unchanged.",
);

const GIT_REFERENCE: &str = concat!(
    "This repository uses git as its version control system.\n",
    "\n",
    "VCS Command Reference:\n",
    "- Use `git status` to see current changes\n",
    "- Use `git diff` to see modifications (use `--cached` for staged \
     changes)\n",
    "- Use `git log --oneline` to see recent commit history\n",
    "- Use `git add <files>` to stage changes — NEVER use `-A` or `.` \
     (always add specific files by name)\n",
    "- Use `git commit -m \"message\"` to commit staged changes\n",
    "- Use `git branch --show-current` to check the current branch\n",
    "- Use `git push` to push to remote\n",
    "\n",
    "Key conventions:\n",
    "- Always stage specific files by name, never bulk-add\n",
    "- Use `--cached` with `git diff` to see what is staged\n",
    "- Prefer atomic, focused commits over large multi-concern commits",
);

fn reference_text(mode: Mode) -> String {
    match mode {
        Mode::Jj => format!(
            "This repository uses jujutsu (jj) as its version control \
             system (mode: jj).\n\n{JJ_COMMAND_REFERENCE}"
        ),
        Mode::JjColocated => format!(
            "This repository uses jujutsu (jj) as its version control \
             system (mode: jj-colocated).\n\n{JJ_COMMAND_REFERENCE}"
        ),
        Mode::Git => GIT_REFERENCE.to_owned(),
    }
}

/// The boundary block's kind suffix and the boundary/jj-parent/git-parent
/// paths a `Classification` arm carries, or `None` for `Main`/`None`, which
/// carry no boundary at all.
fn boundary_parts(
    classification: &Classification,
) -> Option<(&'static str, &Path, Option<&Path>, Option<&Path>)> {
    match classification {
        Classification::Main | Classification::None => Option::None,
        Classification::JjSecondary {
            boundary,
            jj_parent,
        } => Some(("", boundary, Some(jj_parent.as_path()), Option::None)),
        Classification::GitWorktree {
            boundary,
            git_parent,
        } => Some(("", boundary, Option::None, Some(git_parent.as_path()))),
        Classification::Colocated {
            boundary,
            jj_parent,
            git_parent,
        } => Some((
            " (colocated)",
            boundary,
            Some(jj_parent.as_path()),
            Some(git_parent.as_path()),
        )),
        Classification::NestedJjInGit {
            boundary,
            jj_parent,
            git_parent,
        }
        | Classification::NestedGitInJj {
            boundary,
            jj_parent,
            git_parent,
        } => Some((
            " (nested)",
            boundary,
            Some(jj_parent.as_path()),
            Some(git_parent.as_path()),
        )),
    }
}

fn parent_block(label: &str, parent: &Path) -> String {
    let parent = parent.display();
    format!(
        "Parent repository ({label}): {parent}\n\
         do not edit files in {parent}\n\
         do not run VCS commands against {parent}\n\
         do not grep, find, or research files in {parent}\n"
    )
}

/// The boundary block body, starting directly at `WORKSPACE BOUNDARY
/// DETECTED` with no leading or trailing blank lines — matching a `$(...)`
/// capture of the shell's `build_boundary_block`, minus its leading `\n\n`
/// (which existed only to separate it from the always-on shell `CONTEXT`
/// prefix this port makes conditional).
fn boundary_block(classification: &Classification) -> Option<String> {
    let (kind_suffix, boundary, jj_parent, git_parent) =
        boundary_parts(classification)?;
    let mut body = format!(
        "WORKSPACE BOUNDARY DETECTED{kind_suffix}\n\
         You are inside a checkout that is NOT the main repository.\n\
         Boundary (active workspace): {}\n\n",
        boundary.display()
    );
    if let Some(jj_parent) = jj_parent {
        body.push_str(&parent_block("jj", jj_parent));
        if git_parent.is_some() {
            body.push('\n');
        }
    }
    if let Some(git_parent) = git_parent {
        body.push_str(&parent_block("git", git_parent));
    }
    while body.ends_with('\n') {
        body.pop();
    }
    Some(body)
}

/// The rendered body: `None` when there is nothing to report (a `--descriptive`
/// carries the reference text regardless; a structured, non-`--descriptive`
/// invocation is `None` unless a boundary exists).
fn render(
    mode: Mode,
    classification: &Classification,
    descriptive: bool,
) -> Option<String> {
    let boundary = boundary_block(classification);
    if !descriptive {
        return boundary;
    }
    let mut text = reference_text(mode);
    if let Some(body) = boundary {
        text.push_str("\n\n");
        text.push_str(&body);
        text.push('\n');
    }
    Some(text)
}

/// Detects the checkout containing `start` and renders the `SessionStart`
/// envelope, or `None` for zero bytes of output (a main checkout, no
/// boundary, non-`--descriptive`).
///
/// Two independent sources can fail: the direct mode-determination query and
/// the classifier's own probe calls (for the boundary block). Under
/// `fail_safe`, either failure degrades to a single `adapter_failure`
/// `systemMessage` envelope rather than propagating.
///
/// # Errors
///
/// The probe's `kernel::Error`, when `fail_safe` is `false`.
pub fn run<P>(
    start: &Path,
    probe: &P,
    descriptive: bool,
    fail_safe: bool,
) -> Result<Option<String>, kernel::Error>
where
    P: ModeProbe + CheckoutProbe,
{
    let mode = vcs::mode::determine(start, probe);
    let classification = vcs::classify::classify(start, probe);
    let (mode, classification) = match (mode, classification) {
        (Ok(mode), Ok(classification)) => (mode, classification),
        (Err(error), _) | (_, Err(error)) => {
            return if fail_safe {
                Ok(Some(kernel::hooks::adapter_failure(&error.to_string())))
            } else {
                Err(error)
            };
        }
    };
    Ok(render(mode, &classification, descriptive)
        .map(|text| kernel::hooks::session_start(&text, None)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vcs::checkout::DualRoots;
    use vcs::checkout::JjRepositoryFacts;
    use vcs::checkout::JjWorkspaceRole;
    use vcs::checkout::WorktreeFacts;

    use super::run;

    #[derive(Default, Clone)]
    struct StubProbe {
        is_bare: Option<Result<Option<bool>, String>>,
        worktree: Option<Result<Option<WorktreeFacts>, String>>,
        jj_repository: Option<Result<Option<JjRepositoryFacts>, String>>,
        jj_workspace_root: Option<Result<Option<PathBuf>, String>>,
        dual_git: Option<Result<Option<PathBuf>, String>>,
        dual_jj: Option<Result<Option<PathBuf>, String>>,
        mode_fails: bool,
        classify_fails: bool,
    }

    fn to_kernel<T>(result: Result<T, String>) -> Result<T, kernel::Error> {
        result.map_err(kernel::Error::Failed)
    }

    impl vcs::classify::CheckoutProbe for StubProbe {
        fn is_bare(
            &self,
            _start: &std::path::Path,
        ) -> Result<Option<bool>, kernel::Error> {
            if self.classify_fails {
                return Err(kernel::Error::Failed("classify boom".to_owned()));
            }
            to_kernel(self.is_bare.clone().unwrap_or(Ok(None)))
        }

        fn worktree(
            &self,
            _start: &std::path::Path,
        ) -> Result<Option<WorktreeFacts>, kernel::Error> {
            to_kernel(self.worktree.clone().unwrap_or(Ok(None)))
        }

        fn jj_repository(
            &self,
            _start: &std::path::Path,
        ) -> Result<Option<JjRepositoryFacts>, kernel::Error> {
            to_kernel(self.jj_repository.clone().unwrap_or(Ok(None)))
        }

        fn dual_roots(&self, _start: &std::path::Path) -> DualRoots {
            DualRoots {
                git: to_kernel(self.dual_git.clone().unwrap_or(Ok(None))),
                jj: to_kernel(self.dual_jj.clone().unwrap_or(Ok(None))),
            }
        }
    }

    impl vcs::mode::ModeProbe for StubProbe {
        fn jj_workspace_root(
            &self,
            _start: &std::path::Path,
        ) -> Result<Option<PathBuf>, kernel::Error> {
            if self.mode_fails {
                return Err(kernel::Error::Failed("mode boom".to_owned()));
            }
            to_kernel(self.jj_workspace_root.clone().unwrap_or(Ok(None)))
        }

        fn dual_roots(&self, _start: &std::path::Path) -> DualRoots {
            DualRoots {
                git: to_kernel(self.dual_git.clone().unwrap_or(Ok(None))),
                jj: to_kernel(self.dual_jj.clone().unwrap_or(Ok(None))),
            }
        }
    }

    fn start() -> PathBuf {
        PathBuf::from("/tmp/somewhere")
    }

    fn main_git_probe() -> StubProbe {
        StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Ok(Some(WorktreeFacts {
                linked: false,
                git_dir: PathBuf::from("/tmp/somewhere/.git"),
                common_dir: PathBuf::from("/tmp/somewhere/.git"),
                main_worktree_root: Some(PathBuf::from("/tmp/somewhere")),
            }))),
            jj_repository: Some(Ok(None)),
            jj_workspace_root: Some(Ok(None)),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_jj: Some(Ok(None)),
            ..StubProbe::default()
        }
    }

    fn jj_secondary_probe() -> StubProbe {
        StubProbe {
            is_bare: Some(Ok(None)),
            jj_repository: Some(Ok(Some(JjRepositoryFacts {
                role: JjWorkspaceRole::Secondary,
                main_root: PathBuf::from("/tmp/jj-main"),
            }))),
            jj_workspace_root: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_jj: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            ..StubProbe::default()
        }
    }

    #[test]
    fn a_main_git_checkout_without_descriptive_produces_zero_bytes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = run(&start(), &main_git_probe(), false, false)?;
        assert_eq!(output, None);
        Ok(())
    }

    #[test]
    fn a_main_git_checkout_with_descriptive_carries_the_git_reference(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = run(&start(), &main_git_probe(), true, false)?
            .ok_or("expected output")?;
        assert!(output.contains("uses git as its version control system"));
        assert!(!output.contains("WORKSPACE BOUNDARY DETECTED"));
        Ok(())
    }

    #[test]
    fn a_jj_secondary_workspace_without_descriptive_carries_only_the_boundary(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = run(&start(), &jj_secondary_probe(), false, false)?
            .ok_or("expected output")?;
        assert!(output.contains("WORKSPACE BOUNDARY DETECTED"));
        assert!(!output.contains("VCS Command Reference"));
        Ok(())
    }

    #[test]
    fn a_jj_secondary_workspace_with_descriptive_carries_both(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = run(&start(), &jj_secondary_probe(), true, false)?
            .ok_or("expected output")?;
        assert!(output.contains("mode: jj"));
        assert!(output.contains("VCS Command Reference"));
        assert!(output.contains("WORKSPACE BOUNDARY DETECTED"));
        Ok(())
    }

    #[test]
    fn a_failing_mode_probe_without_fail_safe_propagates() {
        let probe = StubProbe {
            mode_fails: true,
            ..main_git_probe()
        };
        assert!(run(&start(), &probe, false, false).is_err());
    }

    #[test]
    fn a_failing_mode_probe_under_fail_safe_degrades_to_adapter_failure(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let probe = StubProbe {
            mode_fails: true,
            ..main_git_probe()
        };
        let output =
            run(&start(), &probe, true, true)?.ok_or("expected output")?;
        assert!(output.contains("systemMessage"));
        assert!(!output.contains("hookSpecificOutput"));
        Ok(())
    }

    #[test]
    fn a_failing_classify_probe_without_fail_safe_propagates() {
        let probe = StubProbe {
            classify_fails: true,
            ..main_git_probe()
        };
        assert!(run(&start(), &probe, false, false).is_err());
    }

    #[test]
    fn a_failing_classify_probe_under_fail_safe_degrades_to_adapter_failure(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let probe = StubProbe {
            classify_fails: true,
            ..main_git_probe()
        };
        let output =
            run(&start(), &probe, false, true)?.ok_or("expected output")?;
        assert!(output.contains("systemMessage"));
        assert!(!output.contains("hookSpecificOutput"));
        Ok(())
    }
}

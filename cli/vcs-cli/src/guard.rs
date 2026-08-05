//! `vcs guard`: the `PreToolUse` hook blocking (or warning about) git VCS
//! commands with jj equivalents, reproducing `hooks/vcs-guard.sh`.

use std::path::Path;

use vcs::guard::GuardDecision;
use vcs::mode::Mode;
use vcs::mode::ModeProbe;

/// Only ever called with `Jj` or `JjColocated` — `run` returns before
/// evaluating any command in `Git` mode.
fn reason(mode: Mode, subcommand: &str, suggestion: &str) -> String {
    match mode {
        Mode::JjColocated => format!(
            "This is a jj-colocated repository. Prefer jj over git \
             {subcommand}. Suggested equivalent: {suggestion}"
        ),
        Mode::Jj => format!(
            "This is a pure jujutsu repository. Use jj instead of git \
             {subcommand}. Equivalent: {suggestion}"
        ),
        Mode::Git => unreachable!("git mode never reaches guard decision text"),
    }
}

/// Guards a Bash tool call's `command` against the checkout containing
/// `start`, or `None` when there is nothing to say (no jj workspace present,
/// no command to evaluate, or the command is allowed).
///
/// Mode is determined first: no jj workspace at this location allows
/// unconditionally, without evaluating `command` at all, matching the
/// shell's own early exit on `.jj` absence before it ever parses the
/// command. Otherwise (pure-jj or colocated), `vcs::guard::decide` chooses
/// the envelope shape: pure-jj denies, colocated warns.
///
/// # Errors
///
/// The probe's `kernel::Error`, when `fail_safe` is `false`.
pub fn run<P>(
    start: &Path,
    probe: &P,
    command: Option<&str>,
    fail_safe: bool,
) -> Result<Option<String>, kernel::Error>
where
    P: ModeProbe,
{
    let mode = match vcs::mode::determine(start, probe) {
        Ok(mode) => mode,
        Err(error) => {
            return if fail_safe {
                Ok(Some(kernel::hooks::adapter_failure(&error.to_string())))
            } else {
                Err(error)
            };
        }
    };
    if mode == Mode::Git {
        return Ok(None);
    }
    let Some(command) = command else {
        return Ok(None);
    };
    match vcs::guard::decide(command) {
        GuardDecision::Allow => Ok(None),
        GuardDecision::Block {
            subcommand,
            suggestion,
        } => {
            let text = reason(mode, &subcommand, &suggestion);
            Ok(Some(match mode {
                Mode::Jj => kernel::hooks::pre_tool_use_deny(&text),
                Mode::JjColocated => kernel::hooks::pre_tool_use_warn(&text),
                Mode::Git => {
                    unreachable!("git mode never reaches guard rendering")
                }
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vcs::checkout::DualRoots;

    use super::run;

    #[derive(Default, Clone)]
    struct StubProbe {
        jj_workspace_root: Option<Result<Option<PathBuf>, String>>,
        dual_git: Option<Result<Option<PathBuf>, String>>,
        mode_fails: bool,
    }

    fn to_kernel<T>(result: Result<T, String>) -> Result<T, kernel::Error> {
        result.map_err(kernel::Error::Failed)
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
                jj: Ok(None),
            }
        }
    }

    fn start() -> PathBuf {
        PathBuf::from("/tmp/somewhere")
    }

    fn pure_jj_probe() -> StubProbe {
        StubProbe {
            jj_workspace_root: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_git: Some(Ok(None)),
            mode_fails: false,
        }
    }

    fn colocated_probe() -> StubProbe {
        StubProbe {
            jj_workspace_root: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            mode_fails: false,
        }
    }

    fn git_probe() -> StubProbe {
        StubProbe {
            jj_workspace_root: Some(Ok(None)),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            mode_fails: false,
        }
    }

    #[test]
    fn a_git_only_checkout_allows_without_evaluating_the_command(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = run(&start(), &git_probe(), Some("git status"), false)?;
        assert_eq!(output, None);
        Ok(())
    }

    #[test]
    fn no_command_allows() -> Result<(), Box<dyn std::error::Error>> {
        let output = run(&start(), &pure_jj_probe(), None, false)?;
        assert_eq!(output, None);
        Ok(())
    }

    #[test]
    fn an_allowed_command_in_a_pure_jj_checkout_allows(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = run(&start(), &pure_jj_probe(), Some("git push"), false)?;
        assert_eq!(output, None);
        Ok(())
    }

    #[test]
    fn a_blocked_command_in_a_pure_jj_checkout_denies(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output =
            run(&start(), &pure_jj_probe(), Some("git status"), false)?
                .ok_or("expected output")?;
        assert!(output.contains("\"permissionDecision\":\"deny\""));
        assert!(output.contains("pure jujutsu repository"));
        Ok(())
    }

    #[test]
    fn a_blocked_command_in_a_colocated_checkout_warns(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output =
            run(&start(), &colocated_probe(), Some("git status"), false)?
                .ok_or("expected output")?;
        assert!(!output.contains("permissionDecision"));
        assert!(output.contains("jj-colocated repository"));
        Ok(())
    }

    #[test]
    fn a_failing_mode_probe_without_fail_safe_propagates() {
        let probe = StubProbe {
            mode_fails: true,
            ..pure_jj_probe()
        };
        assert!(run(&start(), &probe, Some("git status"), false).is_err());
    }

    #[test]
    fn a_failing_mode_probe_under_fail_safe_degrades_to_adapter_failure(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let probe = StubProbe {
            mode_fails: true,
            ..pure_jj_probe()
        };
        let output = run(&start(), &probe, Some("git status"), true)?
            .ok_or("expected output")?;
        assert!(output.contains("systemMessage"));
        assert!(!output.contains("permissionDecision"));
        Ok(())
    }
}

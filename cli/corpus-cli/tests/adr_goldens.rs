//! `accelerator-corpus adr` against pre-captured expected values,
//! transliterated from `skills/decisions/scripts/test-adr-scripts.sh`.
//!
//! Unconditional: unlike this codebase's other binary-spawning golden suites
//! (which compare against a live bash process under the `bash-parity`
//! feature), these compare against values captured once, during development,
//! from the bash scripts this phase deletes — there is no bash left to
//! compare against once it's gone, so this suite runs in the default `cargo
//! test` invocation with no feature gate.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-corpus");

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("corpus-adr-golden-{tag}-"))
        .tempdir()?)
}

fn repo(dir: &Path) -> Result<(), TestError> {
    fs::create_dir_all(dir.join(".git"))?;
    Ok(())
}

fn run(dir: &Path, args: &[&str]) -> Result<Output, TestError> {
    Ok(Command::new(BIN).current_dir(dir).args(args).output()?)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

mod next_number {
    use super::{repo, run, stderr, stdout, tempdir, TestError};
    use std::fs;

    #[test]
    fn no_decisions_directory_outputs_0001() -> Result<(), TestError> {
        let dir = tempdir("no-dir")?;
        repo(dir.path())?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert!(output.status.success());
        assert_eq!(stdout(&output), "0001\n");
        Ok(())
    }

    #[test]
    fn an_empty_decisions_directory_outputs_0001() -> Result<(), TestError> {
        let dir = tempdir("empty-dir")?;
        repo(dir.path())?;
        fs::create_dir_all(dir.path().join("meta/decisions"))?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert!(output.status.success());
        assert_eq!(stdout(&output), "0001\n");
        Ok(())
    }

    #[test]
    fn a_single_existing_adr_increments() -> Result<(), TestError> {
        let dir = tempdir("single")?;
        repo(dir.path())?;
        let decisions = dir.path().join("meta/decisions");
        fs::create_dir_all(&decisions)?;
        fs::write(decisions.join("ADR-0003-foo.md"), "")?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert_eq!(stdout(&output), "0004\n");
        Ok(())
    }

    #[test]
    fn gaps_use_the_highest_existing_number() -> Result<(), TestError> {
        let dir = tempdir("gaps")?;
        repo(dir.path())?;
        let decisions = dir.path().join("meta/decisions");
        fs::create_dir_all(&decisions)?;
        fs::write(decisions.join("ADR-0001-first.md"), "")?;
        fs::write(decisions.join("ADR-0005-fifth.md"), "")?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert_eq!(stdout(&output), "0006\n");
        Ok(())
    }

    #[test]
    fn non_adr_files_are_ignored() -> Result<(), TestError> {
        let dir = tempdir("non-adr")?;
        repo(dir.path())?;
        let decisions = dir.path().join("meta/decisions");
        fs::create_dir_all(&decisions)?;
        fs::write(decisions.join("README.md"), "")?;
        fs::write(decisions.join("DRAFT-notes.md"), "")?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert_eq!(stdout(&output), "0001\n");
        Ok(())
    }

    #[test]
    fn mixed_adr_and_non_adr_files() -> Result<(), TestError> {
        let dir = tempdir("mixed")?;
        repo(dir.path())?;
        let decisions = dir.path().join("meta/decisions");
        fs::create_dir_all(&decisions)?;
        fs::write(decisions.join("ADR-0002-something.md"), "")?;
        fs::write(decisions.join("README.md"), "")?;
        fs::write(decisions.join("DRAFT-notes.md"), "")?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert_eq!(stdout(&output), "0003\n");
        Ok(())
    }

    #[test]
    fn count_three_from_the_highest() -> Result<(), TestError> {
        let dir = tempdir("count")?;
        repo(dir.path())?;
        let decisions = dir.path().join("meta/decisions");
        fs::create_dir_all(&decisions)?;
        fs::write(decisions.join("ADR-0002-something.md"), "")?;
        let output = run(dir.path(), &["adr", "next-number", "--count", "3"])?;
        assert_eq!(stdout(&output), "0003\n0004\n0005\n");
        Ok(())
    }

    #[test]
    fn count_zero_is_rejected() -> Result<(), TestError> {
        let dir = tempdir("count-zero")?;
        repo(dir.path())?;
        let output = run(dir.path(), &["adr", "next-number", "--count", "0"])?;
        assert_eq!(output.status.code(), Some(1));
        Ok(())
    }

    #[test]
    fn count_abc_is_rejected() -> Result<(), TestError> {
        let dir = tempdir("count-abc")?;
        repo(dir.path())?;
        let output =
            run(dir.path(), &["adr", "next-number", "--count", "abc"])?;
        assert_eq!(output.status.code(), Some(1));
        Ok(())
    }

    #[test]
    fn adr_9999_overflows_untruncated() -> Result<(), TestError> {
        let dir = tempdir("overflow")?;
        repo(dir.path())?;
        let decisions = dir.path().join("meta/decisions");
        fs::create_dir_all(&decisions)?;
        fs::write(decisions.join("ADR-9999-overflow.md"), "")?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert_eq!(stdout(&output), "10000\n");
        Ok(())
    }

    #[test]
    fn a_configured_decisions_path_is_honoured() -> Result<(), TestError> {
        let dir = tempdir("configured")?;
        repo(dir.path())?;
        fs::create_dir_all(dir.path().join(".accelerator"))?;
        fs::write(
            dir.path().join(".accelerator/config.md"),
            "---\npaths:\n  decisions: adr\n---\n",
        )?;
        let decisions = dir.path().join("adr");
        fs::create_dir_all(&decisions)?;
        fs::write(decisions.join("ADR-0001-first.md"), "")?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert!(output.status.success());
        assert_eq!(stdout(&output), "0002\n");
        Ok(())
    }

    #[test]
    fn a_legacy_config_layout_fails_closed() -> Result<(), TestError> {
        let dir = tempdir("legacy")?;
        repo(dir.path())?;
        fs::create_dir_all(dir.path().join(".claude"))?;
        fs::write(dir.path().join(".claude/accelerator.md"), "legacy")?;
        let output = run(dir.path(), &["adr", "next-number"])?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("legacy"));
        Ok(())
    }

    #[test]
    fn a_legacy_config_layout_under_fail_safe_degrades_to_exit_0(
    ) -> Result<(), TestError> {
        let dir = tempdir("legacy-fail-safe")?;
        repo(dir.path())?;
        fs::create_dir_all(dir.path().join(".claude"))?;
        fs::write(dir.path().join(".claude/accelerator.md"), "legacy")?;
        let output = run(dir.path(), &["adr", "next-number", "--fail-safe"])?;
        assert!(output.status.success());
        assert_eq!(stdout(&output), "");
        assert!(stderr(&output).contains("legacy"));
        Ok(())
    }
}

mod read_status {
    use super::{run, stderr, stdout, tempdir, TestError};
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    fn write_adr(dir: &Path, body: &str) -> Result<PathBuf, TestError> {
        let path = dir.join("ADR-0001-test.md");
        fs::write(&path, body)?;
        Ok(path)
    }

    fn path_arg(path: &Path) -> String {
        path.display().to_string()
    }

    #[test]
    fn a_proposed_status_is_read() -> Result<(), TestError> {
        let dir = tempdir("proposed")?;
        let path = write_adr(
            dir.path(),
            "---\nadr_id: ADR-0001\nstatus: proposed\n---\n\n# ADR-0001: \
             Test\n",
        )?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert!(output.status.success());
        assert_eq!(stdout(&output), "proposed\n");
        Ok(())
    }

    #[test]
    fn an_accepted_status_is_read() -> Result<(), TestError> {
        let dir = tempdir("accepted")?;
        let path = write_adr(
            dir.path(),
            "---\nadr_id: ADR-0001\nstatus: accepted\n---\n\n# ADR-0001: \
             Test\n",
        )?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert_eq!(stdout(&output), "accepted\n");
        Ok(())
    }

    #[test]
    fn a_quoted_value_is_unwrapped() -> Result<(), TestError> {
        let dir = tempdir("quoted")?;
        let path = write_adr(
            dir.path(),
            "---\nadr_id: ADR-0001\nstatus: \"proposed\"\n---\n\n# Test\n",
        )?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert_eq!(stdout(&output), "proposed\n");
        Ok(())
    }

    #[test]
    fn no_space_after_the_colon_is_read() -> Result<(), TestError> {
        let dir = tempdir("no-space")?;
        let path = write_adr(
            dir.path(),
            "---\nadr_id: ADR-0001\nstatus:proposed\n---\n\n# Test\n",
        )?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert_eq!(stdout(&output), "proposed\n");
        Ok(())
    }

    #[test]
    fn trailing_whitespace_is_stripped() -> Result<(), TestError> {
        let dir = tempdir("trailing-ws")?;
        let path = write_adr(
            dir.path(),
            "---\nadr_id: ADR-0001\nstatus: proposed  \n---\n\n# Test\n",
        )?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert_eq!(stdout(&output), "proposed\n");
        Ok(())
    }

    #[test]
    fn a_missing_file_exits_1() -> Result<(), TestError> {
        let dir = tempdir("missing")?;
        let output =
            run(dir.path(), &["adr", "read-status", "/nonexistent/file.md"])?;
        assert_eq!(output.status.code(), Some(1));
        Ok(())
    }

    #[test]
    fn a_file_with_no_frontmatter_exits_1() -> Result<(), TestError> {
        let dir = tempdir("no-frontmatter")?;
        let path = dir.path().join("no-frontmatter.md");
        fs::write(&path, "# Just a regular file\n\nNo frontmatter here.\n")?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert_eq!(output.status.code(), Some(1));
        Ok(())
    }

    #[test]
    fn an_unclosed_fence_exits_1() -> Result<(), TestError> {
        let dir = tempdir("unclosed")?;
        let path = dir.path().join("unclosed.md");
        fs::write(&path, "---\nadr_id: ADR-0001\nstatus: proposed\n")?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert_eq!(output.status.code(), Some(1));
        Ok(())
    }

    #[test]
    fn a_status_like_line_in_the_body_is_ignored() -> Result<(), TestError> {
        let dir = tempdir("body-status")?;
        let path = write_adr(
            dir.path(),
            "---\nadr_id: ADR-0001\nstatus: proposed\n---\n\n# Test\n\n\
             status: accepted\n",
        )?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert_eq!(stdout(&output), "proposed\n");
        Ok(())
    }

    #[test]
    fn an_empty_status_value_succeeds() -> Result<(), TestError> {
        let dir = tempdir("empty-status")?;
        let path = write_adr(
            dir.path(),
            "---\nadr_id: ADR-0001\nstatus: \n---\n\n# Test\n",
        )?;
        let output =
            run(dir.path(), &["adr", "read-status", &path_arg(&path)])?;
        assert!(output.status.success());
        assert_eq!(stdout(&output), "\n");
        Ok(())
    }

    #[test]
    fn no_arguments_exits_1() -> Result<(), TestError> {
        let dir = tempdir("no-args")?;
        let output = run(dir.path(), &["adr", "read-status"])?;
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr(&output).contains("Usage"));
        Ok(())
    }
}

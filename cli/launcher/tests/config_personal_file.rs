//! An insecure `config.local.md` through `accelerator config`: every read
//! runs on team values with one warning, the `SessionStart` summary carries
//! the warning in both hook fields, and every writer refuses.
#![cfg(unix)]

use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const WARNING: &str = "warning: E_LOCAL_PERMS_INSECURE";

struct Fixture {
    root: PathBuf,
    _guard: TempDir,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let guard = tempfile::Builder::new()
            .prefix("config-personal-file-")
            .tempdir()?;
        let root = guard.path().to_path_buf();
        fs::create_dir_all(root.join(".git"))?;
        fs::create_dir_all(root.join(".accelerator"))?;
        fs::write(
            root.join(".accelerator/config.md"),
            "---\npaths:\n  work: team-work\nagents:\n  reviewer: team-reviewer\n---\nTeam context.\n",
        )?;
        let personal = root.join(".accelerator/config.local.md");
        fs::write(
            &personal,
            "---\npaths:\n  work: personal-work\n---\nPersonal context.\n",
        )?;
        fs::set_permissions(&personal, fs::Permissions::from_mode(0o644))?;
        Ok(Self {
            root,
            _guard: guard,
        })
    }

    fn personal_bytes(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(fs::read(self.root.join(".accelerator/config.local.md"))?)
    }

    fn run(&self, args: &[&str]) -> Result<Output, Box<dyn Error>> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator"));
        command
            .current_dir(&self.root)
            .env_remove("ACCELERATOR_LOG")
            .env_remove("ACCELERATOR_CACHE_DIR")
            .env_remove("ACCELERATOR_RELEASE_BASE_URL")
            .env("ACCELERATOR_PLUGIN_ROOT", plugin_root())
            .env(
                "ACCELERATOR_VCS_BIN",
                env!("CARGO_BIN_EXE_accelerator-fixture"),
            )
            .args(args);
        Ok(command.output()?)
    }
}

fn plugin_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plugin")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn reads_run_on_team_values_and_warn_once() -> TestResult {
    let fixture = Fixture::new()?;
    for (args, expected) in [
        (&["config", "get", "paths.work"][..], "team-work"),
        (&["config", "dump"][..], "team-work"),
        (&["config", "agents"][..], "team-reviewer"),
        (&["config", "context"][..], "Team context."),
    ] {
        let output = fixture.run(args)?;

        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(
            stdout(&output).contains(expected),
            "{args:?}: {}",
            stdout(&output)
        );
        assert!(!stdout(&output).contains("personal-work"), "{args:?}");
        assert!(!stdout(&output).contains("Personal context."), "{args:?}");
        assert_eq!(
            stderr(&output).matches(WARNING).count(),
            1,
            "{args:?}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn the_session_start_summary_warns_once_in_both_hook_fields() -> TestResult {
    let fixture = Fixture::new()?;

    let output = fixture.run(&["config", "summary", "--format", "hook"])?;

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let envelope = stdout(&output);
    let (context, system_message) =
        envelope
            .split_once("\"systemMessage\":")
            .ok_or_else(|| format!("no systemMessage in {envelope}"))?;
    assert!(context.contains("E_LOCAL_PERMS_INSECURE"), "{envelope}");
    assert!(
        system_message.contains("E_LOCAL_PERMS_INSECURE"),
        "{envelope}"
    );
    assert!(system_message.contains("[accelerator]"), "{envelope}");
    assert_eq!(envelope.lines().count(), 1, "{envelope}");
    assert!(
        !stderr(&output).contains("E_LOCAL_PERMS_INSECURE"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn the_plain_summary_warns_once_on_stderr() -> TestResult {
    let fixture = Fixture::new()?;

    let output = fixture.run(&["config", "summary"])?;

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stderr(&output).matches(WARNING).count(),
        1,
        "{}",
        stderr(&output)
    );
    assert!(!stdout(&output).contains("E_LOCAL_PERMS_INSECURE"));
    Ok(())
}

#[test]
fn a_personal_write_is_refused_and_leaves_the_file_untouched() -> TestResult {
    let fixture = Fixture::new()?;
    let before = fixture.personal_bytes()?;

    let output = fixture.run(&["config", "set", "paths.work", "x"])?;

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("E_LOCAL_PERMS_INSECURE"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fixture.personal_bytes()?, before);
    Ok(())
}

#[test]
fn template_writers_refuse_and_write_nothing() -> TestResult {
    let fixture = Fixture::new()?;
    let overrides = fixture.root.join(".accelerator/templates");
    fs::create_dir_all(&overrides)?;
    fs::write(overrides.join("demo.md"), "# Mine\n")?;

    for args in [
        &["config", "templates", "eject", "demo", "--force"][..],
        &["config", "templates", "reset", "demo", "--confirm"][..],
    ] {
        let output = fixture.run(args)?;

        assert_eq!(
            output.status.code(),
            Some(1),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("E_LOCAL_PERMS_INSECURE"),
            "{args:?}: {}",
            stderr(&output)
        );
        assert_eq!(fs::read_to_string(overrides.join("demo.md"))?, "# Mine\n");
    }
    Ok(())
}

//! How the executor reports the `design.browser_path` hatch, through the
//! compiled binary. With no launcher reachable the executor downgrades after
//! resolving the hatch, so nothing here needs a Playwright runtime.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-design");
const OVERRIDE: &str = "ACCELERATOR_DESIGN_BROWSER_PATH";

struct Scratch {
    work: tempfile::TempDir,
    repo: PathBuf,
}

impl Scratch {
    fn new() -> Result<Self, TestError> {
        let work = tempfile::Builder::new()
            .prefix("design-hatch-bin-")
            .tempdir()?;
        let repo = work.path().join("repo");
        fs::create_dir_all(repo.join(".accelerator"))?;
        Hermetic::rooted_at(work.path())?.git(&["init", "--quiet"], &repo)?;
        fs::create_dir_all(work.path().join("empty-bin"))?;
        Ok(Self { work, repo })
    }

    fn outside_browser(&self) -> Result<PathBuf, TestError> {
        let browser = self.work.path().join("outside/chrome");
        fs::create_dir_all(self.work.path().join("outside"))?;
        fs::write(&browser, "")?;
        Ok(browser)
    }

    fn personal(&self, value: &Path, mode: u32) -> Result<(), TestError> {
        let path = self.repo.join(".accelerator/config.local.md");
        fs::write(
            &path,
            format!(
                "---\ndesign:\n  browser_path: \"{}\"\n---\n",
                value.display()
            ),
        )?;
        fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
        Ok(())
    }

    fn track_personal(&self) -> Result<(), TestError> {
        Hermetic::rooted_at(self.work.path())?.git(
            &["add", "--force", ".accelerator/config.local.md"],
            &self.repo,
        )?;
        Ok(())
    }

    fn empty_plugin_root(&self) -> Result<PathBuf, TestError> {
        let root = self.work.path().join("plugin-root");
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn browser_slots(&self) -> Result<Vec<String>, TestError> {
        let state = self
            .repo
            .join(".accelerator/tmp/inventory-design-playwright");
        let mut slots = fs::read_dir(state)?
            .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        slots.sort();
        Ok(slots)
    }

    fn stderr(&self, environment: &[(&str, &Path)]) -> String {
        let mut command = Command::new(BIN);
        command.args(["executor", "ping"]);
        command.current_dir(&self.repo);
        command.env_clear();
        command.env("PATH", self.work.path().join("empty-bin"));
        command.env("HOME", self.work.path());
        if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        for (name, value) in environment {
            command.env(name, value);
        }
        let output = command
            .output()
            .unwrap_or_else(|error| unreachable!("the binary runs: {error}"));
        String::from_utf8_lossy(&output.stderr).into_owned()
    }
}

#[test]
fn an_environment_browser_prints_a_notice_naming_the_variable_and_path(
) -> Result<(), TestError> {
    let scratch = Scratch::new()?;
    let browser = scratch.outside_browser()?;

    let stderr = scratch.stderr(&[(OVERRIDE, &browser)]);

    assert!(
        stderr.lines().any(|line| line
            == format!(
                "notice: design.browser_path taken from {OVERRIDE}: {}",
                browser.display()
            )),
        "{stderr}"
    );
    assert!(!stderr.contains("warning:"), "{stderr}");
    Ok(())
}

#[test]
fn a_personal_browser_prints_no_notice() -> Result<(), TestError> {
    let scratch = Scratch::new()?;
    let browser = scratch.outside_browser()?;
    scratch.personal(&browser, 0o600)?;

    let stderr = scratch.stderr(&[]);

    assert!(!stderr.contains("notice:"), "{stderr}");
    assert!(!stderr.contains("warning:"), "{stderr}");
    Ok(())
}

#[test]
fn a_tracked_personal_browser_is_refused_for_the_bundled_browser(
) -> Result<(), TestError> {
    let scratch = Scratch::new()?;
    let browser = scratch.outside_browser()?;
    scratch.personal(&browser, 0o600)?;
    scratch.track_personal()?;
    let plugin_root = scratch.empty_plugin_root()?;

    let stderr = scratch.stderr(&[("ACCELERATOR_PLUGIN_ROOT", &plugin_root)]);

    assert!(
        stderr.contains("warning: E_CONSENT_KEY_TRACKED: design.browser_path"),
        "{stderr}"
    );
    assert_eq!(scratch.browser_slots()?, ["bundled"]);
    Ok(())
}

#[test]
fn a_relative_personal_browser_is_warned_about() -> Result<(), TestError> {
    let scratch = Scratch::new()?;
    scratch.personal(Path::new("./chromium"), 0o600)?;

    let stderr = scratch.stderr(&[]);

    assert!(
        stderr.contains(
            "warning: E_EXECUTABLE_PATH_RELATIVE: design.browser_path"
        ),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn an_insecure_personal_browser_is_reported_once_as_the_ignored_file(
) -> Result<(), TestError> {
    let scratch = Scratch::new()?;
    let browser = scratch.outside_browser()?;
    scratch.personal(&browser, 0o644)?;

    let stderr = scratch.stderr(&[]);

    assert_eq!(
        stderr.matches("warning: E_LOCAL_PERMS_INSECURE").count(),
        1,
        "{stderr}"
    );
    assert_eq!(stderr.matches("warning:").count(), 1, "{stderr}");
    Ok(())
}

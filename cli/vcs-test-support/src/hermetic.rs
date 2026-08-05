//! The empty-config environment the fixture matrix is built and measured in,
//! and the preconditions that make its absence cells mean anything.
//!
//! The variables and value shapes live in one place so the recorded expectations
//! and the shipped harness cannot drift apart per platform.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::Error;

/// The lowest `git` the fixture matrix can be built with.
///
/// `git init --ref-format=reftable` landed in 2.45, and `git` is pinned nowhere
/// in this repo, so a floor with a named diagnostic beats recording a version.
pub const MINIMUM_GIT: (u32, u32) = (2, 45);

/// An isolated `HOME`, config and ceiling, applied to every `git` and `jj`
/// invocation the fixtures make.
#[derive(Debug, Clone)]
pub struct Hermetic {
    home: PathBuf,
    config_home: PathBuf,
    jj_config: PathBuf,
    ceiling: PathBuf,
}

impl Hermetic {
    /// Creates the isolated config tree beneath `base`.
    ///
    /// # Errors
    ///
    /// When the config tree cannot be written.
    pub fn rooted_at(base: &Path) -> Result<Self, Error> {
        let home = base.join("hermetic-home");
        let config_home = base.join("hermetic-xdg");
        let jj_config = base.join("hermetic-jj.toml");
        fs::create_dir_all(&home)?;
        fs::create_dir_all(&config_home)?;

        // Without an identity, git and jj fall back to user@hostname and refuse
        // the commit when the hostname carries no domain — the CI norm.
        fs::write(
            &jj_config,
            "[user]\nname = \"Fixture\"\nemail = \"fixture@example.com\"\n",
        )?;

        Ok(Self {
            home,
            config_home,
            jj_config,
            ceiling: base.to_path_buf(),
        })
    }

    /// Applies the isolated environment to `command`.
    pub fn apply(&self, command: &mut Command) {
        command.env("HOME", &self.home);
        command.env("XDG_CONFIG_HOME", &self.config_home);
        command.env("JJ_CONFIG", &self.jj_config);
        command.env("GIT_CEILING_DIRECTORIES", &self.ceiling);
        // A boolean, not a path: this is the only thing suppressing the
        // *system* gitconfig, whose location differs per platform.
        command.env("GIT_CONFIG_NOSYSTEM", "1");
        command.env("GIT_CONFIG_GLOBAL", "/dev/null");
        for key in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_COMMON_DIR",
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        ] {
            command.env_remove(key);
        }
    }

    /// Runs `git` in `dir` with the identity and isolation the fixtures need,
    /// passed per invocation rather than written into any config file.
    ///
    /// # Errors
    ///
    /// When git cannot be run or exits non-zero.
    pub fn git<S: AsRef<OsStr>>(
        &self,
        args: &[S],
        dir: &Path,
    ) -> Result<String, Error> {
        let mut command = Command::new("git");
        command.args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "protocol.file.allow=always",
        ]);
        command.args(args);
        self.run(command, dir, "git")
    }

    /// Runs `jj` in `dir` with the identity and isolation the fixtures need.
    ///
    /// # Errors
    ///
    /// When jj cannot be run or exits non-zero.
    pub fn jj<S: AsRef<OsStr>>(
        &self,
        args: &[S],
        dir: &Path,
    ) -> Result<String, Error> {
        let mut command = Command::new("jj");
        command.arg("--color=never");
        command.arg("--no-pager");
        command.args(args);
        self.run(command, dir, "jj")
    }

    fn run(
        &self,
        mut command: Command,
        dir: &Path,
        binary: &str,
    ) -> Result<String, Error> {
        command.current_dir(dir);
        self.apply(&mut command);
        let output = command
            .output()
            .map_err(|error| Error::message(format!("{binary}: {error}")))?;
        if !output.status.success() {
            return Err(Error::message(format!(
                "{binary} failed in {}: {}",
                dir.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
}

/// Fails when any ancestor of `base` carries a `.git` or `.jj` marker.
///
/// `gix::discover` reads no environment, so `GIT_CEILING_DIRECTORIES` cannot
/// fence it. On a host whose `TMPDIR` sits inside a repository, every cell
/// expecting absence would silently resolve the enclosing one.
///
/// # Errors
///
/// When a marked ancestor is found, naming it.
pub fn assert_no_repository_ancestor(base: &Path) -> Result<(), Error> {
    let mut dir = Some(base.canonicalize()?);
    while let Some(current) = dir {
        if current != base
            && (current.join(".git").exists() || current.join(".jj").exists())
        {
            return Err(Error::message(format!(
                "the fixture base {} lies inside the repository at {} — every \
                 absence expectation in the matrix would silently resolve that \
                 repository instead. Point TMPDIR outside any checkout.",
                base.display(),
                current.display()
            )));
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    Ok(())
}

/// Fails unless the installed `jj` CLI matches `jj_lib_version` at major.minor.
///
/// The pin-lockstep test compares two *declarations*; nothing else checks the
/// binaries that write the formats the libraries read, and a skew there surfaces
/// as an apparently wrong answer in the expected-value table.
///
/// Major.minor, because the tilde range permits the exact CLI pin to sit
/// alongside a resolved patch of the library.
///
/// # Errors
///
/// When `jj` cannot be run, or its version's major.minor differs.
pub fn assert_jj_matches(jj_lib_version: &str) -> Result<(), Error> {
    let reported = tool_version("jj")?;
    let expected = major_minor(jj_lib_version).ok_or_else(|| {
        Error::message(format!("unparsable jj-lib version {jj_lib_version}"))
    })?;
    let found = major_minor(&reported).ok_or_else(|| {
        Error::message(format!("unparsable jj --version output {reported}"))
    })?;
    if found != expected {
        return Err(Error::message(format!(
            "the installed jj CLI is {reported} but jj-lib is \
             {jj_lib_version}: the CLI writes the format the library reads, so \
             the fixture matrix would be measuring a skew. Run `mise install`."
        )));
    }
    Ok(())
}

/// Fails unless the installed `git` meets [`MINIMUM_GIT`].
///
/// # Errors
///
/// When `git` cannot be run, or is older than the floor.
pub fn assert_git_is_recent_enough() -> Result<(), Error> {
    let reported = tool_version("git")?;
    let found = major_minor(&reported).ok_or_else(|| {
        Error::message(format!("unparsable git --version output {reported}"))
    })?;
    if found < MINIMUM_GIT {
        return Err(Error::message(format!(
            "git {reported} is below the {}.{} floor the fixture matrix needs \
             (--ref-format=reftable landed in 2.45)",
            MINIMUM_GIT.0, MINIMUM_GIT.1
        )));
    }
    Ok(())
}

fn tool_version(binary: &str) -> Result<String, Error> {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|error| Error::message(format!("{binary}: {error}")))?;
    if !output.status.success() {
        return Err(Error::message(format!(
            "{binary} --version exited non-zero"
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn major_minor(text: &str) -> Option<(u32, u32)> {
    let digits = text.split_whitespace().find_map(|word| {
        let trimmed = word.trim_start_matches(|c: char| !c.is_ascii_digit());
        trimmed.chars().next()?.is_ascii_digit().then_some(trimmed)
    })?;
    let mut parts = digits.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

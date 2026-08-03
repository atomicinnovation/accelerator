//! The empty-config environment the fixture matrix is built and measured in,
//! and the preconditions that make its absence cells mean anything.
//!
//! The exact variables and value shapes here are the ones the recorded oracle
//! mapping was measured with. They live in one place so the mapping and the
//! shipped harness cannot drift apart per platform.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::Error;

/// The lowest `git` the fixture matrix can be built with.
///
/// `git init --ref-format=reftable` landed in 2.45; the mapping itself was
/// calibrated against 2.54.0. `git` is pinned nowhere in this repo, so a floor
/// with a named diagnostic beats recording a version nobody reads.
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

        // An identity is written rather than left empty: with HOME at a temp
        // dir, GIT_CONFIG_GLOBAL at /dev/null and GIT_CONFIG_NOSYSTEM set, jj
        // and git otherwise fall back to an auto-detected user@hostname and
        // *refuse* the commit when the hostname carries no domain — the normal
        // case on CI runners and in containers.
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
        // A boolean, not a path. This is the only thing suppressing the
        // *system* gitconfig, which lives at /etc/gitconfig on ubuntu and
        // inside the Command Line Tools on macOS; pointing it at a temp dir
        // would leave host config leaking in on one platform and not the other.
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

    /// Runs `git` in `dir` with the identity and isolation the fixtures need.
    ///
    /// `protocol.file.allow` and the identity are passed per invocation rather
    /// than written into a config file, even one under the temp `HOME`.
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
/// Roughly a quarter of the oracle mapping's cells assert absence for the
/// gix-backed queries, and `gix::discover` reads **no** environment — so
/// `GIT_CEILING_DIRECTORIES`, which produced the oracle side's exit-128s, cannot
/// fence the library side. On a host whose `TMPDIR` resolves inside a
/// repository, those cells would silently resolve the enclosing repository
/// instead, and the failure would read as a mass of confusing per-cell
/// mismatches rather than one legible message.
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
/// binaries that write the repository formats the libraries read. This session's
/// planning hit exactly that skew, and it would otherwise surface as an
/// apparently wrong answer in a 24-row expected-value table.
///
/// Compared at major.minor because the tilde range deliberately permits the
/// exact CLI pin to sit alongside a resolved patch of the library; an
/// exact-equality assertion would fail the whole matrix on a skew the pins
/// pre-authorise.
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

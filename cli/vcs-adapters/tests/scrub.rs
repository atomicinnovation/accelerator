//! Every query answers identically whether or not the ambient git and jj
//! environment is poisoned.
//!
//! A property to verify rather than implement: `gix::discover`/`gix::open` do
//! not consult the environment and jj-lib's loader is pure filesystem reads, so
//! nothing scrubs — but the claim is only worth checking against a poison that
//! would really change the answer.
//!
//! Poison means *another fixture's real `.git`*, not an empty or nonexistent
//! path, which both git and the libraries ignore. The variable set covers the
//! object-directory and `GIT_CONFIG_COUNT` families as well, because
//! `gix::open`'s default permissions do consult the environment for object
//! directories and for config discovery, and `is_bare()` reads `core.bare`
//! through that config.
//!
//! Both arms run through the same child binary: comparing an in-process value
//! against a child's rendered output would compare two serialisation routes. A
//! child is needed at all because libtest runs each test on a spawned thread, so
//! in-process `set_var` is racy by construction.
#![cfg(feature = "bash-parity")]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use vcs_test_support::fixtures::Matrix;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const FIXTURE: &str = env!("CARGO_BIN_EXE_vcs-adapters-fixture");

fn run(start: &Path, poison: Option<&Poison>) -> Result<String, TestError> {
    let mut command = Command::new(FIXTURE);
    command.arg("all").arg(start);
    if let Some(poison) = poison {
        poison.apply(&mut command);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "fixture binary failed for {}: {}",
            start.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// A real repository's git dir plus a divergent global config, the two families
/// `gix::open`'s default permissions actually consult — enough to move a facts
/// answer if anything read the environment (nothing does).
struct Poison {
    git_dir: String,
    global_config: String,
}

impl Poison {
    fn apply(&self, command: &mut Command) {
        for key in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_COMMON_DIR",
            "GIT_OBJECT_DIRECTORY",
        ] {
            command.env(key, &self.git_dir);
        }
        command.env("GIT_INDEX_FILE", format!("{}/index", self.git_dir));
        command.env(
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            format!("{}/objects", self.git_dir),
        );
        command.env("GIT_CONFIG", &self.global_config);
        command.env("GIT_CONFIG_GLOBAL", &self.global_config);
        command.env("GIT_CONFIG_SYSTEM", &self.global_config);
        command.env("JJ_CONFIG", &self.global_config);
        command.env("GIT_CONFIG_NOSYSTEM", "0");
        // The three-part override family, which git honours as a config source.
        command.env("GIT_CONFIG_COUNT", "1");
        command.env("GIT_CONFIG_KEY_0", "core.bare");
        command.env("GIT_CONFIG_VALUE_0", "true");
    }
}

#[test]
fn every_query_is_unmoved_by_a_poisoned_environment() -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("vcs-scrub-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;

    // The poison is a *real* repository's git dir, and the global config it
    // names asserts a divergent `core.bare` — the Phase-2 empty-config
    // environment would otherwise mask exactly the config influence this
    // invariant is meant to exclude.
    let poison_repo = matrix.start("PG-r")?.join(".git");
    let poison_config = matrix.base.join("poison.gitconfig");
    fs::write(&poison_config, "[core]\n\tbare = true\n")?;
    let poison = Poison {
        git_dir: poison_repo.display().to_string(),
        global_config: poison_config.display().to_string(),
    };

    let mut differences = String::new();
    for fixture in &matrix.fixtures {
        let clean = run(&fixture.start, None)?;
        let poisoned = run(&fixture.start, Some(&poison))?;
        if clean != poisoned {
            writeln!(
                differences,
                "  {}\n    clean:    {}\n    poisoned: {}",
                fixture.key,
                clean.replace('\n', " | "),
                poisoned.replace('\n', " | ")
            )?;
        }
    }

    assert!(
        differences.is_empty(),
        "the environment moved these answers:\n{differences}"
    );
    Ok(())
}

#[test]
fn the_poison_is_live() -> Result<(), TestError> {
    let base = tempfile::Builder::new()
        .prefix("vcs-scrub-control-")
        .tempdir()?;
    let matrix = Matrix::build_in(base.path())?;

    let poison_repo = matrix.start("PG-r")?.join(".git");
    let poison_config = matrix.base.join("poison.gitconfig");
    fs::write(&poison_config, "[core]\n\tbare = true\n")?;
    let poison = Poison {
        git_dir: poison_repo.display().to_string(),
        global_config: poison_config.display().to_string(),
    };

    // `discover_with_environment_overrides` is the ready-made control: same
    // library, same fixture, same poison — but it *does* read the environment.
    // Without it the invariant above could hold because nothing was poisoned.
    let start = matrix.start("NGJ-o")?;
    let mut command = Command::new(FIXTURE);
    command.arg("control").arg(start);
    poison.apply(&mut command);
    let output = command.output()?;
    let reported = String::from_utf8(output.stdout)?;

    let expected = poison_repo.canonicalize()?;
    assert!(
        reported.contains(&expected.display().to_string()),
        "the control should have resolved the poison target {}, but reported \
         {reported} — if it did not, the poisoning is not live and the \
         invariant test above proves nothing",
        expected.display()
    );
    Ok(())
}

/// Status reads global git config, unlike the facts queries. The old subprocess
/// path forced `GIT_CONFIG_NOSYSTEM=1` and scrubbed the global config; in-process
/// `gix::open` honours it, exactly as real `git status` shows the developer. This
/// pins that intended behaviour rather than a false invariance — a
/// `core.excludesFile` that ignores an untracked file removes it from the render.
#[test]
fn status_honours_the_global_excludes_file() -> Result<(), TestError> {
    let base = tempfile::Builder::new()
        .prefix("vcs-scrub-status-")
        .tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;

    let repo = base.path().join("repo");
    fs::create_dir_all(&repo)?;
    env.git(&["init", "--quiet"], &repo)?;
    fs::write(repo.join("tracked.txt"), "t\n")?;
    env.git(&["add", "tracked.txt"], &repo)?;
    env.git(&["commit", "--quiet", "-m", "init"], &repo)?;
    fs::write(repo.join("ignore-me.log"), "noise\n")?;

    let excludes = base.path().join("excludes");
    fs::write(&excludes, "*.log\n")?;
    let global_config = base.path().join("global.gitconfig");
    fs::write(
        &global_config,
        format!("[core]\n\texcludesFile = {}\n", excludes.display()),
    )?;

    let status_under = |config: &Path| -> Result<String, TestError> {
        let output = Command::new(FIXTURE)
            .arg("status")
            .arg(&repo)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_CONFIG_GLOBAL", config)
            .output()?;
        assert!(
            output.status.success(),
            "fixture status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(String::from_utf8(output.stdout)?)
    };

    let clean = status_under(Path::new("/dev/null"))?;
    assert!(
        clean.contains("untracked  ignore-me.log"),
        "with no excludes, the untracked file must appear: {clean:?}"
    );

    let hidden = status_under(&global_config)?;
    assert!(
        !hidden.contains("ignore-me.log"),
        "the global core.excludesFile must hide the untracked file, proving \
         status honours global config: {hidden:?}"
    );
    Ok(())
}

//! The scrub invariant: every query answers identically whether or not the
//! ambient git and jj environment is poisoned.
//!
//! This is a property to **verify**, not to implement. `gix::discover`/
//! `gix::open` do not consult the environment, and jj-lib's workspace loader is
//! pure filesystem reads, so no scrub is written — but the claim is only worth
//! anything if it is checked, and checked against a poison that would really
//! change the answer.
//!
//! Poison means *another fixture's real `.git`*, not an empty or nonexistent
//! path, which both git and the libraries ignore. The variable set is
//! everything `scrub_environment` touches, **plus** the object-directory and
//! `GIT_CONFIG_COUNT` families: `gix::open`'s default permissions do consult the
//! environment for object directories and for system/global config discovery,
//! and `is_bare()` reads `core.bare` *through* that config, so verifying two
//! variables and generalising to "uniformly immune" would be broader than the
//! evidence.
//!
//! Both arms run through the same child binary. Comparing an in-process value
//! against a child's rendered output would compare two serialisation routes —
//! producing false failures across the matrix, and the more dangerous false
//! pass where both arms render absence identically for different reasons. A
//! child is needed at all because libtest runs each test on a spawned thread,
//! so in-process `set_var` is racy by construction.
#![cfg(feature = "bash-parity")]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use vcs_test_support::fixtures::Matrix;
use vcs_test_support::hermetic::assert_git_is_recent_enough;

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

/// Every variable the subprocess probe scrubs or forces, plus the two families
/// `gix::open`'s default permissions actually consult.
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
    assert_git_is_recent_enough()?;
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
    assert_git_is_recent_enough()?;
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

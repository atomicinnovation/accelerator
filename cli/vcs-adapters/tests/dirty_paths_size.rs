//! The jj dirty paths honour `snapshot.max-new-file-size` from every layer jj
//! reads for `jj status`, conditional scopes included, read through the
//! fixture binary under a hermetic environment and compared with `jj status`.
#![cfg(feature = "bash-parity")]

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use support::dirty_paths;
use support::TestError;
use tempfile::TempDir;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::jj_status::changed_paths;

const COLOCATIONS: [&str; 2] = ["--no-colocate", "--colocate"];
const TWO_KIB: &str = "meta/two";
const ONE_KIB: &str = "meta/one";
const HALF_KIB: &str = "meta/half";
const IDENTITY: &str =
    "[user]\nname = \"Fixture\"\nemail = \"fixture@example.com\"\n";

struct Repo {
    work: TempDir,
    env: Hermetic,
    root: PathBuf,
}

impl Repo {
    fn new(colocation: &str) -> Result<Self, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let work = tempfile::Builder::new()
            .prefix("vcs-dirty-size-")
            .tempdir()?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(root.join("meta"))?;
        env.jj(&["git", "init", colocation], &root)?;
        fs::write(root.join("meta/base.md"), "base")?;
        env.jj(&["commit", "-m", "base"], &root)?;
        Ok(Self { work, env, root })
    }

    fn jj(&self, args: &[&str]) -> Result<String, TestError> {
        Ok(self.env.jj(args, &self.root)?)
    }

    fn set(&self, level: &str, value: &str) -> Result<(), TestError> {
        self.jj(&[
            "config",
            "set",
            level,
            "snapshot.max-new-file-size",
            value,
        ])?;
        Ok(())
    }

    fn with_user_config(&self, extra: &str) -> Result<Hermetic, TestError> {
        let path = self.work.path().join("user.toml");
        fs::write(&path, format!("{IDENTITY}{extra}"))?;
        Ok(self.env.clone().with_jj_config(path))
    }

    fn write_new_files(&self) -> Result<(), TestError> {
        fs::write(self.root.join(TWO_KIB), vec![b'x'; 2048])?;
        fs::write(self.root.join(ONE_KIB), vec![b'x'; 1024])?;
        fs::write(self.root.join(HALF_KIB), vec![b'x'; 512])?;
        Ok(())
    }

    fn listed_under(
        &self,
        env: &Hermetic,
    ) -> Result<BTreeSet<String>, TestError> {
        let output = support::query(env, "dirty_paths", &self.root)?;
        let stderr = String::from_utf8(output.stderr)?;
        assert!(!stderr.contains("WARN"), "{stderr}");
        let listed = dirty_paths(env, &self.root)?;
        assert_eq!(listed, changed_paths(env, &self.root)?);
        Ok(listed)
    }
}

fn limit(value: &str) -> String {
    format!("[snapshot]\nmax-new-file-size = {value}\n")
}

fn paths(listed: &[&str]) -> BTreeSet<String> {
    listed.iter().map(|path| (*path).to_owned()).collect()
}

fn a_one_kib_limit_leaves_out_the_larger_file(
    configure: impl Fn(&Repo) -> Result<Hermetic, TestError>,
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        let env = configure(&repo)?;
        repo.write_new_files()?;

        assert_eq!(repo.listed_under(&env)?, paths(&[HALF_KIB, ONE_KIB]));
    }
    Ok(())
}

fn a_one_mib_limit_lists_every_file(
    configure: impl Fn(&Repo) -> Result<Hermetic, TestError>,
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        let env = configure(&repo)?;
        repo.write_new_files()?;

        assert_eq!(
            repo.listed_under(&env)?,
            paths(&[HALF_KIB, ONE_KIB, TWO_KIB])
        );
    }
    Ok(())
}

#[test]
fn the_repo_limit_wins_over_the_user_limit() -> Result<(), TestError> {
    a_one_kib_limit_leaves_out_the_larger_file(|repo| {
        repo.set("--repo", "1KiB")?;
        repo.with_user_config(&limit("\"1MiB\""))
    })
}

#[test]
fn the_workspace_limit_wins_over_the_repo_limit() -> Result<(), TestError> {
    a_one_kib_limit_leaves_out_the_larger_file(|repo| {
        repo.set("--repo", "1MiB")?;
        repo.set("--workspace", "1KiB")?;
        Ok(repo.env.clone())
    })
}

#[test]
fn a_user_limit_from_jj_config_applies() -> Result<(), TestError> {
    a_one_kib_limit_leaves_out_the_larger_file(|repo| {
        repo.with_user_config(&limit("\"1KiB\""))
    })
}

#[test]
fn every_value_form_reads_the_same() -> Result<(), TestError> {
    for value in ["1024", "\"1024\"", "\"1KiB\""] {
        a_one_kib_limit_leaves_out_the_larger_file(|repo| {
            repo.with_user_config(&limit(value))
        })?;
    }
    Ok(())
}

#[test]
fn a_repo_limit_can_raise_the_user_limit() -> Result<(), TestError> {
    a_one_mib_limit_lists_every_file(|repo| {
        repo.set("--repo", "1MiB")?;
        repo.with_user_config(&limit("\"1KiB\""))
    })
}

#[test]
fn a_workspace_limit_can_raise_the_repo_limit() -> Result<(), TestError> {
    a_one_mib_limit_lists_every_file(|repo| {
        repo.set("--repo", "1KiB")?;
        repo.set("--workspace", "1MiB")?;
        Ok(repo.env.clone())
    })
}

#[test]
fn an_unset_limit_is_one_mib_inclusive() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        fs::write(repo.root.join("meta/exact"), vec![b'x'; 1024 * 1024])?;
        fs::write(repo.root.join("meta/over"), vec![b'x'; 2 * 1024 * 1024])?;

        assert_eq!(repo.listed_under(&repo.env)?, paths(&["meta/exact"]));
    }
    Ok(())
}

#[test]
fn a_zero_limit_lists_every_file() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        repo.set("--repo", "0")?;
        fs::write(repo.root.join("meta/over"), vec![b'x'; 2 * 1024 * 1024])?;

        assert_eq!(repo.listed_under(&repo.env)?, paths(&["meta/over"]));
    }
    Ok(())
}

fn hashed(files: &[PathBuf]) -> Result<Vec<Option<Vec<u8>>>, TestError> {
    files.iter().map(|file| Ok(fs::read(file).ok())).collect()
}

#[test]
fn a_legacy_config_inside_jj_is_read_and_left_alone() -> Result<(), TestError> {
    for legacy in ["repo/config.toml", "workspace-config.toml"] {
        for colocation in COLOCATIONS {
            let repo = Repo::new(colocation)?;
            let file = repo.root.join(".jj").join(legacy);
            fs::write(&file, limit("\"1KiB\""))?;
            let env = repo.with_user_config(&limit("\"1MiB\""))?;
            repo.write_new_files()?;
            let watched = [
                file.clone(),
                repo.root.join(".jj/repo/config-id"),
                repo.root.join(".jj/workspace-config-id"),
            ];
            let before = hashed(&watched)?;

            let listed = dirty_paths(&env, &repo.root)?;

            assert_eq!(listed, paths(&[HALF_KIB, ONE_KIB]), "{legacy}");
            assert_eq!(hashed(&watched)?, before, "{legacy}");
            assert!(!file.is_symlink(), "{legacy}");
        }
    }
    Ok(())
}

#[test]
fn a_copied_repository_reads_the_limit_of_the_config_it_was_copied_with(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        repo.set("--repo", "1KiB")?;
        repo.jj(&["status"])?;
        let copy = repo.work.path().join("copy");
        let status = std::process::Command::new("cp")
            .args(["-R"])
            .arg(&repo.root)
            .arg(&copy)
            .status()?;
        assert!(status.success());
        fs::write(copy.join(TWO_KIB), vec![b'x'; 2048])?;
        fs::write(copy.join(ONE_KIB), vec![b'x'; 1024])?;
        let id_file = copy.join(".jj/repo/config-id");
        let before = fs::read(&id_file)?;

        let listed = dirty_paths(&repo.env, &copy)?;

        assert_eq!(listed, paths(&[ONE_KIB]));
        assert_eq!(fs::read(&id_file)?, before);
    }
    Ok(())
}

#[test]
fn a_repository_scope_applies_only_to_its_repository() -> Result<(), TestError>
{
    for (scoped_to_this, expected) in [
        (true, paths(&[HALF_KIB, ONE_KIB])),
        (false, paths(&[HALF_KIB, ONE_KIB, TWO_KIB])),
    ] {
        let repo = Repo::new("--no-colocate")?;
        let repository = if scoped_to_this {
            repo.root.canonicalize()?.display().to_string()
        } else {
            "/elsewhere".to_owned()
        };
        let env = repo.with_user_config(&format!(
            "[[--scope]]\n--when.repositories = [{repository:?}]\n\
             [--scope.snapshot]\nmax-new-file-size = \"1KiB\"\n"
        ))?;
        repo.write_new_files()?;

        assert_eq!(repo.listed_under(&env)?, expected, "{repository}");
    }
    Ok(())
}

#[test]
fn an_environment_scope_applies_only_when_the_variable_is_set(
) -> Result<(), TestError> {
    for (set, expected) in [
        (true, paths(&[HALF_KIB, ONE_KIB])),
        (false, paths(&[HALF_KIB, ONE_KIB, TWO_KIB])),
    ] {
        let repo = Repo::new("--no-colocate")?;
        let mut env = repo.with_user_config(
            "[[--scope]]\n--when.environments = [\"LIMIT_SMALL=yes\"]\n\
             [--scope.snapshot]\nmax-new-file-size = \"1KiB\"\n",
        )?;
        if set {
            env = env.with_variable("LIMIT_SMALL", "yes");
        }
        repo.write_new_files()?;

        assert_eq!(repo.listed_under(&env)?, expected, "set: {set}");
    }
    Ok(())
}

#[test]
fn a_hostname_scope_applies_on_this_host() -> Result<(), TestError> {
    let repo = Repo::new("--no-colocate")?;
    let hostname = std::process::Command::new("hostname").output()?;
    let hostname = String::from_utf8(hostname.stdout)?.trim().to_owned();
    let env = repo.with_user_config(&format!(
        "[[--scope]]\n--when.hostnames = [{hostname:?}]\n\
         [--scope.snapshot]\nmax-new-file-size = \"1KiB\"\n"
    ))?;
    let mut probe = std::process::Command::new("jj");
    env.apply(&mut probe);
    let resolved = probe
        .args(["config", "get", "snapshot.max-new-file-size"])
        .current_dir(&repo.root)
        .output()?;
    assert_eq!(String::from_utf8(resolved.stdout)?.trim(), "1KiB");
    repo.write_new_files()?;

    assert_eq!(repo.listed_under(&env)?, paths(&[HALF_KIB, ONE_KIB]));
    Ok(())
}

#[test]
fn an_invalid_limit_is_warned_about_and_every_file_listed(
) -> Result<(), TestError> {
    let repo = Repo::new("--no-colocate")?;
    repo.set("--repo", "\"abc\"")?;
    repo.write_new_files()?;

    let output = support::query(&repo.env, "dirty_paths", &repo.root)?;

    assert!(output.status.success());
    let listed: BTreeSet<String> = String::from_utf8(output.stdout)?
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(listed, paths(&[HALF_KIB, ONE_KIB, TWO_KIB]));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("WARN"), "{stderr}");
    assert!(stderr.contains("snapshot.max-new-file-size"), "{stderr}");
    Ok(())
}

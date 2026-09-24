use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;

use super::JjConfigEnvironment;
use super::JjConfigSources;
use super::MaxNewFileSize;

type TestError = Box<dyn std::error::Error>;

const KIB: u64 = 1024;
const MIB: u64 = 1024 * 1024;
const REPO_ID: &str = "0123456789abcdef0123";
const WORKSPACE_ID: &str = "fedcba9876543210fedc";

struct Setup {
    dir: TempDir,
    environment: JjConfigEnvironment,
}

impl Setup {
    fn new() -> Result<Self, TestError> {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("ws/.jj/repo"))?;
        let environment = JjConfigEnvironment {
            jj_config: None,
            home: Some(dir.path().join("home")),
            user_config_dir: Some(dir.path().join("config")),
            system_config_paths: vec![
                dir.path().join("etc/jj/config.toml"),
                dir.path().join("etc/jj/conf.d"),
            ],
            hostname: "builder".to_owned(),
            variables: HashMap::from([("CI".to_owned(), "true".to_owned())]),
        };
        Ok(Self { dir, environment })
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn workspace_root(&self) -> PathBuf {
        self.path("ws")
    }

    fn repo_path(&self) -> PathBuf {
        self.path("ws/.jj/repo")
    }

    fn write(&self, relative: &str, content: &str) -> Result<(), TestError> {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn system(&self, content: &str) -> Result<(), TestError> {
        self.write("etc/jj/config.toml", content)
    }

    fn user(&self, content: &str) -> Result<(), TestError> {
        self.write("config/jj/config.toml", content)
    }

    fn repo(&self, content: &str) -> Result<(), TestError> {
        self.write("ws/.jj/repo/config-id", REPO_ID)?;
        self.write(&format!("config/jj/repos/{REPO_ID}/config.toml"), content)
    }

    fn workspace(&self, content: &str) -> Result<(), TestError> {
        self.write("ws/.jj/workspace-config-id", WORKSPACE_ID)?;
        self.write(
            &format!("config/jj/workspaces/{WORKSPACE_ID}/config.toml"),
            content,
        )
    }

    fn sources(&self) -> JjConfigSources {
        JjConfigSources::for_workspace(
            &self.workspace_root(),
            &self.repo_path(),
            &self.environment,
        )
    }

    fn limit(&self) -> Result<u64, TestError> {
        let (root, repo) = (self.workspace_root(), self.repo_path());
        let context = self.environment.status_resolution_context(&root, &repo);
        Ok(MaxNewFileSize::resolve(&self.sources(), &context)?
            .as_snapshot_limit())
    }

    fn limit_or_unlimited(&self) -> u64 {
        let (root, repo) = (self.workspace_root(), self.repo_path());
        let context = self.environment.status_resolution_context(&root, &repo);
        MaxNewFileSize::resolve_or_unlimited(&self.sources(), &context)
            .as_snapshot_limit()
    }
}

fn limit_setting(value: &str) -> String {
    format!("[snapshot]\nmax-new-file-size = {value}\n")
}

#[test]
fn the_most_specific_layer_setting_the_limit_wins() -> Result<(), TestError> {
    type Layer = fn(&Setup, &str) -> Result<(), TestError>;
    let cases: [(&str, Layer, Layer); 4] = [
        ("repo over user", Setup::repo, Setup::user),
        ("workspace over repo", Setup::workspace, Setup::repo),
        ("user over system", Setup::user, Setup::system),
        ("repo over system", Setup::repo, Setup::system),
    ];
    for (name, winner, loser) in cases {
        let setup = Setup::new()?;
        winner(&setup, &limit_setting("\"1KiB\""))?;
        loser(&setup, &limit_setting("\"1MiB\""))?;

        assert_eq!(setup.limit()?, KIB, "{name}");
    }
    Ok(())
}

#[test]
fn a_later_layer_can_raise_the_limit() -> Result<(), TestError> {
    type Layer = fn(&Setup, &str) -> Result<(), TestError>;
    let cases: [(&str, Layer, Layer); 3] = [
        ("repo raises user", Setup::repo, Setup::user),
        ("workspace raises repo", Setup::workspace, Setup::repo),
        ("user raises system", Setup::user, Setup::system),
    ];
    for (name, later, earlier) in cases {
        let setup = Setup::new()?;
        earlier(&setup, &limit_setting("\"1KiB\""))?;
        later(&setup, &limit_setting("\"1MiB\""))?;

        assert_eq!(setup.limit()?, MIB, "{name}");
    }
    Ok(())
}

#[test]
fn a_single_layer_sets_the_limit() -> Result<(), TestError> {
    type Layer = fn(&Setup, &str) -> Result<(), TestError>;
    let layers: [(&str, Layer); 2] =
        [("user", Setup::user), ("system", Setup::system)];
    for (name, layer) in layers {
        let setup = Setup::new()?;
        layer(&setup, &limit_setting("\"1KiB\""))?;

        assert_eq!(setup.limit()?, KIB, "{name}");
    }
    Ok(())
}

#[test]
fn every_value_form_jj_accepts_reads_the_same() -> Result<(), TestError> {
    for value in ["1024", "\"1024\"", "\"1KiB\""] {
        let setup = Setup::new()?;
        setup.repo(&limit_setting(value))?;

        assert_eq!(setup.limit()?, KIB, "{value}");
    }
    Ok(())
}

#[test]
fn an_unset_limit_is_jjs_default() -> Result<(), TestError> {
    assert_eq!(Setup::new()?.limit()?, MIB);
    Ok(())
}

#[test]
fn a_zero_limit_imposes_none() -> Result<(), TestError> {
    let setup = Setup::new()?;
    setup.repo(&limit_setting("0"))?;

    assert_eq!(setup.limit()?, u64::MAX);
    Ok(())
}

#[test]
fn an_invalid_limit_names_the_setting_and_imposes_none() -> Result<(), TestError>
{
    let setup = Setup::new()?;
    setup.repo(&limit_setting("\"abc\""))?;

    let error = setup.limit().err().ok_or("expected an error")?;

    assert!(
        error.to_string().contains("snapshot.max-new-file-size"),
        "{error}"
    );
    assert_eq!(setup.limit_or_unlimited(), u64::MAX);
    Ok(())
}

#[test]
fn a_malformed_layer_imposes_no_limit() -> Result<(), TestError> {
    let setup = Setup::new()?;
    setup.repo("[snapshot\n")?;

    assert_eq!(setup.limit_or_unlimited(), u64::MAX);
    Ok(())
}

#[test]
fn jj_config_replaces_the_user_files_and_drops_the_system_layer(
) -> Result<(), TestError> {
    let mut setup = Setup::new()?;
    setup.system(&limit_setting("\"1KiB\""))?;
    setup.user(&limit_setting("\"1KiB\""))?;
    setup.write("override.toml", &limit_setting("\"2KiB\""))?;
    setup.environment.jj_config = Some(setup.path("override.toml").into());

    assert_eq!(setup.limit()?, 2 * KIB);

    setup.write("override.toml", "")?;
    assert_eq!(setup.limit()?, MIB);
    Ok(())
}

#[test]
fn conf_d_wins_over_the_user_config_file() -> Result<(), TestError> {
    let setup = Setup::new()?;
    setup.user(&limit_setting("\"1MiB\""))?;
    setup.write("config/jj/conf.d/limit.toml", &limit_setting("\"1KiB\""))?;

    assert_eq!(setup.limit()?, KIB);
    Ok(())
}

#[test]
fn a_scope_for_this_repository_applies() -> Result<(), TestError> {
    let setup = Setup::new()?;
    let repository = setup.workspace_root().display().to_string();
    setup.user(&format!(
        "[[--scope]]\n--when.repositories = [{repository:?}]\n\
         [--scope.snapshot]\nmax-new-file-size = \"1KiB\"\n"
    ))?;

    assert_eq!(setup.limit()?, KIB);
    Ok(())
}

#[test]
fn a_scope_for_another_repository_does_not_apply() -> Result<(), TestError> {
    let setup = Setup::new()?;
    setup.user(
        "[[--scope]]\n--when.repositories = [\"/elsewhere\"]\n\
         [--scope.snapshot]\nmax-new-file-size = \"1KiB\"\n",
    )?;

    assert_eq!(setup.limit()?, MIB);
    Ok(())
}

#[test]
fn a_conf_d_file_conditioned_on_another_repository_does_not_apply(
) -> Result<(), TestError> {
    let setup = Setup::new()?;
    setup.write(
        "config/jj/conf.d/elsewhere.toml",
        "--when.repositories = [\"/elsewhere\"]\n\
         [snapshot]\nmax-new-file-size = \"1KiB\"\n",
    )?;

    assert_eq!(setup.limit()?, MIB);
    Ok(())
}

#[test]
fn a_scope_for_another_command_does_not_apply() -> Result<(), TestError> {
    let setup = Setup::new()?;
    setup.user(
        "[[--scope]]\n--when.commands = [\"log\"]\n\
         [--scope.snapshot]\nmax-new-file-size = \"1KiB\"\n",
    )?;

    assert_eq!(setup.limit()?, MIB);
    Ok(())
}

#[test]
fn a_hostname_scope_applies_only_on_that_host() -> Result<(), TestError> {
    for (host, expected) in [("builder", KIB), ("elsewhere", MIB)] {
        let setup = Setup::new()?;
        setup.user(&format!(
            "[[--scope]]\n--when.hostnames = [{host:?}]\n\
             [--scope.snapshot]\nmax-new-file-size = \"1KiB\"\n"
        ))?;

        assert_eq!(setup.limit()?, expected, "{host}");
    }
    Ok(())
}

#[test]
fn an_environment_scope_applies_only_when_the_variable_matches(
) -> Result<(), TestError> {
    for (variable, expected) in [("CI=true", KIB), ("DEPLOY=true", MIB)] {
        let setup = Setup::new()?;
        setup.user(&format!(
            "[[--scope]]\n--when.environments = [{variable:?}]\n\
             [--scope.snapshot]\nmax-new-file-size = \"1KiB\"\n"
        ))?;

        assert_eq!(setup.limit()?, expected, "{variable}");
    }
    Ok(())
}

#[test]
fn a_malformed_config_id_reads_no_repo_config() -> Result<(), TestError> {
    let setup = Setup::new()?;
    setup.repo(&limit_setting("\"1KiB\""))?;
    setup.write("ws/.jj/repo/config-id", "not-an-id")?;

    assert_eq!(setup.sources().repo, None);
    assert_eq!(setup.limit()?, MIB);
    Ok(())
}

#[test]
fn legacy_config_inside_jj_is_read_without_an_id_file() -> Result<(), TestError>
{
    let setup = Setup::new()?;
    setup.write("ws/.jj/repo/config.toml", &limit_setting("\"1KiB\""))?;
    assert_eq!(setup.limit()?, KIB);

    let setup = Setup::new()?;
    setup.write("ws/.jj/workspace-config.toml", &limit_setting("\"1KiB\""))?;
    assert_eq!(setup.limit()?, KIB);
    Ok(())
}

fn snapshot(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, TestError> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push((path.clone(), fs::read(&path)?));
            }
        }
    }
    found.sort();
    Ok(found)
}

#[test]
fn resolving_writes_nothing() -> Result<(), TestError> {
    for legacy in [false, true] {
        let setup = Setup::new()?;
        if legacy {
            setup
                .write("ws/.jj/repo/config.toml", &limit_setting("\"1KiB\""))?;
            setup.write("ws/.jj/workspace-config.toml", "")?;
        } else {
            setup.repo(&limit_setting("\"1KiB\""))?;
            setup.workspace("")?;
        }
        let before = snapshot(setup.dir.path())?;

        setup.limit()?;

        assert_eq!(snapshot(setup.dir.path())?, before, "legacy: {legacy}");
    }
    Ok(())
}

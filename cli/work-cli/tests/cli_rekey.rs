//! `work rekey` through the binary: the override it honours, and what a
//! re-keyed corpus resolves to afterwards.

use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use vcs_test_support::hermetic::Hermetic;

mod common;

type TestError = Box<dyn std::error::Error>;

const CONFIG: &str = "---\nwork:\n  integration: linear\n  id_pattern: \
                      \"{key}-{number:04d}\"\n  key: ACC\n---\n";
const ITEM: &str =
    "---\nid: \"0042\"\ntitle: \"Search\"\n---\n\n# 0042: Search\n";
const NOTE: &str = "meta/notes/n.md";

struct Repo {
    dir: tempfile::TempDir,
    _env_dir: tempfile::TempDir,
    env: Hermetic,
}

impl Repo {
    fn committed() -> Result<Self, TestError> {
        Self::committed_with(&[])
    }

    fn committed_with(items: &[(&str, &str)]) -> Result<Self, TestError> {
        let mut files = vec![
            (".accelerator/config.md", CONFIG),
            ("meta/work/0042-search.md", ITEM),
            (NOTE, "See work-item:0042.\n"),
        ];
        files.extend_from_slice(items);
        Self::committed_as(&files)
    }

    fn committed_as(files: &[(&str, &str)]) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix("work-cli-rekey-")
            .tempdir()?;
        let env_dir = tempfile::tempdir()?;
        let env = Hermetic::rooted_at(env_dir.path())?;
        let repo = Self {
            dir,
            _env_dir: env_dir,
            env,
        };
        for (relative, content) in files {
            repo.write(relative, content)?;
        }
        repo.env.git(&["init", "--quiet"], repo.root())?;
        repo.commit("init")?;
        Ok(repo)
    }

    fn commit(&self, message: &str) -> Result<(), TestError> {
        self.env.git(&["add", "-A"], self.root())?;
        self.env
            .git(&["commit", "--quiet", "-m", message], self.root())?;
        Ok(())
    }

    fn work_items(&self) -> Result<Vec<String>, TestError> {
        let mut names: Vec<String> =
            fs::read_dir(self.root().join("meta/work"))?
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_file())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
        names.sort();
        Ok(names)
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, relative: &str, content: &str) -> Result<(), TestError> {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn run(
        &self,
        args: &[&str],
        environment: &[(&str, &str)],
    ) -> Result<std::process::Output, TestError> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
        command
            .args(args)
            .current_dir(self.root())
            .env("ACCELERATOR_PLUGIN_ROOT", self.root())
            .env_remove("ACCELERATOR_FORCE")
            .env_remove("ACCELERATOR_MIGRATE_FORCE")
            .stdin(Stdio::null());
        self.env.apply(&mut command);
        common::scrub_provider_env(&mut command);
        command.envs(environment.iter().copied());
        Ok(command.output()?)
    }

    fn rekey(
        &self,
        environment: &[(&str, &str)],
    ) -> Result<std::process::Output, TestError> {
        self.run(&["rekey"], environment)
    }
}

struct RefusingProxy {
    url: String,
    contacts: Arc<AtomicUsize>,
}

impl RefusingProxy {
    fn start() -> Result<Self, TestError> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}", listener.local_addr()?);
        let contacts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&contacts);
        std::thread::spawn(move || {
            for connection in listener.incoming() {
                counter.fetch_add(1, Ordering::SeqCst);
                drop(connection);
            }
        });
        Ok(Self { url, contacts })
    }
}

#[test]
fn a_rekeyed_items_old_id_resolves_to_its_new_path() -> Result<(), TestError> {
    let repo = Repo::committed()?;
    let rekeyed = repo.rekey(&[])?;
    assert_eq!(
        rekeyed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&rekeyed.stderr)
    );

    let resolved = repo.run(&["resolve", "0042"], &[])?;

    assert_eq!(resolved.status.code(), Some(0));
    assert!(String::from_utf8(resolved.stdout)?
        .trim_end()
        .ends_with("meta/work/ACC-0042-search.md"));
    Ok(())
}

#[test]
fn a_numeric_rekey_reaches_no_network() -> Result<(), TestError> {
    let repo = Repo::committed()?;
    let proxy = RefusingProxy::start()?;

    let output = repo.rekey(&[
        ("ACCELERATOR_LINEAR_TOKEN", "fixture-token"),
        ("HTTPS_PROXY", &proxy.url),
        ("https_proxy", &proxy.url),
        ("ALL_PROXY", &proxy.url),
        ("NO_PROXY", ""),
        ("no_proxy", ""),
    ])?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(proxy.contacts.load(Ordering::SeqCst), 0);
    Ok(())
}

#[test]
fn accelerator_force_one_applies_over_a_dirty_working_copy(
) -> Result<(), TestError> {
    let repo = Repo::committed()?;
    repo.write(NOTE, "Edited: work-item:0042.\n")?;

    let output = repo.rekey(&[("ACCELERATOR_FORCE", "1")])?;

    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("ACCELERATOR_FORCE is set"), "{stderr}");
    assert_eq!(
        fs::read_to_string(repo.root().join(NOTE))?,
        "Edited: work-item:ACC-0042.\n"
    );
    Ok(())
}

#[test]
fn accelerator_force_zero_does_not_force() -> Result<(), TestError> {
    let repo = Repo::committed()?;
    repo.write(NOTE, "Edited.\n")?;

    let output = repo.rekey(&[("ACCELERATOR_FORCE", "0")])?;

    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(4), "{stderr}");
    assert!(stderr.contains("E_REKEY_DIRTY_WORKING_COPY"), "{stderr}");
    Ok(())
}

#[test]
fn the_deprecated_migrate_force_does_not_override_rekey(
) -> Result<(), TestError> {
    let repo = Repo::committed()?;
    repo.write(NOTE, "Edited.\n")?;

    let output = repo.rekey(&[("ACCELERATOR_MIGRATE_FORCE", "1")])?;

    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(4), "{stderr}");
    assert!(stderr.contains("E_REKEY_DIRTY_WORKING_COPY"), "{stderr}");
    Ok(())
}

#[test]
fn an_upper_case_md_item_is_listed_and_re_keyed_like_any_other(
) -> Result<(), TestError> {
    let repo = Repo::committed_with(&[(
        "meta/work/0001-x.MD",
        "---\nid: \"0001\"\ntitle: \"Upper\"\n---\n\n# 0001: Upper\n",
    )])?;

    let rekeyed = repo.rekey(&[])?;

    assert_eq!(
        rekeyed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&rekeyed.stderr)
    );
    let mut names: Vec<String> = fs::read_dir(repo.root().join("meta/work"))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    assert_eq!(names, vec!["ACC-0001-x.md", "ACC-0042-search.md"]);
    Ok(())
}

#[test]
#[allow(clippy::literal_string_with_formatting_args)]
fn rekey_refuses_a_malformed_pattern() -> Result<(), TestError> {
    let repo = Repo::committed()?;
    repo.write(
        ".accelerator/config.md",
        "---\nwork:\n  id_pattern: \"{number:4d}\"\n---\n",
    )?;

    let output = repo.rekey(&[])?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "E_WORK_ID_PATTERN_INVALID: work.id_pattern '{number:4d}':"
        ),
        "{stderr}"
    );
    assert!(repo.root().join("meta/work/0042-search.md").exists());
    assert!(!repo.root().join(".accelerator/state/rekey").exists());
    Ok(())
}

fn literal_item(id: &str, title: &str) -> String {
    format!("---\nid: \"{id}\"\ntitle: \"{title}\"\n---\n\n# {id}: {title}\n")
}

fn rekeyed_twice(repo: &Repo) -> Result<(), TestError> {
    let first = repo.rekey(&[])?;
    assert_eq!(
        first.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    repo.commit("rekeyed")?;
    let before = repo.work_items()?;
    let second = repo.rekey(&[])?;
    assert_eq!(
        second.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(repo.work_items()?, before);
    Ok(())
}

#[test]
#[allow(clippy::literal_string_with_formatting_args)]
fn rekey_moves_literal_text_items_to_a_new_pattern() -> Result<(), TestError> {
    let x = literal_item("v001", "X");
    let y = literal_item("v002", "Y");
    let repo = Repo::committed_as(&[
        (
            ".accelerator/config.md",
            "---\nwork:\n  id_pattern: \"{key}-{number:04d}\"\n  \
             key: PP\n---\n",
        ),
        ("meta/work/v001-x.md", &x),
        ("meta/work/v002-y.md", &y),
    ])?;

    rekeyed_twice(&repo)?;

    assert_eq!(repo.work_items()?, vec!["PP-0001-x.md", "PP-0002-y.md"]);
    Ok(())
}

#[test]
#[allow(clippy::literal_string_with_formatting_args)]
fn rekey_adds_literal_text_then_leaves_it() -> Result<(), TestError> {
    let x = literal_item("001", "X");
    let repo = Repo::committed_as(&[
        (
            ".accelerator/config.md",
            "---\nwork:\n  id_pattern: \"v{number:03d}\"\n---\n",
        ),
        ("meta/work/001-x.md", &x),
    ])?;

    rekeyed_twice(&repo)?;

    assert_eq!(repo.work_items()?, vec!["v001-x.md"]);
    Ok(())
}

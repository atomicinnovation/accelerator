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
        repo.write(".accelerator/config.md", CONFIG)?;
        repo.write("meta/work/0042-search.md", ITEM)?;
        repo.write(NOTE, "See work-item:0042.\n")?;
        repo.env.git(&["init", "--quiet"], repo.root())?;
        repo.env.git(&["add", "-A"], repo.root())?;
        repo.env
            .git(&["commit", "--quiet", "-m", "init"], repo.root())?;
        Ok(repo)
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

//! An insecure `config.local.md` over the real store: the store never reads it
//! and refuses writes to it, and composition screens it out of every read.
#![cfg(unix)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::path::PathBuf;

use config::{
    ConfigAccess, ConfigError, Key, Level, Node, PersonalFile, ReadConfigLevel,
    ReadContent, Resolved, Scalar, Value, WriteConfigLevel,
};
use config_adapters::{compose, FileConfigStore, LegacyPolicy};
use tempfile::TempDir;

struct Project {
    root: TempDir,
}

impl Project {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("cfg-personal-file-")
            .tempdir()
            .expect("a scratch project");
        fs::create_dir_all(root.path().join(".git")).expect("mark the root");
        fs::create_dir_all(root.path().join(".accelerator"))
            .expect("create .accelerator");
        fs::write(
            root.path().join(".accelerator/config.md"),
            "---\npaths:\n  work: team-work\njira:\n  site: team\n---\nteam body\n",
        )
        .expect("write the team config");
        Self { root }
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    fn personal(&self) -> PathBuf {
        self.root().join(".accelerator/config.local.md")
    }

    fn with_personal(self, mode: u32) -> Self {
        fs::write(
            self.personal(),
            "---\npaths:\n  work: personal-work\n---\npersonal body\n",
        )
        .expect("write the personal config");
        self.chmod(mode);
        self
    }

    fn chmod(&self, mode: u32) {
        fs::set_permissions(self.personal(), fs::Permissions::from_mode(mode))
            .expect("chmod the personal config");
    }

    fn with_symlinked_personal(self) -> Self {
        let target = self.root().join("real.local.md");
        fs::write(&target, "---\npaths:\n  work: personal-work\n---\n")
            .expect("write the link target");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600))
            .expect("chmod the link target");
        std::os::unix::fs::symlink(&target, self.personal())
            .expect("symlink the personal config");
        self
    }

    fn store(&self) -> FileConfigStore {
        FileConfigStore::at(self.root())
    }
}

fn key(name: &str) -> Key {
    Key::parse(name).expect("a valid key")
}

fn text(value: &str) -> Resolved {
    Resolved::Found(Value::Scalar(Scalar::String(value.to_owned())))
}

#[test]
fn an_insecure_personal_read_is_its_own_refusal() {
    for project in [
        Project::new().with_personal(0o644),
        Project::new().with_symlinked_personal(),
    ] {
        let error = project
            .store()
            .read(Level::Personal)
            .expect_err("an insecure personal file is never read");
        assert!(
            matches!(error, ConfigError::InsecurePersonalFile { .. }),
            "{error:?}"
        );
        assert!(error.is_refusal());
    }
}

#[test]
fn the_probe_answers_without_reading_the_file() {
    assert_eq!(
        Project::new().store().probe_personal_file().unwrap(),
        PersonalFile::Absent
    );
    assert_eq!(
        Project::new()
            .with_personal(0o600)
            .store()
            .probe_personal_file()
            .unwrap(),
        PersonalFile::Readable
    );
    let project = Project::new().with_personal(0o640);
    assert_eq!(
        project.store().probe_personal_file().unwrap(),
        PersonalFile::Ignored {
            path: project.personal(),
            mode: 0o640
        }
    );
    assert!(matches!(
        Project::new()
            .with_symlinked_personal()
            .store()
            .probe_personal_file()
            .unwrap(),
        PersonalFile::Ignored { .. }
    ));
}

#[test]
fn a_write_to_an_insecure_personal_file_is_refused_and_leaves_it_untouched() {
    let project = Project::new().with_personal(0o644);
    let before = fs::read(project.personal()).unwrap();

    let error = project
        .store()
        .write(Level::Personal, &Node::Mapping(config::Mapping::new()))
        .expect_err("the store refuses to write an insecure file");
    assert!(matches!(error, ConfigError::InsecurePersonalFile { .. }));

    let composed = compose(project.root(), LegacyPolicy::Reject).unwrap();
    let error = composed
        .service
        .set(&key("paths.work"), "changed", Level::Personal)
        .expect_err("a composed personal write is refused too");
    assert!(matches!(error, ConfigError::InsecurePersonalFile { .. }));

    assert_eq!(fs::read(project.personal()).unwrap(), before);
    assert_eq!(
        fs::metadata(project.personal())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
}

#[test]
fn composition_screens_an_insecure_personal_file_out_of_every_read() {
    let project = Project::new().with_personal(0o644);
    let composed = compose(project.root(), LegacyPolicy::Reject).unwrap();
    let service: &dyn ConfigAccess = &composed.service;

    assert_eq!(
        service.personal_file(),
        &PersonalFile::Ignored {
            path: project.personal(),
            mode: 0o644
        }
    );
    assert_eq!(composed.personal_file(), service.personal_file());
    assert_eq!(
        service.get(&key("paths.work"), None).unwrap(),
        text("team-work")
    );
    assert_eq!(
        service
            .get(&key("paths.work"), Some(Level::Personal))
            .unwrap(),
        Resolved::Absent
    );
    assert_eq!(
        service
            .effective(&key("paths.work"), Some(Level::Personal))
            .unwrap()
            .source(),
        config::Source::Catalogue
    );
    assert_eq!(
        service
            .effective_nonempty(&key("paths.work"), None)
            .unwrap()
            .rendered(),
        "team-work"
    );
    assert_eq!(composed.screened.read(Level::Personal).unwrap(), None);
    assert_eq!(
        composed.screened.config_body(Level::Personal).unwrap(),
        None
    );
    assert_eq!(
        composed
            .screened
            .config_body(Level::Team)
            .unwrap()
            .as_deref(),
        Some("team body")
    );
}

#[test]
fn a_personal_file_that_turns_insecure_after_composition_fails_closed() {
    let project = Project::new().with_personal(0o600);
    let composed = compose(project.root(), LegacyPolicy::Reject).unwrap();
    project.chmod(0o644);

    let error = composed
        .service
        .get(&key("paths.work"), None)
        .expect_err("a file loosened after the probe is still refused");

    assert!(matches!(error, ConfigError::InsecurePersonalFile { .. }));
}

#[test]
fn writers_are_refused_only_for_an_ignored_file() {
    for (project, refused) in [
        (Project::new(), false),
        (Project::new().with_personal(0o600), false),
        (Project::new().with_personal(0o604), true),
    ] {
        let composed = compose(project.root(), LegacyPolicy::Reject).unwrap();
        let outcome = composed.require_readable_personal_file();
        assert_eq!(
            matches!(outcome, Err(ConfigError::InsecurePersonalFile { .. })),
            refused
        );
        assert_eq!(outcome.is_ok(), !refused);
    }
}

#[test]
fn a_readable_personal_file_still_wins() {
    let project = Project::new().with_personal(0o400);
    let composed = compose(project.root(), LegacyPolicy::Reject).unwrap();

    assert_eq!(
        composed.service.get(&key("paths.work"), None).unwrap(),
        text("personal-work")
    );
    assert_eq!(composed.personal_file(), &PersonalFile::Readable);
}

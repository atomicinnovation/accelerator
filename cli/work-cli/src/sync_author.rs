//! The binary-layer [`LocalAuthor`] the sync engine drives for its two create
//! paths: authoring a local work item from a discovered remote issue, and
//! linking a freshly-created remote id back into an unsynced local draft.
//!
//! This is where the config, the id scheme, and the frontmatter renderer the
//! engine cannot reach live, so the engine stays a pure orchestration over the
//! tracker port and the baseline.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use corpus::AtomicWrite;
use corpus::FilenameTimestampFormat;
use corpus::IdOwnership;
use corpus_adapters::lock::acquire;
use corpus_adapters::lock::LockOptions;
use corpus_adapters::metadata::derive_at;
use corpus_adapters::metadata::VcsBackedRepoFactsProbe;
use corpus_adapters::FileCorpusStore;
use document::Scalar;
use document::Yaml;
use tracker::ExternalId;
use work::create::resolve_author;
use work::create::CreateInputs;
use work::create::TypedLinkage;
use work::identity::holder_of;
use work::identity::linker_of;
use work::sync::PendingPush;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles;
use work_adapters::author::RepositoryIdentityProbe;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::sync::create::AuthoredLocal;
use work_adapters::sync::create::DiscoveredIssue;
use work_adapters::sync::create::LocalAuthor;

use crate::config::resolve_scheme;

/// The default kind, status, and priority an imported remote issue is authored
/// with. A tracker issue carries no work-item taxonomy, so the import picks
/// safe, editable defaults rather than guessing — a human refines them after.
const IMPORTED_KIND: &str = "task";
const IMPORTED_STATUS: &str = "ready";
const IMPORTED_PRIORITY: &str = "medium";
const IMPORTED_PRODUCER: &str = "sync-work-items";

pub struct ConfiguredLocalAuthor<'a> {
    config: &'a dyn ConfigAccess,
    root: PathBuf,
    work_dir: PathBuf,
}

impl<'a> ConfiguredLocalAuthor<'a> {
    #[must_use]
    pub fn new(
        config: &'a dyn ConfigAccess,
        root: PathBuf,
        work_dir: PathBuf,
    ) -> Self {
        Self {
            config,
            root,
            work_dir,
        }
    }
}

impl ConfiguredLocalAuthor<'_> {
    /// The key a pulled issue takes as its `id`, refused when another item
    /// already holds it or links to it.
    fn adoptable(&self, key: &ExternalId) -> Result<String, kernel::Error> {
        let files = FilesystemWorkItemFiles::new(&self.work_dir).files()?;
        let items = identities(&files);
        if let Some(held) = holder_of(key.as_str(), &items) {
            return Err(failed(format!(
                "refusing to adopt '{}': {} already holds it in `{}`",
                key.as_str(),
                held.item.path.display(),
                held.field.frontmatter_key()
            )));
        }
        if let Some(linker) = linker_of(key.as_str(), &items) {
            return Err(failed(format!(
                "refusing to adopt '{}': {} already links to it",
                key.as_str(),
                linker.path.display()
            )));
        }
        Ok(key.as_str().to_owned())
    }

    /// Removes each `created` pending-push marker naming `key`: its issue now
    /// has a local file, so no create can resume from it.
    fn spend_created_markers(&self, key: &ExternalId) {
        let Ok(integration) =
            crate::config::effective_nonempty(self.config, "work.integration")
        else {
            return;
        };
        let Ok(integrations_root) =
            crate::sync::integrations_dir(self.config, &self.root)
        else {
            return;
        };
        let Ok(markers) = work_adapters::sync::pending_push::outstanding(
            &integrations_root,
            &integration,
        ) else {
            return;
        };
        for (path, marker) in markers {
            if let PendingPush::Created { external_id, .. } = marker {
                if external_id.as_str().eq_ignore_ascii_case(key.as_str()) {
                    std::fs::remove_file(&path).ok();
                }
            }
        }
    }
}

/// Splits a projected remote body — a title line, then the description — into
/// `(title, description)`.
fn split_projected(projected: &str) -> (String, String) {
    let mut lines = projected.lines();
    let title = lines.next().unwrap_or_default().to_owned();
    let description = lines.collect::<Vec<_>>().join("\n");
    (title, description)
}

fn failed(message: impl std::fmt::Display) -> kernel::Error {
    kernel::Error::Failed(message.to_string())
}

/// Upsert `external_id` into a work item's frontmatter, preserving every other
/// field and the body verbatim.
pub fn link_external_id(
    path: &Path,
    external_id: &ExternalId,
) -> Result<(), kernel::Error> {
    let content = std::fs::read_to_string(path).map_err(failed)?;
    let mut yaml = document::parse(&content).map_err(failed)?;
    let Yaml::Mapping(mapping) = &mut yaml else {
        return Err(failed("frontmatter is not a mapping"));
    };
    mapping.set(
        "external_id".to_owned(),
        Yaml::Scalar(Scalar::String(external_id.as_str().to_owned())),
    );
    let rendered = document::render(Some(&content), &yaml).map_err(failed)?;
    let store =
        FileCorpusStore::new(path.parent().unwrap_or_else(|| Path::new(".")));
    AtomicWrite::write(&store, path, rendered.as_bytes()).map_err(failed)?;
    Ok(())
}

impl LocalAuthor for ConfiguredLocalAuthor<'_> {
    fn author_from_remote(
        &self,
        issue: &DiscoveredIssue,
    ) -> Result<AuthoredLocal, kernel::Error> {
        let scheme = resolve_scheme(self.config)?;
        let project = scheme.key.clone();
        let (title, description) = split_projected(&issue.issue.body);

        std::fs::create_dir_all(&self.work_dir).map_err(failed)?;
        let lockdir = self.work_dir.join(crate::create::LOCK_FILE_NAME);
        let _guard =
            acquire(&lockdir, LockOptions::default()).map_err(failed)?;

        let id = match scheme.ownership() {
            IdOwnership::Local => crate::create::allocate_id(
                &scheme,
                &self.work_dir,
                project.as_deref(),
            )
            .map_err(failed)?,
            IdOwnership::Tracker => self.adoptable(&issue.external_id)?,
        };
        let metadata = derive_at(
            &self.root,
            FilenameTimestampFormat::DateTimeUnderscored,
            &VcsBackedRepoFactsProbe,
        )
        .map_err(failed)?;
        let author =
            resolve_author(None, &RepositoryIdentityProbe::new(&self.root))
                .map_err(failed)?;

        let inputs = CreateInputs {
            id: &id,
            title: &title,
            kind: IMPORTED_KIND,
            priority: IMPORTED_PRIORITY,
            status: IMPORTED_STATUS,
            linkage: TypedLinkage {
                parent: None,
                blocks: &[],
                blocked_by: &[],
                derived_from: &[],
                relates_to: &[],
                source: None,
            },
            tags: &[],
            author: &author,
            producer: IMPORTED_PRODUCER,
            date: &metadata.datetime_utc,
            external_id: Some(issue.external_id.as_str()),
        };
        let frontmatter_block =
            crate::create::render_frontmatter(&inputs).map_err(failed)?;
        let content = format!("{frontmatter_block}\n{description}\n");

        let slug = crate::create::slugify(&title);
        let target = self.work_dir.join(format!("{id}-{slug}.md"));
        let store = FileCorpusStore::new(&self.work_dir);
        work_adapters::sync::create::exclusive_write(
            &store,
            &target,
            content.as_bytes(),
        )?;
        if scheme.ownership() == IdOwnership::Tracker {
            self.spend_created_markers(&issue.external_id);
        }
        Ok(AuthoredLocal { id, path: target })
    }

    fn link_external_id(
        &self,
        path: &Path,
        external_id: &ExternalId,
    ) -> Result<(), kernel::Error> {
        link_external_id(path, external_id)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;

    use ::config::ConfigAccess;
    use ::config::ConfigError;
    use ::config::Key;
    use ::config::Level;
    use ::config::Resolved;
    use ::config::Scalar;
    use ::config::Value;
    use tracker::ExternalId;
    use tracker::RemoteIssue;
    use tracker::RemoteTimestamp;
    use work::sync::PendingPush;
    use work::sync::RequestFingerprint;
    use work::work_item_files::identity_of;
    use work::work_item_files::WorkItemFile;
    use work_adapters::sync::create::AuthoredLocal;
    use work_adapters::sync::create::DiscoveredIssue;
    use work_adapters::sync::create::LocalAuthor;
    use work_adapters::sync::pending_push;

    use super::ConfiguredLocalAuthor;

    struct FakeConfig(HashMap<String, String>);

    impl ConfigAccess for FakeConfig {
        fn get(
            &self,
            key: &Key,
            _level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            Ok(self
                .0
                .get(&key.to_string())
                .map_or(Resolved::Absent, |value| {
                    Resolved::Found(Value::Scalar(Scalar::String(
                        value.clone(),
                    )))
                }))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            unreachable!("authoring never writes config")
        }
    }

    struct TrackerRepo {
        dir: tempfile::TempDir,
        config: FakeConfig,
    }

    impl TrackerRepo {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("tempdir");
            let git = |args: &[&str]| {
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(dir.path())
                    .status()
                    .expect("git invocation");
            };
            git(&["init", "-q"]);
            git(&["config", "user.name", "Test User"]);
            git(&["config", "user.email", "test@example.com"]);
            let config = FakeConfig(
                [
                    ("work.id_pattern", "{tracker}"),
                    ("work.integration", "linear"),
                    ("linear.team_key", "ENG"),
                    ("paths.integrations", "integrations"),
                ]
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
            );
            let repo = Self { dir, config };
            std::fs::create_dir_all(repo.work_dir()).expect("work dir");
            repo
        }

        fn work_dir(&self) -> PathBuf {
            self.dir.path().join("meta/work")
        }

        fn integrations(&self) -> PathBuf {
            self.dir.path().join("integrations")
        }

        fn author(&self) -> ConfiguredLocalAuthor<'_> {
            ConfiguredLocalAuthor::new(
                &self.config,
                self.dir.path().to_path_buf(),
                self.work_dir(),
            )
        }

        fn pull(&self, key: &str) -> Result<AuthoredLocal, kernel::Error> {
            let key = ExternalId::new(key.to_owned());
            self.author().author_from_remote(&DiscoveredIssue {
                external_id: key.clone(),
                issue: RemoteIssue {
                    key,
                    updated: RemoteTimestamp::Reported(
                        "2026-06-01T00:00:00Z".to_owned(),
                    ),
                    body: "A pulled title\nWhat it is.".to_owned(),
                },
            })
        }

        fn write_item(&self, name: &str, frontmatter: &str) -> PathBuf {
            let path = self.work_dir().join(name);
            std::fs::write(&path, format!("---\n{frontmatter}---\n\n# T\n"))
                .expect("write item");
            path
        }

        fn write_marker(&self, slug: &str, marker: &PendingPush) -> PathBuf {
            let path = pending_push::path(&self.integrations(), "linear", slug);
            std::fs::create_dir_all(path.parent().expect("marker dir"))
                .expect("marker dir");
            std::fs::write(&path, pending_push::render(marker))
                .expect("write marker");
            path
        }

        fn filenames(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(self.work_dir())
                .expect("list")
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension().is_some_and(|extension| extension == "md")
                })
                .filter_map(|path| {
                    path.file_name()?.to_str().map(str::to_owned)
                })
                .collect();
            names.sort();
            names
        }
    }

    fn identity_at(path: &Path) -> work::identity::ItemIdentity {
        identity_of(&WorkItemFile {
            path: path.to_path_buf(),
            content: std::fs::read_to_string(path).expect("read"),
        })
        .expect("an identity")
    }

    fn fingerprint(title: &str) -> RequestFingerprint {
        RequestFingerprint {
            title: title.to_owned(),
            digest: pending_push::request_digest(title, "Body\n", "task"),
            attempted_at: 1_700_000_000,
            failure: None,
        }
    }

    #[test]
    fn a_pulled_issue_takes_its_external_id_as_id_under_tracker() {
        let repo = TrackerRepo::new();

        let authored = repo.pull("ENG-42").expect("the pull authors a file");

        assert_eq!(authored.id, "ENG-42");
        assert_eq!(
            authored.path,
            repo.work_dir().join("ENG-42-a-pulled-title.md")
        );
        let identity = identity_at(&authored.path);
        assert_eq!(identity.id, "ENG-42");
        assert_eq!(identity.external_id.as_deref(), Some("ENG-42"));
    }

    #[test]
    fn a_pull_consumes_no_local_number() {
        let repo = TrackerRepo::new();
        repo.write_item("0007-legacy.md", "id: \"0007\"\n");

        repo.pull("ENG-42").expect("the pull authors a file");

        assert_eq!(
            repo.filenames(),
            vec!["0007-legacy.md", "ENG-42-a-pulled-title.md"]
        );
    }

    #[test]
    fn a_pulled_issue_outside_the_creation_home_keeps_its_own_key() {
        let repo = TrackerRepo::new();

        let authored = repo.pull("OPS-7").expect("the pull authors a file");

        assert_eq!(authored.id, "OPS-7");
        assert_eq!(identity_at(&authored.path).id, "OPS-7");
    }

    #[test]
    fn a_pulled_key_already_held_as_an_id_or_alias_is_refused() {
        let repo = TrackerRepo::new();
        repo.write_item(
            "ENG-9-other.md",
            "id: \"ENG-9\"\naliases: [\"ENG-42\"]\n",
        );

        let refused = repo.pull("eng-42");

        assert!(refused.is_err(), "{refused:?}");
        assert_eq!(repo.filenames(), vec!["ENG-9-other.md"]);
    }

    #[test]
    fn a_pulled_key_already_linked_by_another_item_is_refused() {
        let repo = TrackerRepo::new();
        repo.write_item(
            "0230-legacy.md",
            "id: \"0230\"\nexternal_id: \"ENG-42\"\n",
        );

        let refused = repo.pull("ENG-42");

        assert!(refused.is_err(), "{refused:?}");
        assert_eq!(repo.filenames(), vec!["0230-legacy.md"]);
    }

    #[test]
    fn a_pulled_issue_whose_key_matches_a_created_marker_removes_the_marker() {
        let repo = TrackerRepo::new();
        let adopted = repo.write_marker(
            "a-pulled-title",
            &PendingPush::Created {
                request: fingerprint("A pulled title"),
                external_id: ExternalId::new("ENG-42".to_owned()),
            },
        );
        let other = repo.write_marker(
            "another",
            &PendingPush::Created {
                request: fingerprint("Another"),
                external_id: ExternalId::new("ENG-43".to_owned()),
            },
        );
        let attempted = repo.write_marker(
            "attempted",
            &PendingPush::Attempted {
                request: fingerprint("Attempted"),
            },
        );

        repo.pull("ENG-42").expect("the pull authors a file");

        assert!(!adopted.exists(), "the adopted key's marker is spent");
        assert!(other.exists());
        assert!(attempted.exists());
    }

    #[test]
    fn a_legacy_unsynced_item_pushed_under_tracker_keeps_its_id() {
        let repo = TrackerRepo::new();
        let path = repo.write_item("0001-legacy.md", "id: \"0001\"\n");

        repo.author()
            .link_external_id(&path, &ExternalId::new("ENG-5".to_owned()))
            .expect("the link lands");

        let identity = identity_at(&path);
        assert_eq!(identity.id, "0001");
        assert_eq!(identity.external_id.as_deref(), Some("ENG-5"));
        assert_eq!(repo.filenames(), vec!["0001-legacy.md"]);
    }
}

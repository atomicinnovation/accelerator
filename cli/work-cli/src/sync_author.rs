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
use work_adapters::author::VcsBackedIdentityProbe;
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
        let project = scheme.default_project_code.clone();
        let (title, description) = split_projected(&issue.issue.body);

        std::fs::create_dir_all(&self.work_dir).map_err(failed)?;
        let lockdir = self.work_dir.join(crate::create::LOCK_FILE_NAME);
        let _guard =
            acquire(&lockdir, LockOptions::default()).map_err(failed)?;

        let id = crate::create::allocate_id(
            &scheme,
            &self.work_dir,
            project.as_deref(),
        )
        .map_err(failed)?;
        let metadata = derive_at(
            &self.root,
            FilenameTimestampFormat::DateTimeUnderscored,
            &VcsBackedRepoFactsProbe,
        )
        .map_err(failed)?;
        let author =
            resolve_author(None, &VcsBackedIdentityProbe).map_err(failed)?;

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

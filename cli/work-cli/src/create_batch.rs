//! `accelerator work create-batch`: creates a manifest's items parents
//! first, each through the same strategy `work create` uses, linking every
//! child's `parent` to the ID its parent took.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use ::config::ReadTemplate;
use corpus_adapters::FileCorpusStore;
use work::create_batch::BatchKeyword;
use work::draft_id::DraftId;
use work::hierarchy::parents_first;
use work::hierarchy::BatchOrder;
use work::hierarchy::HierarchyNode;
use work::identity::resolve_identity;
use work::identity::IdentityResolution;
use work::identity::ItemIdentity;
use work::sync::PushOutcome;
use work::work_item_files::identities;
use work::work_item_files::identity_of;
use work::work_item_files::WorkItemFile;
use work::work_item_files::WorkItemFiles as _;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::promotion::Detail;
use work_adapters::sync::pending_push;

use crate::batch_journal::BatchJournal;
use crate::batch_journal::JournalEntry;
use crate::batch_manifest::Authorship;
use crate::batch_manifest::ManifestEntry;
use crate::batch_manifest::ParentLink;
use crate::cli::CreateBatchArgs;
use crate::create::create_item;
use crate::create::CreateFailure;
use crate::create::CreationOutcome;
use crate::create::CreationStore;
use crate::create::Seams;
use crate::exit_codes;
use crate::promotion_report::detail_line;
use crate::sync::severity;
use crate::tracker_registry::TrackerRegistry;

pub const E_BATCH_CYCLE: &str = "E_BATCH_CYCLE";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItem {
    pub reference: String,
    /// A file that exists, or `None` when an issue was created but no
    /// local file carries it.
    pub path: Option<PathBuf>,
    pub keyword: BatchKeyword,
    pub key: Option<String>,
    /// What the entry's children name as their `parent`.
    pub resulting_id: Option<String>,
    pub cause: Option<String>,
    pub details: Vec<Detail>,
}

impl BatchItem {
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}",
            self.reference,
            self.path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            self.keyword.keyword(),
            self.key.as_deref().unwrap_or_default()
        )
    }
}

#[derive(Debug)]
pub struct BatchReport {
    pub lines: Vec<String>,
    /// What stderr tells the user, one line per item that has something to
    /// say.
    pub causes: Vec<String>,
    pub code: u8,
}

#[derive(Debug)]
pub enum BatchOutcome {
    Reported(BatchReport),
    /// The manifest was refused; nothing was written.
    Invalid(Vec<String>),
    /// An entry could not be created; the report covers the ones before it.
    Failed {
        report: BatchReport,
        message: String,
    },
}

#[must_use]
pub fn report(items: &[BatchItem]) -> BatchReport {
    let mut lines: Vec<String> = items.iter().map(BatchItem::line).collect();
    lines.extend(items.iter().flat_map(|item| {
        item.details
            .iter()
            .map(|detail| detail_line(&item.reference, detail))
    }));
    BatchReport {
        lines,
        causes: items
            .iter()
            .filter_map(|item| {
                item.cause
                    .as_ref()
                    .map(|cause| format!("{}: {cause}", item.reference))
            })
            .collect(),
        code: exit_code(items.iter().map(|item| item.keyword)),
    }
}

/// The code of the item whose code takes precedence, 0 when every item's is.
#[must_use]
pub fn exit_code(keywords: impl IntoIterator<Item = BatchKeyword>) -> u8 {
    keywords
        .into_iter()
        .map(BatchKeyword::exit_code)
        .max_by_key(|code| severity(*code))
        .unwrap_or(exit_codes::CLEAN)
}

#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateBatchArgs,
    registry: &dyn TrackerRegistry,
) -> BatchOutcome {
    let store_at = |root: &Path| -> Box<dyn CreationStore> {
        Box::new(FileCorpusStore::new(root))
    };
    run_with(
        start,
        config,
        templates,
        args,
        &mut Seams {
            registry,
            store_at: &store_at,
            draws: &mut RandomSuffixDraws,
        },
        now_epoch(),
    )
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[must_use]
pub fn run_with(
    start: &Path,
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    args: &CreateBatchArgs,
    seams: &mut Seams<'_>,
    now: u64,
) -> BatchOutcome {
    let entries = match crate::batch_manifest::read(&args.manifest) {
        Ok(entries) => entries,
        Err(problems) => return BatchOutcome::Invalid(problems),
    };
    let order = match creation_order(&entries) {
        Ok(order) => order,
        Err(problem) => return BatchOutcome::Invalid(vec![problem]),
    };
    let root = config_adapters::FileConfigStore::discover_root(start);
    let workspace = match BatchWorkspace::open(config, &root, args.push, now) {
        Ok(workspace) => workspace,
        Err(message) => {
            return BatchOutcome::Failed {
                report: report(&[]),
                message,
            };
        }
    };
    let mut batch = Batch {
        start,
        config,
        templates,
        authorship: Authorship {
            author: args.author.as_deref(),
            producer: &args.producer,
            push: args.push,
        },
        workspace,
        fingerprint: crate::batch_manifest::fingerprint(&entries),
        now,
        created: Vec::new(),
    };
    for entry in order {
        if let Err(message) = batch.create(entry, seams) {
            return BatchOutcome::Failed {
                report: report(&batch.created),
                message: format!("{}: {message}", entry.reference),
            };
        }
    }
    BatchOutcome::Reported(report(&batch.created))
}

/// The entries in the order they are created: every in-batch parent before
/// its children.
fn creation_order(
    entries: &[ManifestEntry],
) -> Result<Vec<&ManifestEntry>, String> {
    let nodes: Vec<HierarchyNode<'_>> = entries
        .iter()
        .map(|entry| HierarchyNode {
            key: &entry.reference,
            parent: match &entry.parent {
                Some(ParentLink::InBatch(parent)) => Some(parent),
                Some(ParentLink::Existing(_)) | None => None,
            },
        })
        .collect();
    let by_reference: BTreeMap<&str, &ManifestEntry> = entries
        .iter()
        .map(|entry| (entry.reference.as_str(), entry))
        .collect();
    match parents_first(&nodes) {
        BatchOrder::ParentsFirst(order) => Ok(order
            .into_iter()
            .filter_map(|reference| by_reference.get(reference).copied())
            .collect()),
        BatchOrder::Cycle(members) => Err(format!(
            "{E_BATCH_CYCLE}: the parents of {} form a cycle; nothing was \
             created",
            members.join(", ")
        )),
    }
}

/// Where the batch reads the corpus from and journals what it creates.
struct BatchWorkspace {
    work_dir: PathBuf,
    journal: Option<BatchJournal>,
}

impl BatchWorkspace {
    fn open(
        config: &dyn ConfigAccess,
        root: &Path,
        push: bool,
        now: u64,
    ) -> Result<Self, String> {
        let work_dir = crate::config::resolve_work_dir(config, root)
            .map_err(|error| error.to_string())?;
        let journal = push
            .then(|| {
                BatchJournal::open(&root.join(crate::sync::STATE_DIR), now)
            })
            .transpose()?;
        Ok(Self { work_dir, journal })
    }

    fn items(&self) -> Result<Vec<ItemIdentity>, String> {
        FilesystemWorkItemFiles::new(&self.work_dir)
            .files()
            .map(|files| identities(&files))
            .map_err(|error| error.to_string())
    }
}

struct Batch<'a> {
    start: &'a Path,
    config: &'a dyn ConfigAccess,
    templates: &'a dyn ReadTemplate,
    authorship: Authorship<'a>,
    workspace: BatchWorkspace,
    fingerprint: String,
    now: u64,
    created: Vec<BatchItem>,
}

impl Batch<'_> {
    fn create(
        &mut self,
        entry: &ManifestEntry,
        seams: &mut Seams<'_>,
    ) -> Result<(), String> {
        let parent = self.parent_of(entry);
        let digest = pending_push::request_digest(
            &entry.title,
            entry.body.as_deref().unwrap_or_default(),
            &entry.kind,
        );
        if let Some(item) = self.journalled(entry, &digest)? {
            self.created.push(item);
            return Ok(());
        }
        let args = entry.create_args(parent.clone(), &self.authorship);
        let mut journal = self.workspace.journal.as_mut();
        let mut journal_failure = None;
        let result = create_item(
            self.start,
            self.config,
            self.templates,
            &args,
            seams,
            &mut |draft: &DraftId| {
                if let Some(journal) = journal.as_deref_mut() {
                    let recorded = journal.record(JournalEntry {
                        content_digest: digest.clone(),
                        batch: self.fingerprint.clone(),
                        reference: entry.reference.clone(),
                        title: entry.title.clone(),
                        id: Some(draft.as_str().to_owned()),
                        key: None,
                        outcome: None,
                        recorded_at: self.now,
                    });
                    journal_failure = recorded.err();
                }
            },
        );
        let mut item = match result {
            Ok(outcome) => created(&entry.reference, outcome),
            Err(CreateFailure::Pending {
                message,
                existing_draft,
            }) => BatchItem {
                reference: entry.reference.clone(),
                resulting_id: existing_draft.as_deref().and_then(id_at),
                path: existing_draft,
                keyword: BatchKeyword::Pending,
                key: None,
                cause: Some(message),
                details: Vec::new(),
            },
            Err(CreateFailure::Failed(message)) => return Err(message),
        };
        if let Some(journal) = self.workspace.journal.as_mut() {
            journal.record(JournalEntry {
                content_digest: digest,
                batch: self.fingerprint.clone(),
                reference: entry.reference.clone(),
                title: entry.title.clone(),
                id: item.path.as_deref().and_then(id_at),
                key: item.key.clone(),
                outcome: Some(item.keyword.keyword().to_owned()),
                recorded_at: self.now,
            })?;
        }
        if let Some(failure) = journal_failure {
            return Err(failure);
        }
        if let (Some(ParentLink::InBatch(parent_ref)), None) =
            (&entry.parent, &parent)
        {
            let unlinked = format!(
                "its parent {parent_ref} took no ID, so it was created \
                 without a parent"
            );
            item.cause = Some(item.cause.map_or_else(
                || unlinked.clone(),
                |cause| format!("{cause}; {unlinked}"),
            ));
        }
        self.created.push(item);
        Ok(())
    }

    /// The `parent` the entry's item is created with: an in-batch parent's
    /// resulting ID, or the typed reference the manifest gave.
    fn parent_of(&self, entry: &ManifestEntry) -> Option<String> {
        match entry.parent.as_ref()? {
            ParentLink::Existing(reference) => Some(reference.clone()),
            ParentLink::InBatch(reference) => self
                .created
                .iter()
                .find(|item| &item.reference == reference)
                .and_then(|item| item.resulting_id.as_deref())
                .map(|id| format!("work-item:{id}")),
        }
    }

    /// What an earlier run of this batch made of the entry, read back from
    /// the corpus, or `None` when this run must create it.
    fn journalled(
        &mut self,
        entry: &ManifestEntry,
        digest: &str,
    ) -> Result<Option<BatchItem>, String> {
        let claimed = match self.workspace.journal.as_mut() {
            Some(journal) => journal.claim(
                digest,
                &self.fingerprint,
                &entry.reference,
                &entry.title,
            )?,
            None => None,
        };
        let Some(recorded) = claimed else {
            return Ok(None);
        };
        let items = self.workspace.items()?;
        let found = |token: &str| match resolve_identity(token, &items) {
            IdentityResolution::Unique(item) => Some(item.clone()),
            IdentityResolution::Conflicting(_)
            | IdentityResolution::Unmatched => None,
        };
        let holder = recorded
            .id
            .as_deref()
            .and_then(found)
            .or_else(|| recorded.key.as_deref().and_then(found));
        let item =
            |keyword, path, key: Option<String>, resulting_id| BatchItem {
                reference: entry.reference.clone(),
                path,
                keyword,
                key,
                resulting_id,
                cause: None,
                details: Vec::new(),
            };
        let reported = match (holder, recorded.key.clone()) {
            (Some(held), _) if DraftId::parse(&held.id).is_some() => item(
                BatchKeyword::Pending,
                Some(held.path),
                None,
                Some(held.id),
            ),
            (Some(held), _) => item(
                BatchKeyword::Pushed(PushOutcome::WriteOnce),
                Some(held.path),
                held.external_id.or_else(|| Some(held.id.clone())),
                Some(held.id),
            ),
            (None, Some(key)) => item(
                BatchKeyword::Pushed(PushOutcome::CreatedUnwritten),
                None,
                Some(key.clone()),
                Some(key),
            ),
            (None, None) => {
                if let Some(journal) = self.workspace.journal.as_mut() {
                    journal.forget(&recorded)?;
                }
                return Ok(None);
            }
        };
        Ok(Some(reported))
    }
}

/// The item a create left, and the ID its children link to: the tracker
/// key once an issue exists that the item is still to take, otherwise the
/// ID of the file it wrote.
fn created(reference: &str, outcome: CreationOutcome) -> BatchItem {
    let CreationOutcome { path, push } = outcome;
    let id_on_disk = path.as_deref().and_then(id_at);
    let Some(report) = push else {
        return BatchItem {
            reference: reference.to_owned(),
            path,
            keyword: BatchKeyword::Declined,
            key: None,
            resulting_id: id_on_disk,
            cause: None,
            details: Vec::new(),
        };
    };
    let resulting_id = match report.outcome {
        PushOutcome::CreatedUnwritten | PushOutcome::RetirementIncomplete => {
            report.external_id.clone().or(id_on_disk)
        }
        _ => id_on_disk.or_else(|| report.external_id.clone()),
    };
    BatchItem {
        reference: reference.to_owned(),
        path,
        keyword: BatchKeyword::Pushed(report.outcome),
        key: report.external_id,
        resulting_id,
        cause: report.cause,
        details: report.details,
    }
}

fn id_at(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    identity_of(&WorkItemFile {
        path: path.to_path_buf(),
        content,
    })
    .map(|identity| identity.id)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::rc::Rc;

    use tracker::TrackerError;
    use tracker_test_support::Call;
    use tracker_test_support::RecordingTracker;

    use super::*;
    use crate::test_support::FakeConfig;
    use crate::test_support::Faults;
    use crate::test_support::PluginWorkItemTemplate;
    use crate::test_support::StubRegistry;

    const NOW: u64 = 1_800_000_000;

    struct BatchRepo {
        root: tempfile::TempDir,
        config: FakeConfig,
    }

    type Failing = Option<(fn(&Path) -> bool, usize)>;

    impl BatchRepo {
        fn tracker_owned() -> Self {
            Self::configured(&[
                ("work.id_pattern", "{tracker}"),
                ("work.integration", "linear"),
                ("paths.integrations", "integrations"),
            ])
        }

        fn numeric() -> Self {
            Self::configured(&[
                ("work.integration", "jira"),
                ("paths.integrations", "integrations"),
            ])
        }

        fn configured(pairs: &[(&str, &str)]) -> Self {
            let root = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
            std::fs::create_dir_all(root.path().join("meta/work"))
                .expect("work dir");
            let config = FakeConfig(
                pairs
                    .iter()
                    .map(|(key, value)| {
                        ((*key).to_owned(), (*value).to_owned())
                    })
                    .collect(),
            );
            Self { root, config }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.path().join(relative)
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.path(relative);
            std::fs::create_dir_all(path.parent().expect("dir")).expect("dir");
            std::fs::write(path, content).expect("write");
        }

        fn run(
            &self,
            manifest: &str,
            push: bool,
            tracker: &Rc<RecordingTracker>,
        ) -> BatchOutcome {
            self.run_failing(manifest, push, tracker, None)
        }

        fn run_failing(
            &self,
            manifest: &str,
            push: bool,
            tracker: &Rc<RecordingTracker>,
            failing: Failing,
        ) -> BatchOutcome {
            self.write("manifest.json", manifest);
            let (applies, left) = failing.unwrap_or((|_| false, 0));
            let store_at = move |root: &Path| -> Box<dyn CreationStore> {
                Box::new(Faults {
                    inner: FileCorpusStore::new(root),
                    applies,
                    left: std::cell::Cell::new(left),
                })
            };
            run_with(
                self.root.path(),
                &self.config,
                &PluginWorkItemTemplate,
                &CreateBatchArgs {
                    manifest: self.path("manifest.json"),
                    push,
                    author: Some("A Tester".to_owned()),
                    producer: "extract-work-items".to_owned(),
                },
                &mut Seams {
                    registry: &StubRegistry(Rc::clone(tracker)),
                    store_at: &store_at,
                    draws: &mut RandomSuffixDraws,
                },
                NOW,
            )
        }

        fn files_under(&self, relative: &str) -> Vec<PathBuf> {
            let mut found: Vec<PathBuf> =
                std::fs::read_dir(self.path(relative))
                    .map(|entries| {
                        entries
                            .flatten()
                            .map(|entry| entry.path())
                            .filter(|path| {
                                path.extension().is_some_and(|ext| ext == "md")
                            })
                            .collect()
                    })
                    .unwrap_or_default();
            found.sort();
            found
        }
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).expect("read")
    }

    const EPIC_AND_TWO_STORIES: &str = r#"[
        {"ref": "story-a", "title": "Story A", "kind": "story",
         "priority": "medium", "parent": {"ref": "epic"}},
        {"ref": "epic", "title": "The epic", "kind": "epic",
         "priority": "high"},
        {"ref": "story-b", "title": "Story B", "kind": "story",
         "priority": "low", "parent": {"ref": "epic"}}
    ]"#;

    fn reported(outcome: BatchOutcome) -> BatchReport {
        match outcome {
            BatchOutcome::Reported(report) => report,
            BatchOutcome::Invalid(problems) => {
                panic!("the manifest was refused: {problems:?}")
            }
            BatchOutcome::Failed { message, .. } => {
                panic!("the batch failed: {message}")
            }
        }
    }

    fn invalid(outcome: BatchOutcome) -> Vec<String> {
        match outcome {
            BatchOutcome::Invalid(problems) => problems,
            other => panic!("the manifest was accepted: {other:?}"),
        }
    }

    /// Each item line's fields, keyed by the entry's ref.
    fn items(
        report: &BatchReport,
    ) -> BTreeMap<String, (String, String, String)> {
        report
            .lines
            .iter()
            .filter(|line| !line.starts_with('#'))
            .map(|line| {
                let fields: Vec<&str> = line.split('\t').collect();
                assert_eq!(fields.len(), 4, "{line}");
                (
                    fields[0].to_owned(),
                    (
                        fields[1].to_owned(),
                        fields[2].to_owned(),
                        fields[3].to_owned(),
                    ),
                )
            })
            .collect()
    }

    fn id_of(content: &str) -> String {
        identity_of(&WorkItemFile {
            path: PathBuf::new(),
            content: content.to_owned(),
        })
        .expect("an id")
        .id
    }

    fn parent_of(content: &str) -> Option<String> {
        let (frontmatter, _) =
            work_adapters::sync::digest::split_frontmatter_and_body(content)
                .expect("frontmatter");
        work::show::read_field_raw(&frontmatter, "parent")
    }

    fn creates(tracker: &RecordingTracker) -> usize {
        tracker
            .calls()
            .iter()
            .filter(|call| matches!(call, Call::Create { .. }))
            .count()
    }

    fn retryable() -> TrackerError {
        TrackerError::Retryable {
            detail: "connection refused".to_owned(),
        }
    }

    #[test]
    fn push_declined_writes_every_item_as_a_draft_linked_to_parent_drafts() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let report = reported(repo.run(EPIC_AND_TWO_STORIES, false, &tracker));

        let lines = items(&report);
        assert_eq!(report.code, 0);
        assert_eq!(creates(&tracker), 0);
        assert!(lines
            .values()
            .all(|(_, keyword, key)| keyword == "declined" && key.is_empty()));
        let epic = read(Path::new(&lines["epic"].0));
        let epic_id = id_of(&epic);
        assert!(DraftId::parse(&epic_id).is_some(), "{epic_id}");
        for story in ["story-a", "story-b"] {
            let path = PathBuf::from(&lines[story].0);
            assert!(path.starts_with(repo.path("meta/work/drafts")));
            assert_eq!(
                parent_of(&read(&path)),
                Some(format!("work-item:{epic_id}"))
            );
        }
    }

    #[test]
    fn push_accepted_creates_all_three_and_links_children_to_the_epic_key() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let report = reported(repo.run(EPIC_AND_TWO_STORIES, true, &tracker));

        let lines = items(&report);
        assert_eq!(report.code, 0);
        assert_eq!(creates(&tracker), 3);
        assert_eq!(
            report.lines[0],
            format!(
                "epic\t{}\twrite-once\tREC-1",
                repo.path("meta/work/REC-1-the-epic.md").display()
            ),
            "parents are created first"
        );
        for story in ["story-a", "story-b"] {
            let (path, keyword, key) = &lines[story];
            assert_eq!(keyword, "write-once");
            assert!(key.starts_with("REC-"), "{key}");
            assert_eq!(
                parent_of(&read(Path::new(path))),
                Some("work-item:REC-1".to_owned())
            );
        }
        assert!(repo.files_under("meta/work/drafts").is_empty());
    }

    #[test]
    fn a_cycle_stops_the_batch_before_any_create_naming_the_members() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let problems = invalid(repo.run(
            r#"[
                {"ref": "a", "title": "A", "kind": "story", "priority": "low",
                 "parent": {"ref": "b"}},
                {"ref": "free", "title": "Free", "kind": "story",
                 "priority": "low"},
                {"ref": "b", "title": "B", "kind": "story", "priority": "low",
                 "parent": {"ref": "a"}}
            ]"#,
            true,
            &tracker,
        ));

        assert_eq!(
            problems,
            vec![
                "E_BATCH_CYCLE: the parents of a, b form a cycle; nothing was \
                 created"
                    .to_owned()
            ]
        );
        assert_eq!(creates(&tracker), 0);
        assert!(repo.files_under("meta/work").is_empty());
        assert!(repo.files_under("meta/work/drafts").is_empty());
    }

    #[test]
    fn a_rejected_manifest_writes_nothing() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let problems = invalid(repo.run(
            r#"[{"ref": "a", "title": "A", "kind": "story", "priority": "low"},
                {"ref": "a", "title": "B", "kind": "story", "priority": "low"}]"#,
            true,
            &tracker,
        ));

        assert!(problems[0].contains("entry 'a'"), "{problems:?}");
        assert_eq!(creates(&tracker), 0);
        assert!(repo.files_under("meta/work/drafts").is_empty());
    }

    #[test]
    fn a_local_save_parent_still_creates_its_children_linked_to_its_draft() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(
            RecordingTracker::holding(Vec::new()).failing_create(retryable()),
        );

        let report = reported(repo.run(EPIC_AND_TWO_STORIES, true, &tracker));

        let lines = items(&report);
        assert_eq!(report.code, 0);
        let epic_id = id_of(&read(Path::new(&lines["epic"].0)));
        for story in ["story-a", "story-b"] {
            assert_eq!(lines[story].1, "local-save");
            assert_eq!(
                parent_of(&read(Path::new(&lines[story].0))),
                Some(format!("work-item:{epic_id}"))
            );
        }

        let healthy = Rc::new(RecordingTracker::holding(Vec::new()));
        let promoted = crate::promote::run(
            repo.root.path(),
            &repo.config,
            &crate::cli::PromoteArgs {
                draft_id: epic_id,
                adopt: None,
                create: false,
            },
            &StubRegistry(Rc::clone(&healthy)),
        );
        assert!(
            matches!(
                promoted,
                crate::promote::PromoteOutcome::Reported { code: 0, .. }
            ),
            "promoting_the_parent_rewrites_both_children: {promoted:?}"
        );
        for story in ["story-a", "story-b"] {
            assert_eq!(
                parent_of(&read(Path::new(&lines[story].0))),
                Some("work-item:REC-1".to_owned()),
                "promoting_the_parent_rewrites_both_children"
            );
        }
    }

    fn in_the_work_directory_itself(path: &Path) -> bool {
        path.parent().is_some_and(|dir| dir.ends_with("meta/work"))
            && path.extension().is_some_and(|extension| extension == "md")
    }

    #[test]
    fn a_write_failed_parent_links_children_to_its_tracker_key() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let report = reported(repo.run_failing(
            EPIC_AND_TWO_STORIES,
            true,
            &tracker,
            Some((in_the_work_directory_itself, 2)),
        ));

        let lines = items(&report);
        assert_eq!(lines["epic"].1, "created-unwritten");
        assert_eq!(lines["epic"].2, "REC-1");
        assert_eq!(report.code, exit_codes::TERMINAL);
        for story in ["story-a", "story-b"] {
            assert_eq!(
                parent_of(&read(Path::new(&lines[story].0))),
                Some("work-item:REC-1".to_owned())
            );
        }
    }

    fn item(keyword: BatchKeyword, key: Option<&str>) -> BatchItem {
        BatchItem {
            reference: keyword.keyword().to_owned(),
            path: None,
            keyword,
            key: key.map(str::to_owned),
            resulting_id: None,
            cause: None,
            details: Vec::new(),
        }
    }

    #[test]
    fn each_line_carries_the_outcome_keyword_and_key() {
        let lines: Vec<String> = [
            item(BatchKeyword::Pushed(PushOutcome::WriteOnce), Some("PP-901")),
            item(BatchKeyword::Pushed(PushOutcome::LocalSave), None),
            item(BatchKeyword::Pushed(PushOutcome::LoudTerminal), None),
            item(
                BatchKeyword::Pushed(PushOutcome::CreatedUnwritten),
                Some("PP-904"),
            ),
            item(
                BatchKeyword::Pushed(PushOutcome::RetirementIncomplete),
                Some("PP-905"),
            ),
        ]
        .iter()
        .map(BatchItem::line)
        .collect();

        assert_eq!(
            lines,
            vec![
                "write-once\t\twrite-once\tPP-901",
                "local-save\t\tlocal-save\t",
                "loud-terminal\t\tloud-terminal\t",
                "created-unwritten\t\tcreated-unwritten\tPP-904",
                "retirement-incomplete\t\tretirement-incomplete\tPP-905",
            ],
            "a_retirement_incomplete_line_carries_its_key"
        );
    }

    #[test]
    fn the_batch_exits_with_the_highest_precedence_item_code() {
        use BatchKeyword::Declined;
        use BatchKeyword::Pending;
        use BatchKeyword::Pushed;
        use PushOutcome::CreatedBlocked;
        use PushOutcome::CreatedUnwritten;
        use PushOutcome::LocalSave;
        use PushOutcome::Rejected;
        use PushOutcome::WriteOnce;

        assert_eq!(
            exit_code([Pushed(LocalSave), Pushed(CreatedUnwritten)]),
            71
        );
        assert_eq!(exit_code([Pushed(Rejected), Pushed(LocalSave)]), 75);
        assert_eq!(exit_code([Pushed(Rejected), Pushed(CreatedUnwritten)]), 71);
        assert_eq!(exit_code([Pushed(CreatedBlocked), Pushed(Rejected)]), 4);
        assert_eq!(exit_code([Pending, Pushed(Rejected)]), 4);
        assert_eq!(exit_code([Pushed(WriteOnce), Declined]), 0);
    }

    #[test]
    fn a_rerun_of_a_partial_batch_creates_only_the_unreached_entries() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let first_two = r#"[
            {"ref": "epic", "title": "The epic", "kind": "epic",
             "priority": "high"},
            {"ref": "story-a", "title": "Story A", "kind": "story",
             "priority": "medium", "parent": {"ref": "epic"}}
        ]"#;
        reported(repo.run(first_two, true, &tracker));

        let report = reported(repo.run(EPIC_AND_TWO_STORIES, true, &tracker));

        let lines = items(&report);
        assert_eq!(creates(&tracker), 3, "one remote create per entry");
        assert_eq!(lines["epic"].1, "write-once");
        assert_eq!(lines["epic"].2, "REC-1");
        assert_eq!(lines["story-a"].2, "REC-2");
        assert_eq!(lines["story-b"].2, "REC-3");
        assert_eq!(
            parent_of(&read(Path::new(&lines["story-b"].0))),
            Some("work-item:REC-1".to_owned()),
            "children_of_a_journalled_parent_link_to_its_recorded_key"
        );
    }

    #[test]
    fn a_draft_journalled_before_its_create_finished_is_reported_from_the_corpus(
    ) {
        let repo = BatchRepo::tracker_owned();
        let crashed = Rc::new(
            RecordingTracker::holding(Vec::new()).failing_create(retryable()),
        );
        let only_epic = r#"[{"ref": "epic", "title": "The epic",
                             "kind": "epic", "priority": "high"}]"#;
        let first = items(&reported(repo.run(only_epic, true, &crashed)));
        let draft = first["epic"].0.clone();

        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let report = reported(repo.run(only_epic, true, &tracker));

        assert_eq!(
            items(&report)["epic"],
            (draft, "pending".to_owned(), String::new())
        );
        assert_eq!(creates(&tracker), 0);
        assert_eq!(report.code, exit_codes::UNRESOLVED);
    }

    #[test]
    fn a_regenerated_manifest_with_reworded_bodies_matches_journalled_entries_by_title(
    ) {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        repo.write("epic.md", "# NNNN: The epic\n\nFirst wording.\n");
        let manifest = r#"[{"ref": "epic", "title": "The epic",
                            "kind": "epic", "priority": "high",
                            "body_file": "epic.md"}]"#;
        reported(repo.run(manifest, true, &tracker));
        repo.write("epic.md", "# NNNN: The epic\n\nSecond wording.\n");

        let report = reported(repo.run(manifest, true, &tracker));

        assert_eq!(items(&report)["epic"].1, "write-once");
        assert_eq!(items(&report)["epic"].2, "REC-1");
        assert_eq!(creates(&tracker), 1);
    }

    #[test]
    fn an_unrelated_batch_sharing_a_title_creates_its_own_item() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        repo.write("first.md", "# NNNN: Add tests\n\nFor the parser.\n");
        repo.write("second.md", "# NNNN: Add tests\n\nFor the renderer.\n");
        reported(repo.run(
            r#"[{"ref": "tests", "title": "Add tests", "kind": "task",
                 "priority": "low", "body_file": "first.md"}]"#,
            true,
            &tracker,
        ));

        let report = reported(repo.run(
            r#"[{"ref": "cleanup", "title": "Remove dead code",
                 "kind": "task", "priority": "low"},
                {"ref": "tests", "title": "Add tests", "kind": "task",
                 "priority": "low", "body_file": "second.md"}]"#,
            true,
            &tracker,
        ));

        assert_eq!(items(&report)["tests"].1, "write-once");
        assert_eq!(items(&report)["tests"].2, "REC-3");
        assert_eq!(creates(&tracker), 3);
    }

    #[test]
    fn a_reordered_manifest_resumes_identically() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let first =
            items(&reported(repo.run(EPIC_AND_TWO_STORIES, true, &tracker)));
        let reordered = r#"[
            {"ref": "story-b", "title": "Story B", "kind": "story",
             "priority": "low", "parent": {"ref": "epic"}},
            {"ref": "epic", "title": "The epic", "kind": "epic",
             "priority": "high"},
            {"ref": "story-a", "title": "Story A", "kind": "story",
             "priority": "medium", "parent": {"ref": "epic"}}
        ]"#;

        let second = items(&reported(repo.run(reordered, true, &tracker)));

        assert_eq!(second, first);
        assert_eq!(creates(&tracker), 3);
    }

    #[test]
    fn a_batch_item_matching_a_pending_draft_is_pending_and_its_children_link_to_it(
    ) {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let only_epic = r#"[{"ref": "epic", "title": "The epic",
                             "kind": "epic", "priority": "high"}]"#;
        let declined = items(&reported(repo.run(only_epic, false, &tracker)));
        let draft = declined["epic"].0.clone();
        let draft_id = id_of(&read(Path::new(&draft)));

        let report = reported(repo.run(EPIC_AND_TWO_STORIES, true, &tracker));

        let lines = items(&report);
        assert_eq!(lines["epic"], (draft, "pending".to_owned(), String::new()));
        assert_eq!(creates(&tracker), 2);
        assert_eq!(report.code, exit_codes::UNRESOLVED);
        assert!(
            report
                .causes
                .iter()
                .any(|cause| cause.starts_with("epic: E_DRAFT_EXISTS")),
            "{:?}",
            report.causes
        );
        assert_eq!(
            parent_of(&read(Path::new(&lines["story-a"].0))),
            Some(format!("work-item:{draft_id}"))
        );
    }

    #[test]
    fn a_child_whose_parent_took_no_id_says_it_was_created_unlinked() {
        let repo = BatchRepo::tracker_owned();
        let marker = pending_push::path(
            &repo.path("integrations"),
            "linear",
            "the-epic",
        );
        repo.write(
            marker
                .strip_prefix(repo.root.path())
                .expect("inside")
                .to_str()
                .expect("utf8"),
            &pending_push::render(&work::sync::PendingPush::Attempted {
                request: work::sync::RequestFingerprint {
                    title: "The epic".to_owned(),
                    digest: "old".to_owned(),
                    attempted_at: 1,
                    failure: None,
                },
            }),
        );
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let report = reported(repo.run(EPIC_AND_TWO_STORIES, true, &tracker));

        let lines = items(&report);
        assert_eq!(
            lines["epic"],
            (String::new(), "pending".to_owned(), String::new())
        );
        assert_eq!(parent_of(&read(Path::new(&lines["story-a"].0))), None);
        assert!(
            report.causes.iter().any(|cause| cause
                == "story-a: its parent epic took no ID, so it was created \
                    without a parent"),
            "{:?}",
            report.causes
        );
    }

    #[test]
    fn blocked_and_incomplete_items_carry_trailing_detail_lines() {
        let repo = BatchRepo::tracker_owned();
        repo.write(
            "meta/work/0002-holder.md",
            "---\nid: \"0002\"\naliases: [\"REC-1\"]\n---\n\n# 0002: Holder\n",
        );
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let report = reported(repo.run(
            r#"[{"ref": "epic", "title": "The epic", "kind": "epic",
                 "priority": "high"}]"#,
            true,
            &tracker,
        ));

        assert_eq!(items(&report)["epic"].1, "created-blocked");
        assert_eq!(items(&report)["epic"].2, "REC-1");
        assert_eq!(
            report.lines.last().map(String::as_str),
            Some(
                format!(
                    "#\tdetail\tepic\tholder\t{}",
                    repo.path("meta/work/0002-holder.md").display()
                )
                .as_str()
            )
        );
        assert_eq!(report.code, exit_codes::UNRESOLVED);
    }

    #[test]
    fn an_incomplete_item_renders_its_paths_and_recovery_directory() {
        let incomplete = BatchItem {
            details: vec![
                Detail {
                    source: work_adapters::promotion::DetailSource::Recovery {
                        location: PathBuf::from(".accelerator/state/r"),
                    },
                    path: PathBuf::from("meta/plans/p.md"),
                },
                Detail {
                    source: work_adapters::promotion::DetailSource::Vcs,
                    path: PathBuf::from("meta/work/x.md"),
                },
            ],
            ..item(
                BatchKeyword::Pushed(PushOutcome::RetirementIncomplete),
                Some("PP-905"),
            )
        };

        let lines = report(&[incomplete]).lines;

        assert_eq!(
            lines[1..],
            [
                "#\tdetail\tretirement-incomplete\trecovery\tmeta/plans/p.md\t\
                 .accelerator/state/r",
                "#\tdetail\tretirement-incomplete\tvcs\tmeta/work/x.md",
            ]
        );
    }

    #[test]
    fn a_parent_given_as_a_typed_reference_to_an_existing_item_is_kept() {
        let repo = BatchRepo::tracker_owned();
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        let legacy = [
            ("meta/work/0042-a.md", "---\nid: \"0042\"\n---\n\n# 0042: A\n"),
            (
                "meta/work/ACC-0042-b.md",
                "---\nid: \"ACC-0042\"\n---\n\n# ACC-0042: B\n",
            ),
            (
                "meta/work/0230-c.md",
                "---\nid: \"0230\"\nexternal_id: \"PP-760\"\n---\n\n# 0230: C\n",
            ),
        ];
        for (path, content) in legacy {
            repo.write(path, content);
        }

        let report = reported(repo.run(
            r#"[
                {"ref": "a", "title": "Under 0042", "kind": "story",
                 "priority": "low", "parent": "work-item:0042"},
                {"ref": "b", "title": "Under ACC-0042", "kind": "story",
                 "priority": "low", "parent": "work-item:ACC-0042"},
                {"ref": "c", "title": "Under PP-760", "kind": "story",
                 "priority": "low", "parent": "work-item:PP-760"}
            ]"#,
            true,
            &tracker,
        ));

        let lines = items(&report);
        for (reference, parent) in [
            ("a", "work-item:0042"),
            ("b", "work-item:ACC-0042"),
            ("c", "work-item:PP-760"),
        ] {
            assert_eq!(
                parent_of(&read(Path::new(&lines[reference].0))),
                Some(parent.to_owned())
            );
        }
        for (path, content) in legacy {
            assert_eq!(
                read(&repo.path(path)),
                content,
                "legacy_items_named_as_parents_keep_their_ids"
            );
        }
    }

    #[test]
    fn a_numeric_pattern_batch_allocates_sequential_numbers() {
        let repo = BatchRepo::numeric();
        repo.write("meta/work/0007-existing.md", "---\nid: \"0007\"\n---\n");
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));

        let report = reported(repo.run(EPIC_AND_TWO_STORIES, false, &tracker));

        let lines = items(&report);
        assert_eq!(
            lines["epic"].0,
            repo.path("meta/work/0008-the-epic.md")
                .display()
                .to_string()
        );
        assert_eq!(
            lines["story-a"].0,
            repo.path("meta/work/0009-story-a.md").display().to_string()
        );
        assert_eq!(
            parent_of(&read(Path::new(&lines["story-b"].0))),
            Some("work-item:0008".to_owned())
        );
    }
}

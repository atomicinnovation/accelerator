//! Backfills the required `tags` base field, as `tags: []`, into every work
//! item under `paths.work` whose frontmatter lacks it.
//!
//! The insertion walks byte offsets rather than using `migrations/text.rs`,
//! whose `lines()`/`join` drops `\r`, and its helpers are inline because an
//! applied migration is immutable history. Key presence and document type
//! deliberately come from the live `corpus::frontmatter_validation` parser,
//! as in m0007, so the migration repairs what `MISSING-BASE-FIELD` reports.
//! That parser only reads bare column-0 keys, so each repair is also checked
//! with a YAML parse, which keeps YAML readers from meeting a duplicate key
//! or a changed value. A failed write or realignment restores the items
//! already repaired, because once an item holds `tags` a re-run could never
//! see its original to realign its sync baseline.

use std::path::Path;
use std::path::PathBuf;

use corpus::frontmatter_validation::declared_type;
use corpus::frontmatter_validation::is_present;
use corpus::frontmatter_validation::parse_entries;
use corpus::FrontmatterValue;
use corpus::Mapping;

use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::ports::TextFile;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0012;

impl MigrationMeta for Migration0012 {
    fn id(&self) -> &'static str {
        "0012-backfill-work-item-tags"
    }

    fn description(&self) -> &'static str {
        "Add the required tags field, empty, to every work item missing it."
    }
}

impl Migration for Migration0012 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let Some(configured) = backfillable_work_dir(ctx)? else {
            return Ok(ApplyOutcome::Applied);
        };
        let work_dir = ctx.root().join(&configured);
        if !ctx.dir_exists(&work_dir) {
            eprintln!("Warning: 0012: no work directory at {configured}");
            eprintln!("{}", summary(&configured, 0, 0));
            return Ok(ApplyOutcome::Applied);
        }
        let mut backfill = Backfill::default();
        let realigned = match backfill.repair_and_realign(ctx, &work_dir) {
            Ok(realigned) => realigned,
            Err(failure) => return Err(backfill.restore_after(ctx, &failure)),
        };
        eprintln!(
            "{}",
            summary(&configured, backfill.repairs.len(), realigned)
        );
        if let Some(note) = untouched_note(ctx.root(), &backfill.untouched) {
            eprintln!("{note}");
        }
        Ok(ApplyOutcome::Applied)
    }
}

fn backfillable_work_dir(
    ctx: &dyn MigrationContext,
) -> Result<Option<String>, MigrationError> {
    let configured = ctx.config_value("paths.work")?.unwrap_or_default();
    if configured.is_empty() {
        return Err(MigrationError::new(
            "0012: config path returned empty for 'work'",
        ));
    }
    if is_dangerous_path(&configured) {
        eprintln!(
            "Warning: 0012: not backfilling paths.work value {configured}, \
             which is not a plain path inside the repository; add `tags: []` \
             by hand to each work item there that lacks it"
        );
        return Ok(None);
    }
    Ok(Some(configured))
}

fn is_dangerous_path(path: &str) -> bool {
    path.is_empty()
        || path == "."
        || path == ".."
        || path == "/"
        || path.starts_with('/')
        || path.ends_with("/..")
        || path.starts_with("../")
        || path.contains("/../")
        || path.contains("/./")
}

fn summary(configured: &str, repaired: usize, realigned: usize) -> String {
    format!(
        "0012: added tags to {repaired} work item(s) under {configured}, \
         {realigned} sync baseline(s) realigned"
    )
}

#[derive(Debug, PartialEq, Eq)]
enum Untouched {
    NotUtf8,
    UnparseableFrontmatter(MigrationError),
    FrontmatterNotAMapping,
    NonCanonicalTagsKey,
    InsertionChangesMeaning,
}

impl Untouched {
    fn reason_and_fix(&self) -> String {
        match self {
            Self::NotUtf8 => "not UTF-8; add `tags: []` if it is a work item \
                              lacking it"
                .to_owned(),
            Self::UnparseableFrontmatter(cause) => format!(
                "frontmatter does not parse as YAML ({cause}); fix the \
                 frontmatter, then add `tags: []`"
            ),
            Self::FrontmatterNotAMapping => "frontmatter is not a YAML \
                                             mapping; rewrite it as `key: \
                                             value` lines, then add \
                                             `tags: []`"
                .to_owned(),
            Self::NonCanonicalTagsKey => "its tags key is spelled \
                                          non-canonically; rewrite the key \
                                          as `tags:`"
                .to_owned(),
            Self::InsertionChangesMeaning => "inserting `tags: []` would \
                                              change another value; add \
                                              `tags: []` by hand where it \
                                              does not"
                .to_owned(),
        }
    }
}

fn untouched_note(
    root: &Path,
    untouched: &[(PathBuf, Untouched)],
) -> Option<String> {
    let listing = untouched
        .iter()
        .map(|(file, reason)| {
            format!(
                "\n  {}: {}",
                relative_to(root, file).display(),
                reason.reason_and_fix()
            )
        })
        .collect::<Vec<_>>()
        .concat();
    (!listing.is_empty()).then(|| {
        format!(
            "note: 0012: left these files untouched, each needing the fix \
             shown:{listing}"
        )
    })
}

fn relative_to<'a>(root: &Path, file: &'a Path) -> &'a Path {
    file.strip_prefix(root).unwrap_or(file)
}

struct Repair {
    file: PathBuf,
    original: String,
    repaired: String,
}

enum Restoration {
    AlreadyOriginal,
    Restored,
    ChangedSinceRepair,
    Failed(MigrationError),
}

impl Repair {
    fn restore(&self, ctx: &dyn MigrationContext) -> Restoration {
        let current = match ctx.read_text(&self.file) {
            Ok(Some(TextFile::Utf8(current))) => current,
            Ok(_) => return Restoration::ChangedSinceRepair,
            Err(cause) => return Restoration::Failed(cause),
        };
        if current == self.original {
            return Restoration::AlreadyOriginal;
        }
        if current != self.repaired {
            return Restoration::ChangedSinceRepair;
        }
        match ctx.write(&self.file, &self.original) {
            Ok(()) => Restoration::Restored,
            Err(_) if self.holds_original(ctx) => Restoration::Restored,
            Err(cause) => Restoration::Failed(cause),
        }
    }

    fn holds_original(&self, ctx: &dyn MigrationContext) -> bool {
        matches!(
            ctx.read_text(&self.file),
            Ok(Some(TextFile::Utf8(current))) if current == self.original
        )
    }
}

enum Failure {
    Backfilling(MigrationError),
    Realigning(MigrationError),
}

impl Failure {
    fn headline(&self) -> String {
        match self {
            Self::Backfilling(cause) => {
                format!("0012: adding tags failed: {cause}.")
            }
            Self::Realigning(cause) => {
                format!("0012: realigning sync baselines failed: {cause}.")
            }
        }
    }
}

const RECOVERY_STEPS: &str =
    "If the migrate pre-flight then reports changes from \
                        this run, including any sync baselines, commit them \
                        all rather than discarding them, and don't push yet; \
                        re-run /accelerator:migrate before syncing.";

fn restoration_message(
    root: &Path,
    failure: &Failure,
    restored: usize,
    by_hand: &[(&Path, MigrationError)],
) -> String {
    let mut sentences = vec![failure.headline()];
    if restored > 0 {
        sentences.push(format!(
            "Left the {restored} work item(s) it had repaired as it found \
             them."
        ));
    }
    if !by_hand.is_empty() {
        let named = by_hand
            .iter()
            .map(|(file, cause)| {
                format!("{} ({cause})", relative_to(root, file).display())
            })
            .collect::<Vec<_>>()
            .join(", ");
        sentences.push(format!(
            "Before re-running, remove the tags: [] line 0012 added from: \
             {named}."
        ));
    }
    sentences.push(RECOVERY_STEPS.to_owned());
    sentences.join("\n")
}

#[derive(Default)]
struct Backfill {
    repairs: Vec<Repair>,
    untouched: Vec<(PathBuf, Untouched)>,
}

impl Backfill {
    fn repair_and_realign(
        &mut self,
        ctx: &dyn MigrationContext,
        work_dir: &Path,
    ) -> Result<usize, Failure> {
        self.over(ctx, work_dir).map_err(Failure::Backfilling)?;
        ctx.realign_sync_baseline(&self.pre_migration())
            .map_err(Failure::Realigning)
    }

    fn over(
        &mut self,
        ctx: &dyn MigrationContext,
        work_dir: &Path,
    ) -> Result<(), MigrationError> {
        for file in ctx.list_md_files(work_dir)? {
            let content = match ctx.read_text(&file)? {
                None => continue,
                Some(TextFile::NotUtf8) => {
                    self.untouched.push((file, Untouched::NotUtf8));
                    continue;
                }
                Some(TextFile::Utf8(content)) => content,
            };
            let Some(backfilled) = with_empty_tags(&content) else {
                continue;
            };
            if let Some(reason) = yaml_obstacle(ctx, &content, &backfilled) {
                self.untouched.push((file, reason));
                continue;
            }
            self.attempt_and_record(
                ctx,
                Repair {
                    file,
                    original: content,
                    repaired: backfilled,
                },
            )?;
        }
        Ok(())
    }

    /// Records the repair whatever the write reports, because a write can
    /// land and still report failure, and a rollback must find it.
    fn attempt_and_record(
        &mut self,
        ctx: &dyn MigrationContext,
        repair: Repair,
    ) -> Result<(), MigrationError> {
        let written = ctx.write(&repair.file, &repair.repaired);
        self.repairs.push(repair);
        written
    }

    fn pre_migration(&self) -> Vec<(PathBuf, String)> {
        self.repairs
            .iter()
            .map(|repair| (repair.file.clone(), repair.original.clone()))
            .collect()
    }

    fn restore_after(
        &self,
        ctx: &dyn MigrationContext,
        failure: &Failure,
    ) -> MigrationError {
        let mut restored = 0usize;
        let mut by_hand = Vec::new();
        for repair in &self.repairs {
            match repair.restore(ctx) {
                Restoration::AlreadyOriginal => {}
                Restoration::Restored => restored += 1,
                Restoration::ChangedSinceRepair => by_hand.push((
                    repair.file.as_path(),
                    MigrationError::new("it changed after the repair"),
                )),
                Restoration::Failed(cause) => {
                    by_hand.push((repair.file.as_path(), cause));
                }
            }
        }
        MigrationError::new(restoration_message(
            ctx.root(),
            failure,
            restored,
            &by_hand,
        ))
    }
}

fn yaml_obstacle(
    ctx: &dyn MigrationContext,
    original: &str,
    repaired: &str,
) -> Option<Untouched> {
    let before = match ctx.parse_frontmatter(original) {
        Ok(FrontmatterValue::Mapping(before)) => before,
        Ok(_) => return Some(Untouched::FrontmatterNotAMapping),
        Err(cause) => return Some(Untouched::UnparseableFrontmatter(cause)),
    };
    if before.get("tags").is_some() {
        return Some(Untouched::NonCanonicalTagsKey);
    }
    match ctx.parse_frontmatter(repaired) {
        Ok(FrontmatterValue::Mapping(after))
            if is_original_plus_empty_tags(&before, &after) =>
        {
            None
        }
        _ => Some(Untouched::InsertionChangesMeaning),
    }
}

fn is_original_plus_empty_tags(before: &Mapping, after: &Mapping) -> bool {
    let empty_tags = FrontmatterValue::Sequence(Vec::new());
    after.entries().len() == before.entries().len() + 1
        && after.get("tags") == Some(&empty_tags)
        && before
            .entries()
            .iter()
            .all(|(key, value)| after.get(key) == Some(value))
}

const FENCE: &str = "---";
const EMPTY_TAGS: &str = "tags: []";

struct Line<'a> {
    start: usize,
    text: &'a str,
}

impl<'a> Line<'a> {
    fn content(&self) -> &'a str {
        self.text.trim_end_matches('\n').trim_end_matches('\r')
    }

    fn ending(&self) -> &'a str {
        &self.text[self.content().len()..]
    }

    const fn end(&self) -> usize {
        self.start + self.text.len()
    }

    fn holds_key(&self, key: &str) -> bool {
        self.content()
            .split_once(':')
            .is_some_and(|(found, _)| found == key)
    }
}

fn lines_with_offsets(content: &str) -> Vec<Line<'_>> {
    let mut start = 0;
    content
        .split_inclusive('\n')
        .map(|text| {
            let line = Line { start, text };
            start += text.len();
            line
        })
        .collect()
}

struct Frontmatter<'a> {
    opening: &'a Line<'a>,
    fields: &'a [Line<'a>],
    closing: &'a Line<'a>,
    entries: Vec<(String, String)>,
}

struct Insertion<'a> {
    at: usize,
    ending: &'a str,
}

fn fenced_frontmatter<'a>(
    content: &str,
    lines: &'a [Line<'a>],
) -> Option<Frontmatter<'a>> {
    let (opening, rest) = lines.split_first()?;
    if opening.content() != FENCE {
        return None;
    }
    let fence_at = rest.iter().position(|line| line.content() == FENCE)?;
    let (fields, from_closing) = rest.split_at(fence_at);
    let closing = from_closing.first()?;
    Some(Frontmatter {
        opening,
        fields,
        closing,
        entries: parse_entries(&content[opening.end()..closing.start]),
    })
}

impl<'a> Frontmatter<'a> {
    fn is_work_item_lacking_tags(&self) -> bool {
        declared_type(&self.entries) == Some("work-item")
            && !is_present(&self.entries, "tags")
    }

    fn tags_insertion(&self) -> Insertion<'a> {
        let insert_before = self
            .fields
            .iter()
            .position(|line| line.holds_key("last_updated"))
            .unwrap_or(self.fields.len());
        Insertion {
            at: self.fields.get(insert_before).unwrap_or(self.closing).start,
            ending: self.line_above(insert_before).ending(),
        }
    }

    fn line_above(&self, field: usize) -> &'a Line<'a> {
        field
            .checked_sub(1)
            .and_then(|above| self.fields.get(above))
            .unwrap_or(self.opening)
    }
}

fn with_empty_tags(content: &str) -> Option<String> {
    let lines = lines_with_offsets(content);
    let frontmatter = fenced_frontmatter(content, &lines)?;
    if !frontmatter.is_work_item_lacking_tags() {
        return None;
    }
    let Insertion { at, ending } = frontmatter.tags_insertion();
    Some(format!(
        "{}{EMPTY_TAGS}{ending}{}",
        &content[..at],
        &content[at..]
    ))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;

    use corpus::frontmatter_validation::parse_entries;
    use corpus::FrontmatterValue;
    use corpus::Mapping;
    use corpus::Scalar;

    use super::is_dangerous_path;
    use super::untouched_note;
    use super::with_empty_tags;
    use super::Backfill;
    use super::Migration0012;
    use super::Untouched;
    use crate::ports::CorpusIndex;
    use crate::ports::DocTypeDir;
    use crate::ports::MigrationContext;
    use crate::ports::MigrationError;
    use crate::ports::TextFile;
    use crate::registry::ApplyOutcome;
    use crate::registry::Migration as _;

    type TestResult = Result<(), MigrationError>;

    type Parsed = Result<FrontmatterValue, MigrationError>;

    struct NoIndex;

    impl CorpusIndex for NoIndex {
        fn target_exists(&self, _: &str, _: &str) -> bool {
            false
        }
    }

    struct Context {
        root: PathBuf,
        work: Option<String>,
        work_dir_exists: bool,
        files: RefCell<BTreeMap<PathBuf, String>>,
        not_utf8: BTreeSet<PathBuf>,
        parses: HashMap<String, Parsed>,
        failing_writes: BTreeSet<(PathBuf, String)>,
        landing_writes: BTreeSet<(PathBuf, String)>,
        edits_on_failure: Vec<(PathBuf, String)>,
        deletions_on_failure: Vec<PathBuf>,
        unreadable_after_failed_writes: BTreeSet<(PathBuf, String)>,
        unreadable: RefCell<BTreeSet<PathBuf>>,
        failing_listing: bool,
        fail_realign: bool,
        writes: RefCell<Vec<PathBuf>>,
        realigned: RefCell<Option<Vec<(PathBuf, String)>>>,
    }

    impl Context {
        fn new() -> Self {
            Self {
                root: PathBuf::from("/repo"),
                work: Some("meta/work".to_owned()),
                work_dir_exists: true,
                files: RefCell::new(BTreeMap::new()),
                not_utf8: BTreeSet::new(),
                parses: HashMap::new(),
                failing_writes: BTreeSet::new(),
                landing_writes: BTreeSet::new(),
                edits_on_failure: Vec::new(),
                deletions_on_failure: Vec::new(),
                unreadable_after_failed_writes: BTreeSet::new(),
                unreadable: RefCell::new(BTreeSet::new()),
                failing_listing: false,
                fail_realign: false,
                writes: RefCell::new(Vec::new()),
                realigned: RefCell::new(None),
            }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        fn holding(self, relative: &str, content: &str) -> Self {
            self.files
                .borrow_mut()
                .insert(self.path(relative), content.to_owned());
            self
        }

        fn holding_bytes_that_are_not_utf8(mut self, relative: &str) -> Self {
            self.not_utf8.insert(self.path(relative));
            self.holding(relative, "")
        }

        fn parsing(mut self, content: &str, parsed: Parsed) -> Self {
            self.parses.insert(content.to_owned(), parsed);
            self
        }

        fn failing_write_of(mut self, relative: &str, content: &str) -> Self {
            self.failing_writes
                .insert((self.path(relative), content.to_owned()));
            self
        }

        fn landing_write_of(mut self, relative: &str, content: &str) -> Self {
            self.landing_writes
                .insert((self.path(relative), content.to_owned()));
            self
        }

        fn editing_on_failure(mut self, relative: &str, content: &str) -> Self {
            self.edits_on_failure
                .push((self.path(relative), content.to_owned()));
            self
        }

        fn deleting_on_failure(mut self, relative: &str) -> Self {
            self.deletions_on_failure.push(self.path(relative));
            self
        }

        fn unreadable_after_failed_write_of(
            mut self,
            relative: &str,
            content: &str,
        ) -> Self {
            self.unreadable_after_failed_writes
                .insert((self.path(relative), content.to_owned()));
            self
        }

        fn holds_file(&self, relative: &str) -> bool {
            self.files.borrow().contains_key(&self.path(relative))
        }

        fn content(&self, relative: &str) -> String {
            self.files
                .borrow()
                .get(&self.path(relative))
                .cloned()
                .unwrap_or_default()
        }

        fn write_failure(&self, path: &Path) -> MigrationError {
            for (edited, content) in &self.edits_on_failure {
                self.files
                    .borrow_mut()
                    .insert(edited.clone(), content.clone());
            }
            for deleted in &self.deletions_on_failure {
                self.files.borrow_mut().remove(deleted);
            }
            MigrationError::new(format!("cannot write {}", path.display()))
        }
    }

    impl MigrationContext for Context {
        fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
            Vec::new()
        }

        fn corpus_index(&self) -> &dyn CorpusIndex {
            &NoIndex
        }

        fn root(&self) -> &Path {
            &self.root
        }

        fn write(&self, path: &Path, content: &str) -> TestResult {
            self.writes.borrow_mut().push(path.to_path_buf());
            let attempt = (path.to_path_buf(), content.to_owned());
            let lands = !self.failing_writes.contains(&attempt);
            if lands {
                self.files
                    .borrow_mut()
                    .insert(path.to_path_buf(), content.to_owned());
                if !self.landing_writes.contains(&attempt) {
                    return Ok(());
                }
            }
            if self.unreadable_after_failed_writes.contains(&attempt) {
                self.unreadable.borrow_mut().insert(path.to_path_buf());
            }
            Err(self.write_failure(path))
        }

        fn config_value(
            &self,
            key: &str,
        ) -> Result<Option<String>, MigrationError> {
            Ok(self.work.clone().filter(|_| key == "paths.work"))
        }

        fn dir_exists(&self, path: &Path) -> bool {
            self.work_dir_exists
                && self
                    .work
                    .as_deref()
                    .is_some_and(|work| path == self.path(work))
        }

        fn list_md_files(
            &self,
            dir: &Path,
        ) -> Result<Vec<PathBuf>, MigrationError> {
            if self.failing_listing {
                return Err(MigrationError::new("cannot list"));
            }
            Ok(self
                .files
                .borrow()
                .keys()
                .filter(|path| path.starts_with(dir))
                .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
                .cloned()
                .collect())
        }

        fn read_text(
            &self,
            path: &Path,
        ) -> Result<Option<TextFile>, MigrationError> {
            if self.unreadable.borrow().contains(path) {
                return Err(MigrationError::new(format!(
                    "cannot read {}",
                    path.display()
                )));
            }
            if self.not_utf8.contains(path) {
                return Ok(Some(TextFile::NotUtf8));
            }
            Ok(self.files.borrow().get(path).cloned().map(TextFile::Utf8))
        }

        fn parse_frontmatter(&self, content: &str) -> Parsed {
            self.parses.get(content).cloned().unwrap_or_else(|| {
                Ok(FrontmatterValue::Mapping(mapping_of(content)))
            })
        }

        fn realign_sync_baseline(
            &self,
            pre_migration: &[(PathBuf, String)],
        ) -> Result<usize, MigrationError> {
            if self.fail_realign {
                return Err(MigrationError::new("cannot write a baseline"));
            }
            *self.realigned.borrow_mut() = Some(pre_migration.to_vec());
            Ok(pre_migration.len())
        }
    }

    fn mapping_of(content: &str) -> Mapping {
        let frontmatter = content
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("---\n"))
            .map(|(frontmatter, _)| frontmatter)
            .unwrap_or_default();
        parse_entries(frontmatter)
            .into_iter()
            .map(|(key, value)| {
                let value = if value == "[]" {
                    FrontmatterValue::Sequence(Vec::new())
                } else {
                    string(&value)
                };
                (key, value)
            })
            .collect()
    }

    fn string(value: &str) -> FrontmatterValue {
        FrontmatterValue::Scalar(Scalar::String(value.to_owned()))
    }

    fn with_entries(
        content: &str,
        extra: &[(&str, FrontmatterValue)],
    ) -> FrontmatterValue {
        FrontmatterValue::Mapping(
            mapping_of(content)
                .entries()
                .iter()
                .cloned()
                .chain(
                    extra
                        .iter()
                        .map(|(key, value)| ((*key).to_owned(), value.clone())),
                )
                .collect(),
        )
    }

    fn empty_tags() -> (&'static str, FrontmatterValue) {
        ("tags", FrontmatterValue::Sequence(Vec::new()))
    }

    fn untagged(title: &str) -> String {
        format!(
            "---\ntype: \"work-item\"\ntitle: \"{title}\"\n\
             last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n# {title}\n"
        )
    }

    fn tagged(title: &str) -> String {
        format!(
            "---\ntype: \"work-item\"\ntitle: \"{title}\"\ntags: [\"api\"]\n\
             last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n# {title}\n"
        )
    }

    fn repaired(content: &str) -> String {
        with_empty_tags(content).unwrap_or_default()
    }

    fn backfill(ctx: &Context) -> Result<Backfill, MigrationError> {
        let mut backfill = Backfill::default();
        backfill.over(ctx, &ctx.path("meta/work"))?;
        Ok(backfill)
    }

    fn failure_of(ctx: &Context) -> String {
        Migration0012
            .apply(ctx)
            .err()
            .map_or_else(String::new, |error| error.0)
    }

    const FIRST: &str = "meta/work/0001-a.md";
    const SECOND: &str = "meta/work/0002-b.md";
    const THIRD: &str = "meta/work/0003-c.md";
    const REMOVE_BY_HAND: &str = "remove the tags: [] line 0012 added from:";

    #[test]
    fn repairs_only_the_work_items_missing_tags() -> TestResult {
        let ctx = Context::new()
            .holding(FIRST, &untagged("A"))
            .holding(SECOND, &tagged("B"))
            .holding("meta/work/README.md", "# Work\n");

        Migration0012.apply(&ctx)?;

        assert_eq!(ctx.content(FIRST), repaired(&untagged("A")));
        assert_eq!(ctx.content(SECOND), tagged("B"));
        assert_eq!(ctx.content("meta/work/README.md"), "# Work\n");
        assert_eq!(*ctx.writes.borrow(), vec![ctx.path(FIRST)]);
        Ok(())
    }

    #[test]
    fn reads_the_configured_work_directory() -> TestResult {
        let ctx = Context {
            work: Some("docs/work".to_owned()),
            ..Context::new()
        }
        .holding("docs/work/0001-a.md", &untagged("A"))
        .holding(SECOND, &untagged("B"));

        Migration0012.apply(&ctx)?;

        assert_eq!(
            ctx.content("docs/work/0001-a.md"),
            repaired(&untagged("A"))
        );
        assert_eq!(ctx.content(SECOND), untagged("B"));
        Ok(())
    }

    #[test]
    fn a_work_directory_needing_no_repair_still_applies() -> TestResult {
        let ctx = Context::new().holding(FIRST, &tagged("A"));

        let outcome = Migration0012.apply(&ctx)?;

        assert!(matches!(outcome, ApplyOutcome::Applied));
        assert!(ctx.writes.borrow().is_empty());
        assert_eq!(*ctx.realigned.borrow(), Some(Vec::new()));
        Ok(())
    }

    #[test]
    fn a_missing_work_directory_applies_without_listing_or_realigning(
    ) -> TestResult {
        let ctx = Context {
            work_dir_exists: false,
            ..Context::new()
        }
        .holding(FIRST, &untagged("A"));

        let outcome = Migration0012.apply(&ctx)?;

        assert!(matches!(outcome, ApplyOutcome::Applied));
        assert!(ctx.writes.borrow().is_empty());
        assert_eq!(*ctx.realigned.borrow(), None);
        Ok(())
    }

    #[test]
    fn refuses_an_empty_work_path() {
        let ctx = Context {
            work: Some(String::new()),
            ..Context::new()
        };

        let message = failure_of(&ctx);

        assert!(
            message.contains("0012: config path returned empty for 'work'"),
            "{message}"
        );
    }

    #[test]
    fn refuses_an_unset_work_path() {
        let ctx = Context {
            work: None,
            ..Context::new()
        };

        let message = failure_of(&ctx);

        assert!(
            message.contains("0012: config path returned empty for 'work'"),
            "{message}"
        );
    }

    #[test]
    fn dangerous_work_paths_are_refused() {
        for path in [
            "", ".", "..", "/", "/abs", "a/..", "../b", "a/../b", "a/./b",
        ] {
            assert!(is_dangerous_path(path), "{path}");
        }
        for path in ["meta/work", "docs/work"] {
            assert!(!is_dangerous_path(path), "{path}");
        }
    }

    #[test]
    fn applies_without_touching_a_work_path_outside_the_project() -> TestResult
    {
        let ctx = Context {
            work: Some("../elsewhere".to_owned()),
            ..Context::new()
        }
        .holding("../elsewhere/0001-x.md", &untagged("X"));

        let outcome = Migration0012.apply(&ctx)?;

        assert!(matches!(outcome, ApplyOutcome::Applied));
        assert!(ctx.writes.borrow().is_empty());
        assert_eq!(*ctx.realigned.borrow(), None);
        Ok(())
    }

    #[test]
    fn repairs_a_draft_in_a_nested_directory() -> TestResult {
        let draft = "meta/work/drafts/draft-a1b2c3-x.md";
        let ctx = Context::new().holding(draft, &untagged("X"));

        Migration0012.apply(&ctx)?;

        assert_eq!(ctx.content(draft), repaired(&untagged("X")));
        Ok(())
    }

    #[test]
    fn leaves_a_file_that_is_not_utf8_untouched() -> TestResult {
        let ctx = Context::new().holding_bytes_that_are_not_utf8(FIRST);

        let backfill = backfill(&ctx)?;

        assert_eq!(
            backfill.untouched,
            vec![(ctx.path(FIRST), Untouched::NotUtf8)]
        );
        assert!(backfill.repairs.is_empty());
        Ok(())
    }

    #[test]
    fn repairs_an_item_holding_a_literal_replacement_character() -> TestResult {
        let item = untagged("\u{FFFD}");
        let ctx = Context::new().holding(FIRST, &item);

        Migration0012.apply(&ctx)?;

        assert_eq!(ctx.content(FIRST), repaired(&item));
        assert!(ctx.content(FIRST).contains("tags: []\n"));
        Ok(())
    }

    #[test]
    fn leaves_an_item_whose_yaml_already_holds_tags_untouched() -> TestResult {
        let item = untagged("A");
        let ctx = Context::new().holding(FIRST, &item).parsing(
            &item,
            Ok(with_entries(
                &item,
                &[("tags", FrontmatterValue::Sequence(vec![string("api")]))],
            )),
        );

        let backfill = backfill(&ctx)?;

        assert_eq!(
            backfill.untouched,
            vec![(ctx.path(FIRST), Untouched::NonCanonicalTagsKey)]
        );
        assert!(ctx.writes.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn leaves_an_item_whose_frontmatter_does_not_parse_untouched() -> TestResult
    {
        let item = untagged("A");
        let cause = MigrationError::new("did not find expected key");
        let ctx = Context::new()
            .holding(FIRST, &item)
            .parsing(&item, Err(cause.clone()));

        let backfill = backfill(&ctx)?;

        assert_eq!(
            backfill.untouched,
            vec![(ctx.path(FIRST), Untouched::UnparseableFrontmatter(cause))]
        );
        assert!(ctx.writes.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn leaves_an_item_whose_frontmatter_is_not_a_mapping_untouched(
    ) -> TestResult {
        let item = untagged("A");
        let ctx = Context::new()
            .holding(FIRST, &item)
            .parsing(&item, Ok(FrontmatterValue::Sequence(Vec::new())));

        let backfill = backfill(&ctx)?;

        assert_eq!(
            backfill.untouched,
            vec![(ctx.path(FIRST), Untouched::FrontmatterNotAMapping)]
        );
        assert!(ctx.writes.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn frontmatter_that_is_not_a_mapping_is_not_reported_as_unparseable() {
        let reason = Untouched::FrontmatterNotAMapping.reason_and_fix();

        assert!(!reason.contains("does not parse"), "{reason}");
        assert!(reason.contains("not a YAML mapping"), "{reason}");
    }

    fn assert_insertion_changes_meaning(
        repair_parses_as: Parsed,
    ) -> TestResult {
        let item = untagged("A");
        let ctx = Context::new()
            .holding(FIRST, &item)
            .parsing(&repaired(&item), repair_parses_as);

        let backfill = backfill(&ctx)?;

        assert_eq!(
            backfill.untouched,
            vec![(ctx.path(FIRST), Untouched::InsertionChangesMeaning)]
        );
        assert!(ctx.writes.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn leaves_an_item_whose_insertion_would_change_a_value_untouched(
    ) -> TestResult {
        let item = untagged("A");
        let changed: Mapping = mapping_of(&item)
            .entries()
            .iter()
            .map(|(key, value)| {
                let value = if key == "title" {
                    string("changed")
                } else {
                    value.clone()
                };
                (key.clone(), value)
            })
            .chain([empty_tags()].map(|(key, value)| (key.to_owned(), value)))
            .collect();

        assert_insertion_changes_meaning(Ok(FrontmatterValue::Mapping(changed)))
    }

    #[test]
    fn leaves_an_item_whose_repair_lacks_empty_tags_untouched() -> TestResult {
        assert_insertion_changes_meaning(Ok(with_entries(
            &untagged("A"),
            &[("extra", string("x"))],
        )))
    }

    #[test]
    fn leaves_an_item_whose_repair_adds_more_than_tags_untouched() -> TestResult
    {
        assert_insertion_changes_meaning(Ok(with_entries(
            &untagged("A"),
            &[empty_tags(), ("extra", string("x"))],
        )))
    }

    #[test]
    fn leaves_an_item_whose_repair_reads_tags_as_non_empty_untouched(
    ) -> TestResult {
        assert_insertion_changes_meaning(Ok(with_entries(
            &untagged("A"),
            &[("tags", FrontmatterValue::Sequence(vec![string("x")]))],
        )))
    }

    #[test]
    fn leaves_an_item_whose_repair_does_not_parse_untouched() -> TestResult {
        assert_insertion_changes_meaning(Err(MigrationError::new("bad")))
    }

    #[test]
    fn parses_only_items_offered_a_repair() -> TestResult {
        let item = tagged("A");
        let ctx = Context::new()
            .holding(FIRST, &item)
            .parsing(&item, Err(MigrationError::new("bad")));

        let backfill = backfill(&ctx)?;

        assert!(backfill.untouched.is_empty());
        Ok(())
    }

    #[test]
    fn names_each_untouched_file_with_its_reason_in_the_note() {
        let root = Path::new("/repo");
        let note = untouched_note(
            root,
            &[
                (root.join("meta/work/a.md"), Untouched::NotUtf8),
                (
                    root.join("meta/work/b.md"),
                    Untouched::UnparseableFrontmatter(MigrationError::new(
                        "did not find expected key",
                    )),
                ),
                (root.join("meta/work/c.md"), Untouched::NonCanonicalTagsKey),
                (
                    root.join("meta/work/d.md"),
                    Untouched::InsertionChangesMeaning,
                ),
            ],
        )
        .unwrap_or_default();

        for expected in [
            "\n  meta/work/a.md: not UTF-8; add `tags: []` if it is a work \
             item lacking it",
            "\n  meta/work/b.md: frontmatter does not parse as YAML (did not \
             find expected key); fix the frontmatter, then add `tags: []`",
            "\n  meta/work/c.md: its tags key is spelled non-canonically; \
             rewrite the key as `tags:`",
            "\n  meta/work/d.md: inserting `tags: []` would change another \
             value; add `tags: []` by hand where it does not",
        ] {
            assert!(note.contains(expected), "{expected}\n---\n{note}");
        }
        assert_eq!(untouched_note(root, &[]), None);
    }

    #[test]
    fn realigns_the_baselines_of_backfilled_items() -> TestResult {
        let ctx = Context::new()
            .holding(FIRST, &untagged("A"))
            .holding(SECOND, &tagged("B"));

        Migration0012.apply(&ctx)?;

        assert_eq!(
            *ctx.realigned.borrow(),
            Some(vec![(ctx.path(FIRST), untagged("A"))])
        );
        Ok(())
    }

    fn three_untagged_items() -> Context {
        Context::new()
            .holding(FIRST, &untagged("A"))
            .holding(SECOND, &untagged("B"))
            .holding(THIRD, &untagged("C"))
    }

    #[test]
    fn a_failed_write_restores_the_items_already_repaired() {
        let ctx = three_untagged_items()
            .failing_write_of(SECOND, &repaired(&untagged("B")));

        let message = failure_of(&ctx);

        assert_eq!(ctx.content(FIRST), untagged("A"));
        assert_eq!(ctx.content(SECOND), untagged("B"));
        assert_eq!(ctx.content(THIRD), untagged("C"));
        assert_eq!(*ctx.realigned.borrow(), None);
        assert!(
            message.starts_with("0012: adding tags failed:"),
            "{message}"
        );
        assert!(message.contains("Left the 1 work item(s)"), "{message}");
        assert!(!message.contains(REMOVE_BY_HAND), "{message}");
    }

    #[test]
    fn a_write_that_lands_then_fails_is_restored() {
        let ctx = three_untagged_items()
            .landing_write_of(SECOND, &repaired(&untagged("B")));

        let message = failure_of(&ctx);

        assert!(
            message.starts_with("0012: adding tags failed:"),
            "{message}"
        );
        assert_eq!(ctx.content(SECOND), untagged("B"));
    }

    #[test]
    fn a_restoring_write_that_lands_then_fails_counts_as_restored() {
        let ctx = three_untagged_items()
            .failing_write_of(SECOND, &repaired(&untagged("B")))
            .landing_write_of(FIRST, &untagged("A"));

        let message = failure_of(&ctx);

        assert_eq!(ctx.content(FIRST), untagged("A"));
        assert!(message.contains("Left the 1 work item(s)"), "{message}");
        assert!(!message.contains(REMOVE_BY_HAND), "{message}");
    }

    #[test]
    fn a_write_that_never_landed_is_not_counted_as_left_as_found() {
        let ctx = Context::new()
            .holding(FIRST, &untagged("A"))
            .failing_write_of(FIRST, &repaired(&untagged("A")));

        let message = failure_of(&ctx);

        assert_eq!(ctx.content(FIRST), untagged("A"));
        assert!(
            message.starts_with("0012: adding tags failed:"),
            "{message}"
        );
        assert!(!message.contains("Left the"), "{message}");
        assert!(!message.contains(REMOVE_BY_HAND), "{message}");
    }

    #[test]
    fn a_listing_failure_omits_the_left_count() {
        let ctx = Context {
            failing_listing: true,
            ..Context::new()
        };

        let message = failure_of(&ctx);

        assert!(
            message.starts_with("0012: adding tags failed:"),
            "{message}"
        );
        assert!(!message.contains("Left the"), "{message}");
    }

    #[test]
    fn a_failed_realignment_restores_every_repaired_item() {
        let ctx = Context {
            fail_realign: true,
            ..Context::new()
        }
        .holding(FIRST, &untagged("A"))
        .holding(SECOND, &untagged("B"));

        let message = failure_of(&ctx);

        assert_eq!(ctx.content(FIRST), untagged("A"));
        assert_eq!(ctx.content(SECOND), untagged("B"));
        assert!(
            message.starts_with("0012: realigning sync baselines failed:"),
            "{message}"
        );
        assert!(message.contains("Left the 2 work item(s)"), "{message}");
        assert!(!message.contains(REMOVE_BY_HAND), "{message}");
        assert!(!message.contains("last-sync.json"), "{message}");
        for step in [
            "including any sync baselines",
            "commit them all rather than discarding them",
            "don't push yet",
            "re-run /accelerator:migrate before syncing",
        ] {
            assert!(message.contains(step), "{step}\n---\n{message}");
        }
    }

    #[test]
    fn an_item_changed_after_its_repair_is_named_not_overwritten() {
        let ctx = three_untagged_items()
            .failing_write_of(SECOND, &repaired(&untagged("B")))
            .editing_on_failure(FIRST, "edited\n");

        let message = failure_of(&ctx);

        assert_eq!(ctx.content(FIRST), "edited\n");
        let named = message
            .find(&format!("{REMOVE_BY_HAND} {FIRST} ("))
            .unwrap_or(usize::MAX);
        let recovery = message.find("commit them all").unwrap_or(0);
        assert!(named < recovery, "{message}");
    }

    #[test]
    fn a_failed_restore_names_the_write_error_when_the_re_check_cannot_read() {
        let ctx = three_untagged_items()
            .failing_write_of(SECOND, &repaired(&untagged("B")))
            .failing_write_of(FIRST, &untagged("A"))
            .unreadable_after_failed_write_of(FIRST, &untagged("A"));

        let message = failure_of(&ctx);

        assert!(
            message.contains(&format!(
                "{REMOVE_BY_HAND} {FIRST} (cannot write /repo/{FIRST})"
            )),
            "{message}"
        );
    }

    #[test]
    fn an_item_that_cannot_be_read_back_is_named_with_the_read_error() {
        let landed = repaired(&untagged("B"));
        let ctx = three_untagged_items()
            .landing_write_of(SECOND, &landed)
            .unreadable_after_failed_write_of(SECOND, &landed);

        let message = failure_of(&ctx);

        assert!(
            message.contains(&format!(
                "{REMOVE_BY_HAND} {SECOND} (cannot read /repo/{SECOND})"
            )),
            "{message}"
        );
        assert!(message.contains("Left the 1 work item(s)"), "{message}");
    }

    #[test]
    fn an_item_deleted_after_its_repair_is_named_not_recreated() {
        let ctx = three_untagged_items()
            .failing_write_of(SECOND, &repaired(&untagged("B")))
            .deleting_on_failure(FIRST);

        let message = failure_of(&ctx);

        assert!(!ctx.holds_file(FIRST));
        assert!(
            message.contains(&format!(
                "{REMOVE_BY_HAND} {FIRST} (it changed after the repair)"
            )),
            "{message}"
        );
    }

    #[test]
    fn names_the_items_it_could_not_restore() {
        let ctx = three_untagged_items()
            .failing_write_of(SECOND, &repaired(&untagged("B")))
            .failing_write_of(FIRST, &untagged("A"));

        let message = failure_of(&ctx);

        assert!(
            message.contains(&format!(
                "{REMOVE_BY_HAND} {FIRST} (cannot write /repo/{FIRST})"
            )),
            "{message}"
        );
    }

    const UNTAGGED: &str = "---\n\
        type: \"work-item\"\n\
        id: \"0001\"\n\
        title: \"Example\"\n\
        date: \"2026-01-01T00:00:00+00:00\"\n\
        author: \"Toby\"\n\
        producer: \"create-work-item\"\n\
        status: \"draft\"\n\
        kind: \"story\"\n\
        priority: \"medium\"\n\
        last_updated: \"2026-01-01T00:00:00+00:00\"\n\
        last_updated_by: \"Toby\"\n\
        schema_version: 1\n\
        ---\n\
        \n\
        # 0001: Example\n";

    const BACKFILLED: &str = "---\n\
        type: \"work-item\"\n\
        id: \"0001\"\n\
        title: \"Example\"\n\
        date: \"2026-01-01T00:00:00+00:00\"\n\
        author: \"Toby\"\n\
        producer: \"create-work-item\"\n\
        status: \"draft\"\n\
        kind: \"story\"\n\
        priority: \"medium\"\n\
        tags: []\n\
        last_updated: \"2026-01-01T00:00:00+00:00\"\n\
        last_updated_by: \"Toby\"\n\
        schema_version: 1\n\
        ---\n\
        \n\
        # 0001: Example\n";

    #[test]
    fn inserts_tags_immediately_before_last_updated() {
        assert_eq!(with_empty_tags(UNTAGGED).as_deref(), Some(BACKFILLED));
    }

    #[test]
    fn a_repaired_item_validates() {
        let repaired = with_empty_tags(UNTAGGED).unwrap_or_default();
        let frontmatter = repaired
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("---\n"))
            .map(|(frontmatter, _)| frontmatter)
            .unwrap_or_default();

        assert_eq!(
            corpus::frontmatter_validation::validate_file(frontmatter),
            Vec::new()
        );
    }

    #[test]
    fn inserts_tags_as_the_last_frontmatter_line_without_last_updated() {
        assert_eq!(
            with_empty_tags(
                "---\ntype: \"work-item\"\nid: \"0001\"\n---\n\nBody\n"
            )
            .as_deref(),
            Some(
                "---\ntype: \"work-item\"\nid: \"0001\"\ntags: []\n---\n\n\
                 Body\n"
            )
        );
    }

    #[test]
    fn last_updated_by_is_not_mistaken_for_last_updated() {
        assert_eq!(
            with_empty_tags(
                "---\ntype: \"work-item\"\nlast_updated_by: \"Toby\"\n\
                 schema_version: 1\n---\n"
            )
            .as_deref(),
            Some(
                "---\ntype: \"work-item\"\nlast_updated_by: \"Toby\"\n\
                 schema_version: 1\ntags: []\n---\n"
            )
        );
    }

    #[test]
    fn keeps_crlf_line_endings() {
        assert_eq!(
            with_empty_tags(
                "---\r\ntype: \"work-item\"\r\n\
                 last_updated: \"2026-01-01T00:00:00+00:00\"\r\n\
                 ---\r\n\r\nBody\r\n"
            )
            .as_deref(),
            Some(
                "---\r\ntype: \"work-item\"\r\ntags: []\r\n\
                 last_updated: \"2026-01-01T00:00:00+00:00\"\r\n\
                 ---\r\n\r\nBody\r\n"
            )
        );
    }

    #[test]
    fn keeps_a_missing_final_newline() {
        assert_eq!(
            with_empty_tags(
                "---\ntype: \"work-item\"\n\
                 last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\nBody"
            )
            .as_deref(),
            Some(
                "---\ntype: \"work-item\"\ntags: []\n\
                 last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\nBody"
            )
        );
    }

    #[test]
    fn leaves_flow_tags_alone() {
        assert_eq!(
            with_empty_tags("---\ntype: \"work-item\"\ntags: [\"api\"]\n---\n"),
            None
        );
    }

    #[test]
    fn leaves_block_tags_alone() {
        assert_eq!(
            with_empty_tags("---\ntype: \"work-item\"\ntags:\n  - api\n---\n"),
            None
        );
    }

    #[test]
    fn leaves_empty_tags_alone() {
        assert_eq!(
            with_empty_tags("---\ntype: \"work-item\"\ntags: []\n---\n"),
            None
        );
    }

    #[test]
    fn ignores_a_tags_line_in_the_body() {
        assert_eq!(
            with_empty_tags("---\ntype: \"work-item\"\n---\n\ntags: x\n")
                .as_deref(),
            Some("---\ntype: \"work-item\"\ntags: []\n---\n\ntags: x\n")
        );
    }

    #[test]
    fn leaves_other_document_types_alone() {
        assert_eq!(
            with_empty_tags("---\ntype: \"plan\"\nid: \"p\"\n---\n"),
            None
        );
    }

    #[test]
    fn leaves_an_item_without_a_type_alone() {
        assert_eq!(with_empty_tags("---\nid: \"0001\"\n---\n"), None);
    }

    #[test]
    fn repairs_an_item_with_an_unquoted_work_item_type() {
        assert_eq!(
            with_empty_tags(
                "---\ntype: work-item\n\
                 last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n"
            )
            .as_deref(),
            Some(
                "---\ntype: work-item\ntags: []\n\
                 last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n"
            )
        );
    }

    #[test]
    fn leaves_unfenced_content_alone() {
        assert_eq!(with_empty_tags("type: \"work-item\"\n\nBody\n"), None);
    }

    #[test]
    fn leaves_an_unterminated_fence_alone() {
        assert_eq!(with_empty_tags("---\ntype: \"work-item\"\nid: x\n"), None);
    }

    #[test]
    fn a_repaired_item_needs_no_further_repair() {
        assert_eq!(with_empty_tags(BACKFILLED), None);
    }
}

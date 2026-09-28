//! ID retirement: replacing a work item's `id`, keeping the old one as an
//! alias, and rewriting every reference to it across the corpus.
//!
//! Planning is pure; applying the plan all-or-nothing is the adapter's job.

use std::path::Path;
use std::path::PathBuf;

use corpus::references::rewrite_references;
use corpus::references::IdShape;
use corpus::references::Renaming;
use corpus::StoreError;

use crate::dirtiness::Dirtiness;
use crate::draft_id::DraftId;
use crate::identity::holder_of;
use crate::identity::linker_of;
use crate::identity::IdentityField;
use crate::identity::ItemIdentity;
use crate::work_item_files::DRAFTS_DIRECTORY;

const WORK_ITEM: &str = "work-item";

/// Where every retirement keeps its recovery copies, relative to the state
/// directory.
pub const RECOVERY_PARENT: &str = "retirement-recovery";
const WORK_ITEM_REVIEW: &str = "work-item-review";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retirement<'a> {
    pub old_id: &'a str,
    pub new_id: &'a str,
    pub new_external_id: Option<&'a str>,
}

impl Retirement<'_> {
    /// Where the pre-retirement bytes of uncommitted files are kept, relative
    /// to the state directory; one per retirement, so a resumed retirement
    /// finds the copies its first attempt made.
    #[must_use]
    pub fn recovery_dir(&self) -> PathBuf {
        PathBuf::from(RECOVERY_PARENT)
            .join(format!("{}--{}", self.old_id, self.new_id))
    }
}

/// The durable note that a retirement has started.
///
/// Written before it applies and removed once it has finished, rolled back
/// or been refused, so a leftover record always names a retirement still in
/// progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementRecord {
    pub old: String,
    pub new: String,
    pub new_external_id: Option<String>,
    pub recovery_dir: PathBuf,
}

impl RetirementRecord {
    #[must_use]
    pub fn of(retirement: &Retirement<'_>) -> Self {
        Self {
            old: retirement.old_id.to_owned(),
            new: retirement.new_id.to_owned(),
            new_external_id: retirement.new_external_id.map(str::to_owned),
            recovery_dir: retirement.recovery_dir(),
        }
    }

    #[must_use]
    pub fn retirement(&self) -> Retirement<'_> {
        Retirement {
            old_id: &self.old,
            new_id: &self.new,
            new_external_id: self.new_external_id.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusFile {
    pub path: PathBuf,
    pub content: String,
    pub dirtiness: Dirtiness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRewrite {
    pub path: PathBuf,
    pub original: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineRename {
    pub from: String,
    pub to: String,
}

/// The steps a retirement still has to take. A plan for an interrupted
/// retirement holds only those its earlier attempt did not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementPlan {
    pub from: PathBuf,
    /// `from`'s bytes when planned, or `None` once it has been removed.
    pub from_original: Option<String>,
    pub to: PathBuf,
    /// What to create at `to`, or `None` when it has already been written.
    pub retired_item: Option<String>,
    pub rewrites: Vec<FileRewrite>,
    pub baseline_rename: BaselineRename,
    /// Touched paths whose changes version control could not restore.
    pub recovery: Vec<PathBuf>,
    pub recovery_dir: PathBuf,
    pub resumes: bool,
}

impl RetirementPlan {
    /// Whether no file is left to write, create or remove, as when every
    /// corpus step of an interrupted retirement has already landed.
    #[must_use]
    pub const fn leaves_corpus_unchanged(&self) -> bool {
        self.rewrites.is_empty()
            && self.retired_item.is_none()
            && self.from_original.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetirementRefusal {
    IdTaken {
        holder: PathBuf,
        field: IdentityField,
    },
    KeyLinked {
        holder: PathBuf,
    },
    TargetExists(PathBuf),
    ItemNotFound(String),
}

impl RetirementRefusal {
    #[must_use]
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::IdTaken { .. } => "id-taken",
            Self::KeyLinked { .. } => "key-linked",
            Self::TargetExists(_) => "target-exists",
            Self::ItemNotFound(_) => "item-not-found",
        }
    }

    #[must_use]
    pub fn message(&self, retirement: &Retirement<'_>) -> String {
        let Retirement { old_id, new_id, .. } = retirement;
        match self {
            Self::IdTaken { holder, field } => format!(
                "cannot retire {old_id} for {new_id}: {} already holds \
                 {new_id} in its {}",
                holder.display(),
                field.frontmatter_key()
            ),
            Self::KeyLinked { holder } => format!(
                "cannot retire {old_id} for {new_id}: {} is already linked \
                 to {}",
                holder.display(),
                retirement.new_external_id.unwrap_or(new_id)
            ),
            Self::TargetExists(path) => format!(
                "cannot retire {old_id} for {new_id}: {} already exists",
                path.display()
            ),
            Self::ItemNotFound(id) => {
                format!("cannot retire {id}: no work item has that id")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetirementFailure {
    RolledBack {
        cause: RetirementCause,
    },
    RestoreIncomplete {
        cause: RetirementCause,
        unrestored: Vec<PathBuf>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementCause {
    pub path: PathBuf,
    pub kind: RetirementCauseKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetirementCauseKind {
    Store(StoreError),
    ChangedSinceSnapshot,
    TargetAppeared,
}

/// The reason keyword a retirement that could not restore the corpus is
/// reported under.
pub const RETIREMENT_INCOMPLETE: &str = "retirement-incomplete";

impl std::fmt::Display for RetirementCause {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path.display();
        match &self.kind {
            RetirementCauseKind::Store(error) => write!(formatter, "{error}"),
            RetirementCauseKind::ChangedSinceSnapshot => {
                write!(formatter, "{path} changed while retiring")
            }
            RetirementCauseKind::TargetAppeared => {
                write!(formatter, "{path} appeared while retiring")
            }
        }
    }
}

impl RetirementFailure {
    #[must_use]
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::RolledBack { .. } => "retirement-failed",
            Self::RestoreIncomplete { .. } => RETIREMENT_INCOMPLETE,
        }
    }

    /// `recovery_location` is where the recovery directory lives on disk.
    #[must_use]
    pub fn message(
        &self,
        retirement: &Retirement<'_>,
        recovery_location: &Path,
    ) -> String {
        let Retirement { old_id, new_id, .. } = retirement;
        match self {
            Self::RolledBack { cause } => format!(
                "retiring {old_id} as {new_id} was rolled back: {cause}"
            ),
            Self::RestoreIncomplete { cause, unrestored } => {
                let paths =
                    unrestored.iter().fold(String::new(), |listing, path| {
                        listing + "\n  " + &path.display().to_string()
                    });
                format!(
                    "{RETIREMENT_INCOMPLETE}: retiring {old_id} as {new_id} \
                     failed ({cause}) and these paths could not be \
                     restored:{paths}\nrecovery copies: {}\ncompare each \
                     path with its recovery copy and merge: restore these \
                     paths from version control, or from the recovery \
                     directory for files with uncommitted changes, then \
                     re-run",
                    recovery_location.display()
                )
            }
        }
    }
}

const fn same_id(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn is_interrupted_target(item: &ItemIdentity, r: &Retirement<'_>) -> bool {
    same_id(&item.id, r.new_id)
        && item.aliases.iter().any(|alias| same_id(alias, r.old_id))
}

fn shape_of(id: &str) -> IdShape {
    if DraftId::parse(id).is_some() || corpus::is_tracker_key(id) {
        IdShape::Distinctive
    } else {
        IdShape::NumericOnly
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The filename after its leading ID, so a retired file keeps its title.
fn slug_after(id: &str, path: &Path) -> String {
    let name = file_name(path);
    let stem = name.strip_suffix(".md").unwrap_or(&name);
    let without_id = stem
        .get(..id.len())
        .filter(|prefix| same_id(prefix, id))
        .map_or(stem, |_| &stem[id.len()..]);
    without_id.trim_start_matches('-').to_owned()
}

fn file_for(dir: &Path, id: &str, slug: &str) -> PathBuf {
    if slug.is_empty() {
        dir.join(format!("{id}.md"))
    } else {
        dir.join(format!("{id}-{slug}.md"))
    }
}

fn home_of(id: &str, work_dir: &Path) -> PathBuf {
    if DraftId::parse(id).is_some() {
        work_dir.join(DRAFTS_DIRECTORY)
    } else {
        work_dir.to_path_buf()
    }
}

fn path_spellings(
    from: &Path,
    to: &Path,
    work_dir: &Path,
) -> Vec<(String, String)> {
    let old_name = file_name(from);
    let new_name = file_name(to);
    let mut spellings = Vec::new();
    if from.parent() == Some(work_dir.join(DRAFTS_DIRECTORY).as_path()) {
        spellings
            .push((format!("{DRAFTS_DIRECTORY}/{old_name}"), new_name.clone()));
    }
    spellings.push((old_name, new_name));
    spellings
}

/// # Errors
///
/// A [`RetirementRefusal`] when no item holds `old_id`, another item already
/// holds `new_id` or is linked to `new_external_id`, or a file other than an
/// interrupted retirement's already occupies the target path.
pub fn plan_retirement(
    retirement: &Retirement<'_>,
    work_dir: &Path,
    items: &[ItemIdentity],
    corpus: &[CorpusFile],
) -> Result<RetirementPlan, RetirementRefusal> {
    let retiring = items
        .iter()
        .find(|item| same_id(&item.id, retirement.old_id));
    let interrupted = items
        .iter()
        .find(|item| is_interrupted_target(item, retirement));
    let from = match (retiring, interrupted) {
        (Some(item), _) => item.path.clone(),
        (None, Some(target)) => file_for(
            &home_of(retirement.old_id, work_dir),
            retirement.old_id,
            &slug_after(retirement.new_id, &target.path),
        ),
        (None, None) => {
            return Err(RetirementRefusal::ItemNotFound(
                retirement.old_id.to_owned(),
            ))
        }
    };
    refuse_collisions(retirement, items, retiring, interrupted)?;

    let to = interrupted.map_or_else(
        || {
            file_for(
                work_dir,
                retirement.new_id,
                &slug_after(retirement.old_id, &from),
            )
        },
        |target| target.path.clone(),
    );
    if interrupted.is_none() && corpus.iter().any(|file| file.path == to) {
        return Err(RetirementRefusal::TargetExists(to));
    }

    let renaming_spellings = path_spellings(&from, &to, work_dir);
    let renaming = Renaming {
        doc_type: WORK_ITEM,
        old_id: retirement.old_id,
        new_id: retirement.new_id,
        shape: shape_of(retirement.old_id),
        path_spellings: &renaming_spellings,
    };
    let from_file = corpus.iter().find(|file| file.path == from);
    let rewrites: Vec<FileRewrite> = corpus
        .iter()
        .filter(|file| file.path != from && file.path != to)
        .filter_map(|file| {
            let content = rewrite_review_link(
                &rewrite_references(&file.content, &renaming),
                retirement,
            );
            (content != file.content).then(|| FileRewrite {
                path: file.path.clone(),
                original: file.content.clone(),
                content,
            })
        })
        .collect();
    let retired_item = match (interrupted, from_file) {
        (None, Some(file)) => Some(retired_item_content(
            &rewrite_references(&file.content, &renaming),
            retirement,
        )),
        _ => None,
    };
    let recovery = corpus
        .iter()
        .filter(|file| {
            file.path == from
                || rewrites.iter().any(|rewrite| rewrite.path == file.path)
        })
        .filter(|file| file.dirtiness != Dirtiness::Clean)
        .map(|file| file.path.clone())
        .collect();

    Ok(RetirementPlan {
        from_original: from_file.map(|file| file.content.clone()),
        from,
        to,
        retired_item,
        rewrites,
        baseline_rename: BaselineRename {
            from: retirement.old_id.to_owned(),
            to: retirement.new_id.to_owned(),
        },
        recovery,
        recovery_dir: retirement.recovery_dir(),
        resumes: interrupted.is_some(),
    })
}

/// What the retiring item's file at `from` holds once retired: every
/// reference rewritten, and its `id`, `external_id`, H1 and `aliases` set.
#[must_use]
pub fn retired_content(
    content: &str,
    retirement: &Retirement<'_>,
    from: &Path,
    work_dir: &Path,
) -> String {
    let to = file_for(
        work_dir,
        retirement.new_id,
        &slug_after(retirement.old_id, from),
    );
    let spellings = path_spellings(from, &to, work_dir);
    let renaming = Renaming {
        doc_type: WORK_ITEM,
        old_id: retirement.old_id,
        new_id: retirement.new_id,
        shape: shape_of(retirement.old_id),
        path_spellings: &spellings,
    };
    retired_item_content(&rewrite_references(content, &renaming), retirement)
}

fn refuse_collisions(
    retirement: &Retirement<'_>,
    items: &[ItemIdentity],
    retiring: Option<&ItemIdentity>,
    interrupted: Option<&ItemIdentity>,
) -> Result<(), RetirementRefusal> {
    let is_party = |item: &ItemIdentity| {
        retiring == Some(item) || interrupted == Some(item)
    };
    let others: Vec<ItemIdentity> = items
        .iter()
        .filter(|item| !is_party(item))
        .cloned()
        .collect();
    if let Some(held) = holder_of(retirement.new_id, &others) {
        return Err(RetirementRefusal::IdTaken {
            holder: held.item.path.clone(),
            field: held.field,
        });
    }
    if let Some(linker) = retirement
        .new_external_id
        .and_then(|key| linker_of(key, &others))
    {
        return Err(RetirementRefusal::KeyLinked {
            holder: linker.path.clone(),
        });
    }
    Ok(())
}

fn frontmatter_range(lines: &[String]) -> Option<(usize, usize)> {
    let is_fence = |line: &String| line.trim_end() == "---";
    if !lines.first().is_some_and(is_fence) {
        return None;
    }
    let close = lines.iter().skip(1).position(is_fence)? + 1;
    Some((1, close))
}

fn key_of(line: &str) -> Option<&str> {
    let (key, _) = line.split_once(':')?;
    (!key.is_empty() && !key.starts_with(char::is_whitespace))
        .then_some(key.trim_end())
}

fn unquoted(value: &str) -> &str {
    value.trim().trim_matches(|c| c == '"' || c == '\'')
}

fn rewrite_review_link(content: &str, retirement: &Retirement<'_>) -> String {
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    let Some((start, end)) = frontmatter_range(&lines) else {
        return content.to_owned();
    };
    let is_review = lines[start..end].iter().any(|line| {
        key_of(line) == Some("type")
            && line
                .split_once(':')
                .is_some_and(|(_, value)| unquoted(value) == WORK_ITEM_REVIEW)
    });
    if !is_review {
        return content.to_owned();
    }
    for line in &mut lines[start..end] {
        let names_old_id = key_of(line) == Some("work_item_id")
            && line.split_once(':').is_some_and(|(_, value)| {
                same_id(unquoted(value), retirement.old_id)
            });
        if names_old_id {
            *line = format!("work_item_id: \"{}\"", retirement.new_id);
        }
    }
    rejoin(&lines, content)
}

fn rejoin(lines: &[String], original: &str) -> String {
    let mut joined = lines.join("\n");
    if original.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

fn position_of(
    lines: &[String],
    range: (usize, usize),
    key: &str,
) -> Option<usize> {
    (range.0..range.1).find(|&index| key_of(&lines[index]) == Some(key))
}

fn set_scalar(
    lines: &mut Vec<String>,
    range: &mut (usize, usize),
    key: &str,
    value: &str,
    after: &[&str],
) {
    let line = format!("{key}: \"{value}\"");
    if let Some(index) = position_of(lines, *range, key) {
        lines[index] = line;
        return;
    }
    let anchor = after
        .iter()
        .find_map(|anchor| position_of(lines, *range, anchor))
        .map_or(range.1, |index| index + 1);
    lines.insert(anchor, line);
    range.1 += 1;
}

fn quoted_list(items: &[String]) -> String {
    let quoted: Vec<String> =
        items.iter().map(|item| format!("\"{item}\"")).collect();
    format!("[{}]", quoted.join(", "))
}

fn append_alias(
    lines: &mut Vec<String>,
    range: &mut (usize, usize),
    old_id: &str,
) {
    let Some(index) = position_of(lines, *range, "aliases") else {
        set_list(lines, range, &[old_id.to_owned()]);
        return;
    };
    let inline = lines[index]
        .split_once(':')
        .map(|(_, value)| value.trim().to_owned())
        .unwrap_or_default();
    if inline.is_empty() {
        let continuation = (index + 1..range.1)
            .take_while(|&next| lines[next].starts_with(char::is_whitespace))
            .count();
        let held =
            lines[index + 1..=index + continuation].iter().any(|entry| {
                same_id(
                    unquoted(entry.trim_start().trim_start_matches('-')),
                    old_id,
                )
            });
        if !held {
            lines.insert(index + continuation + 1, format!("  - \"{old_id}\""));
            range.1 += 1;
        }
        return;
    }
    let mut aliases = crate::tags::parse_current_tags(&inline);
    if !aliases.iter().any(|alias| same_id(alias, old_id)) {
        aliases.push(old_id.to_owned());
    }
    lines[index] = format!("aliases: {}", quoted_list(&aliases));
}

fn set_list(
    lines: &mut Vec<String>,
    range: &mut (usize, usize),
    aliases: &[String],
) {
    let anchor = ["external_id", "id", "work_item_id"]
        .iter()
        .find_map(|anchor| position_of(lines, *range, anchor))
        .map_or(range.1, |index| index + 1);
    lines.insert(anchor, format!("aliases: {}", quoted_list(aliases)));
    range.1 += 1;
}

fn retitle(line: &str, retirement: &Retirement<'_>) -> Option<String> {
    let heading = line.strip_prefix("# ")?;
    let (prefix, title) = heading.split_once(": ")?;
    (same_id(prefix, retirement.old_id) || same_id(prefix, retirement.new_id))
        .then(|| format!("# {}: {title}", retirement.new_id))
}

fn retired_item_content(content: &str, retirement: &Retirement<'_>) -> String {
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    let Some(mut range) = frontmatter_range(&lines) else {
        return content.to_owned();
    };
    let id_key = if position_of(&lines, range, "id").is_none()
        && position_of(&lines, range, "work_item_id").is_some()
    {
        "work_item_id"
    } else {
        "id"
    };
    set_scalar(&mut lines, &mut range, id_key, retirement.new_id, &[]);
    if let Some(key) = retirement.new_external_id {
        set_scalar(&mut lines, &mut range, "external_id", key, &[id_key]);
    }
    append_alias(&mut lines, &mut range, retirement.old_id);
    if let Some(index) =
        (range.1..lines.len()).find(|&index| lines[index].starts_with("# "))
    {
        if let Some(retitled) = retitle(&lines[index], retirement) {
            lines[index] = retitled;
        }
    }
    rejoin(&lines, content)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::plan_retirement;
    use super::CorpusFile;
    use super::Retirement;
    use super::RetirementPlan;
    use super::RetirementRefusal;
    use crate::dirtiness::Dirtiness;
    use crate::identity::IdentityField;
    use crate::work_item_files::identities;
    use crate::work_item_files::WorkItemFile;

    const WORK_DIR: &str = "/repo/meta/work";

    fn work_dir() -> &'static Path {
        Path::new(WORK_DIR)
    }

    fn work_item(path: &str, frontmatter: &str, title: &str) -> CorpusFile {
        let id = frontmatter
            .lines()
            .find_map(|line| line.strip_prefix("id: "))
            .map(|id| id.trim_matches('"').to_owned())
            .unwrap_or_default();
        CorpusFile {
            path: PathBuf::from(path),
            content: format!("---\n{frontmatter}---\n\n# {id}: {title}\n"),
            dirtiness: Dirtiness::Clean,
        }
    }

    fn file(path: &str, content: &str) -> CorpusFile {
        CorpusFile {
            path: PathBuf::from(path),
            content: content.to_owned(),
            dirtiness: Dirtiness::Clean,
        }
    }

    fn draft() -> CorpusFile {
        work_item(
            "/repo/meta/work/drafts/draft-k7mq3x-add-search.md",
            "id: \"draft-k7mq3x\"\ntitle: \"Add search\"\n",
            "Add search",
        )
    }

    fn promotion() -> Retirement<'static> {
        Retirement {
            old_id: "draft-k7mq3x",
            new_id: "PP-900",
            new_external_id: Some("PP-900"),
        }
    }

    fn plan(
        retirement: &Retirement<'_>,
        corpus: &[CorpusFile],
    ) -> Result<RetirementPlan, RetirementRefusal> {
        let items = identities(
            &corpus
                .iter()
                .filter(|file| file.path.starts_with(WORK_DIR))
                .map(|file| WorkItemFile {
                    path: file.path.clone(),
                    content: file.content.clone(),
                })
                .collect::<Vec<_>>(),
        );
        plan_retirement(retirement, work_dir(), &items, corpus)
    }

    #[test]
    fn retired_content_is_what_the_plan_creates() {
        let corpus = [draft()];
        let planned = planned(&promotion(), &corpus);

        assert_eq!(
            planned.retired_item.as_deref(),
            Some(
                super::retired_content(
                    &corpus[0].content,
                    &promotion(),
                    &corpus[0].path,
                    work_dir()
                )
                .as_str()
            )
        );
    }

    fn planned(
        retirement: &Retirement<'_>,
        corpus: &[CorpusFile],
    ) -> RetirementPlan {
        match plan(retirement, corpus) {
            Ok(planned) => planned,
            Err(refusal) => unreachable!("refused: {refusal:?}"),
        }
    }

    fn retired(plan: &RetirementPlan) -> &str {
        plan.retired_item.as_deref().unwrap_or_default()
    }

    fn rewrite_of<'a>(plan: &'a RetirementPlan, path: &str) -> Option<&'a str> {
        plan.rewrites
            .iter()
            .find(|rewrite| rewrite.path == Path::new(path))
            .map(|rewrite| rewrite.content.as_str())
    }

    #[test]
    fn the_retired_items_file_takes_the_new_ids_filename_id_and_h1() {
        let corpus = [work_item(
            "/repo/meta/work/PP-760-follow-keys.md",
            "id: \"PP-760\"\nexternal_id: \"PP-760\"\n",
            "Follow keys",
        )];
        let retirement = Retirement {
            old_id: "PP-760",
            new_id: "ENG-42",
            new_external_id: Some("ENG-42"),
        };

        let planned = planned(&retirement, &corpus);

        assert_eq!(
            planned.from,
            Path::new("/repo/meta/work/PP-760-follow-keys.md")
        );
        assert_eq!(
            planned.to,
            Path::new("/repo/meta/work/ENG-42-follow-keys.md")
        );
        assert_eq!(
            retired(&planned),
            "---\nid: \"ENG-42\"\nexternal_id: \"ENG-42\"\n\
             aliases: [\"PP-760\"]\n---\n\n# ENG-42: Follow keys\n"
        );
        assert_eq!(
            planned.baseline_rename,
            super::BaselineRename {
                from: "PP-760".to_owned(),
                to: "ENG-42".to_owned(),
            }
        );
    }

    #[test]
    fn a_numeric_id_is_retitled_although_prose_is_left() {
        let corpus = [work_item(
            "/repo/meta/work/0230-tracker-ids.md",
            "id: \"0230\"\nexternal_id: \"PP-760\"\n",
            "Tracker ids",
        )];
        let retirement = Retirement {
            old_id: "0230",
            new_id: "PP-760",
            new_external_id: None,
        };

        let planned = planned(&retirement, &corpus);

        assert_eq!(
            retired(&planned),
            "---\nid: \"PP-760\"\nexternal_id: \"PP-760\"\n\
             aliases: [\"0230\"]\n---\n\n# PP-760: Tracker ids\n"
        );
    }

    #[test]
    fn the_old_id_is_appended_to_aliases() {
        let inline = work_item(
            "/repo/meta/work/PP-760-a.md",
            "id: \"PP-760\"\naliases: [\"0230\"]\n",
            "A",
        );
        let block = work_item(
            "/repo/meta/work/PP-760-a.md",
            "id: \"PP-760\"\naliases:\n  - \"0230\"\ntags: []\n",
            "A",
        );
        let retirement = Retirement {
            old_id: "PP-760",
            new_id: "ENG-42",
            new_external_id: None,
        };

        assert!(retired(&planned(&retirement, &[inline]))
            .contains("aliases: [\"0230\", \"PP-760\"]\n"));
        assert!(retired(&planned(&retirement, &[block]))
            .contains("aliases:\n  - \"0230\"\n  - \"PP-760\"\ntags: []\n"));
    }

    #[test]
    fn a_draft_moves_from_the_drafts_directory_to_the_work_directory() {
        let planned = planned(&promotion(), &[draft()]);

        assert_eq!(
            planned.from,
            Path::new("/repo/meta/work/drafts/draft-k7mq3x-add-search.md")
        );
        assert_eq!(
            planned.to,
            Path::new("/repo/meta/work/PP-900-add-search.md")
        );
    }

    #[test]
    fn a_new_external_id_is_set_when_given() {
        let planned = planned(&promotion(), &[draft()]);

        assert_eq!(
            retired(&planned),
            "---\nid: \"PP-900\"\nexternal_id: \"PP-900\"\n\
             aliases: [\"draft-k7mq3x\"]\ntitle: \"Add search\"\n---\n\n\
             # PP-900: Add search\n"
        );
    }

    #[test]
    fn references_across_the_corpus_are_rewritten_and_unchanged_files_left() {
        let corpus = [
            draft(),
            file(
                "/repo/meta/plans/p.md",
                "---\nparent: \"work-item:draft-k7mq3x\"\n---\nSee draft-k7mq3x.\n",
            ),
            file("/repo/meta/notes/n.md", "Nothing to see.\n"),
        ];

        let planned = planned(&promotion(), &corpus);

        assert_eq!(planned.rewrites.len(), 1);
        assert_eq!(
            rewrite_of(&planned, "/repo/meta/plans/p.md"),
            Some("---\nparent: \"work-item:PP-900\"\n---\nSee PP-900.\n")
        );
    }

    #[test]
    fn a_work_item_reviews_work_item_id_equal_to_the_old_id_is_rewritten() {
        let corpus = [
            work_item(
                "/repo/meta/work/0230-a.md",
                "id: \"0230\"\n",
                "A",
            ),
            file(
                "/repo/meta/reviews/work/r.md",
                "---\ntype: \"work-item-review\"\nwork_item_id: \"0230\"\n---\n\
                 Reviewed 0230.\n",
            ),
            file(
                "/repo/meta/plans/p.md",
                "---\ntype: \"plan\"\nwork_item_id: \"0230\"\n---\n",
            ),
        ];
        let retirement = Retirement {
            old_id: "0230",
            new_id: "PP-760",
            new_external_id: None,
        };

        let planned = planned(&retirement, &corpus);

        assert_eq!(
            rewrite_of(&planned, "/repo/meta/reviews/work/r.md"),
            Some(
                "---\ntype: \"work-item-review\"\nwork_item_id: \"PP-760\"\n\
                 ---\nReviewed 0230.\n"
            )
        );
        assert_eq!(rewrite_of(&planned, "/repo/meta/plans/p.md"), None);
    }

    #[test]
    fn a_path_reference_to_the_retired_file_is_rewritten() {
        let corpus = [
            work_item("/repo/meta/work/PP-760-a.md", "id: \"PP-760\"\n", "A"),
            file(
                "/repo/meta/plans/p.md",
                "[a](meta/work/PP-760-a.md) and [b](../work/PP-760-a.md)\n",
            ),
        ];
        let retirement = Retirement {
            old_id: "PP-760",
            new_id: "ENG-42",
            new_external_id: None,
        };

        let planned = planned(&retirement, &corpus);

        assert_eq!(
            rewrite_of(&planned, "/repo/meta/plans/p.md"),
            Some("[a](meta/work/ENG-42-a.md) and [b](../work/ENG-42-a.md)\n")
        );
    }

    #[test]
    fn a_relative_drafts_link_is_rewritten_to_the_work_directory() {
        let corpus = [
            draft(),
            file(
                "/repo/meta/plans/p.md",
                "[a](meta/work/drafts/draft-k7mq3x-add-search.md) \
                 [b](../work/drafts/draft-k7mq3x-add-search.md)\n",
            ),
        ];

        let planned = planned(&promotion(), &corpus);

        assert_eq!(
            rewrite_of(&planned, "/repo/meta/plans/p.md"),
            Some(
                "[a](meta/work/PP-900-add-search.md) \
                 [b](../work/PP-900-add-search.md)\n"
            )
        );
    }

    #[test]
    fn retirement_refuses_when_the_new_id_is_another_items_id() {
        let corpus = [
            draft(),
            work_item(
                "/repo/meta/work/PP-900-other.md",
                "id: \"PP-900\"\n",
                "O",
            ),
        ];

        assert_eq!(
            plan(&promotion(), &corpus),
            Err(RetirementRefusal::IdTaken {
                holder: PathBuf::from("/repo/meta/work/PP-900-other.md"),
                field: IdentityField::Id,
            })
        );
    }

    #[test]
    fn retirement_refuses_when_the_new_id_is_another_items_alias() {
        let corpus = [
            draft(),
            work_item(
                "/repo/meta/work/ENG-1-other.md",
                "id: \"ENG-1\"\naliases: [\"pp-900\"]\n",
                "O",
            ),
        ];

        assert_eq!(
            plan(&promotion(), &corpus),
            Err(RetirementRefusal::IdTaken {
                holder: PathBuf::from("/repo/meta/work/ENG-1-other.md"),
                field: IdentityField::Alias,
            })
        );
    }

    #[test]
    fn retirement_refuses_when_the_new_external_id_is_linked_by_another_item() {
        let corpus = [
            draft(),
            work_item(
                "/repo/meta/work/0007-other.md",
                "id: \"0007\"\nexternal_id: \"PP-900\"\n",
                "O",
            ),
        ];

        assert_eq!(
            plan(&promotion(), &corpus),
            Err(RetirementRefusal::KeyLinked {
                holder: PathBuf::from("/repo/meta/work/0007-other.md"),
            })
        );
    }

    #[test]
    fn retirement_refuses_when_the_target_path_exists() {
        let corpus = [
            draft(),
            file("/repo/meta/work/PP-900-add-search.md", "# Not an item\n"),
        ];

        assert_eq!(
            plan(&promotion(), &corpus),
            Err(RetirementRefusal::TargetExists(PathBuf::from(
                "/repo/meta/work/PP-900-add-search.md"
            )))
        );
    }

    #[test]
    fn a_target_holding_the_new_id_without_the_old_alias_is_still_refused() {
        let corpus = [
            draft(),
            work_item(
                "/repo/meta/work/PP-900-add-search.md",
                "id: \"PP-900\"\naliases: [\"draft-zzzzzz\"]\n",
                "Add search",
            ),
        ];

        assert!(matches!(
            plan(&promotion(), &corpus),
            Err(RetirementRefusal::IdTaken { .. })
        ));
    }

    #[test]
    fn an_unknown_old_id_is_refused() {
        assert_eq!(
            plan(&promotion(), &[]),
            Err(RetirementRefusal::ItemNotFound("draft-k7mq3x".to_owned()))
        );
    }

    #[test]
    fn each_refusal_names_both_items() {
        let holder = PathBuf::from("/repo/meta/work/PP-900-other.md");
        let refusals = [
            RetirementRefusal::IdTaken {
                holder: holder.clone(),
                field: IdentityField::Id,
            },
            RetirementRefusal::KeyLinked {
                holder: holder.clone(),
            },
            RetirementRefusal::TargetExists(holder),
        ];

        for refusal in refusals {
            let message = refusal.message(&promotion());
            assert!(message.contains("draft-k7mq3x"), "{message}");
            assert!(message.contains("PP-900-other.md"), "{message}");
        }
    }

    fn interrupted_target() -> CorpusFile {
        work_item(
            "/repo/meta/work/PP-900-add-search.md",
            "id: \"PP-900\"\nexternal_id: \"PP-900\"\n\
             aliases: [\"draft-k7mq3x\"]\n",
            "Add search",
        )
    }

    fn referencing(content: &str) -> CorpusFile {
        file("/repo/meta/plans/p.md", content)
    }

    #[test]
    fn an_interrupted_target_is_exempt_from_the_collision_checks() {
        let corpus = [draft(), interrupted_target()];

        let planned = planned(&promotion(), &corpus);

        assert!(planned.resumes);
        assert_eq!(planned.retired_item, None);
    }

    #[test]
    fn an_interrupted_retirement_is_completed_by_the_next_retirement() {
        let unrewritten = referencing("See draft-k7mq3x.\n");
        let rewritten = referencing("See PP-900.\n");
        let stages = [
            (
                "after the rewrites",
                vec![draft(), rewritten.clone()],
                true,
                false,
            ),
            (
                "after writing the target",
                vec![draft(), interrupted_target(), unrewritten],
                true,
                true,
            ),
            (
                "after removing the draft",
                vec![interrupted_target(), rewritten],
                false,
                true,
            ),
        ];

        for (stage, corpus, draft_remains, target_written) in stages {
            let planned = planned(&promotion(), &corpus);

            assert_eq!(
                planned.from_original.is_some(),
                draft_remains,
                "{stage}"
            );
            assert_eq!(
                planned.retired_item.is_none(),
                target_written,
                "{stage}"
            );
            assert_eq!(planned.resumes, target_written, "{stage}");
            let expected_rewrites = usize::from(
                corpus
                    .iter()
                    .any(|file| file.content == "See draft-k7mq3x.\n"),
            );
            assert_eq!(planned.rewrites.len(), expected_rewrites, "{stage}");
            assert_eq!(
                planned.from,
                Path::new("/repo/meta/work/drafts/draft-k7mq3x-add-search.md"),
                "{stage}"
            );
            assert_eq!(
                planned.to,
                Path::new("/repo/meta/work/PP-900-add-search.md"),
                "{stage}"
            );
        }
    }

    #[test]
    fn a_completed_retirement_leaves_the_corpus_unchanged() {
        let finished = vec![interrupted_target(), referencing("See PP-900.\n")];
        let unfinished =
            vec![interrupted_target(), referencing("See draft-k7mq3x.\n")];

        assert!(planned(&promotion(), &finished).leaves_corpus_unchanged());
        assert!(!planned(&promotion(), &unfinished).leaves_corpus_unchanged());
    }

    #[test]
    fn a_record_carries_its_retirement_back() {
        let record = super::RetirementRecord::of(&promotion());

        assert_eq!(record.retirement(), promotion());
        assert_eq!(record.recovery_dir, promotion().recovery_dir());
    }

    #[test]
    fn plan_retirement_marks_dirty_and_unknown_paths_for_recovery_and_not_clean_ones(
    ) {
        let mut dirty_draft = draft();
        dirty_draft.dirtiness = Dirtiness::Dirty;
        let mut unknown = referencing("See draft-k7mq3x.\n");
        unknown.dirtiness = Dirtiness::Unknown;
        let clean = file("/repo/meta/notes/n.md", "Also draft-k7mq3x.\n");
        let mut untouched = file("/repo/meta/notes/u.md", "Nothing.\n");
        untouched.dirtiness = Dirtiness::Dirty;

        let planned =
            planned(&promotion(), &[dirty_draft, unknown, clean, untouched]);

        assert_eq!(
            planned.recovery,
            vec![
                PathBuf::from(
                    "/repo/meta/work/drafts/draft-k7mq3x-add-search.md"
                ),
                PathBuf::from("/repo/meta/plans/p.md"),
            ]
        );
    }

    #[test]
    fn an_incomplete_restore_names_both_ids_every_unrestored_path_the_recovery_directory_and_the_remedy(
    ) {
        let failure = super::RetirementFailure::RestoreIncomplete {
            cause: super::RetirementCause {
                path: PathBuf::from("/repo/meta/work/PP-900-add-search.md"),
                kind: super::RetirementCauseKind::TargetAppeared,
            },
            unrestored: vec![
                PathBuf::from("/repo/meta/plans/p.md"),
                PathBuf::from("/repo/meta/work/0001-child.md"),
            ],
        };

        let message = failure.message(
            &promotion(),
            Path::new("/repo/.accelerator/state/retirement-recovery/x"),
        );

        for expected in [
            "retirement-incomplete",
            "draft-k7mq3x",
            "PP-900",
            "/repo/meta/plans/p.md",
            "/repo/meta/work/0001-child.md",
            "/repo/.accelerator/state/retirement-recovery/x",
            "restore these paths from version control",
            "then re-run",
        ] {
            assert!(message.contains(expected), "{expected}: {message}");
        }
    }

    #[test]
    fn the_recovery_directory_is_named_by_both_ids() {
        assert_eq!(
            promotion().recovery_dir(),
            Path::new("retirement-recovery/draft-k7mq3x--PP-900")
        );
    }
}

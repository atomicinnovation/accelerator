//! `accelerator work list`: scan, filter, classify and render work items.
//!
//! Read-only. The sync-status column reuses the sync engine's classifier
//! (`work::sync::plan` over `fetch::gather`) so the five rendered states are
//! a presentation mapping over one source of truth, never a second one.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use corpus_adapters::canonicalise_id;
use tracker::ExternalId;
use tracker::RemoteTracker;
use work::show::read_field_raw;
use work::sync::label;
use work::sync::plan;
use work::sync::RenderableState;
use work::sync::SyncDirection;
use work::sync::SyncState;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::digest::split_frontmatter_and_body;
use work_adapters::sync::digest::LazyItemDigests;
use work_adapters::sync::fetch;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;

/// One scanned, well-formed work item with the fields the listing renders
/// plus the identifiers the classifier keys on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedItem {
    pub id: String,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub tags: Vec<String>,
    pub parent: Option<String>,
    pub path: PathBuf,
    pub external_id: Option<ExternalId>,
}

/// The scan result: every well-formed work item, plus one warning line per
/// malformed file skipped along the way.
pub struct Scan {
    pub items: Vec<ScannedItem>,
    pub warnings: Vec<String>,
}

/// The conjunctive filter parsed from the command flags. Every populated
/// field must hold for an item to be listed.
#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub status: Option<String>,
    pub kind: Option<String>,
    pub priority: Option<String>,
    pub parent: Option<String>,
    pub tags: Vec<String>,
    pub term: Option<String>,
}

impl Filter {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.kind.is_none()
            && self.priority.is_none()
            && self.parent.is_none()
            && self.tags.is_empty()
            && self.term.is_none()
    }

    const fn is_title_only(&self) -> bool {
        self.term.is_some()
            && self.status.is_none()
            && self.kind.is_none()
            && self.priority.is_none()
            && self.parent.is_none()
            && self.tags.is_empty()
    }
}

fn strip_frontmatter_ref(raw: &str) -> &str {
    raw.split_once(':').map_or(raw, |(_, rest)| rest).trim()
}

fn canonical_ref(raw: &str, pattern: &str, project: &str) -> String {
    let bare = strip_frontmatter_ref(raw);
    canonicalise_id(bare, pattern, project).unwrap_or_else(|_| bare.to_owned())
}

enum Frontmatter {
    Found(String),
    Missing,
    Unclosed,
}

fn frontmatter_of(content: &str) -> Frontmatter {
    match split_frontmatter_and_body(content) {
        Ok((frontmatter, _)) => Frontmatter::Found(frontmatter),
        Err(_) if content.lines().next().is_some_and(is_fence) => {
            Frontmatter::Unclosed
        }
        Err(_) => Frontmatter::Missing,
    }
}

fn is_fence(line: &str) -> bool {
    line.starts_with("---") && line[3..].chars().all(char::is_whitespace)
}

fn nonempty_external_id(frontmatter: &str) -> Option<ExternalId> {
    read_field_raw(frontmatter, "external_id")
        .filter(|raw| !raw.trim().is_empty())
        .map(ExternalId::new)
}

fn scanned_item(path: PathBuf, frontmatter: &str) -> Option<ScannedItem> {
    let id = read_field_raw(frontmatter, "id")
        .or_else(|| read_field_raw(frontmatter, "work_item_id"))
        .filter(|raw| !raw.trim().is_empty())?;
    let tags = read_field_raw(frontmatter, "tags")
        .map(|raw| work::tags::parse_current_tags(&raw))
        .unwrap_or_default();
    Some(ScannedItem {
        id,
        title: read_field_raw(frontmatter, "title"),
        kind: read_field_raw(frontmatter, "kind"),
        status: read_field_raw(frontmatter, "status"),
        priority: read_field_raw(frontmatter, "priority"),
        tags,
        parent: read_field_raw(frontmatter, "parent")
            .filter(|raw| !raw.trim().is_empty()),
        external_id: nonempty_external_id(frontmatter),
        path,
    })
}

/// Scans `work_dir` for `*.md` work items.
///
/// A file with no opening fence or an unclosed one yields a skip warning; a
/// well-formed file with no `id`/`work_item_id` is silently excluded (it is
/// simply not a work item). Items are sorted by id.
#[must_use]
pub fn scan(work_dir: &Path) -> Scan {
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    let Ok(entries) = std::fs::read_dir(work_dir) else {
        return Scan { items, warnings };
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(std::ffi::OsStr::to_str) == Some("md")
        })
        .collect();
    paths.sort();
    for path in paths {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        match frontmatter_of(&content) {
            Frontmatter::Missing => {
                warnings.push(format!("{name}: skipped — no frontmatter"));
            }
            Frontmatter::Unclosed => {
                warnings
                    .push(format!("{name}: skipped — unclosed frontmatter"));
            }
            Frontmatter::Found(frontmatter) => {
                if let Some(item) = scanned_item(path, &frontmatter) {
                    items.push(item);
                }
            }
        }
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    Scan { items, warnings }
}

/// Keeps the items satisfying every populated filter conjunct.
#[must_use]
pub fn apply_filter<'a>(
    items: &'a [ScannedItem],
    filter: &Filter,
    pattern: &str,
    project: &str,
) -> Vec<&'a ScannedItem> {
    let wanted_parent = filter
        .parent
        .as_deref()
        .map(|raw| canonical_ref(raw, pattern, project));
    items
        .iter()
        .filter(|item| {
            matches_scalar(filter.status.as_deref(), item.status.as_deref())
                && matches_scalar(filter.kind.as_deref(), item.kind.as_deref())
                && matches_scalar(
                    filter.priority.as_deref(),
                    item.priority.as_deref(),
                )
                && matches_parent(
                    wanted_parent.as_deref(),
                    item.parent.as_deref(),
                    pattern,
                    project,
                )
                && filter.tags.iter().all(|tag| item.tags.contains(tag))
                && matches_title(filter.term.as_deref(), item.title.as_deref())
        })
        .collect()
}

fn matches_scalar(wanted: Option<&str>, actual: Option<&str>) -> bool {
    wanted.is_none_or(|value| actual == Some(value))
}

fn matches_parent(
    wanted: Option<&str>,
    actual: Option<&str>,
    pattern: &str,
    project: &str,
) -> bool {
    wanted.is_none_or(|value| {
        actual.is_some_and(|raw| canonical_ref(raw, pattern, project) == value)
    })
}

fn matches_title(wanted: Option<&str>, actual: Option<&str>) -> bool {
    wanted.is_none_or(|term| {
        actual.is_some_and(|title| {
            title.to_lowercase().contains(&term.to_lowercase())
        })
    })
}

/// Classifies every item's sync state through the sync engine, keyed by id.
///
/// A classifier read failure leaves an item out of the map; the caller then
/// falls back to the presence-only label. Returns an empty map when the
/// plan cannot be built at all.
#[must_use]
pub fn classify(
    items: &[LocalItem],
    tracker: &dyn RemoteTracker,
    baseline: &Baseline,
    status: &dyn WorkingCopyStatus,
) -> BTreeMap<String, SyncState> {
    let facts = fetch::gather(
        items,
        baseline,
        tracker,
        status,
        RetrievalStrategy::Bulk,
    );
    let digests: Vec<LazyItemDigests<'_>> = items
        .iter()
        .map(|item| {
            let remote_content = facts
                .per_id
                .get(&item.id)
                .and_then(|(remote, _)| remote.body.clone());
            LazyItemDigests::new(&item.path, remote_content)
        })
        .collect();
    let inputs =
        facts.plan_inputs(items, &digests, baseline, baseline.timestamp());
    match plan::plan(&inputs, SyncDirection::Bidirectional, &BTreeMap::new()) {
        Ok(computed) => computed
            .actions
            .into_iter()
            .map(|action| (action.id, action.state))
            .collect(),
        Err(_) => BTreeMap::new(),
    }
}

/// The rendered `<glyph> <text>` label for one item, mapping a classified
/// state to its glyph and falling back to the presence-only label when the
/// state carries none (`remote-absent`/`indeterminate`) or was never
/// classified.
#[must_use]
pub fn sync_label(
    state: Option<SyncState>,
    external_id: Option<&ExternalId>,
) -> &'static str {
    state
        .and_then(|state| RenderableState::try_from(state).ok())
        .map_or_else(|| presence_label(external_id), label)
}

const fn presence_label(external_id: Option<&ExternalId>) -> &'static str {
    if external_id.is_some() {
        label(RenderableState::Synced)
    } else {
        label(RenderableState::Unsynced)
    }
}

const ABSENT: &str = "—";

fn cell(value: Option<&str>) -> &str {
    value.filter(|raw| !raw.is_empty()).unwrap_or(ABSENT)
}

/// Renders the filtered items as a markdown table. A column that is `—` for
/// every row is suppressed (except ID). The Sync column appears only when a
/// label map is supplied.
#[must_use]
pub fn render_table(
    items: &[&ScannedItem],
    labels: Option<&BTreeMap<String, &'static str>>,
) -> String {
    let show_title = items.iter().any(|item| item.title.is_some());
    let show_kind = items.iter().any(|item| item.kind.is_some());
    let show_status = items.iter().any(|item| item.status.is_some());
    let show_priority = items.iter().any(|item| item.priority.is_some());

    let mut header = vec!["ID"];
    if show_title {
        header.push("Title");
    }
    if show_kind {
        header.push("Kind");
    }
    if show_status {
        header.push("Status");
    }
    if show_priority {
        header.push("Priority");
    }
    if labels.is_some() {
        header.push("Sync");
    }

    let mut lines = Vec::with_capacity(items.len() + 2);
    lines.push(format!("| {} |", header.join(" | ")));
    lines.push(format!(
        "|{}|",
        header.iter().map(|_| " --- ").collect::<Vec<_>>().join("|")
    ));
    for item in items {
        let mut row = vec![item.id.clone()];
        if show_title {
            row.push(cell(item.title.as_deref()).to_owned());
        }
        if show_kind {
            row.push(cell(item.kind.as_deref()).to_owned());
        }
        if show_status {
            row.push(cell(item.status.as_deref()).to_owned());
        }
        if show_priority {
            row.push(cell(item.priority.as_deref()).to_owned());
        }
        if let Some(labels) = labels {
            row.push((*labels.get(&item.id).unwrap_or(&ABSENT)).to_owned());
        }
        lines.push(format!("| {} |", row.join(" | ")));
    }
    lines.join("\n")
}

fn tree_line(item: &ScannedItem, label: Option<&&'static str>) -> String {
    let kind = cell(item.kind.as_deref());
    let status = cell(item.status.as_deref());
    let title = cell(item.title.as_deref());
    let base =
        format!("{} — {title} (kind: {kind}, status: {status})", item.id);
    match label {
        Some(label) => format!("{base} {label}"),
        None => base,
    }
}

/// Renders the parent/child tree. Items whose parent is outside the result
/// set surface at the top level (with a `(parent … not found)` suffix); a
/// parent cycle renders its members flat with a `(cycle)` marker so the walk
/// always terminates.
#[must_use]
pub fn render_hierarchy(
    items: &[&ScannedItem],
    labels: Option<&BTreeMap<String, &'static str>>,
    pattern: &str,
    project: &str,
) -> String {
    let canonical_id: BTreeMap<String, &ScannedItem> = items
        .iter()
        .map(|item| (canonical_ref(&item.id, pattern, project), *item))
        .collect();
    let parent_of = |item: &ScannedItem| -> Option<String> {
        item.parent
            .as_deref()
            .map(|raw| canonical_ref(raw, pattern, project))
            .filter(|key| canonical_id.contains_key(key))
    };

    let in_cycle = cyclic_members(items, pattern, project, &canonical_id);
    let children: BTreeMap<String, Vec<&ScannedItem>> =
        child_index(items, pattern, project, &canonical_id, &in_cycle);

    let mut lines = Vec::new();
    for item in items {
        let key = canonical_ref(&item.id, pattern, project);
        if in_cycle.contains(&key) {
            lines.push(format!(
                "{} (cycle)",
                tree_line(item, labels.and_then(|map| map.get(&item.id)))
            ));
            continue;
        }
        let parent = parent_of(item);
        if parent.is_none() {
            if let Some(raw) = &item.parent {
                if !in_cycle.contains(&key) {
                    lines.push(format!(
                        "{} (parent {} not found)",
                        tree_line(
                            item,
                            labels.and_then(|map| map.get(&item.id))
                        ),
                        strip_frontmatter_ref(raw)
                    ));
                    render_children(
                        &key, &children, labels, &mut lines, 1, pattern,
                        project,
                    );
                    continue;
                }
            }
            lines.push(tree_line(
                item,
                labels.and_then(|map| map.get(&item.id)),
            ));
            render_children(
                &key, &children, labels, &mut lines, 1, pattern, project,
            );
        }
    }
    lines.join("\n")
}

fn cyclic_members(
    items: &[&ScannedItem],
    pattern: &str,
    project: &str,
    index: &BTreeMap<String, &ScannedItem>,
) -> BTreeSet<String> {
    let mut cyclic = BTreeSet::new();
    for item in items {
        let mut seen = BTreeSet::new();
        let mut cursor = canonical_ref(&item.id, pattern, project);
        loop {
            if !seen.insert(cursor.clone()) {
                cyclic.insert(canonical_ref(&item.id, pattern, project));
                break;
            }
            let Some(current) = index.get(&cursor) else {
                break;
            };
            let Some(parent) = current
                .parent
                .as_deref()
                .map(|raw| canonical_ref(raw, pattern, project))
                .filter(|key| index.contains_key(key))
            else {
                break;
            };
            cursor = parent;
        }
    }
    cyclic
}

fn child_index<'a>(
    items: &[&'a ScannedItem],
    pattern: &str,
    project: &str,
    index: &BTreeMap<String, &ScannedItem>,
    in_cycle: &BTreeSet<String>,
) -> BTreeMap<String, Vec<&'a ScannedItem>> {
    let mut children: BTreeMap<String, Vec<&ScannedItem>> = BTreeMap::new();
    for item in items {
        let key = canonical_ref(&item.id, pattern, project);
        if in_cycle.contains(&key) {
            continue;
        }
        if let Some(parent) = item
            .parent
            .as_deref()
            .map(|raw| canonical_ref(raw, pattern, project))
            .filter(|parent| {
                index.contains_key(parent) && !in_cycle.contains(parent)
            })
        {
            children.entry(parent).or_default().push(item);
        }
    }
    children
}

fn render_children(
    parent: &str,
    children: &BTreeMap<String, Vec<&ScannedItem>>,
    labels: Option<&BTreeMap<String, &'static str>>,
    lines: &mut Vec<String>,
    depth: usize,
    pattern: &str,
    project: &str,
) {
    let Some(group) = children.get(parent) else {
        return;
    };
    let indent = "  ".repeat(depth);
    for (position, child) in group.iter().enumerate() {
        let connector = if position + 1 == group.len() {
            "└── "
        } else {
            "├── "
        };
        lines.push(format!(
            "{indent}{connector}{}",
            tree_line(child, labels.and_then(|map| map.get(&child.id)))
        ));
        let key = canonical_ref(&child.id, pattern, project);
        render_children(
            &key,
            children,
            labels,
            lines,
            depth + 1,
            pattern,
            project,
        );
    }
}

fn conjuncts(filter: &Filter) -> Vec<String> {
    let mut parts = Vec::new();
    if let Some(status) = &filter.status {
        parts.push(format!("status={status}"));
    }
    if let Some(kind) = &filter.kind {
        parts.push(format!("kind={kind}"));
    }
    if let Some(priority) = &filter.priority {
        parts.push(format!("priority={priority}"));
    }
    for tag in &filter.tags {
        parts.push(format!("tag={tag}"));
    }
    if let Some(term) = &filter.term {
        parts.push(format!("title~{term}"));
    }
    parts
}

/// The one-line filter echo printed above the results.
#[must_use]
pub fn filter_echo(filter: &Filter, count: usize) -> String {
    if filter.is_empty() {
        return format!("All work items ({count} total)");
    }
    if let Some(parent) = &filter.parent {
        if conjuncts(filter).is_empty() {
            return format!(
                "Children of {parent} ({count} matches)",
                parent = strip_frontmatter_ref(parent)
            );
        }
    }
    let mut parts = conjuncts(filter);
    if let Some(parent) = &filter.parent {
        parts.insert(0, format!("parent={}", strip_frontmatter_ref(parent)));
    }
    format!("Filter: {} ({count} matches)", parts.join(", "))
}

/// The empty-result message, with the free-text tip appended when the only
/// filter was a title search.
#[must_use]
pub fn empty_message(filter: &Filter) -> String {
    let description = if filter.is_empty() {
        "all work items".to_owned()
    } else if let Some(parent) = &filter.parent {
        if conjuncts(filter).is_empty() {
            format!("children of {}", strip_frontmatter_ref(parent))
        } else {
            conjuncts(filter).join(", ")
        }
    } else {
        conjuncts(filter).join(", ")
    };
    let mut message = format!("No work items matched: {description}.");
    if filter.is_title_only() {
        message.push_str(
            "\nTip: to filter by field value, use `status <value>`, \
             `kind <value>`, or `tagged <value>`.",
        );
    }
    message
}

/// The directory-not-found guidance.
#[must_use]
pub fn missing_directory_message(work_dir: &Path) -> String {
    format!(
        "Work items directory `{}` not found. Check the `paths.work` \
         configuration or run `/create-work-item` to create the first work \
         item.",
        work_dir.display()
    )
}

/// The no-items-at-all message.
#[must_use]
pub fn empty_directory_message(work_dir: &Path) -> String {
    format!("No work items found in `{}`.", work_dir.display())
}

fn local_items(items: &[ScannedItem]) -> Vec<LocalItem> {
    items
        .iter()
        .map(|item| LocalItem {
            id: item.id.clone(),
            path: item.path.clone(),
            external_id: item.external_id.clone(),
        })
        .collect()
}

/// Builds the id → label map for the Sync column.
///
/// With no baseline on disk, every item is presence-only (no remote read),
/// matching the skill's floor. With a baseline, the classifier upgrades
/// tracked items to the five-state vocabulary; `remote-absent`/
/// `indeterminate` degrade back to presence-only.
#[must_use]
pub fn sync_column(
    items: &[ScannedItem],
    states: &BTreeMap<String, SyncState>,
) -> BTreeMap<String, &'static str> {
    items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                sync_label(
                    states.get(&item.id).copied(),
                    item.external_id.as_ref(),
                ),
            )
        })
        .collect()
}

/// Assembles the full rendered listing for the resolved and filtered items.
#[must_use]
pub fn render(
    items: &[ScannedItem],
    filter: &Filter,
    labels: Option<&BTreeMap<String, &'static str>>,
    hierarchy: bool,
    scheme: (&str, &str),
) -> String {
    let (pattern, project) = scheme;
    let selected = apply_filter(items, filter, pattern, project);
    if selected.is_empty() {
        return empty_message(filter);
    }
    let echo = filter_echo(filter, selected.len());
    let body = if hierarchy {
        render_hierarchy(&selected, labels, pattern, project)
    } else {
        render_table(&selected, labels)
    };
    format!("{echo}\n{body}")
}

/// Resolves configuration, scans, classifies and renders the listing.
#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    registry: &dyn crate::tracker_registry::TrackerRegistry,
    args: &crate::cli::ListArgs,
) -> RunOutcome {
    let root = config_adapters::FileConfigStore::discover_root(start);
    let work_dir = match crate::config::resolve_work_dir(config, &root) {
        Ok(dir) => dir,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    if !work_dir.is_dir() {
        return RunOutcome::MissingDirectory {
            message: missing_directory_message(&work_dir),
        };
    }

    let pattern = crate::config::effective_nonempty(config, "work.id_pattern")
        .unwrap_or_default();
    let project =
        crate::config::effective_nonempty(config, "work.default_project_code")
            .unwrap_or_default();

    let scan_result = scan(&work_dir);
    if scan_result.items.is_empty() {
        return RunOutcome::EmptyDirectory {
            message: empty_directory_message(&work_dir),
            warnings: scan_result.warnings,
        };
    }

    let filter = Filter {
        status: args.status.clone(),
        kind: args.kind.clone(),
        priority: args.priority.clone(),
        parent: args.parent.clone(),
        tags: args.tags.clone(),
        term: args.term.clone(),
    };

    let labels = classify_labels(config, &root, registry, &scan_result.items);

    let output = render(
        &scan_result.items,
        &filter,
        labels.as_ref(),
        args.hierarchy,
        (&pattern, &project),
    );
    RunOutcome::Rendered {
        output,
        warnings: scan_result.warnings,
    }
}

fn classify_labels(
    config: &dyn ConfigAccess,
    root: &Path,
    registry: &dyn crate::tracker_registry::TrackerRegistry,
    items: &[ScannedItem],
) -> Option<BTreeMap<String, &'static str>> {
    let integration =
        crate::config::effective_nonempty(config, "work.integration")
            .unwrap_or_default();
    if integration.is_empty() {
        return None;
    }
    let integrations_root = crate::sync::integrations_dir(config, root).ok()?;
    let baseline_path =
        work_adapters::sync::baseline::path(&integrations_root, &integration);

    let states = if baseline_path.is_file() {
        resolve_states(registry, &integration, &baseline_path, items)
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    Some(sync_column(items, &states))
}

fn resolve_states(
    registry: &dyn crate::tracker_registry::TrackerRegistry,
    integration: &str,
    baseline_path: &Path,
    items: &[ScannedItem],
) -> Option<BTreeMap<String, SyncState>> {
    use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;
    let tracker = registry.resolve(integration).ok()?;
    let content = std::fs::read_to_string(baseline_path).ok();
    let (baseline, _) = Baseline::read(content.as_deref());
    let root = baseline_path.parent()?;
    let status = VcsWorkingCopyStatus::probed_from(root);
    let locals = local_items(items);
    Some(classify(&locals, tracker.as_ref(), &baseline, &status))
}

pub enum RunOutcome {
    Rendered {
        output: String,
        warnings: Vec<String>,
    },
    EmptyDirectory {
        message: String,
        warnings: Vec<String>,
    },
    MissingDirectory {
        message: String,
    },
    Failed(String),
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use tracker::RemoteIssue;
    use tracker::RemoteTimestamp;
    use tracker_test_support::RecordingTracker;
    use work::sync::Dirtiness;
    use work_adapters::sync::baseline::Entry;

    use super::*;

    const PATTERN: &str = "{number:04d}";

    struct AlwaysClean;
    impl WorkingCopyStatus for AlwaysClean {
        fn is_dirty(&self, _path: &Path) -> Dirtiness {
            Dirtiness::Clean
        }
    }

    fn item(id: &str) -> ScannedItem {
        ScannedItem {
            id: id.to_owned(),
            title: Some(format!("Item {id}")),
            kind: Some("story".to_owned()),
            status: Some("draft".to_owned()),
            priority: Some("medium".to_owned()),
            tags: Vec::new(),
            parent: None,
            path: PathBuf::from(format!("/items/{id}.md")),
            external_id: None,
        }
    }

    fn with_external(mut item: ScannedItem, external_id: &str) -> ScannedItem {
        item.external_id = Some(ExternalId::new(external_id.to_owned()));
        item
    }

    #[test]
    fn sync_label_maps_every_state_to_the_skill_vocabulary() {
        let id = ExternalId::new("ENG-1".to_owned());
        assert_eq!(sync_label(Some(SyncState::Synced), Some(&id)), "🟢 synced");
        assert_eq!(sync_label(Some(SyncState::Unsynced), None), "⚪ unsynced");
        assert_eq!(
            sync_label(Some(SyncState::LocallyModified), Some(&id)),
            "🔵 locally modified"
        );
        assert_eq!(
            sync_label(Some(SyncState::RemotelyModified), Some(&id)),
            "🟣 remotely modified"
        );
        assert_eq!(
            sync_label(Some(SyncState::Conflict), Some(&id)),
            "🔴 conflict"
        );
    }

    #[test]
    fn remote_absent_and_indeterminate_degrade_to_presence_only() {
        let id = ExternalId::new("ENG-1".to_owned());
        assert_eq!(
            sync_label(Some(SyncState::RemoteAbsent), Some(&id)),
            "🟢 synced"
        );
        assert_eq!(
            sync_label(Some(SyncState::Indeterminate), Some(&id)),
            "🟢 synced"
        );
        assert_eq!(
            sync_label(Some(SyncState::Indeterminate), None),
            "⚪ unsynced"
        );
        assert_eq!(sync_label(None, Some(&id)), "🟢 synced");
        assert_eq!(sync_label(None, None), "⚪ unsynced");
    }

    #[test]
    fn the_table_renders_all_five_status_labels_in_the_sync_column() {
        let items = vec![
            with_external(item("0001"), "ENG-1"),
            with_external(item("0002"), "ENG-2"),
            with_external(item("0003"), "ENG-3"),
            with_external(item("0004"), "ENG-4"),
            item("0005"),
        ];
        let mut states = BTreeMap::new();
        states.insert("0001".to_owned(), SyncState::Synced);
        states.insert("0002".to_owned(), SyncState::LocallyModified);
        states.insert("0003".to_owned(), SyncState::RemotelyModified);
        states.insert("0004".to_owned(), SyncState::Conflict);
        let labels = sync_column(&items, &states);
        let selected: Vec<&ScannedItem> = items.iter().collect();

        let table = render_table(&selected, Some(&labels));

        assert!(table.contains("| Sync |"), "{table}");
        for expected in [
            "🟢 synced",
            "🔵 locally modified",
            "🟣 remotely modified",
            "🔴 conflict",
            "⚪ unsynced",
        ] {
            assert!(table.contains(expected), "missing {expected} in\n{table}");
        }
    }

    #[test]
    fn the_sync_column_is_absent_without_a_label_map() {
        let items = vec![item("0001")];
        let selected: Vec<&ScannedItem> = items.iter().collect();
        let table = render_table(&selected, None);
        assert!(!table.contains("Sync"), "{table}");
    }

    #[test]
    fn a_column_that_is_absent_for_every_row_is_suppressed() {
        let mut only = item("0001");
        only.priority = None;
        let items = vec![only];
        let selected: Vec<&ScannedItem> = items.iter().collect();
        let table = render_table(&selected, None);
        assert!(!table.contains("Priority"), "{table}");
        assert!(table.contains("Kind"), "{table}");
    }

    #[test]
    fn each_scalar_filter_selects_its_subset() {
        let mut bug = item("0001");
        bug.kind = Some("bug".to_owned());
        bug.status = Some("done".to_owned());
        let mut story = item("0002");
        story.kind = Some("story".to_owned());
        let items = vec![bug, story];

        let filter = Filter {
            kind: Some("bug".to_owned()),
            ..Filter::default()
        };
        let selected = apply_filter(&items, &filter, PATTERN, "");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "0001");
    }

    #[test]
    fn a_multi_flag_filter_is_conjunctive() {
        let mut done_bug = item("0001");
        done_bug.kind = Some("bug".to_owned());
        done_bug.status = Some("done".to_owned());
        let mut draft_bug = item("0002");
        draft_bug.kind = Some("bug".to_owned());
        draft_bug.status = Some("draft".to_owned());
        let items = vec![done_bug, draft_bug];

        let filter = Filter {
            kind: Some("bug".to_owned()),
            status: Some("done".to_owned()),
            ..Filter::default()
        };
        let selected = apply_filter(&items, &filter, PATTERN, "");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "0001");
    }

    #[test]
    fn a_repeatable_tag_filter_requires_every_tag() {
        let mut both = item("0001");
        both.tags = vec!["backend".to_owned(), "api".to_owned()];
        let mut one = item("0002");
        one.tags = vec!["backend".to_owned()];
        let items = vec![both, one];

        let filter = Filter {
            tags: vec!["backend".to_owned(), "api".to_owned()],
            ..Filter::default()
        };
        let selected = apply_filter(&items, &filter, PATTERN, "");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "0001");
    }

    #[test]
    fn the_parent_filter_canonicalises_both_sides() {
        let mut child = item("0043");
        child.parent = Some("work-item:0042".to_owned());
        let orphan = item("0044");
        let items = vec![child, orphan];

        let filter = Filter {
            parent: Some("42".to_owned()),
            ..Filter::default()
        };
        let selected = apply_filter(&items, &filter, PATTERN, "");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "0043");
    }

    #[test]
    fn the_title_term_is_a_case_insensitive_substring() {
        let mut login = item("0001");
        login.title = Some("Login Form Rework".to_owned());
        let mut other = item("0002");
        other.title = Some("Sync engine".to_owned());
        let items = vec![login, other];

        let filter = Filter {
            term: Some("login".to_owned()),
            ..Filter::default()
        };
        let selected = apply_filter(&items, &filter, PATTERN, "");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "0001");
    }

    #[test]
    fn a_filter_matching_nothing_selects_the_empty_set() {
        let items = vec![item("0001")];
        let filter = Filter {
            kind: Some("epic".to_owned()),
            ..Filter::default()
        };
        assert!(apply_filter(&items, &filter, PATTERN, "").is_empty());
    }

    #[test]
    fn scan_warns_on_malformed_files_and_excludes_non_items() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("0001-good.md"),
            "---\nid: \"0001\"\ntitle: Good\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("0002-nofm.md"), "no fence here\n")
            .unwrap();
        std::fs::write(
            dir.path().join("0003-unclosed.md"),
            "---\nid: \"0003\"\nnever closed\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("readme.md"),
            "---\ntitle: not a work item\n---\nbody\n",
        )
        .unwrap();

        let scan = scan(dir.path());

        assert_eq!(scan.items.len(), 1);
        assert_eq!(scan.items[0].id, "0001");
        assert!(scan
            .warnings
            .iter()
            .any(|w| w == "0002-nofm.md: skipped — no frontmatter"));
        assert!(scan
            .warnings
            .iter()
            .any(|w| w == "0003-unclosed.md: skipped — unclosed frontmatter"));
    }

    #[test]
    fn scan_parses_tags_and_external_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("0001.md"),
            "---\nid: \"0001\"\ntags: [backend, api]\nexternal_id: \"ENG-9\"\n\
             ---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("0002.md"),
            "---\nid: \"0002\"\nexternal_id: \"\"\n---\nbody\n",
        )
        .unwrap();

        let scan = scan(dir.path());

        let first = &scan.items[0];
        assert_eq!(first.tags, vec!["backend", "api"]);
        assert_eq!(
            first.external_id.as_ref().map(ExternalId::as_str),
            Some("ENG-9")
        );
        assert!(scan.items[1].external_id.is_none());
    }

    fn write_item(dir: &Path, id: &str, body: &str) -> PathBuf {
        let path = dir.join(format!("{id}.md"));
        std::fs::write(&path, format!("---\nid: \"{id}\"\n---\n{body}\n"))
            .unwrap();
        path
    }

    #[test]
    fn classify_reports_synced_unsynced_and_locally_modified() {
        let dir = tempfile::tempdir().expect("tempdir");
        let synced_path = write_item(dir.path(), "0001", "unchanged body");
        let modified_path = write_item(dir.path(), "0002", "changed body");
        let unsynced_path = write_item(dir.path(), "0003", "no remote");

        let synced_content = std::fs::read_to_string(&synced_path).unwrap();
        let synced_hash =
            work_adapters::sync::digest::local(&synced_content).unwrap();

        let stamp = RemoteTimestamp::Reported("2026-06-01".to_owned());
        let tracker = RecordingTracker::holding(vec![
            (
                ExternalId::new("ENG-1".to_owned()),
                RemoteIssue {
                    updated: stamp.clone(),
                    body: "Item\nunchanged body\n".to_owned(),
                },
            ),
            (
                ExternalId::new("ENG-2".to_owned()),
                RemoteIssue {
                    updated: stamp.clone(),
                    body: "Item\nremote body\n".to_owned(),
                },
            ),
        ]);

        let mut baseline = Baseline::read(None).0;
        baseline.set(
            "0001",
            Entry {
                remote_updated_at: stamp.clone(),
                remote_hash: "unused".to_owned(),
                local_hash: synced_hash,
            },
        );
        baseline.set(
            "0002",
            Entry {
                remote_updated_at: stamp,
                remote_hash: "unused".to_owned(),
                local_hash: "a-different-hash".to_owned(),
            },
        );

        let items = vec![
            LocalItem {
                id: "0001".to_owned(),
                path: synced_path,
                external_id: Some(ExternalId::new("ENG-1".to_owned())),
            },
            LocalItem {
                id: "0002".to_owned(),
                path: modified_path,
                external_id: Some(ExternalId::new("ENG-2".to_owned())),
            },
            LocalItem {
                id: "0003".to_owned(),
                path: unsynced_path,
                external_id: None,
            },
        ];

        let states = classify(&items, &tracker, &baseline, &AlwaysClean);

        assert_eq!(states.get("0001"), Some(&SyncState::Synced));
        assert_eq!(states.get("0002"), Some(&SyncState::LocallyModified));
        assert_eq!(states.get("0003"), Some(&SyncState::Unsynced));
    }

    #[test]
    fn an_all_indeterminate_read_degrades_to_presence_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_item(dir.path(), "0001", "body");
        let tracker = RecordingTracker::truncating(
            Vec::new(),
            vec![ExternalId::new("ENG-1".to_owned())],
        );
        let baseline = Baseline::read(None).0;
        let scanned = vec![with_external(
            ScannedItem {
                path,
                ..item("0001")
            },
            "ENG-1",
        )];
        let items = local_items(&scanned);

        let states = classify(&items, &tracker, &baseline, &AlwaysClean);

        assert_eq!(
            sync_label(
                states.get("0001").copied(),
                scanned[0].external_id.as_ref()
            ),
            "🟢 synced"
        );
    }

    #[test]
    fn filter_echo_wording_matches_the_skill() {
        assert_eq!(
            filter_echo(&Filter::default(), 29),
            "All work items (29 total)"
        );
        assert_eq!(
            filter_echo(
                &Filter {
                    status: Some("draft".to_owned()),
                    ..Filter::default()
                },
                3
            ),
            "Filter: status=draft (3 matches)"
        );
        assert_eq!(
            filter_echo(
                &Filter {
                    parent: Some("0042".to_owned()),
                    ..Filter::default()
                },
                2
            ),
            "Children of 0042 (2 matches)"
        );
    }

    #[test]
    fn empty_and_directory_messages_match_the_skill() {
        assert_eq!(
            empty_message(&Filter {
                kind: Some("epic".to_owned()),
                ..Filter::default()
            }),
            "No work items matched: kind=epic."
        );
        let title_only = empty_message(&Filter {
            term: Some("nope".to_owned()),
            ..Filter::default()
        });
        assert!(title_only.starts_with("No work items matched: title~nope."));
        assert!(title_only.contains("Tip: to filter by field value"));
        assert_eq!(
            missing_directory_message(Path::new("meta/work")),
            "Work items directory `meta/work` not found. Check the \
             `paths.work` configuration or run `/create-work-item` to create \
             the first work item."
        );
        assert_eq!(
            empty_directory_message(Path::new("meta/work")),
            "No work items found in `meta/work`."
        );
    }

    #[test]
    fn the_hierarchy_nests_children_and_marks_orphans() {
        let mut parent = item("0042");
        parent.title = Some("Epic".to_owned());
        parent.kind = Some("epic".to_owned());
        parent.status = Some("ready".to_owned());
        let mut child = item("0043");
        child.parent = Some("work-item:0042".to_owned());
        let mut orphan = item("0044");
        orphan.parent = Some("work-item:9999".to_owned());
        let items = vec![parent, child, orphan];
        let selected: Vec<&ScannedItem> = items.iter().collect();

        let tree = render_hierarchy(&selected, None, PATTERN, "");

        assert!(
            tree.contains("0042 — Epic (kind: epic, status: ready)"),
            "{tree}"
        );
        assert!(tree.contains("└── 0043 — Item 0043"), "{tree}");
        assert!(
            tree.contains("0044 — Item 0044 (kind: story, status: draft) (parent 9999 not found)"),
            "{tree}"
        );
    }

    #[test]
    fn a_parent_cycle_renders_flat_with_a_marker() {
        let mut a = item("0001");
        a.parent = Some("work-item:0002".to_owned());
        let mut b = item("0002");
        b.parent = Some("work-item:0001".to_owned());
        let items = vec![a, b];
        let selected: Vec<&ScannedItem> = items.iter().collect();

        let tree = render_hierarchy(&selected, None, PATTERN, "");

        assert!(
            tree.contains(
                "0001 — Item 0001 (kind: story, status: draft) (cycle)"
            ),
            "{tree}"
        );
        assert!(
            tree.contains(
                "0002 — Item 0002 (kind: story, status: draft) (cycle)"
            ),
            "{tree}"
        );
    }

    #[test]
    fn the_hierarchy_appends_sync_labels() {
        let mut parent = item("0042");
        parent.external_id = Some(ExternalId::new("ENG-1".to_owned()));
        let items = vec![parent];
        let selected: Vec<&ScannedItem> = items.iter().collect();
        let mut labels = BTreeMap::new();
        labels.insert("0042".to_owned(), "🟢 synced");

        let tree = render_hierarchy(&selected, Some(&labels), PATTERN, "");

        assert!(tree.ends_with("🟢 synced"), "{tree}");
    }
}

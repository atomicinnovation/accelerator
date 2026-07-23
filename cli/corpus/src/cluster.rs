//! Lifecycle clustering over a corpus.
//!
//! Groups artifacts into work-item clusters by walking ADR-0034 typed-linkage
//! frontmatter back to a canonical work-item id, with a slug-equivalence bridge
//! for artifacts that carry no typed linkage. The algorithm is pure and
//! infra-free: it reads each artifact through the [`ClusterEntry`] view, takes
//! the id convention as an injected [`WorkItemIdScheme`] + [`IdScanner`], and
//! returns records keyed by path so the caller can back-fill its own store.

// The convention functions thread the caller's concrete `HashMap`s (default
// hasher) verbatim; generalising every signature over the hasher would bloat
// the recursive walk for no caller benefit.
#![allow(clippy::implicit_hasher)]

use std::collections::HashMap;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use crate::doc_type::DocTypeKey;
use crate::typed_ref::parse_typed_ref;
use crate::typed_ref::TypedRef;
use crate::work_item_id::IdScanner;
use crate::work_item_id::WorkItemIdScheme;

/// A read-only view over a corpus artifact, exposing exactly the fields the
/// clustering algorithm reads. Implemented by the caller over its own index
/// entry type.
pub trait ClusterEntry {
    fn path(&self) -> &Path;
    fn doc_type(&self) -> DocTypeKey;
    fn slug(&self) -> Option<&str>;
    fn title(&self) -> &str;
    fn mtime_ms(&self) -> i64;
    /// True when the frontmatter parsed cleanly (drives `title` preference).
    fn frontmatter_parsed(&self) -> bool;
    /// The filename-derived work-item id, for work-item artifacts.
    fn work_item_id(&self) -> Option<&str>;
    /// The `parent:` frontmatter scalar, if present.
    fn parent(&self) -> Option<&str>;
    /// The `target:` frontmatter scalar, if present.
    fn target(&self) -> Option<&str>;
}

/// Per-cluster stage-presence flags plus the ordered list of present stage
/// keys. Mirrors the visualiser's lifecycle pipeline model.
// One flag per lifecycle stage — the wire contract, not an unmodelled state.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completeness {
    pub has_work_item: bool,
    pub has_research: bool,
    pub has_plan: bool,
    pub has_plan_review: bool,
    pub has_validation: bool,
    pub has_pr_description: bool,
    pub has_pr_review: bool,
    pub has_decision: bool,
    pub has_notes: bool,
    pub has_design_inventory: bool,
    pub has_design_gap: bool,
    pub present: Vec<String>,
}

/// One lifecycle cluster.
///
/// Carries its representative slug/title, member indices (into the input slice)
/// in canonical order, aggregate completeness, most-recent mtime, and canonical
/// cluster key (the resolved work-item id, or `None` for slug-fallback
/// clusters). Membership is by index rather than path so callers can re-project
/// distinct entries even when two share a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    pub slug: String,
    pub title: String,
    pub members: Vec<usize>,
    pub completeness: Completeness,
    pub last_changed_ms: i64,
    pub cluster_key: Option<String>,
}

/// The full clustering result: the clusters themselves plus the per-path
/// back-fill maps the caller applies to its canonical entry store.
pub struct Clustering {
    pub clusters: Vec<Cluster>,
    pub completeness_by_path: HashMap<PathBuf, Completeness>,
    pub cluster_key_by_path: HashMap<PathBuf, Option<String>>,
}

/// Injected snapshots and conventions the resolver reads. Built once per
/// clustering pass from a coherent view of the entries.
pub struct ClusterContext<'a, E: ClusterEntry> {
    pub entries_by_path: HashMap<PathBuf, &'a E>,
    pub work_item_by_id: &'a HashMap<String, PathBuf>,
    pub plans_by_id: &'a HashMap<String, PathBuf>,
    pub project_root: &'a Path,
    pub scheme: &'a WorkItemIdScheme,
    pub scanner: &'a dyn IdScanner,
}

impl<'a, E: ClusterEntry> ClusterContext<'a, E> {
    /// Build `entries_by_path` from the borrowed entries slice so the snapshot
    /// is trivially coherent with the slice being clustered.
    #[must_use]
    pub fn from_entries(
        entries: &'a [E],
        work_item_by_id: &'a HashMap<String, PathBuf>,
        plans_by_id: &'a HashMap<String, PathBuf>,
        project_root: &'a Path,
        scheme: &'a WorkItemIdScheme,
        scanner: &'a dyn IdScanner,
    ) -> Self {
        let entries_by_path = entries
            .iter()
            .map(|e| (e.path().to_path_buf(), e))
            .collect();
        Self {
            entries_by_path,
            work_item_by_id,
            plans_by_id,
            project_root,
            scheme,
            scanner,
        }
    }
}

/// The longest legitimate chain in today's vocabulary is
/// work-item-review → plan → parent → work-item (3 hops); 8 gives headroom for
/// transitional path-target shapes without measurable cost. When hit, the walk
/// returns `None` (the entry falls back to a slug bucket).
const MAX_DEPTH: u8 = 8;

#[allow(clippy::type_complexity)]
const STAGE_PUSH_ORDER: &[(fn(&Completeness) -> bool, &str)] = &[
    (|c| c.has_work_item, "work-items"),
    (|c| c.has_research, "research"),
    (|c| c.has_plan, "plans"),
    (|c| c.has_plan_review, "plan-reviews"),
    (|c| c.has_validation, "validations"),
    (|c| c.has_pr_description, "pr-descriptions"),
    (|c| c.has_pr_review, "pr-reviews"),
    (|c| c.has_decision, "decisions"),
    (|c| c.has_notes, "notes"),
    (|c| c.has_design_inventory, "design-inventories"),
    (|c| c.has_design_gap, "design-gaps"),
];

/// Resolves an entry's canonical cluster key by walking typed-linkage
/// frontmatter back to a work-item id. `None` when no chain reaches a work
/// item; callers fall back to the entry's slug.
#[must_use]
pub fn resolve_cluster_key<E: ClusterEntry>(
    entry: &E,
    entries_by_path: &HashMap<PathBuf, &E>,
    work_item_by_id: &HashMap<String, PathBuf>,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
    scheme: &WorkItemIdScheme,
    scanner: &dyn IdScanner,
) -> Option<String> {
    walk(
        entry,
        entries_by_path,
        work_item_by_id,
        plans_by_id,
        project_root,
        scheme,
        scanner,
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn walk<E: ClusterEntry>(
    entry: &E,
    entries_by_path: &HashMap<PathBuf, &E>,
    work_item_by_id: &HashMap<String, PathBuf>,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
    scheme: &WorkItemIdScheme,
    scanner: &dyn IdScanner,
    depth: u8,
) -> Option<String> {
    if depth >= MAX_DEPTH {
        return None;
    }
    match entry.doc_type() {
        DocTypeKey::WorkItems => entry.work_item_id().map(str::to_owned),
        DocTypeKey::Plans
        | DocTypeKey::Research
        | DocTypeKey::PrDescriptions => {
            cluster_key_from_parent(entry, scheme, scanner)
        }
        DocTypeKey::PlanReviews
        | DocTypeKey::WorkItemReviews
        | DocTypeKey::PrReviews
        | DocTypeKey::Validations => {
            if let Some(s) = entry.target() {
                if let Some(TypedRef::WorkItem(id)) = parse_typed_ref(s) {
                    return scheme.canonicalise_id(&id);
                }
            }
            let target_path = target_path_from_entry(
                entry.doc_type(),
                entry.target(),
                plans_by_id,
                work_item_by_id,
                scheme,
                project_root,
            )?;
            let target_entry: &E = *entries_by_path.get(&target_path)?;
            walk(
                target_entry,
                entries_by_path,
                work_item_by_id,
                plans_by_id,
                project_root,
                scheme,
                scanner,
                depth + 1,
            )
        }
        DocTypeKey::Decisions
        | DocTypeKey::Notes
        | DocTypeKey::DesignGaps
        | DocTypeKey::DesignInventories
        | DocTypeKey::RootCauseAnalyses
        | DocTypeKey::Templates => None,
    }
}

fn cluster_key_from_parent<E: ClusterEntry>(
    entry: &E,
    scheme: &WorkItemIdScheme,
    scanner: &dyn IdScanner,
) -> Option<String> {
    entry
        .parent()
        .and_then(|raw| id_from_value(raw, scheme, scanner))
}

fn id_from_value(
    raw: &str,
    scheme: &WorkItemIdScheme,
    scanner: &dyn IdScanner,
) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    match parse_typed_ref(s) {
        Some(TypedRef::WorkItem(id)) => scheme.canonicalise_id(&id),
        Some(TypedRef::Path(p)) => {
            let stem = p.file_stem()?.to_str()?;
            scheme.extract_id(&format!("{stem}.md"), scanner)
        }
        Some(_) => None,
        None => scheme.canonicalise_id(s),
    }
}

/// Resolves a review/validation `target:` value to the target artifact's path.
///
/// Handles the ADR-0034 forms (path, `plan:<id>`, `work-item:NNNN`). Returns
/// `None` for entries that carry no `target:`, or for `adr:`/`pr:` targets.
#[must_use]
pub fn target_path_from_entry(
    doc_type: DocTypeKey,
    target: Option<&str>,
    plans_by_id: &HashMap<String, PathBuf>,
    work_item_by_id: &HashMap<String, PathBuf>,
    scheme: &WorkItemIdScheme,
    project_root: &Path,
) -> Option<PathBuf> {
    if !doc_type.carries_target_frontmatter() {
        return None;
    }
    let raw = target?;
    match parse_typed_ref(raw)? {
        TypedRef::Plan(id) => plans_by_id.get(&id).cloned(),
        TypedRef::WorkItem(id) => {
            let canon = scheme.canonicalise_id(&id)?;
            work_item_by_id.get(&canon).cloned()
        }
        TypedRef::Path(p) => {
            let raw_str = p.to_str()?;
            normalize_target_key(raw_str, project_root)
        }
        _ => None,
    }
}

/// Validates and normalises a `target:` path value against `project_root`.
///
/// Returns `None` for an empty value, an absolute path, any `.`/`..`/NUL/
/// backslash segment, or a path that escapes `project_root` after joining.
#[must_use]
pub fn normalize_target_key(raw: &str, project_root: &Path) -> Option<PathBuf> {
    if raw.is_empty() || raw.starts_with('/') {
        return None;
    }
    for segment in raw.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.contains('\0')
        {
            return None;
        }
    }
    let joined = project_root.join(raw);
    let normalized = normalize_absolute(&joined);
    if !normalized.starts_with(project_root) {
        return None;
    }
    Some(normalized)
}

/// Lexically normalises a path: drops `.`, resolves `..` against `Normal`
/// components only (never past a root/prefix). No filesystem access, no
/// Unicode/case folding, no symlink resolution.
fn normalize_absolute(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => {
                out.push(c.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                let popped = out
                    .components()
                    .next_back()
                    .is_some_and(|c| matches!(c, Component::Normal(_)));
                if popped {
                    out.pop();
                }
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

/// Groups `entries` into lifecycle clusters and returns the clusters plus the
/// per-path completeness and cluster-key back-fill maps.
#[allow(clippy::too_many_lines, clippy::option_if_let_else)]
#[must_use]
pub fn compute<E: ClusterEntry>(
    entries: &[E],
    ctx: &ClusterContext<'_, E>,
) -> Clustering {
    let mut cluster_key_by_path: HashMap<PathBuf, Option<String>> =
        HashMap::with_capacity(entries.len());
    for e in entries {
        if matches!(e.doc_type(), DocTypeKey::Templates) {
            continue;
        }
        let key = resolve_cluster_key(
            e,
            &ctx.entries_by_path,
            ctx.work_item_by_id,
            ctx.plans_by_id,
            ctx.project_root,
            ctx.scheme,
            ctx.scanner,
        );
        cluster_key_by_path.insert(e.path().to_path_buf(), key);
    }

    let mut slug_to_cluster_key: HashMap<String, String> = HashMap::new();
    for e in entries {
        if e.doc_type() != DocTypeKey::WorkItems {
            continue;
        }
        if let (Some(slug), Some(ck)) = (
            e.slug(),
            cluster_key_by_path.get(e.path()).and_then(|o| o.as_deref()),
        ) {
            slug_to_cluster_key.insert(slug.to_owned(), ck.to_owned());
        }
    }
    for e in entries {
        if e.doc_type() == DocTypeKey::WorkItems {
            continue;
        }
        if let (Some(slug), Some(ck)) = (
            e.slug(),
            cluster_key_by_path.get(e.path()).and_then(|o| o.as_deref()),
        ) {
            slug_to_cluster_key
                .entry(slug.to_owned())
                .or_insert_with(|| ck.to_owned());
        }
    }

    let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        if matches!(e.doc_type(), DocTypeKey::Templates) {
            continue;
        }
        let key = cluster_key_by_path.get(e.path()).and_then(|o| o.as_deref());
        let bucket_key = match key {
            Some(k) => Some(k.to_owned()),
            None if e.doc_type().participates_in_lifecycle() => {
                match e.slug().and_then(|s| slug_to_cluster_key.get(s)) {
                    Some(ck) => {
                        cluster_key_by_path
                            .insert(e.path().to_path_buf(), Some(ck.clone()));
                        Some(ck.clone())
                    }
                    None => e.slug().map(str::to_owned),
                }
            }
            None => match e.slug().and_then(|s| slug_to_cluster_key.get(s)) {
                Some(ck) => {
                    cluster_key_by_path
                        .insert(e.path().to_path_buf(), Some(ck.clone()));
                    Some(ck.clone())
                }
                None => Some(format!("__orphan__::{}", e.path().display())),
            },
        };
        let Some(k) = bucket_key else { continue };
        buckets.entry(k).or_default().push(i);
    }

    let mut completeness_by_path: HashMap<PathBuf, Completeness> =
        HashMap::new();
    let mut clusters: Vec<Cluster> = buckets
        .into_values()
        .map(|mut members| {
            members.sort_by(|&a, &b| {
                canonical_rank(entries[a].doc_type())
                    .cmp(&canonical_rank(entries[b].doc_type()))
                    .then(entries[a].mtime_ms().cmp(&entries[b].mtime_ms()))
            });
            let bucket: Vec<&E> =
                members.iter().map(|&i| &entries[i]).collect();
            let last_changed_ms =
                bucket.iter().map(|e| e.mtime_ms()).max().unwrap_or(0);
            let cluster_key =
                pick_representative_cluster_key(&bucket, &cluster_key_by_path);
            let representative_slug =
                pick_representative_slug(&bucket, cluster_key.as_deref());
            let title = derive_title(&representative_slug, &bucket);
            let completeness = derive_completeness(&bucket);
            for e in &bucket {
                completeness_by_path
                    .insert(e.path().to_path_buf(), completeness.clone());
            }
            Cluster {
                slug: representative_slug,
                title,
                members,
                completeness,
                last_changed_ms,
                cluster_key,
            }
        })
        .collect();

    clusters.sort_by(|a, b| a.slug.cmp(&b.slug));
    Clustering {
        clusters,
        completeness_by_path,
        cluster_key_by_path,
    }
}

fn pick_representative_cluster_key<E: ClusterEntry>(
    bucket: &[&E],
    cluster_key_by_path: &HashMap<PathBuf, Option<String>>,
) -> Option<String> {
    if let Some(wi) = bucket
        .iter()
        .find(|e| e.doc_type() == DocTypeKey::WorkItems)
    {
        if let Some(key) = cluster_key_by_path.get(wi.path()).cloned().flatten()
        {
            return Some(key);
        }
    }
    let mut sorted: Vec<&&E> = bucket.iter().collect();
    sorted.sort_by(|a, b| a.path().cmp(b.path()));
    for e in &sorted {
        if let Some(key) = cluster_key_by_path.get(e.path()).cloned().flatten()
        {
            return Some(key);
        }
    }
    None
}

fn pick_representative_slug<E: ClusterEntry>(
    bucket: &[&E],
    cluster_key: Option<&str>,
) -> String {
    if let Some(wi_slug) = bucket
        .iter()
        .find(|e| e.doc_type() == DocTypeKey::WorkItems)
        .and_then(|e| e.slug())
    {
        return wi_slug.to_owned();
    }
    let mut sorted: Vec<&&E> = bucket.iter().collect();
    sorted.sort_by(|a, b| a.path().cmp(b.path()));
    if let Some(s) = sorted.iter().find_map(|e| e.slug()) {
        return s.to_owned();
    }
    cluster_key.unwrap_or("").to_owned()
}

const fn canonical_rank(kind: DocTypeKey) -> u8 {
    match kind {
        DocTypeKey::WorkItems => 0,
        DocTypeKey::Research => 1,
        DocTypeKey::Plans => 2,
        DocTypeKey::PlanReviews => 3,
        DocTypeKey::Validations => 4,
        DocTypeKey::PrDescriptions => 5,
        DocTypeKey::PrReviews | DocTypeKey::WorkItemReviews => 6,
        DocTypeKey::Decisions => 7,
        DocTypeKey::Notes => 8,
        DocTypeKey::DesignInventories => 9,
        DocTypeKey::DesignGaps => 10,
        DocTypeKey::RootCauseAnalyses | DocTypeKey::Templates => u8::MAX,
    }
}

fn derive_title<E: ClusterEntry>(slug: &str, entries: &[&E]) -> String {
    for e in entries {
        if e.frontmatter_parsed() && !e.title().is_empty() {
            return e.title().to_owned();
        }
    }
    if let Some(e) = entries.first() {
        if !e.title().is_empty() {
            return e.title().to_owned();
        }
    }
    slug.to_owned()
}

fn derive_completeness<E: ClusterEntry>(entries: &[&E]) -> Completeness {
    let mut c = Completeness {
        has_work_item: false,
        has_research: false,
        has_plan: false,
        has_plan_review: false,
        has_validation: false,
        has_pr_description: false,
        has_pr_review: false,
        has_decision: false,
        has_notes: false,
        has_design_inventory: false,
        has_design_gap: false,
        present: Vec::new(),
    };
    for e in entries {
        match e.doc_type() {
            DocTypeKey::WorkItems => c.has_work_item = true,
            DocTypeKey::Research => c.has_research = true,
            DocTypeKey::Plans => c.has_plan = true,
            DocTypeKey::PlanReviews => c.has_plan_review = true,
            DocTypeKey::WorkItemReviews
            | DocTypeKey::RootCauseAnalyses
            | DocTypeKey::Templates => {}
            DocTypeKey::Validations => c.has_validation = true,
            DocTypeKey::PrDescriptions => c.has_pr_description = true,
            DocTypeKey::PrReviews => c.has_pr_review = true,
            DocTypeKey::Decisions => c.has_decision = true,
            DocTypeKey::Notes => c.has_notes = true,
            DocTypeKey::DesignGaps => c.has_design_gap = true,
            DocTypeKey::DesignInventories => c.has_design_inventory = true,
        }
    }
    for (test, key) in STAGE_PUSH_ORDER {
        if test(&c) {
            c.present.push((*key).into());
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_item_id::IdScan;

    struct DigitRunScanner;

    impl IdScanner for DigitRunScanner {
        fn scan(&self, text: &str) -> Option<IdScan> {
            let digits: String =
                text.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() || !text[digits.len()..].starts_with('-') {
                return None;
            }
            let match_end = digits.len() + 1;
            Some(IdScan { digits, match_end })
        }
    }

    #[derive(Clone)]
    struct TestEntry {
        path: PathBuf,
        doc_type: DocTypeKey,
        slug: Option<String>,
        title: String,
        mtime_ms: i64,
        work_item_id: Option<String>,
        parent: Option<String>,
        target: Option<String>,
    }

    impl TestEntry {
        fn new(doc_type: DocTypeKey, slug: &str) -> Self {
            Self {
                path: PathBuf::from(format!("/repo/{slug}.md")),
                doc_type,
                slug: Some(slug.to_owned()),
                title: slug.to_owned(),
                mtime_ms: 1,
                work_item_id: None,
                parent: None,
                target: None,
            }
        }
    }

    impl ClusterEntry for TestEntry {
        fn path(&self) -> &Path {
            &self.path
        }
        fn doc_type(&self) -> DocTypeKey {
            self.doc_type
        }
        fn slug(&self) -> Option<&str> {
            self.slug.as_deref()
        }
        fn title(&self) -> &str {
            &self.title
        }
        fn mtime_ms(&self) -> i64 {
            self.mtime_ms
        }
        fn frontmatter_parsed(&self) -> bool {
            true
        }
        fn work_item_id(&self) -> Option<&str> {
            self.work_item_id.as_deref()
        }
        fn parent(&self) -> Option<&str> {
            self.parent.as_deref()
        }
        fn target(&self) -> Option<&str> {
            self.target.as_deref()
        }
    }

    fn resolve(
        entry: &TestEntry,
        entries: &HashMap<PathBuf, &TestEntry>,
        work_item_by_id: &HashMap<String, PathBuf>,
        plans_by_id: &HashMap<String, PathBuf>,
        scheme: &WorkItemIdScheme,
    ) -> Option<String> {
        resolve_cluster_key(
            entry,
            entries,
            work_item_by_id,
            plans_by_id,
            Path::new("/repo"),
            scheme,
            &DigitRunScanner,
        )
    }

    fn empty() -> HashMap<PathBuf, &'static TestEntry> {
        HashMap::new()
    }

    #[test]
    fn work_item_returns_its_own_work_item_id() {
        let mut wi = TestEntry::new(DocTypeKey::WorkItems, "pipeline");
        wi.work_item_id = Some("0040".into());
        let scheme = WorkItemIdScheme::numeric();
        assert_eq!(
            resolve(&wi, &empty(), &HashMap::new(), &HashMap::new(), &scheme)
                .as_deref(),
            Some("0040")
        );
    }

    #[test]
    fn typed_and_bare_parent_resolve_equally() {
        let scheme = WorkItemIdScheme::numeric();
        let mut typed = TestEntry::new(DocTypeKey::Plans, "pipeline");
        typed.parent = Some("work-item:0042".into());
        let mut bare = TestEntry::new(DocTypeKey::Plans, "pipeline");
        bare.parent = Some("0042".into());
        let t = resolve(
            &typed,
            &empty(),
            &HashMap::new(),
            &HashMap::new(),
            &scheme,
        );
        let b =
            resolve(&bare, &empty(), &HashMap::new(), &HashMap::new(), &scheme);
        assert_eq!(t, b);
        assert_eq!(t.as_deref(), Some("0042"));
    }

    #[test]
    fn path_shape_parent_resolves_via_extract_id() {
        let scheme = WorkItemIdScheme::numeric();
        let mut plan = TestEntry::new(DocTypeKey::Plans, "pipeline");
        plan.parent = Some("meta/work/0033-foo.md".into());
        assert_eq!(
            resolve(&plan, &empty(), &HashMap::new(), &HashMap::new(), &scheme)
                .as_deref(),
            Some("0033")
        );
    }

    #[test]
    fn review_target_path_resolves_transitively_via_plan() {
        let scheme = WorkItemIdScheme::numeric();
        let plan_path =
            PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
        let mut plan = TestEntry::new(DocTypeKey::Plans, "pipeline");
        plan.path = plan_path.clone();
        plan.parent = Some("work-item:0040".into());
        let mut entries: HashMap<PathBuf, &TestEntry> = HashMap::new();
        entries.insert(plan_path, &plan);
        let mut review = TestEntry::new(DocTypeKey::PlanReviews, "rev");
        review.target = Some("meta/plans/2026-05-31-0040-pipeline.md".into());
        assert_eq!(
            resolve(
                &review,
                &entries,
                &HashMap::new(),
                &HashMap::new(),
                &scheme
            )
            .as_deref(),
            Some("0040")
        );
    }

    #[test]
    fn work_item_review_typed_target_short_circuits() {
        let scheme = WorkItemIdScheme::numeric();
        let mut review = TestEntry::new(DocTypeKey::WorkItemReviews, "rev");
        review.target = Some("work-item:0033".into());
        assert_eq!(
            resolve(
                &review,
                &empty(),
                &HashMap::new(),
                &HashMap::new(),
                &scheme
            )
            .as_deref(),
            Some("0033")
        );
    }

    #[test]
    fn cycle_between_two_reviews_terminates() {
        let scheme = WorkItemIdScheme::numeric();
        let path_a = PathBuf::from("/repo/meta/reviews/plans/a.md");
        let path_b = PathBuf::from("/repo/meta/reviews/plans/b.md");
        let mut a = TestEntry::new(DocTypeKey::PlanReviews, "a");
        a.path = path_a.clone();
        a.target = Some("meta/reviews/plans/b.md".into());
        let mut b = TestEntry::new(DocTypeKey::PlanReviews, "b");
        b.path = path_b.clone();
        b.target = Some("meta/reviews/plans/a.md".into());
        let mut entries: HashMap<PathBuf, &TestEntry> = HashMap::new();
        entries.insert(path_a, &a);
        entries.insert(path_b, &b);
        assert_eq!(
            resolve(&a, &entries, &HashMap::new(), &HashMap::new(), &scheme),
            None
        );
    }

    #[test]
    fn orphan_by_design_types_resolve_none() {
        let scheme = WorkItemIdScheme::numeric();
        for kind in [
            DocTypeKey::Notes,
            DocTypeKey::DesignGaps,
            DocTypeKey::DesignInventories,
            DocTypeKey::Decisions,
            DocTypeKey::Templates,
        ] {
            let e = TestEntry::new(kind, "x");
            assert_eq!(
                resolve(
                    &e,
                    &empty(),
                    &HashMap::new(),
                    &HashMap::new(),
                    &scheme
                ),
                None,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn project_pattern_bare_id_gains_prefix() {
        #[allow(clippy::literal_string_with_formatting_args)]
        let scheme = WorkItemIdScheme {
            id_pattern: "{project}-{number:04d}".into(),
            default_project_code: Some("PROJ".into()),
        };
        let mut plan = TestEntry::new(DocTypeKey::Plans, "p");
        plan.parent = Some("42".into());
        assert_eq!(
            resolve(&plan, &empty(), &HashMap::new(), &HashMap::new(), &scheme)
                .as_deref(),
            Some("PROJ-0042")
        );
    }

    #[test]
    fn malformed_empty_typed_parent_resolves_none() {
        let scheme = WorkItemIdScheme::numeric();
        let mut plan = TestEntry::new(DocTypeKey::Plans, "p");
        plan.parent = Some("work-item:".into());
        assert_eq!(
            resolve(&plan, &empty(), &HashMap::new(), &HashMap::new(), &scheme),
            None
        );
    }

    fn ctx_for<'a>(
        entries: &'a [TestEntry],
        work_item_by_id: &'a HashMap<String, PathBuf>,
        plans_by_id: &'a HashMap<String, PathBuf>,
        scheme: &'a WorkItemIdScheme,
    ) -> ClusterContext<'a, TestEntry> {
        ClusterContext::from_entries(
            entries,
            work_item_by_id,
            plans_by_id,
            Path::new("/repo"),
            scheme,
            &DigitRunScanner,
        )
    }

    #[test]
    fn same_slug_clusters_into_one_ordered_bucket() {
        let scheme = WorkItemIdScheme::numeric();
        let mut plan = TestEntry::new(DocTypeKey::Plans, "foo");
        plan.path = PathBuf::from("/repo/plan.md");
        plan.mtime_ms = 10;
        let mut review = TestEntry::new(DocTypeKey::PlanReviews, "foo");
        review.path = PathBuf::from("/repo/review.md");
        review.mtime_ms = 20;
        let mut wi = TestEntry::new(DocTypeKey::WorkItems, "foo");
        wi.path = PathBuf::from("/repo/wi.md");
        wi.mtime_ms = 5;
        let entries = vec![plan, review, wi];
        let wib = HashMap::new();
        let plb = HashMap::new();
        let out = compute(&entries, &ctx_for(&entries, &wib, &plb, &scheme));
        assert_eq!(out.clusters.len(), 1);
        let c = &out.clusters[0];
        assert_eq!(c.slug, "foo");
        assert_eq!(c.members.len(), 3);
        assert_eq!(
            c.completeness.present,
            vec!["work-items", "plans", "plan-reviews"]
        );
        assert!(c.completeness.has_plan_review);
        assert_eq!(c.last_changed_ms, 20);
    }

    #[test]
    fn templates_are_excluded() {
        let scheme = WorkItemIdScheme::numeric();
        let mut plan = TestEntry::new(DocTypeKey::Plans, "shared");
        plan.path = PathBuf::from("/repo/plan.md");
        let mut tmpl = TestEntry::new(DocTypeKey::Templates, "shared");
        tmpl.path = PathBuf::from("/repo/tmpl.md");
        let entries = vec![plan, tmpl];
        let wib = HashMap::new();
        let plb = HashMap::new();
        let out = compute(&entries, &ctx_for(&entries, &wib, &plb, &scheme));
        assert_eq!(out.clusters.len(), 1);
        assert_eq!(out.clusters[0].members.len(), 1);
    }

    #[test]
    fn orphan_types_with_colliding_slugs_do_not_merge() {
        let scheme = WorkItemIdScheme::numeric();
        let mut a = TestEntry::new(DocTypeKey::Notes, "shared");
        a.path = PathBuf::from("/repo/a.md");
        let mut b = TestEntry::new(DocTypeKey::Notes, "shared");
        b.path = PathBuf::from("/repo/b.md");
        let entries = vec![a, b];
        let wib = HashMap::new();
        let plb = HashMap::new();
        let out = compute(&entries, &ctx_for(&entries, &wib, &plb, &scheme));
        assert_eq!(out.clusters.len(), 2);
    }

    #[test]
    fn normalize_target_key_rejects_escapes() {
        let root = Path::new("/repo");
        assert!(normalize_target_key("meta/work/x.md", root).is_some());
        assert!(normalize_target_key("../escape.md", root).is_none());
        assert!(normalize_target_key("/abs.md", root).is_none());
        assert!(normalize_target_key("", root).is_none());
    }
}

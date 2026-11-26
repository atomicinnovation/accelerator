#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use corpus::cluster::{
    compute, resolve_cluster_key, ClusterContext, ClusterEntry,
};
use corpus::linkage::resolve_path_target;
use corpus::{DocTypeCatalogue, DocTypeKey, IdConvention, LinkageIds};

struct FourDigitIds;

fn four_digits(token: &str) -> bool {
    token.len() == 4 && token.chars().all(|c| c.is_ascii_digit())
}

impl IdConvention for FourDigitIds {
    fn slug_start(&self, stem: &str) -> Option<usize> {
        let (head, _) = stem.split_once('-')?;
        four_digits(head).then_some(head.len() + 1)
    }

    fn is_id(&self, token: &str) -> bool {
        four_digits(token)
    }

    fn identity_key(&self, reference: &str) -> Option<String> {
        four_digits(reference).then(|| reference.to_owned())
    }

    fn id_in_filename(&self, filename: &str) -> Option<String> {
        let (head, _) = filename.split_once('-')?;
        four_digits(head).then(|| head.to_owned())
    }
}

impl LinkageIds for FourDigitIds {
    fn id_in_filename(&self, filename: &str) -> Option<String> {
        IdConvention::id_in_filename(self, filename)
    }

    fn bare_ids_in(&self, _line: &str) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Clone)]
struct Entry {
    path: PathBuf,
    doc_type: DocTypeKey,
    slug: Option<String>,
    mtime_ms: i64,
    work_item_id: Option<String>,
    parent: Option<String>,
    target: Option<String>,
}

impl Entry {
    fn new(doc_type: DocTypeKey, name: &str, slug: &str) -> Self {
        Self {
            path: PathBuf::from(format!("/repo/{name}.md")),
            doc_type,
            slug: Some(slug.to_owned()),
            mtime_ms: 1,
            work_item_id: None,
            parent: None,
            target: None,
        }
    }

    fn linked_to_work_item_0042(mut self) -> Self {
        self.work_item_id = Some("0042".into());
        self.parent = Some("work-item:0042".into());
        self.target = Some("work-item:0042".into());
        self
    }
}

impl ClusterEntry for Entry {
    fn path(&self) -> &Path {
        &self.path
    }
    fn doc_type(&self) -> DocTypeKey {
        self.doc_type
    }
    fn slug(&self) -> Option<&str> {
        self.slug.as_deref()
    }
    fn title(&self) -> &'static str {
        ""
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

fn kind(wire_token: &str) -> DocTypeKey {
    DocTypeCatalogue::built_in()
        .from_wire_token(wire_token)
        .expect("a built-in wire token")
}

fn every_built_in_doc_type() -> Vec<DocTypeKey> {
    DocTypeCatalogue::built_in().doc_types().to_vec()
}

fn assert_covers_every_built_in<T>(table: &[(&str, T)]) {
    let covered: Vec<&str> = table.iter().map(|(token, _)| *token).collect();
    let declared: Vec<&str> = every_built_in_doc_type()
        .iter()
        .map(|kind| kind.wire_token())
        .collect();
    assert_eq!(covered, declared);
}

fn cluster(entries: &[Entry]) -> corpus::cluster::Clustering {
    let no_work_items = HashMap::new();
    let no_addresses = HashMap::new();
    let context = ClusterContext::from_entries(
        entries,
        &no_work_items,
        &no_addresses,
        Path::new("/repo"),
        &FourDigitIds,
    );
    compute(entries, &context)
}

#[test]
fn each_built_in_doc_type_derives_its_slug() {
    let table: [(&str, (&str, Option<&str>)); 15] = [
        ("decisions", ("ADR-0007-use-rust.md", Some("use-rust"))),
        ("work-items", ("0042-ship-it.md", Some("ship-it"))),
        ("plans", ("2026-01-02-0042-a-plan.md", Some("a-plan"))),
        (
            "codebase-research",
            ("2026-01-02-findings.md", Some("findings")),
        ),
        (
            "plan-reviews",
            ("2026-01-02-a-plan-review-1.md", Some("a-plan")),
        ),
        ("pr-reviews", ("2026-01-02-pr-7-review-2.md", Some("pr-7"))),
        (
            "work-item-reviews",
            ("0042-ship-it-review-1.md", Some("ship-it")),
        ),
        ("validations", ("2026-01-02-a-plan.md", Some("a-plan"))),
        ("notes", ("2026-01-02-a-note.md", Some("a-note"))),
        ("pr-descriptions", ("2026-01-02-pr-7.md", Some("pr-7"))),
        ("design-gaps", ("2026-01-02-a-gap.md", Some("a-gap"))),
        (
            "design-inventories",
            ("2026-01-02-an-app.md", Some("an-app")),
        ),
        (
            "topic-research",
            ("2026-01-02-mcp.md", Some("2026-01-02-mcp")),
        ),
        (
            "root-cause-analyses",
            ("2026-01-02-outage.md", Some("outage")),
        ),
        ("templates", ("plan.md", None)),
    ];
    assert_covers_every_built_in(&table);
    for (token, (filename, slug)) in table {
        assert_eq!(
            corpus::slug::derive(kind(token), filename, &FourDigitIds)
                .as_deref(),
            slug,
            "{token}"
        );
    }
}

#[test]
fn each_built_in_doc_type_resolves_its_cluster_key() {
    let table: [(&str, Option<&str>); 15] = [
        ("decisions", None),
        ("work-items", Some("0042")),
        ("plans", Some("0042")),
        ("codebase-research", Some("0042")),
        ("plan-reviews", Some("0042")),
        ("pr-reviews", Some("0042")),
        ("work-item-reviews", Some("0042")),
        ("validations", Some("0042")),
        ("notes", None),
        ("pr-descriptions", Some("0042")),
        ("design-gaps", None),
        ("design-inventories", None),
        ("topic-research", None),
        ("root-cause-analyses", None),
        ("templates", None),
    ];
    assert_covers_every_built_in(&table);
    for (token, key) in table {
        let entry =
            Entry::new(kind(token), "doc", "doc").linked_to_work_item_0042();
        assert_eq!(
            resolve_cluster_key(
                &entry,
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                Path::new("/repo"),
                &FourDigitIds,
            )
            .as_deref(),
            key,
            "{token}"
        );
    }
}

#[test]
fn only_work_items_anchor_the_cluster_they_join() {
    let plans = kind("plans");
    for anchor_candidate in every_built_in_doc_type() {
        let entries = [
            Entry::new(anchor_candidate, "z", "z").linked_to_work_item_0042(),
            Entry::new(plans, "a", "a").linked_to_work_item_0042(),
        ];
        let clustering = cluster(&entries);
        let plan_cluster = clustering
            .clusters
            .iter()
            .find(|cluster| cluster.members.contains(&1))
            .expect("the plan is clustered");
        let anchors = plan_cluster.slug == "z";
        assert_eq!(
            anchors,
            anchor_candidate.wire_token() == "work-items",
            "{anchor_candidate:?}"
        );
    }
}

#[test]
fn each_built_in_doc_type_marks_its_lifecycle_stage() {
    let table: [(&str, Option<&str>); 15] = [
        ("decisions", Some("decisions")),
        ("work-items", Some("work-items")),
        ("plans", Some("plans")),
        ("codebase-research", Some("codebase-research")),
        ("plan-reviews", Some("plan-reviews")),
        ("pr-reviews", Some("pr-reviews")),
        ("work-item-reviews", None),
        ("validations", Some("validations")),
        ("notes", Some("notes")),
        ("pr-descriptions", Some("pr-descriptions")),
        ("design-gaps", Some("design-gaps")),
        ("design-inventories", Some("design-inventories")),
        ("topic-research", None),
        ("root-cause-analyses", None),
        ("templates", None),
    ];
    assert_covers_every_built_in(&table);
    for (token, stage) in table {
        let clustering = cluster(&[Entry::new(kind(token), "doc", "doc")]);
        let present: Vec<String> = clustering
            .clusters
            .iter()
            .flat_map(|cluster| cluster.completeness.present.clone())
            .collect();
        let expected: Vec<String> = stage.into_iter().map(Into::into).collect();
        assert_eq!(present, expected, "{token}");
    }
}

#[test]
fn members_of_a_cluster_follow_the_lifecycle_rank_of_their_types() {
    let entries: Vec<Entry> = every_built_in_doc_type()
        .into_iter()
        .enumerate()
        .map(|(position, kind)| {
            let mut entry =
                Entry::new(kind, &format!("doc-{position}"), "shared");
            entry.mtime_ms = 100 - i64::try_from(position).unwrap_or(0);
            if kind.wire_token() == "work-items" {
                entry.work_item_id = Some("0042".into());
            }
            entry
        })
        .collect();
    let clustering = cluster(&entries);
    assert_eq!(clustering.clusters.len(), 1);
    let ranked: Vec<&str> = clustering.clusters[0]
        .members
        .iter()
        .map(|&member| entries[member].doc_type.wire_token())
        .collect();
    assert_eq!(
        ranked,
        [
            "work-items",
            "codebase-research",
            "plans",
            "plan-reviews",
            "validations",
            "pr-descriptions",
            "work-item-reviews",
            "pr-reviews",
            "decisions",
            "notes",
            "design-inventories",
            "design-gaps",
            "root-cause-analyses",
            "topic-research",
        ]
    );
}

type PathAndAddress = (&'static str, Option<(&'static str, &'static str)>);

#[test]
#[allow(clippy::too_many_lines)]
fn each_built_in_doc_type_derives_the_linkage_id_of_a_path() {
    let table: [(&str, PathAndAddress); 15] = [
        (
            "decisions",
            ("decisions/ADR-0007-use-rust.md", Some(("adr", "ADR-0007"))),
        ),
        (
            "work-items",
            ("work/0042-ship-it.md", Some(("work-item", "0042"))),
        ),
        (
            "plans",
            (
                "plans/2026-01-02-a-plan.md",
                Some(("plan", "2026-01-02-a-plan")),
            ),
        ),
        (
            "codebase-research",
            (
                "research/2026-01-02-x.md",
                Some(("codebase-research", "2026-01-02-x")),
            ),
        ),
        (
            "plan-reviews",
            (
                "reviews/plans/a-review-1.md",
                Some(("plan-review", "a-review-1")),
            ),
        ),
        (
            "pr-reviews",
            (
                "reviews/prs/pr-7-review-1.md",
                Some(("pr-review", "pr-7-review-1")),
            ),
        ),
        (
            "work-item-reviews",
            (
                "reviews/work/0042-review-1.md",
                Some(("work-item-review", "0042-review-1")),
            ),
        ),
        (
            "validations",
            ("validations/a.md", Some(("plan-validation", "a"))),
        ),
        ("notes", ("notes/a-note.md", Some(("note", "a-note")))),
        (
            "pr-descriptions",
            (
                "prs/2026-01-02-pr-123-x.md",
                Some(("pr-description", "123")),
            ),
        ),
        (
            "design-gaps",
            ("gaps/a-gap.md", Some(("design-gap", "a-gap"))),
        ),
        (
            "design-inventories",
            (
                "inventories/2026-01-02-app/inventory.md",
                Some(("design-inventory", "2026-01-02-app")),
            ),
        ),
        (
            "topic-research",
            (
                "topics/mcp/manifest.md",
                Some(("topic-research", "manifest")),
            ),
        ),
        (
            "root-cause-analyses",
            ("issues/outage.md", Some(("issue-research", "outage"))),
        ),
        ("templates", ("templates/plan.md", None)),
    ];
    assert_covers_every_built_in(&table);
    let directories: Vec<(DocTypeKey, PathBuf)> = table
        .iter()
        .map(|(token, (path, _))| {
            let directory = Path::new(path).parent().expect("a directory");
            let directory = if *token == "design-inventories"
                || *token == "topic-research"
            {
                directory.parent().expect("a type directory")
            } else {
                directory
            };
            (kind(token), Path::new("meta").join(directory))
        })
        .collect();
    for (token, (path, address)) in table {
        let resolved = resolve_path_target(
            &format!("meta/{path}"),
            &directories,
            &FourDigitIds,
        );
        assert_eq!(
            resolved.as_ref().map(|(name, id)| (*name, id.as_str())),
            address,
            "{token}"
        );
    }
}

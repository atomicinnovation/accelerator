// Seed data for Accelerator visualiser prototype.
// Modelled after the spec's ten doc types. Content kept realistic so the
// design reads as a plausible working repo, not lorem ipsum.

const DOC_TYPES = [
  { key: "work",                label: "Work items",         count: 14, pulse: false, kind: "doc" },
  { key: "work-reviews",        label: "Work item reviews",  count: 11, pulse: true,  kind: "doc" },
  { key: "design-inventories",  label: "Design inventories", count: 4,  pulse: true,  kind: "doc" },
  { key: "design-gaps",         label: "Design gaps",        count: 3,  pulse: false, kind: "doc" },
  { key: "research",            label: "Research",           count: 12, pulse: false, kind: "doc" },
  { key: "plans",               label: "Plans",              count: 18, pulse: true,  kind: "doc" },
  { key: "plan-reviews",        label: "Plan reviews",       count: 22, pulse: false, kind: "doc" },
  { key: "validations",         label: "Validations",        count: 7,  pulse: false, kind: "doc" },
  { key: "pr-descriptions",     label: "PR descriptions",    count: 6,  pulse: false, kind: "doc" },
  { key: "pr-reviews",          label: "PR reviews",         count: 8,  pulse: false, kind: "doc" },
  { key: "decisions",           label: "Decisions",          count: 9,  pulse: true,  kind: "doc" },
  { key: "notes",               label: "Notes",              count: 0,  pulse: false, kind: "doc" },
];

// Library grouping: keeps the left-nav and the Library landing in sync. Work
// units are the primary artifacts; research/plans/reviews are the in-flight
// material; shipping is the code artifacts; supporting holds durable
// cross-cutting records (decisions, notes) that don't belong to a single unit.
const LIBRARY_GROUPS = [
  { key: "define",   label: "Define",   types: ["work", "work-reviews"] },
  { key: "discover", label: "Discover", types: ["design-inventories", "design-gaps", "research"] },
  { key: "build",    label: "Build",    types: ["plans", "plan-reviews", "validations"] },
  { key: "ship",     label: "Ship",     types: ["pr-descriptions", "pr-reviews"] },
  { key: "remember", label: "Remember", types: ["decisions", "notes"] },
];

const VIEWS = [
  { key: "kanban",    label: "Kanban",    kind: "view" },
  { key: "lifecycle", label: "Lifecycle", kind: "view" },
];

const META = [
  { key: "templates", label: "Templates", count: 5, kind: "meta" },
];

// Lifecycle stages, in canonical order. The spec calls out: work-item → research
// → plan → plan-review → validation → PR → pr-review → decision.
const STAGES = [
  { key: "work",          short: "WRK", label: "Work item",    hue: 0 },
  { key: "research",      short: "RSC", label: "Research",     hue: 28 },
  { key: "plans",         short: "PLN", label: "Plan",         hue: 220 },
  { key: "plan-reviews",  short: "P/R", label: "Plan review",  hue: 260 },
  { key: "validations",   short: "VAL", label: "Validation",   hue: 160 },
  { key: "pr-descriptions", short: "PR",  label: "PR description", hue: 200 },
  { key: "pr-reviews",    short: "P/R", label: "PR review",    hue: 280 },
  { key: "decisions",     short: "ADR", label: "Decision",     hue: 355 },
];

// A handful of work-unit clusters. Each entry represents one slug; the stages
// array maps to presence/absence in the cluster.
const CLUSTERS = [
  {
    slug: "three-layer-review-system-architecture",
    title: "Three-layer review system architecture",
    status: "in-progress",
    updated: "2m ago",
    owner: "Toby Clemson",
    present: ["work", "research", "plans", "plan-reviews", "validations", "decisions"],
    entries: [
      { type: "work",      id: "PROJ-0001", title: "Add three-layer review pipeline",                date: "2026-02-18", mtime: "2m ago",  status: "in-progress" },
      { type: "research",     id: "2026-02-20",  title: "Prior art: multi-lens code review systems",       date: "2026-02-20", mtime: "2d ago" },
      { type: "plans",        id: "2026-02-22",  title: "Three-layer review system architecture",          date: "2026-02-22", mtime: "18h ago" },
      { type: "plan-reviews", id: "review-1",    title: "Plan review · round 1",                           date: "2026-03-01", mtime: "14d ago", verdict: "approve-with-changes" },
      { type: "plan-reviews", id: "review-2",    title: "Plan review · round 2",                           date: "2026-03-08", mtime: "7d ago",  verdict: "approve" },
      { type: "validations",  id: "2026-03-12",  title: "Validation: agent / orchestrator / convention",    date: "2026-03-12", mtime: "3d ago",  verdict: "pass" },
      { type: "decisions",    id: "ADR-0002",    title: "Three-layer review system architecture",           date: "2026-03-14", mtime: "2m ago" },
    ],
  },
  {
    slug: "pr-review-agents",
    title: "PR review agents",
    status: "in-progress",
    updated: "1h ago",
    owner: "Toby Clemson",
    present: ["work", "research", "plans", "plan-reviews", "pr-descriptions", "pr-reviews"],
    entries: [
      { type: "work",      id: "PROJ-0007", title: "Ship PR review agents behind a flag",  date: "2026-02-22", mtime: "1h ago", status: "in-progress" },
      { type: "research",     id: "2026-02-25",  title: "Agent orchestration patterns",          date: "2026-02-25", mtime: "1w ago" },
      { type: "plans",        id: "2026-03-02",  title: "PR review agents — design",             date: "2026-03-02", mtime: "3d ago" },
      { type: "plan-reviews", id: "review-1",    title: "Plan review · round 1",                 date: "2026-03-06", mtime: "5d ago", verdict: "request-changes" },
      { type: "pr-descriptions", id: "PR-133",   title: "feat(agents): pr-review scaffolding",   date: "2026-04-08", mtime: "2h ago" },
      { type: "pr-reviews",   id: "review-1",    title: "PR review · round 1",                   date: "2026-04-12", mtime: "1h ago", verdict: "request-changes" },
    ],
  },
  {
    slug: "meta-visualisation",
    title: "Meta directory visualiser",
    status: "in-progress",
    updated: "5m ago",
    owner: "Toby Clemson",
    present: ["work", "research", "plans", "plan-reviews", "notes"],
    entries: [
      { type: "work",      id: "META-0011", title: "Browser-based visualiser for meta/",    date: "2026-04-14", mtime: "5m ago", status: "in-progress" },
      { type: "research",     id: "2026-04-15",  title: "Companion tools for Claude Code",        date: "2026-04-15", mtime: "8d ago" },
      { type: "plans",        id: "2026-04-17",  title: "Meta directory visualiser — design",     date: "2026-04-17", mtime: "4d ago" },
      { type: "plan-reviews", id: "review-1",    title: "Plan review · round 1",                  date: "2026-04-18", mtime: "3d ago", verdict: "approve-with-changes" },
      { type: "notes",        id: "2026-04-19",  title: "Open questions on SSE reconnect",        date: "2026-04-19", mtime: "2d ago" },
    ],
  },
  {
    slug: "config-resolve-template",
    title: "Three-tier template resolver",
    status: "done",
    updated: "11d ago",
    owner: "Toby Clemson",
    present: ["work", "plans", "plan-reviews", "validations", "pr-descriptions", "pr-reviews", "decisions"],
    entries: [
      { type: "work",      id: "META-0004", title: "Config resolves templates across tiers", date: "2026-01-14", mtime: "11d ago", status: "done" },
      { type: "plans",        id: "2026-01-18",  title: "Three-tier template resolver",            date: "2026-01-18", mtime: "1mo ago" },
      { type: "plan-reviews", id: "review-1",    title: "Plan review · round 1",                   date: "2026-01-21", mtime: "1mo ago", verdict: "approve" },
      { type: "validations",  id: "2026-01-30",  title: "Validation: resolver correctness",        date: "2026-01-30", mtime: "1mo ago", verdict: "pass" },
      { type: "pr-descriptions", id: "PR-091",   title: "feat(config): tier resolver",             date: "2026-02-04", mtime: "3w ago" },
      { type: "pr-reviews",   id: "review-1",    title: "PR review · round 1",                     date: "2026-02-05", mtime: "3w ago", verdict: "approve" },
      { type: "decisions",    id: "ADR-0001",    title: "Template resolution priority order",      date: "2026-02-06", mtime: "3w ago" },
    ],
  },
  {
    slug: "kanban-status-writes",
    title: "Kanban status writes",
    status: "todo",
    updated: "6d ago",
    owner: "Toby Clemson",
    present: ["work", "plans"],
    entries: [
      { type: "work", id: "PROJ-0013", title: "Expose work item status writes", date: "2026-04-14", mtime: "6d ago", status: "todo" },
      { type: "plans",   id: "2026-04-16",  title: "Kanban status patch path",     date: "2026-04-16", mtime: "5d ago" },
    ],
  },
  {
    slug: "frontmatter-patcher",
    title: "YAML-aware frontmatter patcher",
    status: "done",
    updated: "2w ago",
    owner: "Toby Clemson",
    present: ["work", "research", "plans", "validations", "pr-descriptions", "pr-reviews"],
    entries: [
      { type: "work",    id: "META-0006", title: "Surgical frontmatter patcher",       date: "2026-03-01", mtime: "6w ago", status: "done" },
      { type: "research",   id: "2026-03-03",  title: "YAML-aware patching strategies",     date: "2026-03-03", mtime: "6w ago" },
      { type: "plans",      id: "2026-03-05",  title: "Patcher module design",               date: "2026-03-05", mtime: "6w ago" },
      { type: "validations",id: "2026-03-15",  title: "Validation: comment preservation",   date: "2026-03-15", mtime: "5w ago", verdict: "pass" },
      { type: "pr-descriptions", id: "PR-118", title: "feat(patcher): yaml-aware writes",    date: "2026-03-20", mtime: "4w ago" },
      { type: "pr-reviews", id: "review-1",    title: "PR review · round 1",                 date: "2026-03-21", mtime: "2w ago", verdict: "approve" },
    ],
  },
  {
    slug: "sse-reconnect-backoff",
    title: "SSE reconnect with backoff",
    status: "todo",
    updated: "4d ago",
    owner: "Toby Clemson",
    present: ["work", "research"],
    entries: [
      { type: "work",  id: "ENG-0014", title: "Graceful SSE reconnect on disconnect", date: "2026-04-15", mtime: "4d ago", status: "todo" },
      { type: "research", id: "2026-04-16",  title: "SSE reconnect patterns in browsers",   date: "2026-04-16", mtime: "4d ago" },
    ],
  },
  {
    slug: "binary-acquisition",
    title: "Binary acquisition flow",
    status: "done",
    updated: "1mo ago",
    owner: "Toby Clemson",
    present: ["work", "plans", "plan-reviews", "validations", "pr-descriptions", "decisions"],
    entries: [
      { type: "work",    id: "ENG-0003", title: "Fetch server binary from releases",   date: "2026-02-10", mtime: "2mo ago", status: "done" },
      { type: "plans",      id: "2026-02-12",  title: "Binary download + checksum verify",   date: "2026-02-12", mtime: "2mo ago" },
      { type: "plan-reviews",id: "review-1",   title: "Plan review · round 1",               date: "2026-02-14", mtime: "2mo ago", verdict: "approve" },
      { type: "validations",id: "2026-02-20",  title: "Validation: checksum gate",           date: "2026-02-20", mtime: "2mo ago", verdict: "pass" },
      { type: "pr-descriptions", id: "PR-072", title: "feat(launch): release binary gate",   date: "2026-02-24", mtime: "2mo ago" },
      { type: "decisions",  id: "ADR-0003",    title: "Binaries via GitHub Releases",        date: "2026-02-26", mtime: "1mo ago" },
    ],
  },
];

// Flat list of work items for the Kanban, pulled from the clusters above plus a
// few extras so each column feels populated.
const WORK_ITEMS = [
  { id: "PROJ-0001", kind: "epic",  slug: "three-layer-review-system-architecture", title: "Add three-layer review pipeline",        status: "in-progress", mtime: "2m ago",  cluster: 0, linked: 7 },
  { id: "PROJ-0007", kind: "story", slug: "pr-review-agents",                       title: "Ship PR review agents behind a flag",    status: "in-progress", mtime: "1h ago",  cluster: 1, linked: 6 },
  { id: "META-0011", kind: "epic",  slug: "meta-visualisation",                     title: "Browser-based visualiser for meta/",      status: "in-progress", mtime: "5m ago",  cluster: 2, linked: 5 },
  { id: "PROJ-0013", kind: "story", slug: "kanban-status-writes",                   title: "Expose work item status writes",             status: "todo",        mtime: "6d ago",  cluster: 4, linked: 2 },
  { id: "ENG-0014",  kind: "bug",   slug: "sse-reconnect-backoff",                  title: "Graceful SSE reconnect on disconnect",    status: "todo",        mtime: "4d ago",  cluster: 6, linked: 2 },
  { id: "0015",      kind: "task",  slug: "rust-embed-dev-feature",                 title: "Dev feature flag for ServeDir",           status: "todo",        mtime: "1w ago",  linked: 1 },
  { id: "0016",      kind: "spike", slug: "owner-pid-watch",                        title: "Owner-PID death triggers shutdown",       status: "todo",        mtime: "2w ago",  linked: 1 },
  { id: "ENG-0017",  kind: "task",  slug: "idle-timeout",                           title: "30min idle timeout with graceful close",  status: "todo",        mtime: "2w ago",  linked: 1 },
  { id: "META-0004", kind: "story", slug: "config-resolve-template",                title: "Config resolves templates across tiers",  status: "done",        mtime: "11d ago", cluster: 3, linked: 7 },
  { id: "META-0006", kind: "story", slug: "frontmatter-patcher",                    title: "Surgical frontmatter patcher",            status: "done",        mtime: "6w ago",  cluster: 5, linked: 6 },
  { id: "ENG-0003",  kind: "task",  slug: "binary-acquisition",                     title: "Fetch server binary from releases",       status: "done",        mtime: "2mo ago", cluster: 7, linked: 6 },
  { id: "ENG-0009",  kind: "spike", slug: "etag-content-hash",                      title: "Strong ETag via sha256",                  status: "done",        mtime: "1mo ago", linked: 3 },
];

// Markdown-ish content samples for the library doc page. Stored as
// plain strings — a tiny renderer below converts headings, fences, lists.
const DOC_CONTENT = {
  "ADR-0002": {
    type: "decisions",
    slug: "three-layer-review-system-architecture",
    frontmatter: {
      title: "Three-layer review system architecture",
      type: "decision",
      status: "accepted",
      date: "2026-03-14",
      author: "Toby Clemson",
      work_item: "PROJ-0001",
      supersedes: null,
    },
    body: `# Three-layer review system architecture

## Context

The review system needs to evaluate code across multiple quality dimensions:
correctness, convention adherence, and architectural fit. A single monolithic
reviewer conflates these concerns and makes its verdicts harder to interpret
and to override.

We want each dimension to produce an independently readable verdict, while
still composing into a single decision the author can act on.

## Decision

Adopt **three layers** — agent, orchestrator, convention — each owning one
dimension of the review.

1. **Agent** — per-diff AI reviewer. Produces natural-language findings with
   severity markers and suggested patches.
2. **Orchestrator** — coordinates the agent across files and consolidates
   findings into a structured verdict.
3. **Convention** — rule-based checks that run independently of the LLM
   (lint, format, forbidden imports).

The orchestrator is the only layer the author interacts with directly. It
merges the three streams and surfaces them as a single PR-review artefact.

## Consequences

- Each layer is independently testable and replaceable.
- The orchestrator becomes the sole coupling point; its interface is the
  thing to get right.
- Convention rules stay deterministic and cheap; agent output stays opt-in
  and cacheable.

## Sketch

\`\`\`rust
/// One layer of the review pipeline. The trait is intentionally minimal —
/// each implementation owns its own caching, fan-out, and prompt formatting.
#[async_trait]
pub trait ReviewLayer: Send + Sync {
    fn name(&self) -> &'static str;
    async fn review(&self, diff: &Diff) -> Result<Vec<Finding>, ReviewError>;
}

pub struct Orchestrator {
    layers: Vec<Arc<dyn ReviewLayer>>,
}

impl Orchestrator {
    pub async fn run(&self, diff: &Diff) -> Verdict {
        let runs = self.layers.iter().map(|l| l.review(diff));
        let results = futures::future::join_all(runs).await;
        Verdict::merge(results) // collapses per-layer findings into one stream
    }
}
\`\`\`

A typical wiring at the binary boundary:

\`\`\`rust
let orchestrator = Orchestrator::new(vec![
    Arc::new(AgentLayer::with_model("claude-haiku-4-5")),
    Arc::new(ConventionLayer::from_workspace(&root)?),
    Arc::new(BlastRadiusLayer::default()),
]);
\`\`\`

## Links

- work-item [[PROJ-0001]]
- plan 2026-02-22 \`plans/2026-02-22-three-layer-review-system-architecture.md\`
- validation 2026-03-12
`,
  },
  "2026-04-17-plan": {
    type: "plans",
    slug: "meta-visualisation",
    frontmatter: {
      title: "Meta directory visualiser — design",
      type: "plan",
      status: "draft",
      date: "2026-04-17",
      last_updated: "2026-04-18",
      author: "Toby Clemson",
      slug: "meta-visualisation",
    },
    body: `# Meta directory visualiser — design

## Purpose

Provide a local, browser-based visualiser for the artifacts the accelerator
plugin writes into a project's \`meta/\` directory. The primary use case is
running the visualiser alongside an active Claude Code session — a companion
window that reads and lightly interacts with the artifacts Claude produces.

## Scope (v1)

Three views on the ten supported document types:

1. **Library** — a reader for every doc type.
2. **Lifecycle** — timeline view of work units formed by slug-clustering.
3. **Kanban** — a three-column board for work items.

## Architecture

The server is a single axum-based binary composed of focused modules. The
frontend is a React SPA, embedded into the server binary via \`rust-embed\`
at compile time.

\`\`\`
server ──► file_driver · indexer · watcher · sse_hub · patcher
             │
             └──► meta/**/*.md
\`\`\`

## Module sketch

The indexer is the load-bearing piece. It owns the in-memory document graph
and exposes a typed query API to the HTTP layer:

\`\`\`rust
/// In-memory document store. Built once at startup, mutated by the watcher.
pub struct Indexer {
    docs:     HashMap<DocId, Document>,
    by_slug:  HashMap<String, Vec<DocId>>,
    by_type:  HashMap<DocType, Vec<DocId>>,
    sse:      SseHub,
}

impl Indexer {
    pub fn new(root: &Path) -> Result<Self, IndexError> {
        let mut idx = Self::default();
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() && entry.path().extension() == Some("md".as_ref()) {
                idx.ingest(entry.path())?;
            }
        }
        Ok(idx)
    }

    /// Fetch every document in a cluster, in canonical lifecycle order.
    pub fn cluster(&self, slug: &str) -> Vec<&Document> {
        let mut docs: Vec<&Document> = self.by_slug
            .get(slug)
            .into_iter()
            .flatten()
            .filter_map(|id| self.docs.get(id))
            .collect();
        docs.sort_by_key(|d| STAGES.iter().position(|s| s == &d.kind));
        docs
    }
}
\`\`\`

The watcher pushes change events onto the SSE hub. Clients subscribe with
a single event source:

\`\`\`typescript
// Client-side: connect once at app boot, fan out via React context.
const source = new EventSource("/api/events");

source.addEventListener("doc:changed", (e) => {
  const { path, etag, type } = JSON.parse(e.data);
  queryClient.invalidateQueries({ queryKey: ["doc", path] });
  toaster.push({
    kind: "info",
    title: "External edit",
    body: \`A reviewer agent updated \${path}\`,
  });
});

source.addEventListener("error", () => {
  // Reconnect with exponential backoff, capped at 30s. SSE drops are common
  // on laptop wake; the indexer rebuilds the connection silently.
  scheduleReconnect();
});
\`\`\`

## Local launch

\`\`\`bash
# Build the binary and start a localhost-only server.
cargo build --release --features embed-ui
./target/release/accelerator-visualiser \\
    --root ./meta \\
    --port 0          # bind to a dynamic port
    --pid ./.acc.pid  # write a pid file so re-invocations reuse this instance
\`\`\`

## Links

- work-item [[META-0011]]
- research 2026-04-15
`,
  },
  "META-0011": {
    type: "work",
    slug: "meta-visualisation",
    frontmatter: {
      title: "Browser-based visualiser for meta/",
      type: "work-item",
      status: "in-progress",
      id: "0011",
      date: "2026-04-14",
      author: "Toby Clemson",
    },
    body: `# Browser-based visualiser for meta/

Provide a browser UI that reads every artifact in \`meta/\` and renders three
views: library, lifecycle, kanban.

## Acceptance

- Runs via \`/accelerator:visualise\` and \`accelerator-visualiser\` CLI.
- Binds to 127.0.0.1 on a dynamic port.
- Detects live instance via PID file; reuses rather than duplicating.
- Embedded frontend bundle — one static binary per arch.

## Links

- plan 2026-04-17
- research 2026-04-15
`,
  },
  "2026-04-18-review": {
    type: "plan-reviews",
    slug: "meta-visualisation",
    frontmatter: {
      title: "Plan review · round 1",
      type: "plan-review",
      verdict: "approve-with-changes",
      date: "2026-04-18",
      author: "Reviewer agent",
      target: "plans/2026-04-17-meta-visualisation.md",
      round: 1,
    },
    body: `# Plan review · round 1

## Verdict

\`approve-with-changes\` — scope is well-bounded; three structural notes.

## Findings

### 1. SSE reconnect semantics (medium)

The plan assumes transparent reconnect but does not specify what happens to
in-flight PATCH requests on the wire when the SSE drops. Recommend:

- Explicit \`invalidate-all\` on reconnect (already noted).
- Describe backoff curve (exponential up to 30s).
- Surface disconnect state in the UI.

### 2. Binary checksum manifest (low)

The committed \`checksums.json\` is the gate, but the plan does not describe
what happens if the manifest drifts from the release assets. Recommend: the
launcher should error out verbosely with the mismatched hash, not silently
re-download.

### 3. Templates virtual type (low)

The \`TemplateDetail\` shape is clean; one question: does the UI need a diff
view between tiers? Current design shows them in priority order but does not
call out the differences.

## Links

- target plan 2026-04-17 \`plans/2026-04-17-meta-visualisation.md\`
- work-item [[META-0011]]
`,
  },
  "WR-0030-r4": {
    type: "work-reviews",
    slug: "centralise-path-defaults",
    frontmatter: {
      title: "Work item review · centralise PATH defaults · round 4",
      type: "work-item-review",
      verdict: "approve",
      date: "2026-05-08",
      author: "Reviewer agent",
      target: "meta/work/0030-centralise-path-defaults.md",
      work_item: "0030",
      review_number: 4,
      lenses: "clarity, completeness, dependency, scope, testability",
    },
    body: `# Work Item Review: Centralise PATH and TEMPLATE config arrays

**Verdict:** COMMENT

No major findings remain. The work item is acceptable for implementation. The remaining observations are all minor or suggestion-level — labelling consistency, a cross-criterion reference in AC1, and the DIR_KEYS follow-on tracking gap.

## Cross-Cutting Themes

- **TEMPLATE_DEFAULTS contradiction** — resolved across all four passes. The title, Summary, Requirements, and Acceptance Criteria now consistently reference three arrays.
- **DIR_KEYS / DIR_DEFAULTS** — moved into Open Questions with a clear deferral; tracked via a placeholder follow-on work item.
- **AC3 regression baseline** — anchored to named test categories (\`config-dump\`, \`config-init\`, \`path-resolution\`).

## Findings

### Minor

- 🔵 **Clarity**: AC1 references "the same entries as the pre-migration definitions" — ambiguous referent across four parallel definition files.
- 🔵 **Clarity**: "consumer sites" in Assumptions vs "consumer scripts" in Summary — pick one label.
- 🔵 **Completeness**: Requirements does not explicitly state the 11 consumer scripts are intentionally unmodified.
- 🔵 **Dependency**: DIR_KEYS follow-on absent from Blocks; planning tools won't surface it.
- 🔵 **Testability**: AC1 defers content correctness to AC3 without defining "same entries".

### Strengths

- ✅ Grep-based AC2 produces an unambiguous pass/fail result.
- ✅ Technical Notes provide exact file paths and line ranges.
- ✅ Bidirectional coupling with 0052 is documented in both work items.
- ✅ Assumptions section resolves former open questions about co-location.

## Re-Review History

| Pass | Verdict | Resolved | Outstanding |
|------|---------|----------|-------------|
| 1    | REVISE  | 0/6      | 6 major     |
| 2    | REVISE  | 3/6      | 3 major     |
| 3    | REVISE  | 5/6      | 1 major + 2 new |
| 4    | COMMENT | 6/6      | 7 minor     |

## Verification snippet

The grep-based acceptance criterion runs against the workspace root and
should return exactly four hits — one per definition site — and nothing else:

\`\`\`bash
# AC2: confirm the migration left only the expected definition sites.
$ rg --no-heading --line-number 'declare -ra (PATH|TEMPLATE)_DEFAULTS' \\
     --type sh meta/ scripts/
meta/config/defaults.sh:4:declare -ra PATH_DEFAULTS=(
meta/config/defaults.sh:18:declare -ra TEMPLATE_DEFAULTS=(
meta/config/defaults.sh:32:declare -ra DIR_DEFAULTS=(   # tracked separately
scripts/lib/legacy_paths.sh:9:declare -ra PATH_DEFAULTS=(   # to be removed in 0052
\`\`\`

## Links

- target work item [[WORK-ITEM-0030]]
- references [[ADR-0023]]
`,
  },
  "DI-2026-05-06-current-app": {
    type: "design-inventories",
    slug: "design-system-rollup",
    frontmatter: {
      title: "Design inventory: current-app",
      type: "design-inventory",
      source: "current-app",
      source_kind: "running-app",
      source_location: "http://127.0.0.1:51771/",
      git_commit: "94a0ed6",
      branch: "main",
      crawler: "hybrid",
      author: "Toby Clemson",
      status: "draft",
      sequence: 1,
      date: "2026-05-06",
    },
    body: `# Design Inventory: current-app

## Overview

**Scope:** the Accelerator Visualiser frontend (React 19 SPA served by a local Rust HTTP server). Crawl covered every top-level navigation entry exposed by the sidebar — Documents (12 doc-type list views + per-document detail views), Views (Lifecycle index + cluster detail, Kanban board), and Meta (Templates index + per-template detail).

**Crawler methodology:** hybrid. Code-static analysis was the ground truth for design tokens, components, and routes. Runtime navigation captured screen states and screenshots from a live server instance.

## Screenshots

[[screenshots:6]]

## Design System

### Color tokens

| Token                      | Value     | Source                    |
| -------------------------- | --------- | ------------------------- |
| \`--color-text\`             | \`#0f172a\` | \`src/styles/global.css:2\` |
| \`--color-muted-text\`       | \`#4b5563\` | \`src/styles/global.css:3\` |
| \`--color-divider\`          | \`#e5e7eb\` | \`src/styles/global.css:5\` |
| \`--color-focus-ring\`       | \`#2563eb\` | \`src/styles/global.css:6\` |
| \`--color-warning-bg\`       | \`#fff8e6\` | \`src/styles/global.css:7\` |
| \`--color-warning-text\`     | \`#7c2d12\` | \`src/styles/global.css:9\` |

Plus ~14 hard-coded hex values used inline across CSS modules — a de-facto palette not yet promoted to tokens.

### Token snapshot (verbatim from \`global.css\`)

\`\`\`css
:root {
  --color-text:         #0f172a;
  --color-muted-text:   #4b5563;
  --color-divider:      #e5e7eb;
  --color-focus-ring:   #2563eb;
  --color-warning-bg:   #fff8e6;
  --color-warning-text: #7c2d12;
}

/* Hard-coded inline — flagged for promotion. */
.kanban-column {
  background: #f9fafb;
  border: 1px solid #d1d5db;
}
.toast--err { background: #fee2e2; color: #991b1b; }
\`\`\`

### Typography

- Body / chrome: \`system-ui, sans-serif\`
- Monospace: \`monospace\` (slug, path, etag)
- Code highlighting: \`highlight.js/styles/github.css\`
- Body line-height: \`1.6\`

### Layout primitives

- Sidebar width: \`220px\`
- Library list / lifecycle index max-width: \`900px\`
- Doc article max-width: \`1100px\` with right aside \`260px\`
- Kanban column min-width: \`16rem\`

## Component catalogue

### RootLayout

App shell — sidebar + main scroll area; provides \`DocEventsContext\`.

### Sidebar

Partitions doc types into Documents / Meta + a fixed Views group.

### LibraryTypeView

Sortable table of documents for one doc-type. Sort driven by column-header clicks.

### LibraryDocView

Single document — title + frontmatter chips + markdown body + related artifacts aside.

### PipelineDots

Row of present/absent dots indicating workflow-pipeline completeness.

## Known gaps

- Per-document detail screenshots were captured for only a subset of doc types.
- No loading-state screenshots — queries resolve instantly against local backend.
- Drag-and-drop interaction states for the Kanban board were not exercised.
`,
  },
  "DG-2026-05-06-current-vs-prototype": {
    type: "design-gaps",
    slug: "design-system-rollup",
    frontmatter: {
      title: "Design gap: current-app → claude-design-prototype",
      type: "design-gap",
      current_inventory: "DI-2026-05-06-current-app",
      target_inventory: "DI-2026-05-06-claude-design-prototype",
      author: "Toby Clemson",
      status: "draft",
      date: "2026-05-06",
    },
    body: `# Design Gap Analysis: current-app → claude-design-prototype

## Overview

This analysis compares the running Accelerator Visualiser frontend against the Claude design prototype. The current app is a fully functional implementation; the prototype proposes a substantially more developed visual language — a layered token system, a top-bar app shell, lifecycle-phase-grouped sidebar navigation, an activity feed, and a richer set of decorated components.

## Token drift

The current app exposes only eight named CSS custom properties for colour and relies on ~14 hard-coded hex values. The prototype defines a layered token system with a brand palette (\`--atomic-*\`), legacy semantic aliases (\`--fg-*\`, \`--bg-*\`), and an active semantic surface layer (\`--ac-*\`) with full light/dark overrides under \`[data-theme]\`.

The current app has no formal typography scale. The prototype defines a four-family stack (Sora display, Inter body, Fira Code mono, Raleway support) plus an eleven-step size scale.

The current app has no formal spacing or radius scales — each module hard-codes values. The prototype defines an eleven-step spacing scale and a four-step radius scale.

### Concrete diff: token block

\`\`\`diff
 :root {
-  --color-text: #0f172a;
-  --color-muted-text: #4b5563;
-  --color-divider: #e5e7eb;
+  --ac-fg-strong:  #0A111B;
+  --ac-fg:         #14161F;
+  --ac-fg-muted:   #5F6378;
+  --ac-fg-faint:   #8B90A3;
+  --ac-stroke:     rgba(32, 34, 49, 0.10);
+  --ac-accent:     #595FC8;   /* indigo, product accent */
 }

+[data-theme="dark"] {
+  --ac-bg:         #0A111B;
+  --ac-fg-strong:  #FFFFFF;
+  --ac-stroke:     rgba(255, 255, 255, 0.08);
+  --ac-accent:     #8A90E8;
+}
\`\`\`

### Type stack — proposed

\`\`\`json
{
  "fonts": {
    "display": ["Sora", "system-ui", "sans-serif"],
    "body":    ["Inter", "system-ui", "sans-serif"],
    "mono":    ["Fira Code", "ui-monospace", "monospace"],
    "support": ["Raleway", "system-ui", "sans-serif"]
  },
  "scale": {
    "11": 68, "10": 48, "9": 36, "8": 28, "7": 22,
    "6": 18, "5": 16, "4": 14, "3": 12, "2": 11, "1": 10
  }
}
\`\`\`

## Component drift

- **Sidebar grouping** — current app partitions doc types Documents / Meta; prototype groups by lifecycle phase (Define / Build / Ship / Remember).
- **Topbar** — net-new; current app has none. Brand wordmark, breadcrumbs, server-origin pill, SSE indicator, theme toggle.
- **Chip vocabulary** — current app has only \`FrontmatterChips\`; prototype defines a generic \`Chip\` with five variants.
- **Glyph** — net-new; per-doc-type colored square icon at multiple sizes.
- **Toaster** — net-new ephemeral notification component.
- **Activity feed** — net-new sidebar block, rolling file-change events with per-doc-type glyphs.

## Screen drift

- **Library overview hub** — current redirects to \`/library/decisions\`; prototype groups doc types by phase with per-type cards.
- **Lifecycle cluster detail** — needs hexchain header strip above the existing timeline.
- **Templates view** — needs inline tier-presence row and sha256 etag header.

## Net-new features

- Light / dark theming via \`data-theme\` attribute.
- Font-mode swap via \`[data-font="mono"]\`.
- Sidebar search box with \`/\` keyboard shortcut.
- Unseen-changes dot per doc type.
- External-edit toast triggered by SSE.
- Server-origin pill with green pulse.

## Suggested sequencing

The token migration is the load-bearing prerequisite — until \`--ac-*\` tokens exist, no component re-skin can land without diverging. Sequence: token introduction first, then theming and font-mode, then component refits in parallel (Topbar, Sidebar, Chip, Glyph), then net-new features, then per-document detail screen redesign.

## Links

- current inventory [[DI-2026-05-06-current-app]]
- target inventory [[DI-2026-05-06-claude-design-prototype]]
- related [[ADR-0024]] *Configurable Kanban Column Set*
`,
  },
  "PR-133": {
    type: "pr-descriptions",
    slug: "test-restructure",
    frontmatter: {
      title: "Reorganize tests: move from src/ to top-level tests/",
      type: "pr-description",
      pr_number: 133,
      status: "merged",
      date: "2026-04-02",
      author: "Toby Clemson",
      jira: "PRI-3375",
    },
    body: `# Reorganize tests: move from src/ to top-level tests/

## Summary

This PR refactors the test structure by moving all tests out of the \`src/\` directory into a top-level \`tests/\` directory, improving separation of concerns and following Python best practices for project structure.

## Problem

Tests were previously located inside the \`src/\` directory alongside application code. This structure mixes application code with test code, makes it harder to exclude tests from production builds, and doesn't follow modern Python project conventions.

## Solution

Reorganize the entire test suite into a top-level \`tests/\` directory with clear separation by test type:

- \`tests/unit/\` — unit tests organized by Django app
- \`tests/integration/\` — integration tests organized by Django app
- \`tests/shared/test_support/\` — shared test utilities and factories

Update all configuration files (pytest, mypy, makefiles, dockerignore) to support the new structure. Use pytest's \`--import-mode=importlib\` to enable cleaner imports without requiring \`__init__.py\` files.

## Changes

### Test directory restructure

- \`src/core/tests/unit_tests/\` → \`tests/unit/core/\`
- \`src/liquidity_providers/tests/\` → \`tests/{unit,integration}/liquidity_providers/\`
- \`src/pricing/tests/\` → \`tests/{unit,integration}/pricing/\`
- \`src/spreads/tests/\` → \`tests/{unit,integration}/spreads/\`

### Configuration updates

- **makefiles/test.mk** — point at \`tests/unit\` and \`tests/integration\` instead of pytest markers.
- **mypy.ini** — \`files = src,tests\`; \`mypy_path = tests/shared\`; \`explicit_package_bases = True\`.
- **pytest.ini & pyproject.toml** — \`pythonpath\` includes \`tests/shared\`; \`--import-mode=importlib\`.

### Example fixture migration

\`\`\`python
# tests/shared/test_support/factories.py
from dataclasses import dataclass
from decimal import Decimal

import pytest

from pricing.models import Quote


@dataclass(frozen=True)
class QuoteFactory:
    symbol: str = "EURUSD"
    bid: Decimal = Decimal("1.0850")
    ask: Decimal = Decimal("1.0852")

    def build(self, **overrides) -> Quote:
        return Quote(**{**self.__dict__, **overrides})


@pytest.fixture
def quote_factory() -> QuoteFactory:
    return QuoteFactory()
\`\`\`

### pyproject.toml diff

\`\`\`toml
[tool.pytest.ini_options]
minversion = "7.0"
testpaths = ["tests/unit", "tests/integration"]
pythonpath = ["src", "tests/shared"]
addopts = "--import-mode=importlib --strict-markers -ra"

[tool.mypy]
files = ["src", "tests"]
mypy_path = "tests/shared"
explicit_package_bases = true
strict = true
\`\`\`

## Breaking changes

None — internal test structure refactoring with no application-code changes.

## Verification

\`\`\`bash
# Unit + integration suites both pass against the new layout.
$ make unit-test
pytest tests/unit -q
............................................ [100%]
40 passed in 1.42s

$ make integration-test
pytest tests/integration -q
.............................................................. [ 25%]
.............................................................. [ 50%]
.............................................................. [ 75%]
............................................................   [100%]
246 passed in 18.7s

$ make pylint
pylint src tests --rcfile=.pylintrc
------------------------------------
Your code has been rated at 10.00/10
\`\`\`
`,
  },
  "adr-template": {
    type: "templates",
    name: "adr",
    tiers: [
      { source: "config-override", path: ".claude/accelerator/templates/adr.md", present: false, active: false },
      { source: "user-override",   path: "meta/templates/adr.md",                present: true,  active: true,  etag: "sha256-8f1a…",
        content: `---
title: "ADR-0007: Slug-based clustering as lifecycle anchor"
type: decision
status: proposed
date: 2026-04-08
author: jordan
work_item: META-0011
supersedes: null
---

# ADR-0007: Slug-based clustering as lifecycle anchor

## Context

Work units span many doc types — research, plan, review, PR description.
We need a stable identifier that survives renames and keeps every artifact
for one unit grouped together, without standing up a new database.

## Decision

Cluster by frontmatter \`slug\`. Filenames stay free; slug is the join key.

## Consequences

Easier: lifecycle view, related-artifact rollups, cross-doc navigation.
Harder: enforcing slug uniqueness — a CI check catches collisions for now.
Revisit if the collision rate climbs above 1% across the corpus.

## Links

- work-item [[META-0011]]
- supersedes [[ADR-0003]]
` },
      { source: "plugin-default",  path: "<plugin-root>/templates/adr.md",        present: true,  active: false, etag: "sha256-3c72…",
        content: `---
title: "ADR-0005: Rust + axum for the visualiser server"
type: decision
status: accepted
date: 2026-04-02
author: alex
---

# ADR-0005: Rust + axum for the visualiser server

## Context

The visualiser needs to ship as one cross-platform binary, talk to the
filesystem at native speed, and embed a static SPA bundle with no
runtime dependencies on the host machine.

## Decision

Build the server in Rust with \`axum\` for routing and \`tower-http\` for
static-file serving. Embed the SPA bundle via \`include_dir!\` at build
time so distribution stays a single binary.

## Consequences

Single 12MB binary, no runtime install. Slightly higher build complexity
than a Node prototype would carry. Hot reload during development is handled
by a separate Vite dev server proxying through to the binary.
` },
    ],
  },
};

// Activity feed — most recent mutations across all types.
const ACTIVITY = [
  { type: "plans",        doc: "2026-04-17-meta-visualisation.md",           when: "2m ago",  action: "edited",  slug: "meta-visualisation" },
  { type: "work",      doc: "PROJ-0007.md",                             when: "1h ago",  action: "moved to in-progress", slug: "pr-review-agents" },
  { type: "pr-reviews",   doc: "2026-04-12-pr-review-agents-review-1.md",    when: "1h ago",  action: "created", slug: "pr-review-agents" },
  { type: "plan-reviews", doc: "2026-04-18-meta-visualisation-review-1.md",  when: "3d ago",  action: "created", slug: "meta-visualisation" },
  { type: "research",     doc: "2026-04-16-sse-reconnect-patterns.md",       when: "4d ago",  action: "created", slug: "sse-reconnect-backoff" },
  { type: "notes",        doc: "2026-04-19-open-questions-sse.md",           when: "2d ago",  action: "edited",  slug: "meta-visualisation" },
];

// Per-type index listings shown on /library/:type
const LIBRARY_INDEX = {
  decisions: [
    { id: "ADR-0003", title: "Binaries via GitHub Releases",               date: "2026-02-26", status: "accepted", slug: "binary-acquisition" },
    { id: "ADR-0002", title: "Three-layer review system architecture",     date: "2026-03-14", status: "accepted", slug: "three-layer-review-system-architecture" },
    { id: "ADR-0001", title: "Template resolution priority order",        date: "2026-02-06", status: "accepted", slug: "config-resolve-template" },
    { id: "ADR-0004", title: "ETag scheme uses sha256 content hash",       date: "2026-03-22", status: "accepted", slug: "etag-content-hash" },
    { id: "ADR-0005", title: "Rust + axum for the visualiser server",      date: "2026-04-02", status: "accepted", slug: "meta-visualisation" },
    { id: "ADR-0006", title: "Two distinct review doc types",              date: "2026-04-05", status: "accepted", slug: "meta-visualisation" },
    { id: "ADR-0007", title: "Slug-based clustering as lifecycle anchor",  date: "2026-04-08", status: "proposed", slug: "meta-visualisation" },
    { id: "ADR-0008", title: "Localhost-only with no auth in v1",          date: "2026-04-10", status: "proposed", slug: "meta-visualisation" },
    { id: "ADR-0009", title: "Dynamic port + PID file for instance reuse", date: "2026-04-11", status: "proposed", slug: "meta-visualisation" },
  ],
  work: WORK_ITEMS.map(t => ({ id: t.id, title: t.title, date: t.mtime, status: t.status, slug: t.slug })),
  plans: [
    { id: "2026-04-17", title: "Meta directory visualiser — design",       date: "2026-04-17", status: "draft",     slug: "meta-visualisation" },
    { id: "2026-04-16", title: "Kanban status patch path",                  date: "2026-04-16", status: "draft",     slug: "kanban-status-writes" },
    { id: "2026-03-05", title: "Patcher module design",                     date: "2026-03-05", status: "accepted",  slug: "frontmatter-patcher" },
    { id: "2026-03-02", title: "PR review agents — design",                 date: "2026-03-02", status: "accepted",  slug: "pr-review-agents" },
    { id: "2026-02-22", title: "Three-layer review system architecture",    date: "2026-02-22", status: "accepted",  slug: "three-layer-review-system-architecture" },
    { id: "2026-02-12", title: "Binary download + checksum verify",         date: "2026-02-12", status: "accepted",  slug: "binary-acquisition" },
    { id: "2026-01-18", title: "Three-tier template resolver",              date: "2026-01-18", status: "accepted",  slug: "config-resolve-template" },
  ],
  research: [
    { id: "2026-04-16", title: "SSE reconnect patterns in browsers",         date: "2026-04-16", slug: "sse-reconnect-backoff" },
    { id: "2026-04-15", title: "Companion tools for Claude Code",            date: "2026-04-15", slug: "meta-visualisation" },
    { id: "2026-03-03", title: "YAML-aware patching strategies",             date: "2026-03-03", slug: "frontmatter-patcher" },
    { id: "2026-02-25", title: "Agent orchestration patterns",                date: "2026-02-25", slug: "pr-review-agents" },
    { id: "2026-02-20", title: "Prior art: multi-lens code review systems",  date: "2026-02-20", slug: "three-layer-review-system-architecture" },
  ],
  "plan-reviews": [
    { id: "review-1", title: "Plan review · round 1 — meta-visualisation",  date: "2026-04-18", verdict: "approve-with-changes", slug: "meta-visualisation" },
    { id: "review-2", title: "Plan review · round 2 — three-layer-review",  date: "2026-03-08", verdict: "approve",               slug: "three-layer-review-system-architecture" },
    { id: "review-1", title: "Plan review · round 1 — three-layer-review",  date: "2026-03-01", verdict: "approve-with-changes", slug: "three-layer-review-system-architecture" },
    { id: "review-1", title: "Plan review · round 1 — pr-review-agents",    date: "2026-03-06", verdict: "request-changes",      slug: "pr-review-agents" },
    { id: "review-1", title: "Plan review · round 1 — binary-acquisition",  date: "2026-02-14", verdict: "approve",               slug: "binary-acquisition" },
  ],
  "pr-reviews": [
    { id: "review-1", title: "PR review · round 1 — pr-review-agents",      date: "2026-04-12", verdict: "request-changes", slug: "pr-review-agents" },
    { id: "review-1", title: "PR review · round 1 — frontmatter-patcher",   date: "2026-03-21", verdict: "approve",         slug: "frontmatter-patcher" },
    { id: "review-1", title: "PR review · round 1 — config-resolve-template", date: "2026-02-05", verdict: "approve",         slug: "config-resolve-template" },
  ],
  validations: [
    { id: "2026-03-15", title: "Validation: comment preservation",           date: "2026-03-15", verdict: "pass", slug: "frontmatter-patcher" },
    { id: "2026-03-12", title: "Validation: agent / orchestrator / convention", date: "2026-03-12", verdict: "pass", slug: "three-layer-review-system-architecture" },
    { id: "2026-02-20", title: "Validation: checksum gate",                  date: "2026-02-20", verdict: "pass", slug: "binary-acquisition" },
    { id: "2026-01-30", title: "Validation: resolver correctness",           date: "2026-01-30", verdict: "pass", slug: "config-resolve-template" },
  ],
  "pr-descriptions": [
    { id: "PR-133", title: "feat(agents): pr-review scaffolding",          date: "2026-04-08", status: "open",   slug: "pr-review-agents" },
    { id: "PR-118", title: "feat(patcher): yaml-aware writes",             date: "2026-03-20", status: "merged", slug: "frontmatter-patcher" },
    { id: "PR-091", title: "feat(config): tier resolver",                  date: "2026-02-04", status: "merged", slug: "config-resolve-template" },
    { id: "PR-072", title: "feat(launch): release binary gate",            date: "2026-02-24", status: "merged", slug: "binary-acquisition" },
    { id: "PR-128", title: "refactor(tests): move tests out of src/",      date: "2026-04-02", status: "merged", slug: "test-restructure" },
    { id: "PR-141", title: "feat(visualiser): library detail screens",     date: "2026-04-22", status: "open",   slug: "meta-visualisation" },
  ],
  "work-reviews": [
    { id: "WR-0030-r1", title: "Centralise PATH defaults · round 1",       date: "2026-05-08", verdict: "request-changes", slug: "centralise-path-defaults" },
    { id: "WR-0030-r2", title: "Centralise PATH defaults · round 2",       date: "2026-05-08", verdict: "request-changes", slug: "centralise-path-defaults" },
    { id: "WR-0030-r3", title: "Centralise PATH defaults · round 3",       date: "2026-05-08", verdict: "request-changes", slug: "centralise-path-defaults" },
    { id: "WR-0030-r4", title: "Centralise PATH defaults · round 4",       date: "2026-05-08", verdict: "approve",         slug: "centralise-path-defaults" },
    { id: "WR-0007-r1", title: "Reviewer-agent subagent access · round 1", date: "2026-04-12", verdict: "approve-with-changes", slug: "pr-review-agents" },
    { id: "WR-0011-r1", title: "Meta visualiser scope · round 1",          date: "2026-04-15", verdict: "approve-with-changes", slug: "meta-visualisation" },
    { id: "WR-0045-r1", title: "Work management integration · round 1",    date: "2026-05-02", verdict: "request-changes",    slug: "work-management" },
    { id: "WR-0045-r2", title: "Work management integration · round 2",    date: "2026-05-05", verdict: "approve",            slug: "work-management" },
    { id: "WR-0014-r1", title: "SSE reconnect work item · round 1",         date: "2026-04-19", verdict: "approve",            slug: "sse-reconnect-backoff" },
    { id: "WR-0013-r1", title: "Kanban status writes · round 1",            date: "2026-04-17", verdict: "approve",            slug: "kanban-status-writes" },
    { id: "WR-0009-r1", title: "ETag content hash · round 1",               date: "2026-03-22", verdict: "approve",            slug: "etag-content-hash" },
  ],
  "design-inventories": [
    { id: "DI-2026-05-06-current-app",             title: "Inventory: current-app (running React SPA)",     date: "2026-05-06", status: "draft",     slug: "design-system-rollup" },
    { id: "DI-2026-05-06-claude-design-prototype", title: "Inventory: claude-design-prototype",             date: "2026-05-06", status: "draft",     slug: "design-system-rollup" },
    { id: "DI-2026-04-12-kanban-board-states",     title: "Inventory: kanban board interaction states",     date: "2026-04-12", status: "accepted",  slug: "kanban-status-writes" },
    { id: "DI-2026-03-28-templates-tier-display",  title: "Inventory: templates tier display",              date: "2026-03-28", status: "accepted",  slug: "config-resolve-template" },
  ],
  "design-gaps": [
    { id: "DG-2026-05-06-current-vs-prototype",   title: "current-app → claude-design-prototype",            date: "2026-05-06", status: "draft",    slug: "design-system-rollup" },
    { id: "DG-2026-04-15-kanban-vs-target",       title: "kanban board: current → target",                  date: "2026-04-15", status: "accepted", slug: "kanban-status-writes" },
    { id: "DG-2026-03-29-templates-vs-target",    title: "templates view: current → target",                date: "2026-03-29", status: "accepted", slug: "config-resolve-template" },
  ],
  notes: [
    // Intentionally empty — demonstrates the zero-doc empty state. Restore the
    // curated rows from git history to repopulate this type.
  ],
  templates: [
    { name: "adr",            tiers: ["—", "user", "default"], active: "user" },
    { name: "plan",           tiers: ["config", "user", "default"], active: "config" },
    { name: "research",       tiers: ["—", "—", "default"], active: "default" },
    { name: "validation",     tiers: ["—", "user", "default"], active: "user" },
    { name: "pr-description", tiers: ["—", "—", "default"], active: "default" },
  ],
};

// ─── Synthetic per-type content ──────────────────────────────────────────
// Generates a realistic, type-specific detail page for any LIBRARY_INDEX row
// that doesn't have a curated DOC_CONTENT entry. Each branch produces the
// frontmatter shape + body skeleton that type would actually carry on disk.
function synthDocContent(type, row) {
  const slug = row.slug || "untitled";
  const date = row.date || "2026-05-01";
  const id = row.id || "";
  const title = row.title || id;
  const author = "Toby Clemson";
  const cluster = (window.CLUSTERS.find(c => c.slug === slug) || {}).title || slug;

  const common = { title, date, author, slug };

  switch (type) {
    case "work": return {
      type, slug,
      frontmatter: { ...common, type: "work-item", id, status: row.status || "todo" },
      body: `# ${title}

## Summary

${title}. Owned by ${author}; tracked in the \`${slug}\` lifecycle cluster.

## Context

Pulled from the active backlog. The ${cluster} cluster is the parent work unit; this item is the entry point for research, planning and downstream review artifacts.

## Requirements

- The change must be localized to the \`${slug}\` cluster and leave unrelated modules untouched.
- All four configuration files must be updated in lockstep — no partial migration.
- A single \`source\` directive must replace the existing inline declarations.

## Acceptance criteria

1. **Given** the work item is implemented, **when** \`mise run check\` runs, **then** the new structure is in place and no regressions are detected.
2. **Given** a grep against the affected paths, **when** the migration is complete, **then** only the expected definition sites remain.
3. **Given** the existing test suite, **when** \`mise run test\` executes, **then** the \`config-dump\`, \`config-init\`, and \`path-resolution\` groups all pass.

## Technical notes

- Definition sites: 4 files across the workspaces.
- Consumer sites: 11 scripts that \`source\` the centralised file.
- Bash sourcing semantics assumed (POSIX \`set -e\` safe).

## Dependencies

- Blocked by: none.
- Triggers follow-up in: downstream consumer cluster.

## Links

- research [[research/${date}-${slug}.md]]
- plan [[plans/${date}-${slug}.md]]
`,
    };

    case "work-reviews": return {
      type, slug,
      frontmatter: { ...common, type: "work-item-review", verdict: row.verdict, target: `meta/work/${id.replace(/-r\d+$/,"")}.md`, work_item: id.match(/\d{4}/)?.[0], review_number: parseInt(id.match(/r(\d+)$/)?.[1] || "1") },
      body: `# Work Item Review: ${cluster}

**Verdict:** ${(row.verdict || "comment").toUpperCase()}

The work item is generally well-structured and uses precise technical language. This review pass surfaces ${row.verdict === "approve" ? "no major blocking findings" : "a small number of issues that need attention before implementation"}.

## Cross-cutting themes

- **Scope coherence** — all requirements serve a single unified purpose; no bundling of unrelated concerns.
- **Testability** — acceptance criteria are concrete and runnable in most cases; one criterion still relies on a tautological baseline.
- **Dependency capture** — primary downstream consumer is named with a clear rationale; bidirectional coupling is documented.

## Findings

### Major

- 🟡 **Clarity + Completeness**: a contradiction exists between the Requirements section and the Technical Notes around scope.
- 🟡 **Testability**: the regression criterion is unanchored — "no regressions" is always arguable while the test suite is green.

### Minor

- 🔵 **Scope**: the title overstates what will actually be migrated.
- 🔵 **Dependency**: one downstream coupling is mentioned in prose but not captured in the Dependencies section.

## Strengths

- ✅ Grep-based AC2 provides a precise, copy-pasteable verification command.
- ✅ Technical Notes are unusually thorough — exact file paths and line ranges.
- ✅ Bidirectional coupling with the downstream consumer is documented in both work items.

## Recommended changes

1. Resolve the Requirements ↔ Technical Notes contradiction.
2. Reframe the regression criterion with named test groups.
3. Capture the downstream coupling explicitly in Dependencies.

## Links

- target [[${id.replace(/-r\d+$/,"")}]]
`,
    };

    case "research": return {
      type, slug,
      frontmatter: { ...common, type: "research", status: "complete" },
      body: `# ${title}

## Purpose

Survey the prior art and open patterns relevant to \`${slug}\`, ahead of writing a plan. The aim is to gather enough material for an informed decision, not to commit to an approach yet.

## Sources reviewed

- Existing internal artefacts in \`${slug}\` (5 documents).
- Two external precedents that take a similar shape.
- Adjacent ADRs that touch the same surface area.

## Findings

1. **The dominant pattern is composable** — most systems in this space decompose the problem into a small set of orthogonal layers, with a thin orchestrator coordinating them.
2. **Deterministic checks pair well with LLM-driven analysis** — keeping the cheap, fast checks separate from the expensive, opinionated ones produces clearer verdicts and a cheaper feedback loop.
3. **Verdict language is load-bearing** — the way a system labels its outputs (approve / approve-with-changes / request-changes) shapes how authors respond to them more than the underlying scoring does.

## Sketches considered

A layered orchestrator in Rust — the shape we keep arriving at:

\`\`\`rust
async fn review(diff: &Diff, layers: &[Arc<dyn ReviewLayer>]) -> Verdict {
    let findings: Vec<Finding> = futures::stream::iter(layers)
        .map(|l| l.review(diff))
        .buffer_unordered(layers.len())
        .try_concat()
        .await
        .unwrap_or_default();

    Verdict::from_findings(findings)
}
\`\`\`

The same shape, expressed as a TypeScript reference for comparison:

\`\`\`typescript
type ReviewLayer = {
  name: string;
  review(diff: Diff): Promise<Finding[]>;
};

async function review(diff: Diff, layers: ReviewLayer[]): Promise<Verdict> {
  const results = await Promise.all(layers.map((l) => l.review(diff)));
  return Verdict.fromFindings(results.flat());
}
\`\`\`

## Open questions

- How should the orchestrator surface conflicting verdicts from independent layers?
- What is the right cache key for the agent layer's per-diff output?
- Should convention rules ever override an agent-level verdict, or only annotate it?

## Recommended next step

Write a plan that adopts the layered shape and defers the cache + override questions to a follow-up.

## Links

- work item [[work/${slug}.md]]
- planned plan [[plans/${date}-${slug}.md]]
`,
    };

    case "plans": return {
      type, slug,
      frontmatter: { ...common, type: "plan", status: row.status || "draft", last_updated: date },
      body: `# ${title}

## Purpose

Lay out the design for \`${slug}\` so an implementer can pick it up without further investigation. Scope is bounded to the single concern named in the title.

## Scope (v1)

1. **Surface** — the public entry points the change exposes.
2. **Mechanism** — the underlying flow that implements those entry points.
3. **Failure modes** — the error conditions an implementer must handle.

## Architecture

\`\`\`
caller ──► orchestrator ──► [ agent · convention · validator ]
                │
                └──► artefact store (meta/${slug})
\`\`\`

## Public surface

\`\`\`rust
/// Entry point exposed to callers. Returns the merged verdict from every
/// active layer, in the order the orchestrator ran them.
pub async fn run(
    diff: &Diff,
    opts: RunOptions,
) -> Result<Verdict, OrchestratorError> {
    let layers = registry::active_layers(&opts)?;
    let mut findings = Vec::with_capacity(layers.len() * 4);
    for layer in &layers {
        let mut chunk = layer.review(diff).await?;
        findings.append(&mut chunk);
    }
    Ok(Verdict::from_findings(findings))
}
\`\`\`

## Configuration

\`\`\`yaml
# meta/config/${slug}.yml — layered defaults; per-workspace overrides win.
orchestrator:
  layers:
    - name: agent
      model: claude-haiku-4-5
      cache: true
      timeout_ms: 12000
    - name: convention
      rules: workspace
    - name: blast-radius
      threshold: medium
  on_error: skip          # one layer failing must not poison the verdict
  verdict_policy: strictest
\`\`\`

## Open questions

- Cache invalidation semantics on partial diff overlap.
- Whether the orchestrator should expose a streaming API or only a final verdict.

## Links

- work item [[work/${slug}.md]]
- research [[research/${date}-${slug}.md]]
`,
    };

    case "plan-reviews": return {
      type, slug,
      frontmatter: { ...common, type: "plan-review", verdict: row.verdict, target: `plans/${date}-${slug}.md`, round: parseInt(id.match(/\d+/)?.[0] || "1") },
      body: `# Plan review · ${id.replace("-"," ")} — ${cluster}

**Verdict:** \`${row.verdict}\`

Scope is well-bounded and the architecture sketch is concrete. The review surfaces ${row.verdict === "approve" ? "no blocking issues" : "a handful of structural notes that should be addressed before the plan is accepted"}.

## Findings

### 1. Reconnect semantics (medium)

The plan assumes transparent reconnect but does not specify the behaviour for in-flight writes when the channel drops. Recommend:

- Explicit \`invalidate-all\` on reconnect.
- Describe the backoff curve (exponential up to 30s).
- Surface disconnect state in the UI.

### 2. Manifest drift (low)

The committed manifest is the gate, but the plan does not describe what happens if the manifest drifts from the underlying assets. Recommend a verbose error on mismatch rather than a silent retry.

### 3. Tier-diff affordance (low)

Tiers are shown in priority order but their differences are not highlighted. Worth confirming whether a diff view is needed before this lands.

## Links

- target plan [[plans/${date}-${slug}.md]]
- work item [[work/${slug}.md]]
`,
    };

    case "validations": return {
      type, slug,
      frontmatter: { ...common, type: "validation", verdict: row.verdict || "pass" },
      body: `# Validation: ${title.replace(/^Validation:\s*/,"")}

**Verdict:** \`${row.verdict || "pass"}\`

This validation exercises the behaviour described in the parent plan against the implementation under test.

## Test plan

1. Construct the canonical input documented in the plan.
2. Run the implementation end-to-end.
3. Assert against the documented post-conditions.
4. Repeat with three pathological inputs (empty, oversized, malformed).

## Reproduction

\`\`\`bash
# Build once, then exercise the four cases from the plan in sequence.
cargo build --release --features validation-${slug}

for case in canonical empty oversized malformed; do
    ./target/release/${slug}-validate \\
        --case "$case" \\
        --baseline ./meta/validations/baselines.json \\
        --verbose
done | tee meta/validations/${date}-${slug}.log
\`\`\`

## Results

| Case | Expected | Observed | Status |
|------|----------|----------|--------|
| canonical | pass | pass | ✅ |
| empty input | clean error | clean error | ✅ |
| oversized | clean error | clean error | ✅ |
| malformed | clean error | clean error | ✅ |

## Programmatic assertion

\`\`\`python
# Snapshot test — pinned baseline lives next to this file.
from pathlib import Path
import json, pytest

from accelerator.validations import run_case, BASELINES

@pytest.mark.parametrize("case", ["canonical", "empty", "oversized", "malformed"])
def test_${slug.replace(/-/g,"_")}(case: str) -> None:
    actual = run_case(case)
    expected = BASELINES[case]
    assert actual.verdict == expected["verdict"], (
        f"case={case!r} drifted from baseline; rerun with --update"
    )
\`\`\`

## Findings

- All cases pass; behaviour matches the plan.
- One minor observation: error messages on the oversized case could be tightened — captured as a follow-up note rather than a blocking issue.

## Links

- plan [[plans/${date}-${slug}.md]]
- work item [[work/${slug}.md]]
`,
    };

    case "pr-descriptions": return {
      type, slug,
      frontmatter: { ...common, type: "pr-description", pr_number: parseInt(id.replace(/\D/g,"")) || null, status: row.status || "open" },
      body: `# ${title}

## Summary

${title}. This PR implements the design captured in the parent plan and clears the relevant acceptance criteria from the work item.

## Problem

The pre-change state had inline declarations scattered across four files, making it hard to reason about the canonical source and easy for drift to accumulate. The structure was also resistant to additional consumers being added later.

## Solution

- Centralise the affected arrays into a single defaults file.
- Have each consumer \`source\` the centralised file rather than redeclare.
- Update test configuration to match the new layout.

## Key diff

\`\`\`diff
--- a/scripts/lib/path_defaults.sh
+++ b/scripts/lib/path_defaults.sh
@@ -1,18 +1,8 @@
-#!/usr/bin/env bash
-# Inline copy — kept in sync by hand. DO NOT EDIT.
-PATH_DEFAULTS=(
-    "/usr/local/bin"
-    "/opt/accelerator/bin"
-    "$HOME/.cargo/bin"
-    "$HOME/.local/bin"
-)
-export PATH_DEFAULTS
+#!/usr/bin/env bash
+# All defaults now live in meta/config/defaults.sh.
+# shellcheck source=../../meta/config/defaults.sh
+source "${"${CLAUDE_PLUGIN_ROOT}"}/meta/config/defaults.sh"
\`\`\`

## Changes

- **Reorganization** — moved the four definition sites into one canonical file.
- **Configuration** — updated pytest, mypy, and the makefiles to point at the new path.
- **Documentation** — added a comment at the top of the new file linking back to the work item.

## Breaking changes

None — internal refactor only.

## How to verify

\`\`\`bash
# Each step must produce a clean exit code.
mise run check
mise run test --filter ${slug}
rg --no-heading 'declare -ra (PATH|TEMPLATE)_DEFAULTS' --type sh
\`\`\`

## Links

- work item [[work/${slug}.md]]
- plan [[plans/${date}-${slug}.md]]
`,
    };

    case "pr-reviews": return {
      type, slug,
      frontmatter: { ...common, type: "pr-review", verdict: row.verdict, target: `pr-descriptions/${slug}.md`, round: parseInt(id.match(/\d+/)?.[0] || "1") },
      body: `# PR review · ${id.replace("-"," ")} — ${cluster}

**Verdict:** \`${row.verdict}\`

This review pass covers the diff against the parent plan and surfaces the items below.

## Inline findings

### 1. Boundary handling

The new \`source\` directive uses a literal path rather than \`${"${CLAUDE_PLUGIN_ROOT}"}\`. This works in the test environment but will break if the file is invoked from a different installation root.

**Suggested change:**

\`\`\`diff
-source "/opt/accelerator/meta/config/defaults.sh"
+source "${"${CLAUDE_PLUGIN_ROOT}"}/meta/config/defaults.sh"
\`\`\`

### 2. Comment density

Some of the new functions have no docstring. Recommend a one-line summary at minimum to preserve the readability the existing file has.

\`\`\`rust
/// Resolve the configured PATH entries from the layered defaults file.
///
/// Returns the merged list in priority order (config > user > plugin),
/// with duplicates dropped and missing entries silently skipped.
pub fn resolve_path_defaults(cfg: &Config) -> Vec<PathBuf> {
    cfg.tiers()
        .flat_map(|tier| tier.path_defaults().iter().cloned())
        .unique()
        .collect()
}
\`\`\`

### 3. Test coverage

The migration is covered by the existing test suite, but no test asserts the new file's path-resolution semantics directly. A focused test would protect the next refactor from a regression.

## Recommended changes

1. Switch the literal path to \`${"${CLAUDE_PLUGIN_ROOT}"}\`.
2. Add a one-line summary to each new function.
3. Add a focused path-resolution test.

## Links

- target PR [[pr-descriptions/${slug}.md]]
- work item [[work/${slug}.md]]
`,
    };

    case "decisions": return {
      type, slug,
      frontmatter: { ...common, type: "decision", status: row.status || "accepted", supersedes: null },
      body: `# ${title}

## Context

The \`${slug}\` surface forces a choice between two viable approaches, each with downstream implications that outlast the immediate implementation. Capturing the decision in an ADR keeps the rationale legible for future maintainers and provides a single citation for downstream artefacts.

## Decision

Adopt the approach described below.

1. **Primary mechanism** — the load-bearing piece of the design.
2. **Supporting structure** — the scaffolding around it.
3. **Defaults** — the behaviour when the surface is invoked with no overrides.

The choice is bounded to this surface; adjacent surfaces continue to use their existing patterns until an explicit follow-up decision is recorded.

## Reference shape

\`\`\`rust
/// Stable, documented surface for the chosen mechanism. Adjacent layers
/// depend on this signature, not on the implementation behind it.
pub trait ${slug.split("-").map(s => s[0]?.toUpperCase() + s.slice(1)).join("")}: Send + Sync {
    fn resolve(&self, ctx: &Context) -> Result<Resolution, ResolveError>;
    fn priority(&self) -> Priority { Priority::Default }
}

impl Default for Resolution {
    fn default() -> Self {
        Self { tier: Tier::Plugin, source: Source::Builtin, etag: None }
    }
}
\`\`\`

## Consequences

- Each layer is independently testable and replaceable.
- The orchestrator becomes the sole coupling point.
- Future migrations away from this choice require an explicit superseding ADR.

## Alternatives considered

- A monolithic mechanism — rejected for poor testability.
- A plugin-only approach — rejected for excessive ceremony at low call volume.

## Links

- work item [[work/${slug}.md]]
- plan [[plans/${date}-${slug}.md]]
`,
    };

    case "notes": return {
      type, slug,
      frontmatter: { ...common, type: "note" },
      body: `# ${title}

A short note captured outside the regular research → plan → review flow. Useful for hallway conversations, follow-ups that aren't worth a full plan, and open questions that should be remembered without blocking the active work.

## Open questions

- Should the reconnect handshake invalidate just the affected query, or all queries in the active scope?
- Is the 30-second cap on the backoff curve aggressive enough for the long-tail of dropped connections we see in practice?
- Where is the right place to surface a disconnected state — sidebar footer, topbar pill, or both?

## Followups

- Capture the SSE reconnect semantics in a focused plan rather than leaving them in this note.
- Confirm with the reviewer agents whether the toast lifecycle should be tied to the active route or the entire app.

## Links

- related cluster [[lifecycle/${slug}]]
`,
    };

    case "design-inventories": return {
      type, slug,
      frontmatter: { ...common, type: "design-inventory", source: id.replace(/^DI-\d{4}-\d{2}-\d{2}-/,""), status: row.status || "draft", sequence: 1 },
      body: `# Design inventory: ${id.replace(/^DI-\d{4}-\d{2}-\d{2}-/,"")}

## Overview

Crawl of the named surface produced an inventory of every observable token, component and screen. The aim is to give a downstream gap analysis something concrete to compare against, not to make any judgements about the design itself.

## Screenshots

[[screenshots:6]]

## Design system

### Color tokens

| Token | Value | Source |
|-------|-------|--------|
| \`--ac-fg-strong\` | \`#0f172a\` | \`global.css\` |
| \`--ac-fg-muted\`  | \`#4b5563\` | \`global.css\` |
| \`--ac-stroke\`    | \`#e5e7eb\` | \`global.css\` |
| \`--ac-bg-card\`   | \`#ffffff\` | \`global.css\` |
| \`--ac-bg-active\` | \`#dbeafe\` | \`Sidebar.module.css\` |
| \`--ac-accent\`    | \`#2563eb\` | \`global.css\` |

### Typography

- Body / chrome: \`system-ui, sans-serif\`
- Monospace: \`Fira Code, monospace\`
- Body line-height: \`1.6\`

### Layout primitives

- Sidebar width: \`220px\`
- Main column max-width: \`1100px\`
- Card grid: \`repeat(auto-fill, minmax(320px, 1fr))\`

## Component catalogue

- **Sidebar** — primary navigation, partitions doc types into groups.
- **LibraryTable** — sortable per-doc-type listing.
- **DocPage** — single-document view with frontmatter chips + body + aside.
- **Topbar** — server origin pill + SSE indicator + theme toggle.

## Known gaps

- Loading-state screenshots not captured.
- Drag-and-drop interaction states not exercised.
`,
    };

    case "design-gaps": return {
      type, slug,
      frontmatter: { ...common, type: "design-gap", status: row.status || "draft" },
      body: `# Design gap: ${title}

## Overview

Compare the current state of the surface against the target state captured in the paired inventory. The aim is to enumerate the deltas in a form that a downstream plan can pick up directly.

## Token drift

The current surface exposes a small number of named tokens and relies on inline hex values for the rest. The target system defines a layered token set with semantic surface tokens, full light/dark overrides, and a formal typography scale.

## Component drift

- **Navigation grouping** — current is storage-shape; target groups by lifecycle phase.
- **Topbar** — net-new in the target; current has none.
- **Chip vocabulary** — current is limited to frontmatter chips; target defines a generic Chip with five variants.
- **Glyph** — net-new; per-doc-type coloured icon at multiple sizes.

## Screen drift

- **Library overview hub** — current redirects to a single type; target groups types by phase.
- **Cluster detail** — target adds a hexchain strip above the existing timeline.
- **Templates view** — target adds inline tier-presence row and a sha256 etag header.

## Net-new features

- Light / dark theming via \`data-theme\` attribute.
- Sidebar search box with \`/\` keyboard shortcut.
- Activity feed in the sidebar, fed by SSE.

## Suggested sequencing

Token migration first (load-bearing), then theming and font-mode, then a per-component refit, then net-new features, then screen-level redesigns last.

## Links

- current inventory [[${slug}-current]]
- target inventory [[${slug}-target]]
`,
    };

    default: return {
      type, slug,
      frontmatter: { ...common, type, status: row.status || "draft" },
      body: `# ${title}\n\n_No detail template registered for type \`${type}\` yet._\n`,
    };
  }
}

Object.assign(window, {
  DOC_TYPES, LIBRARY_GROUPS, VIEWS, META, STAGES, CLUSTERS, WORK_ITEMS,
  DOC_CONTENT, ACTIVITY, LIBRARY_INDEX, synthDocContent,
});


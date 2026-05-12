---
date: "2026-05-03T14:44:01+01:00"
researcher: Toby Clemson
git_commit: 4c47a7a65115d4991f1b8886036d864593973a61
branch: visualisation-system (jj workspace, change pwzplnxyxyok)
repository: accelerator
topic: "Updating the visualiser to support work-item terminology and the new work-management approach"
tags: [research, visualiser, work-item, ticket, migration, terminology, kanban, wiki-links, configurable-id-pattern]
status: complete
last_updated: "2026-05-03"
last_updated_by: Toby Clemson
---

# Research: Updating the visualiser to support work-item terminology

**Date**: 2026-05-03 14:44 BST
**Researcher**: Toby Clemson
**Git Commit**: `4c47a7a65115d4991f1b8886036d864593973a61`
**Branch**: `visualisation-system` jj workspace (change `pwzplnxyxyok`)
**Repository**: `accelerator` (workspaces/visualisation-system)

## Research Question

The visualisation system was developed against the old "ticket" terminology
(documented in `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`).
While Phases 1–11 shipped, the surrounding plugin underwent a sweeping rename:
"ticket" → "work-item" and the category `tickets/` → `work/`
(`meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md`,
`meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`),
plus a new configurable filename ID pattern
(`meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`). The
visualiser's ticket-related features are now broken. What changes are required
to bring it in line with the new terminology and approach, and how should that
work be phased?

## Summary

The visualiser is broken at one root cause: **the launcher emits a `tickets`
config key that the plugin no longer recognises**. The plugin's
`scripts/config-read-path.sh:7-19` now exposes `work` and `review_work` keys
(replacing the migrated-away `tickets` and `review_tickets`), and migration
`0001-rename-tickets-to-work` has been applied in this workspace
(`meta/.migrations-applied:1`). The visualiser launcher
(`skills/visualisation/visualise/scripts/write-visualiser-config.sh:39,89,105`)
still calls `config-read-path.sh tickets meta/tickets`, which silently
resolves to the hardcoded fallback `meta/tickets/` — a directory that doesn't
exist in this workspace. The Rust server's writable-roots wiring at
`server/src/server.rs:60` then keys on `cfg.doc_paths.get("tickets")`, gets
nothing, and **every kanban PATCH returns `OnlyTicketsAreWritable` while the
read-only library/lifecycle/kanban views show zero work-items**.

Surface area of the rename: ~130 touch-points across the launcher (1 file,
1 SKILL.md), the Rust server (12 files, ~50 sites including a `DocTypeKey`
enum variant, a `TicketStatus` enum, an `IndexEntry.ticket` field, a
`Completeness.has_ticket` field, an `OnlyTicketsAreWritable` error variant,
slug derivation, cluster ordering, a writable-roots whitelist, integration
tests, and a fixture directory), and the React frontend (~20 files, ~40
sites including the `DocTypeKey` union literal `'tickets'`,
`LIFECYCLE_PIPELINE_STEPS[0]`, `TicketCard.tsx` with a hardcoded
`params={{ type: 'tickets' }}`, the wiki-link regex `/\[\[(ADR|TICKET)-...]]/`,
every drag aria-live announcement string, kanban toasts and empty states,
test fixtures hardcoded to `meta/tickets/0001-...md` paths, and e2e specs
pinned to `tests/fixtures/meta/tickets/`).

Beyond the rename, three substantive shifts must be absorbed:

1. **Configurable ID patterns** — `skills/work/scripts/work-item-pattern.sh`
   exposes a compiler that turns a pattern like `{project}-{number:04d}` into
   a `(scan_regex, format_string)` pair. The visualiser's `slug.rs:35-42`
   hard-codes the `<digits>-<rest>` shape; project-prefixed filenames
   (`PROJ-0042-foo.md`) won't slug-derive, won't cluster, won't resolve
   via wiki-links.
2. **Status enum widening** — `templates/work-item.md:7` defines seven
   statuses (`draft|ready|in-progress|review|done|blocked|abandoned`); the
   visualiser hard-codes `todo|in-progress|done` and shoves anything else
   into "Other".
3. **Frontmatter cross-references move from `ticket:` to a triple
   (`work-item:`, `parent:`, `related:`)** — `templates/work-item.md:9`
   defines `parent:` for the parent epic/story; `templates/plan.md:4` now
   uses `work-item:` (renamed from `ticket:`); the work-item template's
   "Dependencies" body section enumerates related items but isn't a
   structured frontmatter field today. The Rust indexer's
   `frontmatter::ticket_of` (`server/src/frontmatter.rs:297-306`) only reads
   the legacy `ticket:` key and would degrade to `None` for every work-item
   under the new schema.

The visualiser's Phase 12 (packaging/release) is complete (the plan file
should be marked accordingly); this work-item migration is the next thing to
ship, sequenced as a coordinated four-phase change ending in a validated
release.

The 30 files in this workspace's `meta/work/` are still legacy schema
(`type: adr-creation-task`, status values `todo|done|proposed`, no
`work_item_id` field). The migration script renamed paths and one config
key but did not rewrite per-file frontmatter — and isn't expected to. The
visualiser must therefore tolerate **both** the legacy-schema work-items
that exist on disk in pre-migration repos **and** the new-schema work-items
that future skills will create.

## Detailed Findings

### 1. The single break-point: launcher → server config wiring

The launcher script
`skills/visualisation/visualise/scripts/write-visualiser-config.sh` resolves
plugin paths and writes a `config.json` consumed by the Rust binary. Three
lines refer to the old key:

- Line 39: `TICKETS="$(abs_path tickets meta/tickets)"`
- Line 89: `--arg tickets "$TICKETS"`
- Line 105: `doc_paths: { ..., tickets: $tickets, ... }`

`config-read-path.sh` no longer accepts `tickets`; it accepts `work` (default
`meta/work`). The script's behaviour for an unknown key is to fall through
to the supplied default — which for the launcher is `meta/tickets`. So the
launcher writes a `config.json` referencing a non-existent directory.

The Rust server then:

- `server/src/server.rs:60` reads `cfg.doc_paths.get("tickets")` to populate
  the **writable-roots whitelist** for `LocalFileDriver`
  (`server/src/server.rs:67`).
- `server/src/file_driver.rs:370` checks `canonical.starts_with(r)` against
  that whitelist on every PATCH; a missing entry → empty whitelist → all
  PATCHes return `PathNotWritable`, mapped to `ApiError::OnlyTicketsAreWritable`
  at `server/src/api/mod.rs:155`.

Every other downstream symptom (zero kanban cards, zero ticket entries in
the lifecycle pipeline, no wiki-link resolution for `[[TICKET-NNNN]]`)
follows from the same launcher-side miss.

`SKILL.md:22` carries the same stale lookup
(`!`config-read-path.sh tickets meta/tickets``) — it's display-only since
the launcher does its own resolution, but the placeholder text is wrong
to a user reading the slash-command output.

### 2. Server-side ticket terminology — what each module exposes

#### `server/src/docs.rs` — the `DocTypeKey` enum
- Variant `DocTypeKey::Tickets` at `:8`; listed in `all()` at `:23`.
- `#[serde(rename_all = "kebab-case")]` at `:5` → wire form `"tickets"`.
- `config_path_key()` at `:38` returns `Some("tickets")` (read by
  `server.rs:60` and the file-driver).
- `label()` at `:53` returns `"Tickets"` (sidebar label shipped to the
  frontend via `GET /api/types`).
- `in_kanban()` at `:70`: tickets are the only kanban type today.
- Round-trip test asserting wire form at `:115`.

#### `server/src/slug.rs` — slug derivation for clustering
- `DocTypeKey::Tickets => strip_prefix_ticket_number(stem)` at `:11`.
- `strip_prefix_ticket_number` at `:35-42`: hard-coded `<digits>-<rest>`.
- This works for the default `{number:04d}` pattern but **fails for project-
  prefixed IDs** (`PROJ-0042-foo.md` returns `None` → the work-item is
  excluded from lifecycle clusters and wiki-link resolution silently).

#### `server/src/clusters.rs` — lifecycle pipeline ordering
- `Completeness.has_ticket: bool` at `:11`; serde camelCase makes this
  `hasTicket` on the wire.
- `derive_completeness` initialises at `:98`; sets at `:110` on
  `DocTypeKey::Tickets`.
- Canonical rank `DocTypeKey::Tickets => 0` at `:69` (tickets come first
  in the lifecycle pipeline).

#### `server/src/patcher.rs` — kanban write-path
- `TicketStatus { Todo, InProgress, Done }` enum at `:14-20` with kebab
  serde. **Hard-coded three values** — does not match the seven-state
  template enum, even though the actual `patch_status` function at `:49`
  is type-agnostic and only edits the YAML `status:` line.
- `FrontmatterPatch::Status(TicketStatus)` at `:5`.

#### `server/src/frontmatter.rs` — frontmatter accessor
- `ticket_of(parsed: &FrontmatterState) -> Option<String>` at `:297-306`,
  reads the literal `ticket:` key. **Does not read `work-item:`,
  `parent:`, or `related:`** — every modern work-item degrades to `None`.

#### `server/src/indexer.rs` — secondary indexes
- `IndexEntry.ticket: Option<String>` field at `:25`.
- `Indexer.ticket_by_number: Arc<RwLock<HashMap<u32, PathBuf>>>` at `:55`,
  populated/maintained at `:109,132-134,152,156,477-493,531-538`. The
  `u32` key is wrong for project-prefixed IDs, which are strings.
- `ticket_number_from_entry` at `:611-617` gates on
  `DocTypeKey::Tickets`; `parse_ticket_number` at `:632-635` splits at the
  first `-` and parses the left as `u32`.
- `build_entry` populates `ticket = frontmatter::ticket_of(...)` at
  `:568,595`.

#### `server/src/server.rs` — boot wiring
- `tickets_root` at `:58-63`: looks up `cfg.doc_paths.get("tickets")` and
  hands a single-element `Vec<PathBuf>` to `LocalFileDriver::new` at `:67`.
- This is **the active broken wire**.

#### `server/src/api/` — REST surface
- `ApiError::OnlyTicketsAreWritable` variant + literal string at
  `mod.rs:53,54,95,155`.
- `PatchFields { status: Option<TicketStatus> }` at `docs.rs:107-109`.
- Kanban PATCH gate at `docs.rs:154-156`:
  `if entry.r#type != DocTypeKey::Tickets { return Err(...) }`.
- The PATCH URL is type-agnostic (`/api/docs/{*path}/frontmatter`); the
  type check is enforced in code.

#### `server/src/sse_hub.rs` — SSE wire format
- Both `SsePayload::DocChanged.docType` and `DocInvalid.docType` at
  `:8-21` carry the kebab-cased enum literal — emits `"tickets"` on every
  ticket-related event.

#### `server/src/file_driver.rs` — fixtures
- Lines `:594-616`: `seeded_write_driver` test helper hard-codes the
  literal `tickets` directory and inserts a `tickets`-keyed entry into
  the `doc_paths` map.
- Tests at `:724,762,791,825,866,879,918,952,997` use the same fixture
  shape.

#### `server/tests/fixtures/meta/tickets/` — integration fixtures
- `0001-first-ticket.md` and `0005-sse-test-ticket.md` are referenced
  by both the Rust integration tests and the frontend e2e tests
  (`frontend/e2e/kanban.spec.ts:6-11,46,54,61,89,96,108`). The frontmatter
  is legacy (`status: todo`, `ticket: 1`).

### 3. Frontend ticket terminology — what each surface exposes

#### Wire format / API consumers

- `src/api/types.ts:4-7` — `DocTypeKey` union includes `'tickets'` and
  has no `'work-item-reviews'` slot.
- `src/api/types.ts:13-17` — `DOC_TYPE_KEYS` runtime array literal.
- `src/api/types.ts:48` — `IndexEntry.ticket: string | null`.
- `src/api/types.ts:98` — `Completeness.hasTicket: boolean`.
- `src/api/types.ts:128,139` — `LIFECYCLE_PIPELINE_STEPS[0]`:
  `{ key: 'hasTicket', docType: 'tickets', label: 'Ticket',
  placeholder: 'no ticket yet' }`.
- `src/api/types.ts:158-175` — `KanbanColumnKey = 'todo' | 'in-progress'
  | 'done'`, `STATUS_COLUMNS` hardcoded with `Todo / In progress / Done`
  labels and an `Other` swimlane.

#### API helpers

- `src/api/fetch.ts:29-54` — `patchTicketFrontmatter(relPath, { status },
  etag)` (function name only; the URL itself is generic).
- `src/api/ticket.ts:9,30` — `parseTicketNumber`, `groupTicketsByStatus`.
- `src/api/use-move-ticket.ts:8,13,15,24-26,45` — `MoveTicketVars`,
  `MoveTicketContext`, `useMoveTicket`; cache keys `queryKeys.docs('tickets')`.
- `src/api/wiki-links.ts:6,20,22,45,79-86,110` —
  `WIKI_LINK_PATTERN = /\[\[(ADR|TICKET)-(\d{1,6})\]\]/g`,
  `WikiLinkKind = 'ADR' | 'TICKET'`, `ticketByNumber: Map<...>`.
- `src/api/use-wiki-link-resolver.ts:19,36-39,41,45` —
  `useQuery({ queryKey: queryKeys.docs('tickets'), queryFn: () =>
  fetchDocs('tickets') })`.
- `src/api/use-doc-events.ts:30,74` —
  `if (event.docType === 'tickets')` cache invalidation.
- `src/components/MarkdownRenderer/wiki-link-plugin.ts:17,77` —
  `Resolver = (prefix: 'ADR' | 'TICKET', n: number) => ...`.

#### Kanban UI — strings, components, route hardcoding

- `src/routes/kanban/KanbanBoard.tsx:11,12,15,21-22,28,33,37,54-55,152` —
  imports from `../../api/ticket`, hardcoded toasts and Other-column
  copy that say "tickets"/"ticket".
- `src/routes/kanban/TicketCard.tsx` — file/component name; line 26 calls
  `parseTicketNumber`, line 39 hardcodes the link target
  `params={{ type: 'tickets', fileSlug }}` so every kanban card links
  to `/library/tickets/<slug>` regardless of the doc type's actual
  config key.
- `src/routes/kanban/KanbanColumn.tsx:3,19,44,48` —
  `ticketWord = count === 1 ? 'ticket' : 'tickets'` aria-label;
  `'No tickets'` empty-state copy; `<TicketCard>` import.
- `src/routes/kanban/announcements.ts:9,21-49` — `ticketNumberFromRelPath`
  helper; every drag aria-live message starts with `"ticket "` or
  `"Picked up ticket"` / `"Moved ticket"` / `"Drag of ticket"`.

#### Tests and e2e

- `src/api/test-fixtures.ts:15` — `IndexEntry` factory defaults
  `ticket: null`.
- `src/routes/kanban/KanbanBoard.test.tsx:57,65,74-99,159-204` — every
  fixture sets `type: 'tickets'`, `relPath: 'meta/tickets/...'`.
- `src/routes/kanban/TicketCard.test.tsx` — file name, component-import,
  every fixture path.
- `src/routes/kanban/announcements.test.ts:6-58` — fixture, regex
  expectations on `"ticket {id}: {title}"` aria-live strings.
- `src/api/wiki-links.test.ts:25` — `[[TICKET-1]]` literal regex test.
- `frontend/e2e/kanban.spec.ts:6-11,46,54,61,89,96,108` — pinned to
  `tests/fixtures/meta/tickets/` literal paths and `0001-first-ticket.md`
  / `0005-sse-test-ticket.md` filenames.

### 4. Plugin-side state of the migration

`scripts/config-read-path.sh:7-19` (authoritative recognised keys):

| Key | Default |
|---|---|
| `plans` | `meta/plans` |
| `research` | `meta/research` |
| `decisions` | `meta/decisions` |
| `prs` | `meta/prs` |
| `validations` | `meta/validations` |
| `review_plans` | `meta/reviews/plans` |
| `review_prs` | `meta/reviews/prs` |
| `review_work` | `meta/reviews/work` |
| `templates` | `meta/templates` |
| `work` | `meta/work` |
| `notes` | `meta/notes` |
| `tmp` | `meta/tmp` |

Compared to the visualiser's spec assumption (12 keys with `tickets` and
`review_tickets`), the keys are: `tickets` → `work` and `review_tickets`
→ `review_work`. The total stays at 12.

**Live `meta/work/` content.** 30 files; status distribution
(`status: done` × 20, `status: todo` × 9, `status: proposed` × 1) and
type distribution (`type: adr-creation-task` × 29, `type: task` × 1).
None use `work_item_id:` frontmatter; all are legacy schema. Filenames
are `NNNN-slug.md` (default pattern), so the visualiser's existing slug
regex still parses them — but the modern `work_item_id`, `priority`,
`parent`, and `tags` frontmatter fields are absent and the only `status`
values found in real life are `todo`, `done`, and `proposed`.

**Templates.**
- `templates/work-item.md` exists; defines `work_item_id` (string),
  `type` enum (`epic|story|task|bug|spike`), `status` enum
  (`draft|ready|in-progress|review|done|blocked|abandoned`),
  `priority` (`high|medium|low`), `parent` (work-item number),
  `tags` (array).
- `templates/ticket.md` was removed.
- `templates/plan.md:4` was migrated: previously `ticket:`, now
  `work-item: "{work-item reference, if any}"`. Single string value.

**Configurable ID pattern.** Shipped at
`skills/work/scripts/work-item-pattern.sh` (CLI:
`--validate <pattern>`, `--compile-scan <pattern> <project>` →
ERE regex with capture group 1 = number,
`--compile-format <pattern> <project>` → printf format string). Backed by
shared functions in `skills/work/scripts/work-item-common.sh`
(`wip_validate_pattern:212`, `wip_compile_scan:223`, `wip_compile_format:231`,
`wip_pattern_max_number:239`, `wip_is_legacy_id:269`,
`wip_pad_legacy_number:278`, `wip_parse_full_id:290`,
`wip_canonicalise_id:354`, `wip_extract_id_from_filename:428`,
`wip_is_work_item_file:449`).
Default pattern: `{number:04d}`. Project-prefixed example:
`{project}-{number:04d}` produces filenames like `PROJ-0042-foo.md`.

**Review subsystem.** `scripts/config-read-review.sh:15` supports modes
`pr | plan | work-item`. `BUILTIN_WORK_ITEM_LENSES = clarity completeness
dependency scope testability` at `:54-60`. Lens directory layout uses
neutral names — no rename needed for the lens directories. New config
keys `review.work_item_revise_severity` and
`review.work_item_revise_major_count` exist at `:85-86`. The visualiser
never had a corresponding `ticket-reviews` DocType; `review_work` becomes
a **net-new doctype** to surface.

**Migration framework.** `meta/.migrations-applied:1` lists
`0001-rename-tickets-to-work` as applied. The migration script
(`skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`) renames
`paths.tickets` → `paths.work` and `paths.review_tickets` → `paths.review_work`
in `.claude/accelerator{,.local}.md`, but does not rewrite per-file frontmatter.
ADR-0023 (`meta/decisions/ADR-0023-meta-directory-migration-framework.md`)
documents the framework.

**Plugin manifest.** `.claude-plugin/plugin.json:10-21` lists `./skills/work/`
at line 16 and `./skills/visualisation/` at line 19. The category is
already correctly registered.

### 5. Decisions that have been made (the answers locked in)

For each design question raised during this research, the resolution is:

- **Kanban column model (Q1).** Configurable column set in
  `.claude/accelerator.md`, defaulting to the `templates/work-item.md`
  status enum (`draft | ready | in-progress | review | done | blocked
  | abandoned`). Server-driven so the launcher can inject the list and
  the frontend renders dynamically. Unknown statuses still flow into
  the "Other" swimlane.
- **ID pattern and slug derivation (Q2).** Pattern-aware. The launcher
  invokes `work-item-pattern.sh --compile-scan` against the configured
  pattern + default project code and writes the compiled regex into
  `config.json`. The server's slug derivation, indexer ID lookup, and
  cluster keying all consume that regex.
- **Wiki-link prefix (Q3).** `[[WORK-ITEM-<id>]]` where `<id>` is whatever
  the configured ID pattern produces. Default: `[[WORK-ITEM-0042]]`.
  Project-prefixed: `[[WORK-ITEM-PROJ-0042]]`. The literal `WORK-ITEM-`
  disambiguates from `[[ADR-NNNN]]` and any future namespaces. The bare
  `[[NNNN]]` form remains unsupported.
- **`work-item-reviews` DocType (Q4).** Add it. Reads from `review_work`
  (default `meta/reviews/work`); slug derivation strips `YYYY-MM-DD-`
  prefix and `-review-N` suffix exactly as the existing `plan-reviews`
  and `pr-reviews` types do.
- **Frontmatter cross-refs (Q5).** Read three fields:
  `work-item:` (matches `templates/plan.md:4`), `parent:`
  (matches `templates/work-item.md:9`), and `related:` (forward-looking
  for inter-work-item references). Aggregate into a list; the wire
  format becomes `IndexEntry.workItemRefs: string[]` (replacing the
  scalar `IndexEntry.ticket`).
- **Sequencing (Q6).** Phase 12 of the visualiser is in fact complete
  (the plan's `status: draft` is stale and should be marked done in
  passing). This work-item migration ships **after** Phase 12, as a
  coordinated four-phase change.

### 6. Open issues found in the visualiser code beyond the rename

These don't strictly belong to the rename but should be addressed in the
same window because the same files are being touched:

1. The `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`
   audit identified `agents/documents-locator.md:25,54,74,121` as still
   referencing `tickets/`. That document also flags
   `templates/pr-description.md:24` ("Link to relevant ticket, plan, or
   research document") — neither file was touched by Migration 0001 and
   both still need updating. They are **outside** the visualiser scope but
   should be confirmed clean before the visualiser rename ships, since
   the visualiser surfaces both.
2. The migration script ran successfully in this workspace, but the live
   `meta/work/` files retain `type: adr-creation-task` (legacy schema).
   The visualiser must tolerate both legacy and new-schema work-items
   gracefully — the plan must explicitly call this out as a regression-
   test target.

## Phased Plan

The migration ships as four phases. Phase 1 unbreaks the visualiser end-to-end
on the default ID pattern (which is what every consumer uses today). Phases
2 and 3 layer the new infrastructure (configurable patterns, configurable
columns, multi-field cross-refs) on top. Phase 4 lands the documentation
and ADR work and validates a fresh-project install.

### Phase 1 — Coordinated rename: server, launcher, and frontend (default pattern)

**Goal.** Visualiser works end-to-end in a freshly migrated project on the
default `{number:04d}` ID pattern, with all wire-format keys, file paths,
field names, route segments, UI strings, and tests renamed in lockstep.

**Server changes.**
- `DocTypeKey::Tickets` → `DocTypeKey::WorkItems` at `docs.rs:8`; kebab
  serde → `"work-items"`. Add `DocTypeKey::WorkItemReviews` reading from
  `review_work` (default `meta/reviews/work/`); slug-strip
  `YYYY-MM-DD-(.+)-review-\d+\.md`.
- `config_path_key()`, `label()`, `in_kanban()`, `all()` updated.
- `slug.rs`: rename `strip_prefix_ticket_number` → `strip_prefix_work_item_id`;
  default behaviour unchanged (still `<digits>-<rest>`).
- `clusters.rs`: rename `Completeness.has_ticket` → `has_work_item` (wire
  `hasWorkItem`); update set-arm and ranking accordingly. Canonical rank
  unchanged (work-items remain at the head of the lifecycle pipeline).
- `patcher.rs`: rename `TicketStatus` → `WorkItemStatus`. **Variant set
  unchanged in this phase** — still `Todo | InProgress | Done` so the
  existing kanban frontend continues to function. Phase 3 widens this.
- `frontmatter.rs`: rename `ticket_of` → `work_item_refs_of` returning
  `Vec<String>`. **In Phase 1 the function still reads only the legacy
  `ticket:` field** (single-element vec, or empty). The new
  `work-item: / parent: / related:` triple lands in Phase 3.
- `indexer.rs`: rename `IndexEntry.ticket` → `IndexEntry.workItemRefs`
  (`Vec<String>`); rename `ticket_by_number` → `work_item_by_number`;
  rename `parse_ticket_number` and `ticket_number_from_entry`. Internal
  representation stays `u32`-keyed in this phase.
- `api/mod.rs`: rename `OnlyTicketsAreWritable` → `OnlyWorkItemsAreWritable`;
  update literal string `"only work-items are writable"`.
- `api/docs.rs`: type guard at `:154-156` updated to compare against
  `DocTypeKey::WorkItems`.
- `server.rs`: writable-roots wiring at `:60` reads
  `cfg.doc_paths.get("work")`.
- Move `tests/fixtures/meta/tickets/` → `tests/fixtures/meta/work/`;
  rename fixture filenames (`0001-first-ticket.md` →
  `0001-first-work-item.md`); update `seeded_write_driver` and other
  test helpers in `file_driver.rs`.

**Launcher changes.**
- `scripts/write-visualiser-config.sh`:
  - Line 39: rename to `WORK="$(abs_path work meta/work)"`.
  - Add a `REVIEW_WORK="$(abs_path review_work meta/reviews/work)"` line.
  - Line 89/105 jq invocation: replace `tickets` with `work`; add a
    `review_work` entry under `doc_paths`.
- `SKILL.md:22`: replace `Tickets directory` placeholder with `Work
  directory`; add a `Work reviews directory` line.
- Plugin-level `.claude-plugin/plugin.json` already registers
  `./skills/visualisation/` (line 19) — no change.

**Frontend changes.**
- `src/api/types.ts`:
  - Line 4-7: `DocTypeKey` literal `'tickets'` → `'work-items'`; add
    `'work-item-reviews'` slot.
  - Line 13-17: `DOC_TYPE_KEYS` runtime array.
  - Line 48: `IndexEntry.ticket` → `IndexEntry.workItemRefs: string[]`.
  - Line 98: `Completeness.hasTicket` → `hasWorkItem`.
  - Line 128/139: `LIFECYCLE_PIPELINE_STEPS[0]` becomes
    `{ key: 'hasWorkItem', docType: 'work-items', label: 'Work item',
    placeholder: 'no work item yet' }`.
  - `STATUS_COLUMNS` hard-coded values stay in Phase 1; widening to
    configurable lands in Phase 3.
- `src/api/fetch.ts:29-54`: rename `patchTicketFrontmatter` →
  `patchWorkItemFrontmatter`.
- File rename: `src/api/ticket.ts` → `src/api/work-item.ts` (and the
  matching `.test.ts`); rename exports `parseTicketNumber` →
  `parseWorkItemNumber`, `groupTicketsByStatus` →
  `groupWorkItemsByStatus`.
- File rename: `src/api/use-move-ticket.ts` → `use-move-work-item.ts`;
  rename exports.
- `src/api/wiki-links.ts:20`: change regex to
  `/\[\[(ADR|WORK-ITEM)-([A-Z0-9-]+)\]\]/g` (the `WORK-ITEM-` literal,
  with an opaque tail that future Phase 2 makes pattern-aware).
- `src/api/wiki-links.ts:22`: `WikiLinkKind = 'ADR' | 'WORK-ITEM'`.
- `src/components/MarkdownRenderer/wiki-link-plugin.ts:17,77`:
  Resolver type updated.
- `src/api/use-wiki-link-resolver.ts:19,36-39,41,45`:
  `queryKeys.docs('work-items')`, `fetchDocs('work-items')`.
- `src/api/use-doc-events.ts:30,74`:
  `event.docType === 'work-items'`.
- `src/routes/kanban/KanbanBoard.tsx`:
  - Imports update to renamed modules.
  - Line 21-22: Other-column description updated.
  - Lines 28,33,37: toast strings: `"loading the work items"`,
    `"This work item was updated by another editor"`, `"The work item
    could not be saved"`.
  - Lines 54,55,152: `queryKeys.docs('work-items')`,
    `fetchDocs('work-items')`.
- File rename: `src/routes/kanban/TicketCard.tsx` →
  `WorkItemCard.tsx` (component, props, CSS module rename in lockstep).
  Critically, line 39's hardcoded link `params={{ type: 'tickets',
  fileSlug }}` becomes `params={{ type: 'work-items', fileSlug }}`.
- `src/routes/kanban/KanbanColumn.tsx:3,19,44,48`:
  `import { WorkItemCard }`, `workItemWord = count === 1 ? 'work item'
  : 'work items'`, `'No work items'`, `<WorkItemCard>`.
- `src/routes/kanban/announcements.ts`: rename
  `ticketNumberFromRelPath` → `workItemNumberFromRelPath`; every aria-
  live string updated to say "work item".
- `src/api/test-fixtures.ts:8,15`: default `type` and field names.
- All `*.test.tsx`, `*.test.ts` updates: literal paths
  `meta/tickets/` → `meta/work/`, fixture types
  `'tickets'` → `'work-items'`, expectation strings.
- `frontend/e2e/kanban.spec.ts:6-11,46,54,61,89,96,108`: fixture
  directory and filename renames.

**What this phase intentionally does NOT change.**
- Status enum stays `Todo | InProgress | Done`. Phase 3 widens.
- Slug regex stays `<digits>-<rest>`. Phase 2 makes pattern-aware.
- Frontmatter cross-ref still reads only `ticket:`. Phase 3 reads
  `work-item:`, `parent:`, `related:`.
- Wiki-link inner pattern still matches digits-only. Phase 2 widens
  to project-prefixed IDs.

**Deliverable.** A fresh project initialised via `/accelerator:init` with
default-pattern work-items in `meta/work/` boots the visualiser, lists
the 11 doc types in the sidebar, renders work-items in the kanban,
allows drag-drop status changes that persist, resolves
`[[WORK-ITEM-NNNN]]` (default pattern) and `[[ADR-NNNN]]` wiki-links,
and surfaces `meta/reviews/work/` artefacts in their own library tab.

**Estimated complexity.** High (~130 touch-points), but the wire-format
flip is atomic — server and frontend ship together as a single PR.

### Phase 2 — Configurable ID pattern support

**Goal.** Visualiser correctly handles project-prefixed work-item IDs
when the user has configured `work.id_pattern: "{project}-{number:04d}"`
(or another non-default pattern).

**Launcher changes.**
- `scripts/write-visualiser-config.sh`:
  - Read `work.id_pattern` and `work.default_project_code` via
    `config-read-value.sh`.
  - Invoke
    `${PLUGIN_ROOT}/skills/work/scripts/work-item-pattern.sh
    --compile-scan "$ID_PATTERN" "$PROJECT_CODE"` to obtain the
    scan regex.
  - Write `work_item.scan_regex`, `work_item.id_pattern`, and
    `work_item.default_project_code` into `config.json`.
  - Validate the pattern at launch time (fail fast with a clear
    message if `wip_validate_pattern` rejects).

**Server changes.**
- `config.rs`: extend the schema with a `work_item: WorkItemConfig {
  scan_regex: String, id_pattern: String, default_project_code:
  Option<String> }` field. Compile the regex once at boot via the
  `regex` crate; cache it.
- `slug.rs`: `strip_prefix_work_item_id` consumes the configured regex
  rather than hard-coding `^(\d+)-`. Capture group 1 is the ID; the
  remainder of the stem (after the trailing `-`) is the slug.
- `indexer.rs`: change `work_item_by_number: HashMap<u32, PathBuf>` to
  `work_item_by_id: HashMap<String, PathBuf>` (string keys
  accommodate `PROJ-0042`). Update `parse_work_item_number` →
  `parse_work_item_id` returning `Option<String>`. Update
  `ticket_number_from_entry` → `work_item_id_from_entry`.
- `clusters.rs`: cluster keys remain slugs, not IDs — no change.
- New tests: a fixture with three work-items under a project pattern,
  asserting slug derivation, ID extraction, and lookup.

**Frontend changes.**
- `src/api/wiki-links.ts:20`: regex extension. The `WIKI_LINK_PATTERN`
  becomes `/\[\[(ADR|WORK-ITEM)-([A-Za-z][A-Za-z0-9]*-\d+|\d+)\]\]/g`
  (project-prefixed or bare-numeric ID after `WORK-ITEM-`). The
  resolver looks up `IndexEntry.workItemId` (added below) verbatim.
- `src/api/types.ts`: add `IndexEntry.workItemId: string | null`
  emitted by the server alongside the existing fields.
- `wiki-links.ts:79-86`: rebuild `workItemById: Map<string, IndexEntry>`
  keyed on full ID (was `Map<number, IndexEntry>`).
- Test: a fixture project pattern showing a wiki-link resolves
  correctly from `[[WORK-ITEM-PROJ-0042]]` to a work-item with
  `workItemId: "PROJ-0042"`.

**Deliverable.** A fixture project configured with
`{project}-{number:04d}` and `default_project_code: "PROJ"` renders
correctly: cards show, slugs cluster, `[[WORK-ITEM-PROJ-0042]]` links
resolve, kanban drag-drop succeeds.

**Estimated complexity.** Medium. Self-contained on top of Phase 1.

### Phase 3 — Configurable kanban columns + multi-field cross-references

**Goal.** Kanban columns are configurable per-project; frontmatter cross-
references read from `work-item:`, `parent:`, `related:`.

**Configuration schema.**
- New optional config block in `.claude/accelerator.md`:

  ```yaml
  visualiser:
    kanban_columns:
      - { key: ready,       label: "Ready" }
      - { key: in-progress, label: "In progress" }
      - { key: review,      label: "In review" }
      - { key: done,        label: "Done" }
  ```

- Default (when omitted): the seven `templates/work-item.md:7` statuses,
  in template order:
  `draft | ready | in-progress | review | done | blocked | abandoned`.
- Document under `skills/config/configure/SKILL.md` in a new
  "Visualiser" subsection.
- A new path-resolver helper, e.g. `config-read-visualiser.sh`,
  modelled on `config-read-review.sh`, exposes the column list as
  YAML/JSON.

**Launcher changes.**
- `write-visualiser-config.sh`: read the column list and emit it under
  a new `kanban_columns` field in `config.json`. Validate that each
  key is a syntactically valid status string; emit a clear error if
  the list is malformed.

**Server changes.**
- `config.rs`: extend schema with `kanban_columns:
  Vec<KanbanColumn { key, label }>`.
- `patcher.rs`: replace the hard-coded `TicketStatus` enum's
  validation with a string-based check against the configured
  column keys. The patcher itself remains a thin YAML-line editor.
- `api/docs.rs:107-109`: `PatchFields { status: Option<String> }`;
  validate the value against the configured column keys at request
  time, returning `400 Bad Request` for unknown values (with a list
  of accepted keys).
- New endpoint or extension to `GET /api/types`:
  ship `kanban_columns` to the frontend (one server roundtrip;
  cached in TanStack Query).

**Frontend changes.**
- `src/api/types.ts:158-175`: replace the hardcoded `STATUS_COLUMNS`
  with a TanStack Query consumer that reads server-provided columns.
- `src/routes/kanban/KanbanBoard.tsx:189-196`: render dynamic
  columns; "Other" still catches unknowns, with a help-hover that
  surfaces which statuses are configured.
- `src/routes/kanban/KanbanColumn.tsx`: column count adapts.
- Drag-drop allowed targets: any configured column except Other.

**Server-side multi-field cross-refs.**
- `frontmatter.rs::work_item_refs_of` reads three keys in priority
  order: `work-item` (string), `parent` (string), `related` (string
  or array). Returns deduplicated `Vec<String>` of full IDs. Numeric
  values are coerced via `wip_canonicalise_id`-equivalent logic
  (zero-padded to four digits when no project pattern is configured;
  otherwise format-applied per `config.work_item.id_pattern`).
- `indexer.rs`: build a reverse cross-ref index — for any work-item
  referenced by another doc's `work-item:`, `parent:`, or `related:`,
  populate `IndexEntry.referencedBy` (already structurally present
  for plan-reviews via `target:` per ADR-0017 / D7).

**Frontend changes for cross-refs.**
- `RelatedArtifacts/RelatedArtifacts.tsx`: render multiple cross-refs
  as a list rather than a single field; preserve the
  declared/inferred visual distinction.
- `library` views: surface `parent` work-items prominently for
  story/task work-items; surface child work-items on epics.

**Deliverable.** A project that configures
`visualiser.kanban_columns: [ready, in-progress, review, done]`
sees four columns. A work-item with `parent: 0001` and
`related: [0007, 0009]` shows three cross-ref links in the library
view's "Related artifacts" aside, and is reverse-linked from
work-items 0001/0007/0009.

**Estimated complexity.** Medium-high. The kanban-column
plumbing crosses every layer; the cross-ref work is more contained.

### Phase 4 — Documentation, ADR, and validation

**Goal.** Ship-ready release of the renamed visualiser.

**Deliverables.**
- New ADR (probably `ADR-0028` or next free ADR number) documenting:
  - The terminology rename's impact on the visualiser surface.
  - The configurable kanban-column model and default fallback.
  - The wiki-link prefix `[[WORK-ITEM-...]]` and its pattern-aware tail.
  - The cross-ref frontmatter triple (`work-item`, `parent`, `related`).
- README updates: section on configurable columns; updated screenshots
  if any; mention of the wiki-link prefix change.
- CHANGELOG entry covering Phases 1–3.
- `skills/config/configure/SKILL.md` updates: new "Visualiser"
  subsection covering the columns config schema.
- Manual validation in two fresh projects:
  1. Default ID pattern (covers Phase 1).
  2. Project ID pattern with custom kanban columns (covers Phases 2–3).
- `meta/validations/2026-XX-XX-visualiser-work-item-migration-validation.md`
  documenting the validation runs.
- A small straggler-cleanup pass over the visualiser code looking for
  comments, log messages, error strings, and CSS class names that
  still say "ticket" — using the same audit technique as
  `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`.

**Estimated complexity.** Low.

### Phase ordering and dependency rules

| Phase | Depends on | Ships when |
|---|---|---|
| 1 | Phase 12 of visualiser (already complete) | Single PR; server + frontend together |
| 2 | Phase 1 | Single PR; can be in same window as Phase 1 if reviewer prefers |
| 3 | Phase 1 (kanban-cols) and Phase 2 (cross-refs IDs) | Single PR or two |
| 4 | Phases 1–3 | Final PR; ADR + docs + validation |

Phases 2 and 3 can be developed in parallel if desired — they touch
different surfaces (slug/wiki-link vs kanban-cols/cross-refs). Phase 1
must land first because it sets up the type renames and field shapes
the others build on.

### Side cleanups to bundle

- Update Phase 12 plan's frontmatter `status:` from `draft` to
  `complete` (or delete the field — many plan templates don't carry it
  once shipped). Adds nothing functional but the metadata is wrong.
- Re-audit `agents/documents-locator.md` and `templates/pr-description.md`
  per the open items in
  `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`
  — these were flagged but never actioned. Both surface in the
  visualiser's library, so users will see the stale "ticket" text
  until they're updated.
- Workspace's `meta/work/` files retain `type: adr-creation-task` legacy
  schema. Out of scope for this work; flag in Phase 4 validation that
  the visualiser correctly renders both legacy and new-schema files.

## Code References

### Visualiser launcher
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:39,89,105`
  — broken `tickets` lookup
- `skills/visualisation/visualise/SKILL.md:22` — stale path placeholder

### Server
- `skills/visualisation/visualise/server/src/server.rs:58-67` — writable-roots
  wiring (root cause of broken kanban PATCH)
- `skills/visualisation/visualise/server/src/docs.rs:5,8,23,38,53,70,115`
  — `DocTypeKey` enum, wire format, sidebar label
- `skills/visualisation/visualise/server/src/slug.rs:11,35-42` — slug
  derivation hardcoded to digit-prefix
- `skills/visualisation/visualise/server/src/clusters.rs:11,69,98,110`
  — `Completeness.has_ticket`, canonical rank
- `skills/visualisation/visualise/server/src/patcher.rs:5,14-20,49`
  — `TicketStatus` enum
- `skills/visualisation/visualise/server/src/frontmatter.rs:297-306`
  — `ticket_of` accessor
- `skills/visualisation/visualise/server/src/indexer.rs:25,55,109,132-134,152,156,477-493,531-538,611-617,632-635`
  — `IndexEntry.ticket`, `ticket_by_number`, parsers
- `skills/visualisation/visualise/server/src/api/mod.rs:53,54,95,155`
  — `OnlyTicketsAreWritable` error
- `skills/visualisation/visualise/server/src/api/docs.rs:107-109,154-156`
  — `PatchFields`, ticket-only gate
- `skills/visualisation/visualise/server/src/sse_hub.rs:8-21` — SSE
  payload `docType` literals
- `skills/visualisation/visualise/server/src/file_driver.rs:594-616`
  — fixture helper
- `skills/visualisation/visualise/server/tests/fixtures/meta/tickets/`
  — fixtures pinned to old path

### Frontend
- `skills/visualisation/visualise/frontend/src/api/types.ts:4-7,13-17,48,98,128,139,158-175`
- `skills/visualisation/visualise/frontend/src/api/fetch.ts:29-54`
- `skills/visualisation/visualise/frontend/src/api/ticket.ts:9,30`
- `skills/visualisation/visualise/frontend/src/api/use-move-ticket.ts:8,13,15,24-26,45`
- `skills/visualisation/visualise/frontend/src/api/wiki-links.ts:6,20,22,45,79-86,110`
- `skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts:19,36-39,41,45`
- `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:30,74`
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/wiki-link-plugin.ts:17,77`
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.tsx:11,12,15,21-22,28,33,37,54-55,152`
- `skills/visualisation/visualise/frontend/src/routes/kanban/TicketCard.tsx:6,8,26,39`
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanColumn.tsx:3,19,44,48`
- `skills/visualisation/visualise/frontend/src/routes/kanban/announcements.ts:9,21-49`
- `skills/visualisation/visualise/frontend/src/api/test-fixtures.ts:8,15`
- `skills/visualisation/visualise/frontend/e2e/kanban.spec.ts:6-11,46,54,61,89,96,108`

### Plugin (already migrated)
- `scripts/config-read-path.sh:7-19` — recognised path keys
- `scripts/config-read-review.sh:15,28-31,54-60,85-86` — review modes,
  lens set, config keys
- `skills/work/scripts/work-item-pattern.sh` — pattern compiler CLI
- `skills/work/scripts/work-item-common.sh:212,223,231,239,269,278,290,354,428,449`
  — pattern-compiler helpers
- `skills/work/scripts/work-item-next-number.sh:58-89` — pattern-driven
  ID allocator
- `templates/work-item.md` — canonical schema
- `templates/plan.md:4` — `work-item:` cross-ref key
- `meta/decisions/ADR-0022-work-item-terminology.md` — terminology decision
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
  — migration framework
- `meta/.migrations-applied:1` — `0001-rename-tickets-to-work` applied
- `.claude-plugin/plugin.json:16,19` — work and visualisation skill
  categories registered

## Architecture Insights

- **Wire-format atomicity is the binding constraint on phase splits.** Any
  rename that touches a `DocTypeKey` literal, an `IndexEntry` field name,
  or a `Completeness` flag must flip on server and frontend in lockstep.
  Phase 1 absorbs all of these into a single coordinated PR; Phases 2 and
  3 layer additive features that don't break the wire.
- **The configurable ID pattern infrastructure is naturally additive to
  the visualiser** — the launcher already orchestrates plugin scripts via
  `config-read-path.sh`, so calling `work-item-pattern.sh --compile-scan`
  fits the existing pattern. The Rust server consumes a regex string;
  it doesn't need to know about the pattern grammar.
- **The kanban-column generalisation flows naturally from the
  string-typed status approach.** Once `WorkItemStatus` becomes a
  validated `String` (not an enum) and the frontend reads columns from
  the server, adding new statuses is configuration-only. This is closer
  in spirit to the plugin's existing `BUILTIN_*_LENSES` patterns than
  to a hardcoded enum.
- **The frontmatter cross-ref triple (`work-item`, `parent`, `related`)
  is a small abstraction that scales** — the Rust indexer can build the
  reverse-link index once at scan time, and any number of future
  doc-types can populate any subset of the three fields without
  visualiser changes.
- **The `WORK-ITEM-` literal in wiki-links is more verbose than the
  old `TICKET-` but disambiguates cleanly from `ADR-`.** Future ID
  namespaces (`EPIC-`, `RFC-`) extend the same pattern. The literal
  is a static prefix; the configurable tail handles project-prefixed
  IDs without regex re-engineering.
- **Phase 12 (packaging) being complete eliminates a release-coupling
  risk.** This work-item migration ships independently as the next
  visualiser release.

## Historical Context

- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`
  — the design doc for the visualiser, written when "ticket" was still
  the canonical term. The 12-phase plan, D1–D10 design decisions, and
  spec-vs-reality gaps it captured remain authoritative apart from the
  terminology.
- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — the original
  ticket-skill design that introduced the term. Resolved Question 1
  flagged the configurable filename pattern as a future enhancement,
  which subsequently shipped per the 2026-04-28 research.
- `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md` — the
  rename's full surface analysis and six-phase plan; the visualiser was
  not in scope of that work.
- `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`
  — audit of stragglers after the rename. Identifies
  `agents/documents-locator.md` and `templates/pr-description.md` as
  still carrying ticket references; both are visible from the visualiser.
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md` —
  introduces the pattern compiler, the per-project `{project}` token,
  and the migration extension. The visualiser must consume this
  infrastructure in Phase 2.
- `meta/decisions/ADR-0022-work-item-terminology.md` — the canonical
  ADR for the rename; documents why "work-item" was chosen over `task`,
  `story`, `backlog-item`, `issue`, and `item`.
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` —
  the migration framework backing
  `meta/.migrations-applied:1`'s `0001-rename-tickets-to-work` entry.
- `meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md`
  — Phase 12 plan; per the user's confirmation this work is in fact
  complete and the plan's `status: draft` is stale.

## Related Research

- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md` —
  visualiser design and Phase 1–12 plan.
- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — original
  ticket-management research.
- `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md` —
  terminology rename plan.
- `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`
  — post-migration cleanup audit.
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md` —
  pattern compiler and per-project token design.

## Resolved Questions

1. **Kanban column model**. Configurable column set in
   `.claude/accelerator.md`, defaulting to the seven statuses defined
   in `templates/work-item.md:7`. Server-driven; the frontend renders
   dynamically with an "Other" swimlane for unknown values.
2. **Configurable ID pattern**. Pattern-aware throughout. The launcher
   compiles the configured pattern via
   `skills/work/scripts/work-item-pattern.sh --compile-scan` and ships
   the resulting regex to the server in `config.json`.
3. **Wiki-link prefix**. `[[WORK-ITEM-<id>]]` where `<id>` is the
   ID-pattern's full output. Default pattern: `[[WORK-ITEM-0042]]`.
   Project-prefixed: `[[WORK-ITEM-PROJ-0042]]`. The literal
   `WORK-ITEM-` disambiguates from `[[ADR-NNNN]]`.
4. **`work-item-reviews` DocType**. Add as a net-new DocType reading
   from `review_work` (default `meta/reviews/work/`). Slug derivation
   matches `plan-reviews` and `pr-reviews`.
5. **Frontmatter cross-ref fields**. Read three keys: `work-item:`
   (matches `templates/plan.md:4`), `parent:` (matches
   `templates/work-item.md:9`), and `related:` (forward-looking).
   Aggregate into a `Vec<String>` on `IndexEntry.workItemRefs`.
6. **Sequencing relative to Phase 12**. Phase 12 is in fact complete.
   This work ships immediately afterwards as the next visualiser
   release.

## Open Questions

None blocking. The phased plan above is fully specified given the
resolved decisions; remaining choices are tactical (PR boundaries,
CHANGELOG wording, ADR number assignment) and best left to the
implement-plan stage.

---
type: "codebase-research"
id: "2026-10-10-0296-sync-round-trip-defects"
title: "Sync round-trip defects between local work items and the tracker"
date: "2026-10-10T21:09:12+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0296"
parent: "work-item:0296"
topic: "Sync round-trip defects between local work items and the tracker"
tags: ["research", "codebase", "sync", "baseline", "digest", "projection", "dossier", "work-adapters", "remote-projection", "jira-client", "linear"]
revision: "7cd9e74bbad6ca74adcfb95de719dcb231baf4f4"
repository: "accelerator"
last_updated: "2026-10-10T21:09:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Sync round-trip defects between local work items and the tracker

**Date**: 2026-10-10T21:09:12+00:00
**Author**: Toby Clemson
**Git Commit**: 7cd9e74bbad6ca74adcfb95de719dcb231baf4f4
**Branch**: detached working copy (workspace `ticket-management`)
**Repository**: accelerator

## Research Question

What does the codebase look like today for work item 0296 (Sync Round-Trip
Defects Between Local Work Items and the Tracker)? The question covers where
each defect originates, which code the required behaviour touches, what
constrains the design, and where the work item's description of the code is
wrong or incomplete.

## Summary

Both defects come from one design choice. `RemoteIssue.body` is the projected
`"{title}\n{description}"`, and every consumer except create-from-remote
treats it as the local body. The baseline `remote_hash`, the pull
reconstruction and the dossier's remote side all use the full projected body.
Only `split_projected` (`work-cli`) strips the title, and its only caller is
`author_from_remote`.

The work item's Technical Notes are accurate. The live code adds the
following:

- 🔴 **Jira pulls write raw ADF JSON into the local body.** Today's pull
  writes `"{summary}\n{canonical ADF JSON}"` verbatim. Defect 1 is worse for
  Jira than the item states, and nothing in the read or sync path calls
  `document_to_markdown`.
- **The `adf` question is answered by the rules.** Making
  `remote-projection` depend on `jira-client` creates a hard Cargo cycle,
  since `jira-client` already depends on `remote-projection`. The role lint
  forbids it too: a technical library may depend only on technical libraries.
  `adf` has to move down into a technical library.
- **Equivalence cannot live in `work`.** ADR-0069 rule 1 keeps codecs out of
  the domain closure. `classify` and `differing_sections` are both in `work`,
  so each needs a new port or an adapter-side decision.
- ✅ **`--preview` already leaves `last-sync.json` untouched.** It returns
  before `ItemApplier` is built (`run.rs:1163-1192`). The requirement is a
  regression test, not a fix.
- **Targeted pull-create already writes no title line.** It goes through
  `create_from_remote` → `author_from_remote` → `split_projected`. For a Jira
  item it still writes ADF JSON as the body.
- **More baseline writers than the item lists.** `Entry` is built in 9
  production sites; 0296 names 4. The others are `work update` (`update.rs`),
  `realign.rs`, `promotion.rs` and `created_baseline.rs`. All of them must
  move to the new recipe and entry shape.
- **Promotion compares projected bodies.** `promotion.rs` matches remote
  bodies against `"{title}\n{body}"` and compares `remote_body` hashes in
  `agree_but_for_ids`. Changing the recipe changes these comparisons too.

## Detailed Findings

### Projection and the tracker port

- `RemoteIssue { key, updated, body }` is defined at `cli/tracker/src/lib.rs:107-135`.
  The doc comment at 114-133 describes the body as the title line, then the
  description, with no blank line between them and one trailing newline. A
  structured description is canonicalised, and the body is un-normalised.
  Lines 128-129 state that a push followed by a read is not the identity.
- `RemoteTracker::update(id, title, body)` (`tracker/src/lib.rs:641-646`) and
  `create(title, body, kind)` (617-622) take the local title and body
  separately. The port has no notion of the projected title on the write
  side.
- `project_raw` (`cli/remote-projection/src/lib.rs:97-124`) is the only
  production projector. Its callers are `cli/jira-client/src/client.rs:470`
  and `cli/linear-client/src/client.rs:791`. Both wrap the result in
  `port_body` (`cli/tracker-support/src/transport.rs:45-48`), which trims
  trailing `\n` and appends exactly one.
  - Linear: `format!("{title}\n{description}")` (lines 118-122).
  - Jira: `format!("{summary}\n{canonical ADF}")`. A missing description
    becomes the literal `null` (lines 107-112). `canonicalise` is
    `serde_json::to_string`, key-sorted because `preserve_order` is off
    (lines 48-54).
- `remote-projection` depends only on `serde_json`, and its role is
  `technical-library` (`Cargo.toml:10,16`).

### Classification and change detection

- `classify` (`cli/work/src/sync/classify.rs:110-134`) works in this order:
  1. No `external_id` gives `Unsynced`.
  2. Presence `Indeterminate` or `Absent` gives that state.
  3. Otherwise both `local_changed` and `remote_changed` run, and their
     results map to `Synced`, `LocallyModified`, `RemotelyModified` or
     `Conflict`.
- `local_changed` (66-80) returns `true` immediately when the baseline local
  hash is `None`. Otherwise an mtime at or below the per-item watermark
  (`local_synced_at`) short-circuits to unchanged, and failing that it
  compares `digest::local` with the baseline.
- `remote_changed` (82-99) returns `true` when the baseline remote hash is
  `None`. Otherwise a reported stamp equal to the baseline's proves
  unchanged, a body that was not fetched counts as changed, and failing that
  the hashes are compared.
- The doc comment at 101-105 makes a missing baseline yield `Conflict` on
  purpose. 0296's recovery rule, which treats a missing local hash as unknown
  rather than changed, reverses this contract. It needs a new third outcome,
  and that changes the existing `sync_classify.rs` tests.
- The `ItemDigests` port (19-39) has three methods: `mtime`, `local` and
  `remote_body`. `LazyItemDigests` (`cli/work-adapters/src/sync/digest.rs:104-170`)
  reads the file lazily. Equivalence is a body-to-body comparison, so the
  port must grow a method that exposes both bodies, or a sibling port. Rule 1
  in ADR-0069 forbids parsing in `work`.
- Empty baseline strings become `None` in `GatheredFacts::plan_inputs` at
  `cli/work-adapters/src/sync/fetch.rs:125-138`. With no entry, both hashes
  are `None` and the timestamp is `NotRead`. The watermark falls back to
  `baseline.timestamp()` (113-116).
- The remote body is `None` when the bulk stamp proves the issue unchanged,
  or when the follow-up `show` fails (`fetch.rs:341-352, 364-371`).
  ⚠️ Recovery and equivalence need the remote body. An item with no baseline
  entry has a `NotRead` timestamp, so the stamp cannot prove it unchanged
  and its body is fetched. An equivalent item with a stale entry may skip
  the read; that does no harm, because it classifies as `Synced`.
- `plan()` (`cli/work/src/sync/plan.rs:105-136`) calls `classify` then
  `decide`. A `--resolve` choice rewrites `Prompt` to `Pull` or `Push`
  (119-128). `needs_body_read` (74-93) duplicates the stamp test.

### Digests

- `sha256_hex` (`digest.rs:52-61`) produces 64 lowercase hex characters,
  using `sha2` 0.11 (`cli/Cargo.toml:80`).
- `split_frontmatter_and_body` (22-49) splits on `lines()` and needs both
  fences. The body keeps its leading blank line and loses its trailing
  newline.
- `digest::local` (76-86) hashes the result of `filter_frontmatter_keys`, a
  `\n`, `trim_lines(body)` and another `\n`. Frontmatter and body are hashed
  together. This is the legacy recipe.
- `digest::remote_body` (91-94) is `sha256_hex(trim_lines(projected_body))`,
  so the title line is hashed too.
- `trim_lines` (`cli/work/src/normalise.rs:45-47`) trims ASCII whitespace
  from each line, drops trailing blank lines and keeps leading blank lines.
  `IGNORE_KEYS` (10-17) holds `last_updated`, `last_updated_by`, `id`,
  `external_id`, `updated_at` and `revision`.
- No Markdown parser is in `cli/Cargo.lock`. The only Markdown tokenizer is
  the hand-rolled one in `cli/jira-client/src/adf/`.

**Production digest call sites**, all of which move to the new recipe:

| Site | Digest | Remote body source |
|---|---|---|
| `work-adapters/src/sync/apply.rs:205-214` `push` | both | `show` read-back; `""` on failure |
| `apply.rs:244-246` `pull` | both | `request.projected_body` |
| `apply.rs:297-299` `create_from_remote` | both | `issue.body` |
| `apply.rs:474-482` `link_and_baseline` | both | `show` read-back; `""` on failure |
| `work-adapters/src/creation.rs:543-570` | both | `read_back_of`; `Unknown` → `""` |
| `work-adapters/src/promotion.rs:338,345,703,798` | both | read-back; `agree_but_for_ids` |
| `work-adapters/src/sync/realign.rs:100,109` | `local` | — |
| `work-cli/src/update.rs:241-244` | both | `show` read-back; `""` on failure |
| `work-adapters/src/sync/run.rs:1053`, `work-cli/src/list.rs:205` | lazy | gathered body |

### Pull path (Defect 1)

```mermaid
flowchart LR
  F[fetch: RemoteIssue.body<br/>title⏎description] -->|Pull / resolve=remote| R[reconstruct_pulled_content]
  R -->|local fm + projected body| W[file: title line above H1]
  F -->|untracked / --target| A[author_from_remote]
  A -->|split_projected| W2[file: fm title, description body]
  F -->|Prompt| D[readable_dossier]
  D -->|reconstruct_pulled_content| S[(preamble) holds title line]
  R -->|digest::remote_body full body| B[(last-sync.json)]
  A -->|digest::remote_body full body| B
```

- `reconstruct_pulled_content` (`cli/work-adapters/src/sync/run.rs:378-386`)
  builds `format!("---\n{frontmatter}\n---\n\n{remote_body}")`. The
  frontmatter is kept as raw text and the remote title never reaches
  frontmatter `title`. This is the only reconstruction, so the ordinary pull
  already is 0296's frontmatter-kept, body-replaced pull.
- `pull_outcome` (`run.rs:736-757`) passes `remote.body.unwrap_or("")`. A
  body that was never read becomes an empty pulled body. An unreadable local
  file gives `---\n\n---\n\n<projected>`.
- `--resolve <id>=remote` works as follows. It is parsed at
  `cli/work-cli/src/sync.rs:279-302`, and the token is mapped at
  `cli/work/src/sync/decide.rs:126-134`. It applies only to `Prompt`, which
  covers `Conflict` and dirty `RemotelyModified` (`decide.rs:91-113`). It
  runs through the same `pull_outcome`.
- `split_projected` (`cli/work-cli/src/sync_author.rs:211-216`) takes the
  first line as the title and joins the rest with `\n`. Its only caller is
  `author_from_remote` (249-326), which writes
  `format!("{frontmatter_block}\n{description}\n")` (312). `work-cli` is a
  composition root, so the pull path in `work-adapters` cannot call it.
  `remote-projection` is the natural new home, since it already owns the
  projection shape.
- The targeted pull-create (0285) has no separate path. `untracked_to_import`
  (`run.rs:905-917`) goes to `ItemApplier::create_from_remote`
  (`apply.rs:274-313`) and then to `author_from_remote`.
- `ItemApplier::pull` (`apply.rs:236-259`) records
  `remote_hash = digest::remote_body(request.projected_body)`. That hash
  covers the full projected body, title included.
- Push (`run.rs:708-734`) sends the frontmatter `title` and the raw body
  after the closing fence. The body includes the leading blank line and any
  `# id: title` H1. `ItemApplier::push` (`apply.rs:193-227`) reads back with
  `show` and baselines the projected body.
- ⚠️ Jira: nothing on the read path converts ADF to Markdown. The only
  production caller of `document_to_markdown` is `cli/jira-cli/src/render.rs:48`.
  Today a Jira pull or create-from-remote writes canonical ADF JSON as the
  local body.

### Conflict dossiers (Defect 2, real-conflict case)

- `build_dossiers` (`run.rs:499-512`) runs from `prepare_run` (`run.rs:1087`)
  in both preview and apply mode.
- `readable_dossier` (`run.rs:545-563`) diffs the raw local file against
  `reconstruct_pulled_content(local, remote_body)`. The remote side is the
  local frontmatter plus the projected body, so the bare title line falls
  into `(preamble)`.
- The header's `title:` is the local frontmatter title only, taken through
  `local_title_and_body` (`run.rs:369-376`). The header is written by
  `dossier_header` (`run.rs:426-436`) with the lines `# Conflict:`,
  `status:`, `title:`, `local modified:` and `remote updated:`.
- `differing_sections` (`cli/work/src/section_diff.rs:176-205`) builds
  sections in this order: `frontmatter`, then `(preamble)`, then the union of
  `## ` headings. Only level-2 headings split, and the match is exact. Each
  section goes through `trim_lines` and is then compared with `==`. A list
  re-serialised from `-` to `*` therefore marks its section as differing.
- Rendering is `work_adapters::diff::render` (`cli/work-adapters/src/diff.rs:10-20`),
  injected at `cli/work-cli/src/sync.rs:1641`. It frames each section as
  `=== {name} (- LOCAL / + REMOTE) ===` and renders the diff with `similar`.
- `persist_conflict_dossiers` (`cli/work-cli/src/sync.rs:603-617`) is called
  with no mode check. It writes `<paths.integrations>/<integration>/conflicts/<id>.md`,
  adds `.gitignore` `*` and clears stale dossiers (554-568).
- `differing_sections` is in the domain crate, so omitting equivalent
  sections means one of two things. The equivalence predicate can be
  injected into `differing_sections` as a port, or `readable_dossier` in
  `work-adapters` can filter the returned `SectionDiff`s.
- The skill text is at `skills/work/sync-work-items/SKILL.md:253-256`. It
  lists six render fields, including "the **title**". The `title:` header
  line itself is not described. Lines 245-252 pin the `status:` line to the
  header region above the first `=== `.

### Baselines and `last-sync.json`

- `Entry` (`cli/work-adapters/src/sync/baseline.rs:17-23`) holds
  `remote_updated_at`, `remote_hash`, `local_hash` and `local_synced_at`. It
  has no serde attributes: parsing and rendering are written by hand.
  - `parse_entry` (86) reads a missing string field as `""`.
  - A missing `local_synced_at` backfills from the document `timestamp`
    (74-82). This is the only compatibility shim, and it is the precedent for
    reading unmarked legacy entries.
  - `render_entry` (101-110) and `render` (178-190) write compact
    single-line JSON with `BTreeMap` order.
- No version or recipe field exists today. Adding `digest_recipe`,
  `local_frontmatter_hash` and `local_body_hash` means:
  - a sum type over the legacy and new recipes in `Entry`;
  - changes to `parse_entry` and `render_entry`;
  - updates to every construction site listed below.
- `BaselineStore` (`cli/work-adapters/src/sync/baseline_store.rs:25-149`)
  supports `load`, `set`, `remove`, `rename` and `finalise_run`. Every
  mutation reloads and then rewrites atomically.
- `finalise_run` (122-148) and `finalise_baseline` (`run.rs:329-367`, apply
  mode only) do two things:
  - They blank `local_hash` for `Prompt`, `SkipConflict` or `SkipDirty` when
    the state is `Conflict` (`run.rs:793-798`). This produces the
    empty-local-hash recovery row.
  - They advance the watermark only for definitively reconciled items.
    `RemoteAbsent` and `Indeterminate` entries are left untouched.
- An unresolved conflict therefore changes only `local_hash` under today's
  code. That fits 0296's "entry unchanged / local hash emptied" rows. But
  0296's last criterion asks that an unresolved conflict's entry keep
  byte-identical hashes, and blanking breaks that whenever `local_hash` was
  non-empty. Of the not-equivalent rows, only "no entry" and "empty local
  hash" leave the entry byte-identical.
- `record_created_baseline` (`cli/work-adapters/src/sync/created_baseline.rs:17-38`)
  maps `RemoteHash::Unknown` to `(NotRead, "")`. This is the empty
  `remote_hash` recovery row.

**`Entry` construction sites (production):**

- `baseline.rs:86`: parse
- `baseline_store.rs:134`: blank `local_hash`
- `apply.rs:219`: push
- `apply.rs:251`: pull
- `apply.rs:304`: create-from-remote
- `apply.rs:487`: link and baseline
- `created_baseline.rs:31`: `create --push`, promotion
- `realign.rs:117`: re-render migration
- `work-cli/src/update.rs:249`: `work update` push, with `local_synced_at: 0`

### Jira `adf` converter

- The public entry points are `markdown_to_document(md, seed) -> Result<Value, AdfError>`
  (`cli/jira-client/src/adf/mod.rs:84-90`) and
  `document_to_markdown(&Value) -> Result<String, AdfError>` (98-100). Both
  are re-exported at `cli/jira-client/src/lib.rs:30-31`.
- Rejections live in `tokenise.rs:74-114`. Fenced lines are exempt.
  - `>` at the start of a line gives `UnsupportedBlockquote`.
  - A line that starts and ends with `|` gives `UnsupportedTable`.
  - An indented list marker gives `UnsupportedNestedList`.
  - Control characters `\u{1e}` and `\u{1f}` give `BadInput`.
- Losses on render come from `render.rs`:
  - `strike` and `underline` are never examined (170-212).
  - A list item renders only `content[0]` (143-150).
  - An unsafe link scheme drops the link and keeps the text.
  - Unknown nodes become `[unsupported ADF node: …]`.
  - Blocks are joined with `\n\n`, with no trailing newline.
- `AdfError` (`mod.rs:26-34`) has render-side variants (`RootNotDoc`,
  `HeadingWithoutLevel`, `ListWithoutContent`) and assemble-side variants.
  ⚠️ The render side can fail, which 0296 does not cover. An unrenderable
  remote ADF needs a defined equivalence result and pull behaviour.
- Moving `adf` touches a fixture corpus, `cli/jira-client/tests/fixtures/adf/`,
  with `assemble-*` and `render-*` cases and frozen bash oracles. It also
  touches `tests/adf.rs`, `adf_differential.rs`, `adf_inventory.rs` and the
  quirks list.

### Test infrastructure

- `RecordingTracker` (`cli/tracker-test-support/src/lib.rs`) stores
  `format!("{title}\n{body}")` on create and update (522, 535, 562). Its
  `Call` enum (41-76) records `Update { id, title, body }`. It never
  re-serialises Markdown; a `*`-rewriting builder would need to be added.
- Engine tests live in `cli/work-adapters/tests/sync_run.rs`.
  - `projected_body()` returns `"Title\nRemote body\n"` (149-151).
  - `pullable()` and `pushable()` (156-214) seed a file and a baseline entry.
  - `baseline_document()` (216-255) seeds `last-sync.json`.
  - Assertions cover `report.reported[..].planned.state`,
    `spy.content_of(path)` and filtered `Call::Update`.
  - No test checks that a pulled file lacks a title line.
- The real-client seam is `cli/work-adapters/tests/sync_run_real_client.rs`.
  It drives the real `JiraClient` and `LinearClient` against a mock HTTP
  server, which exercises `project_raw`. It is the natural home for the
  no-title-line criteria across both trackers.
- `a_create_then_sync_reports_synced` (`cli/work-cli/src/sync.rs:3737-3792`)
  infers `synced` from exit 0 and the absence of `Create` or `Update` calls.
  It never checks the state string.
- The baseline corpus `cli/work-adapters/tests/fixtures/work-item-sync-baseline/case-*/`
  holds `remote.json`, `expected.json`, `integration.txt` and `local.md`. It
  is checked by `sync_baseline_corpus.rs` (case floor 11) and
  `cli/work-adapters/tests/corpus_hashes.rs`. ⚠️ Every `expected.json`
  remote hash pins the legacy recipe, so all of them change or split by
  recipe.
- No fixture has been captured from a live API so far. `accelerator linear
  show` (`cli/linear-cli/src/main.rs:88-114`) prints the `SHOW_DETAILED`
  envelope plus `"outcome"`. That is a superset of the `remote.json` shape,
  so strip `state`, `comments`, `assignee` and `outcome` before committing.
- Golden-test precedents:
  - Hand-rolled `assert_golden` with `UPDATE_GOLDEN=1`
    (`cli/linear-cli/tests/support/mod.rs:81-102`).
  - A `Cargo.lock` version check, `the_jj_binary_matches_the_resolved_library`
    (`cli/vcs-adapters/tests/queries.rs:611-633`), which is the template for
    0296's parser-version golden.

### Dependency and layering constraints

- **Role lint** (`tasks/lint/crate_dependencies.py:56-93, 352-389`):
  - A technical library depends only on kernel or technical libraries.
  - A domain depends only on kernel or upstream domains.
  - An adapter never depends on another context's adapter, except shared
    `tracker-support`.
  - The lint ignores dev-dependencies.
- **cargo-pup** (`cli/pup.ron`):
  - `work_domain_imports_only_permitted` (151-167) admits std,
    `kernel::Error`, `corpus`, `tracker` and the crate itself. A parser
    import in `work` fails here.
  - `remote_projection_extracts_json_only` (207-221) denies only process and
    HTTP crates.
  - `work_adapters_is_zero_spawn` (362-374) applies to the adapter crate.
- **cargo-deny** (`cli/deny.toml`):
  - The licence allow-list includes MIT and Apache-2.0.
  - `multiple-versions = "deny"`; crates.io is the only source.
  - Adding a crate also regenerates `cli/licence-audit/new-trees.txt`
    (`tests/integration/deny/test_licence_closure.py`) and runs
    `notices:update`.
- **Public API** (`tasks/public_api.py:15-30`) pins `remote-projection`,
  `work` and `tracker`, among others. Parser types must not leak into their
  public surface. `jira-client` and `work-adapters` are exempt.
- **New crate:** if `adf` moves into its own technical library, follow
  "Registering a library crate" (`tasks/README.md:760-824`). Sync the lock
  with `cargo metadata`, never `generate-lockfile`.

## Code References

- `cli/tracker/src/lib.rs:107-135`: `RemoteIssue`, with the not-identity contract.
- `cli/remote-projection/src/lib.rs:97-124`: `project_raw`, both tracker shapes.
- `cli/work/src/sync/classify.rs:66-134`: `local_changed`, `remote_changed` and `classify`.
- `cli/work-adapters/src/sync/fetch.rs:125-138`: empty hash becomes `None`.
- `cli/work-adapters/src/sync/digest.rs:22-94`: split, `local`, `remote_body`.
- `cli/work/src/normalise.rs:10-86`: `trim_lines`, `filter_frontmatter_keys`.
- `cli/work-adapters/src/sync/run.rs:369-386`: `local_title_and_body`, `reconstruct_pulled_content`.
- `cli/work-adapters/src/sync/run.rs:545-563`: `readable_dossier`.
- `cli/work-adapters/src/sync/run.rs:736-757`: `pull_outcome`.
- `cli/work-adapters/src/sync/run.rs:329-367`: `finalise_baseline`.
- `cli/work-adapters/src/sync/run.rs:1163-1192`: preview exit before `ItemApplier`.
- `cli/work-adapters/src/sync/apply.rs:193-313`: push, pull, create-from-remote baselines.
- `cli/work-cli/src/sync_author.rs:211-216`: `split_projected`.
- `cli/work-cli/src/sync.rs:603-617`: `persist_conflict_dossiers`.
- `cli/work/src/section_diff.rs:176-205`: `differing_sections`.
- `cli/work-adapters/src/sync/baseline.rs:17-190`: `Entry`, parse, render.
- `cli/work-adapters/src/sync/created_baseline.rs:17-38`: empty `remote_hash` on unknown read-back.
- `cli/jira-client/src/adf/mod.rs:84-100`: converter pair.
- `cli/tracker-test-support/src/lib.rs:41-76, 522-562`: `RecordingTracker`.
- `skills/work/sync-work-items/SKILL.md:253-256`: dossier field description.

## Architecture Insights

- **Projection is a transport artefact that leaked into the domain.** The
  write side of the port carries title and body separately, while the read
  side merges them. A `RemoteIssue` that exposed `title` and `description`
  separately, or a single `split_projected` in `remote-projection`, would
  remove the position-based stripping from every consumer. Changing
  `RemoteIssue` touches the contract harness and both clients. A shared
  splitter is the smaller change.
- **Placement under ADR-0069.** CommonMark normalisation and equivalence
  belong in a technical library, either `remote-projection` or a new
  Markdown crate. `adf` moves to a technical library so that one can reach
  it. `work` sees equivalence only through a port; `ItemDigests` is the
  existing seam. `work-adapters` wires the implementation.
- **`classify` becomes three-valued per side.** The values are changed,
  unchanged and unknown. Unknown applies to a missing local hash, and to a
  missing remote hash under recovery. The decision then needs one more
  input, whether the bodies are equivalent, before choosing `Synced`,
  `LocallyModified` or `Conflict`. This is a new domain rule, not a digest
  tweak.
- **The recipe marker is a sum type, not a flag.** Legacy entries keep one
  combined local hash; new entries hold two. 0290 replaces the frontmatter
  hash with per-field values, so the dispatch should be extensible.
- **The ordering requirement is real.** All 0291 and 0230 pull paths funnel
  through `reconstruct_pulled_content` or `author_from_remote`. Fixing
  `reconstruct_pulled_content` and centralising `split_projected` covers
  every current and future pull path.

## Historical Context

- `meta/reviews/work/0296-…-review-1.md`: five review passes, ending on
  COMMENT. The text edited in after pass 5 was never re-reviewed:
  - recovery precedence;
  - rewriting every classified entry;
  - the mechanical corpus check;
  - fixture capture;
  - 0230 coordination.

  The reviewer still flags scope bundling, with four shippable pieces.
- `meta/prs/156-description.md`: settles the equivalence definition, the
  split digest, `digest_recipe`, the recovery table and the read-only
  preview. It leaves the parser, `adf` placement and `issueCreate` capture
  open. The rewrite table came from uncommitted dossiers, so fixtures must
  be re-captured.
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`:
  rules 1, 2, 4 and 5 constrain parser and `adf` placement.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`:
  ports and adapters, plus cargo-pup.
- `meta/research/codebase/2026-08-30-0201-in-process-section-diff.md`:
  `differing_sections` design. Its precedent is that a new MIT or Apache
  crate needs only a `[workspace.dependencies]` pin.
- `meta/research/codebase/2026-08-18-0213-conversational-conflict-resolution-flow.md`:
  dossier design. Preview writes dossiers by design, and the
  `sync_run_real_client.rs` seam is the place for real-projection tests.
- `meta/prs/102-description.md`: `last-sync.json` is committed as one line,
  so the migration produces one large diff. It records a push-only run
  refreshing `remote_hash` on unresolved entries; see Open Questions.
- `meta/work/0320-linear-push-corrupts-markdown-content.md`: needs
  equivalence as a callable comparison, not only a digest.
- `meta/work/0290-bidirectional-field-mapping.md`: extends `Entry` and bumps
  the recipe. Per-field values supersede `local_frontmatter_hash`.

## Related Research

- `meta/research/codebase/2026-08-18-0213-conversational-conflict-resolution-flow.md`
- `meta/research/codebase/2026-09-08-0285-targeted-pull-of-remote-only-work-items.md`
- `meta/research/codebase/2026-09-26-0230-tracker-owned-work-item-id-generation.md`
- `meta/research/codebase/2026-08-30-0201-in-process-section-diff.md`
- `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`

## Open Questions

- ❓ **Parser.** `pulldown-cmark` or `comrak`; neither is in the lock. Check
  each against `multiple-versions = "deny"` and the licence closure. Event
  stream access is more direct in `pulldown-cmark`.
- ❓ **`adf` destination.** A new technical-library crate, or a move into
  `remote-projection`. Either way `jira-client` re-exports or depends on it,
  and the oracle fixture corpus moves or is referenced across crates.
- ❓ **Unrenderable remote ADF.** `document_to_markdown` can return
  `RootNotDoc`, `HeadingWithoutLevel` or `ListWithoutContent`. 0296 defines
  only the rejected-local case.
- ❓ **Unresolved-conflict entry bytes.** `finalise_run` blanks `local_hash`.
  The final criterion ("hashes are byte-identical" for an unresolved
  conflict) holds only when `local_hash` was already empty or no entry
  exists. Restate that criterion or exempt the blanking. PR #102's report
  of remote fields refreshed on unresolved entries does not match today's
  code, which changes only `local_hash`; it probably predates the current
  engine. That was not verified against history.
- ❓ **Promotion and `realign`.** Both compare `remote_body` or `local`
  hashes (`promotion.rs:345-346, 706-718`, `realign.rs:100`). 0296 does not
  say whether they adopt the equivalence-normalised recipe or keep raw
  comparison.
- ❓ **`work update` baseline.** `update.rs:249` writes an `Entry` with
  `local_synced_at: 0`. It must adopt the new entry shape, and it is absent
  from 0296's list.
- **Leading blank line in pushes.** `local_title_and_body` sends the body
  with its post-fence blank line. Whether Linear strips it affects the
  "strip the title line by position" rule only if the stored description can
  start with an empty line. This was not checked against a live read-back.

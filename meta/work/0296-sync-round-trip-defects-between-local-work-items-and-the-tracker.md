---
type: "work-item"
id: "0296"
title: "Sync Round-Trip Defects Between Local Work Items and the Tracker"
date: "2026-09-24T22:48:31+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0146"
blocks: ["work-item:0290", "work-item:0320"]
relates_to: ["work-item:0213", "work-item:0230", "work-item:0285", "work-item:0290", "work-item:0291", "work-item:0320", "work-item:0321"]
external_id: "PP-880"
tags: ["sync", "linear", "bug", "baseline", "projection", "digest"]
last_updated: "2026-10-10T12:40:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0296: Sync Round-Trip Defects Between Local Work Items and the Tracker

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

`accelerator work sync` treats the tracker's projected title line (the title
the tracker port prepends to the body) and its markdown re-serialisation as real content: pulls write the title line into
local files, and noise (cosmetic rewrites by the tracker) alone registers as
change and conflict. Together they
make a full bidirectional sync unsafe to run: on 2026-10-10 a preview of this
repo's corpus planned 7 pulls and 25 conflicts, many with no cause beyond
noise or a missing baseline entry. Jira's projection has the same title-line
shape, so the fix covers both trackers. Sync compares bodies by markdown
equivalence, strips the projected title line from pulls and from conflict
dossiers (the per-item conflict files sync writes), baselines equivalent items
left without a baseline, and moves baseline entries to a new digest recipe as
they are rewritten.

## Context

Change detection compares each side only against its own baseline digest; a
missing digest counts as changed. Digests normalise only per-line whitespace.
The Linear port body is deliberately `"{title}\n{description}"`, and the port
contract states that a push followed by a read is not the identity. Linear
re-serialises `description` in thirteen ways, tabulated under Equivalence: ten
are cosmetic and three change meaning. The Jira port body is
`"{summary}\n{description}"` where the description is canonicalised ADF
(Atlassian Document Format) JSON, not markdown.

Developers running `work sync` meet spurious conflicts and corrupted pulls;
agents and skills that anchor on local markdown structure need local files
left alone when nothing really changed.

Neither side has priority for content: a real edit on either side propagates
to the other. The local side owns representation only while the bodies are
equivalent: a sync never rewrites a local file whose body is equivalent to the
remote one, but a pull of a real remote edit replaces the whole body in the
tracker's serialisation, unedited sections included.

`work create --push` once recorded no baseline; it now records one from a
read-back. 0275, 0276, 0293, 0294 and 0302 have no baseline entry and are
permanently conflicted, and are believed to predate that fix (see
Assumptions). Items whose remote body carries a meaning-changing rewrite (at
least 0161, 0203, 0276 and 0293 in the 2026-10-10 dossiers) stay conflicted
until 0320 lands and they are resolved with `--resolve <id>=local`.

## Requirements

**Equivalence**

Equivalence applies to bodies only. Frontmatter is digested on its own. The
local side has changed when its frontmatter hash or its body hash differs
from the baseline.
The projected title line is stripped from the remote body by position. Two
markdown bodies are equivalent when their CommonMark event streams, from a
parser pinned to an exact version, are equal after:

- treating soft line breaks as spaces and merging adjacent text;
- comparing emphasis and strong ranges per character, not per span, and
  ignoring emphasis on code-span characters;
- unwrapping `<…>` link destinations;
- treating a link whose text equals its destination as that text;
- ignoring the bullet-marker character, task-marker case, table delimiter rows
  and list tightness, which is how blank lines between list items surface in
  the event stream.

Fenced code is compared verbatim. Ordered-list start numbers are compared.

Linear's rewrites, from the 2026-10-10 inspection, and how each is treated:

| Rewrite | Local → remote | Treatment |
|---|---|---|
| Escaping | `~550` → `\~550` | cosmetic: parsing |
| Joined continuation lines | wrapped → joined | cosmetic: soft breaks |
| Per-line bold | `**a⏎b**` → `**a**⏎**b**` | cosmetic: per-character |
| Bold hoisted off code | ``**x `c`**`` → ``**x** `c` `` | cosmetic: code-span emphasis |
| Link destinations | `[t](u)` → `[t](<u>)` | cosmetic: unwrapping |
| Autolinked URLs | `u` → `[u](<u>)` | cosmetic: text equals destination |
| Bullet markers | `- a` → `* a` | cosmetic: bullet character |
| Task-marker case | `[x]` → `[X]` | cosmetic: task-marker case |
| Table delimiters | `\|---\|` → `\| -- \|` | cosmetic: delimiter rows |
| Inserted blank lines | `- a⏎- b` → `- a⏎⏎- b` | cosmetic: list tightness |
| Wrapped digit lines | `⏎  0160.` → `160.` list item | meaning: difference |
| Dotted tokens autolinked | `crates.io` → `[crates.io](<http://crates.io>)` | meaning: difference |
| Code-span indentation | `` `a⏎  b` `` → `` `a   b` `` | meaning: difference |

0320 fixes the three meaning-changing rewrites on push.

For Jira, the local body is round-tripped through the local converter
(`markdown_to_document` then `document_to_markdown`), the remote ADF is
rendered with `document_to_markdown`, and the two results are compared as
above. A local body the converter rejects is not equivalent.

Sync compares by equivalence, not bytes. **Noise** means a difference made
only of the cosmetic rewrites in the table.

A **digest recipe** is the normalisation and hashing a baseline digest was
computed with; its marker names that recipe in each baseline entry. An
legacy-recipe entry holds `local_hash` and `remote_hash`; a new-recipe entry
holds `local_frontmatter_hash`, `local_body_hash`, `remote_hash` and the
marker `digest_recipe`, an integer that is `1` for this item's recipe. An empty local hash means an empty `local_hash` on an legacy-recipe
entry, or empty `local_frontmatter_hash` and `local_body_hash` on a
new-recipe one.

**Defect 1 — pulls write the projected title line into the local body**

- Reproduction: a `remotely-modified` item is pulled, or a conflict is resolved
  with `--resolve <id>=remote`.
- Expected: the local body is the remote body without its projected title
  line: Linear's `description`, or `document_to_markdown` of Jira's
  description ADF.
- Actual: the pulled file is built from the full projected body, so a title
  line lands above the H1. The next push sends it back to Linear as description
  content.

**Defect 2 — noise counts as change and conflict**

- Reproduction (noise-only conflict): take a linked item whose local body and
  remote `description` are equivalent but whose baseline is missing or has an
  empty local hash; run `work sync --preview`.
  - Expected: the item is reported `synced`, no dossier is written, and
    `last-sync.json` is unchanged; a following `work sync` records the
    baseline without a local file write.
  - Actual: `unresolved conflict`, with a dossier listing only noise.
- Reproduction (real conflict): on an item with a list in one section, edit a
  different section both locally and in Linear; run `work sync --preview`;
  open `<paths.integrations>/linear/conflicts/<id>.md`.
  - Expected: only the edited section is listed; the remote side carries no
    title line.
  - Actual: the dossier's `(preamble)` section holds the title line, and
    every section containing a list is listed as differing.
- No Jira case has been observed; Jira coverage is preventive, because its
  projection has the same shape.

**Required behaviour**

- The local digest splits into a frontmatter hash and a body hash. The body
  hash and the remote hash digest the equivalence-normalised body, so noise
  never registers as a change on either side. For Jira, the body hash digests
  the round-tripped local body and the remote hash the rendered remote body;
  a local body the converter rejects is digested without the round trip. Each
  digest strips the projected title line, then normalises, then hashes.
- Whenever a side changed and the bodies are equivalent, sync records a
  baseline, leaves the local file untouched, reports no conflict and writes no
  dossier. It sends an update only when the local side changed and the local
  title differs from the remote title, since the title is the only field
  beyond the body that a push carries until 0290; the update sends the local
  title and body. The item reports `locally-modified` when the local
  frontmatter hash changed, otherwise `synced`. Under the legacy recipe, a
  changed local hash counts as a frontmatter change.
- A local change to the frontmatter alone, with a real remote body edit, is
  pulled: the remote body replaces the local body and the local frontmatter
  is kept.
- A pull of a real remote edit, or a `remote` resolution, writes the remote
  body as defined in Defect 1. Jira pulls accept the converter's losses
  (`strike`, `underline`, a list item's later children).
- A conflict dossier strips the projected title line from the remote side and
  omits sections that are equivalent. When the remote title differs from the
  local `title`, the header's `title:` line reads `<local> → <remote>`;
  otherwise it reads the local title alone.
- The `sync-work-items` skill's description of the dossier names the
  title-difference form of the `title:` line.
- The no-title-line pull criteria are permanent regression tests over every
  pull entry point: a `remotely-modified` pull, a `remote` resolution, 0285's
  targeted pull-create (`work sync --target`) and the frontmatter-kept pull.
  The title-line strip lands before, or with, 0291's pull changes, and 0291
  adds its entry points to the same tests.
- `--preview` never writes `last-sync.json`; it reports what a sync would
  record.

**Recovery for the existing corpus**

- An item with an `external_id` but no baseline entry, an empty local hash,
  or an empty `remote_hash` is compared by equivalence: equivalent items are
  baselined without a local file write; others remain conflicts. This covers
  items created before `create --push` recorded baselines, and creates whose
  read-back failed.

| Baseline state | Remote body | Reported | Local file written | Dossier | Entry after |
|---|---|---|---|---|---|
| no entry | equivalent | `synced` | no | no | new recipe, all hashes set |
| no entry | not equivalent | unresolved conflict | no | yes | absent |
| empty local hash | equivalent | `synced` | no | no | new recipe, all hashes set |
| empty local hash | not equivalent | unresolved conflict | no | yes | unchanged |
| empty `remote_hash` | equivalent | `synced` | no | no | new recipe, all hashes set |
| empty `remote_hash` | not equivalent | unresolved conflict | no | yes | local hash emptied |

The not-equivalent rows keep today's handling of an unresolved conflict, which
empties the local hash on an existing entry and creates none. A missing or
empty local hash leaves nothing to compare, so it counts as unknown rather
than changed: an equivalent recovered item reports `synced` and sends no
update.

**Digest migration**

- Each baseline entry carries a digest-recipe marker. An entry without one
  uses the **legacy recipe** (combined frontmatter and body, per-line
  whitespace only) to decide which sides changed; equivalence then applies as
  in Required behaviour, whichever sides changed.
- A run rewrites every entry it classifies, including one unchanged on both
  sides, under the new recipe, computing its hashes from the current local and
  remote bodies with no file write or tracker update. Only entries a run does
  not classify (`remote-absent`, `indeterminate`, an unresolved conflict, or
  any entry under `--preview`) keep the legacy recipe, so one full sync
  migrates every other entry. A change to `remote_updated_at` alone does not
  alter an entry's hashes.
- Under the legacy recipe, noise on one side still classifies as changed. The
  bodies are then equivalent, so the equivalence rule in Required behaviour
  applies. Whether or not an update is sent, the entry is rewritten under the
  new recipe.
- Later recipe bumps, such as 0290's, keep classifying unmarked entries with
  the legacy recipe; the item that introduces a bump defines how entries
  marked with a superseded recipe are classified.
- A parser upgrade bumps the recipe marker. A golden-digest fixture fails
  when a digest changes, or when the parser version changes without a
  recipe-marker bump.

**Out of scope**

- Pulling the title, or syncing any field beyond the title and body (0290).
  Pushes already carry the local title. A remote edit to the title alone no
  longer registers as a body change. Until 0290 lands such an edit goes
  undetected, showing only in the `title:` header of a dossier written for
  some other real conflict, and a later local change that pushes the local
  title overwrites it. That window is accepted.
- Normalising anything beyond the equivalence definition above.
- Fixing Linear's meaning-changing rewrites on push.
- Deciding the sync authority model beyond this item's representation rule.
- Removing the legacy recipe; a follow-up does so once `last-sync.json` holds no
  unmarked entry.

## Acceptance Criteria

Unless a criterion names the recipe, its baseline entry is on the new
recipe. "Unresolved conflict" is the report's `unresolved` line.

- [ ] Given a `remotely-modified` item, when it is pulled, then the local body
      equals the remote body as defined in Defect 1, with no projected title
      line above the H1.
- [ ] Given a `remotely-modified` Jira-linked item whose ADF contains
      `strike` and `underline` text and a list item with a later child, when
      it is pulled, then the local body equals `document_to_markdown` of the
      remote ADF, with the strike, underline and later child dropped and no
      projected title line above the H1.
- [ ] Given a targeted pull-create (`work sync --target <remote-id>`), a
      frontmatter-kept pull and a conflict resolved with
      `--resolve <id>=remote`, each for a Linear-linked and a Jira-linked
      item, when each writes its file, then it contains no projected title
      line.
- [ ] Given a pulled item, when `work sync` runs again with no edits on either
      side, for a Linear-linked and a Jira-linked item, then it reports
      `synced` and sends no update to the tracker.
- [ ] Given a linked item whose remote `description` differs from the remote
      body at the last sync only by one of the ten cosmetic rewrites in the
      Equivalence table, for each rewrite separately, when `work sync` runs,
      then it reports `synced` and its baseline entry's hashes are unchanged.
- [ ] Given each Equivalence-table row, when the body hash and remote hash are
      computed for the row's local and remote forms, then they are equal for
      the ten cosmetic rows and differ for the three meaning-changing rows.
- [ ] Given a baselined item whose remote title changed but whose
      description did not, when `work sync` runs, then it reports `synced`,
      the local file's bytes are unchanged, and no update is sent to the
      tracker.
- [ ] Given a legacy-recipe entry whose remote body changed and is equivalent
      to the local body, when `work sync` runs, then it reports `synced`, a
      new-recipe baseline is recorded and the local file's bytes are
      unchanged.
- [ ] Given a remote `description` that differs from the local body only by
      one of the three meaning-changing rewrites in the Equivalence table, for
      each rewrite separately, when bodies are compared, then they are not
      equivalent.
- [ ] Given a `*`/`-` swap inside a fenced code block, when bodies are
      compared, then they are not equivalent.
- [ ] Given ordered lists that differ only in start number (`1.` locally,
      `3.` remotely), when bodies are compared, then they are not equivalent.
- [ ] Given each of these near-misses, when bodies are compared, then they are
      not equivalent: `**a**` and `a`; `[t](u)` and `[u](u)`; `- a` and
      `1. a`; a backslash hard break and a soft break; `` `a b` `` and
      `` `a c` ``; table alignment `:--` and `--:`; `- [ ] a` and `- [x] a`;
      `# a` and `## a`; `a⏎⏎b` and `a⏎b` outside a list.
- [ ] Given ``**`c`**`` locally and `` `c` `` remotely, when bodies are
      compared, then they are equivalent.
- [ ] Given a remote body whose description is empty, or whose first
      description line equals the title, when the projected title line is
      stripped, then only the first line is removed.
- [ ] Given a new-recipe entry and a local edit made only of one of the ten
      cosmetic rewrites, for each rewrite separately, when `work sync` runs,
      then it reports `synced`, sends no update to the tracker and leaves the
      local file's bytes unchanged.
- [ ] Given a linked item with no baseline entry and an equivalent remote
      body, when `work sync --preview` runs, then it reports `synced` and no
      dossier is written for it.
- [ ] Given each recovery-table row, when `work sync --preview` runs and then
      `work sync` runs, then both report the same state.
- [ ] Given any `work sync --preview` run, including one that reports
      conflicts, then `last-sync.json` is byte-identical before and after.
- [ ] Given both sides changed to equivalent bodies and no frontmatter change,
      when `work sync` runs, then it reports `synced`, a baseline is recorded,
      the local file's bytes are unchanged, and no dossier is written for it.
- [ ] Given a local edit to both frontmatter and body where the body is
      equivalent to a changed remote body, when `work sync` runs, then the
      item reports `locally-modified`, the local file's bytes are unchanged
      and the entry carries the new recipe marker; with matching titles no
      update is sent, and with a changed local title exactly one update,
      carrying the local title and body, is sent.
- [ ] Given a remote title edit and a cosmetic remote body change, when
      `work sync` runs, then it reports `synced`, no update is sent, the
      remote title stands and the local file's bytes are unchanged.
- [ ] Given a local edit to the frontmatter alone (a status change, title
      unchanged) and an unchanged remote, when `work sync` runs, then it
      reports `locally-modified`, records a baseline and sends no update.
- [ ] Given a local edit to the frontmatter alone and a real remote body edit,
      when `work sync` runs, then it reports `remotely-modified`, the remote
      body replaces the local body, the local frontmatter is kept, no
      projected title line is written, and the next sync with no edits reports
      `synced`.
- [ ] Given a Jira-linked item whose local body, round-tripped through
      `markdown_to_document` then `document_to_markdown`, equals
      `document_to_markdown` of the remote ADF, when bodies are compared, then
      they are equivalent; given a local body `markdown_to_document` rejects
      (blockquote, pipe table, nested list), then they are not.
- [ ] Given a Jira-linked item whose remote ADF paragraph text differs from
      the local body, when bodies are compared, then they are not equivalent.
- [ ] Given a baselined Jira-linked item whose local body contains a pipe
      table and no edits on either side, when `work sync` runs, then it
      reports `synced`.
- [ ] Given a Jira-linked item with no baseline entry whose local body
      contains a pipe table, when `work sync` runs, then it reports an
      unresolved conflict.
- [ ] Given a golden-digest fixture holding one body per Equivalence-table
      row plus fenced code and a table, with the recipe marker and pinned
      parser version recorded beside the digests, when it runs, then every
      digest matches; when the parser version resolved in `Cargo.lock`
      differs from the recorded one and the marker does not, then it fails,
      which a deliberately mismatched recorded version demonstrates.
- [ ] Given a real conflict, for a Linear-linked and a Jira-linked item, on
      an item whose untouched section's list differs remotely only by
      cosmetic rewrites (`-` locally, `*` remotely), when
      `work sync --preview` writes its dossier, then only the edited section
      is listed, and the remote side begins with the body's H1 rather than
      the projected title line.
- [ ] Given a real conflict where the remote title differs from the local
      `title`, when the dossier is written, then its header `title:` line
      reads `<local> → <remote>`; when the titles match, it reads the local
      title alone.
- [ ] Given the `sync-work-items` skill, when its dossier description is
      read, then it names the title-difference form of the `title:` line
      alongside the existing render fields.
- [ ] Given an item with an `external_id` in each row of the recovery table,
      when `work sync` runs, then the reported state, local file write,
      dossier and resulting entry match that row, and for each equivalent row
      the next sync reports `synced`.
- [ ] Given a `create --push` whose read-back failed, leaving an empty
      `remote_hash`, when `work sync` runs with no edits on either side, then
      the item reports `synced`, its baseline entry gains a non-empty
      `remote_hash`, the local file's bytes are unchanged, and no update is
      sent to the tracker.
- [ ] Given fixtures for 0275, 0276, 0293, 0294 and 0302 (local file, no
      baseline entry, and the remote description captured with `accelerator
      linear show` and committed before implementation), when sync classifies
      them, then 0275, 0294 and 0302 report `synced`, and 0276 and 0293
      report an unresolved conflict while their remote bodies still carry
      the rewrites 0320 fixes.
- [ ] Given this repo's corpus at merge time, when `work sync --preview` runs,
      then every planned pull is for an item whose bodies are not
      equivalent, and every unresolved conflict's dossier lists at least one
      section or a title difference; the PR records, for each of the 32 items
      pulled or conflicted on 2026-10-10, whether it is now `synced`, still
      differs, or was edited since.
- [ ] Given a baseline entry with no recipe marker, when its first sync runs,
      then an item unchanged on both sides reports `synced` with no file write
      or tracker update and its entry gains the new recipe marker, an item edited only locally reports
      `locally-modified`, an item edited only in the tracker reports
      `remotely-modified`, and an item changed on both sides to equivalent
      bodies is baselined and reports `locally-modified`; an item whose remote
      body changed only by cosmetic rewrites reports `synced` without a local
      file write; every rewritten entry carries the new recipe marker.
- [ ] Given a baseline entry with no recipe marker and a local edit made only
      of cosmetic rewrites, when `work sync` runs, then no update is sent and
      the entry carries the new recipe marker.
- [ ] Given a baseline entry with no recipe marker that the run does not
      rewrite (`remote-absent`: the issue no longer exists remotely;
      `indeterminate`: the remote could not be read; an unresolved conflict;
      `--preview`), when the run ends, then the entry still has no recipe
      marker and its hashes are byte-identical, and a later sync with no edits
      on either side reports `synced`.

## Open Questions

The first three are resolved before work starts.

- Which parser: `pulldown-cmark` or `comrak`? Either is pinned to an exact
  version.
- Does `remote-projection` depend on `jira-client`, or does `adf` move down a
  crate?
- Does a captured `issueCreate` read-back reproduce the Equivalence table?
  If it does not, the table and its criteria are updated before
  implementation.
- Do the corpus items without baselines predate the `create --push` read-back
  fix? The recovery rule covers them either way.

## Dependencies

- Blocked by: none.
- Blocks: 0320 — Linear Push Corrupts Markdown Content; its criteria compare
  by this item's equivalence.
- Blocks: 0290 — it extends the baseline `Entry` and digest recipe this item
  introduces, bumping the recipe marker rather than adding a migration. 0290
  also closes the title-detection gap this item opens.
- Related: 0321 — Sync Authority Model Between Local Work Items and the
  Tracker. This item proceeds without waiting for its ADR, which may ratify
  or revise the representation rule.
- External: Linear's description serialiser. A change to it means revisiting
  the Equivalence table.
- External: the pinned CommonMark parser crate, a new third-party dependency.
- External: Jira's description ADF, read through the `adf` converter pair; a
  change to either means revisiting Jira equivalence.
- Blocks: 0291 — Parent-Child Relationship Synchronisation, for its pull-path
  changes only; the title-line strip lands before, or with, them.
- Related: 0285 — Targeted Pull of Remote-Only Work Items and Resolution
  Normalisation (done); its targeted pull-create is a pull entry point these
  regression tests cover.
- Coordinated: 0230 — Tracker-Owned Work Item ID Generation (in progress)
  adds baseline writes and pull paths (pending-push adoption, promotion,
  remote-only pulls). Whichever lands second adapts: 0230's baseline writes
  use the entry shape current at the time, and its pull paths join the
  no-title-line regression tests.
- Related: 0213 — Conversational Conflict Resolution Flow, which defined the
  conflict dossier this item amends.
- Internal: `split_projected` moves below `work-cli`, and Jira equivalence needs
  `remote-projection` to reach the `adf` converter, by depending on
  `jira-client` or by moving `adf` down a crate.
- Before equivalence work starts: the parser is chosen and clears the repo's
  dependency checks, the `adf` crate placement is decided, and the
  `issueCreate` read-back is captured.
- External: credentialed access to the PP Linear workspace, for the
  read-back capture, the fixture capture and the merge-time preview, whose
  result also depends on live corpus and Linear state.
- Affects: the `sync-work-items` skill, which describes the dossier's fields;
  this item updates it for the title-difference form of the `title:` line.

## Assumptions

- Linear's re-serialisation is server-side, not a client transform; a fixture
  capture of a real `issueCreate` read-back would confirm it. Inspecting the 32
  pulled or conflicted items on 2026-10-10 found the thirteen rewrites in the
  Equivalence table.
- Jira's projection shares the title-line shape, but its description is
  canonicalised ADF JSON, so equivalence round-trips the local body through
  the local converter rather than comparing markdown directly.
- Local work items own their representation only while bodies are
  equivalent; neither side has priority for content.
- The corpus items without baselines predate the `create --push` read-back fix;
  this was not checked against history.

## Technical Notes

- Classification: `local_changed`, `remote_changed` and `classify` in
  `cli/work/src/sync/classify.rs`; empty baseline strings become `None` in
  `cli/work-adapters/src/sync/fetch.rs`.
- Digests: `digest::local` and `digest::remote_body` in
  `cli/work-adapters/src/sync/digest.rs`, built on `trim_lines` in
  `cli/work/src/normalise.rs`, which keeps leading blank lines.
- Projection: `project` and `project_raw` in `cli/remote-projection/src/lib.rs`;
  the not-identity contract is on `RemoteIssue.body` in `cli/tracker/src/lib.rs`.
- Title split: `split_projected` in `cli/work-cli/src/sync_author.rs`, called
  only by `author_from_remote`. The pull path lives in `work-adapters`, so
  sharing it may mean moving it down a crate.
- Pull: `pull_outcome` and `reconstruct_pulled_content` in
  `cli/work-adapters/src/sync/run.rs`; the pull's baseline `remote_hash` is
  computed from the full projected body in `ItemApplier`
  (`cli/work-adapters/src/sync/apply.rs`).
- Dossiers: `readable_dossier` and `render_dossier` in `run.rs`, diffing with
  `differing_sections` in `cli/work/src/section_diff.rs`;
  `persist_conflict_dossiers` in `cli/work-cli/src/sync.rs` writes them in
  both preview and apply modes.
- Baseline writes: `ItemApplier::push` and `ItemApplier::link_and_baseline`
  store a read-back digest; `read_back_of` and `record_baseline_after_create`
  in `cli/work-adapters/src/creation.rs` do so for `create --push`, storing an
  empty `remote_hash` when the read-back fails.
- Unresolved conflicts empty `local_hash` via `finalise_baseline` in `run.rs`
  and `BaselineStore::finalise_run`; 0136, 0158, 0169, 0203, 0215, 0216 and
  0217 are in this state.
- Baseline format: `Entry` in `cli/work-adapters/src/sync/baseline.rs`
  (`last-sync.json`) holds `remote_updated_at`, `remote_hash`, `local_hash` and
  `local_synced_at`, with no version or digest-recipe marker.
- Tests: `RecordingTracker` and the canned fixtures already model the projected
  title line, and `a_create_then_sync_reports_synced` covers create-then-sync.
  Nothing models `*` re-serialisation or checks pulled files for a title line.
- `digest::local` hashes frontmatter and body together; splitting it into
  `local_frontmatter_hash` and `local_body_hash` changes `Entry`, alongside the
  recipe marker.
- Equivalence compares CommonMark event streams (`pulldown-cmark` or `comrak`,
  pinned to an exact version); a parser upgrade bumps the recipe marker.
- Jira: the Jira body is `"{summary}\n{canonicalise(description)}"` in
  `project` (`cli/remote-projection/src/lib.rs`), compact ADF JSON.
  `markdown_to_document` and `document_to_markdown` in
  `cli/jira-client/src/adf/` are the round-trip pair; `remote-projection`
  depends only on `serde_json`, so using them means a dependency on
  `jira-client` or moving `adf` down a crate.
- The `adf` dialect drops `strike`, `underline` and a `listItem`'s later
  children when rendering, and `markdown_to_document` rejects blockquotes,
  pipe tables and nested lists; Jira pulls accept that loss.

## Drafting Notes

- Splitting the pull-corruption defect into its own item was considered and
  rejected; the ordering requirement keeps new pull behaviour from landing
  before the title line is stripped.
- The original `create --push` defect was dropped after verification showed it
  fixed; its legacy is covered by the recovery requirement.
- The dossier hides equivalent sections although a `remote` resolution rewrites
  them in the tracker's serialisation: equivalence, not byte consistency, is
  the standard.
- Line-level normalisation rules were replaced by parsed comparison after the
  corpus inspection: joined continuations, per-line bold and hoisted emphasis
  are inline-structure changes no line rule absorbs.
- Jira was kept in this item rather than split into a sibling; its equivalence
  round-trips through the local converter because the dialect's own losses
  would otherwise register as change.
- Splitting the local digest was chosen over a single hash because a single
  hash cannot tell a frontmatter edit from a body edit, and the migration
  already rewrites every entry.
- The no-write rule on equivalence follows from local work items owning their
  representation: agents and skills anchor on local markdown structure.
- The parser is pinned, with a recipe-marker bump on upgrade, so a parser
  release cannot silently reclassify the corpus.
- The dossier's `<local> → <remote>` title form only displays the difference;
  resolving a title belongs to 0290's field sync. It reuses the existing
  `title:` header line rather than adding a second `title:` key.

## References

- Parent: 0146 — Work Item Synchronisation Enhancements
- Related: 0213 — Conversational Conflict Resolution Flow for Sync
- Related: 0230 — Tracker-Owned Work Item ID Generation
- Related: 0285 — Targeted Pull of Remote-Only Work Items and Resolution
  Normalisation
- Related: 0290 — Bidirectional Field Mapping for Work Item Sync
- Related: 0291 — Parent-Child Relationship Synchronisation for Work Item Sync
- Related: 0320 — Linear Push Corrupts Markdown Content
- Related: 0321 — Sync Authority Model Between Local Work Items and the Tracker

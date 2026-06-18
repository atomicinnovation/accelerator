---
type: codebase-research
id: "2026-06-18-0051-sync-work-items-skill"
title: "Research: Sync Work Items Skill (0051)"
date: "2026-06-18T12:48:49+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0051"
parent: "work-item:0051"
relates_to: ["codebase-research:2026-06-15-0047-core-skills-sync-integration"]
topic: "Implementing /sync-work-items: reconciliation engine, last-sync.json baseline, conflict UX, and the /list-work-items five-state extension"
tags: [research, codebase, work, integrations, sync, jira, last-sync]
revision: "2aec82d6560fe5407629156cde6eb8d99b208b6a"
repository: "ticket-management"
last_updated: "2026-06-18T12:48:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Sync Work Items Skill (0051)

**Date**: 2026-06-18T12:48:49+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 2aec82d6560fe5407629156cde6eb8d99b208b6a
**Branch**: HEAD (jj workspace `ticket-management`)
**Repository**: ticket-management

## Research Question

What does the codebase already provide, and what is net-new, for implementing
`/sync-work-items` (work item 0051) — an on-demand bidirectional sync skill that
reconciles local `meta/work/` items against the active remote tracker, persists a
`last-sync.json` baseline, resolves conflicts with a confirmation prompt, and
extends `/list-work-items` to render the three baseline-dependent sync states
(locally modified / remotely modified / conflict)?

## Summary

The story sits on a deliberately-prepared seam. Story 0047 (plan and ADR-0044,
both accepted/ready) built **exactly** the two extension points 0051 needs and
left the rest for this story:

1. **`external_id`-presence classification** (synced/unsynced) is implemented and
   centralised in one script — `work-item-sync-label.sh` — whose header comment
   *names story 0051* as the consumer that adds the three baseline-dependent
   states. The classifier (`sync_classify`) and the label table
   (`sync_status_label`) are a two-function slot; 0051 adds case arms, and both
   `/list-work-items` render call sites (table column + hierarchy suffix) render
   the new states with **zero call-site edits**.

2. **Create-then-writeback** is implemented in `/create-work-item`: a single
   `[y/N]` fail-safe push prompt, dispatch through the one sanctioned bridge
   (`work-item-create-remote.sh`), and pre-write substitution of the
   remote-returned key into the `external_id` frontmatter line. The conflict
   override UX 0051 must build should mirror this prompt shape verbatim.

The integration read/write APIs 0051 reuses already exist and are
Jira-complete (Linear too): `search-jira-issues` (filter flags →  JQL),
`show-jira-issue` (per-item read, returns `fields.updated`), and
`jira-emit-key.sh` (create → bare validated key). Config reading
(`work.integration`, `work.default_project_code`, the integrations state path) is
done and validated.

**Net-new work** (none of this exists yet, confirmed by pattern search):

- The `last-sync.json` per-item baseline file (path + schema sketched, but **no
  shell helper writes/reads a JSON map keyed by id** under `.accelerator/state/`).
- Normalised-content **hashing** of a work item (`local_hash`) — the sha256 and
  trim primitives exist, but the normalise-then-hash operation is net-new and its
  ignored-field set is unspecified by 0047.
- The **section-by-section diff** (no markdown-diff or section-split helper
  exists today; refine-work-item does section work via string-anchor prose).
- The **reconciliation engine** itself (four modes, conflict detection,
  resumability), and the **per-item remote read** added to `/list-work-items`.

The single most important correction inherited from 0047 research: `last-sync.json`
must be keyed by **`external_id`** (per ADR-0044 and the 0047 plan's References),
NOT the `work_item_id` shown in 0047's relayed schema — though note the **0051
story itself (line 266) says key by the stable local `id`**, with the live file's
`external_id` locating the remote counterpart. This is a genuine open conflict to
resolve in the plan (see Open Questions).

## Detailed Findings

### Component 1 — The `/list-work-items` status-slot seam (the 0051 plug-in point)

`/list-work-items` is strictly read-only (`skills/work/list-work-items/SKILL.md:43-46`,
reinforced `:366-372`) and performs **no remote read today**. Classification is
presence-based and owned entirely by one script.

- **The slot**: `skills/work/scripts/work-item-sync-label.sh:41-63`. Two
  functions form the contract:
  - `sync_classify()` (`:41-50`) — strips surrounding quotes/whitespace from the
    raw `external_id`; non-empty ⇒ `synced`, else `unsynced`.
  - `sync_status_label()` (`:53-63`) — a `case` mapping a status keyword to a
    `"<glyph> <text>"` label: `synced` → `🟢 synced`, `unsynced` → `⚪ unsynced`.
  - A `main()` dispatcher (`:65-97`) exposes `--classify <value>`, `--label
    <status>`, and a default mode that composes both.
- **The seam is documented in-code** at `work-item-sync-label.sh:24-27`: "Story
  0051 extends the classifier and the label table with the baseline-dependent
  states (locally-modified, remotely-modified, conflict) **without changing the
  /list-work-items rendering call site** — add a case arm to sync_status_label
  (and the classifier) and the new state renders." Echoed in markdown at
  `SKILL.md:266-272`.
- **Render call sites** (untouched by 0051): table column `SKILL.md:280-292` (a
  **Sync** column appended only when an integration is configured), hierarchy
  suffix `SKILL.md:315-328`.
- **Integration gate**: read once at `SKILL.md:27` via `config-read-work.sh
  integration`; branch on the *string* (non-empty = configured), NOT the exit
  code, because the reader exits 0 with a blank line when unconfigured
  (`SKILL.md:29-33`).
- **Label+colour distinctness invariant** (the story's "no two states share an
  identical label+colour pairing"): already tested for the two current states at
  `skills/work/scripts/test-work-item-scripts.sh:1146-1161` — they must differ in
  **both** glyph and text. Labels are markdown-native (glyph + text, never ANSI),
  separately tested at `:1163-1173`. 0051 adds three more states ⇒ all pairwise
  combinations must stay distinct in both dimensions.
- **Widening required**: today `--classify` takes only the `external_id` value.
  For the baseline-dependent states, 0051 must widen the classifier's inputs
  (file path / `last-sync.json` baseline / remote state) to compute the
  `(local-changed, remote-changed)` verdict. The render call sites still stay
  untouched.
- **Gotcha**: the `canonical-tree-fence` block is byte-for-byte identical between
  `list-work-items/SKILL.md` and `refine-work-item/SKILL.md` (asserted by
  `scripts/test-hierarchy-format.sh`) and was deliberately kept label-free. 0051
  must NOT inject status labels into the shared fence.

### Component 2 — Create-then-writeback and the confirmation prompt to mirror

`/create-work-item` Step 6 is a push state machine
(`skills/work/create-work-item/SKILL.md:503-580`).

- **The prompt shape to mirror** (`SKILL.md:528-532`):
  ```
  Push to <tracker> now? [y/N]  (y = create the remote issue + save locally;
  anything else = save locally only, unsynced — you can push it later by
  running /create-<tracker>-issue <path>)
  ```
  Single line naming the tracker, bracketed default-No affordance, both outcomes
  stated inline; strict interpretation — exactly `y`/`Y` after trimming accepts,
  anything else declines (`SKILL.md:534-537`). This was copied from the
  enrich-mode `Proceed? (y/n)` gate (`SKILL.md:636-645`). **This is the shape the
  0051 conflict override prompt should reuse** (where the default answer is the
  *remote* version per the story).
- **Push decision seam**: `skills/work/scripts/work-item-push-decide.sh` — pure
  function `(--code, --attempt, --write-failed)` → one keyword (`write-once`,
  `retry`, `local-save`, `loud-terminal`). Terminal codes (71) are never retried
  (a post-send failure could duplicate the issue). This deterministic decision
  table is the model for 0051's per-item push outcome handling.
- **Dispatch bridge**: `skills/work/scripts/work-item-create-remote.sh` —
  `case "$integration"` (`:197-236`) routes `linear`/`jira` to their create
  scripts, `trello`/`github-issues` → not-available (72), unknown → 73. On
  success prints **only** the bare validated remote identifier (`:248`) after a
  YAML-safety check (`_wicr_identifier_safe`, `:67-87`). Has a real `--dry-run`
  preview (`_wicr_dry_run`, `:114-141`) — directly reusable for `--preview`.
- **`external_id` writeback** happens **in memory, pre-write**, in the SKILL
  (outcome table `SKILL.md:567`): substitute the returned identifier into the
  in-memory `external_id` line, then Write once. The single Write is the only
  disk mutation on the success path (`SKILL.md:574-580`).
- **`id` vs `external_id`**: `id` is allocated locally by `work-item-next-number.sh`,
  written as a quoted YAML scalar + the body H1, and is **never** passed to the
  dispatcher (only `--title`, `--body-file`, `--kind` cross the boundary). Only
  the remote→local direction crosses: the tracker key flows into `external_id`.

### Component 3 — Jira read/write APIs the sync engine reuses

- **`search-jira-issues`** (`skills/integrations/jira/scripts/jira-search-flow.sh`)
  — filter flags parsed `:152-242`, mapped to JQL in `jira-jql.sh:265-322`:
  `--project`, `--all-projects`, `--status`, `--label`, `--assignee` (`@me`
  resolved), `--type`, `--component`, `--reporter`, `--parent`, `--watching`,
  `--text`, `--jql`, `--limit`, `--page-token`, `--fields`, `--render-adf`,
  `--quiet`. The story's "accept the same filter flags as the search-* skill"
  maps onto forwarding these verbatim — `jira-search-flow.sh` already resolves the
  default project (`work_resolve_default_project`) and `@me`.
  - **Critical**: the `updated` timestamp is **NOT returned by default** — with no
    `--fields`, Jira returns only `key`/`id`. The sync change-detector MUST pass
    `--fields updated,summary,description` to get `.issues[].fields.updated`
    (ISO-8601), the `remote_updated_at` source.
  - Pagination via `nextPageToken` (`SKILL.md:90-97`).
- **`show-jira-issue`** (`jira-show-flow.sh`) — `GET /rest/api/3/issue/<key>`,
  **defaults to `*all` fields** (`:115-121`) so a bare call returns `fields.updated`
  + `fields.summary` + `fields.description` without naming them — the natural
  per-item remote read for `/list-work-items`'s baseline-dependent states and for
  conflict-time body fetch. Note `--render-adf` defaults **ON** here (asymmetric
  vs search); pass `--no-render-adf` to compare raw ADF bodies.
- **Create + key emission**: `jira-emit-key.sh` runs `jira-create-flow.sh`, then
  extracts and validates `.key` against `^[A-Z][A-Z0-9]+-[0-9]+$`, printing **only
  the bare key** (`:43-50`) or exit 16 (`E_REQ_BAD_RESPONSE`). The work→integration
  bridge already wires this. A pre-create guard (`jira-resolve-fields.sh` exit 109
  `E_RESOLVE_ALREADY_SYNCED`) stops a create when `external_id` is non-empty —
  relevant for sync idempotency (don't re-create an item that's already synced).
- **State directory**: `jira_state_dir()` (`jira-common.sh:69-87`) →
  `<root>/.accelerator/state/integrations/jira/` (default from
  `config-defaults.sh:39,59`, override via `paths.integrations`); writes via
  `jira_atomic_write_json` (`:98-113`). `site.json`/`fields.json` already live
  here. **Decision point**: `JIRA_INNER_GITIGNORE_RULES` (`:53-57`) currently
  excludes only `site.json`/`.refresh-meta.json`/`.lock/` from commit — so a
  `last-sync.json` would be committable by default; decide whether it should be
  gitignored (and keep it byte-equal to the copy in migration
  `0003-relocate-accelerator-state.sh`, pinned by `test-jira-paths.sh`).

### Component 4 — Config reading

- **`work.integration`**: `config-read-work.sh:24-58`. Default empty; validated
  against `WORK_INTEGRATION_VALUES` = {`jira`, `linear`, `trello`,
  `github-issues`} (`config-defaults.sh:91-96`) only when non-empty (out-of-set ⇒
  `log_die`). **When unset it exits 0 with a blank line** — so the unconfigured
  error AC ("exit with a clear error") must be raised by the sync skill itself,
  not relied upon from the reader.
- **`work.default_project_code`**: `config-read-work.sh default_project_code`
  (default empty). Convenience helper `work_resolve_default_project()`
  (`scripts/work-common.sh:17-26`) warns (non-fatal) when integration is set but
  the project code is empty.
- **Integrations state path**: `config-read-path.sh integrations` →
  `.accelerator/state/integrations` (default). **No helper appends the `<system>/`
  segment** — the sync skill must assemble `<integrations-path>/<work.integration>/last-sync.json`
  itself.
- **No integration-name → skill-family dispatch helper exists** in shared
  `scripts/`. The only dispatcher is the ad-hoc `case "$integration"` inside
  `work-item-create-remote.sh`. 0051 will either replicate that case (likely
  extending the bridge to cover read/search/show) or map `work.integration` to
  the `<system>` path segment and invoke the per-system skill families by name.
- **Live config in this repo has NO `work:` section** (`.accelerator/config.md`
  only has `visualiser:`), so in this checkout the integration reads as empty
  (unconfigured). Tests will need a `work.integration: jira` fixture.

### Component 5 — Reusable patterns and the net-new gaps

**Exist and reusable:**
- **Batch accept-all UX**: `skills/work/extract-work-items/SKILL.md:213-218` +
  `:317-321` ("accept remaining as-is" fast-path; only touches unreviewed
  candidates, never resurrects skipped ones) — the closest model for the
  unsynced-items batch push offer.
- **Per-item y/n grammar with re-prompt-once + default-decline**:
  `skills/work/update-work-item/SKILL.md:220-226`.
- **Dry-run / preview**: the `--dry-run` in `work-item-create-remote.sh:114-141`
  and the migrate `--preview` banner (`skills/config/migrate/SKILL.md:11,35`).
- **Atomic write toolkit**: `scripts/atomic-common.sh` — `atomic_write`
  (`:16-32`, same-dir temp + `mv` + EXIT-trap), `atomic_jsonl_append` with a
  mkdir-based portable lock (`:177-210`), `atomic_jsonl_remove_by_key`
  (`:212-247`).
- **jq JSON building**: `--argjson` accumulator-merge idiom
  (`linear-graphql.sh:373`, `linear-update-flow.sh:151-162`); atomic jq-built
  state file template at `launcher-helpers.sh:100-106`.
- **sha256 helper (portable)**: `sha256_of()` at
  `skills/visualisation/visualise/scripts/launcher-helpers.sh:10-16`
  (`sha256sum` || `shasum -a 256`) — the only shell sha256 helper in the repo.
- **Frontmatter/body split**: `config_extract_frontmatter` / `config_extract_body`
  (`scripts/config-common.sh:74-101`); section work by string-anchor in
  `refine-work-item/SKILL.md:198-224`.
- **ISO8601 generation (bash 3.2 / macOS safe)**:
  `scripts/artifact-derive-metadata.sh:5-6` — `date -u +%Y-%m-%dT%H:%M:%S+00:00`
  (no GNU-only flags). Remote timestamps come back as ISO strings, so compare
  lexicographically rather than re-parsing.

**Net-new (confirmed absent):**
- No shell helper writes/reads a **JSON map keyed by id** under
  `.accelerator/state/` — the migrations ledgers are newline-delimited ID lists,
  not JSON maps. `last-sync.json` is net-new (compose from `atomic_write` + jq).
- No code **hashes normalised work-item content** or strips trailing whitespace
  for comparison. The primitives exist; the normalise-then-hash op is net-new.
- No **markdown-diff** or **named-section-split** helper (Summary / Context /
  Requirements / Acceptance Criteria). The section-by-section diff is net-new.
- No **N-retry-in-memory loop**; the closest "hold partial decisions across
  attempts" precedent is the migrate interactive resume-state file
  (`skills/config/migrate/scripts/interactive-lib.sh:300-306`). The house
  resumability pattern is build-state-then-commit-ledger-last
  (`run-migrations.sh:293-294`) + loud-fail idempotency guard.

## Code References

- `skills/work/scripts/work-item-sync-label.sh:24-27,41-63` — the 0051 extension
  seam (classifier + label table); in-code mention of story 0051.
- `skills/work/list-work-items/SKILL.md:27-33,266-272,280-292,315-328` —
  integration gate, seam declaration, table + hierarchy render call sites.
- `skills/work/scripts/test-work-item-scripts.sh:1146-1173` — label distinctness
  + markdown-native (no-ANSI) tests the five states must satisfy.
- `skills/work/create-work-item/SKILL.md:503-580,528-537,636-645` — push state
  machine, the `[y/N]` prompt to mirror, enrich-mode gate it copied.
- `skills/work/scripts/work-item-push-decide.sh` — deterministic
  code/attempt/write-failed → keyword decision table.
- `skills/work/scripts/work-item-create-remote.sh:67-87,114-141,197-248` —
  the bridge: dry-run preview, integration case-dispatch, identifier safety, bare
  key emission.
- `skills/integrations/jira/scripts/jira-search-flow.sh:152-242,285-304` +
  `jira-jql.sh:265-322` — filter flags → JQL; `--fields` needed for `updated`.
- `skills/integrations/jira/scripts/jira-show-flow.sh:115-161` — per-item read,
  `*all` fields incl. `fields.updated`.
- `skills/integrations/jira/scripts/jira-emit-key.sh:23,43-50` — create → bare
  validated key.
- `skills/integrations/jira/scripts/jira-common.sh:53-57,69-87,98-113` — state
  dir resolution, atomic JSON write, gitignore rules.
- `scripts/config-read-work.sh:24-58` — `work.integration` validation + empty
  default; `scripts/work-common.sh:17-26` — default-project resolution.
- `scripts/config-read-path.sh:31-75` + `config-defaults.sh:39,59` —
  integrations path (`.accelerator/state/integrations`).
- `scripts/atomic-common.sh:16-32,177-247` — atomic write + JSONL helpers.
- `skills/visualisation/visualise/scripts/launcher-helpers.sh:10-16,100-106` —
  portable sha256; atomic jq-built JSON state file template.
- `scripts/artifact-derive-metadata.sh:5-6` — bash-3.2-safe ISO8601 generation.

## Architecture Insights

- **Single source of truth for state derivation.** The story's insistence that
  `/sync-work-items` and `/list-work-items` share one local-vs-remote-vs-baseline
  comparison maps cleanly onto the existing pattern: classification lives in one
  script (`work-item-sync-label.sh`) and both render surfaces consult it. 0051
  extends that script rather than duplicating logic — and should similarly put the
  baseline-comparison engine in a shared script both the sync skill and the
  list-extension call.
- **Bridge-script dispatch, not in-skill branching.** The codebase routes
  integration-specific work through one bridge (`work-item-create-remote.sh`) whose
  caller passes the config-resolved `--integration` so "gate and route cannot
  diverge". 0051's read/search/show dispatch should follow the same shape —
  ideally extending the bridge to a read side — so the sync skill never re-derives
  the active integration.
- **Presence vs content-parity are orthogonal signals** (ADR-0044): `external_id`
  answers "exists remotely?"; `last-sync.json` answers "changed since last sync?".
  Keep them separate; the synced/unsynced labels never need the baseline, the
  three new states always do.
- **Crash-safe ordering**: the house pattern is commit the local/remote write
  first, update the per-item baseline entry **last**, so an interrupted run leaves
  unprocessed items in their pre-sync state and `last-sync.json` reflects only
  reconciled items (the resumability AC). Mirror `run-migrations.sh`'s
  build-then-commit-ledger-last.
- **Two-way normalised equality, not three-way merge** (story Assumptions): the
  `local_hash` is a digest of normalised local content used purely as an equality
  check against the baseline — explicitly *not* a SHA-based three-way merge.

## Historical Context

- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md:78-103` —
  the load-bearing convention: `id` always-local, `external_id` is the remote key
  and the per-item mapping, synced = `external_id` present. Carves out
  `last-sync.json` content-parity baseline as 0051's separate concern.
- `meta/plans/2026-06-15-0047-core-skills-sync-integration.md:36-40,165-169,588-630,931-933`
  — presence-based (not format-based) classification; the `status → {label,
  glyph}` lookup seam; the canonical-fence byte-equality gotcha; and the
  References note that 0051's `last-sync.json` "will key by `external_id`".
- `meta/research/codebase/2026-06-15-0047-core-skills-sync-integration.md:254-267,373-376`
  — the "0051 seam" section: anticipated `last-sync.json` path/schema, the 2×2
  state derivation, and Open Question 6 flagging 0047's mis-cited legacy
  `meta/integrations/` path (correct path is `.accelerator/state/integrations`).
- `meta/research/codebase/2026-06-14-0048-linear-integration-apis.md` — Linear
  create-then-writeback to `external_id`; mirror for the non-Jira sync path.
- `meta/work/0045-work-management-integration.md` (parent epic);
  `0046` (config gate), `0047` (this seam), `0048`/`0049`/`0050` (other
  integrations broadening sync coverage but not blockers).

## Related Research

- `meta/research/codebase/2026-06-15-0047-core-skills-sync-integration.md` — the
  direct predecessor; defines everything 0051 builds on.
- `meta/research/codebase/2026-06-14-0048-linear-integration-apis.md` — Linear API
  shapes for the non-Jira reconciliation path.
- `meta/research/codebase/2026-05-08-0046-work-management-system-configuration.md`
  — the `work.integration` config gate.

## Open Questions

1. **`last-sync.json` key: `id` vs `external_id`?** A genuine conflict in the
   source docs. The 0051 story (`meta/work/0051-sync-work-items-skill.md:266`,
   Technical Notes) says **key by the stable local `id`**, with the live file's
   `external_id` locating the remote counterpart. The 0047 plan References
   (`:931-933`) and ADR-0044 imply keying by `external_id`. The story's reasoning
   (local `id` is always present and stable on disk; `external_id` may be absent
   for never-pushed items) is sound — but unsynced items have no baseline entry
   anyway, so both schemes only ever key *synced* items. Resolve explicitly in the
   plan; the story's `id`-keyed choice looks correct and should likely win.
2. **`local_hash` normalisation set.** The story fixes a *minimum* (trim per-line
   leading/trailing whitespace + trailing newlines; ignore remote-managed fields
   like `updated_at` and any field absent from the local schema) but leaves
   per-integration extensions to the plan. Which exact frontmatter fields are
   excluded (`last_updated`, `last_updated_by`, `revision`? `external_id` itself?)
   needs nailing down — and the normaliser must be deterministic and shared with
   the `/list-work-items` extension.
3. **Per-item commit semantics for resumability** — exact ordering of local
   write, remote write, and `last-sync.json` update per item (story Open
   Questions); the migrate resume-state file is the closest precedent.
4. **Section-by-section diff implementation** — net-new; need a named-section
   splitter (frontmatter / Summary / Context / Requirements / Acceptance Criteria)
   and a per-section textual diff. Decide script vs in-SKILL prose.
5. **Should the bridge be extended to a read side?** 0051 needs search/show
   dispatch across integrations; either extend `work-item-create-remote.sh` (rename
   to a general bridge) or add a parallel read bridge. Affects how cleanly the
   filter-flag passthrough and per-item remote read are shared.
6. **`last-sync.json` gitignore decision** — commit it (shareable baseline across
   a team) or gitignore it (per-machine)? Affects `JIRA_INNER_GITIGNORE_RULES` and
   the migration-copy byte-equality test.
7. **`--all` + filter composition** — `--all` bypasses only the
   `work.default_project_code` scope while user filters still apply; confirm
   `jira-search-flow.sh`'s `--all-projects` (which omits the project clause) is the
   right primitive to reuse.

---
type: plan
id: "2026-06-18-0051-sync-work-items-skill"
title: "Sync Work Items Skill Implementation Plan"
date: "2026-06-18T12:58:12+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0051"
parent: "work-item:0051"
tags: [work-management, integrations, sync]
derived_from: ["codebase-research:2026-06-18-0051-sync-work-items-skill"]
revision: "73cdbddec9bc53b0c84a1b780b3e143aa78ca773"
repository: "ticket-management"
last_updated: "2026-06-19T00:21:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Sync Work Items Skill Implementation Plan

## Overview

Implement `/sync-work-items` — an on-demand skill that reconciles local work
items in `meta/work/` against the remote tracker named by `work.integration`,
persisting a `last-sync.json` content-parity baseline. The skill supports four
modes (bidirectional default, `--push-only`, `--pull-only`, `--preview`),
resolves conflicts with a section-by-section diff and a remote-default override
prompt, offers to push never-pushed items, and pulls untracked remote issues.
Because this skill produces the `last-sync.json` baseline, it also completes the
`/list-work-items` five-state sync display by adding the three baseline-dependent
states (locally modified, remotely modified, conflict) on top of the
synced/unsynced subset that 0047 already ships.

The work is decomposed into eight independently-mergeable phases. Phases 1–4 are
pure, unit-tested shell scripts (no skill wiring). Phase 5 ships the
`/list-work-items` half. Phases 6–8 build the `/sync-work-items` skill in
shippable increments. Every phase leaves `mise run` green and the repo in a
coherent, releasable state.

## Current State Analysis

Story 0047 (plan + ADR-0044, both accepted) built the two seams 0051 plugs into
and left the rest:

- **The classifier/label seam** — `skills/work/scripts/work-item-sync-label.sh:41-63`
  is a two-function slot. `sync_classify()` (`:41-50`) maps a raw `external_id`
  to `synced`/`unsynced`; `sync_status_label()` (`:53-63`) maps a status keyword
  to a `"<glyph> <text>"` markdown-native label. The header comment (`:24-27`)
  names story 0051 as the consumer that adds the baseline-dependent states "by
  adding a case arm … without changing the /list-work-items rendering call site".
- **`/list-work-items`** is strictly read-only and performs **no remote read
  today** (`skills/work/list-work-items/SKILL.md:43-46`). The integration gate is
  read once at `:27` (`config-read-work.sh integration`) and branched on the
  **string**, not the exit code (`:29-33`). Render call sites — the **Sync**
  table column (`:280-292`) and the hierarchy suffix (`:315-328`) — call
  `work-item-sync-label.sh` and need no edits for new label vocabulary. The
  `canonical-tree-fence` block (`:305-310`) is byte-pinned identical to
  `refine-work-item/SKILL.md` by `test-hierarchy-format.sh` and must stay
  label-free.
- **The create bridge** — `skills/work/scripts/work-item-create-remote.sh` routes
  a create to the active tracker, returns a bare validated identifier, applies a
  YAML-safety check (`_wicr_identifier_safe`, `:67-87`), and already implements a
  real `--dry-run` preview (`:114-141`). The push UX it serves
  (`create-work-item/SKILL.md:503-580`, prompt `:528-532`) plus the deterministic
  `work-item-push-decide.sh` decision table are the shapes 0051 mirrors.
- **Jira read/write APIs** exist and are complete: `jira-search-flow.sh`
  (filter flags → JQL; **`updated` is not returned without `--fields`**),
  `jira-show-flow.sh` (defaults to `*all` fields, returns `fields.updated`),
  `jira-emit-key.sh` (create → bare validated key).
- **State dir**: `jira_state_dir()` (`jira-common.sh:69-87`) →
  `.accelerator/state/integrations/jira/`. `JIRA_INNER_GITIGNORE_RULES`
  (`:53-57`) excludes only `site.json`/`.refresh-meta.json`/`.lock/` — so
  `last-sync.json` is committable by default, which is the chosen behaviour (see
  Decisions Locked). That array is byte-pinned to a copy in
  `0003-relocate-accelerator-state.sh` by `test-jira-paths.sh`.
- **Linear read/write APIs** exist and are complete (0048), and are the second
  tracker this story supports: `linear-search-flow.sh` (filters
  state/assignee/label/text, **single-team** — the team is catalogue-fixed, there
  is no key-set filter and no cross-team `--all`; **auto-paginates internally**;
  **`updatedAt` is NOT in the selection today** — a required net-new GraphQL field
  add, see Decision #10), `linear-show-flow.sh` (by identifier; **Markdown-native
  bodies, no ADF render**), `linear-create-flow.sh` (no-file mode prints the bare
  validated identifier and distinguishes pre-create/retryable from
  post-create/not-safe-to-retry — parallel to `jira-emit-key.sh`),
  `linear-update-flow.sh` (`--title`/`--description` inline, `--print-payload`
  dry-run). Native exit codes differ (search 70–73, show 80–82, update 110–114).
- **Linear state dir**: `linear_state_dir()` (`linear-common.sh:67-90`) →
  `.accelerator/state/integrations/linear/`. `LINEAR_INNER_GITIGNORE_RULES`
  (`:52-56`) excludes only `viewer.json`/`.refresh-meta.json`/`.lock/` (and
  `catalogue.json` is deliberately committed) — so `last-sync.json` is committable
  by default here too. Unlike the Jira array this one is **not** pinned to a
  migration-script copy (the Linear state path is net-new — no migration writes
  it); `test-linear-paths.sh` asserts the rules directly. The baseline-path
  assembly (`<integrations>/<system>/last-sync.json`) is already tracker-generic.

**Confirmed net-new** (none exist today): a JSON map keyed by `id` under
`.accelerator/state/`; normalise-then-hash of a work item; a markdown
section-splitter / section diff; the read-side dispatch bridge **and the
write/update dispatch bridge** (the create bridge only creates — the tracker
update *flows* exist but no work→integrations update bridge wraps them); the
(mode, state) decision script and the fault-injectable apply helper; the
reconciliation engine; the bulk remote read feeding both `/list-work-items` and
sync; and **one change inside the Linear integration itself — adding `updatedAt`
to the Linear search/show GraphQL selection** (every other Linear difference is
absorbed by the bridge adapters, not the integration); and a small
**consolidation** of the **full-digest** portable sha256 idiom into a
general-purpose `scripts/hash-common.sh`. Today the only full-digest copy is the
visualiser's `sha256_of` (`launcher-helpers.sh:10-16`); the design playwright
`run.sh:30` / `ensure-playwright.sh:51` carry a *different*, **8-char-truncating**
`sha256_of` (a cache-namespace key, `cut -c1-8`), so they are not the same idiom and
are out of the full-digest consolidation's default scope (see Phase 2). Primitives
that genuinely exist and are reused as-is: `atomic_write` (`atomic-common.sh:16-32`),
`config_extract_frontmatter`/`config_extract_body` (`config-common.sh:74-101`),
and bash-3.2-safe ISO8601 (`artifact-derive-metadata.sh:5-6`). The full-digest sha256
idiom is **not** reused in place from the launcher-scoped copy — it is relocated to
the shared utility first (so reuse is genuine, not a second copy).

## Desired End State

- `/sync-work-items` exists and is registered in `.claude-plugin/plugin.json`. It
  errors cleanly when `work.integration` is unset; otherwise it reconciles synced
  items, offers to push unsynced items, pulls untracked remote issues, resolves
  conflicts interactively (bidirectional) or reports-and-skips them (directional),
  and persists `last-sync.json` crash-safely (skipped under `--preview`).
- `/list-work-items` renders all five sync states when a baseline exists and an
  integration is configured, degrading to synced/unsynced when the remote is
  unreachable or no baseline exists — never failing or hanging.
- All five label states are pairwise distinct in **both** glyph and text, are
  markdown-native (no ANSI), and the `canonical-tree-fence` stays label-free.
- `mise run` is green; the new scripts are covered by unit tests; `last-sync.json`
  round-trips through write/read/resume.

### Key Discoveries

- The classifier seam is a two-function slot explicitly built for this story
  (`work-item-sync-label.sh:24-27,41-63`).
- `/list-work-items` render sites need **zero edits** for new labels, but
  classification input must widen from `external_id`-only to
  (file, baseline, remote) — done in a *new* engine script, not in the render
  path (`SKILL.md:280-292,315-328`).
- Jira `search` omits `updated` unless `--fields updated,…` is passed
  (`jira-search-flow.sh` / `jira-jql.sh:265-322`); `show` returns it by default
  (`jira-show-flow.sh:115-121`).
- House resumability pattern: commit the side-effect first, update the per-item
  baseline entry **last**; advance the global pre-filter timestamp only on clean
  completion (`run-migrations.sh:293-294`).
- The bridge pattern: integration-specific work routes through one dispatcher
  whose caller passes the config-resolved `--integration` so gate and route
  cannot diverge (`work-item-create-remote.sh:23-25`).

## Decisions Locked (resolving the research Open Questions)

1. **`last-sync.json` key** → the stable local **`id`** (story line 266 is
   authoritative; research concurs). The live file's `external_id` locates the
   remote counterpart. Only synced items get baseline entries.
2. **`last-sync.json` VCS status** → **committed** (team-shared baseline). No
   change to `JIRA_INNER_GITIGNORE_RULES`. Correctness across machines/teammates
   rests on the **authoritative hashes** (`local_hash`, `remote_hash`) being
   deterministic digests of *normalised* content (portable across machines, fixed
   `LANG=C`, canonicalised remote payload), **not** on the mtime pre-filter. The
   mtime/`timestamp` gate is **advisory**: it may only short-circuit to
   *unchanged*, and the engine falls through to the hash on any uncertainty
   (clock skew, mtime-preserving checkout, cross-platform `stat`/`date`). So the
   earlier "fresh-checkout mtime is always newer" reasoning is no longer
   load-bearing, and the teammate-baseline hazard is self-correcting: a stale
   committed `remote_updated_at`/`local_hash` written by another machine merely
   triggers the authoritative hash comparison on the next sync rather than a wrong
   verdict. (If a future story makes the multi-writer staleness unacceptable,
   gitignoring the file is the escape hatch — see Migration Notes.)
3. **Normalisation ignored-field set** (fixed contract, ≥ the story minimum):
   ignore frontmatter fields `last_updated`, `last_updated_by`, `id`,
   `external_id`, `revision` (ignored unconditionally — it is provenance/VCS
   metadata, not authored content, so dropping it is safe whether or not tooling
   stamps it; this also removes a latent footgun if tooling ever does), and
   remote-managed/absent-from-local-schema fields (`updated_at` and any field with
   no local-schema analog), plus per-line leading/trailing whitespace and trailing
   newlines. The pass runs under `LANG=C`/`LC_ALL=C` for byte-stable
   cross-machine output. The "absent from local schema" rule is evaluated on the
   **projected** form of a remote payload (remote summary→title, description→body,
   canonicalised), so local and remote are judged identically. A bare re-save that
   only restamps `last_updated` (or `revision`) is therefore **not** a local
   change.
4. **Resumability ordering** → per item: side-effect (remote/local write) →
   then update that id's baseline entry. Global `timestamp` advanced only on
   clean completion. Re-run is idempotent: reconciled items match their baseline;
   untracked remote issues already created locally now carry an `external_id` so
   are no longer untracked.
5. **Read dispatch shape** → a **new parallel read bridge**
   `work-item-fetch-remote.sh` mirroring `work-item-create-remote.sh`, with
   `search` and `show` subcommands. Create and read stay separate scripts.
6. **Section diff** → a net-new tested script `work-item-section-diff.sh`
   (named-section splitter + per-section `diff`), not in-SKILL prose.
7. **`--all`** → reuses `jira-search-flow.sh`'s `--all-projects` primitive to drop
   only the project clause; user filters still apply.
8. **Remote side gets a real baseline** → each `last-sync.json` entry stores a
   `remote_hash` (sha256 of the normalised, projected remote content at last sync)
   alongside `remote_updated_at` and `local_hash`. This makes the remote-changed
   verdict authoritative and symmetric with the local side, resolving the
   research/review finding that the remote-side comparison had no persisted
   referent (no remote content or hash was stored). The `updated_at` pre-filter
   stays the cheap first gate; `remote_hash` is the authoritative confirm.
9. **Push of a synced item uses a write bridge, not the create bridge** → a new
   `work-item-update-remote.sh` mirrors `work-item-create-remote.sh` and dispatches
   whole-item updates (summary/title + body) to the existing tracker update flows
   (`jira-update-flow.sh`, `linear-update-flow.sh`), which already support
   whole-item replacement and a `--print-payload` dry-run. The create bridge stays
   create-only; create/read/update are three symmetric single-purpose bridges
   sharing one exit taxonomy.
10. **Both Jira and Linear are supported via per-tracker adapters behind the bridge
    boundary** (both are complete integrations). One tracker-agnostic bridge/engine
    contract; the divergences live only in the adapters: (a) **bulk read** — Jira
    fetches the tracked keys with a key-scoped `key in (…)` + `--all-projects`
    chunked/paginated query, Linear (which has no key filter and auto-paginates)
    issues one team-wide `linear-search-flow.sh` and indexes by identifier; (b)
    **body** — Jira ADF (`jq -S` canonicalised), Linear Markdown (no `jq -S`); (c)
    **remote `updated`** — Jira `fields.updated`, Linear `updatedAt`, which is a
    **required GraphQL field addition** to the Linear search/show queries (it is not
    emitted today); (d) **exit codes** — each adapter maps its flow's native codes
    into the shared 70/71/72/73 taxonomy. `LINEAR_INNER_GITIGNORE_RULES` mirrors the
    Jira array, so the committed-`last-sync.json` decision (#2) applies symmetrically
    under `.accelerator/state/integrations/linear/` (its byte-pin in
    `test-linear-paths.sh` is likewise unaffected, since no rule changes). Trello and
    GitHub Issues stay not-available (72) until built.

## What We're NOT Doing

- No SHA-based three-way merge — comparison is two-way normalised-equality
  against the stored baseline (story Assumptions).
- No background/scheduled sync — on-demand only.
- No multi-system mirroring — exactly one active integration per invocation.
- **Jira and Linear are both supported** (both are complete integrations). We are
  **not** adding fetch/sync paths for Trello or GitHub Issues — the read/update
  bridges return "not available" (72) for trackers whose APIs are not built
  (parallel to the create bridge's 72 code), to be wired when those integrations
  land. We are also not changing Linear's *single-team* scoping model (the team is
  fixed by the catalogue at `/init-linear` time; there is no cross-team `--all`
  equivalent — `--all` applies only to Jira's project scope).
- No change to the `canonical-tree-fence` shared example (stays label-free).
- No rich/graphical diff UI — section-grouped textual diff only.
- No edits to the `/list-work-items` render call sites (table/hierarchy) beyond
  feeding them the new label keyword.

## Implementation Approach

State derivation lives in **one** shared engine (`work-item-sync-classify.sh`)
that both `/sync-work-items` and `/list-work-items` call, so the five-state
classification is never duplicated. Integration I/O routes through three symmetric bridges sharing one exit taxonomy
(`work-item-create-remote.sh` for creates, new `work-item-update-remote.sh` for
updates, new `work-item-fetch-remote.sh` for reads) so neither skill re-derives
the active tracker. The (mode, state) → action decision and the per-item commit
sequence are themselves extracted into pure scripts (`work-item-sync-decide.sh`,
`work-item-sync-apply.sh`) so the safety-critical orchestration is CI-testable
rather than living in model-executed SKILL prose. Foundation scripts
(Phases 1–4) are pure and unit-tested first (TDD); the two consuming skills are
then layered on in shippable increments. Each phase is mergeable on its own and
keeps `mise run` green.

**Two trackers, one contract, per-tracker adapters.** This story supports **both
Jira and Linear** — both are complete integrations today. The bridges and engine
are written against a **tracker-agnostic contract**, and the parts that genuinely
differ are isolated into per-tracker **adapters** behind the bridge boundary
(exactly as the create bridge already dispatches `jira`/`linear`). The skills and
the engine never branch on tracker; only the adapter inside a bridge does. The
divergences the Linear adapter must own — none of which leak past the bridge:

- **Bulk read strategy differs.** Jira supports a key-scoped `key in (…)` query, so
  its adapter fetches exactly the tracked keys (chunked, paginated). Linear has **no
  identifier-set filter** (single-team, filters are state/assignee/label/text only)
  and **auto-paginates internally**, so its adapter issues **one team-wide
  `linear-search-flow.sh`** and indexes the result by identifier; the caller then
  selects the tracked subset. Both adapters satisfy the same bridge contract:
  "given the tracked external_ids, return an `external_id → {updated, body}` map"
  (plus the remote-absent / indeterminate markers).
- **Remote `updated` source.** Jira: `fields.updated` (via the injected `--fields`).
  Linear: `updatedAt` — **which `linear-search-flow.sh`/`linear-show-flow.sh` do not
  emit today**, so a prerequisite sub-task adds `updatedAt` to the Linear GraphQL
  selection (`linear-graphql.sh` query + the search/show projections). Lexical
  ISO-8601 comparison holds per-tracker (Jira offset form, Linear `Z` form) because
  a tracker's value is only ever compared against a baseline written from the same
  tracker.
- **Body projection / canonicalisation.** Jira bodies are ADF JSON → project +
  `jq -S` canonicalise before normalising. Linear bodies are **Markdown-native** →
  no ADF, no `jq -S`; the Markdown body is normalised directly (still under
  `LANG=C`). The normaliser's `project(remote)` step is the per-tracker seam.
- **Exit codes.** Each bridge adapter maps its flow's native codes into the shared
  70/71/72/73 dispatch taxonomy: Jira (request-layer 11–34) and Linear (search
  70–73, show 80–82, update 110–114) both collapse to retryable(70)/terminal(71).
- **State dir / gitignore parity.** `linear_state_dir()` and
  `LINEAR_INNER_GITIGNORE_RULES` mirror the Jira ones, so the committed
  `last-sync.json` decision (Decision #2) applies symmetrically under
  `.accelerator/state/integrations/linear/`.

Trello and GitHub Issues remain not-available (bridge code 72) until their
integrations land — the same posture the create bridge already takes.

Tests are added to the existing aggregate suite
`skills/work/scripts/test-work-item-scripts.sh` for work-scoped scripts, and to
`skills/integrations/jira/scripts/test-jira-paths.sh` where the state-dir/path is
involved. New scripts follow the repo's bash-3.2 floor (no associative arrays,
no `${var,,}`), 80-column width, and `set -euo pipefail`.

---

## Phase 1: Read-side and write-side integration bridges

### Overview

Add the two integration bridges that complete the create-bridge family:

- `work-item-fetch-remote.sh` — the **read** counterpart to
  `work-item-create-remote.sh`. It dispatches `search` (filters → remote issue
  list) and `show` (one issue by `external_id`) to the active tracker.
- `work-item-update-remote.sh` — the **write/update** counterpart, dispatching
  an `update` of an already-synced item's whole content (summary + body) to the
  active tracker's update flow. This closes the gap that the create bridge only
  *creates*: the dominant write in bidirectional sync (push of a local-ahead
  synced item) and the conflict-override push both route through it.

Both return a uniform contract so the consuming skills never branch on tracker
output. Both are pure scripts with no skill wiring — fully mergeable alone.

**Bridge skeleton (both)**: reproduce `work-item-create-remote.sh`'s structure
verbatim — all logic inside a `_wifr_main` / `_wiur_main` arg-parse function,
`readonly E_*` exit constants, dispatch inside the function (not bare top-level
`case`/`return`), and the `[ "${BASH_SOURCE[0]}" = "${0}" ]` source-guard. The
code sketches below show the dispatch arm only; the surrounding skeleton matches
the create bridge so the bridges stay structurally identical.

### Changes Required

#### 1. New read bridge script

**File**: `skills/work/scripts/work-item-fetch-remote.sh` (new)
**Changes**: Mirror the create bridge's structure and exit taxonomy.

```bash
# Usage:
#   work-item-fetch-remote.sh --integration <sys> search [filter-flags…]
#   work-item-fetch-remote.sh --integration <sys> show --external-id <key>
#
# search → forwards filter flags to the tracker's search adapter and prints its
#   JSON. jira: ALWAYS injects --fields updated,summary,description so
#   .issues[].fields.updated (remote_updated_at source) is present. linear:
#   linear-search-flow.sh emits its own shape and auto-paginates; it must include
#   updatedAt (GraphQL field add — see below).
# show   → per-item read returning the issue body + updated timestamp.
#          jira:   jira-show-flow.sh <key> --no-render-adf (raw ADF compare).
#          linear: linear-show-flow.sh <identifier> (Markdown-native, no ADF).
#
# Exit taxonomy parallels work-item-create-remote.sh:
#   0 success; 70 retryable (arg/auth/connect); 71 terminal;
#   72 not-available (trello/github-issues read not built); 73 unrecognised <sys>.
case "$integration" in
  jira)
    case "$op" in
      search) "$INTEGRATIONS/jira/scripts/jira-search-flow.sh" \
                --fields updated,summary,description "$@" ;;
      show)   "$INTEGRATIONS/jira/scripts/jira-show-flow.sh" \
                "$external_id" --no-render-adf ;;
    esac ;;
  linear)
    case "$op" in
      # --keys → team-wide fetch (no key filter exists): drop the key list, run
      # one --limit 250 search, mark indeterminate if the result is truncated:true,
      # and let the caller select the tracked subset. Plain search forwards "$@".
      search) "$INTEGRATIONS/linear/scripts/linear-search-flow.sh" \
                ${keys:+--limit 250} "$@" ;;
      show)   "$INTEGRATIONS/linear/scripts/linear-show-flow.sh" \
                "$external_id" ;;
    esac ;;
  trello|github-issues) return $E_DISPATCH_NOT_AVAILABLE ;;
  *) return $E_DISPATCH_UNRECOGNISED ;;
esac    # E_DISPATCH_* sourced from the shared work-item-bridge-codes.sh
```

**Key-scoped read (`search --keys`) — one contract, two adapters.** The consuming
skills need the remote state of a *known set* of tracked issues, not a whole
project. The bridge exposes a single `search --keys k1,k2,…` mode whose **contract**
is tracker-agnostic: *given the tracked external_ids, return the complete
`external_id → {updated, body}` map (with remote-absent / indeterminate markers)*.
The two adapters satisfy it differently because the APIs differ:

- **Jira adapter** — Jira supports a key-set query, so fetch exactly the tracked
  keys: build a `key in (k1,k2,…)` JQL clause (via the `--jql` surface), **chunked**
  to stay within JQL length / `IN`-cardinality limits (≤ 50 keys/request), **paired
  with `--all-projects`** so the key set is the sole filter — essential because
  `jql_compose` otherwise injects a mandatory `project = <default>` clause (and
  errors `E_JQL_NO_PROJECT`/30 with no project flag), which would AND-narrow to the
  default project and drop cross-project tracked keys (false remote-absent). Set
  `--limit 100` so a ≤50-key chunk fits one page (no speculative page-2 probe) and
  still loop `nextPageToken` to exhaustion. Project scoping belongs only to the
  untracked-discovery `search` (Phase 8), never to `--keys`.
- **Linear adapter** — Linear has **no identifier-set filter** and **auto-paginates
  internally**, so the key-scoped approach is impossible *and unnecessary*: issue
  **one** team-wide `linear-search-flow.sh --limit 250` (it follows all pages
  itself; `--limit 250` is the max page size, ~5× fewer round-trips than the
  default 50), build the `identifier → {updated}` map from the result, and the
  bridge selects the tracked subset. Because Linear is single-team
  (catalogue-fixed) there is no cross-team scope and no `--all`. **The Linear map
  carries `updated` only, not the body** — `linear-search-flow.sh` does not select
  `description`; like Jira, the body for the genuinely-changed minority comes from a
  per-item `show` (`remote_hash` is always computed from the `show`-fidelity body).
  **Truncation guard (the load-bearing correctness point):**
  `linear-graphql.sh` caps pagination at `MAX_PAGES=20` and on hitting the cap (or a
  stalled cursor) returns a **partial** node set with `truncated:true` in the body
  **while still exiting 0**. The adapter MUST inspect `truncated` and map an
  incomplete team-wide fetch to **indeterminate** for every un-confirmed tracked
  identifier — **never** remote-absent — so the "absent ⇒ remote-absent" verdict is
  only ever drawn from a *provably complete* (`truncated:false`) fetch.

Both adapters are individually timeout-bounded. For Jira the page/chunk count is
capped; for Linear completeness is determined by the `truncated` flag, not the exit
code. In both, an incomplete fetch yields **indeterminate** for the un-fetched keys
(skip + report needs-retry), **never** silent remote-absent, so the "absent from a
*complete* successful fetch ⇒ remote-absent" invariant holds for both trackers.

**Linear GraphQL prerequisite (net-new, in this phase).** `linear-search-flow.sh`
and `linear-show-flow.sh` do **not** currently select `updatedAt`. Add `updatedAt`
to the issue selection in `linear-graphql.sh` (and surface it through both flows'
projections) — without it the Linear remote pre-filter has no timestamp and the
whole change-detection contract degrades to a body-hash-every-item fetch. Update
`test-linear-search.sh`/`test-linear-show.sh` (and their fixtures) to assert
`updatedAt` is requested and returned. This is the one change to the Linear
integration itself; everything else routes through the bridge adapters.

Plain `search [filter-flags…]` (no `--keys`) keeps forwarding user filters
verbatim for the untracked-pull discovery path (Phase 8), and also paginates to
exhaustion. Reconcile pre-filter cost is per-tracker: **Jira** ≈ `ceil(tracked/50)`
chunked calls; **Linear** ≈ one team-wide search of `ceil(team/250)` internal pages
(≤ `MAX_PAGES`). Either way it is **not** N per-item calls — `show` is reserved for
the genuinely-changed minority on both trackers.

#### 2. New update bridge script

**File**: `skills/work/scripts/work-item-update-remote.sh` (new)
**Changes**: Mirror the create bridge's structure and exit taxonomy. Dispatch a
whole-item `update` (summary/title + body) to the active tracker's update flow,
which already supports whole-item replacement (verified: `jira-update-flow.sh KEY
--summary … --body-file …`; `linear-update-flow.sh ID --title … --description …`;
both expose `--print-payload` as a real dry-run). The bridge accepts a uniform
`--title` + `--body-file` interface and maps per tracker — note Linear's update
flow takes `--description TEXT` inline (no file variant), so the bridge reads the
body file and passes it inline for Linear, matching the create bridge's input
normalisation.

```bash
# Usage:
#   work-item-update-remote.sh --integration <sys> update \
#     --external-id <key> --title <t> --body-file <path>
#
# update → replace the remote issue's summary/title and description/body in one
#   call, then exit 0. Map each tracker's update-flow exit codes (jira 110-117 /
#   linear 110-114, plus propagated transport codes) into the SAME
#   retryable(70)/terminal(71) taxonomy the create bridge defines, so the sync
#   skill never interprets per-tracker update codes. A real dry-run (--dry-run)
#   forwards the tracker's --print-payload and makes no write.
#
# Exit taxonomy (shared with the fetch/create bridges — see note below):
#   0 success; 70 retryable (arg/auth/connect, pre-mutation);
#   71 terminal (at/after mutation — NEVER auto-retried due to RESPONSE
#      UNCERTAINTY: the PUT may have applied but the response was lost, so the run
#      must report rather than guess. NB a whole-item update is idempotent, unlike
#      create, so the hazard is uncertainty, not double-apply); 72 not-available
#      (trello/github-issues update not built); 73 unrecognised <sys>.
case "$integration" in
  jira)   "$INTEGRATIONS/jira/scripts/jira-update-flow.sh" \
            "$external_id" --summary "$title" --body-file "$body_file" \
            ${dry_run:+--print-payload} ;;
  linear) "$INTEGRATIONS/linear/scripts/linear-update-flow.sh" \
            "$external_id" --title "$title" --description "$(cat "$body_file")" \
            ${dry_run:+--print-payload} ;;
  trello|github-issues) return $E_DISPATCH_NOT_AVAILABLE ;;
  *) return $E_DISPATCH_UNRECOGNISED ;;
esac    # E_DISPATCH_* sourced from the shared work-item-bridge-codes.sh
```

**Shared exit taxonomy (single source, not duplicated)**: the fetch, update, and
create bridges use one canonical 70/71/72/73 namespace. Factor the `E_*` constants
into **one sourced definition** (e.g. `work-item-bridge-codes.sh`) sourced by all
three bridges and the decide script, and **retrofit the existing copies to source
it** — both `work-item-create-remote.sh` *and* `work-item-push-decide.sh` currently
hand-declare their own `readonly E_DISPATCH_*` block, so both must be converted so
the taxonomy has exactly one owner (no per-script copy left behind). A unit test
asserts the numeric values. For the **read** bridge, 70/71
collapse to "retryable read failure / degrade" — a read mutates nothing, so the
terminal-may-have-mutated meaning of 71 does not apply; state this in the fetch
bridge header.

#### 3. Tests

**File**: `skills/work/scripts/test-work-item-create-remote.sh` is driven by a
mock HTTP server (`mock-jira-server.py`) with scenario fixtures, and the bridge
invokes integration scripts **by absolute path** — so PATH stubs cannot intercept
them. Mirror that harness rather than PATH-stubbing:
- New `test-work-item-fetch-remote.sh` and `test-work-item-update-remote.sh`,
  driven by the mock server and registered into the work-script suite the same way
  the create-bridge test is. **The mirrored `start_mock` must thread
  `--captured-bodies-file`/`--captured-urls-file`** (as the integration
  `test-jira-search.sh` does — the create-bridge `start_mock` does **not**), or
  every "assert against the captured request" check has nothing to assert against.
  The existing `search-200.json`/`issue-200.json` fixtures carry **no `updated`
  field** (the thing the bridge injection and the remote pre-filter depend on), so
  add **net-new fixtures**: `search-updated-200.json` / `issue-updated-200.json`
  with `fields.updated` in the Jira millisecond+offset form; an **update-204**
  success scenario; a **multi-page** search scenario (a `nextPageToken` then a final
  page); and a net-new **update dropped-response** scenario — a Linear `issueUpdate`
  partial/dropped body (the existing `create-response-dropped-200.json` is
  `issueCreate`, so it does not cover update) plus a Jira PUT 5xx-after-write.
  **Linear coverage uses the Linear mock server** (`mock-linear-server.py`, already
  used by `test-linear-search.sh`/`test-linear-update.sh`): add Linear search /
  show fixtures **carrying `updatedAt`** — a multi-page **complete** (`truncated:false`)
  scenario so the auto-pagination merge is exercised, **and** a multi-page
  **truncated** (`truncated:true`) scenario so the truncation→indeterminate guard is
  exercised — plus an `issueUpdate`-success fixture and the dropped-response above.
  Both bridge tests run their assertions against **both** the Jira and Linear mock
  servers so the adapter dispatch is covered per tracker, not just for Jira.
**Changes** (TDD — write first):
- **fetch**: unrecognised/empty `<sys>` → 73; `trello`/`github-issues` → 72;
  plain `search` forwards arbitrary filter flags unchanged and (jira) injects
  `--fields updated,summary,description` (assert against the captured request);
  `search --keys k1…kN` builds a `key in (…)` clause **paired with `--all-projects`**
  (assert the captured request carries no `project =` clause), **chunks** at the
  key-count cap (assert exactly 1 request at N == chunk size; ≥2 when N exceeds it),
  **paginates to exhaustion** (assert page 2 is fetched when `nextPageToken` is
  present), and returns a **merged result set** (assert N distinct `external_id`s
  recovered across chunks/pages, not just a request count); hitting the page/chunk
  cap yields **indeterminate** for the un-fetched keys, **not** remote-absent;
  `show` requires `--external-id`.
- **fetch (linear adapter)**: `search --keys` issues a **single team-wide**
  `linear-search-flow.sh --limit 250` (no `key in (…)`, no `--all-projects`), relies
  on Linear's internal pagination, and the bridge **indexes the result by identifier
  and selects the tracked subset**. Assert the **merged distinct-identifier count**
  across a multi-page fixture (mirroring the Jira "N distinct" check, so a
  first-page-only indexing bug is caught), that an untracked team issue is excluded,
  and that each entry carries `updatedAt` (guards the GraphQL field add) but **not**
  a body (search omits `description`). A tracked identifier absent from a
  **`truncated:false`** (provably complete) result → **remote-absent**; a
  **`truncated:true`** result → **indeterminate** for un-confirmed identifiers, never
  remote-absent (assert with a truncated multi-page fixture). Linear's native exit
  codes (search 70–73, show 80–82) map into the shared 70/71/72 taxonomy (assert).
- **update**: unrecognised/empty `<sys>` → 73; `trello`/`github-issues` → 72;
  `update` requires `--external-id`/`--title`/`--body-file`; a successful update
  exits 0 and issues a PUT carrying the new summary + body (assert against the
  captured request); a terminal update code is mapped to **71 and not
  auto-retried** — covering **both** the Jira 5xx-after-PUT case **and** the Linear
  dropped-mutation-response case (the GraphQL POST gives no HTTP-status signal, so
  assert the mapping explicitly against the dropped-response fixture); `--dry-run`
  forwards `--print-payload` and the mock records **no** write.

### Success Criteria

#### Automated Verification

- [x] Fetch bridge tests pass: `bash skills/work/scripts/test-work-item-fetch-remote.sh`
- [x] Update bridge tests pass: `bash skills/work/scripts/test-work-item-update-remote.sh`
- [x] Shell lint/format clean: `mise run scripts:check`
- [x] bashisms (3.2 floor) clean: `bash scripts/lint-bashisms.sh skills/work/scripts/work-item-fetch-remote.sh skills/work/scripts/work-item-update-remote.sh`
- [x] Full read-only gate green: `mise run check`

#### Manual Verification

- [ ] With `work.integration: jira` configured, `work-item-fetch-remote.sh
      --integration jira search --label foo` returns JSON whose issues carry a
      `fields.updated` value.
- [ ] `… show --external-id <real-key>` returns that issue's body + updated time.
- [ ] `work-item-update-remote.sh --integration jira update --external-id
      <real-key> --title T --body-file F --dry-run` prints the payload and makes
      no remote write; without `--dry-run` the remote issue's summary + body
      change.

---

## Phase 2: Normalisation + content hashing

### Overview

Add `work-item-normalise.sh`: emit the canonical normalised form of a work item
(for diff/equality) and a `--hash` mode that prints its sha256 digest
(`local_hash`). Pure script, TDD-first, mergeable alone.

### Changes Required

#### 1. New normaliser script

**File**: `skills/work/scripts/work-item-normalise.sh` (new)
**Changes**: Split frontmatter/body via `config_extract_frontmatter` /
`config_extract_body`; drop ignored frontmatter keys; trim per-line
leading/trailing whitespace; strip trailing newlines. The whole pass runs under
`LANG=C`/`LC_ALL=C` so BSD-vs-GNU `awk`/`sed` locale handling cannot change the
normalised bytes across machines (mirrors the Playwright-launcher locale
precedent) — this is load-bearing for the committed cross-machine baseline.

The normaliser does exactly **one** job — emit normalised content — with two
input modes, removing the earlier `--hash`/`--hash-stdin` footgun (two flags that
differed in whether normalisation ran):

```bash
# Usage:
#   work-item-normalise.sh <file>     # normalise a local work-item file
#   work-item-normalise.sh --stdin    # normalise content on stdin (remote bodies)
#
# Ignored frontmatter keys (fixed contract — see plan Decisions Locked #3).
#   `revision` is ignored unconditionally (provenance/VCS metadata, not authored
#   content), so a bare re-save never misclassifies as a local change.
IGNORE_KEYS="last_updated last_updated_by id external_id updated_at revision"
```

Hashing is **not** a normaliser mode. The portable sha256 idiom is
**consolidated into one general-purpose, repo-root shared utility** —
`scripts/hash-common.sh` — rather than reused from a skill-scoped home or
re-copied. Be precise about what is consolidated: the visualiser's
`launcher-helpers.sh:10-16` `sha256_of` emits the **full** digest, and that is the
only existing copy of the *full-digest* idiom — reusing it from work scripts would
be a cross-skill `source`, and adding a `wip_sha256_stdin` to `work-item-common.sh`
would mint a second full-digest copy, so neither is acceptable; hence the shared
utility. The design playwright copies (`run.sh:30`, `ensure-playwright.sh:51`) are
**not** the same function — they truncate to an 8-char cache-namespace key
(`cut -c1-8`), and `run.sh` already sources `launcher-helpers.sh` then *shadows*
`sha256_of` with the truncating form — so they are out of the full-digest
consolidation's direct scope (see below). The shared utility sits alongside the
other repo-root libraries (`scripts/atomic-common.sh`, `config-common.sh`,
`vcs-common.sh`, `work-common.sh`, `log-common.sh`) and exposes both forms:

```bash
# scripts/hash-common.sh — portable SHA-256 (general-purpose, no skill scope)
# Backend chosen by DETECTION (command -v), matching the existing sha256_of —
# not an exit-status `||` fallback — so behaviour is identical, not just digest-equal.
if command -v sha256sum >/dev/null 2>&1; then _HASH_BIN="sha256sum"
else _HASH_BIN="shasum -a 256"; fi
hash_sha256_file()  { $_HASH_BIN "$1" | awk '{print $1}'; }
hash_sha256_stdin() { $_HASH_BIN      | awk '{print $1}'; }
```

The normaliser/baseline callers compose `work-item-normalise.sh <file> |
hash_sha256_stdin`. The visualiser's `launcher-helpers.sh` is updated to **source
`scripts/hash-common.sh` and keep `sha256_of` as a one-line wrapper** over
`hash_sha256_file` (preserving its existing callers — `launch-server.sh:142,157`,
which rely on the **full** digest — and its tests, with zero behavioural change).
The design playwright copies are **deliberately left as-is by default** because
they are a different (truncating) function: consolidating them would mean wrapping
as `hash_sha256_file "$1" | cut -c1-8` (full digest then truncate, preserving the
8-char namespace key), and `run.sh` would also need to stop shadowing the sourced
name. That is an optional follow-on; if not done, it is recorded as a standalone
exception — **the point is we do not pretend they are the same full-digest idiom**.
`sha256_of` (full digest) and the 8-char namespace key are distinct contracts; any
truncating caller composes `| cut -c1-8` at the call site rather than redefining a
same-named function.

**Remote-side projection (used by Phase 4) — per tracker.** `--stdin` normalises a
fetched remote body on the *same* rules as local content. Because a remote payload
has its own field vocabulary, the caller first **projects** it into the comparable
local shape (remote summary/title → the work item's title/H1; remote
description/body → the work-item body). The **canonicalisation** step is
tracker-specific and is the per-tracker seam:
- **Jira** — the body is **ADF JSON**, so canonicalise with `jq -S` (sorted keys)
  before piping to `--stdin`, so key-ordering / whitespace differences in the ADF
  cannot flip the equality check across machines or fetches.
- **Linear** — the body is **Markdown-native** (no ADF, no JSON envelope), so there
  is **no `jq -S` step**; the Markdown body is piped to `--stdin` directly. The
  `LANG=C` per-line trim + trailing-newline rules carry the canonicalisation.

"Ignore any key absent from the local schema" is well-defined for both: it applies
to the projected form, not the raw remote payload. The projection+canonicalisation
that a persisted `remote_hash` is computed from must be the **same** for the value
written and the value later compared (see Phase 4 provenance rule).

#### 2. Shared hashing utility (consolidation)

**File**: `scripts/hash-common.sh` (new, repo-root shared library)
**Changes**: Define the portable SHA-256 idiom **once** as `hash_sha256_file
<file>` and `hash_sha256_stdin` (both `sha256sum || shasum -a 256`, awk-trimmed),
bash-3.2-safe, `set -euo pipefail`-safe. This is a general-purpose utility, not a
launcher or work helper — it sits beside `atomic-common.sh` / `config-common.sh`.

**File**: `skills/visualisation/visualise/scripts/launcher-helpers.sh`
**Changes**: Source `scripts/hash-common.sh` and reduce `sha256_of` to a one-line
wrapper over `hash_sha256_file`, so the visualiser's existing callers and tests are
unchanged but the idiom is no longer *defined* here. (The design playwright
`run.sh` / `ensure-playwright.sh` copies are flagged for the same consolidation, or
an explicit standalone-bootstrap exception — see Current State Analysis.)

**Tests**: assert `hash_sha256_file` and `hash_sha256_stdin` agree on the same
content. **Golden-digest** assertion: the hash of a fixed fixture equals a
hard-coded known SHA-256 constant — this catches per-machine format/trim drift on
whichever backend the host runs, independent of which one is present. **Both
branches in one run**: because the backend is chosen by `command -v` detection,
add a case that **forces the `shasum` fallback on a host that has `sha256sum`**
(shadow `sha256sum` on `PATH`, or inject `_HASH_BIN`) and assert it still yields
the golden digest — otherwise the non-default branch is only ever exercised on the
other OS leg of the CI matrix (or never, for single-OS `scripts:check` tasks).
shellcheck/bashisms clean. The visualiser suite continues to pass with `sha256_of`
re-exported.

#### 3. Tests

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes** (write first): equal content differing only in trailing whitespace /
trailing newlines → identical hash; bumping only `last_updated` /
`last_updated_by` (and `revision`, if stamped) → identical hash; changing
`external_id` or `id` → identical hash (ignored); a real Summary/Requirements
edit → different hash; an unknown extra frontmatter field is ignored.
Determinism: same input twice → same digest, **and** same input under a non-C
locale (`LANG=en_US.UTF-8`) → same digest (guards the committed cross-machine
baseline). Remote projection: a remote body fed through `--stdin` with reordered
ADF/JSON keys (`jq` un-sorted vs `jq -S`) normalises to the same digest.

### Success Criteria

#### Automated Verification

- [x] Normaliser + hash-utility tests pass: `bash skills/work/scripts/test-work-item-scripts.sh`
      (normaliser) and `bash scripts/test-hash-common.sh` (hash utility)
- [x] `sha256_of` still resolves and behaves identically after the launcher-helpers
      retrofit (its existing shell callers/tests pass) — the idiom is re-exported,
      not removed
- [x] `mise run scripts:check` clean
- [x] bashisms clean: `bash scripts/lint-bashisms.sh skills/work/scripts/work-item-normalise.sh scripts/hash-common.sh`
- [x] `mise run check` green

#### Manual Verification

- [ ] Hashing a real `meta/work/*.md` file, then re-saving it untouched, yields
      the same digest; editing its Summary changes the digest.

---

## Phase 3: last-sync.json baseline store

### Overview

Add `work-item-sync-baseline.sh` — read/write helpers for the `last-sync.json`
per-item map keyed by local `id`. Pure script, TDD-first, mergeable alone.

### Changes Required

#### 1. New baseline-store script

**File**: `skills/work/scripts/work-item-sync-baseline.sh` (new)
**Changes**: Resolve the baseline path
`<integrations-path>/<work.integration>/last-sync.json` (assemble the `<system>/`
segment in-script — no shared helper appends it; `config-read-path.sh
integrations` gives the base). Use jq + `atomic_write` (mirroring
`jira_atomic_write_json`) for all mutations.

```bash
# Schema (committed; keyed by local id):
# { "timestamp": <epoch-seconds int>,   # run-START time of the last clean sync;
#                                        # epoch (not ISO) so the local mtime
#                                        # pre-filter is a pure integer compare —
#                                        # no cross-platform date formatting.
#   "items": { "<id>": {
#       "remote_updated_at": "<ISO8601>",                 # remote-side pre-filter
#       "remote_hash": "<sha256 of normalised remote content at last sync>",
#       "local_hash":  "<sha256 of normalised local content at last sync>" } } }
#
# remote_hash makes the remote side authoritative and symmetric with local_hash:
# the engine can decide "remote changed since baseline?" without storing the full
# remote body, exactly as local_hash does for the local side. (Resolves the
# remote-side "baseline-equivalent has no referent" gap — see Decisions Locked.)
#
# Usage:
#   work-item-sync-baseline.sh path                         # print resolved path
#   work-item-sync-baseline.sh get <id>                     # print entry JSON or empty
#   work-item-sync-baseline.sh set <id> <remote_updated_at> <remote_hash> <local_hash>
#   work-item-sync-baseline.sh set-timestamp <epoch-secs>   # global pre-filter ref
#   work-item-sync-baseline.sh remove <id>
```

`set` is the per-item commit primitive (called last per item, per Decision #4);
`set-timestamp` is called once on clean completion. Missing file → treated as
empty baseline (every item re-evaluates authoritatively).

#### 2. Tests

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes** (write first): path assembly inserts the `<system>/` segment under a
fixture `paths.integrations`; `set` then `get` round-trips an entry **including
`remote_hash`**; `set` is idempotent (second identical `set` is a no-op diff);
`remove` deletes one entry leaving others; reading a non-existent file yields
empty, not an error; a **present-but-unparseable / VCS-conflict-markered** file
also yields empty from `get` (exit 0, never an error) — the hard contract that lets
a botched merge degrade to presence-only + full re-hash rather than crashing every
sync on a freshly-merged branch; output is valid JSON (`jq empty`). **Crash-safety
(structural, not inferred from idempotency)**: assert the write goes through
`atomic_write` (same-dir temp + `mv`) — after a simulated interrupted write no
partial temp file survives and the existing file still parses with `jq empty`, so
a mid-write kill never leaves a truncated baseline (the property the resumability
design rests on).

### Success Criteria

#### Automated Verification

- [x] Baseline-store tests pass: `bash skills/work/scripts/test-work-item-scripts.sh`
- [x] `mise run scripts:check` clean; bashisms clean on the new script
- [x] `mise run check` green

#### Manual Verification

- [ ] `work-item-sync-baseline.sh path` under a `jira` fixture prints
      `…/.accelerator/state/integrations/jira/last-sync.json`.
- [ ] A `set` followed by a `get` returns the stored entry.

---

## Phase 4: Change-detection engine + label vocabulary

### Overview

Add the shared classification engine `work-item-sync-classify.sh` implementing
the two-stage change-detection contract, and extend `work-item-sync-label.sh`
with the three baseline-dependent label arms. Both consuming skills call the
engine; the label table stays the single label vocabulary. Mergeable alone — the
engine and labels exist but nothing renders them yet (harmless).

### Changes Required

#### 1. Extend the label table

**File**: `skills/work/scripts/work-item-sync-label.sh`
**Changes**: Add three arms to `sync_status_label()` (`:53-63`), distinct in both
glyph and text from each other and from synced/unsynced:

```bash
case "$status" in
  synced)            printf '🟢 synced' ;;
  unsynced)          printf '⚪ unsynced' ;;
  locally-modified)  printf '🔵 locally modified' ;;
  remotely-modified) printf '🟣 remotely modified' ;;
  conflict)          printf '🔴 conflict' ;;
  *) … ;;
esac
```

#### 2. New change-detection engine

**File**: `skills/work/scripts/work-item-sync-classify.sh` (new)
**Changes**: Given a local file path, its baseline entry (Phase 3), and the
remote state (Phase 1 `show`), compute one of the five status keywords via the
contract in the story Assumptions:

```text
local side:   pre-filter (ADVISORY): file mtime epoch (dual stat: BSD `stat -f %m`
                || GNU `stat -c %Y`) ≤ baseline.timestamp (epoch) ⇒ candidate-unchanged
                — pure integer compare, NO date formatting.
              authoritative: hash(normalise(file)) == local_hash ⇒ unchanged, else changed
remote side:  TRUSTED short-circuit: remote.updated == remote_updated_at ⇒ unchanged
                (ISO lexical; no body fetch, no hash — see asymmetry note below)
              else (updated differs): fetch the full-fidelity `show` body, then
                hash(normalise(project(show-body))) == remote_hash ⇒ unchanged, else changed
verdict:      neither→synced  local→locally-modified  remote→remotely-modified  both→conflict
no baseline entry (or no external_id)  ⇒ presence-only: synced/unsynced
tracked (external_id set) but key ABSENT from a SUCCESSFUL remote fetch
  (deleted / out-of-scope / no longer matching)  ⇒ remote-absent: /list shows
  presence-only; /sync reports + skips (never pushes to a non-existent issue)
failed/timed-out remote read  ⇒ indeterminate: /list degrades to presence-only;
                                /sync skips the item (neither side written)
```

The pre-filter may only short-circuit to *unchanged* (never declare *changed*),
so the cheap check and the authoritative check can never disagree. The local
mtime gate is a **pure epoch-integer comparison** using the repo's verified dual
`stat` idiom — there is **no** `date` formatting anywhere (the global `timestamp`
is stored as epoch seconds), which removes the earlier cross-platform `date`
portability risk entirely. The gate stays **advisory**: on any uncertainty (clock
skew, mtime-preserving checkout tooling) the engine still confirms with the
authoritative hash, so a skewed mtime can only cost an extra hash, never a wrong
verdict. A **missing or non-numeric** `stat` result (both forms failing on an
unexpected platform) is coerced to a sentinel that forces the hash path — it must
never let an empty string reach the `[ "$mtime" -le … ]` arithmetic and abort under
`set -euo pipefail` (mirror the `|| { echo 0; }` guard in the existing
`stat`-using helpers) — the hash, not the timestamp ordering, is what makes a fresh checkout
correct (superseding the old Decision #2 "fresh-checkout mtime is newer"
reasoning). **TOCTOU**: the global `timestamp` persisted on clean completion is the
run's **START** time (captured before any item is read), so a file edited *during*
a run has mtime > timestamp on the next run and is re-hashed rather than wrongly
short-circuited. Remote timestamps are compared lexicographically (ISO-8601). The
engine reuses Phase 2's normaliser for both sides — projecting + canonicalising
the remote payload before `--stdin` — so local and remote are judged identically.

**Remote-side asymmetry (deliberate, documented).** Unlike the local side — where
the mtime gate is advisory and the hash is always authoritative — the remote side
**trusts `updated`-equality**: if `remote.updated == remote_updated_at` the item is
unchanged *without* fetching a body or hashing. This is required because the cheap
bulk `search` carries `updated` for the whole corpus but the authoritative
`remote_hash` is a digest of the **`show`-fidelity** body, which the list path
deliberately does not fetch for unchanged items (avoiding the N+1). The trade is
sound: a remote `updated` that ticks without a content change (e.g. a label or
transition edit) merely forces one `show` + hash and resolves to `synced` — it is
never a false *push*. Two provenance rules make this safe and stable across
machines: (1) any **persisted** `remote_hash` is **always** computed from the
`show`-fidelity body, **never** from `search`'s `description` (which may not be
byte-equivalent), so a baseline written on one path and read on another cannot
spuriously mismatch; (2) `remote_updated_at` is **always** persisted from the same
field the pre-filter reads (the `search` `fields.updated` raw string), so the
lexical equality is byte-for-byte and an identical instant never reads as "changed".

**First-sync-on-dirty completeness.** An item carrying an `external_id` but with
**no baseline entry** is classified via the *full* contract (absent `local_hash` /
`remote_hash` count as *changed* on their side), **not** the presence-only
shortcut — so a first-sync item that is both remote-ahead and locally dirty
surfaces as a conflict rather than being masked as `synced`. The presence-only
shortcut applies only when there is no `external_id` (genuinely never-pushed).

The engine's remote-state **input contract** is pinned so both call sites feed it
identically: the engine accepts a *pre-fetched* remote record (`{updated, body}`
or the remote-absent / indeterminate marker) for an item; the **caller** owns the
bulk-vs-`show` orchestration (Phase 5/6 build the `external_id → record` map from
the key-scoped paginated `search` and decide when to fan out a `show`). The engine
never fetches; it classifies from what it is handed.

#### 3. Tests

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes** (write first):
- **Label distinctness**: extend the existing distinctness test (`:1146-1161`) to
  assert **all five** labels are pairwise distinct in both glyph and text, and
  the no-ANSI test (`:1163-1173`) covers all five.
- **Engine**: a table of (local-changed, remote-changed) → expected state across
  all four cells, where remote-changed is driven by `remote_hash` mismatch (not a
  phantom "baseline-equivalent"); pre-filter short-circuits (old mtime →
  candidate-unchanged then hash-confirmed without re-fetch where possible; equal
  remote `updated` → candidate-unchanged); a touch/reformat with equal hash →
  synced; whitespace-only + `updated_at`-only remote delta → synced (the story's
  "unchanged" AC); **baseline entry present but `external_id` absent → presence-only**
  (the 5th branch); **lexicographic ISO-8601 comparison** exercised with a
  realistic Jira `updated` value (e.g. millisecond + offset form) to confirm the
  no-reparse assumption holds for the actual format; a **tracked-but-remote-absent**
  case (item carries an `external_id` but its key is missing from a *successful*
  fetch response) → remote-absent (presence-only on list, report+skip on sync) —
  asserted **separately** from the failed-read case; an **indeterminate** (failed)
  remote read → presence-only on the list path and skip on the sync path; the local
  mtime gate is a pure integer compare (a file mtime numerically ≤ the epoch
  `timestamp` → candidate-unchanged, then hash-confirmed).

### Success Criteria

#### Automated Verification

- [x] Engine + label tests pass: `bash skills/work/scripts/test-work-item-scripts.sh`
- [x] All five labels pairwise distinct in glyph AND text (asserted in suite)
- [x] `mise run scripts:check` clean; bashisms clean on both scripts
- [x] `mise run check` green

#### Manual Verification

- [ ] Hand-running the engine against a synced item whose local body was edited
      reports `locally-modified`; editing the remote instead reports
      `remotely-modified`; both → `conflict`.

---

## Phase 5: `/list-work-items` five-state extension

### Overview

Wire the engine (Phase 4) and read bridge (Phase 1) into `/list-work-items` Step
4 so tracked items with a baseline render the three new states, degrading
gracefully when no baseline exists or the remote is unreachable. Render call
sites are unchanged — they receive the engine's keyword. Ships the
`/list-work-items` half of the story on its own.

### Changes Required

#### 1. Step 4 classification extension

**File**: `skills/work/list-work-items/SKILL.md`
**Changes**: In the "Sync classification" block (`:203-217`) and "Sync Status
Labels" section (`:250-272`): when an integration is configured **and** a
`last-sync.json` baseline exists, classify each item with `external_id` via
`work-item-sync-classify.sh`, feeding the resulting keyword to
`work-item-sync-label.sh --label`. Items with no baseline entry, or no
`external_id`, keep the presence-only synced/unsynced path. The
`canonical-tree-fence` stays label-free.

**Bulk read, not per-item `show` (avoids the N+1)**: the remote-side pre-filter
only needs each issue's `updated` timestamp. So drive it from the bridge's
`search --keys <tracked external_ids>` (Phase 1), which returns the tracker-agnostic
`external_id → {updated[, body]}` map — the **adapter** picks the strategy (Jira:
chunked `key in (…)` + `--all-projects`, `ceil(tracked/50)` calls; Linear: one
team-wide `--limit 250` auto-paginated search indexed by identifier). The caller
does **not** branch on tracker. A tracked `external_id` **absent** from a
*provably complete* result maps to the engine's remote-absent branch
(presence-only here); a **truncated/incomplete** result maps to indeterminate
(Linear: `truncated:true`; Jira: page/chunk cap hit), never remote-absent. A
per-item `show` is reserved for the genuinely-changed minority whose `updated`
differs from `remote_updated_at`: **Jira** search carries `description` (confirm
ADF parity, else `show`); **Linear** search does **not** carry `description`, so a
changed Linear item always fetches its body via `show` — the same shape as Jira's
fallback, not a Linear-only "no show needed" optimisation.

**Graceful degradation**: the fetch bridge returns a distinct "remote-unavailable"
signal on failure/timeout; on that signal every item falls back to its
presence-only label, the command still exits 0 and never hangs (bridge read bound
by the integration's existing timeout; no retry storm). Because the read is now a
single bulk call, one timeout — not N — bounds the whole degradation path. For a
large changed set that still needs some per-item fetches, emit incremental
progress (classifying item k of N) so a long pass does not read as a hang.

#### 2. Documentation of the new states

**File**: `skills/work/list-work-items/SKILL.md`
**Changes**: Update the seam note (`:266-272`) from "story 0051 adds…" (future)
to describe the now-present five-state rendering and the degradation rule.

### Success Criteria

#### Automated Verification

- [x] Hierarchy fence byte-equality still holds: `bash scripts/test-hierarchy-format.sh`
- [x] Work-item script suite green: `bash skills/work/scripts/test-work-item-scripts.sh`
- [x] Degradation is unit-tested (not manual-only): `work-item-sync-classify.sh`
      returns `indeterminate` (which the list path maps to presence-only) when
      handed `--remote-status indeterminate` — the fetch bridge's
      remote-unavailable signal (asserted in the engine state table)
- [x] SKILL frontmatter/config tests green: `bash scripts/test-config.sh`
- [x] `mise run check` green

#### Manual Verification

- [ ] With a `jira` integration, a baseline present, and a locally-edited tracked
      item, `/list-work-items` shows `🔵 locally modified` for it.
- [ ] A remote-edited item shows `🟣 remotely modified`; a both-edited item shows
      `🔴 conflict`.
- [ ] With the remote unreachable, `/list-work-items` still lists every item with
      at least its synced/unsynced label and exits without error or hang.
- [ ] With no `last-sync.json`, only synced/unsynced labels appear.

---

## Phase 6: `/sync-work-items` core reconciliation

### Overview

Create the skill: config gate, the four modes, and reconciliation of **synced**
items (push local-ahead, pull remote-ahead). Conflicts are **reported and
skipped in all modes** at this stage (directional modes keep this behaviour
permanently; bidirectional gains interactive resolution in Phase 7). Persists
`last-sync.json` crash-safely; `--preview` makes no writes. Ships a usable,
safe sync skill.

### Changes Required

#### 1. New skill

**File**: `skills/work/sync-work-items/SKILL.md` (new)
**Changes**: Frontmatter (`name`, `description`, `argument-hint:
"[--push-only|--pull-only] [--preview] [--all] [filter-flags…]"`, `allowed-tools`
scoped to `config-*`, the work `scripts/*`, and the integration read/create/update
bridges). Body:
- **Live context** via `!` preprocessor: read `work.integration`,
  `work.default_project_code`, the baseline path.
- **Config gate**: if `work.integration` is empty (the reader exits 0 with a
  blank line — `config-read-work.sh:24-58`), the skill prints a clear, actionable
  error and exits (AC: unconfigured → clear error). The message states **what**
  (no `work.integration` configured), **why** (sync needs an active tracker), and
  **how to fix** — naming the key, its valid values (`jira` / `linear` / `trello`
  / `github-issues`), and the concrete step (`/accelerator:configure`). This is
  the common first-run path (the live repo has no `work:` section).
- **Mode parse** via the decision script's validation: `--push-only` /
  `--pull-only` (mutually exclusive → error), `--preview`, `--all`, plus
  pass-through filter flags. Bidirectional is the default; the help/argument-hint
  states explicitly that omitting directional flags means bidirectional and shows
  a `--push-only --preview` composition example.
- **Reconcile synced items**: for each local item with `external_id`, classify
  via `work-item-sync-classify.sh`, then ask `work-item-sync-decide.sh` for the
  action given (mode, state). Actions:
  - **push** (local-ahead, push-permitted) → `work-item-update-remote.sh update`
    (Phase 1 write bridge — **not** the create bridge, which only creates).
  - **pull** (remote-ahead, pull-permitted) → overwrite the local file from the
    fetched remote body via `atomic_write` (never an in-place truncate-rewrite),
    and emit a per-item summary line `<id>: local replaced from remote` so the
    overwrite is visible and revertable. **Dirty-working-copy guard** (see new
    `work-item-file-dirty.sh`, Phase 6 §2): the recovery model is VCS revert, which
    cannot recover *working-copy* changes not yet captured in a commit — and this
    applies to **both** supported VCSs (we assume the user knows their VCS; we do
    not assume one has no working-copy state). Before overwriting, call the guard
    (mode resolved `.jj`-present-wins, so a jj-colocated checkout is treated as jj —
    see §2 for the dispatch and the fail-safe-to-dirty behaviour). If dirty, do
    **not** silently overwrite — in bidirectional mode route it through the conflict
    prompt (Phase 7); in `--pull-only`/non-interactive mode **skip-with-warning**
    (report the `id` in the summary, never overwrite). **Aggregate blast-radius gate**: if
    the number of local files a run will overwrite exceeds the shared threshold
    (the same constant as the Phase 8 gate, default 25), pin the prompt
    `N local files will be overwritten from remote. Proceed? [y/N]` and evaluate it
    **before any pull-overwrite write occurs**. The gate **fails safe**: empty
    input, non-interactive context, or a non-`y` answer aborts the entire
    pull-overwrite class with **zero** writes and a non-zero exit — never
    proceed-on-no-answer.
  - **skip-conflict** → report + skip (all modes at this stage; bidirectional
    gains interactive resolution in Phase 7).
  - **indeterminate** (failed/timed-out remote read) → skip the item, write
    neither side, report it as needs-retry. Never push (could clobber a remote
    that is actually ahead) or pull on an unknown remote state.
  `--preview` runs the same classification + decision but every push routes
  through the update bridge's real `--dry-run` (`--print-payload`) and every pull
  is reported, not written; **no** baseline mutation occurs — neither per-item
  `set` **nor** the global `set-timestamp` (a preview that advanced the timestamp
  would poison the next real run's pre-filter).
- **Terminal-failure handling on push** (mirrors the create path): 71/terminal
  codes from the update bridge are **never** auto-retried (a resent PUT could
  double-apply); the item is reported as needs-manual-reconciliation and its
  baseline entry is left **unset**, so the next run re-classifies authoritatively
  rather than blindly re-pushing.
- **Resumable persistence** (Decision #4) via `work-item-sync-apply.sh`: per item,
  perform the side-effect, then `work-item-sync-baseline.sh set <id> …` (with the
  new `remote_hash`); advance the global timestamp via `set-timestamp` only on
  clean completion. Re-run is idempotent.
- **Progress feedback**: the reconcile loop (and the Phase 8 untracked-pull
  creation loop) emits in-flight `processing item k of N` lines, matching the
  `/list-work-items` read path — the write-bearing path is the slower one and a
  silent multi-second pause against a non-VCS-recoverable remote otherwise reads as
  a hang.
- **Summary output**: per-item lines grouped by action (pushed / pulled /
  conflicts-skipped / needs-retry / remote-absent) listing the affected `id`s — not
  bare counts — so the user can see exactly which items changed without re-running.

#### 2. New decision + apply scripts (extract orchestration out of SKILL prose)

Two pure scripts move the safety-critical orchestration out of model-executed
SKILL prose and into CI-testable code, mirroring how `work-item-push-decide.sh`
already isolates the push retry/fallback decision:

**File**: `skills/work/scripts/work-item-sync-decide.sh` (new)
**Changes**: A deterministic decision table — inputs `(mode, classified-state,
local-dirty?, [user-decision])` → one action keyword (`push` / `pull` /
`skip-conflict` / `skip-dirty` / `prompt` / `noop`). Encodes the full (mode ×
state) matrix in one place, including the **forbidden-write cells** for directional
modes (a conflict or remote-ahead item under `--push-only` never pulls; a
local-ahead item under `--pull-only` never pushes), the **dirty-routing** cells (a
remote-ahead/pull-eligible item whose local file is dirty → `prompt` in
bidirectional, `skip-dirty` in directional/non-interactive — so this routing is
unit-asserted, not SKILL prose), the **indeterminate/remote-absent → `noop`** cells,
and the mode-parse validity rule (`--push-only` + `--pull-only` → error). It also
exposes a `resolve-conflict-token <raw>` entry point mapping the conflict prompt's
typed input (`remote`/`local`/`skip`, plus empty/unknown → `skip`) to an action
(`accept-remote`/`push-local`/`skip`), so that destructive-choice interpretation is
unit-tested here too (Phase 7 §2/§3). The SKILL renders this script's output (and
runs the blast-radius gate + prompts around it); it does not re-derive the table in
prose.

**File**: `skills/work/scripts/work-item-sync-apply.sh` (new)
**Changes**: A thin, **fault-injectable** apply helper that performs one item's
per-item commit sequence in one auditable place: side-effect (push via the update
bridge / pull via `atomic_write`) → then `work-item-sync-baseline.sh set`. On the
**pull** branch the baseline `set` must hash from the **post-overwrite** state —
`local_hash` from the just-written local file and `remote_hash` from the remote
projection actually written — never the pre-pull content; otherwise the next run
would misclassify the freshly-pulled item as locally-modified/conflict (a baseline
self-corruption). Honours a test-only fault-injection hook (e.g.
`WORK_SYNC_FAIL_AFTER=side-effect`) so a test can interrupt **between** the
side-effect and the baseline set and assert re-run idempotency — making the
resumability AC (a non-VCS-recoverable remote write) CI-testable rather than
manual-only. A `finalise` sub-action performs the global `set-timestamp` with the
**run-start** epoch (captured by the SKILL before any item is read, passed in), so
the completion-ordering rule (advance only on clean completion; never under
`--preview`) is asserted in the same CI-tested place as the per-item ordering
rather than left to SKILL prose. Structure the script as a thin sub-action
dispatcher delegating to one short function each (`apply_push` / `apply_pull` /
`finalise`) so no single branch grows unwieldy as cells are added; `finalise` stays
a sibling sub-action, never folded into the per-item path.

**File**: `skills/work/scripts/work-item-file-dirty.sh` (new)
**Changes**: A small **VCS-mode-aware** predicate — `work-item-file-dirty.sh
<path>` exits 0 if the file has uncommitted working-copy changes, 1 if clean.

**Mode resolution — `.jj`-present-wins, not topology.** Resolve the repo root with
`vcs-common.sh`'s `find_repo_root`, then select the command set by the repo's
canonical idiom (as `scripts/vcs-status.sh`, `hooks/vcs-detect.sh`, and
`scripts/run-migrations.sh` already do): `[ -d "$ROOT/.jj" ]` ⇒ **jj** (this
*includes the jj-colocated case* where `.git` also exists — jj wins), else `[ -d
"$ROOT/.git" ]` ⇒ **git**. Do **not** drive this off `classify_checkout` (it
returns workspace *topology* kinds — `main`/`colocated`/… — not a git-vs-jj command
selector); in a colocated checkout a topology-based dispatch would wrongly route to
git, whose index *lags* the jj working-copy commit, so live uncommitted jj edits
would read as clean and be silently overwritten. Ideally factor the one-line
`.jj`-present check into a shared `vcs-common.sh` helper (e.g. `vcs_mode`) and have
all four call sites use it, rather than adding a fourth hand-copied duplicate.

**Dispatch.** **jj** → the path appears in `jj --no-pager diff --name-only` for the
working-copy commit `@` (the `--no-pager` flag matches `run-migrations.sh` and
prevents a configured pager from hanging or injecting control codes in this
captured-output context). **git** → `git status --porcelain -- <path>` non-empty;
an untracked path (`^??`) at the target counts as **dirty** for the overwrite guard
(an untracked file is not VCS-recoverable either, so overwriting it must not be
silent) — this is the deliberate deviation from `run-migrations.sh`'s `grep -v
'^??'`, stated here so it is intentional.

Both VCSs genuinely have a working-copy-changes notion (jj's working copy is itself
a commit, but `jj diff` reports its changes against the parent), so the guard is
real under both; we assume the user knows their VCS. **Indeterminate VCS mode**
(no `.jj` and no `.git`, or detection fails) → **fail safe to *dirty*** so the
overwrite is routed to prompt/skip rather than proceeding — the prerequisite check
already requires a VCS binary, so genuine no-VCS is rare and treating it as dirty
errs toward preserving local work. The pull guard (§1) and any other
local-overwrite path call this before `atomic_write`. The dispatch (resolved mode +
the per-VCS status command) is stubbable in tests (inject mode + a recorded status)
so the guard is exercised deterministically under **git, jj, jj-colocated, and
indeterminate** modes without a real working copy.

#### 3. Registration

**File**: `.claude-plugin/plugin.json`
**Changes**: Register `skills/work/sync-work-items` in the skills list.

#### 4. Tests

**Files**: `skills/work/scripts/test-work-item-scripts.sh` (decision-table +
apply-helper logic), the bridge tests from Phase 1 (mock-server driven, reused
here for the push/pull side-effects — **not** PATH stubs), `scripts/test-config.sh`
(SKILL frontmatter + plugin.json registration counts).
**Changes** (write first):
- **Decision table** (`work-item-sync-decide.sh`): the full (mode × state)
  matrix, explicitly asserting the forbidden-write cells (conflict/remote-ahead
  under `--push-only` → no pull; local-ahead under `--pull-only` → no push) and
  `--push-only` + `--pull-only` → error; **`indeterminate` and `remote-absent`
  states → a `noop`/skip action with NO side-effect and NO baseline `set`** (the
  worst-case data-loss path — never push on unknown/absent remote state).
- **Apply/resumability** (`work-item-sync-apply.sh`): inject a failure between the
  side-effect and the baseline `set`; assert re-run does not re-push A, processes
  B exactly once, and an already-created remote-only item is not created twice.
- **Post-pull baseline**: after a pull-overwrite, the item is classified `synced`
  on the immediately-following run (baseline `local_hash`/`remote_hash` were taken
  from the post-overwrite content, not the pre-pull content).
- **Run-start timestamp / concurrent edit**: `finalise` persists the run-START
  epoch; a file edited *during* the run (mtime advanced after it was read) is
  re-hashed — not short-circuited — on the next run.
- **Pull-overwrite guards**: `work-item-file-dirty.sh` is exercised via the
  stubbable dispatch across **all four** mode cases — **git** (porcelain non-empty,
  incl. an untracked `^??` path → dirty), **jj** (path in `jj --no-pager diff
  --name-only` → dirty), **jj-colocated** (`.jj` *and* `.git` present → resolves to
  the **jj** arm, never git), and **indeterminate** (no `.jj`/`.git` → **dirty**,
  fail-safe). A pull-overwrite of a dirty file is not silent (bidirectional →
  prompt; `--pull-only` → skip-with-warning, `id` reported). The aggregate gate
  counts only the **clean** (post-dirty-routing) overwrite set; above-threshold
  requires a `y`; a non-`y`/non-interactive/declined answer aborts the
  pull-overwrite class with **zero** writes (fail-safe).
- **Preview**: a preview run leaves `last-sync.json` **byte-identical** (neither
  `set` nor `finalise`/`set-timestamp` fires) and the update bridge records **no**
  write.
- **Terminal push**: a 71-mapped update outcome is not auto-retried and leaves the
  item's baseline entry unset.

### Success Criteria

#### Automated Verification

- [x] Work-item script suite green: `bash skills/work/scripts/test-work-item-scripts.sh`
      (plus `bash skills/work/scripts/test-work-item-sync-apply.sh` for the
      mock-driven push path)
- [x] Config/registration tests green: `bash scripts/test-config.sh` (skills are
      registered by directory — `./skills/work/` already covers the new skill, so
      no plugin.json edit was needed)
- [x] `mise run scripts:check` clean
- [x] `mise run check` green

#### Manual Verification

- [ ] `/sync-work-items` with no `work.integration` prints a clear error and
      exits.
- [ ] `/sync-work-items --preview` reports intended changes and writes nothing
      (no local edits, `last-sync.json` unchanged).
- [ ] Default mode with no conflicts: remote-ahead items updated locally,
      local-ahead pushed, `last-sync.json` updated.
- [ ] `--push-only` writes no local file; `--pull-only` makes no remote write;
      conflicts are reported and skipped in both.
- [ ] Killing a run after one item then re-running does not re-push/re-pull the
      reconciled item.

---

## Phase 7: Conflict resolution UX

### Overview

Add the section-by-section diff and the remote-default override prompt so
**bidirectional** mode resolves conflicts interactively (directional modes keep
report-and-skip). Emits an override-log line when the user keeps local. Builds on
Phase 6.

### Changes Required

#### 1. Section-splitter + diff script

**File**: `skills/work/scripts/work-item-section-diff.sh` (new)
**Changes**: Split each side into named sections (frontmatter, Summary, Context,
Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions,
Technical Notes, Drafting Notes, References — by `^## ` headings plus the
frontmatter block) and emit a per-section textual diff, so large items stay
reviewable. Each section diff is **explicitly headed with `LOCAL` / `REMOTE`
labels** and a **fixed, documented direction** (local as the baseline/`-` side,
remote as the change/`+` side, matching the default-accept side) so the user can
unambiguously map the diff to the prompt's default. Use only the **POSIX-portable
`diff -u`** surface (no GNU-only long flags, no `--color`) to avoid GNU-vs-BSD
output divergence. **Byte-equality of a section is decided by the normaliser +
hash, not by `diff`'s exit status** (whose `1`=differences semantics is a
portability trap); byte-equal-after-normalisation sections are omitted.

#### 2. Conflict prompt in the skill

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes**: When bidirectional hits a conflict, render the LOCAL/REMOTE section
diff, then a prompt that requires a **typed token** rather than a `y/n` keystroke.
This satisfies the story policy two ways: remote is the **recommended** choice
(named first / as the default the user is steered to), **and** no local write
occurs without explicit confirmation (the AC's requirement) — and it avoids the
trap of a bare Enter silently discarding local edits, which the suite's `[y/N]`
muscle-memory would otherwise invite. Because it is not a `y/n` prompt, it does not
collide with the `[y/N]` polarity used by the batch-push and create prompts. Pin
the exact string:

```
Conflict on <id> (<external_id>). Recommended: keep remote.
Type 'remote' to OVERWRITE your local edits with the remote version,
'local' to push your local version to the remote, or
'skip' to leave both unchanged and resolve it later. [remote/local/skip]
No default — Enter (or an unrecognised entry) re-asks once, then skips.
```

Strict interpretation: `remote` (after trimming) accepts remote (overwrite local
from remote via `atomic_write`); `local` overrides and pushes local via the update
bridge; `skip` (the safe "resolve later" outcome) reports + skips with **no**
write; empty input or any other token re-prompts **once**, then defaults to `skip`
(never to a destructive write). There is deliberately **no Enter default** —
'Recommended: keep remote' steers the choice but still requires typing the word, so
a reflexive Enter can never discard local edits. On the `local` override, emit an
**override-log line** to the summary naming the item's `id`, its `external_id`, and
the direction, e.g. `OVERRIDE <id> (<external_id>): pushed local→remote`. Either
write resolution then commits its side-effect and updates the baseline through
`work-item-sync-apply.sh` (Phase 6 ordering, incl. post-write `remote_hash`). The
strict token→action mapping is **not** SKILL prose: it is a named pure entry point
on the decision script — `work-item-sync-decide.sh resolve-conflict-token <raw>` →
one action (`accept-remote` / `push-local` / `skip`) — so the destructive-choice
interpretation is unit-asserted in the same tested place as the rest of the
decision vocabulary. The SKILL reads the raw input, passes it to this entry point,
and renders/acts on the returned action; it never re-derives the mapping. The same
typed-token prompt (plus `skip`) is what the dirty-working-copy guard (Phase 6 §1)
routes to in bidirectional mode.

#### 3. Tests

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes** (write first): `work-item-section-diff.sh` groups changes under the
right section headings; byte-equal sections are omitted; frontmatter is its own
section; a conflict spanning two sections shows both. **Conflict token mapping**
(the highest-stakes branch — a misparse discards local edits): assert
`work-item-sync-decide.sh resolve-conflict-token` maps `remote` → `accept-remote`,
`local` → `push-local`, `skip` → `skip`, with surrounding-whitespace trimming and
case handling, and that **empty input or any unrecognised token resolves to `skip`
(never to either destructive write)** — covering the re-prompt-once-then-default-skip
contract. (Only the literal prompt *wording* stays SKILL prose; the mapping is
asserted here.)

### Success Criteria

#### Automated Verification

- [ ] Section-diff tests pass: `bash skills/work/scripts/test-work-item-scripts.sh`
- [ ] `mise run scripts:check` clean; bashisms clean on the new script
- [ ] `mise run check` green

#### Manual Verification

- [ ] A conflict in bidirectional mode shows a section-grouped diff with remote
      labelled as the default choice; pressing the default accepts remote and
      overwrites local.
- [ ] Choosing the override pushes local to remote and prints an override-log
      line naming the item `id` and direction.
- [ ] In `--push-only`/`--pull-only`, the same conflict is reported and skipped
      with no prompt.

---

## Phase 8: Unsynced batch push + untracked remote pull

### Overview

Complete the skill: offer to push items with no `external_id` (per-item or
batch), and pull untracked remote issues (default scoped to
`work.default_project_code`, `--all` drops the scope, filter flags narrow the
set). Builds on Phase 6.

### Changes Required

#### 1. Unsynced push offer

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes**: For each local item with no `external_id`, offer a push using **one
pinned grammar** (the two cited precedents use *different* grammars — a numbered
menu vs y/n — so reproduce exactly one here): a per-item `[y/N]` prompt whose
fast-path keys are surfaced in the string itself so they are discoverable. Pin it:

```
Push <id> "<title>" to <tracker>? [y/N]  (a = push all remaining, d = decline all remaining)
```

The fast-path touches only un-decided items and **never resurrects declines**.
Accepted items push via `work-item-create-remote.sh`; the returned key is
substituted into the `external_id` line in memory, then the whole item (frontmatter
incl. `external_id` + body) is written in a **single `atomic_write`** (mirroring
`create-work-item/SKILL.md:567,574-580`) so the file never exists in a
half-linked state; declined items are untouched. `work-item-push-decide.sh`
governs retry/terminal handling.

#### 2. Untracked remote pull

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes**: Fetch remote issues via `work-item-fetch-remote.sh search`
(Phase 1), forwarding user filter flags verbatim. Default scope is
`work.default_project_code`; `--all` reuses the tracker's `--all-projects`
primitive to drop only the project clause while user filters still apply. Compute
the untracked set (remote issues whose key is not already held by any local
`external_id`). **Blast-radius gate**: when the untracked set exceeds the shared threshold
(a single constant, default 25, used by both this gate and the pull-overwrite
gate), pin the prompt grammar to match the batch-push polarity — safe default is
"no":

```
N untracked remote issues will be created. Proceed? [y/N]
```

Evaluated **before any creation write occurs**; it **fails safe** — empty input,
non-interactive context, or a non-`y` answer aborts the untracked-pull class with
**zero** creations and a non-zero exit. This keeps a mis-scoped `--all` or an
automation-flooded project from flooding `meta/work/` and exhausting IDs.
For the confirmed set, **allocate the whole batch up front** with
`work-item-next-number.sh --count N` (sequential ids in one call) — never a
per-item allocation in a loop, which would hand every pulled item the same number
until each file lands. For each issue, build the full frontmatter (incl.
`external_id` = remote key and the allocated `id`) in memory and write it in a
single `atomic_write`, then record its baseline entry. Idempotent across re-runs
(a created item is no longer untracked, and a half-written file cannot occur).
`work-item-next-number.sh` reserves nothing (it scans for the highest id at call
time), so the up-front `--count N` batch is correct under the **single-writer**
assumption this on-demand skill operates under; the apply loop re-validates each
allocated id is still free immediately before its `atomic_write` and aborts the
batch on an unexpected collision rather than overwriting.

#### 3. Tests

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes** (write first): batch accept-all pushes all undecided, decline-all
pushes none, declines are not resurrected; `external_id` writeback uses the
bridge-returned key; untracked-detection treats a remote key already held by a
local `external_id` as tracked (no duplicate create); `--all` drops project scope
while a `--label` filter still applies (filter-passthrough parity with the
search bridge); **blast-radius gate** — an untracked set above the threshold blocks
creation until confirmation and below it proceeds; **batch allocation** — pulling N
untracked issues yields N **distinct sequential** ids in one `--count N` call (no
duplicates), never a per-item loop.

### Success Criteria

#### Automated Verification

- [ ] Work-item script suite green: `bash skills/work/scripts/test-work-item-scripts.sh`
- [ ] `mise run scripts:check` clean
- [ ] `mise run check` green
- [ ] Full CI mirror green: `mise run`

#### Manual Verification

- [ ] Items with no `external_id` trigger a per-item push offer; batch accept-all
      pushes all and writes each one's returned key to `external_id`; declines
      stay unchanged.
- [ ] An untracked remote issue within `work.default_project_code` is created as a
      local item carrying the remote key as `external_id` and an independent local
      `id`.
- [ ] Supplying `--label X` pulls exactly the set the integration's `search-*`
      skill returns for `--label X`; `--all` bypasses only the project scope.
- [ ] Re-running does not re-create an already-pulled remote issue.

---

## Testing Strategy

### Unit Tests

- **Bridges (Phase 1)** are tested with the **mock-HTTP-server harnesses** the
  integrations already use — `mock-jira-server.py` **and** `mock-linear-server.py`
  + scenario fixtures — in dedicated `test-work-item-fetch-remote.sh` /
  `test-work-item-update-remote.sh` files run against **both** trackers (not PATH
  stubs, which cannot intercept the absolute-path integration invocations). Assert
  per tracker: the Jira `--fields` injection / `key in (…)` + `--all-projects`
  chunked fetch, the Linear team-wide-search-indexed-by-identifier + `updatedAt`
  presence, the update payload, and the shared exit-taxonomy mapping — against
  captured requests.
- **Phases 2–4** are validated by unit tests in `test-work-item-scripts.sh` (and
  `test-jira-paths.sh` for path equality): normalisation equivalence classes
  (incl. locale-stability and remote-projection determinism), baseline
  round-trip + structural crash-safety, the engine's state table driven by
  `remote_hash` + the 5th presence-only branch + lexicographic ISO edge, and
  five-state label distinctness.
- **Phases 6–8** unit-test the **extracted** script logic directly:
  `work-item-sync-decide.sh`'s full (mode × state) matrix including the
  forbidden-write cells, and `work-item-sync-apply.sh`'s fault-injected
  resumability — with the bridges driven by the mock server, no live tracker.

### Integration Tests

- A `work.integration: jira` config fixture drives end-to-end scenarios (the live
  repo has no `work:` section, so tests must supply one).
- Resumability is **automated, not manual**: `work-item-sync-apply.sh`'s
  fault-injection hook interrupts between the side-effect and the baseline `set`;
  the test asserts the re-run reconciles B once and never re-pushes A or
  re-creates a pulled item. The per-item commit ordering is exercised in the
  script, not only in SKILL prose.

### Manual Testing Steps

1. Configure `work.integration: jira` and `work.default_project_code`.
2. Run `/sync-work-items --preview` — confirm zero writes.
3. Edit a synced item locally; run default sync — confirm push + baseline update.
4. Edit a synced item only on the remote; run sync — confirm local update.
5. Edit both sides; run bidirectional — confirm section diff + remote-default
   prompt; test both accept-remote and override-local paths.
6. Create a local item with no `external_id`; run sync — confirm push offer +
   `external_id` writeback.
7. Run `/list-work-items` — confirm all five states render; kill the remote and
   confirm graceful degradation.

## Performance Considerations

- **Avoid the N+1**: the remote-side pre-filter needs only each issue's `updated`
  timestamp, fetched in bulk by the `search --keys` adapter (Phase 1) — **not** N
  per-item calls. **Jira**: the search flow caps at 100 results/page with no key
  filter, so the adapter wraps chunked `key in (…)` JQL (`--fields
  updated,summary,description`) with `nextPageToken` pagination → `ceil(tracked/50)`
  calls. **Linear**: no key filter and internal auto-pagination, so one team-wide
  `--limit 250` search → `ceil(team/250)` internal pages, bounded by
  `MAX_PAGES=20` (a board beyond ~5000 issues returns `truncated:true`, which the
  adapter maps to indeterminate, not remote-absent). Reserve per-item `show` for the
  genuinely-changed minority: Jira only where `description` is not fidelity-equivalent
  to `show`'s body; Linear always (its search omits `description`).
- Each chunk/page request is individually timeout-bounded and the chunk/page count
  is capped, so a slow/unreachable remote degrades (list) or fails-soft (sync)
  within a finite bound, never hanging the whole run. Chunk/page fetches are
  **sequential** (the simplest correct shell shape); at the expected corpus scale
  this is a handful of round-trips, so bounded concurrency is deliberately not worth
  the bash complexity — the degradation ceiling is `chunk-count × per-request
  timeout`, which the cap keeps finite. The local `normalise+hash`
  cost is per-item but cheap; on a cold run (no baseline / fresh checkout) every
  item is hashed once, which is acceptable for the expected corpus size and is the
  authoritative path the advisory mtime gate cannot replace.

## Migration Notes

- `last-sync.json` is **committed** (Decision #2); **no change to either
  `JIRA_INNER_GITIGNORE_RULES` or `LINEAR_INNER_GITIGNORE_RULES`** — under both
  trackers `last-sync.json` is committable by default (neither array excludes it),
  so there is no change to the byte-pinned copies in
  `0003-relocate-accelerator-state.sh`/`test-jira-paths.sh` (Jira) or
  `test-linear-paths.sh` (Linear). If a later story reverses the committed decision
  (e.g. to remove the multi-writer staleness window), every copy and pin test (both
  trackers) must change together — and that reversal should **collapse the
  duplication to a single sourced definition** rather than re-pinning more copies.
- No data migration: the baseline (including the new `remote_hash` field) is
  created on first successful sync; its absence — and the absence of `remote_hash`
  on a pre-existing entry — is a valid state (the entry simply re-evaluates
  authoritatively via the hash on next sync). Presence-only classification applies
  whenever no baseline entry exists.
- Because `last-sync.json` is committed, concurrent syncs on different branches can
  produce a VCS **merge conflict** on the file. The baseline reader treats an
  unparseable / conflict-markered file as an **empty baseline** (hard contract —
  `jq` parse failure ⇒ empty, never an error), so a botched merge degrades
  gracefully to presence-only + a full re-hash on the next sync rather than
  crashing. This is the accepted trade for a shared baseline.
- Runtime prerequisites: `jq` (≥ 1.5 — the floor for `-S` key-sort
  canonicalisation, already the repo's pinned floor), `sha256sum`/`shasum`, and
  `git`/`jj` for the dirty check. An early prerequisite check (before any remote
  call) names each missing tool and how to obtain it, mirroring the what/why/how
  structure of the `work.integration` config-gate message — never degrade to a raw
  `jq: command not found` or, worse, a silently non-canonical divergent hash. A jq
  present but unable to canonicalise (no `-S`) is treated as missing.

## References

- Original work item: `meta/work/0051-sync-work-items-skill.md`
- Research: `meta/research/codebase/2026-06-18-0051-sync-work-items-skill.md`
- Seam: `skills/work/scripts/work-item-sync-label.sh:24-27,41-63`
- Render sites: `skills/work/list-work-items/SKILL.md:27-33,203-217,250-292,315-328`
- Create bridge / push UX: `skills/work/scripts/work-item-create-remote.sh`,
  `skills/work/scripts/work-item-push-decide.sh`,
  `skills/work/create-work-item/SKILL.md:503-580`
- Jira APIs: `skills/integrations/jira/scripts/jira-search-flow.sh`,
  `jira-show-flow.sh:115-161`, `jira-emit-key.sh`, `jira-common.sh:53-87`
- Linear APIs (single-team, Markdown-native, auto-paginating): `linear-search-flow.sh`
  (state/assignee/label/text filters — **no key-set filter, no `--all`**),
  `linear-show-flow.sh` (by identifier), `linear-graphql.sh` (the `updatedAt`
  selection add lands here), `linear-common.sh` (`linear_state_dir`,
  `LINEAR_INNER_GITIGNORE_RULES`)
- Update flows (whole-item replace + `--print-payload` dry-run, wrapped by the new
  write bridge): `skills/integrations/jira/scripts/jira-update-flow.sh`
  (`--summary`/`--body-file`), `skills/integrations/linear/scripts/linear-update-flow.sh`
  (`--title`/`--description` inline)
- Bridge test harnesses to mirror: `skills/work/scripts/test-work-item-create-remote.sh`
  + `mock-jira-server.py`; the Linear equivalent driven by `mock-linear-server.py`
  (`test-linear-search.sh`/`test-linear-update.sh`)
- ADR: `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md`
- Predecessor plan: `meta/plans/2026-06-15-0047-core-skills-sync-integration.md`
- This review: `meta/reviews/plans/2026-06-18-0051-sync-work-items-skill-review-1.md`

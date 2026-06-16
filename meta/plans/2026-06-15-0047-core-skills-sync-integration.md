---
type: plan
id: "2026-06-15-0047-core-skills-sync-integration"
title: "Core Skills Sync Integration Implementation Plan"
date: "2026-06-15T22:35:56+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0047"
parent: "work-item:0047"
derived_from: ["codebase-research:2026-06-15-0047-core-skills-sync-integration"]
tags: [work-management, integrations, sync, list-work-items, create-work-item]
revision: "86a80de9ff33fe3e4b413d32f3f7d82cfa0b2097"
repository: "ticket-management"
last_updated: "2026-06-16T07:23:53+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Core Skills Sync Integration Implementation Plan

## Overview

When `work.integration` is configured, make the two core work-management skills
sync-aware: `/list-work-items` renders a visually distinct (markdown glyph + text)
synced/unsynced label per item, and `/create-work-item` offers an interactive push
to the remote tracker after drafting. Two decisions taken during planning reshape
the work item's original framing:

1. **The remote identifier lives in `external_id`, not `work_item_id`.** `id`
   stays the local own-identity (`(<project-code>-)?\d{4}`); `external_id` holds
   the remote tracker's identifier. No work item carries a `work_item_id`
   remote-key any more — the Linear create flow (the only writer of it) is
   refactored onto the `id`/`external_id` split, and the Jira create skill is
   harmonised to the same contract.
2. **Sync classification is presence-based, not format-based.** An item is
   *synced* iff it carries a non-empty `external_id`, *unsynced* otherwise. The
   work item's `^[0-9]+$` format rule (AC #4/#5) is dropped — it misclassifies
   project-coded local IDs (`PROJ-0042`) as synced.

## Current State Analysis

- **`/list-work-items`** (`skills/work/list-work-items/SKILL.md`) scans six
  frontmatter fields via a single `awk` pass (`:181-190`), derives the displayed
  ID from the *filename* (`:159-166`, authoritative per `:310-314`), and renders
  a plain markdown table (`:226`) plus a test-load-bearing `canonical-tree-fence`
  block (`:246-251`). It does **not** read `work.integration`, has **no** colour
  output, and has **no** status→label lookup. `allowed-tools` already permits
  `config-*` and `skills/work/scripts/*` (`:7-9`).
- **`/create-work-item`** (`skills/work/create-work-item/SKILL.md`) drafts
  entirely in memory through Step 4, allocates the ID only at Step 5 via
  `work-item-next-number.sh` (`:406-411`), and writes **`id:`** (quoted) once at
  `:489`. `external_id` already exists as an omit-by-default cross-system pointer
  (`:484-486`). The fail-safe `y`/`Y`-only gate at `:545-551` is the prompt
  template to copy. No `work.integration` read.
- **`work-item-read-field.sh`** bridges `id`↔`work_item_id` only when the
  requested key is *absent* (`:90-100`) — this is the **legacy own-identity**
  bridge, unrelated to the remote key.
- **`config_set_frontmatter_field`** (`config-common.sh:122-209`) is
  **replace-only** — exit 5 if the key is absent; it cannot insert.
- **Linear create** (`linear-create-flow.sh`) is **file-first**: reads
  `title`+`work_item_id` from the file (`:126`), rejects remote-format
  `work_item_id` as already-synced (`:138-142`), creates via GraphQL, validates
  the returned identifier (`:201-205`), and writes it back to **`work_item_id`**
  (`:209`) behind a loud `E_CREATE_WRITEBACK_FAILED` (`:210-213`). This is the
  **only** site that writes a remote key into `work_item_id`.
- **Jira create** (`jira-create-flow.sh`) is **content-driven**: takes
  `--summary`/`--body-file`/`--type`/`--project` flags, prints the `{id,key,self}`
  response, does **no** file I/O and **no** writeback. `create-jira-issue/SKILL.md`
  is a generic flag skill that never touches a work-item file.
- **Only jira and linear are built**; trello (0049) and github-issues (0050) are
  unbuilt. **No dispatcher** maps `work.integration` → a create flow.

### Key Discoveries

- `work_item_id` has **three distinct uses** today; only the third is in scope:
  (A) legacy own-identity (the pre-`id` schema, handled by the bridge and
  migrations — **untouched**); (B) foreign-reference linkage in plans/research/
  PRs (`templates/*.md`, `describe-pr`, `research-*` — **untouched**); (C) Linear's
  remote-key writeback (`linear-create-flow.sh:209` — **refactored to
  `external_id`**).
- `config-read-work.sh integration` exits **0 with an empty line** when
  unconfigured (`:46-58`); both skills must branch on the **empty string**, not
  the exit code.
- The two integration create flows diverge on *content source* (flags vs file)
  and *who writes back* (caller vs script). The canonical contract this plan
  standardises on: **integration primitive = content-in → validated
  identifier-out, no file writeback**; **user-facing create skill =
  work-item-file-driven, caller-side `external_id` writeback**.
- `external_id` *is* the per-item local→remote mapping — no separate mapping
  store is needed. It **may equal `id`** (Jira/Linear, aligned schemes) or be
  **independent** (Trello, opaque card IDs). It is **always written on push
  success**, even when equal to `id` — presence is the synced signal.

## Desired End State

With `work.integration` set:

- `/list-work-items` shows a visually distinct **synced** (has `external_id`) /
  **unsynced** (no `external_id`) label per item, via a data-driven
  `status → {label, glyph}` lookup that 0051 can extend without touching the
  rendering call site. **The label uses a markdown-native glyph + distinct text**
  (e.g. `🟢 synced` / `⚪ unsynced`), *not* raw ANSI escapes: `/list-work-items`
  output is emitted by the model as a markdown table in the conversation, not
  streamed to a TTY, so ANSI escapes would render as literal text. This
  supersedes work item 0047's "terminal ANSI colour output" wording (AC #3 /
  Assumptions) — reconcile via the follow-up `/update-work-item` already flagged
  in References. Unconfigured → no label, unchanged output.
- `/create-work-item` offers a push after drafting; on accept the remote creates
  the issue and the local file is written **once** with the returned key in
  `external_id`; retry-once-then-local-fallback on failure; decline → local save;
  no file exists until one of those resolves.
- Both `create-linear-issue` and `create-jira-issue` follow one contract and
  write the remote key to `external_id`. No code path writes a `work_item_id`
  remote key.
- The `id`/`external_id` relationship is documented in `templates/work-item.md`
  and `configure/SKILL.md`.

Verify: `mise run check` green; the per-component shell test suites green
(below); manual walk-throughs of both skills under a configured integration.

## What We're NOT Doing

- **Not** touching the legacy own-identity `work_item_id` (use A): the
  `work-item-read-field.sh` bridge, `wip_is_work_item_file`, the migrations
  (0001/0002/0006/0007), and the "id (or work_item_id on legacy files)" prose all
  stay. Those are a separate canonicalisation concern (0064).
- **Not** touching the foreign-reference `work_item_id` in plan/research/PR/review
  templates and their producer skills (use B).
- **Not** building trello/github-issues create paths (0049/0050) — they get a
  clean "not available" message + local save.
- **Not** implementing the three baseline-dependent sync states (locally
  modified / remotely modified / conflict) or `last-sync.json` — owned by 0051.
- **Not** merging `/create-jira-issue` and `/create-linear-issue` into a single
  command — they remain per-integration skills sharing one contract.
- **Not** renaming files to the remote key or adopting the remote key as `id`.
- **Not** editing the 0047 work item here; this plan diverges from its AC #4/#5
  and `work_item_id` wording, which should be reconciled separately via
  `/update-work-item` (see References).

## Implementation Approach

A foundational **Phase 0** wires the shell suites into CI, then four feature
phases, each independently mergeable and green on its own. Phase 3 is fully
independent; Phases 1 and 2 are independent of each other; Phase 4 builds on the
primitives from 1–2 but lands green on top. Use TDD throughout: extend or add the
standalone shell test suites first (red), then implement (green). Skill-body
behaviour (`SKILL.md` prose, model-driven) is verified by the eval suites and
manual checks rather than unit tests.

**Phase 0 must land first**: the Linear/Jira/work shell suites this plan extends
and adds are not currently part of the CI gate, so without it the bulk of the
automated verification below would never run in CI (it would only pass when a
human runs each `bash` line by hand). Recommended landing order: **0 → 3
(independent, low-risk) → 1 → 2 → 4**. Any order that respects "0 before the
suites it wires are relied upon" and "4 after 1+2" is valid.

**Shared convention — the integration gate.** Three call sites
(`/list-work-items`, `/create-work-item`, and the dispatcher) gate on the same
rule, stated once here and referenced from each phase: **integration-configured
:= the output of `config-read-work.sh integration` is non-empty**. The script
exits **0 with an empty line** when unset, so all three branch on the *string*,
not the exit code.

**Shared convention — "non-empty `external_id`".** The Linear guard (Phase 1 §2),
the Jira guard (Phase 2 §1), and the `/list-work-items` classifier (Phase 3 §2)
all use one definition: **length > 0 after stripping surrounding quotes and
whitespace** — so `external_id: ""` / quote-only / whitespace-only reads as
*unsynced*.

---

## Phase 0: Wire the shell suites into CI

### Overview

The suites this plan extends and adds (`test-linear-create.sh`,
`test-jira-create.sh`, `test-work-item-scripts.sh`, the new
`test-work-item-create-remote.sh`) live in subtrees that the CI gate does **not**
currently run: `tasks/test/integration.py`'s `run_shell_suites` covers `scripts`,
`skills/visualisation`, `skills/decisions`, `hooks`, `skills/config/migrate`, and
`skills/github` — not `skills/work` or `skills/integrations`. So `mise run check`
(the stated definition of done) would execute none of them. Wire them in first,
so every later phase's automated verification genuinely gates.

### Changes Required

#### 1. Register the new subtrees

**Files**: `tasks/test/integration.py`, `mise.toml`
**Changes**: Add `run_shell_suites` coverage for `skills/work` and
`skills/integrations`, with matching `test:integration:*` task entries added to
the `test:integration` rollup in `mise.toml`. Confirm the existing
`scripts`-subtree suite `test-config.sh` (which gains the upsert tests) already
gates — it does — so only the two new subtrees need wiring. **Note these subtrees
spawn the stdlib-only `python3` mock servers** (`test-helpers/mock-*-server.py`):
confirm `python3` is provisioned in the CI image these suites now run under, and
account for the mock-server startup-timeout given the known parallel-load
flakiness of shell suites in this repo (poll-with-timeout on the url-file, and
keep the suites tolerant of slow mock startup under CI load).

#### 2. Executable-bit guard

**File**: the relevant `test/helpers` count-floor (mirror the existing
migrate/config guards)
**Changes**: `run_shell_suites` discovers only `**/test-*.sh` that carry the
exec bit (`os.access(p, os.X_OK)`); a suite committed without `chmod +x` vanishes
silently. Add an at-least count-floor assertion for the newly wired subtrees so a
dropped exec bit fails loudly rather than yielding a false green. **Size each floor
to the full discovered suite count at wiring time** — `skills/integrations` newly
captures the ~33 pre-existing jira/linear suites for the first time, so a floor set
only to the handful of new create suites would leave those inherited suites
unprotected against a silent exec-bit drop. Mirror the whole-subtree sizing of the
existing `_EXPECTED_CONFIG_SUITES` / `_EXPECTED_MIGRATE_SUITES` guards. Every new
`test-*.sh` in this plan is created executable.

### Success Criteria

#### Automated Verification

- [x] `mise run test:integration` lists and runs the `skills/work` and
      `skills/integrations` suites (verify by name in the output)
- [x] Removing the exec bit from a wired suite makes the count-floor guard fail
- [x] Full read-only gate: `mise run check`

---

## Phase 1: Linear — `external_id` convention + content-in/id-out primitive

### Overview

Refactor the Linear create path onto the canonical contract and the
`id`/`external_id` split. Split `linear-create-flow.sh` into a no-file
create-and-return core; move the writeback up into `create-linear-issue/SKILL.md`,
now targeting `external_id`. Add an insert-if-missing frontmatter helper (the
writeback must land on files that have no `external_id` line). Document the
`id`/`external_id` relationship.

### Changes Required

#### 1. Insert-if-missing frontmatter helper

**File**: `scripts/config-common.sh`
**Changes**: Add `config_upsert_frontmatter_field <file> <key> <value>`. **Fail
closed: insert only on a field that is genuinely absent from a well-formed
frontmatter block.**

> **Exit-code surface first.** `config_set_frontmatter_field`
> (`config-common.sh:122-209`) does **not** expose distinct exit codes today: its
> internal awk distinguishes no-frontmatter / unclosed / field-absent /
> duplicate, but the wrapper **collapses all of them to a single `return 1`**, and
> there is no `E_FM_*` constant. So the upsert helper **cannot** branch on a
> field-absent code. Resolve this one of two ways (pick one; do not leave both):
> 1. **(Preferred, smaller)** Re-detect absence independently: on a non-zero
>    return, confirm via `config_extract_frontmatter` that the block *parses* AND
>    the key is *absent* before inserting; otherwise propagate the failure. No
>    change to `config_set_frontmatter_field`'s contract.
> 2. **(Alternative)** Refactor `config_set_frontmatter_field` to surface distinct
>    `readonly E_FM_*` exit codes (backward-compatible — current callers only test
>    zero/non-zero) and branch on the absent code. List this refactor as an
>    explicit prerequisite change with its own tests.

Insert a new `key: value` line immediately before the closing `---`. Value passed
via the environment (injection-safe), **the same integrity re-check the replace
path runs (candidate frontmatter still parses AND the field reads back as the
value), run after the insert too** — factor that check out of
`config_set_frontmatter_field` so both branches share one implementation. The
inserted line is written **unquoted** (matching today's identifier writeback) so
the shared read-back equality check holds; if a quoted value is ever written, the
inserted-line formatting must match the read-back normalisation. Same
`atomic_write`. Insert only inside the frontmatter range; never the body.

```bash
# Upsert a top-level frontmatter field: replace if present, else insert before
# the closing `---`. Same injection-safety + integrity guarantees as
# config_set_frontmatter_field. Fails closed on malformed/unclosed/duplicate.
config_upsert_frontmatter_field() {
  local file="$1" key="$2" value="$3"
  if config_set_frontmatter_field "$file" "$key" "$value"; then
    return 0  # replaced in place
  fi
  # Non-zero is a COLLAPSED code (no/unclosed frontmatter, field-absent, or
  # duplicate). Re-detect "genuinely absent in a parseable block" before
  # inserting; every other condition must propagate, leaving the file
  # byte-unchanged — fail closed. (Option 2 instead branches on a distinct
  # E_FM_FIELD_ABSENT once config_set_frontmatter_field surfaces it.)
  local fm
  if ! fm="$(config_extract_frontmatter "$file" 2>/dev/null)"; then
    return 1  # not well-formed (no / unclosed frontmatter) → do NOT insert
  fi
  # Presence check uses the SAME `^key:` anchoring as config_set_frontmatter_field's
  # kpat, so a duplicate/present key reads as present and propagates (fail closed).
  if printf '%s\n' "$fm" | grep -q "^${key}:"; then
    return 1  # key present (e.g. duplicate) → not an insert case
  fi
  # Genuinely absent in a well-formed block → insert "key: value" before the
  # closing `---` (awk: track frontmatter range; value via ENVIRON), then run the
  # SAME shared integrity re-check before atomic_write; on failure leave the file
  # untouched and return non-zero.
  ...
}
```

> No shared single-field getter exists today (`config-common.sh` exposes
> `config_extract_frontmatter` and `config_set_frontmatter_field`; the only
> per-field readers are private — `_read_frontmatter_scalar`, `_linear_fm_field`).
> The presence check above therefore greps the extracted block directly. If a
> reusable `config_get_frontmatter_field` getter is preferred, add it to
> `config-common.sh` as an explicit sub-change with its own tests and use it here
> and in the Phase 3 classifier.

#### 2. Split the Linear flow into create-and-return vs writeback

**File**: `skills/integrations/linear/scripts/linear-create-flow.sh`
**Changes**:
- Add a no-file create mode that takes explicit content and prints **exactly the
  bare validated identifier on stdout** with **no writeback** — e.g. `--title
  TEXT --body-file PATH` (mirrors Jira's `jira-create-flow.sh` shape). The
  existing payload-build → GraphQL `issueCreate` → identifier-validation sequence
  is reused unchanged; only the input source and the final writeback differ.
- **Pre/post-create exit-code distinction (required by Phase 4's retry safety):**
  the no-file mode must use *distinct* exit codes for failures that occur
  **before** the `issueCreate` mutation is sent versus **at or after** it. The
  boundary is drawn conservatively around the **ambiguous window**: only failures
  *provably before transmission* — argument/validation errors, auth failures
  resolved pre-send, DNS resolution failures, connection-refused — are
  retryable-pre-create. **Any failure where the request was, or may have been,
  transmitted — including read timeouts and connection resets after the request
  body is sent — maps to post-create** (a remote issue may already exist; **not**
  safe to retry), because a mid-flight timeout is indistinguishable from a
  successful create whose response was lost. The dispatcher (Phase 4 §1) maps
  these into its taxonomy.
- Keep the file-first invocation working, but route its writeback to
  `external_id` via `config_upsert_frontmatter_field`, and change the
  already-synced guard to test **`external_id`** for presence instead of testing
  `work_item_id` against `LINEAR_IDENTIFIER_RE`.
- **This is a semantic shift, not just a field rename**: `E_CREATE_ALREADY_SYNCED`
  now fires on *presence* (any non-empty `external_id`), where it previously fired
  on *remote-format* `work_item_id`. Apply the same quote/whitespace trimming the
  current guard uses (`linear-create-flow.sh:136-141`): **"non-empty `external_id`"
  means length > 0 after stripping surrounding quotes and whitespace**, so a
  `external_id: ""` / quote-only / whitespace-only value reads as *unsynced*, not
  already-synced. This exact normalisation is the shared definition used by the
  Jira guard (Phase 2 §1) and the `/list-work-items` classifier (Phase 3 §2).
- Rename the writeback failure message: it now names `external_id`.

> **Legacy `work_item_id` migration — out of scope and unnecessary.** The Linear
> integration is **unreleased**, so no work items in the wild carry a remote key
> under `work_item_id`. The presence-based guard and classifier therefore have no
> legacy-synced items to misclassify or re-push; no transitional `work_item_id`
> read and no data migration are needed (see Migration Notes).

#### 3. Linear create skill

**File**: `skills/integrations/linear/create-linear-issue/SKILL.md`
**Changes**: Replace every `work_item_id` reference (`:10`, `:53`, `:65`, `:71`,
`:88`, `:94`) with `external_id`. **Note `:10` is inside the YAML `description`
frontmatter** — the discovery-bearing summary that drives skill matching — so
reword the description prose, not only the body references, and confirm no
`description`/`argument-hint` text still mentions `work_item_id` after the edit
(the Phase 1 `! grep work_item_id` success-criterion check covers this). The skill
reads the work-item file, previews, confirms (unchanged y/n gate), calls the
create-and-return primitive, then writes `external_id` back via the helper. The
loud writeback-failure guidance now tells the user to set `external_id:
<IDENTIFIER>` by hand.

#### 4. Exit codes

**File**: `skills/integrations/linear/scripts/EXIT_CODES.md`
**Changes**: This table is **derived** — the `readonly E_*=NN` constants and their
adjacent comments in `linear-create-flow.sh` are the documented source of truth.
Update **both sides in lockstep**: the script-side `readonly E_*`
constants/comments AND the `101`/`102`/`107` descriptions, all referencing
`external_id` (absent / already-synced-by-presence / writeback-failed) instead of
`work_item_id`. Also update the `100–109` range-summary note (it currently
mentions "`work_item_id` writeback codes"). **Note: there is no automated
value-parity check in `test-linear-create.sh` today** — it asserts behavioural
exit codes and stderr symbols, not table↔constant equality. Do not rely on a
non-existent guard: either *add* a parity assertion as new work in this phase
(grep the `readonly E_*=NN` declarations, parse the table, assert equality), or
rely explicitly on the `! grep work_item_id skills/integrations/linear/`
success-criterion (which must cover `EXIT_CODES.md`) to catch stale prose. State
which.

#### 5. Documentation of the id/external_id relationship

**Files**: `templates/work-item.md` (`external_id` comment), `skills/config/configure/SKILL.md` (work section)
**Changes**: State that `id` is the local own-identity and `external_id` is the
remote tracker's identifier; they **may coincide** (Jira/Linear) or be
**independent** (Trello); `external_id` is the per-item mapping and its presence
is the synced signal.

### Success Criteria

#### Automated Verification

- [ ] New helper unit tests pass: `bash scripts/test-config.sh` (cover
      upsert-replace, upsert-insert-before-closing-fence, **fail-closed on
      no-frontmatter / unclosed-frontmatter / duplicate-key (insert NOT
      attempted, error propagated, file left byte-unchanged)**, inserted line
      lands inside the frontmatter range and the block still parses,
      injection-safe value — `&`, `/`, `\`, embedded newline rejected).
      `test-config.sh` runs in CI today.
- [ ] Linear create tests pass: `bash skills/integrations/linear/scripts/test-linear-create.sh`
      (re-point existing cases — the writeback case, the already-synced case
      (3), and the loud-writeback-failure case (5) — from `work_item_id` to
      `external_id`. The **byte-identical-remainder** case must exclude
      `external_id` from the comparison **and start from a fixture with no
      `external_id` line**, so it proves *insertion* (not replacement); it must
      **additionally assert the inserted `external_id:` line falls inside the
      frontmatter fence** (remainder-equality alone does not check insertion
      position). Case 5
      must keep a **deterministic, permission-independent** fail trigger (e.g. a
      duplicated `external_id` line that the upsert helper propagates as the
      duplicate-key failure), not a filesystem-permission trigger. Add
      already-synced-via-`external_id` (incl. quote-only `""` → *unsynced*),
      writeback-insert, **no-file create-and-return mode (asserts bare identifier
      on stdout, GraphQL body captured, input file byte-unchanged)**, and
      **pre-create vs post-create exit-code (incl. a response-dropped-after-send
      case → post-create)** cases). Runs in CI only once wired — see Phase 0.
- [ ] No remaining `work_item_id` remote-key reference anywhere under Linear,
      **including `EXIT_CODES.md`**:
      `! grep -rn "work_item_id" skills/integrations/linear/` (other than
      historical mentions, if any, intentionally annotated)
- [ ] Shell lint/format clean: `mise run scripts:check`
- [ ] Full read-only gate: `mise run check`

#### Manual Verification

- [ ] `/create-linear-issue <file>` on an item with no `external_id` creates the
      issue and writes `external_id: <KEY>` (line inserted)
- [ ] Re-running on the now-synced item is refused (already-synced via
      `external_id`)
- [ ] Writeback-failure path prints the loud non-idempotent guidance naming
      `external_id`

---

## Phase 2: Jira — harmonise to the work-item-file contract

### Overview

Give `create-jira-issue` a work-item-file-driven mode mirroring Linear's
contract, with caller-side `external_id` writeback. Preserve Jira's existing
flag-driven mode additively. After this phase both integrations expose the same
user-facing create contract.

### Changes Required

#### 1. Jira create skill — add work-item-file mode

**File**: `skills/integrations/jira/create-jira-issue/SKILL.md`
**Changes**: Add a mode that accepts a work-item-file path: read `title`→
`--summary`, body→`--body-file`, resolve `--project` and map `kind`→issue-type
(`story`→Story, `bug`→Bug, `task`/`spike`→Task, `epic`→Epic; default Task) via the
**non-optional** read-only resolver script (Phase 2 §2) — the single source of
truth shared with the Phase 4 dispatcher's Jira branch, so the two entry points
can never map the same work item to different types. The preview **states both
the resolved issue type and the resolved project plus which source it came from**
(`work.default_project_code` vs the project code embedded in `id`), and when a
`kind` falls through to the default it says so explicitly (e.g. `kind "spike" →
Task (default)`) so the confirm gate is informed. **Resolve and validate the
project *before* presenting the push/confirm** — an unresolvable project (e.g. a
bare-numeric `id` with no `work.default_project_code`) is a pre-create,
non-retryable failure surfaced before the create, naming `work.default_project_code`,
not a post-accept error. After the confirmed create, parse `.key` from the
response and write it to `external_id` via `config_upsert_frontmatter_field`.
**Keep** the existing flag-driven invocation as an alternate mode (no capability
removed). Add an already-synced guard that refuses when the file already has a
non-empty `external_id`, using the **same quote/whitespace normalisation defined
in Phase 1 §2** (`""` / whitespace-only → *not* synced → create proceeds).

#### 2. Jira resolver + bare-identifier extraction

**Files**: `skills/integrations/jira/scripts/jira-create-flow.sh`,
`skills/integrations/jira/scripts/jira-resolve-fields.sh` (new, read-only)
**Changes**: `jira-create-flow.sh` needs no change for create-and-return (it
already prints `{id,key,self}` with no file I/O). Add a **non-optional** small
read-only resolver `jira-resolve-fields.sh` that owns the kind→issue-type mapping
and the `--project` resolution (from `work.default_project_code` or the `id`
project code), so both the user-facing skill mode (§1) and the dispatcher's Jira
branch (Phase 4 §1) call **one** implementation — no prose-duplicated mapping that
can drift. To keep the dispatcher's contract uniform (Phase 4 §1), the `.key`
extraction from the `{id,key,self}` response lives on the Jira side as a **thin
post-create wrapper distinct from the field resolver** (response parsing is a
different responsibility from pre-create field resolution — keep
`jira-resolve-fields.sh` purely a resolver), so every integration the dispatcher
calls returns a **bare validated identifier on stdout** and the dispatcher carries
no per-tracker response parsing.

#### 3. Exit codes + tests

**Files**: `skills/integrations/jira/scripts/EXIT_CODES.md`, `skills/integrations/jira/scripts/test-jira-create.sh`
**Changes**: Assign any new resolver/guard exit codes from the **reserved
`108–109` band** (Jira already owns the full `100–107` band, `107` =
`E_CREATE_BAD_ASSIGNEE`) — do not reuse or collide with existing create codes —
and update both the `## Codes` table and the `## Phase 4 namespace summary` in
`EXIT_CODES.md`. Add tests for: kind→issue-type mapping (each kind + unknown→Task);
project resolution from `work.default_project_code` and from the `id` project code,
**plus the unresolvable-project pre-create failure**; the **already-synced refusal**
(file with a non-empty `external_id`); and **end-to-end writeback** (a `.key` from a
mock 201 lands in `external_id` on the work-item file — not just the shared-helper
unit test, which exercises the helper in isolation rather than the Jira flow's
invocation of it).

### Success Criteria

#### Automated Verification

- [ ] Jira create tests pass: `bash skills/integrations/jira/scripts/test-jira-create.sh`
      (runs in CI only once wired — see Phase 0)
- [ ] kind→issue-type mapping covered (each kind → expected type; unknown →
      Task), via the shared resolver
- [ ] Project resolution covered (from config, from `id` project code, and
      unresolvable → pre-create failure naming `work.default_project_code`)
- [ ] Jira already-synced refusal covered (non-empty `external_id`; `""` →
      proceeds)
- [ ] End-to-end Jira writeback covered (`.key` from a mock 201 lands in
      `external_id` on the work-item file)
- [ ] New Jira exit codes drawn from the reserved `108–109` band; `EXIT_CODES.md`
      Codes table + Phase 4 namespace summary updated
- [ ] Shell lint/format clean: `mise run scripts:check`
- [ ] Full read-only gate: `mise run check`

#### Manual Verification

- [ ] `/create-jira-issue <work-item-file>` resolves project + type, previews,
      creates, and writes `external_id: <KEY>` back
- [ ] The legacy flag-driven form (`--project --type --summary …`) still works
      unchanged
- [ ] Both create skills now read/preview/confirm/create/write-back identically
      in shape

---

## Phase 3: `/list-work-items` sync-status labels

### Overview

Add the `work.integration` gate, read `external_id`, classify presence-based, and
render a markdown-native label (glyph + text) through an extensible
`status → {label, glyph}` lookup. Fully independent of the other phases.

### Changes Required

#### 1. Config gate

**File**: `skills/work/list-work-items/SKILL.md`
**Changes**: Add a `!`-preprocessor read of
`config-read-work.sh integration` alongside the existing reads (`:24-26`). Branch
on the **empty string**: empty → render exactly as today (no label); non-empty →
render the sync column/label.

#### 2. Extend the frontmatter scan

**File**: `skills/work/list-work-items/SKILL.md` (Step 2, `:181-190`)
**Changes**: Add `external_id` to the parsed fields.

**Reader choice.** Two readers are available and the plan must commit to one as
authoritative (do not split the field across both):
- **(Preferred)** Fold `external_id` into the existing single-pass `awk`
  frontmatter scan (`:181-190`) that already emits every frontmatter line per
  file — the field is in that stream at zero extra process cost — and reconcile
  AC #4's "via `work-item-read-field.sh`" wording in the flagged follow-up
  `/update-work-item`.
- **(AC-literal)** Read via `work-item-read-field.sh external_id <file>` (one
  subshell per item; see Performance Considerations). Note it has **no
  `id`↔`external_id` bridge** (unlike `id`/`work_item_id`), so an absent
  `external_id` is never silently substituted.

**Classification semantics (apply to whichever reader is authoritative).** The
scan's Step 2 already validates each file's frontmatter and **skip-and-warns**
malformed / no-frontmatter files; `external_id` is read only for files that
*passed* that validity check. For such a file: **field absent (or the accessor's
exit 1) means *unsynced*, not an error**; a present value that normalises to empty
(apply the Phase 1 §2 normalisation — strip surrounding quotes + whitespace — **to
the reader's output**, since `work-item-read-field.sh` only strips one quote pair
and outer whitespace) also means unsynced; a non-empty normalised value means
synced. A read failure on a file Step 2 already flagged stays a **skip**, not an
unsynced row. The filename remains the authoritative displayed ID (unchanged).

#### 3. Status lookup + rendering

**File**: `skills/work/list-work-items/SKILL.md` (Step 4, `:220-262`)
**Changes**: Define a data-driven lookup, e.g.:

```
sync status → { label text, glyph }
  synced   → { "synced",   "🟢" }
  unsynced → { "unsynced", "⚪" }
```

Render the label by a single lookup consulted once per item (the seam 0051
extends — adding a `locally-modified`/`conflict` entry yields a rendered label
with no call-site edit). The label is **markdown-native** (glyph + text), *not*
ANSI: `/list-work-items` output is a markdown table emitted into the conversation
by the model, never written to a TTY, so escape codes would surface as literal
`\033[…]` text. The codebase has no ANSI precedent; do not introduce one here. Add
a **Sync** column to the default table (suppressed when integration unset, per the
existing all-`—` suppression rule) and append the label to the
`canonical-tree-fence` per-item line (see Change 4 for the shared-fence
constraint). The two states **must differ in both glyph and text** so the signal
survives glyph-blind / monochrome contexts. Centralise the `status → {label,
glyph}` table in one place (a small read-only helper under the already-allowed
`skills/work/scripts/*`, or a single SKILL.md lookup block) so the two render
surfaces — table and tree fence — share one source rather than duplicating the
glyph vocabulary.

#### 4. Tree fence — do **not** sync-label the shared canonical fence

**File**: `skills/work/list-work-items/SKILL.md` `canonical-tree-fence` block;
constraint enforced by `scripts/test-hierarchy-format.sh`.
**Constraint**: `scripts/test-hierarchy-format.sh` (in the CI-wired `scripts`
subtree) asserts the `canonical-tree-fence` block is **byte-for-byte identical**
between `list-work-items/SKILL.md` and `refine-work-item/SKILL.md`. The fence is
a *static prose example*, not rendered output, and `refine-work-item` has no
`work.integration` gate — so a conditional sync label cannot live inside the
shared fence without either breaking the equality test or wrongly leaking a sync
label into `refine-work-item`.
**Decision**: Keep the shared `canonical-tree-fence` **label-free** — it stays the
integration-agnostic baseline, and the byte-identical assertion is preserved
unchanged. Describe sync-label rendering in the tree (hierarchy) view in prose
adjacent to the fence, and — if an illustrative example is warranted — use a
**separate, clearly non-canonical** example block (outside the
`canonical-tree-fence` markers) that the equality test does not cover. The
synced/unsynced labels are still verified end-to-end by the manual checks and the
work-item-script tests, not by mutating the shared fence.

### Success Criteria

#### Automated Verification

- [ ] `scripts/test-hierarchy-format.sh` still passes — the shared
      `canonical-tree-fence` byte-identical assertion (list-work-items ≡
      refine-work-item) is preserved (the sync label is NOT added to the shared
      fence, per Change 4)
- [ ] Work-item script tests pass: `bash skills/work/scripts/test-work-item-scripts.sh`
      (these run in CI only once the suite is wired — see Phase 0)
- [ ] Status lookup defines distinct glyph **and** distinct text for synced vs
      unsynced (signal survives monochrome / glyph-blind rendering); no ANSI
      escape codes are emitted
- [ ] Full read-only gate: `mise run check`

#### Manual Verification

- [ ] Integration unset → output identical to today (no Sync column, no label)
- [ ] Integration set, no baseline → every item shows exactly **synced** or
      **unsynced**; the glyph + label text render correctly in the conversation
      markdown table (no literal `\033[…]` sequences)
- [ ] An item with `external_id: "PROJ-0042"` and an item with `id: "PROJ-0042"`
      but no `external_id` classify as synced and unsynced respectively (no
      format-rule misclassification)
- [ ] An `external_id: ""` (quoted-empty) item classifies as **unsynced** (not
      synced) — the normalisation from Phase 1 §2 holds
- [ ] Hierarchy mode shows labels inline in the tree

---

## Phase 4: `/create-work-item` push-on-accept

### Overview

After the Step 4 draft is approved, offer an interactive push. Route to the
configured integration's create-and-return primitive via a new dispatcher, then
defer-write `external_id` into the single file write. Retry once, then fall back
to local save; decline → local save; unbuilt trackers → clean message + local
save. No file exists until push succeeds, the user declines, or local fallback is
confirmed.

### Changes Required

#### 1. Push dispatcher

**File**: `skills/work/scripts/work-item-create-remote.sh` (new)
**Changes**: `work-item-create-remote.sh --integration <sys> --title TEXT
--body-file PATH [--kind KIND]`. Routes on `<sys>`: `linear` → the Phase 1
create-and-return mode; `jira` → the Jira create path + resolver (Phase 2 §2),
which itself returns a bare identifier; `trello`/`github-issues` → the
not-available code. Internally execs the integration scripts (it lives under the
already-allowed `skills/work/scripts/*` prefix, so no `allowed-tools` widening);
this dispatcher is the **single sanctioned `work` → `integrations` bridge** and
its dependence on the integration scripts' invocation signatures is a tested
internal contract (the dispatcher tests are what catch a future
relocation/rename).

**Normalised contract (so the caller never branches on tracker-specific output or
codes):**

- **stdout**: on success, *exactly the bare validated identifier* for every
  integration. The dispatcher carries **no** per-tracker response parsing — Linear
  emits the bare identifier already, and the Jira `.key` extraction is pushed down
  to the Jira side (Phase 2 §2). **Identifier *format* validation is per-tracker**:
  each integration validates its own native shape (Linear's `^[A-Z][A-Z0-9]*-[0-9]+$`,
  Jira's `PROJ-123`, and — for 0049/0050 — Trello's opaque card ID and GitHub's
  `owner/repo#42`). The dispatcher must **not** apply a single tracker's regex to
  all; it performs only a tracker-agnostic safety check on the returned string
  before passing it through. Scope that check narrowly to what actually breaks an
  unquoted YAML scalar writeback — reject control characters, newlines, a leading
  `---`, and a leading-space-`#` comment trigger — and **explicitly permit `/`,
  `#`, and `@` mid-token**, since GitHub (`owner/repo#42`) and Trello identifiers
  legitimately contain them; a blanket "no special characters" filter would
  re-break 0049/0050.
- **exit-code taxonomy** (a dispatcher-owned namespace that maps each
  integration's native codes into these — documented in an `EXIT_CODES.md` for
  `skills/work/scripts/`):
  - `0` — success; identifier on stdout.
  - *retryable-transport* — failure provably **before** the remote mutation was
    sent (arg/validation/auth/connect-refused). **Safe to retry.**
  - *terminal-post-create* — failure **at or after** the mutation (request sent;
    response/identifier lost or invalid). A remote issue **may already exist** —
    **NOT safe to retry.**
  - *not-available* — `trello`/`github-issues`: no create path yet.
  - *unrecognised* — any `<sys>` not in `{linear, jira, trello, github-issues}`,
    or empty. **Fail closed** (never guess/route ambiguously).
- The integration error text is surfaced on stderr alongside the mapped code.
- **Single source of truth for the active integration**: the `--integration`
  argument must be sourced from the *same* `config-read-work.sh integration` read
  as the gate (one resolution per invocation) — never a separately-derived or
  caller-guessed value — so "which tracker is active" cannot diverge between the
  gate and the route. A dispatcher test asserts the routed tracker matches the
  configured integration.
- **Codes pinned + documented**: assign the four outcomes concrete integers in a
  declared band and declare them as `readonly E_*=NN` constants (the source-of-
  truth idiom the Linear/Jira `EXIT_CODES.md` files use), recorded in the new
  `skills/work/scripts/EXIT_CODES.md`.

#### 2. Config gate + push offer

**File**: `skills/work/create-work-item/SKILL.md`
**Changes**:
- Add a `!`-preprocessor read of `config-read-work.sh integration` near the
  existing reads (`:23-31`), applying the **integration-configured convention**
  (see "Integration gate" note below): *configured* iff the output is non-empty
  (the script exits 0 with an empty line when unset — branch on the string, not
  the exit code).
- After Step 4 approval and **before** the Step 5 write, if integration is set,
  present the push offer using the fail-safe y/N gate copied from `:545-551`
  (exactly `y`/`Y` proceeds; anything else → decline). Preview the target tracker,
  the title, and the resolvable target fields: for Jira the resolved issue type
  **and project + source** (per Phase 2 §1); for Linear the team/issue context the
  flow resolves (or state explicitly that Linear's create takes no such
  user-resolvable fields, if so) — so both trackers present a comparably
  informative preview before the gate.
- **State both outcomes inline** so the single keystroke maps to an understood,
  non-destructive result and the second consecutive gate (draft-approval, then
  this) does not read as a trap — e.g. `Push to <tracker> now? [y/N]  (y = create
  remote issue + save locally; anything else = save locally only, unsynced)`.
  Since `/sync-work-items` (0051) is **not yet built**, the decline / fallback
  messaging must not instruct the user to run it as if it exists — phrase the
  "push later" recovery as running the standalone `/create-<tracker>-issue <path>`
  skill on the saved file (which now shares the same `external_id` contract), and
  only mention `/sync-work-items` as the future batch path.

#### 3. Push state machine + defer-write

**File**: `skills/work/create-work-item/SKILL.md` (Step 5)
**Changes**: Allocate the local ID (`work-item-next-number.sh`) and build the
frontmatter block in memory as today. The branching keys off the dispatcher's
**exit-code taxonomy** (Phase 4 §1), so the prose state machine reduces to a flat
decision per code rather than re-deriving tracker-specific logic. **Retry is only
ever safe for `retryable-transport`** — remote create is non-idempotent, and
blindly retrying a `terminal-post-create` failure would create a *duplicate*
issue (the exact mode the existing Linear `E_CREATE_WRITEBACK_FAILED` stance is
designed to prevent).

Explicit outcome table:

| Trigger | Dispatcher result | Action |
|---|---|---|
| **Accept** | `0` (success) | Substitute the returned key into the in-memory `external_id` line (add it if omit-by-default dropped it), then Write **once**. |
| Accept | `retryable-transport` | Offer **1 retry**. The retry's result **re-enters this table**: `0` → write once with `external_id`; `terminal-post-create` → the terminal-post-create row (no further retry, loud guidance); `retryable-transport` again → save locally **without** `external_id`, inform the user it can be pushed later. |
| Accept | `terminal-post-create` | **Do NOT retry.** Save locally without `external_id` (on confirmation), then print loud non-idempotent guidance **naming the saved file's absolute path**: a remote issue may already have been created — *do not blindly re-run `/create-work-item`*; check the tracker, and if the issue exists, reconcile by running `/create-<tracker>-issue <saved-path>` (which performs the writeback through the helper and is guarded against double-create) **or** set the top-level `external_id: <KEY>` frontmatter field on that file by hand. |
| Accept → success, **but the single Write fails** | (local) | Remote issue exists and the identifier is known, but nothing is on disk. Print the **same loud non-idempotent guidance**, echoing the returned identifier and the intended file path — do **not** silently retry the create (a re-run would duplicate). Recovery: re-run `/create-<tracker>-issue <path>` against the recreated draft, or record `external_id: <KEY>` by hand. |
| Accept | `not-available` (trello/github-issues) | Inform the user create support for `<sys>` is not built yet (cite 0049/0050), reassure the item is saved locally and will sync once support lands; save locally without `external_id`. |
| Accept | `unrecognised` | Fail closed: report the misconfigured `work.integration` value; save locally without `external_id`. |
| **Decline** | — | Write locally without `external_id`. |

The single Write remains the only disk mutation on the success path;
`external_id` is substituted pre-write, sidestepping the replace-only limitation
entirely (no helper needed on this path). The "no file exists until one of
success / decline / confirmed-local-fallback resolves" invariant holds across
every row above — including the Write-failure row, where the failure leaves no
partial file and the loud guidance hands recovery to the user.

#### 4. Tests

**File**: `skills/work/scripts/test-work-item-create-remote.sh` (new, **`chmod
+x`** — `run_shell_suites` skips non-executable `test-*.sh` silently)
**Changes**: Cover, using the existing Linear/Jira mock servers
(`test-helpers/mock-*-server.py`):
- dispatch routing (linear/jira → create path; trello/github-issues →
  not-available; unrecognised/empty `<sys>` → fail-closed `unrecognised`);
- bare-identifier pass-through (stdout is exactly the identifier for both linear
  and jira — no JSON leakage);
- **exit-code taxonomy mapping**: a pre-mutation failure surfaces as
  `retryable-transport` and a post-mutation/identifier-lost failure surfaces as
  `terminal-post-create` (drive both via mock-server behaviours), since the
  retry-safety decision depends entirely on this distinction.

**State-machine coverage** (the §3 decision table): the create-work-item `evals/`
harness is **model-driven and does not gate in CI** (no `mise` task, not in any
`test:*` rollup), so the safety-critical retry/fallback logic must **not** rely on
it alone. **Extract the code→action mapping into a thin, deterministic testable
seam** — e.g. `skills/work/scripts/work-item-push-decide.sh` that emits the next
action (`write-once` / `retry` / `local-save` / `loud-terminal`) from its inputs.
Its input must cover **every** row of the outcome table, so it takes not just the
dispatcher outcome (exit code / attempt count) but also a **post-dispatcher
write-result flag** — the `Accept → success but the single Write fails` row is a
*local* failure after the dispatcher returned `0`, which no dispatcher exit code
can express; the seam maps that (dispatcher-`0` + write-failed) input to
`loud-terminal`. Unit-test **every row** in the (now CI-wired) dispatcher/work
suite: accept-success, retryable-then-retry-success, retry→terminal-post-create,
retry-exhausted, terminal-post-create (no retry), Write-failure-after-success,
decline, not-available, unrecognised. The SKILL.md prose then just invokes the seam and renders
its decision, keeping the model out of the branch that prevents duplicate issues.
The `evals/` cases remain as supplementary end-to-end checks of the rendered UX,
not the gating safety test.

### Success Criteria

#### Automated Verification

- [ ] Dispatcher tests pass: `bash skills/work/scripts/test-work-item-create-remote.sh`
      (executable; runs in CI once wired — see Phase 0)
- [ ] Dispatcher routes each `work.integration` value correctly, returns the
      not-available code for trello/github-issues, and **fails closed** on an
      unrecognised/empty `<sys>`
- [ ] Dispatcher maps pre-mutation failures to `retryable-transport` and
      post-mutation/identifier-lost failures to `terminal-post-create` (distinct
      codes)
- [ ] `create-work-item` eval suite passes (no regression to the
      no-integration path): the skill's `evals/` harness, with the §4 transition
      cases
- [ ] Shell lint/format clean: `mise run scripts:check`
- [ ] Full read-only gate: `mise run check`

#### Manual Verification

- [ ] Integration unset → `/create-work-item` behaves exactly as today (no push
      offer)
- [ ] Accept + success → file written **once** with the remote key in
      `external_id`; appears as **synced** in `/list-work-items`
- [ ] Accept + induced **transport** failure → 1 retry offered; on exhausted
      retry, local save (no `external_id`) + "push later via
      `/create-<tracker>-issue`" message
- [ ] Accept + induced **post-create** failure (response lost after the mutation)
      → **no retry**; loud non-idempotent guidance (don't re-run; a duplicate may
      exist) + local save
- [ ] Decline → file written with no `external_id` (unsynced)
- [ ] Mid-push (before resolve) no file exists in the work directory
- [ ] `work.integration: trello` → clean "not available" (citing 0049/0050) +
      local save
- [ ] `work.integration` set to a bogus value → fail-closed message + local save

---

## Testing Strategy

### Unit Tests

- `config_upsert_frontmatter_field`: replace-existing, insert-missing,
  fail-closed on malformed/unclosed frontmatter, injection-safe value
  (`&`, `/`, `\`, embedded newline rejected), integrity re-check.
- Linear create-and-return: identifier validation, already-synced via
  `external_id`, no-file mode prints bare identifier.
- Jira: kind→issue-type mapping (each kind + unknown→Task), project resolution
  from config and from the `id` project code.
- Dispatcher: routing per integration, not-available for unbuilt, **fail-closed
  on unrecognised `<sys>`**, bare-identifier pass-through, and the
  `retryable-transport` vs `terminal-post-create` exit-code mapping.
- list-work-items: presence-based classification (incl. quote-only `""` →
  unsynced); the shared `canonical-tree-fence` byte-identical assertion is
  **preserved** (the sync label is not added to the shared fence — Phase 3 §4).

### Integration Tests

- Linear/Jira create flows against the existing mock servers, asserting
  `external_id` is written (not `work_item_id`).
- `/create-work-item` push happy-path and retry/fallback via the dispatcher +
  mock server.

### Manual Testing Steps

1. Configure `work.integration: linear` (with `/init-linear` catalogue) in a
   scratch repo.
2. `/create-work-item` → accept push → confirm single write with `external_id`.
3. `/list-work-items` → confirm synced glyph + label text (markdown, no literal
   escape sequences).
4. Repeat declining the push → confirm unsynced.
5. Force a transport failure (bad token) → confirm retry-then-local-fallback.
6. Force a post-create failure (response dropped after the create) → confirm
   **no retry** and loud non-idempotent guidance (no duplicate issue).
7. Set `work.integration: trello` → confirm graceful not-available + local save.

## Performance Considerations

With the **preferred** reader choice (Phase 3 §2 — fold `external_id` into the
existing single-pass `awk` frontmatter scan, `:181-190`), `/list-work-items` adds
**no** extra per-file process: the field is parsed from the scan output the skill
already produces. Only the AC-literal alternative (reading via
`work-item-read-field.sh external_id <file>`) would add one subshell per item,
which is still negligible for realistic work-directory sizes (tens to low hundreds
of items). Either way, no measurable impact.

## Migration Notes

- No data migration. Existing items without `external_id` simply render as
  unsynced — correct (they were never pushed).
- **No legacy `work_item_id` remote keys exist in the wild.** The Linear
  integration (use C, the only writer of a `work_item_id` remote key) is
  **unreleased**, so no work item carries a remote-key `work_item_id`. The
  presence-based guard and classifier therefore have nothing to misclassify or
  re-push: the transitional-read and `work_item_id` → `external_id` migration
  concerns are moot. (The legacy *own-identity* `work_item_id`, use A, is a
  separate matter — still bridged and untouched; see "What We're NOT Doing".)

## References

- Work item: `meta/work/0047-core-skills-sync-integration.md` — **needs a
  follow-up `/update-work-item`** to reconcile: AC #4/#5 (drop the `^[0-9]+$`
  format rule for presence-based `external_id`); `work_item_id` wording →
  `external_id`; the "terminal ANSI colour output" wording (AC #3 / Assumptions)
  → a markdown-native glyph+text label, since `/list-work-items` output is
  rendered as conversation markdown, not a TTY; and AC #7's "run `/sync-work-items`
  later" → "push later via `/create-<tracker>-issue`", since `/sync-work-items`
  (0051) is unbuilt.
- Research: `meta/research/codebase/2026-06-15-0047-core-skills-sync-integration.md`
- Downstream: `meta/work/0051-sync-work-items-skill.md` — inherits the
  presence-based classification and the `status → {label, glyph}` slot; its
  `last-sync.json` will key by `external_id`.
- Precedents: `skills/integrations/linear/scripts/linear-create-flow.sh:197-213`
  (validate + writeback + loud fail), `skills/work/create-work-item/SKILL.md:545-551`
  (fail-safe y/N gate), `scripts/config-common.sh:122-209`
  (`config_set_frontmatter_field`).
- **Companion ADR (recommended): document the `id` / `external_id` / (retired)
  `work_item_id`-remote-key model.** This change leaves the system mid-narrowing
  — one field name (`work_item_id`) still carries two live roles (own-identity,
  foreign-reference) after the remote-key role moves to `external_id` — so a
  short ADR gives the deferred 0064 canonicalisation a stable reference point and
  aligns with the ADR initiative.

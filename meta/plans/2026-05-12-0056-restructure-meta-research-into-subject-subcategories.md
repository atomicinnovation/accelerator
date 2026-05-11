---
date: "2026-05-12T00:00:00+00:00"
type: plan
skill: create-plan
work-item: "0056"
status: accepted
---

# 0056 — Restructure `meta/research/` into Subject Subcategories — Implementation Plan

## Overview

Restructure the flat `meta/research/` directory into four subject subcategories
(`codebase/`, `issues/`, `design-inventories/`, `design-gaps/`), absorbing the
top-level `meta/design-inventories/` and `meta/design-gaps/` directories into
the research umbrella. Rename the corresponding `paths.*` keys, ship an
`accelerator:migrate` migration that rewrites userspace repos atomically, and
update every plugin surface (skills, agent, README, visualiser frontend,
configure UX) in lockstep.

Implementation is driven test-first: phases 1–6 introduce failing tests
before the code that makes them pass; phase 7 applies the resulting migration
to the plugin's own `meta/`; phase 8 hand-edits narrative surfaces the
inbound-link rewriter does not touch.

Plan review 1 (`meta/reviews/plans/2026-05-12-…-review-1.md`) shaped
four design decisions (D1–D4 below in "Design Decisions From Plan
Review") that materially affect Phases 3–8: how to handle value rename
under user overrides, the upgrade-without-migrate window, the
visualiser JSON wire-key surface, and template-key migration handling.

## Current State Analysis

- `meta/research/` is flat — 37 `*.md` files plus `.gitkeep`. Mixes codebase
  research outputs, idea/strategy notes, and historical design work.
- `meta/design-inventories/` is two directories-per-investigation (each with
  `inventory.md` + `screenshots/`) plus a stray `.DS_Store`.
- `meta/design-gaps/` contains a single `*.md` plus `.gitkeep`.
- `scripts/config-defaults.sh:26-62` defines `paths.research`,
  `paths.design_inventories`, `paths.design_gaps` via index-paired
  `PATH_KEYS`/`PATH_DEFAULTS` arrays. A parallel `DIR_KEYS`/`DIR_DEFAULTS` in
  `skills/config/init/scripts/init.sh:18-31` shadows these for `init`
  scaffolding (`config-read-path.sh:27-29` short-circuits when an explicit
  default is passed).
- `scripts/config-read-all-paths.sh:14` excludes `design_inventories` and
  `design_gaps` from the `accelerator:paths` block.
- Six SKILL.md sites invoke `config-read-path.sh research`:
  `research-codebase:23`, `research-issue:22`, `visualise:15`,
  `extract-adrs:25`, `init:21`, `extract-work-items:26`.
- One SKILL.md site invokes `config-read-template.sh research`:
  `research-codebase:128`. Template registry at
  `config-defaults.sh:64-71` lists `templates.research` alongside other
  bare-key templates (`plan`, `adr`, `validation`, `pr-description`,
  `work-item`).
- `agents/documents-locator.md` has two layers of hardcoding: an inline
  legend (lines 33-41) and a flat output template with `### Research
  Documents` (lines 71-97), plus "Historic intent" purpose hints (110-117).
- `skills/config/configure/SKILL.md` has two mirrored blocks — paths table
  (386-402) and example YAML (406-422). The YAML has already drifted:
  it omits `design_inventories`, `design_gaps`, and `integrations`.
- `skills/config/paths/SKILL.md:25-37` carries a hand-written legend that
  is *not* derived from `PATH_KEYS`.
- `README.md` references the legacy paths in seven places: line 42 (ASCII
  development-loop diagram), 47 (`research-codebase` narrative), 64
  (`research-issue` narrative), 84-95 (`meta/` table — three rows), 408-415
  (ASCII ADR-flow diagram), 579 (`meta/design-gaps/` narrative).
- `CHANGELOG.md:98,155,165,170` references the legacy paths in historical
  release notes.
- `skills/visualisation/visualise/frontend/src/api/types.ts:7,17,157,164`
  uses `'design-gaps'` / `'design-inventories'` as `docType` string IDs.
  These are logical artifact-kind identifiers, not paths.
- `skills/config/migrate/SKILL.md` lines 11-17 and 46-57 carry stale text
  referencing the pre-0003 state-file location `meta/.migrations-*`.
- The migration framework's discovery glob (`run-migrations.sh:92`) filters
  to `*.sh` at `-maxdepth 1`, so a sibling `fixtures/` directory inside
  `migrations/` would not interfere. Existing fixture convention is
  `skills/config/migrate/scripts/test-fixtures/<id>/` (chosen for 0004).
- The migration framework provides no structured-notification primitive;
  the AC #3 "Renamed paths.X → paths.Y …" line is plain `echo` and
  `run-migrations.sh:184` merges stdout/stderr.

### Key discoveries

- **DIR_DEFAULTS shadows PATH_DEFAULTS at init time.** `init.sh:33-39`
  passes `DIR_DEFAULTS[i]` explicitly to `config-read-path.sh`, which
  short-circuits the centralised lookup. Updating only `PATH_DEFAULTS`
  leaves fresh-init repos scaffolding the wrong directory names.
- **0003's `_move_if_pending` (lines 32-64) is the idempotent move helper**;
  plain `mv` (not `jj mv` / `git mv`) preserves VCS rename history because
  jj snapshots and git's similarity detection both work from filesystem
  state.
- **0003's `probe_paths_key` (lines 66-102)** is the column-0-anchored
  awk reader that lets a migration capture old path values *before* its
  own config rewrite mutates them — required by 0004 because Phase B
  (config rewrite) must happen before Phase C (inbound-link rewrite)
  can know the old/new path pairs to rewrite.
- **`accelerator:paths` block as scan corpus.**
  `scripts/config-read-all-paths.sh:24-33` emits `- key: value` bullets
  for every non-excluded `PATH_KEYS` entry. The 0004 migration parses
  this into a `SCAN_DIRS[]` array, satisfying AC #4's "union of paths
  surfaced by `accelerator:paths`" without hardcoding `meta/**/*.md`.

## Desired End State

After all phases land in a single atomic commit:

- The plugin's `meta/` tree has four research subcategories:
  `meta/research/codebase/` (37 ex-flat files), `meta/research/issues/`
  (empty + `.gitkeep`), `meta/research/design-inventories/` (two
  inventory directories), `meta/research/design-gaps/` (one gap file).
- No top-level `meta/design-inventories/` or `meta/design-gaps/`
  directories remain.
- `paths.research_codebase`, `paths.research_issues`,
  `paths.research_design_inventories`, `paths.research_design_gaps`
  exist in `config-defaults.sh`. The legacy `paths.research`,
  `paths.design_inventories`, `paths.design_gaps` keys are gone.
- `templates.codebase-research` replaces `templates.research`; the
  template file is renamed `templates/research.md` →
  `templates/codebase-research.md`.
- `research-codebase` writes to `paths.research_codebase`;
  `research-issue` writes to `paths.research_issues`. Four wider callers
  (`visualise`, `extract-adrs`, `init`, `extract-work-items`) read from
  `paths.research_codebase`.
- `documents-locator` renders four `### Research (codebase|issues|
  design-inventories|design-gaps)` output groups; its inline legend
  enumerates the four new keys; the "Historic intent" hint references
  the four research keys.
- A new `accelerator:migrate` migration
  (`0004-restructure-meta-research-into-subject-subcategories.sh`):
  - Moves legacy files into the new subcategory layout per D1:
    research files always nest into `${OLD_RESEARCH}/codebase/`;
    design-inventories and design-gaps move to the new default
    location **only if the user has no explicit override** for those
    keys (overrides are honored and files stay in place). VCS rename
    history is preserved.
  - Ensures every destination directory (`codebase/`,
    `design-inventories/`, `design-gaps/`, `issues/`) contains a
    `.gitkeep` after the move — preserved as an invariant across all
    meta dirs regardless of whether they are empty or populated. Sweeps
    `.DS_Store` siblings, and emits a diagnostic when a legacy
    directory cannot be removed because of unexpected residual files.
  - Rewrites the three legacy `paths.*` config keys + the
    `templates.research` key (D4) to their renamed forms per D1's
    value semantics: research_codebase appends `/codebase` to the
    user value; design_inventories/design_gaps preserve verbatim;
    templates.research preserves verbatim. Emits per-key notifications.
  - Refuses to proceed (with a clear diagnostic) when both a legacy
    key and its renamed form are present in config (mixed-state
    detection).
  - Rewrites inbound references inside markdown files under every path
    surfaced by `accelerator:paths`. Prefix matches are anchored on
    path-segment boundaries (followed by `/`, `"`, `'`, whitespace,
    `)`, `]`, `#`, or end-of-line) to avoid `meta/research-templates/`
    style false positives.
  - Performs an up-front collision check (across all planned moves)
    before any `mv` is executed; on collision, fails with per-conflict
    diagnostics and zero filesystem mutation.
  - Performs an up-front dirty-tree pre-flight covering the full
    `accelerator:paths` scan corpus (broader than the framework's
    default `meta/`+`.accelerator/` perimeter) and fails closed when
    no VCS is detected.
  - Is idempotent (second run is a no-op, including `jj diff` /
    `git status --porcelain` empty) and inherits the framework's
    dirty-tree guard.
- README narrative, tables, and ASCII diagrams reference the new
  subcategory paths. CHANGELOG has a new entry for the restructure
  (historical entries unchanged). `migrate/SKILL.md` stale text about
  `meta/.migrations-*` is fixed.

### Verification

- `accelerator:configure` paths reference and example YAML list the
  four new research keys plus `integrations` (drift fix).
- `accelerator:paths` block lists the four new keys; legacy keys absent.
- `accelerator:migrate` on a fresh userspace clone applies cleanly,
  prints notifications for renamed keys, and exits 0 on re-run.
- `bash skills/config/migrate/scripts/test-migrate.sh` exits 0 with
  all new test cases green.
- `bash scripts/test-config.sh` exits 0 with updated assertions green.
- `jj log --follow meta/research/codebase/<any-file>.md` shows history
  pre-dating the move.
- `grep -rn 'meta/research/[0-9]\|meta/design-inventories\|meta/design-gaps' README.md CHANGELOG.md` returns only entries
  under the new layout (or, for CHANGELOG, only historical entries
  intentionally left verbatim).

## What We're NOT Doing

- Subcategorising any other `meta/` directory (`notes/`, `work/`,
  `plans/`, `specs/`).
- Splitting top-level `research/` and `analysis/` (these are
  out-of-scope per the work item).
- Introducing a `meta/strategy/` home for external research.
- Building the forthcoming idea/concept research skill. The
  `paths.research_ideas` key and `meta/research/ideas/` directory
  are not part of 0056.
- Per-investigation directory promotion (a file growing into a
  directory mid-life).
- Renaming visualiser `docType` strings (`'design-gaps'`,
  `'design-inventories'`). These are API-level logical IDs, not paths;
  the Rust resolver behind them switches to read from
  `paths.research_design_*` but the surface IDs stay.
- Rewriting historical CHANGELOG entries. A new entry covers 0056;
  historical entries describe what shipped at the time and stay
  verbatim.
- Migrating 0002/0003 test fixtures to a different layout. 0004's
  fixtures use the existing `skills/config/migrate/scripts/test-fixtures/0004/`
  convention.

## AC Refinements Negotiated During Planning

Two AC items in the work item are superseded by planning decisions:

1. **AC #5 fixture location.** The AC mandates `fixtures/` sibling to
   the migration script. Adopting that location would be net-new
   framework infrastructure inconsistent with 0002/0003. 0004 uses the
   existing `skills/config/migrate/scripts/test-fixtures/0004/`
   convention. The work item AC text will be updated to match before
   merge.
2. **AC #13 `scripts/research-metadata.sh` literal check.** The script
   is at `skills/research/research-codebase/scripts/research-metadata.sh`
   (not under top-level `scripts/`). The AC refers to that file. No
   change in spirit, just file path clarification.

Additionally, 0056 lands two scope decisions outside the original AC:

3. **Template key rename.** `templates.research` → `templates.codebase-research`;
   template file `templates/research.md` → `templates/codebase-research.md`.
   Mirrors the path-key rename and the naming convention used by
   `templates.pr-description` and `templates.work-item`.
4. **Wider `config-read-path.sh research` callers.** `visualise:15`,
   `extract-adrs:25`, `init:21`, `extract-work-items:26` all rewrite
   to `research_codebase`. Each call site reads "where existing
   codebase research lives".
5. **`migrate/SKILL.md` stale text (lines 11-17, 46-57)** referencing
   `meta/.migrations-*` is fixed in this commit.
6. **`configure/SKILL.md` example YAML drift** — restore `integrations`
   plus add the four new research keys (the table-vs-YAML mirroring
   is fixed in the same edit).

## Design Decisions From Plan Review (review-1)

Plan review 1 surfaced four design questions whose answers materially
shape Phases 3–8. Recorded here so reviewers and implementers can
trace the decisions back to their rationale.

### D1 — Value rename strategy: "Honor independent overrides"

- **`paths.research` → `paths.research_codebase`**: Files always move
  from `${OLD_RESEARCH}/` into `${OLD_RESEARCH}/codebase/` (flat → nested
  structural change). The new key's value is `${OLD_RESEARCH}/codebase`
  — i.e. when the user has an explicit override (`docs/research`), the
  migration writes `paths.research_codebase: docs/research/codebase`
  (suffix appended to honor the user's intent that codebase research
  lives under their chosen `docs/research/`). When the user is on
  defaults, no explicit key is written and the new default
  (`meta/research/codebase`) resolves.
- **`paths.design_inventories` → `paths.research_design_inventories`**
  and **`paths.design_gaps` → `paths.research_design_gaps`**: If the
  user has an explicit override, value is preserved verbatim and files
  are **not moved** (just key renamed; the user's intentional placement
  is respected). If no override, files move from the legacy default
  (`meta/design-inventories/`, `meta/design-gaps/`) into the new
  default location (`meta/research/design-inventories/`,
  `meta/research/design-gaps/`), with no explicit key written.
- **Rationale**: research_codebase carries a structural transformation
  that requires the value to recompute; design-inventories/design-gaps
  are pure key renames if user overrode them.
- **Notification line**: `Renamed paths.<old> → paths.<new> (value:
  <oldval> → <newval>)` when value changes;
  `Renamed paths.<old> → paths.<new> (value preserved: <val>)` when
  value unchanged.

### D2 — Upgrade-without-migrate window: "Documentation only"

- No hard-block or per-skill pre-flight is added.
- The existing SessionStart migrate-discoverability hook continues to
  warn users that migrations are pending.
- The CHANGELOG entry (Phase 8) explicitly calls out the hazard: skills
  that previously wrote to `meta/research/` will produce different
  results until `accelerator:migrate` runs.
- `migrate/SKILL.md` is amended (Phase 8) to document the upgrade
  sequence: pull plugin → run `/accelerator:migrate` → resume normal
  workflow.

### D3 — Visualiser JSON wire keys: "Rename to research_codebase etc."

- `doc_paths.research` → `doc_paths.research_codebase` (and a new
  `doc_paths.research_issues`).
- `doc_paths.design_gaps` → `doc_paths.research_design_gaps`.
- `doc_paths.design_inventories` → `doc_paths.research_design_inventories`.
- Updates required (enumerated in Phase 8):
  - `skills/visualisation/visualise/scripts/write-visualiser-config.sh`
    — `abs_path` callers and JSON `--arg`/`jq` shape.
  - `skills/visualisation/visualise/server/src/docs.rs` —
    `config_path_key()` returned string constants.
  - `skills/visualisation/visualise/server/tests/fixtures/config.valid.json`
    and `config.optional-override-null.json` — JSON `doc_paths` block.
  - `skills/visualisation/visualise/server/scripts/test-launch-server.sh`
    — JSON wire-key strings.
  - `skills/visualisation/visualise/server/tests/common/mod.rs` — any
    fixture-key constants.
  - `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`
    — embedded path literals (re-check after main edits).
- `frontend/src/api/types.ts` `docType` strings (`'design-gaps'`,
  `'design-inventories'`) remain unchanged as documented in "What
  We're NOT Doing" — those are docType IDs, distinct from the
  `doc_paths` config wire keys decided here.

### D4 — Templates rename: "Extend 0004 migration to cover templates"

- Migration handles `templates.research` → `templates.codebase-research`
  in user config (`.accelerator/config.md` / `.accelerator/config.local.md`)
  using the same rewrite mechanism as the path keys.
- If the user has an explicit `templates.research` override pointing
  at a custom file path, the migration renames the key only (verbatim
  value preservation — there is no structural transformation analogous
  to research_codebase's flat→nested move).
- If the user has placed a custom `templates/research.md` under their
  configured `paths.templates`, the migration renames the file in
  place (lifting the same `_move_if_pending` helper used for path
  moves). No move if the file does not exist (default-layout case
  where plugin's `templates/codebase-research.md` is the source of
  truth).
- Notification line: `Renamed templates.research → templates.codebase-research (value preserved: <path>)`.

## Implementation Approach

Eight phases. Phases 1–6 are strict TDD (test before impl). Phase 7
applies the migration to the plugin's own `meta/` as an integration
test. Phase 8 hand-edits narrative surfaces the rewriter does not
reach (READMEs, source-code-shaped narrative, CHANGELOG, prebuilt
bundles).

Development happens on a feature branch with incremental commits per
phase to keep review tractable, then squashes into a single atomic
commit at merge per the work item's commit-atomicity requirement.

---

## Phase 1 — Config-layer foundations

### Overview

Introduce the four new path keys, retire the three legacy keys, and
update every config-layer registry that knows about them. This phase
establishes the contract every later phase consumes.

### Changes required

#### 1. Path-key registry

**File**: `scripts/config-defaults.sh`

In-place edit of `PATH_KEYS` (line 27, 39, 40) and `PATH_DEFAULTS`
(line 46, 58, 59). Append one new row to each:

```bash
PATH_KEYS=(
  "paths.plans"
  "paths.research_codebase"               # was: paths.research
  "paths.decisions"
  …
  "paths.research_design_inventories"     # was: paths.design_inventories
  "paths.research_design_gaps"            # was: paths.design_gaps
  "paths.global"
  "paths.research_issues"                 # new — appended
)

PATH_DEFAULTS=(
  "meta/plans"
  "meta/research/codebase"                # was: meta/research
  "meta/decisions"
  …
  "meta/research/design-inventories"      # was: meta/design-inventories
  "meta/research/design-gaps"             # was: meta/design-gaps
  "meta/global"
  "meta/research/issues"                  # new — appended
)
```

Also rename the template key (lines 64-71):

```bash
TEMPLATE_KEYS=(
  "templates.plan"
  "templates.codebase-research"           # was: templates.research
  "templates.adr"
  …
)
```

#### 2. Init scaffold registry

**File**: `skills/config/init/scripts/init.sh`

In-place edit of `DIR_KEYS`/`DIR_DEFAULTS` (lines 18-31). Apply the
same three renames + one addition. Order must remain index-paired with
the local arrays (not with `PATH_KEYS`).

#### 3. Documents-discovery exclusion list

**File**: `scripts/config-read-all-paths.sh`

Edit line 14:

```bash
EXCLUDED_KEYS=(tmp templates integrations)
```

(removing `design_inventories` and `design_gaps`).

#### 4. Hand-written path legend

**File**: `skills/config/paths/SKILL.md`

Edit lines 25-37. Replace the `research` bullet with four new bullets
(`research_codebase`, `research_issues`, `research_design_inventories`,
`research_design_gaps`), each with a one-line description matching the
configure paths table.

#### 5. Template file rename

`templates/research.md` → `templates/codebase-research.md`. Use `jj mv`
(or plain `mv` followed by a jj snapshot) so rename history is
preserved.

### Success criteria

#### Automated verification

- [ ] `bash scripts/test-config.sh` — exits 0 with new assertions for:
  - [ ] `PATH_KEYS` length is 17 (was 16; net +1 after 3 renames +
    1 add).
  - [ ] `PATH_KEYS` contains `paths.research_codebase`,
    `paths.research_issues`, `paths.research_design_inventories`,
    `paths.research_design_gaps`.
  - [ ] `PATH_KEYS` does not contain `paths.research`,
    `paths.design_inventories`, `paths.design_gaps`.
  - [ ] `PATH_DEFAULTS` for the four new keys are
    `meta/research/{codebase,issues,design-inventories,design-gaps}`.
  - [ ] `TEMPLATE_KEYS` contains `templates.codebase-research`, not
    `templates.research`.
  - [ ] `EXCLUDED_KEYS` in `config-read-all-paths.sh` is
    `(tmp templates integrations)`.
  - [ ] `DIR_KEYS`/`DIR_DEFAULTS` in `init.sh` match the path renames.
- [ ] `bash scripts/config-read-all-paths.sh` (in a fresh-config repo)
  emits bullet lines for the four new keys.
- [ ] `bash scripts/config-read-path.sh research_codebase` returns
  `meta/research/codebase`.
- [ ] `bash scripts/config-read-template.sh codebase-research`
  resolves to the renamed template file.

#### Manual verification

- [ ] Inspect `paths/SKILL.md` legend — 14 entries, no `research`
  bullet, four `research_*` bullets present.
- [ ] `jj log --follow templates/codebase-research.md` shows the
  pre-rename history.

---

## Phase 2 — Skill consumers, wider callers & template invocation

### Overview

Update every `config-read-path.sh research` and
`config-read-template.sh research` invocation in the plugin to point
at the new key names. Six SKILL.md sites + the template-key callers.

### Changes required

#### 1. Research skills

**File**: `skills/research/research-codebase/SKILL.md`
- Line 23: bare key `research` → `research_codebase`.
- Line 128: `config-read-template.sh research` →
  `config-read-template.sh codebase-research`.

**File**: `skills/research/research-issue/SKILL.md`
- Line 22: bare key `research` → `research_issues`.

#### 2. Wider callers

All point at `research_codebase` (each reads "where existing codebase
research lives"):

- `skills/visualisation/visualise/SKILL.md:15`
- `skills/decisions/extract-adrs/SKILL.md:25`
- `skills/config/init/SKILL.md:21`
- `skills/work/extract-work-items/SKILL.md:26`

#### 3. Test assertions in `scripts/test-config.sh`

Update lines 4361, 4379 (assertions on the two research SKILL.md
bang-call literals) and lines 5789-5792 (assertions on
`paths.design_inventories` / `paths.design_gaps` defaults) to match
the new key names and defaults.

### Success criteria

#### Automated verification

- [ ] `bash scripts/test-config.sh` — exits 0 with updated assertions
  green.
- [ ] `grep -rn "config-read-path.sh research[^_]" skills/ agents/` —
  returns no matches (every `research` literal has been replaced with a
  subcategory key).
- [ ] `grep -rn "config-read-template.sh research[^-]" skills/` —
  returns no matches.

#### Manual verification

- [ ] Spot-check each updated SKILL.md to confirm the substituted
  key is semantically correct for the surrounding instruction.
- [ ] Manually run `bash skills/research/research-codebase/SKILL.md`
  preprocessor (or invoke the skill) and confirm the resolved path
  resolves to `meta/research/codebase`.

---

## Phase 3 — Migration: filesystem moves

### Overview

Create the migration script's scaffolding plus its filesystem-move
phase, validated by test-migrate.sh test cases driving fixtures under
`scripts/test-fixtures/0004/`. Per D1, value semantics differ across
the three legacy keys: research_codebase always nests files under
`${OLD_RESEARCH}/codebase/`; design-inventories and design-gaps move
files **only when the user has no explicit override**.

### Changes required

#### 1. Test fixtures

**Directory**: `skills/config/migrate/scripts/test-fixtures/0004/`

Six fixture trees covering the value-semantics matrix plus edge
cases. Files marked `(N)` indicate non-`.md` siblings that must be
handled correctly (moved if part of an artifact, or surfaced via
diagnostic if unexpected).

```
test-fixtures/0004/
├── default-layout/                # all three keys at defaults
│   ├── meta/
│   │   ├── research/.gitkeep
│   │   ├── research/2026-01-01-example.md
│   │   ├── design-inventories/.DS_Store               (N) — macOS junk
│   │   ├── design-inventories/2026-05-06-x/inventory.md
│   │   ├── design-inventories/2026-05-06-x/inventory.md links to ../foo.md
│   │   ├── design-inventories/2026-05-06-x/screenshots/01.png
│   │   ├── design-gaps/.gitkeep
│   │   └── design-gaps/2026-05-06-x.md
│   └── .accelerator/config.md     # empty paths block — uses defaults
│
├── research-override-only/        # only paths.research overridden
│   ├── docs/research/2026-01-01-example.md
│   ├── meta/design-inventories/2026-05-06-x/inventory.md
│   ├── meta/design-gaps/2026-05-06-x.md
│   └── .accelerator/config.md     # paths.research: docs/research
│
├── all-overridden/                # all three keys overridden
│   ├── docs/research/2026-01-01-example.md
│   ├── assets/inv/2026-05-06-x/inventory.md
│   ├── gaps/2026-05-06-x.md
│   └── .accelerator/config.md     # paths.research: docs/research,
│                                  # paths.design_inventories: assets/inv,
│                                  # paths.design_gaps: gaps
│
├── partial-state/                 # half-migrated state
│   ├── meta/research/codebase/already-moved.md
│   ├── meta/research/2026-01-01-still-flat.md
│   └── .accelerator/config.md     # paths.research absent
│                                  # paths.research_codebase: meta/research/codebase
│
├── mixed-config/                  # both legacy and renamed key present
│   ├── meta/research/foo.md
│   └── .accelerator/config.md     # paths.research: meta/research
│                                  # paths.research_codebase: meta/research/codebase
│
├── local-config-only/             # override lives in config.local.md only
│   ├── docs/research/foo.md
│   ├── .accelerator/config.md     # empty paths block
│   └── .accelerator/config.local.md # paths.research: docs/research
│
└── inbound-corpus/                # used by Phase 5 fixtures (see Phase 5)
    └── …
```

Plus a brand-new `setup_legacy_research_repo_*` helper per fixture
shape that copies the fixture into a fresh temp dir and initialises
a `.jj` (and parallel `.git` for the git-specific test) working copy.

#### 2. Test cases in `skills/config/migrate/scripts/test-migrate.sh`

Add test cases covering the value-semantics matrix, dotfile handling,
mixed-state refusal, and per-VCS rename detection:

**D1 value semantics — research_codebase always nests:**

- "0004: default-layout — flat research files move to meta/research/codebase/"
- "0004: default-layout — meta/research/.gitkeep stays in place (NOT moved into codebase/)"
- "0004: default-layout — meta/research/codebase/.gitkeep created post-move"
- "0004: default-layout — meta/research/design-inventories/.gitkeep created post-move"
- "0004: default-layout — meta/research/design-gaps/.gitkeep created post-move"
- "0004: default-layout — meta/research/issues/.gitkeep created post-move"
- "0004: research-override-only — files move from docs/research/ to docs/research/codebase/"
- "0004: research-override-only — new key written as paths.research_codebase: docs/research/codebase"

**D1 value semantics — design-inv/gaps honor overrides:**

- "0004: default-layout — design-inventories move to meta/research/design-inventories/"
- "0004: default-layout — design-gaps move to meta/research/design-gaps/"
- "0004: all-overridden — design-inventories DO NOT MOVE (paths.design_inventories override honored)"
- "0004: all-overridden — design-gaps DO NOT MOVE (paths.design_gaps override honored)"
- "0004: all-overridden — new key paths.research_design_inventories: assets/inv (verbatim)"
- "0004: all-overridden — new key paths.research_design_gaps: gaps (verbatim)"

**Dotfile / non-md handling:**

- "0004: default-layout — .DS_Store is swept (not preserved into new layout)"
- "0004: default-layout — legacy meta/design-gaps/.gitkeep is swept so empty parent can be removed"
- "0004: default-layout — empty legacy parent meta/design-inventories/ is removed"
- "0004: default-layout — empty legacy parent meta/design-gaps/ is removed"
- "0004: default-layout — legacy parent with unexpected file residue emits diagnostic and is preserved"
- "0004: idempotent — re-running does not touch existing .gitkeep files (no mtime churn)"

**Mixed-state refusal:**

- "0004: mixed-config — refuses to proceed (both paths.research and paths.research_codebase present)"
- "0004: partial-state — refuses to proceed (paths.research_codebase explicitly set in config)"

**Idempotency and conflict:**

- "0004: default-layout — second run is no-op (jj diff --name-only empty)"
- "0004: conflict at destination halts migration with zero filesystem mutation (pre-flight collision check)"

**VCS rename detection (both jj and git):**

- "0004: VCS rename history preserved (jj log --follow)"
- "0004: VCS rename history preserved (git log --follow shows R-status)"

**Local config:**

- "0004: local-config-only — override read from config.local.md, value semantics applied"

#### 3. Migration script — Step 0 (capture) + Step 1 (moves)

**File**: `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh`

Key design decisions in this phase:

1. **`probe_legacy_path` uses 0003's column-0-anchored awk reader**
   (`probe_paths_key`, lines 66-102) — it reads `.accelerator/config.md`
   (and `.accelerator/config.local.md`) directly, bypassing
   `config-read-path.sh` because Phase 1 has retired the legacy keys
   from `PATH_KEYS`. The helper distinguishes "key absent" (returns
   empty + sets `<KEY>_HAD_OVERRIDE=0`) from "key present" (returns
   value + sets `_HAD_OVERRIDE=1`). This distinction is required for
   D1 semantics.

2. **Up-front collision check** before any `mv` is executed. Mirrors
   0002's `check_collisions` pattern. Enumerates all planned moves,
   then verifies each destination is absent. On collision, prints
   per-conflict diagnostics and exits non-zero with zero filesystem
   mutation.

3. **Mixed-state refusal** before any work. If the user's config has
   both a legacy key (e.g. `paths.research`) and its renamed form
   (`paths.research_codebase`), the migration refuses to proceed with
   a clear diagnostic naming the conflicting keys.

4. **Dotfile-safe globbing.** Move loops use `[!.]*` (non-dotfile
   glob) for the artifact moves and explicitly enumerate `.gitkeep` /
   `.DS_Store` handling: `.gitkeep` is NEVER moved — it is treated as a
   per-directory marker, not content. After moves complete, a separate
   step ensures `.gitkeep` is present in every destination directory
   (`codebase/`, `design-inventories/`, `design-gaps/`, `issues/`),
   creating it via `touch` if absent. The legacy `${OLD_RESEARCH}/.gitkeep`
   remains in place because `${OLD_RESEARCH}` IS the new parent of those
   subdirs (e.g. `meta/research/.gitkeep` stays). Legacy directories
   that are fully removed after the move (`${OLD_INV}`, `${OLD_GAPS}`)
   have any pre-existing `.gitkeep` swept along with `.DS_Store` so
   `rmdir` can succeed. `.DS_Store` is removed (it's macOS junk, not
   content). Any other dotfile or non-`.md` sibling produces an
   informational `Note: legacy <path> contains <file> — preserved
   as-is, manual cleanup recommended` line, and the legacy parent dir
   is left in place rather than `rmdir`'d.

5. **Per-key value semantics (D1):**
   - `research_codebase`: always nests. New value = `${OLD_RESEARCH}/codebase`.
   - `research_design_inventories`: if `INV_HAD_OVERRIDE=1`, no move,
     new value = `$OLD_INV` (verbatim). Else move, new value =
     `${OLD_RESEARCH}/design-inventories` (or new default).
   - `research_design_gaps`: same shape as design-inventories.
   - `research_issues`: never moves (it's a new bucket). New value =
     `${OLD_RESEARCH}/issues` for the explicit-override case; new
     default otherwise.

**Logging convention.** This script uses only the existing helpers
defined in `scripts/log-common.sh`: `log_die` (write message and
exit 1) and `log_warn` (write `Warning: msg` and return). All other
output goes to stdout via plain `echo` or `printf` — consistent with
the conventions used by 0001, 0002, 0003.

**Environment variables.** Two distinct escape hatches with
separate names so they cannot be confused:
- `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` — bypass the no-VCS-detected
  refusal in the Step 0 pre-flight (user accepts no-rollback risk).
- `ACCELERATOR_MIGRATE_FORCE=1` — the framework-level dirty-tree
  bypass, unchanged from existing behaviour.

```bash
#!/usr/bin/env bash
# DESCRIPTION: Restructure meta/research/ into subject subcategories
set -euo pipefail

source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/log-common.sh"

# ── jj op-id breadcrumb for rollback ──
# Print to stderr so the user has a recovery anchor even if the
# script aborts mid-flight. jj-only; on git the equivalent is the
# pre-migration HEAD captured by run-migrations.sh.
if command -v jj >/dev/null 2>&1 && [ -d "$PROJECT_ROOT/.jj" ]; then
  _0004_op_id=$(jj op log -l 1 --no-graph -T 'self.id().short()' \
                  2>/dev/null | head -1 || true)
  if [ -n "$_0004_op_id" ]; then
    echo "0004: pre-migration jj op-id: $_0004_op_id" >&2
    echo "0004: roll back with: jj op restore $_0004_op_id" >&2
  fi
fi

# ── Step 0a: capture legacy path values via direct config read ──
# Bypasses config-read-path.sh because Phase 1 retired the legacy keys
# from PATH_KEYS. Pattern lifted from 0003:66-102 (probe_paths_key).

# Single-file probe parameterised over prefix (paths|templates) and key.
# Returns "<present>\t<value>" on stdout where <present> is 0 or 1.
# Caller splits on tab. This avoids the subshell-trap of out-parameter
# assignment under $(…) capture.
#
# Value extraction strips inline `# comments` and trailing whitespace.
# Distinguishes "absent" (present=0) from "present with empty value"
# (present=1, value="").
probe_key_in_file() {
  local cfg="$1" prefix="$2" key="$3"
  [ -f "$cfg" ] || { printf '0\t'; return 0; }
  # Uses ~ + sub() (not 3-arg match) for BSD awk portability (macOS).
  local result
  result=$(awk -v prefix="$prefix" -v key="$key" '
    BEGIN {
      block_re="^" prefix ":"
      nested_re="^[[:space:]]+" key ":"
      flat_re="^" prefix "\\." key ":"
      in_block=0
    }
    $0 ~ block_re { in_block=1; next }
    in_block && /^[^[:space:]]/ { in_block=0 }
    in_block && $0 ~ nested_re {
      sub(nested_re, ""); sub(/^[[:space:]]+/, "")
      sub(/[[:space:]]*#.*$/, ""); sub(/[[:space:]]+$/, "")
      print "P\t" $0; exit
    }
    $0 ~ flat_re {
      sub(flat_re, ""); sub(/^[[:space:]]+/, "")
      sub(/[[:space:]]*#.*$/, ""); sub(/[[:space:]]+$/, "")
      print "P\t" $0; exit
    }
  ' "$cfg")
  if [ -n "$result" ]; then
    printf '1\t%s' "${result#P$'\t'}"
  else
    printf '0\t'
  fi
}

# Cross-file probe: returns first match across config.local.md (preferred)
# then config.md. Used by Step 0a to capture legacy values once.
probe_key() {
  local prefix="$1" key="$2" cfg p
  for cfg in "$PROJECT_ROOT/.accelerator/config.local.md" \
             "$PROJECT_ROOT/.accelerator/config.md"; do
    p=$(probe_key_in_file "$cfg" "$prefix" "$key")
    if [ "${p%%$'\t'*}" = "1" ]; then
      printf '%s' "$p"
      return 0
    fi
  done
  printf '0\t'
}

# Capture probe results once via stdout so the boolean reaches the
# caller. (Previous `printf -v` out-parameter pattern was lost in the
# $() subshell.)
_probe_research=$(probe_key "paths" "research")
_probe_inv=$(probe_key "paths" "design_inventories")
_probe_gaps=$(probe_key "paths" "design_gaps")

RESEARCH_HAD_OVERRIDE="${_probe_research%%$'\t'*}"
INV_HAD_OVERRIDE="${_probe_inv%%$'\t'*}"
GAPS_HAD_OVERRIDE="${_probe_gaps%%$'\t'*}"

OLD_RESEARCH="${_probe_research#*$'\t'}"
OLD_INV="${_probe_inv#*$'\t'}"
OLD_GAPS="${_probe_gaps#*$'\t'}"

# Legacy defaults when no override present
[ "$RESEARCH_HAD_OVERRIDE" = "1" ] || OLD_RESEARCH="meta/research"
[ "$INV_HAD_OVERRIDE" = "1" ] || OLD_INV="meta/design-inventories"
[ "$GAPS_HAD_OVERRIDE" = "1" ] || OLD_GAPS="meta/design-gaps"

# D1: research_codebase always nests; design-inv/gaps honor overrides.
NEW_RESEARCH_CODEBASE="${OLD_RESEARCH}/codebase"
NEW_RESEARCH_ISSUES="${OLD_RESEARCH}/issues"
if [ "$INV_HAD_OVERRIDE" = "1" ]; then
  NEW_INV="$OLD_INV"          # honor override — no move
else
  NEW_INV="${OLD_RESEARCH}/design-inventories"   # new default
fi
if [ "$GAPS_HAD_OVERRIDE" = "1" ]; then
  NEW_GAPS="$OLD_GAPS"
else
  NEW_GAPS="${OLD_RESEARCH}/design-gaps"
fi

# ── Step 0b: mixed-state detection ──
# Fail closed ONLY when both a legacy key AND its renamed counterpart
# are present in the same config (true mixed state — user has been
# editing). If only the renamed key is present (post-successful prior
# run state), exit 0 silently — the migration is already applied and
# the framework's state-file gate should have prevented re-execution
# anyway; this is a belt-and-braces idempotency safety net.

# Triples are "prefix:old_key:new_key" — covers both paths and templates.
_assert_no_mixed_state() {
  local triple prefix old new old_present new_present p
  for triple in \
      "paths:research:research_codebase" \
      "paths:design_inventories:research_design_inventories" \
      "paths:design_gaps:research_design_gaps" \
      "templates:research:codebase-research"; do
    prefix="${triple%%:*}"
    local rest="${triple#*:}"
    old="${rest%:*}"; new="${rest#*:}"
    # Per-file probe across both config files; we only care whether
    # the key is present anywhere in the user's config.
    local cfg
    old_present=0; new_present=0
    for cfg in "$PROJECT_ROOT/.accelerator/config.md" \
               "$PROJECT_ROOT/.accelerator/config.local.md"; do
      [ -f "$cfg" ] || continue
      p=$(probe_key_in_file "$cfg" "$prefix" "$old")
      [ "${p%%$'\t'*}" = "1" ] && old_present=1
      p=$(probe_key_in_file "$cfg" "$prefix" "$new")
      [ "${p%%$'\t'*}" = "1" ] && new_present=1
    done
    if [ "$old_present" = "1" ] && [ "$new_present" = "1" ]; then
      log_die "0004: mixed-state config detected — both ${prefix}.${old} and ${prefix}.${new} are set. Resolve manually (remove ${prefix}.${old}) and retry."
    fi
  done
}
_assert_no_mixed_state

# If all three legacy keys are absent AND all four new keys are absent,
# the user is either fresh-defaults (no paths block) or already-migrated.
# In either case the move loops will no-op via _move_if_pending; nothing
# special to do — proceed normally.

# ── Step 0c: scan-corpus dirty-tree pre-flight ──
# Runs BEFORE any mutation (moves, config rewrites, inbound rewrites).
# Broader than run-migrations.sh's default meta/+.accelerator/ perimeter.

# Single canonical helper; reused by Step 0c pre-flight (here) and
# Step 3 inbound rewrite walk (Phase 5).
build_scan_corpus() {
  # Echoes one absolute directory per line. Splits each
  # `- key: value` bullet at the FIRST `: ` and keeps the whole
  # remainder as the value (so values containing `: ` survive).
  bash "$CLAUDE_PLUGIN_ROOT/scripts/config-read-all-paths.sh" \
    | awk '
        /^- [^[:space:]]+: / {
          # Strip leading "- key: ", preserving the rest verbatim.
          sub(/^- [^[:space:]]+: /, "")
          print
        }
      ' \
    | while IFS= read -r v; do
        [ -d "$PROJECT_ROOT/$v" ] && printf '%s\n' "$PROJECT_ROOT/$v"
      done
}

_preflight_scan_corpus_clean() {
  local dirs=()
  while IFS= read -r d; do dirs+=("$d"); done < <(build_scan_corpus)
  [ "${#dirs[@]}" -eq 0 ] && return 0
  local dirty=""
  if command -v jj >/dev/null 2>&1 && [ -d "$PROJECT_ROOT/.jj" ]; then
    dirty=$(jj diff --name-only -r @ -- "${dirs[@]}" 2>/dev/null || true)
  elif [ -d "$PROJECT_ROOT/.git" ]; then
    dirty=$(git -C "$PROJECT_ROOT" status --porcelain -- "${dirs[@]}" 2>/dev/null \
              | grep -v '^??' || true)
  else
    if [ "${ACCELERATOR_MIGRATE_FORCE_NO_VCS:-}" = "1" ]; then
      log_warn "0004: no VCS detected — proceeding because ACCELERATOR_MIGRATE_FORCE_NO_VCS=1. Recovery from a botched migration is not possible without VCS."
      return 0
    fi
    log_die "0004: no VCS detected (.jj or .git absent). Recovery via VCS rollback is unavailable. Set ACCELERATOR_MIGRATE_FORCE_NO_VCS=1 to proceed anyway."
  fi
  if [ -n "$dirty" ]; then
    echo "0004: scan corpus has uncommitted changes — commit or stash first:" >&2
    printf '%s\n' "$dirty" | sed 's/^/  /' >&2
    exit 1
  fi
}
_preflight_scan_corpus_clean

# ── Step 0d: Perl version pre-flight ──
# Step 3's negative-lookbehind needs Perl 5.30+. Fail closed early
# rather than aborting mid-Step-3 after Steps 1-2 have mutated.
perl -e 'require 5.030;' 2>/dev/null \
  || log_die "0004: Perl >= 5.30 required for Step 3's regex engine. Detected: $(perl -e 'print $]')."

# ── Step 1: plan moves, check collisions, then execute ──
# Up-front collision check ensures zero filesystem mutation on conflict.

PLANNED_MOVES=()  # entries are "src<TAB>dst"
_plan_move() {
  PLANNED_MOVES+=("$1"$'\t'"$2")
}

# Helpers use subshells to scope `shopt` toggles — callers see no
# residual nullglob/dotglob state.
_plan_research_moves() {
  [ -d "$PROJECT_ROOT/$OLD_RESEARCH" ] || return 0
  local f base
  while IFS= read -r f; do
    base=$(basename "$f")
    [ "$base" = ".DS_Store" ] && continue
    # .gitkeep stays in place — OLD_RESEARCH IS the new parent dir,
    # so its existing .gitkeep is the parent's marker. A fresh
    # .gitkeep for the new subdir is created post-move.
    [ "$base" = ".gitkeep" ] && continue
    _plan_move "$OLD_RESEARCH/$base" "$NEW_RESEARCH_CODEBASE/$base"
  done < <(
    cd "$PROJECT_ROOT/$OLD_RESEARCH" && \
      ( shopt -s nullglob dotglob; for f in *; do [ -f "$f" ] && printf '%s\n' "$f"; done )
  )
}

_plan_inv_moves() {
  [ "$INV_HAD_OVERRIDE" = "1" ] && return 0
  [ -d "$PROJECT_ROOT/$OLD_INV" ] || return 0
  local d base
  while IFS= read -r d; do
    base=$(basename "$d")
    _plan_move "$OLD_INV/$base" "$NEW_INV/$base"
  done < <(
    cd "$PROJECT_ROOT/$OLD_INV" && \
      ( shopt -s nullglob; for d in */; do printf '%s\n' "${d%/}"; done )
  )
}

_plan_gaps_moves() {
  [ "$GAPS_HAD_OVERRIDE" = "1" ] && return 0
  [ -d "$PROJECT_ROOT/$OLD_GAPS" ] || return 0
  local f base
  while IFS= read -r f; do
    base=$(basename "$f")
    [ "$base" = ".DS_Store" ] && continue
    # .gitkeep in the legacy dir is a stale marker — the dir itself
    # is removed post-move. A fresh .gitkeep for NEW_GAPS is created
    # post-move.
    [ "$base" = ".gitkeep" ] && continue
    _plan_move "$OLD_GAPS/$base" "$NEW_GAPS/$base"
  done < <(
    cd "$PROJECT_ROOT/$OLD_GAPS" && \
      ( shopt -s nullglob dotglob; for f in *; do [ -f "$f" ] && printf '%s\n' "$f"; done )
  )
}

_plan_research_moves
_plan_inv_moves
_plan_gaps_moves

_check_collisions() {
  local conflicts=()
  for entry in "${PLANNED_MOVES[@]+"${PLANNED_MOVES[@]}"}"; do
    local dst="${entry#*$'\t'}"
    if [ -e "$PROJECT_ROOT/$dst" ]; then
      conflicts+=("$dst")
    fi
  done
  if [ "${#conflicts[@]}" -gt 0 ]; then
    echo "0004: destination collision(s) detected. Migration aborted with no filesystem changes." >&2
    local c
    for c in "${conflicts[@]}"; do
      echo "  conflict: $c already exists" >&2
    done
    exit 1
  fi
}
_check_collisions

# ── Step 1 execute: lifted _move_if_pending from 0003:32-64 ──
# Plain `mv` (not jj mv) so both jj and git pick up the rename via
# filesystem snapshot.
_move_if_pending() {
  local src_rel="$1" dst_rel="$2"
  local src="$PROJECT_ROOT/$src_rel" dst="$PROJECT_ROOT/$dst_rel"
  [ -e "$src" ] || return 0           # already moved (idempotent)
  mkdir -p "$(dirname "$dst")"
  mv "$src" "$dst"
  echo "0004: moved $src_rel → $dst_rel"
}

for entry in "${PLANNED_MOVES[@]+"${PLANNED_MOVES[@]}"}"; do
  src="${entry%$'\t'*}"; dst="${entry#*$'\t'}"
  _move_if_pending "$src" "$dst"
done

# ── Cleanup legacy parents ──
# rmdir succeeds only if empty. Emit a diagnostic when residue blocks
# the cleanup, rather than silently swallowing the error. .gitkeep is
# swept here (not preserved) because the legacy dir itself is being
# removed — the new destination gets its own fresh .gitkeep below.
_cleanup_legacy_parent() {
  local d="$1"
  local full="$PROJECT_ROOT/$d"
  [ -d "$full" ] || return 0
  # Sweep marker/junk files first; harmless to delete.
  [ -f "$full/.DS_Store" ] && rm -f "$full/.DS_Store"
  [ -f "$full/.gitkeep" ] && rm -f "$full/.gitkeep"
  if rmdir "$full" 2>/dev/null; then
    echo "0004: removed empty legacy directory $d"
  else
    log_warn "0004: legacy directory $d not empty — preserved as-is."
    local residual
    while IFS= read -r residual; do
      log_warn "  contains: $(basename "$residual") (manual cleanup may be needed)"
    done < <(
      cd "$full" && \
        ( shopt -s nullglob dotglob; for r in *; do printf '%s\n' "$r"; done )
    )
  fi
}

# Only clean legacy parents that are being fully replaced. OLD_RESEARCH
# is NOT cleaned because it IS the new parent of codebase/, design-
# inventories/, design-gaps/, and issues/. OLD_INV/OLD_GAPS stay in
# place when the user overrode them (no move was planned).
[ "$INV_HAD_OVERRIDE" = "1" ] || _cleanup_legacy_parent "$OLD_INV"
[ "$GAPS_HAD_OVERRIDE" = "1" ] || _cleanup_legacy_parent "$OLD_GAPS"

# ── Ensure .gitkeep in every destination directory ──
# Invariant: all meta dirs retain a .gitkeep regardless of whether
# they are empty or populated, so the directory survives a checkout
# that strips its contents. Idempotent: existing .gitkeep is left
# untouched (no mtime churn).
_ensure_gitkeep() {
  local d="$1"
  local full="$PROJECT_ROOT/$d"
  [ -d "$full" ] || mkdir -p "$full"
  [ -e "$full/.gitkeep" ] || { : > "$full/.gitkeep"; echo "0004: created $d/.gitkeep"; }
}

_ensure_gitkeep "$NEW_RESEARCH_CODEBASE"
_ensure_gitkeep "$NEW_RESEARCH_ISSUES"
# Only ensure .gitkeep in the new default locations the migration
# created. User-overridden destinations (NEW_INV/NEW_GAPS == OLD_*)
# are outside this invariant — the user manages their custom layout.
[ "$INV_HAD_OVERRIDE" = "1" ] || _ensure_gitkeep "$NEW_INV"
[ "$GAPS_HAD_OVERRIDE" = "1" ] || _ensure_gitkeep "$NEW_GAPS"
```

Note: this phase introduces Steps 0 and 1 only; Step 2 (config
rewrite) and Step 3 (inbound rewriting) follow in phases 4 and 5.

### Success criteria

#### Automated verification

- [ ] `bash skills/config/migrate/scripts/test-migrate.sh` — all
  0004 filesystem-move test cases green (≈18 cases from §2 above).
- [ ] `bash scripts/test-config.sh` — still green (no regressions).
- [ ] All six fixture trees exist under
  `skills/config/migrate/scripts/test-fixtures/0004/`.
- [ ] Mixed-state test asserts exit non-zero AND zero filesystem
  mutation occurred (move counts == 0 after the failed run).
- [ ] Collision test asserts the same atomicity property.
- [ ] `jj diff --name-only` empty after second run on default-layout
  fixture (true idempotency).

#### Manual verification

- [ ] Manually run the migration against a hand-built `all-overridden`
  temp dir and confirm design-inventories/gaps are NOT moved.
- [ ] `jj log --follow <new-path>` shows pre-move history.
- [ ] `git log --follow --name-status <new-path>` shows R-status
  (rename) entries in a parallel git-mode fixture.

---

## Phase 4 — Migration: config-key rewrites & per-key notifications

### Overview

Extend the 0004 migration script with Step 2 — the config-key rewrite
phase. Implements D1 value semantics (research_codebase appends
`/codebase`; design-inv/gaps verbatim), D4 template-key rename
(`templates.research` → `templates.codebase-research`), specifies the
`paths.research_issues` insertion algorithm, and applies to both
`.accelerator/config.md` and `.accelerator/config.local.md`.

### Changes required

#### 1. New test cases in `test-migrate.sh`

**D1 value rewrite — research_codebase always appends suffix:**

- "Phase 4: default-layout — paths.research_codebase value resolves to meta/research/codebase via new default (no explicit key written)"
- "Phase 4: research-override-only — paths.research_codebase written as `docs/research/codebase` (suffix appended to override value)"
- "Phase 4: research-override-only — paths.research key removed from config"

**D1 value rewrite — design-inv/gaps verbatim:**

- "Phase 4: all-overridden — paths.research_design_inventories: assets/inv (verbatim)"
- "Phase 4: all-overridden — paths.research_design_gaps: gaps (verbatim)"
- "Phase 4: default-layout — paths.design_inventories key removed (no explicit research_design_inventories written; new default applies)"

**D4 templates rename:**

- "Phase 4: templates.research override → templates.codebase-research with value preserved verbatim"
- "Phase 4: templates.research absent → no rewrite emitted, no key injected"
- "Phase 4: user-overridden template file renamed if present at user's templates path; absent file is no-op"

**Notification lines (informational, not a parseable contract):**

- "Phase 4: emits a `Renamed paths.research → paths.research_codebase` line including old/new value when value changes"
- "Phase 4: emits a `Renamed paths.design_inventories → paths.research_design_inventories` line including value when value verbatim"
- "Phase 4: emits no notification when no legacy key is present"
- "Phase 4: notification lines use `grep -qE` for substring match (assertion is informational; format may evolve)"

**research_issues insertion (only when paths.research was rewritten):**

- "Phase 4: research-override-only → paths.research_issues injected as `docs/research/issues` (sibling to research_codebase)"
- "Phase 4: all-overridden → paths.research_issues injected with value `${OLD_RESEARCH}/issues`"
- "Phase 4: default-layout (no research override) → no paths.research_issues injection; new default applies via PATH_DEFAULTS"
- "Phase 4: nested-YAML paths block insertion uses sibling indent"
- "Phase 4: flat-dotted paths.* form appends after last paths.* line"
- "Phase 4: paths block absent → no injection (never injects a paths block where none existed)"
- "Phase 4: paths.research_issues already present → no duplicate injection (file byte-identical)"

**config.local.md:**

- "Phase 4: local-config-only fixture — rewrite applies to config.local.md only; config.md unchanged"
- "Phase 4: both files override same key — both rewritten; notification emitted once per file (key precedence is file-local)"

**Absent-key silence:**

- "Phase 4: empty config — no Renamed lines on stdout"

#### 2. Migration script — Step 2

Four design decisions:

1. **Explicit form-detection.** The rewriter detects whether the user's
   `paths:` block is nested-YAML or flat-dotted form per file, and
   applies the appropriate rewrite.

2. **Value transform happens in bash, not awk.** The transform
   functions run on the value captured by `probe_key_in_file` *before*
   the awk rewriter sees it. The awk rewriter only does line
   substitution with the new value passed in via `-v` — no awk→shell
   round-trip, no `shellquote()`, no 3-arg `match()`.

3. **Per-file probing.** `probe_key_in_file` (defined in Step 0a)
   reads exactly one file. The per-file loop in `_rewrite_pair`
   calls it once per file and skips files where the key is absent
   — eliminating cross-file probing ambiguity.

4. **Notification format is informational.** The lines are
   human-readable diagnostics, not a stability contract for
   downstream automation. Tests assert substring presence via
   `grep -qE` rather than exact-line equality.

5. **Pre-rewrite backup.** Before the first rewrite of any config
   file, `cp $cfg $cfg.0004.bak` captures a verbatim copy under
   `.accelerator/` (a tracked location). Recovery from a botched
   awk rewrite is one `cp` away. Backup files survive the migration
   and can be removed by the user once the result is verified.

```bash
# ── Helpers ──

# probe_key_in_file is defined in Step 0a (Phase 3). It is reused
# here without redefinition.

# Backup helper. Idempotent — if the .bak already exists (e.g. from a
# prior failed run), it is not overwritten.
_backup_config_once() {
  local cfg="$1"
  [ -f "$cfg" ] || return 0
  local bak="$cfg.0004.bak"
  [ -e "$bak" ] && return 0
  cp "$cfg" "$bak"
  echo "0004: backed up $(basename "$cfg") → $(basename "$bak") (remove after verifying migration)"
}

# Detect whether a config file uses nested-YAML or flat-dotted form
# for a given prefix. Returns "nested", "flat", or "absent".
detect_form() {
  local cfg="$1" prefix="$2"
  [ -f "$cfg" ] || { printf 'absent'; return 0; }
  if grep -qE "^${prefix}:" "$cfg"; then printf 'nested'; return; fi
  if grep -qE "^${prefix}\\." "$cfg"; then printf 'flat'; return; fi
  printf 'absent'
}

# Rewrite a single key inside a config file. The new value is computed
# in bash and passed in via -v new_val=…; awk only does the line
# substitution. Uses ~ + sub() (BSD-awk portable; no 3-arg match()).
rewrite_one_key() {
  local cfg="$1" prefix="$2" old_key="$3" new_key="$4" new_val="$5"
  local form; form=$(detect_form "$cfg" "$prefix")
  case "$form" in
    nested)
      awk -v prefix="$prefix" -v old="$old_key" -v new="$new_key" -v val="$new_val" '
        BEGIN {
          nested_block_re="^" prefix ":"
          old_re="^([[:space:]]+)" old ":"
          in_block=0
        }
        $0 ~ nested_block_re { in_block=1; print; next }
        in_block && /^[^[:space:]]/ { in_block=0 }
        in_block && $0 ~ old_re {
          # Capture leading indent by stripping the rest.
          line=$0
          sub(old ":.*", "", line)   # line now contains just the indent
          print line new ": " val
          next
        }
        { print }
      ' "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"
      ;;
    flat)
      awk -v prefix="$prefix" -v old="$old_key" -v new="$new_key" -v val="$new_val" '
        BEGIN { old_re="^" prefix "\\." old ":" }
        $0 ~ old_re { print prefix "." new ": " val; next }
        { print }
      ' "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"
      ;;
    absent)
      return 0
      ;;
  esac
}

# Value-transform functions
_xform_identity() { cat; }
_xform_append_codebase() { printf '%s/codebase' "$(cat)"; }

# Informational diagnostic emitter (NOT a parseable contract)
emit_rename_notice() {
  local prefix="$1" old="$2" new="$3" oldval="$4" newval="$5"
  if [ "$oldval" = "$newval" ]; then
    echo "0004: renamed ${prefix}.${old} → ${prefix}.${new} (value: ${oldval})"
  else
    echo "0004: renamed ${prefix}.${old} → ${prefix}.${new} (value: ${oldval} → ${newval})"
  fi
}

# ── Step 2a: per-key rewrites with notifications ──
# Per-file probing eliminates cross-file probing ambiguity.

_rewrite_pair() {
  local prefix="$1" old="$2" new="$3" xform="$4"
  local cfg
  for cfg in "$PROJECT_ROOT/.accelerator/config.md" \
             "$PROJECT_ROOT/.accelerator/config.local.md"; do
    [ -f "$cfg" ] || continue
    local probe; probe=$(probe_key_in_file "$cfg" "$prefix" "$old")
    local present="${probe%%$'\t'*}"
    local oldval="${probe#*$'\t'}"
    [ "$present" = "1" ] || continue
    _backup_config_once "$cfg"
    local newval; newval=$(printf '%s' "$oldval" | "$xform")
    rewrite_one_key "$cfg" "$prefix" "$old" "$new" "$newval"
    emit_rename_notice "$prefix" "$old" "$new" "$oldval" "$newval"
  done
}

# Paths renames (D1 value semantics)
_rewrite_pair "paths" "research" "research_codebase" _xform_append_codebase
_rewrite_pair "paths" "design_inventories" "research_design_inventories" _xform_identity
_rewrite_pair "paths" "design_gaps" "research_design_gaps" _xform_identity

# Templates rename (D4) — same _rewrite_pair, different prefix.
_rewrite_pair "templates" "research" "codebase-research" _xform_identity

# D4 also handles renaming the user's overridden template file if
# they have one at <paths.templates>/research.md. The plugin's own
# templates/ tree is handled by Phase 1's `jj mv`.
_rename_user_template_file_if_present() {
  local templates_dir
  templates_dir=$(bash "$CLAUDE_PLUGIN_ROOT/scripts/config-read-path.sh" templates)
  local src="$PROJECT_ROOT/$templates_dir/research.md"
  local dst="$PROJECT_ROOT/$templates_dir/codebase-research.md"
  [ -f "$src" ] || return 0
  [ -e "$dst" ] && log_die "0004: destination $dst already exists — cannot rename user template."
  mv "$src" "$dst"
  echo "0004: renamed $templates_dir/research.md → $templates_dir/codebase-research.md"
}
_rename_user_template_file_if_present

# ── Step 2b: paths.research_issues injection ──
# CRITICAL: only inject when paths.research was actually rewritten in
# this run. Without an explicit research override, the user is on the
# new default and doesn't need an explicit key.
_inject_research_issues() {
  local cfg="$1"
  [ -f "$cfg" ] || return 0
  local form; form=$(detect_form "$cfg" "paths")
  case "$form" in
    absent) return 0 ;;   # never inject a paths block
    nested)
      # Skip if already present.
      grep -qE '^[[:space:]]+research_issues:' "$cfg" && return 0
      # Find any sibling indent inside the paths: block. Insert
      # `research_issues: <value>` after the last sibling.
      awk -v val="${OLD_RESEARCH}/issues" '
        BEGIN { in_block=0; last_sib=0; indent="" }
        /^paths:/ { in_block=1; lines[NR]=$0; n=NR; next }
        in_block && /^[^[:space:]]/ { in_block=0 }
        in_block && match($0, /^[[:space:]]+[^[:space:]]/) {
          # Capture indent on each sibling line; updates so we get
          # the indent of the LAST sibling.
          last_sib=NR
          sib_indent=substr($0, 1, RLENGTH-1)
        }
        { lines[NR]=$0; n=NR }
        END {
          for (i=1; i<=n; i++) {
            print lines[i]
            if (i==last_sib) print sib_indent "research_issues: " val
          }
        }
      ' "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"
      ;;
    flat)
      grep -qE '^paths\.research_issues:' "$cfg" && return 0
      awk -v val="${OLD_RESEARCH}/issues" '
        /^paths\./ { last=NR }
        { lines[NR]=$0; n=NR }
        END {
          for (i=1; i<=n; i++) {
            print lines[i]
            if (i==last) print "paths.research_issues: " val
          }
        }
      ' "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"
      ;;
  esac
}

# Only inject when the user explicitly overrode paths.research.
# Default-layout users get the new default via PATH_DEFAULTS — no
# explicit key needed in their config.
if [ "$RESEARCH_HAD_OVERRIDE" = "1" ]; then
  for cfg in "$PROJECT_ROOT/.accelerator/config.md" \
             "$PROJECT_ROOT/.accelerator/config.local.md"; do
    _backup_config_once "$cfg"
    _inject_research_issues "$cfg"
  done
fi
```

Notes on the algorithm:
- All awk uses `~` + `sub()` (BSD-awk portable). No 3-arg `match()`,
  no `shellquote()`, no awk→shell round-trip.
- The value transform happens in bash before the awk rewriter sees
  it; the awk script substitutes only the literal new value.
- `_rewrite_pair` is per-file: it probes each config file
  independently, so files without the key are not visited by
  `rewrite_one_key`.
- `_rewrite_pair` is idempotent: a successful prior run removes the
  legacy key, so the probe returns `0\t` on the next run and the
  loop skips. (Belt-and-braces idempotency on top of the framework's
  state-file gate.)
- `_inject_research_issues` only fires when `RESEARCH_HAD_OVERRIDE=1`
  — users on defaults don't get a pinned explicit value.
- D4's template-file rename uses `mv` directly (plain rename of a
  user-overridden template file), not `_move_if_pending`. Collision
  is fatal — the user must resolve it.

### Success criteria

#### Automated verification

- [ ] All Phase 4 test cases pass, including value-semantic, templates,
  insertion, and config.local.md cases.
- [ ] `bash skills/config/migrate/scripts/test-migrate.sh` overall exit 0.
- [ ] Substring notification match: `grep -qE 'renamed paths.research → paths.research_codebase' <captured-stdout>` succeeds when applicable.
- [ ] research_issues injection NEVER adds a `paths:` block to a config
  that had none (assert post-run config equals pre-run when paths
  absent).
- [ ] research_issues injection ONLY fires when RESEARCH_HAD_OVERRIDE=1
  (assert default-layout fixture's post-migration config does NOT
  contain `paths.research_issues`).

#### Manual verification

- [ ] Inspect the post-migration `config.md` of `research-override-only`
  fixture — `paths.research_codebase: docs/research/codebase` (value
  rewritten with suffix).
- [ ] Inspect `all-overridden` fixture — `paths.research_design_inventories:
  assets/inv` (verbatim).
- [ ] Notification stdout matches the per-key format documented above.

---

## Phase 5 — Migration: config-driven inbound-link rewriting

### Overview

Add Step 3 — inbound-link rewriting. This is the most complex phase:
lift 0002's three rewriters (frontmatter, markdown-link, prose),
parameterise over `[(old, new)]` pairs rather than a single id-rename,
and replace the hardcoded `meta/**/*.md` scan with a union of paths
surfaced by `accelerator:paths`.

### Changes required

#### 1. Fixture corpus

Extend `test-fixtures/0004/inbound-corpus/` with broader prose shapes
beyond fenced code blocks (real corpus contains backtick-inline,
bullet-list-bare, and narrative-prose references), plus explicit
non-matching prefix shapes that exercise the path-boundary anchor:

```
test-fixtures/0004/inbound-corpus/
├── matching/
│   ├── frontmatter-scalar.md
│   ├── frontmatter-list-inline.md
│   ├── frontmatter-list-multiline.md
│   ├── markdown-link.md
│   ├── prose-fenced-code.md       # inside ```…``` blocks
│   ├── prose-backtick-inline.md   # `meta/research/foo.md` inline
│   ├── prose-bullet-bare.md       # - meta/research/foo.md — …
│   ├── prose-narrative.md         # … as described in meta/research/…
│   └── moved-file-internal-link.md # file that itself moves AND links to a sibling
└── non-matching/
    ├── frontmatter-scalar.md       # unrelated key with similar value
    ├── markdown-link.md            # link to meta/research-templates/
    ├── prose-mention.md            # meta/researchers.md, meta/research-archive/
    └── boundary-edge-cases.md      # meta/research_archive/, meta/research?ref=…
```

Each `matching/` fixture contains a reference to each of the three
legacy paths (`meta/research/foo.md`, `meta/design-inventories/bar/`,
`meta/design-gaps/baz.md`). Each `non-matching/` fixture contains
look-alike but distinct content that must remain byte-identical.

The `moved-file-internal-link.md` fixture is placed under
`meta/research/` (so Step 1 moves it to `meta/research/codebase/`)
and contains an inbound link to a sibling that also moves. Asserts
that internal cross-links between moved files are rewritten correctly
post-move.

#### 2. New test cases in `test-migrate.sh`

**Per-shape rewrite tests (positive):**

- "Phase 5: rewrites markdown links"
- "Phase 5: rewrites frontmatter scalars"
- "Phase 5: rewrites frontmatter inline lists"
- "Phase 5: rewrites frontmatter multi-line lists"
- "Phase 5: rewrites prose mentions in fenced code blocks"
- "Phase 5: rewrites prose backtick-inline references"
- "Phase 5: rewrites prose bullet-list bare paths"
- "Phase 5: rewrites prose narrative-text mentions"

**Boundary-anchor tests (negative — byte-identity):**

- "Phase 5: meta/research-templates/ unchanged"
- "Phase 5: meta/researchers.md unchanged"
- "Phase 5: meta/research-archive/ unchanged"
- "Phase 5: meta/research_archive/ unchanged"
- "Phase 5: unrelated-key frontmatter scalar unchanged"

**Moved-file cross-links:**

- "Phase 5: file that itself moves still has its internal links rewritten correctly"

**Scan corpus & pre-flight (note: pre-flight runs in Step 0c, before
Steps 1-2 mutate — so failures abort with zero filesystem change):**

- "Phase 5: scan corpus derives from accelerator:paths" — fixture
  config sets `paths.work: custom/work`; place a matching reference
  inside `custom/work/some.md`; assert rewritten. Place an identical
  reference inside `unrelated/dir/some.md`; assert untouched.
- "Phase 5: Step 0c pre-flight refuses on dirty scan corpus" —
  fixture has uncommitted change inside a configured non-default
  path; assert migration refuses with zero filesystem mutation (no
  files moved, no config rewritten).
- "Phase 5: Step 0c pre-flight refuses with no VCS" — fixture
  without .jj or .git; assert exit non-zero with diagnostic.
- "Phase 5: Step 0c pre-flight bypass via ACCELERATOR_MIGRATE_FORCE_NO_VCS=1" —
  same no-VCS fixture with env var set; assert migration proceeds
  with a warning.

**Idempotency (stricter):**

- "Phase 5: idempotent — second run leaves jj diff --name-only empty"
- "Phase 5: double-substitution recovery — partially-rewritten
  fixture (some lines pre-rewritten, others not) → migration
  completes the rewrite without producing `meta/research/codebase/codebase/foo.md`
  paths"

#### 3. Migration script — Step 3

Two structural changes relative to 0002's id-rename rewriter:

1. **Path-segment boundary anchor.** Every prefix match requires the
   matched prefix to be followed by one of `/`, `"`, `'`, whitespace,
   `)`, `]`, `#`, or end-of-line. Implemented as a regex character
   class. Applied uniformly across frontmatter scalar, frontmatter
   list, markdown-link, and prose rewriters.

2. **Prose rewriter expanded beyond fenced code blocks.** 0002's
   prose branch handled only fenced code. 0004 extends to: inline
   backticks, bullet-list bare paths, and narrative text. Each is a
   distinct regex pass over each markdown body line, with the
   boundary anchor applied.

Note: the scan-corpus dirty-tree pre-flight runs in Step 0c (before
Steps 1-2 mutate), not here. Similarly, the Perl version pre-flight
runs in Step 0d. Step 3 itself can assume the environment is ready.

```bash
# ── Step 3: inbound-link rewriting ──

# 3a. Build (old, new) pair set from D1 semantics.
# Research files moved; pair is always present.
# Design pairs included only when files actually moved.
PAIRS=()  # entries are "old<TAB>new"
PAIRS+=("$OLD_RESEARCH"$'\t'"$NEW_RESEARCH_CODEBASE")
[ "$INV_HAD_OVERRIDE" = "1" ] || PAIRS+=("$OLD_INV"$'\t'"$NEW_INV")
[ "$GAPS_HAD_OVERRIDE" = "1" ] || PAIRS+=("$OLD_GAPS"$'\t'"$NEW_GAPS")

# 3b. build_scan_corpus is defined in Step 0c (Phase 3) and reused here.

# 3c. Pre-mutation summary banner (no dry-run, but transparent).
_corpus_count=$(build_scan_corpus | wc -l)
echo "0004 Step 3: scanning $((_corpus_count)) configured paths for inbound references…"

# 3d. Per-file rewriter.
# The `\Q…\E` form treats `${old}`/`${new}` as literal strings so
# values containing /, ., etc. don't need shell or regex escaping.
#
# The boundary-anchor character class `[/"'\s)\]\#]|$` requires the
# match to end at a path-segment boundary, preventing false positives
# like `meta/research-templates/` matching `meta/research`.
#
# Negative lookbehind `(?<!\Q${new}\E/)` prevents double-substitution
# when the migration is re-run on already-rewritten content. Phase 0d
# enforces Perl >= 5.30 which supports this construction.
#
# Uses bracketing `s{}{}g` delimiters (not `s|...|...|g`) so that the
# `|$` alternation inside the lookahead character class cannot be
# confused with a delimiter boundary.

rewrite_file_with_pairs() {
  local file="$1"
  local entry old new
  for entry in "${PAIRS[@]+"${PAIRS[@]}"}"; do
    old="${entry%$'\t'*}"; new="${entry#*$'\t'}"
    OLD="$old" NEW="$new" perl -i -pe '
      my $old = $ENV{OLD};
      my $new = $ENV{NEW};
      s{(?<!\Q$new\E/)\Q$old\E(?=[/"'\''\s)\]\#]|$)}{$new}g;
    ' "$file"
  done
}

# 3e. Walk the scan corpus.
while IFS= read -r dir; do
  while IFS= read -r -d '' file; do
    rewrite_file_with_pairs "$file"
  done < <(find "$dir" -type f -name '*.md' -print0)
done < <(build_scan_corpus)
```

Notes:
- Variable interpolation into perl regex is via the `OLD`/`NEW`
  environment variables, not bash string substitution into a
  double-quoted perl script. This avoids shell-quote escapes inside
  the perl source and keeps `\Q…\E` literal-mode-safe across
  arbitrary path characters.
- Negative-lookbehind requires Perl 5.30+ (enforced in Step 0d).
- The boundary anchor is shared across all reference shapes —
  frontmatter, markdown-link, prose all match the same regex.
- Idempotency: on a second pass, every literal `<old>` occurrence
  in the file is preceded by `<new>/` (because Step 1's prior
  rewrite produced `<new>/…`). The negative-lookbehind suppresses
  the match, so the second pass is byte-identical.

### Success criteria

#### Automated verification

- [ ] All 8 per-shape positive-rewrite tests pass.
- [ ] All 5 boundary-anchor negative tests pass (byte-identity).
- [ ] Moved-file cross-link test passes.
- [ ] Scan-corpus derivation test passes.
- [ ] Step 0c dirty-tree pre-flight refuses on a dirty scan-corpus
  fixture with zero filesystem mutation.
- [ ] Step 0c no-VCS pre-flight refuses by default; bypasses with
  `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1`.
- [ ] Idempotency test: `jj diff --name-only` empty after second run.
- [ ] Double-substitution test: partially-rewritten input → second
  run brings file to consistent fully-rewritten state without
  producing `<new>/<new>/…` paths.
- [ ] `bash skills/config/migrate/scripts/test-migrate.sh` exit 0.

#### Manual verification

- [ ] Inspect each rewriter against a hand-built file with mixed
  matching and non-matching content; confirm only matches rewrite.
- [ ] Re-run the migration on a previously-migrated fixture; confirm
  zero file modifications via `jj diff --name-only`.
- [ ] Spot-check the negative-lookbehind path: a partially-rewritten
  file (some lines rewritten on prior aborted run, some not) is
  brought to fully-rewritten state without double-substitution.

---

## Phase 6 — `documents-locator` surgery & `configure` UX

### Overview

Update the agent definition (legend + output template + historic
intent) and the configure SKILL paths reference (table + example
YAML, including drift fix).

### Changes required

#### 1. `agents/documents-locator.md`

**Legend (lines 33-41)** — replace the single `research` bullet with
four bullets matching the new keys:

```markdown
- `work` — work items
- `research_codebase` — codebase research documents
- `research_issues` — issue/RCA research documents
- `research_design_inventories` — design-inventory artifacts (one directory per snapshot, with screenshots/)
- `research_design_gaps` — design-gap analysis artifacts
- `plans` — implementation plans
…
```

**Output template (lines 71-97)** — replace the single
`### Research Documents` group with four explicit groups:

```markdown
### Research (codebase)
- `{research_codebase}/<file>.md` - …

### Research (issues)
- `{research_issues}/<file>.md` - …

### Research (design-inventories)
- `{research_design_inventories}/<dir>/inventory.md` - …

### Research (design-gaps)
- `{research_design_gaps}/<file>.md` - …
```

Update the trailing prose at lines 99-100 to enumerate the four
substituted variables, and add an explicit instruction:
*"Omit any `### Research (…)` group that contains zero findings;
prefer rendering only the subcategories with actual hits."* This
keeps the output proportional to the artifact density (a query
hitting only codebase research shouldn't render three empty
subcategory headers).

**Legend bullet ordering** — order the four `research_*` bullets to
match `PATH_KEYS` ordering (the same convention used by `configure/SKILL.md`
paths table and example YAML). Codebase research first, issues, then
design inventories, then design gaps.

**Historic intent (line 112)** — replace the bare `research` token
with the four `research_*` keys.

#### 2. `skills/config/configure/SKILL.md`

**Paths table (386-402)** — replace the `research`,
`design_inventories`, `design_gaps` rows with four new rows. Order
mirrors `PATH_KEYS`.

**Example YAML (406-422)** — replace `research:` line with the four
new keys, restore `design_inventories`/`design_gaps` (now under their
renamed forms) and `integrations` (drift fix). New shape:

```yaml
---
paths:
  plans: docs/plans
  research_codebase: docs/research/codebase
  research_issues: docs/research/issues
  research_design_inventories: docs/research/design-inventories
  research_design_gaps: docs/research/design-gaps
  decisions: docs/adrs
  prs: docs/prs
  validations: docs/validations
  review_plans: docs/reviews/plans
  review_prs: docs/reviews/prs
  review_work: docs/reviews/work
  templates: docs/templates
  work: docs/work
  notes: docs/notes
  tmp: docs/tmp
  integrations: docs/integrations
---
```

#### 3. Test assertions in `scripts/test-config.sh`

Add/update assertions for the configure paths table (each row exists
with correct default) and example YAML shape.

### Success criteria

#### Automated verification

- [ ] `bash scripts/test-config.sh` — configure-table and YAML-
  example assertions pass.
- [ ] `grep -n "Research Documents" agents/documents-locator.md` —
  returns no match (flat group label gone).
- [ ] `grep -n "Research (codebase)\|Research (issues)\|Research (design-inventories)\|Research (design-gaps)" agents/documents-locator.md` —
  returns four matches.
- [ ] `grep -n "^- .research" agents/documents-locator.md` —
  returns four legend lines, none for bare `research`.

#### Manual verification

- [ ] Manually invoke `documents-locator` (or read its output
  template + preloaded paths block) and confirm the four research
  groups render with their resolved paths.
- [ ] Inspect `configure/SKILL.md` rendered table and YAML for
  consistency with `PATH_KEYS` order.

---

## Phase 7 — Apply migration to plugin `meta/`

### Overview

Run the 0004 migration script against the plugin's own repo as the
final integration test, with an **isolated rehearsal** against a tmp
clone first to surface defects before they touch the working copy.
Take a named jj operation snapshot at entry so rollback is a single
`jj op restore <op-id>` rather than a hunt through the op log.

### Changes required

1. **Pre-rehearsal cleanup** of plugin-specific residue:
   - Delete `meta/design-inventories/.DS_Store` (stray macOS metadata).
   - (Phase 1 already created the empty `meta/research/issues/` via
     init-scaffold updates; no manual `.gitkeep` required because the
     restructure migration ensures `.gitkeep` is present in every
     destination directory — `codebase/`, `design-inventories/`,
     `design-gaps/`, and `issues/` — after the move.)

2. **Isolated rehearsal** against a tmp clone:
   ```bash
   tmpdir=$(mktemp -d)
   jj git clone --colocate . "$tmpdir/plugin-rehearsal"
   cd "$tmpdir/plugin-rehearsal"
   bash skills/config/migrate/scripts/run-migrations.sh
   ```
   Inspect the rehearsal output and `jj diff` before proceeding.
   Verify:
   - No errors emitted by Step 0 (config probe), Step 1 (moves),
     Step 2 (config rewrite), Step 3 (inbound).
   - Rewritten content in 3 spot-check files looks correct
     (a work item, a plan, an ADR — each containing inbound links
     to research).
   - `jj log --follow` on a rehearsed file shows pre-move history.

3. **Snapshot the real working copy** before production application:
   ```bash
   cd /path/to/accelerator
   jj op log -l 1   # capture the op id for rollback breadcrumb
   ```

4. **Apply to plugin's own meta/**:
   ```bash
   bash skills/config/migrate/scripts/run-migrations.sh
   ```

5. **Inspect output:**
   - Plugin's config (`.accelerator/config.md`) does not explicitly
     override the three legacy keys (`paths.research`,
     `paths.design_inventories`, `paths.design_gaps`) — so **zero**
     `Renamed paths.X → paths.Y` lines are emitted. Per-key
     notifications only fire when the user has an explicit override.
   - Move operations logged via `_move_if_pending`.
   - One pre-mutation summary line from Step 3 banner.
   - Exit 0, state file updated.

6. **Verify post-conditions:** see Success Criteria.

### Success criteria

#### Automated verification

- [ ] `bash skills/config/migrate/scripts/run-migrations.sh` exits 0
  on the plugin repo (first run, after rehearsal).
- [ ] Re-run exits 0 with `No pending migrations.`
- [ ] `find meta/design-inventories meta/design-gaps -maxdepth 0 -type d 2>/dev/null` — empty.
- [ ] `find meta/research/codebase -name '*.md' | wc -l` — 37.
- [ ] `find meta/research/design-inventories -name 'inventory.md' | wc -l` — 2.
- [ ] `find meta/research/design-gaps -name '*.md' | wc -l` — 1.
- [ ] `.gitkeep` invariant — all four destination subdirs plus the
  parent retain a marker file:
  - [ ] `test -f meta/research/.gitkeep`
  - [ ] `test -f meta/research/codebase/.gitkeep`
  - [ ] `test -f meta/research/design-inventories/.gitkeep`
  - [ ] `test -f meta/research/design-gaps/.gitkeep`
  - [ ] `test -f meta/research/issues/.gitkeep`
- [ ] **Rewrite-correctness spot checks** (not just count/absence):
  - [ ] `grep -F 'meta/research/codebase/2026-05-08-0052' meta/work/0052-make-documents-locator-paths-config-driven.md` returns at least one rewritten reference.
  - [ ] `grep -F 'meta/research/codebase/2026-05-11-0056' meta/plans/2026-05-12-0056-restructure-meta-research-into-subject-subcategories.md` returns the rewritten reference (this very plan's References section).
  - [ ] No file under `meta/` contains the bare prefix `meta/research/[0-9]` (every legacy file ref rewritten).
- [ ] Broadened residue check:
  `grep -rnE '\bmeta/research/(?!codebase|issues|design-)|\bmeta/design-inventories\b|\bmeta/design-gaps\b' meta/ --include='*.md'` — empty.
- [ ] `bash skills/config/migrate/scripts/test-migrate.sh` — still passes.
- [ ] Plugin's `meta/.accelerator/config.md` (if it has no `paths:`
  block) is byte-identical pre/post-migration — no `paths:` block
  injected.

#### Manual verification

- [ ] `jj log --follow meta/research/codebase/2026-05-08-0052-documents-locator-config-driven-paths.md` shows pre-move history.
- [ ] Spot-check ~5 work items / notes / plans / ADRs for inbound
  reference rewriting correctness (no broken links, no half-rewrites,
  no double-substitution from the boundary-anchor regex).
- [ ] Rollback dry-run: confirm `jj op restore <captured-op-id>` would
  return the repo to the pre-migration state. (Do not actually
  restore unless rolling back.)

---

## Phase 8 — Narrative surfaces: README, visualiser, CHANGELOG, migrate doc

### Overview

Hand-edit narrative content the migration's inbound-link rewriter
does not cover: README (top-level, not under `meta/`), CHANGELOG (a
new entry, leaving historical entries verbatim per scope decision),
the visualiser frontend (TypeScript, not markdown), and the stale
text in `migrate/SKILL.md`.

### Changes required

#### 1. `README.md`

- Line 42 (ASCII development-loop diagram): `meta/research/` →
  `meta/research/codebase/`.
- Line 47 (`research-codebase` narrative): `in \`meta/research/\` with`
  → `in \`meta/research/codebase/\` with`.
- Line 64 (`research-issue` narrative): `in \`meta/research/\``
  → `in \`meta/research/issues/\``.
- Lines 84-95 (`meta/` table): replace the three rows
  (`research/`, `design-inventories/`, `design-gaps/`) with four
  indented rows under a `research/` parent row. Use the `└─` /
  `├─` tree glyphs in the Directory column so the nesting is
  visually clear (mirrors how `notes/` and `work/` are top-level
  siblings, while `codebase/`, `issues/`, `design-inventories/`,
  `design-gaps/` are children of `research/`). Add a one-sentence
  prelude above the table: *"`research/` is itself subdivided into
  four subcategories — codebase research, issue/RCA research,
  design inventories, and design gaps."*
- Lines 408-415 (ADR-flow diagram): `meta/research/` →
  `meta/research/codebase/`.
- Line 579 (`The resulting gap artifact under
  \`meta/design-gaps/\``): → `\`meta/research/design-gaps/\``.

#### 2. Visualiser bash → Rust JSON contract (D3)

Per D3, JSON wire keys move to `research_codebase`/`research_issues`/
`research_design_gaps`/`research_design_inventories`. Updates required
across both sides of the bash → Rust boundary:

**`skills/visualisation/visualise/scripts/write-visualiser-config.sh`:**
- `abs_path research` call (~line 54) → `abs_path research_codebase`,
  plus add `abs_path research_issues`.
- `abs_path design_gaps` (~line 60) → `abs_path research_design_gaps`.
- `abs_path design_inventories` (~line 61) → `abs_path research_design_inventories`.
- JSON `--arg` list and `jq` object shape (~lines 153-180): rename
  the `research` / `design_gaps` / `design_inventories` keys to their
  renamed forms; add the new `research_issues` entry.

**`skills/visualisation/visualise/server/src/docs.rs`:**
- `config_path_key()` returned string constants (~lines 46, 53, 54):
  `"research"` → `"research_codebase"`, `"design_gaps"` →
  `"research_design_gaps"`, `"design_inventories"` →
  `"research_design_inventories"`. Add a new arm for issue research
  routing to `"research_issues"`.

**Server fixtures:**
- `skills/visualisation/visualise/server/tests/fixtures/config.valid.json`
  (~lines 14, 20-21): rename keys inside `doc_paths` block to the
  renamed forms; add `research_issues`.
- `skills/visualisation/visualise/server/tests/fixtures/config.optional-override-null.json`:
  same shape.
- `skills/visualisation/visualise/server/scripts/test-launch-server.sh`
  (~line 70): rename embedded JSON keys.
- `skills/visualisation/visualise/server/tests/common/mod.rs` (~lines
  58-59): rename fixture-key constants if present.

**Frontend:**
- `frontend/src/api/types.ts:7,17,157,164` left unchanged —
  docType strings (`'design-gaps'`, `'design-inventories'`) are
  API-level logical IDs distinct from the bash→Rust config wire
  keys decided here (see "What We're NOT Doing").
- `frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`
  (~lines 160-161): re-check for embedded path literals after the
  main edits; rewrite any legacy `meta/research`/`meta/design-*`
  literals to the new layout.

**Bundle rebuild:** Run `vite build` (or whatever the project's
frontend build command is) — `frontend/dist/assets/index-*.js`
should regenerate without hand-editing.

#### 3. Plugin-internal template body references

The plugin's `templates/*.md` files contain legacy `meta/research/`
references in their bodies (they live at the plugin root, not under
the user's `paths.templates`, so the migration's Step 3 scan corpus
does not cover them). Hand-edit:

- `templates/adr.md` (~line 52): `meta/research/<file>.md` → `meta/research/codebase/<file>.md`.
- `templates/work-item.md` (~line 71): same shape.
- `templates/plan.md` (~line 107): same shape.
- `templates/research.md` → `templates/codebase-research.md` (renamed
  per Phase 1, item 5); inside, update body reference at ~line 48.

#### 4. `configure/SKILL.md` line 724 narrative

The narrative paragraph (~line 724) hardcodes both `meta/research/`
as an example and `paths.research: docs/research` as the override
demonstration. Update both:
- `meta/research/` → `meta/research/codebase/`
- `paths.research: docs/research` → `paths.research_codebase: docs/research/codebase`

#### 5. `CHANGELOG.md`

Add a new entry under `## [Unreleased]` (or the next-version section).
Per D2 (documentation-only upgrade window), expand to make the
upgrade hazard explicit and the new `paths.research_issues` key's
purpose narrative:

```markdown
- **research-directory restructure**: `meta/research/` now has four
  subcategories — `codebase/` (codebase research outputs from
  `/accelerator:research-codebase`), `issues/` (issue/RCA research
  outputs from `/accelerator:research-issue`), `design-inventories/`,
  and `design-gaps/`. The new `paths.research_issues` key gives
  issue-research artifacts their own bucket, separable from codebase
  research in `documents-locator` output and the visualiser.

  **Breaking change**: the legacy `paths.research`,
  `paths.design_inventories`, `paths.design_gaps` keys are renamed
  to `paths.research_codebase`, `paths.research_design_inventories`,
  `paths.research_design_gaps`. The `templates.research` key is
  renamed to `templates.codebase-research`.

  **Upgrade sequence**: pull the new plugin version, then run
  `/accelerator:migrate` to bring your repo in line. Skills that
  read or write research paths will resolve to new defaults — and
  may write to non-existent directories — between the plugin upgrade
  and the migration run. Run `/accelerator:migrate` before your
  next `/accelerator:research-codebase`,
  `/accelerator:research-issue`, `/accelerator:extract-adrs`,
  `/accelerator:extract-work-items`, `/accelerator:visualise`, or
  `/accelerator:init` invocation.

  The migration is config-driven: user overrides on
  `paths.design_inventories` and `paths.design_gaps` are honored
  (files stay where the user placed them, only the key is renamed),
  and the inbound-link rewriter scans every directory surfaced by
  `accelerator:paths`. Content imported into your repo after the
  migration runs (branch merges, sibling-repo copies, pasted legacy
  paths) is not rewritten. To rescan: remove the `0004-…` line from
  `.accelerator/state/migrations-applied` and re-run
  `/accelerator:migrate`.

  > **Note**: historical CHANGELOG entries below pre-date this
  > change and reference the legacy paths verbatim. The path
  > references in those entries describe what shipped at the time
  > of each release.
```

Historical entries at lines 98, 155, 165, 170 are not modified.

#### 6. `skills/config/migrate/SKILL.md` — stale text fix + upgrade-sequence doc

- Lines 11-17: replace the example SessionStart hint referencing
  `meta/.migrations-applied` with the post-0003 path
  `.accelerator/state/migrations-applied`.
- Lines 46-57: replace the documentation of `meta/.migrations-applied` /
  `meta/.migrations-skipped` with the actual state-file paths under
  `.accelerator/state/`.
- Append a new paragraph documenting the upgrade sequence:
  *"After pulling a new plugin version, run `/accelerator:migrate`
  before invoking any skill that reads or writes paths affected by
  pending migrations. Skills do not gate themselves on pending
  migrations; the SessionStart hook warns when migrations are
  pending."*

#### 7. Final validation

Run all tests; broadened residue grep:
```bash
grep -rnE '\bmeta/research/(?!codebase|issues|design-)|\bmeta/design-inventories\b|\bmeta/design-gaps\b' \
  README.md CHANGELOG.md templates/ \
  skills/visualisation/visualise/scripts/ \
  skills/visualisation/visualise/server/ \
  skills/visualisation/visualise/frontend/src/
```
should return only entries in historical CHANGELOG sections.

### Success criteria

#### Automated verification

- [ ] Broadened README residue check:
  `grep -nE '\bmeta/research/(?!codebase|issues|design-)|\bmeta/design-inventories\b|\bmeta/design-gaps\b' README.md` —
  no matches.
- [ ] `grep -n 'meta/\.migrations-' skills/config/migrate/SKILL.md` —
  no matches.
- [ ] `bash scripts/test-config.sh` — exit 0.
- [ ] `bash skills/config/migrate/scripts/test-migrate.sh` — exit 0.
- [ ] Visualiser server tests pass (`cargo test` or equivalent) after
  D3's JSON wire-key rename across both bash writer, Rust resolver,
  and JSON fixtures.
- [ ] `grep -rnE '\bmeta/research/(?!codebase|issues|design-)|\bmeta/design-inventories\b|\bmeta/design-gaps\b' templates/` —
  no matches (template body references rewritten).
- [ ] `grep -nE 'doc_paths\.(research|design_gaps|design_inventories)[^_]' skills/visualisation/visualise/` —
  no matches with legacy bare keys (post-D3 rename complete).

#### Manual verification

- [ ] Render the README locally (`grip` or GitHub preview) and skim
  the meta/ section, ASCII diagrams, and design-convergence
  paragraphs for visual correctness.
- [ ] If visualiser was edited, launch it and confirm
  design-inventory / design-gap artifacts still resolve and render.
- [ ] CHANGELOG entry reads naturally under `[Unreleased]`.

---

## Testing Strategy

### Unit / harness tests

- `scripts/test-config.sh` covers `config-defaults.sh`, init.sh,
  `config-read-all-paths.sh`, `config-read-path.sh`,
  `config-read-template.sh`, paths/SKILL.md legend assertions, the six
  SKILL.md bang-call sites, and the configure paths table + example
  YAML.
- `skills/config/migrate/scripts/test-migrate.sh` covers the new
  migration's three steps via fixtures under
  `scripts/test-fixtures/0004/`. ~36 new test cases across six
  fixture trees (default-layout, research-override-only,
  all-overridden, partial-state, mixed-config, local-config-only)
  plus the `inbound-corpus` subtree:
  - **Filesystem moves (Phase 3):** ~18 cases covering D1 value
    semantics (research_codebase always nests; design-inv/gaps
    honor overrides), `.gitkeep`/`.DS_Store` handling, mixed-state
    refusal, up-front collision detection, jj+git rename
    detection, idempotency (`jj diff --name-only` empty).
  - **Config rewrites (Phase 4):** ~12 cases covering D1 value
    semantics (suffix vs verbatim), D4 templates rename,
    informational notification format (substring match with `grep -qE`),
    research_issues insertion (nested-YAML / flat-dotted / absent),
    config.local.md handling, no-paths-block-injection guarantee.
  - **Inbound rewriting (Phase 5):** ~14 cases covering eight
    positive shapes (markdown links, frontmatter scalars, inline
    lists, multiline lists, fenced code, backtick-inline, bullet-
    bare, narrative-prose), five boundary-anchor negatives
    (`meta/research-templates/`, `meta/researchers.md`,
    `meta/research-archive/`, etc.), moved-file cross-link
    rewriting, scan-corpus pre-flight on dirty corpus, idempotency.

### Integration test

Phase 7 runs the migration against an isolated rehearsal clone
first, then against the plugin's own repo, with a captured jj op-id
as the single-point rollback. If the rehearsal produces a clean
diff and the spot-check files are correctly rewritten, the migration
is production-ready.

### Manual testing

- Spot-check inbound references in a few work items / plans / notes /
  ADRs for both correctness and "didn't over-rewrite" verification.
- Launch the visualiser post-D3 and confirm research/design-inventory/
  design-gap artifacts still resolve and render.
- Re-read README narrative for prose flow.

## Performance Considerations

The migration's `find` walks over the scan corpus (every path in
`accelerator:paths`) could be slow on very large repos (>10k markdown
files). For the typical accelerator userspace (~hundreds of files),
this is negligible. No optimisation required.

## Migration Notes

The migration is the centrepiece of this work item. Users running
`accelerator:migrate` on an upgrade will:

1. See the framework's dirty-tree pre-flight (refuses to proceed
   with uncommitted `meta/` or `.accelerator/` changes).
2. See 0004's own Step 0c pre-flight extending the dirty-tree check
   to the full `accelerator:paths` scan corpus, and Step 0d's Perl
   version check. Both run BEFORE any filesystem or config mutation.
   - No VCS detected → refuses to run. Bypass with
     `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` (user accepts no-rollback
     risk).
   - Perl < 5.30 → refuses to run (Step 3's regex requires
     lookbehind).
3. See a jj op-id printed to stderr at script entry, naming the
   rollback target: `0004: roll back with: jj op restore <op-id>`.
4. See the per-migration preview line for 0004.
5. See zero, one, two, three, or four `0004: renamed paths.X → paths.Y …`
   informational lines depending on which legacy keys the user has
   explicitly overridden (no notification fires for default-resolved
   keys). The `templates.research → templates.codebase-research`
   rename has its own notification line if the user has that
   override.
6. See moves logged via `_move_if_pending`, plus a Step 3 banner
   indicating how many configured paths Step 3 is scanning
   (transparency without a dry-run).
7. See per-key diagnostics if a legacy directory cannot be
   `rmdir`'d (unexpected residue → manual cleanup recommended).
8. Have their inbound references rewritten across every markdown
   file under every configured `paths.*` directory, anchored on
   path-segment boundaries to avoid `meta/research-templates/`-style
   false positives.

**Recovery from a botched migration** is `jj op restore <op-id>`
(operation ID is captured at script entry and printed to stderr,
e.g., `0004: pre-migration jj op-id: abc123de`) or `git reset --hard`
to the pre-migration commit. The framework explicitly forbids
dry-run mode (`migrate/SKILL.md:41`); the Step 0 pre-flights provide
last-checkpoint signals before mutation.

For config-file specifically: the migration creates `*.0004.bak`
copies of `.accelerator/config.md` and `.accelerator/config.local.md`
before the first rewrite. If the rewrite produces unexpected output,
`cp .accelerator/config.md.0004.bak .accelerator/config.md`
restores the pre-migration content without affecting the filesystem
moves. Backup files survive the migration; remove them once the
result is verified.

**Atomicity and partial-failure recovery.** All pre-flights run in
Step 0 before any mutation, so the most common failure modes (dirty
corpus, missing Perl, mixed-state config, destination collision,
no VCS) abort with zero filesystem/config change. The remaining
mutation phases are each individually idempotent:
- Step 1 (moves): `_move_if_pending` is per-file idempotent; partial
  Step 1 followed by re-run completes outstanding moves.
- Step 2 (config rewrites): rewrites are idempotent because the
  legacy key is removed after a successful rewrite; second-run probe
  returns empty.
- Step 3 (inbound rewriting): negative-lookbehind regex prevents
  double-substitution; rewriting `meta/research/foo.md` to
  `meta/research/codebase/foo.md` does not match on a second pass.

**Environment variables.** Two distinct safety bypasses with
separate names:
- `ACCELERATOR_MIGRATE_FORCE=1` — framework-level dirty-tree
  bypass (existing).
- `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` — 0004-specific bypass for
  the no-VCS-detected refusal.

**Scope limitations made explicit in the migration's stdout output:**
- The inbound-link rewriter only scans directories surfaced by
  `accelerator:paths`. Documentation outside these paths (e.g.,
  top-level `README.md`, `ARCHITECTURE.md`) is not rewritten — a
  closing diagnostic line names the scanned directories and
  recommends a manual `grep` sweep of any out-of-corpus content.
- The migration runs once. Content imported into the repo *after*
  the migration runs (branch merges, sibling-repo copies, pasted
  legacy paths) is not rewritten. To rescan: manually remove the
  `0004-…` line from `.accelerator/state/migrations-applied` and
  re-run `/accelerator:migrate`. (There is no env-var shortcut for
  this — the state file is the authoritative gate.)
- **Override + legacy-default references edge case.** When the
  user has `paths.design_inventories` or `paths.design_gaps`
  overridden, Step 1 honors the override (no file move) and Step 3
  excludes the design pair from inbound rewriting. If the user's
  corpus contains references to the *legacy default* path
  (`meta/design-inventories/...`) — for example because the
  override was added recently while older content still uses the
  default literal — those references are not rewritten and may
  not resolve. The closing diagnostic recommends `grep -rn
  'meta/design-inventories\b\|meta/design-gaps\b' <configured-paths>`
  as a post-migration sweep for users with overrides.
- **jj op-id capture timing.** The op-id is captured at script
  entry before the framework's dirty-tree check. Under
  `ACCELERATOR_MIGRATE_FORCE=1`, the captured op-id anchors a
  snapshot that includes uncommitted edits — rolling back will
  also revert those edits. Commit first when in doubt.

**`migrate-common.sh` extraction (deferred).** This work lifts
patterns from migrations 0001, 0002, and 0003. Plan review 1
flagged the resulting duplication as a maintainability concern.
Decision: defer extraction to a separate work item rather than
expanding 0056's scope. Rationale: (1) atomic-commit constraint
already large; (2) migrations are conventionally snapshot-frozen
against framework state at authoring time, so extraction is a
framework-evolution decision distinct from this migration's
content; (3) the duplication concentrates in three discrete
patterns (`_move_if_pending`, `probe_paths_key`, three-rewriter
inbound) with stable interfaces — extraction can be done later
without changing migration semantics. A follow-up work item will
track this.

**Upgrade sequence (D2 — documentation only).** Skills do not
gate themselves on pending migrations. The SessionStart hook warns
when migrations are pending. Users running skills between plugin
upgrade and migration will see results written to or read from
new-default paths, which may not yet exist on disk. The CHANGELOG
entry and `migrate/SKILL.md` document this hazard and the
recommended sequence (pull → migrate → resume).

## References

- Original work item: `meta/work/0056-restructure-meta-research-into-subject-subcategories.md`
- Source research: `meta/research/2026-05-11-0056-restructure-meta-research-into-subject-subcategories.md`
- Source note: `meta/notes/2026-05-02-research-directory-subcategory-restructure.md`
- Builds on: `meta/work/0030-centralise-path-defaults.md`,
  `meta/work/0052-make-documents-locator-paths-config-driven.md`
- Pattern sources:
  - `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:111-134` (config-key rewrite)
  - `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:113-313` (three-rewriter inbound rewrite)
  - `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:32-64,66-102` (idempotent moves + probe_paths_key)
- ADR: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`

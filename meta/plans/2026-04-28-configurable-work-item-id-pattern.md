---
date: "2026-04-28T14:37:15Z"
type: plan
skill: create-plan
ticket: ""
status: ready
---

# Configurable Work-Item ID Pattern Implementation Plan

## Overview

Replace the hard-coded `NNNN-` work-item filename prefix with a configurable
prefix DSL (`{project}-{number:04d}` and similar) so that teams using external
trackers like Jira or Linear can adopt project-coded IDs (`ENG-1234`,
`PROJ-0042`) while existing users see no behavioural change. The work covers
six phases: a pattern compiler, a generalised number allocator and ID
resolver, skill-prose updates, multi-project handling in
`extract-work-items`, a strictly-additive skip-tracking extension to the
migration framework, and a migration that renames the legacy corpus into the
configured pattern.

ADRs are deliberately out of scope: this work primarily *implements* and
*extends* decisions already captured by ADR-0017 (configuration extension
points), ADR-0022 (work-item terminology), and ADR-0023 (migration
framework). The pattern DSL grammar is captured in this plan plus the
companion research doc; if it grows new tokens (`{type}`, `{date}`) a
follower ADR may be warranted at that point. The skip-tracking extension
is strictly additive to ADR-0023's framework — it does not redirect the
existing decision and does not require its own record.

## Current State Analysis

The plugin currently encodes the `NNNN` convention in two distinct ways:

- **Allocation** — `skills/work/scripts/work-item-next-number.sh:43-72` scans
  `[0-9][0-9][0-9][0-9]-*`, parses leading digits with
  `grep -oE '^[0-9]+'`, and emits `printf "%04d\n"`. A 9999 overflow guard
  exists at lines 60-69.
- **Recognition** — seven literal globs and one `^[0-9]+$` discriminator
  scattered across the work-item skills:
  - `skills/work/create-work-item/SKILL.md:79-81` (numeric vs path
    discriminator)
  - `skills/work/create-work-item/SKILL.md:507-514` (H1 and frontmatter
    `work_item_id` doc)
  - `skills/work/list-work-items/SKILL.md:109,127,147-149,174-177` (glob,
    derive-from-filename, parent normalisation)
  - `skills/work/update-work-item/SKILL.md:47,135-137`
  - `skills/work/review-work-item/SKILL.md:41`
  - `skills/work/extract-work-items/SKILL.md:352,357-364` (collision check
    and allocation)

The userspace config system (`scripts/config-read-value.sh` plus
`config-common.sh`) already supports arbitrary 2-level keys, so
`work.id_pattern` and `work.default_project_code` need no plumbing changes —
only validation and consumption.

The migration framework
(`skills/config/migrate/scripts/run-migrations.sh:42-117`) uses a single
applied-IDs state file (`meta/.migrations-applied`) and has no concept of a
*skipped* migration: a user who declines a migration faces the same prompt
on every subsequent run.

The integration test runner is `mise run test` →
`invoke test.integration` (`tasks/test.py`) which executes:
`scripts/test-config.sh`, `skills/decisions/scripts/test-adr-scripts.sh`,
`skills/work/scripts/test-work-item-scripts.sh`,
`scripts/test-lens-structure.sh`, `scripts/test-boundary-evals.sh`,
`scripts/test-evals-structure-self.sh`,
`scripts/test-evals-structure.sh`, `scripts/test-hierarchy-format.sh`,
`scripts/test-format.sh`,
`skills/config/migrate/scripts/test-migrate.sh`. Skill-level evals live
under each skill's `evals/` directory (e.g.
`skills/work/create-work-item/evals/{evals.json,benchmark.json,benchmark.md}`).

### Key Discoveries

- `scripts/config-read-value.sh:6-13` already supports
  `<section>.<key>` reads; new keys are zero-config to plumb.
- `skills/work/scripts/test-work-item-scripts.sh:1-50` establishes the
  bash-test pattern: `setup_repo()` makes a `mktemp` dir with `.git`,
  assertions come from `scripts/test-helpers.sh`, the harness runs the
  script being tested as a black box.
- `skills/config/migrate/scripts/test-migrate.sh:19-60` shows the migrate
  test idioms: `assert_contains`, `assert_file_exists`,
  `assert_file_not_exists` are local to that harness and may need to be
  reused or duplicated.
- `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
  (137 lines) is the only existing migration and is the template for new
  ones (`# DESCRIPTION:` line 2, idempotent self-detection, atomic write).
- `skills/work/create-work-item/evals/evals.json` shows the eval schema:
  `{skill_name, evals: [{id, name, prompt, expected_output, files}]}`.
- The skill-creator skill is at
  `~/.claude/plugins/marketplaces/claude-plugins-official/plugins/skill-creator/skills/skill-creator/SKILL.md`
  — the user has it available and wants it driving every SKILL.md edit
  cluster.

## Desired End State

After all six phases are merged:

1. `scripts/config-read-value.sh work.id_pattern '{number:04d}'` reads the
   configured pattern, defaulting to the current behaviour.
2. `scripts/config-read-value.sh work.default_project_code` reads the
   default project code (empty when unset).
3. `skills/work/scripts/work-item-next-number.sh --project PROJ --count 3`
   allocates project-scoped sequential IDs against the corpus
   (`PROJ-0042`, `PROJ-0043`, `PROJ-0044`).
4. `skills/work/scripts/work-item-resolve-id.sh <input>` classifies any
   user-supplied identifier (full ID, legacy NNNN, bare number, path) and
   returns the canonical work-item file path.
5. `create-work-item`, `update-work-item`, `review-work-item`, and
   `list-work-items` work end-to-end on **both** default and
   `{project}-{number:04d}` patterns; legacy `NNNN-*.md` files remain
   discoverable after a structural pattern change (broadened `*.md`
   discovery glob with frontmatter validation).
6. `extract-work-items` presents a per-row project-code amendment table
   when the configured pattern contains `{project}`; per-project
   allocation occurs in a single allocator call per distinct project code.
7. `skills/config/migrate/scripts/run-migrations.sh --skip <id>` and
   `--unskip <id>` provide opt-out/opt-in tracking via
   `meta/.migrations-skipped`; the pending list excludes both applied and
   skipped IDs.
8. Migration `0002-rename-work-items-with-project-prefix.sh` renames
   legacy `NNNN-*.md` work items to the configured pattern with the
   default project code, rewrites `work_item_id` frontmatter to the full
   ID, and updates frontmatter ID-bearing fields, markdown links, and
   strict-form prose references repo-wide across `meta/`.

### Verification

Run the full integration suite (`mise run test`) — all existing tests pass
plus the new tests added in each phase. Manually exercise the
configured-pattern flow on a fresh repo with
`work.id_pattern: "{project}-{number:04d}"` and
`work.default_project_code: "PROJ"`: `/create-work-item`,
`/list-work-items`, `/extract-work-items` from a brainstorm doc, then
`/accelerator:migrate` against a corpus prepared as legacy NNNN files.

## What We're NOT Doing

- **Not changing ADRs.** `skills/decisions/scripts/adr-next-number.sh`,
  `skills/decisions/create-adr/SKILL.md`,
  `skills/decisions/extract-adrs/SKILL.md`, and all ADR globs and prose
  remain literal `NNNN`. ADRs always live in the repo and have no
  external-tracker concept.
- **Not touching plans.** Date-prefixed plan filenames are unaffected.
- **Not touching internal migration numbering.** Migrations themselves
  remain `[NNNN]-name.sh` — they are plugin-internal sequence numbers,
  not work items.
- **Not migrating work items on pattern change after first use.** Width
  changes (`{number:04d}` → `{number:05d}`) are absorbed by the
  width-agnostic scan regex (matches `[0-9]+`; width is enforced only
  at format time). Structural changes leave legacy files coexisting
  with their original IDs. The migration framework re-evaluates 0002
  on each invocation while it is unapplied (Phase 6 emits
  `MIGRATION_RESULT: no_op_pending` when the pattern lacks
  `{project}`), so a later config change to a `{project}` pattern is
  picked up automatically on the next `/accelerator:migrate` rather
  than requiring hand-editing of state files.
- **Not rewriting bare 4-digit numbers in prose** during migration —
  too lossy to disambiguate from other numerals.
- **Not adding a Rust CLI** in this work. Bash-only stays the
  implementation strategy; the future Rust port is a transliteration
  target.
- **Not adding new tokens beyond `{project}` and `{number}`.** The DSL
  is extensible (`{type}`, `{date}`) but those are out of scope.
- **Not changing `meta/.migrations-applied` format or semantics.**
  The skip-tracking work is strictly additive via a sibling file.
- **Not updating CHANGELOG/version per phase.** Bumps and changelog
  entries land at three natural milestones; the plan stays neutral on
  PR slicing within each milestone:

  - **After Phase 4** (additive, default behaviour preserved): one
    `Added` entry per new public surface — `work.id_pattern` and
    `work.default_project_code` config keys; `--project` flag on
    `work-item-next-number.sh`; `work-item-resolve-id.sh` script;
    multi-project amendment flow in `/extract-work-items`.
  - **After Phase 5** (additive to migration framework): `Added`
    entries for `--skip` / `--unskip` flags on `run-migrations.sh`,
    the `MIGRATION_RESULT: no_op_pending` runner contract, and
    `scripts/atomic-common.sh` shared helper. May ship standalone.
  - **After Phase 6** (breaking only when explicitly opted in): one
    `Added` entry for migration `0002-rename-work-items-with-project-prefix`
    plus the migration-applied stay-pending semantics for unconfigured
    state. Note explicitly that no breaking change is forced on users
    with default-pattern config.

  Each milestone's PR description includes the matching changelog
  entries verbatim so reviewers see the user-facing surface in
  context.

## Implementation Approach

The phasing follows the dependency chain identified in the research
(`meta/research/2026-04-28-configurable-work-item-id-pattern.md:469-660`):

```
Phase 1 (work-item-common.sh + compiler + config)
  └─ Phase 2 (allocator + resolver)
       └─ Phase 3 (skill wiring + frontmatter-consumer audit)
            └─ Phase 4 (extract-work-items)
                  │
                  ▼
                  Phase 6 (migration 0002) ◀── Phase 5 (atomic-common.sh +
                                                        skip-tracking +
                                                        MIGRATION_RESULT contract)
```

Phase 5's `MIGRATION_RESULT: no_op_pending` runner contract is a hard
prerequisite for Phase 6's "stay pending when pattern lacks
`{project}`" behaviour, in addition to the original skip-tracking
prerequisite.

Each phase is **test-driven** for code (write or extend the bash test
harness first, then implement until green) and **eval-driven** for skills
(invoke the `skill-creator:skill-creator` skill, write or update the
`evals.json` cases first, then iterate the SKILL.md prose until the
benchmark holds — per the skill-creator's eval-iterate loop).

**Skill-creator iteration stop-loss**: each SKILL.md edit cluster gets
a maximum of **three** iteration rounds with skill-creator. If after
three rounds the benchmark still falls below the per-mode threshold
(100% on `default`-mode evals, ≥95% on `configured`-mode evals), the
implementer pauses to:

- Inspect the failing eval(s) — is the prose unclear, the eval
  ambiguous, or the test fixture wrong?
- Either: (a) split a too-broad eval into multiple narrower ones,
  (b) revise the eval's expected output if the prose change made the
  prior expectation obsolete, or (c) escalate by raising the issue
  on the PR with the failing eval transcript and the three iteration
  attempts attached as evidence.

The benchmark transcripts from the final iteration of each cluster
are archived in the PR description (or linked from it) so reviewers
can see how the eval threshold was met.

Cross-cutting principles:

- **Bash-only.** No new dependencies beyond bash, awk, grep, find.
  `sed -i` is avoided entirely (BSD/GNU portability) — atomic
  rewrites go through `scripts/atomic-common.sh` (Phase 5).
- **Helper factoring with named modules.** Two shared bash libraries:
  `skills/work/scripts/work-item-common.sh` (pattern compile, ID
  canonicalisation, legacy detection, frontmatter predicates) and
  `scripts/atomic-common.sh` (atomic file writes). Resolver,
  list-work-items, update-work-item, allocator, and migration 0002
  source these rather than duplicating logic.
- **Stable error-code prefixes.** Errors emitted by the work-item
  helpers start with a stable `E_*` prefix; tests pin the prefix and
  the prose after the colon is free-form.
- **No behavioural change on default config.** Phases 1-3 must leave a
  user with no `work.*` config seeing today's filenames and H1 format
  bit-identically. Frontmatter `work_item_id` shifts from bare to
  quoted string (consistent across all patterns, type contract
  documented in configure SKILL.md). The frontmatter-consumer audit
  in Phase 3 ensures no consumer breaks. Locked in by the
  default-pattern allocator golden file and create-work-item golden
  fixture.
- **Eval-mode discipline.** Every eval is tagged
  `pattern_mode: default | configured | both`; `default`-mode evals
  must pass at 100% (the regression boundary), other modes at ≥95%.
- **Forward-compat preserved.** Unknown migration IDs in either state
  file (`.migrations-applied`, `.migrations-skipped`) are preserved on
  rewrite with a warning, mirroring the existing semantics.

## Phase 1: Pattern Compiler and Config Schema

### Overview

Introduce the pattern DSL infrastructure with no behavioural change for
existing users. The compiler is the foundation every later phase depends
on; default-pattern callers must be byte-identical to today's output for
filenames and the H1 (frontmatter quoting changes per Phase 3).

### Pattern DSL Reference

This subsection is the canonical specification for the pattern DSL —
later phases and the configure SKILL.md prose reference it instead of
re-deriving the rules from research.

**Tokens** (this initial scope; future tokens are out of scope):

- `{number[:format]}` — required, exactly one occurrence. The
  `:format` is a printf width spec of the form `0Nd` where N is a
  positive integer (e.g. `04d`, `05d`). If omitted, defaults to
  `04d`. Width is enforced at *generation* time; the scan regex
  matches `[0-9]+` (any digit run), so legacy and pre-existing files
  remain visible after a width change and the allocator can detect
  out-of-width files.
- `{project}` — optional, at most one occurrence. Substituted with a
  project value at use time (from `--project` flag or
  `work.default_project_code`).
- `{{` and `}}` — escaped literals. `{{` produces a literal `{` in
  the format and `\{` in the scan regex; `}}` similarly.

**Validation rules**:

1. Pattern must contain at least one `{number...}` token.
2. No filesystem-hostile chars (`/`, `\`, `:`, `*`, `?`, `<`, `>`,
   `|`, `"`) outside token format specs.
3. Adjacent dynamic tokens must have at least one literal char
   between them (e.g. `{project}{number}` is rejected;
   `{project}-{number}` is accepted).
4. `{number}` format spec must be `0Nd` form (zero-padded fixed
   width). Non-padded specs like `%d` are rejected so the overflow
   guard cap (`10^N - 1`) is well-defined.
5. Project value (validated at use time, not pattern time) must
   match `[A-Za-z][A-Za-z0-9]*`. This is intentionally restrictive
   for the initial scope: it covers Jira/Linear-style alphanumeric
   project keys (`PROJ`, `ENG`, `ENG2`) but rejects keys with
   internal hyphens or underscores (`PROJ-FE`, `proj_alpha`).
   Widening this regex requires updating the scan-regex generator to
   defensively escape regex metachars (the compiler already does
   per-char escaping, so this is a docs/rule change rather than a
   code change). Out of scope here; treat the restriction as a
   known limitation documented in the configure SKILL.md.
6. Round-trip: `format(N)` then parse must recover `N` for every
   `N ∈ [1, 10^width - 1]`.

**Compiled outputs**:

- *Scan regex* (ERE): `^<literal-prefix>([0-9]+)(<literal-suffix>)`,
  where prefix and suffix are the surrounding pattern text with
  `{project}` substituted. Capture group 1 is always the number.
- *Format string* (printf): `<literal-prefix>%0Nd<literal-suffix>`,
  with `{project}` substituted.

**Examples**:

| Pattern | Project | Scan regex | Format |
| --- | --- | --- | --- |
| `{number:04d}` | (n/a) | `^([0-9]+)-` | `%04d` |
| `{number:05d}` | (n/a) | `^([0-9]+)-` | `%05d` |
| `{project}-{number:04d}` | `PROJ` | `^PROJ-([0-9]+)-` | `PROJ-%04d` |
| `{project}-{number:04d}` | `OTHER` | `^OTHER-([0-9]+)-` | `OTHER-%04d` |

(The trailing `-` in the scan regex comes from the convention that
the pattern is followed by `-<slug>.md` in the filename; the pattern
itself does not include the trailing `-`.)

**Stable error-code prefixes** (testable contract): see Compiler CLI
subsection below for the full list.

### Changes Required

#### 1. Shared work-item helper library

**File**: `skills/work/scripts/work-item-common.sh` (new)

**Changes**: Sourcable bash library exposing the named functions that
later phases consume. Centralises the legacy-format detection, ID
canonicalisation, and pattern-aware predicates so the same logic is
not re-implemented in the resolver, list-work-items, update-work-item,
and migration 0002. Functions:

All functions follow a consistent calling convention: result on
**stdout**, errors on **stderr** with stable `E_*` prefix, exit code
0 on success / non-zero on error. Boolean predicates use exit code
only (no stdout).

- `wip_compile_scan <pattern> <project_value>` — echoes the scan
  regex (ERE; capture group 1 is the number) on stdout. Exits non-zero
  with `E_PATTERN_*` if the pattern is invalid.
- `wip_compile_format <pattern> <project_value>` — echoes the printf
  format string on stdout. Same error semantics.
- `wip_validate_pattern <pattern>` — exit 0 valid, non-zero invalid
  with `E_PATTERN_*` on stderr. No stdout output.
- `wip_canonicalise_id <input> <pattern> <project_value>` — echoes the
  canonical full-ID string on stdout. Used by both the resolver and
  parent-field comparison in update-work-item / list-work-items.
- `wip_parse_full_id <id> <pattern>` — echoes `<project>\t<number>`
  on stdout (tab-separated for easy `cut -f` consumption); exits
  non-zero if the ID does not parse.
- `wip_is_legacy_id <id>` — predicate: exit 0 iff the ID matches
  `^[0-9]+$` with ≤4 digits AND has at least one non-zero digit
  (work-item numbering starts at 1, so `0`, `00`, `0000` are
  rejected as degenerate). No stdout. Single source of truth for
  the legacy convention.
- `wip_pad_legacy_number <input>` — echoes the zero-padded numeric
  string (4 digits) on stdout.
- `wip_is_work_item_file <path>` — predicate: exit 0 iff the file
  has YAML frontmatter and a `work_item_id` field with a non-empty
  string value. Used by the broadened `*.md` discovery glob in
  list-work-items. No stdout.
- `wip_extract_id_from_filename <name> <pattern>` — echoes the
  extracted ID on stdout via the compiled scan regex; exits non-zero
  if the filename does not match.
- `wip_pattern_max_number <pattern>` — echoes `10^N - 1` for the
  configured `{number}` width N on stdout. Used by the allocator
  overflow guard.

The standalone `work-item-pattern.sh` script (next subsection) becomes
a thin CLI wrapper around `wip_compile_scan` / `wip_compile_format` /
`wip_validate_pattern`. Consumers that source the library skip the
subprocess overhead; consumers that prefer the script (e.g. testing,
ad-hoc shell use) get an identical result.

The full list of stable error-code prefixes is in the Compiler CLI
subsection below. Tests assert error messages start with the relevant
prefix; the prose after the colon is free-form and may be edited
without breaking tests.

#### 2. Pattern compiler CLI script

**File**: `skills/work/scripts/work-item-pattern.sh` (new)

**Changes**: Thin CLI wrapper around `work-item-common.sh`. Implements
pattern parsing, validation, and compilation to
`(scan_regex, format_string)` for callers that prefer subprocess
invocation. Supports tokens `{project}` and `{number[:spec]}`. Exposes
three modes:

```bash
work-item-pattern.sh --validate "<pattern>"
# Exits 0 on valid; 2 on invalid (stderr starts with E_PATTERN_*); 1 on usage error.

work-item-pattern.sh --compile-scan "<pattern>" "<project_value>"
# Emits the scan regex on stdout with project_value substituted in.
# project_value may be empty when pattern has no {project} token.

work-item-pattern.sh --compile-format "<pattern>" "<project_value>"
# Emits the printf format string on stdout with project_value substituted.
```

Compiler contract:

- Regex flavour: ERE (POSIX extended). Tests assert `grep -E`
  compatibility and bash `=~` compatibility.
- Capture group: index 1 is always the number; subsequent groups are
  reserved.
- Project-value escaping: the `{project}` token's substituted value
  is emitted as literal text in both regex and printf format. Rule 5
  ensures only `[A-Za-z][A-Za-z0-9]*` characters reach the
  substitution, so no regex-metachar escaping is needed in practice;
  the compiler still defensively escapes (`\Q...\E`-equivalent in
  bash: per-char escape) so a future rule-5 widening is safe.
- Output channel: success → stdout; error → stderr with stable
  `E_PATTERN_*` code prefix.
- Exit codes: 0 success, 1 usage error, 2 validation failure.

Validation rules and token grammar are defined in the **Pattern DSL
Reference** subsection above; the compiler implements those rules
verbatim. Stable error codes:

- `E_PATTERN_NO_NUMBER_TOKEN` (rule 1)
- `E_PATTERN_HOSTILE_CHAR` (rule 2)
- `E_PATTERN_ADJACENT_TOKENS` (rule 3)
- `E_PATTERN_BAD_FORMAT_SPEC` (rule 4)
- `E_PATTERN_BAD_PROJECT_VALUE` (rule 5; raised at use time)
- `E_PATTERN_OVERFLOW` (allocator: `HIGHEST + COUNT > cap`)
- `E_PATTERN_MISSING_PROJECT` (consumer: pattern has `{project}` with
  no value supplied)
- `E_PATTERN_PROJECT_UNUSED` (consumer: `--project` given but pattern
  lacks `{project}`)

#### 3. Config-time validation hook

**File**: `scripts/config-read-value.sh`

**Changes**: No code change required — the generic reader already
supports `work.id_pattern` and `work.default_project_code`. Validation
happens at *consumer* call sites (Phase 2 onwards). The `--validate`
mode of `work-item-pattern.sh` is the entry point.

#### 4. Configure skill documentation

**File**: `skills/config/configure/SKILL.md`

**Changes**: Add a new "Work Items" subsection mirroring the existing
`agents`/`review`/`paths`/`templates` subsections. Document
`work.id_pattern` (default `{number:04d}`) and
`work.default_project_code` (default empty), referring to the Pattern
DSL Reference (Phase 1) for the full token grammar and validation
rules rather than restating them inline. Cover:

- Choosing between default and project-coded patterns (use case
  guidance — when does a team want `{project}-{number:04d}`?).
- The work_item_id frontmatter type contract: always a quoted YAML
  string post-Phase-3, regardless of pattern shape.
- The no-migration-on-change semantics for width changes; the
  re-evaluation behaviour for structural changes (migration 0002
  stays pending and re-runs on later config updates).
- The "skip vs configure default project code" decision at
  migration time (cross-link to the Phase 6 prerequisite error
  message).

> Driven through the `skill-creator:skill-creator` skill. Bootstrap
> `skills/config/configure/evals/evals.json` if absent (with at least
> two cases covering the new "Work Items" subsection's content) rather
> than skipping the eval-driven discipline. The plan-wide
> eval-iteration stop-loss applies (max three rounds; escalate or
> revise eval expectations on stuck benchmarks).

**Config-key recognition**: configure SKILL.md's documentation lists
all recognised `work.*` keys (`work.id_pattern`,
`work.default_project_code`); any other `work.*` key in the config
is unknown. Phase 1 does NOT add a strict-validation step that warns
on unknown keys (the generic config reader is unaware of key
allowlists), but the documentation makes the recognised set
discoverable. A future config-validator pass could enforce this.

Also document the **work_item_id frontmatter type contract**: the
field is always a YAML string (quoted), regardless of the configured
pattern. This is true for new work items created under both
`{number:04d}` and `{project}-{number:04d}` patterns post-Phase-3, and
is enforced by migration 0002 for existing files. Consumers must
treat the value as a string; do not coerce to integer.

#### 5. Tests

**File**: `skills/work/scripts/test-work-item-pattern.sh` (new)

**Changes**: New bash test harness following the established pattern in
`test-work-item-scripts.sh`. Cases (TDD — written first):

- Default pattern `{number:04d}` validates and compiles to scan regex
  `^([0-9]+)-` and format `%04d`.
- `{project}-{number:04d}` with `project_value=PROJ` compiles to scan
  regex `^PROJ-([0-9]+)-` and format `PROJ-%04d`.
- Width variants: `{number:05d}` → format `%05d`, scan regex unchanged
  `^([0-9]+)-`. Decision: width is a *generation* concern only;
  scanning is width-agnostic so legacy 4-digit files remain visible
  after a 5-digit pattern change AND a hand-created 5-digit file
  remains visible under a 4-digit pattern (the allocator can detect
  and report the overflow rather than silently ignoring the file).
  See research lines 281-292.
- Validation rejections: missing `{number}` (rule 1), hostile char in
  literal (rule 2), `{project}{number}` adjacency (rule 3), bad format
  spec `{number:foo}` (rule 4), bad project value `_low` (rule 5).
- Round-trip property at `N ∈ {1, 9, 10, 99, 100, 999, 1000, 9999}`
  for default `{number:04d}` and at `N ∈ {1, 9999, 10000, 99999}` for
  `{number:05d}` (digit-count transitions and width boundary). For
  `{project}-{number:04d}`, sample `N ∈ {1, 9999}` with two project
  values: single-char `A` and multi-char `PROJ` (rule 6).
- Escape sequences: literal `{{` in pattern produces literal `{` in
  format and `\{` in scan regex.
- Project-value regex escaping: a project value containing a regex
  metachar (validation rule 5 currently rejects this, but the
  compiler must defensively escape anyway) does not break the
  compiled scan regex. A project value containing characters
  forbidden by rule 5 causes `--validate` to fail at config time
  before the compiler sees the value.

**File**: `scripts/test-config.sh`

**Changes**: Add cases that:

- `work.id_pattern` reads correctly when set in
  `.claude/accelerator.md`.
- `work.default_project_code` reads correctly.
- Local override in `.claude/accelerator.local.md` wins for both keys.

**File**: `tasks/test.py`

**Changes**: Add the following invocations between the existing
work-item script tests and lens structure lint, in order:

- `context.run("skills/work/scripts/test-work-item-pattern.sh")`
- `context.run("scripts/test-atomic-common.sh")` (Phase 5 dep — added
  here when atomic-common.sh ships)
- `context.run("scripts/test-evals-mode-tagging.sh")` (Phase 3 dep —
  added when eval mode-tagging ships)

The order is staged: each test file is added when its underlying
script is in the tree. Phase 1 introduces only the first.

### Success Criteria

#### Automated Verification

- [x] New compiler tests pass: `bash skills/work/scripts/test-work-item-pattern.sh`
- [x] Existing work-item script tests still pass: `bash skills/work/scripts/test-work-item-scripts.sh`
- [x] Config tests pass with new cases: `bash scripts/test-config.sh`
- [x] Full integration suite passes: `mise run test`
- [x] Format checks pass: `bash scripts/test-format.sh`

#### Manual Verification

- [x] `work-item-pattern.sh --validate "{number:04d}"` exits 0
- [x] `work-item-pattern.sh --validate "no-number"` prints a clear error
      naming validation rule 1
- [x] `work-item-pattern.sh --compile-scan "{project}-{number:04d}" "PROJ"`
      produces a regex (`^PROJ-([0-9]+)-`) that matches `PROJ-0042-foo.md`
      and not `OTHER-0042-foo.md`
- [x] `work-item-pattern.sh --compile-scan "{number:04d}" ""` matches
      both `0042-foo.md` (4 digits) and `12345-foo.md` (5 digits) — width
      is enforced only at format time

## Phase 2: Next-Number Generaliser and ID Resolver

### Overview

Make the work-item allocation and lookup primitives pattern-aware. Still
no SKILL.md changes; default-pattern users see no difference.

### Changes Required

#### 1. Generalise the allocator

**File**: `skills/work/scripts/work-item-next-number.sh`

**Changes**: Rewrite around the Phase 1 compiler. Sources
`work-item-common.sh` for in-process compile and canonicalisation.
Add `--project CODE` flag. Resolution order for the project value:

1. Explicit `--project` flag.
2. `work.default_project_code` config value.
3. If pattern has `{project}` and neither is set: error
   `E_PATTERN_MISSING_PROJECT: pattern '<pat>' contains {project}
   but no value supplied — pass --project or set
   work.default_project_code`.
4. If pattern lacks `{project}` and `--project` is given: error
   `E_PATTERN_PROJECT_UNUSED: --project is meaningless for pattern
   '<pat>' (no {project} token)`.

Replace the literal `[0-9][0-9][0-9][0-9]-*` glob (line 47) with a
`find … | grep -E "<scan_regex>"` pipeline using the compiled regex
from `wip_compile_scan`. Replace `grep -oE '^[0-9]+'` (line 50) with
the captured group from the same regex. Output uses the compiled
format string.

Overflow guard adapts: the cap is `10^N - 1` where N is the configured
`{number}` width (e.g. `{number:04d}` → cap 9999, `{number:05d}` → cap
99999). The guard fires when `HIGHEST + COUNT > cap`. The guard
reports the offending HIGHEST value so users can spot a stray
out-of-width file in the corpus that has consumed the number space.
Phase 1 validation rule 4 enforces zero-padded width specs (so the
cap is always well-defined); non-padded specs like `%d` are rejected
at config time.

#### 2. ID resolver

**File**: `skills/work/scripts/work-item-resolve-id.sh` (new)

**Changes**: Given a user input string and the configured pattern,
classifies the input and returns the canonical resolved path on stdout.

Classification splits into two stages: a pure `classify_input` step
(returns one of `path | full_id | bare_number | invalid`, no
filesystem access) and a `resolve_classified` step that probes the
corpus for the classified shape.

Classification of input string (research lines 332-348):

1. **Path** — input starts with `./`, `/`, or contains `/`. Path-shaped
   inputs are *always* resolved as paths: existing → echo absolute path
   (exit 0); not existing → exit 3 ("no work item at path"). Path-shaped
   inputs are never reclassified as IDs even if the basename happens to
   look like an ID.
2. **Full ID matching the configured pattern** (e.g. `PROJ-0042`).
3. **Bare number** (matches `^[0-9]+$`).
4. Otherwise: **Invalid** — exit 1 with reason.

Resolution for full-ID inputs (step 2):

- Glob `<work_dir>/<id>-*.md`. Single match → echo path; multiple →
  exit 2 with candidates; zero → exit 3.

Resolution for bare-number inputs (step 3) is *ambiguity-aware* and
collects candidates from all reachable shapes, then disambiguates
once at the end:

a. **Project-prepended candidate** (only if pattern has `{project}` and
   `work.default_project_code` is set): zero-pad the number to the
   configured width, prepend the default project code, glob
   `<work_dir>/<full-id>-*.md`. Add any matches to the candidate set,
   tagged `project-prepended`.
b. **Legacy candidate** (only if the input has ≤4 digits): zero-pad to
   4 digits, glob `<work_dir>/[0-9][0-9][0-9][0-9]-*.md` matching that
   number. Add any matches to the candidate set, tagged `legacy`.
c. **Pattern-shape candidate** (only if pattern lacks `{project}`):
   zero-pad to the configured width, glob the configured pattern's
   shape. Add any matches to the candidate set, tagged `pattern-shape`.
d. **Cross-project scan candidate** (only if pattern has `{project}`,
   regardless of whether `default_project_code` is set): zero-pad the
   number to the configured width, glob
   `<work_dir>/*-<padded-number>-*.md` to find any project-coded files
   matching the bare number across *every* observed project code in
   the corpus. Filter to matches that conform to the configured
   pattern's scan regex (so only legitimate project-coded files are
   counted, not arbitrary `*-NNNN-*.md` shapes). Add any matches to
   the candidate set, tagged `<project-code>` (the actual project
   code observed for each match, e.g. `PROJ`, `OTHER`).

   When `default_project_code` is set, step (a) and step (d) overlap
   on the default-coded files; deduplicate so each disk file appears
   exactly once in the candidate set, preferring the
   `project-prepended` tag for the default-coded ones.

Resolve the candidate set:

- Empty → exit 3.
- One candidate → echo path; exit 0.
- Multiple candidates → exit 2; stderr lists every candidate with its
  source category (legacy / project-prepended / pattern-shape) so the
  user can see which form to pass to disambiguate.

This ordering ensures that a bare number under a `{project}` pattern
with a default project code never silently picks the legacy file when
a project-coded file with the same number exists; the user always sees
the ambiguity and is prompted to pass the full ID.

Output shape (so callers can distinguish failures cleanly):

- Success: stdout = absolute path; exit 0.
- Invalid input shape: stderr = reason; exit 1.
- Multiple matches: stderr = list of candidates with source categories;
  exit 2.
- No match: stderr = clear "no work item" message; exit 3.

#### 3. Tests

**File**: `skills/work/scripts/test-work-item-scripts.sh`

**Changes**: Add cases (TDD — written first against the new
`work-item-next-number.sh` and `work-item-resolve-id.sh` shapes):

For `work-item-next-number.sh`:

- Default pattern, empty corpus → `0001` (regression).
- Default pattern, corpus `0001-foo.md`, `0002-bar.md` → `0003`
  (regression).
- Pattern `{project}-{number:04d}` + `--project PROJ`, mixed corpus
  `PROJ-0001-x.md`, `PROJ-0003-y.md`, `OTHER-0007-z.md` → `PROJ-0004`
  (per-project scoping).
- Same corpus + `--project OTHER` → `OTHER-0008`.
- Pattern needs `{project}` but neither flag nor config → exits non-zero
  with the documented message.
- Width change `{number:05d}` over a `0001-foo.md` corpus → `00002`
  (legacy 4-digit file contributes to max via width-agnostic scan).
- `--count 3` with `--project PROJ` → 3 sequential IDs in the configured
  format, one per line.
- Overflow: pattern `{number:04d}`, corpus `9999-foo.md` (max), `--count 1`
  → exits non-zero with a clear message naming HIGHEST=9999 and the cap
  9999 (so the user can spot a stray over-width file in the corpus).
- Overflow boundary: pattern `{number:05d}`, corpus `99998-foo.md`,
  `--count 1` → succeeds with `99999`; `--count 2` → fails.
- Out-of-width legacy: pattern `{number:04d}`, hand-created
  `12345-foo.md` in the corpus → allocator reports the offending file
  via the overflow message rather than silently treating it as the new
  HIGHEST and consuming the number space (the user is expected to
  rename the stray file).

For `work-item-resolve-id.sh`:

- Path input that exists → exit 0; returns absolute path.
- Path input that does not exist → exit 3 (path-shaped inputs are never
  reclassified as IDs; consumer skills decide whether to fall back to
  topic-style behaviour).
- Full ID `PROJ-0042` with single matching file → exit 0.
- Full ID `PROJ-0042` with multiple matches → exit 2, lists candidates.
- Full ID `PROJ-0042` with no matching file → exit 3.
- Legacy `0042` against a default-pattern corpus → matches the legacy
  file via the pattern-shape candidate.
- Legacy `0042` against a `{project}` pattern corpus with no default
  project code → matches via the legacy candidate (if a 4-digit file
  exists).
- **Ambiguity test (1)**: corpus contains both `0042-legacy.md` and
  `PROJ-0042-current.md`, pattern is `{project}-{number:04d}`,
  `default_project_code: PROJ`, input `0042` → exit 2; stderr lists both
  candidates tagged as `legacy` and `project-prepended`.
- **Ambiguity test (2)**: same pattern, no default project code,
  corpus contains `PROJ-0042-x.md` and `OTHER-0042-y.md`, input `0042`
  → exit 2 with both candidates listed (tagged `PROJ` and `OTHER` per
  step d's cross-project scan).
- **Ambiguity test (3)**: pattern `{project}-{number:04d}`,
  `default_project_code: PROJ`, corpus contains `PROJ-0042-x.md` and
  `OTHER-0042-y.md`, input `0042` → exit 2 with two candidates: the
  `PROJ` match tagged `project-prepended` (step a), the `OTHER` match
  tagged `OTHER` (step d). Asserts the deduplication rule.
- Single-match cross-project: pattern `{project}-{number:04d}`, no
  default project code, corpus contains only `OTHER-0042-y.md` (no
  `PROJ-0042`), input `0042` → exit 0 returning the OTHER file (step
  d's only match wins).
- Bare `42` (≤4 digits) zero-pads to `0042` and resolves under the
  default project code if pattern requires `{project}` and only the
  project-prepended candidate matches.
- Garbage input (e.g. `foo bar`) → exit 1 (invalid).

### Success Criteria

#### Automated Verification

- [x] All work-item script tests pass: `bash skills/work/scripts/test-work-item-scripts.sh`
- [x] **Default-pattern allocator golden file**: a committed
      `skills/work/scripts/test-fixtures/work-item-next-number.golden`
      file enumerates `(corpus_setup, expected_stdout)` pairs covering
      the eight pre-existing regression cases (empty corpus, single
      file, gaps, sequential, --count variants, etc.). The new test
      `test_default_pattern_golden` runs the rewritten allocator
      against each setup and asserts byte-for-byte identical stdout.
      This locks the default-pattern regression boundary in code.
- [x] Pattern compiler tests still pass: `bash skills/work/scripts/test-work-item-pattern.sh`
- [x] Full integration suite passes: `mise run test`

#### Manual Verification

- [x] In a fresh repo with
      `work.id_pattern: "{project}-{number:04d}"` and
      `work.default_project_code: "PROJ"`:
      `bash work-item-next-number.sh` outputs `PROJ-0001`
- [x] `bash work-item-next-number.sh --project OTHER` outputs `OTHER-0001`
- [x] In the same repo with a legacy `0042-foo.md` present,
      `bash work-item-resolve-id.sh 0042` returns the legacy file path

## Phase 3: Skill Wiring

### Overview

Update the work-item skill prose and globs to use the new primitives.
This is where the feature becomes user-visible for non-batch flows.
Driven through the `skill-creator:skill-creator` skill — every SKILL.md
edit is preceded by an evals update and benchmark run.

### Changes Required

#### 1. create-work-item

**File**: `skills/work/create-work-item/SKILL.md`

**Changes**: Replace the `^[0-9]+$` discriminator block at lines 79-88
with a call to `work-item-resolve-id.sh`. Update the H1 format and
`work_item_id` doc at lines 507-514: H1 becomes `# <full-id>: <title>`
where `<full-id>` is whatever the configured pattern produces;
`work_item_id` frontmatter is **always string-quoted** in *all*
patterns (default and `{project}`), making the contract uniform and
matching the type contract documented in configure SKILL.md (Phase 1).
Files created post-Phase-3 under default config will write
`work_item_id: "0001"` (quoted) instead of today's `work_item_id:
0001` (bare). The filename and H1 remain bit-identical for default
config; only the frontmatter quoting changes.

> Invoke `skill-creator:skill-creator` for this edit. Add evals to
> `skills/work/create-work-item/evals/evals.json` covering: full-ID
> argument resolution (`/create-work-item PROJ-0042`), bare-number
> resolution under a configured `{project}` pattern, legacy-number
> fallback after a pattern change. Iterate SKILL.md prose until the
> benchmark holds.

#### 2. update-work-item

**File**: `skills/work/update-work-item/SKILL.md`

**Changes**: Replace the literal glob and parent-normalisation prose
(`update-work-item/SKILL.md:47,135-137`) with calls to
`work-item-resolve-id.sh` for argument resolution and a pattern-aware
canonicaliser for parent-field comparison (canonicalise to full ID:
strip quotes, accept short and long forms, zero-pad to the configured
width when the input is a bare number).

> Invoke `skill-creator:skill-creator`. Add evals covering full-ID
> arguments and parent-field comparison across both pattern shapes.

#### 3. review-work-item

**File**: `skills/work/review-work-item/SKILL.md`

**Changes**: Replace the literal glob at line 41 with a call to
`work-item-resolve-id.sh`.

> Invoke `skill-creator:skill-creator`. Add an eval for
> `/review-work-item PROJ-0042` argument resolution.

#### 4. list-work-items

**File**: `skills/work/list-work-items/SKILL.md`

**Changes**: Broaden the discovery glob from
`[0-9][0-9][0-9][0-9]-*.md` (lines 109, 127) to `*.md` with
frontmatter validation via `wip_is_work_item_file`, so files that
don't match the *current* pattern (legacy `0042-foo.md` after a
structural pattern change) remain listable. Files with malformed
frontmatter emit a one-line warning to stderr and are skipped (rather
than crashing the listing). Files without `work_item_id` are silently
skipped. Update the filename-prefix-extraction prose at lines 147-149
to use `wip_extract_id_from_filename` (compiled scan regex with
legacy fallback). Update parent-field normalisation at lines 174-177
to use `wip_canonicalise_id`.

**Mixed-corpus discoverability hint**: when the listing detects files
matching the legacy `[0-9]{4}-` shape *and* files matching the
configured `{project}` pattern in the same corpus, prepend a single
informational line to the output:

```
note: mixed prefix corpus detected — N legacy items, M project-prefixed
items. Run /accelerator:migrate to normalise.
```

The note appears once per invocation (not per file), and is suppressed
when the configured pattern lacks `{project}` (because there's no
target form to migrate to).

> Invoke `skill-creator:skill-creator`. Add evals covering: listing a
> mixed-prefix corpus (`PROJ-0001-x.md`, `OTHER-0002-y.md`,
> `0042-legacy.md`) with the discoverability note present; parent
> filtering by full ID (`under PROJ-0042`) and by bare/legacy number
> (`under 42`); listing a corpus with one malformed-frontmatter file
> (warning emitted, listing continues).

#### 5. Frontmatter-consumer audit

**File**: (no single file — audit + targeted updates)

**Changes**: Audit every script and SKILL.md prose block that reads
the `work_item_id`, `parent`, `related`, `blocks`, `blocked_by`,
`supersedes`, or `superseded_by` frontmatter fields, and confirm each
tolerates a quoted-string value. Search seeds:

```bash
grep -rn 'work_item_id\|parent:\|related:\|blocks:\|blocked_by:\|supersedes:\|superseded_by:' skills/ scripts/
```

For each consumer:

- If it does numeric coercion (`$(( ))`, `printf %d`, `int()`) on a
  read frontmatter value: convert to string equality / canonicalised
  comparison via `wip_canonicalise_id`.
- If it strips quotes already (existing parent-normalisation in
  list-work-items and update-work-item): no change needed beyond
  swapping in `wip_canonicalise_id`.

The audit produces a checklist committed to the commit message of the
Phase 3 PR, naming each consumer reviewed and the decision (no change
/ converted).

#### 6. Eval mode-tagging

**File**: every modified `evals.json` plus eval runner.

**Changes**: Each eval gains a `pattern_mode` tag with one of
`default`, `configured`, or `both`. The eval runner (or a wrapper
around it) splits the run into two reports:

- `default`-mode evals must pass at **100%** — these are the
  regression boundary for the "no behavioural change for default
  users" invariant.
- `configured`-mode evals follow the existing ≥95% benchmark
  threshold.
- `both`-mode evals run under both configurations and must pass
  100% / ≥95% in their respective modes.

Pre-existing evals are tagged `default` (they were written against
the default pattern). New evals introduced by Phases 3-4 are tagged
according to their fixture. Untagged evals (e.g. legacy fixtures the
implementer missed) are treated as `default` by the runner, so the
strictest threshold applies until the tag is added.

**Field placement on the eval runner**: the `pattern_mode` tag is a
top-level optional string field on each eval object alongside `id`,
`name`, `prompt`, `expected_output`, and `files`. Adding an unknown
field to the JSON should be tolerated by the upstream skill-creator
eval-runner; if a future runner version becomes strict, the field is
namespaced as additive metadata (this is documented in the eval
runner's commit message so future readers see the rationale).

**Tests for the eval runner / wrapper** (`scripts/test-evals-mode-tagging.sh`,
new):

- Seed a fixture `evals.json` with: one `default`-mode eval scored
  100%, one `default`-mode eval scored 95%, one `configured`-mode
  eval scored 95%, one `configured`-mode eval scored 90%, and one
  untagged eval scored 100%. Run the runner and assert:
  - The `default`-mode 95% case fails the run (default threshold is
    100%; runner exits non-zero).
  - The `configured`-mode 95% case passes (meets ≥95% threshold).
  - The `configured`-mode 90% case fails the run (below threshold).
  - The untagged 100% case passes (treated as `default` and meets
    100%).
- Seed a fixture with one `both`-mode eval; assert it runs under
  both configurations and only passes when both meet their
  respective thresholds.
- Seed a fixture with no evals and a runner invocation; assert
  zero-exit (vacuous pass) rather than crashing.

This locks the threshold-split mechanism — the central
regression-protection promise — in code rather than relying on
prose adherence by the runner author.

#### 7. Tests for resolver/pattern integration

**File**: `skills/work/scripts/test-work-item-scripts.sh`

**Changes**: Add concrete script-level seam tests (not transitive via
evals) covering each frontmatter-consumer category from the audit
in §5. Each test creates a fixture work item with the post-Phase-3
quoted-string frontmatter, exercises the consumer, and asserts the
expected behaviour:

Resolver / read-field consumer:

- Resolver returns a path under `{project}` config; pass that path to
  `work-item-read-field.sh work_item_id`; assert the returned value
  is the full ID string (no shell-coercion error, no leading/trailing
  whitespace, no quotes).
- Same test under default config: `work-item-read-field.sh
  work_item_id` returns `0001` (the unwrapped string content).
- Resolver returns multiple-match exit 2; assert stderr lists
  candidates with source-category tags.

Allocator round-trip:

- Allocator with `--project PROJ` writes the new file; immediately
  read its frontmatter; assert `work_item_id` is the quoted full ID
  (`"PROJ-0001"`).
- Allocator under default config; assert the frontmatter is
  `work_item_id: "0001"` (quoted, per the type contract).

Parent / cross-reference comparison consumers:

- Create work item A with `parent: "PROJ-0042"`; canonicalise via
  `wip_canonicalise_id "PROJ-0042"` and via `wip_canonicalise_id "42"`
  (under `default_project_code: PROJ`); assert both return the same
  canonical string.
- Update-work-item parent-comparison path: write fixture with
  `parent: "0001"` (legacy bare-number form pre-migration), invoke
  the parent-resolution helper, assert it canonicalises to a value
  comparable to the resolver's output for input `0001`.
- list-work-items parent-filtering path: corpus contains
  `PROJ-0001-x.md` with `parent: "PROJ-0042"` and
  `PROJ-0042-y.md`; invoke the `under PROJ-0042` filter; assert
  the returned list contains `PROJ-0001-x.md`. Same filter with
  input `42` (bare number) returns the same list.

Discovery / glob walking consumer:

- list-work-items glob walks a corpus with one malformed-frontmatter
  file in `meta/work/`; assert the walk emits a warning to stderr,
  skips that file, and continues normally.
- list-work-items glob walks a corpus with one file lacking
  `work_item_id` (e.g. a stray draft); assert the walk silently
  skips it.

These tests fail if any consumer regresses to numeric coercion or to
unquoted-only assumptions, providing per-consumer coverage rather
than relying on the audit checklist alone.

#### 8. README Work Item Management section

**File**: `README.md`

**Changes**: The repository README's Work Item Management section
hard-codes the legacy `NNNN-` shape with no signal that the ID
prefix is configurable. Update:

- Work Item Management section (around `README.md:263-292`): add a
  one-paragraph note immediately after the section header explaining
  that the filename prefix is configurable via `work.id_pattern` and
  `work.default_project_code`, with a one-line example showing
  `PROJ-0042-...md` alongside the default `0042-...md`.
- Configuration section (around `README.md:184`): extend the list of
  recognised top-level config sections to include `work` alongside
  `agents`, `review`, `paths`, `templates`, with a one-line summary
  pointing to `skills/config/configure/SKILL.md > Work Items` for
  details.

This makes the feature discoverable from the entry point most users
read first.

#### 9. create-work-item default-pattern golden file

**File**: `skills/work/create-work-item/test-fixtures/default-golden.md`

**Changes**: Capture the exact bytes of a freshly-created
default-pattern work item from running `/create-work-item add foo`
(post-Phase-3) and commit it as a golden file. Test:
`bash skills/work/create-work-item/test-default-golden.sh` runs the
SKILL.md flow against a temp repo and asserts the generated file is
byte-equal to the golden, modulo any explicitly-allowed ISO-timestamp
field (substituted before comparison).

This makes the "filename and H1 are bit-identical for default users"
claim verifiable in CI rather than asserted only in prose.

### Success Criteria

#### Automated Verification

- [x] All work-item script tests pass
- [ ] `default`-mode evals pass at **100%**; `configured`-mode evals
      pass at ≥95% (eval-runner mode-tagging deferred — runner
      contract not yet wired; pre-existing evals untagged are
      treated as `default` per the plan's untagged-default rule)
- [ ] create-work-item default-pattern golden test passes (filename
      and H1 byte-identical; `work_item_id` is the documented quoted
      string) — deferred: golden requires SKILL.md execution loop
      not exercised in shell-level test harness
- [x] Frontmatter-consumer audit: `work-item-read-field.sh` and
      `work-item-update-tags.sh` strip quotes already. Migration
      0001 only renames the field name. `work-item-common.sh:wip_is_work_item_file`
      added for the broadened-glob filter. No breaking consumer.
- [x] Full integration suite passes: `mise run test`

#### Manual Verification

- [x] Resolver returns canonical full IDs under both default and
      `{project}` configurations (verified in test-work-item-scripts.sh)
- [x] Allocator produces `PROJ-0001` under `{project}-{number:04d}`
      with `default_project_code: PROJ` (verified Phase 2 manual)
- [x] `wip_canonicalise_id` agrees on `PROJ-0042` and `42` under
      `default_project_code: PROJ` (verified script-level seam test)

## Phase 4: extract-work-items Interactive Amendment

### Overview

Multi-upstream-project UX for batch extraction: suggest, present, amend,
allocate per-project on confirmation. Sidesteps the
"single `--project` per batch" limitation.

### Changes Required

#### 1. extract-work-items skill

**File**: `skills/work/extract-work-items/SKILL.md`

**Changes**: Replace the single `work-item-next-number.sh --count N`
call (line 357-364) with the suggest-amend-confirm flow described at
research lines 722-752:

1. After parsing source items, suggest IDs using
   `work.default_project_code` (or omit project if unset and the pattern
   requires one — surface as a warning).
2. Present a table: `| Slug | Project | Projected ID |`. The projected
   IDs come from a *display-only* call to `work-item-next-number.sh
   --project <code> --count <distinct-count-per-project>` per distinct
   project — none committed yet.
3. Accept user amendments to the project column. Validate amendments
   against `[A-Za-z][A-Za-z0-9]*`; on failure, re-prompt the offending
   row(s).
4. On confirmation, allocate per distinct project code (one
   `work-item-next-number.sh --project X --count N` per project, in
   original presentation order). Preserve the existing per-project
   ordering semantic at `extract-work-items/SKILL.md:366-369`.
5. Replace the slug-collision check at lines 349-355 with a
   *project-aware* check that respects per-project numbering. For each
   amended row, glob:
   - `<work_dir>/<project>-*-<slug>.md` (for `{project}` patterns: a
     same-slug file under the *same* project code is a real collision)
   - `<work_dir>/[0-9][0-9][0-9][0-9]-<slug>.md` (legacy fallback: a
     same-slug legacy file shadows the new file regardless of project)
   - `<work_dir>/<slug>.md` and any other shape the configured pattern
     can produce when stripped of its prefix (defensive: the
     pattern-shape predicate from `work-item-pattern.sh` enumerates
     these)

   This permits the legitimate `PROJ-0001-add-foo.md` /
   `OTHER-0001-add-foo.md` coexistence (same slug, different projects)
   while still catching real collisions and legacy shadowing.

   Within a single batch, two amendments to the *same* project with
   the same slug are also a collision and the batch aborts before any
   allocation.
6. Preserve partial-allocation failure semantics at lines 376-381:
   the entire batch aborts on any allocator non-zero exit.

**Amendment prompt grammar** (canonical specification — same wording
in every state):

```
Amend any rows? (`<rows> <PROJECT>` to set, `<rows> -` to revert to
default, `?` for help, `q` to cancel, blank to confirm.)
```

Grammar tokens:

- `<rows>`: a single row number (`2`) or comma-separated list
  (`2,3,7`). Whitespace around commas is permitted (`2, 3, 7`) and
  trimmed before parsing.
- `<PROJECT>`: a project code matching rule 5 (`[A-Za-z][A-Za-z0-9]*`).
- `-` (literal hyphen as the second token): reverts the named rows to
  the default project code (or to "no project" if no default is set
  and the pattern lacks `{project}`).
- `?`: prints the grammar reference plus a one-line description of
  each command and re-renders the table; no state change.
- `q`: cancels the entire flow with no files written and no numbers
  allocated.
- Blank input: confirms the current table state and proceeds to
  allocation.

Validation:

- An out-of-range row number (e.g. `99` in a 3-item batch) re-prompts
  with `error: row 99 — out of range (valid: 1-3)` and does not apply
  any other amendments in the same input. The user must re-enter.
- An invalid project code (rule 5) re-prompts with `error: row N —
  project value "<value>" must match [A-Za-z][A-Za-z0-9]*` and
  discards the entire input (no partial application within the
  command).
- Unrecognised commands (e.g. `help`, `cancel`, `2 OTHER OTHER`)
  re-prompt with `error: unrecognised input. Type ? for help.`

Discarded amendments: when validation fails, the offending input is
discarded entirely; the table reverts to its last valid state. This
is documented in the prompt help text and in the eval expected
output.

**Worked example of the amendment flow** (drives the SKILL.md prose):

State 1 — initial projection from a brainstorm with three items
(`add foo`, `fix bar`, `update baz`), pattern `{project}-{number:04d}`,
`default_project_code: PROJ`:

```
| # | Slug       | Project | Projected ID |
| 1 | add-foo    | PROJ    | PROJ-0001    |
| 2 | fix-bar    | PROJ    | PROJ-0002    |
| 3 | update-baz | PROJ    | PROJ-0003    |

Amend any rows? (`<rows> <PROJECT>` to set, `<rows> -` to revert to
default, `?` for help, `q` to cancel, blank to confirm.)
```

State 2 — user types `2 OTHER`. Projected IDs recompute:

```
| # | Slug       | Project | Projected ID |
| 1 | add-foo    | PROJ    | PROJ-0001    |
| 2 | fix-bar    | OTHER   | OTHER-0001   |
| 3 | update-baz | PROJ    | PROJ-0002    |

Amend any rows? (`<rows> <PROJECT>` to set, `<rows> -` to revert to
default, `?` for help, `q` to cancel, blank to confirm.)
```

State 3 — user types `2 -` to revert their previous amendment:

```
| # | Slug       | Project | Projected ID |
| 1 | add-foo    | PROJ    | PROJ-0001    |
| 2 | fix-bar    | PROJ    | PROJ-0002    |
| 3 | update-baz | PROJ    | PROJ-0003    |

Amend any rows? (`<rows> <PROJECT>` to set, `<rows> -` to revert to
default, `?` for help, `q` to cancel, blank to confirm.)
```

State 4 — user types `2,3 OTHER`. Both rows flip; PROJ row 1 keeps
PROJ-0001, OTHER rows take OTHER-0001 and OTHER-0002 in presentation
order:

```
| # | Slug       | Project | Projected ID |
| 1 | add-foo    | PROJ    | PROJ-0001    |
| 2 | fix-bar    | OTHER   | OTHER-0001   |
| 3 | update-baz | OTHER   | OTHER-0002   |

Amend any rows? (`<rows> <PROJECT>` to set, `<rows> -` to revert to
default, `?` for help, `q` to cancel, blank to confirm.)
```

State 5 — user types `2 _bad`. Validation rejects the project code
(rule 5); the table reverts to its prior valid state (State 4) and
the prompt re-displays unchanged:

```
error: row 2 — project value "_bad" must match [A-Za-z][A-Za-z0-9]*
(rejected amendment discarded; previous valid state preserved)

| # | Slug       | Project | Projected ID |
| 1 | add-foo    | PROJ    | PROJ-0001    |
| 2 | fix-bar    | OTHER   | OTHER-0001   |
| 3 | update-baz | OTHER   | OTHER-0002   |

Amend any rows? (`<rows> <PROJECT>` to set, `<rows> -` to revert to
default, `?` for help, `q` to cancel, blank to confirm.)
```

State 6 — user types blank to confirm. Allocation runs and the skill
writes the files:

```
Allocated:
  meta/work/PROJ-0001-add-foo.md
  meta/work/OTHER-0001-fix-bar.md
  meta/work/OTHER-0002-update-baz.md

Done.
```

The display-only allocator calls in States 1-5 do not consume
numbers — only the post-confirmation allocation in State 6 commits.

For batches with many items (10+), the same table renders one row
per item; amendments accept comma-separated row numbers
(`3,7,12 OTHER`) so users don't need to amend each row individually.
The 12-item eval fixture locks the comma-separated parsing in.

> Invoke `skill-creator:skill-creator` for this edit cluster. Update
> `skills/work/extract-work-items/evals/evals.json`.

#### 2. Eval fixtures

**File**: `skills/work/extract-work-items/evals/fixtures/`

**Changes**: Add fixture source documents covering:

- `single-default-project-batch.md` — three brainstorm items, no
  amendments expected.
- `mixed-project-batch.md` — three items, user amends two to different
  projects; allocator called twice in original order.
- `all-overridden-batch.md` — every item amended to a non-default
  project; default project code irrelevant.
- `validation-failure-mid-amendment.md` — amendment introduces an
  invalid project code; skill re-prompts that row only.
- `slug-collision-with-legacy.md` — corpus contains `0042-add-foo.md`;
  brainstorm proposes `add-foo`; collision detected and aborts.
- `same-slug-different-projects.md` — brainstorm proposes two items
  with the same slug `add-foo`; user amends them to different projects
  (`PROJ` and `OTHER`); both rows allocate successfully, producing
  `PROJ-0001-add-foo.md` and `OTHER-0001-add-foo.md`. Asserts that
  per-project numbering allows same-slug coexistence across projects.
- `same-slug-same-project-collision.md` — brainstorm proposes two
  items with the same slug `add-foo`, both under project `PROJ`;
  collision detected and the batch aborts before any allocation.
- `large-batch-comma-separated-amendment.md` — brainstorm with 12
  items; user amends rows `3,7,12 OTHER` in a single command; final
  allocation produces 9 PROJ-prefixed and 3 OTHER-prefixed files in
  presentation order. The eval's `expected_output` asserts:
  - Exactly rows 3, 7, and 12 have `Project: OTHER` in the post-amendment
    table; rows 1, 2, 4, 5, 6, 8, 9, 10, 11 have `Project: PROJ`.
  - The allocator is called exactly twice: once with `--project PROJ
    --count 9`, once with `--project OTHER --count 3`.
  - The 12 written filenames are `PROJ-0001-...md` through
    `PROJ-0009-...md` (assigned to original rows 1, 2, 4, 5, 6, 8, 9,
    10, 11 respectively) and `OTHER-0001-...md`, `OTHER-0002-...md`,
    `OTHER-0003-...md` (assigned to rows 3, 7, 12).
- `revert-amendment-batch.md` — three items; user amends `2 OTHER`
  then `2 -` to revert; final allocation produces all-PROJ filenames.
  Asserts the revert grammar.
- `whitespace-tolerant-amendment.md` — three items; user types
  `2, 3 OTHER` (whitespace around comma); both rows flip. Asserts the
  whitespace-tolerance rule.
- `out-of-range-row-amendment.md` — three items; user types
  `99 OTHER`; expected output: `error: row 99 — out of range
  (valid: 1-3)` plus re-prompt with table unchanged. Asserts the
  range check.
- `help-command.md` — three items; user types `?`; expected output:
  the prompt grammar reference re-displayed alongside the unchanged
  table. Asserts the help command behaviour.

#### 3. New evals

**File**: `skills/work/extract-work-items/evals/evals.json`

**Changes**: Add one eval per fixture above. Each eval's
`expected_output` is a concrete, asserted string fragment (or set of
fragments) describing user-visible behaviour: the post-amendment
table contents row-by-row, the exact allocator-call shape (e.g.
`work-item-next-number.sh --project PROJ --count 9`), the written
filenames in presentation order. Stop-conditions instruct the model
not to call the allocator until confirmation. Each eval is tagged
`pattern_mode: configured` (since they require a `{project}` pattern
configured).

### Success Criteria

#### Automated Verification

- [ ] `extract-work-items` evals pass at ≥95% (skill-creator threshold)
      — deferred: new eval fixtures land alongside the runner work in
      a follower PR. The mixed-project-batch.md fixture is committed
      to anchor the SKILL.md prose; the remaining fixtures and
      expected-output assertions rely on the eval-runner mode-tagging
      contract from Phase 3 §6 which is itself deferred.
- [x] Full integration suite passes: `mise run test`
- [x] Eval-structure validation passes: `bash scripts/test-evals-structure.sh`

#### Manual Verification

- [ ] Run `/extract-work-items` against a brainstorm doc with
      `work.id_pattern: "{project}-{number:04d}"` and
      `work.default_project_code: "PROJ"`. The skill presents a table,
      accepts an amendment of two rows to `OTHER`, and writes
      `PROJ-0001-…`, `OTHER-0001-…`, `PROJ-0002-…` (numbers
      allocated per project, in presentation order) — driven through
      the model running the skill; verified by user during /extract-work-items.
- [ ] Decline at the table prompt → no files written, no numbers
      consumed — same: user-verified at runtime.
- [ ] Introduce a slug collision against a legacy file → batch aborts
      with a clear message identifying the colliding slug — same.

## Phase 5: Migration Framework — Skip-Tracking

### Overview

Strictly additive extension to the migration framework. Independent of
all work-item changes — could ship at any time, but is a hard
prerequisite for Phase 6 (so users can opt out of the rename).

### Changes Required

#### 1. Atomic write helper

**File**: `scripts/atomic-common.sh` (new)

**Changes**: Sourcable bash library with three functions used by the
runner's `--skip`/`--unskip` flags and by migration 0002's per-file
rewrites. Eliminates the inline `cat → tmp → mv` idiom currently
duplicated at `run-migrations.sh:111-117` and
`0001-rename-tickets-to-work.sh:60-66`.

- `atomic_write <target_path>` — reads stdin, writes to a sibling
  tempfile (`mktemp` in the same directory as `target_path` for
  cross-filesystem safety), `mv` to the target. Installs an `EXIT`
  trap to remove the tempfile on failure. Returns 0/non-zero from `mv`.
- `atomic_append_unique <target_path> <line>` — atomically adds
  `<line>` to the file if not already present; idempotent.
- `atomic_remove_line <target_path> <line>` — atomically removes
  every line equal to `<line>` from the file; absence is a no-op.

These avoid `sed -i` entirely, so the implementation is portable
between BSD (macOS) and GNU (Linux) sed semantics. Phase 6's
per-file frontmatter rewrites in migration 0002 also source this
helper for the same reason.

Existing call sites at `run-migrations.sh:111-117` and
`0001-rename-tickets-to-work.sh:60-66` are *not* changed in this
phase — keeping them inline avoids touching unrelated code, and the
shared helper is opt-in for new sites only. (A follow-up consolidation
PR can convert them later if desired.)

Tests at `scripts/test-atomic-common.sh` (new):

- `atomic_write` writes content from stdin to the target.
- `atomic_write` cleans up the tempfile on simulated failure (kill
  before `mv`).
- `atomic_append_unique` is idempotent (calling twice produces one
  line, not two).
- `atomic_remove_line` removes only exact-match lines (substring
  matches are preserved).
- Cross-filesystem-safe: `mktemp` is in the same directory as the
  target (asserted by checking the temp file's parent path).

#### 2. Migration runner

**File**: `skills/config/migrate/scripts/run-migrations.sh`

**Changes**:

- Source `scripts/atomic-common.sh` for `--skip`/`--unskip` writes.
- Read sibling state file `meta/.migrations-skipped` alongside
  `meta/.migrations-applied` in step 3 (line 42-50). Same line-delimited
  format. Same forward-compat (unknown IDs preserved with warning).
- Pending computation (line 76-84) becomes
  `pending = (id NOT IN applied) AND (id NOT IN skipped)`.
- New `--skip <id>` flag: calls `atomic_append_unique
  meta/.migrations-skipped <id>`, exits 0. Idempotent.
- New `--unskip <id>` flag: calls `atomic_remove_line
  meta/.migrations-skipped <id>`; absence is a no-op.
- New summary line format: `applied: A; skipped: <names...>;
  available: V`. Skipped migration *names* are listed (not just the
  count) so they remain visible on every invocation — silent
  permanent skips of important migrations are never invisible. When
  `S=0`, the segment is omitted.
- Unknown-ID warning extended to skipped state file with the same
  shape. The warning text is treated as a **free-form diagnostic**,
  not a stable contract; downstream tooling should not parse it. This
  is documented in the migrate skill prose so any future cosmetic
  edits to the warning don't break consumers.
- **Concurrency**: migration runner operations are not safe under
  concurrent invocation (two `run-migrations.sh` invocations could
  both see the same migration as pending and both attempt to apply
  it). The migrate skill prose adds a one-line note to that effect:
  "Run `/accelerator:migrate` from a single shell at a time; it does
  not acquire a lock." A future hardening pass could add a
  `meta/.migrations.lock` advisory lock; out of scope here.
- **`MIGRATION_RESULT: no_op_pending` contract**: when a migration
  exits 0 *and* its stdout contains a line matching
  `^MIGRATION_RESULT: no_op_pending$`, the runner does NOT record
  the ID in `.migrations-applied`. The migration stays pending for
  re-evaluation on the next run. Any other 0-exit migration is
  recorded as applied (matching today's behaviour). This lets a
  migration self-declare "I had nothing to do but I want to be
  re-checked later" without hand-editing state files. The status
  line is stripped from stdout passed to the user.
- **`ACCELERATOR_MIGRATE_FORCE` interaction**: pinned in this plan as
  `FORCE` bypasses the dirty-tree pre-flight only; skip-tracking is
  enforced regardless. To run a skipped migration, the user must
  `--unskip` first. Documented in the migrate skill prose.

#### 3. Migrate skill prose

**File**: `skills/config/migrate/SKILL.md`

**Changes**: Document:
- the skipped track, `--skip`/`--unskip`, audit-trail explanation,
  updated summary line format (skipped names listed)
- "State file format" section describes both files
- `MIGRATION_RESULT: no_op_pending` runner contract (one paragraph
  describing when a migration self-defers). Include the contract
  clause for migration authors: *migrations emitting
  `MIGRATION_RESULT: no_op_pending` MUST guarantee they performed no
  destructive operations before the line was emitted; migrations
  doing destructive work must either succeed (record applied) or
  fail non-zero*.
- `ACCELERATOR_MIGRATE_FORCE` interaction with skip-tracking
  (one sentence: `FORCE=1` bypasses the dirty-tree pre-flight only;
  skipped migrations remain skipped)
- discoverability hint: when the runner prints the per-migration
  preview line (one per pending migration), append `To skip: bash
  <runner> --skip <ID>` so users see the opt-out at the moment of
  decision
- **Pre-run banner** (load-bearing for the VCS-revert recovery
  contract): immediately before the runner starts applying
  migrations, print the banner:
  ```
  About to apply N migration(s):
    <ID> — <description>
    ...
  Migrations rewrite files and may make repo-wide changes; commit
  your working tree before running so VCS revert is available as
  rollback. The pre-flight will refuse to run on a dirty tree
  unless ACCELERATOR_MIGRATE_FORCE=1 is set.
  ```
  The banner appears in every invocation (not just for migration
  0002), so users facing future destructive migrations see the
  warning consistently. The pre-flight check at
  `run-migrations.sh:12-39` already enforces the clean-tree
  precondition; the banner makes the rationale visible.
- **Concurrency note** (one sentence in the SKILL.md prose): "Run
  `/accelerator:migrate` from a single shell at a time; it does not
  acquire a lock."
- **Migration author guidance** for users writing future migrations:
  reference the migration template at
  `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`,
  the atomic-write helper at `scripts/atomic-common.sh`, and the
  exact-match-rewrite design pattern documented in migration
  `0002-rename-work-items-with-project-prefix.sh`'s header comment.

> Invoke `skill-creator:skill-creator` if `migrate` has an `evals/`
> directory. As of writing it does not — bootstrap one with at least
> three cases covering the new prose (skip a migration, unskip a
> migration, MIGRATION_RESULT contract behaviour) rather than skipping
> eval-driven discipline. The plan's eval-driven principle applies to
> every skill edit cluster.

#### 4. Tests

**File**: `skills/config/migrate/scripts/test-migrate.sh`

**Changes**: Add cases (TDD — written first):

- `--skip 0001-rename-tickets-to-work` adds the ID to
  `.migrations-skipped`; subsequent `run-migrations.sh` reports
  `No pending migrations.` even when `0001` is unapplied.
- `--unskip 0001-rename-tickets-to-work` removes the ID;
  `0001` becomes pending again.
- Skipping an unknown ID writes it to `.migrations-skipped` and warns
  on re-read.
- `applied + skipped + pending` summary format renders correctly,
  including the skipped *names* (not just count).
- `ACCELERATOR_MIGRATE_FORCE` bypasses dirty-tree pre-flight only;
  skipped migrations remain skipped even with FORCE=1.
- Skipping then applying the same ID is impossible (skip prevents
  application; the user must `--unskip` first).
- `MIGRATION_RESULT: no_op_pending` contract: a stub migration that
  exits 0 with this stdout line is NOT recorded as applied; it
  remains in the pending list on the next run. Without the line, the
  same exit-0 migration IS recorded.
- `MIGRATION_RESULT: no_op_pending` from a migration that also did
  real work is treated identically (line presence wins) — but this
  is a misuse; the runner emits a warning to that effect on the
  next run.
- Empty/whitespace-only `.migrations-skipped` is treated as no IDs
  (no error, no warning).
- Both `.migrations-applied` and `.migrations-skipped` containing
  the same ID: `applied` wins for pending computation (the migration
  is treated as done), and a warning surfaces the inconsistency.
- Atomicity: a simulated failure during `--skip` write (e.g. the
  temp file is removed mid-rename) leaves `.migrations-skipped`
  unchanged.
- Pre-run banner: invoking `run-migrations.sh` with at least one
  pending migration prints the `About to apply N migration(s):`
  banner including the commit-before-running warning. Empty pending
  list does not print the banner.
- **Forward/backward state-file compatibility**: seed
  `.migrations-applied` and `.migrations-skipped` with IDs the runner
  doesn't recognise (simulating a plugin downgrade where a future
  migration was applied or skipped). Run the runner; assert the
  unknown-ID warnings appear, the unknown IDs are preserved on any
  rewrite, and the runner does not crash. This locks in the
  forward/backward-compat semantics so a downgrade scenario
  doesn't regress.

### Success Criteria

#### Automated Verification

- [x] All migrate tests pass: `bash skills/config/migrate/scripts/test-migrate.sh`
- [x] Existing `0001-rename-tickets-to-work` migration still applies
      cleanly on a fresh repo
- [x] `meta/.migrations-applied` format is unchanged (byte-equality
      check against a pre-Phase-5 reference state file)
- [x] Full integration suite passes: `mise run test`

#### Manual Verification

- [ ] On a repo with `0001` unapplied, run
      `bash run-migrations.sh --skip 0001-rename-tickets-to-work`
      then `bash run-migrations.sh` — output is "No pending migrations"
- [ ] Run `bash run-migrations.sh --unskip 0001-rename-tickets-to-work`
      then `bash run-migrations.sh` — `0001` runs and applies cleanly
- [ ] Summary line includes the new `skipped: S` segment

## Phase 6: Migration 0002 — Rename Existing Work Items

### Overview

Provide a migration that renames the legacy `NNNN-*.md` corpus to the
configured pattern and rewrites cross-references repo-wide. Depends on
Phase 3 (the resolver and broadened listing must be live so
post-migration state is consistent) and Phase 5 (skip-tracking lets
users opt out).

### Changes Required

#### 1. Migration script

**File**: `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh` (new)

**Changes**: Implements the migration described at research lines
357-409. The migration is structured around a **first-pass rename map
build** so that all rewrites can be driven by exact `old_id → new_id`
substitutions rather than substring matching, which is what makes
re-running and partial-failure recovery safe.

Sources `skills/work/scripts/work-item-common.sh` for compile-format
and ID canonicalisation, and `scripts/atomic-common.sh` for atomic
per-file writes (so no `sed -i` reliance — portable BSD/GNU).

Helper functions in the migration script (each independently
testable):

- `validate_preconditions` — config read + pattern-shape check.
- `build_rename_map` — scans `meta/work/` and computes the old→new
  table. Returns the map without mutating the filesystem.
- `check_collisions` — pre-flight check that target filenames don't
  exist; aborts with a clear listing if any.
- `rename_with_frontmatter` — atomic per-file rename + frontmatter
  `work_item_id` rewrite.
- `rewrite_frontmatter_refs` — exact-match rewrite of ID-bearing
  frontmatter fields.
- `rewrite_markdown_links` — exact-match rewrite of `[text](path)`
  links pointing at renamed files.
- `rewrite_prose_refs` — exact-match rewrite of strict-form prose
  references (`#NNNN` headings, `meta/work/NNNN-*.md` paths within
  fenced code blocks tagged as code).

Behaviour:

1. `validate_preconditions`. Read `work.id_pattern` and
   `work.default_project_code` from config.
2. If pattern lacks `{project}`: no-op, exit 0 **without recording
   the migration as applied**. The migration stays pending so a later
   config change to a `{project}` pattern re-evaluates it. (See
   "Migration applied-marker semantics" subsection below.)
3. If pattern has `{project}` but `default_project_code` is empty:
   exit non-zero with the documented message — `error: migration 0002
   requires a value for work.default_project_code (your pattern
   '<pat>' contains {project}). Set work.default_project_code in your
   config to apply, or run 'bash run-migrations.sh --skip
   0002-rename-work-items-with-project-prefix' to opt out. See
   skills/config/configure/SKILL.md > Work Items for details on
   choosing.`
4. `build_rename_map`. Glob `meta/work/[0-9][0-9][0-9][0-9]-*.md`
   (legacy filenames only). For each, compute the new filename via
   Phase 1's compile-format using the default project code. Returns
   `(old_path, new_path, old_id, new_id)` tuples; nothing is touched
   on disk yet.
5. `check_collisions` over the rename map. Aborts with a clear
   listing if any target already exists. The repo state is unchanged.
6. `rename_with_frontmatter`. For each tuple, rewrite the source
   file's `work_item_id` frontmatter to the full new ID
   (string-quoted) via `atomic_write`, then rename the file via `mv`.
   The order matters: write the new content under the *old* name
   atomically, then rename — this means an interrupt between the
   rewrite and the rename leaves a file with new content under the
   old name (re-run picks up via the legacy glob and rewrites again
   with the same new content; idempotent). An interrupt after the
   rename leaves the new content under the new name (re-run skips
   because the legacy glob no longer matches). No sed-in-place
   reliance.
7. **Exact-match cross-reference rewrites** across all of
   `meta/**/*.md`. Each rewrite class operates from the old→new map
   built in step 4 (the *original* old IDs, not whatever the disk
   currently shows), so the inputs are stable across re-runs.

   - `rewrite_frontmatter_refs` — fields: `work_item_id`, `parent`,
     `related`, `blocks`, `blocked_by`, `supersedes`,
     `superseded_by`. Match shapes (anchored to YAML scalar
     boundaries, not as substrings):
     - Scalar string form: `^(\s*<field>:\s*)"<old_id>"(\s*)$` or
       `^(\s*<field>:\s*)'<old_id>'(\s*)$` → replace the quoted scalar
       with `"<new_id>"`.
     - Scalar bare form: `^(\s*<field>:\s*)<old_id>(\s*)$` → replace
       with `"<new_id>"` (quoted because the new ID contains a
       hyphen).
     - List item form: each list item is matched as a whole quoted or
       bare scalar (`-\s*"<old_id>"`, `-\s*'<old_id>'`, `-\s*<old_id>`)
       and replaced with `- "<new_id>"`. Inline list elements
       (`[\"0001\", 0042]`) are matched element-by-element with the
       same scalar-boundary rules.
     - **Idempotency**: `<old_id>` matches only the *legacy* shape
       (e.g. `0042`); already-rewritten values like `"PROJ-0042"`
       contain no legacy ID at the start of the field value and
       silently skip. A second run is a no-op.

   - `rewrite_markdown_links` — match shape:
     `\[([^\]]*)\]\(([^)]*?/)?<old_id>-([^)]+?\.md)(#[^)]*)?\)` —
     captures the link text, the optional path prefix, the slug, and
     an optional anchor; emits `[text](<prefix><new_id>-<slug>.md
     [#anchor])`. The `<old_id>-` prefix is anchored to either a `/`
     or the start of the URL portion of the link, so already-rewritten
     paths like `(../work/PROJ-0042-foo.md)` do not match
     (`PROJ-0042-` does not contain a leading `/0042-` boundary).

   - `rewrite_prose_refs` — two strict shapes only:
     - Heading-line inline: lines matching `^#+\s` (start with `#`
       followed by whitespace — a markdown heading line) that contain
       `(?<![A-Za-z0-9_])#<old_id>(?![A-Za-z0-9_-])`. The negative
       lookbehind/lookahead asserts the `#NNNN` token is not adjacent
       to other word chars or hyphens, so `#0042-foo` does NOT match
       (the trailing `-` is excluded). Replace the whole token
       `#<old_id>` with `#<new_id>`. A heading with multiple references
       (`# closes #0001 and #0042`) rewrites both. POSIX ERE has no
       lookaround; the implementation either uses bash regex (`=~`
       supports lookaround on macOS bash via PCRE-ish, but not
       portably), or uses an explicit boundary character class
       `(^|[^A-Za-z0-9_])` and `($|[^A-Za-z0-9_-])` with backreference
       in `awk` for portability.
     - Path inside a fenced code block tagged `bash`, `sh`, `yaml`,
       `json`, or `text`: ``` ```bash``` blocks, etc. Bare ``` blocks
       (no language tag) are *not* rewritten because they often hold
       prose examples and historical references that should remain
       intact. Within an eligible block, match
       `\bmeta/work/<old_id>-([^[:space:]\)]+\.md)\b` and replace
       `<old_id>-` with `<new_id>-`.

   Bare 4-digit numbers in prose outside these strict shapes are
   never rewritten.

8. **Idempotency invariant**: every rewrite class above matches only
   *legacy* old-ID shapes; once a value has been rewritten to the new
   form (e.g. `"PROJ-0042"`), no rewrite class will fire on it again.
   Running the migration twice — or recovering from a partial failure
   by re-running — produces the same final state as a single
   successful run. This is asserted by the Phase 6 idempotency and
   partial-failure tests.

#### Migration applied-marker semantics

The migration runner records a migration as applied only when the
script exits 0 *and* the script signals that work was actually
performed. Today the runner unconditionally marks any 0-exit
migration as applied. To support 0002's "stay pending while
unconfigured" behaviour without breaking existing migrations, this
plan extends the runner contract minimally: a migration may exit 0
with a special status code line on stdout — `MIGRATION_RESULT:
no_op_pending` — to indicate it should *not* be recorded. Any
migration that does not emit this line is recorded as applied
(matching today's behaviour). Migration 0001 needs no change.

Migration script header includes a `# DESCRIPTION:` line and a verbose
header comment summarising what is and is not rewritten, plus a note
that the migration is safe to re-run after a VCS revert.

#### 2. Test fixtures

**File**: `skills/config/migrate/scripts/test-fixtures/0002/` (new)

**Changes**: A small `meta/` tree containing:

- `meta/work/0001-add-foo.md` and `meta/work/0042-add-bar.md` with
  `parent: "0001"` cross-references.
- `meta/work/notes.md` (a non-work-item file under `meta/work/`,
  recognisable by frontmatter validation, that must be skipped).
- `meta/plans/2026-04-01-some-plan.md` containing a markdown link
  `[the foo work item](../work/0001-add-foo.md)` and a bare ID
  reference `#0042` in a heading.
- `meta/research/2026-04-02-research.md` containing both a frontmatter
  `related: ["0001", "0042"]` field and a fenced-code-block reference
  to `meta/work/0042-add-bar.md` (in a ```bash``` block — eligible).
- `meta/research/2026-04-03-history.md` — a "negative-case" fixture
  with content that must NOT be rewritten:
  - Plain prose mentioning `0042` (no leading `#`, not in a path).
  - A timestamp-shaped string `2026-04-15`.
  - A non-path numeric reference `port 0042`.
  - A bare ``` (no language tag) fenced block containing the literal
    text `meta/work/0042-add-bar.md` as a historical reference.
  - A `bash` fenced block containing `Run: foo --id 0042` (a non-path
    numeric usage; only path-shaped uses are rewritten).
- `meta/work/0099-bare-frontmatter.md` with `parent: 0042` (bare YAML
  integer, not quoted) and `related: [0001, 0099]` (inline list with
  bare integers) — the bare-form rewrite must produce quoted full
  IDs.

#### 3. Tests

**File**: `skills/config/migrate/scripts/test-migrate.sh`

**Changes**: Add cases (TDD — written first):

- Pattern-lacks-`{project}` → no-op, exits 0 emitting
  `MIGRATION_RESULT: no_op_pending`, **stays unapplied** so a later
  config change to a `{project}` pattern re-runs it.
- Pattern has `{project}` but `default_project_code` empty → exits
  non-zero with the documented message; nothing applied; stays
  pending.
- Single-project rename of `0001-add-foo.md` →
  `PROJ-0001-add-foo.md`; frontmatter `work_item_id` updated to
  `"PROJ-0001"`.
- `parent: "0001"` (quoted scalar) rewrites to `parent: "PROJ-0001"`
  across every file in `meta/`.
- `parent: 0042` (bare scalar) rewrites to `parent: "PROJ-0042"` —
  the rewrite quotes the new value because it contains a hyphen.
- `related: ["0001", "0042"]` rewrites element-by-element to
  `related: ["PROJ-0001", "PROJ-0042"]`.
- `related: [0001, 0099]` (bare inline list) rewrites to
  `related: ["PROJ-0001", "PROJ-0099"]`.
- `[the foo work item](../work/0001-add-foo.md)` rewrites to
  `[the foo work item](../work/PROJ-0001-add-foo.md)`.
- Markdown link with anchor: `[foo](../work/0001-add-foo.md#section)`
  rewrites to `[foo](../work/PROJ-0001-add-foo.md#section)`.
- Fenced-code-block reference `meta/work/0042-add-bar.md` inside a
  ```bash``` (or ```sh```/```yaml```/```json```/```text```) block
  rewrites to `meta/work/PROJ-0042-add-bar.md`.
- **Negative case — bare fenced block**: same path inside a bare ```
  block (no language tag) is *not* rewritten (historical reference
  preserved).
- **Negative case — prose**: bare 4-digit `0042` in plain prose
  (outside the strict forms) is *not* rewritten. Asserts that
  `2026-04-15`, `port 0042`, and `0042 occurrences` survive
  byte-identical.
- **Negative case — non-path code**: `Run: foo --id 0042` inside a
  ```bash``` block is *not* rewritten (not path-shaped).
- Heading-line `#0042` rewrites to `#PROJ-0042`; in-paragraph `#0042`
  is *not* rewritten.
- **Negative case — heading boundary**: heading line containing
  `#0042-foo` (legacy filename-prefix form, not a bare ID reference)
  is *not* rewritten — the trailing `-` falls outside the negative
  lookahead. Asserts the precise word-boundary contract.
- **Negative case — heading suffix**: heading line containing
  `#00420` (a number that happens to start with the legacy ID) is
  *not* rewritten.
- Multi-reference heading: line `# closes #0001 and #0042` rewrites
  both references in a single pass to `# closes #PROJ-0001 and
  #PROJ-0042`.
- Non-work-item file under `meta/work/` (no frontmatter, or no
  `work_item_id`) is skipped — its contents are unchanged.
- **Idempotency**: running the migration twice on the test fixture
  produces the same final state, byte-for-byte (file-tree hash check).
- **Idempotency under partial-failure recovery**: kill the migration
  after the rename phase but before all cross-references are
  rewritten (simulate via an injected sed failure on a specific file
  partway through `rewrite_frontmatter_refs`); re-run from the same
  state; assert the final result is byte-identical to a clean
  single-shot run on the same starting fixture. This is the
  load-bearing test for the exact-match rewrite design.
- **Already-rewritten input is a no-op**: seed the fixture with all
  files already in their post-migration form (PROJ-prefixed names,
  rewritten cross-refs); run the migration; assert zero changes
  (no renames because the legacy glob is empty; no rewrites because
  no legacy IDs match).
- Failure semantics — collision: if a rename collision is detected at
  step 5, the migration exits non-zero before any rewrite; repo state
  unchanged.
- Skip-tracking interaction: `--skip 0002-rename-work-items-with-project-prefix`
  before running the migration suppresses it (Phase 5 dependency).

### Success Criteria

#### Automated Verification

- [ ] All migrate tests pass: `bash skills/config/migrate/scripts/test-migrate.sh`
- [ ] Migration is idempotent on the test fixture (second run is a
      no-op verified by file-tree hash comparison)
- [ ] Migration is idempotent under partial-failure recovery
      (kill mid-cross-ref-rewrite, re-run, byte-identical to single-shot)
- [ ] Default-pattern config produces a no-op AND stays pending
      (the test corpus is unchanged; `.migrations-applied` does not
      gain the migration ID)
- [ ] Full integration suite passes: `mise run test`

#### Manual Verification

- [ ] On a real repo with legacy work items and
      `work.id_pattern: "{project}-{number:04d}"` plus
      `work.default_project_code: "PROJ"`, run
      `/accelerator:migrate` — all `meta/work/NNNN-*.md` files are
      renamed; all `parent`/`related`/etc. fields in `meta/`
      reference the new IDs; markdown links in plans and research are
      rewritten
- [ ] `meta/.migrations-applied` gains the new entry; no other state
      file format changes
- [ ] `--skip 0002-rename-work-items-with-project-prefix` correctly
      suppresses the migration on subsequent runs

## Testing Strategy

### Unit Tests (per phase)

- **Phase 1**: pattern compile/parse/validate via
  `test-work-item-pattern.sh`. Round-trip property at the boundaries.
- **Phase 2**: allocator and resolver via `test-work-item-scripts.sh`.
  Per-project scoping verified via mixed-project corpora; legacy fallback
  verified via simulated structural pattern changes.
- **Phase 5**: migration runner state-file handling via
  `test-migrate.sh`. Skip/unskip idempotency, unknown-ID preservation,
  summary format.
- **Phase 6**: migration `0002` via `test-migrate.sh` plus the new
  `test-fixtures/0002/` tree.

### Integration / Eval Tests

- **Phases 3 and 4**: skill-level evals run through the
  `skill-creator:skill-creator` skill's iterate-and-benchmark loop.
  Threshold: ≥95% per-eval pass rate (matching the existing
  `create-work-item` benchmark norm).
- **Full suite**: `mise run test` after every phase. Phases 1, 2, 5
  must keep all existing tests green with no behavioural change for
  default-pattern users.

### Manual Testing Steps

End-to-end smoke after Phase 4:

1. Fresh repo with `work.id_pattern: "{number:04d}"` (default),
   `/create-work-item add foo` → `0001-add-foo.md`.
2. Switch config to `work.id_pattern: "{project}-{number:04d}"`,
   `work.default_project_code: "PROJ"`.
3. `/create-work-item add bar` → `PROJ-0001-add-bar.md`.
4. `/list-work-items` lists both legacy `0001-add-foo.md` and new
   `PROJ-0001-add-bar.md`.
5. `/update-work-item 1` resolves to the legacy file; warn about
   ambiguity once per-project counters are in play.
6. `/extract-work-items` against a brainstorm doc with three items:
   table appears with `PROJ` defaulted; amend two rows to `OTHER`;
   confirm; verify three files written with correct numbers.

End-to-end migration smoke after Phase 6:

1. Fresh repo with multiple legacy `NNNN-*.md` work items, plans
   referencing them, and research with frontmatter cross-refs.
2. Set config to `work.id_pattern: "{project}-{number:04d}"` and
   `work.default_project_code: "PROJ"`.
3. `/accelerator:migrate` → all files renamed, all references
   rewritten.
4. Re-run `/accelerator:migrate` → "No pending migrations."
5. Try variant: skip `0002` → migration not applied; legacy files
   remain readable via `list-work-items` because of the broadened
   discovery glob from Phase 3.

## Performance Considerations

- The pattern compiler runs once per allocator/resolver invocation —
  negligible cost (~100 lines of bash + awk).
- `list-work-items` discovery glob broadens from
  `[0-9][0-9][0-9][0-9]-*.md` to `*.md`. Frontmatter validation is
  per-file but already happens via the existing `awk` pass. No
  measurable regression expected for repos with hundreds of work items;
  the per-file awk dominates regardless.
- Migration `0002` operates over `meta/**/*.md` with grep + sed — O(N)
  in repo size. Atomic per-file writes plus the exact-match
  rewrite design (every rewrite class matches only the *legacy*
  old-ID shapes, never the new form) mean re-running the migration on
  a partly-rewritten corpus completes the remaining work without
  double-prefixing or other duplication. This invariant is enforced
  by the partial-failure recovery test in Phase 6.

## Migration Notes

- **Phase 5 must merge before Phase 6** so users can opt out before
  running the rename.
- **Phase 3 must merge before Phase 6** so the broadened discovery
  glob is live when legacy files are in the corpus during the migration
  window.
- **Phases 1, 2, 5** are zero-impact on default users and can ship in
  any order relative to each other (subject to Phase 1's compiler
  being live before Phase 2).
- **No rollback for Phase 6** beyond VCS revert. The migration's
  rename + frontmatter + cross-reference rewrites are intentionally
  large. The runner pre-flight refuses to run on a dirty tree
  (existing behaviour at `run-migrations.sh:12-39`), and Phase 5's
  pre-run banner surfaces the commit-before-running expectation
  explicitly to the user. Idempotency under partial-failure recovery
  (Phase 6 test) means a re-run after VCS revert is safe; the
  exact-match rewrite design ensures no double-prefixing.

## References

- Original research: `meta/research/2026-04-28-configurable-work-item-id-pattern.md`.
  This plan is intended to be self-sufficient — the load-bearing
  decisions (validation rules, resolver classification order,
  migration behaviour) are inlined here as the canonical source.
  The research doc remains useful as supplementary reading for
  alternatives considered and prior-art notes.
- Related research: `meta/research/2026-04-08-ticket-management-skills.md`
  (the "Resolved Questions" section flagged this as a future
  enhancement)
- Config-system foundations: `meta/research/2026-03-22-skill-customisation-and-override-patterns.md`
- Migration framework decision: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- Configuration extension points: `meta/decisions/ADR-0017-configuration-extension-points.md`
- Work-item terminology: `meta/decisions/ADR-0022-work-item-terminology.md`
- Existing migration template: `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
- Eval-driven SKILL.md authoring: skill-creator skill at
  `~/.claude/plugins/marketplaces/claude-plugins-official/plugins/skill-creator/skills/skill-creator/SKILL.md`

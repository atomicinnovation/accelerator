---
date: "2026-04-22T00:00:00+01:00"
type: plan
skill: create-plan
ticket: ""
status: draft
---

# Ticket Review (Core) — Phase 4

## Overview

Implement the core ticket review capability for the Accelerator plugin:
three ticket-specific review lenses (`completeness`, `testability`,
`clarity`), a dedicated `ticket-review-output-format` skill, and the
`review-ticket` orchestrator. As a prerequisite, refactor
`scripts/config-read-review.sh` so the lens catalogue is partitioned by
review type — PR, plan, ticket — and add ticket-specific verdict
thresholds. All SKILL.md files are authored TDD-style (evals first via
`/skill-creator:skill-creator`), and all shell-script changes are driven
by tests in the existing `scripts/test-config.sh` harness.

## Current State Analysis

Phases 1–3 of the ticket management initiative are complete. The
following artifacts are in place and covered by the
`skills/tickets/scripts/test-ticket-scripts.sh` regression suite (note:
this suite is **not** currently wired into `tasks/test.py` → `mise run
test`; Phase 4A adds it as a pre-flight step):

- `templates/ticket.md` — 8-field frontmatter plus fixed body sections
  (`Summary`, `Context`, `Requirements`, `Acceptance Criteria`,
  `Open Questions`, `Dependencies`, `Assumptions`, `Technical Notes`,
  `Drafting Notes`, `References`)
- `skills/tickets/scripts/` — `ticket-next-number.sh`,
  `ticket-read-status.sh`, `ticket-read-field.sh`,
  `ticket-update-tags.sh`, `ticket-template-field-hints.sh`
- `skills/tickets/{create-ticket,extract-tickets,list-tickets,update-ticket}/SKILL.md`
- `.claude-plugin/plugin.json` — `./skills/tickets/`,
  `./skills/review/lenses/`, and `./skills/review/output-formats/` all
  registered (subdirectories are auto-scanned)
- `scripts/config-read-path.sh:15` — `review_tickets` key live,
  default `meta/reviews/tickets`
- `skills/config/init/SKILL.md` — provisions `meta/reviews/tickets/`
  during `/accelerator:init`; directory exists with `.gitkeep`

The review infrastructure that Phase 4 builds on:

- `agents/reviewer.md` — generic reviewer agent, takes a lens path and
  an output format path in its task prompt; requires no changes
- `skills/review/lenses/*/SKILL.md` — 13 code-review lenses
  (`architecture`, `code-quality`, `compatibility`, `correctness`,
  `database`, `documentation`, `performance`, `portability`, `safety`,
  `security`, `standards`, `test-coverage`, `usability`)
- `skills/review/output-formats/plan-review-output-format/SKILL.md` —
  JSON schema and finding body conventions; the direct model for the
  new ticket format
- `skills/planning/review-plan/SKILL.md` — multi-lens orchestrator;
  the direct structural model for `review-ticket`
- `scripts/config-read-review.sh` — accepts `pr|plan` today; hardcodes
  a single `BUILTIN_LENSES` array with all 13 lenses; emits the same
  Lens Catalogue regardless of mode
- `skills/config/configure/SKILL.md:158-207` — documents the current
  `review.*` configuration keys

The following do **not** yet exist:

- Any ticket-specific review lens
- A `ticket-review-output-format` skill
- A `review-ticket` orchestrator skill
- `ticket` mode handling in `config-read-review.sh`
- Per-review-type lens partitioning in any script
- `ticket_revise_severity` / `ticket_revise_major_count` config keys

### Key Discoveries

- `config-read-review.sh:34-48` holds the single flat `BUILTIN_LENSES`
  array and emits the full catalogue at lines 350-353 regardless of
  the mode argument. Adding ticket lenses to this array would expose
  them to `/review-plan` and `/review-pr` as well, which is wrong.
- `config-read-review.sh` validates `core_lenses` and `disabled_lenses`
  against the combined set of built-in + custom lens names
  (lines 204-246). After partitioning, validation must not warn when
  a lens named in one of these arrays is valid for some mode but not
  the active mode — those names are intentionally cross-mode.
- `plan-review-output-format/SKILL.md:39-42` lists all 13 valid lens
  identifiers inline. The ticket variant will list only lenses that
  exist in the current phase (`completeness`, `testability`, `clarity`);
  Phase 5 adds `scope` and `dependencies` to the enum when those
  lenses ship. The Lens Catalogue is the canonical source of valid
  lens identifiers.
- `review-plan/SKILL.md:395-446` writes review artifacts inline via
  `Write` using `{plan-stem}-review-{N}.md` naming and a 10-field
  frontmatter. The ticket orchestrator will follow the same shape
  with `{ticket-stem}-review-{N}.md` naming and
  `type: ticket-review`, `skill: review-ticket`.
- Re-review semantics in `review-plan/SKILL.md:491-564`: re-reviews
  append a `## Re-Review (Pass {N}) — {date}` section to the existing
  file and update `verdict`, `review_pass`, `date` frontmatter only.
  Ticket reviews should mirror this exactly.
- Phases 2 and 3 established the skill-creator TDD flow — write eval
  scenarios first, then invoke `/skill-creator:skill-creator` to
  author the SKILL.md against them. This plan follows the same flow
  for every new SKILL.md.
- Lenses have no configuration preamble, no scripts, and no
  severity/confidence taxonomy — they are pure prose following the
  six-section pattern: persona → Core Responsibilities → Key
  Evaluation Questions → Important Guidelines → What NOT to Do →
  Remember paragraph (see `architecture-lens/SKILL.md`).
- `meta/reviews/tickets/` already exists with `.gitkeep`; no
  filesystem provisioning work is required.
- `config-read-review.sh` is exercised only indirectly today. Shell
  tests for the script itself live in `scripts/test-config.sh` (the
  shared integration harness that also covers other `config-*`
  scripts). New coverage for the ticket mode and per-type catalogue
  partitioning belongs in the same file.

## Desired End State

After this plan is complete:

1. `scripts/config-read-review.sh` accepts `pr`, `plan`, and `ticket`
   modes. Each mode emits a Lens Catalogue containing only lenses
   valid for that mode. Mode-specific verdict keys
   (`pr_request_changes_severity`, `plan_revise_severity` +
   `plan_revise_major_count`, `ticket_revise_severity` +
   `ticket_revise_major_count`) are emitted only when their mode is
   active.
2. Custom lenses declared in `.claude/accelerator/lenses/` can specify
   an optional `applies_to: [pr, plan, ticket]` frontmatter field.
   Lenses without this field continue to apply to all modes
   (backwards-compatible).
3. `skills/review/output-formats/ticket-review-output-format/SKILL.md`
   exists, modelled on the plan variant but with ticket-section
   `location` examples and the ticket lens identifier enum.
4. `skills/review/lenses/completeness-lens/SKILL.md`,
   `testability-lens/SKILL.md`, `clarity-lens/SKILL.md` exist and
   follow the existing lens SKILL.md structure. Each is registered in
   `BUILTIN_TICKET_LENSES`.
5. `skills/tickets/review-ticket/SKILL.md` exists and is callable as
   `/review-ticket`. It runs the three ticket lenses in parallel,
   aggregates findings into APPROVE/REVISE/COMMENT, persists results
   to `meta/reviews/tickets/{ticket-stem}-review-{N}.md`, and supports
   appendable re-review passes.
6. `scripts/test-config.sh` covers the new behaviour: per-mode
   catalogue partitioning, `applies_to` filtering, ticket verdict
   keys, validation behaviour.
7. `mise run test` passes with no regressions in pre-existing
   `config-read-review.sh pr|plan` behaviour.
8. `skills/config/configure/SKILL.md` documents the new config keys,
   the per-review-type lens partitioning, and the `applies_to`
   frontmatter field for custom lenses.
9. **No behavioural change** to `/review-plan` or `/review-pr` —
   each continues to receive the same 13 code lenses it sees today.

## What We're NOT Doing

- Not implementing `scope-lens` or `dependencies-lens` — those are
  Phase 5 of the research doc.
- Not implementing `stress-test-ticket` or `refine-ticket` — Phase 6.
- Not mutating the reviewed ticket's `status` field — a REVISE
  verdict does not automatically transition a ticket to `review`
  status; coupling review to status belongs in a separate design.
- Not modifying `review-plan` or `review-pr` behaviour, prompts, or
  allowed-tools. Only `config-read-review.sh` changes, and only in
  ways preserving their current output (same lens catalogue, same
  verdict config).
- Not altering the reviewer agent definition
  (`agents/reviewer.md`). The lens path and output format path in the
  task prompt are the only inputs it needs.
- Not modifying the ticket template or any Phase 1 script.
- Not adding any new `meta/tickets/*.md` file to track this plan —
  the plan itself is the tracking artifact (matching Phase 2 and
  Phase 3 plans which also have empty `ticket:` frontmatter).
- Not migrating the 29 existing `adr-creation-task` tickets — they
  remain untouched. `/review-ticket` operates on any markdown ticket
  in the tickets directory regardless of its type.
- Not adding per-review-type `core_lenses` / `disabled_lenses` config
  keys — the existing cross-mode keys continue to work, with
  informational per-mode filtering. Scoped variants can be added
  later if needed.
- Not handling concurrent `/review-ticket` invocations — the
  read-modify-write pattern for re-review updates is inherited from
  `review-plan` and is last-writer-wins with no locking. Multi-session
  concurrent reviews of the same ticket are out of scope.

## Implementation Approach

Two interleaved tracks:

- **Shell-script TDD** for all changes to
  `scripts/config-read-review.sh`. Tests are added to
  `scripts/test-config.sh` and must fail against the current script,
  then pass after the script is updated. Uses the existing
  `assert_eq` / `assert_contains` / `assert_exit_code` helpers from
  `scripts/test-helpers.sh`.
- **Skill-creator TDD** for all SKILL.md files (output format, three
  lenses, orchestrator). For each, write eval scenarios first
  describing the desired behaviour, then invoke
  `/skill-creator:skill-creator` with those evals as the
  specification. Verify the produced SKILL.md against the evals
  before proceeding.

Sub-phases are ordered by dependency. (This plan uses alpha suffixes
— 4A through 4E — rather than the decimal form used in earlier plans.
The alpha form better conveys the flat dependency graph within
Phase 4, where all sub-phases share a single parent phase.)

1. **4A — Config catalogue refactor**: splits `BUILTIN_LENSES` into
   three per-mode arrays with no functional change to `pr`/`plan`
   modes. Adds `applies_to` filtering for custom lenses. Unblocks
   everything else.
2. **4B — Ticket config + output format**: wires up the `ticket`
   mode in `config-read-review.sh` (with an empty catalogue
   initially) and creates the output format skill. The reviewer
   agent can now be fully configured for ticket reviews even though
   no lenses exist yet.
3. **4C — `completeness-lens`**: first ticket lens; establishes the
   evals pattern for ticket lenses.
4. **4D — `testability-lens` + `clarity-lens`**: remaining two core
   lenses in parallel, reusing the 4C pattern.
5. **4E — `review-ticket` orchestrator**: composes the above;
   includes the only end-to-end manual smoke test.

## Phase 4A: Per-type Lens Catalogue Refactor

### Overview

Split `BUILTIN_LENSES` in `scripts/config-read-review.sh` into three
per-mode arrays and teach the script to emit only the catalogue for
the active mode. Add an optional `applies_to` frontmatter field for
custom lenses. Preserve all current behaviour for `pr` and `plan`
modes. `ticket` mode is accepted as a valid argument but emits an
empty Ticket Lens Catalogue — the three ticket lenses arrive in 4C
and 4D.

### Changes Required

#### 0. Pre-flight: wire ticket-script tests into `mise run test`

- Register `skills/tickets/scripts/test-ticket-scripts.sh` in
  `tasks/test.py` (or equivalently in the `[tasks.test]` section of
  `mise.toml`) so that `mise run test` invokes it alongside the
  existing `scripts/test-config.sh` and ADR-script harnesses.
- Confirm `mise run test` exits 0 with all three harnesses running
  before proceeding to script changes.

#### 1. `scripts/config-read-review.sh`

- Update the header doc-comment (lines 4-9) to list all three modes
  (`pr`, `plan`, `ticket`).
- Replace the single `BUILTIN_LENSES=(...)` block (lines 33-48) with
  two arrays:
  - `BUILTIN_CODE_LENSES` — same 13 names as today's
    `BUILTIN_LENSES` (single source of truth for PR and plan modes)
  - `BUILTIN_TICKET_LENSES` — empty array (populated in 4C/4D)
- Accept `ticket` as a third valid value for `MODE` (line 15); usage
  message updated.
- Introduce a helper `_select_builtin_lenses_for_mode()` that returns
  the appropriate array given `$MODE`. For `pr` and `plan` modes,
  returns `BUILTIN_CODE_LENSES`; for `ticket` mode, returns
  `BUILTIN_TICKET_LENSES`. Use `echo` + `read -ra` idiom for
  Bash 3.2 compatibility (no `nameref`).
- Custom lens discovery (lines 131-202) reads an optional
  `applies_to` field from each custom lens's frontmatter in addition
  to the existing `name` and `auto_detect` fields. Value is a flow
  array of modes: `[pr, plan, ticket]`. Missing field means the
  custom lens applies to all modes.
- Add a `validate_applies_to` helper (paralleling the existing
  `validate_severity` / `validate_positive_int` helpers) that:
  - Accepts YAML flow-array form (`[pr, plan, ticket]`) and bare
    scalars (see below). Block-sequence form is not supported; if
    detected (value starts with `-` on next line), warn and fall
    back to "all modes".
  - Warns on each unknown mode value (e.g., `[prr]`) with
    `Warning: Custom lens 'X' declares applies_to containing
    unrecognised mode 'prr' — ignoring that entry`.
  - Treats a non-array scalar (e.g., `applies_to: pr`) as a
    single-element array `[pr]` with no warning (the existing
    `config_parse_array` handles this).
  - Treats an empty array `applies_to: []` as "applies to no mode"
    (lens is excluded from all catalogues) and emits
    `Warning: Custom lens 'X' has empty applies_to — lens will
    not appear in any mode`.
  - Deduplicates entries silently.
- Extract `_read_frontmatter_scalar <frontmatter> <key>` and
  `_read_frontmatter_array <frontmatter> <key>` helpers to replace
  the existing near-identical awk blocks for `name` (lines 145-163)
  and `auto_detect` (lines 180-196). The new `applies_to` field
  uses `_read_frontmatter_array`.
- A custom lens is included in the mode's catalogue only if
  `applies_to` is absent or contains the active mode.
- Custom-lens name-collision check (lines 172-177): continues to
  validate against the **union** of all built-in arrays
  (`BUILTIN_CODE_LENSES` + `BUILTIN_TICKET_LENSES`), not just the
  active mode's array. This prevents a custom lens from shadowing a
  built-in lens that exists in a different mode.
- Lens Catalogue emission (lines 338-366) iterates
  `_select_builtin_lenses_for_mode` + filtered custom lenses.
- `core_lenses` / `disabled_lenses` validation (lines 204-246):
  validate entries against the **union** of all three built-in
  arrays + all custom lens names (so cross-mode entries don't warn).
  When the user has not set `core_lenses` and the active mode is
  `ticket`, the effective `core_lenses` defaults to all built-in
  ticket lenses (i.e., the full `BUILTIN_TICKET_LENSES` array).
  This avoids an empty effective core set when the user's existing
  `core_lenses` names only PR/plan lenses. For `pr` and `plan`
  modes, the existing default behaviour is unchanged.
  Entries not valid for the active mode are dropped from the mode's
  effective core/disabled set. When entries are dropped, emit an
  informational line in the `## Review Configuration` output block
  listing the filtered entries and the mode(s) they apply to, e.g.,
  `- **Filtered core lenses (not applicable to ticket mode)**:
  architecture, correctness — valid in pr, plan`. This preserves the
  no-false-positive intent while giving users a visible audit trail.
  Unknown entries (not valid in any mode) still produce the existing
  "unrecognised lens" warning.
- Introduce per-mode defaults for `min_lenses`: PR and plan modes
  retain the existing default of `4`; ticket mode defaults to `3`
  (matching the Phase-4-complete catalogue size of 3 built-in
  ticket lenses). This avoids a permanent spurious "Only 3 lenses
  available, but min_lenses is 4" warning on every default ticket
  review. The user-configurable `review.min_lenses` override still
  applies to all modes; the per-mode default is only the fallback
  when the user has not set it.
- Available-lens-count check (lines 248-264): count only lenses
  valid for the active mode. If `available_count < min_lenses` for
  the active mode, warn as today.
- Verdict block (lines 310-336): add a `ticket` branch that emits
  `ticket_revise_severity` and `ticket_revise_major_count` overrides
  when different from defaults. (Default values themselves added in
  4B; 4A only adds the branching structure.)

#### 2. `scripts/test-config.sh`

Add a new test section `=== config-read-review.sh per-type catalogue ===`
covering:

- Invoking with `pr` mode emits a Lens Catalogue containing exactly
  the 13 code lens names and no others.
- Invoking with `plan` mode emits the same 13 names.
- Invoking with `ticket` mode emits zero lens rows (empty catalogue,
  plus the expected warning about `available_count < min_lenses`
  given the per-mode default `min_lenses=3` for ticket mode and 0
  built-in ticket lenses).
- Custom lens in `.claude/accelerator/lenses/example-lens/SKILL.md`
  with `applies_to: [plan]` appears only in plan mode.
- Custom lens without `applies_to` appears in all three modes.
- Custom lens with `applies_to: [ticket, plan]` appears in ticket
  and plan but not pr.
- `core_lenses: [architecture]` in config produces no warning in
  plan mode. (The full cross-mode case with `completeness` is
  covered in 4C once at least one ticket lens exists; use a fixture
  custom lens with `applies_to: [ticket]` to exercise cross-mode
  filter semantics immediately — assert that the "Filtered core
  lenses" informational line appears in the Review Configuration
  block when running in pr mode.)
- Unknown lens identifier `xyz` in `core_lenses` still produces the
  "unrecognised lens" warning as today.
- `applies_to` adversarial inputs on custom lenses:
  - `applies_to: [prr]` — emits "unrecognised mode" warning; lens
    treated as applying to no mode (absent from all catalogues).
  - `applies_to: []` — emits "empty applies_to" warning; lens
    absent from all catalogues.
  - `applies_to: pr` (non-array scalar) — parsed as `[pr]`; lens
    appears in pr mode only.
  - `applies_to: [pr, pr]` (duplicate) — deduplicated; lens appears
    in pr mode once.
- `pr` mode still emits the PR verdict overrides unchanged.
- `plan` mode still emits the plan verdict overrides unchanged.
- Exit code 1 with usage message when given an unknown mode. Assert
  the usage text contains `pr|plan|ticket` to pin the updated string.

Run order: write all new tests first; confirm they fail against the
current `config-read-review.sh`; then apply the script changes;
confirm the whole harness passes.

### Success Criteria

#### Automated Verification:

- [x] `skills/tickets/scripts/test-ticket-scripts.sh` is wired into
      `mise run test` and runs successfully.
- [x] New tests added to `scripts/test-config.sh` fail against the
      current script: `bash scripts/test-config.sh` exits non-zero
      with failures localised to the new section.
- [x] After script changes, all tests pass:
      `bash scripts/test-config.sh` exits 0.
- [x] Regression: `bash skills/tickets/scripts/test-ticket-scripts.sh`
      still exits 0 (should be unaffected).
- [ ] Full test suite passes: `mise run test` exits 0.
- [x] `bash scripts/config-read-review.sh pr` output byte-for-byte
      matches the pre-change output for a clean repo
      (recorded as a golden fixture in the test).
- [x] `bash scripts/config-read-review.sh plan` output byte-for-byte
      matches the pre-change output for a clean repo.
- [x] `bash scripts/config-read-review.sh ticket` exits 0, prints a
      `## Review Configuration` block, and emits a Lens Catalogue
      with zero lens rows.

#### Manual Verification:

- [ ] Inspect `config-read-review.sh` diff: no change to emitted
      output for `pr` or `plan` modes on a clean repo.
- [ ] Place a fixture custom lens at
      `.claude/accelerator/lenses/demo-lens/SKILL.md` with
      `applies_to: [ticket]` and confirm it appears in `ticket` mode
      only.
- [ ] Remove the fixture before committing.

---

## Phase 4B: Ticket Review Config + Output Format

### Overview

Wire up the ticket mode end-to-end: add `ticket_revise_severity` and
`ticket_revise_major_count` config keys to
`scripts/config-read-review.sh`, document them in the configure
skill, and create the `ticket-review-output-format` skill that
reviewer agents read when producing ticket review findings.

### Changes Required

#### 1. `scripts/config-read-review.sh`

- Add defaults: `DEFAULT_TICKET_REVISE_SEVERITY="critical"`,
  `DEFAULT_TICKET_REVISE_MAJOR_COUNT=2`. (The research doc §8.2
  suggests `2` as the ticket-review threshold, reflecting that
  tickets are smaller artifacts than plans.)
- Read `review.ticket_revise_severity` and
  `review.ticket_revise_major_count` via `config-read-value.sh`.
- Validate `ticket_revise_major_count` via the existing
  `validate_positive_int` helper; `ticket_revise_severity` via
  `validate_severity`.
- `ticket` mode emits:
  - `- **ticket revise severity** (`ticket_revise_severity`): {value}`
    (with default annotation if overridden)
  - `- **ticket revise major count** (`ticket_revise_major_count`): {value}`
    (with default annotation if overridden)
  - Plus the existing `min lenses` / `max lenses` labels
  - Conditional `Verdict` override block when either ticket verdict
    key differs from its default, mirroring the plan verdict block
    at lines 320-335.

#### 2. `scripts/test-config.sh`

Extend the ticket-mode test section with:

- Default invocation emits `ticket revise severity: critical` and
  `ticket revise major count: 2` without default annotations.
- Overriding `review.ticket_revise_severity: major` emits the value
  with a `(default: critical)` annotation.
- Overriding `review.ticket_revise_major_count: 5` emits the value
  with `(default: 2)`.
- Invalid `ticket_revise_major_count: 0` triggers the positive-int
  warning and falls back to `2`.
- Invalid `ticket_revise_severity: sometimes` triggers the severity
  warning and falls back to `critical`.
- `ticket_revise_severity: none` produces the "severity-based REVISE
  disabled" verdict line (same phrasing as plan mode at line 329).

#### 3. `skills/review/output-formats/ticket-review-output-format/SKILL.md`

New skill file modelled on `plan-review-output-format/SKILL.md`:

- Frontmatter: `name: ticket-review-output-format`;
  `description: Output format specification for ticket review agents...
  Used by review orchestrators — not invoked directly.`;
  `user-invocable: false`; `disable-model-invocation: true`.
- H1: `# Ticket Review Output Format`.
- `## JSON Schema` — identical shape to the plan variant (top-level
  `lens`, `summary`, `strengths`, `findings[]` with `severity`,
  `confidence`, `lens`, `location`, `title`, `body`). Add an
  optional `synthetic: boolean` field to the finding schema
  (default `false`, omitted by agents; set to `true` only by the
  orchestrator's malformed-agent fallback). This is
  orchestrator-internal metadata — agents never emit it.
- `## Field Reference`:
  - Lens identifier enum lists: `completeness`, `testability`,
    `clarity`. Only lenses that exist in the current phase are
    listed; Phase 5 will add `scope` and `dependencies` to the enum
    when those lenses ship. The canonical source of valid lens
    identifiers is the Lens Catalogue emitted by
    `config-read-review.sh ticket`, not this enum.
  - `location` examples: `"Summary"`, `"Acceptance Criteria"`,
    `"Dependencies"`, `"Open Questions"`, `"Context"`,
    `"Requirements"`, `"Frontmatter: type"` — anchored to ticket
    sections documented in `templates/ticket.md`.
- `## Severity Emoji Prefixes` — identical to plan variant (🔴 / 🟡
  / 🔵).
- `## Finding Body Format` — identical template. Example uses the
  Completeness lens on an "Acceptance Criteria" location:

  ```
  🔴 **Completeness**

  The ticket has no Acceptance Criteria section, so there is no
  definition of what "done" means.

  **Impact**: Implementers cannot verify when the work is complete,
  risking scope drift or premature closure.

  **Suggestion**: Add an Acceptance Criteria section with at least
  two specific, testable bullets.
  ```

- Closing reminder paragraph matches the plan variant line-by-line
  (Output only the JSON block...).

#### 4. `skills/config/configure/SKILL.md`

Update the review section (lines 158-207):

- Header changed from "Customise review behaviour for
  `/accelerator:review-pr` and `/accelerator:review-plan`" to
  include `review-ticket`.
- Add a "Ticket review only (`review-ticket`)" sub-table:

  | Key                         | Default    | Description                                              |
  |-----------------------------|------------|----------------------------------------------------------|
  | `ticket_revise_severity`    | `critical` | Min severity for REVISE (`critical`, `major`, or `none`) |
  | `ticket_revise_major_count` | `2`        | Major findings count to trigger REVISE                   |

- Under the ticket review table, add a brief note explaining why
  `ticket_revise_major_count` defaults to `2` (not `3` like plans):
  "Tickets are smaller artifacts than plans, so a lower threshold
  produces equivalent signal density."
- Add a new `#### Per-Review-Type Lenses` subsection explaining that
  built-in lenses are partitioned by review type (PR, plan, ticket)
  and that cross-mode `core_lenses` / `disabled_lenses` entries are
  filtered to the active mode's catalogue with a visible
  informational note in the Review Configuration block. Document
  that unknown entries (not valid in any mode) still produce the
  existing "unrecognised lens" warning.
- Extend the Custom Lenses subsection (lines 209-238) to document the
  optional `applies_to: [pr, plan, ticket]` frontmatter field,
  including the backwards-compatible "absent means all modes"
  semantics. Keep the existing three-field minimal template
  unchanged; add a separate "Optional fields" snippet showing
  `applies_to` with two side-by-side examples: (a) omitted (explicit
  comment: "applies to all modes — pr, plan, and ticket") and (b)
  `applies_to: [ticket]` (comment: "ticket reviews only"). Note that
  `applies_to` is not used by built-in lenses (which are partitioned
  via script arrays), and list the accepted mode values (`pr`,
  `plan`, `ticket`).
- Update secondary references in `configure/SKILL.md`: the `reviewer`
  agent row description to "Reviews plans, PRs, and tickets"; add
  one `review-ticket/` example to the Per-Skill Customisation
  directory listing (lines 259-263).
- Example configuration (lines 189-204) gains
  `ticket_revise_severity: major` and
  `ticket_revise_major_count: 3`.

### Success Criteria

#### Automated Verification:

- [x] New `test-config.sh` assertions pass:
      `bash scripts/test-config.sh` exits 0.
- [x] `bash scripts/config-read-review.sh ticket` emits labelled
      ticket verdict lines with correct defaults.
- [x] `ticket-review-output-format/SKILL.md` exists, has valid
      YAML frontmatter, and `grep -c '^##' skills/review/output-formats/ticket-review-output-format/SKILL.md`
      returns at least 4 (JSON Schema, Field Reference, Severity
      Emoji Prefixes, Finding Body Format).
- [ ] Full test suite still green: `mise run test` exits 0.

#### Manual Verification:

- [ ] Read the output format side-by-side with the plan variant;
      confirm structural parity and that every example references
      ticket sections (not plan phases).
- [ ] Read the configure SKILL.md diff; confirm the new ticket
      review table, per-type lens note, and `applies_to` docs are
      clear to someone reading the plugin docs for the first time.
- [ ] Run `/accelerator:configure` interactively and confirm the
      new keys appear in the review-section help output.

---

### Lens Scope Boundaries

To prevent overlap between the three ticket lenses, each failure mode
maps to exactly one lens:

| Failure mode | Lens |
|---|---|
| Section missing or empty | completeness |
| Frontmatter field missing | completeness |
| Type-inappropriate content (e.g., no repro steps for bugs) | completeness |
| Criterion is unmeasurable / subjective / unbounded | testability |
| No verification strategy admitted by the spec | testability |
| Missing input/output specification | testability |
| Ambiguous pronoun / unclear referent | clarity |
| Internal contradiction between sections | clarity |
| Undefined acronym or jargon | clarity |
| Passive voice obscuring the actor | clarity |

The `What NOT to Do` section of each lens names the other four
ticket lenses (including Phase 5's `scope` and `dependencies`) as
off-scope and includes "Do not read source code or run codebase
exploration agents."

---

## Phase 4C: Completeness Lens

### Overview

Author the first ticket-specific lens and register it in
`BUILTIN_TICKET_LENSES`. The completeness lens evaluates whether a
ticket has all expected sections populated, whether acceptance
criteria are present, and whether dependencies / context /
assumptions are identified where relevant for the ticket type.

### Changes Required

#### 1. `scripts/config-read-review.sh`

- Add `completeness` to `BUILTIN_TICKET_LENSES`.
- No other script changes.

#### 2. `scripts/test-config.sh`

- Extend the ticket-mode test section: assert the Lens Catalogue
  now contains exactly one row (`completeness`) in `ticket` mode.
- Assert `bash scripts/config-read-review.sh pr` does **not** emit
  `completeness` (cross-mode isolation).
- Assert `bash scripts/config-read-review.sh plan` does **not** emit
  `completeness`.
- Cover the cross-mode core lens case deferred from 4A: setting
  `core_lenses: [architecture, completeness]` produces no warning
  in either `pr`, `plan`, or `ticket` mode — architecture is valid
  in pr/plan, completeness is valid in ticket, and the silent filter
  drops the non-applicable entry per mode.

#### 3. `skills/review/lenses/completeness-lens/SKILL.md`

Author via `/skill-creator:skill-creator` after writing evals.

**Eval scenarios** (written first, then committed alongside the
skill — see Testing Strategy):

- Ticket missing `Acceptance Criteria` section → expect a finding
  at `major` or higher with `location: "Acceptance Criteria"`.
- Ticket with `Acceptance Criteria` present but empty list → expect
  a finding at `major` or higher.
- Ticket with `status: draft`, `type: story`, empty `Dependencies`
  section → expect at most a `suggestion` finding (optional for
  stories that genuinely have no dependencies; wording should not be
  alarmist).
- Ticket with well-populated Summary, Context, Requirements,
  Acceptance Criteria, Dependencies, and Assumptions → expect no
  findings in the `critical`/`major` tier; `strengths` list
  acknowledges structural completeness.
- Bug ticket with no reproduction steps in Requirements → expect a
  finding at `major` or higher noting bugs need reproduction steps.
- Spike ticket with no exit criteria / time-box → expect a finding
  at `major` or higher.
- Epic ticket with no list of child stories in Requirements →
  expect a finding at `minor` or higher.
- Ticket whose `type` frontmatter is absent → expect a finding at
  `major` or higher with `location: "Frontmatter: type"`.

**SKILL.md structure** (matches existing lens SKILL.md pattern):

- Frontmatter: `name: completeness`; `description: Ticket review
  lens for evaluating structural and informational completeness...
  Used by review orchestrators — not invoked directly.`;
  `user-invocable: false`; `disable-model-invocation: true`.
  Intentionally no `applies_to` field — built-in ticket lenses are
  partitioned via `BUILTIN_TICKET_LENSES` in the script, not via
  frontmatter. `applies_to` is for custom lenses only.
- Persona sentence — "Review as a ticket completeness specialist..."
- `## Core Responsibilities` — 3-4 numbered items covering section
  presence, content density, type-appropriate content, frontmatter
  completeness.
- `## Key Evaluation Questions` — grouped by applicability
  (`**Structural completeness** (always applicable):`,
  `**Type-specific content** (based on ticket type):`,
  `**Frontmatter integrity** (always applicable):`).
- `## Important Guidelines` — includes the canonical "Rate
  confidence" line and a "Do not read source code or run codebase
  exploration agents — ticket content is the sole artefact under
  review" line that reminds the lens to stay within ticket content.
- `## What NOT to Do` — first bullet names the other ticket lenses
  (testability, clarity, scope, dependencies) as off-scope. Do not
  reference code-review lenses here.
- Closing "Remember:" paragraph.

### Success Criteria

#### Automated Verification:

- [x] `scripts/test-config.sh` catalogue assertions pass:
      `bash scripts/test-config.sh` exits 0.
- [x] `completeness-lens/SKILL.md` exists and has valid YAML
      frontmatter (checked via a simple awk frontmatter delimit
      count in the test).
- [ ] `mise run test` passes.
- [x] Regression: `bash scripts/config-read-review.sh plan` output
      unchanged from Phase 4B (verified against the golden fixture
      from 4A).

#### Manual Verification:

- [ ] `/skill-creator:skill-creator` evals all pass when run
      against the authored SKILL.md.
- [ ] Diff the finished SKILL.md against `architecture-lens/SKILL.md`
      and `documentation-lens/SKILL.md`: section headings and
      ordering match.
- [ ] Verify no prose in `completeness-lens/SKILL.md` instructs the
      lens to read source code or run scripts.

---

## Phase 4D: Testability and Clarity Lenses

### Overview

Author the remaining two core ticket lenses using the pattern
established in 4C. Both are ticket-content-only; neither spawns
codebase or documents agents.

### Changes Required

#### 1. `scripts/config-read-review.sh`

- Add `testability` and `clarity` to `BUILTIN_TICKET_LENSES`.

#### 2. `scripts/test-config.sh`

- Update the ticket-mode catalogue assertion: expect exactly three
  rows (`completeness`, `testability`, `clarity`) as a set
  (order-independent comparison via `sort`).
- Assert none of the three appear in `pr` or `plan` mode output.

#### 3. `skills/review/lenses/testability-lens/SKILL.md`

Author via `/skill-creator:skill-creator`.

**Eval scenarios**:

- Acceptance criterion "The API should be fast" → expect a finding
  at `major` or higher (unmeasurable / subjective).
- Criterion "Latency p95 < 200ms for /search endpoint under 100 rps
  load" → expect no finding; `strengths` note specificity.
- Story ticket without any Given/When/Then framing and vague prose
  criteria → expect a finding at `minor` or higher suggesting
  Given/When/Then or a testable rephrasing.
- Criterion "All edge cases are handled" → expect a finding at
  `major` or higher (unbounded scope, cannot be verified).
- Bug ticket with expected vs actual behaviour that doesn't specify
  the input that triggers the bug → expect a finding at `major` or
  higher.
- Spike ticket whose exit criteria are enumerable artefacts
  ("produce a decision memo and three timing benchmarks") → expect
  no finding.
- Criterion listing a specific data fixture and a specific output
  shape → expect no finding.

**SKILL.md structure**: same six-section pattern as 4C. Persona
framed around a test engineer evaluating whether the specification
admits a verification strategy. Core Responsibilities cover: each
criterion is specific and measurable; criteria collectively cover
the Summary's intent; type-appropriate verification framing
(Given/When/Then for stories; reproduction + expected for bugs;
enumerable exit criteria for spikes). The `What NOT to Do` section
names the other four ticket lenses.

#### 4. `skills/review/lenses/clarity-lens/SKILL.md`

Author via `/skill-creator:skill-creator`.

**Eval scenarios**:

- Ticket using "the system" and "it" with no clear referent → expect
  a finding at `major` or higher about pronoun resolution.
- Ticket whose Summary states one scope and Requirements state a
  different scope → expect a finding at `major` or higher about
  internal contradiction.
- Ticket with acronyms used without definition
  (`DORA`, `RBAC` used in passing) → expect a finding at `minor`
  or higher.
- Ticket written entirely in passive voice with no identified actor
  → expect a finding at `minor` or higher.
- Ticket whose Context clearly states the forces, whose Summary is
  an unambiguous noun phrase, and whose Requirements use concrete
  verbs → expect no finding; `strengths` cite unambiguous language.
- Ticket using domain jargon ("reify the discriminator", "demarshal
  the envelope") with no link to a glossary or prior document →
  expect a `minor` finding.

**SKILL.md structure**: same six-section pattern. Persona framed
around an unambiguous-communication specialist. Core Responsibilities
cover: unambiguous referents; internal consistency;
jargon/acronym handling; actor and outcome clarity. `What NOT to Do`
names the other four ticket lenses. Explicitly does not evaluate
grammar beyond what affects meaning.

### Success Criteria

#### Automated Verification:

- [x] `scripts/test-config.sh` ticket catalogue assertion expects
      three lens rows: `bash scripts/test-config.sh` exits 0.
- [x] Both `testability-lens/SKILL.md` and `clarity-lens/SKILL.md`
      exist with valid frontmatter.
- [ ] `mise run test` passes.

#### Manual Verification:

- [ ] `/skill-creator:skill-creator` eval runs pass for each lens.
- [ ] Compare the three ticket lenses side-by-side: each "What NOT
      to Do" section references the other four ticket lens names
      consistently (no typos, no stray mentions of code-review
      lenses).
- [ ] Confirm no ticket lens spawns codebase agents in its prose.

---

## Phase 4E: review-ticket Orchestrator

### Overview

Compose the preceding artifacts into the user-facing `review-ticket`
skill. Models its structure on `review-plan/SKILL.md` but invokes
`config-read-review.sh ticket`, uses the ticket output format, and
persists reviews to `meta/reviews/tickets/`. This is the first (and
only) sub-phase with an end-to-end manual smoke test.

### Changes Required

#### 1. `skills/tickets/review-ticket/SKILL.md`

Author via `/skill-creator:skill-creator` after evals.

**Eval scenarios**:

- Invoked with no arguments: skill prompts the user with guidance,
  shows example invocations (matching `/review-plan`'s no-args
  style), and additionally mentions `/list-tickets` as a way to
  find ticket paths. Also accepts ticket-number shorthand (e.g.,
  `/review-ticket 42` → resolves via glob
  `{tickets_dir}/0042-*.md`), matching `/update-ticket`'s
  ergonomics.
- Invoked with a path to a nonexistent ticket: reports the error
  using `/update-ticket`'s phrasing (`"No ticket at <path>."` for
  paths; `"No ticket numbered NNNN found in {tickets_dir}."` for
  numbers) and offers to run `/list-tickets`.
- Invoked with a valid ticket path and no prior reviews: runs the
  three ticket lenses in parallel (spawns three `reviewer` agents,
  each given the lens path from the Lens Catalogue and the ticket
  output format path), aggregates, presents verdict, then writes
  to `meta/reviews/tickets/{ticket-stem}-review-1.md`.
- Invoked with a ticket that has a prior `-review-1.md`: glob finds
  it, informs the user, and on user confirmation runs re-review
  against only the lenses with prior findings, then appends a
  `## Re-Review (Pass 2) — {date}` section to the existing file
  and updates the three frontmatter fields (`verdict`, `review_pass`,
  `date`).
- Verdict aggregation: any finding at or above
  `ticket_revise_severity` → `REVISE`; `ticket_revise_major_count`+
  major findings → `REVISE`; otherwise `COMMENT` if any findings
  else `APPROVE`. Matches the plan aggregation logic at
  `review-plan/SKILL.md:311-331` with ticket-specific thresholds.
- Configuration override: user sets
  `review.ticket_revise_major_count: 1` and a review with a single
  `major` finding verdicts `REVISE` (not `COMMENT`).
- Verdict threshold boundary: default config
  (`ticket_revise_severity: critical`, `ticket_revise_major_count: 2`)
  with exactly 2 major findings → `REVISE`; with 1 major finding →
  `COMMENT`.
- `ticket_revise_severity: none` with 3 major findings and
  `ticket_revise_major_count: 2` → `REVISE` (major-count rule
  applies independently even when severity-based rule is disabled).
- `ticket_revise_severity: major` with 1 major finding → `REVISE`
  (severity threshold met).
- Re-review with all prior findings resolved and no new findings:
  new verdict `APPROVE`, appended section lists each prior finding
  under "Resolved", frontmatter updated.
- Agent returning malformed (non-JSON) output: orchestrator
  fallback — treat the raw output as a single `suggestion`-severity
  finding attributed to that agent's lens with a `synthetic: true`
  marker, warn the user, and include remediation guidance ("try
  re-running with a narrower lens selection or file a bug with the
  raw output above"). The `suggestion` severity prevents a single
  flaky agent from deterministically forcing a REVISE verdict when
  `ticket_revise_severity` is set to `major`. This differs from
  `review-plan/SKILL.md:261-277` (which uses `major`) — document
  the rationale in the orchestrator's Important Guidelines.

**SKILL.md structure** (modelled on `review-plan/SKILL.md`):

- Frontmatter: `name: review-ticket`; `argument-hint: "[path to
  ticket file]"`; `disable-model-invocation: true`.
  No `user-invocable` key — this is a user-facing slash command,
  matching `review-plan/SKILL.md`. (The three lenses and the
  output format set `user-invocable: false`; the orchestrator
  does not.)
  `allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*),
  Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/ticket-read-*)`.
  The ticket-scripts entry is scoped to read-only scripts
  (`ticket-read-field.sh`, `ticket-read-status.sh`) so the
  orchestrator can surface the ticket's `title` or `type` in
  user-facing prompts without granting access to mutating scripts
  like `ticket-update-tags.sh`.
- Preamble:
  ```
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh review-ticket`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
  ```
  Followed by the agent names fallback block and:
  ```
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-review.sh ticket`

  **Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`
  **Ticket reviews directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_tickets meta/reviews/tickets`
  ```
  Trailing: `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh review-ticket``
  (full path, matching the convention in `review-plan/SKILL.md`).
- `## Available Review Lenses` — 3-row table listing completeness,
  testability, clarity. (Structurally parallel to `review-plan`'s
  13-row table.)
- `## Process Steps`:
  - **Step 1: Read and Understand the Ticket** — read the ticket
    fully; parse frontmatter to note `type` and `status`; glob
    `{ticket reviews directory}/{ticket-stem}-review-*.md` for
    prior reviews.
  - **Step 2: Select Review Lenses** — default: all three lenses;
    user may narrow via focus args ("focus on testability").
    Respect `core_lenses` / `disabled_lenses` from config (which
    the per-mode filter has already scoped to ticket lenses).
  - **Step 3: Spawn Review Agents** — parallel Task tool
    invocations with `subagent_type` resolved via
    `config-read-agent-name.sh reviewer`. The agent prompt
    template matches `review-plan/SKILL.md:224-254` adapted for
    ticket reviews: "You are reviewing a ticket through the
    [lens] lens. The ticket is at [path]. Read it fully. Also
    read any referenced source documents in the References
    section." Output format path:
    `${CLAUDE_PLUGIN_ROOT}/skills/review/output-formats/ticket-review-output-format/SKILL.md`.
    Agent analysis strategy explicitly includes "Do not evaluate
    the codebase — ticket content is the sole artefact under
    review."
  - **Step 4: Aggregate and Curate Findings** — same
    dedup / severity-prioritisation / cross-cutting-theme logic
    as `review-plan` Step 4; verdict logic uses
    `ticket_revise_severity` + `ticket_revise_major_count`.
  - **Step 5: Present the Review** — summary, verdict, findings
    tables, strengths.
  - **Step 6: Collaborative Ticket Iteration** — offer to make
    edits based on findings, matching `review-plan` Step 6 but
    editing ticket body sections rather than plan phases.
  - **Step 7: Offer Re-Review** — identical mechanics to
    `review-plan` Step 7; appends to the existing review file,
    updates only `verdict` / `review_pass` / `date` frontmatter
    fields. If the existing review file's frontmatter is malformed
    (cannot be parsed), warn the user and write a fresh
    `-review-{N+1}.md` file instead of appending in place —
    symmetrical with the initial-review fallback for malformed
    prior reviews.
- `## Important Guidelines` — includes "Do not modify the ticket's
  `status` field" and "Do not run codebase exploration agents —
  the reviewer agents stay inside the ticket".
- `## What NOT to Do` — mirrors `review-plan` and adds the two
  items above.
- `## Relationship to Other Commands` — references
  `/create-ticket` or `/extract-tickets` → `/review-ticket` →
  `/update-ticket` (status transition) → `/create-plan` (from
  ticket), matching the lifecycle-diagram format used by
  `review-plan`. Also lists `/list-tickets` as the discovery entry
  point for finding review targets.

Review artifact persisted by the orchestrator at
`{review tickets directory}/{ticket-stem}-review-{N}.md`:

```yaml
---
date: "{ISO timestamp}"
type: ticket-review
skill: review-ticket
target: "{tickets directory}/{ticket-stem}.md"
ticket_id: "{4-digit number, e.g. 0026}"
review_number: {N}
verdict: {APPROVE | REVISE | COMMENT}
lenses: [{list of lenses used}]
review_pass: 1
status: complete
---
```

The `ticket_id` field stores the ticket's stable 4-digit identifier
(extracted from the filename via `ticket-read-field.sh number`),
providing resilience against ticket renames. `target` remains as the
path used at review time.

Body follows the plan-review body structure: `## Ticket Review:
[Ticket Title]`, verdict + combined assessment, optional
Cross-Cutting Themes and Tradeoff Analysis, `### Findings`
subdivided by severity, `### Strengths`, `### Recommended
Changes`, footer, `## Per-Lens Results`. Re-reviews append
`## Re-Review (Pass {N}) — {date}` sections.

#### 2. `README.md` and `CHANGELOG.md`

- Update `README.md` "Ticket Management" section (lines 244-271):
  add a row for `/review-ticket` and update the ticket-flow diagram.
- Update `README.md` "Review System" section (lines 311-334): add a
  "Ticket Review" entry describing the three ticket lenses and the
  ticket-review-output-format.
- Update `CHANGELOG.md` "Unreleased" section: add "Added" bullets
  for `/review-ticket`, the three ticket lenses
  (`completeness-lens`, `testability-lens`, `clarity-lens`),
  `ticket-review-output-format`, the `ticket_revise_severity` and
  `ticket_revise_major_count` config keys, the `applies_to`
  frontmatter field for custom lenses, and per-review-type lens
  partitioning in `config-read-review.sh`.

#### 3. Manual smoke test

Not a code change — documented here so the implementer runs it:

- Pick a populated real ticket (e.g.,
  `meta/tickets/0026-init-skill-for-repo-bootstrap.md`).
- Run `/review-ticket meta/tickets/0026-init-skill-for-repo-bootstrap.md`.
- Confirm three agents are spawned in parallel.
- Confirm the resulting file is written at
  `meta/reviews/tickets/0026-init-skill-for-repo-bootstrap-review-1.md`
  with the documented frontmatter schema.
- Re-run the same command; confirm it detects the prior review and
  offers re-review. Run it; confirm Pass 2 is appended to the same
  file and frontmatter's `review_pass` is now `2`.
- Confirm the ticket itself is unmodified (`jj diff meta/tickets/0026...`).
- Run a second smoke test against a legacy `adr-creation-task` ticket
  (e.g., `meta/tickets/0001-*.md`) — expect the completeness lens to
  flag it heavily (predates the richer template). Confirm the review
  completes without error and the verdict reflects the findings.

### Success Criteria

#### Automated Verification:

- [ ] `skills/tickets/review-ticket/SKILL.md` exists with valid
      frontmatter.
- [ ] `/skill-creator:skill-creator` evals pass for the
      orchestrator.
- [ ] `mise run test` still passes (no test should regress; there
      are no new shell tests in 4E — the orchestrator is a SKILL.md,
      exercised via skill-creator evals).
- [ ] The plugin registers the skill: invoking `/review-ticket` in
      a fresh Claude Code session resolves to the new SKILL.md
      (verified by re-reading the skills table in the session's
      system reminders; `./skills/tickets/` is already in
      `plugin.json`, so no registration change needed).

#### Manual Verification:

- [ ] End-to-end smoke test above completes successfully.
- [ ] Generated review file is well-formatted markdown with
      correct frontmatter and a populated Per-Lens Results section
      for all three lenses.
- [ ] Re-review appends correctly and does not duplicate prior
      findings.
- [ ] Setting `review.ticket_revise_major_count: 1` and re-running
      forces a REVISE verdict where a single major finding exists.
- [ ] Target ticket's `status` is unchanged after review (the
      orchestrator must not mutate the source ticket).
- [ ] `/review-ticket` with no arguments produces the expected
      help text, not an error.

---

## Testing Strategy

### Shell-script tests

All additions go into `scripts/test-config.sh` under a new section
`=== config-read-review.sh ticket mode ===` (introduced in 4A and
extended in each subsequent phase). The harness uses the existing
`assert_eq`, `assert_contains`, `assert_exit_code` helpers sourced
from `scripts/test-helpers.sh`. Tests are run via
`bash scripts/test-config.sh` during phase execution and via
`mise run test` at each phase boundary.

Golden-fixture invariants (recorded during 4A, checked in every
subsequent phase):

- `bash scripts/config-read-review.sh pr` output on a clean repo
  is unchanged across 4A, 4B, 4C, 4D, 4E.
- `bash scripts/config-read-review.sh plan` output on a clean repo
  is unchanged across 4A, 4B, 4C, 4D, 4E.

Golden-fixture normalisation strategy: the script emits absolute
lens paths that vary by checkout location. Tests should record the
golden fixture per-run from a reference invocation immediately
before the refactor, then diff against the post-refactor output.
Both outputs use the same checkout, so paths match. This avoids
committing machine-specific paths into fixtures. All subsequent
phases reuse this same approach.

### SKILL.md evals

Each new SKILL.md is authored via `/skill-creator:skill-creator`
after writing eval scenarios. Evals are committed as checked-in
fixtures alongside each skill so they serve as long-term regression
protection:

- Lens evals: `skills/review/lenses/{lens}-lens/evals/`
- Output-format evals: `skills/review/output-formats/ticket-review-output-format/evals/`
- Orchestrator evals: `skills/tickets/review-ticket/evals/`

Evals drive the skill-creator's edit/test/iterate loop during
authoring. Once all evals pass, the SKILL.md is finalised for its
sub-phase, but the evals remain committed so future prose edits can
be validated against the original specification.

### End-to-end smoke test

Only one, in 4E. Runs `/review-ticket` against a real ticket,
verifies parallel lens execution, artefact persistence, and
re-review append semantics. Not automated — requires a live
Claude Code session.

### Manual Testing Steps

1. **After 4A**: diff `config-read-review.sh` and confirm
   `bash scripts/config-read-review.sh pr` and
   `bash scripts/config-read-review.sh plan` on a clean repo produce
   output byte-identical to pre-change.
2. **After 4B**: run `/accelerator:configure` and confirm the
   ticket review keys appear in the interactive help.
3. **After 4C/4D**: open each lens SKILL.md and verify it follows
   the six-section structure exactly.
4. **After 4E**: run the full end-to-end smoke test described in
   the 4E success criteria.

## Migration Notes

- No data migration. Existing tickets (including the 29
  `adr-creation-task` legacy tickets) are not transformed. They can
  still be passed to `/review-ticket`, but the completeness lens is
  likely to flag them heavily because they predate the richer
  template — this is correct behaviour and documented as expected in
  the lens.
- No plugin version bump is required for this phase on its own;
  version bumps are handled separately per the repo's release
  process.
- Config migration: users with existing `.claude/accelerator.md`
  files do not need to edit anything. New keys
  (`ticket_revise_severity`, `ticket_revise_major_count`) default
  sensibly. Per-type catalogue partitioning is transparent because
  the `BUILTIN_CODE_LENSES` array holds the same names as the
  previous flat `BUILTIN_LENSES`.

## References

- Research document:
  `meta/research/2026-04-08-ticket-management-skills.md`
- Prior phase plans:
  - `meta/plans/2026-04-08-ticket-management-phase-1-foundation.md`
  - `meta/plans/2026-04-19-ticket-creation-skills.md`
  - `meta/plans/2026-04-21-list-and-update-tickets.md`
- Primary structural model:
  `skills/planning/review-plan/SKILL.md`
- Output format model:
  `skills/review/output-formats/plan-review-output-format/SKILL.md`
- Lens structural model: `skills/review/lenses/architecture-lens/SKILL.md`
  (and `documentation-lens/SKILL.md` for a prose-heavy parallel)
- Reviewer agent: `agents/reviewer.md`
- Config script under refactor: `scripts/config-read-review.sh`
- Shared test helpers: `scripts/test-helpers.sh`
- Existing integration harness: `scripts/test-config.sh`
- Configure skill under extension: `skills/config/configure/SKILL.md`

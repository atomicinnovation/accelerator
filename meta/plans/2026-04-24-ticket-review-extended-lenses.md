---
date: "2026-04-24T10:28:36+01:00"
type: plan
skill: create-plan
ticket: ""
status: draft
---

# Ticket Review (Extended Lenses) — Phase 5

## Overview

Extend the Phase 4 ticket review infrastructure with two additional
ticket-specific lenses: `scope` and `dependency`. Both are authored
TDD-style via `/skill-creator:skill-creator` — eval scenarios are
written first and drive the iterative skill-authoring loop. Shell
script changes to `scripts/config-read-review.sh` are driven by new
assertions in `scripts/test-config.sh`. No new orchestrator code
paths, no new config keys, no new agents — the extensions slot into
existing hooks.

Phase 5 also tightens two invariants that Phase 4 established but
did not fully enforce: the **scope-boundary partition** (which lens
owns which failure mode) is extended with concrete rows for the two
new lenses and captured as *recurring* negative-output evals, not a
one-shot manual check; and the lens SKILL.md structural conformance
is checked by a new shared lint script rather than by visual
inspection.

## Current State Analysis

Phase 4 (`meta/plans/2026-04-22-ticket-review-core.md`) is complete.
The following artifacts are in place:

- `scripts/config-read-review.sh:54-58` — `BUILTIN_TICKET_LENSES`
  array holds three lenses: `completeness`, `testability`, `clarity`.
- `skills/review/lenses/completeness-lens/`,
  `skills/review/lenses/testability-lens/`,
  `skills/review/lenses/clarity-lens/` — each contains `SKILL.md` +
  `evals/` (evals.json, benchmark.json, files/).
- `skills/review/output-formats/ticket-review-output-format/SKILL.md:64-65`
  — inline list of valid ticket lens identifiers:
  `completeness`, `testability`, `clarity`.
- `skills/tickets/review-ticket/SKILL.md:64-70` — Available Review
  Lenses table lists exactly three rows; Step 2 prose references
  "three lenses" and runs all three by default.
- `scripts/test-config.sh:1819-1851` — asserts the ticket catalogue
  contains exactly three built-in lens rows and that none of the
  three names leak into `pr` or `plan` mode.

### Key Discoveries

- `BUILTIN_TICKET_LENSES` is the **single authoritative list** for
  built-in ticket lenses. Adding a name here propagates automatically
  through `_select_builtin_lenses_for_mode`
  (`config-read-review.sh:62-68`), the validation code
  (`config-read-review.sh:294`, `:358`), the default `core_lenses`
  resolution (`config-read-review.sh:412-413`), and the emitted Lens
  Catalogue table. No other script changes are required.
- The lens SKILL.md structural pattern is well-established. All three
  existing ticket lenses share the same six-section shape:
  Frontmatter (4 fields: `name`, `description`, `user-invocable:
  false`, `disable-model-invocation: true`), Persona sentence, `##
  Core Responsibilities`, `## Key Evaluation Questions`, `##
  Important Guidelines`, `## What NOT to Do`, closing "Remember:"
  paragraph.
- Lens scope boundaries are already reserved for scope and
  dependency. The Phase 4 plan
  (`meta/plans/2026-04-22-ticket-review-core.md:582-603`) declared:
  "Scope mismatch between sections" → clarity; "Section missing" →
  completeness; "Criterion unmeasurable" → testability. Scope and
  dependency occupy the remaining space:
  - **scope-lens**: ticket sizing (too big, too small), child
    decomposition for epics, orthogonality of requirements within
    a ticket, bundling multiple concerns, presence of out-of-scope
    notes where relevant.
  - **dependency-lens**: explicit identification of blockers,
    prerequisites, downstream consumers, external systems, and
    ordering constraints — not whether the Dependencies *section*
    is populated (completeness's concern), but whether the content
    names everything the ticket actually depends on.
- **Lens name note**: the lens identifier is `dependency`
  (singular), matching the adjective/noun-singular style of the
  Phase 4 siblings (`completeness`, `testability`, `clarity`,
  `scope`). The ticket template section is still named
  `Dependencies` — only the lens identifier is singular.
- **Extended scope boundaries** (Phase 5 addition, refines the
  Phase 4 table):

  | Failure mode                                            | Lens        |
  |---------------------------------------------------------|-------------|
  | Dependencies section absent entirely                    | completeness|
  | Dependencies section present but implied coupling       | dependency  |
  |   (blocker / consumer / external system) not captured   |             |
  | Epic child list absent entirely                         | completeness|
  | Epic child list present but decomposition incoherent    | scope       |
  | Spike exit criteria absent                              | completeness|
  | Spike exit criteria present but unmeasurable            | testability |
  | Spike exit criteria present but research question       | scope       |
  |   unbounded                                             |             |
  | Story bundles multiple independent units of work        | scope       |
  | Ordering between listed children implied but not        | dependency  |
  |   captured                                              |             |
  | Acceptance criterion ambiguous or contradictory         | clarity     |

  Completeness's Core Responsibility 3 ("note missing assumptions
  or dependencies implied by the body text") is explicitly
  narrowed in Phase 5D to defer implied-coupling reasoning to
  the `dependency` lens — completeness still flags absent
  *sections*, but not absent *content within a present section*.
  See Phase 5D Changes #2.
- The existing `completeness`, `testability`, and `clarity` SKILL.md
  `What NOT to Do` sections already reference scope and dependency
  as "the scope and dependencies lenses (Phase 5)" (anchor: the
  phrase `are the scope and dependencies lenses (Phase 5)` appears
  once in each file; line numbers are approximate — 117-118 in
  completeness, 113-114 in testability, 135-136 in clarity — but
  plan edits key off the text anchor, not the line number). When
  the new lenses ship, those forward references become concrete and
  both the `(Phase 5)` suffix and the `dependencies` plural form
  are removed (the plural is a leftover from an earlier naming
  scheme; the final lens identifier is `dependency`).
- `skills/review/lenses/{lens}-lens/evals/evals.json` is the canonical
  format driving `/skill-creator:skill-creator`. Each eval has `id`,
  `prompt` (with absolute paths to the lens SKILL.md, the output
  format, and a ticket fixture), `expected_output` (human-readable
  description of what the lens should flag), and `files: []`.
  Ticket fixtures live at `evals/files/<eval-name>/ticket.md`.
- `benchmark.json` is produced by the skill-creator iterate loop and
  committed alongside `evals.json` as long-term regression evidence
  (see `completeness-lens/evals/benchmark.json`).
- `scripts/test-config.sh` asserts the ticket catalogue via
  order-independent sort of the lens names emitted in the catalogue
  table (anchor: the block beginning `echo "Test: ticket mode
  catalogue contains"`). The test must be updated to expect five
  lenses in the sorted list, and the cross-mode leak test (anchor:
  the `for lens in completeness testability clarity` loop) must
  iterate over all five names.
- Golden-fixture invariants established in Phase 4A (byte-identical
  `pr` and `plan` output across sub-phases) must hold in Phase 5
  too. Phase 5A additionally captures a committed golden fixture
  for ticket-mode output at
  `scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`
  so ticket-mode regressions are caught against a stable baseline.

## Desired End State

After Phase 5 ships:

- `scripts/config-read-review.sh` lists five built-in ticket lenses.
  `bash scripts/config-read-review.sh ticket` on a clean repo emits
  a Lens Catalogue with five rows (clarity, completeness,
  dependency, scope, testability) — order does not matter;
  the assertion is over the sorted set.
- `bash scripts/config-read-review.sh pr` and
  `bash scripts/config-read-review.sh plan` produce output
  byte-identical to Phase 4 (no leak of ticket lenses into code
  review modes), enforced against committed golden fixtures at
  `scripts/test-fixtures/config-read-review/`.
- `scripts/config-read-review.sh ticket` emits a one-time
  informational note to stderr when `core_lenses` is set and does
  not include every non-disabled built-in.
- `skills/review/lenses/scope-lens/SKILL.md` and
  `skills/review/lenses/dependency-lens/SKILL.md` exist, pass the
  new `scripts/test-lens-structure.sh` lint (six-section shape,
  four frontmatter fields, persona sentence, peer-lens references,
  closing `Remember:` paragraph), and each has committed
  `evals/evals.json`, `evals/benchmark.json`, `evals/files/*/ticket.md`
  fixtures (nine for scope, eight for dependency), plus a
  `boundary_evals.json` + `boundary_benchmark.json` suite.
- `skills/review/output-formats/ticket-review-output-format/SKILL.md`
  defers to the Lens Catalogue as the single source for lens
  identifiers; no inline enumeration remains.
- `skills/tickets/review-ticket/SKILL.md` Available Review Lenses
  table contains five rows in alphabetical order; Step 2 describes
  the default as "every lens registered in `BUILTIN_TICKET_LENSES`"
  (count-neutral); the default behaviour runs all lenses unless
  filtered by config or focus arguments.
- The forward references to scope and dependency in the three
  existing lens SKILL.md `What NOT to Do` sections have the
  `(Phase 5)` suffix removed, the `dependencies` plural renamed to
  `dependency`, and are now concrete peer references.
- Completeness's Core Responsibility 3 is narrowed to defer
  implied-coupling reasoning to the `dependency` lens.
- All five lenses have a `boundary_evals.json` suite encoding
  negative-output expectations; boundary evals pass at 100% and
  participate in `mise run test`.
- `CHANGELOG.md` Unreleased section reads "Five-lens ticket review
  capability" with five lens bullets.
- `mise run test` passes.
- A real ticket passed to `/review-ticket` spawns five reviewer
  agents in parallel and aggregates findings across all five lenses;
  the persisted review artefact carries
  `lenses: [clarity, completeness, dependency, scope, testability]`
  in its frontmatter.

### Key Discoveries

(See Current State Analysis above; repeated here for plan-consumer
convenience.)

- `BUILTIN_TICKET_LENSES` in `scripts/config-read-review.sh` is the
  single authoritative registration point for built-in ticket
  lenses.
- The lens catalogue hook is implicitly driven; once a lens name is
  in the array, all downstream scripts, validators, and docs pick
  it up via the existing Lens Catalogue emission.
- Each lens's `What NOT to Do` section names the other four ticket
  lenses explicitly. Phase 5 closes the circle — completeness,
  testability, and clarity already reference the other four; scope
  and dependency must reference the other four too.
- The scope-boundary partition is enforced by recurring
  `boundary_evals.json` suites, not by a one-shot manual check.

## What We're NOT Doing

- **No new config keys.** `review.ticket_revise_severity` and
  `review.ticket_revise_major_count` already govern verdict
  thresholds and stay unchanged.
- **No orchestrator architecture change.** `review-ticket`'s Steps
  1, 3, 4, 5, 6, 7 are untouched (Step 7 re-review semantics stay
  as-is — see Migration Notes for the implication on pre-Phase-5
  review artefacts). Step 2 is rewritten to be count-neutral and
  gains two table rows; the confirmation gate is preserved from
  Phase 4.
- **No new agent definitions.** The generic reviewer agent
  (`agents/reviewer.md`) continues to take a lens path and output
  format path in its task prompt.
- **No `applies_to` frontmatter** on the two new SKILL.md files.
  Built-in ticket lenses are partitioned by the
  `BUILTIN_TICKET_LENSES` array, not by frontmatter — `applies_to`
  is reserved for custom user-space lenses (see
  `config-read-review.sh` — anchor: the `_is_custom_lens`
  dispatcher block).
- **No auto-detect selection logic.** Step 2 of `review-ticket`
  continues to run all lenses by default. The five ticket lenses
  cover orthogonal concerns; unlike `review-plan`'s 13 code lenses,
  there is no need for relevance-based auto-selection.
- **No change to the Step 2 confirmation gate.** The default path
  still asks "Shall I proceed?" even though it always selects
  every lens. Removing the gate is orthogonal to Phase 5's goals
  and deferred to a future phase.
- **No template changes.** `templates/ticket.md` already has all
  the sections both lenses need to evaluate (Dependencies, Summary,
  Context, Requirements, Acceptance Criteria, Open Questions,
  Assumptions).
- **No data migration.** Existing tickets are not re-reviewed, and
  existing review artefacts are not rewritten. Pre-Phase-5 review
  artefacts re-reviewed via Step 7 will not pick up the new
  lenses — see Migration Notes for the opt-in pathway (delete the
  artefact to force a fresh Pass 1).
- **No Step 7 auto-expansion.** Extending Step 7 to include
  newly-introduced built-in lenses on re-review is explicitly out
  of scope; it is a coverage-vs-focus tradeoff that should be
  decided when we add the next lens, not in Phase 5.
- **No `stress-test-ticket` or `refine-ticket` work.** Those belong
  to Phase 6.
- **No plugin version bump.** Version bumps are handled separately
  per the repo's release process. Phase 5A does amend the
  CHANGELOG Unreleased section in-place so release notes capture
  the change when the bump lands.

## Implementation Approach

Two interleaved tracks, mirroring the Phase 4 approach:

- **Shell-script TDD** for all changes to
  `scripts/config-read-review.sh`. Assertions go into
  `scripts/test-config.sh` first, must fail against the current
  script, and pass after the array is extended.
- **Skill-creator TDD** for both new SKILL.md files. Eval scenarios
  are written first as concrete `ticket.md` fixtures plus entries in
  `evals.json`. `/skill-creator:skill-creator` is then invoked with
  the evals as the specification; it iterates SKILL.md prose until
  all evals pass, records outcomes in `benchmark.json`, and the
  skill is finalised. Evals stay committed as long-term regression
  evidence.

Sub-phases are ordered by dependency. The plan uses alpha suffixes
— 5A through 5D — to match the Phase 4 convention.

1. **5A — Wiring** (`BUILTIN_TICKET_LENSES` extension, output
   format update, orchestrator table + prose update, one-time
   info message, CHANGELOG amendment, ticket-mode golden fixture
   capture). Does not require the SKILL.md files to exist because
   the catalogue emits plugin-cache paths whether or not the
   target file is present. Keeping 5A before the lens SKILL.md
   files means the row-count and sorted-set assertions in
   `test-config.sh` fail first and drive 5A's script change.
2. **5B — `scope-lens`**: first new lens. Establishes the Phase 5
   eval pattern (nine fixtures, three runs per eval per
   configuration) and demonstrates the full
   `/skill-creator:skill-creator` loop with the escape-hatch
   protocol. Also introduces `scripts/test-lens-structure.sh`
   wired into `mise run test`.
3. **5C — `dependency-lens`**: second new lens, reusing the 5B
   pattern with eight fixtures. Not parallelised with 5B:
   skill-creator's iterate loop is conversational and serial;
   sharing the conversation keeps cross-lens scope boundaries
   coherent.
4. **5D — Cross-lens regression, boundary evals, orchestrator
   smoke test**: remove `(Phase 5)` suffixes AND rename
   `dependencies` → `dependency` in the three existing lens
   SKILL.md files; narrow completeness's Core Responsibility 3;
   introduce five `boundary_evals.json` suites (one per lens) as
   recurring negative-output regression guards; re-run all five
   lens eval suites to confirm no regression; run the end-to-end
   smoke test.

## Phase 5A: Wiring for Scope and Dependency Lenses

### Overview

Register `scope` and `dependency` in `BUILTIN_TICKET_LENSES`,
update the inline lens-identifier list in the output format
SKILL.md, extend the orchestrator's Available Review Lenses
table + Step 2 prose, emit a one-time informational note for
`core_lenses` overrides, and amend the CHANGELOG Unreleased
section. The two target SKILL.md files do not need to exist yet
for 5A's tests to pass — the Lens Catalogue emits paths, not
validated file contents.

### Changes Required

#### 1. `scripts/test-config.sh`

**File**: `scripts/test-config.sh`
**Changes**: Update existing ticket-mode assertions to expect five
lenses using the `assert_eq` helper; add assertions proving
cross-mode isolation for the new names; add a capture-golden
assertion for ticket-mode output.

Replace the block at lines 1819-1851 with the following. The
block uses `assert_eq` from `scripts/test-helpers.sh` for
uniform PASS/FAIL output, replaces the fragile inline
awk-pipeline with a small helper call, and removes the
short-circuit `break` in the cross-mode loop so all leaks are
enumerated:

```bash
# Helper local to this block: extract sorted built-in lens names
# from a catalogue output. Accepts the output on stdin.
_extract_builtin_lens_names() {
  grep "| built-in |" \
    | awk -F'|' '{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2}' \
    | sort \
    | tr '\n' ' ' \
    | sed 's/ $//'
}

echo "Test: ticket mode catalogue contains exactly 5 built-in rows"
REPO=$(setup_repo)
TICKET_OUT=$(cd "$REPO" && bash "$READ_REVIEW" ticket 2>/dev/null)
# grep -c returns 1 on no match; use pipeline that always exits 0
CATALOGUE_LINES=$(echo "$TICKET_OUT" | awk '/\| .* \| .* \| built-in \|/ {c++} END {print c+0}')
assert_eq "ticket mode emits 5 built-in lens rows" 5 "$CATALOGUE_LINES"

echo "Test: ticket mode catalogue emits the expected sorted lens set"
SORTED_LENSES=$(echo "$TICKET_OUT" | _extract_builtin_lens_names)
assert_eq "ticket mode sorted lens set" \
  "clarity completeness dependency scope testability" \
  "$SORTED_LENSES"

echo "Test: none of the five ticket lenses appear in pr or plan mode"
REPO=$(setup_repo)
PR_OUT=$(cd "$REPO" && bash "$READ_REVIEW" pr)
PLAN_OUT=$(cd "$REPO" && bash "$READ_REVIEW" plan)
LEAKED=""
for lens in completeness testability clarity scope dependency; do
  if echo "$PR_OUT" | grep -q "| $lens |"; then
    LEAKED="$LEAKED pr:$lens"
  fi
  if echo "$PLAN_OUT" | grep -q "| $lens |"; then
    LEAKED="$LEAKED plan:$lens"
  fi
done
assert_eq "no ticket lens leaks into pr or plan catalogue" "" "$LEAKED"

echo "Test: ticket-mode output is byte-identical to its committed golden fixture"
TICKET_GOLDEN="$SCRIPT_DIR/test-fixtures/config-read-review/ticket-mode-golden.txt"
assert_eq "ticket-mode output matches golden fixture" \
  "$(cat "$TICKET_GOLDEN")" \
  "$TICKET_OUT"
```

A new committed fixture `scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`
captures the post-5A ticket-mode output byte-for-byte. It is
regenerated once at the end of 5A (`bash scripts/config-read-review.sh
ticket > scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`)
and is then frozen for 5B, 5C, and 5D. This mirrors the existing
pr/plan golden-fixture invariant.

**Red-step scope**: Only the row-count and sorted-lens-set
assertions participate in the TDD Red→Green cycle. They fail
against the unchanged 3-lens script (catalogue emits three rows;
sorted set is `clarity completeness testability`) and pass after
the array is extended. The cross-mode isolation assertion is a
*future-proofing invariant*: against the unchanged script it
passes trivially (scope/dependency are absent from pr/plan
output), not because the isolation logic works but because the
lenses do not yet exist anywhere. This is acceptable — the
assertion earns its keep if anyone later mis-routes a ticket
lens into pr/plan during Phase 5+ or beyond. The golden-fixture
assertion cannot be written until after `config-read-review.sh`
is updated; it is added as part of the Green step.

Before changing `config-read-review.sh`, run `bash
scripts/test-config.sh` and verify the row-count and sorted-set
assertions fail. That failure is the Red step of TDD.

#### 2. `scripts/config-read-review.sh`

**File**: `scripts/config-read-review.sh`
**Changes**: Extend the `BUILTIN_TICKET_LENSES` array (anchor:
the `# Built-in lens names for ticket reviews (ticket mode)`
comment block) from three entries to five. Alphabetical order
for readability and to match the sorted assertion.

```bash
# Built-in lens names for ticket reviews (ticket mode)
BUILTIN_TICKET_LENSES=(
  clarity
  completeness
  dependency
  scope
  testability
)
```

After this change, re-run `bash scripts/test-config.sh`. The
row-count and sorted-set assertions must pass; the cross-mode
isolation assertion must continue to pass; the ticket-mode
golden fixture must be captured (see Changes #1) and its
assertion must pass. All pr/plan assertions must remain
unchanged (Green step).

#### 3. `skills/review/output-formats/ticket-review-output-format/SKILL.md`

**File**: `skills/review/output-formats/ticket-review-output-format/SKILL.md`
**Changes**:

- Replace the sentence whose current text is
  `The canonical source of valid lens identifiers is the Lens
  Catalogue emitted by` ... `The current ticket lenses are:
  `completeness`, `testability`, `clarity`.` with:

  ```markdown
  The canonical source of valid lens identifiers is the Lens
  Catalogue emitted by `config-read-review.sh ticket`. See the
  Lens Catalogue for the current list.
  ```

  The inline enumeration is dropped entirely: the Lens
  Catalogue is the single source of truth, and the previous
  hand-maintained list was a second source that would drift on
  every lens addition.

- In the `lens` field example (the JSON schema example block
  that currently shows `"completeness"`, `"testability"`,
  `"clarity"` as example identifiers), replace those three
  values with a short ellipsis-form placeholder:
  `(e.g., "completeness", "scope", …)`. This removes the
  per-lens amendment footprint.

#### 4. `skills/tickets/review-ticket/SKILL.md`

**File**: `skills/tickets/review-ticket/SKILL.md`
**Changes**:

- Replace the entirety of the `## Available Review Lenses`
  table (anchor: the table header row
  `| Lens              | Lens Skill`) with five rows in
  alphabetical order. Column widths are normalised so all rows
  align:

```markdown
| Lens             | Lens Skill          | Focus                                                                   |
|------------------|---------------------|-------------------------------------------------------------------------|
| **Clarity**      | `clarity-lens`      | Unambiguous referents, internal consistency, jargon handling            |
| **Completeness** | `completeness-lens` | Section presence, content density, type-appropriate content            |
| **Dependency**   | `dependency-lens`   | Implied couplings not captured — blockers, consumers, external systems |
| **Scope**        | `scope-lens`        | Right-sized, single coherent unit of work; decomposition; orthogonality |
| **Testability**  | `testability-lens`  | Measurable criteria, verifiable outcomes, verification framing          |
```

Add a one-line clarification immediately under the table:

> Note: completeness flags an *absent* Dependencies section;
> dependency flags an *empty or underspecified* section whose
> contents fail to name every coupling the ticket implies.

- Replace the entirety of Step 2 (anchor: the `### Step 2:
  Select Review Lenses` heading, through the closing
  triple-backtick that ends the sample-confirmation code
  block) with:

  ````markdown
  ### Step 2: Select Review Lenses

  By default, run every lens registered in `BUILTIN_TICKET_LENSES`
  unless the user has provided focus arguments or config
  restricts the selection. The five ticket lenses cover orthogonal
  concerns, so there is no relevance-based auto-selection.

  **If the user provided focus arguments:**

  - Map the focus areas to the corresponding lenses
  - Include any additional lenses that are clearly relevant
  - Briefly explain which lenses you're running

  **If no focus arguments were provided:**

  Run all built-in ticket lenses unless:
  - A lens is listed in `disabled_lenses` — remove it from the
    active set
  - The user's configured `core_lenses` has filtered this to a
    subset (see below)

  When `core_lenses` is set in config, apply it as the *minimum
  required set*; add any remaining non-disabled lenses up to
  `max_lenses`. This means users who pinned `core_lenses` to
  `[completeness, testability, clarity]` in Phase 4 will also
  receive `scope` and `dependency` on upgrade, unless they add
  those names to `disabled_lenses` or set `max_lenses` to their
  subset size.

  Present the selection briefly — enumerate the chosen lenses
  with a one-line focus each — then wait for confirmation before
  spawning reviewers. The confirmation gate is preserved from
  Phase 4 even though the default always selects every lens; the
  gate is useful when focus args or config have narrowed the set.

  Example (default path, no focus args, no `core_lenses`
  restriction):

  ```
  I'll review this ticket through all ticket lenses (clarity,
  completeness, dependency, scope, testability). Shall I proceed?
  ```

  Wait for confirmation before spawning reviewers.
  ````

- Replace the frontmatter `description` field with a concise,
  count-neutral phrasing that does not enumerate lens names:

  ```yaml
  description: Review a ticket through multiple ticket-quality
    lenses and collaboratively iterate based on findings. Use
    when the user wants to evaluate a ticket before
    implementation or escalation.
  ```

- In the `## Important Guidelines` section, replace the phrase
  `the three ticket lenses` (and any other occurrence of
  `three ticket lenses` in that file) with `the ticket lenses`.
  This removes the literal count word so future lens additions
  do not require another prose sweep.

**Prose-drift safety net**: after applying the edits above, run
`grep -n -E '\bthree\b|\[completeness, testability, clarity\]' skills/tickets/review-ticket/SKILL.md skills/review/output-formats/ticket-review-output-format/SKILL.md`
and verify no unintentional residual references remain. Expected
intentional matches are: none. Any match indicates a missed
edit. This check is enforced by Success Criteria below.

#### 5. One-time informational note on `core_lenses` overrides

**File**: `scripts/config-read-review.sh`
**Changes**: When the caller's `core_lenses` is set and does
not include every non-disabled built-in ticket lens, emit a
single informational line to stderr (not stdout — stdout
remains the catalogue output and its golden fixture must
stay byte-identical).

Implementation sketch (pseudocode; exact placement is inside
the existing config-resolution block):

```bash
if [ "$MODE" = "ticket" ] && [ -n "$CORE_LENSES_SET" ]; then
  MISSING=""
  for lens in "${BUILTIN_TICKET_LENSES[@]}"; do
    if ! _is_in "$lens" "$DISABLED_LENSES" \
       && ! _is_in "$lens" "$CORE_LENSES"; then
      MISSING="$MISSING $lens"
    fi
  done
  if [ -n "$MISSING" ]; then
    MISSING="${MISSING# }"
    printf >&2 'Note: built-in ticket lens(es) not in your core_lenses but will be added up to max_lenses: %s\n' "$MISSING"
    printf >&2 '      Add them to disabled_lenses to opt out, or raise core_lenses to include them explicitly.\n'
  fi
fi
```

Add an assertion in `scripts/test-config.sh` verifying the note
fires when `core_lenses=[completeness, testability, clarity]`
and does *not* fire when `core_lenses` is empty. The assertion
reads stderr via `2>&1 1>/dev/null` separation, and uses
`grep -q 'Note: built-in ticket lens'` — so the exact message
text can evolve without breaking the test.

#### 6. CHANGELOG Unreleased section amendment

**File**: `CHANGELOG.md`
**Changes**: Amend the existing entry under `Unreleased` that
reads `Three-lens ticket review capability` to read
`Five-lens ticket review capability`. Extend the bullet list
under that entry to include `scope` and `dependency` alongside
the existing three, preserving alphabetical order. Example
post-edit fragment (exact surrounding text depends on the
current CHANGELOG shape):

```markdown
### Added

- Five-lens ticket review capability (`/review-ticket`)
  combining completeness, testability, clarity, scope, and
  dependency lenses.
- `scope-lens` — evaluates ticket sizing, decomposition, and
  orthogonality of requirements.
- `dependency-lens` — evaluates whether implied couplings
  (blockers, consumers, external systems, ordering) are
  explicitly captured.
```

No version bump is performed in this phase; the Unreleased
section carries the change until the next release cadence.

### Success Criteria

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` — row-count assertion:
      ticket mode emits exactly 5 built-in rows.
- [ ] `bash scripts/test-config.sh` — sorted-set assertion:
      ticket mode sorted lens set equals
      `"clarity completeness dependency scope testability"`.
- [ ] `bash scripts/test-config.sh` — cross-mode isolation:
      `LEAKED` variable is empty (none of the five ticket
      lenses appear in pr or plan output).
- [ ] `bash scripts/test-config.sh` — ticket-mode golden
      fixture assertion: `bash scripts/config-read-review.sh
      ticket` on a clean repo matches the committed fixture at
      `scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`
      byte-for-byte.
- [ ] `bash scripts/test-config.sh` — `core_lenses`
      informational-note assertion: the note fires when
      `core_lenses=[completeness, testability, clarity]` and
      does not fire when `core_lenses` is empty.
- [ ] `mise run test` passes.
- [ ] Golden-fixture regression: `bash scripts/config-read-review.sh
      pr` output on a clean repo is byte-identical to the pre-5A
      capture (compare against a just-before-5A snapshot, since no
      canonical committed fixture exists for Phase 4 pr/plan —
      Phase 5A optionally captures these alongside the ticket
      golden fixture for future regression protection).
- [ ] Golden-fixture regression: `bash scripts/config-read-review.sh
      plan` output on a clean repo is byte-identical to the
      pre-5A capture.
- [ ] `bash scripts/config-read-review.sh ticket` exits 0 (the
      target SKILL.md paths may not exist yet — the script emits
      paths without validating file presence).
- [ ] Prose-drift sweep:
      `grep -n -E '\bthree\b|\[completeness, testability, clarity\]' \
      skills/tickets/review-ticket/SKILL.md \
      skills/review/output-formats/ticket-review-output-format/SKILL.md`
      returns zero matches.
- [ ] CHANGELOG amendment present: `grep -q 'Five-lens ticket
      review capability' CHANGELOG.md` succeeds.

#### Manual Verification:

- [ ] Open `review-ticket/SKILL.md` and confirm: the Available
      Review Lenses table has five rows in alphabetical order with
      aligned column widths; Step 2 describes the default as "run
      every lens registered in `BUILTIN_TICKET_LENSES`" (not a
      hard-coded count); Important Guidelines references "the
      ticket lenses" (not "the five ticket lenses").
- [ ] Open `ticket-review-output-format/SKILL.md` and confirm the
      lens-identifier sentence defers to the Lens Catalogue and
      no inline enumeration remains.
- [ ] Open `CHANGELOG.md` and confirm the Unreleased entry reads
      "Five-lens ticket review capability" with five lens bullets.
- [ ] Diff the three existing ticket lens SKILL.md files — they
      remain unchanged in 5A (the "(Phase 5)" suffix removal lives
      in 5D).

---

## Phase 5B: scope-lens

### Overview

Author the `scope` lens via `/skill-creator:skill-creator`. The
scope lens evaluates whether a ticket is appropriately sized —
neither too broad nor too narrow — and whether its requirements
are orthogonal (a single coherent unit of work) rather than
bundling multiple independent concerns. For epics, it evaluates
whether the decomposition strategy describes a sensible split;
for spikes, whether the time-box and research questions define a
bounded exploration.

This lens focuses on ticket *shape*, not ticket *content quality*:

- Completeness asks "is the Acceptance Criteria section present?"
- Testability asks "is each criterion measurable?"
- Clarity asks "is each requirement unambiguous?"
- Scope asks "does this ticket describe one unit of work at the
  right granularity, and can it be delivered incrementally?"

### Changes Required

#### 1. `skills/review/lenses/scope-lens/evals/files/`

Create the following eight ticket fixtures. Each fixture is a
minimal but realistic ticket that exhibits exactly one scope
failure mode (or, for baselines, exhibits none).

**File**: `evals/files/epic-bundling-unrelated-themes/ticket.md`
Epic titled "Platform improvements Q2" that lumps together: user
profile rework, billing migration to Stripe, mobile app push
notifications, and admin audit logs. No coherent decomposition
strategy tying them to a single capability.
**Expected**: finding at `major` or higher, location `Summary` or
`Requirements`, noting the ticket bundles unrelated themes that
should be separate epics.

**File**: `evals/files/story-mixing-feature-and-refactor/ticket.md`
Story that says: "Add CSV export to the reports dashboard and
migrate the dashboard to the new grid framework." The CSV export
is well-scoped; the framework migration is a multi-week
infrastructure effort unrelated to the export feature.
**Expected**: finding at `major` or higher, location `Requirements`,
noting two independent units of work that should be separate
tickets.

**File**: `evals/files/undersized-standalone-story/ticket.md`
Story titled "Rename `user_email` column to `user_email_address`"
with a single one-line requirement. No cross-cutting concerns,
no dependent consumers.
**Expected**: finding at `minor` or `suggestion`, location
`Summary` or `Frontmatter: type`, noting this is a trivial change
that should be a `chore` or `task` type rather than a `story`.
`confidence` should be `medium` to reflect the judgement call.

**File**: `evals/files/spike-unbounded-exploration/ticket.md`
Spike titled "Understand our microservices architecture" with
exit criteria "have a good grasp of the system" and no time-box.
**Expected**: finding at `major` or higher, location
`Acceptance Criteria` or the spike's exit criteria section, noting
unbounded exploration scope that cannot be completed as a single
spike. (Distinct from the testability lens: that would flag the
exit criteria as unmeasurable; scope flags the underlying question
as too broad to bound.)

**File**: `evals/files/over-decomposed-epic/ticket.md`
Epic that lists twelve child stories, most of which are one-line
chores (e.g., "rename a variable", "add a comment"). The epic's
Summary describes a single small refactor.
**Expected**: finding at `minor` or higher, location
`Requirements` or the Stories list, noting over-decomposition —
the whole epic could be a single story or two at most.

**File**: `evals/files/story-crossing-bounded-contexts/ticket.md`
Story that says "When an order is placed, the inventory service
reserves stock, the billing service charges the card, and the
notification service emails the customer." Three distinct service
boundaries, no owning team identified, no orchestration
decision captured.
**Expected**: finding at `major` or higher, location `Requirements`,
noting the ticket spans multiple bounded contexts / service
boundaries that require separate tickets per service plus a
coordination ticket.

**File**: `evals/files/well-scoped-story/ticket.md`
Clean baseline: story with a single coherent capability —
"Add a 'copy to clipboard' button to code blocks in the
documentation site". Three orthogonal acceptance criteria all
covering the same capability.
**Expected**: no findings at `critical` or `major`; may have
zero or one `suggestion` findings. `strengths` list notes the
coherent scope.

**File**: `evals/files/well-scoped-spike/ticket.md`
Clean baseline: spike titled "Evaluate three rate-limiter
libraries for the public API" with a 3-day time-box and three
enumerable exit criteria (a decision memo, a comparison matrix,
and a recommended default).
**Expected**: zero findings at `critical` or `major`; at most
one `minor`/`suggestion` finding total. `strengths` list notes
bounded scope and concrete exit criteria. Grader rejects any
finding whose body mentions sizing, bundling, decomposition, or
unbounded exploration.

**File**: `evals/files/well-scoped-epic/ticket.md` (NEW — added to
balance epic coverage per the test-coverage review; previously the
epic fixtures were both negative)
Clean baseline: epic titled "Search bar on the documentation
site" with three coherent child stories — "Add search index
pipeline", "Add search UI component", "Add analytics for
search usage" — all serving one user-visible capability.
Decomposition strategy is stated in the Summary and the
children are listed explicitly.
**Expected**: zero findings at `critical` or `major`; at most
one `minor`/`suggestion` finding total. `strengths` list notes
coherent epic scope and well-articulated decomposition. Grader
rejects any finding whose body mentions bundling or
over/under-decomposition.

#### 2. `skills/review/lenses/scope-lens/evals/evals.json`

Write `evals.json` following the completeness lens pattern
(`skills/review/lenses/completeness-lens/evals/evals.json`). The
`skill_name` is `"scope"`. Each eval entry has:

- `id`: 1-9 (9 fixtures: 6 negative, 3 positive baselines)
- `prompt`: absolute-path prompt directing the reviewer agent to
  read `skills/review/lenses/scope-lens/SKILL.md`, the output
  format at
  `skills/review/output-formats/ticket-review-output-format/SKILL.md`,
  and the fixture ticket. Paths match the reviewer agent's spawn
  prompt in `review-ticket/SKILL.md` (anchor: the `## Step 3:
  Spawn Review Agents` heading and the agent-prompt template
  below it).
- `expected_output`: the "Expected" description above, rephrased
  as first-person grader guidance. For baselines, the grader
  must reject any finding whose body mentions the failure
  modes the lens is designed to catch (bundling, sizing,
  decomposition, unbounded exploration).
- `files`: `[]` (files are implicit in the prompt's ticket path).

Concrete example entry (to serve as the template for the other
eight; all fields are final form, not pseudocode):

```json
{
  "id": 1,
  "skill_name": "scope",
  "prompt": "Read /Users/.../skills/review/lenses/scope-lens/SKILL.md and /Users/.../skills/review/output-formats/ticket-review-output-format/SKILL.md, then review the ticket at /Users/.../skills/review/lenses/scope-lens/evals/files/epic-bundling-unrelated-themes/ticket.md through the scope lens. Return a single JSON code block matching the output format.",
  "expected_output": "The reviewer produces at least one finding at `major` or higher severity, locating the concern in either the Summary or Requirements section, whose body identifies that the epic bundles unrelated themes (e.g., user profile rework + billing migration + push notifications) and recommends splitting into separate epics.",
  "files": []
}
```

**Multi-run recommendation**: per the benchmark.json note on
the completeness lens, single-run evals cannot surface
non-deterministic failure modes. Run each eval three times per
configuration (`with_skill` and baseline `without_skill`) and
record all runs in `benchmark.json` under the existing
`run_number` schema. Baselines are the highest-value
multi-run targets because false negatives are where the lens
drifts silently.

#### 3. `skills/review/lenses/scope-lens/SKILL.md`

**Do not author this file directly.** Instead, invoke
`/skill-creator:skill-creator` with the following intent:

> Create a ticket review lens named `scope` at
> `skills/review/lenses/scope-lens/SKILL.md`. The lens follows
> the structural pattern used by `completeness-lens/SKILL.md`,
> `testability-lens/SKILL.md`, and `clarity-lens/SKILL.md` —
> please read those three files first and match their shape
> exactly: six H2 sections (`## Core Responsibilities`, `## Key
> Evaluation Questions`, `## Important Guidelines`, `## What NOT
> to Do`, plus frontmatter + H1 + a single persona sentence
> between H1 and the first H2, plus a closing `Remember:`
> paragraph). The persona sentence must match the
> `Review as a ... specialist ...` shape used by the exemplars.
> Frontmatter must have exactly four fields: `name`,
> `description`, `user-invocable: false`,
> `disable-model-invocation: true`. Do not add an `applies_to`
> frontmatter field — built-in ticket lenses are partitioned via
> `BUILTIN_TICKET_LENSES` in `scripts/config-read-review.sh`.
> Evals are at `skills/review/lenses/scope-lens/evals/`. Run the
> evals against drafts and iterate until all nine pass. Do not
> relax eval `expected_output` strings to make them pass. If you
> cannot reach 9/9 after three full iterations, stop, capture
> the failing eval IDs and the current SKILL.md draft in
> `meta/reviews/skills/scope-lens-skill-creator-notes.md`, and
> escalate — do not hand-author around the problem.

The skill-creator produces:

- `evals/benchmark.json` recording each iteration's pass/fail
  outcome per eval and per configuration (with_skill vs baseline
  without_skill), with three runs per configuration.
- `skills/review/lenses/scope-lens/SKILL.md` in its final,
  all-evals-passing state.

**Expected SKILL.md structure** (for reviewer alignment):

- Frontmatter: `name: scope`, `description: Ticket review lens
  for evaluating sizing, decomposition, and orthogonality of
  requirements. Used by review orchestrators — not invoked
  directly.` (note the terse form matching the existing three
  lenses' ~20-word style — do not expand), `user-invocable:
  false`, `disable-model-invocation: true`.
- Persona: "Review as a ticket sizing specialist evaluating
  whether this ticket describes one coherent unit of work at the
  right level of granularity."
- `## Core Responsibilities` — 3-4 numbered items covering:
  coherent unit of work (orthogonality), appropriate sizing for
  the ticket type, decomposition strategy for epics, time-box /
  exit-criteria bounding for spikes.
- `## Key Evaluation Questions` — grouped by applicability
  (`**Sizing and bundling** (always applicable):`,
  `**Type-specific sizing** (based on ticket type):`).
- `## Important Guidelines` — includes "Rate confidence" line,
  "Be proportional" line (a slight sizing drift is not a major
  finding), "Do not read source code or run codebase exploration
  agents" line.
- `## What NOT to Do` — first bullet names the other four ticket
  lenses (clarity, completeness, dependency, testability) as
  off-scope. Distinguishes scope from testability ("unbounded
  exit criteria": scope flags unbounded research question;
  testability flags unmeasurable exit criterion — the same
  ticket may trigger both lenses for related but distinct
  reasons).
- Closing "Remember:" paragraph.

#### 4. `scripts/test-lens-structure.sh` (NEW shared lint script)

**File**: `scripts/test-lens-structure.sh`
**Changes**: Create a small lint script that, for every
`skills/review/lenses/*-lens/SKILL.md`, asserts:

- Frontmatter has exactly the four required keys (`name`,
  `description`, `user-invocable`, `disable-model-invocation`)
  with the expected values (`user-invocable: false`,
  `disable-model-invocation: true`).
- The six required section headings are present by exact string
  match: `## Core Responsibilities`, `## Key Evaluation
  Questions`, `## Important Guidelines`, `## What NOT to Do`.
  (H1 and persona sentence checked separately.)
- A single persona sentence exists between the H1 and the first
  H2 heading.
- `## What NOT to Do` body is non-empty and names at least three
  of the other peer ticket lenses from `BUILTIN_TICKET_LENSES`.
- A closing `Remember:` paragraph exists.

Wire the script into `mise run test` (added as a new step in
the existing task chain). The script lints every file in
`skills/review/lenses/*-lens/` — including the three existing
lenses — so Phase 5B's changes must not regress those.

### Success Criteria

#### Automated Verification:

- [ ] `skills/review/lenses/scope-lens/SKILL.md` exists and
      `bash scripts/test-lens-structure.sh scope-lens` passes
      (enforces the six-section shape, four frontmatter fields,
      persona sentence, peer-lens references, and closing
      `Remember:` paragraph).
- [ ] `skills/review/lenses/scope-lens/evals/evals.json` exists
      and parses as JSON
      (`python3 -c "import json; json.load(open('.../evals.json'))"`).
- [ ] `skills/review/lenses/scope-lens/evals/benchmark.json`
      exists and records pass outcomes for all nine evals under
      the `with_skill` configuration, with three runs per eval
      per configuration.
- [ ] All nine fixture tickets exist at
      `skills/review/lenses/scope-lens/evals/files/<name>/ticket.md`.
- [ ] `mise run test` passes (now includes the lens structural
      lint step added in Changes #4).
- [ ] `bash scripts/test-config.sh` — no regression from 5A
      (ticket catalogue still emits five rows; ticket-mode
      golden fixture unchanged; pr and plan fixtures unchanged).

#### Manual Verification:

- [ ] Open `scope-lens/SKILL.md`. Section headings and ordering
      match `completeness-lens/SKILL.md`.
- [ ] `## What NOT to Do` names the other four ticket lenses
      explicitly (not "(Phase 5)" references — this is a new lens
      authored after Phase 5 exists).
- [ ] Re-run all nine evals via skill-creator and confirm each
      produces a finding matching the `expected_output`
      description. On baselines, confirm the grader rejects any
      finding body that mentions the failure modes the lens is
      designed to catch.
- [ ] On the three baseline fixtures (`well-scoped-story`,
      `well-scoped-spike`, `well-scoped-epic`), confirm zero
      `critical` or `major` findings across all three runs and
      that `strengths` is non-empty.

---

## Phase 5C: dependency-lens

### Overview

Author the `dependency` lens via `/skill-creator:skill-creator`.
The dependency lens evaluates whether the ticket explicitly
identifies its prerequisites (blockers), its downstream consumers
(what this blocks), its external-system or cross-team couplings,
and any ordering constraints implied by the work. It focuses on
*what is missing from the content* — not whether the Dependencies
*section* is present (that is completeness's concern).

### Changes Required

#### 1. `skills/review/lenses/dependency-lens/evals/files/`

Create the following eight ticket fixtures.

**File**: `evals/files/implied-external-dep-missing/ticket.md`
Story that requires reading a new Stripe webhook event type. The
ticket body mentions "parse the `charge.refunded` webhook" but
the Dependencies section is empty — no mention that this requires
Stripe account configuration, webhook registration, or the
shared-secrets manager change to store the webhook signing key.
**Expected**: finding at `major` or higher, location
`Dependencies`, noting the ticket implicitly depends on Stripe
configuration + secrets management but names neither.

**File**: `evals/files/missing-blocks-downstream/ticket.md`
Foundational story to "introduce a versioned API schema file"
with three to four follow-up consumer tickets mentioned in
Context as "will enable further work". Dependencies section
populates Blocked by but leaves Blocks empty.
**Expected**: finding at `minor` or `major`, location
`Dependencies`, noting the Context clearly implies downstream
consumers but they are not captured as Blocks entries.

**File**: `evals/files/epic-unordered-children/ticket.md`
Epic that lists eight child stories in the Stories section. Two
of them have inherent ordering (one must complete before the
other), but no ordering is captured and no dependencies between
children are listed.
**Expected**: finding at `major` or higher, location `Stories` or
`Dependencies`, noting ordering constraints are implied but not
captured.

**File**: `evals/files/circular-dep-to-parent-epic/ticket.md`
Story whose Dependencies section lists "Blocked by: 0042 (parent
epic)" — but the parent epic is itself a decomposition and cannot
be completed until its children are. The circularity is implicit
but traceable from the Context.
**Expected**: finding at `major` or higher, location
`Dependencies`, noting the self-referential dependency on the
parent epic.

**File**: `evals/files/vendor-dep-not-captured/ticket.md`
Bug ticket for a broken export feature. The body states "the
export calls the Hubspot CRM API and the response shape changed"
but the Dependencies section is empty. No mention that
remediation is blocked on either a Hubspot API version choice or
on the Hubspot team's deprecation timeline.
**Expected**: finding at `major` or higher, location
`Dependencies`, noting the external vendor dependency is stated
in the body but not captured.

**File**: `evals/files/spike-missing-downstream-consumers/ticket.md`
Spike titled "Choose a message queue vendor" with a well-bounded
time-box and three enumerable exit criteria. No Dependencies
section entries despite Context naming three feature tickets that
are waiting on this spike's decision.
**Expected**: finding at `minor` or higher, location
`Dependencies`, noting downstream consumers of the spike's
decision are named in Context but not captured as Blocks.

**File**: `evals/files/clean-dependencies/ticket.md`
Clean baseline: story with a complete Dependencies section —
Blocked by lists a prerequisite ticket and a shared-secrets
rotation; Blocks lists two downstream tickets; an "External"
sub-item names the external API and its SLA concerns.
**Expected**: zero findings at `critical` or `major`; at most
one `minor`/`suggestion` finding total. `strengths` list notes
explicit, well-structured dependencies. Grader rejects any
finding whose body alleges missing couplings.

**File**: `evals/files/standalone-chore-no-deps/ticket.md`
Clean baseline: simple chore (e.g., "bump eslint to latest
minor") with an empty Dependencies section. No implied coupling
in the body, no downstream consumers implied.
**Expected**: zero findings at `critical` or `major`. At most
one `minor`/`suggestion` finding (e.g., "the empty Dependencies
section is appropriate for a standalone chore"). Grader rejects
any finding at `major` or higher.

#### 2. `skills/review/lenses/dependency-lens/evals/evals.json`

Same pattern as 5B. `skill_name: "dependency"`. Each eval has
`id` 1-8, a `prompt` pointing at
`skills/review/lenses/dependency-lens/SKILL.md`, the output
format, and the fixture ticket path. `expected_output` rephrases
the "Expected" block above as grader guidance. For baselines,
the grader must reject findings at `major` or higher. Run each
eval three times per configuration, matching the 5B schema.

#### 3. `skills/review/lenses/dependency-lens/SKILL.md`

Invoke `/skill-creator:skill-creator` with intent:

> Create a ticket review lens named `dependency` at
> `skills/review/lenses/dependency-lens/SKILL.md`. Follow the
> structural pattern used by
> `skills/review/lenses/scope-lens/SKILL.md` and the three core
> ticket lenses — six H2 sections, four frontmatter fields, a
> single persona sentence, a closing `Remember:` paragraph. The
> persona sentence must match the `Review as a ... specialist
> ...` shape used by the exemplars. Evals are at
> `skills/review/lenses/dependency-lens/evals/`. Iterate until
> all eight evals pass. Do not relax eval `expected_output`
> strings to make them pass. If you cannot reach 8/8 after
> three full iterations, stop, capture the failing eval IDs and
> the current SKILL.md draft in
> `meta/reviews/skills/dependency-lens-skill-creator-notes.md`,
> and escalate. Do not add an `applies_to` frontmatter field.

**Expected SKILL.md structure**:

- Frontmatter: `name: dependency`, `description: Ticket review
  lens for evaluating explicit capture of blockers, consumers,
  external systems, and ordering. Used by review orchestrators
  — not invoked directly.` (note the terse form matching the
  existing three lenses' ~20-word style — do not expand),
  `user-invocable: false`, `disable-model-invocation: true`.
- Persona: "Review as a dependency-mapping specialist evaluating
  whether every coupling the ticket implies is explicitly
  captured."
- `## Core Responsibilities` — 3-4 numbered items covering:
  upstream blockers, downstream consumers, external-system and
  cross-team couplings, ordering constraints within decomposed
  work (epics, related stories).
- `## Key Evaluation Questions` — grouped by applicability
  (`**Explicit coupling** (always applicable):`,
  `**Type-specific dependencies** (based on ticket type):`).
- `## Important Guidelines` — "Rate confidence" line, "Judge
  implied vs absent" line (an empty Dependencies section is not
  itself a finding — the lens asks whether the *content* implies
  couplings that should be captured), "Do not read source code or
  run codebase exploration agents" line.
- `## What NOT to Do` — first bullet names the other four ticket
  lenses (clarity, completeness, scope, testability) as
  off-scope. Explicitly distinguishes from completeness: "An
  absent Dependencies section is a completeness concern only if
  the ticket type typically requires dependencies; this lens
  flags couplings that are implied by the ticket content but
  not captured, whether the section exists or not."
- Closing "Remember:" paragraph.

### Success Criteria

#### Automated Verification:

- [ ] `skills/review/lenses/dependency-lens/SKILL.md` exists
      and `bash scripts/test-lens-structure.sh dependency-lens`
      passes.
- [ ] `skills/review/lenses/dependency-lens/evals/evals.json`
      exists and parses as JSON.
- [ ] `skills/review/lenses/dependency-lens/evals/benchmark.json`
      exists and records pass outcomes for all eight evals under
      the `with_skill` configuration, with three runs per eval
      per configuration.
- [ ] All eight fixture tickets exist at
      `skills/review/lenses/dependency-lens/evals/files/<name>/ticket.md`.
- [ ] `mise run test` passes.
- [ ] `bash scripts/test-config.sh` — no regression (ticket
      catalogue still emits five rows; ticket-mode golden
      fixture unchanged; pr and plan fixtures unchanged).

#### Manual Verification:

- [ ] Open `dependency-lens/SKILL.md`. Section headings and
      ordering match `scope-lens/SKILL.md` and
      `completeness-lens/SKILL.md`.
- [ ] `## What NOT to Do` names the other four ticket lenses and
      explicitly distinguishes the lens from completeness.
- [ ] Re-run all eight evals via skill-creator and confirm each
      produces a finding matching the `expected_output`
      description.
- [ ] On the two baseline fixtures (`clean-dependencies`,
      `standalone-chore-no-deps`), confirm zero `critical` or
      `major` findings across all three runs.

---

## Phase 5D: Cross-Lens Regression, Boundary Evals, and Orchestrator Smoke Test

### Overview

Close the loop on the orthogonality claim by making the
scope-boundary invariant a recurring test — not a one-shot
markdown artefact. Concretely: update the `(Phase 5)` forward
references in the three existing lens SKILL.md files,
**tighten completeness's Core Responsibility 3 to defer
implied-coupling reasoning to the `dependency` lens**, add a
new `boundary_evals.json` suite per peer lens encoding
negative-output expectations (e.g., completeness × a scope
fixture should *not* produce a bundling finding), run an
end-to-end smoke test with all five lenses, and capture a
narrative record of the smoke-test outcome.

The cross-lens regression check is the scope-boundary invariant
made concrete: `completeness`/`testability`/`clarity` should not
produce findings in the other lenses' domains when run against
the other lenses' fixtures, and vice versa (`scope`/`dependency`
should not produce out-of-domain findings against Phase 4
fixtures). Existing evals stay committed; what this sub-phase
verifies is that the partition holds after the ecosystem grew
from three lenses to five, and that it remains enforced on
every future prose edit.

### Changes Required

#### 1. Remove "(Phase 5)" forward-reference suffixes

**Files** (anchor: the phrase `are the scope and dependencies
lenses (Phase 5)` in each; actual line numbers vary between
~113-118 and may drift — match by content, not line number):

- `skills/review/lenses/completeness-lens/SKILL.md`
- `skills/review/lenses/testability-lens/SKILL.md`
- `skills/review/lenses/clarity-lens/SKILL.md`

**Change**: In each file, replace the suffix
`are the scope and dependencies lenses (Phase 5)` with
`are the scope and dependency lenses` (note both: drop the
`(Phase 5)` suffix AND change `dependencies` to `dependency`
to match the final lens identifier). The surrounding sentence
is otherwise unchanged.

#### 2. Narrow completeness's Core Responsibility 3

**File**: `skills/review/lenses/completeness-lens/SKILL.md`
**Changes**: Amend Core Responsibility 3 (anchor: the numbered
item beginning `3. Note missing assumptions or dependencies`)
to explicitly defer implied-coupling reasoning to the
`dependency` lens. Replace the current responsibility body
with:

```markdown
3. Note missing *sections* whose absence makes the ticket
   under-specified for its type (e.g., a story with no Context
   section, an epic with no Stories list). Do not reason about
   *content* within a present section — implied-but-uncaptured
   couplings (blockers, consumers, external systems, ordering)
   are the `dependency` lens's domain; unmeasurable criteria
   within a present Acceptance Criteria section are the
   `testability` lens's domain.
```

Re-run the completeness-lens evals after this change and
confirm all original evals still pass (see Success Criteria).
If an eval now fails because it relied on completeness flagging
implied-coupling content, move that failure mode into a
`dependency-lens` eval and amend the completeness eval's
`expected_output` to reflect the narrowed responsibility.

#### 3. Boundary evals (recurring, not one-shot)

Add a new eval suite per peer lens encoding *negative-output*
expectations — i.e., what findings the lens should *not*
produce. Each suite is a supplementary `boundary_evals.json`
file sitting alongside the existing `evals.json`, with the
same JSON shape but `expected_output` phrased as a negative
assertion. Runs are automated via the same skill-creator eval
harness and participate in `mise run test`.

**File**: `skills/review/lenses/completeness-lens/evals/boundary_evals.json`
**Contents** (two entries):

1. completeness × `scope-lens/evals/files/epic-bundling-unrelated-themes/ticket.md`
   — `expected_output`: "The reviewer produces no finding whose
   body mentions bundling, sizing, or that the ticket should be
   split into separate epics. Acceptable findings: absent-section
   observations only (e.g., missing Summary). No finding at
   `major` or higher about scope."
2. completeness × `dependency-lens/evals/files/implied-external-dep-missing/ticket.md`
   — `expected_output`: "The reviewer produces no finding whose
   body alleges implied Stripe or secrets-management couplings;
   it may flag the empty Dependencies section as under-specified
   for the ticket type, but must not reason about which couplings
   are missing from the content."

**File**: `skills/review/lenses/testability-lens/evals/boundary_evals.json`
**Contents** (two entries):

1. testability × `scope-lens/evals/files/spike-unbounded-exploration/ticket.md`
   — `expected_output`: "The reviewer produces no finding whose
   body describes the research question as too broad, unbounded,
   or impossible to bound — testability only flags unmeasurable
   exit criteria. The spike's exit-criteria text `have a good
   grasp of the system` may be flagged as unmeasurable; the
   breadth of the underlying question is off-lens."
2. testability × `dependency-lens/evals/files/epic-unordered-children/ticket.md`
   — `expected_output`: "The reviewer produces no finding about
   ordering or dependencies between child stories; it evaluates
   measurability of the epic's acceptance criteria only."

**File**: `skills/review/lenses/clarity-lens/evals/boundary_evals.json`
**Contents** (two entries):

1. clarity × `scope-lens/evals/files/story-mixing-feature-and-refactor/ticket.md`
   — `expected_output`: "The reviewer produces no finding whose
   body alleges the ticket mixes two units of work. Clarity
   evaluates ambiguity and contradiction; bundling is
   off-lens."
2. clarity × `dependency-lens/evals/files/vendor-dep-not-captured/ticket.md`
   — `expected_output`: "The reviewer produces no finding about
   missing external-vendor dependencies. Clarity may flag the
   bug body as ambiguously phrased, but must not reason about
   the presence or absence of captured vendor couplings."

**File**: `skills/review/lenses/scope-lens/evals/boundary_evals.json`
**Contents** (two entries):

1. scope × `completeness-lens/evals/files/<a Phase 4 fixture>/ticket.md`
   — `expected_output`: "The reviewer produces no finding about
   missing sections or empty-section content. Scope evaluates
   shape, not section presence."
2. scope × `testability-lens/evals/files/<a Phase 4 fixture>/ticket.md`
   — `expected_output`: "The reviewer produces no finding about
   measurability of acceptance criteria. Scope evaluates the
   ticket as a unit of work, not the quality of individual
   criteria."

(Exact Phase 4 fixture names to be chosen during
implementation; any fixture whose targeted failure mode is
sections-or-measurability will serve.)

**File**: `skills/review/lenses/dependency-lens/evals/boundary_evals.json`
**Contents** (two entries, mirroring scope):

1. dependency × a completeness fixture — `expected_output`:
   "The reviewer produces no finding about absent sections per
   se; it flags uncaptured couplings only."
2. dependency × a testability fixture — `expected_output`:
   "The reviewer produces no finding about measurability of
   acceptance criteria."

Each `boundary_evals.json` is loaded by the same skill-creator
eval mechanism as `evals.json`; results are recorded in a
supplementary `boundary_benchmark.json` committed alongside
`benchmark.json`. Boundary evals must pass at 100% for all
five lenses for Phase 5D to close.

#### 4. Narrative cross-lens boundary record (relocated)

**File**: `meta/research/2026-04-24-cross-lens-boundary-check.md`
(relocated from `skills/tickets/review-ticket/evals/` per the
standards review — `evals/` is reserved for the canonical
`evals.json` + `benchmark.json` + `files/` structure; narrative
one-shot records belong in `meta/research/`).

**Contents**: Markdown file with the following structure:

```markdown
---
date: "<ISO>"
type: research
skill: review-plan (Phase 5D)
status: complete
---

# Cross-Lens Boundary Check — Phase 5D

## Purpose

Record, per (lens × fixture) pair, the observed findings from
the Phase 5D boundary-eval run and confirm that every pair
respected the Phase 5-extended scope-boundaries table.

## Observed Outcomes

| Lens         | Fixture                               | Observed findings                 | Respected boundary? |
|--------------|---------------------------------------|-----------------------------------|---------------------|
| completeness | epic-bundling-unrelated-themes         | <observed>                        | yes / no            |
| completeness | implied-external-dep-missing           | <observed>                        | yes / no            |
| testability  | spike-unbounded-exploration            | <observed>                        | yes / no            |
| testability  | epic-unordered-children                | <observed>                        | yes / no            |
| clarity      | story-mixing-feature-and-refactor      | <observed>                        | yes / no            |
| clarity      | vendor-dep-not-captured                | <observed>                        | yes / no            |
| scope        | <Phase 4 completeness fixture>         | <observed>                        | yes / no            |
| scope        | <Phase 4 testability fixture>          | <observed>                        | yes / no            |
| dependency   | <Phase 4 completeness fixture>         | <observed>                        | yes / no            |
| dependency   | <Phase 4 testability fixture>          | <observed>                        | yes / no            |

## Assessment

<1-2 paragraphs summarising whether the scope-boundary invariant
held across all 10 (lens × fixture) pairs. If any pair failed,
file it as a bug against the offending lens and track here.>
```

This narrative record is supplementary to the `boundary_evals.json`
files — the evals are the load-bearing regression guard; this
file is a human-readable record for plan reviewers.

#### 5. End-to-end smoke test

Manual, non-automated. Run `/review-ticket` against a real
ticket in `meta/tickets/` (chosen for richness — e.g., an epic
with Dependencies content). Verify:

- Step 2 output enumerates five lenses.
- Step 3 spawns five reviewer agents in parallel (observable via
  streamed tool activity).
- Step 4 aggregates findings from all five lenses; the review
  summary contains per-lens results for all five.
- The persisted review artifact at
  `meta/reviews/tickets/<ticket-stem>-review-N.md` has
  `lenses: [clarity, completeness, dependency, scope, testability]`
  in its frontmatter (alphabetical order matches the catalogue).
- Re-review (Step 7) re-runs only the lenses that had findings;
  if scope or dependency had no findings, they are not re-run.

**Automated post-smoke-test assertion**: after the manual run,
execute:

```bash
NEWEST_REVIEW=$(ls -t meta/reviews/tickets/*-review-*.md | head -1)
grep -qE 'lenses: \[.*\bscope\b.*\]' "$NEWEST_REVIEW" \
  && grep -qE 'lenses: \[.*\bdependency\b.*\]' "$NEWEST_REVIEW" \
  && grep -qE 'lenses: \[.*\bcompleteness\b.*\]' "$NEWEST_REVIEW" \
  && grep -qE 'lenses: \[.*\btestability\b.*\]' "$NEWEST_REVIEW" \
  && grep -qE 'lenses: \[.*\bclarity\b.*\]' "$NEWEST_REVIEW"
```

This closes the smoke-test's frontmatter verification as an
automated check rather than manual inspection.

### Success Criteria

#### Automated Verification:

- [ ] Re-run `skills/review/lenses/completeness-lens/evals/` —
      all five original evals pass. Compare against Phase 4
      `benchmark.json`: pass_rate must equal the Phase 4
      pass_rate; the per-finding severity distribution
      (count at each severity) must match within ±1 per
      severity bucket.
- [ ] Re-run `skills/review/lenses/testability-lens/evals/` —
      same comparison criteria.
- [ ] Re-run `skills/review/lenses/clarity-lens/evals/` — same
      comparison criteria.
- [ ] Re-run `skills/review/lenses/scope-lens/evals/` — all nine
      evals continue to pass (sanity check after the prose
      adjustments to peer lenses).
- [ ] Re-run `skills/review/lenses/dependency-lens/evals/` —
      all eight evals continue to pass.
- [ ] All five lenses' `boundary_evals.json` suites pass at
      100% (recurring negative-output checks).
- [ ] `boundary_benchmark.json` committed for each of the five
      lenses.
- [ ] `mise run test` passes (now includes boundary evals).
- [ ] Phase-5-suffix and lens-name check:
      `grep -nE '\(Phase 5\)|\bdependencies\b' \
      skills/review/lenses/completeness-lens/SKILL.md \
      skills/review/lenses/testability-lens/SKILL.md \
      skills/review/lenses/clarity-lens/SKILL.md`
      returns zero matches. The check is paired because the
      `(Phase 5)` removal and the `dependencies` → `dependency`
      rename both land in the same sentence; a weak check
      (grep for `(Phase 5)` alone) would pass if only the
      suffix was removed.
- [ ] Automated frontmatter assertion on the smoke-test
      review artefact (see End-to-end smoke test, block above).

#### Manual Verification:

- [ ] Open each of the three updated lens SKILL.md files and
      confirm the "(Phase 5)" suffix is removed, `dependencies`
      has been renamed to `dependency`, and the sentence now
      reads as a concrete peer reference.
- [ ] Open `completeness-lens/SKILL.md` and confirm Core
      Responsibility 3 has been narrowed per Changes #2.
- [ ] Open `meta/research/2026-04-24-cross-lens-boundary-check.md`
      and confirm every row's "Respected boundary?" column is
      `yes`. Any `no` row is a bug filed against the offending
      lens.
- [ ] End-to-end smoke test: `/review-ticket` produces a review
      with five per-lens sections; the persisted artifact lists
      five lenses in frontmatter.
- [ ] After the smoke test, inspect the aggregated review output
      and confirm: (a) scope findings appear only under the Scope
      section, (b) dependency findings appear only under the
      Dependency section, (c) cross-cutting themes (if any) are
      attributed to the correct originating lenses.

---

## Testing Strategy

### Shell-script tests

Extensions to `scripts/test-config.sh` under the existing
`=== config-read-review.sh ticket mode ===` section, using the
`assert_eq` helper from `scripts/test-helpers.sh` (no
`assert_contains` exists; use `grep -q` inside an
`assert_eq "..." "true" "<grep check>"` idiom where needed).
Tests are run via `bash scripts/test-config.sh` during 5A and at
each phase boundary, and via `mise run test` at each phase
boundary.

A new committed fixture
`scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`
captures the post-5A ticket-mode output byte-for-byte and is
asserted against at every subsequent phase boundary. The pr/plan
golden-fixture invariants from Phase 4 continue to hold across
5A-5D; if no committed pr/plan fixtures exist, Phase 5A captures
them alongside the ticket fixture at
`scripts/test-fixtures/config-read-review/pr-mode-golden.txt`
and `.../plan-mode-golden.txt` so the invariant is checkable
against a stable baseline (not just tip-of-branch diff).

### SKILL.md evals

Each new lens (`scope`, `dependency`) is authored via
`/skill-creator:skill-creator` after the eval scenarios are
written. Evals are committed as checked-in fixtures alongside
each skill so they serve as long-term regression protection:

- `skills/review/lenses/scope-lens/evals/`
- `skills/review/lenses/dependency-lens/evals/`

Each eval is run three times per configuration (with_skill,
without_skill) to surface non-deterministic failure modes, per
the recommendation recorded in the Phase 4
`completeness-lens/evals/benchmark.json`.

Phase 5D additionally re-runs the three Phase 4 lens eval
suites to confirm no regression after the ecosystem grew from
three lenses to five, with a tightened comparison criterion:
per-finding severity distribution must match Phase 4 within ±1
per severity bucket (not just `pass_rate ≥ Phase 4`).

### Cross-lens boundary evals (recurring)

Phase 5D introduces five supplementary eval suites — one per
lens — at `skills/review/lenses/<lens>-lens/evals/boundary_evals.json`.
Each records negative-output expectations (e.g., "completeness
must not produce a bundling finding against this fixture") and
is loaded by the same skill-creator harness as `evals.json`.
Results are committed as `boundary_benchmark.json`. Boundary
evals participate in `mise run test` and therefore guard the
scope-boundary invariant on every future prose edit — they are
a *recurring regression suite*, not a one-shot milestone.

A narrative summary of the 5D boundary-eval run is also
captured in `meta/research/2026-04-24-cross-lens-boundary-check.md`
for plan-reviewer convenience (relocated from the
`skills/tickets/review-ticket/evals/` path in earlier plan
drafts — `evals/` is reserved for the canonical evals structure).

### Lens SKILL.md structural lint

Phase 5B introduces `scripts/test-lens-structure.sh`, wired
into `mise run test`. It asserts the six-section shape, four
frontmatter fields, persona sentence, peer-lens references, and
closing `Remember:` paragraph on every
`skills/review/lenses/*-lens/SKILL.md`. This provides
mechanical structural conformance checks for all current and
future lens files — the three existing Phase 4 lenses are also
linted.

### End-to-end smoke test

One manual smoke test in 5D, running `/review-ticket` against a
real ticket with all five lenses active. Requires a live Claude
Code session, but the post-run frontmatter assertion is
automated via the grep chain documented in Phase 5D Changes #5.
Verifies parallel five-agent execution, artefact persistence
with all five lens names in frontmatter, and correct aggregation
across all five lenses.

### Manual Testing Steps

1. **After 5A**: run `bash scripts/test-config.sh` — confirm
   all ticket-mode assertions pass (row count, sorted set,
   cross-mode isolation, golden fixture, info-note assertion).
   Open `review-ticket/SKILL.md` — confirm the Available Review
   Lenses table has five rows in alphabetical order; Step 2
   describes the default as count-neutral ("every lens
   registered in `BUILTIN_TICKET_LENSES`"). Open `CHANGELOG.md`
   — confirm the Unreleased entry reads "Five-lens ticket
   review capability".
2. **After 5B**: open `scope-lens/SKILL.md` and confirm
   `bash scripts/test-lens-structure.sh scope-lens` passes.
   Inspect `scope-lens/evals/benchmark.json` for a clean
   with-skill run across three runs per eval.
3. **After 5C**: open `dependency-lens/SKILL.md` and confirm
   the structural lint passes. Inspect `benchmark.json`.
4. **After 5D**: run the end-to-end smoke test described above.
   Confirm the three updated lens SKILL.md files no longer
   contain "(Phase 5)" and all references to the lens identifier
   use `dependency` (singular). Inspect
   `meta/research/2026-04-24-cross-lens-boundary-check.md` and
   confirm every row's "Respected boundary?" column is `yes`.

## Performance Considerations

- `/review-ticket` now spawns five reviewer agents in parallel
  instead of three. The reviewer agent context is small
  (ticket + lens SKILL.md + output format SKILL.md — no codebase
  exploration). Parallel fan-out cost is dominated by the
  longest-running agent, so the wall-clock increase is minor;
  token cost scales roughly linearly with the number of lenses.
- No additional config reads, no additional script invocations.
  Each lens is loaded by its owning reviewer agent only.

## Migration Notes

- No data migration. Existing tickets are not re-reviewed
  automatically, and existing review artifacts
  (`meta/reviews/tickets/*.md` with
  `lenses: [completeness, testability, clarity]` in frontmatter)
  are not rewritten.
- Users with `review.core_lenses` set to a subset of the
  built-in ticket lenses will pick up the two new lenses on next
  `/review-ticket` invocation. Per the orchestrator's Step 2
  resolution, `core_lenses` is the *minimum required set*, and
  remaining non-disabled built-in lenses are added up to
  `max_lenses` (default 8). To preserve the prior behaviour
  exactly, users should either add `scope` and `dependency` to
  `disabled_lenses`, or set `max_lenses` to the size of their
  chosen subset so no augmentation can occur.
- Users with no `review.core_lenses` configured will
  automatically pick up the two new lenses on next
  `/review-ticket` invocation, because the default resolution at
  `config-read-review.sh:412-413` uses all built-in ticket
  lenses. To opt out, add the new lens names to `disabled_lenses`.
- Phase 5A emits a one-time informational note from
  `config-read-review.sh ticket` when `core_lenses` is set and
  does not include every non-disabled built-in, pointing users
  to the opt-in/opt-out levers above. See Phase 5A Changes #5.
- Existing review artefacts in `meta/reviews/tickets/` have
  `lenses: [completeness, testability, clarity]` in their
  frontmatter. Step 7 of `review-ticket` re-runs only the lenses
  that produced findings in the previous pass, so re-reviews of
  pre-Phase-5 artefacts will not evaluate `scope` or
  `dependency`. To include the new lenses on a previously
  reviewed ticket, delete the existing review artefact to force
  a fresh Pass 1. This limitation is documented behaviour, not a
  bug — amending Step 7 to auto-include newly introduced
  built-ins is out of scope for Phase 5.
- No plugin version bump is required for this phase on its own;
  version bumps are handled separately per the repo's release
  process. However, Phase 5A updates the CHANGELOG Unreleased
  section in-place (amending the existing three-lens entry) so
  release notes capture the change regardless of when the bump
  lands. See Phase 5A Changes #6.

## References

- Research document:
  `meta/research/2026-04-08-ticket-management-skills.md`
  (Phase 5 section).
- Prior phase plan (direct dependency):
  `meta/plans/2026-04-22-ticket-review-core.md` — especially
  sub-phases 4C (completeness), 4D (testability and clarity),
  and the lens scope boundaries table (anchor: the `Lens Scope
  Boundaries` heading).
- Plan review informing this revision:
  `meta/reviews/plans/2026-04-24-ticket-review-extended-lenses-review-1.md`.
- Primary structural model for lens SKILL.md files:
  `skills/review/lenses/completeness-lens/SKILL.md`,
  `skills/review/lenses/testability-lens/SKILL.md`,
  `skills/review/lenses/clarity-lens/SKILL.md`.
- Output format:
  `skills/review/output-formats/ticket-review-output-format/SKILL.md`.
- Orchestrator: `skills/tickets/review-ticket/SKILL.md`.
- Config script (registration point):
  `scripts/config-read-review.sh` — anchor: the
  `BUILTIN_TICKET_LENSES` array definition.
- Test harness: `scripts/test-config.sh` — anchor: the ticket-mode
  catalogue-assertion block beginning `echo "Test: ticket mode
  catalogue"`.
- Test helpers: `scripts/test-helpers.sh` — `assert_eq` and
  `assert_exit_code` (no `assert_contains`; use `grep -q`).
- Eval format reference:
  `skills/review/lenses/completeness-lens/evals/evals.json`,
  `skills/review/lenses/completeness-lens/evals/benchmark.json`.
- Skill-creator skill:
  `~/.claude/plugins/cache/claude-plugins-official/skill-creator/unknown/skills/skill-creator/SKILL.md`.
- Reviewer agent: `agents/reviewer.md`.
- CHANGELOG: `CHANGELOG.md` (Unreleased section).

---
type: plan
id: "2026-06-23-0120-prevention-tests-agent-invocation-path"
title: "Prevention Tests for the Agent-Invocation Path Implementation Plan"
date: "2026-06-23T08:46:31+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0120"
parent: "work-item:0120"
derived_from: ["codebase-research:2026-06-23-0120-prevention-tests-agent-invocation-path"]
relates_to: ["work-item:0115", "work-item:0116", "work-item:0118"]
tags: [migrate, interactive-migration, agent-invocation, testing, 0007]
revision: "794b453cdb78aa98360c723799ef1f313ee754c5"
repository: "accelerator"
last_updated: "2026-06-23T09:17:20+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Prevention Tests for the Agent-Invocation Path Implementation Plan

## Overview

Close the two test gaps from incident 0115 (interactive migrations
unsatisfiable under agent invocation) with **regression tests only** — no
production-code change. Two disjoint test areas, in two independently
mergeable phases:

1. **AC2 (the genuine gap)** — add an end-to-end cross-check in the 0007 suite
   that a tolerant `unknown` backfill is a state the corpus validator accepts,
   driven by the *incident-shaped* fixture nothing currently tests: a
   `pr-description` under `meta/prs/` whose filename carries an external tracker
   key (`<TRACKER>-NNNN-description.md`) so `pr_number` is underivable.
2. **AC1 (already covered — harden + relabel)** — the no-input structured-stall
   test already exists (`test-migrate-interactive.sh:1195-1215`, shipped by
   0116) and satisfies every AC1 clause. Harden it (a distinctive pinned key in
   place of the short `k1`) and relabel it as the explicit
   *agent-invocation-path* regression tracing to this incident. No near-duplicate
   test.

## Current State Analysis

The research (`meta/research/codebase/2026-06-23-0120-prevention-tests-agent-invocation-path.md`)
established — and direct re-verification against revision `794b453` confirmed —
that 0116 and 0118 each shipped tests for their own behaviour while 0120 was
still an open blocker. As a result the work item's AC1 premise ("the no-input
branch … is never exercised") is now **stale**, and only AC2 names a fixture
shape that no existing test uses.

**AC1 — already implemented.** `test-migrate-interactive.sh:1195-1215`
(section `=== Structured stall on no decision input (0116 Phase 2) ===`) drives
the driver with `</dev/null`, no decisions file, `ACCELERATOR_MIGRATE_FORCE=1`,
and asserts every AC1 clause:

- non-zero exit (`:1204`);
- (a) the literal pending key `k1` (`:1206`);
- (b) the `--decisions-file` switch (`:1207`), `run-migrations.sh` driver
  (`:1208`), `ACCELERATOR_MIGRATE_DECISIONS_FILE=` env form (`:1209-1210`), and
  the migration id `0002-predicate` in the resume path (`:1211`);
- (c) absence of `failed to obtain decision` (`:1212`);
- plus a guard against `unbound variable` shell crashes (`:1215`).

A companion test (`:1218-1239`) covers the VALIDATE_ERR re-prompt no-input
stall. The pending key is `fields[0]` of the first PROMPT frame and is pinned by
`seed_predicate_sandbox "$SBX" "k1|…|ambiguous|…"` (`:1199`), so it is knowable
in advance — `k1` already qualifies as "a key the fixture fixes," but it is a
short, non-distinctive token.

**AC2 — genuine gap.** The 0007 suite proves the `unknown` sentinel end-to-end,
but only for `pr-review` fixtures with date-only / numberless stems:

- NODEFAULT `meta/reviews/prs/2026-06-20-dateonly-pr-review.md` → `run_0007`
  full run → `Phase 4 corpus exits 0` (`:1254`) →
  `pr_number: unknown` (`:1288-1290`) → `assert_validates` clean (`:1323`);
- direct-run breadcrumb `meta/reviews/prs/no-pr-number-review.md` (`:1339-1381`).

No fixture is a **`pr-description`** (type inferred from `meta/prs/` directly,
longest-dir-wins) whose stem is a tracker key — the exact incident shape.

**Backfill / validator contract (verified):**

- `extra_default()` `pr_number` arm (`0007:201-217`): matches a genuine
  `pr`/`PR` segment (`(^|-)[Pp][Rr]-?[0-9]+`) or a leading numeric stem
  (excluding date-prefixed stems). A tracker-key stem like `ENG-1234-description`
  matches **neither** → returns empty.
- Backfill sentinel branch (`0007:514-523`): an underivable **required** extra
  is stamped `dv='unknown'` — bare for `pr_number`; the emission layer quotes
  string/enum required extras (e.g. `verdict`/`lenses` on `pr-review`) to
  `"unknown"`. Optional extras (`pr_url`/`merge_commit`, in `FM_OPTIONAL_EXTRAS`,
  `frontmatter-emission-rules.sh:74`) are skipped entirely (`0007:510`), never
  stamped — they remain absent, which the validator accepts.
- Validator: MISSING-EXTRA fires only on *absence* (`validate-corpus-frontmatter.sh:345`);
  EMPTY-PLACEHOLDER rejects only literal `""`/`[]` (`:354-355`). A present
  `unknown` (7 chars) is accepted by both.

**The `FAIL:.*MISSING-EXTRA` regex gotcha.** Violations print as
`<file>: MISSING-EXTRA — <msg>` (no `FAIL:` prefix, em-dash `—` U+2014); the only
`FAIL:` line is the codeless summary `FAIL: N frontmatter violation(s)`
(`:63-66`, `:433-436`). The literal regex matches **nothing on a single line**
and is vacuously satisfied. The meaningful assertion is *exit 0 + no
`MISSING-EXTRA` token + validator clean*.

**CI floor (AC3).** `tasks/test/integration.py:8,138-147` enforces only a
suite-*file* count floor (`_EXPECTED_MIGRATE_SUITES = 4`) over
`skills/config/migrate`; there is **no per-suite test-count floor**. Both target
suites are among the four. AC3's "floor that suite already asserts in CI"
resolves to "the suite still exists, executes, and exits 0" — automatically
satisfied by adding passing assertions to existing executable suites. No
bookkeeping needed.

## Desired End State

- The 0007 suite contains an incident-shaped `pr-description` fixture with a
  tracker-key stem proving the backfill→validator contract end-to-end, plus a
  derivable `pr-description` counter-fixture proving the sentinel is *not*
  applied where `pr_number` is derivable.
- The no-input stall test is relabelled as the agent-invocation-path regression
  and hardened with a distinctive pinned key.
- `bash skills/config/migrate/scripts/test-migrate-0007.sh` and
  `bash skills/config/migrate/scripts/test-migrate-interactive.sh` each exit 0
  with their (increased) assertion counts, and `mise run check` is green.

### Key Discoveries:

- AC1 is satisfied-by-0116; the value of 0120 concentrates in AC2's
  incident-shaped fixture (`research §Area 2`, `§Architecture Insights`).
- `pr_number` stamps **bare** `unknown`; required string/enum extras stamp
  quoted `"unknown"` (`0007:201-230`, `test-migrate-0007.sh:1290` vs
  `:1310-1311`). Optional extras are not stamped at all (see below).
- Type inference is longest-dir-wins: the AC2 fixture **must** live directly
  under `meta/prs/` (→ `pr-description`), not `meta/reviews/prs/` (→ `pr-review`)
  (`research §Area 3`).
- `pr-description` lists extras `pr_url pr_number merge_commit`
  (`templates-schema.tsv:5`), but `pr_url`/`merge_commit` are in
  `FM_OPTIONAL_EXTRAS` so the backfill skips them; only `pr_number` is genuinely
  required. The minimal incident fixture therefore stamps **only** `pr_number`
  (bare `unknown`), leaves the optional pair absent, and still validates clean.
- The literal `FAIL:.*MISSING-EXTRA` regex is vacuous — assert the meaningful
  equivalent (`research §Area 4`, Open Question 2).

## What We're NOT Doing

- **No production-code change.** Both 0116 (the stall) and 0118 (the sentinel)
  have landed; this is pure regression-test work.
- **No near-duplicate AC1 test.** We harden/relabel the existing one rather than
  author a second no-input test (decided).
- **No literal `FAIL:.*MISSING-EXTRA` regex** — it is vacuous; we assert the
  meaningful equivalent with an explanatory comment (decided).
- **No guarded-resume coverage** — owned by 0119 (out of scope per the work
  item).
- **No generalised standalone lint over all backfill/validator pairs** — rejected
  in the work item's Open Questions as scope-widening; a separate item if ever
  wanted.
- **No new suite file and no CI-floor edits** — assertions are added to existing
  suites; AC3 is satisfied structurally.

## Implementation Approach

Two phases, each touching exactly one test file, each green on its own and
independently mergeable. Phase 1 (AC2) carries the real regression value and
lands first; Phase 2 (AC1) is the hardening/relabel.

Because the guarded behaviour already ships, a literal red→green TDD cycle is
not available — these are **regression guards**. The TDD discipline we apply is:
write the assertion to the contract, then prove it actually bites by a temporary
local revert (see each phase's "TDD sanity check"), confirm it goes red, restore,
confirm green. The committed state is green; the revert is a throwaway local
check, never committed.

## Phase 1: AC2 — 0007 backfill↔validator cross-check (incident + counter fixture)

### Overview

Add the incident-shaped `pr-description` fixture and a derivable counter-fixture
to the existing Phase 4 corpus block in `test-migrate-0007.sh`, riding the same
`run_0007 "$P4"` full run and the corpus-wide `assert_validates "$P4/meta"`
gate, then add focused assertions. This mirrors the existing NODEFAULT/WIDENING
fixtures, which already live in `$P4`.

### Changes Required:

#### 1. Incident fixture — tracker-key `pr-description`

**File**: `skills/config/migrate/scripts/test-migrate-0007.sh`
**Changes**: In the Phase 4 corpus setup block, **alongside the existing
NODEFAULT/WIDENING heredocs and before `git_init "$P4"` / `run_0007 "$P4"`** (the
suite has drifted, so anchor on those structural landmarks rather than absolute
line numbers — the setup heredocs currently begin ~`:1169` and `run_0007 "$P4"`
is ~`:1253`), add a `meta/prs/` fixture whose stem is a tracker key so
`pr_number` is underivable. The fixtures MUST be created before the `run_0007`
invocation so they are migrated by the same run the assertions inspect. Use a
neutral project prefix (`ENG-1234`) rather than a real tracker id, matching the
neutral `<TRACKER>` convention used throughout the research and the work item's
AC text.

```bash
# INCIDENT (0120 AC2): a pr-description whose filename carries an external
# tracker key (ENG-1234-description). `pr_number` has no derivable default — the
# stem has no pr/PR segment and is not numeric-leading, so the leading-numeric
# fallback returns empty (the date-prefix exclusion is never even reached) — so
# the backfill stamps the bare `unknown` sentinel on this one required extra.
# This is the exact file shape from the 0115 incident; the cross-check proves
# what the tolerant backfill emits is a state the validator accepts.
mkdir -p "$P4/meta/prs"
cat >"$P4/meta/prs/ENG-1234-description.md" <<'EOF'
---
type: pr-description
id: "ENG-1234-description"
title: "Tracker Keyed PR Description"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Tracker Keyed PR Description
EOF
```

#### 2. Counter-fixture — derivable `pr-description`

**File**: `skills/config/migrate/scripts/test-migrate-0007.sh`
**Changes**: Add a sibling `meta/prs/` fixture whose stem *does* carry a `pr-`
segment, so `extra_default` derives the real number and the sentinel is **not**
applied (no-regression guard).

```bash
# COUNTER (0120 AC2): a derivable pr-description — `pr-42-description` matches
# the `(^|-)[Pp][Rr]-?[0-9]+` segment, so pr_number derives to 42 and is NOT
# sentinel-replaced. Guards the boundary: the sentinel fires only where the
# value is genuinely underivable.
cat >"$P4/meta/prs/pr-42-description.md" <<'EOF'
---
type: pr-description
id: "pr-42-description"
title: "Derivable PR Description"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Derivable PR Description
EOF
```

#### 3. Assertions — after the existing Phase 4 assertions (~`:1290-1298`)

**File**: `skills/config/migrate/scripts/test-migrate-0007.sh`
**Changes**: Add focused assertions after `run_0007 "$P4"` and before/around the
existing `assert_validates "Phase 4 corpus validates clean" "$P4/meta"` (`:1323`),
which already covers the corpus-wide "validator accepts" half (now including the
new fixtures).

```bash
# ── 0120 AC2: backfill↔validator cross-check (incident-shaped fixture) ──
INCIDENT="$P4/meta/prs/ENG-1234-description.md"
# Only pr_number is a genuinely REQUIRED extra for pr-description, so the
# underivable tracker-key stem gets the BARE `unknown` sentinel. pr_url and
# merge_commit are in FM_OPTIONAL_EXTRAS (frontmatter-emission-rules.sh:74), so
# the backfill loop skips them (0007:510) and they stay ABSENT — which the
# validator accepts (MISSING-EXTRA also skips optional extras). Assert both: the
# present bare sentinel on the required extra, and the benign absence of the
# optional pair (no quoted "unknown" is ever written for them).
assert_contains "Phase 4 INCIDENT: tracker-key pr_number -> bare unknown sentinel" \
  "$(fm_line "$INCIDENT" pr_number)" 'pr_number: unknown'
# Assert absence against the WHOLE migrated file (not fm_line, which returns
# empty for an absent key and would pass vacuously) so a stray quoted-`unknown`
# stamping anywhere in the frontmatter is caught — the `assert_not_contains
# "$(cat …)"` idiom already used at :164,:169-170.
assert_not_contains "Phase 4 INCIDENT: optional pr_url left absent (not stamped)" \
  "$(cat "$INCIDENT")" 'pr_url:'
assert_not_contains "Phase 4 INCIDENT: optional merge_commit left absent (not stamped)" \
  "$(cat "$INCIDENT")" 'merge_commit:'
# AC2 names the regex `FAIL:.*MISSING-EXTRA`, but it matches NO single validator
# line: violations print `<file>: MISSING-EXTRA — <msg>` (no FAIL: prefix) and
# the only FAIL: line is the codeless summary. So assert the MEANINGFUL
# equivalent: the validator emits no MISSING-EXTRA token over the migrated
# incident fixture, proving the present `unknown` is an accepted state rather
# than a tolerated-but-rejected one. Exit-0 acceptance of this fixture is
# already covered by the corpus-wide `assert_validates "$P4/meta"` below (which
# now includes it), so no separate per-file assert_validates is needed.
INCIDENT_VOUT="$("$VALIDATOR" "$INCIDENT" 2>&1)" || true
assert_not_contains "Phase 4 INCIDENT: no MISSING-EXTRA for present sentinel" \
  "$INCIDENT_VOUT" "MISSING-EXTRA"

# ── 0120 AC2 counter: derivable pr_number is NOT sentinel-replaced ──
# Guards the pr-description / meta/prs/ derivation path specifically — distinct
# from the existing PR430 block (:1264-1298), which proves the same boundary for
# a pr-review under meta/reviews/prs/. Exact-equality positive form (not a bare
# substring) so a regression that DROPPED the line entirely also fails rather
# than passing a vacuous not_contains.
COUNTER="$P4/meta/prs/pr-42-description.md"
assert_eq "Phase 4 COUNTER: derivable pr_number from pr- segment" \
  'pr_number: 42' "$(fm_line "$COUNTER" pr_number)"
assert_not_contains "Phase 4 COUNTER: derivable pr_number NOT sentinel-replaced" \
  "$(fm_line "$COUNTER" pr_number)" 'unknown'
```

Notes for the implementer:
- The corpus-wide `assert_validates "Phase 4 corpus validates clean" "$P4/meta"`
  (`:1323`) and `Phase 4 corpus exits 0` (`:1254`) already cover exit 0 and
  whole-corpus validation including the two new fixtures, so no separate
  per-fixture `assert_validates` is added; the AC-specific targeted form is the
  `INCIDENT_VOUT` no-`MISSING-EXTRA` token check (the meaningful equivalent of
  the AC's vacuous `FAIL:.*MISSING-EXTRA` regex).
- The idempotency check at `:1326-1329` (second `run_0007` → empty `meta/` diff)
  must remain green: on the second run `pr_number` is already present and
  non-empty (`unknown` for INCIDENT, `42` for COUNTER), so the underivable-
  required-extra branch (`0007:514-523`) is never re-entered and nothing is
  re-stamped. Verify it still passes.
- `$VALIDATOR`, `fm_line`, `assert_eq`, `assert_contains`,
  `assert_not_contains`, `assert_validates` are all already defined in the suite
  (`:16`, `:44`, `:61-74`, test-helpers).

### TDD sanity check (local, throwaway):

Temporarily comment out the `dv='unknown'` sentinel branch (`0007:514-523`) so
the underivable `pr_number` is left absent; re-run the suite and confirm it goes
**red**. Note the mechanism: with the sentinel gone, `pr_number` is absent and
0007's own `self_validate_structural` gate (`0007:567-579,784`) makes the
migration **abort mid-run** under `set -e`, so `Phase 4 corpus exits 0`
(`:1254`) is the assertion that fails *first* (the corpus is left
half-migrated) — this is exactly the abort the sentinel was added to prevent.
That suite-goes-red is the proof the guard bites; the per-fixture
`pr_number: unknown` / `INCIDENT_VOUT` assertions are downstream of the abort.
The COUNTER's `pr_number: 42` derives without ever touching the sentinel branch,
so it is unaffected by the revert. Restore the branch; confirm all green. Do not
commit the revert.

### Success Criteria:

#### Automated Verification:

- [ ] 0007 suite passes: `bash skills/config/migrate/scripts/test-migrate-0007.sh`
- [ ] Migrate integration suite passes: `mise run test:integration:migrate`
      (suite-file floor `_EXPECTED_MIGRATE_SUITES = 4` still met)
- [ ] Shell lint/format clean: `mise run lint:scripts:check` and
      `mise run format:scripts:check`
- [ ] Full read-only check green: `mise run check`

#### Manual Verification:

- [ ] Inspecting the migrated `meta/prs/ENG-1234-description.md` shows
      `pr_number: unknown` (bare) and **no** `pr_url:` / `merge_commit:` lines
      (optional extras, left absent — not stamped).
- [ ] The TDD sanity revert confirms the cross-check actually bites (goes red
      without the sentinel branch).

---

## Phase 2: AC1 — harden + relabel the no-input stall test

### Overview

The no-input structured-stall test already satisfies every AC1 clause. Relabel
it as the explicit agent-invocation-path regression tracing to incident 0115,
and replace the short `k1` pinned key with a distinctive token so AC1(a)'s
"a key the fixture fixes and knows in advance (not merely some non-empty token)"
is unambiguous. No new test.

### Changes Required:

#### 1. Relabel + distinctive key

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: At the no-input stall test (`:1192-1215`), (a) extend the section
banner / add a comment naming this as the agent-invocation-path regression and
citing incident 0115/0120; (b) seed a distinctive PROMPT key instead of `k1` and
update the key assertion to match.

```bash
echo "=== Structured stall on no decision input (0116 Phase 2; 0120 AC1) ==="
echo ""

# 0120 AC1 — the AGENT-INVOCATION PATH regression. This is the real
# skill-invocation shape from incident 0115: no TTY, no decisions file, fd 0 at
# EOF (`</dev/null`), ACCELERATOR_MIGRATE_DECISIONS_FILE unset. It must reach the
# bare fd-0 branch of read_decision() (interactive-lib.sh:270-280, status 2) and
# emit the structured stall — NOT the legacy `failed to obtain decision` abort,
# and NOT via a supplied decisions file (which proved the wrong thing).
echo "Test: PROMPT no-input → structured stall (agent-invocation path)"
RC=0
SBX=$(setup_sandbox "stall-no-input")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
# Distinctive pinned key (not the short `k1`) so AC1(a)'s literal-substring
# assertion is unambiguous and self-documenting.
seed_predicate_sandbox "$SBX" "agent-invocation-pending-key|f1|a1|v1|ambiguous|prose1"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" </dev/null 2>&1) || RC=$?
assert_neq "non-zero exit on no-input stall" "0" "$RC"
assert_contains "stall marker present" "$OUTPUT" "MIGRATION STALLED"
assert_contains "names the current key" "$OUTPUT" "agent-invocation-pending-key"
# The resume-command strings below (--decisions-file switch, the
# ACCELERATOR_MIGRATE_DECISIONS_FILE= env form, run-migrations.sh as driver) are
# the output format owned by emit_no_input_stall (interactive-lib.sh:313-346).
# The work item flags 0119 (resume-safe partial-migration failure) as a possible
# future editor of this format — if these substrings break, check whether 0119
# changed the resume hint before assuming a regression here.
assert_contains "resume switch form" "$OUTPUT" "--decisions-file"
assert_contains "resume names the driver" "$OUTPUT" "run-migrations.sh"
assert_contains "resume env-var form" "$OUTPUT" \
  "ACCELERATOR_MIGRATE_DECISIONS_FILE="
assert_contains "migration id in resume path" "$OUTPUT" "0002-predicate"
assert_not_contains "old opaque message gone" "$OUTPUT" "failed to obtain decision"
assert_not_contains "no shell errors on stall path" "$OUTPUT" "unbound variable"
```

Notes for the implementer:
- The pinned key `agent-invocation-pending-key` is committed as written — no
  fallback. It is `fields[0]` of the seeded row, split on `|` by
  `seed_predicate_sandbox` (`:360-369`) and matched with the fixed-string
  `grep -qF` assertion helper, so the hyphens are inert end-to-end (verified).
  Keep this exact spelling everywhere (banner, comment, seed, assertion) so the
  key remains greppable.
- Leave the VALIDATE_ERR re-prompt companion test (`:1218-1239`) functionally
  intact; optionally give its seeded key the same distinctive treatment for
  consistency, but that is not required by AC1.
- `ACCELERATOR_MIGRATE_FORCE=1` stays: AC1/AC3 neither require nor forbid it, and
  it is faithful to the incident (dirty tree from prior migrations). AC3's
  preconditions — fd 0 from `/dev/null`, `ACCELERATOR_MIGRATE_DECISIONS_FILE`
  unset, no pseudo-TTY — all hold.

### TDD sanity check (local, throwaway):

Confirm the hardened assertion bites: temporarily change the seeded key (e.g. to
`other-key`) without updating the `names the current key` assertion and confirm
it goes red; restore. This proves the assertion tracks the *pinned* key rather
than any non-empty token.

### Success Criteria:

#### Automated Verification:

- [ ] Interactive suite passes:
      `bash skills/config/migrate/scripts/test-migrate-interactive.sh`
- [ ] Migrate integration suite passes: `mise run test:integration:migrate`
- [ ] Shell lint/format clean: `mise run lint:scripts:check` and
      `mise run format:scripts:check`
- [ ] Full read-only check green: `mise run check`

#### Manual Verification:

- [ ] The test output / section banner now reads as the agent-invocation-path
      regression and cites the incident, so a future reader traces it to 0115/0120.
- [ ] The pinned key in the stall output is the distinctive token, not `k1`.

---

## Testing Strategy

### Unit / suite Tests:

- **AC2**: incident `pr-description` (tracker key → bare `unknown` sentinel,
  validator clean, no `MISSING-EXTRA`) and counter `pr-description` (derivable
  `pr_number: 42`, not sentinel-replaced), both via the existing `run_0007`
  full-run path and corpus-wide `assert_validates`.
- **AC1**: the existing no-input stall test, relabelled and hardened with a
  distinctive pinned key, exercising the bare fd-0 status-2 branch.

### Key edge cases:

- Required vs optional extras (`pr_number` required → bare `unknown` sentinel;
  `pr_url`/`merge_commit` optional → skipped, left absent).
- Derivable boundary (`pr-42-description` derives; `ENG-1234-description` does not).
- Idempotency of the new fixtures (second `run_0007` → empty diff).

### Manual Testing Steps:

1. Run each suite directly and confirm exit 0 with increased assertion counts.
2. Inspect the migrated incident fixture's frontmatter for the sentinel forms.
3. Perform each phase's TDD sanity revert to confirm the new/hardened assertions
   genuinely bite.

## Performance Considerations

Negligible — two extra heredoc fixtures ride the existing single `run_0007 "$P4"`
invocation; no new migration runs. AC1 changes are label/seed-only.

## Migration Notes

None — no production code or schema changes; no data migration.

## References

- Original work item: `meta/work/0120-prevention-tests-for-agent-invocation-path.md`
- Research: `meta/research/codebase/2026-06-23-0120-prevention-tests-agent-invocation-path.md`
- Incident RCA: `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
- AC1 target (existing test): `skills/config/migrate/scripts/test-migrate-interactive.sh:1195-1215`
- AC2 host block: `skills/config/migrate/scripts/test-migrate-0007.sh:1169-1329`
- Backfill sentinel: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:201-230,514-523`
- Validator acceptance: `scripts/validate-corpus-frontmatter.sh:342-359`
- `pr-description` required extras: `scripts/templates-schema.tsv:5`
- CI suite-file floor: `tasks/test/integration.py:8,138-147`
- 0116 (stall) plan/validation; 0118 (sentinel) plan/validation — both done

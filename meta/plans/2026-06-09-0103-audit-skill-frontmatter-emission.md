---
type: plan
id: "2026-06-09-0103-audit-skill-frontmatter-emission"
title: "Audit Skill Frontmatter Emission Against the Unified Schema Implementation Plan"
date: "2026-06-09T19:05:20+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0103"
parent: "work-item:0103"
derived_from: ["codebase-research:2026-06-09-0103-skill-frontmatter-emission-audit"]
relates_to: ["work-item:0057", "work-item:0070"]
tags: [frontmatter, schema, skills, validation, audit, test-harness]
revision: "cae1eebdfc645a3ac158ee419b52b70c6ee1b780"
repository: "ticket-management"
last_updated: "2026-06-09T21:50:32+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Audit Skill Frontmatter Emission Against the Unified Schema Implementation Plan

## Overview

The 0070 migration unified the meta corpus and shipped a corpus validator
(`scripts/validate-corpus-frontmatter.sh`), a shared emission-rules helper
(`scripts/frontmatter-emission-rules.sh`), and per-type facts
(`scripts/templates-schema.tsv`) — but it validated only the *corpus*, never the
*producers* (the skills that write frontmatter). This plan closes that
producer-side gap: it (1) audits every frontmatter-emitting skill against the
full validator contract, (2) fixes the confirmed producer-text divergences, and
(3) adds an automated producer-conformance guard wired into CI so future skills
cannot drift undetected.

## Current State Analysis

**The contract is a deliberate three-file split** (all confirmed on disk):

- `scripts/templates-schema.tsv` — 13 per-type rows, 7 tab-separated columns:
  `template`, `type`, `code_state_anchored`, `extras`, `status_vocab`,
  `forbidden_own_id_key`, `typed_linkage_keys`.
- `scripts/frontmatter-emission-rules.sh` — cross-cutting rules with no per-type
  column: `FM_BASE_FIELDS` (`:26`), `FM_PROVENANCE_FIELDS` /
  `FM_FORBIDDEN_PROVENANCE_FIELDS` (`:29-30`), `FM_OPTIONAL_EXTRAS` (`:69`),
  `FM_TYPED_REF_RE` (`:83`), `FM_SOURCE_TYPE_RE` (`:36`),
  `fm_linkage_cardinality()` (`:47`), `fm_is_linkage_key()` (`:89`, currently
  unused — a ready-made helper for the guard).
- `scripts/validate-corpus-frontmatter.sh` — the 11-axis runtime gate, sourcing
  the helper (`:26-27`) and parsing the TSV into parallel arrays (`:41-54`,
  bash 3.2 has no associative arrays).

**The validator enforces 11 axes** (`validate-corpus-frontmatter.sh:271-376`):
required base fields, quoted `id`, bare-integer `schema_version`, ISO
`date`/`last_updated`, per-type `status` vocab, provenance-iff-anchored,
forbidden `git_commit`/`branch`, forbidden own-id key, required extras,
omit-when-empty, typed-linkage `"doc-type:id"` shape (+ dangling-ref in
whole-corpus mode).

**Two validator blind spots** (verified at source):

- **Provenance "iff" is one-directional** (`:314-324`): an anchored type missing
  the bundle fails, but a *non-anchored* type that wrongly emits
  `revision`/`repository` is **not** flagged.
- **Only quoted linkage tokens are shape-checked** (`:358`): the parser extracts
  `"…"` tokens; a bare `parent: 0042` yields no tokens and escapes
  `BAD-LINKAGE-SHAPE` entirely.

So "passes the validator" is **necessary but not sufficient** for full
conformance — the per-attribute audit must additionally cover these two axes by
inspection (per the agreed AC4-floor interpretation).

**Confirmed divergences already found:**

1. `skills/planning/validate-plan/SKILL.md:186-188` sets a passing **plan**'s
   `status` to the literal `complete`, outside the plan vocab
   `draft | ready | in-progress | done` (`templates-schema.tsv:3`). Its sibling
   `:161` correctly sets its own **plan-validation** report to `complete`
   (`templates-schema.tsv:4`). ADR-0042 maps plan `complete → done`. **Producer
   fix:** the single literal at `:187` → `done`.
2. `skills/decisions/review-adr/SKILL.md:85,194` documents `status: proposed →
   rejected`, but the `adr` vocab is `proposed | accepted | superseded |
   deprecated` (`templates-schema.tsv:6`) — `rejected` is **not** a member. This
   requires the schema-source-vs-producer triage (Phase 1): either `rejected` is
   a legitimate missing ADR state (→ **schema-source** divergence, raised as a
   child work item under 0057, **not** fixed here), or review-adr should not
   persist a `rejected` ADR (→ producer fix). It is **not** pre-judged in this
   plan.

**Emission is composed, not prose-only.** Several skills (validate-plan among
them) do not substitute every base field in SKILL.md prose — `tags` and the
provenance bundle come from the loaded template / `artifact-derive-metadata.sh`.
The audit and guard must evaluate the *composed* emission (skill literals +
template-supplied fields + metadata helper), or they false-positive on "missing"
fields the template supplies.

**Test wiring** (`tasks/test/integration.py`, `tasks/test/helpers.py`):
`test:integration:config` → `run_shell_suites(context, "scripts")` glob-discovers
executable `scripts/test-*.sh` (`helpers.py:29-35`, exec-bit gate at `:34`,
excludes `test-helpers.sh`), floored at `_EXPECTED_CONFIG_SUITES = 15`
(`integration.py:14,42-49`). Adding a suite requires bumping that floor to 16.

### Key Discoveries:

- The structural precedent for the guard is **`test-validate-corpus-frontmatter.sh`**
  (synthesize fixture in tmpdir → run validator → assert rc/code:
  `emit_valid` `:31-65`, `run_validator` `:68-71`, `trap … EXIT` `:23`), **not**
  the work-item-cited `test-skill-frontmatter-population.sh`, which is a static
  prose checker under `test:unit:templates` that never drives a skill or runs
  the validator (`test-skill-frontmatter-population.sh:80-168`).
- **Skills cannot be deterministically shell-driven** (they are LLM prose). The
  guard verifies the *verbatim literals* a SKILL.md hard-codes (`type`,
  `status`, `producer`, `schema_version`, literal extras), synthesizes a fixture
  around them, and runs the real validator. This is the work item's
  "documented-emission mode … verbatim literal in SKILL.md, so that corrupting
  the literal still trips the negative test."
- `skills-schema.tsv` maps skill → *fields-to-assert* but **not** skill → *type*;
  its population test self-checks `NF == 4` (`test-skill-frontmatter-population.sh:62`),
  so adding a column there would couple this work to that test. The guard derives
  skill → type by extracting the `type:` literal from each SKILL.md instead.
- The population test's "discovery pass" pattern
  (`test-skill-frontmatter-population.sh:242-281`: grep producing-skill markers,
  `comm -23` against an allowlist) is the precedent for keeping the producer set
  enumerated and drift-proof inside the guard.
- `test-validate-corpus-frontmatter.sh:205-235` already demonstrates the
  single-source guard pattern (tamper a copy of the shared helper, point both
  surfaces at it via `FM_EMISSION_RULES=`, assert behaviour flips) — reusable for
  the new guard's liveness self-test.

## Desired End State

- A re-runnable discovery procedure enumerates the producing-skill set, each
  listed with the type(s) it emits.
- A per-(skill, type) conformance table maps every emitted attribute to the
  validator rule it satisfies (or the fix applied), complete when its attribute
  set per type equals the validator-enforced set for that type, plus the two
  blind-spot axes by inspection.
- `validate-plan` sets a passing plan's status to `done` and its own validation
  report's status to `complete`.
- Every confirmed producer-text divergence is fixed; every schema-source
  divergence is raised as a child work item under 0057.
- An automated producer-conformance guard exists, is discovered by
  `test:integration:config`, passes green on the audited set, and **fails when a
  skill is made to emit an out-of-contract attribute**.
- `mise run test:integration:config` stays green.

## What We're NOT Doing

- **Not** fixing schema-source divergences (in `templates-schema.tsv` or
  `frontmatter-emission-rules.sh`) under this work item — those are recorded and
  raised as child work items under epic 0057 (e.g. the ADR `rejected` question).
- **Not** auditing pure field-mutators (`update-work-item` — arbitrary field
  edits, no fixed schema-governed block), consumers (`list-work-items`), or the
  corpus transformer (`config/migrate`, governed differently). Status-transition
  mutators **are** in scope on the status axis only: `validate-plan`→plan,
  `review-adr`→adr.
- **Not** driving skills through a live LLM in the test (impossible/non-
  deterministic); the guard asserts documented literals + composed emission.
- **Not** widening the validator to close its two blind spots — those axes are
  covered by inspection in the audit table and by dedicated assertions in the
  guard, not by changing the shared oracle.
- **Not** re-encoding the contract in the new test — it sources
  `templates-schema.tsv` and `frontmatter-emission-rules.sh`.

## Implementation Approach

Three phases in a linear dependency chain, each leaving the tree green and
mergeable as its own PR:

1. **Audit (docs):** enumerate producers, build the conformance table, triage
   findings into producer-text fixes (Phase 2) vs schema-source child work items.
2. **Fix (TDD via the existing validator):** correct the confirmed producer-text
   divergences, proving each red→green against `validate-corpus-frontmatter.sh`.
3. **Guard (CI):** the permanent automated conformance test, landing green
   because Phase 2 already fixed the bug; its negative self-test proves wiring.

TDD note: for Phase 2 the existing `validate-corpus-frontmatter.sh` *is* the test
oracle (it already rejects a plan carrying `status: complete` with `BAD-STATUS`),
so the fix is driven red→green without new infrastructure. The permanent guard
(Phase 3) carries its own count-gated liveness and negative self-tests.

Mergeability note: Phase 3 must follow Phase 2 in the merge order — a guard that
correctly rejects the still-unfixed `validate-plan` bug would be red, so the fix
lands first.

## Phase 1: Producer Enumeration and Per-Attribute Conformance Audit

### Overview

Produce the auditable artifact AC1 and AC2 require: the re-runnable discovery
procedure, the producer set with emitted type(s), and the per-(skill, type)
conformance table. No code changes. Triage every divergence found into
producer-text (Phase 2) or schema-source (child work item under 0057).

### Changes Required:

#### 1. Discovery procedure (re-runnable, recorded)

Derive the producing-skill set mechanically and record the exact command so the
enumeration is reproducible. Membership rule (per the agreed scope): **a SKILL.md
that emits a fresh schema-governed frontmatter block**, plus the two status-
transition mutators on the status axis only.

```bash
# Producing-skill discovery (run from repo root). Markers indicate a skill
# substitutes/writes schema-governed frontmatter.
grep -rlE 'schema_version:|Populate frontmatter|Substitute .*frontmatter|frontmatter-emission|artifact-derive-metadata\.sh' \
  skills --include='SKILL.md' | sort -u
```

**This raw command returns 17 files, not 16** — it surfaces
`skills/config/migrate/SKILL.md` (which carries `schema_version:` but is the
corpus transformer, explicitly out of scope). It does **not** surface
`skills/decisions/review-adr/SKILL.md`: review-adr is a status-transition mutator
with none of the full-block markers above. So the producer set is *not* the raw
grep output; it is reconciled exactly as the population test does
(`test-skill-frontmatter-population.sh:248-281`): the discovery output, minus a
recorded exclusion allowlist, asserted equal to the emitter allowlist via
`comm -23` — with the status-axis-only mutators tracked in a separate named list
because no full-block marker reaches them.

```bash
# Reconciliation (mirrors the population test's comm -23 discovery assertion).
# EMITTERS: the 16 full-block emitters (cross-checked against skills-schema.tsv
#   rows 2-17). EXCLUDED: surfaced by the grep but out of scope, recorded with
#   reason. STATUS_AXIS_ONLY: mutators the grep does NOT surface, added by hand.
discovered=$(grep -rlE '…' skills --include='SKILL.md' | sort -u)   # 17 files
allowlist=$(printf '%s\n' "${EMITTERS[@]}" "${EXCLUDED[@]}" | sort -u)
comm -23 <(printf '%s\n' "$discovered") <(printf '%s\n' "$allowlist")  # must be empty
```

`EMITTERS` (16 full-block emitters, cross-checked against `skills-schema.tsv`
rows 2-17):

- **work/**: `create-work-item`, `extract-work-items`, `refine-work-item`,
  `review-work-item`
- **planning/**: `create-plan`, `review-plan`, `validate-plan`
- **decisions/**: `create-adr`, `extract-adrs`
- **research/**: `research-codebase`, `research-issue`
- **design/**: `inventory-design`, `analyse-design-gaps`
- **github/**: `describe-pr`, `review-pr`
- **notes/**: `create-note`

`STATUS_AXIS_ONLY` (not surfaced by the discovery grep; tracked by hand):
`validate-plan`→plan (already an emitter; this is its *second* type) and
`review-adr`→adr.

`EXCLUDED` carries **only the grep-surfaced exclusion** the `comm -23` allowlist
needs: `config/migrate` (corpus transformer — surfaced by the discovery grep,
subtracted via the allowlist). `update-work-item` (arbitrary field mutator) and
`list-work-items` (consumer) are **out-of-scope by construction** — neither
carries a discovery marker, so neither is ever surfaced; they are recorded here
for the reader but are *not* members of the Phase 3 `EXCLUDED` array (adding them
would be harmless but misleading, since the allowlist only needs to be a superset
of the discovered set).

#### 2. Per-(skill, type) conformance table

For each (skill, type), one row per attribute the type's contract enforces,
mapping attribute → validator rule satisfied (or fix applied). The attribute set
per type is derived from `templates-schema.tsv` + `frontmatter-emission-rules.sh`
(base fields ∪ per-type extras ∪ provenance-if-anchored ∪ typed-linkage keys ∪
status). Each row records the source of the emitted value: **skill literal**,
**template**, or **metadata helper** (composed-emission attribution).

Completeness is *mechanically* checkable, not merely spot-checked: the Phase 3
guard derives the expected attribute set per type from the contract files (the
union above) and asserts the synthesized fixture exercises every attribute in it
— so a missing attribute fails the guard rather than escaping a human read. The
recorded table is the human-readable rendering of that same derivation; it is a
point-in-time snapshot, not an independently-maintained contract. The snapshot
may legitimately lag the contract as the schema evolves — the guard, not the
table, is the live authority; a stale table is expected, not a defect.

**Scope of the completeness guarantee** (precise, to avoid over-claiming): for
each (skill, type) the guard verifies that the union of (a) the *extracted skill
literals*, non-empty-asserted, and (b) the keys of the *loaded template* covers
the contract-derived enforced set. Attributes supplied dynamically by
`artifact-derive-metadata.sh` (the timestamps, `revision`/`repository`) are
verified present in that composed union, not traced to a specific producer line —
so the guarantee is "the composed emission covers the enforced set", not "every
attribute appears as a verbatim SKILL.md literal". A producer that silently
stopped emitting a *template-supplied* required extra is caught because the
template-key check fails, not because a synthesized placeholder masks it.

Two blind-spot axes are added to each table by inspection (not validator-
checkable): provenance over-emission on non-anchored types; bare/unquoted
typed-linkage values.

**Location**: append the table to the work item body as a "Discovery Pass Record"
section (matching the population test's existing naming,
`test-skill-frontmatter-population.sh:242-246`; body content, does not affect
frontmatter validation), with columns `skill | type | attribute | source
(literal|template|helper) | validator rule / fix`. The discovery command and the
attribute-set derivation live inside the Phase 3 guard; the recorded table shares
the *enumeration* with the guard (single source for the producer set) but is
itself a point-in-time snapshot — the guard, not the table, is the authority on
attribute completeness.

#### 3. Divergence triage

Each divergence is classified:

- **Producer-text** (fix in Phase 2): e.g. `validate-plan:187` `complete → done`.
- **Schema-source** (raise child work item under 0057, do not fix here): e.g.
  ADR `rejected` — decide whether the `adr` vocab should gain `rejected` or
  `review-adr` should stop persisting it; record the decision and, if
  schema-source, draft the child work item via `create-work-item` with
  `parent: "work-item:0057"`.

### Success Criteria:

#### Automated Verification:

- [x] Discovery command is re-runnable and returns the expected 17 files:
      `test "$(grep -rlE 'schema_version:|Populate frontmatter|Substitute .*frontmatter|frontmatter-emission|artifact-derive-metadata\.sh' skills --include='SKILL.md' | sort -u | wc -l)" -eq 17`
- [x] Reconciliation holds — discovery minus the EXCLUDED allowlist equals the
      EMITTERS allowlist (`comm -23` against EMITTERS ∪ EXCLUDED is empty), and
      both status-axis-only mutators (`validate-plan`, `review-adr`) are present.
- [x] The work item still validates: `bash scripts/validate-corpus-frontmatter.sh meta/work/0103-audit-skill-frontmatter-emission-against-unified-schema.md`
- [x] Any drafted child work items validate (0104, 0105). NB: the literal
      `validate-corpus-frontmatter.sh meta/work` runs *whole-corpus* mode against a
      subtree, so every cross-subtree ref (to `adr:`/`codebase-research:`/… in
      other `meta/` subtrees) reports a pre-existing `DANGLING-REF`. The
      meaningful checks both pass: structural file-list validation of the new
      files, and a clean full-corpus run (`validate-corpus-frontmatter.sh meta`,
      rc=0). The new files add no dangling refs (they reference only
      `work-item:0057` / `work-item:0103`, both under `meta/work`).

#### Manual Verification:

- [x] Every discovered producer appears in the table with its emitted type(s).
- [x] The recorded table is a faithful point-in-time rendering of the Phase 3
      guard's contract-derived attribute set (the guard, not this manual check, is
      the completeness authority — confirm the snapshot matches the guard's
      derivation, not the table against `templates-schema.tsv` independently).
- [x] Both blind-spot axes are recorded for every type that can exhibit them.
- [x] Every divergence is classified producer-text vs schema-source, with a
      recorded rationale; schema-source items have child work items under 0057
      (0104 = ADR `rejected` schema-source; 0105 = blind-spot consolidation).

---

## Phase 2: Fix Confirmed Producer-Text Divergences

### Overview

Fix every producer-text divergence the audit confirmed, driven red→green against
the existing corpus validator. At minimum this is `validate-plan:187`
`complete → done`; it includes any further producer-text fixes Phase 1 surfaces
(but excludes schema-source items).

### Changes Required:

#### 1. validate-plan plan-status fix

**File**: `skills/planning/validate-plan/SKILL.md`
**Changes**: change the plan-status literal at `:186-188` from `complete` to
`done`; leave the plan-validation report status at `:161` as `complete`.

```markdown
4. If the validation result is `pass`, update the plan's frontmatter
`status` field to `done` (if the plan has YAML frontmatter with a
`status` field). This closes the plan lifecycle.
```

TDD oracle (no new infra): synthesize a minimal `plan` fixture carrying
`status: complete`, confirm the validator rejects it, then confirm a `status:
done` fixture is accepted — mirroring `test-validate-corpus-frontmatter.sh`'s
`emit_valid` + `assert_rejects`/`assert_accepts` pattern:

```bash
# Use a trap-scoped workdir, not hardcoded /tmp (matches the guard discipline).
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
# RED: a plan with the currently-documented literal is rejected.
printf -- '---\ntype: plan\nid: "x"\ntitle: "t"\ndate: "2026-01-01T00:00:00+00:00"\nauthor: a\ntags: []\nlast_updated: "2026-01-01T00:00:00+00:00"\nlast_updated_by: a\nschema_version: 1\nrevision: "r"\nrepository: "repo"\nstatus: complete\n---\n# t\n' > "$work/plan-complete.md"
bash scripts/validate-corpus-frontmatter.sh "$work/plan-complete.md"   # expect BAD-STATUS, rc=1
# GREEN: the corrected literal validates.
sed 's/status: complete/status: done/' "$work/plan-complete.md" > "$work/plan-done.md"
bash scripts/validate-corpus-frontmatter.sh "$work/plan-done.md"       # expect rc=0
```

#### 2. Any additional producer-text fixes from Phase 1

Apply only divergences classified producer-text in Phase 1, each kept consistent
with `templates-schema.tsv` (no hard-coded parallel contract). Schema-source
items (e.g. ADR `rejected`, if so classified) are **not** touched here.

### Success Criteria:

#### Automated Verification:

- [x] Behavioural regression (authoritative): the red→green oracle above behaves
      as documented — a synthesized `plan` fixture carrying the post-fix
      plan-status literal is **accepted** by `validate-corpus-frontmatter.sh`, and
      one carrying `complete` is **rejected** with `BAD-STATUS`. (This verifies
      emission behaviour, not text position, so it survives routine line shifts.)
- [x] Sanity aid (non-authoritative, no line numbers): `validate-plan/SKILL.md`
      still documents `done` for the plan mutation (`:187`) and retains `complete`
      for the plan-validation report (`:161`). Confirmed by reading the two sites.
- [x] Existing population/prose test stays green: `mise run test:unit:templates`
      (36 passed, 0 failed).
- [x] Corpus validator + existing config suites stay green:
      `mise run test:integration:config` (exit 0, 15 suites).

#### Manual Verification:

- [x] The red→green oracle steps above behave as documented (reject `complete`,
      accept `done`).
- [x] No producer-text divergence classified in Phase 1 is left unfixed; no
      schema-source item was fixed here by mistake (only the `validate-plan`
      plan-status literal changed; the `review-adr` `rejected` item is deferred to
      0104).

---

## Phase 3: Automated Producer-Conformance Guard

### Overview

Add `scripts/test-skill-frontmatter-conformance.sh`: for each producing skill it
extracts the verbatim frontmatter literals from SKILL.md (keyed by (skill, type)),
synthesizes a complete fixture (literals + valid placeholders for dynamic base
fields + provenance/extras per the type), runs the real
`validate-corpus-frontmatter.sh`, and asserts it passes — exercising every
attribute the type's contract enforces and both branch variants of each
conditional axis (anchored vs non-anchored provenance, linkage present vs absent,
each omit-when-empty key present-and-valid vs absent). A negative self-test
mutates each synthesized fixture's value to a known-bad one (one mutation per
axis) and asserts rejection with the specific diagnostic — proving the guard is
wired, not green-path-only. Dedicated assertions, each with its own liveness case,
cover the two validator blind spots. Wire it into `test:integration:config`.

### Changes Required:

#### 1. The guard script

**File**: `scripts/test-skill-frontmatter-conformance.sh` (new, `chmod +x`)
**Changes**: bash 3.2-safe, `set -euo pipefail`, `export LC_ALL=C`, source
`test-helpers.sh` and `frontmatter-emission-rules.sh`, `mktemp -d` + `trap …
EXIT`, end with `test_summary`. Source the contract — never re-encode it. For
fixture synthesis, **factor `emit_valid()` (and the `assert_rejects`/
`assert_accepts` pair) out of `test-validate-corpus-frontmatter.sh` into a new
non-`test-`-prefixed shared helper** (e.g. `scripts/frontmatter-fixtures.sh`,
named so the `test-*.sh` discovery glob never tries to run it) that both suites
source — rather than one `test-*.sh` sourcing another (no precedent in `scripts/`)
or duplicating the synthesizer. Design the extracted-literal override surface up
front; the existing suite must stay green after the move (verify via its own
run). This keeps the two suites peers sharing one fixture authority, so a future
schema tightening lands in one place. Organise the per-producer loop into clearly-labelled helpers per assertion
family — `assert_accepts_composed`, `assert_no_provenance_over_emission`,
`assert_linkage_shape`, `assert_status_in_vocab` — with a comment marking which
bypass the validator (the two blind-spot checks) and why, so the differing
strength of each guarantee is legible.

Structure (modelled on `test-validate-corpus-frontmatter.sh`):

```bash
# Three named arrays reconcile the producer set (Phase 1 reconciliation), so it
# cannot silently grow or shrink — the population test's Phase-11 precedent.
EMITTERS=( skills/work/create-work-item/SKILL.md … skills/notes/create-note/SKILL.md )  # 16
EXCLUDED=( skills/config/migrate/SKILL.md )      # surfaced by discovery, out of scope
STATUS_AXIS=( skills/planning/validate-plan/SKILL.md skills/decisions/review-adr/SKILL.md )
# Liveness gate (pin the count, like the population test's exact-PASS-count idiom):
#   discovered=$(grep -rlE '…' skills --include='SKILL.md' | sort -u)
#   assert "$(printf '%s\n' "$discovered" | wc -l)" -eq 17
#   assert comm -23 <(discovered) <(sort -u EMITTERS∪EXCLUDED) is empty
#   assert ${#EMITTERS[@]} -eq 16

# Extraction is keyed by (skill, TYPE), not (skill, field): validate-plan emits
# TWO types (plan-validation/complete and plan/done), so a per-field lookup is
# ambiguous. The guard resolves each (skill, type) pair independently.
extract_literal() { # $1 SKILL.md, $2 type, $3 field -> verbatim value or ""
  # Two documented instruction-context grammars (mirror
  # test-skill-frontmatter-population.sh:8-17), anchored on ASCII tokens only.
  # Both non-ASCII glyphs the SKILL.md corpus uses — `←` (U+2190) in the
  # substitute-list and `→` (U+2192) in the review-adr transition table — are
  # matched as opaque byte runs under LC_ALL=C, never adjacent to a regex
  # metacharacter, so extraction is byte-identical under BSD and GNU tooling.
  # Capture strictly the `[^`]*` token between a backtick pair (POSIX-class,
  # no GNU-only grep/sed flags):
  #   (a) substitute-list:  - `<field>:` ← `<value>`      (e.g. create-plan:242)
  #   (b) fenced block:      ^<field>: <value>             (within a ``` block)
  # Full-block emitters use (a)/(b); the value is extracted between backticks.
  ...
}
# Status-transition mutators do NOT use (a)/(b) — the target status lives in
# prose or a state table. The guard extracts these via per-skill anchors,
# anchoring on the backtick-wrapped ASCII status tokens (`proposed`, `accepted`,
# `done`) and treating `→`/`←` as opaque bytes:
#   validate-plan -> plan:  prose at SKILL.md:~187 ("status` field to `done`")
#                           (POST-Phase-2 state; pre-fix this line reads `complete`,
#                            so the status-axis liveness presupposes Phase 2 merged)
#   review-adr    -> adr:   transition table at :85-89 + prose at :194
# Each anchor yields a non-empty value AND that value must itself be a member of
# the target type's status_vocab — so a mis-anchored extraction that grabs
# unrelated prose (non-empty but wrong) fails the guard rather than asserting
# against a bogus literal.

# LIVENESS (every claimed extraction must succeed): for each (skill, type, field)
# the guard intends to check, assert extract_literal returned non-empty. A SKILL.md
# whose literal form changed shape (reworded bullet, moved prose) then fails the
# guard LOUDLY instead of silently substituting a placeholder and passing green.
```

Per (skill, type) the guard derives the enforced attribute set from the contract
(`templates-schema.tsv` row ∪ `FM_BASE_FIELDS` ∪ provenance-iff-anchored ∪
typed-linkage keys ∪ status), synthesizes a fixture exercising **every** attribute
in that set (filling dynamic base fields with valid placeholders), and asserts the
validator **accepts** it — so a missing attribute fails the guard (AC2
completeness, mechanical). It additionally asserts the two blind-spot axes the
validator misses, expressed via the **shared helper symbols** (not re-encoded):

- **Provenance over-emission**: for a type whose TSV `code_state_anchored` is not
  `yes`, assert none of `FM_PROVENANCE_FIELDS` appears in the *composed* emission
  (skill literals **and** the loaded template — provenance is template-supplied,
  so a skill-literals-only check would pass vacuously). The guard resolves
  (skill, type) → template by reading the `config-read-template.sh` inclusion line
  each SKILL.md carries (the same resolution the population test relies on), reads
  that template's frontmatter keys, and asserts the provenance keys are absent.
  The liveness fixture (below) must flow through this **same** composed-emission
  assembly — not a hand-built parallel fixture — so the negative case proves the
  production code path can reject. This axis (and the linkage axis) is enforced
  here only until the 0057 child item folds it into `validate-corpus-frontmatter.sh`
  itself (see References); a guard comment must name that child item so the
  temporary duplicate authority has one tracked path back to the single oracle.
- **Bare/unquoted linkage**: any literal typed-linkage value the skill documents
  is quoted and matches `FM_TYPED_REF_RE`.

Status-axis-only producers (`validate-plan`→plan, `review-adr`→adr): assert the
documented status-transition literal(s) for the *target* type are members of that
type's `status_vocab` (this is the axis that caught the validate-plan bug and the
review-adr `rejected` finding). **Deferred-divergence handling**: if Phase 1
classifies `review-adr`'s `rejected` as schema-source (raised as a 0057 child,
not fixed here), the guard asserts review-adr's *vocab-valid* transitions
(`proposed → accepted`) as live assertions, and represents the deferred
`rejected` axis as an explicit **`skip_test`** line keyed to the 0057 child id
(not a prose note) — so the gap is visible in test output and flips to a real
`assert_rejects` when the child item lands, rather than being silently forgotten.
The guard thus neither goes red on a known-deferred divergence nor silently drops
the producer.

#### 2. Negative / liveness self-tests (so the guard cannot go green-path-only)

Corrupt the extracted **value**, not the SKILL.md text. The earlier illustrative
`sed 's/status: done/…/'` was a no-op — `validate-plan/SKILL.md` has no such
substitute-list string (its `done` is prose at `:187`, its only bullet status is
the plan-validation `complete` at `:161`), so the "corrupted" copy was identical
to the original and the validator still accepted it: a false-green wiring proof.
Instead, synthesize each fixture from the extracted literals, then mutate the
fixture's field to a known-bad value and assert REJECT — and assert the mutation
actually changed the fixture before validating, so a no-op mutation fails loudly.

```bash
# Wiring proof (AC5), parameterised over one mutation per axis the guard covers,
# each asserting the SPECIFIC diagnostic — a single status mutation would leave
# the type/extras/schema_version synthesis paths green-path-only.
#   bad type            -> BAD-TYPE        (unknown discriminator)
#   bad status          -> BAD-STATUS      (out-of-vocab for the type)
#   missing req. extra  -> MISSING-EXTRA
#   non-integer ver.    -> BAD-SCHEMA-VERSION
for axis in type status extra schema_version; do
  fixture=$(synthesize_from_extracted "$skill" "$type")        # valid baseline
  mutated=$(mutate_axis "$fixture" "$axis")                    # inject known-bad value
  [ "$mutated" != "$fixture" ] || fail "axis=$axis mutation was a no-op"   # guard the guard
  assert_rejects_with "$mutated" "$(expected_code "$axis")"
done

# Blind-spot liveness: each by-inspection check must itself be shown able to fail.
assert_rejects_blindspot "non-anchored fixture carrying revision/repository"   # provenance over-emission
assert_rejects_blindspot "fixture carrying a bare  parent: 0042  (unquoted)"   # linkage shape

# Count-gated aggregate so a zero-iteration loop can't pass inert (population
# test precedent): discovered==17, comm -23 empty, ${#EMITTERS[@]}==16.
```

#### 3. Wire into CI

**File**: `tasks/test/integration.py`
**Changes**: bump `_EXPECTED_CONFIG_SUITES` from `15` to `16` (`:14`).

```python
_EXPECTED_CONFIG_SUITES = 16
```

The script is auto-discovered by `run_shell_suites(context, "scripts")` once it
lives at `scripts/test-*.sh`, is a regular file, and has the exec bit
(`helpers.py:29-35`). The `_EXPECTED_CONFIG_SUITES` floor only guarantees *at
least* N suites ran, not that *this* gate ran — a guard renamed off the
`test-*.sh` convention would vanish while the count still passes via other
suites. **Also assert** (not optionally — the whole point of this work item is a
gate that cannot drift undetected) that `run_shell_suites`' returned suite list
contains `test-skill-frontmatter-conformance` by identity, so the gate's presence
is checked by name, not just by aggregate count. (If the `migrate`/`config` floors
later adopt the same by-name idiom for consistency, that is a separate, optional
follow-up — noted so the two CI gates can converge.)

### Success Criteria:

#### Automated Verification:

- [ ] The guard is executable: `test -x scripts/test-skill-frontmatter-conformance.sh`
- [ ] The guard passes standalone: `bash scripts/test-skill-frontmatter-conformance.sh`
- [ ] It is discovered and the floor tracks it (16 suites):
      `mise run test:integration:config`
- [ ] Negative test proves wiring — the in-suite self-test mutates each
      synthesized fixture's value (one mutation per axis: type, status, missing
      extra, schema_version), asserts the mutation actually changed the fixture,
      and fails the suite if any mutated fixture is *accepted*.
- [ ] Each conditional axis is exercised on both branches (anchored vs
      non-anchored provenance, linkage present vs absent, omit-when-empty key
      present-and-valid vs absent) — synthesized and validated in-suite, per AC4.
      The omit-when-empty key set is `FM_OPTIONAL_EXTRAS` ∩ the type's extras plus
      the type's typed-linkage keys (not `tags`, which the validator exempts at
      `:343`); includes an `EMPTY-PLACEHOLDER` liveness fixture (a non-`tags` key
      emitted as `""`) alongside the present-and-valid and absent branches.
- [ ] Both blind-spot checks have a liveness case (a deliberately non-conforming
      fixture each must reject), so neither can rot into an assertion-free no-op.
- [ ] Every claimed extraction is non-empty: the guard fails if a (skill, type,
      field) it intends to check yields an empty literal (formatting-drift guard).
- [ ] No re-encoded contract: the guard sources `frontmatter-emission-rules.sh`
      and reads `templates-schema.tsv`
      (`grep -nE "frontmatter-emission-rules\.sh|templates-schema\.tsv" scripts/test-skill-frontmatter-conformance.sh`).
- [ ] Full integration suite green: `mise run test:integration:config`

#### Manual Verification:

- [ ] Temporarily reverting `validate-plan` to `status: complete` for the plan
      makes the guard fail with a plan `BAD-STATUS`-class diagnostic (then revert).
- [ ] The guard is bash 3.2-safe (no associative arrays / bash-4 constructs);
      reasoned-through against the macOS CI floor. In particular, extraction
      anchors on ASCII tokens and treats the `←` (U+2190) glyph as opaque bytes
      under `LC_ALL=C`, so it behaves identically under BSD and GNU tooling.

---

## Testing Strategy

### Unit / prose tests:

- `mise run test:unit:templates` (existing `test-skill-frontmatter-population.sh`,
  `test-template-frontmatter.sh`) stays green — the validate-plan literal change
  must not break the prose population checker.

### Integration tests:

- `mise run test:integration:config` runs the existing corpus-validator suite
  plus the new conformance guard; floor bumped 15 → 16.
- The new guard generates fixtures, runs the real validator, and includes a
  negative self-test and a count-gated liveness assertion.

### Manual Testing Steps:

1. Run the discovery command; confirm it returns 17 files and that the
   reconciliation (discovery − EXCLUDED == EMITTERS, plus the two status-axis
   mutators) holds.
2. Synthesize the plan `complete` vs `done` fixtures (Phase 2 oracle); confirm
   reject then accept.
3. Temporarily revert the validate-plan plan-status to `complete`; confirm the
   Phase 3 guard fails; revert.
4. Temporarily make a non-anchored producer's literals include `revision`;
   confirm the guard's blind-spot assertion (not the validator) catches it.

## Performance Considerations

The guard validates ~16 synthesized single-file fixtures per run via file-list
mode (structural checks only, no whole-corpus walk), well inside the config
suite's existing budget. bash 3.2 / `LC_ALL=C` discipline applies; parse the TSV
with the established `tail -n +2 | IFS=$'\t' read` pattern (no `declare -A`).
Fixture mutation writes to a fresh file (`sed '…' in > out`) or uses
`sed -i.bak` with a mandatory suffix — **never bare `sed -i`**, which fails on
macOS/BSD (the single most common BSD/GNU divergence in this subtree).

## Migration Notes

No data migration. The only corpus-affecting change is the validate-plan literal,
which changes future emissions only; existing plans already migrated to `done`
under 0070 are unaffected. Schema-source divergences are deferred to child work
items under 0057, not applied here.

## References

- Original work item: `meta/work/0103-audit-skill-frontmatter-emission-against-unified-schema.md`
- Related research: `meta/research/codebase/2026-06-09-0103-skill-frontmatter-emission-audit.md`
- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- **Blind-spot consolidation (child of 0057, to be drafted in Phase 1)**: fold the
  two validator blind spots — non-anchored provenance over-emission and
  bare/unquoted typed-linkage shape — into `validate-corpus-frontmatter.sh` so the
  guard's bespoke checks (and the audit table's by-inspection coverage) collapse
  back to the single oracle. The Phase 3 guard's `assert_no_provenance_over_emission`
  / `assert_linkage_shape` helpers must carry a comment naming this child item, so
  the temporary three-authority state (validator-doesn't, guard-does, table-records)
  has one tracked path back to one authority.
- Migration that shipped the contract: `meta/plans/2026-06-07-0070-meta-corpus-unified-schema-migration.md`
- Status reconciliation: `meta/decisions/ADR-0042-reconciling-pre-schema-status-values.md`
- Contract surfaces: `scripts/validate-corpus-frontmatter.sh`,
  `scripts/frontmatter-emission-rules.sh`, `scripts/templates-schema.tsv`
- Test precedents: `scripts/test-validate-corpus-frontmatter.sh` (structural model),
  `scripts/test-skill-frontmatter-population.sh` (discovery-pass / liveness precedent)
- Wiring: `tasks/test/integration.py:14`, `tasks/test/helpers.py:13-40`
- Confirmed divergences: `skills/planning/validate-plan/SKILL.md:186-188`,
  `skills/decisions/review-adr/SKILL.md:85,194`

---
type: plan
id: "2026-06-15-0102-remove-visualiser-legacy-linkage-fallback-arms"
title: "Remove Visualiser-Server Legacy Linkage Fallback Arms Implementation Plan"
date: "2026-06-15T22:59:59+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0102"
parent: "work-item:0102"
blocks: ["work-item:0057"]
derived_from: ["codebase-research:2026-06-15-0102-remove-visualiser-legacy-linkage-fallback-arms"]
relates_to: ["adr:ADR-0034", "adr:ADR-0033"]
tags: [migration, visualiser, frontmatter, linkage, cleanup, contract]
revision: "f693db554a35d0ab9ccc09d54bf22364083faa93"
repository: accelerator
last_updated: "2026-06-16T07:19:46+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Remove Visualiser-Server Legacy Linkage Fallback Arms Implementation Plan

## Overview

This is the **contract** (third) phase of the expand/migrate/contract sequence
that story 0070 began. 0070 expanded the visualiser server's linkage reader to
accept both legacy and unified linkage shapes and *deprecated* (not removed) the
transitional fallback arms so an un-migrated userspace repo would not break. This
plan removes those now-dead arms and their pinning tests, leaving a single
canonical linkage-resolution path, and adds an **observable migration-completion
gate** to this repo's CI so the removal is anchored to a verifiable
"corpus is migration-complete" signal.

Four legacy resolution behaviours are removed across three Rust source files;
one shell gate is added by extending the existing corpus validator; and a small
amount of corpus hygiene (dead `ticket:` frontmatter on legacy plans) is done so
the gate passes clean.

## Current State Analysis

All coordinates below were re-derived against the current working revision
`049d050` (the research doc's `23ce3d8` and the work item's `20e5760` both still
match — no drift in the affected files).

**The three server removal sites** (`skills/visualisation/visualise/server/src/`):

1. `frontmatter.rs:read_ref_keys` (`:321-391`) — a mutually-exclusive
   `if / else if` chain reading cross-reference keys in priority order:
   `work_item_id:` (preferred, **keep**), `work-item:` (legacy, **remove**;
   `tracing::warn!` at `:357-362`), `ticket:` (oldest legacy, **remove**; no
   warn, resolves silently), `target:` (typed ADR-0034, **keep**).
2. `cluster_key.rs:parent_or_legacy_id` (`:128-157`) — `parent:` branch
   (canonical, **keep**) then the legacy `work_item_id:` branch (`:138-155`, bare
   `warn!` at `:148-152`, **remove**). Shared helper `id_from_value`
   (`:164-178`) stays — it serves the `parent:` branch.
3. `indexer.rs` work-item identity chain in `build_entry` (`:1382-1400`) — `id:`
   primary (**keep, becomes sole resolver**), legacy `work_item_id:` arm
   (`:1384-1390`, **remove**), filename fallback (`:1391-1397`, **remove**).

**The migration-completion gate.** The work item specifies "a single recursive
grep over `meta/`… anchored to the frontmatter key at line start" returning zero
legacy own-identity shapes. Investigation showed the literal recipe cannot work:
running `^\s*work_item_id:` / `^\s*ticket_id:` over `meta/` returns **126
matches**, of which:

- **117** are `work_item_id: "NNNN"` on **plans / research / reviews** — the
  *current, template-emitted foreign-reference* field (`codebase-research.md:9`,
  `plan.md:9`, `rca.md:9`, `pr-description.md:9` all ship
  `work_item_id: ""  # foreign reference`). Per ADR-0033, own identity is keyed
  `id:` and foreign refs use `<type>_id:`; 0070 deliberately *kept* foreign refs.
  A line-start anchor does **not** distinguish own-identity from foreign
  reference — only the **doc type** does.
- The legacy *own-identity* shapes the gate actually targets
  (`work_item_id:`/`ticket_id:` on work-item and ADR docs, where those keys are
  forbidden own-ids) are **already at zero** — the existing
  `scripts/validate-corpus-frontmatter.sh` (the "AC-1 corpus validator") already
  enforces this per-type and **passes today**. Verified empirically at revision
  `b2f39a4`: `bash scripts/validate-corpus-frontmatter.sh meta/` exits 0 with no
  output across *all existing* clauses (own-id, dangling-ref, provenance,
  linkage-shape, etc.), not only `FORBIDDEN-OWN-ID`. Note this run predates the new
  `OBSOLETE-LEGACY-KEY` clause, so it proves only the *pre-change* corpus is clean;
  *post-change* exit-0 additionally depends on the 10-file `ticket: null` scrub
  (landed in the same phase). Re-verify the whole-corpus run after both the check
  and the scrub are in place.
- The remaining hits are `ticket: null` in the frontmatter of **10 legacy
  plans** (the old foreign-reference form, never migrated by migration `0001`,
  which only renamed `ticket_id:`→`work_item_id:` inside the tickets directory),
  plus `ticket_id:` / `ticket:` examples inside markdown **body code-fences**
  (not frontmatter at all — already ignored by the validator, which parses only
  the fenced block).

### Key Discoveries:

- **Decision rule resolved (research + evidence):** the frontmatter
  `work_item_id:` **key is retained**. It is the *preferred* cross-ref key in
  `read_ref_keys` (`frontmatter.rs:346-349`), is read by the clustering resolver
  (`cluster_key.rs:140`) and the patcher (`patcher.rs:280,285,288,438,446`), and
  is the current foreign-reference field in four artifact templates. Only its two
  legacy *resolution arms* (identity in `indexer.rs`, clustering in
  `cluster_key.rs`) are removed. The "default to removal" clause does not fire.
- **`target:` is independently load-bearing — keep at all three sites.**
  `target_path_from_entry` (`indexer.rs:974-998`) reads `target:` via
  `parse_typed_ref` and is consumed by the reviews-by-target reverse index
  (`indexer.rs:384`), cross-ref (`:849`), refresh diff (`:1107,:1115,:1171`) and
  clustering (`cluster_key.rs:95`). The `work-item:` *value prefix* inside a
  `target:` value (`typed_ref.rs:38-46`) is unrelated to the `work-item:`
  *frontmatter key* arm and must not be touched.
- **`extract_id` is not dead** after the filename fallback is removed — it is
  still used at `cluster_key.rs:173` (the `parent:` path-shape branch) and in
  `config.rs` tests.
- **`capture_logs` becomes dead and must be removed.** `test_support` is
  `#[cfg(test)]`-gated (`log.rs:58`); `capture_logs` (`log.rs:147`) has exactly
  **four** callers — the four deprecation tests being deleted
  (`frontmatter.rs:577`, `cluster_key.rs:298`, `indexer.rs:3469`, `:3488`). CI
  promotes rustc/clippy warnings to errors via `-D warnings` (`Cargo.toml:74`),
  so a dead `capture_logs` fails `mise run check`. Whichever phase removes the
  last caller must remove the fn. `test_support` itself stays (its
  `build_test_json_subscriber` is still used at `log.rs:295`).
- **Naming traps that survive removal:** the bare `warn!` import
  (`cluster_key.rs:16`) stays — it is also used at `:61` (MAX_DEPTH truncation);
  the shape-validation `tracing::warn!` (`indexer.rs:1372-1377`) stays — it
  guards the surviving `id:` path, not a legacy arm.

## Desired End State

- No legacy linkage-resolution path remains in the visualiser server: a grep
  over `skills/visualisation/visualise/server/src/` for the `work-item:`
  frontmatter-key arm, the `ticket:` arm, `parent_or_legacy_id`, the legacy
  `work_item_id:` clustering/identity branches, and the filename fallback all
  return nothing. The typed `target:` arm and the `work_item_id:` *key* (as
  preferred cross-ref and foreign reference) are retained.
- `mise run test:unit:visualiser` passes in **both** server feature modes
  (`embed-dist` default and `dev-frontend`), with legacy pinning tests deleted
  and survivors retargeted to canonical resolution.
- An observable migration-completion gate runs in CI (`mise run check` ⊇
  `mise run test:integration:config`): the corpus validator flags any frontmatter
  carrying the obsolete `ticket:` / `ticket_id:` keys (anywhere, any type) and
  the established own-identity prohibition (`work_item_id:` on work-item,
  `adr_id:` on ADR) continues to pass. The whole-corpus run is clean
  (`bash scripts/validate-corpus-frontmatter.sh meta/` → `exit 0`).

## What We're NOT Doing

- **Not** removing or altering the typed `target:` arm
  (`frontmatter.rs:369-379`), `parse_typed_ref`, or the `work-item:` value-prefix
  handling in `typed_ref.rs`.
- **Not** removing the frontmatter `work_item_id:` *key* support — only its two
  legacy resolution arms. Foreign-reference `work_item_id:` on
  plans/research/reviews is current and retained.
- **Not** removing the shape-validation warn (`indexer.rs:1372-1377`),
  `id_from_value`, `extract_id`, or the `warn!` import in `cluster_key.rs`.
- **Not** scrubbing `ticket:` / `ticket_id:` occurrences inside markdown **body**
  text / code-fences of historical plans and research docs — they are
  documentation, not frontmatter, and the frontmatter-scoped gate ignores them.
- **Not** touching the existing `build_index` own-id fallback that reads
  `work_item_id`/`adr_id` (`validate-corpus-frontmatter.sh:213-216`) — it is
  tolerant and harmless post-migration (own `id:` is always present); out of
  scope.

## Implementation Approach

The migration-completion gate is implemented **first** (Phase 1): the work item
is explicit that arm removal must not precede migration completion, and the gate
is the observable proxy for that precondition **in this repo's corpus** (it
cannot observe external corpora — see Migration Notes). Establishing and passing the gate
before removing the safety arms mirrors the expand/migrate/contract discipline.

Phases 2–4 then remove the three server sites. Each phase is **independently
mergeable** — every phase leaves `mise run check` green on its own. The only
cross-phase concern is the dead `capture_logs` **capture harness**; it is handled
with an order-independent rule applied in every server phase: *after deleting this
phase's `capture_logs` test(s), grep for remaining `capture_logs` callers in
`src/`; if none remain, remove the entire thread-local capture block in
`log.rs`.* This keeps each phase green regardless of the order in which the phases
merge. `capture_logs` is the **single** cross-phase coupling point (its four
callers span all three server sites); in the expected merge order Phase 4 holds
the last two callers and is therefore the canonical owner of the deletion.

> **Remove the whole harness, not just the fn.** `capture_logs` (`log.rs:147`) is
> the *sole* consumer of an exclusive support block (`log.rs:102-164`): the
> `use std::cell::RefCell;` and `use std::sync::Once;` imports, the `CAPTURE_BUF`
> thread-local, `ThreadLocalWriter` + its `Write` impl, `ThreadLocalMakeWriter` +
> its `MakeWriter` impl, and the `CAPTURE_INIT` static. None is referenced
> elsewhere, so removing only the fn leaves all of them dead and **fails
> `-D warnings`** (`Cargo.toml:74`). The rule therefore removes the entire block
> `log.rs:102-164` as a unit. **Keep** `build_test_json_subscriber` and its
> `Mutex*Writer` helpers (still used at `log.rs:295`) — they are independent of the
> capture harness. Because the rule is evaluated
against tree state **at edit time**, the integrating phase must re-run
`grep -rn capture_logs src/` after any rebase/merge before relying on green — a
stale `capture_logs` fails loud under `-D warnings` (contained, not silent).

Work is test-driven where applicable: for the gate, tests asserting the new
prohibition are written before the check; for the server sites, the test module
is edited first to express the post-removal contract (delete legacy-pinning
tests, retarget survivors to canonical-only resolution), then the arms are
removed, then the suite is run green.

---

## Phase 1: Migration-Completion Gate

### Overview

Extend the existing AC-1 corpus validator to enforce that the **fully-obsolete**
legacy linkage keys `ticket` and `ticket_id` never appear in frontmatter (on any
doc type), and scrub the residual dead `ticket: null` lines from 10 legacy plans
so the whole-corpus run passes. The own-identity prohibition for `work_item_id:`
(work-item) and `adr_id:` (ADR) is already enforced by the validator's
`FORBIDDEN-OWN-ID` check and already passes — this phase makes the *second*
(`ticket:`-lineage) clause real and keeps both clauses observable in CI.

### Changes Required:

#### 1. Cross-cutting obsolete-key prohibition in the validator

**File**: `scripts/validate-corpus-frontmatter.sh`
**Changes**: Declare a fixed list of obsolete legacy linkage keys **at module
scope, alongside the other config arrays** (the `Schema table (parallel arrays;
bash 3.2 has no associative arrays)` section), and add a per-file check (in
`validate_file`, immediately **after** the existing `FORBIDDEN-OWN-ID` loop at
`:307-313`, so the two forbidden-key checks read as one contiguous block) that
flags their presence in the parsed frontmatter. The validator already extracts
and parses only the fenced frontmatter block (`extract_frontmatter` / `parse_fm`),
so body code-fences are ignored for free.

The array name uses a script-local prefix (not the `FM_` prefix, which is owned
by the sourced `frontmatter-emission-rules.sh` and would imply a helper-provided
value):

```bash
# ---- module scope, declared just before the SCHEMA_* parallel-array block ----
# Fully-obsolete legacy linkage keys (forbid on every typed/type-inferable doc —
# see the coverage-boundary note below; this is not a SCHEMA_* parallel column).
# Distinct from FORBIDDEN-OWN-ID (per-type own-id keys via the schema TSV) and
# from build_index's deliberately-tolerant own-id fallback (:213-216): three
# separate policies toward legacy keys live in this script — keep them distinct.
# `ticket`/`ticket_id` were migrated out by 0001 (ticket→work-item) and the 0070
# unified-schema work; no current template emits them on any type.
OBSOLETE_LEGACY_KEYS=(ticket ticket_id)

# ---- in validate_file, immediately after the FORBIDDEN-OWN-ID block ----
for obs in "${OBSOLETE_LEGACY_KEYS[@]}"; do
  bk_present "$obs" &&
    violation "$file" "OBSOLETE-LEGACY-KEY" \
      "obsolete legacy linkage key '$obs' present (use id:/typed references)"
done
```

> Note: this cross-cutting check is preferred over widening the per-type
> `forbidden_own_id_key` column in `templates-schema.tsv`, because `ticket:` /
> `ticket_id:` are obsolete on *every* type (the "anywhere" semantics), not
> own-identity on one type. Clause (a) (`work_item_id:`/`adr_id:` own-identity)
> stays as-is in the schema TSV.

> **Coverage boundary (intentional):** a file only reaches this check if it
> passes the earlier `INVALID-TYPE` guard in `validate_file` (returns early when
> `type:` is absent *and* `infer_type_from_path` yields nothing). So the
> "anywhere, any type" semantics span every *typed/type-inferable* doc — which
> covers all 10 scrub targets (all `plan`-typed) and the whole current corpus.
> An obsolete key in a genuinely untyped/unmapped path would be skipped; the
> validator-test set below includes a case pinning this boundary so it stays
> explicit rather than incidental. The diagnostic carries no story reference
> (matching every sibling violation message); provenance lives in the array
> comment above.

#### 2. Validator test coverage (TDD — write first)

**File**: `scripts/test-validate-corpus-frontmatter.sh`
**Changes**: Add cases (placed in the `=== Failure-mode fixtures ===` section,
beside the existing `bad-ownid` case) asserting the validator:

- (a) emits `OBSOLETE-LEGACY-KEY` and exits non-zero when a fixture's *sole*
  defect is `ticket:`/`ticket_id:`. Build an otherwise-fully-valid fixture and
  assert non-zero exit **with `OBSOLETE-LEGACY-KEY` and no other violation code
  present**, so the gate's standalone observability is pinned (a future reorder
  that let an earlier clause short-circuit first cannot mask it);
- (b) passes on the equivalent clean fixture;
- (c) **negative discrimination** — does **not** emit `OBSOLETE-LEGACY-KEY` for a
  current foreign-reference `work_item_id:` (nor a typed `parent: "work-item:NNNN"`
  ref). The fixture **must be a fully-valid `plan`** (anchored provenance +
  `reviewer` extra, per the suite's existing plan fixtures) — *not* a `work-item`,
  which would trip `FORBIDDEN-OWN-ID` first and mask the negative assertion — then
  `assert_accepts`. This pins the key-vs-reference distinction the whole plan rests
  on (Current State Analysis), so a future accidental widening of
  `OBSOLETE_LEGACY_KEYS` to include `work_item_id` is caught by a unit test rather
  than only the whole-corpus run;
- (d) **coverage boundary** — confirms the intended skip for an obsolete key in an
  untyped/unmapped path. Construct it like the suite's `bad-type` fixture: take an
  `emit_valid` output, `sed` out its `type:` line, and write it at a flat
  `$TMP/*.md` path (which `infer_type_from_path` maps to nothing), so it hits the
  `INVALID-TYPE` early return; `assert_accepts` (no `OBSOLETE-LEGACY-KEY`). Asserts
  the boundary documented in §1 rather than leaving it incidental.

Follow the file's existing fixture/assertion style (planted-input →
expected-violation-code, via `emit_valid` + `assert_rejects`/`assert_accepts`),
and place the cases in the `=== Failure-mode fixtures ===` section beside
`bad-ownid`.

#### 3. Corpus hygiene — remove dead `ticket: null` frontmatter

**Files** (frontmatter `ticket: null` at line 5; mechanical line removal):

```
meta/plans/2026-04-07-add-accelerator-prefix-to-default-agent-names.md
meta/plans/2026-04-07-fix-tmp-directory-usage-in-pr-skills.md
meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md
meta/plans/2026-04-18-meta-visualiser-phase-2-server-bootstrap.md
meta/plans/2026-04-21-meta-visualiser-phase-3-file-driver-indexer-api.md
meta/plans/2026-04-26-meta-visualiser-phase-8-kanban-write-path.md
meta/plans/2026-04-27-meta-visualiser-phase-9-cross-references-and-wiki-links.md
meta/plans/2026-04-28-meta-visualiser-phase-10-error-handling-accessibility-polish.md
meta/plans/2026-04-29-meta-visualiser-phase-11-testing.md
meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md
```

**Changes**: Delete the `ticket: null` line from each file's frontmatter only.

⚠️ **Do not scrub by raw `grep` match.** `grep -rnE '^\s*ticket(_id)?:' meta/`
returns ~20 matches, but most are **body code-fence examples**, not frontmatter
— verified at `b2f39a4`: e.g. `…phase-3-file-driver-indexer-api.md:2925`
(`ticket: None,`), `…phase-6-lifecycle…md:548/714/745`,
`…ticket-management-phase-1-foundation.md:640` and `list-and-update-tickets.md:172`
(`ticket_id: NNNN`), `validation-crossref-frontmatter.md:465`,
`ticket-review-core.md:995`, `…ticket-management-skills.md:490`. These are
documentation and **must be left alone**.

Compute the target set **unambiguously** — only `ticket:`/`ticket_id:` lines
*inside the leading `---` frontmatter fence* — rather than filtering a flat grep
by eye. Use one deterministic per-file command, e.g. an `awk` that tracks fence
state and deletes obsolete-key lines only while inside the first `---`…`---`:

```bash
for f in <the 10 files>; do
  awk '
    /^---[[:space:]]*$/ { fence++; print; next }
    fence==1 && /^[[:space:]]*ticket(_id)?[[:space:]]*:/ { next }  # drop inside fence
    { print }
  ' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done
```

The 10 frontmatter targets are exactly the line-5 `ticket: null` hits listed
above; the validator's own `extract_frontmatter` is the authoritative definition
of "inside the fence". After scrubbing, review the **full** `jj diff` (not just a
spot-check) to confirm every removed line is a line-5 `ticket: null` and nothing
below a closing fence changed — cheap insurance against a malformed/unterminated
fence in any one file.

### Success Criteria:

#### Automated Verification:

- [x] Validator unit suite passes: `bash scripts/test-validate-corpus-frontmatter.sh`
- [x] Whole-corpus validation is clean: `bash scripts/validate-corpus-frontmatter.sh meta/` exits 0
- [x] No obsolete keys remain in any frontmatter: re-running the validator over
      `meta/` reports zero `OBSOLETE-LEGACY-KEY` violations
- [x] Config integration suite passes: `mise run test:integration:config`
- [x] Full read-only CI mirror passes: `mise run check` (verified via the two
      components Phase 1 touches — `scripts:check` + `build-system:check` both
      exit 0; Rust is untouched this phase)

> **How the gate is CI-enforced.** The whole-corpus run is not a standalone task —
> it is already embedded in the validator's own suite: `test-validate-corpus-
> frontmatter.sh:184-191` runs `validate-corpus-frontmatter.sh "$ROOT/meta"`
> against the **real** corpus and `assert_eq`s exit 0 ("migrated corpus validates
> clean"). That suite runs under `test:integration:config` ⊂ `mise run check`, so a
> missed `ticket: null` line (or any future obsolete-key regression) **fails CI**,
> not merely the synthetic-fixture cases. To anchor this by identity rather than
> only the `_EXPECTED_CONFIG_SUITES` count floor, add
> `scripts/test-validate-corpus-frontmatter.sh` to `_REQUIRED_CONFIG_SUITES`
> (`tasks/test/integration.py:21`) so a rename off the `test-*.sh` convention can
> never silently drop the gate.

#### Manual Verification:

- [ ] Spot-check two scrubbed plans render correctly in the visualiser (no
      frontmatter parse regression from the removed line)
- [ ] Confirm the gate is a genuinely observable signal: planting a `ticket_id:`
      into a scratch frontmatter and running the validator fails loudly with the
      file path and `OBSOLETE-LEGACY-KEY` code

---

## Phase 2: `frontmatter.rs` — Remove the `work-item:` and `ticket:` Arms

### Overview

Remove the two legacy fallback arms from `read_ref_keys`, leaving
`work_item_id:` (preferred) and `target:` (typed) as the only arms.

### Changes Required:

#### 1. Chain surgery in `read_ref_keys`

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
**Changes**: Delete the `work-item:` arm (`:350-364`, incl. its `tracing::warn!`)
and the `ticket:` arm (`:365-368`). Re-chain the `target:` arm (`:369`) directly
off the `work_item_id:` arm (`:346-349`). Rewrite the prose so no removed key is
named in surviving code:

- Function doc comment (`:312-320`) → reduce to: reads `work_item_id:` (preferred)
  or, failing that, the typed `target:` (ADR-0034 `doc-type:id` form) as the
  cross-reference scalar, then unconditionally aggregates `parent:` and `related:`.
- Preference comment (`:343-345`) → state simply that `work_item_id:` is preferred
  and `target:` is the typed fallback; drop the `work-item:`/`ticket:` precedence
  prose.
- Delete the removed-arm comment (`:351-355`) outright rather than "revising" it —
  it describes the now-deleted `work-item:` arm.

```rust
if let Some(v) = m.get("work_item_id") {
    if let Some(s) = extract_scalar(v) {
        refs.push(s);
    }
} else if let Some(v) = m.get("target") {
    // Final fallback per ADR-0034 §"Forms": typed-linkage `doc-type:id` form.
    if let Some(s) = extract_scalar(v) {
        if let Some(crate::typed_ref::TypedRef::WorkItem(id)) =
            crate::typed_ref::parse_typed_ref(&s)
        {
            refs.push(id);
        }
    }
}
// `parent:` and `related:` aggregation below is unchanged.
```

#### 2. Tests (edit first)

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs` (test module)
**Changes** (re-derive exact line numbers against current source):
- DELETE the deprecation test
  `read_ref_keys_legacy_work_item_arm_emits_deprecation_warning`
  (~`:572-586`, the file's only `capture_logs` consumer).
- DELETE the removed-resolution tests:
  `read_ref_keys_reads_legacy_work_item_key_via_transitional_fallback`,
  `read_ref_keys_reads_legacy_ticket_key`,
  `read_ref_keys_numeric_ticket_value_is_stringified`.
- DELETE the precedence tests whose legacy value goes inert:
  `read_ref_keys_prefers_work_item_id_over_transitional_work_item`,
  `read_ref_keys_with_both_legacy_and_current_keys_prefers_current`.
- KEEP: the six `target:` tests, `read_ref_keys_reads_work_item_id_key`, the
  parent/related/empty/equivalence tests, **and explicitly
  `read_ref_keys_prefers_work_item_id_alias_over_target` (~`:618`)** — after the
  re-chain this is the sole guard that `work_item_id:` still wins over `target:`
  (the one behavioural change in the chain), so it must not be swept up in the
  deletions.
- Apply the order-independent **`capture_logs` rule**: after these deletions,
  `grep -rn capture_logs src/`; if no callers remain, remove the entire capture
  harness (`log.rs:102-164`, see Implementation Approach), not just the fn.

### Success Criteria:

#### Automated Verification:

- [ ] Server unit tests pass: `mise run test:unit:visualiser`
- [ ] Both feature modes green:
      `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --no-default-features --features dev-frontend`
      and the default (`embed-dist`) test run
- [ ] No `work-item:` or `ticket:` frontmatter-key arm remains:
      `grep -nE 'get\("work-item"\)|get\("ticket"\)' skills/visualisation/visualise/server/src/frontmatter.rs` returns nothing
- [ ] Full read-only CI mirror passes: `mise run check`

#### Manual Verification:

- [ ] In the running visualiser, a doc with a `work_item_id:` cross-ref still
      aggregates correctly; a doc relying only on a legacy `work-item:`/`ticket:`
      key no longer resolves (expected post-removal behaviour)
- [ ] Resolution **shift** (not just disappearance): a doc carrying *both* a
      legacy `work-item:`/`ticket:` key *and* a `target:` (no `work_item_id:`) now
      resolves via `target:` — the resolved ref switches from the legacy id to the
      `target:` id rather than vanishing. Confirm this is the observed (intended)
      behaviour, not a regression.

---

## Phase 3: `cluster_key.rs` — Remove the Legacy Branch and Rename

### Overview

Remove the legacy `work_item_id:` branch from `parent_or_legacy_id`, leaving
`parent:` as the only resolution path, and rename the function to
`cluster_key_from_parent` to reflect that.

### Changes Required:

#### 1. Remove the legacy branch and rename

**File**: `skills/visualisation/visualise/server/src/cluster_key.rs`
**Changes**: Delete the legacy `work_item_id:` branch (`:138-155`, incl. the bare
`warn!` at `:148-152`), so the function falls straight through to `None` after
the `parent:` block. Rename `parent_or_legacy_id` → **`cluster_key_from_parent`**
(describes what it does — derives a cluster key from the parent linkage — and
avoids overloading the `*_id` field/key vocabulary); update the declaration
(`:128`) and the single call site in `walk` (`:77`). Revise the doc comment
(`:123-127`) to drop the legacy shapes (items 3 and 4). Keep `id_from_value`
(`:164-178`) and the `warn!` import (`:16`, still used at `:61`).

#### 2. Tests (edit first)

**File**: `skills/visualisation/visualise/server/src/cluster_key.rs` (test module)
**Changes** (re-derive exact line numbers):
- DELETE `legacy_work_item_id_branch_emits_deprecation_warning`
  (~`:293-317`, `capture_logs` consumer).
- RETARGET (do not delete) the legacy-branch resolution tests
  `plan_with_work_item_id_frontmatter_resolves` (~`:278`) and
  `plan_with_path_shape_work_item_id_resolves` (~`:320`) — these assert the
  *removed* legacy branch and go red. Re-point them to `parent:`-based resolution.
  In particular `plan_with_path_shape_work_item_id_resolves` is the only
  cluster-level test of `id_from_value`'s surviving `TypedRef::Path` →
  `extract_id` branch, so retarget it to a path-shape `parent:`
  (e.g. `parent: "meta/work/0033-foo.md"`) to keep that branch covered — deleting
  it would silently drop coverage of live code.
- KEEP (resolve via `parent:` or WorkItems own-id, unaffected):
  `plan_with_empty_work_item_id_and_no_parent_resolves_none` (~`:336`),
  `plan_with_typed_work_item_parent_resolves`, `plan_with_bare_parent_id_resolves`,
  `parent_typed_form_resolves_same_as_bare_id`, and the review/validation
  transitive tests.
- RETARGET (naming/comment alignment only — they already assert canonical
  `target:` resolution): `work_item_review_target_path_resolves_to_work_item_id`
  (~`:402`) and `work_item_review_typed_work_item_target_short_circuits` (~`:432`).
- Apply the order-independent **`capture_logs` rule**.

### Success Criteria:

#### Automated Verification:

- [ ] Server unit tests pass: `mise run test:unit:visualiser` (both modes)
- [ ] No `parent_or_legacy_id` symbol remains in the tree:
      `grep -rn parent_or_legacy_id skills/visualisation/visualise/server/src/` returns nothing
- [ ] No legacy `work_item_id:` clustering branch remains (verified by reading
      the renamed function — single `parent:` path → `None`)
- [ ] Full read-only CI mirror passes: `mise run check`

#### Manual Verification:

- [ ] In the running visualiser, plan/research/PR-description clustering still
      groups under the correct work item via `parent:`

---

## Phase 4: `indexer.rs` — Remove the Legacy Identity Arm and Filename Fallback

### Overview

Reduce the work-item identity chain to `id:` → `None`, removing the legacy
`work_item_id:` identity arm and the filename fallback.

### Changes Required:

#### 1. Collapse the identity chain

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: In `build_entry`'s `DocTypeKey::WorkItems` branch, delete the legacy
`work_item_id:` arm (`:1384-1390`) and the filename fallback (`:1391-1397`), so
the chain is `id:` → `None`. **Rewrite** the identity-resolution doc comment
(`:1356-1364`) — do not leave the old three-source/per-source-deprecation-warn
prose: state that `id:` (routed through `read_fm_id` → `normalise_id`) is the
**sole** work-item identity source and that its absence yields `None` (the entry
is excluded from the index). **Keep** the shape-validation `tracing::warn!`
(`:1372-1377`, inside `read_fm_id`) — it guards the surviving `id:` path.
`extract_id` stays (still used at `cluster_key.rs:173`).

Do **not** carry a `(was: …)` parenthetical describing the removed chain into the
surviving code — a comment re-narrating fallbacks that no longer exist is worse
than none:

```rust
// `id:` is the sole work-item identity source (validated via normalise_id);
// absence → None, i.e. the entry is excluded from the index.
let id = read_fm_id("id");
```

#### 2. Tests (edit first)

**File**: `skills/visualisation/visualise/server/src/indexer.rs` (test module)
**Changes** (re-derive exact line numbers):
- DELETE the two deprecation tests
  `legacy_work_item_id_key_emits_deprecation_warning` (~`:3466-3483`) and
  `filename_fallback_emits_deprecation_warning` (~`:3485-3502`) — the remaining
  `capture_logs` consumers.
- Handle the legacy/filename identity tests as follows (RETARGET is the default;
  delete only where the test's *entire* reason to exist was the legacy/filename
  source):
  - `work_item_id_uses_frontmatter_when_present` (~`:3393`) — RETARGET to assert
    `id:` is read when present (this *becomes* a near-duplicate of the kept
    primary test; delete only if genuinely redundant with
    `work_item_identity_resolves_via_unified_id_key`).
  - `work_item_id_falls_back_to_filename_when_frontmatter_absent` (~`:3420`) —
    DELETE (its sole purpose is the removed filename fallback).
  - `work_item_id_frontmatter_bare_digits_applies_project_code` (~`:3504`) —
    **RETARGET to `id:`** (e.g. `id: "42"` → project-code applied). This is the
    only direct coverage of `normalise_id`'s project-code path; do not delete.
  - `work_item_id_frontmatter_foreign_prefix_passes_through` (~`:3536`) —
    **RETARGET to `id:`** (e.g. `id: "OPS-7"` passes through). Only direct
    coverage of the foreign-prefix passthrough; do not delete.
  - `work_item_id_frontmatter_shape_invalid_falls_back_to_filename` (~`:3570`) —
    **RETARGET to `id:` and INVERT the assertion**: a shape-invalid `id:`
    (e.g. `id: "PROJ-1.2"`) now resolves to **`None`** (no filename fallback to
    catch it), and the shape-validation `warn!` (`:1372-1377`) still fires. Do
    **not** mechanically keep the old `Some(...)` expectation — the outcome flips.
- RETARGET and RENAME
  `work_item_id_none_when_neither_frontmatter_nor_filename_matches` (~`:3599`) →
  e.g. `work_item_id_none_when_id_absent`: keep it as the negative contract
  (`id:` absent → `None`) with a name that no longer implies filename/legacy
  sources are consulted.
- KEEP `work_item_identity_resolves_via_unified_id_key` (~`:3451-3464`) — the
  post-removal contract (`id:` is the sole resolver) — and all
  `target_path_from_entry` tests.
- Apply the order-independent **`capture_logs` rule** — this phase removes the
  last two callers, so it will typically be the phase that removes the
  `capture_logs` fn.
- Note on the surviving shape-validation `warn!`: deleting the four deprecation
  tests removes the suite's only log assertions, so the shape-validation `warn!`
  (`:1372-1377`) becomes unexercised. The default here is to let it go untested
  (removing `capture_logs` is the goal, and the inverted shape-invalid test above
  already proves the *behaviour* — invalid `id:` → `None`). If guarding the warn's
  emission is judged worthwhile, retain `capture_logs` and retarget one deleted
  deprecation test to assert the warn fires on an invalid `id:`; otherwise treat
  the warn as intentionally untested.

### Success Criteria:

#### Automated Verification:

- [ ] Server unit tests pass: `mise run test:unit:visualiser` (both modes)
- [ ] No legacy identity arm / filename fallback remains: reading
      `build_entry`'s `DocTypeKey::WorkItems` branch shows `id:` → `None` only;
      `grep -n 'work-item identity resolved via\|filename fallback' skills/visualisation/visualise/server/src/indexer.rs` returns nothing
- [ ] No dead `capture_logs`: once the last caller is removed,
      `grep -rn capture_logs skills/visualisation/visualise/server/src/` returns
      **zero matches** (the `log.rs` definition removed by the order-independent
      rule). If a server phase has not yet merged, the only permitted matches are
      the `log.rs` definition plus its remaining live callers
- [ ] Full read-only CI mirror passes: `mise run check`

#### Manual Verification:

- [ ] Using a **real** `meta/work/` file (not only a synthetic fixture), confirm a
      synced work-item whose filename does not encode its id but which carries
      `id:` resolves correctly in the visualiser; a file with neither is simply
      absent (no silent filename-derived identity). This verifies the removed
      degradation path is genuinely dead corpus-wide, not just in a fixture.

---

## Testing Strategy

### Unit Tests:

- **Validator** (`test-validate-corpus-frontmatter.sh`): planted `ticket:` /
  `ticket_id:` frontmatter → `OBSOLETE-LEGACY-KEY` + non-zero exit; clean fixture
  → pass. Mirror the suite's existing fixture/assertion conventions.
- **Server** (cargo `--tests`): each phase deletes the legacy-pinning and
  deprecation-warning tests for its arm and retargets survivors so they assert
  *canonical-only* resolution. The kept tests
  (`read_ref_keys_reads_work_item_id_key`, the `target:` tests,
  `work_item_identity_resolves_via_unified_id_key`, the `parent:`-based
  clustering tests) become the post-removal contract.

### Integration Tests:

- `mise run test:integration:config` exercises the corpus validator as a CI gate
  (the migration-completion gate's home). The whole-corpus
  `validate-corpus-frontmatter.sh meta/` run proves **this repo's corpus is
  migration-complete** — a release prerequisite, not a cross-repo guarantee. It is
  structurally blind to external/userspace corpora; that is an accepted scoping
  limit (see Migration Notes), since migrate-on-use is advisory and no consuming
  corpus is observable from here.

### Manual Testing Steps:

1. Launch the visualiser against this repo's `meta/` and confirm work-item
   clustering, cross-references, and the library/kanban views are unchanged.
2. Confirm a doc relying solely on a legacy `work-item:`/`ticket:` key no longer
   resolves (expected) while `work_item_id:` and `target:` references still do.
3. Plant an obsolete `ticket_id:` frontmatter key into a scratch file and confirm
   `mise run check` fails with a clear `OBSOLETE-LEGACY-KEY` diagnostic.

## Performance Considerations

None. The validator already parses each file's frontmatter once; the new check is
a fixed two-key membership test per file. The server changes remove branches,
shrinking hot-path code.

## Migration Notes

This plan *is* the contract step of the 0070 migration. No new data migration is
introduced; the only corpus mutation is the mechanical removal of 10 dead
`ticket: null` frontmatter lines (Phase 1). Recovery for any phase is via VCS
revert — each phase is an isolated, independently mergeable change.

### Accepted breakage for un-migrated external corpora

Removing the fallback arms changes the failure mode for any **un-migrated**
consuming corpus from "resolves (with a `tracing::warn!`, except `ticket:` which
was always silent)" to "silently does not resolve". The sharpest case is the
filename fallback (Phase 4): migration `0007` is what backfills `id:` into
work-item frontmatter, so an un-migrated work item carrying only a
filename-encoded id resolves to `None` and is simply **absent** from the index
(no panic, no error — graceful degradation, but invisible). Cross-reference
aggregation for legacy `work-item:`/`ticket:` keys (Phase 2) disappears the same
way.

This is the **accepted cost of the contract phase**, not a defect:

- The visualiser binary is version-locked to the plugin
  (`launch-server.sh` rejects manifest-vs-plugin version drift), so a consumer
  always receives the arm-removed binary and the completing migrations (`0001`,
  `0007`) in the *same* plugin version — they never diverge on the released-binary
  path. The two documented override paths (`ACCELERATOR_VISUALISER_BIN` env var,
  `visualiser.binary` config) bypass the drift check and are an explicit opt-out of
  the version coupling — drift there is the user's responsibility.
- Migrate-on-use is deliberately advisory (`migrate/SKILL.md`), and no external
  corpus is observable from this repo, so the in-repo `meta/` gate is the
  accepted **migration-completion proxy for this repo only** — not a cross-repo
  guarantee. No runtime migration diagnostic is added; the contract phase stays a
  pure removal.
- In-repo blast radius is nil: all current `meta/work/` files carry `id:` (the
  `0007` backfill ran here), so no in-repo work item disappears.

The release that ships this change carries a **BREAKING** CHANGELOG entry —
mirroring the `0001` `ticket`→`work-item` precedent — naming the **observable
symptom** so a confused consumer can self-diagnose without reading source: "work
items / cross-references keyed only by legacy `work-item:`/`ticket:`/filename will
silently disappear from the visualiser library/kanban until `/accelerator:migrate`
(migrations `0001`/`0007`) is run." Version coherence across
`plugin.json` / server `Cargo.toml` / `checksums.json` is enforced by the
existing release tooling.

## References

- Original work item: `meta/work/0102-remove-visualiser-legacy-linkage-fallback-arms.md`
- Related research: `meta/research/codebase/2026-06-15-0102-remove-visualiser-legacy-linkage-fallback-arms.md`
- Originating work item / plan: `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`,
  `meta/plans/2026-06-07-0070-meta-corpus-unified-schema-migration.md` (Phase 5 §5)
- ADRs: `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`,
  `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- Migrations: `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`,
  `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
- Removal sites: `frontmatter.rs:321-391`, `cluster_key.rs:128-157`,
  `indexer.rs:1382-1400`; gate: `scripts/validate-corpus-frontmatter.sh`,
  `tasks/test/integration.py:46`

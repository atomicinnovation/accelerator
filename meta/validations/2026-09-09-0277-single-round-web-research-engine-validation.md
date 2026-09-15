---
type: "plan-validation"
id: "2026-09-09-0277-single-round-web-research-engine-validation"
title: "Validation Report: Single-Round Web Research Engine Implementation Plan"
date: "2026-09-15T13:54:36+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-09-09-0277-single-round-web-research-engine"
tags: ["research", "skills", "deep-research", "corpus", "topic-research"]
last_updated: "2026-09-15T13:54:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Single-Round Web Research Engine

All seven phases are implemented and every anchor the plan specifies is present
in the tree. The Rust workspace, visualiser server, and frontend read-only
checks all exit 0, and the plan's own behavioural tests pass (frontmatter
goldens 27/27, migration_0010 7/7, resolve goldens 8/9). Two deterministic
`mise run test` failures are attributable to this engine's co-land with 0278 and
contradict Phase 7's `mise run test` checkbox: a conformance skill-count drift
and a stale resolver exit-code case. The full `brief → outline → conduct →
synthesise` loop remains manually unverified (live web + the 0278 co-land), as
the plan scopes it.

### Implementation Status

- ✅ Phase 1: `(type, kind)` schema and template resolution — `SchemaRow.kind`,
  composite `row_for` with type fallback, `UnknownKind` split, TSV `kind`
  column, template resolver `--kind`.
- ✅ Phase 2: `paths.research_topics` config key — catalogue entry + goldens.
- ✅ Phase 3: `topic-research` typed-linkage source type — present in
  `LINKAGE_SOURCE_TYPES`, `SOURCE_TYPES`, and `linkage.rs` `TYPE_PAIRS`.
- ✅ Phase 4: general `corpus resolve --type <type> <slug>` — `corpus/resolve.rs`
  domain + `corpus-cli/resolve.rs` adapter with the exit-code taxonomy.
- ✅ Phase 5: five `(topic-research, <kind>)` rows + five templates + committed
  full-set fixture with negative cases.
- ✅ Phase 6: generic `researcher` agent (`tools: WebSearch, WebFetch, Write,
  Read` — no `Bash`), `web-profile`, `finding-outputter`, agent key registered.
- ⚠️ Phase 7: `research-topic` skill present and dispatching four verbs; the
  `research_status` field is fully collapsed to base `status` (the 0278 delta).
  Automated `mise run test` checkbox no longer holds (see Potential Issues).

### Automated Verification Results

- ✅ `mise run cli:check` — exit 0 (whole Rust workspace: rustfmt + clippy).
- ✅ `mise run server:check` — exit 0.
- ✅ `mise run frontend:check` — exit 0.
- ✅ `cargo test -p accelerator-corpus --test frontmatter_goldens` — the
  committed set validates clean; `MissingExtra` (missing `primary`),
  `UnknownKind` (bogus/missing kind), `OBSOLETE-LEGACY-KEY` (retired
  `research_status`), whole-corpus dangling-ref resolution + negative control,
  and a `DuplicateId` two-set guard all pass.
- ✅ `cargo test --test resolve_goldens` — 8/9; the exit-code taxonomy
  (0/1/2/3/6) and the symlinked-root case pass. One case fails (below).
- ✅ `cargo test --test migration_0010` — 7/7 (co-land sibling, not owned here).
- ❌ `mise run test` — exit 1. Three failing conditions (below); the suite's
  fail-fast aborts `test:unit:cli` mid-run after conformance fails first, so the
  other two surface only when their targets are run directly.
- ❌ `mise run check` — exit 1, on `deny:check` `RUSTSEC-2026-0285` only
  (orthogonal; see Recommendations).

### Code Review Findings

#### Matches Plan

- The `(type, kind)` mechanism, config key, linkage source type, resolver, five
  templates + rows, agent triad, and skill body all match the plan's file-level
  spec. `SCHEMA` is length 18; `researcher` carries no `Bash`; the lifecycle
  vocab-drift fixture (`topic-research-status-vocab.json`) is emitted by Rust and
  read by the frontend test.

#### Deviations from Plan

- The corpus-cli two-set `DuplicateId` guard is constructed in a tempdir
  (`topic-two-sets` in `frontmatter_goldens.rs`) rather than as a committed
  `another-subject/` fixture (Phase 5 change 7 / the 0278 Phase 2 spec).
  Functionally equivalent — the guard exists and is exercised.
- The finding-outputter was later refactored to reference the
  `topic-research-finding` template, dropping its inline example frontmatter
  scaffold. This is the mechanical cause of the conformance failure below.

#### Potential Issues

- ❌ `tests/integration/conformance::test_producer_set_reconciliation` fails
  deterministically: `discovery returned 18 producing SKILL.md files, expected
  19`. Phase 7 (`Add the research-topic skill`) bumped this assertion `18 → 19`
  on the assumption the finding-outputter stayed discoverable; the later
  outputter refactor (`Load the topic-research-finding template in the finding
  outputter`) removed its `producer:`/`schema_version:` scaffold, so discovery
  now returns 18. The `EXCLUDED` tuple still lists the outputter, but discovery
  no longer surfaces it. Fix: set the assertion to `18` (and the message), or
  restore a discoverable scaffold.
- ❌ `resolve_goldens::an_unregistered_type_is_a_distinct_unknown_type_code`
  fails: `assert Some(4) == Some(3)`. The test passes `--type topic-research` as
  its stand-in for an unregistered type and asserts exit 4 + `E_RESOLVE_UNKNOWN_
  TYPE` — Phase 4's manual note even says "exits 4 (expected until 0278)". Now
  that 0278 has registered `topic-research`, resolving it against an empty tree
  correctly returns exit 3 + `E_RESOLVE_NOT_FOUND`. The CLI behaviour is right;
  the test data is stale. Fix: point the exit-4 case at a genuinely-unregistered
  type token (e.g. `no-such-type`). This is a co-land regression — a 0277 test
  whose premise 0278's registration invalidates.

### Manual Testing Required

The live-web loop (Phase 7 manual criteria) cannot run in this environment —
`WebFetch`/`WebSearch` against live sources plus the 0278 doc-type registration
for non-`brief` slug resolution:

1. `brief` → `manifest.md` + `brief.md`, `status: briefed`, `primary:
   brief.md`, `source_profiles: ["web"]`; both validate.
2. `outline` (post-0278) → `## Round 1` checklist ≤ 8 focus areas, `status:
   outlined`; a hand-edit honoured by `conduct`.
3. `conduct` → one immutable tier-tagged finding per focus area, checkboxes
   flipped, `status: researching`, `round_count: 1`, `finding_count` = retained
   count.
4. `synthesise` → `synthesis.md` with tiers carried forward, `status:
   synthesised`, `primary` flipped.
5. Exactly one generic `researcher` spawned with the web profile injected.

### Recommendations

- Two one-line co-land fixes restore `mise run test` for this engine's scope,
  and both are the actionable items this validation surfaces: set the conformance
  count assertion to `18` (finding-outputter is no longer a discovered producer),
  and repoint the `resolve_goldens` exit-4 case off `topic-research` (now
  registered) onto a genuinely-unregistered type token.
- ⚠️ Two aggregate reds are orthogonal to 0277 and not its to fix: the
  `RUSTSEC-2026-0285` `rustls 0.23.41` advisory (`deny:check`, newly disclosed;
  fix by `cargo update -p rustls` to ≥ 0.23.45) and the pre-existing
  `work-item:0286` DUPLICATE-ID in `this_repositorys_own_corpus_is_clean` (two
  `meta/work/0286-*.md` files claim `id: "0286"`).
- Run the live-web loop manually at the co-land to close Phase 7 before promoting
  the work item past its acceptance gate.

### Remediation applied

A follow-up pass over the whole PR stack (2026-09-15) drove every CI check
green. It also surfaced a **third** `mkrknrnu`-caused regression this validation
missed: `corpus-adapters::research_agent_contract` asserts the finding
outputter's frontmatter scaffold matches the schema row, and the same
template-delegation refactor removed that scaffold. Fixes, applied at their
origin layers rather than in this PR:

- #117 (0277 engine) — conformance count → `18`; `research_agent_contract`
  repointed to the `topic-research-finding` template (the field contract under
  delegation); `rustls` pinned `=0.23.45` to clear `RUSTSEC-2026-0285`.
- #118 (0278 visualiser) — `resolve_goldens` exit-4 case repointed to an
  unregistered type; the duplicate `work-item:0286` (the inventory-design bug)
  renumbered to `0289`.

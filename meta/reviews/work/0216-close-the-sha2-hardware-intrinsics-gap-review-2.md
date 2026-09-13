---
type: "work-item-review"
id: "0216-close-the-sha2-hardware-intrinsics-gap-review-2"
title: "Work Item Review: Close the sha2 hardware-intrinsics gap"
date: "2026-09-10T23:02:10+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0216"
work_item_id: "0216"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 2
review_pass: 3
tags: []
last_updated: "2026-09-10T23:33:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Close the sha2 hardware-intrinsics gap

**Verdict:** REVISE

The work item remains structurally excellent — every section is present and densely
populated, AC 1 is a fully specified Given/When/Then with a deliberate-margin throughput
floor, and the 0215 coupling is captured bidirectionally. Two majors pull it to REVISE: the
Technical Notes still read as a spike-era `asm`-vs-soft investigation that contradicts the
now-resolved 0.11 decision (and 0.11 removes the `asm` feature the notes tell the implementer
to benchmark), and the recent expansion bundles a workspace-wide cargo-deny no-duplicates
cleanup — an open-ended, independently deliverable concern — into what was a tightly scoped
backend switch. Review 1 (verdict APPROVE, after resolving the AC-threshold and
0215-sequencing concerns) predates both: the Technical Notes drift was masked while the notes
matched the then-open remedy question, and the scope expansion landed after that review closed.

### Cross-Cutting Themes

- **Technical Notes contradict the resolved 0.11 decision** (flagged by: clarity, completeness)
  — the "First step: benchmark with and without `features = ["asm"]`" instruction and the
  un-ranked two-remedy list survive from the spike, now foreclosed by the Requirements/Open
  Questions resolution and by 0.11 having removed `asm`. This is the single highest-priority fix.
- **The bundled cargo-deny cleanup strains scope** (flagged by: scope, testability) — the
  `multiple-versions = "deny"` flip plus resolve-or-justify of every pre-existing duplicate
  (`digest`, `cpufeatures`, reqwest/rustls/gix/jj) is separable from the sha2 switch, has no
  disposition under the AC 7 fallback branch, and its server-pin intent is not asserted by any AC.
- **The cargo-deny end state is under-specified across AC 4 and AC 5** (flagged by: clarity,
  testability) — AC 4's bare "cargo-deny passes" does not say whether it means the current
  lenient config or the post-change strict one AC 5 introduces, and AC 5's "justified" skips are
  a soft qualifier behind the hard exit-0 gate.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: asm-feature benchmark instruction contradicts the resolved 0.11 decision
  **Location**: Technical Notes
  Technical Notes gives as the "First step" to benchmark `verifier::sha256_hex` with and without
  `features = ["asm"]`, but Requirements and Open Questions foreclose the asm route and the
  item's own References note `asm` is removed in `sha2` 0.11 — the version the reader is told to
  adopt. An implementer following the notes literally would benchmark a feature that does not
  exist in the chosen version.

- 🟡 **Scope**: cargo-deny no-duplicates cleanup is a separable concern bundled into the switch
  **Location**: Requirements (cargo-deny cleanup) / Acceptance Criteria (AC 5)
  Only the first-party `sha2` collapse follows causally from the bump; the pre-existing `digest`,
  `cpufeatures`, and reqwest/rustls/gix/jj duplicates predate this change and would need resolving
  regardless. The policy flip plus cross-tree audit is independently deliverable, testable, and
  rollback-able, and its effort is open-ended ("every remaining duplicate"), inflating the task and
  coupling the sha2 outcome to an audit whose churn could complicate a clean backend change.

#### Minor

- 🔵 **Clarity**: AC 4's "cargo-deny passes" is ambiguous about which config state
  **Location**: Acceptance Criteria (AC 4)
  AC 4 says "cargo-deny passes" while AC 5 separately requires `cli/deny.toml` to set
  `multiple-versions = "deny"` and pass under it. Read alone, AC 4 does not say whether it means
  today's lenient config or the post-change strict one, so a verifier could sign it off under the
  wrong config state.

- 🔵 **Scope**: the two threads do not share one "done" state
  **Location**: Acceptance Criteria (AC 7) / Open Questions
  AC 7 defines an accepted-fallback state in which the hardware backend is abandoned and 0215 is
  the chosen route, but the bundled cargo-deny policy flip and audit (AC 5) have no defined
  disposition in that branch — they would be orphaned or left half-applied if the sha2 thread exits
  via its fallback.

- 🔵 **Dependency**: inheriting member crates not named as affected consumers
  **Location**: Requirements
  The bump changes the `[workspace.dependencies]` `sha2` pin inherited by launcher, work-adapters,
  jira-client, design-adapters, remote-projection and the server. Only the launcher call site and
  the server literal are named; the other inheriting crates whose backend and API surface this bump
  silently changes are not recorded, so the shared-artefact blast radius is implied rather than
  captured.

- 🔵 **Dependency**: the 0217 reciprocal edge for the deferred musl verification
  **Location**: Dependencies
  AC 3 defers all Linux/musl throughput verification to 0217, making 0217 the item that proves the
  deliverable on the musl statics. The backing research flagged that 0217 carried no reciprocal
  `work-item:0216` edge or sha2-backend criterion. _(Note: this was landed on 0217 after the review
  agents read the source — 0217 now carries the gated-by edge, the aarch64-musl `sha256_hex`
  criterion, and a run-after note. Recommend referencing that from 0216 so the coupling is visible
  here too.)_

- 🔵 **Testability**: AC 3 verifies musl by proxy inspection, not an active-backend check
  **Location**: Acceptance Criteria (AC 3)
  AC 3 verifies the musl target only by inspection — no `-C target-feature=+sha2`, `sha2` 0.11
  present — and defers throughput to 0217. The research notes the "no forcing flag" half is
  trivially satisfiable because nothing sets that flag today, so the observable evidence is a proxy
  for the property the item claims (the runtime-detected path is taken and effective).

- 🔵 **Testability**: subjective qualifiers have no defined threshold
  **Location**: Acceptance Criteria (AC 5, AC 7)
  AC 5's "justified" skip entries and AC 7's "genuinely cannot be made clean" are judgements a
  verifier cannot objectively settle. The hard gates (cargo-deny exit 0 under the strict setting)
  carry the testable weight, but the soft qualifiers leave the fallback trigger open to "could be
  argued either way" — the loophole most likely to close the item without the committed outcome.

#### Suggestions

- 🔵 **Completeness**: residual spike-era framing in Technical Notes
  **Location**: Technical Notes
  The item was converted from spike to task with the 0.11 bump resolved, yet Technical Notes still
  read as an open investigation ("benchmark with and without asm", an un-ranked "Remedy (i)/(ii)"
  list). Trim to the committed 0.11 approach, folding the `asm` alternative into the rejected-note
  already in Open Questions. (Reinforces the clarity major.)

- 🔵 **Dependency**: the rust-embed-utils transitive coupling is not surfaced
  **Location**: Requirements
  The duplicate-free collapse relies on transitive `rust-embed-utils` continuing to be the source of
  `sha2` 0.11.0; a future `rust-embed` change moving that version would reintroduce a duplicate under
  the new deny policy. Note this reliance so the coupling is visible if it later diverges.

- 🔵 **Scope**: `kind: task` strained by the open-ended audit
  **Location**: Requirements / Frontmatter: kind
  The "resolve or skip/skip-tree every existing duplicate" audit spans heavy unrelated trees whose
  effort is uncertain, straining the atomic-increment expectation of a task. If it stays, bound it by
  listing the specific duplicates to resolve versus justify so the scope is finite and estimable.

- 🔵 **Testability**: the server pin conversion is only indirectly gated
  **Location**: Requirements / Acceptance Criteria
  No AC directly asserts the `sha2 = { workspace = true }` conversion; AC 5's cargo-deny check would
  also pass with a server literal pinned to `sha2 = "0.11"`, leaving the single-source-of-truth intent
  unmet yet the criteria satisfied. Add a short inspection criterion for the server declaration.

- 🔵 **Testability**: AC 2's "no new warnings" has no captured baseline
  **Location**: Acceptance Criteria (AC 2)
  The "no warnings beyond an unchanged build" comparison depends on a pre-change per-target warning
  set that is never captured. Either require it as evidence, or — if the four targets build
  warning-free today — simplify to "exits 0 with no warnings".

### Strengths

- ✅ Every standard section is present and substantively populated; frontmatter is complete and
  well-formed (recognised kind, status, priority, parent, derived_from, rich relates_to).
- ✅ AC 1 is a fully specified verification — named input (the 2.49 MB `vcs` sub-binary), action, an
  absolute ≥~1,390 MB/s pass threshold, and a diagnostic band telling the verifier that a ~500 MB/s
  result means the soft backend is still selected.
- ✅ The ≥2.5× floor is set with deliberate margin between the soft (~575 MB/s) and hardware
  (~2,000 MB/s) bands, so host-load noise cannot produce an ambiguous pass/fail.
- ✅ The 0215 coupling is captured bidirectionally and precisely, with an explicit
  no-parallel-scheduling constraint on the shared `reverify`/`sha256_hex` call site.
- ✅ AC 6 converts the throughput figure from a hand-division into a recorded, reproducible quantity
  by persisting `asset_bytes` and documenting the `asset_bytes ÷ median_ms` derivation.
- ✅ Specialised terms are defined inline (MSRV, getauxval/HWCAP), and the committed-vs-fallback logic
  is stated consistently across Summary, Open Questions and AC 7.

### Recommended Changes

1. **Reconcile Technical Notes with the resolved 0.11 decision** (addresses: asm-feature benchmark
   instruction contradicts the resolved 0.11 decision; residual spike-era framing in Technical Notes)
   Recast the diagnostic as soft (0.10) versus 0.11 runtime-detected, drop the "with/without asm"
   step, and mark the `asm` remedy rejected where it currently reads as a live first step. Fold the
   un-ranked "Remedy (i)/(ii)" list into the committed approach.

2. **Decide the cargo-deny cleanup's home** (addresses: cargo-deny no-duplicates cleanup is a
   separable concern; the two threads do not share one "done" state; `kind: task` strained)
   Either extract the `multiple-versions = "deny"` tightening plus the resolve/justify of unrelated
   pre-existing duplicates into its own work item that 0216 precedes or gates, or — if it stays —
   bound the duplicate set explicitly and state what happens to it when the AC 7 fallback fires.

3. **Unify the cargo-deny end state across AC 4 and AC 5** (addresses: AC 4's "cargo-deny passes" is
   ambiguous; subjective qualifiers have no defined threshold)
   Make AC 4 reference the post-change strict config, or fold its assertion into AC 5, so there is a
   single unambiguous cargo-deny end state.

4. **Assert the server pin conversion directly** (addresses: the server pin conversion is only
   indirectly gated)
   Add a short criterion (or extend AC 3) asserting by inspection that `cli/visualiser/server/Cargo.toml`
   declares `sha2 = { workspace = true }`.

5. **Capture the shared-artefact blast radius** (addresses: inheriting member crates not named as
   affected consumers; the rust-embed-utils transitive coupling is not surfaced)
   Enumerate the member crates that inherit the workspace `sha2` pin as consumers re-verified by the
   workspace build, and note the transitive `rust-embed-utils` reliance that keeps the collapse
   duplicate-free.

6. **Strengthen the musl evidence and cross-reference** (addresses: AC 3 verifies musl by proxy
   inspection; the 0217 reciprocal edge)
   Add a stronger inspection artefact for the musl backend (e.g. a linked-symbol probe of the
   runtime-detection path) or make the 0217 cross-reference binding inside AC 3, and reference the
   now-landed `work-item:0216` edge on 0217.

7. **Optional — settle AC 2's warning baseline** (addresses: AC 2's "no new warnings" has no captured
   baseline)
   Require the pre-change warning set as evidence, or simplify to "exits 0 with no warnings" per target
   if the four targets build warning-free today.

## Per-Lens Results

### Clarity

**Summary**: For an unusually dense work item, 0216 is exceptionally clear and internally
consistent: pronouns resolve cleanly, specialised terms are defined inline, and every acceptance
criterion is phrased as an observable outcome tied to a named artefact. The one genuine
internal-consistency defect is in Technical Notes, whose "benchmark with and without features=asm"
first step contradicts the resolved decision to adopt `sha2` 0.11 (in which the item itself states
the asm feature is removed). Two thinner ambiguities — which cargo-deny config state AC 4 asserts,
and the Summary's verifier-scoped framing versus Context's crate-wide reach — are resolvable by
cross-reading but could be tightened.

**Strengths**:
- Low-level and acronym terms are defined inline: "MSRV (minimum supported Rust version)",
  "getauxval/HWCAP (the libc auxiliary-vector feature query)".
- Acceptance criteria are observable end states with named artefacts and explicit thresholds (the
  ≥2.5× / ~1,390 MB/s floor labelled "the single binding gate", ~2,000+ MB/s labelled
  expectation-not-gate).
- Committed-outcome-versus-fallback logic is stated consistently across Summary, Open Questions and
  AC 7.
- Cross-document consistency with 0215 holds (gate direction, name/version-binding replacement).

**Findings**:
- 🟡 major (high) — Technical Notes: asm-feature benchmark instruction contradicts the resolved 0.11
  decision. "First step: benchmark with and without `features = ["asm"]`" conflicts with the resolved
  0.11 adoption and 0.11 having removed `asm`; an implementer would benchmark a non-existent feature.
- 🔵 minor (medium) — Acceptance Criteria: AC 4's "cargo-deny passes" is ambiguous about which config
  state (current lenient vs the strict config AC 5 introduces).
- 🔵 suggestion (low) — Summary: localises the shortfall to "the launcher's verifier" while Context
  scopes it crate-wide ("every sha256 the Rust binaries compute"); align the Summary's opening with
  the crate-global reach.

### Completeness

**Summary**: An exceptionally complete task: every expected section is present and densely populated,
frontmatter is intact with a recognised kind/status/priority, and the deliverable — the committed
0.11 backend enablement plus the bundled cargo-deny cleanup — is clearly stated across Summary and
Requirements with seven acceptance criteria and an explicit conditional fallback. The only observation
is minor: residual spike-era exploratory framing survives in Technical Notes after the conversion from
spike to task.

**Strengths**:
- Every standard section present and substantively populated — no empty/placeholder/sparse sections;
  Open Questions carry recorded resolutions.
- Frontmatter complete and well-formed (task, ready, priority, parent 0136, derived_from, rich
  relates_to).
- Summary captures the load-bearing scope expansion (the workspace-wide deny cleanup) rather than
  leaving it implicit.
- Seven specific acceptance criteria plus a clearly-specified conditional fallback (AC 7).
- Context motivates with concrete figures; Requirements enumerate the specific pre-existing duplicates
  for the audit.

**Findings**:
- 🔵 suggestion (low) — Technical Notes: residual open-investigation framing ("benchmark with and
  without asm", un-ranked two-remedy list) is more appropriate to a spike than a committed task; trim
  to the committed 0.11 approach.

### Dependency

**Summary**: Unusually thorough dependency mapping: the bidirectional 0215 relationship (gate,
fallback route, shared call site, no-parallel constraint), the term-harness prerequisite with a
confirm-exists step, and the 0217 hand-off are all captured. The residual gaps are minor and
interpretive: the workspace `sha2` pin is a shared artefact inherited by several unnamed first-party
crates, and the musl verification is deferred to 0217 whose reciprocal edge the research flagged as
not yet landed.

**Strengths**:
- The 0215 coupling is captured bidirectionally and precisely, with an explicit no-parallel-scheduling
  constraint on the shared call site.
- The term-harness prerequisite is named with a "confirm it exists" step, handling the fact that the
  harness has no producing work item.
- The server's independent `sha2 = "0.10"` literal is named and its move to the workspace pin captured.
- 0189/0205/0191, the epic parent 0136, and the 0217 hand-off are recorded in both Dependencies and
  frontmatter.

**Findings**:
- 🔵 minor (medium) — Requirements: the member crates inheriting the workspace `sha2` pin (launcher,
  work-adapters, jira-client, design-adapters, remote-projection, server) are not all named as
  affected consumers; the shared-artefact blast radius is implied.
- 🔵 minor (medium) — Dependencies: AC 3 defers musl verification to 0217, but the research flagged
  0217 carried no reciprocal `work-item:0216` edge or sha2-backend criterion. _(Landed on 0217 after
  the agents read the source; recommend referencing it from 0216.)_
- 🔵 suggestion (low) — Requirements: the duplicate-free collapse relies on transitive `rust-embed-utils`
  remaining the source of `sha2` 0.11.0; surface this coupling so a future divergence is visible.

### Scope

**Summary**: The core deliverable — the 0.11 bump, four-target verification, and darwin measurement —
is a coherent, well-bounded task with deliberately drawn edges (linux to 0217, cache-hit removal to
0215). The recent expansion folds in a workspace-wide cargo-deny cleanup that resolves pre-existing
duplicates unrelated to sha256, an independently deliverable concern the item's own source research
flags as a scope expansion. The bundling gives the task two threads with different completion
conditions, so it warrants decomposition.

**Strengths**:
- The sha2 backend switch itself is tightly scoped (bump, four-target verification, darwin before/after,
  the small measure.py change) around the single goal.
- Boundaries are drawn deliberately and explicitly — linux/musl to 0217, cache-hit removal gated to
  0215 with a no-parallel note.
- The spike-to-task conversion is appropriately sized; the accept-gap route is correctly demoted to a
  fallback.

**Findings**:
- 🟡 major (high) — Requirements (cargo-deny cleanup): bundles two separable concerns; only the
  first-party `sha2` collapse follows from the bump, while the `digest`/`cpufeatures`/reqwest/rustls/gix/jj
  duplicates predate it and are independently deliverable. Extract the tightening plus unrelated-duplicate
  resolution into its own item that 0216 precedes or gates.
- 🔵 minor (medium) — AC 7 / Open Questions: the two threads do not share one "done" — the deny cleanup
  has no disposition in the accepted-fallback branch and would be orphaned or half-applied.
- 🔵 suggestion (medium) — Requirements / Frontmatter: kind: the open-ended cross-tree audit strains the
  atomic-increment expectation of a `task`; if it stays, bound the duplicate set explicitly.

### Testability

**Summary**: For a task-kind item this is unusually testable: most criteria name an exact command,
task, file, target triple, or numeric threshold, and AC 1 states a concrete Given/When/Then floor with
a diagnostic band. The residual weaknesses are soft qualifiers ("justified" skips, "genuinely cannot be
made clean") a verifier cannot objectively settle, and that the headline outcome is only measured on
darwin-arm64 while the musl backend is confirmed by inspection alone. One Requirement (the server pin
conversion) is only indirectly gated.

**Strengths**:
- AC 1 is a fully specified verification (named input, action, absolute pass threshold, diagnostic
  clause).
- The ≥2.5× floor is set with deliberate margin so host-load noise cannot produce an ambiguous
  pass/fail.
- AC 6 converts the throughput figure into a recorded, reproducible quantity via persisted
  `asset_bytes`.
- AC 2 enumerates the four exact triples; AC 4/5/6 each name the concrete task or file constituting the
  evidence.
- Primary and fallback completion states are explicitly separated (AC 7) with concrete fallback
  deliverables.

**Findings**:
- 🔵 minor (medium) — AC 3: musl enablement is verified by proxy inspection (crate present, no forcing
  flag), not by any active-backend or throughput check within scope; the "no forcing flag" half is
  trivially satisfiable. Add a stronger inspection artefact or make the 0217 cross-ref binding.
- 🔵 minor (medium) — AC 5, AC 7: subjective qualifiers ("justified" skips, "genuinely cannot be made
  clean") have no defined threshold; state the concrete evidence that constitutes "cannot be made clean".
- 🔵 suggestion (medium) — Requirements / AC: converting the server literal to the workspace pin is only
  indirectly gated; a `sha2 = "0.11"` literal would also pass AC 5. Add an inspection criterion for the
  server declaration.
- 🔵 suggestion (low) — AC 2: "no warnings beyond an unchanged build" has no captured baseline; require
  it as evidence or simplify to "exits 0 with no warnings".

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** REVISE

The edits resolved the original clarity major and nearly every minor and suggestion, but the
item stays at REVISE on six majors: the scope major persists by the deliberate keep-and-bound
decision, and the strengthening edits introduced new majors (an unverifiable AC 3 symbol probe,
an undefined AC 7 fallback pass-set, and a "fallback"/`asm` contradiction in Technical Notes),
while cross-referencing the research doc surfaced the pre-existing `cargo build` vs `cargo zigbuild`
mismatch. The introduced majors are all quick, local fixes.

### Previously Identified Issues

- 🟡 **Clarity** — asm-benchmark instruction contradicts the 0.11 decision — **Partially resolved**.
  The benchmark first step is gone, but the Technical Notes rewrite left a related contradiction
  (`asm` listed as a retained fallback while 0.11 removed it), now the pass-2 clarity major.
- 🟡 **Scope** — cargo-deny cleanup is a separable concern — **Still present** (by decision). Bounded
  to the enumerated 18-crate set, but bounding addresses tractability, not coherence; AC 7's
  "cleanup completes regardless of the sha2 outcome" is itself the separability signal.
- 🔵 **Clarity** — AC 4 cargo-deny config ambiguity — **Resolved** (AC 4 reproducibility-only; AC 5
  the single end state).
- 🔵 **Scope** — two threads, no shared "done" — **Resolved** (AC 7 states the cleanup completes
  regardless of the fallback).
- 🔵 **Dependency** — inheriting crates not named — **Resolved** (all six consumers enumerated).
- 🔵 **Dependency** — 0217 reciprocal edge — **Resolved in fact** (edge landed on 0217), but
  re-raised as a major: 0216 asserts it rather than owning it.
- 🔵 **Testability** — AC 3 musl proxy inspection — **Partially resolved** (strengthened with a
  symbol probe, but the probe is under-specified and may be infeasible on a stripped binary).
- 🔵 **Testability** — subjective qualifiers — **Partially resolved** (AC 7 "cannot be made clean"
  now concrete; AC 5 "justified" still soft, downgraded to minor).
- 🔵 **Completeness** — spike-era Technical Notes framing — **Resolved**.
- 🔵 **Dependency** — rust-embed transitive coupling — **Resolved** (noted in Requirements).
- 🔵 **Scope** — kind: task strained — **Resolved** (audit bounded to a finite enumerated set).
- 🔵 **Testability** — server pin not asserted — **Resolved** (folded into AC 3).
- 🔵 **Testability** — AC 2 warning baseline — **Resolved** (AC 2 requires a captured baseline).

### New Issues Introduced

- 🟡 **Testability / Clarity** — AC 2/AC 3 name `cargo build --release`, but the musl targets
  cross-build only via `cargo zigbuild`; the pass procedure is not runnable for the musl statics.
  (Pre-existing; surfaced by cross-referencing the research doc.)
- 🟡 **Testability** — AC 3's added "linked-symbol probe" is under-specified and may be infeasible
  against a `strip = true` release binary — the one substantive backend-selection check has no
  runnable procedure. (Introduced by the AC 3 strengthening edit.)
- 🟡 **Testability** — AC 7 clarifies AC 5's fallback status but leaves AC 1/2/3/4/6 binding status
  undefined — the complete pass set in the fallback outcome is undetermined. (Introduced by the AC 7
  edit.)
- 🟡 **Clarity** — "fallback" now names two mechanisms (build-failure → 0215 vs performance →
  asm/crate-swap/vendored-asm), and Technical Notes list `asm` as a retained fallback while stating
  0.11 removed it. (Introduced by the Technical Notes rewrite.)
- 🟡 **Dependency** — the 0217 hand-off is asserted as done in 0216 but not owned or gated by it; had
  the reciprocal edit lagged, the musl measurement would fall between items. (Reframing of the
  now-landed edge.)
- 🔵 **Dependency** — the pre-bump ordering is unstated (teach `measure.py` → capture soft-backend
  baselines → apply the bump).
- 🔵 **Clarity** — "vendor shims" / "its marker" used without an in-item gloss.
- 🔵 **Testability** — AC 5 "justified" skip has no objective bar (exit-0 is the binding gate).
- 🔵 **Testability** — the six-consumer blast radius is named but no criterion verifies it (suggest
  `mise run cli:check`).
- 🔵 **Completeness** — `priority: low` sits oddly against the gating role over 0215 (`medium`) and
  the expanded scope.
- 🔵 **Dependency** — the `measure.py` `asset_bytes` change is a prerequisite 0217 inherits but is not
  named as such in the hand-off.

### Assessment

One more focused edit pass should clear the introduced majors: point AC 2/AC 3 at `cargo zigbuild` /
the `mise run` build tasks; make the AC 3 musl evidence a runnable check (a behavioural backend print,
or a probe against a pre-strip artefact) rather than a symbol grep on a stripped binary; enumerate
which criteria remain binding under the AC 7 fallback; and resolve the `asm`-as-fallback contradiction
in Technical Notes (drop `asm`, since 0.11 removed it, and label the two "fallback" mechanisms
distinctly). With those cleared, the scope major stands alone as your accepted keep-and-bound decision
(1 major → COMMENT). The work item is not yet ready for implementation, but is close.

## Re-Review (Pass 3) — 2026-09-11

**Verdict:** COMMENT

The verification pass confirms every pass-2 major is cleared — the `cargo build` vs
`cargo zigbuild` mismatch, the unverifiable symbol probe, the AC 7 fallback pass-set, the
`asm`/fallback contradiction, and the asserted-not-owned 0217 edge. Only the scope major
remains, and it is the deliberate keep-and-bound decision. With a single major and no
critical, the verdict is COMMENT: acceptable as-is, with refinement-grade residue.

### Previously Identified Issues (pass 2)

- 🟡 **Clarity** — asm/fallback contradiction — **Resolved**.
- 🟡 **Testability** — `cargo build` vs `cargo zigbuild` — **Resolved** (AC 2/AC 3 name the
  zigbuild flow).
- 🟡 **Testability** — unverifiable symbol probe — **Resolved** (now a runnable check against an
  unstripped build/`.rlib` or a debug print).
- 🟡 **Testability** — AC 7 fallback pass-set undefined — **Resolved** (each criterion's binding
  status enumerated).
- 🟡 **Dependency** — 0217 asserted not owned — **Resolved** (converted to a closure-gating
  confirmation obligation).
- 🟡 **Scope** — cargo-deny cleanup separable — **Still present**, the sole remaining major: the
  accepted keep-and-bound decision.
- 🔵 All pass-2 minors/suggestions — **Resolved** (ordering note, vendor-shims gloss, AC 5
  justification bar, `cli:check` consumer coverage, priority Drafting Note, measure.py-as-0217
  prerequisite, 0215 consolidation).

### New Issues Introduced (all minor / suggestion)

- 🔵 **Testability** — AC 1 mixes an absolute floor with a relative multiplier over a fresh
  baseline; pin to `median ≤ ~1.79 ms` (≥1,390 MB/s over the 2,493,792-byte asset).
- 🔵 **Testability** — AC 3's `nm`/`strings` routes prove the backend is *available*, not
  *selected*; designate the debug-print as the conclusive check.
- 🔵 **Testability** — the server-pin `workspace = true` conversion is bound only by the
  fallback-waivable AC 3; bind it in a non-waivable criterion.
- 🔵 **Testability** — AC 7's "genuinely cannot be made clean" is still partly subjective;
  enumerate the approaches that must be shown to fail.
- 🔵 **Clarity** — `rust-embed` vs `rust-embed-utils` name shift; AC 3 evidence-list phrasing;
  "soft backend" / "0.10.9" first-use gloss.
- 🔵 **Dependency** — `priority: low` contradicts the 0216→0215 gate ordering (the frontmatter
  signal itself, beyond the Drafting Note).
- 🔵 **Dependency** — the `multiple-versions = "deny"` flip is a diffuse downstream coupling on
  future dependency-adding work; note it.
- 🔵 **Completeness** — optional dedicated Evidence/Results section for the recorded quantities.

### Assessment

Converged. The work item is acceptable for implementation as-is (COMMENT); the one remaining
major is the accepted scope decision, and the rest are refinement-grade. Worth doing before
implementation: pin AC 1 to the exact ms threshold, bind the server-pin conversion in a
non-waivable criterion, reconcile the priority signal against the 0216→0215 gate, and record
the keep-and-bound scope decision in the item itself (it currently lives only in the research
doc). No further review pass is needed unless those edits are made and re-verification is wanted.

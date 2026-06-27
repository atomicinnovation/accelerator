---
type: plan-review
id: "2026-06-27-0157-port-luminosity-adrs-and-feeding-spikes-review-1"
title: "Plan Review: Port Luminosity ADRs and Feeding Spikes"
date: "2026-06-27T12:37:11+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-27-0157-port-luminosity-adrs-and-feeding-spikes"
parent: "plan:2026-06-27-0157-port-luminosity-adrs-and-feeding-spikes"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [correctness, documentation, safety, standards, architecture, test-coverage]
review_number: 1
review_pass: 2
tags: [adr, decisions, porting, luminosity, supersession]
last_updated: "2026-06-27T14:45:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Port Luminosity ADRs and Feeding Spikes

**Verdict:** REVISE

This is a strong, unusually well-grounded plan: the +44 ID-shift arithmetic is
internally consistent and verified against the research's cross-reference graph,
the perspective-inversion problem is named and exemplified, the phase
decomposition follows the genuine dependency structure, and the frontmatter
conformance claims match the live corpus. The plan is also commendably honest
about its primary gate's limits. The reason for REVISE is concentrated almost
entirely in **one area — the supersession machinery**: the corpus validator does
not check `superseded_by` *at all*, the Phase 3 enactment is not executable via
the tooling it names, the in-body status convention it prescribes contradicts
the live corpus, and the reject branch of the acceptance gate is unhandled. A
secondary cluster concerns the verification battery: the grep assertions are
weaker than their checklist text claims, and the riskiest defect class
(wrong-but-resolvable references) is left to manual review when it could be
mechanised from the §D map.

### Cross-Cutting Themes

- **The supersession mechanism is the dominant risk** (flagged by: correctness,
  safety, architecture, test-coverage, standards, documentation) — Six of the
  six lenses independently converged on the supersede edges as the weakest part
  of the plan, from distinct angles: `superseded_by` is not a validated ADR
  linkage key so it is never shape- or resolution-checked (test-coverage,
  safety); no symmetry or status-legality check exists, so a half-applied or
  mis-targeted supersession passes every gate green (safety, architecture,
  correctness); the named Phase 3 tooling (`create-adr --supersedes` /
  `/review-adr`) cannot actually flip an *existing* ADR's targets (correctness);
  the proposed→rejected branch leaves a dangling `supersedes:` declaration
  (correctness); and the prescribed `**Status**: Superseded by ADR-00XX` in-body
  text has no precedent in the corpus (standards, documentation).

- **Wrong-but-resolvable references are the most likely defect and the least
  covered** (flagged by: test-coverage, documentation, correctness) — The §D map
  involves ~25 per-ADR reference reclassifications (shift +44 vs. leave fixed). A
  mis-classified ref still *resolves* (ADR-0003, ADR-0016 all exist locally), so
  the validator and grep checks pass; only a single manual bullet guards it. The
  in-prose ordinal references ("the 5th ADR", "decision 9/10/11") flagged in
  research §B get no edit instruction at all.

- **The grep assertion battery is weaker than its checklist text claims**
  (flagged by: test-coverage, standards) — `grep -c` prints per-file counts (not
  the "sums to 11" the text asserts); negative `! grep` passes vacuously if the
  glob expands to fewer files than expected; and the "leftover Luminosity" scan
  is structurally incapable of failing because every ported ADR is *required* to
  contain a `…/luminosity/…` URL.

- **The load-bearing Rust-CLI assumption is under-surfaced** (flagged by:
  architecture, documentation, correctness) — Six of the eleven ADRs describe an
  architecture Accelerator has only researched (0136), not adopted. The
  assumption is compressed into a single table cell, and the CLI command name
  (`accelerator`) is hard-coded though 0136 lists it as an open question.

### Tradeoff Analysis

- **Match the (inconsistent) live corpus vs. establish a cleaner convention**:
  Standards and Documentation both flag that the plan's prescribed superseded
  in-body status text matches neither existing precedent (ADR-0026 keeps
  `Accepted` + scoping blockquote; ADR-0036 keeps `Accepted` in-body while
  frontmatter says `superseded`). The corpus precedent is itself inconsistent, so
  the plan can either *conform* to the messy precedent or *establish* a clean
  convention — but it should pick one deliberately and say so, rather than assert
  a format that exists nowhere.

### Findings

#### Critical

- 🔴 **Test Coverage**: Phase 3 `superseded_by` edges have zero automated referential safety net
  **Location**: Phase 3 Automated Verification; Testing Strategy (Primary gate)
  `superseded_by` is not in the ADR row's validated linkage keys
  (`templates-schema.tsv` lists only `parent supersedes relates_to`), so the
  validator never shape-checks or resolution-checks it. The Phase 3 greps only
  confirm the literal string is present, not that ADR-0047/0052 exist — a
  fat-fingered or stale target sails past both the validator and the grep on the
  hardest-to-reverse operation in the port.

#### Major

- 🟡 **Correctness**: Phase 2 hand-writes `supersedes:` outside the sanctioned `create-adr --supersedes` path it claims to use
  **Location**: Phase 2 per-ADR specs ADR-0047 / ADR-0052; Migration Notes
  `create-adr --supersedes` writes `supersedes` atomically *with* flipping the
  target to `superseded` and requires the target be `accepted` then — it has no
  mode to declare `supersedes` on a still-proposed ADR. The Phase 2 declaration
  is therefore a raw frontmatter edit the tooling does not support, contradicting
  the Migration Notes' "honoured transition table" guarantee.

- 🟡 **Correctness**: No sanctioned skill performs the Phase 3 edge-flip on pre-existing ADRs
  **Location**: Phase 3 §2 (Enact supersession)
  `create-adr --supersedes` *creates a new ADR* (would mint ADR-0056/0057, not
  flip the existing edge); `review-adr` explicitly states supersession is handled
  by `create-adr`, not it. No skill takes an existing accepted ADR and applies
  `superseded_by` to its targets — the real mechanism is manual frontmatter
  edits, which the plan does not acknowledge.

- 🟡 **Correctness**: No handling for the proposed→rejected branch of the acceptance gate
  **Location**: Phase 3 §1 (Acceptance gate); Desired End State
  ADR-0031 allows `proposed → rejected`. If a superset is rejected, the corpus is
  left with a rejected ADR still carrying the Phase-2 `supersedes:` declaration
  and the targets never superseded — an incoherent terminal state the validator
  cannot catch.

- 🟡 **Safety**: Primary safety net cannot detect a half-applied or asymmetric supersession
  **Location**: Phase 3 Automated Verification; Testing Strategy (Primary gate)
  Phase 3 mutates three accepted artifacts in a non-atomic edit. If interrupted
  (e.g. ADR-0016 flipped but ADR-0047 never gains the reciprocal `supersedes`),
  the corpus is left semantically broken while the validator and `mise run check`
  both report green.

- 🟡 **Safety**: Frontmatter/in-body status divergence is invisible to every automated gate
  **Location**: Phase 3 Manual Verification; Phase 2 superset authoring
  The validator reads frontmatter `status` as authoritative and never compares it
  to the in-body `**Status**:` block (ADR-0016 already diverges:
  `status: accepted` vs body "Active — file paths superseded in part"). Phase 3
  must update both but relegates consistency to a manual checkbox.

- 🟡 **Architecture**: Load-bearing Rust-CLI assumption underpins six ADRs but is not surfaced as a risk
  **Location**: Settled decisions ("Class-B import gate"); Phase 2 (Class-B ADRs)
  ADR-0045/0046/0048/0053/0054/0055 describe an unadopted architecture. The
  research flags this assumption as load-bearing; the plan compresses it to a
  one-line table cell with no fallback if 0136 does not land.

- 🟡 **Architecture**: Supersession edge correctness rests entirely on manual review
  **Location**: Phase 2 (supersedes declarations) → Phase 3; Current State Analysis
  The validator checks only resolution. The correctness of the cross-corpus edges
  (ADR-0047↔0016/0017, ADR-0052↔0027) rests on manual spot-review across two
  separately-merged phases; a reciprocal-edge error passes every automated gate.

- 🟡 **Test Coverage**: Wrong-but-resolvable references are the most likely defect and the least covered
  **Location**: Key Discoveries; Testing Strategy (Manual review)
  A luminosity self-ref left un-shifted (`adr:ADR-0003` for `adr:ADR-0047`) or an
  Accelerator ref wrongly shifted both *resolve* and pass the validator; only one
  manual bullet guards them. The §D table is already an exhaustive expected-value
  list and could be turned into per-file `relates_to` grep assertions.

- 🟡 **Test Coverage**: Several Phase 2 grep assertions are weaker than their checklist text claims
  **Location**: Phase 2 Automated Verification (status / no-producer-parent)
  `grep -c '^status: proposed' …glob…` prints per-file counts, not the "sums to
  11" stated, and does not fail if one file is mis-statused; negative `! grep …`
  passes vacuously if the glob matches fewer files than expected. Use
  `grep -l … | wc -l` = 11 and pair negatives with a positive file-count gate.

- 🟡 **Test Coverage**: The "leftover Luminosity" scan is structurally incapable of failing
  **Location**: Phase 2 Automated Verification (leftover `Luminosity` scan)
  `grep -rn 'Luminosity\|\.luminosity/' …` has no expected result and cannot
  distinguish the required origin URL (contains `luminosity`) from an
  un-rewritten product name. It degrades to an unbounded manual eyeball on the
  heaviest editorial risk. Make it exclusionary (match `luminosity` on lines that
  do *not* contain the origin URL; assert empty).

- 🟡 **Documentation**: In-prose ordinal cross-references are never given an edit instruction
  **Location**: Phase 2 per-ADR specs; §D reference-rewrite map
  Research §B flags in-prose ordinals ("the 5th ADR in this set", "decision
  9/10/11") needing rewrite to local numbers. The plan only addresses frontmatter
  `relates_to`; no instruction or grep covers the prose ordinals, which no
  automated check catches.

- 🟡 **Documentation**: Phase 3 superseded status-block format contradicts local precedent
  **Location**: Phase 3 Manual Verification (in-body status blocks)
  The asserted `**Status**: Superseded by ADR-00XX` matches neither ADR-0036
  (keeps in-body `Accepted`) nor ADR-0026 (partial; `accepted` + prose note). The
  verification step checks for a string the corpus has never used.

#### Minor

- 🔵 **Correctness**: Transient asymmetric `supersedes` edge between Phase 2 and Phase 3
  **Location**: Key Discoveries; Phase 2 Overview
  Between Phase 2 merge and Phase 3 acceptance, a `proposed` ADR asserts it
  supersedes three live `accepted` ADRs that carry no reciprocal `superseded_by`.
  Validator-green only because the symmetry check is absent; acknowledge it as an
  accepted intermediate state or defer the declaration to Phase 3.

- 🔵 **Correctness**: ADR-0055 `relates_to` ADR-0052 may be vestigial after the toolchain rework
  **Location**: Phase 2 ADR-0055 spec; Settled decisions (toolchain count)
  The 0055→0052 edge was inherited mechanically; after reworking ADR-0055's
  reasoning, re-confirm each of its four relations still reflects a real
  dependency.

- 🔵 **Correctness**: ADR-0049 `relates_to` ADR-0047 introduces a relation to a possibly-rejected superset
  **Location**: Phase 2 ADR-0049 spec
  Correct +44 mapping, but if ADR-0047 is rejected in Phase 3, ADR-0049 relates
  to a rejected ADR. Tolerated by the validator; note it in the reject-branch
  contingency.

- 🔵 **Safety**: Irreversible `content/` drop guarded only by manual eyeball comparison
  **Location**: Phase 2 ADR-0052 ("Drop the content/ marketing half entirely")
  Low-risk (the luminosity source remains a cited URL) but the only safeguard
  that the surviving `meta/` half is preserved intact is one negative manual
  bullet; consider a positive subsection-presence check.

- 🔵 **Safety**: Stale ID allocation if the corpus moves between allocation and authoring
  **Location**: Migration Notes; Phase 1/2 ID allocation
  The §D map hard-codes 0045/0158; a concurrent corpus change would invalidate
  every cross-reference. Add a pre-flight assertion that the allocator still
  returns 0045/0158 before authoring.

- 🔵 **Standards**: Prescribed superseded status-block text has no precedent in the live corpus
  **Location**: Phase 3 Manual Verification
  A corpus-wide grep for `**Status**: Superseded` returns zero hits. (See the
  Documentation major and the Tradeoff above — resolve together.)

- 🔵 **Standards**: In-body status-block shorthand omits the Date/Author lines the convention requires
  **Location**: Phase 2 Changes Required (Body): "H1 → status block (`**Status**: Proposed`)"
  The documented and live block is three lines (`**Date**` / `**Status**` /
  `**Author**`). Spell out the full block so an implementer does not emit a
  one-line status.

- 🔵 **Standards**: Validator failure for a frontmatter luminosity URL is mis-named
  **Location**: Settled decisions ("Luminosity origin in frontmatter")
  A frontmatter URL fails as `BAD-LINKAGE-SHAPE` (or is not checked at all, since
  `source` is not an ADR linkage key), not `DANGLING-REF`. The conclusion (cite
  in prose `## References`) is correct; only the rationale is mis-stated.

- 🔵 **Architecture**: Dropping the `content/` half narrows ADR-0052's scope without recording the omission
  **Location**: Phase 2 ADR-0052 spec; §D map
  Right call domain-wise, but a future reader comparing against the luminosity
  original may be confused. Note in `## Context`/`## References` that the
  `content/` deliverables tree was intentionally not ported.

- 🔵 **Architecture / Documentation**: Class-B ADRs hard-code an unsettled CLI name
  **Location**: Settled decisions (CLI naming); Phase 2 ADR-0053/0054 specs
  `accelerator` / `accelerator-<sub>` is baked in though 0136 lists the command
  name as open. Mark it provisional pending 0136, or confirm it is now settled.

- 🔵 **Architecture**: Editing the toolchain count erodes a load-bearing premise of ADR-0055
  **Location**: Phase 2 ADR-0048 / ADR-0055 specs
  ADR-0048's three→four edit invalidates ADR-0055's "no fourth toolchain"
  argument. The plan prescribes the rework; strengthen the manual check to a
  negative grep for residual "fourth toolchain" / "three languages" phrasing.

- 🔵 **Test Coverage**: Phase 3 transition-legality precondition has no assertion
  **Location**: Phase 3 §1 (Acceptance gate)
  The `accepted → superseded` legality (targets must be `accepted` first) is a
  prose parenthetical with no grep. Add a pre-flip `grep '^status: accepted'` gate
  on ADR-0016/0017/0027.

- 🔵 **Test Coverage**: "Supersets declare intent" grep does not check the targets
  **Location**: Phase 2 Automated Verification (supersets declare intent)
  `grep -l 'supersedes:' … | wc -l → 2` does not catch ADR-0047 declaring only
  one of its two config targets. Assert the exact targets are present.

- 🔵 **Test Coverage**: Spike content-fidelity has no mechanical backstop
  **Location**: Phase 1 & Phase 2 Manual Verification
  Verbatim preservation of `## Spike Outcome` / `## Recommendation` and the dated
  facts rests on one broad manual bullet. Add a content diff against `../luminosity`
  or grep for the named date-anchor strings.

- 🔵 **Documentation**: Embedded luminosity work-item path references lack a rewrite instruction
  **Location**: Phase 2 Universal edits / Success Criteria
  Research §F lists `meta/work/000N-…` luminosity paths needing drop/repoint/URL.
  Universal edits cover names and `.luminosity/` but not these; add an edit and a
  grep asserting no bare luminosity work-item paths remain.

- 🔵 **Documentation**: Heavy-burden superset ADRs under-specify how the merged Decision should read
  **Location**: Phase 2 ADR-0047 / ADR-0052 specs
  The specs cover frontmatter `supersedes` and the deferred flip but not which
  local decisions (e.g. ADR-0016's coupled axes) must be carried into the merged
  Decision so the superset genuinely covers what it supersedes.

- 🔵 **Documentation**: Spike sections beyond Outcome/Recommendation are not addressed for perspective inversion
  **Location**: Phase 1 spike porting
  The ~373/~316-line spikes contain research/options sections carrying luminosity
  vantage and command names; clarify the verbatim-vs-invert boundary and add a
  leftover-`luminosity` grep on the ported spikes.

#### Suggestions

- 🔵 **Standards**: Wholesale `superseded` flip of ADR-0016 conflicts with the partial-supersession style
  **Location**: Phase 3 §2 (ADR-0016 → status: superseded)
  ADR-0026 keeps `accepted` + a scoping blockquote for partial supersession.
  Confirm ADR-0047 is a *full* superseder of ADR-0016; if partial, follow the
  ADR-0026 pattern instead of a wholesale status flip.

- 🔵 **Documentation**: Spike-fed enumeration may overstate spike 0158 as ADR-0046's primary provenance
  **Location**: Desired End State (spike-fed ADR list) vs §D map
  ADR-0046 (distribution) may have the Accelerator pipeline as its primary basis
  rather than the Rust-CLI spike. Confirm and word the References note accordingly.

### Strengths

- ✅ The +44 ID shift is arithmetically sound; the §D rewrite table is fully
  consistent with the research's cross-reference graph, with no luminosity
  self-reference left unshifted and the four embedded Accelerator refs
  (0016/0017/0020/0021) correctly held fixed (all confirmed present locally).
- ✅ The ascending-authoring invariant genuinely holds — every ADR-to-ADR target
  is strictly lower-numbered and the only non-self targets (ADR-0001, work-items
  0158/0159) are authored earlier or pre-exist.
- ✅ Phase boundaries map onto real dependency edges, and each phase declares its
  own validator-green, independently-mergeable invariant.
- ✅ The supersession graph is reasoned at the concern level: ADR-0001 is linked
  via `relates_to` rather than superseded because it bundles still-live
  agent-separation and token-budget decisions (verified against its title/body).
- ✅ Frontmatter conformance is accurate against the live corpus and the actual
  validator: omitting `producer`/`parent`, no `revision`/`repository`, quoted
  `id`/`title`, bare-int `schema_version`, typed `doc-type:id` refs.
- ✅ The plan is honest about its primary gate's limits (resolution, not intended
  target) and routes perspective-inversion and the cross-reference graph to
  manual review; date-sensitive facts are explicitly preserved verbatim.

### Recommended Changes

1. **Rebuild the supersession mechanism (Phases 2–3) end to end** (addresses:
   the Critical and all five supersession-related majors). Decide and document
   the *actual* mechanism for flipping pre-existing ADRs (it is manual frontmatter
   editing — `create-adr --supersedes` cannot do it and `review-adr` disclaims
   it); decide whether `supersedes:` is declared in Phase 2 (hand-authored,
   transiently asymmetric) or deferred to Phase 3; add a reject-branch
   contingency; and reconcile the in-body status-block format with the
   ADR-0026/0036 precedent (or consciously establish a new convention).

2. **Add explicit supersession symmetry/legality assertions to Phase 3**
   (addresses: superseded_by-not-validated, half-applied-supersession,
   transition-legality). Because the validator ignores `superseded_by` entirely,
   add greps that: each `superseded_by: adr:X` names an ADR that is `accepted`
   and lists this file in its `supersedes:`; each target was `accepted` before the
   flip; and no ADR is `superseded` without a reciprocal edge. Run as an
   all-or-nothing post-condition.

3. **Mechanise the §D reference map and tighten the grep battery** (addresses:
   wrong-but-resolvable refs, weak grep assertions, leftover-Luminosity scan,
   supersets-declare-intent). Turn each ADR's expected `relates_to` into an exact
   per-file grep; switch counts to `grep -l … | wc -l` = 11 with a positive
   glob-expansion gate; make the leftover-`luminosity` scan exclusionary of the
   origin URL; assert exact `supersedes` targets.

4. **Add a universal edit + checks for in-prose ordinals and luminosity
   work-item paths** (addresses: in-prose ordinals, embedded work-item paths).
   Instruct rewriting "the Nth ADR" / "decision N" to local numbers and
   dropping/repointing/URL-ifying `meta/work/000N-…` luminosity paths, each with a
   matching manual or grep check.

5. **Surface the Rust-CLI assumption and CLI-name provisionality explicitly**
   (addresses: load-bearing assumption, hard-coded CLI name). Promote the
   assumption to a named risk with a fallback (hold/reject the six Class-B ADRs if
   0136 does not land), and mark the `accelerator` command name provisional
   pending 0136 — or confirm 0136 has settled it.

6. **Strengthen content-fidelity and superset-Decision specs** (addresses: spike
   fidelity, content/ drop traceability, under-specified superset Decisions).
   Add a content-diff/anchor-grep for the spikes' preserved sections; have
   ADR-0052 note the intentional `content/` omission; and specify which local
   decisions each superset must carry into its merged Decision.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan's core arithmetic (uniform +44 shift, lum 0001..0011 →
local 0045..0055) and its reference-rewrite map are internally consistent: every
edge in research §B's cross-reference graph traces through the +44 shift, the §D
table and per-ADR edit specs agree, and the four embedded Accelerator refs
(0016/0017/0020/0021) are correctly held fixed. The ascending-authoring invariant
holds for all ten ported self-references plus the two pre-existing targets. The
serious gaps are in state management: the supersession declaration/enactment
scheme contradicts the sanctioned tooling, leaves an asymmetric `supersedes` edge
between Phase 2 and Phase 3, and has no handling for the proposed→rejected branch.

**Strengths**:
- +44 ID shift arithmetically sound; §D table fully consistent with §B graph; no
  self-ref left unshifted.
- §D table and per-ADR specs agree on every `relates_to` list — no internal
  contradiction.
- Ascending-authoring invariant genuinely satisfied; only non-self targets
  (ADR-0001, work-items 0158/0159) pre-exist or are authored earlier.
- Immutability sequencing correctly identified (proposed→accepted before
  accepted→superseded), matching ADR-0031 verified live.
- Embedded-Accelerator-refs caveat correct; all four files confirmed present.

**Findings**:
- 🟡 **major** (high) — *Phase 2 declares `supersedes:` outside the sanctioned
  `create-adr --supersedes` path it claims to use.* `create-adr --supersedes`
  writes `supersedes` atomically with flipping the target to `superseded` and
  requires the target be `accepted`; it offers no mode to declare on a
  still-proposed ADR. The Phase 2 declaration is a raw frontmatter edit the tooling
  does not support, so the Migration Notes' "honoured transition table" claim does
  not hold. *Loc: Phase 2 ADR-0047/0052 specs; Migration Notes.*
- 🟡 **major** (high) — *No sanctioned skill performs the Phase 3 edge-flip on
  pre-existing ADRs.* `create-adr --supersedes` creates a new ADR (would mint
  0056/0057); `review-adr` disclaims supersession. The real mechanism is manual
  frontmatter edits, unacknowledged. *Loc: Phase 3 §2.*
- 🟡 **major** (high) — *No handling for the proposed→rejected branch.* If a
  superset is rejected, the corpus has a rejected ADR with a dangling `supersedes:`
  and the targets never superseded. *Loc: Phase 3 §1; Desired End State.*
- 🔵 **minor** (high) — *Transient asymmetric `supersedes` edge between Phase 2 and
  Phase 3* — validator-green only because the symmetry check is absent.
- 🔵 **minor** (medium) — *ADR-0055 `relates_to` ADR-0052 may be vestigial after
  the toolchain rework* — re-confirm its four relations.
- 🔵 **minor** (medium) — *ADR-0049 `relates_to` ADR-0047 (a possibly-rejected
  superset)* — note in the reject-branch contingency.

### Documentation

**Summary**: Strong on the editorial dimension — the perspective-inversion problem
is named with before/after examples, per-ADR specs carry burden labels, dated
facts are preserved verbatim, and luminosity provenance is correctly routed to
prose `## References` URLs (no typed `source:` key). The chief gaps: (1) in-prose
ordinal cross-references flagged in research §B get no edit instruction, only
frontmatter `relates_to` is mapped; and (2) the Phase 3 superseded in-body status
text contradicts local precedent. Several per-ADR specs are thinner than their
stated "Heavy" burden warrants.

**Strengths**:
- Perspective-inversion named as the central task with concrete examples and a
  manual-verification check.
- Provenance handled correctly: full-URL `## References`, no typed `source:` key;
  spike-fed ADRs cite local spike primary + luminosity secondary.
- Date-sensitive facts called out for verbatim preservation.
- Two content-accuracy edits pinpointed (toolchain three→four with ADR-0055
  knock-on; false ".sh = linter" claim with ~226 figure).
- Two-layered reference-rewrite caveat carried into the §D map and a manual check.

**Findings**:
- 🟡 **major** (high) — *In-prose ordinal cross-references never given an edit
  instruction* ("the 5th ADR", "decision 9/10/11"); only frontmatter `relates_to`
  addressed; no grep catches them. *Loc: Phase 2 per-ADR specs; §D map.*
- 🟡 **major** (high) — *Phase 3 superseded status-block format contradicts local
  precedent* (ADR-0036 keeps in-body `Accepted`; ADR-0026 keeps `accepted` + prose
  note). *Loc: Phase 3 Manual Verification.*
- 🔵 **minor** (medium) — *CLI naming resolution diverges from research's open
  question without rationale.* *Loc: Settled decisions; ADR-0053/0054 specs.*
- 🔵 **minor** (medium) — *Work-item path references (research §F) lack a rewrite
  instruction.* *Loc: Phase 2 Universal edits / Success Criteria.*
- 🔵 **minor** (medium) — *Heavy-burden superset ADRs under-specify how the merged
  Decision should read.* *Loc: Phase 2 ADR-0047/0052 specs.*
- 🔵 **minor** (low) — *Spike-fed enumeration may overstate spike 0158 as ADR-0046's
  primary provenance.* *Loc: Desired End State vs §D map.*
- 🔵 **minor** (medium) — *Spike sections beyond Outcome/Recommendation not addressed
  for perspective inversion.* *Loc: Phase 1 spike porting.*

### Safety

**Summary**: Blast radius is bounded (Markdown in a dev-tooling repo; recovery is
a jj revert) and the plan is commendably safety-aware: it sequences irreversible
supersession behind a human acceptance gate, defers status flips to Phase 3, and
adds a "no local ADR edited yet" guard. The principal weakness is the plan's own
admission that its primary safety net — the corpus validator — does not check
supersession symmetry, transition legality, or even the `superseded_by` reference,
so a half-applied or mis-targeted supersession can leave the corpus broken while
every gate reports green. Secondary: an undetectable frontmatter/in-body status
mismatch (already latent in ADR-0016) and the irreversible `content/` drop relying
on manual comparison.

**Strengths**:
- Irreversible ops correctly sequenced: all 11 import `proposed` (editable);
  near-terminal flips isolated into a gated Phase 3 enforced by immutability itself.
- Phase 2 includes a `jj diff --stat` negative guard detecting accidental early
  edits to ADR-0016/0017/0027.
- Supersession blast radius deliberately narrowed (ADR-0001 not superseded;
  confirmed with requester).
- Clear, adequate recovery path (jj revert); correctly resists heavyweight
  dry-run scaffolding.
- Phase 3 pre-flight legality check (verify targets `accepted`).

**Findings**:
- 🔴 **major** (high) — *Primary safety net cannot detect a half-applied or
  asymmetric supersession.* Verified schema TSV: validated `adr` linkage keys are
  `parent supersedes relates_to`; `superseded_by` not validated at all. Non-atomic
  Phase 3 edits across three files can leave one-sided edges while everything
  reports green. *Loc: Phase 3 Automated Verification; Testing Strategy.*
- 🟡 **major** (high) — *Frontmatter/in-body status divergence invisible to every
  automated gate.* Validator never compares frontmatter `status` to the in-body
  block; ADR-0016 already diverges. *Loc: Phase 3 Manual Verification; Phase 2.*
- 🔵 **minor** (high) — *Irreversible `content/` drop guarded only by manual eyeball
  comparison* (low-risk; source remains a cited URL). *Loc: Phase 2 ADR-0052.*
- 🔵 **minor** (medium) — *Stale ID allocation if the corpus moves between
  allocation and authoring* — add a pre-flight allocator assertion. *Loc: Migration
  Notes; Phase 1/2.*

### Standards

**Summary**: Unusually well-grounded in documented conventions — file-naming
(ADR-0029), body-section ordering (ADR-0030), typed-linkage shape (ADR-0034),
omit-when-empty (ADR-0040), and the immutability transition table (ADR-0031) are
all correctly applied. The two load-bearing frontmatter claims (local ADRs omit
`producer`/`parent`; no `revision`/`repository`) are accurate against ADR-0016/
0017/0027 and the validator's `FM_BASE_FIELDS`. Remaining concerns are narrow: an
under-specified in-body status block, a Phase-3 supersession-block convention with
no corpus precedent, and a mis-stated validator failure mode.

**Strengths**:
- Frontmatter key/quoting prescriptions match the validator and live corpus
  exactly (quoted `id`/`title`, bare-int `schema_version`, bareword `status`,
  `adr_id` correctly avoided).
- Omitting `producer`/`parent` matches the live corpus and the enforced
  `FM_BASE_FIELDS`.
- Typed-linkage refs conform to ADR-0034; cross-type ADR→work-item `relates_to` is
  legitimate.
- File naming and script-driven allocation conform to ADR-0029; ascending order
  keeps targets pre-existing.
- ADR-0030 body-section ordering matches the template exactly.

**Findings**:
- 🔵 **minor** (high) — *Prescribed superseded status-block text has no precedent*
  (corpus grep for `**Status**: Superseded` → zero hits). *Loc: Phase 3 Manual
  Verification.*
- 🔵 **minor** (medium) — *In-body status-block shorthand omits the Date/Author
  lines* the three-line convention requires. *Loc: Phase 2 Changes Required (Body).*
- 🔵 **minor** (high) — *Validator failure for a frontmatter luminosity URL is
  mis-named* (BAD-LINKAGE-SHAPE, not DANGLING-REF; conclusion is correct). *Loc:
  Settled decisions.*
- 🔵 **suggestion** (medium) — *Wholesale `superseded` flip of ADR-0016 conflicts
  with the partial-supersession style* (ADR-0026 keeps `accepted` + blockquote);
  confirm ADR-0047 is a full superseder. *Loc: Phase 3 §2.*

### Architecture

**Summary**: The real architecture is the decision-record graph and the phase
pipeline that produces it. The decomposition (spikes → proposed ADRs → gated
supersession) is well-chosen: dependencies flow strictly forward, each phase is
independently validator-green, and the human gate is correctly placed where
immutability forces it. The supersession graph is coherent and the ADR-0001
bundled-concern handling is sound. The principal structural risk is importing
eleven proposed ADRs describing an unadopted architecture against a single
load-bearing assumption, with the validator providing no semantic guard on the
supersession edges.

**Strengths**:
- Phase boundaries map onto genuine dependency edges; sequencing follows the
  constraint structure, not arbitrary batching.
- Supersession graph reasoned at the concern level (ADR-0052 supersedes only
  ADR-0027; links ADR-0001 via `relates_to`), verified against ADR-0001's bundled
  decisions.
- Class-A vs Class-B distinction carried from research into editorial treatment.
- Two-layered reference rewrite resists a blind transform that would corrupt
  ADR-0047's config lineage.
- Each phase leaves `meta/` independently mergeable.

**Findings**:
- 🔴 **major** (high) — *Load-bearing Rust-CLI assumption underpins six ADRs but is
  not surfaced as a plan-level risk* (compressed to a one-line table cell; no
  fallback if 0136 does not land). *Loc: Settled decisions; Phase 2.*
- 🟡 **major** (high) — *Supersession edge correctness rests entirely on manual
  review* — the validator guards resolution only; a reciprocal-edge error passes
  every gate. *Loc: Phase 2 → Phase 3; Current State Analysis.*
- 🔵 **minor** (medium) — *Dropping the `content/` half narrows ADR-0052's scope
  without recording the omitted reasoning.* *Loc: Phase 2 ADR-0052; §D map.*
- 🔵 **minor** (medium) — *Class-B ADRs hard-code an unsettled CLI name*, coupling
  proposed records to an undecided 0136 choice. *Loc: Phase 2 ADR-0053/0054.*
- 🔵 **minor** (medium) — *Editing the toolchain count erodes a load-bearing premise
  of ADR-0055's argument* — strengthen the manual check to a negative grep. *Loc:
  Phase 2 ADR-0048/0055.*

### Test Coverage

**Summary**: The verification strategy is unusually well-considered for a
no-unit-test Markdown port: it names the validator as the primary gate, states its
limits explicitly, and routes judgement-heavy work to manual review. However it
materially overstates the validator's coverage of the port's highest-risk
operation — Phase 3 supersession — because `superseded_by` is not a validated ADR
linkage key, so none of Phase 3's automated assertions exercise the validator's
referential-integrity machinery. Several grep assertions are also weaker than the
checklist text claims, leaving the riskiest failure modes resting on manual review.

**Strengths**:
- TDD framing (write checks first) is the right discipline; each phase carries
  concrete runnable assertions.
- Honest about the automated/manual split (resolution vs. intended target).
- Phase ordering makes referential integrity satisfiable at every boundary.
- Negative assertions included where they matter (no leaked parent/blocks, no
  producer/parent keys, `jj diff --stat` guard).

**Findings**:
- 🔴 **critical** (high) — *Phase 3 `superseded_by` edges have zero automated
  referential safety net.* `superseded_by` is absent from the `adr` linkage keys
  (`templates-schema.tsv`; `frontmatter-emission-rules.sh` keeps it only as a
  closed-set guard), so it is never shape- or resolution-checked; the Phase 3 greps
  confirm only literal-string presence. *Loc: Phase 3 Automated Verification;
  Testing Strategy.*
- 🟡 **major** (high) — *Wrong-but-resolvable references are the most likely defect
  and the least covered* — the §D map (already an exhaustive expected-value list)
  could be turned into per-file `relates_to` greps. *Loc: Key Discoveries; Testing
  Strategy.*
- 🟡 **major** (high) — *Several Phase 2 grep assertions are weaker than their
  checklist text claims* (`grep -c` per-file not summed; negative `! grep` passes
  vacuously). Use `grep -l … | wc -l` = 11 + a glob-expansion gate. *Loc: Phase 2
  Automated Verification.*
- 🟡 **major** (medium) — *The leftover-`Luminosity` scan is structurally incapable
  of failing* (every ADR is required to contain a `…/luminosity/…` URL). Make it
  exclusionary. *Loc: Phase 2 Automated Verification.*
- 🔵 **minor** (high) — *Phase 3 transition-legality precondition has no assertion*
  — add a pre-flip `grep '^status: accepted'` gate. *Loc: Phase 3 §1.*
- 🔵 **minor** (medium) — *"Supersets declare intent" grep does not check the
  targets* — assert exact targets. *Loc: Phase 2 Automated Verification.*
- 🔵 **minor** (medium) — *Spike content-fidelity has no mechanical backstop* — add
  a content diff or date-anchor greps. *Loc: Phase 1 & 2 Manual Verification.*

---

## Re-Review (Pass 2) — 2026-06-27T14:45:41+00:00

**Verdict:** APPROVE

The plan was revised across two iterations: first reworking the supersession
model, then (on a re-run of all six lenses) fixing a swapped-facts defect and
resolving a convention conflict the deeper source-read surfaced. All previously
identified Critical/Major findings are now resolved, and the supersession model
is corpus-consistent with explicit assertions for the gaps the validator cannot
cover. This verdict reflects the post-remediation state of the plan.

### Previously Identified Issues

- 🔴 **Test Coverage** — `superseded_by` edges had zero referential safety net —
  **Resolved**. Supersession is now bilateral (matching ADR-0036/0043 and
  `create-adr --supersedes`); since the validator does not check `superseded_by`,
  Phase 3 adds an explicit `superseded_by`-target assertion plus a reverse-link
  invariant (every `status: superseded` ADR round-trips with its superset).
- 🟡 **Correctness** ×3 (hand-write path / no Phase-3 skill / reject branch) —
  **Resolved**. Phase 3 reproduces the sanctioned `create-adr --supersedes`
  frontmatter end-state by hand (owner-sanctioned, supersets already exist); the
  reject branch leaves the old ADR `accepted` with no `superseded_by`.
- 🟡 **Safety** ×2 (half-applied / status divergence) — **Resolved**. Reverse-link
  invariant + "only status + superseded_by changed" `jj diff` assertion catch a
  partial enactment; the frontmatter/in-body divergence is now explicitly
  documented as matching the ADR-0036 convention.
- 🟡 **Architecture** (Rust-CLI assumption) — **Resolved** via a new Risks and
  Assumptions subsection with mitigation and a Class-A/Class-B split.
- 🟡 **Architecture** (edge correctness on manual review) — **Resolved** by the
  §D exact-match `relates_to` assertion and the reverse-link invariant.
- 🟡 **Test Coverage** ×3 (weak greps / wrong-but-resolvable refs / leftover scan)
  — **Resolved**. Count idioms → `grep -l … | wc -l`; glob-expands-to-11 gate;
  runnable §D exact-`adr:`-set command; token-level (`grep -oE`) leftover scans.
- 🟡 **Documentation** (in-prose ordinals) — **Resolved + extended** (coverage now
  includes ADR-0045/0046/0048 and bare-prose work-item references, with a grep
  proxy).
- 🟡 **Standards** / 🔵 minors (status-block text, three-line block, BAD-LINKAGE-SHAPE
  rename, partial-supersession) — **Resolved** (bilateral model now matches the
  corpus; three-line block specified; rationale corrected).

### New Issues Introduced / Surfaced (this pass) — all addressed

- 🔴 **Documentation/Correctness** — the two date-sensitive spike facts were
  **swapped** (OpenAI/promptfoo is in lum 0003 → 0159; sigstore in lum 0002 →
  0158). **Verified against source and fixed** in the Phase 1 specs and the
  anchor greps.
- 🟡 **Documentation** — ADR-0045's "No cross-refs" note was wrong (it has a
  "decision 9" ordinal + bare work-item refs); the ordinal coverage list omitted
  ADR-0045/0046/0048. **Fixed** (spec corrected; universal-edit list expanded).
- 🔴/🟡 **Convention conflict** — the interim "status-only, no `superseded_by`"
  model contradicted the live corpus/tooling (ADR-0036 carries `superseded_by`).
  **Resolved by an explicit user decision** to adopt the bilateral pattern; all
  supersession sections and the false-precedent rationale were rewritten to match.
- 🔵 minors (line-granular `grep -v`; ADR-0049 "sole operative enforcement"
  framing; ADR-0016 stale-but-now-linked body note; producer/ADR-0040 divergence;
  title-prefix; ADR-0046 provenance wording) — **all addressed**.

### Assessment

The plan is now internally consistent, corpus- and tooling-consistent on
supersession, and validator-green (`validate-corpus-frontmatter.sh meta/` exits
0). The bilateral supersession model directly implements what multiple lenses
recommended rather than introducing new risk, and the assertion battery now
mechanises the highest-risk wrong-but-resolvable and half-applied-supersession
classes. One residual non-blocking follow-up (noted in the plan as a suggestion):
confirm corpus consumers — e.g. the visualiser — distinguish `proposed` from
`accepted` so the six Class-B ADRs are not read as adopted architecture while
0136 is unresolved. Ready for implementation.

---
*Re-review generated by /accelerator:review-plan*

---
type: plan-review
id: "2026-06-06-0067-create-note-skill-review-1"
title: "Plan Review: Create create-note Skill"
date: "2026-06-06T11:54:02+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-06-0067-create-note-skill"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [correctness, standards, test-coverage, architecture, documentation, usability, code-quality]
review_number: 1
review_pass: 3
tags: [skills, notes, templates, frontmatter, typed-linkage]
last_updated: "2026-06-06T12:32:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Create create-note Skill

**Verdict:** REVISE

The plan is exceptionally well-researched and its load-bearing mechanical
claims hold: every reviewer that traced the actual gate scripts confirmed
the proposed `note.md` TSV row, the linkage-comment regexes, the `captured`
status grep, the closed-set check, the cross-check union math, and the
skill-population field/omit assertions all behave as the plan states, with
sound TDD red→green sequencing and genuinely independently-mergeable phases.
The REVISE verdict is driven not by errors in the wiring but by a single
pervasive weakness — the `SKILL.md` is specified abstractly where both the
population gate and the precedent skills demand concrete structure and prose
— compounded by a verification command that does not test what it claims and
behavioural contracts that have no automated regression net. None of the
seven major findings is hard to fix; most are tightenings of under-specified
prose rather than design changes.

### Cross-Cutting Themes

- **`SKILL.md` is under-specified, in two distinct ways** (flagged by:
  correctness, standards, usability, code-quality) — The plan describes the
  skill's populate section, linkage bullets, elicitation flow, slug
  derivation, abort message and confirmation line only loosely. This has a
  *test-passing* face (the population gate's `in_imperative_section` and
  `in_populate_section_with_guidance` helpers impose exact structural
  conditions — a `#`-prefixed populate-ish heading, colon-anchored `field:`
  references, per-field whole-word `fill`/`omit` keywords) and a *DX/
  consistency* face (the precedent skills spell out greeting text, parameter
  checks, ownership prompts and a `Note created: {path}` confirmation that
  this plan leaves to the author). An authored-but-failing or
  authored-but-jarring skill is the most likely way this plan goes wrong.

- **The omit-when-empty authority (ADR-0040) is cited unevenly** (flagged
  by: documentation, standards) — The plan *correctly* identifies ADR-0040
  as the real authority and fixes the work item's ADR-0034 mis-citation in
  the SKILL.md guidance (a genuine strength). But the 0067 Schema Reference
  prose still names only "ADR-0033 and ADR-0034" as authoritative, omitting
  ADR-0040 for the very `parent`/`relates_to` omit behaviour it documents —
  while the *inline* ADR citation the plan adds to the SKILL.md diverges from
  every precedent skill, which carry no ADR numbers at all. The citation
  needs to be made consistent in both directions.

- **Hand-maintained parallel lists must stay in lockstep, and one comment
  goes stale** (flagged by: architecture, code-quality) — The note type is
  asserted across the TSV row, the 0067 Schema Reference table, the
  `WORK_ITEM_MDS` array, `IN_SCOPE_PRODUCERS`, and `plugin.json` with no
  single source of truth. The plan wires all of them correctly, but the
  `WORK_ITEM_MDS` edit leaves the script's own comment (lines 24-25: "both
  0065 and 0066…") contradicting the now-three-entry array.

- **Behavioural contracts rely on one-time manual checks, and one automated
  check is mis-scoped** (flagged by: test-coverage) — Routing dispatch
  (AC10), ownership-driven linkage placement (AC6), `source`/`derived_from`
  suppression in the skill prose (AC7), the path-existence guard, and slug
  quality (AC5) are all manual-only. Worse, the Phase 2 automated
  routing-keyword check greps the whole `SKILL.md` body rather than the
  `description` frontmatter that AC8 actually targets, so it passes even if
  the routing surface is wrong.

### Tradeoff Analysis

- **Path-collision: hard abort (consistency) vs friendly disambiguation
  (usability)**: The plan inherits create-work-item's abort-on-collision
  guard, which is internally consistent and matches the project's
  "VCS is the recovery path" stance. Usability argues same-day same-topic
  capture is an everyday non-race occurrence for notes and a bare abort
  discards the just-elicited body. Recommendation: keep the abort (do not
  add preview/confirm UX), but specify a *note-appropriate* abort message
  (the inherited "another session may have written concurrently" wording is
  meaningless for notes) and decide explicitly whether slug auto-
  disambiguation is in scope.

- **Inline ADR citation in SKILL.md (traceability) vs precedent (no ADR
  numbers in any skill)**: The plan wants the skill to cite ADR-0040 for the
  omit rule; no existing producer skill cites ADRs inline. Recommendation:
  drop the inline citation from the SKILL.md to match precedent and instead
  ensure the *Schema Reference table* (the documentation surface that does
  cite ADRs) names ADR-0040 — or consciously accept the divergence and note
  why.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness**: Populate-frontmatter section must meet the population
  test's exact structural conditions
  **Location**: Phase 2, Section 4 — Populate frontmatter section
  The `in_imperative_section` helper only treats a line as a section start
  inside its `/^#/` branch; the create-work-item precedent passes because its
  fields sit under a `## Step 5` heading with a Substitute verb. If the note
  skill lists `revision`/`repository`/etc. as prose without colon-anchored
  `field:` tokens under a `#`-headed section (or in a non-template fenced
  block), the population test fails — a green Phase 1 followed by a red
  Phase 2 the plan does not flag.

- 🟡 **Correctness**: Per-field `fill`/`omit` guidance binding is strict
  **Location**: Phase 2, Section 4 — per-field fill/omit bullets
  `in_populate_section_with_guidance` binds the keyword to each field's own
  bullet window and matches `fill`/`omit` only as a whole word. The plan's
  bullets work, but if the keyword lands on a line beginning a new list item,
  or appears as a substring (e.g. "backfill"), the omit-when-empty assertion
  for `parent`/`relates_to` fails despite the field being documented.

- 🟡 **Test Coverage**: Routing-keyword automated check greps the whole file,
  not the `description` frontmatter AC8 targets
  **Location**: Phase 2, Success Criteria → Automated Verification
  `grep -Eiq 'note' … && grep -Eiq 'capture|jot' …` scans the entire
  `SKILL.md`, which will contain those words regardless of whether the
  `description:` frontmatter (the routing surface AC8 specifies) carries them.
  The gate gives false confidence; scope it to the description line.

- 🟡 **Test Coverage**: No automated assertion that the skill never instructs
  emitting `source`/`derived_from` (AC7's skill-prose half)
  **Location**: Phase 2, Changes #4 and Testing Strategy
  AC7 is enforced at the template level (closed-set rejects those keys), but
  the *skill prose* contract has no automated coverage — the population test
  only asserts presence of declared fields and omit guidance. A future edit
  adding `source`/`derived_from` population guidance ships undetected; only a
  one-time manual live run covers it.

- 🟡 **Usability**: No-argument vs argument-provided behaviour behind
  `argument-hint` is never defined
  **Location**: Phase 2, Section 4 — Frontmatter & elicitation
  The skill advertises `argument-hint: "[note topic]"` but never says what
  happens when a topic arg is/ isn't passed. A user running
  `create-note "deploys are slow"` — the natural quick-capture path — may be
  re-asked for the topic, defeating the in-the-moment purpose; a no-arg run
  has no defined greeting.

- 🟡 **Usability**: Ownership-confirmation prompt wording is unspecified
  **Location**: Phase 2, Section 4 — Linkage handling
  AC6 hinges on how the user confirms a related artifact *owns* the note, but
  no prompt wording is given. The likely outcomes are a leading prompt
  ("Is this the parent? (y/n)", biasing toward yes and assuming the user
  knows `parent` semantics) or an ambiguous one — making a consequential
  schema distinction hard to answer correctly.

- 🟡 **Usability**: Path-existence guard is a bare hard-abort with no message
  or disambiguation path
  **Location**: Phase 2, Section 4 — Filename derivation
  The inherited create-work-item abort message ("another session may have
  written concurrently") is meaningless for notes, where same-day same-topic
  capture is an everyday occurrence, not a race. A user hits an opaque dead
  end and loses the just-elicited body. (See Tradeoff Analysis.)

#### Minor

- 🔵 **Correctness**: The 0067 Schema Reference row's first cell is lexically
  load-bearing — the awk extractor only matches a lowercase, backticked
  `note.md` with a leading space; a cosmetic edit could silently drop it from
  the union. **Location**: Phase 1, Section 2.
- 🔵 **Correctness**: The appended TSV row must use real TABs (7 columns, the
  final column holding the space-joined `parent relates_to`); a space-for-tab
  slip aborts the whole suite via the `NF != 7` self-check.
  **Location**: Phase 1, Section 1.
- 🔵 **Correctness**: `source`/`derived_from` absence is enforced only by the
  template closed-set check and manual AC7 — not by the skill-population test;
  the Success Criteria imply test coverage that doesn't exist.
  **Location**: Phase 2, Section 1.
- 🔵 **Test Coverage**: All four behavioural contracts (linkage placement,
  source suppression, path guard, routing) are merge-time-only manual checks
  with no repeatable regression net; the path guard is the most mechanically
  testable and could warrant a scripted assertion. **Location**: Testing
  Strategy → Manual Testing Steps.
- 🔵 **Test Coverage**: Filename-slug derivation (AC5 "meaningful kebab
  summary, not raw input") has no automated coverage and only a loose manual
  check. **Location**: Phase 1, Changes #4 / Success Criteria.
- 🔵 **Standards**: `plugin.json` inserts `"./skills/notes/"` before
  `"./skills/config/"`; the array has no enforced ordering, but `notes` would
  read more naturally next to its conceptual siblings (`research`/`work`).
  **Location**: Phase 2, Section 3.
- 🔵 **Standards**: The two omit-when-empty bullets must be separate top-level
  bullets, each with its own standalone "Fill … otherwise omit" clause — a
  merged/prose note would fail the gate for one key. **Location**: Phase 2,
  Section 4.
- 🔵 **Architecture**: A new top-level `skills/notes/` category hosts a single
  create-only skill; structurally consistent (the plugin groups by artifact
  family and nests where needed) but thin until list/show land.
  **Location**: Phase 2, Section 3 / Current State Analysis.
- 🔵 **Architecture**: The three data-driven gating surfaces are implicitly
  coupled with no single source of truth; the TSV/Schema-Reference cross-check
  is exact-equality, so any drift fails the suite. This is the standing
  extension tax, worth flagging as a checklist for the next artifact author.
  **Location**: Current State Analysis / Phases 1 & 2 wiring.
- 🔵 **Architecture**: The `skills-schema.tsv` row loop fails hard if the
  `SKILL.md` is absent, so the row and the skill are an inseparable Phase 2
  unit — unlike the discovery allowlist line, which is missing-file tolerant.
  Keep Phase 2 atomic. **Location**: Phase 2, Sections 1 & 2.
- 🔵 **Documentation**: The plan adds an inline ADR citation to the SKILL.md
  Populate section, but `create-work-item`/`research-codebase` carry no ADR
  numbers anywhere — a divergence from the convention it claims to mirror.
  **Location**: Phase 2, Section 4. (See Tradeoff Analysis.)
- 🔵 **Documentation**: The 0067 Schema Reference prose names "ADR-0033 and
  ADR-0034" as authoritative but omits ADR-0040, the actual authority for the
  omit-when-empty behaviour of the `parent`/`relates_to` slots it documents.
  **Location**: Phase 1, Section 2.
- 🔵 **Code Quality**: The `WORK_ITEM_MDS` edit leaves the script's own
  comment (lines 24-25, "both 0065 and 0066 carry Schema Reference tables")
  contradicting the now-three-entry array. **Location**: Phase 1, Section 3.
- 🔵 **Code Quality**: The `IN_SCOPE_PRODUCERS` append should be stated as
  "append last" so it stays row-aligned with the `skills-schema.tsv` append,
  preserving the two lists' positional diffability. **Location**: Phase 2,
  Section 2.
- 🔵 **Code Quality**: Comment-column alignment in the template is verified
  only by a manual checkbox (shfmt doesn't cover markdown frontmatter);
  copying `codebase-research.md`'s frontmatter verbatim and editing values in
  place preserves alignment mechanically. **Location**: Phase 1, Section 4.
- 🔵 **Code Quality**: Slug-derivation and directory-creation edge behaviour
  is underspecified relative to its create-work-item precedent (degenerate
  topic → empty slug; abort message text); point the author at
  create-work-item's exact slug rule and collision block. **Location**:
  Phase 2, Section 4.
- 🔵 **Usability**: Post-write confirmation line format is unspecified; mirror
  the precedent's `Note created: {path}` so the output path is actionable.
  **Location**: Phase 2, Section 4 — Write step.
- 🔵 **Usability**: The elicitation flow lacks the concrete greeting/prompt
  scaffolding both precedents provide, and doesn't say whether the four inputs
  are batched or sequential — risking a multi-round-trip first run.
  **Location**: Phase 2, Section 4.
- 🔵 **Usability**: Routing keyword "capture" overlaps create-work-item's
  description ("capturing a … work item"); lean on the note-distinctive verbs
  ("jot"/"note") and keep "short-form note in meta/notes/" prominent.
  **Location**: Phase 2, Section 4 / AC9-AC10.

#### Suggestions

- 🔵 **Standards**: Record the deliberate `work_item_id` omission (a departure
  from the codebase-research.md/rca.md exemplars, ADR-grounded) in the 0067
  Schema Reference / drafting notes so a future reader sees it is intentional.
  **Location**: Phase 1, Section 4 — Notes on shape.
- 🔵 **Architecture**: Briefly record the author/provenance rationale
  (code-state-anchored types carry the VCS-derived `author` by convention),
  since notes are the first author-authored *and* provenance-bearing type and
  set a precedent. **Location**: Phase 1, Section 4.
- 🔵 **Documentation**: The note template's `relates_to` comment can only show
  `work-item:NNNN` (the `note:` token is rejected by SOURCE_TYPE_RE), so it
  never illustrates the plausibly-common note→note relation; consider noting
  the constraint or a path-form example. **Location**: Phase 1, Section 4.
- 🔵 **Documentation**: The cross-check only compares the table's first cell;
  `type`/`schema_version`/provenance/extras are unverified by any test, so
  make the manual-verification step explicit that those columns need a
  by-hand check against the template and ADRs. **Location**: Phase 1,
  Section 2.
- 🔵 **Code Quality**: `allowed-tools` grants the broad `artifact-*` glob
  whose only consumer is `artifact-derive-metadata.sh`; keep it (house style)
  but note the breadth is intentional. **Location**: Phase 2, Section 4.

## Re-Review (Pass 2) — 2026-06-06T12:22:42+00:00

**Verdict:** REVISE

All seven major findings from the initial review are resolved. However, the
revisions introduced four new major findings, all clustered in the two areas
that changed most — the routing-keyword automated check and the new
auto-disambiguation collision behaviour. None are structural; each is a
tightening of a just-added specification.

### Previously Identified Issues

- 🟡 **Correctness**: Populate-section structural conditions — **Resolved.**
  Phase 2 §4 now pins a real `#`-prefixed `POPULATE_HEADING_RE`-matching
  heading, colon-anchored `field:` tokens with an imperative verb, and
  per-field bullets; verified to satisfy `in_imperative_section` /
  `in_populate_section_with_guidance`.
- 🟡 **Correctness**: Per-field `fill`/`omit` binding — **Resolved** (with a
  new minor refinement: the match is case-sensitive lowercase, so only the
  lowercase `omit` counts — see New Issues).
- 🟡 **Test Coverage**: Routing-keyword check grepped the whole file —
  **Partially resolved.** Now scoped to the frontmatter block, but the
  `'note'` half still false-passes because `name: create-note` always
  contains the substring (see New Issues).
- 🟡 **Test Coverage**: No automated AC7 assertion — **Resolved.** A negative
  grep (`! grep -Eq '\`(source|derived_from):\`'`) was added with a
  prohibition-prose constraint verified sound.
- 🟡 **Usability**: No-arg vs argument-provided behaviour — **Resolved.** An
  explicit parameter-check step (arg → topic, slug, skip prompt; no-arg →
  defined greeting) was added.
- 🟡 **Usability**: Ownership-confirmation prompt wording — **Resolved.** A
  neutral `[owns / related]` prompt defaulting to `relates_to`, with the
  leading `(y/n)` form explicitly prohibited.
- 🟡 **Usability**: Path-existence guard hard-abort — **Resolved** as a design
  decision (auto-disambiguate instead of abort), but the new behaviour brought
  its own gaps (see New Issues).
- 🔵 **Minors/suggestions** (stale lines 24-25 comment, `IN_SCOPE_PRODUCERS`
  append-last ordering, ADR-0040 in the Schema Reference prose, dropped inline
  ADR citation, `plugin.json` placement rationale, single-skill-category
  framing, `work_item_id` omission note, author/provenance rationale,
  copy-frontmatter-verbatim, `relates_to` token constraint, confirmation-line
  format, behavioural-contracts merge-time-only note, AC5 slug check) —
  **All resolved.** Standards, documentation, architecture, and code-quality
  lenses returned no major findings this pass; their remaining items are
  minors and out-of-scope follow-ups (e.g. recording the author/provenance
  pairing in ADR-0033; consolidating the three artifact-registration
  surfaces; adding `note` to `SOURCE_TYPE_RE` once an in-scope artifact links
  to a note).

### New Issues Introduced

- 🔴 **Correctness / Test Coverage**: The AC8 frontmatter routing grep for
  `'note'` is **inert** — the extracted frontmatter always contains
  `name: create-note`, so `grep -Eiq 'note'` passes unconditionally and
  cannot catch a description that omits the keyword (the exact regression the
  check was added to fix). The `capture|jot` half is meaningful; the `note`
  half is not. **Fix**: scope the grep to the `description:` value (line +
  YAML continuation lines), not the whole frontmatter block.
- 🟡 **Test Coverage**: The new auto-disambiguation collision behaviour — the
  story's most data-loss-adjacent branch — has **no standing automated
  guard**, only a single merge-time manual step against LLM-driven prose.
  **Fix**: add a cheap non-mention regression net (e.g. assert the SKILL.md
  carries no inherited abort-on-collision phrasing, or carries a
  disambiguation instruction token).
- 🟡 **Usability**: On a disambiguated write the user is **not guaranteed to
  learn the final path differs** from the topic-derived slug; the confirmation
  line is specified generically. **Fix**: require the confirmation to print
  the literal final path and signal that the slug was qualified to avoid a
  same-day collision.
- 🟡 **Usability / Code Quality**: The disambiguation qualifier is specified
  **two divergent ways** (`-2`/`-3` numeric suffix *or* a "brief distinguishing
  token"); the token form is non-deterministic and conflicts with the skill's
  own deterministic-path / no-random-suffix contract. **Fix**: pin a single
  deterministic strategy — an incrementing numeric suffix that probes
  until the first free path — and drop the token alternative.

### New Minor Findings

- 🔵 **Correctness**: `in_populate_section_with_guidance` matches `fill`/`omit`
  case-sensitively; the example bullets pass only via the lowercase `omit`, so
  the constraint should state lowercase `fill`/`omit` verbatim. Location:
  Phase 2 §4.
- 🔵 **Correctness**: `last_updated` needs its own colon-anchored
  `` `last_updated:` `` token distinct from `` `last_updated_by:` `` (the
  latter does not satisfy the former's assertion). Location: Phase 2 §4.
- 🔵 **Correctness/Architecture**: A note carrying `parent`/`relates_to` to a
  work-item inverts ADR-0034's canonical work-item→note edge direction
  (note as extraction-origin target); the vocabulary permits it, but confirm
  the intended semantic and that note-side links remain discoverable.
  Location: Current State / Phase 1 §4.
- 🔵 **Documentation**: Prefer the *generalised* wording for the lines 24-25
  comment rewrite over re-enumerating work-item numbers, to avoid re-creating
  the staleness. Location: Phase 1 §3.
- 🔵 **Code Quality**: The dense Populate-section *test-mechanics* commentary is
  authoring guidance only — instruct that it must not be transcribed into the
  user-facing SKILL.md. Location: Phase 2 §4.

### Assessment

The plan is very close. Every concern from the first pass was addressed
cleanly, and the standards/documentation/architecture/code-quality lenses are
now major-finding-free. The four new majors are all narrow consequences of the
two specific changes made between passes (the routing-grep scoping and the
auto-disambiguation behaviour) — the routing grep needs to target the
`description:` value rather than the whole frontmatter, and the
auto-disambiguation needs one deterministic algorithm, a user-visible final
path, and a cheap regression net. These are quick, self-contained fixes; once
applied the plan should reach APPROVE.

## Re-Review (Pass 3) — 2026-06-06T12:32:52+00:00

**Verdict:** APPROVE

Focused verification pass on the three lenses that carried pass-2 majors
(correctness, test-coverage, usability). **All four pass-2 majors are
resolved and no new major was introduced** — all three lenses returned zero
major findings, only minors and suggestions, which have now also been
addressed.

### Previously Identified Issues (pass-2 majors)

- 🔴 **Correctness / Test Coverage**: AC8 `'note'` frontmatter grep inert —
  **Resolved.** The check now isolates the `description:` value via
  `awk '/^description:/{d=1;print;next} d&&/^[[:space:]]/{print;next} {d=0}'`;
  both lenses independently traced the awk and confirmed it excludes
  `name: create-note` and tests the routing surface.
- 🟡 **Test Coverage**: Auto-disambiguation had no automated guard —
  **Resolved.** A `grep -Eiq 'disambiguat'` floor was added and verified to
  match the skill's own vocabulary, correctly framed as a floor backstopped by
  the manual collision step.
- 🟡 **Usability**: User not told the path was disambiguated — **Resolved.**
  The Write step now prints the literal final path and a non-alarming signal
  that the slug was adjusted (now also pointing at the pre-existing note).
- 🟡 **Usability / Code Quality**: Two divergent disambiguation strategies —
  **Resolved.** Pinned to a single deterministic incrementing-numeric-suffix
  probe-until-free; the non-deterministic token alternative was dropped.

### New Issues Introduced

None at major severity. The pass-3 lenses surfaced only minors, all now
addressed:
- 🔵 **Correctness / Test Coverage**: The example `fill`/`omit` bullets used
  capitalised "Fill", contradicting the plan's own case-sensitive-lowercase
  rule and passing only incidentally via the trailing lowercase "omit" —
  **fixed** (bullets now use lowercase `fill`/`omit` verbatim, with an
  explicit reminder).
- 🔵 **Usability**: Possible double-prompt ambiguity between the no-argument
  greeting and the elicitation step — **fixed** (the greeting is now stated to
  be the single elicitation turn).
- 🔵 **Usability**: Disambiguation message could point at the pre-existing
  note; the `-N` family is expected — **fixed** (message now references the
  earlier path; a note records the suffix family is acceptable and must not be
  capped).
- 🔵 **Correctness/Test-Coverage (suggestions, not applied)**: optionally
  tighten the second routing grep to `jot` specifically — deliberately kept as
  `capture|jot` for AC9 conformance, since the description-scoped `note` grep
  already pins the note↔work-item disambiguator; and noting routing keywords
  must sit on the `description:` line/continuation, which the awk already
  enforces.

### Assessment

The plan is ready for implementation. Across three passes the original 7
majors and the 4 regression-introduced majors are all resolved, every lens is
major-finding-free, and the remaining items were minors that have been
applied. The plan is precise about the data-driven gates it must satisfy, the
skill's interaction surface, and the deliberate departures from
`create-work-item` (auto-disambiguation, no allocator, no TEMPLATE_KEYS), each
with recorded rationale. **Verdict: APPROVE.**

---
*Re-review generated by /accelerator:review-plan*

### Strengths

- ✅ The cross-check union math is correct: 0065/0066 list 12 templates
  (none being `note.md`), so adding `note.md` to both the TSV and a new 0067
  Schema Reference table keeps `wi_templates == tsv_templates` with no
  duplicate, even though the union uses `sort` rather than `sort -u`.
- ✅ The `parent`/`relates_to` comment lines exactly satisfy
  `check_linkage_slot`'s single/list regexes, and the `work-item:NNNN` token
  is correctly chosen because `work-item` is in `SOURCE_TYPE_RE` while `note`
  deliberately is not — the reasoning is precisely right.
- ✅ Correctly resolves the omit-when-empty authority to ADR-0040 in the
  SKILL.md guidance, fixing (rather than propagating) the work item's
  ADR-0034 mis-attribution.
- ✅ Avoids a false-green trap: registering `note` in `TEMPLATE_KEYS` would
  break the order-locked assertion at `test-config.sh:2461`; the plan
  excludes it and relies on file-existence-based template resolution.
- ✅ TDD red→green is concretely achievable (verified against the gate
  scripts' file-existence guard and union cross-check), and the two phases
  are genuinely independently mergeable and main-green at each step.
- ✅ The notes-as-extraction-origin asymmetry (notes never own
  `source`/`derived_from`) is enforced structurally — the template omits the
  slots, `SOURCE_TYPE_RE` is left unchanged, and AC7 is a pass/fail gate.
- ✅ "What We're NOT Doing" is exhaustive and self-justifying, pre-empting
  over-engineering (no allocator, no TEMPLATE_KEYS, no list/show) with
  precedent-tied reasons — strong YAGNI discipline.
- ✅ Every edit is anchored to an explicit precedent (codebase-research.md
  template, research-codebase TSV row, create-work-item skill), keeping the
  change reviewable by analogy.

### Recommended Changes

1. **Pin the Populate-frontmatter section's structure** (addresses: both
   Correctness majors, Standards per-field-bullet minor) — Specify that the
   section uses a `#`-prefixed heading matching the population test's
   `POPULATE_HEADING_RE`, renders every `fields_to_assert` entry as a
   colon-anchored bullet (`` - `revision:` ← … ``) with a Substitute/
   Populate/Set verb, and that `parent:`/`relates_to:` are each a separate
   top-level bullet carrying a standalone whole-word "Fill … otherwise omit"
   clause (not a continuation line that begins a new list item, no buried
   substrings).

2. **Specify the skill's interaction surface concretely** (addresses:
   Usability majors on arg-handling, ownership prompt, path-guard; Usability
   minors on confirmation line and elicitation; Code-Quality slug minor) —
   Add a parameter-check step (arg present → treat as topic, skip the topic
   prompt; absent → defined greeting); give a neutral ownership-confirmation
   prompt that explains `parent` vs `relates_to` in plain terms and defaults
   to `relates_to`; specify a note-appropriate abort message (and decide
   whether slug auto-disambiguation is in scope); specify the
   `Note created: {path}` confirmation; point the author at
   create-work-item's exact slug rule.

3. **Fix the routing-keyword automated check** (addresses: Test-Coverage
   routing-grep major) — Scope the grep to the `description:` frontmatter
   line (extract the YAML block first) so the gate actually exercises the
   AC8 routing surface rather than incidental body prose.

4. **Decide and document the `source`/`derived_from` skill-prose coverage**
   (addresses: Test-Coverage AC7 major, Correctness AC7 minor) — Either add
   a lightweight grep-based negative assertion that the SKILL.md carries no
   `source:`/`derived_from:` population instruction, or explicitly record in
   the Testing Strategy that this half of AC7 is manual-only and why the
   residual risk is accepted.

5. **Make the ADR-0040 citation consistent** (addresses: Documentation ×2,
   the citation tradeoff) — Add ADR-0040 to the 0067 Schema Reference
   "Authoritative source" line, and decide whether the inline SKILL.md ADR
   citation stays (departing from precedent) or is dropped to match the
   no-ADR-numbers convention of sibling skills.

6. **Update the stale gate comment and pin list ordering** (addresses:
   Code-Quality stale-comment + IN_SCOPE_PRODUCERS ordering minors,
   Architecture coupling minor) — Have Phase 1 Section 3 also update
   `test-template-frontmatter.sh:24-25` to name 0065/0066/0067 (or
   generalise it), and state that the `IN_SCOPE_PRODUCERS` entry is appended
   last to stay row-aligned with the `skills-schema.tsv` append.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan's correctness rests almost entirely on whether its
precise assertions about three data-driven gates hold, and each was traced
against the actual scripts: the 7-field TSV row, the linkage-slot regexes,
the status-vocab grep, the closed-set check, the cross-check union math
(`sort`, not `sort -u`; no duplicate `note.md` introduced), the awk
Schema-Reference extractor, and the skill-population field/omit assertions
all check out exactly as the plan claims. The TDD red/green ordering within
each phase is logically coherent. The residual risk is in the
under-specified SKILL.md prose: the population test's `in_imperative_section`
/ `in_populate_section_with_guidance` helpers impose exact structural
conditions the plan describes only loosely.

**Strengths**:
- The cross-check union math is correct (0065+0066 list 12 templates, none
  `note.md`; adding it to both TSV and a 0067 table keeps the union equal
  with no duplicate despite `sort` not `sort -u`).
- The `parent`/`relates_to` comments exactly satisfy `check_linkage_slot`'s
  regexes; the `work-item:NNNN` token is correct because `note` is
  deliberately absent from `SOURCE_TYPE_RE`.
- Closed-set analysis sound: only `parent`/`relates_to` are linkage-
  vocabulary keys present and both are declared; `work_item_id` is correctly
  outside `LINKAGE_VOCABULARY` so its omission is inert.
- TDD ordering is watertight — the intermediate state produces red solely
  from the file-not-found branch while cross-check and discovery stay green.

**Findings**:
- **major** (medium): Populate section must meet the population test's exact
  structural conditions (`#`-headed section + colon-anchored field refs).
  `in_imperative_section` only starts a section in its `/^#/` branch; prose
  listing of provenance fields without `field:` tokens fails the gate.
  Location: Phase 2, Section 4.
- **major** (medium): Per-field `fill`/`omit` binding is strict (own bullet
  window, whole-word match). A keyword on a new-list-item line or a substring
  like "backfill" fails the omit assertion. Location: Phase 2, Section 4.
- **minor** (high): The 0067 table's first cell is lexically load-bearing for
  the awk extractor (lowercase, backticked, leading space). Location:
  Phase 1, Section 2.
- **minor** (high): The appended TSV row must be TAB-separated (7 columns);
  a space slip aborts the suite via the `NF != 7` self-check. Location:
  Phase 1, Section 1.
- **minor** (medium): `source`/`derived_from` absence is enforced by the
  template closed-set and manual AC7 only — not the skill-population test;
  Success Criteria imply coverage that doesn't exist. Location: Phase 2,
  Section 1.

### Standards

**Summary**: The plan is exceptionally well-grounded in established
conventions: the note template spine mirrors codebase-research.md
field-for-field, the linkage comment grammar matches `check_linkage_slot`
exactly, the skill structure follows the create-work-item/research-codebase
precedent, and registration into the three gates is correct. The single most
important standards call — that omit-when-empty authority lives in ADR-0040,
not ADR-0034 — is handled correctly. Remaining observations are minor.

**Strengths**:
- Template spine matches the codebase-research.md/rca.md exemplars
  field-for-field (id quoted, schema_version bare integer last, status
  comment, linkage header).
- Linkage comments exactly match the single/list regexes including the
  curated `SOURCE_TYPE_RE` token requirement.
- Correctly resolves omit-when-empty authority to ADR-0040, fixing the work
  item's ADR-0034 mis-attribution rather than propagating it.
- Skill conventions faithfully mirrored (allowed-tools globs, config-read-*
  injections, numbered steps, `### Populate frontmatter` heading, closing
  instructions injection).
- Correctly identifies that `note` is deliberately absent from
  `TEMPLATE_KEYS` and that registration would break `test-config.sh:2461`.

**Findings**:
- **minor** (medium): `plugin.json` places `notes` before `config`; no
  enforced ordering, but it would read better next to `research`/`work`.
  Location: Phase 2, Section 3.
- **minor** (medium): The two omit-when-empty bullets must each be a separate
  top-level bullet with its own standalone Fill/omit clause. Location:
  Phase 2, Section 4.
- **suggestion** (high): Record the deliberate `work_item_id` omission (an
  ADR-grounded departure from the exemplars) so it reads as intentional.
  Location: Phase 1, Section 4.

### Test Coverage

**Summary**: The TDD red→green sequencing is genuinely achievable — adding
the TSV row (and wiring 0067 into `WORK_ITEM_MDS` plus its table) before
authoring each artifact produces a real red via the file-existence guard and
union mismatch, and authoring turns it green. Reusing the existing
negative-fixture self-tests is well-justified. The principal gaps are
behavioural: runtime contracts (routing, ownership linkage, source
suppression, path guard) rest on one-time manual checks, and one automated
check is weaker than the AC it claims to cover.

**Strengths**:
- TDD red→green is concrete, verified against the gate scripts' file-
  existence guard and the union cross-check.
- Correctly avoids the `TEMPLATE_KEYS` false-green trap (order-locked
  assertion at `test-config.sh:2461`).
- Reusing the existing self-tests is sound — the `captured` vocab and
  `work-item:NNNN` token traverse already-covered pure-function paths.
- Phase success criteria name exact runnable commands.

**Findings**:
- **major** (high): Routing-keyword check greps the whole file, not the
  `description` frontmatter AC8 targets — passes even if the routing surface
  is wrong. Location: Phase 2 Success Criteria.
- **major** (high): No automated assertion that the skill never instructs
  emitting `source`/`derived_from` (AC7's skill-prose half) — manual-only.
  Location: Phase 2, Changes #4 / Testing Strategy.
- **minor** (high): All four behavioural contracts are merge-time-only manual
  checks with no regression net; the path guard is the most testable.
  Location: Testing Strategy.
- **minor** (medium): Filename-slug derivation (AC5) has no automated
  coverage and only a loose manual check. Location: Phase 1, Changes #4.

### Architecture

**Summary**: Architecturally sound and unusually well-grounded — correctly
applies the template-as-authoring-surface / producer-as-emission-policy
split, justifies the no-allocator and no-TEMPLATE_KEYS decisions against
verified precedent, and decomposes work into two independently-mergeable
phases. The decisions worth scrutiny are the thin single-skill category and a
few implicit couplings to the gating surfaces, none blocking.

**Strengths**:
- Correct ADR-0040 template/producer split (template keeps empty slots; skill
  omits when empty).
- No-allocator/no-TEMPLATE_KEYS decisions verified against concrete precedent
  (file-existence resolution, six unregistered templates, order-locked
  assertion).
- Phases genuinely independently mergeable; intra-phase red-before-green
  correct.
- Notes-as-extraction-origin asymmetry enforced structurally (template omits
  source/derived_from, SOURCE_TYPE_RE unchanged, AC7 gate).
- Curated-token decision (`work-item:NNNN` not `note:`) correctly derived
  from the SOURCE_TYPE_RE constraint.

**Findings**:
- **minor** (medium): New top-level `skills/notes/` category hosts a single
  create-only skill — consistent but thin until list/show land. Location:
  Phase 2, Section 3.
- **minor** (high): Three implicitly-coupled gating surfaces with no single
  source of truth; the TSV/Schema-Reference cross-check is exact-equality.
  Worth flagging as the standing extension tax. Location: Current State /
  Phases 1 & 2.
- **minor** (high): The `skills-schema.tsv` row loop fails hard on a missing
  SKILL.md, so row+skill are an inseparable Phase 2 unit (unlike the
  missing-file-tolerant discovery line); keep Phase 2 atomic. Location:
  Phase 2, Sections 1 & 2.
- **suggestion** (medium): Record the author/provenance rationale (code-state-
  anchored ⇒ VCS-derived author), since notes set this precedent. Location:
  Phase 1, Section 4.

### Documentation

**Summary**: The plan produces well-considered author-facing documentation —
template comments, SKILL.md Populate guidance, and the 0067 Schema Reference
table are mostly accurate and grounded in the right precedents. The table
content matches the template, and the no-templates.note-row decision is
documented coherently. The main concerns are an ADR-citation inconsistency:
the plan adds an inline ADR citation the precedent skills don't carry, while
the Schema Reference prose omits ADR-0040 — the actual omit-when-empty
authority.

**Strengths**:
- The 0067 Schema Reference table is accurate and self-consistent
  (type=note, schema_version=1, provenance=yes, extras=topic).
- The no-templates.note-row decision is documented coherently in two
  coordinated places with rationale.
- Template comments and SKILL.md fill/omit bullets faithfully modelled on the
  exemplars.
- Correctly documents why the linkage comment uses the curated
  `work-item:NNNN` token rather than own-type `note:`.

**Findings**:
- **minor** (high): The plan adds an inline ADR citation to the SKILL.md
  Populate section, but create-work-item/research-codebase carry no ADR
  numbers anywhere — a divergence from the convention it mirrors. Location:
  Phase 2, Section 4.
- **minor** (high): The Schema Reference prose names only ADR-0033/0034 as
  authoritative, omitting ADR-0040 for the omit behaviour it documents.
  Location: Phase 1, Section 2.
- **suggestion** (medium): The `relates_to` comment can only show
  `work-item:NNNN` (the `note:` token is rejected), so it never illustrates
  note→note; consider noting the constraint or a path-form example. Location:
  Phase 1, Section 4.
- **suggestion** (medium): The cross-check only compares the table's first
  cell; make the manual step explicit that type/schema_version/provenance/
  extras are unverified by any test. Location: Phase 1, Section 2.

### Usability

**Summary**: Structurally sound — mirrors a proven precedent, reuses wired
config/path/template machinery, and correctly scopes the skill to fast
capture. But the plan under-specifies the interaction surface: no-arg vs
arg-provided behaviour is undefined, the ownership-confirmation prompt and
confirmation line have no wording, and the path-existence guard is a bare
hard-abort with no message or disambiguation. These risk a schema-conformant
but jarring skill.

**Strengths**:
- Deliberately strips create-work-item ceremony, matching in-the-moment
  capture intent.
- Reuses existing primitives (paths.notes, file-existence template loading,
  artifact-derive-metadata.sh) — zero new config.
- Linkage defaulting (relates_to default; parent only on confirmation; never
  infer) is the least-surprising default.
- Routing keyword strategy is discoverable and convention-consistent.

**Findings**:
- **major** (high): No-arg vs argument-provided behaviour behind
  `argument-hint` is never defined. Location: Phase 2, Section 4.
- **major** (medium): Ownership-confirmation prompt wording unspecified,
  risking a leading/confusing prompt. Location: Phase 2, Section 4.
- **major** (medium): Path-existence guard is a bare hard-abort with no
  note-appropriate message or disambiguation; discards the elicited body.
  Location: Phase 2, Section 4.
- **minor** (high): Post-write confirmation line format unspecified; mirror
  `Note created: {path}`. Location: Phase 2, Section 4.
- **minor** (medium): Elicitation flow lacks the concrete greeting/prompt
  scaffolding the precedents provide; batched vs sequential unspecified.
  Location: Phase 2, Section 4.
- **minor** (low): Routing keyword "capture" overlaps create-work-item;
  lean on "jot"/"note". Location: Phase 2, Section 4 / AC9-AC10.

### Code Quality

**Summary**: Unusually disciplined for data-driven config/test maintenance —
every edit is minimal, mirrors a named precedent, and is wired into the gates
that enforce it. The main risks are hand-maintained parallel-list drift (TSV
rows, gate arrays, Schema Reference tables that must stay in lockstep, plus
one stale comment) and a couple of under-specified authoring details. None
blocking.

**Strengths**:
- Every artifact anchored to an explicit precedent — reviewable by analogy.
- "What We're NOT Doing" is exhaustive and self-justifying (YAGNI
  discipline).
- TDD ordering keeps each phase independently mergeable and main green.
- Correctly chooses the `work-item:NNNN` curated token over introducing a
  `note:` source-type, avoiding a cross-cutting vocabulary edit.

**Findings**:
- **minor** (high): Three/four hand-maintained parallel lists must stay in
  lockstep with no single source of truth; add a sentence naming the gate as
  the consistency enforcer. Location: Phase 1, Sections 1 & 2.
- **minor** (high): The `WORK_ITEM_MDS` edit leaves the script's lines 24-25
  comment ("both 0065 and 0066…") stale. Location: Phase 1, Section 3.
- **minor** (medium): The `IN_SCOPE_PRODUCERS` append should be stated as
  "append last" to stay row-aligned with the TSV. Location: Phase 2,
  Section 2.
- **minor** (medium): Comment-column alignment is manual-only; copy the
  exemplar frontmatter verbatim and edit in place. Location: Phase 1,
  Section 4.
- **minor** (medium): Slug-derivation/directory-creation edge behaviour
  underspecified vs the create-work-item precedent. Location: Phase 2,
  Section 4.
- **suggestion** (high): `allowed-tools` grants the broad `artifact-*` glob
  whose only consumer is `artifact-derive-metadata.sh`; keep it (house style)
  but note the breadth is intentional. Location: Phase 2, Section 4.

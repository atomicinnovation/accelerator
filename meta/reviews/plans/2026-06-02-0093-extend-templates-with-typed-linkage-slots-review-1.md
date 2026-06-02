---
date: "2026-06-04T12:38:29+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-06-02-0093-extend-templates-with-typed-linkage-slots.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, correctness, test-coverage, documentation, standards, usability, portability]
review_pass: 3
status: complete
---

## Plan Review: Extend Templates With Typed-Linkage Slots

**Verdict:** REVISE

This is an unusually thorough, well-structured plan: the seventh TSV column is the right closed-set mechanism, the always-emitted/omit-when-empty boundary is explicitly tabulated, the phases are reviewable in isolation, and the consumer-impact analysis is grounded in the visualiser's actual parser behaviour. However, two gating defects sit at its centre. First, the list-cardinality comment regex is anchored with `\[\]$`, but the only list slot carrying the inverse-key trailing sentence (`blocked_by`) has text *after* the `[]` — so the assertion can never match the very template the plan ships, and Phase 1 cannot reach green. Second, because this is a plan whose deliverable *is* test assertions, the absence of any negative fixture (and reliance on "no FAIL lines" as the green signal) means inert or broken assertions would pass silently. A third high-confidence issue — the closed-set check false-failing on `design-inventory`'s non-linkage `source:` field — also blocks Phase 1. None of these are architectural; they are precise, fixable contradictions between the spec, the test logic, and the shipped artefacts.

### Cross-Cutting Themes

- **The `blocked_by` regex/template contradiction** (flagged by: Correctness 🔴, Test Coverage 🔴, Documentation 🟡, Standards 🔵, Usability 🟡) — five lenses independently hit the same line. The list regex ends `...or[[:space:]]+\[\]$`; the shipped `blocked_by` line appends ` Producers SHOULD prefer the canonical side ("supersedes" / "blocks").` after the `[]`. The anchored `$` makes the main assertion unsatisfiable, the spec is internally inconsistent, and (per Usability) the line is also the least readable comment in the whole vocabulary despite carrying its most behaviourally-important instruction.
- **Assertions-as-deliverable, never proven to fail** (flagged by: Test Coverage 🔴🟡, Code Quality 🟡) — no negative fixtures exercise the closed-set, comment-grammar, or guidance checks; the Phase 2/3 "no-op until Phase 4" design ships assertion code that never runs; and the loose `/fill|omit/` section-wide match plus "no FAIL lines" success criteria mean a broken or inert assertion looks identical to a passing one.
- **Cross-skill ref-form consistency** (flagged by: Standards 🟡, Usability 🔵, Architecture 🔵) — the bare-id→typed-form normalisation is applied unevenly: Phase 6 names only `refine-work-item` and `create-plan`, but Phase 3's bullets silently give `create-work-item`/`extract-work-items` the typed `"work-item:NNNN"` form, and existing artifacts keep bare-id — so the corpus will mix `parent: "0057"` and `parent: "work-item:0057"` with no canonical answer recorded.

### Tradeoff Analysis

- **Enforcement rigour vs scope** — Architecture and Test Coverage both want a fixture/artifact-level backstop proving the omit-when-empty contract holds; the plan deliberately keeps enforcement at the "SKILL.md prose mentions fill/omit" level and defers corpus migration to 0070. This is a legitimate scope boundary, but the *absence of enforcement* should be a recorded, conscious tradeoff rather than an implicit gap — especially since downstream consumer 0070 may assume the convention is enforced.
- **Portability floor vs convenience** — the associative-array cardinality map is the cleanest expression of the rule (Code Quality likes it), but it raises the script's bash floor to 4.0+ and breaks default macOS bash 3.2 (Portability). A `case` statement carries identical logic at the cost of a little elegance; given CI is ubuntu-only and won't catch the regression, the portable form is the safer call.

### Findings

#### Critical

- 🔴 **Correctness / Test Coverage / Documentation / Standards**: Inverse-key list regex `\[\]$` can never match the `blocked_by` line that carries the trailing sentence
  **Location**: Normative comment grammar (list ERE, ~line 197) vs Phase 1 §3 work-item.md example (~line 568)
  The list regex anchors `[]` to end-of-line, but `blocked_by` (a *list* slot) ships with the inverse sentence appended after `[]`. `assert_in_block` runs `grep -qE` line-by-line, so the `$` anchor fails on every `blocked_by` slot (work-item.md, plan.md). Phase 1's TDD cycle never reaches green, blocking the whole plan.

- 🔴 **Test Coverage**: New assertions are never exercised by a negative fixture; green is not a sound signal
  **Location**: Phase 1 / Phase 2 / Phase 3 Success Criteria ("no FAIL lines"); closed-set check §2(d)
  The closed-set, comment-grammar, and guidance assertions are only ever run against templates/skills made to pass. No fixture feeds a spurious key, a wrong cardinality (`parent: []`), a malformed comment, or a missing fill/omit note. A broken assertion (regex matching nothing, loop iterating zero times) produces zero FAIL *and* zero PASS lines — indistinguishable from success. For a plan whose deliverable is assertions, this gives false confidence.

#### Major

- 🟡 **Architecture**: Closed-set check collides with `design-inventory`'s non-linkage `source:` field
  **Location**: Phase 1 §2(d) closed-set check (~lines 518–525)
  The check walks a global `LINKAGE_VOCABULARY` that includes `source`, greps each template for `^source:`, and FAILs if the key is absent from that row's `typed_linkage_keys`. `design-inventory.md` carries `source: "{source-id}"` as a foreign-source *extra* (row lists only `parent relates_to`), so the name-based check reports `unexpected linkage key 'source'` — directly contradicting the plan's "no source rename / no collision" assertion. **Gating: blocks Phase 1 on a template the plan intends to leave untouched.** Fix by subtracting the row's `extras`/base/provenance keys before intersecting with the vocabulary.

- 🟡 **Portability**: `declare -A` introduces a bash 4.0+ dependency that breaks default macOS bash 3.2
  **Location**: Phase 1 §2(c) `LINKAGE_CARDINALITY` associative array
  The existing script uses only indexed arrays and runs on bash 3.2; `declare -A` errors with `invalid option` under `set -euo pipefail`, aborting before any assertion. CI is ubuntu-only (bash 5), so this is a works-in-CI/breaks-locally trap for macOS contributors. Use a `case "$lkey" in parent|...) card=single ;; ...) card=list ;; esac` (3.2-safe) or pin a modern bash in `mise.toml` + a version guard.

- 🟡 **Test Coverage / Code Quality**: Guidance keyword check is too loose to verify per-field fill/omit
  **Location**: Phase 2 §2(c) `in_populate_section_with_guidance`
  `has_guidance` is set if *any* line in the section matches `/fill|omit/`, decoupled from the field, and matches substrings ("backfill", "fulfil"). A section naming ten fields with one unrelated "fill in the title" sentence passes for all ten. AC #3's per-field guarantee is not actually verified per field. Anchor to whole words and tie the keyword to the field's own bullet/line.

- 🟡 **Test Coverage**: Phase 2/3 ship assertion code never proven to fail on bad input
  **Location**: Phase 2 Overview & Success Criteria ("no-op until Phases 4–6")
  The `omit_when_empty` column is empty on every row through Phase 3, so the new helper and loop execute zero iterations; the first real exercise is deferred to content phases where green could equally mean "works" or "inert". Add a fixture/unit-test proving the helper returns FAIL for a field lacking guidance before content depends on it.

- 🟡 **Test Coverage**: Phase 6 equivalence regression test is left to chance
  **Location**: Phase 6 §5 ("add `parent_typed_form_resolves_same_as_bare_id` if not present")
  Existing cluster_key.rs tests cover typed and bare forms *separately* but none asserts both shapes of the same id resolve identically. The conditional "verify… or add" leaves the regression-protecting test optional even though Phase 6 changes producers on the explicit claim of equivalence. Make it an unconditional deliverable with its own success criterion.

- 🟡 **Architecture**: Omit-when-empty contract has no automated enforcement on real artifacts
  **Location**: Implementation Approach §Emission model / Emission classification
  The core deliverable (artifacts omit empty optional keys) is enforced only by a prose keyword check; no test inspects a generated artifact, and the visualiser reads empty/absent identically, so a producer emitting `external_id: ""` violates the convention with zero test signal. Record this as a conscious, accepted tradeoff (and flag it to 0070) or add a fixture-based producer-output test.

- 🟡 **Documentation**: ADR-0040 spec is under-specified vs house ADR conventions
  **Location**: Phase 0 §1 (content spec + filename)
  Two gaps: (a) the content list omits house-mandated **Decision Drivers** and **Considered Options** sections (and the Positive/Negative/Neutral Consequences split) used by sibling supplement ADRs 0033/0035/0037 — an implementer following it verbatim risks a review-adr bounce; (b) the filename `ADR-0040-omit-when-empty-frontmatter-emission.md` omits the established `-supplement-to-adr-0033` convention (0035/0037 set this for supplements). Also, the Phase 0 automated-verification claim that `mise run test:unit:templates` validates the authored ADR's frontmatter is incorrect — that test asserts template files, not corpus artifacts.

- 🟡 **Standards**: Reviewer heading-lift offers two incompatible heading forms
  **Location**: Phase 4 §2(a)
  Step (a) permits `**Populate frontmatter**:` OR `### Populate frontmatter`, but the Phase 4 verification grep only matches the bold form (`^\*\*Populate frontmatter\*\*`), while the awk detector keys on `/populate frontmatter/` in any `^#` heading. Picking the H3 form (which matches the Group A `### Step N:` canon) would fail the grep. Choose one form for all four reviewers and align both checks.

- 🟡 **Standards**: bare-id→typed-form normalisation is applied unevenly across producers
  **Location**: Overview/Desired End State vs Phase 3 vs Phase 6
  Phase 6 names only `refine-work-item` and `create-plan` as normalised, but Phase 3's bullet silently gives `create-work-item`/`extract-work-items` the typed form without flagging a value-shape change. The research recommended treating all four bare-id producers as one consistent change. State the create-work-item/extract-work-items move explicitly and fold them into the normalisation narrative.

- 🟡 **Usability**: Omit-when-empty creates a silent absence indistinguishable from oversight
  **Location**: Desired End State / Migration Notes
  A reader of generated artifacts cannot tell whether a missing `relates_to` is a deliberate "no link" or an authoring/producer bug — the documenting comment lives only in the template, never in the emitted file. Ensure ADR-0040's Consequences states the reader-facing rule ("an absent optional key means no value, never an error") explicitly.

- 🟡 **Usability**: The "canonical Populate-frontmatter snippet shape" does not exist uniformly
  **Location**: Phases 3–6 ("follows the canonical snippet shape")
  The research records four shape groups (A/B/C/D); a maintainer learning the pattern from one Group A skill cannot guess the Group B reviewer form. Define one literal canonical bullet template in a single place (ADR-0040 or a shared snippet doc) that all phases reference verbatim.

#### Minor

- 🔵 **Correctness**: `superseded_by` handling is dead code — it is a single-ref slot in the grammar/post-check, yet no template carries it (work item §2 line 69); only the *list* slot `blocked_by` actually needs the trailing sentence. Signals the cardinality conflation behind the critical regex bug.
- 🔵 **Correctness**: Empty trailing TSV column depends on the trailing tab surviving editors/formatters; if stripped, `NF` self-check fails (fail-safe, not silent). Consider a `-` sentinel like the existing `forbidden_own_id_key` convention.
- 🔵 **Standards**: source-type token spelling may disagree (`adr:ADR-NNNN` vs `adr:NNNN`); SKILL-bullet-vs-template token agreement is unenforced by any test. Pin one id-placeholder convention per source-type.
- 🔵 **Standards**: Phase 6 should convert refine-work-item's *entire* decompose substitution list (base + linkage) to the `←` arrow form, not just the new linkage bullets, to avoid two styles in one op.
- 🔵 **Standards**: The ungated non-linkage comment rewrites (`external_id`, `reviewer`, etc.) are not exhaustively enumerated; the `key:`/`empty` → `ref:`/`""` drift can persist on un-checked fields. Add a Phase 1 manual-verification checklist of every comment site.
- 🔵 **Architecture / Code Quality**: `in_populate_section_with_guidance` near-duplicates `in_imperative_section` with a *narrower* heading regex, creating two parallel "population section" definitions that can drift. Factor into one parameterised helper.
- 🔵 **Code Quality**: The inverse-sentence and closed-set checks hand-roll PASS/FAIL bookkeeping instead of using `assert_contains`/`assert_in_block`; the single/list `case` arms duplicate the assertion call; the space-fenced `grep -qF " $vkey "` membership idiom is non-idiomatic for these scripts.
- 🔵 **Documentation**: Absolute line-number references (`lines 436-459`, `cluster_key.rs:138-152`) will rot — several point into scripts that earlier phases edit. Pair each with a content anchor (heading/function/token).
- 🔵 **Documentation**: Whether the illustrative template comment strings (`pr_url: ""  # omitted until populated…`) are normative or examples is unstated, and lifecycle-annotation migration for three of four reviewers has no per-skill acceptance line — an annotation could be silently dropped.
- 🔵 **Portability**: The awk `[ \t]` class may not expand under BSD awk (macOS); use `[[:space:]]`. Also `tolower()`/`[A-Za-z]` are locale-sensitive and no `LC_ALL=C` is set — consistent with the project's existing `LANG=C` discipline, export it in the scripts.
- 🔵 **Portability**: The `test` CI job runs only on ubuntu-latest, so none of the shell-portability regressions above would surface in CI. Note as a known limitation or add a macOS leg for the `test-*.sh` scripts.

#### Suggestions

- 🔵 **Test Coverage**: "Watch the test fail first" is asserted but unverifiable (tests + content land in one merge). Land the assertion in a commit preceding the content, or paste failing-run output into phase notes.
- 🔵 **Test Coverage**: Run the visualiser suite (`mise run test`) at every phase boundary, not just Phases 1 and 6, since the content sweeps change frontmatter the visualiser consumes.
- 🔵 **Architecture**: The closed vocabulary projects edges (`derived_from`, `relates_to`, `blocks`, etc.) no consumer reads today; note explicitly that these slots are write-ahead-of-consumer and that referential-integrity validation is deferred to the future graph epic.
- 🔵 **Code Quality**: Extend the unknown-linkage-key FAIL message to point at the fix ("add it to `LINKAGE_CARDINALITY`/`LINKAGE_VOCABULARY` or correct the TSV row").
- 🔵 **Usability**: Lead the linkage-bullet group with one "omit-by-default" sentence so the common case (a fresh draft with no explicit edges) is stated once rather than inferred from seven negative clauses; make linkage-slot template comments carry the same omission cue as the other optionals.
- 🔵 **Standards**: ADR id 0040 is the next free sequential number (0039 was claimed by an unrelated ADR on a later rebase) — keep the "verify still free at authoring time" guard.

### Strengths

- ✅ The seventh `typed_linkage_keys` TSV column is the right closed-set mechanism — table-driven, single home per key, no overloading of `extras`.
- ✅ The always-emitted vs omit-when-empty boundary is explicitly tabulated and anchored to an existing learned pattern (`status: ""` → bare `status: ready`), giving authors a concrete mental hook.
- ✅ Phase 0 gates the rest on an accepted ADR; templates-before-SKILL.md ordering enables clean partial revert — good dependency hygiene.
- ✅ Consumer-impact analysis is rigorous and grounded in the visualiser's actual `typed_ref.rs`/`cluster_key.rs` behaviour (empty and absent read identically; both ref shapes tolerated), so the additive change carries no runtime risk.
- ✅ The plan restates the closed slot table and SOURCE_TYPE set in full for self-containment, extracts the comment grammar into a labelled normative block, and is explicit about superseding the work item's `present-but-empty`/`leave empty` text.
- ✅ Filename discrepancies flagged by research (`issue-research.md`→`rca.md`, `plan-validation.md`→`validation.md`) are correctly resolved throughout.
- ✅ The cardinality map and closed-set space-padding membership logic are individually correct (verified by the correctness pass), and the POSIX-ERE escaping survives bash double-quote interpolation as written.

### Recommended Changes

1. **Fix the inverse-key list regex / template contradiction** (addresses: critical regex finding; Documentation, Standards, Usability variants). Decide one form and make spec + example + regex consistent: either (a) relax the list anchor for inverse keys to `or[[:space:]]+\[\]([[:space:]]+.*)?$` and keep the `grep -qF` post-check for the exact sentence, or (b) move the trailing sentence to its own comment line above the slot and keep `\[\]$`. Option (b) also resolves the Usability readability finding. Update the work-item.md/plan.md `blocked_by` example to match. Drop or mark-reserved the `superseded_by` single-ref-with-trailing-sentence branch (no template carries it).

2. **Fix the closed-set check's `source:` collision** (addresses: Architecture major). Subtract the row's `extras` (and base/provenance) keys before intersecting the frontmatter with `LINKAGE_VOCABULARY`, so `design-inventory`'s foreign-source `source:` is exempt. Add a one-line note that a vocabulary name appearing in `extras` is intentionally exempt.

3. **Add negative fixtures and gate on PASS counts** (addresses: critical assertions-never-fail finding; loose-guidance and no-op findings). Feed each new assertion a known-bad input (spurious key, wrong cardinality, missing inverse sentence, field with no fill/omit note, guidance in a different section) and assert the script reports FAIL. Replace "no FAIL lines" criteria with positive expected-PASS-count assertions. In Phase 2, prove `in_populate_section_with_guidance` returns FAIL before content depends on it, and tie the `fill`/`omit` keyword to each field's own bullet with whole-word matching.

4. **Make the script bash-3.2-safe** (addresses: Portability major). Replace `declare -A LINKAGE_CARDINALITY` with a `case` statement (or pin a modern bash in `mise.toml` and add a `BASH_VERSINFO` guard). Switch awk `[ \t]` to `[[:space:]]` and export `LC_ALL=C` in the test scripts. Note the ubuntu-only CI matrix as a known limitation.

5. **Complete the ADR-0040 spec** (addresses: Documentation major). Name the full house section set (Context, Decision Drivers, Considered Options, Decision, Consequences split Positive/Negative/Neutral, References), adopt the `-supplement-to-adr-0033` filename suffix (or justify dropping it), state the reader-facing "absent = no value" rule in Consequences, and correct the Phase 0 automated-verification claim to file-existence + manual review-adr acceptance.

6. **Resolve ref-form consistency end to end** (addresses: Standards/Usability/Architecture cross-cutting). State explicitly that `create-work-item` and `extract-work-items` also move to typed-form `parent`, fold all four bare-id producers into one normalisation narrative, pin one id-placeholder convention per source-type, make the Phase 6 `parent_typed_form_resolves_same_as_bare_id` equivalence test unconditional, and record in ADR-0040/template comments that typed-linkage form is canonical for new writes while bare-id is tolerated legacy.

7. **Pick one reviewer heading form** (addresses: Standards major). Use `### Populate frontmatter` for all four reviewers to match the Group A canon, and align both the Phase 4 grep and the awk detector to it; remove the "or" from Phase 4 §2(a).

8. **Reduce duplication and record the enforcement gap** (addresses: Code Quality / Architecture). Factor the two awk section-walkers into one parameterised helper; route the new checks through existing `assert_*` helpers; and record the omit-when-empty "no artifact-level enforcement" boundary as a conscious tradeoff, flagged to 0070.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound: the seventh TSV column is a clean closed-set mechanism, the authoring-surface/emission split is well-drawn, and the phase decomposition is coherent with a thorough consumer-compatibility analysis. The main concerns are the closed-set collision with design-inventory's `source:`, the lack of any automated backstop for the omit-when-empty contract on real artifacts, and a multi-document divergence around ADR-0040's scope table.

**Strengths**:
- Seventh TSV column is the right structural choice for per-template closed sets without overloading `extras`.
- Always-emitted vs omit-when-empty boundary explicitly tabulated; template/artifact split mirrors the existing `status` resolution.
- Phase 0 ADR gating + templates-before-SKILL.md ordering show good dependency direction.
- Consumer-impact analysis grounded in the visualiser parsers treating empty/absent identically.

**Findings**:
- 🟡 major (high) — *Closed-set check collides with design-inventory's non-linkage `source:` field* (Phase 1 §2(d)). The name-based walk over `LINKAGE_VOCABULARY` greps `^source:` on design-inventory (a foreign-source extra) and FAILs because the row lists only `parent relates_to` — contradicting the plan's no-collision claim and blocking Phase 1. Subtract extras/base before intersecting.
- 🟡 major (medium) — *Omit-when-empty contract has no automated enforcement boundary* (Emission model). Enforced only by a prose keyword check; no test inspects a generated artifact, so producers can violate it with zero signal. Record as accepted tradeoff or add a fixture test.
- 🔵 minor (high) — *New section-matching helper diverges from existing heading semantics* (Phase 2 §2(c)). Two parallel "population section" definitions can drift. Factor into one parameterised helper.
- 🔵 minor (medium) — *ADR-0040 supersedes work-item requirements, creating multi-source-of-truth divergence* (Phase 0). The boundary spans four documents with no sync mechanism. Make ADR-0040's scope table the single normative source.
- 🔵 minor (medium) — *Phase 6 bundles two unrelated concerns and conditionally edits the out-of-scope visualiser* (Phase 6). Split the allowlist lift from the bare-id normalisation; pre-verify the equivalence test.
- 🔵 suggestion (low) — *Closed vocabulary anticipates edges no consumer reads* (Phase 6 / Key Discoveries). Note explicitly that these slots are write-ahead-of-consumer by design.

### Code Quality

**Summary**: The bulk of the proposed bash reuses existing harness idioms well. Concerns concentrate in the two new snippets: the Phase 1 typed-linkage block reinvents PASS/FAIL bookkeeping and uses a non-idiomatic membership idiom, and the Phase 2 awk helper near-duplicates `in_imperative_section` while relying on a loose `fill|omit` substring match.

**Strengths**:
- Reuses `while IFS=$'\t' read`, the `NF != N` self-check, `assert_in_block`, and top-of-script array declarations.
- Cardinality modelled as a single declarative associative array + parallel vocabulary list.
- Phase 2 deliberately lands as a no-op ahead of content sweeps — independently reviewable.
- Well-targeted inline comments on the new arrays.

**Findings**:
- 🟡 major (medium) — *`in_populate_section_with_guidance` near-clones `in_imperative_section`* with a narrower heading regex; two structurally-identical awk programs to maintain in lockstep. Factor into one parameterised helper.
- 🔵 minor (high) — *Inverse-key and closed-set checks hand-roll PASS/FAIL* instead of using `assert_contains`/`assert_in_block`.
- 🔵 minor (high) — *single/list `case` arms duplicate the assertion call*; set only `regex` per arm then call once.
- 🔵 minor (medium) — *closed-set space-fenced `grep -qF " $vkey "` idiom* is non-idiomatic; use exact-token iteration or a `contains_word` helper.
- 🔵 minor (medium) — *regex escaping inconsistent between arms* (single vs doubled backslashes); normalise and cross-reference the normative grammar block.
- 🔵 minor (medium) — *guidance `/fill|omit/` is a bare section-wide substring match*; anchor to whole words and the field's own line.
- 🔵 suggestion (medium) — *unknown-linkage-key FAIL message* should point at the fix (add to the cardinality/vocabulary arrays).

### Correctness

**Summary**: Logic is largely correct — the closed-set space-padding, ERE escaping through bash double-quotes, the empty-column zero-iteration loop, and the awk same-section latch all check out. But there is a provable blocking error: the list regex `\[\]$` anchor can never match the `blocked_by` line that carries the trailing sentence.

**Strengths**:
- Closed-set `grep -qF " $vkey "` correctly space-pads; `blocks` does not false-match inside `blocked_by`.
- ERE escaping survives interpolation (`\"\"`→`""`, `\\[\\]`→`\[\]`).
- `for fld in $omit_when_empty` correctly iterates zero times when empty.
- awk helper resets per-heading via `flush()`, never resets `found`, flushes at END.
- `adr.md` `supersedes` matches the list regex (`ADR-NNNN` satisfies `[A-Za-z0-9-]+`).

**Findings**:
- 🔴 critical (high) — *List regex anchored `\[\]$` can never match the `blocked_by` line with the trailing sentence* (Phase 1 §2(d) vs work-item.md example). Relax the anchor for inverse keys or move the sentence to its own line.
- 🔵 minor (high) — *`superseded_by` handling is dead code* — single-ref in the grammar yet no template carries it; only the *list* slot `blocked_by` needs the trailing sentence.
- 🔵 minor (medium) — *Empty trailing TSV column depends on the trailing tab surviving*; if stripped the NF self-check fails (fail-safe). Consider a `-` sentinel.

### Test Coverage

**Summary**: For a plan whose deliverable IS assertions, assertion quality is everything. The template-shape assertions are well-targeted, but several are never exercised by a negative case, the inverse-key main regex contradicts the shipped template line, and the Phase 2/3 "no-op" design merges assertion code never proven to FAIL. "No FAIL lines" is not a sound green signal.

**Strengths**:
- Closed-set check is a genuine negative-coverage mechanism for spurious slots.
- Cardinality keyed by an in-script map — good maintainability.
- Correctly gates Phase 6 on `test:unit:visualiser` and identifies the relevant existing tests.
- Per-phase TDD framing is the right discipline.

**Findings**:
- 🔴 critical (high) — *Inverse-key main regex contradicts the shipped template line; the assertion can never pass.*
- 🔴 critical (high) — *Closed-set and comment-grammar assertions are never exercised by a negative fixture* — mutation test yields no failure.
- 🟡 major (high) — *"Absence of FAIL lines" is not a sound green signal*; a never-run assertion looks identical to a passing one. Gate on PASS counts.
- 🟡 major (high) — *Phase 2/3 ship assertion code never proven to fail on bad input* (no-op until Phase 4).
- 🟡 major (medium) — *Guidance keyword check too loose* to catch a note lacking fill/omit semantics.
- 🟡 major (medium) — *Phase 6 equivalence regression test left to chance* ("if not present"). Make it unconditional.
- 🔵 minor (medium) — *Closed-set edge cases* (cross-section guidance, neither-keyword note, malformed comment) not enumerated.
- 🔵 minor (high) — *TDD "watch it fail" is asserted but unverifiable* (tests + content in one merge).
- 🔵 minor (medium) — *Visualiser gate only at Phases 1 and 6*, not where producer changes land.

### Documentation

**Summary**: Unusually thorough as a document — restates the slot table, extracts the comment grammar, and is explicit about superseding the work item text. Material gaps are in the Phase 0 ADR-0040 spec (missing house-mandated sections and the supplement filename convention) and an internal inconsistency in the normative grammar an implementer would hit head-on.

**Strengths**:
- Explicitly documents superseding work item §1/§3 and AC #1/#3.
- Restates the closed slot table and SOURCE_TYPE set for self-containment.
- Filename discrepancies correctly resolved (rca.md/validation.md).
- Comment grammar extracted into a labelled block (satisfies Pass 3 polish item a).
- Per-phase Automated + Manual verification checklists.

**Findings**:
- 🟡 major (high) — *ADR-0040 spec omits house-mandated sections* (Decision Drivers, Considered Options, Consequences split). Risks a review-adr bounce.
- 🟡 major (high) — *ADR-0040 filename omits the `-supplement-to-adr-NNNN` convention* set by 0035/0037.
- 🟡 major (high) — *Normative grammar internally inconsistent* — list `\[\]$` anchor vs the same-line trailing sentence in the example.
- 🔵 minor (medium) — *Work-item AC #3 keyword reconciliation* (`leave empty` vs `omit`) spread across plan/work-item/test. State one authoritative pair.
- 🔵 minor (medium) — *Phase 0 automated-verification claim incorrect* — `test:unit:templates` validates template files, not the authored ADR.
- 🔵 minor (high) — *Absolute line-number references will rot*; pair with content anchors.
- 🔵 minor (medium) — *Illustrative template comments unstated as normative vs example*; no acceptance criterion for their content.
- 🔵 suggestion (medium) — *Lifecycle-annotation migration for three of four reviewers* described only generically; add per-skill verification lines.

### Standards

**Summary**: Largely faithful to project conventions (next-free ADR id, seventh-column mandate, canonical writer snippet shape). The significant risks are internal-consistency hazards that manifest as test failures or convention drift: source-type token agreement, the reviewer heading-lift offering two forms, and uneven bare-id→typed-form normalisation.

**Strengths**:
- Correctly cites ADR-0029 and the next free id is genuinely 0040.
- Honours the mandatory seventh-column TSV form.
- Preserves the canonical 0065 snippet shape for the eight writer skills.
- Keeps design-gap/design-inventory carve-outs verbatim and out of the closed set.
- ADR status lifecycle (proposed→accepted) handled correctly.

**Findings**:
- 🔴 major (high) — *Reviewer heading lift offers two forms; Phase 4 grep only matches the bold form.* Pick `### Populate frontmatter` and align both checks.
- 🟡 major (medium) — *bare-id→typed-form normalisation uneven*; create-work-item/extract-work-items silently get typed form in Phase 3, not flagged in Phase 6.
- 🟡 major (medium) — *source-type token spelling may disagree* (`adr:ADR-NNNN` vs `adr:NNNN`); SKILL-vs-template agreement unenforced.
- 🔵 minor (high) — *refine-work-item em-dash vs arrow*: convert the whole decompose substitution list, not just the new bullets.
- 🔵 minor (medium) — *work-item.md `blocked_by` line vs anchored regex* (mirrors the critical finding).
- 🔵 minor (medium) — *ungated non-linkage comment rewrites not exhaustively enumerated*; drift can persist.
- 🔵 suggestion (high) — *ADR filename pattern otherwise conforms*; keep the "verify 0040 free" guard.

### Usability

**Summary**: The authoring-surface-vs-emitted-artifact split is explicitly named and justified — a real strength. DX concerns: the dual mental model is learnable but easy to get backwards; the inverse-key comment line is overlong and hard to read; the silent absence of omitted keys is undistinguishable from oversight for downstream readers; and the "canonical snippet shape" the guidance depends on does not exist uniformly.

**Strengths**:
- Authoring-surface vs emitted-artifact distinction anchored to a learned pattern.
- Fill/omit guidance enforced mechanically so it cannot silently rot.
- Convention is forgiving — empty and absent both read as "no value".
- Closed-set assertion keeps the surface authors must learn bounded.

**Findings**:
- 🟡 major (high) — *Inverse-key comment line is overlong and mixes two grammars on one line* — the most behaviourally-important instruction is the least readable.
- 🟡 major (medium) — *Omit-when-empty silent absence indistinguishable from oversight* for downstream readers. Document the "absent = no value" rule in ADR-0040.
- 🟡 major (medium) — *"Canonical snippet shape" does not exist uniformly* — cross-skill consistency fails. Define one literal template referenced verbatim.
- 🔵 minor (high) — *Template comments say two different things about empty optionals* (linkage `or ""` vs `omitted when…`), inviting keep-or-drop confusion.
- 🔵 minor (medium) — *"Omit the key entirely" repeated per-bullet*; lead with one omit-by-default sentence.
- 🔵 minor (medium) — *Two ref-form idioms coexist* during/after rollout; record which is canonical.

### Portability

**Summary**: The plan extends scripts that currently run on bash 3.2. Phase 1 introduces `declare -A` (bash 4.0+), which default macOS bash lacks; since CI is ubuntu-only, this is a works-in-CI/breaks-locally regression the plan does not acknowledge. The awk helper and regex character classes carry milder BSD-awk/locale risks.

**Strengths**:
- New regexes are POSIX ERE with `[[:space:]]` and `grep -E` — within the portable subset.
- Reuses already-portable harness idioms (`grep -qF --`, `assert_in_block`).
- No new external binaries or services.

**Findings**:
- 🟡 major (high) — *`declare -A` introduces a bash 4.0+ dependency that breaks default macOS bash 3.2.* Use a `case` statement or pin bash + add a version guard.
- 🔵 minor (medium) — *awk `[ \t]` class behaves differently under BSD awk*; use `[[:space:]]`.
- 🔵 minor (medium) — *`tolower()` / `[A-Za-z]` are locale-sensitive*; export `LC_ALL=C` per the project's existing `LANG=C` discipline.
- 🔵 minor (high) — *Test job runs only on ubuntu-latest*, so shell-portability regressions are invisible to CI. Note as a limitation or add a macOS leg.

## Re-Review (Pass 2) — 2026-06-03T23:01:03+00:00

**Verdict:** REVISE

The revision resolved **both pass-1 criticals** and the large majority of the majors/minors — the plan is substantially healthier. However, the edits introduced one genuine regression (an awk control-flow bug, flagged independently by two lenses) plus a cluster of new majors centred on the self-test wiring, a heading-convention tension, and an omit-cue asymmetry. With 0 criticals but ≥3 standing majors, the verdict remains REVISE — though all remaining items are quick, well-scoped fixes rather than structural rework.

### Previously Identified Issues

**Correctness** (all 3 resolved)
- 🔴→✅ List regex `\[\]$` vs `blocked_by` trailing sentence — **Resolved** (standalone guidance comment line; anchor preserved; whole-block `grep -qF`).
- 🔵→✅ `superseded_by` dead code — **Resolved** (dropped from active grammar; retained only as a closed-set guard).
- 🔵→✅ Trailing-empty TSV column fragility — **Resolved** (`-` sentinel + loop guard; verified NF==4).

**Test Coverage**
- 🔴→✅ Inverse-key regex contradiction — **Resolved**.
- 🔴→✅ Assertions never exercised by a negative fixture — **Resolved** (§2e negative-fixture + Phase 2 liveness self-tests added).
- 🟡→🟡 "Absence of FAIL" not a sound signal — **Partially resolved** (now gates on PASS count, but the count is still "≈36", not a concrete integer — see new issues).
- 🟡→✅ Phase 2/3 inert assertion code — **Resolved** (liveness self-test).
- 🟡→✅ Guidance check too loose — **Resolved** (whole-word, bullet-bound).
- 🟡→✅ Phase 6 equivalence test left to chance — **Resolved** (unconditional deliverable + criterion).
- 🔵→✅ TDD watch-fail unverifiable / visualiser gate — **Resolved** (Migration Notes).

**Documentation**
- 🟡→✅ ADR-0040 missing house sections — **Resolved**.
- 🟡→✅ ADR-0040 filename convention — **Resolved** (in the plan; but the work item still lags — see new issues).
- 🟡→✅ Normative-grammar inconsistency — **Resolved**.
- 🔵→🟡 AC #3 keyword reconciliation — **Partially resolved** (AC keyword fixed; work-item §1 inverse-comment requirement still contradicts the new standalone-line approach).
- 🔵→✅ Phase 0 automated-verification claim — **Resolved**.
- 🔵→🔵 Line-number references rot — **Still present** (not swept; consciously deferred).
- 🔵→✅ Illustrative-vs-normative comments — **Resolved**.
- 🔵→✅ Lifecycle-annotation per-reviewer verification — **Resolved**.

**Standards**
- 🔴→✅ Reviewer heading two-forms — **Resolved** (pinned `### Populate frontmatter` + aligned grep).
- 🟡→✅ Uneven bare-id→typed normalisation — **Resolved**.
- 🟡→🟡 Source-type token spelling — **Partially resolved** (pinned; one stray `adr:NNNN` at the Key Discoveries no-op note remains).
- 🔵→✅ refine-work-item em-dash vs arrow — **Resolved**.
- 🔵→✅ `blocked_by` line vs anchored regex — **Resolved**.
- 🔵→✅ Ungated comment rewrites enumeration — **Resolved**.

**Architecture**
- 🟡→✅ Closed-set `source:` collision — **Resolved** (extras exemption).
- 🟡→✅ No omit-when-empty enforcement — **Resolved** (acknowledged tradeoff flagged to 0070).
- 🔵→🟡 Section-helper divergence — **Partially resolved** (shares heading predicate; flush semantics still differ — see new issues).
- 🔵→🟡 ADR-0040 multi-source-of-truth — **Partially resolved** (relationship named; authority still split across 3 docs until edits land).
- 🔵→✅ Phase 6 bundling / conditional visualiser edit — **Resolved**.
- 🔵→✅ Write-ahead-of-consumer vocabulary — **Resolved**.

**Usability**
- 🟡→✅ Inverse-key overlong line — **Resolved**.
- 🟡→✅ Silent absence — **Resolved** (ADR reader-facing rule).
- 🟡→🟡 No uniform canonical snippet shape — **Partially resolved** (Group A pinned; B/D reshaped, not unified).
- 🔵→✅ Keep-or-drop comment confusion — **Resolved** (for non-linkage optionals; but see new asymmetry issue).
- 🔵→🔵 Omit-by-default lead sentence — **Still present** (per-bullet "omit" repetition unchanged).
- 🔵→✅ Ref-form canonical recorded — **Resolved**.

**Portability**
- 🟡→✅ `declare -A` bash-4 dependency — **Resolved** (`case` function + bash-3.2 criterion).
- 🔵→✅ awk `[ \t]` class — **Resolved** (`[[:space:]]`).
- 🔵→🟡 Locale `LC_ALL=C` — **Partially resolved** (added to template script; skill-population script still lacks it).
- 🔵→🔵 CI ubuntu-only — **Still present** (macOS leg not added; consciously deferred).

### New Issues Introduced

- 🟡 **Correctness + Code Quality (high/medium, 2 lenses)**: **Bullet-window awk discards a valid match when the field's bullet is the last before a `#` heading.** `in_populate_section_with_guidance` commits `found=1` only at the next bullet or `END`; the `/^#/` branch resets `tracking`/`saw` *without committing first*. A correctly-authored skill whose fill/omit bullet is the section's last line before another heading would spuriously FAIL. **This is a regression introduced in pass 1's edits.** Fix: commit the pending window in the heading branch too (factor a `flush()` as `in_imperative_section` does).
- 🟡 **Standards (medium)**: **The broadened awk heading predicate (`…|step [0-9]`) does not actually enforce the literal `Populate frontmatter` heading AC #3 requires** — reviewer prose already sits under `### Step 4:`, which the predicate matches, so the literal heading is enforced only by the one-off Phase 4 grep, not the contract test. The two mechanisms encode two different contracts for one requirement.
- 🟡 **Standards (high)**: **Bare `### Populate frontmatter` diverges from the universal `### Step N: …` heading style** every other producer skill uses; the plan's awk comment conflates "matches the detector" with "matches the convention." Consider `### Step N: Populate frontmatter` (the predicate matches it equally) or document the deliberate departure.
- 🟡 **Test Coverage (high)**: **Self-tests offer a "same script OR sibling wired into `tasks/test/unit.py`" choice with no Changes Required step wiring the sibling in** — a sibling self-test could ship un-run by CI, silently re-opening the criticals it closes. Pick the in-script form or add the unit.py wiring step.
- 🟡 **Test Coverage (high)**: **The "positive PASS count" gate still says "≈36"** — an approximation can't be asserted. The count is exactly derivable (36 shape + 12 closed-set + 2 inverse-line); pin it as a hard equality (`grep -c`).
- 🟡 **Test Coverage / Code Quality (medium)**: **§2e presupposes refactoring the §2d assertions into pure return-valued functions, but §2d is still written as inline counter-mutating blocks** — the self-test is un-implementable as described without that refactor, which the plan sketches rather than specifies.
- 🟡 **Usability (high)**: **The omit-cue asymmetry makes linkage slots read as "keep when empty."** Non-linkage optionals now say `# … omitted when not linked`, while adjacent linkage slots end at `or ""` with no omit hint — `or ""` naturally reads as "empty is a valid value to keep," the opposite of ADR-0040. Add a single non-slot header comment above the linkage block (doesn't break the gated regex).
- 🟡 **Documentation (high)**: **The work item still records the un-suffixed ADR-0040 filename** (`…-emission.md`, no `-supplement-to-adr-0033`), contradicting the plan it is updated alongside. One-line fix in the work item's Dependencies.
- 🔵 **Code Quality (medium)**: `in_populate_section_with_guidance` and `in_imperative_section` still have divergent flush semantics; the plan comment over-promises "shares the same skeleton." Two parallel vocab sources (`linkage_cardinality()` vs `LINKAGE_VOCABULARY`) risk drift with no enforcing check.
- 🔵 **Architecture / Correctness (low, latent)**: the name-based extras exemption can't distinguish a genuine typed `source:` edge from design-inventory's foreign-source extra (single-template, latent — `source:` is scoped out of design-inventory).
- 🔵 **Documentation (high)**: Phase 0 ADR spec omits the house dual-title (`# ADR-0040: … — supplement to ADR-0033`), the in-body Date/Status/Author block (ADR-0030 mandate), and the recursive-supplement clause (ADR-0035/0037 pattern).
- 🔵 **Standards**: stray `adr:NNNN` at the Key Discoveries no-op note (line ~264) contradicts the pinned `adr:ADR-NNNN`.
- 🔵 **Portability**: `LC_ALL=C` added to the template script but not `test-skill-frontmatter-population.sh`, which also uses locale-sensitive `tolower()`/`[[:alpha:]]`.
- 🔵 **Test Coverage**: liveness fixtures don't include a bold-lead-in (`**Populate frontmatter**:`, no `#`) case to prove the `^#` heading requirement is enforced.
- ✅ **Portability (confirmation)**: the em-dash in `INVERSE_GUIDANCE_LINE` under `grep -qF` + `LC_ALL=C` is byte-match-safe — *not* a risk (verified by the portability lens); only an encoding round-trip could break it.

### Assessment

The plan is now in good structural shape: every pass-1 critical is resolved, the contract-test logic is internally consistent, and the portability/ADR/standards gaps are closed. The verdict stays REVISE because pass-1's edits introduced a real awk control-flow bug (spurious FAILs once the producer sweeps run) and a handful of new majors — but all are quick, localised fixes, not redesigns. Priority order for a pass-3 cycle: (1) the awk heading-boundary flush bug; (2) reconcile the literal-heading requirement with the broadened predicate (and the heading-style divergence); (3) pin the self-test wiring + concrete PASS count + the §2d→pure-function refactor; (4) the omit-cue header comment; (5) the work-item filename one-liner and the remaining doc/standards/portability minors.

---
*Re-review generated by /review-plan*

## Re-Review (Pass 3) — 2026-06-04T12:38:29+00:00

**Verdict:** REVISE

The pass-2 fixes all hold: the awk flush regression is gone, the self-tests are in-script with exact count gates, the ADR house-style is complete, the work-item filename matches, `LC_ALL=C` parity is in, the source-type token is consistent, and the omit-cue asymmetry + omit-by-default + canonical-snippet-uniformity items are resolved. No pass-2 item regressed. The verdict stays REVISE only because the pass-2 edits introduced one genuine self-contradiction (flagged by two lenses) plus a cluster of small "make-the-implicit-explicit" gaps. All are quick; none is structural.

### Previously Identified Issues (pass-2 items)

- ✅ **Correctness**: awk bullet-window flush bug — **Resolved** (flush() on heading/bullet/END; traced clean across all three cases, monotone `found` latch, no double-count).
- ✅ **Test Coverage**: self-test sibling-wiring, "≈36" approximate count, self-test count gates, §2e refactor under-spec, bold-lead-in fixture, Phase 2 CI wiring — **all Resolved** (in-script mandated; exact 36/12/2/6/5 gates with `grep -c`; function signatures given; arithmetic verified = 36).
- ✅ **Documentation**: work-item ADR-0040 filename — **Resolved**; ADR dual-title/in-body-status-block/recursive-supplement clause — **Resolved** (match ADR-0035/0037 verbatim).
- ✅ **Standards**: literal-heading vs broad-predicate reconciliation — **Resolved** (split + test-gated); heading-style divergence wording — **Resolved**; stray `adr:NNNN` — **Resolved**.
- ✅ **Usability**: omit-cue asymmetry — **Resolved** (block-header comment); omit-by-default lead — **Resolved**; canonical snippet uniformity — **Resolved** (`←` across all groups).
- ✅ **Portability**: `LC_ALL=C` parity — **Resolved** (both scripts; `LC_ALL=C` is the stronger pin vs the repo's `LANG=C` — an improvement); all new constructs (em-dash header, `-v headingre`, literal-heading ERE, function refactor) confirmed portable.
- ✅ **Architecture**: extras-exemption boundary, inverse-key authority consolidation — **Resolved/documented**.
- 🟡 **Code Quality / Architecture / Correctness**: helper near-clone — **Partially resolved** (shares `flush()` + `POPULATE_HEADING_RE` via `-v`, but `in_imperative_section` keeps its inline copy; migration left as a parenthetical, so two predicate copies still ship).
- 🔵 **Documentation**: work-item §1 still records the obsolete inverse-comment form (trailing-sentence + `superseded_by`) — **Partially resolved** (plan supersedes-note covers it; work item itself stale in isolation).
- 🔵 **Code Quality**: space-fenced `grep -qF` idiom, inter-arm regex escaping inconsistency — **Still present** (consciously minor).
- 🔵 **Documentation**: absolute line-number references — **Still present** (consciously deferred).

### New Issues Introduced

- 🟡 **Code Quality + Usability (high, two lenses)**: **Lead-sentence/bullet self-contradiction.** Phase 3 says the trailing "otherwise omit" clause *can be dropped* from bullets once the omit-by-default lead states it, but every example bullet still carries the clause. Worse, the skill test requires a whole-word `fill`/`omit` keyword *per bullet window*, so a bullet that drops "omit" without a "fill" would FAIL. The genuine must-fix of this pass — pick one form and make examples match (keep one short cue word per bullet).
- 🟡 **Code Quality (high) / Architecture (minor) / Correctness (suggestion)**: **`in_imperative_section` migration is only a parenthetical.** The shared `POPULATE_HEADING_RE` is consumed only by the new helper; the sibling keeps its inline literal, so two copies of the heading vocabulary still ship and can drift. Make the one-line migration a concrete Phase 2 step.
- 🟡 **Test Coverage (high)**: **The new literal-heading reviewer assertion is not count-gated** (no `grep -c … -eq 4`, absent from Testing Strategy), so it could go inert — the exact failure mode the other exact-count gates were added to prevent. Add the gate + list it.
- 🟡 **Standards (high)**: **Standalone full-line frontmatter comments are a new convention** no existing template uses (all current frontmatter comments are inline/trailing). Acknowledge it as a deliberate convention extension and confirm the metadata-helpers parser tolerates standalone `#` lines inside the `---` block.
- 🟡 **Architecture (medium)**: **Contract-test-layer proportionality.** The meta-test machinery (shared regex var, pure-function refactor, two self-test harnesses, five exact-count magic numbers, vocab-drift guard, literal-heading assertion) is now a sizable subsystem for a static surface; the hard-coded counts couple the harness to the current template inventory. Suggest deriving counts from the TSV (also fixes the magic-number coupling) or recording the meta-test cost as an accepted tradeoff.
- 🔵 **Correctness (minor)**: §2d still shows counter-mutating sample code inconsistent with §2e's pure-function refactor; and the `blocked_by` inverse-line check has no defined home in the refactored functions, so its self-test fixture could be inert. Rewrite §2d to the final pure-function+wrapper shape and fold the inverse check into `check_linkage_slot` (or a third function).
- 🔵 **Correctness (minor)**: awk window can drop a satisfied match if a continuation line re-mentions the field key without a fill/omit word (resets `saw`). Arm the field match once per window.
- 🔵 **Test Coverage (minor)**: the vocab-drift structural guard and the per-field skill-test PASS totals (Phases 3-6) aren't count-gated; add `-eq 9` / a derived total.
- 🔵 **Standards (minor) / Correctness (suggestion)**: pin the literal-heading assertion to key on reviewer *producer rows* (TSV-driven), not a hardcoded path list.
- 🔵 **Documentation (minor)**: verify ADR-0030 actually mandates the in-body Date/Status/Author block before citing it as a mandate (soften to "house convention, cf. ADR-0030" if unverified).

### Assessment

The plan is in strong shape — three passes have eliminated every critical and every pass-2 major, with no regressions. The remaining work is genuine final polish: one real contradiction (the lead-sentence/bullet clash, which can also trip the contract test), a few "make the parenthetical/implicit explicit" items (helper migration, literal-heading count gate, §2d pure-function rewrite), one convention acknowledgement (standalone comments), and an architecture proportionality note. None requires redesign. With these applied the plan should reach APPROVE. Priority: (1) lead-sentence/bullet contradiction; (2) `in_imperative_section` migration step + §2d pure-function rewrite incl. inverse-check home; (3) literal-heading + vocab-drift count gates and TSV-keyed row selection; (4) acknowledge the standalone-comment convention (+ metadata-helper tolerance); (5) proportionality note / derive counts; (6) remaining minors.

### Pass-3 Resolution — verdict APPROVE

All pass-3 items above were applied to the plan:

- ✅ Lead-sentence/bullet contradiction — removed the "can be dropped" claim; each bullet keeps a `fill`/`omit` cue (required by the per-bullet test gate).
- ✅ §2d rewritten to the single authoritative pure-function form (`check_linkage_slot`/`check_closed_set` return rc; live loop owns PASS/FAIL); the `blocked_by` inverse-guidance check folds into `check_linkage_slot` so the self-test exercises it.
- ✅ `in_imperative_section` migration to `$POPULATE_HEADING_RE` is now a concrete required Phase 2 step (f) — one source for the heading vocabulary.
- ✅ Literal-heading assertion count-gated (`-eq 4`), TSV-keyed on reviewer producer rows; vocab-drift guard count-gated (`-eq 9`); shape+comment and omit-when-empty totals **derived from the TSV** (no magic numbers), which also resolves the proportionality concern.
- ✅ awk continuation-line fragility fixed (`!tracking` guard arms each window once).
- ✅ Standalone in-`---` comment acknowledged as a deliberate convention extension, with a Phase 1 parser-tolerance verification; ADR-0030 citation softened; work-item §1 carries a supersede pointer.

No critical, structural, or major items remain open; the residual deferrals (absolute line-number references; adding a macOS CI leg) are consciously accepted and documented in the plan. **Verdict: APPROVE — ready for implementation.**

---
*Re-review generated by /review-plan*

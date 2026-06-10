---
type: work-item-review
id: "0106-invoke-plugin-scripts-by-bare-path-review-1"
title: "Work Item Review: Invoke Plugin Scripts by Bare Path in Skill Bodies"
date: "2026-06-11T12:32:29+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0106"
work_item_id: "0106"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: [permissions, allowed-tools, skills, plugin, authoring-convention]
last_updated: "2026-06-11T12:53:48+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Invoke Plugin Scripts by Bare Path in Skill Bodies

**Verdict:** REVISE

This is a well-grounded, tightly-scoped task that faithfully implements "Option A"
from its source research, with internally consistent Summary/Context/Requirements
and acceptance criteria that have been smartly reframed from a hard-to-reproduce
runtime symptom into static, inspectable conditions. The blocking weakness is that
the work item deliberately broadens scope to "every `artifact-*`/`config-*` call
site across all affected skills" but never enumerates that set nor defines the
procedure that produces it — leaving the two "every site" acceptance criteria
without a definite denominator to verify against. A secondary gap is that the
required directive's minimal conforming content is left under-specified, so
"carries the directive" has no objective pass test.

### Cross-Cutting Themes

- **Unenumerated "affected skills" set undermines verifiability and bounding**
  (flagged by: testability, completeness, scope, clarity) — Four lenses
  independently converged on the same root issue: the work item widens scope past
  "the ~15 named in the source research" to "the full set" but never lists that set
  or pins the grep that derives it. Testability frames it as an unverifiable "every
  site" criterion (major); completeness as a missing baseline the implementer must
  re-derive; scope as an open-ended consistency sweep enlarging the delivery
  surface; clarity as an external count the reader cannot resolve in place. This is
  the single most actionable fix and resolving it addresses all four.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: 'Every call site' is unverifiable without an enumerated inventory of affected sites
  **Location**: Acceptance Criteria
  The first and third criteria depend on the set of call sites being knowable, but
  the work item provides no authoritative enumeration — the Assumptions section
  broadens scope beyond "the ~15 named" to "the full set" yet never lists it. A
  verifier cannot conclusively confirm "every" site is covered, so the criterion can
  be claimed met while sites are silently missed.

- 🟡 **Testability**: The required directive content is under-specified, so 'carries the directive' has no objective pass test
  **Location**: Acceptance Criteria
  The first criterion asks a verifier to confirm each site "carries the bare-path
  directive", but neither it nor the Requirements define what minimally constitutes a
  conforming directive (exact phrasing, which wrappers must be named, whether the
  rationale must be present). Two reviewers could disagree on whether a site that
  names only `bash` but not `sh`/`env`, or paraphrases without naming wrappers,
  passes.

#### Minor

- 🔵 **Clarity**: Forbidden-wrapper set is stated inconsistently (`bash`/`sh` vs `bash`/`sh`/`env`)
  **Location**: Summary
  The Summary says "never prefix with `bash`/`sh`" (two wrappers) while Requirements
  and both Acceptance Criteria say "`bash`/`sh`/`env`" (three). The source research's
  Option A also names only `bash`/`sh`. A reader cannot tell whether `env` is in
  scope for the directive text.

- 🔵 **Completeness**: Affected-file scope is defined functionally but never enumerated
  **Location**: Requirements / Assumptions
  Scope is defined as "every `artifact-*`/`config-*` call site across all affected
  skills" but the set is never listed; named examples in Technical Notes are flagged
  illustrative, not exhaustive. The implementer must independently grep and derive
  the list with no captured baseline to confirm completeness against. (Reinforces the
  cross-cutting theme.)

- 🔵 **Dependency**: Option B layering/ordering relationship is not captured
  **Location**: Dependencies
  Option B broadens the same `allowed-tools` surface as a drift backstop and the
  source research says to "layer Option B on top for robustness," but Dependencies
  records it only as a non-directional "Relates to". Without an ordering note a
  planner could schedule B independently and lose the intended belt-and-suspenders
  relationship.

- 🔵 **Dependency**: Prevention follow-up (lint/test guardrail) is not tracked as a relation
  **Location**: Dependencies
  The source research's Prevention section recommends a lint/test that cross-checks
  every invocation against `allowed-tools` rules — automating exactly the manual grep
  this item's criteria rely on. That downstream guardrail is not named in
  Dependencies, so the convention this item establishes has no captured enforcement
  item and may silently erode in later edits.

- 🔵 **Clarity**: 'Option A'/'Option B' referents are only defined in the source research
  **Location**: Context
  The work item refers to "Option A only" / "Fix Option B" across four sections but
  never states inside the item what they are beyond a parenthetical for B. A reader
  who has not opened the research must guess what "Option A only" scopes in and out.

- 🔵 **Clarity**: "the ~15 named in the source research" relies on an external count the reader cannot resolve in-place
  **Location**: Assumptions
  The "Affected skills" assumption and Drafting Notes contrast the full set against a
  "~15"/"named 15" list that exists only in the research's Affected Components
  section, so the in/out boundary is unverifiable from the work item alone.
  (Reinforces the cross-cutting theme.)

- 🔵 **Testability**: Third criterion partially overlaps the second for non-ADR sites
  **Location**: Acceptance Criteria
  Because Technical Notes state the other call sites are already inline-code, the
  third criterion is expected to pass trivially for everything except the two ADR
  sites the second criterion already covers, so it may not exercise a distinct
  condition. It would be stronger framed explicitly as a regression guard whose
  expected outcome is "zero matches across the whole set."

#### Suggestions

- 🔵 **Scope**: Whole-corpus directive sweep is broader and less bounded than the named-amplifier fix
  **Location**: Requirements
  The first requirement widens from the named high-risk ADR fence sites to every
  call site across an unenumerated set, while the inline-code sites are
  research-classified as lower-risk. Bundling the high-confidence two-site fix with
  an open-ended consistency sweep enlarges the delivery surface; consider splitting
  the sweep into a thin follow-on, or retain the grep-enumeration criterion so the
  boundary stays concrete and closeable.

- 🔵 **Clarity**: "the authorized path becomes a mere argument" compresses the mechanism into an assumed-context clause
  **Location**: Summary
  The Summary parenthetical packs the root-cause mechanism ("stripped wrapper", path
  "becomes a mere argument") into one clause whose terms are only defined later in
  Context. Either gloss "stripped wrapper" on first use or defer the mechanism to
  Context.

### Strengths

- ✅ Summary, Context, Requirements, and Acceptance Criteria describe one coherent
  intent (Fix Option A only) with no drift between sections; Option B is explicitly
  and repeatedly carved out across Requirements, Acceptance Criteria, Dependencies,
  and Drafting Notes.
- ✅ Acceptance Criteria are smartly reframed from the original hard-to-reproduce
  runtime "no prompt" symptom into static, inspectable conditions — the right
  testability move for an editing task — and the Drafting Notes deliberately
  acknowledge this reframing.
- ✅ All expected sections for a `task` kind are present and substantively populated
  (Summary, Context, Requirements, Acceptance Criteria, Dependencies, Assumptions,
  Open Questions, Technical Notes, Drafting Notes, References); frontmatter is valid
  with a recognised kind, status, priority, and a source link.
- ✅ The two highest-risk sites are pinned with exact file:line references
  (`create-adr` SKILL.md:124-126, `extract-adrs` SKILL.md:120-122), giving a verifier
  unambiguous targets, and the negative "no `allowed-tools` modified" criterion is
  concretely diff-verifiable.
- ✅ The sole upstream prerequisite (scripts' shebang + execute bit) is captured in
  both Assumptions and Requirements, and Blocked-by/Blocks are explicitly "none",
  appropriate for a self-contained editing task.

### Recommended Changes

1. **Define the canonical set of affected call sites as a verification procedure**
   (addresses: "Every call site is unverifiable…", "Affected-file scope is defined
   functionally but never enumerated", "Whole-corpus directive sweep…", "the ~15
   named…")
   Add an explicit first step / acceptance criterion that names the grep producing
   the authoritative set — e.g. the set produced by
   `grep -rl 'scripts/\(artifact\|config\)-' skills/**/SKILL.md` — so "every site"
   has a definite denominator. Either inline the resulting list or state that
   deriving it via grep is the first task step, so the inventory is intentionally
   derived rather than silently omitted.

2. **Specify the minimal conforming directive as a checkable predicate**
   (addresses: "The required directive content is under-specified…", "Forbidden-wrapper
   set is stated inconsistently")
   State once, verbatim, the wrapper list to forbid (e.g. `bash`/`sh`/`env`) and use
   it consistently in Summary, Requirements, and Acceptance Criteria. Define what
   minimally constitutes a conforming directive (e.g. "an imperative instruction
   naming all three wrappers as prohibited and the bare path as required"), or adopt
   the source-research blockquote as the normative template the check matches against.

3. **Capture the Option B layering and the prevention guardrail as directional relations**
   (addresses: "Option B layering/ordering relationship is not captured", "Prevention
   follow-up (lint/test guardrail) is not tracked")
   In the Dependencies "Relates to" entry for Option B, note the intended sequencing
   ("reinforcing follow-up applied on top of A; not required for A to ship,
   recommended after"). Add a "Relates to" entry for a current-or-future lint/test
   guardrail work item that enforces invocation-shape-vs-rule lockstep.

4. **Frame the third acceptance criterion explicitly as a regression guard**
   (addresses: "Third criterion partially overlaps the second…")
   State that its expected result is "zero matches across the whole set", making the
   empty result the verifiable outcome and distinguishing it from the second
   criterion rather than reading as redundant.

5. **(Optional) Gloss Option A/B and the root-cause mechanism for self-containment**
   (addresses: "'Option A'/'Option B' referents…", "the authorized path becomes a
   mere argument…")
   On first use, gloss Option A and Option B in one clause each, and either define
   "stripped wrapper" on first use in the Summary or defer that mechanism detail to
   Context where it is already explained.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually clear: it names a single, well-scoped change
(add a 'run by bare path, never prefix with bash/sh/env' directive at every
artifact-*/config-* call site, plus convert two bare code fences to inline code), and
its Summary, Context, Requirements, and Acceptance Criteria are internally consistent
and traceable to the source research. The main clarity risks are minor terminological
wobble around the exact wrapper set the directive must forbid (bash/sh in the Summary
vs bash/sh/env in the Requirements) and a couple of referents ('Option A', 'Option B',
'the ~15 named') that depend on the reader having the source research open.

**Strengths**:
- The Summary, Context, and Requirements describe a single coherent intent (Fix Option
  A only), with Option B explicitly and repeatedly carved out, so there is no scope
  ambiguity between sections.
- Acceptance Criteria use Given/When/Then phrasing with concrete, named referents
  (create-adr/SKILL.md, extract-adrs/SKILL.md, grep for artifact-*/config-*), leaving
  little room for divergent interpretation.
- The actor is consistently 'the model' and the mechanism (allowed-tools prefix match,
  bash not being a stripped wrapper) is explained once in Context and not contradicted
  elsewhere.

**Findings**:
- 🔵 minor (confidence: high) — **Forbidden-wrapper set is stated inconsistently
  (bash/sh vs bash/sh/env)** — _Location: Summary_. The set of wrappers the directive
  must forbid is stated differently across sections: the Summary says "never prefix
  with `bash`/`sh`" (two wrappers), while Requirements (first bullet) and both
  Acceptance Criteria say "`bash`/`sh`/`env`" (three). The source research's Option A
  also names only `bash`/`sh`. Impact: an implementer could omit or include `env`
  inconsistently across call sites. Suggestion: state the exact wrapper list once and
  use it verbatim everywhere.
- 🔵 minor (confidence: medium) — **'Option A'/'Option B' referents are only defined in
  the source research, not the work item** — _Location: Context_. The item refers to
  "Fix Option B", "Option B", "Option A only" across four sections but never states
  what they are beyond a parenthetical for B. Impact: a reader who has not opened the
  research must guess what "Option A only" scopes in/out. Suggestion: gloss Option A
  and B in one clause each on first use.
- 🔵 minor (confidence: medium) — **'the ~15 named in the source research' relies on an
  external count the reader cannot resolve in-place** — _Location: Assumptions_. The
  "Affected skills" assumption and Drafting Notes contrast the full set against a list
  that exists only in the research. Impact: the in/out boundary is unverifiable from
  the work item. Suggestion: restate scope positively and self-containedly, or note
  where the list lives.
- 🔵 minor (confidence: low) — **'the authorized path becomes a mere argument'
  compresses the mechanism into a clause that assumes prior context** — _Location:
  Summary_. The parenthetical packs the root-cause mechanism into one dependent clause
  whose terms ("stripped wrapper") are only explained later. Suggestion: gloss
  "stripped wrapper" on first use, or defer the detail to Context.

### Completeness

**Summary**: This task-kind work item is structurally complete and densely populated:
Summary, Context, Requirements, Acceptance Criteria, Dependencies, Assumptions, Open
Questions, Technical Notes, and References are all present and substantively filled.
The work to be done is clearly defined for a chore/task — what to edit, where, and
what to leave untouched — and frontmatter is intact with a recognised kind and status.
The only notable gap is that the affected scope is described as 'every
artifact-*/config-* call site across all affected skills' without enumerating those
skills, which leaves the implementer needing to derive the file set rather than having
it stated.

**Strengths**:
- All expected sections for a task are present and substantively populated.
- Frontmatter is complete and valid: kind is task, status (draft) and priority
  (medium) present, source reference links to the research document.
- The Context section explains the motivating problem rather than restating the
  Summary, and explicitly scopes out Option B.
- Requirements clearly state the work to be done with concrete file/line references
  for the highest-risk sites.

**Findings**:
- 🔵 minor (confidence: medium) — _Location: Requirements / Assumptions_. The scope of
  affected files is defined functionally as 'every artifact-*/config-* call site across
  all affected skills' but never enumerated; named examples in Technical Notes are
  flagged illustrative, not exhaustive. Impact: an implementer must independently grep
  and derive the list with no captured baseline to confirm completeness. Suggestion:
  inline the enumerated list (or the grep that produces it) into Requirements, or state
  explicitly that producing the enumeration via grep is the first step.

### Dependency

**Summary**: The work item's couplings are largely well-captured for a self-contained
authoring-convention task. The single upstream dependency (existing shebang + execute
bit) is explicitly captured in both Assumptions and Requirements, and the one named
downstream relation (Fix Option B) appears correctly in the Dependencies section as a
relation. The main gap is an implied ordering/coupling between this item and Option B
around the shared allowed-tools rules that the source research treats as
belt-and-suspenders layers, plus an unnamed prevention follow-up (the lint/test
guardrail) that the source research recommends to prevent regression.

**Strengths**:
- The sole upstream prerequisite (shebang + execute bit) is captured in both
  Assumptions and Requirements.
- The one downstream-reinforcing relation (Option B) is named in Dependencies as
  "Relates to" and cross-referenced in Context, correctly distinguished from a blocker.
- Blocked-by and Blocks are both explicitly "none", and Open Questions confirms no
  blocking unknowns — appropriate for a self-contained editing task.

**Findings**:
- 🔵 minor (confidence: medium) — _Location: Dependencies_. The source research treats
  Option A and B as layered fixes on the same allowed-tools surface ("layer Option B on
  top for robustness"); Option B is captured only as a non-directional "Relates to" with
  no sequencing. Impact: a planner could schedule B before/independently of A and lose
  the belt-and-suspenders relationship. Suggestion: note the intended layering in the
  "Relates to" entry.
- 🔵 minor (confidence: medium) — _Location: Dependencies_. The research's Prevention
  section recommends a lint/test cross-checking invocations against allowed-tools rules
  — automating the manual grep this item's criteria rely on — but that guardrail item is
  not named in Dependencies. Impact: the convention has no captured downstream
  enforcement and may silently erode. Suggestion: add a "Relates to" entry referencing a
  current/future lint/test guardrail item.

### Scope

**Summary**: This task describes one coherent, atomic unit of work: closing the
model-discretion gap that lets the bash wrapper defeat the allowed-tools permission
match for artifact-*/config-* scripts. Its two sub-activities (adding the bare-path
directive at every call site, and converting two bare code fences to inline code) both
serve the single purpose identified as Option A in the source research, and the
reinforcing Option B is cleanly excluded and tracked separately. The declared task kind
fits the mechanical, single-owner editing scope, and the boundaries are explicit and
verifiable.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria describe the same scope with no drift.
- The two distinct edits are both root-cause remedies for the same wrapping prior, so
  they form one coherent unit rather than bundled independent concerns.
- Scope boundary with Option B is explicit across Requirements, Acceptance Criteria,
  Dependencies, and Drafting Notes.
- The task kind is appropriate for mechanical, single-author editing with no
  cross-service or cross-team coordination.

**Findings**:
- 🔵 suggestion (confidence: medium) — **Whole-corpus directive sweep is broader and
  less bounded than the named-amplifier fix** — _Location: Requirements_. The first
  requirement widens from the ~15 named skills to every call site across the full skill
  set, while the source research's highest-value edit is the two named bare-fence sites;
  the rest are inline-code, classified lower-risk. Impact: bundling a high-confidence
  two-site fix with an open-ended consistency sweep across an unenumerated set enlarges
  the delivery surface and makes the unit harder to bound and verify. Suggestion:
  consider splitting the sweep into a thin follow-on, or retain the grep-enumeration
  criterion so the boundary stays concrete and closeable.

### Testability (Pass 1)

**Summary**: This task item is unusually well-suited to verification: its Acceptance
Criteria are reframed as static, inspectable conditions rather than the original runtime
'no prompt' symptom, which the Drafting Notes explicitly acknowledge. The main
testability gap is the absence of a defined, enumerable list of 'affected skills'/'call
sites', which makes the first and third criteria's coverage unbounded — a verifier
cannot conclusively confirm 'every' site is covered without an authoritative inventory.
The directive's required wording is also under-specified, leaving room for inconsistent
interpretation when checking each site.

**Strengths**:
- Acceptance Criteria are framed as inspectable static conditions rather than the
  original runtime 'no prompt' reproduction — the right testability move — and the
  Drafting Notes deliberately acknowledge this.
- The two highest-risk sites are pinned with exact file:line references, giving a
  verifier concrete, unambiguous targets.
- The third criterion defines a near-mechanical grep pass/fail check.
- The negative criterion 'No allowed-tools frontmatter is modified' is concretely
  verifiable via diff and cleanly scopes Option B out.

**Findings**:
- 🟡 major (confidence: high) — **'Every call site' is unverifiable without an enumerated
  inventory of affected sites** — _Location: Acceptance Criteria_. The first and third
  criteria depend on the set of call sites being knowable, but the item provides no
  authoritative enumeration; Assumptions broadens scope beyond "the ~15 named" to "the
  full set" yet that set is never listed and the research's list ends in an ellipsis.
  Impact: completeness can only be approximated, so the criterion can be claimed met
  while sites are silently missed. Suggestion: define the procedure that establishes the
  canonical set (e.g. a specific grep), or include the enumerated inventory.
- 🟡 major (confidence: medium) — **The required directive content is under-specified, so
  'carries the directive' has no objective pass test** — _Location: Acceptance Criteria_.
  Neither the criterion nor Requirements define what minimally constitutes a conforming
  directive (exact phrasing, which wrappers named, whether rationale required); the
  research offers an example blockquote but the item does not adopt it as normative.
  Impact: two reviewers could disagree on whether a given site passes. Suggestion:
  specify the minimal conforming directive as a checkable predicate, or reference the
  source-research blockquote as the canonical template.
- 🔵 minor (confidence: medium) — **Third criterion partially overlaps the second but
  adds no new independently failing condition for non-ADR sites** — _Location: Acceptance
  Criteria_. Because Technical Notes state other sites are already inline-code, the third
  criterion is expected to pass trivially for everything except the two ADR sites already
  covered by criterion two. Impact: a verifier may treat it as auto-satisfied.
  Suggestion: explicitly frame it as a regression guard whose expected result is "zero
  matches across the whole set", or merge it into the second.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-11

**Verdict:** REVISE

Both original major findings are resolved, and all eight original minor/suggestion
findings are resolved or substantially addressed. However, the edits introduced three
new major findings — a misattributed canonical directive template, an undefined
regression-guard procedure in AC3, and undefined "call site" granularity — so the
verdict remains REVISE. All three are tightly scoped and quickly fixable; the work
item is materially stronger than at Pass 1 and close to APPROVE.

### Previously Identified Issues

- 🟡 **Testability**: 'Every call site' unverifiable without an enumerated inventory — **Resolved.** AC1 now anchors the denominator to the canonical grep `grep -rl 'scripts/\(artifact\|config\)-' skills/**/SKILL.md`.
- 🟡 **Testability**: Required directive content under-specified — **Resolved.** Requirements now define a "conforming directive" with explicit (a)/(b) sub-conditions.
- 🔵 **Clarity**: Forbidden-wrapper set stated inconsistently — **Resolved** in the body (all sections now say `bash`/`sh`/`env`), but see new finding on the quoted template's attribution.
- 🔵 **Completeness**: Affected-file scope defined functionally but never enumerated — **Resolved.** The canonical grep is now the first task step.
- 🔵 **Dependency**: Option B layering/ordering not captured — **Resolved.** Dependencies now states "B applied on top of A, not a substitute".
- 🔵 **Dependency**: Prevention follow-up (lint/test guardrail) not tracked — **Resolved.** A "Relates to" entry now names the guardrail.
- 🔵 **Clarity**: 'Option A'/'Option B' referents defined only in the research — **Partially resolved.** Both are now glossed inline in Context; clarity still flags Option A's first use as a label before its gloss (suggestion).
- 🔵 **Clarity**: "~15 named" external count — **Resolved.** Assumptions now references the canonical grep instead of the external count.
- 🔵 **Testability**: Third criterion overlapped the second — **Resolved.** AC3 reframed as a regression guard with expected result "empty" (but see new finding on its undefined procedure).

### New Issues Introduced

- 🔴 **Clarity** (major): **Misattributed canonical template.** Requirements quotes a directive blockquote as "the source research's Recommended Fix blockquote", but the quoted text adds `env` and rewords the original, which prohibits only `bash`/`sh`. The cited exemplar therefore disagrees with the real source and with AC1's "all three wrappers" check. (Also flagged by completeness (suggestion) and testability (minor).)
- 🟡 **Testability** (major): **AC3 regression-guard procedure undefined.** AC3 asserts "the regression-guard check returns zero matches presenting a script path inside a bare unlabeled code fence" but supplies no command/pattern; detecting an unlabeled fence containing a script path is a multi-line condition a line-oriented `grep` cannot express, so two verifiers could implement different checks. (Also flagged by clarity (minor).)
- 🟡 **Testability** (major): **'Call site' granularity undefined.** The denominator grep `-rl` returns one path per file, but AC1 requires a directive at "every call site" and a single SKILL.md may invoke several scripts in different passages; whether one directive per file or per occurrence satisfies the criterion is unstated.
- 🔵 **Dependency** (minor): Reliance on the Claude Code matcher's stripped-wrapper set and prefix-match semantics is not captured as an external behavioural coupling.
- 🔵 **Dependency** (minor): The source research's unresolved "first-call-only enforcement" question conditions the strength of the Option B coupling; not noted against the Option B relation.
- 🔵 **Clarity** (minor): Singular "the script" framing in Summary/Assumptions could under-scope the change to the one named metadata script.
- 🔵 **Completeness** (suggestion): The `SKILL.md:124-126` / `:120-122` line numbers are not anchored to a plugin version/revision and may drift.

### Assessment

The work item is close to ready. The three new majors are all artifacts of the Pass 1
edits and are narrowly fixable: (1) relabel the quoted template as "adapted from the
source research's Recommended Fix to add `env`" (or stop attributing it), (2) specify
the concrete regression-guard command (e.g. a multiline `rg` pattern matching an
unlabeled ``` fence whose body contains `scripts/artifact-`/`scripts/config-`), and
(3) define the "call site" unit (per-file vs per-occurrence, switching the denominator
to `grep -rn` if per-occurrence). Addressing those three would clear the path to
APPROVE; the remaining minors are polish.

---
*Re-review generated by /accelerator:review-work-item*

## Re-Review (Pass 3) — 2026-06-11

**Verdict:** COMMENT

All three Pass-2 majors are resolved. This pass surfaced three further majors —
one genuinely introduced by the Pass-2 dependency edit (an Option B status
contradiction) and two real defects in the acceptance commands (a depth-fragile
`skills/**/SKILL.md` glob, and AC2 checking only fence-absence with no presence
check). All three have now been fixed. The work item is converging: each pass's new
findings are progressively narrower and concern the verification harness for what is
a low-risk mechanical documentation edit. Verdict downgraded to COMMENT — the item is
acceptable to implement; remaining items are polish and need not gate the work.

### Previously Identified Issues (Pass 2 → Pass 3)

- 🔴 **Clarity**: Misattributed canonical template — **Resolved.** Now explicitly "adapted from the source research's Recommended Fix to add `env`"; clarity flagged the reconciliation as a strength this pass.
- 🟡 **Testability**: AC3 regression-guard procedure undefined — **Resolved.** AC3 now carries a concrete multiline `rg -U` command, an "unlabeled fence" definition, and (newly) a known-positive validation step.
- 🟡 **Testability**: 'Call site' granularity undefined — **Resolved.** Defined as per-occurrence; denominator switched to `grep -rn`, and a same-passage locality rule now binds each directive to its occurrence.
- 🔵 **Dependency** (matcher coupling), 🔵 **Dependency** (enforcement-quirk conditional), 🔵 **Clarity** ("the script" singular), 🔵 **Completeness** (line-number anchoring) — **Resolved** in the Pass-2 edits.

### New Issues Introduced — now fixed

- 🟡 **Clarity** (major): **Option B status contradiction** — Dependencies stated B was both "recommended afterwards, not required" and "a firmer dependency". **Fixed:** Dependencies now states a single position (B is non-blocking; item can ship and close on its own) and moves the conditional escalation to Open Questions.
- 🟡 **Testability** (major): **AC1 denominator glob may under-count** — `skills/**/SKILL.md` depends on shell globstar and fixed depth. **Fixed:** switched to `grep -rn 'scripts/\(artifact\|config\)-' --include=SKILL.md skills/` (depth-independent) in Requirements and AC1.
- 🟡 **Testability** (major): **AC2 had no positive check** — only fence-absence was verified, so a deleted path could pass. **Fixed:** AC2 now requires both a backtick-delimited inline presence and fence absence.

### Remaining (polish — not addressed, do not gate implementation)

- 🔵 **Testability**: AC3 known-positive is now described in prose; an implementer still executes it manually (acceptable for a manual-verification task).
- 🔵 **Dependency / Completeness**: Option B and the lint/test guardrail are referenced without work-item IDs because those items do not yet exist; Open Questions now records that they should be created.
- 🔵 **Clarity**: Option A/B/C labels remain partly reliant on the source research's Fix Options table despite inline glosses in Context.

### Assessment

The work item is ready to implement. Its acceptance criteria are now concrete,
reproducible, and depth-robust, and the one self-contradiction has been resolved. The
residual findings are documentation polish and the creation of two clearly-described
sibling work items, none of which block this item's mechanical editing work. Further
review passes would yield diminishing returns relative to the low risk of the change.

**Reviewer decision (2026-06-11):** Toby Clemson **APPROVED** the work item. The
lens-suggested verdict was COMMENT (polish-only residual findings); the reviewer
accepts those as non-blocking and approves the item for implementation. Frontmatter
`verdict` set to APPROVE accordingly.

---
*Re-review generated by /accelerator:review-work-item*

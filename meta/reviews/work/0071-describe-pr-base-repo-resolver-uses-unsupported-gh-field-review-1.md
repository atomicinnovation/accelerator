---
date: "2026-05-18T20:13:13+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0071-describe-pr-base-repo-resolver-uses-unsupported-gh-field.md"
work_item_id: "0071"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
---

## Work Item Review: describe-pr cannot post PR body — pr-base-repo.sh uses unsupported gh JSON field

**Verdict:** REVISE

The work item is a thorough, precisely written bug report: it names the failing component, the unsupported `gh --json` field, the chain of callers, and exact reproduction commands; acceptance criteria are outcome-shaped Given/When/Then rules; scoping decisions are documented in Drafting Notes. Two structural concerns drive the REVISE verdict — the Dependencies section understates couplings that are clearly implied elsewhere in the document (the fix lives in a separate plugin repository, three sibling skills are unblocked by it, and a `gh` version range is the load-bearing external assumption), and the referenced source bug-report file (cited from Context, Open Questions, and References for the environment table and the three candidate fix paths) is not present in this checkout, so any deferred detail cannot be retrieved by a reviewer or implementer.

### Cross-Cutting Themes

- **Missing source bug-report** (flagged by: clarity, completeness) — The work item repeatedly defers detail (full reproduction transcript, environment table, three candidate fix paths) to `.accelerator/tmp/2026-05-18-describe-pr-update-body-bug.md`, which does not exist in the repository. This weakens both the self-contained completeness of the report and the clarity of the Open Questions discussion of fix paths.
- **Outcome-shaped ACs vs. bounded preconditions** (flagged by: clarity, testability) — Acceptance criteria avoid prescribing a solution (a strength), but the preconditions they use — "any reasonably current `gh` release", "a real PR", "test, doc, or CI matrix entry" — leave a verifier room to declare the criterion met under weaker conditions than intended.

### Findings

#### Major
- 🟡 **Clarity + Completeness**: Referenced source bug-report is not present in the checkout
  **Location**: Context (2nd bullet), Open Questions (1st bullet), References
  The work item points to `.accelerator/tmp/2026-05-18-describe-pr-update-body-bug.md` for the full reproduction transcript, environment table, and the three candidate fix paths summarised only briefly in Open Questions. That file does not exist in this checkout, so any reviewer or implementer who follows the citation cannot retrieve the deferred context.

- 🟡 **Dependency**: Cross-repository coupling to accelerator plugin not captured as a blocker
  **Location**: Dependencies
  Dependencies says "Blocked by: nothing locally", but the fix lives in `atomic-innovation-prerelease/accelerator` (a separate plugin repo on its own pre-release cadence). Open Questions and Assumptions both surface this venue question; Dependencies should record it as an explicit external blocker so the item is not treated as locally actionable.

- 🟡 **Dependency**: Downstream consumer skills named in Context not listed as Blocks
  **Location**: Dependencies
  Context names three consumer skills (`describe-pr`, `review-pr`, `respond-to-pr`) that share the broken resolver, and AC #4 requires all three to be exercised end-to-end after the fix. Dependencies records "Blocks: nothing currently in flight", leaving the downstream coupling invisible at the planning layer.

#### Minor
- 🔵 **Clarity**: Fifth acceptance criterion has a malformed Given/When/Then structure
  **Location**: Acceptance Criteria (5th bullet)
  AC #5 reads "Given the upstream defect, the fix is accompanied by a regression check…" — the Given simply re-states that the bug exists and there is no When clause, breaking the rhythm of the four preceding criteria.

- 🔵 **Clarity**: "That fallback path" has two plausible referents
  **Location**: Open Questions (3rd bullet)
  "Fixing the resolver makes that fallback path unnecessary in practice" — "that fallback path" could refer to the `gh pr edit --body-file` operator workaround or to the deprecation-failure code path inside `gh` itself.

- 🔵 **Clarity**: "Reasonably current gh release" is left undefined
  **Location**: Context (1st bullet) / Acceptance Criteria
  Expected behaviour invokes "reasonably current `gh` release" without pinning a floor; Assumptions raises the question but does not resolve it. The implementer cannot pick between fix options without choosing a definition themselves.

- 🔵 **Dependency**: External system (gh CLI version range) not captured in Dependencies
  **Location**: Dependencies
  The defect is fundamentally a coupling to the `gh` `--json` allowlist as it stood at 2.65.0. Neither the tool nor a version range appears in Dependencies, so the SLA-style assumption lives only in Assumptions prose.

- 🔵 **Dependency**: Ordering between fix and regression-check AC not explicit
  **Location**: Acceptance Criteria
  AC #5 binds a regression check to the fix, but does not require they ship together; the CI/`gh`-availability coupling that the smoke check introduces is also not captured.

- 🔵 **Testability**: Regression-check criterion accepts disjunctive artefact types without a pass/fail threshold
  **Location**: Acceptance Criteria (5th bullet)
  "Test, doc, or CI matrix entry that would catch a future re-introduction" — three different artefact types satisfy it, and "would catch" has no defined threshold, so reviewers can disagree about whether the criterion is met.

- 🔵 **Testability**: "Any reasonably current gh release" and "a real PR" are unbounded
  **Location**: Acceptance Criteria (4th bullet) and Expected behaviour
  Two verifiers could rationally test against different `gh` versions and PR shapes and reach different pass/fail conclusions.

- 🔵 **Testability**: Error-classification criterion lists preconditions without defining how each is induced
  **Location**: Acceptance Criteria (3rd bullet)
  AC #3 enumerates four failure modes (auth, network, malformed JSON, deleted PR) and requires the message to identify "which precondition is missing" — without specifying how each is induced or what substring the message must contain.

#### Suggestions
- 🔵 **Clarity**: "Almost certainly mocks gh" is speculative inside Technical Notes
  **Location**: Technical Notes (3rd bullet)
  A hedged guess sits inside a section readers treat as authoritative; either verify the claim or reframe as an explicit assumption.

- 🔵 **Scope**: Regression-check AC may be a separable chore
  **Location**: Acceptance Criteria
  Adding real-`gh` smoke coverage to a helper that currently mocks `gh` is testing-infrastructure work whose scope could grow (CI provisioning, gating); consider narrowing AC #5 or splitting into a follow-up.

- 🔵 **Scope**: Upstream-vs-local fix venue is a scope-affecting open question
  **Location**: Open Questions
  Until the venue question (plugin upstream vs local override) resolves, the unit of work is ambiguous. Resolve before moving out of draft, or split into two items.

### Strengths
- ✅ Summary precisely names the failing skill, the failing helper, the unsupported field, and the affected `gh` version — no guesswork required.
- ✅ Reproduction steps, Expected behaviour, and Actual behaviour use concrete commands and exact error strings.
- ✅ Acceptance Criteria use a consistent Given/When/Then structure (apart from AC #5) with named actors (operator, resolver) rather than passive voice.
- ✅ Acceptance criteria are deliberately outcome-shaped rather than solution-shaped (documented in Drafting Notes), so verification does not depend on which of the three candidate fixes is chosen.
- ✅ Drafting Notes explicitly explain scope, severity, and identifier-scrubbing decisions, pre-empting reader questions about intent.
- ✅ Frontmatter is complete and valid (`type=bug`, `status=draft`, `priority=medium`, identifying fields all present).
- ✅ Cross-fork safety is captured as a distinct, independently verifiable criterion with a concrete setup and a concrete check.
- ✅ Error-path verification is included (AC #3) — genuine resolution failures must not be falsely attributed to "Unknown JSON field".

### Recommended Changes

1. **Either commit the source bug-report into the repository or inline the parts the work item depends on** (addresses: missing source bug-report theme)
   Commit `.accelerator/tmp/2026-05-18-describe-pr-update-body-bug.md` to a durable location, or pull its load-bearing content (the three fix-path sketches, environment table) directly into Context and Open Questions and drop the dangling reference.

2. **Rewrite the Dependencies section to capture all three implied couplings** (addresses: cross-repo coupling, downstream consumers, gh-CLI external system)
   Replace "Blocked by: nothing locally" with an explicit external blocker on `atomic-innovation-prerelease/accelerator` (and reference the upstream-issue Open Question as a prerequisite action). Add a Blocks entry naming the three consumer skills (`describe-pr`, `review-pr`, `respond-to-pr`). Add an "External systems" entry naming `gh` CLI with the relevant version range.

3. **Pin a minimum supported `gh` version (or version matrix) and propagate it through the ACs** (addresses: "reasonably current gh release", AC #4 unbounded preconditions)
   Choose a concrete floor (e.g. "≥ 2.40, mise stable") and use that wording in Expected behaviour and AC #4 in place of "reasonably current". Enumerate the PR states (open same-repo, open cross-fork) and the `gh` versions the regression check must cover.

4. **Tighten AC #5 to a single concrete artefact and reshape it into Given/When/Then** (addresses: AC #5 malformed structure, AC #5 disjunctive artefact types, ordering between fix and regression-check)
   Restate as a Given/When/Then triple and replace "test, doc, or CI matrix entry" with the specific artefact, e.g. "an executable test in `test-pr-update-body-scripts.sh` that invokes the resolver against a real `gh` binary (skipped if `gh` is not on PATH) and fails when the resolver requests an unsupported `--json` field — shipped in the same change as the fix."

5. **Define how each failure mode in AC #3 is induced and what substring the message must contain** (addresses: error-classification AC granularity)
   For auth/network/malformed-JSON/deleted-PR, name the precise reproduction step and the required message substring (e.g. "auth", "network", "JSON", "not found").

6. **Disambiguate "that fallback path" in Open Questions** (addresses: clarity minor)
   Replace with "the operator-fallback path of running `gh pr edit --body-file` manually" or equivalent explicit phrasing.

7. **Verify or reframe the "almost certainly mocks `gh`" claim in Technical Notes** (addresses: Technical Notes speculation)
   Either read the existing `test-pr-update-body-scripts.sh` and state the fact, or mark the claim as an explicit assumption to be verified by the implementer.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally well-written with precise technical language, named actors, and concrete file/line references. The major clarity concerns are localised: a small number of ambiguous pronoun/referent issues, one acceptance criterion whose conditional structure obscures the rule, and a missing referenced source document that the reader is told to consult for context.

**Strengths**:
- Summary precisely names the failing component, the unsupported field, the gh version, and the chain of callers — no guesswork required.
- Reproduction steps, Expected behaviour, and Actual behaviour use concrete commands and exact error strings, leaving no ambiguity about what 'broken' looks like.
- Acceptance Criteria use a consistent Given/When/Then structure with named actors (operator, resolver) rather than passive constructions.
- Drafting Notes explicitly explain scoping decisions, pre-empting reader questions about intent.

**Findings**:
- 🟡 **major / high** — Referenced source bug report is not present in the checkout (References / Context). The work item repeatedly directs the reader to `.accelerator/tmp/2026-05-18-describe-pr-update-body-bug.md` for the full reproduction transcript, environment table, alternative fixes, and severity rationale. That file does not exist in this checkout, so any claim deferred to it cannot be verified or expanded by the reader.
- 🔵 **minor / high** — Fifth acceptance criterion has a malformed Given/Then structure (Acceptance Criteria 5th bullet). It omits the When clause and the Given simply re-states that the bug exists rather than naming a precondition.
- 🔵 **minor / medium** — "That fallback path" has two plausible referents (Open Questions 3rd bullet). Could refer to the `gh pr edit` command as operator workaround, or to the deprecation-failure code path inside gh.
- 🔵 **minor / medium** — "Reasonably current gh release" is left undefined (Context 1st bullet / Acceptance Criteria). An implementer choosing between fix options cannot decide which gh range they must cover.
- 🔵 **suggestion / medium** — "Almost certainly mocks gh" is speculative inside Technical Notes (Technical Notes 3rd bullet). Embeds a hedged guess in a section the reader will treat as authoritative implementation guidance.

### Completeness

**Summary**: The bug work item is structurally thorough: all expected sections are present and substantively populated, the type-required reproduction/expected/actual triad is explicit, and the frontmatter is complete and valid. The only completeness concern is that the linked source bug-report is not available in this checkout, which weakens the otherwise self-contained nature of the report.

**Strengths**:
- Frontmatter is complete and valid: type=bug, status=draft, priority=medium, and identifying fields all present.
- Summary is a clear, unambiguous statement of the defect.
- Bug-type content is fully populated: reproduction steps, expected behaviour, and actual behaviour are all present.
- Context section explains why the work matters, including discovery date, environment, affected scripts with line numbers, and a related compounding gh-CLI issue.
- Acceptance Criteria contains five specific given/when/then bullets covering happy path, fork PRs, error reporting, sibling-skill coverage, and a regression check.
- Optional sections (Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References) are all populated with substantive content.

**Findings**:
- 🔵 **minor / high** — Referenced source bug-report is not present in this checkout (References / Context). Anyone wanting to consult the linked transcript, environment table, or the three fix candidates discussed there cannot do so.

### Dependency

**Summary**: The work item explicitly enumerates the three downstream consumer skills and names the external system (gh CLI 2.65.0) that drives the defect, but the Dependencies section is dismissive — claiming nothing is blocked locally and nothing is blocked downstream — when the body in fact implies a cross-repository coupling (the fix lives in the accelerator plugin) and a gh-version coupling that should be captured.

**Strengths**:
- Context explicitly names all three skills (describe-pr, review-pr, respond-to-pr) that share the broken resolver.
- The external system (gh CLI 2.65.0) and its specific JSON-field allowlist are named precisely in Context and Requirements.
- Assumptions section calls out the cross-repository coupling possibility (accelerator plugin vs. local override).

**Findings**:
- 🟡 **major / high** — Cross-repository coupling to accelerator plugin not captured as a blocker (Dependencies). Dependencies says "Blocked by: nothing locally" but the fix lives in `atomic-innovation-prerelease/accelerator` on its own pre-release cadence.
- 🟡 **major / high** — Downstream consumer skills named in Context not listed as Blocks (Dependencies). Three consumer skills are unblocked by the fix but Dependencies records "Blocks: nothing currently in flight".
- 🔵 **minor / high** — External system (gh CLI version range) not captured in Dependencies (Dependencies). The bug is fundamentally a coupling to the `gh` CLI's `--json` field allowlist; the coupling lives only in Assumptions prose.
- 🔵 **minor / medium** — Ordering between fix and regression-check AC not explicit (Acceptance Criteria). Without ordering captured, the regression-check work risks being deferred even though AC #5 binds the two together.

### Scope

**Summary**: The work item is tightly scoped to a single defect with a clear, indivisible fix surface (one resolver script consumed by three sibling skills). The author has explicitly addressed scope decisions in Drafting Notes, deferring the related gh-CLI deprecation issue to context-only treatment.

**Strengths**:
- Explicit 'Scope decision: primary defect only' note in Drafting Notes documents the boundary call.
- Summary, Requirements, and Acceptance Criteria all describe the same unit of work.
- Acceptance criterion covering all three consuming skills keeps the fix scoped to the single shared resolver.
- Outcome-shaped acceptance criteria preserve a single coherent deliverable.

**Findings**:
- 🔵 **suggestion / medium** — Regression-check AC may be a separable chore (Acceptance Criteria). Adding real-`gh` smoke coverage to a helper that currently mocks `gh` is testing-infrastructure work whose scope could grow.
- 🔵 **suggestion / low** — Upstream-vs-local fix venue is a scope-affecting open question (Open Questions). Until resolved, the unit of work is ambiguous: upstream PR, local shim, or both.

### Testability

**Summary**: The bug specification is strongly testable: it provides exact reproduction commands, expected vs actual outcomes, and acceptance criteria framed as Given/When/Then with concrete environmental preconditions. A few criteria contain unbounded language ("any reasonably current gh release", "a real PR") and one regression-check criterion accepts alternative artefact types without a defined acceptance threshold.

**Strengths**:
- Reproduction is fully specified — exact gh version, exact command lines, and the precise expected error text.
- Expected vs Actual sections are paired and observable: exit codes, error messages, and byte-for-byte body-match contract.
- Acceptance criteria are outcome-shaped rather than solution-shaped.
- Cross-fork safety is captured as a distinct, independently verifiable criterion.
- Error-path verification is included — AC #3 requires that genuine resolution failures produce non-misleading messages.

**Findings**:
- 🔵 **minor / high** — Regression-check criterion accepts disjunctive artefact types without a pass/fail threshold (Acceptance Criteria 5th bullet).
- 🔵 **minor / high** — "Any reasonably current gh release" and "a real PR" are unbounded (Acceptance Criteria 4th bullet and Expected behaviour).
- 🔵 **minor / medium** — Error-classification criterion lists preconditions without defining how each is induced (Acceptance Criteria 3rd bullet). Does not specify how a verifier should induce each failure mode or what substring the message must contain.

## Re-Review (Pass 2) — 2026-05-18T20:13:13+00:00

**Verdict:** COMMENT

The revision resolves all three major findings and most minors from pass 1. One new major surfaces in testability: AC #4 still under-specifies what "demonstrably exercised end-to-end" means for each of the three consumer skills, even though the gh-version × PR-shape matrix is now bounded. The remaining findings are minors and suggestions that polish an already strong work item — no blocker to implementation, but worth addressing before promoting from draft.

### Previously Identified Issues

#### Pass 1 Major
- 🟡 **Clarity + Completeness**: Referenced source bug-report not present in checkout — **Resolved**. Environment table, REST PATCH confirmation, and three fix-path candidates now inlined into Context, Technical Notes, and Open Questions. Source-file reference removed.
- 🟡 **Dependency**: Cross-repo coupling to accelerator plugin not captured as a blocker — **Resolved**. Dependencies "Blocked by" now names `atomic-innovation-prerelease/accelerator` and the required pre-release, and cross-references the upstream-issue Open Question.
- 🟡 **Dependency**: Three consumer skills not listed as Blocks — **Resolved**. Dependencies "Blocks" enumerates `describe-pr`, `review-pr`, `respond-to-pr`.

#### Pass 1 Minor / Suggestion
- 🔵 **Clarity**: AC #5 malformed Given/When/Then — **Resolved**. Rewritten with explicit Given/When/Then.
- 🔵 **Clarity**: "That fallback path" ambiguous referent — **Resolved**. Replaced with "the operator-fallback path of running `gh pr edit --body-file` manually".
- 🔵 **Clarity**: "Reasonably current gh release" undefined — **Resolved**. Minimum support floor pinned at `gh ≥ 2.40.0`; Expected behaviour now refers to "the supported range named in Acceptance Criteria".
- 🔵 **Dependency**: gh CLI version range not in Dependencies — **Resolved**. "External systems" entry pins `gh ≥ 2.40.0 … latest stable` and names the `--json` allowlist as the coupling point.
- 🔵 **Dependency**: Fix↔regression-check ordering not explicit — **Resolved**. AC #5 now requires the regression test to ship in the same change; CI tooling dependency captured.
- 🔵 **Testability**: AC #5 disjunctive artefact types without threshold — **Resolved**. Single specific artefact (executable test at `scripts/test-pr-update-body-scripts.sh` with `gh`-gated skip semantics and a defined assertion).
- 🔵 **Testability**: AC #4 unbounded ("reasonably current gh", "a real PR") — **Partially resolved**. The gh-version × PR-shape matrix is now defined, but "demonstrably exercised" per skill remains under-specified — re-flagged as a new major below.
- 🔵 **Testability**: AC #3 failure-mode induction under-specified — **Resolved**. Each of the four failure modes now specifies the exact induction step and a required message substring.
- 🔵 **Clarity (suggestion)**: "Almost certainly mocks gh" speculative — **Resolved**. Reframed in Technical Notes as an explicit working hypothesis the implementer must verify; lifted to Assumptions.
- 🔵 **Scope (suggestion)**: Regression-check AC may be a separable chore — **Still present**. Scope lens re-flags this as a suggestion in pass 2 — the new real-`gh`-gated allowlist-introspection check could grow into its own sub-effort.
- 🔵 **Scope (suggestion)**: Upstream-vs-local fix venue is a scope-affecting open question — **Still present**. Scope lens re-flags in pass 2: the work item now explicitly states it cannot be closed without an upstream change, but Open Question #2 is unresolved.

### New Issues Introduced

#### Major
- 🟡 **Testability**: AC #4 — "Demonstrably exercised end-to-end" lacks defined inputs and per-skill pass/fail signal (Acceptance Criteria, AC #4)
  The regression matrix is now bounded, but the criterion never defines what success looks like for each of the three skills (`describe-pr`, `review-pr`, `respond-to-pr`) or identifies the "designated sandbox PR". A verifier could argue the criterion is met by simply invoking each skill without error.

#### Minor
- 🔵 **Clarity**: AC #3 — "The resolver" vs "the helper" boundary is implicit (Acceptance Criteria)
  AC #3 says "when the resolver runs … then it exits non-zero with a message containing the indicated substring". Unclear whether the substring must appear in the resolver's stderr, the helper's wrapped message, or both.
- 🔵 **Dependency**: Upstream issue-tracker action named but no concrete URL/ID/owner captured (Dependencies / Open Questions)
  The cross-repo blocker is identified, but the upstream issue is not yet filed and there is no link to track its progress.
- 🔵 **Dependency**: Sandbox PR fixture for AC #4 not enumerated as a prerequisite resource (Acceptance Criteria / Dependencies)
  The "designated sandbox PR" referenced in AC #4 is not named, and no entry under Dependencies records the same-repo and cross-fork PR fixtures the regression matrix requires.
- 🔵 **Testability**: AC #3 malformed-JSON precondition — PATH-stub scope under-specified (Acceptance Criteria)
  Not stated whether the `gh` stub must intercept only `pr view --json baseRepository` (or the chosen replacement sub-command) or all `gh` invocations; an indiscriminate stub could trip the auth or not-found path first.
- 🔵 **Testability**: AC #5 — source of truth for the running `gh`'s allowlist not specified (Acceptance Criteria)
  Test must "assert the resolver does not request any `--json` field absent from the running `gh`'s allowlist" but does not specify how the allowlist is obtained (e.g. parsing `gh pr view --json __probe__ 2>&1`).
- 🔵 **Testability**: AC #1 "byte-for-byte" body match risks false negatives from GitHub line-ending normalisation (Acceptance Criteria)
  GitHub's PATCH endpoint may normalise CRLF → LF or strip trailing whitespace; strict byte comparison against the local file could see spurious mismatches unrelated to the bug.

#### Suggestion
- 🔵 **Clarity**: "Sandbox" / "sandbox-scoped token" referenced without definition (Acceptance Criteria / Dependencies)
- 🔵 **Clarity**: "PATCH" used as a verb without expansion on first use (Context / Technical Notes)

### Assessment

The revision substantively addresses the pass 1 verdict drivers: the three major findings are resolved and most minors with them. The work item now stands as a self-contained, well-scoped bug report with a bounded regression matrix, explicit failure-mode preconditions, and concrete external dependencies.

One pass-1 minor (AC #4 unboundedness) returns as a pass-2 major because the matrix definition exposed a deeper underspecification — what "exercised end-to-end" means per skill — that was previously masked. The remaining new findings are surface-level polish: terminology disambiguation, a few testability edge cases (line-ending normalisation, stub scope, allowlist source), and an outstanding prerequisite-resource gap (sandbox PR fixtures).

Recommended pre-promotion actions: (1) tighten AC #4 to name per-skill observable success conditions and identify or stub-name the sandbox PR fixtures; (2) resolve Open Question #2 (file the upstream issue and link it in Dependencies). Everything else can be addressed inline during implementation without revisiting the work item.

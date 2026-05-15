---
date: "2026-05-15T10:36:50+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0058-workspace-worktree-boundary-detection.md"
work_item_id: "0058"
review_number: 1
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
---

## Work Item Review: Workspace and Worktree Boundary Detection at Session Start

**Verdict:** REVISE

The work item is structurally complete, scope-coherent, and overwhelmingly testable — every section is substantively populated and the unit of work is well-bounded. However, two major clarity issues need fixing before implementation: the cross-reference to "AC1" has no labelling scheme to resolve against (criteria are unnumbered bullets), and the colocated-case acceptance criterion under-specifies what its single emitted block must actually contain. A cluster of minor findings around testability (un-pinned prohibition wording, no baseline for "unchanged" comparison, normalisation scope) and dependency hygiene (Claude Code 2.1.0+ runtime, jj upstream coupling) would further sharpen the work item but do not block it independently.

### Cross-Cutting Themes

- **Under-specified message contracts** (flagged by: clarity, testability) — AC1's "three prohibitions" and AC3's "single coherent message" use category-level wording without pinning exact form. Both lenses suggest either a fixture/example or token-presence assertions.
- **Hook-placement decision left open** (flagged by: clarity, scope) — Requirements asks "Decide and document…" while Open Questions restates the same choice with a lean. Either decide now or split the spike out.

### Findings

#### Critical

(none)

#### Major

- 🟡 **Clarity**: Reference to 'AC1' has no antecedent labelling
  **Location**: Acceptance Criteria
  AC2 references "the same three prohibitions enumerated in AC1", but the criteria are unnumbered checkbox bullets. If criteria are reordered or inserted, the cross-reference silently shifts.

- 🟡 **Clarity**: Colocated-case criterion under-specifies the emitted block's content
  **Location**: Acceptance Criteria
  AC3 requires "exactly one additionalContext block" naming "the shared boundary path once and the parent repo path(s) for each VCS independently", but does not say whether the three prohibitions apply, nor what structural form the dual parent paths take. Two implementers could produce different outputs and both claim conformance.

#### Minor

- 🔵 **Clarity**: Term 'colocated' introduced without definition
  **Location**: Context / Requirements
  Used with two meanings (cross-VCS nesting vs same-path jj+git workspace). Define on first use and reserve a distinct phrase for the nesting case.

- 🔵 **Clarity**: 'Main workspace' and 'main worktree' used as antonyms without definition
  **Location**: Requirements / Acceptance Criteria
  AC5's negative-case test depends on what "main" means; the definition should appear in Context.

- 🔵 **Clarity**: Open question phrased as both a question and a tentative answer
  **Location**: Open Questions
  The hook-placement question states a "lean toward extending" — close it explicitly or remove the lean.

- 🔵 **Dependency**: Claude Code 2.1.0+ runtime requirement not captured as a dependency
  **Location**: Context / Dependencies
  The minimum-version coupling for silent `additionalContext` delivery is buried in Context — surface in Dependencies or Assumptions.

- 🔵 **Dependency**: Upstream jj feature request acknowledged but not framed as an external coupling
  **Location**: Technical Notes / Dependencies
  The `.jj/repo` internal marker (tracking jj-vcs/jj#8758) is a real external coupling — lift into Dependencies.

- 🔵 **Dependency**: Related ticket 0020 not mirrored in Dependencies
  **Location**: Dependencies / References
  0020 is only in References; tooling that scans Dependencies will miss it.

- 🔵 **Testability**: Three prohibitions enumerated by category but exact wording is not pinned
  **Location**: Acceptance Criteria (AC1, AC2)
  Provide a fixture sentence or reduce assertion to required keyword tokens so tests are deterministic.

- 🔵 **Testability**: 'Single coherent message' is subjective and not directly verifiable
  **Location**: Acceptance Criteria (AC3)
  Specify expected JSON shape (e.g., labelled fields `boundary_path`, `parent_jj`, `parent_git`).

- 🔵 **Testability**: Path normalisation requirement scoped only to AC4 but applies more broadly
  **Location**: Acceptance Criteria (AC4)
  Lift `realpath` normalisation into a global note or repeat in AC1/AC2/AC7.

- 🔵 **Testability**: 'Callable from a shell without sourcing the SessionStart hook' lacks a concrete test procedure
  **Location**: Acceptance Criteria (AC7)
  Pin function names, invocation form, stdout, and exit codes.

- 🔵 **Testability**: 'Existing VCS-mode context is unchanged' has no defined comparison baseline
  **Location**: Acceptance Criteria (AC5)
  Capture a golden snapshot or rephrase as byte-identical to pre-implementation output.

#### Suggestions

- 🔵 **Clarity**: Passive 'may need either extension or supplementary helpers' obscures the decision actor
  **Location**: Technical Notes
  Reword as guidance for the implementer or delete (already covered in Requirements).

- 🔵 **Scope**: Hook-placement decision embedded in scope may be a small spike
  **Location**: Requirements / Open Questions
  Either resolve the placement question in this work item before moving out of draft, or split it into a brief spike.

- 🔵 **Scope**: Type classification (story vs bug) flagged by author as debatable
  **Location**: Frontmatter: type / Drafting Notes
  Confirm against team conventions for destructive-behaviour-fix tickets.

- 🔵 **Testability**: 'Decide and document' requirement has no verifiable artefact location
  **Location**: Requirements
  Add an AC naming where the decision is recorded (top-of-file comment or ADR).

### Strengths

- ✅ Every required section is substantively populated; frontmatter is well-formed and aligned with the body (completeness).
- ✅ Subjects are named consistently across Summary, Context, Requirements, and Acceptance Criteria (clarity).
- ✅ Acceptance Criteria mostly use Given/When/Then with concrete observable outcomes and authoritative probe commands (clarity, testability).
- ✅ Detection signals defined explicitly with exact filesystem semantics (clarity).
- ✅ AC4 anchors verification on `jj workspace root` and `git rev-parse --git-common-dir` outputs (testability).
- ✅ AC7 enumerates an exhaustive classifier return set, making the helper API testable (testability).
- ✅ Scope is tightly bounded; PreToolUse enforcement is explicitly deferred to a separate story (scope).
- ✅ Out-of-scope items (gitignore introspection, symmetric main-repo messaging) are explicitly named (scope).
- ✅ Dependencies section explicitly states "Blocked by: none" rather than leaving it implicit (dependency).
- ✅ Related ticket 0020 linked with a justification for "Related rather than parent" (dependency).
- ✅ External upstream constraint (jj-vcs/jj#8758) acknowledged in Technical Notes (dependency).

### Recommended Changes

1. **Number the acceptance criteria explicitly (AC1, AC2, …)** (addresses: clarity reference-to-AC1, testability prohibition wording)
   Add `**AC1**:`, `**AC2**:`, etc. prefixes inside each checkbox, so cross-references are stable.

2. **Tighten AC3 with explicit structural form for the colocated block** (addresses: clarity colocated under-specification, testability single-coherent-message)
   Replace "names the shared boundary path once and the parent repo path(s) for each VCS independently" with a pinned JSON shape (e.g., `boundary_path`, `parent_jj`, `parent_git`) and explicitly state that the three prohibitions from AC1 also apply.

3. **Define "colocated", "main workspace", and "main worktree" in Context** (addresses: clarity term-definitions)
   Add a one-paragraph glossary distinguishing same-path colocation from cross-VCS nesting; define "main" as the checkout whose `.jj/repo` is a directory (jj) or whose `.git` is a directory equal to `--git-common-dir` (git).

4. **Pin prohibition wording or require keyword tokens in AC1** (addresses: testability prohibition wording)
   Either add a fixture sentence form, or rephrase as "the injected block contains the substrings `edit`, `VCS commands`, and `research`".

5. **Lift `realpath` normalisation into a global note** (addresses: testability path-normalisation scope)
   Add a single line under Requirements: "All emitted paths are `realpath`-normalised for macOS `/private/var` vs `/var` equivalence."

6. **Capture a baseline for AC5's "unchanged" comparison** (addresses: testability comparison-baseline)
   Rephrase as "byte-identical to the SessionStart `additionalContext` produced before this work item was implemented for the same working directory."

7. **Pin AC7 invocation contract** (addresses: testability AC7-procedure)
   State that after `source scripts/vcs-common.sh`, each helper has a named function, expected stdout, and exit code 0.

8. **Add three Dependencies entries** (addresses: dependency Claude-Code-version, dependency jj-upstream, dependency 0020-mirror)
   `Requires: Claude Code 2.1.0+ (for silent SessionStart additionalContext delivery)`; `External: jj (`.jj/repo` internal marker; tracking jj-vcs/jj#8758)`; `Related: 0020`.

9. **Close the hook-placement open question** (addresses: clarity question-and-tentative-answer, scope embedded-spike, testability decide-and-document)
   Either commit to extending `vcs-detect.sh` (with rationale in Technical Notes) or split a one-paragraph spike. If deciding now, add an AC: "the decision rationale is recorded as a comment at the top of the hook file or referenced ADR."

10. **Resolve story-vs-bug classification** (addresses: scope type-classification)
    Confirm with team conventions; flip `type` to `bug` if the team treats destructive-behaviour-absent-feature as a bug.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Largely clear with well-named referents and concrete acceptance criteria. Main issues: cross-reference to "AC1" has no labelling scheme, colocated-case AC under-specifies its emitted block, and a few terms ("colocated", "main workspace/worktree") are used without first definition.

**Strengths**:
- Subjects named consistently across all sections.
- ACs use Given/When/Then with concrete observable outcomes.
- Detection signals defined with exact filesystem semantics.
- Open Questions and Drafting Notes transparently flag non-obvious choices.

**Findings**:
- 🟡 major: Reference to 'AC1' has no antecedent labelling — criteria are unnumbered bullets; cross-reference will silently shift if criteria are reordered.
- 🟡 major: Colocated-case criterion under-specifies the emitted block's content — "names … once" and "parent repo path(s) for each VCS independently" leave structural form ambiguous; two implementers could disagree on conformance.
- 🔵 minor: Term 'colocated' introduced without definition — used with two meanings (cross-VCS nesting vs same-path jj+git workspace).
- 🔵 minor: 'Main workspace' and 'main worktree' used as antonyms without definition — AC5's test depends on this.
- 🔵 minor: Open question phrased as both a question and a tentative answer — close explicitly or remove the lean.
- 🔵 suggestion: Passive 'may need either extension or supplementary helpers' obscures decision actor.

### Completeness

**Summary**: Structurally complete and richly populated. Every expected section is present and substantively filled. Frontmatter is well-formed with recognised type/status/priority. No completeness gaps.

**Strengths**:
- Summary is action-oriented and identifies user, capability, and rationale in one sentence.
- Context explains current behaviour and failure mode without restating Summary.
- Eight specific scenario-framed ACs (positive, negative, colocated, cross-VCS, helper-API).
- All optional sections (Open Questions, Assumptions, Technical Notes, Drafting Notes, References) populated with substantive content.
- Frontmatter complete and consistent with body.

**Findings**: (none)

### Dependency

**Summary**: Captures primary upstream and downstream couplings explicitly. Implied couplings on existing artefacts are named in References/Context, though the Claude Code 2.1.0+ runtime requirement is not lifted into Dependencies and the upstream jj issue is acknowledged but not framed as a coupling.

**Strengths**:
- "Blocked by: none" stated explicitly rather than implicit.
- References enumerates existing artefacts the work touches.
- External upstream constraint (jj-vcs/jj#8758) acknowledged in Technical Notes.
- 0020 explicitly linked with justification for "Related rather than parent".

**Findings**:
- 🔵 minor: Claude Code 2.1.0+ runtime requirement not captured as a dependency — buried in Context.
- 🔵 minor: Upstream jj feature request acknowledged but not framed as an external coupling — Technical Notes only.
- 🔵 minor: Related ticket 0020 not mirrored in Dependencies — only in References.

### Scope

**Summary**: Single coherent unit of work: detection-and-injection at SessionStart. Scope is appropriately bounded — enforcement is deferred to a separate work item. Two minor observations around an embedded design decision and an author-flagged type-classification question.

**Strengths**:
- Summary, Requirements, and ACs tightly aligned on a single goal.
- Scope boundaries explicit (PreToolUse enforcement deferred).
- Cross-VCS and colocated cases treated as facets of one feature.
- Out-of-scope items (gitignore introspection, symmetric main-repo messaging) explicitly named.

**Findings**:
- 🔵 suggestion: Hook-placement decision embedded in scope may be a small spike — resolve before leaving draft or split.
- 🔵 suggestion: Type classification (story vs bug) flagged by author as debatable.

### Testability

**Summary**: Strongly testable overall — most ACs use Given/When/Then with concrete observable outputs. A few criteria contain unbounded language or ambiguous thresholds; one criterion leaves a path-normalisation gap; AC7's invocation contract is under-specified.

**Strengths**:
- AC4 anchors on exact command outputs (`jj workspace root`, `git rev-parse --git-common-dir`).
- AC5 and AC6 specify negative cases with explicit "no output, no error" outcomes.
- AC7 enumerates an exhaustive classifier return set.
- AC8 ties hook registration to a comparable existing entry.

**Findings**:
- 🔵 minor (AC1, AC2): Three prohibitions enumerated by category but exact wording is not pinned — assertion form unclear.
- 🔵 minor (AC3): 'Single coherent message' is subjective — specify expected JSON shape inline.
- 🔵 minor (AC4): Path normalisation requirement scoped only to AC4 but applies more broadly.
- 🔵 minor (AC7): 'Callable from a shell without sourcing the SessionStart hook' lacks a concrete test procedure — pin invocation, stdout, exit codes.
- 🔵 minor (AC5): 'Existing VCS-mode context is unchanged' has no defined comparison baseline.
- 🔵 suggestion (Requirements): 'Decide and document' requirement has no verifiable artefact location.

## Re-Review (Pass 2) — 2026-05-15

**Verdict:** REVISE (2 new major findings, both testability — direct consequences of pass-1 pinning)

### Previously Identified Issues

**Clarity (pass 1)**
- 🟡 AC1 reference no antecedent — Resolved (explicit AC1–AC9 numbering)
- 🟡 AC3 under-specified — Partially resolved (structural form added; field labels still missing)
- 🔵 'colocated' two meanings — Resolved (Terminology block)
- 🔵 'main workspace/worktree' undefined — Resolved (Terminology block)
- 🔵 Open question phrased as question + answer — Resolved (question removed; decision committed)
- 🔵 Passive 'may need extension' — Resolved (no longer flagged)

**Dependency (pass 1)**
- 🔵 Claude Code 2.1.0+ not in Dependencies — Resolved
- 🔵 jj upstream not framed as coupling — Resolved
- 🔵 0020 not in Dependencies — Resolved

**Scope (pass 1)**
- 🔵 Hook-placement decision embedded — Resolved (decision committed)
- 🔵 Story-vs-bug classification — Resolved (kept as story)

**Testability (pass 1)**
- 🔵 Prohibitions wording un-pinned — Partially resolved (pinned to substrings; new finding flags substrings too generic)
- 🔵 'Single coherent message' subjective — Resolved (structural form)
- 🔵 Path normalisation scoped only to AC4 — Resolved (lifted globally)
- 🔵 AC7 invocation contract — Resolved
- 🔵 AC5 'unchanged' baseline — Partially resolved (rephrased to byte-identical against snapshot; capture procedure now flagged as next gap)
- 🔵 Decide-and-document had no artefact — Resolved (AC9 added)

### New Issues Introduced

- 🟡 **Testability (AC1, AC2, AC3)**: substring assertions (`edit`, `VCS commands`, `research`) are too generic — could be trivially satisfied by phrases like "edit the workspace freely". Direct consequence of pass-1 AC1 rewrite. Tighten by requiring substrings to co-occur with parent repo path, or specify canonical sentence phrasing.
- 🟡 **Testability (AC5)**: golden-snapshot capture procedure unspecified — no defined working directories, fixture path, or pre-implementation step. Rephrasing exposed an absent capture step. Add an explicit pre-implementation capture step naming the fixture path.
- 🔵 **Clarity**: "workspace/worktree boundary" never operationally defined (filesystem subtree vs VCS-tracked set?)
- 🔵 **Clarity (AC3)**: prohibition format ambiguous for two parent repos — one block or two?
- 🔵 **Clarity**: AC7 says "or a sibling sourced module" but Requirements mandates `scripts/vcs-common.sh` — inconsistent.
- 🔵 **Clarity**: AC1 says "grep/find/research" but assertion requires literal `research` — verbiage vs assertion drift.
- 🔵 **Dependency**: existing `hooks/vcs-detect.sh`, `scripts/vcs-common.sh`, `hooks/hooks.json` are implicit prerequisites not in Dependencies.
- 🔵 **Dependency (AC5)**: pre-implementation snapshot capture is an ordering constraint not in Dependencies.
- 🔵 **Dependency**: jj/git/realpath CLI runtime requirements not named in Dependencies.
- 🔵 **Testability (AC3)**: field labels and ordering not specified.
- 🔵 **Testability (AC6)**: 'does not error' threshold (exit code? stderr?) undefined.
- 🔵 **Testability (AC7)**: failure-mode contract for helpers not specified.
- 🔵 **Testability (AC9)**: comment content lacks measurable substrings.
- 🔵 **Scope (AC9)**: documentation-of-rationale AC is meta vs behavioural (suggestion only).
- 🔵 **Scope (AC7)**: function-name contract may over-prescribe internal factoring (suggestion only).
- 🔵 **Clarity**: 'ADR' acronym used in Drafting Notes / References without expansion (suggestion only).

### Assessment

Pass 1's major findings are resolved. The two new majors in pass 2 are both refinements of the substring-pinning approach I introduced in pass 1, not structural problems with the work item. We're entering diminishing returns: each refinement pass surfaces finer-grained testability gaps.

Recommendation: address the two new majors (canonical phrasing for AC1 prohibitions; explicit snapshot capture step) and the implicit-prerequisite Dependencies bullet, then mark complete — the remaining minors are mostly suggestions a downstream planner can resolve when writing tests.

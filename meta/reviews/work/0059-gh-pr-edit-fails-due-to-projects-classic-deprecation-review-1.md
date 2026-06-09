---
date: "2026-05-15T14:16:55+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0059"
work_item_id: "0059"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
id: "0059-gh-pr-edit-fails-due-to-projects-classic-deprecation-review-1"
title: "0059-gh-pr-edit-fails-due-to-projects-classic-deprecation-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-15T14:16:55+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: gh pr edit Fails Due to GitHub Projects Classic Deprecation

**Verdict:** REVISE

The work item is structurally complete with strong reproduction steps, rich Technical Notes, and Given/When/Then-style acceptance criteria. However, four major findings cluster around two themes: an unresolved cross-fork PR scope decision (flagged by all five lenses) and two acceptance criteria (AC3, AC4) that bundle multiple checks or prescribe implementation form rather than observable outcomes.

### Cross-Cutting Themes

- **Cross-fork PR decision unresolved** (flagged by: clarity, completeness, dependency, scope, testability — all 5 lenses) — Technical Notes ends the cross-fork bullet with "Decide whether the fix supports cross-fork PRs or punts on it", but no Requirement, Open Question, or Acceptance Criterion captures the resolution. An implementer would have to surface this before starting.
- **AC3/AC4 verifiability** (flagged by: clarity, testability) — AC3 bundles four distinct checks (frontmatter stripping, body file cleanup, optional wrapper cleanup, byte-for-byte fidelity) into one bullet; AC4 prescribes a specific command form rather than an observable outcome, and its wrapper-file references create a logical inconsistency with AC3's conditional cleanup clause.

### Findings

#### Major
- 🟡 **Clarity**: Acceptance Criterion conflates a single prescribed call with an alternative form
  **Location**: Acceptance Criteria (AC4)
  AC4 offers stdin pipe vs JSON wrapper file as interchangeable, but AC3 requires removal of "any new intermediate JSON wrapper file" — which only exists in one variant. Implementer cannot tell which is preferred.
- 🟡 **Clarity**: Cross-fork PR support is identified as undecided
  **Location**: Technical Notes (Cross-fork PR caveat)
  Two reasonable implementations satisfy the criteria differently — one cross-fork-safe, one fork-blind. Reviewer can't tell whether fork-blind is a bug or accepted limitation.
- 🟡 **Testability**: AC3 bundles four distinct verifications into a single bullet
  **Location**: Acceptance Criteria (AC3)
  Single checkbox can't represent pass/fail across frontmatter stripping, body cleanup, wrapper cleanup (conditional), and byte-for-byte fidelity. The wrapper-file clause is vacuously satisfied for the stdin variant.
- 🟡 **Testability**: AC4 prescribes implementation rather than an observable outcome
  **Location**: Acceptance Criteria (AC4)
  Dictates the command form (`jq -Rs` / `--input`); a correct future refactor using a different mechanism would technically fail AC4. Outcome already covered by AC1+AC3.

#### Minor
- 🔵 **Clarity**: "Trips the deprecation" is vague about when the error fires
  **Location**: Context
- 🔵 **Clarity**: Mixed brace-style placeholders may confuse readers
  **Location**: Requirements / Acceptance Criteria
- 🔵 **Clarity**: "Matches the stripped content byte-for-byte" assumes a referent not made explicit
  **Location**: Acceptance Criteria (AC3)
- 🔵 **Dependency**: GitHub REST API coupling not named in Dependencies
  **Location**: Dependencies
- 🔵 **Dependency**: Cross-fork PR support is an unresolved coupling left to implementer
  **Location**: Technical Notes
- 🔵 **Testability**: Cross-fork PR scope decision is not reflected in any criterion
  **Location**: Acceptance Criteria
- 🔵 **Testability**: "No deprecation note is emitted" leaves the fallback path's status implicit
  **Location**: Acceptance Criteria (AC2)

#### Suggestions
- 🔵 **Completeness**: Cross-fork decision left open
  **Location**: Technical Notes
- 🔵 **Scope**: Cross-fork PR support left as an open scope decision
  **Location**: Technical Notes
- 🔵 **Dependency**: ADR-0010 precedent reference not surfaced in Dependencies
  **Location**: Dependencies
- 🔵 **Testability**: Manual verification depends on environment-specific precondition
  **Location**: Acceptance Criteria (AC5)

### Strengths

- ✅ Reproduction section captures the exact failing command and verbatim error output
- ✅ Frontmatter is complete with all nine fields and a recognised `bug` type
- ✅ Technical Notes is unusually rich for a bug — owner/repo, body encoding, method, error parity, cross-fork, ADR-0010 precedent all enumerated
- ✅ Subjects (describe-pr, review-pr, respond-to-pr) are consistently named and referent ambiguity is avoided
- ✅ Scope is tightly bounded: cross-skill impact framed as transitive inheritance, not bundled work
- ✅ Dependencies, Assumptions, Drafting Notes all substantive — no placeholder text

### Recommended Changes

1. **Resolve the cross-fork PR decision** (addresses: 5 findings across all lenses)
   Decide in-scope vs out-of-scope. If in scope: pin Requirements/AC to `gh pr view --json baseRepository` as the resolver. If out of scope: add an Open Questions entry or a note in Requirements stating cross-fork PRs are a known limitation, file a follow-up.

2. **Split AC3 into separate criteria** (addresses: AC3 bundling, AC3 referent ambiguity)
   One bullet each for: frontmatter stripping, tmp file cleanup, byte-for-byte fidelity. Name the artefact explicitly ("the content of `{prs directory}/{number}-description.md` with its YAML frontmatter removed"). Make the wrapper-cleanup clause conditional on the chosen variant, or pin the variant first.

3. **Reframe AC4 as an observable outcome** (addresses: AC4 conflation, AC4 implementation prescription)
   Either remove AC4 entirely (outcome already covered by AC1+AC3) or restate as a property test: "Given a body containing shell-special characters (backticks, `$`, literal `@`-prefixed strings), when the skill posts the body, then the posted content matches the source byte-for-byte." Move the prescriptive `jq -Rs` command form to Technical Notes.

4. **Tighten AC2 to remove the fallback ambiguity** (addresses: AC2 fallback implicit)
   Add "...on the first attempt with no GraphQL error encountered" so the criterion fails if the primary path silently regresses and the fallback masks it. Or add a separate AC that the try-edit-fallback-rest pattern is removed.

5. **Name the GitHub REST API + ADR-0010 in Dependencies** (addresses: REST coupling, ADR-0010 precedent)
   Add an external-system bullet for the GitHub REST PR-update endpoint. Add ADR-0010 as a related decision.

6. **Clarify "trips the deprecation" precondition** (addresses: Context vagueness, AC5 environment-specific)
   Note in Context whether the error appears universal or conditional on repo state. Either rewrite AC5 to a trace-based check ("no `gh pr edit` invocation appears in the trace") or name a specific reproducible repo/PR.

7. **Optional: normalise placeholder syntax** (addresses: mixed brace styles)
   Pick one convention (`{...}` for substitution tokens, or `<...>`) and apply consistently, or add a leading note explaining the conventions.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear with well-named subjects, consistent terminology, and explicit references to file paths and line numbers. The main clarity issues are a contradiction inside one Acceptance Criterion that simultaneously prescribes a single approach and parenthetically allows an alternative, and an unresolved decision point in Technical Notes (cross-fork PR support) that leaves the implementer to guess.

**Strengths**:
- Subjects are consistently named throughout (describe-pr, review-pr, respond-to-pr are introduced explicitly and reused without pronoun ambiguity).
- File paths and line numbers are cited precisely (e.g., `skills/github/describe-pr/SKILL.md:130`), eliminating referent ambiguity for the edit site.
- Actors are explicit in Requirements (the skill writes, runs, falls back, emits) rather than hidden behind passive constructions.
- Internal consistency between Summary, Context, Requirements, and Acceptance Criteria is strong.

**Findings**:
- **major / high** — Acceptance Criterion conflates a single prescribed call with an alternative form (AC4)
- **major / high** — Cross-fork PR support is identified as undecided (Technical Notes)
- **minor / medium** — "Trips the deprecation" is vague about when the error fires (Context)
- **minor / medium** — Mixed brace-style placeholders may confuse readers (Requirements / Acceptance Criteria)
- **minor / medium** — "Matches the stripped content byte-for-byte" assumes a referent not made explicit (AC3)

### Completeness

**Summary**: The work item is structurally complete for a bug: it has well-populated Summary, Context, Requirements (with reproduction + actual/expected behaviour), Acceptance Criteria, Dependencies, Assumptions, Technical Notes, and References sections, and the frontmatter is fully specified with a recognised type.

**Strengths**:
- Reproduction steps, actual behaviour (with verbatim error output), and expected behaviour are all explicitly captured under Requirements.
- Frontmatter is complete with recognised `bug` type and all nine fields populated.
- Technical Notes section is unusually rich — owner/repo resolution, body encoding, method, error parity, cross-fork caveat all enumerated.
- Acceptance Criteria contains five specific, well-scoped criteria.
- Dependencies, Assumptions, and Drafting Notes are each populated with substantive content.

**Findings**:
- **suggestion / low** — Cross-fork decision left open (Technical Notes)

### Dependency

**Summary**: The work item captures the primary external coupling (GitHub Projects classic deprecation and GitHub REST API) and correctly maps the transitive consumer relationship through review-pr and respond-to-pr. Dependencies are explicitly listed as none, which is defensible, but implied couplings (ADR-0010 precedent, cross-fork resolution via gh pr view) deserve explicit acknowledgement.

**Strengths**:
- External system coupling (GitHub Projects classic deprecation) explicitly named with sunset notice link.
- Transitive downstream consumers identified with file paths.
- ADR-0010 referenced as precedent for REST-API-via-gh api posting.
- Assumptions section justifies why fix scope is bounded to describe-pr.

**Findings**:
- **minor / medium** — GitHub REST API coupling not named in Dependencies
- **minor / medium** — Cross-fork PR support is an unresolved coupling left to implementer (Technical Notes)
- **suggestion / low** — ADR-0010 precedent reference not surfaced in Dependencies

### Scope

**Summary**: Work item 0059 is a tightly scoped bug fix targeting a single line in a single skill file, with all requirements, acceptance criteria, and technical notes converging on one coherent change. The scope is coherent and atomic — the transitive impact on `review-pr` and `respond-to-pr` is explicitly addressed as inherited rather than independent work.

**Strengths**:
- Single coherent purpose: every requirement and acceptance criterion serves the goal of eliminating the Projects-classic deprecation error.
- Scope boundaries are explicit — Drafting Notes clarifies the fix is correctly confined to `describe-pr`.
- Cross-skill impact framed as transitive inheritance, not bundled work.
- The work item explicitly considers but defers a related refactor (shared `gh` wrapper) to a future work item.

**Findings**:
- **suggestion / medium** — Cross-fork PR support left as an open scope decision (Technical Notes)

### Testability

**Summary**: The bug report has strong reproduction steps, an explicit actual-vs-expected contrast, and acceptance criteria framed in Given/When/Then. Most criteria are verifiable, though one mixes multiple distinct checks into a single bullet and another contains an implementation-detail clause that complicates pass/fail evaluation. The cross-fork caveat raised in Technical Notes is not reflected in any acceptance criterion.

**Strengths**:
- Reproduction section names the exact command and the exact error output.
- Acceptance Criteria use explicit Given/When/Then framing with observable outcomes.
- Manual verification criterion names a concrete procedure.
- AC3 specifies a byte-for-byte equality check, a definitive pass/fail measure.

**Findings**:
- **major / high** — AC3 bundles four distinct verifications into a single bullet
- **major / high** — AC4 prescribes implementation rather than an observable outcome
- **minor / medium** — Cross-fork PR scope decision is not reflected in any criterion (Acceptance Criteria)
- **minor / medium** — "No deprecation note is emitted" leaves the fallback path's status implicit (AC2)
- **minor / low** — Manual verification depends on an environment-specific precondition (AC5)


## Re-Review (Pass 2) — 2026-05-15T14:16:55+00:00

**Verdict:** COMMENT

Work item is acceptable but could be improved — see major finding below.

### Previously Identified Issues

#### Major
- 🟡 **Clarity**: Acceptance Criterion conflates a single prescribed call with an alternative form (AC4) — **Resolved** (AC4 removed; wrapper-file cleanup is now folded into a split cleanup AC with explicit "if used" wording).
- 🟡 **Clarity**: Cross-fork PR support is identified as undecided — **Resolved** (in scope; baked into Requirements, Technical Notes, and a dedicated AC).
- 🟡 **Testability**: AC3 bundles four distinct verifications — **Resolved** (split into three separate AC bullets for frontmatter stripping, cleanup, and byte-for-byte fidelity).
- 🟡 **Testability**: AC4 prescribes implementation rather than observable outcome — **Resolved** (AC4 removed; outcome covered by AC1 + byte-for-byte AC).

#### Minor / Suggestion
- 🔵 **Clarity**: "Trips the deprecation" vague — **Partially resolved** (clarified to "fires broadly on recent gh versions"; pass-2 minor flags the new phrasing still hedges, but the contradiction is reduced).
- 🔵 **Clarity**: Mixed brace-style placeholders — **Still present** (user chose to skip placeholder normalisation).
- 🔵 **Clarity**: "Matches the stripped content byte-for-byte" referent — **Resolved** (the split AC now names `{prs directory}/{number}-description.md` explicitly).
- 🔵 **Dependency**: REST API coupling not in Dependencies — **Resolved**.
- 🔵 **Dependency**: Cross-fork PR coupling unresolved — **Resolved**.
- 🔵 **Testability**: Cross-fork PR scope not in any criterion — **Resolved** (new AC3 added).
- 🔵 **Testability**: AC2 fallback implicit — **Resolved** (AC2 tightened to "primary REST path on the first attempt and no deprecation-fallback note is emitted").
- 🔵 **Testability**: Manual verification environment-specific (AC5) — **Resolved** (rewritten as trace-based: "no `gh pr edit` invocation appears in the command trace").
- 🔵 **Completeness**: Cross-fork decision left open — **Resolved**.
- 🔵 **Scope**: Cross-fork left as open scope decision — **Resolved**.
- 🔵 **Dependency**: ADR-0010 not in Dependencies — **Resolved**.

### New Issues Introduced

#### Major
- 🟡 **Testability**: Cross-fork criterion lacks a verifiable procedure
  **Location**: Acceptance Criteria (cross-fork AC)
  The criterion ("then it uses the `baseRepository` field from `gh pr view`") describes an implementation detail rather than an observable outcome. Verifier would have to inspect SKILL.md to confirm, conflating verification with code review.
  **Suggestion**: Reframe as outcome-only — "Given a PR whose head is a fork and whose base is an upstream repo, when the skill posts the body, then the PATCH request URL targets `{upstream-owner}/{upstream-repo}/pulls/{number}` and the updated body appears on the upstream PR."

#### Minor
- 🔵 **Clarity**: Passive "YAML frontmatter is stripped" obscures the actor (AC4) — does not name who strips or whether the source file is mutated on disk vs stripped in-memory.
- 🔵 **Clarity**: Placeholder tokens `{tmp directory}` / `{prs directory}` not defined in the work item itself (related to the skipped placeholder normalisation; surfaces more sharply now that other ambiguities are resolved).
- 🔵 **Dependency**: `gh` CLI and `jq` not named as tooling dependencies — the fix is mediated entirely through them.
- 🔵 **Dependency**: Downstream consumer skills (`review-pr`, `respond-to-pr`) not listed under Blocks — their user-visible fallback note is gated on this work item.
- 🔵 **Testability**: Frontmatter-stripping verification ambiguity — unclear whether the source file on disk is stripped or only the transmitted body.
- 🔵 **Testability**: Cleanup criterion ("any intermediate JSON wrapper, if the wrapper-file variant was chosen") still partly conditional on implementation choice.
- 🔵 **Testability**: First-attempt success not tied to a single observation method until the manual-verification bullet.
- 🔵 **Testability**: Expected behaviour could specify the success signal (HTTP 200, exit code 0, response body match).

#### Suggestion
- 🔵 **Clarity**: "Persistence style is the implementer's call" mildly ambiguous about whether the implementer also chooses the resolver.
- 🔵 **Dependency**: GitHub Projects-classic sunset timeline not captured as a vendor-driven deadline in Dependencies.
- 🔵 **Scope**: Frontmatter-stripping AC may read as adjacent scope — clarify it's a regression guard for existing behaviour, not new behaviour.

### Assessment

All four pass-1 major findings are resolved; the cross-cutting cross-fork theme is comprehensively addressed across Requirements, Technical Notes, Dependencies, and Acceptance Criteria. One new major was introduced: the cross-fork AC describes the implementation mechanism (which JSON field is consulted) rather than the observable outcome (which URL is hit). Fixing it is a single-bullet rewrite.

The remaining minor / suggestion findings (passive voice, placeholder definitions, tooling dependencies, conditional cleanup phrasing, success-signal specificity) are quality polish — none block implementation. The work item is ready to plan against; addressing the one cross-fork-AC rewrite would make it fully verifiable without code-review-as-test.

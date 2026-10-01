---
type: "work-item-review"
id: "0280-academic-source-profiles-review-1"
title: "Work Item Review: Academic Source Profiles"
date: "2026-09-23T02:02:06+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0280"
work_item_id: "0280"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-09-23T16:25:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Academic Source Profiles

**Verdict:** REVISE

Every expected section of the work item is filled in, the Context explains the
move from the epic's keyless/`mailto` premise to a CLI fetcher, and the
identifiers are concrete. The weaknesses are in the rules that must be
deterministic: the tier mapping has overlaps and gaps, the "source unavailable"
outcome and retry bound are undefined, the rule for assigning profiles to
researchers is ambiguous, and the hook guard's fail-closed behaviour contradicts
"main thread unaffected" when `agent_type` is absent. Scope is also heavy for
one story. It bundles a new sub-binary, a security hook, the conduct plumbing
and a human quality gate.

### Cross-Cutting Themes

- **Tier mapping is not deterministic** (flagged by: clarity, testability) —
  some inputs match more than one rule (retracted tier-1, `preprint` in an
  `is_core` source). An arXiv entry with a `journal_ref`/`doi` matches no rule,
  yet an Acceptance Criterion tests that exact fixture. "Recognised index" is
  undefined.
- **"Source unavailable" contract undefined** (flagged by: completeness,
  clarity, testability) — no exit code, JSON shape or retry bound is given. The
  criteria mix "exits with" and "returns", and the `429` retry case overlaps the
  `429` budget-exhaustion case.
- **`agent_type` capability and fail-closed guard** (flagged by: clarity,
  dependency, testability, scope) — the hook identifies the researcher only
  through `agent_type`, so failing closed without it would block the main
  thread. The Claude Code version that provides `agent_type` is an unrecorded
  platform dependency and may force a plugin-wide raise of the minimum version.
- **Profile-to-researcher assignment** (flagged by: clarity, testability) — it
  is unclear whether a focus area gets one profile or one researcher per
  profile, and no assignment rule is stated, so "the round mixes" is not
  deterministic.
- **Stale correction notes for 0278 and 0121** (flagged by: dependency, scope)
  — both corrections appear to have been applied already.

### Findings

#### Major

- 🟡 **Clarity / Testability**: Academic tier mapping has overlapping and
  unresolved cases
  **Location**: Requirements
  A retracted journal article matches both tier-1 and tier-3, and an arXiv
  entry with a `journal_ref` or `doi` matches no rule. The mapping needs an
  explicit precedence order and a defined fallback.
- 🟡 **Clarity / Dependency / Testability**: Guard fail-closed conflicts with
  "main thread unaffected"; `agent_type` is an uncaptured platform dependency
  **Location**: Open Questions / Dependencies / Acceptance Criteria
  Without `agent_type` the hook cannot tell a researcher call from a
  main-thread call. The Dependencies section omits the Claude Code capability
  and the version it needs, and no criterion covers the missing-`agent_type`
  case.
- 🟡 **Testability / Clarity / Completeness**: "Source unavailable" outcome
  and retry bound never defined
  **Location**: Requirements
  Three criteria assert an outcome that has no observable definition. "Bounded
  exponential backoff" gives no maximum number of attempts.
- 🟡 **Clarity / Testability**: Unclear whether a focus area is researched by
  one profile or by every profile
  **Location**: Summary / Requirements
  The two models differ in researcher count, how the `breadth` cap applies,
  and findings per focus area. No assignment rule is stated.
- 🟡 **Testability**: Several requirements have no acceptance criterion
  **Location**: Acceptance Criteria
  No criterion covers `synthesise` carrying tiers forward unchanged, profiles
  never using `WebFetch`, the keyless path, key-resolution precedence, record
  fields beyond tier and URL, or arXiv `GET`-only over one connection.
- 🟡 **Testability / Clarity**: Hook block set is open-ended; "bare
  invocation" undefined
  **Location**: Acceptance Criteria / Requirements
  The examples omit `||`, `&`, newlines and process substitution. The spec
  does not say whether a launcher path qualified with `${CLAUDE_PLUGIN_ROOT}`,
  or a quoted query containing `|`, is allowed. A security boundary needs an
  explicit allow/deny table.
- 🟡 **Scope**: Story bundles several streams that could ship independently
  **Location**: Requirements
  The fetcher sub-binary, the `PreToolUse` guard, the conduct/profile plumbing
  and the human gate could each close separately. Any one of them slipping
  delays the gate that 0283 waits on.

#### Minor

- 🔵 **Clarity / Completeness**: OpenAlex key described as both required and
  optional; the keyless path is not covered by Assumptions
  **Location**: Context / Assumptions
  The Context says a key "has required" since February 2026, but the
  Requirements make it optional. By the item's own figures, a keyless round
  exhausts its allowance about ten times over.
- 🔵 **Completeness**: Argument surface of `search`/`lookup` and the contents
  of "venue signals" are not specified
  **Location**: Requirements
  The profiles and the hook both depend on the command's interface.
- 🔵 **Clarity**: `E_TOKEN_CMD_FROM_SHARED_CONFIG` is called both an emission
  and a warning; the case of `openalex.api_key` in shared config is not stated
  **Location**: Requirements / Acceptance Criteria
- 🔵 **Dependency**: OpenAlex and arXiv are not recorded as external
  dependencies
  **Location**: Dependencies
- 🔵 **Dependency**: Quality-gate prerequisites (distributed build in a
  consuming repo, reviewer, API key) not captured; 0283 unblocks on sign-off,
  not merge
  **Location**: Acceptance Criteria (quality gate)
- 🔵 **Dependency / Scope**: Pending corrections to 0278 and 0121 appear
  already applied
  **Location**: Dependencies / Drafting Notes
- 🔵 **Testability**: "No source-specific branches" in `researcher.md` has no
  mechanical check
  **Location**: Acceptance Criteria
- 🔵 **Testability**: Quality gate does not require academic sources in
  `synthesis.md` and does not name the reference subject
  **Location**: Acceptance Criteria
- 🔵 **Scope**: OpenAlex and arXiv could ship as separate increments
  **Location**: Requirements
- 🔵 **Scope**: Human quality gate is bundled with the engineering work, and it
  validates the epic's premise, not only this slice
  **Location**: Acceptance Criteria
- 🔵 **Scope**: A possible plugin-wide raise of the minimum Claude Code
  version is embedded in a feature story
  **Location**: Open Questions

#### Suggestions

- 🔵 **Clarity**: "Slice 1", "the epic" and "injection seam" are used before
  being linked to 0277 and 0121
  **Location**: Summary / Context
- 🔵 **Clarity**: The spec does not say where a retraction is "noted"
  **Location**: Requirements
- 🔵 **Dependency**: `conduct` overlap with 0282 is not recorded
  **Location**: Dependencies
- 🔵 **Testability**: Round-time and budget assumptions have no thresholds
  **Location**: Assumptions

### Strengths

- ✅ The Summary is a proper user story, and the Context explains the forces
  (OpenAlex key and pricing, arXiv throttling, `WebFetch` limits) behind the
  CLI-fetcher design.
- ✅ Concrete identifiers throughout: command shape, config keys, env vars,
  error code, HTTP statuses and headers.
- ✅ Deriving tiers in the CLI is justified explicitly as making the mapping
  deterministic and testable.
- ✅ Several criteria are directly testable: the fixture-driven tier set, the
  three-second serialisation, and the credential criterion's no-leak check.
- ✅ Boundaries are explicit: 0284 owns tier rendering, the citation pass is
  deferred with a reason, and the web mechanism is reused.
- ✅ Work-item couplings (blocked by 0277, blocks 0283 with a rationale) agree
  with the neighbouring items.
- ✅ "Reputation tier" is defined as venue standing, not claim correctness.
- ✅ The hook criterion includes the negative case: other agents and the main
  thread are unaffected.

### Recommended Changes

1. **Rewrite the tier mapping as an ordered decision list** (addresses: tier
   mapping overlaps/gaps)
   Retracted → tier-3, then the tier-1 rules, then tier-2, else tier-3. State
   the tier for arXiv entries that have a `journal_ref`/`doi`, and name the
   `listed_in` indexes that count. Optionally put the expected tier for each
   fixture into the criterion.
2. **Resolve the `agent_type` Open Question before planning** (addresses:
   fail-closed conflict, platform dependency, embedded floor raise)
   Either commit to a minimum Claude Code version that supplies `agent_type`,
   recorded as a dependency, or state the behaviour when it is absent and
   reconcile it with "main thread unaffected". Add a criterion for that case.
3. **Define the "source unavailable" contract and retry bound** (addresses:
   undefined outcome, exit/return wording, `429` overlap)
   Give the exit status, the stdout JSON shape (including the source family),
   the maximum number of attempts, and a statement that `429` +
   `X-RateLimit-Remaining: 0` means budget exhaustion.
4. **State the profile assignment rule** (addresses: one-vs-every profile,
   non-deterministic "round mixes")
   For example: round-robin in `source_profiles` order, one finding per focus
   area. Fix the preconditions of the first criterion accordingly.
5. **Turn the hook criterion into an allow/deny table** (addresses: open-ended
   block set, undefined "bare")
   Specify the accepted command form, including quoted operator characters in
   queries and the path form of the launcher.
6. **Add criteria for requirements that have none** (addresses: requirements
   without criteria)
   Cover `synthesise` tier carry-forward, no `WebFetch` in the academic
   profiles, keyless OpenAlex without an `Authorization` header, resolution
   precedence, and the record fields.
7. **Decide on decomposition** (addresses: under-decomposition, separable
   families, bundled gate)
   Consider three children: the fetcher CLI, the Bash grant plus guard, and the
   profiles plus conduct plumbing plus gate. Alternatively, move the gate into
   its own task that blocks 0283.
8. **Tidy Dependencies and Context** (addresses: external APIs, gate
   prerequisites, stale 0278/0121 notes, keyless wording, 0282 overlap)

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Precise for the most part, but the rules that must be
deterministic have ambiguities: tier-mapping overlaps, the fail-closed guard
contradicting "main thread unaffected", and one-versus-many profile
assignment. There are also smaller wording inconsistencies (key required vs
optional, warning vs error, exit vs return).

**Strengths**: concrete identifiers; Context reasons through the design change;
"reputation tier" is defined; actors are named explicitly.

**Findings**:
- major/high — Requirements — Tier mapping has overlapping and unresolved
  cases.
- major/high — Open Questions — Guard fail-closed conflicts with "main thread
  unaffected" when `agent_type` is absent.
- major/medium — Summary — Unclear whether a focus area is researched by one
  profile or by every profile.
- minor/high — Requirements — OpenAlex key described as both required and
  optional.
- minor/medium — Requirements — "Source unavailable" undefined; exit vs return
  worded inconsistently; `503`/`429` overlap.
- minor/medium — Acceptance Criteria — `E_TOKEN_CMD_FROM_SHARED_CONFIG` called
  both an emission and a warning; shared `api_key` case unstated.
- minor/medium — Requirements — "Bare" invocation and "recognised index"
  undefined.
- suggestion/medium — Summary — "Slice 1", "the epic" and "injection seam"
  used before being linked.
- suggestion/low — Requirements — Where the retraction is "noted" is
  unspecified.

### Completeness

**Summary**: A well-populated story with every expected section present. The
small gaps: the argument surface, the "source unavailable" outcome, and the
Assumptions for the keyless path.

**Strengths**: a proper user story; Context explains the forces behind the
design; extensive Given/When/Then criteria; Dependencies, Assumptions and Open
Questions all filled in; tier mapping written out; Technical Notes point to
implementers' assets.

**Findings**:
- minor/medium — Requirements — "Source unavailable" outcome referenced as
  documented but never defined.
- minor/medium — Requirements — `search`/`lookup` argument surface
  unspecified.
- minor/medium — Assumptions — Keyed budget covered, keyless default path not.

### Dependency

**Summary**: Work-item couplings are well recorded. Missing: the external
APIs, the Claude Code `agent_type` capability, and the quality gate's
prerequisites. The 0278/0121 correction notes look stale.

**Strengths**: blocked by 0277 with a reason; blocks 0283 in both frontmatter
and body; ownership of rendering handed to 0284 and mirrored there; internal
building blocks named.

**Findings**:
- major/high — Open Questions / Dependencies — Claude Code `agent_type`
  hook-input capability is an uncaptured platform dependency.
- minor/high — Dependencies — OpenAlex and arXiv not recorded as external
  dependencies.
- minor/medium — Acceptance Criteria — Quality-gate prerequisites not
  captured.
- minor/high — Dependencies / Drafting Notes — Pending corrections to 0278 and
  0121 appear already applied.
- suggestion/medium — Requirements / Dependencies — Overlap with 0282 on
  `conduct` not noted.

### Scope

**Summary**: Coherent in purpose, with explicit boundaries, but too large for
one story since the CLI-fetcher rework.

**Strengths**: explicit out-of-scope boundaries; Summary, Requirements and
Acceptance Criteria are aligned; builds on 0277 rather than duplicating it;
accepted risks recorded.

**Findings**:
- major/medium — Requirements — Story bundles several independently
  deliverable streams.
- minor/medium — Requirements — OpenAlex and arXiv support are separable
  increments.
- minor/medium — Acceptance Criteria — Human output-quality gate bundled with
  the engineering delivery.
- minor/low — Open Questions — Possible plugin-wide floor raise embedded in a
  feature story.
- suggestion/medium — Dependencies — Unclear whether correcting 0278/0121 is
  in scope.

### Testability

**Summary**: Most criteria are concrete Given/When/Then statements, but
several depend on outcomes the item never defines, and several requirements
have no criterion.

**Strengths**: fixture-driven tier criterion; CLI-side tiering justified for
testability; measurable three-second threshold; credential criterion includes
a no-leak check; the human gate has an auditable artefact; the hook criterion
includes the negative case.

**Findings**:
- major/high — Requirements — "Source unavailable" outcome and retry bound
  never defined.
- major/high — Requirements — Tier mapping has gaps and overlaps that the
  fixture criterion depends on.
- major/high — Acceptance Criteria — Several requirements have no
  corresponding criterion.
- major/medium — Acceptance Criteria — Hook criterion's block set is
  open-ended and misses the fail-closed case.
- minor/medium — Acceptance Criteria — Profile assignment rule unspecified.
- minor/medium — Acceptance Criteria — "No source-specific branches" lacks a
  concrete check.
- minor/medium — Acceptance Criteria — Human gate does not require academic
  sources to appear or name the reference subject.
- suggestion/low — Assumptions — Round-time and budget assumptions have no
  thresholds.

## Re-Review (Pass 2) — 2026-09-23T15:42:18+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Tier mapping overlaps and gaps — Resolved
  (the grouping of the tier-1 "or" clauses remains a minor issue)
- 🟡 **Clarity / Dependency / Testability**: Guard fail-closed vs `agent_type`
  — Resolved (`agent_id`/`agent_type` present since v2.1.69)
- 🟡 **Testability / Clarity / Completeness**: "Source unavailable" contract
  and retry bound — Partially resolved (the shape and bound are defined, but
  how each failure maps to a `reason` and the backoff timing are unasserted)
- 🟡 **Clarity / Testability**: Profile assignment — Resolved (`outline`
  chooses profiles; one finding per (focus area, profile))
- 🟡 **Testability**: Requirements without criteria — Partially resolved
  (`--limit`, DOI lookup and invalid-argument exits still have no criterion)
- 🟡 **Testability / Clarity**: Hook block set open-ended — Resolved (edge
  cases for prefix matching and whitespace trimming remain a minor issue)
- 🟡 **Scope**: Under-decomposed — Accepted by the author; now minor
- 🔵 Keyless wording, argument surface, `E_TOKEN_CMD_FROM_SHARED_CONFIG`,
  external dependencies, gate prerequisites, gate academic-source
  requirement, floor raise — Resolved
- 🔵 **Dependency / Scope**: Stale 0278/0121 notes — Resolved, but the new
  0282 entry is itself stale

### New Issues Introduced

- 🟡 **Dependency**: 0281's outline proposals must adopt the `— profiles:`
  suffix; the coupling is unrecorded
- 🟡 **Clarity**: "Returned nothing" conflates a successful empty result with
  an unavailable source, which risks re-spawning the same researcher forever
- 🟡 **Testability**: The credential-ladder criterion never tests precedence
  or the condition that `config.local.md` is absent
- 🟡 **Testability**: `--limit` bounds, DOI/`W…` lookup and invalid-argument
  exits have no criterion; the handling of `--limit` above 25 is unstated
- 🟡 **Testability / Clarity**: How each failure maps to an unavailable
  `reason`, and the backoff/`Retry-After` behaviour, are unspecified and
  unasserted
- 🔵 **Dependency**: 0282 is done, so this slice lands second and reconciles
  the key-count test, `dump.golden` and `public-api.txt`; 0279 (gap-fill
  origin) is unrecorded; 0284 also consumes the multi-finding layout
- 🔵 **Clarity**: Rung 5 "absent" should say file, not key; the `tier-3
  (retracted)` text conflicts with "0284 owns all tier rendering"; the arXiv
  fixture mapping relies on a count; unclear whether a failed sign-off
  unblocks 0283
- 🔵 **Completeness**: The contents of the academic profile skills are barely
  specified; the epic's documentation criterion is not carried as an
  acceptance criterion
- 🔵 **Scope**: The Summary omits the change to the finding unit; nobody owns
  the 0121 amendment; the outcome of a failed gate is unstated
- 🔵 **Testability**: The profile-swap check has no observation procedure;
  boundary tier fixtures are missing; the precondition of the unavailable
  criterion is vague

### Assessment

The design-level majors from pass 1 are resolved. The five remaining majors
are narrow specification gaps (empty-result semantics, precedence tests, CLI
argument criteria, reason mapping, and 0281 coupling), not open design
questions. One more editing pass should reach COMMENT or APPROVE.

## Re-Review (Pass 3) — 2026-09-23T15:56:41+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency**: 0281 coupling — Resolved (unannotated items default to
  `web`)
- 🟡 **Clarity**: "Returned nothing" conflation — Resolved (a mixed case with
  some calls unavailable and zero records remains a minor issue)
- 🟡 **Testability**: Credential-ladder precedence — Resolved (key-leak checks
  on error paths remain a minor issue)
- 🟡 **Testability**: CLI argument criteria — Resolved (`research fetch` help
  text is still unasserted)
- 🟡 **Testability / Clarity**: `reason` mapping and backoff — Resolved
  (tolerances for the wait times and an in-range `Retry-After` case remain
  minor issues)
- 🔵 0282/0279 records, rung-5 wording, tier-rendering ownership, fixture
  pairing, failed sign-off, profile contents, documentation criterion,
  Summary scope, 0121 ownership, profile-swap procedure, boundary fixtures,
  unavailable precondition — Resolved

### New Issues Introduced

- 🟡 **Clarity**: The gate names "three judgements" but lists four or five
  checks; it is unclear whether the citation minimums count toward the verdict
- 🟡 **Clarity**: The format of a ticked outline item carrying several profiles
  is undefined
- 🟡 **Testability**: No criterion checks that the tier the CLI assigns reaches
  the finding unchanged
- 🟡 **Testability**: The "`preprint` in an `is_core` source" fixture leaves
  out the source type and version that decide its tier
- 🟡 **Scope**: The gate plus "a failed judgement keeps this story open"
  makes completion open-ended after release
- 🔵 **Clarity**: Tier rule 2 first clause vs `preprint`; the scope of "only a
  `submittedVersion`"; whether the shared `api_key_cmd` refusal is
  unconditional; how the hook resolves the researcher name
- 🔵 **Completeness / Clarity**: The list of 0121 passages to amend is
  incomplete, including 0121's hook keyed on `agent_type` alone
- 🔵 **Dependency**: 0283 and 0284 do not mirror the constraints this story
  places on them; the 0279 edge is absent from frontmatter; 0278's
  relationship is unexplained; 0281's reading of multi-finding layouts is not
  addressed
- 🔵 **Testability**: No criterion for the retraction suffix, quarantined
  `.invalid` pairs, the stubbing seam, or measuring gate spend; tier boundary
  fixtures remain

### Assessment

Every pass-2 major is resolved. The five pass-3 majors are local wording and
fixture gaps. Four are mechanical fixes; the fifth, the gate's open-ended
completion, is the author's decision about where the gate lives. After this
pass the item is close to COMMENT.

## Re-Review (Pass 4) — 2026-09-23T16:18:27+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Gate "three judgements" mismatch — Resolved (four named
  judgements)
- 🟡 **Clarity**: Ticked multi-profile outline format — Resolved (no finding
  links, as 0277 built)
- 🟡 **Testability**: Tier carry-through from CLI to finding — Resolved
- 🟡 **Testability**: Underspecified `preprint` fixture — Resolved (a
  15-row table; one row still lacks its work type)
- 🟡 **Scope**: Open-ended gate completion — Accepted by the author
- 🔵 Every pass-3 minor finding — Resolved, apart from the shared-rung
  credential edge cases, which are now narrower

### New Issues Introduced

- 🟡 **Clarity**: The researcher outcome rules mix "returned records" and
  "relevant records", so irrelevant records plus an unavailable call can
  close a pair
- 🟡 **Testability**: No criterion shows that `outline` ever assigns an
  academic profile
- 🟡 **Testability**: No outcome is defined for a lookup `404`, a `401`/`403`
  (bad key), a `400`, or network errors; a bad key could yield a false
  "None found"
- 🟡 **Testability**: The gap-fill criterion does not assert that `<nn>` is
  reused or that the checkbox flips
- 🔵 **Clarity**: The `<nn>` allocation rule; rule 4's "no version" wording;
  shared-rung edge cases; the time-bound formula; `<slug>` clashing with the
  epic's glossary Slug
- 🔵 **Testability**: `breadth` not pinned in the fan-out criterion; the
  canonical-URL precedence; a missing `lookup` ID; the arXiv pacing
  criterion should use separate processes; the tracked/insecure
  `config.local.md` refusal; samples of one for model-driven criteria
- 🔵 **Scope**: The Summary omits the 0281/0283/0284 amendments; split-seam
  notes
- 🔵 **Dependency**: 0281/0284 not ordered after this story
- 🔵 **Completeness**: No sign-off placeholder in Technical Notes

### Assessment

Every pass-3 major is resolved. Pass 4's majors are narrower still. Two are
real behavioural gaps: the outcome classification, and HTTP errors outside
the specified set, where a bad key must not produce a false "None found".
The other two are extensions of existing criteria. The review is converging:
each pass now finds edge cases in the previous pass's additions rather than
structural problems.

## Verdict Override — 2026-09-23T16:21:12+00:00

**Verdict:** COMMENT

The author applied every pass-4 major and minor finding and set the verdict
to COMMENT without a fifth pass. The fixes were: outcome classification by
relevant records; defined outcomes for `404`, `401`/`403`, other `4xx`, and
network errors; an `outline` academic-assignment criterion plus an unedited
outline for the gate; and `<nn>` reuse and checkbox assertions on gap-fill.
The remaining risk sits in edge cases that planning will surface, and
successive passes were converging on detail rather than structure.

## Verdict Override — 2026-09-23T16:25:57+00:00

**Verdict:** APPROVE

The author approved the work item for implementation after the sibling
contracts (0121, 0281, 0283, 0284) were amended to match.

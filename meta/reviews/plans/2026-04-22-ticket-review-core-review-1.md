---
date: "2026-04-22T18:30:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-22-ticket-review-core.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, compatibility, usability, standards, documentation]
review_pass: 2
status: complete
---

## Plan Review: Ticket Review (Core) — Phase 4

**Verdict:** REVISE

The plan is well-structured, carefully scoped, and faithfully mirrors the
proven `/review-plan` orchestrator model. Its TDD discipline, golden-fixture
invariants for `pr`/`plan` output, and explicit non-goals give it a strong
foundation. However, across all eight lenses it surfaces a cluster of related
semantic and diagnosability issues (silent cross-mode filtering of
`core_lenses`/`disabled_lenses`, under-specified `applies_to` parsing,
permanent `min_lenses` warning in ticket mode), two critical test-coverage
gaps (the `mise run test` wiring claim is factually wrong, and the
orchestrator has no committed automated regression protection), documentation
gaps (README/CHANGELOG not updated for discoverability, lens SKILL.md bodies
are sketched rather than specified), and several contract-stability and
standards nuances that deserve explicit treatment before the refactor lands.

### Cross-Cutting Themes

- **Silent cross-mode filtering is a diagnosability problem** (flagged by:
  architecture, code-quality, correctness, usability — 4 lenses). The plan
  deliberately suppresses warnings when a `core_lenses` or `disabled_lenses`
  entry is valid in another mode but not the active one. This is
  indistinguishable from a typo or a silent bug to the user, and
  contradicts the existing convention of warning on unrecognised lens
  names. All four lenses recommend the same remediation: surface the
  filtered entries somewhere visible (Review Configuration block or
  info-level stderr) to preserve the no-false-positive intent while giving
  the user an audit trail.

- **`applies_to` is under-specified for malformed inputs** (flagged by:
  code-quality, correctness, compatibility, usability — 4 lenses). The
  plan defines the happy-path (`[pr, plan, ticket]`, absent = all modes)
  but does not specify behaviour for typos (`[prr]`), non-array scalars
  (`pr`), empty arrays (`[]`), duplicates, block form, or unknown modes.
  The existing `config_parse_array` helper's behaviour on these inputs is
  accidental rather than designed, so the contract is effectively
  undefined.

- **`min_lenses` default of 4 is wrong for ticket mode** (flagged by:
  architecture, correctness, compatibility — 3 lenses). After Phase 4D,
  ticket mode has exactly 3 built-in lenses and `min_lenses` defaults to
  4. Every default ticket review will emit a spurious
  "Only 3 lenses available, but min_lenses is 4" warning, with no way to
  resolve until Phase 5 lands. All three lenses recommend per-mode
  defaults or a conditional suppression.

- **Malformed agent output fallback is both untested and
  semantically coupled to verdict thresholds** (flagged by: test-coverage,
  correctness, usability — 3 lenses). Plan inherits review-plan's
  "treat as single major finding" strategy. There's no automated test
  path; under `ticket_revise_severity: major` one flaky agent always
  forces REVISE; and the error UX drops raw text on users with no
  remediation guidance.

- **Output format enum listing Phase 5 lens names creates
  forward-compat risk** (flagged by: architecture, compatibility — 2
  lenses). Pre-listing `scope` and `dependencies` in the
  `ticket-review-output-format` enum invites agents to emit those values
  before the lenses exist. Both lenses recommend either listing only the
  three existing lenses, or explicitly noting the phase-gating, or
  dropping the enum in favour of the catalogue being the single source
  of truth.

- **`allowed-tools` for `review-ticket` grants more than needed**
  (flagged by: code-quality, usability — 2 lenses). The proposed
  `Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)` entry grants
  access to mutating scripts (`ticket-update-tags.sh`) just to read two
  frontmatter fields. Both lenses recommend either dropping the entry
  (read the ticket file directly as review-plan does) or scoping it to
  `ticket-read-*` only.

- **README and CHANGELOG not updated for `/review-ticket`
  discoverability** (flagged by: documentation, usability — 2 lenses).
  The README has dedicated "Ticket Management" and "Review System"
  sections listing existing skills; neither will mention `/review-ticket`
  after Phase 4 ships. CHANGELOG "Unreleased" similarly lacks an entry.
  Users who skim these surfaces won't discover the new command exists.

- **Lens SKILL.md content is sketched, not specified** (flagged by:
  documentation). The plan describes Core Responsibilities as "3-4
  numbered items covering..." and Key Evaluation Questions as "grouped
  by applicability" but provides no actual prose. Without at least one
  worked example per lens, the skill-creator may produce lenses that
  overlap in scope (e.g., "ambiguous requirements" could land in either
  clarity or completeness), causing rework or runtime finding
  duplication.

- **`BUILTIN_PR_LENSES` and `BUILTIN_PLAN_LENSES` duplicate the same
  13 names** (flagged by: standards). The current single
  `BUILTIN_LENSES` array is one source of truth; splitting into two
  identical arrays invites future divergence. A single
  `BUILTIN_CODE_LENSES` with aliasing would preserve single-source
  semantics.

- **Lens SKILL.md "seven-section pattern" is actually six sections**
  (flagged by: standards). The plan names the count as seven but
  enumerates six items, which matches the actual lens files. Minor
  authoring-time confusion risk.

### Tradeoff Analysis

- **Silent filtering vs noisy feedback**: The plan's silent-filter
  choice avoids false-positive warnings for cross-mode configs (a real
  concern) but at the cost of hiding genuine user mistakes. The lenses
  converge on "informational note, not warning" as the balanced
  resolution.

- **DRY vs skill-file independence**: code-quality flags that
  `review-ticket` is a large prose duplicate of `review-plan` and two
  files will drift. The counter-argument is that skill files are read
  verbatim by the model and an `!include` mechanism doesn't exist in
  this plugin today. Reasonable resolutions: accept the duplication
  consciously (document the canonical source) or add a diffing test
  that enforces parity of the shared sections.

### Findings

#### Critical

- 🔴 **Test Coverage**: `mise run test` does not actually run the ticket-script suite
  **Location**: Current State Analysis (lines 28-29); per-phase Success Criteria
  The plan's regression gate assumes `tasks/test.py` invokes
  `skills/tickets/scripts/test-ticket-scripts.sh`; inspection shows it
  invokes only `scripts/test-config.sh` and the ADR-scripts test. Every
  "mise run test exits 0" checkbox provides a weaker guarantee than the
  plan claims, and new ticket-side regressions would be invisible to CI.

- 🔴 **Test Coverage**: Orchestrator correctness has no automated regression protection
  **Location**: Phase 4E Success Criteria
  Phase 4E relies entirely on a manual smoke test plus `skill-creator`
  evals that are explicitly not committed. Verdict aggregation,
  re-review append semantics, malformed-agent fallback, and glob-based
  prior-review detection all lack committed coverage. The orchestrator
  composes everything else in the plan and carries the highest
  regression risk, yet is the least tested.

#### Major

- 🟡 **Architecture / Code Quality / Correctness / Usability**: Silent
  cross-mode filtering of `core_lenses` / `disabled_lenses`
  **Location**: Phase 4A §1 (validation bullet)
  Filtering entries silently when they apply in some other mode makes
  typos and mis-scoped config indistinguishable from a silent bug.
  Contradicts the existing "warn on unrecognised lens" convention.
  Suggested: emit an info-level note in the `## Review Configuration`
  block listing filtered entries and the mode they'd apply in, or add
  scoped keys (`ticket.core_lenses`) now rather than deferring.

- 🟡 **Architecture**: Two mechanisms for lens-to-mode partitioning
  **Location**: Phase 4A §1 + Phase 4C §3
  Built-in lenses are partitioned via hardcoded `BUILTIN_*_LENSES`
  arrays while custom lenses declare `applies_to` in frontmatter. A
  built-in lens can never appear in multiple modes; an `applies_to` on a
  built-in lens is specified to be silently ignored. Either unify on
  `applies_to` for all lenses, or explicitly forbid `applies_to` on
  built-in lenses so the two mechanisms cannot collide.

- 🟡 **Code Quality**: `applies_to` parsing duplicates existing awk blocks
  **Location**: Phase 4A §1 (custom lens discovery)
  The custom-lens discovery loop already has two near-identical ~17-line
  awk blocks (for `name` and `auto_detect`); adding a third is technical
  debt, and `applies_to` is a flow-array, not a scalar, so the existing
  pattern does not apply unchanged. Extract
  `_read_frontmatter_scalar` and `_read_frontmatter_array` helpers
  before the third copy lands.

- 🟡 **Code Quality**: `review-ticket` is specified as a near-duplicate of `review-plan`
  **Location**: Phase 4E §1 (Process Steps)
  500+ lines of orchestrator prose will exist twice and inevitably
  drift. When a bug is fixed in re-review append semantics or malformed-
  JSON fallback in one, the other will be forgotten. Options: shared
  orchestrator skill, `!include`-style composition, or a diffing test
  that fails when shared sections diverge.

- 🟡 **Correctness / Architecture / Compatibility**: `min_lenses` default
  of 4 exceeds the final ticket-lens count (3)
  **Location**: Phase 4A available-lens-count check; implication
  through 4D
  After Phase 4D, every default ticket review emits a spurious
  "Only 3 lenses available, min_lenses is 4" warning. The plan
  acknowledges this for the 0-lens intermediate state but not for the
  Phase-4-complete end state. Either introduce a per-mode
  `min_lenses` default, cap `effective_min_lenses` to the catalogue
  size, or raise it when Phase 5 lenses arrive.

- 🟡 **Correctness / Compatibility / Architecture**: Output format enum
  lists Phase 5 lens identifiers that won't exist until Phase 5
  **Location**: Phase 4B §3 (`ticket-review-output-format/SKILL.md`)
  The enum enumerates `completeness, testability, clarity, scope,
  dependencies` — three exist after Phase 4, two don't. Agents may
  emit `scope` or `dependencies` findings that downstream consumers
  can't attribute. Either limit the enum to existing lenses, drop the
  enum in favour of the catalogue, or phase-gate the unreleased
  identifiers.

- 🟡 **Correctness / Test Coverage / Usability**: Malformed-agent-output
  fallback creates a coupling to verdict thresholds
  **Location**: Phase 4E §1 (eval scenarios — malformed output); Phase
  4B verdict thresholds
  Treating malformed output as a single `major` finding deterministically
  forces REVISE under `ticket_revise_severity: major` — one flaky agent
  determines verdict. Also, this path is listed as an eval but there's
  no described mechanism for the eval harness to inject malformed
  output. Options: demote fallback to `suggestion`, mark fallback
  findings as synthetic and exclude from threshold rules, or document
  the coupling explicitly.

- 🟡 **Correctness / Code Quality / Compatibility**: `applies_to` parsing
  under-specified for malformed values
  **Location**: Phase 4A §1 (custom lens `applies_to` bullet)
  The plan doesn't specify behaviour for typos (`[prr]`), non-array
  scalars (`pr`), empty arrays (`[]`), duplicates, or block-sequence
  YAML form. The existing `config_parse_array` helper's behaviour on
  these inputs is accidental. Add a `validate_applies_to` helper, spell
  out the rejected-form warnings, and add test cases.

- 🟡 **Correctness**: Re-review has no fallback when prior frontmatter
  is malformed
  **Location**: Phase 4E Step 7 (offer re-review)
  The initial-review path has a "warn and proceed as if no prior review
  exists" fallback for malformed prior-review frontmatter; the re-review
  path does not. An unparseable prior file could be silently corrupted
  further by the "update three fields in place" strategy. Extend the
  fallback symmetrically: warn, and write a fresh `-review-{N+1}.md`
  file instead of appending.

- 🟡 **Test Coverage**: Eval scenarios for lens SKILL.md files are
  explicitly not committed
  **Location**: Testing Strategy — SKILL.md evals subsection
  Lens SKILL.md files have no post-authoring regression protection.
  Prose-is-the-product: a future edit that neutralises a lens
  directive is a behaviour change with no CI signal. Commit the eval
  scenarios as checked-in fixtures under each lens's directory.

- 🟡 **Test Coverage**: `applies_to` filter under-tested against adversarial inputs
  **Location**: Phase 4A test list
  Only three happy-path cases are covered (`[plan]`, absent,
  `[ticket, plan]`). Add: empty array, unknown-mode value, missing
  value, non-list scalar, duplicates — with assertions on warning text
  or documented fallback behaviour.

- 🟡 **Test Coverage**: Byte-for-byte golden comparison needs explicit
  path-normalisation strategy
  **Location**: Phase 4A Success Criteria
  The script emits absolute lens paths that vary by checkout location
  and CI worker. Either normalise (`{PLUGIN_ROOT}` placeholder) or
  record per-run — the plan doesn't say which. Specify now so every
  phase reuses the same normalisation.

- 🟡 **Test Coverage**: Cross-mode validation test in 4C is fragile and
  couples invariants to specific lens names
  **Location**: Phase 4C test list
  The deferred `core_lenses: [architecture, completeness]` assertion
  passes only because of the current catalogue membership. Use a
  fixture custom lens with explicit `applies_to` to isolate the
  filter-semantics invariant from catalogue churn.

- 🟡 **Test Coverage**: Verdict threshold config overrides tested only
  via eval + one manual check
  **Location**: Phase 4E eval scenarios; Manual Verification
  Threshold × severity × finding-count is a combinatorial branch;
  one scenario gives 2/24 coverage. Off-by-one bugs on `>=` vs `>` on
  `major_count` will escape. Add eval scenarios for each severity/count
  boundary.

- 🟡 **Compatibility**: Usage-message change breaks the error-path
  contract for tooling that greps the stderr text
  **Location**: Phase 4A §1 (usage message update)
  Current `Usage: config-read-review.sh <pr|plan>` string may be
  consumed by wrappers or doc tools. Either pin the new text in 4A
  success criteria + Migration Notes, or construct the usage string
  dynamically so the wording is trivially correct.

- 🟡 **Compatibility**: Silent per-mode filtering weakens the "no
  behavioural change to `/review-plan`" promise on stderr
  **Location**: Phase 4A §1; Desired End State §9
  Pre-change, `disabled_lenses: [completeness]` in a plan setup warns
  ("unrecognised lens"); post-change, once `completeness` is a
  recognised ticket lens, the warning silently disappears for plan
  mode. Either extend golden-fixture coverage to stderr or scope
  §9 explicitly to stdout.

- 🟡 **Usability**: Asymmetric default (ticket=2 vs plan=3) is surprising
  and rationale isn't surfaced in the configure docs
  **Location**: Phase 4B §1 and §4
  Users comparing the two tables will see two near-identical keys with
  different defaults and no explanation. Add a short "why" note under
  the ticket review table in `configure/SKILL.md`, or align both
  defaults to 3 and let users tune.

- 🟡 **Usability**: `/review-ticket` no-args behaviour diverges from
  `/review-plan` despite structural parity
  **Location**: Phase 4E §1 (no-args eval scenario)
  `/review-plan` doesn't suggest running `/list-plans`; `/review-ticket`
  is specified to suggest `/list-tickets`. Pick one pattern and
  document the rationale.

- 🟡 **Documentation**: README and CHANGELOG not updated for
  `/review-ticket` discoverability
  **Location**: Desired End State / Migration Notes
  The README "Ticket Management" section (lines 244-271) and "Review
  System" section (lines 311-334) won't mention `/review-ticket` or the
  three ticket lenses. CHANGELOG "Unreleased" has no entry. Add Phase
  4E Changes Required entries for both.

- 🟡 **Documentation**: Lens SKILL.md bodies are sketched, not specified
  **Location**: Phase 4C §3 and 4D §3-4
  Core Responsibilities and Key Evaluation Questions are described as
  bullet lists of topics, not actual prose. Without at least one worked
  example per lens (a sample responsibility paragraph, two key
  questions), the skill-creator may produce lenses that overlap in scope.
  Add a "lens scope boundaries" subsection mapping each failure mode to
  exactly one of the three lenses.

- 🟡 **Documentation**: Output format lens enum lists lenses that won't
  exist after this phase
  **Location**: Phase 4B §3 Field Reference
  Reinforces the architecture/compatibility forward-compat finding.
  First-time readers inspecting the output format alongside the Lens
  Catalogue will see an unreconcilable mismatch without Phase 5
  context.

#### Minor

- 🔵 **Architecture**: Shared `min_lenses`/`max_lenses` defaults
  unsuitable for 3-lens ticket catalogue — even without the Phase-5
  trajectory, per-mode defaults are the cleaner long-term model.

- 🔵 **Architecture**: `config-read-review.sh` accumulating mode-specific
  logic in four places (partitioning, validation, verdict emission,
  available-count) weakens cohesion. A `MODE_CONFIG` data structure
  would simplify future mode additions.

- 🔵 **Architecture**: Review artifact's `target: "path"` breaks when
  tickets are renamed. Tickets have a stable 4-digit ID; consider
  persisting `ticket_id` alongside `target`.

- 🔵 **Architecture**: Reviewer agent's Read/Grep/Glob/LS tooling
  remains enabled for ticket reviews; prose instruction "do not
  evaluate the codebase" is a weak barrier. Ticket lens `What NOT to
  Do` sections should include "do not read source code".

- 🔵 **Code Quality**: Available-lens count arithmetic is subtle post-
  refactor (pre-filter by mode, intersect with disabled). Risk of
  off-by-one; add an explicit test per mode × disabled-overlap.

- 🔵 **Code Quality**: `_select_builtin_lenses_for_mode` idiom not
  pinned. macOS default Bash 3.2 rules out `nameref`; nominate the
  approach (e.g., `echo` space-separated + `read -ra`) in the plan.

- 🔵 **Code Quality**: Missing `validate_applies_to` helper to parallel
  existing `validate_severity`/`validate_positive_int` discipline.

- 🔵 **Code Quality / Usability**: `allowed-tools` for `/review-ticket`
  grants `Bash(.../skills/tickets/scripts/*)` — broader than the
  read-only scripts the prose actually calls, and includes mutating
  scripts. Scope to `ticket-read-*` or drop entirely.

- 🔵 **Code Quality**: Deferred cross-mode test in 4A is a known-stale
  assertion — use a fixture custom lens with `applies_to: [ticket]`
  to exercise the behaviour immediately.

- 🔵 **Code Quality**: Usage string should be updated in the header
  doc-comment at lines 4-9 of the script, not just the stderr text.

- 🔵 **Correctness**: `ticket_revise_severity: none` + major-count
  independence rule is implicit by reference to `review-plan`; an
  implementer may drop the independence. Add an eval:
  `ticket_revise_severity: none` with 3 majors expecting REVISE.

- 🔵 **Correctness**: Default `core_lenses` at script line 30 names
  PR/plan lenses only. With silent filtering, ticket mode's effective
  `core_lenses` is empty by default. Either per-mode defaults or
  auto-include-all for ticket mode when `core_lenses` is unset.

- 🔵 **Correctness**: Custom-lens name-collision check currently
  iterates `BUILTIN_LENSES`. After partitioning, clarify whether the
  check is against the union (prevents future collisions across modes)
  or just the active-mode array.

- 🔵 **Correctness**: Concurrent `/review-ticket` invocations can
  corrupt the review artifact — mirrors review-plan's inherited
  assumption. Either call it out in What We're NOT Doing or add an
  advisory lock.

- 🔵 **Test Coverage**: Fixture custom-lens in 4A manual verification
  is placed in the working tree by hand; prefer the `setup_repo`
  hermetic pattern already used elsewhere in `test-config.sh`.

- 🔵 **Test Coverage**: No test covers the interaction between
  `applies_to` (custom lens) and `core_lenses`/`disabled_lenses`
  cross-mode references.

- 🔵 **Test Coverage**: Single hand-chosen ticket for the 4E E2E smoke
  test doesn't exercise legacy (`adr-creation-task`) or
  deliberately-malformed tickets.

- 🔵 **Test Coverage**: Three-row assertion in 4D couples the test to
  array-declaration order; prefer set-equality.

- 🔵 **Compatibility**: `ticket` mode with zero built-in lenses in 4A
  will warn on every invocation until 4D lands — trap for any
  interim release.

- 🔵 **Compatibility**: Phase 4B §4 updates the minimal custom-lens
  template example — keep the three-field form as the minimal template
  and show `applies_to` in a separate "optional fields" snippet to
  preserve backwards-compatible onboarding.

- 🔵 **Compatibility**: Ticket-stem vs plan-stem naming share a logical
  namespace; document that consumers must key on `type:` frontmatter,
  not filename, to disambiguate reviews.

- 🔵 **Usability**: Flow-array syntax for `applies_to` isn't
  discoverable — add a worked example in `configure/SKILL.md` and an
  explicit "omitting = applies to all modes" callout.

- 🔵 **Usability**: Review Configuration label style
  ("ticket revise major count") reads unevenly next to
  "pr request changes severity". Either annotate with the raw config
  key or document the label-to-key mapping.

- 🔵 **Usability**: Nonexistent-ticket error message under-specified;
  reuse `update-ticket`'s phrasing and accept ticket-number shorthand
  for input ergonomics.

- 🔵 **Usability**: No plan to surface `/review-ticket` in top-level
  discoverability surfaces (README, CLAUDE.md). If plugin.json + skill
  registration is the intended surface, note it explicitly.

- 🔵 **Standards**: Lens SKILL.md "seven-section pattern" is actually
  six sections. The plan enumerates six items correctly but names the
  count as seven. Change to "six-section pattern" or drop the count.

- 🔵 **Standards**: Sub-phase labelling "Phase 4A–4E" diverges from
  prior plans which use "Subphase 2.1/3.1" decimal form. Rename to
  "Subphase 4.1–4.5" for consistency, or note the departure.

- 🔵 **Standards**: Missing explicit `user-invocable` note on
  `review-ticket` orchestrator frontmatter. The omission is correct
  (it is user-invocable), but Phase 4C/4D explicitly set
  `user-invocable: false` on lenses — add a note explaining the
  asymmetry so the implementer doesn't copy the wrong setting.

- 🔵 **Standards**: Output-format trailing reminder paragraph placement
  should be explicitly noted as outside the last H2 section, matching
  `plan-review-output-format/SKILL.md:102`.

- 🔵 **Standards**: `applies_to` frontmatter key has no documented
  value vocabulary or validation — add an "unrecognised mode" warning
  analogous to the existing "unrecognised lens" warning.

- 🔵 **Standards**: `BUILTIN_PR_LENSES` and `BUILTIN_PLAN_LENSES`
  duplicate the same 13 names. Keep a single `BUILTIN_CODE_LENSES`
  and derive per-mode arrays from it to preserve single-source-of-truth.

- 🔵 **Standards**: Preamble trailing line
  `!config-read-skill-instructions.sh review-ticket` omits the full
  `${CLAUDE_PLUGIN_ROOT}/scripts/` prefix used by all other skills.

- 🔵 **Documentation**: Configure SKILL.md header update is incomplete
  — secondary references to review-pr/review-plan (agents table,
  per-skill customisation examples, troubleshooting) are not updated
  to mention review-ticket.

- 🔵 **Documentation**: Process Step names in review-ticket are terse
  for first-time readers; "Relationship to Other Commands" section
  omits `/list-tickets` and lacks the lifecycle-diagram format used
  by review-plan.

- 🔵 **Documentation**: `applies_to` semantics docs need two
  side-by-side examples (omit = all modes; explicit = ticket-only) to
  disambiguate the default for plugin authors.

- 🔵 **Documentation**: Silent cross-mode filtering of
  `core_lenses`/`disabled_lenses` needs user-facing documentation in
  `configure/SKILL.md` explaining the behaviour and its interaction
  with typo detection.

- 🔵 **Documentation**: Review artifact frontmatter schema is
  documented only in the plan, not in any user-facing surface. Add a
  brief "Review Artifact" subsection to `review-ticket/SKILL.md`.

#### Suggestions

- 🔵 **Architecture**: Define `applies_to` semantics for unknown future
  modes (warn and treat as absent) to lock the forward-compat story.

- 🔵 **Code Quality**: Severity expectations in lens evals (e.g.,
  "expect a major finding") are brittle — prefer severity floors
  ("at major or higher") for judgmental cases.

- 🔵 **Usability**: Malformed-agent-output warning includes the raw
  output but no remediation guidance; add "try a narrower lens
  selection or file a bug with the raw output above" alongside.

### Strengths

- ✅ Strong TDD discipline: tests-fail-first, script-change,
  tests-pass is explicit; each phase leaves the system in a working
  state.
- ✅ Golden-fixture invariants for `pr`/`plan` output carried across
  every sub-phase anchor the "no behavioural change" promise.
- ✅ Sub-phase dependency ordering (4A → 4E) is well-chosen; failures
  in any phase localise cleanly.
- ✅ Explicit non-goals (don't mutate ticket `status`, don't alter
  `/review-plan` or `/review-pr`) prevent scope creep.
- ✅ Review-ticket faithfully mirrors `review-plan` (frontmatter
  schema, re-review append semantics, fallback behaviour) — maximises
  transfer of learning.
- ✅ Reviewer agent and plugin manifest are unchanged; plan correctly
  identifies that no registration work is required.
- ✅ Evolutionary forward-look: the `BUILTIN_TICKET_LENSES` extension
  point and pre-enumerated output-format fields anticipate Phase 5.
- ✅ Validation edge cases for ticket verdict keys (invalid severity,
  invalid count, `none` severity) are well enumerated in 4B.
- ✅ Config key naming (`ticket_revise_severity`,
  `ticket_revise_major_count`) mirrors existing
  `plan_revise_severity` pattern.
- ✅ `applies_to` introduced with sensible backwards-compatible
  default (absent = all modes) for existing custom lenses.
- ✅ Frontmatter conventions on new skills (`user-invocable`,
  `disable-model-invocation`, `allowed-tools`, `argument-hint`) are
  consistent with existing lens, output-format, and orchestrator
  conventions across the repo.
- ✅ Config key naming and exit-code conventions match existing
  patterns exactly (`snake_case` keys, `exit 1` on unknown mode).
- ✅ Output-format SKILL.md section structure (JSON Schema, Field
  Reference, Severity Emoji Prefixes, Finding Body Format) mirrors
  `plan-review-output-format` precisely.
- ✅ Desired End State item 8 makes documentation a first-class
  deliverable; Phase 4B manual verification explicitly checks
  configure SKILL.md from a "first-time reader" perspective.
- ✅ Example finding in the output format uses a realistic ticket
  section ("Acceptance Criteria"), giving lens authors a concrete
  pattern to follow.

### Recommended Changes

Ranked by impact.

1. **Fix the `mise run test` wiring claim and add orchestrator regression
   coverage** (addresses: both critical findings).
   a. Add an explicit pre-flight step to Phase 4A that registers
      `skills/tickets/scripts/test-ticket-scripts.sh` in `tasks/test.py`
      (or the equivalent mise configuration), with its own Success
      Criteria checkbox. Correct the Current State Analysis (lines
      28-29) so it matches reality.
   b. Commit the `/review-ticket` orchestrator eval scenarios under
      `skills/tickets/review-ticket/evals/` with a shell entry point,
      and wire that entry point into `tasks/test.py`. Minimum
      scenarios: verdict aggregation across severity-threshold
      boundaries, malformed-agent fallback, re-review frontmatter
      preservation.

2. **Decide cross-mode filtering UX** (addresses: 4-lens silent-filter
   theme).
   Pick one of the three lenses' recommendations and apply it uniformly:
   (a) add a visible "Filtered for this mode" row in the Review
   Configuration block; (b) emit an info-level stderr line; or (c)
   introduce scoped config keys (`ticket.core_lenses`). Document the
   choice in `configure/SKILL.md`. Note: the plan explicitly declines
   (c) today; (a) is the least-disruptive fix.

3. **Fix `min_lenses` default for ticket mode** (addresses:
   architecture/correctness/compatibility permanent-warning finding).
   Introduce a per-mode default (e.g., `DEFAULT_TICKET_MIN_LENSES=3`)
   or suppress the warning when `available_count == 0` and no custom
   ticket lenses are installed. Add a test that default ticket mode
   produces no configuration warning at rest.

4. **Specify `applies_to` parsing completely** (addresses: 4-lens
   under-specification theme).
   In Phase 4A §1: list the accepted YAML forms (flow only vs. flow
   and block); define behaviour for typos (warn), non-array scalars
   (warn, fall back to "all modes"), empty arrays (define explicitly),
   and unknown modes (warn). Add a `validate_applies_to` helper and
   cover each case in `test-config.sh`.

5. **Decide the output-format enum story** (addresses: forward-compat
   theme).
   Pick one: limit the enum to three existing lenses and update it in
   Phase 5, or drop the enum and reference the Lens Catalogue as the
   single source of truth, or add explicit "phase-gated" annotation.
   Apply matching guidance to `review-ticket` Step 4 on how to handle
   a finding whose `lens` is not in the active catalogue.

6. **Fix `allowed-tools` for `/review-ticket`** (addresses:
   code-quality/usability finding).
   Scope to `Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/ticket-read-*)`
   or drop the entry and read the ticket file directly.

7. **Extract `applies_to` + `_read_frontmatter_*` helpers before
   duplicating the awk pattern a third time** (addresses: code-quality
   DRY finding).
   Collapse the existing `name` and `auto_detect` blocks into shared
   helpers, and add `_read_frontmatter_array` for `applies_to`.

8. **Add re-review frontmatter-malformed fallback** (addresses:
   correctness re-review finding).
   Extend Step 7 symmetrically with Step 1: if prior frontmatter is
   unparseable, warn the user and write a fresh `-review-{N+1}.md`
   instead of appending in place.

9. **Address malformed-agent-output + verdict coupling** (addresses:
   3-lens fallback theme).
   Pick one: demote fallback findings to `suggestion` severity,
   flag them as synthetic and exclude from threshold evaluation, or
   document the coupling in `configure/SKILL.md` for users selecting
   `ticket_revise_severity: major`.

10. **Commit lens SKILL.md eval scenarios** (addresses: test-coverage
    prose-drift finding).
    Move evals from `meta/tmp/review-core-evals/` to
    `skills/review/lenses/{lens}-lens/evals/` as checked-in fixtures,
    so future edits have regression protection.

11. **Normalise golden-fixture comparison** (addresses: test-coverage
    golden-fixture finding).
    Specify `{PLUGIN_ROOT}` placeholder substitution in Phase 4A
    test helpers so fixtures are portable and subsequent phases
    reuse the same approach.

12. **Add rationale for `ticket_revise_major_count=2` in configure docs**
    (addresses: usability asymmetric-default finding).
    One-sentence note under the ticket review table explaining why
    the ticket threshold differs from the plan threshold.

13. **Align `/review-ticket` no-args UX with `/review-plan`**
    (addresses: usability no-args divergence finding).
    Decide whether no-args redirects to `/list-tickets` (novel) or
    shows only example invocations (existing pattern) and apply
    consistently.

14. **Update the script header doc-comment** (addresses: code-quality
    stale-docs minor finding).
    Add to 4A Changes Required: update lines 4-9 of
    `config-read-review.sh` to list all three modes.

15. **Strengthen ticket lens `What NOT to Do` sections**
    (addresses: architecture codebase-exploration finding).
    Ensure all three ticket lenses explicitly forbid reading source
    code, mirroring Phase 4C's "Don't cross-check code" bullet.

16. **Update README and CHANGELOG for `/review-ticket` discoverability**
    (addresses: documentation major finding).
    Add Phase 4E Changes Required entries: README "Ticket Management"
    section gains a review-ticket row, "Review System" gains a ticket
    review subsection, and CHANGELOG "Unreleased" gains "Added" bullets
    for the new skill, lenses, output format, and config keys.

17. **Add lens scope boundaries or worked examples**
    (addresses: documentation lens-body-sketched finding).
    For each of completeness, testability, and clarity, add at least
    one sample Core Responsibility paragraph and two key evaluation
    questions to the plan. Alternatively, add a "lens scope boundaries"
    subsection mapping each failure mode to exactly one lens.

18. **Use single `BUILTIN_CODE_LENSES` array**
    (addresses: standards array-duplication finding).
    Keep one 13-element array; derive `BUILTIN_PR_LENSES` and
    `BUILTIN_PLAN_LENSES` as aliases in `_select_builtin_lenses_for_mode`.

19. **Fix "seven-section" count to "six-section"**
    (addresses: standards section-count finding).
    One-word change in Key Discoveries.

20. **Rename sub-phases to "Subphase 4.1–4.5"**
    (addresses: standards labelling finding).
    Match the decimal form used by Phase 2 and Phase 3 plans, or
    note the departure explicitly.

21. **Fix preamble trailing line to use full `${CLAUDE_PLUGIN_ROOT}/scripts/` path**
    (addresses: standards preamble finding).
    Match the convention in `review-plan/SKILL.md:638`.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Sound structural choices overall — preserves
lens/orchestrator/reviewer-agent separation, models review-ticket
faithfully on review-plan, and deliberately scopes pr/plan behaviour
to zero change via golden fixtures. Main concerns: coupling between
built-in partitioning and `applies_to` (two ways to express the same
concept), weakening cohesion in `config-read-review.sh`, and
evolution-risk spots (hardcoded lens enum, shared `min/max_lenses`,
silent filtering) that deserve explicit acknowledgement before being
locked in.

**Strengths**: Strong adherence to existing structural model; clean
separation of concerns; dependency ordering well-chosen; evolutionary
fitness for Phase 5 considered; functional-core / imperative-shell
separation preserved.

**Findings**: 2 major, 5 minor, 1 suggestion (see full findings list
above).

### Code Quality

**Summary**: Broadly well-structured and follows the repo's existing
TDD-driven shell conventions, but several aspects introduce
maintenance friction: `BUILTIN_LENSES` split coupled with
`all_lens_names` validation shifts semantics non-obviously;
`applies_to` parsing duplicates two existing awk blocks; the new
`review-ticket` SKILL.md is a near-copy of a 600+-line `review-plan`
rather than identifying reusable structure.

**Strengths**: Strong TDD discipline for both shell and SKILL.md;
golden-fixture invariants; explicit scope boundaries; helper
extraction flagged; cross-mode validation addressed; correct
sub-phase ordering.

**Findings**: 3 major, 5 minor, 2 suggestions.

### Test Coverage

**Summary**: Strong TDD for the shell-script refactor, but coverage
falls off sharply after the shell-script layer: orchestrator has no
automated regression coverage at all, SKILL.md evals are explicitly
discarded after authoring, and several promised behaviours
(malformed-agent fallback, verdict thresholds, re-review append) live
entirely inside the unverified orchestrator. The `mise run test`
wiring claim is factually wrong.

**Strengths**: Explicit tests-fail-first sequencing; golden-fixture
invariants carried forward; cross-mode negative assertions; edge-case
validation enumerated; thoughtful deferral of flaky-test scenarios.

**Findings**: 2 critical, 6 major, 4 minor.

### Correctness

**Summary**: Thorough and well-structured, but contains several gaps
around cross-mode semantics, verdict edge cases, and re-review error
handling. The most significant: `min_lenses=4` default vs the
eventual 3-ticket-lens catalogue guarantees a permanent warning;
malformed-agent fallback + `ticket_revise_severity: major` creates
flaky verdicts; re-review has no fallback for malformed prior
frontmatter.

**Strengths**: Cross-mode validation problem addressed; ticket
default values justified in-line; re-review semantics carefully
specified on the happy path; golden-fixture regression tests.

**Findings**: 4 major, 5 minor.

### Compatibility

**Summary**: Mostly sound on contract stability — byte-for-byte
invariant for pr/plan output is explicitly promised and covered; the
`applies_to` field is framed as additive with sensible defaults.
However, the usage-message change is itself a contract break for
tooling that parses stderr, the lens identifier enum lists two Phase 5
lenses that don't yet exist, and the plan swaps loud validation
warnings for silent filtering without a migration notice.

**Strengths**: Golden-fixture byte-for-byte parity; `applies_to`
backwards-compatible default; reviewer-agent contract preserved;
plugin manifest registration verified; config keys additive;
artifact frontmatter structurally identical to plan-review.

**Findings**: 3 major, 4 minor.

### Usability

**Summary**: Carefully structured with strong attention to backwards
compatibility and consistency with existing review-plan/review-pr
patterns, but has notable DX gaps: silent cross-mode filtering with
no hint to the user; undocumented rationale for the asymmetric
default; divergent no-args behaviour between `/review-ticket` and
`/review-plan` despite structural-parity claim.

**Strengths**: Backwards compatibility as first-class goal; parity
with review-plan maximises transfer of learning; golden fixtures give
confidence; configure skill updated in-phase; Steps 6 and 7 mirror
proven UX.

**Findings**: 3 major, 6 minor, 1 suggestion.

### Standards

**Summary**: Well-structured and largely consistent with repo
conventions for SKILL.md frontmatter, allowed-tools patterns, config
key naming, and review artifact schemas. Notable deviations: the plan
miscounts the lens section pattern as "seven" when it's six;
sub-phase labelling uses alpha suffixes (4A-4E) rather than the
decimal form (4.1-4.5) established by prior ticket-management plans;
`BUILTIN_PR_LENSES` and `BUILTIN_PLAN_LENSES` duplicate 13 names
that were previously a single source of truth.

**Strengths**: Frontmatter conventions precisely match existing
lenses; config key naming follows `snake_case` pattern; exit-code
behaviour matches; output-format section structure mirrors plan
variant; `argument-hint` syntax follows established convention;
review artifact frontmatter is structurally identical to plan-review.

**Findings**: 0 major, 8 minor.

### Documentation

**Summary**: Documentation-heavy and generally well-structured with
concrete line-range references and explicit doc deliverables in
Desired End State. However, README and CHANGELOG are not updated for
discoverability, lens SKILL.md bodies are sketched rather than
specified (risking scope overlap between lenses), the output format
enum lists lenses that won't exist yet, and several secondary
references in configure/SKILL.md are missed.

**Strengths**: Exact documentation artifacts enumerated; Desired End
State makes docs a first-class deliverable; manual verification
includes first-time-reader perspective check; example finding uses
realistic ticket section; Migration Notes correctly explains no user
action needed for config.

**Findings**: 3 major, 5 minor.

---

## Re-Review (Pass 2) — 2026-04-22

**Verdict:** COMMENT

Both critical findings from Pass 1 are fully resolved. The majority of
major findings are resolved or substantially addressed. One accepted
tradeoff remains (review-ticket near-duplication of review-plan), and a
handful of minor items carry forward. The plan is ready for
implementation; the remaining items below are improvement suggestions,
not blockers.

### Resolution Summary

#### Critical (2/2 resolved)

- ✅ **`mise run test` does not run ticket-script suite** — Resolved.
  Phase 4A now has a pre-flight step 0 that wires
  `test-ticket-scripts.sh` into `tasks/test.py`. Current State
  Analysis corrected.

- ✅ **Orchestrator has no automated regression protection** —
  Resolved. Testing Strategy now commits evals as checked-in fixtures
  alongside each skill under `evals/` directories. Orchestrator evals
  are wired in at `skills/tickets/review-ticket/evals/`.

#### Major — Resolved (15/19)

- ✅ Silent cross-mode filtering → informational line in Review
  Configuration block.
- ✅ Two mechanisms for lens-to-mode partitioning → consolidated to
  `BUILTIN_CODE_LENSES` + `BUILTIN_TICKET_LENSES`; applies_to is
  custom-lens-only.
- ✅ `applies_to` parsing duplicates awk blocks →
  `_read_frontmatter_scalar` / `_read_frontmatter_array` helpers
  specified.
- ✅ `min_lenses` default of 4 exceeds ticket catalogue → per-mode
  defaults introduced (ticket=3).
- ✅ Output format enum lists Phase 5 lenses → limited to 3 existing
  lenses with explicit phase-gating note.
- ✅ Malformed-agent fallback verdict coupling → demoted to
  `suggestion` severity with `synthetic: true` marker.
- ✅ `applies_to` parsing under-specified → `validate_applies_to`
  helper with adversarial test cases added.
- ✅ Re-review has no malformed-frontmatter fallback → symmetric
  fallback added to Step 7.
- ✅ Lens evals explicitly not committed → Testing Strategy changed
  to commit evals alongside skills.
- ✅ `applies_to` filter under-tested → adversarial test cases added
  (typo, empty, scalar, duplicate).
- ✅ Golden comparison needs normalisation → per-run recording
  strategy documented.
- ✅ Cross-mode test in 4C fragile → fixture custom lens with
  `applies_to: [ticket]` specified.
- ✅ Verdict threshold config under-tested → 4 boundary eval
  scenarios added.
- ✅ Asymmetric default rationale missing → note added under ticket
  review table.
- ✅ README/CHANGELOG not updated → Phase 4E deliverable added.

#### Major — Accepted Tradeoff (1/19)

- ⚠️ **`review-ticket` is a near-duplicate of `review-plan`** — The
  plan acknowledges this in Tradeoff Analysis. No `!include` mechanism
  exists in the plugin today. Accepted: the canonical source is
  `review-plan`, and divergence is managed via the implementation
  workflow (copying + adapting) rather than structural deduplication.

#### Major — Partially Resolved (2/19)

- 🔵 **`allowed-tools` scope** — Scoped to `ticket-read-*` glob. The
  correctness agent notes that `config-*` scripts also remain in
  `allowed-tools`; these are needed for preamble execution and are
  consistent with `review-plan`.

- 🔵 **Lens SKILL.md bodies sketched, not specified** — Lens Scope
  Boundaries table added mapping each failure mode to exactly one
  lens. Individual lens sections now have detailed eval scenarios.
  The correctness agent suggests adding one worked Core Responsibility
  paragraph per lens — this is a minor polish item, not a blocker.

- ✅ **Usage-message change breaks error-path contract** — Usage
  string update is in the header doc-comment step, and the exit-code-1
  test now asserts the usage text contains `pr|plan|ticket`.

### New Findings (Pass 2)

#### Minor

- 🔵 **Test Coverage**: Eval commit/discard wording inconsistency —
  Phase 4C line 628 previously said "discarded after the skill is
  finalised" while Testing Strategy says "committed as checked-in
  fixtures". **Now fixed** in this pass (Phase 4C updated to
  "committed alongside the skill").

- 🔵 **Correctness**: `validate_applies_to` spec says "Accepts only
  YAML flow-array form" but also says scalars are accepted. **Now
  fixed** — wording updated to "Accepts YAML flow-array form and
  bare scalars".

- 🔵 **Test Coverage**: Phase 4A ticket-mode test references
  `min_lenses=4` but the plan introduces per-mode default of 3 for
  ticket mode. **Now fixed** — test assertion updated to reference
  per-mode default.

- 🔵 **Correctness**: `synthetic: true` marker on malformed-agent
  fallback findings has no schema definition in the output format
  SKILL.md. **Now fixed** — added as optional `synthetic: boolean`
  field in the JSON Schema section, documented as orchestrator-internal
  metadata.

- 🔵 **Architecture**: The dual-mechanism boundary (built-in arrays
  vs `applies_to` frontmatter) is now well-documented but the two
  mechanisms remain conceptually overlapping. This is clean enough
  for Phase 4; Phase 5 could consider unifying if more modes are
  added. (Accepted — no plan change needed.)

- 🔵 **Architecture**: Review artifact `target: "path"` uses a
  filesystem path rather than the ticket's stable 4-digit ID.
  **Now fixed** — `ticket_id` field added to the review artifact
  frontmatter schema alongside `target`.

- 🔵 **Standards**: Sub-phase labelling "Phase 4A–4E" diverges from
  prior plans' decimal form. **Now fixed** — rationale note added
  to the Implementation Approach section explaining why alpha
  suffixes are used.

- 🔵 **Standards**: Label-to-key mapping in Review Configuration
  output (e.g., "ticket revise major count" vs config key
  `ticket_revise_major_count`) could benefit from showing the raw
  key in parentheses. **Now fixed** — label format updated to
  include raw config key.

#### Suggestions

- 💡 Default `core_lenses` at script line 30 names PR/plan lenses
  only. With per-mode filtering, ticket mode's effective `core_lenses`
  is empty by default. **Now fixed** — ticket mode auto-includes all
  built-in ticket lenses when `core_lenses` is unset.

- 💡 Severity expectations in lens evals ("expect a major finding")
  are brittle. Prefer severity floors ("at major or higher"). **Now
  fixed** — all eval scenarios updated to use "at X or higher"
  phrasing.

### Per-Lens Summaries (Pass 2)

**Architecture** (7/8 resolved): Core structural concerns addressed —
BUILTIN_CODE_LENSES consolidation, per-mode min_lenses, output format
enum scoping, ticket_id in review artifact. One accepted item remains:
dual-mechanism boundary (clean for Phase 4, revisit in Phase 5).

**Code Quality** (8/10 resolved): Helper extraction, applies_to spec,
allowed-tools scoping, golden-fixture strategy, eval severity floors
all addressed. The review-ticket duplication is an accepted tradeoff.
One minor item partially resolved (available-lens count test gap).

**Test Coverage** (both criticals resolved; 4/6 majors resolved):
mise-run-test wiring fixed, evals committed, applies_to adversarial
tests added, verdict boundary evals added, golden normalisation
documented, cross-mode fixture test specified. Remaining: orchestrator
evals not wired into CI (they're committed but run via skill-creator,
not a shell harness — consistent with how plan evals work), and
malformed-agent eval has no injection mechanism (inherent to
SKILL.md-based evals).

**Correctness** (all 9 prior findings resolved, both new minors
fixed): min_lenses default, applies_to spec, re-review fallback,
output enum, malformed-agent severity, cross-mode filtering UX —
all addressed. Validator spec wording fixed; synthetic marker added
to schema.

**Compatibility/Usability/Standards/Documentation** (20/22 resolved,
2 partially): Cross-mode filtering UX, no-args behaviour, asymmetric
default rationale, README/CHANGELOG, lens scope boundaries, preamble
path, section count, BUILTIN array consolidation, sub-phase labelling,
label-to-key mapping, usage-string test — all addressed. Two partially
resolved: allowed-tools scope (correct, consistent with review-plan)
and lens body detail (scope boundaries table added, full worked
examples deferred to skill-creator).

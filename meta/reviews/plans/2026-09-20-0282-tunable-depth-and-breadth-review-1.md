---
type: "plan-review"
id: "2026-09-20-0282-tunable-depth-and-breadth-review-1"
title: "Plan Review: Tunable Depth and Breadth Implementation Plan"
date: "2026-09-21T21:56:03+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-20-0282-tunable-depth-and-breadth"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "usability", "documentation", "performance"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-22T08:53:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Tunable Depth and Breadth Implementation Plan

**Verdict:** REVISE

The plan's mechanical core is sound and well-evidenced: Phase 1 faithfully
mirrors the `REVIEW_KEYS` precedent, every hand-enumerated wiring site is
correctly identified, and the correctness lens verified the key-count arithmetic
(63→65), the alphabetical `public-api.txt` insertion, the dump order, and the
`config get` precedence model against the actual source. What pushes it to REVISE
is not the Rust plumbing but the prompt-resolved prose that carries the feature's
behaviour: five major findings cluster on the resolution/validation surface —
a default duplicated across Rust and Markdown with nothing binding them, an
under-specified clamp warning, an internal work-item id leaking into user-facing
output, a cost guard that does not bound the point where cost is actually
incurred, and the absence of any automated regression seam for four of the nine
acceptance criteria. None is a rewrite; each is a targeted tightening of Phases 2
and 3.

### Cross-Cutting Themes

- **The default lives in two independent places with no binding** (flagged by:
  architecture, code-quality, test-coverage, correctness) — the catalogue's
  `Default::Scalar("8")` drives `config dump`/`default_for`, while the SKILL's
  `--default 8` literal drives the actual runtime resolution (because `config get
  --default` never consults the catalogue). Four lenses independently reached
  this; architecture rates it major. Nothing in the test suite catches a drift
  between them.
- **The clamp-and-warn message contract is under-specified** (flagged by:
  usability, standards, code-quality) — the depth notice ships a worked example,
  but the clamp warning is described only behaviourally, with no canonical string,
  no `Warning:` prefix, no knob name, and no "here's how to fix it". The plan's own
  Current State Analysis cites the `review.rs` house style but the Phase 2/3 edit
  prose does not carry it through.
- **Internal work-item id `0283` leaks into user-facing surfaces** (flagged by:
  usability, documentation) — both the printed `conduct` notice and the
  `configure help` depth row hardcode `0283`, which a plugin user cannot resolve.
  Both lenses rate this major.
- **Breadth is called the cost guard but does not bound peak resource use**
  (flagged by: performance, architecture) — breadth caps per-round commissioning
  only; `conduct` fans out one parallel researcher per outstanding focus area
  across all rounds, so peak concurrency is `breadth × rounds`, and a large value
  passes silently with no high-end signal.
- **Prompt-resolved logic has no automated regression net** (flagged by:
  test-coverage, code-quality) — flag precedence, clamp-and-warn, and the depth
  notice live entirely in SKILL prose; the two automated checks assert only exit-0
  and invocation form, so a later prose edit inverting the precedence or weakening
  the clamp passes CI silently.

### Tradeoff Analysis

- **Clamp-to-floor vs reject-and-default**: The work item pinned clamp-to-1 in its
  stress test, and it is a deliberate divergence from `review.rs` (which
  substitutes the key's default). For a genuinely invalid value the choice is
  defensible, but two consequences deserve a second look: clamp-to-1 on a large
  near-valid typo (`--breadth 12.5`) yields the narrowest possible round, the
  opposite of intent; and on an empty read-failure value it collapses breadth from
  8 to 1 rather than falling back to the default. Recommendation: keep clamp-to-1
  for invalid input, but exempt the empty/unparseable case to fall back to the
  knob default, and make the warning loud enough that a near-valid typo cannot pass
  unnoticed.
- **Prose resolution vs a typed `config research` consumer**: The plan chose prose
  (the knobs are prompt-resolved, so validation must sit where resolution sits),
  accepting eval-level rather than unit-level verification. This is a legitimate
  ADR-0045 call, but it is the direct cause of both the no-regression-seam and the
  duplicated-default themes. Recommendation: keep prose for this slice, but record
  the trigger at which a typed consumer becomes the maintainable path (a third
  consumer, or a rule more complex than clamp-and-warn).
- **Unbounded breadth vs a soft ceiling**: "breadth is itself the cost guard"
  controls cost rhetorically but leaves peak parallel fan-out and web-fetch load
  unbounded against a fat-fingered value. Recommendation: keep the no-hard-cap
  design but add a high-water notice, mirroring the visibility the low-end clamp
  already gives.

### Findings

#### Critical

None.

#### Major

- 🟡 **Architecture / Code Quality / Test Coverage / Correctness**: Resolved default is decoupled from the catalogue entry that appears to own it
  **Location**: Phase 1 §1 (catalogue declaration) + Phase 2 §2 (knob-resolution block)
  Because `config get --default N` deliberately skips the catalogue, the SKILL's
  `--default 8`/`--default 1` literals — not the `RESEARCH_KEYS` entries — drive
  the resolved value; the catalogue values surface only via `config dump` and
  `default_for`, which nothing in the resolution path consumes. The two copies are
  kept in sync by hand with no test binding them, so a change to the catalogue
  default silently drifts from runtime behaviour.

- 🟡 **Usability / Documentation**: Internal work-item id `0283` leaks into the user-facing notice and help
  **Location**: Phase 3 §3 (conduct depth notice) + Phase 4 §1 (research help table)
  The depth notice ("recursive deepening is unavailable until 0283") and the help
  row ("**No effect until 0283**") both surface a bare tracker id a plugin user has
  no way to resolve or track. The notice's entire purpose is to explain why
  `--depth` did nothing; anchoring it to opaque jargon defeats its actionability
  and ships into persistent docs.

- 🟡 **Usability / Standards / Code Quality**: Clamp warning contract is under-specified
  **Location**: Phase 2 §2 (knob-resolution block); Phase 3 §3
  The depth notice has a concrete worked example; the clamp warning is described
  only as "warns, naming the invalid value and stating it was clamped to 1" — no
  literal template, no `Warning:` prefix, no knob name, and no instruction on how
  to supply a valid value. The plan cites the `review.rs` house style in its own
  analysis but does not carry it into the edit prose, so the most consequential
  message in the feature is the least specified.

- 🟡 **Performance**: Breadth caps per-round commissioning but not the peak fan-out at conduct
  **Location**: Phase 2/Phase 3; "What We're NOT Doing: No breadth re-check at conduct"; Performance Considerations
  `conduct` spawns one parallel researcher per still-outstanding focus area across
  all rounds, and breadth is a per-round ceiling never re-checked at conduct, so
  peak concurrent spawns and web fetches equal the accumulated set (`breadth ×
  rounds`, or an arbitrary hand-edited `outline.md`), not `breadth`. The cost guard
  fails to bound resource use at the exact point where the spawns execute.

- 🟡 **Test Coverage / Code Quality**: Prompt-resolved validation and flag precedence have no automated regression protection
  **Location**: Implementation Approach; Testing Strategy; "What We're NOT Doing: No eval suite"
  Flag-over-config resolution, clamp-and-warn, and the depth notice live entirely
  in SKILL prose with no eval suite. `test:integration:skill-invocation` asserts
  only that the `config get` site exits 0 with non-empty stdout (it never sees the
  flag, clamp, or notice); `lint:bare-invocation:check` guards only invocation
  form. Four of the nine acceptance criteria (Flag override, Validation, Breadth
  ceiling, Depth dormant) therefore have no test that fails if the prose regresses.

#### Minor

- 🔵 **Performance / Architecture**: No upper bound and no high-end signal — a large breadth passes silently
  **Location**: "What We're NOT Doing: No upper bound"; Phase 2 §2
  Validation warns loudly on the low end (0, negative, non-integer → 1) but any
  positive integer passes with no signal. A shared-config typo such as `breadth:
  80` resolves silently to an 80-way parallel spawn — the most cost-dangerous input
  is the one case that produces no visibility, and there is no resilience failsafe
  against resource exhaustion or rate-limit cascade.

- 🔵 **Correctness**: An empty (fail-safe-degraded) resolved value clamps to 1 rather than the default
  **Location**: Phase 2 §2; Phase 3 §3
  Under `--fail-safe`, `config get` degrades a read failure to empty stdout; the
  preprocessor injects an empty string, and the validation prose treats empty as
  invalid and clamps to 1 rather than falling back to the intended default (8/1).
  The "> default" tier of the three-tier rule is unreachable exactly here, and the
  warning would name the invalid value as `''`. State that an empty/unparseable
  value falls back to the knob default.

- 🔵 **Usability / Code Quality**: Clamp-to-1 (not to default) surprises on large or near-valid input
  **Location**: Phase 2 §2; "What We're NOT Doing: No upper bound"
  An invalid value clamps to 1 "whatever its magnitude", so `--breadth 12.5` yields
  a single focus area rather than the configured value or default 8. This diverges
  from the cited `review.rs` reject-and-default idiom and gives the same codebase
  two validation philosophies for numeric tunables, raising the cognitive cost of
  the next knob.

- 🔵 **Usability**: Misplaced or malformed flag handling is unspecified
  **Location**: Phase 2 (outline `--breadth`); Phase 3 (conduct `--depth`)
  The flags are asymmetric (`--breadth` on outline only, `--depth` on conduct only)
  and the verbs have no arg parser. The plan does not say what happens for `outline
  SLUG --depth 2`, a bare `--breadth` with no value, a repeated flag, or an unknown
  flag — exactly the mistakes users are likely to make. Add a shared prose rule.

- 🔵 **Architecture / Code Quality**: Dormant-depth surface is speculative and scattered with no single toggle
  **Location**: Phases 3-4
  Depth is resolved, flag-read, clamp-validated, and notice-gated across three
  prose sites while driving no behaviour until 0283, with no single switch binding
  them. Enabling depth becomes a coordinated multi-site edit; a missed site leaves
  a stale notice or inconsistent contract.

- 🔵 **Architecture / Code Quality**: A seventh group compounds the missing-master-group-list smell
  **Location**: Phase 1 (five hand-enumerated wiring sites); Migration Notes (0280 coupling)
  Each new group family touches five independently-maintained sites in three
  orderings plus the golden, because there is no master group registry. The plan
  wires all of them correctly, but the flagged 0280 landing-order coupling is a
  direct symptom, and each future group re-incurs the manual reconciliation.
  Out-of-scope for this slice; worth capturing as tracked tech debt.

- 🔵 **Documentation**: `### research` help section is placed away from the related `### review` knobs
  **Location**: Phase 4 §1 (insertion before line 388)
  Inserting `### research` before `### paths` lands it after the intervening
  per-skill-customisation content rather than adjacent to the `### review` family
  it most resembles — both document behavioural tuning knobs. Mildly reduces
  discoverability of the related knob groups.

#### Suggestions

- 🔵 **Test Coverage**: The new parity test duplicates the catalogue unit test
  **Location**: Phase 1 §6 (`parity.rs`)
  `the_research_knobs_default_to_declared_scalars` calls `default_for` directly —
  substantively identical to the Phase 1 catalogue unit test — so it bypasses
  `parity.rs`'s actual purpose (asserting resolved values against fixtures). Either
  drop it, or make it materialise a fixture setting the keys at team and personal
  levels and assert the resolved value, giving genuine automated coverage of the
  precedence the plan otherwise verifies only by hand.

- 🔵 **Correctness**: Keep the per-round breadth invariant stated once
  **Location**: Phase 2 §3 (outline ceiling rule)
  The replacement prose preserves the per-round semantics correctly ("≤ N per
  round; may accrete across rounds") with no off-by-one relative to the original.
  This is a confirmation, not a defect — but state the invariant once and keep
  `conduct` breadth-free so a later edit cannot reintroduce a hardcoded bound.

- 🔵 **Performance**: Breadth/depth are resolved eagerly at SKILL load for every verb
  **Location**: Phase 2/3 (new `config get` preprocessor sites)
  The two `config get` calls run at load on every `research-topic` invocation,
  including `brief`, `synthesise`, and `finalise`, which never use them, because
  the `!` preprocessor cannot resolve lazily. Negligible in absolute terms; note it
  as inherent to the preprocessor model so it is not later mistaken for a fixable
  inefficiency.

- 🔵 **Usability**: Advertising a dormant `--depth` flag may set false expectations
  **Location**: Phase 3 (argument-hint)
  The argument-hint promotes `conduct SLUG [--depth N]`, but `--depth` is a no-op
  until 0283. A conscious "dormant but honest" trade-off; if a low-cost signal is
  wanted, the configure-help dormancy caveat is the right place to set
  expectations.

- 🔵 **Standards**: Phase 4 help table breaks the file's padded-column alignment
  **Location**: Phase 4 §1 (Key/Default/Description table)
  The `depth` row's Description overflows the padding used by the surrounding
  `### review`/`### paths` tables. Cosmetic (rendered markdown is unaffected);
  either pad to the longest cell or move the dormancy detail into the prose beneath
  the table.

### Strengths

- ✅ Phase 1 faithfully mirrors the `REVIEW_KEYS` precedent — same `&[(&str,
  Default)]` slice, string-scalar numeric defaults — adding no new architectural
  pattern; every hand-enumerated wiring site (declaration, `default_for` scan,
  `dump::assemble` loop, alphabetical `public-api.txt`, count-encoding test) is
  correctly identified and wired.
- ✅ The mechanical claims are verified correct against source: key-count 63→65
  across seven groups, `RESEARCH_KEYS` sorting between `PATH_KEYS` and `REVIEW_KEYS`
  with a single insertion, the dump order and golden position, and the `config get`
  personal > team > `--default` model that never consults the catalogue.
- ✅ Clean separation of the two knobs' enforcement points — breadth bounds outline
  sizing, depth bounds conduct — with an explicit guard keeping `conduct`
  breadth-free (confirmed breadth-free in the current SKILL), preserving
  single-responsibility per verb.
- ✅ A single shared "knob-resolution rule" that both verbs reference keeps the
  resolution/validation contract DRY within the prose medium and gives it one place
  to change.
- ✅ The four phases are independently mergeable with forward-only dependencies, and
  "What We're NOT Doing" applies strong YAGNI discipline (no numeric type, no
  `config research` subcommand, no recursion).
- ✅ Phase 1 is genuinely test-driven (a red-first declared-value assertion), and the
  count-encoding key-count test is a deliberate canary forcing conscious
  reconciliation on every catalogue change.
- ✅ Depth ships dormant and genuinely avoids the multiplicative spend — `conduct`
  spawns one researcher per focus area regardless of value — with a `>1` notice that
  surfaces over-ambitious intent without paying for it.
- ✅ Strong developer experience for the common case: sensible zero-config defaults
  (8/1), progressive disclosure of optional flags in the argument-hint,
  discoverability across `config dump`/`configure help`/the SKILL, and a
  consistently stated `flag > personal > team > default` order.
- ✅ Phase 4's `### research` section faithfully mirrors the `### review` help
  template (intro, table, escaped YAML example, trailing note), and all four
  AC-required documentation elements are present.
- ✅ The clamp and depth-notice boundaries match the pinned review-1 edges: a
  non-integer of magnitude ≥ 1 clamps rather than passes, and the notice fires
  strictly above 1.

### Recommended Changes

1. **Strip `0283` from user-facing strings** (addresses: Internal work-item id
   `0283` leaks) — replace with plain language ("recursive deepening is not yet
   available" / "no effect in this release") in both the Phase 3 conduct notice and
   the Phase 4 help Description cell. Keep `0283` in plan and work-item prose only.
   The work-item AC itself uses the `0283` phrasing, so confirm the reader-facing
   wording with the author.

2. **Pin the clamp-warning template and define the empty-value fallback**
   (addresses: Clamp warning under-specified; Empty resolved value clamps to 1) —
   specify an exact clamp warning in the SKILL prose with a `Warning:` prefix, the
   knob name, the single-quoted invalid value, and a fix hint (e.g. `Warning:
   research.breadth must be a positive integer, got '2.5' — clamping to 1; pass
   --breadth N or set research.breadth to an integer of 1 or more`); and state that
   an empty or unparseable resolved value falls back to the knob default (8/1), not
   to 1.

3. **Bind the two defaults or make their drift loud** (addresses: Resolved default
   decoupled from the catalogue) — add a lightweight test asserting each SKILL
   `--default N` literal equals `default_for("research.<knob>")`, or record the
   hand-sync invariant at both edit sites as a documented pair (matching the
   line-width duplication convention) so the coupling is discoverable.

4. **Close the cost-guard gap at conduct** (addresses: Breadth caps per-round not
   peak fan-out; No high-end signal) — either bound `conduct`'s peak concurrency
   independently of breadth (spawn outstanding focus areas in fixed-size waves), or
   state explicitly that breadth bounds only per-round commissioning and peak
   conduct fan-out is intentionally uncapped so the cost claim is not overstated;
   and add a soft high-water notice when a resolved breadth exceeds a sensible
   threshold.

5. **Commit to an eval seam or record the deferral** (addresses: No automated
   regression protection) — acknowledge the regression exposure explicitly and name
   where the eval-able prose gains a harness (e.g. scheduled with 0283, which
   consumes depth), even a minimal `evals/` scaffold asserting the clamp and notice,
   converting a permanent gap into a deferred one.

6. **Specify wrong-verb and malformed flag handling** (addresses: Misplaced or
   malformed flag handling unspecified) — add one shared prose rule: ignore-with-a-
   note (or refuse) a flag belonging to the other verb, and treat a
   present-but-non-numeric or absent-value flag as the invalid case that
   clamps-and-warns.

7. **Polish** (addresses: dormant-depth toggle; help placement; table alignment;
   parity test) — record the exact 0283 cleanup sites in the hand-off so the
   dormant→live flip is atomic; consider placing `### research` next to `### review`;
   fix the Phase 4 table padding; and either drop the parity test or make it assert
   resolved precedence against a fixture.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Structurally a low-risk, well-scoped change that mirrors the
established `REVIEW_KEYS` catalogue pattern faithfully (extend-not-invent), keeps
the two knobs' enforcement points cleanly separated (outline owns breadth, conduct
owns depth), and introduces a single shared knob-resolution rule both verbs
reference rather than duplicating logic. The one genuine architectural decision —
moving validation from typed Rust into SKILL prose — is explicitly acknowledged
and justified against ADR-0045/0053, a fair tradeoff rather than an oversight. The
main structural concerns are cohesion-related: the resolved default is decoupled
from the catalogue entry that appears to own it, and the dormant-depth "until
0283" contract is scattered across multiple prose sites with no binding or single
toggle.

**Strengths**:
- Extend-not-invent: `RESEARCH_KEYS` mirrors the `REVIEW_KEYS` slice pattern
  exactly, and every hand-enumerated site (default_for scan array, dump::assemble
  loop ordering, alphabetical public-api insertion, key-count test) is correctly
  identified and wired — no new architectural pattern to reason about.
- Clean separation of the two knobs' enforcement points, with an explicit guard to
  keep conduct breadth-free, preserving single-responsibility per verb and matching
  the domain vocabulary.
- A single named knob-resolution rule that both verbs invoke is DRY within the
  prose medium and gives the resolution/validation contract one place to change.
- The split between the `!` preprocessor (config I/O at load) and the verb prose
  (flag override + validation at invocation) respects the load-before-invocation
  constraint cleanly.
- Consistent `--fail-safe` graceful degradation on the new config reads, matching
  the existing preprocessor idiom.
- Phases independently mergeable with forward-only dependencies, and the divergence
  from the review.rs validation model is stated plainly rather than hidden.

**Findings**:
- 🟡 **major** (confidence: medium) — Resolved default is decoupled from the
  catalogue entry that appears to own it. **Location**: Phase 1 (catalogue
  declaration) + Phase 2/3 knob-resolution block. Because `config get --default N`
  skips the catalogue, the SKILL's `--default 8`/`--default 1` literals drive the
  resolved value; the catalogue values only surface via `config dump`/`default_for`,
  which nothing in the resolution path consumes. The same default lives in two
  independent places with no test binding them, so a maintainer changing
  `RESEARCH_KEYS` to `6` would change what `config dump` advertises while the SKILL
  silently keeps resolving `8`. Suggestion: bind the two sources with a unit test or
  a lint over the preprocessor lines.
- 🔵 **minor** (confidence: medium) — Dormant-depth "until 0283" contract is
  scattered across sites with no single toggle. **Location**: Phases 3 and 4. The
  dormancy state is expressed in three prose sites plus the spawn behaviour, with no
  single switch and the internal id `0283` baked into user-facing text. Enabling
  depth becomes a coordinated multi-site edit; a missed site leaves a stale notice.
  Suggestion: centralise the dormancy statement, record the exact 0283 cleanup
  sites, and phrase the notice without the raw work-item id.
- 🔵 **minor** (confidence: medium) — Unbounded breadth fan-out has no resilience
  failsafe. **Location**: "No upper bound" + Phase 2 outline ceiling. A resolved
  breadth of 200 fans out 200 parallel Task agents; the acknowledged tradeoff covers
  cost, not resource exhaustion or stability. Suggestion: add a soft sanity ceiling
  (warn, not clamp) or explicitly acknowledge the resource/stability implication.
- 🔵 **minor** (confidence: low) — Seventh group compounds the
  missing-master-group-list smell. **Location**: Phase 1 (five wiring sites) +
  Migration Notes (0280 coupling). Each new group multiplies the shotgun-surgery
  surface across five sites in three orderings; the 0280 landing-order coupling is a
  direct symptom. Suggestion: capture a single master group registry as tracked tech
  debt (out of scope for this slice).

### Code Quality

**Summary**: A well-scoped, low-complexity plan: the Rust side is a faithful mirror
of `REVIEW_KEYS`, the shared knob-resolution rule is stated once and referenced by
both verbs, and the four phases are independently mergeable with explicit scope
boundaries. The main code-quality tensions are all consequences of the deliberate
prose-resolution architecture: a user-facing default is duplicated across Rust and
Markdown with no sync guard, the flag-precedence and clamp-and-warn logic lives in
untestable prose, and the validation/warning semantics diverge from the `review.rs`
precedent the plan cites. Each is acknowledged and defensible; none rises above
minor for this lens.

**Strengths**:
- Phase 1 is a minimal, idiomatic mirror of `REVIEW_KEYS` reusing existing
  `defaulted_row`/`default_for` machinery — low cyclomatic and cognitive complexity.
- The resolution is factored into a single shared rule both verbs reference rather
  than restating per verb.
- The four phases are each independently mergeable and green under `mise run check`,
  with forward-only dependencies.
- "What We're NOT Doing" applies strong YAGNI discipline.
- Phase 1 is genuinely test-driven, and conduct is deliberately left breadth-free by
  not editing it.

**Findings**:
- 🔵 **minor** (confidence: medium) — Default constants duplicated across Rust
  catalogue and SKILL prose with no sync guard. **Location**: Current State + Phase 1
  §1 + Phase 2 §2. The defaults 8/1 are written twice with no automated link; the
  cited `review.rs` precedent does not have this problem because it reads the
  catalogue as a single source of truth. Suggestion: a test that greps the SKILL for
  `research.breadth --default N` and asserts N equals `default_for(...)`.
- 🔵 **minor** (confidence: medium) — Resolution and validation logic placed in prose
  with no isolation seam. **Location**: Phase 2 §2/§3 + Phase 3 §3. The behavioural
  core cannot be exercised in isolation; a future editor can silently invert the
  precedence with nothing failing. Suggestion: accept the tradeoff explicitly, and
  note that a third consumer or more complex rule makes a typed `config research`
  verb the maintainable path.
- 🔵 **minor** (confidence: low) — Two divergent validation philosophies for numeric
  config tunables. **Location**: Overview + Key Discoveries. `review.*` rejects and
  substitutes the default; `research.*` clamps to 1. A maintainer who internalised
  the review idiom will be surprised. Suggestion: state the clamp-vs-reject contrast
  as a documented decision.
- 🔵 **minor** (confidence: low) — Clamp warning and depth notice specified as loose
  prose rather than a canonical message. **Location**: Phase 2 §2 + Phase 3 §3.
  Unlike the fixed `review.rs` template, these messages have no canonical form, so
  text varies between invocations — harder to grep, harder to eval. Suggestion: pin
  an exact target string for both with a `Warning:` prefix.
- 🔵 **minor** (confidence: low) — Dormant depth threads resolution/validation/notice
  prose that drives no behaviour. **Location**: Phase 3. The "if I removed this, what
  would break?" smell, answered by "nothing until 0283"; also a mild asymmetry
  (invalid `--depth 0` warns, valid `--depth 5` notices — two paths for the same
  no-recursion outcome). Suggestion: keep the dormant knob but record 0283 as the
  owner that collapses the notice path.
- 🔵 **minor** (confidence: low) — New group compounds the pre-existing
  hand-enumerated group list. **Location**: Phase 1. Four independent edit sites plus
  the count-in-the-name test, because there is no master registry. Suggestion: flag
  the duplication as a candidate for future consolidation (a refactor here exceeds
  scope).

### Test Coverage

**Summary**: Phase 1 is genuinely test-driven with specific, mutation-resistant
coverage — a red-first declared-value unit test, a key-count canary that encodes
the numbers, an exact dump golden, and a public-api snapshot. The material weakness
is Phases 2-4: the most defect-prone logic in the whole change (flag-over-config
precedence, clamp-and-warn validation, the depth>1 notice) lives entirely in
SKILL.md prose and is deliberately given no eval suite, so four of the nine
acceptance criteria have zero automated regression protection and rely solely on
one-time manual verification. The plan is honest about this and the decision is
defensible for prompt-resolved knobs, but the riskiest behaviour is the least
protected.

**Strengths**:
- Phase 1 follows red-green-refactor with specific assertions: a declared-value unit
  test, an exact-row dump golden edit, and an alphabetical public-api snapshot line —
  a mutation would fail a named test.
- The renamed key-count test is a deliberate canary whose name encodes the numbers,
  forcing conscious reconciliation on every catalogue change (including the 0280
  collision).
- The manual verification steps for Phases 2-4 are thorough and boundary-aware: they
  enumerate the clamp edge cases (0, -2, 2.5), confirm integer≥1 passes, verify
  conduct stays breadth-free, and distinguish the depth notice at 1 vs >1.
- The plan correctly recognises "breadth not re-checked at conduct" needs no new
  code, only a guard against introducing one.

**Findings**:
- 🟡 **major** (confidence: high) — Prose-resolved validation and flag precedence have
  no automated regression protection. **Location**: Implementation Approach / Testing
  Strategy / "No eval suite". The highest-risk logic lives entirely in prose with no
  eval suite; `test:integration:skill-invocation` only asserts exit-0 with non-empty
  stdout (never sees the flag/clamp/notice) and `lint:bare-invocation:check` only
  guards invocation form. Four of nine ACs have no test that fails if the prose
  regresses — changing "clamps to 1" to "clamps to 0" would pass CI silently.
  Suggestion: commit to an eval harness (e.g. with 0283) or a minimal `evals/`
  scaffold asserting the clamp and notice.
- 🔵 **suggestion** (confidence: medium) — New parity test duplicates the catalogue
  unit test without exercising the resolution harness. **Location**: Phase 1 §6. The
  planned `parity.rs` test calls `default_for` directly, substantively identical to
  the catalogue unit test, bypassing parity's purpose (resolved values vs fixtures).
  Suggestion: drop it, or materialise a fixture setting the keys at team/personal
  levels and assert the resolved value.
- 🔵 **minor** (confidence: medium) — No test guards the hand-synced catalogue default
  against the SKILL `--default` literal. **Location**: Current State + Phase 2 §2. The
  golden pins only the catalogue side; the skill-invocation test renders whatever the
  SKILL literal says without cross-checking. Suggestion: a cheap test asserting the
  literal equals `default_for(...)`, or record the decision so the absence is
  deliberate.

### Correctness

**Summary**: The mechanical Rust plumbing in Phase 1 is logically sound: verified
the current catalogue sums to exactly 63 keys across six groups (PATH 18 + TEMPLATE
18 + WORK 4 + REVIEW 11 + AGENT 10 + VISUALISER 2), so +2 research keys yields 65
across seven groups; RESEARCH_KEYS sorts correctly between PATH_KEYS and REVIEW_KEYS
with a single insertion; and the dump.rs REVIEW→RESEARCH→AGENT loop order is
consistent with the golden insertion. The resolution model is also correct — get.rs
confirms `config get --default` never consults the catalogue, and the
preprocessor-injects-config / prose-applies-flag split is coherent. The residual
correctness concerns are all in the prompt-resolved prose: the two hand-synced 8/1
literals can silently drift, and the clamp-and-warn contract does not clearly define
behaviour for an empty (fail-safe-degraded) or non-numeric resolved value.

**Strengths**:
- Key-count arithmetic is provably correct: 18+18+4+11+10+2 == 63, so +2 gives 65
  across seven groups, matching the renamed test and its updated `use` import.
- The public-api.txt insertion is alphabetically correct (P<R, RES<REV) and needs
  only one insertion because the group constants are not re-exported at the crate
  root.
- The dump ordering is internally consistent: dump.rs emits RESEARCH_KEYS after
  REVIEW_KEYS and before AGENT_KEYS, matching the golden position.
- The resolution chain is modelled correctly: get.rs returns personal>team>`--default`
  and never consults the catalogue, with the flag layered above in prose, and the
  preprocessor timing (load before invocation) is handled correctly.
- The clamp boundary matches the pinned review-1 edge (non-integer of magnitude ≥1
  clamps rather than passes).
- The depth-notice boundary is correct: fires strictly >1, exactly 1 emits no notice,
  and the dormant one-researcher-per-focus-area behaviour is preserved.

**Findings**:
- 🔵 **minor** (confidence: medium) — The resolved default lives in two independent
  places that no test cross-checks. **Location**: Phase 2 §2 + Current State. The
  catalogue's `Default::Scalar("8")` and the SKILL's `--default 8` can silently
  diverge because `config get --default` never consults the catalogue, producing a
  state where `config dump`/`default_for` disagree with what `outline`/`conduct`
  resolve. Suggestion: note the hand-sync invariant at both edit sites and add a
  lightweight assertion.
- 🔵 **minor** (confidence: medium) — An empty (fail-safe-degraded) resolved value
  clamps to 1 rather than the default. **Location**: Phase 2 §2 + Phase 3 §3. Under
  `--fail-safe`, a read failure suppresses to empty stdout; the validation prose
  treats empty as invalid and clamps to 1, and the warning would name the value as
  `''`. Suggestion: state that an empty/unparseable value falls back to the default
  (8/1) and reword the clause to name the non-numeric/empty case.
- 🔵 **minor** (confidence: low) — The three-tier "flag > configured value > default"
  rule collapses because the injected `config get` line already encodes
  config-or-default. **Location**: Phase 2 §2. The standalone "> default" tier has no
  separate representation, so it is unreachable exactly when the injected value is
  empty. Suggestion: clarify that the injected value already encodes
  "config-or-default" and define the fallback for an empty injected value directly.
- 🔵 **suggestion** (confidence: medium) — The outline ceiling replacement preserves
  per-round semantics with no off-by-one (≤8 becomes ≤N, identical at N=8).
  **Location**: Phase 2 §3. A confirmation, not a defect — but state the invariant
  ("at most N per round, unbounded across rounds") once and keep conduct breadth-free
  so a later edit cannot reintroduce a hardcoded bound.

### Standards

**Summary**: A faithful, convention-respecting extension of established patterns: it
mirrors the REVIEW_KEYS precedent exactly, inserts RESEARCH_KEYS at the correct
alphabetical position in public-api.txt, matches the real dump.rs group order and
golden layout, renames the word-encoded key-count test correctly, follows the
`### review` template for the new `### research` help section, and uses the
`config get … --default … --fail-safe` preprocessor idiom the CLI genuinely
supports. The only standards gaps are soft: the concrete SKILL edit prose does not
carry the warning house-style it cites in its own analysis, and the Phase 4 help
table breaks the file's padded-column alignment. No critical or major convention
violations were found.

**Strengths**:
- RESEARCH_KEYS mirrors the REVIEW_KEYS shape precisely, and the group-constant name
  matches its `research.*` namespace exactly as REVIEW_KEYS↔`review.*`.
- The public-api.txt insertion is verified alphabetically correct, and the plan
  correctly notes a single insertion (constants not re-exported at crate root).
- The dump.rs loop placement and dump.golden row position both match the actual
  assemble order and committed golden.
- The key-count test rename faithfully follows the word-encoding convention, and the
  arithmetic (63→65, six→seven) is correct.
- The `### research` help section follows the `### review` template shape, reuses the
  bold `A > B > C` precedence idiom, and mirrors the in-cell dormancy annotation
  convention.
- The `!` preprocessor idiom is consistent with the file's existing use, and
  `--fail-safe` is verified a real parsed flag on `config get`.

**Findings**:
- 🔵 **minor** (confidence: medium) — Clamp-warning wording does not carry the cited
  review.rs house style into the SKILL prose. **Location**: Phase 2 §2 + Current
  State. The plan identifies the house style (single-quoted value, em-dash join,
  `Warning:` prefix, naming the key) but the edit prose only says "warns, naming the
  invalid value and stating it was clamped to 1" — not prescribing the prefix,
  quoting, knob name, or construction. Users would see two differently-shaped warning
  surfaces from the same tool. Suggestion: specify the exact template, e.g. `Warning:
  research.breadth must be a positive integer, got '{value}' — clamping to 1`.
- 🔵 **suggestion** (confidence: low) — Phase 4 help table breaks the file's
  padded-column alignment convention. **Location**: Phase 4 (Key/Default/Description
  table). The depth row's description overflows the padding used by the surrounding
  tables; cosmetic (rendered markdown unaffected). Suggestion: pad to the longest
  cell, or shorten the depth Description cell by moving the dormancy detail into the
  prose beneath.

### Usability

**Summary**: From a first-time developer's perspective the plan gets the
fundamentals right: sensible defaults (breadth 8 / depth 1) mean the feature works
with zero configuration, the flags are advertised per-verb in the argument-hint, and
the knobs are discoverable across three surfaces with a consistently stated
flag > personal > team > default order. The main DX weaknesses are in the wording and
specification of the user-facing signals: the depth notice and configure-help text
leak the internal work-item id `0283` to plugin users who cannot resolve it, and the
clamp warning contract is under-specified relative to the depth notice. Handling of
misplaced or malformed flags is also left undefined, which matters because the verbs
are prose-resolved with no argument parser and the two flags are split asymmetrically
across two verbs.

**Strengths**:
- Sensible defaults mean the feature works out of the box with zero configuration.
- Progressive disclosure is well handled: the common case stays `outline SLUG` /
  `conduct SLUG`, with the optional flags surfaced in the argument-hint where they
  apply.
- Strong discoverability — the knobs appear in `config dump`, `configure help`, the
  argument-hint, and the in-SKILL knob-resolution block.
- Resolution order is stated consistently as `flag > personal > team > default`,
  reusing the repo's bold precedence idiom.
- The depth notice is a good graceful-degradation pattern: it names the resolved
  value and states what actually happens.
- Flag naming drops the `research.` prefix (`--breadth`, not `--research-breadth`),
  consistent with the `--project`/`work.key` precedent.

**Findings**:
- 🟡 **major** (confidence: high) — Internal work-item id `0283` leaks into the
  user-facing notice and docs. **Location**: Phase 3 (notice wording) + Phase 4
  (research table). A plugin user has no access to `meta/work/` and cannot resolve
  `0283`; the notice's purpose is to explain why `--depth` did nothing, and anchoring
  it to an opaque id defeats its actionability. Suggestion: replace `0283` with plain
  language in both surfaces; keep the id in plan/work-item prose only.
- 🟡 **major** (confidence: medium) — Clamp warning is under-specified compared to the
  depth notice. **Location**: Phase 2 (knob-resolution block) + Testing Strategy. The
  depth notice ships a worked example; the clamp warning is abstract, with no literal
  example, no channel/prefix, and no instruction on how to supply a valid value.
  Suggestion: give a concrete worked example, a consistent `Warning:` prefix to
  stderr, and require it to name a valid form.
- 🔵 **minor** (confidence: medium) — Clamp-to-1 (not to default) is surprising for
  large or near-valid inputs. **Location**: Phase 2 + "No upper bound". `--breadth
  12.5` yields a single focus area rather than the configured value or default 8,
  diverging from `review.rs`. Suggestion: ensure the warning is loud and unmissable,
  or reconsider falling back to the default.
- 🔵 **minor** (confidence: medium) — Misplaced or malformed flag handling is
  unspecified. **Location**: Phase 2 + Phase 3. The plan does not say what happens for
  `outline SLUG --depth 2`, a bare `--breadth`, a repeated flag, or an unknown flag.
  Suggestion: add a short shared prose rule (ignore-with-a-note or refuse the
  wrong-verb flag; treat non-numeric/absent-value as the invalid case).
- 🔵 **suggestion** (confidence: low) — Advertising a dormant `--depth` flag may set
  false expectations. **Location**: Phase 3 argument-hint. A developer reading the
  hint expects the flag to change behaviour. A conscious "dormant but honest"
  trade-off; the configure-help caveat is the right place to set expectations.

### Documentation

**Summary**: The plan's documentation work is accurate, complete against the
acceptance criteria, and closely mirrors the existing help-block template; Phase 4
correctly documents both knobs, their defaults, the flag > personal > team > default
resolution order, the clamp-to-1 validation, and depth's dormancy caveat, all
consistent with Phases 1-3. The main audience-fit concern is that the raw internal
work-item ID `0283` leaks into user-facing surfaces (the configure-help caveat and
the printed conduct notice), which end users cannot resolve or track. A minor
discoverability nit is the placement of the new `### research` section away from the
conceptually-related `### review` knob section.

**Strengths**:
- Phase 4's `### research` section faithfully mirrors the established help-block
  template shape, matching the `### review` and `### paths` sections it sits among.
- Documentation is accurate against Phases 1-3: descriptions, defaults, resolution
  order, clamp-to-1 validation, and "no upper bound" all match the edit prose.
- All four AC-required documentation elements are present and complete.
- The argument-hint updates document the new flags at point of use, and the shared
  knob-resolution block keeps the SKILL documentation DRY.
- The plan documents its change context well for implementers (two-independent-8s
  hand-sync, the 0280 landing-order coupling, the cost rationale).

**Findings**:
- 🟡 **major** (confidence: medium) — Internal work-item ID 0283 leaks into
  user-facing help and runtime notice. **Location**: Phase 4 (depth Description cell)
  + Phase 3 (conduct notice). `configure help` and runtime notices are
  end-user-facing; the existing help block only exposes user-meaningful tokens
  (semver versions, CLI-actionable migration IDs), never a bare work-item id.
  Suggestion: phrase the caveat in user-meaningful terms in both surfaces; confirm
  the wording with the author since the AC itself uses "0283".
- 🔵 **minor** (confidence: low) — `### research` section placed away from the
  conceptually-related `### review` knobs. **Location**: Phase 4 (placement before
  line 388). Inserting before `### paths` lands it after the intervening per-skill-
  customisation section rather than adjacent to the `### review` family it most
  resembles. Suggestion: place `### research` directly after the review family.

### Performance

**Summary**: From a capacity-planning view the plan is dominated by concurrency- and
I/O-resource concerns rather than algorithmic ones: breadth sizes a parallel
researcher/Task fan-out and its web-fetch load, and the plan positions the knob as
the sole cost guard with no upper bound. The algorithmic footprint (Phase 1
catalogue plumbing, low-end clamp) is trivially O(1) and fine, and shipping depth
dormant is a genuinely sound decision that avoids the multiplicative spend until
0283. The material gap is that the stated cost guard bounds per-round commissioning
but not the peak parallel spawn/fetch fan-out at conduct, which sweeps outstanding
focus areas across all rounds and is never re-checked against breadth.

**Strengths**:
- Depth ships dormant and genuinely avoids the multiplicative spend: conduct spawns
  exactly one researcher per focus area regardless of resolved depth, so no recursive
  fan-out cost is incurred until 0283. Threading it now with only a `>1` notice
  surfaces intent without paying for it.
- Phase 1 config registration is purely additive O(1) plumbing with no hot-path,
  allocation, or data-structure concern.
- The default breadth of 8 is a sane parallel fan-out ceiling for the common case.
- The plan explicitly reasons about cost in a dedicated Performance Considerations
  section.

**Findings**:
- 🟡 **major** (confidence: medium) — Breadth caps per-round commissioning but not the
  peak parallel researcher/fetch fan-out at conduct. **Location**: Phase 2/Phase 3 +
  "No breadth re-check at conduct" + Performance Considerations. Conduct spawns one
  researcher per still-outstanding focus area across all rounds, so peak concurrent
  spawns equal the accumulated set (`breadth × rounds`, or an arbitrary hand-edited
  outline), not `breadth`. The cost guard fails to bound resource use where the
  spawns actually execute. Suggestion: bound conduct's peak concurrency in fixed-size
  waves, or state explicitly that breadth bounds only per-round commissioning so the
  cost claim is not overstated.
- 🔵 **minor** (confidence: medium) — Asymmetric guard rails: the low end warns
  loudly, an arbitrarily large breadth passes silently. **Location**: Phase 2
  knob-resolution + "No upper bound". A shared-config typo such as `breadth: 80`
  resolves silently to an 80-way parallel spawn; the most cost-dangerous input is the
  one case producing no visibility. Suggestion: add a soft high-water notice when a
  resolved breadth exceeds a sensible threshold.
- 🔵 **suggestion** (confidence: low) — Breadth/depth resolved eagerly at SKILL load
  for every verb, including those that never use them. **Location**: Phase 2/3
  preprocessor sites. The two `config get` calls run at load on `brief`,
  `synthesise`, and `finalise` too, because the `!` preprocessor cannot resolve
  lazily. Negligible; note it as inherent to the preprocessor model so it is not
  later mistaken for a fixable inefficiency.

## Re-Review (Pass 2) — 2026-09-22

**Verdict:** REVISE

The pass-1 findings are largely resolved: the plan was restructured around a new
Phase 2 that makes the catalogue the single source of truth for `config get`,
dissolving the biggest cross-cutting theme (the duplicated default), and the
`0283` leak, the under-specified clamp warning, the conduct-fan-out cost claim,
and the help-section placement are all addressed. But the new Phase 2 introduced
**four new majors**, every one clustered on the `config get` change: it is not
green as written (it breaks an existing test), its fail-safe-to-catalogue
behaviour cannot use the shared degrade path, and its "mirrors `config path`"
justification reverses a deliberate, test-pinned decision from work item 0167.
The verdict holds at REVISE, and a course decision is now worth taking — see the
Assessment.

### Previously Identified Issues

- 🟡 **Architecture**: Resolved default decoupled from the catalogue — Resolved (Phase 2 makes the catalogue the single source; the SKILL carries no literal).
- 🟡 **Usability / Documentation**: Internal id `0283` leaks into user-facing notice and help — Resolved (plain "not yet available" language; explicit no-id manual checks in Phases 4-5).
- 🟡 **Usability / Standards / Code Quality**: Clamp warning under-specified — Resolved (pinned `Warning:` template mirroring `review.rs`; one residual — the template hardcodes the knob name, see New Issues).
- 🟡 **Performance**: Breadth caps per-round, not peak conduct fan-out — Resolved as a major (now documented as intentional; a residual minor remains about a spawn-side bound).
- 🟡 **Test Coverage / Code Quality**: Prompt-resolved logic has no regression seam — Still present (the eval deferral is now recorded, but the clamp/precedence/notice coverage gap persists).
- 🔵 **Performance / Architecture**: No upper bound / large breadth passes silently — Partially resolved (documented as intentional; a high-water notice is still suggested).
- 🔵 **Correctness**: Empty fail-safe value clamps to 1 — Resolved for the unset case (catalogue fallback), with a refinement: an explicitly-set empty value still renders empty (see New Issues).
- 🔵 **Usability / Code Quality**: Clamp-to-1 surprising for near-valid input — Not re-raised (accepted; the empty sub-case is gone).
- 🔵 **Usability**: Misplaced/malformed flag handling unspecified — Partially resolved (shared rule for wrong-verb + missing value; repeated/unknown flag still open).
- 🔵 **Architecture / Code Quality**: Dormant-depth surface scattered — Partially resolved (still spread; the depth notice is now restated in three places — see New Issues).
- 🔵 **Architecture / Code Quality**: Seventh group / no master group registry — Still present (re-raised as a suggestion; out of scope, tracked tech debt).
- 🔵 **Documentation**: `### research` placed away from `### review` — Resolved (now adjacent).
- 🔵 **Test Coverage**: Parity test duplicates the catalogue test — Partially resolved (changed to a resolution test, but it cannot exercise the catalogue leg through `get` — see New Issues).
- 🔵 **Standards / Documentation**: Phase help table breaks padded-column alignment — Still present (the `### research` example still overflows a column).

### New Issues Introduced

- 🟡 **Correctness / Test Coverage**: Fail-safe change breaks an unenumerated existing test — `config get agents.reviewer --fail-safe` (`config_read.rs:424`), a catalogued key, would degrade to `accelerator:reviewer` instead of empty, failing its `is_empty()` assertion, so Phase 2 is **not green as written**.
- 🟡 **Architecture / Code Quality / Correctness**: Fail-safe-to-catalogue is new behaviour — it cannot be expressed by the shared `finish_scalar`/`Degrade` path (only `Suppress`/`Notice`), so it duplicates the Absent-branch fallback across the core and the shell; a naive implementation would ignore `--default` under a read failure.
- 🟡 **Architecture / Standards**: "Mirrors `config path`" oversells the alignment — Phase 2 reverses the deliberate, test-pinned work-item-0167 decision to keep `config get` catalogue-blind, and `get`/`path` still diverge on the empty-`--default` edge (path filters empty and folds the catalogue; get lets empty win) and on unknown keys (get accepts, path refuses); the `--default` flag vs path's positional adds a third asymmetry.
- 🟡 **Documentation**: `--fail-safe` help text left stale — Phase 2 changes fail-safe behaviour but keeps the `--fail-safe` doc comment, so `config get --help` will misdescribe the common case.
- 🔵 **Architecture**: `cli/migrate/src/ports.rs:101-104` documents a `config get <key> ""` equivalence that the grammar change invalidates — missing from the caller/doc reconciliation.
- 🔵 **Code Quality**: The pinned clamp template hardcodes `research.breadth` but is reused for depth — parameterise the knob name (`research.<knob>`).
- 🔵 **Code Quality**: The knob-resolution block conflates "misplaced flag → ignore" and "malformed value → clamp" under one "invalid case" label.
- 🔵 **Code Quality**: The depth notice is restated in three Phase 4 sites with differing example wording — a drift hazard.
- 🔵 **Correctness**: "Empty case eliminated" is imprecise — an explicitly-set empty/null value still renders empty; keep the empty/non-integer clamp branch in the SKILL prose.
- 🔵 **Correctness / Documentation**: `--level` + a miss now injects the catalogue default; the intent is unstated in the doc and plan.
- 🔵 **Usability**: `config get` behaviour is now data-dependent (catalogued key → default, uncatalogued → empty), invisible from the CLI surface.
- 🔵 **Usability / Documentation**: Three names for the built-in default ("catalogue" / "built-in" / "plugin-standard") across help surfaces.
- 🔵 **Test Coverage**: The new fail-safe test covers only 1 of 3 degradation branches, and the changed `get --help` contract is unpinned.
- 🔵 **Performance**: Peak conduct fan-out is documented but has no spawn-side bound (the Task-harness parallel limit is not noted).
- 🔵 **Standards**: `research.*` shares the word with the `paths.research_*` directory keys.
- 🔵 **Architecture** (suggestion): The Phase 2 blast radius (a breaking change to a shared command + reversing 0167) is taken on to remove one duplication the plan's own research judged acceptable; the tradeoff vs the contained `--default 8` approach is not weighed.
- 🔵 **Documentation** (suggestion): The deliberate divergence from the AC's "until 0283" wording is not reconciled in the plan.

### Assessment

The rewrite fixed the pass-1 cross-cutting themes but the new `config get` change
carries its own cluster of four majors, all real: Phase 2 is not green (it breaks
`config_read.rs:424`), the fail-safe-to-catalogue behaviour needs a bespoke path
outside the shared `Degrade` mechanism plus a shared helper to avoid drift, the
`--fail-safe` help goes stale, and "mirrors `config path`" is inaccurate because
it reverses a deliberate, reviewed 0167 decision and leaves `get`/`path`
diverging on two edges. None is unfixable — a refinement pass (add `:424` to the
rewrite list, factor one shared fallback helper, decide and document the
`get`/`path` edge truth table and the 0167 reversal, update the migrate port and
the fail-safe help, parameterise the clamp template) would close them. The
decision worth taking first: the Phase 2 blast radius — a breaking change to a
shared command, reversing a reviewed decision, a broken test, a missed cross-crate
caller — is large relative to its goal of removing one duplicated literal the
research likened to the repo's tolerated line-width duplication. The contained
alternative (keep the SKILL's `--default 8`/`--default 1` literal, add a
drift-guard test binding it to `default_for`, leave `config get` untouched)
sidesteps every new major. Verdict REVISE: choose the course (refine Phase 2, or
fall back to contained) before the next edit pass.

## Re-Review (Pass 3) — 2026-09-22

**Verdict:** COMMENT

The author chose to refine Phase 2 and clarified that `--fail-safe` need only keep
the skill loadable (not manufacture a default). That single simplification —
leaving `--fail-safe` unchanged — dissolved three of the four pass-2 majors at
their root. This pass re-ran the six lenses that carried the pass-2 majors
(architecture, code-quality, correctness, standards, documentation, test-coverage)
and **all four pass-2 majors are confirmed resolved**. The pass then surfaced
three new majors, every one a mechanical slip in the artifacts added to fix pass 2
(a wrong table row, a missing changelog, an un-flipped test assertion) — all
corrected in this pass. What remains is accepted tradeoffs and minor polish; the
plan is ready for implementation.

### Previously Identified Issues

- 🟡 **Architecture / Code Quality / Test Coverage / Correctness**: Resolved default decoupled from the catalogue — Resolved (catalogue is the single source; SKILL is literal-free).
- 🟡 **Architecture / Code Quality / Correctness**: Fail-safe-to-catalogue new behaviour / duplicated fallback / breaks `:424` — Resolved (fail-safe left unchanged; fallback confined to the core `Absent` arm; `:424` verified to stay green).
- 🟡 **Documentation**: `--fail-safe` help stale — Resolved (fail-safe behaviour unchanged, so its doc comment stays accurate).
- 🟡 **Architecture / Standards**: "Mirrors `config path`" oversells / 0167 reversal / grammar divergence — Resolved in framing (0167 reversal explicitly owned; a `get`/`path` differences table replaces the overclaim), though the table's `--level` row was itself wrong — see New Issues.
- 🔵 **Standards / Correctness**: Empty `--default` diverged from `path` — Resolved (aligned to `path`'s `filter(!is_empty)`).
- 🔵 **Code Quality**: Clamp template hardcoded / rules conflated / notice restated — Resolved (parameterised `research.<knob>`; rules split; single canonical notice referenced).
- 🔵 **Standards / Documentation**: `### research` table alignment / namespace overlap — Resolved (aligned; `paths.research_*` note added).
- 🔵 **Test Coverage**: Parity test overstated / `get --help` unpinned — Resolved (Testing Strategy scoped to `personal > team`; help assertion pinned).
- 🔵 **Test Coverage / Code Quality**: Prompt-resolved contracts have no regression seam — Still present (accepted no-eval-infra tradeoff; deferral recorded to the recursion-engine slice).
- 🔵 **Architecture / Code Quality**: No master group registry — Still present (pre-existing, out of scope, tracked).

### New Issues Introduced (all corrected in this pass)

- 🟡 **Architecture**: The `get`/`path` differences table misstated the `--level` row — verified against source, `config path --level` folds the built-in default (its `resolve_with_fallback` ignores level) while `config get --level` does not. Fixed: the table row and Migration Notes now state the genuine divergence (a deliberate choice — single-level `get` is a precise inspection).
- 🟡 **Documentation**: The breaking `config get` change had no `CHANGELOG.md` entry. Fixed: Phase 2 §6 adds a `### Changed` entry under `[Unreleased]`.
- 🟡 **Test Coverage / Correctness**: Switching `:320` to `--default` would invert its assertion (`path_with_an_empty_default_falls_through_to_the_catalogue` pins the exact divergence Phase 2 removes). Fixed: §5 now says to change the `get` assertion to `meta/plans` and retire the divergence framing, not merely swap syntax.
- 🔵 **Correctness**: The Phase 2 black-box precedence test used `research.breadth`, coupling Phase 2 to Phase 1 and contradicting "phases independent". Fixed: it now uses an already-catalogued key (`review.min_lenses`), keeping Phase 2 self-contained.
- 🔵 **Standards / Documentation**: Sibling `config path`/`config work` doc comments still said "plugin-standard"/"catalogue default". Fixed: Phase 2 §1 reconciles all three `ConfigAction` doc comments to "built-in default".
- 🔵 **Documentation**: `config dump` discovery pointer imprecise, and `get.rs` states "never the catalogue default" in two places. Fixed: pointer tightened to "shown with a value under the `default` source"; §2 now updates both doc sites.
- 🔵 **Test Coverage**: Missing `--level` + `--default` and uncatalogued-empty-default assertions. Fixed: both added to §5.
- 🔵 **Documentation**: Phase 3 interim heading over-claimed depth as "resolved" while hardcoded. Fixed: the interim heading scopes "resolved" to breadth; Phase 4 broadens it.
- 🔵 **Correctness**: Empty-value rule assumed empty was reachable only on a read failure. Fixed: the parenthetical now covers an explicitly-empty value too.
- 🔵 **Architecture / Code Quality**: Dormant-depth flip is a multi-site prose edit with no toggle. Fixed: the exact flip sites are recorded in the "No recursion" hand-off note.
- 🔵 **Standards**: Flag-vs-positional grammar difference lacked a rationale. Fixed: the Phase 2 table now explains it (clean bare read, avoid a second positional) and records `path` migration as a possible follow-up.

### Assessment

The plan is ready for implementation. All four pass-2 majors are resolved at their
root by leaving `--fail-safe` unchanged and confining the catalogue fallback to
the core resolution, and the three mechanical majors this pass surfaced — the
`--level` table row, the missing changelog, and the `:320` assertion — are
corrected, along with every actionable minor. Verdict moves from REVISE to
COMMENT: no criticals, no unaddressed majors. The only standing items are
explicitly accepted tradeoffs — the prose-resolution regression-seam gap (deferred
to the recursion-engine slice's eval harness), the pre-existing no-master-group
registry, and the documented `get`/`path` grammar divergence — none of which
blocks implementation.

## Approval — 2026-09-22

**Verdict:** APPROVE

Author accepted the Pass 3 recommendation and approved the plan for
implementation. The standing items (the prose-resolution regression-seam gap, the
no-master-group registry, and the documented `get`/`path` grammar divergence)
remain accepted tradeoffs, not blockers.

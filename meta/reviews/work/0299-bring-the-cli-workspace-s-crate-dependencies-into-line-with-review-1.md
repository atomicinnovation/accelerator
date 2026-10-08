---
type: "work-item-review"
id: "0299-bring-the-cli-workspace-s-crate-dependencies-into-line-with-review-1"
title: "Work Item Review: Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069"
date: "2026-10-05T21:12:24+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0299"
work_item_id: "0299"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: []
last_updated: "2026-10-06T00:19:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069

**Verdict:** REVISE

The work item is precise and well grounded. The violation table traces every
edge to an ADR-0069 rule, removals are named down to file and symbol, and the
lint's acceptance criteria are concrete fixture-driven checks. It needs
revision for three reasons. AC6 contradicts the shared-context exception the
item relies on. Behaviour preservation across the port-injection and
catalogue refactors has no criterion beyond the config dump. The launcher's
new MPL-2.0 obligation from `uluru` is not captured. Separately, the scope
lens judges the item to be epic-sized under a `task` kind; the author has
already acknowledged this bundling in the Drafting Notes.

### Cross-Cutting Themes

- **Launcher licence obligation** (flagged by: completeness, dependency) — ADR-0069 says the in-process launcher carries `uluru`'s MPL-2.0 notice obligation; no requirement updates the launcher's licence-audit or notice registration.
- **Requirements with no acceptance criterion** (flagged by: completeness, testability) — Several requirement clauses could be skipped while every criterion still passes: the dead `work` → `config` edge and its `pup.ron` permission, `TEMP_PREFIX` → `kernel`, the `tasks/README.md` checklist step, build-dependency coverage, the `verify` exemption, and shared-context downstream declarations.
- **Summary understates scope** (flagged by: completeness, scope, clarity) — The Summary omits the catalogue block work (called "the largest piece"), the `migrate` ports and the domain → domain review, and it says every adapter edge becomes a port, although the file lock moves to `store` without one.
- **Measurement method undefined** (flagged by: clarity, testability) — "Cold", "warm" and "warm-dispatch" have no defined meaning, and the run count and statistic behind the 10× gate are unspecified, so before/after results may not be comparable.
- **Unbounded rule-3 review** (flagged by: scope, testability, clarity) — Requirement 9's "justified or removed" makes scope depend on a mid-implementation judgement. It can be satisfied by any written justification, and it conflicts with Requirement 8's reliance on `migrate` → `corpus`.

### Findings

#### Major

- 🟡 **Testability**: AC6's adapter-to-adapter check contradicts the shared-context exception
  **Location**: Acceptance Criteria
  "No adapter crate depends on another context's adapter crate" fails against `jira-client`/`linear-client` → `tracker-support`, which the Assumptions say rule 4's shared-context exception allows.
- 🟡 **Testability**: Preserved behaviour across the port-injection refactor has no criterion
  **Location**: Acceptance Criteria
  Only AC9 checks preserved behaviour. Consent tracking at the 7 non-launcher roots, `m0001`/`m0008` rendering, sync-baseline refresh and the derived `tracker-support` parsers rely on whatever the existing suite happens to cover.
- 🟡 **Dependency** + **Completeness**: Launcher's MPL-2.0 licence/notice obligation from `uluru` not captured
  **Location**: Requirements
  ADR-0069's Negative consequences say this obligation arrives with in-process tracking. Requirement 10 covers only the `deny.toml` symbol table.
- 🟡 **Clarity**: Requirement 8 depends on the `migrate` → `corpus` edge that Requirement 9 allows to be removed
  **Location**: Requirements
  Parsing through the `corpus` frontmatter port requires the edge Requirement 9 leaves open. It also sits awkwardly with rule 1's "its own ports".
- 🟡 **Scope**: Epic-scale refactor declared as a single task
  **Location**: Frontmatter: kind
  The item touches eight contexts, the launcher and the Python build system, and contains at least six independently deliverable strands. The Drafting Notes acknowledge the bundling.

#### Minor

- 🔵 **Completeness** + **Testability**: Several requirements have no acceptance criterion
  **Location**: Acceptance Criteria
  Uncovered: the dead `work` → `config` edge and its `pup.ron` permission, the launcher dropping `store`, the README checklist step, and shared-context downstream declarations in AC4.
- 🔵 **Testability**: Build-dependency coverage, dev-dependency handling and the `verify` exemption are untested
  **Location**: Requirements
  A lint that only inspects normal dependencies, or that does not exempt `verify`, meets every criterion.
- 🔵 **Testability** + **Clarity**: Before/after measurement method and cold/warm conditions are unspecified
  **Location**: Acceptance Criteria
  The item fixes neither the method nor the statistic, and "cold" can mean an empty launcher cache, a cold OS cache, or the first repository open.
- 🔵 **Testability**: Rule-3 justification criterion can always be claimed as met
  **Location**: Acceptance Criteria
  Any written justification satisfies AC12. Requiring it to name the upstream types the downstream model is expressed in would make it checkable.
- 🔵 **Testability**: AC7's "in any state" is unbounded
  **Location**: Acceptance Criteria
  The item does not list which states must be exercised.
- 🔵 **Dependency**: Ordering constraints inside the unsplit item are not stated
  **Location**: Requirements
  The baseline measurements must come first. The lint can only be wired into `check` after every remediation lands. The catalogue must be built before `tracker-support` derives its parsers from it.
- 🔵 **Dependency**: Launcher/sub-binary version coupling for removing `vcs tracking` not stated
  **Location**: Requirements
  If launcher and `vcs` versions can skew, an old launcher calling `vcs tracking` against a new `vcs` binary hits the removed subcommand.
- 🔵 **Scope**: "No exception list" forces all fixes to land atomically with the lint
  **Location**: Requirements
  A ratcheting allow-list would let the lint land first and protect the workspace during the refactor.
- 🔵 **Scope**: User-visible behaviour removal bundled into an architectural refactor
  **Location**: Requirements
  Requirement 5 can't be rolled back independently of the pure refactors.
- 🔵 **Scope**: Non-violation changes included beyond the stated purpose
  **Location**: Requirements
  Moving `TEMP_PREFIX` and deduplicating the `dump.rs` field lists widen the boundary.
- 🔵 **Scope**: Domain-to-domain review has an unbounded possible outcome
  **Location**: Requirements
  Removing an edge could mean substantial model translation with no size estimate.
- 🔵 **Completeness** + **Scope** + **Clarity**: Summary understates scope and says every edge becomes a port
  **Location**: Summary
  It omits the catalogue work, the `migrate` ports and the rule-3 review, and it contradicts the file-lock move to `store`, which needs no port.
- 🔵 **Clarity**: "One forbidden dependency per rule" does not match the cases listed or the ADR's numbered rules
  **Location**: Acceptance Criteria
  Two of the listed cases fall under rule 1, the test-support case matches no numbered rule, and "two platform contexts depending on each other" is undefined.
- 🔵 **Clarity**: "The plan" could mean plan 0226 or this item's future plan
  **Location**: Acceptance Criteria
  Decisions should be recorded in 0299's implementation plan, not appended to 0226.
- 🔵 **Clarity**: Where product contexts with no domain crate declare their kind is unclear
  **Location**: Requirements
  `jira` and `linear` may be adapter-only contexts.
- 🔵 **Clarity**: The separate `launcher` role leaves rule 5's composition-root check unclear
  **Location**: Requirements
  The item should state that rule 5 still applies to the `launcher` role, in both directions.

#### Suggestions

- 🔵 **Completeness**: No Open Questions section despite several deferred decisions
  **Location**: Open Questions
  The rule-3 judgements, the two stop-for-decision gates and the split decision are spread across sections.
- 🔵 **Dependency**: Concurrent work touching the same adapter crates not acknowledged
  **Location**: Dependencies
  0300 (sync pulls) and others may touch `work-adapters` and the tracker clients.
- 🔵 **Clarity**: Several terms are overloaded or used before they are defined
  **Location**: Requirements
  "The consent adapters", "each composition root", `config` (context, crate or subcommand), and `uluru` with no rationale given.

### Strengths

- ✅ The violation table names each edge with the rule it breaks and what it is used for, and matches ADR-0069's count of 10 adapter → adapter edges across 7 crates.
- ✅ The Requirements say what the work is: the ports to introduce, crates to merge, and symbols and subcommands to remove.
- ✅ The lint criteria are fixture-driven and cover both directions: rejections per rule, exemptions, shared-context downstreams, and malformed declarations.
- ✅ The Technical Notes give a landing place for every `corpus-adapters`/`config-adapters` symbol, the 8 tracking-port call sites, and every registration to remove.
- ✅ Deviations from the ADR are stated and justified: the separate `launcher` role, rule 3 left to review, and `TEMP_PREFIX` as a simplification.
- ✅ The 10× and visualiser-linking stop-gates put hard limits on the riskiest outcomes.
- ✅ The Technical Notes already mark the seams along which the item could be split.

### Recommended Changes

1. **Reword AC6 to honour the shared-context exception** (addresses: AC6 contradiction)
   Either add "except a declared downstream's adapter depending on the shared context's adapter", or list the allowed cross-context adapter edges explicitly.
2. **Add behaviour-preservation criteria at the riskiest seams** (addresses: preserved behaviour has no criterion)
   Cover `jira-cli`/`linear-cli` block parsing (same results and errors), byte-identical `m0001`/`m0008` output on a fixture corpus, and a non-launcher root still reporting a tracked `config.local.md` in git and jj.
3. **Capture the launcher's licence obligation** (addresses: MPL-2.0 obligation)
   Add a requirement and criterion that the launcher's licence-audit and notice registrations cover the `gix`/`jj-lib`/`uluru` closure.
4. **Settle `migrate` → `corpus`** (addresses: Requirement 8 vs 9)
   Drop it from Requirement 9 as settled by Requirement 8, and state how parsing through `corpus`'s port satisfies rule 1.
5. **Decide the item's shape** (addresses: epic-scale task, atomic landing, bundled behaviour removal, unbounded rule-3 review)
   Either re-kind it as an epic and split it along the Technical Notes' seams, or keep it as one item and record the rationale. Either way, limit Requirement 9 to recording judgements and spin removals out as follow-ups.
6. **Close the criterion gaps** (addresses: uncovered requirements, build/dev/`verify` coverage, AC7 unbounded)
   Add `cargo metadata` checks for `work` ↛ `config` and launcher ↛ `store`, a `pup.ron` check, a README checklist check, a build-dependency fixture, dev-dependency and `verify` exemption fixtures, a shared context with no downstreams in AC4, and an enumerated state list for AC7.
7. **Define the measurement method** (addresses: method unspecified, "cold" ambiguous)
   Name the tool, warm-up and run counts, what cold and warm mean, and that medians are compared.
8. **Tighten wording** (addresses: Summary, "the plan", rule mapping, launcher role, adapter-only contexts, overloaded terms)
   Bring the Summary in line with Requirement 7 and the full scope, say "0299's implementation plan", map each fixture case to a rule, state that `launcher` is a composition root for rule 5, and say where adapter-only contexts declare their kind.
9. **Add sequencing and an Open Questions section** (addresses: ordering constraints, no Open Questions, version skew, concurrent work)
   Record the order: baseline measurements, declarations, the lint, the refactors, then wiring the lint into `check`. Add an Assumption about launcher/sub-binary version pinning and a note on overlap with 0300.

## Per-Lens Results

### Clarity

**Summary**: The work item is precise and well grounded, and uses the ADR vocabulary consistently. The clarity problems are internal tensions and overloaded referents. Requirement 8 relies on the `migrate` → `corpus` edge that Requirement 9 allows to be removed. "The plan" and "cold" each have two meanings. "One forbidden dependency per rule" does not map onto the eight rules.

**Strengths**:
- The violation table traces every claim in Requirements 4–8 back to a concrete entry.
- The ADR-0069 vocabulary is used consistently, with the ADR linked as the definition.
- Deviations from the ADR are stated and justified.
- The removals are listed down to file paths and symbols.

**Findings**:
- 🟡 major / medium — Requirements — Requirement 8 depends on the `migrate` → `corpus` edge that Requirement 9 allows to be removed. Parsing through the `corpus` port requires the edge; rule 1 says "one of its own ports". Suggest settling the edge in Requirement 8 and dropping it from Requirement 9.
- 🔵 minor / high — Acceptance Criteria — "One forbidden dependency per rule" does not match the cases. Two of the eight cases fall under rule 1, the test-support case matches no numbered rule, and "two platform contexts depending on each other" is undefined. Suggest mapping each fixture case to a rule and defining a context-level cycle.
- 🔵 minor / high — Acceptance Criteria — "The plan" could mean plan 0226 or 0299's future plan.
- 🔵 minor / medium — Requirements — "Cold" SessionStart latency has two possible meanings, and "warm-dispatch" is undefined.
- 🔵 minor / medium — Requirements — It is unclear where product contexts without a domain crate (`jira`, `linear`) declare their kind.
- 🔵 minor / medium — Requirements — The separate `launcher` role leaves it unclear whether rule 5 still applies to the launcher.
- 🔵 minor / high — Summary — The Summary says every adapter → adapter edge becomes a port, but Requirement 7 and the file-lock note allow moving the code to a technical library instead.
- 🔵 suggestion / medium — Requirements — Several terms are overloaded: "the consent adapters", "each composition root", `config`, `uluru`.

### Completeness

**Summary**: The work item is thoroughly specified for a task, with a motivating Context, a violation table, ten requirements, thirteen Given/When/Then criteria, substantive Assumptions and detailed Technical Notes. The gaps are small. Some requirements have no matching criterion, the Summary omits the largest strand, the ADR's licence obligation is missing, and there is no Open Questions section.

**Strengths**:
- The Context explains the motivation and lists every violation with its rule, matching the ADR's count.
- The Requirements say what the work is, not just the outcome.
- The Acceptance Criteria cover the lint's positive, negative and schema-error cases, plus behaviour preservation and the `mise run` criterion.
- The Assumptions classify contexts and justify leaving rule 3 to review.
- The frontmatter is complete and valid.
- The symbol landing table and the Removals note are exhaustive.

**Findings**:
- 🔵 minor / high — Acceptance Criteria — Several requirements have no acceptance criterion: `work` → `config` and `pup.ron`, `TEMP_PREFIX` → `kernel`, and the README checklist step.
- 🔵 minor / medium — Summary — The Summary omits what the Technical Notes call the largest piece of work, the catalogue blocks.
- 🔵 minor / medium — Requirements — The launcher's MPL-2.0 licence-notice obligation from ADR-0069 is not captured.
- 🔵 suggestion / medium — Open Questions — There is no Open Questions section despite several deferred decisions.

### Dependency

**Summary**: The work item maps its internal couplings well, and "Blocked by: none" holds because ADR-0069 is accepted and plan 0226 is done. The gaps are at the edges: the launcher's licence obligation, ordering inside the unsplit item, and concurrent work on the same crates.

**Strengths**:
- The Dependencies section explains why nothing blocks the work.
- The violation table names every edge being cut.
- The Technical Notes list the 8 call sites and every tooling registration to remove.
- The item cites the existing `cargo metadata` lint as precedent and notes the downstream README checklist effect.

**Findings**:
- 🟡 major / medium — Requirements — The launcher's licence/notice obligation from `uluru` (MPL-2.0) is not captured. Suggest a requirement and criterion for the launcher's licence-audit and notice registrations.
- 🔵 minor / medium — Requirements — Ordering constraints inside the unsplit item are not stated: baseline first, declarations before the lint, wiring into `check` last, and the catalogue before the `tracker-support` derivation.
- 🔵 minor / low — Requirements — Launcher/sub-binary version coupling for removing `vcs tracking` is not stated.
- 🔵 suggestion / low — Dependencies — Concurrent work touching the same adapter crates (e.g. 0300) is not acknowledged.

### Scope

**Summary**: Every requirement serves one goal, but the item bundles at least six independently deliverable refactors under a `task` kind. The "no exception list" criterion forces them all to land together. The author acknowledges this, and the seams for splitting are already documented.

**Strengths**:
- The unifying purpose is crisp and traceable to ADR-0069 rules.
- Boundaries are explicit: preserved behaviour, with two named removals.
- The Technical Notes already identify natural decomposition seams.
- The measurement stop-gates bound the largest risk.
- The bundling decision is visible in the Drafting Notes.

**Findings**:
- 🟡 major / medium — Frontmatter: kind — This is an epic-scale refactor declared as a single task. Suggest re-kinding it as an epic and splitting it into (a) the lint and declarations, (b) the tracking port, consent merge and in-process launcher, (c) catalogue blocks, (d) the repository-facts port, (e) the corpus symbol landings and the lock move to `store`, (f) the `migrate` ports, sync baselines and `work` → `config`, and (g) the domain → domain review.
- 🔵 minor / medium — Requirements — "No exception list" forces all fixes to land atomically with the lint. Suggest a ratcheting allow-list.
- 🔵 minor / medium — Requirements — A user-visible behaviour removal is bundled into an architectural refactor.
- 🔵 minor / high — Requirements — Non-violation changes are included (`TEMP_PREFIX`, `dump.rs` deduplication).
- 🔵 minor / medium — Requirements — The domain-to-domain review has an unbounded possible outcome. Suggest limiting it to recording judgements.
- 🔵 suggestion / medium — Summary — The Summary understates the scope described in the Requirements.

### Testability

**Summary**: Most criteria are fixture-driven Given/When/Then checks, so the lint can be tested well. The weak spots are AC6 contradicting the shared-context exception, behaviour preservation checked only by AC9, several requirement clauses without criteria, an unfixed measurement method, and a rule-3 criterion that can always be claimed as met.

**Strengths**:
- The lint criteria use concrete per-rule fixtures that name the crate, the dependency and the rule.
- AC2 and AC3 check that the lint allows what it should, not only that it rejects violations.
- AC4 checks malformed or missing declarations.
- AC6 states the launcher's allowed dependency set as a closed list.
- AC7 gives an exact precondition for the cold-session case.
- The stop conditions are clear and auditable.

**Findings**:
- 🟡 major / high — Acceptance Criteria — AC6's adapter-to-adapter check contradicts the shared-context exception (`jira-client`/`linear-client` → `tracker-support`).
- 🟡 major / medium — Acceptance Criteria — Preserved behaviour across the port-injection refactor has no criterion. Suggest criteria for `jira-cli`/`linear-cli` block parsing, byte-identical `m0001`/`m0008` output, and a non-launcher root reporting tracked files.
- 🔵 minor / high — Requirements — Build-dependency coverage, dev-dependency handling and the `verify` exemption are untested.
- 🔵 minor / high — Requirements — Removal of the dead `work` → `config` edge cannot be verified through the lint.
- 🔵 minor / medium — Acceptance Criteria — The before/after measurement method and the cold/warm conditions are unspecified.
- 🔵 minor / medium — Acceptance Criteria — The rule-3 justification criterion can always be claimed as met. Suggest requiring each justification to name the upstream types used in the downstream model.
- 🔵 minor / medium — Acceptance Criteria — AC7's "in any state" is unbounded, and the shared-context downstream declaration and the README checklist step have no criterion.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-05T21:22:30+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: AC6 contradicts the shared-context exception — Resolved
- 🟡 **Testability**: Preserved behaviour has no criterion — Partially resolved (criteria added, but baselines aren't captured first and 6 of 8 tracking sites remain unchecked)
- 🟡 **Dependency** + **Completeness**: Launcher's MPL-2.0 obligation not captured — Resolved
- 🟡 **Clarity**: Requirement 8 vs Requirement 9 on `migrate` → `corpus` — Resolved
- 🟡 **Scope**: Epic-scale refactor declared as a task — Still present (the author has accepted it, as recorded in the Drafting Notes)
- 🔵 **Completeness** + **Testability**: Requirements with no criterion — Resolved
- 🔵 **Testability**: Build/dev/`verify` coverage untested — Resolved
- 🔵 **Testability** + **Clarity**: Measurement method unspecified — Resolved
- 🔵 **Testability**: Rule-3 justification always satisfiable — Still present (the author chose to keep it)
- 🔵 **Testability**: AC7 "in any state" unbounded — Resolved
- 🔵 **Dependency**: Ordering constraints unstated — Resolved
- 🔵 **Dependency**: Launcher/sub-binary version coupling — Resolved
- 🔵 **Scope**: No exception list forces atomic landing — Still present (accepted)
- 🔵 **Scope**: Behaviour removal bundled with refactor — Still present (accepted)
- 🔵 **Scope**: Non-violation changes in scope — Still present
- 🔵 **Scope**: Unbounded rule-3 outcome — Still present (accepted)
- 🔵 **Completeness** + **Scope** + **Clarity**: Summary understates scope — Resolved
- 🔵 **Clarity**: Fixture cases do not map to rules — Partially resolved (the product-context fixture is still too broad)
- 🔵 **Clarity**: "The plan" ambiguous — Resolved
- 🔵 **Clarity**: Kind for adapter-only contexts — Resolved
- 🔵 **Clarity**: Launcher role under rule 5 — Partially resolved ("in both directions" is cryptic, and there is no launcher → root fixture)
- 🔵 **Completeness**: No Open Questions section — Resolved
- 🔵 **Dependency**: Concurrent work not acknowledged — Partially resolved (the effect of wiring the lint on in-flight crate work is not noted)
- 🔵 **Clarity**: Overloaded terms — Partially resolved ("call sites" vs "roots", unnamed "consumers")

### New Issues Introduced

- 🟡 **Dependency** + **Testability** + **Scope**: Warm-dispatch "after" figure depends on a signed release — Dependencies says "Blocked by: none", and the 10× gate may only fire after the change has shipped.
- 🟡 **Testability**: Pre-change baselines not captured before the code is removed — The `work-cli`, config dump and tracker-parse criteria compare against behaviour the refactor deletes. Golden characterisation tests should come first.
- 🟡 **Testability**: SessionStart criterion lacks a negative assertion — An implementation that always warns would pass. Untracked and outside-repository files must produce no warning.
- 🟡 **Scope**: Launcher stop gates would halt unrelated refactors — The gates apply only to the launcher, but a trip pauses the whole item.
- 🔵 **Clarity** + **Testability**: Binary size falls outside the 10× gate as worded — The AC says "any median".
- 🔵 **Clarity**: "Each adapter → adapter dependency" is broader than intended — Qualify it to cross-context edges forbidden by rule 4.
- 🔵 **Clarity**: The `Unchecked` state is not named as a specific symbol.
- 🔵 **Clarity**: The test-support and declaration-validity checks are not named in Requirement 2.
- 🔵 **Clarity**: It is unclear whether `migrate` → `corpus` exists today or is new.
- 🔵 **Testability**: No proof that the lint is wired into `mise run check`.
- 🔵 **Testability**: Catalogue-derived parsers are verified only by output equivalence — No check shows a single source of truth.
- 🔵 **Testability**: Missing fixtures for conflicting kind declarations and unknown role values.
- 🔵 **Completeness**: The manifest declaration format (table, keys, values) is unspecified.
- 🔵 **Dependency**: Wiring the lint into `check` affects in-flight crate work.
- 🔵 Suggestions: Open Questions mixes settled and open items; no decision-maker is named for the stop gates; the migration fixture corpus contents are unspecified; several named removals have no criterion; `vcs tracking` consumers other than the launcher are unconfirmed.

### Assessment

Pass 1's core defects are fixed. AC6, the licence obligation, the Requirement 8/9 conflict, the measurement method and most criterion gaps are resolved. Three new majors are about verification mechanics: characterisation baselines must be captured before the refactor, the SessionStart criterion needs a negative assertion, and the warm-dispatch release dependency is unresolved. Each can be fixed with one or two sentences. The two scope majors follow from the author's decision to keep this as one task. If that decision stands, the verdict stays REVISE on the count rule. Once the three verification majors are fixed, the item is ready for planning in substance.

## Re-Review (Pass 3) — 2026-10-05T22:07:03+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency** + **Testability** + **Scope**: Warm-dispatch "after" figure depends on a signed release — Resolved (it is now a "Requires before merge" entry)
- 🟡 **Testability**: Pre-change baselines not captured — Resolved for the tracked paths (now extended, see new issues)
- 🟡 **Testability**: SessionStart criterion lacks a negative assertion — Resolved
- 🟡 **Scope**: Launcher stop gates halt unrelated refactors — Not re-raised as major (it now appears as a dependency suggestion about author-decision gates)
- 🟡 **Scope**: Epic-scale refactor declared as a task — Still present (accepted by the author)
- 🔵 **Clarity** + **Testability**: Binary size outside the 10× gate — Resolved
- 🔵 **Clarity**: "Each adapter → adapter" too broad — Resolved
- 🔵 **Clarity**: `Unchecked` referent — Resolved
- 🔵 **Clarity**: Test-support and declaration checks unnamed — Resolved
- 🔵 **Clarity**: `migrate` → `corpus` existing or new — Resolved
- 🔵 **Testability**: Lint wiring proof — Resolved
- 🔵 **Testability**: Derived parsers single source — Partially resolved ("recognise" has no defined outcome)
- 🔵 **Testability**: Conflicting-declaration fixtures — Partially resolved (more declaration cases are still missing)
- 🔵 **Completeness**: Declaration format unspecified — Resolved
- 🔵 **Dependency**: In-flight crate work — Partially resolved (crates being restructured are not covered)
- 🔵 **Scope**: `TEMP_PREFIX` beyond purpose — Still present
- 🔵 **Scope**: Rule-3 outcome open-ended — Still present (accepted; completeness wants it in Open Questions)

### New Issues Introduced

- 🟡 **Testability**: No behaviour-preservation check for the remaining adapter refactors — repository facts (metadata, working-copy status, author), frontmatter validation, `research-adapters` store injection, the lock move, and the other `migrate-adapters` ports.
- 🟡 **Testability**: Lint verdict for `kernel` and technical-library dependencies undefined — ADR-0069 does not constrain them, and no fixture covers them.
- 🔵 **Clarity**: The Summary still says `migrate` gets a frontmatter-parsing port, which contradicts Requirement 8.
- 🔵 **Clarity**: Requirement 2's "every dependency ADR-0069 forbids" conflicts with rule 3 being enforced by review.
- 🔵 **Clarity**: Edges into the bootstrap verifier (`launcher` → `verify`) are not classified, so rule 6 would flag them.
- 🔵 **Testability**: Characterisation tests "pass unchanged" across an API reshape — the level they run at is not stated.
- 🔵 **Testability**: Missing fixtures — the launcher depending on a product context or on `tracker-support`, an adapter depending on a root, a test-support build dependency, and unknown kind, misplaced or unknown downstreams, or partial kind declarations.
- 🔵 **Testability**: The licence criterion only checks the positive branch.
- 🔵 **Dependency**: Port-before-consumer ordering inside the refactor phase is not stated.
- 🔵 **Dependency**: The signed-prerelease mechanism and who can trigger it are not named.
- 🔵 **Scope**: Catalogue structured blocks are a separable feature, and the Summary omits the subcommand removal and the binary-wide re-measurement.
- 🔵 Suggestions: `relates_to` lacks `adr:ADR-0069`; the "10×" ratio wording; rule 1 is cited for `work` → `config`; 0226 terms are undefined; the launcher repository-roots path is not characterised; author-decision gates are absent from Dependencies.

### Assessment

All three verification majors from pass 2 are resolved, and so are most minors. Two new testability majors remain. Behaviour preservation is pinned down for only some of the refactored paths, and the lint's verdict on `kernel` and technical-library edges is undefined, an ADR-0069 gap that 0299 has to settle. With those two plus the author-accepted scope major, the verdict stays REVISE. Each testability major can be fixed with a criterion and a sentence. After that, only the accepted scope major would hold the verdict at REVISE.

## Re-Review (Pass 4) — 2026-10-05T22:52:48+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: No behaviour-preservation check for the remaining adapter refactors — Resolved (binary-level characterisation tests now cover them)
- 🟡 **Testability**: Lint verdict for `kernel` and technical-library edges undefined — Resolved
- 🟡 **Scope**: Epic-scale refactor declared as a task — Downgraded to minor (the lens now accepts the Drafting Notes rationale)
- 🔵 **Clarity**: Summary contradicts Requirement 8 — Resolved
- 🔵 **Clarity**: Requirement 2 vs rule 3 — Resolved
- 🔵 **Clarity**: Edges into the bootstrap verifier — Partially resolved (the launcher allow-list criterion now contradicts it; see new issues)
- 🔵 **Testability**: Characterisation test level — Resolved
- 🔵 **Testability**: Missing fixtures — Partially resolved (shared-context domain edges and allowed-edge fixtures are still missing)
- 🔵 **Testability**: "Recognise" undefined — Resolved (`push` is not exercised)
- 🔵 **Testability**: Licence criterion positive-only — Resolved
- 🔵 **Dependency**: Ordering inside the refactor phase — Resolved
- 🔵 **Dependency**: Prerelease mechanism unnamed — Partially resolved (the key holder and release host are not named)
- 🔵 **Dependency**: Coordination covers restructured crates — Partially resolved (no specific work items are named)
- 🔵 **Scope**: Rule-3 outcome open-ended — Still present (now also raised by completeness, dependency and testability)
- 🔵 **Scope**: `TEMP_PREFIX` move — Still present (suggestion)

### New Issues Introduced

- 🟡 **Clarity**: The launcher allow-list in the `cargo metadata` criterion omits the bootstrap verifier, which Requirement 2 and the not-reported fixture allow.
- 🟡 **Dependency** + **Testability** + **Completeness**: No contingency if `measure:warm-dispatch` rejects an unattested local prerelease. A check of `tasks/measure.py` during this pass found no SLSA requirement: the harness needs only a minisign-signed release published for the tree's version (`tasks/measure.py:1352-1363`). That settles the Open Question, but the work item does not record it yet.
- 🟡 **Testability**: No criterion checks that each real crate's declared role, context and kind are correct. A violation could be hidden by misclassifying a crate.
- 🔵 **Clarity**: Unclear referents — which `vcs` port and what replaces `InProcessProbe`; "them" and "merging of the config root" in the Tracking port note; inconsistently rooted removal paths.
- 🔵 **Testability**: Before and after measurements are not required to be on the same host.
- 🔵 **Dependency**: Sequencing omits the closing steps: re-measure, rewrite `deny.toml` and notices, cut the prerelease, take the after figure.
- 🔵 Suggestions: SLSA, "consent-gated", "landing-table" and the shape of the `MigrationContext` port are undefined; the Summary does not mention that the lint covers normal and build edges only.

### Assessment

Pass 3's two testability majors are resolved, and the scope lens no longer counts the bundling as major. Three new majors remain, and each is a small edit: add the verifier to the launcher allow-list; record that the harness checks minisign, not SLSA (which closes the prerelease contingency); and add a table of expected declarations that the manifests must match. The rule-3 removal branch is now flagged by four lenses as minor. Limiting this item to justification and spinning out any removal would close it.

## Re-Review (Pass 5) — 2026-10-05T23:48:06+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Launcher allow-list omits the bootstrap verifier — Resolved
- 🟡 **Dependency** + **Testability** + **Completeness**: No contingency for the local prerelease — Resolved (minisign-only check recorded; Open Question closed)
- 🟡 **Testability**: Declared roles, contexts and kinds unchecked — Resolved (Expected declarations table plus criterion)
- 🔵 **Clarity**: Unclear referents in Technical Notes — Resolved
- 🔵 **Testability**: Same-host measurement — Resolved
- 🔵 **Dependency**: Closing sequencing steps — Resolved
- 🔵 **Testability**: Shared-context domain edges and allowed-edge fixtures — Partially resolved (some role-to-role pairs remain incidental)
- 🔵 **Scope** + **Completeness** + **Dependency** + **Testability**: Rule-3 removal branch open-ended — Still present (kept by the author; scope now rates it major)
- 🔵 **Scope**: Epic-scale task — Re-raised as major (it was minor in pass 4; the content is unchanged and the author has accepted it)

### New Issues Introduced

- 🟡 **Clarity**: Requirement 7 allows two remedies, but the Landing table uses a third: it moves symbols into the `corpus` domain crate.
- 🟡 **Testability**: The characterisation list omits the `work-adapters` VCS-facts reroute (author and working-copy status in `work-cli`), and "a … command" per binary may not exercise every rerouted symbol.
- 🟡 **Scope**: "No exception list" forces delivery in one big merge (re-raised; it was minor in pass 1 and the author has accepted it).
- 🔵 **Clarity**: The three ADR-open constraints have no check names; "composition root" is used both with and without the launcher; the migrations to characterise are unnamed.
- 🔵 **Dependency**: The "before" warm-dispatch figure also needs a published release; the sequencing gaps inside `migrate` and the launcher; release-host availability and network effects.
- 🔵 **Testability**: The cold baseline is not like-for-like (before can fetch or fall back to `Unchecked`); the latency fixture repositories are unshaped; the catalogue test seam and the literal check are unstated or fragile; `notices:check` may not prove launcher coverage.
- 🔵 Suggestions: `visualiser/server` is a path, not a package name; the `work` → `config` removal sits inside Requirement 8; the repository-facts and sync-baseline ports should be named in Requirement 7; the tracked-file warning text is unpinned.

### Assessment

All three majors from pass 4 are resolved. Of the five majors now raised, three are scope findings about decisions the author has already made and recorded: a single task, no exception list, and a rule-3 branch that stays open. Their severity has moved between minor and major across passes on unchanged content. That makes them review noise rather than new information. Only two majors are new defects, and each is a one-line fix: name the third remedy in Requirement 7, and add the `work-cli` author and working-copy-status paths to the characterisation list. While the author keeps the scope decisions, the verdict will stay REVISE on the count rule, however much else is fixed.

## Verdict Change — 2026-10-05T23:50:36+00:00

**Verdict:** COMMENT (changed from REVISE by the reviewer)

After pass 5, the work item was edited to fix both new defects:

- 🟡 **Clarity**: Requirement 7's remedies — Resolved. Requirement 7 and the Summary now name a third remedy, moving context-owned logic or types into the upstream domain crate, and point to the Landing table.
- 🟡 **Testability**: Characterisation coverage of the `work-adapters` reroute — Resolved. The list now names the `work-cli` author and working-copy-status paths, and requires at least one command per rerouted symbol.

The three remaining majors come from the scope lens: a single task rather than an epic, no exception list, and an open rule-3 branch. Each reflects a decision the author has made and recorded in the Drafting Notes, Requirement 9 and Open Questions. The reviewer accepts them as trade-offs rather than defects, so the item is acceptable as is. The pass-5 minors are left for planning, notably the cold-latency baseline not being like-for-like, the unshaped latency fixtures, the catalogue test seam, and the `visualiser/server` package name.

## Verdict Change — 2026-10-06T00:19:54+00:00

**Verdict:** APPROVE (changed from COMMENT by the reviewer)

The reviewer approves the work item as ready for planning, with the pass-5 minors carried into planning.

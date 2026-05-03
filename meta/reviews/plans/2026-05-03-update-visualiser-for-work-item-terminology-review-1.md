---
date: "2026-05-04T15:00:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-03-update-visualiser-for-work-item-terminology.md"
review_number: 1
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, compatibility, documentation, standards, safety]
review_pass: 3
status: complete
---

## Plan Review: Update Visualiser for Work-Item Terminology

**Verdict:** REVISE

The plan is well-sequenced, recognises atomicity as the binding constraint for Phase 1, and applies a defensible TDD discipline that distinguishes net-new behaviour from mechanical-rename work. However, several substantive concerns recur across multiple lenses: a Phase 1/Phase 3 status-handling gap that breaks PATCH for legacy `proposed` work-items mid-upgrade; a two-step rename trajectory (`parseTicketNumber` → `parseWorkItemNumber` → `parseWorkItemId`, and `TicketStatus` enum → `WorkItemStatus` enum → string newtype) that creates avoidable churn and a type-safety regression; under-specified pattern/wiki-link regexes that won't accept the project codes the compiler advertises; and a documentation pass (single mega-ADR, missing README, placeholder ADR number, lax CHANGELOG) that doesn't match the project's tightly-scoped conventions. None are critical, but the count and overlap of major issues warrant a revise pass before implementation.

### Cross-Cutting Themes

- **Phase 1 / Phase 3 status-handling gap** (flagged by: correctness, compatibility, test-coverage) — Phase 1 keeps the `Todo | InProgress | Done` enum, but live `meta/work/` has at least one file with `status: proposed` and the seven-status template includes `draft|ready|review|blocked|abandoned`. Read-side falls into "Other" (fine), but the PATCH path 400s for any legacy/templated value until Phase 3 lands. The plan claims "drag-drop status changes succeed" as a Phase 1 success criterion without acknowledging this regression window.
- **Two-step rename trajectory** (flagged by: code-quality, standards, architecture) — `parseTicketNumber` → `parseWorkItemNumber` (Phase 1) → `parseWorkItemId` (Phase 2); `TicketStatus` enum → `WorkItemStatus` enum (Phase 1) → `WorkItemStatus(String)` newtype (Phase 3); `work_item_by_number` → `work_item_by_id`. Each rename touches every call site twice and freezes intermediate-only names into git history.
- **Pattern/wiki-link regex correctness** (flagged by: correctness, test-coverage, architecture) — The example regex `^([A-Z]+-\d+)-` rejects valid project codes containing digits or lowercase letters; the wiki-link tail `[A-Za-z][A-Za-z0-9]*-\d+` rejects multi-segment project codes (`ACME-CORE-0042`). The frontend re-implements the parsing instead of consuming the server-supplied compiled regex.
- **Multi-field cross-ref under-specification** (flagged by: correctness, code-quality, architecture, test-coverage) — Numeric canonicalisation rule isn't defined when the configured pattern is project-prefixed; conflicting values across `work-item:`/`parent:`/`related:` aren't tested; cycles, self-references, and malformed YAML (null, scalar-where-array, integer vs string) aren't tested; pulling pattern config into `frontmatter.rs` couples a previously-pure parser to runtime config; composition with ADR-0017's reverse index is asserted but not specified.
- **Status-validation ownership and type safety** (flagged by: architecture, code-quality, standards) — Phase 3 replaces the typed enum with a `WorkItemStatus(String)` newtype validated at runtime, with validation in both `patcher::apply` and `api/docs.rs`. Loss of compile-time exhaustive-match checks plus duplicated validation invites drift. Newtype-of-`String` is also uncharacteristic of the surrounding crate.
- **Cleanup-grep / rename-completion verification** (flagged by: code-quality, test-coverage, safety) — The Step 1.7 grep excludes `\.test\.|/fixtures/|CHANGELOG`, can miss `Ticket` in CamelCase symbols, doesn't sweep the wider repo for `[[TICKET-...]]`, and doesn't catch `OnlyTicketsAreWritable` log-pattern matchers in operator tooling.
- **Unmigrated-project / fail-fast on missing config** (flagged by: compatibility, safety, architecture) — Post-rename, a project that hasn't run `0001-rename-tickets-to-work` reproduces the same silent-empty-writable-roots symptom the plan was created to fix, just under a new key. Server doesn't validate that `doc_paths.work` exists at boot.
- **Shell-test convention / launcher test gap** (flagged by: test-coverage, standards) — Steps 1.2a and 3.1a leave the test convention as "verify before adding" / "or extend test-config.sh". The launcher is the wire that, when broken, causes the entire bug being fixed; its test layer is the least specified.
- **ADR scope and numbering** (flagged by: documentation, standards) — Step 4.1 bundles four decisions (terminology rename, configurable kanban columns, wiki-link pattern, cross-ref triple) into a single ADR titled `visualiser-work-item-terminology`. Conflicts with the project's tightly-scoped, short-mechanism-titled convention. Number is left as `NNNN` despite parallel ADR work.
- **VCS hygiene in jj workspace** (flagged by: standards, safety) — Step 1.5 prescribes `git mv`, but this is a jj workspace; recovery from a partial rename is harder and the user has explicit memory feedback to use jj.

### Tradeoff Analysis

- **Atomic Phase 1 wire-format flip vs. Phase 1/3 PATCH regression on legacy statuses** — Keeping the Phase 1 enum unchanged minimises Phase 1 review surface but ships a real PATCH regression for legacy-schema work-items. Widening to a permissive newtype in Phase 1 (with the Phase 3 config-validation arriving later) collapses the two breaking refactors of `WorkItemStatus` into one and closes the regression window. **Recommendation**: take the destination shape (string-typed) in Phase 1 and add config-driven validation in Phase 3, OR explicitly document that PATCH of out-of-set values is expected to 400 until Phase 3 — don't leave it implicit.
- **Read pattern-aware multi-field cross-refs in Phase 3 vs. read `work-item:` in Phase 1** — Phase 1 deliberately reads only the legacy `ticket:` key. But `templates/plan.md` already migrated to `work-item:`, so any plan written from the new template post-Phase 1 has empty `workItemRefs` until Phase 3. **Recommendation**: have Phase 1 read `work-item:` (one extra key, single-element vec) and defer `parent:`/`related:` aggregation to Phase 3.

### Findings

#### Critical

(none)

#### Major

- 🟡 **Compatibility / Correctness / Test Coverage**: Phase 1 patcher rejects legacy `proposed` status — round-trip impossible until Phase 3
  **Location**: Phase 1 Step 1.4b, Migration Notes, Phase 3 Step 3.1c
  Phase 1 keeps `WorkItemStatus { Todo, InProgress, Done }` unchanged and serde-strict. PATCHing a card whose disk value is `proposed` (or to `proposed`) returns a 400; "Other" swimlane drag-drop is silently broken in pre-migration repos until Phase 3 lands.

- 🟡 **Compatibility / Safety / Architecture**: No fallback or fail-fast for projects that haven't run `0001-rename-tickets-to-work`
  **Location**: Phase 1 Steps 1.2 and 1.4b
  Post-rename launcher writes `doc_paths.work` resolving to a (default) `meta/work/` that doesn't exist for unmigrated repos; server's `cfg.doc_paths.get("work")` returns None or a missing dir, writable-roots becomes empty, and every PATCH returns `OnlyWorkItemsAreWritable` — the exact silent failure mode the plan was created to fix, transposed onto the new key.

- 🟡 **Code Quality / Architecture / Standards**: `WorkItemStatus(String)` newtype shifts validation to runtime, splits ownership across patcher and API, and is uncharacteristic of the crate
  **Location**: Phase 3 Step 3.1c
  Replacing the enum loses exhaustive-match guarantees; both `patcher::apply` and `api/docs.rs` validate against config keys (drift risk); newtype-of-`String` has no precedent in this crate. Recommend: parse-don't-validate via a private constructor at the API boundary and treat `patcher::apply` as taking only validated values.

- 🟡 **Code Quality / Standards / Architecture**: Two-step rename trajectory creates avoidable churn (`parseTicketNumber` → `parseWorkItemNumber` → `parseWorkItemId`; `TicketStatus` enum → `WorkItemStatus` enum → string newtype)
  **Location**: Phase 1 Steps 1.4b/1.6b, Phase 2 Step 2.1b, Phase 3 Step 3.1c
  Same identifiers and same type take two breaking shapes in two consecutive PRs. Either pick the destination names/shape in Phase 1, or defer the rename to where the semantic change lands. Don't freeze intermediate-only names into git history.

- 🟡 **Correctness**: Pattern-aware slug regex `^([A-Z]+-\d+)-` rejects valid project codes containing digits or lowercase letters
  **Location**: Phase 2 Step 2.1a
  Tests use `[A-Z]+`; the configurable pattern grammar advertises free-form `{project}` (e.g. `web2`, `ACME-CORE`). Drive the test from the actual `work-item-pattern.sh --compile-scan` output rather than a hand-rolled assumption.

- 🟡 **Correctness**: Wiki-link regex is ambiguous for multi-segment project IDs and won't match `[[WORK-ITEM-ACME-CORE-0042]]`
  **Location**: Phase 2 Step 2.3b
  `[A-Za-z][A-Za-z0-9]*-\d+` only allows a single project segment before digits. Either consume the server-supplied `work_item.scan_regex` on the frontend, or document and test the explicit grammar this regex supports.

- 🟡 **Correctness**: Numeric canonicalisation rule is under-specified when a project pattern is configured
  **Location**: Phase 3 Step 3.2b
  When pattern is `{project}-{number:04d}` and `default_project_code: "PROJ"`, what does YAML `parent: 42` canonicalise to — `"0042"`, `"PROJ-0042"`, or `"42"`? Heterogeneous shapes (`42`, `"42"`, `"PROJ-0042"`) referring to the same work-item won't be deduplicated unless the rule is pinned. Reuse `wip_canonicalise_id` from `work-item-common.sh:354` if its semantics fit.

- 🟡 **Correctness**: String-keyed indexer has no defined precedence for mixed bare-numeric / project-prefixed IDs
  **Location**: Phase 2 Step 2.1b
  `HashMap<String, PathBuf>` keyed on `"0001"` vs `"PROJ-0001"`. Mixed workspaces (transient during pattern rollout) get silent mis-resolution — wiki-link `[[WORK-ITEM-0001]]` is genuinely ambiguous. Either fail-fast on detected mixing or define a precedence rule and test it.

- 🟡 **Architecture**: Numeric canonicalisation in `work_item_refs_of` couples a pure parser to pattern config
  **Location**: Phase 3 Step 3.2b
  `frontmatter.rs` becomes config-aware — every consumer needs the pattern in scope; tests need config fixtures. Keep `work_item_refs_of` returning raw values and canonicalise in `indexer.rs` where pattern context already lives.

- 🟡 **Architecture / Test Coverage**: Two reverse-ref indexes will coexist without a unifying abstraction; cycle/self-reference cases untested
  **Location**: Phase 3 Step 3.2b, Step 3.2a
  ADR-0017's `reviews_by_target` plus the new work-item reverse-ref index. The plan asserts "compose" but doesn't specify whether they share `IndexEntry.referencedBy` or remain separate. Tests don't cover self-reference, two-way cycles, or non-existent target IDs.

- 🟡 **Test Coverage**: Legacy-schema tolerance is asserted only via manual regression, not a failing test
  **Location**: Migration Notes, Phase 4 Regression scenario
  No fixture pins `type: adr-creation-task`, `status: proposed`, missing `work_item_id:`, missing `parent:`/`related:` and asserts on the documented null/empty contract. The most explicitly stated tolerance contract has no automated guard.

- 🟡 **Test Coverage / Standards**: Shell-test convention is invoked conditionally without resolution; launcher is the wire that motivated this whole plan
  **Location**: Phase 1 Step 1.2a, Phase 3 Step 3.1a
  "Or whatever the existing shell test convention is — verify before adding" leaves the lowest-test-coverage area unspecified. The convention exists (`scripts/test-helpers.sh`, `skills/work/scripts/test-work-item-pattern.sh`) but `skills/visualisation/visualise/scripts/` has no shell tests at all today.

- 🟡 **Test Coverage**: Invalid `work.id_pattern` has no failing test, only a "fail fast" assertion
  **Location**: Phase 2 Steps 2.1b/2.2
  Phase 2 success criteria checks this manually. With no automated regression test, the "clear message" contract erodes (panic replaces it; exit code changes; message becomes generic).

- 🟡 **Test Coverage**: No tests for conflicting `work-item:`/`parent:`/`related:` values, malformed YAML shapes, or PATCH boundary between configured columns and "Other"
  **Location**: Phase 3 Step 3.2a, Step 3.1b/c
  Hand-edited frontmatter is the dominant input source. The 30 legacy `meta/work/` files land here unmodified. Tests need: scalar-where-array, null, integer-vs-string, duplicated keys, self-ref via different keys, and the PATCH "card currently in Other" boundary semantics.

- 🟡 **Test Coverage**: Phase 4 validation scenarios depend on manual judgement and aren't reproducible
  **Location**: Phase 4 Step 4.4
  "Verify all visualiser surfaces" with no defined predicate, no script to seed fixtures, no captured output. Convert each scenario into a Playwright spec or shell script that asserts specific DOM/API shapes.

- 🟡 **Compatibility**: `OnlyTicketsAreWritable` → `OnlyWorkItemsAreWritable` is a breaking error-string rename without versioning, deprecation, or repo-wide audit
  **Location**: Phase 1 Step 1.4b
  Wire-form string `"only work-items are writable"` changes; any external consumer (operator tooling, log alerting) breaks silently. CHANGELOG should call this out explicitly.

- 🟡 **Compatibility**: `IndexEntry.ticket` removal is a JSON-shape break without consumer enumeration
  **Location**: Phase 1 Step 1.6b
  Field shape changes from scalar nullable string to array; `workItemRefs` is also intentionally narrow (still reads only `ticket:`) until Phase 3, which means the new field is misleadingly empty for new-schema work-items mid-upgrade.

- 🟡 **Documentation / Standards**: ADR bundles four decisions; title format diverges from short-mechanism convention; number is left as `NNNN`
  **Location**: Phase 4 Step 4.1
  Single ADR titled `visualiser-work-item-terminology` covers terminology, columns, wiki-link prefix, cross-refs. Per project memory, ADRs are tightly scoped to one concern with short titles naming the core mechanism. Pick a number now to avoid collision with parallel ADR work.

- 🟡 **Documentation**: Plan references a README that does not exist; visualiser config docs have no clear home
  **Location**: Phase 4 Step 4.2
  `skills/visualisation/visualise/README.md (verify exact path)` — there is no README at that path. Designate ONE canonical location for `visualiser.kanban_columns` schema docs (the `skills/config/configure/SKILL.md` "Work Items" section pattern is the established precedent).

- 🟡 **Documentation / Compatibility**: CHANGELOG framing of breaking config change overstates migration coverage
  **Location**: Phase 4 Step 4.3
  "Auto-applied by migration `0001-rename-tickets-to-work`" assumes migration has run. Pre-migration upgraders see the same silent-empty-kanban failure with no CHANGELOG guidance to run `/accelerator:migrate`.

- 🟡 **Documentation**: Validation file uses placeholder date and lacks frontmatter spec
  **Location**: Phase 4 Step 4.4
  `2026-05-XX` placeholder; no quoted frontmatter shape. Compare to `templates/validation.md` if one exists; populate `work-item:` linkage explicitly.

- 🟡 **Standards / Safety**: `git mv` is wrong VCS for this jj workspace
  **Location**: Step 1.5
  User memory: "Always run jj commands from within the active workspace, never from the repo root." Use `mv` (jj snapshots automatically) or explicit `jj file move` from inside the workspace. Add a precondition to verify the workspace is clean before fixture moves.

- 🟡 **Code Quality / Test Coverage / Safety**: Step 1.7 cleanup grep is too narrow to reliably catch ~50 + ~40 rename sites
  **Location**: Step 1.7
  `grep -rni "ticket"` excludes `.test.`, `/fixtures/`, `CHANGELOG`, `.module.css.map`. Doesn't catch CamelCase residue (`Ticket[A-Z]`, `Tickets?`), doesn't sweep the wider repo for `[[TICKET-...]]` body-text wiki-links, and doesn't audit operator tooling for old error-string log-pattern matchers.

- 🟡 **Safety**: No fail-fast validation when `work` config key is absent or points at a missing directory; recovery path when `id_pattern` changes after first launch is undocumented
  **Location**: Phase 1 Step 1.4b, Phase 2 Step 2.1b
  Both reproduce silent-degradation failure modes. Validate `work` (and `review_work`) at `AppState::build`; document that pattern changes require restart + cache-bust.

- 🟡 **Safety**: Empty or malformed `kanban_columns` config behaviour is unspecified — could lock all writes or accept anything
  **Location**: Phase 3 Step 3.1c
  Tests cover only populated lists. Pin behaviour for empty-list, missing-field, and malformed-YAML cases; document in the ADR.

#### Minor

- 🔵 **Correctness**: Phase 1 `work_item_refs_of` reads only `ticket:`, but `templates/plan.md:4` already migrated to `work-item:`
  **Location**: Phase 1 Step 1.4b
  New plans created post-migration but pre-Phase-3 will silently show no cross-refs in "Related artifacts". One-line fix to also read `work-item:` in Phase 1.

- 🔵 **Correctness / Code Quality**: `IndexEntry.workItemId` contract conflates "regex doesn't match" with "frontmatter has no ID" and the `Completeness.has_work_item` boolean is now ambiguous next to `workItemRefs: []`
  **Location**: Phase 1 Step 1.4b, Phase 2 Step 2.1b
  Field is filename-derived, not frontmatter-derived. Migration Notes phrasing is misleading. Also: drop `has_work_item` in favour of `!workItemRefs.is_empty()` at the consumer site, or rename precisely.

- 🔵 **Code Quality / Architecture**: Storing `scan_regex: String` in `Config` invites recompilation; cached `Regex` location unspecified
  **Location**: Phase 2 Step 2.1b
  Hold the compiled `regex::Regex` directly (alongside the original string) via a fallible constructor at boot. Exactly one place can fail to compile.

- 🔵 **Code Quality / Architecture**: `work_item_refs_of` carries three orthogonal responsibilities (read, dedup, canonicalise)
  **Location**: Phase 3 Step 3.2b
  Split into pure pieces: `read_ref_keys` (parser), `dedup_refs` (set logic), `canonicalise_ref` (config-aware). Composer in `indexer.rs`.

- 🔵 **Architecture / Test Coverage**: Shell-out to `work-item-pattern.sh --compile-scan` creates an implicit version contract with no contract test
  **Location**: Phase 2 Step 2.2
  Document the CLI contract in the pattern compiler's SKILL.md and add a contract-level shell test exercising it from the visualiser side.

- 🔵 **Architecture**: Sequential breaking refactors of `WorkItemStatus` in adjacent phases
  **Location**: Phase 1 Step 1.4b and Phase 3 Step 3.1c
  Same type takes two breaking shapes in two consecutive PRs. See cross-cutting recommendation above.

- 🔵 **Architecture / Standards**: Open question on plugin-vs-launcher placement of `config-read-visualiser.sh` deferred to implementation; precedent (`config-read-review.sh`) is the wrong model
  **Location**: Phase 3 Step 3.1a
  `config-read-review.sh` is a 605-line markdown emitter. Skill-scoped helpers like `config-read-path.sh` / `config-read-value.sh` are the right pattern. Or read `visualiser.kanban_columns` directly via `config-read-value.sh`.

- 🔵 **Architecture / Documentation**: Dual-schema support (legacy + new) is permanent but framed as transitional; no follow-up plan tracked
  **Location**: Desired End State (item 6), Migration Notes
  Capture in the ADR as a first-class architectural decision with an enumerated supported-legacy-shape list and a regression fixture. Track or close the "may follow this plan" frontmatter migration.

- 🔵 **Compatibility**: Existing `[[TICKET-NNNN]]` references in documents stop resolving with no migration or fallback
  **Location**: Phase 1 Step 1.6b
  Audit `meta/` for `[[TICKET-` patterns or accept both prefixes for one release.

- 🔵 **Compatibility**: `meta/reviews/work/` directory may not exist on fresh installs; server should return `200 []`, not 500
  **Location**: Phase 1 Success Criteria
  Add an automated test asserting `GET /api/docs/work-item-reviews` returns `200 []` when the configured directory does not exist.

- 🔵 **Compatibility**: SSE `docType` field flips from `"tickets"` to `"work-items"` mid-stream on upgrade
  **Location**: Phase 1 Step 1.4b
  Long-lived browser sessions surviving a server restart fall into a stale-cache gap. Either bump an SSE version field or document the upgrade-then-refresh expectation.

- 🔵 **Compatibility**: Phase 3 changes the PATCH error response shape — `{ error, accepted_keys }` is bespoke vs the existing `ApiError` envelope
  **Location**: Phase 3 Step 3.1c
  Add an `ApiError::UnknownKanbanStatus { accepted_keys }` variant so the wire format stays uniform.

- 🔵 **Test Coverage**: 'Rename tests first' has no test asserting the old names are gone; project-pattern Playwright fixture setup is under-specified; `api_work_item_pattern` integration test misses negative cases; `test-fixtures.ts` defaults have no schema-shape assertion
  **Location**: Phase 1 Steps 1.4a/1.6a, Phase 2 Step 2.4 / Step 2.1c, Step 1.6a
  Tighten the grep, specify the spec's setup, add a mismatched-filename fixture, add a meta-test on default-fixture shape.

- 🔵 **Documentation**: Pattern-compiler CLI contract not quoted; SKILL.md / README updates scattered without a consolidated checklist; deferred frontmatter migration left as informal TODO; Phase 12 status fix-up buried in Phase 1 Step 1.7
  **Location**: Phase 2 Step 2.2, Phase 4 Step 4.2, Migration Notes, Step 1.7
  Add a Phase 4 "Documentation deliverables" subsection enumerating every doc file the plan touches; track or close the follow-up migration explicitly; extract Phase 12 frontmatter flip as a standalone change.

- 🔵 **Standards**: `GET /api/kanban/config` vs extending `GET /api/types` left as `or` decision; pick one
  **Location**: Phase 3 Step 3.1c
  Cleaner REST shape is a dedicated `/api/kanban/config`. Lock it in so the frontend hook name aligns.

- 🔵 **Safety**: Plan doesn't assert no panic on edge-case YAML in `work_item_refs_of`; `OnlyTicketsAreWritable` rename may break log-pattern matchers; grep exclusion list could miss release notes / screenshots
  **Location**: Phase 3 Step 3.2a, Phase 1 Step 1.4b, Step 1.7
  Wrap `work_item_refs_of` with a per-file try/catch at the indexer call site; CHANGELOG-list the error-string change explicitly; widen the rename-completion grep beyond the visualiser tree.

- 🔵 **Correctness**: Composition with ADR-0017's plan-review reverse index is asserted but not specified
  **Location**: Phase 3 Step 3.2b
  Document the composed `IndexEntry.referencedBy` shape (e.g. `Vec<{ kind, relPath }>`) with a heterogeneous-test fixture.

- 🔵 **Correctness / Standards**: Status validation case-sensitivity, whitespace-trim, and key-vs-label confusion not specified
  **Location**: Phase 3 Step 3.1c
  Pin exact-match (case-sensitive, no trim) and document; add a test for case-mismatch path.

#### Suggestions

- 🔵 **Standards / Documentation**: Phase 12 status fix-up belongs outside the work-item-terminology PR — split into a standalone trivial commit either before Phase 1 or as part of Phase 4's doc pass.
- 🔵 **Code Quality**: Add a `// TODO(Phase 3): replace with server-driven config — see plan 2026-05-03` next to Phase 1's hardcoded `STATUS_COLUMNS` so the intermediate state is obviously deliberate.

### Strengths

- ✅ Phase 1 correctly framed as an atomic wire-format flip with a single PR boundary; "no incremental bridge" is acknowledged explicitly.
- ✅ Phase sequencing isolates the must-fix (rename) from additive features (pattern, columns, cross-refs); Phase 1 alone restores end-to-end function.
- ✅ TDD discipline differentiated by change-shape (red→green for new behaviour vs compiler-driven for renames) is pragmatic and correctly applied; each phase begins with a green-baseline gate.
- ✅ Explicit "What We're NOT Doing" list prevents scope creep, especially around generic kanban support and frontmatter migration.
- ✅ Pattern compiler reuse via shell-out to existing `skills/work/scripts/work-item-pattern.sh` avoids duplicating the compiler in Rust.
- ✅ Dual-schema tolerance (legacy + new frontmatter) is recognised and folded into the verification matrix via the regression scenario.
- ✅ Wire-format casing is internally consistent (kebab-case wire forms, CamelCase Rust variants, camelCase TS fields, snake_case config keys).
- ✅ Test file naming and Rust integration-test conventions slot in cleanly with existing patterns (`api_smoke.rs`, `api_lifecycle.rs`).
- ✅ Step 3.1b includes the explicit boundary case `patch_status_rejects_unconfigured_value_with_400_and_accepted_keys` with strong assertion specificity.
- ✅ Step 1.3 atomically updates SKILL.md path placeholders alongside the launcher rename.
- ✅ References section cross-links the right prior research and ADRs (0017, 0022, 0023).
- ✅ Fail-safe writable-roots default — when the key is missing, the whitelist is empty and PATCHes are denied; there is no fail-open path.

### Recommended Changes

1. **Resolve the Phase 1 status-handling gap** (addresses: Phase 1 patcher rejects legacy `proposed`; Sequential breaking refactors; Type-safety regression)
   Either (a) widen `WorkItemStatus` to a permissive newtype/string in Phase 1 with hardcoded default validation, leaving only the *config-driven* validation to Phase 3; or (b) explicitly document in Phase 1 success criteria that PATCH of values outside `{todo, in-progress, done}` is expected to 400 until Phase 3 lands and update Migration Notes accordingly. Option (a) collapses two breaking refactors of the same type into one and closes the regression window for legacy-schema repos.

2. **Add fail-fast validation for required config keys at server boot** (addresses: No fallback for unmigrated projects; No fail-fast on missing `work` directory; Single config key drives writable-roots)
   At `AppState::build`, validate that `cfg.doc_paths.get("work")` exists, resolves to an existing directory, and that `cfg.doc_paths.get("review_work")` follows the same rule. On failure, exit with a precise message naming the missing key and pointing at `/accelerator:migrate`. Mirror this for `work_item.scan_regex` compilation.

3. **Adopt destination names in Phase 1; eliminate the two-step rename** (addresses: Two-step rename trajectory; Naming inconsistencies; Sequential breaking refactors)
   Rename `parseTicketNumber` → `parseWorkItemId` (not `parseWorkItemNumber`), `ticket_by_number` → `work_item_by_id`, and have the Phase 1 implementation parse the digit prefix as a string under the default pattern. Frontend renames similarly. Phase 2 then only adds pattern-aware logic without touching identifiers.

4. **Pin the multi-field cross-ref semantics before implementation** (addresses: Numeric canonicalisation under-specified; Conflicting values untested; Frontmatter coupled to pattern config; Single function carries three responsibilities; Composition with ADR-0017 unspecified)
   Define the canonical form (e.g. apply `format_string` then prefix `default_project_code` if pattern requires `{project}`); reuse `wip_canonicalise_id` from `work-item-common.sh:354` if its semantics fit. Split `work_item_refs_of` into `read_ref_keys` (pure) + `dedup_refs` + `canonicalise_ref` (config-aware, in indexer). Pin precedence for conflicting `work-item:`/`parent:`/`related:` values. Specify the composed `IndexEntry.referencedBy` shape with reference to ADR-0017. Add tests for: scalar-where-array, null, integer-vs-string, duplicated keys, self-reference, two-way cycle, non-existent target.

5. **Drive the pattern regex from the compiler, not from hand-rolled assumptions** (addresses: Slug regex over-restrictive; Wiki-link regex ambiguous for multi-segment IDs)
   Have the frontend consume the server-supplied `work_item.scan_regex` rather than maintaining a parallel hand-written regex. In server tests, drive `derive_work_item_with_regex` cases from the actual `work-item-pattern.sh --compile-scan` output. Widen the example regex in Step 2.1a to admit lowercase letters and digits in project codes.

6. **Restructure documentation pass** (addresses: ADR bundles four decisions; ADR title divergence; Number collision risk; README does not exist; CHANGELOG framing; Validation file placeholder)
   Split into ≥3 ADRs (configurable kanban columns; cross-ref aggregation; visualiser conformance to ADR-0022 if needed at all) with short mechanism-named titles. Reserve contiguous numbers now and record them in this plan + References. Drop the README bullet (or scope it explicitly); designate `skills/config/configure/SKILL.md > Visualiser` as the canonical home for `visualiser.kanban_columns`. Reword CHANGELOG breaking entry to direct unmigrated users to `/accelerator:migrate`. Replace `2026-05-XX` placeholder; quote validation frontmatter shape. Add a "Documentation deliverables" subsection to Phase 4.

7. **Tighten the rename-completion verification** (addresses: Step 1.7 grep too narrow; Stale wiki-links not audited; Operator tooling patterns)
   Drop the `\.test\.|/fixtures/` exclusion. Add a CamelCase grep (`grep -rn -E '\bTicket[A-Z]|\bTickets?\b' skills/visualisation/visualise/`). Sweep the wider repo for `[[TICKET-` body-text patterns. Audit non-visualiser code/tooling for the old error-literal `only tickets are writable`. List allowed exceptions by exact path.

8. **Resolve the shell-test convention up front and add a launcher contract test** (addresses: Shell-test convention vague; Launcher test gap)
   State explicitly: "create `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`, sourcing `scripts/test-helpers.sh`, modelled on `skills/work/scripts/test-work-item-pattern.sh`." Add a contract-level shell test exercising `work-item-pattern.sh --compile-scan` from the visualiser side. Add tests for invalid `work.id_pattern` (server boot error path) and for empty/missing/malformed `kanban_columns`.

9. **Replace `WorkItemStatus(String)` newtype with idiomatic alternative** (addresses: Type-safety regression; Validation ownership split; Newtype-of-String uncharacteristic)
   Either keep an enum and let unknown statuses fall into "Other" at render-time, or drop the newtype entirely and store status as plain `String` validated exactly once at the API boundary via `WorkItemStatus::parse(s, &allowed) -> Result<...>`. Patcher takes the validated value and concerns itself only with YAML mutation. Use the existing `ApiError` envelope (`UnknownKanbanStatus { accepted_keys }`) instead of a bespoke 400 body. Pick `/api/kanban/config` (dedicated endpoint) over extending `/api/types`.

10. **Replace `git mv` with jj-aware filesystem operations and add a workspace-clean precondition** (addresses: jj VCS convention; Fixture-move safety)
    Use `mv` (jj snapshots automatically) inside the active workspace; precondition: `jj st` clean before fixture moves; `jj st` after to confirm rename captured. Add the same convention to any future plan steps that move files.

11. **Address legacy-schema and edge-case test gaps explicitly** (addresses: Legacy-schema only manual; Reverse-ref cycles; Conflicting cross-ref values; Phase 4 manual scenarios)
    Add a server fixture under `tests/fixtures/meta/work-legacy/` with the exact 30-file shape and an integration test asserting `workItemId == None`, `workItemRefs == []`, `proposed` falls into "Other", and the file appears in kanban+library. Convert Phase 4 scenarios into Playwright specs / scripts with deterministic seeds and asserted outputs. Add tests for self-ref / two-way cycle / unknown-target reverse-ref cases. Wrap `work_item_refs_of` per-file at the indexer call site so a single bad file degrades gracefully.

12. **Read `work-item:` frontmatter key in Phase 1** (addresses: Phase 1 reads only `ticket:` but plan template migrated)
    One-line addition to Phase 1's `work_item_refs_of`: read `ticket:` (legacy) OR `work-item:` (current) into the single-element vec. Defer only `parent:`/`related:` aggregation to Phase 3.

13. **Extract Phase 12 frontmatter status fix to a separate change** (addresses: Bookkeeping mixed with rename PR)
    Land the `status: draft` → `status: complete` flip on `meta/plans/2026-04-30-...phase-12....md` as a standalone trivial commit before or after Phase 1, not bundled in.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-structured architecturally: it correctly identifies the atomic wire-format flip as a binding constraint, sequences phases so Phase 1 alone restores end-to-end function, and layers additive features onto a stable rename. Main concerns: (a) where new responsibilities sit — runtime status validation is split across patcher and API without a clear owner, (b) new coupling between pure frontmatter parsing and pattern config, (c) two reverse-ref indexes that should compose intentionally rather than accidentally.

**Findings**: 3 major (status-validation ownership split; numeric canonicalisation couples frontmatter parsing; two reverse-ref indexes coexisting). 5 minor (shell-out version contract; sequential breaking refactors; dual-schema permanence; missing fail-fast on `work` key; plugin-vs-launcher helper placement).

### Code Quality

**Summary**: Phasing well-justified and TDD discipline pragmatic, but several maintainability concerns: Phase 3 enum-to-string newtype is a type-safety regression; naming inconsistencies (`workItemRefs`/`workItemId`/`parseWorkItemNumber`/`parseWorkItemId`) risk reader confusion; Step 1.7 grep too lax to reliably catch leftovers across ~50 + ~40 sites.

**Findings**: 3 major (type-safety regression; cleanup grep narrow; naming inconsistencies). 5 minor (composite `work_item_refs_of`; bespoke error body; `scan_regex: String`; ambiguous `has_work_item`; intermediate `STATUS_COLUMNS`).

### Test Coverage

**Summary**: Solid TDD spine for net-new behaviour and defensible "rename tests first" for mechanical work. Risk-bearing gaps: legacy-schema tolerance asserted only manually; reverse cross-ref index lacks cycle/self-reference tests; shell-test convention left vague; error/sad paths around invalid id_pattern, malformed frontmatter, and conflicting cross-ref values not exercised. Phase 4 mixes automated checks with manual judgement.

**Findings**: 8 major (legacy-schema only manual; reverse-ref cycles; shell-test convention; PATCH boundary; invalid id_pattern; conflicting cross-ref values; malformed frontmatter; manual Phase 4). 4 minor (rename-completion test; Playwright fixture setup; pattern integration negatives; test-fixtures.ts assertion).

### Correctness

**Summary**: Several correctness concerns lurk in regex pattern handling, numeric/string ID canonicalisation, and the wire-format flip. Most consequential: over-restrictive pattern-aware slug regex; ambiguous wiki-link alternation for multi-segment IDs; under-specified numeric canonicalisation when project pattern is configured; un-stated collision risk in string-keyed indexer; Phase 1 leaves `proposed` legacy status with no rendering path until Phase 3.

**Findings**: 5 major (slug regex `[A-Z]+`; wiki-link multi-segment; canonicalisation rule; indexer collision; Phase 1 `proposed` gap). 4 minor (status validation case-sensitivity; ADR-0017 composition; Phase 1 reads only `ticket:`; `workItemId` three-state contract).

### Compatibility

**Summary**: Coordinated Phase 1 PR is the right strategy. Several compatibility hazards exist around mid-upgrade states: legacy work-items can't be PATCHed during Phase 1 because the `Todo|InProgress|Done` enum forbids `proposed`; wire-format breaking changes not behind a versioned API; unmigrated-project case not addressed.

**Findings**: 4 major (Phase 1 patcher rejects `proposed`; no fallback for unmigrated; `OnlyTicketsAreWritable` rename without versioning; `IndexEntry.ticket` JSON shape break). 5 minor (`[[TICKET-...]]` references; CHANGELOG framing; Phase 3 PATCH error shape; `meta/reviews/work/` existence; SSE docType flip).

### Documentation

**Summary**: Plan threads documentation work through every phase and references the right canonical sources, but several deliverables are vague, mis-located, or risk overlap. ADR bundles four decisions; Step 4.2 references a non-existent README; ADR number collision risk; CHANGELOG framing too thin for unmigrated repos.

**Findings**: 5 major (ADR bundles 4 decisions; README does not exist; ADR number `NNNN`; CHANGELOG framing; Validation placeholder). 4 minor (deferred migration TODO; Phase 12 in Step 1.7; Pattern-compiler CLI contract; SKILL.md/README scattered).

### Standards

**Summary**: Plan generally aligns with project naming conventions, but has specific gaps: relies on shell-test convention without verifying its existence; uses `git mv` in a jj workspace; prescribes a two-step rename creating churn; picks a non-idiomatic ADR title format; introduces a `WorkItemStatus(String)` newtype without checking prevailing patterns. Error-response shape and column-config endpoint under-specified.

**Findings**: 3 major (`git mv` wrong VCS; shell-test convention; two-step rename). 5 minor (`WorkItemStatus(String)` newtype; `config-read-visualiser.sh` precedent; ADR title; custom error body; `/api/kanban/config` placement). 1 suggestion (Phase 12 status fix-up scope).

### Safety

**Summary**: Generally safe (writable-roots fails closed; atomic wire-format flip; no per-file frontmatter rewrites). Operational-safety gaps: launcher doesn't validate that configured work path exists; `git mv` in jj workspace can lose uncommitted edits; multi-field cross-ref aggregation has under-specified parsing safety for malformed YAML; a misconfigured `work.id_pattern` after first launch silently changes how IDs are parsed without state rebuild guidance.

**Findings**: 4 major (no fail-fast on missing `work` key; `git mv` jj-unsafe; empty `kanban_columns` lock-out; `id_pattern` change recovery). 4 minor (Phase 12 frontmatter scope; YAML edge-case panics; error-literal log matchers; grep exclusion masks).

---

## Re-Review (Pass 2) — 2026-05-04

**Verdict:** REVISE

The revision substantially improves the plan: nearly every original major finding is genuinely resolved (status validation cleanly owned by API layer; canonicalisation moved to indexer; reverse-ref composition with ADR-0017 specified; fail-fast at boot for missing config keys, invalid scan_regex, empty kanban_columns; legacy-schema regression fixture added; ADRs split with reserved numbers; jj-aware fixture moves; tightened cleanup grep). However, the revision introduces several new substantive issues — most importantly a **critical correctness defect** in Phase 2's wiki-link approach: the plan assumes the pattern compiler emits a class-based regex like `[A-Za-z][A-Za-z0-9-]*-\d+`, but `wip_compile_scan` actually substitutes the literal configured project value char-by-char (with rule 5 forbidding hyphens), so the multi-segment-project tests and the `--compile-wiki-link` flow as written cannot be implemented. Additional new majors centre on `Completeness.has_ticket` being incorrectly dropped (it's a cluster-presence flag, not a per-entry frontmatter check), `todo` being silently dropped from the accepted PATCH-status set without CHANGELOG callout, the validation file's frontmatter contract being unsupported by the canonical template, `config-read-value.sh` being scalar-only (can't read the YAML list), and the boot-time work-directory-must-exist check refusing to start in legitimate empty-state scenarios.

### Previously Identified Issues

#### Resolved (originally major)

- ✅ **Compatibility / Correctness / Test Coverage**: Phase 1 patcher rejects legacy `proposed` — Resolved (Phase 1 takes seven-status default; explicit Other-boundary test added)
- ✅ **Compatibility / Safety / Architecture**: No fallback for unmigrated projects — Resolved (launcher pre-migration check + server fail-fast)
- ✅ **Code Quality / Architecture / Standards**: `WorkItemStatus(String)` newtype — Resolved (no newtype; plain `String` validated at API boundary; patcher takes `&str`)
- ✅ **Code Quality / Standards / Architecture**: Two-step rename trajectory — Resolved (single-pass to destination names: `parseWorkItemId`, `work_item_by_id`, `workItemById`)
- ✅ **Correctness**: Pattern-aware slug regex `[A-Z]+` — Resolved (compiler-driven tests via `compile_scan_via_cli`)
- ✅ **Correctness**: String-keyed indexer collision — Resolved (precedence rule defined; mixed-shape integration test specified)
- ✅ **Correctness**: Phase 1 leaves `proposed` legacy with no rendering path — Resolved (seven-status default in Phase 1; tests pin the boundary)
- ✅ **Architecture**: Numeric canonicalisation couples frontmatter parsing to pattern config — Resolved (split into pure `read_ref_keys` + indexer-side `canonicalise_refs`)
- ✅ **Architecture / Test Coverage**: Reverse-ref index composition with ADR-0017; cycle/self-ref untested — Resolved (composition specified; explicit tests for self-reference, two-way cycle, unknown-target, dedup, kind-merging)
- ✅ **Test Coverage**: Legacy-schema tolerance only manual — Resolved (`api_legacy_schema.rs` fixture under `tests/fixtures/meta/work-legacy/`)
- ✅ **Test Coverage / Standards**: Shell-test convention vague — Resolved (`test-write-visualiser-config.sh` modelled on `test-work-item-pattern.sh`, sources `test-helpers.sh`)
- ✅ **Test Coverage**: Invalid `work.id_pattern` no failing test — Resolved (config unit test + shell test + boot integration)
- ✅ **Test Coverage**: No tests for conflicting/malformed cross-ref values — Resolved (explicit fixtures for null, scalar-where-array, array-where-scalar, int-vs-string)
- ✅ **Test Coverage**: Phase 4 manual scenarios not reproducible — Resolved (Playwright specs `scenarios/*.spec.ts`)
- ✅ **Compatibility**: `OnlyTicketsAreWritable` rename without versioning — Resolved (CHANGELOG calls out wire-string change)
- ✅ **Compatibility**: `IndexEntry.ticket` JSON shape break — Resolved (CHANGELOG enumerates breaking shapes)
- ✅ **Documentation / Standards**: ADR bundles four decisions — Resolved (split into ADR-0024 kanban columns + ADR-0025 cross-ref aggregation; numbers reserved)
- ✅ **Documentation**: README does not exist — Resolved (dropped; `skills/config/configure/SKILL.md` designated as canonical config home)
- ✅ **Documentation / Compatibility**: CHANGELOG framing — Resolved (rewritten with `/accelerator:migrate` guidance and explicit before/after wire shapes)
- ✅ **Documentation**: Validation file placeholder date — Resolved (2026-05-04 used; though see new finding on frontmatter contract)
- ✅ **Standards / Safety**: `git mv` wrong VCS — Resolved (`mv` inside jj workspace with snapshot precondition; though see new minor about enforcement)
- ✅ **Code Quality / Test Coverage / Safety**: Step 1.7 cleanup grep too narrow — Resolved (CamelCase pass, body-text wiki-link sweep across whole repo, error-literal pattern check)
- ✅ **Safety**: Empty `kanban_columns` config behaviour unspecified — Resolved (boot-time validation: missing → defaults; empty → reject; malformed → reject)

#### Partially resolved

- 🟡 **Correctness**: Wiki-link regex multi-segment IDs — Partially resolved. The frontend now consumes server-supplied regex (good), but the assumed compiler output shape doesn't match what `wip_compile_scan` actually emits — see new critical finding below.
- 🟡 **Correctness**: Numeric canonicalisation under-specified — Partially resolved. The bare-numeric → padded and bare-numeric → project-prefixed cases are pinned; the prefixed-input-under-default-pattern case and the legacy-bare-numeric-under-project-pattern case are still under-specified. See new majors below.
- 🟡 **Architecture**: Single config key drives writable-roots; no graceful degradation — Partially resolved. Fail-fast added, but the directory-must-exist requirement is now too aggressive for legitimate empty-state cases.

#### Still present

(none — all originally-identified issues were addressed in some form)

### New Issues Introduced

#### Critical

- 🔴 **Correctness**: Pattern compiler does not emit a generic project-class regex; multi-segment project tests and the `[A-Za-z][A-Za-z0-9-]*-\d+` inner-pattern claim are unrealisable
  **Location**: Phase 2 Step 2.3 (frontend tests + `buildWikiLinkPattern`); Step 2.1a (`compile_scan_via_cli` tests); Phase 2 Success Criteria
  `wip_compile_scan` substitutes the literal configured project value char-by-char (e.g. configured `PROJ` → `^PROJ-([0-9]+)-`), and rule 5 of `wip_validate_pattern` explicitly rejects projects containing hyphens. There is no compiler path that emits a class-based or multi-segment-capable inner pattern. The `[[WORK-ITEM-ACME-CORE-0042]]` test as written cannot pass; the success criterion bullet about multi-segment IDs papers over the gap with a hypothetical that contradicts the compiler grammar. Either drop the multi-segment test and restate the inner pattern as the literal-project form (`PROJ-\d+|\d+`), or extend `work-item-pattern.sh` with a documented `--compile-wiki-link` subcommand that emits class-based output (and document the divergence between scan grammar and wiki-link grammar).

#### Major

- 🟡 **Code Quality**: `Completeness.has_ticket` should be renamed (cluster-presence flag), not dropped
  **Location**: Phase 1 Step 1.4b (clusters.rs); "What We're NOT Doing"
  The plan drops `Completeness.has_ticket` and tells consumers to derive `!entry.workItemRefs.is_empty()`, but these are different concepts. `has_ticket` is a per-cluster flag set when any entry in the lifecycle cluster has `DocTypeKey::Tickets` — parallel to `has_plan`, `has_decision`, `has_research`, `has_pr_review`, all of which are retained. Replacing it with a per-entry frontmatter-presence check breaks lifecycle pipeline rendering and creates an inconsistent `Completeness` shape. Rename `has_ticket` → `has_work_item` instead (single-pass), preserving cluster-presence semantics.

- 🟡 **Compatibility**: Phase 1 PATCH widening silently drops `todo` from the accepted-status set
  **Location**: Phase 1 Step 1.4b; Phase 4 Step 4.3 CHANGELOG
  Phase 1 widens from the legacy three (`todo|in-progress|done`) to the seven template defaults (`draft|ready|in-progress|review|done|blocked|abandoned`). `todo` is removed; only `in-progress` and `done` overlap with the legacy enum. External PATCH clients sending `todo` now receive 400. The CHANGELOG breaking entries cover other wire-shape changes but don't call out this specific PATCH-input contract change. Add an explicit CHANGELOG bullet naming `todo` as removed; consider an alias map (`todo` → `ready`) for one release as a deprecation bridge.

- 🟡 **Standards**: `config-read-value.sh` is scalar-only and cannot return the YAML list `visualiser.kanban_columns`
  **Location**: Phase 3 Step 3.1a
  The plan reads `visualiser.kanban_columns` (a YAML list) via `config-read-value.sh`, citing the established pattern. The script is in fact a scalar reader (single-line `key: value`); the multi-line list shape (`- { key: ready, label: Ready }`) is outside its contract. The boot-time validation rules the plan promises (reject empty list, reject malformed YAML) cannot be implemented without going beyond the helper. Either extend `config-read-value.sh` with a documented `--list` mode, or commit to a different mechanism (jq, or a small new helper following `config-read-path.sh`'s precedent).

- 🟡 **Documentation**: Validation frontmatter contract is unsupported by the canonical template
  **Location**: Phase 4 Step 4.4
  Step 4.4 names `templates/validation.md` as the canonical model for the "standard validation frontmatter shape (`date`, `type: validation`, `skill: validate-plan`, `work-item: ""`, `status: complete`)". The actual template has no frontmatter — it begins with `## Validation Report:` on line 1. Either cite a different canonical source or update `templates/validation.md` to carry the frontmatter contract before authoring the validation file.

- 🟡 **Documentation**: `--compile-wiki-link` subcommand has no SKILL.md / CLI usage doc update
  **Location**: Phase 2 Step 2.3; Phase 4 Documentation Deliverables checklist
  Step 2.3 introduces a sibling `--compile-wiki-link` subcommand on `work-item-pattern.sh`. The existing CLI documents three modes (`--validate`, `--compile-scan`, `--compile-format`). The Phase 4 checklist contains no entry for updating the script's usage block, the v1.20.0 CHANGELOG note about the pattern-compiler CLI, or the configure SKILL.md Pattern DSL Reference. Add the doc updates as deliverables, or commit to deriving the wiki-link inner pattern from `--compile-scan` output inside the launcher (no new subcommand).

- 🟡 **Safety**: Boot-time directory-must-exist check refuses to start in legitimate empty-state scenarios
  **Location**: Phase 1 Step 1.4b (server.rs:58-67)
  The fail-fast validates `cfg.doc_paths.work` is present **and resolves to an existing directory**. The original bug was a missing config *key*, not a missing directory. Requiring directory existence on disk refuses to start a fresh repo where `meta/work/` hasn't been created yet, or after a recovery operation where the directory was temporarily moved. `review_work` is correctly tolerated when absent on disk. Apply the same rule to `work`: reject only when the config key is missing; tolerate a non-existent directory by treating it as empty.

- 🟡 **Correctness**: Round-trip-from-Other test name conflicts with PATCH-into-Other rejection rule
  **Location**: Phase 3 Step 3.1b
  `patch_status_round_trip_from_other_swimlane_to_configured_column` describes seeding `status: proposed` (legacy) and PATCHing to `ready`. The seed step bypasses the API (write to disk directly), but the test name implies "round-trip" which would require both directions to work — and `patch_status_to_other_swimlane_value_rejected` says PATCH back to `proposed` returns 400. Rename to `patch_status_seeds_in_other_then_moves_to_configured_column` to make the seed-via-disk path explicit; expand the boundary contract test to cover the realistic "PATCH-with-status-unchanged when existing status is in Other" case; document in ADR-0024 that Other is genuinely write-blocked, not merely read-segregated.

- 🟡 **Correctness**: Canonicalisation rule undefined when input is project-prefixed but the configured pattern lacks `{project}`
  **Location**: Phase 3 Step 3.2 (canonicalisation rule)
  The pinned rule covers bare-numeric under default and bare-numeric under project. It says "already prefixed values pass through unchanged" but doesn't specify behaviour when a doc carries `parent: "PROJ-0042"` and the *current* config is project-less (`{number:04d}`). `wip_canonicalise_id` would return `E_PATTERN_BAD_FORMAT_SPEC` in this case. Add a fourth rule (pass-through verbatim; rely on the indexer's lookup to find/fail-to-find a match) and a test.

- 🟡 **Correctness**: Phase 1 single-key read undefined when both `ticket:` and `work-item:` are present
  **Location**: Phase 1 Step 1.4b (`work_item_refs_of`)
  Reads either `ticket:` (legacy) OR `work-item:` (current); the disjunction is undefined when both are present. A migrated repo where someone hand-edited a file to add `work-item:` without removing `ticket:` would index unpredictably depending on serde ordering. Pin behaviour: `work-item:` wins (preferred — newer key); both contribute and dedup; or reject as malformed. Add a test.

- 🟡 **Architecture**: Server tests gain a cross-process compile-time dependency on an external shell script
  **Location**: Phase 2 Step 2.1a
  The `slug.rs` unit tests shell out to `${PLUGIN_ROOT}/skills/work/scripts/work-item-pattern.sh --compile-scan` at suite startup. This pins the contract usefully but couples the server test suite to plugin layout — `cargo test` in isolation would fail if the script isn't discoverable, and the cross-process boundary isn't documented. Either add a contract-test file at the pattern-compiler skill level (compiler owns its output stability), or introduce a thin Rust-side wrapper module (`pattern_compiler.rs`) encapsulating the shell-out.

- 🟡 **Architecture**: Boot-time pattern-immutability invariant deserves explicit ADR capture
  **Location**: Migration Notes; Phase 3 manual verification
  The plan documents that `work.id_pattern` changes require a restart, but only as a manual-verification line and a SKILL.md note. Future contributors adding live config reload (likely once `kanban_columns` reload appetite emerges) will hit this implicit floor. Capture in ADR-0024 (or a sibling ADR) which fields are boot-immutable (`scan_regex`, `default_project_code`, `doc_paths`) vs reload-safe (`kanban_columns`).

#### Minor (sample — full list in per-lens section)

- 🔵 **Architecture**: Asymmetric `work` vs `review_work` treatment conflates two failure modes (key-missing vs directory-missing); related to the major safety finding above
- 🔵 **Architecture**: Two distinct config endpoints (`/api/config` for wiki-link inner pattern + `/api/kanban/config`) with overlapping purpose risk version-skew — pin which endpoint serves which field
- 🔵 **Architecture**: `wip_canonicalise_id` reuse decision deferred to implementation — resolve before Phase 3 starts (port + parity test, or shell out)
- 🔵 **Architecture**: `IndexEntry.workItemRefs: Vec<String>` shape may need provenance (`{ kind, id }`) for Phase 3's per-key UI distinctions; commit upfront in Phase 1 to avoid mid-plan shape change
- 🔵 **Code Quality**: `api/docs.rs` handler accumulating responsibilities — extract `validate_kanban_status` helper
- 🔵 **Code Quality**: Test-rename-first wording risks transient non-compiling commits; reserve commit-boundary semantics for net-new behaviour (Steps 1.1, 2.1, 2.3, 3.1, 3.2), not mechanical renames
- 🔵 **Code Quality**: Phase 1's `work_item_refs_of` shim has a 0-line lifespan — land `read_ref_keys` directly in Phase 1
- 🔵 **Code Quality**: Add doc-comments distinguishing `IndexEntry.workItemRefs` (cross-refs to others) from `IndexEntry.workItemId` (this doc's own ID)
- 🔵 **Test Coverage**: `read_ref_keys` tests assert on a private helper signature; lift structural assertions to indexer-level integration tests
- 🔵 **Test Coverage**: Stderr substring match in `invalid_id_pattern_fails_compilation_with_clear_message` is too coupled to compiler vocabulary; assert structural properties only
- 🔵 **Test Coverage**: No assertion pins the SSE event `docType` literal value flipping from `tickets` to `work-items`
- 🔵 **Test Coverage**: ETag/conflict path under renamed wire shape lacks an explicit assertion
- 🔵 **Test Coverage**: Frontend rendering of mixed-kind `referencedBy` is untested
- 🔵 **Test Coverage**: Aggregation tests don't pin the order of aggregated `workItemRefs` — UI risks non-determinism
- 🔵 **Correctness**: Pattern recompilation invariant not just affects wiki-links — also produces stale reverse-index entries; document broader operator-observable failure
- 🔵 **Correctness**: Mixed-pattern precedence rule excludes legacy bare-numeric files under project-prefixed configs without an operator workflow; document the rename requirement or add a fallback admission rule
- 🔵 **Correctness**: YAML integer parsing under `{number:>=10d}` may cross i32 boundary; restrict configured width or document string-only canonical domain
- 🔵 **Compatibility**: `--compile-wiki-link` subcommand without confirming it ships in the plugin version; pin a version contract or commit to deriving from `--compile-scan`
- 🔵 **Compatibility**: `/accelerator:migrate` pointer in error messages assumes Claude-Code invocation context; add a fallback script path
- 🔵 **Documentation**: `wip_canonicalise_id` parity contract has no documented home — add a doc-comment on the bash function or an ADR-0025 subsection
- 🔵 **Documentation**: `ApiError::UnknownKanbanStatus` JSON envelope shape isn't documented; pin in ADR-0024 or Step 1.4b
- 🔵 **Documentation**: Existing CHANGELOG.md Unreleased lines (11, 23, 32-33, 39-40) need explicit before/after spec, not a vague "update existing references"
- 🔵 **Documentation**: Visualiser subsection placement under `skills/config/configure/SKILL.md` needs a precise insertion point
- 🔵 **Standards**: New `frontend/e2e/scenarios/` subdirectory introduces a layout convention without precedent
- 🔵 **Standards**: CHANGELOG `**BREAKING**:` prefix tagging diverges from existing Added/Changed/Notes structure — pick one
- 🔵 **Safety**: Legacy `proposed` becomes write-locked once columns configured; document recovery path (direct file edit / VCS revert) in ADR-0024
- 🔵 **Safety**: Workspace-clean precondition for fixture moves is described but not enforced — add a guard or `jj new` to land moves on a fresh change
- 🔵 **Safety**: Pattern-change-after-launch silent failure mode could be made loud via a config-fingerprint check; out of scope but worth noting

### Assessment

The plan has improved substantially. Of ~24 original major findings, **23 are fully resolved and 3 are partially resolved** (the partial ones intersect with new findings). However, the revision introduces 1 critical and ~11 major new issues, plus a long tail of minors — most concentrated around (a) the wiki-link / pattern-compiler grammar mismatch, (b) the `Completeness.has_ticket` semantic confusion, (c) the over-aggressive boot-time directory check, (d) under-specified canonicalisation edges, and (e) doc-deliverable gaps (`--compile-wiki-link` SKILL.md update, validation template frontmatter).

Recommended next steps:

1. **Fix the critical**: decide between dropping the multi-segment-project claim (matching the actual compiler grammar) or genuinely extending the pattern compiler with a class-based wiki-link mode. Either is fine; the plan must commit and the compiler-side work needs to be in scope if option (b).
2. **Reverse the `has_ticket` decision**: rename to `has_work_item` (single-pass, matches sibling fields), restoring cluster-presence semantics.
3. **Soften the boot-time check**: tolerate missing directory; require only key presence.
4. **Pin three under-specified edges**: dual-key-present resolution; canonicalisation under prefixed-input-default-pattern; PATCH-status-unchanged when existing value is in Other.
5. **Resolve doc gaps**: validation template frontmatter; `--compile-wiki-link` documentation; `config-read-value.sh` list-mode contract; `ApiError` envelope shape.
6. **Address the `todo` PATCH compatibility regression** in CHANGELOG (and decide whether to ship an alias bridge).

The remaining minors are largely polish and can be batched. A second revision pass should resolve everything except the 1 critical, which needs an explicit design decision.

---

## Re-Review (Pass 3) — 2026-05-04

**Verdict:** REVISE

The third revision resolves the critical from pass 2 (wiki-link grammar mismatch) and most of the pass-2 majors. The plan is now in substantially better shape: 0 critical, 6 major, ~30 minor. The remaining majors are mostly stale-text artefacts from the revision sweep (Testing Strategy still references the now-restored `has_work_item` rename as if it were dropped; Phase 3 still mentions removing a `work_item_refs_of` shim that Phase 1 no longer introduces; Step 3.1a contains an internal contradiction about `config-read-value.sh` that emerged from my own edit), one genuine new correctness gap (the case-3 vs case-4 boundary in the four-case canonicalisation rule is undefined for borderline inputs), and one user-deferred item the agents re-surface (the `todo` PATCH regression — explicitly deferred to the separate work-item status migration plan, but flagged here because users hit the symptom in the interim).

### Previously Identified Issues

#### Resolved (originally critical/major from pass 2)

- ✅ **Correctness (CRITICAL)**: Pattern compiler doesn't emit class-based regex — Resolved. Frontend builds the regex from the literal `default_project_code`; multi-segment codes are explicitly out of scope with a pinned negative test.
- ✅ **Code Quality**: `Completeness.has_ticket` semantics confused — Resolved. Renamed to `has_work_item`, preserving cluster-presence semantics parallel to siblings.
- ✅ **Compatibility**: Phase 1 widening drops `todo` — Partially resolved (deferred per user direction; see new finding below).
- ✅ **Standards**: `config-read-value.sh` is scalar-only — Resolved in approach (use jq on parsed YAML), but introduced a new contradiction in Step 3.1a — see new finding.
- ✅ **Documentation**: Validation frontmatter contract — Resolved. Cites `meta/plans/*.md` shape; template update declared out of scope.
- ✅ **Documentation**: `--compile-wiki-link` SKILL.md update — Resolved by abandoning the subcommand.
- ✅ **Safety**: Boot-time directory-must-exist too aggressive — Resolved. Only key presence required; missing on-disk tolerated.
- ✅ **Architecture**: Cross-process compile-time shell-out coupling — Acknowledged as deliberate tradeoff; agent suggests a fixture-snapshot pattern to soften the cost (now flagged as minor).
- ✅ **Architecture**: Boot-immutability invariant deserves ADR capture — Resolved. ADR-0024 now covers it (though agent notes this widens ADR-0024's scope; see new finding).
- ✅ **Correctness**: Round-trip-from-Other test name conflict — Resolved. Test renamed to `patch_status_seeds_in_other_then_moves_to_configured_column` with explicit comment.
- ✅ **Correctness**: Canonicalisation rule for prefixed input under default — Resolved. Case 3 added (pass-through verbatim).
- ✅ **Correctness**: Phase 1 dual-key resolution (both `ticket:` and `work-item:` present) — Resolved. `work-item:` wins; test added.
- ✅ **Architecture**: Asymmetric `work` vs `review_work` — Resolved. Both keys symmetric (key required; missing on-disk tolerated).

#### Partially resolved

- 🟡 **Compatibility**: `todo` PATCH regression — Deferred per user direction (separate work-item migration plan owns it). The agent flag remains valid for the interim period (see new minor finding below).
- 🟡 **Test Coverage**: SSE docType emission, ETag/conflict, mixed-kind referencedBy rendering, aggregation ordering — Deferred per user as polish; agents re-flag as residual.

#### Still present (none from pass 2 — all originally-identified issues from pass 1+2 have been addressed in some form)

### New Issues Introduced by Pass-3 Revisions

#### Critical

(none)

#### Major

- 🟡 **Code Quality**: Testing Strategy contradicts the restored `has_work_item` rename
  **Location**: Testing Strategy section, `clusters.rs` bullet
  Still reads "completeness derivation from `!workItemRefs.is_empty()` (no longer reads a dropped boolean field)" — the rename was reverted but this paragraph wasn't updated. Replace with "completeness derivation sets `has_work_item` when any cluster entry has `DocTypeKey::WorkItems`".

- 🟡 **Code Quality**: Phase 3 Step 3.2b still references dropping a `work_item_refs_of` shim that Phase 1 no longer introduces
  **Location**: Phase 3 Step 3.2b implementation
  After the revision landing `read_ref_keys` directly in Phase 1, the line "split `work_item_refs_of` into the pure `read_ref_keys` helper... Drop the existing `work_item_refs_of` shim" describes a refactor of a function that doesn't exist. Rewrite to: "extend `read_ref_keys` (landed in Phase 1) to also read `parent:` and `related:` keys, aggregating into the same `Vec<RawRef>`."

- 🟡 **Standards**: Internal contradiction in kanban_columns read approach (Step 3.1a)
  **Location**: Phase 3 Step 3.1a
  The opening paragraph says "`config-read-value.sh` is a scalar reader and does not handle YAML list values, so it cannot be used here. Read … directly via the YAML-aware tooling…" Six paragraphs later the implementation paragraph says "Implement the launcher read using `config-read-value.sh visualiser.kanban_columns '<defaults>'`". These two prescriptions contradict. Reconcile to one approach: either commit to inline-array form via `config-read-value.sh` + `config_parse_array` (which exists in `scripts/config-common.sh:72-85`), or commit to a new YAML-list reader.

- 🟡 **Standards**: Mischaracterisation of the existing config-read pipeline
  **Location**: Phase 3 Step 3.1a rationale
  The plan asserts "the existing config-read pipeline already parses YAML to JSON; reuse that parsed-JSON form via `jq`". This is incorrect: `config-read-value.sh` is awk-based, walking frontmatter line-by-line, returning a single scalar string per call. There is no YAML-to-JSON layer. `config_parse_array` splits inline `[a,b,c]` form via string ops; the launcher composes JSON via `jq -nc --arg ...`. Replace the rationale with an accurate statement of the existing primitives, or scope a new helper as an addition.

- 🟡 **Correctness**: Case 3 predicate (already-prefixed) is undefined; boundary with case 4 ambiguous
  **Location**: Phase 3 Step 3.2 canonicalisation rule
  The four-case rule specifies pass-through for "already project-prefixed input (e.g. `PROJ-0042`)" but does not define the predicate distinguishing case 3 from case 4. Inputs like `FOO-1` (token+digits), `42-foo` (digits+token), `PROJ-` (token+empty), `-0042` (empty+digits) are unclassified. Specify the predicate explicitly (e.g. "matches `^[A-Za-z][A-Za-z0-9]*-\d+$`") and add boundary tests.

- 🟡 **Compatibility**: Legacy `todo` status PATCH-stuck-in-Other not surfaced (deferred per user direction)
  **Location**: Phase 4 Step 4.3 CHANGELOG
  Per user direction this is deferred to a separate work-item status migration plan. Re-surfaced here as a "pre-existing user-visible interim state" — between when this plan ships and when the migration ships, users with legacy `status: todo` files will see them rendered in Other and unable to be PATCHed back. The user's framing (visualiser is prerelease; only user-visible additions in CHANGELOG) is internally consistent and resolves this concern; flagging only because the agent observed the user-visible symptom would benefit from a brief operator-facing note in the CHANGELOG ("legacy `todo`/`proposed` values appear in Other; round-trip support arrives with the work-item status migration") if you want to triangulate user expectations. **No action required if the deferred-to-separate-plan framing is accepted.**

#### Minor (sample — full list in per-lens section)

- 🔵 **Architecture**: ADR-0024 now bundles two concerns (kanban columns + boot-immutability invariant) — split or rename
- 🔵 **Architecture**: `work_item_by_id` key type silently widens between phases (`HashMap<u32>` → `HashMap<String>`); document or enumerate call-site impact in Phase 2
- 🔵 **Architecture (suggestion)**: Launcher→server `config.json` contract has no schema; consider a shared fixture
- 🔵 **Architecture (suggestion)**: `GET /api/kanban/config` could be consolidated with `/api/config` (the `default_project_code` flow goes through `/api/config`)
- 🔵 **Code Quality**: `RawRef` type used in tests/signatures but never defined — add a one-line definition at first introduction
- 🔵 **Code Quality**: `validate_kanban_status` placement (`api/docs.rs` vs `api/validation.rs`) left to implementer
- 🔵 **Code Quality**: Mixed-pattern fallback admission could factor into a single `canonicalise_id` function reused by indexer + cross-ref canonicaliser (DRY)
- 🔵 **Code Quality**: Both-keys-present silent precedence — consider warn-level log naming the file when both are present with different values
- 🔵 **Test Coverage**: Move YAML-shape robustness tests (null, scalar-where-array, etc.) into Phase 1 alongside dual-key test, since `read_ref_keys` lands in destination shape in Phase 1
- 🔵 **Test Coverage**: Tighten fail-fast assertions to require both the missing-key name AND the `/accelerator:migrate` recovery pointer in stderr
- 🔵 **Test Coverage**: Mixed-pattern admission fixture sprawl risk; either parameterise or comment-link siblings
- 🔵 **Correctness**: Case-3 pass-through under default pattern preserves cross-refs that can never resolve — narrow case 3 to project-prefixed patterns only, or update rationale
- 🔵 **Correctness**: `WorkItemConfig::default()` referenced in tests but no `Default` impl shown (regex doesn't auto-derive Default); add explicit `default_for_tests()`
- 🔵 **Correctness**: Race between `/api/kanban/config` and `/api/docs/work-items` not specified; pin loading-state behaviour
- 🔵 **Correctness**: Aggregation ordering not pinned (`HashSet` vs `IndexSet`) — define and assert
- 🔵 **Compatibility**: Wiki-link inner pattern narrower than prior `[A-Za-z][A-Za-z0-9-]*-\d+` — extend Step 1.7 grep to catch any prerelease references with multi-segment codes
- 🔵 **Compatibility**: Confirm `Completeness.hasTicket` rename has no cross-skill consumers via plugin-tree grep (audit step in Step 1.7)
- 🔵 **Documentation**: `RawRef` type and `read_ref_keys` signature need definition at first introduction
- 🔵 **Documentation**: Mixed-pattern fallback admission rule has user-facing implications not surfaced in CHANGELOG or ADR-0025
- 🔵 **Documentation**: Add a "Type-shape trajectory" table at top of Phase 1 listing each renamed identifier × phase × type
- 🔵 **Documentation**: CHANGELOG could quote the launcher's pre-migration exit message skeleton for triage
- 🔵 **Documentation (suggestion)**: `wip_canonicalise_id` parity contract has no documented home; add a section to ADR-0025 outline
- 🔵 **Standards**: Repeated `**Visualiser**:` prefix in CHANGELOG diverges from existing topic-led bullet style; nest under existing `**Meta visualiser**` topic or use topic-led bolding
- 🔵 **Standards**: `read_ref_keys` name ambiguous (returns values, not keys) — consider `read_refs` or `cross_refs_of`
- 🔵 **Safety**: Tolerated missing-on-disk work directory has no operator-facing surface signal — empty kanban indistinguishable from "no work-items yet"
- 🔵 **Safety**: Pattern reconfiguration could leave stale cross-refs admitted under prior pattern's fallback rule (deferred config-fingerprint check would catch this)
- 🔵 **Safety**: Fixture rename has no working-copy-clean precondition guard (deferred per user as polish)

### Assessment

The plan is now in good shape. Of pass-2's 1 critical + 11 majors, **all are resolved or deferred per explicit user direction**. The 6 new majors are predominantly stale-text artefacts from the revision sweep (Testing Strategy, Phase 3 shim mention, Step 3.1a contradiction + mischaracterisation) and one genuine correctness gap (case-3 predicate). These are mechanical fixes, not design changes.

Recommended next steps:

1. **Update Testing Strategy** `clusters.rs` bullet to reflect the restored `has_work_item` rename.
2. **Rewrite Phase 3 Step 3.2b** `frontmatter.rs` bullet to drop the shim-removal language; describe Phase 3 as extending `read_ref_keys` to read `parent:`/`related:`.
3. **Reconcile Step 3.1a**: pick one approach for reading `visualiser.kanban_columns` (recommend `config-read-value.sh` + `config_parse_array`, since both already exist) and remove the contradictory text + the inaccurate "parses YAML to JSON" rationale.
4. **Pin the case-3 predicate** in the canonicalisation rule (e.g. `^[A-Za-z][A-Za-z0-9]*-\d+$`) and add boundary tests.
5. **Optional polish**: define `RawRef` at first introduction, pick a single home for `validate_kanban_status`, narrow case-3 to project-prefixed patterns only (or update the rationale), add the CHANGELOG note about the legacy `todo`/`proposed` interim state, log warn-level on dual-key conflict.

After these mechanical fixes a third revision pass should land at COMMENT or APPROVE. The deferred minors (test polish, config-fingerprint check, parity-contract section in ADR-0025, fixture-clean precondition) are appropriately scoped to follow-up work.

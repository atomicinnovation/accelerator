---
date: "2026-04-18T17:30:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-08-ticket-management-phase-1-foundation.md"
review_number: 2
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability]
review_pass: 2
status: complete
---

## Plan Review: Ticket Management Phase 1 — Foundation and Configuration

**Verdict:** REVISE

The three new issues raised by review-1 pass 2 (Test 11 description, `config_enumerate_templates` assertion updates, `config-dump.sh` `PATH_KEYS`/`PATH_DEFAULTS` updates for `review_tickets`) are all resolved in the current plan. However, reviewing fresh through six lenses has surfaced several additional concerns the prior pass did not catch or chose to defer: a numbering invariant that breaks at 10000, a schema divergence with all 25 existing tickets that has now been elevated to major across three lenses, missing test coverage for the new `paths.review_tickets` and `templates.ticket` rows in `config-dump.sh`, and an opportunity to consolidate frontmatter parsing onto the existing `config_extract_frontmatter` helper rather than entrenching a third copy.

### Cross-Cutting Themes

- **Schema divergence with the 25 existing tickets** (flagged by: Architecture, Standards, Usability) — The proposed `templates/ticket.md` introduces an 8-field frontmatter (`ticket_id`, `date`, `author`, `type`, `status`, `priority`, `parent`, `tags`) and an enumerated type (`story|epic|task|bug|spike`) that is disjoint from the schema all 25 existing tickets use (`title`, `type: adr-creation-task`, `status`). The plan declines to migrate the existing tickets but provides no schema-detection convention or contract for what `ticket-read-field.sh` consumers can rely on finding.

- **Numbering invariant breaks at 10000** (flagged by: Correctness, Code Quality, Usability) — The scanning glob `[0-9][0-9][0-9][0-9]-*` requires exactly four digits followed by a dash, but `printf "%04d"` produces 5+ digit strings when the count exceeds 9999 (Test 10 explicitly asserts `10000` as valid output). A 5-digit ticket file is invisible to subsequent scans, so the script will hand out colliding numbers.

- **Duplicate-key behaviour is implicit and untested** (flagged by: Architecture, Code Quality, Test Coverage, Correctness) — The frontmatter parser does not `break` on first match, so duplicate keys silently resolve to the last occurrence. Neither the script header nor the test enumeration documents which semantics is contract.

- **Hardcoded enumeration coordination tax** (flagged by: Architecture, Standards, Test Coverage) — At least six locations must be updated in lockstep to add a single template key, and three locations for a path key. The plan reinforces this duplication; `config_enumerate_templates` already provides a dynamic source of truth that `config-dump.sh` does not use.

- **Empty `skills/tickets/` directory registration** (flagged by: Architecture, Code Quality, Usability) — Registering `./skills/tickets/` in `plugin.json` before any `SKILL.md` exists creates a window where the manifest advertises a skill category that contributes nothing. The plan's claim that this is "harmless" is unverified.

- **Delegated status reader loses the allowed-values hint** (flagged by: Correctness, Usability) — `adr-read-status.sh` emits an enum hint on the error path; `ticket-read-status.sh` (now an `exec` wrapper) cannot. The error-path postcondition has changed, undermining the stated goal of pattern parity with the ADR variant.

### Tradeoff Analysis

- **Pattern consistency vs DRY consolidation**: The plan continues to mirror the ADR pattern (separate companion scripts per category, separate test harnesses, hardcoded key arrays in `config-dump.sh`) rather than refactoring toward shared infrastructure (`config_extract_frontmatter`, dynamic key enumeration, shared test helpers). This is a defensible Phase 1 choice — pattern consistency lowers the barrier to implementation and keeps the change diff narrow — but every pass through this plan accumulates more "we know this is duplicated" debt. Worth deciding now whether Phase 2+ will consolidate or whether the duplication is permanent.

- **Schema flexibility vs lifecycle contracts**: The new ticket template adds rich fields (`priority`, `parent`, `tags`) that downstream skills will want, but creates a corpus where common queries fail on legacy tickets. Either accept that new tickets are a different "kind" (and document it), or trim the template to the intersection of fields actually present everywhere.

### Findings

#### Major

- 🟡 **Architecture, Standards, Usability**: New ticket template schema diverges from all 25 existing tickets without a bridge contract
  **Location**: Subphase 1C, Section 1: templates/ticket.md
  Existing tickets use `title`/`type: adr-creation-task`/`status`; the new template uses an 8-field schema with a different `type` enumeration and replaces `title` with the heading. Downstream skills built on `ticket-read-field.sh` will need ad-hoc per-schema branching, and `priority`/`parent`/`tags` queries will fail silently on legacy tickets. The plan declines migration but provides no schema-detection convention. (Previously raised as a minor in review-1; elevated here because three lenses independently flagged it as major.)

- 🟡 **Correctness, Code Quality, Usability**: Numbering invariant breaks at the 10000 boundary
  **Location**: Subphase 1A, Section 2 (ticket-next-number.sh) and Test 10
  `printf "%04d"` happily emits `10000`, but the scanning glob `[0-9][0-9][0-9][0-9]-*` requires exactly four digits followed by `-`, so a `10000-foo.md` file is invisible to subsequent runs. `HIGHEST` regresses to the last 4-digit ticket and the script hands out a colliding number. Test 10 currently encodes the bug as intended behaviour. Either clamp/error at 9999, widen the glob and `printf` width, or document the upper bound explicitly.

- 🟡 **Code Quality**: Frontmatter parser reimplemented instead of reusing `config_extract_frontmatter`
  **Location**: Subphase 1B, Section 3 (ticket-read-field.sh implementation)
  `scripts/config-common.sh` already exports `config_extract_frontmatter`, which handles closure validation and warning emission for malformed frontmatter. The proposed script reimplements the state machine from scratch — a third copy alongside `adr-read-status.sh` and the existing helper. Consolidating onto the helper would centralise the parsing invariant and let `ticket-read-field.sh` focus on field lookup.

- 🟡 **Test Coverage**: No test for `paths.review_tickets` resolution in `test-config.sh`
  **Location**: Subphase 1C, Section 7 (Config test updates)
  Every other path key has a dedicated test in `scripts/test-config.sh` (e.g., `paths.review_plans` ~line 2004, `paths.review_prs` ~line 2016, `paths.tickets` ~line 2040). The plan adds `review_tickets` to the path-key plumbing but does not add a parallel test, leaving silent regression risk for the new key.

- 🟡 **Test Coverage**: No assertion that `config-dump.sh` emits the new `templates.ticket` and `paths.review_tickets` rows
  **Location**: Subphase 1C, Section 7 (Config test updates)
  There is precedent for this kind of test at line ~3094 (`config-dump.sh pr-description template key`). Without analogous assertions, accidental removal of the new entries in the hardcoded `TEMPLATE_KEYS` / `PATH_KEYS` arrays would not be caught.

- 🟡 **Architecture**: Hardcoded template/path key enumeration is a growing coordination hotspot
  **Location**: Subphase 1C, Section 6 (config-dump.sh updates)
  Six locations must be updated to add a single template key (`config-read-template.sh` comment, `config-dump.sh` TEMPLATE_KEYS, `configure/SKILL.md` table + example + keys list, `README.md` keys list, multiple `test-config.sh` assertions); three for a path key. `config_enumerate_templates` already provides a dynamic enumeration that `config-dump.sh` does not use. Either schedule the consolidation in a follow-on phase or explicitly accept the debt in "What We're NOT Doing".

- 🟡 **Usability**: Template placeholders require manual substitution with no tooling guidance
  **Location**: Subphase 1C, Section 1 (templates/ticket.md)
  `ticket_id: NNNN`, `date: "YYYY-MM-DDTHH:MM:SS+00:00"`, `author: Author Name`, and `# NNNN: Title as Short Noun Phrase` must all be hand-filled. There is no Phase 1 guidance for what helper commands to run before Phase 2's create-ticket skill ships. A first-time author either fills placeholders inconsistently or abandons the template.

#### Minor

- 🔵 **Architecture, Code Quality, Test Coverage, Correctness**: Duplicate-field behaviour is implicit (last-wins) and untested
  **Location**: Subphase 1B, Section 3 (ticket-read-field.sh)
  The loop does not `break` on first match, so duplicate keys silently resolve to the last occurrence. Neither the script header nor the test enumeration captures this contract. Add `break` (first-wins, consistent with `config-read-value.sh`) and a test, or document the current last-wins behaviour and pin it with a test.

- 🔵 **Architecture, Code Quality, Usability**: Empty `skills/tickets/` directory registration is unverified
  **Location**: Subphase 1C, Section 2 (plugin.json)
  The plan asserts this is "harmless" without verifying that the plugin loader emits no warnings for empty skill directories. Either add a smoke check, defer registration to Phase 2, or add a stub `README.md` explaining that skills land in Phase 2.

- 🔵 **Correctness, Usability**: Delegated status reader loses the allowed-values hint
  **Location**: Subphase 1B, Section 2 (ticket-read-status.sh)
  `adr-read-status.sh` emits a second stderr line listing valid status values; the new `exec` wrapper cannot. Either re-emit the hint before delegating, or accept the regression and note the deliberate divergence.

- 🔵 **Code Quality**: Triple-piped sed with embedded quote juggling is hard to read and tolerates mismatched quotes
  **Location**: Subphase 1B, Section 3 (ticket-read-field.sh value cleanup)
  Three `sed` invocations with cascaded quote escaping; mismatched input like `"draft'` is silently coerced to `draft`. Collapse to a single `sed -e ... -e ...` or mirror `config-read-value.sh`'s awk-based quote-aware stripping. Add a test pinning mismatched-quote behaviour.

- 🔵 **Code Quality**: Automated verification hard-codes `0026`
  **Location**: Subphase 1A, Success Criteria
  `Returns 0026 when run from the repo root (25 existing tickets)` drifts the moment anyone adds or removes a ticket. Soften to "matches `^[0-9]{4}$`" or move to manual verification with a point-in-time note.

- 🔵 **Code Quality**: Stricter ticket glob than ADR glob is undocumented
  **Location**: Subphase 1A, Section 2 (ticket-next-number.sh)
  ADR uses `ADR-[0-9][0-9][0-9][0-9]*` (any trailing char); ticket uses `[0-9][0-9][0-9][0-9]-*` (dash required). Add a comment explaining the two invariants and consider a test for the no-dash case `0001.md`.

- 🔵 **Architecture**: Sequential number allocation is not concurrency-safe
  **Location**: Subphase 1A, Section 2 (ticket-next-number.sh)
  Two concurrent invocations will receive the same number, causing collisions when both callers create files. Same characteristic as the ADR script, but more plausible for tickets given multi-agent extraction. Document the limitation in "What We're NOT Doing".

- 🔵 **Test Coverage**: Two stale 5-template assertions remain
  **Location**: Subphase 1C, Section 7 (Config test updates)
  Line ~3107 (`echo "Test: Unknown template still lists all 5 template names..."`) becomes misleading after the change, and the `Unknown template name` test at ~line 2269 does not add an assertion that `ticket` is present in the error output.

- 🔵 **Test Coverage**: No test for prefix-match defensiveness
  **Location**: Subphase 1B (ticket-read-field.sh test enumeration)
  The defensive `[[ "$line" == "${PREFIX}"* ]]` design is motivated against metacharacter injection and prefix collisions but no tests cover (a) `tag` query not matching `tags:`, (b) field name with dots/brackets, (c) indented nested key not matching the top-level key.

- 🔵 **Test Coverage**: No automated check that init skill's directory count matches the actual list
  **Location**: Subphase 1C, Section 4 (init/SKILL.md update)
  The plan updates the count from 11 to 12 but `grep -q 'review_tickets'` passes even if the count sentence stays at 11. Add an assertion that the count text and the directory-line count agree.

- 🔵 **Standards**: New script uses different frontmatter-parsing style than ADR sibling without a marker
  **Location**: Subphase 1B, Section 3 (ticket-read-field.sh)
  ADR uses `grep -qE`/`sed`; ticket uses bash `[[ == ]]` with parameter expansion. The plan justifies the divergence in prose but the script header doesn't reference the convention source (`config-read-value.sh`). Add a brief comment.

- 🔵 **Standards**: PATH_KEYS/PATH_DEFAULTS ordering convention is unstated
  **Location**: Subphase 1C, Section 6b (config-dump.sh updates)
  The plan inserts `review_tickets` after `review_prs` (grouped) but the existing array also omits `tmp` (which `config-read-path.sh` documents). Document the ordering convention explicitly and decide whether to fix the `tmp` omission as part of this phase.

- 🔵 **Standards**: Init skill update misses the Path Resolution section in addition to Step 4 report
  **Location**: Subphase 1C, Section 4 (init/SKILL.md update)
  The plan shows the Path Resolution snippet but only explicitly mentions Step 4 report insertion. A literal reading would update count and report but miss adding the `config-read-path.sh review_tickets` line to Path Resolution. Make both insertion points explicit.

- 🔵 **Correctness**: Quote stripper tolerates mismatched quote pairs
  **Location**: Subphase 1B, Section 3 (ticket-read-field.sh value cleanup)
  Strips one leading and one trailing quote independently, regardless of whether they match. ADR equivalent shares this behaviour; widen to matched-pair stripping or document.

- 🔵 **Usability**: Array values returned raw will surprise callers
  **Location**: Subphase 1B (Test 7 for ticket-read-field.sh)
  `tags: [a, b]` returns the raw bracketed string. Note in the script header that array values are returned as raw YAML; callers parse them themselves.

- 🔵 **Usability**: Silently ignoring 5-digit-prefixed files is confusing
  **Location**: Subphase 1A (Test 11)
  A typo like `00003-foo.md` is silently dropped. Optionally emit a stderr warning when files matching a looser pattern exist but are skipped.

#### Suggestions

- 🔵 **Architecture**: Test updates rely on brittle line-number references
  **Location**: Subphase 1C, Section 7
  Anchors like `~3122`, `~3128`, `~3344` will drift. Replace with content-anchored descriptions ("append to the loop ending in `pr-description`").

- 🔵 **Code Quality**: Wrapper pattern asymmetry with `adr-read-status.sh`
  **Location**: Subphase 1B, Section 2
  Either add a header comment in `ticket-read-status.sh` explaining the divergence from the ADR variant, or file a follow-up to refactor ADR onto the same shared primitive.

- 🔵 **Test Coverage**: Test 11 comment phrasing is slightly confusing
  **Location**: Subphase 1A (Test 11)
  Reword the rationale (`position 5 is '3', not '-'`) to match the actual filename indices (`00003-foo.md` has `0` at position 5, `3` at position 4) so the narrative stays accurate.

- 🔵 **Standards**: Available template keys list is duplicated across three places
  **Location**: Subphase 1C, Sections 5c and 6c
  Add a "What We're NOT Doing" note acknowledging the future consolidation onto `config_enumerate_templates`.

- 🔵 **Usability**: Missing-directory warning could suggest the init command
  **Location**: Subphase 1A, Section 2 (ticket-next-number.sh)
  Extend the warning to: `... Run /accelerator:init to create it.` Closes the DX loop on first run.

### Strengths

- ✅ All three issues from review-1 pass 2 (Test 11 description, `config_enumerate_templates` assertions, `config-dump.sh` PATH_KEYS/PATH_DEFAULTS) are resolved in the current plan
- ✅ Excellent structural consistency with the established decisions/ category — script naming, directory layout, plugin registration, and configuration mirror proven patterns
- ✅ `ticket-read-status.sh` cleanly delegates to `ticket-read-field.sh` via `exec`, avoiding state-machine duplication that exists in the ADR equivalent
- ✅ Defensive bash prefix-matching (`[[ "$line" == "${PREFIX}"* ]]`) avoids the regex-injection hazard the prior review pass flagged in earlier drafts
- ✅ Strong TDD ordering with ~37 enumerated test cases covering happy paths, error paths, boundary conditions, and invalid arguments
- ✅ `--count` value guard (`[ $# -lt 2 ]`) produces a clear error rather than an opaque `set -u` diagnostic
- ✅ Argument-order rationale for `ticket-read-field.sh` (verb-then-target) is explicitly documented against Unix convention
- ✅ Cross-file enumeration of every place template/path keys are hardcoded shows the author traced the full update surface
- ✅ Clear "What We're NOT Doing" section prevents scope creep with explicit rationale for each exclusion

### Recommended Changes

1. **Decide the 10000 overflow contract** (addresses: numbering invariant breaks at 10000)
   Either widen the glob and `printf` width to support 5+ digit prefixes, clamp at 9999 with an explicit error, or document the upper bound. Update Test 10 to assert the chosen invariant rather than encoding the current bug.

2. **Reconcile the template schema with existing tickets** (addresses: schema divergence)
   Either (a) trim the template to the field intersection actually present in legacy tickets and treat richer fields as future enhancements, or (b) explicitly document the two-schema corpus in the template + plan + configure skill, including which fields are guaranteed-present-on-all-tickets so downstream consumers have a clear contract.

3. **Add `paths.review_tickets` and `templates.ticket` test assertions to `test-config.sh`** (addresses: missing test coverage)
   Mirror the existing `paths.review_prs` resolution test and the `pr-description` `config-dump.sh` row test for the two new keys.

4. **Consolidate frontmatter parsing onto `config_extract_frontmatter`** (addresses: parser reimplementation)
   Have `ticket-read-field.sh` source `config-common.sh` and delegate to the existing helper for frontmatter extraction, then run field lookup against the extracted block. Either include `adr-read-status.sh` migration in this phase or schedule it as a follow-up.

5. **Pin the duplicate-key behaviour** (addresses: duplicate-field semantics undocumented)
   Decide between `break`-on-first-match (consistent with `config-read-value.sh`) or last-match-wins (current behaviour, consistent with ADR script). Document in the script header and add a test pinning the chosen semantics.

6. **Verify the empty-directory plugin registration is genuinely silent** (addresses: empty skills directory)
   Either run a plugin-loading smoke check and add it to Success Criteria, defer the `plugin.json` change to Phase 2, or add a stub `skills/tickets/README.md` explaining the timing.

7. **Soften the hardcoded `0026` success criterion** (addresses: drifting verification)
   Replace with `output matches ^[0-9]{4}$` or move to a clearly-marked manual point-in-time check.

8. **Restore the allowed-values hint on `ticket-read-status.sh` errors** (addresses: error-message regression)
   Have the wrapper trap the missing-field case (or accept the regression and note the deliberate divergence in the plan).

9. **Make Path Resolution insertion explicit in init/SKILL.md update** (addresses: init skill update gap)
   Spell out both the Path Resolution line addition and the Step 4 report addition; add an assertion that the directory count matches the resolved-paths line count.

10. **Replace line-number anchors with content anchors** (addresses: brittle line references)
    Update all `around line N` references in Subphase 1C to content-based anchors that survive code drift.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-grounded in the existing decisions/ category pattern and cleanly separates three concerns (numbering script, field extraction scripts, and config/template wiring) into dependency-ordered subphases. Module boundaries, path-key delegation, and the decision to factor a reusable `ticket-read-field.sh` with a thin `ticket-read-status.sh` wrapper are architecturally sound. The main architectural concern is the hardcoded key-enumeration coupling across several scripts (config-dump, README, configure SKILL, test-config) for both paths and templates — the plan propagates the existing pattern rather than mitigating it, which is pragmatic for Phase 1 but should be acknowledged as a growing coordination tax.

**Strengths**:
- Subphases dependency-ordered with clear rationale
- `ticket-read-field.sh` as primitive with `ticket-read-status.sh` as thin wrapper avoids state-machine duplication present in ADR pattern
- Path-key delegation through `config-read-path.sh` preserves three-tier resolution contract
- Bash prefix matching (`[[ ... == "${PREFIX}"* ]]`) defensively avoids metacharacter injection
- Plan acknowledges legacy tickets coexist; glob designed to match existing convention
- Plugin.json registration upfront avoids subsequent config churn

**Findings**:

**Major — New ticket template schema diverges from legacy ticket schema without an abstraction to bridge them** (confidence: high)
Location: Subphase 1C, Section 1: templates/ticket.md and the research document (Section 6)

Proposed `templates/ticket.md` defines frontmatter disjoint from the schema used by all 25 existing tickets. The plan declines migration but no architectural bridge is defined — no versioning field, no schema-detection convention, no documented contract for what ticket-reading scripts can rely on finding. `ticket-read-field.sh` will silently exit 1 for missing fields, requiring downstream skills to do per-schema branching.

**Major — Hardcoded template/path key enumeration is already a coordination hotspot and the plan propagates the duplication** (confidence: high)
Location: Subphase 1C, Section 6b: config-dump.sh TEMPLATE_KEYS/PATH_KEYS/PATH_DEFAULTS arrays

Six locations must be updated in lockstep to add a single template key and three for a path key. `config_enumerate_templates` already provides dynamic enumeration that `config-dump.sh` does not use. Either schedule consolidation or explicitly accept the debt in "What We're NOT Doing".

**Minor — Generic field reader does not short-circuit on first match, so later body values could leak if frontmatter bounds are not respected** (confidence: high)
Location: Subphase 1B, Section 3: ticket-read-field.sh

Loop does not `break` after setting `FOUND_FIELD=true` — last occurrence wins. Mirrors ADR precedent but worth documenting or fixing for the new generic primitive.

**Minor — Registering an empty skills/tickets/ directory creates a temporary architectural inconsistency** (confidence: medium)
Location: Subphase 1C, Section 2: plugin.json skill registration

Manifest advertises a skill category that contributes nothing until Phase 2. Establishes a new "scripts-only registered skill directory" category.

**Minor — Sequential number allocation is not concurrency-safe, and no filesystem locking is proposed** (confidence: high)
Location: Subphase 1A, Section 2: ticket-next-number.sh

Two concurrent invocations receive the same number. ADR has same characteristic but tickets more plausibly involve multi-agent batch extraction.

**Suggestion — Test updates that rely on brittle line-number references will drift** (confidence: medium)
Location: Subphase 1C, Section 7

Line numbers will be stale by implementation time. Replace with content-anchored descriptions.

### Code Quality

**Summary**: The plan is well-structured, pragmatically scoped, and models itself on an established codebase pattern (the ADR companion scripts), which will make the resulting code easy for the next maintainer to read. The decision to introduce a generic `ticket-read-field.sh` and have `ticket-read-status.sh` delegate to it is a clear DRY improvement over the ADR precedent. The main code-quality concerns are around duplicated frontmatter parsing (the codebase already has a `config_extract_frontmatter` helper that is being bypassed), a fragile three-stage `sed` pipeline for value cleanup, and a handful of edge-case behaviours (duplicate keys, mismatched quotes, empty arrays, files without a trailing dash) that are not explicitly decided or tested.

**Strengths**:
- Mirrors established `skills/decisions/scripts/` pattern exactly
- `ticket-read-field.sh` with `ticket-read-status.sh` wrapper removes state-machine duplication
- Plan calls out rationale for non-obvious choices (defensive string comparison, `--count` guard, frontmatter closure)
- Test enumeration is specific and covers happy/error/edge paths
- "What We're NOT Doing" crisply scopes the phase

**Findings**:

**Major — Frontmatter parsing reimplemented instead of reusing config_extract_frontmatter** (confidence: high)
Location: Subphase 1B, Section 3: ticket-read-field.sh

`scripts/config-common.sh` already exports `config_extract_frontmatter` with closure validation. Reimplementing creates a third copy. Have `ticket-read-field.sh` source `config-common.sh` and delegate.

**Minor — Triple-piped sed with embedded quote juggling is hard to read** (confidence: high)
Location: Subphase 1B, Section 3: ticket-read-field.sh

Three sed invocations with cascaded quote escaping; mismatched input is silently coerced. Collapse to single sed or mirror awk-based stripping from `config-read-value.sh`.

**Minor — Duplicate-key behaviour is implicit (last-wins) and untested** (confidence: medium)
Location: Subphase 1B, Section 3: ticket-read-field.sh

Loop continues after match. Document in header and add a pinning test.

**Minor — Automated verification hard-codes '0026', which will break as tickets grow** (confidence: high)
Location: Subphase 1A, Success Criteria

Soften to regex match or move to manual verification.

**Minor — Stricter ticket glob than ADR glob is an undocumented behavioural difference** (confidence: medium)
Location: Subphase 1A, Section 2: ticket-next-number.sh

Add comment explaining the two invariants (four-digit prefix, dash separator). Consider testing the no-dash case.

**Suggestion — Wrapper pattern diverges from adr-read-status.sh, creating sibling-script asymmetry** (confidence: medium)
Location: Subphase 1B, Section 2: ticket-read-status.sh

Add explanatory header comment or schedule ADR refactor onto a shared primitive.

**Suggestion — Registering an empty skills directory should be verified, not assumed harmless** (confidence: low)
Location: Subphase 1C, Section 2: Plugin registration

Add smoke verification or defer registration to Phase 2.

### Test Coverage

**Summary**: The plan takes a strongly test-first approach with 37 tests across three new scripts, mirroring and expanding the proven ADR test-harness pattern. Coverage is proportional and largely thorough, but several meaningful gaps remain — notably the absence of a dedicated test for the newly added `paths.review_tickets` key, no tests verifying that `config-dump.sh` emits the new `templates.ticket` and `paths.review_tickets` rows, and no regression test that `ticket-read-field.sh` correctly rejects indented/namespaced keys.

**Strengths**:
- Subphase 1A enumerates 13 concrete test cases including edge cases
- Test-first ordering explicit; harness reuses established decisions-script pattern correctly
- Subphase 1C enumerates specific line-number updates to test-config.sh
- Mutation-resistance solid for frontmatter parsing

**Findings**:

**Major — Missing test for paths.review_tickets in config-read-path.sh** (confidence: high)
Location: Subphase 1C, Section 7: Config test updates

Every other path key has a dedicated test. Add a `Test: paths.review_tickets configured` block following the `paths.review_prs` pattern.

**Major — No test for new config-dump.sh rows (templates.ticket, paths.review_tickets)** (confidence: high)
Location: Subphase 1C, Section 7: Config test updates (config-dump.sh)

Precedent at line ~3094 for `pr-description`. Add analogous assertions for the two new keys.

**Minor — Two stale 5-template assertions not updated** (confidence: high)
Location: Subphase 1C, Section 7a-7e: Config test updates

Line ~3107 `echo` description becomes misleading; line ~2269 error-message assertion does not check `ticket`. Update label and add the missing assertion.

**Minor — No test for defensive prefix-match behaviour when field name appears indented or as substring** (confidence: medium)
Location: Subphase 1B: Ticket Field Reading Scripts

Add tests for: (a) indented nested key not matching top-level, (b) `tag` not matching `tags:`, (c) field name with dot/bracket.

**Minor — Duplicate-field behaviour (last-wins) is untested and undocumented** (confidence: medium)
Location: Subphase 1B, Section 3: ticket-read-field.sh

Decide first-wins vs last-wins; pin with a test.

**Minor — No automated test verifying the init skill directory count text matches reality** (confidence: high)
Location: Subphase 1C, Section 4 and 6: Init skill and README updates

Success Criteria's `grep -q 'review_tickets'` passes even if count stays at 11. Add a count-vs-list assertion.

**Suggestion — Glob-boundary test comment asserts behaviour that depends on glob semantics worth double-checking** (confidence: medium)
Location: Subphase 1A, Test 11 and Implementation Approach

Reword the rationale text in Test 11 to match actual filename indices.

### Correctness

**Summary**: The plan is generally well-reasoned from a correctness perspective, with explicit attention to edge cases (empty directory, missing frontmatter, unclosed frontmatter, argument validation, leading-zero arithmetic, glob specificity). However, there is a silent invariant breakage when the ticket count crosses the 9999 → 10000 boundary: the script prints 5+ digits but the scanning glob `[0-9][0-9][0-9][0-9]-*` requires exactly four digits, so subsequent runs will not observe 5-digit-numbered tickets and will return stale or colliding numbers. A secondary concern is that the `ticket-read-status.sh` wrapper loses the domain-specific error hint (allowed status values) that the ADR pattern surfaces.

**Strengths**:
- Explicit `--count` missing-value guard
- Validation of `--count` regex correctly rejects zero/negative/non-numeric
- `$((10#$NUM))` prevents octal interpretation
- Test 11 correctly analyses glob boundary
- Defensive frontmatter parser requires both closure and field-found
- Bash quoted-prefix pattern correctly defensive against metacharacter injection
- Test 13 (body field) correctly handled by `break` on closing `---`

**Findings**:

**Major — Numbering invariant breaks at 10000 because glob requires exactly 4 digits** (confidence: high)
Location: Subphase 1A, ticket-next-number.sh implementation + Test 10

`printf "%04d"` emits 5+ digit strings beyond 9999, but the scanner glob requires exactly 4 digits + dash. 5-digit tickets are invisible on subsequent runs, causing collisions. Test 10 currently encodes the bug as intended behaviour.

**Minor — Delegated status reader loses the allowed-values hint on the error path** (confidence: high)
Location: Subphase 1B, ticket-read-status.sh delegation to ticket-read-field.sh

ADR script's enum-listing second stderr line is lost. Either re-emit or note the deliberate divergence.

**Minor — Duplicate field in frontmatter silently returns the last occurrence** (confidence: medium)
Location: Subphase 1B, ticket-read-field.sh — repeated field handling

Document chosen semantics in script header; add pinning test.

**Minor — Quote stripper tolerates mismatched quote pairs** (confidence: medium)
Location: Subphase 1B, ticket-read-field.sh — quote-stripping sed pipeline

Strips one leading and one trailing quote independently. Document or tighten to matched-pair stripping.

**Suggestion — No reservation/locking around next-number allocation** (confidence: medium)
Location: Subphase 1A, ticket-next-number.sh — concurrent invocation

TOCTOU between read-max and file-create. Document non-safety, or add `--reserve` flag in a future phase.

**Suggestion — Missing boundary tests for glob ambiguity around the 4-digit prefix** (confidence: medium)
Location: Subphase 1A, test enumeration (Test 11 / Test 12)

Add tests for `0001.md` (no dash), `12345-foo.md` (5-digit prefix), and the glob-expands-to-itself `-e` guard.

### Standards

**Summary**: The plan closely mirrors the established decisions/ADR pattern for companion scripts, tests, templates, and plugin registration, with careful documentation of hardcoded enumerations that must remain synchronised. Naming conventions, file organisation, and config plumbing are largely consistent with existing codebase standards. However, the proposed ticket template introduces a frontmatter schema and body structure that diverges significantly from the 25 existing tickets without addressing the inconsistency, and several places where implicit conventions could be made explicit are left unflagged.

**Strengths**:
- Companion script naming and directory placement parallel `skills/decisions/scripts/` cleanly
- Plan enumerates every hardcoded location of template/path keys requiring update
- Glob pattern differences ADR vs ticket called out in explicit comparison table
- `ticket-read-field.sh` argument order explicitly justified against Unix convention
- Test-first ordering consistent with TDD pattern
- Plugin registration preserves existing categorical grouping

**Findings**:

**Major — Proposed ticket template schema diverges from all 25 existing tickets** (confidence: high)
Location: Subphase 1C, Section 1: Ticket template (templates/ticket.md)

Frontmatter and body structure incompatible with existing tickets. `What We're NOT Doing` declines migration but creates a permanent two-schema corpus.

**Minor — New script uses different frontmatter-parsing style than ADR sibling** (confidence: high)
Location: Subphase 1B, Section 3: ticket-read-field.sh

Plan justifies divergence in prose but rendered script header doesn't reference the convention source. Add brief comment.

**Minor — PATH_KEYS/PATH_DEFAULTS convention requires tmp to remain last; plan inserts review_tickets mid-array** (confidence: high)
Location: Subphase 1C, Section 6b (config-dump.sh updates)

Existing array also omits `tmp`; ordering convention is ambiguous (grouped vs declaration-order). Document the convention and decide on `tmp`.

**Minor — Step 1 directory count update misses review_tickets arithmetic** (confidence: medium)
Location: Subphase 1C, Section 4: Init skill update

Plan covers Step 4 report insertion but not Path Resolution section insertion. Make both explicit.

**Suggestion — Available template keys list is duplicated across three places without a single source of truth** (confidence: medium)
Location: Subphase 1C, Sections 5c and 6c

Add a "What We're NOT Doing" note about future consolidation onto `config_enumerate_templates`.

### Usability

**Summary**: The plan is thoughtful about DX parity with the ADR pattern: argument ordering for the generic field reader follows Unix convention, error messages are concrete and actionable, and defaults fall back gracefully when the tickets directory is missing. The main usability risks are around template authoring ergonomics (placeholders that authors must manually substitute), a minor asymmetry with the ADR status reader's error message, and an inconsistency between the new template's frontmatter and the existing 25 tickets that developers will encounter in the same directory.

**Strengths**:
- Usage strings explicit and printed to stderr
- Argument ordering follows Unix convention with explicit justification
- Sensible default when tickets directory missing (0001 with stderr warning)
- `--count` value guard closes a real DX gap
- `ticket-read-status.sh` is a discoverable convenience wrapper
- Subphase 1C updates all enumerated surfaces consistently

**Findings**:

**Major — Template placeholders require manual substitution with no tooling guidance** (confidence: high)
Location: Subphase 1C, Section 1: Ticket template (templates/ticket.md)

`NNNN`, `YYYY-MM-DDTHH:MM:SS+00:00`, `Author Name`, `# NNNN: Title...` must all be hand-filled. No Phase 1 guidance for what helper commands to run before Phase 2 lands.

**Major — New template frontmatter diverges from existing 25 tickets in the same directory** (confidence: high)
Location: Subphase 1C, Section 1: Ticket template (frontmatter schema)

Two incompatible shapes coexist; consumers of `ticket-read-field.sh` must handle "field missing" routinely.

**Minor — Missing-field error loses the helpful 'expected values' hint that adr-read-status.sh provides** (confidence: high)
Location: Subphase 1B, Section 3: ticket-read-field.sh error message

Either re-emit hint or accept regression.

**Minor — Array values returned raw will surprise callers expecting structured output** (confidence: high)
Location: Subphase 1B, Tests for ticket-read-field.sh (Test 7)

Document raw-YAML behaviour in script header.

**Minor — Silently ignoring 5-digit-prefixed files is confusing when running manually** (confidence: high)
Location: Subphase 1A, Implementation: ticket-next-number.sh (Test 11)

Optionally emit stderr warning when looser-pattern files exist but are skipped.

**Suggestion — Warning when directory missing could suggest the init command** (confidence: medium)
Location: Subphase 1A, Implementation: ticket-next-number.sh (warning message)

Extend warning to mention `/accelerator:init`.

**Suggestion — Registering an empty skills directory may surface as a discoverability gap** (confidence: medium)
Location: Subphase 1C, Section 2: Plugin registration

Defer registration to Phase 2 or add a stub README explaining the timing.

## Re-Review (Pass 2) — 2026-04-18

**Verdict:** COMMENT

The plan has substantially improved — six of seven prior major findings are
resolved, and the remaining one (enumeration coordination) is now explicitly
acknowledged as deferred debt. However, one edit introduced a new major
concern (the authoring-note HTML comment will leak into every template
preview and consumer copy because `config-read-template.sh` emits the entire
file verbatim), and several minor concerns remain or are newly surfaced.
Verdict is COMMENT rather than REVISE because only one major finding is
present (below the 3-major threshold) and no criticals — the plan is
acceptable but should address the template-preview leak before shipping.

### Previously Identified Issues

**Prior Major Findings:**

- ✅ **Architecture, Standards, Usability** (major): Schema divergence from
  25 existing tickets — **Resolved**. Documented contract added to "What
  We're NOT Doing" guaranteeing `type` and `status` on all tickets;
  template is user-overridable. Verified that all 25 legacy tickets carry
  both fields.

- ✅ **Correctness, Code Quality, Usability** (major): 10000 overflow —
  **Resolved**. `ticket-next-number.sh` now clamps at 9999 with an
  explicit error; Tests 10, 14, and 15 pin the new behaviour. See note
  below about the missing-directory branch asymmetry.

- ✅ **Code Quality** (major): Frontmatter parser reimplemented —
  **Resolved**. `ticket-read-field.sh` now sources `config-common.sh` and
  delegates to `config_extract_frontmatter`. Empirically verified that
  the helper exits 1 for both no-frontmatter and unclosed cases, so the
  `||` trap in the plan fires correctly. (The code-quality reviewer's
  pass-2 claim that the helper exits 0 for no-frontmatter files was
  based on a misreading of awk `exit` semantics; it runs END which
  triggers `exit 1`.)

- ✅ **Test Coverage** (major): Missing `paths.review_tickets` test —
  **Resolved**. Section 7h specifies the test block.

- ✅ **Test Coverage** (major): No `config-dump.sh` row tests —
  **Resolved**. Section 7i specifies both tests.

- 🟡 **Architecture** (major): Hardcoded enumeration coordination tax —
  **Resolved**. Explicitly accepted as debt in "What We're NOT Doing"
  with rationale and future-consolidation reference.

- ✅ **Usability** (major): Template placeholder ergonomics —
  **Resolved** for hand-copiers (authoring note added), **but
  introduces new major** (see below).

**Prior Minor Findings:**

- ✅ **Architecture, Code Quality, Test Coverage, Correctness** (minor):
  Duplicate-key behaviour — **Resolved**. Implementation now `break`s on
  first match; Test 14 pins first-match-wins; script header documents
  the contract.

- ⚪ **Architecture, Code Quality, Usability** (minor): Empty skills
  directory registration — **Still present**. Not addressed by edits;
  claim of "harmless" remains unverified.

- ⚪ **Correctness, Usability** (minor): Status reader loses
  allowed-values hint — **Still present**. Acknowledged as unaddressed
  in the review conversation.

- ✅ **Code Quality** (minor): Triple-piped sed — **Partially resolved**.
  Collapsed to a single `sed -e ... -e ...` invocation, but the middle
  `-e` still contains two commands joined by `;`. See minor finding
  below.

- ✅ **Code Quality** (minor): Hardcoded `0026` in automated check —
  **Partially resolved**. Automated Verification softened to regex match;
  Manual Verification block at plan line 966 still reads "should output
  `0026`" (see new minor below).

- ⚪ **Code Quality** (minor): Stricter ticket glob undocumented — **Still
  present**. A 9999-boundary row was added to the comparison table, but
  no explanatory comment was added to the implementation snippet. Low
  priority given Test 15 now covers the no-dash case.

- ✅ **Architecture** (minor): Concurrency non-safety — **Resolved**
  (acknowledged in "What We're NOT Doing" with rationale).

- ✅ **Test Coverage** (minor): Stale 5-template assertions — **Resolved**.
  Section 7g addresses lines ~3107 and ~2269.

- 🟡 **Test Coverage** (minor): No prefix-match defence tests — **Partially
  resolved**. Tests 15 and 16 added; Test 16 remains under-specified (see
  new minor below).

- 🟡 **Test Coverage** (minor): No init directory-count invariant check —
  **Partially resolved**. Success criterion adds a literal `grep -q '12
  directories'` check, but the count-vs-list invariant is described only
  in prose, not as a concrete test block (see new minor below).

- ✅ **Standards** (minor): Frontmatter-parsing style divergence from ADR —
  **Resolved structurally**. The ticket reader now uses the shared
  helper; ADR non-migration is explicit in "What We're NOT Doing".

- ⚪ **Standards** (minor): `paths.tmp` omission in `config-dump.sh`
  arrays — **Still present**. The plan adds `paths.review_tickets` but
  not `paths.tmp`.

- ⚪ **Standards** (minor): `PATH_KEYS` ordering convention unstated —
  **Still present**. The plan inserts consistently with the implied
  convention but doesn't document it.

- ✅ **Standards** (minor): Init SKILL.md update misses Path Resolution —
  **Resolved**. Section 4 restructured as 4a/4b/4c with all three
  insertion points explicit.

- ⚪ **Correctness** (minor): Quote stripper tolerates mismatched pairs —
  **Still present**. Accepted as unaddressed.

- ✅ **Usability** (minor): Array values returned raw surprise callers —
  **Resolved**. Script header now documents the behaviour and references
  `config_parse_array`.

- ⚪ **Usability** (minor): Silent 5-digit-prefix ignore — **Still
  present** but partially mitigated by the 9999 clamp making the
  scenario unreachable on the happy path.

**Prior Suggestions:** mostly still present; partially addressed via the
restructuring of Section 4 and the rewording of Test 11, but line-number
anchors remain throughout Sections 5-7.

### New Issues Introduced

- 🟡 **Usability** (major, high confidence): Authoring-note HTML comment
  leaks into every template render
  **Location**: Subphase 1C, Section 1 (templates/ticket.md)
  `scripts/config-read-template.sh` emits the entire template verbatim
  inside markdown fences — it does not strip HTML comments. The new
  `<!-- Authoring notes ... -->` block therefore appears in every
  template preview and will be copied into any Phase 2+ create-ticket
  skill output unless each consumer explicitly strips it. No other
  shipping template (`adr.md`, `plan.md`, `research.md`) uses this
  pattern; `adr.md` places hints inline on frontmatter lines using YAML
  `# ...` comments.
  **Suggestion**: Move the authoring notes inline as YAML frontmatter
  comments (e.g., `type: story  # story | epic | task | bug | spike`)
  following the adr.md convention. Guidance that genuinely needs a
  block format can live in a sibling `templates/ticket.README.md` or in
  `skills/config/configure/SKILL.md`.

- 🔵 **Correctness** (minor, high confidence): 9999 clamp is not applied
  to the missing-directory branch
  **Location**: Subphase 1A, ticket-next-number.sh
  The `if [ ! -d "$TICKETS_DIR" ]` branch runs a raw `for` loop with no
  upper bound, so `--count 10000` against a fresh repo emits
  `0001..9999` followed by `10000` and exits 0 — the exact invariant
  violation the clamp was added to prevent. Apply the clamp before the
  directory check, or duplicate the clamp inside the missing-directory
  branch.

- 🔵 **Correctness** (minor, high confidence): Quote stripper leaves an
  orphan quote when a closing quote is followed by trailing whitespace
  **Location**: Subphase 1B, ticket-read-field.sh sed pipeline
  The sed order is: strip-leading-whitespace → strip-leading-quote →
  strip-trailing-quote → strip-trailing-whitespace. For input
  `"draft"  ` (trailing whitespace after closing quote), step 3 sees
  whitespace at end and doesn't strip; step 4 strips whitespace, leaving
  `draft"`. Reorder so trailing whitespace is stripped before trailing
  quote.

- 🔵 **Standards** (minor, medium confidence): Ticket/ADR field readers
  diverge on duplicate-key semantics
  **Location**: Subphase 1B, Section 3
  The plan documents first-match-wins as "consistent with
  `config-read-value.sh`", but `adr-read-status.sh` actually uses
  last-match-wins (its while-loop has no `break`). Two parallel
  interfaces now encode opposite contracts. Acknowledge the divergence
  explicitly, and/or note that future convergence will adopt the
  first-match-wins contract.

- 🔵 **Test Coverage** (minor, high confidence): Init directory-count
  invariant is informally described, not specified
  **Location**: Subphase 1C, Section 4 (prose) + success criterion
  The success criterion is a literal `grep -q '12 directories'` which
  only catches the current change, not future drift. Section 4
  describes a count-vs-list assertion in prose but gives no bash
  snippet. Promote to a concrete test block in Section 7: compute
  `EXPECTED=$(grep -cE '^\*\*[A-Z][^*]* directory\*\*:' …)` and assert
  the count in prose matches `$EXPECTED`.

- 🔵 **Test Coverage** (minor, medium confidence): Test 16 for
  regex-metacharacter field names is under-specified
  **Location**: Subphase 1B, Section 1 — Test 16
  Test 16 says "matches literally (bash quoted-prefix is not a regex)"
  without specifying fixture content or expected behaviour. A
  conscientious implementer could satisfy this with a positive-match
  fixture (proves literal matching works) while missing the
  negative-match case (proves `.` is not a regex wildcard). Tighten to
  specify both: (a) fixture has `sub.type: foo`, query `sub.type` →
  outputs `foo`; (b) fixture has `subXtype: foo` with no `sub.type:`
  key, query `sub.type` → exits 1.

- 🔵 **Code Quality** (minor, high confidence): Manual Verification still
  hardcodes `0026`
  **Location**: Testing Strategy → Manual Verification (plan line 966)
  The automated block was softened to a regex match but the manual
  block still reads "should output `0026` (after the 25 existing
  tickets)". Apply the same softening.

- 🔵 **Code Quality** (minor, high confidence): Collapsed sed still hides
  two commands in one `-e`
  **Location**: Subphase 1B, Section 3
  The middle `-e 's/^["...]//; s/["...]$//'` joins two commands with
  `;`. Splitting into four `-e` flags (or extracting a shared
  `config_strip_value` helper into `config-common.sh`) would make the
  readability goal of the prior edit land fully.

- 🔵 **Usability** (minor, medium confidence): 9999-exhaustion error
  suggests "widening the pattern" but no such path exists in Phase 1
  **Location**: Subphase 1A, ticket-next-number.sh
  Test 10 pins the error message across future phases. Phase 1
  explicitly declines to support configurable filename patterns, so the
  suggested remediation points at capability that does not yet exist.
  Soften to a pragmatic alternative (e.g., "archive completed tickets
  or file an enhancement for a 5-digit pattern") or reference a future
  enhancement ticket by ID.

- 🔵 **Usability** (minor, medium confidence): Delegated frontmatter
  error collapses no-frontmatter and unclosed-frontmatter into one
  message
  **Location**: Subphase 1B, Section 3
  Both failure modes produce `Error: No valid frontmatter in <file>.`
  making the common unclosed-`---` typo less self-diagnosing. Either
  pre-check for a leading `---` so the two cases can be messaged
  separately, or evolve `config_extract_frontmatter` to emit the
  specific diagnostic itself.

- 🔵 **Code Quality** (suggestion, medium confidence): Sourcing
  `config-common.sh` silently rebinds `SCRIPT_DIR` in the caller
  **Location**: Subphase 1B, Section 3
  `config-common.sh` line 7 resets `SCRIPT_DIR` when sourced. The
  plan's current code is safe only because `SCRIPT_DIR` is read once
  before the `source`. A future edit that uses `SCRIPT_DIR` after the
  source would get the wrong path. Rename the local to
  `TICKET_SCRIPT_DIR` or save/restore around the source.

- 🔵 **Architecture** (suggestion, high confidence): Line-number
  anchors remain throughout Sections 5-7
  **Location**: Subphase 1C, Sections 5, 6, 7
  Section 4 was restructured with semantic anchors, but Sections 5 and
  6 still reference "around line 337", "line 7", "around line 209",
  etc., and Section 7 threads ten or so line-number references through
  `test-config.sh`. These will drift by the time the plan is
  implemented.

### Assessment

The plan is now in substantially better shape than it was at the start of
this review. Six of seven major findings from pass 1 are resolved, the
remaining major is explicitly accepted as debt, and the critical test-
coverage gaps for the new path/template keys are closed. The one
consequential regression — the authoring-note HTML comment leaking into
every template preview — is a direct artefact of one of the edits made
to address a prior major finding, and has a clean fix (move the hints
inline as YAML frontmatter comments, matching `templates/adr.md`). The
remaining minor concerns are mostly editorial or tangential, and several
prior minors (allowed-values hint, quote stripper, empty directory
registration) were consciously deferred.

Per the configured thresholds (3 majors or any critical → REVISE), the
verdict is COMMENT rather than REVISE — but the authoring-note leak is
worth fixing before implementation starts, because once Phase 2
create-ticket skills land they will all need to independently strip the
HTML comment.

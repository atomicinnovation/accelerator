---
date: "2026-04-21T16:40:54Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-21-list-and-update-tickets.md"
review_number: 2
verdict: COMMENT
lenses: [ architecture, code-quality, test-coverage, correctness, standards, usability, compatibility, safety ]
review_pass: 2
status: complete
---

## Plan Review: Ticket Listing and Updating Skills (Phase 3)

**Verdict:** REVISE

The plan has matured substantially since review-1: all 13 recommended changes
were addressed, the specification is significantly tighter, and the two new
bash helper scripts (P.4, P.5) give deterministic ownership to the riskiest
operations (tag mutation and template hint extraction). However, a fresh review
through the same 8 lenses surfaces four new major findings — three centred on
the `ticket-update-tags.sh` contract with `ticket-read-field.sh`, and one on
a missing eval for a materially different edit operation (field insertion vs.
replacement). The remaining findings are refinements rather than structural
concerns, and most are addressable with small targeted edits.

### Cross-Cutting Themes

- **`ticket-update-tags.sh` relies on `ticket-read-field.sh` in ways the
  existing script cannot fully support** (flagged by: correctness,
  code-quality, compatibility) — Block-style YAML detection and absent-field
  vs. file-error disambiguation both require the tag script to read the raw
  file rather than relying solely on `ticket-read-field.sh` output. Two
  distinct correctness findings share this root cause.
- **Display text conversion for body label sync is non-deterministic**
  (flagged by: code-quality, usability, correctness, compatibility) — The
  "English language judgement" rule for converting hyphenated frontmatter
  values to display text (`waiting-on-legal` vs `in-flight`) will produce
  inconsistent results across invocations. Four lenses flag this as a
  predictability concern.
- **Non-atomic multi-edit write risk is acknowledged but under-remediated**
  (flagged by: safety, architecture, code-quality) — The sequential
  frontmatter-then-body Edit calls are an accepted tradeoff, but the warning
  message could include a concrete remediation command (`jj restore`).
- **New frontmatter field insertion (absent field → add) is a distinct
  operation not covered by existing evals** (flagged by: test-coverage,
  correctness, compatibility, safety) — Adding `tags` or `priority` to a
  legacy ticket that has never had those fields requires line insertion, not
  replacement. The Edit tool behaviour for this case differs from all other
  update scenarios.
- **Frontmatter parsing duplication remains a known debt** (flagged by:
  architecture, code-quality) — Both new skills re-implement multi-field
  parsing in their prompts. This was flagged in review-1 and explicitly
  deferred; it recurs here as acknowledged technical debt rather than a new
  finding.

### Tradeoff Analysis

- **Deterministic display text vs. LLM linguistic judgement**: Code-quality
  and correctness lenses want a deterministic rule (always replace hyphens
  with spaces and title-case). Usability lens notes that compound adjectives
  like `in-flight` look wrong as `In Flight`. Recommend: define a simple
  deterministic default rule and accept minor cosmetic imperfection, since
  predictability matters more than linguistic perfection for machine-edited
  text.
- **Raw file reading in tag script vs. composing on ticket-read-field.sh**:
  Correctness lens argues the tag script must read the raw file for
  block-style detection and error disambiguation. Architecture lens prefers
  composing on existing helpers to avoid a third parser. Recommend: add a
  narrow raw-file pre-check (validate file exists, frontmatter is closed,
  check next line after `tags:` for block-style indicators) before delegating
  to `ticket-read-field.sh` for the actual value extraction.

### Findings

#### Major

- 🟡 **Correctness / Code Quality**: Block-style YAML detection in
  `ticket-update-tags.sh` cannot rely solely on `ticket-read-field.sh`
  **Location**: Prerequisites P.4
  For block-style YAML (`tags:\n  - api\n  - search`), `ticket-read-field.sh`
  returns an empty string on the `tags:` line — identical to `tags:` with no
  value. The tag script cannot distinguish these cases and would silently
  treat block-style tags as empty, corrupting the file on the next `add`
  operation. The script must read the raw file to detect block-style format.

- 🟡 **Correctness**: `ticket-update-tags.sh` cannot distinguish absent tags
  field from file/frontmatter errors
  **Location**: Prerequisites P.4
  When `ticket-read-field.sh` exits with code 1, this could mean the field
  is absent (legitimate for `add` to create `[<tag>]`), the file does not
  exist, or frontmatter is malformed. The plan's P.4 spec does not specify
  how the tag script disambiguates these cases. Without validation, calling
  the script with a non-existent path would silently produce `[<tag>]`
  instead of an error.

- 🟡 **Compatibility**: Phase 2 quality-guidelines field enumeration needs
  `title:` added in P.2/P.3
  **Location**: Prerequisites P.2, P.3
  Both `create-ticket/SKILL.md` (line 282) and `extract-tickets/SKILL.md`
  (line 421) enumerate the required frontmatter fields explicitly. The plan
  says P.2/P.3 are "minimal touchup" but does not include updating these
  field lists. An LLM following the quality-guidelines enumeration strictly
  could omit `title:` even though the template includes it.

- 🟡 **Test Coverage / Correctness**: No eval scenario for adding a field
  that does not exist in frontmatter
  **Location**: Subphase 3.2 Approach Evals
  Scenario 28 covers adding a tag to an absent `tags` field, but no
  equivalent scenario covers setting a scalar field absent from frontmatter
  (e.g., `/update-ticket 0011 priority high` on a legacy ticket with no
  `priority:` line). Field insertion is a materially different Edit operation
  from field replacement, and the LLM's behaviour for it is untested.

#### Minor

- 🔵 **Safety / Architecture / Code Quality**: Non-atomic write with no
  remediation command in warning
  **Location**: Subphase 3.2, Step 5
  The sequential frontmatter-then-body Edit calls are an acknowledged
  tradeoff. The warning message on body sync failure should include a
  concrete remediation command (e.g., `jj restore <file>`) so the user knows
  how to recover.

- 🔵 **Code Quality / Usability / Correctness / Compatibility**: Display
  text conversion for hyphenated values is non-deterministic
  **Location**: Subphase 3.2, Scenario 27 + Step 4
  The "English language judgement" rule distinguishing multi-word phrases from
  compound adjectives will produce inconsistent body labels across
  invocations. The same frontmatter value could render differently each time.

- 🔵 **Correctness / Compatibility / Safety**: New field insertion position
  unspecified
  **Location**: Subphase 3.2, Scenarios 28, Step 5
  Adding `tags:` or `priority:` to a legacy ticket with no such field
  requires inserting a new frontmatter line. The plan does not specify where
  the new field should be placed. If inserted outside the `---` delimiters,
  the frontmatter structure is corrupted.

- 🔵 **Code Quality / Compatibility**: Hardcoded P.5 fallback values may
  drift from template
  **Location**: Prerequisites P.5
  The hardcoded fallback for `type`, `status`, and `priority` in
  `ticket-template-field-hints.sh` must stay in sync with the template's
  trailing comments. No test verifies this invariant.

- 🔵 **Correctness / Test Coverage**: Multi-op no-op detection only addresses
  single-field case
  **Location**: Subphase 3.2, Step 3
  The plan says "print 'No change needed' and exit" when the new value
  equals the current value. For multi-op commands where one field is already
  at the target value and another is not, the expected behaviour is
  unspecified. An implementer could exit on the first no-op, discarding the
  second valid operation.

- 🔵 **Standards**: Scenario numbering inconsistency (2a/2b vs flat integers)
  **Location**: Subphase 3.1 Approach Evals
  List-tickets uses letter-suffixed sub-scenarios (2a, 2b) while
  update-ticket uses flat integers (1-34). The eval count "19 scenarios" is
  correct when counting 2a and 2b separately, but the inconsistent scheme
  could confuse the skill-creator eval runner.

- 🔵 **Test Coverage**: Missing tests for per-character quoting triggers in
  tags
  **Location**: Prerequisites P.4 Tests
  The "special characters" test is a single case. The three distinct quoting
  triggers (comma, colon, hash) should each have a dedicated test, since a
  bug in one delimiter's handling would not be caught by a single generic
  test.

- 🔵 **Test Coverage**: No test for `config-read-template.sh` failure in P.5
  **Location**: Prerequisites P.5 Tests
  If `config-read-template.sh` fails (template missing, no fallback
  available), `ticket-template-field-hints.sh` should still exit 0 and fall
  back to hardcoded defaults. No test verifies this.

- 🔵 **Compatibility**: Custom template users won't receive `title:`
  automatically
  **Location**: Prerequisites P.1
  Users who have overridden the template via `meta/templates/ticket.md` will
  not get the new `title:` field. Tickets created from their custom template
  may lack `title:`, breaking the plan's assumption that "both schemas
  uniformly carry a `title:` frontmatter field." The changelog should note
  this schema evolution.

- 🔵 **Usability**: Block-style tag rejection message lacks conversion
  example
  **Location**: Prerequisites P.4
  The error "convert to `tags: [...]` first" tells users what to do but not
  how. A concrete example in the message (e.g., `tags: [api, search]`) would
  reduce friction.

- 🔵 **Test Coverage**: No eval for a ticket file that does not match the
  `NNNN-*.md` glob
  **Location**: Subphase 3.1 Approach Evals
  No scenario verifies that non-ticket files (e.g., `README.md`) in the
  tickets directory are silently excluded from the listing.

- 🔵 **Usability**: Legacy status values require non-obvious explicit-form
  syntax
  **Location**: Subphase 3.1, Scenario 16 + filter rules 4-5
  The current 29 tickets use `status: todo` or `status: done`, but these
  values are not in the template's hint output. `/list-tickets todo` falls
  to rule 5 (title search) rather than status filtering. Users must know to
  write `/list-tickets status todo`. When title search returns zero matches,
  a fallback hint would help discoverability.

#### Suggestions

- 🔵 **Architecture**: Cycle detection in hierarchy mode should include an
  algorithmic hint (e.g., "track visited IDs during tree construction")
- 🔵 **Architecture**: P.1-P.3 could be committed independently as a
  standalone preparatory change
- 🔵 **Architecture**: NL interpretation in update-ticket lacks a
  correction loop after echoing the interpretation
- 🔵 **Test Coverage**: SKILL.md structural invariants (frontmatter fields,
  allowed-tools format) could be verified by grep-based smoke tests in CI
- 🔵 **Test Coverage**: Parent cycle test only covers two-node cycles; a
  self-referential cycle (A → A) is a distinct edge case
- 🔵 **Correctness**: Glob pattern `NNNN-*.md` should use explicit shell
  glob `[0-9][0-9][0-9][0-9]-*.md` for consistency with
  `ticket-next-number.sh`
- 🔵 **Correctness**: `config-read-template.sh` output is code-fenced;
  P.5 spec should note this and specify fence stripping
- 🔵 **Safety**: Two-strike confirmation exit message should explicitly say
  "Treating as decline"
- 🔵 **Standards**: Body label display text conversion is a new convention
  not present in create-ticket/extract-tickets — worth documenting
- 🔵 **Usability**: When a free-text title search returns zero matches and
  the term appears as a tag value, surface a hint: "Did you mean:
  `tagged <term>`?"

### Strengths

- ✅ All 13 recommended changes from review-1 were addressed, demonstrating
  disciplined iteration and willingness to tighten specifications.
- ✅ The two new bash helper scripts (`ticket-update-tags.sh`,
  `ticket-template-field-hints.sh`) give deterministic ownership to the
  riskiest operations, moving tag mutation and hint extraction out of
  LLM-interpreted prompts.
- ✅ The 5-rule filter precedence cascade in list-tickets is explicit,
  numbered, and covers the full input domain — a significant improvement
  over review-1's "clearly indicates a field" phrasing.
- ✅ Body label sync now covers all four template labels (`**Status**:`,
  `**Type**:`, `**Priority**:`, `**Author**:`) with code-fence awareness
  and first-non-fenced-occurrence matching.
- ✅ Parent normalisation is consistently specified in both skills: zero-padded
  4-digit quoted strings on write, normalised comparison on read.
- ✅ `ticket_id` is hard-blocked with a concrete pointer to the `jj mv`
  workflow, eliminating the filename/frontmatter divergence risk.
- ✅ Confirmation flow specifies exact acceptance tokens (`y`/`yes`),
  re-prompt behaviour, and fail-safe decline on unrecognised input.
- ✅ Legacy-schema coexistence is thoroughly addressed: both schemas carry
  `title:` uniformly, missing fields render as `—`, all-empty columns are
  suppressed, and legacy values are reachable via explicit structured filters.
- ✅ "What We're NOT Doing" expanded to 10 concrete exclusions including the
  two new deferred items (consolidated parser, template-only changes).
- ✅ TDD framing remains explicit and consistently applied across all
  subphases with Phase 1 regression suite as a gate.
- ✅ Pattern fidelity with `review-adr` maintained for the frontmatter-edit
  flow while extending it appropriately for the open-vocabulary ticket
  context.
- ✅ Diff preview + explicit confirmation always gates writes; decline is
  byte-for-byte verifiable (Scenario 14).

### Recommended Changes

Ordered by impact. Addressing #1 through #4 would move the plan to APPROVE.

1. **Fix `ticket-update-tags.sh` block-style detection and error
   disambiguation** (addresses: block-style YAML detection, absent-field
   error disambiguation)
   Specify that `ticket-update-tags.sh` performs three pre-checks before
   calling `ticket-read-field.sh`: (a) validate the file exists (exit 1 with
   file-not-found error if not), (b) validate frontmatter is present and
   closed (exit 1 with appropriate error if not), (c) read the raw `tags:`
   line and check if the next line matches `^  - ` to detect block-style
   (exit 1 with block-style error if so). Only after these checks pass,
   delegate to `ticket-read-field.sh` for the actual value extraction. Update
   the P.4 tests to cover: block-style detected via raw file check,
   non-existent file produces a clear error, malformed frontmatter produces
   a clear error.

2. **Update Phase 2 quality-guidelines field enumeration** (addresses:
   compatibility finding on P.2/P.3 field lists)
   Expand P.2 and P.3 scope to include updating the quality-guidelines
   field enumeration in `create-ticket/SKILL.md` and
   `extract-tickets/SKILL.md` to add `title:` alongside the other listed
   fields. This is a one-line addition per skill.

3. **Add an eval for absent-field insertion** (addresses: test-coverage
   finding on missing insertion eval)
   Add a scenario (e.g., Scenario 35) for
   `/update-ticket 0011 priority high` on a legacy ticket with no
   `priority:` line, verifying that the diff preview shows a pure addition
   (`+priority: high`) and the field is inserted before the closing `---`.

4. **Specify new field insertion position** (addresses: field insertion
   position finding)
   Add a note to Step 5 or Quality Guidelines stating that when a field does
   not exist in frontmatter, it should be inserted as the last line before
   the closing `---` delimiter.

5. **Clarify multi-op no-op behaviour** (addresses: multi-op no-op finding)
   Add a sentence to Step 3: "In multi-op mode, no-op detection is
   per-field: fields already at the target value are excluded from the diff
   with an informational note. The 'no change needed' exit only triggers
   when ALL requested operations are no-ops."

6. **Add concrete remediation to partial-write warning** (addresses:
   non-atomic write finding)
   Append to the warning message: "To revert, run: `jj restore <filename>`".

7. **Tighten display text conversion** (addresses: non-determinism finding)
   Replace "English language judgement" with a deterministic default: "Single
   words are capitalised. Hyphenated values: replace hyphens with spaces and
   apply title case (keeping small words lowercase unless they open the
   phrase)." Accept that compound adjectives like `in-flight` will become
   `In Flight` — predictability over linguistic perfection.

8. **Minor test and spec polish** (addresses: remaining minor findings)
   - Expand P.4 "special characters" test into three cases (comma, colon,
     hash) and add a test for `config-read-template.sh` failure in P.5
   - Renumber list-tickets scenarios to flat integers (1-19) for consistency
   - Add a concrete example to the block-style rejection error message
   - Add a test verifying P.5 hardcoded fallback values match the shipping
     template's comments
   - Note in changelog that `title:` is a template schema evolution for
     custom template users

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan demonstrates excellent architectural consistency with
established Phase 1-2 patterns, clean module boundaries between scripts and
SKILL.md prompt files, and a well-considered approach to backward
compatibility with legacy tickets. The two new helper scripts (P.4, P.5) are
appropriately scoped as narrow, composable utilities. The primary architectural
concern is the duplicated frontmatter parsing logic across SKILL.md files
without a shared executable substrate, which the plan explicitly acknowledges
and defers. A secondary concern is the coupling between the list-tickets
filter precedence rules and the template hint infrastructure, which could
create brittleness as the template evolves.

**Strengths**:

- ✅ Strong adherence to the established architectural pattern: SKILL.md files as
  LLM prompts, bash scripts as deterministic helpers, Edit tool for writes
- ✅ The decision to keep status transitions unenforced is architecturally sound
  for this phase, maintaining the open-closed principle
- ✅ The ticket-update-tags.sh script owns the complete tag mutation round-trip,
  keeping the canonical format in one place
- ✅ Legacy ticket coexistence is well-designed: both schemas supported without
  migration, field absence renders as dashes rather than errors
- ✅ Plugin registration architecture leveraged correctly: directory-scan pattern
  means zero registration changes needed

**Findings**:

- 🟡 major/medium: Duplicated frontmatter parsing in SKILL.md prompts risks drift
  **Location**: What We're NOT Doing: 'No consolidated frontmatter parser'
- 🟡 major/medium: Filter resolution couples tightly to template comment syntax
  **Location**: Subphase 3.1, Step 1 - Resolve filter, rules 3-4
- 🔵 minor/high: Non-atomic multi-edit write with no rollback creates partial
  update risk
  **Location**: Subphase 3.2, Step 5 - Write
- 🔵 minor/high: Cycle detection strategy is underspecified
  **Location**: Subphase 3.1, Scenario 18
- 🔵 minor/medium: Tag script output coupling requires caller to perform file write
  **Location**: Prerequisites P.4
- 🔵 suggestion/medium: Natural language interpretation lacks feedback loop for
  misinterpretation recovery
  **Location**: Subphase 3.2, Step 3, rule 4
- 🔵 suggestion/low: Template and Phase 2 skill modifications could be isolated
  as a separate atomic change
  **Location**: Implementation Approach, Prerequisites P.1-P.3

### Code Quality

**Summary**: The plan is thorough and well-structured, demonstrating strong
adherence to existing codebase conventions and thoughtful separation of
concerns. The two new bash helper scripts (P.4, P.5) are narrowly scoped and
testable, and the SKILL.md specifications follow established patterns from
Phase 2. The main code quality concerns are around duplicated frontmatter
parsing logic across SKILL.md prompts (acknowledged but deferred), the
complexity of the natural language filter parser in list-tickets, and a subtle
data integrity issue in the tag update script's interaction with
ticket-read-field.sh's quote-stripping behaviour.

**Strengths**:

- ✅ Excellent consistency with existing codebase conventions: structural patterns,
  frontmatter layout, allowed-tools declarations, agent fallback blocks
- ✅ The 'What We're NOT Doing' section is exemplary for maintainability
- ✅ Helper scripts are narrowly scoped with single responsibilities, matching
  existing patterns
- ✅ The plan handles dual-schema coexistence gracefully throughout
- ✅ The TDD approach is a pragmatic adaptation for prompt-as-code artifacts

**Findings**:

- 🟡 major/medium: Quote-stripping in ticket-read-field.sh may corrupt
  bracket-containing tag values
  **Location**: Prerequisites P.4: ticket-update-tags.sh
- 🟡 major/medium: Five-rule filter precedence cascade has high cognitive
  complexity
  **Location**: Subphase 3.1: list-tickets, Step 1
- 🔵 minor/high: Hyphen-to-display-text conversion relies on LLM language
  judgement with no deterministic fallback
  **Location**: Subphase 3.2, Scenario 27 + Step 4
- 🔵 minor/high: Hardcoded fallback values duplicate the template and may drift
  **Location**: Prerequisites P.5
- 🔵 minor/medium: Sequential Edit calls without transactional guarantee create
  partial-update risk
  **Location**: Subphase 3.2, Step 5
- 🔵 suggestion/medium: Deferred frontmatter parsing consolidation should have
  a tracking mechanism
  **Location**: Plan-wide
- 🔵 suggestion/low: Block-style YAML detection rule may be fragile for edge
  cases
  **Location**: Prerequisites P.4

### Test Coverage

**Summary**: The plan adopts a disciplined TDD approach where approach evals
serve as the specification for SKILL.md files, and bash helper scripts (P.4
and P.5) receive concrete unit tests added to the existing regression suite.
The test strategy is well-matched to the project's architecture. However,
there are gaps in edge case coverage for the new bash scripts, the SKILL.md
eval scenarios lean heavily on happy-path and functional paths with limited
adversarial/mutation-resistant coverage, and the integration verification
phase relies entirely on manual checks with no automation harness.

**Strengths**:

- ✅ TDD order is explicit and consistently applied
- ✅ 9 concrete test cases for ticket-update-tags.sh and 5 for
  ticket-template-field-hints.sh, covering key edge cases
- ✅ 19 + 34 approach evals are thorough and scenario-driven
- ✅ Plan explicitly acknowledges what cannot be automated
- ✅ Regression protection built into the workflow at every subphase boundary

**Findings**:

- 🟡 major/high: Missing test for concurrent/multi-word tags and
  comma-containing values
  **Location**: Prerequisites P.4 Tests
- 🟡 major/high: No test for template file not found or
  config-read-template.sh failure
  **Location**: Prerequisites P.5 Tests
- 🟡 major/medium: No eval scenario for adding a field that does not yet
  exist in frontmatter
  **Location**: Subphase 3.2 Approach Evals
- 🟡 major/medium: No eval for a ticket file that does not match the
  NNNN-*.md glob
  **Location**: Subphase 3.1 Approach Evals
- 🔵 minor/high: No test for idempotent remove on empty array
  **Location**: Prerequisites P.4 Tests
- 🔵 minor/high: No automated test for title field population in updated
  Phase 2 skills
  **Location**: Prerequisites P.2 and P.3
- 🔵 minor/medium: No eval for multi-op where one op is a no-op and another
  is a real change
  **Location**: Subphase 3.2, Scenario 33
- 🔵 suggestion/medium: No smoke test for SKILL.md structural invariants in CI
  **Location**: Testing Strategy section
- 🔵 suggestion/low: Parent cycle test only covers two-node cycles
  **Location**: Subphase 3.1, Scenario 18

### Correctness

**Summary**: The plan is thorough and well-specified, with detailed eval
scenarios covering most boundary conditions. However, there are a few
correctness concerns: ticket-update-tags.sh cannot reliably distinguish
block-style YAML from empty/absent tags using ticket-read-field.sh alone,
there is a missing edge case around multi-op no-op detection, and the
absent-field insertion path for tags relies on implicit LLM reasoning not
spelled out in the skill flow.

**Strengths**:

- ✅ Filter precedence rules are well-ordered with explicit fallthrough semantics
- ✅ Parent normalisation is consistently specified in both skills
- ✅ Cycle detection in hierarchy rendering prevents infinite loops
- ✅ Edge cases for ticket-update-tags.sh are well-covered
- ✅ Plan correctly identifies that ticket-read-field.sh returns raw YAML array
  values verbatim

**Findings**:

- 🟡 major/high: ticket-read-field.sh cannot distinguish block-style tags from
  empty tags
  **Location**: Prerequisites P.4: Block-style detection
- 🟡 major/high: Cannot distinguish absent tags field from file/frontmatter
  errors
  **Location**: Prerequisites P.4: Error path disambiguation
- 🔵 minor/medium: No-op detection for multi-op commands only addresses
  single-field case
  **Location**: Subphase 3.2, Step 3
- 🔵 minor/medium: Skill flow does not explicitly describe tag field insertion
  into frontmatter
  **Location**: Subphase 3.2, Scenario 28
- 🔵 minor/medium: Display text conversion rules are underspecified for edge
  cases
  **Location**: Subphase 3.2, Step 4
- 🔵 suggestion/medium: Glob pattern NNNN-*.md does not enforce exactly 4-digit
  prefix
  **Location**: Subphase 3.1, Step 2
- 🔵 suggestion/low: Template output includes code fences that must be handled
  during parsing
  **Location**: Prerequisites P.5

### Standards

**Summary**: The plan demonstrates strong adherence to established project
conventions across naming, file organisation, frontmatter structure,
allowed-tools patterns, and configuration preamble ordering. It correctly
identifies and follows the patterns set by Phase 2 skills and the analogous
decisions category. Two minor naming observations and one convention gap
around description field format are worth noting.

**Strengths**:

- ✅ Skill directory naming follows established kebab-case convention
- ✅ Frontmatter fields and ordering match Phase 2 exemplars exactly
- ✅ allowed-tools pattern is identical to Phase 2 skills
- ✅ Configuration preamble ordering matches the established convention
- ✅ Agent fallback block uses canonical 'accelerator:' prefix consistently
- ✅ Script naming follows the 'ticket-<verb>-<noun>.sh' pattern
- ✅ H1 headings follow Title Case convention
- ✅ New scripts add tests to existing test-ticket-scripts.sh harness
- ✅ Plugin registration requires no changes

**Findings**:

- 🟡 major/high: Scenario numbering inconsistency (2a/2b vs flat integers)
  **Location**: Subphase 3.1 Approach Evals
- 🔵 minor/high: Description field uses > block scalar notation in spec (with
  clarifying parenthetical)
  **Location**: Subphase 3.1, Changes Required
- 🔵 minor/medium: Script argument order inconsistent with ticket-read-field.sh
  convention
  **Location**: Prerequisites P.4
- 🔵 minor/high: ticket-template-field-hints.sh does not take a ticket-path
  argument unlike other ticket-* scripts
  **Location**: Prerequisites P.5
- 🔵 suggestion/medium: Scenario 18 cycle marker notation not established in
  skill spec
  **Location**: Subphase 3.1, Scenario 18
- 🔵 suggestion/high: Body label display text conversion rules diverge from
  create-ticket conventions
  **Location**: Subphase 3.2, Step 4

### Usability

**Summary**: The plan delivers a thoughtful developer experience with echoed
filter interpretations, graceful degradation for malformed tickets, and
consistent interaction patterns across both skills. The main usability concern
is the 5-rule filter precedence cascade in list-tickets, which creates a
non-obvious gap where common legacy status values like 'todo' require
explicit-form syntax rather than the shorthand that works for
template-defined values.

**Strengths**:

- ✅ Always echoing the interpreted filter before results gives users confidence
  and a correction path
- ✅ Graceful degradation for malformed and legacy tickets
- ✅ Confirmation preview follows the principle of least surprise
- ✅ Bare invocations handled well with progressive disclosure
- ✅ Consistent error messages with actionable remediation steps
- ✅ Parent value normalisation ensures consistent matching

**Findings**:

- 🟡 major/high: Legacy status values require non-obvious explicit-form syntax
  **Location**: Subphase 3.1, Scenario 16 + filter rules 4-5
- 🔵 minor/medium: Non-deterministic display text conversion for hyphenated
  values
  **Location**: Subphase 3.2, Scenario 27
- 🔵 minor/medium: Block-style tag rejection offers no conversion guidance
  **Location**: Prerequisites P.4
- 🔵 suggestion/medium: Single-token filter defaults may surprise users
  expecting tag search
  **Location**: Subphase 3.1, Scenario 15
- 🔵 suggestion/medium: Multi-op parsing requires memorising field-value pair
  syntax
  **Location**: Subphase 3.2, Step 3
- 🔵 suggestion/low: No pagination or count limit for large ticket sets
  **Location**: Subphase 3.1, Step 4

### Compatibility

**Summary**: The plan is well-designed for backward compatibility with the
existing 29 legacy tickets and the Phase 1 script contract. The key
compatibility risk is the template schema change (adding `title:`) and the
prerequisite modifications to Phase 2 skills, which alter the implicit API
contract for tickets created by those skills. The plan explicitly addresses
legacy coexistence, but a few edge cases in the Phase 2 quality-guidelines
enumeration and custom template handling deserve attention.

**Strengths**:

- ✅ Legacy ticket coexistence thoroughly considered
- ✅ Avoids breaking the existing ticket-read-field.sh contract
- ✅ Filter precedence rules layered so new shorthand values cannot shadow
  legacy values
- ✅ ticket-update-tags.sh interface uses idempotent no-change signal
- ✅ Plugin registration covers new subdirectories automatically

**Findings**:

- 🟡 major/high: Template schema change alters the contract for downstream
  template consumers
  **Location**: Prerequisites P.1
- 🟡 major/medium: Quality guidelines field enumeration in Phase 2 skills may
  become inconsistent
  **Location**: Prerequisites P.2/P.3
- 🔵 minor/high: Quote stripping in ticket-read-field.sh may affect tag values
  containing quotes
  **Location**: Prerequisites P.4
- 🔵 minor/high: Parent normalisation and ticket-read-field.sh quote stripping
  interact correctly but implicitly
  **Location**: Subphase 3.1, Scenario 8
- 🔵 minor/medium: Adding a new frontmatter field to a legacy ticket requires
  insertion position awareness
  **Location**: Subphase 3.2, Scenario 28
- 🔵 suggestion/medium: Display text conversion rules are LLM-dependent and not
  deterministic
  **Location**: Subphase 3.2, Scenario 27
- 🔵 suggestion/high: Hardcoded fallback values in ticket-template-field-hints.sh
  create a shadow contract
  **Location**: Prerequisites P.5

### Safety

**Summary**: The plan demonstrates strong safety awareness for a developer
tooling context. The update-ticket skill includes confirmation previews
before all writes, hard-blocks on identity field changes, and warns on
sensitive field edits. The list-tickets skill is read-only. Two areas warrant
attention: the multi-step write in update-ticket has an acknowledged
partial-write risk with no rollback, and the new ticket-update-tags.sh script
could operate on invalid input if file/frontmatter errors are not
distinguished from absent fields.

**Strengths**:

- ✅ Confirmation preview before every write operation
- ✅ Hard-block on ticket_id changes prevents the most dangerous mutation
- ✅ Warning gate on date field edits
- ✅ No-op detection prevents unnecessary writes
- ✅ list-tickets is explicitly read-only
- ✅ Malformed frontmatter handling aborts cleanly without auto-repair
- ✅ Status transition enforcement explicitly deferred
- ✅ Cycle detection prevents infinite loops in hierarchy mode

**Findings**:

- 🟡 major/high: Partial write risk with no rollback on body sync failure
  **Location**: Subphase 3.2, Step 5
- 🔵 minor/medium: Tag script outputs new value but does not write — caller
  must coordinate read-modify-write
  **Location**: Prerequisites P.4
- 🔵 minor/high: Adding a new frontmatter field requires inserting a line at
  the right position
  **Location**: Subphase 3.2, Scenario 28
- 🔵 suggestion/medium: Two-strike confirmation could be more explicit about
  defaulting to decline
  **Location**: Subphase 3.2, Step 4
- 🔵 suggestion/medium: Prerequisite changes to Phase 2 skills lack explicit
  verification of backward compatibility
  **Location**: Prerequisites P.2 and P.3

## Re-Review (Pass 2) — 2026-04-21

**Verdict:** COMMENT

### Previously Identified Issues

#### Major (4 from initial review)

- ✅ **Correctness / Code Quality**: Block-style YAML detection in
  `ticket-update-tags.sh` — **Resolved**. P.4 now has three pre-checks
  (file existence, frontmatter validity, raw-file block-style detection)
  before delegating to `ticket-read-field.sh`.
- ✅ **Correctness**: Absent tags field vs file/frontmatter error
  disambiguation — **Resolved**. Pre-checks validate file and frontmatter
  before calling `ticket-read-field.sh`, so exit-1 unambiguously means
  "field absent."
- ✅ **Compatibility**: Phase 2 quality-guidelines field enumeration —
  **Resolved**. P.2 and P.3 now explicitly include updating field
  enumerations to add `title:`.
- ✅ **Test Coverage / Correctness**: Absent-field insertion eval —
  **Resolved**. New Scenario 34 covers `/update-ticket 0011 priority high`
  on a legacy ticket with no `priority:` line.

#### Minor findings addressed

- ✅ **Safety / Architecture**: Non-atomic write warning — **Resolved**.
  Step 5 warning now includes `jj restore <filename>` remediation.
- ✅ **Code Quality / Usability / Correctness**: Display text conversion
  — **Resolved**. Changed from "English language judgement" to deterministic
  rule (replace hyphens with spaces, apply title case).
- ✅ **Correctness / Compatibility / Safety**: Field insertion position —
  **Resolved**. Step 5 specifies "last line before closing `---`."
- ✅ **Correctness**: Multi-op no-op behaviour — **Resolved**. Clarified
  as per-field; "no change needed" exit only when ALL ops are no-ops.
- ✅ **Standards**: Scenario numbering inconsistency — **Resolved**.
  Renumbered to flat integers (1-19 for list-tickets, 1-35 for
  update-ticket).
- ✅ **Test Coverage**: P.4 per-delimiter quoting tests — **Resolved**.
  Expanded to individual tests for comma, colon, hash.
- ✅ **Test Coverage**: P.5 config-read-template failure test — **Resolved**.
  Added test for template-missing fallback and tripwire test for
  fallback-vs-template sync.
- ✅ **Compatibility**: Custom template changelog note — **Resolved**.
  P.1 now includes release-notes guidance.
- ✅ **Usability**: Block-style error message example — **Resolved**.
  Error now includes `Example: tags: [api, search]`.

#### Findings still present (carried forward, not regressions)

- 🟡 **Architecture**: Filter resolution couples to template comment syntax
  — **Partially resolved**. P.5 fallback chain mitigates missing comments;
  the comment-format dependency remains implicit but is documented.
- 🟡 **Test Coverage**: No eval for non-matching files in tickets directory
  — **Still present**. No scenario verifies that `README.md` or other
  non-ticket files in `{tickets_dir}` are excluded from the listing.
- 🟡 **Usability**: Legacy status values require explicit-form syntax —
  **Still present**. `/list-tickets todo` falls to title search rather than
  status filter. A zero-result hint would improve discoverability.

### New Issues Introduced

- ✅ **Correctness** (minor): Bare `tags:` — **Resolved**. Added explicit
  note to P.4 mutation rules that bare `tags:` is treated as `tags: []`.
- ✅ **Correctness** (minor): Block-style detection regex — **Resolved**.
  Widened from `^  - ` to `^[[:space:]]+- ` to catch any indentation.
- ✅ **Code Quality** (minor): Title-case small words — **Resolved**.
  Enumerated explicit small-words set in the display text conversion rule.
- ✅ **Usability** (minor): Scenario 31 error message — **Resolved**.
  Updated to include the `Example: tags: [api, search]` suffix matching
  P.4's spec.
- ✅ **Test Coverage** (minor): Partial no-op multi-op — **Resolved**.
  Added Scenario 36 covering a multi-op where one field is already at the
  target value and the other is not.

### Assessment

The plan is now in good shape. All 4 major findings from the initial
review-2 pass are resolved, along with 10 minor findings. The 3 remaining
major-severity items are carryovers that represent accepted design
tradeoffs or narrow gaps rather than structural issues:

- The filter-template coupling is mitigated by P.5's fallback chain
- The non-matching-files eval gap is narrow and covered by the existing
  `ticket-next-number.sh` test precedent
- The legacy status discoverability is a UX convenience, not a correctness
  problem

All 5 new minor issues from the re-review have been resolved. The plan
is ready for implementation.

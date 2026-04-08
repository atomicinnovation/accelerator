---
date: "2026-04-08T15:15:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-08-ticket-management-phase-1-foundation.md"
review_number: 1
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability]
review_pass: 2
status: complete
---

## Plan Review: Ticket Management Phase 1 — Foundation and Configuration

**Verdict:** REVISE

The plan is well-structured, follows established ADR script patterns faithfully,
and demonstrates a strong TDD methodology with ~35 tests across three companion
scripts. However, it has one critical gap — adding `templates/ticket.md` will
break existing config test assertions that hardcode a count of 5 templates — and
several major omissions: multiple files that enumerate template keys explicitly
are not updated, the generic field reader has a regex injection vulnerability
already mitigated elsewhere in the codebase, and the argument order of
`ticket-read-field.sh` is inconsistent with its sibling script.

### Cross-Cutting Themes

- **Regex injection in `ticket-read-field.sh`** (flagged by: Architecture, Code
  Quality, Correctness, Standards, Usability) — The field name is interpolated
  directly into `grep -qE` and `sed` regex patterns without escaping. The
  codebase already mitigates this in `config-read-value.sh` using `awk` with
  `substr()` string comparison. This is the most widely-flagged concern.

- **Incomplete template key enumeration updates** (flagged by: Standards) — The
  plan correctly updates `skills/config/configure/SKILL.md` but misses three
  other files that explicitly enumerate template keys: `scripts/config-dump.sh`,
  `scripts/config-read-template.sh`, and `README.md`. Additionally,
  `scripts/test-config.sh` hardcodes a count of 5 templates that will break.

- **Code duplication across script categories** (flagged by: Architecture, Code
  Quality, Test Coverage) — Both the frontmatter-parsing logic (duplicated
  between `ticket-read-status.sh` and `ticket-read-field.sh`, and again with
  `adr-read-status.sh`) and the test helper functions (`assert_eq`,
  `assert_exit_code`, `setup_repo`) are copied verbatim rather than shared.

### Tradeoff Analysis

- **Consistency vs DRY**: The plan explicitly prioritises pattern consistency
  with ADR scripts over DRY principles. This is a reasonable initial choice
  that lowers the barrier to implementation, but should be acknowledged as
  intentional technical debt with a path to extraction later.

### Findings

#### Critical

- 🔴 **Standards**: Adding `templates/ticket.md` will break existing config test
  assertions
  **Location**: Subphase 1C: Ticket template
  `scripts/test-config.sh` contains hardcoded assertions expecting exactly 5
  templates (line ~3122: `assert_eq "5 template rows" "5" "$LINE_COUNT"`) and
  template key iteration loops covering only 5 keys. Adding `templates/ticket.md`
  causes `config_enumerate_templates` to return 6, breaking these tests.

#### Major

- 🟡 **Architecture, Code Quality, Correctness, Standards, Usability**: Regex
  injection via unsanitized field name in `ticket-read-field.sh`
  **Location**: Subphase 1B: ticket-read-field.sh implementation (line 478)
  The field name is interpolated directly into `grep -qE "^${FIELD_NAME}:"` and
  `sed "s/^${FIELD_NAME}:..."`. Field names with regex metacharacters (e.g.,
  `sub.type`) would match unintended patterns. The codebase already mitigates
  this in `config-read-value.sh` using `awk` with `substr()`.

- 🟡 **Architecture, Code Quality**: Duplicated frontmatter parsing logic
  between `ticket-read-status.sh` and `ticket-read-field.sh`
  **Location**: Subphase 1B: Ticket Field Reading Scripts
  Both scripts contain identical ~30-line frontmatter-parsing state machines.
  The plan acknowledges `ticket-read-field.sh` "subsumes the status reader" but
  keeps both. `ticket-read-status.sh` could delegate to `ticket-read-field.sh`
  instead of duplicating the logic.

- 🟡 **Standards**: Plan misses template key enumerations in `config-dump.sh`,
  `config-read-template.sh`, and `README.md`
  **Location**: Subphase 1C: Template, Plugin Registration, and Config Updates
  Three additional files explicitly enumerate template keys: `config-dump.sh`
  `TEMPLATE_KEYS` array (line ~209), `config-read-template.sh` comment header
  (line 7), and `README.md` (line ~183). None are mentioned in the plan.

- 🟡 **Usability**: Argument order inconsistency between `ticket-read-field.sh`
  and `ticket-read-status.sh`
  **Location**: Subphase 1B: ticket-read-field.sh argument order
  `ticket-read-field.sh` takes `<field-name> <file>` while `ticket-read-status.sh`
  takes `<file>`. Developers who learn the status script will guess `<file>
  <field>` for the generic version. Consider file-first ordering or named flags.

- 🟡 **Test Coverage**: No automated tests for Subphase 1C configuration changes
  **Location**: Subphase 1C: Template, Plugin Registration, and Config Updates
  Subphase 1C changes 5 files but relies on grep-based manual verification
  rather than repeatable automated tests. A lightweight integration test (e.g.,
  verifying `config-read-template.sh ticket` resolves) would provide regression
  coverage.

#### Minor

- 🔵 **Test Coverage, Correctness**: Test 11 for extra-digit filenames is
  underspecified
  **Location**: Subphase 1A: Test harness, Test 11
  "Files with extra digits (00003-foo.md) — handles correctly" doesn't specify
  the expected output. The glob `[0-9][0-9][0-9][0-9]-*` will match
  `00003-foo.md` (matching first four `0`s), extracting `00003`. The expected
  result should be explicit.

- 🔵 **Test Coverage, Correctness, Usability**: `--count` without a value causes
  opaque error
  **Location**: Subphase 1A: ticket-next-number.sh argument parsing
  `shift 2` under `set -euo pipefail` produces an "unbound variable" bash error
  rather than the friendly usage message. Add a `[ $# -lt 2 ]` guard.

- 🔵 **Architecture, Code Quality, Test Coverage**: Test helper functions
  duplicated across test harnesses
  **Location**: Subphase 1A: Test harness
  `assert_eq`, `assert_exit_code`, and `setup_repo` are copied from
  `test-adr-scripts.sh`. Consider extracting to a shared `test-helpers.sh`.

- 🔵 **Architecture, Standards**: Template frontmatter schema diverges from
  existing ticket convention
  **Location**: Subphase 1C: Ticket template
  The proposed template introduces `ticket_id`, `date`, `author`, `priority`,
  `parent`, `tags` — fields not present in the 25 existing tickets (which have
  only `title`, `type`, `status`). Document this schema evolution explicitly.

- 🔵 **Code Quality, Test Coverage**: No test for field names containing regex
  metacharacters
  **Location**: Subphase 1B: ticket-read-field.sh test cases
  Adding a test with a field name like `sub.type` would immediately reveal the
  regex injection issue and drive the fix.

- 🔵 **Usability**: Generic field reader error message less helpful than status
  reader
  **Location**: Subphase 1B: ticket-read-field.sh error message
  `ticket-read-status.sh` lists expected values; `ticket-read-field.sh` gives
  only "No field found." Consider listing available frontmatter fields.

- 🔵 **Test Coverage**: No test for field name that is a substring of another
  field name
  **Location**: Subphase 1B: Tests for ticket-read-field.sh
  A fixture with both `type: story` and `type_detail: extended` would verify
  the colon anchor prevents false matches.

- 🔵 **Test Coverage**: Array value test should clarify quote-stripping behavior
  **Location**: Subphase 1B: Tests for ticket-read-field.sh, Test 7
  The test says "outputs `[a, b]` raw" but the implementation strips quotes.
  The test fixture should be explicit about whether the YAML value is quoted.

- 🔵 **Test Coverage**: No test for CRLF line endings in frontmatter
  **Location**: Subphase 1B: Tests for ticket-read-status.sh
  The parser uses `[ "$line" = "---" ]` which fails on `---\r`. Consistent with
  ADR scripts but worth noting as a known limitation.

#### Suggestions

- 🔵 **Code Quality**: Quote stripping applied unconditionally may affect
  array-like values
  **Location**: Subphase 1B: ticket-read-field.sh implementation
  Document the quote-stripping behavior in the script's usage comment.

- 🔵 **Usability**: Template placeholder guidance could be clearer about what to
  keep vs remove
  **Location**: Subphase 1C: Ticket template
  Type-conditional guidance (e.g., "[For stories: ...]", "[For epics: ...]")
  requires manual selective editing. Consider a brief comment.

- 🔵 **Usability**: Init skill directory count in prose is a maintenance burden
  **Location**: Subphase 1C: Init skill update
  The hardcoded "12 directories" must be updated for every new path key.
  Consider removing the count from prose.

- 🔵 **Correctness**: Missing test for field present only in body (not
  frontmatter)
  **Location**: Subphase 1B: ticket-read-field.sh test specification
  A test with a field only in the body (not frontmatter) would verify the
  parser correctly ignores body content.

### Strengths

- ✅ Excellent structural consistency with the established decisions/ category —
  script naming, directory layout, plugin registration, and configuration all
  mirror proven patterns
- ✅ Strong TDD approach with test-first ordering and ~35 tests covering happy
  paths, edge cases, error conditions, and boundary scenarios
- ✅ Clear dependency ordering across subphases (1A and 1B independent, 1C
  depends on directory existing)
- ✅ Explicit "What We're NOT Doing" section prevents scope creep with clear
  rationale for each exclusion
- ✅ Comprehensive edge case coverage for numbering (gaps, overflow, mixed files)
  and frontmatter parsing (unclosed, empty values, body-vs-frontmatter)
- ✅ Clean integration with existing configuration infrastructure via
  `config-read-path.sh` and `config-read-template.sh`
- ✅ Both automated verification (test harness) and manual verification against
  real data are specified for each subphase

### Recommended Changes

1. **Update `scripts/test-config.sh` template assertions** (addresses: config
   test breakage)
   Change count assertions from 5 to 6 and add `ticket` to template key
   iteration loops in `scripts/test-config.sh`.

2. **Fix regex injection in `ticket-read-field.sh`** (addresses: regex injection)
   Use `grep -qF "${FIELD_NAME}:"` with an awk-based approach matching
   `config-read-value.sh`, or escape the field name before interpolation. Apply
   the same fix to the `sed` substitution.

3. **Have `ticket-read-status.sh` delegate to `ticket-read-field.sh`**
   (addresses: duplicated frontmatter parsing)
   Replace the duplicated parsing logic with delegation:
   `exec "$SCRIPT_DIR/ticket-read-field.sh" status "$1"`, customizing only the
   error message.

4. **Update template key enumerations in 3 additional files** (addresses:
   incomplete template key updates)
   Add `ticket` to: `scripts/config-dump.sh` `TEMPLATE_KEYS` array,
   `scripts/config-read-template.sh` comment header, and `README.md` template
   keys list.

5. **Reconsider `ticket-read-field.sh` argument order** (addresses: argument
   order inconsistency)
   Either change to `<file> <field>` for consistency with `ticket-read-status.sh`,
   or add a clear rationale for the current ordering in the plan.

6. **Clarify Test 11 expected output** (addresses: underspecified test)
   State the expected output explicitly (e.g., "00003-foo.md is matched,
   outputs 0004" or "ignored, outputs 0001").

7. **Guard `--count` for missing value** (addresses: opaque error)
   Add `[ $# -lt 2 ]` check in the `--count` case branch before accessing `$2`.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan follows established architectural patterns faithfully and
demonstrates strong structural consistency with the existing decisions/ category.
The modular decomposition into three subphases with clear dependency ordering is
sound. However, there is a missed opportunity to extract shared
frontmatter-parsing logic into a reusable module, and the relationship between
ticket-read-status.sh and ticket-read-field.sh introduces a cohesion concern
worth addressing before the foundation solidifies.

**Strengths**:
- Excellent architectural consistency: the plan mirrors the decisions/ category
  structure almost exactly, which makes the system predictable and reduces
  cognitive load for contributors
- Clear dependency ordering across subphases: 1A and 1B are independent, while
  1C depends on the scripts directory existing
- Explicit scoping of what is NOT being done protects against scope creep
- Use of config-read-path.sh for dynamic path resolution maintains the
  open-closed principle
- TDD approach with test-first ordering ensures structural contracts are
  verified before implementation

**Findings**:

**Major — Duplicated frontmatter parsing logic across three scripts**
(confidence: high)
Location: Subphase 1B: Ticket Field Reading Scripts

The plan introduces `ticket-read-status.sh` and `ticket-read-field.sh`, both
containing identical YAML frontmatter parsing loops. Combined with the existing
`adr-read-status.sh`, this creates three copies of the same state machine. As the
plugin grows, every new artifact category that needs frontmatter reading will copy
this logic again. Suggestion: extract into a shared function in
`scripts/frontmatter-common.sh`.

**Minor — ticket-read-status.sh adds no value over ticket-read-field.sh**
(confidence: high)
Location: Subphase 1B: Ticket Field Reading Scripts

`ticket-read-field.sh status <file>` provides identical functionality. Having
both means two scripts to maintain and test when one suffices. Consider making
`ticket-read-status.sh` a one-line wrapper that delegates to
`ticket-read-field.sh`.

**Minor — Regex injection risk in ticket-read-field.sh field name matching**
(confidence: medium)
Location: Subphase 1B: Ticket Field Reading Scripts

Field name interpolated directly into `grep -qE "^${FIELD_NAME}:"`. The existing
`config-read-value.sh` explicitly avoids this using `awk` with string comparison.
Suggestion: use `grep -qF` or the awk approach.

**Suggestion — Template frontmatter diverges from existing ticket schema**
(confidence: medium)
Location: Subphase 1C: Template

Two different ticket schemas will coexist without documentation. Consider
documenting the schema evolution and whether `ticket-read-field.sh` should
support a `--default` flag.

**Suggestion — Test harness helpers are duplicated from ADR test harness**
(confidence: medium)
Location: Subphase 1A: Ticket Numbering Script

Shared helpers could be extracted to `scripts/test-common.sh`. Low-priority
improvement that could be deferred.

### Code Quality

**Summary**: The plan follows established patterns well and demonstrates sound
TDD methodology. The primary code quality concerns are: near-total duplication
between ticket-read-status.sh and ticket-read-field.sh, regex injection risk from
unsanitized field names, and duplicated test infrastructure.

**Strengths**:
- Strict adherence to established codebase conventions
- TDD approach with comprehensive test coverage (~35 tests)
- Clean separation of concerns across subphases
- Well-defined success criteria with automated and manual verification
- Explicit "What We're NOT Doing" section prevents scope creep

**Findings**:

**Major — Near-complete code duplication between status and field reading
scripts** (confidence: high)
Location: Subphase 1B: ticket-read-status.sh and ticket-read-field.sh

Identical frontmatter parsing logic in both scripts. Suggestion: implement
`ticket-read-field.sh` as the canonical parser and have `ticket-read-status.sh`
delegate to it.

**Major — Unsanitized field name used in regex pattern** (confidence: high)
Location: Subphase 1B: ticket-read-field.sh implementation

User-supplied `FIELD_NAME` interpolated directly into extended regex. Suggestion:
use `grep -qF` or escape the field name before interpolation.

**Minor — Test helper functions duplicated across test harnesses**
(confidence: medium)
Location: Subphase 1A: test-ticket-scripts.sh

`assert_eq`, `assert_exit_code`, and `setup_repo` copied verbatim from
`test-adr-scripts.sh`. Consider extracting to a common script.

**Suggestion — No test for field names containing regex metacharacters**
(confidence: medium)
Location: Subphase 1B: ticket-read-field.sh test cases

A test with a field name like `sub.type` would reveal the regex injection issue.

**Suggestion — Quote stripping applied unconditionally** (confidence: low)
Location: Subphase 1B: ticket-read-field.sh implementation

Document the quote-stripping behavior in the script's usage comment.

### Test Coverage

**Summary**: The plan demonstrates a strong test-first approach with
comprehensive coverage. Test cases cover happy paths, edge cases, error
conditions, and boundary scenarios mirroring the ADR test harness. Gaps exist in
edge case coverage for argument parsing, regex injection, and Subphase 1C has no
automated tests.

**Strengths**:
- TDD approach explicitly enforced with test-first ordering
- Comprehensive edge case coverage for numbering script
- Good error path coverage across all three scripts
- Test isolation with temporary directories and cleanup
- Both automated and manual verification steps

**Findings**:

**Major — No automated tests for Subphase 1C configuration changes**
(confidence: high)
Location: Subphase 1C: Template, Plugin Registration, and Config Updates

Five files changed with only grep-based manual verification. Consider adding
integration tests for template resolution and path configuration.

**Minor — Test 11 for extra-digit filenames is underspecified** (confidence: high)
Location: Subphase 1A: Test harness, Test 11

"Handles correctly" doesn't specify the expected output. The glob behavior should
be documented and the test made explicit.

**Minor — No test for --count without a value argument** (confidence: high)
Location: Subphase 1A: Test harness

`--count` as final argument would cause `shift 2` to fail with an opaque bash
error.

**Minor — No test for field names containing regex metacharacters**
(confidence: medium)
Location: Subphase 1B: Tests for ticket-read-field.sh

Testing `sub.type` as a field name would exercise the regex injection path.

**Minor — Array value test should clarify quote-stripping behavior**
(confidence: high)
Location: Subphase 1B: Tests for ticket-read-field.sh, Test 7

Test description says "raw" but implementation does perform quote stripping.
The fixture should be explicit.

**Minor — No test for CRLF line endings** (confidence: medium)
Location: Subphase 1B: Tests for ticket-read-status.sh

`[ "$line" = "---" ]` would fail on `---\r`. Consistent with ADR scripts but
worth noting.

**Minor — No test for field name that is a substring of another field name**
(confidence: high)
Location: Subphase 1B: Tests for ticket-read-field.sh

A fixture with `type` and `type_detail` would verify the colon anchor prevents
false matches.

**Suggestion — Test helper functions duplicated** (confidence: medium)
Location: Subphase 1A: Test harness

Consider extracting shared test helpers into a common file.

### Correctness

**Summary**: The plan closely follows established ADR script patterns, providing
high confidence in happy-path logic. The generic `ticket-read-field.sh` script
introduces a regex injection vulnerability through unsanitized field names — a
category of issue the codebase has already identified and mitigated in
`config-read-value.sh`. Test specifications are thorough with one ambiguous case
and one missing edge case.

**Strengths**:
- Meticulous mirroring of proven ADR patterns with clear comparison table
- Comprehensive test specifications with 35+ tests
- Correct use of `$((10#$NUM))` for base-10 interpretation
- Glob pattern correctly discriminates ticket files from non-ticket files

**Findings**:

**Major — Regex metacharacter injection via unsanitized field name**
(confidence: high)
Location: Subphase 1B: ticket-read-field.sh implementation (lines 478-479)

Field name interpolated into `grep -qE` and `sed` regex patterns. The codebase
already recognizes this risk in `config-read-value.sh`. Use `grep -qF` or awk
with `substr()`.

**Minor — Ambiguous expected behavior for extra-digit filenames**
(confidence: high)
Location: Subphase 1A: Test specification, Test 11

Test description doesn't specify expected output. The glob pattern behavior
should be explicit.

**Minor — Missing test case for --count as final argument without value**
(confidence: medium)
Location: Subphase 1A: ticket-next-number.sh argument parsing

`$2` unbound under `set -u` produces bash diagnostic rather than usage message.

**Suggestion — Missing test for field present only in body** (confidence: medium)
Location: Subphase 1B: ticket-read-field.sh test specification

A test with a field only in the body would document the parser's
frontmatter-only invariant.

### Standards

**Summary**: The plan demonstrates strong adherence to project conventions for
script structure, test harness design, and directory layout. However, it misses
several locations where template keys are hardcoded — most critically in
`scripts/test-config.sh` where adding `templates/ticket.md` would break existing
assertions expecting exactly 5 templates.

**Strengths**:
- Script implementations closely mirror ADR script patterns
- Test harness follows exact structure of `test-adr-scripts.sh`
- File naming follows `<entity>-<verb>-<noun>.sh` convention
- Plugin registration placement follows alphabetical/categorical ordering
- TDD approach and subphase ordering follow project methodology

**Findings**:

**Critical — Adding templates/ticket.md will break existing config test
assertions** (confidence: high)
Location: Subphase 1C: Ticket template

`scripts/test-config.sh` hardcodes assertions expecting 5 templates (count
assertions and key iteration loops). Adding the ticket template breaks these
tests.

**Major — config-dump.sh TEMPLATE_KEYS array omits ticket** (confidence: high)
Location: Subphase 1C: Ticket template

`scripts/config-dump.sh` hardcodes 5 template keys without `ticket`.
Configuration diagnostics will be incomplete.

**Major — config-read-template.sh comment header omits ticket**
(confidence: high)
Location: Subphase 1C: Ticket template

Line 7 lists template names explicitly without `ticket`. Developers reading
the script header will not see it as a supported key.

**Major — README.md template key list not updated** (confidence: high)
Location: Subphase 1C: Template, Plugin Registration, and Config Updates

Line ~183 lists available template keys without `ticket`.

**Minor — Template frontmatter schema diverges from existing ticket convention**
(confidence: medium)
Location: Subphase 1C: Ticket template

Proposed template introduces fields not present in existing tickets. The schema
divergence should be documented.

**Suggestion — ticket-read-field.sh uses field name in grep regex without
escaping** (confidence: medium)
Location: Subphase 1B: Ticket Field Reading Scripts

Consider `grep -qF` for fixed-string matching.

### Usability

**Summary**: The plan demonstrates strong consistency with existing ADR patterns,
making it predictable. Scripts have clear responsibilities, sensible defaults, and
actionable error messages. Two friction points: the argument order of
`ticket-read-field.sh` reverses the sibling script's convention, and the generic
field reader's error message provides less guidance than the status-specific one.

**Strengths**:
- Consistent structural parallel with ADR scripts enables transferability of
  mental models
- Sensible defaults throughout (missing directory returns 0001 with warning)
- Error messages include the problematic value and usage hints
- Progressive disclosure: dedicated status script for common case, generic
  field reader for general case
- Configuration changes use existing mechanisms with no new concepts

**Findings**:

**Major — Argument order inconsistency between ticket-read-field.sh and
ticket-read-status.sh** (confidence: high)
Location: Subphase 1B: ticket-read-field.sh argument order

`ticket-read-field.sh` takes `<field> <file>` while `ticket-read-status.sh`
takes `<file>`. Consider file-first ordering or named flags.

**Minor — Generic field reader error message less helpful than status reader**
(confidence: high)
Location: Subphase 1B: ticket-read-field.sh error message

`ticket-read-status.sh` lists expected values; `ticket-read-field.sh` only says
"No field found." Consider listing available frontmatter fields.

**Minor — Field name used in grep regex without escaping** (confidence: medium)
Location: Subphase 1B: ticket-read-field.sh

Creates surprising failures for field names with regex metacharacters.

**Minor — --count without value causes opaque error** (confidence: high)
Location: Subphase 1A: ticket-next-number.sh

`shift 2` under `set -euo pipefail` produces bash diagnostic. Add a guard.

**Suggestion — Template placeholder guidance could be clearer** (confidence: medium)
Location: Subphase 1C: Ticket template

Type-conditional guidance requires manual selective editing. Consider a brief
HTML comment.

**Suggestion — Init skill directory count in prose is a maintenance burden**
(confidence: medium)
Location: Subphase 1C: Init skill update

Hardcoded "12 directories" must be updated for every new path key. Consider
removing the count from prose.

## Re-Review (Pass 2) — 2026-04-08

**Verdict:** REVISE

### Previously Identified Issues

- ✅ **Standards** (was critical): Config test assertions will break — **Partially
  resolved**. The plan now updates 4 assertions in `test-config.sh` (Sections
  7a-7d), but misses additional `config_enumerate_templates` assertions at lines
  ~2972-2974.
- ✅ **Architecture, Code Quality, Correctness, Standards, Usability** (was
  major): Regex injection in `ticket-read-field.sh` — **Resolved**. Now uses bash
  string comparison and parameter expansion.
- ✅ **Architecture, Code Quality** (was major): Duplicated frontmatter parsing —
  **Resolved**. `ticket-read-status.sh` delegates via `exec`.
- ✅ **Standards** (was major): Missing template key enumerations — **Resolved**.
  Updates added for `config-dump.sh`, `config-read-template.sh`, and `README.md`.
- ✅ **Usability** (was major): Argument order inconsistency — **Resolved**. Kept
  as-is with documented rationale.
- ✅ **Test Coverage** (was major): No automated tests for Subphase 1C —
  **Resolved**. Config test update section (Section 7) added.
- 🟡 **Test Coverage, Correctness** (was minor): Test 11 underspecified —
  **Partially resolved**. Expected behavior is now documented, but the described
  behavior is incorrect — the glob `[0-9][0-9][0-9][0-9]-*` does NOT match
  `00003-foo.md` (position 5 is `3`, not `-`). The file would be ignored and
  output would be `0001`, not `0004`.
- ✅ **Test Coverage, Correctness, Usability** (was minor): `--count` guard —
  **Resolved**. Guard added with clear error message.
- ⚪ **Architecture, Code Quality, Test Coverage** (was minor): Test helper
  duplication — Still present, accepted as intentional.
- ⚪ **Architecture, Standards** (was minor): Template schema divergence — Still
  present, accepted.
- ✅ **Code Quality, Test Coverage** (was minor): No regex metacharacter test —
  **Resolved**. No longer needed since regex is not used.
- ⚪ **Usability** (was minor): Generic error message — Still present.
- ⚪ **Test Coverage** (was minor): Substring field test — Still present.
- ⚪ **Test Coverage** (was minor): Array value test clarity — Still present.
- ⚪ **Test Coverage** (was minor): CRLF handling — Still present, accepted.
- ⚪ **Usability** (was suggestion): Template placeholder guidance — Still present.
- ⚪ **Usability** (was suggestion): Init skill directory count — Still present.

### New Issues Introduced

- 🟡 **Correctness**: Test 11 describes incorrect glob matching behavior for
  5-digit filenames. The glob `[0-9][0-9][0-9][0-9]-*` does NOT match
  `00003-foo.md` — position 5 is `3` not `-`. The expected output should be
  `0001` (file ignored), not `0004`.

- 🟡 **Test Coverage, Correctness**: Missing `config_enumerate_templates`
  assertion updates in `test-config.sh` at lines ~2972-2974. The count assertion
  (`"outputs 5 keys"`) and missing `ticket` assertion will break.

- 🟡 **Architecture, Standards, Correctness**: Missing `config-dump.sh`
  `PATH_KEYS`/`PATH_DEFAULTS` update for `paths.review_tickets`. The plan adds
  `review_tickets` to `config-read-path.sh`, `init/SKILL.md`, and
  `configure/SKILL.md`, but not to the hardcoded arrays in `config-dump.sh`.

### Assessment

The plan has improved substantially — 6 of 6 major findings and the critical
finding from review 1 are resolved or substantially resolved. The remaining
issues are targeted gaps: one incorrect test description, two additional
`test-config.sh` assertions, and a missing `PATH_KEYS` entry. All three are
straightforward to fix. After these corrections, the plan should be ready for
implementation.

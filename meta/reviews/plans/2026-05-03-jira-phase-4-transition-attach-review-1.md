---
date: "2026-05-03T00:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-03-jira-phase-4-transition-attach.md"
review_number: 1
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, standards, safety, security, usability]
review_pass: 2
status: complete
---

## Plan Review: Jira Phase 4 — Workflow and Attachments

**Verdict:** REVISE

The plan is clearly structured and inherits strong codebase conventions from Phases 1–3: TDD sequencing with an enforced must-fail gate, the `consume: false` dry-run guard pattern, a pre-reserved exit-code namespace, and SKILL.md frontmatter that matches existing write-skill templates exactly. However, a single design decision — that `jira-transition-flow.sh --describe` exits before making the `GET /transitions` call — creates a cascade of interconnected problems across six lenses: disambiguation cannot surface at preview time, the describe output cannot show the resolved transition ID (even when `--transition-id` is supplied), the success render has no state name to display in the `--transition-id`-only path, the describe guard fixture is wired to the wrong request type, and the SKILL.md's disambiguation step will only trigger unexpectedly post-confirmation rather than pre-confirmation. This core issue, plus path-injection and jq-injection risks in the two new flow scripts, requires plan revisions before implementation begins.

---

### Cross-Cutting Themes

- **`--describe` GET elision** (flagged by: Correctness [critical], Architecture, Code Quality, Safety, Test Coverage, Usability) — The decision to skip the `GET /transitions` lookup during `--describe` is the single largest thread in this review. Every downstream problem — null `transition_id` in the preview, disambiguation surfacing post-confirmation, the guard fixture targeting the wrong request, the success render failing for the `--transition-id`-only path — traces back to it. Resolving the root causes resolves or simplifies most related findings.

- **Conditional `jira-body-input.sh` sourcing** (flagged by: Architecture, Code Quality, Standards) — All three lenses independently identified the conditional mid-script sourcing of `jira-body-input.sh` as a deviation from the established unconditional top-of-file pattern used by every other flow script.

- **Guard fixture accuracy** (flagged by: Test Coverage, Correctness, Standards) — The transition describe guard targets a `GET` that the algorithm never makes in describe mode; additionally both new guard fixtures omit `capture_url: true` present in all existing guards.

- **File existence validation ordering** (flagged by: Architecture, Safety) — Placing file validation after the `--describe` short-circuit means `--describe` can succeed with non-existent file paths, making the confirmation preview inaccurate.

### Tradeoff Analysis

- **Eager GET in `--describe` vs. pure static describe**: Making `--describe` perform the GET would fully resolve the critical finding and several major findings, at the cost of a network call on the dry-run path. Every other write skill (update, comment-add, comment-edit) performs full body resolution during dry-run — this would align with that pattern. The alternative (accepting the static describe and adjusting the SKILL.md to handle mid-execution disambiguation) produces a more complex skill that breaks the established describe-then-confirm contract. The review recommends eager GET as the simpler path.

---

### Findings

#### Critical

- 🔴 **Correctness + Architecture + Safety**: Disambiguation cannot surface at describe/preview time
  **Location**: Phase 2: Algorithm step 2; Phase 3: SKILL.md Step 6
  The `--describe` algorithm exits before making the `GET /transitions` call where ambiguity is detected. Exit 123 (ambiguous state) will only surface during the live POST invocation in SKILL.md Step 8 — after the user has already confirmed. The confirmation gate loses its meaning for the state-name lookup path: the user confirms a preview that cannot tell them which transition (or whether a valid transition exists at all) will be applied. The SKILL.md's disambiguation block will never be triggered by the describe call; it can only be triggered by an unexpected exit 123 arriving at Step 8's error handler, which has no defined recovery behaviour.

#### Major

- 🟡 **Correctness + Code Quality + Usability**: `--describe` output always emits `transition_id: null` even when `--transition-id` is supplied
  **Location**: Phase 2: Algorithm step 2
  When the caller passes `--transition-id 21`, the describe output should show `"transition_id": "21"` — the ID is already known and does not require a GET. Emitting `null` is a factual inaccuracy in the preview, and Phase 3 Step 5 will always fall back to rendering "by state name" even when the ID is explicitly known. The `resolution` and `comment` fields have the same problem: the plan's example shows `null` and `false` hardcoded, rather than reflecting the actual parsed flag values.

- 🟡 **Correctness**: Success render references `STATE_NAME` which may be absent in `--transition-id`-only invocations
  **Location**: Phase 3: SKILL.md Step 9
  The success message `✓ **<KEY>** transitioned to "<STATE_NAME>"` has no state name to display when only `--transition-id` was supplied. The 204 response has no body; the state name cannot be recovered from it. The plan does not address this case.

- 🟡 **Correctness + Usability**: `STATE_NAME` positional marked mandatory in usage signature but is optional when `--transition-id` is given
  **Location**: Phase 2: Usage signature and Algorithm steps 1–4
  The usage line `jira-transition-flow.sh [--describe] KEY STATE_NAME ...` makes `STATE_NAME` look mandatory. Exit code 121 correctly notes "and no `--transition-id`" but argument-parsing code written against the usage signature would consume a second positional as STATE_NAME, causing `jira-transition-flow.sh ENG-1 --transition-id 21` (a valid invocation per the SKILL.md spec) to exit 121 incorrectly.

- 🟡 **Architecture + Code Quality + Standards**: Conditional sourcing of `jira-body-input.sh` deviates from established unconditional pattern
  **Location**: Phase 2: Sourcing section
  Every existing flow script (`jira-update-flow.sh` line 54, `jira-comment-flow.sh` line 35) sources `jira-body-input.sh` unconditionally at the top of the file before any argument parsing. The plan proposes runtime conditional sourcing based on parsed flags — creating load-order dependency, making static analysis harder, and surprising future maintainers who check the top of the file for all dependencies. All three lenses (Architecture, Code Quality, Standards) identified this independently.

- 🟡 **Standards**: Guard fixtures omit `capture_url: true` present in all existing guard fixtures
  **Location**: Phase 1: `transition-describe-guard.json`; Phase 4: `attach-describe-guard.json`
  Both `comment-add-print-payload-guard.json` and `comment-delete-describe-guard.json` pair `"consume": false` with `"capture_url": true`. The plan's new guard fixtures specify only `"consume": false`, deviating from the established pattern and potentially weakening the guard's ability to detect accidental dry-run network calls.

- 🟡 **Test Coverage + Correctness**: Transition `--describe` guard targets a `GET` that the algorithm never makes
  **Location**: Phase 1: `transition-describe-guard.json`; Phase 1 Test script Case 4
  The guard scenario places `consume: false` on a `GET` request, but the `--describe` algorithm exits before making any GET. The guard therefore watches for a path that never occurs, while leaving the genuinely dangerous mistake — accidentally POSTing a real transition during `--describe` — unguarded. The attach describe guard correctly targets the POST; the transition guard should do the same.

- 🟡 **Test Coverage**: Exit codes 125 (`E_TRANSITION_NO_BODY`) and 126 (`E_TRANSITION_BAD_RESOLUTION`) defined but have no test cases
  **Location**: Phase 1: Test script Cases table vs. Phase 2 exit code table
  Both exit codes are defined in the Phase 2 table but absent from the 15-case test table. These are argument-validation paths requiring no mock server and are straightforward to add. Without them, a wrong exit code or wrong validation condition would go undetected.

- 🟡 **Test Coverage + Code Quality**: Two-file attach test can only assert exit code — actual file count sent is untestable
  **Location**: Phase 4: Test script Cases table, Case 2
  The plan acknowledges the mock cannot enumerate multipart parts. Exit 0 is the only assertion. A bug that silently drops all but the first `--multipart` argument would pass all 10 test cases. The multipart body contains `Content-Disposition: form-data; name="file"; filename=` per part, so a `capture_body: true` fixture and `grep -c 'filename='` count assertion is viable as a client-side check.

- 🟡 **Security**: No path sanitisation specified before `file=@<path>` multipart argument construction
  **Location**: Phase 5: Algorithm step 5
  FILE arguments from user input are passed directly as `file=@<path>` curl multipart parts after only an existence/readability check. Paths beginning with `-` (e.g. `-` itself, which reads from stdin) pass the readability check; symlinks to device files (`/dev/urandom`, `/proc/self/mem`) also pass; FIFOs would block the process. The plan should specify rejection of paths beginning with `-` and add a symlink-to-device guard.

- 🟡 **Security**: `STATE_NAME` used in jq filter — injection risk if string-interpolated
  **Location**: Phase 2: Algorithm step 5
  The plan does not specify how `STATE_NAME` is substituted into the jq filter expression. If interpolated into the filter string rather than passed via `--arg`, a crafted state name containing `"` or `\` could break out of the jq string literal and alter the filter logic — bypassing the ambiguity check (exit 123) or producing an unintended match. The plan should explicitly mandate `jq --arg state_name "$STATE_NAME"` with the variable referenced as `$state_name` in the filter, never string-interpolated.

- 🟡 **Usability**: Exit-123 disambiguation flow has no error path for malformed or empty stdout
  **Location**: Phase 3: SKILL.md Step 6
  Step 6 instructs parsing a JSON array from stdout, but specifies no fallback when stdout is empty or non-JSON. A developer following the plan exactly will implement no recovery behaviour, producing inconsistent results across implementations.

- 🟡 **Usability + Safety**: File size derivation via `Read` tool in attach preview is underspecified and fragile
  **Location**: Phase 6: SKILL.md Step 4
  Step 4 says to show "basename + size if checkable via `Read`." The `Read` tool reads file contents, not metadata — obtaining size requires reading the whole file, which is slow for large binaries. The phrase "if checkable" leaves recovery behaviour undefined, causing implementers to either skip size silently or read large files into memory. No other existing skill uses this pattern.

- 🟡 **Safety + Architecture**: File existence validation occurs after `--describe` exits in attach flow
  **Location**: Phase 5: Algorithm steps 2–3
  `--describe` short-circuits at step 2; file validation runs at step 3. A user running `--describe ENG-1 nonexistent.log` sees a clean preview listing the non-existent file, confirms, then gets exit 132 on the real invocation. The confirmation gate falsely signals readiness. Moving validation to before `--describe` (step 1.5) would make `--describe` also exit 132 for invalid paths, matching the transition flow's argument-first validation philosophy.

#### Minor

- 🔵 **Test Coverage**: Case 6 (`--transition-id` bypass) lacks an explicit named fixture
  **Location**: Phase 1: Desired end state file list and Cases table
  Case 6 asserts "only POST in mock" but the fixture list contains no single-POST-only scenario. The two-step fixtures both begin with a GET that would cause a mock server 500 when the GET is skipped. A `transition-post-204-direct.json` fixture (single POST expectation, no GET) should be added to the desired end state and referenced in the Case 6 row.

- 🔵 **Test Coverage + Code Quality**: Cases table does not specify which fixture file each case uses
  **Location**: Phase 1: Test script Cases table
  Cases 1, 2, 7, 8, and 9 all involve the same GET endpoint but require different fixture shapes. The plan leaves fixture-to-case mapping implicit, making incorrect wiring easy and the resulting failures hard to diagnose.

- 🔵 **Standards**: EXIT_CODES.md section heading not updated from "Phase 3 namespace summary"
  **Location**: Phase 2: EXIT_CODES.md update
  The plan instructs adding Phase 4 rows to the namespace summary but does not mention renaming the section heading, which will read "Phase 3 namespace summary" after the change.

- 🔵 **Standards**: Confirmation gate prose spec omits exact canonical phrasing
  **Location**: Phase 3 and Phase 6: SKILL.md skill-creator specs
  Both SKILL.md specs describe the gate as "standard gate: `y` → proceed, `n` → revise, other → abort" without quoting the canonical text used in `update-jira-issue/SKILL.md` and `comment-jira-issue/SKILL.md`: "Send this to Jira? Reply **y** to confirm, **n** to revise, anything else to abort."

- 🔵 **Usability**: STATE_NAME has no syntactic marker distinguishing it from a mandatory positional
  **Location**: Phase 2: Usage signature; Phase 3: argument-hint frontmatter
  The usage line `KEY STATE_NAME [--transition-id ID]` makes both look required. A parentheses-pipe form `KEY (STATE_NAME | --transition-id ID)` would make the mutual-exclusion visible at a glance.

- 🔵 **Usability**: Disambiguation prompt reply format is ambiguous in context
  **Location**: Phase 3: SKILL.md Step 6
  "Reply with the transition ID (e.g. `41`)" asks for a bare integer that could be misread as a yes/no confirmation response by an implementer who applies the confirmation gate logic too broadly. Adding "or `cancel` to abort" and distinguishing the format from the y/n idiom would eliminate ambiguity.

- 🔵 **Security**: `--transition-id` value format not validated — non-numeric IDs accepted silently
  **Location**: Phase 2: Algorithm argument parsing; Phase 3: SKILL.md Step 6
  The plan specifies no format check for `--transition-id` values. A non-numeric ID (including one with shell metacharacters) passes argument parsing and reaches the jq `--arg` call, producing a malformed Jira POST body and a confusing 400 error rather than a clear script-level validation failure. Accepting only `^[0-9]+$` and exiting 124 on mismatch would match existing flag-validation conventions.

- 🔵 **Usability**: Exit code 12 in transition SKILL.md gives misleading remediation for 403
  **Location**: Phase 3: SKILL.md Step 10 exit-code table
  The inherited exit-12 message ("Check credentials with `/init-jira`") is incorrect for the transitions endpoint, where 403 means the user lacks the `TRANSITION_ISSUES` project permission — a Jira admin issue, not a credential issue. The exit-12 row should be overridden with a specific message.

- 🔵 **Code Quality**: POST body construction technique for optional fields not specified
  **Location**: Phase 2: Algorithm step 6
  The algorithm does not specify whether optional fields (`fields.resolution`, `update.comment`) should be built via `jq -n --argjson` composition (the pattern used in `jira-update-flow.sh` lines 361–366) or via another approach. This leaves the implementer to choose, risking string-interpolation-based JSON assembly.

- 🔵 **Code Quality**: Case 5 (`E_ATTACH_FILE_MISSING`) tests only single missing file
  **Location**: Phase 4: Test script Cases table, Case 5
  The algorithm's "first missing file" fail-fast contract is only tested with a single missing file, not with a two-file input where only the second is missing. An implementer who silently skips missing files would pass all 10 test cases.

- 🔵 **Safety**: Disambiguation loop has no explicit iteration bound
  **Location**: Phase 3: SKILL.md Step 6
  No maximum number of disambiguation rounds is specified, unlike `comment-jira-issue/SKILL.md` which caps revision loops at 3. An explicit abort-after-3 clause would be consistent with the established pattern.

#### Suggestions

- 🔵 **Usability**: Humanise file size in attach response render
  **Location**: Phase 6: SKILL.md Step 7
  `52428800 bytes` is hard to parse at a glance. Format as KB if ≥ 1024, MB if ≥ 1 048 576.

---

### Strengths

- ✅ TDD sequencing strictly enforced with must-fail gate — prevents silent green-from-the-start test suites
- ✅ `consume: false` guard pattern applied to dry-run paths for both skills, providing automated regression protection
- ✅ Exit code namespace (120–139) pre-reserved in EXIT_CODES.md and filled without collisions; reserved ranges (127–129, 134–139) explicitly documented
- ✅ Both SKILL.md frontmatter specs match existing write-skill templates exactly (`disable-model-invocation: true`, `allowed-tools`, `description` phrasing)
- ✅ `expect_headers: {"X-Atlassian-Token": "no-check"}` used in attach fixtures — guards against the most common attachment API failure mode
- ✅ Auth deferred past `--describe` exit in both flow scripts — dry-run path never touches credentials
- ✅ Argument-validation exit codes (missing key, missing state/files, bad flags) explicitly verified without a mock server
- ✅ `transition-post-204-capture.json` uses body capture to assert actual POST payload content, not just exit code
- ✅ Scope is explicitly bounded: no delete-attachment, no bulk-transition, no download — prevents boundary erosion
- ✅ Disambiguation (exit 123) handled via user-driven `--transition-id` re-invocation, keeping the human in the decision path

---

### Recommended Changes

1. **Restructure `--describe` to perform the GET eagerly** (addresses: critical disambiguation finding + related null-transition_id, describe guard, success render findings)
   Make `--describe` call `GET /transitions`, resolve the state name, include the resolved `transition_id` (or surface exit 122/123) before emitting the describe JSON and exiting. If eager GET is not acceptable, document the deviation explicitly and restructure the SKILL.md to perform a pre-describe resolution call without `--describe`. Update `transition-describe-guard.json` to guard the POST (not GET) after this change.

2. **Clarify that `--describe` output reflects actual flag values** (addresses: null transition_id when `--transition-id` supplied, null resolution, false comment)
   Add a note to Algorithm step 2: describe output uses the supplied `--transition-id` value (not null), the supplied `--resolution` value (not null), and `true` for comment if `--comment`/`--comment-file` is present.

3. **Fix the `STATE_NAME` positional / `--transition-id` mutual exclusion** (addresses: usage signature correctness, exit 121 description, success render)
   Update usage to `KEY (STATE_NAME | --transition-id ID) [options]`. Update Algorithm steps 1–4 to treat STATE_NAME as absent when `--transition-id` is supplied. Add handling for the `--transition-id`-only success render in SKILL.md Step 9.

4. **Source `jira-body-input.sh` unconditionally at script load time** (addresses: Architecture/Code Quality/Standards triple-flag on conditional sourcing)
   Move the source call to the unconditional sourcing block alongside `jira-common.sh`, matching `jira-update-flow.sh` lines 52–55.

5. **Fix guard fixtures: add `capture_url: true`; transition guard should target POST** (addresses: Standards guard omission + Test Coverage/Correctness guard mismatch)
   Add `"capture_url": true` to both `transition-describe-guard.json` and `attach-describe-guard.json`. Change the transition guard to guard a `POST` (not `GET`) with `"consume": false`.

6. **Add test cases for exit 125 and 126** (addresses: Test Coverage major gap)
   Add Case 16: `--comment-file PATH-NOT-FOUND → exit 125, no network call`. Add Case 17: `--resolution '' → exit 126, no network call`.

7. **Specify path sanitisation for `file=@` arguments** (addresses: Security major)
   Add to Phase 5 Algorithm step 3: reject paths beginning with `-`; reject symlinks resolving to `/dev`, `/proc`, `/sys`; quote all paths when assembling multipart arguments.

8. **Specify `jq --arg` for STATE_NAME in the filter** (addresses: Security jq-injection risk)
   Add to Phase 2 Algorithm step 5: "Pass STATE_NAME as a jq `--arg` variable, never interpolated into the filter string."

9. **Replace `Read`-based file size with `wc -c`** (addresses: Usability/Safety file-size underspecification)
   Replace "if checkable via `Read`" in Phase 6 Step 4 with an explicit `wc -c` Bash call per file, or remove the size requirement and add only the irreversibility warning.

10. **Move attach file validation before `--describe` short-circuit** (addresses: Safety/Architecture file-validation ordering)
    Reorder Phase 5 Algorithm: validate file existence and readability before step 2 (`--describe` check), so `--describe` with invalid paths also exits 132.

11. **Add `transition-post-204-direct.json` to desired end state; add Fixture column to cases table** (addresses: Test Coverage fixture gap + fixture mapping)
    Add the single-POST scenario to the file list and to the Case 6 row. Add a Fixture column to the Phase 1 cases table specifying which scenario file each case uses.

12. **Update EXIT_CODES.md heading instruction; quote canonical confirmation gate text** (addresses: Standards minor findings)
    Instruct renaming the section heading to "Phase 4 namespace summary" (or "Namespace summary"). In both SKILL.md prose specs, quote the canonical phrasing: "Send this to Jira? Reply **y** to confirm, **n** to revise, anything else to abort."

---

*Review generated by /review-plan*

---

## Per-Lens Results

### Architecture

**Summary**: The plan follows the established Jira integration architectural pattern faithfully. The primary concern is a deliberate asymmetry in `--describe` mode for `jira-transition-flow.sh`: it emits a static description without making the GET call, meaning the SKILL.md must handle mid-execution branching for disambiguation. A secondary concern is conditional sourcing of `jira-body-input.sh`, which is a divergence from every other flow script's static sourcing chain.

**Strengths**:
- Exit code namespace treated as a first-class architectural contract
- TDD sequencing enforced as a hard gate via the must-fail success criterion
- Both new flow scripts correctly delegate all HTTP concerns to `jira-request.sh`
- Guard fixture pattern (`consume: false`) correctly ported from the comment/update patterns
- Scope explicitly bounded — no delete-attachment, no bulk-transition
- Test runner update is purely additive

**Findings**:

*Major* — `--describe` emits null `transition_id`, forcing mid-execution branching in the SKILL.md
Location: Phase 2 Algorithm step 2; Phase 3 SKILL.md Step 6
The describe mode not making the GET call breaks the describe-then-confirm contract. The SKILL.md must handle disambiguation mid-execution rather than pre-confirmation.

*Minor* — Conditional sourcing of `jira-body-input.sh` creates implicit coupling
Location: Phase 2 Sourcing section
Mid-script sourcing based on runtime flags makes the script's dependency graph non-discoverable at load time.

*Minor* — `--transition-id` bypass path: describe mode doesn't reflect the known ID
Location: Phase 1 Case 6; Phase 2 Algorithm step 4
`--describe` with `--transition-id 42` should emit `transition_id: "42"`, not `null`.

*Minor* — Attach flow validates files after `--describe` exits, creating asymmetric validation ordering
Location: Phase 4 Case 5; Phase 5 Algorithm step 3
`--describe` can succeed on non-existent file paths, making the preview inaccurate.

---

### Code Quality

**Summary**: Strong overall. Two major issues: the `--describe` output is semantically incomplete for the transition ID case, and the proposed conditional sourcing of `jira-body-input.sh` breaks the consistent eager-source convention used in every other flow script.

**Strengths**:
- Guard fixture pattern applied consistently
- Exit code design is clean and orthogonal
- Describe mode and describe guard are mutually consistent (both make no GET)
- Two-step fixture pattern mirrors the established multi-expectation pattern
- Error handling follows established propagation pattern

**Findings**:

*Major* — `--describe` output omits transition ID when state name is given, making preview semantically incomplete
Location: Phase 2 Algorithm step 2
The preview cannot tell the user which specific transition will be applied, undermining the confirmation gate's value.

*Major* — Conditional sourcing of `jira-body-input.sh` breaks the consistent eager-source pattern
Location: Phase 2 Sourcing section
Creates load-order dependency and makes static analysis harder; matches no existing flow script.

*Minor* — Case 6 (`--transition-id` bypass) lacks a body-capture assertion
Location: Phase 1 Cases table, Case 6
Without body capture, a bug that sends the wrong ID or empty body in the `--transition-id` path would pass.

*Minor* — POST body construction technique for optional fields not specified
Location: Phase 2 Algorithm step 6
Should explicitly reference the `jq -n --argjson` composition pattern from `jira-update-flow.sh`.

*Minor* — Case 5 (`E_ATTACH_FILE_MISSING`) underspecified for multi-file partial-missing
Location: Phase 4 Cases table, Case 5
Fail-fast contract not verified for two-file input where only the second file is missing.

---

### Test Coverage

**Summary**: Strong TDD discipline. Three major gaps: exit codes 125/126 have no test cases, the two-file attach test cannot verify file count, and the transition describe guard is wired to the wrong request type.

**Strengths**:
- TDD sequencing explicit and enforceable
- `consume: false` guard pattern applied consistently
- Argument-validation cases structured without mock server
- Body capture used for meaningful behavioural assertions
- `expect_headers` on attach fixtures guards the critical token header

**Findings**:

*Major* — Exit codes 125 and 126 defined but have no test cases
Location: Phase 1 Cases table vs. Phase 2 exit code table
`E_TRANSITION_NO_BODY` and `E_TRANSITION_BAD_RESOLUTION` are implemented but untested.

*Major* — Two-file attach test can only assert exit code; actual file count untestable
Location: Phase 4 Cases table, Case 2
`capture_body: true` + `grep -c 'filename='` would provide a viable client-side assertion.

*Major* — Transition `--describe` guard targets a GET that the algorithm never makes
Location: Phase 1 `transition-describe-guard.json`
The guard watches for a path that cannot occur, leaving the genuinely dangerous mistake (accidental POST) unguarded.

*Minor* — Case 6 lacks an explicit named fixture in the desired end state
Location: Phase 1 file list and Cases table
A `transition-post-204-direct.json` (single POST, no GET) is needed.

*Minor* — Cases table does not specify which fixture file each case uses
Location: Phase 1 Cases table
Fixture-to-case mapping is implicit; incorrect wiring produces hard-to-diagnose failures.

*Minor* — Transition HTTP error cases cover only 3 of 7 propagated exit codes
Location: Phase 1 Cases table, Cases 12–14
Acceptable risk given `test-jira-request.sh` coverage, but noted.

---

### Correctness

**Summary**: One critical issue: disambiguation cannot surface at describe/preview time, breaking the describe-then-confirm contract. Three related major issues trace back to the same root cause.

**Strengths**:
- Exit 121 correctly notes "and no `--transition-id`"
- `consume: false` guard prevents silent false positives
- Two-step GET-then-POST fixture correctly models the API sequence
- Exit codes propagated without re-wrapping
- File validation placed before credential resolution

**Findings**:

*Critical* — Disambiguation (exit 123) cannot surface at describe/preview time
Location: Phase 2 Algorithm step 2; Phase 3 SKILL.md Step 6
`--describe` exits before the GET that detects ambiguity. Exit 123 arrives post-confirmation at Step 8, with no defined recovery behaviour.

*Major* — `STATE_NAME` marked mandatory in usage but is optional when `--transition-id` given
Location: Phase 2 Algorithm steps 1–4
Argument parsing against the current usage signature will exit 121 on valid `--transition-id`-only invocations.

*Major* — `--describe` always emits `transition_id: null` even when `--transition-id` is supplied
Location: Phase 2 Algorithm step 2
The preview is factually inaccurate for `--transition-id`-supplied invocations; also applies to `resolution` and `comment` fields.

*Major* — Success render references `STATE_NAME` which may be absent
Location: Phase 3 SKILL.md Step 9
No state name is available in the 204 response body; the `--transition-id`-only path has no source for the state name to display.

*Minor* — Describe guard fixture assumes a GET is attempted but `--describe` exits before GET
Location: Phase 1 Test Case 4
`consume: false` on GET cannot detect the more dangerous mistake (accidental POST).

*Minor* — `attach-post-403.json` comment misleadingly implies missing-token cause
Location: Phase 4 Cases table, Case 8
The fixture comment should clarify this simulates a permissions error, not a missing `X-Atlassian-Token` (which `jira-request.sh` already injects).

---

### Standards

**Summary**: Faithfully follows codebase conventions in most respects. Two concrete deviations found: guard fixtures omit `capture_url: true`, and conditional `jira-body-input.sh` sourcing departs from the unconditional top-of-file pattern.

**Strengths**:
- `_JIRA_TRANSITION_SCRIPT_DIR` and `_JIRA_ATTACH_SCRIPT_DIR` follow the established naming convention
- `--describe` flag correctly chosen over `--print-payload` for write-only operations
- Fixture JSON structure matches the schema across all existing scenario files
- Exit code ranges fill the pre-reserved namespace without collisions
- SKILL.md frontmatter fields match the `update-jira-issue/SKILL.md` template exactly

**Findings**:

*Major* — Guard fixtures omit `capture_url: true` present in all existing guard fixtures
Location: Phase 1 `transition-describe-guard.json`; Phase 4 `attach-describe-guard.json`
Deviates from the pattern established by both existing guard fixtures.

*Major* — Conditional `jira-body-input.sh` sourcing deviates from unconditional top-of-file pattern
Location: Phase 2 Sourcing section
Every existing flow script that uses `jira-body-input.sh` sources it unconditionally at startup.

*Minor* — EXIT_CODES.md section heading not updated from "Phase 3 namespace summary"
Location: Phase 2 EXIT_CODES.md update
The heading will be stale after adding Phase 4 rows.

*Minor* — Confirmation gate prose spec omits exact canonical phrasing
Location: Phase 3 and Phase 6 skill-creator specs
The canonical text ("Send this to Jira? Reply **y** to confirm, **n** to revise, anything else to abort") should be quoted directly.

---

### Safety

**Summary**: Strong safety awareness. The core concern is that the `--describe` GET elision means the confirmation gate cannot warn about ambiguous or missing transitions before the user commits. File validation ordering in the attach flow creates a parallel problem: a non-existent file produces a clean preview followed by an immediate error.

**Strengths**:
- `--describe` required before any network write for both skills
- `consume: false` guard pattern makes dry-run safety testable and automated
- SKILL.md frontmatter explicitly marks both skills as irreversible
- Argument validation confirmed without network calls
- Disambiguation keeps human in the decision path
- File-not-found checked before credential resolution
- No delete-attachment skill included

**Findings**:

*Major* — `--describe` skips GET so preview cannot warn about ambiguous or missing transitions
Location: Phase 2 Algorithm step 2
User confirms a preview that omits the most likely failure mode (ambiguous state name).

*Major* — File existence validation occurs after `--describe` exits in attach flow
Location: Phase 5 Algorithm step 3
`--describe` with non-existent paths returns a clean preview; the error only surfaces post-confirmation.

*Minor* — File size in attach preview depends on a `Read` tool call that may be silently skipped
Location: Phase 6 SKILL.md Step 4
"If checkable via Read" leaves recovery undefined; a `wc -c` Bash call is more reliable.

*Minor* — Disambiguation loop has no explicit iteration bound
Location: Phase 3 SKILL.md Step 6
The established write-skill pattern (comment-jira-issue) caps revision loops at 3.

---

### Security

**Summary**: Two major concerns: user-supplied file paths reach `file=@<path>` curl arguments without sanitisation; and the jq filter for state-name matching may be string-interpolated rather than `--arg`-parameterised. Credential handling and the dry-run guard are well-designed.

**Strengths**:
- Credentials sourced only after `--describe` exits
- `consume: false` guard makes zero-network-call property testable
- `jira-request.sh` path validation guards the issue key at the request layer
- Auth deferred past argument validation errors
- File existence and readability validated before credential resolution

**Findings**:

*Major* — No path sanitisation before `file=@<path>` multipart argument construction
Location: Phase 5 Algorithm step 5
Paths beginning with `-` (stdin), symlinks to `/dev`/`/proc`, and FIFOs pass readability checks but cause curl to read unexpected sources.

*Major* — `STATE_NAME` in jq filter — injection risk if string-interpolated
Location: Phase 2 Algorithm step 5
Plan does not mandate `jq --arg`; a string-interpolated filter is vulnerable to jq-logic injection via crafted state names.

*Minor* — Resolution name value not validated beyond empty-string check
Location: Phase 2 Algorithm argument parsing
A basic format check would prevent confusingly Jira-originating 400 errors reflecting back user input.

*Minor* — `--transition-id` value not format-validated; non-numeric IDs accepted
Location: Phase 2 Algorithm argument parsing; Phase 3 SKILL.md Step 6
Accepting only `^[0-9]+$` and exiting 124 would match existing flag-validation conventions and prevent confusing 400 errors.

---

### Usability

**Summary**: Solid foundation, but three major UX friction points need resolution before implementation: the disambiguation flow lacks a malformed-stdout fallback, the describe preview always shows `null` for `transition_id`, and the file-size derivation approach in the attach preview is fragile.

**Strengths**:
- Argument-hint frontmatter strings are compact and complete
- Exit code naming convention (`E_VERB_NOUN`) is consistent and learnable
- Disambiguation placed before confirmation gate — correct sequencing
- Guard fixture pattern prevents silent dry-run regressions
- Mutual-exclusion rule surfaced in both SKILL.md spec and exit code table
- Irreversibility warning placed in preview step, not just confirmation

**Findings**:

*Major* — Exit-123 disambiguation flow has no error path for malformed stdout
Location: Phase 3 SKILL.md Step 6
No recovery behaviour specified for empty or non-JSON stdout; different implementers will behave inconsistently.

*Major* — `--describe` preview shows `transition_id: null`, misleading in rendered preview
Location: Phase 2 Algorithm step 2
SKILL.md Step 5's conditional render ("via transition `<ID>` if known") will always fall back to "by state name" because `transition_id` is always `null` in describe output when a state name is given.

*Major* — File size derivation via `Read` tool in attach preview is underspecified and fragile
Location: Phase 6 SKILL.md Step 4
`Read` is for file contents, not metadata; `wc -c` is the correct tool. "If checkable" leaves recovery undefined.

*Minor* — STATE_NAME has no syntactic marker distinguishing it from a mandatory positional
Location: Phase 2 usage signature; Phase 3 argument-hint
`KEY (STATE_NAME | --transition-id ID)` notation would make mutual-exclusion visible.

*Minor* — Disambiguation prompt reply format is ambiguous in context
Location: Phase 3 SKILL.md Step 6
A bare integer reply could be confused with a confirmation response; adding "or `cancel` to abort" disambiguates.

*Minor* — Exit code 12 in transition SKILL.md gives misleading remediation for 403
Location: Phase 3 SKILL.md Step 10 exit-code table
"Check credentials with `/init-jira`" is wrong for the transitions endpoint; 403 there means missing `TRANSITION_ISSUES` project permission.

*Minor* — `--transition-id` only invocations: usage line shows STATE_NAME as required
Location: Phase 2 usage line
Inconsistent with exit 121 description which notes the mutual-exclusion.

*Suggestion* — Humanise file size in attach response render
Location: Phase 6 SKILL.md Step 7
Format as KB/MB rather than raw bytes for readability.

---

## Re-Review (Pass 2) — 2026-05-03

**Verdict:** REVISE

The plan edits resolved every critical and major finding from pass 1 — the eager GET in `--describe`, the describe output reflecting actual flag values, the STATE_NAME/`--transition-id` mutual exclusion, unconditional sourcing, guard fixtures with `capture_url: true`, and all test coverage gaps. However, the change that made `--describe` perform the GET eagerly introduced a new cluster of related problems that require a further revision pass.

### Previously Identified Issues

- ✅ **Correctness**: Disambiguation cannot surface at preview time — **Resolved**. `--describe` now performs the GET eagerly for STATE_NAME; exit 122/123 surface pre-confirmation.
- ✅ **Architecture + Code Quality + Standards**: Conditional `jira-body-input.sh` sourcing — **Resolved**. Unconditional load-time sourcing block specified.
- ✅ **Correctness + Code Quality + Usability**: `--describe` always emits `transition_id: null` — **Resolved**. Actual flag values reflected in all cases.
- ✅ **Correctness**: Success render references absent `STATE_NAME` — **Resolved**. Null-state path renders "transition ID <ID> applied."
- ✅ **Correctness + Usability**: `STATE_NAME` mandatory in usage signature — **Resolved**. `KEY (STATE_NAME | --transition-id ID)` notation used.
- ✅ **Architecture + Safety**: File validation after `--describe` in attach — **Resolved**. Validation now precedes `--describe` short-circuit.
- ✅ **Standards**: Guard fixtures omit `capture_url: true` — **Resolved**. Both guard fixtures now carry `consume: false` + `capture_url: true`.
- ✅ **Standards**: EXIT_CODES.md heading not renamed — **Resolved**. Explicit rename instruction added.
- ✅ **Standards**: Canonical confirmation phrase omitted — **Resolved**. Verbatim quote in both SKILL.md specs.
- ✅ **Test Coverage**: Exit codes 125/126 no test cases — **Resolved**. Cases 16 and 17 added.
- ✅ **Test Coverage**: Two-file attach count untestable — **Resolved**. `capture_body: true` + `grep -c 'filename='` assertion specified.
- ✅ **Test Coverage**: Case 6 lacks explicit fixture — **Resolved**. `transition-post-204-direct.json` added with `capture_body: true`.
- ✅ **Test Coverage**: Cases table lacks Fixture column — **Resolved**. Both tables now carry Fixture column.
- ✅ **Safety**: No disambiguation iteration bound — **Resolved**. Explicit 3-attempt cap with abort message.
- ✅ **Safety + Usability**: File size via `Read` tool — **Resolved**. `wc -c` via Bash specified; humanised KB/MB rendering in both preview and response.
- ✅ **Security**: No path sanitisation for `file=@` — **Resolved**. Three layered checks now specified (dash-prefix, symlink-to-device, existence/readability).
- ✅ **Security**: `STATE_NAME` jq injection risk — **Resolved**. `jq --arg` mandated verbatim in algorithm.
- ✅ **Usability**: Disambiguation no malformed-stdout fallback — **Resolved**. Explicit fallback clause added.
- ✅ **Usability**: Exit code 12 misleading remediation — **Resolved**. Overridden with TRANSITION_ISSUES permission message.
- 🟡 **Security (minor)**: Resolution name only validated for empty string — **Still present**.
- 🟡 **Security (minor)**: `--transition-id` value not format-validated — **Still present**.

### New Issues Introduced

The eager-GET change in `--describe` introduced a guard fixture that is now mismatched with the algorithm it is meant to test. Three lenses elevated this to **major**.

- 🔴 **Test Coverage + Code Quality**: `transition-describe-guard.json` has no GET expectation — Case 4 will fail when invoked with STATE_NAME
  The plan's fixture prose now says "The GET is expected (describe resolves the state name eagerly); only the POST must never happen" — but the JSON body contains only a POST expectation. The mock server responds HTTP 500 to unexpected requests and records the error, causing the mock to exit 1 at shutdown regardless of whether the POST guard fired. If Case 4 uses STATE_NAME, it fails unconditionally for the wrong reason. If Case 4 uses `--transition-id` to avoid the GET, the guard covers only that bypass path and leaves the STATE_NAME eager-GET describe surface unguarded. Fix: add a consuming GET expectation as the first entry in the fixture, followed by the POST guard.

- 🔴 **Correctness + Standards**: `jira_resolve_credentials` called before `--describe`, blocking the `--transition-id` offline-preview path
  Moving credential resolution to algorithm step 2 (before the `--describe` check) means `--describe --transition-id 21` — which makes no network call — still fails if credentials are not configured. This contradicts the purpose of the bypass path's offline preview and is inconsistent with `jira-attach-flow.sh`, which correctly defers credential resolution until after `--describe` exits. Fix: move `jira_resolve_credentials` so it is called only when a GET will actually be made (inside the STATE_NAME describe branch, and before the live GET/POST steps for non-describe execution), not unconditionally before all `--describe` paths.

- 🔴 **Standards + Code Quality**: `jira-attach-flow.sh` sources `jira-auth.sh` mid-algorithm while `jira-transition-flow.sh` sources it at load time
  The two companion scripts, introduced in the same plan, follow different sourcing conventions. The plan specifies a clean load-time sourcing block for transition but defers `jira-auth.sh` to algorithm step 4 for attach. The fix above (defer credential resolution in transition too) would align both scripts: `jira-auth.sh` sourced at load time in both; `jira_resolve_credentials` called only when needed.

- 🔵 **Architecture (suggestion)**: Test scaffold comment "Cases 1-15" is stale — cases table now defines 17 cases.

- 🔵 **Usability (minor)**: Case 3 assertion does not verify `state` field equals the supplied STATE_NAME — an implementation returning `state: null` after a successful eager GET would pass all tests but render the preview incorrectly.

- 🔵 **Usability (minor)**: Disambiguation loop-back prose ("omit STATE_NAME") does not explain why — a skill author following the prose may inadvertently re-include STATE_NAME on the re-invocation.

### Assessment

The plan is in significantly better shape than after pass 1. The critical finding and all major findings from the first review are fully resolved. Three new major issues were introduced by the eager-GET design change, all of which trace to the same root: the credential-resolution ordering did not fully account for the `--transition-id` offline-describe path, and the guard fixture was not updated to include the GET that the new describe path now issues. These are targeted fixes, not structural redesigns.

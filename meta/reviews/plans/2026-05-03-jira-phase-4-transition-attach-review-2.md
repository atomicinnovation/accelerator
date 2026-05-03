---
date: "2026-05-04T00:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-03-jira-phase-4-transition-attach.md"
review_number: 2
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, standards, safety, security, usability]
review_pass: 3
status: complete
---

## Plan Review: Jira Phase 4 — Workflow and Attachments (Pass 3)

**Verdict:** REVISE

The three major findings from pass 2 are all correctly addressed: `transition-describe-guard.json` now has a consuming GET expectation before the POST guard; the Phase 5 attach flow has an explicit load-time sourcing block; and the `--describe --transition-id` offline-preview path correctly short-circuits before any credential resolution. The plan is significantly tighter than it was at the start of this review cycle. However, eight new major findings emerged across the lenses, with two clear cross-cutting themes: (1) the ambiguous-state output is NDJSON but the SKILL.md expects a parseable JSON array, which would silently break the disambiguation flow in production; and (2) `jira-auth.sh` is sourced at load time in both new flow scripts, contradicting the convention uniformly followed by all five existing flow scripts that delegate credential resolution to `jira-request.sh` entirely. Together these, plus gaps in argument mutual-exclusion enforcement, path guards for `--comment-file`, and missing automated coverage for the offline describe path, require another revision before implementation.

---

### Cross-Cutting Themes

- **Credential-handling convention break** (flagged by: Standards, Architecture, Code Quality) — All five existing flow scripts (`jira-comment-flow.sh`, `jira-create-flow.sh`, `jira-update-flow.sh`, `jira-search-flow.sh`, `jira-show-flow.sh`) neither source `jira-auth.sh` nor call `jira_resolve_credentials` — they delegate credential resolution entirely to `jira-request.sh` as a subprocess. Both new scripts are specified to source `jira-auth.sh` at load time and call `jira_resolve_credentials` explicitly in the algorithm, which is redundant (since `jira-request.sh` resolves credentials internally) and contradicts the established convention. The Standards lens verified this against the actual codebase.

- **Mutual exclusion enforcement absent** (flagged by: Code Quality, Correctness, Usability) — `STATE_NAME` and `--transition-id` are declared mutually exclusive in the spec but the algorithm has no exit code, no test case, and no detection logic for when both are supplied simultaneously. The current algorithm silently prefers `--transition-id`, discarding the positional `STATE_NAME` with no error.

- **Guard fixture captured-URLs assertion gap** (flagged by: Test Coverage, Safety) — Both Case 4 (transition) and Case 4 (attach) specify `consume: false + capture_url: true` on their guard fixtures, but neither case specifies an assertion that reads the captured-URLs output file and verifies it is empty. The `consume: false` mechanism prevents the mock from pop-routing the guarded expectation, but only an explicit `captured_urls == []` check proves no call was made. Without it, an accidental POST during `--describe` would be silently missed.

---

### Findings

#### Major

- 🟡 **Correctness**: Ambiguous-match output is NDJSON, but SKILL.md expects a JSON array
  **Location**: Phase 2: Algorithm step 5 (2+ matches branch); Phase 3: SKILL.md Step 4 (exit 123 handler)
  The jq filter `.transitions[] | select(.to.name | ascii_downcase == ($s | ascii_downcase))` outputs multiple newline-delimited JSON objects (NDJSON) when 2+ transitions match. The SKILL.md spec says "parse the JSON array from stdout," and Case 5 says "stdout contains both IDs." The algorithm never specifies wrapping the jq results in `[]` before writing to stdout. In production the SKILL.md will attempt to parse NDJSON as a JSON array, fail, and trigger the "not parseable" fallback — degrading the disambiguation UX even when the data is perfectly valid.

- 🟡 **Code Quality + Correctness + Usability**: Mutual exclusion of `STATE_NAME` and `--transition-id` is unenforceable as specified
  **Location**: Phase 2: Algorithm step 1; Phase 1: Test cases table
  The spec declares the two forms mutually exclusive but the algorithm provides no exit code or error for simultaneous use, and there is no test case covering it. Steps 2 and 4 both branch on "if `--transition-id` given" — meaning `STATE_NAME` is silently discarded when both are supplied. The SKILL.md constraint ("both may not be supplied together") is therefore not backed by the flow script.

- 🟡 **Security**: `--comment-file` path in transition flow lacks the dash-prefix and symlink-to-device guards specified for the attach flow
  **Location**: Phase 2: Algorithm step 1 (`--comment-file` validation)
  The attach flow (Phase 5 step 2) specifies three-layer path sanitisation: dash-prefix rejection, symlink-to-device guard, and existence/readability check. The transition flow specifies only existence check via `jira_resolve_credentials` delegation to `jira-body-input.sh`, which implements only `[ -f "$body_file" ]`. A path of `-` passes the existence check and reads from stdin; `/dev/stdin` or `/dev/urandom` passes both existence and readability on most systems. The asymmetry is not mentioned in the plan.

- 🟡 **Standards**: Both new flow scripts source `jira-auth.sh` at load time — breaks the convention established by all five existing flow scripts
  **Location**: Phase 2: Sourcing block; Phase 5: Sourcing block
  Every existing flow script delegates credential resolution entirely to `jira-request.sh` (verified in `jira-comment-flow.sh`, `jira-create-flow.sh`, `jira-update-flow.sh`, `jira-search-flow.sh`, `jira-show-flow.sh`). None source `jira-auth.sh` at load time or call `jira_resolve_credentials` explicitly. The plan's rationale ("needed for the eager GET") does not hold: the eager GET is made via `bash jira-request.sh`, which resolves credentials internally within its own subprocess — the parent shell does not need `$JIRA_SITE`/`$JIRA_EMAIL`/`$JIRA_TOKEN` set. The only script that sources `jira-auth.sh` at load time is `jira-init-flow.sh`, which uses credential variables in the parent shell to write `site.json` — a use case that does not apply here. The explicit `jira_resolve_credentials` calls at Phase 2 algorithm step 2 (inside describe branch) and step 3 (before live GET/POST) are therefore redundant and violate the established convention.

- 🟡 **Code Quality**: `jira_require_dependencies` absent from both new flow script algorithms
  **Location**: Phase 2: Algorithm (transition); Phase 5: Algorithm (attach)
  Every existing flow script calls `jira_require_dependencies` as the first statement of its main function (verified in `jira-update-flow.sh` line 122, `jira-create-flow.sh` line 117, `jira-comment-flow.sh` line 452). The plan specifies no such call for either new script, meaning a missing `jq` or `curl` binary surfaces as a confusing mid-execution failure rather than a clear `E_MISSING_DEP` error at startup. Both scripts already source `jira-common.sh` (which defines the function), so no additional sourcing is needed.

- 🟡 **Test Coverage**: Case 4 guard in both transition and attach tests lacks an explicit captured-URLs assertion
  **Location**: Phase 1: Test Case 4 (`transition-describe-guard.json`); Phase 4: Test Case 4 (`attach-describe-guard.json`)
  Both guard cases are described as confirming "no POST was made" via `consume: false + capture_url: true`. But the test must explicitly read `server.captured_urls` after `--describe` exits and assert it equals `[]`. Without this assertion, an accidental POST during `--describe` would be recorded in the captured-URLs file but never read by the test, causing a silent false-PASS. The pattern in `test-jira-comment.sh` Case 25a explicitly checks `assert_eq "[]" "$(jq -c '.' "$URLS_25a")"` — the new cases must do the same.

- 🟡 **Correctness**: No automated test case for the `--describe --transition-id` path (no credentials, no GET, `state: null`)
  **Location**: Phase 1: Test cases table; Phase 2: Manual Verification checklist
  The `--describe --transition-id` path has unique behavioural properties: no credentials required, no network call, and `state: null` in output. These properties appear only in the manual verification checklist, not as a numbered test case. A bug where the implementation accidentally calls `jira_resolve_credentials` on this path would not be caught by the automated test suite — it would only surface in production when credentials are absent.

- 🟡 **Security**: `--transition-id` value not validated as numeric before use in POST body
  **Location**: Phase 2: Algorithm step 1 (argument parsing); Phase 2: Algorithm step 4 (bypass path)
  The algorithm accepts any string as the `--transition-id` value and passes it directly into the POST body. Without a `^[0-9]+$` guard at parse time, a non-numeric value (including metacharacter-carrying strings) reaches the `jq -n --argjson` composition. The `jq --argjson` mandate contains the JSON structural risk, but the user receives a confusing Jira 400 error rather than a clear local validation failure. Adding numeric-only validation at step 1 and exiting 124 on mismatch matches the existing flag-validation convention (`--page-size` in `jira-comment-flow.sh`).

- 🟡 **Usability + Test Coverage**: Case 3 assertion does not verify `state` is non-null, leaving the eager-GET path untested
  **Location**: Phase 1: Test case table — Case 3 (Key assertion column)
  Case 3's key assertion reads "exit 0; stdout has resolved `transition_id`" but does not assert that `state` is non-null. An implementation that uses the `--transition-id` describe path (no GET, `state: null`) would satisfy this assertion while bypassing the eager GET that distinguishes the STATE_NAME describe path. The assertion must also verify `state != null` — e.g. `jq -e '.state != null and .transition_id != null'` — and ideally pin the resolved ID to `"21"` from the fixture.

- 🟡 **Usability**: Disambiguation loop-back instruction omits the reason `STATE_NAME` must be dropped on re-invocation
  **Location**: Phase 3: SKILL.md prose spec — Step 4, exit 123 loop-back
  The spec says "re-invoke with `--describe --transition-id <chosen_id>` (omit STATE_NAME)" but gives no rationale. A skill author could plausibly add `STATE_NAME` back as helpful context without realising it would trigger undefined behaviour — STATE_NAME and `--transition-id` are declared mutually exclusive, and the algorithm has no error for simultaneous use (see mutual-exclusion finding above). A one-sentence explanation — "omit STATE_NAME because the flow script treats STATE_NAME and `--transition-id` as mutually exclusive positional/flag slots" — would eliminate this ambiguity.

#### Minor

- 🔵 **Correctness + Standards**: Phase 2 overview says "15 test cases" but actual count is 17
  **Location**: Phase 2: Overview paragraph (first sentence)
  "Write `jira-transition-flow.sh` until all 15 test cases pass" is stale — Cases 16 and 17 were added in pass 1 revisions. The success criteria checkbox already correctly says "all 17 cases PASS." Change the overview prose to match.

- 🔵 **Architecture**: `--describe` STATE_NAME dry-run boundary not documented in SKILL.md Step 3
  **Location**: Phase 3: SKILL.md prose spec — Step 3 (`--describe` invocation)
  The SKILL.md Step 3 presents `--describe` as a uniform dry-run step. When STATE_NAME is given, it makes a live GET to Jira, meaning auth errors (exit 11/22) or network errors (exit 21) can surface at the preview stage — not just at the POST stage. Step 4's error handling does not distinguish preview-read failures from confirmation failures. A parenthetical ("When STATE_NAME is given, `--describe` makes a read-only GET; auth and network errors may occur here") would prevent implementers from routing all non-zero exits identically.

- 🔵 **Architecture**: Exit 132 (`E_ATTACH_FILE_MISSING`) not listed in Phase 6 SKILL.md's Step 3 failure handling
  **Location**: Phase 6: SKILL.md prose spec — Step 3 (preview failure handling)
  Phase 5 correctly specifies that file validation runs before `--describe`, meaning `--describe` with a non-existent path exits 132. Phase 6 Step 3 says only "non-zero exit → stop, no API call." Without exit 132 specifically handled, the skill will render a generic "preview failed" message when the real cause is a missing file. Specifying exit 132 → "File not found: `<path>`" in Step 3 provides actionable feedback.

- 🔵 **Architecture**: `jira-md-to-adf.sh` invoked as subprocess but plan does not clarify this
  **Location**: Phase 2: Algorithm step 6
  The sourcing block lists `jira-common.sh`, `jira-body-input.sh`, and `jira-auth.sh`. Step 6 says "ADF converted via `jira-md-to-adf.sh`." This script is always invoked as a subprocess (`bash …/jira-md-to-adf.sh`) in the existing flow scripts — it has a top-level `main` call and `set -euo pipefail`. If an implementer adds it to the sourcing block, its standalone `main` invocation at source time will produce unintended behaviour. A one-line note in step 6 ("invoked as a subprocess, not sourced") would prevent this.

- 🔵 **Test Coverage**: Case 2 (case-insensitive match) shares a fixture with Case 1 but asserts only exit 0
  **Location**: Phase 1: Test cases table — Case 2
  Case 2 invokes with lowercase `"in progress"` against a fixture where the transition is stored as `"In Progress"` (ID `"21"`). The only specified assertion is exit 0. Using `transition-post-204-capture.json` (which has `capture_body: true`) and asserting the POST body contains `"id":"21"` would make the case-insensitive path mutation-sensitive.

- 🔵 **Test Coverage**: Cases 12 and 13 use "inline mock response" — a pattern not present in the codebase
  **Location**: Phase 1: Test cases table — Cases 12/13
  All existing test suites use named scenario files. The parenthetical "(inline mock response)" is not explained. Clarify whether this means a temp scenario file written by the test (via mktemp) or reuse of existing `error-401.json`/`error-404.json` fixtures from `test-jira-request.sh`. The existing fixtures are the simpler path and should be referenced explicitly.

- 🔵 **Usability**: Success message after disambiguation flow omits the state name the user originally requested
  **Location**: Phase 3: SKILL.md prose spec — Step 8 (success rendering after disambiguation)
  After disambiguation, the re-invoke uses `--describe --transition-id <chosen_id>` (no STATE_NAME), so describe output has `state: null`. Step 8 therefore renders "✓ **<KEY>** transition ID <ID> applied" — not the state name the user originally requested. Since the STATE_NAME is known from the user's original invocation, the spec could instruct the skill to carry it through the disambiguation turns and render "✓ **<KEY>** transitioned to `"In Review"` (via transition ID <ID>)."

- 🔵 **Security**: `--resolution NAME` not explicitly specified to use `jq --arg` in POST body composition
  **Location**: Phase 2: Algorithm step 6 (POST body construction)
  The `STATE_NAME` jq-injection guard is explicitly mandated at step 5. `--resolution NAME` is also a user-supplied string that enters the POST body at step 6, but step 6 only references the `jq -n --argjson` composition pattern without explicitly calling out that resolution name must be passed via `jq --arg r "$RESOLUTION"` — never string-interpolated. Mirror the step 5 documentation pattern for this value.

- 🔵 **Security**: Disambiguation chosen ID should be validated as numeric before shell re-invocation
  **Location**: Phase 3: SKILL.md prose spec — Step 4 (exit 123, disambiguation prompt)
  The SKILL.md instructs re-invoking with `--describe --transition-id <chosen_id>` where `<chosen_id>` comes from user input. If the user replies with a non-numeric string (or a string containing shell metacharacters), the Bash invocation interpolates it unguarded. The spec should state: validate the chosen reply as a string of digits only before constructing the Bash invocation; reject (count against the 3-attempt limit) if not purely numeric.

- 🔵 **Safety**: Large file uploads proceed without a size pre-check; curl timeout is the only backstop
  **Location**: Phase 5: Performance Considerations; Phase 5: Algorithm step 2
  The plan explicitly omits size validation ("Jira's server-side limit is enforced via the HTTP response"). However, `jira-request.sh` uses `--max-time 30` — a very large file will time out with exit 21 (`E_REQ_CONNECT`) rather than a clear "file too large" message. A lightweight pre-check (`wc -c`) warning when a file exceeds Jira Cloud's 10 MB default limit would give actionable feedback before the upload attempt.

- 🔵 **Safety**: Whitespace-only `--resolution` value passes exit 126 guard and reaches the POST body
  **Location**: Phase 2: Algorithm step 1 (exit 126 guard)
  Case 17 verifies exit 126 on `--resolution ''`. A whitespace-only value (e.g. `--resolution '   '`) passes the empty-string check and is included in the POST body, triggering a 400 from Jira (exit 34) with a confusing server-side error. Extending the guard to trim leading/trailing whitespace before the empty check would surface this as a clear local validation failure.

- 🔵 **Code Quality**: Test case count "11 cases" in Phase 5 success criteria is ambiguous given 5a/5b split
  **Location**: Phase 5: Success Criteria — Automated Verification
  "All 11 cases PASS (Cases 1–4, 5a, 5b, 6–10)" — 5a and 5b are within the same numbered entry but counted separately in the "11" total. Either renumber 5b as Case 11 (making the "11 cases" count match a contiguous sequence) or add a parenthetical clarifying that 5a and 5b are separate assert blocks within Case 5's test block.

---

### Strengths

- ✅ `transition-describe-guard.json` now correctly sequences a consuming GET (serving the eager STATE_NAME resolve) before the non-consuming POST guard — the key correctness fix from pass 2 is mechanically sound
- ✅ `--describe --transition-id` correctly short-circuits before any credential resolution or network call — the offline-preview invariant holds as specified
- ✅ `jira-attach-flow.sh` sourcing block is now explicit (`jira-common.sh` + `jira-auth.sh` at load time) with a note explaining credential deferral past `--describe`
- ✅ File-path validation in attach runs before `--describe` short-circuit — describe mode is also protected by path sanitisation, making the preview accurate
- ✅ `jq --arg` mandate for `STATE_NAME` is explicit and correctly positioned in step 5, preventing injection of crafted state names
- ✅ Three-layer path sanitisation for attach (dash-prefix, symlink-to-device, existence/readability) is well-specified and tested in Cases 5a/5b
- ✅ Body-capture assertions (Cases 8, 9 for transition; Case 2 for attach) verify actual POST payload content — not just exit codes
- ✅ Disambiguation retry limit (3 attempts then abort with a clear message) matches the established write-skill pattern
- ✅ Exit-code namespace is filled without collisions (120–129 transition, 130–139 attach), with reserved ranges documented
- ✅ SKILL.md frontmatter matches existing write-skill templates exactly (`disable-model-invocation: true`, `allowed-tools`, phrasing)
- ✅ `jira-body-input.sh` sourced unconditionally at load time in transition flow — matching the pattern in `jira-update-flow.sh`

---

### Recommended Changes

1. **Fix ambiguous-match output to produce a JSON array** (addresses: Correctness — NDJSON vs JSON array)
   Amend algorithm step 5 to pipe the select results through `jq -s '.'` or rewrite as `.transitions | map(select(.to.name | ascii_downcase == ($s | ascii_downcase)))`. Update Case 5 assertion to verify `jq 'type'` returns `"array"`. Apply the same fix to the `--describe STATE_NAME` 2+ match branch in step 2.

2. **Enforce mutual exclusion — add exit code and test case for simultaneous STATE_NAME + `--transition-id`** (addresses: Mutual exclusion finding)
   Add an explicit check after argument parsing: if both STATE_NAME and `--transition-id` are set, exit 124 with a clear error. Add a test case asserting this combination is rejected.

3. **Add dash-prefix and symlink-to-device guards for `--comment-file` in transition flow** (addresses: Security — --comment-file path guards)
   Add the same three-layer validation (dash-prefix, symlink-to-/dev/proc/sys, existence/readability) at algorithm step 1 for `--comment-file PATH`, before delegating to `jira-body-input.sh`. Alternatively, add the guards inside `jira-body-input.sh` so all callers benefit.

4. **Remove `jira-auth.sh` sourcing and explicit `jira_resolve_credentials` calls from both flow scripts** (addresses: Standards — convention violation)
   Follow the convention of all five existing flow scripts: source only `jira-common.sh` (and `jira-body-input.sh` for transition) at load time. Remove `source … jira-auth.sh` and the explicit `jira_resolve_credentials` calls from the algorithm. The eager GET and live GET/POST both use `jira-request.sh` as a subprocess, which resolves credentials internally. Simplify the algorithm by removing steps 2.b's explicit credential call and step 3 entirely.

5. **Add `jira_require_dependencies` as the first call in both flow script main functions** (addresses: Code Quality — missing dependency check)
   Following `jira-update-flow.sh` line 122, add `jira_require_dependencies` before argument parsing in both `jira-transition-flow.sh` and `jira-attach-flow.sh`. Both scripts already source `jira-common.sh`.

6. **Add captured-URLs assertion to Case 4 in both transition and attach test scripts** (addresses: Test Coverage — guard has no captured-URLs assertion)
   For each Case 4, after invoking `--describe` with the guard fixture loaded, assert `jq -c '.' "$CAPTURED_URLS_FILE" == "[]"` — mirroring the pattern in `test-jira-comment.sh` Case 25a.

7. **Add an automated test case for `--describe --transition-id` (no credentials, no GET)** (addresses: Correctness — offline path untested)
   Add a numbered test case that invokes `--describe ENG-1 --transition-id 21` with no `ACCELERATOR_JIRA_TOKEN` set and no mock server running, asserting: exit 0, `state` is `null`, `transition_id` is `"21"`, and the script does not attempt a network connection (verified by the absence of a running mock — the script must not hang or fail trying to connect).

8. **Add numeric-only validation for `--transition-id` value** (addresses: Security — numeric validation)
   Add `[[ "$TRANSITION_ID" =~ ^[0-9]+$ ]]` at argument parsing step 1; exit 124 on mismatch with a clear error. Add or extend an existing test case to assert non-numeric `--transition-id` exits 124.

9. **Strengthen Case 3 assertion to verify `state != null` and pin the resolved ID** (addresses: Usability + Test Coverage — Case 3 assertion)
   Change the key assertion to: `exit 0; stdout has state != null AND transition_id == "21"`. Use `jq -e '.state != null and .transition_id == "21"'` as the assertion expression.

10. **Add rationale to disambiguation loop-back instruction** (addresses: Usability — loop-back explanation)
    In Phase 3 Step 4, after "(omit STATE_NAME)" add: "(STATE_NAME and `--transition-id` are mutually exclusive positional/flag slots — passing both produces an argument error or undefined behaviour in the flow script)."

11. **Update Phase 2 overview case count from 15 to 17** (addresses: Correctness/Standards — stale count)
    Change "all 15 test cases pass" to "all 17 test cases pass."

---

*Review generated by /review-plan*

---

## Per-Lens Results

### Architecture

**Summary**: Structurally sound; follows established layered patterns. Two architectural gaps: the `--describe` STATE_NAME path makes a live network call that is not labelled as such in the SKILL.md, creating a dry-run boundary mismatch; and both new flow scripts add `jira-auth.sh` to their sourcing convention, deviating from every existing flow script.

**Strengths**:
- Guard fixture two-expectation design (consuming GET + POST guard) is mechanically correct for the eager-GET describe path
- `--transition-id` offline describe path correctly short-circuits before credential resolution or network call
- File-path safety hardening in attach correctly placed before `--describe` short-circuit
- `jira-body-input.sh` reused for `--comment` body in transition flow
- Exit-code namespace managed purely additively

**Findings**:

*Major* — `--describe` STATE_NAME path makes a live network call, violating the dry-run boundary the SKILL.md implies
Location: Phase 2 Algorithm step 2 (describe STATE_NAME branch); Phase 3 SKILL.md Step 3
The SKILL.md presents `--describe` as a uniform dry-run step. When STATE_NAME is given, a real GET to Jira is made. Auth errors (exit 11/22) or network errors (exit 21) can surface during the preview stage, but Step 4's error handling does not distinguish them from validation failures.

*Minor* — `jira-md-to-adf.sh` invoked as subprocess but sourcing block gives no indication of this
Location: Phase 2 sourcing block; Algorithm step 6
Adding `jira-md-to-adf.sh` to the sourcing block (a plausible implementer mistake) would cause its standalone `main()` call to fire at source time.

*Minor* — Exit 132 not included in Phase 6 SKILL.md failure handling
Location: Phase 6 SKILL.md prose spec Step 3
Since file validation precedes `--describe` in attach, `--describe` can exit 132 on missing files. Step 3 only says "non-zero exit → stop" — without mapping exit 132 to "File not found: `<path>`", the skill renders a generic failure.

*Suggestion* — "Inline mock response" pattern for Cases 12/13 not documented
Location: Phase 1 Cases table, Cases 12/13
The codebase uses named fixture files exclusively; the plan should clarify whether "inline mock response" means a temp scenario file or reuse of existing error fixtures.

---

### Code Quality

**Summary**: Strong structure; established patterns followed closely. Two gaps: `jira_require_dependencies` is absent from both new script algorithms; and the mutual exclusion between `STATE_NAME` and `--transition-id` has no specified error path or test.

**Strengths**:
- Credential-deferral now explicit and correctly structured
- `jira-attach-flow.sh` sourcing block explicit with rationale
- `jq -n --argjson` composition pattern correctly referenced
- `jq --arg` for STATE_NAME injection prevention
- Test case split 5a/5b covers the fail-fast contract adequately

**Findings**:

*Major* — `jira_require_dependencies` absent from both flow script algorithms
Location: Phase 2 and Phase 5 Algorithm sections
Every existing flow script calls this as the first statement. Omitting it means missing `jq` or `curl` produces mid-execution failures rather than a clear `E_MISSING_DEP` error.

*Major* — Mutual exclusion of STATE_NAME and `--transition-id` has no specified exit code or test case
Location: Phase 2 Algorithm step 1; Phase 1 Test cases table
The spec declares mutual exclusion but the algorithm silently prefers `--transition-id`, discarding STATE_NAME with no error. No test case covers the combination.

*Minor* — Test case count "11 cases" in Phase 5 success criteria is ambiguous given 5a/5b
Location: Phase 5 Success Criteria
Either renumber 5b as Case 11 or add a parenthetical clarifying that 5a/5b are two assert blocks within Case 5.

*Minor* — Describe branch calls `jira_resolve_credentials` but skips body resolution — asymmetry not documented
Location: Phase 2 Algorithm step 2 (STATE_NAME describe branch)
The asymmetry (credentials needed for GET, body resolution intentionally deferred) is correct but could be surprising to a maintainer. A brief inline note would make it self-documenting.

---

### Test Coverage

**Summary**: Strong TDD discipline. Three major gaps: both guard Cases 4 lack captured-URLs assertions; the `--describe --transition-id` path has no automated test; and Case 3's assertion is too weak to detect an implementation that returns `state: null` after the eager GET.

**Strengths**:
- `transition-describe-guard.json` correctly sequences consuming GET before non-consuming POST guard
- `capture_body: true` used for meaningful behavioural assertions (Cases 8, 9 for transition; Case 2 for attach)
- `grep -c 'filename='` assertion in attach Case 2 is mutation-sensitive
- TDD must-fail gate is explicitly verifiable

**Findings**:

*Major* — Case 4 (transition and attach) guard lacks captured-URLs assertion
Location: Phase 1 Case 4; Phase 4 Case 4
Both cases rely on `consume: false + capture_url: true` but specify no assertion reading `server.captured_urls`. Without it, an accidental POST during `--describe` records the URL but is never checked.

*Major* — No automated test for `--describe --transition-id` path
Location: Phase 1 Cases table; Phase 2 Manual Verification
The unique properties of this path (no credentials, no GET, `state: null`) are only in manual verification. An automated test covering these properties is required.

*Minor* — Case 2 shares fixture with Case 1 but asserts only exit 0
Location: Phase 1 Cases table, Case 2
Using `transition-post-204-capture.json` and asserting the POST body contains `"id":"21"` would make the case-insensitive match mutation-sensitive.

*Minor* — Case 3 asserts `transition_id` not null but does not verify the resolved ID value
Location: Phase 1 Cases table, Case 3
An implementation returning the first transition regardless of name match would pass the current assertion. Pin the ID to `"21"` from the fixture.

*Minor* — Cases 12 and 13 use "inline mock response" — an undocumented pattern
Location: Phase 1 Cases table, Cases 12/13
Clarify whether this means a temp scenario file or reuse of existing error fixtures.

*Minor (low confidence)* — No test case for `--describe STATE_NAME` with 0-match or 2-match from the eager GET
Location: Phase 2 Algorithm step 2; Phase 1 Cases table
The `--describe` branch's 0-match (exit 122) and 2-match (exit 123) paths have dedicated tests only for the live flow path (Cases 5 and 7), not for the describe branch specifically.

---

### Correctness

**Summary**: Credential-resolution ordering is now fully correct. Three major logic gaps: ambiguous-match output is NDJSON but the SKILL.md expects a JSON array; no error is specified for simultaneous STATE_NAME + `--transition-id`; and the offline `--describe --transition-id` path has no automated test.

**Strengths**:
- Credential-resolution ordering correct for all three paths (STATE_NAME describe, --transition-id describe, live)
- `transition-describe-guard.json` two-expectation design is mechanically correct for mock shutdown ordering
- `jq --arg` prevents STATE_NAME injection

**Findings**:

*Major* — Ambiguous-match output is NDJSON but SKILL.md expects a JSON array
Location: Phase 2 Algorithm step 5; Phase 3 SKILL.md Step 4
The jq `select()` filter produces NDJSON for 2+ matches. SKILL.md attempts to parse it as a JSON array and falls back to the "not parseable" error path, breaking the disambiguation UX.

*Major* — No error specified when STATE_NAME and `--transition-id` are supplied together
Location: Phase 2 Algorithm step 1; Phase 1 Cases table
The current algorithm silently prefers `--transition-id`, discarding STATE_NAME. The spec's "mutually exclusive" constraint is not enforced.

*Major* — No automated test for `--describe --transition-id` (no credentials, no GET, `state: null`)
Location: Phase 1 Cases table; Phase 2 Manual Verification
A bug where this path accidentally calls `jira_resolve_credentials` would not be caught.

*Minor* — Phase 2 overview says "15 test cases" but actual count is 17
Location: Phase 2 Overview paragraph
Stale from before Cases 16/17 were added. Success criteria correctly says 17.

---

### Standards

**Summary**: Largely convention-compliant. One material deviation: both new flow scripts source `jira-auth.sh` at load time and call `jira_resolve_credentials` explicitly, contradicting the convention established uniformly by all five existing flow scripts.

**Strengths**:
- Naming conventions (script names, variable names, exit code names, skill directory names) all correct
- Guard fixture design correctly extends the `consume: false + capture_url: true` pair
- SKILL.md frontmatter matches the template exactly
- Test scaffold mirrors `test-jira-comment.sh` in structure
- `test-jira-scripts.sh` updates consistent with accumulator pattern

**Findings**:

*Major* — Both new flow scripts source `jira-auth.sh` at load time, breaking the established convention
Location: Phase 2 Sourcing block; Phase 5 Sourcing block
Verified against five existing flow scripts: none source `jira-auth.sh` or call `jira_resolve_credentials` — they delegate entirely to `jira-request.sh`. The redundant explicit call is a convention violation with no compensating benefit.

*Minor* — Stale case count "15" in Phase 2 overview conflicts with the 17-case test table
Location: Phase 2 Overview paragraph
Change to "17 test cases."

---

### Safety

**Summary**: Strong safety posture for a developer tooling plugin. Two major concerns: the attach Case 3/4 guard delineation leaves the guard potentially unexercised; and the disambiguation loop has no safe abort on user non-response between turns.

**Strengths**:
- Every write path fronted by `--describe` dry-run
- Guard fixtures with `consume: false + capture_url: true` provide automated dry-run safety checks
- Argument validation fully exercised without mock server
- File path safety checks in attach run before `--describe`
- `--transition-id` offline describe correctly isolated from credential resolution

**Findings**:

*Major* — Attach describe Case 3 has no mock; Case 4's guard may never be exercised against an actual `--describe` invocation
Location: Phase 4 Cases table, Cases 3 and 4
Case 3 specifies "none (no mock needed)" — correct for verify-no-network-call. But this means Case 4 must explicitly invoke `--describe` against a running mock to exercise the guard. If an implementer merges Cases 3 and 4 into one test, the guard fixture may be loaded but never actually tested. Specify that Case 4 must invoke `--describe` with the mock running and assert `captured_urls == []`.

*Major* — Disambiguation loop has no safe abort when user stops responding between turns
Location: Phase 3 SKILL.md Step 4
Three failed attempts trigger an explicit abort, but no specification covers what happens if the user closes the session mid-disambiguation. The spec should clarify that each turn presents a fresh prompt with no carry-over state.

*Minor* — Large file uploads proceed without size pre-check; curl timeout is the only backstop
Location: Phase 5 Performance Considerations
Files larger than Jira Cloud's 10 MB limit will produce a cryptic timeout (exit 21) rather than an actionable "file too large" message.

*Minor* — Whitespace-only `--resolution` passes exit 126 guard
Location: Phase 2 Algorithm step 1
`--resolution '   '` passes the empty-string check and produces a Jira 400 error (exit 34) rather than a clear local validation failure.

*Minor* — `transition-describe-guard.json` GET expectation is single-use (`consume: true` by default)
Location: Phase 1 fixture, transition-describe-guard.json
A future test refactor that invokes `--describe` twice in Case 4 would see a 500 on the second GET. Document this as single-use.

---

### Security

**Summary**: jq injection prevention for STATE_NAME is explicit and correct; file path sanitisation in attach is well-specified. Two gaps: `--transition-id` has no numeric-only validation; and `--comment-file` in the transition flow lacks the path guards that attach correctly specifies.

**Strengths**:
- `jq --arg` mandate for STATE_NAME explicitly documented
- `jira_resolve_credentials` deferred past `--describe` for offline paths
- `transition-describe-guard.json` consuming GET expectation correctly added
- Three-layer path sanitisation in attach (dash, symlink-to-device, existence/readability) well-specified
- `X-Atlassian-Token` injection delegated to `jira-request.sh`
- All `--multipart` file paths quoted to prevent word-splitting
- Trust-boundary paragraph correctly scoped to `--comment` content only

**Findings**:

*Major* — `--transition-id` value not validated as numeric before use in POST body
Location: Phase 2 Algorithm step 1; Algorithm step 4
Without `^[0-9]+$` validation, non-numeric values pass argument parsing and reach the POST body, producing confusing server-side 400 errors rather than a clear local validation failure.

*Major* — `--comment-file` path in transition flow lacks dash-prefix and symlink-to-device guards
Location: Phase 2 Algorithm step 1 (`--comment-file` validation)
The three-layer guards specified for attach are absent here. `--comment-file -` reads from stdin; `--comment-file /dev/urandom` passes both existence and readability checks on most systems.

*Minor* — `--resolution NAME` not explicitly specified to use `jq --arg` in POST body
Location: Phase 2 Algorithm step 6
The `STATE_NAME` mandate is documented; `--resolution NAME` is not. Mirror the same documentation pattern.

*Minor* — Disambiguation chosen ID not validated as numeric before shell re-invocation
Location: Phase 3 SKILL.md Step 4
User-supplied disambiguation reply is interpolated into the re-invocation command. Specify numeric-only validation before constructing the Bash call.

*Minor (low confidence)* — File paths with embedded newlines could survive path validation and corrupt multipart arguments
Location: Phase 5 Algorithm step 2
A file path containing a newline would corrupt the `"file=@<path>"` string assembled for curl. A control-character rejection guard would prevent this.

---

### Usability

**Summary**: Good developer experience overall. Two major gaps that will affect skill authors directly: the disambiguation loop-back instruction omits the rationale for dropping STATE_NAME; and Case 3's assertion is too weak to detect the eager-GET regression it is meant to guard.

**Strengths**:
- `--describe` pattern consistently applied with clear offline/online distinction per path
- Guard fixture update correctly closes the pass-2 gap
- Disambiguation retry limit (3 attempts + abort) matches established pattern
- File validation before `--describe` makes describe mode accurate for attach

**Findings**:

*Major* — Case 3 assertion does not verify `state` is non-null
Location: Phase 1 Cases table — Case 3 (Key assertion column)
An implementation using the `--transition-id` describe path (no GET, `state: null`) would satisfy "exit 0; stdout has resolved `transition_id`" while silently bypassing the eager GET. The assertion must also check `state != null`.

*Major* — Disambiguation loop-back instruction omits the reason STATE_NAME must be dropped
Location: Phase 3 SKILL.md prose spec — Step 4 loop-back
"(omit STATE_NAME)" without rationale creates a plausible skill-author mistake. A one-sentence explanation prevents it.

*Minor* — Success message after disambiguation omits the state name the user originally requested
Location: Phase 3 SKILL.md Step 8
After disambiguation via `--transition-id`, `state` is null and Step 8 renders "transition ID <ID> applied" — not the state name requested. The original STATE_NAME is known from the user's invocation and could be carried through.

*Minor* — No exit code or test case for simultaneous STATE_NAME + `--transition-id`
Location: Phase 2 Algorithm step 1; Phase 1 Cases table
The mutual-exclusion constraint has no enforcement and no test, creating a silently inconsistent behaviour for skill authors.

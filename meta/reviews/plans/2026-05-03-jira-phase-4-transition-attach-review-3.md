---
date: "2026-05-04T00:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-03-jira-phase-4-transition-attach.md"
review_number: 3
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, standards, safety, security, usability]
review_pass: 4
status: complete
---

## Plan Review: Jira Phase 4 — Workflow and Attachments (Pass 4)

**Verdict:** REVISE

The pass-3 fixes are substantially correct: credential delegation is clean, `jira_require_dependencies` is present in both scripts, the jq filter uses `[...]` wrapping, mutual exclusion is specified, and the three-layer `--comment-file` guards are in place. However, three critical-severity defects emerged. The `[...]` wrapping introduced by pass 3 broke the downstream `.id` extraction — the plan says "take its `.id`" but the single match is now at `.[0].id`, meaning an implementation following the plan verbatim would POST `{"transition":{"id":null}}` on every state-name lookup. Two related fixture problems mean transition Cases 12/13 will silently mis-exercise the mock (wrong endpoint path), and the attach Case 9 has no implementable fixture at all. Thirteen additional major findings cover algorithm gaps (no shared lookup helper, unspecified `jira_resolve_body` flag mapping, underspecified mutual-exclusion check ordering), missing test cases (offline-describe credential isolation, non-numeric `--transition-id`, `--no-notify` URL parameter, `--describe STATE_NAME` 0-match), a guard ordering bug (stop_mock must precede the captured-URLs assertion), and two SKILL.md gaps (confirmation-gate revise branch, missing exit 22 in the exit-code table).

---

### Cross-Cutting Themes

- **Fixture path mismatch** (flagged by: Test Coverage, Standards) — The plan directs transition Cases 12/13 to reuse `error-401.json` and `error-404.json`, but those fixtures hardcode `"path": "/rest/api/3/myself"`. The mock server does exact path matching; a request to `/rest/api/3/issue/ENG-1/transitions` produces a 500 "Unexpected request" error rather than a 401/404, silently voiding the error-propagation tests. The attach Case 9 has the same problem — no named fixture file exists or is listed in the desired end state. Both issues stem from misapplying the reuse convention from `test-jira-request.sh` (which does call `/myself`) to the new test scripts (which do not).

- **`.[0].id` extraction gap** (flagged by: Correctness, Code Quality) — The `[...]` wrapping fix from pass 3 was applied correctly to the jq filter but not propagated to the ID extraction step. The plan says "take its `.id`" in step 4 and "same as step 4" in the `--describe` STATE_NAME branch. Both must use `.[0].id` after the array wrapping, or every state-name lookup path posts a null transition ID to Jira.

- **Guard-assertion ordering** (flagged by: Safety, Code Quality) — `mock-jira-server.py` writes the captured-URLs file only in its `finally` block, which runs after `server.serve_forever()` returns. `stop_mock` triggers shutdown. If `jq -c '.' "$CAPTURED_URLS"` runs before `stop_mock`, the file is unwritten (or stale), `jq` returns `[]` regardless of whether a stray POST occurred, and the guard silently passes. The plan specifies the assertion but not the ordering. Both Case 4s must mandate `stop_mock` before the assertion.

---

### Findings

#### Critical

- 🔴 **Test Coverage + Standards**: error-401.json and error-404.json have wrong paths — transition Cases 12/13 will mock-500 at runtime
  **Location**: Phase 1: Test script — Cases 12 and 13; Desired End State (missing fixtures)
  `error-401.json` and `error-404.json` have expectations locked to `"path": "/rest/api/3/myself"`. The transition test calls `GET /rest/api/3/issue/ENG-1/transitions`. The mock returns HTTP 500 "Unexpected request" on a path mismatch — not 401/404. Both error-propagation cases will always FAIL after implementation, giving false negatives and hiding bugs in 401/404 handling. Two new fixtures are required: `transition-list-401.json` and `transition-list-404.json` with the correct path, added to the desired end state.

- 🔴 **Test Coverage + Standards**: Attach Case 9 has no implementable fixture — inline mock response pattern does not exist in the codebase
  **Location**: Phase 4: Test script — Case 9; Desired End State (missing attach-post-401.json)
  Attach Case 9 ("POST 401 → exits 11") has "(inline mock response)" in its fixture column. The test infrastructure has no inline fixture mechanism — `start_mock` always takes a file path. `error-401.json` targets `/rest/api/3/myself`. No `attach-post-401.json` file is listed in the desired end state. The case has no viable implementation path as written.

- 🔴 **Correctness**: `.id` extraction incorrect after `[...]` wrapping — every state-name lookup posts `id: null`
  **Location**: Phase 2: Algorithm Step 4 (and the mirrored lookup inside Step 2 `--describe` STATE_NAME branch)
  The jq filter `'[.transitions[] | select(...)]'` wraps all matches into a JSON array. Step 4 then says "If the array has 1 entry: take its `.id`". In jq, `.id` on an array returns `null`. The correct path is `.[0].id`. The same bug applies to the `--describe` STATE_NAME branch which calls "the state-name lookup (same as step 4 below)". An implementation following the plan verbatim would POST `{"transition":{"id":null}}`, which Jira rejects with 400 (exit 34). Cases 1, 2, 8, and 9 would silently fail with exit 34 instead of exit 0; Case 14 (POST 400) would spuriously pass.

#### Major

- 🟡 **Architecture**: STATE_NAME lookup logic is duplicated between `--describe` and normal flow — no shared helper planned
  **Location**: Phase 2: Flow script Algorithm Steps 2 and 4
  The plan specifies that `--describe` with STATE_NAME "performs the state-name lookup (same as step 4 below)" but there is no shared private function planned. The GET + jq filter + empty/ambiguous/single-match branching will be written twice in the same file. Any future change to the lookup logic (e.g. expanding the match field, adding query params) must be applied in both places, and the duplication is invisible at code-review time.

- 🟡 **Code Quality**: `--comment-file` validation duplicates logic from `jira-body-input.sh` inline
  **Location**: Phase 2: Algorithm Step 1 (`--comment-file` validation)
  The plan specifies three-layer `--comment-file` guards inline in argument parsing, rather than routing through `jira_resolve_body` from the already-sourced `jira-body-input.sh`. Any future change to file validation rules must be applied in both the inline guards and the helper, creating a silent divergence risk.

- 🟡 **Code Quality**: `transition-describe-guard.json` mixes a consuming GET with a non-consuming POST — guard failure is indistinguishable from a credential error
  **Location**: Phase 1: Test fixture `transition-describe-guard.json`; Phase 1: Test script Case 4
  If `--describe` exits before reaching the GET (e.g., due to a credential resolution error that the updated algorithm should have eliminated), the consuming GET expectation is never consumed — but the POST's captured-URLs assertion still returns `[]` (no POST was made), giving a false pass. The guard cannot distinguish "no write was made because --describe worked correctly" from "no write was made because the script exited early before making any request."

- 🟡 **Code Quality**: Multi-file `--multipart` argument assembly pattern is underspecified
  **Location**: Phase 5: Flow script Algorithm Step 4
  The plan says "one `--multipart "file=@<path>"` per file" and "all file paths must be quoted." It does not specify the bash array-construction idiom. With `set -euo pipefail`, the null-safe array expansion idiom (`"${array[@]+${array[@]]}"`) from `jira-update-flow.sh` lines 335–344 is required — an implementer using naive `"${array[@]}"` will hit a `set -u` abort on an empty array.

- 🟡 **Test Coverage**: Case 18 credential isolation is underspecified — standard wrapper sets `ACCELERATOR_JIRA_TOKEN`
  **Location**: Phase 1: Test script — Case 18
  Case 18 invokes `--describe ENG-1 --transition-id 21` "with no credentials set," but provides no credential-free wrapper function. The standard `transition()` wrapper (analogous to `comment()` in `test-jira-comment.sh`) will unconditionally set `ACCELERATOR_JIRA_TOKEN="$TEST_TOKEN"`, meaning credentials are present when they must not be. A dedicated `transition_no_creds()` wrapper that unsets the token, URL override, and site config is required.

- 🟡 **Test Coverage**: No test case for non-numeric `--transition-id` value despite algorithm mandating exit 124
  **Location**: Phase 1: Test script — case table; Phase 2: Algorithm Step 1
  The algorithm explicitly states exit 124 for `--transition-id ID` where `ID` does not match `^[0-9]+$`. No test case exercises this path. A mutation that removes the numeric validation would not be caught by any test.

- 🟡 **Test Coverage**: No automated test for `--no-notify` adding `?notifyUsers=false` to the transition POST URL
  **Location**: Phase 1: Test script — case table; Phase 2: Algorithm Step 6
  The `--no-notify` flag is part of the flow script interface, and the algorithm specifies `?notifyUsers=false` appended to the POST URL when present. None of the 20 cases verify this URL transformation. A mutation that silently drops `--no-notify` would go undetected.

- 🟡 **Correctness**: No test case for `--describe STATE_NAME` 0-match path (exit 122 from describe branch)
  **Location**: Phase 1: Test cases table; Phase 2: Algorithm Step 2
  Step 2 has three branches for `--describe` + STATE_NAME: 0 matches → exit 122, 1 match → describe JSON, 2+ matches → exit 123. Case 3 covers the 1-match branch; Case 5 covers 2+ matches in the non-describe flow. No case tests `--describe ENG-1 "Nonexistent"` → exit 122 from the describe path specifically. An implementation that handles step 4's 0-match correctly but omits the same check in step 2's describe branch would pass all tests while silently misbehaving.

- 🟡 **Correctness**: Mutual exclusion check ordering underspecified — `--transition-id`-first argument order may slip through
  **Location**: Phase 2: Algorithm Step 1 — mutual exclusion check
  The plan says "exit 124 if both STATE_NAME and `--transition-id` are supplied simultaneously" but does not specify whether the check is during parsing or post-parse. A during-parse implementation that only checks for `--transition-id` arriving after `STATE_NAME` will miss the reversed ordering (`ENG-1 --transition-id 21 "Done"`). Case 19 does not specify which ordering is tested, so only one direction may be covered.

- 🟡 **Correctness**: `jira_resolve_body` flag mapping from `--comment`/`--comment-file` to `--body`/`--body-file` is unspecified
  **Location**: Phase 2: Algorithm Step 5 (POST body construction — comment ADF)
  The flow script uses `--comment TEXT` and `--comment-file PATH` as user-facing flags, but `jira_resolve_body` accepts `--body`/`--body-file`. The algorithm says "ADF converted via `jira-md-to-adf.sh`" but does not describe how the body text is obtained — whether via `jira_resolve_body --body "$COMMENT_TEXT"` or by reading the file directly. Calling `jira_resolve_body --comment "$TEXT"` would hit `E_BODY_BAD_FLAG` (exit 1), which the flow script would not map correctly to exit 125.

- 🟡 **Safety**: Guard assertion silently vacuous if `stop_mock` is called after the `jq` assertion
  **Location**: Phase 1: Test script Case 4 (transition); Phase 4: Test script Case 4 (attach)
  `mock-jira-server.py` writes the captured-URLs file only in its `finally` block — after `server.serve_forever()` returns. `stop_mock` triggers shutdown. If `jq -c '.' "$CAPTURED_URLS"` runs before `stop_mock`, the file is either absent or stale, `jq` returns `[]` regardless of whether a stray POST was captured, and the assertion always passes. Both Case 4s must mandate: `stop_mock` → then assert captured URLs, mirroring `test-jira-comment.sh` Case 25a.

- 🟡 **Usability**: Confirmation gate "n / revise" branch not specified for transition SKILL.md
  **Location**: Phase 3: SKILL.md prose spec — Step 6
  The Step 6 spec says only to use the canonical confirm phrase. It does not specify the three-branch interpretation: clear-yes proceeds, clear-no triggers a revision loop (re-apply trust-boundary check, rebuild from Step 3, 3-revision cap), ambiguous/off-topic aborts. Both `update-jira-issue` and `comment-jira-issue` SKILL.md files specify the full three-branch model. Without it, `skill-creator` will likely omit the revision loop, leaving users in a dead-end after replying "n".

- 🟡 **Usability**: Exit 22 (`E_REQ_NO_CREDS`) missing from transition SKILL.md Step 9 exit-code table
  **Location**: Phase 3: SKILL.md prose spec — Step 9 (exit-code table)
  The inherited code list reads "11, 13, 19, 20, 21, 34" but omits exit 22 (`E_REQ_NO_CREDS`), the code emitted by `jira-request.sh` when no credentials are resolvable at all — a common first-run failure. The transition `--describe STATE_NAME` path makes an eager GET, so exit 22 can surface during the preview stage. Both `update-jira-issue` and `comment-jira-issue` SKILL.md files include exit 22. Exit 12 should also appear explicitly.

#### Minor

- 🔵 **Architecture**: Symlink-to-device rejection untested for `/proc` and `/sys` paths
  **Location**: Phase 5: Algorithm Step 2 (file validation); Phase 4: Test script case table
  The three-layer guard rejects symlinks to `/dev`, `/proc`, and `/sys`, but no test case creates a symlink resolving to `/proc` or `/sys`. An implementer that implements only the `/dev` check would pass all tests.

- 🔵 **Code Quality**: EXIT_CODES.md description for exit 125 shorter than the algorithm spec
  **Location**: Phase 2: EXIT_CODES.md update row for 125
  The update block uses "`--comment-file` not found" but the flow script exit-code table says "`--comment-file` path invalid (dash-prefix, symlink to device, missing, or unreadable)". A maintainer reading EXIT_CODES.md will not know exit 125 also covers dash-prefix and symlink cases.

- 🔵 **Code Quality**: `wc -c` size check mentioned in Performance section but absent from algorithm steps
  **Location**: Performance Considerations section; Phase 5: Algorithm Step 2
  The Performance section states "Size is checked via `wc -c` on each file during step 2 validation," but step 2 lists only three checks (dash-prefix, symlink, existence/readability). An implementer following only the algorithm steps would omit the large-file warning.

- 🔵 **Test Coverage**: Attach Case 3 does not specify file setup — validation runs before `--describe`
  **Location**: Phase 4: Test script — Case 3
  Attach Case 3 tests `--describe` with no mock, but the flow script validates file existence before the `--describe` short-circuit. The case must supply a real file (created via `mktemp`) or it will exit 132 before producing any describe output, silently testing file-not-found instead of the describe path.

- 🔵 **Test Coverage**: Attach Case 2 body assertion does not verify correct filenames
  **Location**: Phase 4: Test script — Case 2
  `grep -c 'filename=' body == 2` would also pass if the implementation sends the same file twice. A stronger assertion verifying both basenames (e.g. `grep -q 'filename="a.txt"'` and `grep -q 'filename="b.txt"'`) is needed.

- 🔵 **Correctness**: EXIT_CODES.md Phase 4/5 namespace summary heading is inconsistently renamed
  **Location**: Phase 2: EXIT_CODES.md update (rename instruction)
  The plan renames `## Phase 3 namespace summary` to `## Phase 4 namespace summary` but then also instructs Phase 5 to add attach rows to the same section. Both transition (Phase 4) and attach (Phase 5) end up under "Phase 4". Adding a separate `## Phase 5 namespace summary` section for the attach ranges (or renaming to "Phase 4–5") would be more accurate and additive-only.

- 🔵 **Safety**: Whitespace-only `--resolution` value has no dedicated test case
  **Location**: Phase 2: Algorithm Step 1 (exit 126 guard); Phase 1: Test case table
  Case 17 tests `--resolution ''` (empty string). There is no case for `--resolution '   '` (whitespace-only), which requires an explicit pattern match (`[[ "$RESOLUTION" =~ ^[[:space:]]*$ ]]`), not a simple `-z` check. A naive implementation passes Case 17 but silently sends a whitespace-only resolution to Jira.

- 🔵 **Safety**: Large-file warning ownership ambiguous between flow script and SKILL.md
  **Location**: Performance Considerations; Phase 5: Algorithm Step 2; Phase 6: SKILL.md Step 4
  The Performance section says the flow script should emit a warning in step 2. The SKILL.md spec also runs `wc -c` in Step 4 for display. It is unspecified whether the flow script's `wc -c` fires in `--describe` mode (validation runs before the short-circuit, so logically yes), and whether the two checks are intentionally redundant or whether one is the canonical implementation.

- 🔵 **Usability**: Exit 133 (`E_ATTACH_BAD_FLAG`) not handled in attach SKILL.md Step 3 error spec
  **Location**: Phase 6: SKILL.md prose spec — Step 3 (preview-failure handling)
  Step 3 maps exit 132 to "File not found" and all other non-zero exits to the generic "Preview failed" message. Exit 133 (unrecognised flag) is a user-recoverable error with a clear fix (correct the flag). A specific "Unrecognised flag. Usage: /attach-jira-issue ISSUE-KEY FILE [FILE...] [--quiet]" message would provide actionable feedback instead of a confusing generic failure.

- 🔵 **Usability**: EXIT_CODES.md description for exit 124 understates its scope
  **Location**: Phase 2: EXIT_CODES.md update row for 124
  The update row reads `E_TRANSITION_BAD_FLAG | Unrecognised flag` but exit 124 is now defined to fire on three conditions: unrecognised flag, conflicting argument combination (STATE_NAME + `--transition-id`), and non-numeric `--transition-id` value. A developer who sees exit 124 in a log and consults EXIT_CODES.md will only read "Unrecognised flag" and may debug the wrong thing.

- 🔵 **Standards**: Both SKILL.md prose specs omit the config-read include lines required by existing write skills
  **Location**: Phase 3: SKILL.md prose spec (transition); Phase 6: SKILL.md prose spec (attach)
  Every existing write-skill SKILL.md opens its body with two `!`-include lines: `` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh` `` and `` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh <skill-name>` ``. Both prose specs do not mention these lines, so a `skill-creator` invocation following the spec as written will produce SKILL.md files missing them.

- 🔵 **Standards**: EXIT_CODES.md section heading rename destroys Phase 3 history rather than adding a Phase 4 section
  **Location**: Phase 2: EXIT_CODES.md update — section heading instruction
  The plan instructs renaming `## Phase 3 namespace summary` to `## Phase 4 namespace summary`, which mutates existing documented history. The Migration Notes section states the changes are "purely additive." A new `## Phase 4 namespace summary` section alongside the existing one is the additive-only approach.

- 🔵 **Security**: Curl `-F` metacharacter injection via file paths containing semicolons
  **Location**: Phase 5: Algorithm Step 4 (`--multipart` argument assembly)
  Curl's `-F` flag interprets semicolons as separators for parameter modifiers (e.g., `;type=text/html`). A file path containing a semicolon — such as `/tmp/upload;type=application/x-sh` — would override the MIME type of the uploaded part. The three-layer path guard does not include a semicolon filter.

---

### Strengths

- ✅ Credential resolution correctly delegated to `jira-request.sh` in both flow scripts — no `jira-auth.sh` sourcing, no explicit `jira_resolve_credentials` calls
- ✅ `jira_require_dependencies` placed before argument parsing in both scripts, matching the exact position from `jira-update-flow.sh`
- ✅ Mutual exclusion (STATE_NAME + `--transition-id` → exit 124) is now specified with an exit code and a test case (Case 19)
- ✅ Three-layer `--comment-file` guards (dash-prefix, symlink-to-device, existence/readability) now match the attach flow's path sanitisation
- ✅ Numeric-only validation for `--transition-id` (`^[0-9]+$` → exit 124) correctly prevents non-numeric values from reaching the POST body
- ✅ The `[...]` wrapping on the jq filter correctly unifies 0/1/2+ match handling into a single array representation (though the `.id` extraction step needs the follow-on fix)
- ✅ Disambiguation numeric validation in SKILL.md Step 4 (validate chosen ID as digits-only before re-invocation) correctly closes the shell injection vector
- ✅ Case 18 intent (offline `--describe --transition-id` with no credentials) is the right test — the isolation mechanism just needs specification
- ✅ Cases 19 and 20 close the mutual exclusion and dash-prefix gaps identified in pass 3
- ✅ The `stop_mock`-before-assertion ordering is correct in `test-jira-comment.sh` Case 25a; the fix for Case 4 is to mirror that same pattern
- ✅ Case 3 assertion strength (`jq -e '.state != null and .transition_id == "21"'`) is now sufficient to detect the eager-GET regression it guards against
- ✅ File path validation in attach runs before `--describe` short-circuit, making describe mode honest about invalid paths
- ✅ `jira-md-to-adf.sh` subprocess note correctly prevents a sourcing mistake

---

### Recommended Changes

1. **Fix `.[0].id` extraction in step 4 and step 2 describe branch** (addresses: Correctness — id extraction)
   Change "take its `.id`" to "take its `.[0].id`" in both step 4 and the step 2 `--describe` STATE_NAME branch. Add a concrete example: `transition_id=$(printf '%s' "$matches" | jq -r '.[0].id')`.

2. **Add `transition-list-401.json`, `transition-list-404.json`, and `attach-post-401.json` fixtures** (addresses: Critical — fixture path mismatch; Critical — attach Case 9)
   Create three new scenario files with the correct endpoint paths (`/rest/api/3/issue/ENG-1/transitions` for transition error fixtures, `/rest/api/3/issue/ENG-1/attachments` for attach). Add all three to the desired end-state file list. Update Cases 12, 13 (transition) and Case 9 (attach) to reference the named fixtures.

3. **Specify a private lookup helper function for the STATE_NAME transition lookup** (addresses: Architecture — duplicated logic; Code Quality — duplicated logic)
   Add a `_jira_transition_lookup KEY STATE_NAME` internal helper that encapsulates the GET + `[...]`-wrapped filter + match-count branching. Both the `--describe` step 2 branch and the live step 4 path call this helper, eliminating duplication.

4. **Specify `jira_resolve_body` flag mapping for `--comment`/`--comment-file`** (addresses: Correctness — flag mapping)
   Add to the algorithm: for `--comment TEXT`, pass `--body "$COMMENT_TEXT"` to `jira_resolve_body`; for `--comment-file PATH`, pass `--body-file "$PATH"`. Note that after the three-layer path validation in step 1, `--comment-file` is guaranteed to point to a readable regular file, so either routing through the helper or reading directly is acceptable — but the choice must be explicit.

5. **Specify post-parse mutual exclusion check and add reversed-ordering test** (addresses: Correctness — check ordering)
   Specify that the STATE_NAME + `--transition-id` conflict check is performed after the full argument-parsing loop, comparing `[[ -n "$STATE_NAME" && -n "$TRANSITION_ID" ]]`. Update Case 19 to explicitly test the reversed ordering: `ENG-1 --transition-id 21 "Done"` → exit 124.

6. **Add `transition_no_creds()` wrapper spec to Case 18** (addresses: Test Coverage — credential isolation)
   Specify a credential-free wrapper that unsets `ACCELERATOR_JIRA_TOKEN`, `ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST`, and any site config before invoking the script, used exclusively by Case 18.

7. **Add test case for non-numeric `--transition-id`** (addresses: Test Coverage — non-numeric ID)
   Add Case 21: `ENG-1 --transition-id abc` → exit 124, no network call (argument validation before any HTTP call; no fixture required).

8. **Add test case for `--no-notify` URL parameter** (addresses: Test Coverage — --no-notify)
   Add a case (e.g. Case 22): invoke the happy path with `--no-notify`; use a fixture variant with `capture_url: true` on the POST expectation; assert the captured URL contains `notifyUsers=false`.

9. **Add test case for `--describe STATE_NAME` 0-match path** (addresses: Correctness — 0-match from describe)
   Add Case 3b: `--describe ENG-1 "Nonexistent"` → exit 122, using `transition-list-200.json` (which has no "Nonexistent" state). Tests the 0-match branch of the `--describe` STATE_NAME handler independently from Case 7.

10. **Specify `stop_mock` ordering before captured-URLs assertion in both Case 4s** (addresses: Safety — guard ordering)
    For both transition Case 4 and attach Case 4, add an explicit ordering note: `stop_mock` must be called before `jq -c '.' "$CAPTURED_URLS"`. Mirror the exact sequence from `test-jira-comment.sh` Case 25a.

11. **Add confirmation-gate "n / revise" branch spec to transition SKILL.md Step 6** (addresses: Usability — confirmation gate)
    Add the three-branch interpretation to the Step 6 spec: clear-yes proceeds to Step 7; clear-no triggers "What would you like to change?" and rebuilds from Step 3 (with trust-boundary check for `--comment` revisions, 3-revision cap); ambiguous/off-topic aborts with "Aborted — no Jira write was made." Mirror `update-jira-issue` SKILL.md lines 124–135.

12. **Add exit 22 (and exit 12 explicitly) to transition SKILL.md Step 9 exit-code table** (addresses: Usability — missing exit 22)
    Expand the inherited-code list to include exit 22 (`E_REQ_NO_CREDS`) and confirm exit 12 is listed. Group as "11, 12, 22 | auth | Check credentials with /init-jira", matching the pattern from `update-jira-issue` SKILL.md.

13. **Add config-read include lines to both SKILL.md prose specs** (addresses: Standards — missing include lines)
    Add the two `!`-include lines to both prose specs immediately after the frontmatter block, matching `comment-jira-issue/SKILL.md` lines 18–19.

---

*Review generated by /review-plan*

---

## Per-Lens Results

### Architecture

**Summary**: Structurally sound and consistent with the established layering. One major concern: the STATE_NAME lookup logic is specified to exist in both the `--describe` branch and the live flow without a shared private function, creating a silent duplication that will accumulate change-risk over time. Three minor concerns: ADF subprocess pattern (acceptable given established convention), dual-layer numeric validation across SKILL.md and flow script (intentionally redundant), and the symlink guard having no test for `/proc`/`/sys` paths.

**Strengths**:
- Credential resolution fully delegated to `jira-request.sh` — no auth state in the flow script's process
- `--transition-id` offline describe correctly short-circuits before any I/O
- File validation in attach placed before `--describe` short-circuit — correct layer ownership
- Guard fixture two-expectation design (consuming GET + non-consuming POST) is structurally sound for the eager-GET describe path
- Exit-code namespace managed purely additively with reserved ranges documented
- `wc -c` size pre-check placed in flow script step 2 validation — correct layer for an operational constraint

**Findings**:

*Major* — STATE_NAME lookup logic is duplicated between `--describe` and normal flow — no shared helper planned
Location: Phase 2: Flow script Algorithm Steps 2 and 4
The GET + jq filter + match-count branching will be written twice in the same file. Any future change to the lookup algorithm must be applied in both places; the duplication is invisible at code-review time. A private `_jira_transition_lookup` helper called from both paths eliminates this.

*Minor* — Symlink-to-device rejection untested for `/proc` and `/sys` prefixes
Location: Phase 5: Algorithm Step 2; Phase 4: Test case table
An implementation checking only `/dev` would pass all specified tests while leaving `/proc` and `/sys` targets unguarded.

*Minor* — Disambiguation re-invocation creates dual-layer numeric validation
Location: Phase 3: SKILL.md Step 4; Phase 2: Algorithm Step 1
The SKILL.md's digit-only check and the flow script's `^[0-9]+$` guard are intentionally redundant (SKILL.md for UX, flow script for correctness), but this should be documented so maintainers do not remove one as an apparent duplicate.

---

### Code Quality

**Summary**: Most algorithm descriptions are detailed enough to implement. Three major gaps: `--comment-file` validation is specified inline rather than routed through the already-sourced `jira-body-input.sh`; the guard fixture's consuming GET creates an indistinguishable failure mode; and the multi-file `--multipart` array construction idiom is underspecified for `set -euo pipefail` environments.

**Strengths**:
- `jq_require_dependencies` placed before argument parsing in both scripts
- jq `[...]`-wrapped filter is explicitly called out with the `--arg` parameterisation — both good defensive habits
- `transition-post-204-direct.json` (POST-only) correctly guards the `--transition-id` bypass path
- Exit code naming follows `E_<VERB>_<NOUN>` convention consistently

**Findings**:

*Major* — `--comment-file` validation duplicates `jira-body-input.sh` inline
Location: Phase 2: Algorithm Step 1
Writing the three path guards inline creates two independent implementations; any future fix to the validation rules must be applied in both places.

*Major* — Guard fixture mixes consuming GET with non-consuming POST — guard failure is indistinguishable from an early exit
Location: Phase 1: `transition-describe-guard.json`; Case 4
If the script exits before reaching the GET (e.g. a credential error), the captured-URLs assertion still returns `[]` and the case passes, concealing the early exit.

*Major* — Multi-file `--multipart` array expansion pattern underspecified
Location: Phase 5: Algorithm Step 4
Without specifying the null-safe `"${array[@]+${array[@]]}"` idiom from `jira-update-flow.sh` lines 335–344, an implementer may use `"${array[@]}"` which aborts under `set -u` when the array is empty.

*Minor* — EXIT_CODES.md description for exit 125 shorter than the algorithm spec
*Minor* — `wc -c` size check mentioned in Performance section but absent from algorithm steps
*Minor* — Case 18 credential isolation underspecified (overlaps with Test Coverage major)

---

### Test Coverage

**Summary**: Strong TDD discipline overall. Two critical fixture problems will prevent Cases 12/13 (transition) and Case 9 (attach) from exercising the intended error paths at runtime. Three additional major gaps: Case 18 credential isolation, missing non-numeric `--transition-id` test, missing `--no-notify` URL test.

**Strengths**:
- Guard fixtures correctly use both `consume: false` and `capture_url: true` together on all dry-run paths
- Cases 19, 20 close the mutual-exclusion and dash-prefix gaps from pass 3
- Case 18 intent (offline describe, no credentials) is the right test — the isolation mechanism needs specification
- `transition-post-204-direct.json` (POST-only) provides an implicit guard for the bypass path

**Findings**:

*Critical* — `error-401.json` / `error-404.json` wrong paths — transition Cases 12/13 will produce mock 500s
*Critical* — Attach Case 9 has no implementable fixture

*Major* — Case 18 credential-free invocation underspecified
*Major* — No test for non-numeric `--transition-id`
*Major* — No test for `--no-notify` URL parameter

*Minor* — Attach Case 3 doesn't specify file setup (mktemp)
*Minor* — Attach Case 2 doesn't verify correct filenames (only count)
*Minor* — Transition Case 3 has no explicit request-count assertion (relies on mock's implicit 500 on unexpected requests)
*Minor* — Transition Case 9 ADF assertion shallow (only checks `"type":"doc"`, not content)

---

### Correctness

**Summary**: The `[...]`-wrapping fix from pass 3 is the right approach but created a downstream ID-extraction bug. Two additional major logic gaps: no test for `--describe STATE_NAME` 0-match, and the mutual-exclusion check ordering is underspecified for the `--transition-id`-first argument order. The `jira_resolve_body` flag mapping gap could cause the comment body path to fail silently.

**Strengths**:
- `[...]`-wrapped jq filter correctly handles 0/1/2+ matches with length-based branching
- Credential-resolution ordering correct: both describe paths and live paths work without explicit auth calls
- `jira-md-to-adf.sh` subprocess note prevents a sourcing mistake

**Findings**:

*Critical* — `.[0].id` extraction incorrect — array wrapping breaks `.id` access
*Major* — No test for `--describe STATE_NAME` 0-match path (exit 122 from describe branch)
*Major* — Mutual exclusion check ordering underspecified — `--transition-id`-first ordering may slip through
*Major* — `jira_resolve_body` flag mapping from `--comment`/`--comment-file` to `--body`/`--body-file` unspecified

*Minor* — EXIT_CODES.md Phase 4/5 namespace summary heading inconsistency
*Minor* — Attach `--describe` guard provides redundant protection (mock 500 on unexpected method/path also catches accidental POSTs)

---

### Standards

**Summary**: Sourcing convention, variable naming, exit code naming, and test script structure are all correct. Two remaining deviations: the fixture reuse for error cases applies the wrong convention (the reused fixtures target the wrong endpoint path), and the SKILL.md prose specs are missing the config-read include lines that all existing write skills require.

**Strengths**:
- Sourcing blocks match the established pattern exactly
- `_JIRA_TRANSITION_SCRIPT_DIR` and `_JIRA_ATTACH_SCRIPT_DIR` follow the existing naming convention
- Guard fixtures correctly extend the `consume: false + capture_url: true` pair
- SKILL.md frontmatter matches the template exactly

**Findings**:

*Critical* — Error fixtures (`error-401.json`, `error-404.json`) hardcoded to wrong endpoint path
*Major* — Attach Case 9 still uses "inline mock response" instead of a named fixture file

*Minor* — SKILL.md prose specs omit config-read include lines
*Minor* — EXIT_CODES.md section heading rename destroys Phase 3 history rather than adding Phase 4 section
*Minor* — Attach SKILL.md Step 8 exit-code table includes exit 34 which is not a realistic response from the attachments endpoint

---

### Safety

**Summary**: Dry-run guard fixtures are structurally correct, captured-URLs assertion is now explicitly specified. One major gap: the assertion ordering is not specified — `stop_mock` must precede the captured-URLs `jq` call or the file is unwritten at assertion time, voiding the guard entirely.

**Strengths**:
- Captured-URLs assertion (`jq -c '.' "$CAPTURED_URLS"` equals `[]`) now explicitly in both Case 4s
- File validation in attach runs before `--describe` — describe mode is honest about invalid paths
- Exit 126 guard covers whitespace-only values (conceptually — Case 17b is missing as a test)

**Findings**:

*Major* — Guard assertion silently vacuous if `stop_mock` called after `jq` assertion

*Minor* — Whitespace-only `--resolution` has no dedicated test case (Case 17b)
*Minor* — Large-file warning ownership ambiguous (flow script step 2 vs SKILL.md Step 4)
*Minor* — Guard fixture GET expectation is single-use — second `--describe` invocation in Case 4 gets 500 (acceptable, worth documenting)

---

### Security

**Summary**: All three critical security findings from pass 3 are correctly resolved. One residual minor concern: curl's `-F` flag interprets semicolons as parameter-modifier separators, and a file path containing a semicolon could override the MIME type of an uploaded part. All other security controls are robust.

**Strengths**:
- `--transition-id` numeric validation closes the non-numeric injection vector
- `--resolution NAME` uses `jq --arg r "$RESOLUTION"` — not string-interpolated
- `--comment-file` three-layer guards are in the correct order and placement
- Disambiguation chosen ID validated as digits-only before shell re-invocation
- `STATE_NAME` passed exclusively via `jq --arg s` — no interpolation

**Findings**:

*Minor* — Curl `-F` metacharacter injection via file paths containing semicolons
Location: Phase 5: Algorithm Step 4
A path like `/tmp/file;type=application/x-sh` would override the MIME type of the uploaded part without triggering any current guard.

*Suggestion* — No test case for curl metacharacter injection (pairs with the guard fix)

---

### Usability

**Summary**: The pass-3 usability fixes are solid — Case 3 assertion, disambiguation rationale, and Step 8 success-message branching are all correctly specified. Two remaining major gaps: the confirmation-gate revision loop is unspecified (leaving `skill-creator` likely to omit it), and exit 22 is missing from the Step 9 table. One minor: exit 133 has no specific handler in the attach SKILL.md Step 3.

**Strengths**:
- Disambiguation flow end-to-end is now complete: table rendering, numeric validation, 3-attempt cap, loop-back rationale, abort message
- Three-way Step 8 success-message rendering covers STATE_NAME-known, `--transition-id`-only, and post-disambiguation cases
- `--describe` boundary note in Step 3 clearly distinguishes the two describe modes

**Findings**:

*Major* — Confirmation gate "n / revise" branch not specified for transition SKILL.md
*Major* — Exit 22 missing from transition SKILL.md Step 9 exit-code table

*Minor* — Exit 133 not handled in attach SKILL.md Step 3 error spec
*Minor* — EXIT_CODES.md exit 124 description understates its scope (three triggers, not one)
*Suggestion* — Disambiguation unparseable-stdout branch leaves user without recovery guidance (should abort immediately, not count against attempt limit)

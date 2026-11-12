---
type: "pr-review"
id: "181-review-1"
title: "[0338] Make linear update send only the fields it is given"
date: "2026-11-06T14:07:47+00:00"
author: "Phil Helm"
producer: "review-pr"
status: "complete"
target: "pr:181"
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "safety", "standards", "documentation"]
review_number: 1
pr_number: 181
tags: []
last_updated: "2026-11-06T14:07:47+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Code Review: #181 - [0338] Make linear update send only the fields it is given

**Verdict:** COMMENT

This PR fixes the description-erasing bug where it starts. `UpdateFields::input()` cannot send a field that was not given, `--state` now goes into the same single `issueUpdate`, and the tracker port's `update`/`retitle` are pinned to send byte-for-byte what they sent before. The logic is correct and the tests compare exact inputs. Most findings are about wording: the skill's new "empty value is cleared" rule holds only for `--description`, and the CHANGELOG and `--help` don't yet tell script authors what changed. The rest are small modelling refinements (`StateId`, one field enumeration, one `issueUpdate` path) that can follow later.

### Cross-Cutting Themes

- **Empty values only clear the description** (flagged by: documentation, safety, standards). The skill tells Claude to preview any empty field as cleared. Linear refuses an empty `--title` or `--assignee-id` (exit 34, as the PR's own live checks show), and a whitespace-only description erases the body without being shown as cleared.
- **The state UUID is protected only by a doc comment** (flagged by: architecture, code-quality, standards). `state_id: Option<&str>` relies on a comment to keep state names out. A `StateId` newtype returned by `resolve_state` would enforce that in the type and let the comment go.
- **Two `issueUpdate` paths** (flagged by: architecture, code-quality). `update.rs` and `transition.rs` each own an `issueUpdate` document with different failure handling, so `linear update --state` and `linear transition` now report the same failure differently.
- **The updatable fields are listed by hand three times** (flagged by: code-quality, standards). `is_empty`, `input` and `names_no_field` each enumerate the five fields, and the same rule has two names (`is_empty` and `names_no_field`).
- **Migration guidance for script authors** (flagged by: compatibility, documentation). The Breaking entry says "the shared rule" without explaining it, doesn't name the new exit codes, and doesn't point to `linear transition` as the unchanged JSON alternative. The Fixed entry doesn't mention that some calls which used to exit 0 can now exit 34.

### Tradeoff Analysis

- **Safety vs simplicity of `--description ""`**: Safety would like an explicit `--clear-description`, or a stderr notice, because an unset `$BODY` in a script silently erases the body. The current design (empty is sent as typed) is simple and documented in the skill, and the PR deliberately limited its scope. A stderr notice is the lowest-friction improvement.

### Strengths

- ✅ `UpdateFields::input()` keeps `Some("")` and drops `None`, which gives exactly the absent-versus-empty distinction the fix needs. An omitted flag can no longer become an empty overwrite, by construction.
- ✅ The tracker port's `update`/`retitle` send byte-for-byte what they sent before. Three exact pins in `contract_offline.rs` cover them, including the new titled-update pin, so `work sync`, `work update --push` and promotion are unchanged.
- ✅ `--state` is resolved from the catalogue before any request, using the same base-team resolution and the same 122/123 mapping as before. Combining it with the other flags makes the update all-or-nothing instead of silently dropping fields.
- ✅ This is a clean functional-core / imperative-shell split. It mirrors Jira's `UpdateFields` + `update_op`, and `update.rs` follows `transition.rs`'s layout, with `call_op`/`refuse_identifier_op` opened only to `pub(crate)`.
- ✅ The tests compare the whole `input` with `assert_eq!` against `json!`, not with `contains`, and every refusal path asserts `hits == 0`. Mutation analysis finds `is_empty`/`names_no_field` fully covered.
- ✅ The skill no longer claims that every failure left the issue unwritten. It separates the refusals made before any request from failures that might follow an applied write.
- ✅ `linear-client` is an adapter that is exempt from the cargo-public-api pin, and its only callers are in the repo, so removing `retitle_op` is safe.

### General Findings

- 🔵 **Safety**: The tracker port's `update` still always sends `description: Some(body)`. One latent path: `promotion.rs` builds that body with `split_frontmatter_and_body(..).unwrap_or_default()`, so if the split ever failed, the freshly created issue's body would be erased. This is unlikely and predates the PR; consider propagating the error in a follow-up.
- 🔵 **Code Quality**: `UpdateArgs::quiet` is parsed but never read by `run_update`. The new test `a_quiet_state_change_reports_the_updated_keyword` implies the flag has meaning. Either drop the flag or rename the test to say that it's accepted and ignored.
- 🔵 **Correctness**: `resolve_state` resolves against the base team, not the issue's team. This is unchanged, but now an invalid cross-team `stateId` also fails the title and description changes given alongside it.
- 🔵 **Standards**: Jira's CLI treats an empty `--assignee ""` as an explicit clear (`FieldEdit::Clear`). Linear sends `""` and Linear refuses it. That's a known gap ("no way to unassign"), but the two tracker CLIs now behave differently for the same input.

### Additional Findings

- 🔵 `cli/linear-cli/src/main.rs:308` — Identifier refusal runs after state resolution, so a bad state masks a refused identifier (correctness)
- 🔵 `cli/linear-client/src/update.rs:25` — `assignee_id` can't express "unassign" without a breaking type change (architecture)
- 🔵 `skills/integrations/linear/update-linear-issue/SKILL.md:66` — The preview doesn't show the description a non-empty `--description` replaces (safety)

## Inline Comments

### `skills/integrations/linear/update-linear-issue/SKILL.md:42-43` — "A field given an empty value is cleared" is true only for `--description` (Documentation, Safety)
**Severity**: major | **Confidence**: high | **Lens**: documentation, safety

🟡 **Documentation**

Step 2 tells the skill that *any* field given an empty value is cleared, and to preview it that way. That holds only for `--description`. The PR's own live check against Linear found that `--title ""` and `--assignee-id ""` are refused (exit 34, `linear update failed`) and the field is left unchanged. `--priority` can't be empty, and an empty `--state` fails catalogue resolution.

**Impact**: Someone trying to unassign with `--assignee-id ""` sees a preview promising the assignee will be cleared, then gets an unexplained `linear update failed`.

🔵 **Safety**

The rule also matches only a literally empty value. A whitespace-only `--description " "` erases the body just as completely, but wouldn't be shown as cleared at the confirmation step.

**Suggestion**: Narrow it to the field that actually clears. For example: "A `--description` that is empty or only whitespace erases the existing description; show it as `description: (cleared — the existing description is erased)`. Linear refuses an empty title or assignee, so warn that the update will fail."

---

### `CHANGELOG.md:796-800` — The Fixed entry doesn't say that some calls which used to succeed can now fail
**Severity**: minor | **Confidence**: high | **Lens**: compatibility

🔵 **Compatibility**

This entry reads as a pure fix, but some `linear update` calls that used to exit 0 can now exit 34. `--assignee-id` and `--priority` were parsed and never sent, so values Linear rejects (such as `--priority 9` or `--assignee-id ""`) used to be silently dropped while the command still succeeded. The PR's live testing shows `--assignee-id ""` now exits 34. In the same way, `--state X --title Y` used to drop the title and now writes it.

**Impact**: After upgrading, a script that passed these flags and got exit 0 can start failing, or start writing fields it never wrote before.

**Suggestion**: Add a sentence here or to the Breaking entry. For example: "Because `--assignee-id`, `--priority` and the flags given beside `--state` are now sent, an update Linear refuses for one of them now exits 34 where it used to succeed."

---

### `cli/linear-client/src/update.rs:30-37` — The updatable fields are listed by hand three times, and the rule has two names (Code Quality, Standards)
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, standards

🔵 **Code Quality**

The five updatable fields are enumerated in `UpdateFields::is_empty`, in `UpdateFields::input`, and in `UpdateArgs::names_no_field` (`cli.rs`). Adding a field (labels, project, estimate) means changing six places across two crates. If `is_empty` is missed, an update carrying only the new field is refused, and the compiler won't catch it.

**Suggestion**: Have `update_op` build the input once and refuse when it is empty, then drop `is_empty`. For example, make `input()` return `Map<String, Value>`, then `let input = fields.input(); if input.is_empty() { return Err(…) }`.

🔵 **Standards**

The same rule is named `names_no_field` in the CLI, which matches the error text ("an update must name at least one field") and the test names, and `is_empty` here, which is a generic collection-style name. Under strict DDD naming, use the domain term in both places, or let the CLI ask the client type.

---

### `cli/linear-client/src/update.rs:15-17` — Two parallel `issueUpdate` paths with different failure handling (Architecture, Code Quality)
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture**

The client now has two independent routes to `issueUpdate`, each with its own copy of the document:
- this `UPDATE` via `call_op` → `LinearFailure`;
- `transition.rs`'s `ISSUE_UPDATE` via `transport().send` + `interpret` → `SurfaceError`.

So `linear update X --state Done` and `linear transition X Done` make the same change but report and exit differently for the same failure (e.g. a bare 401). `transition.rs`'s module doc also still says it uses "the same `issueUpdate` mutation the port's `update` uses", which is no longer true.

🔵 **Code Quality**

The two documents differ only in that `ISSUE_UPDATE` also selects `state { name }`, and `LinearClient::transition` repeats the resolve-then-send-`stateId` sequence that `run_update` now does through `update_op`.

**Suggestion**: This shouldn't block the PR. At minimum, fix `transition.rs`'s module doc. As a follow-up, make `transition` delegate to `update_op` with `UpdateFields { state_id, ..default() }`, or share one document constant.

---

### `cli/linear-cli/src/main.rs:320` — `--state` failures no longer print the error detail they used to
**Severity**: minor | **Confidence**: medium | **Lens**: code-quality

🔵 **Code Quality**

A failed `linear update --state` used to print the `SurfaceError` detail. It now goes through the generic `linear update failed` path, which binds `failure` only to get the exit code and throws away the classified `detail` that `LinearFailure::Wire` carries. `resolve_requested_state` still prints a detailed error, so one command's two failure paths now report differently.

**Impact**: An operator whose `--state` update starts failing now sees one line with no cause, where before they saw the reason.

**Suggestion**: Keep the fixed prefix so the skill still matches it, and append the detail, e.g. `eprintln!("linear update failed: {detail}")`. If the bare message is deliberate (the PR lists it as unchanged), consider a follow-up covering `create` and `update` together.

---

### `cli/linear-cli/src/main.rs:315` — An empty `--description` erases the body, and only the skill guards against it
**Severity**: minor | **Confidence**: medium | **Lens**: safety

🔵 **Safety**

`description: args.description.as_deref()` sends `--description ""` as typed, which erases the body. That's intended, but the skill's preview is the only safeguard. Scripts and agents calling the CLI directly with `--description "$BODY"` and an unset variable will silently erase the issue's description, with exit 0 and `updated\t<KEY>`. That's the same data loss this PR fixes, reached through a different input.

**Suggestion**: Make the destructive effect visible. Either refuse an empty or whitespace-only `--description` unless a `--clear-description` flag is given, or at least print a stderr notice such as `description cleared`.

---

### `cli/linear-client/src/update.rs:23-24` — The state-UUID rule lives in a doc comment rather than the type (Architecture, Code Quality, Standards)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, code-quality, standards

🔵 **Architecture / Code Quality / Standards**

`state_id: Option<&str>` relies on "A UUID from `LinearClient::resolve_state`, never a state name" to keep human state names off the wire. `resolve_state` returns a plain `String`, so nothing ties the two together. A future caller of this public struct can pass `"In Progress"`. It compiles, and Linear rejects it at runtime as a generic exit 34. The comment is doing work the type should do, which the project's comment policy discourages.

**Suggestion**: Have `resolve_state` return a small `StateId` newtype and take `state_id: Option<&'a StateId>`. Resolution then becomes the only way to get a value the update accepts, and the doc comment can go.

---

### `cli/linear-cli/src/cli.rs:72-73` — `linear update --help` still describes none of its flags
**Severity**: minor | **Confidence**: medium | **Lens**: documentation

🔵 **Documentation**

`UpdateArgs`' fields have no `///` docs, unlike `CreateArgs` just above, so `update --help` (and `cli_surface.golden`) shows every option with no help text. This PR settles the semantics a direct caller most needs: an omitted flag leaves the field untouched, an empty `--description` erases the body, and `--state` takes a workflow-state name resolved through the catalogue. `--quiet` is listed but has no effect on `update`.

**Suggestion**: Add one-line `///` docs to match `CreateArgs`. For example, `--description`: "New Markdown description; an empty value erases it"; `--state`: "Workflow state name, resolved via the catalogue". Then refresh `cli_surface.golden`. Either document `--quiet` as having no effect on `update`, or drop it.

---

### `CHANGELOG.md:200-207` — The Breaking entry needs plainer migration guidance (Documentation, Compatibility)
**Severity**: minor | **Confidence**: medium | **Lens**: documentation, compatibility

🔵 **Documentation**

"An identifier the shared rule refuses" is internal jargon, so a changelog reader can't tell which identifiers it means. "Exits with the code a plain `linear update` would" doesn't name the codes. The 401 example says neither the old code nor the new one. Script authors are told to update their exit-code handling, but not what to update it to.

🔵 **Compatibility**

The entry points readers to `linear show` to read the new state. The closer replacement is `linear transition <ID> <STATE>`, which is unchanged and still prints the JSON envelope with `"outcome"` for a state-only change.

**Suggestion**: Replace "the shared rule" with an example (e.g. "a malformed identifier such as `#BLA-1`"). Name the exit codes or point to them. Add: "Scripts that need the JSON envelope for a state-only change can call `linear transition`, whose output is unchanged."

---

### `cli/linear-cli/tests/flow_update.rs:235-236` — The 401 equivalence test doesn't pin the expected exit code
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`a_failed_state_change_exits_as_a_plain_update_would` only checks that the `--state` code equals the plain code and isn't `Some(0)`. `assert_ne!(plain, Some(0))` also passes for `None` (a process killed by a signal), and if both paths drifted to the same wrong code, the test would still pass. The Breaking note describes a specific new code for this case, and nothing locks it in.

**Suggestion**: Also assert the specific exit code, e.g. `assert_eq!(plain, Some(i32::from(<expected code>)))`, so the test pins both the "same as a plain update" contract and the actual 401 mapping.

---

## Per-Lens Results

### Architecture

**Summary**: The structure is sound. Two narrow wire operations become one `UpdateFields` value and one `update_op`, mirroring Jira, so the CLI and the tracker port share one path to `issueUpdate`. Pure input assembly is separated from I/O, and state is resolved in the shell before any request. The remaining points are evolutionary: `linear transition` keeps a parallel `issueUpdate` path with a different failure taxonomy, and `state_id`/`assignee_id` carry rules and future needs their types don't express.

**Strengths**:
- `UpdateFields::input` is a pure function from the value to the GraphQL input; catalogue resolution happens in the imperative shell first.
- Folding `retitle_op` into `update_op` shrinks the public surface. Port `update`/`retitle` become thin adapters, guarded by the wire-input pins.
- It follows Jira's `mutation::UpdateFields` + `update_op(&ExternalId, &UpdateFields)` idiom.
- `update.rs` as its own `impl LinearClient` module matches the crate's per-operation split, and the widening stops at `pub(crate)`.
- The update is an absolute-value `issueUpdate`, so it is idempotent under retry. The skill rightly stops claiming that a failure always means no write landed.
- Every refusal is enforced before any request leaves, at both the CLI and library level.

**Comments**:
- `cli/linear-client/src/update.rs:15-17` (minor, high) — Two parallel `issueUpdate` paths with different failure taxonomies; `transition.rs`'s module doc is now stale. Suggest delegating `transition` to `update_op`, or at least sharing one document.
- `cli/linear-client/src/update.rs:23-24` (suggestion, medium) — The state-UUID rule lives in a doc comment; suggest a `StateId` newtype.
- `cli/linear-client/src/update.rs:25` (suggestion, medium) — `assignee_id: Option<&str>` can't express Linear's `assigneeId: null` unassign. Jira models this with `FieldEdit::{Set, Clear}`, and adding it later is a breaking type change on an exported struct with all-`pub` fields. Consider `Option<FieldEdit<'a>>` now, or record the deferral.

**General Findings**: none.

### Code Quality

**Summary**: A well-shaped change. Port methods and the CLI share one `update_op`, input construction is a small filter_map, and `run_update` is a flat sequence of guard clauses. The remaining points: the field list is hand-maintained in several places, the state UUID is a raw string, and `--state` failures lose detail.

**Strengths**:
- One way to build an `issueUpdate`. The `title: Option` / `body: &str` asymmetry behind the bug is gone.
- Absent flags can't be sent, by construction, while empty strings are still sent as typed.
- `run_update` reads as a straight line, and `resolve_requested_state` hides the Option/Result plumbing.
- `update.rs` mirrors `transition.rs`'s layout, and the widening stops at `pub(crate)`.
- The no-field refusal reuses the existing `Outcome::RequestInvalid` pre-send refusal.

**Comments**:
- `cli/linear-client/src/update.rs:30-37` (minor, high) — The field list is enumerated three times. Suggest refusing when `input()` builds an empty map, and dropping `is_empty`.
- `cli/linear-client/src/update.rs:23-24` (minor, medium) — The state UUID is a bare `&str` guarded only by a doc comment; suggest a `StateId` newtype.
- `cli/linear-cli/src/main.rs:320` (minor, medium) — `--state` failures lose the detail they used to print; suggest appending `LinearFailure`'s detail to the fixed prefix.
- `cli/linear-client/src/update.rs:15-17` (suggestion, medium) — Two near-identical `issueUpdate` documents in sibling modules; record a follow-up.

**General Findings**:
- (suggestion) `--quiet` on `linear update` is never read, and a new test name implies it has meaning. Drop the flag or rename the test to say it's accepted and ignored.

### Test Coverage

**Summary**: Strong coverage, in proportion to the risk. Client tests compare the exact `input` for every field alone, all together, and an empty description, plus the refusals with nothing sent. Flow tests run the real binary over both catalogue shapes and the 122/123 refusals. Port paths are pinned exactly. One loose assertion remains in the 401 equivalence test.

**Strengths**:
- The port's `update`/`retitle` each have exact `variables.input` pins (`retitle_sends_only_the_title`, `update_without_a_title_sends_only_the_description`, and the new `update_with_a_title_sends_the_title_and_description`).
- Assertions compare the whole `input` with `assert_eq!` against `json!`, replacing the old `contains("Renamed")`.
- Mutation analysis: `is_empty`/`names_no_field` are fully covered against dropped conjuncts and `&&`→`||`.
- Every refusal asserts `hits == 0`, and `sent_input` asserts `hits == 1`, which shows state resolution makes no extra request.
- `seed_ambiguous_catalogue` covers the 123 path offline.

**Comments**:
- `cli/linear-cli/tests/flow_update.rs:235-236` (minor, medium) — The 401 equivalence test doesn't pin the expected exit code, and `assert_ne!(.., Some(0))` passes for `None`.

**General Findings**: none.

### Correctness

**Summary**: The logic is sound. The port's `update`/`retitle` send byte-for-byte what they did before (same document, variables, and key order). `--state` resolves exactly as the old `transition` did (base-team `team_states()`, 122/123 mapping). The only ordering quirk: identifier refusal now runs after state resolution.

**Strengths**:
- `input` drops `None` but keeps `Some("")`, which is the exact absent-versus-empty distinction needed.
- The port reproduces the old `map_or_else` branches exactly, and the `UPDATE` document was carried over character-for-character.
- `resolve_state` is the same function the old `transition` called first, so the UUID and refusal mapping are unchanged and happen before any request.
- Folding `--state` into one `issueUpdate` makes the update all-or-nothing.
- `names_no_field` covers all five flags, and a state always resolves or refuses, so the library guard can't be reached from the CLI, with no gap between the two checks.
- Neither the old nor the new path checks `issueUpdate.success`, so moving to `call_op` loses no check.

**Comments**:
- `cli/linear-cli/src/main.rs:308-312` (suggestion, high) — A refused identifier plus a bad state reports 122, not the identifier refusal. Nothing is sent either way. Either check the identifier first, or pin the current precedence with a test.

**General Findings**:
- (suggestion) State resolution targets the base team, not the issue's team. This is unchanged, but a cross-team failure now also fails the other fields given with it, and is reported only as `linear update failed`.

### Compatibility

**Summary**: The deliberate break to `linear update --state` is under `[Unreleased] ### Breaking`, and the only in-repo consumer is updated. The `linear-client` API change is safe (an adapter exempt from cargo-public-api, with only in-repo callers). Small gaps: the Fixed entry doesn't mention calls that used to succeed and now fail, and it doesn't name `linear transition` as the JSON alternative.

**Strengths**:
- The Breaking entry names the output change, the exit-code change (with an example) and the earlier identifier refusal.
- Port wire input is unchanged and pinned.
- `linear-client` has no `public-api.txt` snapshot, and no out-of-tree callers were found.
- The 122/123 stderr lines and codes, the `update` keyword set, and `linear transition` are all unchanged.
- The `update-linear-issue` skill is updated in the same PR, and the docs-site reference is generated (gitignored).

**Comments**:
- `CHANGELOG.md:796-800` (minor, high) — The Fixed entry hides that some calls that used to succeed can now fail.
- `CHANGELOG.md:200-207` (suggestion, medium) — The Breaking entry could name `linear transition` as the way to keep JSON output.

**General Findings**: none.

### Safety

**Summary**: The data-loss bug is fixed at its source, `--state` can no longer drop flags or leave a partial write, and the skill's "no write was made" claims now match the code. Remaining risks: `--description ""` erases with no guard outside the skill, the skill's empty-value rule is inaccurate, and the port's `update` still always overwrites the description (a pre-existing behaviour).

**Strengths**:
- The root cause is removed structurally (`input()` includes only `Some`; `as_deref()` mapping).
- State resolution happens before the single request, so a 122/123 refusal sends nothing.
- `update_op` refuses an empty update before any request, which guards the library path too.
- Skill Step 4 limits "no write was made" to the three refusals that really happen before any request.
- The port is byte-for-byte unchanged, and the new pin guards it.

**Comments**:
- `cli/linear-cli/src/main.rs:315` (minor, medium) — An empty `--description` erases the body, and only the skill guards against it.
- `skills/integrations/linear/update-linear-issue/SKILL.md:42-43` (minor, medium) — The preview rule is too broad (title and assignee are refused, not cleared) and too narrow (whitespace-only erases too).
- `skills/integrations/linear/update-linear-issue/SKILL.md:66-68` (suggestion, low) — The preview doesn't show the description a non-empty `--description` replaces; consider fetching and showing it first.

**General Findings**:
- (minor) The tracker port's `update` always overwrites the remote description. `promotion.rs`'s retitle branch uses `split_frontmatter_and_body(..).unwrap_or_default()`, which could send an empty body if the split ever failed. This is unlikely and predates the PR.

### Standards

**Summary**: The PR closely follows project conventions: the `transition.rs` module layout, Jira's naming and signature, an alphabetised `lib.rs`, the `&Args` helper shape, and the CHANGELOG house style. Small issues: two names for one rule, and a doc comment standing in for a type.

**Strengths**:
- `update.rs` copies `transition.rs`'s pattern, and the `pub(crate)` widening matches existing practice.
- `UpdateFields<'a>`/`update_op` match Jira's names and signature, re-exported the same way.
- `lib.rs` stays alphabetised, and `tests/update.rs` is named after its module.
- `run_update(&UpdateArgs)` and `resolve_requested_state` match the binary's other helpers.
- The CHANGELOG Breaking and Fixed entries follow the house style.

**Comments**:
- `cli/linear-client/src/update.rs:30-37` (minor, medium) — The same concept is named `is_empty` here and `names_no_field` in the CLI.
- `cli/linear-client/src/update.rs:23-24` (suggestion, low) — The doc comment carries a rule a type could express.

**General Findings**:
- (suggestion) An empty value means "clear" in Jira but is sent as-is in Linear (`--assignee-id ""` is refused). This is a known gap, but the skill's wording should be narrowed meanwhile.

### Documentation

**Summary**: The documentation is well placed and mostly accurate. The CHANGELOG entries sit under the right `[Unreleased]` headings, and skill Step 4 describes the single success shape correctly, including the timeout caveat. Gaps: the Step 2 "empty value is cleared" rule holds only for `--description`, and `update --help` documents no flags.

**Strengths**:
- The Breaking and Fixed entries are correctly placed and follow the bold-lead style.
- The Breaking entry gives concrete migration guidance (`linear show`; scripts need updating).
- Skill Step 4's refusal-versus-failure distinction matches the code.
- Module docs in `update.rs`, `flow_update.rs` and `tests/update.rs` are accurate, and `# Errors` names the new refusal.
- The `state_id` rustdoc records the rule a caller could get wrong.

**Comments**:
- `skills/integrations/linear/update-linear-issue/SKILL.md:42-43` (major, high) — "A field given an empty value is cleared" is true only for `--description`.
- `CHANGELOG.md:203-207` (minor, medium) — The Breaking entry uses internal jargon ("the shared rule") and doesn't name the new exit codes.
- `cli/linear-cli/src/cli.rs:72-73` (minor, medium) — `linear update --help` still describes none of its flags, and `--quiet` has no effect.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*

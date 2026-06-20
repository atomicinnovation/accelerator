---
type: plan-review
id: "2026-06-20-0116-structured-stall-on-no-decision-input-review-1"
title: "Plan Review: Structured Stall on No Decision Input"
date: "2026-06-20T16:26:50+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-20-0116-structured-stall-on-no-decision-input"
target: "plan:2026-06-20-0116-structured-stall-on-no-decision-input"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, test-coverage, code-quality, architecture, usability, compatibility, safety]
review_number: 1
review_pass: 3
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-20T18:19:59+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Structured Stall on No Decision Input

**Verdict:** REVISE

This is a well-scoped, low-risk plan that is unusually well-grounded in the
actual code: every cited line, fixture, helper, and the validator message all
match, the core correctness claim (returning `2` only on the bare-fd-0 EOF
branch precisely isolates the no-input case) is provably exact, and the
non-regression strategy is anchored on substantive existing gates. The verdict
is REVISE — not because the design is wrong, but because four major findings
cluster on a single weak spot: **the stall message's resume command, which is
the entire point of the change, does not actually work as printed.** Two
structural issues make it un-copy-pasteable, a safety gap makes it potentially
destructive against the partially-mutated tree it fires on, and a coverage gap
leaves the legacy (non-stall) branch net-new untested.

### Cross-Cutting Themes

- **The resume command is the deliverable, and it is broken three ways**
  (flagged by: usability, safety, architecture, compatibility) — The plan's
  Overview and Desired End State both promise an "exact, copy-pasteable resume
  command." But the helper prefixes *every* line with `[$id]` (so the command
  pastes as `[0002-predicate] bash …`, not a command), splits the env-var form
  across a backslash-continuation that breaks across the prefixes, names a file
  that does not exist yet without a clear create-first cue, and advertises a
  one-step "resume" that is actually an N-round-trip breadcrumb against a tree
  the migration has already partially mutated. The single most valuable thing
  this change ships needs to be runnable verbatim.

- **`read_decision`'s contract is silently widened** (flagged by: code-quality,
  architecture) — The return contract goes from `{0,1}` to `{0,1,2}` with `2`
  now load-bearing, but the function's header comment
  (`interactive-lib.sh:236-237`) documents neither the old nor new semantics.
  Any future caller (notably 0117, which touches the same region) that treats
  any non-zero as a generic failure silently loses the stall.

- **A "test-only / never user-facing" seam becomes user-facing**
  (flagged by: architecture, compatibility) — Phase 2 prints
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` and Phase 1 wires a public
  `--decisions-file` flag onto it, while the source comment at
  `run-migrations.sh:14-18` still asserts it is "never documented in --help or
  any user-facing banner." The comment will contradict shipped behaviour.

- **The new branch's *other* arm is untested** (flagged by: test-coverage,
  safety) — Both new tests exercise only the `rc==2` (stall) arm. The legacy
  `rc!=2` "failed to obtain decision" arm and the `--skip`/`--unskip` recovery
  flags after the validation reorder rest on a "covered implicitly" claim and a
  manual-verification checkbox respectively — neither is actually gated.

### Tradeoff Analysis

- **80-column source rule vs copy-pasteable runtime output**: The usability lens
  argues the env-var resume line should be emitted as a single unwrapped line
  even past 80 columns, because the 80-col rule governs source width, not
  runtime stderr, and a runnable command outweighs visual wrapping. The
  compatibility/standards instinct is to wrap. Recommendation: favour the
  single-line runnable command — wrap the *source* `echo` if needed (string
  concatenation), not the emitted output.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Usability**: `[$id]` prefix on the resume-command lines breaks copy-paste
  **Location**: Phase 2, Section 2 (`emit_no_input_stall`)
  The helper prefixes every line — including the two resume-command lines — with
  `[$id]`, so a copied command reads `[0002-predicate]     bash …
  --decisions-file …`, which is not a runnable command. This contradicts the
  plan's own "copy-pasteable as-is" goal and Manual Verification item.

- 🟡 **Usability**: Backslash line-continuation in the env-var form is not
  copy-pasteable
  **Location**: Phase 2, Section 2 (env-var equivalent)
  The env-var form is split across two lines joined by a trailing `\`, but the
  next line begins with the literal `[$id]      ` prefix and the whole is wrapped
  in a `(equivalent: … )` parenthetical. Pasting yields `\` followed by
  `[0002-predicate]` and a stray `)` — not a valid continuation.

- 🟡 **Safety**: Printed resume command is unsafe against the already-mutated
  tree the stall fires on, with no warning
  **Location**: Phase 2, Section 2 / Desired End State item 3
  The stall fires *after* the migration has begun mutating the corpus (the
  PROMPT/VALIDATE_ERR abort sites do not `rm -f` resume-state or wait on the
  child). The printed "To resume" command will hit the clean-tree guard; an
  operator reaching for `ACCELERATOR_MIGRATE_FORCE=1` to get past it bypasses the
  dirty-tree protection and re-runs a partially-applied migration — risking
  double-application or divergence. No caveat is printed.

- 🟡 **Test Coverage**: Legacy `rc!=2` branch ("failed to obtain decision")
  becomes net-new untested
  **Location**: Phase 2, Sections 3-4 + Testing Strategy "Key edge cases"
  Both new tests exercise only the `==2` arm; the legacy `else` arm is exercised
  by no test. The decisions-file-exhausted path (the realistic way to reach it)
  is claimed "covered implicitly by existing tests that exhaust files on happy
  paths" — but those supply exactly enough decisions and exit 0, never reaching a
  post-exhaustion prompt. The claim is effectively vacuous; a branch-inversion
  mutation would pass both new tests.

#### Minor

- 🔵 **Architecture / Usability / Compatibility**: Resume command reads as a
  one-shot batch fix but is an N-round-trip breadcrumb pre-0117
  **Location**: Phase 2, Section 2 / Desired End State item 3
  "Write one decision per line … then re-run the driver" implies author-all-then-
  resume, but until 0117's `--list` lands the migration emits prompts one at a
  time with no accumulator, so the named file can answer only the current prompt;
  resuming yields the next stall (N round-trips). Wording should be softened and
  made forward-compatible with 0117's `list → decide → write → resume` flow (and
  the printed flag/path confirmed to match what 0117 will adopt) so it need not
  change twice.

- 🔵 **Code Quality / Architecture**: `read_decision` return contract widened to
  `{0,1,2}` but its header comment is not updated
  **Location**: Phase 2, Section 1
  The new `return 2` carries the load-bearing "no input channel" meaning, but the
  function header (`interactive-lib.sh:236-237`) documents only source selection
  and output globals. Add a one-line return-contract note (`0`=decided, `1`=read
  error / exhausted / TTY-EOF, `2`=no input channel) at the definition site.

- 🔵 **Architecture / Compatibility**: `run-migrations.sh:14-18` "never
  user-facing" comment is falsified by this change
  **Location**: Phase 1, Section 1
  Phase 1 wires a public flag onto the env var and Phase 2 prints it, so the
  source comment asserting it is "never documented … or any user-facing banner"
  becomes false. Update the comment within Phase 1 (full `--help` promotion can
  stay deferred to 0117) so source and behaviour stay coherent.

- 🔵 **Correctness / Compatibility**: `--decisions-file` cannot be combined with
  `--skip`/`--unskip` (single-`$1` flag block, not a loop)
  **Location**: Phase 1, Section 1
  The flag block is a one-shot `if [ $# -gt 0 ]; then case "$1"`. `--skip`/
  `--unskip` exit early; `--decisions-file` shifts and falls through, so it works
  only as the first arg and a second flag is silently ignored. Acceptable for the
  single-flag mitigation, but the constraint is undocumented — note it, or
  convert to a `while`/`shift` loop (0117 will add `--list`, forcing this anyway).

- 🔵 **Correctness**: Validation reorder lets `--skip`/`--unskip` bypass env-var
  validation
  **Location**: Phase 1, Section 1
  Moving the validation block after the flag block means `--skip`/`--unskip`
  (which `exit 0`) no longer trigger it, so `--skip <id>` with an invalid
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` (currently exit 1) now succeeds silently.
  Harmless in practice (skip ignores the file) but an unstated semantic change;
  note it or validate the flag-supplied path separately.

- 🔵 **Code Quality**: Near-duplicated `dec_rc`/branch/teardown block at both
  emit sites
  **Location**: Phase 2, Sections 3 and 4
  The nine-line read-status-branch-emit-teardown block is copied verbatim at both
  sites, differing only in the key variable and the legacy verb — the exact drift
  hazard 0116 is fixing. Consider a `read_decision_or_stall <id> <key> <verb>`
  wrapper so each site collapses to one line and the status semantics live in one
  place.

- 🔵 **Code Quality / Safety**: Hand-built decisions path diverges from the
  state-path convention and sits beside live state files
  **Location**: Phase 2, Section 2
  `migrations-${id}-decisions.txt` is a fourth ad-hoc `migrations-${id}-*` path
  with a brand-new suffix and no shared builder, placed in the same
  `.accelerator/state/` dir as the authoritative `-session.jsonl` /
  `-resume-state.tmp` / FIFOs. Steering a human to hand-author files there invites
  an off-by-one filename slip that could clobber the resumable artifact. Consider
  a `migration_state_path` helper and/or steering the authored file outside the
  runner-owned state dir.

- 🔵 **Test Coverage**: Phase 1 `--decisions-file` tests are prose-only; the
  parity test must assert the JSONL count
  **Location**: Phase 1, Success Criteria
  Unlike Phase 2, Phase 1's three tests are checklist prose with no code. The
  load-bearing parity test must capture and `assert_eq` the JSONL line count
  (`wc -l <"$LOG" | tr -d ' '`) from a `--decisions-file` run against the env-var
  run — an exit-0-only check would pass even if the flag silently failed to feed
  decisions through.

- 🔵 **Test Coverage**: VALIDATE_ERR test's "surfaced first" ordering is not
  actually verified
  **Location**: Phase 2, Section 6
  `assert_contains "validator message surfaced first"` only checks presence
  anywhere in the combined stream — not that the validator error precedes the
  stall, nor that exactly one VALIDATE_ERR occurred. Add a frame-count assertion
  (`VE_COUNT == 1`, as the existing AC-8 test does) to genuinely cover the
  prompt → validate-err → re-prompt → stall sequence.

- 🔵 **Safety**: `--skip`/`--unskip` non-regression after the reorder rests on a
  manual-only checkbox
  **Location**: Phase 1, Manual Verification
  These are the operator's recovery levers; a silent regression from the flag-
  block reorder would remove a safety control and not be caught by CI. Add a cheap
  automated assertion that `--skip <id>`/`--unskip <id>` still write/remove the
  skip-file entry and exit 0.

- 🔵 **Code Quality**: 14-line `echo` block repeats the `[$id]` prefix and embeds
  fragile continuation escaping
  **Location**: Phase 2, Section 2
  Fourteen separate `echo "[$id] …"` lines with a hand-managed `\` continuation
  are easy to break on edit (the Manual Verification "no escaping artifacts" item
  exists because of this). Consider piping a prefix-free body through
  `sed "s/^/[$id]   /"` (the idiom already at `:568`) so the prefix is applied
  once. (Resolving this together with the two major copy-paste findings is
  natural — the command lines should be prefix-free regardless.)

- 🔵 **Correctness**: `return 2` conflates a genuine read error with EOF
  **Location**: Phase 2, Section 1
  The bare-fd-0 `read … || return 2` cannot distinguish a true read error from
  EOF — both yield 2 and report "no decision input available." Benign for a
  non-TTY fd (which fails essentially only at EOF); just soften the plan's "only
  EOF raises 2" wording to acknowledge it.

- 🔵 **Test Coverage**: Exit-status assertion is loose (any non-zero passes)
  **Location**: Phase 2, Section 5
  `assert_neq … "0" "$RC"` passes for any non-zero exit, including a crash or
  `set -u` abort in the new plumbing. The `MIGRATION STALLED` marker assertion is
  the real exit-correctness gate; consider also asserting absence of
  `unbound variable` noise.

#### Suggestions

- 🔵 **Code Quality / Usability**: `--decisions-file` falls through while its
  siblings exit — add a comment
  **Location**: Phase 1, Section 1
  It is the only arm that doesn't `exit`, and (with `--help` deferred to 0117) the
  stall message is its only documentation. Add a one-line comment noting the
  deliberate fall-through vs the exiting siblings.

- 🔵 **Test Coverage**: TTY (`/dev/tty`) branch remains permanently untested
  **Location**: Testing Strategy, "Key edge cases"
  Correctly the pragmatic call (no fake TTY in CI), but record it as a documented
  known coverage hole so a future maintainer touching `read_decision` knows the
  channel-distinguishing arm is unguarded.

- 🔵 **Architecture**: Stall helper's global defensiveness is asymmetric
  **Location**: Phase 2, Section 2
  The helper guards `${RUNNER_SCRIPT_DIR:-.}` but uses `$PROJECT_ROOT` bare on the
  same failure path. Largely theoretical (`PROJECT_ROOT` is reliably exported),
  but either default both or document why only one is defended.

### Strengths

- ✅ The `return 2` isolation claim is provably exact: `interactive-lib.sh:262`
  sits inside the `else` of both the unset-env-var and non-TTY conditions, so `2`
  is structurally unreachable when a decisions file or TTY exists; piped data
  still succeeds, only EOF returns 2.
- ✅ `read_decision || dec_rc=$?` is `set -e`-safe (left of `||` suppresses
  `set -e`), and both emit-site conversions preserve `exec 7>&-; return 1`
  byte-for-byte, so FIFO teardown and non-zero propagation are unaffected.
- ✅ The plan is exceptionally well-grounded: every cited line, the
  `seed_predicate_sandbox` signature, the `0002-predicate` fixture's key-first
  field order, and the `empty value not allowed` reject all match the source.
- ✅ The plan correctly defuses the real flakiness hazard for the PROMPT test
  (`</dev/null` to force `! [ -t 0 ]` + immediate EOF rather than relying on the
  ambient TTY of `$(…)`), and the VALIDATE_ERR test reuses the proven AC-8
  pattern.
- ✅ AC3 non-regression is anchored on substantive gates (exit-0 + exact JSONL
  counts at `:378-666`, the 5-run byte-identical determinism gate at
  `:1069-1111`), and no existing test asserts on the old baseline strings, so the
  conversion breaks nothing.
- ✅ The two-phase split is coherent: Phase 1's `--decisions-file` flag is
  genuinely standalone-valuable and mergeable; Phase 2 depends on it cleanly.
- ✅ The helper takes id+key as parameters, correctly accommodating the two sites
  naming the key through different variables (`p_key` vs `LAST_PROMPT_KEY`).

### Recommended Changes

1. **Make the resume command actually copy-pasteable** (addresses: `[$id]`
   prefix breaks copy-paste; backslash continuation not copy-pasteable; 14-line
   echo block fragility). Emit the `bash … --decisions-file …` line and the
   env-var equivalent flush-left (no `[$id]` prefix), each as a single unwrapped
   line, dropping the `\`-continuation and the `(equivalent: … )` wrapper. Keep
   `[$id]` on the diagnostic/explanatory lines for log grep-ability. The existing
   pre-flight hint (`run-migrations.sh:111-132`) already prints unprefixed
   commands — mirror that. Update the Manual Verification step to actually paste
   and run the emitted line.

2. **Warn that the tree is partially mutated before advertising "resume"**
   (addresses: unsafe resume against mutated tree). Add a line to the stall
   stating the migration may have partially mutated the working tree, that the
   operator should inspect/commit (or revert) before resuming, and that
   partial-run resume safety is owned by sibling 0119 — so the breadcrumb does not
   read as a safe one-step resume.

3. **Close the legacy-branch coverage gap** (addresses: legacy `rc!=2` branch
   untested; `--skip`/`--unskip` manual-only). Add a test that exhausts a
   decisions file mid-run (N-1 decisions for N prompts) asserting the output
   contains `decisions file exhausted` and `failed to obtain` but NOT
   `MIGRATION STALLED`; and add a cheap automated `--skip`/`--unskip` round-trip
   assertion after the reorder.

4. **Soften and forward-proof the resume wording** (addresses: one-shot vs
   N-round-trip; 0117 forward-compat). Clarify that the file answers the current
   prompt and another stall may follow, and confirm the printed flag spelling and
   `migrations-<id>-decisions.txt` path match what 0117 will adopt so the
   consumer-facing contract is introduced once.

5. **Document the widened contracts and stale comment** (addresses:
   `read_decision` contract; "never user-facing" comment). Add the `{0,1,2}`
   return-contract note to `read_decision`'s header, and update
   `run-migrations.sh:14-18` to reflect that the env var / `--decisions-file` flag
   is now referenced by the stall.

6. **Spell out the Phase 1 parity test and tighten the VALIDATE_ERR assertion**
   (addresses: Phase 1 prose-only tests; "surfaced first" ordering). Make the
   parity test `assert_eq` the JSONL line count against the env-var run, and add a
   `VALIDATE_ERR`-count assertion to the section-6 test.

7. **(Optional) Fold the duplicated emit-site block and the path construction**
   (addresses: near-duplicated block; hand-built path divergence). A
   `read_decision_or_stall` wrapper and a `migration_state_path` helper would
   collapse both sites and give the state-path convention one home — naturally
   done alongside change 1.

## Per-Lens Results

### Correctness

**Summary**: The plan's central correctness claim — that returning 2 only on the
bare-fd-0 EOF branch precisely isolates the no-input case — is verified against
the source: the branch is doubly guarded by the unset-env-var and non-TTY
conditions, so a decisions file or TTY can never raise 2. The set-e safety of
`read_decision || dec_rc=$?`, the preservation of `exec 7>&-; return 1`, and both
test premises hold against the actual code and fixture. The only genuine
correctness wrinkles are a behavioural change from reordering env-var validation
below the early-exiting `--skip`/`--unskip` block, and a minor conflation of
read-error-vs-EOF that is benign in the non-TTY domain.

**Strengths**:
- The `return 2` isolation claim is provably exact (interactive-lib.sh:262 sits
  inside the `else` of both guards); piped data succeeds, only EOF returns 2.
- `read_decision || dec_rc=$?` is set-e-safe; capturing into a pre-initialised
  `dec_rc=0` is correct.
- Both emit-site conversions preserve the existing teardown on every branch.
- The VALIDATE_ERR test premise is sound (one piped `edit ` → empty → reject →
  re-prompt → EOF → return 2).
- Key-source asymmetry correctly handled via parameters (`p_key` vs
  `LAST_PROMPT_KEY`).

**Findings**:
- 🔵 minor / high — Reorder validation below flag parsing (Phase 1, Change 1):
  `--skip`/`--unskip` `exit 0` before the relocated validation, so a skip with an
  invalid `ACCELERATOR_MIGRATE_DECISIONS_FILE` (currently exit 1) now succeeds
  silently. Harmless but an unstated semantic change; note it or validate the
  flag-supplied path separately.
- 🔵 minor / medium — `--decisions-file` single-shot case (Phase 1, Change 1):
  one non-looping `case "$1"`, so the flag works only as the first arg and cannot
  combine with another flag. State the constraint or use a `while`/`shift` loop.
- 🔵 minor / low — read-error vs EOF conflation (Phase 2, Change 1): `read … ||
  return 2` cannot distinguish a true read error from EOF; benign for a non-TTY
  fd. Soften the "only EOF raises 2" wording.

### Test Coverage

**Summary**: The plan is unusually well-grounded in the existing test
infrastructure — `seed_predicate_sandbox`, `setup_sandbox`, the `0002-predicate`
fixture, the `empty value not allowed` reject, and the assert helpers all exist
and match exactly. The two new Phase 2 tests reliably reach their target emit
sites and assert behaviour rather than implementation. The principal gaps are
negative-coverage gaps introduced by the new branch: the legacy `rc!=2` arm and
the decisions-file-exhausted path are claimed "covered implicitly" but have no
exercising test.

**Strengths**:
- The two new tests assert observable behaviour (marker, named key, resume
  substrings, id-in-path) plus absence of the old baseline string.
- Test-infra reuse is verified-correct against the source.
- Correctly defuses the PROMPT-test flakiness hazard with `</dev/null`.
- VALIDATE_ERR test reuses the proven AC-8 pattern.
- AC3 non-regression anchored on substantive gates (exit-0 + JSONL counts;
  5-run determinism gate).

**Findings**:
- 🟡 major / high — Legacy `rc!=2` branch becomes net-new untested (Phase 2,
  Sections 3-4): both new tests cover only `==2`; "covered implicitly by existing
  tests that exhaust files on happy paths" is vacuous because those exit 0 before
  reaching a post-exhaustion prompt. Add a mid-run exhaustion test asserting
  `decisions file exhausted` + `failed to obtain` but NOT `MIGRATION STALLED`.
- 🔵 minor / high — Phase 1 tests prose-only (Phase 1, Success Criteria): the
  parity test must `assert_eq` the JSONL line count, not just exit 0.
- 🔵 minor / medium — Loose exit assertion (Phase 2, Section 5): `assert_neq …
  "0"` passes for any non-zero; the marker assertion is the real gate. Consider
  asserting absence of `unbound variable`.
- 🔵 minor / high — "surfaced first" not actually ordered (Phase 2, Section 6):
  `assert_contains` is presence-only; add a `VALIDATE_ERR`-count assertion.
- 🔵 suggestion / high — TTY branch permanently untested (Testing Strategy):
  acceptable pragmatic call; record as a documented known coverage hole.

### Code Quality

**Summary**: A well-scoped, low-risk re-messaging that preserves the established
teardown idiom and follows the existing per-line `[$id]` stderr precedent. The
main maintainability concern is the near-identical block copied at both emit
sites; secondary concerns are the undocumented magic return code 2, the
hand-built path diverging from the existing naming convention, and the fragile
14-line echo block.

**Strengths**:
- Correctly a re-messaging of an existing halt; preserves `exec 7>&-; return 1`.
- The helper takes id+key as parameters — right call given the two key variables.
- Routes to stderr with `[$id]` prefixing, consistent with `:566-572`.
- The distinct `2` is raised at exactly one branch; other paths unchanged.
- Explicit about bash 3.2 / ASCII-only constraints.

**Findings**:
- 🔵 minor / high — Near-duplicated block at both emit sites (Phase 2, Sections
  3-4): fold into a `read_decision_or_stall` wrapper.
- 🔵 minor / high — Magic `2` not documented in `read_decision`'s contract
  (Phase 2, Section 1): add a return-contract header note.
- 🔵 minor / medium — Hand-built path invents a naming convention diverging from
  existing state paths (Phase 2, Section 2): add a `migration_state_path` helper
  or pass the path in.
- 🔵 minor / medium — 14-line echo block repeats prefix + fragile continuation
  (Phase 2, Section 2): pipe a prefix-free body through `sed "s/^/[$id]   /"`.
- 🔵 suggestion / medium — `--decisions-file` falls through vs exiting siblings
  (Phase 1, Section 1): add a comment noting the deliberate fall-through.

### Architecture

**Summary**: Architecturally sound and well-scoped: the stall preserves exit
semantics, the no-input signal is hoisted to the single precise detection point,
and the parameterised helper accommodates the two emit sites. The two-phase
split is coherent and Phase 1 is independently valuable. The main tension is the
deliberate 0116↔0117 seam — introducing `--decisions-file` and surfacing a
test-only env var ahead of 0117's formal promotion — which is defensible but
leaves a documentation/contract gap and a forward-compat risk in the printed
resume command.

**Strengths**:
- Stall preserves exit semantics exactly — a message-layer-only footprint.
- Hoisting the no-input signal into `read_decision` centralises detection at the
  one unambiguous point.
- Helper takes id+key as parameters (good cohesion/reuse).
- Phase 1's flag is genuine standalone value; the validation relocation gives a
  single validation site.
- Two-phase ordering is justified.

**Findings**:
- 🔵 minor / high — Resume command embeds a single-line assumption but 0117 makes
  it positional/N-line (Phase 2, Change 2 vs 0117 AC1-2): soften wording / add a
  forward-reference to `--list` so it need not change twice.
- 🔵 minor / high — `read_decision` contract widened to `{0,1,2}` but header not
  updated (Phase 2, Change 1): add the contract note at the definition site.
- 🔵 minor / medium — 0116 surfaces a test-only env var and falsifies the
  `run-migrations.sh:14-18` comment (Implementation Approach / What We're NOT
  Doing): update the comment within 0116's boundary.
- 🔵 suggestion / medium — Asymmetric global defensiveness (Phase 2, Change 2):
  `RUNNER_SCRIPT_DIR` defended, `PROJECT_ROOT` not; default both or document why.

### Usability

**Summary**: The plan delivers a genuinely actionable improvement (named key,
inline accept|skip|edit format, a first-class `--decisions-file` switch). But for
the primary consumer — a no-TTY Claude Code agent, plus humans debugging — the
stall block has two concrete copy-paste hazards: every line is prefixed with
`[$id]`, and the env-var equivalent spans two lines joined by a trailing
backslash. Both break a naive copy-paste of the very commands the message tells
the reader to run.

**Strengths**:
- Promoting the hidden env var to a first-class `--decisions-file` switch is a
  strong DX move.
- Explains the decisions-file format inline so an agent can author it.
- Stable machine-detectable marker for agent parsing.
- Consistent error experience between env-var and flag entry points.
- Migration id as a literal substring makes the message self-locating.

**Findings**:
- 🟡 major / high — `[$id]` prefix breaks copy-paste of the resume command
  (Phase 2, Section 2): emit the command line flush-left.
- 🟡 major / high — Backslash continuation across prefixed lines not
  copy-pasteable (Phase 2, Section 2): single unwrapped env-var line; drop the
  `\` and parentheses.
- 🔵 minor / medium — Resume command implies one-shot but is N-round-trip
  pre-0117 (Desired End State item 3): add a scope-clarifying line.
- 🔵 minor / medium — Resume path names a non-existent file with no create-first
  cue (Phase 2, Section 2): tie the "write decisions to" instruction visually to
  the command line.
- 🔵 suggestion / medium — `--decisions-file` behaves differently from siblings
  (Phase 1): add a code comment given `--help` is deferred.

### Compatibility

**Summary**: Well-scoped and largely additive: the new status 2 has exactly two
callers (both converted), so the widened contract introduces no third-caller
risk. All snippets are bash-3.2-clean and pass the bashisms denylist, and the
detection point is precisely isolated so existing decisions-file and TTY
behaviour is unchanged. The principal concerns are forward-compatibility of the
printed resume command with 0117's `--list` flow and the new flag's interaction
with the single-`$1` flag block.

**Strengths**:
- `read_decision` has exactly two callers, both converted — no third-caller
  misread (verified via grep).
- Only the no-input branch returns 2; exhausted/TTY paths unchanged.
- All snippets bash-3.2-clean; `IFS= read -r line || return 2` is portable.
- `--decisions-file` is additive; the runner globs pending migrations rather than
  selecting by `$1`, so `shift 2` + fall-through is safe.
- No existing test asserts the old baseline strings.

**Findings**:
- 🔵 minor / medium — Resume command should be forward-compatible with 0117's
  `--list` flow (Phase 2, Section 2 / Overview): confirm flag name + path are the
  exact forms 0117 will adopt.
- 🔵 minor / high — `--decisions-file` cannot combine with `--skip`/`--unskip`
  (Phase 1, Section 1): document the mutual exclusivity or use a `while` loop;
  verify the Phase 1 test invokes the flag alone.
- 🔵 minor / high — Makes a documented test-only env var user-facing (Phase 1 /
  `run-migrations.sh:14-18`): update the stale comment.

### Safety

**Summary**: A low-blast-radius change to a developer-only migration tool that
re-messages an existing non-zero halt and adds a thin alias for an
already-validated env var. It preserves the `exec 7>&-; return 1` teardown and
does not touch corpus-mutation, state-write, or clean-tree-guard logic, so it
introduces no new data-loss vector of its own. The principal residual concern is
in the breadcrumb it prints: the advertised resume command is not a safe
self-sufficient resume against a tree the migration has already partially
mutated, and the plan does not warn the operator.

**Strengths**:
- Re-messaging only — both emit sites keep `exec 7>&-; return 1` byte-for-byte;
  no new orphaned-process / hung-FIFO risk.
- `return 2` raised only on the bare-fd-0 EOF branch; fails safe (still halts
  non-zero either way).
- Phase 1 relocation keeps fail-closed validation for both supply routes before
  any migration forks.
- Does not alter when/whether the corpus is mutated, nor the clean-tree guard or
  `FORCE` semantics.
- The suggested decisions path is a distinct filename and is neither created nor
  read unless supplied.

**Findings**:
- 🟡 major / high — Printed resume command is unsafe against the already-mutated
  tree, with no warning (Phase 2, Section 2 / Desired End State item 3): an
  operator using `ACCELERATOR_MIGRATE_FORCE=1` to clear the clean-tree guard would
  re-run a partially-applied migration. Add a partial-mutation warning and a
  pointer to 0119.
- 🔵 minor / medium — Suggested decisions path sits beside live state files
  (Phase 2, Section 2 / Migration Notes): an off-by-one filename slip could
  clobber the session log / resume-state. Steer the file outside
  `.accelerator/state/` or warn it must not overwrite `migrations-<id>-*`.
- 🔵 minor / high — `--skip`/`--unskip` non-regression rests on a manual-only
  checkbox (Phase 1, Manual Verification): these are recovery levers; add a cheap
  automated round-trip assertion.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-20T17:13:26+00:00

**Verdict:** REVISE

The revision cleanly resolved all four prior major findings in substance — the
stall command is now copy-pasteable, the legacy branch is genuinely tested, and
the partial-mutation warning landed. But re-running all seven lenses against the
revised plan surfaced **three new major findings**, the most important of which
is that the cross-work-item coordination notes added in the revision rest on a
false premise: **0117 does not promote `--decisions-file`** — its AC4 promotes
the env var (`ACCELERATOR_MIGRATE_DECISIONS_FILE`) into `--help` and adds a
`--list` flag, with no `--decisions-file` flag anywhere. The plan makes
`--decisions-file` the *primary* resume form, diverging from the contract 0117
actually builds. Combined with a safety dead-end (the printed resume command is
refused by the clean-tree guard on the dirty tree the stall says is likely) and
the flag's absence from `--help`, the plan needs one more revision pass.

### Previously Identified Issues

**Major (all four resolved in substance):**
- 🟡 **Usability** — `[$id]` prefix breaks copy-paste — **Resolved**. Command
  lines now emitted flush-left; confirmed by the usability re-review as a
  strength.
- 🟡 **Usability** — backslash line-continuation not copy-pasteable —
  **Resolved**. Env-var form is now a single unwrapped line.
- 🟡 **Safety** — resume unsafe vs partially-mutated tree, no warning —
  **Partially resolved**. The inspect-or-revert warning + 0119 pointer landed and
  resolves the "no warning" gap, but the re-review surfaced a sharper related
  issue (the printed command is itself refused by the clean-tree guard — see New
  Issues N3).
- 🟡 **Test Coverage** — legacy `rc!=2` branch untested — **Resolved**. Test 7
  (decisions-file-exhausted → legacy abort, not stall) genuinely pins the arm.

**Minor / suggestion (resolved unless noted):**
- 🔵 Resume one-shot vs N-round-trip wording (arch/usability/compat) —
  **Resolved** (softened); the compat angle is now reframed as New Issue N1.
- 🔵 `read_decision` `{0,1,2}` contract undocumented — **Resolved** (header note).
- 🔵 Stale "never user-facing" comment — **Resolved** (Phase 1 instructs update).
- 🔵 `--decisions-file` can't combine with `--skip`/`--unskip` — **Resolved**
  (documented; reiterated as accepted this pass).
- 🔵 Validation reorder bypass — **Resolved** (documented); the re-review notes
  the deliberately-changed arm itself isn't yet tested (New, minor).
- 🔵 Duplicated emit-site block — **Resolved** (`read_decision_or_stall` wrapper).
- 🔵 Hand-built path convention / clobber-adjacency — **Resolved** (guard note);
  safety re-review keeps a residual minor (orphaned FIFOs in same dir).
- 🔵 Phase 1 prose-only tests — **Resolved** (spelled out, JSONL-count parity).
- 🔵 VALIDATE_ERR ordering assertion — **Resolved** (VE_COUNT added); re-review
  suggests frame-count is still stronger (New, minor).
- 🔵 `--skip`/`--unskip` manual-only — **Resolved** (automated test 5).
- 🔵 14-line echo block fragility — **Partially resolved**; still a ~20-line
  echo-wall, reiterated as minor by code-quality.
- 🔵 read-error vs EOF wording — **Resolved** (softened); correctness re-raised a
  distinct adjacent edge case (New, minor — non-newline-terminated line).
- 🔵 Loose exit assertion — **Resolved** (`unbound variable` guard).
- 🔵 Suggestions (fall-through comment, TTY known-gap, global defensiveness) — all
  **Resolved**.

### New Issues Introduced

#### Major

- 🟡 **Compatibility**: 0117 does not promote `--decisions-file`, so the
  "introduce the contract once" coordination is unfounded
  **Location**: Migration Notes (Coordinate with 0117) / Phase 1, Section 1
  Verified against `meta/work/0117-…md`: AC4 requires the **env var** name in
  `--help` and adds a `--list` flag; no `--decisions-file` flag is mentioned. The
  plan makes `--decisions-file` the *primary* resume form and asks to "confirm the
  flag spelling 0117 will promote" — but 0117 promotes the env var. The
  consumer-facing contract risks being introduced twice. **Resolve before merge:**
  either update 0117 to promote `--decisions-file` as canonical (env var as
  equivalent), or make the env-var form primary in the 0116 stall to match what
  0117 actually builds; and correct the coordination notes either way.

- 🟡 **Usability**: resume command points at a `--decisions-file` flag absent from
  `--help`
  **Location**: Phase 1, Section 1 + What We're NOT Doing
  The stall's primary command tells the consumer to run `--decisions-file`, but
  the plan defers adding it to `--help` to 0117 (which, per N1, won't add the flag
  at all). A consumer cannot confirm the flag the error told them to use. Add a
  one-line `--decisions-file <path>` entry to the runner's usage/help now (full
  contract docs can still wait), at near-zero cost.

- 🟡 **Safety**: advertised resume command is refused by the clean-tree guard
  after a partial mutation, nudging toward unsafe `FORCE`
  **Location**: Phase 2, Section 2 (resume command) — interacts with
  `run-migrations.sh:67-141`
  The stall fires after the tree may be partially mutated, but the printed
  `bash run-migrations.sh --decisions-file <path>` does not set
  `ACCELERATOR_MIGRATE_FORCE`. On the common first-prompt stall (empty/no session
  log), re-running hits the generic dirty-tree error whose only remedy message is
  to set `FORCE=1` — disabling the very safeguard protecting the inconsistent
  partial state. Make the revert-to-clean-then-resume ordering explicit in the
  stall text; do not bake `FORCE` into the printed command. (The
  inspect-or-revert warning already added is a partial mitigation.)

#### Minor / suggestion (new)

- 🔵 **Correctness** (minor): on the bare-fd-0 branch, a final stdin line with
  data but no trailing newline makes `read` return non-zero → `return 2`,
  misclassifying a real decision as no-input. Guard on `[ -z "$line" ]` as well as
  read status. (Plus suggestions: a genuine read *error* also raises the stall;
  initialise `LAST_PROMPT_KEY=` for a provable `set -u` guarantee.)
- 🔵 **Code Quality** (minor, high conf): two snippet lines exceed 80 columns (the
  `decisions_path=` assignment ~88 cols and a comment ~81 cols) — wrap before
  implementing. Plus: the ~20-line echo-wall could use a local `d()` prefix helper;
  the `verb` param is cosmetic; the `${RUNNER_SCRIPT_DIR:-.}` default contradicts
  its own "never taken" comment.
- 🔵 **Test Coverage** (minor): prefer counting `VALIDATE_ERR` frames against the
  migration log over grepping the message string (test 6); add content-level
  parity (byte-identical logs) to test 2; pin the full
  `migrations-0002-predicate-decisions.txt` path token (not just the id) in test
  5; the deliberately-changed "`--skip` with invalid env var now succeeds" arm is
  itself untested; tighten test 7 to `failed to obtain decision for k2`.
- 🔵 **Architecture** (minor): `read_decision_or_stall` abstracts only the failure
  half — `DECIDE_OUTCOME`/`DECIDE_VALUE` still flow through globals the caller
  reads; note this in the wrapper header. The 0117 forward-compat rests on a note,
  not a structural guarantee (overlaps N1).
- 🔵 **Usability** (minor): two resume forms presented with no stated preference;
  the create-this-file guard names a glob that may raise doubt rather than
  reassure; long flush-left commands may wrap in narrow terminals (verify at 80
  cols).
- 🔵 **Safety** (minor): the early-return leaves orphaned FIFOs / resume-state in
  the same dir the operator is told to author the decisions file in; consider
  `rm`-ing the FIFOs on the stall path. (Suggestion: add an automated end-to-end
  test that the printed resume command actually completes a stalled migration.)

### Assessment

The revision was successful on its own terms — every prior major was addressed
and the plan is materially better. The new findings are not regressions in the
edits themselves but issues the deeper re-read exposed, clustered on one root
cause: **0116 invents a `--decisions-file` flag and elevates it to the primary,
user-facing resume contract, but the sibling item meant to formalise that
contract (0117) standardises the env var instead.** That mismatch drives the
compatibility major (unfounded coordination), the usability major (flag not in
`--help`), and colours the safety major (the command it tells users to run is
refused on a dirty tree). The recommended next step is a focused revision that
(1) reconciles the primary resume form with what 0117 actually promotes, (2) adds
a minimal `--help` entry, and (3) makes the revert-first ordering explicit — then
a final re-review of those three areas. The minors (especially the 80-col
violations and the non-newline-terminated stdin edge) are quick fixes.

## Re-Review (Pass 3) — 2026-06-20T18:19:59+00:00

**Verdict:** APPROVE

All three pass-2 majors are resolved or acceptably scoped, **correctness found
zero issues**, and no new majors or criticals surfaced. The plan is sound and
ready for implementation. Two concrete pass-3 minors were fixed inline during
this pass (three echo lines still over 80 columns; the new emptiness-guard
fall-through was unexercised). The remaining items are explicitly-accepted
cross-item tradeoffs (deferred to 0117/0119) or optional polish.

### Previously Identified Issues (pass-2 majors)

- 🟡 **Compatibility** — 0117 doesn't promote `--decisions-file` —
  **Resolved**. Contract-ownership notes corrected and verified against 0117 AC4;
  0116 now owns the flag and self-documents it in `--help`, with no false 0117
  dependency. Confirmed by the compatibility re-review.
- 🟡 **Usability** — flag absent from `--help` — **Resolved**. A minimal
  `--help`/`-h` handler now lists all three flags; Phase 1 test 6 pins it.
- 🟡 **Safety** — resume command refused by clean-tree guard / FORCE nudge —
  **Acceptably deferred**. The safety re-review confirms 0116 introduces no new,
  non-deferrable data-loss risk: the stall mutates nothing, never advertises
  `FORCE`, and carries the inspect/revert warning + 0119 pointer; the real fix is
  correctly owned by 0119.
- 🔵 **Correctness** — non-newline-terminated stdin line misclassified —
  **Resolved**. The `if ! read ... && [ -z "$line" ]` guard was verified sound
  across all cases (including the blank-line trap) by the correctness re-review,
  which returned **no findings**.
- 🔵 **Code Quality** — 80-col violations — **Resolved this pass** (see below).

### New Issues Introduced

None of major or higher. All pass-3 findings are minor/suggestion:

#### Fixed inline during pass 3
- 🔵 **Code Quality**: three `echo` lines in `emit_no_input_stall` (the
  partial-tree warning and the resume steps) were still 81-82 columns — the
  pass-2 `state_dir` fix only shortened the path-bearing lines. **Fixed**: reworded
  across the existing continuation lines so all are ≤80.
- 🔵 **Test Coverage**: the new emptiness-guard fall-through (an unterminated
  final decision line is parsed, not stalled) was unexercised. **Fixed**: added
  Phase 2 test 8 (`printf 'accept'` with no newline → exit 0, one JSONL record, no
  stall).

#### Accepted as-is / deferred (no change)
- 🔵 **Safety** (minor): clean-tree-guard refusal of the printed resume command —
  deferred to 0119 by author decision; interim warning is adequate.
- 🔵 **Compatibility / Architecture** (minor): the single-leading-flag `if`/`case`
  is acknowledged and its `while`/`shift` conversion is owned by 0117.
- 🔵 **Architecture** (minor x2): `--help` and the `migrations-<id>-decisions.txt`
  path are surfaces co-owned with 0117 — suggestion to add a co-presence AC / shared
  path constant in 0117; recorded for the 0117 plan, not blocking 0116.
- 🔵 **Usability** (suggestion): `-h` alias untested; `--help` omits the env-var
  form; decision-grammar string duplicated in stall + `--help`. Optional polish.
- 🔵 **Code Quality** (suggestion): shell width is convention-only (not CI-enforced
  — shfmt/ShellCheck/bashisms have no length rule), so the implementer must keep
  ≤80 by hand; the ~20-line echo-wall is a defensible style tradeoff.
- 🔵 **Test Coverage** (suggestion): test 6 could also assert `--skip`/`--unskip`
  appear in `--help`.

### Assessment

The plan is in strong shape and approved. The three-pass arc resolved a genuine
contract-ownership mismatch between 0116 and 0117, hardened the test coverage
(legacy arm, JSONL-count parity, VALIDATE_ERR multiplicity, the unterminated-line
fall-through), made the stall command genuinely copy-pasteable and discoverable
via `--help`, and kept the change a faithful re-messaging of an existing halt.
The residual notes are optional polish or work explicitly owned by sibling items
0117 (interface promotion) and 0119 (partial-run resume safety); none blocks
implementation. Recommend proceeding to `/implement-plan`, carrying the two
cross-item suggestions (a `--help` co-presence AC and a shared decisions-file
path constant) into the 0117 plan.

---
type: plan-review
id: "2026-06-21-0117-agent-decisions-bridge-and-invoker-contract-review-1"
title: "Plan Review: Agent-Decisions Bridge and Documented Invoker Contract"
date: "2026-06-21T08:10:27+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-06-21-0117-agent-decisions-bridge-and-invoker-contract"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, correctness, code-quality, test-coverage, safety, portability, compatibility, usability]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-06-22T13:53:23+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Agent-Decisions Bridge and Documented Invoker Contract

**Verdict:** REVISE

The plan's core design is sound and well-argued: a child-side `--list` dry-emit
that dumps the already-buffered, predicate-filtered `TX_LINES` (rather than a
runner that feeds skips) is the correct boundary choice, the wire-protocol
extension is genuinely additive and backward-compatible (verified against
`read_frame`), AC1–AC6 traceability is excellent, and the fail-closed validator
is correctly placed before the apply loop. The reason for REVISE is a single
correctness defect plus a set of reinforcing themes rather than a flawed
approach: the count==N validator can accept a decisions file that under-feeds at
apply time (leaving a partially-mutated corpus), and four cross-cutting concerns
— logic duplicated from single-source-of-truth code, an unspecified/untested awk
fixture insert, `--list` escaping at the agent boundary, and the scope of the
"never a partial application" guarantee — recur across multiple lenses.

### Cross-Cutting Themes

- **Duplication / silent divergence from single-source-of-truth code**
  (flagged by: architecture, code-quality, correctness, compatibility,
  portability) — Three distinct pieces of existing logic are intentionally
  cloned: the child's TX-parse + resume + predicate filter (into
  `_harness_emit_list`), the FIFO fork/teardown (into
  `enumerate_interactive_transformations`), and `read_decision`'s verb grammar +
  CRLF/blank parse (into `classify_decision_verb` and the validator loop). Each
  clone is acknowledged in the plan, but none is structurally prevented from
  drifting. The whole feature rests on `--list`/validate staying 1:1 with what
  the apply loop consumes — duplication is exactly what breaks that invariant
  silently.

- **The fixture's awk frontmatter insert is unspecified, untested, and the AC2
  oracle** (flagged by: test-coverage, safety, portability) — Phase 1 §7 leaves
  `migration_apply_decision`'s awk body as `...`. Test-coverage notes AC2 trusts
  this unverified insert as its own correctness oracle; safety notes a malformed
  insert could corrupt rather than fail; portability notes BSD (macOS CI) vs GNU
  awk divergence. The shipped `0002-predicate` fixture sidesteps all three by
  appending to a separate sentinel log instead of mutating real frontmatter.

- **`--list` output robustness at the agent boundary** (flagged by: usability,
  correctness) — The wire protocol escapes TAB/newline/backslash per field, but
  the enumeration path un-escapes via `read_frame` and re-emits with a bare
  `printf`, so a `proposed`/`path` value containing a literal tab or newline
  injects stray columns/lines into the tab-delimited stdout — reintroducing the
  corruption escaping was meant to prevent. The pinned ASCII fixture cannot
  catch it.

- **Scope of the fail-closed guarantee** (flagged by: correctness, safety,
  test-coverage) — The validator guarantees "corpus unmutated" only on pre-apply
  rejection. Once the apply loop starts it mutates per transformation with no
  all-or-nothing semantics, so a mid-apply failure leaves a half-written corpus.
  The Phase 3 SKILL.md sketch's "never a partial application" overstates this.
  Compounding it: a VALIDATE_ERR re-prompt consumes an *extra* decisions-file
  line beyond N, so a file the count==N validator accepts can still under-feed
  at apply time.

### Tradeoff Analysis

- **Shared-helper extraction now vs deferred (architecture/code-quality vs the
  0116 merge constraint)**: Architecture and code-quality both want
  `_interactive_fork` / a shared `_harness_classify_tx` / a single verb-grammar
  definition extracted in this work. The plan defers them partly to avoid
  touching the 0116 shared region during merge coordination. Recommendation:
  the harness-side parse/classify and the verb-grammar extractions do **not**
  touch 0116's `read_decision` stall region and are worth doing now; the
  `_interactive_fork` FIFO extraction is the one with a real 0116-adjacency
  argument and can stay deferred with a binding cross-reference comment.

- **`--list` dirty-tree bypass (usability win vs safety blind spot)**: Running
  `--list` read-only on a dirty tree removes friction (usability) but means the
  read-only path is silent about an in-flight partial session that the mutating
  path warns loudly about (safety, minor). Recommendation: keep the bypass; add
  a one-line stderr (not stdout) notice when an in-flight session log is
  detected.

### Findings

#### Critical

- 🔴 **Correctness**: count==N validator accepts files that under-feed at apply
  time after a VALIDATE_ERR re-prompt
  **Location**: Phase 2, §2 (Validator — count check vs apply-time consumption)
  The validator enforces exactly N verbs (N = decision-requiring prompts), but a
  rejected `edit` triggers VALIDATE_ERR and the runner reads an extra
  decisions-file line. The fixture's `migration_validate_edit` rejects empty
  values, so this is reachable: a file with exactly N verbs including a
  soon-to-be-rejected edit passes the up-front check, then exhausts mid-run after
  earlier transformations have already mutated the corpus — violating the
  fail-closed guarantee. Conversely a valid N+1 recovery file is wrongly
  rejected.

#### Major

- 🟡 **Safety**: fail-closed guarantee is pre-apply only; multi-transformation
  apply has no mid-apply atomicity
  **Location**: Phase 2 (validation) + Phase 3 SKILL.md sketch ("never a partial
  application")
  True for pre-apply rejection, false for mid-apply failure — the child mutates
  per transformation, so file 1 can be written before file 2 fails. The
  documented contract should not claim unconditional all-or-nothing.

- 🟡 **Test Coverage**: AC2's oracle is a brand-new, untested awk frontmatter
  insert grepped in the very file it wrote
  **Location**: Phase 1, §7 (Standalone fixture)
  The test trusts unverified fixture apply logic to be its own correctness
  oracle. `0002-predicate` decouples this via a separate sentinel log; the new
  fixture should too (or pin full file contents with byte-for-byte comparison).

- 🟡 **Test Coverage**: AC6 "corpus unmutated" uses grep-for-absence, not
  byte-for-byte pre/post comparison
  **Location**: Phase 2, §3 (AC6 a/b/c tests)
  Grep-for-absence passes if a partial write produced *any* text other than the
  searched string — a weak mutation oracle for the property AC6 cares most about.

- 🟡 **Portability**: unspecified awk frontmatter insert risks BSD/GNU awk
  divergence on macOS vs Linux CI
  **Location**: Phase 1, §7 (`migration_apply_decision`)
  Left as `...`. BSD awk (macOS) and gawk (Linux) diverge on common idioms; an
  unwritten one-liner is exactly where a macOS-green / Linux-red split is born.
  Pin a POSIX-awk-safe pattern or use a pure-bash insert.

- 🟡 **Safety**: fixture's awk insert could silently corrupt a file instead of
  failing cleanly
  **Location**: Phase 1, §7
  No handling for a target lacking a closing `---`. Because this fixture is the
  pattern an author may copy for real-corpus migrations, it should fail loudly
  (FAIL frame) on a missing delimiter rather than silently no-op or mis-insert.

- 🟡 **Usability**: `--list` un-escapes embedded tabs/newlines, breaking the
  tab-delimited contract it promises
  **Location**: Phase 1, §3 & §5 (enumeration helper + reformat)
  `read_frame` un-escapes; the `--list` `printf` does not re-escape. A
  tab/newline in `proposed`/`path` splits a logical entry. Emit still-escaped
  fields (documenting the escape convention) or fail closed on such values.

- 🟡 **Correctness**: `enumerate_interactive_transformations` may not mirror the
  apply path's custom-session-log resume rebuild
  **Location**: Phase 1, §3
  The apply path rebuilds resume state on READY for migrations with a custom
  `migration_session_log_path` (`interactive-lib.sh:486-505`); the enumerate
  sketch doesn't. For such migrations `--list`/N would compute exclusions from
  the wrong log, diverging from apply. Deriving exclusion entirely child-side
  (from the INIT-supplied path) avoids the runner-side rebuild divergence.

- 🟡 **Correctness**: list-mode TX parse must match the main loop's unescape
  exactly or `--list` diverges from apply
  **Location**: Phase 1, §2 (`_harness_emit_list`)
  The list emit must unescape from the TX buffer (as the main loop does) and let
  `emit_frame` re-escape; otherwise fields with TAB/newline/backslash round-trip
  incorrectly. The plan flags `TX_LINES` locality but not the unescape step.

- 🟡 **Architecture / Code Quality**: list-mode TX parse + resume + predicate
  filter duplicates the main loop and will silently diverge
  **Location**: Phase 1, §2
  `_harness_emit_list` must reproduce `interactive-harness.sh:286-336` (field
  extraction, malformed-TX guard, resume lookup, the SIGPIPE-safe here-string
  predicate eval) 1:1. Extract a shared `_harness_classify_tx <tx>` that both
  the main loop and the list path consume.

- 🟡 **Architecture / Code Quality**: unresolved inline-vs-function decision for
  `_harness_emit_list` around `TX_LINES` locality
  **Location**: Phase 1, §2 (the Note at lines 259-263)
  `TX_LINES` is `local` to `harness_run`; the plan offers "inline or pass-args"
  without choosing. Commit to `_harness_emit_list "${TX_LINES[@]}"` so the
  emitter stays a separately-testable unit rather than bloating an already-~110-
  line function.

- 🟡 **Architecture**: duplicated FIFO/fork/teardown gives two functions one
  bash-3.2-fragile mechanism to maintain in lockstep
  **Location**: Phase 1, §3
  `enumerate_interactive_transformations` re-implements the literal fd 7/8 setup,
  fork, INIT handshake, reap, and teardown from `run_interactive_migration`. The
  deferral is defensible (0116 adjacency), but at minimum add a cross-reference
  comment binding the two copies; preferably extract `_interactive_fork`.

- 🟡 **Architecture**: `classify_decision_verb` forks `read_decision`'s verb
  grammar into a second, drift-prone definition
  **Location**: Phase 2, §1
  Two independent definitions of "what is a legal verb" can disagree — the exact
  silent divergence the fail-closed gate exists to prevent. Route
  `read_decision`'s inner parse through the shared classifier (does not touch its
  three-valued return or the 0116 stall).

- 🟡 **Usability**: no header/legend in `--list`; an agent must infer the
  4-column schema from prose
  **Location**: Phase 1, §5 / Phase 3, §1
  Both `key` and `proposed` are opaque `work-item:NNNN`-shaped strings in the
  fixture. A `# pos\tkey\tproposed\tpath:field` header (or an adjacent legend in
  SKILL.md) prevents an agent writing semantically-wrong-but-validator-passing
  verbs.

#### Minor

- 🔵 **Correctness**: unknown-verb scan runs over all verbs, so a surplus unknown
  verb is reported as "unknown" rather than "surplus" (Phase 2 §2). Still
  fail-closed and names the right position; diagnostic-class ambiguity only.
- 🔵 **Correctness**: validator opens/consumes the decisions file independently
  of the apply-time fd-9 stream (Phase 2 §2). Correct under the single-process
  domain assumption; add a CRLF/blank-line test to lock the two parses together.
- 🔵 **Compatibility**: validator's blank-line/surplus handling may reject files
  the apply path (which stops reading at N) would accept (Phase 2 §2). Mirror
  `read_decision`'s drain semantics; test trailing-blank/junk files.
- 🔵 **Compatibility**: state explicitly that `run_interactive_migration`'s INIT
  (both live and protocol-log emission, `interactive-lib.sh:434-441`) stays
  two-field; only the enumeration fork emits the third (Phase 1 §2/§3).
- 🔵 **Safety**: validator's enumeration-fork failure must fail closed, never
  fall through to apply with a stale/zero N (Phase 2 §2). Add an
  injected-enumeration-failure test.
- 🔵 **Safety**: `--list` on a dirty tree is silent about an in-flight session
  the mutating path warns about (Phase 1 §5). Emit a one-line stderr notice.
- 🔵 **Architecture / Usability**: global `--list` position numbering across
  multiple interactive migrations is not 1:1 with the per-migration decisions
  file (Phase 1 §5). Segment per migration id, or fail closed when >1 interactive
  migration is pending.
- 🔵 **Architecture**: enumeration runs twice on a real resume; document the
  determinism/idempotency requirement on `migration_emit_transformations` +
  `migration_evaluate_predicate` (Phase 2 §2).
- 🔵 **Code Quality**: validator logic is all-inline in `run-migrations.sh` and
  not unit-testable; lift to a `validate_decisions_file <path> <n>` in
  `interactive-lib.sh` (Phase 2 §2).
- 🔵 **Code Quality**: helpers communicate via a global `LIST_ENTRIES` array;
  document or self-enforce the reset-before-call contract (Phase 1/2).
- 🔵 **Code Quality**: `predicate_rc` semantics fork between list mode
  (`1) : ;`) and the main loop (`1` = mechanical-apply); add an explicit comment
  on the list-mode arm (Phase 1 §2).
- 🔵 **Test Coverage**: CRLF / blank-line / bare-`edit` validator cases are named
  in the strategy but never asserted (Testing Strategy).
- 🔵 **Test Coverage**: the VALIDATE_ERR-re-prompt vs count-N interaction is
  untested (ties to the critical finding).
- 🔵 **Test Coverage**: AC1 should also assert stderr is diagnostic-free on the
  success path and pin the exact terminal newline (Phase 1 §8).
- 🔵 **Test Coverage**: the `_harness_emit_list` FAIL-frame path (predicate rc
  other than 0/1) is untested (Phase 1 §2).
- 🔵 **Test Coverage**: no test combines pre-seeded resume state with a decisions
  file to confirm the validator's N reflects the post-resume subset (Phase 1 §8 /
  Phase 2).
- 🔵 **Portability**: confirm the Phase-1 test wires both `PROJECT_ROOT` and
  `CLAUDE_PLUGIN_ROOT` (reuse `seed_predicate_sandbox` plumbing verbatim)
  (Phase 1 §7).
- 🔵 **Usability**: `--help` prints to stderr, so an agent's conventional
  `--help | grep` (stdout) sees nothing; route explicit `--help`/`-h` to stdout
  (Phase 1 §4/§6).
- 🔵 **Usability**: SKILL.md "write" step omits where to put the decisions file
  and that the path must exist/be readable before resume (Phase 3 §1).
- 🔵 **Usability**: count-mismatch errors name a position but not the offending
  `key`/`path` (already in `LIST_ENTRIES`); echo it to close the loop without a
  re-`--list` (Phase 2 §2).

#### Suggestions

- 🔵 **Code Quality**: preserve the per-arm `shift`/`exit` discipline in the new
  `while/shift` flag loop with a guardrail comment (Phase 1 §4).
- 🔵 **Portability**: the "ASCII-only" concern for the `→` arrow is overstated —
  it lives only in SKILL.md (markdown) and prose; no non-ASCII enters a `.sh`
  file, and the rule is an unenforced convention. No action needed for this plan.
- 🔵 **Compatibility**: consider a one-line CHANGELOG note that an unrecognised
  driver flag now exits non-zero (previously silently ignored) (Migration Notes).

### Strengths

- ✅ The central design — a child-side `--list` mode dumping the already-buffered,
  predicate-filtered `TX_LINES` — is the correct boundary choice: single source
  of truth for ordering/filtering, and it structurally avoids the
  skip-still-records-a-session-log side effect of a runner-feeds-skips approach.
- ✅ The wire-protocol change is genuinely additive and backward-compatible,
  verified against the actual `read_frame` (`${FRAME_FIELDS[2]:-}`) and the
  two-field normal INIT — existing mechanical and interactive migrations are
  unaffected and never emit the new frames.
- ✅ Excellent AC-to-test traceability: every in-scope criterion (AC1–AC6) maps
  to a named, asserted test, including AC2's three per-position outcomes with the
  negative (proposed value NOT written for the edited row), and AC6's
  unknown-before-count ordering is reasoned through to the exact positions the
  work item pins (2 / 3 / 4).
- ✅ The fail-closed validator is correctly placed before the apply loop and
  leaves 0116's three-valued `read_decision` return and stall region untouched,
  honouring the shared-region merge constraint.
- ✅ Every new shell construct is bash-3.2-safe and mirrors existing idioms
  (C-style for-loops, `<<<`, `declare -F`, empty-array-safe expansion, literal
  fd 7/8, `printf '%s\t'`), and clears the bashisms denylist.
- ✅ Extracting a pure, separately-testable `classify_decision_verb` and routing
  data→stdout / diagnostics→stderr are the right ergonomic choices for an agent
  invoker.
- ✅ The plan is unusually candid about its own DRY shortcuts, the
  single-interactive-migration scope, and the editorial-judgment premise tension
  — it names its tradeoffs rather than hiding them.
- ✅ Strict unknown-flag rejection is a genuine hardening (today an unknown first
  arg silently runs a full mutating migration), with a verified blast radius
  (only the skill and the test suite call the driver).

### Recommended Changes

1. **Reconcile the validator's count model with apply-time consumption**
   (addresses: the critical correctness finding, the safety pre-apply-only
   finding, the re-prompt test-coverage gap). Decide and document one of: (a) the
   validator counts initial decisions only and re-prompts are explicitly out of
   scope (with rationale), or (b) the validator pre-classifies `edit` values
   through `migration_validate_edit` so a rejecting edit is caught up front, or
   (c) treat N as a lower bound. Add a test where an edit is rejected then
   corrected. Then qualify the SKILL.md guarantee: "no mutation when validation
   fails; once apply begins, partial application is possible — VCS revert is the
   recovery path."

2. **Specify and de-risk the fixture's frontmatter insert** (addresses: AC2
   untested-oracle, awk corruption, BSD/GNU divergence). Pin a POSIX-awk-safe (or
   pure-bash) insert that fails loudly on a missing `---`, AND have the fixture
   append a separate sentinel log à la `0002-predicate`; assert AC2 against the
   sentinel (and/or pin full file contents byte-for-byte) rather than grepping
   the file the insert just wrote.

3. **Strengthen the "corpus unmutated" assertions** (addresses: AC6 weak-oracle).
   Checksum each seed file after seeding and assert byte-for-byte equality (or
   `cmp`) after every AC6 rejection, plus an automated assertion that
   `.accelerator/state/` holds no session-log/applied entry.

4. **Eliminate the three duplication sites or bind them** (addresses: the
   duplication/divergence theme). Extract `_harness_classify_tx` (shared by the
   main loop and `_harness_emit_list`, carrying the SIGPIPE here-string
   discipline once) and route `read_decision`'s parse through
   `classify_decision_verb`. Commit to `_harness_emit_list "${TX_LINES[@]}"`.
   The FIFO `_interactive_fork` extraction may stay deferred but must get a
   cross-reference comment in both copies. Specify that list mode unescapes TX
   fields exactly as the main loop does.

5. **Fix `--list` escaping and self-description** (addresses: the agent-boundary
   theme). Keep enumerated fields escaped on the way to stdout (document the
   `\t`/`\n` escape convention in `--help` and SKILL.md) or fail closed on values
   containing tabs/newlines; add a self-describing header or an adjacent
   4-column legend in SKILL.md.

6. **Handle the multi-interactive-migration case** (addresses:
   architecture/usability global-numbering). Either segment `--list` output per
   migration id (and document it), or fail `--list`/validation closed with a
   clear message when more than one interactive migration is pending, so the
   numbering an agent sees always maps 1:1 to the single file it must write.

7. **Smaller hardenings**: mirror the custom-session-log resume rebuild in the
   enumerate helper (or derive exclusion child-side); make enumeration-fork
   failure fail closed; lift the validator into a testable
   `validate_decisions_file` function; route `--help` to stdout; add the
   CRLF/blank-line/bare-`edit`/FAIL-frame/resume+validate tests; emit a dirty-tree
   in-flight-session notice on stderr; echo the offending key/path in
   count-mismatch errors.

---

## Per-Lens Results

### Architecture

**Summary**: The central design decision — a child-side `--list` mode that dumps
the already-buffered, predicate-filtered `TX_LINES` rather than runner-feeds-skips
— is architecturally sound and well-justified. The additive wire-protocol
extension is backward-compatible and verified. The main structural risks are the
deliberately duplicated FIFO/fork/teardown and the duplicated verb/predicate
parsing across three independent copies that must stay in lockstep, each
acknowledged but none structurally mitigated.

**Strengths**: Child-side boundary keeps ordering/filter in the single source of
truth and guarantees no mutation path in a dry emit; additive protocol change
needs no INIT change in `run_interactive_migration`; validation reuses the
enumeration for N rather than re-deriving it; fail-closed validator placed before
apply without altering the 0116 region; clean, independently-mergeable phase
decomposition.

**Findings**:
- 🟡 major / high — *Duplicated FIFO/fork/teardown gives `run_interactive_migration`
  and the enumerator two reasons to change in lockstep* (Phase 1 §3). Re-implements
  the most bash-3.2-fragile plumbing (literal fds, mkfifo ordering, open-after-fork
  EOF discipline). Extract `_interactive_fork`/`_interactive_teardown`, or at
  minimum a comment-level invariant binding the copies + a test that both INIT
  frames stay field-identical.
- 🟡 major / high — *Unresolved inline-vs-function decision for `_harness_emit_list`*
  (Phase 1 §2). Presented as a standalone function, but the Note says `TX_LINES` is
  local so it must be inlined or passed the lines. Decide: extract `_harness_parse_tx`
  shared by both paths and make `_harness_emit_list` take the lines as args.
- 🟡 major / medium — *`classify_decision_verb` forks `read_decision`'s verb grammar*
  (Phase 2 §1). Two independent definitions of a legal verb can disagree. Route
  `read_decision`'s inner parse through the shared classifier without touching its
  three-valued return or the stall.
- 🔵 minor / medium — *List-mode predicate evaluation re-derives the SIGPIPE-safe
  here-string discipline by hand* (Phase 1 §2). Factor resume-lookup + predicate
  routing into one `_harness_classify_tx`.
- 🔵 minor / high — *Enumeration runs twice on a real resume* (Phase 2 §2). The
  emitter and predicate are evaluated twice per resume; document the
  determinism/idempotency requirement.
- 🔵 minor / medium — *Global position numbering across multiple interactive
  migrations is not 1:1 with per-migration consumption* (Phase 1 §5). Number
  per-migration with an id prefix, or guard that at most one interactive migration
  is pending.

### Correctness

**Summary**: Logically careful about the position-mapping invariant, and the
validator's check ordering produces the AC6 positions it claims. But there is a
real gap between the count==N validator and apply-time consumption: a VALIDATE_ERR
re-prompt consumes an extra line beyond N, so an accepted file can under-feed at
apply time and stall mid-migration with a partially-mutated corpus. Secondary:
the enumerate helper's resume-state construction may not mirror the custom
session-log rebuild, and the harness-side parse-divergence note is load-bearing.

**Strengths**: Correctly maps position to the decision-requiring, non-resumed
subset; the unknown-before-count ordering provably yields AC6's positions; leaves
the three-valued return and stall untouched; `_harness_emit_list`'s rc handling
matches the main loop; the `IFS=$'\t'` reformat maps fields correctly and tolerates
an empty trailing `proposed`.

**Findings**:
- 🔴 critical / high — *count==N validator accepts files that under-feed at apply
  time after a VALIDATE_ERR re-prompt* (Phase 2 §2). The fixture's
  `migration_validate_edit` rejects empty values, making the path reachable; the
  fail-closed guarantee is violated for the edit-revalidation case and valid
  recovery files are wrongly rejected. Document re-prompts out of scope, pre-validate
  edits, or model N as a lower bound; add a reject-then-correct test.
- 🟡 major / medium — *`enumerate_interactive_transformations` may not mirror the
  apply path's custom-session-log resume rebuild* (Phase 1 §3). For custom-log
  migrations, `--list`/N would compute exclusions from the wrong log. Handle the
  READY rebuild or derive exclusion child-side.
- 🟡 major / medium — *list-mode TX parse must match the main loop's unescape exactly*
  (Phase 1 §2). Unescape from the TX buffer and let `emit_frame` re-escape, else
  fields with TAB/newline/backslash diverge from apply; the ASCII fixture won't catch
  it.
- 🔵 minor / medium — *unknown-verb scan runs over all verbs, so a surplus unknown
  verb is reported as "unknown" not "surplus"* (Phase 2 §2). Behaviour correct;
  diagnostic-class ambiguity only.
- 🔵 minor / high — *validator opens/consumes the decisions file independently of the
  apply-time fd-9 stream* (Phase 2 §2). Correct under single-process; add a
  CRLF/blank-line test to lock the parses together.

### Code Quality

**Summary**: Well-structured, honest about tradeoffs, cleanly decomposed into
three TDD phases with a credible independently-mergeable claim. The dominant risk
is duplication: three pieces of logic cloned from single-source-of-truth code with
the shared-helper extractions deferred. The `classify_decision_verb` extraction is
a genuine win; the unresolved `TX_LINES` locality choice and the all-inline
validator are the weakest spots.

**Strengths**: Pure testable `classify_decision_verb`; unusually candid about DRY
shortcuts; sound phase decomposition; error handling consistent with the existing
harness (FAIL propagation, validate-before-apply, stdout/stderr split); validator
messages name position + expected verbs.

**Findings**:
- 🟡 major / high — *List-mode TX parse/resume/predicate filter duplicates the main
  loop and will silently diverge* (Phase 1 §2). Extract `_harness_classify_tx`.
- 🟡 major / high — *Unresolved inline-vs-pass-args for `TX_LINES`* (Phase 1 §2).
  Commit to `_harness_emit_list "${TX_LINES[@]}"`.
- 🔵 minor / high — *FIFO fork/teardown duplicated wholesale; shared helper deferred*
  (Phase 1 §3). Reconsider pulling extraction in, or add cross-reference comments.
- 🔵 minor / medium — *Validator logic all-inline and not independently testable*
  (Phase 2 §2). Lift into `validate_decisions_file <path> <n>` in interactive-lib.sh.
- 🔵 minor / medium — *Helpers communicate via a global `LIST_ENTRIES` array* (Phase
  1/2). Document or self-enforce the reset-before-call contract.
- 🔵 minor / medium — *`predicate_rc` semantics fork between list mode and main loop
  without a shared comment* (Phase 1 §2). Add an explicit comment on the list-mode
  arm.
- 🔵 suggestion / low — *while/shift conversion clean but watch per-arm shift
  discipline* (Phase 1 §4). Keep each arm's shift/exit at its tail with a guardrail
  comment.

### Test Coverage

**Summary**: Unusually disciplined AC-to-test traceability: every in-scope AC maps
to a named, asserted test, and the load-bearing edges are identified. The biggest
risks are weak oracles, not absent tests: AC2 trusts an untested awk insert as its
own oracle, and AC6 "corpus unmutated" is grep-for-absence rather than byte-for-byte.
A few named edges (re-prompt count interaction, CRLF/blank/bare-edit) are never
asserted.

**Strengths**: Complete AC1–AC6 traceability incl. AC2's three outcomes with the
negative; research-flagged edges carried into tests; appending before `test_summary`
is sound for the floor; AC6 ordering reasoned to exact positions; test isolation
follows the established sandbox pattern.

**Findings**:
- 🟡 major / high — *AC2 oracle is a brand-new untested awk insert grepped in the file
  it wrote* (Phase 1 §7). Use a decoupled sentinel log and/or pin full file contents.
- 🟡 major / high — *AC6 "corpus unmutated" via grep-for-absence, not byte-for-byte*
  (Phase 2 §3). Checksum/`cmp` before vs after; assert clean `.accelerator/state/`.
- 🔵 minor / high — *CRLF / blank-line / bare-`edit` validator cases named but never
  asserted* (Testing Strategy).
- 🔵 minor / medium — *VALIDATE_ERR re-prompt vs count-N interaction untested* (ties
  to the critical finding).
- 🔵 minor / medium — *AC1 should also assert stderr is diagnostic-free and pin the
  exact terminal newline* (Phase 1 §8).
- 🔵 minor / medium — *`_harness_emit_list` FAIL-frame path untested* (Phase 1 §2).
- 🔵 minor / low — *No test combines pre-seeded resume with a decisions file to confirm
  the validator's N reflects the post-resume subset* (Phase 1 §8 / Phase 2).

### Safety

**Summary**: Fundamentally a corpus-mutation safety story, and it gets the most
important protection right: a fail-closed validator before the apply loop, a
genuinely read-only `--list` (no fd 9, no session-log writes, no APPLY), and a safe
dirty-tree bypass. The residual gap is pre-existing and correctly scoped out: the
apply loop has no mid-apply atomicity, so the "corpus unmutated" guarantee holds
only for pre-apply rejection — and the plan's wording risks overstating it.

**Strengths**: All validator failure paths exit before mutation; `--list` is a
dedicated read-only fork; dirty-tree bypass gated correctly; tests assert the
negative property; existing clean-tree enforcement and revert guidance preserved;
unknown-flag rejection is a hardening with a well-characterised blast radius.

**Findings**:
- 🟡 major / high — *Fail-closed guarantee is pre-apply only; multi-transformation
  apply has no mid-apply atomicity* (Phase 2 / Phase 3 sketch). Qualify the SKILL.md
  guarantee; don't claim "never a partial application" unconditionally.
- 🟡 major / medium — *Fixture's awk insert could silently corrupt a file* (Phase 1
  §7). Make it fail loudly on a missing `---`; it's a pattern authors may copy.
- 🔵 minor / medium — *Validator's enumeration-fork failure should fail closed, not
  fall through with a stale/zero N* (Phase 2 §2). Add an injected-failure test.
- 🔵 minor / low — *Strict unknown-flag rejection is safe but verify no caller passes
  through-args* (Phase 1 §4). Grep hooks/tasks/skill bodies; record the audit in
  Migration Notes.
- 🔵 minor / medium — *`--list` on a dirty tree is silent about an in-flight session*
  (Phase 1 §5). Emit a one-line stderr notice pointing at resume/discard guidance.

### Portability

**Summary**: Largely sound: every introduced construct is bash-3.2-safe, matches
existing idioms, and clears the bashisms denylist. The one genuine hazard is the
unspecified awk frontmatter insert (BSD vs GNU). The "ASCII-only" concern for the
`→` arrow is overstated — no non-ASCII lands in a `.sh` file and the rule is an
unenforced convention.

**Strengths**: All new constructs bash-3.2-safe and mirrored in existing code;
honours the fd constraint (literal 7/8, no fd 9 in the read-only fork); portable
`printf '%s\t'` not `echo -e`; 80-col error continuations match the existing idiom.

**Findings**:
- 🟡 major / high — *Unspecified awk frontmatter-insert risks BSD/GNU divergence on
  macOS CI* (Phase 1 §7). Pin a POSIX-awk-safe pattern or use pure bash.
- 🔵 minor / high — *ASCII-only "risk" for the arrow is overstated* (Phase 3). No
  non-ASCII enters a `.sh`; no enforced check exists; shipped shell already contains
  non-ASCII comments. No action needed.
- 🔵 minor / medium — *Fixture relies on `$PROJECT_ROOT`/`$CLAUDE_PLUGIN_ROOT`* (Phase
  1 §7). Reuse `seed_predicate_sandbox` plumbing verbatim so both roots are provided.

### Compatibility

**Summary**: The wire-protocol changes are genuinely additive and backward-compatible
(verified against `read_frame` and the two-field normal INIT); the new frames are
never emitted outside list mode. The CLI hardening is safe in practice (only the
skill and tests call the driver) though a break for any out-of-tree caller. The main
residual risk is parser divergence between the new validator and `read_decision`'s
consumption parse.

**Strengths**: Optional third INIT field verified backward-compatible; new frames
reuse `escape_field` and only emit in list mode; respects 0116's `--decisions-file`
ownership (no second flag); three-valued return and stall left untouched;
unknown-flag claim validated in-tree.

**Findings**:
- 🔵 minor / medium — *Validator's blank-line/surplus handling may reject files the
  apply path would accept* (Phase 2 §2). Mirror `read_decision`'s drain semantics;
  test trailing-blank/junk files.
- 🔵 minor / high — *State explicitly that `run_interactive_migration`'s INIT stays
  two-field; only the enumeration fork emits the third* (Phase 1 §2/§3). Confirm
  protocol-log INIT assertions still match.
- 🔵 minor / medium — *Consider a CHANGELOG note that an unrecognised driver flag now
  exits non-zero* (Migration Notes). The resume guidance forms remain valid.

### Usability

**Summary**: From the invoker's perspective the plan delivers a coherent,
well-sequenced contract. The biggest risk is the machine-parseability of `--list`,
which silently un-escapes embedded tabs/newlines on the way to stdout, plus the
absence of any documented mapping from a `--list` line to a decisions-file position
in the output itself. Error messages and discoverability are strong; the
single-migration scope and editorial-judgment premise are surfaced honestly.

**Strengths**: `list → decide → write → resume` documented end-to-end; fail-closed
messages name position + expected verbs; coherent discovery path; correct
stdout/stderr discipline; scope limitation and premise tension acknowledged;
read-only `--list` works on a dirty tree.

**Findings**:
- 🟡 major / high — *`--list` un-escapes embedded tabs/newlines, breaking the
  tab-delimited contract* (Phase 1 §3/§5). Emit escaped fields (document the
  convention) or fail closed on such values.
- 🟡 major / medium — *No header/legend in `--list`; the 4-column schema must be
  inferred from prose* (Phase 1 §5 / Phase 3 §1). Add a `#`-led header or an adjacent
  legend in SKILL.md.
- 🔵 minor / high — *`--help` prints to stderr, invisible to a naive `--help` capture*
  (Phase 1 §4/§6). Route explicit `--help`/`-h` to stdout.
- 🔵 minor / medium — *Global `--list` numbering vs per-migration decisions-file scope
  could surprise an agent* (Phase 1 §5). Segment per migration, or fail closed when
  >1 is pending.
- 🔵 minor / medium — *Contract omits where to put the decisions file and how to create
  it* (Phase 3 §1). Add a concrete sample path + creation one-liner; note the path
  must exist/be readable.
- 🔵 minor / medium — *Too-few/too-many errors name a position but not the verbs/key*
  (Phase 2 §2). Echo the offending `key`/`path:anchor` from `LIST_ENTRIES`.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-22

**Verdict:** REVISE

The plan was substantially revised across four user decisions: validation
became a **no-mutation dry-apply pass** (replacing the unsound count==N check),
the FIFO fork / TX-parse / verb-grammar duplication was **extracted into shared
helpers** (`_interactive_fork`, `_harness_classify_tx`, reuse of `read_decision`),
`--list` fields now **fail closed on tab/newline**, and `--list` output is
**segmented per migration**. Re-running all eight lenses against the revised plan
shows **the pass-1 critical and every pass-1 major resolved or substantially
addressed**. The verdict remains REVISE because the deeper design surfaced a fresh
layer: one genuine new behavioural gap (the dirty-tree pre-flight blocks the very
`--decisions-file` resume the contract documents) plus a cluster of
implementation-specification gaps the dedup/dry-apply introduced. None overturns
the approach — this is a markedly healthier REVISE than pass 1, needing a
tightening pass rather than a rethink.

### Previously Identified Issues

- 🔴 **Correctness**: count==N validator accepts files that under-feed at apply
  time — **Resolved**. Dry-apply reuses `read_decision`, so consumption equals the
  live run by construction; a bad edit is a hard reject before any write.
- 🟡 **Safety**: fail-closed guarantee pre-apply only / "never a partial
  application" overstated — **Resolved**. Wording now qualified (validation
  failure → unmutated; apply-time failure → VCS revert).
- 🟡 **Test Coverage**: AC2 oracle was an untested awk grepped in the file it
  wrote — **Resolved**. Decoupled `.fixture/applied/log` sentinel added (assertion
  *form* still loose — see new minors).
- 🟡 **Test Coverage**: AC6 "corpus unmutated" via grep-for-absence — **Partially
  resolved**. Plan now specifies byte-identical/`cmp`, but the helper to do so does
  not exist (see new major N4).
- 🟡 **Portability**: unspecified awk → BSD/GNU divergence — **Resolved**. POSIX-awk
  spec pinned, identical across platforms (a *different* `-v` escaping issue
  appeared — N5).
- 🟡 **Safety**: fixture awk could silently corrupt — **Resolved**. Fails loud
  (`exit 3` → FAIL) on a missing `---` (unchecked `mv` is a new minor).
- 🟡 **Usability**: `--list` un-escapes embedded tabs/newlines — **Resolved**.
  Fail-closed guard + shared classify unescape round-trip.
- 🟡 **Correctness**: enumerate may not mirror custom-session-log resume rebuild —
  **Resolved** (centralised in `_interactive_fork`; one caveat: the pre-fork
  *default* build must also be centralised — new minor).
- 🟡 **Correctness**: list-mode TX parse unescape divergence — **Resolved** (shared
  `_harness_classify_tx`).
- 🟡 **Architecture / Code Quality**: TX-parse duplication — **Resolved**
  (`_harness_classify_tx`).
- 🟡 **Architecture / Code Quality**: inline-vs-function `TX_LINES` — **Resolved**
  (passed as args).
- 🟡 **Architecture**: FIFO fork/teardown duplication — **Resolved in approach**
  (`_interactive_fork` extracted); the extraction is under-specified for the live
  handler (new major N3).
- 🟡 **Architecture**: `classify_decision_verb` forks the verb grammar —
  **Resolved** (dry-apply reuses `read_decision`; no classifier).
- 🟡 **Usability**: no header/legend in `--list` — **Resolved** (legend in
  `--help`/SKILL.md, AC1 byte-exactness preserved; vocabulary-consistency minor).
- 🟡 **Architecture / Usability**: global multi-migration numbering — **Resolved**
  (per-migration segmentation), but the segmentation decision spawned two new
  majors (N6).

### New Issues Introduced

- 🟡 **Correctness** (major, high): The dirty-tree pre-flight is gated only on
  `-z "$LIST_MODE"`, so a `--decisions-file` resume still requires a clean tree or
  `FORCE`. A partial interactive run is exactly what dirties the tree, and the
  stall message tells the agent to resume with `bash … --decisions-file …` — which
  then fails the pre-flight. The documented `list → decide → write → resume`
  escape hatch is non-functional for a real partial-run resume. (AC2 masks this
  with `FORCE=1`.) **Most important new finding** — bypass/down-grade the
  pre-flight when a decisions file is set, mirroring `--list`, and test resume on a
  dirty tree without `FORCE`.
- 🟡 **Correctness / Safety** (major): Dry/live parity assumes
  `migration_validate_edit` is pure of corpus state. If a validator reads on-disk
  frontmatter, dry-apply (unmutated corpus) can pass while the live run (earlier
  files mutated) fails later and re-prompts/exhausts mid-run — the exact
  fail-closed violation dry-apply removed. Make purity-of-arguments an explicit
  author contract; add a fixture proving it.
- 🟡 **Architecture / Code Quality** (major): `_interactive_fork` is fully
  specified for the enum/dry handlers but **not for the live run**, whose
  ~150-line frame loop (PROMPT/VALIDATE_ERR/RECORDED/APPLY/DRIFT + watchdog +
  no_op_pending soft-defer + STATE_FILE/INTERACTIVE_APPLIED tail) is the bulk of
  the function. Without a `_live_handle_frame` sketch the dedup risks relocating
  the easy plumbing while the hard stateful loop stays cloned or wedged awkwardly.
- 🟡 **Test Coverage** (major): The "byte-identical (`cmp`/checksum)" corpus and
  "byte-for-byte" stdout assertions assume a helper that doesn't exist — `assert_eq`
  / `assert_file_content_eq` compare via command substitution, which strips
  trailing newlines and cannot pin a terminal newline. Add a `cmp`-based helper or
  the central byte-identity claims are unenforceable.
- 🟡 **Portability** (major): The fixture's `awk -v line="$key: [$value]"`
  escape-processes backslashes in the value (uniformly on BSD and gawk), so a value
  with a `\` writes mangled frontmatter — in a fixture authors are told to copy.
  Pass the value via `ENVIRON[]` (no escape processing) instead of `-v`.
- 🟡 **Usability** (major ×2): (a) A blind agent following `list → write` cannot
  learn the migration `<id>` in the single-migration case — `--list` deliberately
  omits the `# migration <id>` header (AC1), yet the write step needs the id in
  `migrations-<id>-decisions.txt`; the id is only learned by first hitting the
  stall, inverting the documented order. (b) Multi-migration segmentation is a
  parse-contract cliff: an agent that learned "every stdout line is data" will
  mis-parse the `#` header, and there is no supported multi-file resume behind it.
- 🔵 **Minors** (several, across lenses): `_dry_send_decide` / `_decisions_have_more`
  are referenced but undefined (and `_decisions_have_more` must reuse
  `read_decision`'s blank/CRLF skip or a trailing blank line false-positives a
  surplus); the rc-10 loop-stop sentinel is an unnamed magic number; the `--list`
  segmentation block uses a parallel-array + manual-cursor tangle (emit per-migration
  inside the enumerate loop instead); the `--help` test should drop `2>&1` to
  actually pin the stdout routing; the bare-`edit` test should assert DRY_REJECT
  (empty value), not just "valid verb"; the unchecked `mv` in the fixture; the
  pre-fork default `build_resume_state_file` must also live in `_interactive_fork`;
  `ROUTE=fail` overloads the classify out-param; mechanical-route applies are not
  dry-validated; the SKILL.md/`--help`/work-item column vocabulary is inconsistent;
  CHANGELOG should frame the flag-rejection/stdout-help changes alongside 0116.

### Assessment

The redesign worked: the pass-1 critical correctness defect and all 14 majors are
resolved, and the approach (dry-apply = apply-minus-mutation; shared helpers;
per-migration segmentation) is now sound and well-grounded in the actual code. The
plan is close. One more tightening pass should: (1) fix the dirty-tree-blocks-resume
gap (N1) — the only new finding that breaks the documented contract; (2) sketch the
`_interactive_fork` live handler (N3); (3) add the `cmp`-based test helper (N4);
(4) make `migration_validate_edit` purity an explicit contract (N2); (5) switch the
fixture off `awk -v` (N5); and (6) surface the migration `<id>` to a blind agent and
clarify the multi-migration parse contract (N6). With those, the plan is ready to
implement. Re-review verdict: REVISE — but a tightening pass, not a rethink.

## Re-Review (Pass 3) — 2026-06-22

**Verdict:** APPROVE

The pass-2 tightening pass was applied via six user-confirmed changes, and pass-3
re-ran all eight lenses. **Every pass-2 finding is resolved or consciously
accepted.** Pass-3 surfaced a thin further layer that was either (a) two
inconsistencies introduced *by* the tightening edits, (b) completion detail on the
`_interactive_fork` boundary spec, or (c) one cross-doc contradiction from the
0119 deferral — all now addressed in-session. What remains is one accepted tradeoff
(validator purity is documented, not enforced — a user decision) and
implementation-time test/polish detail. The approach has been validated across
three passes; the plan is ready to implement.

### Previously Identified Issues (pass-2 findings)

- 🟡 **Correctness**: dirty-tree pre-flight blocks the documented `--decisions-file`
  resume (N1) — **Resolved by deferral**: recorded as a 0119 functional precondition
  in the plan (Dependencies/References) and work item 0117, with the interim `FORCE`
  path documented in SKILL.md.
- 🟡 **Correctness/Safety**: `migration_validate_edit` purity-of-corpus-state (N2) —
  **Resolved (documented)**: explicit author-contract note (Phase 3 §2) + the
  double-invocation reality; enforcement fixture consciously declined (user decision).
- 🟡 **Architecture/Code-quality**: `_interactive_fork` live-handler unspecified (N3)
  — **Resolved**: responsibility-split sketch (fork vs `_live_handle_frame` vs
  live-only tail), now with the full boundary-global set named (see pass-3 fixes).
- 🟡 **Test-coverage**: byte-identical assertions assumed a non-existent helper (N4)
  — **Resolved**: shared `cmp`-based `assert_files_identical`/`assert_stdout_exact`
  added (Phase 1 §8); AC1/AC3 rewired to use them (pass-3 fix).
- 🟡 **Portability**: fixture `awk -v` escape-processing (N5) — **Resolved**:
  switched to value-transparent `ENVIRON[]` + guarded `mv` (Phase 1 §7).
- 🟡 **Usability**: blind-agent `<id>` + multi-migration parse cliff (N6) —
  **Resolved**: `<id>` sourced from the stall, `#`-skip parse rule + multi-migration
  stderr notice (Phase 1 §5, Phase 3 §1).

### New Issues Introduced (pass-3) — all addressed in-session

- 🟡 **Test-coverage**: AC1/AC3 still named `assert_eq` (strips trailing newline),
  defeating the new `cmp` helper — **Fixed**: rewired to `assert_stdout_exact`.
- 🟡 **Portability**: non-ASCII `→` arrows in new shell snippets (`_harness_emit_list`,
  fixture, `_dry_handle_frame`), contradicting the ASCII-clean claim — **Fixed**:
  converted to `->` in `.sh`-destined snippets (prose/markdown arrows left).
- 🟡 **Architecture/Code-quality**: `_interactive_fork` boundary contract incomplete —
  `mig_in`/`mig_out`/`runner_log_path` cross it unnamed, `no_op_pending` is a
  pre-parse raw match that doesn't fit the handler model, and `_harness_classify_tx`
  published only 4 of the 7 TX fields the live PROMPT needs — **Fixed**: named the
  full global set, made `no_op_pending` a fork-owned pre-dispatch hook, and extended
  the classify helper to publish all seven fields.
- 🔵 **Code-quality**: `--list` tab/newline guard ran *after* the tab-join (so an
  embedded tab corrupted the row first) — **Fixed**: moved the guard into
  `_enum_handle_frame`, on the individual fields before joining.
- 🟡 **Usability/Safety**: the 0116 stall's copy-paste resume command omits the
  `FORCE` that step 4 requires on a dirty tree — **Fixed**: step 4 now flags the
  omission, scopes `FORCE` tightly (read the in-flight guidance first; verify the
  dirty paths), and notes reconciling the stall output belongs with 0119.
- 🔵 **Portability**: `readonly _FORK_STOP` re-source hazard — **Fixed**: guarded
  (`[ -n "${_FORK_STOP:-}" ] || readonly …`).

### Consciously Deferred / Accepted (no further change)

- Validator purity is documented, not enforced (user decision — no enforcement
  fixture).
- Mechanical-route applies are not dry-validated (documented scope; the all-prompt
  fixture does not exercise it; VCS revert is the recovery path).
- Implementation-time test/polish detail: assertions for the in-flight and
  multi-migration stderr notices and the tab/newline guard; the `_FORK_STOP` abort
  path; a decisions-file line number alongside the position in error messages;
  mirroring the literal-TAB note into `--help`; dropping the redundant `<&9`. These
  are naturally handled while implementing and do not block the plan.

### Assessment

Three passes took the plan from a critical correctness defect (pass-1) through a
sound redesign (pass-2: dry-apply, shared helpers, segmentation) to spec-completion
and self-consistency (pass-3). All design-level concerns are resolved; the residual
items are one accepted tradeoff and implementation-time detail. Continuing to
re-review would be polishing with diminishing returns. **Verdict: APPROVE — ready
to implement.**

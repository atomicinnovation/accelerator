---
date: "2026-05-30T19:26:56Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-30-0069-migration-framework-interactive-validation-hooks.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, correctness, code-quality, test-coverage, safety, compatibility, usability, documentation]
review_pass: 3
status: complete
---

## Plan Review: 0069 — Migration Framework Interactive Validation Hooks

**Verdict:** REVISE

The plan establishes a thoughtful opt-in interactive contract layered on top of the existing mechanical migration framework, with strong phase decomposition, TDD discipline, an explicit AC-1 regression net, and a CI-asserted documentation drift test — all real strengths. However, five critical findings (one architectural assumption-bug, two persistence-helper correctness bugs, one safety-critical ordering window, and two compatibility breakages) plus a large cluster of major findings concentrated around the JSONL persistence helpers, the in-band JSON composition by string-slicing, the bash 4+ floor, and the TSV-with-base64 author surface mean the plan should not proceed to implementation in its current shape. Most issues are fixable with targeted edits rather than redesign, and several findings overlap across multiple lenses — addressing the persistence-helper and JSON-composition concerns once will close findings in five separate lenses.

### Cross-Cutting Themes

- **`atomic_jsonl_append` / `atomic_jsonl_remove_by_key` correctness** (Architecture, Correctness, Code Quality, Safety, Test Coverage) — The PIPE_BUF/sync atomicity claim is wrong for regular-file appends, the substring-grep removal silently corrupts adjacent records under prefix collisions or extras-containing-key-substrings, and there is a dead `pattern` variable indicating the implementer intended one approach and wrote another. The proposed concurrent/SIGKILL durability tests prove weaker properties than their AC claims.
- **String-slice JSON composition for session-log records** (Architecture, Correctness, Code Quality, Safety, Usability) — Stripping `{` from migration-authored `extras_json` and splicing framework fields fails on `{}`, leading whitespace, key collisions with framework-mandatory fields, and embedded quotes/backslashes. Migration authors hand-writing JSON in bash will produce malformed records under realistic inputs.
- **TSV protocol ergonomics for the migration-author surface** (Code Quality, Usability) — 7-field positional TSV with embedded base64 multi-line and inline JSON is hostile to hand-authoring and difficult to extend. A `harness_emit_transformation key=value …` helper would hide the wire format from migration authors.
- **bash 4+ floor (coproc, declare -A) is unaddressed** (Architecture, Compatibility) — macOS ships bash 3.2 by default; the plan adds two bash-4-only constructs to a runner that previously worked on 3.2 without any version assertion, error message, or SKILL.md note.
- **Apply-then-record ordering creates a partial-mutation window** (Safety, with Correctness implications via DRIFT detection) — `migration_apply_decision` runs before the runner writes the JSONL record; a crash in the window leaves the artefact mutated but the session log unaware, and the resume comparison against the now-mutated artefact can mis-classify the state as drift, leading to re-prompts or double-mutation.
- **Recursive validation re-prompt in bash** (Correctness, Code Quality) — `read_decide_and_recurse` re-prompts via recursion; bash has no tail-call optimisation and `set -e` interacts unpredictably with nested local state. An explicit `while true` loop is the right shape.
- **AC-1 regression net is behavioural, not byte-identical** (Compatibility, Safety) — `test-migrate.sh`'s 80 assertions verify substrings/outcomes, not byte-identical artefact bytes or stdout/stderr; a snapshot `diff -r` against a pre-change capture is needed to honour the AC-1 promise.
- **Documentation strategy and drift redaction** (Documentation) — Docs land only in Phase 7 despite Phases 1–6 shipping user-visible surface (`--decisions` flag, new helpers); the drift test's timestamp-only redaction will fail on first CI run because absolute paths (tempdirs, session-log path) also vary.

### Tradeoff Analysis

- **Production simplicity vs test instrumentation** (Architecture vs Test Coverage): `MIGRATION_PROTOCOL_LOG` couples test concerns into production runtime files (Architecture concern), but its absence forces tests to assert only on end-state, which weakens coverage (Test Coverage concern). Recommendation: centralise frame I/O behind `emit_frame`/`read_frame` functions on each side so the env-var check lives in exactly two places with a clear `# test-only` comment — or push instrumentation into a test-only `tee` wrapper on the coproc pipes.
- **Bash-callback flexibility vs author burden** (Usability): The five-callback model is more flexible than a declarative manifest but burdens every first-time interactive-migration author (and every author will be a first-timer for the lifetime of the corpus). Recommendation: keep the callback model but provide author-facing helpers (`harness_emit_transformation`, `harness_extras_set`, `harness_reject`) so the wire format, JSON escaping, and error-emission conventions become single-source-of-truth in the harness library.
- **`--decisions` CLI vs env var** (Usability vs Compatibility): A user-facing CLI flag for a test-only mechanism is ergonomically wrong (Usability) but adding it now and quietly later is also a compatibility shift. Recommendation: move to `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var from day one — env-var precedent already exists in the codebase (`ACCELERATOR_MIGRATE_FORCE`).

### Findings

#### Critical

- 🔴 **Compatibility**: `scripts/test-atomic-common.sh` already exists; plan declares it `(new)`
  **Location**: Phase 2 §§1–2
  Phase 2 §1/§2 declare the file `(new)` and bootstrap it from scratch. The file exists today with assertions for `atomic_write` / `atomic_append_unique` / `atomic_remove_line`. Literal execution of the plan would clobber the existing AC-1 regression net for atomic helpers. **Fix**: extend the existing file, do not create.

- 🔴 **Compatibility**: Interactive coproc driver silently breaks `MIGRATION_RESULT: no_op_pending` contract
  **Location**: Phase 3 §4 + What We're NOT Doing
  The frame loop has no branch for `MIGRATION_RESULT:` and no default branch; a pre-`harness_run` emission of `no_op_pending` is silently discarded, the migration exits 0 with `saw_done=0`, and the runner reports "exited without DONE" — exactly the opposite of the soft-defer semantics the plan promises. **Fix**: either recognise `MIGRATION_RESULT: no_op_pending` as a pre-handshake pseudo-frame and treat as soft-defer, or revise the "What We're NOT Doing" claim to disclaim the interaction.

- 🔴 **Safety**: Apply-then-record ordering opens a window where artefact mutation outlives its session-log record
  **Location**: Phase 4 §6 and Phase 5 §5 (harness dispatch)
  The harness calls `migration_apply_decision` *before* emitting `RECORDED`, and the runner persists only after receiving `RECORDED`. A SIGKILL in the window leaves the artefact mutated with no log record. On re-entry, DRIFT may fire spuriously (because `proposed_value` is recomputed against the mutated artefact) and `apply_decision` may run a second time. Breaks ADR-0037 §3 "no decisions lost" guarantee for non-idempotent migrations. **Fix**: invert ordering — runner persists JSONL first, then signals harness to apply (write-ahead-log discipline), with the runner owning the apply confirmation.

- 🔴 **Correctness / Safety**: `atomic_jsonl_append` durability and atomicity claims are based on a misreading of POSIX
  **Location**: Phase 2 §3
  The helper uses `printf '%s\n' "$line" >> "$target"; sync`. The docstring claims PIPE_BUF-atomic per POSIX — but PIPE_BUF governs pipes/FIFOs, not regular files; `sync(8)` is a system-wide flush that does not guarantee per-file synchronous completion on all platforms. JSONL lines containing base64-encoded `display_lines_b64` can easily exceed PIPE_BUF, in which case `printf` issues multiple `write(2)` syscalls and concurrent writers can interleave. **Fix**: either (a) lock with `flock` around the append, or (b) implement via temp-then-rename using the existing `atomic_write`; correct the docstring to state the actual guarantee.

- 🔴 **Correctness / Safety / Architecture**: `atomic_jsonl_remove_by_key` `grep -F` substring match removes wrong records
  **Location**: Phase 2 §4
  `grep -v -F '"transformation_key":"<key>"'` substring-matches anywhere on the line. (i) Adjacent keys with shared prefixes (`foo` and `foobar`) collide — though the trailing `"` happens to guard the obvious case, no test documents the invariant; (ii) more importantly, any record whose other fields (extras_json, proposed_value, user_value) contain the literal substring is silently deleted; (iii) the computed `pattern` local is dead code — the actual `grep` uses an un-`sed`-escaped inline pattern, suggesting the implementer intended one design and wrote another; (iv) `|| true` masks pipeline failures including disk-full / EROFS on `atomic_write`. **Fix**: parse the record's first field with `awk`, or anchor `grep -E` to start-of-line with a canonical key-first JSONL composition rule; remove the dead `pattern` variable; remove `|| true`.

#### Major

- 🟡 **Usability / Code Quality**: Hand-emitting 7-field TSV with embedded base64 and inline JSON is hostile to first-time authors
  **Location**: Migration-author surface (lines 84–126); Implementation Approach — Wire protocol
  Authors must base64-encode display blocks, hand-write `extras_json` (and the runner then string-merges it), keep TAB-counts correct across 7 positional fields, and escape literal TAB/newline/`\` in any field. With ~few interactive migrations projected for the corpus's lifetime, every author will be a first-timer. **Fix**: provide `harness_emit_transformation key=v path=v anchor=v proposed=v predicate_value=v display=$'multi\nline'` and `harness_extras_set k v` helpers; the wire format becomes an implementation detail.

- 🟡 **Architecture / Correctness / Code Quality / Safety / Usability**: String-slice JSON merge for `extras_json` is fragile across multiple edge cases
  **Location**: Phase 4 §8
  Stripping the leading `{` from `extras_json` and splicing in framework fields produces invalid JSON for `{}` (trailing comma), `{ "k":"v" }` (whitespace), nested-brace string values, and admits silent key collisions with framework-mandatory fields. Authors hand-writing JSON in bash will produce malformed records under realistic inputs; corruption surfaces on next-run resume parsing, not at emit time. **Fix**: either require migrations to emit extras as `key=value` TSV and have the runner compose the JSON, or provide a `harness_extras_set` helper that accumulates pairs and emits canonical JSON; ban migration-declared keys that collide with framework keys.

- 🟡 **Architecture / Compatibility**: bash 4+ requirement (`coproc`, `declare -A`) not declared; macOS bash 3.2 floor is unaddressed
  **Location**: Phase 3 §4 (coproc), Phase 6 §6 (declare -A)
  The existing runner works on bash 3.2; the plan adds two bash-4-only constructs with no version assertion or SKILL.md note. macOS users on stock `/bin/bash` will see opaque syntax errors the first time an opt-in migration ships. **Fix**: add an explicit bash 4+ check at runner startup with a clear error; document the floor in SKILL.md and CHANGELOG.

- 🟡 **Correctness / Code Quality**: Validation re-prompt via `read_decide_and_recurse` uses bash recursion
  **Location**: Phase 5 §5
  Bash has no tail-call optimisation; recursion interacts with `set -e`, locals, and IFS state unpredictably; a `--decisions` file with many invalid entries can grow the stack. **Fix**: explicit `while true` loop inside `harness_run`'s per-transformation handler — read DECIDE, validate, break on success, emit VALIDATE_ERR and continue on failure.

- 🟡 **Correctness**: Coproc fd lifecycle is not closed between iterations; second migration may collide
  **Location**: Phase 3 §4
  `coproc MIG { ... }` per migration with no explicit `exec {MIG[0]}<&- {MIG[1]}>&-` after `wait` risks the second migration hitting bash's one-coproc-per-shell limit (intermittent on different bash versions). **Fix**: explicit fd close after wait; add an integration test that runs two interactive migrations in one invocation.

- 🟡 **Correctness**: Sed-style JSONL field extraction in `build_resume_state_file` is unsafe for values containing escape chars or commas
  **Location**: Phase 6 §5
  Naive regex `"proposed_value":"([^"]*)"` truncates at the first embedded `\"`, returning a wrong value; resume comparison then sees spurious drift and re-prompts already-decided transformations. ADR-0038's prose-derived values will contain JSON escape characters. **Fix**: implement a real JSON-unescape pass in the extractor with tests for each escape sequence, or constrain framework-mandatory fields to a subset that requires no escaping (and validate at write time).

- 🟡 **Test Coverage / Correctness**: SIGKILL durability test only proves happy-path persistence
  **Location**: Phase 2 §1 + Phase 6 §1
  The proposed test SIGKILLs the writer *after* `printf … >> file; sync` returns — i.e. after the data is durable. AC-9 requires durability under crash *during* the write. **Fix**: race the SIGKILL — loop write+random-delay-then-kill — and assert every line present is well-formed JSON.

- 🟡 **Test Coverage**: Concurrent-write test does not exercise PIPE_BUF boundary or large records
  **Location**: Phase 2 §1
  No size parametrisation; long lines (with base64 display content) above PIPE_BUF can silently interleave. **Fix**: parametrise the concurrent test over multiple line sizes spanning PIPE_BUF; document the size limit in the helper if one cannot be enforced.

- 🟡 **Test Coverage**: Key-removal tests miss prefix-collision and JSON-escape edge cases
  **Location**: Phase 2 §2
  No tests for keys containing `"`/`\`, no test for prefix collisions, no test for substring matches in adjacent records' extras fields. **Fix**: add explicit prefix-collision and substring-in-other-field tests; round-trip a key with embedded quotes through writer and remove.

- 🟡 **Test Coverage**: Incremental-write ordering test mechanism is under-specified
  **Location**: Phase 6 §1
  "Fixture that pauses (reads from a synchronisation FIFO) between transformations" doesn't name where the FIFO read happens — the harness emits transformations in a tight loop with no natural pause. **Fix**: specify the synchronisation point: the runner emits a sentinel log line after each `atomic_jsonl_append` returns and before reading the next frame; the test asserts on that sentinel.

- 🟡 **Test Coverage**: Per-callback contract violations not tested
  **Location**: Testing Strategy section + all phases
  No tests for `migration_emit_transformations` producing wrong field count, `migration_evaluate_predicate` polluting stdout, `migration_validate_edit` writing errors to stdout, `migration_apply_decision` failing mid-loop, `migration_session_log_path` returning an unwritable path. **Fix**: add a contract-violation test section with one fixture per callback × violation; route callback stdout through a captured stream so wire-protocol stdout is isolated.

- 🟡 **Test Coverage / Safety**: Resume edge cases beyond AC-10 not covered
  **Location**: Phase 6
  No tests for partial JSONL line at EOF (SIGKILL mid-printf), session log records for keys the current emission no longer produces, unknown `outcome` values, `user_value` containing `|` (which collides with the resume-state pack format), or partial-apply state where RECORDED was written but `apply_decision` failed. **Fix**: add tests for each; switch the resume-pack format from `|`-separated to TAB+escape_field.

- 🟡 **Test Coverage**: `MIGRATION_PROTOCOL_LOG` concurrent appends may interleave
  **Location**: Phase 3 §6
  Both sides `>>` to the same file without locking; long PROMPT frames could interleave and produce flaky protocol assertions. **Fix**: split into per-side log files (`_RUNNER`, `_MIGRATION`) and merge at the test site, or `flock`-guard the shared append.

- 🟡 **Safety / Compatibility**: AC-1 regression net is behavioural, not byte-identical
  **Location**: Phase 1 §1
  Existing assertions verify outcomes/substrings, not byte-identical bytes/stdout/stderr. A subtle reformat in a migrated artefact passes the suite. **Fix**: capture full pre-change `diff -r` snapshot of the migrated tree (and stdout+stderr with timestamp redaction) as a CI-enforced AC-1 artefact.

- 🟡 **Safety**: Dirty-tree pre-flight treats in-flight session log as discardable
  **Location**: Migration Notes
  A user encountering "uncommitted changes in `.accelerator/`" on a clean repo with only an in-flight session log will reasonably `jj abandon` and lose hours of decisions. **Fix**: emit a distinct message for session-log files specifically (`Found in-flight migration session: <id>. Resume with /accelerator:migrate to continue, or rm <path> to discard`).

- 🟡 **Safety**: DRIFT detection compares post-mutation `proposed_value`, so partial-apply crashes look like drift
  **Location**: Phase 6 §6
  A prior run killed between mutation and RECORDED produces a state where re-entry sees drift and re-prompts; a non-idempotent `apply_decision` then double-applies. **Fix**: tie to the apply-then-record ordering fix (critical finding) — or have the harness pass the *recorded* `proposed_value` back through matching rather than recomputing from the live artefact.

- 🟡 **Compatibility**: Session-log JSON format-stability is at risk from naive composition
  **Location**: Phase 4 §8
  No escaping for `"`, `\`, tab, newline in the `transformation_key` write path; the remove helper then `grep -F`s the un-escaped raw key. Cross-version forward-compat is also unaddressed. **Fix**: specify the exact escape set, apply it on both write and match paths, add a `schema_version: 1` field from day one.

- 🟡 **Compatibility**: Cross-version session-log compat when plugin upgrades mid-session is unaddressed
  **Location**: Migration Notes + Phase 6
  No `schema_version`; a session log committed under plugin N may be read by N+1 with different schema, silently misinterpreting records. **Fix**: include `schema_version` in every record from day one; runner warns on unknown versions.

- 🟡 **Usability**: `--decisions <file>` testing-only flag exposed on user-facing CLI
  **Location**: Phase 1 §2
  Users running `--help` see this alongside supported flags; safety-relevant prompts can be silently bypassed by anyone who finds it. **Fix**: move behind `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var (precedent exists: `ACCELERATOR_MIGRATE_FORCE`).

- 🟡 **Usability**: Validation error forces full re-prompt with no edit-state preserved
  **Location**: Phase 5
  After `edit foo` is rejected, the user must retype the verb and value from scratch. For multi-token edits (ADR-0038's identifiers), this is high friction. **Fix**: pre-fill the re-prompt input with the prior value via `read -e -i "$prior_value"`, or at minimum echo the rejected value back for copy-paste correction.

- 🟡 **Usability**: Session-log path discoverability, lifecycle, and cleanup are unspecified
  **Location**: Phase 4 §7 + Migration Notes
  Users won't see the path mentioned anywhere, won't know when it's cleaned up, and won't be told what's dirty when pre-flight blocks them. **Fix**: print the path in the prompt banner; state the cleanup policy in SKILL.md (keep as audit artefact, mirror `migrations-applied`); add to `## State file format` section.

- 🟡 **Code Quality**: Duplicated `escape_field`/`unescape_field` across two libraries (or layering inversion)
  **Location**: Phase 3 §§4–5
  Plan says "source-or-dup" — either choice degrades maintainability and risks silent escaping drift. **Fix**: extract to a third shared file (`scripts/interactive-protocol.sh`) sourced from both sides, mirroring how `atomic-common.sh` is shared today.

- 🟡 **Code Quality**: `set -euo pipefail` interaction with coproc and child-exit detection is under-specified
  **Location**: Phase 3 §4
  Failure-mode matrix is not documented: child-exit-before-DONE, SIGPIPE on write to closed coproc, validator subprocess failure, decisions-file-exhausted mid-flow, `wait` non-zero. **Fix**: document each failure's resulting exit code, stderr message, and ledger effect; consider scoped `set +e` around the read loop; add a test that kills the coproc mid-loop.

- 🟡 **Code Quality**: Seven-field positional TSV is at the readability cliff
  **Location**: Implementation Approach — Wire protocol
  Adding a field is a coordinated change across runner, harness, fixtures, and protocol-log assertions. **Fix**: move to `KEY=value` field encoding within each frame, or centralise parsing in a single `parse_prompt_frame` function that returns named locals.

- 🟡 **Documentation**: Timestamp-only redaction insufficient for byte-identical drift test
  **Location**: Phase 7 §3
  Absolute tempdir paths, the session-log path embedded in INIT, resume-state temp paths will all vary between runs. **Fix**: enumerate the full redaction set (timestamps + paths under `$TMPDIR_BASE` + session-log path); run in a deterministic sandbox; add a 10-run consecutive-pass success criterion.

- 🟡 **Documentation**: Documentation lands only in Phase 7; intermediate phases ship undocumented user-visible surface
  **Location**: Phases 1–6 vs Phase 7
  `--decisions` (Phase 1) and the new helpers (Phase 2) ship without doc updates. **Fix**: either add doc-update items to Phases 1 and 2, or revise the "independently shippable" claim.

- 🟡 **Documentation**: "Worked example" content under-specified for the self-sufficiency goal
  **Location**: Phase 7 §1
  No minimum coverage — does the example exercise edit, skip, validation re-prompt, non-trivial validator, source-drift? **Fix**: enumerate the worked example's minimum coverage: ≥1 predicate=true + ≥1 predicate=false transformation; transcript shows accept + edit + skip + at least one VALIDATE_ERR cycle; extras_json carries one non-trivial value; the apply callback mutates a realistic artefact shape.

#### Minor

- 🔵 **Architecture**: `APPLIED` frame conflates mechanical-route and resume-already-decided semantics — split into `APPLIED` and `RESUMED_APPLIED`, or collapse `RESUMED_SKIP` into `APPLIED` for symmetry.
- 🔵 **Architecture**: No explicit protocol state machine documented — add a transition table to SKILL.md or the plan's Implementation Approach.
- 🔵 **Architecture**: Session-log write ownership inverted from ADR-0037 §3 wording without acknowledgement — add an explicit one-sentence note.
- 🔵 **Architecture / Code Quality**: Bare `sync` is process-wide on Linux, weak/imprecise primitive — re-evaluate per the durability finding.
- 🔵 **Architecture / Code Quality**: Test-only `MIGRATION_PROTOCOL_LOG` woven into production runtime — centralise via `emit_frame`/`read_frame` wrappers.
- 🔵 **Architecture**: Phase independence claim is true only because no production migration opts in — document the implicit Phase-1-through-6 dependency for 0070.
- 🔵 **Correctness**: Dead `pattern` variable in `atomic_jsonl_remove_by_key` — delete.
- 🔵 **Correctness**: Unescape ordering for `\\`, `\t`, `\n` is order-sensitive but unspecified — define a single-pass state machine; add round-trip tests.
- 🔵 **Correctness**: RESUMED line uses `|` separator but proposed/user values may contain `|` — switch to TAB+escape_field consistent with wire.
- 🔵 **Correctness**: EOF-before-DONE diagnostics swallow the migration's stderr — capture last N stderr lines into the error message.
- 🔵 **Correctness**: Decisions-file accounting against VALIDATE_ERR re-prompts is ambiguous — runner emits an informational stderr line per consumed line.
- 🔵 **Test Coverage**: Doc-drift extraction has no guard against empty marker regions — pre-assert non-empty + sentinel-string presence.
- 🔵 **Test Coverage**: escape_field/unescape_field round-trip not unit-tested — add round-trip tests for every escape-significant character.
- 🔵 **Test Coverage**: `--decisions` file edge cases not tested — add a robustness block (oversupply, malformed, missing, CRLF).
- 🔵 **Test Coverage**: Header-marker variant matching not tested — table of variant fixtures with explicit pass/fail expectations.
- 🔵 **Test Coverage**: Mid-stream FAIL (after some RECORDED frames) not tested — add a Phase 6 test that emits FAIL after 2 of 5 decisions.
- 🔵 **Test Coverage**: Test-isolation risk from coproc lifecycle not addressed — add a smoke test that loops the runner 10× in one shell session.
- 🔵 **Safety**: Resume-state `mktemp` leaks on SIGKILL — use a deterministic path under `.accelerator/state/` and unlink explicitly.
- 🔵 **Safety**: FAIL waits on coproc pid without timeout — wrap in bounded timeout with SIGTERM/SIGKILL escalation.
- 🔵 **Compatibility**: Header-marker convention collides with `set -euo pipefail` line position — tighten SKILL.md wording and ship an explicit template.
- 🔵 **Compatibility**: Argument-parsing rewrite subtly changes precedence for combined flags — document the flag-order in a code comment.
- 🔵 **Code Quality**: Shellcheck cleanliness is a floor not a ceiling — downgrade from phase Success Criteria to a precondition; replace with behavioural criteria.
- 🔵 **Code Quality**: Pipe-delimited packed values in `RESUME[$key]` re-invent parsing — use three parallel associative arrays.
- 🔵 **Code Quality**: Hand-rolled JSON parsing in AC-5 test duplicates the runner's hand-rolled composition — pipe through `python3 -m json.tool` as a parse precondition.
- 🔵 **Usability**: Decision-line syntax (`accept` / `edit <value>` / `skip`) not discoverable from the prompt — append `[accept | edit <new-value> | skip] > ` to the prompt.
- 🔵 **Usability**: Display rendered to stderr conflates user-facing UI with diagnostic noise — render to stdout, reserve stderr for runner diagnostics.
- 🔵 **Usability**: Bash-script-as-data-emitter pattern is heavyweight for a small projected corpus — flag the manifest alternative in "What We're NOT Doing"; defer decision until the second interactive migration.
- 🔵 **Usability**: Migration-author mistakes surface as silent corruption or cryptic protocol errors — add defensive validation at each harness boundary with actionable errors.
- 🔵 **Usability**: SKILL.md self-sufficiency Manual Verification is unreliable when performed by the plan author — route through external review or to 0070's author as the natural fresh reader.
- 🔵 **Usability**: VALIDATE_ERR message format inconsistent across migration authors — define a convention; consider `harness_reject "<message>"` helper.
- 🔵 **Documentation**: Existing SKILL.md cross-reference to ADR-0023 already broken (`ADR-0023-migration-framework.md` vs actual `ADR-0023-meta-directory-migration-framework.md`) — fix while in the file.
- 🔵 **Documentation**: Runner-level decisions rationale not flagged in SKILL.md — add framing line: "runner-level, not ADR-0037 §§1-4 framework primitives; disagreement from a second consumer is the signal to promote to a supplementary ADR per ADR-0037 §5."
- 🔵 **Documentation**: Wire-protocol frames have no permanent home for maintainers — header docstring at the top of `interactive-harness.sh` enumerating frames, directions, fields, and field-escaping rules.
- 🔵 **Documentation**: `shellcheck` invocation undocumented despite gating multiple phases — pin the exact `mise exec --` invocation; or add a top-level task.
- 🔵 **Documentation**: Field-escaping rules and key-fallback synthesis need verbatim documentation, not narrative prose — include literal escape table and a one-line example in SKILL.md.

#### Suggestions

- 🔵 **Safety**: No kill switch / rate cap on prompt loop — add `--max-prompts <n>` (default ~10,000) and a progress indicator `[42/140] prompting…`.
- 🔵 **Safety**: AC-1 byte-identical verification is asserted, not demonstrated — explicit pre/post `diff -r` snapshot test in Phase 1.

### Strengths

- ✅ Strong AC-1 regression posture: the existing test-migrate.sh suite is locked in *before* any runner change; opt-in marker absent from every bundled migration.
- ✅ Clean module decomposition: runner-side `interactive-lib.sh`, migration-side `interactive-harness.sh`, wire protocol as the only crossing surface; child-process isolation via coproc.
- ✅ Wire protocol confines JSON to the on-disk session log — no JSON parser needed on the wire, consistent with the no-deps constraint.
- ✅ Phase independence and TDD discipline are designed-in: each phase ships green and gated by either `--decisions` (test-only) or the `# INTERACTIVE: yes` header (absent from bundled migrations until 0070).
- ✅ Documentation drift is structurally prevented (AC-13): worked example extracted with markers and CI-diffed against fixture output.
- ✅ Extensibility posture aligns with ADR-0037 §5: new frame types can be added without re-architecting the protocol layer.
- ✅ Mechanical-path preservation via header-gated `if/else` keeps the existing `bash "$f" >"$STDOUT_FILE" 2>&1` invocation byte-identically untouched.
- ✅ Reuses existing primitives (atomic_write, atomic_append_unique, `.accelerator/state/`, MIGRATION_RESULT precedent, header-comment metadata convention) rather than re-inventing.
- ✅ Session-log path under `.accelerator/state/` inherits dirty-tree pre-flight protection for free.
- ✅ Forbidding new dependencies (no jq, no Python) keeps the trust surface inside the existing shell-only audit boundary.
- ✅ Full-completion ledger append is explicitly gated on both `DONE` received AND `wait $pid` returning 0.

### Recommended Changes

Ordered by impact. Many findings cluster on a small number of underlying fixes; addressing the top-tier changes closes findings across multiple lenses.

1. **Replace `atomic_jsonl_append`'s atomicity story** (addresses: critical PIPE_BUF claim; major SIGKILL test; major PIPE_BUF concurrent test). Either (a) `flock`-guard the append, or (b) read-existing + concat + temp-then-rename via the existing `atomic_write`. Drop the PIPE_BUF docstring; document the actual guarantee. Race-the-SIGKILL durability test loops writes with randomised kill delays and asserts every line on disk is well-formed JSON.

2. **Replace `atomic_jsonl_remove_by_key`'s match logic** (addresses: critical substring-match bug; major key-removal edge cases; minor dead `pattern` variable). Switch to an `awk`-based or anchored-regex match against the first field, with a canonical write rule that puts `transformation_key` first. Remove the dead `pattern` variable. Remove `|| true`. Add tests for prefix collisions and substring-in-other-field scenarios.

3. **Invert apply-vs-record ordering to write-ahead-log discipline** (addresses: critical Safety ordering window; major DRIFT-vs-partial-apply ambiguity; major resume edge-case coverage gap). Runner persists the JSONL record first, signals harness, harness then calls `migration_apply_decision`. Add explicit tests killing between the two phases.

4. **Replace JSON-merge-by-string-slicing with structured composition** (addresses: major fragility across Architecture/Correctness/Code Quality/Safety/Usability lenses; minor schema-versioning compat). Either have migrations emit `extras_json` as `key=value` TSV pairs and let the runner own all JSON construction, or provide a `harness_extras_set k v` helper that accumulates pairs and emits canonical JSON. Add `schema_version: 1` to every record. Ban migration-declared keys colliding with framework-mandatory keys.

5. **Fix Compatibility blockers in Phase 2 and Phase 3** (addresses: two critical Compatibility findings). Change Phase 2 §§1–2 from "(new)" to "extend existing `scripts/test-atomic-common.sh`". Add a coproc-driver branch for `MIGRATION_RESULT: no_op_pending` so the soft-defer contract is honoured for interactive migrations too.

6. **Address the bash 4+ floor** (addresses: major Architecture/Compatibility). Add an explicit bash version assertion at runner startup (only when the interactive path is entered, so mechanical migrations remain bash-3.2-compatible). Document the floor in SKILL.md and CHANGELOG. Note that 0070 cannot ship until this is in place.

7. **Provide author-facing helpers to eliminate the TSV/base64/JSON hand-encoding surface** (addresses: major Usability/Code Quality hand-authoring; major TSV readability cliff; major migration-author mistakes surfacing as silent corruption). Add `harness_emit_transformation key=v path=v anchor=v proposed=v predicate_value=v display=$'multi\nline'`, `harness_extras_set k v`, and `harness_reject "<message>"` to `interactive-harness.sh`. Wire format becomes an internal contract authors do not see.

8. **Replace recursive validation re-prompt with an explicit loop** (addresses: major Correctness/Code Quality recursion). `while true; do read decide; if validate then apply; emit_recorded; break; else emit_validate_err; continue; fi; done`. Symmetric loop on the runner side.

9. **Move `--decisions` behind an env var** (addresses: major Usability CLI leak). `ACCELERATOR_MIGRATE_DECISIONS_FILE`, mirroring existing env-var precedent. Document in the runner header comment, not in user-facing SKILL.md.

10. **Tighten the AC-1 regression net to byte-identical** (addresses: major Compatibility/Safety AC-1 strength). Add a Phase 1 snapshot test: capture full `diff -r` of the migrated tree (and stdout+stderr with timestamp redaction) under the pre-change runner; assert empty diff under the post-change runner.

11. **Address dirty-tree pre-flight UX for in-flight session logs** (addresses: major Safety silent decision loss). Detect session-log files in the pre-flight and emit a distinct message naming the resume command and the discard command.

12. **Fix coproc fd lifecycle and FAIL-wait timeout** (addresses: major Correctness; minor Safety hang). Explicit fd close after `wait`; wrap FAIL-wait in a 30s timeout with SIGTERM/SIGKILL escalation; add a multi-interactive-migration integration test.

13. **Define the wire-protocol state machine** (addresses: minor Architecture state machine; minor Documentation maintainer-facing home). Add a transition table to a header docstring in `interactive-harness.sh` (maintainer-facing) and a one-paragraph summary in SKILL.md (author-facing — what they can rely on, not the frames themselves).

14. **Extract shared escape/unescape into a single source-of-truth file** (addresses: major Code Quality duplication; minor Correctness ordering). `scripts/interactive-protocol.sh` sourced from both sides; specify a single-pass unescape algorithm; add round-trip tests.

15. **Strengthen Phase 7 documentation** (addresses: major Documentation drift redaction; major worked-example under-specification; major docs-only-in-Phase-7). Enumerate the full redaction set for the drift test; specify worked-example minimum coverage; add doc-update items to Phases 1 and 2 (or revise the "independently shippable" framing); fix the broken ADR-0023 cross-reference; add the runner-level-decisions rationale framing.

16. **Add `schema_version` to session-log records and document cross-version upgrade behaviour** (addresses: major Compatibility cross-version; major Compatibility format-stability). From day one.

17. **Address minor findings as a clean-up sweep**: pre-fill VALIDATE_ERR re-prompt with prior value (`read -e -i`); render display to stdout, not stderr; print session-log path in the prompt banner; specify cleanup policy; split `APPLIED` into mechanical vs resumed variants; specify decision-line syntax inline at the prompt; downgrade `shellcheck` from Success Criteria to precondition with a pinned invocation.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan establishes a clean separation between mechanical and interactive migration paths via opt-in header detection, with a coproc-based process model and a TAB-framed wire protocol that keeps JSON confined to the on-disk session log. The architectural decomposition (runner ↔ harness library ↔ migration callbacks) is sound and largely preserves ADR-0023's mechanical-default invariant. However, several architectural concerns deserve attention before implementation: a missing protocol state machine, fragile JSON string-splicing across the runner/migration trust boundary, a substring-grep approach to JSONL key removal that risks false matches, conflation of two distinct semantics in the `APPLIED` frame, and a bash `coproc` dependency whose portability ceiling needs explicit acknowledgement.

**Strengths**: mechanical-default preservation; clean module boundary; JSON confined to disk; phase independence + TDD discipline; doc drift structurally prevented; extensibility per ADR-0037 §5.

**Findings**: JSONL key removal fragile substring grep (major/high); textual JSON merge fragile (major/high); bash coproc portability ceiling unacknowledged (major/medium); APPLIED frame conflation (minor/high); no protocol state machine (minor/high); session-log write ownership inverted vs ADR-0037 (minor/high); bare sync semantics (minor/medium); test-only protocol logging in production (minor/medium); duplicated escape helpers (minor/medium); phase independence claim partial (minor/low).

### Correctness

**Summary**: The plan covers ADR-0037 and decomposes work into independently shippable phases with strong TDD discipline, but several correctness claims at the wire/IPC/persistence boundary are overstated or incorrect. The most serious issues are SIGKILL/atomicity guarantees for `atomic_jsonl_append` (PIPE_BUF doesn't apply to regular-file writes on macOS), substring-match `grep -v -F` removal that can match the wrong records, and brittle JSON-merge-by-string-concatenation.

**Strengths**: clear state-machine spec; mechanical-path preservation via existing suite; TSV-on-wire / JSON-only-on-disk split; resume + drift specified concretely; ordering + key-schema invariants spelled out.

**Findings**: atomic_jsonl_append PIPE_BUF/atomicity claim wrong (critical/high); grep -v -F substring removes wrong records (critical/high); JSON-merge invalid for {}/whitespace/collisions (major/high); SIGKILL test only proves happy-path (major/high); recursive validation re-prompt (major/high); coproc fd lifecycle (major/medium); sed JSONL extraction unsafe for escapes (major/medium); dead `pattern` variable (minor/high); escape ordering unspecified (minor/medium); RESUMED line `|` vs TAB inconsistency (minor/medium); EOF-before-DONE diagnostics swallow stderr (minor/medium); decisions file exhaustion ambiguity (minor/low).

### Code Quality

**Summary**: Substantial bash complexity (coproc, 7-field TSV with base64 + JSON, duplicated escape helpers, string-slice JSON merge) under `set -euo pipefail` semantics that interact badly with coproc child-exit and read EOF. Migration-author surface is reasonable; implementation surface is at the readability cliff.

**Strengths**: clean phase decomposition; well-factored callback surface; existing primitives reused; TDD discipline per phase; doc-drift CI test.

**Findings**: string-slice JSON merge fragile (major/high); duplicated escape helpers (major/high); 7-field TSV at readability cliff (major/high); set -euo pipefail + coproc under-specified (major/high); grep-based JSONL key matching brittle + dead code (major/medium); recursion in validation re-prompt (major/medium); test-only protocol log couples production (minor/high); shellcheck floor not ceiling (minor/high); pipe-delimited packed values in RESUME (minor/medium); sync() after every append (minor/medium); hand-rolled JSON parsing in tests (minor/medium).

### Test Coverage

**Summary**: TDD-disciplined test scaffolding with one test per AC and the existing ~80-assertion suite as the AC-1 regression net. However, several proposed tests prove weaker properties than their AC text requires (SIGKILL durability, incremental-write ordering), and substantial gaps remain in per-callback contract validation, resume edge cases, escape round-trips, and atomic-helper correctness under boundary conditions.

**Strengths**: existing ~80-assertion suite as AC-1 regression net; TDD discipline up-front; each AC has a named test; MIGRATION_PROTOCOL_LOG enables wire-level assertion; --decisions decouples interactive tests from TTY; doc-drift CI test; separate test file for new helpers.

**Findings**: SIGKILL durability test only proves happy-path (major/high); concurrent-write test misses PIPE_BUF boundary (major/high); key-removal misses prefix/escape cases (major/high); incremental-write ordering test under-specified (major/high); per-callback contract violations untested (major/medium); resume edge cases beyond AC-10 (major/medium); MIGRATION_PROTOCOL_LOG concurrent interleave (major/medium); doc-drift extraction empty-marker guard (minor/high); escape_field round-trip not unit-tested (minor/medium); --decisions edge cases (minor/high); header-marker variants (minor/medium); mid-stream FAIL untested (minor/medium); coproc lifecycle test-isolation (minor/low).

### Safety

**Summary**: Mechanical-path preservation, VCS-revert recovery, and incremental write are real safety wins. However, durability/atomicity for the new session log is built on shaky technical claims; apply-then-record ordering creates a window where artefact mutation can outlive its session-log record, breaking the resumability invariant; the dirty-tree pre-flight treats in-flight session logs as discardable dirt.

**Strengths**: mechanical-path preservation non-negotiable; VCS-revert preserved as rollback; resumability incremental-write; coproc child isolation; full-completion ledger gated on DONE + wait==0; no new dependencies.

**Findings**: apply-then-record ordering window (critical/high); PIPE_BUF/sync wrong primitive (major/high); grep -v -F + || true masks errors (major/high); dirty-tree pre-flight treats session log as discardable (major/high); DRIFT detection misclassifies partial-apply as drift (major/medium); resume-state mktemp leak (minor/high); FAIL waits with no timeout (minor/medium); JSON-merge corrupts session log (minor/medium); no kill switch on prompt loop (suggestion/medium); AC-1 byte-identical asserted not demonstrated (suggestion/high).

### Compatibility

**Summary**: Largely additive; preserves ADR-0023 by gating new behaviour behind an opt-in header marker; `--decisions` is additive with no external consumer impact. However, two critical compatibility breakages (file-already-exists conflict in Phase 2; MIGRATION_RESULT no_op_pending interaction broken by coproc driver), plus a bash 4+ floor that isn't declared, plus session-log format-stability gaps.

**Strengths**: --decisions additive; header-marker detection anchored and case-sensitive; mechanical/interactive dispatch preserves bash-call boundary verbatim; session log under .accelerator/state/ inherits clean-tree protection; no new dependencies.

**Findings**: scripts/test-atomic-common.sh already exists (critical/high); MIGRATION_RESULT: no_op_pending interaction silently broken (critical/high); AC-1 regression net behavioural not byte-identical (major/medium); bash 4+ floor unaddressed (major/high); session-log JSON format-stability at risk (major/medium); cross-version session-log compat unaddressed (major/medium); header marker line 3 vs set -e collision (minor/high); arg parsing precedence (minor/medium).

### Usability

**Summary**: Preserves the mechanical path; the migration-author surface is sensibly scoped at five callbacks. However, sharp edges in the developer experience: hand-emitting 7-field TSV with base64 + inline JSON, prompt UX on stderr only with no inline help, full re-prompt on validation error with no edit-state preservation, --decisions exposed in the user-facing CLI.

**Strengths**: small callback surface; mechanical path zero new load; opt-in marker follows DESCRIPTION precedent; doc-drift CI test guarantees doc accuracy; phase ordering supports incremental ship; reuses .accelerator/state/.

**Findings**: TSV with 7 fields hostile to hand-authoring (major/high); extras_json string-merge footgun (major/high); --decisions in user-facing CLI (major/high); validation re-prompt no edit-state (major/high); session-log path discoverability/lifecycle (major/medium); decision-line syntax not in prompt (minor/high); display rendered to stderr (minor/medium); bash-script-as-data-emitter heavyweight (minor/medium); author mistakes surface as silent corruption (minor/medium); SKILL.md self-sufficiency Manual Verification unreliable (minor/medium); VALIDATE_ERR message format inconsistent (minor/low).

### Documentation

**Summary**: Phase 7 SKILL.md update + CI-asserted worked example is the right end-state. But docs land only in Phase 7 despite Phases 1–6 shipping user-visible surface; timestamp-only redaction in drift test is insufficient for true byte-determinism; worked example scope under-specified; several cross-referencing/discoverability gaps; existing broken ADR-0023 cross-reference not fixed.

**Strengths**: CI-asserted worked example transcript drift test; protocol frames correctly omitted from SKILL.md; SKILL.md is the right home for runner-level decisions; concrete subsection enumeration; ADR cross-references explicitly required; Manual Verification names the self-sufficiency claim explicitly.

**Findings**: timestamp-only redaction insufficient (major/high); docs land only in Phase 7 (major/high); worked example under-specified for self-sufficiency (major/medium); existing ADR-0023 cross-reference broken (minor/high); runner-level decisions rationale not flagged in SKILL.md (minor/high); wire-protocol no permanent home (minor/medium); shellcheck invocation undocumented (minor/medium); field-escaping rules need verbatim documentation (minor/medium); self-sufficiency check needs external reviewer (minor/medium).


---

## Re-Review (Pass 2) — 2026-05-30

**Verdict:** REVISE

The revisions substantively address every prior finding — 4 of 5 criticals fully resolved, 1 partially resolved (with a new critical implementation bug arising from the fix); 27 majors largely closed; ~30 minors largely closed; cross-cutting themes about persistence helpers, JSON composition, write-ahead-log ordering, bash 4+ floor, AC-1 byte-identical, and documentation strategy all addressed at the right architectural level. The write-ahead-log inversion (RECORDED → APPLY → APPLIED_CONFIRM) is end-to-end specified and tested; jsonl_compose_record replaces string-slice JSON merging with structured composition; flock+temp-then-rename replaces the wrong PIPE_BUF claim; the bash 4+ floor is gated at the interactive-path entry; AC-1 is now byte-identical via a checked-in snapshot test; SKILL.md docs land alongside their phase rather than only in Phase 7.

However, three NEW critical implementation bugs surface from close reading of the code samples introduced by the fixes: (1) `awk -v p=` interprets backslash escapes in the JSON-escaped prefix and defeats the anchored-prefix removal exactly for keys with escape-significant bytes; (2) `timeout 30 bash -c "wait $pid"` cannot wait on a process that is not a child of the *invoking* shell, so the bounded-timeout escalation is a no-op; (3) `exec {MIG[0]}<&- {MIG[1]}>&-` uses bash's allocate-only `{var}` form which does not expand array elements for closing, so the coproc fd close is a silent no-op. All three are quick fixes — a few lines each — but until they land the related findings (substring-match safety, FAIL-wait safety, multi-migration coproc lifecycle) are not actually closed despite appearing addressed at the prose layer.

Additional new findings of substance: a MAJOR Correctness issue that the bash 4+ assertion cannot fire because `coproc`/`declare -A` fail at parse-time when `interactive-lib.sh` is sourced on bash 3.2 (the assertion needs to gate the *source* line, not just the call site); a MAJOR Safety hole that the write-ahead-log inversion introduces a symmetric "recorded-but-never-mutated" window (runner crashes between persist and APPLY emission, leaving the session log marking the decision as applied while the artefact was never touched) that is acknowledged in protocol prose but not detected on resume; a MAJOR Compatibility issue that `flock(1)` is not on stock macOS and the link(2) fallback is declared but not specified; and a MAJOR Correctness issue around RESUMED_SKIPPED keys whose predicate has since become mechanical (sticky-skip never re-evaluated).

The plan also has minor internal inconsistencies after the revisions: several narrative sections still reference `--decisions <file>` after the migration to the env var; the wire-protocol table specifies `extras_tsv` but harness pseudocode names the variable `extras_json`; a few Phase 6 / Testing Strategy lines still reference singular `MIGRATION_PROTOCOL_LOG` after the split into per-side files; the schema-version recovery instruction names a `--force-discard-session` flag that is defined nowhere.

### Previously Identified Issues

#### Critical (Pass 1)

- 🔴 **Architecture / Correctness / Safety**: grep -v -F substring match removes wrong records — **PARTIALLY RESOLVED**. The design switched to anchored-prefix awk against a canonical first-field invariant (correct shape); the bash invocation `awk -v p="$prefix"` re-introduces the same class of bug because `-v` interprets backslash escapes, defeating matches for any key containing JSON-escape-significant bytes. See new Critical #1 below.
- 🔴 **Correctness / Safety**: atomic_jsonl_append PIPE_BUF/sync claim wrong — **RESOLVED**. Replaced with flock + temp-then-rename via atomic_write; docstring rewritten; sync dropped. (Residual minor: flock availability — see new Major #4.)
- 🔴 **Safety**: Apply-then-record ordering window — **RESOLVED**. Write-ahead-log inversion specified across wire protocol, harness pseudocode (Phase 4 §6, Phase 5 §5), and runner persistence (Phase 4 §8). Symmetric crash-window tests added (Phase 6 §1). (Residual: persist-before-APPLY-emit window — see new Major #5.)
- 🔴 **Compatibility**: scripts/test-atomic-common.sh already exists — **RESOLVED**. Changed to "extend existing".
- 🔴 **Compatibility**: MIGRATION_RESULT: no_op_pending silently broken — **RESOLVED**. Explicit pre-handshake branch in coproc driver; tests added for pre- and post-handshake cases.

#### Major (Pass 1, by lens)

**Architecture**:
- JSONL key removal substring grep — RESOLVED (anchored awk; canonical first-field invariant); but see new Critical #1.
- Textual JSON merge fragile — RESOLVED (jsonl_compose_record + reserved-key rejection).
- bash coproc portability ceiling — RESOLVED (runtime assertion + Prerequisites + Migration Notes + CHANGELOG); but see new Major #3.

**Correctness**:
- JSON merge invalid for {} / whitespace / collisions — RESOLVED (structured composition).
- SIGKILL durability test only proves happy path — RESOLVED (race-the-crash randomised-timing test + two crash-window tests).
- Recursive validation re-prompt — RESOLVED (explicit while-true loop on both sides).
- Coproc fd lifecycle — PARTIALLY RESOLVED (intent right, code wrong; see new Critical #3).
- Sed JSONL extraction unsafe — RESOLVED (awk JSON-aware extractor).

**Code Quality**:
- String-slice JSON merge — RESOLVED.
- Duplicated escape helpers — RESOLVED (single scripts/interactive-protocol.sh).
- 7-field TSV at readability cliff — RESOLVED (author helpers hide wire format).
- set -euo pipefail + coproc under-specified — PARTIALLY RESOLVED (mechanisms in place; failure-mode matrix still not enumerated as a single artefact).
- grep brittle + dead code — RESOLVED.
- Recursion in validation re-prompt — RESOLVED.

**Test Coverage**:
- SIGKILL durability test (happy path only) — RESOLVED.
- Concurrent-write PIPE_BUF boundary — RESOLVED (parametrised over 100B–64 KiB).
- Key-removal prefix/escape cases — RESOLVED at the test design layer (but blocked by new Critical #1 until the awk -v issue is fixed; the test will reveal it).
- Incremental-write ordering — RESOLVED (FIFO + protocol-log inspection).
- Per-callback contract violations untested — RESOLVED.
- Resume edge cases — RESOLVED (partial line, unknown outcome, orphan, escape-bearing user_value).
- MIGRATION_PROTOCOL_LOG interleave — RESOLVED (per-side files).

**Safety**:
- PIPE_BUF/sync wrong primitive — RESOLVED.
- grep -v -F + || true masks failures — RESOLVED.
- Dirty-tree treats session log as discardable — RESOLVED (distinct named message + decision count).
- DRIFT misclassifies partial-apply — RESOLVED (write-ahead-log invariant means DRIFT now unambiguous; explicit note added).

**Compatibility**:
- AC-1 behavioural not byte-identical — RESOLVED (test-migrate-snapshot.sh).
- bash 4+ floor unaddressed — RESOLVED at the runtime layer; PARTIALLY RESOLVED at the parse-time layer (see new Major #3).
- Session-log JSON format-stability — RESOLVED (schema_version + canonical ordering + shared escape).
- Cross-version session-log compat — RESOLVED (schema_version: 1 from day one); minor cleanup (schema-version recovery references undefined flag — see new Minor below).

**Usability**:
- 7-field TSV hostile to hand-authoring — RESOLVED (author helpers).
- extras_json string-merge footgun — RESOLVED (harness_extras_set + structured composition).
- --decisions in user-facing CLI — RESOLVED (ACCELERATOR_MIGRATE_DECISIONS_FILE env var).
- Validation re-prompt no edit-state — RESOLVED (read -e -i pre-fill).
- Session-log discoverability/lifecycle — RESOLVED (banner + cleanup policy + dirty-tree-aware pre-flight).

**Documentation**:
- Timestamp-only redaction insufficient — RESOLVED (full redaction set + deterministic sandbox + 5-run determinism gate + test-the-test).
- Docs only in Phase 7 — RESOLVED (Phase 1 ships SKILL.md Prerequisites + CHANGELOG entries; pre-flight message is itself user-visible doc).
- Worked example under-specified — RESOLVED (minimum coverage enumerated).

#### Minor + Suggestion (Pass 1)

The following Pass-1 minors are all RESOLVED via the revisions: APPLIED frame conflation (split into MECHANICAL_APPLIED / RESUMED_APPLIED / RESUMED_SKIPPED), no protocol state machine (state-machine docstring in interactive-protocol.sh), session-log write ownership ack, bare sync semantics, test-only protocol log in production (centralised in emit_frame/read_frame), duplicated escape helpers, dead `pattern` variable, unescape ordering, RESUMED-line `|` separator, decisions accounting (informational stderr line), doc-drift extraction empty-marker guard, escape_field round-trip unit tests, --decisions edge cases, header-marker variants, coproc lifecycle test isolation (smoke test), resume-state mktemp leak (deterministic path), FAIL waits no timeout (intent in place; implementation has new Critical #2), JSON merge corruption, pipe-delimited RESUME values (parallel arrays), sync() after every append, hand-rolled JSON parsing in tests, shellcheck floor not ceiling, header marker line 3 collision (template skeleton), arg parsing precedence (moot — flag removed), decision-line syntax not in prompt (inline help), display rendered to stderr (stdout when TTY), author mistakes silent corruption (contract violation tests), SKILL.md self-sufficiency Manual Verification (external 0070-author reviewer), VALIDATE_ERR message format (harness_reject), ADR-0023 broken cross-reference fix, runner-level decisions framing, wire-protocol maintainer home, shellcheck pinned invocation, field-escaping verbatim docs, self-sufficiency external reviewer.

Pass-1 minors that remain **STILL PRESENT** or **PARTIALLY RESOLVED**:
- 🔵 **Architecture**: Phase independence claim does not flag the cross-phase 0070 dependency — PARTIALLY RESOLVED (bash 4+ part captured in Migration Notes).
- 🔵 **Correctness**: EOF-before-DONE diagnostics swallow migration stderr — STILL PRESENT.
- 🔵 **Code Quality**: set -euo pipefail + coproc failure-mode matrix not enumerated as a single artefact — PARTIALLY RESOLVED.
- 🔵 **Safety / Suggestion**: No kill switch / rate cap / progress indicator on prompt loop — STILL PRESENT.
- 🔵 **Test Coverage**: Mid-stream FAIL (after some RECORDED frames) — STILL PRESENT.

### New Issues Introduced

#### Critical

- 🔴 **Correctness / Architecture**: `awk -v p="$prefix"` interprets backslash escapes in the JSON-escaped key prefix, defeating the anchored-prefix match for any key containing `\"` / `\\` / `\n` / `\t` / `\u00XX`.
  **Location**: Phase 2 §4 (atomic_jsonl_remove_by_key implementation snippet)
  Source-drift removal silently no-ops for any key with escape-significant bytes — the stale record persists and the next resume read mis-classifies the transformation. AC-12 source-drift test will fail (or worse, pass-and-hide-the-bug if the test fixture key happens to be plain-ASCII). **Fix**: pass the prefix through the environment: `prefix="$prefix" awk 'BEGIN{p=ENVIRON["prefix"]} index($0,p)!=1{print}' "$target"`. Add an explicit test with key `key-with-"-and-\` so the JSON-escaped prefix contains `\\"` and `\\\\` and the test would catch the regression.

- 🔴 **Correctness / Safety**: `timeout 30 bash -c "wait $pid"` cannot reap the coproc child.
  **Location**: Phase 3 §4 (run_interactive_migration bounded-timeout wait)
  `wait` is a shell builtin that only reaps children of the *invoking* shell. `bash -c "wait $pid"` forks a new bash where `$pid` is not a child; `wait` fails immediately with exit 127, `timeout` exits 0, and the `if ! ... then escalate` branch never fires. The Safety finding on FAIL-wait timeout is not actually closed; a hung migration will hang the runner indefinitely. **Fix**: implement bounded wait inline — `( sleep 30 && kill -TERM $pid 2>/dev/null && sleep 1 && kill -KILL $pid 2>/dev/null ) & watchdog=$!; wait $pid; status=$?; kill $watchdog 2>/dev/null` — or poll with `kill -0 $pid` until exit then `wait` non-blocking.

- 🔴 **Correctness**: `exec {MIG[0]}<&- {MIG[1]}>&-` does not close the coproc fds.
  **Location**: Phase 3 §4 (explicit coproc fd close after wait)
  Bash's `{varname}` redirection form is allocate-only — it assigns a newly-opened fd into `varname` for `<`/`>`/`<>`. It does NOT expand an existing variable's value for `<&-`/`>&-`. Bash parses the line as redirections to fds literally *named* `MIG[0]` and `MIG[1]`, which is semantically not what the plan intends; `2>/dev/null || true` swallows any diagnostic, so the close is a silent no-op. The Correctness finding on coproc fd lifecycle is not closed; the 10-back-to-back multi-migration smoke test will fail on the second iteration. **Fix**: `eval "exec ${MIG[0]}<&- ${MIG[1]}>&-"` (array-value expansion via eval), and drop the `2>/dev/null || true` so real failures surface.

#### Major

- 🟡 **Correctness**: bash 4+ assertion cannot run before bash 3.2 fails to parse coproc/declare -A.
  **Location**: Phase 3 §4 + Migration Notes
  The runtime assertion is gated on entering the interactive path — but `source interactive-lib.sh` (which contains `coproc` + `declare -A`) parses the whole file before any function runs. On bash 3.2 the source itself fails with a syntax error, regardless of whether the runner intended to enter the interactive path. **Fix**: guard the `source interactive-lib.sh` line in run-migrations.sh behind `[ "${BASH_VERSINFO[0]:-0}" -ge 4 ]`; have the interactive dispatch check `type -t run_interactive_migration` and emit a clear bash-version error if undefined.

- 🟡 **Safety**: Runner-crash-after-persist-before-APPLY leaves a recorded-but-never-mutated record undetected on resume.
  **Location**: Wire protocol invariant prose + SKILL.md author guidance
  The write-ahead-log inversion correctly prevents "mutated without a record" but introduces the inverse: if the runner persists the JSONL record then crashes before emitting `APPLY`, the next run takes `RESUMED_APPLIED` and explicitly does NOT call `migration_apply_decision`, so the un-mutation is silent and permanent. Acknowledged in protocol prose only for the APPLY→APPLIED_CONFIRM window; the symmetric runner-side window has identical symptoms and is not called out. **Fix**: either (a) verify-then-skip on resume — harness reads artefact at (path, anchor) and only emits `RESUMED_APPLIED` if recorded value matches live state; otherwise routes through prompt; (b) explicit residual-risk note + recovery instruction in SKILL.md; (c) optional `migration_verify_applied` callback.

- 🟡 **Correctness / Compatibility**: `flock(1)` availability not verified at runtime; link(2) fallback declared but not specified.
  **Location**: Phase 2 §3 (atomic_jsonl_append flock dependency)
  flock is not stock macOS; the plan notes the dependency and says "add a Phase 2 task to verify availability or implement the fallback" but neither lands. First-run failure on stock macOS produces opaque `flock: command not found` against a helper the migration author did not write. **Fix**: pick a path — either add a `command -v flock` precondition with a clear remediation message (Homebrew / mise) and ship as a Phase 2 success criterion, or implement the link(2) fallback in this phase with the same tests.

- 🟡 **Correctness**: RESUMED_SKIPPED does not reconcile with a now-false predicate.
  **Location**: Phase 6 §6 (resume-state load and replay)
  A transformation skipped when ambiguous (predicate true → prompted → user skipped) that is now resolved (predicate would be false → mechanical) stays permanently skipped on every future run. The DRIFT path only triggers when `proposed_value` differs. **Fix**: either document as intentional sticky-skip semantics in SKILL.md + runner-level decisions, OR change resume rule so skipped records still call predicate and honour skip only when predicate still fires.

#### Minor

- 🔵 **Architecture**: `harness_emit_transformation` is documented as writing to stdout, but its output actually feeds the harness's own loop (not the runner). The contract is misleading; either reword to "writes to an internal harness stream" or use an explicit harness-private fd.
- 🔵 **Architecture**: Mixed I/O models — `migration_evaluate_predicate` takes TSV on stdin while the other callbacks take positional args. Asymmetry re-leaks the wire format. Suggest positional args + a `HARNESS_EXTRAS[]` associative array.
- 🔵 **Architecture / Code Quality**: `jsonl_json_escape` and `jsonl_compose_record` home file ambivalent ("or a sibling…"). Commit to one location before implementation.
- 🔵 **Architecture**: `extras_tsv` stacks three encoding layers (US/=, TSV-escape, JSON-escape). Document the layering explicitly with a 0x1F-rejection invariant, or collapse to JSON-on-wire for this field.
- 🔵 **Architecture**: Per-prompted-transformation IPC cost is now 4 round-trips (PROMPT→DECIDE→RECORDED→APPLY→APPLIED_CONFIRM). Size this in Performance Considerations; flag future-batch-decide as a documented evolution path.
- 🔵 **Architecture / Documentation**: Phase independence claim still doesn't enumerate the Phases-1–6 dependency for 0070.
- 🔵 **Correctness**: Frame parsing without TAB leaks the type into the field (e.g. `READY` with no field sets `session_log` to the literal "READY"). Validate frame arity at the parse boundary.
- 🔵 **Correctness**: Post-no_op_pending drain loop blocks indefinitely on a buggy migration that hangs after emitting the sentinel. Close runner's writing end of coproc input + bounded timeout.
- 🔵 **Correctness**: Extras key regex `^[a-z][a-z0-9_]*$` enforced at `harness_extras_set` but not on the runner side on receipt — asymmetric defence. Mirror the check in `write_session_record`.
- 🔵 **Correctness**: EOF-before-DONE diagnostics still swallow the migration's stderr. Capture coproc stderr to per-migration tempfile; tail-N into runner stderr on EOF-without-DONE.
- 🔵 **Code Quality**: set -euo pipefail / coproc failure-mode matrix is implied across multiple sections but not enumerated as a single artefact (state-machine docstring is the natural home).
- 🔵 **Compatibility**: `python3 -m json.tool` is a test-time dependency but not declared in plan prerequisites. Either use `mise exec --` consistently or replace with awk-based JSON smoke check.
- 🔵 **Compatibility / Documentation**: Schema-version recovery instruction references `--force-discard-session` flag that is defined nowhere. Either define the flag in Phase 1, or use the documented `rm <session-log-path>` discard command from the Phase 1 §4 pre-flight message.
- 🔵 **Test Coverage**: Mid-stream FAIL after N successful RECORDED frames is not tested (still present from pass 1).
- 🔵 **Test Coverage**: bash 4+ assertion test path under-specified — no way to run the test under bash 3.2 in CI (Linux-default) without committing a bash-3.2 binary or stubbing the version-check function.
- 🔵 **Test Coverage**: `timeout 30` SIGTERM/SIGKILL escalation path not exercised by any named test.
- 🔵 **Test Coverage**: Redaction completeness verified only indirectly via the 5-run same-process determinism gate; doesn't cover cross-machine variance (hostname, locale, TZ).
- 🔵 **Test Coverage**: FIFO-based write-ahead-log ordering test depends on `apply_decision` being called — skip path not exercised by the same mechanism.
- 🔵 **Test Coverage**: External self-sufficiency check (0070 author) is a one-shot subjective human gate with no objective acceptance criterion beyond "~30 minutes" and no recourse if the verifier is already pre-loaded on the ADRs.
- 🔵 **Usability**: bash 4+ runtime error message not pinned with copy-pasteable install commands (Homebrew bash / mise).
- 🔵 **Usability**: Inline help line on every prompt becomes visual noise across long sessions (no "quiet-after-first" mode).
- 🔵 **Usability**: Author API surface (5 callbacks + 5 helpers + harness_run + extras-reset lifecycle + header marker placement + bash 4+ floor) needs a one-page cheat sheet table in SKILL.md.
- 🔵 **Usability**: Session-log banner placement not pinned — risk of being buried below existing runner banner, or rendered for fully-resumed runs where no PROMPT fires.
- 🔵 **Usability**: harness_extras_set auto-clear after emission is a footgun for authors who factor extras-setting into a separate helper (set-once, emit-many pattern silently drops extras after first record).
- 🔵 **Documentation**: Several narrative sections still reference `--decisions <file>` after the env-var migration (Current State Analysis, Desired End State, Independence between phases, Phase 4–6 test sections).
- 🔵 **Documentation**: Wire-protocol table specifies `extras_tsv` field name but harness pseudocode uses bash variable `$extras_json` throughout Phase 4 §6 and Phase 5 §5.
- 🔵 **Documentation**: A few Phase 6 / Testing Strategy lines still reference singular `MIGRATION_PROTOCOL_LOG` after the split into per-side files in Phase 3 §6.

### Assessment

The plan has moved from "should not proceed in current shape" (Pass 1) to "structurally sound but with three small implementation bugs in code samples that must be fixed before implementation begins". The architectural decisions are all defensible — the write-ahead-log inversion in particular is a clean resolution of the apply-then-record ordering window — and the test plan is now substantially stronger than before. The remaining critical findings are all narrow implementation defects (awk -v escape, timeout/wait semantics, bash array-fd-close syntax) that a single 20-line edit pass can close. The four new major findings are smaller in scope than the original cluster and address realistic-but-narrower failure modes.

Recommended path: a third pass addressing the three critical implementation bugs (~15 minutes), then the four majors (~30 minutes), then a sweep on the minor cleanups (the `--decisions`/`extras_tsv`/`MIGRATION_PROTOCOL_LOG` consistency, the bash-version-gated source, the flock fallback, the safety hole on persist-before-APPLY). After that the plan can move to **APPROVE**. If you would prefer to land the plan now and address the implementation bugs at the start of Phase 2/3 instead, the residual risk is moderate — the bugs surface immediately on the first relevant test run, so they cannot ship to users.

---

## Re-Review (Pass 3) — 2026-05-30

**Verdict:** APPROVE

All Pass-2 critical findings and the four substantive Pass-2 majors have been addressed by targeted edits:

### Pass-2 findings → resolution

- 🔴 **Correctness / Architecture**: `awk -v p="$prefix"` backslash interpretation → **RESOLVED**. Replaced with `ENVIRON["JSONL_REMOVE_PREFIX"]`; explicit regression-guard test added.
- 🔴 **Correctness / Safety**: `timeout 30 bash -c "wait $pid"` non-child wait → **RESOLVED**. Replaced with inline background watchdog using `sleep 30 && kill -TERM` → `sleep 1 && kill -KILL`, plus foreground `wait $pid || wait_status=$?` and explicit watchdog cancellation.
- 🔴 **Correctness**: `exec {MIG[0]}<&- {MIG[1]}>&-` allocate-only syntax → **RESOLVED**. Replaced with `eval "exec ${MIG[0]}<&- ${MIG[1]}>&-"` (array-value expansion via eval).
- 🟡 **Correctness**: bash 4+ assertion runs after parse-time failure → **RESOLVED**. Moved version gate to the `source interactive-lib.sh` line in `run-migrations.sh`; interactive dispatch checks `type -t run_interactive_migration` and emits a clear error with copy-pasteable Homebrew/mise install commands.
- 🟡 **Safety**: Recorded-but-never-mutated window → **RESOLVED via opt-in detection**. Introduced optional `migration_verify_applied` callback called on the resume path before `RESUMED_APPLIED`; on failure the harness emits DRIFT and re-prompts. Documented as residual risk with VCS-revert recovery path when callback is not declared. Two new Phase 6 tests cover both cases.
- 🟡 **Correctness / Compatibility**: flock availability + python3 dependency → **RESOLVED**. `command -v flock` precondition with actionable error in `atomic_jsonl_append`; link(2) fallback explicitly rejected with rationale; python3 invocations move to `mise exec -- python3` consistently; mise.toml gets a `flock` pin.
- 🟡 **Correctness**: RESUMED_SKIPPED predicate reconciliation → **RESOLVED** as documented sticky-skip semantics. Runner-level-decisions section explains the rationale (user consent preservation), the author guidance (design stable predicates), and the user override (delete the session-log line).

### Pass-2 minor findings → resolution

All Pass-2 documentation/consistency minors addressed:
- ✅ `--decisions` narrative references replaced with `ACCELERATOR_MIGRATE_DECISIONS_FILE`; Phase 4 §Overview adds a shorthand note for test-section readability.
- ✅ `extras_json` → `extras_tsv` in all pseudocode (sed-replaced; verified zero remaining instances).
- ✅ Singular `MIGRATION_PROTOCOL_LOG` references updated to per-side `_RUNNER` + `_MIGRATION` (Phase 6 §§1, 2, 4; Testing Strategy).
- ✅ `--force-discard-session` reference removed; recovery instruction now uses the documented `rm <session-log-path>` discard command from Phase 1 §4. Added schema-version upgrade-policy paragraph.
- ✅ Mid-stream FAIL test fixture added as Phase 5 §7.
- ✅ EOF-without-DONE diagnostics now capture and tail the migration's stderr with `2>"$stderr_file"` and `tail -n 20 | sed "s/^/[$id]   /"`.
- ✅ Post-no_op_pending drain loop closes the runner's writing end of the coproc to prevent migration-stdin deadlock; bounded by the existing watchdog.
- ✅ Extras key regex enforced on the runner side on receipt (defensive symmetric check).
- ✅ Frame parsing arity check: explicit `rest=""` when no TAB present, preventing type-leaks-into-field.
- ✅ Performance Considerations now sizes per-prompted-transformation IPC at 4 round-trips; documents prompt-loop runaway protection (progress prefix + 1000-prompt soft-threshold warning).
- ✅ `harness_extras_set` documents the auto-clear-after-emission lifecycle with explicit set-inside-loop guidance (closes the footgun finding).
- ✅ Session-log banner placement pinned to "immediately before the first PROMPT" (suppressed on fully-resumed runs).
- ✅ Inline-help line is full syntax on first PROMPT + after VALIDATE_ERR; compact `> ` otherwise — closes the noise-vs-discoverability tradeoff.
- ✅ SKILL.md Phase 7 §1 gains an "API reference at a glance" table (12 rows: 6 callbacks + 5 helpers + harness_run) as the scan-not-read discoverability anchor.
- ✅ `jsonl_json_escape` and `jsonl_compose_record` committed to single home `scripts/jsonl-common.sh` (sourced by both atomic-common.sh and interactive-lib.sh).
- ✅ Bash 4+ runtime error message pinned with copy-pasteable `brew install bash` / `mise use bash@5` commands.
- ✅ Phase 3 success criterion for bash 4+ gating now specifies both branches (3.2 mechanical-only OK; 3.2 with interactive pending → clear error) with a `bash3.2-shim` test fixture approach.
- ✅ "Independence between phases" now explicitly notes the Phases 1-6 dependency for 0070.

### Residual minor items (intentionally deferred or documented)

A small number of Pass-2 minors are deferred with explicit rationale rather than implemented:

- **Failure-mode matrix as a single enumerated artefact** (Code Quality minor): the matrix exists implicitly across protocol prose, runner pseudocode, and Migration Notes; consolidating it would be a documentation pass during Phase 3 implementation rather than a planning-level concern. Note in Phase 3 §6 success criteria: state-machine docstring is the natural home for the matrix when it lands.
- **External self-sufficiency check objective criterion** (Test Coverage minor): kept as a one-shot human gate with the 0070 author as the natural reviewer; the alternative proxies (grep checklists, second reviewer) would add ceremony without commensurate confidence. The 30-minute criterion is a soft guideline.
- **Cross-machine determinism for the doc-drift test** (Test Coverage minor): the 5-run same-process determinism gate is the floor; CI variance (locale, TZ, hostname) is an enumerated future-evolution concern documented in the Phase 7 redaction set but not exercised by an explicit test. Sufficient for ship.

### Assessment

The plan is in good shape for implementation. The architecture is sound (write-ahead-log invariant + opt-in verify-applied + canonical-first-field invariant + shared protocol library + bash-version-gated source line), the test plan is substantive and covers crash windows, escape boundaries, and resume edge cases that were silent gaps in Pass 1, and the documentation strategy (Phase 1 ships Prerequisites and CHANGELOG inline; Phase 7 ships CI-asserted worked example; maintainer state-machine docstring in interactive-protocol.sh) is appropriately layered. Three critical implementation bugs from Pass 2 — all narrow code-sample errors — are fixed at the source. The plan can move to implementation.

Implementation order is unchanged from the original phase decomposition (1 → 2 → 3 → 4 → 5 → 6 → 7); each phase ships green, with the bash 4+ source gate landing in Phase 1 (declarative) and Phase 3 (runtime check). Work item 0070 cannot ship until Phases 1–6 are all merged; Phase 7 docs may lag without blocking.

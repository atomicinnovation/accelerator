---
type: plan-review
id: "2026-08-22-0191-batch-shim-hashes-review-1"
title: "Plan Review: Batch the bootstrap's two shim hashes into one sha256 invocation"
date: "2026-08-22T23:33:00+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-08-22-0191-batch-shim-hashes"
target: "plan:2026-08-22-0191-batch-shim-hashes"
reviewer: Toby Clemson
verdict: "APPROVE"
lenses: [correctness, security, test-coverage, portability, code-quality, performance]
review_number: 1
review_pass: 3
tags: [shell, performance, bootstrap, bash-3.2, sha256, security]
last_updated: "2026-08-23T01:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Batch the bootstrap's two shim hashes into one sha256 invocation

**Verdict:** REVISE

The plan's goal — one backend fork, zero `awk` on the warm path — is sound, the TDD sequencing is real for the fork-count test, and the planted-stub regression guard is kept unmodified. But the plan diverges from the research's recommended **bounded guarded-batch** shape to a **glob-all-candidates + path-column parse** shape, and that single design decision is the root of a security trust-bypass, an unbounded-input performance regression, and several correctness and portability defects. The two new tests also do not verify the batched multi-file parse they are billed to cover.

### Cross-Cutting Themes

- **Glob-all + path-column parse is the wrong shape** (flagged by: security, performance, correctness, portability) — the plan hashes *every* executable file matching `accelerator-verify-${platform}-*` and trusts the digest on whichever output line's path column equals `${shim}`. The research recommended batching only the two known files inside the existing guard. The glob shape widens the trusted input to attacker-nameable files (security critical), grows warm-path hashing input without bound as stale shims and orphaned `.staging.$$` temps accumulate (performance major), pulls staging temps and traversable directories into the hash (correctness minor), and maximises exposure to per-backend multi-file output quirks (portability minor). Every severe finding traces back here.
- **`|| fail_integrity` on the batched call aborts on any candidate failure** (flagged by: correctness, security) — the `$(...) || fail_integrity` fires on the backend's exit status, which is non-zero if *any* input fails, not just the source. A candidate vanishing mid-run (TOCTOU) aborts the warm start — a regression versus today's graceful re-stage, and the exact opposite of the plan's own note claiming the `[[ -n "${shim_digest}" ]]` guard tolerates it. That guard is unreachable.
- **The new tests don't verify the batched fallback parse** (flagged by: test-coverage, portability) — `test_warm_path_on_the_shasum_fallback_stages_and_trusts` runs one cold bootstrap, so it takes the empty-candidate (single-input) branch and never exercises the batched two-input `shasum` form. It asserts only `returncode == 0` and passes unchanged on the old code. The multi-file Perl `shasum` parse — where the saving is largest — ships unverified.
- **GNU coreutils escapes filenames; Apple and Perl do not** (flagged by: portability, correctness, security) — coreutils prefixes a `\` and escapes `\n`/`\\` in the path column. A cache path with a backslash or newline corrupts both the line-1 source read and the path-column match on the Linux lane only — a cross-backend divergence invisible on the darwin host the change was developed against.
- **Divergent variable names across the two branches** (flagged by: correctness, code-quality) — the zero-candidate branch assigns `shim_digest_line`; the else branch assigns `shim_digests`; the later loop reads `${shim_digests:-}`. Correct only by accident of the `:-` default under `set -u`.

### Tradeoff Analysis

- **Bounded guarded-batch vs glob-discovery**: The plan argues the glob is *required* because `${shim}`'s path depends on the source digest, so the source must be hashed before the staged path is known — making a single fork over source-plus-staged impossible without discovering the candidate independently. This is a real constraint, but the research's guarded shape resolves it differently: hash the source first (one fork), then batch nothing — or batch only when the exact-named `${shim}` exists (`[[ -x "${shim}" ]]`), hashing at most two known paths and reading column 1. That form gives up strict AC-1 "one fork on every warm path" (it is one fork when no staged shim exists, two when re-verifying), but eliminates the critical, the performance regression, and the path-column parse entirely. Given this is a low-priority ~2.5 ms item, the security and correctness cost of chasing the single guaranteed fork is not obviously worth it. **Recommendation**: adopt the bounded shape, or if the glob is kept, constrain it to a strict 64-char lowercase-hex suffix and re-hash the matched candidate by known path rather than parsing a path column.

### Findings

#### Critical

- 🔴 **Security**: Path-column parse over attacker-named cache files enables a trust-boundary bypass the original never had
  **Location**: Phase 1, Section 4 (replacing `bin/accelerator:292-296`)
  The original re-hashes the single known file at `${shim}` and reads column 1, binding the digest to that file. The new design trusts `staged_digest` from whichever output line's path column matches `${shim}`. Filenames may contain newlines; a candidate whose glob-suffix embeds a newline can inject a second output line `<real_source_digest>␣␣${shim}`, so a stub separately planted at `${shim}` is trusted by name and executed as the verifier. Coreutils escapes this away; the macOS `shasum` fallback the plan ships does not. Directly defeats `test_planted_staged_shim_via_cache_dir_is_not_trusted`'s threat model (caller-controlled `ACCELERATOR_CACHE_DIR`), and the single-file planted-stub tests stay green while the two-file variant slips through.

#### Major

- 🔴 **Correctness**: `|| fail_integrity` on the batched call aborts on any candidate failure, contradicting the plan's TOCTOU note
  **Location**: Phase 1, Section 4 (non-zero-candidate branch and the ⚠️ `fail_integrity` note)
  All three backends exit 1 if *any* input fails. A candidate deleted between the glob and the hash (a concurrent bootstrap's `mv`) triggers `fail_integrity` and aborts, because the `||` runs before the `[[ -n "${shim_digest}" ]]` guard the note relies on. Regression versus today's graceful re-stage.

- 🔴 **Performance**: The batched call hashes every executable candidate; input grows unbounded under version churn and orphaned staging temps
  **Location**: Phase 1, Section 4; Performance Considerations
  Warm-path hashing input becomes O(executable candidates in bytes), not two fixed files. The plan declines to clean up stale shims, and the glob prefix matches `...-<digest>.staging.<pid>` temps (chmod +x'd before the rename), so a SIGKILL'd bootstrap leaves a file hashed on every warm start forever. ~5–8 accumulated ~475 KB files erase the 2.48 ms saving on the Apple backend; one or two make it a net regression on the ~3× slower `shasum` fallback.

- 🟡 **Test Coverage**: Stale-shim rejection — the glob design's signature new path — has no automated test
  **Location**: Phase 1, Manual Verification bullet 3; Manual Testing Step 4
  The multiple-candidate selection that trusts only the current-digest path and skips stale-version candidates is the design's new risk, yet it is verified only manually. A mis-selection (off-by-one path compare, trusting line 1 rather than the path-matched line) would ship silently.

- 🟡 **Test Coverage / Portability**: The forced-fallback test never exercises the batched multi-file parse and passes on the old code
  **Location**: Phase 1, Section 2; Phase 2, Section 3 (AC-5)
  The test runs one cold bootstrap → empty-candidate branch (single input) → asserts only `returncode == 0`. The pre-change `sha256_file` already handles the `shasum` single-file fallback, so it is neither red-first nor a genuine test of the batched fallback. On Linux CI `shasum` may be absent and the test skips. The batched Perl `shasum` format — the plan's largest saving and its weakest evidence — could ship completely unverified.

- 🟡 **Test Coverage**: AC-4 (batched source digest equals standalone value, keyed by path not position) is asserted by no test
  **Location**: Implementation Approach ("satisfied vacuously"); Phase 1, Section 4 empty-array branch
  AC-4 exists to guard against positional mis-assignment; the plan declares it satisfied by design rather than testing it. The cold empty-array branch's digest correctness is also never asserted — both new tests warm the cache first.

- 🟡 **Portability**: GNU coreutils filename escaping diverges from Apple/Perl and can corrupt the parse
  **Location**: Phase 1, Section 4 (path-column parse)
  On the Linux lane, a `cache_dir` (from caller-controlled `ACCELERATOR_CACHE_DIR`) containing a backslash or newline makes coreutils emit `\<hex>…`, corrupting `read -r shim_digest _` (wrong shim name) and the `candidate_path == shim` match — a permanent re-stage loop invisible on the darwin dev host.

- 🟡 **Code Quality**: The error message `could not hash the verify shim source` is triplicated and, on the batched branch, inaccurate
  **Location**: Phase 1, Section 4 (both `||` handlers plus the presence guard)
  The same message appears three times, and on the batched branch it fires on *any* backend non-zero exit — reporting a source-hash failure when the source hashed fine. Contradicts the plan's own re-stage-on-TOCTOU claim. (Same root as the correctness major.)

- 🟡 **Code Quality**: Divergent variable names across the if/else branches; the `:-` default masks an unset variable
  **Location**: Phase 1, Section 4 (`shim_digest_line` vs `shim_digests`)
  Two names for the same value across sibling branches; the `${shim_digests:-}` fallback silently hides that `shim_digests` is never set on the zero-candidate path, inviting a future edit that breaks the empty case.

- 🟡 **Code Quality**: The load-bearing rationale comment at `bin/accelerator:281-291` is left stale
  **Location**: Phase 1, Section 4 ("keep the staging body unchanged")
  The comment describes "skip-if-exists verifies the staged bytes against that digest" — the exact short-circuit being removed. On a security boundary, a comment that misdescribes how trust is established is worse than none; the plan is silent on updating it.

#### Minor

- 🔵 **Correctness**: The glob matches `.staging.$$` temps and traversable directories
  **Location**: Phase 1, Section 4 (`for candidate in "${shim_prefix}"*`)
  `[[ -x dir ]]` is true for traversable directories; the prefix also matches chmod-+x'd staging temps. Both get passed to the batched hash (a directory → "Is a directory" non-zero exit), compounding the abort risk. Tighten to `[[ -f && -x ]]` and exclude `.staging.*`.

- 🔵 **Portability**: Source digest keys on line-1 order while the candidate loop keys on path column — inconsistent
  **Location**: Phase 1, Section 4 ("Argument-order dependence" note)
  The candidate loop is order-robust by design, but the source digest relies on emission order, verified only on Apple. Key the source on its path column too for internal consistency and cross-backend safety.

- 🔵 **Test Coverage**: Cold-path fork count (one fork, no `awk`) is only manually verified
  **Location**: Desired End State; Phase 1, Manual Verification bullet 2
  The Desired End State claims the guarantee on both paths; only the warm path has an automated assertion. Add a cold-cache xtrace variant.

- 🔵 **Test Coverage**: AC-3 (no missing-input stderr on a cold run) relegated to a manual diff though cheaply automatable
  **Location**: Phase 1, Manual Verification bullet 2
  With the glob design this is trivially assertable (no `No such file` line in combined output); leaving it manual means it will not re-run in CI.

- 🔵 **Security**: Candidate-churn TOCTOU aborts the bootstrap rather than re-staging (availability)
  **Location**: Phase 1, Section 4 (`2>/dev/null` and `|| fail_integrity`)
  An attacker with cache-dir write can rapidly create-then-delete matching executable files to make every warm start abort. Same fix as the correctness major.

- 🔵 **Security**: The whole trust reduction hinges on an unverified line-1-equals-source assumption with no defence in depth
  **Location**: Phase 1, Section 4 (`read -r shim_digest _`)
  If a backend emits diagnostics first, reorders, or escapes the source line, `shim_digest` silently becomes a candidate's digest. Assert output line count equals input count; reject escaped/malformed source lines.

- 🔵 **Performance**: The before-then-after measurement is drift-vulnerable and clean-state only
  **Location**: Phase 2, Section 1 (AC-6)
  A fixed before/after ordering biases the second tree under thermal/load drift; the clean single-shim state cannot detect the accumulation cost. Interleave A/B samples; record n and dispersion; measure a seeded-accumulation state.

- 🔵 **Code Quality**: The replacement grows the staging condition from ~5 to ~33 lines
  **Location**: Phase 1, Section 4
  Disproportionate cognitive complexity for a ~2.5 ms low-priority item; four interacting steps (glob loop, two-arm digest, guard, match loop) replace two. Largely resolved by adopting the bounded shape.

#### Suggestions

- 🔵 **Correctness**: Note the domain constraint that install/cache paths contain no newlines/backslashes, or strip a leading `\` before parsing (GNU backend).
- 🔵 **Test Coverage**: Move the two new tests from "Unit Tests" to "Integration Tests" in the Testing Strategy — both drive the full bootstrap via `run_bootstrap`.
- 🔵 **Code Quality**: Drop the what-comment on the fallback-bin setup; reproduce the `297-312` staging body (comments included) rather than eliding it behind a placeholder.
- 🔵 **Performance**: The glob triggers a full cache-dir scan (launcher binaries, `.minisig`, fetch temps); negligible at expected scale but a new per-warm-start linear term worth noting.

### Strengths

- ✅ Renaming `sha256_file` → variadic `sha256_files` honestly signals the new contract and drops the `awk` post-processing.
- ✅ The warm-path fork-count test is a genuine red-green driver (2 forks + awk → red; 1 fork, no awk → green), and its regex anchor on `/` robustly excludes the `command -v sha256sum` detection line.
- ✅ The three planted-stub trust-boundary tests are kept unmodified as the standing regression guard.
- ✅ The bash 3.2 hazards are handled correctly: the empty-array `${#...[@]}` branch, the nonmatching-glob fallthrough via `[[ -x ]]` without `nullglob`, and `read`/here-strings instead of bash-4 `mapfile`.
- ✅ AC-7 (warm-dispatch ratio) is correctly scoped as evidence for tightening 0189's C5 ceiling, not a pass condition, avoiding coupling a millisecond change to a noisy ratio gate.
- ✅ Measuring before/after digest-bracket medians back-to-back in one session is the right control for the documented cross-session host variance.

### Recommended Changes

1. **Adopt the research's bounded guarded-batch shape, or strictly constrain the glob** (addresses: the security critical, the performance major, the correctness `.staging`/directory minor, the portability glob-exposure minor, the code-quality complexity minor)
   Prefer batching only the two known files inside the existing `[[ -x "${shim}" ]]` guard, re-hashing `${shim}` by its known path and reading column 1 — no path-column parse over arbitrary filenames. If the glob is kept for the single-fork guarantee, constrain it to `accelerator-verify-${platform}-[0-9a-f]*` with a 64-char lowercase-hex length/charset check before adding a candidate, exclude `.staging.*`, require `[[ -f && -x ]]`, reject any output line beginning with `\`, and re-hash the matched candidate by known path rather than trusting a parsed path column.

2. **Fix the batched-call error handling** (addresses: the correctness major, the security TOCTOU minor, the code-quality triplicated-message major)
   Do not `|| fail_integrity` on the batched call. Capture its output unconditionally and rely solely on `[[ -n "${shim_digest}" ]]` as the single abort point with one shared message, so only an unhashable *source* aborts and candidate churn degrades to re-staging.

3. **Make the fallback test actually exercise the batched multi-file parse** (addresses: the two test-coverage/portability fallback majors, AC-5)
   Run the fallback bootstrap warm (twice against the same cache dir) so the two-input `shasum` form is parsed, assert the digest equals the Python `hashlib` oracle, assert via xtrace exactly one `shasum -a 256 /` line and zero `sha256sum`/`awk`, and record a captured `shasum -a 256 f1 f2` fixture rather than asserting "verified on all three backends".

4. **Add automated tests for the new paths** (addresses: stale-shim, AC-4, AC-3, cold-path fork count)
   Seed a stale `accelerator-verify-${platform}-<old>` alongside the current one and assert the stale file is neither trusted nor removed; assert the batched source digest equals the `hashlib` oracle (path-keyed, AC-4); assert a cold run emits no missing-input stderr (AC-3); add a cold-cache xtrace fork-count assertion.

5. **Bound stale-shim accumulation, or measure the accumulated state** (addresses: the performance major)
   Since the current digest is known once staging confirms, unlink non-matching stale/orphan candidates (also fixes the orphan-temp leak), or at minimum measure a cache seeded with several stale/orphan candidates so AC-6 characterises steady state rather than the best case.

6. **Update the rationale comment and unify the variable name** (addresses: the two code-quality/correctness minors)
   Rewrite `bin/accelerator:281-291` to describe how trust is now established (or trim it to the non-obvious *why*), and use one variable name in both branches, assigning it empty in the zero-candidate case so the re-stage-on-empty behaviour is intentional, not incidental.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Security

**Summary**: The plan preserves the name-plus-rehash trust rule on its happy path and keeps the three planted-stub tests unmodified, but introduces a materially new attack surface: instead of re-hashing the single known file at `${shim}` and reading column 1, the batched design hashes an attacker-influenceable set of files and trusts the digest reported for whichever output line's path column matches `${shim}`. Parsing a path column out of hash-tool output over attacker-named files is not equivalent to re-hashing a known path, and the plan neither analyses nor tests the two-file injection variant of the exact threat these tests defend.

**Strengths**:
- The core trust invariant is retained on the happy path: trust requires both the content-addressed name and the bytes hashing to the fresh source digest.
- The three planted-stub tests are kept and must pass unmodified; a forced-`shasum`-fallback test is added.
- The empty-array-under-`set -u` bash 3.2 hazard is called out and handled by branching on `${#shim_candidates[@]}`.
- The `[[ -n "${shim_digest}" ]]` guard correctly refuses to derive a path when no source digest was produced.

**Findings**:
- 🔴 Critical (confidence medium) — Phase 1, Change 4: Path-column parse over attacker-named cache files enables a trust-boundary bypass. Filenames may contain newlines; a candidate whose suffix embeds a newline injects a second output line `<real_source_digest>␣␣${shim}`. Candidates are globbed in sorted order and the loop breaks on the first path match, so an attacker can name the injection file to sort ahead of a malicious file planted at the real `${shim}` path; the spoofed digest wins, re-staging is skipped, and the planted stub is executed as the verifier. Coreutils escapes newlines; the macOS `shasum` fallback the plan ships has no such guarantee. Defeats `test_planted_staged_shim_via_cache_dir_is_not_trusted`'s threat model. Suggestion: constrain the glob to a strict 64-char lowercase-hex suffix, reject lines beginning with `\`, treat more than one path match as re-stage.
- 🔵 Minor (medium) — `2>/dev/null` and `|| fail_integrity`: candidate-churn TOCTOU aborts the bootstrap rather than re-staging. An attacker with cache-dir write can create-then-delete matching files to make every warm start abort. Separate source-hash error handling from candidate-hash error handling.
- 🔵 Minor (medium) — `read -r shim_digest _`: the whole trust reduction hinges on an unverified line-1-equals-source assumption with no defence in depth. Assert line count equals input count; reject escaped/malformed source lines.

### Correctness

**Summary**: The trust logic (glob discovery, path-keyed candidate match, re-stage on mismatch) is sound and preserves the planted-stub defence, and the bash 3.2 empty-array and no-match-glob cases are handled correctly. But the batched call's error handling contradicts the plan's own stated intent: `|| fail_integrity` aborts the warm start whenever any candidate vanishes mid-run — both the TOCTOU the plan claims to tolerate and a regression versus today's graceful re-stage. A secondary variable-naming divergence works only by accident of the `:-` guard.

**Strengths**:
- The empty-array-under-`set -u` hazard is avoided by branching on `${#shim_candidates[@]}` before expanding; the no-match glob is filtered by `[[ -x ]]`.
- The candidate loop keys the staged digest on the path column, so a reordering backend cannot mis-assign it.
- The planted-stub trust boundary is preserved exactly: a same-named garbage stub and a stale old-digest file are both distrusted.

**Findings**:
- 🔴 Major (high) — non-zero-candidate branch: `$(...) || fail_integrity` fires on the backend exit status, which is non-zero if any input fails. A candidate deleted between glob and hash aborts the warm start; the `[[ -n "${shim_digest}" ]]` guard the note relies on is never reached. Capture output unconditionally and rely on the presence guard.
- 🔵 Minor (high) — `shim_digest_line` vs `shim_digests`: divergent names; correct only by accident of `:-` under `set -u`. Unify to one name assigned empty in the zero-candidate case.
- 🔵 Minor (medium) — the glob matches `.staging.$$` temps and traversable directories (`[[ -x dir ]]` is true). Tighten to `[[ -f && -x ]]` and exclude `.staging.*`.
- 🔵 Suggestion (low) — GNU coreutils escapes `\`/newline in the path column; a pathological cache/source path corrupts the parse. Note the domain constraint or strip a leading `\`.

### Test Coverage

**Summary**: The plan follows red-green ordering for the fork-count test and retains the three planted-stub tests unmodified, and the fork-count regex is well-anchored. But the glob-discovery design introduces genuinely new code paths — multi-candidate selection, stale-shim rejection, the empty-array cold branch, TOCTOU fallthrough — several relegated to manual verification or left untested. Most critically, the forced-fallback test asserts only `returncode == 0` and passes unchanged on the old code, and AC-4 is declared "satisfied vacuously" rather than asserted.

**Strengths**:
- The planted-stub tests are kept as the trust-boundary guard and incidentally exercise the wrong-bytes re-stage case.
- The warm-path fork-count test is a genuine red-green driver.
- The `_backend_execs` regex anchoring on `/` robustly excludes the detection line.
- The bash 3.2 empty-array gotcha is acknowledged and handled.

**Findings**:
- 🔴 Major (high) — stale-shim rejection has no automated test (Manual Verification bullet 3, Manual Testing Step 4). Add an integration test seeding a stale-digest executable and asserting it is neither trusted nor removed.
- 🟡 Major (high) — the forced-fallback test asserts only `returncode == 0`, does not verify the `shasum` branch or batching, and passes unchanged on the old code. Run under xtrace, assert one `shasum -a 256 /` line, zero `sha256sum`, zero `awk`.
- 🟡 Major (medium) — AC-4 (path-keyed digest equals standalone) asserted by no test; the cold empty-array branch's digest correctness is never asserted. Assert against the `hashlib` oracle and cover the cold branch.
- 🔵 Minor (high) — cold-path fork count only manually verified though the Desired End State claims it on both paths.
- 🔵 Minor (medium) — AC-3 (no missing-input stderr on cold run) relegated to a manual diff though cheaply automatable.
- 🔵 Minor (medium) — TOCTOU glob-then-delete fallthrough has no regression guard; add a genuinely-unhashable-source test to pin the source/candidate discrimination.
- 🔵 Suggestion (high) — both new tests are integration tests but listed under "Unit Tests".

### Portability

**Summary**: The bash 3.2 constructs are handled carefully — the nonmatching-glob fallthrough via `[[ -x ]]`, the empty-array `${#...[@]}` branch, and the `${shim_digests:-}` guard all avoid the `set -u` traps. The principal risk is that the plan asserts the multi-file format and ordering are "verified on all three backends" when only Apple is confirmed, and the one new fallback test never actually exercises the batched Perl `shasum` path.

**Strengths**:
- The nonmatching-glob case is handled portably without `shopt -s nullglob`.
- The empty-array `set -u` trap is avoided; the later loop guards its here-string with `${shim_digests:-}`.
- `read -r`/here-strings are used rather than bash-4 `mapfile`; the last `read` field absorbs internal spaces in the path column.

**Findings**:
- 🟡 Major (high) — Perl `shasum` multi-file batched format asserted "verified" but never exercised: the fallback test's single cold bootstrap takes the empty-candidate (single-input) branch; on Linux CI `shasum` may be absent and the test skips. Make the test warm, assert the oracle digest, record a captured `shasum -a 256 f1 f2` fixture.
- 🟡 Major (medium) — GNU coreutils escapes `\`/newline; on the Linux lane a cache path with a backslash makes line 1 read `\<hex>`, corrupting both the source read and the path match → permanent re-stage loop. Note the behaviour and normalise/reject or detect the leading-`\` form.
- 🔵 Minor (medium) — the source digest keys on line-1 order while the candidate loop keys on path column; inconsistent and Apple-only-verified. Key the source on its path column too.
- 🔵 Minor (medium) — the glob-all shape widens cross-backend output-format exposure the research's bounded form avoided. Adopt the bounded shape or record why the wider exposure is acceptable with observed multi-candidate evidence.

### Code Quality

**Summary**: The plan trades a compact five-line hash-and-compare for a ~33-line block: a glob loop, a two-branch if/else, a presence guard, and a while-read match loop. That is a substantial cognitive-complexity increase in a root-of-trust entry point to save ~2.5 ms on a low-priority item, and the replacement carries three concrete smells — divergent variable names, a triplicated-and-partly-inaccurate error message, and an unaddressed stale rationale comment. The TDD sequencing, the `sha256_files` rename, and keeping the planted-stub tests unmodified are genuine strengths.

**Strengths**:
- The variadic `sha256_files` rename signals the new contract and drops `awk`.
- Red-first TDD ordering is explicit; the planted-stub tests are preserved.
- The bash 3.2 empty-array gotcha is called out and handled.

**Findings**:
- 🟡 Major (high) — complexity: ~5 → ~33 lines with four interacting steps; disproportionate to the saving. Consider the bounded shape or a named helper.
- 🟡 Major (high) — divergent variable names across branches; `${shim_digests:-}` masks an unset variable. Use one name.
- 🟡 Major (high) — the `could not hash the verify shim source` message is triplicated and, on the batched branch, fires on any backend non-zero exit — inaccurate and contradicting the plan's re-stage-on-TOCTOU claim.
- 🟡 Major (medium) — the rationale comment at `bin/accelerator:281-291` describes the removed short-circuit and goes stale; on a security boundary that is worse than none. Add a step to update it.
- 🔵 Minor (high) — what-comments in the new test bodies; the `# … unchanged cp/chmod/mv staging body …` placeholder risks dropping the real staging comments.
- 🔵 Suggestion (medium) — the glob hashes every executable candidate as a side effect of chasing a single fork; sanity-check the simpler guarded-batch shape or record the trade-off.

### Performance

**Summary**: The core structural claim is sound — replacing two backend forks plus two `awk` forks with one fork-free glob loop and one batched invocation removes real process-creation cost, and the ~2.48 ms saving is credible in a clean single-staged-shim state. The material risk is that the design changes the warm-path hashing input from two fixed files to every executable candidate in the cache dir, which grows unboundedly under version churn and orphaned staging temps; on the slow Perl `shasum` fallback this could offset or reverse the saving.

**Strengths**:
- The glob loop is genuinely cheaper than the `awk` it removes (in-process syscalls vs fork+exec); the warm path drops from ~4 forks to 1.
- `read`/`while read` here-string parsing is pure builtin work, not a reintroduced fork.
- AC-7 is correctly scoped as evidence, not a pass condition.
- Back-to-back one-session before/after medians are the right control for cross-session host variance.

**Findings**:
- 🔴 Major (medium) — the batched call hashes every executable candidate; input is O(candidates in bytes). Stale shims (declined for cleanup) and orphaned `...-<digest>.staging.<pid>` temps (chmod +x'd before rename, matched by the glob) accumulate; ~5–8 files erase the saving on Apple, one or two reverse it on `shasum`. Unlink non-matching candidates once the current shim is confirmed, or at least exclude `.staging.`, and measure an accumulated state.
- 🔵 Minor (medium) — the before-then-after single-session measurement is drift-vulnerable and clean-state only. Interleave A/B samples; record n and dispersion; measure a seeded-accumulation state.
- 🔵 Suggestion (low) — the glob triggers a full cache-dir directory scan (launcher binaries, `.minisig`, fetch temps); negligible at expected scale but a new linear term.

---

## Re-Review (Pass 2) — 2026-08-23

**Verdict:** APPROVE

The Pass 1 edits resolved the critical and every prior major across all six lenses. The re-run then surfaced new issues introduced by those edits — most importantly a source-digest trust gap flagged independently by both security and correctness — which were addressed in-session by a follow-up round of edits. The design shape moved from the vulnerable glob-all + path-column parse to a hex-constrained glob with both digests keyed on the path column; the plan now carries no known open critical or major.

Note: the Pass 2 agents reviewed the plan *after* the Pass 1 edits but *before* the follow-up fixes below. Each new finding is annotated with the fix applied afterwards. A third verifying pass is available if independent confirmation of the fixes is wanted.

### Previously Identified Issues (from Pass 1)

- 🔴 **Security**: filename-injection trust bypass via path-column parse — **Resolved**. Candidates constrained to a strict 64-char lowercase-hex name-suffix (`case "" | *[!0-9a-f]*` + `[[ ${#suffix} -eq 64 ]]` + `[[ -f && -x ]]`); no candidate filename can carry the newline/space/backslash the injection needs.
- 🔴 **Correctness**: `|| fail_integrity` aborted on any candidate failure — **Resolved**. The batched branch drops `||`; abort is gated solely by the source-digest presence check.
- 🔴 **Performance**: unbounded hashed input from stale shims and `.staging` temps — **Resolved**. Hex filter excludes `.staging.$$` temps; stale shims bounded by release churn; seeded-accumulation measurement added with a break-even N.
- 🟡 **Test Coverage**: stale-shim rejection, AC-4, forced-fallback batching, cold-path/AC-3 untested or weak — **Resolved** (with new implementation gaps found this pass, see below).
- 🟡 **Portability**: Perl `shasum` batched form never exercised; coreutils candidate-name escaping — **Resolved**. The warm forced-fallback test now exercises the batched `shasum` form and the CI matrix (macos + ubuntu legs) gives real cross-backend coverage; hex names cannot be escaped.
- 🟡 **Code Quality**: divergent variable names, triplicated/inaccurate message, stale rationale comment — **Resolved**. Unified `shim_digests`; message accurate; change 5 rewrites the comment.

### New Issues Introduced (and fixes applied in-session)

- 🟡 **Security + Correctness** (cross-cutting, medium): the source digest was still read from line 1 by position under the now-unchecked batched call, so a partial-failure call that dropped the source line could promote a candidate's digest to the trust anchor. **Fixed** — the source digest is now keyed on the output line whose path column equals `${shim_source}`, exactly as the candidate loop keys on `${shim}`; the presence guard aborts precisely when the source line is absent. The `read -r shim_digest _` line-1 approach and the stale "Argument-order dependence" note were removed; the Implementation Approach and ⚠️ notes updated.
- 🟡 **Test Coverage** (high): the new tests called helpers `_staged_shim_path`/`_staged_shim_digest` that don't exist and `_source_shim_digest(harness)` with the wrong signature (real: `(root, host_platform)`), omitting the `host_platform` fixture. **Fixed** — change 2 adds a helper-creation sub-step, the tests request `host_platform`, and calls use the real signature.
- 🟡 **Test Coverage** (medium): the AC-4 digest-keying case was prose-only with no observation seam. **Fixed** — added `test_batched_source_digest_keying_matches_oracle` (plant a stale hex candidate, warm, assert the trusted shim's name-suffix equals the `hashlib` source oracle) and stated the AC-4 reinterpretation (the glob-guarded design never produces a missing input).
- 🔵 **Test Coverage** (minor): the stale-shim test planted a byte-identical copy, so "not trusted" was unobservable. **Fixed** — the stale file now holds distinct garbage bytes, so a wrongful selection would fail the run.
- 🔵 **Portability** (minor): the "path-column parse is safe" claim over-reached — the cache-dir *directory* prefix is not hex-constrained. **Fixed** — added a directory-path domain-constraint ⚠️ note (benign re-stage-every-time on a pathological path, no trust breach).
- 🔵 **Portability** (suggestion): the `[0-9a-f]` range glob is locale/collation-sensitive. **Fixed** — the ⚠️ note now states the exact-path match plus the length check are the real guard and the range is a cheap ASCII prefilter.
- 🔵 **Code Quality** (minor/suggestion): change 5 didn't say to drop the comment's stale test-name references; the name literal appeared three times. **Fixed** — change 5 now says to drop the test-name enumeration; the suffix is derived by stripping `shim_prefix`, removing the third literal.
- 🔵 **Performance** (minor): break-even N asserted not quantified; "input set bounded" conflated the hashed and scanned sets. **Fixed** — Phase 2 records the break-even N; Performance Considerations distinguishes the bounded hashed set from the O(entries) scanned set.

### Assessment

The plan is in good shape for implementation. The trust boundary is now established by a hex-constrained glob with both digests keyed on the path column and a fail-closed source guard — a design two lenses independently endorsed. Test coverage is concrete and implementable (helpers specified, AC-4 given an observation seam, stale-shim made falsifiable), and the residual portability/performance items are documented domain constraints and precise wording rather than open risks. The one honest caveat: the in-session fixes to the Pass 2 findings have not themselves been through an independent review pass.

---
*Re-review generated by /accelerator:review-plan*

---

## Re-Review (Pass 3) — 2026-08-23

**Verdict:** APPROVE

Third pass on the three lenses whose Pass 2 fixes touched logic (security, correctness, test-coverage), to verify the in-session fixes hold and introduced nothing new. The source-digest path-keying fix is confirmed sound; the test fixes are confirmed to reference real infrastructure. One new security major was raised and resolved by author decision (accept + document + measure); the remaining test-coverage findings were mechanical and are fixed.

### Previously Identified Issues (from Pass 2)

- 🟡 **Security + Correctness**: source digest keyed by position under the unchecked batched call — **Resolved and verified**. Correctness traced it under `set -uo pipefail`: empty output aborts cleanly, path matches are verbatim, the double-space separator is collapsed by `read` (no leading-space bug). Security confirmed no candidate can impersonate the source line (`${shim_source}` has no hex suffix; candidates always carry `-<64hex>`) and the fix is strictly fail-closed.
- 🟡 **Test Coverage**: non-existent helpers / wrong signatures — **Resolved and verified** against the test file: `_source_shim_digest(root, host_platform)` exists (line 512), the `host_platform` fixture exists, `run_bootstrap` accepts `path=` + `xtrace=True`, `Harness.root` exists, and `assert_hermetic` does not inspect PATH.

### New Issues (this pass)

- 🟡 **Security** (major, medium) — **adversarial DoS**: the single-fork design hashes every hex-named candidate, so a cache-dir-write attacker can seed many ~475 KB valid-shaped files, each hashed on every warm bootstrap. No trust breach (content-addressing holds). **Accepted by author decision** — cache-dir write is an already-degraded posture with stronger DoS vectors, and bounding it would cost AC-1 or capping complexity. Added a threat-model note (Performance Considerations) and extended the Phase 2 seeded measurement to record the adversarial worst-case N.
- 🟡 **Test Coverage** (major, high) — the dedicated AC-4 test (`test_batched_source_digest_keying_matches_oracle`) observed run 1's filename, not the multi-input parse, and depended on a fragile glob tie-break with two hex candidates present. **Fixed** — dropped the redundant test; the stale-shim no-re-stage test is now stated as the genuine AC-4 observation (a mis-keyed source triggers a spurious re-stage → inode change → failure), with the AC-4 reinterpretation recorded.
- 🔵 **Test Coverage** (minor) — `_staged_shim_path`/`_staged_shim_digest` helper spec omitted the 64-hex filter and multi-candidate tie-break. **Fixed** — helpers now specified to apply the production hex filter and resolve the shim matching the current source digest.
- 🔵 **Test Coverage** (minor) — stale-shim garbage-bytes rationale overstated exec-failure observability. **Fixed** — reworded so the inode/mtime no-re-stage assertion is credited as load-bearing.
- 🔵 **Test Coverage** (minor) — the existing newline-cache-dir test (`test_accelerator_entrypoint.py:1210`) was not linked as an affected guard. **Fixed** — added to the must-stay-green list.
- 🔵 **Correctness** (minor, low) — the whitespace domain-constraint note omitted leading/trailing path-segment spaces. **Fixed** — note extended.

### Assessment

The plan is ready for implementation. Across three passes the trust boundary went from a filename-injection bypass to a hex-constrained glob with both digests keyed on the path column and a fail-closed source guard, verified sound under `set -uo pipefail`. Test coverage is concrete and confirmed to target real infrastructure, with the genuine AC-4 observation carried by the stale-shim no-re-stage test. The one residual — an adversarial DoS reachable only from an already-degraded attacker-writable-cache posture — is an accepted, documented, and about-to-be-measured tradeoff, not an open defect. No open critical or major remains.

---
*Re-review generated by /accelerator:review-plan*

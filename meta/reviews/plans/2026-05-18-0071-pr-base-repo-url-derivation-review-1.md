---
date: "2026-05-19T00:17:15+00:00"
type: plan-review
producer: review-plan
target: "plan:2026-05-18-0071-pr-base-repo-url-derivation"
review_number: 1
verdict: APPROVE
lenses: [correctness, test-coverage, code-quality, compatibility, safety, portability, documentation]
review_pass: 4
status: complete
id: "2026-05-18-0071-pr-base-repo-url-derivation-review-1"
title: "2026-05-18-0071-pr-base-repo-url-derivation-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-19T00:17:15+00:00"
last_updated_by: Toby Clemson
---

## Plan Review: pr-base-repo.sh URL-Derivation Migration

**Verdict:** REVISE

The plan is small, surgical, and well-structured — it identifies a single broken line, preserves the resolver's stdout contract, and introduces a thoughtful new smoke-test layer (Phase 4) that addresses a documented blind spot in the PATH-stub harness. The TDD staging and the reuse of established patterns (PHASE gating, `assert_grep_empty`, default_payload helpers) are strengths. However, several load-bearing claims in the plan are unverified or incorrect: the Phase 1 "red" claim fails for tests 8/9/11 against the unchanged production script; the cross-fork-safety property the resolver exists for has no automated test coverage on any gh version; the `gh ≥ 2.40.0` floor is asserted as fact in the new header but only verified on the workspace's pinned 2.89.0; and the Phase 4 smoke check, named as the corrective layer, is a loose help-text scrape that can false-pass on field-removal regressions and false-fail on cosmetic help-format changes. These need to be addressed before implementation.

### Cross-Cutting Themes

- **Phase 4 smoke check is a syntactic, not semantic, guarantee** (flagged by: test-coverage, compatibility, safety, portability) — The help-text scrape regex `(^|[[:space:],])$field([[:space:],]|$)` can match `url` anywhere in the help blob, including flag descriptions and examples. It cannot catch field-semantics or URL-shape regressions, only literal token disappearance. This is the plan's primary corrective net for the structural defect class, so its limitations matter.

- **`gh ≥ 2.40.0` floor is asserted but unverified** (flagged by: compatibility, documentation, portability) — Only the pinned 2.89.0 is checked. The new header text presents the floor as fact; Phase 2 manual verification only checks the pinned gh; nothing verifies 2.40.0 or 2.65.0 specifically.

- **Cross-fork-safety has no automated coverage** (flagged by: safety, test-coverage) — Test 4 stubs an upstream URL and asserts it parses correctly, which is tautological because `install_fake_gh` dispatches only on `$1 $2`. The entire reason the resolver exists is verified only by one-off manual checks.

- **Test 24 / smoke harness PHASE-gating asymmetries** (flagged by: test-coverage, portability, safety) — Test 24 is gated to phases 3+, so `PHASE=2` after merge silently skips the regression guard. The new Phase 4 smoke harness has no PHASE gate at all, so it will fail at PHASE=1 against the unchanged production code.

- **Post-regex empty-check is dead code** (flagged by: correctness, code-quality) — The regex `([^/]+)` cannot produce empty captures, so the proposed empty-check block is unreachable; the header comment then promises a defensive guard that does not fire.

- **Phase 1 red claim is broken or weak for tests 8, 9, 11** (flagged by: correctness, test-coverage) — Tests 8/9 assert `exit 1 + stderr contains "url"`, both satisfied by the existing structured-payload null-guard message that echoes the raw payload (which contains the literal JSON key `url`). Test 11 asserts only `exit 1`, also satisfied.

### Tradeoff Analysis

- **Strict TDD landing order vs. safety of intermediate state**: The plan claims each phase is landable as a separate change, but Phase 1 alone leaves production unchanged while assertions are reshaped — a contributor reading red CI could revert Phase 1 instead of completing Phase 2. Recommendation: either land Phases 1+2 atomically as a single commit/PR, or invert the order so the production fix lands first.

- **Defense-in-depth (dead empty-check) vs. code clarity**: The post-regex empty check is provably unreachable, but removing it might be argued to weaken future safety. Recommendation: drop it and lean on the regex; if defence-in-depth is wanted, relax the regex to `[^/]*` and let the empty-check do the work — pick one layer, not both.

- **Forward-compat trailing regex group vs. strict matching**: `(/.*)?$` accepts `/files` suffixes that `gh pr view --json url` never emits today, but rejects `?query` and `#fragment` shapes that are equally plausible future variations. Recommendation: tighten to the strict `https://<host>/<owner>/<repo>/pull/<n>$` form matching today's gh output; let a future regression be loud rather than papering over it speculatively.

### Findings

#### Critical
- 🔴 **Safety**: Cross-fork-safety claim has no automated coverage
  **Location**: Phase 2: Manual Verification (Desired End State, Key Discoveries)
  The entire safety case rests on `gh pr view --json url` returning the upstream URL from a fork checkout. Test 4 only proves the stubbed payload passes through; it does not prove `gh` actually emits the upstream URL. If `gh` ever returns the fork's URL (regression, head-PR variant, transferred PR), `pr-update-body.sh` will PATCH the fork — potentially hitting an unrelated same-numbered PR there.

#### Major
- 🟡 **Correctness**: Tests 8, 9, and 11 will not actually be red against the unchanged production script
  **Location**: Phase 1: Red — Reshape Harness Assertions (Tests 8, 9, 11)
  All three pass against current production: test 11 (`{}`) extracts empty owner → exits 1 (only assertion); tests 8/9 assert stderr contains `url` — satisfied by the existing `Raw payload: {...}` echo. The red→green TDD discipline is broken for these tests. Tighten assertions to the post-fix-specific message text (e.g. `could not extract owner/repo from url`).

- 🟡 **Test Coverage**: Help-text scrape regex is over-permissive (false PASS risk)
  **Location**: Phase 4: Smoke — Real-`gh` Field Allowlist Check
  Regex matches `url` anywhere in the help blob — flag descriptions, examples, or prose. A future regression where `url` is removed from the JSON allowlist but mentioned elsewhere in help would PASS the smoke check. Anchor to the `JSON Fields:` section (e.g. `awk '/JSON Fields:/{f=1;next} f && /^$/{exit} f'`) or invoke `gh pr view --json INVALID 2>&1` and parse the allowlist from the error.

- 🟡 **Test Coverage**: Reshaped tests 8/9 drop coverage of the post-extraction null-guard
  **Location**: Phase 1, Section 1: Reshape Test 8/9
  Original 8/9 covered structured-payload null cases that hit the resolver's post-extraction null-guard. Reshaped 8/9 hit the regex-failure path instead. The retained post-regex null-guard (lines 454-458) is now untested — mutation-testing would not detect its removal. Add a test that targets that guard, or simplify by removing it (see dead-code finding).

- 🟡 **Test Coverage**: PHASE=2 after merge silently skips test 24
  **Location**: Phase 3, Section 2: Test 24 PHASE gating
  Test 24's gate keys on phase value, not on whether the production change has happened. Anyone running `PHASE=2` post-merge (CI matrix, bisect, partial-revert validation) silently skips the regression guard. Remove the gate once Phase 3 lands, or replace it with a sentinel-file check (e.g. presence of `--json url` in resolver).

- 🟡 **Test Coverage**: Test 4 (cross-fork) is tautological — no fork resolution exercised
  **Location**: Phase 1, Section 1: Reshape Test 4
  `install_fake_gh` dispatches only on `$1 $2` — the harness has no concept of fork-vs-upstream. Test 4 only verifies URL parsing of an upstream-shaped string. If the resolver were silently switched to `gh repo view` (returning fork coords on a fork checkout), tests would still pass. Rename the test honestly, add a tree-state guard against `gh repo view`, or flag cross-fork as a manual-only property.

- 🟡 **Compatibility**: `gh ≥ 2.40.0` floor and `url` allowlist presence asserted but not verified across the range
  **Location**: Overview / Key Discoveries / Phase 2 Manual Verification
  Only the pinned 2.89.0 is verified. Verification at 2.40.0 and 2.65.0 is "flagged for Phase 1" but not enumerated in Phase 1 success criteria. If any release in the window lacks `url`, this fix doesn't help those users. Promote the 2.40.0/2.65.0 checks to hard preconditions or anchor the floor to actually-verified versions.

- 🟡 **Compatibility**: Declining version pin leaves users on `gh < 2.40.0` with the same confusing failure
  **Location**: What We're NOT Doing — declining candidate (c)
  The plan rejects a gh version pin on the grounds that URL derivation works on every gh ≥ 2.40.0, but does nothing for users below the floor — they get the same class of `Unknown JSON field` error the plan claims to eliminate. Either add a minimal `gh --version` preflight in the resolver with an actionable upgrade message, or document explicitly that out-of-range users get a similar-shaped error and that this is accepted.

- 🟡 **Safety**: Phase 1 lands a broken-for-everyone state if shipped alone
  **Location**: Implementation Approach (TDD sequence rationale)
  Phase 1 reshapes tests to the new shape while production is unchanged. A contributor reading the red CI could revert Phase 1 instead of completing Phase 2. The "each phase is landable" claim is unsafe. Land Phases 1+2 atomically, or invert the order.

- 🟡 **Safety**: URL regex permits hosts-with-ports and unusual paths without test coverage
  **Location**: Phase 2, Changes Required #1 (regex extraction)
  The regex allows `[^/]+` for host (accepts `:8443`), `(/.*)?$` rejects query/fragment URLs, and no test covers GHE hosts, ports, query strings, or percent-encoded characters. Downstream, `pr-update-body.sh` interpolates the result into a `gh api` URL without re-escaping. Add tests for GHE-with-port, query strings, and percent-encoded variants; consider tightening to GitHub's actual repo-name charset.

- 🟡 **Safety**: Smoke check is a name-in-help-text scrape, not a behaviour assertion
  **Location**: Phase 4: Smoke — Real-`gh` Field Allowlist Check
  The smoke check cannot catch (a) field present in help text but with changed semantics, (b) field present but returning different shapes on fork vs same-repo, or (c) field present but empty on cross-fork. The corrective layer is structurally blind to the most plausible real-world failure modes. Add a semantic check that invokes real `gh pr view --json url` against a known stable public PR and asserts the URL extracts correctly.

- 🟡 **Safety**: No rollback or fallback path if URL derivation breaks on a future gh
  **Location**: What We're NOT Doing ("Not introducing dual code paths")
  Removal of the `baseRepository` path is total. Recovery from a future gh-side surprise is "land another plan" — measured in PR review cycles, not minutes. Either accept this explicitly under Migration Notes, or add a `PR_BASE_REPO_RESOLVER_MODE` env-var escape hatch.

- 🟡 **Portability**: Phase 4 smoke harness is not PHASE-gated
  **Location**: Phase 4: Smoke — Real-`gh` Field Allowlist Check
  Sibling tree-state guards (tests 22/23/24) gate on PHASE; the new smoke harness has no gate. Running `PHASE=1 mise run test:integration:github` after Phase 1 lands but before Phase 2 will extract `baseRepository` from the still-unmodified resolver and fail on `gh 2.65.0`. Add `PHASE` gating mirroring tests 22/23/24, or land Phase 4 only after Phase 2 is in.

- 🟡 **Portability**: Test 24 `\-\-json baseRepository` escaping differs from tests 22/23
  **Location**: Phase 3, Section 2: test 24 assert_grep_empty pattern
  Tests 22/23 pass bare patterns; only test 24 introduces `\-\-` backslash-escaping. The bare-pattern approach is safe in `assert_grep_empty`'s argv construction. The novel BRE escape is unnecessary and behaviour-undefined on strict POSIX grep. Drop the escaping and pass the literal `"--json baseRepository"`.

- 🟡 **Documentation**: Header asserts `gh ≥ 2.40.0` floor as fact without verification
  **Location**: Phase 3, Header Rewrite (lines 561-564)
  Mirror of the compatibility finding: the new header documents an unverified claim as authoritative. Either verify 2.40.0 explicitly in Phase 2 manual verification, or soften the header text to the actually-checked range.

- 🟡 **Documentation**: Test 24 failure message lacks operator-recovery context
  **Location**: Phase 3, Test 24
  The FAIL message lists file references only. A future operator hitting it has no breadcrumb to work item 0071 or this plan. Add a comment in the harness above test 24 (or extend the assertion's FAIL emission) pointing at `meta/work/0071-...md` and `pr-base-repo.sh`'s header.

- 🟡 **Documentation**: Phase 4 smoke FAIL message lacks recovery context
  **Location**: Phase 4 Smoke Check FAIL message (lines 711-714)
  Same shape as test 24's gap: failure names the symptom but provides no rationale or link to the plan/work item. Add a one-line hint pointing at `meta/work/0071-...md`.

#### Minor
- 🔵 **Correctness / Code Quality**: Post-regex empty-check is unreachable dead code
  **Location**: Phase 2 (lines 454-458)
  `[^/]+` cannot produce empty captures. Drop the block, or — if defence-in-depth is genuinely wanted — relax the regex to `[^/]*` and let the empty-check do the work. Pick one layer, not both.

- 🔵 **Correctness**: Regex rejects query strings and fragments
  **Location**: Phase 2 regex
  `(/.*)?$` accepts `/files` suffixes (which gh doesn't emit) but rejects `?query` and `#fragment` shapes. Either tighten to strict `https://<host>/<owner>/<repo>/pull/<n>$` matching today's gh, or broaden to `([/?#].*)?$`.

- 🔵 **Correctness**: Smoke-check field-extraction grep captures any `--json` token in source
  **Location**: Phase 4 smoke harness
  A future heredoc, comment, or example containing `--json foo` would be picked up as a field to validate. Anchor to a known invocation pattern, or assert exactly one `--json` line is found in the resolver.

- 🔵 **Correctness**: Phase 2 success criteria misdescribes test 24 state at PHASE=final
  **Location**: Phase 2 Success Criteria (4th bullet)
  Test 24 doesn't exist yet at end of Phase 2; it isn't "skipped", it isn't present. Rephrase: "harness terminates after test 23; `mise run test:integration:github` is fully green."

- 🔵 **Correctness**: Inconsistent header-comment line ranges
  **Location**: Phase 3 Overview ("lines 15-18") vs Section 1 ("lines 4-28")
  Clarify the replacement covers exactly lines 4-28 (the comment block after `set -euo pipefail`).

- 🔵 **Test Coverage**: Smoke check has no auth/help-output failure-mode tests
  **Location**: Phase 4 Success Criteria
  If `gh pr view --help` itself fails or emits empty output, every field FAILs with misleading attribution. Check the help command's exit code before greping and emit a SKIP with the captured output on non-zero exit.

- 🔵 **Test Coverage**: Reshaped test 11 dropped stderr-content assertion
  **Location**: Phase 1: Reshape Test 11
  Tests 8/9 assert stderr contains `url`; test 11 asserts only exit 1. Add the matching `assert_contains` for parity.

- 🔵 **Test Coverage**: AC #3 rationale conflates stub coverage with real-gh coverage
  **Location**: What We're NOT Doing (AC #3)
  The whole reason 0071 exists is that the stub didn't catch a gh-side contract drift. The same class of risk applies to the `gh repo set-default` stderr-phrase match. Either document explicitly or add a follow-up to extend Phase 4 to grep gh's stderr for the depended-on phrase.

- 🔵 **Test Coverage**: No GHE host coverage
  **Location**: Desired End State / Implementation Approach
  Regex permits any host but no test exercises a non-github.com URL. Add one parametrised GHE-host test (e.g. `https://github.acme.corp/acme/app/pull/119`).

- 🔵 **Test Coverage**: Manual verification gates on a gh version the implementer may not have
  **Location**: Phase 2 Manual Verification
  Document a fallback (e.g. `mise install gh@2.65.0`) or accept the Phase 4 smoke + reshaped units as adequate.

- 🔵 **Code Quality**: Field extraction by grepping resolver source is fragile
  **Location**: Phase 4 lines 684-696
  Refactor to variable-based `--json "$FIELDS"` would silently break extraction. Document the literal-string constraint explicitly in the harness header.

- 🔵 **Code Quality**: Resolver error message hardcodes `url` — drifts if the field set evolves
  **Location**: Phase 2 line 438
  The smoke harness is field-agnostic; the resolver isn't. Either accept the asymmetry with a comment or generalise to `.url was empty/null in gh response.`.

- 🔵 **Code Quality**: Substantial new header may exceed WHY-only norm
  **Location**: Phase 3 lines 541-576
  The "validates owner/name non-empty" bullet describes a guard that doesn't really fire (see dead-code finding) and should be trimmed after that finding is resolved.

- 🔵 **Code Quality**: `set -e` omission in smoke harness lacks the sibling harness's rationale comment
  **Location**: Phase 4 line 648
  Copy the `# set -e intentionally omitted...` comment block from `test-pr-base-repo-scripts.sh:12-14`.

- 🔵 **Code Quality**: Sourcing only `PLUGIN_ROOT/scripts/test-helpers.sh` (not the github-local helpers)
  **Location**: Phase 4 lines 667-668
  Add a one-line comment noting this is intentional (smoke harness exercises real gh, not the PATH stub).

- 🔵 **Compatibility**: GHE hostname behaviour permitted by regex but unverified
  **Location**: Phase 2 regex / Phase 3 header
  Either statement GHE in Migration Notes ("not explicitly tested, regex permits any host") or scope explicitly to github.com.

- 🔵 **Compatibility**: Help-text scrape format is itself a gh-version-coupled compatibility surface
  **Location**: Phase 4 smoke check
  A future gh help-format change could cause CI noise or mask genuine regressions. Document the assumption in the harness header.

- 🔵 **Compatibility**: URL shape stability assertion isn't anchored to a gh-CLI contract
  **Location**: Overview / Phase 2
  `gh`'s `url` is a pass-through of GitHub's GraphQL `PullRequest.url` field — gh doesn't own the URL shape. Add unit tests for plausible-but-non-canonical URL shapes.

- 🔵 **Safety**: Failure-mode operator message is generic, not actionable
  **Location**: Phase 2 extraction error message
  Extend to "Expected https://<host>/<owner>/<repo>/pull/<n>. If <url> looks correct, please file an issue with the URL above."

- 🔵 **Safety**: Test 24 regression-guard staging admits a brief silent-revert window
  **Location**: Phase 3, test 24 staging
  Acceptable trade-off as long as Phases 1-3 land as a single PR; document the dependency explicitly in Implementation Approach.

- 🔵 **Safety**: No end-to-end real-PR coverage for a safety-critical operation
  **Location**: AC #4 out-of-scope
  Add one opt-in real-PR end-to-end check against a stable known-cross-fork PR; one invocation, one assertion.

- 🔵 **Portability**: Help-text scrape couples to gh cobra layout
  **Location**: Phase 4 line 707
  Pin the search to `JSON Fields:` section with `awk` to narrow the false-positive surface.

- 🔵 **Portability**: Smoke harness captures stderr+stdout, which may include update-warning chatter
  **Location**: Phase 4 line 699
  Use `2>/dev/null` instead of `2>&1` to avoid false-positive matches from gh's stderr.

- 🔵 **Portability**: Optional `BASH_REMATCH[3]` could trip future `set -u` callers on macOS bash 3.2
  **Location**: Phase 2 regex
  Either drop the unused trailing group or comment that `BASH_REMATCH[3]` is intentionally unread.

- 🔵 **Documentation**: Manual-verification path uses unresolved `...` placeholder
  **Location**: Phase 2 Manual Verification line 510
  Resolve to a glob (`~/.claude/plugins/cache/atomic-innovation-prerelease/accelerator/*/skills/...`) or document how to resolve the version segment.

- 🔵 **Documentation**: `gh` version floor not surfaced to skill operators
  **Location**: What We're NOT Doing / consumer SKILL.md files
  Either add a "Requires gh ≥ 2.40.0" line to each consumer SKILL.md, or explicitly document the deferral.

#### Suggestions
- 🔵 **Code Quality**: Field-name interpolation into a regex is a latent injection hazard
  **Location**: Phase 4 line 707
  Add a one-line comment near the interpolation pinning the safe-interpolation invariant.

- 🔵 **Documentation**: Two header paragraphs duplicate the cross-fork rationale
  **Location**: Phase 3 header lines 555-557 and 561-564
  Trim the cross-fork bullet and fold the `gh repo view` comparison into a single trailing sentence.

### Strengths

- ✅ Stdout contract (`<owner>/<name>\n`) is preserved byte-for-byte, so the three consumer SKILL.md callers (describe-pr, review-pr, respond-to-pr) require no edits.
- ✅ Strict TDD discipline with phase-gated regression guards mirrors the existing 0059 plan pattern.
- ✅ The new Phase 4 real-`gh` smoke layer is the right shape (autodiscovered, skips cleanly when gh absent) and addresses a genuine structural blind spot in the PATH-stubbed harness.
- ✅ Tree-state regression guard (test 24) gives durable protection against `--json baseRepository` reappearing under `skills/github/`.
- ✅ Cross-fork-safety is correctly identified as the central invariant and explicitly preserved in the new header.
- ✅ BASH_REMATCH usage is consistent with prior precedents (work-item scripts, design/inventory-design scripts) — choosing bash regex over an external sed/awk shell-out is in keeping with codebase conventions.
- ✅ Smuggling guards (tests 8/9) prevent the most obvious shell-substitution hazards (`/<name>`, `<owner>/`) downstream.
- ✅ Reshaping `default_payload` and `upstream_payload` helpers in the sibling harness is a DRY win — one edit flips the entire downstream chain.
- ✅ The decision not to write an ADR for this fix is correct — it's a tactical one-line data-source swap, not an architectural commitment.
- ✅ Error messages in the resolver are improved over the originals: they name the failure mode and replay the raw payload to stderr.
- ✅ The new harness's FAIL message names the field, installed gh version, and script path — actionable for an on-call operator.

### Recommended Changes

Ordered by impact:

1. **Add automated coverage for cross-fork-safety** (addresses: Cross-fork-safety claim has no automated coverage; Test 4 tautological)
   Add an opt-in real-PR smoke check against a stable known-cross-fork public PR that invokes `gh pr view <pr> --json url` and asserts the returned URL host/path matches the upstream coords. Skip when offline/unauthenticated. Even one invocation locks the property in.

2. **Land Phases 1+2 atomically (single commit/PR)** (addresses: Phase 1 broken-for-everyone state)
   Document in the Implementation Approach section that Phases 1 and 2 are inseparable; the staged-landing claim is unsafe. Or invert the order so production lands first.

3. **Tighten Phase 1 assertions to post-fix-specific text** (addresses: Tests 8/9/11 not actually red; reshaped test 11 dropped stderr assertion)
   Assert stderr contains `could not extract owner/repo from url` (tests 8/9) and `url was empty/null` (test 11), matching the exact post-fix messages. Then re-trace red-claim for each.

4. **Replace help-text scrape with allowlist-section parse or error-message probe** (addresses: Help-text scrape over-permissive; smoke check is syntactic not semantic; couples to cobra layout)
   Option A: `awk '/JSON Fields:/{f=1;next} f && /^$/{exit} f'` to anchor to the section. Option B: invoke `gh pr view --json INVALID 2>&1` and parse the structured allowlist gh emits in the error message — same runtime surface the resolver hits, more behaviourally meaningful.

5. **Verify `gh 2.40.0` and `gh 2.65.0` explicitly in Phase 2 manual verification or soften the header** (addresses: gh ≥ 2.40.0 floor unverified [compat]; Header asserts floor as fact [docs])
   Add hard preconditions to Phase 2 with captured `gh pr view --help` excerpts, or change the header text to "verified on gh 2.65.0 and 2.89.0; the Phase 4 smoke check enforces the assertion on the installed gh at integration-test time".

6. **Drop the post-regex empty-check or relax the regex** (addresses: Dead code; header promises a guard that doesn't fire)
   Choose one layer. Recommended: drop the empty-check; the regex's `[^/]+` is the structural guarantee. Then trim the corresponding header bullet.

7. **Remove PHASE gate from test 24 (or land Phase 4 only after Phase 2)** (addresses: PHASE=2 silently skips test 24; Phase 4 smoke not PHASE-gated)
   Once Phases 1-3 land together, PHASE gating on test 24 has no defensive purpose. Either remove the gate post-merge, or gate the new smoke harness consistently with tests 22/23/24.

8. **Drop the `\-\-` escaping in test 24** (addresses: Test 24 escaping differs from 22/23)
   Pass the bare pattern `"--json baseRepository"` to `assert_grep_empty`. The argv positional safety of the helper makes the escaping unnecessary and the BRE backslash-escape is behaviour-undefined on strict POSIX grep.

9. **Add operator-recovery context to Test 24 and Phase 4 FAIL messages** (addresses: Test 24 message lacks context; Phase 4 FAIL lacks context)
   Add a one-line breadcrumb pointing at `meta/work/0071-...md` and `pr-base-repo.sh`'s header. Cheap, shifts documentation burden from operator investigation time to harness output.

10. **Tighten the URL regex against unsafe character classes** (addresses: URL regex permits unsafe characters; reshaped tests 8/9 lost null-guard coverage)
    Restrict owner/repo to GitHub's actual repo-name charset (`[A-Za-z0-9][A-Za-z0-9._-]*`); add tests for percent-encoded characters (must reject), hosts-with-ports (must extract correctly), and query strings.

11. **Document the rollback path explicitly under Migration Notes** (addresses: No rollback / fallback path)
    Either accept "recovery requires a new plan; rollback path is `git revert`" or add a `PR_BASE_REPO_RESOLVER_MODE` env-var escape hatch.

12. **Address the gh < 2.40.0 user experience** (addresses: Declining version pin leaves out-of-range users with confusing errors)
    Either add a minimal `gh --version` preflight in the resolver with an actionable upgrade message, or update "What We're NOT Doing" to explicitly state that out-of-range users get a similar-shaped error and that this is acceptable.

13. **Resolve the `...` placeholder in Phase 2 manual-verification path** (addresses: Manual-verification path uses unresolved `...`)
    Either spell out the version segment with discovery instructions, or use a glob that expands deterministically.

14. **Adopt minor code-quality polish** (addresses: minor code-quality findings collectively)
    Copy the `set -e omitted` rationale comment into Phase 4 harness; add the "do not source github-local helpers" comment; consider GHE-host test cases.

---
*Review generated by /review-plan*

## Per-Lens Results

### Correctness

**Summary**: The core technical approach (URL derivation via bash regex) is sound and the regex correctly handles GitHub Enterprise hosts, hyphens/dots/underscores in owners and repos, and rejects the malformed/truncated URL test inputs as claimed. However, the Phase 1 'red' claim is partially incorrect: tests 8, 9, and 11 will actually PASS against the unchanged production script because their assertions (exit code 1 + 'url' substring in stderr) are satisfied by the existing baseRepository null-guard path whose error message echoes the raw payload (which contains the literal JSON key 'url'). The post-regex emptiness guard at lines 454-458 is dead code given that `[^/]+` captures cannot be empty.

**Strengths**:
- Regex correctly handles GHE hosts, dots/hyphens/underscores, and optional trailing suffixes
- Tests 8/9 (malformed/truncated URL) are correctly rejected by the `[^/]+` non-empty requirement
- `jq -r '.url // ""'` correctly handles JSON null, missing, and non-string variants
- Stdout contract is byte-identical for all valid URLs
- Cross-fork safety preserved by URL field semantics

**Findings**:
- 🟡 major: Tests 8/9/11 not actually red against unchanged production (stderr `url` substring matches the raw-payload echo)
- 🔵 minor: Post-regex emptiness guard is unreachable dead code
- 🔵 minor: Regex rejects query/fragment URLs while accepting speculative `/files` suffixes
- 🔵 minor: Field-extraction regex in smoke harness captures any `--json` token in source
- 🔵 minor: Phase 2 success criteria misdescribes test 24 state at PHASE=final
- 🔵 minor: Inconsistent header-comment line range references (15-18 vs 4-28)

### Test Coverage

**Summary**: The plan's TDD structure is sound and the reshape of tests 3-5, 11, and the sibling test 6 maps cleanly to the new argv/payload contract. The new Phase 4 real-gh smoke layer is a valuable corrective for a documented blind spot, but its regex-based help-text scraping is fragile and over-trusting, and the negative-case reshape (tests 8/9) silently weakens coverage of the resolver's null-guard semantics. PHASE-gating logic for test 24 also has a subtle hide-regression failure mode.

**Strengths**:
- Strict red-then-green TDD discipline with explicit confirmation step
- Reuses established PHASE env-var pattern
- Phase 4 introduces a genuinely new coverage layer
- Test 24 mirrors proven 22/23 tree-state guard pattern
- Author audited which tests must flip vs orthogonal
- Stdout contract preserved byte-for-byte

**Findings**:
- 🟡 major: Help-text scrape regex over-permissive — short tokens like `url` can false-PASS
- 🟡 major: Reshaped tests 8/9 cover regex-failure path, not original null-guard branch — null-guard is now untested
- 🟡 major: PHASE=2 after merge silently skips test 24
- 🟡 major: Test 4 (cross-fork) is tautological — no fork resolution exercised
- 🔵 minor: Smoke check has no auth / help-failure / empty-extraction error paths
- 🔵 minor: Reshaped test 11 dropped explicit stderr-content assertion
- 🔵 minor: AC #3 rationale conflates stub coverage with real-gh coverage
- 🔵 minor: GHE host not exercised by any test
- 🔵 minor: Manual verification gates on a gh version the implementer may not have

### Code Quality

**Summary**: The plan is well-structured TDD with a small, surgical change to the resolver and a thoughtful new smoke-test layer. The bash regex extraction is consistent with conventions elsewhere in the codebase and the proposed error messages improve on the originals. However, a few code-quality issues warrant attention: a redundant post-regex empty-check, a help-text scrape that is clever but brittle in surprising ways, an inconsistent error-message vocabulary in the resolver, and substantial new header comments that push the limit of the codebase's WHY-only comment guidance.

**Strengths**:
- Strict TDD with phase-gated tree-state guards mirrors 0059 plan pattern
- Error messages name failure mode and replay payload to stderr
- BASH_REMATCH consistent with existing precedents (work-item scripts)
- Smoke harness FAIL names field, gh version, script — actionable
- Test 24 reuses `assert_grep_empty` helper and phase-gating idiom verbatim
- Reshaping default_payload/upstream_payload helpers is a DRY win

**Findings**:
- 🔵 minor: Defensive post-regex empty-check is dead code
- 🔵 minor: Field extraction by grepping resolver source is fragile to refactors
- 🔵 minor: Resolver error message hardcodes `url` — drifts if field set evolves
- 🔵 minor: Substantial new header comment may exceed the WHY-only norm
- 🔵 minor: `set -e` omission in smoke harness lacks rationale comment
- 🔵 suggestion: Field-name interpolation into regex is a latent injection hazard
- 🔵 suggestion: Sourcing only PLUGIN_ROOT/scripts/test-helpers.sh — comment the intent

### Compatibility

**Summary**: The plan correctly identifies the gh CLI allowlist mismatch as a one-line fix and preserves the resolver's stdout contract (so the three downstream skills require no changes), which is the strongest compatibility property of the proposal. However, the load-bearing claim that `--json url` is in the allowlist on every `gh ≥ 2.40.0` release is asserted rather than verified — only gh 2.89.0 has been confirmed, with verification at gh 2.40.0 and gh 2.65.0 deferred to Phase 1 manual steps — and the plan leaves several environment dimensions unevaluated (GHE hostnames, URL-shape stability, help-text scrape format stability, and operator experience on gh < 2.40.0).

**Strengths**:
- Stdout contract preserved byte-for-byte — clean backward-compatible API
- URL extraction regex permits any host, making fix host-agnostic
- Trailing `(/.*)?$` group anticipates forward-compatible URL variants
- Cross-fork-safety property preserved via GitHub URL scheme convention
- Phase 4 smoke is the right corrective layer for the allowlist-blind harness
- Phase 3 tree-state guard prevents `--json baseRepository` regression
- `jq -r '.url // ""'` syntax is ancient and broadly compatible

**Findings**:
- 🟡 major: `gh ≥ 2.40.0` floor and `url` allowlist presence asserted but not verified across range
- 🟡 major: Declining version pin leaves `gh < 2.40.0` users with same confusing failure mode
- 🔵 minor: GHE hostname behaviour permitted by regex but unverified anywhere
- 🔵 minor: Help-text scrape format is brittle compatibility surface across gh versions
- 🔵 minor: URL shape stability asserted but not anchored to a gh CLI contract
- 🔵 minor: Cross-fork URL property asserted on convention but not verified in test suite

### Safety

**Summary**: The plan addresses a critical safety-relevant defect (broken resolver blocks all describe-pr posts) and preserves the cross-fork-safety guarantee in its design intent. However, the central cross-fork-safety claim — that `gh pr view --json url` returns the upstream PR URL from a fork checkout — is not exercised by any automated test, only by manual verification. Combined with permissive URL regex parsing, staged-landing foot-guns in Phase 1, and a smoke check that only verifies field-name presence in help text rather than semantic behaviour, the plan has several gaps that could allow a contributor to PATCH the wrong repository.

**Strengths**:
- Plan recognises cross-fork-safety as central and preserves it as documented invariant
- Smuggling guards (tests 8/9) prevent `/<name>` and `<owner>/` shell-substitution hazards
- Tree-state guard (test 24) is permanent floor against broken field reappearing
- Resolver exits 1 on every malformed-URL path — propagates failure rather than PATCHing bogus base
- `--method PATCH` explicit in `pr-update-body.sh` prevents method-default drift

**Findings**:
- 🔴 critical: Cross-fork-safety claim has no automated coverage
- 🟡 major: Phase 1 lands a broken-for-everyone state if shipped alone
- 🟡 major: URL regex permits hosts-with-ports and unusual paths but no test covers them
- 🟡 major: Smoke check is name-in-help-text scrape, not behaviour assertion
- 🟡 major: No rollback or fallback path if URL derivation breaks on a future gh
- 🔵 minor: Failure-mode operator message is generic, not actionable
- 🔵 minor: Regression-guard staging admits a brief silent-revert window
- 🔵 minor: No end-to-end coverage on the pinned gh for a safety-critical operation

### Portability

**Summary**: The plan's bash idioms (BASH_REMATCH regex, here-strings, set -uo pipefail, grep -oE/-qE, awk, tr, sort -u) are all portable across bash 3.2 (macOS) and bash 4.x/5.x (Linux), and the existing codebase already uses BASH_REMATCH precedents. The main portability concerns are orchestration-level: the Phase 4 smoke harness is not PHASE-gated while the sibling tree-state tests are, which creates inconsistent staged-landing semantics; the help-text scrape regex is plausibly stable but couples to gh's cobra help layout in ways that could drift; and the `--json baseRepository` regex escaping for test 24 is novel relative to tests 22/23.

**Strengths**:
- Resolver shebang `#!/usr/bin/env bash` legitimises bash-only features
- URL regex correctly unquoted on RHS of `=~` (bash 3.2+ portable form)
- Field-name extraction regex uses ASCII-only ranges, locale-safe
- All CLI tools (jq, gh, awk, tr, sort, grep) used in POSIX-portable flag combinations
- Smoke harness skips cleanly when `gh` is absent

**Findings**:
- 🟡 major: Phase 4 smoke harness is not PHASE-gated — breaks staged-landing portability
- 🟡 major: Test 24 `\-\-json baseRepository` escaping differs from tests 22/23 and may misbehave on BSD grep
- 🔵 minor: Help-text scrape couples to gh cobra help layout
- 🔵 minor: `gh pr view --help` may behave differently in non-TTY contexts; use `2>/dev/null`
- 🔵 minor: Optional `BASH_REMATCH[3]` unread but could trip future `set -u` on macOS bash 3.2
- 🔵 minor: Header asserts `gh ≥ 2.40.0` floor without citing verification source
- 🔵 suggestion: Smoke harness `set -e` omission deserves the sibling's rationale comment

### Documentation

**Summary**: The plan is generally accurate on documentation impact: the 'no SKILL.md edits required' claim is verifiably correct, and the proposed header rewrite faithfully describes the new Phase 2 behaviour. The main documentation gaps are operator-facing: the gh version floor is asserted as fact in the new header but never independently verified; Test 24 and Phase 4 failure messages don't point future operators at this plan or work item 0071; and the manual-verification path contains an unexpanded `...` placeholder.

**Strengths**:
- 'No SKILL.md edits required' claim is accurate — no consumer mentions baseRepository or gh-version requirements
- Proposed header faithfully describes Phase 2 behaviour
- References block paths are real
- Decision not to write an ADR is correct (tactical fix)
- Header preserves high-WHY content (cross-fork, sourcing/trap caveat)

**Findings**:
- 🟡 major: Header asserts `gh ≥ 2.40.0` floor as fact but plan never verifies it
- 🟡 major: Test 24 failure message lacks operator-recovery context
- 🟡 major: Phase 4 smoke FAIL doesn't tell operator where to go for context
- 🔵 minor: Manual-verification path uses unresolved `...` placeholder
- 🔵 minor: gh version floor not surfaced to skill operators
- 🔵 suggestion: Two header paragraphs duplicate the cross-fork rationale

## Re-Review (Pass 2) — 2026-05-18

**Verdict:** REVISE

The pass-1 edits resolve most of the original findings cleanly: the dead empty-check is gone, stderr assertions are tightened for genuine red-claim, the help-text scrape is replaced with an error-message probe, test 24 is unconditional, the gh ≥ 2.40.0 floor claim is dropped, the cross-fork manual step is enumerated, and rollback is documented. However, the edits **introduce seven new major findings** across four themes, and one user-accepted-as-out-of-scope safety gap remains. The plan is closer to ready than pass 1 but needs another iteration.

### Previously Identified Issues

#### Critical
- 🔴 **Safety**: Cross-fork-safety claim has no automated coverage — **Partially resolved** (user chose stub-only + manual-verification approach; test 4 renamed honestly, enumerated manual step added with captured-output requirement; the property remains automation-absent by user decision but is now honest about it)

#### Major (pass 1) — Resolution status
- 🟡 **Correctness**: Tests 8/9/11 not actually red — **Resolved** (stderr assertions now require post-fix-specific text `could not extract owner/repo from url` / `url was empty/null`)
- 🟡 **Test Coverage**: Help-text scrape over-permissive — **Resolved** (replaced entirely with `gh pr view --json INVALID` error-message probe; new brittleness flagged below)
- 🟡 **Test Coverage**: Reshaped tests 8/9 drop null-guard coverage — **Resolved** (empty-check was dropped entirely; tests now hit the regex-failure path with tightened stderr assertion)
- 🟡 **Test Coverage**: PHASE=2 silently skips test 24 — **Resolved** (PHASE gate removed; test 24 unconditional)
- 🟡 **Test Coverage**: Test 4 (cross-fork) tautological — **Resolved** (renamed to "upstream URL parses to upstream coords"; enumerated manual step added)
- 🟡 **Compatibility**: gh ≥ 2.40.0 floor unverified — **Resolved** (claim dropped entirely; smoke check is source of truth)
- 🟡 **Compatibility**: Declining version pin leaves gh<2.40 users with confusing failure — **Resolved** (documented explicitly in Not-doing and Migration Notes)
- 🟡 **Safety**: Phase 1 lands broken-for-everyone — **Resolved** (Implementation Approach requires atomic Phase 1+2 landing; "each phase landable" claim dropped)
- 🟡 **Safety**: URL regex permits unsafe chars — **Resolved** (regex tightened, test 4c added) — *but see new finding about over-tightening*
- 🟡 **Safety**: Smoke check is syntactic, not semantic — **Resolved** (error-message probe is the same surface the resolver hits) — *but see new finding about parser fragility*
- 🟡 **Safety**: No rollback path — **Resolved** (Migration Notes documents `git revert` + new plan)
- 🟡 **Portability**: Phase 4 smoke not PHASE-gated — **Resolved by atomic landing** (Phase 4 lands after Phase 2; no partial state)
- 🟡 **Portability**: Test 24 `\-\-` escaping differs from 22/23 — **Resolved escape syntax, but introduced a different bug** (see new finding: bare `--json baseRepository` pattern hits grep's long-option parser)
- 🟡 **Documentation**: Header asserts unverified floor — **Resolved** (header no longer mentions a specific version)
- 🟡 **Documentation**: Test 24 message lacks context — **Resolved** (test name includes breadcrumb to resolver header; comment block above test points at meta/work/0071-*.md)
- 🟡 **Documentation**: Phase 4 FAIL lacks context — **Resolved** (FAIL messages now name work item 0071 and resolver header)

#### Minor — Status
- Most minor findings: **Resolved** (line ranges, success-criteria text, test 11 stderr assertion, GHE host coverage, manual-verification fallback documented, manual-verification path placeholder, comment additions in harness)
- 🔵 **Code Quality**: Resolver error message hardcodes `url` — **Still present** (not addressed; remains a minor drift hazard if field set evolves)
- 🔵 **Safety**: Failure-mode operator message generic — **Still present** (the extraction-failure message is unchanged; one-line tightening still beneficial)
- 🔵 **Test Coverage**: AC #3 stub-vs-real-gh conflation — **Still present** (NOT-doing text unchanged from pass 1; the `gh repo set-default` stderr-phrase coupling remains uncovered)

### New Issues Introduced

#### Major (NEW)

- 🟡 **Correctness / Test Coverage / Portability**: **Test 24 grep invocation will fail with "unrecognized option `--json`"**
  **Location**: Phase 3, Section 2 (`assert_grep_empty "..." "$PLUGIN_ROOT/skills/github/" "--json baseRepository"`)
  The pass-1 edit dropped the `\-\-` escaping to match tests 22/23's bare-pattern style. But tests 22/23 patterns start with `gh`, not `--`. With a bare `--json baseRepository` pattern, the helper expands to `grep -rn "--json baseRepository" path/`. Both GNU and BSD grep parse `--<long-option>` regardless of argv position (option parsing stops only at `--` separator or non-option-looking operand). The plan's claim that "non-leading argv position" prevents this conflates the `--` separator mechanism with `--<name>` long-option syntax. Test 24 will FAIL with `unrecognized option` on every run — the regression guard is permanently red, hiding genuine regressions. **Fix**: pass `-e -- "--json baseRepository"` via the helper's variadic `$@`, or extend `assert_grep_empty` to inject a `--` separator before the pattern.

- 🟡 **Compatibility**: **Tightened regex rejects valid GitHub repo names starting with `.` or `_`**
  **Location**: Phase 2, regex `[A-Za-z0-9][A-Za-z0-9._-]*`
  GitHub allows repo names to start with `.` (e.g. `.github`, used by every org with workflow config) and `_`. The pass-1 tightening requires the first char to be `[A-Za-z0-9]`, rejecting these real-world repo names. Test 4b would pass (alphanumeric `team-a`/`repo`), but a PR against a `.github` repo would now silently fail extraction where the prior `[^/]+` regex handled it correctly. This is a real backward-compatibility regression introduced by the tightening. **Fix**: relax the leading-char class for the repo capture (and possibly owner) to `[A-Za-z0-9._]`, add a test case for a `.github` repo URL.

- 🟡 **Correctness / Compatibility / Safety / Portability / Test Coverage**: **Probe parser fragile to gh error-message format changes**
  **Location**: Phase 4 — `grep -v "Unknown JSON field" | tr ',' '\n' | awk '{print $1}'`
  Five lenses converged on variations of this. The parse assumes (a) gh emits a recognisable `Unknown JSON field` line, (b) the rest of stderr is comma-separated bare field tokens, (c) `awk '{print $1}'` correctly extracts the field name. If gh rewords the error (e.g. localisation, `error: invalid JSON field`), reformats with bullets (`- additions`), quotes tokens (`"additions"`), or wraps onto multiple lines, the parser produces garbage that manifests as `'url' NOT in allowlist` FAILs. The probe-format check is manual-only. **Fix**: add a sanity assertion that `allowlist_tokens` is non-empty after parsing despite non-empty stderr; emit a SKIP (not FAIL) with `gh error format may have changed; see meta/work/0071-*.md` if so. Also probe a known-stable control field like `number` and FAIL on the harness itself if the control is missing.

- 🟡 **Portability / Compatibility / Test Coverage**: **Probe invocation lacks PR number / repo context, may not reach field validator**
  **Location**: Phase 4 — `gh pr view --json __ACCEL_PROBE__ 2>&1`
  `gh pr view` has positional-argument validation, repo discovery, and auth checking. Depending on gh's argv-parsing order and the runtime context (CI container, fresh dev machine, unauthenticated shell, outside git checkout), `gh` may emit `accepts 1 arg(s), received 0` or `no git remotes found` or `authentication required` *before* validating `--json`. The harness's empty-stderr guard doesn't catch this — stderr is non-empty, just contains the wrong content. Every field then FAILs with misleading attribution. **Fix**: invoke with `gh pr view 1 --repo cli/cli --json __ACCEL_PROBE__` (or similar known stable public repo) to force the field-validator branch, AND add the format-sanity assertion above.

- 🟡 **Documentation**: **Current State Analysis still describes Phase 4 as probing help output**
  **Location**: Current State Analysis (lines 96-101)
  Line 98-99 says "The real-`gh` smoke check probes the installed `gh`'s help output for the requested field". The new Phase 4 implementation explicitly rejects help-text scraping. **Fix**: rewrite to reflect the error-message-probe approach.

- 🟡 **Documentation**: **Key Discoveries still asserts gh ≥ 2.40.0 floor and references dropped Phase 1 verification**
  **Location**: Key Discoveries (lines 148-151)
  The bullet still asserts `url is in the --json allowlist on every gh ≥ 2.40.0 release` and references `flagged for Phase 1 manual verification against gh 2.40.0 and gh 2.65.0` — but Phase 1 manual verification no longer enumerates these checks, and the Overview now explicitly drops the floor claim. The plan contradicts itself. **Fix**: rewrite this bullet to match the Overview's no-static-claim stance and the smoke-check-is-source-of-truth framing.

- 🟡 **Documentation**: **Phase 2 manual `gh pr view --help | grep` step contradicts Phase 4's explicit rejection of help-text scraping**
  **Location**: Phase 2 Manual Verification (lines 563-572, 599-601)
  Phase 2 instructs the operator to confirm via `gh pr view --help 2>&1 | grep -F -- "url"`, which is exactly the technique Phase 4's harness header rejects on grounds of false-PASS risk from short tokens. **Fix**: either drop the Phase 2 manual help-grep step (Phase 4 supersedes it) or replace it with the same error-message probe used by Phase 4.

#### Minor (NEW)

- 🔵 **Test Coverage**: Test 4c only covers percent-encoded slash in owner — add variant for repo segment and leading-hyphen owner
- 🔵 **Test Coverage**: Counterfactual check is a one-shot manual revert dance; could be a permanent opt-in fixture
- 🔵 **Test Coverage**: Cross-fork manual step has a tripwire — could be run from upstream checkout by mistake (add a precondition check)
- 🔵 **Code Quality**: Allowlist parsing pipeline lacks an inline why-comment on the `awk '{print $1}'` step
- 🔵 **Code Quality**: Header now has two adjacent locations explaining the regex-charset rationale (inline at the regex + header bullet); prune one
- 🔵 **Code Quality**: `__ACCEL_PROBE__` could be lifted to a named constant
- 🔵 **Code Quality**: `grep -qx` membership check is correct but the line-per-token invariant isn't stated at the use site
- 🔵 **Compatibility**: Probe field name `__ACCEL_PROBE__` could theoretically collide with future gh extensions; add empty-allowlist guard
- 🔵 **Compatibility**: URL regex now strictly anchored at `$`; document the trade-off explicitly (future GitHub URL augmentation breaks extraction)
- 🔵 **Safety**: Atomic Phase 1+2 landing enforced only by documentation, not tooling
- 🔵 **Documentation**: Phase 2 success criteria test-count description ("the existing total plus the three new tests (4b, 4c, and the strengthened-but-renumbered 8/9/11)") is ambiguous — three items or five?
- 🔵 **Documentation**: Minor residual overlap between cross-fork bullet and source-of-truth bullet in the rewritten header

### Assessment

The plan is meaningfully better than after pass 1 — 16 of 17 major findings are resolved or proportionately addressed. However, the pass-1 edits introduced two genuine bugs (test 24 will fail with a grep usage error; the regex regresses `.github` repos), three documentation inconsistencies that contradict the new design, and a cluster of probe-robustness gaps that converge across multiple lenses. The test-24 grep issue and the regex regression are blocking — the regression guard will be permanently red, and a class of valid PRs will silently fail extraction. The probe-robustness gaps are the next-most-important; the documentation staleness is mechanically fixable.

Recommend one more pass focused narrowly on:
1. Fix test 24 grep invocation (one-line helper change or `-e --` form)
2. Relax regex leading-char class to accept `.github` and `_*` repos (add tests)
3. Harden Phase 4 probe (PR number + `--repo`, format-sanity guard, SKIP on parse failure)
4. Sweep Current State Analysis, Key Discoveries, Phase 2 manual verification for stale help-output references

After those, the plan should be ready for implementation.

## Re-Review (Pass 3) — 2026-05-18

**Verdict:** REVISE

All seven pass-2 major findings are resolved cleanly. The plan is in materially better shape: test 24's grep invocation works, the regex accepts `.github` repos, the Phase 4 probe is hardened with PR number + repo context + marker-sanity SKIP + control-field check, and the three stale documentation sections were rewritten. However, pass 3 introduced **3 new major findings** — all about the quality of the new safety nets rather than correctness — plus a handful of polish-level minors. The thresholds-based verdict remains REVISE, but the remaining issues are accept-as-trade-off candidates rather than blockers.

Note: the Compatibility lens agent was interrupted mid-run; six of seven lenses returned. Findings below aggregate the six returned plus the pass-1/pass-2 compatibility context.

### Previously Identified Issues (pass 2 majors)

- 🟡 **Correctness / Test Coverage / Portability**: Test 24 grep will FAIL with `unrecognized option --json` — **Resolved** (`-F --` extras correctly terminate option processing before the pattern; traced against `assert_grep_empty` at scripts/test-helpers.sh:322)
- 🟡 **Compatibility**: Tightened regex rejects `.github` and `_*` repos — **Resolved** (split into owner `[A-Za-z0-9][A-Za-z0-9-]*` and repo `[A-Za-z0-9._-]+`; locked by new tests 4d and 4e)
- 🟡 **Correctness / Compatibility / Safety / Portability / Test Coverage**: Probe parser fragile to gh error-format changes — **Partially resolved** (marker-sanity SKIP and control-field check turn most format-drift cases into diagnosable outcomes; some parser-fragility minor findings remain — see "New Issues" below)
- 🟡 **Portability / Compatibility / Test Coverage**: Probe invocation lacks PR number/repo context — **Resolved** (now uses `gh pr view 1 --repo cli/cli --json __ACCEL_PROBE__`)
- 🟡 **Documentation**: Current State Analysis describes Phase 4 as help-output probe — **Resolved**
- 🟡 **Documentation**: Key Discoveries asserts gh ≥ 2.40.0 floor — **Resolved**
- 🟡 **Documentation**: Phase 2 manual `gh pr view --help | grep` contradicts Phase 4 — **Resolved** (replaced with `gh pr view 1 --repo cli/cli --json url` probe)

### Pass-2 minors still present (carried forward, user-accepted or low-priority)

- 🔵 Resolver error message hardcodes `url` — still unchanged (low-priority drift hazard)
- 🔵 Failure-mode operator message is generic — still unchanged (one-line improvement candidate)
- 🔵 AC #3 stub-vs-real-gh conflation in NOT-doing text — still unchanged

### New Issues Introduced (pass 3)

#### Major (3)

- 🟡 **Test Coverage**: **"Auth-absent check" listed under Automated Verification is actually a manual step**
  **Location**: Phase 4 success criteria
  The new bullet says "Reproduce the smoke harness on a machine where `gh auth status` reports unauthenticated...". This is a manual reproduction, not something CI runs. Implementers may assume it runs automatically and skip the manual reproduction. **Fix**: move to Manual Verification, OR convert into an automated check that unsets `GH_TOKEN`/`GITHUB_TOKEN` in a subshell and asserts the harness reports SKIP.

- 🟡 **Test Coverage**: **New Phase 4 SKIP and control-field FAIL paths have no automated coverage themselves**
  **Location**: `test-pr-base-repo-real-gh.sh`
  Pass 3 added the marker-sanity SKIP branch and the control-field FAIL branch, but no test exercises either path with synthetic inputs. A future edit that breaks the SKIP (e.g., reordering so `test_summary` runs twice) or the control-field guard (e.g., a typo in the membership check) would land silently — the diagnostic safety net is itself unprotected. **Fix**: add a self-test sibling harness or two opt-in synthetic cases that stub `gh` to produce (a) stderr without the marker and (b) marker present but empty allowlist.

- 🟡 **Portability**: **`--repo cli/cli` is github.com-only; breaks on GitHub Enterprise**
  **Location**: Phase 4 harness probe
  Operators with `GH_HOST` set to a GHE host will have gh attempt to resolve `cli/cli` against the enterprise host where it does not exist — likely yielding `repository not found` before field validation runs. The marker-sanity SKIP would then SKIP the entire check, silently making the smoke harness invisible to GHE operators (exactly the audience most likely to care about cross-fork-safety on private repos). **Fix**: either (a) discover a real repo via `gh repo list --limit 1 --json nameWithOwner --jq .[0].nameWithOwner`, or (b) document the github.com-only assumption in the harness header and accept the GHE gap.

#### Minor

- 🔵 **Correctness**: awk filter regex `^[A-Za-z][A-Za-z0-9_]*$` accepts English prose tokens (`Available`, `Specify`); first real field can be lost when gh emits prose-prefixed comma list (`Specify one of: additions, ...` — `awk '{print $1}'` returns `Specify`, dropping `additions`). The control-field check on `number` mitigates but doesn't fully cover.
- 🔵 **Correctness**: Control field `number` could itself false-pass if `number` appears as prose. Suggest using a multi-camelCase token like `headRefName` instead.
- 🔵 **Test Coverage**: Test 4e mostly duplicates test 4c — both reject `%2f` via the same regex mechanism; mutation-equivalent.
- 🔵 **Test Coverage**: Control field is a single point of dependency; if `number` is ever dropped, harness FAILs with misleading "parser may be broken" attribution.
- 🔵 **Test Coverage**: Probe-format manual check still gated on a one-shot manual run rather than `allowlist_tokens` having a minimum cardinality.
- 🔵 **Code Quality**: DRY — `gh pr view 1 --repo cli/cli --json X` duplicated across Phase 2 manual verification, Phase 4 harness, and Phase 4 success criteria.
- 🔵 **Code Quality**: `cli/cli` repo choice is a hidden coupling without an explicit fallback note at the use site.
- 🔵 **Code Quality**: Phase 2 manual-verification step `only the field-validation outcome matters` requires implicit gh-error-path knowledge.
- 🔵 **Code Quality**: Regex charset rationale partially duplicated between inline Phase 2 comment and Phase 3 header bullet (with the header using an outdated combined charset for brevity).
- 🔵 **Code Quality**: Phase 4 harness comments lean speculative in the awk-pipeline section (describing two hypothetical gh formats not anchored to documented behaviour).
- 🔵 **Code Quality**: awk filter regex rationale not stated immediately above the line (relies on reader to enumerate defects it guards against).
- 🔵 **Code Quality**: Repo regex `[A-Za-z0-9._-]+` admits leading `-`; not addressed in the inline comment.
- 🔵 **Safety**: Repo regex admits `.` and `..` as repo names (defense-in-depth gap; GitHub never emits such URLs).
- 🔵 **Safety**: SKIP-on-marker-missing means smoke layer is silent on combined "gh changed format AND dropped `url`" failures. Acceptable trade-off.
- 🔵 **Safety**: Control-field collision — if `number` is dropped same release as `url`, attribution is misleading.
- 🔵 **Portability**: Multi-line `skip_test` reason renders awkwardly through `printf '(%s)'` template; readability degrades for log-scrapers expecting single-line SKIP entries.
- 🔵 **Portability**: `printf '%s\n' "$probe_stderr"` truncates at first NUL byte (extremely low-probability practical issue).
- 🔵 **Documentation**: `PROBE_FIELD` and `CONTROL_FIELD` constants would benefit from a one-line inline comment at each declaration.

#### Suggestions
- Phase 2 success criteria — render the four new tests as a sub-bulleted list rather than a parenthetical.
- Phase 4 Auth-absent check — enumerate the exact stderr substring (`authentication required`) an implementer should expect on the SKIP path.

### Assessment

The plan is **substantially ready**. Of the three new majors:
- The "auth-absent check is manual not automated" finding is a labelling fix (one-line move from Automated to Manual block)
- The "SKIP/control-field paths uncovered" finding asks for a self-test sibling — useful but defensible to defer if implementation budget is tight
- The "GHE host" portability gap is real but only affects operators with `GH_HOST` set; documenting the assumption is an acceptable trade-off if dynamic repo discovery is too much surface area

The seventeen minor findings cluster around the inherent fragility of parsing free-form CLI error output. They form a reasonable "things to watch for if the smoke harness ever starts producing puzzling FAILs" list rather than a list of must-fix issues.

**Recommendation**: Land the three major fixes (relabel the auth-absent check; add minimal self-tests for SKIP/control-field paths; document or fix the GHE assumption). Treat the minors as advisory — pick up any that fit naturally during implementation. The verdict remains REVISE only because of the major count; if those three are addressed, the plan is implementation-ready.

## Re-Review (Pass 4) — 2026-05-19

**Verdict:** REVISE

Pass 4 cleanly addresses two of the three pass-3 majors. The auth-absent relabel and the github.com-only documentation are both well-executed. The third (self-tests for SKIP/control-field paths) was addressed via manual walkthroughs rather than automated self-tests; two lens agents (correctness, test-coverage) flagged this as a partial resolution rather than a full one, since manual checks don't survive long-term maintenance.

More importantly, pass 4 introduced **one real correctness bug** in the new manual-verification walkthroughs that needs to be fixed before the plan is ready, plus two process-quality concerns that are accept-as-trade-off candidates.

Note: the Safety lens agent was interrupted mid-run; aggregation below is based on the six returned lens results (Correctness, Test Coverage, Code Quality, Compatibility, Portability, Documentation).

### Previously Identified Issues (pass-3 majors)

- 🟡 **Test Coverage / Documentation**: "Auth-absent check listed as Automated but is actually manual" — **Resolved** (moved to Manual Verification with the expected stderr substring `authentication required` documented)
- 🟡 **Test Coverage**: "New Phase 4 SKIP and control-field FAIL paths have no automated coverage" — **Partially resolved**. Pass 4 added manual walkthroughs (Marker-sanity SKIP, Control-field FAIL) that exercise both branches on demand, but they only run when an operator runs the manual reproduction. Two lens agents flagged this as categorically weaker than the requested automated self-tests; a future regression in the diagnostic branches could land silently.
- 🟡 **Portability**: "`--repo cli/cli` is github.com-only" — **Resolved-via-documentation**. The github.com-only assumption is documented at three locations (use-site comment, top-level docblock, GHE manual-verification step) with explicit degradation path (marker-sanity SKIP) and a named fallback hint (`gh repo list --limit 1 --json nameWithOwner`). All six returned lenses accept the documented trade-off as honest, though the compatibility agent flagged the fallback hint as plausible-but-unverified.

### New Issues Introduced (pass 4)

#### Major (1 high-confidence, 2 high-confidence-but-process)

- 🟡 **Correctness**: **Counterfactual walkthrough will not produce FAIL on the workspace's pinned gh 2.89.0**
  **Location**: Phase 4 Manual Verification — Counterfactual check
  The walkthrough instructs the operator to edit `pr-base-repo.sh` to request `--json baseRepository` and confirm the smoke harness reports FAIL on the pinned `gh 2.89.0`. But the plan's own Current State Analysis explicitly notes that gh 2.89.0 *accepts* `baseRepository` (workspace's automated runs cannot reproduce the bug for this reason). The harness would therefore extract `baseRepository`, find it in 2.89.0's allowlist, and report PASS — not FAIL. The walkthrough's pedagogical claim is internally contradicted by the plan.
  **Fix options**: (a) restrict the walkthrough to gh 2.65.0 and note the version dependency, (b) replace `baseRepository` with a guaranteed-absent synthetic field like `__nonexistent_test_field__` to make the FAIL outcome version-independent, or (c) reframe the walkthrough to demonstrate the "FAIL on absent field" behaviour using a synthetic field rather than `baseRepository`. Option (b) is the cleanest.

- 🟡 **Test Coverage**: **Manual walkthroughs require edit-run-revert dance with no enforcement against forgotten reverts**
  **Location**: Phase 4 Manual Verification (Counterfactual, Marker-sanity SKIP, Control-field FAIL, Auth-absent walkthroughs)
  Four of the six walkthroughs instruct edits to `pr-base-repo.sh` or the harness with an explicit "Revert" step. The Counterfactual revert is caught by test 24 (the tree-state regression guard), but the harness self-modifications (PROBE_FIELD, CONTROL_FIELD) have no guard — a forgotten revert would permanently disable the diagnostic branches without anyone noticing. The Auth-absent and GHE walkthroughs mutate environment variables (`GH_TOKEN`, `GH_HOST`) without explicit cleanup instructions.
  **Fix**: Either (a) add a `git diff --exit-code skills/github/scripts/` precondition note at the head of the Manual Verification block, (b) add explicit "Revert: unset GH_HOST" / "Revert: restore GH_TOKEN/GITHUB_TOKEN" instructions to the env-mutating walkthroughs, or (c) recommend running env-mutating walkthroughs in a subshell (`( GH_TOKEN= bash harness.sh )`) so reverts are automatic.

- 🟡 **Test Coverage**: **Manual coverage is a categorically weaker resolution for the SKIP/control-field automated-coverage gap**
  **Location**: Phase 4 Manual Verification (in lieu of automated self-tests)
  The pass-3 finding asked for either a self-test sibling harness or two opt-in synthetic cases. Pass 4 substituted manual walkthroughs. A future edit that breaks the SKIP path or the control-field guard will land silently and remain undetected until a real gh-error-format change manifests in production. The diagnostic safety net is not protected against its own regressions between manual runs.
  **Fix options**: Either accept this as a documented trade-off, or add a small `test-pr-base-repo-real-gh-self-test.sh` sibling (~30 lines) that stubs `gh` via PATH and feeds canned stderr through the parser to verify both branches.

#### Minor (notable additions in pass 4)

- 🔵 **Correctness**: Marker-sanity SKIP walkthrough outcome depends on `url` being in the operator's installed gh's allowlist; works on 2.65.0 and 2.89.0 but not on any hypothetical future gh that drops `url`.
- 🔵 **Code Quality**: Inconsistent Revert step across the six walkthroughs — Counterfactual, Marker-sanity, and Control-field have explicit "Revert" lines; Auth-absent and GHE walkthroughs mutate env vars without matching cleanup.
- 🔵 **Code Quality**: Mild documentation overlap between the github.com-only paragraph in the harness top-level docblock and the use-site paragraph — both explain the same WHY. Defensible (the duplication is intentional — header for orientation, use-site for operational detail) but not strictly necessary.
- 🔵 **Compatibility**: Dynamic-discovery fallback hint `gh repo list --limit 1 --json nameWithOwner` is plausible-but-unverified as a GHE-coverage replacement — it requires auth, repos with PRs, and matches the user's configured host. Future maintainer following it as a recipe would find it doesn't actually solve GHE.
- 🔵 **Compatibility**: Sequential manual walkthroughs need a "workspace clean / `git diff` empty" precondition at the top of Phase 4 Manual Verification so a forgotten revert from walkthrough N doesn't compromise walkthrough N+1's outcome.
- 🔵 **Compatibility / Test Coverage**: GHE host check is opt-out by absence ("if GHE is not part of the operator's environment, skip this check") — most implementers will skip it, so the documented GHE SKIP path remains unverified in practice. Suggestion: include `GH_HOST=github.acme.invalid` env-var override as a synthetic stand-in, exercising the SKIP path without real GHE access.
- 🔵 **Portability**: The auth-absent recipe (`GH_TOKEN=`/`GITHUB_TOKEN=`) may not actually unauthenticate on macOS keychain-equipped machines — gh's auth precedence consults the keyring independent of env vars. Recommend `gh auth logout --hostname github.com` (with documented restore) or fresh-HOME subshell as alternative.
- 🔵 **Portability**: `PATH=/usr/bin bash harness.sh` reproduction is OS-dependent (gh ships to `/usr/bin/gh` on some Linux distros). Recommend `PATH=$(mktemp -d) bash harness.sh` as the OS-independent primary form.
- 🔵 **Portability**: GHE manual step doesn't tell the implementer how to discover whether they're on GHE — `gh auth status` shows the configured host; add this hint.
- 🔵 **Documentation**: PROBE_FIELD and CONTROL_FIELD constants still lack inline definition comments (carried forward from pass 3, unchanged).
- 🔵 **Documentation**: Auth-absent walkthrough has ambiguous pass condition ("either passes or SKIPs") — implementer can't tell which branch they actually exercised.

### Pass-2/3 minors carried forward (unchanged)

DRY of the probe invocation, regex-rationale duplication, speculative awk-pipeline comments, leading-`-` repo regex admission, resolver error-message hardcoding `url`, generic operator failure message, AC #3 stub-vs-real-gh conflation, test 4e duplicates 4c, control-field collision attribution, repo regex admits `.`/`..`, multi-line skip_test rendering, printf NUL truncation.

### Assessment

Pass 4 is **one fix away from implementation-ready**: the Counterfactual walkthrough's pedagogical claim is genuinely wrong on the workspace's pinned gh and must be fixed. The other two pass-4 majors are process-quality issues that have well-scoped one-line fixes (revert-enforcement precondition; accept self-test gap as documented trade-off or add a small sibling harness).

After those, the plan is solid. The pass-2/pass-3 carried-forward minors form an inherent-CLI-parsing-fragility watch list rather than a backlog of must-fix issues; they document where to look first if the smoke harness ever starts producing puzzling output.

**Recommendation**: Fix the Counterfactual walkthrough's version dependency (one-line change to use a synthetic absent field instead of `baseRepository`), add a workspace-clean precondition note above the manual walkthroughs (one paragraph), and either accept the self-test gap explicitly in the plan or add the optional `test-pr-base-repo-real-gh-self-test.sh` sibling. The plan should then be ready.

## Post-Pass-4 Resolution — 2026-05-19

All three pass-4 majors addressed:

1. **Counterfactual walkthrough version dependency** — Resolved. Plan now uses synthetic absent field `__nonexistent_test_field__` for the demo, making the FAIL outcome version-independent. An explicit NOTE explains why `baseRepository` would not work on the pinned gh 2.89.0.

2. **Manual walkthrough revert enforcement** — Resolved. Added a workspace-clean precondition at the top of Phase 4 Manual Verification (requires `jj diff skills/github/scripts/` clean between walkthroughs). The Auth-absent and GHE walkthroughs were rewritten to use subshell-scoped env-var overrides (`( unset GH_TOKEN GITHUB_TOKEN; bash ... )` and `( GH_HOST=github.acme.invalid bash ... )`), eliminating the manual-revert footgun for env-mutating walkthroughs. The harness-self-modification walkthroughs (PROBE_FIELD, CONTROL_FIELD) retain explicit Revert steps that the precondition note backs up.

3. **Self-test gap** — Accepted as documented trade-off. Added an explicit item to "What We're NOT Doing" naming the rationale (small regression surface, manual walkthroughs cover implementation-time verification, sibling-harness ~30 lines not justified by marginal coverage gain) and the follow-up path (if a regression is discovered in the diagnostic branches, add the sibling then).

Also addressed via the same edits: the macOS keychain edge case in the Auth-absent recipe (added fresh-HOME subshell variant), the GH_HOST discovery hint (added `gh auth status` instruction to the GHE walkthrough), and the GHE-without-actual-GHE-access coverage gap (added synthetic `GH_HOST` override so the SKIP path is exercised even on github.com-only operators).

**Final verdict: APPROVE.** The plan is implementation-ready. The remaining minor findings (carried forward from passes 2-3 plus pass-4 minors not addressed) form an advisory watch-list rather than a backlog of must-fix issues.

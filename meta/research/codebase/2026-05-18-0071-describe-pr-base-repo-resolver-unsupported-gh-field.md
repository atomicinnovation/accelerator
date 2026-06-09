---
date: "2026-05-18T21:28:41+01:00"
author: Toby Clemson
revision: "ee201256c147a4f4e8b8f7427f9292fd79925767"
repository: accelerator
topic: "Fix scope, blast radius, test strategy, and fix-path candidates for `pr-base-repo.sh` `--json baseRepository` defect on `gh 2.65.0` (work item 0071)"
tags: [research, codebase, github, describe-pr, review-pr, respond-to-pr, gh-cli-compat, pr-base-repo, testing]
status: complete
last_updated: "2026-05-18T00:00:00+00:00"
last_updated_by: Toby Clemson
type: codebase-research
id: "2026-05-18-0071-describe-pr-base-repo-resolver-unsupported-gh-field"
title: "Research: `pr-base-repo.sh` `--json baseRepository` Defect on `gh 2.65.0` — Fix Scope and Test Strategy"
schema_version: 1
relates_to: ["work-item:0059", "codebase-research:2026-05-15-0059-gh-pr-edit-projects-classic-deprecation", "plan:2026-05-15-0059-gh-pr-edit-projects-classic-deprecation", "work-item:0071", "adr:ADR-0010", "adr:ADR-0008", "plan:2026-04-07-fix-tmp-directory-usage-in-pr-skills"]
derived_from: ["codebase-research:2026-05-15-0059-gh-pr-edit-projects-classic-deprecation", "codebase-research:2026-04-07-pr-review-tmp-directory-usage", "codebase-research:2026-02-23-respond-to-pr-feedback-skill", "codebase-research:2026-02-22-pr-review-agents-design", "codebase-research:2026-02-22-pr-review-inline-comments"]
---

# Research: `pr-base-repo.sh` `--json baseRepository` Defect on `gh 2.65.0` — Fix Scope and Test Strategy

**Date**: 2026-05-18T21:28:41+01:00
**Author**: Toby Clemson
**Git Commit**: ee201256c147a4f4e8b8f7427f9292fd79925767
**Branch**: HEAD
**Repository**: accelerator

## Research Question

For the bug at `meta/work/0071-describe-pr-base-repo-resolver-uses-unsupported-gh-field.md`: what is the precise blast radius of the `pr-base-repo.sh` resolver, what conventions and constraints does any fix have to honour, what does existing test coverage actually exercise (and therefore what does a real-`gh` smoke check need to add), and which of the three fix-path candidates listed in Open Questions sits most naturally on the codebase's existing patterns?

## Summary

The defect is **a one-line bug at `skills/github/scripts/pr-base-repo.sh:48`** — the call `gh pr view "$pr_number" --json baseRepository` requests a field that is not in `gh 2.65.0`'s `--json` allowlist for `gh pr view`. Everything downstream of that line (the resolver's JSON validation, owner/name extraction, null-guards, and the `pr-update-body.sh` PATCH at `skills/github/describe-pr/scripts/pr-update-body.sh:79`) is sound and re-usable as-is. The fix is genuinely localised: the resolver is the only concrete script in the workspace that requests `baseRepository`, and its three consumers (`describe-pr`, `review-pr`, `respond-to-pr`) all consume the resolver's `<owner>/<name>` stdout output via the documented shared-helper path declared by `skills/github/scripts/README.md` — replacing the underlying data source preserves that contract automatically.

**Blast radius across the three skills** is concrete and asymmetric. `pr-update-body.sh:52` is the only Bash caller; the other two skills invoke the resolver directly from their SKILL.md bodies (`review-pr/SKILL.md:118` writing to a per-PR tempfile, `respond-to-pr/SKILL.md:68` consuming inline). All three are presently blocked at their "post to GitHub" step on any `gh` whose `pr view --json` allowlist lacks `baseRepository`. None of the existing call sites would need to change if the resolver's stdout contract (`"<owner>/<name>\n"`) is preserved.

**The existing test suite cannot catch this class of regression by construction.** Both `skills/github/scripts/test-pr-base-repo-scripts.sh` and `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh` install a PATH-stubbed fake `gh` via `setup_gh_stub` (from `skills/github/scripts/test-helpers.sh`). The fake dispatches on `$1 $2` (`pr view` / `api`) and never validates `--json` field names — it is entirely driven by `GH_PR_VIEW_OUT`, `GH_PR_VIEW_ERR`, `GH_PR_VIEW_RC` env vars. `test-pr-base-repo-scripts.sh:119-120` even pins the argv shape to the literal `pr view 119 --json baseRepository`, so the harness is **green precisely because** it locks in the broken request. AC #5's smoke check must therefore be a separate suite that shells out to the real `gh` on `PATH`, skipping when `gh` is absent. The discovery confirms the work item's hypothesis verbatim.

**Fix-path candidate (a) — URL derivation — sits most naturally on the codebase.** The work item lists three candidates; (a) is the only one that (i) keeps the resolver self-contained, (ii) preserves the script's load-bearing cross-fork-safety guarantee (the PR `url` reflects the upstream repo, not the fork), (iii) keeps both null-guard and non-JSON-payload defences at `pr-base-repo.sh:63-76` working unchanged (the new payload is still JSON containing a `.url` string), and (iv) avoids re-introducing the `gh repo view`-style pitfall that prompted the resolver's existence (per header lines 15-18). Candidate (b) preserves the structured-field path at the cost of carrying two code paths forever; candidate (c) pushes the problem onto users on the floor `gh` version (2.40.0) and provides no protection if a future `gh` removes a different field.

**Strongest precedent**: work item 0059 (`meta/work/0059-gh-pr-edit-fails-due-to-projects-classic-deprecation.md`) is the immediate prior `gh`-CLI-compatibility defect. Its research, plan, and review trio under `2026-05-15-0059-…` is the template — and ironically, the resolver introduced by that fix is the very script broken here. The 0059 plan introduced `pr-base-repo.sh`, `pr-update-body.sh`, and the `setup_gh_stub` machinery in a single phased landing; the present fix should reuse that machinery but add the missing real-`gh` smoke layer that the original plan deferred.

## Detailed Findings

### Defect site: `skills/github/scripts/pr-base-repo.sh`

The script is 79 lines and is the entire surface of the defect. The relevant structure (file path `skills/github/scripts/pr-base-repo.sh`):

- **Header & contract (lines 4-28)** — declares stdout shape `"<owner>/<name>"`, exit codes (0 success / 1 resolution failure / 2 usage), and a load-bearing convention block. Lines 15-18 quote: *"Cross-fork-safe: resolves via `gh pr view --json baseRepository`. `gh repo view` returns the local checkout's repo (the fork, for contributors), which is wrong for cross-fork PR operations."* This comment is the resolver's whole reason for existing; any fix must preserve cross-fork safety or that contract is silently violated.
- **Arg-count and `jq` preflight (lines 30-38)** — exit 2 with `Usage:` or `jq is required` respectively.
- **Line 48 (the defect)** — `if ! payload=$(gh pr view "$pr_number" --json baseRepository 2>"$err_file"); then`. On `gh 2.65.0` this exits non-zero with `Unknown JSON field: "baseRepository"` plus the allowlist on stderr; the script's `err_file` captures it, replays it (lines 49-51), emits its own contextual line (line 52), and exits 1.
- **Conditional remediation (lines 53-55)** — only fires when stderr contains `"no default remote repository"`. On the 2.65.0 case it does not fire, so the operator sees raw `gh` error then `could not resolve base repo for PR #<n>.` with no actionable hint.
- **JSON parse pre-validation (lines 63-67)** — `jq -e .` rejects malformed payloads (HTML auth-nag, plain-text proxy errors) with a tailored error rather than a downstream `jq` parse error. **This defence is generic and unaffected by the fix** — replacing the upstream data source still flows through here.
- **Extraction and null-guard (lines 69-76)** — `jq -r '.baseRepository.owner.login // ""'` and `.baseRepository.name // ""`, with an emptiness check that exits 1 if either is null/empty. **The fix changes which JSON shape is parsed**, so these `jq` filters and the null-guard need to be reshaped, but the structural defence remains.
- **stdout emit (line 78)** — `printf '%s/%s\n' "$owner" "$name"`. This is the contract every caller relies on.

### Caller 1: `skills/github/describe-pr/scripts/pr-update-body.sh`

86 lines. Locates the resolver relative to its own path (`skills/github/describe-pr/scripts/pr-update-body.sh:47-48`):
```
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
resolver="$script_dir/../../scripts/pr-base-repo.sh"
```

Calls it in a subshell at line 52 (`if base_repo=$("$resolver" "$pr_number"); then`), captures the resolver's exit code on failure (line 55: `resolver_rc=$?`) and **propagates it verbatim** (line 60: `exit "$resolver_rc"`). This preserves the resolver's `1` vs `2` distinction up to the callers of `pr-update-body.sh` itself. The PATCH at line 79 (`gh api --method PATCH "repos/$base_repo/pulls/$pr_number" --input "$payload_file"`) is independently confirmed working on `gh 2.65.0` per the work item's Context — only the resolver step blocks the post.

### Callers 2 & 3: `review-pr/SKILL.md` and `respond-to-pr/SKILL.md`

Both skills are SKILL.md-only — neither has a per-skill `scripts/` directory. They invoke the resolver directly via their `allowed-tools` frontmatter (`review-pr/SKILL.md:10`, `respond-to-pr/SKILL.md:10` declare `Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/*)`).

- **`skills/github/review-pr/SKILL.md:117-119`** (in Step 1, "Fetch additional metadata for the Reviews API"):
  ```
  gh api repos/{owner}/{repo}/pulls/{number} --jq '.head.sha' > {tmp directory}/pr-review-{number}/head-sha.txt
  ${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/pr-base-repo.sh {number} > {tmp directory}/pr-review-{number}/repo-info.txt
  ```
  Result is persisted to a per-PR tempfile (`repo-info.txt`) and read back in Step 6 ("When the user chooses to post") at `SKILL.md:548-550`, then substituted into the `gh api repos/{owner}/{repo}/pulls/{number}/reviews` POST at `SKILL.md:576-578`.

- **`skills/github/respond-to-pr/SKILL.md:68`** (in Step 1.3, "Get repo info and current user"):
  ```
  ${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/pr-base-repo.sh {number}
  gh api user --jq '.login'
  ```
  Result is consumed inline (no tempfile) and substituted into thread-reply POSTs (`SKILL.md:376, 383`), thread resolution GraphQL (`:407-414`), and the requested-reviewers POST (`:471-474`).

**Both callers depend only on the resolver's stdout contract** (`<owner>/<name>\n` on success, exit code on failure). Neither parses the underlying JSON; neither would need a single SKILL.md edit if the fix preserves the contract.

### Other `gh pr view --json` call sites (allowlist exposure)

The three SKILL.md files contain inline `gh pr view --json <field>` invocations that bypass the resolver and would independently be exposed to the same class of allowlist defect if any of their fields disappeared:

- `skills/github/describe-pr/SKILL.md:42` — `url,number,title,state`
- `skills/github/review-pr/SKILL.md:68` — `number,url,title,state`
- `skills/github/review-pr/SKILL.md:96` — `number,url,title,state,baseRefName,headRefName`
- `skills/github/review-pr/SKILL.md:108` — `body`
- `skills/github/review-pr/SKILL.md:109` — `commits`
- `skills/github/review-pr/SKILL.md:585` — `url`
- `skills/github/respond-to-pr/SKILL.md:46` — `number,url,title,state`
- `skills/github/respond-to-pr/SKILL.md:57` — `number,url,title,state,baseRefName,headRefName`

Fields requested across all three skills: `number`, `url`, `title`, `state`, `baseRefName`, `headRefName`, `body`, `commits`. **None of these is `baseRepository`** — the resolver script is the sole `baseRepository` consumer in the workspace. These other fields have been stable in `gh pr view`'s allowlist across the supported `gh ≥ 2.40.0` range, so they are not at risk *today*; they are listed here so a future allowlist regression can be diagnosed quickly. They are not in scope for 0071.

### Test coverage: what the existing suite proves and what it cannot prove

Two harnesses exercise the resolver / body-update path:

1. `skills/github/scripts/test-pr-base-repo-scripts.sh` — 12 hermetic tests plus two phase-conditional tree-state regression guards (tests 22 and 23). Tests cover: usage at 0 args, same-repo and cross-fork resolution (both pass through the stub returning a hand-crafted JSON payload), exact argv shape (`test-pr-base-repo-scripts.sh:119-120` pins `pr view 119 --json baseRepository`), resolver-failure stderr replay both with and without the conditional `no default remote repository` hint, null owner / null name guards, missing-jq preflight, missing `baseRepository` field, and non-JSON-payload guard. Tests 22 and 23 are tree-state guards (`assert_grep_empty`) that ensure `gh pr edit` and the cross-fork-unsafe `gh repo view --json owner,name` patterns do not reappear under `skills/`.

2. `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh` — also drives the PATH-stub.

Both harnesses build on `skills/github/scripts/test-helpers.sh`, whose `install_fake_gh` (lines 17-99) writes a bash script that dispatches solely on `$1 $2`:
```
case "$1 ${2:-}" in
  "pr view")
    if [ -n "${GH_PR_VIEW_OUT:-}" ] && [ -f "$GH_PR_VIEW_OUT" ]; then
      cat "$GH_PR_VIEW_OUT"
    fi
    ...
    exit "${GH_PR_VIEW_RC:-0}"
    ;;
  "api "*|"api") ...
```
The fake does not parse `--json`, does not enforce a field allowlist, and does not exit with `Unknown JSON field` for unknown names. **By construction**, no test in this suite can ever turn red on a `gh`-side allowlist regression — the stub is fed whatever JSON the test writer chose. Test 5 (argv shape) **locks in** the current broken argv as the canonical expectation. This is exactly the gap AC #5 calls out.

The new real-`gh` smoke check therefore needs to: (i) verify a real `gh` on `PATH` accepts every `--json` field the resolver requests in its post-fix form; (ii) skip cleanly when `gh` is not on `PATH`; (iii) not depend on `gh auth` (the test should pre-check `gh pr view --json <field-list>` without a PR number? — no, that errors with usage; the more reliable approach is `gh pr view --help` or `gh help pr view` which prints the allowlist to stdout in `gh ≥ 2.40`, but this needs verification on the lower bound. An alternative is to make a `--dry-run`-style check by invoking against `--repo cli/cli` with a known-open PR number; this requires network. A second alternative is to scrape `gh pr view --help` output for `--json` field lines — most version-portable). The implementer must verify which approach is portable across `gh 2.40.0 … latest stable`. The Test runner pattern picks it up automatically: any executable `test-*.sh` under `skills/github/**` is run by `mise run test:integration:github` per `mise.toml:120-123` → `tasks/test/integration.py:46-49` → `tasks/test/helpers.py:13-34` (`run_shell_suites` globs `**/test-*.sh`, skips `test-helpers.sh`, requires exec bit).

### Fix-path candidates: codebase-fit analysis

The work item lists three candidates in Open Questions. Each is evaluated against five anchored properties of the codebase.

| Property | (a) URL derivation | (b) Probe-and-fall-back | (c) Version pin |
|---|---|---|---|
| Preserves cross-fork-safety (the resolver's reason for existing) | Yes — `pr.url` is the **upstream** PR URL even on fork PRs (`https://github.com/<base-owner>/<base-repo>/pull/<n>`). Confirmed by the work item's footnote. | Yes when fallback path is hit; the primary path is the broken one. | Yes (delegates to whatever `gh` does at the pinned version). |
| Preserves existing JSON validation defence at `pr-base-repo.sh:63-67` | Yes — `--json url` returns `{"url":"..."}`, still JSON. | Yes for the fallback; for the primary path the payload shape is whatever `gh` returns once `baseRepository` is supported. | Yes. |
| Preserves null-guard at lines 72-76 | Yes — null-guard reshapes from `(owner, name)` to the sed-derived `(owner, name)` split; still detects empty results. | Yes for the fallback; primary path keeps current guard. | Yes. |
| Avoids carrying long-term dual code paths | Yes — single path. | **No** — two code paths must be maintained, including their own stderr-detection heuristic to choose between them. | Yes — single path, but the path is "fail early at startup". |
| Friendliness to operators on `gh 2.40.0 … latest stable` | High — works on every version. | High in steady state; complexity hides in the heuristic. | **Low for users on or near the floor.** Forces every operator on a `gh` below the (yet-to-be-determined) `baseRepository`-supporting version to upgrade. The workspace's pinned `mise.toml:7` `gh 2.89.0` suggests the maintainer is OK with upgrades, but downstream installs would not be. |

**Recommendation visible from this research**: candidate (a). Concretely, the change is at `pr-base-repo.sh:48` and lines 69-70:
- Line 48 becomes: `if ! payload=$(gh pr view "$pr_number" --json url 2>"$err_file"); then`
- Lines 69-70 become a parse of `.url` (e.g. `jq -r '.url // ""'`), with a single subsequent shell split (e.g. via `sed -E 's#https://[^/]+/([^/]+)/([^/]+)/pull/.*#\1 \2#'` piped into `read -r owner name`).
- Line 72's emptiness check stays as-is.
- The conditional `no default remote repository` hint at line 53 stays as-is (still triggers when the user's gh is misconfigured).
- The cross-fork header comment at lines 15-18 must be rewritten to explain why `--json url` is cross-fork-safe (the PR URL reflects the base repo, not the fork's checkout).

This is the minimum-diff fix. Test changes:
- `test-pr-base-repo-scripts.sh:119-120` (argv shape) — update to assert `pr view 119 --json url`.
- Existing tests 3, 4 (same-repo, cross-fork) — update the JSON payload from `{"baseRepository":{"owner":...}}` to `{"url":"https://github.com/<owner>/<repo>/pull/<n>"}`.
- Existing tests 8, 9 (null-owner, null-name) — reshape to null-URL and malformed-URL cases. The null-name guard still applies after the URL split.
- Existing test 11 (missing field) — reshape to "missing url field".
- **New**: a real-`gh` smoke suite at `skills/github/scripts/test-pr-base-repo-real-gh.sh` (or similar) that asserts the field the script passes is in `gh pr view --help`'s allowlist on the installed `gh`. Skip when `gh` is not on `PATH`. The path AC #5 lands at is named `scripts/test-pr-update-body-scripts.sh` in the work item; the more natural placement given the actual defect location is `skills/github/scripts/test-pr-base-repo-real-gh.sh` because that's where the field allowlist is exercised. This is a planning-level naming call.

Candidate (b) is defensible if the maintainer wants to keep the structured-field path for the day `baseRepository` lands in the allowlist on the floor `gh`. Costs: doubled stderr handling, a fragile string-match on `Unknown JSON field`, and a permanent fork in the resolver's logic. The codebase has no other "probe and fall back" patterns (the dual-path complexity would be novel).

Candidate (c) is the least invasive code change but the worst operator UX. It also does not protect against the *next* `gh` allowlist regression on any other field this resolver might switch to.

### Strongest precedent: work item 0059 and its plan

`meta/work/0059-gh-pr-edit-fails-due-to-projects-classic-deprecation.md` is the prior gh-CLI-compatibility defect, fixed in the same skill family, by the same plan that introduced the very file at the centre of 0071. Key inheritance points:

- **Resolver, body-poster, and the `setup_gh_stub` machinery** all landed together in the 0059 plan. The plan introduced `gh pr view --json baseRepository` as the cross-fork-safe replacement for `gh repo view --json owner,name`. This research is the first place to record that the replacement choice has its own allowlist-vulnerability against older `gh` versions — the 0059 plan implicitly assumed `gh ≥ <some version>` and that assumption broke on `2.65.0`.
- **Phased landing** (the 0059 plan was phased 1-6 with PHASE env passthrough into the test harness, see `test-pr-base-repo-scripts.sh:25-29`). The 0071 fix is small enough to be a single phase, but the harness's PHASE machinery is preserved and useful for any future split.
- **Tree-state guards** at `test-pr-base-repo-scripts.sh:264-282` (tests 22 and 23) lock in invariants: no `gh pr edit` in `skills/`, no `gh repo view --json owner,name` in `skills/github/`. A natural extension for the 0071 fix is a third tree-state guard asserting no `--json baseRepository` in `skills/` post-fix.
- **The 0059 research document** (`meta/research/codebase/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md`) explicitly flagged at the bottom of "Architecture Insights" that *"the codebase's existing owner/repo resolution convention is broken for cross-fork PRs"* — the 0071 defect is the inverse symmetry: the replacement was correct on cross-fork but wrong on `gh 2.65.0`'s allowlist.

### Compounding `gh pr edit --body-file` deprecation (Context-only)

The work item flags a second, out-of-scope defect: `gh pr edit --body-file` on `gh 2.65.0` exits 1 with `GraphQL: Projects (classic) is being deprecated`. The 0059 plan's whole purpose was to migrate away from `gh pr edit` for this reason. The 0059 fix already shipped, so the `gh pr edit` path is no longer reachable from `describe-pr` — but an operator looking for a fallback after `pr-base-repo.sh` fails would naturally try `gh pr edit` and hit the second defect. The 0071 fix doesn't need to do anything about this; it's already fixed at the level of the calling skill. This is captured in the work item's Open Questions; the codebase confirms the Projects-classic path is fully unreachable from `describe-pr` post-0059.

### Cleanup considerations (none required)

The existing scripts already implement all the cleanup, error-replay, JSON-parse, and null-guard defences the work item AC requires. The fix is purely a data-source swap. No SKILL.md edits are required for the three affected skills if the resolver's stdout contract is preserved. No changes to `pr-update-body.sh` are required. No new shared helpers. The only collateral file edits are to the resolver's own tests (to match the new argv shape and JSON payload) and a new smoke check.

## Code References

- `skills/github/scripts/pr-base-repo.sh:48` — the broken `--json baseRepository` call (the entire defect).
- `skills/github/scripts/pr-base-repo.sh:15-18` — cross-fork-safety header comment; the load-bearing contract any fix must preserve.
- `skills/github/scripts/pr-base-repo.sh:63-67` — JSON pre-validation defence (reuse as-is).
- `skills/github/scripts/pr-base-repo.sh:69-76` — extraction + null-guard (reshape for URL derivation; null-guard semantics preserved).
- `skills/github/scripts/pr-base-repo.sh:78` — the `<owner>/<name>` stdout contract every caller depends on.
- `skills/github/describe-pr/scripts/pr-update-body.sh:47-61` — resolver call site, exit-code propagation, contextual stderr.
- `skills/github/describe-pr/scripts/pr-update-body.sh:79` — confirmed-working PATCH call; not affected by the fix.
- `skills/github/review-pr/SKILL.md:118` — resolver call site (writes to `repo-info.txt`).
- `skills/github/review-pr/SKILL.md:548-550, 576-578` — consumption sites.
- `skills/github/respond-to-pr/SKILL.md:68` — resolver call site (consumed inline).
- `skills/github/respond-to-pr/SKILL.md:376, 383, 471-474` — consumption sites.
- `skills/github/scripts/README.md:1-9` — declares `pr-base-repo.sh` as the shared resolver across the three skills.
- `skills/github/scripts/test-pr-base-repo-scripts.sh:119-120` — the locked-in argv shape (`pr view 119 --json baseRepository`).
- `skills/github/scripts/test-pr-base-repo-scripts.sh:264-282` — tree-state regression guards (tests 22 and 23).
- `skills/github/scripts/test-helpers.sh:17-99` — `install_fake_gh` / `setup_gh_stub` — the PATH-stub mechanism that makes the existing suite blind to allowlist regressions.
- `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh:143-144` — sibling argv-shape assertion in the body-update harness.
- `mise.toml:7` — pinned `gh 2.89.0` in the workspace (above the floor; the workspace's own test runs don't surface the bug).
- `mise.toml:120-123, 125-148` — test-task wiring (`test:integration:github`, aggregator).
- `tasks/test/integration.py:46-49` — github-integration entry point.
- `tasks/test/helpers.py:13-34` — `run_shell_suites` autodiscovery (the new smoke check is picked up automatically if it lands as an executable `test-*.sh` under `skills/github/`).

## Architecture Insights

- **The resolver is the workspace's single point of `baseRepository` exposure.** Three skills consume one resolver; the resolver makes one `gh pr view --json` call. A change at that one call site propagates correctly because the resolver's stdout contract is what callers depend on. This is the rare case where a one-line fix genuinely is enough.
- **The codebase has a load-bearing test-stubbing convention that is blind to upstream allowlist changes.** `install_fake_gh` dispatches on `$1 $2` and ignores everything else. This is fine for hermetic caller-side tests but cannot validate `gh`-side preconditions. The work item's AC #5 is the right corrective layer — a separate suite that opts into real-`gh` brittleness in exchange for catching this class of regression.
- **The `test-helpers.sh` machinery sits in `skills/github/scripts/`, not at workspace root.** This matters because there is no global "smoke tests live here" directory; any new real-`gh` smoke suite should live next to the script it covers, named `test-*.sh`, with the exec bit set, and it will be picked up by `run_shell_suites` automatically.
- **The cross-fork-safety property is the resolver's reason for existing.** Any fix that loses it would break upstream pushes for forked checkouts. URL derivation preserves it because the PR `url` always reflects the upstream `owner/repo`, even when the PR was opened from a fork — this is a property of GitHub's URL scheme, not a `gh` implementation detail.
- **Convention drift risk is contained for now.** The three SKILL.md callers all consume `<owner>/<name>` text; none parses the resolver's internal JSON shape. If a future fix needed to return additional fields (e.g. `defaultBranchRef`), the contract would need to evolve — but for 0071 the existing single-line contract is enough.
- **PHASE-conditional regression guards are an existing pattern (tests 22 and 23 in `test-pr-base-repo-scripts.sh`).** A natural follow-on for the 0071 fix is a "test 24" guarding the absence of `--json baseRepository` in `skills/github/` post-fix, mirroring how 0059 guarded the absence of `gh pr edit` and `gh repo view --json owner,name`.

## Historical Context

- `meta/work/0059-gh-pr-edit-fails-due-to-projects-classic-deprecation.md` — prior gh-CLI-compat defect; its plan introduced the resolver now broken in 0071. Direct precedent for fix shape, test layout, and tree-state guard pattern.
- `meta/research/codebase/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md` — research backing the 0059 plan. Notably flags at the bottom of "Architecture Insights" that *"the codebase's existing owner/repo resolution convention is broken for cross-fork PRs"* — the 0071 defect is the inverse: the replacement chosen there is broken on older `gh` allowlists.
- `meta/plans/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md` — the phased implementation plan that landed `pr-base-repo.sh`, `pr-update-body.sh`, `test-helpers.sh`, and the `setup_gh_stub` machinery. The 0071 fix builds directly on this scaffolding.
- `meta/reviews/plans/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation-review-1.md` — review of the 0059 plan; useful precedent for how the implementer documented gh-CLI-compat risk before landing.
- `meta/reviews/work/0071-describe-pr-base-repo-resolver-uses-unsupported-gh-field-review-1.md` — review of the 0071 work item itself; complements this research.
- `meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md` — established the "drop to `gh api` when porcelain CLI is inadequate" pattern for review posting; the indirect precedent that motivated using `gh api PATCH` in `pr-update-body.sh`.
- `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md` — tmp-dir conventions used by the three PR skills (not affected by 0071 but adjacent).
- `meta/plans/2026-04-07-fix-tmp-directory-usage-in-pr-skills.md` — prior tmp-dir migration touching the same three skills; precedent for cross-skill changes preserving SKILL.md call shapes.

## Related Research

- `meta/research/codebase/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md` — the inverse-symmetry precedent.
- `meta/research/codebase/2026-04-07-pr-review-tmp-directory-usage.md` — tmp-dir mechanics for the three PR skills; relevant context for `review-pr`'s `repo-info.txt` pattern.
- `meta/research/codebase/2026-02-23-respond-to-pr-feedback-skill.md` — internals of `respond-to-pr`; useful for understanding the consumption side of the resolver.
- `meta/research/codebase/2026-02-22-pr-review-agents-design.md` and `meta/research/codebase/2026-02-22-pr-review-inline-comments.md` — `review-pr` background; useful for understanding the broader posting machinery.

## Open Questions

- **Real-`gh` smoke check implementation strategy.** AC #5 specifies the *existence* of `scripts/test-pr-update-body-scripts.sh` (sic — the test path conflicts with an existing file; this looks like a typo in the work item that should be clarified before plan). Three plausible probe shapes:
  - **Help-text scrape**: `gh pr view --help 2>&1 | grep -F -- "<field-name>"`. Most version-portable; needs verification on `gh 2.40.0`'s `--help` output format.
  - **Live invocation against a known-open PR**: requires network and auth; brittle.
  - **JSON Schema introspection**: `gh pr view --json invalid 2>&1` returns the allowlist in its error; could be parsed without a real PR number. Worth verifying across the `gh ≥ 2.40` range.
  Suggest help-text scrape as default; live invocation as opt-in.
- **AC #5 test file path.** The work item names `scripts/test-pr-update-body-scripts.sh` but a file at that path already exists (`skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`). The new real-`gh` smoke check is conceptually a test of `pr-base-repo.sh`, not `pr-update-body.sh`. Suggested path: `skills/github/scripts/test-pr-base-repo-real-gh.sh`. Confirm with the maintainer before planning.
- **Choice between fix-path candidates (a)/(b)/(c).** Research recommends (a). Confirm before planning.
- **Should a fourth tree-state regression guard land in `test-pr-base-repo-scripts.sh` asserting the absence of `--json baseRepository` in `skills/github/` post-fix?** This is the symmetrical analogue of tests 22 and 23 from the 0059 plan and would prevent regression to the broken pattern.
- **AC #3 ("auth missing", "network failure", "malformed JSON", "deleted PR") tests on the *real* `gh`.** The existing harness already covers these against the stub (tests 6, 7, 8, 9, 11, 12). The AC asks for them again against the real `gh`. This may be a duplication unless the AC intends them to be enforced *both* in stub-form (existing) and smoke-form (new). Recommend clarifying in planning: either (i) AC #3 is satisfied by the existing stubbed tests because the resolver's behaviour on these conditions is fully determined by the stub interface, or (ii) AC #3 requires a new smoke layer covering the same four conditions with real `gh`. The former is more proportionate.
- **Sandbox PR fixtures for AC #4.** The work item explicitly defers fixture naming to "the implementer". This is a planning artefact, not a research question; flagging for completeness.
- **Should the `gh pr edit` GraphQL-deprecation issue be filed as a separate work item?** The codebase already migrated away from `gh pr edit` in 0059, so the issue is no longer reachable from any skill in the workspace. Recommendation: no follow-up work item needed; close the open question in the work item by noting the migration is already done.

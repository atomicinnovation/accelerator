---
work_item_id: "0071"
title: "describe-pr cannot post PR body — pr-base-repo.sh uses unsupported gh JSON field"
date: "2026-05-18T19:23:06+00:00"
author: Toby Clemson
kind: bug
status: done
priority: medium
parent: ""
tags: [accelerator, github, tooling]
---

# 0071: describe-pr cannot post PR body — pr-base-repo.sh uses unsupported gh JSON field

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The `accelerator:describe-pr` skill writes a correct local PR description but then fails to post it to GitHub. The skill's helper `pr-update-body.sh` delegates to `pr-base-repo.sh`, which calls `gh pr view <n> --json baseRepository` — and `baseRepository` is not in the `--json` field allowlist on `gh 2.65.0` (the current stable release distributed via mise). The resolver exits 1, the helper bubbles up `base-repo resolution failed`, and the operator has no way to complete the skill's last step without dropping to a raw `gh api PATCH` themselves. The same defect is reachable through `accelerator:review-pr` and `accelerator:respond-to-pr`, which use the same resolver.

## Context

- Discovered on 2026-05-18 while running `/accelerator:describe-pr <pr-number>` on `gh 2.65.0 (2025-01-06)` installed via mise.
- Environment in which the failure was observed:

  | Component         | Version                                                                              |
  |-------------------|--------------------------------------------------------------------------------------|
  | `gh` CLI          | `2.65.0 (2025-01-06)` (resolved via mise)                                            |
  | Accelerator skill | `atomic-innovation-prerelease/accelerator/1.21.0-pre.35/skills/github/describe-pr`   |
  | Helper script     | `…/describe-pr/scripts/pr-update-body.sh` (delegates to `…/github/scripts/pr-base-repo.sh`) |
  | OS                | Darwin 25.3.0 (macOS arm64)                                                          |
  | Auth              | `gh auth status` healthy; user has push rights on the upstream repository (no fork involved) |

- Affected scripts (paths relative to this repo):
  - `skills/github/scripts/pr-base-repo.sh:48` — issues the broken `gh pr view --json baseRepository`.
  - `skills/github/describe-pr/scripts/pr-update-body.sh:52` — the immediate caller; surfaces the resolution failure verbatim.
  - The defect was originally observed via the deployed plugin cache (`~/.claude/plugins/cache/atomic-innovation-prerelease/accelerator/1.21.0-pre.35/skills/github/…`); the source-of-truth paths above are where the fix lands.
- `gh 2.65.0`'s `gh pr view --json` allowlist includes `baseRefName`, `baseRefOid`, `headRepository`, `headRepositoryOwner`, `url` — but no `baseRepository`. The underlying GraphQL field exists; the CLI just doesn't surface it under that name in this version.
- A separate, gh-CLI-side issue compounds the operator experience: `gh pr edit --body-file` on `2.65.0` exits 1 with `GraphQL: Projects (classic) is being deprecated…` because the underlying mutation requests `projectCards`. So the natural fallback an operator would try after the helper fails is also broken on the same environment. This second issue is out of scope here — recorded for context only — but raises the cost of the primary defect.
- The underlying REST PATCH (`gh api repos/{owner}/{repo}/pulls/{n} --method PATCH -F body=@…`) works correctly; the helper's exit-1 path is entirely caused by base-repo resolution failing before the PATCH is attempted. This was confirmed by running the PATCH manually after computing `{owner}/{repo}` by hand.

## Requirements

**Reproduction steps**

1. On a machine with `gh 2.65.0` (or any `gh` whose `pr view --json` allowlist does not include `baseRepository`), check out any repository that has at least one open PR.
2. Run the skill's helper directly:

   ```bash
   ~/.claude/plugins/cache/atomic-innovation-prerelease/accelerator/1.21.0-pre.35/skills/github/describe-pr/scripts/pr-update-body.sh \
     <pr-number> /tmp/anything.md
   ```

3. Or, run the underlying resolver in isolation:

   ```bash
   gh pr view <pr-number> --json baseRepository
   ```

**Expected behaviour**

- `gh pr view <pr-number> --json baseRepository` (or whatever path the helper uses to determine the upstream `owner/repo`) emits parseable JSON and exits 0.
- `pr-update-body.sh` PATCHes the supplied body file at `repos/{owner}/{repo}/pulls/{n}` and exits 0.
- The `accelerator:describe-pr` skill completes its Step 9 (post to GitHub) without manual intervention on every `gh` release within the supported range named in Acceptance Criteria.

**Actual behaviour**

- `gh pr view <pr-number> --json baseRepository` prints `Unknown JSON field: "baseRepository"` followed by the allowlist, exits 1.
- `pr-base-repo.sh` exits 1 with `could not resolve base repo for PR #<n>`.
- `pr-update-body.sh` exits 1 with `base-repo resolution failed for PR #<n>`.
- The skill's local artefact is correct; only the GitHub PATCH step fails.

## Acceptance Criteria

The minimum supported `gh` version for this fix is `gh ≥ 2.40.0` (the macOS Homebrew floor as of 2026-05; maintainer may raise the floor with rationale). The required regression matrix is `{gh 2.40.0, gh 2.65.0, gh latest stable}` × `{open same-repo PR, open cross-fork PR}`.

- [ ] Given `gh 2.65.0` is installed and the operator has push rights on the upstream repository, when they run `pr-update-body.sh <pr-number> <body-file>` for a valid open PR, then the helper exits 0 and the PR body on GitHub matches the supplied body file byte-for-byte.
- [ ] Given the resolver is invoked against a PR opened from a fork (i.e. the local checkout differs from the upstream repo), when it is asked for the base repo, then it returns the upstream `owner/repo` — not the fork's `owner/repo` — so cross-fork pushes target the right resource.
- [ ] Given a `gh` release at or above the minimum supported version that genuinely cannot determine the base repo, when the resolver runs in each of the four failure-mode preconditions below, then it exits non-zero with a message containing the indicated substring (case-insensitive) — and never the string `Unknown JSON field`:
    - **Auth missing**: induced by `GH_TOKEN=""` and renaming `~/.config/gh/hosts.yml` aside for the run; message must contain `auth` or `not authenticated`.
    - **Network failure**: induced by setting `https_proxy=http://127.0.0.1:1` (no listener); message must contain `network`, `connection`, or `resolve`.
    - **Malformed JSON from `gh`**: induced by injecting a `gh` stub on `PATH` that writes `not json` to stdout and exits 0; message must contain `JSON`, `parse`, or `malformed`.
    - **Deleted PR / not found**: induced by passing a PR number that does not exist on the repo; message must contain `not found` or `does not exist`.
- [ ] Given the `accelerator:describe-pr`, `accelerator:review-pr`, and `accelerator:respond-to-pr` skills all depend on the same resolver, when the fix lands, then for each `(gh version × PR shape)` cell of the regression matrix above each skill is run against the sandbox PR fixtures (named in Dependencies) and produces the per-skill observable outcome below — the criterion only passes when every cell × skill combination passes:
    - **`accelerator:describe-pr`**: the skill's post step exits 0 and the PR body returned by `gh api repos/{owner}/{repo}/pulls/{n} --jq '.body'` equals the supplied body file byte-for-byte (subject to the same comparison rule as AC #1).
    - **`accelerator:review-pr`**: the skill completes its documented "post to GitHub" step (as defined in its `SKILL.md`) and the resulting review artefact is retrievable via `gh api` (at the endpoint that skill's documentation names) with body content matching the supplied input under the same comparison rule as AC #1.
    - **`accelerator:respond-to-pr`**: the skill completes its documented "post to GitHub" step (as defined in its `SKILL.md`) and the resulting comment/response artefact is retrievable via `gh api` (at the endpoint that skill's documentation names) with body content matching the supplied input under the same comparison rule as AC #1.
- [ ] Given the upstream defect, when the fix lands, then an executable regression test ships in the same change at `scripts/test-pr-update-body-scripts.sh` (next to the helper) that invokes the resolver against the real `gh` binary on `PATH`, skips (does not fail) if `gh` is not installed, and asserts the resolver does not request any `--json` field absent from the running `gh`'s allowlist for `gh pr view`.

## Open Questions

- Which fix path should the maintainer take? Three candidates are on the table:
  - **(a) URL-derivation.** Replace `--json baseRepository` with `gh pr view <n> --json url`, parsing `owner/repo` from the URL path. Works on every `gh` version that has `url` in its `gh pr view --json` allowlist (essentially all releases). One-liner sketch: `gh pr view "$n" --json url --jq '.url' | sed -E 's#https://[^/]+/([^/]+/[^/]+)/pull/.*#\1#'`.
  - **(b) Probe-and-fall-back.** Try `--json baseRepository` first; on stderr containing `Unknown JSON field`, fall back to URL-derivation. Preserves the structured field path on `gh` versions that grow `baseRepository` support later.
  - **(c) Version pin.** Pin a minimum `gh` version in `SKILL.md` and assert it in the helper at startup. Least friendly to the user base but produces an actionable error pointing at the real fix.
- Is the `gh pr edit` GraphQL-deprecation failure also worth a separate work item, or should it stay out of scope here on the assumption that fixing the resolver makes the operator-fallback path of running `gh pr edit --body-file` manually unnecessary in practice?

## Dependencies

- **Blocked by**: nothing — the fix lands in this repository.
- **Blocks**: the three accelerator skills that consume the resolver — `accelerator:describe-pr`, `accelerator:review-pr`, `accelerator:respond-to-pr`. Each is unable to complete its GitHub-post step on any `gh` version whose `pr view --json` allowlist lacks `baseRepository`. Until the fix ships in a plugin pre-release, downstream operators on affected `gh` versions remain blocked.
- **External systems**: `gh` CLI, version range `≥ 2.40.0 … latest stable`. The coupling point is the `--json` field allowlist for `gh pr view` (specifically the presence or absence of `baseRepository`); the fix's correctness is defined against this allowlist's behaviour across the supported range.
- **Tooling for the regression check**: a real `gh` binary must be available on `PATH` in any environment that runs the new smoke check, otherwise the check skips. CI environments that intend to enforce the check must provision `gh` and an authenticated token (sandbox-scoped).
- **Sandbox PR fixtures for AC #4**: the regression matrix requires two open PR fixtures on a sandbox-scoped repository — (i) a same-repo open PR (head and base on the same repo) and (ii) a cross-fork open PR (head on a fork, base on the upstream). Both must remain open for the duration of the matrix exercise. The sandbox repository and PR identifiers are to be designated by the implementer and recorded in this section before AC #4 is attempted; "sandbox-scoped" means a repository created for testing whose contents are non-load-bearing.

## Assumptions

- `gh 2.40.0` is a reasonable minimum support floor (macOS Homebrew floor as of 2026-05). The maintainer may raise it with rationale; the regression matrix in Acceptance Criteria must be updated to match.
- The existing `test-pr-update-body-scripts.sh` mocks or stubs `gh` — this is the working hypothesis for why a real-`gh` version mismatch slipped through and must be verified by the implementer before deciding whether the new smoke check replaces or supplements existing tests.

## Technical Notes

- The fix is small in code terms. `pr-base-repo.sh:48` is the only line that needs to change; the existing `jq` validation downstream (lines 63–76) already defends against malformed JSON, so swapping the data source to `gh pr view --json url` plus a sed/awk path-extraction would land cleanly.
- Cross-fork safety is the load-bearing property of this resolver (per its own header comments). Any replacement must preserve it — option (a) in Open Questions does, because the PR `url` field reflects the upstream repo even on fork PRs.
- The helper's own test script — `scripts/test-pr-update-body-scripts.sh` — is assumed (not verified) to mock or stub `gh`. That assumption explains how the version mismatch slipped through, but the implementer must confirm it by reading the script before deciding whether the new real-`gh` smoke check replaces or supplements the existing coverage. A `gh`-gated smoke check (skipped when `gh` is not on `PATH`) is the minimum new artefact AC #5 requires.
- The underlying REST PATCH path the helper uses (`gh api repos/{owner}/{repo}/pulls/{n} --method PATCH -F body=@…`) is independently confirmed to work on `gh 2.65.0`. The defect is entirely localised to base-repo resolution; no change to the PATCH step is required.

## Drafting Notes

- **Scope decision: primary defect only.** The discovery captured two failures — the accelerator's `pr-base-repo.sh` and `gh pr edit`'s GraphQL deprecation. This work item is scoped to the first because it's the one we can fix; the second is captured in Context for situational awareness and called out as an Open Question.
- **Acceptance criteria are outcome-shaped, not solution-shaped.** The AC deliberately do not prescribe "use `gh pr view --json url`" even though it's the obvious fix; Open Questions lists three candidates and a reviewer might prefer a different one (e.g. pinning gh). The AC tests the observable behaviour either way.
- **Severity: medium, not high.** The skill produces correct local content; only the post step fails; a one-line manual `gh api` invocation recovers. No data loss.
- **Source content inlined.** The full reproduction transcript, environment table, and the three fix-path candidates originally lived in an operator-side capture file (`.accelerator/tmp/2026-05-18-describe-pr-update-body-bug.md`) outside this repository. The load-bearing content has been inlined into Context, Open Questions, and Technical Notes so the work item is self-contained; the original capture is no longer referenced.
- **Private identifiers scrubbed.** The originating PR number, organisation, repo name, project ticket key, and PR title have all been removed from this work item on the assumption they are private. If a reviewer needs them for context, they live only in the operator's chat history.

## References

- `pr-base-repo.sh` source (this repo): `skills/github/scripts/pr-base-repo.sh`
- `pr-update-body.sh` source (this repo): `skills/github/describe-pr/scripts/pr-update-body.sh`
- gh CLI release notes: https://github.com/cli/cli/releases/tag/v2.65.0
- gh CLI `pr view --json` field allowlist (varies by release): https://cli.github.com/manual/gh_pr_view
- GitHub Projects (classic) sunset: https://github.blog/changelog/2024-05-23-sunset-notice-projects-classic/

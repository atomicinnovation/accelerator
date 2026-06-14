---
type: issue-research
id: "2026-06-14-release-concurrency-group-blocks-prereleases"
title: "Investigation: Shared release concurrency group blocks prereleases while a stable release awaits approval"
date: "2026-06-14T11:41:37+00:00"
author: "Toby Clemson"
producer: research-issue
status: complete
topic: "GHA concurrency group held by an approval-gated release job blocks subsequent prereleases"
tags: [research, debugging, ci, github-actions, release, concurrency]
revision: "0aa2e8912017c76a65770dbd9fd97a4a08fbb325"
repository: "miscellaneous"
last_updated: "2026-06-14T11:41:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Investigation: Shared release concurrency group blocks prereleases while a stable release awaits approval

**Date**: 2026-06-14 11:41 UTC
**Author**: Toby Clemson
**Git Commit**: 0aa2e8912017c76a65770dbd9fd97a4a08fbb325
**Branch**: main
**Repository**: accelerator (workspace: miscellaneous)

## Issue Description

A concurrency group (`accelerator-release`) was recently added to CI to prevent
two releases from running at the same time. Two release jobs share it: an
automatic `prerelease` job (runs on every push to `main`) and an
approval-gated `release` job (manual approval via a `release` environment). The
intent is: prerelease on every merge, and approve a stable release only when
ready.

Observed: the pipeline **blocks prereleases**, making each new push's
prerelease wait on the *previous* pipeline's stable release. Because the stable
release is approval-gated and may sit unapproved for a long time, prereleases
stall indefinitely. Desired: prevent concurrent releases *across* release type,
while still allowing subsequent pipelines to prerelease.

## Input Classification

Mixed — a behavioral description ("pipeline blocks prereleases") plus concrete
structural detail (two jobs sharing one concurrency group; `release` is
environment-gated).

## Affected Components

- `.github/workflows/main.yml:184` — `prerelease` job; `concurrency: { group:
  accelerator-release, cancel-in-progress: false }`, `if: push`, runs after all
  checks.
- `.github/workflows/main.yml:202` — the shared `concurrency` block on
  `prerelease`.
- `.github/workflows/main.yml:236` — `release` job; `needs: prerelease`,
  **`environment: release`** (the approval gate, line 241), and the **same**
  `concurrency: { group: accelerator-release, cancel-in-progress: false }`
  (line 244).
- `tasks/release.py:23` — `_publish()` = commit → tag → **push** → create
  release → upload; the work that genuinely must be mutually exclusive.
- `tasks/release.py:36,56` — `prerelease_prepare` / `release_prepare` both do
  `git pull` → version bump → build; the prepare→finalise split exists so SLSA
  attestation can be interleaved between build and publish.

## Timeline / Reproduction

1. Push A lands on `main`. All checks pass; `prerelease` (A) acquires group
   `accelerator-release` and runs (bump pre, build, commit, tag, push).
2. `prerelease` (A) completes and releases the group. The `release` (A) job's
   `needs: prerelease` is now satisfied, so it becomes eligible — but it has
   `environment: release`, so it enters the **"Waiting"** state pending manual
   approval.
3. **A job in the "Waiting" approval state holds its concurrency group.** So
   `release` (A) now occupies `accelerator-release` and will keep holding it
   until a human approves (or rejects/cancels) it — potentially hours or days.
4. Push B lands. Its `prerelease` (B) job wants `accelerator-release`, but
   `release` (A) holds it. With `cancel-in-progress: false`, `prerelease` (B)
   goes **pending** and cannot start.
5. Net effect: the group is effectively locked from the *start* of
   `prerelease` (A) straight through to the *approval + completion* of
   `release` (A). Every subsequent push's prerelease stalls behind the
   unapproved stable release. This is exactly the reported behavior.

## Hypotheses

### Hypothesis 1: The approval-gated `release` job holds the shared concurrency group while in the "Waiting" state
- **Evidence for**: Both jobs share the literal group `accelerator-release`
  (`main.yml:203` and `:245`). `release` carries `environment: release`
  (`:241`). Authoritative confirmation (GitHub community Discussion #17401,
  reproduced behavior; consistent with the official model that "concurrency and
  environment are not connected" and concurrency is acquired at *queue* time,
  not at approval time): a job sitting in the **Waiting** approval state
  **occupies its concurrency group** and blocks any later run in the same group
  — the later run cannot even reach its own approval prompt. This matches the
  report precisely: prereleases queue behind an unapproved stable release.
- **Evidence against**: None.
- **Verdict**: **Confirmed.**

### Hypothesis 2: `cancel-in-progress: false` is itself the cause
- **Evidence for**: It does cause queued runs to *wait* rather than abort, which
  superficially resembles "blocking".
- **Evidence against**: `cancel-in-progress: false` is the *correct, desired*
  setting — it stops an in-flight release from being killed mid-push. The
  blocking is caused by *which* job holds the group (the Waiting release), not by
  the wait-vs-cancel policy. Flipping it to `true` would let new pushes cancel an
  in-progress release — strictly worse.
- **Verdict**: **Eliminated** as root cause (the setting is correct and should
  stay).

### Hypothesis 3: The "one pending slot per group" eviction rule contributes
- **Evidence for**: A concurrency group allows at most **one running + one
  pending** run; a newly-queued run **cancels the previously pending** run and
  takes its slot (this is independent of `cancel-in-progress`, which only
  protects the *running* job). So even after Hypothesis 1 is fixed, a pending
  release can be evicted by a later-queued prerelease (and vice versa) when both
  contend for the single pending slot behind a running release.
- **Evidence against**: This is not what produces the *currently reported*
  blocking — that is fully explained by Hypothesis 1.
- **Verdict**: **Inconclusive / contributing** — not the present cause, but a
  real residual risk that the chosen fix must account for (see Contributing
  Factors and Recommended Fix).

## Root Cause

The approval-gated stable-release job (`release`) and the automatic
`prerelease` job **share one job-level concurrency group**
(`accelerator-release`), and the `release` job carries *both* the
`environment: release` approval gate *and* the concurrency key. GitHub Actions
acquires a concurrency group when a job is **queued**, not when it is approved —
so a `release` job sitting in the **Waiting** approval state holds the group for
its entire (unbounded) approval wait. Because `prerelease` shares that group,
every subsequent push's prerelease is blocked until the pending stable release
is approved and finishes.

The mutual-exclusion intent was correct (the actual commit/tag/**push** in
`tasks/release.py:_publish` must never run concurrently across release types,
or two pipelines race their pushes / collide on a version). The mistake is
**scope**: a job-level concurrency group holds the lock for the job's *entire
lifetime, including the approval wait*, not just for the active release window.

## Causal Chain

1. `prerelease` (A) runs and releases the group on completion.
2. `release` (A) becomes eligible (`needs: prerelease` met) but is approval-gated
   → enters "Waiting".
3. The "Waiting" `release` (A) job **acquires/holds** `accelerator-release` at
   queue time, before any human approves it.
4. Push B's `prerelease` (B) requests the same group → blocked, goes pending
   (`cancel-in-progress: false` → wait, don't abort).
5. Prereleases stall until `release` (A) is approved and completes — defeating
   "prerelease on every merge".

## Contributing Factors

- **Concurrency is evaluated at queue time, decoupled from the environment
  approval** — non-obvious GHA semantics; the `environment` and `concurrency`
  keys look independent but interact badly when placed on the same job.
- **The approval wait is unbounded** — a human may not approve a stable release
  for days, so the held lock has no natural time bound.
- **One-pending-slot eviction rule** — any fix that keeps both job types in the
  same group must accept that a pending run can be superseded by a later-queued
  run of either type (only the latest pending survives behind a running release).
- The real exclusivity requirement is narrow: only `_publish` (commit/tag/push)
  truly races; the build/prepare halves do not need the global lock.

## Fix Options

| Option | Description | Risk | Effort |
|--------|-------------|------|--------|
| A | **Split the approval gate from the release work.** Add a lightweight `approve-release` job with `environment: release` and **no** `concurrency` (just an "approved" no-op step), `needs: prerelease`. Move the `concurrency: { group: accelerator-release, cancel-in-progress: false }` onto the `release` work job and **remove `environment`** from it; make `release` `needs: approve-release`. The unbounded approval wait no longer holds the group; the group is acquired only when the *approved* release work is queued. | Low | Low |
| B | **Move stable release to a separate `workflow_dispatch` workflow.** Prerelease stays automatic on push (keeps the group); stable release becomes a manually-dispatched workflow that shares the same group string. The "approval" becomes the manual dispatch. Fully decouples the wait from the pipeline. | Med | Med |
| C | Drop `concurrency` from `release` and serialise via an external/repo lock (e.g. a lock branch or API check). | High | High |

## Recommended Fix

**Option A.** It is minimal, idiomatic (it is the documented community
workaround for exactly this scenario), directly removes the root cause, and
preserves the in-pipeline "approve when ready" UX. Concretely:

```yaml
  approve-release:
    name: Approve release
    runs-on: ubuntu-latest
    if: github.event_name == 'push'
    needs: prerelease
    environment: release          # the approval gate lives here — NO concurrency
    steps:
      - run: echo "Stable release approved"

  release:
    name: Create release
    runs-on: macos-latest
    if: github.event_name == 'push'
    needs: approve-release        # only queued once approved
    concurrency:                  # the lock lives here — NO environment
      group: accelerator-release
      cancel-in-progress: false
    permissions: { id-token: write, contents: write, attestations: write }
    steps:
      # ... unchanged release:prepare / attest / release:finalise / post-stable prerelease
```

Trace under the fix: `prerelease` (A) runs and releases the group →
`approve-release` (A) waits for a human **without holding the group** → push B's
`prerelease` (B) acquires the group and runs normally. When (A) is approved, the
`release` work job queues for the group and serialises against any running
prerelease — preserving mutual exclusion exactly where it matters (the
commit/tag/push window).

**Residual risk to document and accept (Hypothesis 3):** with both job types in
one group, the single pending slot means a just-approved `release` that is
pending behind a *running* `prerelease` can be evicted if a new `prerelease`
queues (and vice versa). Probability is low (requires a push to land precisely
during that short window) and the consequence is recoverable (re-run the
cancelled release job). Mitigations if it proves troublesome: re-dispatch the
release, or adopt the newer `queue: max` concurrency option once confirmed
available on the runner version (it was shipped but not yet in stable docs as of
early 2026 — verify before relying on it).

Note `tasks/release.py:release_prepare` does `git pull` + `version.bump(FINALISE)`
at execution time, so a late-approved release correctly finalises against current
`main` HEAD rather than the original push — Option A does not change or harm this.

## Prevention

- When combining `environment` (approval) with `concurrency` on a release/deploy
  job, **never put both on the same job** — gate on one job, lock on a
  downstream job. Treat the concurrency group as covering only the *active
  work*, never an approval wait.
- Add a short comment in `main.yml` next to the group explaining that the lock
  must not enclose the approval gate (the existing comment at `:197` documents
  the serialisation intent but not this pitfall).
- Consider a CI smoke check / doc note that approval gates and concurrency locks
  are placed on distinct jobs.

## Recent Changes

`jj` history on `.github/workflows/main.yml` shows the shared group was
introduced by change **`nqkwknlr` "Make the release push atomic and serialise
release pipelines"** — the same change that added `accelerator-release` to both
jobs. This correlates exactly with the onset of the blocking behavior: before
it, the jobs did not share a lock; after it, the Waiting `release` job began
holding the group against subsequent prereleases.

## Open Questions

- Is the rare pending-slot eviction (Hypothesis 3) acceptable operationally, or
  should `queue: max` / a separate dispatch workflow (Option B) be adopted to
  guarantee no approved release is ever dropped?
- Should the post-stable prerelease steps (`main.yml:278-291`) remain inside the
  `release` job (they do, and stay within the same lock), or move to the normal
  prerelease path?

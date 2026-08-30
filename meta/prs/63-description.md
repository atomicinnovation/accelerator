---
type: "pr-description"
id: "63"
title: "Pin the cache-root probe to at most one attempt per dispatch"
date: "2026-08-12T15:44:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0189"
parent: "work-item:0189"
relates_to: ["work-item:0169", "work-item:0186", "work-item:0205"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/63"
pr_number: 63
tags: ["cli", "launcher", "bootstrap", "documentation"]
revision: "26ccd657d2ed7725a09c00736194c7bb1b7b1162"
repository: "accelerator"
last_updated: "2026-08-12T15:44:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Pin the cache-root probe to at most one attempt per dispatch

## Summary

Work item 0189 was filed as a bug — the launcher probes the cache root on every dispatch — and the premise turned out to be false: 0169's Phase 5 had already moved the probe off the warm path. This PR re-scopes the item into a regression guard and delivers it, making the at-most-once property observable, structural, and demonstrably capable of failing. It also corrects the documentation 0169 falsified and retracts the stale blocker relationships 0189 was believed to have with 0169 and 0186.

The property already held. The work was to prove that a regression would be noticed.

## Changes

**The probe counter and the invariant tests**

- Add a thread-local `PROBE_ATTEMPTS` count incremented as the first statement of `verify_writable`, plus a `pub fn probe_attempts()` accessor, so the count equals the invocation count on every path — including the one where `create_dir_all` fails and no probe file is ever written. The pre-existing `SEQUENCE` atomic cannot serve: its `fetch_add` sits *after* the `create_dir_all` guard, so it counts probes that reached the write stage rather than calls. `SEQUENCE` keeps its filename-uniqueness meaning unchanged.
- Thread-local rather than a process-wide atomic, so the assertions are sound under a bare `cargo test` (parallel threads in one process) as well as under nextest. No `nextest.toml` test group and no recorded runner precondition are needed.
- Add eight delta assertions in `cli/launcher/tests/resolution.rs` — six new tests, one per branch of `FetchVerifyCacheResolver::resolve` that can reach the probe plus a two-resolution memoisation guard, and deltas retrofitted onto two existing tests. The retrofits are the load-bearing pair: `a_signature_read_io_error_propagates_the_refetch_error_verbatim` is what pins the plain cache-I/O refetch arm by error shape, and `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss` is the only assertion covering a *failing* probe.
- Add `Harness::seed_cache`, `clear_cache`, `resolver_for` and `resolve_offline`. `seed_cache` writes a genuinely verifiable entry with zero prior probes, which is what lets the warm-hit delta of 0 mean what it claims; its findability postcondition is asserted rather than assumed, since three tests expect a delta a cold miss would also produce. `resolve_offline` collapses three verbatim copies of the unreachable-server construction.

**Narrowing the probe's reach**

- Delete `cache_root::resolve`, which had no production caller and was `pub`, so `dead_code` never flagged it.
- Narrow `verify_writable` to `pub(super)`. `verify_writable` now has exactly one production call site, and because `main.rs` is a separate crate from the `accelerator` lib, the composition root can no longer probe at all — a compile error rather than a review question. This does **not** make the invariant compiler-enforced: both violating call sites live inside the permitted scope, and within the package the invariant remains test-enforced.
- Re-home two `cache_root::resolve` unit tests onto `candidate`, replace one with `verify_writable_creates_a_missing_directory` (the create-if-needed assertion no other test discharged), and drop one already covered by `verify_writable_rejects_a_read_only_directory`.

**Documentation 0169 falsified**

- `docs-site/src/content/docs/internals.md` told users that dispatching to a separate binary always makes the launcher probe. It now states that warm sub-binary dispatch neither writes nor probes, names the three cold triggers, and splits the `--fail-safe` behaviour by failure arm: an integrity failure becomes a `Refusal` (exit 2, and a `PreToolUse` block), while plain cache I/O and first-use or version-bump misses are swallowed at exit 0. Both over-broad readings were wrong, and the shipped wording avoids each.
- Correct the same claim in the `[Unreleased]` changelog entry, which 0186 had documented in both files as a pair, and which ships with 1.24.0 if left.
- Rewrite the `cache_root` module header, which described the deleted function and stated `candidate`'s precedence backwards, and fix "resolved cache root" → "selected" in `resolve/mod.rs`.

**Two stale runtime pointers, and a guard**

- `skills/visualisation/visualise/SKILL.md` and `hooks/launcher-link-refresh.sh` both pointed users at `docs/internals.md`, a path that has never existed in this repo. Both now use the published URL, since each reaches a plugin *user* at runtime and someone running an installed plugin has no checkout. The site-relative form would not work: the SKILL.md body renders three directories deeper than `internals.md`, and `starlightLinksValidator` runs with `errorOnRelativeLinks: false`, so nothing would have caught it.
- Add `tests/unit/tasks/test_docs_anchors.py`, asserting the heading and both pointers against a URL composed from the `astro.config.mjs` defaults that own the hosting decision, so a domain move breaks the pointers directly rather than leaving the guard green against a stale literal. Python per ADR-0048, which makes it the test language for the non-Rust surfaces; it joins `test:unit:tasks` beside `test_docs_theme_drift.py`, the existing docs-site drift guard, so no new task registration applies. No by-name `_REQUIRED_CONFIG_SUITES` entry either — that guards against a bash suite renamed off the `test-*.sh` glob, which pytest collection does not do. `_EXPECTED_CONFIG_SUITES` still moves 15 → 16, correcting a floor left one behind the real discovered count.

**Planning and provenance**

- Re-scope work item 0189 from the false bug premise to a regression guard, splitting the deferred latency measurement into a sibling plan. The two share no code, no file and no test, and bundling them would have made a CI-verifiable refactor's closure depend on a one-shot single-host measurement.
- Retract the stale blocker relationships in 0169 and 0186: 0189 does not gate either story's latency work, and the release blocker on a signed `accelerator-vcs` asset is stale — `v1.24.0-pre.35` and `-pre.36` both ship it.
- Add work item 0205, a spike to close the warm-dispatch measurement *method*. Three attempts to specify it in prose have failed review, each a design failure rather than a measurement failure, so the sibling plan is `blocked_by` the spike rather than carrying a fourth unreviewed methodology.

## Context

- Work item: `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`
- Research: `meta/research/codebase/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`
- Plan and validation: `meta/plans/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`, `meta/validations/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee-validation.md`
- Sibling plan (drafted, not implemented here): `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md`, blocked on `meta/work/0205-close-the-warm-dispatch-measurement-method.md`
- Upstream: 0169's Phase 5 delivered the split this PR guards; 0186 established the mutation-recording pattern Phase 2 follows

## Testing

- [x] `mise run` (bare default task) exits 0 end-to-end — the repo's definition of done
- [x] `mise run check`, `mise run docs:check`, `mise run cli:check` all exit 0
- [x] `ACCELERATOR_COVERAGE=off mise run test:unit:cli`, `test:integration:config` and `test:integration:hooks` all pass
- [x] The resolution binary reports 25/25 with **0 skipped**, and `minisign` 0.12 resolves via mise — so the eight delta assertions genuinely execute rather than returning `Ok(())` through `skip_if_no_minisign!`, which nextest would report as PASS
- [x] Every assertion was authored under a mutation that reddens it, then made green by reverting. The full 4 × 8 mutation sweep is recorded in the plan's Validation Results, including the greens and two collateral reds
- [x] Mutations A and B independently re-applied during validation. Under A the cold-miss delta is exactly 2 and the warm-hit assertion stays green; under B eight tests redden including the warm-hit zero. `#[track_caller]` names each test's own line rather than the shared helper's
- [x] Three fixture postconditions demonstrated capable of failing — a degraded seed, a non-clearing `clear_cache`, and an unlinked cache entry — since no probe mutation touches them
- [x] `tests/unit/tasks/test_docs_anchors.py` fails closed: each of the four guarded facts mutated in turn — heading renamed, heading replaced, a pointer repointed off-site, a repository path reintroduced, the `astro.config.mjs` `site` default removed — reddens the relevant test
- [ ] The assembled `cache_root` test binary run as root on linux. The fixture's privilege-independence was confirmed by reproducing the arrangement as uid 0 in `rust:1.90-slim` (`create_dir_all` fails `ENOTDIR`, no residue), but the test binary itself was not run as root
- [ ] Warm-call latency `G ≤ 1.1 × B` on one darwin host. Deferred to the sibling plan by design; this PR closes on CI evidence alone

## Notes for Reviewers

**Where to focus.** The `pub(super)` narrowing is a boundary change, not a proof — Phase 3 states precisely what it does and does not buy, and both Phase 2 mutations compile unchanged under it. If you disagree that a test-enforced invariant plus a narrowed boundary is enough, the alternative is the resolver restructure the work item puts out of scope, and the plan records the cost of that choice rather than presenting it as a win: an injected per-resolver counter would have removed both the public accessor and the new coupling to `cache::store`.

**A known blind spot, recorded rather than implied.** The thread-local bound is asymmetric. A probe *moved* onto another thread still reddens the cold-miss deltas; a probe *added* on another thread — a prefetch, a background warm, a `doctor` built-in — is invisible to all eight assertions. None of the four mutations exercises that shape.

**The memoisation ban has a stated expiry.** It is correct only while a launcher process performs a single resolution. Should a subcommand ever resolve several sub-binaries in one process, per-process memoisation becomes the right design and `each_of_two_cold_misses_probes_the_cache_root_once` is the criterion to retire deliberately rather than fight. Note too that the canary only catches a memo at or above `verify_writable`'s entry, since the increment is that function's first statement.

**One correction landed during validation.** The recorded Mutation A cell for the warm-hit test was red, contradicting both the prediction and its own footnote — the test did not exist when A was in force. Re-running A shows it green; the cell is corrected and the observation appended as a dated note, which also discharges the last clause of the work item's criterion 6.

**Work item 0189 is still `status: draft`** despite the implementation being complete and validated. Worth a decision on whether it moves before or after the sibling measurement plan, since 0189 cannot close on this PR alone.

**Scope note.** The sibling latency plan and work item 0205 are included as drafted artefacts only — no measurement code lands here. `meta/` accounts for the bulk of the diff; the shipped behaviour change is confined to `cli/launcher/`, and the rest is documentation, the anchor guard, and provenance.

---
type: pr-description
id: "81"
title: "Batch the bootstrap's two shim hashes into one sha256 invocation"
date: "2026-08-24T23:03:59+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "work-item:0191"
parent: "work-item:0191"
relates_to: ["work-item:0186", "work-item:0189", "work-item:0205", "work-item:0169"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/81"
pr_number: 81
tags: [shell, performance, bootstrap, bash-3.2, sha256]
revision: "1f6fa3a5ebfb81426f6885b79f997d17eb63e29c"
repository: "accelerator"
last_updated: "2026-08-24T23:03:59+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Batch the bootstrap's two shim hashes into one sha256 invocation

## Summary

The `bin/accelerator` bootstrap hashed the verify-shim source and its staged copy in two separate sha256 invocations, each also forking an `awk` to strip the path column — four forks on the warm path. This collapses them into a single backend fork over both files with no `awk`, a measured ~2.35 ms saving on the fast Apple `sha256sum` backend and ~9.07 ms on the Perl `shasum` fallback, with the content-addressed trust boundary preserved exactly.

## Changes

- **Batched digest with a variadic helper** — `sha256_file` becomes `sha256_files`, one backend fork over all arguments, no `awk`, raw `<digest>␣␣<path>` lines. Stderr is suppressed inside the helper (not at the call site) so the backend fork stays visible to the `bash -x` trace seam while genuine missing-file errors are still discarded.
- **Glob-discovered staged copy** — the staged shim's name embeds the source digest, so it cannot be located until the source is hashed. It is instead discovered by globbing the cache dir, admitting only a regular executable whose name-suffix is a strict 64-character lowercase-hex digest, then hashed together with the source in one call.
- **Path-column keying, not output position** — both the source and staged digests are matched by their path column, so a candidate that vanishes mid-run (a concurrent bootstrap's `mv`) degrades to a benign re-stage instead of promoting a candidate's digest to the trust anchor. The hex-suffix constraint is load-bearing: it makes the path-column parse unforgeable (backends other than GNU coreutils do not escape newlines or spaces in filenames) and keeps `.staging.$$` temps out of the hashed set.
- **Tests** — seven additions to `tests/integration/entrypoint/test_accelerator_entrypoint.py`: warm and cold fork-count (one backend fork, zero `awk`), the forced-`shasum`-fallback batched form, stale-shim rejection (the AC-4 digest-keying guard), plus two shim-path helpers. The three planted-stub trust-boundary tests pass unmodified.
- **Measurements and lifecycle trail** — `warm-dispatch-4.json` records the before/after evidence; the work item, plan, reviews, research, and validation report round out the 0191 trail.

## Context

Implements work item [0191](../work/0191-batch-the-two-shim-hashes-into-one-invocation.md) via plan [2026-08-22-0191-batch-shim-hashes](../plans/2026-08-22-0191-batch-shim-hashes.md). Follows the bootstrap fork-count work in 0186 and 0205; the warm-dispatch ratio drop is evidence for tightening 0189's C5 threshold. Validation report: [2026-08-22-0191-batch-shim-hashes-validation](../validations/2026-08-22-0191-batch-shim-hashes-validation.md).

## Testing

- [x] Entrypoint integration suite green (`63 passed`), including the seven new tests and the three unmodified planted-stub trust tests.
- [x] `scripts/lint-bashisms.sh bin/accelerator` clean — the empty-array-under-`set -u` branch honours the bash 3.2 floor.
- [x] `mise run check` exits 0 (format + lint + types across all four components).
- [x] Bare `mise run` default task passes end-to-end locally.
- [x] Digest-bracket and warm-dispatch measurements recorded from interleaved n=200 samples on darwin-arm64; after strictly below before on both backends.
- [ ] Linux CI lane confirms the GNU coreutils `sha256sum` multi-file format green — cannot run pre-push; the standing gap this PR opens the branch to close.

## Notes for Reviewers

The trust boundary is the review focus: the hex-suffix constraint plus the exact `[[ path == "${shim}" ]]` match are what keep the batched output parse unforgeable, and the batched call deliberately fails open (no `|| fail_integrity`, TOCTOU-tolerant) while the zero-candidate call still fails closed. A documented domain constraint stands: a cache or plugin-root path containing a newline or backslash degrades to a benign re-stage, guarded by the existing `test_a_record_is_always_one_line`. The production diff is small (`bin/accelerator` plus tests); the bulk of the changed lines is the 0191 meta lifecycle trail. Follow-ups 0215 (remove the cache-hit sha256) and 0216 (cheapen the source digest) are sequenced independently and out of scope here.

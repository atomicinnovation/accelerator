# 0196 warm-path measurement: `run.sh` against `accelerator design executor`

A crawl makes 100–200 executor invocations, and this migration changes what
happens on each one. This records the comparison, because the shell baseline is
deleted in the same change that measures it — there is no repeatable CI gate
here, only this one-time result.

**Gate: the ratio (port ≤ shell). PASS.** The delta against the plan's
20–45 ms expectation is recorded as an observation, not a threshold, so a
smaller-but-real improvement would still pass and simply be noted as smaller
than hoped.

## Result

Measured 2026-08-13 on darwin-arm64, 50 interleaved samples per variant from a
single Python process using `perf_counter`, order alternated on each iteration
so any drift in machine state lands on both variants equally.

| Variant | min | median | p90 |
|---|---|---|---|
| `run.sh ping` (shell) | 102.55 | **108.38** | 117.88 |
| `accelerator design executor ping` (Rust) | 40.84 | **43.95** | 46.32 |

All figures in milliseconds.

- **Ratio (Rust ÷ shell): 0.406** — the gate is ≤ 1.0, so this passes with
  substantial headroom.
- **Delta: 64.43 ms per invocation**, comfortably above the 20–45 ms the plan
  expected. Over a 100–200 call crawl that is roughly 6–13 seconds.

Instrument floors, so the harness can be shown not to be measuring itself:

| Floor | median |
|---|---|
| `/usr/bin/true` | 2.20 |
| trivial bash script | 7.51 |

Both variants sit far above both floors, and the shell floor alone accounts for
under a tenth of the shell variant's median.

## What the baseline had to be, and why

⚠️ **The measurement could not be taken from the working copy.** The plan
sequences it "before `run.sh` is deleted", but the phase that gave the daemon
its identity handoff landed first, and that phase makes `run.sh` **inoperable**:
the daemon now reads its identity from an inherited descriptor before it
publishes anything, and `run.sh` does not supply one. Invoked from the current
tree it fails with `daemon-start-timeout`, the bootstrap log showing:

```text
MalformedIdentity [Error]: ACCELERATOR_PLAYWRIGHT_IDENTITY_FD is not set;
the daemon is started by the launcher, not directly
```

So the baseline was reconstructed rather than read off the working copy: the
whole `scripts/playwright/` tree plus `scripts/vcs-common.sh` were restored from
the last revision at which the daemon had no handoff requirement, into a
standalone plugin root at `/tmp/shellbase`, with its own repository root for
`find_repo_root` to discover. That tree is internally consistent — its daemon
writes no token and its client sends none — so it measures the shell path as it
actually stood.

The sequencing flaw is worth recording: **the measurement had to happen before
the daemon-contract phase, not before the deletion phase.** Any future plan that
changes a contract shared by two implementations should measure the outgoing one
before the contract moves, not before the file is removed.

## An incidental finding

The first attempt failed with `another-launcher-running`. The cause was a
`launcher.lock.d` directory dated **9 May** sitting in this repository's state
directory — the mkdir-fallback lock, left behind because the shell's `EXIT` trap
is dropped by `exec node …`. It had been silently blocking every `run.sh`
invocation in this checkout for three months.

That is precisely the "pre-existing `run.sh` quirk" `test-run.sh:178-182`
documents, and precisely why the port drops the mkdir backend and releases its
single `flock` at launcher exit on every path.

## Method

`meta/migrations/0196-warm-path-measurement.py` is not committed: it is a
throwaway harness whose inputs (an absolute plugin root, a reconstructed
baseline tree, a bootstrapped Playwright namespace) cannot be reproduced from
the repository alone. The method is work-item:0186's, restated above in full so
the result can be re-derived rather than re-run.

Both variants measured the **warm reuse path** — a daemon already running,
which is what a crawl takes 100–200 times — with one warm-up invocation per
variant discarded before sampling.

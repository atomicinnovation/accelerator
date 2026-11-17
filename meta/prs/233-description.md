---
type: "pr-description"
id: "233"
title: "Replace humantime with jiff and suppress the chrono advisory"
date: "2026-11-17T12:26:11+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/233"
pr_number: 233
tags: []
revision: "7fac2491c65aa3b2eca03921506fbbebc289b7f4"
repository: "accelerator"
last_updated: "2026-11-17T16:05:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Replace humantime with jiff and suppress the chrono advisory

## Summary

Two advisories now mark `chrono` (RUSTSEC-2026-0543) and `humantime`
(RUSTSEC-2026-0542) as unmaintained, with no safe upgrade for either. They turn
cargo-deny's supply-chain check red on `main` and on every open PR, and that
check also gates prereleases. This PR removes `humantime`, which the visualiser
used for a single duration parse, in favour of `jiff`, the replacement the
advisory recommends. It also adds a dated ignore for `chrono`, which upstream
crates keep in the closure whatever this repository does.

## Changes

- **`visualiser.idle_timeout` is parsed by `jiff`.** The value is read as a
  `jiff::Span` and converted with a day counting as 24 hours and a week as 7
  such days:
  - the forms the tests already pin read as before (`8h`, `30m`, `1h30m`,
    `0s`, `1ns`, `never` and `0`), and `1d`, `2 days` and `1w` work;
  - a month or year, whose length depends on a date, is refused at boot
    (`InvalidIdleTimeout`, now carrying a `jiff::Error`);
  - a negative duration is refused (`NegativeIdleTimeout`), since a `Span`
    can hold one and `humantime` never produced one.
- **The idle-limit clamp is a plain conversion.** The longest `Span` (about
  20,000 years) is far below `MAX_IDLE_LIMIT_MS`, so the old saturation case
  can no longer occur. The clamp remains only as the safe `i128` → `i64`
  conversion with the 1 ms floor.
- **`humantime` leaves the closure.** `jiff` was already in it through
  `gix-date`. The visualiser takes it with default features off, and the
  third-party notices drop `humantime`.
- **`chrono` is ignored until its dependents move.** RUSTSEC-2026-0543 sits in
  `cli/deny.toml` as a dated table entry (`review-by: 2027-02-17`), following
  the break-glass procedure in `tasks/README.md`, which covers the
  `unmaintained` class.
- **The notices check reads its generators' JSON whole.** `notices:check`
  used to take cargo-about's and license-checker's JSON from stdout through
  `invoke`. `invoke` reads a stream in 1000-byte chunks and decodes each one
  separately, in the locale's encoding, replacing anything it can't decode.
  Any multi-byte character in a licence text that fell across a chunk, such
  as the ICU4X em dashes or the Unicode copyright sign, became replacement
  characters. Under the ASCII locale some `ubuntu-26.04` runners have, every
  such character did. Whether the check passed therefore depended on byte
  offsets and on the runner. Removing `humantime` shifted the offsets, and
  `main` is exposed to the same failure. Each generator now writes its JSON
  to a file, which the task reads whole as UTF-8.
- **Docs**: the visualise and configure skills, the docs-site visualiser page,
  the `idle_timeout` rustdoc and a CHANGELOG *Changed* entry describe the
  accepted durations.

## Context

`chrono`'s maintainers announced the end of maintenance in
[chronotope/chrono#1768](https://github.com/chronotope/chrono/issues/1768). The
advisory names `jiff` as the alternative, and `humantime` is flagged because it
wraps `chrono`.

These crates depend on `chrono` directly:

| Dependent | Ours? |
|---|---|
| `accelerator-visualiser` | yes |
| `file-rotate` (visualiser log rotation) | no |
| `jj-lib` | no |
| `interim` (through `jj-lib`, which enables its `chrono` feature) | no |
| `octocrab` | no |

Removing our own uses would therefore not clear the advisory.

## Testing

- [x] The full `mise run` passes, `deny:check` included. The one exception was
      the CLI unit task's coverage floor, tripped by stale coverage data left
      from switching branches. After `cargo llvm-cov clean --workspace`, that
      task's 7417 tests pass with lines at 94.19%, functions at 93.57% and
      regions at 92.88%.
- [x] `cargo deny check advisories` passes, and so does the deny integration
      suite, which checks the ignore's `review-by` date.
- [x] New idle-timeout tests:
      - days and weeks resolve as 24-hour days;
      - the longest week count `jiff` accepts resolves exactly;
      - months and years are refused, which `humantime` accepted;
      - `-1h` and `1h ago` are refused.

      The existing idle-timeout tests pass unchanged, apart from the one that
      pinned saturating `100000000000years`, which `jiff` refuses.
- [x] The fix is pinned with real `invoke` runs under an ASCII locale. A
      stand-in generator emits JSON with an em dash straddling byte 1000, and
      both runners must return it intact; before the fix they returned
      `'���'`, as CI rendered. A temporary diagnostic run of the check on CI
      showed that diff. Linux renders of both halves (Rust 1.90 and Node 22
      in containers) match the committed notices, and `notices:check` passes
      locally with the generators writing files.
- [x] Before the switch, a throwaway probe built against `jiff` 0.2.35
      checked how `Span` and `SignedDuration` parse each pinned input and the
      `humantime` forms in use. That probe is how the `Span` reading was
      chosen: `SignedDuration` refuses days and weeks.

## Notes for Reviewers

- **Behaviour change**: a month or year `idle_timeout`, which `humantime` read
  as 30.44 or 365.25 days, now stops the visualiser at boot with an error
  naming the value. The CHANGELOG says so.
- **Commit order**: the `chrono` ignore lands first, so the intermediate commit
  still fails `deny:check` on `humantime`. The tip passes.
- **Not done here**: moving the visualiser's own timestamps from `chrono` to
  `jiff::Timestamp`. That doesn't clear the advisory while `jj-lib` and
  `octocrab` hold `chrono`, so it is better left as follow-up work.
- `cli/licence-audit/new-trees.txt` still lists `humantime`. It is a dated
  August evidence snapshot that no task checks, so it is left as it was.

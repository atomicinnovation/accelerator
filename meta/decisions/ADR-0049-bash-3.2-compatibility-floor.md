---
id: "ADR-0049"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: accepted
tags: [architecture, toolchain, shell, bash, portability, foundations]
type: adr
title: "ADR-0049: Bash 3.2 Compatibility Floor"
schema_version: 1
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0048", "adr:ADR-0046", "adr:ADR-0047", "adr:ADR-0045"]
---

# ADR-0049: Bash 3.2 Compatibility Floor

**Date**: 2026-06-27
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0048 confines shell to thin wrappers and hook shims that resolve and delegate
to the CLI (the target state; today the shell library is large). Unlike the dev
toolchains, this shell runs in the **host environment** — the user's machine and
the Claude Code host — not under `mise`. `mise` provisions dev tooling; it does
not choose the shell the plugin's entry points run under at a user's site. The
plugin gets whatever shell the host provides.

On macOS that shell is **bash 3.2.57** — the last GPLv2 bash, frozen in 2007 and
still shipped as `/bin/bash` (and `/bin/sh`) on every Mac roughly two decades
later. A large share of users and contributors are on macOS. bash-4+ constructs
(associative arrays, `${var,,}`/`${var^^}` case modification, `mapfile`/
`readarray`, namerefs, `&>>`) therefore fail or silently misbehave on a stock
Mac, and requiring users to install a newer bash before the plugin's shell entry
points work would contradict the zero-setup distribution ethos of ADR-0046.

No off-the-shelf static tool verifies a *version* floor. ShellCheck distinguishes
shell **dialect** (`sh`/`bash`/`dash`/`ksh`/`busybox`) but not bash **version** —
declared as `bash`, it accepts bash-4 features as valid; its version-targeting
request (issue #2850) has been open since 2023 with no implementation. shfmt,
bashate, and checkbashisms do not check bash-version conformance either. A floor
can therefore be enforced only by a hand-rolled denylist or by executing scripts
under a real bash 3.2.

## Decision Drivers

- Shell entry points must run on a stock macOS (bash 3.2.57) with zero setup,
  consistent with ADR-0046 — no newer shell as a prerequisite.
- The host shell is not ours to provision; we must target the oldest shell a
  supported host ships rather than the one we would prefer.
- Enforcement should be faithful and low-maintenance — not resting solely on a
  brittle, hand-maintained checker.
- Shell is bounded toward thin wrappers (ADR-0048), so the cost of forgoing newer
  bash conveniences is contained.

## Considered Options

1. **Bash 3.2 floor** — write to bash 3.2.57 and ban bash-4+ constructs. Enforce
   by running the shell test suite under a real bash 3.2 as the authoritative
   gate, with a static denylist as a backstop.
2. **Target modern bash (4/5)** — require users and contributors to install a
   newer bash (e.g. via Homebrew) and document it as a prerequisite.
3. **Strict POSIX `sh`** — write to the POSIX shell command language only,
   runnable by any `/bin/sh` (dash/busybox included), enforced by stock
   ShellCheck in `sh` mode.
4. **No floor** — rely on convention and review.

## Decision

We will hold a **bash 3.2 compatibility floor** for all shell code: scripts and
wrappers target bash 3.2.57, and bash-4+ constructs are disallowed.

We chose option 1 over the alternatives:

- **Modern bash (option 2)** contradicts ADR-0046 head-on. The host shell is not
  ours to provision, so targeting bash 4/5 would force macOS users to install and
  path-resolve a newer bash before the plugin's shell entry points worked — a
  prerequisite the zero-setup model exists to avoid.
- **Strict POSIX (option 3)** is genuinely attractive: it is maximally portable
  and enforceable by stock ShellCheck, which would retire the bespoke checker
  entirely. We rejected it because its portability serves a target we do not
  have — every supported host (macOS, Linux, WSL) ships bash — while its cost
  lands on the very constructs that make shell safer: POSIX has no arrays, no
  `local`, no `[[ ]]`, no `pipefail`, and no process substitution. Our own
  `lint-bashisms.sh` already relies on arrays, process substitution, and
  `pipefail`; a POSIX rewrite would reintroduce the piped-`while` subshell
  footgun and global-only function variables. Bash 3.2 keeps these safety tools
  and costs only bash-4 niceties we do not need.
- **No floor (option 4)** would let shell break silently on the most common
  contributor platform.

Because no static tool targets a bash version, the faithful check is execution
under bash 3.2.57 itself — available for free on macOS CI runners, which ship
exactly that interpreter. The shell suites run on macOS CI under bash 3.2.57 — the
exact target — so they are exercised against a real floor interpreter. A static
denylist (`lint-bashisms.sh`), alongside the shfmt + ShellCheck pipeline that
already lints all shell, guards branches the suites do not exercise; with the
suites running against a real floor interpreter the denylist need not be
exhaustive.

## Consequences

### Positive

- Shell entry points run on a stock Mac with zero setup, consistent with
  ADR-0046 — no "install a newer bash" prerequisite.
- Targeting the oldest supported bash maximises host portability by construction.
- Retains the safety constructs (arrays, `local`, `[[ ]]`, `pipefail`, process
  substitution) that strict POSIX would forfeit.
- Running the suites on macOS CI under bash 3.2.57 exercises them against the real
  floor interpreter rather than heuristics, letting the static denylist serve as a
  non-exhaustive backstop.

### Negative

- Forgoes ergonomic bash-4 features (associative arrays, case-modification
  expansions, `mapfile`/`readarray`, namerefs); some logic is more verbose. This
  is felt in our current bash config parser (ADR-0047's predecessor), where the
  floor forces design compromises — though the config reader is moving to a
  CLI-native parser (ADR-0047) that is no longer bound by it.
- Behavioural conformance testing is coverage-bound: a banned construct in an
  unexercised branch is caught only by the static backstop, which is incomplete
  by nature.
- No off-the-shelf tool enforces the floor, so we own the enforcement machinery.
- Running a real bash 3.2 off macOS requires provisioning a source-built 3.2.57
  (CI or Docker) — an added cost where the gate must also run on Linux or locally.
- Shell has no autofixer; floor violations are fixed by hand.

### Neutral

- macOS CI runners ship bash 3.2.57 — the exact target — so the suites are
  exercised against the real floor interpreter essentially for free there.
- ShellCheck and shfmt versions are pinned via `mise` (ADR-0050); shfmt reads
  `.editorconfig` with no explicit dialect set.
- The floor governs a substantial existing shell library (~226 `.sh` files), not
  merely the linter — every one of them must stay within bash 3.2.57, and the
  direction (ADR-0048) is to shrink that surface toward thin wrappers as logic
  migrates into the CLI.
- ShellCheck's version-targeting request (#2850) is open upstream; were it to
  ship, it could supplement or replace the static backstop.

## References

- **Ported from luminosity** — original decision (lum ADR-0005, written when the
  shell surface was a single linter; Accelerator's shell library is large, and the
  floor is enforced by running the suites under bash 3.2 on macOS CI plus the
  shfmt/ShellCheck/`lint-bashisms.sh` pipeline):
  https://github.com/atomicinnovation/luminosity/blob/main/meta/decisions/ADR-0005-bash-3.2-compatibility-floor.md
- `meta/decisions/ADR-0048-four-toolchain-split.md` — Bounds shell toward the thin
  wrappers this floor governs.
- `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md` — The
  zero-prerequisite ethos driving the floor.
- `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md` — The
  config model whose former bash parser was constrained by this floor; it moves
  parsing into the CLI to escape it.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — Cites the floor
  as a constraint on shell robustness.
- `scripts/lint-bashisms.sh` — The static backstop (denylist).
- ShellCheck issue #2850 — open upstream request for bash-version targeting.
</content>

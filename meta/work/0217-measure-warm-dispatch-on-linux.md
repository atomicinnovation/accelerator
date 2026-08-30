---
type: "work-item"
id: "0217"
title: "Measure warm dispatch on linux"
date: "2026-08-17T20:36:49+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0136"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
relates_to: ["work-item:0189"]
tags: ["cli", "launcher", "performance", "measurement"]
last_updated: "2026-08-17T20:36:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-746"
---

# 0217: Measure warm dispatch on linux

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Run the committed warm-dispatch harness on linux and record the figures. The
darwin-arm64 result is in hand; whether it transfers is an **open question, not a
known**, and two effects push opposite ways.

## Context

Work item 0189's criterion is verified on darwin-arm64 only. Of the four shipped
platforms, **darwin-x64 and linux-arm64 are exercised by no CI lane at all** —
`.github/workflows/main.yml` matrixes `ubuntu-latest` and `macos-latest` alone.

Two effects run in opposite directions, which is why the transfer cannot be
assumed:

- 0186's breakdown makes `G`'s bootstrap term overwhelmingly **spawn cost**, and
  linux spawns are typically cheaper — which would lower `G`.
- linux ships coreutils `sha256sum` universally, so the **fast** digest backend
  is the norm rather than the exception — which lowers `G` again but lowers `B`
  proportionally less, so the *ratio* may rise.

0205 established that nothing in its findings transfers off darwin-arm64: the
sha256-versus-BLAKE2b inversion is a property of this chip and this crate build.

**The harness is committed and its constants are per-OS data**, so this item runs
a task rather than re-authoring a script. What it must add is a **calibrated
platform entry** for the linux key, including the calibration provenance fields
— chip, resolved `bash`, resolved `shasum` — which the darwin entry currently
leaves `None` because 0205 never recorded them. Until an entry exists the harness
records figures as uncalibrated context and refuses a gating verdict, which is
the correct behaviour and is what this item resolves.

## Requirements

- Prerequisites for **measuring**: a linux host with `jj` at the `mise.toml`
  pin, `git`, `jq`, `realpath`, `bash`, **`curl` or `wget`**
  (`bin/accelerator:145-159` hard-fails without one), **`awk`** (both digest
  pipelines use it), `chmod` and the rest of the mechanically derived tool set, a
  resolvable sha256 backend, **a published signed release for the tree's own
  version**, and network egress to the release base URL. No build: the shipped
  musl artefact is fetched and verified.
- ⚠️ A minimal image with `sha256sum` but **no Perl** cannot construct the
  fallback farm, in which case C3, C4 and C6 are recorded not applicable
  (branch 7) rather than measured.
- Prerequisites for **decomposing** the term set: `rustup target add <musl
  triple>` plus a musl-capable linker natively. `cargo-zigbuild` and `ziglang`
  are the cross-from-darwin mechanism, not a native requirement.
- Add a calibrated platform entry for the linux key with its full calibration
  provenance, and record the absolute ceilings the linux figures justify rather
  than importing darwin's.
- `reverify` ms-per-MB must be recorded against **(architecture, SHA-extension
  support, libc)** rather than the OS name.

## Acceptance Criteria

- [ ] `mise run measure:warm-dispatch` completes a valid session on linux, with
      the record committed under `meta/measurements/`.
- [ ] A calibrated platform entry exists for the linux key, with all four
      calibration provenance fields recorded, and the harness reports the session
      **calibrated** rather than as context.
- [ ] The instrument floors are recorded and their gate values justified from the
      linux figures, not inherited from darwin.
- [ ] Whether the darwin result transferred is stated explicitly, with the
      direction of any difference attributed to spawn cost or digest backend.
- [ ] If Perl is absent, C3/C4/C6 are recorded not applicable with that reason.

## Dependencies

- **Relates to** 0189, which committed the harness and measured darwin-arm64,
  and 0205, which established that its findings do not transfer.
- **Parent**: epic 0136.

## Assumptions

- A linux host meeting the prerequisites is available. If only a container is,
  the cgroup-quota CPU-count rung the harness already implements applies, and the
  load figure must be read as host-scoped rather than container-scoped.

## References

- `tasks/README.md#the-measure-namespace` — prerequisites and what a run requires
- `tasks/measure.py` — `PLATFORM_TABLE`, keyed on `(system, machine)`
- `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md` — the criterion
  and the darwin figures

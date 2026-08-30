---
type: "work-item"
id: "0203"
title: "Ship an MPL-2.0 Attribution Artefact with the Release Uploads"
date: "2026-08-10T18:40:00+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "ready"
kind: "task"
priority: "medium"
parent: "work-item:0136"
relates_to: ["work-item:0185", "work-item:0188", "work-item:0165"]
tags: ["rust", "licensing", "release", "vcs"]
last_updated: "2026-08-10T18:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-733"
---

# 0203: Ship an MPL-2.0 Attribution Artefact with the Release Uploads

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Five of the six dispatched sub-binaries link `uluru` (MPL-2.0), so MPL-2.0
§3.2's notice obligation is live for the signed binaries we distribute today.
The release upload set carries no artefact telling recipients how to obtain
that source. Build one, and stage it alongside the binaries.

## Context

`cli/deny.toml`'s `uluru` exception was recorded on the basis that dead-code
elimination removed the whole `gix`/`jj-lib` closure from every shipped
binary, so §3.2 did not bind. 0185 re-ran that check across all six
`DISPATCHED_SUBBINARIES` rather than the visualiser alone, and the premise
turned out to hold only for the visualiser.

Measured on unstripped `--release` builds for `aarch64-apple-darwin`, by
counting `gix_`/`jj_lib`/`uluru` symbols:

- `accelerator-vcs`, `accelerator-work`, `accelerator-collaboration` and
  `accelerator-migrate` already linked `uluru` **before** 0185's switch. Each
  constructs `InProcessProbe` directly, through call sites unrelated to the
  metadata-read path, so directly-called code cannot be eliminated. This is a
  pre-existing finding 0185 surfaced, not one it caused.
- `accelerator-corpus` linked none of the three before and all of them after.
  This one is caused by 0185 repointing `vcs_adapters::facts` onto the
  library-backed probe.
- `accelerator-visualiser` links none of the three, before or after, so its
  original exception rationale is intact.

`uluru` is `gix-pack`'s LRU pack cache, reached through `gix-odb`'s default
features, so it cannot be feature-gated out of the closure.

Note for whoever picks this up: the two string literals the original
verification procedure used (`extensions.objectFormat` for gix, `There is no
Jujutsu repo` for jj-lib) are unreliable as an absence test — both are missing
from binaries that demonstrably link the closure. Count symbols with `nm -a`
instead. Plain `grep` over a Mach-O binary also reports false positives here;
`strings -a | grep` or `nm -a | grep` are the sound forms.

## Requirements

- Produce a third-party attribution artefact covering the MPL-2.0 components
  linked into the distributed binaries, carrying the notice and the means of
  obtaining the corresponding source.
- Stage it into the release upload set (`_release_uploads()` in
  `tasks/github.py`) so it ships with the signed manifest rather than
  existing only in the repository.
- Extend the release-workflow coverage assertion in `test_workflows.py` to
  cover the new upload, so a future change to the upload set cannot silently
  drop it.
- Decide whether the artefact is generated (e.g. from the cargo metadata
  graph) or hand-maintained, and record why. A generated artefact stays
  correct as the closure moves; a hand-maintained one goes stale silently.
- Update `cli/deny.toml`'s `uluru` exception comment to point at the shipped
  artefact once it exists, replacing the current statement that the release
  upload set carries none.

## Acceptance Criteria

- [ ] The attribution artefact exists and names every MPL-2.0 component in
      the distributed closure, verified against the actual linked binaries
      rather than against the dependency manifest alone.
- [ ] It is present in the release upload set and covered by
      `test_workflows.py`.
- [ ] The generated-versus-maintained decision is recorded on this item.
- [ ] `cli/deny.toml`'s comment reflects the shipped state.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- Relates to: work-item:0185 — surfaced the finding and made
  `accelerator-corpus` reach the closure.
- Relates to: work-item:0188 — delivered the library-backed adapter whose
  closure carries `uluru`.
- Relates to: work-item:0165 — owns the release manifest and upload set this
  artefact has to join.

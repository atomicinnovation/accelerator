---
type: work-item
id: "0165"
title: "Multi-Binary Static Distribution and Release Pipeline with minisign"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: high
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0164"]
tags: [rust, distribution, release, cross-compile, minisign]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-186"
---

# 0165: Multi-Binary Static Distribution and Release Pipeline with minisign

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Extend the existing visualiser release pipeline to build, sign, and publish the
`accelerator` launcher and every `accelerator-<sub>` binary as zero-setup, fully
static, per-platform GitHub Release assets, with minisign + sha256 integrity and
version coherence across the whole workspace (ADR-0046).

## Context

The repo already cross-compiles the visualiser for four targets via
`cargo-zigbuild`, writes `checksums.json`, enforces version coherence across
`plugin.json` / `Cargo.toml` / `checksums.json`, and uploads with
re-download-and-re-verify (`tasks/build.py`, `tasks/github.py`). This story
generalises that pipeline to **many** binaries and **adds minisign** — which does
not exist anywhere yet. ADR-0046 rejects cargo-dist in favour of the hand-rolled
invoke pipeline. This is the distribution half of luminosity 0008, materially
bigger here because of multi-binary coherence and minisign key management.

## Requirements

- Cross-compile every workspace binary (launcher + each sub-binary) for the four
  targets (`aarch64/x86_64-apple-darwin`, `aarch64/x86_64-unknown-linux-musl`) via
  `cargo-zigbuild`, retaining the Mach-O/ELF magic-byte sanity checks.
- Produce per-binary sha256 checksums and a release manifest that includes the
  `description` field the launcher's help/listing needs (0164).
- Add minisign signing: sign each artifact, embed the public key in the launcher,
  and produce detached `.minisig` files the launcher verifies in-process via
  `minisign-verify`. Establish the **minisign key lifecycle** (generation, secure
  storage, rotation) as an operational procedure.
- Extend version coherence to span `plugin.json` + **every crate `Cargo.toml`** +
  the release manifest; fail the build on any mismatch.
- Generalise the `gh`-based upload + re-download-and-re-verify flow to all binaries,
  preserving the draft on verification failure.
- Drop the documented-but-unimplemented runtime provenance hook and correct
  `RELEASING.md` (resolved Q6); keep CI-side SLSA attestations as out-of-band
  provenance.

## Acceptance Criteria

- [ ] Static binaries build for all four targets for the launcher and each
      sub-binary, each verifiably static on the musl targets, published as GitHub
      Release assets with a sha256 and a `.minisig`.
- [ ] The release manifest lists every binary with its checksum and `description`,
      and the launcher verifies sha256 + minisign against it.
- [ ] Version coherence fails the build when `plugin.json`, any crate `Cargo.toml`,
      or the manifest disagree.
- [ ] `RELEASING.md` no longer advertises a runtime `gh attestation verify` hook;
      CI still emits SLSA attestations.
- [ ] The minisign key lifecycle (generate/store/rotate) is documented and the
      signing step runs in the release flow.

## Open Questions

- Minisign key storage location and rotation policy (CI secret vs offline key) —
  decided during implementation.

## Dependencies

- Blocked by: 0163 (workspace exists to build from).
- Relates to: 0164 (the launcher consumes the manifest, checksums, and minisign
  public key this story produces).
- Parent: epic 0136.

## Assumptions

- The existing zigbuild/invoke/`gh` pipeline ports to multi-binary largely
  verbatim; cargo-dist remains rejected (ADR-0046).

## Technical Notes

- `tasks/shared/paths.py` / `targets.py` / `hashing.py` and `tasks/build.py`,
  `tasks/github.py`, `tasks/version.py` are the extension points.
- zig + cargo-zigbuild are PyPI deps in `pyproject.toml`, not mise; rust targets
  added via `rustup target add`.

## Drafting Notes

- Split from luminosity 0008 as the distribution/release half; differs from lum
  0008's slice text by mandating minisign and reusing the invoke pipeline (lum
  0008 is stale relative to its own ADR-0010 on minisign).

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0046, ADR-0054
- Spike: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md` (§4 cross-compile & distribution)
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0008-on-demand-static-binary-distribution-and-launcher.md

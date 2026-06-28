---
type: work-item
id: "0164"
title: "Launcher and Git-Style Dispatch"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: high
parent: "work-item:0136"
blocks: ["work-item:0168"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0165"]
tags: [rust, launcher, dispatch, cli]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-185"
---

# 0164: Launcher and Git-Style Dispatch

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Implement the `accelerator` launcher's git-style dispatch and the on-demand
fetch→verify→cache→exec pipeline in Rust, fronted by a thin bash bootstrap that
fetches the launcher itself on first use — so the CLI grows by on-demand
sub-binaries behind a single command (ADR-0054).

## Context

ADR-0054 settles dispatch as clap `external_subcommand` with Unix `exec`, `version`
and `config` built-in, and a uv-style resolve-once-and-cache launcher whose
fetch/verify/cache/exec logic lives in Rust rather than bash. This is the launcher
half of luminosity 0008, which Accelerator implements as its own story (the
existing `launch-server.sh` is a daemon launcher and does not model git-style
sub-binary dispatch). The distribution/release pipeline and signing are 0165.

## Requirements

- Implement clap derive `#[command(external_subcommand)] External(Vec<OsString>)`
  on the launcher: first element = subcommand name, rest forwarded verbatim;
  `Vec<OsString>` preserves non-UTF-8 args.
- Dispatch via Unix `exec` (`CommandExt::exec`, process-replacing) so exit codes and
  signals propagate. Unix-only (the four targets are all Unix).
- Implement the fetch→verify→cache→exec pipeline in Rust: scan the managed cache
  first (keyed by name+version+checksum), fetch-on-miss over `reqwest` + rustls
  (`default-features = false`), verify sha256 on fetch and re-verify before every
  exec, plus minisign verification (`minisign-verify`, embedded pubkey — see 0165
  for key lifecycle). The cache lives **under `${CLAUDE_PLUGIN_ROOT}`** (resolved
  Q3) so `allowed-tools` matches hold; a new plugin version redownloads.
- Implement the thin bash bootstrap (bash-3.2-safe) that fetches the `accelerator`
  binary itself on first use and then execs it; thereafter the Rust launcher owns
  everything. No launcher self-update.
- Synthesise discoverable help: clap built-in help plus an external-subcommands
  section built from the release manifest's `description` field; delegate
  per-command `--help` by re-exec'ing the child with `--help`.

## Acceptance Criteria

- [ ] Given a sub-binary name, when the launcher needs an absent sub-binary, it
      fetches the asset for the host target, verifies sha256 + minisign, caches it
      under `${CLAUDE_PLUGIN_ROOT}`, and execs it.
- [ ] Given a cached, verified sub-binary, when invoked again, the launcher reuses
      the cache without re-fetching.
- [ ] Given a sha256 or minisign verification failure, the launcher refuses to exec
      and reports the failure.
- [ ] `accelerator <unknown-subcommand>` and `accelerator --help` render the
      manifest-driven external-subcommands listing; `accelerator <sub> --help`
      delegates to the child.
- [ ] The launcher is statically linkable (rustls, no OpenSSL); the bash bootstrap
      is bash-3.2-safe.

## Open Questions

- Whether `config` (and any hook handling) is fully built-in vs partially external
  is settled alongside 0167/0169; this story wires the dispatch mechanism and the
  built-in/external split point.

## Dependencies

- Blocked by: 0163 (scaffold provides the launcher crate skeleton).
- Blocks: 0168 (the visualiser is the first dispatched sub-binary).
- Relates to: 0165 (distribution/signing supplies the manifest, checksums, and
  minisign keys the launcher verifies against).
- Parent: epic 0136.

## Assumptions

- clap derive enables external dispatch without a manual builder call on the pinned
  clap version (to be confirmed at scaffold; low risk per ADR-0054).

## Technical Notes

- `reqwest` + rustls is workspace-wide (blocking in the launcher, async in
  subdomains), accepted as pulling `tokio` into the launcher (ADR-0054).
- Preserve the env-override escape hatch (`ACCELERATOR_*_BIN`-style) for
  air-gapped/offline first use.

## Drafting Notes

- Split from luminosity 0008 as the launcher/dispatch half; the
  cross-compile/release/signing half is 0165.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0046, ADR-0054
- Spike: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md` (§2 dispatch, §3 launcher)
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0008-on-demand-static-binary-distribution-and-launcher.md

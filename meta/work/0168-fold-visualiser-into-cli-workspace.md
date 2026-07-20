---
type: work-item
id: "0168"
title: "Fold the Visualiser into the cli/ Workspace"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0165", "work-item:0166"]
tags: [rust, visualiser, frontend, workspace]
last_updated: "2026-07-19T23:12:20+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-189"
---

# 0168: Fold the Visualiser into the cli/ Workspace

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Relocate the existing `accelerator-visualiser` server and its frontend into the
`cli/` workspace, refactor it onto the shared `config`/`corpus` crates, and move its
start/stop/status orchestration into `accelerator visualiser …` so it becomes the
first concrete on-demand sub-binary dispatched by the unified launcher (ADR-0054).

## Context

The visualiser is already the crate `accelerator-visualiser` with a `[lib]`/`[[bin]]`
split and ~15.4k lines of Rust, much of it corpus logic that duplicates the bash
library. That duplicated corpus logic now lives in the shared
`corpus`/`corpus-adapters`/`document` crates delivered by 0179 (resting on 0178 for
`config`/`config-adapters`); this story retires the visualiser's copies onto them.
The `server`+`frontend` pair moves as a unit to `cli/visualiser/{server,frontend}`
(whether to move them together rather than separately was a question resolved
during extraction — moving as a unit is what preserves the embed path), preserving
their relative layout so the three `../frontend/dist` embed literals are unchanged.
The bespoke `launch-server.sh` daemon launcher is retired in favour of the unified
launcher (0164).

Two constituencies benefit: **maintainers**, who stop carrying the slice of the
crate's ~15.4k lines that duplicates the bash library's corpus logic (the crate
itself is retained — axum/tokio/notify stay — only the duplicated corpus portion
collapses onto the shared crates); and the **unified-launcher release flow**, which
gains the visualiser as its first concrete on-demand sub-binary and so validates the
dispatch path end to end.

## Requirements

- Move `skills/visualisation/visualise/{server,frontend}` to
  `cli/visualiser/{server,frontend}` as a unit; add `visualiser/server` as a
  workspace member; keep `skills/visualisation/visualise/` holding only the skill.
- Refactor the server to consume the shared `corpus`/`config` crates instead of its
  own duplicated frontmatter/doc-type/slug/work-item-ID/typed-linkage code; keep
  axum/tokio/notify isolated in this crate.
- Move start/stop/status orchestration (`visualiser.sh`, `launch-server.sh`,
  `stop-server.sh`, `status-server.sh`, `write-visualiser-config.sh`) into
  `accelerator visualiser start|stop|status`, dispatched by the launcher; preserve
  the owner-PID/`start_time`/idle-shutdown lifecycle and the loopback-binding +
  Host/Origin security model.
- Retire `launch-server.sh` and the visualiser's separate `bin/checksums.json` in
  favour of the unified launcher + release manifest (0164/0165). Note `launch-server.sh`
  plays two roles, retired by two different mechanisms: its daemon start/stop
  orchestration re-homes into `accelerator visualiser start` (previous bullet), while
  its bespoke fetch/distribution role is what the unified launcher replaces. This is
  the distribution cut-over: the file removals land at story completion, but the
  launcher's fetch/verify path lands with or after 0165's manifest entry (see the
  ordering constraint in Dependencies).
- Update the visualiser build wiring (the `../frontend/dist` literals stay valid
  given the unit move; `build.rs` still requires the prebuilt dist under
  `embed-dist`).

## Acceptance Criteria

- [ ] Given a repo carrying a valid visualiser configuration (the `.accelerator/*.md`
      keys the start path requires — the same fixture config the orchestration tests
      below use), when I run `accelerator visualiser start`, then the server binds a
      loopback port and the command prints its `http://127.0.0.1:PORT` URL.
- [ ] `accelerator visualiser stop` terminates only a process whose recorded
      owner-PID **and** `start_time` still match (recycle guard), and leaves a
      recycled/unrelated PID untouched.
- [ ] `accelerator visualiser status` reports the actual lifecycle state across
      transitions, using a `running`/`stopped` token on stdout as the definitive
      signal: before any `start` (never-started state) it prints `stopped`; after
      `start` it prints `running` (and the URL); after `stop` it prints `stopped`.
- [ ] An idle server (no inbound HTTP request within the window) self-shuts-down on
      the `visualiser.idle_timeout` the server resolves — same key and default as
      today (`8h`, `never`/0 to disable). Verified by configuring a short timeout
      (e.g. a few seconds) and asserting shutdown, plus a `never`/0 case asserting the
      server stays up, rather than by waiting out the default. The `8h` default is
      checked as a config-resolution assertion — with no `visualiser.idle_timeout` set,
      the server's resolved value equals `8h` — not by waiting.
- [ ] The loopback + Host/Origin security model is preserved: a request with a
      non-loopback `Host` header is rejected `403 Forbidden`, a state-changing
      (`PATCH`/`POST`/`PUT`/`DELETE`) request carrying a cross-origin `Origin` is
      rejected `403 Forbidden`, and a matching loopback `Host` with a loopback or
      absent `Origin` is accepted — the same `403` statuses as today. A Host-only
      violation and a (state-changing) Origin-only violation are each independently
      rejected `403`, so the host-header and origin guards are exercised separately.
- [ ] The visualiser crate no longer contains its duplicated `docs.rs`
      (`DocTypeKey`), `slug.rs`, `frontmatter.rs`, `patcher.rs`, `typed_ref.rs`, or
      the `WorkItemIdScheme`/ID logic in `config.rs`; each is replaced by the
      `corpus`/`corpus-adapters`/`document` equivalent, and `gray_matter`/`serde_yml`
      are dropped from the server's dependencies.
- [ ] Behaviour parity across the engine swap, verified by golden/parity tests (not
      merely by the old modules being absent). Fixture: one document per `DocTypeKey`
      variant (14) plus known edge cases — multi-line/quoted frontmatter values, the
      fence-offset boundaries `document::fence_offsets` handles (leading blank lines,
      CRLF line endings, no trailing newline, an empty frontmatter block, no frontmatter
      at all), slug inputs exercising `humanise_slug`/`title_case_segment`, and
      work-item-ID inputs. Parity
      relation: for each fixture the refactored `document`/`corpus` parsed frontmatter
      map, derived slug, and inferred doc-type are field-for-field equal to the
      pre-refactor `gray_matter`/`serde_yml` output; differences in error-message text
      and insignificant whitespace are out of scope. The pre-refactor outputs are
      captured as committed golden fixtures **before** the old modules are removed, so
      the parity comparison stays reproducible after the deletion.
- [ ] The crate is a `cli/` workspace member at `cli/visualiser/server`, inherits
      `version`/`edition`/MSRV (minimum supported Rust version) from
      `[workspace.package]` (no hand-copied version literal), and
      `cargo build -p accelerator-visualiser` succeeds from the workspace.
- [ ] The release build embeds the frontend from `cli/visualiser/frontend` with the
      three `../frontend/dist` literals unchanged; `build.rs` still fails cleanly if
      the prebuilt dist is absent under `embed-dist`.
- [ ] The five orchestration scripts — `visualiser.sh`, `launch-server.sh`,
      `stop-server.sh`, `status-server.sh`, and `write-visualiser-config.sh` — and the
      visualiser's standalone `bin/checksums.json` are removed once the
      `accelerator visualiser` subcommands are in place — verifiable at story
      completion.
- [ ] The visualiser is fetched/verified/dispatched by the unified launcher against
      the release manifest. Verifiable within this story against a local/test manifest
      fixture — the launcher's fetch/verify path resolves the visualiser entry and
      dispatches the binary; the equivalent assertion against the live release manifest
      is gated on 0165's manifest carrying a visualiser entry (see the
      distribution-cut-over ordering constraint in Dependencies) and lands when that
      entry does.
- [ ] New tests cover the orchestration surface, not merely the pre-existing suites:
      a test asserts each of `accelerator visualiser start|stop|status` dispatches
      through the launcher — evidenced by an observable, e.g. a launcher test
      double/spy that records the dispatch, or a distinguishing side effect the
      launcher path produces that a direct in-process call does not — and a test
      asserts the server honours configuration from
      the location it is directed to — verified by setting `visualiser.idle_timeout` to
      a non-default value in a fixture config and asserting the server's resolved
      timeout matches it, independent of whether the launcher passes config in or the
      server reads `.accelerator/*.md` directly (that mechanism is Open Question 3).
      With those in place, the visualiser
      E2E/integration suites pass against the relocated, refactored crate and
      `mise run` is green end-to-end (the aggregate gate).

## Open Questions

1. Whether the frontend's own toolchain checks (Biome/vitest/Playwright) move under
   a `cli/visualiser/frontend` task path or stay as-is — decided during
   implementation.
2. Does the server keep an async façade over the sync `corpus-adapters` store (via
   `spawn_blocking`), or do the store primitives gain an async variant?
3. Once `launch-server.sh` is retired, does the server read `.accelerator/*.md`
   directly via `config`/`config-adapters`, or does the launcher resolve config and
   pass it in (replacing the emitted `config.json`)?

## Dependencies

- Blocked by: 0179 (delivers the `corpus`/`corpus-adapters`/`document` crates the
  server refactors onto, resting transitively on 0178 for `config`/`config-adapters`),
  0164 (launcher dispatch). All three — 0179, its transitive dependency 0178, and
  0164 — are now `done`, so the crate-refactor and dispatch prerequisites are
  satisfied.
- Not blocked by: 0180 (atomic-store primitives) — the fold-in reuses
  `corpus-adapters`' existing `FileCorpusStore` write path from 0179; 0180's extra
  JSONL/lock primitives are not on this story's path.
- Relates to: 0165 (the visualiser joins the multi-binary release), 0166 (this
  refactor validates the shared-crate approach 0166 described; 0166 itself built
  nothing — see Drafting Notes).
- Ordering constraint on the distribution cut-over (the criterion retiring
  `launch-server.sh` + the standalone `bin/checksums.json` in favour of the unified
  launcher's fetch/verify): that cut-over is only safe once 0165's release manifest
  actually carries a visualiser entry — otherwise the launcher's fetch/verify path is
  unsatisfiable. 0165 is not a hard blocker for the whole story (relocation, refactor,
  and orchestration can land first), but its manifest entry must land before or with
  the cut-over (mirrors the 0180 "Not blocked by" rationale above).
- External system: the retired bespoke distribution is replaced by the unified
  launcher fetching the pre-compiled visualiser binary from GitHub Releases, so the
  cut-over criterion's fetch/verify path is coupled to that hosting being reachable —
  the same external dependency the launcher already carries (0164/0165), recorded
  here for visibility.
- Parent: epic 0136.

## Assumptions

- Moving the `server`+`frontend` pair as a unit preserves the relative embed path,
  so no literal surgery is needed (the unit-vs-separate move was resolved in favour
  of the unit move during extraction).

## Technical Notes

Duplicated server modules retire onto shared items:

- `docs.rs` (`DocTypeKey`, 14 variants) → `corpus::doc_type::DocTypeKey`.
- `slug.rs` (`derive`, `humanise_slug`, `title_case_segment`) → `corpus::slug`;
  `title_case_segment` is documented there as the retire target.
- `frontmatter.rs` (`fence_offsets`, `parse`) → `document::fence_offsets` +
  `corpus_adapters::document::parse`.
- `patcher.rs` (`patch_status`) → `corpus_adapters::patcher::patch_status`.
- `typed_ref.rs` (`parse_typed_ref`) → `corpus::typed_ref`.
- `config.rs` (`WorkItemConfig` ID logic) → `corpus::WorkItemIdScheme` + injected
  `IdScanner`.
- `file_driver.rs` (`atomic_write_preserving_perms`) →
  `corpus_adapters::FileCorpusStore` (`AtomicWrite`).
- linkage/cluster logic (`related.rs`, `clusters.rs`, `lifecycle.rs`) →
  `corpus::linkage` (`parse_document`, `TYPE_PAIRS`, `Band`).

Reconciliation tensions to resolve during implementation:

- Async vs sync: the server's `FileDriver`/atomic-write and the axum/tokio stack are
  async; `corpus`/`corpus-adapters` are sync. The domain/parsing code moves out; the
  async I/O boundary (spawn_blocking or a thin async façade) stays in this crate.
- Frontmatter engine swap: `gray_matter` + `serde_yml` → the `document` crate
  (serde-saphyr, confined to `document`). Server wire types derive serde (kebab-case
  for the API); `corpus::DocTypeKey` is serde-free, so the wire mapping must be
  re-homed in a thin server view type over `wire_str`/`from_wire_str`.
- No `infer` matcher today: the server keys doc type off which root a path sits under
  (`LocalFileDriver::kind_for_canonical_path`), whereas `corpus` exposes a pure
  `doc_type::infer(path, table)` longest-segment matcher — pick one during the
  refactor.
- Version inheritance: `server/Cargo.toml` hand-copies the version literal; as a
  workspace member it should use `version.workspace = true`.

Retained / unchanged:

- Existing seam: `file_driver.rs` already defines an (async) `FileDriver` port trait.
- The server has no production TLS (loopback + Host/Origin guards); the launcher's
  rustls is the only production TLS.

## Drafting Notes

- Treated as the Phase 5 story; it is both a fold-in and the validation that the
  shared crates delivered by 0179 actually absorb the visualiser's duplicated logic.
- Kept as a single story rather than split into a relocate-plus-dispatch story and a
  refactor story, despite the two streams being notionally separable. The reason is
  that the relocation is the point at which the visualiser becomes a workspace member
  and can `use corpus`/`use document` at all: relocating without refactoring would
  leave the duplicated modules compiling in their new home only to be deleted days
  later, and refactoring without relocating is impossible while the crate lives
  outside the workspace. Delivering them as one increment avoids a throwaway
  intermediate state and lets the fold-in double as the "first on-demand sub-binary"
  proof for the launcher (0164) in a single validated step. The per-stream acceptance
  criteria remain individually checkable, so partial progress is still visible.
- Reconciled the stale "0166" shared-crates citation: 0166 is the draft umbrella
  story that built nothing, so the precise blocker is 0179 (transitively 0178). The
  canonical `blocks: 0168` edges already live on 0179 and 0164, so they were not
  duplicated into this item's `blocked_by`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0053, ADR-0054

---
type: "work-item"
id: "0201"
title: "In-Process Section Diff"
date: "2026-08-08T12:27:18+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "done"
kind: "task"
priority: "medium"
relates_to: ["work-item:0170", "work-item:0174", "work-item:0188", "work-item:0198"]
tags: ["rust", "work-items", "tech-debt"]
last_updated: "2026-08-08T12:27:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-731"
---

# 0201: In-Process Section Diff

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Replace `cli/work-adapters/src/diff_shellout.rs`'s subprocess-based section
diff — which writes two temp files and shells out to the real `diff -u`
binary — with an in-process Rust implementation. This removes the crate's
one external process dependency and its associated spawn/timeout failure mode,
and lets the architecture-enforcement rule that exists solely to quarantine it
be removed or simplified.

## Context

`accelerator work diff` (the Rust port of `work-item-section-diff.sh`,
delivered by 0170) renders per-section differences by shelling out to the
system `diff` binary against two temp files, tolerating its exit-1-means-
differ convention and capping it at a 10s timeout with a 10ms poll loop.
This was a deliberate choice at the time — the same "shell the real tool"
adapter pattern already used for `vcs status`/`vcs log` — and it guarantees
byte-identical output against the legacy bash script during the port,
verified by a `bash-parity` cargo feature that runs the real binary.

That parity guarantee is no longer needed: the port is long complete (0170
is done), and exact byte-for-byte matching against GNU diffutils' specific
hunk-splitting heuristics is not a requirement callers depend on — only the
existing `=== name (- LOCAL / + REMOTE) ===` header/framing contract is.
Removing the subprocess call eliminates a portability assumption (`diff` on
PATH), the spawn/timeout failure mode (`DiffUnavailable`), and lets
`work-adapters` become fully in-process, which in turn should let
`cli/pup.ron`'s `work_adapters_filesystem_reads_in_process` rule — carved
out solely to isolate this one subprocess-spawning module — be removed or
simplified.

Research surfaced `similar` (Armin Ronacher / insta author) as the strongest
default in-process crate for this: dependency-free, ships a one-call
`TextDiff::unified_diff()` that builds hunk headers and `-`/`+` prefixes
directly, Myers/Patience algorithms, ~14.8M downloads/month, actively
maintained, Apache-2.0. `imara-diff` (lower-level, build-your-own-printer)
and `diffy` (patch-parsing focused) are credible alternatives if a
different tradeoff is wanted at implementation time.

## Requirements

- Before changing the diff body's formatting, scan for consumers of
  `accelerator work diff` output beyond `cli/work/src/section_diff.rs` that
  parse or grep the body, and confirm none depend on the body's internal
  formatting. If any do, widen scope to accommodate them before proceeding.
- Replace the subprocess `diff -u` invocation in `diff_shellout.rs` with an
  in-process Rust diff implementation — no `std::process::Command`, no temp
  files.
- Preserve the existing `=== name (- LOCAL / + REMOTE) ===` header and
  blank-line section framing exactly; the diff body's internal hunk
  formatting need not match GNU diffutils' output byte-for-byte.
- Resolve the `DiffUnavailable` failure mode: since an in-process function
  cannot fail to spawn or time out, determine whether `render`'s signature
  becomes infallible or keeps a narrower error type for a different failure
  mode (if any is identified during implementation).
- Retire or rewrite the `bash-parity` cargo feature and
  `cli/work-adapters/tests/diff_shellout_parity.rs`, which currently assert
  byte-identical output against the real `diff` binary — replace with tests
  asserting the new implementation's own output contract against fixed
  expected text.
- Remove the now-subprocess-specific unit tests (spawn failure, timeout/
  `DEFAULT_CAP` handling) and replace with tests appropriate to an
  in-process function.
- Remove or simplify `cli/pup.ron`'s `work_adapters_filesystem_reads_in_process`
  rule to reflect that `work-adapters` no longer spawns any subprocess, once
  confirmed no other subprocess call remains in the crate.

## Acceptance Criteria

- [ ] Given two differing text sections, when the section-diff renderer
      runs, then it returns a unified-diff-style body computed entirely
      in-process, with no `std::process::Command` invocation anywhere in
      `work-adapters`.
- [ ] Given the existing `=== name (- LOCAL / + REMOTE) ===` header and
      blank-line framing that `accelerator work diff` callers depend on,
      when the in-process section-diff renderer replaces the subprocess one,
      then that framing is unchanged.
- [ ] Given a fixed two-section fixture in which one section differs, when the
      section-diff renderer runs, then its full rendered output matches an
      exact expected-text golden held in the test suite, and the diff body
      carries per-line `-`/`+` prefixes and at least one `@@`-style hunk
      header.
- [ ] Given the subprocess call is gone, when the migration completes, then
      `render`'s signature no longer surfaces a `DiffUnavailable` variant —
      either it is infallible, or its only remaining error variant reflects a
      distinct failure mode identified during implementation.
- [ ] Given two identical sections, when the section-diff renderer runs, then
      it reports no differences (an empty diff body), matching current
      behaviour.
- [ ] Given the `bash-parity` feature and `diff_shellout_parity.rs` suite,
      when this change lands, then that suite is retired or rewritten to
      assert the new implementation's own output contract, and
      `cargo test -p work-adapters` no longer requires a `diff` binary on
      `PATH`.
- [ ] Given `cli/pup.ron`'s `work_adapters_filesystem_reads_in_process`
      rule, when the migration completes and no subprocess call remains in
      `work-adapters`, then the rule is removed or updated accordingly, and
      `mise run pup:check` passes.
- [ ] Given the full local CI mirror (`mise run`), when it runs after this
      change, then it passes with no `diff` binary required anywhere in the
      `work`/`work-adapters`/`work-cli` crates' build or test process.

## Open Questions

- Should the module (and file) keep the name `diff_shellout` once it no
  longer shells out, or be renamed (e.g. `diff_render`)? Left for
  implementation-time judgment rather than specified here.
- Is `diff_shellout.rs` genuinely the only subprocess-spawning code in
  `work-adapters`, such that the pup.ron isolation rule can be deleted
  outright rather than narrowed? Needs confirming during implementation.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- The section-diff output is an internal rendering contract, not a
  byte-for-byte frozen format some downstream skill parses or greps — if
  wrong, scope would need to widen to check callers before changing the
  diff body's formatting.
- Adding an in-process diff crate as a new dependency does not itself
  violate the workspace's cargo-deny allow-list — to verify at
  implementation time; doesn't change this item's scope either way.

## Technical Notes

- `similar` (crates.io) is the researched default recommendation:
  dependency-free, provides `TextDiff::unified_diff()` directly, Myers/
  Patience algorithms, large active user base. `imara-diff` (lower-level,
  Histogram algorithm, faster on large inputs) and `diffy` (patch-parsing
  focused) are alternatives worth a quick comparison at implementation
  time, but none is mandated by this work item.
- Current implementation for reference: `cli/work-adapters/src/diff_shellout.rs`
  (`render`, `render_with`, `run_capped`), consumed by `cli/work/src/section_diff.rs`
  and exercised through `accelerator work diff`.

## Drafting Notes

- 0174 (the general shell-tooling-retirement epic) is linked via `relates_to`
  rather than `parent`: this is philosophically part of that effort, but 0174
  doesn't literally scope this item, so an architectural-precedent link fits
  better than a decomposition edge.
- `relates_to` links 0170 (where this code was originally built) and
  0188/0198 (the analogous subprocess-to-in-process migration pattern on
  the VCS side) as architectural precedent, not because they share any
  code.
- `kind: task` chosen over `story` since there's no natural "as a user"
  framing for an internal implementation swap with no externally visible
  behaviour change.

## References

- Related: 0170, 0174, 0188, 0198

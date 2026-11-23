---
type: "work-item"
id: "PP-906"
title: "Linear Push Corrupts Markdown Content"
date: "2026-10-10T16:36:05+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "bug"
priority: "medium"
parent: "work-item:PP-167"
relates_to: ["work-item:PP-880", "work-item:PP-760", "work-item:PP-876", "work-item:PP-907", "work-item:PP-885"]
tags: ["sync", "linear", "bug", "markdown"]
last_updated: "2026-11-23T15:20:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-906"
aliases: ["0320"]
---
# PP-906: Linear Push Corrupts Markdown Content

**Kind**: Bug
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Linear's parsing of a pushed `description` changes the meaning of two kinds of
markdown: a number alone on a wrapped line inside a list item becomes an
ordered list, and bare dotted tokens such as `server.pid` become links.
Local work items are hard-wrapped at 80 columns and name files freely, so
every push risks these rewrites, and PP-880's equivalence reports them as real
differences. Push will escape both constructs so that Linear reads them as
plain text, leaving local files unchanged.

## Context

PP-880 tabulates thirteen ways Linear re-serialises a pushed `description`.
Eleven are cosmetic and absorbed by its equivalence; the two above change
meaning. A capture in `cli/work-adapters/tests/fixtures/linear-rewrites/`
pushed a body through `issueCreate` and read it back, so the rewrites happen
when Linear's API parses the body, not in its editor.

Linear's parser departs from CommonMark: CommonMark lets an ordered list
interrupt a paragraph only when it starts at `1`, and never links a bare
domain-shaped token. Both rewrites can be prevented by escaping that is
invisible to a CommonMark reader, so the fix belongs on the push side.

Until then, every item whose remote body carries a rewrite stays conflicted
on each sync (PP-880 defers PP-180, PP-733, PP-860 and PP-870 to this item),
and its Linear description shows a stray list or links to hosts that do not
exist.

## Requirements

**Defect 1 — a lone number on a wrapped line becomes an ordered list**

- Reproduction: PP-180 has the list item `- The tier infrastructure …
  owned by` wrapped onto a continuation line holding only `  0160.`; push it
  (capture fragment `X09`).
- Expected: the text `0160.` stays part of the list item.
- Actual: Linear reads the line as an ordered list item and stores it as
  `160. `, dropping the leading zero.
- Linear leaves a lone number at top level unchanged (`X09`). Where text
  follows the number, Linear's read-back only escapes it (`0160\.`, fragments
  `R11`, `X04`-`X06`), which is cosmetic.

**Defect 2 — dotted tokens become links**

- Reproduction: PP-860 contains bare `server.pid`; PP-870 contains bare
  `jql.rs:167`; PP-733 contains bare `crates.io`; push any. Capture fragment
  `R12` records `crates.io`; the other two come from diffing PP-860 and
  PP-870 against their `accelerator linear show` descriptions.
- Expected: plain text.
- Actual: `[server.pid](<http://server.pid>)`,
  `[jql.rs:167](<http://jql.rs:167>)` and `[crates.io](<http://crates.io>)`.

**Capture**

The unescaped capture is `linear-rewrites/pushed.md` and its
`read-back.json`; the escaped capture is the same pair under
`linear-rewrites/escaped/`. Each `##` section of either is one fragment.

- Extend the unescaped capture with one fragment per dotted-token
  class: domain name (`crates.io`), file name whose extension is a TLD
  (`server.pid`, `foo.md`), the same with a line number (`jql.rs:167`), file
  name whose extension is not a TLD (`foo.toml`), version and abbreviation
  (`v1.2.3`, `e.g.`), multi-part (`a.b.c`), and mixed case (`Crates.IO`).
  Add one fragment per boundary shape: sentence-final (`crates.io.`),
  bracketed (`(crates.io)`), path (`src/client.rs`) and email (`a@b.io`).
- Add a fragment with a wrapped continuation line holding only `<digits>)`
  inside a list item.
- Record, for each fragment, whether Linear links it or reads it as an
  ordered list.

**Required behaviour**

Push passes every body through an escaper before sending it. The escaper
walks the parsed text and applies two rules, the digit rule and the
dotted-token rule.

- Digit rule: push escapes `<digits>.` as `<digits>\.` and `<digits>)` as
  `<digits>\)`, for one or more digits with no upper bound, where they are the
  first non-whitespace text of a paragraph continuation line, as PP-880's pinned
  CommonMark parser reads it, in any container, and are followed by a space or
  the end of the line. This is wider than the observed defect on purpose. The
  rule does not depend on the container, so it also escapes a top-level lone
  number that Linear leaves alone; where text follows the number it matches the
  escape Linear stores anyway (`R11`); and it covers containers the unescaped
  capture never tried, such as blockquotes and nested or ordered-list items. The
  `<digits>)` escape is pre-emptive: CommonMark accepts `)` as an ordered-list
  delimiter just as it accepts `.`, so it is escaped whatever the unescaped
  capture records for it.
- Dotted-token rule: a bare dotted token is two or more dot-separated
  labels of letters, digits, `-` and `_`, optionally followed by
  `:<digits>`. It is bounded by whitespace, the start or end of a text run
  (the merged text between two non-text nodes), or any character other than
  letters, digits, `-`, `_`, `.` and `:`. A `.` not followed by a label
  character, or a `:` not followed by digits, ends the token and is not
  part of it, so a sentence-final period is never escaped. Push escapes
  every `.` in a bare dotted token whose last label is a top-level domain
  in a pinned snapshot of the IANA root zone list, compared ignoring case,
  and in no other token. A token with some dots already escaped has its
  remaining dots escaped. The unescaped capture is expected to confirm that
  this rule predicts which tokens Linear links.
- Bare scheme URLs are left as they are, dotted tokens inside them
  included; Linear links them as `[u](<u>)`, which PP-880 treats as cosmetic
  because the link text equals its destination. A bare scheme URL follows
  GFM's extended-autolink rule: it starts with `http://`, `https://` or
  `mailto:`, runs to whitespace or `<`, and leaves out trailing
  `. , : ; ! ? " '` and an unbalanced `)`. A schemeless email or `www.` host
  is not a scheme URL and stays subject to the dotted-token rule.
- Escaping works on parsed text, never on raw markdown: text inside code
  spans, fenced or indented code, link text, link destinations, angle-bracket
  autolinks and raw HTML is never escaped, and an escape already present is
  never escaped again.
- Every path that sends a local work item's body to Linear escapes:
  `work create --push`, the `sync` update, and the draft promotion and
  pending-push adoption paths PP-760 added.
- A pushed body read back from Linear is equivalent, under PP-880's
  definition, to the local body.
- A remote description already corrupted is repaired when someone resolves
  its item with `--resolve <id>=local` and the resulting push sends the
  escaped body; there is no bulk rewrite.

**Out of scope**

- The cosmetic re-serialisations PP-880 absorbs, including code-span
  indentation (`X07`, `X08`), which matches the CommonMark reading.
- Jira.
- Shortcut. Its parser has not been checked for the same rewrites.
- Any change to local files.
- The raw `accelerator linear create` and `update` commands, which keep
  sending `--body-file` verbatim so that captures can push unescaped bodies.
- Reporting the parser's behaviour upstream to Linear.

## Acceptance Criteria

Escaper output, tested in CI:

- [ ] Given a list item whose continuation line holds only `0160.` or
      `0160)`, when the body is escaped for push, then that line is sent as
      `0160\.` or `0160\)`.
- [ ] Given a top-level paragraph whose continuation line holds only
      `0160.`, or a continuation line `0160. more text` at top level or in a
      list item, when the body is escaped for push, then the number is sent
      as `0160\.`.
- [ ] Given a continuation line holding only `0160.` or `0160)` inside a
      blockquote, a nested list item and an ordered-list item, when the body
      is escaped for push, then the line is sent as `0160\.` or `0160\)`.
- [ ] Given a list item whose continuation line holds only `1234567890.`,
      when the body is escaped for push, then that line is sent as
      `1234567890\.`.
- [ ] Given a paragraph line followed directly by `1. text`, which
      CommonMark reads as a list interrupting the paragraph, when the body
      is escaped for push, then it is sent unchanged.
- [ ] Given a continuation line `0160) more text` in a list item, when the
      body is escaped for push, then it is sent as `0160\) more text`.
- [ ] Given a continuation line starting `2026.11` (no space after the
      `.`), when the body is escaped for push, then it is sent unchanged.
- [ ] Given each dotted-token fragment in the unescaped capture, when it is
      escaped for push, then every `.` in it is escaped exactly when the
      capture records Linear linking it.
- [ ] Given tokens not in either capture, when each is escaped for push, then
      `lib.rs`, `README.md` and `a.b.io:12` have every `.` escaped, and
      `config.yaml`, `v1.2.3` and `e.g.` are sent unchanged, and `Crates.IO`
      and `a.b\.io` are sent as `Crates\.IO` and `a\.b\.io`.
- [ ] Given `crates.io.`, `(crates.io)`, `src/client.rs` and `a@b.io`, when
      each is escaped for push, then it is sent as `crates\.io.`,
      `(crates\.io)`, `src/client\.rs` and `a@b\.io`.
- [ ] Given an ordered list (`1. a` then `2. b`), when the body is escaped
      for push, then it is sent unchanged.
- [ ] Given `https://crates.io/x.`, `(see https://crates.io)` and
      `mailto:a@b.io`, when each is escaped for push, then it is sent
      unchanged.
- [ ] Given `www.crates.io`, when it is escaped for push, then it is sent as
      `www\.crates\.io`.
- [ ] Given `crates.io` inside bold text, a heading and a table cell, when
      the body is escaped for push, then each is sent as `crates\.io` inside
      the same markup.
- [ ] Given `0160.` or `crates.io` inside a code span, a fenced block, an
      indented code block, link text, a link destination, an angle-bracket
      autolink (`<https://crates.io>`) or raw HTML, when the body is escaped
      for push, then it is sent unchanged.
- [ ] Given a local body that already contains `crates\.io` or `0160\.`,
      when it is escaped for push, then the body sent is unchanged.
- [ ] Given each fragment of the unescaped capture, when its escaped form is
      parsed as CommonMark, then it is equivalent under PP-880 to the
      unescaped fragment.
- [ ] Given a body needing both escapes, when it is pushed through
      `work create --push`, the `sync` update, draft promotion and
      pending-push adoption against a recording tracker (a test double that
      keeps every request body it receives), then each sends the escaped
      body, and the local file's body is byte-identical to its body before
      the push.
- [ ] Given the same body, when it is sent with `accelerator linear create
      --body-file` or `accelerator linear update --body-file`, then it is
      sent unchanged.
- [ ] Given the same body, when it is pushed through `work create --push` and
      the `sync` update to a Jira or Shortcut recording tracker, then it is
      sent unchanged.
- [ ] Given each fragment of the unescaped capture, when it is escaped for
      push, then the output is byte-identical to its fragment in
      `escaped/pushed.md`.

Linear's read-back, pinned from a committed live capture:

- [ ] Given the committed unescaped capture, then it holds a fragment for
      each dotted-token class and boundary shape and for a lone `<digits>)`
      continuation line, and its README table records Linear's rewrite of
      each.
- [ ] Given each fragment of `escaped/pushed.md`, escaped by hand, pushed to
      Linear and read back into `escaped/read-back.json`, when PP-880's
      equivalence compares it with its unescaped fragment, then they are
      equivalent: no ordered list, and no link other than a bare scheme
      URL's.
- [ ] Given the escaped capture's issue, kept until this check is recorded,
      viewed in Linear's web UI, then no
      backslash appears before a `.` or `)`; the `escaped/` README records
      the check.

Post-merge verification, run once by hand:

The affected items are PP-180, PP-733, PP-860 and PP-870, plus any other
conflicted item whose local body, diffed against its `accelerator linear
show` description after PP-880's normalisation, shows one of the two
rewrites. The PR lists the items checked.

- [ ] Given each affected item, when it is resolved with `accelerator work
      sync --resolve <id>=local`, then its next `accelerator work sync`
      reports it `synced`, and its description as rendered in Linear's web
      UI shows no stray list, no link on a dotted token and no backslash
      before a `.` or `)`.

## Open Questions

None open for discussion. Two premises still await the captures, and
refuting either sends all or part of the item back to drafting; see
Assumptions.

## Dependencies

- Blocked by: none (PP-880, which defines the equivalence, is done).
- Blocks: none. Clearing the conflicts PP-880 defers to this item (PP-180,
  PP-733, PP-860 and PP-870, named there by their aliases 0161, 0203, 0276
  and 0293) is this item's post-merge verification.
- External: Linear's API-side description parser and linkifier. A change to
  either means re-capturing the `linear-rewrites` fragments and revisiting
  the escape rules.
- External: credentialed access to the PP Linear workspace, for both
  captures and the post-merge verification, whose result also depends on
  the live state of the conflicted items.
- External: the IANA root zone TLD list, pinned as a snapshot; refreshing it
  can change which tokens are escaped.
- Internal: the escaper parses with PP-880's pinned CommonMark parser, so an
  upgrade of that parser re-runs the escaper's equivalence fixtures.
- Before the dotted-token rule is written: the new fragments are committed
  to the unescaped capture.
- Before the escaped capture is taken: the new fragments are committed to
  the unescaped capture, and each is escaped by hand under the digit and
  dotted-token rules as stated. If only the digit rule lands, the escaped
  capture is retaken with digit escapes alone.
- Before the escaper is written: the hand-escaped fragments are pushed,
  their read-back committed as the escaped capture, and the issue checked in
  Linear's web UI.
- Internal: both captures' read-backs go through the allowlist in
  `work-item-sync-recovery/README.md` before they are committed.
- Coordinated: PP-876 — Bidirectional Field Mapping changes what a `sync`
  push sends and bumps PP-880's digest recipe. If this item lands first,
  PP-876 routes its description-sending path through the escaper; if it
  lands second, the escaper wraps PP-876's path. Either way, the
  equivalence fixtures run against the current recipe.
- Related: PP-885 — Push PR Descriptions to GitHub Unwrapped adds a
  push-side Markdown transform. Whichever lands second reuses the other's
  Markdown parser crate rather than adding another.
- Related: PP-760 — Tracker-Owned Work Item ID Generation (done) added the
  promotion and pending-push adoption paths this item escapes.
- Related: PP-907 — Sync Authority Model. This item proceeds without waiting
  for its ADR; a revised representation rule could move the fix from push to
  the local file.

## Assumptions

- Linear renders `\.` and `\)` as plain `.` and `)`, so the escapes are
  invisible to people reading the issue in Linear. Viewing the escaped
  capture in Linear's web UI confirms or refutes this; if it refutes it, the
  item returns to drafting before the escaper is written.
- Linear links a bare dotted token when its last label is a TLD (`.pid` and
  `.rs` are real TLDs), using a list close enough to IANA's that the pinned
  snapshot predicts every captured fragment. If the unescaped capture
  contradicts the snapshot for a class, only the dotted-token rule returns
  to drafting; the escaper may land with only the digit rule.

## Technical Notes

- `LinearClient` sends `body` unchanged in `create_op`
  (`cli/linear-client/src/client.rs`) and `update`, and its `BodyDialect`
  keeps the default identity `as_pushed`.
- PP-880's equivalence merges adjacent text runs
  (`cli/corpus/src/body/canonical.rs`), so an escaped body and its unescaped
  local form compare equal and the baseline digest is unaffected.
- Linear's serialisation carries escapes such as `0160\.` (`R11`), and a
  pull writes it into the local file; a text-level escaper would turn these
  into `0160\\.`, a literal backslash.
- If Linear widens its linkifier, a newly linked token surfaces as a conflict
  on the item's next sync; the fix is a new capture fragment and a refresh
  of the TLD snapshot.
- The corrupted bodies were found by diffing local bodies against `accelerator
  linear show` descriptions after PP-880's normalisation.

## Drafting Notes

- Framed as a push-time problem because both rewrites trace to hard wrapping
  or bare tokens in local markdown, and escaping keeps the local file
  unchanged.
- Code-span indentation (former Defect 3) dropped after PP-880 and the capture
  showed it matches the CommonMark reading.
- Bare URLs stay linked by decision, not by limitation: escaping them would
  remove a link a reader expects.
- Dotted-token escaping matches Linear's linkifier rather than escaping every
  inner `.`, keeping Linear's stored markdown free of needless escapes at the
  cost of a capture and of drift surfacing as conflicts.

## References

- Parent: PP-167 — Work Item Synchronisation Enhancements
- Related: PP-880 — Sync Round-Trip Defects Between Local Work Items and the
  Tracker
- Related: PP-760 — Tracker-Owned Work Item ID Generation
- Coordinated: PP-876 — Bidirectional Field Mapping
- Related: PP-885 — Push PR Descriptions to GitHub Unwrapped
- Related: PP-907 — Sync Authority Model Between Local Work Items and the
  Tracker
- Capture: `cli/work-adapters/tests/fixtures/linear-rewrites/README.md`
- Allowlist:
  `cli/work-adapters/tests/fixtures/work-item-sync-recovery/README.md`
- PP-880, PP-876, PP-885 and PP-907 refer to this item by its alias 0320.

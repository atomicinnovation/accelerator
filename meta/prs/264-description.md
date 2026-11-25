---
type: "pr-description"
id: "264"
title: "[PP-906] Escape Markdown pushed to Linear and warn when Linear rewrites it"
date: "2026-11-24T18:57:20+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-906"
parent: "work-item:PP-906"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/264"
pr_number: 264
tags: []
revision: "6523c4d21cfe84921f61aae63b5a3e0ca13ff031"
repository: "accelerator"
last_updated: "2026-11-25T23:28:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-906] Escape Markdown pushed to Linear and warn when Linear rewrites it

## Summary

Linear's parser changes the meaning of two things our hard-wrapped work items
write all the time. It turns a bare dotted token such as `crates.io` or
`server.pid` into a link. It also turns a wrapped line holding only `0160.`
into an ordered list. Every description sent to Linear is now escaped so that
Linear reads both as plain text, and local files stay unchanged. Every body
push to any tracker now also compares what was stored with what was sent. A
push whose meaning changed is reported on stderr and marked in the sync
baseline until it is resolved, or accepted when Linear cannot store the
text.

The goal is that a local work item can be synced to and from Linear
repeatedly. Syntax Linear cannot represent at all gets no special handling.
Instead, the push reports the rewrite, and a pull adopts Linear's form.

## Changes

### Escaping (`markdown::linear`, `linear-client`)

- **New `cli/markdown/src/linear` module** with two rules, both probed against
  Linear's live parser.
  - **The digit rule** escapes a lone number on a continuation line.
  - **The dotted-token rule** escapes the `.` in tokens Linear would link. It
    uses a pinned IANA root-zone snapshot (`tlds-alpha-by-domain.txt`) and
    reads labels the way Linear does, including combining marks and format
    characters (NFD or NFC). Tokens split by inline markup are never linked
    by Linear, so they are left alone.
- **`escape_for_push` never changes a body's meaning.** All escapes are applied
  in one pass. Only if that changes how the body reads are they added back one
  at a time, and only those that keep the meaning are kept.
- **`linear-client`'s new `Description` type** escapes every description on
  `issueCreate` and `issueUpdate`. This covers `linear create`, `linear
  update`, `work sync`, draft promotion, `work create --push`, `work
  create-batch --push` and `work update --push`. `--verbatim` on `linear
  create` and `linear update` sends the text exactly as given.
- **New `linear update --body-file <PATH>`**, matching `linear create` and
  the Jira and Shortcut commands. An unreadable file is refused with
  `E_UPDATE_NO_FILE` (exit 115).

### Reporting a rewritten push (`work-adapters`, `work-cli`)

- **`BodyComparison::push_fidelity`** (`sync/push_fidelity.rs`) judges each
  push's read-back with PP-880's equivalence. It gives one of three verdicts:
  faithful, rewritten, or unjudged when no read-back could be taken. A
  rewrite always names at least one changed line, through the `ChangedLines`
  type. If no single section holds the change, the warning shows the whole
  body.
- **Every push site prints the warning**: `work sync`, `work promote`, `work
  create --push`, `work create-batch --push` and `work update --push`. It
  looks like this:

  ```text
  warning: 0001: rewritten on push: Notes
  note: 0001: Notes: sent `a⏎⏎b`, stored `a⏎b`
  ```

  The push still succeeds, the baseline is recorded and exit codes are
  unchanged. Line breaks show as `⏎`, because a `\n` marker would be mistaken
  for one of the backslash escapes these lines are full of.
- **One verdict per push** (`sync/push_verdict.rs`). `PushVerdict` turns a
  read-back into the mark change and the notice. `PushOrigin` (`Pushed`,
  `Created`, `CreatedEarlier`) says whether this run sent the body, so no
  push site chooses `Judged`, `Fresh` or `Unjudged` itself.
- **A promotion is judged when it finishes.** `finish_promotion` judges the
  promoted file against the remote whenever the record shows the retitle
  landed. A promotion resumed after a failed retire therefore marks its key
  as an uninterrupted one would. It reuses the read already made to predict
  the next sync, so no tracker call is added.
- **Warnings print through a testable seam.** `create`, `update` and
  `promote` print through `report(out, err) -> ExitCode`, so tests assert
  what reaches stdout and stderr and the exit code.
- **The rewrite mark** (`sync/rewrite_mark.rs`): `last-sync.json` entries gain
  a `rewritten_on_push` mark.
  - All per-item baseline writes go through `BaselineStore::record`, which
    sets, keeps or clears the mark.
  - Writing an entry directly (`seed`) is possible only under a test-only
    feature, and a lint enforces that.
  - Unknown keys on an entry survive a round trip, so an older build keeps
    what a newer one wrote. A push with no read-back keeps even a mark this
    build cannot read.
  - The mark stores every section a rewrite touched, not just those of its
    first 5 lines, so a standing notice names them all.
- **Clearing the mark.** It is cleared only by a faithful push, or by a
  reconciliation in which classification confirmed that the two bodies agree.
- **Showing the mark.** While it stands, every later sync and `work update
  --push` warns `still rewritten on push`, and `work list` shows the item as
  `🟢 synced · rewritten on push`.
- **Rewrites Linear cannot avoid.** `StorageLimit` (`linear-client`) finds
  an escaped `<` or `&`, or an escaped `|` in a table cell, in a sent line.
  When every changed line holds one, the notice says Linear cannot store it
  and offers `accelerator work update <path> --accept-rewrite` instead of
  the backticks advice and the request to report it.
- **Accepting a rewrite.** `--accept-rewrite` marks the recorded rewrite
  accepted, with no tracker call. The mark stays, so an accepted rewrite
  never looks resolved. `work list` shows `· rewrite accepted`, and nothing
  warns until a push is rewritten differently.

### Simulator and captures (`tracker-test-support`, fixtures)

- **The Linear simulator and the capture oracle share one model**,
  `linear_autolinker::linear_leaves_bare`, so the two cannot drift apart. A
  test checks the simulator against more than 1,000 recorded probes.
- **Live captures from the `TES` team**, each passed through the standard
  allowlist and documented in the fixture READMEs:
  - `linear-rewrites/`: the unescaped and escaped fragments, and the comment
    read-back.
  - `linear-autolink-probes/`: dotted tokens and the TLD sweep, entities, the
    second push of pulled entities, and tokens split by markup or holding
    combining marks.

### Skills, evals and docs

- **Skills that pass on the warnings:** `/sync-work-items`,
  `/create-work-item` and `/extract-work-items` relay the rewrite warning.
  `/sync-work-items` and `/list-work-items` also relay the mark.
- **Skills that gain flags:** `/create-linear-issue` and
  `/update-linear-issue` gain `--verbatim`, and `/update-linear-issue` also
  gains `--body-file`.
- **Targeted evals re-run** under the new skill digests, one commit per skill,
  all passing in full:
  - `/sync-work-items`: 1–4
  - `/create-work-item`: 40, 48, 51, 62 and 66
  - `/extract-work-items`: 8–10
  - `/list-work-items`: 1
- **New evals that feed each skill a rewrite warning**, all passing in full
  (27 of 27 expectations):
  - `/sync-work-items` 5 and 6: a rewritten push, and a rewrite Linear
    cannot avoid
  - `/list-work-items` 2: the rewritten and accepted labels
  - `/create-work-item` 76 and `/extract-work-items` 14: a rewritten push
  - `/update-linear-issue` 1, its first eval: a `--body-file` update sent
    without `--verbatim`
- **CHANGELOG** entries under Changed, Added, Fixed and Migrations.

## Context

- Work item: `meta/work/PP-906-linear-push-corrupts-markdown-content.md`
- Plan: `meta/plans/2026-11-23-PP-906-escape-linear-bound-markdown-on-push.md`.
  Each phase's "As landed" notes record where the code departs from the plan.
- Research:
  `meta/research/codebase/2026-11-23-PP-906-linear-push-corrupts-markdown-content.md`
- Reviews: `meta/reviews/work/PP-906-…-review-1.md` and
  `meta/reviews/plans/2026-11-23-PP-906-…-review-1.md`
- Validation:
  `meta/validations/2026-11-23-PP-906-escape-linear-bound-markdown-on-push-validation.md`
- PR review: `meta/reviews/prs/264-review-1.md`. All 10 threads are answered;
  2 are deferred to PP-1005.
- Follow-up: PP-1005
  (`meta/work/PP-1005-audit-linear-issues-for-rewrites-left-unmarked.md`), an
  opt-in `work sync --audit-rewrites` that marks rewrites this build never
  saw.

## Testing

- [x] The full `mise run` exits 0 at this PR's tip (`ststzopr`), after the
  review response: 8,661 Rust tests passed with 2 skipped, and the
  formatters changed nothing.
- [x] Every behaviour change was written test-first. The probe and
  convergence tests were checked by mutation: breaking a recorded list made
  them fail and name the probe.
- [x] Live captures on `TES` (`TES-38` to `TES-41`). Every scratch issue was
  deleted and confirmed `trashed: true`, and `.accelerator/config.local.md`
  was restored and checked with `cmp`.
- [x] Settling across push → pull → push, measured live: every meaning-changing
  case settles after one cycle, except doubly-encoded `<` and `&` (`E004`,
  `E018`), which take two. A test pins this.
- [ ] Phase 12, post-merge verification. It runs by hand against the `PP` team
  once this release has reached everyone who pushes there, because an older
  build re-corrupts what it pushes. It audits and repairs affected items,
  checks the tables of `PP-87` and `PP-22` by hand, and checks the web UI.
  The plan stays `in-progress`, and PP-906's two post-merge acceptance
  criteria stay open until it is done.

## Notes for Reviewers

- **Stacked on #246** (`0333-always-write-work-item-tags`). Review against
  that base.
- **Unrelated commits in the stack.** `Promote the design-crawl
  request-policy draft onto PP-992` (`pnnrzovw`) is not PP-906 work. It renames
  the `draft-tdjepd` work item to `PP-992` and updates the references to it in
  `meta/prs/253-description.md` and `meta/reviews/prs/253-review-1.md`. The
  same change is on #258's branch. The edits to PP-167, PP-876 and PP-907 are
  cross-references to PP-906.
- **⚠️ Upgrade together.** Any baseline write by an earlier release drops
  every `rewritten_on_push` mark and unknown key in the committed
  `last-sync.json`. An earlier release also still pushes unescaped and
  corrupts descriptions again without warning. Nothing rebuilds a lost mark
  yet; PP-1005 proposes the audit that would.
- **Where to look first:**
  - `escape_for_push` in `cli/markdown/src/linear.rs`, for the rule that an
    escape never changes meaning.
  - `push_fidelity.rs` and `rewrite_mark.rs`, for the verdict and the mark.
  - `BaselineStore::record`, for the single route by which every per-item
    baseline write sets, keeps or clears the mark.
  - `push_verdict.rs` and `finish_promotion`, for how each push site judges
    its read-back.
- **What Linear cannot store, by decision:**
  - No form of `<`, `&` or a table cell's `|` survives as text, whether sent
    as an entity or behind a backslash. The push reports the rewrite, names
    the character, and offers `--accept-rewrite`; a pull adopts Linear's form.
  - Linear stores comments exactly as sent, so `comment add` does no escaping
    and has no `--verbatim`.
  - A non-ASCII last label (`foo.café`) is left bare, as Linear does.
- **Known limits:**
  - A literal `⏎` in a body looks the same as a line break in a warning.
  - On `/create-work-item` evals 48 and 66, the graders could check "the
    draft is unchanged" only from the response text. Both evals still pass.
  - The visualiser kanban end-to-end specs sometimes rewrite
    `cli/visualiser/server/tests/fixtures/meta/work/0001-first-work-item.md`
    (`draft` → `ready`). This flakiness predates this PR.

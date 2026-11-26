---
type: "pr-review"
id: "264-review-1"
title: "[PP-906] Escape Markdown pushed to Linear and warn when Linear rewrites it"
date: "2026-11-24T19:19:46+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:264"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "performance", "compatibility", "usability", "safety"]
review_number: 1
pr_number: 264
tags: []
last_updated: "2026-11-24T19:19:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #264 - [PP-906] Escape Markdown pushed to Linear and warn when Linear rewrites it

**Verdict:** COMMENT

The PR is well designed. The escaper only adds backslashes, it checks every set of escapes against the `document` reading, and it is validated against more than 1,000 live Linear probes. `Description` makes escaping a decision the type system enforces at the wire boundary. `BaselineStore::record` is the only route by which a per-item baseline write sets the rewrite mark. The weaknesses all concern the mark's lifecycle. Several paths lose a rewrite signal, or never create one, and nothing rebuilds a lost mark. Five push sites each assemble the steps after a push by hand. The new skill and stderr relay behaviour has no end-to-end test.

### Cross-Cutting Themes

- **The rewrite mark can be lost and is never rebuilt** (flagged by: safety, compatibility, usability, correctness, architecture). The mark is lost or never set in 6 cases:
  - An older build strips every mark from the committed `last-sync.json` on any baseline write.
  - Items Linear rewrote before the upgrade are never marked.
  - A `Judged(Unjudged)` push erases a mark this build cannot read.
  - A promotion whose retire step fails finishes unjudged.
  - A create resumed from a pending-push marker is never judged.
  - A failed read-back or baseline write prints nothing.

  The common fix is one mechanism. When `work sync` has already fetched an item's remote body, it should judge that body against the local one with `push_fidelity` and set or warn on a mark, without pushing. That lets the mark rebuild itself, and it gives a discovery path for damage done before the upgrade.
- **The steps after a push are reassembled at 5 sites** (flagged by: architecture, code-quality). Those steps are read-back, judge, choose a `MarkChange`, record, and choose a notice. Each site picks `Judged`, `Fresh` or `Unjudged` on its own. The duplication shows in four ways:
  - `sent_create: bool` in `creation.rs` and `IssueCreated` in `apply.rs` model the same concept.
  - Promotion threads the verdict through a `&mut PushFidelity` out-parameter.
  - The rule for choosing a standing notice is duplicated in `update.rs` and `run.rs`.
  - `entry.mark().mark()` appears at 3 sites.
- **Rewrite warnings label the item differently by command** (flagged by: usability, code-quality). The label is an id under `work sync`, the tracker key under promote, and a path under create or update, which is sometimes absolute and sometimes repo-relative.
- **The Linear simulator reuses the production tokenizer** (flagged by: architecture, test-coverage). A token shape that `markdown::linear::dotted_tokens` misses is invisible to every simulator-backed test.
- **Display truncation shapes the persisted mark** (flagged by: architecture, correctness). `RewrittenLines::bounded` (`SHOWN_LINES`, `LINE_BYTES`) decides what lands in `last-sync.json`. Section names beyond the first 5 lines are lost, so the standing notice names fewer sections than the first notice did.

### Tradeoff Analysis

- **Usability vs Safety**: Usability wants a way to stop a permanent `still rewritten on push` warning for content Linear cannot store (`<`, `&`, a `|` in a table cell). Safety treats the mark as the only durable signal of remote damage. The recommendation is an explicit accepted state that records the stored form. Clearing the mark would make an accepted rewrite look the same as a resolved one.
- **One model of Linear vs independent test doubles**: the PR deliberately shares the autolinker model so that the simulator and the oracle cannot drift apart. Test Coverage and Architecture want the simulator's candidate scan kept independent. The recommendation is a simple scan in the simulator that over-matches, pruned by `linear_leaves_bare` and calibrated by the probes. The shared tokenizer is then checked against live captures, not against itself.

### Strengths

- ✅ `escaped_or_original` fails safe. If the full set of escapes changes the reading, it rebuilds the set one escape at a time. The worst case is an under-escape, which is visible and reported, never a silent change of meaning. Escaping is idempotent and only inserts backslashes.
- ✅ Escaping and detection are separate layers. If the escaper's model of Linear is wrong, the read-back comparison in `push_fidelity` still catches the divergence for every tracker.
- ✅ `Description::{Markdown, Verbatim}` forces every `issueCreate`/`issueUpdate` call site to choose. The sync engine stays tracker-agnostic, just as the ADF conversion is kept in `jira-client`.
- ✅ `markdown::linear` is a pure core of small single-purpose modules. Extracting `Splice`/`spliced` removes the duplicated splice loops between the dialects.
- ✅ `BaselineStore::record` plus `MarkChange` give mark transitions a closed vocabulary. `seed` is gated behind a test-only feature with a lint, and unknown keys and unreadable marks survive round trips.
- ✅ `ChangedLines` holds by construction the rule that a rewrite always names at least one changed line.
- ✅ The push adds no extra tracker calls: every verdict reuses the existing `show` read-back. The TLD set is a `LazyLock<HashSet>` built from `include_str!`.
- ✅ The tests are thorough:
  - `assert_escapes` checks expected output, idempotence, insertions only, an unchanged reading, and the same under CRLF.
  - The mark rule is pinned as a table at every layer, through to the real HTTP clients.
  - The expected escaped literals are independent of `escape_for_push`.
- ✅ Every warning ends in a concrete next step. Line breaks show as `⏎`, stored text is reduced to one line so it cannot forge a `warning:` line, and stdout and exit codes are unchanged. The skills are told never to suggest a pull as the remedy.
- ✅ Breaking escaping is listed under Breaking in the CHANGELOG, with a `--verbatim` opt-out. `E_UPDATE_NO_FILE` (115) takes a free slot, and a parity test checks it.
- ✅ The Phase 12 repair plan is defensive: it takes a backup first, needs a y/N confirmation for each item, re-checks `updatedAt`, confirms with `cmp`, and re-audits a week later. The `TES` fixtures hold only synthetic text.

### General Findings

- 🔵 **Usability**: the standing notice repeats the snapshot taken at push time. It does not say "as of the last push", so users may search their current file for text they have since reworded.
- 🔵 **Architecture**: `rewrite_mark.rs` mixes the domain rule for setting, keeping and clearing the mark with JSON persistence. The rule belongs beside `BodyAgreement` in `work::sync`.
- 🔵 **Code Quality**: adding `rewrite` to `ReportedItem`, `PromotionRow` and `BatchItem` took about 15 struct-literal edits. A constructor with defaulted attachments would avoid that.
- 🔵 **Code Quality**: 4 similarly named `rewrite_warnings`/`retitle_warnings` adapters each derive the label and path form differently.
- 🔵 **Compatibility**: existing Linear issues stay corrupted and unmarked until each is pushed again. The Migrations section should give the repair step more prominence.
- 🔵 **Test Coverage**: the `work list` wiring for the mark in `sync_labels` is untested end to end. Passing an empty set would fail no test.
- 🔵 **Test Coverage**: the Linear escape expectation is duplicated across 2 test crates, and the `ESCAPED` literal in `linear-cli`/`linear-client` repeats it.
- 🔵 **Correctness**: the escaper escapes a non-ASCII last label (`foo.рф` → `foo\.рф`), but `linear_leaves_bare` and the PR description say Linear leaves such labels bare. This preserves meaning, but the PR's claim of a single model is inaccurate.
- 🔵 **Safety**: the meaning-preservation guarantee is measured against CommonMark, not Linear. Keep the probe-corpus test as a required gate.
- 🔵 **Performance**: push fidelity re-parses bodies the digest step has just parsed. This is negligible next to the HTTP round-trip.

### Additional Findings

- 🔵 `cli/work-adapters/src/creation.rs:379` — `sent_create: bool` models the same concept as `IssueCreated` (code-quality)
- 🔵 `cli/work-adapters/src/sync/baseline.rs:80-92` — `KNOWN_KEYS` duplicates the key list `render` writes (code-quality)
- 🔵 `cli/work-adapters/src/sync/run.rs:390-393` — `entry.mark().mark()` accessor chain repeated at 3 sites (code-quality)
- 🔵 `cli/work-adapters/src/sync/run.rs:2538` — no test covers the preview path's standing-rewrite notice (test-coverage)
- 🔵 `cli/work-cli/src/create.rs:1354-1358` — the note-line assertion passes when there are no note lines (test-coverage)
- 🔵 `cli/work-cli/src/update.rs:309-326` — the rule for choosing a standing notice is duplicated in `run.rs` (architecture)
- 🔵 `cli/markdown/src/linear.rs:54-59` — the fallback re-parses the whole body twice per candidate escape, O(k·n) (performance)
- 🔵 `cli/work-cli/src/report_records.rs:131-135` — "please report it" is printed even for known, by-decision limits (usability)
- 🔵 `cli/work-cli/src/sync.rs:982-991` — the same warning names the item differently by command (usability)
- 🔵 `skills/integrations/linear/update-linear-issue/SKILL.md:9` — the argument hint advertises `--verbatim`, a flag meant only for capture work (usability)
- 🔵 `cli/work-adapters/src/promotion.rs:447-459` — the `&mut PushFidelity` out-parameter hides the data flow (code-quality)
- 🔵 `cli/work-cli/src/update.rs:259-285` — the verdict is lost when the read-back or the baseline write fails (safety)
- 🔵 `cli/work-adapters/src/sync/rewrite_mark.rs:72-83` — display constants shape the persisted mark (architecture)
- 🔵 `cli/tracker-test-support/src/rewrite.rs:192` — the simulator finds candidates with the production tokenizer (architecture, test-coverage)
- 🔵 `cli/markdown/src/linear/tests.rs:793-816` — the corpus test depends on the live `meta/work` directory (test-coverage)
- 🔵 `cli/work-adapters/src/sync/apply.rs:1703-1714` — a resumed create is recorded unmarked and never judged (correctness)
- 🔵 `cli/work-adapters/src/sync/push_fidelity.rs:53-60` — the whole-body fallback cuts both sides to 200 bytes and gives no pointer to the full text (usability)
- 🔵 `meta/plans/2026-11-23-PP-906-escape-linear-bound-markdown-on-push.md:2606-2607` — the pre-repair backup lives only in a gitignored tmp directory (safety)
- 🔵 `cli/markdown/src/linear.rs:76-78` — `reads_alike` recomputes the original's reading on every call (performance)
- 🔵 `cli/work-adapters/src/sync/push_fidelity.rs:231-243` — `meaningful_hunks` re-runs `as_pushed` once per hunk (performance)
- 🔵 `cli/linear-cli/src/cli.rs:58-62` — consider hiding `--verbatim` from the default `--help` (usability)
- 🔵 `cli/work-cli/src/update.rs:309-320` — the tuple parameter `(path, external_id)` is an unnamed data clump (code-quality)
- 🔵 `cli/work-adapters/src/sync/push_fidelity.rs:137-143` — the section de-duplication loop is duplicated in `RewriteMark::rewritten_lines` (code-quality)
- 🔵 `cli/work-adapters/src/sync/rewrite_mark.rs:46-53` — `MarkChange::Judged(&PushFidelity::Unjudged)` reads as a contradiction (code-quality)
- 🔵 `cli/work/src/sync/plan.rs:339-340` — the body agreement is attached by an optional builder that only one arm calls (code-quality)
- 🔵 `cli/markdown/src/linear/dotted_tokens.rs:104-146` — `DottedToken::in_word` packs a three-stage scanner into one loop (code-quality)
- 🔵 `cli/markdown/src/linear.rs:16` — `is_top_level_domain` is public API with no external consumer (architecture)
- 🔵 `cli/work-adapters/src/sync/baseline.rs:141-147` — kept unknown keys can go stale against refreshed fingerprints (compatibility)
- 🔵 `cli/markdown/src/linear/tests.rs:136-140` — the shared escaping assertion is tied to the internal proposal step (test-coverage)

---
*Review generated by /review-pr*

## Inline Comments

### `cli/work-adapters/src/sync/rewrite_mark.rs:68` — Any older build silently strips every rewrite mark from the committed baseline, with no guard and no rebuild
**Severity**: major | **Confidence**: high | **Lens**: safety

🟡 **Safety**

The `rewritten_on_push` mark is stored inside the shared, committed `.accelerator/state/integrations/linear/last-sync.json`. Any build from before PP-906 rewrites that whole document on every sync (`finalise_run`), so a single sync by one teammate on an old plugin version removes every mark. The removal then spreads through VCS. Nothing enforces the PR's "Upgrade together" note. Once a mark is gone, no path brings it back: a later sync that classifies the item as unchanged does not push, so it never judges the item again. The Linear description stays rewritten, and the only signal there was is gone for good. Phase 12 has the same weakness. Its precondition, that the release has reached everyone who pushes to `PP`, is checked only by someone asserting it.

**Impact**: Remote content whose meaning changed goes back to being silent damage in the system of record.

**Suggestion**: Let the new build rebuild a lost mark. For example, when sync has already fetched the remote for an item it classifies as synced, judge that body against the local one with the same `push_fidelity` equivalence, and set the mark or warn when they differ. Alternatively, keep the marks in a sidecar file that older builds never touch. In either case, state in the PR and the CHANGELOG the minimum plugin version required before Phase 12 runs.

---

### `cli/work-cli/src/update.rs:265-270` — Push → judge → mark → notice protocol is reassembled at every push site, one of them in the CLI layer
**Severity**: major | **Confidence**: high | **Lens**: architecture

🟡 **Architecture**

The steps after a push are put together by hand at 4 places:

- `record_pushed_read_back`/`link_and_baseline` in `apply.rs`.
- `creation.rs`, with its `sent_create` flag.
- `promotion.rs`, with a `&mut PushFidelity` out-parameter.
- Here, in `work-cli/src/update.rs`.

Those steps are: show the issue, call `comparison().push_fidelity(...)`, choose a `MarkChange`, call `BaselineStore::record`, then build `RewriteNotice::on_push` or a standing notice. Each site makes its own choice between `Judged`, `Fresh` and `Unjudged`. `BaselineStore::record` centralises the mark transition, but not the decision about which verdict to pass it.

**Impact**: A site that forgets to judge, or picks `Fresh` where `Judged` was meant, compiles fine and silently drops or keeps a mark. The CLI command now owns sync-domain orchestration that belongs in `work-adapters`.

**Suggestion**: Extract one adapter-level operation, such as `PushedReadBack::record(store, id, sent, read_back, origin) -> (Baseline, Option<RewriteNotice>)` in `work-adapters::sync`, where `origin` is `ThisPush`, `FreshCreate` or `Reused`. Call it from all 5 sites.

---

### `CHANGELOG.md:1333-1335` — Mixed-version migration note understates what an older release does
**Severity**: major | **Confidence**: high | **Lens**: compatibility

🟡 **Compatibility**

The Migrations entry says only that "a sync by an earlier release drops every `rewritten_on_push` mark". In fact a build from before this PR re-renders the whole `last-sync.json` on every baseline write: `work update --push`, `work create --push`, promotion and rename as well as sync. Each such write drops every mark and every unknown key. More seriously, such a build still pushes descriptions to Linear unescaped, which puts back the link and list corruption this PR fixes. It then records the corrupted read-back as the baseline, without a mark.

**Impact**: In a team on mixed versions, one teammate can silently re-corrupt Linear issues and wipe the marks that would have flagged it.

**Suggestion**: Widen the entry to cover any baseline write by an earlier release, including the loss of unknown keys. State that an earlier release still pushes unescaped descriptions and corrupts them again without any warning. Point to `accelerator work update <path> --push` as the repair once everyone has upgraded.

---

### `skills/work/sync-work-items/SKILL.md:375-396` — New skill relay behaviour has no eval that exercises it
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

The skill gains new instructions: relay `rewritten on push` and `still rewritten on push` with their notes verbatim, quote them as data, and never suggest a pull. `/list-work-items`, `/create-work-item` and `/extract-work-items` gain similar instructions. The two Linear skills gain `--verbatim` and `--body-file`, and they have no evals at all. No `evals.json` was changed. The benchmark updates only re-run existing evals, and none of those feeds the skill a rewrite warning.

**Impact**: A skill that dropped or paraphrased the notes, or suggested a pull, would still pass every eval.

**Suggestion**: Add at least one targeted eval per changed skill whose simulated stderr carries the warning block. Check that the response relays the `warning:`/`note:` lines verbatim, names `accelerator work update <path> --push`, and offers no pull. For `/list-work-items`, check that ` · rewritten on push` is relayed.

---

### `cli/work-adapters/src/promotion.rs:994` — A promotion's rewrite is lost when the retire step fails and a later run finishes it
**Severity**: major | **Confidence**: medium | **Lens**: correctness, architecture

🟡 **Correctness**

Only the run whose retitle landed passes a real verdict to `finish_promotion`. If that run's retire step fails, the record stays at `RemoteRetitled`. The next run's `Retire` or `Finish` step then passes `PushFidelity::Unjudged`. The new key has no previous mark, so the entry is recorded unmarked, with remote fingerprints taken from the rewritten read-back. The next sync classifies the item as synced and never judges it again. The one-off warning from the failed run is the only trace.

🔵 **Architecture**

Every other promotion stage is persisted to the promotion record so an interrupted run can resume. The retitle's `PushFidelity` is the exception. It is threaded through a `&mut PushFidelity` out-parameter and never written to the record. That weakens the recovery guarantee in exactly the case this PR is meant to surface durably.

**Suggestion**: Carry the verdict, or the bounded changed lines, in `PromotionStage::RemoteRetitled`/`Retiring` so that `finish` can judge from the record. Alternatively, write the mark when the retitle is judged, before the retire step can fail.

---

### `cli/work-cli/src/report_records.rs:126-130` — One remedy for every rewrite, including content Linear cannot store, and the mark cannot be dismissed
**Severity**: major | **Confidence**: medium | **Lens**: usability

🟡 **Usability**

Every rewrite notice gives the same remedy: put the text in backticks or reword it, then run `work update --push`. The PR says some content cannot be represented, by decision: `<`, `&`, and a `|` in a table cell. Backticks do not stop GFM splitting a table cell at `|`, and they turn prose into code in the local file. Nothing dismisses the mark. `noting_standing_rewrites` attaches `still rewritten on push` to every marked item on every sync, previews included.

**Impact**: An item that legitimately holds such content warns on every sync for good, so users learn to ignore the warning.

**Suggestion**: Make the remedy match the cause where it is known, for example "Linear cannot store `|` in a table cell; this rewrite cannot be avoided". Consider an explicit way to accept a rewrite that records the stored form, so an accepted rewrite stays distinct from a resolved one.

---

### `CHANGELOG.md:1123-1125` — No way to find items Linear rewrote before the upgrade
**Severity**: major | **Confidence**: medium | **Lens**: usability

🟡 **Usability**

The Fixed entry tells users to run `accelerator work update <path> --push` to repair an item Linear has already rewritten. But only a push made with this build sets the `rewritten_on_push` mark. Items corrupted by earlier pushes have no mark, `work list` does not flag them, and no `still rewritten` warning appears. The user has to guess which `<path>` needs repairing. The PR's own Phase 12 audit of the `PP` team is done by hand.

**Impact**: For other users of the plugin, the advertised fix cannot be applied in practice.

**Suggestion**: Provide or document a step that finds these items. It would compare each synced item's stored description with its local body using the same `push_fidelity` judgement, without pushing, and mark or list the ones that differ. A mode such as `work sync --audit-rewrites` would make the repair a command anyone can run.

---

### `cli/work-cli/src/main.rs:287-289` — No binary-level test checks that the rewrite warnings reach stderr
**Severity**: major | **Confidence**: medium | **Lens**: test-coverage

🟡 **Test Coverage**

The warning lines are tested only as data: `RunOutcome::{Created, Updated}`, `PromoteOutcome::Reported` and `rewrite_warnings(&report, ..)`. The last step is untested: the `eprintln!` loops added here and the `console.warn` loop in `sync.rs`. Deleting any of these loops fails no test. `cli/work-cli/tests/` already pins other stderr warnings end to end, with about 90 `stderr.contains` assertions.

**Impact**: The stderr block is the whole user-facing output of this half of the PR, and the skills key on it.

**Suggestion**: Add one binary test per entry point, or at least for `work sync` and `work update --push`, against a mock tracker whose read-back links `crates.io`. Assert that stderr contains `warning: <id>: rewritten on push:` and that the exit code is unchanged.

---

### `cli/work-adapters/src/sync/rewrite_mark.rs:229-231` — An unjudged push erases a mark this build cannot read
**Severity**: minor | **Confidence**: high | **Lens**: safety, compatibility, correctness

🔵 **Safety / Compatibility / Correctness**

`MarkState::after(MarkChange::Judged(fidelity))` passes `self.mark()` as the previous mark, and `mark()` returns `None` for `MarkState::Unreadable`. So `PushFidelity::Unjudged` keeps a `Marked` value but turns an `Unreadable` one into `Absent`. The test `rewrite_mark_after_an_unreadable_mark_is_kept_only_by_keep` pins this behaviour. But `Unjudged` means no read-back was taken, so this build has no evidence that the rewrite a newer build recorded has been resolved. This works against the guarantee the unknown-key work exists to give: an older build keeps what a newer one wrote.

**Impact**: If a later release changes the mark's shape, this release erases those marks whenever a push here fails to get a read-back.

**Suggestion**: Treat `Judged(PushFidelity::Unjudged)` as `Keep` and clone `self` whole, so that `Unreadable` survives. Update the pinning test to match.

---

### `cli/work-adapters/src/sync/rewrite_mark.rs:103-120` — A persisted mark drops the sections whose changed lines were not shown
**Severity**: minor | **Confidence**: high | **Lens**: correctness

🔵 **Correctness**

`RewrittenLines::bounded` collects `sections` from every change, and its doc says "every section a changed line falls in, shown or not". `RewriteMark` persists only the first `SHOWN_LINES` lines plus `more`, and `rewritten_lines()` rebuilds `sections` from the stored lines alone. Take a push that rewrote 6 lines, the sixth in `Summary`. Its first warning names `Notes, Summary`. Every later `still rewritten on push` notice, and the `work update --push` relay, names only `Notes`.

**Impact**: This breaks the invariant `RewrittenLines` claims, and the standing notice under-reports which sections still hold Linear's rewrite.

**Suggestion**: Persist `sections` in the mark alongside `lines` and `more`. Derive them from `lines` only when the field is absent.

---

## Per-Lens Results

### Architecture

**Summary**: The PR splits the work cleanly. Linear's dialect rules live in a pure `markdown::linear` module and are applied at the wire boundary in `linear-client` through `Description`. The rewrite-mark transition has one store-level entry point (`BaselineStore::record`). The main weakness is above that entry point: 4 to 5 push sites reassemble the read-back, judge, `MarkChange` and notice steps by hand, and one of them is in the CLI layer. Smaller coupling points: display-bounded text is persisted, the simulator reuses the production tokenizer, and the public `markdown::linear` surface is wider than it needs to be.

**Strengths**:
- Escaping sits behind the tracker port, in `linear-client`, mirroring how ADF conversion stays in `jira-client`.
- `markdown::linear` is a pure core, guarded by pup. `escape_for_push` re-parses through the same `document::markdown` events that body equivalence uses.
- Mark transitions use a closed vocabulary. `replace` is crate-private and `seed` is gated behind a feature.
- Forward compatibility is designed in: unknown keys and `MarkState::Unreadable` survive.
- `PushFidelity` is computed for every tracker.
- Rendering is centralised in `report_records::rewrite_lines`.

**Comments**:
- `cli/work-cli/src/update.rs:265-270` (major, high): push → judge → mark → notice is reassembled at every push site. Inline above.
- `cli/work-cli/src/update.rs:309-326` (minor, high): the rule for choosing a standing notice is duplicated with `noting_standing_rewrites` in `run.rs`. Suggests `Entry::standing_rewrite(..)` plus `on_push(..).or_else(..)`.
- `cli/work-adapters/src/sync/rewrite_mark.rs:72-83` (minor, medium): `SHOWN_LINES`/`LINE_BYTES` shape the persisted mark. Give storage its own cap and apply `bounded` only for display.
- `cli/work-adapters/src/promotion.rs:448-459` (minor, medium): promotion's verdict is held only in memory, so a resumed promotion cannot record the mark. Merged into the inline comment at `promotion.rs:994`.
- `cli/tracker-test-support/src/rewrite.rs:192` (minor, medium): the simulator finds candidates with the production tokenizer. Give it its own scan that over-matches, or make the dependency explicit.
- `cli/markdown/src/linear.rs:16` (suggestion, high): `is_top_level_domain` is public with no external consumer. Make it `pub(crate)`.

**General Findings**:
- (minor) The rule for setting, keeping and clearing the mark is mixed with JSON persistence in `work-adapters`. Move `MarkChange` and the transition into `work::sync`.

### Correctness

**Summary**: The escaping core holds up: `escape_for_push` re-checks every set of escapes against the `document` reading, so a wrong proposal is dropped rather than shipped. The `BaselineStore::record` rules are coherent for a single successful run. The gaps are at the edges of the mark's lifecycle. A retire step that fails loses a promotion's rewrite. A resumed or unwritten create is never judged. A persisted mark loses section names beyond the first 5 lines.

**Strengths**:
- On every path, `escaped_or_original` guarantees that the `document` reading is unchanged.
- Escaping is idempotent: `TextRun::read` flags characters that follow an escape.
- `ChangedLines` can never be empty, because the whole-body fallback always names a line.
- `MarkState::after` is total and explicit: `Fresh`, `Unjudged`, `Preserved`, and `Agreed` for reconciliations.
- `superseding`, `finalise_run`, `rename` and realign all carry unknown keys and the mark through.

**Comments**:
- `cli/work-adapters/src/promotion.rs:994` (major, medium): a promotion's rewrite is lost when retire fails and a later run finishes it. Inline above.
- `cli/work-adapters/src/sync/rewrite_mark.rs:103-120` (minor, high): a persisted mark drops sections that were not shown. Inline above.
- `cli/work-adapters/src/sync/apply.rs:1703-1714` (minor, medium): a create resumed from a pending-push marker is recorded unmarked and never judged, as are `sent_create == false` and `CreatedUnwritten`. Judge with the pending-push request digest when the local content still matches.
- `cli/work-adapters/src/sync/rewrite_mark.rs:229-231` (minor, low): an unjudged push erases an unreadable mark. Merged inline above.

**General Findings**:
- (suggestion) The escaper and the simulator disagree on non-ASCII last labels: the escaper escapes them and the simulator leaves them bare. The difference preserves meaning, but the PR description is inaccurate.

### Test Coverage

**Summary**: The Rust side is very well tested. The escaping rules have property-style assertions and about 100 targeted cases, checked against more than 1,000 live probes. The mark rule is pinned at the unit, store, sync-run, real-client and CLI `run_with` layers. The gaps are at the edges of the stack: no skill eval covers the new relay behaviour, nothing above `run_with` checks stderr, and the preview path's standing notice is untested. A few assertions are vacuous or depend on live repository data.

**Strengths**:
- `assert_escapes` checks each case five ways: expected output, idempotence, insertions only, an unchanged reading, and the same under CRLF.
- The simulator and the oracle are calibrated against recorded Linear behaviour.
- The mark rule is a table at every layer, and the `every_push_entry_point_*` tests cover Jira, Linear and Shortcut.
- Error paths are tested: an unreadable `--body-file`, `--verbatim` with no input, unjudged read-backs, malformed marks, and a rewound retirement.
- The expected escaped descriptions are independent literals.
- The Python feature-confinement lint has a test for each branch.

**Comments**:
- `skills/work/sync-work-items/SKILL.md:375-396` (major, high): no eval covers the new relay behaviour. Inline above.
- `cli/work-cli/src/main.rs:287-289` (major, medium): no binary-level test checks the stderr warnings. Inline above.
- `cli/work-adapters/src/sync/run.rs:2538` (minor, high): no test covers the preview path's standing notice or the `baseline.load()` failure branch.
- `cli/markdown/src/linear/tests.rs:793-816` (minor, medium): the corpus test depends on the live `meta/work` directory, with a floor of 300 files and a "never abandoned" assertion. Snapshot the bodies into fixtures instead.
- `cli/work-cli/src/create.rs:1354-1358` (minor, high): `warnings[1..].iter().all(..)` is vacuous when there are no note lines. The same pattern is in `create_batch_warns_once_per_rewritten_item`.
- `cli/tracker-test-support/src/rewrite.rs:192` (suggestion, medium): the simulator uses the production tokenizer, so it is blind to shapes that tokenizer misses.
- `cli/markdown/src/linear/tests.rs:136-140` (suggestion, low): `assert_escapes` pins the internal propose-then-verify split.

**General Findings**:
- (minor) The `work list` wiring for the mark (`sync_labels`) is untested end to end. Add a `cli_list.rs` case.
- (suggestion) The Linear escape expectation is duplicated across 2 test crates. Use a shared fixture constant.

### Code Quality

**Summary**: The PR is well put together for its size. The domain concepts it adds (`Description`, `PushFidelity`, `ChangedLines`, `MarkChange`, `BodyAgreement`, `SplitEntry`) are named well and carry their invariants in types. The escaping modules are small and cohesive. The risks are local: the same concept is modelled two ways, there is an out-parameter, a key list is kept in sync by hand, accessor chains read badly, and one new field took about 15 edits.

**Strengths**:
- `Description::{Markdown, Verbatim}` lets the type system enforce the escaping choice.
- `ChangedLines` holds its invariant by construction.
- `Splice`/`spliced` removes duplication, and each Linear rule is its own module.
- `SplitEntry` with builder-style transforms removes struct-update boilerplate.
- `BaselineStore::record` with `MarkChange` forces every write to state its intent.
- `MarkChange::after_reconciliation` reads as a domain rule.

**Comments**:
- `cli/work-adapters/src/creation.rs:379` (minor, high): `sent_create: bool` duplicates the `IssueCreated` enum. The agent anchored this to `sync/creation.rs:380`; corrected here.
- `cli/work-adapters/src/promotion.rs:447-459` (minor, medium): the `&mut PushFidelity` out-parameter, plus `&PushFidelity::Unjudged` hard-coded by 2 of the 3 callers of `finish_retirement`.
- `cli/work-adapters/src/sync/baseline.rs:80-92` (minor, high): `KNOWN_KEYS` duplicates the key list in `render`. Derive both from one table.
- `cli/work-adapters/src/sync/run.rs:390-393` (minor, high): `entry.mark().mark()` appears at 3 sites. Rename to `mark_state()`/`readable()` and add `Entry::standing_rewrite()`.
- `cli/work-cli/src/update.rs:309-320` (suggestion, medium): the tuple parameter `(path, external_id)` is a data clump.
- `cli/work-adapters/src/sync/push_fidelity.rs:137-143` (suggestion, high): the section de-duplication loop is duplicated in `RewriteMark::rewritten_lines`.
- `cli/work-adapters/src/sync/rewrite_mark.rs:46-53` (suggestion, medium): `Judged(&Unjudged)` and `Fresh(&Unjudged)` read as contradictions. Rename the variants, or use `Keep`/`Clear` directly.
- `cli/work/src/sync/plan.rs:339-340` (suggestion, medium): the body agreement defaults silently and is attached in only one constructor arm.
- `cli/markdown/src/linear/dotted_tokens.rs:104-146` (suggestion, medium): `in_word` packs a three-stage scanner into one loop. Extract `host_end`/`port_end`.

**General Findings**:
- (minor) Adding `rewrite` to `ReportedItem`/`PromotionRow`/`BatchItem` took about 15 struct-literal edits. Add constructors with defaulted attachments.
- (suggestion) 4 `rewrite_warnings`-style adapters each derive the label and path form differently. Settle on repo-relative paths via `rewrite_lines_under`.

### Performance

**Summary**: The PR adds no tracker round-trips: every verdict reuses the existing `show` read-back, and the TLD list is a hash set built once. The one real cost is the fallback in `escape_for_push`. It re-splices and re-parses the whole body twice per candidate escape, O(k·n), and in this repository k grows with n because names like `*.rs` and `*.md` end in real TLDs.

**Strengths**:
- No extra API calls.
- The TLD lookup is a `LazyLock<HashSet<&'static str>>` built from `include_str!`.
- The fast path costs 3 parses; the quadratic fallback runs only when the reading changes.
- `BaselineStore::record` returns the baseline it wrote, so `work update --push` no longer re-reads it.
- `noting_standing_rewrites` loads the baseline once per run.
- Every hot loop is linear.

**Comments**:
- `cli/markdown/src/linear.rs:54-59` (minor, medium): the fallback is O(k·n). Cache `reading(markdown)`, and consider testing escapes per block or bisecting the escape set.
- `cli/markdown/src/linear.rs:76-78` (suggestion, high): `reads_alike` recomputes the original's reading on every call.
- `cli/work-adapters/src/sync/push_fidelity.rs:231-243` (suggestion, medium): `meaningful_hunks` re-runs `as_pushed` and the parse once per hunk, and `changed_lines` rebuilds `SectionLines`.

**General Findings**:
- (suggestion) Push fidelity repeats parsing the digest step has just done. This is negligible next to HTTP.

### Compatibility

**Summary**: The PR is careful about compatibility. Baseline keys are additive, unknown keys and unreadable marks are carried through, unmarked entries render byte-for-byte as before, and the changes to the public API are recorded. Default escaping is listed as Breaking, with `--verbatim`. 3 risks remain: the mixed-version note understates the hazard, an unjudged push drops an unreadable mark, and keeping unknown keys verbatim relies on an unwritten rule.

**Strengths**:
- `SplitEntry`, `RewriteMark` and `MarkedLine` keep unknown fields.
- Unmarked entries render unchanged, and keys stay alphabetical.
- Builds from before the PR stay able to read entries carrying `rewritten_on_push`.
- Breaking escaping is in the CHANGELOG, with an opt-out. The new flags are additive and the goldens match.
- `E_UPDATE_NO_FILE` (115) is a new code in a free slot, and `E_CREATE_NO_FILE` keeps its code.
- Escapes are no-ops in CommonMark, and drift in the TLD snapshot shows up as a warning.
- Warnings go to stderr only, so stdout and exit codes are unchanged.
- The `test-support` feature is enabled only from dev-dependencies.

**Comments**:
- `CHANGELOG.md:1333-1335` (major, high): the migration note understates what an older release does. Inline above.
- `cli/work-adapters/src/sync/rewrite_mark.rs:229-231` (minor, high): an unjudged push drops an unreadable mark. Merged inline above.
- `cli/work-adapters/src/sync/baseline.rs:141-147` (suggestion, medium): kept unknown keys can go stale against refreshed fingerprints. Write down the rule for recipe-1 keys.

**General Findings**:
- (minor) Existing Linear issues stay corrupted and unmarked until each is pushed again. Make the repair step prominent in Migrations, and consider an audit command.

### Usability

**Summary**: Seen from the user's side, the PR is mostly well judged. Escaping happens without the user doing anything, warnings name a next step, and `--body-file` matches the other trackers. The weak spot is resolving a standing mark: one remedy is printed for every cause, including content Linear cannot store, and nothing dismisses the mark. Smaller points: labels differ between commands, the report request appears even for known limits, and there is no way to find items damaged before the upgrade.

**Strengths**:
- Every warning ends with a concrete next step, and the overflow note names `accelerator <tracker> show <KEY>`.
- The push succeeds and exit codes are unchanged.
- Line breaks show as `⏎`, and stored text is reduced to one line, which prevents injection.
- The standing variant leaves out the notes that would repeat.
- `E_CREATE_NO_FILE` and `E_UPDATE_NO_FILE` name the path, and `--body-file` gives the commands parity.
- The skills never suggest a pull as the remedy.
- `--verbatim` uses clap `requires`.

**Comments**:
- `cli/work-cli/src/report_records.rs:126-130` (major, medium): one remedy for every rewrite, and the mark cannot be dismissed. Inline above.
- `cli/work-cli/src/report_records.rs:131-135` (minor, medium): "please report it" is printed for limits that are known and decided.
- `CHANGELOG.md:1123-1125` (major, medium): no way to find items rewritten before the upgrade. Inline above. The agent anchored this to `CHANGELOG.md:408-415`, which is outside the diff; corrected here.
- `cli/work-cli/src/sync.rs:982-991` (minor, medium): the warning names the item by id, key or path depending on the command.
- `cli/work-adapters/src/sync/push_fidelity.rs:53-60` (minor, low): the whole-body fallback cuts both sides to 200 bytes, with no `show` pointer.
- `skills/integrations/linear/update-linear-issue/SKILL.md:9` (minor, medium): the argument hint advertises `--verbatim`, unlike `/create-linear-issue`.
- `cli/linear-cli/src/cli.rs:58-62` (suggestion, low): consider hiding `--verbatim` or shortening its help.

**General Findings**:
- (minor) The standing notice repeats a snapshot taken at push time. Say "as of the last push".

### Safety

**Summary**: The PR handles silent remote damage carefully. Escaping can only add backslashes, it keeps only escapes that preserve meaning, and any remaining divergence is caught by the read-back check and recorded durably. The main hazard is the one the PR names: older builds rewrite the committed `last-sync.json` wholesale, and no path rebuilds a lost mark. Smaller paths also drop the signal: an unjudged push over an unreadable mark, a failed read-back, and a failed baseline write after a successful push.

**Strengths**:
- `escaped_or_original` fails safe.
- Escaping and detection are separate layers.
- `BaselineStore::record` is the only write path, and `seed` is gated behind a test-only feature.
- The mark is cleared only when there is positive evidence.
- `--verbatim` is opt-in and documented as a capture tool.
- The Phase 12 repair scripts are defensive.
- The `TES` fixtures hold synthetic text only.

**Comments**:
- `cli/work-adapters/src/sync/rewrite_mark.rs:68` (major, high): older builds strip every mark, and nothing guards or rebuilds them. Inline above.
- `cli/work-adapters/src/sync/rewrite_mark.rs:229-231` (minor, medium): an unjudged push drops an unreadable mark. Merged inline above.
- `cli/work-cli/src/update.rs:259-285` (minor, medium): the verdict is lost when the read-back or the baseline write fails. Print a "could not verify" note, and print the notice before the baseline write.
- `meta/plans/2026-11-23-PP-906-escape-linear-bound-markdown-on-push.md:2606-2607` (minor, low): the pre-repair backup lives only in a gitignored tmp directory. Archive it somewhere durable.

**General Findings**:
- (suggestion) The meaning-preservation guarantee is measured against CommonMark, not Linear. Keep the probe-corpus test as a required gate.

---
*Review generated by /accelerator:review-pr*

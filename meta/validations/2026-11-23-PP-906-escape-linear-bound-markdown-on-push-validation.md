---
type: "plan-validation"
id: "2026-11-23-PP-906-escape-linear-bound-markdown-on-push-validation"
title: "Validation Report: Escape Linear-Bound Markdown on Push Implementation Plan"
date: "2026-11-24T17:21:19+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-11-23-PP-906-escape-linear-bound-markdown-on-push"
tags: ["sync", "linear", "markdown"]
last_updated: "2026-11-24T18:27:21+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Escape Linear-Bound Markdown on Push Implementation Plan

The result is `partial` for one reason only: Phase 12 (post-merge
verification) is pending by design. It runs by hand against the live `PP`
team with the released build, after merge and release. Phases 1-11 are
fully implemented, judged against the plan as amended by its "As landed"
notes. The plan's `status` stays `in-progress` until Phase 12 is done.

### Implementation Status

✓ Phase 1: `accelerator linear update --body-file` - Fully implemented
✓ Phase 2: Extended unescaped capture and the TLD snapshot - Fully implemented
✓ Phase 3: Escaped capture - Fully implemented
✓ Phase 4: The digit rule - Fully implemented
✓ Phase 5: The dotted-token rule - Fully implemented
✓ Phase 6: Escaping every Linear description - Fully implemented
✓ Phase 7: Noticing a push the tracker rewrote - Fully implemented (as landed)
✓ Phase 8: Remembering a rewritten push - Fully implemented (as landed)
✓ Phase 9: The simulator reads tokens as the escaper does - Fully implemented (as landed)
✓ Phase 10: Escaping Linear comments - Gate-only. Sections 3-5 were dropped by
  the recorded gate outcome: Linear stores comments verbatim.
✓ Phase 11: Whether decoded entities change meaning - Fully implemented (as
  landed). No work item was raised, which the user decided.
⏳ Phase 12: Post-merge verification - Pending by design (after release)

### Automated Verification Results

✓ `mise run`, the full default task, exits 0 at `@-` = `spyyxmkl`
  (`7941308a`). It took 258 s.
  - The `cli` unit tests ran 8616 tests: 8616 passed and 2 were skipped.
  - The frontend unit tests ran 126 files and 2697 tests, all passing.
  - The visualiser e2e lanes passed.
  - The working copy was clean afterwards, so the kanban fixture flake did not
    occur.

The full default run includes every per-phase command:

- the `test:unit:cli` filters
- `cli:check`
- `public-api:check`
- `pup:check`
- `test:integration:deny`
- `golden_digests`
- the `test:unit:tasks` skill-structure test

### Code Review Findings

#### Matches Plan:

- **Phase 1**
  - `description_source` ArgGroup: `cli/linear-cli/src/cli.rs:67-79`.
  - `read_body_file` and `requested_description`:
    `cli/linear-cli/src/main.rs:342-384`.
  - `UPDATE_NO_FILE = 115`: `cli/linear-cli/src/exit_codes.rs:94`. It is not
    in the captured codes file.
  - Golden, skill and CHANGELOG are updated, and all named tests are present.
- **Phases 2-3**
  - Fragments `X10`-`X25`, the probe corpus and the README tables are in
    place.
  - The IANA snapshot is at `cli/markdown/src/linear/tlds-alpha-by-domain.txt`.
  - `escaped/` holds the capture (`TES-37`) and its web-UI check.
  - The equivalence exclusions and the escaped-capture tests are in
    `cli/corpus-adapters/tests/body_equivalence.rs`.
- **Phases 4-5**
  - `escape_for_push`, `proposed_escapes` and the fallback:
    `cli/markdown/src/linear.rs:24-93`.
  - `digit_rule.rs` and the dotted-token modules (`text_runs.rs`,
    `scheme_urls.rs` and `dotted_tokens.rs`) are in place.
  - `assert_escapes` checks idempotence, insertions only and an independent
    reading oracle, over both LF and CRLF.
  - A repository sweep runs over every work item body.
  - The capture and probe oracle tests are in
    `cli/work-adapters/tests/linear_captures.rs`.
- **Phase 6**
  - `Description::{Markdown, Verbatim}` and its `wire()`:
    `cli/linear-client/src/description.rs`. It is used at `client.rs:688` and
    `update.rs:39`.
  - `RemoteTracker` sends `Markdown`: `client.rs:899`, `:913`.
  - `--verbatim` is bound to its required groups.
  - The push matrix `every_push_entry_point_escapes_for_linear_alone` is in
    place, as are the `work-cli` push tests.
  - The CHANGELOG has its Breaking and Fixed entries.
- **Phase 7**
  - `PushFidelity`, `ChangedLine` and the bounding (5 lines, 200 bytes) are in
    `cli/work-adapters/src/sync/push_fidelity.rs`.
  - The push sites match the plan's table. A retitle is not judged.
  - The renderer `rewrite_lines` / `rewrite_lines_under` is in
    `cli/work-cli/src/report_records.rs:85-139`.
  - The skill relays and the skill-structure test entry are in place.
- **Phase 8**
  - `RewriteMark` and `MarkChange` are in
    `cli/work-adapters/src/sync/rewrite_mark.rs`, and the key is
    `rewritten_on_push`.
  - Agreement-gated clearing is in place, and `seed` sits behind
    `test-support`.
  - `work list` shows `· rewritten on push` (`cli/work-cli/src/list.rs:544`).
  - Standing-mark notices appear in both sync apply and preview.
- **Phase 9**
  - `linear_leaves_bare` is in
    `cli/tracker-test-support/src/linear_autolinker.rs:16`. The oracle and the
    simulator share it, and no copy of `is_recorded_over_escape` remains.
  - `autolink_dotted_tokens` uses `markdown::linear::dotted_tokens`.
  - `X11`, `X14` and `X16`-`X18` are added to `recorded_rewrites()`.
  - The `pup.ron` rule permits `document::markdown`.
- **Phase 10**
  - `comment-read-back.json` is committed.
  - The README's "Comments" section records the gate.
  - The extended allowlist is recorded in `work-item-sync-recovery/README.md`.
- **Phase 11**
  - `entities.md`, `entities.read-back.json` and
    `entities.second-push.read-back.json` are committed.
  - `ENTITY_PROBES_THAT_CHANGE_MEANING`, `each_entity_probe_reads_as_recorded`
    and `each_pulled_entity_probe_reads_as_recorded_after_a_second_push` are
    in place.
  - The README's "Entities" and "Decision" sections are written.

#### Deviations from Plan:

None of substance beyond the "As landed" notes. The trivial ones:

- `read_body_file` takes `&BodyFileRefusal` rather than taking it by value
  (`cli/linear-cli/src/main.rs:342-384`).
- `push_fidelity` reaches the plan's result via `local_body` and
  `equivalence_to_rendered` rather than `equivalence`. The behaviour is the
  same (`cli/work-adapters/src/sync/push_fidelity.rs:136-163`).
- `reads_alike`'s "link destination" case is tested with an angle-bracket
  autolink rather than an inline link.
- `skills/integrations/linear/update-linear-issue/SKILL.md:50` reads
  awkwardly: "A `--description` or `--body-file` file that is empty".

#### Potential Issues:

Each issue below was fixed after this validation, test-first, and recorded
in the plan's "Validation follow-ups":

1. **A fresh notice can be lost.** Fixed: `BaselineStore::record` returns
   the baseline it wrote, and `work update --push` builds its notice from
   it, with no reload.
2. **A rewrite verdict can carry no lines.** It was reachable: sections
   stored in another order. Fixed: `Rewritten` carries `ChangedLines`, which
   cannot be empty, and such a rewrite is reported as one `(whole body)`
   line. A hunk that only moves line boundaries, such as merged paragraphs,
   is widened over the lines it joins, and a notice shows each line break
   as `⏎`.
3. **Tokens split by inline markup are never seen.** Probed live as
   `TES-41`, since trashed: Linear links none of them, so the escaper
   already matched. A token before a code span is left bare, which
   `linear_leaves_bare` now records.
4. **NFD-encoded labels split.** Fixed: Linear reads combining marks and
   format characters as label characters, and so does the escaper.
5. **The fallback is all-or-nothing and silent.** Fixed: only an escape that
   would change the reading is withheld, the rest are sent. A withheld
   escape is reported only through the rewrite it may cause, by phases 7
   and 8, as Toby Clemson decided.
6. **The two link-depth counters disagree.** Fixed: one `LinkNesting`
   serves both.

The awkward wording in `/update-linear-issue` is fixed too.

### Manual Testing Required:

1. Phase 12, after merge and release, against the live `PP` team:
   - [ ] Audit and repair every affected item with
     `accelerator work update <path> --push`. Each one's description holds
     neither rewrite, its file is unchanged, and the next sync reports it
     `synced`.
   - [ ] Hand-check PP-87's and PP-22's remote tables against their local
     files.
   - [ ] In Linear's web UI, PP-180, PP-733, PP-860, PP-870 and each repaired
     item show no stray list, no link on a dotted token and no backslash
     before a `.` or `)`.
   - [ ] The PR lists every item checked and each one found.
   - [ ] The re-audit follow-up is raised.

### Recommendations:

- Run Phase 12 after release, then re-validate or mark the plan `done`.
- The follow-ups are done; see Potential Issues.
- On PP-906, every acceptance criterion except the two under "Post-merge
  verification" is now ticked. The fallback criterion was reworded to the
  per-escape rule. The comment criterion is met by the
  recorded gate outcome; its "if they are" clause did not apply. The entity
  criterion is met by the README's decision, and no work item was called for,
  per the user's decision.

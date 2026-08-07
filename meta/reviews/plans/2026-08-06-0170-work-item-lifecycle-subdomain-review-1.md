---
type: plan-review
id: "2026-08-06-0170-work-item-lifecycle-subdomain-review-1"
title: "Plan Review: Work-Item Lifecycle Subdomain Implementation Plan"
date: "2026-08-06T09:51:24+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-08-06-0170-work-item-lifecycle-subdomain"
reviewer: Toby Clemson
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, compatibility, safety]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-07T13:12:10+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Work-Item Lifecycle Subdomain Implementation Plan

**Verdict:** REVISE

This is a carefully researched plan — its Key Discoveries section repeatedly corrects the source work item's own premises against verified line references in the actual bash scripts, and its phased, characterization-first structure is a genuinely safe rollout shape. But close, source-verified reading by every lens surfaces the same class of problem repeatedly: the plan's own scope inventory of "what gets ported" and "what gets deleted" doesn't fully agree with itself. Four independent lenses converge on cases where a script is characterized (or a skill dependency is identified) but never actually gets a Rust port before Phase 9 deletes its source, and three lenses independently flag that `document::render`'s whole-tree rewrite contradicts the plan's own "byte-identical"/"single-line diff" verification claims. These are not stylistic nitpicks — several would break real skill behaviour (`sync-work-items`' overwrite-safety guard, `update-work-item`/`list-work-items`' parent-canonicalisation) the moment Phase 9 lands.

### Cross-Cutting Themes

- **`work-item-project-remote.sh` is characterized and deleted but never ported to Rust** (flagged by: architecture, correctness, test-coverage) — Phase 1 captures its golden, Key Discoveries argues at length that it's genuinely in scope, and Phase 9 deletes the script — but no phase between 2 and 8 ever adds a `cli/work/src/project_remote.rs` module. Contrast with `file_dirty`, which gets exactly this treatment in Phase 8 for the identical "future 0194 consumer" rationale. Correctness rates this critical because `sync-work-items` still calls the script after Phase 9 deletes it.
- **`document::render`'s whole-tree rewrite contradicts the plan's own byte-identical claims** (flagged by: architecture, compatibility, safety) — Key Discoveries explicitly documents that `document::render` gives content-preservation, not byte-identical formatting (that's *why* the plan rejects extending `patch_status`). Yet Phase 8's Automated Verification asserts `work update` leaves "every other field byte-identical," and Phase 8's Manual Verification expects `jj diff`/`git diff` to show "a single-line frontmatter change." Both claims are stronger than the documented design supports — a comment, non-default quote style, CRLF ending, or flow-vs-block array style on an *untouched* key can legitimately change on every write.
- **A skill-level dependency has no Rust port and lives in a file scheduled for wholesale deletion** (flagged by: usability, test-coverage) — `wip_canonicalise_id` (inside `work-item-common.sh`) is invoked directly by `update-work-item` and `list-work-items` for parent-field canonicalisation. Phase 9 deletes `work-item-common.sh` as having "no other consumer once [the 11 scripts] are gone," but this function's two skill-level callers aren't among those 11 scripts, and no phase ports the function anywhere. Usability rates this critical.
- **Phase 9's script deletions leave `sync-work-items`' safety guards stranded with no replacement** (flagged by: safety, usability) — the plan already acknowledges `sync-work-items`' calls to `project-remote`/`file-dirty`/`normalise` "cannot be repointed to a CLI surface" and defers the fix to 0194 as a documented gap. Safety traces the concrete consequence: once `work-item-file-dirty.sh` no longer exists, the skill's `if work-item-file-dirty.sh <path>; then DIRTY=1; else DIRTY=0; fi` guard fails closed into `DIRTY=0` (command-not-found reads as false), silently flipping a documented fail-safe check into fail-dangerous for a workflow that overwrites local files from remote data.
- **The plan claims to mirror `vcs`'s three-crate hexagon but collapses two crates into one** (flagged by: architecture, standards) — no `work-adapters` crate is ever created; the `DirectoryLister` filesystem implementation and the `diff -u` subprocess shellout both live directly in the `work-cli` binary crate, unlike `vcs-adapters`, `corpus-adapters`, and `config-adapters`.
- **`work create`'s allocate-then-write sequence has no collision protection** (flagged by: correctness, safety) — the scan-then-write pattern is unlocked, so two concurrent invocations can allocate the same ID, and `store::atomic_write` has no exists-check before its rename, unlike the collision-abort behaviour `sync-work-items` already requires of its own create bridge for the same underlying primitive.

### Tradeoff Analysis

- **Exit-code/error-message rigor is uneven across the five commands**: `resolve` (Phase 4) and `show` (Phase 5) get an explicit, tested exit-code and error-phrasing mapping to the bash originals; `diff`, `create`, and `update` do not (compatibility, major). Code Quality separately notes there's no stated *mechanism* (a shared error type, a `kernel::Error` extension) that would make this consistency easy to enforce across phases implemented at different times — these are the same underlying gap seen from two lenses; fixing the mechanism in Phase 4 would likely resolve both.
- **Domain-crate discipline vs. delivery pragmatism for `create`/`update`**: every other command (`resolve`, `show`, `diff`) keeps its decision logic in the pure `cli/work/` domain crate, unit-tested with doubles; `create` and `update` — the two commands the plan itself calls "the most complex" — push their orchestration logic into the binary crate instead, since no bash script exists to characterize them against. This is an understandable consequence of there being no golden to port from, but it means the newest, highest-risk logic gets the least architectural consistency and the weakest test isolation. Worth a deliberate call: either extract the pure parts into `cli/work/`, or explicitly accept the inconsistency as a scoped trade-off.

### Findings

#### Critical

- 🔴 **Correctness / Architecture / Test Coverage**: `work-item-project-remote.sh` is characterized and its script deleted, but no Rust port is ever implemented
  **Location**: Phase 1 (golden capture); Phase 3 (domain modules); Phase 9 (deletion)
  Confirmed in scope by Key Discoveries, golden-captured in Phase 1, deleted in Phase 9 — but no `cli/work/src/project_remote.rs` (or equivalent) is ever added in Phases 2-8, unlike the identical-rationale `file_dirty` module Phase 8 does add. `sync-work-items` still calls the script after deletion.

- 🔴 **Usability / Test Coverage**: `wip_canonicalise_id` has no ported Rust equivalent, yet two skills call it directly
  **Location**: Phase 3, item 8 (Own-identity predicate and field alias); Phase 9, item 1 (Remaining skill repoints)
  `update-work-item/SKILL.md` and `list-work-items/SKILL.md` both shell out to `work-item-common.sh:wip_canonicalise_id` for parent-field canonicalisation. `work-item-common.sh` is deleted wholesale in Phase 9 with no port of this function anywhere in the plan — both skills' parent-handling breaks once the file is gone.

- 🔴 **Test Coverage**: Phase 9's test-suite deletion line-range contradicts the file's actual interleaved section structure
  **Location**: Phase 9, item 2 (Script and test-suite deletion)
  The plan instructs deleting `test-work-item-scripts.sh:33-1661` as covering "the 10 lifecycle-side scripts," but the file's sync-side sections (`sync-label`, `sync-baseline`, `sync-classify`, `sync-decide`) are interleaved *inside* that range, while the `work-item-section-diff.sh` test section falls *outside* it. Following the instruction literally either destroys sync-side coverage 0194 needs, or leaves a dangling invocation of a just-deleted script.

- 🔴 **Safety**: Deleting `work-item-file-dirty.sh` strands `sync-work-items`' overwrite guard, likely flipping it fail-open
  **Location**: Phase 9, item 1 (Remaining skill repoints); item 2 (Script and test-suite deletion)
  The skill's `if work-item-file-dirty.sh <path>; then DIRTY=1; else DIRTY=0; fi` guard — documented by the script itself as "FAIL SAFE to DIRTY" — fails closed to `DIRTY=0` once the script no longer exists (command-not-found reads as false in the `if`), silently disabling the protection against overwriting uncommitted local edits during a remote pull.

#### Major

- 🟡 **Architecture / Compatibility / Safety**: `document::render`'s whole-tree rewrite contradicts the plan's own byte-identical/single-line-diff verification claims
  **Location**: Phase 8 Automated/Manual Verification; Key Discoveries (document::render paragraph)
  Key Discoveries documents `document::render` as content-preserving, not byte-identical. Phase 8's success criteria nonetheless assert byte-identical untouched fields and a single-line `jj diff`, which the design doesn't guarantee for comments, quote style, or array flow-vs-block formatting.

- 🟡 **Standards / Architecture**: No `work-adapters` crate despite the plan claiming to mirror `vcs`'s three-crate split
  **Location**: Phase 4, item 1; Phase 6, item 1; References
  The `DirectoryLister` filesystem adapter and the `diff -u` subprocess shellout both live directly in `work-cli`, unlike the dedicated `*-adapters` crates every other hexagon in the codebase uses.

- 🟡 **Correctness / Safety**: `work create`'s allocate-then-write sequence has no collision or lock protection
  **Location**: Phase 7, item 2 (Creation flow)
  The scan-then-write pattern is unlocked and `store::atomic_write` has no exists-check, so concurrent invocations can silently produce duplicate IDs or clobber a just-created file — unlike the collision-abort behaviour `sync-work-items` already requires of its own equivalent primitive.

- 🟡 **Usability**: `sync-work-items`' stranded dirty-check guard has no defined interim behaviour
  **Location**: Phase 9, item 1 (sync-work-items note)
  The plan acknowledges the gap but doesn't specify what should actually happen between 0170 landing and 0194 resolving it — see the safety finding above for the concrete failure mode this produces.

- 🟡 **Usability**: `extract-work-items.md` is missing from Phase 9's explicit repoint inventory
  **Location**: Phase 9, item 1 (Remaining skill repoints)
  It invokes `work-item-next-number.sh` at least eight times, including batch allocation (`--count N`) that Phase 7's `Command::Create` signature doesn't currently support (one ID per invocation).

- 🟡 **Usability**: `allowed-tools` frontmatter is only updated for `update-work-item`; other repointed skills can't invoke the new binary without a permission prompt
  **Location**: Phase 8, item 3; Phase 9, item 1
  `list-work-items`, `sync-work-items`, `create-work-item`, `refine-work-item`, and `review-work-item` are all repointed to `accelerator work <verb>` calls without a corresponding `allowed-tools` update, unlike the explicit update Phase 4 makes for `update-work-item`.

- 🟡 **Code Quality**: `create`/`update` orchestration logic lives in the binary crate, breaking the plan's own domain/adapter split
  **Location**: Phase 7; Phase 8, item 2
  Unlike `resolve`/`show`/`diff`, which keep decision logic in the pure `cli/work/` crate tested with doubles, `create`'s field-omission rules and `update`'s `--set`/id-validation logic are described as living directly in `cli/work-cli/`.

- 🟡 **Code Quality**: `create`'s frontmatter field list hand-duplicates `templates/work-item.md`'s schema with no drift check
  **Location**: Phase 7, item 2 (Creation flow)
  The field list is manually re-encoded in `create.rs` rather than derived from or checked against the template, so a future template edit can silently diverge from what `create` writes.

- 🟡 **Correctness**: `AllocationError::Overflow` cannot reconstruct the bash's two distinct overflow messages
  **Location**: Phase 3, item 3 (Next-number allocation)
  The variant carries only `partial`/`highest_file`, not the numeric highest value or cap the bash's two different message wordings need to interpolate.

- 🟡 **Correctness**: `DirectoryLister` has no way to service the `Path`-classified branch of `resolve`
  **Location**: Phase 3, item 2; Phase 4, item 1
  The bash's `path` classification does a real filesystem existence check and canonicalisation on arbitrary paths, which the `filenames() -> Vec<String>` trait can't express.

- 🟡 **Correctness**: `TagError::BlockStyleTags` is attached to a function that cannot detect the condition it names
  **Location**: Phase 3, item 5 (Tag-array mutation)
  `mutate_tags`'s stated inputs (the already-extracted tags value) can't observe the raw multi-line block-style shape that `is_block_style` is separately built to detect.

- 🟡 **Correctness**: The bash tag parser's naive comma-split isn't identified as a quirk to preserve or fix
  **Location**: Phase 3, item 5 (Tag-array mutation)
  `work-item-update-tags.sh` splits on every comma without respecting quoting, so a tag like `"c,d"` written correctly would be mis-split on re-read — the plan's "reproduces the parse exactly" claim doesn't address whether this asymmetry is deliberately preserved.

- 🟡 **Test Coverage**: Several Phase 1 goldens omit the AC-required error-path row
  **Location**: Phase 1 (golden fixture descriptions)
  `section-diff`, `normalise`, `project-remote`, and `template-field-hints`'s described golden rows all omit at least one error path the source AC requires each of the 10 scripts to cover.

- 🟡 **Compatibility**: Exit-code contract unspecified for `diff`/`create`/`update`
  **Location**: Phase 6, 7, 8
  Unlike `resolve` (Phase 4) and `show` (Phase 5), no phase maps these three commands' failure modes to a documented, tested exit-code contract that skills and 0194 can code against.

- 🟡 **Compatibility**: The invented `--set` flag and CLI signatures are 0194's blocked-dependent contract, but nothing freezes them
  **Location**: Phase 8, item 1; cross-referenced against 0194's Assumptions
  0194 explicitly assumes these signatures are stable once implemented, but the plan has no CLI-surface snapshot test or equivalent mechanism to catch accidental drift before 0194 starts depending on them.

#### Minor

- 🔵 **Code Quality**: No unified error/exit-code strategy stated across the five `work-cli` commands (related to the Compatibility exit-code finding above — same underlying gap, viewed from the mechanism side)
- 🔵 **Code Quality**: `update`'s raw-text-scan-before-parse ordering constraint is enforced only by prose, not by the type system
- 🔵 **Compatibility**: The `work` crate's own function signatures are also part of 0194's dependency surface but aren't flagged as frozen alongside the CLI contract
- 🔵 **Compatibility**: Locale-independent (ASCII-only) whitespace semantics for the `trim`/`filter` port aren't called out, risking divergence from the bash's `LANG=C` guarantee on non-ASCII input
- 🔵 **Correctness**: `TaggedCandidate` is applied uniformly to `full_id` and `bare_number` ambiguity, but bash only tags `bare_number` matches — worth confirming only exit-code parity (not format parity) is actually required
- 🔵 **Standards**: Checklist point 11 (user-facing documentation) is never addressed, though the plan's own manual-verification steps show direct human invocation
- 🔵 **Test Coverage**: New content-heavy goldens (section-diff, project-remote, template-field-hints) may not fit the existing bare-filename `<setup>` reader without a new convention
- 🔵 **Test Coverage**: Phase 8's "not called from `file_dirty`" guarantee is tested via source grep rather than behaviourally
- 🔵 **Safety**: `work update`'s read-then-write has no lock, permitting a silent lost-update race between concurrent invocations (pre-existing gap, not a new regression, but now centralised onto a faster primitive)

#### Suggestions

- 🔵 **Architecture**: No stated behaviour for a missing `diff` binary at runtime in `work diff`
- 🔵 **Code Quality**: `is_work_item_file` has no stated consumer in any later phase, unlike the plan's explicit justification for other ported-but-unused functions
- 🔵 **Correctness**: `pattern_max_number`'s `10^N` computation isn't guarded against `u64` overflow for pathological pattern widths
- 🔵 **Standards**: No CHANGELOG entry planned for the new `accelerator work` CLI surface (though the `vcs` precedent has the same gap)
- 🔵 **Standards**: `InvalidInput` breaks the `<Verb>Error` naming pattern the plan uses everywhere else (`PatternError`, `AllocationError`, `TagError`)
- 🔵 **Usability**: No mention of `--help`/doc-comment coverage for the CLI's direct human consumers
- 🔵 **Usability**: The `id`-immutability error message risks drifting between the skill's pre-flight check and the CLI's own hard-block if both are kept
- 🔵 **Safety**: Consider a one-off full real-corpus parity sweep (bash vs. Rust output on every file in `meta/work/`) as a final gate immediately before Phase 9's irreversible deletion

### Strengths

- ✅ Key Discoveries repeatedly cross-checks the source work item's own premises against verified line/grep evidence in the real repository rather than trusting prior prose — correcting the AC's "no test suite" claim, the "no CLI subcommand" claim for `template-field-hints`, and the research document's `project-remote` scoping call.
- ✅ Phase 1's characterization-goldens-first approach pins every script's current behaviour before any deletion, with an explicit cross-check-against-real-invocation step to guard against transcription drift.
- ✅ The domain crate's injected-port design (`DirectoryLister`, `IdScanner`) keeps `resolve`/`next_number`/`section_diff`/`tags` pure and testable with hand-rolled doubles, mirroring `vcs`'s existing hexagon and enforced by a new `pup.ron` rule.
- ✅ Several deliberate simplifications are explicitly documented with rationale and backed by characterization tests rather than silently introduced — the hash-free section comparison, the rejection of a generalised multi-key patcher, and the deliberately-preserved un-stripped-`IGNORE_KEYS` quirk in `work diff`.
- ✅ `work update`'s single atomic whole-file write is a genuine safety improvement over today's multi-step `Edit`-tool mutation, even setting aside the formatting-preservation concern raised above.
- ✅ Exit-code and error-message parity is handled rigorously for `resolve` and `show`, with adapter-boundary tests against real temp directories, not just domain-level doubles.
- ✅ The nine-phase, independently-mergeable vertical-slice structure defers the irreversible deletion step until every replacement command has layered test coverage, keeping the bash originals as a live fallback throughout.

### Recommended Changes

1. **Port `work-item-project-remote.sh` to a `cli/work/src/project_remote.rs` module** (addresses: the project-remote cross-cutting theme) — add it alongside `file_dirty` in Phase 3 or 8, characterization-tested against the Phase 1 golden, before Phase 9 deletes the bash original.

2. **Port `wip_canonicalise_id` and repoint its two skill callers** (addresses: the wip_canonicalise_id critical findings) — add it to the `work` domain crate (it composes naturally with Phase 2's `parse_full_id`), expose it via a CLI surface if needed, and update `update-work-item`/`list-work-items` in Phase 9's repoint list.

3. **Replace Phase 9's single line-range deletion instruction with an explicit list of non-contiguous section boundaries** (addresses: the test-suite deletion critical finding) — verify each `=== ... ===` marker against the real file at implementation time rather than trusting a single span.

4. **Decide and state the interim fate of `sync-work-items`' safety guards before deleting their backing scripts** (addresses: the fail-open critical finding) — either keep `work-item-file-dirty.sh`/`project-remote.sh`/`normalise.sh` undeleted until 0194 rewires the skill, or land a minimal CLI shim so the guard has a working target; don't delete the script while the skill's invocation of it stays in place.

5. **Reconcile Phase 8's success criteria with the documented `document::render` design** (addresses: the byte-identical cross-cutting theme) — either loosen the criteria to assert value-identity per unchanged key rather than byte-identity, or add a round-trip test exercising comments/CRLF/flow-style arrays so the actual behaviour is pinned rather than contradicted by an untested claim.

6. **Add a collision check to `work create`'s write path** (addresses: the allocate-then-write finding) — verify the target path is still absent immediately before the write and fail rather than overwrite, mirroring `sync-work-items`' existing collision-abort requirement for the same primitive.

7. **Add explicit exit-code tables for `work diff`/`create`/`update`** (addresses: the exit-code contract finding) — mirror Phase 4's `resolve` mapping, and consider stating in Phase 4 whether `work-cli` extends `kernel::Error` or defines its own exit-code enum so later phases have one mechanism to converge on.

8. **Add `extract-work-items.md` to Phase 9's repoint list and confirm `--count N` batch-allocation support**, and **add `allowed-tools` updates to every phase that repoints a skill** (addresses: the two usability major findings) — add `mise run lint:skills:check` to Phase 9's success criteria so a missed permission update fails CI rather than surfacing as a runtime prompt.

9. **Either introduce a `work-adapters` crate or explicitly justify its absence** (addresses: the missing-adapters-crate finding) — state the reasoning if the two-crate collapse is a deliberate, scoped simplification rather than an oversight.

## Per-Lens Results

### Architecture

**Summary**: The plan is unusually well-grounded architecturally: it explicitly targets the existing domain/adapter/binary hexagon (mirroring `vcs`/`vcs-cli`), correctly restricts the new `work` domain crate to `std`/`kernel::Error`/`corpus`/`crate` via a cargo-pup rule, and pushes frontmatter parsing/rendering and filesystem/subprocess I/O to the CLI boundary, preserving a clean functional core. Two structural gaps weaken it: `work-item-project-remote.sh` is characterized but never actually ported into any Rust module across Phases 2-8 despite being confirmed in-scope, and the switch from `patch_status`'s line-surgical single-key rewrite to `document::render`'s whole-frontmatter-tree rewrite for `work update` is an unvalidated formatting-regression risk the plan doesn't test for. A third, lower-severity concern is that the plan collapses the adapters layer into the binary crate (`work-cli`) rather than creating a `work-adapters` crate paralleling `vcs-adapters`/`corpus-adapters`/`config-adapters`, despite claiming to mirror that precedent exactly.

**Strengths**:
- The domain/adapter/binary split is faithfully applied where it matters most: the `work` crate takes no dependency on `document`, `std::fs`, or regex — `DirectoryLister` and `IdScanner` are injected ports, and frontmatter extraction is pushed to the adapter boundary, preserving a genuinely testable functional core.
- The plan grounds every architectural claim in verified source locations (file:line references, `pup.ron`, `tasks/README.md`'s registration checklist, `vcs`/`vcs-adapters`/`vcs-cli`) rather than assumed structure, and actively corrects the source work item's own premises against real repository state.
- Phase 4's registration work correctly identifies exactly the checklist points (1, 2, 3, 4, 7, 8) that `tasks/README.md`'s thirteen-point checklist requires to land together.

**Findings**:
- 🔴 (major/high) `work-item-project-remote.sh` is characterized and deleted but never actually ported to Rust — Phase 1/Phase 9.
- 🟡 (major/medium) Whole-tree `document::render` rewrite risks reformatting untouched fields on every write — Phase 7/Phase 8.
- 🔵 (minor/medium) No `work-adapters` crate, despite the plan claiming to mirror `vcs`'s three-crate split — Phase 4/Phase 6.
- 🔵 (suggestion/low) No stated behaviour for a missing `diff` binary at runtime — Phase 6.

### Code Quality

**Summary**: The plan is unusually disciplined for a legacy-port story: it establishes pure, injectable-port domain modules mirroring the vcs/vcs-cli precedent, pins characterization goldens before any deletion, and is explicit about rejected alternatives and their rationale. The main risk is that `create` (Phase 7) and `update` (Phase 8) — the two newest, most complex commands, with no bash script to port from — have their orchestration logic living directly in the binary crate rather than the pure domain crate the rest of the plan uses, breaking with the architecture the plan itself establishes.

**Strengths**:
- Phase 3's domain modules consistently take injected ports and are designed to be tested with hand-rolled doubles rather than a filesystem, with a new `pup.ron` rule enforcing the dependency direction.
- Phase 1's characterization-goldens-first approach gives every later phase a concrete, already-verified behavioural oracle.
- The plan is transparent about deliberate simplifications and rejected alternatives with stated rationale.

**Findings**:
- 🟡 (major/high) `create`/`update` orchestration logic lives in the binary crate, breaking the plan's own domain/adapter split — Phase 7/Phase 8.
- 🟡 (major/medium) `create`'s frontmatter field list hand-duplicates `templates/work-item.md`'s schema with no drift check — Phase 7.
- 🔵 (minor/medium) No stated unified error/exit-code strategy across the five `work-cli` commands — Phase 4-8.
- 🔵 (minor/medium) `update`'s raw-text/parsed-tree ordering constraint is enforced only by prose — Phase 8.
- 🔵 (suggestion/low) `is_work_item_file` has no stated consumer in Phases 4-9 — Phase 3.

### Test Coverage

**Summary**: The plan's core testing strategy is sound — table-driven unit tests against hand-rolled doubles for the domain crate, CLI-boundary tests against real temp directories for the binary crate, and reuse of already-verified bash assertions as goldens. However, verification against the actual `test-work-item-scripts.sh` file and two scripts surfaces concrete gaps: a self-contradictory test-suite deletion instruction for Phase 9, an in-scope script with a captured golden but no planned Rust port, a library function with real skill-level callers that gets neither a golden nor a port, and several Phase 1 goldens that omit the AC-mandated error-path row.

**Strengths**:
- Phase 1 explicitly reuses `test-work-item-scripts.sh`'s and `test-work-item-pattern.sh`'s already-verified assertions as the goldens' source of truth, with a stated cross-check step.
- Domain-level tests are explicitly mutation-testing-aware: exact candidate ordering, exact partial-emission-then-error overflow behaviour, and the un-stripped `IGNORE_KEYS` quirk are all asserted precisely.
- Clean test pyramid mirroring the established `vcs-cli` convention rather than inventing a new pattern.

**Findings**:
- 🔴 (critical/high) Stated test-suite deletion range contradicts the file's actual interleaved section structure — Phase 9.
- 🟡 (major/high) `work-item-project-remote.sh` gets a characterization golden but no planned Rust port — Phase 1/Phase 3.
- 🟡 (major/medium) `wip_canonicalise_id` has no golden and no planned port despite real skill callers — Phase 1/Phase 9.
- 🟡 (major/high) Several Phase 1 goldens omit the AC-required error-path row for their script — Phase 1.
- 🔵 (minor/medium) New goldens may not fit the reused `<setup>|<args>|<expected>` reader's actual setup encoding — Phase 1.
- 🔵 (minor/medium) "Not called from `file_dirty`" guarantee is tested by grepping source rather than exercising behaviour — Phase 8.

### Correctness

**Summary**: The plan is unusually rigorous about verifying bash source against its own claims, and it explicitly preserves two subtle bash quirks with characterization tests asserting they survive the port. However, close reading of the actual scripts against the proposed Rust interfaces surfaces several real gaps: a domain-scope inconsistency where `work-item-project-remote.sh` is argued into scope and its script deleted but never actually ported; an error type that cannot carry enough data to reproduce two distinct bash messages; a tag-mutation interface whose block-style detection cannot actually run given its stated inputs; and a `DirectoryLister` trait with no way to service the Path-classified branch of `resolve`.

**Strengths**:
- Key Discoveries rigorously cross-checks the work item's own premises against actual source with concrete line/grep evidence.
- The plan explicitly identifies and preserves the un-stripped-IGNORE_KEYS quirk in `--stdin` normalisation, backing the claim with an exact line reference that matches the real source.
- Phase 8 includes an explicit negative test guarding against a future accidental wiring of the dirtiness guard into `update`'s control flow.
- The overflow guard's partial-emission-then-error behaviour is called out by name with the correct line range and pinned as an explicit success criterion.

**Findings**:
- 🔴 (critical/high) `work-item-project-remote.sh` is deleted with no Rust port ever implemented — Phase 3/Phase 9.
- 🟡 (major/high) `AllocationError::Overflow` cannot reconstruct the bash's two distinct overflow messages — Phase 3, item 3.
- 🟡 (major/high) `DirectoryLister` has no way to service the Path-classified branch of `resolve` — Phase 3, item 2/Phase 4.
- 🟡 (major/medium) `TagError::BlockStyleTags` is attached to a function that cannot detect the condition it names — Phase 3, item 5.
- 🟡 (major/medium) The bash tag parser's naive comma-split is not identified as a quirk to preserve or fix — Phase 3, item 5.
- 🟡 (major/medium) No allocation lock covers the next-number scan-then-write sequence — Phase 7.
- 🔵 (minor/medium) `TaggedCandidate` is applied uniformly to `full_id` and `bare_number` ambiguity, but bash only tags `bare_number` matches — Phase 3, item 2.
- 🔵 (suggestion/low) `pattern_max_number`'s `10^N` computation is not guarded against `u64` overflow — Phase 2, item 2.

### Standards

**Summary**: The plan is disciplined about the mechanics it does address — it correctly walks through the sub-binary registration checklist's same-change points, reuses the exact existing skill-binding shape, and matches golden-fixture and pup.ron naming conventions already established. The one significant convention gap is structural: the plan explicitly claims to mirror the `vcs`/`vcs-adapters`/`vcs-cli` three-crate hexagon, but no phase ever creates a `work-adapters` crate.

**Strengths**:
- Sub-binary registration is handled precisely: Phase 4 correctly identifies and lands checklist points 1, 2, 3, 4, 7, and 8 together, and explicitly reasons through checklist point 5 as a "no action needed" case.
- The skill-binding rule added in Phase 4 exactly mirrors the existing, working precedent in `skills/vcs/commit/SKILL.md`.
- Golden fixture naming matches the existing `work-item-next-number.golden` format exactly, and the new `pup.ron` rule follows the naming and shape of the existing `vcs_domain_imports_only_permitted`/`corpus_domain_imports_only_permitted` rules.

**Findings**:
- 🔴 (major/high) No `work-adapters` crate despite the plan explicitly claiming to mirror the three-crate `vcs` hexagon — Phase 3/4/6; References.
- 🔵 (minor/low) Checklist point 11 (user-facing documentation) is never addressed — Phase 4, item 2.
- 🔵 (suggestion/low) No CHANGELOG.md entry planned for the new `accelerator work` CLI surface — Implementation Approach/Phase 9.
- 🔵 (suggestion/low) `InvalidInput` error type breaks the sibling `<Verb>Error` naming pattern — Phase 3, item 2.

### Usability

**Summary**: The plan is careful about preserving exit-code and error-message parity for the five direct bash-to-Rust ports, and its vertical-slice structure mirrors an established, well-understood precedent. However, its inventory of the CLI's actual consumers — the skills that shell out to these scripts — is incomplete in ways that will leave at least one skill referencing deleted scripts, several skills unable to invoke the new binary due to un-updated `allowed-tools` permissions, and no ported replacement for a function two skills actively call.

**Strengths**:
- Exit-code and error-message parity is preserved deliberately, so existing skill prose that branches on specific exit codes or matches specific error text keeps working without a skill rewrite.
- Phase 4 grants the `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator work *)` permission once, broadly enough to cover every verb the plan later adds.
- The `--set KEY=VALUE` design for `work update` generalises the existing `--add-tag`/`--remove-tag` convention following a widely-recognised CLI idiom.
- The golden-fixture-first phase pins current behaviour before any deletion, giving future contributors a concrete, checkable baseline.

**Findings**:
- 🔴 (critical/high) `wip_canonicalise_id` has no ported Rust/CLI equivalent, yet two skills depend on it directly — Phase 9, item 1/Phase 3, item 8.
- 🟡 (major/high) `extract-work-items.md` is not in the Phase 9 repoint inventory despite depending heavily on `work-item-next-number.sh` — Phase 9, item 1.
- 🟡 (major/high) `allowed-tools` frontmatter is only updated for `update-work-item`; repointed skills elsewhere can't invoke the new binary without a permission prompt — Phase 8, item 3/Phase 9, item 1.
- 🟡 (major/high) `sync-work-items`' safety-critical dirty-check guard has no defined replacement once its backing script is deleted — Phase 9, item 1.
- 🔵 (suggestion/low) No mention of `--help`/usage text for the direct human-CLI consumer — Phase 4-8.
- 🔵 (suggestion/medium) The id-immutability error message's source of truth may now be duplicated between the skill and the CLI — Phase 8, item 2.

### Compatibility

**Summary**: The plan is careful and explicit about exit-code and message-level parity for `work resolve` and `work show`, and correctly identifies the hash-free section-diff comparison as a non-breaking simplification. However, it leaves the exit-code contract for `work diff`, `work create`, and `work update` unspecified, contains an internal contradiction between its own architectural decision and its Phase 8 manual verification claim, and treats the CLI flag surface 0194 is explicitly blocked on as an internal implementation detail rather than a frozen contract to protect.

**Strengths**:
- Phase 4 and Phase 5 map `work resolve`'s and `work show`'s exit codes and error phrasing to the bash originals explicitly and verify them with adapter-boundary tests.
- The plan correctly re-derives that the bash sha256-based section-equality check is a portability workaround, not a behavioural contract, and adds a specific characterization test to prove the replacement is behaviourally identical.
- Reusing the `bash-parity` feature-flag convention mirrors 0169's established pattern for byte-parity rendering.
- `EXIT_CODES.md`'s scope was independently re-verified against the actual file rather than assumed unaffected.

**Findings**:
- 🔴 (major/high) Exit-code contract left unspecified for `diff`/`create`/`update`, unlike `resolve`/`show` — Phase 6/7/8.
- 🔴 (major/high) `document::render`'s whole-tree re-serialization contradicts the plan's own "single-line diff" verification claim — Phase 8, item 2/Manual Verification; Key Discoveries.
- 🟡 (major/high) The invented `--set` flag and CLI signatures are 0194's blocked-dependent contract, but nothing freezes them — Phase 8, item 1; 0194's Assumptions.
- 🔵 (minor/medium) The `work` crate's own function signatures are also part of 0194's dependency surface but aren't flagged as frozen — Phase 3, items 4-8.
- 🔵 (minor/medium) Locale-independent whitespace semantics not called out for the trim/filter port — Phase 3, item 6.

### Safety

**Summary**: The plan's phased structure — goldens captured first, five independently-tested vertical slices, deletion landed last — is a genuinely safe rollout shape, and `work update`'s single atomic whole-file rewrite is actually a corruption-risk improvement over today's multi-step `Edit`-tool mutation. However, Phase 9's script deletion strands `sync-work-items`' local-overwrite guard with no functioning replacement, plausibly flipping a documented fail-safe check into fail-dangerous; `work create`'s new write path has no existence/collision guard where an equivalent already exists elsewhere in the codebase; and the plan's own Phase 8 success criteria contradict its stated non-byte-identical rewrite design.

**Strengths**:
- The phased/independently-mergeable structure defers the deletion step until every replacement command has layered coverage, keeping the bash originals as a live fallback through Phases 1-8.
- `work update` replaces today's multi-step `Edit`-tool mutation with a single atomic whole-file write via `store::atomic_write`.
- The plan deliberately keeps the interactive confirmation/diff-preview UX at the skill layer rather than folding a false sense of safety into the low-level CLI primitive.
- `work create`'s overflow guard is required to surface as a clear CLI error rather than a partial write.

**Findings**:
- 🔴 (critical/high) Deleting `work-item-file-dirty.sh` strands `sync-work-items`' overwrite guard, likely flipping it fail-open — Phase 9, item 1/item 2.
- 🟡 (major/high) `work create` has no existence check before its atomic write, unlike the existing sync-apply precedent — Phase 7, item 2.
- 🟡 (major/high) Whole-frontmatter re-serialisation contradicts the plan's own byte-identical success criterion — Phase 8 Automated Verification; Key Discoveries.
- 🔵 (minor/medium) `work update`'s read-then-write has no lock, enabling a lost-update race between concurrent invocations — Phase 8, item 2.
- 🔵 (suggestion/medium) Consider a full real-corpus parity sweep before permanently deleting the bash originals — Phase 9/Phase 1.

## Re-Review (Pass 2) — 2026-08-06

**Verdict:** REVISE (superseded by Pass 3 below)

All 4 critical and 11 major findings from the initial review were addressed directly in the plan. Re-running all 8 lenses against the updated plan confirmed every one of those fixes but surfaced a fresh, convergent pattern across 4 lenses: `Command::Create`/`Command::Update`'s new flag surfaces couldn't actually carry what `create-work-item`/`update-work-item` need (drafted body content, five typed-linkage fields, list-field edits), plus a new critical exit-code bug (`resolve`'s Path-class miss mapped to exit 1 instead of bash's exit 3) and several architecture/correctness regressions introduced while fixing Pass 1's findings (a domain-crate purity violation in the relocated `project_remote` module, a cross-phase forward reference in `mutate_tags`, an incorrect `CreateInputs` schema, and collision guards that didn't actually close the races they were built for).

### Previously Identified Issues (Pass 1 → Pass 2)
All Pass 1 critical and major findings — Resolved.

### New Issues Introduced (Pass 2)
- 🔴 `resolve`'s Path-class miss exit code (1, should be 3)
- 🔴 `Command::Create` couldn't carry `create-work-item`'s drafted body content
- 🟡 `work-adapters` crate missing (claimed three-crate mirror, delivered two)
- 🟡 `create`/`update` orchestration logic left in the binary crate
- 🟡 `Command::Create` dropped 5 typed-linkage fields; `--set` couldn't represent list fields
- 🟡 No batch/`--count` primitive for `extract-work-items`/`sync-work-items`
- 🟡 `sync-work-items`' "Changes: none" claim was false (also references `next-number`/`section-diff`, both deleted scripts)
- 🟡 Several correctness/architecture regressions (detailed fixes applied)

### Assessment
Not yet ready — the new findings were fixed and a third pass was run.

## Re-Review (Pass 3) — 2026-08-07

**Verdict:** APPROVE

All Pass 2 findings were addressed: the exit code fixed, `--body-file` added to `Command::Create` (redesigned around a placeholder-substitution scheme so the caller never needs a pre-resolved ID, closing a follow-on id/H1 race the first version of the fix introduced), a `work-adapters` crate introduced mirroring `vcs-adapters` exactly, `create`/`update` domain logic extracted into `cli/work`, the five typed-linkage flags plus `--append`/`--remove` list operations plus `--producer` added, a `work next-number` subcommand built, and `sync-work-items`' repoint corrected. Re-running all 8 lenses a third time found no further critical or major issues: `validate_set_key` now rejects `--set` on array-typed fields, `mutate_list`'s missing-key semantics are specified, `work next-number` has an exit-code contract and overflow test, the CLI-surface snapshot test now covers all 8 subcommands (not just 2), the `0194` dependency-surface list is reconciled, and a concrete `author.rs` module was named for VCS-user resolution (no prior Rust precedent existed to reuse).

### Previously Identified Issues (Pass 2 → Pass 3)
All Pass 2 critical and major findings — Resolved.

### New Issues Introduced (Pass 3)
None at critical or major severity. Remaining findings, accepted as-is rather than fixed:
- 🔵 `KNOWN_FRONTMATTER_KEYS` has no test tying it to `compose_frontmatter`'s actual output (code-quality, minor)
- 🔵 `MissingProject`/`ProjectUnused` allocation-error arms untested (test-coverage, minor)
- 🔵 `--help` smoke test covers only 2 of 8 subcommands (test-coverage, suggestion)
- 🔵 `FieldValue` duplicates rather than reuses `corpus::FrontmatterValue`'s shape; `TypedLinkage` has no cross-reference to `corpus::linkage`'s existing vocabulary; `project_remote.rs` doesn't fit its sibling modules' mechanism-based naming (standards, minor/suggestion — discoverability notes, not defects)
- 🔵 Checklist point-11 "no action" reasoning argues by precedent (mirroring `accelerator-vcs`'s own gap) rather than the checklist's literal "not user-facing" criterion (standards, suggestion — a judgment call)
- 🔵 `--append`/`--remove` vs. `--add-tag`/`--remove-tag` is a second, differently-shaped idiom for structurally identical operations — justified in the plan text (tag-specific bash-parity quirk must not leak into fields with no such history) but not surfaced in `--help` (usability, minor)

### Assessment
Ready for implementation. Three full lens-review cycles (Pass 1 initial + 2 re-reviews) resolved every critical and major finding; the plan file is internally consistent (sequential phase/item numbering, no stale cross-references, verified via automated sweep after each round). Remaining findings are minor/suggestion-level polish accepted as intentional trade-offs rather than unresolved defects.

---
*Review generated by /accelerator:review-plan*

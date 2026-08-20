---
type: plan-review
id: "2026-08-19-0213-conversational-conflict-resolution-flow-review-1"
title: "Plan Review: Conversational Conflict Resolution Flow for Sync"
date: "2026-08-19T02:19:43+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-08-19-0213-conversational-conflict-resolution-flow"
target: "plan:2026-08-19-0213-conversational-conflict-resolution-flow"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, safety, security, usability, compatibility]
review_number: 1
review_pass: 3
tags: [skills, sync, work-items, conflicts, cli]
last_updated: "2026-08-19T09:29:12+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Conversational Conflict Resolution Flow for Sync

**Verdict:** REVISE

The plan is well-researched, genuinely test-driven, and structurally sound: it cleanly separates dossier extraction (pure, in the engine) from rendering (I/O, in the CLI), reuses already-public `work` primitives, and its factual claims about the engine and CLI (awaiting-human branch set, frontmatter-graft guarantee, `--resolve` id-keying, lexicographic-sort equivalence) all verified against the code. It is not yet ready for implementation because two clusters of defects recur across nearly every lens: **silent error-swallowing that desynchronises the report from the on-disk dossiers**, and an **incomplete exit-code taxonomy in the skill** that would parse an absent report as clean on codes 1, 2, and 5 — with exit 5 specifically reachable on the `--resolve` apply run the flow itself emits. A recursive `reset_dir` on a config-resolved, gitignored path adds an unrecoverable data-loss surface.

### Cross-Cutting Themes

- **Silent error-swallowing desynchronises the three channels** (flagged by: code-quality, architecture, safety, test-coverage) — `let _ = std::fs::write(...)`, an unspecified `reset_dir` error contract, and `read_to_string(...).unwrap_or_default()` mean a dossier write can fail, or a local read can fabricate an empty side, while the report still emits an `unresolved` line. The exit code, the report line, and the dossier file are correlated only by convention; nothing binds them, so the skill can be told to read a dossier that was never written, or prompt against fabricated content — defeating the very never-prompt-blind guarantee the design exists to provide.
- **Incomplete exit-code taxonomy** (flagged by: correctness, compatibility, architecture) — the skill handles `{0,4,70,71}` as report-bearing and `{72,73,74}` as surface-and-stop, but `run_sync` also returns `1` (internal/config), `2` (usage), and `5` (bulk-overwrite refusal). Exit 5 is not theoretical: the `--resolve` re-invocation re-classifies the whole corpus and refuses when pulls/pushes exceed the default bound of 25, so resolving 26+ conflicts to one side trips it on exactly the second invocation the flow generates. Exit 70 is additionally dual-sourced (report-bearing vs stderr-only).
- **`reset_dir` on a config-resolved path is unrecoverable** (flagged by: safety, security) — the delete is recursive over `<paths.integrations>/<tracker>/conflicts/`, runs on every sync including `--preview`, and the gitignore entry is hardcoded to the *default* path. A non-default `paths.integrations` both leaks remote issue bodies into VCS and puts user-placed files in reach of a recursive delete that VCS cannot revert.
- **The skill's behaviour has no gating test** (flagged by: test-coverage, compatibility, safety) — the eval suite is not wired into `mise run`, the evidence guard passes when files are absent, and the Phase 4 lint asserts keyword *presence*, not behaviour. The safety-critical behaviours (token normalisation / reflexive-Enter, awaiting-human-without-`unresolved`, never-prompt-on-unrenderable) are guarded only by words appearing in a file.
- **The frozen golden omits state lines the skill parses** (flagged by: compatibility, correctness, test-coverage) — the Phase 1 golden covers the action keywords but not the `remote-absent`/`indeterminate` state lines that Phase 4's awaiting-human branch string-matches, nor the empty-corpus `synced 0` summary row. A rename of those `Display` strings would pass the golden and `public-api:check` yet silently break the skill.

### Tradeoff Analysis

- **Security vs Usability (dossier body exposure)**: the skill must print remote issue bodies so the user can choose a side, which surfaces any secret pasted into a ticket into the live transcript — a disclosure surface the gitignore and evidence-scrub controls do not cover. Largely inherent to the feature; recommend noting the residual exposure in the plan rather than engineering it away.
- **Safety vs the project's anti-confirm-UX stance**: the project rejects dry-run/confirm UX for destructive ops because VCS revert is the recovery path. That rationale does not extend here — dossiers are gitignored, so VCS *cannot* recover them. The safety findings on `reset_dir` scoping are therefore legitimate despite the general stance.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Code-Quality / Architecture / Safety**: Silent dossier-write/reset failure leaves the skill reading a missing or stale dossier
  **Location**: Phase 3, Section 1: Persist dossiers
  `let _ = std::fs::write(&path, body)` and an unspecified `reset_dir` swallow all IO errors. `reset_dir` clears the stale dossiers, a subsequent write fails (disk full, permissions, race), and the run still prints an `unresolved` line — the skill then opens a dossier that is absent (or, if reset failed, stale). The guard only covers a *present* dossier carrying `status: unrenderable`, not a missing or stale one.

- 🟡 **Safety**: `reset_dir` recursively deletes a config-resolved directory, unlike the pending-push precedent it cites
  **Location**: Phase 3, Section 1: Persist dossiers (`reset_dir`)
  `remove_dir_all`-style delete on `<paths.integrations>/<tracker>/conflicts/`, run on every sync including `--preview`. The cited `pending-push/` precedent deliberately never wipes — it only appends per-marker files. A misresolved `paths.integrations`, or user-placed files under `conflicts/`, are silently and unrecoverably erased (the directory is gitignored, so VCS cannot revert).

- 🟡 **Correctness / Compatibility / Architecture**: Skill exit-code handling misses codes 1, 2, and 5
  **Location**: Phase 4, Section 2: Exit-code handling; Phase 3 argv test
  The skill enumerates `{0,4,70,71}` and `{72,73,74}` but `run_sync` also returns `1` (internal/config), `2` (usage), `5` (`REFUSED_BULK_OVERWRITE`) — none of which print a report. Exit 5 is directly reachable on the apply `--resolve` re-invocation: default `max_pulls`/`max_pushes` = 25, so resolving 26+ conflicts to one side trips it. The skill would fall through to the report-reading path against absent stdout and misreport a clean sync.

- 🟡 **Architecture / Correctness**: Exit 70 spans two paths with opposite output contracts
  **Location**: Phase 4: Exit-code handling; Desired End State
  Exit 70 is emitted both from the `Ok(report)` arm (prints the report) and the `Err(RunError::Read(...))` arm (`sync.rs:373`, stderr only, no report). The skill cannot distinguish them from the code alone. Currently latent — `run()` never constructs `RunError::Read` today — but the skill's "read the report on 70" assumption is a trap if that arm becomes live.

- 🟡 **Test-Coverage**: The CLI persistence seam has no automated test
  **Location**: Phase 3: Persist dossiers
  The path resolution, `reset_dir` stale-clear, `render_dossier` wiring, and `<id>.md` writes are exercised only in Manual Verification — the argv test runs an *empty* corpus, so `report.dossiers` is empty and the write loop never runs. A regression in the directory path, the naming, or the stale-clear would pass all of `mise run`.

- 🟡 **Test-Coverage**: The eval suite is not wired into `mise run`, so "mise run exits 0" does not exercise the skill
  **Location**: Phase 5: Eval suite / AC "mise run exits 0"
  `claude plugin eval` is not a `mise run` task and the evidence guard passes on absent files, so `mise run` can exit 0 with the skill flow entirely unexercised and no committed evidence. The behavioural AC rests solely on a manually-run, LLM-graded, non-deterministic suite CI never executes.

- 🟡 **Test-Coverage**: Local/remote value binding is not asserted — an operand swap would pass
  **Location**: Phase 2: Dossier extraction tests (AC1)
  The tests assert each dossier "carries all six fields" but not that the `- LOCAL` side matches the seeded local body and `+ REMOTE` the seeded remote body. Swapping local/remote in `build_dossiers` would still populate six fields and pass a presence-only assertion — precisely the defect that misleads a user into overwriting the wrong side.

- 🟡 **Test-Coverage**: Token normalisation and awaiting-human-without-`unresolved` have no test at any level
  **Location**: Phase 4: token normalisation; Phase 5 eval cases
  Neither the reflexive-Enter/typo re-ask-once-then-skip path nor an exit-4 report carrying only `skip-conflict`/`remote-absent` lines (no `unresolved`) is covered by the two eval cases or the keyword lint — the two behaviours most likely to silently discard local edits or misreport "no conflicts".

- 🟡 **Usability**: Local and remote timestamps render in incomparable formats
  **Location**: Phase 2, Section 1: rendered file shape
  `local modified: 1700000000` (raw epoch) vs `remote updated: 2026-07-01T00:00:00Z` (ISO-8601). The two timestamps exist to help the user judge which side is newer; a user cannot eyeball that without a mental conversion, defeating the recency signal at the moment of a destructive choice.

- 🟡 **Usability**: The per-item all-or-nothing choice is not disclosed when many sections differ
  **Location**: Phase 4, Section 1: items 3, 5
  The flow shows every differing section then asks one whole-item remote/local/skip question. Seeing several separately-diffed sections, a user may expect to answer per section; choosing 'remote' to accept one section silently overwrites every other section's local edits.

- 🟡 **Security**: Gitignore is hardcoded to the default path but the write location is config-driven
  **Location**: Phase 3, Section 3 (gitignore); Desired End State
  Dossiers are written under `paths.integrations` (which honours absolute/custom paths) but the `.gitignore` entry is the literal `.accelerator/state/integrations/*/conflicts/` and the only check tests that default. A non-default `paths.integrations` leaves dossiers — carrying remote issue bodies — untracked-but-not-ignored, and jj auto-snapshots on write.

- 🟡 **Security**: "Absent files pass" plus hardcoded filenames leaves the raw-transcript claim unenforced
  **Location**: Phase 5, Section 2 (evidence hygiene guard)
  The guard inspects only `two_conflict.txt` and `clean.txt` and passes when they are absent. A raw transcript committed under any other name, or anywhere under `evals/files/`, is never scanned — so "no raw transcript is committed" is enforced for two filenames, not the directory.

- 🟡 **Compatibility**: The freeze golden omits the `remote-absent`/`indeterminate` state lines the skill parses
  **Location**: Phase 1: characterisation test
  The golden covers action keywords but not `noop\tremote-absent\t-` / `noop\tindeterminate\t-`, yet Phase 4's awaiting-human branch string-matches those `SyncState` `Display` strings. A rename would pass both the golden and `public-api:check` and silently break the skill.

- 🟡 **Safety**: No lock protects the conflicts directory against concurrent runs
  **Location**: Phase 3 / Implementation Approach
  Every run unconditionally `reset_dir`s then rewrites the shared `conflicts/` directory with no lock. A second `work sync` (another checkout, a retry) can wipe and rewrite mid-read, so the skill renders and prompts against dossiers that no longer match the report it is acting on. The corpus adapters already have a mkdir-lock sentinel for this class of hazard.

#### Minor

- 🔵 **Code-Quality / Architecture**: `mtime_of` reimplements `ItemDigests::mtime` semantics
  **Location**: Phase 2, Section 2 (`mtime_of`)
  A new free function re-reads the mtime "with `ItemDigests::mtime` semantics" that `LazyItemDigests::mtime` already computes in this same pass to decide the conflict. Two independent readers can drift, so a dossier's `local modified` could disagree with the mtime that produced the verdict. Thread the already-computed value instead.

- 🔵 **Architecture**: `ItemIndex` is presented as extracting a structure that does not exist
  **Location**: Phase 2, Section 2
  The apply loop does a linear `.iter().find(...)` per action — there is no `ItemIndex` to "extract". The plan introduces a new type and a new borrow/lifetime shape across the preview return, the dossier pass, and the apply loop; framing it as an extraction understates the real integration risk.

- 🔵 **Architecture**: The dossier directory path is derived independently by CLI and skill
  **Location**: Phase 3, Section 1 / Phase 4, Section 1
  The filesystem is the interface; the CLI builds `integrations_root.join(...).join("conflicts")` and the skill resolves the same path via the config CLI. They agree only while both stay in lockstep with the gitignore glob. Add a cross-surface assertion so the seam has one authoritative definition.

- 🔵 **Code-Quality**: `DossierRender` variants are consumed identically
  **Location**: Phase 2, Section 1 / Phase 3, Section 1
  Both variants carry a `String` and the only consumer collapses them: `Renderable(text) | Unrenderable(text) => text`. The type promises a distinction its callers ignore. Either branch on it (count/log unrenderable distinctly) or return `String` + a status.

- 🔵 **Code-Quality**: `reset_dir` error handling is unspecified
  **Location**: Phase 3, Section 1 (`reset_dir`)
  No signature or error contract; the neighbouring `let _ = write` invites an infallible-looking implementation. The stale-dossier manual check depends on the reset actually succeeding — give it a `Result` and surface failure.

- 🔵 **Correctness**: The Phase 1 test literal includes `dossiers: Vec::new()` before Phase 2 adds the field
  **Location**: Phase 1, Section 2
  The code block includes the field, but the prose says it "is added by Phase 2" if Phase 1 lands first — so Phase 1 would not compile standalone, violating the per-phase-green invariant. Omit it from the Phase 1 literal.

- 🔵 **Test-Coverage**: The absent-mtime path is verified only at the render surface, not the extraction boundary
  **Location**: Phase 2: Absent-field paths (AC2)
  The `None` mtime is discharged by a hand-built `ConflictDossier`, not by `mtime_of` actually returning `None` and flowing through. A `mtime_of` bug (e.g. `Some(0)` instead of `None`) would not be caught. Note the FS-boundary difficulty explicitly or add a focused `mtime_of` unit test.

- 🔵 **Test-Coverage**: The golden hides the `lines.sort()` order and never exercises the empty-corpus summary
  **Location**: Phase 1: characterisation golden
  Zero-padded ids make the lexicographic sort coincide with numeric order (so a sort-key regression is invisible), and item 0008 forces `synced_count > 0` (so the `synced 0` branch is untested). Add an all-clean/empty case.

- 🔵 **Usability**: The unrenderable conflict is a dead end with no values and no recovery guidance
  **Location**: Phase 2, Section 1 (Unrenderable variant); Phase 4, Section 1 item 2
  When `diff` is unavailable the dossier carries only "a one-line note" — no section names, no values — and the skill declines to prompt. The user is told a conflict exists with no way to see it and no next step. Still list the section names and raw values, and point at the dossier path plus a "install diff / resolve manually" hint.

- 🔵 **Usability**: `(not reported)` vs `(not read)` is opaque without a legend
  **Location**: Phase 2, Section 1: absent-field rendering
  The two phrasings depend on the `RemoteTimestamp` variant with nothing explaining the difference. Collapse to `(unavailable)` for the human render or add a short parenthetical.

- 🔵 **Usability**: The flow depends on an unspecified preceding `--preview` run
  **Location**: Phase 4, Section 1 item 1; Migration Notes
  The subsection opens "after the `accelerator work sync --preview` run" but the surrounding step flow that decides preview-vs-apply belongs to 0212. If 0213 lands first, or 0212 wires an apply run, the premise is unmet. State where the dossier-producing run is issued and make the subsection tolerant of either.

- 🔵 **Security**: The dossier filename uses the raw, unvalidated work-item id
  **Location**: Phase 2 `build_dossiers` / Phase 3, Section 1
  `conflicts_dir.join(format!("{}.md", dossier.id))` uses `planned.id` verbatim with no `identifier_is_safe`/`normalise_id` check at the write site (and `identifier_is_safe` permits `/`). A crafted local id with `/` or `..` could escape `conflicts/`. Locally-assigned, so low-likelihood, but nothing constrains it at the boundary.

- 🔵 **Security**: The scrub logic is reimplemented in Python with no shared source
  **Location**: Phase 5, Section 2
  The plan mirrors `is_reduced` in Python, duplicating the grammar and the provider-specific denylist. The two can drift, and a token shape outside the denylist (`ghp_`, OAuth, Basic) inside a `#` header could pass. Prefer shelling to the canonical Rust check or treat the structural grammar (not the denylist) as primary.

- 🔵 **Compatibility**: Adding a `pub` field to non-`#[non_exhaustive]` `RunReport` forces all construction sites in step
  **Location**: Phase 2, Section 3
  No consumer break (`work-adapters` is exempt, all sites in-workspace), but both `run.rs` return sites and the golden test must be updated in one commit. Optionally mark `RunReport` `#[non_exhaustive]`.

#### Suggestions

- 🔵 **Correctness**: The golden's sort equivalence silently depends on 4-digit ids
  **Location**: Phase 1, Section 1
  Lexicographic order diverges from numeric once id widths differ (`"10000"` < `"9999"`). Safe for the `NNNN` domain today; record the fixed-width precondition alongside the golden.

- 🔵 **Security**: Dossier bodies enter the conversation transcript by design
  **Location**: Phase 4, Section 1
  A secret pasted into a remote ticket surfaces in the live transcript, outside the gitignore and evidence-scrub controls. Largely inherent; note the residual exposure and consider truncating body content the diff does not need.

### Strengths

- ✅ Extraction is cleanly separated from rendering — structured `ConflictDossier` built with no subprocess in the engine, `render_dossier` taking an injected section renderer — a textbook functional-core/imperative-shell split that makes the `DiffUnavailable`/`Unrenderable` downgrade deterministically forceable without removing the system `diff`.
- ✅ The dossier reuses already-public `work` primitives (`SectionDiff`, `differing_sections`) and the `work-adapters` public-api exemption, so it adds no snapshot churn to the pinned `work` crate — a verified, well-targeted use of the module boundaries.
- ✅ The plan's factual claims verified against the code: the awaiting-human action/state branch set exactly mirrors `RunReport::awaiting_human()`; `reconstruct_pulled_content` grafts local frontmatter so `differing_sections` can never surface `frontmatter`; the `--resolve` id-keying semantics (duplicate → exit 2, unknown token → warn-and-skip, non-`Prompt` id → inert) are correctly characterised.
- ✅ Test-driven ordering is explicit and correctly sequenced — Phase 1 freezes the untouched report format with a characterisation golden before Phases 2-3 perturb surrounding code — and the dossier is tested at two offline boundaries (`RecordingTracker` and real clients over `MockServer`), exercising the real projection path.
- ✅ The safety instincts are strong where they land: the "conflict writes neither side" property is preserved for work items, the typed-token prompt keeps no Enter default and re-asks once before skip, normalisation is moved into the skill rather than routing raw input into `--resolve`'s silent warn-and-skip, and the never-prompt-blind guard is pinned by the lint.
- ✅ The `diff -u` invocation is spawned with fixed argv over temp files and no shell, so attacker-controlled title/body/section text cannot inject a command, and the reduced-evidence convention is reused rather than reinvented.

### Recommended Changes

1. **Stop swallowing IO errors and define the missing-dossier contract** (addresses: silent write/reset failure, channel desync, prompt-against-missing-dossier). Give `reset_dir` and the dossier write a `Result`, surface failures to stderr with id + path, and specify that the skill treats a *missing or unreadable* dossier for an `unresolved` id as fail-safe (report unrenderable, do not prompt) — the same as `status: unrenderable`.

2. **Scope `reset_dir` to dossier-shaped files and make the ignore guarantee config-aware** (addresses: recursive delete on config path, gitignore hardcoded to default). Remove only files matching the `<id>.md` dossier pattern rather than `remove_dir_all` on the whole directory; either write to an unconditionally-ignored path or derive the ignore from the configured `paths.integrations` at runtime, with a test exercising a customised value.

3. **Complete the exit-code taxonomy in the skill and its lint** (addresses: missing 1/2/5, exit-70 dual-sourcing). Add an explicit catch-all for `1`, `2`, `5` (report-absent → surface stderr and stop), extend the Phase 4 lint's exit-code predicate to require it, add `5` to the Phase 3 argv-acceptance accepted set, and note that exit 70's report-read depends on read failures riding the report.

4. **Make the skill behaviour testable, or state plainly it is out-of-CI** (addresses: eval not wired, keyword-only lint). Add a committed-evidence *existence* check so the "evidence committed" clause is enforced rather than vacuous, add eval cases for the exit-4-without-`unresolved`, the reflexive-Enter/typo, and the `status: unrenderable` paths, and state in the plan that the eval suite is non-gating.

5. **Pin the values, not just their presence** (addresses: operand-swap coverage gap, golden state-line gap). Assert the `- LOCAL`/`+ REMOTE` sides match the seeded local/remote bodies; add `remote-absent`, `indeterminate`, and an empty-corpus item to the Phase 1 golden so every state string the skill parses is frozen.

6. **Fix the two render-surface usability defects** (addresses: incomparable timestamps, undisclosed item-wide scope). Render both timestamps in one comparable format (ISO-8601), and when an item has more than one differing section, state in the prompt that the choice applies to all N sections.

7. **Tighten evidence hygiene and validate the dossier filename** (addresses: absent-files-pass bypass, unvalidated id). Glob the whole `evals/evidence/` directory in the hygiene guard rather than an allowlist of two names, and reject any id containing a path separator or `..` before using it as a filename component.

8. **Correct the plan's mischaracterisations** (addresses: `mtime_of` duplication, `ItemIndex` framing, Phase 1 field). Thread the run's already-computed mtime instead of re-reading it, frame `ItemIndex` as a new shared lookup replacing the linear `find`, and drop `dossiers: Vec::new()` from the Phase 1 test literal.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan makes a clean structural choice — extraction (pure, in the engine) separated from rendering (I/O, in the CLI), reusing already-public `work` primitives and the `work-adapters` public-api exemption, honouring the functional-core/imperative-shell split ADR-0045 mandates. Its main weakness is that the conflict flow is coordinated across three independent, convention-coupled channels — the four-column report on stdout, per-id dossier files on disk, and the process exit code — with no single contract binding them, and several failure modes (a `RunError::Read` exit-70 that prints no report, a silently-swallowed dossier write) can desynchronise those channels beneath a skill that assumes they agree.

**Findings**:
- 🟡 major (medium) — Phase 4 / Desired End State — Exit 70 spans two code paths with different output contracts (`Ok(report)` prints the report and exits 70 when a retryable failure is present; `Err(RunError::Read)` exits 70 with stderr only, no report). The skill cannot distinguish them from the code alone.
- 🟡 major (medium) — Phase 3, Section 1 — Silent dossier-write failure (`let _ = std::fs::write`) desynchronises the report from the on-disk dossiers; a `Prompt` item's `unresolved` line points at a file that may not exist.
- 🔵 minor (medium) — Phase 2, Section 2 — `mtime_of` duplicates the canonical mtime semantics `LazyItemDigests::mtime` already computes this pass; the two readers can drift.
- 🔵 minor (high) — Phase 2, Section 2 — `ItemIndex` is presented as extracting an existing structure that does not exist; it is a new type and a new borrow/lifetime shape.
- 🔵 minor (medium) — Phase 3, Section 1 / Phase 4, Section 1 — The dossier directory path is computed independently by the CLI and the skill; they agree only coincidentally.

### Code-Quality

**Summary**: Well-structured for maintainability — extraction cleanly separated from rendering, a testable `build_dossiers` free function, and reuse of existing public `work` primitives. The main concerns are error-handling smells in the Rust snippets: a fully swallowed dossier write, `unwrap_or_default()` substituting an empty local side into a human-facing conflict surface, and unspecified error handling on the stale-directory reset — plus an enum whose variants are consumed identically and a `mtime_of` that reimplements existing semantics.

**Findings**:
- 🟡 major (high) — Phase 3, Section 1 — Dossier write swallows all IO errors with `let _ = std::fs::write`; a silent failure becomes a confusing skill-side missing-dossier failure with nothing in logs.
- 🟡 major (medium) — Phase 2, Section 2 — `unwrap_or_default()` silently substitutes an empty local side; a transient read error becomes a plausible-but-wrong dossier presented before an irreversible overwrite choice.
- 🔵 minor (medium) — Phase 2/3 — `DossierRender` enum variants are consumed identically; the type promises a distinction its consumers ignore.
- 🔵 minor (medium) — Phase 2, Section 2 — `mtime_of` reimplements `ItemDigests::mtime` semantics instead of reusing them; the copies can diverge silently.
- 🔵 minor (medium) — Phase 3, Section 1 — `reset_dir` error handling is unspecified and easily swallowed; a stale or missing directory produces the wrong signal with no diagnostic.

### Test-Coverage

**Summary**: Genuinely test-driven and its Rust-boundary coverage is strong — the frozen golden, the two-conflict dossier assertions over both `RecordingTracker` and the real-client `MockServer` seam, and the injectable renderer for the `DiffUnavailable` path are well-chosen and network-free. The gaps concentrate at two seams the automated suite never reaches: the CLI persistence layer (reset/write/render wiring is manual-only because the argv test runs an empty corpus) and the entire skill behaviour, whose only CI guard is a keyword-presence lint since the eval suite is not wired into `mise run`.

**Findings**:
- 🔴 major (high) — Phase 3 — The CLI persistence seam (path resolution, `reset_dir` stale-clear, `render_dossier` wiring, `<id>.md` writes) has no automated test; the argv test runs an empty corpus so the write loop never executes.
- 🔴 major (high) — Phase 5 — The eval suite is not wired into `mise run` and the evidence guard passes on absent files, so `mise run` can exit 0 with the skill flow unexercised and no evidence.
- 🟡 major (medium) — Phase 4/5 — Token normalisation (reflexive-Enter/typo → re-ask-once-then-skip) and awaiting-human-without-`unresolved` (exit-4 with only skip-conflict/remote-absent lines) have no test at any level.
- 🟡 major (medium) — Phase 2 (AC1) — The local/remote value binding is not asserted; an operand swap in `build_dossiers` would still populate six fields and pass a presence-only assertion.
- 🔵 minor (medium) — Phase 2 (AC2) — The absent local-mtime case is discharged only by a hand-built dossier at the render surface, not at the extraction boundary the AC names.
- 🔵 minor (medium) — Phase 1 — The golden's zero-padded ids hide the `lines.sort()` order, and item 0008 forces `synced_count > 0` so the empty-corpus `synced 0` summary branch is never exercised.
- 🔵 minor (medium) — Phase 5 / Phase 3 — The eval assertions are gradeable but non-deterministic and non-gating, and the never-prompt-blind-on-unrenderable property is covered by neither eval case nor the lint (Manual Verification only).

### Correctness

**Summary**: The plan's core factual claims about the engine and CLI are accurate — the awaiting-human action/state branch set, the frontmatter-graft guarantee, the id-keyed `--resolve` semantics, the lexicographic-sort-with-zero-padded-ids equivalence, and the preview-builds-dossiers-before-early-return interaction all check out. The one substantive gap is branch completeness in the skill's exit-code handling: the plan enumerates `{0,4,70,71}` and `{72,73,74}` but the binary can also exit 1, 2, and 5 — and exit 5 (bulk-overwrite refusal) is specifically reachable on the `--resolve` re-invocation this flow generates.

**Findings**:
- 🔴 major (medium) — Phase 4, Section 2 / Desired End State — The exit handling is a two-way partition that omits exits 1, 2, and 5; exit 5 trips when a `--resolve` run resolves more than 25 conflicts to one side (default `max_pulls`/`max_pushes`), on exactly the second invocation the flow emits.
- 🔵 minor (medium) — Phase 1, Section 2 — The test literal constructs `RunReport { ..., dossiers: Vec::new() }` before Phase 2 adds the field, so Phase 1 would not compile standalone.
- 🔵 minor (low) — Current State / Phase 4, Section 2 — Exit 70 is dual-sourced; the report-bearing arm is live today but `Err(RunError::Read)` (report-absent) is a latent trap for the skill's report-read-on-70 assumption.
- 🔵 suggestion (low) — Phase 1, Section 1 — The golden's numeric-sort equivalence holds only while ids share a fixed width; record the precondition.

### Safety

**Summary**: Unusually safety-conscious for a skills-layer change — it preserves the "a conflict writes neither side" property for work items, keeps the no-Enter-default typed token, and guards against prompting blind on an unrenderable conflict. The material gaps cluster around the new destructive `reset_dir` on a config-resolved path and the swallowed dossier-write errors, both of which introduce a recursive-delete and a silent-failure mode the pending-push precedent deliberately avoided, and neither recoverable via VCS because the directory is gitignored.

**Findings**:
- 🔴 major (high) — Phase 3, Section 1 — `reset_dir` recursively deletes a config-resolved directory, unlike the append-only pending-push precedent it cites; a misresolved path or user-placed content is silently and unrecoverably erased on every sync.
- 🟡 major (high) — Phase 3, Section 1 — Swallowed dossier-write and reset failures leave the skill prompting against a missing or stale dossier — the exact prompt-blind/stale-file hazard the design otherwise defends against.
- 🟡 major (medium) — Phase 3 / Implementation Approach — No lock protects the conflicts directory against concurrent runs racing reset/write/read; the corpus adapters already have a mkdir-lock sentinel for this class of hazard.
- 🔵 minor (high) — Migration Notes — `--preview` gains a recursive-delete side effect on a flag users treat as read-only; the flag reached for safety is now the first to exercise the destructive delete path.

### Security

**Summary**: The core secret-handling instinct is sound — dossiers carrying remote issue bodies live in gitignored transient state, the diff subprocess is spawned with fixed argv over temp files (no shell), and committed eval evidence is reduced to a secret-scrubbed grammar. The two real exposures are that the gitignore entry is hardcoded to the default state path while the actual write location is config-driven, and that the evidence-hygiene guard's "absent files pass" rule combined with hardcoded filenames gives no positive assurance a raw transcript wasn't committed under some other name. A minor unvalidated-id path-traversal risk exists at the write site.

**Findings**:
- 🔴 major (medium) — Phase 3, Section 3 / Desired End State — The gitignore is hardcoded to `.accelerator/state/integrations/*/conflicts/` but the write location honours `paths.integrations`; a non-default value leaves dossiers (remote issue bodies) untracked-but-not-ignored, and jj auto-snapshots on write.
- 🟡 major (medium) — Phase 5, Section 2 — "Absent files pass" plus hardcoded filenames means the guard only ever inspects two exact names; a raw transcript under any other name passes CI, defeating the stated scrub protection.
- 🔵 minor (medium) — Phase 2 / Phase 3, Section 1 — The dossier filename uses the raw, unvalidated work-item id (`identifier_is_safe` permits `/`); a crafted id with `/` or `..` could escape `conflicts/`.
- 🔵 minor (medium) — Phase 5, Section 2 — Security-critical scrub logic is reimplemented in Python with no shared source; the two copies can drift and the denylist is provider-specific.
- 🔵 suggestion (low) — Phase 4, Section 1 — Dossier bodies enter the conversation transcript by design; a secret in a ticket body surfaces outside the gitignore and evidence-scrub controls.

### Usability

**Summary**: The plan preserves the well-designed typed-token prompt (remote/local/skip, no Enter default, explicit overwrite semantics) and correctly moves token normalisation into the skill so a typo can never be silently discarded — both real DX wins. The main risks are in the rendered dossier the user actually reads to decide: the two timestamps are rendered in incomparable formats, the per-item all-or-nothing choice is not disclosed when many sections are shown, and the unrenderable path leaves the user with no diff, no values, and no recovery guidance.

**Findings**:
- 🔵 major (high) — Phase 2, Section 1 — Local and remote timestamps render in incomparable formats (raw epoch vs ISO-8601), defeating the recency signal at the moment of a destructive choice.
- 🟡 major (medium) — Phase 4, Section 1 (items 3, 5) — The per-item all-or-nothing choice is not disclosed when many sections differ; choosing 'remote' for one section silently overwrites every other section's local edits.
- 🔵 minor (medium) — Phase 2, Section 1 / Phase 4, Section 1 item 2 — The unrenderable conflict is a dead end with no values and no recovery guidance.
- 🔵 minor (medium) — Phase 2, Section 1 — `(not reported)` vs `(not read)` is opaque without a legend.
- 🔵 minor (medium) — Phase 4, Section 1 item 1 / Migration Notes — The flow depends on an unspecified preceding `--preview` run whose issuance belongs to 0212.

### Compatibility

**Summary**: Fundamentally additive and its core contract claims check out — `work-adapters` is genuinely exempt from `cargo-public-api`, `work` is pinned but `SectionDiff`/`differing_sections` are already `pub`, and the four-column report is correctly treated as contractual and frozen by a golden. The `RunReport` widening is safe (all construction sites in-workspace, compile-checked). The main risks are on the exit-code contract: the skill's handling does not cover the binary's full taxonomy (notably exit 5 on the apply `--resolve` run), and the report golden omits the state lines the skill's awaiting-human branch string-matches.

**Findings**:
- 🔴 major (high) — Phase 4, Section 2 / Phase 3 argv test — The skill covers only `0/4/70/71` and `72/73/74`; `run_sync` also returns `1`, `2`, and `5`, and exit 5 is reachable on the write-heavy `--resolve` re-invocation (default bound 25).
- 🔴 major (medium) — Phase 1 — The golden omits `noop\tremote-absent\t-` and `noop\tindeterminate\t-`, yet Phase 4 string-matches those `SyncState` `Display` strings; a rename passes the golden and `public-api:check` but breaks the skill.
- 🔵 minor (high) — Phase 2, Section 3 — Adding `pub dossiers` to non-`#[non_exhaustive]` `RunReport` forces all struct-literal sites in step (no consumer break; consider `#[non_exhaustive]`).

## Re-Review (Pass 2) — 2026-08-19T08:04:46+00:00

**Verdict:** COMMENT

All eight lenses re-ran against the edited plan. **Every one of the ~14 major and 13 minor findings from Pass 1 is resolved** — verified fresh against the code by each lens. The edits, however, introduced a new cluster of issues concentrated in the code I added (`build_dossiers` mtime threading, the persist loop, the config-aware ignore). The re-review agents flagged five new majors; all five have since been addressed in a second edit round (see Assessment). What remains is minor/suggestion-level polish, so the plan is acceptable to implement.

### Previously Identified Issues

- 🟡 **Architecture**: Exit 70 dual-path output contract — **Resolved** (skill falls back to stderr on empty-report 70).
- 🟡 **Architecture**: Silent dossier-write desync — **Resolved** (surfaced `eprintln` + fail-safe skill contract).
- 🟡 **Code-Quality**: Swallowed `let _ = write` — **Resolved**.
- 🟡 **Code-Quality**: `unwrap_or_default` fabricating empty local side — **Resolved** (`local_unreadable` dossier).
- 🟡 **Safety**: `reset_dir` recursive delete on config path — **Resolved** (scoped `<id>.md` clear).
- 🟡 **Safety**: Swallowed failure → missing/stale dossier — **Resolved** (fail-safe skill guard).
- 🟡 **Safety**: No lock on the conflicts dir — **Resolved to accepted** (serialised flow + `--resolve` rewrite; atomic write added this round).
- 🔴 **Correctness / Compatibility**: Exit taxonomy missed 1/2/5 — **Resolved** (surfaced + catch-all; argv accepts 5).
- 🔴 **Test-Coverage**: CLI persistence seam untested — **Partially → now Resolved** (persist loop extracted to `persist_dossiers`, unit-tested this round).
- 🔴 **Test-Coverage**: Eval not wired into `mise run` — **Resolved** (declared non-gating; existence check + directory-glob guard).
- 🟡 **Test-Coverage**: Token-normalisation / awaiting-without-`unresolved` untested — **Resolved** (three new eval cases).
- 🟡 **Test-Coverage**: Value binding not asserted — **Resolved** (value-bound assertions).
- 🔵 **Usability**: Incomparable timestamps — **Resolved** (both ISO-8601).
- 🟡 **Usability**: Undisclosed item-wide scope — **Resolved** (count-and-consequence line).
- 🔴 **Security**: Gitignore hardcoded to default path — **Resolved** (directory-local `.gitignore` `*`; fail-closed added this round).
- 🟡 **Security**: Absent-files-pass evidence bypass — **Resolved** (directory glob, grammar-primary).
- 🔵 **Security**: Unvalidated dossier filename — **Resolved** (`id_is_filename_safe`; tested this round).
- 🔵 **Compatibility**: Golden omits `remote-absent`/`indeterminate` — **Resolved** (added to golden + empty-corpus test).
- 🔵 **Correctness**: Phase 1 literal won't compile — **Resolved** (`dossiers` field removed from the Phase 1 literal).

### New Issues Introduced (by the Pass 1 edits)

- 🟡 **Architecture / Correctness / Code-Quality** (major): `build_dossiers` threaded `item.mtime`, but `LocalItem` has no mtime field and the classifier's `LazyItemDigests::mtime` is cold for the common blanked-hash conflict — would render `(unavailable)` for real conflicts. **Addressed**: reverted to a fresh `file_mtime_secs(item.path)` metadata read (a display value); `ItemIndex` is now a plain id→`&LocalItem` map.
- 🟡 **Test-Coverage** (2× major): the inline persist loop and `id_is_filename_safe` had no test. **Addressed**: extracted `persist_dossiers(dossiers, dir, render)` as a pure function unit-tested against a `TempDir`; added an `id_is_filename_safe` table test and a fail-closed test.
- 🔴 **Security** (major): the sensitive dossier write was not ordered behind a guaranteed ignore. **Addressed**: `prepare_conflicts_dir` is now fail-closed — `persist_dossiers` is not called unless the directory-local `.gitignore` is verified present.
- 🟡 **Security** (major): remote issue body is untrusted content fed to the LLM-driven skill (prompt-injection surface). **Addressed**: added an explicit untrusted-data instruction (human token is the sole authority) plus a lint predicate.
- 🔵 **Correctness** (minor): exit 5 was described as `--resolve`-only; the refusal runs in both modes. **Addressed**: corrected in the argv and exit-handling prose.
- 🔵 **Safety** (minor): the clear pattern risked a `*.md` over-delete. **Addressed**: pinned to the `id_is_filename_safe` id shape; atomic write closes the partial-read window.
- 🔵 **Usability** (minors): `local` under-warned; `<external_id>` not in the dossier. **Addressed**: both choices now say OVERWRITE; the skill sources `external_id` from local frontmatter.

### Residual (accepted, not blocking)

- 🔵 **Code-Quality** (suggestions): `DossierRender`'s two variants collapse at the persist site (tests still branch on them); the remote-gather sequence duplicates the Pull arm (a `remote_view` helper would DRY it); the `local_unreadable` arm discards the remote stamp it holds.
- 🔵 **Architecture / Compatibility** (minors): the `conflicts/<id>.md` path is derived on both the CLI and skill sides, and the report keyword vocabulary is coupled across binary/golden/skill/lint by a manual tripwire — both degrade safely but could be pinned by having the binary emit the path and by deriving the lint's keyword set from the binary's source of truth (or an exhaustive-match golden).
- 🔵 **Usability** (suggestions): the unrenderable note could name the exact file to edit; the multi-section disclosure line's placement (before the token) could be fixed.
- 🔵 **Security** (minor): `id_is_filename_safe` guards only the filename; validating the id once at the domain boundary (`identifier_is_safe`) would also cover the report line and the skill's `--resolve` shell emission.

### Assessment

The plan is in good shape to implement. Pass 1's structural and safety gaps are fully closed, and the five new majors the edits introduced — the mtime regression, the two untested seams, the fail-closed ignore, and the untrusted-content boundary — have each been addressed in a second edit round. The residual items are genuine polish (DRY helpers, path-contract pinning, a couple of prompt-wording niceties) that can be handled during implementation without a further planning cycle. No critical findings at any pass.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-08-19T08:18:17+00:00

**Verdict:** COMMENT

All eight lenses re-ran a final time against the plan as edited after Pass 2. **Every Pass-2 fix is confirmed** — three lenses verified the mtime redesign, the fail-closed ignore, the extracted `persist_dossiers`, and the exit-code taxonomy directly against the code and found them correct. The pass surfaced a further cluster of issues, mostly latent consequences of the Pass-2 edits (the loosely-defined id shape, the new lint's coverage, the new helpers' test seams). Four were major; all four have been addressed in a third edit round. What remains is suggestion-level, so the plan holds at COMMENT and is ready to implement.

### Previously Identified Issues (from Pass 2's new findings)

- 🟡 **Architecture / Correctness / Code-Quality**: `item.mtime` regression — **Resolved and verified** (fresh `file_mtime_secs` metadata read; the classifier's mtime is provably cold for a persisting conflict, confirmed against `classify.rs:66-68`).
- 🔴 **Test-Coverage**: persist loop untested — **Resolved** (pure `persist_dossiers`, TempDir-driven).
- 🔴 **Test-Coverage**: `id_is_filename_safe` untested — **Resolved** (table test).
- 🔴 **Security**: sensitive write not fail-closed — **Resolved and verified** (`persist_dossiers` runs only on the `Ok` arm; explicit AC).
- 🟡 **Security**: untrusted remote body — **Resolved** (explicit boundary + lint predicate; behavioural eval added this round).
- 🔵 **Correctness**: exit 5 `--resolve`-only wording — **Resolved** (refuses in both modes, verified `run.rs:174` precedes `:183`).
- 🔵 **Safety**: `*.md` over-delete — **Superseded** by the sharper finding below, now fixed.

### New Issues (from the Pass 2 edits) — all addressed this round

- 🔴 **Security** (major): the id passed the filename check but reached the skill's `--resolve` shell template — `0001; rm -rf ~` is a command-injection sink. **Addressed**: `id_is_token_safe` now reuses the corpus canonical-id check (`is_canonical_id_token`) at the build boundary — shell-inert and filename-safe — and the skill emits each `--resolve` order as a discrete argv token, never a spliced shell string; a lint predicate pins the argv-safe emission.
- 🔴 **Safety** (major): the stale-clear pinned to the permissive id check still swept user-named `.md` files (`notes.md`), unrecoverably (gitignored). **Addressed**: the clear is pinned to the canonical-id domain (not the permissive guard), verifies the ignore *before* clearing, and also sweeps its own `.tmp-*` artefacts; the AC fixture is now a real `notes.md` that must survive.
- 🟡 **Test-Coverage** (major): the fail-closed guarantee had no offline seam. **Addressed**: extracted `persist_conflict_dossiers(dir, dossiers, render)` as a pure compose helper, driven offline by forcing the `Err` path (read-only dir), asserting no write and no clear.
- 🟡 **Test-Coverage** (major): untrusted-body resistance was prose-lint-only. **Addressed**: added an `injected_body` eval case (forged `status:`/imperative body) asserting the human token stays authoritative.
- 🟡 **Usability** (major): the lint did not pin the safety-critical prompt wording. **Addressed**: added predicates for OVERWRITE-on-both, the multi-section scope line, and the re-ask-once normalisation.
- 🔵 **Security / Architecture** (minors): the unrenderable dossier's verbatim raw values could forge a column-0 `status:` line. **Addressed**: raw values are line-prefixed, and the skill reads the verdict only from the header region above the first `=== ` delimiter.
- 🔵 **Correctness / Code-Quality** (minors): the `local_unreadable` arm hardcoded `NotRead` and read an mtime for an unreadable file. **Addressed**: it now threads the real remote stamp and renders `local modified` absent.
- 🔵 **Architecture** (suggestion): dossiers were built before the refusal early-return. **Addressed**: the pass now runs after `run.rs:174-181`, so an aborting run does no dossier work.
- 🔵 **Usability** (minors): disclosure-line placement and the unrenderable dead-end. **Addressed**: the scope line is placed before the token and names the file; the unrenderable note names the file to edit.

### Residual (accepted, not blocking)

- 🔵 **Compatibility** (minor): the dossier file format is a CLI↔skill contract represented in three places (render tests, lint, eval fixtures) with no single golden — a rename could drift; a dossier-format golden or shared constants would pin it.
- 🔵 **Code-Quality** (suggestions): extract a `gathered_remote(facts, id)` helper shared with the Pull arm; document that `DossierRender`'s discriminant exists for test clarity.
- 🔵 **Safety** (suggestion): label the two header timestamps by source (local file mtime vs remote server time), since they come from unsynchronised clocks.

### Assessment

The plan is ready to implement. Across three passes the trajectory is convergent: Pass 1 closed the structural and safety gaps, Pass 2 the regressions those edits introduced, and Pass 3 the last latent consequences (id-domain tightening, test seams, lint coverage). The security-critical items — the fail-closed ignore, the command-injection sink, and the untrusted-content boundary — are all closed and, where checkable, verified against the code. The remaining suggestions are refinements an implementer can fold in without another planning cycle. No critical findings at any pass.

---
*Re-review (Pass 3) generated by /accelerator:review-plan*

## Approval — 2026-08-19T09:29:12+00:00

**Verdict: APPROVE.** Across three passes every critical, major, and substantive minor finding is resolved and, where checkable, verified against the code; only suggestion-level refinements remain, all safe to fold in during implementation. The plan is approved for implementation and the target plan is marked `ready`.

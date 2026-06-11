---
type: plan-review
id: "2026-06-11-0106-invoke-plugin-scripts-by-bare-path-review-1"
title: "Plan Review: Invoke Plugin Scripts by Bare Path in Skill Bodies"
date: "2026-06-11T13:52:50+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-06-11-0106-invoke-plugin-scripts-by-bare-path"
parent: "plan:2026-06-11-0106-invoke-plugin-scripts-by-bare-path"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, documentation, test-coverage, standards, architecture, usability, portability]
review_number: 1
review_pass: 2
tags: [permissions, allowed-tools, skills, plugin, authoring-convention]
last_updated: "2026-06-11T16:17:52+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Invoke Plugin Scripts by Bare Path in Skill Bodies

**Verdict:** REVISE

This is an unusually well-researched plan: every empirical claim it makes
(the 276-line grep decomposition into 213 + 35 + 14 + 14, the cited line
numbers, the fence-state guard isolating exactly 7 lines / 5 files, the
defectiveness of the original AC3 regex) was re-verified against the live
tree and holds exactly. The *edits it describes* are correct and the
unquoted-braced bare-path invariant is applied consistently. The reason for
REVISE is not the edit logic — it is that the plan's **verification strategy
does not actually prove its own acceptance criteria**: AC1 ("a directive at
every occurrence") is checked only by file-level greps that pass even when a
multi-site file is half-covered, the 12 config/jira sites get no
directive-presence assertion at all, and the headline "guard returns empty"
gate proves only that 7 of 28 sites changed. A cluster of major findings
about the convention having no codified home, an under-specified
restructure, and an unvalidated runtime payoff round out the picture.

### Cross-Cutting Themes

- **The convention has no single source of truth** (flagged by:
  documentation, standards, architecture, usability) — the rule lives only
  as ~28 hand-copied directive passages. There is no authoring note, ADR, or
  shared snippet stating it once. Every angle converged on this: it bloats
  runtime instructions, leaves only the `bash`/`sh`/`env` *fragment* as a
  greppable marker (not a conforming directive), gives future authors nothing
  to discover, and guarantees erosion on the next edit before the
  out-of-scope 0107 guardrail lands.

- **Verification proves the low-risk quarter, not the high-risk three
  quarters** (flagged by: correctness, test-coverage, portability) — the
  fence-state guard only ever sees the 7 bare-fence lines. AC1's real surface
  is the 21 inline/config/jira directive-adds, and those are gated only by a
  file-level `grep -q` (false-passes on multi-site files), a no-op exclusion
  filter, and — for the 12 config/jira sites — *nothing*.

- **The `grep -v create-adr/SKILL.md:153` filter is a no-op** (flagged by:
  correctness, test-coverage, portability) — `grep -rl` emits bare filenames
  with no `:153` suffix, so the exclusion never matches. Harmless to the
  outcome here, but it signals an intent the command does not implement.

- **The extract-work-items restructure is under-specified** (flagged by:
  correctness, test-coverage, usability, architecture) — replacing
  `PATTERN=$(...)` with prose "capture its output as PATTERN" removes a
  concrete, rule-escaping shape but does not pin the replacement shape, so the
  model could re-emit `VAR=$(...)` (the exact escape) or `bash <path>`.

### Tradeoff Analysis

- **Explain-why vs instruction concision**: documentation credits the
  per-site rationale ("escapes the skill's `allowed-tools` permission…") as
  genuinely useful *for a human author*, while usability and documentation
  both note it is token bloat *for the model*, paid on every invocation,
  repeated up to 9× in one file (configure). Recommendation: keep a terse
  imperative at the call sites and state the rationale once in a canonical
  authoring note — which also resolves the "no source of truth" theme.

- **Two directive forms (flexibility) vs one fixed string (greppability)**:
  the plan offers Block and Appended variants to fit different grammatical
  contexts; standards/usability argue this yields 28 slightly-different
  phrasings and a choice burden, and that a single canonical sentence would
  be both recognisable and machine-checkable. Recommendation lean toward one
  pinned string with placement guidance, since 0107 will need a stable marker
  to lint against.

### Findings

#### Critical

- 🔴 **Test Coverage / Correctness**: File-level directive grep cannot prove per-occurrence coverage (AC1 false-PASS)
  **Location**: Phase 1 & Phase 2 Success Criteria
  AC1 requires a directive in the *same passage* as *every* occurrence, but
  the check is `grep -q` at file granularity. `extract-adrs` (:122 + :163),
  `extract-work-items` (:448 + :350-351), and `init-jira` (:63 + :74) each
  hold multiple sites — a file where only one got the directive still passes.

- 🔴 **Test Coverage**: Configure and jira rewrites have no directive-presence assertion at all
  **Location**: Phase 2, Automated Verification
  The 9 configure sites (labeled `bash` fences, invisible to the guard) and
  the 3 jira sites get only structural-absence checks (`bash`-prefix gone,
  unbraced var gone, `VAR=$()` gone). Nothing asserts the `bash`/`sh`/`env`
  directive was actually added to any of these 12 in-scope sites.

#### Major

- 🟡 **Correctness / Test Coverage / Portability**: The `grep -v create-adr/SKILL.md:153` exclusion is a silent no-op
  **Location**: Phase 1, Automated Verification (directive-presence loop)
  `grep -rl` outputs filenames only, so the `:153` line-suffix filter never
  matches. The intent (skip the prose-only mention at create-adr:153) is not
  implemented; the filter is inert.

- 🟡 **Test Coverage**: The "executable specification" runs from an uncommitted `/tmp` file — not reproducible
  **Location**: Testing Strategy / Verification of end state
  `awk -f /tmp/fence-guard.awk` over an absent file produces empty output —
  indistinguishable from a genuine PASS. A fresh checkout, CI, or another
  reviewer cannot re-run the canonical test; it fails open.

- 🟡 **Test Coverage**: "Guard empty after" proves only fence removal, not the 21 directive/correctness edits
  **Location**: Implementation Approach / Phase 2 full-tree guard criterion
  The most-cited completion gate covers only the 7 fenced lines (lowest-risk
  quarter). It says nothing about the 12 inline directives, 9 configure
  rewrites, or the restructure.

- 🟡 **Documentation**: Finished after-text shown for only 2 of 12 heterogeneous inline sites
  **Location**: Phase 1 §2 (inline-site table)
  The 12 sites have varied lead-in verbs ("Invoke", "Run", "Run the … script",
  "Gather metadata using"). The Appended form begins "Run it…", producing
  "Run … . Run it directly…" double-verb awkwardness at sites like
  research-codebase:112 and create-note:81. No target text is given to check
  the "reads naturally" gate against.

- 🟡 **Documentation**: Split-passage sites cannot take the Appended form as written
  **Location**: Implementation Approach / Phase 1 §2 split-passage note
  research-issue:94 and extract-work-items:448 both have a trailing "to obtain
  …" purpose clause *after* the path, so appending the directive at sentence
  end separates it from the verb+path it governs by an intervening clause,
  making "it" ambiguous.

- 🟡 **Documentation / Standards / Architecture / Usability**: The convention has no codified home
  **Location**: Desired End State #4 / Migration Notes / What We're NOT Doing
  No skills-authoring guide, ADR, or shared note states the bare-path rule
  once; it exists only as 28 scattered copies. A wording change means editing
  28 sites; a new author has nothing to discover; the only stable greppable
  token is the `bash`/`sh`/`env` fragment, not a conforming directive.

- 🟡 **Correctness**: The extract-work-items restructure leaves the invocation shape unspecified
  **Location**: Phase 2 §3
  "Run `…config-read-work.sh id_pattern` and capture its output as PATTERN"
  gives no literal command form, so the model may realise it as `PATTERN=$(…)`
  (whose argv[0] is `PATTERN=…`, the exact escape being removed) or `bash
  <path>`. The change does not enforce the bare-path argv[0] it claims.

- 🟡 **Standards / Usability**: Two near-identical directive forms create drift and a choice burden
  **Location**: Implementation Approach: Canonical directive forms
  Block and Appended differ only cosmetically; an author must choose between
  them per site, inviting mixed forms or a third. A single canonical sentence
  + placement guidance would be more recognisable and linter-friendly.

- 🟡 **Architecture**: Durability depends entirely on out-of-scope 0107; this is a stopgap without it
  **Location**: Implementation Approach / What We're NOT Doing
  The plan's own logic is that the convention erodes — which is why 0107
  (lint guardrail) exists — yet 0107 is `blocked_by: 0106`, so no enforcement
  lands here. Committing the guard script as part of this plan, or relaxing
  the block to `relates_to`, would let protection arrive with (or before) the
  cleanup.

- 🟡 **Architecture**: Sole dependency on unversioned harness behaviour with no failover
  **Location**: Migration Notes
  The fix rests on the harness stripping only timeout/time/nice/nohup/stdbuf
  and prefix-matching the bare path. The work item hard-excludes the RCA's
  Option B (also authorize the wrapped form), removing the only redundancy and
  leaving a single point of failure outside the repo, detectable only as
  intermittent user prompts.

- 🟡 **Usability**: The actual prompt-elimination payoff is never re-validated at runtime
  **Location**: Verification of end state / What We're NOT Doing (Hypothesis-3)
  All gates are static, and the plan explicitly declines to investigate the
  work item's "first-call-only enforcement" quirk — which, if present, means
  the directives may not stop the prompts. The plan can pass every gate and
  still fail to deliver its sole motivating outcome.

- 🟡 **Portability**: The plan never states which OS/shell the verification must pass on
  **Location**: Testing Strategy / all Success Criteria
  Given the team's documented macOS bash 3.2 / BSD-userland history, a
  verification green on the author's machine may behave differently elsewhere.
  `grep -r --include=` is GNU-origin (BSD edge cases), and the embedded-backtick
  pattern `'bash`/`sh`/`env`'` is one quoting slip (single→double quotes) away
  from executing `sh`/`env` as commands.

#### Minor

- 🔵 **Correctness**: Off-by-one line-number disagreement between plan and work item for the ADR fences
  **Location**: References / vs work item Technical Notes
  Work item cites 124-126 / 120-122; the plan cites (correctly) 125-127 /
  121-123. The plan is right; the silent disagreement could mislead someone
  reconciling the two.

- 🔵 **Correctness**: Inline-path presence grep counts pre-existing occurrences too
  **Location**: Phase 1 Success Criteria (inline-code presence grep)
  extract-adrs already has the path inline at :163, so the `grep -c … ≥1`
  passes even if the :122 fence is not converted. Only the paired guard-empty
  check actually proves the fence is gone.

- 🔵 **Test Coverage**: extract-work-items restructure verified only by what it removes
  **Location**: Phase 2 Success Criteria (VAR=$() absence)
  Both gates are absence checks; nothing asserts the replacement carries the
  bare braced path, preserves capture semantics, or includes the directive.

- 🔵 **Standards**: Appended form deviates from the work item's mandated canonical template
  **Location**: Canonical directive forms vs work item §Requirements
  The Block form preserves the template near-verbatim; the Appended form
  rephrases it. Semantics match the minimum, but the single source of truth
  for phrasing is lost.

- 🔵 **Standards**: configure stays a fenced block while jira becomes inline — minor presentational split
  **Location**: Phase 2 §1
  The labeled-fence retention is justified by `<args>`/branching, but leaves
  configure as the only family presenting a single-script command as a fenced
  block (e.g. `templates list`).

- 🔵 **Documentation**: Repeated rationale clause is per-invocation token cost for the model
  **Location**: Phase 1 / Phase 2 directives
  The ~2-sentence rationale is re-tokenised every invocation, up to 9× in one
  file. Valuable for authors, arguably noise for the runtime model reader.

- 🔵 **Documentation**: Amended AC3 buries the checkable gate in dense explanatory/historical prose
  **Location**: Phase 3 §1 (AC3 blockquote)
  A ~90-word sentence mixes the gate, the exclusion, a cross-reference, the
  known-positive requirement, and a removal post-mortem. The "why removed"
  rationale also duplicates the Technical Notes entry the plan adds.

- 🔵 **Usability**: Prose "capture its output as PATTERN" is a less familiar instruction shape than the shell it replaces
  **Location**: Phase 2 §3
  Trades an unambiguous, copyable assignment idiom for prose the model must
  reconstruct into a rule-matching argv.

- 🔵 **Architecture**: One call site needs restructuring the convention does not cover
  **Location**: Phase 2 §3
  extract-work-items escapes the rule via an assignment prefix, not a `bash`
  prefix — revealing the directive models only one of several escape shapes
  (wrapper, assignment, quoting/bracing).

- 🔵 **Standards**: AC3 / 0107 supporting-note placement is unspecified by section name
  **Location**: Phase 3 §1 / §2
  "Open Questions / Technical Notes" (either/or) and "a note" leave placement
  to edit-time discretion rather than the established work-item section
  structure.

- 🔵 **Portability**: `grep -r --include=` is GNU-origin syntax with BSD edge cases
  **Location**: line 45 / Phase 1 / Phase 2 criteria
  Supported by modern macOS BSD grep, but older/edge behaviour differs;
  aligning supporting greps on `rg` (already a team tool) would remove the
  variable.

- 🔵 **Portability**: Amended AC3 still quotes the `rg --pcre2` regex, re-importing the PCRE2 dependency into the work item
  **Location**: Phase 3 §1
  The descriptive reference keeps a PCRE2-dependent command visible as prior
  art that 0107 could resurrect; `rg --pcre2` is not a guaranteed ripgrep
  build feature.

#### Suggestions

- 🔵 **Correctness**: Guard handles only triple-backtick fences (no 4+-backtick or tilde fences)
  **Location**: Testing Strategy (awk guard)
  Not an active defect (the corpus uses only plain triple fences), but worth
  documenting the assumption where the guard is handed to 0107.

### Strengths

- ✅ The 276-line grep decomposition (213 substitution + 35 rule + 14 + 14)
  is arithmetically exact and re-verified against the live tree at the pinned
  revision; create-adr:153 is correctly excluded as prose (it lacks the
  `scripts/` prefix).
- ✅ The fence-state awk guard is logically sound for the cases that occur:
  it recomputes labeled/unlabeled state on every opening fence, tracks
  arbitrary indentation, treats labeled ` ```bash ` fences as immune, and
  excludes `!`…`` substitutions — isolating exactly the 7 known-positive
  lines, and it uses only POSIX-portable awk (no gawk-isms).
- ✅ Correctly diagnoses the original AC3 regex as defective (errors without
  `--pcre2`; over-matches 14 files via inter-fence false positives with it)
  and replaces it with a known-positive/known-negative gate — a real
  improvement, and dropping `rg --pcre2` is the right portability call.
- ✅ Correctly identifies the configure cluster as a *content* bug (literal
  `bash ` prefix + unbraced `$CLAUDE_PLUGIN_ROOT`) invisible to the guard
  because its fences are labeled, and standardises it onto the dominant
  unquoted-braced shape (267 of 267 other body occurrences are braced).
- ✅ Phase decomposition is genuinely disjoint by file set; the plan is
  precise that the full-tree guard clears only after Phase 1 **and** Phase 2,
  and reasons the mid-rollout state rather than assuming it.
- ✅ Honest about its central tradeoff (duplicate-the-directive vs
  abstraction) and the external harness coupling, surfacing both in dedicated
  sections rather than hiding them.

### Recommended Changes

1. **Add occurrence-level directive-presence verification** (addresses:
   AC1 false-PASS; configure/jira have no assertion; extract-work-items
   verified only by removal). For each in-scope file, assert the count of the
   directive phrase is ≥ the count of model-issued script-path occurrences in
   that file, and extend this to the configure and jira files. Replace the
   file-level `grep -q` with this occurrence-level check, or explicitly demote
   it to a necessary-but-not-sufficient check backed by the manual same-passage
   review.

2. **Make the regression guard reproducible** (addresses: `/tmp` guard not
   reproducible; "guard empty" fails open). Inline the awk source as a heredoc
   in the verification step, or commit the guard script as part of this plan
   (a small scope addition that also de-risks the 0107 dependency). State that
   guard-empty proves AC2 only, and pair it with the AC1 occurrence-level gate.

3. **Fix the no-op `grep -v …:153` filter** (addresses: silent no-op flagged
   by 3 lenses). Drop it (create-adr legitimately needs a directive via its
   fence conversion) or, if prose-exclusion is genuinely wanted, filter on
   `grep -rn` `file:line` output. Use `grep -F` for the backtick phrase to
   make the embedded backticks unambiguous and bash-3.2-safe.

4. **Establish one canonical statement of the convention** (addresses: the
   "no codified home" theme across 4 lenses; two-forms drift; rationale token
   bloat). Add a short authoring note (or ADR) stating the bare-path rule once
   — including the generalised invariant "the invocation's first token must be
   the bare braced path" (covering assignment/substitution/quoting escapes,
   not just `bash`/`sh`/`env`). Pin a single directive sentence to reproduce
   verbatim at call sites, and have 0107 lint for that exact string. If 0107
   codification is the deliberate home, say so explicitly and cross-link it.

5. **Pin the under-specified edits with concrete after-text** (addresses:
   2-of-12 after-text shown; split-passage graft; extract-work-items shape).
   Show the rendered result for each distinct verb form and both split-passage
   sites (insert mid-sentence, not at end), and give extract-work-items a
   literal bare-path snippet alongside the "capture as PATTERN" prose so the
   model has the exact argv to reproduce.

6. **Close the runtime-payoff loop** (addresses: prompt-elimination never
   validated; Hypothesis-3 deferred). Add one lightweight live confirmation to
   success criteria — invoke one rewritten artifact site and one config site
   and confirm no prompt fires. If a prompt persists, make the Hypothesis-3
   spike blocking rather than deferred.

7. **State the verification environment and reconcile minor inconsistencies**
   (addresses: OS/shell unspecified; line-number drift; AC3 PCRE2 reference;
   AC3 prose density; note placement). Name the target shells/userlands the
   commands must pass on; note the work item's 124-126/120-122 figures are
   pre-drift; tell 0107 to avoid `--pcre2`; trim AC3 to the gate and move the
   removal rationale to a named Technical/Drafting Notes section.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan's core empirical claims are sound and verifiable: the
276-line decomposition (213+35+14+14) is arithmetically exact against the
live tree, the cited line numbers match the current SKILL.md files, the
unquoted-braced bare-path shape is internally consistent across all
rewrites, and the fence-state awk guard correctly tracks fence open/close,
labeled-vs-unlabeled state, and the `!`…`` exclusion to isolate exactly the 7
known-positive lines. The main correctness defects are in the verification
commands rather than the edits themselves: the Phase 1 directive-presence
grep contains a no-op filter and only checks presence per-file (not
per-occurrence as AC1 demands), and the extract-work-items restructure
replaces an executable command-substitution shape with prose that never
specifies a concrete invocation form.

**Strengths**:
- The 276-line grep decomposition is arithmetically exact and verified against the live tree at the pinned revision.
- The fence-state awk guard is logically correct for the cases it must handle (recomputes `unlabeled` on every opening fence, tracks indentation, treats labeled fences as immune, excludes `!`…`` substitutions).
- Correctly identifies the configure cluster as a content bug (literal `bash ` prefix + unbraced var) invisible to the guard because the fences are labeled.
- The unquoted-braced bare-path invariant is applied consistently across every rewrite.
- Correctly diagnoses that the original AC3 regex over-matches via inter-fence false positives and can never return empty.

**Findings**:
- 🟡 (major, high) **Directive-presence grep contains a no-op exclusion filter** — Phase 1 Success Criteria. `grep -rl` emits filenames only, so `grep -v …:153` matches nothing; the intent is unachievable with `-l` and excluding the whole create-adr file would be wrong (it has a real fence site).
- 🟡 (major, high) **Per-file presence grep cannot verify the per-occurrence "same passage" requirement** — Phase 1 / Desired End State #1 / AC1. Files with multiple sites (extract-adrs :122+:163, init-jira :63+:74) pass `grep -q` even if only one site got the directive.
- 🟡 (major, medium) **Replacing VAR=$(...) with prose leaves the invocation shape unspecified** — Phase 2 §3. The prose gives no literal command form; the model may re-emit `PATTERN=$(…)` (the exact escape) or `bash <path>`.
- 🔵 (minor, medium) **The VAR=$() absence check is trivially satisfied** — Phase 2 Success Criteria. A deletion check, not a correctness check; nothing validates the new shape.
- 🔵 (minor, high) **Off-by-one line-number disagreement between plan and work item** — References. Work item cites 124-126/120-122; plan cites (correctly) 125-127/121-123.
- 🔵 (minor, medium) **Inline-path presence grep counts pre-existing occurrences** — Phase 1 Success Criteria. extract-adrs already has the path inline at :163, so the presence grep passes even if the :122 fence is not converted.
- 🔵 (suggestion, low) **Guard does not handle >3-backtick or tilde fences** — Testing Strategy. Not an active defect (corpus uses only plain triple fences) but worth documenting where handed to 0107.

### Documentation

**Summary**: This plan is itself a documentation change: it edits runtime
instructional prose at 28 sites and amends two work items. The two canonical
directive forms are clear and the rationale is meaningful (it tells a future
author *why* the bare path matters), but the plan only shows after-text for 2
of the 12 heterogeneous inline sites, leaving grafting quality at 10 sites
unverified. The bigger documentation-strategy concern is that the convention
is repeated verbatim at 28 sites with no single canonical reference, which
both bloats runtime instructions and creates a drift surface with no source
of truth.

**Strengths**:
- The directive rationale is genuinely useful "why" documentation for a future skill author.
- Distinguishes Block form from Appended form and matches each to its structural context.
- Success Criteria include explicit manual readability checks.
- Terminology is consistent across plan body, directive forms, and the work-item amendment.

**Findings**:
- 🟡 (major, high) **Finished after-text shown for only 2 of 14 sites** — Phase 1 §2. Heterogeneous lead-in verbs make the "Run it…" appended form read as double-verb ("Run … . Run it directly…") at research-codebase:112 and create-note:81.
- 🟡 (major, high) **Split-passage sites cannot take the Appended form as written** — Canonical directive forms / split-passage note. research-issue:94 and extract-work-items:448 have a trailing "to obtain …" clause after the path, separating the directive from its referent.
- 🟡 (major, medium) **Convention documented only by verbatim repetition; no canonical reference** — Canonical directive forms / Migration Notes. No prose home for the rule itself; "the convention" is 28 scattered copies.
- 🔵 (minor, medium) **Repeated rationale clause is per-invocation token cost for the model** — Phase 1/2 directives. Re-tokenised every invocation, up to 9× in configure.
- 🔵 (minor, medium) **jira create-jira-issue:63 after-text not shown** — Phase 2 §2. Surrounding sentence structure differs from init-jira; graft may read awkwardly.
- 🔵 (minor, high) **Amended AC3 buries the checkable gate in dense prose** — Phase 3 §1. ~90-word sentence mixes gate, exclusion, cross-reference, known-positive, and removal post-mortem (which duplicates the Technical Notes entry).

### Test Coverage

**Summary**: The plan's verification strategy is strongest where it is least
needed (the 7 bare-fence sites, which the awk guard covers well) and weakest
where the real risk lives (the 21 directive-add sites the guard cannot see).
AC1 "every occurrence carries a directive" is verified only by file-level
`grep -q`, a structural false-PASS for multi-site files. Phase 2's
correctness rewrites have no directive-presence assertion at all, and the
guard's reliance on an uncommitted `/tmp/fence-guard.awk` undermines
reproducibility of the very test the plan calls its executable specification.

**Strengths**:
- The fence-state guard is a genuine improvement over the defective AC3 regex; demonstrated as a known-positive (7 lines / 5 files) and known-negative.
- Correctly recognises the guard verifies only AC2 and supplements it with separate greps for inline/labeled-fence sites.
- Phase 2's structural checks are precise and verified against the live tree.
- Phase 3's regression checks are sound (the broken look-ahead literal exists exactly once today).

**Findings**:
- 🔴 (critical, high) **File-level directive grep cannot prove per-occurrence coverage (AC1 false-PASS)** — Phase 1 & Phase 2 Success Criteria. Multi-site files (extract-adrs, extract-work-items, init-jira) pass even when half-covered.
- 🔴 (critical, high) **Configure and jira rewrites have no directive-presence assertion at all** — Phase 2 Automated Verification. 12 of 28 in-scope sites have AC1 entirely unverified; only structural-absence checks exist.
- 🟡 (major, high) **The "executable specification" lives in an uncommitted /tmp file — not reproducible** — Testing Strategy. `awk -f /tmp/fence-guard.awk` over an absent file produces empty output, indistinguishable from PASS; fails open.
- 🟡 (major, high) **"Guard empty after" proves only fence removal, not the 21 directive/correctness edits** — Implementation Approach / Phase 2. The headline gate covers the lowest-risk quarter.
- 🔵 (minor, high) **The create-adr:153 exclusion filter in the directive grep is a no-op** — Phase 1 Success Criteria.
- 🔵 (minor, medium) **extract-work-items restructure verified only by what it removes** — Phase 2 Success Criteria. Absence checks only; nothing asserts the replacement shape/semantics/directive.

### Standards

**Summary**: The plan is well-grounded in the project's actual conventions:
it correctly identifies the dominant unquoted-braced `${CLAUDE_PLUGIN_ROOT}`
form (267/267 body occurrences are braced; only configure's 9 sites are the
outlier) and standardises configure onto it. Its two directive forms are
faithful to the work item's mandated minimum. The principal weakness is that
the convention is being baked into 28 hand-edited passages with no codified
home, and the directive wording diverges both from the work item's mandated
template and across its own two variants.

**Strengths**:
- Correctly identifies and standardises on the dominant invocation shape (267 braced body occurrences).
- Inline-code vs labeled-fence treatment is consistent with existing authoring and principled.
- Both directive variants preserve the three-wrapper minimum and "run directly" instruction.
- Phase 3 keeps the AC3 amendment tightly scoped with traceability notes.

**Findings**:
- 🟡 (major, high) **Two directive variants appended to heterogeneous sentences yield 28 different phrasings** — Canonical directive forms. The only stable greppable token is the `bash`/`sh`/`env` fragment; the coverage check implicitly concedes this.
- 🟡 (major, medium) **The convention has no codified home** — Desired End State #4 / Phase 3. No skills-authoring guide or CLAUDE.md under the skills tree; codification deferred to 0107.
- 🔵 (minor, high) **Appended form deviates from the work item's mandated canonical template** — Canonical forms vs work item §Requirements.
- 🔵 (minor, medium) **configure stays a fenced block while jira becomes inline** — Phase 2 §1. Minor presentational inconsistency for structurally-similar "run one script" cases.
- 🔵 (suggestion, medium) **AC3 / 0107 note placement is unspecified by section name** — Phase 3 §1/§2.

### Architecture

**Summary**: A well-researched prose-layer fix that correctly diagnoses the
root cause and is honest about its central tradeoff: the cross-cutting fix is
implemented by duplicating a directive across 28 call sites because the
harness offers no mechanism to share one instruction across skills. The
architecture is a convention enforced by repetition with no in-repo
enforcement (0107 is out of scope), so its evolutionary fitness depends
entirely on an external, unversioned harness behaviour and on a future
guardrail that does not yet exist.

**Strengths**:
- Explicitly acknowledges the duplicate-vs-abstraction tradeoff and harness coupling in dedicated sections.
- Replaces the defective AC3 regex with a verified known-positive/known-negative fence-state parser.
- Phase decomposition is genuinely disjoint; the guard-clears-only-after-both reasoning is explicit.
- Respects the `!`…`` substitution form as the plugin's structural defence (213 lines correctly excluded).
- The configure-cluster rewrite resolves a real self-contradiction.

**Findings**:
- 🔴 (major, high) **Cross-cutting concern implemented by 28-site duplication with no shared abstraction or in-repo enforcement** — Overview / Canonical forms / Migration Notes. The lowest-cohesion structure for an invariant that must hold identically at every site; erosion is invisible until a runtime prompt.
- 🟡 (major, high) **Durability depends entirely on out-of-scope 0107; this is a stopgap absent it** — Implementation Approach / What We're NOT Doing. 0107 is `blocked_by: 0106`, so no enforcement lands; consider committing the guard or relaxing the block to `relates_to`.
- 🟡 (major, medium) **Sole dependency on an unversioned harness behaviour with no failover or detection** — Migration Notes. The work item hard-excludes the RCA's Option B redundancy, leaving a single point of failure outside the repo.
- 🔵 (minor, high) **One call site needs structural restructuring the convention does not cover** — Phase 2 §3. extract-work-items escapes via an assignment prefix, revealing the directive models only one escape shape.

### Usability

**Summary**: The plan's two "users" are the model executing skills at runtime
and the human author maintaining them. The plan is rigorous about static
end-state verification but never closes the loop on the actual DX payoff —
eliminating spurious permission prompts — and explicitly declines to
investigate a known residual quirk (Hypothesis-3) that could mean the prompts
persist. It also institutionalises a verbose, copy-pasted directive at 28
sites with two near-identical forms and no durable authoring guidance.

**Strengths**:
- The directive content is actionable for the model (names prohibited wrappers + states the concrete benefit).
- Phasing is disjoint-by-file and independently mergeable, lowering cognitive load.
- Preserves the exact rule-matching shape and calls out failure-mode variants.
- Replacing the defective regex and propagating the warning to 0107 prevents inheriting a check that can never return empty.

**Findings**:
- 🟡 (major, high) **Plan never re-validates the actual prompt-elimination payoff at runtime** — Verification of end state / Hypothesis-3 exclusion. Can pass all gates and still fail the sole motivating outcome.
- 🟡 (major, high) **No durable authoring convention — the rule lives only as 28 scattered copies** — What We're NOT Doing / Migration Notes. Erodes on the next edit before 0107 ships.
- 🔵 (minor, medium) **Two near-identical directive forms create an unnecessary choice burden** — Canonical directive forms.
- 🔵 (minor, medium) **Repeating the full rationale at every site costs model tokens on every invocation** — Phase 1 §2 / Phase 2 §1.
- 🔵 (minor, medium) **Prose "capture its output as PATTERN" is a less familiar instruction shape than the shell it replaces** — Phase 2 §3.

### Portability

**Summary**: The plan's verification rests entirely on shell commands that
must run on a team with documented macOS bash 3.2 / BSD-userland gotchas, yet
it never states which environment(s) the verification must pass on. The core
awk fence-guard is genuinely portable (POSIX-only, no gawk-isms), and dropping
the rg --pcre2 regex is the right call. However, several verification commands
carry latent BSD-vs-GNU and embedded-backtick fragility.

**Strengths**:
- The fence-guard awk uses only POSIX-portable constructs; behaves identically on macOS one-true-awk and GNU gawk.
- Removing the rg --pcre2 regex eliminates a hard PCRE2 build dependency.
- `-print0 | xargs -0` correctly handles arbitrary filenames across BSD/GNU.
- `/tmp/fence-guard.awk` is portable across the team's macOS and Linux.

**Findings**:
- 🟡 (major, high) **Plan never specifies which OS/shell the verification must pass on** — Verification of end state / all Success Criteria. Given the BSD-vs-GNU history, results may differ across machines/CI.
- 🟡 (major, high) **Directive-presence for-loop is broken regardless of OS and the embedded-backtick pattern needs explicit single-quoting validation** — Phase 1 line 241. The `:153` exclusion is a no-op; `'bash`/`sh`/`env`'` is one quoting slip from executing `sh`/`env`.
- 🔵 (minor, medium) **grep -r --include= is GNU-origin syntax with BSD edge cases** — line 45 / Phase 1 / Phase 2. Aligning supporting greps on `rg` would remove the variable.
- 🔵 (minor, high) **Amended AC3 still references the rg --pcre2 regex, re-importing the PCRE2 dependency into the work item** — Phase 3 §1. Risks accidental reuse in the committed 0107 guardrail.

## Re-Review (Pass 2) — 2026-06-11

**Verdict:** APPROVE

Re-ran all 7 lenses against the edited plan. **Every finding from Pass 1 is resolved or
consciously accepted with documented rationale.** The edits replaced the file-level
directive checks with occurrence/passage-level gates, collapsed the two directive forms
into one verbatim canonical sentence (with a fixed fragment 0107 will lint), made the
guard a self-contained heredoc framed as AC2-only, added per-family AC1 directive counts,
pinned the extract-work-items shape with inline snippets, and documented the harness
coupling / 0107 deferral / dropped-Option-B as deliberate tradeoffs. The re-review surfaced
one new major (an over-strict `-eq 5` configure count) and a handful of minors/suggestions;
the major and the high-value minors were fixed in the same pass, and the guard was hardened
(per-file state reset + POSIX `[[:blank:]]`) and re-verified to still isolate exactly the 7
known-positive lines.

### Previously Identified Issues

- 🔴 **Test Coverage**: File-level directive grep cannot prove per-occurrence coverage — **Resolved** (occurrence-level loop `dir ≥ occ`; extract-adrs now requires 2 directives; verified).
- 🔴 **Test Coverage**: Configure/jira have no directive-presence assertion — **Resolved** (configure `-ge 5`, init-jira `-ge 2`, create-jira-issue `-ge 1`, extract-work-items positive assertion).
- 🟡 **Correctness / Test Coverage / Portability**: `grep -v …:153` no-op filter — **Resolved** (removed; `scripts/`-anchored pattern excludes the prose mention naturally).
- 🟡 **Test Coverage**: Guard in uncommitted `/tmp` fails open — **Resolved** (self-contained heredoc).
- 🟡 **Test Coverage**: "Guard empty" proves only fences — **Resolved** (reframed AC2-only; separate AC1 grep gate).
- 🟡 **Documentation**: After-text shown for only 2 of 12 sites — **Resolved** (rendered exemplar per verb form).
- 🟡 **Documentation**: Split-passage graft — **Resolved** (canonical sentence is standalone; placement rule 3).
- 🟡 **Documentation / Standards / Architecture / Usability**: No codified convention home — **Resolved (deliberate deferral)** (single fixed string + 0107 cross-link, documented as accepted gap).
- 🟡 **Correctness**: extract-work-items shape unspecified — **Resolved** (inline bare-path snippets + capture prose + `VAR=$()` prohibition).
- 🟡 **Standards / Usability**: Two directive forms — **Resolved** (collapsed to one).
- 🟡 **Architecture**: Durability depends on out-of-scope 0107 — **Accepted** (documented as deliberate gap; user chose to keep 0107 deferred).
- 🟡 **Architecture**: Harness coupling / dropped Option B — **Accepted** (documented as deliberate policy tradeoff with escalation path).
- 🟡 **Usability**: Runtime payoff never validated — **Accepted** (user chose static-only; escalation path + observable symptom now documented).
- 🟡 **Portability**: OS/shell unspecified — **Resolved** (Verification environment subsection).
- 🟡 **Portability**: Broken for-loop + backtick fragility — **Resolved** (`grep -F`, no-op removed; validated under zsh).
- 🔵 Minors (line-number drift, inline-presence grep, removal-only check, AC3 density, note placement, `grep -r --include=`, lingering `--pcre2` reference) — **All Resolved** (drift note added, presence grep annotated, positive assertion added, AC3 trimmed, named Drafting Notes, environment note with `rg` fallback, explicit `--pcre2` prohibition to 0107).
- 🔵 Usability F4 (per-site rationale token cost) — **Accepted by design** (one form now; flagged for 0107 to revisit once a shared home exists).

### New Issues Introduced (and dispositions)

- 🟡 **Test Coverage**: configure `-eq 5` exact count false-fails legitimate over-coverage — **Fixed** (changed to `-ge 5`; prose updated to floor semantics; placement left to the named manual gate).
- 🔵 **Correctness / Test Coverage**: awk guard leaked fence state across files (no per-file reset) — **Fixed** (added `FNR==1 { in_fence=0; unlabeled=0 }`; re-verified 7 known-positives).
- 🔵 **Portability**: `\t` in awk regex relied on escape-handling — **Fixed** (switched to POSIX `[[:blank:]]`; re-verified).
- 🔵 **Documentation / Standards**: canonical sentence double-verbs against "Run" hosts — **Addressed** (noted as intentional; "must not be paraphrased"; manual check accepts the cadence).
- 🔵 **Standards**: plan claims to supersede the work-item template but didn't record it in 0106 — **Fixed** (added a second Drafting Note to 0106 in Phase 3 §1).
- 🔵 **Usability**: escalation path was reactive with no signal — **Addressed** (added a concrete observable symptom to Migration Notes).
- 🔵 **Test Coverage**: `grep -vc '!`'` is a no-op against current data; reset step-5 placement is manual-only; **Architecture**: general invariant lives only in Migration Notes; **Usability**: step-b stacks prohibitions; **Portability**: unquoted `for` word-splitting — **Accepted** (all low-severity, on a space-free tree / inherent to the accepted tradeoffs; no churn warranted).

### Assessment

The plan is in good shape and ready for implementation. The verification strategy now
genuinely gates both acceptance criteria (AC1 via occurrence/passage-level directive
counts, AC2 via the hardened self-contained guard), the convention is a single
linter-matchable string, and the architectural/usability tradeoffs the plan cannot resolve
within its scope are explicitly accepted and documented with escalation paths. Residual
items are minor and either fixed or consciously accepted. No critical or major findings
remain open.

---
*Re-review generated by /accelerator:review-plan*

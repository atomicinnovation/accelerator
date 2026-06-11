---
type: plan
id: "2026-06-11-0106-invoke-plugin-scripts-by-bare-path"
title: "Invoke Plugin Scripts by Bare Path in Skill Bodies Implementation Plan"
date: "2026-06-11T13:38:03+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0106"
parent: "work-item:0106"
derived_from: ["codebase-research:2026-06-11-0106-bare-path-script-invocation-call-sites"]
relates_to: ["work-item:0107"]
tags: [permissions, allowed-tools, skills, plugin, authoring-convention]
revision: "b2f3fafe9a2e381b039648f551640426cd6c915f"
repository: "miscellaneous"
last_updated: "2026-06-11T16:31:31+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Invoke Plugin Scripts by Bare Path in Skill Bodies Implementation Plan

## Overview

Skills repeatedly prompt for permission to run `artifact-derive-metadata.sh` and
sibling `artifact-*`/`config-*` scripts because the model wraps the invocation as
`bash <path>`. The permission matcher only strips `timeout`/`time`/`nice`/`nohup`/`stdbuf`
as recognised wrappers — **not** `bash`/`sh`/`env` — so a `bash`-prefixed command no
longer begins with the authorized path and escapes the `allowed-tools` rule
`Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)` / `…/config-*`.

The skill bodies state *what* to run but never *how*, leaving the invocation shape to
model discretion — and the dominant prior for "run a `.sh` file" is `bash script.sh`.
This plan closes the gap at every genuinely model-issued call site by adding a
bare-path directive (naming `bash`/`sh`/`env` as prohibited), converting the
amplifying bare code fences to inline code, and rewriting the one place where the
anti-pattern is literally authored into the text (`configure/SKILL.md`). It also
replaces the work item's defective AC3 regression regex with a verified
fence-state guard.

## Current State Analysis

The work item names the output of
`grep -rn 'scripts/\(artifact\|config\)-' --include=SKILL.md skills/` as the
"denominator against which 'every call site' is verified." Run against the live tree
(revision `10f2286ee7e0e979117e1afbc2a2b4ceb0e36efe`) it returns **276 lines**, which
decompose exactly as the research established:

| Category | Count | Directive applies? |
|---|---|---|
| `!`…`` load-time command substitutions | 213 | **No** — harness-executed at skill-load time; the model never re-issues them, so they cannot be `bash`-wrapped. |
| Frontmatter `allowed-tools` rule lines (`- Bash(…)`) | 35 | **No** — permission rules, not invocations; explicitly out of scope. |
| `artifact-derive-metadata.sh` model-issued invocations | 14 | **Yes** — the RCA-evidenced core target. |
| `config-*` model-issued invocations | 14 | **Yes** — the unanticipated residual cluster. |

(The raw `artifact-derive-metadata` grep returns 15 hits, but `create-adr/SKILL.md:153`
is descriptive prose — "`artifact-derive-metadata.sh` is the source for `date:`…" — not
an invocation, leaving 14 real invocations.)

So the **directive-requiring set is 28 model-issued occurrences across ~17 files**, not
276. The `grep` is the *discovery superset*; the directive applies only to the
model-issued subset. AC1's literal "every occurrence" must be read against this filtered
subset, not the raw 276 lines.

### Key Discoveries

- **AC3's regression regex is defective** (verified empirically against the live tree):
  `rg -U '```\n(?:(?!```)[\s\S])*scripts/(?:artifact|config)-[\s\S]*?```' skills/`
  errors with exit 2 without `--pcre2` ("look-around … is not supported"), and **with**
  `--pcre2` it over-matches **14 files** via inter-fence false positives — the leading
  `` ``` `` cannot distinguish an opening from a closing fence, so the match traverses
  ordinary prose (including `!`…`` substitution lines and inline references) until it
  reaches a script path. It matches the 12 non-target files *before any edit*, so it
  can never return empty and "empty proves cleanliness" is false. **It must be replaced.**
- **A sound replacement is verified** (this plan, §Testing Strategy): a fence-state awk
  parser that tracks opening/closing fences + language tags and excludes `!`…`` lines.
  Against the current tree it isolates **exactly 7 lines across 5 files** (the real
  bare-fence targets), and will return empty once those fences are converted.
- **The 14 artifact sites carry no existing directive.** Two are bare unlabeled fences
  (`create-adr/SKILL.md:125-127`, `extract-adrs/SKILL.md:121-123`); twelve are inline.
  Full enumeration in the §Phase 1 table.
- **The `config-*` cluster has three shapes**, all verified by direct read:
  - `config/configure/SKILL.md` (9 sites, lines 831/841/853/862/868/875/891/907/918):
    authored as `bash "$CLAUDE_PLUGIN_ROOT/scripts/config-…-template.sh" <args>` inside
    ` ```bash ` fences — **the bug is literally hard-coded** (a `bash` prefix *and* an
    unbraced `$CLAUDE_PLUGIN_ROOT`, both of which break the braced rule match). Because
    the fences are *labeled* (` ```bash `), the regression guard never flags them — this
    is a correctness rewrite, not a fence conversion.
  - jira `init-jira/SKILL.md:63,74` and `create-jira-issue/SKILL.md:63` (3 sites):
    bare-path config in **bare unlabeled fences** — correct shape, but the fence is the
    amplifier. Flagged by the guard.
  - `work/extract-work-items/SKILL.md:350-351` (2 sites): `VAR=$(…config-read-work.sh …)`
    command-substitution assignment in a bare fence. argv[0] is `PATTERN=…`, **not** the
    path, so this escapes the rule via the assignment prefix, not a `bash` prefix — the
    templated directive does not address it; it needs restructuring. Flagged by the guard.
- **The proven-matching invocation shape is the *unquoted, braced* bare path**
  (`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh …`), as used by the jira and
  artifact sites today. Rewrites must reproduce this exact shape — **not** a quoted
  (`"${CLAUDE_PLUGIN_ROOT}/…"`) or unbraced (`$CLAUDE_PLUGIN_ROOT`) form, either of which
  fails the prefix match against `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`.

## Desired End State

1. Every model-issued `artifact-*`/`config-*` invocation in a skill body (the 28-site
   subset) carries a **conforming directive** in the same passage — one that (a) names
   `bash`/`sh`/`env` as prohibited and (b) instructs that the bare path is run directly
   as an executable.
2. `create-adr/SKILL.md` and `extract-adrs/SKILL.md` contain the script path as inline
   code; no bare unlabeled fence houses an `artifact-*`/`config-*` path anywhere in
   `skills/` (verified by the fence-state guard returning empty).
3. The `configure` cluster uses the unquoted braced bare path with no `bash` prefix, so
   the body no longer contradicts its own directive and the invocations match the rule.
4. Work item `0106` AC3 is amended to reference the verified fence-state guard instead of
   the broken regex; `0107` carries a note that it inherits and must adopt the corrected
   guard.
5. No `allowed-tools` frontmatter is modified anywhere.

### Verification of end state

- `find skills -name SKILL.md -print0 | xargs -0 awk -f /tmp/fence-guard.awk` → **empty**.
- A presence grep confirms inline script paths in the two ADR files; the guard confirms
  no bare fence remains in them.
- A directive-presence grep confirms each edited file carries the `bash`/`sh`/`env`
  prohibition phrase.

## What We're NOT Doing

- **Not** modifying any `allowed-tools` frontmatter rule (work item constraint).
- **Not** adding directives to the 213 `!`…`` substitution lines or the 35 rule lines —
  they are structurally immune / out of scope.
- **Not** converting the labeled ` ```bash ` fences in `configure` to inline code — they
  carry `<args>` placeholders and branching, so they stay as labeled fences (the guard
  ignores labeled fences); only their *content* is corrected.
- **Not** touching the adjacent out-of-scope `adr-next-number.sh` bare fence in
  `extract-adrs/SKILL.md:148-150` (not an `artifact-*`/`config-*` path).
- **Not** building the automated lint guardrail — that is work item `0107`. This plan's
  guard is a manual verification command, not a committed script.
- **Not** investigating the RCA's Hypothesis-3 "first-call-only enforcement" quirk (a
  non-blocking residual risk noted in the work item).

## Implementation Approach

Test-first: the regression guard (§Testing Strategy) is the executable specification.
It is confirmed today as a **known-positive** (7 lines / 5 files) and must reach
**empty** after the fence conversions land. Each phase is scoped to a disjoint set of
files so the three phases are **independently integratable/mergeable** in any order:

- **Phase 1** — artifact files only (`decisions/`, `planning/`, `research/`, `github/`,
  `notes/`, `work/{create,review,extract}-work-item(s)`).
- **Phase 2** — config files only (`config/configure`, `integrations/jira/*`,
  `work/extract-work-items` config block).
- **Phase 3** — `meta/work/0106` and `meta/work/0107` only.

The guard returns true-empty only once Phase 1 **and** Phase 2 land, but each phase is a
coherent, non-breaking change on its own (e.g. Phase 1 alone removes the two ADR bare
fences; the guard would still report the config fences mid-rollout, which is expected).

### The canonical directive (one fixed sentence)

There is **one** conforming directive, reproduced **verbatim** at every site (adapted
from the work item template's three-wrapper form). It is a self-contained imperative
sentence — its wording never varies; only its *placement* does:

> Run the bare path **directly** as an executable; never prefix it with `bash`/`sh`/`env`
> (a wrapper prefix escapes the skill's `allowed-tools` permission and forces an
> unnecessary prompt).

**Placement rules** (wording is identical in every case):

- **Standalone step / converted fence** — the sentence follows the inline-code path in
  the same step (e.g. "Gather metadata by running `…artifact-derive-metadata.sh`. Run the
  bare path **directly** as an executable; …").
- **Existing inline "Invoke/Run `<path>`…" site** — append the sentence as the next
  sentence of the same passage (bullet/paragraph). Because it is a standalone sentence it
  grafts onto any lead-in verb ("Invoke", "Run", "Gather metadata using") without
  rephrasing the host.
- **Split-passage sites** where a purpose clause ("…to obtain `X`") follows the path
  (`research-issue:93-95`, `extract-work-items:447-450`) — place the directive as the
  **next sentence after the clause that names the path**, so the verb, path, and directive
  stay in one passage. Do not splice it mid-sentence.

Reproduce the sentence exactly — work item `0107`'s lint will match the fixed fragment
`never prefix it with `bash`/`sh`/`env``, so any paraphrase would evade the future guard.
This single fixed string (not a fuzzy paraphrase) is what makes the convention both
recognisable to a human author and greppable by tooling.

This sentence is the **authoritative** wording for the convention; it supersedes the
work item's illustrative blockquote template (which used a two-sentence "Do **not** prefix
the invocation with…" phrasing). Both satisfy the work item's conformance minimum — name
`bash`/`sh`/`env` as prohibited and instruct the bare path is run directly — but only this
fixed string is reproduced verbatim across all sites and matched by 0107.

On host sites whose own lead-in verb is "Run" (the "Run" and "Run the … script" forms, the
`configure` intros, and the `extract-work-items` bullets), the result reads "Run X. Run the
bare path **directly** …". This repeated-verb cadence is **intentional and must not be
paraphrased away** — the verbatim string is what 0107's lint matches. The "reads naturally"
manual check (below) accepts this cadence rather than inviting a local reword.

---

## Phase 1: Artifact call sites + ADR fence conversions

### Overview

Add a conforming directive to all 14 `artifact-derive-metadata.sh` model-issued sites,
converting the two bare fences to inline code in the same edit.

### Changes Required

#### 1. Convert the two bare fences to inline code (AC2 targets)

**File**: `skills/decisions/create-adr/SKILL.md` (fence open 125 / path 126 / close 127)

Before:

    1. **Gather metadata** by running:

    ```
    ${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh
    ```

After:

    1. **Gather metadata** by running
       `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`. Run the bare path
       **directly** as an executable; never prefix it with `bash`/`sh`/`env` (a wrapper
       prefix escapes the skill's `allowed-tools` permission and forces an unnecessary
       prompt).

**File**: `skills/decisions/extract-adrs/SKILL.md` (3-space-indented fence open 121 /
path 122 / close 123) — same conversion, preserving the list-continuation indent.

#### 2. Append the directive to the 12 inline artifact sites

For each, append the canonical directive (above) as the next sentence of the existing
passage. Because the directive is a self-contained sentence, it grafts onto every lead-in
verb without rephrasing the host:

| # | File:line | Existing verb |
|---|---|---|
| 1 | `decisions/extract-adrs/SKILL.md:163` | "Invoke … once for the batch" |
| 2 | `github/describe-pr/SKILL.md:105` | "Invoke" |
| 3 | `github/review-pr/SKILL.md:468` | "Invoke" |
| 4 | `notes/create-note/SKILL.md:81` | "Run" |
| 5 | `planning/create-plan/SKILL.md:230` | "Invoke" |
| 6 | `planning/review-plan/SKILL.md:430` | "Invoke" |
| 7 | `planning/validate-plan/SKILL.md:150` | "Invoke" |
| 8 | `research/research-codebase/SKILL.md:112` | "Run the … script" |
| 9 | `research/research-issue/SKILL.md:94` | "Gather metadata using" |
| 10 | `work/create-work-item/SKILL.md:440` | "Invoke" |
| 11 | `work/extract-work-items/SKILL.md:448` | "Invoke" (verb on :447) |
| 12 | `work/review-work-item/SKILL.md:360` | "Invoke" |

**Rendered after-text, one exemplar per distinct verb form** (apply the same shape to the
remaining sites of that form):

- *"Invoke" form* (#2, #3, #5, #6, #7, #10, #12) — e.g. `create-plan:230`:
  > Invoke `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh` to capture the
  > provenance bundle. Run the bare path **directly** as an executable; never prefix it
  > with `bash`/`sh`/`env` (a wrapper prefix escapes the skill's `allowed-tools`
  > permission and forces an unnecessary prompt).
- *"Run" form* (#4) — `create-note:81`:
  > Run `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh` to obtain the current
  > date/time. Run the bare path **directly** as an executable; never prefix it with
  > `bash`/`sh`/`env` (a wrapper prefix escapes the skill's `allowed-tools` permission and
  > forces an unnecessary prompt).
- *"Run the … script" form* (#8) — `research-codebase:112`:
  > Run the `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh` script to generate
  > all relevant metadata. Run the bare path **directly** as an executable; never prefix
  > it with `bash`/`sh`/`env` (a wrapper prefix escapes the skill's `allowed-tools`
  > permission and forces an unnecessary prompt).
- *Split-passage forms* (#1, #9, #11) where a "…to obtain `X`" clause follows the path —
  the directive becomes the **next sentence after** that clause, keeping verb, path, and
  directive in one passage. E.g. `research-issue:93-95`:
  > Gather metadata using `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh` to
  > obtain `Current Date/Time (UTC):`, `Current Revision:`, and `Repository Name:`. Run
  > the bare path **directly** as an executable; never prefix it with `bash`/`sh`/`env`
  > (a wrapper prefix escapes the skill's `allowed-tools` permission and forces an
  > unnecessary prompt).

The two fence conversions above (`create-adr:126`, `extract-adrs:122`) are the 13th and
14th artifact sites — 14 total.

### Success Criteria

#### Automated Verification

- [x] **(AC2 — fence absence)** No bare fence houses an artifact path in the two ADR
      files (guard reports neither `create-adr` nor `extract-adrs`):
      `find skills -name SKILL.md -print0 | xargs -0 awk -f /tmp/fence-guard.awk | grep -E 'create-adr|extract-adrs'` → empty.
      The guard (see Testing Strategy) is the **authoritative** absence proof for AC2.
- [x] **(AC2 — inline presence, necessary-but-not-sufficient)** Both ADR files contain the
      path as inline (backtick-delimited) code:
      `grep -c '`${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`' skills/decisions/create-adr/SKILL.md skills/decisions/extract-adrs/SKILL.md` → ≥1 each.
      Note: `extract-adrs` already carries an inline occurrence at `:163`, so this presence
      grep can pass even if the `:122` fence were left unconverted — it is the paired
      guard-empty check above that actually proves the fence is gone. Read the two together.
      (Implementation note: the `${…}`/`.` make this an unreliable regex that matched 0 on
      both BSD and GNU grep; use `grep -Fc` to verify the fixed string — create-adr=1,
      extract-adrs=2. Condition satisfied; the bare `grep -c` form is buggy.)
- [x] **(AC1 — per-occurrence directive coverage)** Every model-issued artifact occurrence
      carries the canonical directive, verified at **occurrence** granularity (a file with
      two sites must carry two directives — `extract-adrs` has `:122` and `:163`). The
      occurrence pattern `scripts/artifact-derive-metadata\.sh` is anchored on `scripts/`,
      so it never matches the descriptive prose mention at `create-adr:153` (which lacks the
      `scripts/` prefix) — no exclusion filter is needed:
      ```
      for f in $(grep -rl 'scripts/artifact-derive-metadata\.sh' --include=SKILL.md skills/); do
        occ=$(grep 'scripts/artifact-derive-metadata\.sh' "$f" | grep -vc '!`')
        dir=$(grep -Fc 'never prefix it with `bash`/`sh`/`env`' "$f")
        [ "$dir" -ge "$occ" ] || echo "UNDER-COVERED: $f (dir=$dir < occ=$occ)"
      done
      ```
      → no output. (`grep -Fc` matches the canonical fixed fragment literally, so the
      embedded backticks are unambiguous and bash-3.2-safe.)
- [x] No `allowed-tools` line changed in any edited file (review `jj diff` — only body
      prose differs).

#### Manual Verification

- [x] Each converted/appended passage reads naturally with the directive in place.
- [x] The split-passage sites (`research-issue:93-94`, `extract-work-items:447-448`) keep
      the verb, path, and directive in one coherent passage.

---

## Phase 2: config-* comprehensive rewrite

### Overview

Eliminate every model-issued `config-*` anti-pattern: rewrite the 9 `bash`-prefixed
`configure` examples to the unquoted braced bare path, convert the 3 jira bare fences to
inline, and restructure the 2 `extract-work-items` `VAR=$()` sites.

### Changes Required

#### 1. `config/configure/SKILL.md` — 9 sites (correctness rewrite)

For each of lines 831, 841, 853, 862, 868, 875, 891, 907, 918: drop the `bash ` prefix
and the surrounding quotes, and brace the variable. The fence stays ` ```bash `; add the
canonical directive to the prose introducing the subsection. Example (`templates list`,
:829-832):

Before:

    Run the list script and display its output:

    ```bash
    bash "$CLAUDE_PLUGIN_ROOT/scripts/config-list-template.sh"
    ```

After:

    Run the list script and display its output. Run the bare path **directly** as an
    executable; never prefix it with `bash`/`sh`/`env` (a wrapper prefix escapes the
    skill's `allowed-tools` permission and forces an unnecessary prompt):

    ```bash
    ${CLAUDE_PLUGIN_ROOT}/scripts/config-list-template.sh
    ```

Apply the same transform to the remaining 8 blocks (preserving each block's `<args>` /
`<key|--all>` placeholders).

**Directive placement — one per `####` command subsection (5 total).** The 9 blocks group
under 5 subsections, and a single directive in each subsection's intro prose is "same
passage" for every block beneath it:

| Subsection | Blocks (lines) | Directive location |
|---|---|---|
| `templates list` | 831 | intro prose (shown above) |
| `templates show <key>` | 841 | intro prose |
| `templates eject` | 853, 862, 868, 875 (4 blocks, one passage) | one directive at the subsection head covers all four |
| `templates diff <key>` | 891 | intro prose |
| `templates reset <key>` | 907 (step 2), 918 (step 5) | one directive in the subsection's intro paragraph (`:899-902`) governs both steps of the single reset procedure |

So `configure` gains **at least 5** canonical directives — one per subsection — verified
by a floor count (`-ge 5`) in Success Criteria. A floor rather than an exact count permits
a legitimate extra directive (e.g. a second one placed nearer reset's distant step 5),
which a placement-correct edit might add; the count cannot prove placement, so the
grouped-passage proximity (the eject group of four and the two reset steps) is confirmed by
the named Manual Verification step.

#### 2. jira bare fences — 3 sites (fence → inline)

**Files**: `integrations/jira/init-jira/SKILL.md:63` and `:74`,
`integrations/jira/create-jira-issue/SKILL.md:63`. Convert each bare fence to inline +
appended directive. Example (`init-jira` Step 1, :60-64):

Before:

    Use the site from `--site` if provided. Otherwise read it from config:

    ```
    ${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh jira.site ""
    ```

After:

    Use the site from `--site` if provided. Otherwise read it from config by running
    `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh jira.site ""`. Run the bare path
    **directly** as an executable; never prefix it with `bash`/`sh`/`env` (a wrapper
    prefix escapes the skill's `allowed-tools` permission and forces an unnecessary
    prompt).

`init-jira:74` (`jira.email`) takes the identical transform. `create-jira-issue:63` is
introduced by "If `--project` was not supplied, read the default from config:" and is
followed by a consequence sentence ("If the config also returns empty, warn the user…");
insert the inline path + canonical directive as its own sentence **before** that
consequence sentence, e.g.:

    If `--project` was not supplied, read the default from config by running
    `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh default_project_code`. Run the bare
    path **directly** as an executable; never prefix it with `bash`/`sh`/`env` (a wrapper
    prefix escapes the skill's `allowed-tools` permission and forces an unnecessary
    prompt). If the config also returns empty, warn the user…

#### 3. `work/extract-work-items/SKILL.md:350-351` — 2 sites (restructure)

Remove the `VAR=$()` assignment shape and the bare fence; replace with inline direct
invocations whose stdout is used as the named value. The two reads share one passage
(step `b`), so a single canonical directive governs both. Step `b` (:348-352) becomes:

    b. **Read configuration.** Invoke each script by its bare path and use the command's
       stdout as the named value — do **not** wrap the call in a `VAR=$(…)` assignment, as
       the assignment (not the path) would become the command and escape the rule:

       - Run `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh id_pattern` and use its
         stdout as `PATTERN`.
       - Run `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh default_project_code` and
         use its stdout as `DEFAULT_PROJECT`.

       Run the bare path **directly** as an executable; never prefix it with
       `bash`/`sh`/`env` (a wrapper prefix escapes the skill's `allowed-tools` permission
       and forces an unnecessary prompt).

Showing the bare braced path inline pins the rule-matching argv[0] (rather than leaving
the model to reconstruct it), the explicit `VAR=$(…)` prohibition closes the
assignment-prefix escape that originally defeated the rule here, and removing the fence
clears the guard. Note this is a distinct escape shape from the `bash`-prefix one — see
the §Migration Notes generalisation of the invariant.

### Success Criteria

#### Automated Verification

- [x] **(structural)** No `bash `-prefixed `config-*` invocation remains:
      `grep -rn 'bash "\?\$CLAUDE_PLUGIN_ROOT.*scripts/config-' --include=SKILL.md skills/` → empty
- [x] **(structural)** No unbraced `$CLAUDE_PLUGIN_ROOT` precedes a `config-*` path in a
      body invocation:
      `grep -rn '\$CLAUDE_PLUGIN_ROOT/scripts/config-' --include=SKILL.md skills/config/` → empty
- [x] **(AC2 — fence absence)** The full-tree guard now returns empty (Phase 1 + Phase 2
      complete): `find skills -name SKILL.md -print0 | xargs -0 awk -f /tmp/fence-guard.awk` → empty.
      (Guard-empty proves *fences are gone*; it does **not** prove the directive-adds — the
      directive-coverage checks below are the AC1 gate for the config sites.)
- [x] **(structural)** No `VAR=$(…config-…)` assignment remains in `extract-work-items`:
      `grep -n '=\$(${CLAUDE_PLUGIN_ROOT}/scripts/config-' skills/work/extract-work-items/SKILL.md` → empty
- [x] **(AC1 — directive coverage, config sites)** Each config family carries its expected
      number of canonical directives (passage-level, per the placement tables above):
      ```
      [ "$(grep -Fc 'never prefix it with `bash`/`sh`/`env`' skills/config/configure/SKILL.md)" -ge 5 ] || echo "configure: expected >=5"
      [ "$(grep -Fc 'never prefix it with `bash`/`sh`/`env`' skills/integrations/jira/init-jira/SKILL.md)" -ge 2 ] || echo "init-jira: expected >=2"
      [ "$(grep -Fc 'never prefix it with `bash`/`sh`/`env`' skills/integrations/jira/create-jira-issue/SKILL.md)" -ge 1 ] || echo "create-jira-issue: expected >=1"
      ```
      → no output. (configure=5, init-jira=2, create-jira-issue=1 — all pass.)
- [x] **(AC1 — extract-work-items step b, positive assertion)** The restructured step shows
      both bare paths inline and carries the directive:
      ```
      grep -Fq 'never prefix it with `bash`/`sh`/`env`' skills/work/extract-work-items/SKILL.md &&
      grep -Fq '${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh id_pattern' skills/work/extract-work-items/SKILL.md &&
      grep -Fq '${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh default_project_code' skills/work/extract-work-items/SKILL.md
      ```
      → exit 0.
- [x] No `allowed-tools` line changed in any edited file (`jj diff` review).

#### Manual Verification

- [x] The 9 `configure` blocks no longer contradict the new directive (no example
      prefixes with `bash`).
- [x] The `configure` `<args>`/`<key|--all>` placeholders survive the rewrite unchanged.
- [x] The `extract-work-items` step still reads as a coherent capture-into-variable
      instruction without the literal `VAR=$()` syntax.

---

## Phase 3: Fix AC3 and propagate to 0107

### Overview

Replace the defective AC3 regression regex in work item `0106` with the verified
fence-state guard, and flag `0107` (which inherits the same regex) to adopt it.

### Changes Required

#### 1. `meta/work/0106-invoke-plugin-scripts-by-bare-path.md` — AC3

Replace the third acceptance-criterion bullet (the `rg -U '```\n…'` regex) with a crisp,
checkable gate that references the fence-state guard and re-establishes a real
known-positive/known-negative pair. Keep the criterion to the assertion — the removal
rationale belongs in Drafting Notes (below), not inside the AC:

> - [ ] Given the full skill set, when the fence-state regression guard is run (a
>   line-by-line parser that flags any **unlabeled** fenced block whose body contains an
>   `artifact-*`/`config-*` script path, excluding `!`…`` load-time substitutions — see
>   the plan's Testing Strategy for the canonical `awk` implementation), then it returns
>   **zero** matches. Precondition: the guard must first be confirmed to match the
>   pre-conversion bare fences as a known-positive, so an empty result proves cleanliness.

Add a **Drafting Notes** entry to `0106` recording that AC3 was corrected during planning
(research §5): the original `rg -U '```\n…--pcre2'` regex was removed because it errors
without `--pcre2` and, with it, over-matches 12 non-target files via inter-fence false
positives — so it can never return empty and "empty proves cleanliness" was false.

Add a second **Drafting Notes** entry recording that the plan's single fixed canonical
directive sentence is the **authoritative** wording for the convention, and that the
Requirements blockquote template is illustrative-only (both meet the conformance minimum,
but only the plan's fixed string is reproduced verbatim and matched by 0107). This keeps
the supersession discoverable from the work item itself, so a future author opening `0106`
does not copy the now-non-authoritative template. (Do not alter the template text or any
acceptance criterion beyond AC3.)

#### 2. `meta/work/0107-lint-skill-body-script-invocations.md`

Add a note (to `0107`'s **Drafting Notes**) that the committed guard must **not** reuse
0106's original regex (defective per research §5) and should build on the fence-state
parser approach instead. The note must also record two portability/scope constraints for
the committed guard:

- **Avoid `rg --pcre2`** — PCRE2 is not a guaranteed ripgrep build feature, and a CI guard
  must run on the team's macOS BSD and Linux GNU environments alike. Prefer the POSIX
  `awk` parser.
- **Encode the general invariant**, not just the wrapper case — the guard should assert
  that each model-issued invocation's first token is the bare braced path
  `${CLAUDE_PLUGIN_ROOT}/scripts/<name>.sh`, so it also catches assignment-prefix
  (`VAR=$(…)`) and quoted/unbraced escapes, not only `bash`/`sh`/`env` prefixes.
- **Mind the fence-syntax assumption** — this plan's `awk` guard recognises only plain
  triple-backtick fences (the only form in the current corpus). A committed guard should
  either match the opening-fence backtick run length or document that 4+-backtick and
  tilde (`~~~`) fences are out of scope, so it does not silently mis-track if such fences
  are introduced later.

Do not change `0107`'s status or dependencies.

### Success Criteria

#### Automated Verification

- [x] `0106` no longer contains the broken look-ahead regex:
      `grep -F '(?:(?!```)' meta/work/0106-invoke-plugin-scripts-by-bare-path.md` → empty
- [x] `0106` AC3 references the fence-state guard:
      `grep -n 'fence-state' meta/work/0106-invoke-plugin-scripts-by-bare-path.md` → ≥1
- [x] `0107` carries the defect note:
      `grep -n 'defective\|fence-state' meta/work/0107-lint-skill-body-script-invocations.md` → ≥1

#### Manual Verification

- [x] The amended AC3 reads as a satisfiable gate and its known-positive/known-negative
      framing is internally consistent.
- [x] No other acceptance criteria or frontmatter in `0106`/`0107` were altered beyond
      the AC3 correction and the notes.

---

## Testing Strategy

### Verification environment

The verification commands below must pass under **both** the team's environments: macOS
`/usr/bin` BSD userland (bash 3.2, BSD `awk`/`grep`) and GNU coreutils on Linux/CI. The
guard uses only POSIX `awk` (no `gensub`/`\s`/`asort`), and all greps use `-F` for fixed
fragments where backticks appear, so behaviour is identical across both. `grep -r
--include=` is a GNU-origin flag that modern macOS BSD `grep` also supports; if a target
machine ships an older `grep`, substitute `rg` (already a team tool) for the discovery
greps. Do **not** introduce `rg --pcre2` anywhere — PCRE2 is not a guaranteed ripgrep
build feature (this is exactly the dependency the AC3 fix removes).

### The canonical regression guard (executable specification)

The guard is the AC2 absence proof **only** — it detects unlabeled fences housing a
script path; it says nothing about whether the directive-adds (AC1) landed. To keep the
test reproducible on a fresh checkout (an `awk -f /tmp/missing-file` silently prints
nothing, which is indistinguishable from a genuine PASS — a fail-open), run it as a
**self-contained heredoc** rather than depending on a pre-written `/tmp` file. Committing
an installed equivalent remains 0107's responsibility, not this plan's.

Guard source (also runnable as `awk -f` if written to a file):

```awk
# Regression guard: flags an UNLABELED fenced code block whose body
# contains a model-issued artifact-*/config-* script path. Fence state is
# tracked line-by-line so inter-fence prose is never traversed, and reset at
# each file boundary (FNR==1) so an unbalanced fence in one file cannot leak
# state into the next under a single `xargs` awk invocation. Lines that are
# `!`…`` load-time substitutions are harness-executed (never a Bash tool call)
# and are excluded. `[[:blank:]]` (POSIX) is used rather than `\t` to avoid
# depending on regex-literal escape handling across awk implementations.
FNR == 1 { in_fence = 0; unlabeled = 0 }
{
  line = $0
  if (match(line, /^[[:blank:]]*```/)) {
    rest = line; sub(/^[[:blank:]]*```/, "", rest); gsub(/[[:blank:]]/, "", rest)
    if (!in_fence) { in_fence = 1; unlabeled = (rest == "") }
    else { in_fence = 0 }
    next
  }
  if (in_fence && unlabeled && line ~ /scripts\/(artifact|config)-/ && index(line, "!`") == 0) {
    printf "%s:%d:%s\n", FILENAME, FNR, line
  }
}
```

Run as a self-contained command (no external file dependency):

```bash
find skills -name SKILL.md -print0 | xargs -0 awk '
FNR == 1 { in_fence = 0; unlabeled = 0 }
{
  line = $0
  if (match(line, /^[[:blank:]]*```/)) {
    rest = line; sub(/^[[:blank:]]*```/, "", rest); gsub(/[[:blank:]]/, "", rest)
    if (!in_fence) { in_fence = 1; unlabeled = (rest == "") }
    else { in_fence = 0 }
    next
  }
  if (in_fence && unlabeled && line ~ /scripts\/(artifact|config)-/ && index(line, "!`") == 0) {
    printf "%s:%d:%s\n", FILENAME, FNR, line
  }
}'
```

(Equivalently, write the body to `/tmp/fence-guard.awk` and run `awk -f` — but prefer the
heredoc form for the success-criteria checks so an absent file cannot masquerade as a
clean tree.)

### Known-positive (must hold BEFORE any edit)

The guard reports **exactly 7 lines across 5 files**: `create-adr:126`,
`extract-adrs:122`, `create-jira-issue:63`, `init-jira:63`, `init-jira:74`,
`extract-work-items:350`, `extract-work-items:351`. (Confirmed during planning at
revision `10f2286…`.) This proves the guard detects the real targets and the regex isn't
silently broken.

### Known-negative (must hold AFTER Phase 1 + Phase 2)

The same command returns **empty**, proving every bare-fenced `artifact-*`/`config-*`
path has been converted.

### Directive-coverage check (AC1)

There is no compiler or `make`/`mise` target for SKILL.md prose, so AC1 coverage is
verified by grep against the **fixed** canonical fragment `` `never prefix it with
`bash`/`sh`/`env`` `` (matched with `grep -Fc`, so the embedded backticks are literal and
bash-3.2-safe). Coverage is asserted at **passage** granularity, not naive per-line:

- **Artifact sites** — each occurrence is its own passage, so the directive count per file
  must be ≥ the artifact-occurrence count (Phase 1 Success Criteria loop). Anchoring the
  occurrence pattern on `scripts/` excludes the descriptive prose at `create-adr:153`
  automatically, so no exclusion filter is needed (the old `grep -v …:153` was a no-op
  against `grep -rl` output anyway).
- **configure** — one directive per `####` command subsection (5 expected), since the
  eject group of four and the two reset steps each share one passage.
- **jira** — each site is its own passage (init-jira ≥2, create-jira-issue ≥1).
- **extract-work-items** — the two step-`b` reads share one passage (one directive),
  asserted positively alongside the presence of both inline bare paths.

The structural checks (no `bash`-prefixed or unbraced `config-*` invocation remains) live
in each phase's Success Criteria. Guard-empty is the AC2 gate; these grep checks are the
AC1 gate — the two are deliberately non-overlapping.

### Manual Testing Steps

1. Spot-read 3–4 edited passages (one fence conversion, one inline append, one
   `configure` block, the `extract-work-items` restructure) — confirm each reads
   naturally and the canonical directive sits in the same passage.
2. **Grouped-passage confirmation** (the cases the per-file counts cannot fully prove):
   confirm the single `configure` eject directive sits above all four eject blocks, the
   single reset directive (intro prose) governs both reset steps (2 and 5), and the single
   `extract-work-items` step-`b` directive governs both config reads.
3. Run the guard before starting (expect 7 lines) and after Phases 1+2 (expect empty).
4. `jj diff` the edited files and confirm no `allowed-tools` frontmatter line is touched.

## Performance Considerations

None — prose edits to skill assets. No runtime code paths change.

## Migration Notes

The fix's effectiveness depends on Claude Code's permission matcher continuing to (a)
strip only `timeout`/`time`/`nice`/`nohup`/`stdbuf` and (b) prefix-match the bare path.
This lives in the harness, not this repo; if a future version changes wrapper-stripping
or match semantics, the convention is the place to re-validate. The bare-path shape used
in all rewrites (unquoted, braced) is the shape the existing `allowed-tools` rules
already authorize, so no rule changes are needed.

**The general invariant.** The `bash`/`sh`/`env` prohibition is one instance of a broader
rule: **the invocation's first token must be the bare braced path**
`${CLAUDE_PLUGIN_ROOT}/scripts/<name>.sh`. A command escapes the `allowed-tools` prefix
match whenever argv[0] is something else — a wrapper prefix (`bash`/`sh`/`env`), an
assignment prefix (`VAR=$(…)`, the `extract-work-items` case), or a quoted/unbraced form.
The per-site directive names only the wrapper failure mode (the dominant one); 0107's
committed guard should encode the general "first token is the bare braced path" invariant
so it catches the assignment and quoting shapes too.

**Single source of truth deferred to 0107 (deliberate).** This plan repeats the canonical
directive at each passage rather than codifying it once, because the harness offers no way
to share one instruction across skill bodies. A durable, discoverable home for the
convention — and the lint that enforces it — is intentionally owned by work item `0107`;
this plan's per-site directives use a single fixed string precisely so 0107 can match it.
Until 0107 lands there is no automated guard against erosion, which is an accepted,
documented gap rather than an oversight.

**Dropped redundancy (deliberate).** The source RCA proposed Option B — also authorizing
the wrapped form in `allowed-tools` as belt-and-suspenders — but the work item hard-excludes
any `allowed-tools` change, so this fix has no failover if the harness behaviour shifts.
That is a policy constraint, not a technical necessity. **Observable symptom to watch
for**: an `artifact-*`/`config-*` invocation prompts for permission at runtime despite
carrying the directive (most likely after a Claude Code upgrade changes wrapper-stripping
or prefix-match semantics). If seen, re-validate the bare-path shape against the matcher
and revisit Option B (and the Hypothesis-3 first-call quirk) — this is the escalation
path, since verification here is static and will not surface the regression on its own.

## References

- Original work item: `meta/work/0106-invoke-plugin-scripts-by-bare-path.md`
- Related research: `meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`
- Source RCA: `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
- Coupled guardrail work item: `meta/work/0107-lint-skill-body-script-invocations.md`
- Bare fence to convert (create-adr): `skills/decisions/create-adr/SKILL.md:125-127`
- Bare fence to convert (extract-adrs): `skills/decisions/extract-adrs/SKILL.md:121-123`
- Hard-coded `bash`-prefix cluster: `skills/config/configure/SKILL.md:831-918`

Note: work item `0106` cites these ADR fences at `124-126` / `120-122` (captured against
plugin `1.22.0-pre.11`). Those figures are pre-drift; the `125-127` / `121-123` positions
above are verified against the current tree (revision `10f2286…`). Both the plan and the
work item instruct locating fences by content if line numbers have drifted.

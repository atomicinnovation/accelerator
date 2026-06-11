---
type: codebase-research
id: "2026-06-11-0106-bare-path-script-invocation-call-sites"
title: "Research: Bare-path script-invocation call sites in SKILL.md bodies (work item 0106)"
date: "2026-06-11T13:23:54+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0106"
parent: "work-item:0106"
topic: "Which SKILL.md passages invoke artifact-*/config-* plugin scripts, how each is shaped, and whether the work item's acceptance criteria are satisfiable"
tags: [research, codebase, permissions, allowed-tools, skills, plugin, authoring-convention]
revision: "3b82e5d1b902b6e57db51ac345df5e72105c5455"
repository: "miscellaneous"
last_updated: "2026-06-11T13:23:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Bare-path script-invocation call sites in SKILL.md bodies (work item 0106)

**Date**: 2026-06-11 13:23 UTC
**Author**: Toby Clemson
**Git Commit**: 3b82e5d1b902b6e57db51ac345df5e72105c5455
**Branch**: jj change `opkmzlpxsxxr` (no bookmark)
**Repository**: miscellaneous (accelerator plugin)

## Research Question

For work item 0106 ("Invoke Plugin Scripts by Bare Path in Skill Bodies"), establish:
the canonical set of `artifact-*`/`config-*` script call sites in skill bodies; the
exact shape and location of each (so a "run by bare path, never `bash`/`sh`/`env`"
directive can be placed in the same passage); the precise content/boundaries of the
two bare code fences slated for conversion to inline code; and whether the work item's
acceptance criteria — especially the AC3 regression-guard regex — are sound and
satisfiable as written.

## Summary

The work item names the output of
`grep -rn 'scripts/\(artifact\|config\)-' --include=SKILL.md skills/` as "the
denominator against which 'every call site' is verified." That grep returns **276
lines**, but only a small minority are *model-issued invocations* of the kind the
`bash`-prefix bug affects. The 276 lines decompose exactly as:

| Category | Count | Is it a "call site" the directive applies to? |
|---|---|---|
| `!`…`` load-time command substitutions (`!`${CLAUDE_PLUGIN_ROOT}/scripts/config-…``) | 213 | **No** — executed by the harness at skill-load time; the model never re-issues them, so they cannot be `bash`-wrapped. |
| Frontmatter `allowed-tools` rules (`- Bash(${CLAUDE_PLUGIN_ROOT}/scripts/…)`) | 35 | **No** — permission rules, not invocations; the work item explicitly forbids touching them. |
| `artifact-derive-metadata.sh` model-facing invocations | 14 | **Yes** — the core target (the source RCA's subject). |
| `config-*` model-facing invocations inside code fences/prose | 14 | **Yes (mostly)** — genuine model-issued sites, but heterogeneous (see below). |

So the genuine model-issued call-site set is **28 occurrences across ~17 files**, not
276. **The single most important finding: AC3's regression-guard regex is defective —
it requires `--pcre2` (it errors out otherwise) and, even with `--pcre2`, it
over-matches 14 files via inter-fence false positives rather than isolating the two
target fences. As written, AC3 can never return "empty," so it is unsatisfiable and
its premise ("an empty result proves cleanliness") does not hold.** This needs to be
fixed in the plan (or fed back to the work item) before AC3 can be used as a gate.

A secondary surprise: the `config/configure/SKILL.md` cluster (9 sites) is authored
**with** a hard-coded `bash ` prefix *and* an unbraced `$CLAUDE_PLUGIN_ROOT` — i.e.
the body already contains the exact anti-pattern the work item exists to eliminate.
These were invisible to the source RCA (which only studied `artifact-derive-metadata.sh`)
and force a scope decision the work item does not anticipate.

## Detailed Findings

### 1. The denominator decomposed (the "call site" definition problem)

The reconciliation is exact: `total 276 = 213 (!-subst) + 35 (- Bash() + 14 (artifact)
+ 14 (config residual)`.

The work item contains an internal tension. Its prose defines a call site
semantically — "one such passage [where] a skill body **invokes** an
`artifact-*`/`config-*` script" — yet also states the raw grep "result … is the
denominator against which 'every call site' is verified" and AC1 says "**every**
occurrence in that set" must carry a directive. Read literally, AC1 would require
appending a "don't prefix with `bash`" directive next to all 213 `!`-substitution lines
(e.g. `!`…/config-read-context.sh``) and all 35 frontmatter rule lines — which is
nonsensical: the model never issues those, and the rules are explicitly out of scope.

**Reconciliation the implementer must adopt:** the grep is the *discovery superset*;
the *directive-requiring subset* is the model-issued invocations only (the 14 + 14).
The `!`…`` form is the harness's [Claude Code command-substitution syntax] — executed at
load time, output injected into context — so it is structurally immune to the
`bash`-prefix failure mode and needs no directive. The plan should state this filter
explicitly and not treat AC1's "every occurrence" literally.

### 2. The 14 `artifact-derive-metadata.sh` call sites (the core target)

Confirmed by direct read of every occurrence. **None carries any existing
"bare-path / not-bash" directive.** Two are bare unlabeled fences (the conversion
targets); twelve are already inline code in prose.

| # | File:line | Shape | Verb | Placement passage |
|---|---|---|---|---|
| 1 | `decisions/create-adr/SKILL.md:126` | **bare fence** (open 125 / close 127) | "by running:" | Step 3, numbered item 1 |
| 2 | `decisions/extract-adrs/SKILL.md:122` | **bare fence, 3-sp indented** (open 121 / close 123) | "by running:" | Step 3, numbered item 1 |
| 3 | `decisions/extract-adrs/SKILL.md:163` | inline | "Invoke" | Step 4, nested item 1 |
| 4 | `github/describe-pr/SKILL.md:105` | inline | "Invoke" | Step 8, item 1 |
| 5 | `github/review-pr/SKILL.md:468` | inline | "Invoke" | Populate frontmatter, item 1 |
| 6 | `notes/create-note/SKILL.md:81` | inline | "Run" | Step 2, item 1 |
| 7 | `planning/create-plan/SKILL.md:230` | inline | "Invoke" | Step 5, item 1 |
| 8 | `planning/review-plan/SKILL.md:430` | inline | "Invoke" | Populate frontmatter, item 1 |
| 9 | `planning/validate-plan/SKILL.md:150` | inline | "Invoke" | Populate frontmatter, item 1 |
| 10 | `research/research-codebase/SKILL.md:112` | inline | "Run the … script" | Step 5, bullet |
| 11 | `research/research-issue/SKILL.md:94` | inline | "Gather metadata using" | Step 6, bullet |
| 12 | `work/create-work-item/SKILL.md:440` | inline | "Invoke" | Step 5, nested item 1 |
| 13 | `work/extract-work-items/SKILL.md:448` | inline (verb wraps to prev line) | "Invoke" | letter `h.`, nested item 1 |
| 14 | `work/review-work-item/SKILL.md:360` | inline | "Invoke" | Populate frontmatter, item 1 |

Notes for the implementer:
- **Verb inconsistency**: most say "Invoke", but #1/#2 say "by running:", #6 "Run",
  #10 "Run the … script", #11 "Gather metadata using". A canonical directive must read
  naturally appended to each.
- **Path split across lines**: #11 (verb on :93, path on :94) and #13 (verb "Invoke"
  on :447, path on :448) — the "same passage" spans two lines.
- **Out-of-scope sibling not to be confused**: `extract-adrs/SKILL.md:148-150` is a
  *second* bare fence invoking `adr-next-number.sh` (not `artifact-*`/`config-*`), so it
  is outside this work item but sits right below the in-scope fence.

### 3. The two fence conversions (AC2 target), verbatim

**`create-adr/SKILL.md` lines 123-127:**
```
123	1. **Gather metadata** by running:
124	
125	```
126	${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh
127	```
```
Fence: open **125**, close **127**.

**`extract-adrs/SKILL.md` lines 120-123** (fence is 3-space-indented as a list
continuation):
```
120	1. **Gather metadata** by running:
121	   ```
122	   ${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh
123	   ```
```
Fence: open **121**, close **123**.

Both convert to an inline-code form (path in backticks within the step), plus the
canonical directive. Line numbers match the work item's version-pinned hints closely
(work item cited `create-adr:124-126` / `extract-adrs:120-122` against `1.22.0-pre.11`;
the live tree is one line off for create-adr) — locate by content, as the work item
advises.

### 4. The 14 `config-*` residual call sites (the unanticipated cluster)

These are model-issued `config-*` invocations that are **not** `!`-substitutions and
**not** frontmatter rules. They split into three materially different sub-cases:

**(a) `config/configure/SKILL.md` — 9 sites, lines 831, 841, 853, 862, 868, 875, 891,
907, 918.** Each is inside a ` ```bash ` fenced block and is authored as
`bash "$CLAUDE_PLUGIN_ROOT/scripts/config-…-template.sh" <args>` — i.e. **already
`bash`-prefixed and using unbraced `$CLAUDE_PLUGIN_ROOT`**. Both traits break matching
against `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` (argv[0] becomes `bash`, and the
unbraced var would not equal the braced rule token even after the prefix issue). The
surrounding prose directs the model to execute these ("Run the list script and display
its output:", "then run the actual eject with `--force`:"), so they are genuine
model-executed steps — and the *worst* case, because the anti-pattern is baked into the
authored text rather than left to model discretion.

**(b) jira bare-path fences — `create-jira-issue/SKILL.md:63`, `init-jira/SKILL.md:63`
and `:74`.** Inside bare ` ``` ` fences, written correctly as bare braced paths
(`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-{work,value}.sh …`), no `bash` prefix. They
*match* the rule as authored; risk arises only if the model prepends `bash`.

**(c) `extract-work-items/SKILL.md:350-351`** — `VAR=$(${CLAUDE_PLUGIN_ROOT}/scripts/
config-read-work.sh …)` assignment+command-substitution inside a bare ` ``` ` fence. A
*different* matcher concern: argv[0] is `PATTERN=…`, not the script path, so the
"don't prefix with `bash`" directive does not even address this shape.

**Scope decision the plan must make (not decided here):** the work item's directive
template ("run directly as an executable: `<bare-path>`; do not prefix with
`bash`/`sh`/`env`") fits sub-case (b) cleanly, fits (a) only if the body is *also*
rewritten to drop the `bash ` prefix and brace the var (which is more than "add a
directive"), and does not fit (c) at all. Options: (i) rewrite (a) to bare-path and add
directives to (a)+(b), excluding (c) as a distinct shape; (ii) treat the whole `config-*`
residual as out of scope and limit 0106 to the 14 `artifact-*` sites (matching the source
RCA's actual evidence); (iii) handle (a)'s rewrite under a separate item. The source RCA
only ever studied `artifact-derive-metadata.sh`, so the `config-*` residual is an
extrapolation the work item's "broadest enumeration" assumption pulls in.

### 5. AC3 regression-guard regex is defective (critical)

The work item's AC3 specifies:
`rg -U '```\n(?:(?!```)[\s\S])*scripts/(?:artifact|config)-[\s\S]*?```' skills/`
and asserts (a) it must match the pre-conversion fences as a known-positive, and (b)
after the work it must "return zero matches … empty result proves cleanliness."

Empirically, both halves fail:

1. **It errors without `--pcre2`.** rg's default Rust-regex engine rejects the `(?!```)`
   negative look-ahead: *"look-around … is not supported … Consider enabling PCRE2 with
   the `--pcre2` flag."* (exit 2). As written, the AC command does not run at all.

2. **With `--pcre2`, it over-matches 14 files, not 2.** The known-positive check
   "passes" only by accident: it matches `create-adr` and `extract-adrs` **and** 12
   other files (`create-plan`, `review-plan`, `validate-plan`, `review-pr`,
   `create-work-item`, `review-work-item`, `create-note`, `analyse-design-gaps`,
   `inventory-design`, `init-jira`, `create-jira-issue`, `extract-work-items`).

   Root cause of the false positives: the leading `` ``` `` cannot distinguish an
   *opening* fence from a *closing* fence, and `(?:(?!```)[\s\S])*` then traverses
   ordinary inter-fence **prose** — which legitimately contains `!`…/config-…``
   substitution lines and inline `artifact-…` references — until it reaches a script
   path, then closes on the *next* opening fence. Verified in `create-plan/SKILL.md`:
   the match runs from a closing fence near the template block, across prose containing
   `!`config-read-path.sh`` / `!`config-read-template.sh`` and the inline
   `artifact-derive-metadata.sh`, to the next fence — **no offending bare-fenced path
   exists there at all.**

   Consequence: these 12 files match *before any edit*. Converting the two real fences
   will **not** drive the result to empty — the regex will still report ~12 files. **AC3
   as written is unsatisfiable, and "empty proves clean" is false.** The plan must
   replace it with a sound check (e.g. a per-fence parser, or a tightened pattern that
   anchors the opening fence to a line-start with no preceding fence and forbids `!`…``
   prose — non-trivial in a single regex) and re-establish a real known-positive/
   known-negative pair. This same regex is inherited by work item 0107's AC, so the fix
   should propagate there.

### 6. Mechanism confirmation: `!`…`` vs inline vs rule

In a representative skill (`research/research-codebase/SKILL.md`):
- Frontmatter `:7-9`: `- Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` /
  `…/artifact-*` — the permission grants; both match the **bare** path (no `bash`
  token), which is exactly why a model-issued `bash …` escapes them.
- `:14-16, 24-26`: `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-*…`` — load-time
  substitutions, harness-executed, never a Bash tool call.
- `:112-113`: `- Run the `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
  script …` — inline-code prose instruction the model executes itself; the one form
  vulnerable to a model-added `bash` prefix.

## Code References

- `skills/decisions/create-adr/SKILL.md:125-127` — bare fence to convert (artifact)
- `skills/decisions/extract-adrs/SKILL.md:121-123` — bare fence to convert (artifact)
- `skills/decisions/extract-adrs/SKILL.md:163` — inline artifact invocation
- `skills/config/configure/SKILL.md:831-918` — 9 pre-`bash`-prefixed `config-*` sites in ```bash blocks
- `skills/integrations/jira/create-jira-issue/SKILL.md:63` — bare-path config-* in bare fence
- `skills/integrations/jira/init-jira/SKILL.md:63,74` — bare-path config-* in bare fences
- `skills/work/extract-work-items/SKILL.md:350-351` — `VAR=$(…config-read-work.sh)` (different shape)
- `skills/research/research-codebase/SKILL.md:7-9,14-16,112-113` — rule vs `!`-subst vs inline reference
- 12 inline `artifact-derive-metadata.sh` sites: see table in §2

## Architecture Insights

- **The `!`…`` substitution form is the plugin's structural defense** against the
  `bash`-prefix bug: those invocations are resolved before the model ever sees them, so
  they cannot be mis-wrapped. The vulnerable surface is precisely the residue that
  *can't* be `!`-substituted — `artifact-derive-metadata.sh` (run mid-execution, after
  the model has produced content) and the handful of `config-*` reads embedded in
  conditional/branching example blocks. This explains why the affected set is small and
  why `!`-substitution isn't simply applied everywhere.
- **The rule authorizes the bare shape only**; body invocation shape and rule prefix
  must stay in lockstep (the RCA's "Prevention"). 0106 is the body-side half of that
  lockstep; 0107 is the automated enforcer.
- **The grep-as-denominator framing is a category error baked into the work item**: a
  text-search superset is conflated with a semantic call-site set. Both 0106 (manual)
  and 0107 (automated) inherit this and must filter `!`-substitutions and frontmatter
  out before reasoning about "invocations." A robust 0107 lint must parse fence state
  and the leading `!`, not grep.

## Historical Context

- `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
  — the source RCA. Confirms Hypothesis 1 (leading `bash` breaks the prefix/glob match;
  only `timeout`/`time`/`nice`/`nohup`/`stdbuf` are stripped, not `bash`/`sh`/`env`),
  eliminates Hypothesis 2 (var expansion is handled), leaves Hypothesis 3 (first-call-only
  enforcement quirk) inconclusive. Its scope is strictly `artifact-derive-metadata.sh`;
  the `config-*` residual in §4 is beyond what it evidences. Its "Recommended Fix"
  blockquote names only `bash`/`sh`; 0106 standardises the three-wrapper `bash`/`sh`/`env`
  form. Its Prevention section is the origin of both 0106 and 0107.
- `meta/work/0107-lint-skill-body-script-invocations.md` — the automated guardrail
  (`blocked_by: 0106`). Extracts each body invocation, expands `${CLAUDE_PLUGIN_ROOT}`,
  and asserts each invocation shape is covered by an `allowed-tools` prefix; flags three
  violation classes (wrapper-prefixed, bare-fenced path, uncovered shape) and **reuses
  0106's AC3 regex** — so the §5 defect propagates and should be fixed once. Drafting
  notes flag that `blocked_by` could relax to `relates_to` to develop in parallel.
- `meta/reviews/work/0106-invoke-plugin-scripts-by-bare-path-review-1.md` — an existing
  work-item review of 0106 (verdict not deep-read here).
- **No plan** for 0106 exists yet (newest `meta/plans/` entry is 2026-06-09; 0106 was
  created 2026-06-11). **No ADR or note** codifies a SKILL.md script-invocation
  convention — 0106 and its RCA are the first artifacts to define it.

## Related Research

- `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
  (source RCA, same author, prior day)

## Open Questions

1. **AC3 regex (blocking for verification):** the criterion is unsatisfiable as written
   (needs `--pcre2`; over-matches 14 files; "empty proves clean" is false). Does the
   plan fix the regex/replace the check, or should the work item's AC3 be amended first?
   Whatever is chosen should also land in 0107 (which inherits the same regex).
2. **`config-*` scope (blocking for sizing):** is 0106 limited to the 14 `artifact-*`
   sites (matching the RCA's evidence), or does it also cover the 14 `config-*` residual
   sites? If the latter, the `configure` cluster (9 sites) needs a *rewrite* (drop
   `bash `, brace the var) — more than "add a directive" — and the `extract-work-items`
   `VAR=$(…)` shape (2 sites) is not addressed by the templated directive at all.
3. **Directive-vs-example contradiction in `configure`:** if the 9 ```bash example
   blocks are kept verbatim, a "never prefix with `bash`" directive sits beside an
   example that *does* prefix with `bash`. Keeping both is self-contradictory; the plan
   must rewrite the examples or carve them out.
4. **Non-blocking (from the work item):** is the RCA's Hypothesis-3 "first-call-only
   `allowed-tools` enforcement" quirk present in the current Claude Code version? If so,
   body directives alone may not eliminate every prompt.

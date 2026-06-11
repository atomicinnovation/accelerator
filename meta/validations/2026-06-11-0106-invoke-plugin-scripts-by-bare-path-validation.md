---
type: plan-validation
id: "2026-06-11-0106-invoke-plugin-scripts-by-bare-path-validation"
title: "Validation Report: Invoke Plugin Scripts by Bare Path in Skill Bodies"
date: "2026-06-11T19:33:35+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: "pass"
target: "plan:2026-06-11-0106-invoke-plugin-scripts-by-bare-path"
relates_to: ["work-item:0106", "work-item:0107"]
tags: [permissions, allowed-tools, skills, plugin, authoring-convention]
last_updated: "2026-06-11T19:33:35+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Invoke Plugin Scripts by Bare Path in Skill Bodies

### Implementation Status

✓ Phase 1: Artifact call sites + ADR fence conversions — Fully implemented
✓ Phase 2: config-* comprehensive rewrite — Fully implemented
✓ Phase 3: Fix AC3 and propagate to 0107 — Fully implemented

The work landed as three atomic, phase-aligned commits:

- `zsrpvnyntxny` — Add bare-path directive to artifact script call sites and
  convert ADR fences to inline code (Phase 1)
- `yuusnmxlmuto` — Rewrite config script invocations to bare-path form with
  directives (Phase 2)
- `qzwkwzmymsnt` — Replace defective AC3 regex with fence-state guard and
  propagate to 0107 (Phase 3)

### Automated Verification Results

**Phase 1 — artifact sites + ADR fences**

✓ Fence-state guard reports no `create-adr`/`extract-adrs` bare fence (AC2 absence)
✓ Inline path presence: `create-adr`=1, `extract-adrs`=2 (`grep -Fc`)
✓ Per-occurrence directive coverage loop emits no `UNDER-COVERED` lines (AC1)
✓ No `allowed-tools` rule line changed (no `- Bash(…)` hunk in the diff)

**Phase 2 — config-***

✓ No `bash`-prefixed `config-*` invocation remains (empty)
✓ No unbraced `$CLAUDE_PLUGIN_ROOT` before a `config-*` path in `skills/config/` (empty)
✓ No `VAR=$(…config-…)` assignment remains in `extract-work-items` (empty)
✓ Full-tree fence-state guard returns **empty** (AC2 — Phase 1 + Phase 2 complete)
✓ Directive coverage: `configure`=5 (≥5), `init-jira`=2 (≥2), `create-jira-issue`=1 (≥1)
✓ `extract-work-items` step b positive assertion (both inline bare paths + directive) → exit 0

**Phase 3 — AC3 correction + 0107 propagation**

✓ `0106` no longer contains the broken look-ahead regex `(?:(?!```)` (empty)
✓ `0106` AC3 references the `fence-state` guard (2 hits)
✓ `0107` carries the `defective`/`fence-state` note (2 hits)

**Cross-cutting**

✓ No `allowed-tools` frontmatter rule line was modified in any edited file. The only
  diff lines mentioning `allowed-tools` are (a) the plan's own success-criteria
  checkbox text and (b) the directive prose itself, which references the
  `allowed-tools` permission by design. No `- Bash(…)` rule line changed.

### Code Review Findings

#### Matches Plan:

- The two ADR bare fences (`create-adr`, `extract-adrs`) are converted to inline
  backtick-delimited paths with the canonical directive appended in the same step.
- All 12 inline artifact sites carry the verbatim canonical directive
  (`never prefix it with `bash`/`sh`/`env`…`).
- The `configure` cluster uses the unquoted, braced bare path
  (`${CLAUDE_PLUGIN_ROOT}/scripts/config-*-template.sh`) with no `bash` prefix; the
  `<key|--all>` / `<args>` placeholders survive intact (lines 861/870/876/883).
- The eject subsection uses a single head-level directive that explicitly states it
  "applies to every `config-eject-template.sh` invocation in this …" — correctly
  covering all four eject blocks as one passage.
- The `extract-work-items` step `b` restructure removes the `VAR=$(…)` assignment
  shape, shows both bare paths inline with "use its stdout as `PATTERN`/`DEFAULT_PROJECT`",
  and carries the explicit `VAR=$(…)` prohibition plus the canonical directive.
- AC3 in `0106` is replaced with the fence-state-guard gate; Drafting Notes record the
  removal rationale and the authoritative-directive supersession. `0107` carries the
  portability/scope constraints (avoid `--pcre2`, encode the general invariant, mind the
  fence-syntax assumption).

#### Deviations from Plan:

- None material. The canonical directive sentence is reproduced verbatim across every
  edited site; line-wrapping of the fixed string varies between passages (e.g.
  `extract-work-items` wraps after "executable;"), which is cosmetic and does not break
  the `grep -Fc` fixed-fragment match the plan and 0107's future lint rely on.

#### Potential Issues:

- No automated guard against future erosion exists yet — this is the deliberate, documented
  gap owned by work item `0107`. Until 0107 lands, the convention is enforced only by the
  manual fence-state guard exercised here.
- Static verification cannot surface the Migration-Notes residual risk: an invocation
  could still prompt at runtime if a future Claude Code release changes wrapper-stripping
  or prefix-match semantics. This is the documented escalation path, not a defect in this
  implementation.

### Manual Testing Required:

All manual verification items from the plan were exercised during validation and pass:

1. Passage readability:
  - [x] `create-adr` fence conversion reads naturally with the directive in the same step
  - [x] `extract-work-items` step `b` reads as a coherent capture-into-variable instruction
        without literal `VAR=$()` syntax
  - [x] `configure` eject block uses the braced bare path with placeholders preserved

2. Grouped-passage proximity:
  - [x] Single `configure` eject directive sits above all four eject blocks
  - [x] Single `extract-work-items` step-`b` directive governs both config reads

No further manual testing is required.

### Recommendations:

- Proceed with work item `0107` to commit the fence-state guard as an automated CI
  guardrail, encoding the general "first token is the bare braced path" invariant
  (not just the wrapper case) and avoiding `rg --pcre2` for BSD/GNU portability.
- After the next Claude Code upgrade, re-confirm the bare-path shape still matches the
  matcher (the Migration-Notes observable symptom: a directive-carrying invocation that
  still prompts).

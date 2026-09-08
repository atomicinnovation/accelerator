---
type: "pr-description"
id: "103"
title: "[0245] Invoke accelerator directly in skills"
date: "2026-09-06T22:40:20+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0245"
parent: "work-item:0245"
relates_to: ["work-item:0107"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/103"
pr_number: 103
tags: ["skills", "cli", "lint"]
revision: "943c2f71e2f63fec0e80d5cd48edcdeff7f6090c"
repository: "accelerator"
last_updated: "2026-09-06T22:40:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0245] Invoke accelerator directly in skills

## Summary

Converges every `SKILL.md` onto bare `accelerator …` invocation, dropping the
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator` prefix, and guards the convention with a
new allowlist lint wired into the fast `check` lane. The prefix was redundant —
Claude Code places the plugin `bin/` on `PATH` (since v2.1.91, below our
declared v2.1.144 floor) — and one uniform call form is easier to allowlist,
read, and maintain.

## Changes

- **Converted all 427 call sites across 47 `SKILL.md` files** — body `` !`…` ``
  preprocessor calls, frontmatter `Bash(…)` grants, and prose/fenced examples —
  from `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` to bare `accelerator`.
- **Flipped the shared parser** (`tasks/shared/skill_parsing.py`): `LAUNCHER`
  and the config markers are now bare and defined once; `is_plugin_invocation`
  recognises the bare form while still accepting the `${CLAUDE_PLUGIN_ROOT}/`
  prefix for the surviving non-accelerator plugin references.
- **Retargeted the coupled consumers** to the bare form: the permissions lint
  (`skill_permissions.py`, now importing the shared markers), the write-gate
  mutation matcher (`skill_write_gate.py`), and the integration conformance
  corpus (`skill_corpus.py`, which resolves the binary via `installation.root`
  rather than textual substitution).
- **Added `tasks/lint/bare_invocation.py`** — a guard that fails, naming
  `file:line`, on any reintroduced prefix, absolute path, or `bash`/`sh`/`env`
  wrapper across `skills/**/SKILL.md`; wired into `build-system:check` (fast
  lane) and `lint:check`, with its placement pinned in `test_mise.py`.
- **Closed work item 0107** (superseded by this lint) and recorded the
  intentional hooks boundary in a new `hooks/CLAUDE.md` — hooks keep the pathed
  form because they receive `${CLAUDE_PLUGIN_ROOT}` as an env var but have no
  bin-on-`PATH` guarantee.

## Context

- Work item: `meta/work/0245-invoke-accelerator-directly-in-skills.md` (under
  epic 0136, single-call-form convergence).
- Plan: `meta/plans/2026-09-06-0245-invoke-accelerator-directly-in-skills.md`
  (status `done`).
- Validation: `meta/validations/2026-09-06-0245-…-validation.md` (result
  `pass`).
- Supersedes 0107 (the bare-invocation guard it asked for is built here).

## Testing

- [x] Three acceptance greps return zero: no `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, no `/bin/accelerator` suffix, no shell wrapper across `skills`.
- [x] Coupled + new unit suites pass: `uv run pytest test_skill_permissions test_skill_parsing test_dispatch_coherence test_integration_skills test_bare_invocation test_mise` — 204 passed.
- [x] Integration conformance suite passes on the converted tree: `mise run test:integration:skill-invocation` — 128 passed.
- [x] Guarding lints green: `lint:skill-permissions:check`, `lint:bare-invocation:check`, `lint:integration-skills:check`, `lint:dispatch-coherence:check`.
- [x] Component gate green: `mise run build-system:check` (format + lint + types, carries the new lint).
- [x] Reintroduction guard exercised: scratch skills with a prefixed call and with an `sh`/`env` wrapper each make `lint:bare-invocation:check` exit 1, naming the file.
- [ ] Live spot-check: load `/accelerator:visualise` in a real session and confirm it runs with no permission prompt (requires an interactive session).

## Notes for Reviewers

- ⚠️ **The conversion is behaviour-preserving only if the plugin `bin/` is on
  `PATH`.** The version floor is traced — bin-on-`PATH` shipped in v2.1.91,
  below our v2.1.144 minimum — and the precondition gate confirmed both the
  `!`-preprocessor and Bash-tool surfaces resolve in main and subagent contexts.
  A future Claude Code regression dropping `bin/` from `PATH` would break all
  injection skills at load; the accepted revert path (coordinated lint change)
  is recorded on 0245.
- 🔒 The bare grant `Bash(accelerator config *)` authorises whatever resolves
  first on `PATH` rather than pinning the shipped binary. Low residual risk for
  a developer-tooling surface; the tradeoff is recorded in the plan's Migration
  Notes.
- Focus review on `is_plugin_invocation` (the silent-failure trap on conversion)
  and the `bare_invocation.py` wrapper regex, which is anchored to the leading
  command token so prose naming the launcher is not flagged.

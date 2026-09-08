---
type: "plan-validation"
id: "2026-09-06-0245-invoke-accelerator-directly-in-skills-validation"
title: "Validation Report: Invoke Accelerator Directly In Skills"
date: "2026-09-06T21:26:30+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-09-06-0245-invoke-accelerator-directly-in-skills"
target: "plan:2026-09-06-0245-invoke-accelerator-directly-in-skills"
tags: ["skills", "cli", "lint"]
last_updated: "2026-09-06T21:26:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Invoke Accelerator Directly In Skills

### Implementation Status

All three phases are fully implemented and land as one atomic commit each, in
plan order. Every automated success criterion passes; one manual spot-check
remains unexecuted because it requires a live skill invocation this validation
cannot perform.

- ✅ Phase 1: Reconcile the parser and permissions lint, convert all skills — fully implemented (commit `omxrrrmotrvx`)
- ✅ Phase 2: Add the bare-invocation allowlist lint, wire into check — fully implemented (commit `rzrrnllkokmw`)
- ✅ Phase 3: Close 0107 and record the linkage — fully implemented (commit `mzlxmkowuryw`)

### Automated Verification Results

✅ Acceptance grep 1 — no prefix: `grep -rF '${CLAUDE_PLUGIN_ROOT}/bin/accelerator' skills` → 0
✅ Acceptance grep 2 — no path suffix: `grep -rF '/bin/accelerator' skills` → 0
✅ Acceptance grep 3 — no shell wrapper: `grep -rEn '(^|[^[:alnum:]_])(bash|sh|env) [^`]*accelerator' skills` → 0
✅ Coupled + new unit tests: `pytest test_skill_permissions test_skill_parsing test_dispatch_coherence test_integration_skills test_bare_invocation test_mise` — 204 passed
✅ Integration conformance suite: `mise run test:integration:skill-invocation` — 128 passed
✅ Permissions lint: `mise run lint:skill-permissions:check`
✅ Bare-invocation lint: `mise run lint:bare-invocation:check`
✅ Write-gate / keyword-parity lint: `mise run lint:integration-skills:check`
✅ Dispatch guard: `mise run lint:dispatch-coherence:check`
✅ Build-system component (carries the new gate): `mise run build-system:check`
✅ Frontmatter valid: `corpus frontmatter validate` on both `0107` and `0245`

Reintroduction guard exercised directly, not merely read:

✅ A scratch skill with the `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` prefix makes `lint:bare-invocation:check` exit 1, naming `file:line` and the offending form.
✅ A scratch skill wrapping the call in `sh -c "…"` also exits 1 with the shell-wrapped-launcher message. Both scratch skills removed.

### Code Review Findings

#### Matches Plan

- **Parser flip** (`tasks/shared/skill_parsing.py`) lands exactly as specified: `LAUNCHER = "accelerator"`, `FORBIDDEN_LAUNCHER = f"{PLUGIN_PREFIX}bin/accelerator"`, and `is_plugin_invocation` returns `command == LAUNCHER or command.startswith((PLUGIN_PREFIX, f"{LAUNCHER} "))` — bare form recognised, `${CLAUDE_PLUGIN_ROOT}/` prefix still recognised for the five non-accelerator plugin references.
- **Shared config markers** (`CONFIG_MARKER`, `CONTEXT_SKILL_MARKER`, `CONTEXT_ANY_MARKER`, `INSTRUCTIONS_MARKER`) are defined once in `skill_parsing.py` and imported by both `skill_permissions.py` and the integration corpus, as the plan required.
- **Write-gate matcher** (`skill_write_gate.py:32`) retargeted to `rf"\baccelerator {provider} (?:{verbs})\b"`.
- **New lint** (`tasks/lint/bare_invocation.py`) exposes a pure `violations(root)` plus a thin `@task check`, matching the sibling-guard shape; derives `_PATHED_SUFFIX` from `FORBIDDEN_LAUNCHER` rather than re-typing the literal.
- **Six-step wiring** complete: `lint/__init__.py`, `tasks/__init__.py`, the `mise.toml` façade, membership in `build-system:check` (`mise.toml:511`) and `lint:check` (`mise.toml:653`), and `_BUILD_SYSTEM_CHECK_GATES` in `test_mise.py`.
- **Hooks boundary** recorded durably in a new `hooks/CLAUDE.md`; `hooks/hooks.json` correctly retains the pathed form (4 occurrences).
- **0107 closed** to `status: done` with `relates_to` naming `work-item:0245` and a prose supersession note — `superseded` correctly avoided as an invalid work-item status.

#### Deviations from Plan

- **Wrapper regex is tighter than the plan's sketch, and correct.** The plan proposed `^\s*(?:bash|sh|env)\b` after prefix stripping; the implementation is `^(?:bash|sh|env)\b.*\baccelerator(?=\s|$)` over `line.lstrip().removeprefix("!`")`. Requiring `accelerator` to actually appear on the line narrows the match and removes any residual prose-flagging risk. Behaviourally stronger, not weaker.

#### Potential Issues

- ✅ **Version floor now traced (resolved post-validation).** The Claude Code changelog documents the plugin `bin/` PATH addition at **v2.1.91** (`https://code.claude.com/docs/en/changelog#2-1-91`), below the declared minimum **v2.1.144**. Precondition-gate step 1 is therefore satisfied: bin-on-PATH exists at the supported floor. `0245`'s Open Questions still record the floor as accepted-untraced (`meta/work/0245-…:112-113,146`) — that record should be updated to cite v2.1.91. The empirical gate additionally confirmed the `!`-preprocessor surface (which the changelog does not name explicitly) resolves in a real session, so both surfaces are covered.
- The bare grant authorises whatever `accelerator` resolves first on PATH; the security tradeoff is recorded and accepted in the plan's Migration Notes. No new concern.

### Manual Testing Required

1. Live skill load (the one remaining unchecked plan item):
  - [ ] Invoke `/accelerator:visualise` in a real session and confirm it loads and runs with **no** permission prompt, emitting identical output to the pre-conversion form.
  - [ ] Invoke one config-reading skill in a subagent context and confirm the bare `!`-preprocessor call resolves.

### Recommendations

- Execute the single outstanding manual spot-check before merge; every other criterion is green.
- Update `0245`'s Open Questions to cite the traced floor (v2.1.91, per the changelog), replacing the accepted-untraced wording now that the guarantee is confirmed. Not blocking.
- The follow-on tidy-ups the plan deferred (shared `rglob("SKILL.md")` helper, now recurring in five modules; hosting the rule inside `call_site_migration.py`) remain open as intended.

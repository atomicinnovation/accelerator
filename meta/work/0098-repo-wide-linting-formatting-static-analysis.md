---
type: work-item
id: "0098"
title: "Repo-Wide Linting, Formatting, And Static Analysis Guardrails"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: task
priority: medium
tags: [tooling, linting, formatting, static-analysis, ci, guardrails]
relates_to: ["work-item:0090"]
last_updated: "2026-06-09T19:53:06+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-120
---

# 0098: Repo-Wide Linting, Formatting, And Static Analysis Guardrails

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Set up linting, formatting, and static-analysis tooling across the whole
repository — the visualiser frontend (TypeScript/React) and server (Rust),
plus the wider codebase (Python and shell scripts). The goal is two-fold:
add durable guardrails so issues are caught going forward, and fix all
existing issues across the repository so the tools pass cleanly from day
one — each tool passes cleanly from the moment its check becomes
blocking, with no grandfathered violations. The Requirements enumerate
the complete set of covered file types; anything not listed there is out
of scope.

## Context

The repository is polyglot and currently lacks consistent automated quality
guardrails:

- **Visualiser frontend** — `skills/visualisation/visualise/frontend/`,
  TypeScript + React 19 + Vite + Vitest. No ESLint/Prettier (or Biome)
  configured; `tsc` type-checking runs only as part of `build`.
- **Visualiser server** — `skills/visualisation/visualise/server/`, Rust
  (Cargo). No enforced `rustfmt`/`clippy`.
- **Python** — ~700+ files across skills/scripts, with `pyproject.toml`
  present (pytest + uv configured) but no linter/formatter/type-checker
  (e.g. ruff, mypy).
- **Shell** — ~160 `.sh` scripts with no shellcheck/shfmt enforcement.
- Other candidates: CSS (stylelint), and `.editorconfig` already exists and
  should be honoured by chosen tooling.

## Requirements

- **Frontend**: Biome (v2.x) as the single lint + format tool —
  `recommended` rules on, all applicable domains enabled (`react`, `test`,
  `project`), severities escalated so CI fails on warnings
  (`biome ci --error-on-warnings` or explicit severity config). Enforce
  `tsc --noEmit` as a standalone type-check.
- **Server (Rust)**: enforce `cargo fmt --check`; configure clippy via the
  Cargo `[lints.clippy]` table — default lints denied, `pedantic` at warn
  with `priority = -1` and a short curated override list (`allow`s of
  named impractical pedantic lints, each with a justification comment); CI
  enforces with `-D warnings`.
- **Python**: ruff (lint + format) configured for maximal strictness —
  `select = ["ALL"]` with targeted, documented ignores and a pinned ruff
  version; pyrefly (stable 1.0) as the type-checker with the `strict`
  preset, configured under `[tool.pyrefly]` in the existing
  `pyproject.toml`.
- **Shell**: shellcheck with all optional checks enabled via a repo-root
  `.shellcheckrc` (`enable=all`, plus any documented `disable=` entries) —
  the rc file is the authoritative locus, so plain `shellcheck` picks it
  up everywhere; shfmt in diff mode (`shfmt -d`) with style sourced from
  `.editorconfig` (which shfmt reads natively).
- **CSS**: frontend CSS covered by Biome's CSS lint/format support via
  `lint:frontend` — no separate stylelint. CSS outside the frontend, if
  any appears, is out of scope for this item.
- **Out of scope**: all other file types (e.g. Markdown, YAML, TOML, JSON,
  GitHub Actions workflows) — considered and excluded from this item; any
  can be added later as follow-ups on the infrastructure this item
  establishes.
- **`.editorconfig`**: honoured where tools support it (Biome — on by
  default in v2; shfmt — native); for tools that don't (ruff, rustfmt),
  duplicate line-length/indent settings in their config with a comment
  cross-referencing `.editorconfig`. Sync is by manual convention — no
  automated consistency check is in scope.
- **Fix all existing issues** so every configured tool passes cleanly
  across the entire repository. "All" means every version-controlled file
  of the relevant type (e.g. every tracked `.py` and `.sh` file);
  `workspaces/` (jj workspace checkouts) is the only anticipated
  exclusion, and any exclude entry added to a tool config must carry an
  adjacent comment justifying it.
- **Entry point**: mise tasks following the repo's namespacing —
  `lint:frontend` (`biome ci` + `tsc --noEmit`), `lint:rust`
  (`cargo fmt --check` +
  `cargo clippy --all-targets --all-features -- -D warnings`),
  `lint:python` (`ruff check` + `ruff format --check` + `pyrefly check`),
  `lint:shell` (shellcheck + `shfmt -d`), plus an aggregate `lint` task
  using `depends` on all four. Type-checks live inside the per-language
  tasks. Documented via `description` fields on the tasks in `mise.toml`,
  matching the existing test tasks.
- **CI**: wire all checks into the existing GitHub Actions Main CI as
  blocking jobs (tools installed via mise), structured as separate
  per-language jobs mirroring the `lint:*` tasks so each can be enabled —
  and made blocking — independently if the work lands in stages. Each CI
  job executes its `mise run lint:<language>` task rather than duplicating
  commands in workflow YAML, so the tasks stay the single source of
  truth. CI-only enforcement — no pre-commit hooks.
- **Delivery shape**: a sequence of per-language PRs (config + sweep + CI
  job per language, aggregate task and documentation last) is the
  default; a single PR is acceptable if remediation volume allows. A
  "sweep" is a language's full remediation — mechanical auto-fixes,
  manual fixes, and any suppressions — and a "sweep PR" is the PR that
  lands it. The acceptance criteria are phrased to hold under either
  shape.

## Acceptance Criteria

- [ ] `biome ci` (strict config, react + test + project domains) and
  `tsc --noEmit` pass on the frontend with zero diagnostics.
- [ ] `cargo fmt --check` passes and
  `cargo clippy --all-targets --all-features -- -D warnings` passes on the
  server.
- [ ] `ruff check`, `ruff format --check`, and `pyrefly check` (strict
  preset) pass with zero errors across all Python.
- [ ] shellcheck (optional checks enabled) and `shfmt -d` pass on all `.sh`
  scripts.
- [ ] Committed configs match the mandated strictness: Biome enables the
  react, test, and project domains with warnings escalated (via severity
  config, or `--error-on-warnings` in the committed mise task and CI
  invocations — either committed locus counts); the Cargo
  `[lints.clippy]` table denies default lints and sets `pedantic` to warn
  with `priority = -1`; ruff selects `ALL`; pyrefly uses the `strict`
  preset; the repo-root `.shellcheckrc` enables all optional checks.
- [ ] ruff and rustfmt configs set line-length/indent values matching
  `.editorconfig`, each accompanied by a comment cross-referencing
  `.editorconfig`.
- [ ] Every config-level ignore and inline suppression — any mechanism
  that silences a configured check, including `# noqa`, `#[allow(...)]`,
  `// biome-ignore`, `# shellcheck disable=`, `# pyrefly: ignore`,
  `# type: ignore`, and `// @ts-expect-error` — carries an adjacent
  comment naming the rule and rationale, and each sweep PR's description
  enumerates the suppressions it adds.
- [ ] `mise run lint` runs all four per-language tasks (lint, format
  checks, and type-checks per the Entry point mapping) and exits zero;
  each `lint:*` task and the aggregate carries a `description` field in
  `mise.toml`, matching the existing test tasks.
- [ ] Main CI runs the same checks as separate blocking per-language jobs
  (no `continue-on-error`); verified by introducing one representative
  violation per check command (e.g. for Python: one ruff lint violation,
  one formatting violation, one type error), observing the corresponding
  job exit non-zero, and linking each failing probe run from the relevant
  PR description.
- [ ] Every tool has exactly one authoritative pin at an exact version (no
  range operators): biome and typescript in the frontend `package.json`
  (lockfile committed); ruff, pyrefly, shellcheck, shfmt, and the Rust
  toolchain (already pinned there, supplying rustfmt and clippy) in
  `mise.toml` — so rule sets can't drift on upgrade.
- [ ] Coverage matches the repository: the file sets checked by ruff,
  pyrefly, and the shell tools each cover every tracked `.py`/`.sh` file
  (cross-checked against `git ls-files`, minus `workspaces/`), and every
  exclude entry in any tool config — including `workspaces/` — carries an
  adjacent justification comment.
- [ ] No pre-existing violations remain in a language area once its lint
  job is made blocking, and none remain anywhere in the repository when
  the final job becomes blocking at item close.
- [ ] A follow-up work item re-evaluating pyrefly's `all` preset (slated
  for v1.1) is raised before this item closes.

## Open Questions

None — the four original questions (frontend toolchain, strictness, CI
wiring/pre-commit, sweep commits) were resolved on 2026-06-09; see Drafting
Notes.

## Dependencies

- Related: 0090 (done) — its standing radius-token CI grep gate could be
  consolidated into the lint job structure this item establishes;
  consolidation was considered and is deliberately left to future
  discretion, not gated on this item.
- External: the mise registry must provide pinned versions of ruff,
  pyrefly, shellcheck, and shfmt; fall back to mise backends
  (`cargo:`/`npm:`/`pipx:`) for any tool it lacks.
- External: biome and typescript are supplied via npm, pinned in the
  frontend `package.json` with its committed lockfile — the second supply
  channel alongside the mise registry.
- External: GitHub Actions and the third-party `jdx/mise-action` step are
  the CI install channel (pre-existing infrastructure); the action is
  pinned to a version consistent with the exact-pin policy.
- External: pyrefly roadmap — re-evaluate the `strict` vs `all` preset
  when v1.1 ships; `strict` governs this item regardless, with a follow-up
  item to be raised at completion.
- Ordering: the per-language auto-fix sweep commits should land when no
  (or minimal) work is in flight in the affected language areas; any
  in-flight items must rebase across the sweep. The implementer
  enumerates in-flight items per affected language area immediately
  before each sweep lands, or agrees the sweep window with their owners.

## Assumptions

- "Strict from day one" is interpreted per-tool as the strictest
  *practical* configuration: Biome has no "all rules" switch in v2, so
  strict means recommended + domains + warnings-as-errors; blanket-denying
  `clippy::pedantic` is widely considered impractical, so pedantic surfaces
  as warnings promoted to CI failures with curated overrides; ruff gets
  literal `ALL` minus documented ignores; pyrefly uses its `strict` preset.
  If strict was meant even harder than this, scope grows materially.
- The repository's CSS footprint is plain CSS (no SCSS) — Biome doesn't
  support SCSS yet, so this assumption underpins dropping stylelint.
- Fixing all existing violations across ~700 Python files and ~160 shell
  scripts is acceptable within this single task (per the scoping decision
  to keep 0098 unsplit).
- Contingency: before the first sweep PR, run each tool in report-only
  mode and record per-language violation counts; if any language's
  remediation proves unmanageable (as a guide: a sweep diff too large to
  review as a handful of commits), this item re-scopes to per-language
  children (config + remediation + blocking CI job landing together per
  language), with the aggregate `lint` task and documentation as a final
  child.
- Main CI's status is already required for merge, so making a lint job
  blocking needs no branch-protection change; if staged enablement turns
  out to require required-check updates, that wiring is part of this
  item's CI work.

## Technical Notes

- Biome 2.4.x: type-aware linting since v2; React hooks rules
  (`useExhaustiveDependencies`, `useHookAtTopLevel`) live in the `react`
  domain, auto-enabled when React is in `package.json`; reads
  `.editorconfig` by default; v2 downgraded `style` rules to warn —
  escalate via `--error-on-warnings` or explicit severities.
- pyrefly 1.0 (stable May 2026): config under `[tool.pyrefly]`; the
  `strict` preset enables implicit-Any bans, `unused-ignore`, and
  `strict-callable-subtyping`; `ignore-missing-imports` /
  `replace-imports-with-any` handle untyped third-party deps;
  `--output-format github` gives CI annotations; an `all` preset is slated
  for v1.1 — `strict` governs this item regardless of whether `all` ships
  first; evaluating `all` is a follow-up (see Dependencies).
- ruff is pre-1.0 (0.15.x) — `ALL` implicitly enables new rules on
  upgrade, hence the hard version-pin requirement; ruff does not read
  `.editorconfig` (open issue since 2023).
- shfmt only applies `.editorconfig` when no style flags are passed on the
  CLI — CI must run plain `shfmt -d`.
- mise installs the non-frontend tools (ruff, pyrefly, shellcheck, shfmt)
  from its registry — CI setup stays a single `jdx/mise-action` step plus
  the frontend's existing npm install, which provides biome and typescript
  via `package.json`.
- Recommended process: apply mechanical auto-fixes (formatters,
  `ruff --fix`, etc.) as separate per-language commits, distinct from
  config commits, to keep diffs reviewable.

## Drafting Notes

- Decisions confirmed by the author on 2026-06-09: Biome for the frontend;
  strict-from-day-one rule sets with pyrefly (not mypy) for Python
  type-checking; CI-only enforcement (no pre-commit hooks); kept as a
  single task rather than splitting into an epic.
- "Strict" was mapped to concrete per-tool configurations (see
  Assumptions) because several tools lack a literal all-rules switch.
- The stylelint requirement was dropped because Biome lints and formats
  CSS natively.
- ruff `ALL` + ignores + version pin was chosen over Astral's
  docs-preferred curated select to honour the strict mandate — a reviewer
  may reasonably flip this.
- The original "sweep commit" open question was downgraded to a process
  recommendation in Technical Notes rather than kept as a blocker.
- Review 1 (2026-06-09, verdict REVISE) findings addressed the same day:
  AC1 gained the `project` domain; the open-ended "other file types"
  phrase was replaced with an explicit out-of-scope bullet;
  strictness-verification and suppression-policy ACs were added; the
  Dependencies section was populated (0090, mise registry, pyrefly
  roadmap, sweep ordering); pin ownership, the task-to-command mapping,
  the `.editorconfig` sync convention, a CI falsification probe, the
  file-discovery scope, and a sizing contingency were specified. Kind
  deliberately left as `task` per the unsplit decision, acknowledging the
  reviewer's note that it undersells the breadth.
- Re-review pass 2 (2026-06-09, verdict REVISE) fixes applied the same
  day: delivery shape made explicit (per-language PRs by default) and the
  suppression-enumeration and zero-violations ACs rephrased to hold under
  staged landing; the Rust toolchain's existing `mise.toml` pin added to
  the pin AC; the `lint:rust` mapping now carries the full clippy
  invocation; Biome warning escalation accepted from either committed
  locus; coverage, `.editorconfig`-sync, and pyrefly-follow-up ACs added;
  CSS scoped to the frontend; the npm supply leg and an in-flight
  enumeration step added to Dependencies; the orphaned "full suite"
  definition removed and the documentation target named (`mise.toml`
  task descriptions).
- Re-review pass 3 (2026-06-09, verdict REVISE) fixes applied the same
  day: the CI falsification probe tightened to one violation per check
  command; pyrefly added to the coverage cross-check; the exclude-comment
  rule aligned (every exclude entry, including `workspaces/`, needs a
  justification comment); `.shellcheckrc` made the authoritative shell
  locus and added to the strictness AC; CI jobs now execute the mise
  tasks rather than duplicating commands; "sweep" defined in the Delivery
  shape bullet; the suppression-marker list made open-ended and extended
  with pyrefly/tsc syntaxes; `jdx/mise-action` captured as a third supply
  channel; a report-only dry-run checkpoint added to the re-scope
  contingency; branch-protection assumption recorded; 0090 consolidation
  explicitly left to future discretion; "from day one" anchored to each
  check's blocking moment. Closing verdict recorded as COMMENT: the sole
  remaining major (the unsplit bundle) is answered by the documented
  author decision.

## References

- Biome v2 announcement: https://biomejs.dev/blog/biome-v2/
- Biome v2.4 release notes: https://biomejs.dev/blog/biome-v2-4/
- Biome linter docs (domains, severities): https://biomejs.dev/linter/
- pyrefly 1.0.0 release: https://github.com/facebook/pyrefly/releases/tag/1.0.0
- pyrefly configuration (strict preset): https://pyrefly.org/en/docs/configuration/
- ruff linter docs (rule selection guidance): https://docs.astral.sh/ruff/linter/
- ruff versioning policy: https://docs.astral.sh/ruff/versioning/
- Clippy lint groups: https://doc.rust-lang.org/clippy/lints.html
- Clippy `[lints.clippy]` configuration: https://doc.rust-lang.org/clippy/configuration.html
- shfmt man page (EditorConfig support): https://man.archlinux.org/man/extra/shfmt/shfmt.1.en
- mise tasks: https://mise.jdx.dev/tasks/

---
date: "2026-04-17T17:15:00+01:00"
researcher: Toby Clemson
git_commit: 9f076a60
branch: visualisation-system
repository: accelerator (visualisation-system workspace)
topic: "Meta visualiser v1 — implementation context and phasing"
tags: [ research, codebase, visualisation-system, meta, skill, http-server, sse, react, kanban, rust, axum, github-releases ]
status: complete
last_updated: "2026-04-18"
last_updated_by: Toby Clemson
last_updated_note: "Later on 2026-04-18: revised the 'Major gaps' framing — gap items are now explicitly owned by the phase plan that touches them (Gap 1 → Phase 1; Gaps 2/5/6 → Phase 2; Gaps 3/7 → Phase 12; Gap 4 already resolved by D10) rather than being decided wholesale during Phase 1 planning. Gap 1 is concretely resolved by the Phase 1 plan at `meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md`. Earlier same day: appended follow-up 'Consistency and gap analysis'; corrected the monorepo misconception (`workspaces/visualisation-system/` is a jj workspace, not a monorepo member — single plugin manifest at the repo root); added D9 (Templates view renders all three resolution tiers per template; `templates` is a virtual DocType backed by `config_resolve_template()`); and D10 (frontend embedded into the Rust binary via `rust-embed` — `frontend/dist/` is gitignored, not committed; `dev-frontend` Cargo feature swaps to disk-based `ServeDir` for local iteration). D10 updates D1, D2's tree layout, D3's stack table, Phases 5 and 12, the Open questions list, and resolves follow-up gap #4 (committed-dist drift). Later on 2026-04-18: tightened D4 (and the §1 intro, the jj-workspace architecture insight, the Open-questions recap, and the Code references) to spell out **strict workspace-isolation rules** — Claude must treat the `visualisation-system` workspace root as the sole repository root, never read from or write to any other checkout of the repo on disk, and resolve all relative paths in this document against the workspace root only. Any absolute `/Users/…/accelerator/…` paths that remain are historical artefacts of earlier research gathering, not instructions to step outside the workspace."
---

# Research: Meta visualiser v1 — implementation context and phasing

**Date**: 2026-04-17 17:15 BST
**Researcher**: Toby Clemson
**Git Commit**: `9f076a60` (working copy, atop `17f65bf main — 1.19.0-pre.2`)
**Branch**: `visualisation-system` (jj bookmark)
**Repository**: `accelerator` plugin (working inside the
`visualisation-system` jj workspace — an alternate working-copy view of
the same repo, not a separate subpath)

## Research question

> Based on `meta/specs/2026-04-17-meta-visualisation-design.md`, provide full
> context of the changes required to build v1 of the visualisation system and a
> suggested phasing in small, well-defined phases. Flag and resolve any spec
> uncertainties — either through research or by asking for clarification.

## Summary

The spec is well-grounded but lands on a codebase that will need **new
ground to be broken** in several areas: the accelerator plugin today
contains **zero** JavaScript/TypeScript/Node/Rust code, has **no
precedent** for spawning long-running processes or PID-file management, and
has **no slash-command / SKILL.md** that runs anything other than
short-lived bash. The closest precedent the spec cites —
`superpowers:brainstorming` — is not bundled with this plugin but is
available on disk at
`~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/brainstorming/`
and is **a direct structural template** we should model the launch pattern
after. (Its implementation is Node; we model the *pattern*, not the code.)

Ten design decisions have now been resolved and locked in (see
**Resolved design decisions** below). The most impactful:

- The server is implemented in **Rust** (axum + tokio + notify + gray_matter
  + serde_yml + sha2), shipped as per-arch static binaries.
- Binaries are **not** committed; they are distributed via **GitHub
  Releases** with a SHA-256 manifest committed in the plugin tree.
  `launch-server.sh` fetches the right binary on first use per plugin
  version, verifies it, and caches it locally.
- End users need **no** runtime dependency (Rust, Node, npm) on their
  machine — only a `curl` on first run.

Six spec assumptions do not match reality and need adjustment before v1
implementation:

1. **Path keys**: spec lists 9 doc types; the plugin's config system
   exposes **12** (`reviews` is split into `review_plans` + `review_prs` +
   `review_tickets`; `tmp` is the 12th).
2. **Reviews directory is nested**: `meta/reviews/plans/` and (future)
   `meta/reviews/prs/`. Spec's flat-directory assumption breaks — resolved
   via D5 (three separate DocTypes, each flat).
3. **Templates resolve across three tiers, not one directory**: templates
   aren't a single flat walk. `config_resolve_template()` in
   `scripts/config-common.sh:152-192` applies winner-first precedence:
   (1) `templates.<name>` config override (highest) — a path value in
       `.claude/accelerator.md` / `.claude/accelerator.local.md`,
   (2) `<paths.templates>/<name>.md` userspace override (middle, default
       `meta/templates/<name>.md`),
   (3) `<plugin-root>/templates/<name>.md` plugin default (lowest).
   The five known template names (`adr`, `plan`, `research`,
   `validation`, `pr-description`) are discovered by globbing the
   plugin-default dir (`config_enumerate_templates()` in
   `config-common.sh:102-112`); plugin defaults always exist, the other
   two tiers may or may not. **The visualiser renders all three tiers
   per template** (each with presence and provenance) so the user can
   preview what each template resolves to regardless of current
   configuration — see D9.
4. **Ticket status enum**: spec expects `todo | in-progress | done`; live
   data has only `todo | done`. `in-progress` has never been populated by
   any skill.
5. **Cross-reference frontmatter is almost entirely unpopulated**: only
   one field (`target:` on plan-reviews) contains a real cross-ref in the
   wild. The "declared links" branch still ships in v1 (per D7) but will
   only meaningfully render the `target:` bidirectional link for now.
6. **Frontmatter absence is common**: older plans (pre-2026-03-22) and 2
   of 3 notes have no frontmatter at all. The parser must tolerate this
   as a first-class case — not a "malformed" error.

Details, evidence, and a 12-phase implementation plan follow.

---

## Detailed findings

### 1. Plugin surface area — where new code has to land

All development for the visualiser happens **exclusively inside** the
`visualisation-system` jj workspace at
`…/accelerator/workspaces/visualisation-system/`. This is an **isolated
working copy** of the accelerator repo — a complete self-contained
checkout, not a subpath or monorepo member — containing everything
implementation needs: the plugin manifest (`./.claude-plugin/plugin.json`),
existing skills, agents, hooks, top-level scripts, and templates. Claude
treats this directory as the sole repository root; no other checkout of
the repo on disk is read from or written to. There is a single plugin
with a single `.claude-plugin/plugin.json` inside this workspace; no
per-workspace-vs-root manifest distinction exists. See **D4** below for
the full, strict isolation rules that govern implementation.

Current plugin surface (non-exhaustive, relevant categories only):

- **Skills** — 26 `SKILL.md` files grouped into categories: `config/`,
  `decisions/`, `github/`, `planning/`, `research/`, `review/lenses/` (13),
  `review/output-formats/` (2), `vcs/`.
- **Agents** — 7 markdown files under `agents/` (reviewer, codebase-locator,
  codebase-analyser, codebase-pattern-finder, documents-locator,
  documents-analyser, web-search-researcher).
- **Hooks** — `hooks/hooks.json` + three shell scripts (`vcs-detect.sh`,
  `vcs-guard.sh`, `config-detect.sh`).
- **Scripts (top-level)** — 21 config/VCS helpers under `scripts/`, all bash.
- **Templates** — 5 markdown templates at `<plugin-root>/templates/` (adr, plan,
  research, validation, pr-description).
- **No JS/TS/Node/Rust code anywhere.** No `package.json`, no
  `node_modules/`, no `dist/`, no `Cargo.toml`, no `target/`, no
  `.mcp.json`. The visualiser introduces all three (a Cargo workspace for
  the server, an npm package + Vite build for the frontend, and a GitHub
  Releases pipeline for binary distribution).

**Slash commands ARE SKILL.md files.** There is no separate `commands/`
directory. A skill becomes user-invocable (`/accelerator:<name>`) when its
frontmatter sets `disable-model-invocation: true`. The invocation name is
derived from the skill's folder name (e.g. `skills/config/init/SKILL.md` →
`/accelerator:init`), scoped by the plugin's `accelerator` namespace.

### 2. Path & config resolution — how the visualiser will learn where to read

The config system has three layers (full details in
`scripts/config-read-value.sh:1-130`, `scripts/config-common.sh:15-192`):

1. `<project_root>/.claude/accelerator.md` — team-shared YAML frontmatter
   config (committed).
2. `<project_root>/.claude/accelerator.local.md` — personal overrides (
   gitignored by `/accelerator:init`).
3. Plugin defaults baked into each `config-read-*.sh` script.

Precedence: local overrides team overrides default (last file wins).
`<project_root>` is resolved by `find_repo_root` in
`scripts/vcs-common.sh:8-18` — it walks up for `.jj` or `.git` and falls back to
`$PWD`.

**Path keys** (authoritative list from `scripts/config-read-path.sh:7-19`):

| Key              | Default                | Notes                                                                  |
|------------------|------------------------|------------------------------------------------------------------------|
| `plans`          | `meta/plans`           |                                                                        |
| `research`       | `meta/research`        |                                                                        |
| `decisions`      | `meta/decisions`       |                                                                        |
| `prs`            | `meta/prs`             | Directory not yet populated anywhere.                                  |
| `validations`    | `meta/validations`     | Directory does not exist on disk yet.                                  |
| `review_plans`   | `meta/reviews/plans`   | Not `reviews` — note the split.                                        |
| `review_prs`     | `meta/reviews/prs`     | Not yet populated.                                                     |
| `review_tickets` | `meta/reviews/tickets` | Added 2026-04-24 for ticket review skill.                             |
| `templates`      | `meta/templates`       | Empty in both locations; real templates at `<plugin-root>/templates/`. |
| `tickets`        | `meta/tickets`         |                                                                        |
| `notes`          | `meta/notes`           |                                                                        |
| `tmp`            | `meta/tmp`             | Ephemeral; gitignored via nested `.gitignore`.                         |

That is **12 keys, not 9**, and `reviews` is fundamentally split into three paths.
The spec's `DocTypeKey` union needs to accommodate that distinction — either as
`review_plans` + `review_prs` at the type level, or as a single `reviews` type
that internally unions two directories. See "Spec-vs-reality gaps" for the
recommended resolution.

### 3. `/accelerator:init` behaviour — what lives in a freshly-initialised project

`skills/config/init/SKILL.md:1-126`:

- Resolves all 12 path keys via `config-read-path.sh` in the frontmatter
  preamble (`SKILL.md:16-30`).
- Creates each directory with `mkdir -p` + `.gitkeep` (12 directories, including
  `meta/reviews/` as the implicit parent of `reviews/plans/` and
  `reviews/prs/`).
- Writes a **nested** `.gitignore` at `meta/tmp/.gitignore` with the content:
  ```gitignore
  # Ignore everything in this directory except the directory itself
  *
  !.gitkeep
  !.gitignore
  ```
  This pattern (per `SKILL.md:58-77`) lets the directory be tracked while its
  contents are ignored. Adding `meta/tmp/` to the root `.gitignore` is *
  *explicitly rejected** in the skill — it would stop git from descending into
  the directory, breaking the nested-ignore trick.
- Appends `.claude/accelerator.local.md` to the root `.gitignore`.
- Does **not** write a config file. Creation of `.claude/accelerator.md` is the
  responsibility of `/accelerator:configure`.

**Implication for the visualiser**: the visualiser's runtime state at
`<meta/tmp>/visualiser/` will be auto-gitignored by the nested pattern, provided the
user has run `/accelerator:init`. The `config-summary.sh` hook (
`scripts/config-summary.sh:19-21`) treats the presence of `<tmp>/.gitignore` as
the initialisation sentinel and emits an `INIT_HINT` if missing — the visualiser
preprocessor can reuse this sentinel to fail fast with a friendly message.

### 4. Bash preprocessor pattern — the canonical structure we must follow

Every existing slash-command SKILL.md follows the same shape. Canonical
reference: `skills/github/review-pr/SKILL.md:1-36`.

```markdown
---
name: review-pr
description: ...
argument-hint: "[PR number or URL]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

# Review PR

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh review-pr`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

**PR reviews directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs meta/reviews/prs`
**Tmp directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp meta/tmp`

**IMPORTANT**: Wherever `{tmp directory}` appears … substitute the actual
resolved path …
```

Rules:

- Each preamble line is a **single-backtick** bash one-liner prefixed with `!`.
- Output is exposed via bold Markdown labels and referenced later as `{...}`
  placeholders.
- `${CLAUDE_PLUGIN_ROOT}` (NOT a relative path) is the variable.
- `allowed-tools` frontmatter whitelists the bash scripts with a glob so the
  hook permits them.
- Skills end with `!`$
  {CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh <skill>`` to
  pull user overrides.

`/accelerator:visualise` must follow this exact shape. `/accelerator:init` (
`skills/config/init/SKILL.md:1-31`) is the simplest reference — pure path
resolution only. `skills/decisions/create-adr/SKILL.md:1-25` shows how to extend
`allowed-tools` for skill-local scripts.

### 5. Skill-local helper script pattern

`skills/decisions/scripts/adr-next-number.sh` (pattern at lines 1-50) shows how
skill-local scripts bootstrap themselves:

```bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/vcs-common.sh"
# Call into scripts/config-read-path.sh for configured directories, then
# resolve relative paths against find_repo_root.
```

The visualiser's preprocessor(s) will follow the same idiom:
`skills/visualisation/visualise/scripts/launch-server.sh` discovering
`PLUGIN_ROOT` from its own location, sourcing `vcs-common.sh`, and invoking
`config-read-path.sh` for each of the 12 keys.

### 6. The direct precedent: `superpowers:brainstorming`

The spec cites `superpowers:brainstorming` as a precedent. The plugin is not
present in accelerator but **is installed on disk** at
`~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/brainstorming/`
and has been **recently used by the author** — live session artefacts exist in
this very workspace at `.superpowers/brainstorm/75139-1776434327/` (server.pid,
server.log, server-stopped, ui-structure.html, waiting.html). The log shows the
user brainstormed the very visualiser we are now designing: clicks on options
labelled "Library / Lifecycle / Kanban", "A · Mode switcher on top, tri-pane
below", "B · One sidebar, everything is a 'view'".

**Concrete pattern** (from `skills/brainstorming/scripts/`):

- **`start-server.sh`** (149 lines) — bash preprocessor that:
  - Parses `--project-dir`, `--host`, `--url-host`, `--foreground`,
    `--background`.
  - Auto-detects Windows (MSYS/Cygwin) and Codex (`CODEX_CI` env) and switches
    to `--foreground` to survive process reaping.
  - Creates session dir at
    `<project-dir>/.superpowers/brainstorm/<pid>-<timestamp>/{content,state}/`.
  - Kills any pre-existing server at the stored PID file.
  - Resolves the **harness PID** as grandparent of the script's shell (
    `ppid of ppid`) so the server can watch it and self-terminate when the
    harness exits.
  - Backgrounds with `nohup … &` + `disown`.
  - Polls the log file for `"server-started"` then performs a liveness check (
    20 × 100ms) to catch reaper kills.
  - Emits the one-line JSON startup message to stdout so the caller (skill body)
    can parse it.

- **`server.cjs`** (352 lines) — **pure Node.js stdlib** (`http`, `fs`, `path`,
  `crypto`). No Hono, no chokidar, no gray-matter. Uses:
  - A hand-rolled WebSocket implementation (spec uses SSE, which is simpler).
  - `fs.watch` (not chokidar) for file events, with 100ms per-file debounce.
  - Random port in `49152 + random(16384)` range (spec says "dynamic port" —
    same idea, different range).
  - Env-var config (`BRAINSTORM_DIR`, `BRAINSTORM_HOST`, `BRAINSTORM_PORT`,
    `BRAINSTORM_URL_HOST`, `BRAINSTORM_OWNER_PID`).
  - Writes `state/server-info` with the full startup payload +
    `state/server-stopped` on exit (reason included).
  - **Owner-PID polling every 60s** — shuts down if the harness dies (
    `process.kill(ownerPid, 0)`). The spec does not mention this; it's a
    robustness feature worth adopting.
  - 30-minute idle timeout (matches spec exactly).

- **`stop-server.sh`** (57 lines) — SIGTERM, 2s grace (20 × 100ms), SIGKILL
  escalation, clears PID file and log. Deletes session dir only if under
  `/tmp/` — persistent sessions under `.superpowers/` are preserved.

**What to take from this precedent**:

1. The bash preprocessor design — arg parsing, harness-PID resolution,
   nohup/disown, log-poll-for-ready.
2. The `{session_dir}/{content,state}` subdirectory layout — the spec's
   `<meta/tmp>/visualiser/` can mirror this as
   `<meta/tmp>/visualiser/{server-info.json, server.pid, server.log, server-stopped.json}`.
3. The owner-PID watch — better than idle-only timeout.
4. The graceful-then-forced shutdown pattern.

Explicitly **not** taken from this precedent: superpowers' Windows/MSYS and
Codex auto-foreground detection. Accelerator today targets macOS/unix-based
systems and Claude Code only — no Windows support, no Codex/Gemini/other
harness support. The visualiser inherits that scope. Revisit if accelerator's
supported platforms ever expand.

**Where the precedent and the v1 direction differ**:

The superpowers server is pure-Node-stdlib; the visualiser is Rust. Both
are just different implementations of the same structural pattern. The
table below captures the equivalences.

| Choice                | Superpowers (Node)          | Visualiser v1 (Rust)                              | Notes                                                                                                                                       |
|-----------------------|-----------------------------|---------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------|
| Transport to browser  | WebSockets                  | SSE                                               | One-way is sufficient for doc-changed broadcasts; axum has first-class SSE.                                                                |
| HTTP framework        | Node `http` (hand-rolled)   | `axum`                                            | Tokio-native; trait-based routing composes with the file-driver trait.                                                                      |
| File watcher          | `fs.watch`                  | `notify` crate                                    | FSEvents/inotify under the hood, cross-platform.                                                                                            |
| Frontmatter parsing   | N/A                         | `gray_matter` + `serde_yml`                       | Tolerates absent and malformed frontmatter as distinct states.                                                                              |
| Owner-PID watch       | `process.kill(pid, 0)` @60s | `nix::sys::signal::kill(pid, None)` @60s          | Same robustness pattern — shuts down when harness exits.                                                                                    |
| Session subdir layout | `content/` + `state/` split | `<tmp>/visualiser/` (single dir)                  | Single dir is simpler given there's no "content" half; matches accelerator's `<tmp>/<skill>/` idiom.                                        |
| Runtime dependency    | Node.js on user machine     | None — binary downloaded from GitHub Releases     | End users need no runtime installed; see D8.                                                                                                |
| Ship mechanism        | Plugin cache (Node source)  | GitHub Releases per-arch binary + committed SHA-256 manifest | Keeps the plugin repo small; versioning via release tags.                                                                                   |

### 7. Document types — what actually lives on disk

Agents 2 and 6 mapped the full reality. Summary (plugin root `meta/`):

| Type        | Dir                   | Files     | Filename pattern                                                                                                   | Frontmatter                                                                                                                                          |
|-------------|-----------------------|-----------|--------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------|
| decisions   | `meta/decisions/`     | 14        | `ADR-NNNN-<slug>.md`                                                                                               | `adr_id`, `date`, `author`, `status`, `tags` — consistent                                                                                            |
| tickets     | `meta/tickets/`       | 25        | `NNNN-<slug>.md`                                                                                                   | `title`, `type: adr-creation-task`, `status` — consistent                                                                                            |
| plans       | `meta/plans/`         | 27        | `YYYY-MM-DD-<slug>.md`                                                                                             | **inconsistent**: pre-2026-03-22 have NONE; post have `date`, `type: plan`, `skill: create-plan`, `ticket`, `status`                                 |
| research    | `meta/research/`      | 17        | `YYYY-MM-DD-<slug>.md`                                                                                             | rich: `date`, `researcher`, `git_commit`, `branch`, `repository`, `topic`, `tags`, `status`, `last_updated`, `last_updated_by` — consistent          |
| reviews     | `meta/reviews/plans/` | 6         | `YYYY-MM-DD-<slug>-review-N.md`                                                                                    | `date`, `type: plan-review`, `skill: review-plan`, `target` (full path!), `review_number`, `verdict`, `lenses`, `review_pass`, `status` — consistent |
| validations | **absent**            | 0         | template has no frontmatter                                                                                        | n/a                                                                                                                                                  |
| notes       | `meta/notes/`         | 3         | `YYYY-MM-DD-<slug>.md`                                                                                             | **inconsistent**: 2 of 3 have none; newest has `date`, `author`, `tags`, `status`                                                                    |
| prs         | **absent**            | 0         | template: `<number>-description.md`                                                                                | `date`, `type: pr-description`, `skill: describe-pr`, `pr_number`, `pr_title`, `status`                                                              |
| templates   | **empty** in `meta/`  | 0 in meta | real templates at `<plugin-root>/templates/adr.md`, `plan.md`, `research.md`, `validation.md`, `pr-description.md` | varies; `validation.md` has none                                                                                                                     |

Slug derivation (reality):

- **decisions** — strip `ADR-NNNN-` prefix. E.g.
  `ADR-0002-three-layer-review-architecture` →
  `three-layer-review-architecture`. ✓ matches spec.
- **tickets** — strip `NNNN-`. ✓ matches spec.
- **plans / research / notes / prs / validations** — strip `YYYY-MM-DD-`. ✓
  matches spec.
- **reviews** — strip `YYYY-MM-DD-` prefix AND the **last** occurrence of
  `-review-N` suffix (anchored to end of stem:
  `^\d{4}-\d{2}-\d{2}-(.+)-review-\d+$`, greedy `.+`). **Spec does not
  mention the suffix.** Without this, a plan with slug
  `remaining-configuration-features` would never cluster with its review file
  `2026-03-27-remaining-configuration-features-review-1.md`. The "last
  occurrence" rule ensures slugs that contain the literal substring
  `-review-` (e.g. `plan-review-process`) are handled correctly.
- **templates** — spec says no slug, excluded from lifecycle. ✓ matches template
  ergonomics.

Cross-reference inventory (from Agent 6, grep across all docs):

- `adr_id` — ADRs only; self-reference format `ADR-NNNN`.
- `supersedes` — in ADR template only, **zero live instances** populated.
- `ticket` — in plans only, every instance is `null`, `""`, or key omitted. *
  *Never populated.** Template placeholder is `"{ticket reference, if any}"` —
  format ambiguous.
- `target` — in plan-reviews only, always a full repo-relative path like
  `meta/plans/2026-03-27-remaining-configuration-features.md`. This is the *
  *only real cross-reference** in any frontmatter.
- `skill` — in plans/reviews/PR-descriptions, identifies the authoring skill —
  not an inter-artefact link.
- Absent entirely from frontmatter anywhere: `related`, `superseded-by`,
  `blocks`, `blocked-by`, `parent`, `epic`, `depends-on`, `plan`, `research`,
  `adrs`, `decision`, `pr`, `validates`, `reviews`.

**Implication**: the spec's "Explicit frontmatter links — when a doc declares
`ticket: 0001`, `supersedes: ADR-0002`" will render **nothing** in consumer
repos today. The lifecycle view will depend entirely on slug clustering for v1.
This is fine — the spec explicitly says the visualiser is "the forcing function
for improving that discipline" — but the UI must not be hollow when nothing is
declared.

Status values in tickets (authoritative — grepped across all 25): `todo` (12),
`done` (13). `in-progress` has **never** been written by any skill. Spec's
assumed triple is a superset; the "Other" swimlane will be empty today. This is
cosmetic, not blocking.

### 8. Specs/ directory — an unexpected 10th doc type

`workspaces/visualisation-system/meta/specs/` contains
`2026-04-17-meta-visualisation-design.md` (the visualisation spec). No `specs`
key exists in `config-read-path.sh`. The spec's 9-type enumeration does not
include it. Recommendation: **exclude from v1** and revisit as a roadmap item
when the specs directory becomes an established convention.

### 9. Prior art in `meta/`

Most load-bearing documents for this implementation (from Agent 5's medium +
high bands):

- `meta/plans/2026-03-23-template-and-path-customisation.md` — defines the 11
  path keys. Authoritative.
- `meta/plans/2026-03-23-config-infrastructure.md` — YAML frontmatter extraction
  rules (parser compatibility target).
- `meta/notes/2026-03-24-yaml-block-sequence-array-parsing.md` — array-parsing
  edge cases.
- `meta/research/2026-03-18-meta-management-strategy.md` — conceptual model for
  doc types.
- `meta/plans/2026-03-22-persist-review-artifacts.md` — review persistence
  rules; reason `reviews/` is nested.
- `meta/plans/2026-03-22-validation-crossref-frontmatter.md` — proposed
  cross-ref frontmatter (not yet implemented).
- `meta/tickets/0021-artifact-persistence-lifecycle.md` +
  `0022-artifact-metadata-and-lifecycle.md` — open ADR-creation tasks on
  lifecycle model; relevant if we later surface declared links.
- `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md` —
  `meta/tmp` convention.

---

## Spec-vs-reality gaps and recommended resolutions

| #   | Spec assumption                                                                 | Reality                                                                                              | Recommendation                                                                                                                                                                                                                                                                                                                                |
|-----|---------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| G1  | ~~9 doc types with single `reviews` key~~ (spec updated 2026-04-29)             | 12 path keys, `reviews` split into `review_plans` + `review_prs` + `review_tickets`                  | **Split at the DocType level** (D5): `plan-reviews`, `pr-reviews`, and `ticket-reviews` are distinct `DocTypeKey`s each reading from its own path. Slug derivation is identical for all three. ✅ Spec now aligned.                                                                                                                             |
| G2  | `GET /api/docs/:type/:path` and flat walking                                    | Reviews directory is nested (`meta/reviews/{plans,prs,tickets}/*.md`)                                | Each of `plan-reviews`, `pr-reviews`, and `ticket-reviews` walks its own flat directory. Non-recursive watches suffice per type once split — the nesting is absorbed by the three-type split.                                                                                                                                                  |
| G3  | Slug strips `YYYY-MM-DD-` for reviews                                           | Review filenames also have `-review-N.md` suffix                                                     | Add suffix-strip for reviews: `YYYY-MM-DD-(.+)-review-\d+\.md` → group 1 is the slug. Without this, reviews don't cluster with their targets.                                                                                                                                                                                                 |
| G4  | Wiki-links `[[ADR-NNNN]]` and `[[TICKET-NNNN]]`                                 | Tickets are `NNNN-...md` with **no** `TICKET-` prefix; ADRs do use `ADR-NNNN-`                       | **Spec form wins (D6)**: `[[ADR-NNNN]]` → resolve via index's `adr_id`/filename lookup; `[[TICKET-NNNN]]` → strip the `TICKET-` in the resolver and look up the ticket by its numeric `NNNN-` filename prefix. Bare `[[NNNN]]` is not supported — keeps the prefix available for future namespaces (e.g. `[[EPIC-NNNN]]`).                    |
| G5  | Ticket statuses `todo \| in-progress \| done` + Other                           | Only `todo` and `done` in the wild                                                                   | Keep spec's triple — Kanban's drag-drop will be the first thing to actually write `in-progress`. The "Other" swimlane is a no-op for now.                                                                                                                                                                                                     |
| G6  | Index entry for every doc assumes parseable frontmatter; malformed → "raw-only" | Many docs have **no frontmatter at all** (older plans, 2/3 notes, validation template)               | Differentiate three states: `frontmatter-parsed`, `frontmatter-absent` (normal — no banner), `frontmatter-malformed` (banner). Title falls back: `frontmatter.title` → first H1 → filename.                                                                                                                                                   |
| G7  | Templates rendered in library                                                   | Templates resolve via `config_resolve_template()` with three-tier precedence (config > userspace > plugin default); `meta/templates/` is typically empty until a user ejects; the five plugin defaults always exist at `<plugin-root>/templates/` | **Render all three tiers per template (D9)**: for each of the five known template names, the Templates view shows three entries — plugin default, userspace override, config override — each marked *present* or *absent* and tagged with which is the active winner. Plugin-default content is always shown (that tier is guaranteed present); users preview every template regardless of current configuration. |
| G8  | `specs/` directory not mentioned                                                | It exists in workspace meta                                                                          | Out of v1 scope; revisit as v2.                                                                                                                                                                                                                                                                                                               |
| G9  | Explicit frontmatter cross-references will appear as first-class links          | **Zero** populated cross-refs other than `target:` on reviews                                        | **Ship declared links in v1 (D7)**: render `target:` on plan-reviews as a first-class declared link, and render the reverse index (plan's library page lists reviews whose `target:` points at it). Inferred-vs-declared visual distinction implemented in full so other cross-ref fields activate automatically when authors populate them.  |
| G10 | Server binds `127.0.0.1` only                                                   | Superpowers precedent supports `--host 0.0.0.0` + `--url-host` for remote/containerised environments | **Accept spec's localhost-only default** for v1 (simpler, spec's security posture). Leave a flag hook for v2 to match superpowers.                                                                                                                                                                                                            |
| G11 | Server stack choice                                                             | No Node or Rust code in plugin today                                                                 | **Resolved via D3**: Rust (axum + tokio + notify + gray_matter + serde_yml + sha2). Shipped as per-arch static binaries via GitHub Releases (D8) so end users need no language runtime.                                                                                                                                                      |
| G12 | `sha256` ETag of full file                                                      | Feasible for ~2000 files                                                                             | ✓ keep as spec says; revisit if profiling demands xxhash.                                                                                                                                                                                                                                                                                     |

---

## Resolved design decisions (author confirmed 2026-04-17)

### D1. CLI distribution — in-plugin shell wrapper (Q1 → option b)

All sources — Rust server, frontend TS/React — live in this repo. Built
artefacts (`frontend/dist/` and the per-arch server binaries) are **not
committed**: the frontend bundle is embedded into the Rust binary at
`cargo build` time (see D10), and the binary itself is shipped via
GitHub Releases (D8). End users never run `npm install` or
`npm run build`, and they never need Rust or Node on their machine.
The `accelerator-visualiser` CLI ships as a shell wrapper committed in
the plugin tree; users who want it on `PATH` symlink it themselves.

**Implication for planning**: the pre-release build in Phase 12 is
`npm run build` followed by `cargo zigbuild --release --target
<quadruple>` for each of the four targets; the build script embeds the
fresh `frontend/dist/` into each binary. Checksums are computed over
those binaries, the manifest is committed, and the binaries are
uploaded as GitHub Release assets. No committed built artefacts in
the tree. The `accelerator-visualiser` wrapper is a thin shell script
that execs the same `launch-server.sh` the slash command uses.

### D2. In-plugin tree layout (Q2 → option A)

```
skills/visualisation/visualise/
├── SKILL.md
├── scripts/
│   ├── launch-server.sh      (bash — config + platform detect + binary fetch + exec)
│   └── stop-server.sh        (graceful shutdown via PID file)
├── server/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── build.rs              (verifies `../frontend/dist/` exists when
│   │                          the `embed-dist` feature is on)
│   ├── src/
│   │   ├── main.rs           (axum bootstrap, routes wiring)
│   │   ├── config.rs         (JSON config ingestion from preprocessor)
│   │   ├── file_driver.rs    (list/read/watch + canonicalize + prefix guard)
│   │   ├── indexer.rs        (scan · slug derive · SHA-256 ETag cache)
│   │   ├── watcher.rs        (notify crate + 100ms debounce)
│   │   ├── sse_hub.rs        (tokio::sync::broadcast)
│   │   ├── patcher.rs        (YAML-aware line patcher)
│   │   ├── assets.rs         (embed-dist feature: serves bundled frontend;
│   │   │                      otherwise serves from disk — D10)
│   │   └── routes/           (one file per API endpoint)
│   └── tests/                (axum oneshot integration tests)
├── frontend/
│   ├── package.json
│   ├── package-lock.json
│   ├── vite.config.ts
│   ├── src/                  (React source)
│   └── dist/                 (build output; gitignored — embedded into
│                              release binaries, not committed)
├── bin/
│   ├── checksums.json        (committed SHA-256 manifest per target)
│   └── accelerator-visualiser-* (runtime cache; gitignored — populated on first
│                              run by launch-server.sh from GitHub Releases)
└── cli/
    └── accelerator-visualiser (shell wrapper for CLI — D1)
```

Plugin manifest registration uses `skills/visualisation/visualise/` as the
skill directory. The `accelerator-visualiser` wrapper resolves its own
plugin root and invokes `scripts/launch-server.sh` identically to the slash
command's preprocessor. Two directories are *partially* gitignored:
- `bin/` — `checksums.json` tracked, per-arch binaries gitignored.
- `frontend/` — sources tracked, `dist/` gitignored (embedded into the
  server binary at release build time per D10).

### D3. Server stack — Rust (axum + tokio + notify + gray_matter + serde_yml + sha2)

The server is written in Rust rather than Node.js. End users need **no**
runtime dependency on their machine — binaries are fetched from GitHub
Releases on first run per plugin version (see D8 for the distribution
mechanics).

Stack:

| Concern | Crate | Notes |
|---|---|---|
| HTTP + routing | `axum` | Tokio-native; first-class SSE via `axum::response::sse`. |
| Async runtime | `tokio` | Standard. |
| File watcher | `notify` | Cross-platform: FSEvents on macOS, inotify on Linux. Chokidar equivalent. |
| YAML frontmatter | `gray_matter` + `serde_yml` | Direct port of gray-matter.js semantics. |
| SSE | built into axum | No extra dep. |
| Static file serving | `rust-embed` (release) / `tower-http::services::ServeDir` (dev) | Per D10: default release build embeds `frontend/dist/` into the binary via `rust-embed`; the `dev-frontend` Cargo feature swaps in `ServeDir` from disk for fast local iteration. |
| Hashing (ETag) | `sha2` | `Sha256` over file bytes. |
| Atomic file write | `tempfile::NamedTempFile::persist` | Sibling tempfile + rename. |
| Process / PID checks | `nix::sys::signal::kill(pid, None)` | `kill -0` equivalent for owner-PID watch. |
| CLI / config ingestion | `clap` + `serde_json` | Reads `config.json` written by the preprocessor. |
| Logging | `tracing` + `tracing-subscriber` | JSON layer writes to `server.log`. |

Binary size target: ~6-10 MB per arch with `cargo build --release` + LTO +
`strip`. Four archs × ~8 MB ≈ 30-40 MB total across all platforms — small
enough for GitHub Releases, too large to commit.

### D8. Binary distribution — GitHub Releases with committed checksum manifest

The Rust server is **not** committed as a binary to the plugin repo. It is
built, signed by SHA-256, and attached to a versioned GitHub Release.
`launch-server.sh` fetches the right binary on first invocation per plugin
version. Specifics:

- **Build**: maintainers run `cargo zigbuild --release --target <quadruple>`
  for four targets on a single macOS dev host:
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
  - `aarch64-unknown-linux-musl` (truly static, no glibc coupling)
  - `x86_64-unknown-linux-musl`
  `cargo-zigbuild` uses zig's `cc` as a cross-linker; one-time setup, then
  one command per target. A release helper script orchestrates all four.
- **Release asset naming**: `accelerator-visualiser-<os>-<arch>` — raw binaries, no
  tarball wrapping (avoids a pointless extraction step in the
  preprocessor). The four asset names are:
  - `accelerator-visualiser-darwin-arm64`
  - `accelerator-visualiser-darwin-x64`
  - `accelerator-visualiser-linux-arm64`
  - `accelerator-visualiser-linux-x64`
- **Release tag** matches the plugin version from
  `.claude-plugin/plugin.json` exactly (e.g. `v1.20.0`). Pre-release
  versions (`v1.20.0-pre.1`) also get full binaries — the preprocessor
  uses the literal version string as the tag.
- **Checksum manifest**: `skills/visualisation/visualise/bin/checksums.json`
  is committed to the plugin tree. Shape:
  ```json
  {
    "version": "1.20.0",
    "binaries": {
      "darwin-arm64": "sha256:<hex>",
      "darwin-x64":   "sha256:<hex>",
      "linux-arm64":  "sha256:<hex>",
      "linux-x64":    "sha256:<hex>"
    }
  }
  ```
  Maintainers update this file in the same commit that bumps the version.
  The preprocessor verifies every downloaded binary against this committed
  manifest, so a tampered release cannot pass the check.
- **First-run flow** in `launch-server.sh`:
  1. `OS=$(uname -s | tr '[:upper:]' '[:lower:]')`; `ARCH=$(uname -m)`;
     normalise (`x86_64` → `x64`, `aarch64` / `arm64` → `arm64`).
  2. Read plugin version from
     `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` via `jq`
     (prerequisite; fail fast with "jq is required but not found" if
     absent).
  3. Expected binary at
     `${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/bin/accelerator-visualiser-${OS}-${ARCH}`.
  4. If binary exists and SHA-256 matches the manifest entry (using the
     portable wrapper: `sha256sum` on Linux, `shasum -a 256` on macOS),
     skip to step 7.
  5. Remove any existing `.part` file for this platform (cleans up
     orphans from prior interrupted downloads). Then download:
     `curl -fsSL -o <tmp>.part https://github.com/<owner>/<repo>/releases/download/v<version>/accelerator-visualiser-${OS}-${ARCH}`.
  6. Verify SHA-256 against manifest. On match, `chmod +x` and rename to
     the cache path. On mismatch, delete the partial and exit with a
     clear error.
  7. Exec the binary with the config JSON path.
- **Failure modes**:
  - Unsupported platform (e.g. Windows, or an arch not in the manifest) →
    preprocessor exits with a friendly "platform not supported" message.
  - Network failure on download → clear error pointing at the release
    URL and suggesting `ACCELERATOR_VISUALISER_BIN` for offline use.
  - Checksum mismatch → treated as a security failure: delete the
    partial, print expected/actual hashes and the release URL, mention
    `ACCELERATOR_VISUALISER_BIN` as a recovery path, don't retry, exit.
- **Dev override**: `ACCELERATOR_VISUALISER_BIN=/path/to/dev/binary`
  short-circuits steps 3-6 entirely. Supports `cargo run` / `cargo build`
  workflows without touching the release pipeline. The preprocessor
  still writes `config.json` and invokes the binary normally.
- **Cache invalidation**: a plugin version bump implicitly invalidates
  the cache — the new manifest entries don't match any previously
  downloaded binary, so the next invocation re-downloads. No explicit
  cleanup needed.
- **Plugin repo gitignore** (new): the plugin's `.gitignore` adds
  `skills/visualisation/visualise/bin/accelerator-visualiser-*` so downloaded
  binaries in a dev checkout don't get committed. `checksums.json`
  stays tracked.

### D4. Dev jj workspace — `visualisation-system` (Q4)

All implementation work happens **exclusively inside the
`visualisation-system` jj workspace** at
`…/accelerator/workspaces/visualisation-system/`. This workspace is an
**isolated, self-contained working-copy view** of the accelerator
repository: it is a complete checkout containing everything needed to
build the visualiser — the plugin manifest
(`./.claude-plugin/plugin.json`), all existing skills, agents, hooks,
top-level scripts, and templates. No other checkout of the repository
is involved, visible, or relevant to this work.

**Strict isolation rules** (non-negotiable — Claude and any sub-agent
it spawns must follow these throughout the visualiser build):

- **Treat the workspace root as the only repository root.** The
  workspace directory
  (`…/accelerator/workspaces/visualisation-system/`) is the sole
  working directory. All reads, writes, searches, greps, globs, file
  edits, and VCS operations stay scoped inside it.
- **Never read from paths outside the workspace.** Do not traverse up
  out of the workspace root to inspect, open, grep, or otherwise
  consult any other checkout of the accelerator repo (e.g. the
  parent/trunk working copy at `…/accelerator/`). Even though other
  checkouts exist elsewhere on disk, they are **out of scope**. If a
  file appears to be "missing," it is missing from *this* workspace
  — do not fetch it from anywhere else; investigate why it's absent
  here.
- **Never write to or modify paths outside the workspace.** All edits,
  new files, deletions, and commits target this workspace only. `jj`
  commands run from this workspace automatically scope their effects
  correctly.
- **Relative paths in this document always resolve against the
  workspace root.** Any absolute path of the form
  `/Users/…/accelerator/<rest>` that appears below in "Code
  references" or elsewhere is a historical artefact of how the
  research was initially gathered — it is **not** an instruction to
  read that path. The equivalent file exists at the same
  `<rest>` relative path inside this workspace, and that is what
  implementation must reference.

There is a single plugin and a single `.claude-plugin/plugin.json`; the
jj workspace is a VCS-level working-copy view, not a separate plugin or
plugin manifest. The skill is registered in the workspace's
`./.claude-plugin/plugin.json` from day one. There is no later "promote
to root plugin" step, because there is no separate root to promote to
— the manifest inside this workspace *is* the manifest. When the
visualisation-system bookmark is later integrated into trunk, the
commits carrying the manifest change propagate through normal VCS
operations; no manual file copy between checkouts is required or
permitted.

### D5. Reviews modelling — three separate DocTypes (Q5)

UI, sidebar, and `DocTypeKey` union expose `plan-reviews`, `pr-reviews`, and
`ticket-reviews` as distinct types. Each reads from its configured path
(`review_plans` → `meta/reviews/plans/`, `review_prs` → `meta/reviews/prs/`,
`review_tickets` → `meta/reviews/tickets/`). Slug derivation still strips
`YYYY-MM-DD-` and `-review-N` suffix for all three. Lifecycle clustering
groups across them by slug match.

**Updated DocTypeKey union (v1)**:

```ts
type DocTypeKey =
  | "decisions" | "tickets" | "plans" | "research"
  | "plan-reviews" | "pr-reviews" | "ticket-reviews"
  | "validations" | "notes" | "prs" | "templates";
```

That's 11 keys (9 from the spec with `reviews` split into three, plus
`templates` for library visibility — templates gates out of lifecycle and
kanban as before). `tmp` is **not** a DocType — it's runtime state only.

### D6. Wiki-link forms — `[[TICKET-NNNN]]` with `TICKET-` prefix (Q6)

The visualiser resolves `[[ADR-NNNN]]` against the ADR index (by `adr_id` or
filename prefix) and `[[TICKET-NNNN]]` against the ticket index (by filename
numeric prefix — the `TICKET-` form in the link is stripped when resolving).
Bare `[[NNNN]]` is not supported. Rationale: prefix is more extensible when
future ID namespaces are introduced (e.g. `[[EPIC-NNNN]]`).

### D7. Declared-link rendering — ship in v1 (Q7)

The `target:` field on plan-reviews renders as a first-class declared link in
the "Related artifacts" aside of the review's library page AND on the library
page of the target plan (bidirectional — the plan's aside lists reviews whose
`target:` points at it). The declared/inferred visual distinction is implemented
in Phase 9 regardless. The plan-review `target:` is the only real cross-ref in
the wild today; the rest activate as authors populate them.

### D9. Templates rendered across all three resolution tiers

The Templates section of the library surfaces **every tier** of the
template resolution stack so users can see what each template will
render to regardless of their current configuration. The resolution
model is the one established in ADR-0017 and implemented in
`config_resolve_template()` (`scripts/config-common.sh:152-192`):

| Tier | Source | Path | Presence |
|---|---|---|---|
| 1 (highest) | Config override | value of `templates.<name>` in `.claude/accelerator.md` or `.local.md`, resolved against project root | optional |
| 2 (middle)  | User override   | `<paths.templates>/<name>.md` (default `meta/templates/<name>.md`) | optional |
| 3 (lowest)  | Plugin default  | `<plugin-root>/templates/<name>.md` | always present (authoritative name set) |

**Name enumeration**: the authoritative list of template names comes
from globbing `<plugin-root>/templates/*.md` per
`config_enumerate_templates()` (`config-common.sh:102-112`). Current
set: `adr`, `plan`, `research`, `validation`, `pr-description`.
Files added to tier 2 without a matching tier-3 peer are orphans that
the resolver ignores; the visualiser surfaces them under a separate
"unregistered" badge for discoverability but doesn't promote them to
the canonical template index.

**UI**: the Templates library index lists the five names. Each
template's detail page shows three panels (one per tier) in priority
order, each labelled with its source, its absolute path (link to
"Open in editor"), a presence indicator, and — for the winning tier —
a distinct *active* badge. Absent tiers render as greyed-out cards
with a one-line "not currently configured" note, not hidden, so the
user learns what would change if they ejected or configured.

**Server-side contract**: unlike the other ten DocTypes (flat walk of
one directory), `templates` is a *virtual* DocType backed by
three-tier resolution. The server exposes:
- `GET /api/docs?type=templates` — list of the five names with
  per-tier presence booleans and the active-tier label.
- `GET /api/docs/templates/:name` — an object containing up to three
  entries (`{ source, path, present, content?, etag? }`), always
  including the plugin-default entry.

The preprocessor resolves tier-1 and tier-2 paths per template name
and passes them to the server in `config.json`; the Rust process does
not shell out. This means `config.json` gains a `templates` map of
the shape:

```json
{
  "templates": {
    "adr":            { "config_override": null,
                        "user_override":   "meta/templates/adr.md",
                        "plugin_default":  "<plugin-root>/templates/adr.md" },
    "plan":           { "config_override": null, "user_override": "…", "plugin_default": "…" },
    "research":       { "...": "..." },
    "validation":     { "...": "..." },
    "pr-description": { "...": "..." }
  }
}
```

Paths are absolute. `config_override` is `null` when no
`templates.<name>` key is set; the other two are always strings (the
files they point to may still be absent, which the server reports via
`present: false`).

**Watch behaviour**: tier-3 plugin defaults don't need watching —
they only change on a plugin upgrade, which restarts the server.
Tier-1 and tier-2 paths are watched when present so live edits of
user-ejected or config-overridden templates reflect immediately in
the UI.

**Scope call preserved from the spec**: templates remain
de-emphasised under the sidebar's "Meta" heading and are excluded
from lifecycle and kanban.

### D10. Frontend embedded into the Rust binary (no committed `dist/`)

Once we ship the server via GitHub Releases (D8), committing
`frontend/dist/` to the repo stops being necessary — one release asset
per arch can carry both the server and the frontend. **The frontend
bundle is embedded into the Rust binary at compile time via
`rust-embed`**, so the per-arch binaries remain the only release
artefacts and drift between `src/` and `dist/` becomes impossible.

**Crate**: `rust-embed` (macro-driven; compiles `frontend/dist/**` into
the binary as `&'static [u8]` blobs with mime-type inference and a
thin `Iterator` over entries). The `assets` module exposes one of two
implementations gated by a Cargo feature:

- `embed-dist` (**default**, used for release builds): `rust-embed`
  reads `../frontend/dist` at compile time and serves via a tiny
  `axum` handler that maps URL paths to embedded files; 404 falls back
  to `index.html` for SPA routing.
- `dev-frontend` (off by default, `--features dev-frontend` opt-in):
  serves `../frontend/dist` via `tower-http::services::ServeDir` at
  runtime. Allows `vite build --watch` to write fresh files that the
  running Rust process picks up without a `cargo` rebuild.

**`build.rs`** asserts `../frontend/dist/index.html` exists when
`embed-dist` is enabled, so a release build fails loudly rather than
producing a binary with an empty frontend. A one-line check is enough:

```rust
if cfg!(feature = "embed-dist")
    && !std::path::Path::new("../frontend/dist/index.html").exists()
{
    panic!("frontend/dist/ missing — run `npm run build` before `cargo build --release`");
}
```

**Release build order** (baked into
`scripts/release-visualiser-binaries.sh`):

1. `cd frontend && npm ci && npm run build` — produces a fresh
   `frontend/dist/`.
2. `cd ../server && cargo zigbuild --release --target <quadruple>`
   (repeat for all four targets) — `rust-embed` picks up `dist/`.
3. `strip` each binary; compute SHA-256; update `checksums.json`;
   commit the manifest with the version bump.
4. Tag the release; `gh release upload` the four binaries.

**Binary size impact**: the frontend bundle (React + TanStack Query +
Router + dnd-kit + app code, gzipped and minified) is typically
~200-600 KB. `rust-embed` stores files uncompressed by default;
compress once at build time with `rust-embed`'s `"compression"`
feature (brotli) and serve with `Content-Encoding: br` for
same-wire-size outcomes as a static server. Net effect: each per-arch
binary grows by ~0.5-1 MB over a server-only binary (still well
within the 30-40 MB total across all four targets).

**Gitignore consequences**:
- `skills/visualisation/visualise/frontend/dist/` — gitignored (built
  from source each release, embedded into the binary).
- `skills/visualisation/visualise/frontend/node_modules/` —
  gitignored (standard).
- `skills/visualisation/visualise/server/target/` — gitignored
  (standard Cargo).

**What this decision eliminates**:
- The "committed `dist/` freshness guard" gap the follow-up analysis
  flagged (no committed artefact → no drift).
- Noisy PR diffs on every frontend change (`dist/` no longer tracked).
- A separate release asset or a two-step fetch in
  `launch-server.sh` (one binary still carries everything).

**Dev workflow**:
- Frontend-only iteration: run `vite dev` for the Vite dev server
  with HMR (proxy `/api` to the running Rust server), OR
  `vite build --watch` + `cargo run --features dev-frontend` to
  serve the built bundle through the real Rust process.
- Full release rehearsal: `npm run build` then `cargo run --release`
  (default features) exercises the embedded path locally.

---

## Proposed implementation phasing

Each phase is sized to be a single implement-plan session. Phases are ordered so
every phase ships something demonstrable and each builds on the previous.

### Phase 1 — Skill scaffolding and no-op preprocessor

**Goal**: `/accelerator:visualise` exists, prints a URL placeholder, does
nothing else.

- Create `skills/visualisation/visualise/SKILL.md` with frontmatter (
  `disable-model-invocation: true`,
  `allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/*)`).
- Preamble resolves all 12 path keys (including `tmp`) via `!`-prefixed
  `config-read-path.sh` calls, following the `review-pr` template.
- Add `skills/visualisation/visualise/scripts/launch-server.sh` as a stub that
  prints a hardcoded `http://localhost:0000` JSON line and exits 0.
- Register the skill in the repo-root `.claude-plugin/plugin.json`.
- Add `accelerator-visualiser` CLI wrapper (shell script) per Q1 decision.

**Deliverable**: running `/accelerator:visualise` prints a URL; running the CLI
prints the same URL.

### Phase 2 — Server bootstrap and lifecycle

**Goal**: a Rust server binds a random port, writes `server-info.json`,
survives backgrounding; `launch-server.sh` fetches the correct binary on
first run.

- Create `skills/visualisation/visualise/server/` as a Cargo project:
  `Cargo.toml` pinning `axum`, `tokio` (multi-thread), `notify`,
  `gray_matter`, `serde_yml`, `sha2`, `serde`, `serde_json`, `tempfile`,
  `nix`, `clap`, `tracing`, `tracing-subscriber`, `tower-http`. `src/main.rs`
  as entry point; `src/config.rs` parses the JSON config path from argv.
- Port `start-server.sh` + `stop-server.sh` pattern from superpowers: arg
  parsing, harness-PID resolution, nohup+disown backgrounding,
  log-poll-for-ready, liveness-check-after-start. Skip superpowers'
  Windows/MSYS and Codex auto-foreground branches — accelerator targets
  macOS/unix + Claude Code only.
- **First-run binary fetch (per D8)**: `launch-server.sh` detects platform
  (`uname -s` / `uname -m` → `darwin-arm64` / `darwin-x64` / `linux-arm64`
  / `linux-x64`), reads plugin version from `plugin.json`, checks the
  cache path at `${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/bin/`
  for an existing binary, verifies its SHA-256 against the committed
  `checksums.json`, downloads from the GitHub Release on miss (`curl
  --progress-bar`), re-verifies, and `chmod +x`. Supports
  `ACCELERATOR_VISUALISER_BIN` env override for local dev builds.
  - **Portable SHA-256**: use `sha256sum` on Linux (check availability
    first), fall back to `shasum -a 256` on macOS. Both produce the
    same hex digest; the wrapper normalises the output format.
  - **Progress and error UX**: emit a single-line "Downloading
    visualiser server (first run, ~8 MB)…" to stderr before the
    download. Use `curl --progress-bar` so progress is visible. On
    network failure, suggest retrying and mention
    `ACCELERATOR_VISUALISER_BIN` for offline use. On checksum
    mismatch, print both expected and actual hashes, the release URL
    fetched, and a note to verify the plugin version is current.
- Implement server lifecycle in Rust: env-var config read, `axum::serve`
  listener, `server-info.json` + `server-stopped.json` lifecycle files,
  owner-PID watch loop via `tokio::time::interval` + `nix::sys::signal::kill(pid, None)`
  every 60s, 30-min idle timeout (defined as "no new HTTP requests
  AND no active SSE subscribers" — an open SSE connection keeps the
  server alive; once all subscribers disconnect the countdown begins),
  SIGTERM/SIGINT handlers via `tokio::signal`.
- **CORS**: configure `tower-http::cors::CorsLayer` to reject all
  cross-origin requests (no `Access-Control-Allow-Origin` header
  emitted). This prevents any website open in the user's browser from
  making API requests to the visualiser even if it discovers the
  dynamic port via localhost port scanning.
- `/accelerator:visualise` now prints the real URL from `server-info.json`;
  re-invocation detects live PID **and verifies liveness via a
  `GET /api/types` health-check with a 2s timeout** before declaring
  the server alive. This prevents stale server-info.json (from an
  unclean shutdown where the PID was recycled) from causing a false
  reuse.
- **Owner-PID passed explicitly**: the preprocessor passes the harness
  PID to the server as an explicit field in `config.json` (or env var)
  rather than deriving it from process ancestry (ppid-of-ppid). This
  mirrors the superpowers `BRAINSTORM_OWNER_PID` pattern and avoids
  fragility when the process tree depth between Claude Code and the
  launched script varies.
- **No HTML served yet** — `GET /` returns 200 with placeholder text.

**Deliverable**: slash command or CLI starts the server (first invocation
downloads the binary from GitHub Releases), URL is live in a browser,
ctrl-c or idle kills it. Second invocation reuses the live PID without
re-downloading.

### Phase 3 — FileDriver, Indexer, and read-only API

**Goal**: `GET /api/types`, `GET /api/docs?type=…`, `GET /api/docs/{*path}`
all return real data.

- Implement `file_driver` module with a `FileDriver` trait (methods
  `list`, `read`, `watch`) and a `LocalFileDriver` impl. No
  `write_frontmatter` yet. The **ten ordinary** source directories
  (per D5: `decisions`, `tickets`, `plans`, `research`, `plan-reviews`
  at `review_plans`, `pr-reviews` at `review_prs`, `ticket-reviews`
  at `review_tickets`, `validations`, `notes`, `prs`) are flat walks —
  the three-type review split absorbs the nesting.
- `templates` is a **virtual DocType** backed by the three-tier
  resolver (D9), not a flat walk. Implement it as a separate code
  path that consumes the `config.templates` map and produces one
  IndexEntry-like record per (name, tier) pair — up to three per
  template name, always including the plugin-default tier. Missing
  tier files are indexed with `present: false` (not treated as
  errors).
- Implement `indexer` module: scan the ten ordinary dirs, parse with
  `gray_matter` + `serde_yml`, tolerate absent frontmatter (new
  `FrontmatterState::Absent` variant — not an error), malformed
  frontmatter (`FrontmatterState::Malformed` — indexed raw-only + emit
  `doc-invalid` in Phase 4), compute SHA-256 ETag via `sha2`, derive slug
  per-type including the `-review-N` suffix strip for `plan-reviews`,
  `pr-reviews`, and `ticket-reviews`. Templates are indexed separately —
  no slug, no clustering.
- Wire-format `DocTypeKey` union per D5 (extends spec with `ticket-reviews`):
  `"decisions" | "tickets" | "plans" | "research" | "plan-reviews" | "pr-reviews" | "ticket-reviews" | "validations" | "notes" | "prs" | "templates"`.
  Rust side uses a `DocTypeKey` enum with `#[serde(rename_all = "kebab-case")]`.
- Implement axum routes: `/api/types`, `/api/docs?type=…`,
  `/api/docs/{*path}` with strong ETag and `If-None-Match` → 304.
  Add the templates-specific endpoints per D9:
  - `GET /api/docs?type=templates` → list of the five template names
    with per-tier presence and the active-tier label.
  - `GET /api/docs/templates/:name` → up to three
    `{ source, path, present, content?, etag? }` entries in priority
    order, plugin default always included.
- Path safety: `std::fs::canonicalize` + prefix check against the
  configured roots on every path lookup. Reject symlinks that escape.
  The templates code path canonicalises against each tier's resolved
  directory (plugin-root `templates/`, userspace `templates/`, the
  config override's resolved dir) to keep the same guard.
- `/api/lifecycle` returns pre-computed slug clusters (no per-request
  compute beyond cache lookup).

**Deliverable**: `curl localhost:<port>/api/docs?type=decisions` returns
the 14 ADRs with frontmatter; `?type=plan-reviews` returns the 6
plan-reviews; `?type=templates` returns the five template names with
three-tier presence info; a doc fetch returns markdown + headers;
a template fetch returns up to three tier entries with their content.

### Phase 4 — SSE hub and notify watcher

**Goal**: `GET /api/events` streams `doc-changed` events.

- Implement `sse_hub` module built on `tokio::sync::broadcast`:
  subscriber management via an axum SSE stream, broadcast semantics.
  On `RecvError::Lagged` (slow consumer fell behind the channel
  capacity), inject a synthetic `invalidate-all` event into that
  subscriber's SSE stream so the frontend triggers a full refetch
  rather than silently operating on stale data.
- Wire `notify` watchers to each of the 10 source dirs (non-recursive —
  flat per type after the D5 split; templates is virtual, not watched).
- 100ms per-path debounce implemented with a
  `HashMap<PathBuf, Instant>` recording the scheduled time per path.
  Each spawned task checks on wake whether it is still the latest
  scheduled instance (compare its own timestamp against the map entry);
  only proceed with re-indexing if it is the latest. This avoids
  aborting tasks mid-execution which could leave the index in an
  inconsistent state. After successful re-indexing, remove the path's
  entry from the HashMap to prevent unbounded growth during long server
  sessions. If pending entries exceed a threshold (e.g., 100 paths),
  switch to a full-rescan strategy to handle mass file operations (git
  checkout, format-all) gracefully.
- Route change → index update → ETag recompute → SSE broadcast
  `{type, path, etag}`.
- Handle `FrontmatterState::Malformed` by emitting `doc-invalid`
  alongside indexing-with-raw-only.

**Deliverable**: edit a file on disk, `curl` a listening SSE connection,
see the event.

### Phase 5 — Frontend scaffold and library view

**Goal**: SPA loads, sidebar lists the 11 doc types, clicking a doc renders its
markdown. Critical error UX ships alongside the scaffold.

- Scaffold frontend at `skills/visualisation/visualise/frontend/` with Vite +
  TS + React.
- TanStack Router route tree: `/`, `/library`, `/library/:type`,
  `/library/:type/:fileSlug`. Templates share the same route shape —
  `/library/templates/:name` — but render the three-tier panel layout
  described in D9 instead of a single doc body.
- TanStack Query client with a `ReconnectingEventSource` subscribing to
  `/api/events` with **exponential backoff on disconnect** and
  **invalidate-all on reconnect**, so the UI never silently goes stale.
- Shell layout with sidebar (doc type groupings + views). Templates
  sit under a de-emphasised "Meta" heading per spec and excluded from
  lifecycle and kanban.
- Library views: type index table, doc detail page with markdown rendering,
  frontmatter chips, "Related artifacts" aside (empty for now).
- Templates view (per D9): index page listing the five names with a
  per-tier presence grid and an "active" badge on the winning tier;
  detail page showing three panels (plugin default · user override ·
  config override) in priority order, each with source label, path,
  "Open in editor" link, presence indicator, and — for the winner —
  an *active* badge. Absent tiers render as greyed cards with "not
  currently configured".
- Run `npm run build` in `frontend/` to produce `frontend/dist/` locally;
  the Rust binary embeds this directory via `rust-embed` at compile
  time (D10) and serves it as the `GET /*` fallback. `dist/` is
  gitignored — not committed. During dev, `cargo run --features
  dev-frontend` swaps to `ServeDir` from disk so Vite-rebuilt assets
  appear without a Rust rebuild.
- Markdown rendering: CommonMark + GFM + syntax highlighting. No wiki-link
  resolution yet (Phase 9).
- **Critical error UX (pulled forward from Phase 10)**:
  - Init-not-run detection: if the server reports no configured
    directories exist, show a friendly full-page message with
    `/accelerator:init` hint.
  - Server-shutdown SSE event: when the server emits a `shutdown`
    event (fired before process exit), the frontend shows a clear
    message: "The visualiser server has shut down. Relaunch with
    /accelerator:visualise."
  - Basic keyboard focus management: visible focus rings on
    interactive elements, tab-navigable sidebar.

**Deliverable**: open the URL, navigate the sidebar, read all documents
including the five templates with their three-tier breakdown. SSE
reconnects gracefully; init-not-run and server-shutdown show clear
messages.

### Phase 6 — Lifecycle clusters and view

**Goal**: `/lifecycle` and `/lifecycle/:slug` render.

- Extend `Indexer` with `buildClusters()`: group index entries by slug, compute
  `completeness`, sort by canonical order (ticket → research → plan →
  plan-review → ticket-review → validation → PR → pr-review → decision →
  notes), then by mtime within a type.
- `/api/lifecycle` and `/api/lifecycle/:slug` endpoints.
- `/lifecycle` index — card per cluster with pipeline dots.
- `/lifecycle/:slug` — vertical timeline with placeholders for missing stages.

**Deliverable**: the lifecycle view shows real clusters derived from slug
matching; gaps ("no plan yet") are visible.

### Phase 7 — Kanban read-only

**Goal**: `/kanban` renders three columns and an "Other" swimlane; no drag-drop
yet.

- `/kanban` route; dnd-kit sortable lists (read-only in this phase — `onDrag*`
  handlers no-op).
- Cards show ticket number, title, type, last-modified.
- SSE invalidations update card positions on external status changes.

**Deliverable**: all tickets visible in columns matching their current `status:`
field; unknown values go to "Other".

### Phase 8 — Kanban write path

**Goal**: drag-drop updates `status:` on disk.

- Implement `PATCH /api/docs/tickets/{*path}/frontmatter` with `If-Match`
  strong ETag:
  - Input validation: field allowlist = `{"status"}`, value allowlist =
    `{"todo", "in-progress", "done"}`.
  - Path-escape guard via `std::fs::canonicalize` + prefix check against
    the tickets dir.
  - **Fresh ETag verification at read time**: read the file, compute
    SHA-256 of the bytes just read, and compare that hash against the
    `If-Match` header directly — do not rely on the cached ETag from
    the indexer. This eliminates the TOCTOU window between the indexer's
    debounced cache update and the actual file state.
  - YAML-aware line patcher (new `patcher` module): replaces only the
    `status:` line in the frontmatter block, preserving
    comments/order/whitespace.
  - Atomic write via `tempfile::NamedTempFile::persist` (sibling tempfile
    + rename).
  - Recompute ETag, update the indexer cache, broadcast `doc-changed`
    through the SSE hub.
- Frontend: optimistic mutation, rollback on `412`, toast on conflict
  ("This ticket was modified externally — the board has been refreshed
  with the current state."), SSE reconciles silently on `204`.

**Deliverable**: drag a card → `status:` line in the `.md` file changes →
a second tab receives the update.

### Phase 9 — Cross-references and wiki-links

**Goal**: `[[ADR-NNNN]]` and `[[TICKET-NNNN]]` render as clickable links;
declared `target:` on plan-reviews renders bidirectionally.

- Extend the Indexer with ID lookups: `adrById[NNNN]` (keyed by `adr_id`
  frontmatter or filename prefix) and `ticketById[NNNN]` (keyed by filename
  numeric prefix).
- Markdown renderer extension: rewrite `[[ADR-NNNN]]` and `[[TICKET-NNNN]]` body
  content to library deep links when the index resolves them; leave as plain
  text otherwise. Per D6, bare `[[NNNN]]` is not supported — the `TICKET-`
  prefix is required and stripped at resolution time.
- Build a reverse declared-link index: on every plan-review's `target:` field,
  register the target path → this review. Plans' library pages then list inbound
  reviews in "Related artifacts" alongside slug-cluster matches.
- "Related artifacts" aside merges three sources: slug-cluster matches (tagged
  `inferred`), declared `target:` links from reviews (tagged `declared`), and
  the review's own declared `target:` back to the plan (tagged `declared`).
  Inferred and declared are visually distinct per D7.

**Deliverable**: cross-doc navigation via in-body `[[…]]` references works; a
plan's library page lists its reviews; a review's library page shows the
declared link back to its target plan.

### Phase 10 — Error handling, accessibility, polish

- Malformed-frontmatter banner on doc page; JSON-line logging to
  `<tmp>/visualiser/server.log` with 5MB rotation (retain at most 3
  rotated files, ~20MB maximum); server version header + sidebar footer;
  keyboard-navigable kanban; WCAG AA contrast.
- Note: init-not-run detection, SSE reconnect with backoff, basic focus
  rings, and server-shutdown notification were pulled forward to Phase 5
  so early adopters get a usable experience from the first frontend
  phase.

**Deliverable**: the visualiser handles every failure mode listed in the spec's
matrix, plus is keyboard-usable.

### Phase 11 — Testing

- `cargo test` unit tests colocated with each module: `file_driver`, slug
  derivation (table-driven per type, incl. reviews-suffix), `indexer`,
  `sse_hub`, `patcher`.
- `cargo test` integration tests in `server/tests/` using
  `tower::ServiceExt::oneshot` to drive the full axum router against a
  tmp-dir fixture.
- Playwright E2E: kanban golden path (drag → disk update → second-tab
  SSE), conflict path (`If-Match` mismatch → snap-back + toast), library
  → lifecycle → library deep-link round trip, Mermaid + wiki-link smoke.
  Binary-acquisition smoke: fresh plugin checkout → first invocation
  downloads from a staged release asset → launch succeeds.
- Commit `tests/fixtures/meta/` with 3-5 docs per type including
  deliberately malformed, absent-frontmatter, and review-suffix cases.

**Deliverable**: CI runs all three suites green.

### Phase 12 — Packaging, docs, and release

- **Frontend**: freeze deps, `npm run build` in `frontend/` to produce
  a fresh `frontend/dist/`. Not committed — consumed only by the Rust
  build that follows.
- **Server**: install Rust stable + `cargo-zigbuild` + zig on the
  maintainer's dev host. Build four release binaries via `cargo zigbuild
  --release --target <quadruple>` for `aarch64-apple-darwin`,
  `x86_64-apple-darwin`, `aarch64-unknown-linux-musl`,
  `x86_64-unknown-linux-musl`. The default (release) build has the
  `embed-dist` feature on, so `rust-embed` bakes the just-built
  `frontend/dist/` into each binary (D10). `strip` each binary.
  Compute SHA-256 for each; write results into
  `skills/visualisation/visualise/bin/checksums.json` alongside the
  bumped plugin version. Commit the updated manifest.
- **Release helper script**: add `scripts/release-visualiser-binaries.sh`
  that runs `npm run build` first, then the four cross-compiles, produces
  the checksums.json update, and uploads the assets to the GitHub Release
  via `gh release upload`. The script must fail loudly if `frontend/dist/`
  is missing or stale relative to `frontend/src/` at the moment the
  Rust builds start. **Release ordering for atomicity**: (1) build all
  four binaries; (2) compute SHA-256 checksums; (3) create a **draft**
  GitHub Release and upload all four binaries; (4) verify uploaded
  asset checksums match computed hashes (re-download and check); (5)
  commit `checksums.json` with the version bump; (6) push the tag; (7)
  promote the release from draft to published. If any step fails, the
  script aborts without pushing a tag or publishing, so users never see
  a release with missing or mismatched assets.
- **Plugin `.gitignore`**: add
  `skills/visualisation/visualise/bin/accelerator-visualiser-*` and
  `skills/visualisation/visualise/frontend/dist/` so downloaded
  binaries and local frontend builds don't get committed.
- Document the skill in accelerator's README (only user-facing ref).
- Verify the `/accelerator:init` sentinel path works end-to-end in a
  fresh project.
- Tag and bump plugin version per existing release cadence (CHANGELOG
  entry). Tag creation triggers the release and binary uploads.
- Smoke-test the release: from a clean plugin install (empty
  `skills/visualisation/visualise/bin/`), invoke `/accelerator:visualise`
  on all four supported platforms and confirm successful download →
  verify → launch → browser loads.

**Deliverable**: clean install of the plugin in a test project →
`/accelerator:init` → `/accelerator:visualise` opens a working visualiser.

### Phasing rationale

- **Phase 1 ships immediately**: something invokable even though empty.
  Validates the slash-command wiring before any Rust is written.
- **Phases 2-4 are server-only**: incremental increases in capability. Each
  phase produces a testable API surface.
- **Phases 5-9 layer the frontend**: library → lifecycle → kanban (read) →
  kanban (write) → cross-refs. The write surface (Phase 8) deliberately comes
  after read-only kanban (Phase 7) so we can visually validate correctness
  before introducing mutations.
- **Phase 10 is bounded polish**; the spec's failure matrix is the acceptance
  checklist.
- **Phase 11 tests are deferred** so that earlier phases can land quickly; a
  stress-test pass against the suite can fold late findings back in.
- **Phase 12 is release discipline**, not implementation.

An acceptable MVP cut (if time pressure demands) is Phases 1-5 + 7 (skip write
path, lifecycle, cross-refs) — which still gives a browsable library with live
updates.

---

## Code references

> **All paths below are workspace-relative** — they resolve against the
> `visualisation-system` jj workspace root
> (`…/accelerator/workspaces/visualisation-system/`), which is the only
> repository root Claude should consult. See **D4** for the isolation
> rules. The only exceptions are the `~/.claude/plugins/cache/…`
> entries at the bottom, which point into a third-party plugin cache
> outside this repo (read-only reference material — not part of the
> accelerator codebase and not subject to the workspace-isolation
> rule).

- `scripts/config-read-path.sh:1-24` — path-key wrapper.
- `scripts/config-read-value.sh:1-130` — underlying resolver; layered file precedence.
- `scripts/config-common.sh:15-192` — `find_repo_root`, frontmatter extraction, template resolution.
- `scripts/config-summary.sh:19-21` — init sentinel (check for `<tmp>/.gitignore`).
- `skills/config/init/SKILL.md:1-126` — init skill; 12 path keys, nested `tmp/.gitignore`.
- `skills/github/review-pr/SKILL.md:1-36` — canonical `!`-preamble + `{placeholder}` skill pattern.
- `skills/github/review-pr/SKILL.md:96-117, 627-634` — `{tmp directory}/pr-review-{id}/` subdirectory + cleanup discipline.
- `skills/decisions/create-adr/SKILL.md:1-25` — extended `allowed-tools` with skill-local scripts.
- `skills/decisions/scripts/adr-next-number.sh:1-50` — skill-local script bootstrap pattern.
- `.claude-plugin/plugin.json` — plugin manifest (skill registration point).
- `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/brainstorming/scripts/start-server.sh:1-149` — reference bash launcher.
- `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/brainstorming/scripts/server.cjs:1-352` — reference server implementation.
- `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/brainstorming/scripts/stop-server.sh:1-57` — reference graceful shutdown.

## Architecture insights

- **Config precedence is a single concept** used by 15 scripts: team file →
  local file → default. The visualiser inherits this for free — one
  `config.json` written by the preprocessor captures the fully-resolved view. No
  runtime config reads needed inside the Rust server.
- **Slash command == SKILL.md with `disable-model-invocation`**. There's no
  separate command registry; adding a skill to `plugin.json` is the entire
  registration step.
- **`meta/tmp/` is the universal ephemeral store** and is pre-gitignored via the
  nested-ignore trick — the visualiser's PID/log/config/server-info all belong
  there, matching every other skill's discipline (`review-pr`, `describe-pr`).
- **Canonicalize + prefix check** is the path-escape guard idiom the
  config scripts use (via `realpath` in bash); the Rust server mirrors it
  with `std::fs::canonicalize` inside `LocalFileDriver` to eliminate
  directory traversal.
- **Strong ETags over SHA-256 content hash** work cleanly with SSE
  reconciliation — when the event's ETag matches the query cache's ETag, the
  invalidation can be skipped.
- **Development happens exclusively inside the `visualisation-system` jj
  workspace** — an isolated, self-contained working-copy view of the
  accelerator repo. Claude treats the workspace root as the only
  repository root: all reads, writes, searches, and VCS operations stay
  scoped inside it, and no other checkout of the repo on disk is
  consulted or modified. The single plugin manifest lives inside this
  workspace at `./.claude-plugin/plugin.json`; the skill is registered
  there from day one, and there is no later "promote to root plugin"
  step. See **D4** for the full isolation rules.
- **Owner-PID monitoring** is a robustness pattern the accelerator hasn't used
  before but should adopt — it makes the server self-cleaning when the Claude
  Code session dies.

## Historical context

- `meta/research/2026-03-18-meta-management-strategy.md` — the conceptual model
  for doc types that this visualiser is making visible.
- `meta/plans/2026-03-23-template-and-path-customisation.md` — defined the
  path keys the visualiser reads (originally 11; now 12 with `review_tickets`
  added 2026-04-24). Authoritative on naming.
- `meta/plans/2026-03-22-validation-crossref-frontmatter.md` — proposed the
  cross-ref frontmatter fields the spec assumes are populated. Those proposals
  have not yet been implemented by authoring skills, which is why the
  declared-links branch will render empty in v1.
-
`meta/plans/2026-03-28-initialise-skill-and-review-pr-ephemeral-migration.md` —
established the `meta/tmp/` nested-gitignore convention. The visualiser's
persistent state lands inside this system.
- `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md` — the
  temp-dir sharing convention; informs which namespace under `<tmp>/` to
  reserve (recommendation: `<tmp>/visualiser/`, matching the spec).
- `meta/notes/2026-03-24-yaml-block-sequence-array-parsing.md` — YAML quirks the
  frontmatter parser must handle.

## Related research

- `meta/research/2026-03-22-skill-customisation-and-override-patterns.md` — the
  `.claude/accelerator.md` + `.claude/accelerator.local.md` override chain. The
  visualiser inherits this transparently.
- `meta/research/2026-03-27-skill-customisation-implementation-status.md` —
  status of customisation rollout; relevant for deciding whether `visualise`
  should take per-skill context/instructions overrides.

## Open questions

All design decisions flagged during research are resolved. See the
**Resolved design decisions** section above (D1–D8):

- D1 CLI distribution — shell wrapper committed in plugin tree.
- D2 In-plugin tree layout — `skills/visualisation/visualise/` with
  `server/` (Cargo), `frontend/` (Vite), `bin/`, `cli/`.
- D3 Server stack — **Rust** (axum + tokio + notify + gray_matter +
  serde_yml + sha2).
- D4 Dev jj workspace — `visualisation-system` treated as an
  **isolated** working copy: all reads/writes stay strictly inside this
  workspace, no access to any other checkout of the repo on disk.
  Single plugin manifest at the workspace's own
  `./.claude-plugin/plugin.json` (no "root" outside the workspace, no
  per-workspace-vs-root distinction).
- D5 Reviews modelling — three separate DocTypes.
- D6 Wiki-links — `[[TICKET-NNNN]]` prefixed form only.
- D7 Declared links — shipped in v1 (the `target:` field on plan-reviews
  is the only populated cross-ref today; it renders bidirectionally).
- D8 Binary distribution — **GitHub Releases** with committed SHA-256
  manifest; download on first run per plugin version.
- D9 Templates view — render all three resolution tiers (config
  override > user override > plugin default) per template, with
  provenance and an active-tier badge; templates is a **virtual
  DocType** backed by the resolver, not a flat walk.
- D10 Frontend embedded into the Rust binary — `rust-embed` bundles
  `frontend/dist/` at compile time (`embed-dist` feature, default);
  `frontend/dist/` is gitignored and not committed; a `dev-frontend`
  feature swaps in `ServeDir` from disk for fast local iteration. One
  release asset per arch still carries everything.

No remaining blockers for implementation planning.

---

## Follow-up research 2026-04-18 — consistency and gap analysis

**Date**: 2026-04-18 09:52 BST
**Researcher**: Toby Clemson
**Git Commit**: `17f65bff` (working copy atop `17f65bf main — 1.19.0-pre.2`)
**Trigger**: After several rounds of heavy edits — re-assess whether the document still hangs together against the spec at `meta/specs/2026-04-17-meta-visualisation-design.md` and flag any major gaps.

### Verdict

The research still hangs together as an implementation-planning artefact.
All eight resolved decisions (D1–D8) are self-consistent and match the
spec. The spec-vs-reality matrix (G1–G12) maps cleanly onto the 12-phase
plan. The issues found below are **editorial inconsistencies** and
**implementation-detail gaps**, not structural breakage.

Confidence that a plan skill can proceed from this research: **high**,
provided the editorial fixes are applied. The seven gap items listed
below are **decided in their respective phase plans** (Phase 1's plan
resolves only Gap 1; Gaps 2, 5, 6 land in Phase 2; Gaps 3 and 7 land
in Phase 12; Gap 4 is already resolved by D10). See the "Major gaps"
section below for the per-phase mapping.

### Internal inconsistencies — editorial, fix in place

1. **"Seven design decisions" is stale**. ~~The Summary section says
   "Seven design decisions have now been resolved" but there are D1–D10.~~
   **Fixed**: now reads "Ten design decisions".
2. **"Cargo workspace for the server" is mis-termed**. §1 says "a
   Cargo workspace for the server", but D2's layout shows a single
   Cargo crate (one `Cargo.toml`, one `src/` tree). A *workspace* is a
   `[workspace]` with multiple member crates, which isn't what's
   proposed. Should read "a Cargo project" or "a single Cargo crate".
3. **Phase 1 preamble double-counts `tmp`**. ~~Phase 1 says "Preamble
   resolves all 11 path keys + tmp".~~ **Fixed**: now reads "all 12 path
   keys (including `tmp`)".
4. **Phase 6 canonical order omits `pr-review`**. ~~The phase used a
   collapsed ordering without the D5 split.~~ **Fixed**: Phase 6 now
   reads `ticket → research → plan → plan-review → ticket-review →
   validation → PR → pr-review → decision → notes`.
5. **D5's "10 keys" arithmetic is muddled**. ~~The rationale reads
   "9 from the spec with `reviews` split into two, minus …"~~
   **Fixed**: now reads "11 keys (9 from the spec with `reviews` split
   into three, plus `templates` for library visibility)".
6. **§6 `<tmp>/visualiser/` layout omits `config.json`**. The "What
   to take from this precedent" list enumerates
   `{server-info.json, server.pid, server.log, server-stopped.json}`,
   but the spec's Preprocessor responsibilities explicitly places
   `config.json` there too. Small but matters for anyone implementing
   Phase 2.
7. **§5 skill-local script depth doesn't match the visualiser's
   layout**. The example references
   `skills/decisions/scripts/adr-next-number.sh` using
   `PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"` (three levels
   up from a category-level `scripts/` dir). The visualiser's
   `launch-server.sh` lives at
   `skills/visualisation/visualise/scripts/` — **four** levels up
   (`../../../..`). The pattern works identically; the depth differs.
   Worth a sentence in §5.

### Live-codebase drift since 2026-04-17 snapshot

| Claim in doc                                        | Disk state 2026-04-18                                                                                                                     | Impact                                                                                                                                                                      |
|-----------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `decisions/` has 14 ADRs                            | **17** (ADR-0001 … ADR-0017, no gaps)                                                                                                     | Cosmetic. The new ADRs (0015 test-coverage lens, 0016 userspace config, 0017 config extension points) don't touch the 12 path keys or the 11 DocTypes.                      |
| Config system has 3 layers (team / local / default) | ADR-0016 adds a **userspace** config layer (`~/.claude/accelerator/…`) between local and default                                          | Minor. The visualiser consumes fully-resolved paths via `config-read-path.sh`, so the layer count is invisible to the Rust server — but §2's enumeration is now incomplete. |
| `meta/templates/` is empty in consumer repos        | ADR-0017 defines **userspace** templates at `meta/templates/<name>.md` (user-ejected overrides) + `templates.<name>` config map for paths | **Superseded by D9**: the visualiser now renders **all three resolution tiers** (config override > user override > plugin default) per template, not just user-ejected ones. `templates` becomes a virtual DocType backed by `config_resolve_template()` rather than a flat walk. |

Everything else verified by fresh codebase probes still holds:
- 12 path keys in `scripts/config-read-path.sh` (including `review_tickets`
  added 2026-04-24).
- `/accelerator:init` still creates all 12 dirs + `.gitkeep`, writes
  nested `meta/tmp/.gitignore`, does not write the config file.
- `tickets/` file counts (25) and status distribution — only `todo`
  and `done`, never `in-progress`.
- `plans/` (27), `research/` (17), `notes/` (3), `reviews/plans/` (6).
- `validations/` and `prs/` still absent on disk.
- Cross-reference frontmatter: `target:` on plan-reviews is still
  the only populated cross-ref.

### Major gaps — decided per-phase, not all up-front

These are implementation-detail questions the research doesn't answer.
The initial instinct was to decide all of them before Phase 1 lands,
but on reflection each gap belongs in the phase plan that actually
touches it — deciding Phase 12 mechanics during Phase 1 planning just
bakes in premature contracts. The per-phase mapping below replaces the
earlier "decide before Phase 1" framing; Phase 1's plan resolves Gap 1
only (manifest registration shape, concretely specified via the JSON
diff in that plan's Phase 1.4 section). All other gaps are explicitly
deferred to the phase listed.

1. **Plugin manifest registration** — *owner: Phase 1*. The skill
   registers in `.claude-plugin/plugin.json` by appending
   `"./skills/visualisation/"` to the `skills` array — the full JSON
   diff is pinned in Phase 1.4 of the Phase 1 plan
   (`meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md`).
2. **`config.json` schema** — *owner: Phase 2*. The preprocessor
   writes it and the Rust binary consumes it — schema lands when the
   real preprocessor and Rust server are introduced. Candidate
   fields from the spec plus D9: `doc_paths` (map of the 10 ordinary
   types), `templates` (map of 5 names → `{config_override,
   user_override, plugin_default}`), `plugin_root`, `tmp_path`,
   `owner_pid`, `host`, `log_path`, `plugin_version`. These are
   proposals, not a commitment — Phase 2 locks the exact shape.
3. **Release flow ordering** — *owner: Phase 12*. D8 and Phase 12
   describe the ingredients (cross-compile → checksums → commit
   manifest → tag → upload) but not the precise order. The correct
   sequence (build-four → compute-hashes →
   commit-manifest-with-version-bump → tag → push-tag →
   upload-to-release via `gh release upload`) should be baked into
   `scripts/release-visualiser-binaries.sh` at that time. Particular
   trap: the committed manifest must match the uploaded assets
   byte-for-byte, so the commit and the upload have to be atomic
   from the release-tag's perspective.
4. **`frontend/dist/` freshness guard** — **resolved by D10**. The
   frontend bundle is now embedded into the Rust binary at compile
   time via `rust-embed`, and `frontend/dist/` is gitignored. There
   is no committed artefact to drift: every release build runs
   `npm run build` immediately before `cargo zigbuild`, and the
   `build.rs` check fails loudly if `dist/index.html` is missing
   when `embed-dist` is enabled.
5. **Rust edition and MSRV** — *owner: Phase 2*. For a binary
   shipped to end users, pinning (e.g. edition 2021, MSRV 1.80)
   avoids surprise breakage on maintainer-host toolchain drift.
   Decided when the `Cargo.toml` is introduced in Phase 2.
6. **First-run download UX** — *owner: Phase 2*. On first invocation
   per plugin version the preprocessor fetches a ~6–10 MB binary via
   `curl`. Proposal: emit a single "Downloading visualiser server
   (first run, ~8 MB)…" line to stderr before the `curl`, so the
   slash command doesn't appear to hang. Finalised when the
   preprocessor gains its binary-fetch logic in Phase 2.
7. **Pre-release binary-build burden** — *owner: Phase 12*. D8 says
   "Pre-release versions (`v1.20.0-pre.1`) also get full binaries".
   In practice, pre-release versions are cut frequently (the git log
   shows recent `pre.1` and `pre.2` bumps inside a single day). Four
   cross-compiles per pre-release is non-trivial. Worth deciding at
   release-process-design time: do pre-releases really need
   binaries, or can they fall back to `ACCELERATOR_VISUALISER_BIN`
   for internal dogfooding?

### Non-gaps — explicitly confirmed

- Distribution mechanics (GitHub Releases, SHA-256 manifest,
  `cargo zigbuild` + musl) are internally coherent and match the spec.
- Phasing is independently shippable per phase and the MVP-cut
  (Phases 1–5 + 7) stays viable after the above issues are addressed.
- No ADR supersedes D1–D8; no newer research document contradicts the
  plan.

### Recommended next step

Apply the seven editorial fixes inline (they're all single-sentence
edits). Add one pointer line under §2 noting ADR-0016's userspace
layer. Treat each gap item as owned by the phase plan that actually
touches it (see the per-phase mapping above): Gap 1 is resolved by
Phase 1's plan, Gaps 2/5/6 land with Phase 2's plan, Gaps 3/7 land
with Phase 12's plan, and Gap 4 is already resolved by D10. Don't
front-load the later gaps into Phase 1.

## Related ADRs consulted in this follow-up
- `meta/decisions/ADR-0015-standalone-test-coverage-lens.md` — no impact on the visualiser (lens SKILL.md only).
- `meta/decisions/ADR-0016-userspace-configuration-model.md` — adds a userspace config layer; minor drift against §2.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — formalises userspace template overrides; aligns with G7's chosen direction.

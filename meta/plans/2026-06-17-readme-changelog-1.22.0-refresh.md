---
type: plan
id: "2026-06-17-readme-changelog-1.22.0-refresh"
title: "README and CHANGELOG 1.22.0 Refresh Implementation Plan"
date: "2026-06-17T13:00:50+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
derived_from: ["codebase-research:2026-06-17-readme-changelog-1.22.0-refresh"]
tags: [changelog, readme, release, docs]
revision: "996916b98aac33db7f09f6a5cffb7e1cbf97604e"
repository: "accelerator"
last_updated: "2026-06-17T14:50:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# README and CHANGELOG 1.22.0 Refresh Implementation Plan

## Overview

Refresh the two user-facing documents for the 1.22.0 release so they fully and
accurately describe what shipped since 1.21.0, written for **plugin users**, not
Accelerator developers:

1. **CHANGELOG** — complete the `[Unreleased]` section so it covers every
   user-facing change between 1.21.0 and `main`.
2. **README** — add a top-of-file Getting Started block and a light/dark plan
   hero screenshot, document the Linear integration alongside Jira under a new
   "Remote Work Item Management" umbrella section, and fix accuracy drift
   (integration availability, `create-note`, `visualiser.editor`, visualiser
   views).

Both documents stay terse and high signal-to-noise. The work is split into four
phases that are each independently mergeable.

## Current State Analysis

**CHANGELOG (`CHANGELOG.md:3-44`).** The `[Unreleased]` block currently
documents **only**: the upgrade note for migration 0007, configurable idle
auto-shutdown (Added), and three Changed items (unified-schema linkage reader,
idle default 30m→8h, migration 0001/0003/0004 merge-on-relocation). It is
consumed at release time by `keepachangelog` (`tasks/changelog.py:12`), so it
must remain Keep-a-Changelog-parseable.

The genuinely user-facing surface that is **missing**:

- **Linear Cloud integration** — 8 verb-decomposed skills, registered at
  `.claude-plugin/plugin.json:17`, present under `skills/integrations/linear/`
  (`init/search/show/create/update/comment/transition/attach` + `scripts/`).
- **`create-note`** skill + `note` template (`skills/notes/create-note/`,
  `templates/note.md`; category registered `.claude-plugin/plugin.json:26`).
- **Visualiser** additions: global search, detail-page Copy path / Open in
  editor, not-found / load-error recovery, browsable RCAs, Templates view,
  lifecycle clustering by typed-linkage, reader/markdown polish.
- **`rejected`** ADR status; **remote-tracker sync ergonomics**
  (`/create-work-item` push-on-accept, `/create-jira-issue` work-item-file mode,
  `/list-work-items` sync labels).
- A **Migrations** subsection for **0007** (mirroring the 1.21.0 entry).

**README (`README.md`).** Largely reflects 1.21.0 already. Gaps:

- No top-of-file Getting Started / install; `assets/` holds logos only (no hero
  screenshot). The "[Jump to installation]" line is at `README.md:10`; the full
  Installation section is at `README.md:713`.
- No Linear section; the Jira Integration section is standalone at
  `README.md:327`.
- `work.integration` blurb (`README.md:294-295`) lists all four allowed values
  without noting only `jira`/`linear` have skills (validated set:
  `scripts/config-defaults.sh:91-96`).
- `notes/` row says "Written by **manual**" (`README.md:100`); no `create-note`
  mention.
- Visualiser Customisation table (`README.md:494-501`) lacks `visualiser.editor`
  / `ACCELERATOR_VISUALISER_EDITOR` / `visualiser.editor_project` (confirmed at
  `skills/visualisation/visualise/SKILL.md:122-155`). Views table
  (`README.md:449-453`) lists 3 views and says "tickets"; a Templates view now
  also exists and clustering is typed-linkage based.

### Key Discoveries

- CHANGELOG parseability is verifiable: `keepachangelog.to_dict()` over
  `CHANGELOG.md` (`tasks/changelog.py:1-12`).
- The `configure templates` surface (`list`/`show`/`eject`/`diff`/`reset`) and
  `config_resolve_template` are **filename-driven** over `templates/*.md`, so all
  13 shipped templates — including `note`, `rca`, and the design/review
  templates — are customisable. `TEMPLATE_KEYS`
  (`scripts/config-defaults.sh:66-73`) is consumed only by the config dump
  (`config-dump.sh:185`), which therefore under-reports. **Decision: document the
  true behaviour (the README lists every ejectable template); fixing the config
  dump to derive its rows from the directory is tracked as work item 0113 and is
  out of scope here.**
- Reusable light/dark plan baselines exist
  (`skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/library-doc-view.spec.ts-snapshots/library-doc-view-{light,dark}-visual-regression.png`),
  but **Decision: capture fresh against a real plan** for the best hero image.
- The visualiser exposes an editor jump, theme toggle, and reads the live `meta/`
  corpus — so a real plan in this repo's `meta/plans/` can be rendered directly.
- **Decision: integrations framing** — move both Jira and Linear under a new
  **"Remote Work Item Management"** section (peer to the existing local "Work
  Item Management" section).

## Desired End State

- `CHANGELOG.md` `[Unreleased]` describes every user-facing 1.22.0 change,
  grouped Added / Changed / Migrations, and still parses under `keepachangelog`.
- `README.md` opens with a hero screenshot + Getting Started install, documents
  Jira and Linear under "Remote Work Item Management", and has no remaining
  accuracy drift in the integration/notes/visualiser areas.
- `assets/` contains a committed light + dark plan screenshot embedded via a
  `<picture>` block matching the logo pattern.
- `mise run check` still exits 0; no internal README anchors dangle.

## What We're NOT Doing

- **Not** changing the config-dump template registry; the `TEMPLATE_KEYS`
  under-reporting is tracked in work item 0113 (derive the dump's rows from the
  `templates/` directory). This plan corrects the docs only.
- **Not** adding CHANGELOG entries for non-user-facing work (lint/format/type
  guardrails, bash-3.2 fixes, CI/release-pipeline changes, Docker VR infra,
  `/dev` reference page, corpus-validator internals, dev-task orchestration,
  `CLAUDE.md`, dogfooded planning artifacts).
- **Not** cutting the release or bumping versions — this refreshes documents
  only; `release:prepare` later promotes `[Unreleased]`.
- **Not** restructuring unrelated README sections (Philosophy, Design
  Convergence, Review System, Agents).

## Implementation Approach

Edit the two documents directly. Each phase is a coherent, independently
mergeable unit that leaves both documents valid. TDD has limited applicability
to prose, so where verification has teeth we apply it verification-first:
capture the keepachangelog parse and an internal-anchor check as repeatable
commands and confirm them after each relevant phase. The phases are ordered for
convenience but are order-independent — none depends on another's edits.

---

## Phase 1: CHANGELOG `[Unreleased]` refresh

### Overview

Complete the `[Unreleased]` section with the missing user-facing entries and a
new Migrations subsection for 0007. Keep the existing upgrade note and idle
entry. Fold the visualiser reader-polish items into a single Changed line for
signal-to-noise (per research recommendation).

**Grouping decision**: 1.22.0 uses Added / Changed / Migrations only — no
`Removed` or `Fixed` groups. Nothing user-facing was removed or bug-fixed this
cycle; the one removal-adjacent item (the reader's pre-migration
`work-item:`/`ticket:`/filename fallbacks) is kept under Changed as the existing
draft already files it, since it is inseparable from the unified-schema reader
change. This is a deliberate choice, not an omission.

### Changes Required

#### 1. Expand the `Added` group

**File**: `CHANGELOG.md` (within `[Unreleased]` → `### Added`)
**Changes**: Append entries after the existing idle auto-shutdown item.

```markdown
- **Linear Cloud integration** — eight verb-decomposed skills for working with a
  Linear workspace directly over the GraphQL API (no external CLI dependency).
  Run `/accelerator:init-linear` once to verify the token and cache the team and
  workflow-state catalogue, then use:
  - `search-linear-issues` — search issues by state, assignee, label, or text
  - `show-linear-issue` — read a single issue with comments
  - `create-linear-issue` — create an issue from a work-item file (payload
    preview, then confirm)
  - `update-linear-issue` — edit title, description, state, assignee, or priority
  - `comment-linear-issue` — add a comment
  - `transition-linear-issue` — move an issue through its workflow by state name
  - `attach-linear-issue` — attach a URL or file

  Token-only auth: set `linear.token` (or `linear.token_cmd`) in the gitignored
  `.accelerator/config.local.md`; set `work.integration: linear` to enable
  auto-scoping. `init-linear` caches the team and workflow-state catalogue under
  `.accelerator/state/integrations/linear/`. Read skills auto-trigger on natural
  language; write skills are slash-only with a payload preview and confirmation,
  mirroring the Jira set. See the
  [Linear subsection of the README](README.md#linear).
- **`/accelerator:create-note`** — capture a short-form note (observation,
  insight, snippet) to `meta/notes/` using a new `note` template. Single
  round-trip, no sub-agents.
- **Visualiser — global search**: a sidebar search box (focus with `/`) across
  every indexed document's title, slug, and body preview, bucket-and-rank
  ordered.
- **Visualiser — detail-page actions**: "Copy path" and "Open in editor"
  buttons on document pages. The editor deep-link is configured via the new
  `visualiser.editor` key (VS Code-family and JetBrains presets, or a custom
  `{abs}`/`{rel}` URL template; `ACCELERATOR_VISUALISER_EDITOR` one-shot
  override; `visualiser.editor_project` for JetBrains). The button renders
  disabled with a tooltip when unset.
- **Visualiser — recovery surfaces**: a document-not-found page with ranked
  "Did you mean…" suggestions, a router catch-all not-found page, and a
  load-error surface.
- **Visualiser — browsable root-cause analyses**: RCAs from
  `meta/research/issues/` appear as a first-class document type under a new
  "Operate" category.
- **Visualiser — Templates view**: templates are auto-discovered from the
  `templates/` directory and browsable in the sidebar's META section, each
  showing its active resolution tier and content.
- **`rejected` ADR status** — added to the ADR status vocabulary
  (`proposed | accepted | rejected | superseded | deprecated`).
- **Remote-tracker sync ergonomics**: `/accelerator:create-work-item` offers to
  push to the configured tracker on accept; `/accelerator:create-jira-issue`
  accepts a work-item file and writes the created key back to `external_id`;
  `/accelerator:list-work-items` shows a per-item sync label and Sync column
  when an integration is configured.
```

#### 2. Expand the `Changed` group

**File**: `CHANGELOG.md` (within `[Unreleased]` → `### Changed`)
**Changes**: Add two items above/among the existing reader/idle/migration items.

```markdown
- Visualiser lifecycle clustering now groups entries by composite typed-linkage
  (walking `parent:` / `target:` back to a canonical work-item id) rather than
  by slug alone; decisions (ADRs) and RCAs are dropped from the rendered
  pipeline stages.
- Visualiser reader polish: a remapped numeric typography size scale, shared
  border-radius tokens, styled markdown tables / inline code / task-list
  checkboxes, and smoother kanban drag-and-drop.
```

#### 3. Add a `Migrations` subsection

**File**: `CHANGELOG.md` (new `### Migrations` at the end of `[Unreleased]`,
mirroring the 1.21.0 entry's structure)
**Changes**:

```markdown
### Migrations

This release adds migration **0007**. After updating the plugin, run
`/accelerator:migrate` (see the upgrade note above): the runner applies pending
migrations in numeric order, refuses to run on a dirty working tree, previews
each one, and records results in `.accelerator/state/migrations-applied`.
Recover from a failed migration with a VCS revert (`jj op restore` /
`git reset`) and re-run — every migration is idempotent.

- **0007 — Unify the `meta/` corpus to the ADR-0033/0034 schema.** Canonical
  `id:` identity, typed linkage, provenance fields, status-vocabulary
  reconciliation, and fence-less frontmatter backfill. This is the only
  **interactive** migration (`# INTERACTIVE: yes`): the body-section
  typed-linkage step prompts on ambiguous references. It has a read-only
  precondition pre-pass that refuses to run if 0005/0006 haven't been applied.
  Run `/accelerator:migrate` to apply it: until it runs, items still keyed by
  the old `work-item:` / `ticket:` / filename-derived shapes lose their identity
  and cross-references and drop out of the visualiser library and kanban —
  applying the migration restores them.
```

### Success Criteria

#### Automated Verification

- [x] CHANGELOG still parses: `uv run python -c "import keepachangelog; keepachangelog.to_dict('CHANGELOG.md')"`
- [x] `[Unreleased]` exists exactly once: `test "$(grep -c '^## \[Unreleased\]' CHANGELOG.md)" -eq 1`
- [x] `[Unreleased]` precedes `[1.21.0]` (asserted, not eyeballed):
      `test "$(grep -n '^## \[Unreleased\]' CHANGELOG.md | cut -d: -f1)" -lt "$(grep -n '^## \[1.21.0\]' CHANGELOG.md | cut -d: -f1)"`
- [x] No accidental link/path breakage in the diff: `mise run check`

#### Manual Verification

- [x] Every added entry describes a **user-facing** change (no internal/CI/lint
      items leaked in).
- [x] Skill names, config keys, and paths match the codebase (Linear skills,
      `visualiser.editor`, `meta/notes/`).
- [x] The 0007 Migrations entry is consistent with the existing upgrade note (no
      contradiction or duplication).

---

## Phase 2: README top-of-file — Getting Started + hero screenshot

### Overview

Add a light/dark plan hero screenshot below the logo and a concise Getting
Started install block, then drop the now-redundant "Jump to installation" line.
Capture the screenshot fresh against a real plan rendered by the dev visualiser.

### Changes Required

#### 1. Capture the hero screenshot (fresh, real plan)

**Mechanics**:

1. Build/start the dev stack: `mise run dev:up` (dev binary at
   `skills/visualisation/visualise/server/target/debug/accelerator-visualiser`).
   Point it at this repo's real `meta/` so real plans render.
2. Render `meta/plans/2026-06-06-0067-create-note-skill.md` as the hero. Its
   route is `/library/plans/create-note-skill` — the visualiser's plan slug
   derivation (`strip_prefix_date_and_optional_id`) strips **both** the date
   prefix **and** the leading canonical work-item id (`0067`), so the slug is
   the filename stem minus the date *and* the id. (This holds for this repo's
   default-numeric id pattern, where `0067` is a canonical id token; under a
   project-code-prefixed pattern the id may not be stripped, so the slug is
   config-dependent.) Confirm it resolves from the sidebar before capturing —
   that live check validates the slug regardless of config.
3. Capture light and dark at a fixed desktop viewport (e.g. 1280×800) using
   Playwright (toggle the visualiser theme between captures). Crop/scale
   consistently so both themes match dimensions.
4. Save as `assets/visualiser_plan_light.png` and
   `assets/visualiser_plan_dark.png` (PNG, reasonably optimised file size).

**File**: `assets/visualiser_plan_{light,dark}.png` (new)

#### 2. Embed the screenshot + Getting Started below the logo

**Heading-slug collision (must resolve)**: the README already has a
`### Getting Started` heading at `README.md:240` (under Project Context, about
running `/accelerator:configure` to create or view your config). Adding a
top-level `## Getting Started` would create two headings that both slugify to
`getting-started`; GitHub keeps `#getting-started` for the first in document
order and appends `-1` to the second. To avoid the duplicate text and the
fragile auto `-1` slug, **rename the pre-existing `README.md:240` heading** to
`### Managing Configuration` (it covers creating *and* viewing config via
`/accelerator:configure`, not just first-time setup, and pairs with the sibling
`### Template Management` heading). The new top-of-file `## Getting Started` then
owns `#getting-started`, which the Installation cross-reference below targets;
the renamed section gets the clean, descriptive `#managing-configuration` slug.
Nothing currently links to `#getting-started` (verified), so no existing link
breaks.

**File**: `README.md` (after the existing tagline at `README.md:9`)
**Changes**: Keep the logo block (`:1-7`) and the existing tagline line
(`README.md:9`, "A Claude Code plugin for structured, context-efficient software
development.") **as-is**. Replace **only** the "[Jump to installation]" line
(`README.md:10`) with the hero `<picture>` and a Getting Started block below the
tagline. Do **not** re-introduce the tagline in the new block — that would
duplicate `:9`.

```markdown
<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/visualiser_plan_dark.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/visualiser_plan_light.png">
    <img alt="The Accelerator visualiser rendering a plan document" src="assets/visualiser_plan_light.png" width="760px">
  </picture>
</p>

## Getting Started

Add the marketplace and install the stable plugin:

```bash
/plugin marketplace add atomicinnovation/accelerator
/plugin install accelerator@atomic-innovation
```

Then initialise your project and run the research → plan → implement loop:

```bash
/accelerator:init
/accelerator:research-codebase "how does auth work?"   # 1. research
/accelerator:create-plan                               # 2. plan (optionally pass a work-item key)
/accelerator:implement-plan                            # 3. implement
```

See the [Development Loop](#the-development-loop) for the full workflow, and
[Installation](#installation) for the prerelease channel (where the newest
features land first), local checkout, and compatibility details.
```

(The full Installation section at `README.md:713` stays as the deep reference.
Also add a short clause to the Installation section's opening line noting it
expands on Getting Started — e.g. "The stable install (also shown in
[Getting Started](#getting-started)) plus the prerelease channel, local
checkout, and compatibility." — so the two install blocks are explicitly
related rather than appearing as duplicates. Verify the
`#the-development-loop` / `#getting-started` anchor slugs against the actual
heading text before finalising.)

### Success Criteria

#### Automated Verification

- [x] Both assets exist: `test -f assets/visualiser_plan_light.png && test -f assets/visualiser_plan_dark.png`
- [x] No dangling "Jump to installation" reference remains: `! grep -q 'Jump to installation' README.md`
- [x] The `#installation` anchor target still exists: `grep -qi '^## Installation' README.md`
- [x] Exactly one "Getting Started" heading remains (collision resolved by the
      rename): `test "$(grep -ciE '^#+ +getting started$' README.md)" -eq 1`
- [x] The renamed Configuration section exists: `grep -q '^### Managing Configuration' README.md`
- [x] `mise run check` exits 0.

#### Manual Verification

- [~] On GitHub, the hero renders correctly in both light and dark colour
      schemes and the plan content is legible. Also sanity-check the bare
      `<img>` light fallback in a renderer that ignores `prefers-color-scheme`
      (a dense plan screenshot is less forgiving than the logo), and confirm both
      captures share identical crop/dimensions. — captures verified locally:
      both 1280×800 (identical dimensions), legible in both themes; `<img>`
      fallback points at the light capture. GitHub branch-preview render is the
      remaining human check.
- [x] The hero `<img>` alt text is meaningful and intentionally theme-agnostic
      (one alt string serves both light/dark sources), matching the logo block's
      alt-text precedent — for screen-reader / no-image users.
- [x] Getting Started commands are copy-pasteable and correct (marketplace slug,
      plugin id, `/accelerator:init`).
- [x] The top of the README reads cleanly without the removed jump link.

---

## Phase 3: README — "Remote Work Item Management" umbrella (Jira + Linear)

### Overview

Introduce a "Remote Work Item Management" section (peer to the local "Work Item
Management" section) that houses the existing Jira Integration content (moved
under it as a subsection) and a new Linear subsection mirroring Jira's shape.

### Changes Required

#### 1. Add the umbrella heading and intro

**File**: `README.md` (new `## Remote Work Item Management`, placed immediately
after the "Work Item Management" section ends, before "Architecture Decision
Records")
**Changes**: One short intro paragraph that **names Jira and Linear in its first
sentence** (so a reader's Ctrl-F for the tracker they know lands at the section
top, not on the unfamiliar umbrella term), then frames both as instances of one
integration pattern (read skills auto-trigger; write skills are slash-only with
preview + confirmation; team-shared catalogue + gitignored per-dev credentials;
`external_id`-presence sync signal; selected via `work.integration`). Consider
parenthesising the trackers in the heading itself —
`## Remote Work Item Management (Jira & Linear)` — for discoverability.

#### 2. Move Jira under it as a subsection

**File**: `README.md` (the existing Jira Integration section, `README.md:327-404`)
**Changes**: Demote `## Jira Integration` to `### Jira` (and its subsections from
`###` to `####`) and relocate the block under the new umbrella.

**Anchor fix (decided, not deferred)**: demoting `## Jira Integration` → `### Jira`
changes its slug from `#jira-integration` to `#jira`, which breaks the live link
in the **already-released** 1.21.0 CHANGELOG entry at `CHANGELOG.md:90`
(`[Jira Integration section of the README](README.md#jira-integration)`). We do
**not** edit the frozen 1.21.0 entry — that link documents a historical state and
rewriting released changelog history is undesirable. Instead, **preserve the
`jira-integration` anchor** by giving the demoted heading an explicit HTML anchor
immediately above it, so the existing link continues to resolve:

```markdown
<a id="jira-integration"></a>

### Jira
```

(The auto-generated `#jira` slug also resolves; the explicit anchor keeps the
older `#jira-integration` target alive.) Re-scan for any other in-document
references to `#jira-integration` and confirm they resolve.

#### 3. Add the Linear subsection

**File**: `README.md` (new `### Linear` under the umbrella, mirroring Jira)
**Changes**: Mirror the Jira subsection, stating the differences. Match the
post-demotion heading hierarchy: `### Linear` as the subsection, and any
internal parts (Configuration, Skills) at `####` — the same levels Jira's
sub-parts take after their demotion in Section 2.

- **Auth**: token-only (`linear.token` / `linear.token_cmd`) — no site/email;
  token sent verbatim (no `Bearer`). Resolution env → `config.local.md` →
  `config.md` (token only). `config.local.md` must be mode ≤0600.
- **API**: Linear GraphQL (`api.linear.app/graphql`), Markdown-native (no ADF).
- **Scoping**: single team, fixed at `init-linear`; catalogue cached under
  `.accelerator/state/integrations/linear/` (`catalogue.json` committed;
  `viewer.json` gitignored, per-dev).
- **Enable**: `work.integration: linear`.

Skill table (8 rows). The Usage column mirrors the simplified placeholder
style of the existing Jira table (not the verbatim argument-hint), but each
placeholder must be **accurate per skill** — note the key divergences from Jira:
the identifier token is `<IDENTIFIER>` (not Jira's `<KEY>`), `create-linear-issue`
takes a **required `<work-item-file>` positional** (it is work-item-file-driven,
unlike Jira's flag-based create), and `attach-linear-issue` takes `--url`/`--file`
(not positional `<file...>`):

```markdown
| Skill                       | Usage                                                              | Description                                                              |
|-----------------------------|--------------------------------------------------------------------|--------------------------------------------------------------------------|
| **init-linear**             | `/accelerator:init-linear`                                         | Verify the token, cache the team and workflow-state catalogue            |
| **search-linear-issues**    | `/accelerator:search-linear-issues [flags]`                        | Search issues by state, assignee, label, or text (cursor-paginated)      |
| **show-linear-issue**       | `/accelerator:show-linear-issue <IDENTIFIER>`                      | Read a single issue, with an optional comment slice                      |
| **create-linear-issue**     | `/accelerator:create-linear-issue <work-item-file>`                | Create an issue from a work-item file (payload preview, then confirm)    |
| **update-linear-issue**     | `/accelerator:update-linear-issue <IDENTIFIER> [flags]`            | Edit title, description, state, assignee, or priority on an issue        |
| **comment-linear-issue**    | `/accelerator:comment-linear-issue <IDENTIFIER> --body …`          | Add a comment (`--body` text or `--body-file`)                           |
| **transition-linear-issue** | `/accelerator:transition-linear-issue <IDENTIFIER> <STATE-NAME>`   | Move an issue through its workflow by state name                         |
| **attach-linear-issue**     | `/accelerator:attach-linear-issue <IDENTIFIER> (--url \| --file)`  | Attach a URL or file to an issue                                         |
```

**Binding pre-merge gate** (not optional): before this table ships, cross-check
every Usage cell against the `argument-hint:` frontmatter of the corresponding
`skills/integrations/linear/<skill>/SKILL.md`. The verbatim hints at time of
writing are: `init-linear` `[--team-id <uuid>]`; `search-linear-issues`
`[--state NAME] [--assignee NAME] [--label NAME] [--text STR] [--limit 1..250]
[--quiet]`; `show-linear-issue` `<IDENTIFIER> [--comments N]`; `create-linear-issue`
`<work-item-file> [--print-payload] [--quiet]`; `update-linear-issue`
`<IDENTIFIER> [--title TEXT] [--description TEXT] [--state NAME] [--assignee-id ID]
[--priority N] [--print-payload] [--quiet]`; `comment-linear-issue`
`<IDENTIFIER> --body TEXT | --body-file PATH [--print-payload] [--quiet]`;
`transition-linear-issue` `<IDENTIFIER> <STATE-NAME> [--describe] [--quiet]`;
`attach-linear-issue` `<IDENTIFIER> (--url URL | --file PATH) [--title T]
[--describe] [--quiet]`.

### Success Criteria

#### Automated Verification

- [x] The umbrella section exists: `grep -q '^## Remote Work Item Management' README.md`
- [x] Jira and Linear appear as subsections, not top-level: `grep -nE '^#{2,4} (Jira|Linear)' README.md`
- [x] The `#jira-integration` target survives the demotion (resolves via the
      explicit anchor): `grep -q '<a id="jira-integration">' README.md` — so the
      released `CHANGELOG.md:90` link is not broken.
- [x] No duplicate heading slugs introduced (the rename removes the only
      collision): `grep -oiE '^#+ +[a-z0-9 -]+' README.md | sed -E 's/^#+ +//' | sort | uniq -d`
      is empty. (Resolved a *pre-existing* `Configuration` collision too by
      prefixing the Jira/Linear sub-headings with the tracker name — per user
      decision — so the gate now passes cleanly.)
- [x] All 8 Linear skill names match `ls skills/integrations/linear/`.
- [x] `mise run check` exits 0.

#### Manual Verification

- [x] Anchor cross-check (manual): every `](#...)` target in README/CHANGELOG
      resolves to a generated heading slug or an explicit `<a id="...">` —
      `grep -oE '\]\(#[a-z0-9-]+\)' README.md` reviewed against the heading set
      (this regex misses uppercase/other-char targets, so it is a manual aid,
      not a self-enforcing gate).
- [x] Jira content is unchanged in substance after the demotion (only heading
      levels / placement changed — plus the agreed tracker-name prefix on the
      Configuration/Skills/ADF/state sub-headings).
- [x] Linear subsection accurately states token-only auth, GraphQL,
      Markdown-native, single-team scoping.
- [x] **Every Linear Usage cell matches the skill's `argument-hint` frontmatter**
      (binding gate) — in particular `create-linear-issue`'s required
      `<work-item-file>` positional, `<IDENTIFIER>` (not `<ID>`), the
      `<STATE-NAME>` transition arg, and `attach`'s `--url`/`--file`.
- [x] The umbrella intro reads as a coherent framing of both trackers.

---

## Phase 4: README — accuracy pass

### Overview

Fix the remaining accuracy drift surfaced by the research: integration
availability, `create-note` / `notes/`, the visualiser editor keys and views,
and the stale template-keys list.

### Changes Required

#### 1. Integration availability clause

**File**: `README.md:294-295` (Work Item Management `work.integration` blurb)
**Changes**: Add a clause that only `jira` and `linear` currently have skills;
`trello` and `github-issues` are reserved values with no implementation yet.

#### 2. `create-note` + `notes/` row

**File**: `README.md:100` (the `meta/` directory table) and the Development Loop
or a short Notes line
**Changes**: Change the `notes/` "Written by" cell from `manual` to
`create-note`. Add a one-line mention of `/accelerator:create-note` (capture
short-form notes to `meta/notes/`). Do **not** add a caveat that the `note`
template is uncustomisable — it is ejectable and overridable like every other
template (the `configure templates` surface is filename-driven, not
`TEMPLATE_KEYS`-gated), so such a caveat would be false.

#### 3. Visualiser Customisation table — editor keys

**File**: `README.md:494-501` (Visualiser → Customisation table)
**Changes**: Add rows for the editor deep-link configuration:

```markdown
| `visualiser.editor` config key             | Editor deep-link for the detail-page "Open in editor" action (preset key or `{abs}`/`{rel}` URL template) |
| `ACCELERATOR_VISUALISER_EDITOR`            | One-shot override of `visualiser.editor`                          |
| `visualiser.editor_project` config key     | JetBrains project name for the editor deep-link (defaults to the project directory basename) |
| `ACCELERATOR_VISUALISER_EDITOR_PROJECT`    | One-shot override of `visualiser.editor_project`                  |
```

Keep the existing table's Mechanism-cell convention: config keys carry the
trailing ` config key` suffix and env vars are bare uppercase tokens (the rows
above already follow this — preserve it rather than regress it when inserting).

#### 4. Visualiser Views table + prose

**File**: `README.md:449-453` (Views table) and surrounding prose
**Changes**:

- Fix the Library row's "(plans, research, ADRs, tickets …)" → "work items".
- Fix the Kanban row's "Ticket board" (`README.md:453`) → "Work-item board" —
  the same tickets→work-items drift, in the same table.
- Update the Lifecycle row to note typed-linkage clustering (not slug-only).
- Add a note below the table that templates are also browsable (sidebar META
  section), and that the reader supports global search (`/`), browsable RCAs,
  and not-found / load-error recovery.

#### 5. Template-keys list correction

**File**: `README.md:226-227` ("Available template keys" line)
**Changes**: The list currently reads `plan`, `research`, `adr`, `validation`,
`pr-description`, `work-item`, `design-inventory`, `design-gap` — stale on two
counts: `research` should be `codebase-research`, and it omits other ejectable
templates. The `configure templates` subcommands enumerate
`templates/*.md` (they are **not** gated by `TEMPLATE_KEYS`), so the accurate
list is all 13 shipped template keys: `adr`, `codebase-research`, `design-gap`,
`design-inventory`, `note`, `plan`, `plan-review`, `pr-description`, `pr-review`,
`rca`, `validation`, `work-item`, `work-item-review`. Enumerate those. (The
config dump's narrower `TEMPLATE_KEYS` view is a separate concern, tracked in
work item 0113.)

### Success Criteria

#### Automated Verification

- [x] No stale "manual" in the `notes/` row: `grep -n 'notes/' README.md` shows
      `create-note`.
- [x] Editor keys are documented: `grep -q 'visualiser.editor' README.md && grep -q 'ACCELERATOR_VISUALISER_EDITOR' README.md && grep -q 'editor_project' README.md`
- [x] `create-note` is mentioned: `grep -q 'create-note' README.md`
- [x] Template-keys list reflects the filename-driven surface (all 13 ejectable
      templates), not the narrower `TEMPLATE_KEYS`: it names `codebase-research`
      and re-includes the backtick-quoted `note`, `design-inventory`, and
      `design-gap` template keys:
      ``grep -q 'codebase-research' README.md && grep -q '`design-inventory`' README.md && grep -q '`note`' README.md``
- [x] `mise run check` exits 0.

#### Manual Verification

- [x] The `work.integration` blurb no longer overstates available trackers.
- [x] The visualiser Customisation and Views content matches
      `skills/visualisation/visualise/SKILL.md`.
- [x] The literal sidebar labels asserted in the CHANGELOG ("Operate" category
      for RCAs, the Templates view's META section) match the shipped frontend
      (sidebar source or a running instance) — confirmed against the running dev
      visualiser (sidebar showed the OPERATE category / Root cause analyses) and
      `Sidebar.tsx` (`meta-heading` → "META", `Templates` label, `Operate`
      category label); vocabulary is consistent between the CHANGELOG and the
      README Views prose.
- [x] No "tickets" terminology remains where "work items" is meant.

---

## Testing Strategy

### Automated (per phase, above)

- `keepachangelog.to_dict()` parse of `CHANGELOG.md`.
- `grep`-based invariants: single `[Unreleased]`, anchor integrity, skill-name
  matches, asset presence, key presence.
- `mise run check` as the non-regression gate (confirms no edit broke a path or
  tooling input; README/CHANGELOG are not themselves linted, so the grep checks
  carry the doc-correctness load).

### Manual

0. Wrap all new CHANGELOG/README **prose** at 80 columns (tables exempt, per
   existing precedent) — README/CHANGELOG markdown is not auto-linted, so this
   is hand-enforced. Cheap guard over changed prose lines:
   `awk 'length>80 && $0 !~ /\|/' <changed-lines>`.
1. Render the README on a GitHub branch preview; confirm the hero `<picture>`
   switches light/dark and the plan is legible.
2. Read the README top-to-bottom for flow after the restructure.
3. Cross-read the CHANGELOG against the research's "what to add" list to confirm
   completeness without leaking internal items.

## Migration Notes

None — this changes documentation only. No config keys, directories, or file
formats change.

## References

- Research: `meta/research/codebase/2026-06-17-readme-changelog-1.22.0-refresh.md`
- `CHANGELOG.md:3-44` — current `[Unreleased]`; `:46-217` — 1.21.0 (structure to
  mirror for the Migrations subsection).
- `README.md:10` — "Jump to installation"; `:100` — `notes/` row; `:294-295` —
  `work.integration` blurb; `:327-404` — Jira Integration section;
  `:444-515` — Visualiser section.
- `tasks/changelog.py:1-12` — keepachangelog release contract.
- `scripts/config-defaults.sh:66-96` — `TEMPLATE_KEYS`, `WORK_INTEGRATION_VALUES`.
- `skills/visualisation/visualise/SKILL.md:122-155` — editor config docs.
- `skills/integrations/linear/` — 8 Linear skills; `skills/notes/create-note/`,
  `templates/note.md` — create-note.
- `.claude-plugin/plugin.json:17,26` — Linear + notes registration.

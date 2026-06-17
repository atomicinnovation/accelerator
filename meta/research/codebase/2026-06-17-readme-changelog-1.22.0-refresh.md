---
type: codebase-research
id: "2026-06-17-readme-changelog-1.22.0-refresh"
title: "Research: README and CHANGELOG refresh for the 1.22.0 release"
date: "2026-06-17T11:10:41+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
topic: "README and CHANGELOG refresh for the 1.22.0 release"
tags: [research, codebase, changelog, readme, release, visualiser, linear, migrations]
revision: "d382c2656992f770140fed6f89a2552c0cce91be"
repository: "accelerator"
last_updated: "2026-06-17T11:10:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: README and CHANGELOG refresh for the 1.22.0 release

**Date**: 2026-06-17 11:10 UTC
**Author**: Toby Clemson
**Git Commit**: d382c2656992f770140fed6f89a2552c0cce91be
**Branch**: workspace `ticket-management` (main @ 836a21ace4e1)
**Repository**: accelerator

## Research Question

We are releasing 1.22.0. Two documents need a refresh:

1. **CHANGELOG** — the `[Unreleased]` section must fully represent every
   *user-facing* change between 1.21.0 and the latest commit on main, written
   for **plugin users**, not Accelerator developers.
2. **README** — verify accuracy across everything since 1.20.0 (1.21.0 may not
   have been reflected), add a concise top-of-file **Getting Started** with
   stable-release install instructions, and add a light/dark visualiser
   screenshot of a **plan document** below the logo.

Keep both terse and high signal-to-noise.

## Summary

344 commits separate `v1.21.0` from `main`; the overwhelming majority are
internal (build system, CI, bash-3.2 fixes, linting guardrails, visual
regression infra, dev-task orchestration, corpus-validator internals, and
planning/dogfooding artifacts) and **must not** appear in a user-facing
CHANGELOG. The genuinely user-facing surface since 1.21.0 is:

- **Linear integration** — 8 new skills mirroring the Jira set (biggest gap).
- **`create-note` skill** + `note` template (new `notes/` category).
- **Visualiser**: global sidebar search, detail-page "Copy path" / "Open in
  editor" actions (`visualiser.editor`), not-found / load-error recovery
  surfaces, RCA browsable doc type, lifecycle clustering by typed-linkage, a
  Templates view (auto-discovered), and a typography/markdown polish pass.
- **Remote-tracker sync ergonomics**: `/create-work-item` push-on-accept,
  `/create-jira-issue` work-item-file mode, `/list-work-items` sync labels.
- **`rejected`** added to the ADR status vocabulary.
- **Migration 0007** — unifies the `meta/` corpus to the ADR-0033/0034
  frontmatter schema; the 1.22.0 visualiser reads **only** this schema.

The current `[Unreleased]` CHANGELOG block documents **only** the configurable
idle auto-shutdown, the unified linkage reader, the 8h idle default, and the
migration-merge behaviour. Everything in the list above (except the idle items
and the reader/migration note) is **missing**.

The README largely reflects 1.21.0 already (Jira, visualiser, design
convergence, `research-issue`, `.accelerator/` paths, 8h idle default). Its
gaps are: no Getting Started / top-of-file install, no Linear section, no
`create-note`, a stale `notes/` "Written by manual" row, missing `visualiser.editor`
customisation keys, and an overstated-by-omission integration list.

Reusable light + dark PLAN screenshots **already exist** as committed
visual-regression baselines and can be copied into `assets/`, or fresh
high-fidelity captures can be produced from the dev stack (see Screenshots).

## Detailed Findings

### CHANGELOG — what to add to `[Unreleased]` (1.22.0)

Target audience: plugin users. Proposed grouping (Keep-a-Changelog):

**Added**

- **Linear Cloud integration** — eight verb-decomposed skills over the Linear
  GraphQL API: `init-linear`, `search-linear-issues`, `show-linear-issue`,
  `create-linear-issue`, `update-linear-issue`, `comment-linear-issue`,
  `transition-linear-issue`, `attach-linear-issue`. Token-only auth
  (`linear.token` / `linear.token_cmd`); `work.integration: linear` enables
  auto-scoping. `init-linear` caches the team + workflow-state catalogue under
  `.accelerator/state/integrations/linear/`. Read skills auto-trigger; write
  skills are slash-only with a payload preview + confirmation, exactly like
  Jira. (`skills/integrations/linear/`, registered `plugin.json:17`.)
- **`/accelerator:create-note`** — capture a short-form note (observation,
  insight, snippet) to `meta/notes/` using a new `note` template; single
  round-trip, no sub-agents. (`skills/notes/create-note/`, `templates/note.md`,
  category registered `plugin.json:26`.)
- **Visualiser — global search**: a sidebar search box (focus with `/`) over
  every indexed doc's title/slug/body preview, bucket-and-rank ordered, backed
  by `GET /api/search`.
- **Visualiser — detail-page actions**: "Copy path" and "Open in editor"
  buttons on document pages. Editor deep-link configured via the new
  `visualiser.editor` key (VS Code / JetBrains presets or a `{abs}`/`{rel}`
  URL template; `ACCELERATOR_VISUALISER_EDITOR` one-shot override;
  `visualiser.editor_project` for JetBrains). Disabled with a tooltip when
  unset.
- **Visualiser — recovery surfaces**: a document-not-found page with ranked
  "Did you mean…" nearby-slug suggestions, a router catch-all not-found page,
  and a load-error surface.
- **Visualiser — root-cause analyses are browsable**: RCAs from
  `meta/research/issues/` now appear as a first-class doc type under a new
  "Operate" category (peer to the lifecycle, not in kanban/lifecycle).
- **Visualiser — Templates view**: templates are auto-discovered from the
  `templates/` directory and browsable in the sidebar's META section, each
  showing its active resolution tier and content.
- **`rejected` ADR status** — added to the ADR status vocabulary
  (`proposed | accepted | rejected | superseded | deprecated`).
- **Remote-tracker sync ergonomics**: `/create-work-item` offers push-on-accept
  to the configured tracker; `/create-jira-issue` accepts a work-item file and
  writes the created key back to `external_id`; `/list-work-items` shows a
  per-item sync label + Sync column when an integration is configured.

**Changed**

- **Visualiser — lifecycle clustering** now groups entries by composite
  typed-linkage (walking `parent:`/`target:` back to a canonical work-item id)
  rather than slug only; decisions (ADRs) and RCAs are dropped from the
  rendered pipeline stages.
- **Visualiser — reader polish**: remapped numeric typography size scale,
  shared border-radius tokens, styled markdown tables / inline code /
  task-list checkboxes, smoother kanban drag-and-drop.
- *(already in draft)* unified-schema linkage reader; idle default 30m→8h;
  migration 0001/0003/0004 merge-on-relocation.

**Migrations** (new subsection, mirroring 1.21.0)

- **0007 — Unify the `meta/` corpus to the ADR-0033/0034 schema.** Canonical
  `id:` identity, typed linkage, provenance fields, status-vocabulary
  reconciliation, and fence-less backfill. The only **interactive** migration
  (`# INTERACTIVE: yes`) — the body-section typed-linkage step prompts on
  ambiguous references. Idempotent; has a read-only precondition pre-pass that
  refuses if 0005/0006 haven't run. Until it runs, items still keyed by the old
  `work-item:`/`ticket:` shapes lose their identity/cross-references and drop
  out of the library and kanban.

Already-present idle auto-shutdown entry (Added) stays.

**Excluded as non-user-facing** (do not add): repo-wide lint/format/type
guardrails, ShellCheck/shfmt/bashisms work, bash-3.2 fixes, CI/release-pipeline
changes (concurrency, RUSTUP cache, atomic push), Docker visual-regression
infra + baseline regens, the `/dev` DevDesignSystem reference page (developer
tool), corpus-frontmatter validator internals, dev-task (`circus`)
orchestration, `CLAUDE.md`, and all dogfooded planning artifacts.

### README — required updates

1. **Getting Started (new, top of file).** Insert directly below the logo (and
   the new screenshot) a concise block with the stable install:
   ```
   /plugin marketplace add atomicinnovation/accelerator
   /plugin install accelerator@atomic-innovation
   ```
   Then `/accelerator:init` and a one-line pointer to the research→plan→implement
   loop. The full "Installation" section (README.md:713) stays as the deep
   reference (prerelease channel, dev checkout, compatibility); the existing
   "[Jump to installation]" line (README.md:10) can be dropped once install is
   at the top.

2. **Screenshot below logo (new).** Light + dark `<picture>` of the visualiser
   rendering a plan document (see Screenshots section).

3. **Linear Integration section (new).** Mirror the Jira Integration section
   (README.md:327). Differences to state: token-only auth (no site/email),
   GraphQL, Markdown-native (no ADF), single-team scoping fixed at
   `init-linear`. Skill table: the 8 skills above. Consider re-titling to a
   single **Integrations** umbrella with Jira and Linear subsections.

4. **Integration availability.** `work.integration` accepts
   `jira | linear | trello | github-issues` (validated —
   `config-defaults.sh:91`), but **only jira and linear have skills**; trello /
   github-issues are reserved values with no implementation. The README's Work
   Item Management blurb (README.md:295) lists all four — add a clause that only
   jira and linear are currently implemented so it doesn't overstate.

5. **`create-note`.** Add a brief mention (Development Loop or a short
   "Notes" line) and fix the `meta/` table: `notes/` "Written by" should be
   `create-note`, not "manual" (README.md:100).

6. **Visualiser section** (README.md:444):
   - Add `visualiser.editor` / `ACCELERATOR_VISUALISER_EDITOR` and
     `visualiser.editor_project` to the Customisation table (README.md:494).
   - Optionally note global search (`/`), the Templates view, RCA browsing, and
     not-found recovery. The Views table (README.md:449) lists 3 views; a
     Templates view now also exists (sidebar META section).
   - Idle default already reads 8h (README.md:474, 499) — no change needed.

7. **Spot-checks that are already accurate** (no change): `.accelerator/` config
   paths, 13 built-in review lenses + `.accelerator/lenses/`, per-skill
   customisation paths, migrations section paths
   (`skills/config/migrate/scripts/run-migrations.sh`,
   `.accelerator/state/migrations-{applied,skipped}`), `research/` subcategories,
   `MIGRATION_RESULT: no_op_pending` contract, min Claude Code v2.1.144.

### Linear integration (detail)

- Skills: `skills/integrations/linear/{init-linear,search-linear-issues,
  show-linear-issue,create-linear-issue,update-linear-issue,comment-linear-issue,
  transition-linear-issue,attach-linear-issue}/SKILL.md`.
- API: Linear GraphQL (`api.linear.app/graphql`), cursor pagination (cap 20
  pages), 10k-point complexity cap. Transport `linear-graphql.sh`.
- Auth: `linear.token` / `linear.token_cmd` only (no site/email); token sent
  verbatim (no `Bearer`). Resolution: env → `config.local.md` → `config.md`
  (token only, and only if no local). `config.local.md` must be ≤0600.
- Init caches `viewer.json` (gitignored, per-dev) + `catalogue.json` (committed,
  team-shared) under `.accelerator/state/integrations/linear/`.
- `external_id` convention identical to Jira: local `id` = own identity,
  `external_id` written on push, presence = synced.

### `create-note` (detail)

- `skills/notes/create-note/SKILL.md`, name `create-note`, arg `[note topic]`,
  writes `paths.notes` (default `meta/notes/`), file `YYYY-MM-DD-<slug>.md` with
  collision disambiguation.
- `templates/note.md`: ADR-0033 frontmatter (`type: note`, `id`, `title`,
  `date`, `author`, `producer: create-note`, `status: captured`, omit-when-empty
  `parent`/`relates_to`, `topic`, `tags`, provenance, `schema_version: 1`),
  body is H1 + free text.
- **Caveat**: `note` is **not** in the managed template registry
  (`TEMPLATE_KEYS`, `config-defaults.sh:66`) nor the configure SKILL.md
  template-keys list — it is resolved by filename only, so it is not
  ejectable/diffable/resettable. Decide whether the README/CHANGELOG should
  present it as a managed template (it currently is not). Pre-existing unrelated
  mismatch: the registry key is `codebase-research` but configure docs say
  `research`.

### Migration 0007 (detail)

- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`.
  Four stages: read-only precondition pre-pass → fence-less backfill →
  deterministic awk rewrite → interactive body-section typed linkage.
- Only migration with `# INTERACTIVE: yes`. Idempotent across resume.
- Bundled migrations are now **0001–0007** (0007 highest). The CHANGELOG draft's
  upgrade note is accurate; add a Migrations subsection entry as above.

## Code References

- `CHANGELOG.md:1-45` — current `[Unreleased]` block (idle, reader, merge only).
- `README.md:10` — "Jump to installation"; `:713` — Installation section.
- `README.md:100` — `notes/` "Written by manual" (stale).
- `README.md:295` — `work.integration` allowed values blurb.
- `README.md:327` — Jira Integration section (Linear should mirror).
- `README.md:444-515` — Visualiser section + Customisation table.
- `skills/integrations/linear/` — 8 Linear skills + `scripts/`.
- `skills/notes/create-note/SKILL.md`, `templates/note.md` — create-note.
- `skills/visualisation/visualise/server/src/api/{search,editor_config,library}.rs`,
  `server/src/{docs,cluster_key,clusters,templates}.rs` — visualiser backend.
- `skills/visualisation/visualise/frontend/src/{components/Sidebar,
  components/DetailHeaderActions,routes/library/recovery}/` — visualiser frontend.
- `skills/visualisation/visualise/SKILL.md:122-155` — editor config docs.
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`.
- `scripts/config-defaults.sh:91` — `WORK_INTEGRATION_VALUES`.
- `scripts/templates-schema.tsv:6` — ADR `status_vocab` (incl. `rejected`).

## Screenshots (plan-document, light + dark)

Two viable sources:

1. **Reuse committed baselines (fast).** The visual-regression baselines render
   the fixture plan `first-plan` in both themes:
   - `skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/library-doc-view.spec.ts-snapshots/library-doc-view-light-visual-regression.png`
   - `…/library-doc-view-dark-visual-regression.png`
   These are Docker/Linux-rendered test fixtures (synthetic plan, possibly
   low-fidelity fonts) — usable but not polished.
2. **Capture fresh (higher fidelity).** Launch the dev stack
   (`mise run dev:up`; dev binary at
   `server/target/debug/accelerator-visualiser`) against the fixture meta
   (`server/tests/fixtures/meta/`, plan `plans/2026-01-01-first-plan.md` at
   route `/library/plans/first-plan`) and screenshot light + dark via
   Playwright, or point the visualiser at a richer real plan in this repo's
   `meta/plans/` for a more representative hero image.

Destination: `assets/` (currently logos only:
`accelerator_logo_{light,dark}_bg.{png,svg}`, `accelerator_icon*`). Embed with
a `<picture>` block matching the existing logo pattern (README.md:2-7).

**Decision needed**: reuse the test baselines, or capture fresh against the
fixture plan, or against a real repo plan. Fresh-against-a-real-plan gives the
best hero image but is the most work.

## Architecture Insights

- The integrations layer is a stable pattern: verb-decomposed skills, a
  read/write (auto vs slash-only-confirmed) split, a committed team catalogue +
  gitignored per-dev credentials, and the `external_id`-presence sync signal.
  Linear slots into this cleanly; the README should present Jira and Linear as
  two instances of one pattern.
- The visualiser's 1.22.0 changes are mostly reader UX (search, recovery,
  editor jump, RCA browsing) plus a frontmatter-schema unification (migration
  0007 + reader reads `id:` only). The schema unification is the one change with
  a hard user obligation (run `/accelerator:migrate`), already called out in the
  upgrade note.

## Historical Context

- `meta/research/codebase/` and `meta/plans/` contain the dogfooded planning
  artifacts behind these features (Linear 0048; create-note; visualiser stories
  0083/0086/0087/0090/0091/0094/0095/0096/0099/0102/0110/0111; migration corpus
  0070/0102). They are the per-feature provenance if deeper detail is needed but
  are not themselves user-facing.

## Open Questions

1. **Screenshots** — **DECIDED**: capture fresh light + dark against a *real*
   plan in this repo's `meta/plans/` (best hero image) via the dev visualiser
   stack, then drop into `assets/`. (See Screenshots for the launch/route
   mechanics.)
2. **`note` as a managed template** — present as-is (filename-resolved, not
   ejectable) or wire it into `TEMPLATE_KEYS` first? Affects whether the README
   template-keys list should include `note`.
3. **Integrations framing** — single "Integrations" umbrella section (Jira +
   Linear subsections) vs a standalone Linear section parallel to Jira.
4. **CHANGELOG granularity** — fold the visualiser reader-polish items into one
   line (recommended for signal-to-noise) vs itemise.

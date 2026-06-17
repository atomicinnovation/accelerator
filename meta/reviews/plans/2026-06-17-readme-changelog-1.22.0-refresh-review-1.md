---
type: plan-review
id: "2026-06-17-readme-changelog-1.22.0-refresh-review-1"
title: "Plan Review: README and CHANGELOG 1.22.0 Refresh"
date: "2026-06-17T14:02:53+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-17-readme-changelog-1.22.0-refresh"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [documentation, correctness, usability, standards]
review_number: 1
review_pass: 2
tags: [changelog, readme, release, docs]
last_updated: "2026-06-17T14:50:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: README and CHANGELOG 1.22.0 Refresh

**Verdict:** REVISE

This is a well-scoped, accuracy-driven documentation refresh with strong
audience discipline (plugin users vs Accelerator developers), an explicit
exclusion list that keeps internal churn out of a consumer CHANGELOG, and
verification that has real teeth where prose allows it (keepachangelog parse,
single-`[Unreleased]` invariant, anchor and skill-name grep cross-checks). The
overwhelming majority of the plan's ~40 concrete factual claims verified
correct against the codebase. The verdict is REVISE on the strength of three
major findings — all narrowly fixable in the plan text — plus a cluster of
small drift/duplication issues: the Linear skill table dictates command
signatures that contradict the real `argument-hint` frontmatter, demoting the
Jira heading silently breaks a link in the already-released CHANGELOG, and the
stated hero-screenshot route slug is wrong.

### Cross-Cutting Themes

- **Linear skill-table accuracy** (flagged by: documentation, correctness,
  usability, standards) — every lens independently caught that the Phase 3
  Usage column ships placeholder signatures that diverge from the shipped
  skills. Most consequential: `create-linear-issue` is shown as `[flags]` but
  actually requires a positional `<work-item-file>`; the table uses
  `<ID>`/`<state>` where the skills use `<IDENTIFIER>`/`<STATE-NAME>`; `attach`
  is `(--url URL | --file PATH)` not `<file...>`; `comment` requires
  `--body`/`--body-file`. The plan's trailing "verify before finalising" aside
  is insufficient mitigation when four reviewers reached for it as a finding —
  it needs to become a binding pre-merge gate with the column transcribed
  verbatim from each `SKILL.md`.

- **Getting Started ↔ Installation duplication** (flagged by: documentation,
  correctness, usability) — the new top-of-file block reproduces the two
  install commands verbatim from the Installation section, and (per
  correctness) re-introduces the tagline that already lives at `README.md:9`
  one line above the line being replaced, so a naive apply prints the tagline
  twice. The duplication is defensible as a quick-start/deep-reference split,
  but the tagline collision is a concrete apply bug and the relationship
  between the two install blocks should be made explicit.

### Tradeoff Analysis

- **Terseness vs completeness of the accuracy pass**: the plan's stated goal is
  high signal-to-noise and it deliberately scopes out "unrelated pre-existing
  drift" (template-keys at `README.md:226-227`). Yet standards flags a residual
  "Ticket board" label (`README.md:453`) that *is* the very tickets→work-items
  drift Phase 4 exists to fix, and documentation flags the template-keys row as
  sitting in the same open file. Recommendation: pull the in-scope "Ticket
  board" fix into Phase 4 (it is the same concern, not unrelated drift); leave
  template-keys out but record *why* so it isn't silently forgotten.

### Findings

#### Major

- 🟡 **Documentation / Correctness / Usability / Standards**: Linear skill-table
  Usage column contradicts the actual `argument-hint` frontmatter
  **Location**: Phase 3, Section 3 (Add the Linear subsection — skill table)
  The proposed table is reference content users copy command shapes from, yet
  `create-linear-issue` is shown as `[flags]` when it requires
  `<work-item-file> [--print-payload] [--quiet]`; identifier tokens are `<ID>`
  not `<IDENTIFIER>`; `transition` shows `<state>` not `<STATE-NAME>`; `attach`
  shows `<file...>` not `(--url URL | --file PATH)`; `comment` omits its
  required `--body`/`--body-file`. Make the verification step binding and
  transcribe each hint verbatim.

- 🟡 **Documentation**: Demoting the Jira heading breaks the `#jira-integration`
  anchor referenced from the released CHANGELOG
  **Location**: Phase 3, Section 2 (Move Jira under it as a subsection)
  `## Jira Integration` → `### Jira` changes the slug from `#jira-integration`
  to `#jira`, breaking the live link at `CHANGELOG.md:90`
  (`[Jira Integration section of the README](README.md#jira-integration)`). The
  plan flags this only as "verify and adjust as needed" — the fix should be
  specified, and the question of whether editing a frozen 1.21.0 entry's link
  target is acceptable (vs preserving the historical anchor) decided
  deliberately.

- 🟡 **Correctness**: Plan-hero route slug is wrong — the id prefix is stripped,
  not just the date
  **Location**: Phase 2, Section 1 (capture the hero screenshot)
  The visualiser's `slug::derive` for plans calls
  `strip_prefix_date_and_optional_id`, which strips the date *and* the canonical
  id token `0067`, so the route for
  `meta/plans/2026-06-06-0067-create-note-skill.md` is
  `/library/plans/create-note-skill`, not the plan's stated
  `/library/plans/0067-create-note-skill`. The stated slug rule ("filename stem
  minus the date prefix") is also incomplete. The capture step's
  "confirm it resolves before capturing" would catch this, but the documented
  fact should be corrected.

#### Minor

- 🔵 **Correctness / Documentation / Usability**: Getting Started block
  duplicates the tagline at `README.md:9` and the install commands at
  `README.md:715-720`
  **Location**: Phase 2, Section 2 (Embed the screenshot + Getting Started)
  The replacement block re-introduces "A Claude Code plugin for structured,
  context-efficient software development." which already exists one line above
  the replaced line, so a literal apply duplicates it. State that `:9` is
  replaced too (or drop the tagline from the new block), and note in the
  Installation section that it expands on Getting Started.

- 🔵 **Correctness**: `editor_project` row omits its env override
  **Location**: Phase 4, Section 3 (Visualiser Customisation table — editor keys)
  `SKILL.md:135` documents `ACCELERATOR_VISUALISER_EDITOR_PROJECT` as the
  one-shot override for `visualiser.editor_project`, but the proposed
  `editor_project` row lists only the config key while the other two editor rows
  pair config + env. Add the env-override row for consistency.

- 🔵 **Standards**: Linear subsection heading levels are unspecified
  **Location**: Phase 3, Section 3 (Add the Linear subsection)
  After demotion Jira's internal sections become `#### Configuration / Skills /
  …`. The plan says "mirror Jira" but doesn't state the level for Linear's own
  sub-parts, leaving room to introduce them at `###` and break the hierarchy.
  Specify `####` explicitly.

- 🔵 **Standards**: Residual "Ticket board" label not in the Phase 4
  terminology fixes
  **Location**: Phase 4, Section 4 (Visualiser Views table + prose)
  Phase 4 fixes the Library row's "tickets" but the Kanban row
  (`README.md:453`) still reads "Ticket board" — the same drift the accuracy
  pass exists to remove. Add it to the Phase 4 fix list.

- 🔵 **Documentation**: Pre-existing template-keys drift at `README.md:226-227`
  is excluded
  **Location**: Phase 4 / What We're NOT Doing
  The table lists `research`/`design-inventory`/`design-gap` keys that don't
  match `TEMPLATE_KEYS`, a verified user-facing inaccuracy in the same
  accuracy-pass scope. The plan explicitly scopes it out; either fold the
  one-line fix in (the file is already open) or record the rationale for
  deferring so it isn't silently forgotten.

#### Suggestions

- 🔵 **Documentation**: The `note` template is filename-resolved, not a managed
  template — unlike every other documented template it is not
  ejectable/diffable/resettable. Add a brief note so the README's create-note
  mention matches actual behaviour (Phase 1, Section 1).
- 🔵 **Usability**: Getting Started shows `init` + `research-codebase` but only
  names the plan/implement steps in a comment. Add the next-step commands or a
  pointer to the Development Loop so the loop is traceable end-to-end
  (Phase 2, Section 2).
- 🔵 **Usability**: During the GitHub manual check, also sanity-check the bare
  `<img>` light fallback in dark-mode renderers that ignore
  `prefers-color-scheme`, since a dense plan screenshot is less forgiving than a
  logo (Phase 2, Section 2).
- 🔵 **Standards**: The new Linear and editor-key tables exceed 80 columns, but
  this matches the existing Jira/Customisation tables — no change needed beyond
  keeping column alignment consistent with the adjacent tables (Phase 3 & 4).

### Strengths

- ✅ Clear, correct audience framing: user-facing vs developer-facing changes
  are explicitly separated, with a precise exclusion list (lint/CI/bash-3.2/
  dev-task/dogfooding) so internal churn doesn't leak into a consumer CHANGELOG.
- ✅ Strong behaviour-documentation alignment — migration 0007 (interactive,
  precondition pre-pass, idempotent), the `rejected` ADR status,
  `visualiser.editor`/`editor_project`, and the `WORK_INTEGRATION_VALUES` set
  all verify against the code (`templates-schema.tsv`, `visualise/SKILL.md`,
  `config-defaults.sh`).
- ✅ Every cited README/CHANGELOG line number (`:10`, `:100`, `:294-295`,
  `:327`, `:449-453`, `:494-501`, `:713`; `CHANGELOG.md:3-44`) resolves exactly
  where the plan says, and all 8 Linear skill names match the filesystem and
  their `name:` frontmatter.
- ✅ Verification has real teeth where prose allows it: keepachangelog parse,
  single-`[Unreleased]` invariant preceding `[1.21.0]`, asset presence,
  anchor-resolution grep, skill-name cross-checks against `ls`.
- ✅ Terseness discipline is explicit and well-judged (folding visualiser
  reader-polish into one Changed line; the "What We're NOT Doing" section).
- ✅ The hero `<picture>` block faithfully reuses the proven logo pattern
  (`<source media>` ordering + `<img>` fallback), the GitHub-correct light/dark
  idiom; the `### Migrations` group is established precedent (1.21.0 entry), not
  an invention, and still parses under keepachangelog.

### Recommended Changes

1. **Make the Linear skill-table verification binding and transcribe hints
   verbatim** (addresses: Linear skill-table Usage column). Replace the
   placeholder Usage column with the exact `argument-hint` from each Linear
   `SKILL.md` — especially `create-linear-issue`'s required `<work-item-file>`
   positional — and standardise on `<IDENTIFIER>`. Promote the aside to a hard
   pre-merge gate in Phase 3's success criteria.

2. **Specify the `#jira-integration` anchor fix** (addresses: Jira demotion
   breaks the released CHANGELOG link). Decide and write down the concrete fix:
   update `CHANGELOG.md:90`, or preserve the `jira-integration` slug (explicit
   anchor / heading text), and state whether editing the frozen 1.21.0 entry is
   acceptable. Don't leave it to execution-time judgement.

3. **Correct the hero route slug** (addresses: plan-hero route slug is wrong).
   Change the route to `/library/plans/create-note-skill` and restate the rule
   as "date prefix *and* a canonical work-item id prefix are both stripped."

4. **Fix the Getting Started duplication** (addresses: tagline/install
   duplication). State that `README.md:9` is replaced (or drop the tagline from
   the new block) so a literal apply doesn't print it twice, and add a one-line
   cross-reference from Installation back to Getting Started.

5. **Complete the Phase 4 accuracy pass** (addresses: editor_project env row,
   "Ticket board" label). Add the `ACCELERATOR_VISUALISER_EDITOR_PROJECT` row,
   and fold the `README.md:453` "Ticket board" → work-item-board fix into the
   terminology cleanup. Specify `####` for Linear's sub-parts.

6. **Note the `note` template's non-managed status** (addresses: create-note
   template suggestion) — one line in the README create-note mention, and
   record the deferral rationale for the `README.md:226-227` template-keys
   drift.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Documentation

**Summary**: A well-scoped, accuracy-driven documentation refresh that
correctly distinguishes plugin-user audience from Accelerator-developer noise
and applies terse, signal-rich changelog discipline. The plan's claims about
what shipped (Linear's 8 skills, create-note, visualiser editor keys, migration
0007, rejected ADR status, integration value set) all verify against the
codebase. The main documentation risks are inaccuracies baked into proposed
content the plan presents verbatim — chiefly the Linear skill-table Usage column
and a real broken cross-reference in the already-released CHANGELOG that the plan
defers rather than specifies.

**Strengths**:
- Clear, correct audience framing with a precise exclusion list keeping internal
  churn out of a consumer changelog.
- Strong behaviour-documentation alignment (migration 0007, rejected ADR status,
  visualiser.editor/editor_project, WORK_INTEGRATION_VALUES all match code).
- Explicit, well-judged terseness discipline (folding reader-polish; "What We're
  NOT Doing").
- Phase 4 targets genuine, verified drift (notes/ "manual", overstated
  four-value integration blurb, missing editor keys).
- Verification with real teeth (keepachangelog parse, single-`[Unreleased]`,
  asset presence, anchor grep, skill-name cross-checks).

**Findings**:
- 🟡 **major** (high) — Phase 3, Section 3: Linear skill-table Usage signatures
  contradict actual SKILL.md argument hints (`create-linear-issue` `[flags]` vs
  `<work-item-file> …`; `<ID>` vs `<IDENTIFIER>`; `<state>` vs `<STATE-NAME>`;
  `<file...>` vs `(--url|--file)`). Plan ships this as concrete content with
  only a trailing verify aside. Transcribe verbatim instead.
- 🟡 **major** (high) — Phase 3, Section 2: Demoting `## Jira Integration` →
  `### Jira` changes the slug to `#jira`, breaking `CHANGELOG.md:90`'s
  `README.md#jira-integration` link. Plan defers the fix to "verify and adjust";
  make it explicit and decide whether editing the frozen 1.21.0 entry is
  acceptable.
- 🔵 **minor** (medium) — Phase 4 / What We're NOT Doing: Pre-existing
  template-keys drift at `README.md:226-227`
  (`research`/`design-inventory`/`design-gap` ≠ actual `TEMPLATE_KEYS`) is
  excluded though it sits in the same accuracy-pass scope. Fold in or record the
  deferral rationale.
- 🔵 **suggestion** (medium) — Phase 1, Section 1: create-note entries describe
  the `note` template without noting it is filename-resolved only (not
  ejectable/diffable/resettable like other templates). Add a brief note.
- 🔵 **suggestion** (low) — Phase 2, Section 2: Getting Started reproduces the
  install commands (`README.md:715-720`) and the tagline (`README.md:9`); flag
  the intentional duplication and keep the top block minimal.

### Correctness

**Summary**: Factually accurate on almost every concrete claim verified — the 8
Linear skill names and slash forms, the config keys (linear.token/token_cmd,
visualiser.editor, ACCELERATOR_VISUALISER_EDITOR, visualiser.editor_project,
work.integration's four values), every cited README line number, the
CHANGELOG:3-44 span, the keepachangelog contract, the #jira-integration anchor it
correctly flags, and the migration-0007 facts. The one verified correctness
error is the plan-hero slug: the visualiser strips the canonical id prefix as
well as the date, so the route is /library/plans/create-note-skill. A few
dictated skill argument-hints also diverge, though the plan instructs re-verify.

**Strengths**:
- Every README line-number citation and the CHANGELOG `:3-44` span resolve
  exactly where the plan says.
- All 8 Linear skill names match `ls` and each `name:` frontmatter; slash forms
  correct.
- Config-key claims accurate (WORK_INTEGRATION_VALUES exactly the four values;
  editor keys exist in SKILL.md:122-155; linear.token/token_cmd real; `note`
  correctly noted absent from TEMPLATE_KEYS).
- Migration 0007 facts verified (only `# INTERACTIVE: yes` migration; read-only
  precondition pre-pass refusing if 0005/0006 unapplied; idempotent).
- Correctly anticipates the #jira-integration anchor break and the live
  reference at CHANGELOG.md:90.
- keepachangelog parse command and grep invariants are sound and runnable.

**Findings**:
- 🟡 **major** (high) — Phase 2, Section 1, step 2: Plan-hero route slug wrong.
  `slug::derive` → `strip_prefix_date_and_optional_id` (server/src/slug.rs:154)
  strips the date AND canonical id `0067` (is_canonical_id_token,
  config.rs:129), so the slug is `create-note-skill` and the route is
  `/library/plans/create-note-skill`, not `/library/plans/0067-create-note-skill`.
  Restate the rule as date + id prefix both stripped.
- 🔵 **minor** (high) — Phase 3, Section 3: Dictated Linear Usage hints diverge
  from actual argument-hints (`create-linear-issue` `<work-item-file> …`;
  `transition` `<STATE-NAME>`; `comment` `--body|--body-file`; `attach`
  `(--url URL | --file PATH)`; `<IDENTIFIER>` not `<ID>`). Mitigated by the
  plan's verify instruction; transcribe verbatim.
- 🔵 **minor** (medium) — Phase 2, Section 2 vs README.md:9: Proposed tagline
  duplicates the existing `README.md:9` line; a naive apply prints it twice.
- 🔵 **minor** (medium) — Phase 4, Section 3: editor_project row omits its env
  override; `ACCELERATOR_VISUALISER_EDITOR_PROJECT` exists (SKILL.md:135) and
  the other two editor rows pair config + env.

### Usability

**Summary**: A well-structured docs-only plan with strong attention to reader
experience: the Getting Started commands are verified correct against the
Installation section and marketplace/plugin config, the hero `<picture>` follows
the proven logo pattern, and the "Remote Work Item Management" umbrella improves
discoverability by grouping Jira and Linear as one pattern. The main usability
risks are a slightly redundant Getting Started-to-Installation relationship and
a few copy-pasteable command examples (notably the Linear skill table) that risk
teaching incorrect invocations if the literal text ships unverified.

**Strengths**:
- Getting Started install commands verified to exactly match the Installation
  section and marketplace.json `name: atomic-innovation` / plugin.json `name:
  accelerator` — genuinely copy-pasteable.
- `/accelerator:init` and `/accelerator:research-codebase` both resolve to real
  skills with matching names.
- Hero `<picture>` reuses the established logo pattern — the GitHub-correct
  light/dark idiom, already proven in this README.
- The "Remote Work Item Management" umbrella framing materially improves
  discoverability and sets correct expectations vs two disconnected sections.
- The 0007 Migrations entry is actionable: names the recovery command, the
  failure-recovery path, and the concrete consequence of skipping it.

**Findings**:
- 🔵 **minor** (high) — Phase 3, Section 3: Linear skill-table Usage column
  doesn't match argument hints; most consequentially `create-linear-issue`
  requires a positional `<work-item-file>` shown as `[flags]`. Make the verify
  step a hard pre-merge gate.
- 🔵 **minor** (medium) — Phase 2, Section 2: Getting Started and Installation
  overlap (same two commands verbatim) with no signal the second occurrence is
  the same stable path; mild "did I already do this?" friction. Note the
  relationship in Installation's opening line.
- 🔵 **suggestion** (medium) — Phase 2, Section 2: Getting Started jumps to
  research without naming the plan/implement steps (only a `# research → plan →
  implement` comment). Add the next-step commands or a pointer to the
  Development Loop.
- 🔵 **suggestion** (low) — Phase 2, Section 2: Confirm the light-image `<img>`
  fallback reads acceptably for dark-mode renderers that ignore
  prefers-color-scheme; keep both captures identical crop/dimensions.

### Standards

**Summary**: Adheres well to project documentation conventions: correctly
mirrors the existing Keep-a-Changelog grouping (including the project's
already-used `### Migrations` group), respects manual 80-column wrapping in
prose blocks, reuses the exact `<picture>` logo pattern, and applies correct
heading-demotion hierarchy when restructuring Jira. The main standards risks are
terminology consistency in the new Linear content (Usage-column hints and `<ID>`
vs `<IDENTIFIER>`; Jira table uses `<KEY>`) and a couple of incompletely
specified mirroring details (Linear subsection heading levels and a residual
"Ticket board" label).

**Strengths**:
- `### Migrations` is established precedent (CHANGELOG.md:193, 1.21.0 entry) and
  the file still parses under keepachangelog.
- Phase 1 preserves the Added/Changed grouping, appends rather than reorders,
  keeps the upgrade-note blockquote.
- Phase 2's hero `<picture>` faithfully copies the logo block at README.md:1-7.
- Phase 3 applies correct heading-hierarchy on demotion and flags the resulting
  anchor change, treating anchor integrity as a first-class invariant.
- Prose blocks are hand-wrapped at ~80 columns per .editorconfig.
- Respects the stated "what NOT to do" boundaries consistently; verification
  confirms conventions mechanically.

**Findings**:
- 🔵 **minor** (high) — Phase 3, Section 3: Linear skill-table Usage column uses
  simplified placeholders not matching `argument-hint`, and `<ID>` matches
  neither the Jira table's `<KEY>` nor Linear's `<IDENTIFIER>`. Make the verify
  note binding; adopt one consistent token (`<IDENTIFIER>`).
- 🔵 **minor** (medium) — Phase 3, Section 3: Linear subsection heading levels
  unspecified; after demotion Jira's sub-parts are `####`. Specify `####` for
  Linear's own sub-parts to avoid breaking the hierarchy.
- 🔵 **minor** (medium) — Phase 4, Section 4: Residual "Ticket board" label at
  README.md:453 is the same tickets→work-items drift Phase 4 fixes elsewhere;
  add it to the terminology fixes.
- 🔵 **minor** (low) — Phase 3 & 4: New Linear/editor-key tables exceed 80
  columns, but this matches the existing Jira/Customisation tables (intentional
  inconsistency for tables); no change required beyond keeping alignment
  consistent.

## Re-Review (Pass 2) — 2026-06-17

**Verdict:** APPROVE

All three major findings from the initial review are resolved, verified fresh by
the same four lenses against the edited plan. The re-review surfaced one *new*
major (a heading-slug collision introduced by the new top-of-file Getting
Started section) plus a set of minors; the new major and the cleanly-actionable
minors were addressed in this same iteration pass. The plan is now in good shape
and acceptable for implementation — the remaining un-actioned items are
low-value suggestions and accepted tradeoffs.

### Previously Identified Issues

- 🟡 **Documentation/Correctness/Usability/Standards**: Linear skill-table Usage
  column contradicts argument-hints — **Resolved**. The table now uses
  `<IDENTIFIER>`, the required `<work-item-file>` positional on create,
  `<STATE-NAME>` on transition, and `--url`/`--file` on attach; the verbatim
  hints are quoted inline and verification is a binding pre-merge gate.
  Correctness confirmed all hints match the SKILL.md frontmatter byte-for-byte.
- 🟡 **Documentation**: Jira demotion breaks the released `#jira-integration`
  link — **Resolved**. Explicit `<a id="jira-integration"></a>` preserves the
  anchor without editing the frozen 1.21.0 entry; correctness verified the link
  is at `CHANGELOG.md:90` and no other in-doc references exist.
- 🟡 **Correctness**: Hero route slug wrong — **Resolved**. Corrected to
  `/library/plans/create-note-skill`; correctness re-derived this is right under
  the repo's default-numeric config (id token `0067` is stripped).
- 🔵 **Correctness/Documentation/Usability**: Getting Started tagline/install
  duplication — **Resolved**. README:9 tagline kept, only the `:10` jump line
  replaced; bidirectional Getting Started ↔ Installation cross-reference added.
- 🔵 **Correctness**: `editor_project` env override missing — **Resolved**.
  `ACCELERATOR_VISUALISER_EDITOR_PROJECT` row added; all four editor keys
  confirmed present in SKILL.md.
- 🔵 **Standards**: Linear sub-part heading levels unspecified — **Resolved**.
  `### Linear` with `####` sub-parts now specified.
- 🔵 **Standards**: Residual "Ticket board" label — **Resolved**. Phase 4 now
  fixes both the Library and Kanban rows; standards verified "ticket" occurs
  only at README:451 and :453.
- 🔵 **Documentation** (suggestion): create-note non-managed caveat — **Resolved**
  (then tightened to a single clause this pass, per a new documentation note
  that the prior wording over-explained).
- 🔵 **Usability** (suggestion): Getting Started missing plan/implement steps —
  **Resolved**. Now shows init→research→plan→implement with a Development Loop
  pointer (skills verified to exist).
- 🔵 **Usability** (suggestion): hero `<img>` fallback in dark mode — **Resolved**.
  Sanity-check added to Phase 2 manual verification.
- 🔵 **Documentation** (minor): pre-existing template-keys drift at
  `README.md:226-227` — **Accepted tradeoff** (unchanged); genuinely pre-existing
  drift outside the 1.22.0 surface, rationale recorded in "What We're NOT Doing".

### New Issues Introduced

- 🟡 **Usability** (major): The new top-of-file `## Getting Started` collides
  with the pre-existing `### Getting Started` at `README.md:240` — both slugify
  to `#getting-started`, silently shifting the older one to `#getting-started-1`.
  **Addressed this pass**: the pre-existing `README.md:240` heading is renamed
  from `### Getting Started` to `### Managing Configuration` (clean
  `#managing-configuration` slug; pairs with the sibling `### Template
  Management`), so the new top-of-file `## Getting Started` owns
  `#getting-started`. Verified nothing links to `#getting-started`. Phase 2/Phase 3
  success criteria assert exactly one "Getting Started" heading remains, the
  renamed section exists, and no duplicate heading slugs are introduced.
- 🔵 **Standards** (minor): Hero `<img>` omitted the `width` attribute the logo
  block sets. **Addressed**: `width="760px"` added.
- 🔵 **Correctness** (minor): Two Phase-1/Phase-3 "automated" checks were
  actually manual eyeballing (`grep | head`; the anchor cross-check).
  **Addressed**: single-`[Unreleased]` and ordering are now real `test`
  assertions; the broad anchor cross-check is relabelled manual; the genuinely
  automated `<a id>` and duplicate-slug greps remain in automated.
- 🔵 **Documentation** (minor): RCA "Operate" category label asserted from
  research, not verified against the shipped UI. **Addressed**: Phase 4 manual
  verification now checks the literal sidebar labels.
- 🔵 **Standards** (minor): 80-column convention for new prose was implicit.
  **Addressed**: explicit hand-wrap gate added to the Testing Strategy.
- 🔵 **Documentation** (minor): No `Removed`/`Fixed` group consideration.
  **Addressed**: grouping recorded as a deliberate decision in Phase 1.
- 🔵 **Usability** (minor): "Remote Work Item Management" umbrella less
  discoverable than the tracker names. **Addressed**: intro now leads with
  "Jira and Linear" and the heading may parenthesise them; `create-plan`
  optional-arg comment added.
- 🔵 **Correctness/Usability** (suggestions): slug derivation is config-dependent;
  prerelease-channel signalling in Getting Started; Linear changelog entry shape.
  **All now actioned** — see the follow-up pass below.

### Follow-up edits (post-pass-2, same review cycle)

After the re-review, the user asked to action **every** remaining outstanding
item, including those previously logged as accepted tradeoffs. All are now
addressed in the plan:

- 🔵 **Documentation** (was accepted tradeoff): pre-existing template-keys drift
  at `README.md:226-227` — **Fixed**. Removed from "What We're NOT Doing"; a new
  Phase 4 §5 corrects the list to the real `TEMPLATE_KEYS`
  (`research` → `codebase-research`, drop non-existent `design-inventory` /
  `design-gap`, do not add `note`), with a precise backtick-scoped success
  criterion that avoids catching the legitimate `design-gaps/` dir and
  `analyse-design-gaps` skill.
- 🔵 **Standards**: Linear CHANGELOG entry shape — **Fixed**. Reshaped to mirror
  the frozen 1.21.0 Jira entry: a per-skill sub-bullet list plus a
  `[Linear subsection of the README](README.md#linear)` cross-link.
- 🔵 **Usability**: no prerelease-channel cue in Getting Started — **Fixed**. The
  Installation cross-reference now notes the prerelease channel is "where the
  newest features land first".
- 🔵 **Correctness**: hero slug is config-dependent — **Fixed**. Phase 2 §1 now
  notes the derivation holds for the default-numeric id pattern and is
  config-dependent under a project-code prefix; the live-sidebar check validates
  it regardless.
- 🔵 **Usability**: Migrations entry ordering — **Fixed**. The 0007 entry now
  states `run /accelerator:migrate` immediately before the data-loss consequence
  and closes with "applying the migration restores them".
- 🔵 **Documentation**: hero `<img>` alt text — **Fixed**. Added a Phase 2 manual
  check that the alt text is meaningful and intentionally theme-agnostic.
- 🔵 **Standards**: Customisation-table row convention — **Fixed**. Added a note
  to Phase 4 §3 to preserve the ` config key` suffix / bare-uppercase env-var
  convention when inserting rows.

### Assessment

The plan is in good shape and ready for implementation. Every initial major is
resolved and verified, the one new major plus the actionable minors were fixed
in pass 2, and — per the follow-up — every remaining minor/suggestion and the
two accepted tradeoffs have now been actioned as well. The plan's factual claims
are exceptionally accurate (correctness verified the Linear hints, slug
derivation, anchor fix, config keys, and line numbers all hold), and the
verification commands are honestly labelled (real assertions vs manual aids).
No further review pass is required.

**Approved.** With every finding across both passes and all former accepted
tradeoffs actioned, the review verdict is upgraded to APPROVE and the plan is
marked ready for implementation.
